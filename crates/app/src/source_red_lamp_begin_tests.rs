//! Ordinary movement after Screen Begin must not resample red-lamp tint.
//! Only initial default-room/object setup writes state; all later work is input.
use super::*;
use spelunky_core::{Area, InputFrame, RulesPreset, World};

// Independent literal original-mask metadata and signed inclusive cache edges.
// No production sprite_distance/mask_distance/query helper is the oracle.
fn source_player_edges(game: &Game) -> [f64; 4] {
    let player = &game.player;
    let [left, top, right, bottom] = match player.sprite.as_str() {
        "sStandLeft" => [2, 0, 13, 15],
        "sRunLeft" => [1, 0, 14, 15],
        other => panic!("SOURCE_LIGHT_PLAYER_SPRITE_ADMISSION: {other}"),
    };
    assert_eq!(player.data.get("image_xscale"), Some(&-1.0));
    assert_eq!(player.data.get("image_yscale").copied().unwrap_or(1.0), 1.0);
    assert_eq!(player.data.get("image_angle").copied().unwrap_or(0.0), 0.0);
    let x0 = (player.pos.x - f64::from(left - 8)).round_ties_even();
    let x1 = (player.pos.x - f64::from(right - 8 + 1) - 1.0).round_ties_even();
    [
        x0.min(x1),
        (player.pos.y + f64::from(top - 8)).round_ties_even(),
        x0.max(x1),
        (player.pos.y + f64::from(bottom - 8)).round_ties_even(),
    ]
}

fn source_lamp_distance(game: &Game, id: u64) -> f64 {
    let [left, top, right, bottom] = source_player_edges(game);
    let lamp = game.world.entity(id).expect("SOURCE_LIGHT_LAMP_PRESENT");
    assert!(lamp.engine_active && !lamp.held);
    let (ox, oy) = match lamp.object.as_str() {
        "oLampRed" => {
            assert_eq!(lamp.sprite, "sLampRed");
            (0.0, 0.0)
        }
        "oLampRedItem" => {
            assert_eq!(lamp.sprite, "sLampRedItem");
            (8.0, 12.0)
        }
        other => panic!("SOURCE_LIGHT_CLASS_ADMISSION: {other}"),
    };
    assert_eq!(lamp.data.get("image_xscale").copied().unwrap_or(1.0), 1.0);
    assert_eq!(lamp.data.get("image_yscale").copied().unwrap_or(1.0), 1.0);
    assert_eq!(lamp.data.get("image_angle").copied().unwrap_or(0.0), 0.0);
    // Both original lamp masks aggregate [1,0,14,15], not subimage0's [2,0,13,15].
    let l = (lamp.pos.x + 1.0 - ox).round_ties_even();
    let r = (lamp.pos.x + 14.0 - ox).round_ties_even();
    let t = (lamp.pos.y - oy).round_ties_even();
    let b = (lamp.pos.y + 15.0 - oy).round_ties_even();
    let dx = (l - right).max(left - r).max(0.0);
    let dy = (t - bottom).max(top - b).max(0.0);
    dx.hypot(dy).min(1_000_000.0)
}

fn red_tint(distance: f64) -> [f64; 3] {
    assert!((0.0..=96.0).contains(&distance));
    // All admitted inputs keep components0..255; native packing overflow is
    // separately covered by existing tests and is not inferred here.
    [
        (255.0 - distance).round_ties_even() / 255.0,
        (24.0 + distance).round_ties_even() / 255.0,
        (24.0 + distance).round_ties_even() / 255.0,
    ]
}

fn room(rules: RulesPreset, carried_x: Option<f64>) -> (Game, u64, Option<u64>) {
    let mut game = Game::shell(9, rules);
    game.world = World::new(20, 16, Area::Mines, 1);
    game.world.dark = true;
    for x in 0..20 {
        game.world.set_tile(x, 10, "oHardBlock");
    }
    game.player.pos = Vec2::new(88.0, 152.0);
    game.player.previous_pos = game.player.pos;
    let static_lamp = game.spawn("oLampRed", Vec2::new(160.0, 144.0));
    let carried = carried_x.map(|x| game.spawn("oLampRedItem", Vec2::new(x, 156.0)));
    for _ in 0..2 {
        game.step(&InputFrame::default());
    }
    assert_eq!(game.player.health, 4.0);
    assert!(game.player.engine_active && game.player.grounded);
    assert_eq!(game.player.pos, Vec2::new(88.0, 152.0));
    assert_eq!(source_lamp_distance(&game, static_lamp), 67.0);
    (game, static_lamp, carried)
}

fn ordinary_motion_uses_begin_tint(rules: RulesPreset, carried_x: Option<f64>) {
    let (mut game, static_lamp, carried) = room(rules, carried_x);
    let mut witnessed_phase_difference = false;
    for _ in 0..8 {
        let before_static = source_lamp_distance(&game, static_lamp);
        let before_carried = carried.map(|id| source_lamp_distance(&game, id));
        assert!(before_static <= 96.0);
        let selected = before_carried
            .filter(|d| *d <= 96.0)
            .unwrap_or(before_static);
        let expected = red_tint(selected);
        let previous = game.player.pos;
        game.step(&InputFrame {
            right: true,
            ..Default::default()
        });
        assert_eq!(game.player.health, 4.0);
        assert!(game.player.engine_active && game.player.grounded);
        assert!(game.player.held.is_none());
        let after_static = source_lamp_distance(&game, static_lamp);
        let after_carried = carried.map(|id| source_lamp_distance(&game, id));
        let after_selected = after_carried.filter(|d| *d <= 96.0).unwrap_or(after_static);
        // Fail the old producer only after real input movement and an actual
        // before/after cached-envelope difference establish phase reachability.
        if previous == game.player.pos || selected == after_selected {
            continue;
        }
        witnessed_phase_difference = true;
        let hash = game.state_hash();
        let draw_hash = game.presentation().state_hash();
        for alpha in [0.0, 0.25, 0.5, 0.75, 1.0] {
            let lights = scene_lights(&game, alpha);
            assert!(
                lights.iter().all(|light| light.tint == expected),
                "SOURCE_RED_LAMP_BEGIN_STEP_TINT: selected={selected} after={after_selected} alpha={alpha}"
            );
            assert_eq!(
                red_lamp_distances(&game),
                [Some(before_static), before_carried]
            );
            let draw = game.presentation();
            assert_eq!(
                lights[0].position,
                crate::scene::interpolated_move_to(
                    draw.player.previous_pos,
                    draw.player.pos,
                    alpha,
                )
            );
            assert_eq!(game.state_hash(), hash);
            assert_eq!(game.presentation().state_hash(), draw_hash);
        }
        assert_ne!(
            scene_lights(&game, 0.0)[0].position,
            scene_lights(&game, 1.0)[0].position,
            "positions still interpolate while the source color sample stays fixed"
        );
    }
    assert!(
        witnessed_phase_difference,
        "SOURCE_ORDINARY_RED_LAMP_PHASE_PRECONDITION"
    );
}

#[test]
fn ordinary_movement_keeps_static_red_lamp_begin_step_color_across_refresh_fractions() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        ordinary_motion_uses_begin_tint(rules, None);
    }
}

#[test]
fn ordinary_movement_preserves_near_carried_override_and_far_carried_static_color() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for x in [128.0, 300.0] {
            ordinary_motion_uses_begin_tint(rules, Some(x));
        }
    }
}
