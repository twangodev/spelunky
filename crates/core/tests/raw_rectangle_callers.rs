//! Source isCollisionSolid and isCollisionLadder through ordinary Game.step.
//! Default-Create actors; no actor state or velocity assignments after Create.
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
    let id = game.spawn("oSnake", Vec2::new(112.0, 64.0));
    let actor = game.world.entity(id).unwrap();
    assert_eq!(actor.vel, Vec2::new(2.5, 0.0));
    assert_eq!(actor.health, 1);
    assert_eq!(actor.state, 0);
    assert!(!actor.held);
    assert_eq!(actor.bounds(), Rect::new(114.0, 64.0, 12.0, 16.0));
    let mut expected_block_y: f64 = 80.0;
    let mut expected_y: f64 = 64.0;
    let mut expected_vy: f64 = 0.0;
    for age in 1_u64..=14 {
        // Source oGame Create sets time1, then gameStepEvent increments it.
        let source_time = age + 1;
        // oGame318 applies the initial falling displacement; PushBlock467
        // sinks before Snake552. Preserve that same-frame query schedule.
        if sinking {
            expected_block_y += 0.05;
        }
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
        for &block in &blocks {
            let block = game.world.entity(block).expect("natural support survives");
            assert!(
                (block.pos.y - expected_block_y).abs() < 1e-10,
                "SOURCE_SUPPORT_SINK {rules:?}/{age}"
            );
        }
        let actor = game.world.entity(id).expect("natural Snake survives");
        assert!(
            actor.pos.x >= 80.0 && actor.pos.x <= 160.0,
            "SOURCE_WIDE_SUPPORT {rules:?}/{age}/{:?}",
            actor.pos
        );
        assert_eq!(actor.health, 1, "SOURCE_NO_LAVA_BODY {rules:?}/{age}");
        assert_eq!(game.player.health, 4);
        assert_eq!(
            actor.pos.y, expected_y,
            "SOURCE_SOLID_RAW_RECTANGLE_PLACEMENT {rules:?}/{age}"
        );
        assert!(
            (actor.vel.y - expected_vy).abs() < 1e-10,
            "SOURCE_SOLID_RAW_RECTANGLE_VELOCITY {rules:?}/{age}: actual {} expected {}",
            actor.vel.y,
            expected_vy
        );
    }
    assert_eq!(expected_y, if sinking { 65.0 } else { 64.0 });
}

#[test]
fn natural_default_snake_uses_raw_solid_rectangle_above_sinking_support() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        run_support(rules, false); // Opaque integer compatibility first.
        run_support(rules, true);
    }
}

fn stationary_ladder_mask(entity: &Entity) -> catalog::CollisionMask {
    assert_eq!(entity.object, "oLadderOrange");
    assert_eq!(entity.sprite, "sLadder");
    assert_eq!(entity.data.get("image_xscale").copied().unwrap_or(1.0), 1.0);
    assert_eq!(entity.data.get("image_yscale").copied().unwrap_or(1.0), 1.0);
    assert_eq!(entity.data.get("image_angle").copied().unwrap_or(0.0), 0.0);
    catalog::instance_mask(
        &entity.object,
        &entity.sprite,
        entity.pos,
        entity.animation,
        Vec2::new(1.0, 1.0),
        0.0,
    )
}

