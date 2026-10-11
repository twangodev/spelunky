//! Controlled source-state Game.step regressions, UNCOMPILED/UNEXECUTED.
//! All actor/state setup precedes the first Step; subsequent writes are inputs
//! only. The last test uses an authored room Rock and ordinary pickup/throw.
use spelunky_core::*;

fn rock_room(rules: RulesPreset, authored_rock: bool) -> Game {
    let mut level = level::Level::default();
    level.set_tile(10, 8, '@').unwrap();
    if authored_rock {
        level.set_tile(10, 8, 'r').unwrap();
    }
    // The editor has one cell per object; retain a distinct actual entrance
    // beside the authored Rock and walk Left before pickup in that control.
    if authored_rock {
        level.set_tile(11, 8, '@').unwrap();
    }
    for x in 0..40 {
        level.set_tile(x, 9, '1').unwrap();
    }
    let mut game = Game::shell(6001, rules);
    game.load_custom(&level, "rock-player-blood", "");
    game
}

fn controlled_rock(rules: RulesPreset, vx: f64, safe: bool) -> (Game, u64) {
    let mut game = rock_room(rules, false);
    game.player.pos = Vec2::new(184.0, 80.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Falling;
    game.player.grounded = false;
    game.player.health = 6;
    let rock = game.spawn("oRock", game.player.pos);
    let entity = game.world.entity_mut(rock).unwrap();
    entity.vel.x = vx;
    entity
        .data
        .insert("safe".into(), if safe { 1.0 } else { 0.0 });
    (game, rock)
}

fn blood_count(game: &Game) -> usize {
    game.world
        .entities
        .iter()
        .filter(|entity| entity.object == "oBlood")
        .count()
}

#[test]
fn rock_authoritative_own_step_contact_births_three_and_keeps_rock_in_both_directions() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for vx in [-8.0, 8.0] {
            for bloodless in [false, true] {
                let (mut game, rock) = controlled_rock(rules, vx, false);
                game.player
                    .data
                    .insert("bloodless".into(), if bloodless { 1.0 } else { 0.0 });
                game.player.invincible = 60; // Source Rock ignores contact invulnerability.
                game.step(&InputFrame::default());
                assert_eq!(game.player.health, 4, "actual Rock own-Step damage");
                assert_eq!(game.player.stun, 20);
                assert_eq!(game.player.vel, Vec2::new(vx, -4.0));
                assert_eq!(blood_count(&game), if bloodless { 0 } else { 3 });
                assert!(game.world.entity(rock).is_some(), "Rock must survive");
                assert_eq!(game.events.iter().filter(|event|
                    matches!(event, GameEvent::Sound { name, .. } if name == "sndHurt")).count(), 1);
            }
        }
    }
    // Newborn particles may execute later source events in the same frame;
    // this test deliberately makes no whole-frame RNG/velocity assertion for them.
}

#[test]
fn rock_actual_step_safe_speed_existing_stun_and_dead_controls_do_not_emit_blood() {
    for control in ["safe", "speed4", "speed_minus4", "stunned", "dead"] {
        let vx = match control {
            "speed4" => 4.0,
            "speed_minus4" => -4.0,
            _ => 8.0,
        };
        let (mut game, rock) = controlled_rock(RulesPreset::Classic, vx, control == "safe");
        if control == "stunned" {
            game.player.stun = 20;
            game.player.state = PlayerState::Stunned;
        }
        if control == "dead" {
            game.player.health = 0;
            game.player.state = PlayerState::Dead;
            game.state = RunState::Dead;
        }
        let health = game.player.health;
        game.step(&InputFrame::default());
        assert_eq!(game.player.health, health, "{control}");
        assert_eq!(blood_count(&game), 0, "{control}");
        assert!(game.world.entity(rock).is_some());
        assert!(
            !game
                .events
                .iter()
                .any(|event| matches!(event, GameEvent::Sound { name, .. } if name == "sndHurt")),
            "{control}"
        );
    }
}

#[test]
fn rock_mitt_catch_suppresses_hurt_only_with_empty_hands() {
    for occupied in [false, true] {
        let (mut game, rock) = controlled_rock(RulesPreset::Classic, 8.0, false);
        game.player.equipment.insert("Mitt".into());
        if occupied {
            let jar = game.spawn("oJar", game.player.pos);
            game.world.entity_mut(jar).unwrap().held = true;
            game.player.held = Some(jar);
            game.player.pickup_item_type = "Jar".into();
        }
        game.step(&InputFrame::default());
        if occupied {
            assert_eq!(game.player.health, 4);
            assert_eq!(game.player.stun, 20);
            assert_eq!(blood_count(&game), 3);
        } else {
            assert_eq!(game.player.health, 6);
            assert_eq!(game.player.stun, 0);
            assert_eq!(blood_count(&game), 0);
            assert_eq!(game.player.held, Some(rock));
            assert!(game.world.entity(rock).unwrap().held);
            assert_eq!(game.player.pickup_item_type, "Rock");
        }
        assert!(game.world.entity(rock).is_some());
    }
}

#[test]
fn rock_authored_pickup_and_ordinary_throw_set_safety_without_emitting_blood() {
    let mut game = rock_room(RulesPreset::Classic, true);
    for _ in 0..4 {
        game.step(&InputFrame::default());
    }
    assert!(
        game.player.grounded,
        "FIXTURE: natural entrance must settle"
    );
    let rock = game
        .world
        .entities
        .iter()
        .find(|entity| entity.object == "oRock")
        .unwrap()
        .id;
    // Adjacent editor cells put the player16px from the Rock; ordinary walk
    // brings its sprite into the pickup rectangle without actor relocation.
    for _ in 0..5 {
        game.step(&InputFrame {
            left: true,
            ..Default::default()
        });
    }
    game.step(&InputFrame {
        down: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert_eq!(
        game.player.held,
        Some(rock),
        "FIXTURE: actual pickup required"
    );
    game.step(&InputFrame::default());
    let health = game.player.health;
    game.step(&InputFrame {
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert_eq!(
        game.player.held, None,
        "FIXTURE: actual ordinary throw required"
    );
    let thrown = game.world.entity(rock).unwrap();
    assert!(!thrown.held && thrown.value("safe") != 0.0);
    assert!(thrown.vel.x.abs() > 4.0, "real fast throw");
    assert_eq!(game.player.health, health);
    assert_eq!(game.player.stun, 0);
    assert_eq!(blood_count(&game), 0);
    // No unsafe rebound/self-hit claim: original safety countdown plus wall
    // attenuation makes that a separate trajectory requiring runtime proof.
}
