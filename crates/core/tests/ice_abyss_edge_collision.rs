//! Untouched Ice generation keeps its last side wall active until the source
//! falling-death threshold is reached.
use spelunky_core::{
    CollisionPrecision, Game, InputFrame, Rect, RulesPreset, RunState, Vec2, generation,
};

fn natural_ice_game(right: bool) -> Game {
    let mut game = Game::new(42, RulesPreset::Classic);
    game.world = generation::generate(&mut game.rng, 9, &game.flags);

    // The production world is unchanged; only the player is placed at the
    // lower side-wall row where the suspected escape was reported.
    game.player.pos = Vec2::new(if right { 651.0 } else { 21.0 }, 616.0);
    game.player.previous_pos = game.player.pos;
    game.player.vel = Vec2::default();
    game.player.state = spelunky_core::PlayerState::Falling;
    game.player.grounded = false;
    game.player.health = 99.0;
    game.player.invincible = 999;
    game.player.data.insert("active".into(), 1.0);
    game.state = RunState::Playing;
    game.update_camera();
    game
}

fn active_wall_blocks_player_side_query(game: &Game, right: bool) -> bool {
    let p = game.player.pos;
    // Source getIdCollisionLeft/Right probes x-6 / x+5 and y-3..y+7 for
    // oPlayer1's (-5,-8,5,8) collision bounds.
    let probe = Rect::new(
        if right { p.x + 5.0 } else { p.x - 6.0 },
        p.y - 3.0,
        1.0,
        11.0,
    );
    let wall_x = if right { 656.0 } else { 0.0 };
    game.world.entities.iter().any(|entity| {
        entity.alive
            && entity.engine_active
            && entity.object == "oDark"
            && entity.pos == Vec2::new(wall_x, 608.0)
            && entity.mask_intersects(probe, CollisionPrecision::BoundingBox)
    })
}

#[test]
fn natural_ice_side_wall_blocks_until_fall_death_threshold() {
    for right in [false, true] {
        let mut game = natural_ice_game(right);
        // This normal frame activates nearby Level geometry before movement.
        game.step(&InputFrame::default());
        let wall_x = if right { 656.0 } else { 0.0 };
        assert!(game.world.entities.iter().any(|entity| {
            entity.object == "oDark"
                && entity.pos == Vec2::new(wall_x, 608.0)
                && entity.engine_active
        }));
        assert!(
            active_wall_blocks_player_side_query(&game, right),
            "the y=608 source wall overlaps the side query before it ends"
        );

        let death_line = game.world.height as f64 * 16.0 + 16.0;
        let mut contacted_wall = false;
        for _ in 0..8 {
            game.step(&InputFrame {
                left: !right,
                right,
                run: true,
                ..Default::default()
            });
            assert!(
                (21.0..=651.0).contains(&game.player.pos.x),
                "the falling player must not escape the world side before abyss death: {:?}",
                game.player.pos
            );
            let overlap = active_wall_blocks_player_side_query(&game, right);
            if overlap {
                contacted_wall = true;
                assert!(
                    (21.0..=651.0).contains(&game.player.pos.x),
                    "while source side wall overlaps the horizontal probe: {:?}",
                    game.player.pos
                );
            }
            if game.state != RunState::Playing {
                break;
            }
        }

        assert!(
            contacted_wall,
            "the active y=608 source wall must block movement"
        );
        assert_eq!(game.state, RunState::Dead);
        assert!(
            game.player.pos.y > death_line,
            "the open Ice floor kills only after y > {death_line}: {:?}",
            game.player.pos
        );
    }
}
