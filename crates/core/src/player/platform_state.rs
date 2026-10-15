//! The source platform state is independent of its dead/stunned flags.
//! Keep it only when the host's extra Stunned/Dead enums hide that value.
use crate::{Game, GenerationRevision, Player, PlayerState};

const KEY: &str = "source_platform_state";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum State {
    Standing,
    Running,
    Ducking,
    LookingUp,
    Climbing,
    Jumping,
    Falling,
    Dying,
    Hanging,
    DuckToHang,
}
impl State {
    fn code(self) -> f64 {
        match self {
            Self::Standing => 10.0,
            Self::Running => 11.0,
            Self::Ducking => 12.0,
            Self::LookingUp => 13.0,
            Self::Climbing => 14.0,
            Self::Jumping => 15.0,
            Self::Falling => 16.0,
            Self::Dying => 17.0,
            Self::Hanging => 23.0,
            Self::DuckToHang => 24.0,
        }
    }
    fn decode(code: f64) -> Option<Self> {
        Some(match code {
            10.0 => Self::Standing,
            11.0 => Self::Running,
            12.0 => Self::Ducking,
            13.0 => Self::LookingUp,
            14.0 => Self::Climbing,
            15.0 => Self::Jumping,
            16.0 => Self::Falling,
            17.0 => Self::Dying,
            23.0 => Self::Hanging,
            24.0 => Self::DuckToHang,
            _ => return None,
        })
    }
    fn from_host(state: PlayerState) -> Option<Self> {
        Some(match state {
            PlayerState::Standing => Self::Standing,
            PlayerState::Running => Self::Running,
            PlayerState::Ducking => Self::Ducking,
            PlayerState::LookingUp => Self::LookingUp,
            PlayerState::Climbing => Self::Climbing,
            PlayerState::Jumping => Self::Jumping,
            PlayerState::Falling => Self::Falling,
            PlayerState::Hanging => Self::Hanging,
            PlayerState::Stunned | PlayerState::Dead => return None,
        })
    }
    fn host(self) -> Option<PlayerState> {
        Some(match self {
            Self::Standing => PlayerState::Standing,
            Self::Running => PlayerState::Running,
            Self::Ducking | Self::DuckToHang => PlayerState::Ducking,
            Self::LookingUp => PlayerState::LookingUp,
            Self::Climbing => PlayerState::Climbing,
            Self::Jumping => PlayerState::Jumping,
            Self::Falling => PlayerState::Falling,
            Self::Hanging => PlayerState::Hanging,
            // Declared in characterCreateEvent, with no ordinary Player writer.
            Self::Dying => return None,
        })
    }
    fn grounded(self) -> bool {
        matches!(
            self,
            Self::Standing | Self::Running | Self::Ducking | Self::LookingUp
        )
    }
}

pub(super) fn canonical(game: &Game) -> bool {
    game.generation_revision == GenerationRevision::SourceClassicV2
}

pub(super) fn read(player: &Player, canonical: bool) -> Option<State> {
    if canonical && super::value(player, "duck_to_hang", 0.0) != 0.0 {
        return Some(State::DuckToHang);
    }
    Some(match player.state {
        PlayerState::Standing => State::Standing,
        PlayerState::Running => State::Running,
        PlayerState::Ducking => State::Ducking,
        PlayerState::LookingUp => State::LookingUp,
        PlayerState::Climbing => State::Climbing,
        PlayerState::Jumping => State::Jumping,
        PlayerState::Falling => State::Falling,
        PlayerState::Hanging => State::Hanging,
        PlayerState::Stunned | PlayerState::Dead => {
            if !canonical {
                return None;
            }
            // Older saves carry only two class bits. They cannot identify a
            // Duck/Run or Jump/Fall subtype; never invent one from those bits.
            return player.data.get(KEY).copied().and_then(State::decode);
        }
    })
}

pub(super) fn remember(player: &mut Player, state: State) {
    super::set(player, KEY, state.code());
    super::set(
        player,
        "source_platform_grounded",
        f64::from(state.grounded()),
    );
    super::set(
        player,
        "source_platform_climbing",
        f64::from(state == State::Climbing),
    );
}

pub(super) fn capture(player: &mut Player, canonical: bool) {
    if canonical {
        if let Some(state) = read(player, true) {
            remember(player, state);
        }
    }
}

