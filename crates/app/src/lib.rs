//! Native and WebGL hosts share the same Rust application and fixed simulation.
#[cfg(test)]
mod adaptive_gameplay_tests;
mod atlas;
mod audio;
#[cfg(any(target_arch = "wasm32", test))]
mod browser_shortcuts;
mod cheat_ui;
mod controller;
mod donation_ui;
mod editor_io;
mod editor_ui;
mod host;
mod input;
mod performance;
mod renderer;
mod scene;
mod settings_ui;
mod storage;
mod timing;
mod viewport;

pub fn run() {
    if let Err(error) = host::run() {
        eprintln!("Spelunky could not start: {error}");
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn start() -> Result<(), wasm_bindgen::JsValue> {
    console_error_panic_hook::set_once();
    host::run().map_err(|error| wasm_bindgen::JsValue::from_str(&error))
}

/// Deterministic diagnostic shared with native replay tooling. Seeds are decimal
/// strings so JavaScript cannot lose precision above its integer-safe range.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn replay(seed: &str, ticks: u32, assisted: bool) -> Result<String, wasm_bindgen::JsValue> {
    diagnostic_replay(seed, ticks, assisted, "gm8_lcg32_v1")
        .map_err(|error| wasm_bindgen::JsValue::from_str(&error))
}

/// Explicit legacy diagnostics retain all 64 seed bits. Algorithm names match
/// the versioned replay envelope; unknown names are rejected before generation.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn replay_with_algorithm(
    seed: &str,
    ticks: u32,
    assisted: bool,
    algorithm: &str,
) -> Result<String, wasm_bindgen::JsValue> {
    diagnostic_replay(seed, ticks, assisted, algorithm)
        .map_err(|error| wasm_bindgen::JsValue::from_str(&error))
}

/// Evaluate a native replay-file recording, including version-2 logical view
/// changes and exact recorded input edges. Validation lives in the shared core.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn replay_recording(recording: &str) -> Result<String, wasm_bindgen::JsValue> {
    diagnostic_recording(recording).map_err(|error| wasm_bindgen::JsValue::from_str(&error))
}

#[cfg(any(target_arch = "wasm32", test))]
fn diagnostic_recording(recording: &str) -> Result<String, String> {
    let result = spelunky_core::replay::from_recording_json(recording)?;
    // Match replay-file's JSON field ordering as well as its evaluated result.
    serde_json::to_string(&serde_json::to_value(result).map_err(|error| error.to_string())?)
        .map_err(|error| error.to_string())
}

#[cfg(any(target_arch = "wasm32", test))]
fn diagnostic_replay(
    seed: &str,
    ticks: u32,
    assisted: bool,
    algorithm: &str,
) -> Result<String, String> {
    if seed.is_empty() || !seed.bytes().all(|b| b.is_ascii_digit()) {
        return Err("Seed must be an unsigned decimal string".into());
    }
    let seed = seed
        .parse::<u64>()
        .map_err(|error| format!("Invalid seed: {error}"))?;
    let algorithm: spelunky_core::RngAlgorithm =
        serde_json::from_value(serde_json::Value::String(algorithm.to_owned()))
            .map_err(|error| format!("Invalid replay algorithm: {error}"))?;
    let rules = if assisted {
        spelunky_core::RulesPreset::Assisted
    } else {
        spelunky_core::RulesPreset::Classic
    };
    let result = spelunky_core::replay::try_scripted_with_algorithm(seed, ticks, rules, algorithm)?;
    serde_json::to_string(&result).map_err(|error| error.to_string())
}

#[cfg(test)]
mod replay_tests {
    use super::{diagnostic_recording, diagnostic_replay};

    fn adaptive_recording() -> serde_json::Value {
        serde_json::json!({"version":2,"simulation_version":spelunky_core::SIMULATION_VERSION,
            "algorithm":"gm8_lcg32_v1","seed":"7","rules":"classic",
            "adaptive_view":{"policy":"gameplay_logical_extent_v1",
                "initial":{"width":426.6666564941406_f64,"height":240.0},
                "changes":[{"before_frame":2,"extent":{"width":320.0,"height":640.0}}]},
            "frames":[{}, {"right":true}, {"jump":true,"jump_pressed":true}, {"jump_released":true}]})
    }

