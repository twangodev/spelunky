//! Ordinary authored pool/ladder routes, plus an explicitly configured phase
//! control. All actors/fields are configured before the first ordinary Step;
//! there are no later player, timer, velocity, camera, or world writes.
use spelunky_core::*;

fn value(game: &Game, key: &str) -> f64 {
    *game.player.data.get(key).unwrap_or(&0.0)
}

fn point(game: &Game, at: Vec2, family: &str) -> bool {
    game.world.tiles.iter().enumerate().any(|(i, tile)| {
        tile.engine_active
            && catalog::is_a(&tile.object, family)
            && game
                .world
                .tile_collision_mask((i % game.world.width) as i32, (i / game.world.width) as i32)
                .contains(at, CollisionPrecision::BoundingBox)
    }) || game.world.entities.iter().any(|e| {
        e.alive
            && e.engine_active
            && catalog::is_a(&e.object, family)
            && e.mask_contains(at, CollisionPrecision::BoundingBox)
    })
}

fn safe(game: &Game) {
    assert_eq!(game.state, RunState::Playing);
    assert_eq!(game.player.health, 4);
    assert_eq!(game.player.stun, 0);
    assert!(!game.player.is_stunned());
    assert!(!game.world.solid_at(game.player.pos.x, game.player.pos.y));
    assert_eq!(game.player.pos.x, 104.0);
}

fn room(rules: RulesPreset, wet: bool) -> Game {
    let mut level = level::Level::default();
    level.set_tile(5, 9, '@').unwrap(); // Player (104,168).
    for x in 0..40 {
        level.set_tile(x, 11, '1').unwrap(); // Floor y192.
    }
    for y in [9, 10] {
        level.set_tile(4, y, '1').unwrap();
        level.set_tile(6, y, '1').unwrap();
    }
    for y in 2..=8 {
        level.set_tile(5, y, 'L').unwrap();
    }
    if wet {
        level.set_tile(5, 10, 'w').unwrap();
    }
    let mut game = Game::shell(42, rules);
    game.load_custom(&level, "swimming-climbing-latch", "");
    // The text format has one object per cell. These initial default actors
    // supply the source-permitted pool/ladder overlap at the entrance.
    game.spawn("oLadderOrange", Vec2::new(96.0, 160.0));
    game.spawn("oLadderOrange", Vec2::new(96.0, 176.0));
    if wet {
        game.spawn("oWaterSwim", Vec2::new(96.0, 160.0));
    }
    assert_eq!(game.player.pos, Vec2::new(104.0, 168.0));
    assert_eq!(value(&game, "swimming"), 0.0);
    assert_eq!(point(&game, game.player.pos, "oWaterSwim"), wet);
    game
}

fn bubble_next(incoming: f64) -> f64 {
    if incoming > 0.0 { incoming - 1.0 } else { 20.0 }
}

fn climb_out(game: &mut Game, wet: bool) {
    // One fixed held-Up tape; termination observes the initial dry-Climbing
    // phase, never chooses a seed/route by its damage or later outcome.
    for frame in 0..160 {
        let was_dry_climbing = game.player.state == PlayerState::Climbing
            && game.player.pos.y < 160.0
            && !point(game, game.player.pos, "oWater");
        let timer = value(game, "bubble_timer");
        game.step(&InputFrame {
            up: true,
            up_pressed: frame == 0,
            ..Default::default()
        });
        safe(game);
        assert_eq!(game.player.state, PlayerState::Climbing);
        assert_eq!(
            value(game, "swimming"),
            f64::from(wet),
            "SOURCE_CLIMBING_RETAINS_INCOMING_SWIMMING"
        );
        if wet {
            assert_eq!(value(game, "bubble_timer"), bubble_next(timer));
        } else {
            assert_eq!(value(game, "bubble_timer"), 20.0);
        }
        if was_dry_climbing {
            assert!(!point(game, game.player.pos, "oWater"));
            assert!(!point(game, game.player.pos, "oWeb"));
            assert!(game.player.pos.y < 160.0);
            return;
        }
    }
    panic!("fixed ordinary Up tape did not reach dry climbing");
}

#[test]
fn ordinary_pool_exit_keeps_swimming_through_climbing_end_step_then_clears_on_jump() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = room(rules, true);
        climb_out(&mut game, true);
        let mut wrapped = false;
        for _ in 0..24 {
            let timer = value(&game, "bubble_timer");
            wrapped |= timer == 0.0;
            assert_eq!(value(&game, "swimming"), 1.0);
            assert!(!point(&game, game.player.pos, "oWater"));
            game.step(&InputFrame::default());
            safe(&game);
            assert_eq!(game.player.state, PlayerState::Climbing);
            assert_eq!(value(&game, "swimming"), 1.0);
            assert_eq!(value(&game, "bubble_timer"), bubble_next(timer));
            assert_eq!(value(&game, "fall_timer"), 0.0);
            assert_eq!(value(&game, "fall_ticks"), 0.0);
        }
        assert!(
            wrapped,
            "latched own prelude must reach its 21-Step bubble cycle"
        );
        let timer = value(&game, "bubble_timer");
        game.step(&InputFrame {
            jump: true,
            jump_pressed: true,
            ..Default::default()
        });
        safe(&game);
        assert_eq!(game.player.state, PlayerState::Jumping);
        assert!(game.player.vel.y < 0.0);
        assert!(!point(&game, game.player.pos, "oWater"));
        assert_eq!(
            value(&game, "bubble_timer"),
            bubble_next(timer),
            "own prelude consumes incoming swimming before Character departure"
        );
        assert_eq!(
            value(&game, "swimming"),
            0.0,
            "dry non-Climbing Character branch clears in the departure Step"
        );
        game.step(&InputFrame::default());
        safe(&game);
        assert_eq!(value(&game, "bubble_timer"), 20.0);
        assert_eq!(value(&game, "swimming"), 0.0);
    }
}

