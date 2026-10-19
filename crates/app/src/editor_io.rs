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
    let filename = export_name(name)?;
    std::fs::create_dir_all(directory).map_err(|error| {
        format!(
            "Cannot create custom level directory {}: {error}",
            directory.display()
        )
    })?;
    let path = directory.join(&filename);
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

pub(crate) fn canonical_level_sources(
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

/// Read the Classic native `levels/**/*.lvl` catalog. Keep `!`-prefixed files:
/// Classic hides those from the selector but `scrLoadLevel` can still open
/// them as authored linked destinations. Web levels continue to come from
/// explicit user imports stored in the save data.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn load_native_level_directory(
    directory: &std::path::Path,
) -> Result<std::collections::BTreeMap<String, String>, String> {
    match std::fs::read_dir(directory) {
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(std::collections::BTreeMap::new());
        }
        Err(error) => {
            return Err(format!(
                "Cannot list custom levels in {}: {error}",
                directory.display()
            ));
        }
    }

    let mut files = std::collections::BTreeMap::new();
    read_native_level_files(directory, directory, &mut files)?;
    let mut levels = std::collections::BTreeMap::new();
    for (name, contents) in files {
        let key = canonical_level_path_key(&name);
        if key.is_empty() {
            continue;
        }
        match levels.entry(key.clone()) {
            std::collections::btree_map::Entry::Vacant(entry) => {
                entry.insert(contents);
            }
            std::collections::btree_map::Entry::Occupied(mut entry)
                if name == format!("{key}.lvl") =>
            {
                entry.insert(contents);
            }
            std::collections::btree_map::Entry::Occupied(_) => {}
        }
    }
    Ok(levels)
}

#[cfg(not(target_arch = "wasm32"))]
fn read_native_level_files(
    root: &std::path::Path,
    directory: &std::path::Path,
    files: &mut std::collections::BTreeMap<String, String>,
) -> Result<(), String> {
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
            read_native_level_files(root, &path, files)?;
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
        let contents = std::fs::read_to_string(&path)
            .map_err(|error| format!("Cannot read {}: {error}", path.display()))?;
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
            load_native_level_directory(&directory).unwrap(),
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
                .is_empty()
        );
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

        assert_eq!(
            load_native_level_directory(&directory).unwrap(),
            std::collections::BTreeMap::from([
                ("bonus.lvl".into(), "double extension".into()),
                ("room".into(), "edited room".into()),
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
        let canonical = canonical_level_sources(levels);
        assert_eq!(canonical.len(), 3);
        assert_eq!(canonical["example"], "canonical file");
        assert_eq!(canonical["!hidden"], "hidden");
        assert_eq!(canonical["double.lvl"], "double extension");
    }
}

#[cfg(target_arch = "wasm32")]
pub fn choose_level(
    callback: impl FnOnce(Result<(String, String), String>) + 'static,
) -> Result<(), String> {
    choose_text_file(".lvl,text/plain", "Level", 4 * 1024 * 1024, callback)
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
