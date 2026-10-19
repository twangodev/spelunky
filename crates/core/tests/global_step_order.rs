//! Cross-object birth order in oGame Step follows the source event body.
use spelunky_core::{Game, InputFrame, RulesPreset};

#[test]
fn overdue_ghost_is_created_before_same_frame_water_drips() {
    let mut game = Game::new(123, RulesPreset::Classic);
    game.next_level();
    let x = (2..game.world.width - 2)
        .find(|&x| {
            game.world
                .tile(x as i32, 5)
                .is_some_and(|tile| tile.object.is_empty())
        })
        .expect("generated level has an empty tile for the fluid fixture");
    game.world.set_tile(x, 5, "oWater");
    game.data.insert("xtime".into(), 150_001.0);
    game.flags.remove("ghost_exists");
    game.world.fluid_dirty = true;

    game.step(&InputFrame::default());

    let ghost = game
        .world
        .entities
        .iter()
        .find(|entity| entity.alive && entity.object == "oGhost")
        .expect("overdue level timer creates the source Ghost");
    let first_drip = game
        .world
        .entities
        .iter()
        .filter(|entity| entity.alive && entity.object == "oDrip")
        .map(|entity| entity.creation_serial)
        .min()
        .expect("open fluid creates Drips");
    assert!(ghost.creation_serial < first_drip);
}
