//! Natural Step parity for the three fallen decoration actors.
//! Fixtures author only the room, terrain, and starting actor positions.
use spelunky_core::{Area, Game, InputFrame, RulesPreset, Vec2, World};

fn room() -> Game {
    let mut game = Game::shell(42, RulesPreset::Classic);
    game.world = World::new(40, 24, Area::Mines, 1);
    game.world.name = "rLevel".into();
    for x in 0..20 {
        game.world.set_tile(x, 8, "oHardBlock");
    }
    game.player.pos = Vec2::new(80.0, 120.0);
    game.player.previous_pos = game.player.pos;
    game.world.entrance = game.player.pos;
    game
}

#[test]
fn fake_bones_uses_strict_view_edges_before_falling() {
    // Classic's default view is [0,320) x [0,240). The source guard is
    // x > view_x-16, x < view_x+width, and the same strict y comparisons.
    for (position, should_fall) in [
        (Vec2::new(-16.0, 32.0), false),
        (Vec2::new(-15.0, 32.0), true),
        (Vec2::new(320.0, 32.0), false),
        (Vec2::new(319.0, 32.0), true),
        (Vec2::new(32.0, -16.0), false),
        (Vec2::new(32.0, -15.0), true),
        (Vec2::new(32.0, 240.0), false),
        (Vec2::new(32.0, 239.0), true),
    ] {
        let mut game = room();
        let bones = game.spawn("oFakeBones", position);

        game.step(&InputFrame::default());

        let bones = game.world.entity(bones).unwrap();
        assert_eq!(bones.pos.x, position.x);
        assert_eq!(bones.pos.y, position.y);
        assert_eq!(bones.vel.y, if should_fall { 0.2 } else { 0.0 });
    }
}

#[test]
fn bones_and_used_parachute_fall_offscreen_and_follow_point_support() {
    for object in ["oBones", "oParaUsed"] {
        // These two source Steps have no viewport guard, unlike FakeBones.
        let mut offscreen = room();
        let decoration = offscreen.spawn(object, Vec2::new(-16.0, 32.0));
        offscreen.step(&InputFrame::default());
        let decoration_state = offscreen.world.entity(decoration).unwrap();
        assert_eq!(decoration_state.pos, Vec2::new(-16.0, 32.0));
        assert_eq!(decoration_state.vel.y, 0.2, "{object} has no view cutoff");

        // Source probes x+8,y+16 before movement, advances yVel by yAcc
        // only while clear, then corrects one pixel if x+8,y+15 is solid.
        let mut landing = room();
        landing.world.set_tile(6, 5, "oHardBlock");
        let decoration = landing.spawn(object, Vec2::new(100.0, 63.0));
        for _ in 0..6 {
            landing.step(&InputFrame::default());
        }
        let decoration_state = landing.world.entity(decoration).unwrap();
        assert_eq!(decoration_state.pos, Vec2::new(100.0, 64.2), "{object}");
        assert_eq!(decoration_state.vel.y, 0.8, "{object}");
    }
}
