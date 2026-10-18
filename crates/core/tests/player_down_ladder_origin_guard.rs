//! Fixed authored default actors, four neutral Steps, then ordinary Down.
//! No Player fields or later geometry/actor fields are injected. This is a
//! source-admitted authored room, not a generated-map frequency assertion.
use spelunky_core::*;

fn room(rules: RulesPreset, overhead: bool, origin_ladder: bool) -> Game {
    let mut level = level::Level::default();
    level.set_tile(5, 4, '@').unwrap(); // Source entrance104,88.
    for x in 0..40 {
        level.set_tile(x, 5, '1').unwrap(); // Solid support y96.
    }
    let mut game = Game::shell(42, rules);
    game.load_custom(&level, "down-ladder-origin-guard", "");
    game.spawn("oLadderTop", Vec2::new(96.0, 96.0));
    if overhead {
        game.spawn("oLadderOrange", Vec2::new(96.0, 64.0));
    }
    if origin_ladder {
        game.spawn("oLadderOrange", Vec2::new(96.0, 80.0));
    }
    for _ in 0..4 {
        game.step(&InputFrame::default());
        safe(&game);
    }
    assert_eq!(game.player.state, PlayerState::Standing);
    assert_eq!(game.player.pos, Vec2::new(104.0, 88.0));
    assert_eq!(game.player.vel, Vec2::default());
    assert!(game.player.grounded);
    assert_eq!(game.player.sprite, "sStandLeft");
    assert_eq!(game.player.data.get("image_xscale"), Some(&-1.0));
    game
}

fn safe(game: &Game) {
    assert_eq!(game.state, RunState::Playing);
    assert_eq!(game.player.health, 4.0);
    assert_eq!(game.player.stun, 0);
    assert!(!game.player.is_stunned());
    assert!(!game.world.solid_at(game.player.pos.x, game.player.pos.y));
}

fn point(game: &Game, at: Vec2, family: &str) -> bool {
    game.world.entities.iter().any(|e| {
        e.alive
            && e.engine_active
            && catalog::is_a(&e.object, family)
            && e.mask_contains(at, CollisionPrecision::BoundingBox)
    })
}

fn premises(game: &Game, origin: bool, upper_mask: bool) {
    // All ladders are default entity-backed actors in this fixture. Tile
    // storage contains only the original entrance, room bounds and Solid floor.
    assert_eq!(point(game, Vec2::new(104.0, 88.0), "oLadder"), origin);
    assert!(!point(game, Vec2::new(104.0, 88.0), "oLadderTop"));
    assert!(point(game, Vec2::new(104.0, 97.0), "oLadderTop"));
    assert!(game.world.rectangle_family_except(
        Vec2::new(99.0, 96.0),
        Vec2::new(108.0, 96.0),
        "oSolid",
        None,
        CollisionPrecision::Precise,
    ));
    assert!(!game.world.rectangle_family_except(
        Vec2::new(99.0, 80.0),
        Vec2::new(108.0, 95.0),
        "oPlatform",
        None,
        CollisionPrecision::Precise,
    ));
    // Original StandLeft RECTANGLE bounds2,0..13,15/origin8,8, mirrored by
    // the actual preceding Draw, translated from104,88 to104,80. At x104
    // y76 its mask overlaps the upper Orange's raw97..110/64..79 box.
    let moved = catalog::instance_mask(
        "oPlayer1",
        &game.player.sprite,
        Vec2::new(104.0, 80.0),
        game.player.animation,
        Vec2::new(-1.0, 1.0),
        0.0,
    );
    assert!(moved.contains(Vec2::new(104.0, 76.0), CollisionPrecision::Precise));
    let upper = game.world.entities.iter().any(|e| {
        e.alive
            && e.engine_active
            && e.object == "oLadderOrange"
            && e.pos == Vec2::new(96.0, 64.0)
            && e.mask_contains(Vec2::new(104.0, 76.0), CollisionPrecision::Precise)
    });
    assert_eq!(upper, upper_mask);
}

fn down(game: &mut Game) {
    game.step(&InputFrame {
        down: true,
        down_pressed: true,
        ..Default::default()
    });
    safe(game);
    assert_eq!(game.player.pos.x, 104.0);
    assert_eq!(game.player.vel, Vec2::default());
    assert_eq!(game.engine_time(), 6);
}

#[test]
fn ordinary_down_requires_origin_ladder_before_attaching_to_separate_upper_segment() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = room(rules, true, false);
        premises(&game, false, true);
        down(&mut game);
        // Source543 DUCKING on cached SolidBottom. Source612 then fails
        // colPointLadder even though below-cap and upper-mask tests are true.
        assert_eq!(
            game.player.state,
            PlayerState::Ducking,
            "SOURCE_DOWN_ATTACHMENT_REQUIRES_ORIGIN_LADDER_POINT"
        );
        assert_eq!(game.player.pos, Vec2::new(104.0, 88.0));
        assert!(game.player.grounded);
    }
}

#[test]
fn ordinary_down_without_upper_segment_keeps_solid_floor_ducking() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = room(rules, false, false);
        premises(&game, false, false);
        down(&mut game);
        assert_eq!(game.player.state, PlayerState::Ducking);
        assert_eq!(game.player.pos, Vec2::new(104.0, 88.0));
        assert!(game.player.grounded);
    }
}

#[test]
fn ordinary_down_with_origin_ladder_preserves_source_attachment() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = room(rules, false, true);
        premises(&game, true, false);
        down(&mut game);
        assert_eq!(game.player.state, PlayerState::Climbing);
        assert_eq!(game.player.pos, Vec2::new(104.0, 88.0));
        assert!(!game.player.grounded);
    }
}
