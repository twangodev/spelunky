//! Shared native/Wasm deterministic diagnostic replays.
use crate::{Game, InputFrame, Rng, RngAlgorithm, RulesPreset, RunState};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

pub const MAX_REPLAY_TICKS: u32 = 100_000;
/// The same bounded recording transport is accepted by native tools and Wasm.
pub const MAX_RECORDING_BYTES: usize = 64 * 1024 * 1024;
/// Version of the replay payload and diagnostic result envelope.
pub const REPLAY_VERSION: u32 = 1;
/// Adaptive recordings identify logical view changes separately from inputs.
pub const ADAPTIVE_REPLAY_VERSION: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReplayViewPolicy {
    GameplayLogicalExtentV1,
}

/// Finite validated dimensions in original game pixels, never physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct ReplayViewExtent {
    width: f64,
    height: f64,
}
// Private construction/deserialization excludes NaN, so equality is reflexive.
impl Eq for ReplayViewExtent {}
impl ReplayViewExtent {
    pub fn new(width: f64, height: f64) -> Result<Self, String> {
        crate::SimulationView::from_extent(width, height).map_err(|error| error.to_string())?;
        Ok(Self { width, height })
    }
    pub fn dimensions(self) -> [f64; 2] {
        [self.width, self.height]
    }
    fn apply(self, game: &mut Game) -> Result<(), String> {
        let (width, height) = if game.is_level() && game.opening.is_none() && game.ending.is_none()
        {
            (self.width, self.height)
        } else {
            (320.0, 240.0)
        };
        game.set_simulation_view_extent(width, height)
            .map_err(|error| error.to_string())
    }
}
impl<'de> Deserialize<'de> for ReplayViewExtent {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Extent {
            width: f64,
            height: f64,
        }
        let extent = Extent::deserialize(deserializer)?;
        Self::new(extent.width, extent.height).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayViewChange {
    /// Index in the recorded input stream, independent of stopped room clocks.
    pub before_frame: u32,
    pub extent: ReplayViewExtent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReplayViewTimeline {
    pub policy: ReplayViewPolicy,
    /// Applied after ordinary generation, before the first recorded frame.
    pub initial: ReplayViewExtent,
    #[serde(default)]
    pub changes: Vec<ReplayViewChange>,
}
impl ReplayViewTimeline {
    pub fn fixed(width: f64, height: f64) -> Result<Self, String> {
        Ok(Self {
            policy: ReplayViewPolicy::GameplayLogicalExtentV1,
            initial: ReplayViewExtent::new(width, height)?,
            changes: Vec::new(),
        })
    }
    pub fn validate(&self, frame_count: usize) -> Result<(), String> {
        check_ticks(frame_count)?;
        let mut previous = 0;
        for change in &self.changes {
            if change.before_frame <= previous || change.before_frame as usize >= frame_count {
                return Err("View changes must be strictly increasing frame indices after frame 0 and before the end of the input stream".into());
            }
            previous = change.before_frame;
        }
        Ok(())
    }
}

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
    /// Absent in all historical/original diagnostic outputs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adaptive_view: Option<ReplayViewTimeline>,
}

/// Strict shared legacy/versioned recording decoder. No platform IO or defaults
/// beyond omitted InputFrame fields; view metadata requires explicit version 2.
#[derive(Debug)]
pub struct ReplayRecording {
    pub simulation_version: u32,
    pub seed: u64,
    pub rules: RulesPreset,
    pub algorithm: RngAlgorithm,
    pub frames: Vec<InputFrame>,
    pub adaptive_view: Option<ReplayViewTimeline>,
}

pub fn parse_recording_json(text: &str) -> Result<ReplayRecording, String> {
    if text.len() > MAX_RECORDING_BYTES {
        return Err(format!(
            "Replay JSON exceeds the {MAX_RECORDING_BYTES} byte limit"
        ));
    }
    let value: Value =
        serde_json::from_str(text).map_err(|e| format!("Invalid replay JSON: {e}"))?;
    let object = value.as_object().ok_or("Replay must be an object")?;
    if object.keys().any(|k| {
        !matches!(
            k.as_str(),
            "seed"
                | "rules"
                | "frames"
                | "version"
                | "algorithm"
                | "simulation_version"
                | "adaptive_view"
        )
    }) {
        return Err("Unknown replay property; expected seed, rules, frames, version, algorithm, simulation_version, and version-2 adaptive_view".into());
    }
    let (algorithm, seed, simulation_version) = match (
        object.get("version"),
        object.get("algorithm"),
        object.get("simulation_version"),
    ) {
        (None, None, None) => {
            // Old replay JSON was numeric-u64 and used SplitMix before generation.
            let seed = object
                .get("seed")
                .and_then(Value::as_u64)
                .ok_or("Historical replay seed must be an unsigned integer")?;
            (
                RngAlgorithm::LegacySplitMix64V1,
                seed,
                crate::HISTORICAL_SIMULATION_VERSION,
            )
        }
        (Some(version), Some(algorithm), Some(simulation_version)) => {
            if !matches!(version.as_u64(), Some(1 | 2)) {
                return Err("Unsupported replay version; expected version 1 or 2".into());
            }
            let algorithm: RngAlgorithm = serde_json::from_value(algorithm.clone())
                .map_err(|error| format!("Invalid replay algorithm: {error}"))?;
            let simulation_version = simulation_version
                .as_u64()
                .and_then(|version| u32::try_from(version).ok())
                .ok_or("Replay simulation version must be an unsigned 32-bit integer")?;
            validate_simulation_version(algorithm, simulation_version)?;
            let seed_text = object
                .get("seed")
                .and_then(Value::as_str)
                .ok_or("Versioned replay seed must be a decimal string")?;
            if seed_text.is_empty() || !seed_text.bytes().all(|b| b.is_ascii_digit()) {
                return Err("Versioned replay seed must be an unsigned decimal string".into());
            }
            let seed = seed_text
                .parse::<u64>()
                .map_err(|_| "Invalid replay seed: expected an unsigned integer")?;
            if algorithm == RngAlgorithm::Gm8Lcg32V1 {
                Rng::try_canonical(seed).map_err(|error| error.to_string())?;
            }
            (algorithm, seed, simulation_version)
        }
        _ => {
            return Err(
                "Versioned replays require version, algorithm, and simulation_version".into(),
            );
        }
    };
    let adaptive_view = if object.get("version").and_then(Value::as_u64)
        == Some(u64::from(ADAPTIVE_REPLAY_VERSION))
    {
        Some(
            serde_json::from_value::<ReplayViewTimeline>(
                object
                    .get("adaptive_view")
                    .ok_or("Version 2 requires adaptive_view")?
                    .clone(),
            )
            .map_err(|error| format!("Invalid adaptive view timeline: {error}"))?,
        )
    } else {
        if object.contains_key("adaptive_view") {
            return Err("Adaptive view metadata requires replay version 2".into());
        }
        None
    };
    let rules = match object
        .get("rules")
        .and_then(Value::as_str)
        .ok_or("Replay rules must be a string")?
    {
        "classic" | "Classic" => RulesPreset::Classic,
        "assisted" | "Assisted" => RulesPreset::Assisted,
        _ => return Err("Rules must be classic or assisted".into()),
    };
    let frames = object
        .get("frames")
        .and_then(Value::as_array)
        .ok_or("Replay frames must be an array")?;
    if frames.len() > MAX_REPLAY_TICKS as usize {
        return Err("Replay exceeds the 100000 tick limit".into());
    }
    let defaults = serde_json::to_value(InputFrame::default()).map_err(|e| e.to_string())?;
    let mut parsed = Vec::with_capacity(frames.len());
    for (i, frame) in frames.iter().enumerate() {
        let mut merged = defaults.clone();
        let fields = frame
            .as_object()
            .ok_or_else(|| format!("Frame {i} must be an object"))?;
        for (key, value) in fields {
            if merged.get(key).is_none() {
                return Err(format!("Frame {i} has unknown input {key}"));
            }
            if !value.is_boolean() {
                return Err(format!("Frame {i} input {key} must be boolean"));
            }
            merged[key] = value.clone();
        }
        parsed.push(serde_json::from_value(merged).map_err(|e| format!("Invalid frame {i}: {e}"))?);
    }
    if let Some(view) = &adaptive_view {
        view.validate(parsed.len())?;
    }
    Ok(ReplayRecording {
        simulation_version,
        seed,
        rules,
        algorithm,
        frames: parsed,
        adaptive_view,
    })
}

/// Evaluate the exact same recording transport on native and Wasm hosts.
pub fn from_recording_json(text: &str) -> Result<ReplayResult, String> {
    let parsed = parse_recording_json(text)?;
    if let Some(view) = &parsed.adaptive_view {
        from_frames_with_view_timeline(
            parsed.seed,
            parsed.rules,
            &parsed.frames,
            parsed.algorithm,
            parsed.simulation_version,
            view,
        )
    } else {
        from_frames_with_simulation_version(
            parsed.seed,
            parsed.rules,
            &parsed.frames,
            parsed.algorithm,
            parsed.simulation_version,
        )
    }
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
        fresh_simulation_version(algorithm),
        ticks,
        (0..ticks).map(scripted_input),
        None,
    )
}