#[test]
fn ordinary_dry_ladder_never_acquires_swimming_or_runs_the_latched_bubble_cycle() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = room(rules, false);
        climb_out(&mut game, false);
        for _ in 0..24 {
            game.step(&InputFrame::default());
            safe(&game);
            assert_eq!(game.player.state, PlayerState::Climbing);
            assert_eq!(value(&game, "swimming"), 0.0);
            assert_eq!(value(&game, "bubble_timer"), 20.0);
        }
        assert!(!game.world.entities.iter().any(|e| e.object == "oBubble"));
    }
}

#[test]
fn configured_climbing_snap_samples_water_at_the_late_origin_after_departure() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut level = level::Level::default();
        level.set_tile(4, 4, '1').unwrap(); // Raw Solid x80..95/y80..95.
        level.set_tile(6, 4, '1').unwrap(); // Raw Solid x112..127/y80..95.
        for x in 0..40 {
            level.set_tile(x, 5, '1').unwrap();
        }
        level.set_tile(5, 3, '@').unwrap(); // Marker cannot overwrite pool walls.
        let mut game = Game::shell(42, rules);
        game.load_custom(&level, "configured-swimming-snap-phase", "");
        game.spawn("oLadderOrange", Vec2::new(96.0, 80.0));
        game.spawn("oWaterSwim", Vec2::new(105.0, 80.0));
        // Explicitly configured incoming controller phase, before any Step.
        // This is not a claimed naturally produced off-center Climbing pose.
        game.player.pos = Vec2::new(105.0, 88.0);
        game.player.previous_pos = game.player.pos;
        game.player.state = PlayerState::Climbing;
        game.player.vel = Vec2::default();
        game.player.data.insert("swimming".into(), 1.0);
        game.player.data.insert("bubble_timer".into(), 3.0);
        assert!(point(&game, Vec2::new(105.0, 88.0), "oWater"));
        assert!(!point(&game, Vec2::new(104.0, 88.0), "oWater"));
        // Initial-only source premises: pool cannot drain in Game318 and
        // neither the configured nor snapped character body overlaps a Solid.
        for containment in [
            Vec2::new(89.0, 80.0),
            Vec2::new(121.0, 80.0),
            Vec2::new(105.0, 96.0),
        ] {
            assert!(point(&game, containment, "oSolid"));
        }
        for left in [100.0, 99.0] {
            assert!(!game.world.rectangle_family_except(
                Vec2::new(left, 80.0),
                Vec2::new(left + 9.0, 95.0),
                "oSolid",
                None,
                CollisionPrecision::Precise,
            ));
        }
        assert!(game.world.rectangle_family_except(
            Vec2::new(100.0, 96.0),
            Vec2::new(109.0, 96.0),
            "oSolid",
            None,
            CollisionPrecision::Precise,
        ));
        // isCollisionWaterTop(1) samples tb-1=79, not body-top80.
        // Water begins at80, so the supported normal-jump branch is selected.
        assert!(!game.world.rectangle_family_except(
            Vec2::new(100.0, 79.0),
            Vec2::new(109.0, 79.0),
            "oWater",
            None,
            CollisionPrecision::Precise,
        ));
        game.step(&InputFrame {
            jump: true,
            jump_pressed: true,
            ..Default::default()
        });
        safe(&game);
        // Source245 lands before339's normal jump sets FALLING370. The
        // sign check596 sees zero incoming yVel, before acceleration889.
        // Thus the first upward frame is FALLING; the next may be JUMPING.
        assert_eq!(game.player.state, PlayerState::Falling);
        assert_eq!(game.player.vel, Vec2::new(0.0, -4.0));
        assert_eq!(game.player.pos, Vec2::new(104.0, 84.0));
        assert_eq!(value(&game, "ladder_timer"), 5.0);
        assert_eq!(game.player.jump_time, 1);
        assert_eq!(game.engine_time(), 2);
        assert!(
            game.events.iter().any(|event| {
                matches!(event, GameEvent::Sound { name, .. } if name == "sndJump")
            })
        );
        assert_eq!(value(&game, "bubble_timer"), 2.0);
        assert_eq!(
            value(&game, "swimming"),
            0.0,
            "SOURCE_LATE_FRICTION_QUERIES_SNAPPED_ORIGIN_NOT_ENTRY_CACHE"
        );
        assert!(!point(&game, game.player.pos, "oWater"));
    }
}
