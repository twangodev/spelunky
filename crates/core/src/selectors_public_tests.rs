//! Existing public APIs only: the exact same bodies can be installed on the
//! pre-selector baseline. Source-only preparation does not establish a result.
use crate::{Area, Game, RulesPreset};

#[test]
fn selectors_public_actual_yeti_lair_never_authors_a_second_live_king() {
    // Source Yeti selection only runs once the Moai flag is already set. Seed
    // 11 naturally generates the level-9 Moai, then the ordinary transition
    // selects the Yeti Lair at level 10.
    let mut game = Game::shell(11, RulesPreset::Classic);
    game.start_from_hub(false, 9);
    assert_eq!(game.world.level, 9);
    assert_eq!(game.world.area, Area::Ice);
    assert!(game.flags.contains("made_moai"));
    assert!(
        game.world
            .entities
            .iter()
            .any(|e| e.alive && e.object == "oMoai")
    );

    game.next_level();

    assert_eq!(game.world.level, 10);
    assert_eq!(game.world.area, Area::Ice);
    assert_eq!(game.world.name, "Yeti Lair");
    let count = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && e.engine_active && e.object == "oYetiKing")
        .count();
    assert!(count > 0, "the generated Yeti Lair must contain its King");
    assert!(count <= 1, "generated {count} live Yeti Kings");
}