fn monkey_ladder_recovery(rules: RulesPreset, centered: bool) {
    let mut level = level::Level::default();
    level.set_tile(8, 4, '@').unwrap();
    level.set_tile(8, 5, '1').unwrap();
    let mut game = Game::shell(42, rules);
    game.load_custom(&level, "monkey-counter-expiry-raw-ladder", "");
    let ladder_x = if centered { 80.0 } else { 64.0 };
    let ladders: Vec<_> = (1..=31)
        .map(|row| game.spawn("oLadderOrange", Vec2::new(ladder_x, f64::from(row) * 16.0)))
        .collect();
    let id = game.spawn("oMonkey", Vec2::new(176.0, 64.0));
    let monkey = game.world.entity(id).unwrap();
    assert_eq!(monkey.state, 5, "original default HANG");
    assert_eq!(monkey.health, 1);
    assert_eq!(monkey.vel, Vec2::default());
    assert_eq!(monkey.bounds(), Rect::new(180.0, 70.0, 8.0, 10.0));
    assert_eq!(game.player.pos, Vec2::new(152.0, 88.0));
    assert_eq!(game.player.health, 4);
    let contact_tick = if centered { 32 } else { 36 };
    for age in 1..=contact_tick {
        let before = game.world.entity(id).expect("default Monkey survives");
        if age == contact_tick {
            let contact_ladder = game
                .world
                .entity(ladders[if centered { 13 } else { 15 }])
                .unwrap();
            assert!(contact_ladder.alive && contact_ladder.engine_active);
            assert_eq!(
                contact_ladder.pos,
                Vec2::new(ladder_x, if centered { 224.0 } else { 256.0 })
            );
            assert_eq!(contact_ladder.animation, 0.0);
            let _ = stationary_ladder_mask(contact_ladder);
            assert!(
                !before
                    .sprite_bounds()
                    .intersects(game.player.sprite_bounds()),
                "SOURCE_RECOVER_NO_PLAYER_CONTACT_BEFORE"
            );
            assert_eq!(before.state, 2, "SOURCE_RECOVER_ENTRY {rules:?}/{centered}");
            assert_eq!(
                before.value("vine_counter"),
                if centered { 1.0 } else { 0.0 }
            );
            assert_eq!(
                before.pos,
                if centered {
                    Vec2::new(89.0, 206.0)
                } else {
                    Vec2::new(77.0, 246.0)
                }
            );
            assert_eq!(before.vel, Vec2::new(-3.0, 10.0));
        }
        game.step(&InputFrame {
            left: age <= 8,
            right: age > 8 && age <= 16,
            ..InputFrame::default()
        });
        let monkey = game.world.entity(id).expect("natural Monkey survives");
        assert_eq!(monkey.health, 1);
        assert!(monkey.alive && monkey.engine_active);
        assert!(!monkey.held);
        assert_eq!(game.player.health, 4);
        assert_eq!(game.state, RunState::Playing);
        assert_eq!(game.engine_time(), age as u64 + 1);
        if age == 1 {
            assert_eq!(monkey.state, 6, "SOURCE_DEFAULT_HANG_TO_CLIMB");
        }
        if age == 2 {
            assert_eq!(monkey.state, 1, "SOURCE_NEARBY_CLIMB_TO_BOUNCE");
            assert_eq!(monkey.value("vine_counter"), 30.0);
            assert_eq!(monkey.vel, Vec2::new(-3.0, -3.0));
        }
        if age == 3 {
            assert_eq!(monkey.state, 2, "SOURCE_AIRBORNE_BOUNCE_TO_RECOVER");
        }
        if age >= 2 {
            assert_eq!(
                monkey.value("vine_counter"),
                f64::from((32 - age).max(0)),
                "SOURCE_NATURAL_COOLDOWN {rules:?}/{centered}/{age}"
            );
        }
        if age >= 3 && age < contact_tick {
            assert_eq!(
                monkey.state, 2,
                "SOURCE_PRECONTACT_RECOVER {rules:?}/{centered}/{age}"
            );
        }
        if age != contact_tick {
            continue;
        }
        // Source RECOVER changes velocity/state/timer and final sprite only.
        // moveTo has finished; no later position/bounds writer intervenes.
        let expected_pos = if centered {
            Vec2::new(86.0, 216.0)
        } else {
            Vec2::new(74.0, 256.0)
        };
        assert_eq!(monkey.pos, expected_pos, "SOURCE_NATURAL_RECOVER_POSITION");
        assert!(
            !monkey
                .sprite_bounds()
                .intersects(game.player.sprite_bounds()),
            "SOURCE_RECOVER_NO_PLAYER_CONTACT_AFTER"
        );
        let b = monkey.bounds();
        assert_eq!(
            b,
            Rect::new(expected_pos.x + 4.0, expected_pos.y + 6.0, 8.0, 10.0)
        );
        let floor_y = (b.y + b.h).round_ties_even();
        assert!(
            !game.world.solid_line_except(
                Vec2::new(b.x.round_ties_even(), floor_y),
                Vec2::new((b.x + b.w - 1.0).round_ties_even(), floor_y),
                id
            ),
            "SOURCE_RECOVER_GROUND_FIRST_CLEAR"
        );
        let first = Vec2::new(b.x + 8.0, b.y + 8.0);
        let second = Vec2::new(b.x + b.w - 8.0, b.y + b.h - 8.0);
        assert_eq!(
            first,
            Vec2::new(expected_pos.x + 12.0, expected_pos.y + 14.0)
        );
        assert_eq!(
            second,
            Vec2::new(expected_pos.x + 4.0, expected_pos.y + 8.0)
        );
        let center = Vec2::new(monkey.pos.x + 8.0, monkey.pos.y + 8.0);
        let mut source_hit = false;
        let mut old_point = false;
        for (index, &ladder) in ladders.iter().enumerate() {
            let target = game
                .world
                .entity(ladder)
                .expect("authored stationary ladder survives");
            assert_eq!(target.pos, Vec2::new(ladder_x, (index + 1) as f64 * 16.0));
            assert!(target.alive);
            if target.engine_active {
                let mask = stationary_ladder_mask(target);
                source_hit |= mask.intersects_rectangle(first, second, CollisionPrecision::Precise);
                old_point |= mask.contains(center, CollisionPrecision::BoundingBox);
            }
        }
        // Original sLadder opaque/cache x=anchor+1..anchor+14. The raw-only
        // case includes x78 (rightmost column); center82 is outside. Both
        // reversed Y endpoints264..270 lie within the row256..271 ladder.
        assert!(source_hit, "SOURCE_RAW_LADDER_PRECISE_PREMISE");
        assert_eq!(old_point, centered, "SOURCE_CENTER_VS_RAW_BAND");
        assert_eq!(monkey.value("vine_counter"), 0.0);
        assert_eq!(
            monkey.state, 5,
            "SOURCE_MONKEY_RAW_LADDER_RECOVERY {rules:?}/{centered}/{age}"
        );
        assert_eq!(monkey.vel, Vec2::default());
        assert!((10..=40).contains(&monkey.timer));
        assert_eq!(monkey.sprite, "sMonkeyHangL");
    }
}

#[test]
fn natural_monkey_recovery_queries_its_reversed_raw_ladder_band() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        monkey_ladder_recovery(rules, true); // Existing centered contact after cooldown.
        monkey_ladder_recovery(rules, false); // Same ordinary tape, raw-only outer column.
    }
}
