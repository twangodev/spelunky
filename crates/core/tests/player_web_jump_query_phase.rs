//! Configured incoming off-center Climbing poses, then one ordinary Jump.
//! This proves the controller query phase, not a stock producer of that pose.
//! No health, velocity, timer, camera, sprite, or post-Step world overrides.
use spelunky_core::*;

fn room(rules: RulesPreset, web_x: Option<f64>) -> (Game, Option<u64>) {
    let mut level = level::Level::default();
    level.set_tile(5, 4, '@').unwrap(); // Source entrance (104,88).
    for x in 0..40 {
        level.set_tile(x, 9, '1').unwrap(); // Safe lower floor y160.
    }
    let mut game = Game::shell(42, rules);
    game.load_custom(&level, "configured-web-jump-query-phase", "");
    game.spawn("oLadderOrange", Vec2::new(96.0, 80.0));
    let web = web_x.map(|x| game.spawn("oWeb", Vec2::new(x, 80.0)));
    // Initial phase configuration only. No claimed ordinary producer of the
    // one-pixel off-center Climbing pose; all following input is immutable.
    game.player.pos = Vec2::new(105.0, 88.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Climbing;
    assert_eq!(game.player.vel, Vec2::default());
    assert_eq!(game.player.sprite, "sStandLeft");
    assert_eq!(game.player.animation, 0.0);
    assert_eq!(game.player.data.get("active"), Some(&1.0));
    assert_eq!(game.player.attack_timer, 0);
    assert!(!game.world.rectangle_family_except(
        Vec2::new(100.0, 80.0),
        Vec2::new(109.0, 95.0),
        "oSolid",
        None,
        CollisionPrecision::Precise,
    ));
    assert!(!game.world.rectangle_family_except(
        Vec2::new(100.0, 79.0),
        Vec2::new(109.0, 79.0),
        "oWater",
        None,
        CollisionPrecision::Precise,
    ));
    let ladder = game
        .world
        .entities
        .iter()
        .find(|e| e.object == "oLadderOrange")
        .unwrap();
    assert!(ladder.alive && ladder.engine_active);
    assert_eq!(ladder.pos, Vec2::new(96.0, 80.0));
    assert!(ladder.mask_contains(Vec2::new(105.0, 88.0), CollisionPrecision::BoundingBox));
    if let Some(id) = web {
        let target = game.world.entity(id).unwrap();
        assert!(target.alive && target.engine_active);
        assert_eq!(target.sprite, "sWeb");
        assert_eq!(target.animation, 0.0);
        assert_eq!(target.value("life"), 12.0);
        assert_eq!(target.value("dying"), 0.0);
    }
    (game, web)
}

fn premise(game: &Game, id: u64, entry_hit: bool, snapped_hit: bool, common_pixel: Vec2) {
    let web = game.world.entity(id).unwrap();
    assert_eq!(
        web.mask_contains(Vec2::new(105.0, 88.0), CollisionPrecision::BoundingBox),
        entry_hit
    );
    assert_eq!(
        web.mask_contains(Vec2::new(104.0, 88.0), CollisionPrecision::BoundingBox),
        snapped_hit
    );
    // Independent Original PNG controls: StandLeft is RECTANGLE/AUTO with
    // origin8,8 and bounds2,0..13,15; Web is PRECISE, origin0,0. The literal
    // common opaque pixel establishes instance_place eligibility after snap.
    let player = catalog::instance_mask(
        "oPlayer1",
        "sStandLeft",
        Vec2::new(104.0, 88.0),
        0.0,
        Vec2::new(1.0, 1.0),
        0.0,
    );
    assert!(player.contains(common_pixel, CollisionPrecision::Precise));
    assert!(web.mask_contains(common_pixel, CollisionPrecision::Precise));
}

fn jump(game: &mut Game) {
    game.step(&InputFrame {
        jump: true,
        jump_pressed: true,
        ..Default::default()
    });
    assert_eq!(game.state, RunState::Playing);
    assert_eq!(game.player.health, 4.0);
    assert_eq!(game.player.stun, 0);
    assert!(!game.player.is_stunned());
    assert_eq!(game.player.state, PlayerState::Jumping);
    assert_eq!(game.player.pos.x, 104.0);
    assert_eq!(game.player.vel.x, 0.0);
    assert_eq!(game.engine_time(), 2);
    assert_eq!(game.player.data.get("ladder_timer"), Some(&5.0));
    assert_eq!(game.player.jump_time, 1);
    assert!(!game.world.solid_at(game.player.pos.x, game.player.pos.y));
}

fn life(game: &Game, id: u64) -> f64 {
    let web = game.world.entity(id).unwrap();
    assert!(web.alive && web.engine_active);
    assert_eq!(web.value("dying"), 0.0);
    web.value("life")
}

#[test]
fn configured_ladder_snap_enters_web_before_the_source_jump_point_gate() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, id) = room(rules, Some(89.0));
        let id = id.unwrap();
        // Web bbox89..104 includes the snapped origin, not incoming105.
        // Original Web local13,8 is opaque at world102,88.
        premise(&game, id, false, true, Vec2::new(102.0, 88.0));
        jump(&mut game);
        assert_eq!(
            life(&game, id),
            11.0,
            "SOURCE_WEB_JUMP_POINT_AFTER_CLIMBING_SNAP_ENTERS"
        );
        // climb-4 + gravity1 + Web-4 is clamped to acceleration-6;
        // Web subtracts3 from yVel, then late Web friction0.2 => -1.8.
        assert!((game.player.vel.y + 1.8).abs() < 1e-12);
        assert_eq!(game.player.pos.y, 86.0);
    }
}

#[test]
fn configured_ladder_snap_leaves_web_before_the_source_jump_point_gate() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, id) = room(rules, Some(105.0));
        let id = id.unwrap();
        // Web bbox105..120 includes incoming105 but not snapped104.
        // Its local1,8 opaque pixel106,88 still overlaps the Player mask;
        // that alone cannot replace the separate source origin-point gate.
        premise(&game, id, true, false, Vec2::new(106.0, 88.0));
        jump(&mut game);
        assert_eq!(
            life(&game, id),
            12.0,
            "SOURCE_WEB_JUMP_POINT_AFTER_CLIMBING_SNAP_LEAVES"
        );
        assert_eq!(game.player.vel.y, -3.0); // climb-4 + gravity1, dry friction1.
        assert_eq!(game.player.pos.y, 85.0);
    }
}

#[test]
fn configured_equal_origin_web_gate_and_dry_ladder_jump_keep_their_source_results() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for web_x in [Some(96.0), None] {
            let (mut game, id) = room(rules, web_x);
            if let Some(id) = id {
                premise(&game, id, true, true, Vec2::new(105.0, 88.0));
            }
            jump(&mut game);
            if let Some(id) = id {
                assert_eq!(life(&game, id), 11.0);
                assert!((game.player.vel.y + 1.8).abs() < 1e-12);
                assert_eq!(game.player.pos.y, 86.0);
            } else {
                assert_eq!(game.player.vel.y, -3.0);
                assert_eq!(game.player.pos.y, 85.0);
            }
        }
    }
}
