//! End-to-end water-friction branches from `characterStepEvent.gml:751-766`.
use spelunky_core::{Area, Game, InputFrame, PlayerState, RulesPreset, Vec2, World};

fn stepped_vertical_velocity(
    state: PlayerState,
    initial_y_velocity: f64,
    water_rows: &[usize],
) -> f64 {
    let mut game = Game::shell(19, RulesPreset::Classic);
    game.world = World::new(20, 15, Area::Mines, 1);
    for &row in water_rows {
        game.world.set_tile(5, row, "oWaterSwim");
    }
    game.player.pos = Vec2::new(88.0, 88.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = state;
    game.player.vel = Vec2::new(0.0, initial_y_velocity);
    game.player.data.insert("fall_timer".into(), 0.0);
    game.player.data.insert("fall_ticks".into(), 0.0);

    game.step(&InputFrame::default());

    game.player.vel.y
}

#[test]
fn falling_through_water_applies_half_vertical_friction() {
    let dry = stepped_vertical_velocity(PlayerState::Falling, 2.0, &[]);
    let wet = stepped_vertical_velocity(PlayerState::Falling, 2.0, &[5]);

    assert!(dry > 0.0);
    assert_eq!(wet, dry * 0.5);
}

#[test]
fn upward_motion_uses_the_water_head_probe_for_full_or_reduced_friction() {
    let dry = stepped_vertical_velocity(PlayerState::Jumping, -2.0, &[]);
    let shallow = stepped_vertical_velocity(PlayerState::Jumping, -2.0, &[5]);
    let submerged = stepped_vertical_velocity(PlayerState::Jumping, -2.0, &[4, 5]);

    assert_eq!(shallow, dry, "the point nine pixels above origin is dry");
    assert_eq!(submerged, dry * 0.9, "the head point is also underwater");
}
