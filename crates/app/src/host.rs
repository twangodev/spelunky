//! Window lifecycle, menus, platform services, and fixed-step game integration.
use crate::{
    atlas::Atlas,
    audio::Audio,
    donation_ui::{self, TransitionDonation},
    editor_ui::{EditorCommand, EditorUi},
    input::{InputState, UiButton},
    renderer::{DrawQuad, Renderer},
    scene::{self, rect, text},
    settings_ui::{SettingsCommand, SettingsUi},
    storage::{Record, Store},
    timing::FixedClock,
    viewport::Viewport,
};
use spelunky_core::{Area, Game, GameEvent, RunState, SourceMusicCommand, SourceMusicState};
use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use winit::window::Fullscreen;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

const WHITE: [f32; 4] = [1.0, 0.95, 0.85, 1.0];
const GOLD: [f32; 4] = [1.0, 0.78, 0.30, 1.0];
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Screen {
    Title,
    Playing,
    Pause,
    Settings,
    Editor,
    Custom,
    Shortcuts,
    Records,
    About,
    Finished,
    Donation,
}
enum UserEvent {
    Ready(Result<(Renderer, Atlas), String>),
    #[cfg(target_arch = "wasm32")]
    Imported(String, String),
    #[cfg(target_arch = "wasm32")]
    ImportError(String),
}

/// Presentation history becomes usable only after a tick in the current room.
/// A frame sampled before a menu action must not advance a newly resumed game.
struct HostTiming {
    clock: FixedClock,
    last_frame: f64,
    generation: u64,
    alpha: f64,
    alpha_sample: Option<f64>,
    history_ready: bool,
}
impl Default for HostTiming {
    fn default() -> Self {
        Self {
            clock: FixedClock::new(),
            last_frame: 0.,
            generation: 0,
            alpha: 0.,
            alpha_sample: None,
            history_ready: false,
        }
    }
}
#[derive(Clone, Copy)]
struct HostFrame {
    sampled_at: f64,
    elapsed: f64,
    generation: u64,
}
impl HostTiming {
    fn reset(&mut self, now: f64) {
        self.clock.reset();
        self.last_frame = now;
        self.generation = self.generation.wrapping_add(1);
        self.history_ready = false;
        self.alpha_sample = None;
    }
    fn begin_frame(&mut self, now: f64) -> HostFrame {
        let frame = HostFrame {
            sampled_at: now,
            elapsed: (now - self.last_frame).max(0.),
            generation: self.generation,
        };
        self.last_frame = now;
        frame
    }
    fn advance(&mut self, frame: HostFrame) -> crate::timing::FrameStep {
        let elapsed = if frame.generation == self.generation {
            frame.elapsed
        } else {
            0.
        };
        let step = self.clock.advance(elapsed);
        self.alpha = step.alpha;
        self.alpha_sample = (frame.generation == self.generation && frame.sampled_at.is_finite())
            .then_some(frame.sampled_at);
        step
    }
    fn alpha(&self) -> f64 {
        if self.history_ready { self.alpha } else { 1. }
    }
    /// Account for update work without consuming it from the fixed clock.
    /// The next advance still receives the complete elapsed frame interval.
    fn render_alpha(&self, now: f64, running: bool) -> f64 {
        let alpha = self.alpha();
        if !running || !self.history_ready {
            return alpha;
        }
        let Some(sampled_at) = self.alpha_sample else {
            return alpha;
        };
        if !now.is_finite() || now < sampled_at {
            return alpha;
        }
        (alpha + (now - sampled_at) * f64::from(spelunky_core::TICK_RATE)).clamp(0., 1.)
    }
}

fn render_clock_running(
    screen: Screen,
    focused: bool,
    occluded: bool,
    game: Option<&Game>,
) -> bool {
    focused
        && !occluded
        && (screen == Screen::Playing
            || (screen == Screen::Finished
                && game.is_some_and(|game| game.state == RunState::Dead)))
}

pub fn run() -> Result<(), String> {
    let event_loop = EventLoop::<UserEvent>::with_user_event()
        .build()
        .map_err(|e| e.to_string())?;
    let app = App::new(event_loop.create_proxy());
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut app = app;
        for path in std::env::args()
            .skip(1)
            .filter(|path| path.ends_with(".lvl"))
        {
            match std::fs::read_to_string(&path) {
                Ok(contents) => app.import_level(path, contents),
                Err(error) => app.message = format!("IMPORT FAILED: {error}"),
            }
        }
        event_loop.run_app(&mut app).map_err(|e| e.to_string())
    }
    #[cfg(target_arch = "wasm32")]
    {
        use winit::platform::web::EventLoopExtWebSys;
        event_loop.spawn_app(app);
        Ok(())
    }
}

