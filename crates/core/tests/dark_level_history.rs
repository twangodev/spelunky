//! Positioned source door/tally interactions, not full campaign traversal.
//! History, elapsed time and requested room are initial fixture preconditions.
//! Ordinary InputFrame edges retain the generated room and its live hazards.
mod common;

use common::complete_source_exit;
use spelunky_core::{Game, InputFrame, RulesPreset, RunState, Vec2};

fn requested_room(game: &mut Game, level: u32) {
    game.data.insert("next_level".into(), f64::from(level));
    game.next_level();
    assert_eq!(game.world.level, level);
    assert_eq!(game.state, RunState::Playing);
}

fn positioned_mines_tally(seed: u32, elapsed_ms: f64) -> Game {
    let mut game = Game::new(seed, RulesPreset::Classic);
    requested_room(&mut game, 4);
    // Prior dark Mines history and elapsed time are initial state, not claims
    // that this test played the first four levels or waited 30 seconds.
    game.flags.insert("had_dark_level".into());
    game.data.insert("xtime".into(), elapsed_ms);
    let exit = game
        .world
        .entities
        .iter()
        .find(|entity| {
            entity.alive && entity.object == "oExit" && entity.bounds().contains(game.world.exit)
        })
        .expect("generated Mines exit");
    let bounds = exit.bounds();
    let position = Vec2::new(bounds.x + bounds.w / 2.0, bounds.y + bounds.h / 2.0);
    assert!(
        !game.world.solid_at(exit.pos.x, exit.pos.y),
        "fixture exit gate"
    );
    game.player.pos = position;
    game.player.previous_pos = position;
    game.player.vel = Vec2::default();
    game.player.grounded = false;
    game.update_camera();
    game.previous_camera = game.camera;
    game.step(&InputFrame::default());
    assert_eq!(game.state, RunState::Playing);
    for _ in 0..10 {
        game.step(&InputFrame {
            up: true,
            ..Default::default()
        });
        if game.state != RunState::Playing {
            break;
        }
    }
    assert_eq!(game.state, RunState::Ending, "generated exit must start");
    complete_source_exit(&mut game);
    assert_eq!(game.world.level, 4);
    assert!(game.flags.contains("had_dark_level"));
    game
}

#[test]
fn positioned_mines_exit_can_roll_dark_in_next_area_despite_prior_history() {
    // No-history control is independently configured from the same actual
    // tally result. This does not force darkness or change its probability.
    let mut dark_witness = None;
    for seed in 0..128 {
        let mut history = positioned_mines_tally(seed, 30_000.0);
        assert!(!history.flags.contains("no_dark_level"));
        let mut control = history.clone();
        control.flags.remove("had_dark_level");
        control.next_level();
        history.next_level();
        assert_eq!(history.world.level, 5);
        assert_eq!(control.world.level, 5);
        assert_eq!(history.world.dark, control.world.dark, "seed {seed}");
        assert_eq!(history.rng.algorithm(), control.rng.algorithm());
        assert_eq!(history.rng.state_bits(), control.rng.state_bits());
        assert_eq!(
            serde_json::to_value(&history.world).unwrap(),
            serde_json::to_value(&control.world).unwrap()
        );
        assert_eq!(
            history.flags.contains("had_dark_level"),
            control.flags.contains("had_dark_level")
        );
        if history.world.dark {
            history.step(&InputFrame::default());
            assert!(history.flags.contains("had_dark_level"));
            dark_witness = Some(seed);
            break;
        }
    }
    assert!(
        dark_witness.is_some(),
        "128 transitions need a dark witness"
    );
}

#[test]
fn same_area_public_transitions_keep_history_and_suppress_second_dark_room() {
    for destination in [6, 14] {
        let mut eligible_witness = None;
        for seed in 0..128 {
            let mut retained = Game::new(seed, RulesPreset::Classic);
            // Direct public transition control, not a natural route claim.
            retained.flags.remove("no_dark_level");
            retained.flags.insert("had_dark_level".into());
            let mut eligible = retained.clone();
            eligible.flags.remove("had_dark_level");
            requested_room(&mut eligible, destination);
            requested_room(&mut retained, destination);
            assert!(!retained.world.dark);
            assert!(retained.flags.contains("had_dark_level"));
            if eligible.world.dark {
                eligible_witness = Some(seed);
                break;
            }
        }
        assert!(eligible_witness.is_some(), "positive control {destination}");
    }
}

#[test]
fn every_source_area_entry_clears_history_even_when_darkness_is_disabled() {
    for destination in [1, 5, 9, 13] {
        let mut game = Game::new(16, RulesPreset::Classic);
        game.flags.insert("had_dark_level".into());
        assert!(game.flags.contains("no_dark_level"));
        requested_room(&mut game, destination);
        assert!(
            !game.flags.contains("had_dark_level"),
            "entry {destination}"
        );
        assert!(game.flags.contains("no_dark_level"));
        assert!(!game.world.dark);
    }
}

#[test]
fn standalone_generation_applies_the_same_area_entry_reset() {
    for destination in [5, 13] {
        let mut witness = false;
        for seed in 0..128 {
            let mut prior_flags = std::collections::BTreeSet::new();
            prior_flags.insert("had_dark_level".into());
            let empty_flags = std::collections::BTreeSet::new();
            let mut prior_rng = spelunky_core::Rng::new(seed);
            let mut fresh_rng = spelunky_core::Rng::new(seed);
            let prior =
                spelunky_core::generation::generate(&mut prior_rng, destination, &prior_flags);
            let fresh =
                spelunky_core::generation::generate(&mut fresh_rng, destination, &empty_flags);
            assert_eq!(prior.dark, fresh.dark);
            assert_eq!(prior_rng.state_bits(), fresh_rng.state_bits());
            assert!(prior_flags.contains("had_dark_level"));
            if fresh.dark {
                witness = true;
                break;
            }
        }
        assert!(witness, "standalone positive control {destination}");
    }
}

#[test]
fn ordinary_fast_exit_preserves_speedrun_darkness_gate_after_area_reset() {
    let mut game = positioned_mines_tally(0, 0.0);
    assert!(game.flags.contains("no_dark_level"));
    game.next_level();
    assert_eq!(game.world.level, 5);
    assert!(!game.flags.contains("had_dark_level"));
    assert!(game.flags.contains("no_dark_level"));
    assert!(!game.world.dark);
}

#[test]
fn custom_and_living_hub_loads_preserve_history_while_shortcuts_restart() {
    let mut game = Game::new(16, RulesPreset::Classic);
    game.flags.insert("had_dark_level".into());
    let level = spelunky_core::level::Level::default();
    game.load_custom(&level, "first", "NONE");
    assert!(game.flags.contains("had_dark_level"));
    game.continue_custom(&level, "second", "first");
    assert!(game.flags.contains("had_dark_level"));
    game.load_hub(spelunky_core::hub::HubKind::Title);
    assert!(game.flags.contains("had_dark_level"));
    for destination in [5, 9, 13] {
        game.flags.insert("had_dark_level".into());
        game.start_from_hub(false, destination);
        assert_eq!(game.world.level, destination);
        assert!(game.flags.contains("used_shortcut"));
        assert!(!game.flags.contains("had_dark_level"));
        assert!(game.flags.contains("no_dark_level"));
        assert!(!game.world.dark);
    }
}
