//! Original object-definition sprites survive own/inherited Create. These are
//! birth/query controls, distinct from legitimate subsequent Step artwork.
use spelunky_core::{
    CollisionPrecision, Game, InputFrame, Rect, RngAlgorithm, RulesPreset, Vec2, catalog, enemies,
};

fn configurations(mut run: impl FnMut(RulesPreset, RngAlgorithm)) {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for algorithm in [RngAlgorithm::LegacySplitMix64V1, RngAlgorithm::Gm8Lcg32V1] {
            run(rules, algorithm);
        }
    }
}

fn birth(object: &str, sprite: &str, speed: f64, bounds: [f64; 4], changed_point: Option<Vec2>) {
    configurations(|rules, algorithm| {
        let mut game = Game::shell_with_algorithm(42, rules, algorithm).unwrap();
        game.player.pos = Vec2::new(200.0, 120.0);
        let origin = Vec2::new(80.0, 80.0);
        let mut expected_rng = game.rng.clone();
        // Exactly one source random direction draw for these two Creates;
        // the sprite correction adds/removes no random call in either algorithm.
        if matches!(object, "oMonkey" | "oPiranha") {
            expected_rng.range(2);
        }
        let id = game.spawn(object, origin);
        assert_eq!(
            game.world.entities.len(),
            1,
            "{object} birth has no children"
        );
        let e = game.world.entity(id).unwrap();
        assert_eq!(e.sprite, sprite, "{object}/{rules:?}/{algorithm:?}");
        assert_eq!(e.animation, 0.0);
        assert_eq!(e.image_speed(), speed);
        assert_eq!(e.age, 0);
        assert_eq!(e.pos, origin);
        assert_eq!(e.previous_pos, origin);
        assert_eq!(e.state, if object == "oMonkey" { 5 } else { 0 });
        assert_eq!(e.timer, if object == "oSkeleton" { 20 } else { 0 });
        // Literal Original sprite cached bounds, distinct from custom movement bounds.
        assert_eq!(
            e.sprite_bounds(),
            Rect::new(
                origin.x + bounds[0],
                origin.y + bounds[1],
                bounds[2],
                bounds[3]
            )
        );
        if let Some(offset) = changed_point {
            let point = Vec2::new(origin.x + offset.x, origin.y + offset.y);
            assert!(
                e.mask_contains(point, CollisionPrecision::BoundingBox),
                "{object} first birth query"
            );
            let old_sprite = match object {
                "oMonkey" => "sMonkeyHangL",
                "oSkeleton" => "sSkeletonLeft",
                "oYetiKing" => "sYetiKingRight",
                _ => unreachable!(),
            };
            assert!(
                !catalog::instance_mask(object, old_sprite, origin, 0.0, Vec2::new(1.0, 1.0), 0.0)
                    .contains(point, CollisionPrecision::BoundingBox),
                "{object} premature sprite loses this exact birth query"
            );
        }
        assert_eq!(
            game.rng.clone().next_u64(),
            expected_rng.next_u64(),
            "{object} exact Create stream"
        );
    });
}

#[test]
fn natural_monkey_birth_keeps_left_sprite_and_first_bbox_query_before_hang_step() {
    birth(
        "oMonkey",
        "sMonkeyLeft",
        0.4,
        [2.0, 4.0, 13.0, 12.0],
        Some(Vec2::new(2.0, 4.0)),
    );
}

#[test]
fn natural_piranha_birth_keeps_left_art_before_swimming_step() {
    // Left/right masks are equal here; the proven omission is the actual artwork.
    birth("oPiranha", "sPiranhaLeft", 0.5, [0.0, 0.0, 8.0, 8.0], None);
}

#[test]
fn natural_dead_fish_birth_keeps_left_art_before_direction_step() {
    birth(
        "oDeadFish",
        "sDeadFishLeft",
        0.5,
        [0.0, 1.0, 8.0, 6.0],
        None,
    );
}

#[test]
fn natural_skeleton_birth_keeps_create_sprite_and_first_bbox_query_before_idle_step() {
    birth(
        "oSkeleton",
        "sSkeletonCreateL",
        0.5,
        [1.0, 9.0, 15.0, 7.0],
        Some(Vec2::new(1.0, 9.0)),
    );
}

#[test]
fn natural_yeti_king_birth_keeps_left_sprite_and_first_bbox_query_despite_right_facing() {
    birth(
        "oYetiKing",
        "sYetiKingLeft",
        0.25,
        [1.0, 0.0, 31.0, 32.0],
        Some(Vec2::new(31.0, 0.0)),
    );
}

#[test]
fn ordinary_first_steps_still_choose_source_sprites_after_birth() {
    configurations(|rules, algorithm| {
        for object in ["oMonkey", "oPiranha", "oDeadFish", "oSkeleton", "oYetiKing"] {
            let mut game = Game::shell_with_algorithm(42, rules, algorithm).unwrap();
            game.player.pos = Vec2::new(200.0, 120.0);
            game.player.data.insert("active".into(), 0.0);
            // Original camera stores the view's top-left, not its center.
            // Keep every source callback's (80,80) birth inside the initial view.
            game.camera = Vec2::default();
            game.previous_camera = game.camera;
            // Fish's real Step requires water; author only this initial terrain.
            if matches!(object, "oPiranha" | "oDeadFish") {
                for y in 2..10 {
                    for x in 2..15 {
                        game.world.set_tile(x, y, "oWaterSwim");
                    }
                }
            }
            let id = game.spawn(object, Vec2::new(80.0, 80.0));
            assert!(game.simulation_view_rect().contains(Vec2::new(80.0, 80.0)));
            assert!(game.world.entity(id).unwrap().engine_active);
            game.step(&InputFrame::default());
            let e = game
                .world
                .entity(id)
                .expect("ordinary first Step preserves healthy birth");
            assert_eq!(e.age, 1);
            let expected = match object {
                "oMonkey" => {
                    // Source HANG with Create counter 0 selects CLIMB on this Step.
                    assert_eq!(e.state, 6);
                    "sMonkeyClimbL"
                }
                "oPiranha" if e.facing < 0 => "sPiranhaLeft",
                "oPiranha" => "sPiranhaRight",
                "oDeadFish" => "sDeadFishRight", // source Create dir=0, clear water probe
                "oSkeleton" => {
                    assert_eq!((e.state, e.timer), (0, 19));
                    "sSkeletonLeft"
                }
                "oYetiKing" => {
                    assert_eq!(e.state, 1); // source IDLE counter=0 selects WALK
                    "sYetiKingWalkR"
                }
                _ => unreachable!(),
            };
            assert_eq!(e.sprite, expected, "{object}/{rules:?}/{algorithm:?}");
        }
    });
}

#[test]
fn explicit_post_create_producer_art_and_animation_survive_initializer_reentry() {
    configurations(|rules, algorithm| {
        let mut game = Game::shell_with_algorithm(42, rules, algorithm).unwrap();
        for (object, sprite) in [
            ("oMonkey", "sMonkeyHangL"),
            ("oPiranha", "sPiranhaRight"),
            ("oDeadFish", "sDeadFishRight"),
            ("oSkeleton", "sSkeletonLeft"),
            ("oYetiKing", "sYetiKingRight"),
        ] {
            let id = game.spawn(object, Vec2::new(80.0, 80.0));
            let e = game.world.entity_mut(id).unwrap();
            e.set_sprite(sprite);
            e.animation = 0.75;
            let origin = e.pos;
            enemies::init_entity(e);
            assert_eq!(e.sprite, sprite);
            assert_eq!(e.animation, 0.75);
            assert_eq!(e.pos, origin);
        }
    });
}
