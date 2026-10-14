//! Configured initial YellowBall phases followed by public ordinary Steps.
//! Stock Olmec produces YellowBalls; these tests do not replay a full fight.
use spelunky_core::*;

fn room(rules: RulesPreset, velocity: Vec2) -> (Game, u64) {
    let mut game = Game::shell(42, rules);
    game.world = World::new(40, 20, Area::Mines, 1);
    // No Level activation controller: inactive initial actors stay inactive.
    game.world.name = "rIsolatedYellowBallStep".into();
    for x in 0..40 {
        game.world.set_tile(x, 10, "oHardBlock");
    }
    game.player.pos = Vec2::new(40.0, 152.0);
    game.player.previous_pos = game.player.pos;
    let id = game.spawn("oYellowBall", Vec2::new(96.0, 96.0));
    let ball = game.world.entity_mut(id).unwrap();
    // Explicitly configured post-Create source velocity, before first input.
    assert_eq!(ball.value("launch"), 0.0, "SYNCHRONOUS_YELLOW_BALL_CREATE");
    ball.vel = velocity;
    (game, id)
}

fn healthy(game: &Game) {
    assert_eq!(game.state, RunState::Playing);
    assert_eq!(game.player.health, 4);
    assert!(!game.player.is_stunned());
}

fn near(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-9, "{actual} vs {expected}");
}

fn literal_selection(state: u64) -> &'static str {
    let first = (state as u32).wrapping_mul(0x08088405).wrapping_add(1);
    let width = if (u64::from(first) * 2) >> 32 == 0 {
        4
    } else {
        5
    };
    let second = first.wrapping_mul(0x08088405).wrapping_add(1);
    let index = ((u64::from(second) * width) >> 32) as usize;
    let selected = ["oBat", "oSpider", "oSnake", "oFrog", "oFireFrog"][index];
    // Earlier-resource newborns retain their literal birth origin this Step.
    assert_ne!(selected, "oFireFrog", "fixture requires a pre-579 newborn");
    selected
}

fn assert_impact(game: &mut Game, id: u64) {
    let expected = literal_selection(game.rng.state_bits());
    let next_id = game.world.next_id;
    game.step(&InputFrame::default());
    healthy(game);
    assert!(game.world.entity(id).is_none(), "SOURCE_YELLOW_BALL_IMPACT");
    let newborn = game.world.entity(next_id + 1).unwrap();
    // Alarm 1 precedes Step and creates the first YellowTrail at next_id.
    assert_eq!(newborn.object, expected);
    assert_eq!(newborn.pos, Vec2::new(88.0, 88.0));
    assert_eq!(
        game.world
            .entities
            .iter()
            .filter(|e| e.alive && catalog::is_a(&e.object, "oEnemy"))
            .count(),
        1
    );
}

#[test]
fn yellow_ball_strict_gravity_can_cross_six_and_preserves_six_or_more() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for incoming in [5.9, 6.0, 6.2] {
            let (mut game, id) = room(rules, Vec2::new(2.0, incoming));
            game.step(&InputFrame::default());
            healthy(&game);
            let ball = game.world.entity(id).unwrap();
            near(ball.pos.x, 98.0);
            near(ball.pos.y, 96.0 + incoming);
            near(
                ball.vel.y,
                if incoming < 6.0 {
                    incoming + 0.15
                } else {
                    incoming
                },
            );
        }
    }
}

#[test]
fn yellow_ball_dynamic_solid_impact_uses_literal_inclusive_rectangle() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for facing in [-1.0, 1.0] {
            let (mut game, id) = room(rules, Vec2::new(2.0 * facing, 5.9));
            // Source-valid off-grid HardBlock; its own Create never snaps x.
            // Right target begins at query right=106; left ends at left=86.
            let x = if facing > 0.0 { 106.0 } else { 71.0 };
            let target = game.spawn("oHardBlock", Vec2::new(x, 96.0));
            let target = game.world.entity(target).unwrap();
            assert!(target.engine_active && target.alive);
            assert!(catalog::is_a(&target.object, "oSolid"));
            assert_eq!(target.pos, Vec2::new(x, 96.0));
            assert!(game.world.tile(6, 6).unwrap().object.is_empty());
            assert_impact(&mut game, id);
        }
    }
}

#[test]
fn yellow_ball_terrain_impact_retains_the_source_enemy_birth_origin() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for facing in [-1.0, 1.0] {
            let (mut game, id) = room(rules, Vec2::new(2.0 * facing, 5.9));
            game.world.set_tile(6, 6, "oHardBlock");
            assert_impact(&mut game, id);
        }
    }
}

