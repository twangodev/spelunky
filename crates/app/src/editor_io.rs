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

pub(crate) fn canonical_level_sources(
    levels: std::collections::BTreeMap<String, String>,
) -> std::collections::BTreeMap<String, String> {
    let mut canonical = std::collections::BTreeMap::new();
    for (name, contents) in levels {
        let key = canonical_level_key(&name);
        if key.is_empty() {
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
    }

    #[test]
    fn prior_custom_level_aliases_collapse_to_the_classic_filename_key() {
        let levels = std::collections::BTreeMap::from([
            ("Rooms/Example.LVL".into(), "path variant".into()),
            ("EXAMPLE".into(), "case variant".into()),
            ("example".into(), "canonical file".into()),
            ("!Hidden.LVL".into(), "hidden".into()),
            (".lvl".into(), "empty name".into()),
        ]);
        let canonical = canonical_level_sources(levels);
        assert_eq!(canonical.len(), 2);
        assert_eq!(canonical["example"], "canonical file");
        assert_eq!(canonical["!hidden"], "hidden");
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
