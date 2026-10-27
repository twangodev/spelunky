//! Campaign-level source integration sweep for the forced Udjat chest.
//!
//! Classic sets `genUdjatEye` unconditionally on Mines level 4, then
//! `scrEntityGen.gml` creates one locked chest at the first exit unless the
//! chain was completed earlier. Population-only fixtures do not exercise the
//! complete level builder, accumulated campaign flags, or the generated exit.

use spelunky_core::{Area, Game, RulesPreset};

fn assert_forced_chest_at_level_four(seed: u32) {
    let mut game = Game::new(seed, RulesPreset::Classic);
    while game.world.level < 4 {
        game.next_level();
    }

    assert_eq!(game.world.level, 4, "seed={seed}");
    assert_eq!(game.world.area, Area::Mines, "seed={seed}");

    let chests: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|entity| entity.alive && entity.object == "oLockedChest")
        .collect();
    if game.flags.contains("made_udjat_eye") {
        assert!(chests.len() <= 1, "seed={seed}: {chests:?}");
    } else {
        assert_eq!(
            chests.len(),
            1,
            "Classic forces one locked chest on Mines level 4; seed={seed}"
        );
    }

    let keys = game
        .world
        .entities
        .iter()
        .filter(|entity| entity.alive && entity.object == "oKey")
        .count();
    assert!(
        keys <= 1,
        "Classic sets global.Key after its first key; seed={seed}"
    );
}

#[test]
fn representative_seeds_keep_the_forced_mines_chest_at_level_four() {
    for seed in [0, 1, 42, 9_999] {
        assert_forced_chest_at_level_four(seed);
    }
}