struct App {
    proxy: EventLoopProxy<UserEvent>,
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    atlas: Option<Atlas>,
    input: InputState,
    audio: Audio,
    source_music: SourceMusicState,
    store: Store,
    timing: HostTiming,
    game: Option<Game>,
    screen: Screen,
    settings_return: Screen,
    selection: usize,
    settings_ui: SettingsUi,
    editor: EditorUi,
    editor_play: bool,
    donation: Option<TransitionDonation>,
    donation_confirm: bool,
    recorded: bool,
    cinematic_action: bool,
    pause_start_requested: bool,
    exit_requested: bool,
    focused: bool,
    occluded: bool,
    next_frame: f64,
    #[cfg(target_arch = "wasm32")]
    presentation_cadence: crate::timing::PresentationCadence,
    cursor: [f32; 2],
    cursor_physical: Option<[f64; 2]>,
    mouse_left: bool,
    mouse_right: bool,
    control: bool,
    message: String,
    message_until: f64,
    message_simulated: bool,
    source_message: Option<spelunky_core::SourceMessage>,
    #[cfg(not(target_arch = "wasm32"))]
    epoch: std::time::Instant,
}
impl App {
    fn new(proxy: EventLoopProxy<UserEvent>) -> Self {
        let store = Store::load();
        let mut input = InputState::default();
        input.set_bindings(&store.data.settings.bindings);
        input.set_controller_bindings(&store.data.settings.controller_bindings);
        let source_music = SourceMusicState::default();
        let mut game = Game::shell_with_music(
            wall_seed(),
            store.data.settings.rules,
            source_music.session_state(),
        );
        hydrate_run(&mut game, &store.data);
        spelunky_core::opening::begin_intro(&mut game);
        let editor = EditorUi::with_source_rng(
            spelunky_core::editor::preview::PreviewContext::default(),
            game.rng.clone(),
        );
        Self {
            proxy,
            window: None,
            renderer: None,
            atlas: None,
            input,
            audio: Audio::new(),
            source_music,
            store,
            timing: HostTiming::default(),
            game: Some(game),
            screen: Screen::Playing,
            settings_return: Screen::Title,
            selection: 0,
            settings_ui: SettingsUi::new(),
            editor,
            editor_play: false,
            donation: None,
            donation_confirm: false,
            recorded: false,
            cinematic_action: false,
            pause_start_requested: false,
            exit_requested: false,
            focused: true,
            occluded: false,
            next_frame: 0.,
            #[cfg(target_arch = "wasm32")]
            presentation_cadence: crate::timing::PresentationCadence::default(),
            cursor: [0.; 2],
            cursor_physical: None,
            mouse_left: false,
            mouse_right: false,
            control: false,
            message: String::new(),
            message_until: 0.,
            message_simulated: false,
            source_message: None,
            #[cfg(not(target_arch = "wasm32"))]
            epoch: std::time::Instant::now(),
        }
    }
    fn now(&self) -> f64 {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.epoch.elapsed().as_secs_f64()
        }
        #[cfg(target_arch = "wasm32")]
        {
            web_sys::window()
                .and_then(|w| w.performance())
                .map(|p| p.now() / 1000.)
                .unwrap_or_else(|| js_sys::Date::now() / 1000.)
        }
    }
    fn reset_timing(&mut self) {
        self.cinematic_action = false;
        self.pause_start_requested = false;
        self.donation_confirm = false;
        self.timing.reset(self.now());
        self.input.clear_gameplay();
        self.mouse_left = false;
        self.mouse_right = false;
        self.editor.cancel_pointer();
    }
    fn reset_room_timing(&mut self) {
        if let Some(game) = &mut self.game {
            resume_room_music(game);
        }
        self.donation_confirm = false;
        self.timing.reset(self.now());
        self.input.clear_edges();
    }
    fn screen(&mut self, screen: Screen) {
        if screen == Screen::Editor && self.screen != Screen::Editor {
            if let Some(game) = &self.game {
                self.editor.set_source_rng(game.rng.clone());
                self.editor.set_source_context(editor_preview_context(game));
            }
            if self.editor_play {
                self.editor.return_from_playtest();
            }
            if let Some(game) = &mut self.game {
                game.rng = self.editor.source_rng().clone();
            }
        }
        self.screen = screen;
        self.selection = if screen == Screen::Finished && self.is_hub() {
            1
        } else {
            0
        };
        self.reset_timing();
        self.audio
            .pause(!self.focused || self.occluded || matches!(screen, Screen::Pause));
    }
    fn toast(&mut self, message: impl Into<String>) {
        self.message = message.into();
        self.message_simulated = false;
        self.source_message = None;
        self.message_until = self.now() + 6.;
    }
    fn snapshot_lifetime_stats(&mut self) {
        if let Some(game) = &self.game {
            capture_lifetime_stats(game, &mut self.store.data);
        }
    }
    fn shutdown(&mut self, event_loop: &ActiveEventLoop) {
        self.exit_requested = true;
        self.snapshot_lifetime_stats();
        let _ = self.store.save();
        self.input.clear();
        self.audio.stop_music();
        self.audio.pause(true);
        event_loop.exit();
    }
    fn settings_changed(&mut self) {
        let settings = &self.store.data.settings;
        self.input.set_bindings(&settings.bindings);
        self.input
            .set_controller_bindings(&settings.controller_bindings);
        self.audio
            .set_volumes(settings.music_volume, settings.effects_volume);
        if let Some(renderer) = &mut self.renderer {
            renderer.set_vsync(settings.vsync);
        }
        if let Err(error) = self.store.save() {
            self.toast(error);
        }
    }
    fn fullscreen(&self) {
        #[cfg(target_arch = "wasm32")]
        toggle_browser_fullscreen();
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(window) = &self.window {
            window.set_fullscreen(if window.fullscreen().is_some() {
                None
            } else {
                Some(Fullscreen::Borderless(None))
            });
        }
    }
    fn flush_source_music(&mut self) {
        self.audio.set_volumes(
            self.store.data.settings.music_volume,
            self.store.data.settings.effects_volume,
        );
        if let Some(game) = &mut self.game {
            consume_source_music(&mut self.audio, &mut self.source_music, game);
        }
    }
    fn start_game(&mut self, tutorial: bool) {
        self.flush_source_music();
        self.snapshot_lifetime_stats();
        self.source_message = None;
        let seed = wall_seed() ^ fold_seed_entropy(u128::from(self.now().to_bits()));
        let mut game = if tutorial {
            Game::tutorial_with_music(
                seed,
                self.store.data.settings.rules,
                self.source_music.session_state(),
            )
        } else {
            Game::new_with_music(
                seed,
                self.store.data.settings.rules,
                self.source_music.session_state(),
            )
        };
        hydrate_run(&mut game, &self.store.data);
        if tutorial {
            game.player.bombs = 0;
            game.player.ropes = 2;
        }
        self.game = Some(game);
        self.editor_play = false;
        self.recorded = false;
        self.screen(Screen::Playing);
    }
    fn start_from_hub(&mut self, tutorial: bool, level: u32) {
        self.snapshot_lifetime_stats();
        self.source_message = None;
        if let Some(game) = &mut self.game {
            game.start_from_hub(tutorial, level);
            donation_ui::hydrate(game, &self.store.data.tunnel_remaining);
        } else {
            self.start_game(tutorial);
        }
        self.editor_play = false;
        self.recorded = false;
        self.screen(Screen::Playing);
    }
    fn open_hub(&mut self, kind: spelunky_core::hub::HubKind) {
        self.snapshot_lifetime_stats();
        self.source_message = None;
        if self.game.is_none() {
            self.start_game(false);
        }
        if let Some(game) = &mut self.game {
            resume_room_music(game);
            game.flags.extend(
                self.store
                    .data
                    .unlocks
                    .iter()
                    .filter(|flag| persistent_flag(flag))
                    .cloned(),
            );
            game.load_hub(kind);
            for (key, value) in &self.store.data.challenge_records {
                game.data.insert(format!("best_{key}"), f64::from(*value));
            }
            let records: Vec<_> = self
                .store
                .data
                .records
                .iter()
                .filter(|record| record.rules == game.rules)
                .collect();
            for (key, value) in [
                (
                    "best_money",
                    records.iter().map(|record| record.score).max().unwrap_or(0) as f64,
                ),
                (
                    "best_time",
                    records
                        .iter()
                        .filter(|record| record.won && record.fastest_eligible)
                        .map(|record| record.elapsed_seconds())
                        .min()
                        .unwrap_or(0) as f64,
                ),
                (
                    "best_kills",
                    records.iter().map(|record| record.kills).max().unwrap_or(0) as f64,
                ),
                (
                    "best_saves",
                    records
                        .iter()
                        .map(|record| record.rescues)
                        .max()
                        .unwrap_or(0) as f64,
                ),
                ("total_plays", records.len() as f64),
                (
                    "total_deaths",
                    records.iter().filter(|record| !record.won).count() as f64,
                ),
                (
                    "total_wins",
                    records.iter().filter(|record| record.won).count() as f64,
                ),
            ] {
                game.data.insert(key.into(), value);
            }
        }
        self.editor_play = false;
        self.recorded = false;
        self.screen(Screen::Playing);
    }
    fn is_hub(&self) -> bool {
        self.game
            .as_ref()
            .is_some_and(|game| game.flags.contains("hub"))
    }
    fn is_custom(&self) -> bool {
        self.game
            .as_ref()
            .is_some_and(|game| game.flags.contains("custom"))
    }
    fn return_from_challenge(&mut self) {
        let changed = if let Some(game) = &mut self.game {
            let changed = capture_challenge_records(game, &mut self.store.data.challenge_records);
            prepare_challenge_scores(game);
            changed
        } else {
            false
        };
        if changed {
            if let Err(error) = self.store.save() {
                self.toast(error);
            }
        }
        self.open_hub(spelunky_core::hub::HubKind::Scores);
    }
    fn key(&mut self, key: KeyCode, text_value: Option<&str>) {
        // Deliver earlier room producers before a new F1/selector stop.
        self.flush_source_music();
        if key == KeyCode::F4
            && !(self.screen == Screen::Settings && self.settings_ui.awaiting_binding())
        {
            self.fullscreen();
            return;
        }
        let control = source_host_control(self.screen, self.game.as_ref(), key);
        match control {
            Some(SourceHostControl::Suicide) => {
                if let Some(game) = &mut self.game {
                    apply_source_suicide(game);
                }
                self.screen(Screen::Playing);
                self.audio.stop_music();
                return;
            }
            Some(SourceHostControl::Quit) => {
                self.exit_requested = true;
                self.input.clear();
                self.audio.stop_music();
                self.audio.pause(true);
                return;
            }
            Some(SourceHostControl::Editor | SourceHostControl::CustomLevels) => {
                self.editor_play = false;
                self.source_message = None;
                if let Some(game) = &mut self.game {
                    game.flags.remove("editor_test");
                    game.flags.insert("source_music_stopped".into());
                    if control == Some(SourceHostControl::CustomLevels) {
                        game.flags.insert("source_load_level_menu".into());
                    } else {
                        game.flags.remove("source_load_level_menu");
                    }
                }
                if control == Some(SourceHostControl::Editor) {
                    self.editor = EditorUi::with_context(
                        self.game
                            .as_ref()
                            .map(editor_preview_context)
                            .unwrap_or_default(),
                    );
                    self.screen(Screen::Editor);
                } else {
                    self.screen(Screen::Custom);
                }
                self.audio.stop_music();
                return;
            }
            None => {}
        }
        match self.screen {
            Screen::Donation => {
                if matches!(key, KeyCode::Enter | KeyCode::Escape) {
                    self.continue_transition();
                }
            }
            Screen::Pause => {
                self.pause_start_requested |= key == KeyCode::Escape;
                // Source BeginStep resolves all queued independent edges together.
                // A rebound menu key must not also select a modern pause item.
                if self.pause_start_requested
                    || self.input.action_key("bomb", key)
                    || self.input.action_key("rope", key)
                {
                    return;
                }
                self.menu_key(key);
            }
            Screen::Playing => {
                if self
                    .game
                    .as_ref()
                    .and_then(|game| game.opening.as_ref())
                    .is_some_and(|opening| !opening.can_pause)
                {
                    self.cinematic_action |= source_start_key(key, true);
                    return;
                }
                if self
                    .game
                    .as_ref()
                    .is_some_and(|game| game.state == RunState::Ending)
                {
                    self.cinematic_action |= source_start_key(key, true);
                    return;
                }
                if self
                    .game
                    .as_ref()
                    .is_some_and(|game| game.state == RunState::Dead)
                {
                    self.cinematic_action |= source_start_key(key, false);
                    return;
                }
                if self.pause_key(key) {
                    self.screen(Screen::Pause)
                }
            }
            Screen::Settings => match self.settings_ui.key(key, &mut self.store.data.settings) {
                SettingsCommand::Changed => self.settings_changed(),
                SettingsCommand::Back => self.screen(self.settings_return),
                SettingsCommand::Fullscreen => self.fullscreen(),
                _ => {}
            },
            Screen::Editor => {
                let command = self.editor.key(key, text_value, self.control);
                self.editor_command(command);
            }
            _ => self.menu_key(key),
        }
    }
    fn pause_key(&self, key: KeyCode) -> bool {
        key == KeyCode::Escape || self.action_key("pause", key)
    }
    fn action_key(&self, action: &str, key: KeyCode) -> bool {
        self.store
            .data
            .settings
            .bindings
            .get(action)
            .is_some_and(|keys| {
                keys.split(',')
                    .any(|name| name.trim() == format!("{key:?}"))
            })
    }
    fn menu_key(&mut self, key: KeyCode) {
        let key = if self.screen == Screen::Finished && self.action_key("attack", key) {
            KeyCode::Enter
        } else {
            key
        };
        if key == KeyCode::Escape {
            if self.screen == Screen::Custom {
                self.return_from_custom_selector();
                return;
            }
            match self.screen {
                Screen::Pause => self.screen(Screen::Playing),
                Screen::Finished if self.editor_play => self.screen(Screen::Editor),
                Screen::Finished if self.is_hub() => self.return_from_challenge(),
                _ => self.screen(Screen::Title),
            };
            return;
        }
        let count = match self.screen {
            Screen::Title => 10,
            Screen::Shortcuts => self.shortcuts().len() + 1,
            Screen::Pause => {
                if self.editor_play {
                    4
                } else {
                    3
                }
            }
            Screen::Finished => 2,
            Screen::Custom => self.store.data.custom_levels.len() + 2,
            _ => 1,
        };
        if key == KeyCode::ArrowUp {
            self.selection = (self.selection + count - 1) % count;
            return;
        }
        if key == KeyCode::ArrowDown {
            self.selection = (self.selection + 1) % count;
            return;
        }
        if self.screen == Screen::Custom && key == KeyCode::KeyE {
            if let Some(name) = self
                .store
                .data
                .custom_levels
                .keys()
                .nth(self.selection)
                .cloned()
            {
                let contents = self.store.data.custom_levels[&name].clone();
                if let Some(game) = &self.game {
                    self.editor.set_source_rng(game.rng.clone());
                    self.editor.set_source_context(editor_preview_context(game));
                }
                let result = self.editor.load(&name, &contents);
                if let Some(game) = &mut self.game {
                    game.rng = self.editor.source_rng().clone();
                }
                match result {
                    Ok(()) => self.screen(Screen::Editor),
                    Err(error) => self.toast(error),
                }
            }
            return;
        }
        if !matches!(key, KeyCode::Enter | KeyCode::Space | KeyCode::KeyZ) {
            return;
        }
        match self.screen {
            Screen::Title => match self.selection {
                0 => self.start_game(false),
                1 => self.start_game(true),
                2 => self.open_hub(spelunky_core::hub::HubKind::Title),
                3 => self.screen(Screen::Custom),
                4 => self.screen(Screen::Editor),
                5 => self.screen(Screen::Shortcuts),
                6 => {
                    self.settings_return = Screen::Title;
                    self.screen(Screen::Settings)
                }
                7 => self.screen(Screen::Records),
                8 => self.screen(Screen::About),
                _ => self.fullscreen(),
            },
            Screen::Pause => match self.selection {
                0 => self.screen(Screen::Playing),
                1 => {
                    self.settings_return = Screen::Pause;
                    self.screen(Screen::Settings)
                }
                2 if self.editor_play => self.screen(Screen::Editor),
                _ => self.screen(Screen::Title),
            },
            Screen::Finished => match self.selection {
                0 => {
                    if self.editor_play {
                        self.screen(Screen::Editor)
                    } else if self.is_hub() {
                        let kind = self.game.as_ref().map(retry_hub).unwrap();
                        self.open_hub(kind);
                    } else if self.is_custom() {
                        self.screen(Screen::Custom)
                    } else {
                        self.start_game(false)
                    }
                }
                1 if self.is_hub() => self.return_from_challenge(),
                1 => self.screen(Screen::Title),
                _ => self.screen(Screen::Editor),
            },
            Screen::Shortcuts => {
                let levels = self.shortcuts();
                if let Some(&level) = levels.get(self.selection) {
                    self.start_game(false);
                    if let Some(game) = &mut self.game {
                        game.flags.insert("used_shortcut".into());
                        game.data.insert("next_level".into(), f64::from(level));
                        game.next_level();
                    }
                    self.reset_timing();
                } else {
                    self.screen(Screen::Title)
                }
            }
            Screen::Custom => {
                let names: Vec<String> = self.store.data.custom_levels.keys().cloned().collect();
                if self.selection < names.len() {
                    self.play_custom(&names[self.selection], false)
                } else if self.selection == names.len() {
                    self.import_dialog()
                } else {
                    self.return_from_custom_selector()
                }
            }
            _ => self.screen(Screen::Title),
        }
    }
    fn return_from_custom_selector(&mut self) {
        if self
            .game
            .as_mut()
            .is_some_and(prepare_source_selector_return)
        {
            self.open_hub(spelunky_core::hub::HubKind::Title);
        } else {
            self.screen(Screen::Title);
        }
    }
    fn play_custom(&mut self, name: &str, editor: bool) {
        self.flush_source_music();
        self.snapshot_lifetime_stats();
        self.source_message = None;
        let Some(contents) = self.store.data.custom_levels.get(name) else {
            return;
        };
        match spelunky_core::level::Level::parse(contents) {
            Ok(level) => {
                let mut game = new_custom_game(
                    wall_seed(),
                    self.store.data.settings.rules,
                    &self.source_music,
                );
                hydrate_run(&mut game, &self.store.data);
                load_custom(&mut game, &level, name, "");
                if editor {
                    game.flags.insert("editor_test".into());
                }
                self.game = Some(game);
                self.editor_play = editor;
                self.recorded = false;
                self.screen(Screen::Playing)
            }
            Err(error) => self.toast(error),
        }
    }
    fn import_level(&mut self, name: String, contents: String) {
        match spelunky_core::level::Level::parse(&contents) {
            Ok(_) => {
                let name = level_name(&name);
                if let Some(game) = &self.game {
                    self.editor.set_source_rng(game.rng.clone());
                    self.editor.set_source_context(editor_preview_context(game));
                }
                let result = self.editor.load(&name, &contents);
                if let Some(game) = &mut self.game {
                    game.rng = self.editor.source_rng().clone();
                }
                if let Err(error) = result {
                    self.toast(error);
                    return;
                }
                self.store.data.custom_levels.insert(name.clone(), contents);
                if let Err(error) = self.store.save() {
                    self.toast(error)
                } else {
                    self.toast(format!("IMPORTED {name}"))
                }
            }
            Err(error) => self.toast(format!("INVALID LEVEL: {error}")),
        }
    }
    fn import_dialog(&mut self) {
        #[cfg(target_arch = "wasm32")]
        {
            let proxy = self.proxy.clone();
            if let Err(error) = crate::editor_io::choose_level(move |result| {
                let event = match result {
                    Ok((name, text)) => UserEvent::Imported(name, text),
                    Err(error) => UserEvent::ImportError(error),
                };
                let _ = proxy.send_event(event);
            }) {
                self.toast(error);
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.toast("DROP A .LVL FILE ON THIS WINDOW TO IMPORT");
            self.editor.message("DROP A .LVL FILE ON THE WINDOW".into());
        }
    }
    fn editor_command(&mut self, command: EditorCommand) {
        if self.screen == Screen::Editor {
            if let Some(game) = &mut self.game {
                game.rng = self.editor.source_rng().clone();
                if self.editor.source_check_water() {
                    game.flags.insert("check_water".into());
                    game.flags.insert("checkWater".into());
                }
            }
            for sound in self.editor.take_source_sounds() {
                self.audio.play(sound, 0.0);
            }
        }
        match command {
            EditorCommand::None => {}
            EditorCommand::Back => {
                self.flush_source_music();
                if self.game.is_none() {
                    let mut game = Game::shell_with_music(
                        wall_seed(),
                        self.store.data.settings.rules,
                        self.source_music.session_state(),
                    );
                    hydrate_run(&mut game, &self.store.data);
                    self.game = Some(game);
                }
                if let Some(game) = &mut self.game {
                    prepare_editor_return(game);
                }
                self.editor_play = false;
                self.recorded = false;
                self.open_hub(spelunky_core::hub::HubKind::Title);
            }
            EditorCommand::Import => self.import_dialog(),
            EditorCommand::Save { name, text } => {
                self.store
                    .data
                    .custom_levels
                    .insert(level_name(&name), text);
                let result = self.store.save();
                let command = complete_editor_save(&mut self.editor, &name, result);
                self.editor_command(command);
            }
            EditorCommand::Export { name, text } => match crate::editor_io::export(&name, &text) {
                Ok(()) => self.editor.message(format!("EXPORTED {name}")),
                Err(error) => self.editor.message(error),
            },
            EditorCommand::Play(level) => {
                self.flush_source_music();
                self.snapshot_lifetime_stats();
                self.source_message = None;
                let mut game = new_custom_game(
                    wall_seed(),
                    self.store.data.settings.rules,
                    &self.source_music,
                );
                hydrate_run(&mut game, &self.store.data);
                game.rng = self.editor.source_rng().clone();
                load_custom(&mut game, &level, self.editor.level_name(), "");
                game.flags.insert("editor_test".into());
                self.game = Some(game);
                self.editor_play = true;
                self.recorded = false;
                self.screen(Screen::Playing)
            }
        }
    }
    fn finish_record(&mut self) {
        self.snapshot_lifetime_stats();
        let Some(game) = self.game.as_ref() else {
            return;
        };
        if self.recorded {
            return;
        }
        let Some(record) = campaign_record(game) else {
            return;
        };
        self.recorded = true;
        if self.editor_play {
            return;
        }
        self.game
            .as_mut()
            .unwrap()
            .flags
            .extend(record_improvements(&record, &self.store.data.records));
        let game = self.game.as_ref().unwrap();
        self.store
            .data
            .unlocks
            .extend(game.flags.iter().filter(|s| persistent_flag(s)).cloned());
        self.store
            .data
            .unlocks
            .extend(spelunky_core::hub::record_unlocks(
                record.score,
                (record.won && record.fastest_eligible).then_some(record.elapsed_seconds() as f64),
                record.kills,
                record.rescues,
            ));
        if ["unlock_sun", "unlock_moon", "unlock_stars", "unlock_damsel"]
            .iter()
            .all(|key| self.store.data.unlocks.contains(*key))
        {
            self.store.data.unlocks.insert("unlock_tunnel_man".into());
        }
        if let Err(error) = self.store.add_record(record) {
            self.toast(error)
        }
    }
    fn finish_cinematic(&mut self) -> bool {
        use spelunky_core::ending::{EndingKind, EndingStage};
        let Some(ending) = self.game.as_ref().and_then(|game| game.ending.as_ref()) else {
            return false;
        };
        if ending.stage != EndingStage::Complete {
            return false;
        }
        match ending.kind {
            EndingKind::Campaign => {
                self.finish_record();
                if let Some(game) = &mut self.game {
                    game.flags.insert("reset_run_on_hub".into());
                }
                self.open_hub(spelunky_core::hub::HubKind::Scores);
            }
            EndingKind::Custom => {
                if let Some(game) = &mut self.game {
                    game.flags.insert("reset_run_on_hub".into());
                }
                self.open_hub(spelunky_core::hub::HubKind::Scores);
            }
            EndingKind::TitleCredits => {
                self.snapshot_lifetime_stats();
                if let Some(game) = &mut self.game {
                    game.flags.remove("app_exit_requested");
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
                    self.exit_requested = true;
                }
                #[cfg(target_arch = "wasm32")]
                {
                    self.flush_source_music();
                    self.game = None;
                    self.screen(Screen::Title);
                }
            }
            EndingKind::Transition => self.transition(),
            EndingKind::HubPortal => {
                if self
                    .game
                    .as_ref()
                    .is_some_and(|game| game.state == RunState::Transition)
                {
                    self.transition();
                } else {
                    if let Some(game) = &mut self.game {
                        game.ending = None;
                    }
                    self.reset_room_timing();
                }
            }
        }
        true
    }
    fn finish_game_over(&mut self) -> bool {
        use spelunky_core::progression::GameOverDestination;
        let destination = self
            .game
            .as_ref()
            .and_then(|game| game.game_over.destination);
        let Some(destination) = destination else {
            return false;
        };
        self.snapshot_lifetime_stats();
        if let Some(game) = &self.game {
            capture_challenge_records(game, &mut self.store.data.challenge_records);
        }
        if let Err(error) = self.store.save() {
            self.toast(error);
        }
        match destination {
            GameOverDestination::Editor => self.screen(Screen::Editor),
            GameOverDestination::CustomLevels => self.screen(Screen::Custom),
            GameOverDestination::Scores => {
                if let Some(game) = &mut self.game {
                    game.flags.insert("reset_run_on_hub".into());
                }
                self.open_hub(spelunky_core::hub::HubKind::Scores);
            }
        }
        true
    }
    fn transition(&mut self) {
        if self.game.as_ref().is_some_and(|game| {
            capture_challenge_records(game, &mut self.store.data.challenge_records)
        }) {
            if let Err(error) = self.store.save() {
                self.toast(error);
            }
        }
        let Some(game) = &mut self.game else {
            return;
        };
        consume_onboarding(game, &mut self.store.data);
        if let Some(destination) = game
            .flags
            .iter()
            .find_map(|flag| flag.strip_prefix("hub_destination:"))
            .map(str::to_owned)
        {
            game.flags
                .retain(|flag| !flag.starts_with("hub_destination:"));
            match destination.as_str() {
                "title" => self.open_hub(spelunky_core::hub::HubKind::Title),
                "scores" => self.open_hub(spelunky_core::hub::HubKind::Scores),
                "sun" => self.open_hub(spelunky_core::hub::HubKind::Sun),
                "moon" => self.open_hub(spelunky_core::hub::HubKind::Moon),
                "stars" => self.open_hub(spelunky_core::hub::HubKind::Stars),
                "tutorial" => self.start_from_hub(true, 1),
                "shortcut5" | "shortcut9" | "shortcut13" => {
                    let level = destination
                        .trim_start_matches("shortcut")
                        .parse::<u32>()
                        .unwrap_or(1);
                    self.start_from_hub(false, level);
                }
                _ => self.start_from_hub(false, 1),
            }
            return;
        }
        if game.flags.contains("custom") {
            match custom_exit(game, &self.store.data.custom_levels, self.editor_play) {
                CustomExit::Editor => self.screen(Screen::Editor),
                CustomExit::Finished => {}
                CustomExit::Continued => self.reset_room_timing(),
                CustomExit::Missing(target) => {
                    self.toast(format!("LINKED LEVEL MISSING: {target}"));
                    self.screen(Screen::Custom)
                }
            }
        } else {
            let source_transition_complete = game.ending.as_ref().is_some_and(|ending| {
                ending.kind == spelunky_core::ending::EndingKind::Transition
                    && ending.stage == spelunky_core::ending::EndingStage::Complete
            });
            if let Some(donation) = (!source_transition_complete)
                .then(|| TransitionDonation::begin(game))
                .flatten()
            {
                self.donation = Some(donation);
                self.donation_confirm = false;
                self.screen(Screen::Donation);
            } else {
                game.next_level();
                self.reset_room_timing();
            }
        }
    }
    fn continue_transition(&mut self) {
        self.donation = None;
        self.donation_confirm = false;
        if let Some(game) = &mut self.game {
            game.next_level();
            resume_room_music(game);
        }
        self.screen(Screen::Playing);
    }
    fn shortcuts(&self) -> Vec<u32> {
        [5, 9, 13]
            .into_iter()
            .filter(|level| {
                self.store
                    .data
                    .unlocks
                    .contains(&format!("shortcut_{level}"))
            })
            .collect()
    }
    fn update(&mut self) {
        self.flush_source_music();
        if !matches!(self.screen, Screen::Playing | Screen::Finished) {
            let pressed = self.input.take_music_pressed();
            if pressed && source_music_input_active(self.screen, self.game.as_ref()) {
                if let Some(game) = &mut self.game {
                    spelunky_core::music::key_pressed(game);
                }
            }
        }
        if self.exit_requested {
            return;
        }
        let now = self.now();
        let host_frame = self.timing.begin_frame(now);
        if self.focused && !self.occluded {
            self.input.update_gamepad();
        }
        let controller_buttons = self.input.controller_pressed();
        let mut buttons = self.input.ui_pressed();
        if self.screen == Screen::Settings
            && settings_controller_frame(
                &mut self.settings_ui,
                &mut self.store.data.settings,
                &controller_buttons,
                &mut buttons,
            )
        {
            self.settings_changed();
        }
        if !buttons.is_empty() {
            self.audio.unlock();
        }
        if self.screen == Screen::Pause && self.focused && !self.occluded {
            let input = self.input.take_tick();
            let start = std::mem::take(&mut self.pause_start_requested)
                || buttons.contains(&UiButton::Pause);
            if let Some(control) = source_paused_control(self.game.as_ref(), &input, start) {
                // Bomb's branch consumes Rope/Start even if its room/life guard fails.
                buttons.clear();
                match control {
                    SourcePausedControl::Blocked => {}
                    SourcePausedControl::Suicide => {
                        let stop_music = self.game.as_mut().is_some_and(apply_source_pause_suicide);
                        self.screen(Screen::Playing);
                        if stop_music {
                            self.audio.stop_music();
                        }
                    }
                    SourcePausedControl::Quit => {
                        self.exit_requested = true;
                        self.input.clear();
                        self.audio.stop_music();
                        self.audio.pause(true);
                        return;
                    }
                    SourcePausedControl::Resume => self.screen(Screen::Playing),
                }
            }
        }
        let controller_screen = self.screen;
        for button in buttons {
            // One physical control may be both Confirm/Cancel and remapped
            // Start. Its remaining edges belong to the screen it came from.
            if self.screen != controller_screen {
                break;
            }
            if self.screen == Screen::Donation {
                match button {
                    UiButton::Confirm => self.donation_confirm = true,
                    UiButton::Cancel | UiButton::Pause => self.continue_transition(),
                    _ => {}
                }
                continue;
            }
            if self.screen == Screen::Playing
                && self
                    .game
                    .as_ref()
                    .and_then(|game| game.opening.as_ref())
                    .is_some_and(|opening| !opening.can_pause)
            {
                self.cinematic_action |= matches!(
                    button,
                    UiButton::Confirm | UiButton::Cancel | UiButton::Pause
                );
                continue;
            }
            if self.screen == Screen::Playing
                && self
                    .game
                    .as_ref()
                    .is_some_and(|game| game.state == RunState::Ending)
            {
                self.cinematic_action |= matches!(
                    button,
                    UiButton::Confirm | UiButton::Cancel | UiButton::Pause
                );
                continue;
            }
            if self.screen == Screen::Playing
                && self
                    .game
                    .as_ref()
                    .is_some_and(|game| game.state == RunState::Dead)
            {
                self.cinematic_action |= button == UiButton::Pause;
                continue;
            }
            if button == UiButton::Pause && self.screen == Screen::Playing {
                self.screen(Screen::Pause);
                continue;
            }
            if self.screen != Screen::Playing {
                let capturing =
                    self.screen == Screen::Settings && self.settings_ui.awaiting_binding();
                if let Some(key) = controller_menu_key(button, capturing) {
                    if self.screen == Screen::Pause {
                        // These are menu edges, not physical keyboard presses.
                        // Source action edges were resolved above; keyboard aliases
                        // must not swallow a controller navigation or confirmation.
                        self.menu_key(key);
                    } else {
                        self.key(key, None);
                    }
                }
            }
        }
        if self.screen == Screen::Editor && self.focused && !self.occluded {
            let frame = self.timing.advance(host_frame);
            for _ in 0..frame.ticks {
                let command = self.editor.source_tick();
                self.editor_command(command);
                if self.screen != Screen::Editor {
                    break;
                }
            }
        }
        if self.screen == Screen::Donation && self.focused && !self.occluded {
            let frame = self.timing.advance(host_frame);
            for _ in 0..frame.ticks {
                let mut input = self.input.take_tick();
                input.pay_pressed |= self.donation_confirm;
                self.donation_confirm = false;
                if let (Some(ui), Some(game)) = (&mut self.donation, &mut self.game) {
                    let paid = ui.tick(game, &input);
                    if paid > 0 {
                        self.message = format!("THANKS FOR THE ${paid} DONATION!");
                        self.message_simulated = false;
                        self.message_until = now + 4.;
                    }
                }
            }
        }
        if self.screen == Screen::Playing && self.focused && !self.occluded {
            let frame = self.timing.advance(host_frame);
            for _ in 0..frame.ticks {
                let mut input = self.input.take_tick();
                apply_cinematic_start(&mut input, &mut self.cinematic_action);
                if let Some(game) = &mut self.game {
                    game.down_to_run = self.store.data.settings.down_to_run;
                    consume_source_music(&mut self.audio, &mut self.source_music, game);
                    game.step(&input);
                    consume_source_music(&mut self.audio, &mut self.source_music, game);
                    self.timing.history_ready = true;
                    for event in &game.events {
                        match event {
                            GameEvent::SoundStop { name } => self.audio.stop_sound(name),
                            GameEvent::SoundPan { name, pan } => {
                                self.audio.set_sound_pan(name, *pan)
                            }
                            GameEvent::Sound { name, pos } => self
                                .audio
                                .play(name, ((pos.x - game.camera.x) / 320. * 2. - 1.) as f32),
                            GameEvent::SoundIfIdle { name, pos } => self.audio.play_if_idle(
                                name,
                                ((pos.x - game.camera.x) / 320. * 2. - 1.) as f32,
                            ),
                            GameEvent::Message(message) => {
                                self.message = message.clone();
                                self.message_until = now + 5.;
                                self.message_simulated = game
                                    .data
                                    .get("message_timer")
                                    .is_some_and(|timer| *timer > 0.);
                                self.source_message = self.message_simulated.then(|| {
                                    let (first, second) =
                                        message.split_once('\n').unwrap_or((message, ""));
                                    spelunky_core::SourceMessage {
                                        first: first.into(),
                                        second: second.into(),
                                        ticks: game
                                            .data
                                            .get("message_timer")
                                            .copied()
                                            .unwrap_or(0.)
                                            .max(0.)
                                            as u32,
                                    }
                                });
                            }
                            GameEvent::Dialogue(message) => {
                                self.source_message = Some(message.clone());
                            }
                            _ => {}
                        }
                    }
                }
                if self.finish_cinematic() {
                    break;
                }
                if self.finish_game_over() {
                    break;
                }
                let state = self.game.as_ref().map(|g| g.state);
                if state == Some(RunState::Transition)
                    || self.game.as_ref().is_some_and(|game| {
                        game.flags
                            .iter()
                            .any(|flag| flag.starts_with("hub_destination:"))
                    })
                {
                    self.transition();
                    break;
                }
                if state == Some(RunState::Dead) {
                    // Source writes highscores on the lethal frame, before
                    // corpse/world simulation advances the room clock again.
                    self.finish_record();
                } else if state == Some(RunState::Won) {
                    self.finish_record();
                    self.open_hub(spelunky_core::hub::HubKind::Scores);
                    break;
                }
            }
        }
        if self.screen == Screen::Finished
            && self.focused
            && !self.occluded
            && self
                .game
                .as_ref()
                .is_some_and(|game| game.state == RunState::Dead)
        {
            let frame = self.timing.advance(host_frame);
            for _ in 0..frame.ticks {
                if let Some(game) = &mut self.game {
                    game.down_to_run = self.store.data.settings.down_to_run;
                    let input = self.input.take_tick();
                    consume_source_music(&mut self.audio, &mut self.source_music, game);
                    game.step(&spelunky_core::InputFrame {
                        music_pressed: input.music_pressed,
                        ..Default::default()
                    });
                    consume_source_music(&mut self.audio, &mut self.source_music, game);
                    self.timing.history_ready = true;
                    for event in &game.events {
                        match event {
                            GameEvent::SoundStop { name } => self.audio.stop_sound(name),
                            GameEvent::SoundPan { name, pan } => {
                                self.audio.set_sound_pan(name, *pan)
                            }
                            GameEvent::Sound { name, pos } => self
                                .audio
                                .play(name, ((pos.x - game.camera.x) / 320. * 2. - 1.) as f32),
                            GameEvent::SoundIfIdle { name, pos } => self.audio.play_if_idle(
                                name,
                                ((pos.x - game.camera.x) / 320. * 2. - 1.) as f32,
                            ),
                            _ => {}
                        }
                    }
                }
            }
        }
        if let Some(game) = &mut self.game {
            consume_onboarding(game, &mut self.store.data);
        }
        let highscores_reset = self
            .game
            .as_mut()
            .is_some_and(|game| consume_highscores_reset(game, &mut self.store.data));
        if highscores_reset {
            if let Err(error) = self.store.save() {
                self.toast(error);
            }
        }
        self.snapshot_lifetime_stats();
        if let Some(game) = &self.game {
            let previous = self.store.data.unlocks.clone();
            let previous_challenges = self.store.data.challenge_records.clone();
            let previous_tunnels = self.store.data.tunnel_remaining.clone();
            self.store
                .data
                .tunnel_remaining
                .extend(tunnel_progress(game));
            capture_challenge_records(game, &mut self.store.data.challenge_records);
            if game.flags.contains("hub") {
                self.store
                    .data
                    .unlocks
                    .retain(|flag| !flag.starts_with("character_"));
            }
            self.store.data.unlocks.extend(
                game.flags
                    .iter()
                    .filter(|flag| persistent_flag(flag))
                    .cloned(),
            );
            if self.store.data.unlocks != previous
                || self.store.data.challenge_records != previous_challenges
                || self.store.data.tunnel_remaining != previous_tunnels
            {
                if let Err(error) = self.store.save() {
                    self.toast(error)
                }
            }
        }
        self.audio.set_volumes(
            self.store.data.settings.music_volume,
            self.store.data.settings.effects_volume,
        );
        // Only actual source producers authorize a new play/stop. Presentation
        // polling must not replace literal M selection or restart after OFF.
        self.flush_source_music();
        self.audio.update();
    }
    fn draw(&self, alpha: f64) -> Vec<DrawQuad> {
        let Some(atlas) = &self.atlas else {
            return vec![];
        };
        let viewport = self.viewport();
        let mut quads = if matches!(
            self.screen,
            Screen::Playing | Screen::Pause | Screen::Finished | Screen::Donation
        ) {
            self.game
                .as_ref()
                .map(|g| {
                    scene::game_scene_in_view(
                        g,
                        alpha,
                        self.store.data.settings.pixel_snap,
                        atlas,
                        viewport,
                    )
                })
                .unwrap_or_default()
        } else {
            let mut q = vec![];
            rect(
                &mut q,
                [0., 0., viewport.width, viewport.height],
                [0.055, 0.035, 0.03, 1.],
            );
            q
        };
        // Scene owns the expanded world and its HUD. Host menus, editor and
        // overlays retain their authored coordinates within the centered UI.
        let ui_start = quads.len();
        match self.screen {
            Screen::Playing => {
                if let Some(game) = self.game.as_ref().filter(|game| {
                    game.state == RunState::Dead
                        || (game.flags.contains("hub_moon")
                            && game.data.get("timer").copied().unwrap_or(0.0) < 0.0)
                }) {
                    let hint = source_game_over_hint(
                        game,
                        self.editor_play,
                        &self.input.action_label("attack"),
                    );
                    scene::game_over_overlay(&mut quads, atlas, game, &hint);
                }
            }
            Screen::Donation => {
                rect(
                    &mut quads,
                    [22., 25., 276., 190.],
                    [0.035, 0.02, 0.015, 0.97],
                );
                text(&mut quads, atlas, "TUNNEL MAN", 99., 38., 1.5, GOLD);
                if let Some(frame) = atlas.sprite("sTunnelManLeft", 0.) {
                    quads.push(DrawQuad {
                        rect: [38., 76., frame.size[0] * 2., frame.size[1] * 2.],
                        uv: frame.uv,
                        color: WHITE,
                        ..Default::default()
                    });
                }
                if let (Some(ui), Some(game)) = (&self.donation, &self.game) {
                    text(
                        &mut quads,
                        atlas,
                        &format!("SHORTCUT TO LEVEL {}", ui.destination),
                        91.,
                        75.,
                        1.,
                        WHITE,
                    );
                    text(
                        &mut quads,
                        atlas,
                        &format!("STILL NEEDED: ${}", ui.remaining(game)),
                        91.,
                        94.,
                        0.9,
                        WHITE,
                    );
                    text(
                        &mut quads,
                        atlas,
                        &format!("YOUR GOLD: ${}", game.player.money),
                        91.,
                        111.,
                        0.9,
                        WHITE,
                    );
                    if ui.complete(game) {
                        text(
                            &mut quads,
                            atlas,
                            "THE SHORTCUT IS OPEN!",
                            60.,
                            141.,
                            1.,
                            GOLD,
                        );
                    } else {
                        text(
                            &mut quads,
                            atlas,
                            &format!("DONATE: ${}", ui.amount),
                            83.,
                            141.,
                            1.4,
                            GOLD,
                        );
                    }
                }
                text(
                    &mut quads,
                    atlas,
                    "UP/DOWN SELECT (HOLD TO SPEED UP)",
                    35.,
                    173.,
                    0.85,
                    WHITE,
                );
                text(
                    &mut quads,
                    atlas,
                    "ATTACK / PAY / A: DONATE",
                    56.,
                    188.,
                    0.85,
                    WHITE,
                );
                text(
                    &mut quads,
                    atlas,
                    "ENTER / ESC / B: CONTINUE",
                    54.,
                    201.,
                    0.85,
                    WHITE,
                );
            }
            Screen::Editor => self.editor.draw(atlas, &mut quads),
            Screen::Settings => self
                .settings_ui
                .draw(atlas, &mut quads, &self.store.data.settings),
            Screen::Title => {
                text(&mut quads, atlas, "SPELUNKY", 64., 18., 2.8, GOLD);
                text(&mut quads, atlas, "CLASSIC / RUST", 88., 45., 1., WHITE);
                self.menu(
                    &mut quads,
                    atlas,
                    &[
                        "START ADVENTURE",
                        "TUTORIAL",
                        "PLAY ORIGINAL HUB",
                        "CUSTOM LEVELS",
                        "LEVEL EDITOR",
                        "SHORTCUTS",
                        "SETTINGS / CONTROLS",
                        "RECORDS",
                        "ABOUT / LICENSE",
                        "FULLSCREEN",
                    ],
                    72.,
                    60.,
                    14.,
                );
                text(
                    &mut quads,
                    atlas,
                    "ARROWS + ENTER / CONTROLLER",
                    54.,
                    213.,
                    0.8,
                    WHITE,
                );
                text(
                    &mut quads,
                    atlas,
                    "F2 EDITOR  F4 FULLSCREEN",
                    65.,
                    224.,
                    0.8,
                    WHITE,
                );
                text(
                    &mut quads,
                    atlas,
                    "UNOFFICIAL MOD - (C) 2008,2009 DEREK YU/MOSSMOUTH",
                    12.,
                    234.,
                    0.55,
                    WHITE,
                );
            }
            Screen::Pause => {
                self.panel(&mut quads);
                text(&mut quads, atlas, "PAUSED", 123., 53., 1.5, GOLD);
                self.menu(
                    &mut quads,
                    atlas,
                    if self.editor_play {
                        &["RESUME", "SETTINGS", "RETURN TO EDITOR", "TITLE"]
                    } else {
                        &["RESUME", "SETTINGS", "TITLE"]
                    },
                    84.,
                    91.,
                    20.,
                );
            }
            Screen::Finished => {
                self.panel(&mut quads);
                let won = self.game.as_ref().is_some_and(|g| g.state == RunState::Won);
                text(
                    &mut quads,
                    atlas,
                    if won {
                        "YOU MADE IT!"
                    } else {
                        "ADVENTURE OVER"
                    },
                    82.,
                    49.,
                    1.5,
                    GOLD,
                );
                if let Some(game) = &self.game {
                    text(
                        &mut quads,
                        atlas,
                        &format!(
                            "${}  LEVEL {}  {:.1}S",
                            game.player.money,
                            game.world.level,
                            (source_elapsed_ms(game) / 1000) as f64
                        ),
                        66.,
                        79.,
                        1.,
                        WHITE,
                    );
                }
                self.menu(
                    &mut quads,
                    atlas,
                    if self.editor_play {
                        &["RETURN TO EDITOR", "TITLE"]
                    } else if self.is_hub() {
                        &["RETRY CHALLENGE", "RETURN TO HUB"]
                    } else if self.is_custom() {
                        &["CUSTOM LEVELS", "TITLE"]
                    } else {
                        &["NEW ADVENTURE", "TITLE"]
                    },
                    81.,
                    116.,
                    22.,
                );
            }
            Screen::Shortcuts => {
                text(&mut quads, atlas, "TUNNEL SHORTCUTS", 68., 20., 1.5, GOLD);
                let mut entries: Vec<String> = self
                    .shortcuts()
                    .iter()
                    .map(|level| format!("START AT LEVEL {level}"))
                    .collect();
                if entries.is_empty() {
                    text(
                        &mut quads,
                        atlas,
                        "DONATE TO TUNNEL MAN TO OPEN SHORTCUTS",
                        16.,
                        70.,
                        0.85,
                        WHITE,
                    );
                }
                entries.push("BACK".into());
                let refs: Vec<&str> = entries.iter().map(String::as_str).collect();
                self.menu(&mut quads, atlas, &refs, 73., 103., 23.);
            }
            Screen::Custom => {
                text(&mut quads, atlas, "CUSTOM LEVELS", 74., 12., 1.5, GOLD);
                let mut names: Vec<String> =
                    self.store.data.custom_levels.keys().cloned().collect();
                names.extend(["IMPORT .LVL".into(), "BACK".into()]);
                let start = self.selection.saturating_sub(9);
                for (index, name) in names.iter().enumerate().skip(start).take(11) {
                    let color = if index == self.selection { GOLD } else { WHITE };
                    text(
                        &mut quads,
                        atlas,
                        &format!(
                            "{} {}",
                            if index == self.selection { ">" } else { " " },
                            truncate(name, 32)
                        ),
                        30.,
                        43. + (index - start) as f32 * 15.,
                        1.,
                        color,
                    );
                }
                text(
                    &mut quads,
                    atlas,
                    "ENTER PLAY / E EDIT / DROP .LVL IMPORT",
                    14.,
                    223.,
                    0.8,
                    WHITE,
                );
            }
            Screen::Records => {
                text(&mut quads, atlas, "RUN RECORDS", 88., 14., 1.5, GOLD);
                let mut records: Vec<_> = self.store.data.records.iter().collect();
                records.sort_by_key(|r| std::cmp::Reverse(r.score));
                for (index, record) in records.iter().take(11).enumerate() {
                    text(
                        &mut quads,
                        atlas,
                        &format!(
                            "${:<7} L{:02} {:>5.1}S K{:03} R{:02} {} {}",
                            record.score,
                            record.level,
                            record.elapsed_seconds() as f64,
                            record.kills,
                            record.rescues,
                            if record.rules == spelunky_core::RulesPreset::Classic {
                                "CLASSIC"
                            } else {
                                "ASSIST"
                            },
                            if record.won { "WIN" } else { "" }
                        ),
                        10.,
                        49. + index as f32 * 14.,
                        0.9,
                        WHITE,
                    );
                }
                if records.is_empty() {
                    text(
                        &mut quads,
                        atlas,
                        "YOUR ADVENTURES WILL APPEAR HERE",
                        24.,
                        90.,
                        1.,
                        WHITE,
                    );
                }
                text(
                    &mut quads,
                    atlas,
                    "ESC / ENTER BACK",
                    102.,
                    220.,
                    0.8,
                    WHITE,
                );
            }
            Screen::About => {
                text(&mut quads, atlas, "SPELUNKY CLASSIC", 50., 20., 1.6, GOLD);
                for (index, line) in [
                    "(C) 2008,2009 DEREK YU / MOSSMOUTH",
                    "UNOFFICIAL MODIFIED RUST PORT",
                    "CLASSIC 1.1 REFERENCE",
                    "FREE DISTRIBUTION - DO NOT SELL",
                    "LICENSE: ASSETS/COPYING.TXT",
                    "",
                    "30 HZ AUTHORITATIVE SIMULATION",
                    "SMOOTH DISPLAY-REFRESH PRESENTATION",
                    "NATIVE LINUX + WEBGL 2 / WASM",
                    "",
                    "ARROWS MOVE, Z JUMP, X ATTACK",
                    "C CYCLE, B BOMB, R ROPE, F FLARE",
                    "SHIFT RUN, UP EXIT, P PAY",
                ]
                .iter()
                .enumerate()
                {
                    text(
                        &mut quads,
                        atlas,
                        line,
                        13.,
                        53. + index as f32 * 11.,
                        0.85,
                        WHITE,
                    );
                }
                text(&mut quads, atlas, "ESC / ENTER BACK", 98., 220., 0.8, WHITE);
            }
        }
        if self.store.warning.is_some() {
            rect(&mut quads, [0., 0., 320., 10.], [0.4, 0.08, 0.03, 0.95]);
            text(
                &mut quads,
                atlas,
                "SAVE UNAVAILABLE - SESSION DATA IS IN MEMORY",
                4.,
                1.,
                0.7,
                WHITE,
            );
        }
        let message_visible = if self.message_simulated {
            matches!(
                self.screen,
                Screen::Playing | Screen::Pause | Screen::Finished | Screen::Donation
            ) && self.game.as_ref().is_some_and(|game| {
                game.data
                    .get("message_timer")
                    .is_some_and(|timer| *timer > 0.)
            })
        } else {
            self.now() < self.message_until
        };
        let source_visible = self.source_message.as_ref().is_some_and(|_| {
            self.screen == Screen::Playing
                && self.game.as_ref().is_some_and(|game| {
                    game.data
                        .get("message_timer")
                        .is_some_and(|timer| *timer > 0.)
                })
        });
        if source_visible {
            let message = self.source_message.as_ref().unwrap();
            let pay = self.input.action_label("pay");
            scene::source_message(
                &mut quads,
                atlas,
                &message.first.replace("{PAY}", &pay),
                &message.second.replace("{PAY}", &pay),
                self.game
                    .as_ref()
                    .is_some_and(|game| game.flags.contains("tutorial")),
            );
        }
        if !source_visible
            && !(self.message_simulated && self.source_message.is_some())
            && !self.message.is_empty()
            && message_visible
        {
            let lines = wrap_message(&self.message, 54);
            let height = lines.len() as f32 * 9. + 8.;
            let top = 240. - height;
            rect(&mut quads, [0., top, 320., height], [0., 0., 0., 0.92]);
            for (index, line) in lines.iter().enumerate() {
                text(
                    &mut quads,
                    atlas,
                    line,
                    5.,
                    top + 4. + index as f32 * 9.,
                    0.85,
                    GOLD,
                );
            }
        }
        offset_fixed_ui(&mut quads[ui_start..], viewport.ui_origin());
        quads
    }
    fn viewport(&self) -> Viewport {
        self.renderer
            .as_ref()
            .map(Renderer::viewport)
            .unwrap_or_else(Viewport::canonical)
    }
    #[cfg(target_arch = "wasm32")]
    fn publish_test_snapshot(&self, alpha: f64) {
        let Some(window) = web_sys::window() else {
            return;
        };
        let enabled = js_sys::Reflect::get(&window, &"__SPELUNKY_TEST_MODE__".into())
            .ok()
            .and_then(|value| value.as_bool())
            == Some(true);
        if !enabled {
            return;
        }
        let mut snapshot = test_snapshot(
            self.screen,
            self.game.as_ref(),
            alpha,
            self.store.data.settings.pixel_snap,
        );
        apply_viewport_snapshot(
            &mut snapshot,
            self.game.as_ref(),
            self.viewport(),
            self.store.data.settings.pixel_snap,
        );
        snapshot["sourceMessage"] = self.source_message.as_ref().map(|message| {
            serde_json::json!({
                "first": message.first, "second": message.second,
                "timer": self.game.as_ref().and_then(|game| game.data.get("message_timer")).copied().unwrap_or(0.),
            })
        }).unwrap_or(serde_json::Value::Null);
        if let Ok(value) = js_sys::JSON::parse(&snapshot.to_string()) {
            let _ = js_sys::Reflect::set(&window, &"__spelunkyTest".into(), &value);
        }
    }
    fn panel(&self, q: &mut Vec<DrawQuad>) {
        rect(q, [40., 35., 240., 165.], [0.025, 0.015, 0.015, 0.95]);
    }
    fn menu(
        &self,
        q: &mut Vec<DrawQuad>,
        atlas: &Atlas,
        items: &[&str],
        x: f32,
        y: f32,
        spacing: f32,
    ) {
        for (index, item) in items.iter().enumerate() {
            text(
                q,
                atlas,
                &format!("{} {item}", if index == self.selection { ">" } else { " " }),
                x,
                y + index as f32 * spacing,
                1.,
                if index == self.selection { GOLD } else { WHITE },
            );
        }
    }
    fn pointer(&mut self) {
        if self.screen == Screen::Editor {
            let command = self.editor.stage_pointer(
                self.cursor[0],
                self.cursor[1],
                self.mouse_left,
                self.mouse_right,
            );
            self.editor_command(command);
        }
    }
}
#[cfg(any(test, target_arch = "wasm32"))]
fn test_snapshot(
    screen: Screen,
    game: Option<&Game>,
    alpha: f64,
    pixel_snap: bool,
) -> serde_json::Value {
    let Some(game) = game else {
        return serde_json::json!({ "screen": format!("{screen:?}"), "player": null });
    };
    let presentation = game.presentation();
    let alpha = if presentation.flags.contains("render_discontinuity") {
        1.
    } else {
        alpha
    };
    let interpolate = |previous: spelunky_core::Vec2, current: spelunky_core::Vec2| {
        if previous.distance(current) > 64. {
            current
        } else {
            previous.lerp(current, alpha)
        }
    };
    let player = &game.player;
    let drawn_player = &presentation.player;
    let rendered = interpolate(drawn_player.previous_pos, drawn_player.pos);
    let camera = interpolate(presentation.previous_camera, presentation.camera);
    let screen_pos = spelunky_core::Vec2::new(rendered.x - camera.x, rendered.y - camera.y);
    let screen_x = if pixel_snap {
        screen_pos.x.round()
    } else {
        screen_pos.x
    };
    let screen_y = if pixel_snap {
        screen_pos.y.round()
    } else {
        screen_pos.y
    };
    let flares: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|entity| entity.alive && spelunky_core::catalog::is_a(&entity.object, "oFlare"))
        .map(|entity| {
            serde_json::json!({
                "id": entity.id, "x": entity.pos.x, "y": entity.pos.y,
                "dist_to_player": entity.data.get("dist_to_player"),
            })
        })
        .collect();
    serde_json::json!({
        "screen": format!("{screen:?}"), "state": format!("{:?}", game.state),
        "tick": game.tick, "levelTick": game.level_tick, "alpha": alpha,
        "seed": game.seed.to_string(),
        "rngAlgorithm": game.rng.algorithm(),
        "simulationVersion": game.simulation_version(),
        "rngState": game.rng.state_bits().to_string(),
        "deathTicks": game.data.get("death_ticks").copied().unwrap_or(0.),
        "money": player.money,
        "sourceTimeMs": game.source_elapsed_ms(),
        "opening": game.opening.as_ref().map(|opening| serde_json::json!({
            "kind": format!("{:?}", opening.kind), "stage": opening.stage,
            "drawStatus": opening.draw_status, "canPause": opening.can_pause,
            "playerPresent": opening.player_present, "fadeLevel": opening.fade_level,
        })),
        "gameOver": {
            "drawStatus": game.game_over.draw_status, "moneyCount": game.game_over.money_count,
            "acknowledged": game.game_over.acknowledged,
            "destination": game.game_over.destination.map(|destination| format!("{destination:?}")),
        },
        "ending": game.ending.as_ref().map(|ending| serde_json::json!({
            "kind": format!("{:?}", ending.kind), "stage": format!("{:?}", ending.stage),
            "stageTick": ending.stage_tick, "drawStatus": ending.draw_status,
            "drawLoot": ending.draw_loot,
            "levelMoney": ending.level_money(),
            "moneyCount": ending.money_count, "bonusAwarded": ending.bonus_awarded,
            "recordReady": ending.record_ready, "sourceTimeMs": ending.source_time_ms,
            "shortcutUsed": ending.shortcut_used, "fadeLevel": ending.fade_level,
            "fadeIn": ending.fade_in, "fadeOut": ending.fade_out,
            "creditsPage": ending.credits_page,
        })),
        "darkness": game.data.get("darkness"),
        "darkness_lerp": game.data.get("darkness_lerp"),
        "flares": flares,
        "lighting": scene::lighting_diagnostics(game, alpha),
        "world": {
            "name": game.world.name, "width": game.world.width, "height": game.world.height,
            "level": game.world.level, "entityCount": game.world.entities.len(),
            "dark": game.world.dark,
        },
        "player": {
            "x": player.pos.x, "y": player.pos.y,
            "previousX": player.previous_pos.x, "previousY": player.previous_pos.y,
            "vx": player.vel.x, "vy": player.vel.y, "state": format!("{:?}", player.state),
            "grounded": player.grounded, "sprite": player.sprite,
            "health": player.health, "stun": player.stun, "invincible": player.invincible,
            "bombs": player.bombs, "ropes": player.ropes,
            "dist_to_nearest_light_source": player.data.get("dist_to_nearest_light_source"),
        },
        "rendered": {
            "x": rendered.x, "y": rendered.y, "screenX": screen_x, "screenY": screen_y,
            "previousX": drawn_player.previous_pos.x, "previousY": drawn_player.previous_pos.y,
            "currentX": drawn_player.pos.x, "currentY": drawn_player.pos.y,
            "sprite": drawn_player.sprite, "frame": drawn_player.animation,
        },
        "camera": {
            "x": presentation.camera.x, "y": presentation.camera.y,
            "previousX": presentation.previous_camera.x, "previousY": presentation.previous_camera.y,
            "renderedX": camera.x, "renderedY": camera.y,
            "authoritativeX": game.camera.x, "authoritativeY": game.camera.y,
        },
    })
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SourceHostControl {
    Suicide,
    Quit,
    Editor,
    CustomLevels,
}
fn source_host_control(
    screen: Screen,
    game: Option<&Game>,
    key: KeyCode,
) -> Option<SourceHostControl> {
    if screen == Screen::Pause {
        return match key {
            KeyCode::F1 if game.is_some_and(|game| game.player.health > 0 && game.is_level()) => {
                Some(SourceHostControl::Suicide)
            }
            KeyCode::F10 => Some(SourceHostControl::Quit),
            _ => None,
        };
    }
    if screen == Screen::Playing && game.is_some_and(|game| game.world.name == "rTitle") {
        return match key {
            KeyCode::F2 => Some(SourceHostControl::Editor),
            KeyCode::F3 => Some(SourceHostControl::CustomLevels),
            _ => None,
        };
    }
    None
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SourcePausedControl {
    Blocked,
    Suicide,
    Quit,
    Resume,
}
fn source_paused_control(
    game: Option<&Game>,
    input: &spelunky_core::InputFrame,
    start: bool,
) -> Option<SourcePausedControl> {
    if input.bomb_pressed {
        Some(
            if game.is_some_and(|game| game.player.health > 0 && game.is_level()) {
                SourcePausedControl::Suicide
            } else {
                SourcePausedControl::Blocked
            },
        )
    } else if input.rope_pressed {
        Some(SourcePausedControl::Quit)
    } else if start {
        Some(SourcePausedControl::Resume)
    } else {
        None
    }
}
fn apply_source_pause_suicide(game: &mut Game) -> bool {
    game.player.vel = spelunky_core::Vec2::new(if game.player.facing < 0 { -3. } else { 3. }, -6.);
    game.player.health = -99;
    let stop_music = !game.player.equipment.contains("Ankh");
    if stop_music {
        game.flags.insert("source_music_stopped".into());
    }
    stop_music
}
fn apply_source_suicide(game: &mut Game) {
    game.player.vel = spelunky_core::Vec2::new(if game.player.facing < 0 { -3. } else { 3. }, -6.);
    game.player.health = -99;
    game.flags.insert("source_music_stopped".into());
}
fn source_music_input_active(screen: Screen, game: Option<&Game>) -> bool {
    matches!(
        screen,
        Screen::Playing | Screen::Finished | Screen::Title | Screen::Records | Screen::Donation
    ) && game.is_some_and(spelunky_core::music::has_active_game)
}
fn take_source_music(game: &mut Game, session: &mut SourceMusicState) -> Vec<SourceMusicCommand> {
    let commands = game.source_music.take_commands();
    *session = game.source_music.session_state();
    commands
}
fn supported_source_frequency(hz: i64) -> Result<u32, i64> {
    u32::try_from(hz).ok().filter(|hz| *hz > 0).ok_or(hz)
}
fn consume_source_music(audio: &mut Audio, session: &mut SourceMusicState, game: &mut Game) {
    for command in take_source_music(game, session) {
        match command {
            SourceMusicCommand::Play {
                name,
                looping,
                if_idle,
            } => audio.source_music(&name, looping, if_idle),
            SourceMusicCommand::StopAll => audio.stop_all_music(),
            SourceMusicCommand::Frequency { name, hz } => {
                // Raw signed intent remains in session.frequencies. Unsupported
                // requests retain the last valid backend rate; no clamp/wrap.
                if let Ok(hz) = supported_source_frequency(hz) {
                    audio.set_source_music_frequency(&name, hz);
                }
            }
        }
    }
}
fn resume_room_music(game: &mut Game) {
    game.flags.remove("source_music_stopped");
}
fn prepare_source_selector_return(game: &mut Game) -> bool {
    if !game.flags.remove("source_load_level_menu") {
        return false;
    }
    game.flags.retain(|flag| !flag.starts_with("title_start:"));
    game.flags.insert("title_start:2".into());
    resume_room_music(game);
    true
}
#[cfg(test)]
fn stop_music_immediately(game: &Game, music: &str) -> bool {
    music.is_empty()
        && (game.state == RunState::Dead
            || game.ending.is_some()
            || game.opening.is_some()
            || game.flags.contains("hub")
            || game.flags.contains("source_music_stopped"))
}
/// Source Room Create starts level music before the destination's first Step.
/// Reset here too, because host polling can start a stored resource before the
/// core's first-tick reset event is dispatched.
#[cfg(test)]
fn initial_level_music_frequency(game: &Game, music: &str) -> Option<u32> {
    (game.is_level()
        && game.level_tick == 0
        && game.opening.is_none()
        && game.ending.is_none()
        && !music.is_empty())
    .then_some(44_100)
}
#[cfg(test)]
fn game_music_request(game: &Game) -> (&'static str, bool) {
    (
        game_music(game),
        game.ending
            .as_ref()
            .is_none_or(|ending| ending.music_looping),
    )
}
#[cfg(test)]
fn game_music(game: &Game) -> &'static str {
    if let Some(ending) = &game.ending {
        return match ending.music.as_str() {
            "musVictory" => "victory",
            "musCredits" => "credits",
            "musTitle" => "title",
            "musBoss" => "boss",
            _ => "",
        };
    }
    if let Some(opening) = &game.opening {
        return if opening.kind == spelunky_core::opening::OpeningKind::Title && opening.can_pause {
            "title"
        } else {
            ""
        };
    }
    if game.flags.contains("source_music_stopped") {
        return "";
    }
    if game.flags.contains("hub") {
        return if game.flags.contains("hub_title") {
            "title"
        } else if ["hub_sun", "hub_moon", "hub_stars"]
            .iter()
            .any(|flag| game.flags.contains(*flag))
            && game.level_tick >= 100
            && game.state != RunState::Dead
        {
            "boss"
        } else {
            ""
        };
    }
    if game.state == RunState::Dead {
        ""
    } else if game.state == RunState::Won {
        "victory"
    } else if game.flags.contains("custom") || game.flags.contains("custom_level") {
        // rLoadLevel's selected music does not change campaign area rules.
        // startMusic recognizes these four codes; all other values use Cave.
        match game.data.get("custom_music").copied() {
            Some(1.) => "lush",
            Some(2.) => "ice",
            Some(3.) => "temple",
            Some(4.) if game.player.data.get("active") == Some(&0.0) => "",
            Some(4.) => "boss",
            _ => "cave",
        }
    } else {
        match game.world.area {
            Area::Mines => "cave",
            Area::Jungle => "lush",
            Area::Ice => "ice",
            Area::Temple => "temple",
            // startMusic waits for Olmec Alarm 5 to restore player control.
            Area::Boss if game.player.data.get("active") == Some(&0.0) => "",
            Area::Boss => "boss",
        }
    }
}
fn hydrate_run(game: &mut Game, data: &crate::storage::SaveData) {
    game.lifetime_stats = data.lifetime_stats.clone();
    donation_ui::hydrate(game, &data.tunnel_remaining);
    game.flags.extend(
        data.unlocks
            .iter()
            .filter(|flag| persistent_flag(flag))
            .cloned(),
    );
    if data.first_time {
        game.flags.insert("first_time".into());
    }
    let character = spelunky_core::characters::selected(game);
    spelunky_core::characters::apply(game, character);
}
fn tunnel_progress(game: &Game) -> std::collections::BTreeMap<String, u32> {
    let mut progress = donation_ui::progress(game);
    for (stage, maximum) in [(1, 100_000), (2, 200_000)] {
        let key = format!("tunnel{stage}_remaining");
        if game.data.get(&key) == Some(&f64::from(maximum + 1)) {
            progress.insert(key, maximum + 1);
        }
    }
    progress
}
fn editor_preview_context(game: &Game) -> spelunky_core::editor::preview::PreviewContext {
    spelunky_core::editor::preview::PreviewContext {
        city_of_gold: game.flags.contains("city_of_gold") || game.flags.contains("cityOfGold"),
        treasure_vision: game.player.has("Spectacles") || game.player.has("UdjatEye"),
    }
}

