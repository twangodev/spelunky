// TEST-ONLY PROPOSAL, NOT RUN OR COMPILE-VALIDATED.
// Intended as a temporary integration test for spelunky-core after the four
// sparse increments are applied privately. No mid-run actor creation, pose,
// velocity, image_index, timer, status, or equipment assignments are used.
// Both tests use initial source-created stationary bullets and ordinary inputs.
// No runtime parity claim accompanies this unexecuted packet.
use spelunky_core::{Game, InputFrame, RulesPreset, Vec2, level::Level};

fn first_frame_room(rules: RulesPreset) -> Game {
    let mut level = Level::default();
    level.set_tile(4, 8, '@').unwrap();
    for x in 0..40 { level.set_tile(x, 10, '1').unwrap(); }
    let mut game = Game::shell(42, rules);
    game.load_custom(&level, "draw-mask-initial-actors", "");
    game
}

// Derive the source rectangle's mirrored-only rightmost pixel from bundled
// atlas metadata. This is static fixture geometry, not mutation of game state.
fn mirrored_only_fringe(game: &Game) -> Vec2 {
    let atlas: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"), "/../../assets/atlas.json"
    ))).unwrap();
    let player = &game.presentation().player;
    let sprite = &atlas["sprites"][&player.sprite];
    assert_eq!(sprite["mask"]["shape"], "RECTANGLE");
    let frames = sprite["frames"].as_array().unwrap();
    let frame = &frames[player.animation.floor() as usize % frames.len()];
    let b = frame["bounds"].as_array().unwrap();
    let ox = sprite["origin"][0].as_i64().unwrap();
    let positive_max = b[2].as_i64().unwrap() - ox;
    let mirrored_max = ox - b[0].as_i64().unwrap();
    assert!(mirrored_max > positive_max, "fixture must retain a mirrored-only fringe");
    // Source Bullet Create: stationary, unsafe. Its first opaque pixels in
    // rows 3/4 are x=2, origin=(4,0), hence origin=(fringe+2, player_y-3).
    Vec2::new(player.pos.x + mirrored_max as f64 + 2., player.pos.y - 3.)
}

#[test]
fn initial_stationary_bullet_uses_default_positive_scale_before_first_draw() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
    let mut baseline = first_frame_room(rules);
    baseline.step(&InputFrame::default());
    let bullet_position = mirrored_only_fringe(&baseline);
    let mut game = first_frame_room(rules);
    // All actors exist before the first step. Bullet's source Create supplies
    // zero velocity and safe=false; no post-Create state injection.
    game.spawn("oBullet", bullet_position);
    game.step(&InputFrame::default());
    assert_eq!(game.player.health, 4, "first Collision must use Create image_xscale=+1");
    assert_eq!(game.player.data.get("image_xscale"), Some(&-1.), "fixed Draw persists the right-facing scale");
    assert_eq!(game.presentation().player.data.get("image_xscale"), Some(&-1.));
    }
}

#[test]
fn initial_stationary_bullet_hits_on_turn_using_preceding_draw_scale() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
    let left = InputFrame { left: true, ..Default::default() };
    let mut baseline = first_frame_room(rules);
    baseline.step(&InputFrame::default());
    baseline.step(&left);
    let bullet_position = mirrored_only_fringe(&baseline);
    let mut game = first_frame_room(rules);
    game.spawn("oBullet", bullet_position);
    game.step(&InputFrame::default());
    assert_eq!(game.player.health, 4, "fixture must exclude the first-frame source mask");
    game.step(&left);
    assert_eq!(game.player.facing, -1);
    assert_eq!(game.player.health, 0, "turning Collision must retain preceding Draw's -1 scale");
    assert_eq!(game.player.data.get("image_xscale"), Some(&1.), "this Draw persists the new left-facing scale");
    }
}
