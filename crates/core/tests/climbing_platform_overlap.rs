//! Ordinary Up attaches and climbs an initially authored concrete ladder.
//! Geometry/default actors are configured before Step. No later player,
//! velocity, health, timer, image or world writes; this is not a generated route.
use spelunky_core::*;

fn room(rules: RulesPreset, platform_y: Option<f64>) -> Game {
    let mut level = level::Level::default();
    level.set_tile(8, 4, '@').unwrap(); // raw entrance (152,88)
    for x in 0..40 {
        level.set_tile(x, 5, '1').unwrap(); // raw Solid floor y96
    }
    let mut g = Game::shell(42, rules);
    g.load_custom(&level, "platform-overlap-climbing", "");
    g.spawn("oLadderOrange", Vec2::new(144.0, 80.0));
    if let Some(y) = platform_y {
        g.spawn("oLadderTop", Vec2::new(144.0, y));
    }
    g
}
fn safe(g: &Game) {
    assert_eq!(g.state, RunState::Playing);
    assert_eq!(g.player.health, 4);
    assert_eq!(g.player.stun, 0);
    assert!(!g.player.is_stunned());
    assert!(!g.world.solid_at(g.player.pos.x, g.player.pos.y));
    assert_eq!(g.player.pos.x, 152.0);
}
fn contacts(g: &Game, body_platform: bool) {
    assert_eq!(g.player.pos, Vec2::new(152.0, 88.0));
    // Independent literal character bounds are [147,80,157,96] exclusive.
    // Original isCollisionBottom(1) samples inclusive x147..156, y96.
    assert!(g.world.tiles_line_precision(
        Vec2::new(147.0, 96.0),
        Vec2::new(156.0, 96.0),
        "oSolid",
        CollisionPrecision::Precise,
    ));
    // Original colPlat uses RAW bounds endpoints lb,tb,rb-1,bb-1.
    assert_eq!(
        g.world.rectangle_family_except(
            Vec2::new(147.0, 80.0),
            Vec2::new(156.0, 95.0),
            "oPlatform",
            None,
            CollisionPrecision::Precise,
        ),
        body_platform
    );
}
fn attach(g: &mut Game, body_platform: bool) {
    for _ in 0..4 {
        g.step(&InputFrame::default());
        safe(g);
    }
    assert!(g.player.grounded);
    contacts(g, body_platform);
    g.step(&InputFrame {
        up: true,
        up_pressed: true,
        ..Default::default()
    });
    safe(g);
    assert_eq!(g.player.state, PlayerState::Climbing);
    assert_eq!(g.player.vel, Vec2::default());
    contacts(g, body_platform);
    assert_eq!(g.engine_time(), 6);
}
fn close(actual: f64, expected: f64, marker: &str) {
    assert!(
        (actual - expected).abs() < 1e-12,
        "{marker}: actual={actual} expected={expected}"
    );
}
#[test]
fn natural_up_keeps_climbing_momentum_when_solid_and_platform_body_overlap() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut g = room(rules, Some(80.0));
        attach(&mut g, true);
        g.step(&InputFrame {
            up: true,
            ..Default::default()
        });
        safe(&g);
        contacts(&g, true);
        assert_eq!(g.player.state, PlayerState::Climbing);
        assert_eq!(g.engine_time(), 7);
        close(g.player.vel.y, -0.36, "FIRST_CLIMB_DEFAULT_ACCELERATION");
        // Source203 adds -0.6; source256 DOES NOT reset the saved -0.36
        // while colPlat is true; source679–682 select climbing friction0.6, applied at897.
        // The resulting -0.576 moves one pixel at source time8 (period2).
        g.step(&InputFrame {
            up: true,
            ..Default::default()
        });
        safe(&g);
        assert_eq!(g.player.state, PlayerState::Climbing);
        assert_eq!(g.engine_time(), 8);
        close(
            g.player.vel.y,
            -0.576,
            "SOURCE_PLATFORM_BODY_OVERLAP_PRESERVES_CLIMB_MOMENTUM",
        );
        assert_eq!(g.player.pos, Vec2::new(152.0, 87.0));
    }
}
#[test]
fn natural_up_still_stops_saved_vertical_speed_on_solid_without_platform_overlap() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for top in [None, Some(64.0)] {
            let mut g = room(rules, top);
            attach(&mut g, false);
            for source_time in [7, 8] {
                g.step(&InputFrame {
                    up: true,
                    ..Default::default()
                });
                safe(&g);
                contacts(&g, false);
                assert_eq!(g.player.state, PlayerState::Climbing);
                assert_eq!(g.engine_time(), source_time);
                close(g.player.vel.y, -0.36, "SOURCE_CLEAR_PLATFORM_SOLID_STOP");
            }
        }
    }
}
