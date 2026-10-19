//! DarkFall collision births retain the inherited oDrawnSprite Create state.
use spelunky_core::{Area, Game, InputFrame, RulesPreset, Vec2, World};

#[test]
fn dark_fall_collision_rubble_has_drawn_sprite_create_fields() {
    let mut game = Game::shell(5, RulesPreset::Classic);
    game.world = World::new(24, 18, Area::Mines, 1);
    game.world.name = "rLevel".into();
    game.player.pos = Vec2::new(200.0, 120.0);
    game.player.previous_pos = game.player.pos;
    game.world.set_tile(5, 5, "oHardBlock");
    game.spawn("oDarkFall", Vec2::new(80.0, 80.0));

    game.step(&InputFrame::default());

    let rubble: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|entity| entity.alive && entity.object == "oRubbleDark")
        .collect();
    assert_eq!(
        rubble.len(),
        3,
        "DarkFall solid collision spawns three pieces"
    );
    for entity in rubble {
        assert_eq!(entity.text.get("type").map(String::as_str), Some(""));
        assert_eq!(entity.data.get("blinkToggle"), Some(&0.0));
    }
}
