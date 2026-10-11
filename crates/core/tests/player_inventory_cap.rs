//! Source Player Step23–25: late grants may exceed99 until the next callback.
//! Main regressions use official custom-level supply tiles and ordinary input.
use spelunky_core::{Game, InputFrame, PlayerState, RulesPreset, RunState, level::Level};

const PRESETS: [RulesPreset; 2] = [RulesPreset::Classic, RulesPreset::Assisted];

fn supply_room(rules: RulesPreset, symbol: char, initial: i32) -> Game {
    let mut level = Level::default();
    level.starting_bombs = initial;
    level.starting_ropes = initial;
    level.set_tile(4, 7, '@').unwrap();
    level.set_tile(5, 7, symbol).unwrap();
    for x in 0..40 {
        level.set_tile(x, 8, '1').unwrap();
    }
    let mut game = Game::shell(17, rules);
    game.load_custom(&level, "source-99-cap", "");
    assert_eq!((game.player.bombs, game.player.ropes), (initial, initial));
    game
}

fn acquire_supply(game: &mut Game, object: &str) {
    let id = game
        .world
        .entities
        .iter()
        .find(|e| e.object == object)
        .unwrap()
        .id;
    let supply = game.world.entity(id).unwrap();
    assert_eq!(supply.value("cost"), 0.0);
    assert_eq!(supply.value("forSale"), 0.0);
    assert!(!supply.held);
    for _ in 0..48 {
        game.step(&InputFrame {
            right: true,
            ..Default::default()
        });
        if game.world.entity(id).is_none() {
            assert!(game.player.held.is_none());
            assert_eq!(game.state, RunState::Playing);
            return;
        }
    }
    panic!("FIXTURE_SUPPLY_NOT_ACQUIRED {object}");
}

#[test]
fn bomb_box_grant_precedes_next_player_cap_and_real_bomb_use() {
    for rules in PRESETS {
        for initial in [98, 10] {
            let mut game = supply_room(rules, ':', initial);
            acquire_supply(&mut game, "oBombBox");
            assert_eq!(
                game.player.bombs,
                initial + 12,
                "CAP_GRANT_MUST_NOT_BE_EARLY"
            );
            let first_birth = game.world.next_id;
            game.step(&InputFrame {
                bomb_pressed: true,
                ..Default::default()
            });
            assert_eq!(
                game.player.bombs,
                (initial + 12).min(99) - 1,
                "CAP_BOMB_BEFORE_NEXT_USE"
            );
            assert_eq!(game.player.ropes, initial);
            assert!(
                game.world
                    .entities
                    .iter()
                    .any(|e| e.id >= first_birth && e.object == "oBomb"),
                "FIXTURE_REAL_BOMB_USE_REQUIRED"
            );
        }
    }
}

#[test]
fn rope_pile_grant_precedes_next_player_cap_and_real_rope_use() {
    for rules in PRESETS {
        for initial in [98, 10] {
            let mut game = supply_room(rules, 'R', initial);
            acquire_supply(&mut game, "oRopePile");
            assert_eq!(
                game.player.ropes,
                initial + 3,
                "CAP_ROPE_GRANT_MUST_NOT_BE_EARLY"
            );
            let first_birth = game.world.next_id;
            game.step(&InputFrame {
                rope_pressed: true,
                ..Default::default()
            });
            assert_eq!(
                game.player.ropes,
                (initial + 3).min(99) - 1,
                "CAP_ROPE_BEFORE_NEXT_USE"
            );
            assert_eq!(game.player.bombs, initial);
            assert!(
                game.world.entities.iter().any(|e| e.id >= first_birth
                    && matches!(e.object.as_str(), "oRopeThrow" | "oRope" | "oRopeTop")),
                "FIXTURE_REAL_ROPE_USE_REQUIRED"
            );
        }
    }
}

#[test]
fn source_cap_health_and_inventory_upper_boundary_does_not_cap_arrows() {
    // Initial upper-boundary control supplements real tile-grant/use cases.
    for rules in PRESETS {
        let mut game = supply_room(rules, '0', 4);
        game.player.health = 101;
        game.player.bombs = 101;
        game.player.ropes = 101;
        game.data.insert("arrows".into(), 123.0);
        game.step(&InputFrame::default());
        assert_eq!(
            (game.player.health, game.player.bombs, game.player.ropes),
            (99, 99, 99),
            "CAP_UPPER_BOUNDARY"
        );
        assert_eq!(game.data["arrows"], 123.0);
    }
}

#[test]
fn source_cap_preserves_negative_corpse_health_inventory_and_arrows() {
    // This separate control runs on both old and candidate arms. Already-dead
    // authored state avoids the unrelated first-death health=0 assignment.
    for rules in PRESETS {
        let mut game = supply_room(rules, '0', 4);
        game.player.health = -7;
        game.player.bombs = -3;
        game.player.ropes = -9;
        game.data.insert("arrows".into(), 123.0);
        game.state = RunState::Dead;
        game.player.state = PlayerState::Dead;
        game.step(&InputFrame::default());
        assert_eq!(
            (game.player.health, game.player.bombs, game.player.ropes),
            (-7, -3, -9),
            "CAP_LOWER_VALUES_UNCHANGED"
        );
        assert_eq!(game.data["arrows"], 123.0);
    }
}
