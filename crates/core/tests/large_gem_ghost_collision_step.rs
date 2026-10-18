//! Big treasure's own Ghost Collision overrides are exercised through the live
//! Game::step dispatcher, not a direct callback invocation.
use spelunky_core::{Area, Game, InputFrame, RulesPreset, Vec2, World};

#[test]
fn ordinary_ghost_contact_turns_each_large_gem_into_a_diamond() {
    for large_gem in ["oEmeraldBig", "oSapphireBig", "oRubyBig"] {
        let mut game = Game::shell(42, RulesPreset::Classic);
        game.world = World::new(30, 20, Area::Mines, 1);
        game.world.name = "rLevel".into();
        for column in 0..30 {
            game.world.set_tile(column, 10, "oHardBlock");
        }
        game.player.pos = Vec2::new(240.0, 152.0);
        game.player.previous_pos = game.player.pos;
        game.world.entrance = game.player.pos;
        let ghost = game.spawn("oGhost", Vec2::new(112.0, 112.0));
        let ghost_bounds = game.world.entity(ghost).unwrap().bounds();
        let gem_pos = Vec2::new(
            ghost_bounds.x + ghost_bounds.w / 2.0,
            ghost_bounds.y + ghost_bounds.h / 2.0,
        );
        let gem = game.spawn(large_gem, gem_pos);
        assert!(game.world.entity(ghost).unwrap().engine_active);
        assert!(game.world.entity(gem).unwrap().engine_active);

        game.step(&InputFrame::default());

        assert!(
            game.world.entity(gem).is_none(),
            "source destroys {large_gem}"
        );
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|entity| entity.alive && entity.object == "oDiamond")
                .count(),
            1,
            "source collision births exactly one Diamond from {large_gem}"
        );
    }
}
