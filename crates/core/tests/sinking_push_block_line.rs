//! Ordinary default-input steps over a source-authored slowly sinking solid.
use spelunky_core::*;

fn scene(rules: RulesPreset, sinking: bool) -> (Game, u64) {
    let mut level = level::Level::default();
    level.set_tile(4, 3, '@').unwrap();
    if !sinking {
        // Custom tiles gain the source room border: tile4,5 is world80,96.
        level.set_tile(4, 5, '1').unwrap();
    }
    let mut game = Game::shell(42, rules);
    game.load_custom(&level, "source-sinking-support", "");
    assert_eq!(game.player.pos, Vec2::new(88.0, 72.0));
    // Initial actors only. Source MoveableSolid Create starts yVel0, gravity.6;
    // its first global pass moves the unsupported block from79 to80. Lava
    // then zeroes its gravity and its own Step adds.05 once per frame.
    let block = game.spawn(
        "oPushBlock",
        Vec2::new(80.0, if sinking { 79.0 } else { 80.0 }),
    );
    if sinking {
        game.spawn("oLava", Vec2::new(80.0, 80.0));
    }
    (game, block)
}

fn assert_sinking_support(rules: RulesPreset) {
    let (mut game, block) = scene(rules, true);
    let mut source_y = 80.0;
    for age in 1..=11 {
        game.step(&InputFrame::default());
        source_y += 0.05;
        let solid = game.world.entity(block).unwrap();
        assert!(
            (solid.pos.y - source_y).abs() < 1e-10,
            "{rules:?}/sink{age}"
        );
        assert_eq!(solid.value("push_gravity"), 0.0);
        assert_eq!(game.player.pos, Vec2::new(88.0, 72.0));
        assert!(game.player.grounded, "{rules:?}/sink{age}");
        assert_eq!(game.player.health, 4);
    }
    // The eleven stored double increments give≈80.55, not an exact.5
    // literal. Original native line controls prove raw footy80 misses
    // cachedtop81; a float height1 Rect[80,81) incorrectly overlaps it.
    assert!(source_y > 80.5 && source_y < 80.6);
    game.step(&InputFrame::default());
    assert_eq!(
        game.player.pos,
        Vec2::new(88.0, 73.0),
        "{rules:?}: source line must release false floating support",
    );
    assert_eq!(game.player.state, PlayerState::Falling);
    assert_eq!(game.player.health, 4);
    assert!((game.world.entity(block).unwrap().pos.y - (source_y + 0.05)).abs() < 1e-10);
    // Re-grounding after the one-pixel fall is legitimate: footy81 now
    // meets the native pixel mask. Do not require !grounded after movement.
}

#[test]
fn naturally_sinking_push_block_releases_classic_player_at_rounded_top_boundary() {
    assert_sinking_support(RulesPreset::Classic);
}

#[test]
fn naturally_sinking_push_block_releases_assisted_player_at_rounded_top_boundary() {
    assert_sinking_support(RulesPreset::Assisted);
}
#[test]
fn supported_integer_push_block_keeps_player_on_its_opaque_top() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, block) = scene(rules, false);
        for _ in 0..13 {
            game.step(&InputFrame::default());
            assert_eq!(game.world.entity(block).unwrap().pos, Vec2::new(80.0, 80.0));
            assert_eq!(game.player.pos, Vec2::new(88.0, 72.0));
            assert!(game.player.grounded);
            assert_eq!(game.player.health, 4);
        }
    }
}