pub fn from_frames(
    seed: u64,
    rules: RulesPreset,
    frames: &[InputFrame],
) -> Result<ReplayResult, String> {
    from_frames_with_algorithm(seed, rules, frames, RngAlgorithm::Gm8Lcg32V1)
}

/// Select a fresh generation contract for this algorithm. Recorded imports
/// should also pass their simulation version to `from_frames_with_simulation_version`.
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
        fresh_simulation_version(algorithm),
        frames.len() as u32,
        frames.iter().copied(),
        None,
    )
}

/// Decode a recording's generation contract as well as its random algorithm.
/// Version 1 canonical recordings retain the historical generation path.
pub fn from_frames_with_simulation_version(
    seed: u64,
    rules: RulesPreset,
    frames: &[InputFrame],
    algorithm: RngAlgorithm,
    simulation_version: u32,
) -> Result<ReplayResult, String> {
    check_ticks(frames.len())?;
    validate_simulation_version(algorithm, simulation_version)?;
    run(
        seed,
        rules,
        algorithm,
        simulation_version,
        frames.len() as u32,
        frames.iter().copied(),
        None,
    )
}

/// Explicit adaptive replay identity; original recording APIs keep version 1.
pub fn from_frames_with_view_timeline(
    seed: u64,
    rules: RulesPreset,
    frames: &[InputFrame],
    algorithm: RngAlgorithm,
    simulation_version: u32,
    view: &ReplayViewTimeline,
) -> Result<ReplayResult, String> {
    view.validate(frames.len())?;
    run(
        seed,
        rules,
        algorithm,
        simulation_version,
        frames.len() as u32,
        frames.iter().copied(),
        Some(view.clone()),
    )
}

