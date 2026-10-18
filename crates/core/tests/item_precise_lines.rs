//! Neutral-input authored sinking support witness, plus explicitly controlled
//! fractional-origin rounding fixtures. No post-Create actor velocity writes.
use spelunky_core::*;

fn scene(rules: RulesPreset, sinking: bool) -> (Game, u64, u64) {
    let mut level = level::Level::default();
    level.set_tile(14, 8, '@').unwrap();
    level.set_tile(14, 9, '1').unwrap();
    if !sinking {
        level.set_tile(4, 5, '1').unwrap();
    }
    let mut game = Game::shell(42, rules);
    game.load_custom(&level, "item-source-sinking-support", "");
    assert_eq!(game.player.pos, Vec2::new(248.0, 152.0));
    let block = game.spawn(
        "oPushBlock",
        Vec2::new(80.0, if sinking { 79.0 } else { 80.0 }),
    );
    if sinking {
        game.spawn("oLava", Vec2::new(80.0, 80.0));
    }
    let rock = game.spawn("oRock", Vec2::new(88.0, 76.0));
    assert_eq!(game.world.entity(rock).unwrap().vel, Vec2::default());
    (game, block, rock)
}
fn sinking_support(rules: RulesPreset) {
    let (mut game, block, rock) = scene(rules, true);
    let mut source_y = 80.0;
    for age in 1..=11 {
        game.step(&InputFrame::default());
        source_y += 0.05;
        let solid = game.world.entity(block).unwrap();
        assert!((solid.pos.y - source_y).abs() < 1e-10, "sink{age}");
        assert_eq!(solid.value("push_gravity"), 0.0);
        let item = game.world.entity(rock).unwrap();
        assert_eq!(item.pos, Vec2::new(88.0, 76.0));
        assert_eq!(item.vel.y, 0.0);
        assert_eq!(item.value("colBot"), 1.0);
        assert_eq!(game.player.health, 4.0);
    }
    assert!(source_y > 80.5 && source_y < 80.6);
    game.step(&InputFrame::default());
    let item = game.world.entity(rock).unwrap();
    // Rock450 queries before PushBlock467 adds the next.05. The source line
    // at foot80 misses cached top81 after11 natural sink increments. Item
    // movement precedes gravity, so acceleration does not move it this frame.
    assert_eq!(
        item.value("colBot"),
        0.0,
        "SOURCE_ITEM_RELEASES_ROUNDED_FLOAT_SUPPORT {rules:?}"
    );
    assert_eq!(item.vel.y, 0.6);
    assert_eq!(item.pos, Vec2::new(88.0, 76.0));
    assert!((game.world.entity(block).unwrap().pos.y - (source_y + 0.05)).abs() < 1e-10);
    assert_eq!(game.player.health, 4.0);
}
#[test]
fn natural_sinking_block_releases_classic_item_support() {
    sinking_support(RulesPreset::Classic);
}
#[test]
fn natural_sinking_block_releases_assisted_item_support() {
    sinking_support(RulesPreset::Assisted);
}
#[test]
fn integer_supported_block_keeps_the_item_on_its_opaque_top() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, block, rock) = scene(rules, false);
        for _ in 0..13 {
            game.step(&InputFrame::default());
            assert_eq!(game.world.entity(block).unwrap().pos, Vec2::new(80.0, 80.0));
            let item = game.world.entity(rock).unwrap();
            assert_eq!(item.pos, Vec2::new(88.0, 76.0));
            assert_eq!(item.vel.y, 0.0);
            assert_eq!(item.value("colBot"), 1.0);
            assert_eq!(game.player.health, 4.0);
        }
    }
}
#[test]
fn controlled_half_tie_item_endpoint_does_not_gain_an_extra_pixel() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for dynamic in [false, true] {
            let mut level = level::Level::default();
            level.set_tile(14, 8, '@').unwrap();
            level.set_tile(14, 9, '1').unwrap();
            let mut game = Game::shell(42, rules);
            game.load_custom(&level, "controlled-item-half-tie", "");
            let item = game.spawn("oRock", Vec2::new(77.5, 76.0));
            if dynamic {
                game.spawn("oDarkFall", Vec2::new(80.0, 80.0));
            } else {
                game.world.set_tile(5, 5, "oDarkFall");
            }
            game.step(&InputFrame::default());
            let item = game.world.entity(item).unwrap();
            // RTE(lb=73.5)=74 but RTE(rb-1=80.5)=80. A width8 Rect
            // built from the rounded first endpoint incorrectly adds x81.
            assert_eq!(
                item.value("colBot"),
                0.0,
                "CONTROLLED_HALF_TIE_SOURCE_ENDPOINT {rules:?}/{dynamic}"
            );
            assert_eq!(item.vel.y, 0.6);
            assert_eq!(item.pos, Vec2::new(77.5, 76.0));
        }
    }
}
