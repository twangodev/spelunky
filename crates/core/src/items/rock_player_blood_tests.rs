//! Source producer-phase oracle. UNCOMPILED/UNEXECUTED.
//! These private calls isolate Rock's own Player-Step query from later engine
//! events; the separate integration tests exercise authoritative Game.step.
use super::*;

fn rock_source_fixture(health: i32) -> (Game, u64) {
    let mut game = Game::shell(6001, RulesPreset::Classic);
    game.world = World::new(40, 30, Area::Mines, 1);
    game.player.pos = Vec2::new(184.0, 80.0);
    game.player.health = health;
    game.player.state = PlayerState::Standing;
    let rock = game.spawn("oRock", game.player.pos);
    game.world.entity_mut(rock).unwrap().vel.x = 8.0;
    (game, rock)
}

#[test]
fn rock_literal_script_cap_bloodless_and_four_create_draws_per_birth() {
    // Source scrCreateBlood checks <16 before EACH synchronous Create.
    for (active, inactive, unrelated, bloodless, expected_count) in [
        (0, 0, 0, false, 3),
        (14, 0, 0, false, 2),
        (15, 0, 0, false, 1),
        (16, 0, 0, false, 0),
        (0, 20, 0, false, 3),
        (15, 12, 0, false, 1),
        (0, 0, 20, false, 3),
        (0, 0, 0, true, 0),
    ] {
        let (mut game, rock) = rock_source_fixture(6);
        for _ in 0..active {
            game.spawn("oBone", Vec2::new(400.0, 80.0));
        }
        for _ in 0..inactive {
            let id = game.spawn("oBone", Vec2::new(400.0, 80.0));
            game.world.entity_mut(id).unwrap().engine_active = false;
        }
        for _ in 0..unrelated {
            game.spawn("oRubbleSmall", Vec2::new(400.0, 80.0));
        }
        game.player
            .data
            .insert("bloodless".into(), if bloodless { 1.0 } else { 0.0 });
        let origin = game.player.pos;
        let mut oracle = game.rng.clone();
        let expected: Vec<_> = (0..expected_count)
            .map(|_| {
                // Literal oBlood Create: random4 - random4; -1-random2;
                // rand1..6*0.1. No producer spread or fifth RNG call exists.
                let velocity = Vec2::new(
                    oracle.unit() * 4.0 - oracle.unit() * 4.0,
                    -1.0 - oracle.unit() * 2.0,
                );
                let gravity = f64::from(1 + oracle.range(6)) * 0.1;
                (velocity, gravity)
            })
            .collect();
        rock_player_contact(&mut game);
        let blood: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|e| e.object == "oBlood")
            .collect();
        assert_eq!(blood.len(), expected_count);
        for (entity, (velocity, gravity)) in blood.iter().zip(expected) {
            assert_eq!(entity.pos, origin);
            assert_eq!(entity.vel, velocity);
            assert_eq!(entity.value("gravity"), gravity);
            assert_eq!(&entity.alarms[..3], &[1, 1, 5]);
            assert_eq!(entity.image_speed(), 0.3);
        }
        assert_eq!(
            game.rng.next_u64(),
            oracle.next_u64(),
            "literal producer/Create RNG continuation"
        );
        assert_eq!(game.player.health, 4);
        assert_eq!(game.player.vel, Vec2::new(8.0, -4.0));
        assert_eq!(game.player.stun, 20);
        assert!(
            game.world.entity(rock).is_some(),
            "Rock survives source contact"
        );
        assert_eq!(
            game.events
                .iter()
                .filter(|event| matches!(event, GameEvent::Sound { name, .. } if name == "sndHurt"))
                .count(),
            1
        );
    }
}

#[test]
fn rock_zero_hp_without_prior_dead_still_creates_blood_but_does_not_subtract_again() {
    let (mut game, rock) = rock_source_fixture(0);
    rock_player_contact(&mut game);
    assert_eq!(game.player.health, 0);
    assert_eq!(game.player.stun, 20);
    assert_eq!(
        game.world
            .entities
            .iter()
            .filter(|e| e.object == "oBlood")
            .count(),
        3
    );
    assert!(game.world.entity(rock).is_some());
    assert!(
        !game
            .events
            .iter()
            .any(|event| matches!(event, GameEvent::Damage { .. }))
    );
}
