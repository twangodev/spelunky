//! Shared native/Wasm deterministic diagnostic replays.
use crate::{Game, InputFrame, RulesPreset, RunState};
use serde::Serialize;

pub const MAX_REPLAY_TICKS: u32 = 100_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReplayResult {
    pub seed: u64,
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

/// Panics for an oversized replay. Untrusted hosts should use `try_scripted`.
pub fn scripted(seed: u64, ticks: u32, rules: RulesPreset) -> ReplayResult {
    try_scripted(seed, ticks, rules).expect("replay exceeds 100000 ticks")
}

pub fn try_scripted(seed: u64, ticks: u32, rules: RulesPreset) -> Result<ReplayResult, String> {
    check_ticks(ticks as usize)?;
    Ok(run(seed, rules, ticks, (0..ticks).map(scripted_input)))
}

pub fn from_frames(
    seed: u64,
    rules: RulesPreset,
    frames: &[InputFrame],
) -> Result<ReplayResult, String> {
    check_ticks(frames.len())?;
    Ok(run(
        seed,
        rules,
        frames.len() as u32,
        frames.iter().copied(),
    ))
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
    requested_ticks: u32,
    frames: impl IntoIterator<Item = InputFrame>,
) -> ReplayResult {
    let mut game = Game::new(seed, rules);
    for input in frames {
        game.step(&input);
    }
    ReplayResult {
        seed,
        rules,
        requested_ticks,
        ticks: game.tick,
        hash: format!("{:016x}", game.state_hash()),
        score: game.player.money,
        health: game.player.health,
        level: game.world.level,
        entities: game.world.entities.len(),
        state: game.state,
    }
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
}
