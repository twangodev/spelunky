//! User-selected text import/export for native and browser hosts.

/// Explicit user-requested export; native exports never overwrite an existing file.
pub fn export(name: &str, text: &str) -> Result<(), String> {
    let filename = export_name(name)?;
    #[cfg(target_arch = "wasm32")]
    {
        download(&filename, text)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&filename)
            .map_err(|error| {
                if error.kind() == std::io::ErrorKind::AlreadyExists {
                    format!("{filename} already exists; use N to choose another name")
                } else {
                    format!("Cannot export {filename}: {error}")
                }
            })?;
        file.write_all(text.as_bytes())
            .map_err(|error| format!("Cannot write {filename}: {error}"))?;
        Ok(())
    }
}

/// Save the editor's active level back to the native `levels/` catalog. Classic
/// saves the selected level in place, while explicit Export remains a separate
/// create-only action in the working directory.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn save_native_level(name: &str, text: &str) -> Result<(), String> {
    save_native_level_to(std::path::Path::new("levels"), name, text)
}

#[cfg(not(target_arch = "wasm32"))]
fn save_native_level_to(directory: &std::path::Path, name: &str, text: &str) -> Result<(), String> {
    let filename = relative_level_filename(name)?;
    std::fs::create_dir_all(directory).map_err(|error| {
        format!(
            "Cannot create custom level directory {}: {error}",
            directory.display()
        )
    })?;
    let path = directory.join(&filename);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "Cannot create custom level directory {}: {error}",
                parent.display()
            )
        })?;
    }
    std::fs::write(&path, text).map_err(|error| format!("Cannot save {}: {error}", path.display()))
}

/// Write the Classic stats report to the source-compatible `stats.txt` path.
/// Native shutdown uses this directly; the Web build exports only by gesture.
#[cfg(not(target_arch = "wasm32"))]
pub fn write_classic_stats_file(text: &str) -> Result<(), String> {
    std::fs::write("stats.txt", text).map_err(|error| format!("Cannot write stats.txt: {error}"))
}

/// Explicit report export shared by the Extended Stats screen on both hosts.
pub fn export_classic_stats_report(text: &str) -> Result<(), String> {
    #[cfg(target_arch = "wasm32")]
    {
        download("stats.txt", text)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        write_classic_stats_file(text)
    }
}

/// Classic opens level paths with `string_lower(levelName) + ".lvl"`. Keep the
/// filename stem common to web/native and portable across their filesystems.
pub(crate) fn canonical_level_key(value: &str) -> String {
    let name = value.rsplit(['/', '\\']).next().unwrap_or(value);
    let stem = if name.to_ascii_lowercase().ends_with(".lvl") {
        &name[..name.len() - 4]
    } else {
        name
    };
    stem.to_ascii_lowercase()
}

/// Canonicalize an authored relative link while preserving its directory.
/// Classic appends the target to `levels/`, so `sub/room` and `room` are
/// different destinations even though selector/import names are basenames.
pub(crate) fn canonical_level_path_key(value: &str) -> String {
    canonical_relative_level_path(value, true)
}

/// Validate an editor/import path as a portable path below the levels root.
/// A filename without an extension receives `.lvl`, matching the editor.
pub(crate) fn relative_level_filename(value: &str) -> Result<String, String> {
    let path = safe_relative_path(value)?;
    let filename = path.rsplit('/').next().unwrap_or("");
    let stem = if filename.to_ascii_lowercase().ends_with(".lvl") {
        &filename[..filename.len() - 4]
    } else {
        filename
    };
    if stem.is_empty() {
        return Err("Enter a valid level filename".into());
    }
    if filename.to_ascii_lowercase().ends_with(".lvl") {
        Ok(path)
    } else {
        Ok(format!("{path}.lvl"))
    }
}

/// Convert a directory-picker path to a safe catalog key. The picker returns
/// file paths relative to the selected levels directory, with `.lvl` included.
pub(crate) fn imported_level_key(value: &str) -> Result<String, String> {
    let path = safe_relative_path(value)?;
    if !path.to_ascii_lowercase().ends_with(".lvl") {
        return Err("Only .lvl files can be imported".into());
    }
    let key = canonical_level_path_key(&path);
    if key.is_empty() {
        return Err("The level path is empty".into());
    }
    Ok(key)
}

