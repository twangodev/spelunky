//! Source Create/Alarm view writes and native finite pre-Draw follow controls.
//! Authored normal-step scenes use fresh actors and ordinary inputs. Direct
//! follow controls are configured callback projections, not native execution.
use spelunky_core::*;

fn shell(canonical: bool, rules: RulesPreset) -> Game {
    let mut game = if canonical {
        Game::shell(42, rules)
    } else {
        Game::legacy_shell(1, rules)
    };
    game.world = World::new(40, 40, Area::Mines, 1);
    game.world.name = "rLevel".into();
    game.player.pos = Vec2::new(128.0, 216.0);
    game.player.previous_pos = game.player.pos;
    game.player.data.insert("active".into(), 1.0);
    game.world.set_tile(8, 14, "oBlock");
    game
}
fn variants(mut run: impl FnMut(bool, RulesPreset)) {
    for canonical in [true, false] {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            run(canonical, rules);
        }
    }
}

#[test]
fn fresh_olmec_create_writes_view_without_following_or_disabling_player() {
    variants(|canonical, rules| {
        let mut game = shell(canonical, rules);
        game.camera = Vec2::new(17.0, 53.0);
        let id = game.spawn("oOlmec", Vec2::new(550.0, 96.0));
        assert_eq!(game.player.data.get("active"), Some(&1.0));
        assert_eq!(game.camera.x, 17.0, "CREATE_PRESERVES_X");
        assert_eq!(game.camera.y, 400.0, "CREATE_Y400_WITHOUT_EAGER_FOLLOW");
        assert_eq!(game.data.get("camera_hborder"), Some(&0.0));
        assert_eq!(game.data.get("camera_vborder"), Some(&0.0));
        assert_eq!(game.world.entity(id).unwrap().pos, Vec2::new(550.0, 96.0));
        assert_eq!(game.world.entity(id).unwrap().state, -1);
    });
}

#[test]
fn active_player_normal_steps_follow_first_boss_after_initial_activation() {
    variants(|canonical, rules| {
        let mut game = shell(canonical, rules);
        game.world = World::new(40, 24, Area::Boss, 16);
        game.world.name = "rLevel".into();
        for x in 0..40 {
            game.world.set_tile(x, 14, "oBlock");
        }
        for x in (1..6).chain(34..39) {
            game.world.set_tile(x, 10, "oBlock");
        }
        game.set_simulation_view_extent(640.0, 240.0).unwrap();
        // Source-correct initial view context in both arms. The independent
        // Create test above discriminates the actual constructor write.
        game.camera = Vec2::new(0.0, 400.0);
        let first = game.spawn("oOlmec", Vec2::new(550.0, 96.0));
        let second = game.spawn("oOlmec", Vec2::new(16.0, 96.0));
        let trap = game.spawn("oArrowTrapLeft", Vec2::new(48.0, 16.0));
        game.step(&InputFrame::default());
        assert_eq!(game.player.health, 4.0);
        assert_eq!(game.player.data.get("active"), Some(&1.0));
        // Adaptive activation derives Y144 from canonical400, so this trap
        // lies above its Y48 lower halo and outside Player/first-boss halos.
        assert!(
            game.presentation()
                .dispatcher
                .hidden_entities()
                .iter()
                .any(|entity| entity.id == trap && entity.alive && !entity.engine_active),
            "INITIAL_ACTIVATION_PRECEDES_AUTOMATIC_FOLLOW"
        );
        assert!(
            game.world
                .entity(trap)
                .is_some_and(|entity| !entity.engine_active)
        );
        // Native follow still uses canonical320x240, zeroH, and Level's V96.
        assert_eq!(
            game.camera,
            Vec2::new(230.0, 0.0),
            "CURRENT_FIRST_BOSS_TARGET_WITH_ACTIVE_PLAYER"
        );
        assert_eq!(game.world.entity(first).unwrap().state, -1);
        assert_eq!(game.world.entity(second).unwrap().state, -1);
        game.step(&InputFrame::default());
        assert!(
            game.world
                .entity(trap)
                .is_some_and(|entity| entity.engine_active)
        );
        for id in [first, second] {
            let boss = game.world.entity(id).unwrap();
            assert_eq!(boss.state, -2, "SOURCE_START1_ARMS_ALARM_AFTER_FOLLOW");
            assert_eq!(boss.alarms[1], 100);
        }
        assert_eq!(game.player.health, 4.0);
    });
}