fn complete_editor_save(
    editor: &mut EditorUi,
    name: &str,
    result: Result<(), String>,
) -> EditorCommand {
    match result {
        Ok(()) => {
            editor.message(format!("SAVED {name}"));
            editor.saved()
        }
        Err(error) => {
            editor.message(error);
            EditorCommand::None
        }
    }
}

fn prepare_editor_return(game: &mut Game) {
    game.flags.retain(|flag| !flag.starts_with("title_start:"));
    game.flags.insert("title_start:2".into());
}

fn capture_lifetime_stats(game: &Game, data: &mut crate::storage::SaveData) -> bool {
    if data.lifetime_stats == game.lifetime_stats {
        return false;
    }
    data.lifetime_stats = game.lifetime_stats.clone();
    true
}
fn consume_highscores_reset(game: &mut Game, data: &mut crate::storage::SaveData) -> bool {
    if !game.flags.remove("highscores_clear_requested") {
        return false;
    }
    data.records.clear();
    data.challenge_records.clear();
    data.unlocks.retain(|flag| {
        !flag.starts_with("unlock_")
            && !flag.starts_with("trophy_")
            && !flag.starts_with("shortcut_")
    });
    data.tunnel_remaining = std::collections::BTreeMap::from([
        ("tunnel1_remaining".into(), 100_001),
        ("tunnel2_remaining".into(), 200_001),
    ]);
    true
}
fn record_improvements(record: &Record, records: &[Record]) -> std::collections::BTreeSet<String> {
    let previous: Vec<_> = records
        .iter()
        .filter(|old| old.rules == record.rules)
        .collect();
    let mut flags = std::collections::BTreeSet::new();
    for (name, current, best) in [
        (
            "new_money",
            record.score,
            previous.iter().map(|old| old.score).max().unwrap_or(0),
        ),
        (
            "new_kills",
            record.kills,
            previous.iter().map(|old| old.kills).max().unwrap_or(0),
        ),
        (
            "new_saves",
            record.rescues,
            previous.iter().map(|old| old.rescues).max().unwrap_or(0),
        ),
    ] {
        if current > best {
            flags.insert(name.into());
        }
    }
    let best_time = previous
        .iter()
        .filter(|old| old.won && old.fastest_eligible)
        .map(|old| old.elapsed_seconds())
        .min()
        .unwrap_or(0);
    if record.won
        && record.fastest_eligible
        && (best_time == 0 || record.elapsed_seconds() < best_time)
    {
        flags.insert("new_time".into());
    }
    flags
}
fn consume_onboarding(game: &mut Game, data: &mut crate::storage::SaveData) {
    if game.flags.remove("onboarding_consumed") {
        data.first_time = false;
    }
}
fn source_elapsed_ms(game: &Game) -> u64 {
    game.source_elapsed_ms()
}

fn controller_menu_key(button: UiButton, capturing: bool) -> Option<KeyCode> {
    if capturing {
        return None;
    }
    Some(match button {
        UiButton::Up => KeyCode::ArrowUp,
        UiButton::Down => KeyCode::ArrowDown,
        UiButton::Left => KeyCode::ArrowLeft,
        UiButton::Right => KeyCode::ArrowRight,
        UiButton::Confirm => KeyCode::Enter,
        UiButton::Cancel | UiButton::Pause => KeyCode::Escape,
    })
}

fn settings_controller_frame(
    ui: &mut SettingsUi,
    settings: &mut crate::storage::Settings,
    physical: &[crate::controller::Button],
    menu: &mut Vec<UiButton>,
) -> bool {
    let capture = ui.awaiting_controller_binding();
    let awaiting = ui.awaiting_binding();
    let mut changed = false;
    if capture {
        for &button in physical {
            changed |= ui.controller(button, settings) == SettingsCommand::Changed;
        }
    }
    // Controller navigation never masquerades as physical keyboard input.
    // Suppress the completing capture too, even after the wizard closes.
    if awaiting {
        menu.clear();
    }
    changed
}

fn apply_cinematic_start(input: &mut spelunky_core::InputFrame, pending: &mut bool) {
    input.start_pressed |= std::mem::take(pending);
}
fn source_start_key(key: KeyCode, allow_enter: bool) -> bool {
    key == KeyCode::Escape || (allow_enter && key == KeyCode::Enter)
}
fn source_game_over_hint(game: &Game, editor_play: bool, attack: &str) -> String {
    let destination = if editor_play || game.flags.contains("editor_test") {
        "TO EDIT LEVEL."
    } else if game.flags.contains("custom") {
        "TO LOAD ANOTHER LEVEL."
    } else {
        "FOR HIGH SCORES."
    };
    format!("PRESS {attack} {destination}")
}

