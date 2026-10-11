//! Shared native/Wasm deterministic diagnostic replays.
use crate::{Game, InputFrame, RngAlgorithm, RulesPreset, RunState, SIMULATION_VERSION};
use serde::Serialize;

pub const MAX_REPLAY_TICKS: u32 = 100_000;
/// Version of the replay payload and diagnostic result envelope.
pub const REPLAY_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReplayResult {
    pub version: u32,
    pub simulation_version: u32,
    pub algorithm: RngAlgorithm,
    /// Decimal text preserves the full legacy seed in JavaScript consumers.
    pub seed: String,
    pub rules: RulesPreset,
    pub requested_ticks: u32,
    /// Actual simulation ticks; a finished run stops advancing.
    pub ticks: u64,
    pub hash: String,
    pub score: u32,
    pub health: i32,
    pub level: u32,
    pub entities: usize,
    pub state: RunState,
}

pub fn scripted_input(t: u32) -> InputFrame {
    InputFrame {
        right: t % 180 < 100,
        left: t % 180 >= 140,
        jump: t % 45 < 10,
        jump_pressed: t % 45 == 0,
        bomb_pressed: t == 90,
        rope_pressed: t == 150,
        attack_pressed: t % 37 == 0,
        up: t % 100 < 5,
        ..InputFrame::default()
    }
}

/// Canonical replay. Panics for an oversized replay.
/// Untrusted hosts should use `try_scripted`.
pub fn scripted(seed: u32, ticks: u32, rules: RulesPreset) -> ReplayResult {
    try_scripted(u64::from(seed), ticks, rules).expect("replay exceeds 100000 ticks")
}

pub fn try_scripted(seed: u64, ticks: u32, rules: RulesPreset) -> Result<ReplayResult, String> {
    try_scripted_with_algorithm(seed, ticks, rules, RngAlgorithm::Gm8Lcg32V1)
}

/// Explicit algorithm selection happens before initial level generation.
pub fn try_scripted_with_algorithm(
    seed: u64,
    ticks: u32,
    rules: RulesPreset,
    algorithm: RngAlgorithm,
) -> Result<ReplayResult, String> {
    check_ticks(ticks as usize)?;
    run(
        seed,
        rules,
        algorithm,
        ticks,
        (0..ticks).map(scripted_input),
    )
}

pub fn from_frames(
    seed: u64,
    rules: RulesPreset,
    frames: &[InputFrame],
) -> Result<ReplayResult, String> {
    from_frames_with_algorithm(seed, rules, frames, RngAlgorithm::Gm8Lcg32V1)
}

/// Historical replay imports must explicitly select LegacySplitMix64V1.
pub fn from_frames_with_algorithm(
    seed: u64,
    rules: RulesPreset,
    frames: &[InputFrame],
    algorithm: RngAlgorithm,
) -> Result<ReplayResult, String> {
    check_ticks(frames.len())?;
    run(
        seed,
        rules,
        algorithm,
        frames.len() as u32,
        frames.iter().copied(),
    )
}

fn check_ticks(ticks: usize) -> Result<(), String> {
    if ticks > MAX_REPLAY_TICKS as usize {
        Err(format!("Replay exceeds the {MAX_REPLAY_TICKS} tick limit"))
    } else {
        Ok(())
    }
}

