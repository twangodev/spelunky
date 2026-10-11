//! characterStepEvent117 selects oIce only;774-778 applies friction .98.
//! oIceBlock inherits oMoveableSolid, so its top retains ordinary friction .6.
//! Initial authored geometry and source entrance, then ordinary Step inputs.
use spelunky_core::{Area, Game, InputFrame, PlayerState, RulesPreset, Vec2, World};

fn supported(rules: RulesPreset, support: &str) -> (Game, Option<u64>) {
    let mut game = Game::shell(42, rules);
    game.world = World::new(24, 20, Area::Ice, 9);
    for x in 0..24 {
        game.world.set_tile(x, 11, "oBrick");
    }
    game.player.pos = Vec2::new(88.0, 152.0);
    game.player.previous_pos = game.player.pos;
    let support_id = if support == "oIce" {
        game.world.set_tile(5, 10, "oIce");
        None
    } else {
        Some(game.spawn(support, Vec2::new(80.0, 160.0)))
    };
    for _ in 0..2 {
        game.step(&InputFrame::default());
        assert_eq!(game.player.pos, Vec2::new(88.0, 152.0));
        assert_eq!(game.player.state, PlayerState::Standing);
        assert!(game.player.grounded);
    }
    (game, support_id)
}

fn first_walk(
    rules: RulesPreset,
    support: &str,
    right: bool,
    expected_speed: f64,
    expected_pixels: f64,
) {
    let (mut game, id) = supported(rules, support);
    let input = InputFrame {
        left: !right,
        right,
        ..Default::default()
    };
    for _ in 0..2 {
        game.step(&input);
        assert_eq!(game.player.vel.x, 0.0, "source two-tick direction delay");
        assert_eq!(game.player.pos, Vec2::new(88.0, 152.0));
    }
    game.step(&input);
    let sign = if right { 1.0 } else { -1.0 };
    assert!(
        (game.player.vel.x - sign * expected_speed).abs() < 1e-12,
        "{support} {rules:?} right={right}: {:?}",
        game.player.vel
    );
    assert_eq!(
        game.player.pos,
        Vec2::new(88.0 + sign * expected_pixels, 152.0)
    );
    assert_eq!(game.player.state, PlayerState::Running);
    assert!(game.player.grounded && !game.player.is_stunned());
    assert_eq!(game.player.health, 4);
    if let Some(id) = id {
        let support = game.world.entity(id).unwrap();
        assert!(support.alive && support.engine_active);
        assert_eq!(support.pos, Vec2::new(80.0, 160.0));
    }
}

#[test]
fn natural_walk_on_movable_ice_block_uses_ordinary_ground_friction() {
    assert!(!spelunky_core::catalog::is_a("oIceBlock", "oIce"));
    assert!(spelunky_core::catalog::is_a("oIceBlock", "oMoveableSolid"));
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for right in [false, true] {
            first_walk(rules, "oIceBlock", right, 1.8, 2.0);
        }
    }
}

#[test]
fn natural_walk_on_push_block_retains_ordinary_ground_friction() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for right in [false, true] {
            first_walk(rules, "oPushBlock", right, 1.8, 2.0);
        }
    }
}

#[test]
fn natural_walk_on_source_ice_terrain_retains_ice_friction() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for right in [false, true] {
            first_walk(rules, "oIce", right, 2.94, 3.0);
        }
    }
}
