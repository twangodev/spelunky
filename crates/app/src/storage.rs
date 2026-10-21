//! Versioned settings and run history, shared by the native and web hosts.
use serde::{Deserialize, Serialize};
use spelunky_core::{RngAlgorithm, RulesPreset};
use std::collections::{BTreeMap, BTreeSet};

pub const SAVE_VERSION: u32 = 2;
#[cfg(target_arch = "wasm32")]
const STORAGE_KEY: &str = "spelunky-rust.save";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub rules: RulesPreset,
    pub pixel_snap: bool,
    /// Classic `global.graphicsHigh`: adds source-gated decorations and effects.
    pub graphics_high: bool,
    /// Source settings default: hold Down while moving to run.
    pub down_to_run: bool,
    pub music_volume: f32,
    pub effects_volume: f32,
    pub vsync: bool,
    pub frame_cap: Option<u32>,
    /// Action names map to winit physical-key names, separated by commas.
    pub bindings: BTreeMap<String, String>,
    /// Classic controller actions map to portable physical button names.
    pub controller_bindings: BTreeMap<String, String>,
    /// Raw controls calibrated for directions on browser gamepads whose
    /// layout the browser does not recognize. Standard pads ignore these.
    pub browser_direction_bindings: crate::controller::BrowserDirectionBindings,
    /// Classic `global.gamepadOn`: controller input is opt-in and saved.
    pub controller_enabled: bool,
    /// Classic starts fullscreen and saves the user's display preference.
    pub fullscreen: bool,
}

