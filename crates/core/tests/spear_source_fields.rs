//! Authored spear scenes: no generated campaign route is claimed.
use spelunky_core::*;

fn room(rules: RulesPreset, object: &str) -> (Game, u64) {
    let mut game = Game::shell(71, rules);
    game.world = World::new(40, 24, Area::Jungle, 5);
    game.world.name = "rLevel".into();
    for x in 0..40 {
        game.world.set_tile(x, 7, "oHardBlock");
    }
    game.player.pos = Vec2::new(120.0, 104.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    let trap = game.spawn(object, Vec2::new(160.0, 96.0));
    (game, trap)
}

fn spears(game: &Game) -> Vec<&Entity> {
    game.world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oSpearsLeft")
        .collect()
}

#[test]
fn spear_step_preserves_fractional_countdown_and_configured_proximity() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for object in ["oSpearTrapTop", "oSpearTrapBottom", "oSpearTrapLit"] {
            for (field, value, expected_fired) in [("fired", 0.5, -0.5), ("prox", 0.0, 0.0)] {
                let (mut game, trap) = room(rules, object);
                game.world
                    .entity_mut(trap)
                    .unwrap()
                    .data
                    .insert(field.into(), value);
                game.step(&InputFrame::default());
                assert_eq!(game.player.pos, Vec2::new(120.0, 104.0));
                assert_eq!(game.player.health, 4);
                assert!(
                    spears(&game).is_empty(),
                    "{object}/{field}: source requires fired == 0 and a strict configured proximity"
                );
                assert_eq!(
                    game.world.entity(trap).unwrap().value("fired"),
                    expected_fired
                );
                assert_eq!(game.world.entity(trap).unwrap().value("range"), 64.0);
            }
        }
    }
}

#[test]
fn spear_step_uses_fired_max_and_still_assigns_final_nearest_scratch() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for object in ["oSpearTrapTop", "oSpearTrapBottom", "oSpearTrapLit"] {
            let (mut game, trap) = room(rules, object);
            let rock = game.spawn("oRock", Vec2::new(240.0, 104.0));
            game.world
                .entity_mut(trap)
                .unwrap()
                .data
                .insert("firedMax".into(), 3.0);
            game.step(&InputFrame::default());
            assert_eq!(game.player.pos, Vec2::new(120.0, 104.0));
            assert_eq!(game.player.health, 4);
            assert_eq!(spears(&game).len(), 1);
            assert_eq!(spears(&game)[0].pos, Vec2::new(144.0, 96.0));
            let receiver = game.world.entity(trap).unwrap();
            assert_eq!(
                receiver.value("fired"),
                3.0,
                "source copies the actual firedMax field"
            );
            assert_eq!(
                receiver.value("obj"),
                rock as f64,
                "the final Item lookup runs after an earlier Player branch fired"
            );
            assert_eq!(receiver.value("range"), 64.0);
            game.step(&InputFrame::default());
            assert_eq!(game.world.entity(trap).unwrap().value("fired"), 2.0);
            assert_eq!(spears(&game).len(), 1);
        }
    }
}

#[test]
fn zero_fired_max_allows_independent_player_and_moveable_branches() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for object in ["oSpearTrapTop", "oSpearTrapBottom", "oSpearTrapLit"] {
            let (mut game, trap) = room(rules, object);
            let block = game.spawn("oPushBlock", Vec2::new(144.0, 96.0));
            game.world
                .entity_mut(trap)
                .unwrap()
                .data
                .insert("firedMax".into(), 0.0);
            game.step(&InputFrame::default());
            assert_eq!(game.player.health, 4);
            assert_eq!(
                game.world.entity(block).unwrap().pos,
                Vec2::new(144.0, 96.0)
            );
            assert_eq!(
                spears(&game).len(),
                2,
                "source Player and stationary MoveableSolid branches are independent"
            );
            assert!(
                spears(&game)
                    .iter()
                    .all(|e| e.pos == Vec2::new(144.0, 96.0))
            );
            assert_eq!(game.world.entity(trap).unwrap().value("fired"), 0.0);
        }
    }
}