#[test]
fn yellow_ball_active_olmec_overlap_suppresses_impact() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, id) = room(rules, Vec2::new(2.0, 5.9));
        let boss = game.spawn("oOlmec", Vec2::new(88.0, 88.0));
        let boss = game.world.entity_mut(boss).unwrap();
        boss.set_sprite("sOlmec");
        boss.state = 0;
        boss.timer = 60;
        boss.data.insert("my_gravity".into(), 0.0);
        game.step(&InputFrame::default());
        healthy(&game);
        let ball = game.world.entity(id).unwrap();
        assert_eq!(ball.pos, Vec2::new(98.0, 101.9));
        near(ball.vel.y, 6.05);
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|e| {
                    e.alive && catalog::is_a(&e.object, "oEnemy") && e.object != "oOlmec"
                })
                .count(),
            0
        );
    }
}

#[test]
fn yellow_ball_inactive_or_retired_olmec_cannot_suppress_solid_impact() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for retired in [false, true] {
            let (mut game, id) = room(rules, Vec2::new(2.0, 5.9));
            game.spawn("oHardBlock", Vec2::new(104.0, 96.0));
            let boss = game.spawn("oOlmec", Vec2::new(88.0, 88.0));
            let boss = game.world.entity_mut(boss).unwrap();
            boss.set_sprite("sOlmec");
            if retired {
                boss.alive = false;
            } else {
                boss.engine_active = false;
            }
            assert_impact(&mut game, id);
        }
    }
}

#[test]
fn yellow_ball_create_runs_three_launch_draws_before_caller_overrides() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = Game::shell(42, rules);
        game.world = World::new(40, 20, Area::Mines, 1);
        game.world.name = "rIsolatedYellowBallCreate".into();
        for x in 0..40 {
            game.world.set_tile(x, 10, "oHardBlock");
        }
        game.player.pos = Vec2::new(40.0, 152.0);
        game.player.previous_pos = game.player.pos;
        let mut state = game.rng.state_bits() as u32;
        state = state.wrapping_mul(0x08088405).wrapping_add(1);
        let vy = -4.0 - f64::from(state) / 4294967296.0 * 3.0;
        state = state.wrapping_mul(0x08088405).wrapping_add(1);
        let mut vx = 2.0 + ((u64::from(state) * 4) >> 32) as f64;
        state = state.wrapping_mul(0x08088405).wrapping_add(1);
        if (u64::from(state) * 2) >> 32 == 0 {
            vx = -vx;
        }
        let id = game.spawn("oYellowBall", Vec2::new(96.0, 96.0));
        let ball = game.world.entity_mut(id).unwrap();
        near(ball.vel.x, vx);
        near(ball.vel.y, vy);
        assert_eq!(ball.text.get("type").map(String::as_str), Some(""));
        assert_eq!(ball.value("blinkToggle"), 0.0);
        assert_eq!(ball.value("launch"), 0.0);
        assert_eq!(ball.alarms[1], 1);
        assert_eq!(game.rng.state_bits(), u64::from(state));
        // A producer override follows Create; the first Step cannot reroll it.
        game.world.entity_mut(id).unwrap().vel = Vec2::new(1.0, 0.0);
        game.step(&InputFrame::default());
        healthy(&game);
        let ball = game.world.entity(id).unwrap();
        assert_eq!(ball.pos, Vec2::new(97.0, 96.0));
        near(ball.vel.y, 0.15);
        assert_eq!(game.rng.state_bits(), u64::from(state));
    }
}

#[test]
fn yellow_ball_alarm_trail_cap_counts_only_active_family_members() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for active in [false, true] {
            let (mut game, id) = room(rules, Vec2::new(1.0, 0.0));
            for _ in 0..12 {
                let trail = game.spawn("oYellowTrail", Vec2::new(300.0, 96.0));
                game.world.entity_mut(trail).unwrap().engine_active = active;
            }
            let next_id = game.world.next_id;
            game.step(&InputFrame::default());
            healthy(&game);
            let ball = game.world.entity(id).unwrap();
            assert_eq!(ball.alarms[1], 4);
            assert_eq!(ball.pos, Vec2::new(97.0, 96.0));
            let born = game.world.entity(next_id);
            if active {
                assert!(born.is_none(), "ACTIVE_TWELVE_SUPPRESS_TRAIL_BIRTH");
            } else {
                let born = born.unwrap();
                assert_eq!(born.object, "oYellowTrail");
                assert_eq!(born.pos, Vec2::new(96.0, 96.0));
                assert!(born.engine_active);
            }
            let trails: Vec<_> = game
                .world
                .entities
                .iter()
                .filter(|e| e.alive && catalog::is_a(&e.object, "oYellowTrail"))
                .collect();
            assert_eq!(trails.len(), if active { 12 } else { 13 });
            assert_eq!(
                trails.iter().filter(|e| e.engine_active).count(),
                if active { 12 } else { 1 }
            );
        }
    }
}
