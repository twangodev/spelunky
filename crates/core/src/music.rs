//! Literal source music scripts and oGame Key M ownership.
use crate::{Area, Game, GameEvent, SourceMusicCommand};

pub const RESOURCES: [&str; 8] = [
    "musTitle",
    "musCave",
    "musLush",
    "musIce",
    "musTemple",
    "musBoss",
    "musVictory",
    "musCredits",
];

/// Authored membership does not reactivate a paused/deactivated receiver.
pub fn has_active_game(game: &Game) -> bool {
    if game.flags.contains("source_game_inactive") {
        return false;
    }
    let mut actual = game
        .world
        .entities
        .iter()
        .chain(game.dispatcher.hidden_entities().iter())
        .filter(|e| e.alive && e.object == "oGame")
        .peekable();
    if actual.peek().is_some() {
        return actual.any(|e| e.engine_active);
    }
    level_room(game)
        || matches!(
            game.world.name.as_str(),
            "rIntro" | "rTitle" | "rHighscores" | "rSun" | "rMoon" | "rStars" | "rEnd"
        )
}

// Cinematic dispatch temporarily owns game.ending; retained custom flags must
// not make its authored room a level while that state is borrowed out.
fn level_room(game: &Game) -> bool {
    if game.state == crate::RunState::Ending
        && matches!(
            game.world.name.as_str(),
            "rEnd" | "rEnd2" | "rEnd3" | "rEndCustom" | "rCredits1" | "rCredits2"
        )
    {
        false
    } else {
        // Custom display filenames may begin with r; only real cinematic
        // identities override the retained custom flags during borrowed Step.
        game.is_level()
    }
}

fn emit_play(game: &mut Game, name: &str, looping: bool, if_idle: bool) {
    game.source_music.pending.push(SourceMusicCommand::Play {
        name: name.into(),
        looping,
        if_idle,
    });
    game.events.push(GameEvent::MusicPlay {
        name: name.into(),
        looping,
        if_idle,
    });
}

/// playMusic checks enabled; actual resource-idle ownership belongs to the host.
pub fn play(game: &mut Game, name: &str, looping: bool) {
    if game.source_music.enabled {
        emit_play(game, name, looping, true);
    }
}

/// oTitle Alarm3 uses playSound, whose unused second argument cannot loop.
pub(crate) fn title_alarm_play(game: &mut Game) {
    emit_play(game, "musTitle", false, false);
}

pub fn stop_all(game: &mut Game) {
    game.source_music.pending.push(SourceMusicCommand::StopAll);
    game.events.push(GameEvent::MusicStopAll);
}

fn boss_active(game: &Game) -> bool {
    game.player.data.get("active").copied().unwrap_or(1.0) != 0.0
}

/// KeyM/fade select gameplay levelType, deliberately ignoring custom music.
pub(crate) fn level_track(game: &Game) -> Option<&'static str> {
    if !level_room(game) {
        return None;
    }
    match game.world.area {
        Area::Jungle => Some("musLush"),
        Area::Ice => Some("musIce"),
        Area::Temple => Some("musTemple"),
        Area::Boss if !boss_active(game) => None,
        Area::Boss => Some("musBoss"),
        Area::Mines => Some("musCave"),
    }
}

/// The host may call this once for an active source UI room with no Game.step.
pub fn key_pressed(game: &mut Game) {
    if !has_active_game(game) {
        return;
    }
    game.source_music.enabled = !game.source_music.enabled;
    if !game.source_music.enabled {
        stop_all(game);
    } else if let Some(track) = level_track(game) {
        play(game, track, true);
    } else if game.world.name == "rTitle" {
        play(game, "musTitle", true);
    }
}

pub(crate) fn start_music(game: &mut Game) {
    if !game.source_music.enabled {
        return;
    }
    let track = if level_room(game) && game.is_load_level() {
        match game.data.get("custom_music").copied().unwrap_or(0.0) {
            1.0 => Some("musLush"),
            2.0 => Some("musIce"),
            3.0 => Some("musTemple"),
            4.0 if !boss_active(game) => None,
            4.0 => Some("musBoss"),
            _ => Some("musCave"),
        }
    } else if level_room(game) {
        level_track(game)
    } else if matches!(game.world.name.as_str(), "rSun" | "rMoon" | "rStars") {
        Some("musBoss")
    } else {
        None
    };
    if let Some(track) = track {
        play(game, track, true);
        game.set_music_frequency(track, 44_100);
        game.data.insert("music_frequency".into(), 44_100.0);
    }
}

pub(crate) fn room_create(game: &mut Game) {
    if !level_room(game) || game.data.contains_key("source_music_room_created") {
        return;
    }
    game.data.insert("source_music_room_created".into(), 1.0);
    game.flags.remove("music_fade");
    game.data.insert("music_fade_timer".into(), 0.0);
    if let Some(track) = level_track(game) {
        game.data.insert(
            "music_frequency".into(),
            game.source_music.frequency(track) as f64,
        );
    }
    start_music(game);
}