impl Default for Settings {
    fn default() -> Self {
        let bindings = [
            ("left", "ArrowLeft"),
            ("right", "ArrowRight,KeyD"),
            ("up", "ArrowUp,KeyW"),
            ("down", "ArrowDown"),
            ("jump", "KeyZ,Space"),
            ("attack", "KeyX"),
            ("cycle", "KeyC"),
            ("bomb", "KeyA"),
            ("rope", "KeyS"),
            ("flare", "KeyF"),
            ("pay", "KeyP"),
            ("run", "ShiftLeft,ShiftRight"),
            ("exit", "ArrowUp,KeyW"),
            ("pause", "Escape"),
        ]
        .into_iter()
        .map(|(action, key)| (action.into(), key.into()))
        .collect();
        Self {
            rules: RulesPreset::Classic,
            pixel_snap: false,
            graphics_high: true,
            down_to_run: true,
            // Classic starts both controls at 15 and applies
            // (2000 + 8000 * (volume / 18)) / 10000 to loaded sounds.
            music_volume: 0.2 + 0.8 * (15.0 / 18.0),
            effects_volume: 0.2 + 0.8 * (15.0 / 18.0),
            // Classic's Global Game Settings.xml sets useVsync=false. Keep the
            // uncapped presentation default available on supported surfaces.
            vsync: false,
            frame_cap: None,
            bindings,
            controller_bindings: crate::controller::default_bindings(),
            browser_direction_bindings: crate::controller::BrowserDirectionBindings::default(),
            controller_enabled: false,
            fullscreen: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    pub seed: u64,
    /// Historical records retain their original generator even when new runs
    /// use a different default. Missing identity never means the new default.
    #[serde(default = "legacy_record_algorithm")]
    pub rng_algorithm: RngAlgorithm,
    /// Zero identifies historical records predating simulation metadata.
    #[serde(default)]
    pub simulation_version: u32,
    pub rules: RulesPreset,
    pub score: u32,
    pub level: u32,
    pub ticks: u64,
    /// Classic stores room-speed units, adding 30 for each live level tick.
    /// Older saves only retained ticks, so their duration is derived on read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub elapsed_ms: Option<u64>,
    /// Shortcut wins count toward score and wins, but never fastest time.
    #[serde(default = "eligible_legacy_record")]
    pub fastest_eligible: bool,
    pub won: bool,
    #[serde(default)]
    pub kills: u32,
    #[serde(default)]
    pub rescues: u32,
}

fn eligible_legacy_record() -> bool {
    true
}

fn existing_save_first_time() -> bool {
    false
}

fn legacy_record_algorithm() -> RngAlgorithm {
    RngAlgorithm::LegacySplitMix64V1
}

impl Record {
    pub fn elapsed_ms(&self) -> u64 {
        self.elapsed_ms
            .unwrap_or_else(|| self.ticks.saturating_mul(30))
    }

    pub fn elapsed_seconds(&self) -> u64 {
        self.elapsed_ms() / 1000
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SaveData {
    pub version: u32,
    /// Keep first-run onboarding alive across saves made before the player
    /// enters the Start portal. Older saves without this field are established
    /// profiles and must not replay onboarding.
    #[serde(default = "existing_save_first_time")]
    pub first_time: bool,
    pub settings: Settings,
    pub records: Vec<Record>,
    /// Original lifetime counters are independent of run records and unlocks.
    #[serde(default)]
    pub lifetime_stats: spelunky_core::statistics::LifetimeStats,
    /// A Classic `stats.txt` report can be imported once; remember the choice so
    /// a later user action cannot replace newer Rust-session counters.
    #[serde(default)]
    pub classic_stats_imported: bool,
    /// Source minigame bests: mini1 = Sun survival seconds, mini2 = Moon archery
    /// hits, mini3 = Stars shopkeeper kills. Unknown keys remain compatible.
    #[serde(default)]
    pub challenge_records: BTreeMap<String, u32>,
    /// Outstanding shortcut donations, keyed by tunnel1_remaining through
    /// tunnel3_remaining. Missing keys use original costs (100k, 200k, 300k).
    #[serde(default)]
    pub tunnel_remaining: BTreeMap<String, u32>,
    pub unlocks: BTreeSet<String>,
    /// Original .lvl text, keyed by the user-visible level name.
    pub custom_levels: BTreeMap<String, String>,
}

impl Default for SaveData {
    fn default() -> Self {
        Self {
            version: SAVE_VERSION,
            first_time: true,
            settings: Settings::default(),
            records: Vec::new(),
            lifetime_stats: Default::default(),
            classic_stats_imported: false,
            challenge_records: BTreeMap::new(),
            tunnel_remaining: BTreeMap::new(),
            unlocks: BTreeSet::new(),
            custom_levels: BTreeMap::new(),
        }
    }
}

impl SaveData {
    /// Apply one Classic `stats.txt` import, replacing rather than adding the
    /// source lifetime counters.
    pub fn import_classic_stats_report(&mut self, report: &str) -> Result<(), String> {
        if self.classic_stats_imported {
            return Err("Classic stats have already been imported".into());
        }
        let stats = spelunky_core::statistics::LifetimeStats::from_classic_report(report)?;
        self.lifetime_stats = stats;
        self.classic_stats_imported = true;
        Ok(())
    }

    pub fn from_json(json: &str) -> Result<Self, String> {
        // Inspect the version before deserializing, so a future schema gets a
        // useful message even when its fields are incompatible with this one.
        let value: serde_json::Value =
            serde_json::from_str(json).map_err(|error| format!("Invalid save JSON: {error}"))?;
        let object = value.as_object().ok_or("Save data must be a JSON object")?;
        let version = match object.get("version") {
            Some(value) => value.as_u64().ok_or("Invalid save version")?,
            None => 0,
        };
        if version > u64::from(SAVE_VERSION) {
            return Err(format!(
                "Save version {version} is newer than supported version {SAVE_VERSION}"
            ));
        }
        let mut data: Self =
            serde_json::from_value(value).map_err(|error| format!("Invalid save data: {error}"))?;
        data.custom_levels = if version >= 2 {
            crate::editor_io::canonical_level_sources_v2(data.custom_levels)
        } else {
            crate::editor_io::canonical_level_sources_v1(data.custom_levels)
        };
        // Versions zero and one used the same flat level-key format.
        data.version = SAVE_VERSION;
        // Newly introduced actions receive defaults; existing remaps and
        // deliberately empty (unbound) actions remain intact.
        let default_bindings = Settings::default().bindings;
        if !data.settings.bindings.contains_key("exit") {
            // Classic uses the same Up action at doors. Older Rust saves
            // predate the separate Exit alias, so carry their Up remap forward.
            let up = data
                .settings
                .bindings
                .get("up")
                .or_else(|| default_bindings.get("up"))
                .cloned()
                .expect("default settings include an Up binding");
            data.settings.bindings.insert("exit".into(), up);
        }
        for (action, keys) in default_bindings {
            data.settings.bindings.entry(action).or_insert(keys);
        }
        data.settings.controller_bindings =
            crate::controller::validated_bindings(&data.settings.controller_bindings);
        data.validate()?;
        Ok(data)
    }

    pub fn to_json(&self) -> Result<String, String> {
        self.validate()?;
        serde_json::to_string_pretty(self).map_err(|error| format!("Cannot encode save: {error}"))
    }

    fn validate(&self) -> Result<(), String> {
        if self.version != SAVE_VERSION {
            return Err(format!("Cannot write save version {}", self.version));
        }
        for record in &self.records {
            if record.rng_algorithm == RngAlgorithm::Gm8Lcg32V1 && record.simulation_version == 0 {
                return Err("Canonical records require a positive simulation version".into());
            }
            if record.rng_algorithm == RngAlgorithm::Gm8Lcg32V1 && record.seed > u64::from(u32::MAX)
            {
                return Err(format!(
                    "Canonical record seed {} exceeds {}",
                    record.seed,
                    u32::MAX
                ));
            }
        }
        for (name, volume) in [
            ("music", self.settings.music_volume),
            ("effects", self.settings.effects_volume),
        ] {
            if !volume.is_finite() || !(0.0..=1.0).contains(&volume) {
                return Err(format!("{name} volume must be finite and between 0 and 1"));
            }
        }
        if let Some(cap) = self.settings.frame_cap {
            if !(1..=1000).contains(&cap) {
                return Err("Frame cap must be between 1 and 1000, or unlimited".into());
            }
        }
        for (key, cost) in [
            // scrResetHighscores uses max+1 to defer the first tunnel visit.
            ("tunnel1_remaining", 100_001),
            ("tunnel2_remaining", 200_001),
            ("tunnel3_remaining", 300_000),
        ] {
            if self
                .tunnel_remaining
                .get(key)
                .is_some_and(|remaining| *remaining > cost)
            {
                return Err(format!("{key} cannot exceed its source limit of {cost}"));
            }
        }
        Ok(())
    }
}

pub struct Store {
    pub data: SaveData,
    pub warning: Option<String>,
    overwrite_blocked: bool,
}

impl Store {
    pub fn load() -> Self {
        let stored = read();
        #[cfg(not(target_arch = "wasm32"))]
        let should_check_classic_stats = should_import_classic_stats(&stored);
        #[cfg(not(target_arch = "wasm32"))]
        let legacy = if should_check_classic_stats {
            read_classic_stats_file()
        } else {
            Ok(None)
        };
        #[cfg(target_arch = "wasm32")]
        let legacy: Result<Option<String>, String> = Ok(None);

        let (mut store, imported) = Self::from_read_and_classic_stats(stored, legacy);
        if imported {
            let _ = store.save();
        }
        store
    }

    fn from_read(result: Result<Option<String>, String>) -> Self {
        match result.and_then(|json| json.map(|json| SaveData::from_json(&json)).transpose()) {
            Ok(data) => Self {
                data: data.unwrap_or_default(),
                warning: None,
                overwrite_blocked: false,
            },
            Err(error) => Self {
                data: SaveData {
                    first_time: false,
                    ..SaveData::default()
                },
                warning: Some(format!(
                    "{error}. Existing save is preserved; reset explicitly to replace it."
                )),
                overwrite_blocked: true,
            },
        }
    }

    fn from_read_and_classic_stats(
        stored: Result<Option<String>, String>,
        legacy: Result<Option<String>, String>,
    ) -> (Self, bool) {
        let should_import = should_import_classic_stats(&stored);
        let mut store = Self::from_read(stored);
        if !should_import || store.overwrite_blocked {
            return (store, false);
        }
        let Ok(Some(report)) = legacy else {
            return (store, false);
        };
        match store.import_classic_stats(&report) {
            Ok(()) => (store, true),
            Err(_) => (store, false),
        }
    }

    /// Imports a Classic report without adding it to the current counters.
    /// The one-time marker prevents a later import from erasing Rust progress.
    pub fn import_classic_stats(&mut self, report: &str) -> Result<(), String> {
        if self.overwrite_blocked {
            return Err(self
                .warning
                .clone()
                .unwrap_or_else(|| "Save overwrite blocked".into()));
        }
        self.data.import_classic_stats_report(report)
    }

    /// The native Classic report is derived from the loaded save. Do not
    /// overwrite it when that save could not be read: `data` then contains a
    /// fresh placeholder rather than the user's lifetime counters.
    pub fn may_write_classic_stats_report(&self) -> bool {
        !self.overwrite_blocked
    }

    pub fn save(&mut self) -> Result<(), String> {
        if self.overwrite_blocked {
            return Err(self
                .warning
                .clone()
                .unwrap_or_else(|| "Save overwrite blocked".into()));
        }
        let result = self.data.to_json().and_then(|json| write(&json));
        match &result {
            Ok(()) => self.warning = None,
            Err(error) => self.warning = Some(error.clone()),
        }
        result
    }

    /// Adds a completed run without mixing Classic and Assisted rule identities.
    /// A failed write leaves the record in memory for a later retry.
    pub fn add_record(&mut self, record: Record) -> Result<(), String> {
        self.data.records.push(record);
        self.save()
    }

    /// Explicit user action permitting replacement of unreadable/future data.
    pub fn reset(&mut self) -> Result<(), String> {
        self.data = SaveData::default();
        self.overwrite_blocked = false;
        self.save()
    }
}

/// A Classic stats report is the migration source for new profiles and Rust
/// saves written before lifetime counters were added. A current save that has
/// the field (even when all values are zero) remains authoritative.
fn should_import_classic_stats(stored: &Result<Option<String>, String>) -> bool {
    match stored {
        Ok(None) => true,
        Ok(Some(json)) => {
            let Ok(value) = serde_json::from_str::<serde_json::Value>(json) else {
                return false;
            };
            let Some(object) = value.as_object() else {
                return false;
            };
            let version = object
                .get("version")
                .map(serde_json::Value::as_u64)
                .unwrap_or(Some(0));
            version.is_some_and(|version| version <= u64::from(SAVE_VERSION))
                && !object.contains_key("lifetime_stats")
                && object
                    .get("classic_stats_imported")
                    .and_then(serde_json::Value::as_bool)
                    != Some(true)
        }
        Err(_) => false,
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn read_classic_stats_file() -> Result<Option<String>, String> {
    match std::fs::read_to_string("stats.txt") {
        Ok(report) => Ok(Some(report)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("Cannot read classic stats.txt: {error}")),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn path() -> Result<std::path::PathBuf, String> {
    let env_path = |name: &str| std::env::var_os(name).map(std::path::PathBuf::from);
    let environment = NativeDataEnvironment {
        xdg_data_home: env_path("XDG_DATA_HOME"),
        home: env_path("HOME"),
        appdata: env_path("APPDATA"),
        local_appdata: env_path("LOCALAPPDATA"),
        user_profile: env_path("USERPROFILE"),
    };
    let platform = if cfg!(target_os = "windows") {
        NativePlatform::Windows
    } else if cfg!(target_os = "macos") {
        NativePlatform::MacOs
    } else {
        NativePlatform::Unix
    };
    Ok(native_data_directory(platform, &environment)?.join("spelunky-rust/save.json"))
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Copy)]
enum NativePlatform {
    Windows,
    MacOs,
    Unix,
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Default)]
struct NativeDataEnvironment {
    xdg_data_home: Option<std::path::PathBuf>,
    home: Option<std::path::PathBuf>,
    appdata: Option<std::path::PathBuf>,
    local_appdata: Option<std::path::PathBuf>,
    user_profile: Option<std::path::PathBuf>,
}

#[cfg(not(target_arch = "wasm32"))]
fn native_data_directory(
    platform: NativePlatform,
    environment: &NativeDataEnvironment,
) -> Result<std::path::PathBuf, String> {
    use std::path::{Path, PathBuf};

    fn require_absolute(path: &Path, name: &str) -> Result<PathBuf, String> {
        if !path.is_absolute() {
            return Err(format!("{name} must be an absolute path"));
        }
        Ok(path.to_owned())
    }

    fn present(path: &Option<PathBuf>) -> Option<&Path> {
        path.as_deref().filter(|path| !path.as_os_str().is_empty())
    }

    match platform {
        NativePlatform::Windows => {
            if let Some(path) = present(&environment.appdata) {
                return require_absolute(path, "APPDATA");
            }
            if let Some(path) = present(&environment.local_appdata) {
                return require_absolute(path, "LOCALAPPDATA");
            }
            if let Some(path) = present(&environment.user_profile) {
                return Ok(require_absolute(path, "USERPROFILE")?
                    .join("AppData")
                    .join("Roaming"));
            }
            if let Some(path) = present(&environment.home) {
                return Ok(require_absolute(path, "HOME")?.join(".local/share"));
            }
            Err("No Windows user data directory available for save data".into())
        }
        NativePlatform::MacOs => {
            let home =
                present(&environment.home).ok_or("No home directory available for save data")?;
            Ok(require_absolute(home, "HOME")?
                .join("Library")
                .join("Application Support"))
        }
        NativePlatform::Unix => {
            if let Some(path) = present(&environment.xdg_data_home) {
                return require_absolute(path, "XDG_DATA_HOME");
            }
            let home =
                present(&environment.home).ok_or("No home directory available for save data")?;
            Ok(require_absolute(home, "HOME")?.join(".local/share"))
        }
    }
}

/// Before this host used OS-specific user data directories it wrote to
/// `XDG_CONFIG_HOME` or `$HOME/.local/share`. Read that location only when the
/// new path is absent, then the next normal save migrates it atomically.
#[cfg(not(target_arch = "wasm32"))]
fn legacy_path() -> Result<std::path::PathBuf, String> {
    use std::path::PathBuf;
    if let Some(config) = std::env::var_os("XDG_CONFIG_HOME").filter(|path| !path.is_empty()) {
        let path = PathBuf::from(config);
        if !path.is_absolute() {
            return Err("XDG_CONFIG_HOME must be an absolute path".into());
        }
        Ok(path.join("spelunky-rust/save.json"))
    } else {
        let home = std::env::var_os("HOME")
            .filter(|path| !path.is_empty())
            .ok_or("No home directory available for save data")?;
        Ok(PathBuf::from(home).join(".local/share/spelunky-rust/save.json"))
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn read_native_save(
    primary: &std::path::Path,
    legacy: Option<&std::path::Path>,
) -> Result<Option<String>, String> {
    fn read_one(path: &std::path::Path) -> Result<Option<String>, std::io::Error> {
        match std::fs::read_to_string(path) {
            Ok(json) => Ok(Some(json)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }

    match read_one(primary) {
        Ok(Some(json)) => Ok(Some(json)),
        Ok(None) => match legacy.filter(|path| *path != primary) {
            Some(path) => read_one(path).map_err(|error| format!("Cannot read save data: {error}")),
            None => Ok(None),
        },
        Err(error) => Err(format!("Cannot read save data: {error}")),
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn read() -> Result<Option<String>, String> {
    let primary = path()?;
    let legacy = legacy_path().ok();
    read_native_save(&primary, legacy.as_deref())
}

#[cfg(not(target_arch = "wasm32"))]
fn write(json: &str) -> Result<(), String> {
    use std::io::Write;
    let path = path()?;
    let directory = path.parent().ok_or("Invalid save path")?;
    std::fs::create_dir_all(directory)
        .map_err(|error| format!("Cannot create save directory: {error}"))?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| format!("Cannot timestamp save: {error}"))?
        .as_nanos();
    let temporary = directory.join(format!(".save-{}-{stamp}.tmp", std::process::id()));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|error| format!("Cannot create temporary save: {error}"))?;
    let result = (|| {
        file.write_all(json.as_bytes())?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&temporary, &path)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result.map_err(|error| format!("Cannot save data: {error}"))
}

#[cfg(target_arch = "wasm32")]
fn browser_storage() -> Result<web_sys::Storage, String> {
    web_sys::window()
        .ok_or("Browser window unavailable")?
        .local_storage()
        .map_err(|_| "Browser storage access denied")?
        .ok_or_else(|| "Browser storage unavailable".into())
}

#[cfg(target_arch = "wasm32")]
fn read() -> Result<Option<String>, String> {
    browser_storage()?
        .get_item(STORAGE_KEY)
        .map_err(|_| "Cannot read browser save data".into())
}

#[cfg(target_arch = "wasm32")]
fn write(json: &str) -> Result<(), String> {
    browser_storage()?
        .set_item(STORAGE_KEY, json)
        .map_err(|_| "Cannot save: browser storage is unavailable or its quota is exhausted".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classic_custom_level_names_migrate_to_lowercase_filename_identity() {
        let data = SaveData::from_json(
            r#"{"version":1,"custom_levels":{
                "Rooms/Example.LVL":"path variant",
                "EXAMPLE":"case variant",
                "example":"canonical file",
                "!Hidden":"hidden",
                "double.lvl":"double extension"
            }}"#,
        )
        .unwrap();

        assert_eq!(data.custom_levels.len(), 3);
        assert_eq!(data.custom_levels["example"], "canonical file");
        assert_eq!(data.custom_levels["!hidden"], "hidden");
        assert_eq!(data.custom_levels["double.lvl"], "double extension");
    }

    #[test]
    fn version_two_save_round_trip_preserves_nested_custom_level_paths() {
        let mut data = SaveData::default();
        data.custom_levels
            .insert("start".into(), "start text".into());
        data.custom_levels
            .insert("sub/next".into(), "nested text".into());
        data.custom_levels
            .insert("sub/deep/room.lvl".into(), "double extension".into());
        data.custom_levels
            .insert(".lvl".into(), "double extension root".into());

        let json = data.to_json().unwrap();
        assert!(json.contains("\"version\": 2"));
        let decoded = SaveData::from_json(&json).unwrap();
        assert_eq!(decoded.custom_levels["start"], "start text");
        assert_eq!(decoded.custom_levels["sub/next"], "nested text");
        assert_eq!(
            decoded.custom_levels["sub/deep/room.lvl"],
            "double extension"
        );
        assert_eq!(decoded.custom_levels[".lvl"], "double extension root");
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn native_save_directory_uses_each_platforms_user_data_location() {
        let env = NativeDataEnvironment {
            xdg_data_home: Some("/home/player/game-data".into()),
            home: Some("/home/player".into()),
            appdata: Some("/users/player/AppData/Roaming".into()),
            local_appdata: Some("/users/player/AppData/Local".into()),
            user_profile: Some("/users/player".into()),
        };
        assert_eq!(
            native_data_directory(NativePlatform::Unix, &env).unwrap(),
            std::path::PathBuf::from("/home/player/game-data")
        );
        assert_eq!(
            native_data_directory(NativePlatform::MacOs, &env).unwrap(),
            std::path::PathBuf::from("/home/player/Library/Application Support")
        );
        assert_eq!(
            native_data_directory(NativePlatform::Windows, &env).unwrap(),
            std::path::PathBuf::from("/users/player/AppData/Roaming")
        );

        let mut windows_without_appdata = env.clone();
        windows_without_appdata.appdata = None;
        windows_without_appdata.local_appdata = None;
        assert_eq!(
            native_data_directory(NativePlatform::Windows, &windows_without_appdata).unwrap(),
            std::path::PathBuf::from("/users/player/AppData/Roaming")
        );
        windows_without_appdata.user_profile = None;
        assert_eq!(
            native_data_directory(NativePlatform::Windows, &windows_without_appdata).unwrap(),
            std::path::PathBuf::from("/home/player/.local/share")
        );

        let unix_without_xdg = NativeDataEnvironment {
            xdg_data_home: None,
            ..env
        };
        assert_eq!(
            native_data_directory(NativePlatform::Unix, &unix_without_xdg).unwrap(),
            std::path::PathBuf::from("/home/player/.local/share")
        );
        let relative_xdg = NativeDataEnvironment {
            xdg_data_home: Some("relative-data".into()),
            ..unix_without_xdg
        };
        assert!(native_data_directory(NativePlatform::Unix, &relative_xdg).is_err());
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn native_save_reads_the_new_location_first_then_falls_back_to_the_old_path() {
        use std::time::{SystemTime, UNIX_EPOCH};

        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "spelunky-storage-migration-{}-{stamp}",
            std::process::id()
        ));
        std::fs::create_dir_all(&directory).unwrap();
        let primary = directory.join("current/save.json");
        let legacy = directory.join("legacy/save.json");
        std::fs::create_dir_all(legacy.parent().unwrap()).unwrap();
        std::fs::write(&legacy, "legacy profile").unwrap();

        assert_eq!(
            read_native_save(&primary, Some(&legacy)).unwrap(),
            Some("legacy profile".into())
        );
        std::fs::create_dir_all(primary.parent().unwrap()).unwrap();
        std::fs::write(&primary, "current profile").unwrap();
        assert_eq!(
            read_native_save(&primary, Some(&legacy)).unwrap(),
            Some("current profile".into())
        );
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn first_time_onboarding_survives_settings_saves_until_consumed() {
        let fresh = Store::from_read(Ok(None));
        assert!(fresh.data.first_time);
        let json = fresh.data.to_json().unwrap();
        assert!(json.contains("\"first_time\": true"));
        let restarted = Store::from_read(Ok(Some(json)));
        assert!(restarted.data.first_time);
        assert!(restarted.data.records.is_empty());
        assert!(restarted.data.unlocks.is_empty());

        let mut consumed = restarted.data;
        consumed.first_time = false;
        let consumed_json = consumed.to_json().unwrap();
        assert!(!Store::from_read(Ok(Some(consumed_json))).data.first_time);

        for legacy in ["{}", r#"{"version":0}"#, r#"{"version":1}"#] {
            assert!(
                !SaveData::from_json(legacy).unwrap().first_time,
                "legacy save without a first_time field must not be treated as new"
            );
        }
        assert!(
            SaveData::from_json(r#"{"version":1,"first_time":true}"#)
                .unwrap()
                .first_time
        );
    }

    #[test]
    fn unreadable_storage_does_not_install_fresh_profile_onboarding() {
        for result in [
            Ok(Some("invalid".into())),
            Ok(Some(r#"{"version":99}"#.into())),
            Err("Storage access denied".into()),
        ] {
            let store = Store::from_read(result);
            assert!(!store.data.first_time);
            assert!(store.overwrite_blocked);
            assert!(store.warning.is_some());
        }
    }

    #[test]
    fn unreadable_save_blocks_overwriting_the_legacy_stats_report_with_defaults() {
        for result in [
            Ok(Some("invalid".into())),
            Ok(Some(r#"{"version":99}"#.into())),
            Err("Storage access denied".into()),
        ] {
            let store = Store::from_read(result);
            assert!(store.overwrite_blocked);
            assert!(!store.may_write_classic_stats_report());
        }

        assert!(Store::from_read(Ok(None)).may_write_classic_stats_report());
    }

    #[test]
    fn current_save_round_trip_preserves_records_rules_and_levels() {
        let mut data = SaveData::default();
        data.settings.rules = RulesPreset::Assisted;
        data.settings.frame_cap = Some(240);
        data.records.push(Record {
            seed: 42,
            rng_algorithm: RngAlgorithm::Gm8Lcg32V1,
            simulation_version: spelunky_core::SIMULATION_VERSION,
            rules: RulesPreset::Classic,
            score: 4000,
            level: 3,
            ticks: 900,
            elapsed_ms: Some(27_000),
            fastest_eligible: true,
            won: false,
            kills: 12,
            rescues: 2,
        });
        data.records.push(Record {
            seed: 42,
            rng_algorithm: RngAlgorithm::LegacySplitMix64V1,
            simulation_version: 0,
            rules: RulesPreset::Assisted,
            score: 6000,
            level: 4,
            ticks: 800,
            elapsed_ms: Some(24_000),
            fastest_eligible: false,
            won: false,
            kills: 15,
            rescues: 3,
        });
        data.unlocks.insert("jungle-shortcut".into());
        data.custom_levels
            .insert("test".into(), "level text\nkept verbatim\n".into());
        let decoded = SaveData::from_json(&data.to_json().unwrap()).unwrap();
        assert_eq!(decoded.settings.rules, RulesPreset::Assisted);
        assert_eq!(decoded.settings.frame_cap, Some(240));
        assert_eq!(decoded.records[0].rules, RulesPreset::Classic);
        assert_eq!(decoded.records[1].rules, RulesPreset::Assisted);
        assert_eq!(decoded.records[0].rng_algorithm, RngAlgorithm::Gm8Lcg32V1);
        assert_eq!(
            decoded.records[0].simulation_version,
            spelunky_core::SIMULATION_VERSION
        );
        assert_eq!(
            decoded.records[1].rng_algorithm,
            RngAlgorithm::LegacySplitMix64V1
        );
        assert_eq!(decoded.records[1].simulation_version, 0);
        assert_eq!(decoded.records[0].kills, 12);
        assert_eq!(decoded.records[0].rescues, 2);
        assert_eq!(decoded.records[1].kills, 15);
        assert_eq!(decoded.records[1].rescues, 3);
        assert_eq!(decoded.records[0].elapsed_ms(), 27_000);
        assert!(!decoded.records[1].fastest_eligible);
        assert_eq!(decoded.custom_levels, data.custom_levels);
        assert_eq!(decoded.unlocks, data.unlocks);
    }

    #[test]
    fn unversioned_and_v0_data_migrate_with_defaults_and_remaps() {
        for json in [
            r#"{"settings":{"bindings":{"jump":"KeyJ"}}}"#,
            r#"{"version":0,"settings":{"bindings":{"jump":"KeyJ"}}}"#,
        ] {
            let data = SaveData::from_json(json).unwrap();
            assert_eq!(data.version, SAVE_VERSION);
            assert_eq!(data.settings.bindings["jump"], "KeyJ");
            assert!(data.settings.bindings.contains_key("rope"));
            assert_eq!(data.settings.bindings["bomb"], "KeyA");
            assert_eq!(data.settings.bindings["rope"], "KeyS");
            assert!(data.settings.graphics_high);
            assert!(!data.settings.vsync);
            assert!(!data.settings.pixel_snap);
            assert!(data.settings.fullscreen);
        }
    }

    #[test]
    fn fullscreen_preference_defaults_on_for_legacy_saves_and_round_trips() {
        let legacy = SaveData::from_json(r#"{"version":1,"settings":{}}"#).unwrap();
        assert!(legacy.settings.fullscreen);

        let mut windowed = legacy;
        windowed.settings.fullscreen = false;
        let reopened = SaveData::from_json(&windowed.to_json().unwrap()).unwrap();
        assert!(!reopened.settings.fullscreen);
    }

    #[test]
    fn graphics_detail_migrates_to_classic_high_and_preserves_explicit_low() {
        assert!(
            SaveData::from_json(r#"{"settings":{}}"#)
                .unwrap()
                .settings
                .graphics_high
        );
        let low = SaveData::from_json(r#"{"settings":{"graphics_high":false}}"#).unwrap();
        assert!(!low.settings.graphics_high);
        let reopened = SaveData::from_json(&low.to_json().unwrap()).unwrap();
        assert!(!reopened.settings.graphics_high);
    }

    #[test]
    fn fresh_keyboard_defaults_match_classic_controls_without_conflicting_aliases() {
        let bindings = Settings::default().bindings;
        for (action, key) in [
            ("left", "ArrowLeft"),
            ("right", "ArrowRight,KeyD"),
            ("up", "ArrowUp,KeyW"),
            ("down", "ArrowDown"),
            ("jump", "KeyZ,Space"),
            ("attack", "KeyX"),
            ("cycle", "KeyC"),
            ("bomb", "KeyA"),
            ("rope", "KeyS"),
            ("flare", "KeyF"),
            ("pay", "KeyP"),
            ("run", "ShiftLeft,ShiftRight"),
        ] {
            assert_eq!(bindings[action], key, "{action}");
        }
    }

    #[test]
    fn fresh_audio_defaults_match_classic_volume_formula() {
        let settings = Settings::default();
        let source_default = (2000.0 + 8000.0 * (15.0 / 18.0)) / 10000.0;

        assert!((settings.music_volume - source_default).abs() < f32::EPSILON);
        assert!((settings.effects_volume - source_default).abs() < f32::EPSILON);
    }

    #[test]
    fn legacy_saves_receive_controller_defaults_without_changing_keyboard_remaps() {
        for json in [
            r#"{}"#,
            r#"{"version":0,"settings":{"bindings":{"jump":"KeyJ","attack":""}}}"#,
            r#"{"version":1,"settings":{"bindings":{"jump":"KeyJ","attack":""}}}"#,
        ] {
            let data = SaveData::from_json(json).unwrap();
            assert_eq!(
                data.settings.controller_bindings,
                crate::controller::default_bindings()
            );
            assert_eq!(
                data.settings.browser_direction_bindings,
                crate::controller::BrowserDirectionBindings::default()
            );
            assert!(!data.settings.controller_enabled);
            if json != "{}" {
                assert_eq!(data.settings.bindings["jump"], "KeyJ");
                assert_eq!(data.settings.bindings["attack"], "");
            }
        }
    }

    #[test]
    fn controller_remaps_survive_save_round_trip_with_records_unchanged() {
        let mut data = SaveData::default();
        data.settings.controller_enabled = true;
        data.settings
            .controller_bindings
            .insert("jump".into(), "East".into());
        data.settings
            .controller_bindings
            .insert("start".into(), "Mode".into());
        data.settings.browser_direction_bindings = crate::controller::BrowserDirectionBindings {
            left: Some(crate::controller::BrowserGamepadControl::Axis {
                index: 2,
                positive: false,
            }),
            right: Some(crate::controller::BrowserGamepadControl::Axis {
                index: 2,
                positive: true,
            }),
            up: Some(crate::controller::BrowserGamepadControl::Button { index: 16 }),
            down: Some(crate::controller::BrowserGamepadControl::Button { index: 17 }),
        };
        data.settings.bindings.insert("jump".into(), "KeyJ".into());
        data.unlocks.insert("shortcut_5".into());
        data.challenge_records.insert("mini1".into(), 37);
        let decoded = SaveData::from_json(&data.to_json().unwrap()).unwrap();
        assert_eq!(
            decoded.settings.controller_bindings,
            data.settings.controller_bindings
        );
        assert_eq!(
            decoded.settings.browser_direction_bindings,
            data.settings.browser_direction_bindings
        );
        assert!(decoded.settings.controller_enabled);
        assert_eq!(decoded.settings.bindings, data.settings.bindings);
        assert_eq!(decoded.unlocks, data.unlocks);
        assert_eq!(decoded.challenge_records, data.challenge_records);
    }

    #[test]
    fn invalid_controller_values_normalize_and_missing_actions_receive_defaults() {
        let data = SaveData::from_json(
            r#"{"settings":{"controller_bindings":{"jump":"B1","attack":"","cycle":"Button18","run":"RightStick","unknown":"East"}}}"#,
        )
        .unwrap();
        let mut expected = crate::controller::default_bindings();
        expected.insert("run".into(), "RightStick".into());
        assert_eq!(data.settings.controller_bindings, expected);
        let reopened = SaveData::from_json(&data.to_json().unwrap()).unwrap();
        assert_eq!(reopened.settings.controller_bindings, expected);
    }

    #[test]
    fn duplicate_controller_assignments_remain_valid_after_save_round_trip() {
        let data = SaveData::from_json(
            r#"{"settings":{"controller_bindings":{"jump":"South","attack":"South","start":"South"}}}"#,
        )
        .unwrap();
        let decoded = SaveData::from_json(&data.to_json().unwrap()).unwrap();
        for action in ["jump", "attack", "start"] {
            assert_eq!(decoded.settings.controller_bindings[action], "South");
        }
        assert_eq!(decoded.settings.controller_bindings.len(), 9);
    }

    #[test]
    fn historical_run_records_default_kills_and_rescues_to_zero() {
        for version in [0, SAVE_VERSION] {
            let json = format!(
                r#"{{"version":{version},"records":[{{"seed":42,"rules":"Classic","score":4000,"level":3,"ticks":900,"won":false}},{{"seed":43,"rules":"Assisted","score":8000,"level":5,"ticks":1200,"won":false}}]}}"#
            );
            let data = SaveData::from_json(&json).unwrap();
            assert_eq!(data.version, SAVE_VERSION);
            assert_eq!(data.records.len(), 2);
            for record in &data.records {
                assert_eq!(record.rng_algorithm, RngAlgorithm::LegacySplitMix64V1);
                assert_eq!(record.simulation_version, 0);
                assert_eq!(record.kills, 0);
                assert_eq!(record.rescues, 0);
                assert!(record.fastest_eligible);
                assert_eq!(record.elapsed_ms(), record.ticks * 30);
            }
            assert_eq!(data.records[0].rules, RulesPreset::Classic);
            assert_eq!(data.records[1].rules, RulesPreset::Assisted);
            let decoded = SaveData::from_json(&data.to_json().unwrap()).unwrap();
            assert_eq!(decoded.records[0].score, 4000);
            assert_eq!(decoded.records[1].score, 8000);
        }
    }

    #[test]
    fn historical_record_identity_preserves_full_width_seed_and_profile() {
        let json = r#"{
            "version":1,
            "settings":{"rules":"Assisted","frame_cap":240,"bindings":{"jump":"KeyJ"}},
            "records":[{"seed":18446744073709551615,"rules":"Classic","score":12345,
                        "level":16,"ticks":900,"elapsed_ms":27000,"fastest_eligible":false,
                        "won":true,"kills":12,"rescues":3}],
            "lifetime_stats":{"total_crates":17},
            "challenge_records":{"mini1":42},
            "tunnel_remaining":{"tunnel1_remaining":12345},
            "unlocks":["character_damsel","shortcut_5"],
            "custom_levels":{"test":"level text\nkept verbatim\n"}
        }"#;
        let loaded = SaveData::from_json(json).unwrap();
        let record = &loaded.records[0];
        assert_eq!(record.seed, u64::MAX);
        assert_eq!(record.rng_algorithm, RngAlgorithm::LegacySplitMix64V1);
        assert_eq!(record.simulation_version, 0);
        assert_eq!(record.score, 12345);
        assert_eq!(record.level, 16);
        assert_eq!(record.ticks, 900);
        assert_eq!(record.elapsed_ms(), 27000);
        assert_eq!((record.kills, record.rescues), (12, 3));
        assert!(record.won);
        assert!(!record.fastest_eligible);
        assert_eq!(loaded.settings.rules, RulesPreset::Assisted);
        assert_eq!(loaded.settings.frame_cap, Some(240));
        assert_eq!(loaded.settings.bindings["jump"], "KeyJ");
        assert_eq!(loaded.lifetime_stats.total_crates, 17);
        assert_eq!(loaded.challenge_records["mini1"], 42);
        assert_eq!(loaded.tunnel_remaining["tunnel1_remaining"], 12345);
        let encoded = loaded.to_json().unwrap();
        let reopened = SaveData::from_json(&encoded).unwrap();
        assert_eq!(
            serde_json::to_value(&reopened).unwrap(),
            serde_json::to_value(&loaded).unwrap()
        );
        assert!(reopened.unlocks.contains("character_damsel"));
        assert!(reopened.unlocks.contains("shortcut_5"));
        assert_eq!(
            reopened.custom_levels["test"],
            "level text\nkept verbatim\n"
        );
    }

    #[test]
    fn canonical_record_history_requires_bounded_seed_and_explicit_version() {
        let record = |seed: u64, metadata: &str| {
            format!(
                r#"{{"records":[{{"seed":{seed},"rules":"Classic","score":4000,"level":3,"ticks":900,"won":false,{metadata}}}]}}"#
            )
        };
        for simulation_version in [1, u32::MAX] {
            let json = record(
                u64::from(u32::MAX),
                &format!(
                    r#""rng_algorithm":"gm8_lcg32_v1","simulation_version":{simulation_version}"#
                ),
            );
            let saved = SaveData::from_json(&json).unwrap();
            let reopened = SaveData::from_json(&saved.to_json().unwrap()).unwrap();
            assert_eq!(reopened.records[0].rng_algorithm, RngAlgorithm::Gm8Lcg32V1);
            assert_eq!(reopened.records[0].simulation_version, simulation_version);
            assert_eq!(reopened.records[0].seed, u64::from(u32::MAX));
            assert!(reopened.records[0].fastest_eligible);
        }
        for (seed, metadata) in [
            (
                u64::from(u32::MAX) + 1,
                r#""rng_algorithm":"gm8_lcg32_v1","simulation_version":1"#,
            ),
            (42, r#""rng_algorithm":"gm8_lcg32_v1"#),
            (
                42,
                r#""rng_algorithm":"gm8_lcg32_v1","simulation_version":0"#,
            ),
            (42, r#""rng_algorithm":"future_rng","simulation_version":1"#),
        ] {
            assert!(SaveData::from_json(&record(seed, metadata)).is_err());
        }
        let legacy = SaveData::from_json(&record(
            u64::MAX,
            r#""rng_algorithm":"legacy_splitmix64_v1","simulation_version":1"#,
        ))
        .unwrap();
        assert_eq!(legacy.records[0].seed, u64::MAX);
        assert_eq!(legacy.records[0].simulation_version, 1);
    }

    #[test]
    fn challenge_records_extend_existing_saves_without_version_change() {
        for json in ["{}", r#"{"version":0}"#, r#"{"version":1}"#] {
            let mut data = SaveData::from_json(json).unwrap();
            assert!(data.challenge_records.is_empty());
            data.challenge_records.extend([
                ("mini1".into(), 45),
                ("mini2".into(), 8),
                ("mini3".into(), 101),
            ]);
            let decoded = SaveData::from_json(&data.to_json().unwrap()).unwrap();
            assert_eq!(decoded.version, SAVE_VERSION);
            assert_eq!(decoded.challenge_records, data.challenge_records);
        }
        let data =
            SaveData::from_json(r#"{"version":1,"challenge_records":{"future-challenge":42}}"#)
                .unwrap();
        assert_eq!(data.challenge_records["future-challenge"], 42);
        for invalid in ["-1", "1.5", "4294967296", "null", "\"8\""] {
            let json = format!(r#"{{"version":1,"challenge_records":{{"mini1":{invalid}}}}}"#);
            assert!(SaveData::from_json(&json).is_err(), "{json}");
        }
    }

    #[test]
    fn tunnel_donations_migrate_and_round_trip_without_version_change() {
        for json in ["{}", r#"{"version":0}"#, r#"{"version":1}"#] {
            let mut data = SaveData::from_json(json).unwrap();
            assert!(data.tunnel_remaining.is_empty());
            data.tunnel_remaining.extend([
                ("tunnel1_remaining".into(), 0),
                ("tunnel2_remaining".into(), 125_000),
                ("tunnel3_remaining".into(), 300_000),
            ]);
            let decoded = SaveData::from_json(&data.to_json().unwrap()).unwrap();
            assert_eq!(decoded.version, SAVE_VERSION);
            assert_eq!(decoded.tunnel_remaining, data.tunnel_remaining);
        }
    }

    #[test]
    fn tunnel_donations_reject_out_of_bounds_values() {
        for (key, limit) in [
            ("tunnel1_remaining", 100_001),
            ("tunnel2_remaining", 200_001),
            ("tunnel3_remaining", 300_000),
        ] {
            for invalid in [
                "-1".into(),
                "1.5".into(),
                "null".into(),
                (limit + 1).to_string(),
            ] {
                let json = format!(r#"{{"version":1,"tunnel_remaining":{{"{key}":{invalid}}}}}"#);
                assert!(SaveData::from_json(&json).is_err(), "{json}");
            }
            let mut data = SaveData::default();
            data.tunnel_remaining.insert(key.into(), limit + 1);
            assert!(data.to_json().is_err());
        }
    }

    #[test]
    fn invalid_data_is_preserved_by_overwrite_guard() {
        for json in [
            "broken",
            "null",
            "[]",
            r#"{"version":99}"#,
            r#"{"version":-1}"#,
            r#"{"settings":{"music_volume":2}}"#,
        ] {
            let mut store = Store::from_read(Ok(Some(json.into())));
            assert!(store.warning.is_some(), "{json}");
            assert!(store.overwrite_blocked);
            // The guard returns before any filesystem or browser write.
            assert!(store.save().is_err());
        }
    }

    #[test]
    fn invalid_settings_are_rejected_before_serialization() {
        let mut data = SaveData::default();
        data.settings.effects_volume = f32::NAN;
        assert!(data.to_json().is_err());
        data.settings.effects_volume = 0.5;
        for cap in [0, 1001, u32::MAX] {
            data.settings.frame_cap = Some(cap);
            assert!(data.to_json().is_err());
        }
        data.settings.frame_cap = Some(240);
        assert!(data.to_json().is_ok());
    }

    #[test]
    fn unavailable_storage_does_not_panic_or_discard_runtime_data() {
        let mut store = Store::from_read(Err("Storage access denied".into()));
        store.data.unlocks.insert("earned-this-session".into());
        assert!(store.save().is_err());
        assert!(store.data.unlocks.contains("earned-this-session"));
        assert!(store.warning.unwrap().contains("Storage access denied"));
    }
    #[test]
    fn lifetime_statistics_migrate_and_preserve_all_89_source_counters() {
        for json in ["{}", r#"{"version":0}"#, r#"{"version":1}"#] {
            assert_eq!(
                SaveData::from_json(json).unwrap().lifetime_stats,
                spelunky_core::statistics::LifetimeStats::default()
            );
        }
        let mut fields =
            serde_json::to_value(spelunky_core::statistics::LifetimeStats::default()).unwrap();
        let mut counter = 0u64;
        for value in fields.as_object_mut().unwrap().values_mut() {
            if let serde_json::Value::Array(entries) = value {
                for entry in entries {
                    counter += 1;
                    *entry = counter.into();
                }
            } else {
                counter += 1;
                *value = counter.into();
            }
        }
        assert_eq!(counter, 89);
        let mut data = SaveData::default();
        data.lifetime_stats = serde_json::from_value(fields.clone()).unwrap();
        let reopened = SaveData::from_json(&data.to_json().unwrap()).unwrap();
        assert_eq!(
            serde_json::to_value(reopened.lifetime_stats).unwrap(),
            fields
        );
        let partial = SaveData::from_json(r#"{"lifetime_stats":{"total_crates":17}}"#).unwrap();
        assert_eq!(partial.lifetime_stats.total_crates, 17);
        assert_eq!(partial.lifetime_stats.level_deaths, [0; 16]);
    }

    #[test]
    fn classic_stats_migrate_only_without_a_rust_save_and_then_remain_one_time() {
        let mut legacy_stats = spelunky_core::statistics::LifetimeStats::default();
        legacy_stats.total_crates = 41;
        legacy_stats.enemy_kills[8] = 3;
        legacy_stats.total_monkey_kills = 2;
        let legacy_report = legacy_stats.to_classic_report();

        let (first, imported) =
            Store::from_read_and_classic_stats(Ok(None), Ok(Some(legacy_report.clone())));
        assert!(imported);
        assert!(first.data.classic_stats_imported);
        assert_eq!(first.data.lifetime_stats, legacy_stats);
        let saved_json = first.data.to_json().unwrap();
        assert!(saved_json.contains("classic_stats_imported"));
        let newer_legacy = spelunky_core::statistics::LifetimeStats::default().to_classic_report();
        let (reopened, imported_again) =
            Store::from_read_and_classic_stats(Ok(Some(saved_json)), Ok(Some(newer_legacy)));
        assert!(!imported_again);
        assert_eq!(reopened.data.lifetime_stats, legacy_stats);

        let mut reopened = reopened;
        assert!(reopened.import_classic_stats(&legacy_report).is_err());
        assert_eq!(reopened.data.lifetime_stats, legacy_stats);
    }

    #[test]
    fn classic_stats_migrate_into_older_rust_saves_without_lifetime_stats() {
        let mut legacy_stats = spelunky_core::statistics::LifetimeStats::default();
        legacy_stats.total_crates = 41;
        legacy_stats.damsels_bought = 7;
        legacy_stats.enemy_kills[8] = 3;
        legacy_stats.total_monkey_kills = 2;
        let legacy_report = legacy_stats.to_classic_report();
        let old_save = r#"{"version":1,"first_time":false,"unlocks":["old-unlock"]}"#;

        let (migrated, imported) = Store::from_read_and_classic_stats(
            Ok(Some(old_save.into())),
            Ok(Some(legacy_report.clone())),
        );

        assert!(imported);
        assert!(migrated.data.classic_stats_imported);
        assert_eq!(migrated.data.lifetime_stats, legacy_stats);
        assert!(!migrated.data.first_time);
        assert!(migrated.data.unlocks.contains("old-unlock"));
        let saved = SaveData::from_json(&migrated.data.to_json().unwrap()).unwrap();
        assert_eq!(saved.lifetime_stats, legacy_stats);
        assert!(saved.classic_stats_imported);

        let current_stats = spelunky_core::statistics::LifetimeStats::default();
        let current_save = SaveData {
            lifetime_stats: current_stats.clone(),
            ..SaveData::default()
        }
        .to_json()
        .unwrap();
        let (current, imported) =
            Store::from_read_and_classic_stats(Ok(Some(current_save)), Ok(Some(legacy_report)));
        assert!(!imported);
        assert_eq!(current.data.lifetime_stats, current_stats);
        assert!(!current.data.classic_stats_imported);

        let (future, imported) = Store::from_read_and_classic_stats(
            Ok(Some(r#"{"version":999}"#.into())),
            Ok(Some(legacy_stats.to_classic_report())),
        );
        assert!(!imported);
        assert!(future.overwrite_blocked);
        assert!(!future.may_write_classic_stats_report());
        assert_eq!(
            future.data.lifetime_stats,
            spelunky_core::statistics::LifetimeStats::default()
        );
    }

    #[test]
    fn malformed_classic_stats_do_not_mark_or_replace_a_fresh_save() {
        let (mut store, imported) =
            Store::from_read_and_classic_stats(Ok(None), Ok(Some("not stats.txt".into())));
        assert!(!imported);
        assert!(!store.data.classic_stats_imported);
        assert_eq!(store.data.lifetime_stats.total_crates, 0);
        assert!(store.import_classic_stats("not stats.txt").is_err());
        assert!(!store.data.classic_stats_imported);
    }

    #[test]
    fn down_to_run_legacy_settings_default_and_explicit_false_round_trip() {
        let legacy: Settings = serde_json::from_str("{}").unwrap();
        assert!(legacy.down_to_run);
        let disabled: Settings = serde_json::from_str(r#"{"down_to_run":false}"#).unwrap();
        assert!(!disabled.down_to_run);
        let serialized = serde_json::to_string(&disabled).unwrap();
        let reopened: Settings = serde_json::from_str(&serialized).unwrap();
        assert!(!reopened.down_to_run);
        assert_eq!(reopened.rules, disabled.rules);
        assert_eq!(reopened.bindings, disabled.bindings);
    }

    #[test]
    fn legacy_up_remap_is_used_for_new_door_binding() {
        let data =
            SaveData::from_json(r#"{"version":1,"settings":{"bindings":{"up":"KeyI"}}}"#).unwrap();
        assert_eq!(data.settings.bindings["exit"], "KeyI");
        let explicitly_rebound = SaveData::from_json(
            r#"{"version":1,"settings":{"bindings":{"up":"KeyI","exit":"KeyO"}}}"#,
        )
        .unwrap();
        assert_eq!(explicitly_rebound.settings.bindings["exit"], "KeyO");
        let up_unbound =
            SaveData::from_json(r#"{"version":1,"settings":{"bindings":{"up":""}}}"#).unwrap();
        assert_eq!(up_unbound.settings.bindings["exit"], "");

        let mut input = crate::input::InputState::default();
        input.set_bindings(&data.settings.bindings);
        input.key(winit::keyboard::KeyCode::ArrowUp, true, false);
        let former_default = input.take_tick();
        assert!(!former_default.up);
        assert!(!former_default.exit_pressed);

        input.key(winit::keyboard::KeyCode::ArrowUp, false, false);
        input.key(winit::keyboard::KeyCode::KeyI, true, false);
        let remapped = input.take_tick();
        assert!(remapped.up);
        assert!(remapped.up_pressed);
        assert!(remapped.exit_pressed);
    }
}