fn campaign_record(game: &Game) -> Option<Record> {
    if ["custom", "tutorial", "hub"]
        .iter()
        .any(|flag| game.flags.contains(*flag))
    {
        return None;
    }
    let won = game.ending.as_ref().is_some_and(|ending| {
        ending.kind == spelunky_core::ending::EndingKind::Campaign && ending.record_ready
    });
    if game.state != RunState::Dead && !won {
        return None;
    }
    Some(Record {
        seed: game.seed,
        rng_algorithm: game.rng.algorithm(),
        simulation_version: game.simulation_version(),
        rules: game.rules,
        score: game.player.money,
        level: game
            .ending
            .as_ref()
            .map(|ending| ending.completed_level)
            .unwrap_or(game.world.level),
        ticks: game.tick,
        elapsed_ms: Some(source_elapsed_ms(game)),
        fastest_eligible: !game
            .ending
            .as_ref()
            .map(|ending| ending.shortcut_used)
            .unwrap_or_else(|| game.flags.contains("used_shortcut")),
        won,
        kills: game.data.get("kills").copied().unwrap_or(0.).max(0.) as u32,
        rescues: game.data.get("damsels").copied().unwrap_or(0.).max(0.) as u32,
    })
}

fn retry_hub(game: &Game) -> spelunky_core::hub::HubKind {
    use spelunky_core::hub::HubKind;
    if game.flags.contains("hub_sun") {
        HubKind::Sun
    } else if game.flags.contains("hub_moon") {
        HubKind::Moon
    } else if game.flags.contains("hub_stars") {
        HubKind::Stars
    } else if game.flags.contains("hub_scores") {
        HubKind::Scores
    } else {
        HubKind::Title
    }
}

fn prepare_challenge_scores(game: &mut Game) {
    let start = match retry_hub(game) {
        spelunky_core::hub::HubKind::Sun => 1,
        spelunky_core::hub::HubKind::Moon => 2,
        spelunky_core::hub::HubKind::Stars => 3,
        _ => 0,
    };
    game.flags.retain(|flag| !flag.starts_with("scores_start:"));
    game.flags.insert(format!("scores_start:{start}"));
}

fn runtime_level(level: &spelunky_core::level::Level) -> spelunky_core::level::Level {
    // The editor accepts file names as links; source files traditionally use
    // basenames. Resolve both forms without rewriting the stored document.
    let mut runtime_level = level.clone();
    for target in &mut runtime_level.exit_targets {
        *target = runtime_level_name(target);
    }
    runtime_level
}
/// Preserve canonical generation/RNG while withholding music producers from
/// the temporary ordinary room that custom loading immediately discards.
fn new_custom_game(
    seed: u32,
    rules: spelunky_core::RulesPreset,
    session: &SourceMusicState,
) -> Game {
    let mut temporary = session.session_state();
    temporary.enabled = false;
    let mut game = Game::new_with_music(seed, rules, temporary);
    game.source_music = session.session_state();
    game
}
fn load_custom(game: &mut Game, level: &spelunky_core::level::Level, name: &str, previous: &str) {
    game.load_custom(
        &runtime_level(level),
        &runtime_level_name(name),
        &runtime_level_name(previous),
    );
}
fn continue_custom(
    game: &mut Game,
    level: &spelunky_core::level::Level,
    name: &str,
    previous: &str,
) {
    game.continue_custom(
        &runtime_level(level),
        &runtime_level_name(name),
        &runtime_level_name(previous),
    );
}
#[derive(Debug, PartialEq, Eq)]
enum CustomExit {
    Editor,
    Finished,
    Continued,
    Missing(String),
}
fn custom_exit(
    game: &mut Game,
    levels: &std::collections::BTreeMap<String, String>,
    editor_play: bool,
) -> CustomExit {
    // Source oPlayer1 Animation end returns testLevel sessions directly to
    // the editor before ordinary custom-room linking and transition rewards.
    if editor_play {
        return CustomExit::Editor;
    }
    let target = game
        .flags
        .iter()
        .find_map(|flag| flag.strip_prefix("custom_level:"))
        .or_else(|| {
            game.flags
                .iter()
                .find_map(|flag| flag.strip_prefix("default_next_custom:"))
        })
        .unwrap_or("")
        .to_owned();
    if target.is_empty() || target == "NONE" {
        spelunky_core::ending::begin_custom(game);
        return CustomExit::Finished;
    }
    let Some(source) = custom_level_source(levels, &target) else {
        // Source scrLoadLevel treats a missing continuation as the end of an
        // active adventure. Initial imports still report missing/invalid files.
        spelunky_core::ending::begin_custom(game);
        return CustomExit::Finished;
    };
    let Ok(level) = spelunky_core::level::Level::parse(source) else {
        return CustomExit::Missing(target);
    };
    let previous = game
        .ending
        .as_ref()
        .filter(|ending| ending.kind == spelunky_core::ending::EndingKind::Transition)
        .map(|ending| ending.completed_name.clone())
        .unwrap_or_else(|| game.world.name.clone());
    continue_custom(game, &level, &target, &previous);
    CustomExit::Continued
}
fn capture_challenge_records(
    game: &Game,
    records: &mut std::collections::BTreeMap<String, u32>,
) -> bool {
    let mut changed = false;
    if game.flags.contains("hub") {
        for key in ["mini1", "mini2", "mini3"] {
            if let Some(value) = game.data.get(key).filter(|value| value.is_finite()) {
                let score = value.clamp(0., 99.) as u32;
                let best = records.entry(key.into()).or_default();
                changed |= score > *best;
                *best = (*best).max(score);
            }
        }
    }
    changed
}
fn persistent_flag(value: &str) -> bool {
    value.starts_with("shortcut_")
        || value.starts_with("unlock_")
        || value.starts_with("character_")
        || value.starts_with("trophy_")
}
fn truncate(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}
fn wrap_message(value: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in value.split('\n') {
        let mut line = String::new();
        for word in paragraph.split_whitespace() {
            if !line.is_empty() && line.chars().count() + 1 + word.chars().count() > width {
                lines.push(std::mem::take(&mut line));
            }
            if !line.is_empty() {
                line.push(' ');
            }
            for letter in word.chars() {
                if line.chars().count() == width {
                    lines.push(std::mem::take(&mut line));
                }
                line.push(letter);
            }
        }
        lines.push(line);
    }
    if lines.len() > 22 {
        lines.truncate(22);
        lines[21] = "...".into();
    }
    lines
}
fn level_name(value: &str) -> String {
    let name = value.rsplit(['/', '\\']).next().unwrap_or(value);
    if name.to_ascii_lowercase().ends_with(".lvl") {
        name[..name.len() - 4].to_owned()
    } else {
        name.to_owned()
    }
}
fn runtime_level_name(value: &str) -> String {
    level_name(value).to_ascii_lowercase()
}
fn custom_level_source<'a>(
    levels: &'a std::collections::BTreeMap<String, String>,
    target: &str,
) -> Option<&'a str> {
    let key = level_name(target);
    levels
        .get(target)
        .or_else(|| levels.get(&key))
        .or_else(|| {
            levels.iter().find_map(|(name, text)| {
                level_name(name).eq_ignore_ascii_case(&key).then_some(text)
            })
        })
        .map(String::as_str)
}
/// Fold host-provided automatic entropy into the canonical seed domain.
/// This is a portable host policy, not the original runner's QPC randomize.
/// External deterministic seeds use the core's checked constructors instead.
fn fold_seed_entropy(entropy: u128) -> u32 {
    let bytes = entropy.to_le_bytes();
    bytes.chunks_exact(4).fold(0, |seed, word| {
        seed ^ u32::from_le_bytes(word.try_into().unwrap())
    })
}

fn wall_seed() -> u32 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| fold_seed_entropy(d.as_nanos()))
            .unwrap_or(1)
    }
    #[cfg(target_arch = "wasm32")]
    {
        fold_seed_entropy(u128::from(js_sys::Date::now().to_bits()))
    }
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attributes = Window::default_attributes().with_title("Spelunky Classic - Rust");
        #[cfg(not(target_arch = "wasm32"))]
        let attributes = attributes.with_inner_size(winit::dpi::LogicalSize::new(960., 720.));
        #[cfg(target_arch = "wasm32")]
        let attributes = {
            use wasm_bindgen::JsCast;
            use winit::platform::web::WindowAttributesExtWebSys;
            let canvas = web_sys::window()
                .and_then(|w| w.document())
                .and_then(|d| d.get_element_by_id("spelunky-canvas"))
                .and_then(|e| e.dyn_into::<web_sys::HtmlCanvasElement>().ok());
            attributes.with_canvas(canvas).with_prevent_default(true)
        };
        let window = match event_loop.create_window(attributes) {
            Ok(window) => Arc::new(window),
            Err(error) => {
                self.toast(error.to_string());
                event_loop.exit();
                return;
            }
        };
        self.window = Some(window.clone());
        self.timing.reset(self.now());
        self.settings_changed();
        let atlas = match Atlas::load() {
            Ok(atlas) => atlas,
            Err(error) => {
                self.toast(error);
                event_loop.exit();
                return;
            }
        };
        let proxy = self.proxy.clone();
        let task = async move {
            let result = Renderer::new(window, atlas.rgba(), atlas.width, atlas.height)
                .await
                .map(|renderer| (renderer, atlas));
            let _ = proxy.send_event(UserEvent::Ready(result));
        };
        #[cfg(not(target_arch = "wasm32"))]
        pollster::block_on(task);
        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(task);
    }
    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        match event {
            UserEvent::Ready(Ok((mut renderer, atlas))) => {
                renderer.set_vsync(self.store.data.settings.vsync);
                self.renderer = Some(renderer);
                self.atlas = Some(atlas);
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            UserEvent::Ready(Err(error)) => {
                eprintln!("Graphics initialization failed: {error}");
                #[cfg(target_arch = "wasm32")]
                if let Some(status) = web_sys::window()
                    .and_then(|w| w.document())
                    .and_then(|d| d.get_element_by_id("status"))
                {
                    status.set_text_content(Some(&error));
                    let _ = status.remove_attribute("hidden");
                    let _ = status.set_attribute("class", "error");
                }
                event_loop.exit()
            }
            #[cfg(target_arch = "wasm32")]
            UserEvent::Imported(name, text) => self.import_level(name, text),
            #[cfg(target_arch = "wasm32")]
            UserEvent::ImportError(error) => {
                self.editor.message(error.clone());
                self.toast(error)
            }
        }
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        if self.exit_requested {
            self.shutdown(event_loop);
            return;
        }
        match event {
            WindowEvent::CloseRequested => self.shutdown(event_loop),
            WindowEvent::Resized(size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size.width, size.height);
                }
                // A resize changes the editor's inverse projection even if
                // the mouse is stationary. End the old stroke before remapping.
                self.mouse_left = false;
                self.mouse_right = false;
                self.editor.cancel_pointer();
                if let Some([x, y]) = self.cursor_physical {
                    self.cursor = logical_pointer(x, y, size.width, size.height);
                }
            }
            WindowEvent::Focused(focused) => {
                self.focused = focused;
                self.mouse_left = false;
                self.mouse_right = false;
                self.control = false;
                if !focused && self.screen == Screen::Playing {
                    self.screen(Screen::Pause)
                }
                self.reset_timing();
                if !focused {
                    self.input.clear();
                }
                self.audio
                    .pause(!focused || self.occluded || self.screen == Screen::Pause)
            }
            WindowEvent::Occluded(occluded) => {
                self.occluded = occluded;
                if occluded && self.screen == Screen::Playing {
                    self.screen(Screen::Pause);
                }
                self.reset_timing();
                if occluded {
                    self.input.clear();
                }
                self.audio
                    .pause(occluded || !self.focused || self.screen == Screen::Pause);
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.control = modifiers.state().control_key()
            }
            WindowEvent::KeyboardInput {
                event,
                is_synthetic,
                ..
            } => {
                if let PhysicalKey::Code(key) = event.physical_key {
                    let pressed = event.state == ElementState::Pressed;
                    // Winit can recreate held key-downs on focus gain. The
                    // matching focus loss already cleared input and paused;
                    // restoring an old Escape/Confirm must not resume play.
                    if is_synthetic && pressed {
                        return;
                    }
                    if pressed {
                        self.audio.unlock();
                    }
                    self.input.music_key(
                        key,
                        pressed,
                        event.repeat,
                        is_synthetic,
                        source_music_input_active(self.screen, self.game.as_ref()),
                    );
                    if matches!(
                        self.screen,
                        Screen::Playing | Screen::Donation | Screen::Pause
                    ) {
                        self.input.key(key, pressed, event.repeat);
                    }
                    if pressed && !event.repeat {
                        if key == KeyCode::F2 && self.screen == Screen::Title {
                            self.screen(Screen::Editor)
                        } else {
                            self.key(key, event.text.as_deref())
                        }
                        if self.exit_requested {
                            self.shutdown(event_loop);
                            return;
                        }
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_physical = Some([position.x, position.y]);
                if let Some(window) = &self.window {
                    self.cursor = logical_pointer(
                        position.x,
                        position.y,
                        window.inner_size().width,
                        window.inner_size().height,
                    );
                    self.pointer();
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                let pressed = state == ElementState::Pressed;
                if pressed {
                    self.audio.unlock();
                }
                match button {
                    MouseButton::Left => self.mouse_left = pressed,
                    MouseButton::Right => self.mouse_right = pressed,
                    _ => {}
                }
                self.pointer();
            }
            #[cfg(not(target_arch = "wasm32"))]
            WindowEvent::DroppedFile(path) => match std::fs::read_to_string(&path) {
                Ok(text) => self.import_level(path.to_string_lossy().into_owned(), text),
                Err(error) => self.toast(format!("IMPORT FAILED: {error}")),
            },
            WindowEvent::RedrawRequested => {
                let frame_started = self.now();
                #[cfg(target_arch = "wasm32")]
                if !self
                    .presentation_cadence
                    .ready(frame_started, self.store.data.settings.frame_cap)
                {
                    // Sample browser controllers at display cadence even when
                    // presentation is capped. InputState retains press/release
                    // and menu edges until the next rendered update consumes them.
                    if self.focused && !self.occluded {
                        self.input.update_gamepad();
                    }
                    // Leave timing and queued input untouched until presentation
                    // is due. about_to_wait schedules the next browser RAF.
                    return;
                }
                self.update();
                if self.exit_requested {
                    self.shutdown(event_loop);
                    return;
                }
                // Sample once after simulation; scene and diagnostics share this fraction.
                let alpha = self.timing.render_alpha(
                    self.now(),
                    render_clock_running(
                        self.screen,
                        self.focused,
                        self.occluded,
                        self.game.as_ref(),
                    ),
                );
                let quads = self.draw(alpha);
                #[cfg(target_arch = "wasm32")]
                self.publish_test_snapshot(alpha);
                if let Some(window) = &self.window {
                    window.pre_present_notify();
                }
                let result = self
                    .renderer
                    .as_mut()
                    .map(|r| r.render(&quads, [0.005, 0.005, 0.005, 1.]));
                match result {
                    Some(Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated)) => {
                        if let (Some(renderer), Some(window)) = (&mut self.renderer, &self.window) {
                            let size = window.inner_size();
                            renderer.resize(size.width, size.height)
                        }
                    }
                    Some(Err(wgpu::SurfaceError::OutOfMemory)) => event_loop.exit(),
                    Some(Err(_)) => {}
                    _ => {}
                }
                self.next_frame = frame_started
                    + self
                        .store
                        .data
                        .settings
                        .frame_cap
                        .map(|cap| 1. / f64::from(cap))
                        .unwrap_or(0.);
            }
            _ => {}
        }
    }
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        #[cfg(target_arch = "wasm32")]
        {
            // Winit's web request_redraw is requestAnimationFrame. Keep one RAF
            // chain instead of polling browser tasks between display frames.
            event_loop.set_control_flow(ControlFlow::Wait);
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let delay = (self.next_frame - self.now()).max(0.);
            if delay > 0. {
                event_loop.set_control_flow(ControlFlow::WaitUntil(
                    std::time::Instant::now() + std::time::Duration::from_secs_f64(delay),
                ));
            } else {
                event_loop.set_control_flow(ControlFlow::Poll);
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
        }
    }
}
fn offset_fixed_ui(quads: &mut [DrawQuad], origin: [f32; 2]) {
    for quad in quads {
        quad.rect[0] += origin[0];
        quad.rect[1] += origin[1];
    }
}