/// Remove the selected-directory prefix from `File.webkitRelativePath`.
/// Browsers include that directory's name as the first path component.
pub(crate) fn strip_selected_directory(value: &str) -> Result<&str, String> {
    value
        .split_once('/')
        .map(|(_, relative)| relative)
        .filter(|relative| !relative.is_empty())
        .ok_or_else(|| "Browser folder paths do not include the selected directory".into())
}

fn safe_relative_path(value: &str) -> Result<String, String> {
    let normalized = value.replace('\\', "/");
    if normalized.starts_with('/') || normalized.starts_with("//") {
        return Err("Level paths must be relative to the levels folder".into());
    }
    if normalized.len() > 1024 || normalized.chars().any(char::is_control) {
        return Err("Use a valid relative level path".into());
    }

    let mut components = Vec::new();
    for component in normalized.split('/') {
        match component {
            "" | "." => {}
            ".." => {
                if components.pop().is_none() {
                    return Err("Level paths cannot escape the levels folder".into());
                }
            }
            name => {
                let portable = name.chars().all(|c| {
                    c.is_ascii_alphanumeric()
                        || matches!(
                            c,
                            ' ' | '!' | '@' | '$' | '%' | '&' | '(' | ')' | '.' | '-' | '_'
                        )
                });
                if name.len() > 100 || !portable || name == "." || name == ".." {
                    return Err("Use portable level path components".into());
                }
                components.push(name);
            }
        }
    }
    if components.is_empty() {
        return Err("Enter a valid level path".into());
    }
    Ok(components.join("/"))
}

/// Normalize an authored link exactly as `string_lower(levelName)` does before
/// Classic appends `.lvl`. Unlike a filename key, this keeps any `.lvl` in the
/// target because `room.lvl` opens `room.lvl.lvl`.
pub(crate) fn canonical_level_target_key(value: &str) -> String {
    canonical_relative_level_path(value, false)
}

fn canonical_relative_level_path(value: &str, strip_extension: bool) -> String {
    let normalized = value.replace('\\', "/");
    let path = if strip_extension && normalized.to_ascii_lowercase().ends_with(".lvl") {
        &normalized[..normalized.len() - 4]
    } else {
        &normalized
    };
    let mut components = Vec::new();
    for component in path.split('/') {
        match component {
            "" | "." => {}
            ".." => {
                if components.last().is_some_and(|part| part != "..") {
                    components.pop();
                } else {
                    components.push("..".to_owned());
                }
            }
            name => components.push(name.to_ascii_lowercase()),
        }
    }
    components.join("/")
}

/// Canonicalize save data written by v0/v1, whose path-like names historically
/// represented aliases and were flattened to the selected filename.
pub(crate) fn canonical_level_sources_v1(
    levels: std::collections::BTreeMap<String, String>,
) -> std::collections::BTreeMap<String, String> {
    let mut canonical = std::collections::BTreeMap::new();
    for (name, contents) in levels {
        // Save-data keys are logical selector names, not filenames. Preserve a
        // trailing `.lvl` because it can name the distinct file `room.lvl.lvl`;
        // older imported path aliases still collapse to their basename.
        let key = if name.contains(['/', '\\']) {
            canonical_level_key(&name)
        } else {
            name.to_ascii_lowercase()
        };
        if key.is_empty() || key == ".lvl" {
            continue;
        }
        match canonical.entry(key.clone()) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(contents);
            }
            std::collections::btree_map::Entry::Occupied(mut entry) if name == key => {
                // Prefer a prior exact lowercase key over a case/path alias.
                entry.insert(contents);
            }
            std::collections::btree_map::Entry::Occupied(_) => {}
        }
    }
    canonical
}

/// Canonicalize v2 catalog keys while retaining safe nested relative paths.
pub(crate) fn canonical_level_sources_v2(
    levels: std::collections::BTreeMap<String, String>,
) -> std::collections::BTreeMap<String, String> {
    let mut canonical = std::collections::BTreeMap::new();
    for (name, contents) in levels {
        let key = if name.contains(['/', '\\']) {
            let Ok(path) = safe_relative_path(&name) else {
                continue;
            };
            canonical_relative_level_path(&path, false)
        } else {
            name.to_ascii_lowercase()
        };
        if key.is_empty() {
            continue;
        }
        match canonical.entry(key.clone()) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(contents);
            }
            std::collections::btree_map::Entry::Occupied(mut entry) if name == key => {
                entry.insert(contents);
            }
            std::collections::btree_map::Entry::Occupied(_) => {}
        }
    }
    canonical
}

