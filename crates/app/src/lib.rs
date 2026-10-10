//! Native and WebGL hosts share the same Rust application and fixed simulation.
mod atlas;
mod audio;
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
    let seed = seed
        .parse::<u64>()
        .map_err(|error| wasm_bindgen::JsValue::from_str(&format!("Invalid seed: {error}")))?;
    let rules = if assisted {
        spelunky_core::RulesPreset::Assisted
    } else {
        spelunky_core::RulesPreset::Classic
    };
    let result = spelunky_core::replay::try_scripted(seed, ticks, rules)
        .map_err(|error| wasm_bindgen::JsValue::from_str(&error))?;
    serde_json::to_string(&result)
        .map_err(|error| wasm_bindgen::JsValue::from_str(&error.to_string()))
}
