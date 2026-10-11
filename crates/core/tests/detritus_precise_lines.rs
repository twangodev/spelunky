//! Authored initial Bone/DarkFall scenes. Bone velocities, gravity, sprite,
//! life and alarms come from ordinary Create; no mid-frame state injection.
//! This is a default-Create motion witness, not a claim about a combat producer.
use spelunky_core::*;

fn scene(rules: RulesPreset, dynamic: bool, x: f64) -> (Game, u64) {
    let mut game = Game::shell(5, rules);
    game.world = World::new(30, 20, Area::Ice, 9);
    game.world.name = "rLevel".into();
    for x in 0..30 {
        game.world.set_tile(x, 10, "oHardBlock");
    }
    game.player.pos = Vec2::new(240.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.grounded = true;
    game.player.state = PlayerState::Standing;
    if dynamic {
        game.spawn("oDarkFall", Vec2::new(80.0, 80.0));
    } else {
        game.world.set_tile(5, 5, "oDarkFall");
    }
    // DarkFall has no random Create expressions. Independent literal GM8 LCG
    // plus the Bone Create expression order predict all four consumed draws.
    assert_eq!(game.rng.state_bits(), 5);
    let mut state = 5u64;
    let mut next = || {
        state = state.wrapping_mul(0x08088405).wrapping_add(1) & 0xffff_ffff;
        state
    };
    let vx = next() as f64 / 4294967296.0 * 4.0 - next() as f64 / 4294967296.0 * 4.0;
    let vy = -1.0 - next() as f64 / 4294967296.0 * 2.0;
    let gravity = (1 + ((next() * 6) >> 32)) as f64 * 0.1;
    let id = game.spawn("oBone", Vec2::new(x, 93.0));
    let bone = game.world.entity(id).unwrap();
    assert_eq!(bone.vel, Vec2::new(vx, vy));
    assert_eq!(bone.value("gravity"), gravity);
    assert_eq!(bone.bounds(), Rect::new(x - 4.0, 89.0, 8.0, 8.0));
    assert!(vx.abs() < 0.1 && vy < -2.0 && vy > -2.2);
    assert_eq!(game.rng.state_bits(), state);
    (game, id)
}

#[test]
fn default_bone_crosses_darkfall_transparent_bottom_corner() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for dynamic in [false, true] {
            let (mut game, id) = scene(rules, dynamic, 77.0);
            // Source top lines at y88 and y87, x73..80: the only overlapping
            // DarkFall column is local x0, transparent on both selected rows.
            game.step(&InputFrame::default());
            let bone = game.world.entity(id).unwrap();
            assert_eq!(
                bone.pos,
                Vec2::new(77.0, 91.0),
                "PRECISE_BONE_CORNER rules={rules:?} dynamic={dynamic}"
            );
            assert!(bone.alive && bone.engine_active);
            assert_eq!(bone.value("life"), 59.0);
            assert!(
                bone.vel.y > 0.0,
                "the next top line y86 meets an opaque row and bounces"
            );
            assert_eq!(bone.sprite, "sBone");
        }
    }
}

#[test]
fn default_bone_stops_at_darkfall_opaque_bottom_pixel() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for dynamic in [false, true] {
            let (mut game, id) = scene(rules, dynamic, 79.0);
            // x75..82 includes local x2 on row8: source mask bit is opaque.
            game.step(&InputFrame::default());
            let bone = game.world.entity(id).unwrap();
            assert_eq!(bone.pos, Vec2::new(79.0, 93.0));
            assert!(bone.alive && bone.engine_active && bone.vel.y > 0.0);
            assert_eq!(bone.value("life"), 59.0);
            assert_eq!(bone.sprite, "sBone");
        }
    }
}