fn run(
    seed: u64,
    rules: RulesPreset,
    algorithm: RngAlgorithm,
    requested_ticks: u32,
    frames: impl IntoIterator<Item = InputFrame>,
) -> Result<ReplayResult, String> {
    let mut game =
        Game::new_with_algorithm(seed, rules, algorithm).map_err(|error| error.to_string())?;
    for input in frames {
        game.step(&input);
    }
    Ok(ReplayResult {
        version: REPLAY_VERSION,
        simulation_version: SIMULATION_VERSION,
        algorithm,
        seed: seed.to_string(),
        rules,
        requested_ticks,
        ticks: game.tick,
        hash: format!("{:016x}", game.state_hash()),
        score: game.player.money,
        health: game.player.health,
        level: game.world.level,
        entities: game.world.entities.len(),
        state: game.state,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scripted_and_recorded_inputs_produce_identical_results() {
        let frames: Vec<_> = (0..240).map(scripted_input).collect();
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            let expected = scripted(8765, 240, rules);
            assert_eq!(expected, from_frames(8765, rules, &frames).unwrap());
            assert_eq!(expected, scripted(8765, 240, rules));
        }
    }
    #[test]
    fn zero_ticks_hashes_initial_state_and_seeds_affect_state() {
        let a = scripted(1, 0, RulesPreset::Classic);
        assert_eq!(a.ticks, 0);
        assert_eq!(
            a.hash,
            format!("{:016x}", Game::new(1, RulesPreset::Classic).state_hash())
        );
        assert_ne!(a.hash, scripted(2, 0, RulesPreset::Classic).hash);
    }
    #[test]
    fn untrusted_replays_are_bounded() {
        assert!(try_scripted(0, MAX_REPLAY_TICKS + 1, RulesPreset::Classic).is_err());
        let frames = vec![InputFrame::default(); MAX_REPLAY_TICKS as usize + 1];
        assert!(from_frames(0, RulesPreset::Classic, &frames).is_err());
    }
    #[test]
    fn canonical_replay_rejects_oversized_seed_but_legacy_preserves_all_bits() {
        assert!(try_scripted(u64::from(u32::MAX) + 1, 0, RulesPreset::Classic).is_err());
        assert!(from_frames(u64::MAX, RulesPreset::Classic, &[]).is_err());
        let legacy = from_frames_with_algorithm(
            u64::MAX,
            RulesPreset::Classic,
            &[],
            RngAlgorithm::LegacySplitMix64V1,
        )
        .unwrap();
        assert_eq!(legacy.seed, "18446744073709551615");
        assert_eq!(legacy.algorithm, RngAlgorithm::LegacySplitMix64V1);
        assert_eq!(legacy.version, REPLAY_VERSION);
        assert_eq!(legacy.simulation_version, SIMULATION_VERSION);
        assert_eq!(
            legacy.hash,
            format!(
                "{:016x}",
                Game::legacy(u64::MAX, RulesPreset::Classic).state_hash()
            )
        );
        let json = serde_json::to_value(&legacy).unwrap();
        assert_eq!(json["seed"], "18446744073709551615");
        assert_eq!(json["algorithm"], "legacy_splitmix64_v1");
    }

    #[test]
    fn selected_algorithm_drives_generation_before_the_first_input() {
        use crate::{Rng, generation};
        use std::collections::BTreeSet;
        let flags = BTreeSet::from(["no_dark_level".to_owned()]);
        for (seed, algorithm, mut expected_rng) in [
            (
                u64::from(u32::MAX),
                RngAlgorithm::Gm8Lcg32V1,
                Rng::new_canonical(u32::MAX),
            ),
            (
                u64::MAX,
                RngAlgorithm::LegacySplitMix64V1,
                Rng::legacy(u64::MAX),
            ),
        ] {
            let expected_world = generation::generate(&mut expected_rng, 1, &flags);
            let game = Game::new_with_algorithm(seed, RulesPreset::Classic, algorithm).unwrap();
            assert_eq!(game.rng.algorithm(), algorithm);
            assert_eq!(game.rng.state_bits(), expected_rng.state_bits());
            assert_eq!(
                serde_json::to_value(&game.world.tiles).unwrap(),
                serde_json::to_value(&expected_world.tiles).unwrap()
            );
            let result =
                from_frames_with_algorithm(seed, RulesPreset::Classic, &[], algorithm).unwrap();
            assert_eq!(result.hash, format!("{:016x}", game.state_hash()));
        }
    }

    #[test]
    fn legacy_scripted_inputs_remain_explicit_for_full_width_seeds() {
        let frames: Vec<_> = (0..30).map(scripted_input).collect();
        let scripted = try_scripted_with_algorithm(
            u64::MAX,
            30,
            RulesPreset::Classic,
            RngAlgorithm::LegacySplitMix64V1,
        )
        .unwrap();
        let recorded = from_frames_with_algorithm(
            u64::MAX,
            RulesPreset::Classic,
            &frames,
            RngAlgorithm::LegacySplitMix64V1,
        )
        .unwrap();
        assert_eq!(scripted, recorded);
        assert_eq!(recorded.algorithm, RngAlgorithm::LegacySplitMix64V1);
        assert_eq!(recorded.seed, "18446744073709551615");
    }
}
