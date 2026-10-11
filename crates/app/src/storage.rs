//! Versioned settings and run history, shared by the native and web hosts.
use serde::{Deserialize, Serialize};
use spelunky_core::RulesPreset;
use std::collections::{BTreeMap, BTreeSet};

pub const SAVE_VERSION: u32 = 1;
#[cfg(target_arch = "wasm32")]
const STORAGE_KEY: &str = "spelunky-rust.save";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub rules: RulesPreset,
    pub pixel_snap: bool,
    pub music_volume: f32,
    pub effects_volume: f32,
    pub vsync: bool,
    pub frame_cap: Option<u32>,
    /// Action names map to winit physical-key names, separated by commas.
    pub bindings: BTreeMap<String, String>,
    /// Classic controller actions map to portable physical button names.
    pub controller_bindings: BTreeMap<String, String>,
}

impl Default for Settings {
    fn default() -> Self {
        let bindings = [
            ("left", "ArrowLeft,KeyA"),
            ("right", "ArrowRight,KeyD"),
            ("up", "ArrowUp,KeyW"),
            ("down", "ArrowDown,KeyS"),
            ("jump", "KeyZ,Space"),
            ("attack", "KeyX"),
            ("cycle", "KeyC"),
            ("bomb", "KeyB"),
            ("rope", "KeyR"),
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
            music_volume: 0.6,
            effects_volume: 0.8,
            vsync: true,
            frame_cap: None,
            bindings,
            controller_bindings: crate::controller::default_bindings(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    pub seed: u64,
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
    /// Source firstTime is a runtime sentinel for newly initialized storage.
    /// An existing save starts with it false, even before a completed run.
    #[serde(skip)]
    pub first_time: bool,
    pub settings: Settings,
    pub records: Vec<Record>,
    /// Original lifetime counters are independent of run records and unlocks.
    #[serde(default)]
    pub lifetime_stats: spelunky_core::statistics::LifetimeStats,
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
            challenge_records: BTreeMap::new(),
            tunnel_remaining: BTreeMap::new(),
            unlocks: BTreeSet::new(),
            custom_levels: BTreeMap::new(),
        }
    }
}

impl SaveData {
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
        // Version zero used the same fields without a required schema marker.
        data.version = SAVE_VERSION;
        // Classic initializes storage on its first launch. Later launches do
        // not repeat the firstTime tutorial gate, regardless of score values.
        data.first_time = false;
        // Newly introduced actions receive defaults; existing remaps and
        // deliberately empty (unbound) actions remain intact.
        for (action, keys) in Settings::default().bindings {
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
        Self::from_read(read())
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

#[cfg(not(target_arch = "wasm32"))]
fn path() -> Result<std::path::PathBuf, String> {
    use std::path::PathBuf;
    let base =
        if let Some(config) = std::env::var_os("XDG_CONFIG_HOME").filter(|path| !path.is_empty()) {
            let path = PathBuf::from(config);
            if !path.is_absolute() {
                return Err("XDG_CONFIG_HOME must be an absolute path".into());
            }
            path
        } else {
            let home = std::env::var_os("HOME")
                .filter(|path| !path.is_empty())
                .ok_or("No home directory available for save data")?;
            PathBuf::from(home).join(".local/share")
        };
    Ok(base.join("spelunky-rust/save.json"))
}

#[cfg(not(target_arch = "wasm32"))]
fn read() -> Result<Option<String>, String> {
    match std::fs::read_to_string(path()?) {
        Ok(json) => Ok(Some(json)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("Cannot read save data: {error}")),
    }
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
    fn only_missing_storage_initializes_the_runtime_first_time_sentinel() {
        let fresh = Store::from_read(Ok(None));
        assert!(fresh.data.first_time);
        let json = fresh.data.to_json().unwrap();
        assert!(!json.contains("first_time"));
        let restarted = Store::from_read(Ok(Some(json)));
        assert!(!restarted.data.first_time);
        assert!(restarted.data.records.is_empty());
        assert!(restarted.data.unlocks.is_empty());
        for existing in [
            "{}",
            r#"{"version":0}"#,
            r#"{"version":1,"first_time":true}"#,
        ] {
            assert!(!SaveData::from_json(existing).unwrap().first_time);
        }
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
    fn current_save_round_trip_preserves_records_rules_and_levels() {
        let mut data = SaveData::default();
        data.settings.rules = RulesPreset::Assisted;
        data.settings.frame_cap = Some(240);
        data.records.push(Record {
            seed: 42,
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
            assert!(data.settings.vsync);
            assert!(!data.settings.pixel_snap);
        }
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
            if json != "{}" {
                assert_eq!(data.settings.bindings["jump"], "KeyJ");
                assert_eq!(data.settings.bindings["attack"], "");
            }
        }
    }

    #[test]
    fn controller_remaps_survive_save_round_trip_with_records_unchanged() {
        let mut data = SaveData::default();
        data.settings
            .controller_bindings
            .insert("jump".into(), "East".into());
        data.settings
            .controller_bindings
            .insert("start".into(), "Mode".into());
        data.settings.bindings.insert("jump".into(), "KeyJ".into());
        data.unlocks.insert("shortcut_5".into());
        data.challenge_records.insert("mini1".into(), 37);
        let decoded = SaveData::from_json(&data.to_json().unwrap()).unwrap();
        assert_eq!(
            decoded.settings.controller_bindings,
            data.settings.controller_bindings
        );
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
}
