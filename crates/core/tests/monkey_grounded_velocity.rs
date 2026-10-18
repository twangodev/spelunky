//! SOURCE-ONLY PROPOSAL: UNCOMPILED/UNRUN. Default Monkey Create, authored room
//! and initial grounded Player, then ordinary inputs only. No ledge claim.
use spelunky_core::*;

fn scene(rules: RulesPreset, canonical: bool) -> (Game, u64) {
    let mut game = if canonical {
        Game::shell(42, rules)
    } else {
        Game::legacy_shell(42, rules)
    };
    game.world = World::new(40, 20, Area::Jungle, 1);
    game.world.name = "rLevel".into();
    for x in 0..40 {
        game.world.set_tile(x, 10, "oHardBlock");
    }
    // Authored initial Player setup, not a claim about stock Player Create.
    game.player.pos = Vec2::new(136.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.player.facing = 1;
    game.world.entrance = game.player.pos;
    let id = game.spawn("oMonkey", Vec2::new(80.0, 144.0));
    let monkey = game.world.entity(id).unwrap();
    assert_eq!((monkey.health, monkey.state, monkey.timer), (1, 5, 0));
    assert_eq!(monkey.vel, Vec2::default());
    assert!(!monkey.held);
    (game, id)
}

fn ordinary_tape(tick: usize) -> InputFrame {
    InputFrame {
        right: tick < 20,
        run: tick < 20,
        right_released: tick == 20,
        ..Default::default()
    }
}

fn supported(game: &Game, e: &Entity) -> bool {
    // Source Monkey Create movement bounds (4,6,12,16), then exact bottom line.
    game.world.solid_line_except(
        Vec2::new(
            (e.pos.x + 4.0).round_ties_even(),
            (e.pos.y + 16.0).round_ties_even(),
        ),
        Vec2::new(
            (e.pos.x + 11.0).round_ties_even(),
            (e.pos.y + 16.0).round_ties_even(),
        ),
        e.id,
    )
}

fn active_and_healthy(game: &Game, id: u64) {
    let e = game.world.entity(id).expect("default Monkey survives");
    assert!(e.alive && e.engine_active && !e.held);
    assert_eq!(e.health, 1);
    assert_ne!(e.state, 7, "ordinary route must never become GRAB");
    assert_eq!(e.value("swimming"), 0.0);
    assert_eq!(e.value("burning"), 0.0);
    assert_eq!(game.player.health, 4.0);
    assert_eq!(game.player.stun, 0);
    assert!(game.player.held.is_none());
    let view = game.simulation_view_rect();
    assert!(e.pos.x > view.x - 20.0 && e.pos.x < view.x + view.w + 4.0);
    assert!(e.pos.y > view.y - 20.0 && e.pos.y < view.y + view.h + 4.0);
}

fn rng_wire(game: &Game) -> serde_json::Value {
    serde_json::to_value(&game.rng).unwrap()
}

#[test]
fn default_monkey_ordinary_bounce_recovers_then_preserves_supported_idle_gravity() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for canonical in [false, true] {
            let (mut game, id) = scene(rules, canonical);
            let mut saw_bounce = false;
            let mut saw_recover = false;
            let mut saw_supported_recovery = false;
            let mut accepted_idle = false;
            for tick in 0..119 {
                active_and_healthy(&game, id);
                let before = game.world.entity(id).unwrap().clone();
                let mut expected_rng = game.rng.clone();
                let expected_landing_counter = 10 + expected_rng.range(31);
                game.step(&ordinary_tape(tick));
                active_and_healthy(&game, id);
                let after = game.world.entity(id).unwrap().clone();
                saw_bounce |= after.state == 1;
                saw_recover |= after.state == 2;
                if before.state != 2 || after.state != 0 || !supported(&game, &after) {
                    continue;
                }
                assert!(
                    saw_bounce && saw_recover,
                    "normal HANG/CLIMB/BOUNCE route prerequisite"
                );
                assert_eq!(
                    after.vel,
                    Vec2::default(),
                    "SOURCE_RECOVER_LANDING_EXPLICIT_ZERO"
                );
                assert_eq!(
                    after.timer, expected_landing_counter,
                    "SOURCE_RECOVER_COUNTER_DRAW"
                );
                assert_eq!(
                    rng_wire(&game),
                    serde_json::to_value(&expected_rng).unwrap()
                );
                saw_supported_recovery = true;
                // A conservative horizontal bound, not an invented exact distance
                // oracle: unit Monkey canvas16 and Player canvas16 put both native
                // bboxes >=64 apart at this >=96 origin separation. The next
                // ordinary IDLE tail therefore remains IDLE, rather than BOUNCE.
                if game.player.pos.x - after.pos.x < 96.0 {
                    continue;
                }
                assert_eq!(after.pos.y, 144.0);
                assert_eq!(after.vel.y, 0.0);
                let prior_rng = rng_wire(&game);
                game.step(&ordinary_tape(tick + 1));
                active_and_healthy(&game, id);
                let idle = game.world.entity(id).unwrap();
                assert!(supported(&game, idle));
                assert_eq!(idle.state, 0, "actual supported IDLE must persist");
                assert_eq!(
                    idle.pos, after.pos,
                    "no stored entry displacement after RECOVER zero"
                );
                assert_eq!(idle.timer, after.timer - 1);
                assert_eq!(
                    idle.value("inherited_gravity"),
                    0.6,
                    "SOURCE_PARENT_DRY_GRAVITY"
                );
                assert_eq!(rng_wire(&game), prior_rng, "SOURCE_IDLE_NO_RANDOM_DRAW");
                // Target behavior oracle comes from Monkey Step34,37-38 and the
                // lack of a generic support reset; source .6 is retained on floor.
                assert_eq!(
                    idle.vel.y, 0.6,
                    "MONKEY_SOURCE_SUPPORTED_IDLE_RETAINS_GRAVITY"
                );
                accepted_idle = true;
                break;
            }
            assert!(
                saw_supported_recovery,
                "UNQUALIFIED_ROUTE: no natural RECOVER landing"
            );
            assert!(
                accepted_idle,
                "UNQUALIFIED_ROUTE: no landing with far Player in 120 ticks"
            );
        }
    }
}

