//! Browser level import/export, kept separate from platform-independent editing.

/// Explicit user-requested export; native exports never overwrite an existing file.
pub fn export(name: &str, text: &str) -> Result<(), String> {
    #[cfg(target_arch = "wasm32")]
    {
        download(name, text)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::io::Write;
        let path = export_name(name)?;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|error| {
                if error.kind() == std::io::ErrorKind::AlreadyExists {
                    format!("{path} already exists; use N to choose another name")
                } else {
                    format!("Cannot export {path}: {error}")
                }
            })?;
        file.write_all(text.as_bytes())
            .map_err(|error| format!("Cannot write {path}: {error}"))?;
        Ok(())
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn export_name(name: &str) -> Result<String, String> {
    let basename = name.rsplit(['/', '\\']).next().unwrap_or("");
    let stem = if basename.to_ascii_lowercase().ends_with(".lvl") {
        &basename[..basename.len() - 4]
    } else {
        basename
    };
    if stem.is_empty()
        || stem.len() > 100
        || !stem
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err("Use N to name the export with letters, numbers, - or _".into());
    }
    Ok(format!("{stem}.lvl"))
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    #[test]
    fn export_stays_in_working_directory() {
        assert_eq!(export_name("../../custom.lvl").unwrap(), "custom.lvl");
        assert_eq!(export_name("map_2.LVL").unwrap(), "map_2.lvl");
        for invalid in ["..", ".lvl", "bad:name", "hello world", ""] {
            assert!(export_name(invalid).is_err());
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub fn choose_level(
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
    input.set_accept(".lvl,text/plain");
    let picker = input.clone();
    let handler = Closure::once_into_js(move |_event: web_sys::Event| {
        // Break the DOM callback reference as soon as selection completes.
        if let Some(handler) = picker.onchange() {
            let _ = picker.remove_event_listener_with_callback("cancel", &handler);
        }
        picker.set_onchange(None);
        let file = picker.files().and_then(|files| files.get(0));
        let Some(file) = file else {
            callback(Err("No level selected".into()));
            return;
        };
        if file.size() > 4.0 * 1024.0 * 1024.0 {
            callback(Err("Level exceeds the 4 MiB import limit".into()));
            return;
        }
        wasm_bindgen_futures::spawn_local(async move {
            let result = wasm_bindgen_futures::JsFuture::from(file.text())
                .await
                .map_err(|error| format!("Cannot read level: {error:?}"))
                .and_then(|text| text.as_string().ok_or("Level is not text".into()))
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
