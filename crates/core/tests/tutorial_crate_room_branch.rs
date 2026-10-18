//! The tutorial crate's guaranteed Bomb Bag is selected by Classic's room
//! identity, then exercised through the ordinary Player Step input path.
use spelunky_core::{Area, Game, InputFrame, PlayerState, RulesPreset, Vec2, World};

#[test]
fn crate_loot_uses_tutorial_room_identity_without_a_runtime_flag() {
    let mut game = Game::shell(22, RulesPreset::Classic);
    game.world = World::new(24, 20, Area::Mines, 1);
    game.world.name = "rTutorial".into();
    for x in 0..game.world.width {
        game.world.set_tile(x, 10, "oHardBlock");
    }
    game.player.pos = Vec2::new(80.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.world.entrance = game.player.pos;
    assert!(!game.flags.contains("tutorial"));
    let crate_id = game.spawn("oCrate", game.player.pos);
    let bombs = game.player.bombs;

    game.step(&InputFrame {
        up: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });

    assert!(game.world.entity(crate_id).is_none());
    assert_eq!(game.player.bombs, bombs + 3.0);
    assert!(game.world.entities.iter().any(|entity| {
        entity.alive && entity.object == "oItemsGet" && entity.sprite == "sBombsGet"
    }));
    assert!(
        !game
            .world
            .entities
            .iter()
            .any(|entity| { entity.alive && entity.object == "oBow" })
    );
    assert_eq!(
        game.rng.state_bits(),
        22,
        "tutorial branch consumes no loot RNG"
    );
}
