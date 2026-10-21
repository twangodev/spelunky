use crate::{Game, RulesPreset};

#[test]
fn temple_fourteen_persists_source_gold_chance_after_failed_candidates() {
    // Classic scrEntityGen sets genGoldEntrance for level 14, then decrements
    // global.goldChance once for each failed candidate before placing the door.
    let mut game = Game::shell(0, RulesPreset::Classic);
    game.start_from_hub(false, 14);

    assert_eq!(game.world.area, crate::Area::Temple);
    assert!(
        game.world
            .entities
            .iter()
            .any(|entity| entity.alive && entity.object == "oGoldDoor")
    );
    assert_eq!(game.data.get("goldChance"), Some(&133.0));

    // The global survives the next room transition just as it does in the
    // source run namespace; starting a fresh local 160 would lose this state.
    game.next_level();
    assert_eq!(game.world.level, 15);
    assert_eq!(game.data.get("goldChance"), Some(&133.0));
}
