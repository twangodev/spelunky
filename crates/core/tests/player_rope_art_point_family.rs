//! Authored concrete ladder and initial default Rope/RopeTop actors. After
//! loading, one ordinary Up attaches; no player or actor fields are injected.
//! This tests source point-family selection, not stock rope-cap overlap frequency.
use spelunky_core::*;

fn room(rules: RulesPreset, extra: Option<&str>) -> (Game, Option<u64>) {
    let mut level = level::Level::default();
    level.set_tile(5, 4, '@').unwrap(); // Source entrance104,88.
    for x in 0..40 {
        level.set_tile(x, 5, '1').unwrap(); // Source floor96.
    }
    let mut game = Game::shell(42, rules);
    game.load_custom(&level, "rope-art-point-family", "");
    game.spawn("oLadderOrange", Vec2::new(96.0, 80.0));
    let id = extra.map(|name| {
        game.spawn(
            name,
            if name == "oRopeTop" {
                Vec2::new(104.0, 92.0)
            } else {
                Vec2::new(96.0, 88.0)
            },
        )
    });
    assert_eq!(game.player.pos, Vec2::new(104.0, 88.0));
    assert_eq!(game.player.vel, Vec2::default());
    assert_eq!(game.player.health, 4.0);
    assert!(game.world.rectangle_family_except(
        Vec2::new(99.0, 96.0),
        Vec2::new(108.0, 96.0),
        "oSolid",
        None,
        CollisionPrecision::Precise,
    ));
    if let Some(id) = id {
        let target = game.world.entity(id).unwrap();
        assert!(target.alive && target.engine_active);
        assert!(target.mask_contains(game.player.pos, CollisionPrecision::BoundingBox));
        if target.object == "oRopeTop" {
            // Original sprite origin4,4 / bbox0,0..7,7 => raw100..107,
            // y88..95. It contains the point but is NOT an oRope descendant.
            assert_eq!(target.pos, Vec2::new(104.0, 92.0));
            assert!(!catalog::is_a(&target.object, "oRope"));
            assert!(!catalog::is_a(&target.object, "oLadder"));
        } else {
            // Original sRope origin4,4 / bbox10,0..13,7 =>102..105,84..91.
            assert_eq!(target.pos, Vec2::new(96.0, 88.0));
            assert!(catalog::is_a(&target.object, "oRope"));
            assert!(catalog::is_a(&target.object, "oLadder"));
        }
    }
    (game, id)
}

fn up(game: &mut Game) {
    game.step(&InputFrame {
        up: true,
        up_pressed: true,
        ..Default::default()
    });
    assert_eq!(game.state, RunState::Playing);
    assert_eq!(game.player.health, 4.0);
    assert_eq!(game.player.stun, 0);
    assert!(!game.player.is_stunned());
    assert_eq!(game.player.state, PlayerState::Climbing);
    assert_eq!(game.player.pos, Vec2::new(104.0, 88.0));
    assert_eq!(game.player.vel, Vec2::default());
    assert!(!game.world.solid_at(game.player.pos.x, game.player.pos.y));
    assert_eq!(game.engine_time(), 2);
}

#[test]
fn ordinary_up_on_concrete_ladder_does_not_use_rope_art_for_non_rope_cap() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, id) = room(rules, Some("oRopeTop"));
        up(&mut game);
        let target = game.world.entity(id.unwrap()).unwrap();
        assert!(target.alive && target.engine_active);
        assert_eq!(target.pos, Vec2::new(104.0, 92.0));
        assert!(target.mask_contains(game.player.pos, CollisionPrecision::BoundingBox));
        assert_eq!(
            game.player.sprite, "sClimbUp",
            "SOURCE_CHARACTER_SPRITE_QUERIES_ROPE_NOT_CAP"
        );
    }
}

#[test]
fn ordinary_up_preserves_real_rope_art_and_plain_ladder_controls() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for extra in [Some("oRope"), None] {
            let (mut game, id) = room(rules, extra);
            up(&mut game);
            if let Some(id) = id {
                let rope = game.world.entity(id).unwrap();
                assert!(rope.alive && rope.engine_active);
                assert_eq!(rope.pos, Vec2::new(96.0, 88.0));
                assert!(rope.mask_contains(game.player.pos, CollisionPrecision::BoundingBox));
                assert_eq!(game.player.sprite, "sClimbUp2");
            } else {
                assert_eq!(game.player.sprite, "sClimbUp");
            }
        }
    }
}
