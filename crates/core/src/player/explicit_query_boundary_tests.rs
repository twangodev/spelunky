use super::*;

#[test]
fn source_platform_rectangle_includes_fractional_query_rounded_edge_pixel() {
    let mut game = Game::shell(31, RulesPreset::Classic);
    assert_eq!(
        game.generation_revision,
        crate::GenerationRevision::SourceClassicV2
    );
    game.world = World::new(8, 8, Area::Mines, 1);

    // sLadderTop is a shipped 16x16 RECTANGLE mask, parented to oPlatform.
    // At this placement its top-left pixel is exactly the player's query's
    // rounded inclusive lower-right edge: (41.1, 103.1) rounds to (41, 103).
    let platform_id = game.world.spawn("oLadderTop", Vec2::new(41.0, 103.0));
    let platform_mask = game.world.entity(platform_id).unwrap().collision_mask();
    assert!(platform_mask.supports_source_query_pixels());
    assert!(platform_mask.contains(Vec2::new(41.0, 103.0), CollisionPrecision::Precise));

    game.player.pos = Vec2::new(37.1, 96.1);
    let query = bounds_at(&game.player, game.player.pos);
    assert_eq!(query.x + query.w - 1.0, 41.1);
    assert_eq!(query.y + query.h - 1.0, 103.1);
    assert!(precise_rect(&game.world, query, "oPlatform"));
    assert!(!platform(&game.world, &game.player, game.player.pos, 1.0));

    // Moving the fractional query just outside either raw cached edge must
    // fail before rounding can pull the endpoint back onto the pixel.
    for player_pos in [Vec2::new(36.9, 96.1), Vec2::new(37.1, 95.9)] {
        let query = bounds_at(&game.player, player_pos);
        assert!(
            !precise_rect(&game.world, query, "oPlatform"),
            "query outside inclusive edge should miss at {player_pos:?}"
        );
    }
}
