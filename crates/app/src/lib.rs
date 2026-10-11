//! Native and WebGL hosts share the same Rust application and fixed simulation.
mod atlas;
mod audio;
mod controller;
mod donation_ui;
mod editor_io;
mod editor_ui;
mod host;
mod input;
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
    use super::diagnostic_replay;

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
