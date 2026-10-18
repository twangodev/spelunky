//! oTombLord Step counter ordering, driven by default actors and ordinary Steps.
use spelunky_core::{Game, InputFrame, RulesPreset, RunState, Vec2, level::Level};

fn room(canonical: bool, rules: RulesPreset) -> (Game, u64) {
    let mut level = Level {
        music: "TEMPLE".into(),
        ..Default::default()
    };
    level.set_tile(6, 7, '@').unwrap();
    level.set_tile(13, 6, 'T').unwrap();
    for x in 0..spelunky_core::level::LEVEL_WIDTH {
        level.set_tile(x, 8, '1').unwrap();
    }
    // Source custom-loader border is +16. This low cover ends at world x208
    // and catches attack flies while the player stands below it.
    for x in 0..=11 {
        level.set_tile(x, 6, '1').unwrap();
    }
    let mut game = if canonical {
        Game::shell(42, rules)
    } else {
        Game::legacy_shell(42, rules)
    };
    game.load_custom(&level, "tomb-timer-order", "");
    let lord = game
        .world
        .entities
        .iter()
        .find(|e| e.object == "oTombLord")
        .unwrap()
        .id;
    let e = game.world.entity(lord).unwrap();
    assert_eq!(e.pos, Vec2::new(224.0, 112.0));
    assert_eq!((e.state, e.timer, e.health, e.facing), (0, 0, 20, 1));
    (game, lord)
}

fn step_healthy(game: &mut Game, lord: u64) {
    game.step(&InputFrame::default());
    assert_eq!(game.state, RunState::Playing);
    assert_eq!(
        game.player.health, 4.0,
        "authored cover keeps player healthy"
    );
    assert_eq!(game.world.entity(lord).unwrap().health, 20);
}

#[test]
fn natural_attack_end_idle_countdown_transitions_on_the_zero_step() {
    for canonical in [false, true] {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            let (mut game, lord) = room(canonical, rules);
            let mut attacked = false;
            let mut recovered = false;
            for _ in 0..180 {
                let incoming = game.world.entity(lord).unwrap().state;
                step_healthy(&mut game, lord);
                let e = game.world.entity(lord).unwrap();
                attacked |= e.state == 3;
                if incoming == 3 && e.state == 0 {
                    assert!(attacked);
                    assert_eq!(e.timer, 30, "natural Attack AnimationEnd producer");
                    assert_eq!(e.image_speed(), 0.25);
                    recovered = true;
                    break;
                }
            }
            assert!(recovered, "default Tomb Lord must finish a natural attack");
            for expected in (1..30).rev() {
                let pos = game.world.entity(lord).unwrap().pos;
                step_healthy(&mut game, lord);
                let e = game.world.entity(lord).unwrap();
                assert_eq!((e.state, e.timer), (0, expected));
                assert_eq!(e.pos, pos, "positive IDLE countdown does not move");
                assert_eq!(e.vel.x, 0.0);
            }
            let pos = game.world.entity(lord).unwrap().pos;
            step_healthy(&mut game, lord);
            let e = game.world.entity(lord).unwrap();
            assert_eq!(e.timer, 0);
            assert_eq!(
                e.state, 1,
                "SOURCE_TOMB_IDLE_DECREMENT_THEN_ZERO_TRANSITION"
            );
            assert_eq!(e.vel.x, 0.0, "selected IDLE arm does not also execute WALK");
            assert_eq!(e.pos, pos);
            assert_eq!(e.sprite, "sTombLordWalkL");
            step_healthy(&mut game, lord);
            let e = game.world.entity(lord).unwrap();
            assert_eq!((e.state, e.timer, e.vel.x), (1, 0, -1.0));
            assert_eq!(
                e.pos, pos,
                "first selected WALK chooses next-frame velocity"
            );
        }
    }
}

#[test]
fn natural_turn_end_walk_decrements_counter_before_selecting_attack() {
    for canonical in [false, true] {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            let (mut game, lord) = room(canonical, rules);
            let mut turned = false;
            for _ in 0..120 {
                let incoming = game.world.entity(lord).unwrap().state;
                step_healthy(&mut game, lord);
                let e = game.world.entity(lord).unwrap();
                if incoming == 2 && e.state == 1 {
                    assert_eq!((e.timer, e.facing), (30, -1));
                    assert_eq!(e.value("attack_cooldown"), 0.0);
                    turned = true;
                    break;
                }
            }
            assert!(
                turned,
                "default right-facing Tomb Lord must naturally turn to player"
            );
            step_healthy(&mut game, lord);
            let e = game.world.entity(lord).unwrap();
            assert_eq!(e.state, 3, "natural WALK selects ATTACK");
            assert_eq!(e.timer, 29, "SOURCE_TOMB_WALK_COUNTER_BEFORE_ATTACK_GUARD");
            assert_eq!(e.vel.x, 0.0);
            assert_eq!(e.sprite, "sTombLordAttackL");
        }
    }
}

#[test]
fn controlled_initial_stunned_selects_walk_without_executing_its_body() {
    // Secondary callback selection control: one deliberate initial status is
    // authored before the first Step; this is not a natural stun acquisition.
    for canonical in [false, true] {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            let (mut game, lord) = room(canonical, rules);
            game.world.entity_mut(lord).unwrap().state = 98;
            step_healthy(&mut game, lord);
            let e = game.world.entity(lord).unwrap();
            assert_eq!(e.state, 1, "SOURCE_TOMB_STUNNED_SELECTS_WALK_WITHOUT_BODY");
            assert_eq!(e.timer, 0);
            assert_eq!(e.vel.x, 2.5);
            assert_eq!(e.sprite, "sTombLordWalkR");
        }
    }
}

#[test]
fn natural_default_create_idle_retains_velocity_for_the_following_move() {
    for canonical in [false, true] {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            let (mut game, lord) = room(canonical, rules);
            assert_eq!(game.world.entity(lord).unwrap().vel.x, 2.5);
            step_healthy(&mut game, lord);
            let first = game.world.entity(lord).unwrap();
            assert_eq!((first.state, first.timer), (1, 0));
            // Literal moveTo: floor(2.5) plus one on an even oGame clock.
            let dx = 2.0 + f64::from(game.engine_time() % 2 == 0);
            assert_eq!(first.pos.x, 224.0 + dx);
            assert_eq!(first.vel.x, 2.5, "SOURCE_TOMB_IDLE_RETAINS_CREATE_VELOCITY");
            let x = first.pos.x;
            step_healthy(&mut game, lord);
            let second = game.world.entity(lord).unwrap();
            let dx = 2.0 + f64::from(game.engine_time() % 2 == 0);
            assert_eq!(
                second.pos.x,
                x + dx,
                "retained velocity moves before TURN selection"
            );
            assert_eq!((second.state, second.timer, second.vel.x), (2, 30, 2.5));
            let x = second.pos.x;
            step_healthy(&mut game, lord);
            let third = game.world.entity(lord).unwrap();
            let dx = 2.0 + f64::from(game.engine_time() % 2 == 0);
            assert_eq!(
                third.pos.x,
                x + dx,
                "TURN moves once with the incoming WALK velocity before stopping"
            );
            assert_eq!((third.state, third.timer, third.vel.x), (2, 30, 0.0));
        }
    }
}
