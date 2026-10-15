//! Complete literal scrResetHighscores.gml; real storage remains required.
use crate::source_generation_clear_globals::Globals;

/// Primitive engine table operations, not a prefilled host score profile.
/// Each method runs at its reached source statement and preserves its own
/// partial effects on error. No method has a default implementation.
pub(crate) trait Highscores: Globals {
    fn highscore_clear(&mut self) -> Result<(), Self::Error>;
    fn highscore_add(&mut self, name: &'static str, value: f64) -> Result<(), Self::Error>;
    fn highscore_value(&mut self, source_index: u8) -> Result<f64, Self::Error>;
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Error<E> {
    Capability(E),
    UnsupportedScore { index: u8, value: f64 },
    UnsupportedMaximum { name: &'static str, value: f64 },
    UnsupportedAdd { name: &'static str, value: f64 },
}

/// Native registered highscore_value563da4 returns signed32 EAX through
/// FILD DWORD563dc3 then FSTP QWORD563dc6. Check that exact domain at the
/// reached query; do not round/clamp fractional or out-of-range providers.
pub(crate) fn score<H: Highscores>(h: &mut H, index: u8) -> Result<f64, Error<H::Error>> {
    let value = h.highscore_value(index).map_err(Error::Capability)?;
    if value.is_finite()
        && value.fract() == 0.0
        && !(value == 0.0 && value.is_sign_negative())
        && value >= f64::from(i32::MIN)
        && value <= f64::from(i32::MAX)
    {
        Ok(value)
    } else {
        Err(Error::UnsupportedScore { index, value })
    }
}

fn maximum<E>(name: &'static str, value: f64) -> Result<i32, Error<E>> {
    if value.is_finite()
        && value.fract() == 0.0
        && value >= f64::from(i32::MIN)
        && value <= f64::from(i32::MAX)
    {
        Ok(value as i32)
    } else {
        Err(Error::UnsupportedMaximum { name, value })
    }
}

/// Qualification of this script's ordinary exact-integer arithmetic. Preserve
/// left-to-right base+maximum then +1 in exact signed64 intermediates; the
/// final add argument must be signed32. Other native FISTP/overflow domains
/// need a separate primitive and are explicitly unsupported, never clamped.
fn add_argument<E>(name: &'static str, base: i64, maximum: i32) -> Result<f64, Error<E>> {
    let first = base + i64::from(maximum);
    let second = first + 1;
    i32::try_from(second)
        .map(f64::from)
        .map_err(|_| Error::UnsupportedAdd {
            name,
            value: second as f64,
        })
}

pub(crate) fn run<H: Highscores>(h: &mut H) -> Result<(), Error<H::Error>> {
    h.highscore_clear().map_err(Error::Capability)?;
    h.highscore_add("MONEY", 9_000_000.0)
        .map_err(Error::Capability)?;
    h.highscore_add("TIME", 8_000_000.0)
        .map_err(Error::Capability)?;
    h.highscore_add("KILLS", 7_000_000.0)
        .map_err(Error::Capability)?;
    h.highscore_add("SAVES", 6_000_000.0)
        .map_err(Error::Capability)?;
    h.highscore_add("PLAYS", 5_000_000.0)
        .map_err(Error::Capability)?;
    h.highscore_add("WINS", 4_000_000.0)
        .map_err(Error::Capability)?;
    h.highscore_add("DEATHS", 3_000_000.0)
        .map_err(Error::Capability)?;
    let raw_maximum1 = h.number("tunnel1Max").map_err(Error::Capability)?;
    let maximum1 = maximum("tunnel1Max", raw_maximum1)?;
    let argument1 = add_argument("TUNNEL1", 2_000_000, maximum1)?;
    h.highscore_add("TUNNEL1", argument1)
        .map_err(Error::Capability)?;
    let raw_maximum2 = h.number("tunnel2Max").map_err(Error::Capability)?;
    let maximum2 = maximum("tunnel2Max", raw_maximum2)?;
    let argument2 = add_argument("TUNNEL2", 1_000_000, maximum2)?;
    h.highscore_add("TUNNEL2", argument2)
        .map_err(Error::Capability)?;
    h.highscore_add("MINIGAMES", 0.0)
        .map_err(Error::Capability)?;
    let tunnel1 = score(h, 8)? - 2_000_000.0;
    h.set_number("tunnel1", tunnel1)
        .map_err(Error::Capability)?;
    let tunnel2 = score(h, 9)? - 1_000_000.0;
    h.set_number("tunnel2", tunnel2)
        .map_err(Error::Capability)?;
    Ok(())
}