/// Read the Classic native `levels/**/*.lvl` catalog. Keep `!`-prefixed files:
/// Classic hides those from the selector but `scrLoadLevel` can still open
/// them as authored linked destinations. Web levels continue to come from
/// explicit user imports stored in the save data.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Default)]
pub(crate) struct NativeLevelCatalog {
    pub levels: std::collections::BTreeMap<String, String>,
    /// Paths that Classic can enumerate, but the host could not read. They are
    /// retained for the selector so failure occurs when the user selects them.
    pub unreadable: std::collections::BTreeSet<String>,
    pub warnings: Vec<String>,
}

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn load_native_level_directory(
    directory: &std::path::Path,
) -> Result<NativeLevelCatalog, String> {
    load_native_level_directory_with(directory, &|path| std::fs::read(path))
}

#[cfg(not(target_arch = "wasm32"))]
fn load_native_level_directory_with(
    directory: &std::path::Path,
    read_file: &impl Fn(&std::path::Path) -> std::io::Result<Vec<u8>>,
) -> Result<NativeLevelCatalog, String> {
    match std::fs::read_dir(directory) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(NativeLevelCatalog::default());
        }
        Err(error) => {
            return Err(format!(
                "Cannot list custom levels in {}: {error}",
                directory.display()
            ));
        }
    }

    let mut files = std::collections::BTreeMap::new();
    let mut catalog = NativeLevelCatalog::default();
    read_native_level_files(
        directory,
        directory,
        &mut files,
        &mut catalog.warnings,
        read_file,
    )?;
    let mut normalized = std::collections::BTreeMap::new();
    for (name, contents) in files {
        let key = canonical_level_path_key(&name);
        if key.is_empty() {
            continue;
        }
        match normalized.entry(key.clone()) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(contents);
            }
            std::collections::btree_map::Entry::Occupied(mut entry)
                if name == format!("{key}.lvl") =>
            {
                let _ = entry.insert(contents);
            }
            std::collections::btree_map::Entry::Occupied(_) => {}
        }
    }
    for (key, contents) in normalized {
        match contents {
            Ok(contents) => {
                catalog.levels.insert(key, contents);
            }
            Err(()) => {
                catalog.unreadable.insert(key);
            }
        }
    }
    Ok(catalog)
}

