//! Historical SplitMix layout geometry acceptance starts at the actual generated entrance. Mines uses
//! full Game.step scheduling and physical key edges; Ice keeps its isolated
//! controller route. Enemy/hazard actors are deliberately excluded, so these
//! establish collision-scene reachability rather than complete-run balance.
//! Canonical layout/controller recordings are separate and do not establish original-runtime parity.
use crate::{Game, InputFrame, Rng, RulesPreset, catalog, player};
use std::collections::BTreeSet;

fn traverse_legacy_recording(seed: u64, level: u32, actions: &[u8]) {
    traverse_legacy_steps(seed, level, actions, false);
}

fn traverse_legacy_steps(seed: u64, level: u32, actions: &[u8], full_steps: bool) {
    let mut game = Game::legacy_shell(seed, RulesPreset::Classic);
    game.world = super::generate(&mut Rng::legacy(seed), level, &BTreeSet::new());
    game.world.entities.retain(|entity| {
        entity.solid
            || catalog::is_a(&entity.object, "oLadder")
            || catalog::is_a(&entity.object, "oPlatform")
            || catalog::is_a(&entity.object, "oWater")
            || entity.object == "oExit"
    });
    game.player.pos = game.world.entrance;
    game.player.previous_pos = game.player.pos;
    // The isolated controller still queries the source instance ledger.
    if !full_steps {
        crate::dispatcher::register_instances(&mut game);
    }
    let spawn = game.player.pos;
    let geometry: Vec<_> = game
        .world
        .tiles
        .iter()
        .map(|tile| tile.object.clone())
        .collect();
    let mut previous_input = InputFrame::default();
    for &action in actions {
        for phase in 0..6 {
            let jump = matches!(action, 3 | 4 | 5 | 10 | 11 | 12) && phase < 5;
            let mut input = InputFrame {
                left: matches!(action, 1 | 3 | 6 | 11),
                right: matches!(action, 2 | 4 | 7 | 12),
                up: matches!(action, 6 | 7 | 8 | 10 | 11 | 12),
                down: action == 9,
                jump,
                jump_pressed: jump && phase == 0,
                jump_released: !jump && phase == 5,
                run: true,
                ..InputFrame::default()
            };
            if full_steps {
                input.left_released = previous_input.left && !input.left;
                input.right_released = previous_input.right && !input.right;
                input.up_pressed = input.up && !previous_input.up;
                input.down_pressed = input.down && !previous_input.down;
                input.jump_pressed = input.jump && !previous_input.jump;
                input.jump_released = previous_input.jump && !input.jump;
                game.step(&input);
                assert_eq!(game.state, crate::RunState::Playing);
                assert_eq!(game.player.health, 4);
                assert!(!game.player.is_stunned());
            } else {
                game.tick += 1;
                game.level_tick += 1;
                player::tick(&mut game, &input);
            }
            previous_input = input;
            assert!(
                !game.world.solid_at(game.player.pos.x, game.player.pos.y),
                "seed {seed}, level {level}, tick {} player {:?}",
                game.tick,
                game.player.pos
            );
        }
    }
    if full_steps {
        assert_eq!(
            game.world
                .tiles
                .iter()
                .map(|tile| tile.object.clone())
                .collect::<Vec<_>>(),
            geometry,
            "must traverse the generated geometry without changing its tiles"
        );
    }
    assert!(game.player.pos.distance(spawn) > 128.0);
    assert!(game.player.grounded, "must reach the actual supported exit");
    assert!(
        game.world
            .entities
            .iter()
            .any(|entity| entity.object == "oExit" && entity.bounds().contains(game.player.pos)),
        "seed {seed}, level {level}, pos {:?}, exit {:?}",
        game.player.pos,
        game.world.exit
    );
}

#[test]
fn legacy_recorded_mines_are_traversed_from_the_entrance_with_ordinary_jumps() {
    // Full-Step entrance replay uses physical input edges. Turn right one
    // action earlier than the old recording, which relied on two pixels of
    // upward head intrusion when jumping directly beneath a solid ceiling.
    traverse_legacy_steps(
        0,
        1,
        &[
            6, 7, 9, 3, 3, 1, 1, 1, 1, 0, 2, 9, 6, 1, 1, 1, 1, 1, 0, 9, 3, 2, 2, 1, 9, 1, 1, 2, 3,
            2, 0, 2, 1, 2, 1, 9, 7, 1, 2, 1, 0, 1, 1, 1, 3, 3, 1, 2, 9, 1,
        ],
        true,
    );
}

#[test]
fn legacy_recorded_ice_islands_are_traversed_without_an_artificial_bottom_floor() {
    // Recorded controller route through the source-ordered seed-zero layout.
    traverse_legacy_recording(
        0,
        9,
        &[
            6, 3, 1, 1, 1, 1, 0, 1, 0, 1, 0, 2, 2, 2, 4, 1, 3, 1, 9, 3, 3, 1, 0, 0, 3, 1, 1, 1, 3,
            3, 9, 6, 2, 2,
        ],
    );
}

