//! Proposed integration regressions for the private Jetpack refuel increment.
//! Source review only: this file has not been built or executed.
//! Source Player Step: lava -> owned-Jetpack refuel -> pit -> active controller.
use spelunky_core::{Game, InputFrame, PlayerState, RulesPreset, level::Level};

fn fuel(game: &Game) -> f64 {
    game.player.data.get("jetpack_fuel").copied().unwrap_or(0.0)
}

fn source_grounded(state: PlayerState) -> bool {
    matches!(
        state,
        PlayerState::Standing
            | PlayerState::Running
            | PlayerState::Ducking
            | PlayerState::LookingUp
    )
}

fn initial_floor_and_pack(rules: RulesPreset) -> Game {
    let mut level = Level::default();
    level.set_tile(4, 3, '@').unwrap();
    for x in 0..40 {
        level.set_tile(x, 4, '1').unwrap();
    }
    let mut game = Game::shell(7, rules);
    game.load_custom(&level, "jetpack-refuel-order", "");
    let pack = game.spawn("oJetpack", game.player.pos);
    assert_eq!(game.world.entity(pack).unwrap().value("cost"), 20000.0);
    // No injected cost, equipment, position, controller state, or fuel. Item
    // Step naturally clears the source shop price when no keeper exists.
    for _ in 0..8 {
        game.step(&InputFrame::default());
    }
    assert!(source_grounded(game.player.state));
    assert!(game.player.grounded);
    assert!(!game.player.has("Jetpack"));
    assert_eq!(game.world.entity(pack).unwrap().value("cost"), 0.0);
    game
}

#[test]
fn natural_grounded_jetpack_pickup_refills_on_the_following_step() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = initial_floor_and_pack(rules);
        // The old controller precharged every unequipped grounded player.
        assert_eq!(fuel(&game), 0.0);
        game.step(&InputFrame {
            down: true,
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert!(game.player.has("Jetpack"));
        assert!(source_grounded(game.player.state));
        // scrStealItem runs later than this Step's owned-Jetpack refuel test.
        assert_eq!(fuel(&game), 0.0);
        game.step(&InputFrame::default());
        assert_eq!(fuel(&game), 50.0);
    }
}

// Airborne first acquisition remains a pending source-valid witness. Source
// Player Step 1219-1245 selects whip before pickup when state is not DUCKING
// or DUCKTOHANG, even with Down held. The port's existing falling-Cape test
// permits Down+attack pickup in that situation and is not a source oracle.
#[test]
fn naturally_owned_jetpack_consumes_fuel_in_air_and_refills_after_landing() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = initial_floor_and_pack(rules);
        assert_eq!(fuel(&game), 0.0);
        game.step(&InputFrame {
            down: true,
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert!(game.player.has("Jetpack"));
        assert_eq!(fuel(&game), 0.0);
        game.step(&InputFrame::default());
        assert_eq!(fuel(&game), 50.0);
        game.step(&InputFrame {
            jump: true,
            jump_pressed: true,
            ..Default::default()
        });
        assert!(!source_grounded(game.player.state));
        assert_eq!(
            fuel(&game),
            50.0,
            "a fresh ground jump does not activate thrust"
        );
        game.step(&InputFrame {
            jump_released: true,
            ..Default::default()
        });
        game.step(&InputFrame {
            jump: true,
            jump_pressed: true,
            ..Default::default()
        });
        assert_eq!(fuel(&game), 49.0);
        let mut landed = false;
        for _ in 0..60 {
            game.step(&InputFrame::default());
            assert_eq!(fuel(&game), 49.0, "first landing Step still began airborne");
            if source_grounded(game.player.state) {
                landed = true;
                break;
            }
        }
        assert!(landed);
        game.step(&InputFrame::default());
        assert_eq!(fuel(&game), 50.0);
    }
}