#[cfg(not(target_arch = "wasm32"))]
fn read_native_level_files<F>(
    root: &std::path::Path,
    directory: &std::path::Path,
    files: &mut std::collections::BTreeMap<String, Result<String, ()>>,
    warnings: &mut Vec<String>,
    read_file: &F,
) -> Result<(), String>
where
    F: Fn(&std::path::Path) -> std::io::Result<Vec<u8>>,
{
    let entries = std::fs::read_dir(directory).map_err(|error| {
        format!(
            "Cannot list custom levels in {}: {error}",
            directory.display()
        )
    })?;
    let mut entries = entries
        .map(|entry| {
            entry.map_err(|error| {
                format!(
                    "Cannot list custom levels in {}: {error}",
                    directory.display()
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|error| format!("Cannot inspect {}: {error}", path.display()))?;
        if file_type.is_dir() {
            read_native_level_files(root, &path, files, warnings, read_file)?;
            continue;
        }
        if !file_type.is_file() {
            continue;
        }
        if !path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("lvl"))
        {
            continue;
        }
        let Some(name) = path
            .strip_prefix(root)
            .ok()
            .and_then(std::path::Path::to_str)
        else {
            continue;
        };
        let contents = match read_file(&path) {
            Ok(contents) => Ok(String::from_utf8_lossy(&contents).into_owned()),
            Err(error) => {
                // Classic enumerates level names without opening each file.
                // A single unreadable level must not hide the rest of the
                // catalog. Keep its path separately so it can be selected and
                // report the failed open at the same point Classic does.
                warnings.push(format!("Cannot read {}: {error}", path.display()));
                Err(())
            }
        };
        // Retain the final extension until canonicalization. `file_stem()` on
        // `foo.lvl.lvl` would already be `foo.lvl`, and stripping another
        // extension there would incorrectly merge it with `foo.lvl`. Relative
        // directories are also part of a linked target's identity.
        files.insert(name.to_owned(), contents);
    }
    Ok(())
}

fn export_name(name: &str) -> Result<String, String> {
    let basename = name.rsplit(['/', '\\']).next().unwrap_or("");
    let stem = if basename.to_ascii_lowercase().ends_with(".lvl") {
        &basename[..basename.len() - 4]
    } else {
        basename
    };
    let portable = stem.chars().all(|c| {
        c.is_ascii_alphanumeric()
            || matches!(
                c,
                ' ' | '!' | '@' | '$' | '%' | '&' | '(' | ')' | '.' | '-' | '_'
            )
    });
    if stem.is_empty() || stem == "." || stem == ".." || stem.len() > 100 || !portable {
        return Err(
            "Use a portable level name with letters, numbers, spaces, ! @ $ % & ( ) . - or _"
                .into(),
        );
    }
    Ok(format!("{}.lvl", stem.to_ascii_lowercase()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn export_uses_classic_case_folding_and_one_portable_name_policy() {
        assert_eq!(export_name("../../Custom.LVL").unwrap(), "custom.lvl");
        assert_eq!(export_name("MY LEVEL").unwrap(), "my level.lvl");
        assert_eq!(export_name("N!GHT $%&().lvl").unwrap(), "n!ght $%&().lvl");
        assert_eq!(export_name("map_2.LVL").unwrap(), "map_2.lvl");
        for invalid in ["..", ".lvl", "bad:name", "hello🦇", "room?2", ""] {
            assert!(export_name(invalid).is_err());
        }
    }

    #[test]
    fn classic_level_keys_ignore_path_extension_and_case() {
        assert_eq!(canonical_level_key("/tmp/Example.LVL"), "example");
        assert_eq!(canonical_level_key("C:\\levels\\EXAMPLE.lvl"), "example");
        assert_eq!(canonical_level_key("map.lvl.lvl"), "map.lvl");
        assert_eq!(canonical_level_path_key("Sub\\Room.LVL"), "sub/room");
        assert_eq!(canonical_level_path_key("sub/../room.lvl"), "room");
        assert_eq!(canonical_level_path_key("map.lvl.lvl"), "map.lvl");
        assert_eq!(canonical_level_target_key("Sub\\Room.LVL"), "sub/room.lvl");
        assert_eq!(canonical_level_target_key("sub/../room"), "room");
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn native_level_directory_keeps_linkable_and_double_extension_files() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "spelunky-native-level-catalog-{}-{nonce}",
            std::process::id()
        ));
        let directory = root.join("levels");
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(directory.join("RoomA.LVL"), "first").unwrap();
        std::fs::write(directory.join("second.lvl"), "second").unwrap();
        std::fs::write(directory.join("!hidden.lvl"), "hidden").unwrap();
        std::fs::write(directory.join("foo.lvl"), "ordinary foo").unwrap();
        std::fs::write(directory.join("foo.lvl.lvl"), "double extension foo").unwrap();
        std::fs::write(directory.join("notes.txt"), "not a level").unwrap();
        std::fs::create_dir(directory.join("nested.lvl")).unwrap();
        std::fs::create_dir(directory.join("sub")).unwrap();
        std::fs::write(directory.join("sub/Room.LVL"), "nested room").unwrap();

        assert_eq!(
            load_native_level_directory(&directory).unwrap().levels,
            std::collections::BTreeMap::from([
                ("!hidden".into(), "hidden".into()),
                ("foo".into(), "ordinary foo".into()),
                ("foo.lvl".into(), "double extension foo".into()),
                ("rooma".into(), "first".into()),
                ("second".into(), "second".into()),
                ("sub/room".into(), "nested room".into()),
            ])
        );
        assert!(
            load_native_level_directory(&root.join("missing"))
                .unwrap()
                .levels
                .is_empty()
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn native_catalog_retains_legacy_text_and_unreadable_level_names() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "spelunky-native-level-unreadable-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("valid.lvl"), "readable level").unwrap();
        std::fs::write(root.join("legacy.lvl"), [0xff, 0xfe]).unwrap();
        std::fs::write(root.join("unreadable.lvl"), "unreadable").unwrap();

        let catalog = load_native_level_directory_with(&root, &|path| {
            if path
                .file_name()
                .is_some_and(|name| name == "unreadable.lvl")
            {
                Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied))
            } else {
                std::fs::read(path)
            }
        })
        .unwrap();

        assert_eq!(
            catalog.levels,
            std::collections::BTreeMap::from([
                ("legacy".into(), "��".into()),
                ("valid".into(), "readable level".into()),
            ])
        );
        assert_eq!(
            catalog.unreadable,
            std::collections::BTreeSet::from(["unreadable".into()])
        );
        assert_eq!(catalog.warnings.len(), 1);
        assert!(catalog.warnings[0].contains("unreadable.lvl"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn native_editor_save_replaces_catalog_file_and_keeps_double_extension_key() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "spelunky-native-level-save-{}-{nonce}",
            std::process::id()
        ));
        let directory = root.join("levels");
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(directory.join("room.lvl"), "old room").unwrap();

        save_native_level_to(&directory, "room.lvl", "edited room").unwrap();
        save_native_level_to(&directory, "bonus.lvl.lvl", "double extension").unwrap();
        save_native_level_to(&directory, "sub/room.lvl", "nested room").unwrap();

        assert_eq!(
            load_native_level_directory(&directory).unwrap().levels,
            std::collections::BTreeMap::from([
                ("bonus.lvl".into(), "double extension".into()),
                ("room".into(), "edited room".into()),
                ("sub/room".into(), "nested room".into()),
            ])
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn prior_custom_level_aliases_collapse_to_the_classic_filename_key() {
        let levels = std::collections::BTreeMap::from([
            ("Rooms/Example.LVL".into(), "path variant".into()),
            ("EXAMPLE".into(), "case variant".into()),
            ("example".into(), "canonical file".into()),
            ("!Hidden".into(), "hidden".into()),
            ("double.lvl".into(), "double extension".into()),
            (".lvl".into(), "empty name".into()),
        ]);
        let canonical = canonical_level_sources_v1(levels);
        assert_eq!(canonical.len(), 3);
        assert_eq!(canonical["example"], "canonical file");
        assert_eq!(canonical["!hidden"], "hidden");
        assert_eq!(canonical["double.lvl"], "double extension");
    }

    #[test]
    fn directory_import_preserves_relative_level_paths_and_rejects_escapes() {
        assert_eq!(
            strip_selected_directory("levels/start.lvl").unwrap(),
            "start.lvl"
        );
        assert_eq!(
            strip_selected_directory("levels/sub/next.lvl").unwrap(),
            "sub/next.lvl"
        );
        assert!(strip_selected_directory("start.lvl").is_err());
        assert_eq!(imported_level_key("Start.LVL").unwrap(), "start");
        assert_eq!(imported_level_key(".lvl.lvl").unwrap(), ".lvl");
        assert!(imported_level_key(".lvl").is_err());
        assert_eq!(imported_level_key("Sub\\Next.LVL").unwrap(), "sub/next");
        assert_eq!(imported_level_key("sub/../Next.LVL").unwrap(), "next");
        for unsafe_path in [
            "../outside.lvl",
            "sub/../../outside.lvl",
            "/outside.lvl",
            "C:\\outside.lvl",
        ] {
            assert!(imported_level_key(unsafe_path).is_err(), "{unsafe_path}");
        }
        assert!(imported_level_key("sub/notes.txt").is_err());
    }

    #[test]
    fn v2_level_source_normalization_preserves_safe_nested_paths() {
        let levels = std::collections::BTreeMap::from([
            ("Sub\\Room".into(), "nested".into()),
            ("ROOM".into(), "root".into()),
            (".lvl".into(), "double extension root".into()),
            ("../escape".into(), "unsafe".into()),
        ]);
        assert_eq!(
            canonical_level_sources_v2(levels),
            std::collections::BTreeMap::from([
                (".lvl".into(), "double extension root".into()),
                ("room".into(), "root".into()),
                ("sub/room".into(), "nested".into()),
            ])
        );
    }

    #[test]
    fn editor_level_filename_keeps_safe_nested_paths() {
        assert_eq!(
            relative_level_filename("Sub/Room"),
            Ok("Sub/Room.lvl".into())
        );
        assert_eq!(
            relative_level_filename(r"sub\room.lvl"),
            Ok("sub/room.lvl".into())
        );
        assert!(relative_level_filename("../room").is_err());
        assert!(relative_level_filename("C:\\room").is_err());
        assert!(relative_level_filename("/tmp/room").is_err());
    }
}

#[cfg(target_arch = "wasm32")]
pub fn choose_level(
    callback: impl FnOnce(Result<(String, String), String>) + 'static,
) -> Result<(), String> {
    choose_text_file(".lvl,text/plain", "Level", 4 * 1024 * 1024, callback)
}

#[cfg(target_arch = "wasm32")]
pub fn choose_level_directory(
    callback: impl FnOnce(Result<Vec<(String, String)>, String>) + 'static,
) -> Result<(), String> {
    use wasm_bindgen::{JsCast, closure::Closure};
    const MAX_TOTAL_BYTES: usize = 4 * 1024 * 1024;
    let document = web_sys::window()
        .and_then(|window| window.document())
        .ok_or("Browser document is unavailable")?;
    let input: web_sys::HtmlInputElement = document
        .create_element("input")
        .map_err(|error| format!("{error:?}"))?
        .dyn_into()
        .map_err(|_| "Cannot create directory picker")?;
    input.set_type("file");
    input.set_accept(".lvl,text/plain");
    input.set_multiple(true);
    input.set_webkitdirectory(true);
    let picker = input.clone();
    let handler = Closure::once_into_js(move |_event: web_sys::Event| {
        if let Some(handler) = picker.onchange() {
            let _ = picker.remove_event_listener_with_callback("cancel", &handler);
        }
        picker.set_onchange(None);
        let Some(file_list) = picker.files() else {
            callback(Err("No folder selected".into()));
            return;
        };
        if file_list.length() == 0 {
            callback(Err("No folder selected".into()));
            return;
        }
        wasm_bindgen_futures::spawn_local(async move {
            let mut total_bytes = 0usize;
            let mut imported = Vec::new();
            for index in 0..file_list.length() {
                let Some(file) = file_list.get(index) else {
                    continue;
                };
                let relative_path = js_sys::Reflect::get(
                    file.as_ref(),
                    &wasm_bindgen::JsValue::from_str("webkitRelativePath"),
                )
                .ok()
                .and_then(|value| value.as_string())
                .filter(|path| !path.is_empty())
                .ok_or_else(|| "Browser did not provide a folder-relative file path".to_owned());
                let relative_path = match relative_path {
                    Ok(path) => path,
                    Err(error) => {
                        callback(Err(error));
                        return;
                    }
                };
                // webkitRelativePath includes the selected directory itself
                // as the first component (for example `levels/sub/next.lvl`).
                // Classic links are relative to that directory, so retain only
                // the path beneath it (`sub/next.lvl`).
                let relative_path = match strip_selected_directory(&relative_path) {
                    Ok(path) => path.to_owned(),
                    Err(error) => {
                        callback(Err(error));
                        return;
                    }
                };
                if !relative_path.to_ascii_lowercase().ends_with(".lvl") {
                    continue;
                }
                let size = file.size() as usize;
                total_bytes = total_bytes.saturating_add(size);
                if total_bytes > MAX_TOTAL_BYTES {
                    callback(Err("Folder levels exceed the 4096 KiB import limit".into()));
                    return;
                }
                let text = match wasm_bindgen_futures::JsFuture::from(file.text()).await {
                    Ok(value) => match value.as_string() {
                        Some(text) => text,
                        None => {
                            callback(Err(format!("{} is not text", file.name())));
                            return;
                        }
                    },
                    Err(error) => {
                        callback(Err(format!("Cannot read {}: {error:?}", file.name())));
                        return;
                    }
                };
                imported.push((relative_path, text));
            }
            if imported.is_empty() {
                callback(Err("No .lvl files found in the selected folder".into()));
            } else {
                callback(Ok(imported));
            }
        });
    });
    input.set_onchange(Some(handler.unchecked_ref()));
    input
        .add_event_listener_with_callback("cancel", handler.unchecked_ref())
        .map_err(|error| format!("Cannot listen for folder selection: {error:?}"))?;
    input.click();
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub fn choose_stats(
    callback: impl FnOnce(Result<(String, String), String>) + 'static,
) -> Result<(), String> {
    choose_text_file(".txt,text/plain", "Stats report", 1024 * 1024, callback)
}

#[cfg(target_arch = "wasm32")]
fn choose_text_file(
    accept: &'static str,
    description: &'static str,
    max_bytes: usize,
    callback: impl FnOnce(Result<(String, String), String>) + 'static,
) -> Result<(), String> {
    use wasm_bindgen::{JsCast, closure::Closure};
    let document = web_sys::window()
        .and_then(|window| window.document())
        .ok_or("Browser document is unavailable")?;
    let input: web_sys::HtmlInputElement = document
        .create_element("input")
        .map_err(|error| format!("{error:?}"))?
        .dyn_into()
        .map_err(|_| "Cannot create file picker")?;
    input.set_type("file");
    input.set_accept(accept);
    let description = description.to_owned();
    let picker = input.clone();
    let handler = Closure::once_into_js(move |_event: web_sys::Event| {
        // Break the DOM callback reference as soon as selection completes.
        if let Some(handler) = picker.onchange() {
            let _ = picker.remove_event_listener_with_callback("cancel", &handler);
        }
        picker.set_onchange(None);
        let file = picker.files().and_then(|files| files.get(0));
        let Some(file) = file else {
            callback(Err("No file selected".into()));
            return;
        };
        if file.size() > max_bytes as f64 {
            callback(Err(format!(
                "{description} exceeds the {} KiB import limit",
                max_bytes / 1024
            )));
            return;
        }
        wasm_bindgen_futures::spawn_local(async move {
            let result = wasm_bindgen_futures::JsFuture::from(file.text())
                .await
                .map_err(|error| format!("Cannot read {description}: {error:?}"))
                .and_then(|text| {
                    text.as_string()
                        .ok_or_else(|| format!("{description} is not text"))
                })
                .map(|text| (file.name(), text));
            callback(result);
        });
    });
    input.set_onchange(Some(handler.unchecked_ref()));
    input
        .add_event_listener_with_callback("cancel", handler.unchecked_ref())
        .map_err(|error| format!("Cannot listen for file selection: {error:?}"))?;
    input.click();
    Ok(())
}

#[cfg(target_arch = "wasm32")]
pub fn download(name: &str, text: &str) -> Result<(), String> {
    use wasm_bindgen::JsCast;
    let document = web_sys::window()
        .and_then(|window| window.document())
        .ok_or("Browser document is unavailable")?;
    let parts = js_sys::Array::new();
    parts.push(&wasm_bindgen::JsValue::from_str(text));
    let options = web_sys::BlobPropertyBag::new();
    options.set_type("text/plain;charset=utf-8");
    let blob = web_sys::Blob::new_with_str_sequence_and_options(&parts, &options)
        .map_err(|error| format!("Cannot create download: {error:?}"))?;
    let url = web_sys::Url::create_object_url_with_blob(&blob)
        .map_err(|error| format!("Cannot create download URL: {error:?}"))?;
    let result = (|| {
        let anchor: web_sys::HtmlAnchorElement = document
            .create_element("a")
            .map_err(|error| format!("{error:?}"))?
            .dyn_into()
            .map_err(|_| "Cannot create download link")?;
        anchor.set_href(&url);
        anchor.set_download(name);
        // Attaching the link also supports Firefox's download handling.
        let body = document.body().ok_or("Browser body is unavailable")?;
        body.append_child(&anchor)
            .map_err(|error| format!("{error:?}"))?;
        anchor.click();
        anchor.remove();
        Ok(())
    })();
    // Defer revocation until the click's download task has consumed the URL.
    let revoke = wasm_bindgen::closure::Closure::once_into_js(move || {
        let _ = web_sys::Url::revoke_object_url(&url);
    });
    if let Some(window) = web_sys::window() {
        let _ = window
            .set_timeout_with_callback_and_timeout_and_arguments_0(revoke.unchecked_ref(), 1000);
    }
    result
}
