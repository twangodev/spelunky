//! Complete literal scrClearGlobals.gml, behind strict actual-global ports.
//! Uncalled preparation: the parent owns reached lifecycle/namespace binding.
//! This leaf never initializes a map, reads flags, allocates, or draws RNG.

/// Actual single-kind Source global namespace. Missing/wrong-kind GETs fail;
/// SET is performed immediately, with its real host projections, at this row.
/// No inferred defaults, whole-map replacement or deferred commit is allowed.
pub(crate) trait Globals {
    type Error;
    fn number(&mut self, name: &'static str) -> Result<f64, Self::Error>;
    fn set_number(&mut self, name: &'static str, value: f64) -> Result<(), Self::Error>;
    fn set_text(&mut self, name: &'static str, value: &'static str) -> Result<(), Self::Error>;
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Error<E> {
    Capability(E),
}

/// Source statements execute in order. A port error keeps every earlier write
/// and returns before later statements. isTunnelMan is read only at source104;
/// selected-character caches do not supply that value. The >=0.5 comparison is
/// the existing accepted v5 finite Source-real truth rule, without a bool cast.
/// Native nonfinite truth semantics are not certified by this leaf.
pub(crate) fn run<G: Globals>(globals: &mut G) -> Result<(), Error<G::Error>> {
    // scrClearGlobals.gml:26
    globals
        .set_number("titleStart", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:27
    globals
        .set_number("ghostExists", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:29
    globals
        .set_number("gameStart", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:30
    globals
        .set_number("entityGen", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:32
    globals
        .set_number("drawHUD", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:34
    globals
        .set_number("collect", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:35
    globals
        .set_number("collectCounter", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:37
    globals.set_number("shake", 0.).map_err(Error::Capability)?;
    // scrClearGlobals.gml:38
    globals
        .set_number("shakeToggle", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:39
    globals
        .set_number("yviewPrev", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:40
    globals.set_number("xview", 0.).map_err(Error::Capability)?;
    // scrClearGlobals.gml:41
    globals.set_number("yview", 0.).map_err(Error::Capability)?;
    // scrClearGlobals.gml:43
    globals
        .set_number("checkWater", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:44
    globals
        .set_number("waterCounter", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:46
    globals
        .set_text("pickupItem", "")
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:48
    globals
        .set_number("crapsPoint", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:50
    globals
        .set_number("bloodLevel", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:52
    globals
        .set_number("noDarkLevel", 1.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:55
    globals
        .set_number("probDarkLevel", 12.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:56
    globals
        .set_number("probSnakePit", 8.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:57
    globals
        .set_number("probCemetary", 10.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:58
    globals
        .set_number("probLake", 10.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:59
    globals
        .set_number("probYetiLair", 6.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:60
    globals
        .set_number("probAlien", 10.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:61
    globals
        .set_number("probSacPit", 8.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:64
    globals
        .set_number("thiefLevel", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:65
    globals
        .set_number("murderer", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:66
    globals.set_number("favor", 0.).map_err(Error::Capability)?;
    // scrClearGlobals.gml:67
    globals
        .set_number("kaliGift", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:68
    globals
        .set_number("kaliPunish", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:70
    globals
        .set_number("madeUdjatEye", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:71
    globals
        .set_number("genUdjatEye", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:72
    globals
        .set_number("madeMarketEntrance", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:73
    globals
        .set_number("genMarketEntrance", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:74
    globals
        .set_number("marketChance", 160.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:75
    globals
        .set_number("marketEntrance", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:76
    globals
        .set_number("madeBlackMarket", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:77
    globals
        .set_number("genBlackMarket", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:78
    globals
        .set_number("madeMoai", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:79
    globals
        .set_number("madeGoldEntrance", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:80
    globals
        .set_number("genGoldEntrance", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:81
    globals
        .set_number("goldChance", 160.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:82
    globals
        .set_number("goldEntrance", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:83
    globals
        .set_number("cityOfGold", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:85
    globals
        .set_number("hasUdjatEye", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:86
    globals
        .set_number("hasAnkh", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:87
    globals
        .set_number("hasCrown", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:88
    globals
        .set_number("hasKapala", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:89
    globals
        .set_number("hasStickyBombs", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:90
    globals
        .set_number("hasSpectacles", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:91
    globals
        .set_number("hasCompass", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:92
    globals
        .set_number("hasParachute", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:93
    globals
        .set_number("hasSpringShoes", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:94
    globals
        .set_number("hasSpikeShoes", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:95
    globals
        .set_number("hasJordans", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:96
    globals
        .set_number("hasNinjaSuit", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:97
    globals
        .set_number("hasCape", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:98
    globals
        .set_number("hasJetpack", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:99
    globals
        .set_number("hasGloves", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:100
    globals
        .set_number("hasMitt", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:101
    globals.set_number("exitX", 0.).map_err(Error::Capability)?;
    // scrClearGlobals.gml:102
    globals.set_number("exitY", 0.).map_err(Error::Capability)?;
    let tunnel = globals.number("isTunnelMan").map_err(Error::Capability)? >= 0.5;
    if tunnel {
        // scrClearGlobals.gml:106
        globals.set_number("plife", 2.).map_err(Error::Capability)?;
        // scrClearGlobals.gml:107
        globals.set_number("bombs", 0.).map_err(Error::Capability)?;
        // scrClearGlobals.gml:108
        globals.set_number("rope", 0.).map_err(Error::Capability)?;
    } else {
        // scrClearGlobals.gml:112
        globals.set_number("plife", 4.).map_err(Error::Capability)?;
        // scrClearGlobals.gml:113
        globals.set_number("bombs", 4.).map_err(Error::Capability)?;
        // scrClearGlobals.gml:114
        globals.set_number("rope", 4.).map_err(Error::Capability)?;
    }
    // scrClearGlobals.gml:116
    globals
        .set_number("flares", 3.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:117
    globals
        .set_number("arrows", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:119
    globals.set_number("money", 0.).map_err(Error::Capability)?;
    // scrClearGlobals.gml:120
    globals.set_number("time", 0.).map_err(Error::Capability)?;
    // scrClearGlobals.gml:121
    globals.set_number("kills", 0.).map_err(Error::Capability)?;
    // scrClearGlobals.gml:122
    globals
        .set_number("damsels", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:124
    globals.set_number("gold", 0.).map_err(Error::Capability)?;
    // scrClearGlobals.gml:125
    globals
        .set_number("goldbar", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:126
    globals
        .set_number("goldbars", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:127
    globals
        .set_number("nuggets", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:128
    globals
        .set_number("rubies", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:129
    globals
        .set_number("bigrubies", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:130
    globals
        .set_number("sapphires", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:131
    globals
        .set_number("bigsapphires", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:132
    globals
        .set_number("emeralds", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:133
    globals
        .set_number("bigemeralds", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:134
    globals
        .set_number("diamonds", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:135
    globals
        .set_number("scarabs", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:136
    globals.set_number("idols", 0.).map_err(Error::Capability)?;
    // scrClearGlobals.gml:137
    globals
        .set_number("skulls", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:138
    globals
        .set_number("xdamsels", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:139
    globals
        .set_number("xmoney", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:140
    globals.set_number("xtime", 0.).map_err(Error::Capability)?;
    // scrClearGlobals.gml:142
    globals.set_number("bats", 0.).map_err(Error::Capability)?;
    // scrClearGlobals.gml:143
    globals
        .set_number("snakes", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:144
    globals
        .set_number("spiders", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:145
    globals
        .set_number("skeletons", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:146
    globals.set_number("frogs", 0.).map_err(Error::Capability)?;
    // scrClearGlobals.gml:147
    globals
        .set_number("firefrogs", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:148
    globals
        .set_number("piranhas", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:149
    globals
        .set_number("mantraps", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:150
    globals.set_number("yetis", 0.).map_err(Error::Capability)?;
    // scrClearGlobals.gml:151
    globals
        .set_number("aliens", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:152
    globals.set_number("ufos", 0.).map_err(Error::Capability)?;
    // scrClearGlobals.gml:153
    globals
        .set_number("cavemen", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:154
    globals
        .set_number("hawkmen", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:155
    globals
        .set_number("monkeys", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:156
    globals
        .set_number("zombies", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:157
    globals
        .set_number("vampires", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:158
    globals
        .set_number("deadfish", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:159
    globals
        .set_number("alienbosses", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:160
    globals
        .set_number("giantspiders", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:161
    globals
        .set_number("yetikings", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:162
    globals
        .set_number("megamouths", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:163
    globals
        .set_number("tomblords", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:164
    globals
        .set_number("shopkeepers", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:165
    globals
        .set_number("damselsKilled", 0.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:168
    globals
        .set_number("currLevel", 1.)
        .map_err(Error::Capability)?;
    // scrClearGlobals.gml:169
    globals
        .set_number("levelType", 0.)
        .map_err(Error::Capability)?;
    Ok(())
}

#[cfg(test)]
#[path = "source_generation_clear_globals_tests.rs"]
mod tests;
