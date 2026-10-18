//! Prepared source fixtures: no execution or whole-frame RNG claim yet.
//! Arrow producer and airborne-prefix witness use an authored room and real input.
use spelunky_core::*;

fn blood(game: &Game) -> usize {
    game.world
        .entities
        .iter()
        .filter(|e| e.object == "oBlood")
        .count()
}

fn diagnostic(game: &Game) -> String {
    format!(
        "tick={} pos={:?} vel={:?} hp={} state={:?} sprite={} stun={} blood={} bounced={:?}",
        game.tick,
        game.player.pos,
        game.player.vel,
        game.player.health,
        game.player.state,
        game.player.sprite,
        game.player.stun,
        blood(game),
        game.player.data.get("bounced")
    )
}

fn authored_hang_room(rules: RulesPreset, bloodless: bool) -> Game {
    let mut level = level::Level::default();
    level.set_tile(10, 8, '@').unwrap();
    for x in 0..40 {
        level.set_tile(x, 9, '1').unwrap();
    }
    for x in 11..=13 {
        for y in 7..=8 {
            level.set_tile(x, y, '1').unwrap();
        }
    }
    level.set_tile(6, 7, '>').unwrap();
    let mut game = Game::shell(42, rules);
    game.load_custom(&level, "arrow-player-blood", "");
    game.player
        .data
        .insert("bloodless".into(), f64::from(bloodless));
    game
}

#[test]
fn natural_trap_arrow_hurts_hanging_player_with_three_prefix_blood_or_bloodless_zero() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for bloodless in [false, true] {
            let mut game = authored_hang_room(rules, bloodless);
            for _ in 0..4 {
                game.step(&InputFrame::default());
            }
            assert!(
                game.player.grounded,
                "FIXTURE_GROUND: {}",
                diagnostic(&game)
            );
            assert_eq!(game.player.health, 4.0);
            assert_eq!(blood(&game), 0);
            let mut observed_arrow = false;
            let mut trace = Vec::new();
            let mut hit = false;
            for tick in 0..60 {
                let prior_hang = game.player.state == PlayerState::Hanging;
                game.step(&InputFrame {
                    right: true,
                    jump: true,
                    jump_pressed: tick == 0,
                    ..Default::default()
                });
                trace.push(diagnostic(&game));
                observed_arrow |= game.world.entities.iter().any(|e| e.object == "oArrow");
                if game.player.health < 4.0 {
                    assert!(
                        observed_arrow && prior_hang,
                        "FIXTURE_TRAP_HANG: {trace:#?}"
                    );
                    assert_eq!(game.player.health, 2.0, "first Arrow hit: {trace:#?}");
                    assert_eq!(game.player.stun, 20);
                    assert_eq!(
                        game.player.data.get("bounced").copied().unwrap_or(0.0),
                        0.0,
                        "FIXTURE_AIRBORNE: late bottom script must not add another three: {trace:#?}"
                    );
                    assert_eq!(
                        blood(&game),
                        if bloodless { 0 } else { 3 },
                        "CONTRACT_ARROW_PREFIX: {trace:#?}"
                    );
                    assert_eq!(
                        game.events
                            .iter()
                            .filter(
                                |e| matches!(e, GameEvent::Sound { name, .. } if name == "sndHurt")
                            )
                            .count(),
                        1
                    );
                    hit = true;
                    break;
                }
                assert_eq!(blood(&game), 0, "FIXTURE_NO_EARLIER_BLOOD: {trace:#?}");
                assert_eq!(
                    game.state,
                    RunState::Playing,
                    "FIXTURE_SURVIVAL: {trace:#?}"
                );
            }
            assert!(hit, "FIXTURE_TRAP_HIT_TIMEOUT: {trace:#?}");
        }
    }
}

fn initial_arrow_room(
    rules: RulesPreset,
    floor: bool,
    prior_dead: bool,
    count: usize,
    bloodless: bool,
) -> Game {
    let mut game = Game::shell(71, rules);
    game.world.name = "rLevel".into();
    for x in 0..game.world.width {
        game.world.set_tile(x, 10, "oBlock");
    }
    game.player.pos = Vec2::new(104.0, if floor { 152.0 } else { 100.0 });
    game.player.previous_pos = game.player.pos;
    game.player.state = if prior_dead {
        PlayerState::Dead
    } else if floor {
        PlayerState::Standing
    } else {
        PlayerState::Falling
    };
    game.player.grounded = floor;
    game.player.health = if prior_dead { 0.0 } else { 4.0 };
    if prior_dead {
        game.state = RunState::Dead;
    }
    game.player.data.insert("bounced".into(), 0.0);
    game.player.data.insert("wall_hurt".into(), 0.0);
    game.player.data.insert("wallHurt".into(), 0.0);
    game.player
        .data
        .insert("bloodless".into(), f64::from(bloodless));
    for _ in 0..count {
        game.spawn("oBone", Vec2::new(200.0, 80.0));
    }
    let id = game.spawn(
        "oArrow",
        Vec2::new(game.player.pos.x - 4.0, game.player.pos.y - 4.0),
    );
    game.world.entity_mut(id).unwrap().vel.x = 12.0;
    game
}

#[test]
fn initial_arrow_grounded_hurt_preserves_separate_prefix_and_late_bottom_script_caps() {
    // An explicitly initialized source-default projectile is distinct from the
    // authored natural producer above. No actors are edited after ticks begin.
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for count in [0, 15, 16] {
            for bloodless in [false, true] {
                let mut game = initial_arrow_room(rules, true, false, count, bloodless);
                assert_eq!(
                    game.world
                        .entities
                        .iter()
                        .filter(|e| e.alive
                            && e.engine_active
                            && catalog::is_a(&e.object, "oDetritus"))
                        .count(),
                    count
                );
                game.step(&InputFrame::default());
                assert_eq!(game.player.health, 2.0, "{}", diagnostic(&game));
                assert_eq!(game.player.stun, 20);
                assert_eq!(
                    game.player.data.get("bounced").copied(),
                    Some(1.0),
                    "late independent bottom branch"
                );
                let expected = if bloodless {
                    0
                } else {
                    (16_usize.saturating_sub(count)).min(6)
                };
                assert_eq!(
                    blood(&game),
                    expected,
                    "Arrow script3 THEN independent late-bottom script3: {}",
                    diagnostic(&game)
                );
                assert!(
                    !game
                        .world
                        .entities
                        .iter()
                        .any(|e| e.alive && e.object == "oArrow")
                );
                assert_eq!(
                    game.events
                        .iter()
                        .filter(|e| matches!(e, GameEvent::Sound { name, .. } if name == "sndHurt"))
                        .count(),
                    1
                );
            }
        }
    }
}

#[test]
fn initial_airborne_corpse_arrow_still_hurts_without_reducing_nonpositive_health() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for bloodless in [false, true] {
            let mut game = initial_arrow_room(rules, false, true, 0, bloodless);
            game.step(&InputFrame::default());
            assert_eq!(game.player.health, 0.0);
            assert_eq!(
                game.player.stun,
                20,
                "Arrow has no dead gate: {}",
                diagnostic(&game)
            );
            assert_eq!(blood(&game), if bloodless { 0 } else { 3 });
            assert!(
                !game
                    .world
                    .entities
                    .iter()
                    .any(|e| e.alive && e.object == "oArrow")
            );
            assert_eq!(game.player.data.get("bounced").copied().unwrap_or(0.0), 0.0);
            assert_eq!(
                game.events
                    .iter()
                    .filter(|e| matches!(e, GameEvent::Sound { name, .. } if name == "sndHurt"))
                    .count(),
                1
            );
        }
    }
}
