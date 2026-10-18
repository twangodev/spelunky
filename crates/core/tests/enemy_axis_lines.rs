//! Default-Create Caveman and movable supports; ordinary neutral Game.step.
//! The Y oracle follows source moveTo/time and the independently executed GM8
//! cached-bbox line kernel. No actor state or velocity is assigned after Create.
use spelunky_core::*;

fn run_support(rules: RulesPreset, sinking: bool) {
    let mut level = level::Level::default();
    level.set_tile(14, 8, '@').unwrap();
    level.set_tile(14, 9, '1').unwrap();
    let mut game = Game::shell(42, rules);
    game.load_custom(&level, "enemy-axis-source-lines", "");
    let mut blocks = Vec::new();
    for x in [64.0, 80.0, 96.0, 112.0, 128.0, 144.0, 160.0, 176.0] {
        if !sinking {
            game.world.set_tile((x / 16.0) as usize, 6, "oBlock");
        }
        blocks.push(game.spawn(
            "oPushBlock",
            Vec2::new(x, if sinking { 79.0 } else { 80.0 }),
        ));
        if sinking {
            game.spawn("oLava", Vec2::new(x, 80.0));
        }
    }
    let id = game.spawn("oCaveman", Vec2::new(112.0, 64.0));
    let actor = game.world.entity(id).unwrap();
    assert_eq!(actor.vel, Vec2::new(2.5, 0.0));
    assert_eq!(actor.health, 3);
    assert_eq!(actor.state, 0);
    assert!(!actor.held);
    assert_eq!(actor.bounds(), Rect::new(114.0, 64.0, 12.0, 16.0));
    let mut expected_block_y: f64 = 80.0;
    let mut expected_y: f64 = 64.0;
    let mut expected_vy: f64 = 0.0;
    for age in 1_u64..=14 {
        // Source oGame Create sets time1, then gameStepEvent increments it.
        let source_time = age + 1;
        // oGame318 applies the initial falling displacement before Caveman325;
        // PushBlock467 performs its .05 lava sink after the Caveman Step.
        let top = expected_block_y.round_ties_even();
        let frac = expected_vy.abs().fract();
        let extra = frac > 0.0 && source_time.is_multiple_of((1.0 / frac).round_ties_even() as u64);
        let dy = expected_vy.abs().floor() as u32 + u32::from(extra);
        for _ in 0..dy {
            // Source bottom1 line includes y+16, and the source mask starts at
            // independently rounded top. All walking positions remain over
            // contiguous full masks, so horizontal location cannot change it.
            if (expected_y + 16.0).round_ties_even() >= top {
                break;
            }
            expected_y += 1.0;
        }
        expected_vy = (expected_vy + 0.6).min(10.0);
        if (expected_y + 16.0).round_ties_even() >= top {
            expected_vy = 0.0;
        }
        game.step(&InputFrame::default());
        assert_eq!(
            game.engine_time(),
            source_time,
            "SOURCE_CLOCK {rules:?}/{age}"
        );
        if sinking {
            expected_block_y += 0.05;
        }
        for &block in &blocks {
            let block = game.world.entity(block).expect("natural support survives");
            assert!(
                (block.pos.y - expected_block_y).abs() < 1e-10,
                "SOURCE_SUPPORT_SINK {rules:?}/{age}"
            );
        }
        let actor = game.world.entity(id).expect("natural Caveman survives");
        assert!(
            actor.pos.x >= 80.0 && actor.pos.x <= 160.0,
            "SOURCE_WIDE_SUPPORT {rules:?}/{age}/{:?}",
            actor.pos
        );
        assert_eq!(actor.health, 3, "SOURCE_NO_LAVA_BODY {rules:?}/{age}");
        assert_eq!(game.player.health, 4.0);
        assert_eq!(
            actor.pos.y, expected_y,
            "SOURCE_ENEMY_AXIS_PLACEMENT {rules:?}/{age}"
        );
        assert!(
            (actor.vel.y - expected_vy).abs() < 1e-10,
            "SOURCE_ENEMY_AXIS_VELOCITY {rules:?}/{age}: actual {} expected {}",
            actor.vel.y,
            expected_vy
        );
    }
    assert_eq!(expected_y, if sinking { 65.0 } else { 64.0 });
}

#[test]
fn natural_caveman_descends_at_source_fractional_support_classic() {
    run_support(RulesPreset::Classic, true);
}
#[test]
fn natural_caveman_descends_at_source_fractional_support_assisted() {
    run_support(RulesPreset::Assisted, true);
}
#[test]
fn integer_default_caveman_keeps_opaque_support() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        run_support(rules, false);
    }
}

#[test]
fn natural_caveman_clamps_wall_jump_with_earlier_top_contact() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for roof in [false, true] {
            let mut level = level::Level::default();
            level.set_tile(14, 8, '@').unwrap();
            level.set_tile(14, 9, '1').unwrap();
            let mut game = Game::shell(42, rules);
            game.load_custom(&level, "caveman-cached-top", "");
            game.world.set_tile(7, 6, "oBlock");
            game.world.set_tile(8, 4, "oBrick");
            if roof {
                game.world.set_tile(7, 3, "oBrick");
            }
            let block = game.spawn("oPushBlock", Vec2::new(112.0, 80.0));
            let id = game.spawn("oCaveman", Vec2::new(112.0, 64.0));
            assert_eq!(game.world.entity(id).unwrap().vel, Vec2::new(2.5, 0.0));
            game.step(&InputFrame::default());
            let actor = game.world.entity(id).expect("source corner survives");
            // Source clock2 produces dx3; the last reduced-strip probe at128
            // hits the right wall, leaving origin114. Its raw top point130,64
            // requests a wall jump; cached colTop at y63 cancels it if roofed.
            assert_eq!(actor.pos, Vec2::new(114.0, 64.0));
            assert_eq!(actor.health, 3);
            assert_eq!(game.player.health, 4.0);
            assert_eq!(
                game.world.entity(block).unwrap().pos,
                Vec2::new(112.0, 80.0)
            );
            assert_eq!(
                actor.vel.y,
                if roof { 0.0 } else { -6.0 },
                "SOURCE_CAVEMAN_CACHED_TOP_CLAMP {rules:?}/{roof}"
            );
        }
    }
}
