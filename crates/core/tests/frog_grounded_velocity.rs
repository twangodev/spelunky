//! Frogs retain source yVel while idle on support, even when the supporting
//! floor is later removed. Exercise the ordinary public Game::step pipeline.
use spelunky_core::*;

fn scene(object: &str) -> (Game, u64) {
    let mut game = Game::shell(101, RulesPreset::Classic);
    game.world = World::new(20, 12, Area::Jungle, 1);
    game.world.name = "rLevel".into();
    for x in 0..20 {
        game.world.set_tile(x, 5, "oBlock");
    }

    // Keep the Player and Frog both within the room/view and more than 64 px
    // apart, so the frog stays idle throughout this source-step reproduction.
    game.player.pos = Vec2::new(240.0, 64.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.world.entrance = game.player.pos;
    let frog = game.spawn(object, Vec2::new(88.0, 64.0));
    (game, frog)
}

#[test]
fn frog_species_retain_fall_speed_after_support_is_removed() {
    for object in ["oFrog", "oFireFrog"] {
        let (mut game, id) = scene(object);

        for _ in 0..50 {
            game.step(&InputFrame::default());
        }

        let resting = game.world.entity(id).expect("frog survives while resting");
        assert_eq!(resting.state, 0, "{object} remains idle");
        assert_eq!(resting.pos.y, 64.0, "{object} stays on the floor");
        assert!(
            (resting.vel.y - 10.0).abs() < 1e-9,
            "{object} accumulates capped yVel: {}",
            resting.vel.y
        );

        // The 16 px source movement body spans x+4..x+15, so these two tiles
        // are the full support directly beneath the frog.
        game.world.set_tile(5, 5, "");
        game.world.set_tile(6, 5, "");
        game.step(&InputFrame::default());

        let falling = game.world.entity(id).expect("frog survives floor removal");
        assert_eq!(
            falling.pos.y, 74.0,
            "{object} immediately falls at stored speed"
        );
        assert!(
            (falling.vel.y - 10.0).abs() < 1e-9,
            "{object} remains at the speed cap: {}",
            falling.vel.y
        );
    }
}
