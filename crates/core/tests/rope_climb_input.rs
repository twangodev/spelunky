//! Exercise the player-created rope as a ladder through ordinary input.
use spelunky_core::{Area, Game, InputFrame, PlayerState, RulesPreset, Vec2, World};

#[test]
fn thrown_rope_deploys_and_can_be_climbed() {
    for (rules, facing) in [
        (RulesPreset::Classic, -1),
        (RulesPreset::Classic, 1),
        (RulesPreset::Assisted, -1),
        (RulesPreset::Assisted, 1),
    ] {
        let mut game = Game::shell(42, rules);
        game.world = World::new(30, 20, Area::Mines, 1);
        game.world.name = "rLevel".into();
        for x in 0..game.world.width {
            game.world.set_tile(x, 10, "oBrickSmooth");
        }
        game.player.pos = Vec2::new(152.0, 152.0);
        game.player.previous_pos = game.player.pos;
        game.player.state = PlayerState::Standing;
        game.player.grounded = true;
        game.player.facing = facing;
        game.world.entrance = game.player.pos;
        game.step(&InputFrame {
            rope_pressed: true,
            ..Default::default()
        });
        assert_eq!(game.player.ropes, 3.0);
        for _ in 0..60 {
            game.step(&InputFrame::default());
        }
        let top = game
            .world
            .entities
            .iter()
            .find(|e| e.alive && e.object == "oRopeTop")
            .expect("rope reaches its apex");
        let rope_x = top.pos.x;
        assert!(
            game.world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oRope")
        );
        assert!(
            !game
                .world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oRopeThrow"),
            "deployment finishes"
        );
        let mut latched = false;
        for _ in 0..60 {
            let dx = rope_x - game.player.pos.x;
            game.step(&InputFrame {
                left: dx < -2.0,
                right: dx > 2.0,
                up: true,
                ..Default::default()
            });
            if game.player.state == PlayerState::Climbing {
                latched = true;
                break;
            }
        }
        assert!(
            latched,
            "{rules:?}: failed to latch at {:?}, rope x={rope_x}",
            game.player.pos
        );
        let latch_y = game.player.pos.y;
        for _ in 0..20 {
            game.step(&InputFrame {
                up: true,
                ..Default::default()
            });
        }
        assert!(
            game.player.pos.y < latch_y - 8.0,
            "{rules:?}: failed to climb from {latch_y} to {}",
            game.player.pos.y
        );
        assert_eq!(
            game.player.state,
            PlayerState::Climbing,
            "still on rope before jumping"
        );
        game.step(&InputFrame {
            jump: true,
            jump_pressed: true,
            ..Default::default()
        });
        assert_eq!(
            game.player.state,
            PlayerState::Jumping,
            "jump releases the rope"
        );
        assert!(game.player.vel.y < 0.0, "rope jump moves upward");
    }
}