#[test]
fn default_monkey_near_lower_player_naturally_bounces_then_recovers_airborne() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for canonical in [false, true] {
            let (mut game, id) = scene(rules, canonical);
            let prior_rng = rng_wire(&game);
            game.step(&InputFrame::default());
            active_and_healthy(&game, id);
            let climb = game.world.entity(id).unwrap();
            assert_eq!((climb.state, climb.timer), (6, 0));
            assert_eq!(climb.vel, Vec2::default(), "HANG_EXCLUDES_GRAVITY");
            assert_eq!(rng_wire(&game), prior_rng);
            let mut expected_rng = game.rng.clone();
            let hang_counter = 10 + expected_rng.range(31);
            let launch_y = -f64::from(2 + expected_rng.range(3));
            game.step(&InputFrame::default());
            active_and_healthy(&game, id);
            let bounce = game.world.entity(id).unwrap().clone();
            assert_eq!((bounce.state, bounce.timer), (1, hang_counter));
            assert_eq!(bounce.pos, Vec2::new(80.0, 144.0));
            assert_eq!(bounce.vel, Vec2::new(3.0, launch_y));
            assert_eq!(bounce.value("vine_counter"), 30.0);
            assert_eq!(
                rng_wire(&game),
                serde_json::to_value(&expected_rng).unwrap()
            );
            let prior_rng = rng_wire(&game);
            game.step(&InputFrame::default());
            active_and_healthy(&game, id);
            let recover = game.world.entity(id).unwrap();
            assert_eq!(recover.state, 2, "SOURCE_AIRBORNE_BOUNCE_TO_RECOVER");
            assert!(!supported(&game, recover));
            assert_eq!(recover.pos, Vec2::new(83.0, 144.0 + launch_y));
            assert_eq!(recover.vel, Vec2::new(3.0, launch_y + 0.6));
            assert_eq!(rng_wire(&game), prior_rng);
            assert_eq!(
                game.events
                    .iter()
                    .filter(|event| matches!(event,
                GameEvent::Sound { name, .. } if name == "sndMonkey"))
                    .count(),
                1
            );
        }
    }
}
