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
};
use spelunky_core::{Area, Game, GameEvent, RunState};
use std::sync::Arc;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    keyboard::{KeyCode, PhysicalKey},
    window::{Fullscreen, Window, WindowId},
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
    history_ready: bool,
}
impl Default for HostTiming {
    fn default() -> Self {
        Self {
            clock: FixedClock::new(),
            last_frame: 0.,
            generation: 0,
            alpha: 0.,
            history_ready: false,
        }
    }
}
#[derive(Clone, Copy)]
struct HostFrame {
    elapsed: f64,
    generation: u64,
}
impl HostTiming {
    fn reset(&mut self, now: f64) {
        self.clock.reset();
        self.last_frame = now;
        self.generation = self.generation.wrapping_add(1);
        self.history_ready = false;
    }
    fn begin_frame(&mut self, now: f64) -> HostFrame {
        let frame = HostFrame {
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
        step
    }
    fn alpha(&self) -> f64 {
        if self.history_ready { self.alpha } else { 1. }
    }
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
    finish_tick: Option<u64>,
    focused: bool,
    occluded: bool,
    next_frame: f64,
    cursor: [f32; 2],
    mouse_left: bool,
    mouse_right: bool,
    control: bool,
    message: String,
    message_until: f64,
    message_simulated: bool,
    #[cfg(not(target_arch = "wasm32"))]
    epoch: std::time::Instant,
}
impl App {
    fn new(proxy: EventLoopProxy<UserEvent>) -> Self {
        let store = Store::load();
        let mut input = InputState::default();
        input.set_bindings(&store.data.settings.bindings);
        Self {
            proxy,
            window: None,
            renderer: None,
            atlas: None,
            input,
            audio: Audio::new(),
            store,
            timing: HostTiming::default(),
            game: None,
            screen: Screen::Title,
            settings_return: Screen::Title,
            selection: 0,
            settings_ui: SettingsUi::new(),
            editor: EditorUi::new(),
            editor_play: false,
            donation: None,
            donation_confirm: false,
            recorded: false,
            finish_tick: None,
            focused: true,
            occluded: false,
            next_frame: 0.,
            cursor: [0.; 2],
            mouse_left: false,
            mouse_right: false,
            control: false,
            message: String::new(),
            message_until: 0.,
            message_simulated: false,
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
        self.donation_confirm = false;
        self.timing.reset(self.now());
        self.input.clear_gameplay();
        self.mouse_left = false;
        self.mouse_right = false;
        self.editor.cancel_pointer();
    }
    fn reset_room_timing(&mut self) {
        self.donation_confirm = false;
        self.timing.reset(self.now());
        self.input.clear_edges();
    }
    fn screen(&mut self, screen: Screen) {
        self.screen = screen;
        self.selection = 0;
        self.reset_timing();
        self.audio
            .pause(!self.focused || self.occluded || matches!(screen, Screen::Pause));
    }
    fn toast(&mut self, message: impl Into<String>) {
        self.message = message.into();
        self.message_simulated = false;
        self.message_until = self.now() + 6.;
    }
    fn settings_changed(&mut self) {
        let settings = &self.store.data.settings;
        self.input.set_bindings(&settings.bindings);
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
        if let Some(window) = &self.window {
            window.set_fullscreen(if window.fullscreen().is_some() {
                None
            } else {
                Some(Fullscreen::Borderless(None))
            });
        }
    }
    fn start_game(&mut self, tutorial: bool) {
        let seed = (self.now() * 1_000_000.) as u64 ^ wall_seed();
        let mut game = if tutorial {
            Game::tutorial(seed, self.store.data.settings.rules)
        } else {
            Game::new(seed, self.store.data.settings.rules)
        };
        hydrate_run(&mut game, &self.store.data);
        if tutorial {
            game.player.bombs = 0;
            game.player.ropes = 2;
        }
        self.game = Some(game);
        self.editor_play = false;
        self.recorded = false;
        self.finish_tick = None;
        self.screen(Screen::Playing);
    }
    fn open_hub(&mut self, kind: spelunky_core::hub::HubKind) {
        if self.game.is_none() {
            self.start_game(false);
        }
        if let Some(game) = &mut self.game {
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
                        .filter(|record| record.won)
                        .map(|record| record.ticks)
                        .min()
                        .unwrap_or(0) as f64
                        / 30.,
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
        self.finish_tick = None;
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
    fn key(&mut self, key: KeyCode, text_value: Option<&str>) {
        if key == KeyCode::F4
            && !(self.screen == Screen::Settings && self.settings_ui.awaiting_binding())
        {
            self.fullscreen();
            return;
        }
        match self.screen {
            Screen::Donation => {
                if matches!(key, KeyCode::Enter | KeyCode::Escape) {
                    self.continue_transition();
                }
            }
            Screen::Playing => {
                if self
                    .game
                    .as_ref()
                    .is_some_and(|game| game.state == RunState::Dead)
                {
                    if self.pause_key(key)
                        || self.action_key("attack", key)
                        || key == KeyCode::Enter
                    {
                        if let Some(game) = &mut self.game {
                            skip_death_reveal(game);
                        }
                        self.input.clear_edges();
                    }
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
            match self.screen {
                Screen::Pause => self.screen(Screen::Playing),
                Screen::Finished if self.editor_play => self.screen(Screen::Editor),
                Screen::Finished if self.is_hub() => {
                    self.open_hub(spelunky_core::hub::HubKind::Title)
                }
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
                match self.editor.load(&name, &contents) {
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
                        let kind = if self
                            .game
                            .as_ref()
                            .is_some_and(|game| game.flags.contains("hub_sun"))
                        {
                            spelunky_core::hub::HubKind::Sun
                        } else if self
                            .game
                            .as_ref()
                            .is_some_and(|game| game.flags.contains("hub_moon"))
                        {
                            spelunky_core::hub::HubKind::Moon
                        } else {
                            spelunky_core::hub::HubKind::Stars
                        };
                        self.open_hub(kind);
                    } else if self.is_custom() {
                        self.screen(Screen::Custom)
                    } else {
                        self.start_game(false)
                    }
                }
                1 if self.is_hub() => self.open_hub(spelunky_core::hub::HubKind::Title),
                1 => self.screen(Screen::Title),
                _ => self.screen(Screen::Editor),
            },
            Screen::Shortcuts => {
                let levels = self.shortcuts();
                if let Some(&level) = levels.get(self.selection) {
                    self.start_game(false);
                    if let Some(game) = &mut self.game {
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
                    self.screen(Screen::Title)
                }
            }
            _ => self.screen(Screen::Title),
        }
    }
    fn play_custom(&mut self, name: &str, editor: bool) {
        let Some(contents) = self.store.data.custom_levels.get(name) else {
            return;
        };
        match spelunky_core::level::Level::parse(contents) {
            Ok(level) => {
                let mut game = Game::new(wall_seed(), self.store.data.settings.rules);
                hydrate_run(&mut game, &self.store.data);
                load_custom(&mut game, &level, name, "");
                self.game = Some(game);
                self.editor_play = editor;
                self.recorded = false;
                self.finish_tick = None;
                self.screen(Screen::Playing)
            }
            Err(error) => self.toast(error),
        }
    }
    fn import_level(&mut self, name: String, contents: String) {
        match spelunky_core::level::Level::parse(&contents) {
            Ok(_) => {
                let name = level_name(&name);
                if let Err(error) = self.editor.load(&name, &contents) {
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
        match command {
            EditorCommand::None => {}
            EditorCommand::Back => self.screen(Screen::Title),
            EditorCommand::Import => self.import_dialog(),
            EditorCommand::Save { name, text } => {
                self.store
                    .data
                    .custom_levels
                    .insert(level_name(&name), text);
                match self.store.save() {
                    Ok(()) => self.editor.message(format!("SAVED {name}")),
                    Err(error) => self.editor.message(error),
                }
            }
            EditorCommand::Export { name, text } => match crate::editor_io::export(&name, &text) {
                Ok(()) => self.editor.message(format!("EXPORTED {name}")),
                Err(error) => self.editor.message(error),
            },
            EditorCommand::Play(level) => {
                let mut game = Game::new(wall_seed(), self.store.data.settings.rules);
                hydrate_run(&mut game, &self.store.data);
                load_custom(&mut game, &level, self.editor.level_name(), "");
                self.game = Some(game);
                self.editor_play = true;
                self.recorded = false;
                self.finish_tick = None;
                self.screen(Screen::Playing)
            }
        }
    }
    fn finish_record(&mut self) {
        let Some(game) = self.game.as_ref() else {
            return;
        };
        if self.recorded {
            return;
        }
        self.recorded = true;
        self.finish_tick = Some(game.tick);
        if self.editor_play
            || game.flags.contains("custom")
            || game.flags.contains("tutorial")
            || game.flags.contains("hub")
        {
            return;
        }
        let record = Record {
            seed: game.seed,
            rules: game.rules,
            score: game.player.money,
            level: game.world.level,
            ticks: game.tick,
            won: game.state == RunState::Won,
            kills: game.data.get("kills").copied().unwrap_or(0.).max(0.) as u32,
            rescues: game.data.get("damsels").copied().unwrap_or(0.).max(0.) as u32,
        };
        self.store
            .data
            .unlocks
            .extend(game.flags.iter().filter(|s| persistent_flag(s)).cloned());
        self.store
            .data
            .unlocks
            .extend(spelunky_core::hub::record_unlocks(
                game.player.money,
                (game.state == RunState::Won).then_some(game.tick as f64 / 30.),
                game.data.get("kills").copied().unwrap_or(0.).max(0.) as u32,
                game.data.get("damsels").copied().unwrap_or(0.).max(0.) as u32,
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
                "tutorial" => self.start_game(true),
                "shortcut5" | "shortcut9" | "shortcut13" => {
                    let level = destination
                        .trim_start_matches("shortcut")
                        .parse::<u32>()
                        .unwrap_or(1);
                    self.start_game(false);
                    if let Some(game) = &mut self.game {
                        game.data.insert("next_level".into(), f64::from(level));
                        game.next_level();
                    }
                    self.reset_timing();
                }
                _ => self.start_game(false),
            }
            return;
        }
        if game.flags.contains("tutorial") {
            prepare_tutorial_return(game);
            self.open_hub(spelunky_core::hub::HubKind::Title);
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
            if let Some(donation) = TransitionDonation::begin(game) {
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
        let now = self.now();
        let host_frame = self.timing.begin_frame(now);
        if self.focused && !self.occluded {
            self.input.update_gamepad();
        }
        let buttons = self.input.ui_pressed();
        if !buttons.is_empty() {
            self.audio.unlock();
        }
        for button in buttons {
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
                    .is_some_and(|game| game.state == RunState::Dead)
            {
                if matches!(
                    button,
                    UiButton::Confirm | UiButton::Cancel | UiButton::Pause
                ) {
                    if let Some(game) = &mut self.game {
                        skip_death_reveal(game);
                    }
                    self.input.clear_edges();
                }
                continue;
            }
            if button == UiButton::Pause && self.screen == Screen::Playing {
                self.screen(Screen::Pause);
                continue;
            }
            if self.screen != Screen::Playing {
                self.key(
                    match button {
                        UiButton::Up => KeyCode::ArrowUp,
                        UiButton::Down => KeyCode::ArrowDown,
                        UiButton::Left => KeyCode::ArrowLeft,
                        UiButton::Right => KeyCode::ArrowRight,
                        UiButton::Confirm => KeyCode::Enter,
                        UiButton::Cancel | UiButton::Pause => KeyCode::Escape,
                    },
                    None,
                )
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
                if self
                    .game
                    .as_ref()
                    .is_some_and(|game| game.state == RunState::Dead)
                {
                    input = spelunky_core::InputFrame::default();
                }
                if let Some(game) = &mut self.game {
                    game.step(&input);
                    self.timing.history_ready = true;
                    for event in &game.events {
                        match event {
                            GameEvent::Sound { name, pos } => self
                                .audio
                                .play(name, ((pos.x - game.camera.x) / 320. * 2. - 1.) as f32),
                            GameEvent::Message(message) => {
                                self.message = message.clone();
                                self.message_until = now + 5.;
                                self.message_simulated = game
                                    .data
                                    .get("message_timer")
                                    .is_some_and(|timer| *timer > 0.);
                            }
                            _ => {}
                        }
                    }
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
                    if self.game.as_ref().is_some_and(death_reveal_ready) {
                        self.screen(Screen::Finished);
                        break;
                    }
                } else if state == Some(RunState::Won) {
                    self.finish_record();
                    self.screen(Screen::Finished);
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
                    game.step(&spelunky_core::InputFrame::default());
                    self.timing.history_ready = true;
                    for event in &game.events {
                        if let GameEvent::Sound { name, pos } = event {
                            self.audio
                                .play(name, ((pos.x - game.camera.x) / 320. * 2. - 1.) as f32);
                        }
                    }
                }
            }
        }
        if let Some(game) = &mut self.game {
            consume_onboarding(game, &mut self.store.data);
        }
        if let Some(game) = &self.game {
            let previous = self.store.data.unlocks.clone();
            let previous_challenges = self.store.data.challenge_records.clone();
            let previous_tunnels = self.store.data.tunnel_remaining.clone();
            self.store
                .data
                .tunnel_remaining
                .extend(donation_ui::progress(game));
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
        let music = if self.screen == Screen::Title {
            "title"
        } else if matches!(
            self.screen,
            Screen::Playing | Screen::Pause | Screen::Finished | Screen::Donation
        ) {
            self.game.as_ref().map(game_music).unwrap_or("title")
        } else {
            "title"
        };
        self.audio.music(music);
        self.audio.update();
    }
    fn draw(&self) -> Vec<DrawQuad> {
        let Some(atlas) = &self.atlas else {
            return vec![];
        };
        let mut quads = if matches!(
            self.screen,
            Screen::Playing | Screen::Pause | Screen::Finished | Screen::Donation
        ) {
            self.game
                .as_ref()
                .map(|g| {
                    scene::game_scene(
                        g,
                        self.timing.alpha(),
                        self.store.data.settings.pixel_snap,
                        atlas,
                    )
                })
                .unwrap_or_default()
        } else {
            let mut q = vec![];
            rect(&mut q, [0., 0., 320., 240.], [0.055, 0.035, 0.03, 1.]);
            q
        };
        match self.screen {
            Screen::Playing => {
                if let Some(game) = self
                    .game
                    .as_ref()
                    .filter(|game| game.state == RunState::Dead)
                {
                    text(&mut quads, atlas, "GAME OVER", 88., 48., 1., GOLD);
                    if game.data.get("death_ticks").copied().unwrap_or(0.) >= 50. {
                        text(&mut quads, atlas, "FINAL SCORE:", 88., 80., 0.75, GOLD);
                    }
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
                            self.finish_tick.unwrap_or(game.tick) as f64 / 30.
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
                            record.ticks as f64 / 30.,
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
        if !self.message.is_empty() && message_visible {
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
        quads
    }
    #[cfg(target_arch = "wasm32")]
    fn publish_test_snapshot(&self) {
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
        let snapshot = test_snapshot(
            self.screen,
            self.game.as_ref(),
            &self.timing,
            self.store.data.settings.pixel_snap,
        );
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
            let command = self.editor.pointer(
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
    timing: &HostTiming,
    pixel_snap: bool,
) -> serde_json::Value {
    let Some(game) = game else {
        return serde_json::json!({ "screen": format!("{screen:?}"), "player": null });
    };
    let alpha = if game.flags.contains("render_discontinuity") {
        1.
    } else {
        timing.alpha()
    };
    let interpolate = |previous: spelunky_core::Vec2, current: spelunky_core::Vec2| {
        if previous.distance(current) > 64. {
            current
        } else {
            previous.lerp(current, alpha)
        }
    };
    let player = &game.player;
    let rendered = interpolate(player.previous_pos, player.pos);
    let camera = interpolate(game.previous_camera, game.camera);
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
        "deathTicks": game.data.get("death_ticks").copied().unwrap_or(0.),
        "money": player.money,
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
        "rendered": { "x": rendered.x, "y": rendered.y, "screenX": screen_x, "screenY": screen_y },
        "camera": {
            "x": game.camera.x, "y": game.camera.y,
            "previousX": game.previous_camera.x, "previousY": game.previous_camera.y,
            "renderedX": camera.x, "renderedY": camera.y,
        },
    })
}
fn game_music(game: &Game) -> &'static str {
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
    let character = if game.flags.contains("character_damsel")
        && game.flags.contains("unlock_damsel")
    {
        spelunky_core::characters::Character::Damsel
    } else if game.flags.contains("character_tunnelman") && game.flags.contains("unlock_tunnel_man")
    {
        spelunky_core::characters::Character::TunnelMan
    } else {
        spelunky_core::characters::Character::Explorer
    };
    spelunky_core::characters::apply(game, character);
}
fn consume_onboarding(game: &mut Game, data: &mut crate::storage::SaveData) {
    if game.flags.remove("onboarding_consumed") {
        data.first_time = false;
    }
}
fn prepare_tutorial_return(game: &mut Game) {
    game.flags.remove("first_time");
    game.flags.insert("onboarding_consumed".into());
    game.flags.insert("title_start:3".into());
    game.flags.insert("reset_run_on_title".into());
}
fn death_reveal_ready(game: &Game) -> bool {
    game.state == RunState::Dead && game.data.get("death_ticks").copied().unwrap_or(0.) >= 100.
}
fn skip_death_reveal(game: &mut Game) {
    if game.state == RunState::Dead {
        let elapsed = game.data.entry("death_ticks".into()).or_default();
        *elapsed = elapsed.max(100.);
    }
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
        game.state = RunState::Won;
        return CustomExit::Finished;
    }
    let Some(level) = custom_level_source(levels, &target)
        .and_then(|source| spelunky_core::level::Level::parse(source).ok())
    else {
        return CustomExit::Missing(target);
    };
    let previous = game.world.name.clone();
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
fn wall_seed() -> u64 {
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(1)
    }
    #[cfg(target_arch = "wasm32")]
    {
        (js_sys::Date::now() * 1000.) as u64
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
        match event {
            WindowEvent::CloseRequested => {
                let _ = self.store.save();
                event_loop.exit()
            }
            WindowEvent::Resized(size) => {
                if let Some(renderer) = &mut self.renderer {
                    renderer.resize(size.width, size.height);
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
                    if matches!(self.screen, Screen::Playing | Screen::Donation) {
                        self.input.key(key, pressed, event.repeat);
                    }
                    if pressed && !event.repeat {
                        if key == KeyCode::F2 && self.screen == Screen::Title {
                            self.screen(Screen::Editor)
                        } else {
                            self.key(key, event.text.as_deref())
                        }
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
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
                self.update();
                let quads = self.draw();
                #[cfg(target_arch = "wasm32")]
                self.publish_test_snapshot();
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
            event_loop.set_control_flow(ControlFlow::Poll);
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
fn logical_pointer(x: f64, y: f64, width: u32, height: u32) -> [f32; 2] {
    let scale = (width as f64 / 320.).min(height as f64 / 240.);
    if scale <= 0. {
        return [-1., -1.];
    }
    [
        ((x - (width as f64 - 320. * scale) * 0.5) / scale) as f32,
        ((y - (height as f64 - 240. * scale) * 0.5) / scale) as f32,
    ]
}
#[cfg(test)]
mod tests {
    use super::*;
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
        let snapshot = test_snapshot(Screen::Playing, Some(&game), &timing, false);
        assert_eq!(snapshot["screen"], "Playing");
        assert_eq!(snapshot["seed"], "42");
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
        let paused = test_snapshot(Screen::Pause, Some(&game), &timing, false);
        assert_eq!(paused["alpha"], 1.);
        assert_eq!(paused["rendered"]["x"], paused["player"]["x"]);
        timing.history_ready = true;
        game.flags.insert("render_discontinuity".into());
        let teleported = test_snapshot(Screen::Playing, Some(&game), &timing, false);
        assert_eq!(teleported["alpha"], 1.);
        assert_eq!(teleported["rendered"]["x"], teleported["player"]["x"]);
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
        let snapshot = test_snapshot(Screen::Playing, Some(&game), &HostTiming::default(), false);
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
        let uninitialized =
            test_snapshot(Screen::Playing, Some(&game), &HostTiming::default(), false);
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
    fn death_reveal_keeps_corpse_and_world_simulating_for_two_fifty_tick_stages() {
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
        assert!(!death_reveal_ready(&game));
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
            assert_eq!(death_reveal_ready(&game), elapsed >= 100);
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
    fn entering_finished_cannot_consume_the_same_presentation_frame_twice() {
        let mut game = Game::shell(42, spelunky_core::RulesPreset::Classic);
        game.player.health = 0;
        game.step(&spelunky_core::InputFrame::default());
        game.data.insert("death_ticks".into(), 99.);
        let tick_before_reveal = game.tick;
        let mut timing = HostTiming::default();
        timing.reset(1.);
        let now = 1. + 1. / 30.;
        let sampled = timing.begin_frame(now);
        for _ in 0..timing.advance(sampled).ticks {
            game.step(&spelunky_core::InputFrame::default());
        }
        assert!(death_reveal_ready(&game));
        assert_eq!(game.tick, tick_before_reveal + 1);
        // App::screen(Finished) resets timing between the Playing and Finished
        // update branches, which both see the frame sampled before the change.
        timing.reset(now);
        for _ in 0..timing.advance(sampled).ticks {
            game.step(&spelunky_core::InputFrame::default());
        }
        assert_eq!(game.tick, tick_before_reveal + 1);
        assert_eq!(timing.alpha(), 1.);
    }

    #[test]
    fn skipping_death_reveal_preserves_the_corpse_and_only_applies_after_death() {
        let mut game = Game::shell(42, spelunky_core::RulesPreset::Classic);
        skip_death_reveal(&mut game);
        assert!(!game.data.contains_key("death_ticks"));
        game.player.health = 0;
        game.step(&spelunky_core::InputFrame::default());
        let body = (game.player.pos, game.player.vel);
        let tick = game.tick;
        skip_death_reveal(&mut game);
        assert!(death_reveal_ready(&game));
        assert_eq!((game.player.pos, game.player.vel), body);
        assert_eq!(game.tick, tick);
        game.step(&spelunky_core::InputFrame::default());
        skip_death_reveal(&mut game);
        assert_eq!(game.data["death_ticks"], 101.);
    }

    #[test]
    fn tutorial_completion_returns_to_the_source_hub_door_and_resets_run_inventory() {
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
        prepare_tutorial_return(&mut game);
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
        prepare_tutorial_return(&mut completed);
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
            spelunky_core::characters::Character::Explorer
        );
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
            game.state = RunState::Dead;
            assert_eq!(game_music(&game), "");
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
        assert_eq!(gun.value("ammo"), 3.);
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
    fn ordinary_custom_exit_follows_links_and_reports_missing_or_final_rooms() {
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
        let original = game.state_hash();
        assert_eq!(
            custom_exit(&mut game, &levels, false),
            CustomExit::Missing("missing.lvl".into())
        );
        assert_eq!(game.state_hash(), original);
        game.flags.remove("custom_level:missing.lvl");
        assert_eq!(custom_exit(&mut game, &levels, false), CustomExit::Finished);
        assert_eq!(game.state, RunState::Won);
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
    fn pointer_letterbox_coordinates_match_renderer() {
        assert_eq!(logical_pointer(240., 0., 1920, 1080), [0., 0.]);
        assert_eq!(logical_pointer(1680., 1080., 1920, 1080), [320., 240.]);
        assert_eq!(logical_pointer(0., 100., 800, 800), [0., 0.]);
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
}
