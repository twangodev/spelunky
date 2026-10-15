//! Full literal Objects/oGlobals.events/Create.xml, action603.
//! Required namespace/physical/highscore ports; no automatic Game constructor.
use crate::source_generation_clear_globals as clear;
use crate::source_generation_reset_highscores as reset;

/// One actual provider owns raw Source globals and reached engine operations.
/// Every method is required. attach must create the actual empty namespace,
/// never replace an already-attached namespace or prepopulate source values.
pub(crate) trait Capabilities: reset::Highscores {
    fn attach_globals_receiver(&mut self) -> Result<(), Self::Error>;
    /// Record globalvar registration only. No fabricated numeric default;
    /// source's immediately following joyid=1 supplies its first value.
    fn declare_globalvar(&mut self, name: &'static str) -> Result<(), Self::Error>;
    fn joystick_exists(&mut self, source_slot: u8) -> Result<bool, Self::Error>;
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Error<E> {
    Capability(E),
    Clear(clear::Error<E>),
    Score(reset::Error<E>),
    Reset(reset::Error<E>),
}

fn score<C: Capabilities>(c: &mut C, index: u8) -> Result<f64, Error<C::Error>> {
    reset::score(c, index).map_err(Error::Score)
}

/// Invoke once at the real oGlobals receiver's own Create ordinal. A failed
/// attach has no source statements; all later errors retain completed prefix
/// and any mutations made by the failed capability itself. No Player exists
/// merely because this helper was called, and no character preference is read.
pub(crate) fn create<C: Capabilities>(c: &mut C) -> Result<(), Error<C::Error>> {
    c.attach_globals_receiver().map_err(Error::Capability)?;
    // XML globalvar joyid; then its actual first assignment.
    c.declare_globalvar("joyid").map_err(Error::Capability)?;
    c.set_number("joyid", 1.0).map_err(Error::Capability)?;
    let joystick1 = c.joystick_exists(1).map_err(Error::Capability)?;
    let joystick2 = c.joystick_exists(2).map_err(Error::Capability)?;
    if !joystick1 & joystick2 {
        c.set_number("joyid", 2.0).map_err(Error::Capability)?;
    }

    c.set_number("levelSelect", 0.0)
        .map_err(Error::Capability)?;
    c.set_number("customLevel", 0.0)
        .map_err(Error::Capability)?;
    c.set_text("testLevel", "").map_err(Error::Capability)?;
    c.set_number("firstTime", 0.0).map_err(Error::Capability)?;
    c.set_number("usedShortcut", 0.0)
        .map_err(Error::Capability)?;
    c.set_number("darkLevel", 0.0).map_err(Error::Capability)?;
    c.set_number("lake", 0.0).map_err(Error::Capability)?;
    c.set_number("music", 1.0).map_err(Error::Capability)?;
    c.set_text("message", "").map_err(Error::Capability)?;
    c.set_text("message2", "").map_err(Error::Capability)?;
    c.set_number("messageTimer", 0.0)
        .map_err(Error::Capability)?;
    c.set_number("darknessLerp", 0.0)
        .map_err(Error::Capability)?;
    c.set_number("hadDarkLevel", 0.0)
        .map_err(Error::Capability)?;
    c.set_number("udjatBlink", 0.0).map_err(Error::Capability)?;
    c.set_number("udjatCounter", 0.0)
        .map_err(Error::Capability)?;
    c.set_number("cleanSolids", 0.0)
        .map_err(Error::Capability)?;

    c.set_number("tunnel1Max", 100_000.0)
        .map_err(Error::Capability)?;
    c.set_number("tunnel2Max", 200_000.0)
        .map_err(Error::Capability)?;
    c.set_number("tunnel3Max", 300_000.0)
        .map_err(Error::Capability)?;
    let tunnel1 = score(c, 8)? - 2_000_000.0;
    c.set_number("tunnel1", tunnel1)
        .map_err(Error::Capability)?;
    let tunnel2 = score(c, 9)? - 1_000_000.0;
    c.set_number("tunnel2", tunnel2)
        .map_err(Error::Capability)?;

    c.set_number("mini1", 0.0).map_err(Error::Capability)?;
    c.set_number("mini2", 0.0).map_err(Error::Capability)?;
    c.set_number("mini3", 0.0).map_err(Error::Capability)?;
    c.set_number("scoresStart", 0.0)
        .map_err(Error::Capability)?;
    c.set_number("isDamsel", 0.0).map_err(Error::Capability)?;
    c.set_number("isTunnelMan", 0.0)
        .map_err(Error::Capability)?;
    c.set_number("newMoney", 0.0).map_err(Error::Capability)?;
    c.set_number("newKills", 0.0).map_err(Error::Capability)?;
    c.set_number("newSaves", 0.0).map_err(Error::Capability)?;
    c.set_number("newTime", 0.0).map_err(Error::Capability)?;

    clear::run(c).map_err(Error::Clear)?;

    // Original GM8 AND is eager: every reached score read is a separate engine
    // query at this ordinal, including when the very first comparison is false.
    let s1 = score(c, 1)? <= 9_000_000.0;
    let s2 = score(c, 2)? <= 8_000_000.0;
    let s3 = score(c, 3)? <= 7_000_000.0;
    let s4 = score(c, 4)? <= 6_000_000.0;
    let s5 = score(c, 5)? <= 5_000_000.0;
    let s6 = score(c, 6)? <= 4_000_000.0;
    let s7 = score(c, 7)? <= 3_000_000.0;
    let s8 = score(c, 8)? <= 2_000_000.0;
    let s9 = score(c, 9)? <= 1_000_000.0;
    if s1 & s2 & s3 & s4 & s5 & s6 & s7 & s8 & s9 {
        reset::run(c).map_err(Error::Reset)?;
        c.set_number("firstTime", 1.0).map_err(Error::Capability)?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "source_generation_globals_create_tests.rs"]
mod tests;