pub fn try_scripted_with_view_timeline(
    seed: u64,
    ticks: u32,
    rules: RulesPreset,
    algorithm: RngAlgorithm,
    view: &ReplayViewTimeline,
) -> Result<ReplayResult, String> {
    view.validate(ticks as usize)?;
    run(
        seed,
        rules,
        algorithm,
        fresh_simulation_version(algorithm),
        ticks,
        (0..ticks).map(scripted_input),
        Some(view.clone()),
    )
}

pub fn validate_simulation_version(algorithm: RngAlgorithm, version: u32) -> Result<(), String> {
    match (algorithm, version) {
        (_, crate::HISTORICAL_SIMULATION_VERSION)
        | (RngAlgorithm::Gm8Lcg32V1, crate::SIMULATION_VERSION) => Ok(()),
        _ => Err(format!(
            "Unsupported replay simulation version {version} for {algorithm:?}"
        )),
    }
}

fn fresh_simulation_version(algorithm: RngAlgorithm) -> u32 {
    match algorithm {
        RngAlgorithm::LegacySplitMix64V1 => crate::HISTORICAL_SIMULATION_VERSION,
        RngAlgorithm::Gm8Lcg32V1 => crate::SIMULATION_VERSION,
    }
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
    simulation_version: u32,
    requested_ticks: u32,
    frames: impl IntoIterator<Item = InputFrame>,
    view: Option<ReplayViewTimeline>,
) -> Result<ReplayResult, String> {
    validate_simulation_version(algorithm, simulation_version)?;
    let mut game = if simulation_version == crate::HISTORICAL_SIMULATION_VERSION {
        Game::new_historical_with_algorithm(seed, rules, algorithm)
    } else {
        Game::new_with_algorithm(seed, rules, algorithm)
    }
    .map_err(|error| error.to_string())?;
    let mut changes = view.as_ref().map(|view| view.changes.iter().peekable());
    let mut current_extent = view.as_ref().map(|view| view.initial);
    if let Some(extent) = current_extent {
        extent.apply(&mut game)?;
    }
    for (frame, input) in frames.into_iter().enumerate() {
        if let Some(changes) = &mut changes {
            if changes
                .peek()
                .is_some_and(|change| change.before_frame as usize == frame)
            {
                current_extent = Some(changes.next().expect("peeked view change").extent);
            }
        }
        // Restore the configured extent after leaving authored room/cinematic
        // scope, just as the host supplies its current viewport before a tick.
        if let Some(extent) = current_extent {
            extent.apply(&mut game)?;
        }
        game.step(&input);
    }
    Ok(ReplayResult {
        version: if view.is_some() {
            ADAPTIVE_REPLAY_VERSION
        } else {
            REPLAY_VERSION
        },
        simulation_version: game.simulation_version(),
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
        adaptive_view: view,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_replay_result_keeps_exact_version_one_serialization() {
        // The preceding public envelope's field order and omission contract.
        #[derive(Serialize)]
        struct VersionOne<'a> {
            version: u32,
            simulation_version: u32,
            algorithm: RngAlgorithm,
            seed: &'a str,
            rules: RulesPreset,
            requested_ticks: u32,
            ticks: u64,
            hash: &'a str,
            score: u32,
            health: i32,
            level: u32,
            entities: usize,
            state: RunState,
        }
        for algorithm in [RngAlgorithm::Gm8Lcg32V1, RngAlgorithm::LegacySplitMix64V1] {
            let result =
                try_scripted_with_algorithm(7, 8, RulesPreset::Classic, algorithm).unwrap();
            let old = VersionOne {
                version: 1,
                simulation_version: result.simulation_version,
                algorithm,
                seed: &result.seed,
                rules: result.rules,
                requested_ticks: result.requested_ticks,
                ticks: result.ticks,
                hash: &result.hash,
                score: result.score,
                health: result.health,
                level: result.level,
                entities: result.entities,
                state: result.state,
            };
            assert_eq!(
                serde_json::to_vec(&result).unwrap(),
                serde_json::to_vec(&old).unwrap()
            );
            assert!(result.adaptive_view.is_none());
            let mut manual = if result.simulation_version == crate::HISTORICAL_SIMULATION_VERSION {
                Game::new_historical_with_algorithm(7, RulesPreset::Classic, algorithm).unwrap()
            } else {
                Game::new_with_algorithm(7, RulesPreset::Classic, algorithm).unwrap()
            };
            for input in (0..8).map(scripted_input) {
                manual.step(&input);
            }
            assert_eq!(result.hash, format!("{:016x}", manual.state_hash()));
        }
    }

    #[test]
    fn adaptive_replay_resizes_before_recorded_input_and_preserves_identity() {
        let mut view = ReplayViewTimeline::fixed(480.0, 240.0).unwrap();
        view.changes = vec![
            ReplayViewChange {
                before_frame: 3,
                extent: ReplayViewExtent::new(320.0, 640.0).unwrap(),
            },
            ReplayViewChange {
                before_frame: 9,
                extent: ReplayViewExtent::new(320.0, 240.0).unwrap(),
            },
        ];
        let frames: Vec<_> = (0..12).map(scripted_input).collect();
        let restored: ReplayViewTimeline =
            serde_json::from_slice(&serde_json::to_vec(&view).unwrap()).unwrap();
        assert_eq!(restored, view);
        let result = from_frames_with_view_timeline(
            7,
            RulesPreset::Classic,
            &frames,
            RngAlgorithm::Gm8Lcg32V1,
            crate::SIMULATION_VERSION,
            &restored,
        )
        .unwrap();
        let mut manual = Game::new(7, RulesPreset::Classic);
        for (index, input) in frames.iter().enumerate() {
            let extent = match index {
                0..=2 => (480.0, 240.0),
                3..=8 => (320.0, 640.0),
                _ => (320.0, 240.0),
            };
            manual
                .set_simulation_view_extent(extent.0, extent.1)
                .unwrap();
            manual.step(input);
        }
        assert_eq!(result.hash, format!("{:016x}", manual.state_hash()));
        assert_eq!(result.ticks, manual.tick);
        assert_eq!(result.version, 2);
        assert_eq!(result.adaptive_view.as_ref(), Some(&view));
        assert_eq!(
            result,
            try_scripted_with_view_timeline(
                7,
                12,
                RulesPreset::Classic,
                RngAlgorithm::Gm8Lcg32V1,
                &view
            )
            .unwrap()
        );
        let canonical = ReplayViewTimeline::fixed(320.0, 240.0).unwrap();
        let unchanged = from_frames_with_view_timeline(
            7,
            RulesPreset::Classic,
            &frames,
            RngAlgorithm::Gm8Lcg32V1,
            crate::SIMULATION_VERSION,
            &canonical,
        )
        .unwrap();
        assert_eq!(unchanged.hash, scripted(7, 12, RulesPreset::Classic).hash);
        assert_ne!(result.adaptive_view, unchanged.adaptive_view);
    }

    #[test]
    fn adaptive_replay_rejects_invalid_extents_and_ambiguous_frame_changes() {
        for (w, h) in [
            (319.0, 240.0),
            (320.0, 239.0),
            (f64::NAN, 240.0),
            (320.0, f64::INFINITY),
        ] {
            assert!(ReplayViewTimeline::fixed(w, h).is_err());
        }
        let mut view = ReplayViewTimeline::fixed(480.0, 240.0).unwrap();
        for frames in [vec![0], vec![1, 1], vec![3, 2], vec![4]] {
            view.changes = frames
                .into_iter()
                .map(|before_frame| ReplayViewChange {
                    before_frame,
                    extent: view.initial,
                })
                .collect();
            assert!(view.validate(4).is_err());
        }
        for value in [
            r#"{"policy":"invented","initial":{"width":480,"height":240}}"#,
            r#"{"policy":"gameplay_logical_extent_v1","initial":{"width":319,"height":240}}"#,
            r#"{"policy":"gameplay_logical_extent_v1","initial":{"width":480,"height":240,"extra":1}}"#,
            r#"{"policy":"gameplay_logical_extent_v1","initial":{"width":480,"height":240},"extra":1}"#,
        ] {
            assert!(serde_json::from_str::<ReplayViewTimeline>(value).is_err());
        }
        view.changes.clear();
        assert!(view.validate(0).is_ok());
        assert!(view.validate(MAX_REPLAY_TICKS as usize + 1).is_err());
    }

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
        assert_eq!(
            legacy.simulation_version,
            crate::HISTORICAL_SIMULATION_VERSION
        );
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