#[test]
fn configured_marked_follow_receiver_is_skipped_and_no_receiver_preserves_view() {
    // Marked (+108) controls, not pending engine-inactive (+109) writes.
    // No Normal Step or Destroy callback is represented by these projections.
    for mark_all in [false, true] {
        let mut game = shell(true, RulesPreset::Classic);
        game.camera = Vec2::new(55.0, 400.0);
        let first = game.spawn("oOlmec", Vec2::new(550.0, 96.0));
        let second = game.spawn("oOlmec", Vec2::new(304.0, 496.0));
        game.world.entity_mut(first).unwrap().alive = false;
        if mark_all {
            game.world.entity_mut(second).unwrap().alive = false;
        }
        let before = game.camera;
        let rng = game.rng.clone().next_u64();
        let events = game.events.clone();
        game.update_camera();
        assert_eq!(
            game.camera,
            if mark_all {
                before
            } else {
                Vec2::new(55.0, 400.0)
            },
            "MARKED_SKIP_OR_NO_TARGET_NO_COORDINATE_WRITES"
        );
        assert_eq!(game.rng.clone().next_u64(), rng);
        assert_eq!(game.events, events);
    }
}

#[test]
fn configured_player_follow_rounds_half_pixel_origins_without_moving_player() {
    for (origin, expected) in [
        (Vec2::new(199.5, 199.5), Vec2::new(8.0, 56.0)),
        (Vec2::new(200.5, 200.5), Vec2::new(8.0, 56.0)),
        (Vec2::new(201.5, 201.5), Vec2::new(10.0, 58.0)),
        (Vec2::new(-0.5, -1.5), Vec2::new(0.0, 0.0)),
        // Valid FISTP64 target, whose low32 origin is negative before clamps.
        (Vec2::new(2147483848.5, 200.5), Vec2::new(0.0, 56.0)),
    ] {
        let mut game = shell(true, RulesPreset::Classic);
        game.player.pos = origin;
        game.camera = Vec2::default();
        game.data.insert("camera_hborder".into(), 128.0);
        game.data.insert("camera_vborder".into(), 96.0);
        let rng = game.rng.clone().next_u64();
        let events = game.events.clone();
        game.update_camera();
        assert_eq!(game.camera, expected, "PLAYER_FISTP_NEAREST_EVEN_ORIGIN");
        assert_eq!(game.player.pos, origin);
        assert_eq!(game.rng.clone().next_u64(), rng);
        assert_eq!(game.events, events);
    }
}

#[test]
fn new_custom_room_discards_boss_target_and_preserves_default_wire_shape() {
    let mut game = shell(true, RulesPreset::Classic);
    game.spawn("oOlmec", Vec2::new(550.0, 96.0));
    let mut level = level::Level::default();
    level.set_tile(9, 6, '@').unwrap();
    for x in 0..40 {
        level.set_tile(x, 7, '1').unwrap();
    }
    game.load_custom(&level, "next-authored-room", "");
    assert!(
        !game
            .world
            .entities
            .iter()
            .any(|actor| actor.object == "oOlmec")
    );
    assert!(game.data.get("camera_hborder").is_none());
    assert_eq!(game.player.pos, Vec2::new(168.0, 120.0));
    let encoded = serde_json::to_value(&game).unwrap();
    assert!(
        encoded.get("view_target").is_none(),
        "DEFAULT_TARGET_WIRE_OMISSION"
    );
    // Existing snapshot JSON without the new optional field still decodes.
    let mut decoded: Game = serde_json::from_value(encoded).unwrap();
    assert_eq!(decoded.state_hash(), game.state_hash());
    decoded.camera = Vec2::default();
    decoded.update_camera();
    assert_eq!(decoded.camera, Vec2::default(), "NEW_ROOM_PLAYER_DEFAULTS");
}
