//! Per-event modifiers for browser import shortcuts (I/O only).
//! Winit's Web KeyboardInput precedes its ModifiersChanged for the same DOM
//! event. Keep the event's own Control value, even when dispatch is queued
//! until after a short chord has been released. These are queued event records,
//! not held-key state: ordinary focus/timing resets do not discard them before
//! Winit delivers their corresponding key. A replacement observer starts empty.
use std::collections::VecDeque;
use winit::keyboard::KeyCode;

#[derive(Default)]
struct ShortcutSnapshots {
    pending: VecDeque<(KeyCode, bool)>,
}
impl ShortcutSnapshots {
    fn record(&mut self, code: &str, control: bool, repeat: bool) {
        if repeat {
            return;
        }
        let key = match code {
            "KeyI" => KeyCode::KeyI,
            "KeyO" => KeyCode::KeyO,
            _ => return,
        };
        self.pending.push_back((key, control));
    }
    fn take(&mut self, key: KeyCode) -> Option<bool> {
        if !matches!(key, KeyCode::KeyI | KeyCode::KeyO) {
            return None;
        }
        match self.pending.pop_front() {
            Some((recorded, control)) if recorded == key => Some(control),
            Some(_) => {
                // A missing upstream event must not block every later import
                // behind a stale front record. Do not guess its modifiers.
                self.pending.clear();
                None
            }
            None => None,
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub(crate) struct BrowserShortcutModifiers {
    canvas: web_sys::HtmlCanvasElement,
    snapshots: std::rc::Rc<std::cell::RefCell<ShortcutSnapshots>>,
    keydown: wasm_bindgen::closure::Closure<dyn FnMut(web_sys::KeyboardEvent)>,
}
#[cfg(target_arch = "wasm32")]
impl BrowserShortcutModifiers {
    pub(crate) fn new(canvas: web_sys::HtmlCanvasElement) -> Result<Self, String> {
        use wasm_bindgen::JsCast;
        let snapshots = std::rc::Rc::new(std::cell::RefCell::new(ShortcutSnapshots::default()));
        let captured = snapshots.clone();
        let keydown =
            wasm_bindgen::closure::Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
                captured
                    .borrow_mut()
                    .record(&event.code(), event.ctrl_key(), event.repeat());
            })
                as Box<dyn FnMut(web_sys::KeyboardEvent)>);
        // Capture runs before Winit's canvas bubble listener, regardless of
        // listener registration order. It neither changes nor cancels input.
        canvas
            .add_event_listener_with_callback_and_bool(
                "keydown",
                keydown.as_ref().unchecked_ref(),
                true,
            )
            .map_err(|error| format!("Cannot observe editor shortcut modifiers: {error:?}"))?;
        Ok(Self {
            canvas,
            snapshots,
            keydown,
        })
    }
    pub(crate) fn take(&self, key: KeyCode) -> Option<bool> {
        self.snapshots.borrow_mut().take(key)
    }
}
#[cfg(target_arch = "wasm32")]
impl Drop for BrowserShortcutModifiers {
    fn drop(&mut self) {
        use wasm_bindgen::JsCast;
        let _ = self.canvas.remove_event_listener_with_callback_and_bool(
            "keydown",
            self.keydown.as_ref().unchecked_ref(),
            true,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor_ui::{EditorCommand, EditorUi};

    #[test]
    fn outside_held_control_first_import_uses_key_event_not_stale_modifiers() {
        let mut editor = EditorUi::new();
        let stale_winit_control = false;
        assert!(matches!(
            editor.key(KeyCode::KeyI, None, stale_winit_control),
            EditorCommand::None
        ));
        let mut events = ShortcutSnapshots::default();
        // The canvas never received Control-down: its first DOM event is I,
        // with ctrlKey=true while Control was already held outside its focus.
        events.record("KeyI", true, false);
        let control = events.take(KeyCode::KeyI).unwrap_or(stale_winit_control);
        assert!(matches!(
            editor.key(KeyCode::KeyI, None, control),
            EditorCommand::Import
        ));
        assert_eq!(events.take(KeyCode::KeyI), None);
    }

    #[test]
    fn queued_short_chords_keep_independent_event_modifiers_after_release() {
        let mut events = ShortcutSnapshots::default();
        events.record("KeyI", true, false);
        events.record("KeyO", false, false);
        let current_control_after_release = false;
        let mut editor = EditorUi::new();
        assert!(matches!(
            editor.key(
                KeyCode::KeyI,
                None,
                events
                    .take(KeyCode::KeyI)
                    .unwrap_or(current_control_after_release)
            ),
            EditorCommand::Import
        ));
        assert!(matches!(
            editor.key(
                KeyCode::KeyO,
                None,
                events
                    .take(KeyCode::KeyO)
                    .unwrap_or(current_control_after_release)
            ),
            EditorCommand::None
        ));
        assert_eq!(events.take(KeyCode::KeyI), None);
    }

    #[test]
    fn ignored_repeats_other_keys_and_mismatch_do_not_replay_an_import() {
        let mut events = ShortcutSnapshots::default();
        events.record("KeyI", true, true);
        events.record("ArrowUp", true, false);
        assert_eq!(events.take(KeyCode::KeyI), None);
        events.record("KeyO", true, false);
        assert_eq!(events.take(KeyCode::ArrowUp), None);
        assert_eq!(events.take(KeyCode::KeyI), None);
        events.record("KeyI", false, false);
        assert_eq!(events.take(KeyCode::KeyI), Some(false));
        assert_eq!(events.take(KeyCode::KeyI), None);
    }

    #[test]
    fn queued_import_keeps_event_control_across_gameplay_and_clock_resets() {
        let mut events = ShortcutSnapshots::default();
        events.record("KeyI", true, false);
        let mut input = crate::input::InputState::default();
        let mut bindings = crate::storage::Settings::default().bindings;
        bindings.insert("attack".into(), "KeyI".into());
        input.set_bindings(&bindings);
        input.key(KeyCode::KeyI, true, false);
        let mut clock = crate::timing::FixedClock::new();
        clock.advance(0.005);
        // These are the real components called by timing reset/focus loss.
        // Already-captured DOM/Winit event history is not held gameplay input.
        input.clear_gameplay();
        clock.reset();
        input.clear();
        assert!(!input.take_tick().attack_pressed);
        assert_eq!(clock.advance(0.).ticks, 0);
        let mut editor = EditorUi::new();
        assert!(matches!(
            editor.key(KeyCode::KeyI, None, events.take(KeyCode::KeyI).unwrap()),
            EditorCommand::Import
        ));
        assert_eq!(events.take(KeyCode::KeyI), None);
        events.record("KeyI", false, false);
        assert!(matches!(
            editor.key(KeyCode::KeyI, None, events.take(KeyCode::KeyI).unwrap()),
            EditorCommand::None
        ));
    }

    #[test]
    fn fresh_capture_owner_does_not_reuse_previous_owner_import_records() {
        let mut old = ShortcutSnapshots::default();
        old.record("KeyI", true, false);
        let mut owner = Some(old);
        // Replacement drops the old capture history. Actual listener Drop and
        // WindowId rejection are source-reviewed, not exercised by this test.
        drop(owner.replace(ShortcutSnapshots::default()));
        let fresh = owner.as_mut().unwrap();
        assert_eq!(fresh.take(KeyCode::KeyI), None);
        fresh.record("KeyI", false, false);
        let mut editor = EditorUi::new();
        assert!(matches!(
            editor.key(KeyCode::KeyI, None, fresh.take(KeyCode::KeyI).unwrap()),
            EditorCommand::None
        ));
    }
}