// Separate canonical Rust layout/controller reachability; original runtime was not executed.
fn traverse_canonical_recording(
    seed: u32,
    level: u32,
    revision: crate::GenerationRevision,
    actions: &[u8],
) {
    let mut game = Game::shell(seed, RulesPreset::Classic);
    game.generation_revision = revision;
    // Keep the original separate generation RNG and untouched Game RNG seed.
    let mut generation_rng = Rng::new(seed);
    let mut generation_state = super::GenerationState::fresh(&generation_rng);
    generation_state.revision = revision;
    game.world = super::generate_with_campaign(
        &mut generation_rng,
        level,
        &BTreeSet::new(),
        super::GenerationContext::default(),
        &mut generation_state,
    );
    game.world.entities.retain(|entity| {
        entity.solid
            || catalog::is_a(&entity.object, "oLadder")
            || catalog::is_a(&entity.object, "oPlatform")
            || catalog::is_a(&entity.object, "oWater")
            || entity.object == "oExit"
    });
    game.player.pos = game.world.entrance;
    game.player.previous_pos = game.player.pos;
    let spawn = game.player.pos;
    let geometry: Vec<_> = game
        .world
        .tiles
        .iter()
        .map(|tile| tile.object.clone())
        .collect();
    let mut previous_input = InputFrame::default();
    for &action in actions {
        for phase in 0..6 {
            let jump = matches!(action, 3 | 4 | 5 | 10 | 11 | 12) && phase < 5;
            let mut input = InputFrame {
                left: matches!(action, 1 | 3 | 6 | 11),
                right: matches!(action, 2 | 4 | 7 | 12),
                up: matches!(action, 6 | 7 | 8 | 10 | 11 | 12),
                down: action == 9,
                jump,
                jump_pressed: jump && phase == 0,
                jump_released: !jump && phase == 5,
                run: true,
                ..InputFrame::default()
            };
            // Run the complete Player Step, including late hurt sprite selection
            // and the following stationary-stun countdown; direct tick omits it.
            input.left_released = previous_input.left && !input.left;
            input.right_released = previous_input.right && !input.right;
            input.up_pressed = input.up && !previous_input.up;
            input.down_pressed = input.down && !previous_input.down;
            input.jump_pressed = input.jump && !previous_input.jump;
            input.jump_released = previous_input.jump && !input.jump;
            game.step(&input);
            previous_input = input;
            assert!(
                !game.world.solid_at(game.player.pos.x, game.player.pos.y),
                "canonical seed {seed}, level {level}, tick {} player {:?}",
                game.tick,
                game.player.pos
            );
            assert!(
                game.player.health > 0 && game.player.state != crate::PlayerState::Dead,
                "ordinary controller route must remain alive"
            );
        }
    }
    assert_eq!(
        game.world
            .tiles
            .iter()
            .map(|tile| tile.object.clone())
            .collect::<Vec<_>>(),
        geometry,
        "must traverse the generated geometry without changing its tiles"
    );
    assert!(game.player.pos.distance(spawn) > 128.0);
    assert!(game.player.grounded, "must reach the actual supported exit");
    assert!(
        game.world
            .entities
            .iter()
            .any(|entity| entity.object == "oExit" && entity.bounds().contains(game.player.pos)),
        "canonical seed {seed}, level {level}, pos {:?}, exit {:?}",
        game.player.pos,
        game.world.exit
    );
}

#[test]
fn canonical_seed_zero_mines_is_traversed_from_the_generated_entrance() {
    // Preserve the original canonical-RNG recording under its historical selector contract.
    // Full Game.step, physical key edges, actor filter and assertions stay unchanged.
    traverse_canonical_recording(
        0,
        1,
        crate::GenerationRevision::HistoricalV1,
        &[
            4, 2, 1, 1, 3, 0, 2, 2, 2, 4, 2, 2, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 4, 2, 2, 4, 0, 2, 4,
            2, 0, 4, 4, 2, 4, 2, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 3, 3, 1, 1, 1,
            3, 3, 0, 0, 1, 0, 1, 1, 0,
        ],
    );
}

#[test]
fn source_classic_seed_zero_mines_is_traversed_from_the_generated_entrance() {
    // Actual SourceClassicV2 seed-zero geometry, with the original tape followed
    // by two left jumps and a right correction onto the newly selected exit ledge.
    // Full Game.step, physical key edges, actor filter and assertions are shared.
    traverse_canonical_recording(
        0,
        1,
        crate::GenerationRevision::SourceClassicV2,
        &[
            4, 2, 1, 1, 3, 0, 2, 2, 2, 4, 2, 2, 1, 1, 1, 1, 1, 2, 2, 2, 2, 2, 4, 2, 2, 4, 0, 2, 4,
            2, 0, 4, 4, 2, 4, 2, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 3, 3, 1, 1, 1,
            3, 3, 0, 0, 1, 0, 1, 1, 0, 3, 3, 2,
        ],
    );
}

#[test]
fn canonical_seed_zero_ice_is_traversed_without_an_artificial_bottom_floor() {
    // Independently discovered canonical route; generated islands and abyss remain intact.
    traverse_canonical_recording(
        0,
        9,
        crate::GenerationRevision::HistoricalV1,
        &[
            3, 1, 2, 2, 0, 2, 3, 0, 2, 2, 4, 2, 2, 0, 1, 1, 9, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 3, 1,
            2, 0,
        ],
    );
}

#[test]
fn source_classic_seed_zero_ice_is_traversed_without_an_artificial_bottom_floor() {
    // Recorded with full Game.step on SourceClassicV2 seed zero. Ordinary
    // falls, recovery waits and jumps reach the unchanged supported exit.
    // The original historical seed-zero tape above remains independent.
    traverse_canonical_recording(
        0,
        9,
        crate::GenerationRevision::SourceClassicV2,
        &[
            4, 4, 4, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 4, 4, 4, 0, 0, 2, 1, 0,
        ],
    );
}
