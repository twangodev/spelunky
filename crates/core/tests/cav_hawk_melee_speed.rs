//! Default Caveman/Hawkman sight creates ATTACK; ordinary whip then resets
//! its source animation speed. Initial authored scene only, no midway writes.
use spelunky_core::*;

fn scene(object: &str, rules: RulesPreset, algorithm: RngAlgorithm, rear: bool) -> (Game, u64) {
    for seed in 0..128 {
        let mut game = Game::shell_with_algorithm(seed, rules, algorithm).unwrap();
        game.world = World::new(24, 20, Area::Mines, 1);
        game.world.name = "rLevel".into();
        for x in 0..game.world.width {
            game.world.set_tile(x, 10, "oBlock");
            // Touching ceiling prevents Hawkman's own ATTACK leap while
            // leaving the natural sight ray and player at floor height clear.
            game.world.set_tile(x, 8, "oBlock");
        }
        game.player.pos = Vec2::new(200.0, 152.0);
        game.player.previous_pos = game.player.pos;
        let id = game.spawn(object, Vec2::new(96.0, 144.0));
        let e = game.world.entity(id).unwrap();
        assert_eq!(e.state, 0);
        assert_eq!(e.health, if object == "oCaveman" { 3 } else { 4 });
        assert_eq!(e.vel, Vec2::new(2.5, 0.0));
        // All authored terrain and default Create hooks precede this read.
        // The first idle-to-walk draw selects RIGHT, casting its own sight.
        if game.rng.clone().range(2) != 1 {
            continue;
        }
        game.step(&InputFrame {
            left: !rear,
            ..Default::default()
        });
        assert_eq!(game.player.facing, if rear { 1 } else { -1 });
        assert_eq!(game.player.health, 4.0);
        assert_eq!(game.world.entity(id).unwrap().facing, 1);
        return (game, id);
    }
    panic!("SOURCE_INITIAL_RIGHT_FACING_SEED_NOT_FOUND");
}

fn variants(mut check: impl FnMut(&str, RulesPreset, RngAlgorithm)) {
    for object in ["oCaveman", "oHawkman"] {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            for algorithm in [RngAlgorithm::Gm8Lcg32V1, RngAlgorithm::LegacySplitMix64V1] {
                check(object, rules, algorithm);
            }
        }
    }
}

fn accepted_hit(object: &str, rules: RulesPreset, algorithm: RngAlgorithm, rear: bool) {
    let (mut game, id) = scene(object, rules, algorithm, rear);
    let starting_hp = game.world.entity(id).unwrap().health;
    let mut armed = false;
    for _ in 0..96 {
        let before = game.world.entity(id).unwrap().clone();
        let gap = game.player.pos.x - before.pos.x;
        // Rear Whip is born immediately behind a right-facing player.
        // Forward Whip needs the natural attack animation to pass frame4;
        // begin far enough away to keep the approaching body out of contact.
        let range = if rear { 27.0..=32.0 } else { 50.0..=56.0 };
        let fire =
            !armed && before.state == 2 && before.image_speed() == 1.0 && range.contains(&gap);
        game.step(&InputFrame {
            attack: fire,
            attack_pressed: fire,
            ..Default::default()
        });
        armed |= fire;
        assert_eq!(
            game.player.health, 4.0,
            "ordinary chase and weapon route must avoid contact injury"
        );
        let e = game
            .world
            .entity(id)
            .expect("nonlethal source melee leaves a body");
        if e.health == starting_hp {
            continue;
        }
        assert!(
            armed,
            "default actor must be hit only after ordinary attack input"
        );
        assert_eq!(before.state, 2, "SOURCE_NATURAL_CHASE_BEFORE_MELEE");
        assert_eq!(
            before.image_speed(),
            1.0,
            "SOURCE_ATTACK_SPEED_BEFORE_MELEE"
        );
        assert_eq!(e.health, starting_hp - 1);
        assert_eq!(e.state, 98);
        assert_eq!(e.timer, 200);
        assert_eq!(e.vel.y, -3.0);
        assert!(e.vel.x.abs() == 2.0);
        assert!(
            game.world
                .entities
                .iter()
                .any(|w| w.alive && w.object == if rear { "oWhipPre" } else { "oWhip" }),
            "source-selected actual front/rear whip must exist"
        );
        assert_eq!(
            e.image_speed(),
            0.5,
            "SOURCE_CAV_HAWK_ACCEPTED_MELEE_IMAGE_SPEED"
        );
        return;
    }
    panic!("SOURCE_NATURAL_CHASE_AND_REQUESTED_WHIP_HIT_NOT_REACHED");
}

#[test]
fn natural_caveman_and_hawkman_front_whip_resets_chase_animation_speed() {
    variants(|object, rules, algorithm| accepted_hit(object, rules, algorithm, false));
}
#[test]
fn natural_caveman_and_hawkman_rear_whip_resets_chase_animation_speed() {
    variants(|object, rules, algorithm| accepted_hit(object, rules, algorithm, true));
}
#[test]
fn natural_caveman_and_hawkman_without_melee_keep_attack_animation_speed() {
    variants(|object, rules, algorithm| {
        let (mut game, id) = scene(object, rules, algorithm, true);
        let hp = game.world.entity(id).unwrap().health;
        let mut chase_frames = 0;
        for _ in 0..64 {
            let before = game.world.entity(id).unwrap().state;
            game.step(&InputFrame::default());
            assert_eq!(game.player.health, 4.0);
            let e = game.world.entity(id).unwrap();
            assert_eq!(e.health, hp);
            if before == 2 {
                assert_eq!(e.state, 2);
                assert_eq!(
                    e.image_speed(),
                    1.0,
                    "source own ATTACK step keeps speed1 without accepted melee"
                );
                chase_frames += 1;
                if chase_frames == 4 {
                    return;
                }
            }
        }
        panic!("SOURCE_DEFAULT_SIGHT_DID_NOT_ESTABLISH_FOUR_CHASE_FRAMES");
    });
}
