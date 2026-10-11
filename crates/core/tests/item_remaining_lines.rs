//! Source line query coverage. Sinking supports use default Create and neutral
//! ordinary input; half-origin actor states below are explicitly controlled.
use spelunky_core::*;
fn room(rules: RulesPreset) -> Game {
    let mut level = level::Level::default();
    level.set_tile(14, 8, '@').unwrap();
    level.set_tile(14, 9, '1').unwrap();
    let mut game = Game::shell(42, rules);
    game.load_custom(&level, "remaining-item-source-lines", "");
    game
}
fn projectile_support(sinking: bool, kinds: &[(&str, f64)]) {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for &(kind, y) in kinds {
            let mut game = room(rules);
            if !sinking {
                game.world.set_tile(5, 6, "oBlock");
            }
            let block = game.spawn(
                "oPushBlock",
                Vec2::new(80.0, if sinking { 79.0 } else { 80.0 }),
            );
            if sinking {
                game.spawn("oLava", Vec2::new(80.0, 80.0));
            }
            let id = game.spawn(kind, Vec2::new(88.0, y));
            assert_eq!(game.world.entity(id).unwrap().vel, Vec2::default());
            let mut expected_block = 80.0;
            let release_age = if kind == "oBomb" { 12 } else { 11 };
            for age in 1..=release_age {
                game.step(&InputFrame::default());
                if sinking {
                    expected_block += 0.05;
                }
                assert!(
                    (game.world.entity(block).unwrap().pos.y - expected_block).abs() < 1e-10,
                    "BLOCK_SOURCE_SINK {rules:?}/{kind}/{age}"
                );
                let item = game.world.entity(id).unwrap();
                assert!(item.alive);
                assert_eq!(
                    item.pos,
                    Vec2::new(88.0, y),
                    "movement precedes acceleration {kind}/{age}"
                );
                if sinking && age == release_age {
                    assert_eq!(
                        item.value("colBot"),
                        0.0,
                        "SOURCE_PROJECTILE_RELEASE {rules:?}/{kind}"
                    );
                    assert_eq!(item.vel.y, 0.6);
                } else {
                    assert_eq!(item.value("colBot"), 1.0);
                    assert_eq!(item.vel.y, 0.0);
                }
                assert_eq!(game.player.health, 4);
            }
        }
    }
}
#[test]
fn natural_default_bomb_releases_sinking_support() {
    projectile_support(true, &[("oBomb", 76.0)]);
}
#[test]
fn natural_default_frog_bomb_releases_sinking_support() {
    projectile_support(true, &[("oFireFrogBomb", 72.0)]);
}
#[test]
fn natural_default_rope_throw_releases_sinking_support() {
    projectile_support(true, &[("oRopeThrow", 76.0)]);
}
#[test]
fn integer_default_projectile_families_keep_opaque_support() {
    projectile_support(
        false,
        &[
            ("oBomb", 76.0),
            ("oFireFrogBomb", 72.0),
            ("oRopeThrow", 76.0),
        ],
    );
}
#[test]
fn controlled_fragile_bottom_zero_uses_cached_integer_support() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for fractional in [false, true] {
            for (kind, y) in [("oJar", 75.0), ("oSkull", 77.0)] {
                let mut game = room(rules);
                game.world.set_tile(5, 6, "oBlock");
                let block_y = if fractional { 80.55 } else { 80.0 };
                let block = game.spawn("oPushBlock", Vec2::new(80.0, block_y));
                let id = game.spawn(kind, Vec2::new(80.0, y));
                game.step(&InputFrame::default());
                assert_eq!(game.world.entity(block).unwrap().pos.y, block_y);
                let item = game.world.entity(id).unwrap();
                assert!(item.alive);
                assert_eq!(item.value("colBot"), 1.0);
                // Controlled one-pixel initial overlap: Bottom1 is81 and hits
                // both tops. Bottom0 is80; source cached fractional top81
                // excludes it, while integer top80 requires the one-pixel lift.
                assert_eq!(
                    item.pos,
                    Vec2::new(
                        if fractional { 80.0 } else { 79.0 },
                        y - if fractional { 0.0 } else { 1.0 }
                    ),
                    "SOURCE_FRAGILE_BOTTOM_ZERO {rules:?}/{fractional}/{kind}"
                );
                assert_eq!(item.vel.y, 0.0);
                assert_eq!(item.value("colLeft"), 0.0);
                assert_eq!(item.value("colRight"), if fractional { 0.0 } else { 1.0 });
            }
        }
    }
}
#[test]
fn controlled_dice_unsupported_corner_retains_roll_image() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for dynamic in [false, true] {
            let mut game = room(rules);
            if dynamic {
                game.spawn("oDarkFall", Vec2::new(80.0, 80.0));
            } else {
                game.world.set_tile(5, 5, "oDarkFall");
            }
            let id = game.spawn("oDice", Vec2::new(75.5, 72.0));
            // Controlled roll-art state isolates the child's low-speed support query;
            // no claim that ordinary input produces this exact fractional X.
            game.world.entity_mut(id).unwrap().sprite = "sDiceRoll".into();
            game.step(&InputFrame::default());
            let item = game.world.entity(id).unwrap();
            assert_eq!(item.value("colBot"), 0.0);
            assert_eq!(item.vel.y, 0.6);
            assert_eq!(
                item.sprite, "sDiceRoll",
                "SOURCE_DICE_BOTTOM_LINE {rules:?}/{dynamic}"
            );
        }
    }
}
#[test]
fn controlled_falling_rope_passes_transparent_bottom_endpoint() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for dynamic in [false, true] {
            let mut game = room(rules);
            if dynamic {
                game.spawn("oDarkFall", Vec2::new(80.0, 80.0));
            } else {
                game.world.set_tile(5, 5, "oDarkFall");
            }
            let id = game.spawn("oRopeThrow", Vec2::new(77.5, 68.0));
            // Controlled valid postdeployment state. Its child Step advances8 then
            // queries inclusive x74..80 at y80; opaque x81 must not stop extension.
            game.world
                .entity_mut(id)
                .unwrap()
                .data
                .insert("falling".into(), 1.0);
            game.step(&InputFrame::default());
            assert!(
                game.world.entity(id).is_some(),
                "SOURCE_FALLING_ROPE_LINE receiver survives {rules:?}/{dynamic}"
            );
            let rope = game.world.entity(id).unwrap();
            assert!(rope.alive, "SOURCE_FALLING_ROPE_LINE {rules:?}/{dynamic}");
            assert_eq!(rope.pos, Vec2::new(77.5, 76.0));
            assert_eq!(rope.value("fall_count"), 1.0);
            assert!(
                game.world
                    .entities
                    .iter()
                    .any(|e| e.alive && e.object == "oRope" && e.pos == Vec2::new(69.5, 76.0))
            );
        }
    }
}