pub(super) fn assign(player: &mut Player, state: State, canonical: bool) {
    debug_assert!(state != State::Dying, "no ordinary Player DYING writer");
    if canonical {
        // A literal state=DUCKING (12) replaces DUCKTOHANG (24); callers
        // assigning the DTH state must request it explicitly.
        if state == State::DuckToHang {
            super::set(player, "duck_to_hang", 1.0);
        } else if super::value(player, "duck_to_hang", 0.0) != 0.0 {
            super::set(player, "duck_to_hang", 0.0);
        }
        if matches!(player.state, PlayerState::Stunned | PlayerState::Dead) {
            remember(player, state);
            return;
        }
    }
    if let Some(host) = state.host() {
        player.state = host;
    }
    if canonical && player.data.contains_key(KEY) {
        remember(player, state);
    }
}

pub(super) fn recovered(player: &mut Player, canonical: bool) {
    if player.state == PlayerState::Stunned {
        player.state = if canonical {
            read(player, true)
                .and_then(State::host)
                .unwrap_or(PlayerState::Standing)
        } else {
            PlayerState::Standing
        };
    }
}

pub(super) fn grounded(player: &Player, canonical: bool) -> bool {
    if canonical {
        if let Some(state) = read(player, true) {
            return state.grounded();
        }
    }
    // Compatibility fallback: only the known class, never an invented subtype.
    super::source_grounded(player)
}

// These two historical Player preludes used the raw enum/class cache,
// unlike their source_grounded item/character callers (which exclude DTH).
pub(super) fn prelude_grounded(player: &Player, canonical: bool) -> bool {
    if canonical {
        grounded(player, true)
    } else {
        super::grounded_state(player.state)
            || (matches!(player.state, PlayerState::Stunned | PlayerState::Dead)
                && super::value(player, "source_platform_grounded", 0.0) != 0.0)
    }
}

pub(crate) fn source_grounded(game: &Game) -> bool {
    grounded(&game.player, canonical(game))
}
pub(crate) fn source_airborne(game: &Game) -> bool {
    matches!(
        read(&game.player, canonical(game)),
        Some(State::Jumping | State::Falling)
    )
}
/// Source platform CLIMBING, independent of the dead/stunned presentation enum.
/// Used by character attachments as well as the physical source callbacks.
pub fn source_climbing(game: &Game) -> bool {
    let canonical = canonical(game);
    match read(&game.player, canonical) {
        Some(state) => state == State::Climbing,
        None => canonical && super::value(&game.player, "source_platform_climbing", 0.0) != 0.0,
    }
}
pub(crate) fn source_ducking(game: &Game) -> bool {
    matches!(read(&game.player, canonical(game)), Some(State::Ducking))
}
pub(crate) fn source_hanging(game: &Game) -> bool {
    matches!(read(&game.player, canonical(game)), Some(State::Hanging))
}
pub(crate) fn source_falling(game: &Game) -> bool {
    matches!(read(&game.player, canonical(game)), Some(State::Falling))
}

pub(crate) fn enter_stunned(game: &mut Game) {
    let canonical = canonical(game);
    let player = &mut game.player;
    capture(player, canonical);
    if canonical {
        super::set(player, "source_stunned", 1.0);
    }
    if !canonical || player.state != PlayerState::Dead {
        player.state = PlayerState::Stunned;
    }
}
pub(crate) fn enter_dead(game: &mut Game) {
    let canonical = canonical(game);
    capture(&mut game.player, canonical);
    game.player.state = PlayerState::Dead;
}
pub(crate) fn assign_platform(game: &mut Game, state: PlayerState) {
    let canonical = canonical(game);
    if let Some(state) = State::from_host(state) {
        assign(&mut game.player, state, canonical);
    } else {
        debug_assert!(false, "use independent stun/dead adapters for host flags");
    }
}
pub(crate) fn revive(game: &mut Game) {
    if game.player.state != PlayerState::Dead {
        return;
    }
    let canonical = canonical(game);
    let player = &mut game.player;
    if canonical {
        if let Some(state) = read(player, true).and_then(State::host) {
            player.state = state;
            return;
        }
    }
    // Existing historical/old-save adapter when no exact state was retained.
    player.state = if player.stun > 0 {
        PlayerState::Stunned
    } else {
        PlayerState::Falling
    };
}

#[cfg(test)]
#[path = "platform_state_tests.rs"]
mod tests;