#[cfg(any(test, target_arch = "wasm32"))]
fn apply_viewport_snapshot(
    snapshot: &mut serde_json::Value,
    game: Option<&Game>,
    viewport: Viewport,
    pixel_snap: bool,
) {
    let [ui_x, ui_y] = viewport.ui_origin();
    snapshot["viewport"] = serde_json::json!({
        "surfaceWidth": viewport.surface_width,
        "surfaceHeight": viewport.surface_height,
        "logicalWidth": viewport.width,
        "logicalHeight": viewport.height,
        "scale": viewport.scale,
        "uiOriginX": ui_x,
        "uiOriginY": ui_y,
    });
    let Some(game) = game else { return };
    let Some(x) = snapshot["camera"]["renderedX"].as_f64() else {
        return;
    };
    let Some(y) = snapshot["camera"]["renderedY"].as_f64() else {
        return;
    };
    let presentation = game.presentation();
    let canonical_camera = spelunky_core::Vec2::new(x, y);
    let camera = if viewport.width != 320. || viewport.height != 240. {
        viewport.camera(
            canonical_camera,
            presentation.world.width as f64 * 16.,
            presentation.world.height as f64 * 16.,
        )
    } else {
        canonical_camera
    };
    snapshot["camera"]["renderedX"] = camera.x.into();
    snapshot["camera"]["renderedY"] = camera.y.into();
    for (coordinate, camera_position, key) in
        [("x", camera.x, "screenX"), ("y", camera.y, "screenY")]
    {
        if let Some(position) = snapshot["rendered"][coordinate].as_f64() {
            let projected = position - camera_position;
            snapshot["rendered"][key] = if pixel_snap {
                projected.round()
            } else {
                projected
            }
            .into();
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn toggle_browser_fullscreen() {
    use wasm_bindgen::{JsCast, JsValue};
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    let (target, method): (JsValue, &str) = if document.fullscreen_element().is_some() {
        (document.clone().into(), "exitFullscreen")
    } else if let Some(root) = document.document_element() {
        (root.into(), "requestFullscreen")
    } else {
        return;
    };
    // web-sys 0.3.91's generated fullscreen methods discard the DOM Promise.
    // Invoke the platform method directly to retain and handle its rejection,
    // without depending on a shell-specific JavaScript global.
    let result = js_sys::Reflect::get(&target, &method.into())
        .and_then(|value| value.dyn_into::<js_sys::Function>())
        .and_then(|function| function.call0(&target));
    match result {
        Ok(value) => {
            if let Ok(promise) = value.dyn_into::<js_sys::Promise>() {
                wasm_bindgen_futures::spawn_local(async move {
                    if wasm_bindgen_futures::JsFuture::from(promise).await.is_err() {
                        browser_fullscreen_failed();
                    }
                    focus_browser_canvas();
                });
            }
        }
        Err(_) => {
            browser_fullscreen_failed();
            focus_browser_canvas();
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn browser_fullscreen_failed() {
    const MESSAGE: &str = "Could not change fullscreen. Use your browser's fullscreen controls.";
    if let Some(status) = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.get_element_by_id("fullscreen-status"))
    {
        status.set_text_content(Some(MESSAGE));
        let _ = status.remove_attribute("hidden");
    }
}

#[cfg(target_arch = "wasm32")]
fn focus_browser_canvas() {
    use wasm_bindgen::JsCast;
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        return;
    };
    if document.has_focus().unwrap_or(false) {
        if let Some(canvas) = document
            .get_element_by_id("spelunky-canvas")
            .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok())
        {
            let _ = canvas.focus();
        }
    }
}

fn logical_pointer(x: f64, y: f64, width: u32, height: u32) -> [f32; 2] {
    Viewport::from_surface(width, height).logical_pointer(x, y)
}
#[cfg(test)]
mod tests {
    #[test]
    fn custom_constructor_starts_only_selected_lush_and_preserves_canonical_rng() {
        let rules = spelunky_core::RulesPreset::Classic;
        let mut session = SourceMusicState::default();
        session.frequencies.insert("musCave".into(), 34_100);
        let mut level = spelunky_core::level::Level::default();
        level.music = "LUSH".into();
        level.set_tile(1, 1, '@').unwrap();
        // Canonical constructors are deliberate: this compares the existing
        // ordinary generation path to the new custom-loading helper.
        let mut previous = Game::new_with_music(42, rules, session.session_state());
        let mut custom = new_custom_game(42, rules, &session);
        assert!(custom.source_music.pending.is_empty());
        assert_eq!(custom.source_music.frequency("musCave"), 34_100);
        assert_eq!(custom.rng.algorithm(), previous.rng.algorithm());
        assert_eq!(custom.rng.state_bits(), previous.rng.state_bits());
        previous.source_music.take_commands();
        load_custom(&mut previous, &level, "lush-fixture", "");
        load_custom(&mut custom, &level, "lush-fixture", "");
        assert_eq!(custom.rng.state_bits(), previous.rng.state_bits());
        let mut actual = custom.rng.clone();
        let mut expected = previous.rng.clone();
        for _ in 0..8 {
            assert_eq!(actual.next_u64(), expected.next_u64());
        }
        assert_eq!(custom.source_music.frequency("musCave"), 34_100);
        assert_eq!(custom.source_music.frequency("musLush"), 44_100);
        let commands = custom.source_music.take_commands();
        assert_eq!(commands.len(), 2);
        assert!(
            matches!(&commands[0], SourceMusicCommand::Play { name, looping: true, if_idle: true } if name == "musLush")
        );
        assert!(
            matches!(&commands[1], SourceMusicCommand::Frequency { name, hz: 44_100 } if name == "musLush")
        );
        assert!(custom.source_music.enabled);
        // Ordinary new-run construction still emits its eligible Cave start.
        let ordinary = Game::new_with_music(42, rules, session.session_state());
        assert!(ordinary.source_music.pending.iter().any(
            |command| matches!(command, SourceMusicCommand::Play { name, .. } if name == "musCave")
        ));
    }
    #[test]
    fn source_music_session_drains_before_replacement_without_replaying_commands() {
        let mut game = Game::legacy(42, spelunky_core::RulesPreset::Classic);
        game.source_music.take_commands();
        game.source_music.enabled = false;
        game.source_music
            .frequencies
            .insert("musTitle".into(), 34_100);
        game.source_music.pending.push(SourceMusicCommand::StopAll);
        game.source_music
            .pending
            .push(SourceMusicCommand::Frequency {
                name: "musTitle".into(),
                hz: 34_100,
            });
        let mut session = SourceMusicState::default();
        let commands = take_source_music(&mut game, &mut session);
        assert!(matches!(
            &commands[..],
            [
                SourceMusicCommand::StopAll,
                SourceMusicCommand::Frequency { hz: 34_100, .. }
            ]
        ));
        assert!(game.source_music.pending.is_empty());
        assert!(!session.enabled && session.pending.is_empty());
        assert_eq!(session.frequencies["musTitle"], 34_100);
        for tutorial in [false, true] {
            let mut replacement = if tutorial {
                Game::tutorial_with_music(
                    42,
                    spelunky_core::RulesPreset::Classic,
                    session.session_state(),
                )
            } else {
                Game::new_with_music(
                    42,
                    spelunky_core::RulesPreset::Classic,
                    session.session_state(),
                )
            };
            assert!(!replacement.source_music.enabled);
            assert_eq!(replacement.source_music.frequencies["musTitle"], 34_100);
            assert!(
                take_source_music(&mut replacement, &mut session).is_empty(),
                "disabled room Create must not start/reset music"
            );
        }
        assert!(SourceMusicState::default().enabled);
    }
    #[test]
    fn music_input_activation_and_f1_stop_reason_are_independent() {
        let mut game = Game::legacy(42, spelunky_core::RulesPreset::Classic);
        assert!(source_music_input_active(Screen::Playing, Some(&game)));
        assert!(!source_music_input_active(Screen::Pause, Some(&game)));
        assert!(!source_music_input_active(Screen::Editor, Some(&game)));
        assert!(!source_music_input_active(Screen::Playing, None));
        game.flags.insert("source_game_inactive".into());
        assert!(!source_music_input_active(Screen::Playing, Some(&game)));
        game.flags.remove("source_game_inactive");
        game.source_music.enabled = false;
        apply_source_suicide(&mut game);
        assert!(!game.source_music.enabled);
        spelunky_core::music::key_pressed(&mut game);
        assert!(game.source_music.enabled);
        assert!(game.flags.contains("source_music_stopped"));
        resume_room_music(&mut game);
        assert!(game.source_music.enabled);
        game.source_music.enabled = false;
        resume_room_music(&mut game);
        assert!(!game.source_music.enabled);
    }
    #[test]
    fn signed_source_frequency_adapter_retains_unsupported_raw_requests() {
        for hz in [0, -100, i64::from(u32::MAX) + 1, i64::MIN] {
            assert_eq!(supported_source_frequency(hz), Err(hz));
        }
        assert_eq!(supported_source_frequency(34_100), Ok(34_100));
        assert_eq!(supported_source_frequency(44_100), Ok(44_100));
        let mut game = Game::legacy_shell(42, spelunky_core::RulesPreset::Classic);
        game.set_source_music_frequency("musCave", -100);
        let mut session = SourceMusicState::default();
        let commands = take_source_music(&mut game, &mut session);
        assert!(matches!(
            commands.last(),
            Some(SourceMusicCommand::Frequency { hz: -100, .. })
        ));
        assert_eq!(session.frequency("musCave"), -100);
    }
    use crate::settings_ui::test_rows::{CONFIGURE_CONTROLLER, FIRST_BINDING};

    fn controller_remap_enter_wizard(ui: &mut SettingsUi, settings: &mut crate::storage::Settings) {
        for _ in 0..CONFIGURE_CONTROLLER {
            ui.key(KeyCode::ArrowDown, settings);
        }
        ui.key(KeyCode::Enter, settings);
        assert!(ui.awaiting_controller_binding());
    }

    #[test]
    fn controller_remap_host_suppresses_later_menu_edges_in_the_capture_entry_frame() {
        for row in [FIRST_BINDING, CONFIGURE_CONTROLLER] {
            let mut ui = SettingsUi::new();
            let mut settings = crate::storage::Settings::default();
            for _ in 0..row {
                ui.key(KeyCode::ArrowDown, &mut settings);
            }
            let keyboard = settings.bindings.clone();
            let controller = settings.controller_bindings.clone();
            for button in [
                UiButton::Confirm,
                UiButton::Cancel,
                UiButton::Up,
                UiButton::Confirm,
            ] {
                if let Some(key) = controller_menu_key(button, ui.awaiting_binding()) {
                    ui.key(key, &mut settings);
                }
            }
            assert!(ui.awaiting_binding());
            assert_eq!(settings.bindings, keyboard);
            assert_eq!(settings.controller_bindings, controller);
            if row == CONFIGURE_CONTROLLER {
                ui.controller(crate::controller::Button::East, &mut settings);
                assert_eq!(
                    settings.controller_bindings["jump"], "East",
                    "Cancel skipped Jump"
                );
            } else {
                ui.key(KeyCode::KeyJ, &mut settings);
                assert_eq!(settings.bindings["left"], "KeyJ");
            }
        }
    }

    #[test]
    fn controller_remap_host_keeps_ordinary_menu_navigation_unchanged() {
        let mut ui = SettingsUi::new();
        let mut settings = crate::storage::Settings::default();
        let before = serde_json::to_value(&settings).unwrap();
        let original = vec![
            UiButton::Up,
            UiButton::Down,
            UiButton::Left,
            UiButton::Right,
            UiButton::Confirm,
            UiButton::Cancel,
            UiButton::Pause,
        ];
        let mut menu = original.clone();
        assert!(!settings_controller_frame(
            &mut ui,
            &mut settings,
            &[crate::controller::Button::South],
            &mut menu
        ));
        assert_eq!(menu, original);
        assert_eq!(serde_json::to_value(&settings).unwrap(), before);
    }

    #[test]
    fn controller_remap_host_does_not_convert_controller_menu_edges_to_keyboard_bindings() {
        let mut ui = SettingsUi::new();
        let mut settings = crate::storage::Settings::default();
        for _ in 0..FIRST_BINDING {
            ui.key(KeyCode::ArrowDown, &mut settings);
        }
        ui.key(KeyCode::Enter, &mut settings);
        assert!(ui.awaiting_binding() && !ui.awaiting_controller_binding());
        let before = settings.bindings.clone();
        let mut menu = vec![UiButton::Confirm, UiButton::Up, UiButton::Cancel];
        assert!(!settings_controller_frame(
            &mut ui,
            &mut settings,
            &[crate::controller::Button::West],
            &mut menu
        ));
        assert!(menu.is_empty());
        assert_eq!(settings.bindings, before);
        assert!(ui.awaiting_binding());
        assert_eq!(
            ui.key(KeyCode::KeyJ, &mut settings),
            SettingsCommand::Changed
        );
        assert_eq!(settings.bindings["left"], "KeyJ");
    }

    #[test]
    fn controller_remap_host_consumes_capturing_and_completing_menu_edges_and_persists() {
        let mut ui = SettingsUi::new();
        let mut settings = crate::storage::Settings::default();
        let keyboard = settings.bindings.clone();
        controller_remap_enter_wizard(&mut ui, &mut settings);
        let mut menu = vec![UiButton::Cancel];
        assert!(settings_controller_frame(
            &mut ui,
            &mut settings,
            &[crate::controller::Button::East],
            &mut menu
        ));
        assert!(menu.is_empty());
        assert_eq!(settings.controller_bindings["jump"], "East");
        for _ in 1..8 {
            ui.key(KeyCode::Escape, &mut settings);
        }
        let mut menu = vec![UiButton::Pause, UiButton::Confirm];
        assert!(settings_controller_frame(
            &mut ui,
            &mut settings,
            &[crate::controller::Button::Mode],
            &mut menu
        ));
        assert!(!ui.awaiting_binding());
        assert!(menu.is_empty(), "completion leaked a menu activation");
        assert_eq!(settings.controller_bindings["start"], "Mode");
        assert_eq!(settings.bindings, keyboard);
        let mut data = crate::storage::SaveData::default();
        data.settings = settings;
        let decoded = crate::storage::SaveData::from_json(&data.to_json().unwrap()).unwrap();
        assert_eq!(decoded.settings.controller_bindings["jump"], "East");
        assert_eq!(decoded.settings.controller_bindings["start"], "Mode");
        assert_eq!(decoded.settings.bindings, keyboard);
    }

    #[test]
    fn controller_remap_host_does_not_capture_the_button_that_opened_the_wizard() {
        let mut ui = SettingsUi::new();
        let mut settings = crate::storage::Settings::default();
        for _ in 0..CONFIGURE_CONTROLLER {
            ui.key(KeyCode::ArrowDown, &mut settings);
        }
        let before = settings.controller_bindings.clone();
        let mut menu = vec![UiButton::Confirm];
        assert!(!settings_controller_frame(
            &mut ui,
            &mut settings,
            &[crate::controller::Button::South],
            &mut menu
        ));
        assert_eq!(menu, vec![UiButton::Confirm]);
        // This is the same ordinary Confirm-to-Enter path used by the host.
        ui.key(KeyCode::Enter, &mut settings);
        assert!(ui.awaiting_controller_binding());
        let mut menu = vec![];
        assert!(!settings_controller_frame(
            &mut ui,
            &mut settings,
            &[],
            &mut menu
        ));
        assert_eq!(settings.controller_bindings, before);
        assert!(ui.awaiting_controller_binding());
    }

    #[test]
    fn ordinary_fatal_collision_stops_music_at_player_death_step_but_ankh_keeps_it_playing() {
        for has_ankh in [false, true] {
            let mut game = Game::shell(123, spelunky_core::RulesPreset::Classic);
            game.world = spelunky_core::World::new(40, 30, Area::Mines, 1);
            for x in 0..40 {
                game.world.set_tile(x, 10, "oBlock");
            }
            game.world.entrance = spelunky_core::Vec2::new(40., 152.);
            game.player.pos = spelunky_core::Vec2::new(88., 152.);
            game.player.previous_pos = game.player.pos;
            game.player.grounded = true;
            game.player.state = spelunky_core::PlayerState::Standing;
            if has_ankh {
                game.player.equipment.insert("Ankh".into());
            }
            game.world
                .spawn("oBullet", spelunky_core::Vec2::new(88., 149.));
            assert_eq!(game_music(&game), "cave");
            assert!(!stop_music_immediately(&game, game_music(&game)));
            game.step(&spelunky_core::InputFrame::default());
            assert_eq!(game.player.health, 0);
            assert_eq!(game.state, RunState::Playing);
            assert_eq!(game_music(&game), "cave");
            assert!(
                !stop_music_immediately(&game, game_music(&game)),
                "The collision occurs after Player Step; the independent dead flag is still clear"
            );
            game.step(&spelunky_core::InputFrame::default());
            if has_ankh {
                assert_eq!(game.state, RunState::Playing);
                assert_eq!(game.player.health, 4);
                assert!(!game.player.has("Ankh"));
                assert_eq!(game_music(&game), "cave");
                assert!(!stop_music_immediately(&game, game_music(&game)));
                assert!(
                    !game
                        .events
                        .iter()
                        .any(|event| matches!(event, GameEvent::Died))
                );
            } else {
                assert_eq!(game.state, RunState::Dead);
                assert_eq!(game_music(&game), "");
                assert!(stop_music_immediately(&game, game_music(&game)));
                assert!(
                    game.events
                        .iter()
                        .any(|event| matches!(event, GameEvent::Died))
                );
            }
        }
    }
    #[test]
    fn source_custom_selector_return_clears_session_markers_and_loads_title_start_two() {
        let mut game = Game::shell(7, spelunky_core::RulesPreset::Classic);
        assert!(!prepare_source_selector_return(&mut game));
        game.flags.extend(
            [
                "source_load_level_menu",
                "source_music_stopped",
                "title_start:0",
            ]
            .map(str::to_owned),
        );
        assert!(prepare_source_selector_return(&mut game));
        assert!(!game.flags.contains("source_load_level_menu"));
        assert!(!game.flags.contains("source_music_stopped"));
        assert!(!game.flags.contains("title_start:0"));
        assert!(game.flags.contains("title_start:2"));
        game.load_hub(spelunky_core::hub::HubKind::Title);
        assert_eq!(game.world.name, "rTitle");
        assert!(game.opening.is_none());
        assert_eq!(game_music(&game), "title");
    }
    #[test]
    fn source_paused_bomb_consumes_rope_and_start_even_when_life_or_room_blocks_suicide() {
        let mut game = Game::shell(7, spelunky_core::RulesPreset::Classic);
        let all = spelunky_core::InputFrame {
            bomb_pressed: true,
            rope_pressed: true,
            ..Default::default()
        };
        assert_eq!(
            source_paused_control(Some(&game), &all, true),
            Some(SourcePausedControl::Suicide)
        );
        game.player.health = 0;
        assert_eq!(
            source_paused_control(Some(&game), &all, true),
            Some(SourcePausedControl::Blocked)
        );
        game.player.health = 4;
        for room in [
            "rTitle",
            "rHighscores",
            "rSunRoom",
            "rMoonRoom",
            "rStarsRoom",
            "rIntro",
            "rTransition1",
            "rEnd",
        ] {
            game.world.name = room.into();
            assert_eq!(
                source_paused_control(Some(&game), &all, true),
                Some(SourcePausedControl::Blocked),
                "{room}"
            );
        }
        for room in [
            "rTutorial",
            "rLoadLevel",
            "rLevel",
            "rLevel2",
            "rLevel3",
            "rOlmec",
        ] {
            game.world.name = room.into();
            assert_eq!(
                source_paused_control(Some(&game), &all, true),
                Some(SourcePausedControl::Suicide),
                "{room}"
            );
        }
        assert_eq!(
            source_paused_control(None, &all, true),
            Some(SourcePausedControl::Blocked)
        );
        let rope = spelunky_core::InputFrame {
            rope_pressed: true,
            ..Default::default()
        };
        assert_eq!(
            source_paused_control(None, &rope, true),
            Some(SourcePausedControl::Quit)
        );
        assert_eq!(
            source_paused_control(None, &Default::default(), true),
            Some(SourcePausedControl::Resume)
        );
        assert_eq!(
            source_paused_control(None, &Default::default(), false),
            None
        );
    }
    #[test]
    fn source_paused_bomb_ankh_revival_keeps_music_but_preserves_an_existing_stop() {
        for already_stopped in [false, true] {
            let mut game = Game::shell(7, spelunky_core::RulesPreset::Classic);
            game.player.equipment.insert("Ankh".into());
            game.player.invincible = 77;
            if already_stopped {
                game.flags.insert("source_music_stopped".into());
            }
            assert!(!apply_source_pause_suicide(&mut game));
            assert_eq!(game.player.health, -99);
            assert_eq!(game.player.invincible, 77);
            assert!(game.player.has("Ankh"));
            assert!(game.events.is_empty());
            let bombs = game.player.bombs;
            game.step(&spelunky_core::InputFrame {
                bomb_pressed: true,
                ..Default::default()
            });
            assert_eq!(
                game.player.bombs, bombs,
                "nonpositive life blocks the player Bomb action before Ankh revival"
            );
            assert_eq!(game.state, RunState::Playing);
            assert_eq!(game.player.health, 4);
            assert!(!game.player.has("Ankh"));
            assert_eq!(game_music(&game), if already_stopped { "" } else { "cave" });
            assert!(
                !game
                    .events
                    .iter()
                    .any(|event| matches!(event, GameEvent::Damage { .. } | GameEvent::Died))
            );
        }
        let mut game = Game::shell(7, spelunky_core::RulesPreset::Classic);
        assert!(apply_source_pause_suicide(&mut game));
        assert!(game.flags.contains("source_music_stopped"));
        game.step(&Default::default());
        assert_eq!(game.state, RunState::Dead);
        assert_eq!(game.lifetime_stats.level_deaths, [0; 16]);
    }
    #[test]
    fn source_paused_bound_actions_keep_validated_aliases_and_require_a_fresh_edge() {
        let mut input = InputState::default();
        let mut bindings = std::collections::BTreeMap::new();
        bindings.insert("bomb".into(), "Enter,KeyQ".into());
        bindings.insert("rope".into(), "Enter".into());
        input.set_bindings(&bindings);
        assert!(input.action_key("bomb", KeyCode::Enter));
        input.key(KeyCode::Enter, true, false);
        let game = Game::shell(7, spelunky_core::RulesPreset::Classic);
        assert_eq!(
            source_paused_control(Some(&game), &input.take_tick(), true),
            Some(SourcePausedControl::Suicide)
        );
        assert_eq!(
            source_paused_control(Some(&game), &input.take_tick(), false),
            None
        );
        input.key(KeyCode::Enter, true, true);
        assert_eq!(
            source_paused_control(Some(&game), &input.take_tick(), false),
            None
        );
        input.key(KeyCode::Enter, false, false);
        input.take_tick();
        input.key(KeyCode::Enter, true, false);
        assert_eq!(
            source_paused_control(Some(&game), &input.take_tick(), false),
            Some(SourcePausedControl::Suicide)
        );
        bindings.insert("bomb".into(), "invalid".into());
        bindings.insert("rope".into(), "".into());
        input.set_bindings(&bindings);
        assert!(input.action_key("bomb", KeyCode::KeyB));
        assert!(!input.action_key("bomb", KeyCode::Enter));
        assert!(!input.action_key("rope", KeyCode::KeyR));
    }
    #[test]
    fn source_f1_requires_pause_life_and_an_active_level_while_f10_only_requires_pause() {
        let mut game = Game::shell(7, spelunky_core::RulesPreset::Classic);
        assert_eq!(
            source_host_control(Screen::Playing, Some(&game), KeyCode::F1),
            None
        );
        assert_eq!(
            source_host_control(Screen::Pause, Some(&game), KeyCode::F1),
            Some(SourceHostControl::Suicide)
        );
        game.player.health = 0;
        assert_eq!(
            source_host_control(Screen::Pause, Some(&game), KeyCode::F1),
            None
        );
        assert_eq!(
            source_host_control(Screen::Pause, Some(&game), KeyCode::F10),
            Some(SourceHostControl::Quit)
        );
        assert_eq!(
            source_host_control(Screen::Playing, Some(&game), KeyCode::F10),
            None
        );
        game.load_hub(spelunky_core::hub::HubKind::Title);
        assert_eq!(
            source_host_control(Screen::Pause, Some(&game), KeyCode::F1),
            None
        );
        assert_eq!(
            source_host_control(Screen::Pause, Some(&game), KeyCode::F10),
            Some(SourceHostControl::Quit)
        );
    }
    #[test]
    fn source_title_function_keys_exist_during_title_opening_but_not_intro_or_pause() {
        let mut game = Game::shell(7, spelunky_core::RulesPreset::Classic);
        spelunky_core::opening::begin_intro(&mut game);
        for key in [KeyCode::F2, KeyCode::F3] {
            assert_eq!(source_host_control(Screen::Playing, Some(&game), key), None);
        }
        game.flags.insert("title_start:0".into());
        game.load_hub(spelunky_core::hub::HubKind::Title);
        assert!(game.opening.is_some());
        assert_eq!(
            source_host_control(Screen::Playing, Some(&game), KeyCode::F2),
            Some(SourceHostControl::Editor)
        );
        assert_eq!(
            source_host_control(Screen::Playing, Some(&game), KeyCode::F3),
            Some(SourceHostControl::CustomLevels)
        );
        for key in [KeyCode::F2, KeyCode::F3] {
            assert_eq!(source_host_control(Screen::Pause, Some(&game), key), None);
        }
    }
    #[test]
    fn source_f1_sets_raw_health_and_knockback_before_ordinary_ankh_processing() {
        for facing in [-1, 1] {
            let mut game = Game::shell(7, spelunky_core::RulesPreset::Classic);
            game.player.facing = facing;
            game.player.invincible = 77;
            game.player.equipment.insert("Ankh".into());
            apply_source_suicide(&mut game);
            assert_eq!(game.player.health, -99);
            assert_eq!(
                game.player.vel,
                spelunky_core::Vec2::new(f64::from(facing) * 3., -6.)
            );
            assert_eq!(game.player.invincible, 77);
            assert!(game.player.has("Ankh"));
            assert_eq!(game.state, RunState::Playing);
            assert!(game.events.is_empty());
            assert_eq!(game_music(&game), "");
            game.step(&spelunky_core::InputFrame::default());
            assert_eq!(game.player.health, 4);
            assert!(!game.player.has("Ankh"));
            assert_eq!(game.state, RunState::Playing);
            assert!(
                game.events.iter().any(
                    |event| matches!(event, GameEvent::Sound {name,..} if name == "sndTeleport")
                )
            );
            assert!(
                !game
                    .events
                    .iter()
                    .any(|event| matches!(event, GameEvent::Damage { .. } | GameEvent::Died))
            );
            assert_eq!(
                game_music(&game),
                "",
                "Ankh revival does not restart F1-stopped music"
            );
            assert_eq!(game.lifetime_stats.level_deaths, [0; 16]);
        }
    }
    #[test]
    fn source_f1_ordinary_death_keeps_level_death_for_later_acknowledgment() {
        let mut game = Game::shell(7, spelunky_core::RulesPreset::Classic);
        apply_source_suicide(&mut game);
        game.step(&spelunky_core::InputFrame::default());
        assert_eq!(game.state, RunState::Dead);
        assert_eq!(game.player.health, 0);
        assert!(
            game.events
                .iter()
                .any(|event| matches!(event, GameEvent::Died))
        );
        assert!(
            !game
                .events
                .iter()
                .any(|event| matches!(event, GameEvent::Damage { .. }))
        );
        assert_eq!(game.lifetime_stats.level_deaths, [0; 16]);
    }
    #[test]
    fn source_f1_stopped_music_resumes_only_after_a_later_room_entry() {
        let mut game = Game::shell(7, spelunky_core::RulesPreset::Classic);
        game.player.equipment.insert("Ankh".into());
        apply_source_suicide(&mut game);
        game.step(&spelunky_core::InputFrame::default());
        assert_eq!(game_music(&game), "");
        game.next_level();
        resume_room_music(&mut game);
        assert_eq!(game_music(&game), "cave");
        assert!(!game.flags.contains("source_music_stopped"));
    }
    #[test]
    fn browser_presentation_caps_preserve_actual_core_tick_and_rng_traces() {
        let fixture = camera_follow_fixture();
        let mut expected = fixture.clone();
        let trace = |game: &Game| {
            let mut rng = game.rng.clone();
            (
                game.state_hash(),
                game.presentation().state_hash(),
                [rng.next_u64(), rng.next_u64(), rng.next_u64()],
                game.events.clone(),
            )
        };
        let input_at = |tick| spelunky_core::InputFrame {
            right: tick < 30,
            left: tick >= 30,
            jump: (5..10).contains(&tick),
            jump_pressed: tick == 5,
            jump_released: tick == 10,
            ..Default::default()
        };
        let mut expected_traces = Vec::new();
        for tick in 0..60 {
            expected.step(&input_at(tick));
            expected_traces.push(trace(&expected));
        }
        for refresh in [30, 60, 120, 144, 240] {
            for cap in [None, Some(30), Some(60), Some(120), Some(144), Some(240)] {
                let mut cadence = crate::timing::PresentationCadence::default();
                let mut timing = HostTiming::default();
                timing.reset(0.);
                let mut game = fixture.clone();
                let mut tick = 0;
                for callback in 0..=refresh * 2 {
                    let now = f64::from(callback) / f64::from(refresh);
                    if !cadence.ready(now, cap) {
                        continue;
                    }
                    let frame = timing.begin_frame(now);
                    for _ in 0..timing.advance(frame).ticks {
                        game.step(&input_at(tick));
                        assert_eq!(
                            trace(&game),
                            expected_traces[tick],
                            "{refresh}Hz RAF, cap {cap:?}, tick {tick}"
                        );
                        tick += 1;
                    }
                }
                assert_eq!(tick, 60, "{refresh}Hz RAF, cap {cap:?}");
                assert_eq!(game.state_hash(), expected.state_hash());
                assert_eq!(game.rng.clone().next_u64(), expected.rng.clone().next_u64());
                assert_eq!(
                    game.presentation().state_hash(),
                    expected.presentation().state_hash()
                );
            }
        }
    }

    #[test]
    fn presentation_rates_draw_fractional_motion_with_authoritative_sprite_phase() {
        use spelunky_core::{InputFrame, PlayerState, Vec2};

        let atlas = Atlas::load().unwrap();
        let mut fixture = camera_follow_fixture();
        // A normal walking interval below the horizontal camera-follow border.
        // Initial setup only: all later endpoints come from actual Game::step.
        fixture.player.pos = Vec2::new(88., 472.);
        fixture.player.previous_pos = fixture.player.pos;
        fixture.player.vel = Vec2::default();
        fixture.player.state = PlayerState::Standing;
        fixture.player.grounded = true;
        fixture.player.sprite = "sStandLeft".into();
        fixture.camera = Vec2::new(0., 328.);
        fixture.previous_camera = fixture.camera;
        let input = InputFrame {
            right: true,
            ..Default::default()
        };
        // Classic waits for more than two held-direction ticks to accelerate.
        // Reach ordinary walking through input, without forcing its counters.
        for _ in 0..2 {
            fixture.step(&input);
        }
        let trace = |game: &Game| {
            let mut rng = game.rng.clone();
            (
                game.state_hash(),
                game.presentation().state_hash(),
                [rng.next_u64(), rng.next_u64(), rng.next_u64()],
                game.events.clone(),
            )
        };
        let mut reference = fixture.clone();
        let expected: Vec<_> = (0..4)
            .map(|_| {
                reference.step(&input);
                trace(&reference)
            })
            .collect();

        for rate in [30, 60, 120, 240] {
            let slots = rate / spelunky_core::TICK_RATE;
            let mut timing = HostTiming::default();
            timing.reset(0.);
            let mut game = fixture.clone();
            let mut tick = 0;
            let mut motion = vec![Vec::new(); 4];
            let mut distances = [0.; 4];
            let mut poses = vec![None; 4];
            // Include the startup interval and four complete interpolation
            // intervals. Integer callback slots define the expected phases;
            // floating clock arithmetic is checked with a tolerance.
            for callback in 0..5 * slots {
                let now = f64::from(callback) / f64::from(rate);
                let frame = timing.begin_frame(now);
                for _ in 0..timing.advance(frame).ticks {
                    game.step(&input);
                    assert_eq!(trace(&game), expected[tick], "{rate}Hz tick {tick}");
                    tick += 1;
                    timing.history_ready = true;
                }
                assert_eq!(tick, (callback / slots) as usize, "{rate}Hz callback");
                if tick == 0 {
                    assert_eq!(timing.alpha(), 1., "startup has no old room history");
                    continue;
                }
                let alpha = timing.render_alpha(now, true);
                let phase = f64::from(callback % slots) / f64::from(slots);
                assert!((alpha - phase).abs() < 1e-8, "{rate}Hz phase {phase}");
                let before_draw = trace(&game);
                let drawn = game.presentation();
                assert_eq!(drawn.previous_camera.x, 0.);
                assert_eq!(drawn.camera.x, 0.);
                let distance = drawn.player.pos.x - drawn.player.previous_pos.x;
                assert!(distance > 0., "ordinary walking must move at {rate}Hz");
                let anchor = drawn_player_anchor(&game, alpha, &atlas);
                let expected_x = drawn.player.previous_pos.x + distance * phase;
                assert!((anchor[0] - expected_x).abs() < 1e-4, "{rate}Hz {phase}");
                let pose = (drawn.player.sprite.clone(), drawn.player.animation);
                let slot = tick - 1;
                if let Some(first) = &poses[slot] {
                    assert_eq!(&pose, first, "rendering must retain authored sprite phase");
                } else {
                    poses[slot] = Some(pose);
                }
                motion[slot].push(anchor[0]);
                distances[slot] = distance;
                assert_eq!(trace(&game), before_draw, "actual scene is read-only");
            }
            assert_eq!(tick, 4);
            for (positions, distance) in motion.iter().zip(distances) {
                assert_eq!(positions.len(), slots as usize, "{rate}Hz sample count");
                for pair in positions.windows(2) {
                    assert!(
                        pair[1] > pair[0],
                        "{rate}Hz must draw distinct motion samples"
                    );
                    assert!(
                        (pair[1] - pair[0] - distance / f64::from(slots)).abs() < 1e-4,
                        "{rate}Hz motion must use evenly spaced interpolation"
                    );
                }
            }
            assert_eq!(trace(&game), expected[3]);
        }
    }

    #[test]
    fn skipped_browser_raf_keeps_keyboard_tap_edges_until_the_next_tick() {
        let mut cadence = crate::timing::PresentationCadence::default();
        let mut timing = HostTiming::default();
        timing.reset(0.);
        let mut input = InputState::default();
        assert!(cadence.ready(0., Some(30)));
        let frame = timing.begin_frame(0.);
        assert_eq!(timing.advance(frame).ticks, 0);
        input.key(KeyCode::KeyZ, true, false);
        input.key(KeyCode::KeyZ, false, false);
        input.key(KeyCode::KeyB, true, false);
        input.key(KeyCode::KeyB, false, false);
        for callback in 1..8 {
            assert!(!cadence.ready(f64::from(callback) / 240., Some(30)));
        }
        assert!(cadence.ready(1. / 30., Some(30)));
        let frame = timing.begin_frame(1. / 30.);
        assert_eq!(timing.advance(frame).ticks, 1);
        let tick = input.take_tick();
        assert!(tick.jump_pressed && tick.jump_released && !tick.jump);
        assert!(tick.bomb_pressed);
        let next = input.take_tick();
        assert!(!next.jump_pressed && !next.jump_released && !next.bomb_pressed);
    }

    #[test]
    fn automatic_seed_entropy_folds_all_words_without_float_saturation() {
        assert_eq!(fold_seed_entropy(0), 0);
        assert_eq!(fold_seed_entropy(u128::from(u32::MAX)), u32::MAX);
        for word in 0..4 {
            assert_eq!(
                fold_seed_entropy(0x1234_5678_u128 << (word * 32)),
                0x1234_5678
            );
        }
        assert_eq!(fold_seed_entropy(u128::MAX), 0);
        let early = fold_seed_entropy(u128::from(1.0_f64.to_bits()));
        let later = fold_seed_entropy(u128::from(2.0_f64.to_bits()));
        assert_ne!(early, later);
        let seed = u32::MAX ^ later;
        let game = Game::shell(seed, spelunky_core::RulesPreset::Classic);
        assert_eq!(game.seed, u64::from(seed));
        assert_eq!(
            game.rng.algorithm(),
            spelunky_core::RngAlgorithm::Gm8Lcg32V1
        );
    }

    #[test]
    fn new_record_captures_actual_rng_identity_and_simulation_version() {
        use spelunky_core::{RngAlgorithm, RulesPreset};
        let mut canonical = Game::shell(u32::MAX, RulesPreset::Classic);
        canonical.state = RunState::Dead;
        canonical.player.money = 12345;
        canonical.data.insert("time".into(), 27000.);
        let record = campaign_record(&canonical).unwrap();
        assert_eq!(record.seed, u64::from(u32::MAX));
        assert_eq!(record.rng_algorithm, RngAlgorithm::Gm8Lcg32V1);
        assert_eq!(record.simulation_version, spelunky_core::SIMULATION_VERSION);
        assert_eq!(record.score, 12345);
        assert_eq!(record.elapsed_ms(), 27000);
        assert!(record.fastest_eligible);

        let mut legacy = Game::legacy(u64::MAX, RulesPreset::Classic);
        legacy.state = RunState::Dead;
        let record = campaign_record(&legacy).unwrap();
        assert_eq!(record.seed, u64::MAX);
        assert_eq!(record.rng_algorithm, RngAlgorithm::LegacySplitMix64V1);
        assert_eq!(
            record.simulation_version,
            spelunky_core::HISTORICAL_SIMULATION_VERSION
        );
        assert!(record.fastest_eligible);
    }

    #[test]
    fn render_alpha_eight_ms_work_is_read_only_for_actual_core_traces() {
        for rate in [60, 120, 144, 240] {
            let mut immediate = HostTiming::default();
            let mut busy = HostTiming::default();
            immediate.reset(10.);
            busy.reset(10.);
            let mut first = camera_follow_fixture();
            let mut second = first.clone();
            let mut count = 0;
            for frame in 1..=rate {
                let now = 10. + f64::from(frame) / f64::from(rate);
                let first_frame = immediate.begin_frame(now);
                let second_frame = busy.begin_frame(now);
                let ticks = immediate.advance(first_frame).ticks;
                assert_eq!(busy.advance(second_frame).ticks, ticks);
                for _ in 0..ticks {
                    let input = spelunky_core::InputFrame {
                        right: count < 15,
                        left: count >= 15,
                        jump: (5..10).contains(&count),
                        jump_pressed: count == 5,
                        jump_released: count == 10,
                        ..Default::default()
                    };
                    first.step(&input);
                    second.step(&input);
                    immediate.history_ready = true;
                    busy.history_ready = true;
                    count += 1;
                    assert_eq!(first.state_hash(), second.state_hash());
                    assert_eq!(first.rng.clone().next_u64(), second.rng.clone().next_u64());
                    assert_eq!(
                        serde_json::to_value(&first.events).unwrap(),
                        serde_json::to_value(&second.events).unwrap()
                    );
                    assert_eq!(
                        first.presentation().state_hash(),
                        second.presentation().state_hash()
                    );
                }
                let before = second.state_hash();
                let base = immediate.render_alpha(now, true);
                let corrected = busy.render_alpha(now + if ticks > 0 { 0.008 } else { 0. }, true);
                assert!(corrected >= base);
                assert!(corrected <= 1.);
                assert_eq!(second.state_hash(), before, "render sampling changed core");
                // Sampling twice must not consume time or create another core tick.
                assert_eq!(
                    busy.render_alpha(now + if ticks > 0 { 0.008 } else { 0. }, true),
                    corrected
                );
            }
            assert_eq!(count, 30, "rate {rate}");
        }
    }

    #[test]
    fn render_alpha_work_and_next_frame_share_one_phase_without_double_counting() {
        let atlas = Atlas::load().unwrap();
        let mut game = camera_follow_fixture();
        let mut timing = HostTiming::default();
        timing.reset(10.);
        let boundary = 10. + 1. / 30.;
        let frame = timing.begin_frame(boundary);
        assert_eq!(timing.advance(frame).ticks, 1);
        game.step(&spelunky_core::InputFrame {
            right: true,
            ..Default::default()
        });
        timing.history_ready = true;
        let alpha = timing.render_alpha(boundary + 0.008, true);
        assert!((alpha - 0.24).abs() < 1e-10);
        assert!(
            timing.alpha() < 1e-10,
            "read-only sample advanced accumulator"
        );
        let hash = game.state_hash();
        let snapshot = test_snapshot(Screen::Playing, Some(&game), alpha, false);
        let drawn = game.presentation();
        let expected_x =
            drawn.player.previous_pos.x + (drawn.player.pos.x - drawn.player.previous_pos.x) * 0.24;
        assert!((snapshot["rendered"]["x"].as_f64().unwrap() - expected_x).abs() < 1e-9);
        let anchor = drawn_player_anchor(&game, alpha, &atlas);
        assert!((snapshot["rendered"]["screenX"].as_f64().unwrap() - anchor[0]).abs() < 1e-4);
        assert_eq!(game.state_hash(), hash);

        // First subsequent 240Hz opportunity after an 8ms blocking update.
        let next = boundary + 3. / 240.;
        let frame = timing.begin_frame(next);
        assert_eq!(timing.advance(frame).ticks, 0);
        assert!((timing.render_alpha(next, true) - 0.375).abs() < 1e-10);
        assert!((timing.render_alpha(next + 0.001, true) - 0.405).abs() < 1e-10);
        let end = timing.render_alpha(boundary + 1. / 30., true);
        assert!((end - 1.).abs() < 1e-10);
        let frame = timing.begin_frame(boundary + 1. / 30.);
        assert_eq!(timing.advance(frame).ticks, 1);
        assert!(timing.render_alpha(boundary + 1. / 30., true) < 1e-10);
        let previous_end = test_snapshot(Screen::Playing, Some(&game), 1., false);
        game.step(&spelunky_core::InputFrame {
            right: true,
            ..Default::default()
        });
        let next_start = test_snapshot(
            Screen::Playing,
            Some(&game),
            timing.render_alpha(boundary + 1. / 30., true),
            false,
        );
        assert!(
            (previous_end["rendered"]["x"].as_f64().unwrap()
                - next_start["rendered"]["x"].as_f64().unwrap())
            .abs()
                < 1e-9
        );
        assert!(
            (previous_end["camera"]["renderedX"].as_f64().unwrap()
                - next_start["camera"]["renderedX"].as_f64().unwrap())
            .abs()
                < 1e-9
        );
        // New endpoints meet the previous endpoint; no extrapolation when late.
        assert_eq!(timing.render_alpha(boundary + 1., true), 1.);
    }

    #[test]
    fn render_alpha_preserves_reset_history_and_rejects_invalid_sample_times() {
        let mut timing = HostTiming::default();
        timing.reset(10.);
        let frame = timing.begin_frame(10. + 1. / 30.);
        assert_eq!(timing.advance(frame).ticks, 1);
        assert_eq!(timing.render_alpha(20., true), 1., "no room history yet");
        timing.history_ready = true;
        let base = timing.alpha();
        for now in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 10.] {
            assert_eq!(timing.render_alpha(now, true), base);
        }
        assert_eq!(timing.render_alpha(20., false), base);
        let old_frame = timing.begin_frame(20.);
        timing.reset(20.);
        assert_eq!(timing.advance(old_frame).ticks, 0);
        assert_eq!(timing.render_alpha(20.008, true), 1.);
        assert!(timing.alpha_sample.is_none(), "old generation was retained");
        // Even a later history mark cannot resurrect an earlier room's sample.
        timing.history_ready = true;
        assert_eq!(timing.render_alpha(20.008, true), timing.alpha());
        timing.reset(20.);
        for frame in 1..8 {
            let now = 20. + f64::from(frame) / 240.;
            let sample = timing.begin_frame(now);
            assert_eq!(timing.advance(sample).ticks, 0);
            assert_eq!(timing.render_alpha(now + 0.008, true), 1.);
        }
        let now = 20. + 1. / 30.;
        let sample = timing.begin_frame(now);
        assert_eq!(timing.advance(sample).ticks, 1);
        timing.history_ready = true;
        assert!((timing.render_alpha(now + 0.008, true) - 0.24).abs() < 1e-10);
        let invalid = timing.begin_frame(f64::NAN);
        timing.advance(invalid);
        assert!(timing.alpha_sample.is_none());
        assert_eq!(timing.render_alpha(100., true), timing.alpha());
    }

    #[test]
    fn render_alpha_gates_match_playing_and_finished_dead_only() {
        let mut game = camera_follow_fixture();
        let mut timing = HostTiming::default();
        timing.reset(10.);
        let now = 10. + 1. / 30.;
        let frame = timing.begin_frame(now);
        timing.advance(frame);
        timing.history_ready = true;
        for screen in [
            Screen::Title,
            Screen::Pause,
            Screen::Settings,
            Screen::Editor,
            Screen::Custom,
            Screen::Shortcuts,
            Screen::Records,
            Screen::About,
            Screen::Donation,
            Screen::Finished,
        ] {
            let running = render_clock_running(screen, true, false, Some(&game));
            assert!(!running, "{screen:?}");
            assert_eq!(timing.render_alpha(now + 0.008, running), timing.alpha());
        }
        assert!(render_clock_running(
            Screen::Playing,
            true,
            false,
            Some(&game)
        ));
        assert!(!render_clock_running(
            Screen::Playing,
            false,
            false,
            Some(&game)
        ));
        assert!(!render_clock_running(
            Screen::Playing,
            true,
            true,
            Some(&game)
        ));
        assert!(!render_clock_running(Screen::Finished, true, false, None));
        game.state = RunState::Dead;
        assert!(render_clock_running(
            Screen::Finished,
            true,
            false,
            Some(&game)
        ));
        assert!(!render_clock_running(
            Screen::Finished,
            false,
            false,
            Some(&game)
        ));
        assert!(!render_clock_running(
            Screen::Finished,
            true,
            true,
            Some(&game)
        ));
        // A source teleport/discontinuity retains the existing scene snap.
        game.flags.insert("render_discontinuity".into());
        game.player.previous_pos.x = game.player.pos.x - 4.;
        let atlas = Atlas::load().unwrap();
        let alpha = timing.render_alpha(now + 0.008, true);
        let snapshot = test_snapshot(Screen::Playing, Some(&game), alpha, false);
        assert_eq!(snapshot["alpha"], 1.);
        let actual: Vec<_> = scene::game_scene(&game, alpha, false, &atlas)
            .iter()
            .map(|quad| quad.rect)
            .collect();
        let snapped: Vec<_> = scene::game_scene(&game, 1., false, &atlas)
            .iter()
            .map(|quad| quad.rect)
            .collect();
        assert_eq!(actual, snapped);
    }
    use super::*;
    #[test]
    fn scores_reset_persists_source_sentinels_and_preserves_the_selected_character_and_lifetime_stats()
     {
        let mut save = crate::storage::SaveData::default();
        save.first_time = false;
        save.lifetime_stats.total_crates = 17;
        save.unlocks.extend(
            [
                "character_tunnelman",
                "unlock_tunnel_man",
                "unlock_sun",
                "trophy_money_3",
                "shortcut_5",
            ]
            .map(str::to_owned),
        );
        save.challenge_records.insert("mini1".into(), 42);
        let mut game = Game::shell(7, save.settings.rules);
        hydrate_run(&mut game, &save);
        game.state = RunState::Dead;
        save.records.push(campaign_record(&game).unwrap());
        game.data.insert("tunnel1_remaining".into(), 100_001.);
        game.data.insert("tunnel2_remaining".into(), 200_001.);
        assert!(!consume_highscores_reset(&mut game, &mut save));
        assert_eq!(save.records.len(), 1);
        game.flags.insert("highscores_clear_requested".into());
        assert!(consume_highscores_reset(&mut game, &mut save));
        assert!(!consume_highscores_reset(&mut game, &mut save));
        assert!(save.records.is_empty());
        assert!(save.challenge_records.is_empty());
        assert_eq!(
            save.unlocks,
            std::collections::BTreeSet::from(["character_tunnelman".into()])
        );
        save.tunnel_remaining.extend(tunnel_progress(&game));
        assert_eq!(save.tunnel_remaining["tunnel1_remaining"], 100_001);
        assert_eq!(save.tunnel_remaining["tunnel2_remaining"], 200_001);
        let reopened = crate::storage::SaveData::from_json(&save.to_json().unwrap()).unwrap();
        let mut next = Game::shell(8, reopened.settings.rules);
        hydrate_run(&mut next, &reopened);
        assert_eq!(
            spelunky_core::characters::selected(&next),
            spelunky_core::characters::Character::TunnelMan
        );
        assert_eq!(next.lifetime_stats.total_crates, 17);
        assert!(!next.flags.contains("first_time"));
    }
    #[test]
    fn source_record_markers_compare_previous_scores_and_eligible_winning_times() {
        let old = Record {
            seed: 1,
            rng_algorithm: spelunky_core::RngAlgorithm::LegacySplitMix64V1,
            simulation_version: 0,
            rules: spelunky_core::RulesPreset::Classic,
            score: 100,
            level: 16,
            ticks: 0,
            elapsed_ms: Some(500_000),
            fastest_eligible: true,
            won: true,
            kills: 5,
            rescues: 2,
        };
        assert!(record_improvements(&old, std::slice::from_ref(&old)).is_empty());
        let mut next = old.clone();
        next.score += 1;
        next.kills += 1;
        next.rescues += 1;
        next.elapsed_ms = Some(499_000);
        assert_eq!(
            record_improvements(&next, std::slice::from_ref(&old)),
            ["new_money", "new_kills", "new_saves", "new_time"]
                .map(str::to_owned)
                .into_iter()
                .collect()
        );
        next.fastest_eligible = false;
        assert!(!record_improvements(&next, std::slice::from_ref(&old)).contains("new_time"));
        next.fastest_eligible = true;
        next.won = false;
        assert!(!record_improvements(&next, std::slice::from_ref(&old)).contains("new_time"));
        next.won = true;
        assert!(record_improvements(&next, &[]).contains("new_time"));
    }
    #[test]
    fn lifetime_snapshots_survive_new_runs_without_aggregating_deaths() {
        use spelunky_core::statistics::{EnemyKill, LifetimeCounter};
        let mut save = crate::storage::SaveData::default();
        save.lifetime_stats.enemy_kills[0] = 3;
        let mut game = Game::shell(7, save.settings.rules);
        hydrate_run(&mut game, &save);
        assert_eq!(game.lifetime_stats.enemy_kills[0], 3);
        game.lifetime_stats.enemy_kill(EnemyKill::Snake);
        game.lifetime_stats.increment(LifetimeCounter::TotalCrates);
        assert!(capture_lifetime_stats(&game, &mut save));
        assert!(!capture_lifetime_stats(&game, &mut save));
        let reopened = crate::storage::SaveData::from_json(&save.to_json().unwrap()).unwrap();
        let mut next = Game::shell(8, reopened.settings.rules);
        hydrate_run(&mut next, &reopened);
        assert_eq!(next.lifetime_stats, game.lifetime_stats);
        assert_eq!(next.lifetime_stats.level_deaths, [0; 16]);
        assert_eq!(next.lifetime_stats.misc_deaths, [0; 12]);
        assert_eq!(next.lifetime_stats.total_crates, 1);
    }
    #[test]
    fn browser_snapshot_reports_cinematic_gates_without_advancing_them() {
        use spelunky_core::ending::{EndingKind, EndingStage, EndingState};
        let mut game = Game::shell(7, spelunky_core::RulesPreset::Classic);
        game.data.insert("xmoney".into(), 123.);
        game.data.insert("time".into(), 549_000.);
        let mut ending = EndingState::new(&game, EndingKind::Transition, EndingStage::Transition);
        ending.draw_loot = 2;
        ending.money_count = 123;
        game.ending = Some(ending);
        game.state = RunState::Ending;
        let hash = game.state_hash();
        let snapshot = test_snapshot(
            Screen::Playing,
            Some(&game),
            HostTiming::default().alpha(),
            false,
        );
        assert_eq!(snapshot["ending"]["kind"], "Transition");
        assert_eq!(snapshot["ending"]["stage"], "Transition");
        assert_eq!(snapshot["ending"]["drawLoot"], 2);
        assert_eq!(snapshot["ending"]["levelMoney"], 123);
        assert_eq!(snapshot["ending"]["moneyCount"], 123);
        assert_eq!(snapshot["sourceTimeMs"], 549_000);
        assert_eq!(hash, game.state_hash());
    }
    #[test]
    fn cinematic_start_cannot_confirm_or_pay_a_tunnel_donation() {
        use spelunky_core::{
            InputFrame, RulesPreset,
            ending::{self, EndingKind, EndingStage},
        };
        let mut game = Game::shell(25, RulesPreset::Classic);
        game.world.level = 4;
        game.player.money = 100_000;
        game.data.insert("tunnel1_remaining".into(), 100_000.);
        ending::begin_exit(&mut game, EndingKind::Transition);
        for _ in 0..100 {
            if game.ending.as_ref().unwrap().stage == EndingStage::Transition {
                break;
            }
            game.step(&InputFrame::default());
        }
        let tunnel = game
            .world
            .entities
            .iter()
            .find(|entity| entity.object == "oTunnelMan")
            .unwrap()
            .id;
        for _ in 0..100 {
            if game.world.entity(tunnel).unwrap().value("talk") == 1. {
                break;
            }
            game.step(&InputFrame::default());
        }
        assert_eq!(game.world.entity(tunnel).unwrap().value("talk"), 1.);
        let mut input = InputFrame::default();
        let mut pending = true;
        apply_cinematic_start(&mut input, &mut pending);
        assert!(input.start_pressed);
        assert!(!input.attack_pressed);
        assert!(!pending);
        game.step(&input);
        assert_eq!(game.world.entity(tunnel).unwrap().value("talk"), 1.);
        assert_eq!(game.player.money, 100_000);
        let mut controls = InputState::default();
        controls.key(KeyCode::KeyX, true, false);
        let attack = controls.take_tick();
        assert!(attack.attack_pressed);
        game.step(&attack);
        assert_eq!(game.world.entity(tunnel).unwrap().value("talk"), 2.);
        game.step(&InputFrame {
            up: true,
            ..Default::default()
        });
        assert_eq!(game.world.entity(tunnel).unwrap().value("donate"), 100.);
        game.step(&input);
        assert_eq!(game.world.entity(tunnel).unwrap().value("talk"), 2.);
        assert_eq!(game.player.money, 100_000);
        let donation = game.world.entity(tunnel).unwrap().value("donate") as u32;
        game.step(&attack);
        assert_eq!(game.world.entity(tunnel).unwrap().value("talk"), 3.);
        assert_eq!(game.player.money, 100_000 - donation);
        assert_eq!(
            game.data["tunnel1_remaining"],
            f64::from(100_000 - donation)
        );
        assert_eq!(game.state, RunState::Ending);
    }
    #[test]
    fn browser_snapshot_is_read_only_and_tracks_effective_presentation() {
        let mut game = Game::tutorial(42, spelunky_core::RulesPreset::Classic);
        game.player.previous_pos = spelunky_core::Vec2::new(96., 98.);
        game.player.pos = spelunky_core::Vec2::new(100., 100.);
        game.previous_camera = spelunky_core::Vec2::new(80., 80.);
        game.camera = spelunky_core::Vec2::new(84., 82.);
        game.player.bombs = 2;
        game.player.ropes = 3;
        game.player.money = 1234;
        let hash = game.state_hash();
        let mut timing = HostTiming {
            alpha: 0.25,
            history_ready: true,
            ..HostTiming::default()
        };
        let snapshot = test_snapshot(Screen::Playing, Some(&game), timing.alpha(), false);
        assert_eq!(snapshot["screen"], "Playing");
        assert_eq!(snapshot["seed"], "42");
        assert_eq!(snapshot["rngAlgorithm"], "gm8_lcg32_v1");
        assert_eq!(
            snapshot["simulationVersion"],
            spelunky_core::SIMULATION_VERSION
        );
        assert_eq!(snapshot["rngState"], game.rng.state_bits().to_string());
        assert_eq!(snapshot["player"]["x"], 100.);
        assert_eq!(snapshot["rendered"]["x"], 97.);
        assert_eq!(snapshot["rendered"]["screenX"], 16.);
        assert_eq!(snapshot["rendered"]["screenY"], 18.);
        assert_eq!(snapshot["player"]["bombs"], 2);
        assert_eq!(snapshot["player"]["ropes"], 3);
        assert_eq!(snapshot["money"], 1234);
        assert_eq!(snapshot["world"]["name"], game.world.name);
        assert_eq!(snapshot["world"]["width"], game.world.width);
        assert_eq!(snapshot["world"]["height"], game.world.height);
        assert_eq!(snapshot["world"]["level"], game.world.level);
        assert_eq!(snapshot["world"]["entityCount"], game.world.entities.len());
        assert_eq!(game.state_hash(), hash);
        timing.reset(10.);
        let paused = test_snapshot(Screen::Pause, Some(&game), timing.alpha(), false);
        assert_eq!(paused["alpha"], 1.);
        assert_eq!(paused["rendered"]["x"], paused["player"]["x"]);
        timing.history_ready = true;
        game.flags.insert("render_discontinuity".into());
        let teleported = test_snapshot(Screen::Playing, Some(&game), timing.alpha(), false);
        assert_eq!(teleported["alpha"], 1.);
        assert_eq!(teleported["rendered"]["x"], teleported["player"]["x"]);
    }
    fn camera_follow_fixture() -> Game {
        use spelunky_core::{RulesPreset, Vec2, World};
        let mut game = Game::shell(7, RulesPreset::Classic);
        game.world = World::new(40, 40, Area::Mines, 1);
        for x in 0..game.world.width {
            game.world.set_tile(x, 30, "oBlock");
        }
        game.player.pos = Vec2::new(224., 248.);
        game.player.previous_pos = game.player.pos;
        game.player.vel = Vec2::new(3., 1.);
        game.player.state = spelunky_core::PlayerState::Falling;
        game.player.sprite = "sFallLeft".into();
        game.camera = Vec2::new(32., 104.);
        game.previous_camera = game.camera;
        game
    }
    fn drawn_player_anchor(game: &Game, alpha: f64, atlas: &Atlas) -> [f64; 2] {
        drawn_player_anchor_in_view(game, alpha, atlas, Viewport::canonical(), false)
    }
    fn drawn_player_anchor_in_view(
        game: &Game,
        alpha: f64,
        atlas: &Atlas,
        viewport: Viewport,
        pixel_snap: bool,
    ) -> [f64; 2] {
        let player = &game.presentation().player;
        let frame = atlas.sprite(&player.sprite, player.animation).unwrap();
        let mut uv = frame.uv;
        let mirrored = player.facing > 0;
        if mirrored {
            uv[0] += uv[2];
            uv[2] = -uv[2];
        }
        let quads = scene::game_scene_in_view(game, alpha, pixel_snap, atlas, viewport);
        let quad = quads.iter().find(|quad| quad.uv == uv).unwrap();
        [
            f64::from(
                quad.rect[0]
                    + if mirrored {
                        frame.size[0] - frame.origin[0]
                    } else {
                        frame.origin[0]
                    },
            ),
            f64::from(quad.rect[1] + frame.origin[1]),
        ]
    }
    #[test]
    fn camera_follow_actual_scene_stays_continuous_across_high_refresh_ticks() {
        let atlas = Atlas::load().unwrap();
        let mut game = camera_follow_fixture();
        for _ in 0..4 {
            let previous_camera = game.camera;
            game.step(&spelunky_core::InputFrame {
                right: true,
                ..Default::default()
            });
            assert!(game.camera.x > previous_camera.x);
            assert!(game.camera.y > previous_camera.y);
            let hash = game.state_hash();
            for alpha in [0., 0.125, 0.25, 0.5, 0.875, 1.] {
                let anchor = drawn_player_anchor(&game, alpha, &atlas);
                assert!((anchor[0] - 192.).abs() < 0.0001, "{alpha}: {anchor:?}");
                assert!((anchor[1] - 144.).abs() < 0.0001, "{alpha}: {anchor:?}");
                let timing = HostTiming {
                    alpha,
                    history_ready: true,
                    ..Default::default()
                };
                let snapshot = test_snapshot(Screen::Playing, Some(&game), timing.alpha(), false);
                assert!(
                    (snapshot["rendered"]["screenX"].as_f64().unwrap() - anchor[0]).abs() < 0.0001
                );
                assert!(
                    (snapshot["rendered"]["screenY"].as_f64().unwrap() - anchor[1]).abs() < 0.0001
                );
                assert_eq!(
                    snapshot["rendered"]["frame"],
                    game.presentation().player.animation
                );
            }
            assert_eq!(game.state_hash(), hash, "presentation is read-only");
        }
    }
    #[test]
    fn camera_follow_room_entry_and_teleport_keep_snap_controls() {
        use spelunky_core::{InputFrame, PlayerState, RulesPreset, Vec2};
        let atlas = Atlas::load().unwrap();
        let mut game = camera_follow_fixture();
        game.step(&InputFrame {
            right: true,
            ..Default::default()
        });
        assert!(game.draw_view.is_some());
        let mut room = spelunky_core::level::Level::default();
        room.set_tile(1, 1, '@').unwrap();
        game.load_custom(&room, "new-room", "");
        assert!(game.draw_view.is_none());
        assert_eq!(game.camera, game.previous_camera);
        assert_eq!(game.player.pos, game.player.previous_pos);
        let room_anchor = drawn_player_anchor(&game, 1., &atlas);
        for alpha in [0., 0.125, 0.5, 0.875] {
            assert_eq!(drawn_player_anchor(&game, alpha, &atlas), room_anchor);
        }

        let mut game = Game::shell(11, RulesPreset::Classic);
        for x in 0..game.world.width {
            game.world.set_tile(x, 11, "oBlock");
        }
        game.player.pos = Vec2::new(160., 168.);
        game.player.previous_pos = game.player.pos;
        game.player.state = PlayerState::Standing;
        game.player.grounded = true;
        let teleporter = game.spawn("oTeleporter", game.player.pos);
        game.world.entity_mut(teleporter).unwrap().held = true;
        game.player.held = Some(teleporter);
        game.step(&InputFrame {
            attack_pressed: true,
            ..Default::default()
        });
        assert!(game.flags.contains("render_discontinuity"));
        assert!(game.player.pos.x > 160.);
        assert_eq!(game.presentation().camera, game.camera);
        let target = drawn_player_anchor(&game, 1., &atlas);
        for alpha in [0., 0.125, 0.5, 0.875] {
            assert_eq!(drawn_player_anchor(&game, alpha, &atlas), target);
        }
    }
    #[test]
    fn browser_snapshot_preserves_source_darkness_and_cached_light_distances() {
        let mut game = Game::tutorial(42, spelunky_core::RulesPreset::Classic);
        game.world.dark = true;
        game.data.insert("darkness".into(), -0.3);
        game.data.insert("darkness_lerp".into(), 0.7);
        game.player
            .data
            .insert("dist_to_nearest_light_source".into(), 37.);
        let flare = game
            .world
            .spawn("oFlare", spelunky_core::Vec2::new(96., 96.));
        game.world
            .entity_mut(flare)
            .unwrap()
            .data
            .insert("dist_to_player".into(), 18.);
        let hash = game.state_hash();
        let snapshot = test_snapshot(
            Screen::Playing,
            Some(&game),
            HostTiming::default().alpha(),
            false,
        );
        assert_eq!(snapshot["world"]["dark"], true);
        assert_eq!(snapshot["darkness"], -0.3);
        assert_eq!(snapshot["darkness_lerp"], 0.7);
        assert_eq!(snapshot["player"]["dist_to_nearest_light_source"], 37.);
        assert_eq!(snapshot["flares"][0]["id"], flare);
        assert_eq!(snapshot["flares"][0]["dist_to_player"], 18.);
        assert_eq!(
            snapshot["lighting"]["final_tint_rgb"],
            serde_json::json!([76, 77, 0])
        );
        assert_eq!(snapshot["lighting"]["circles"][0]["x"], game.player.pos.x);
        assert_eq!(snapshot["lighting"]["circles"][0]["radius"], 115.2);
        assert_eq!(
            snapshot["lighting"]["red_lamp_distances"],
            serde_json::json!([null, null])
        );
        assert_eq!(game.state_hash(), hash);
        game.data.remove("darkness");
        let uninitialized = test_snapshot(
            Screen::Playing,
            Some(&game),
            HostTiming::default().alpha(),
            false,
        );
        assert!(uninitialized["darkness"].is_null());
    }
    #[test]
    fn reset_keeps_current_snapshot_until_a_new_tick_at_high_refresh() {
        let atlas = Atlas::load().unwrap();
        let mut game = Game::tutorial(42, spelunky_core::RulesPreset::Classic);
        game.player.previous_pos = game.player.pos;
        game.player.pos.x += 4.;
        let hash = game.state_hash();
        let expected: Vec<_> = scene::game_scene(&game, 1., false, &atlas)
            .iter()
            .map(|quad| quad.rect)
            .collect();
        let mut timing = HostTiming::default();
        timing.reset(10.);
        for frame in 1..8 {
            let stamp = timing.begin_frame(10. + f64::from(frame) / 240.);
            assert_eq!(timing.advance(stamp).ticks, 0);
            let actual: Vec<_> = scene::game_scene(&game, timing.alpha(), false, &atlas)
                .iter()
                .map(|quad| quad.rect)
                .collect();
            assert_eq!(actual, expected, "frame {frame} rewound the player");
            assert_eq!(game.state_hash(), hash);
        }
        let stamp = timing.begin_frame(10. + 1. / 30.);
        assert_eq!(timing.advance(stamp).ticks, 1);
        game.step(&spelunky_core::InputFrame::default());
        timing.history_ready = true;
        assert!(timing.alpha() < 0.001);
    }
    #[test]
    fn resuming_during_gamepad_poll_discards_pre_resume_elapsed() {
        for pause in [0.25, 5., 120.] {
            let mut timing = HostTiming::default();
            timing.reset(1.);
            let sampled_before_resume = timing.begin_frame(1. + pause);
            timing.reset(1. + pause);
            assert_eq!(timing.advance(sampled_before_resume).ticks, 0);
            assert_eq!(timing.alpha(), 1.);
            let next = timing.begin_frame(1. + pause + 1. / 240.);
            assert_eq!(timing.advance(next).ticks, 0);
            let first_tick = timing.begin_frame(1. + pause + 1. / 30.);
            assert_eq!(timing.advance(first_tick).ticks, 1);
        }
    }
    #[test]
    fn source_game_over_keeps_corpse_and_world_simulating_without_automatic_return() {
        use spelunky_core::{Area, InputFrame, RulesPreset, Vec2, World};
        let mut game = Game::shell(42, RulesPreset::Classic);
        game.world = World::new(30, 20, Area::Mines, 1);
        for x in 0..30 {
            game.world.set_tile(x, 10, "oBlock");
        }
        game.player.pos = Vec2::new(80., 64.);
        game.player.vel = Vec2::new(3., -4.);
        game.player.health = 0;
        let rock = game.world.spawn("oRock", Vec2::new(200., 64.));
        game.step(&InputFrame::default());
        let lethal_tick = game.tick;
        assert_eq!(game.state, RunState::Dead);
        assert!(game.game_over.destination.is_none());
        assert_eq!(
            game.events
                .iter()
                .filter(|event| matches!(event, GameEvent::Died))
                .count(),
            1
        );
        let corpse_start = game.player.pos;
        let rock_start = game.world.entity(rock).unwrap().pos;
        for elapsed in 1..=100 {
            game.step(&InputFrame::default());
            assert_eq!(game.tick, lethal_tick + elapsed);
            assert_eq!(game.state, RunState::Dead);
            assert!(game.game_over.destination.is_none());
            assert!(
                !game
                    .events
                    .iter()
                    .any(|event| matches!(event, GameEvent::Died))
            );
            assert!(
                !game.events.iter().any(
                    |event| matches!(event, GameEvent::Sound { name, .. } if name == "sndDie")
                )
            );
            if elapsed == 5 {
                assert_ne!(game.player.pos, corpse_start);
                assert_ne!(game.world.entity(rock).unwrap().pos, rock_start);
            }
        }
        let menu_tick = game.tick;
        game.step(&InputFrame::default());
        assert_eq!(
            game.tick,
            menu_tick + 1,
            "the visible menu must not freeze the room"
        );
    }

    #[test]
    fn source_game_over_return_cannot_consume_the_same_presentation_frame_twice() {
        let mut game = Game::shell(42, spelunky_core::RulesPreset::Classic);
        game.player.health = 0;
        game.step(&spelunky_core::InputFrame::default());
        let tick_before_return = game.tick;
        let mut timing = HostTiming::default();
        timing.reset(1.);
        let now = 1. + 1. / 30.;
        let sampled = timing.begin_frame(now);
        for _ in 0..timing.advance(sampled).ticks {
            game.step(&spelunky_core::InputFrame::default());
        }
        assert_eq!(game.tick, tick_before_return + 1);
        // A host route resets timing; an already sampled frame cannot advance
        // the destination room a second time after that reset.
        timing.reset(now);
        for _ in 0..timing.advance(sampled).ticks {
            game.step(&spelunky_core::InputFrame::default());
        }
        assert_eq!(game.tick, tick_before_return + 1);
        assert_eq!(timing.alpha(), 1.);
    }

    #[test]
    fn source_start_acknowledges_game_over_after_hastening_without_a_death_aggregate() {
        let mut game = Game::shell(42, spelunky_core::RulesPreset::Classic);
        let mut save = crate::storage::SaveData::default();
        game.player.health = 0;
        game.step(&spelunky_core::InputFrame::default());
        capture_lifetime_stats(&game, &mut save);
        assert_eq!(save.lifetime_stats.level_deaths[0], 0);
        assert_eq!(game.lifetime_stats.level_deaths[0], 0);
        let mut input = spelunky_core::InputFrame::default();
        let mut pending = true;
        apply_cinematic_start(&mut input, &mut pending);
        game.step(&input);
        assert!(game.game_over.destination.is_none());
        assert_eq!(game.lifetime_stats.level_deaths[0], 0);
        game.step(&spelunky_core::InputFrame::default());
        game.step(&input);
        assert_eq!(
            game.game_over.destination,
            Some(spelunky_core::progression::GameOverDestination::Scores)
        );
        assert_eq!(game.lifetime_stats.level_deaths[0], 1);
        assert!(capture_lifetime_stats(&game, &mut save));
        let reopened = crate::storage::SaveData::from_json(&save.to_json().unwrap()).unwrap();
        assert_eq!(reopened.lifetime_stats.level_deaths[0], 1);
        game.step(&input);
        assert_eq!(game.lifetime_stats.level_deaths[0], 1);
    }

    #[test]
    fn source_game_over_start_uses_escape_without_inventing_an_enter_attack() {
        assert!(source_start_key(KeyCode::Escape, false));
        assert!(!source_start_key(KeyCode::Enter, false));
        assert!(!source_start_key(KeyCode::KeyP, false));
        assert!(source_start_key(KeyCode::Enter, true));
        let mut input = spelunky_core::InputFrame::default();
        let mut pending = source_start_key(KeyCode::Escape, false);
        apply_cinematic_start(&mut input, &mut pending);
        assert!(input.start_pressed);
        assert!(!input.attack_pressed);
        assert!(!pending);
    }

    #[test]
    fn tutorial_manual_return_uses_its_authored_title_door_and_resets_run_inventory() {
        let mut game = Game::tutorial(42, spelunky_core::RulesPreset::Classic);
        spelunky_core::characters::apply(
            &mut game,
            spelunky_core::characters::Character::TunnelMan,
        );
        game.flags.insert("unlock_tunnel_man".into());
        game.player.health = 1;
        game.player.bombs = 7;
        game.player.ropes = 8;
        game.player.money = 42_000;
        game.player.equipment.insert("Jetpack".into());
        for _ in 0..5 {
            game.step(&spelunky_core::InputFrame::default());
        }
        game.step(&spelunky_core::InputFrame {
            up: true,
            ..Default::default()
        });
        for _ in 0..100 {
            game.step(&spelunky_core::InputFrame::default());
            if game.state == RunState::Transition {
                break;
            }
        }
        assert!(game.flags.contains("hub_destination:title"));
        game.load_hub(spelunky_core::hub::HubKind::Title);
        assert_eq!(game.player.pos, spelunky_core::Vec2::new(344., 184.));
        assert_eq!(
            (game.player.health, game.player.bombs, game.player.ropes),
            (2, 0, 0)
        );
        assert_eq!(game.player.money, 0);
        assert!(game.player.equipment.is_empty());
        assert!(game.flags.contains("unlock_tunnel_man"));
        assert_eq!(
            spelunky_core::characters::selected(&game),
            spelunky_core::characters::Character::TunnelMan
        );
        assert_eq!(game.state, RunState::Playing);
    }

    #[test]
    fn onboarding_is_hydrated_without_overriding_explicit_campaign_or_tutorial_choices() {
        let mut save = crate::storage::SaveData::default();
        for tutorial in [false, true] {
            let mut game = if tutorial {
                Game::tutorial(42, save.settings.rules)
            } else {
                Game::new(42, save.settings.rules)
            };
            hydrate_run(&mut game, &save);
            assert_eq!(game.flags.contains("tutorial"), tutorial);
            assert!(game.flags.contains("first_time"));
            consume_onboarding(&mut game, &mut save);
            assert!(
                save.first_time,
                "an explicit start does not consume firstTime"
            );
            game.flags.remove("first_time");
            game.flags.insert("onboarding_consumed".into());
            consume_onboarding(&mut game, &mut save);
            assert!(!save.first_time);
            assert!(!game.flags.contains("onboarding_consumed"));
            save.first_time = true;
        }
        let mut completed = Game::tutorial(42, save.settings.rules);
        hydrate_run(&mut completed, &save);
        completed.player.pos = spelunky_core::Vec2::new(56., 248.);
        completed.player.previous_pos = completed.player.pos;
        for _ in 0..5 {
            completed.step(&spelunky_core::InputFrame::default());
        }
        completed.step(&spelunky_core::InputFrame {
            up: true,
            ..Default::default()
        });
        for _ in 0..100 {
            completed.step(&spelunky_core::InputFrame::default());
            if completed.world.name == "rTransition1" {
                break;
            }
        }
        assert_eq!(completed.world.name, "rTransition1");
        consume_onboarding(&mut completed, &mut save);
        assert!(!save.first_time);
        assert!(!completed.flags.contains("first_time"));
        let mut next = Game::new(43, save.settings.rules);
        hydrate_run(&mut next, &save);
        assert!(!next.flags.contains("first_time"));
    }
    #[test]
    fn custom_file_links_select_the_return_entrance_without_rewriting_documents() {
        let mut level = spelunky_core::level::Level::default();
        level.set_tile(1, 1, '@').unwrap();
        level.set_tile(15, 10, '@').unwrap();
        level.exit_targets = vec!["elsewhere".into(), "previous.lvl".into()];
        let original = level.clone();
        let mut game = Game::new(42, spelunky_core::RulesPreset::Classic);
        load_custom(&mut game, &level, "next.lvl", "previous");
        assert_eq!(game.player.pos, spelunky_core::Vec2::new(264., 184.));
        assert_eq!(game.player.previous_pos, game.player.pos);
        assert_eq!(game.previous_camera, game.camera);
        assert_eq!(game.world.name, "next");
        assert_eq!(level, original);
    }
    #[test]
    fn mixed_case_custom_file_names_select_the_same_return_entrance() {
        let mut level = spelunky_core::level::Level::default();
        level.set_tile(1, 1, '@').unwrap();
        level.set_tile(15, 10, '@').unwrap();
        level.exit_targets = vec!["elsewhere".into(), "PREVIOUS.LVL".into()];
        let mut game = Game::new(42, spelunky_core::RulesPreset::Classic);
        load_custom(&mut game, &level, "Next.LVL", "Previous.lvl");
        assert_eq!(game.world.name, "next");
        assert_eq!(game.player.pos, spelunky_core::Vec2::new(264., 184.));
        assert_eq!(level.exit_targets[1], "PREVIOUS.LVL");
        let levels = std::collections::BTreeMap::from([("Next".into(), level.to_text())]);
        for target in ["next", "NEXT.LVL", "C:\\levels\\Next.LVL"] {
            assert_eq!(
                custom_level_source(&levels, target),
                Some(levels["Next"].as_str())
            );
        }
        assert!(custom_level_source(&levels, "missing.lvl").is_none());
    }
    #[test]
    fn linked_custom_exit_retains_filename_through_the_authored_transition_room() {
        use spelunky_core::{InputFrame, RulesPreset, Vec2, World};
        let mut next = spelunky_core::level::Level::default();
        next.set_tile(1, 1, '@').unwrap();
        next.set_tile(15, 10, '@').unwrap();
        next.exit_targets = vec!["elsewhere".into(), "PREVIOUS.LVL".into()];
        let levels = std::collections::BTreeMap::from([("Next".into(), next.to_text())]);
        let mut game = Game::shell(7, RulesPreset::Classic);
        game.world = World::new(40, 20, Area::Mines, 2);
        game.world.name = "previous".into();
        game.flags.insert("custom".into());
        for x in 0..game.world.width {
            game.world.set_tile(x, 4, "oBrick");
        }
        let exit = game.world.spawn("oExit", Vec2::new(128., 48.));
        game.world
            .entity_mut(exit)
            .unwrap()
            .text
            .insert("leadsTo".into(), "NEXT.LVL".into());
        game.player.pos = Vec2::new(136., 56.);
        game.player.previous_pos = game.player.pos;
        game.player.grounded = true;
        game.step(&InputFrame {
            up: true,
            ..Default::default()
        });
        for _ in 0..1_000 {
            if game.state == RunState::Transition {
                assert!(game.world.name.starts_with("rTransition"));
                assert_eq!(game.ending.as_ref().unwrap().completed_name, "previous");
                assert_eq!(
                    custom_exit(&mut game, &levels, false),
                    CustomExit::Continued
                );
                assert_eq!(game.world.name, "next");
                assert_eq!(game.player.pos, Vec2::new(264., 184.));
                assert!(!game.player.is_exiting());
                let spawn_x = game.player.pos.x;
                for _ in 0..3 {
                    game.step(&InputFrame {
                        right: true,
                        ..Default::default()
                    });
                }
                assert!(
                    game.player.pos.x > spawn_x,
                    "Linked-room arrival must restore player movement"
                );
                return;
            }
            game.step(&InputFrame {
                attack_pressed: true,
                ..Default::default()
            });
        }
        panic!("The linked adventure never completed its authored transition");
    }
    #[test]
    fn custom_and_editor_starts_preserve_the_unlocked_character_selection() {
        for (flag, unlock, expected) in [
            (
                "character_damsel",
                "unlock_damsel",
                spelunky_core::characters::Character::Damsel,
            ),
            (
                "character_tunnelman",
                "unlock_tunnel_man",
                spelunky_core::characters::Character::TunnelMan,
            ),
        ] {
            let mut save = crate::storage::SaveData::default();
            save.unlocks.extend([flag.into(), unlock.into()]);
            let mut level = spelunky_core::level::Level::default();
            level.starting_life = 7;
            level.starting_bombs = 8;
            level.starting_ropes = 9;
            let mut game = Game::new(42, save.settings.rules);
            hydrate_run(&mut game, &save);
            load_custom(&mut game, &level, "custom.lvl", "");
            assert_eq!(spelunky_core::characters::selected(&game), expected);
            assert_eq!(
                (game.player.health, game.player.bombs, game.player.ropes),
                (7, 8, 9)
            );
        }
        let mut save = crate::storage::SaveData::default();
        save.unlocks.insert("character_tunnelman".into());
        let mut game = Game::new(42, save.settings.rules);
        hydrate_run(&mut game, &save);
        assert_eq!(
            spelunky_core::characters::selected(&game),
            spelunky_core::characters::Character::TunnelMan
        );
    }
    #[test]
    fn destination_music_resets_before_first_step_then_preserves_tick_frequency() {
        let mut game = Game::legacy(42, spelunky_core::RulesPreset::Classic);
        game.data.insert("music_frequency".into(), 34_100.0);
        game.next_level();
        assert_eq!(game.world.level, 2);
        assert_eq!(game.level_tick, 0);
        let (music, _) = game_music_request(&game);
        assert_eq!(music, "cave");
        // No destination Step has run: the host must reset before music().
        assert_eq!(initial_level_music_frequency(&game, music), Some(44_100));
        assert_eq!(initial_level_music_frequency(&game, music), Some(44_100));
        game.step(&spelunky_core::InputFrame::default());
        assert!(game.level_tick > 0);
        // Subsequent polling must leave source tick-driven deceleration intact.
        game.data.insert("music_frequency".into(), 44_000.0);
        assert_eq!(
            initial_level_music_frequency(&game, game_music(&game)),
            None
        );
    }

    #[test]
    fn initial_frequency_reset_excludes_muted_boss_hubs_and_cinematics() {
        let mut boss = Game::legacy(42, spelunky_core::RulesPreset::Classic);
        boss.data.insert("next_level".into(), 16.0);
        boss.next_level();
        assert_eq!(boss.level_tick, 0);
        assert_eq!(game_music(&boss), "");
        assert_eq!(
            initial_level_music_frequency(&boss, game_music(&boss)),
            None
        );

        let mut game = Game::legacy(42, spelunky_core::RulesPreset::Classic);
        assert_eq!(initial_level_music_frequency(&game, ""), None);
        game.load_hub(spelunky_core::hub::HubKind::Title);
        assert_eq!(game_music(&game), "title");
        assert_eq!(
            initial_level_music_frequency(&game, game_music(&game)),
            None
        );

        let mut game = Game::legacy(42, spelunky_core::RulesPreset::Classic);
        spelunky_core::opening::begin_title(&mut game, 0);
        assert_eq!(initial_level_music_frequency(&game, "title"), None);

        let mut game = Game::legacy(42, spelunky_core::RulesPreset::Classic);
        spelunky_core::ending::begin_exit(&mut game, spelunky_core::ending::EndingKind::Transition);
        assert!(game.is_level(), "Exit remains in the level room");
        assert_eq!(initial_level_music_frequency(&game, "cave"), None);
    }

    #[test]
    fn custom_music_tracks_are_independent_from_campaign_area_rules() {
        for (code, track) in [
            ("CAVE", "cave"),
            ("LUSH", "lush"),
            ("ICE", "ice"),
            ("TEMPLE", "temple"),
            ("BOSS", "boss"),
            ("JUNGLE", "cave"),
            ("unknown", "cave"),
        ] {
            let mut level = spelunky_core::level::Level::default();
            level.music = code.into();
            let mut game = Game::shell(42, spelunky_core::RulesPreset::Classic);
            load_custom(&mut game, &level, "music.lvl", "");
            assert_eq!(
                game.world.area,
                Area::Mines,
                "{code} must not change gameplay rules"
            );
            assert_eq!(game_music(&game), track, "custom music code {code}");
            assert_eq!(game_music_request(&game), (track, true));
            game.state = RunState::Dead;
            assert_eq!(game_music(&game), "");
        }
    }

    #[test]
    fn natural_custom_summary_keeps_its_one_shot_music_request() {
        let mut game = Game::shell(42, spelunky_core::RulesPreset::Classic);
        spelunky_core::ending::begin_custom(&mut game);
        assert_eq!(game_music_request(&game).0, "");
        for _ in 0..50 {
            game.step(&spelunky_core::InputFrame::default());
        }
        assert_eq!(game_music_request(&game), ("victory", false));
        for _ in 0..100 {
            game.step(&spelunky_core::InputFrame::default());
            assert_eq!(game_music_request(&game), ("victory", false));
        }
    }

    #[test]
    fn boss_music_waits_for_the_natural_intro_unlock() {
        let mut game = Game::new(42, spelunky_core::RulesPreset::Classic);
        while game.world.level < 16 {
            game.next_level();
        }
        assert_eq!(game_music(&game), "");
        for _ in 0..600 {
            game.step(&spelunky_core::InputFrame::default());
            if game.player.data.get("active") == Some(&1.0) {
                assert_eq!(game_music(&game), "boss");
                return;
            }
            assert_eq!(game_music(&game), "");
        }
        panic!("Olmec intro never restored player control and boss music");
    }

    #[test]
    fn source_opening_and_hub_music_follow_room_alarms() {
        use spelunky_core::{InputFrame, RulesPreset, hub::HubKind, opening};
        let mut game = Game::shell(42, RulesPreset::Classic);
        opening::begin_intro(&mut game);
        assert_eq!(game_music(&game), "");
        opening::begin_title(&mut game, 0);
        for _ in 0..269 {
            game.step(&InputFrame::default());
            assert_eq!(game_music(&game), "");
        }
        game.step(&InputFrame::default());
        assert_eq!(game_music(&game), "title");
        game.load_hub(HubKind::Scores);
        assert_eq!(game_music(&game), "");
        for kind in [HubKind::Sun, HubKind::Moon, HubKind::Stars] {
            game.load_hub(kind);
            for _ in 0..99 {
                game.step(&InputFrame::default());
                assert_eq!(game_music(&game), "");
            }
            game.step(&InputFrame::default());
            assert_eq!(game_music(&game), "boss");
        }
    }

    #[test]
    fn linked_custom_rooms_keep_run_resources_instead_of_reapplying_start_headers() {
        let mut first = spelunky_core::level::Level::default();
        first.starting_life = 8;
        first.starting_bombs = 9;
        first.starting_ropes = 10;
        let mut second = spelunky_core::level::Level::default();
        second.set_tile(1, 1, '@').unwrap();
        second.set_tile(15, 10, '@').unwrap();
        second.exit_targets = vec!["elsewhere.lvl".into(), "first.lvl".into()];
        second.starting_life = 99;
        second.starting_bombs = 99;
        second.starting_ropes = 99;
        let original = second.clone();
        let mut game = Game::new(42, spelunky_core::RulesPreset::Classic);
        load_custom(&mut game, &first, "first.lvl", "");
        game.player.health = 3;
        game.player.bombs = 2;
        game.player.ropes = 1;
        game.player.money = 12_345;
        game.player.equipment.insert("cape".into());
        let gun = game.world.spawn("oPistol", game.player.pos);
        let entity = game.world.entity_mut(gun).unwrap();
        entity.held = true;
        entity.data.insert("ammo".into(), 3.);
        game.player.held = Some(gun);
        game.flags.insert("carry:oPistol".into());
        continue_custom(&mut game, &second, "second.lvl", "first.lvl");
        assert_eq!(game.player.pos, spelunky_core::Vec2::new(264., 184.));
        assert_eq!(
            (game.player.health, game.player.bombs, game.player.ropes),
            (3, 2, 1)
        );
        assert_eq!(game.player.money, 12_345);
        assert!(game.player.equipment.contains("cape"));
        let gun = game.world.entity(game.player.held.unwrap()).unwrap();
        assert_eq!(gun.object, "oPistol");
        assert!(
            !gun.data.contains_key("ammo"),
            "Source recreates a fresh unlimited-ammo pistol by type"
        );
        assert_eq!(gun.value("cost"), 0.);
        assert_eq!(gun.value("new"), 0.);
        assert!(gun.held);
        assert_eq!(gun.previous_pos, gun.pos);
        assert_eq!(second, original);
        continue_custom(&mut game, &first, "first.lvl", "second.lvl");
        assert_eq!(
            (game.player.health, game.player.bombs, game.player.ropes),
            (3, 2, 1)
        );
        assert_eq!(game.player.money, 12_345);
    }
    #[test]
    fn editor_playtest_uses_document_name_and_source_spawn_offset() {
        let mut editor = EditorUi::new();
        // The source editor starts blank. Author the entrance whose offset
        // this test verifies rather than relying on the old seeded template.
        let mut authored = spelunky_core::level::Level::default();
        authored.set_tile(2, 30, '@').unwrap();
        editor.load("custom.lvl", &authored.to_text()).unwrap();
        let EditorCommand::Play(level) = editor.key(KeyCode::F5, None, false) else {
            panic!("F5 must play the current document");
        };
        let mut game = Game::new(42, spelunky_core::RulesPreset::Classic);
        load_custom(&mut game, &level, editor.level_name(), "");
        assert_eq!(game.world.name, "custom");
        assert_eq!(game.player.pos, spelunky_core::Vec2::new(56., 504.));
        assert_eq!(game.player.previous_pos, game.player.pos);
        assert_eq!(game.previous_camera, game.camera);
    }
    #[test]
    fn editor_live_preview_rebuilds_after_f5_and_an_ordinary_authored_door_exit() {
        let mut level = spelunky_core::level::Level::default();
        level.set_tile(1, 12, '@').unwrap();
        level.set_tile(3, 12, 'X').unwrap();
        level.set_tile(8, 12, '{').unwrap();
        for x in 0..20 {
            level.set_tile(x, 13, '1').unwrap();
        }
        level.exit_targets = vec!["NONE".into(), "NONE".into()];
        let mut editor = EditorUi::new();
        editor.load("roundtrip.lvl", &level.to_text()).unwrap();
        editor.pointer(76.0, 48.0, false, true);
        editor.pointer(0.0, 0.0, false, false);
        let atlas = Atlas::load().unwrap();
        let mega = atlas.sprite("sMegaMouthTile", 0.0).unwrap().uv;
        let has_mega = |editor: &EditorUi| {
            let mut quads = Vec::new();
            editor.draw(&atlas, &mut quads);
            quads
                .iter()
                .any(|q| q.uv == mega && q.rect[0] < 240.0 && q.rect[1] >= 36.0)
        };
        assert!(!has_mega(&editor), "off-anchor erase removed live preview");
        let EditorCommand::Play(document) = editor.key(KeyCode::F5, None, false) else {
            panic!("F5");
        };
        assert_eq!(
            document.get_tile(8, 12),
            Some('{'),
            "serialized anchor was preserved"
        );
        let mut game = Game::shell(937, spelunky_core::RulesPreset::Classic);
        game.rng = editor.source_rng().clone();
        load_custom(&mut game, &document, editor.level_name(), "");
        game.flags.insert("editor_test".into());
        for _ in 0..160 {
            game.step(&spelunky_core::InputFrame {
                right: true,
                up: true,
                ..Default::default()
            });
            if game.state == RunState::Transition {
                break;
            }
        }
        assert_eq!(
            game.state,
            RunState::Transition,
            "normal movement and Up must exit authored door"
        );
        assert!(
            game.ending.is_none(),
            "editor return bypasses transition room"
        );
        assert_eq!(
            custom_exit(&mut game, &Default::default(), true),
            CustomExit::Editor
        );
        editor.set_source_rng(game.rng.clone());
        editor.return_from_playtest();
        assert!(
            has_mega(&editor),
            "source return rebuilds saved F5 document"
        );
    }

    #[test]
    fn editor_playtest_exits_return_to_editor_before_following_or_finishing_links() {
        let level = spelunky_core::level::Level::default();
        let levels = std::collections::BTreeMap::from([("next".into(), level.to_text())]);
        for target in ["NONE", "NEXT.LVL", "missing.lvl"] {
            let mut game = Game::new(42, spelunky_core::RulesPreset::Classic);
            load_custom(&mut game, &level, "edited.lvl", "");
            game.flags.insert(format!("custom_level:{target}"));
            game.state = RunState::Transition;
            game.player.health = 2;
            let original = game.state_hash();
            assert_eq!(custom_exit(&mut game, &levels, true), CustomExit::Editor);
            assert_eq!(game.world.name, "edited");
            assert_eq!(game.state_hash(), original);
        }
    }

    #[test]
    fn ordinary_custom_exit_follows_links_and_ends_missing_or_final_rooms() {
        let level = spelunky_core::level::Level::default();
        let levels = std::collections::BTreeMap::from([("Next".into(), level.to_text())]);
        let mut game = Game::new(42, spelunky_core::RulesPreset::Classic);
        load_custom(&mut game, &level, "first.lvl", "");
        game.player.health = 2;
        game.player.bombs = 1;
        game.flags.insert("custom_level:NEXT.LVL".into());
        assert_eq!(
            custom_exit(&mut game, &levels, false),
            CustomExit::Continued
        );
        assert_eq!(game.world.name, "next");
        assert_eq!((game.player.health, game.player.bombs), (2, 1));
        game.flags.insert("custom_level:missing.lvl".into());
        assert_eq!(custom_exit(&mut game, &levels, false), CustomExit::Finished);
        assert_eq!(game.state, RunState::Ending);
        assert_eq!(
            game.ending.as_ref().unwrap().kind,
            spelunky_core::ending::EndingKind::Custom
        );
        assert!(campaign_record(&game).is_none());
        continue_custom(&mut game, &level, "next.lvl", "first.lvl");
        game.flags.remove("custom_level:missing.lvl");
        assert_eq!(custom_exit(&mut game, &levels, false), CustomExit::Finished);
        assert_eq!(game.state, RunState::Ending);
    }
    #[test]
    fn challenge_score_is_captured_before_scores_room_clears_run_counters() {
        let mut game = Game::new(42, spelunky_core::RulesPreset::Classic);
        game.flags.extend(["unlock_moon".into(), "hub".into()]);
        game.load_hub(spelunky_core::hub::HubKind::Moon);
        game.data.insert("mini2".into(), 12.);
        let mut records = std::collections::BTreeMap::new();
        records.insert("mini2".into(), 5);
        assert!(capture_challenge_records(&game, &mut records));
        game.load_hub(spelunky_core::hub::HubKind::Scores);
        capture_challenge_records(&game, &mut records);
        assert_eq!(records["mini2"], 12);
    }
    #[test]
    fn challenge_return_preserves_retry_and_source_scores_spawn() {
        use spelunky_core::hub::HubKind;
        for (kind, expected) in [
            (HubKind::Sun, spelunky_core::Vec2::new(40., 56.)),
            (HubKind::Moon, spelunky_core::Vec2::new(72., 88.)),
            (HubKind::Stars, spelunky_core::Vec2::new(40., 120.)),
        ] {
            let mut game = Game::shell(7, spelunky_core::RulesPreset::Classic);
            game.load_hub(kind);
            assert_eq!(retry_hub(&game), kind);
            game.state = RunState::Dead;
            prepare_challenge_scores(&mut game);
            game.load_hub(HubKind::Scores);
            assert_eq!(game.player.pos, expected, "{kind:?}");
            assert_eq!(retry_hub(&game), HubKind::Scores);
        }
        let mut game = Game::shell(7, spelunky_core::RulesPreset::Classic);
        game.load_hub(HubKind::Title);
        assert_eq!(retry_hub(&game), HubKind::Title);
    }

    #[test]
    fn death_record_uses_live_source_clock_and_excludes_shortcut_fastest_time() {
        let mut game = Game::shell(7, spelunky_core::RulesPreset::Classic);
        game.tick = 18_300;
        game.data.insert("time".into(), 549_000.);
        game.state = RunState::Dead;
        let record = campaign_record(&game).unwrap();
        assert_eq!(record.elapsed_seconds(), 549);
        assert!(!record.won);
        game.flags.insert("used_shortcut".into());
        assert!(!campaign_record(&game).unwrap().fastest_eligible);
        for flag in ["custom", "tutorial", "hub"] {
            game.flags.insert(flag.into());
            assert!(campaign_record(&game).is_none(), "{flag}");
            game.flags.remove(flag);
        }
        game.state = RunState::Won;
        assert!(
            campaign_record(&game).is_none(),
            "A win without completed credits cannot be persisted"
        );
    }
    #[test]
    fn campaign_win_record_waits_for_bonus_and_credits_completion() {
        use spelunky_core::{InputFrame, RulesPreset, Vec2, World};
        let mut game = Game::shell(7, RulesPreset::Classic);
        game.world = World::new(40, 20, Area::Boss, 16);
        for x in 0..game.world.width {
            game.world.set_tile(x, 4, "oTemple");
        }
        game.world.spawn("oXEnd", Vec2::new(128., 48.));
        game.player.pos = Vec2::new(136., 56.);
        game.player.previous_pos = game.player.pos;
        game.player.grounded = true;
        game.player.money = 150_000;
        game.data.insert("time".into(), 549_000.);
        game.step(&InputFrame {
            up: true,
            ..Default::default()
        });
        assert_eq!(game.state, RunState::Ending);
        assert_eq!(game.player.money, 150_000);
        assert!(campaign_record(&game).is_none());
        let mut clock = None;
        let mut bonus_seen = false;
        for _ in 0..3_000 {
            game.step(&InputFrame {
                attack: true,
                attack_pressed: true,
                ..Default::default()
            });
            if game.ending.as_ref().unwrap().stage != spelunky_core::ending::EndingStage::Exit {
                assert_eq!(
                    source_elapsed_ms(&game),
                    *clock.get_or_insert(source_elapsed_ms(&game)),
                    "Cinematics freeze the run clock"
                );
            }
            bonus_seen |= game.player.money == 200_000;
            if let Some(record) = campaign_record(&game) {
                assert_eq!(game.state, RunState::Won);
                assert!(bonus_seen);
                assert_eq!(record.score, 200_000);
                assert_eq!(record.elapsed_seconds(), 549);
                let unlocks = spelunky_core::hub::record_unlocks(
                    record.score,
                    record
                        .fastest_eligible
                        .then_some(record.elapsed_seconds() as f64),
                    record.kills,
                    record.rescues,
                );
                assert!(unlocks.contains("unlock_sun"));
                assert!(unlocks.contains("unlock_moon"));
                game.flags.insert("used_shortcut".into());
                game.ending.as_mut().unwrap().shortcut_used = true;
                let shortcut = campaign_record(&game).unwrap();
                let unlocks = spelunky_core::hub::record_unlocks(
                    shortcut.score,
                    shortcut
                        .fastest_eligible
                        .then_some(shortcut.elapsed_seconds() as f64),
                    shortcut.kills,
                    shortcut.rescues,
                );
                assert!(unlocks.contains("unlock_sun"));
                assert!(!unlocks.contains("unlock_moon"));
                return;
            }
        }
        panic!("Campaign credits never reached the source record boundary");
    }
    #[test]
    fn custom_summary_awards_its_bonus_without_a_campaign_record() {
        let level = spelunky_core::level::Level::default();
        let mut game = Game::shell(7, spelunky_core::RulesPreset::Classic);
        load_custom(&mut game, &level, "final.lvl", "");
        game.player.money = 150_000;
        assert_eq!(
            custom_exit(&mut game, &Default::default(), false),
            CustomExit::Finished
        );
        for _ in 0..2_000 {
            assert!(campaign_record(&game).is_none());
            if game.ending.as_ref().unwrap().stage == spelunky_core::ending::EndingStage::Complete {
                assert_eq!(game.player.money, 200_000);
                assert!(game.ending.as_ref().unwrap().bonus_awarded);
                assert!(!game.ending.as_ref().unwrap().record_ready);
                return;
            }
            game.step(&spelunky_core::InputFrame {
                attack_pressed: true,
                ..Default::default()
            });
        }
        panic!("Custom summary never completed");
    }
    #[test]
    fn pointer_letterbox_coordinates_match_renderer() {
        for (actual, expected) in [
            (logical_pointer(240., 0., 1920, 1080), [0., 0.]),
            (logical_pointer(1680., 1080., 1920, 1080), [320., 240.]),
            (logical_pointer(0., 100., 800, 800), [0., 0.]),
        ] {
            for index in 0..2 {
                assert!((actual[index] - expected[index]).abs() < 0.0001);
            }
        }
        assert!(
            logical_pointer(0., 0., 0, 100)
                .iter()
                .all(|value| !value.is_finite())
        );
    }
    #[test]
    fn resized_editor_pointer_paints_the_same_source_cells_without_joining_strokes() {
        let mut editor = EditorUi::new();
        for ((width, height), [source_x, source_y]) in
            [((1920, 1080), [12., 48.]), ((800, 800), [100., 72.])]
        {
            // Derive physical positions from the fixed UI's independent
            // surface centering contract, then exercise the real editor.
            let scale = (width as f64 / 320.).min(height as f64 / 240.);
            let x = (width as f64 - 320. * scale) * 0.5 + source_x * scale;
            let y = (height as f64 - 240. * scale) * 0.5 + source_y * scale;
            let [logical_x, logical_y] = logical_pointer(x, y, width, height);
            editor.pointer(logical_x, logical_y, true, false);
            editor.cancel_pointer();
        }
        let EditorCommand::Play(level) = editor.key(KeyCode::F5, None, false) else {
            panic!("Editor did not produce its painted level");
        };
        assert_eq!(level.get_tile(1, 12), Some('1'));
        assert_eq!(level.get_tile(12, 15), Some('1'));
        assert_eq!(level.get_tile(6, 13), Some('0'));
    }
    #[test]
    fn viewport_diagnostics_match_actual_player_projection_without_mutating_core() {
        let atlas = Atlas::load().unwrap();
        let mut game = camera_follow_fixture();
        game.step(&spelunky_core::InputFrame {
            right: true,
            ..Default::default()
        });
        let hash = game.state_hash();
        for (width, height) in [(1920, 1080), (800, 800), (1080, 1920), (320, 240)] {
            let viewport = Viewport::from_surface(width, height);
            for alpha in [0., 0.25, 0.5, 1.] {
                for pixel_snap in [false, true] {
                    let mut snapshot =
                        test_snapshot(Screen::Playing, Some(&game), alpha, pixel_snap);
                    let source_camera = snapshot["camera"]["authoritativeX"].clone();
                    apply_viewport_snapshot(&mut snapshot, Some(&game), viewport, pixel_snap);
                    let anchor =
                        drawn_player_anchor_in_view(&game, alpha, &atlas, viewport, pixel_snap);
                    for (index, key) in ["screenX", "screenY"].into_iter().enumerate() {
                        assert!(
                            (snapshot["rendered"][key].as_f64().unwrap() - anchor[index]).abs()
                                < 0.0001,
                            "{width}x{height}, alpha={alpha}, pixel_snap={pixel_snap}, {key}"
                        );
                    }
                    assert_eq!(snapshot["camera"]["authoritativeX"], source_camera);
                    assert_eq!(snapshot["viewport"]["surfaceWidth"], width);
                    assert_eq!(snapshot["viewport"]["surfaceHeight"], height);
                }
            }
        }
        assert_eq!(
            game.state_hash(),
            hash,
            "viewport diagnostics changed simulation"
        );
    }
    #[test]
    fn canonical_viewport_keeps_small_room_authored_camera_in_diagnostics_and_scene() {
        use spelunky_core::{Vec2, World};
        let atlas = Atlas::load().unwrap();
        let mut game = camera_follow_fixture();
        game.world = World::new(10, 8, Area::Mines, 1);
        game.player.previous_pos = Vec2::new(76., 62.);
        game.player.pos = Vec2::new(80., 64.);
        game.previous_camera = Vec2::new(9., 5.);
        game.camera = Vec2::new(11., 7.);
        game.flags.remove("render_discontinuity");
        let hash = game.state_hash();
        for (width, height) in [(320, 240), (960, 720), (1920, 1440)] {
            let viewport = Viewport::from_surface(width, height);
            assert_eq!([viewport.width, viewport.height], [320., 240.]);
            for alpha in [0., 0.5, 1.] {
                let mut snapshot = test_snapshot(Screen::Playing, Some(&game), alpha, false);
                let authored_camera = [
                    snapshot["camera"]["renderedX"].clone(),
                    snapshot["camera"]["renderedY"].clone(),
                ];
                apply_viewport_snapshot(&mut snapshot, Some(&game), viewport, false);
                assert_eq!(snapshot["camera"]["renderedX"], authored_camera[0]);
                assert_eq!(snapshot["camera"]["renderedY"], authored_camera[1]);
                let anchor = drawn_player_anchor_in_view(&game, alpha, &atlas, viewport, false);
                for (index, key) in ["screenX", "screenY"].into_iter().enumerate() {
                    assert!(
                        (snapshot["rendered"][key].as_f64().unwrap() - anchor[index]).abs()
                            < 0.0001
                    );
                }
            }
        }
        assert_eq!(
            game.state_hash(),
            hash,
            "canonical presentation changed core"
        );
    }
    #[test]
    fn sign_messages_preserve_lines_and_wrap_without_losing_words() {
        assert_eq!(wrap_message("FIRST\nSECOND", 54), ["FIRST", "SECOND"]);
        let message = "A MESSAGE WITH WORDS THAT MUST WRAP";
        let lines = wrap_message(message, 12);
        assert!(lines.iter().all(|line| line.chars().count() <= 12));
        assert_eq!(lines.join(" "), message);
        assert_eq!(
            wrap_message("SUPERCALIFRAGILISTIC", 8).join(""),
            "SUPERCALIFRAGILISTIC"
        );
    }
    #[test]
    fn imported_names_are_portable() {
        assert_eq!(level_name("/tmp/example.lvl"), "example");
        assert_eq!(level_name("C:\\levels\\example.lvl"), "example");
        assert_eq!(level_name("C:\\levels\\Example.LVL"), "Example");
        assert_eq!(level_name("map.lvl.lvl"), "map.lvl");
    }
    #[test]
    fn editor_exit_save_failure_keeps_the_draft_until_success_then_returns_to_authored_title() {
        let mut editor = EditorUi::new();
        editor.key(KeyCode::KeyN, None, false);
        editor.key(KeyCode::KeyA, Some("saved-room"), false);
        editor.key(KeyCode::Enter, None, false);
        // A genuine pointer edit, followed by ordinary source Escape/Y input.
        editor.pointer(4., 40., true, false);
        editor.pointer(4., 40., false, false);
        assert!(matches!(
            editor.key(KeyCode::Escape, None, false),
            EditorCommand::None
        ));
        let EditorCommand::Save { name, text } = editor.key(KeyCode::KeyY, None, false) else {
            panic!("source Y must request saving before leaving");
        };
        assert_eq!(name, "saved-room.lvl");
        assert_eq!(
            spelunky_core::level::Level::parse(&text)
                .unwrap()
                .get_tile(0, 11),
            Some('1')
        );
        assert!(matches!(
            complete_editor_save(&mut editor, &name, Err("quota exhausted".into())),
            EditorCommand::None
        ));
        let EditorCommand::Save {
            name: retry_name,
            text: retry_text,
        } = editor.key(KeyCode::KeyY, None, false)
        else {
            panic!("a failed save must preserve the confirmation and draft for retry");
        };
        assert_eq!((&retry_name, &retry_text), (&name, &text));
        let mut save = crate::storage::SaveData::default();
        save.custom_levels.insert(level_name(&name), text.clone());
        let restored = crate::storage::SaveData::from_json(&save.to_json().unwrap()).unwrap();
        assert_eq!(restored.custom_levels["saved-room"], text);
        assert!(matches!(
            complete_editor_save(&mut editor, &name, Ok(())),
            EditorCommand::Back
        ));
        let mut game = Game::shell(19, spelunky_core::RulesPreset::Classic);
        game.load_custom(&spelunky_core::level::Level::default(), "saved-room", "");
        game.flags.insert("title_start:1".into());
        prepare_editor_return(&mut game);
        game.load_hub(spelunky_core::hub::HubKind::Title);
        assert_eq!(game.world.name, "rTitle");
        assert_eq!(game.player.pos, spelunky_core::Vec2::new(600., 184.));
        assert_eq!(game.player.facing, -1);
        assert!(!game.flags.contains("custom"));
        assert!(!game.flags.contains("custom_level"));
    }

    #[test]
    fn editor_new_save_failure_does_not_clear_old_level_and_success_restarts_source_defaults() {
        let mut editor = EditorUi::new();
        let mut old = spelunky_core::level::Level::default();
        old.starting_life = 9;
        old.author = "OLD AUTHOR".into();
        old.set_tile(0, 11, 'I').unwrap();
        old.signs = vec!["PRESERVE THIS AUTHORED SIGN".into()];
        editor.load("old.lvl", &old.to_text()).unwrap();
        editor.key(KeyCode::KeyN, None, true);
        let EditorCommand::Save { name, text } = editor.key(KeyCode::KeyY, None, false) else {
            panic!("source New/Y must save the old document first");
        };
        assert_eq!(spelunky_core::level::Level::parse(&text).unwrap(), old);
        complete_editor_save(&mut editor, &name, Err("write failed".into()));
        let EditorCommand::Save { text: retry, .. } = editor.key(KeyCode::KeyY, None, false) else {
            panic!("retry");
        };
        assert_eq!(retry, text);
        assert!(matches!(
            complete_editor_save(&mut editor, &name, Ok(())),
            EditorCommand::None
        ));
        editor.key(KeyCode::KeyF, Some("fresh"), false);
        editor.key(KeyCode::Enter, None, false);
        let EditorCommand::Save { name, text } = editor.key(KeyCode::KeyS, None, true) else {
            panic!("fresh save");
        };
        assert_eq!(name, "fresh.lvl");
        assert_eq!(
            spelunky_core::level::Level::parse(&text).unwrap(),
            spelunky_core::level::Level::default()
        );
    }
}

#[cfg(test)]
#[path = "moon_timeout_host_tests.rs"]
mod moon_timeout_host_tests;