    #[test]
    fn recording_wrapper_matches_native_recorded_frames_and_resize_timeline() {
        let recording = adaptive_recording().to_string();
        let result = spelunky_core::replay::from_recording_json(&recording).unwrap();
        assert_eq!(
            diagnostic_recording(&recording).unwrap(),
            serde_json::to_string(&serde_json::to_value(&result).unwrap()).unwrap()
        );
        let parsed = spelunky_core::replay::parse_recording_json(&recording).unwrap();
        let view = parsed.adaptive_view.unwrap();
        let mut manual = spelunky_core::Game::new(7, spelunky_core::RulesPreset::Classic);
        for (index, input) in parsed.frames.iter().enumerate() {
            let [width, height] = if index < 2 {
                view.initial.dimensions()
            } else {
                view.changes[0].extent.dimensions()
            };
            manual.set_simulation_view_extent(width, height).unwrap();
            manual.step(input);
        }
        assert_eq!(result.hash, format!("{:016x}", manual.state_hash()));
        assert_eq!(result.ticks, manual.tick);
        assert_eq!(result.version, 2);
        assert_eq!(result.adaptive_view, Some(view));
        let old =
            diagnostic_recording(r#"{"seed":18446744073709551615,"rules":"classic","frames":[]}"#)
                .unwrap();
        let old: serde_json::Value = serde_json::from_str(&old).unwrap();
        assert_eq!(old["version"], 1);
        assert_eq!(old["seed"], "18446744073709551615");
        assert!(old.get("adaptive_view").is_none());
    }

    #[test]
    fn recording_wrapper_rejects_malformed_seed_algorithm_inputs_and_view_timeline() {
        let recording = adaptive_recording();
        for seed in [
            "",
            "+1",
            "-1",
            "1.0",
            " 1",
            "4294967296",
            "18446744073709551616",
        ] {
            let mut invalid = recording.clone();
            invalid["seed"] = seed.into();
            assert!(diagnostic_recording(&invalid.to_string()).is_err());
        }
        let mut invalid = recording.clone();
        invalid["algorithm"] = "unknown".into();
        assert!(diagnostic_recording(&invalid.to_string()).is_err());
        invalid = recording.clone();
        invalid["frames"] = serde_json::json!([{"jump_pressed":1}]);
        assert!(diagnostic_recording(&invalid.to_string()).is_err());
        for frame in [0, 4] {
            invalid = recording.clone();
            invalid["adaptive_view"]["changes"][0]["before_frame"] = frame.into();
            assert!(diagnostic_recording(&invalid.to_string()).is_err());
        }
        invalid = recording.clone();
        invalid["adaptive_view"]["initial"]["width"] = 319.into();
        assert!(diagnostic_recording(&invalid.to_string()).is_err());
        invalid = recording.clone();
        invalid["frames"] =
            serde_json::Value::Array(vec![
                serde_json::json!({});
                spelunky_core::replay::MAX_REPLAY_TICKS as usize + 1
            ]);
        assert!(
            diagnostic_recording(&invalid.to_string())
                .unwrap_err()
                .contains("tick limit")
        );
        // Rejected by the shared byte bound before parsing or level generation.
        let oversized = " ".repeat(spelunky_core::replay::MAX_RECORDING_BYTES + 1);
        assert!(
            diagnostic_recording(&oversized)
                .unwrap_err()
                .contains("byte limit")
        );
    }

    #[test]
    fn diagnostic_seed_strings_keep_legacy_precision_and_check_new_seeds() {
        assert!(diagnostic_replay("4294967296", 0, false, "gm8_lcg32_v1").is_err());
        assert!(diagnostic_replay("9007199254740993", 0, false, "gm8_lcg32_v1").is_err());
        let output =
            diagnostic_replay("18446744073709551615", 0, false, "legacy_splitmix64_v1").unwrap();
        let value: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert_eq!(value["seed"], "18446744073709551615");
        assert_eq!(value["algorithm"], "legacy_splitmix64_v1");
        assert_eq!(value["version"], 1);
        assert_eq!(value["simulation_version"], 1);
        let canonical = diagnostic_replay("4294967295", 0, true, "gm8_lcg32_v1").unwrap();
        let value: serde_json::Value = serde_json::from_str(&canonical).unwrap();
        assert_eq!(value["algorithm"], "gm8_lcg32_v1");
        assert_eq!(
            value["simulation_version"],
            spelunky_core::SIMULATION_VERSION
        );
        assert_eq!(value["rules"], "Assisted");
    }

    #[test]
    fn diagnostic_rejects_unknown_algorithms_and_malformed_decimal_seeds() {
        assert!(diagnostic_replay("1", 0, false, "unknown").is_err());
        for seed in ["", "+1", "-1", "1.0", " 1", "18446744073709551616"] {
            assert!(diagnostic_replay(seed, 0, false, "legacy_splitmix64_v1").is_err());
        }
    }
}
