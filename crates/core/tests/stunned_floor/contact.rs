//! SOURCE-ONLY UNRUN fixture revision. No timing or witness is accepted.
//! Runnable private acceptance tests; executor needs explicit ROOT runtime GO.
//! At most3172 Game.step calls across8 friction+12 branch+4 ordinary cases.
//! Initial authored actors only; every later mutation is Game.step(input).
//! New authored Solid walkout/trap route; reachability remains UNRUN.
//! Integration-test proposal; coordinator owns copying/build/runtime approval.
use spelunky_core::*;

fn value(g: &Game, key: &str) -> f64 {
    g.player.data.get(key).copied().unwrap_or(0.)
}
fn room(
    rules: RulesPreset,
    character: characters::Character,
    trap: bool,
    cached_down: bool,
    branch_top: Option<f64>,
) -> Game {
    let mut level = level::Level::default();
    level.set_tile(20, 8, '@').unwrap();
    // One ordinary solid starting cell. A one-way starting branch can retain
    // the player after the source-correct overlap bounce, masking lower probes.
    level.set_tile(20, 9, '1').unwrap();
    let floor_row = if branch_top.is_some() { 19 } else { 13 };
    for x in 0..40 {
        level.set_tile(x, floor_row, '1').unwrap();
    }
    let mut g = Game::shell(42, rules);
    g.load_custom(&level, "stunned-floor-natural-proposal", "");
    characters::apply(&mut g, character);
    if let Some(y) = branch_top {
        for column in 1..41 {
            g.spawn("oTreeBranchTile", Vec2::new(column as f64 * 16., y));
        }
    }
    if trap {
        let name = if cached_down {
            "oArrowTrapLeft"
        } else {
            "oArrowTrapRight"
        };
        // Below the starting cell, so its detector has a clear outward ray.
        // By this height the ordinary walkout has cleared the cell's sides.
        g.spawn(name, Vec2::new(336., 192.));
    }
    g
}
fn rect_hit(g: &Game, r: Rect, group: &str) -> bool {
    g.world
        .tiles_collide_precision(r, group, CollisionPrecision::Precise)
        || g.world.entities.iter().any(|e| {
            e.alive
                && !e.held
                && (catalog::is_a(&e.object, group) || group == "oSolid" && e.solid)
                && e.mask_intersects(r, CollisionPrecision::Precise)
        })
}
fn entity_line(e: &Entity, r: Rect) -> bool {
    let scale = Vec2::new(
        e.data.get("image_xscale").copied().unwrap_or(1.),
        e.data.get("image_yscale").copied().unwrap_or(1.),
    );
    let angle = e
        .data
        .get("image_angle")
        .map(|a| -a.to_radians())
        .unwrap_or_else(|| {
            if matches!(e.object.as_str(), "oArrow" | "oFishBone") {
                e.value("rotation")
            } else {
                0.
            }
        });
    catalog::instance_mask(&e.object, &e.sprite, e.pos, e.animation, scale, angle)
        .intersects_axis_line(
            Vec2::new(r.x, r.y),
            Vec2::new(r.x + r.w - 1., r.y + r.h - 1.),
            CollisionPrecision::Precise,
        )
}
fn line_hit(g: &Game, r: Rect, group: &str) -> bool {
    g.world.tiles_line_precision(
        Vec2::new(r.x, r.y),
        Vec2::new(r.x + r.w - 1., r.y + r.h - 1.),
        group,
        CollisionPrecision::Precise,
    ) || g.world.entities.iter().any(|e| {
        e.alive
            && !e.held
            && (catalog::is_a(&e.object, group) || group == "oSolid" && e.solid)
            && entity_line(e, r)
    })
}
// Rect values below encode source inclusive line endpoints; line_hit uses the
// distinct axis-line API. Body-overlap alone uses precise rectangle queries.
// Query custom movement bounds, never Player sprite_bounds.
fn bottom(g: &Game) -> Rect {
    let r = g.player.bounds();
    Rect::new(
        r.x.round(),
        (r.y + r.h).round(),
        (r.x + r.w - 1.).round() - r.x.round() + 1.,
        1.,
    )
}
fn top(g: &Game) -> Rect {
    let r = g.player.bounds();
    Rect::new(
        r.x.round(),
        (r.y - 1.).round(),
        (r.x + r.w - 1.).round() - r.x.round() + 1.,
        1.,
    )
}
fn side(g: &Game, right: bool) -> Rect {
    let r = g.player.bounds();
    Rect::new(
        (if right { r.x + r.w } else { r.x - 1. }).round(),
        r.y.round(),
        1.,
        (r.y + r.h - 1.).round() - r.y.round() + 1.,
    )
}
fn isolated(g: &Game) -> bool {
    g.player.is_stunned()
        && g.player.stun > 1
        && g.player.health == 2
        && !line_hit(g, top(g), "oSolid")
        && !line_hit(g, side(g, false), "oSolid")
        && !line_hit(g, side(g, true), "oSolid")
        && !g
            .world
            .entities
            .iter()
            .any(|e| e.alive && e.object == "oArrow")
}
fn diagnostic(g: &Game) -> String {
    format!(
        "tick={} pos={:?} vel={:?} state={:?} sprite={} hp={} stun={} bounced={} cachedDown={} bounds={:?}",
        g.tick,
        g.player.pos,
        g.player.vel,
        g.player.state,
        g.player.sprite,
        g.player.health,
        g.player.stun,
        value(g, "bounced"),
        value(g, "source_k_down"),
        g.player.bounds()
    )
}
fn near(actual: f64, expected: f64, context: &str) {
    assert!(
        (actual - expected).abs() < 1e-12,
        "CONTRACT actual={actual:.17} expected={expected:.17} {context}"
    );
}
fn settle(g: &mut Game) {
    for _ in 0..4 {
        g.step(&InputFrame::default());
    }
    assert!(
        g.player.grounded && g.player.pos == Vec2::new(344., 152.),
        "FIXTURE_TIMING settle {}",
        diagnostic(g)
    );
}
fn hit(g: &mut Game, down: bool) {
    let mut departed = false;
    for _ in 0..40 {
        if !departed
            && g.player.state == PlayerState::Falling
            && !g.player.grounded
            && g.player.vel.y > 0.
            && g.player.bounds().y + g.player.bounds().h > 160.
        {
            assert_eq!(
                g.player.health,
                4,
                "FIXTURE_TIMING injury before departure {}",
                diagnostic(g)
            );
            assert!(
                !g.player.is_stunned(),
                "FIXTURE_TIMING stun before departure {}",
                diagnostic(g)
            );
            assert_eq!(
                value(g, "duck_to_hang"),
                0.,
                "FIXTURE_TIMING ledge departure {}",
                diagnostic(g)
            );
            departed = true;
        }
        // Walk out without Down, then release horizontal input after an
        // observed fall. Down thereafter tests the cached-input contract.
        g.step(&InputFrame {
            down: down && departed,
            left: down && !departed,
            right: !down && !departed,
            ..Default::default()
        });
        assert_eq!(
            g.state,
            RunState::Playing,
            "FIXTURE_TIMING left gameplay {}",
            diagnostic(g)
        );
        assert_eq!(
            value(g, "duck_to_hang"),
            0.,
            "FIXTURE_TIMING unexpected ledge transition {}",
            diagnostic(g)
        );
        if g.player.is_stunned() {
            assert!(
                departed,
                "FIXTURE_TIMING Arrow arrived before walking off initial support {}",
                diagnostic(g)
            );
            assert!(
                !line_hit(g, bottom(g), "oSolid"),
                "FIXTURE_TIMING Arrow must hit before landing on lower solid floor {}",
                diagnostic(g)
            );
            assert_eq!(
                value(g, "source_k_down"),
                f64::from(down),
                "FIXTURE_TIMING intended controller Down was not sampled before Arrow {}",
                diagnostic(g)
            );
            assert_eq!(
                g.player.health,
                2,
                "FIXTURE_TIMING must be real Arrow damage {}",
                diagnostic(g)
            );
            assert_eq!(
                g.player.stun,
                20,
                "FIXTURE_TIMING real Arrow countdown {}",
                diagnostic(g)
            );
            assert!(
                if down {
                    g.player.vel.x < 0.
                } else {
                    g.player.vel.x > 0.
                },
                "FIXTURE_TIMING initial Arrow knockback sign {}",
                diagnostic(g)
            );
            assert!(
                g.events
                    .iter()
                    .any(|e| matches!(e, GameEvent::Damage { amount: 2 })),
                "FIXTURE_TIMING missing real damage event"
            );
            return;
        }
        assert_eq!(
            g.player.health,
            4,
            "FIXTURE_TIMING injury without expected Arrow stun {}",
            diagnostic(g)
        );
    }
    panic!("FIXTURE_TIMING no natural Arrow hit {}", diagnostic(g));
}
fn source_supported_velocity(g: &Game) -> Vec2 {
    let x = g.player.vel.x;
    // This fixture has no Ice; the source threshold is BEFORE damping.
    let vx = if x.abs() < 0.1 { 0. } else { x * 0.3 };
    let vy = g.player.vel.y + if value(g, "bounced") != 0. { 1. } else { 0.6 };
    Vec2::new(
        vx.clamp(-10., 10.),
        (if vy > 1. { -vy * 0.5 } else { 0. }).clamp(-10., 10.),
    )
}

fn friction_case(rules: RulesPreset, character: characters::Character, cached_down: bool) {
    let mut g = room(rules, character, true, cached_down, None);
    settle(&mut g);
    hit(&mut g, cached_down);
    let mut air_control = false;
    for _ in 0..96 {
        let support = line_hit(&g, bottom(&g), "oSolid");
        let eligible = isolated(&g);
        let threshold = eligible && support && (0.1..(1. / 3.)).contains(&g.player.vel.x.abs());
        let airborne = eligible && !support && !line_hit(&g, bottom(&g), "oPlatform");
        let before = diagnostic(&g);
        let expected = source_supported_velocity(&g);
        let air_x = g.player.vel.x.clamp(-10., 10.);
        let air_y =
            (g.player.vel.y + if value(&g, "bounced") != 0. { 1. } else { 0.6 }).clamp(-10., 10.);
        g.step(&InputFrame::default());
        assert_eq!(
            g.state,
            RunState::Playing,
            "FIXTURE_TIMING left gameplay {before}"
        );
        if airborne {
            near(g.player.vel.x, air_x, &before);
            near(g.player.vel.y, air_y, &before);
            air_control = true;
        }
        if threshold {
            near(g.player.vel.x, expected.x, &before);
            near(g.player.vel.y, expected.y, &before);
            assert!(
                g.player.vel.x != 0.,
                "CONTRACT source keeps subthreshold result for this contact {before}"
            );
            assert_eq!(
                value(&g, "bounced"),
                1.,
                "CONTRACT floor bounce flag {before}"
            );
            assert!(air_control, "FIXTURE_TIMING no separate airborne control");
            return;
        }
    }
    panic!(
        "FIXTURE_TIMING no isolated pre-damping threshold sample {}",
        diagnostic(&g)
    );
}

fn branch_contact(
    rules: RulesPreset,
    character: characters::Character,
    overlap: bool,
    cached_down: bool,
    branch_top: f64,
) -> bool {
    let mut g = room(rules, character, true, cached_down, Some(branch_top));
    settle(&mut g);
    hit(&mut g, cached_down);
    for _ in 0..96 {
        let lower = g.world.entities.iter().any(|e| {
            e.alive
                && e.object == "oTreeBranchTile"
                && !e.held
                && e.pos.y == branch_top
                && entity_line(e, bottom(&g))
        });
        let body = rect_hit(&g, g.player.bounds(), "oPlatform");
        // Keep this bounce witness independent of the threshold-order bug.
        let x = g.player.vel.x.abs();
        let candidate = isolated(&g)
            && lower
            && body == overlap
            && (x < 0.1 || x >= 1. / 3.)
            && !line_hit(&g, bottom(&g), "oSolid")
            && value(&g, "source_k_down") == f64::from(cached_down);
        let before = diagnostic(&g);
        let expected = source_supported_velocity(&g);
        // Opposite live Down proves the stunned bounce and retained cached input
        // are independent. Only Game.step changes post-start gameplay state.
        g.step(&InputFrame {
            down: !cached_down,
            ..Default::default()
        });
        assert_eq!(
            g.state,
            RunState::Playing,
            "FIXTURE_TIMING left gameplay {before}"
        );
        if candidate {
            near(g.player.vel.x, expected.x, &before);
            near(g.player.vel.y, expected.y, &before);
            assert_eq!(
                value(&g, "bounced"),
                1.,
                "CONTRACT platform-bottom bounce ignores body-overlap and kDown {before}"
            );
            assert_eq!(
                value(&g, "source_k_down"),
                f64::from(cached_down),
                "CONTRACT retained controller input {before}"
            );
            return true;
        }
    }
    false
}
#[test]
fn natural_arrow_solid_friction_threshold_precedes_damping() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for character in [
            characters::Character::Explorer,
            characters::Character::Damsel,
        ] {
            for cached_down in [false, true] {
                friction_case(rules, character, cached_down);
            }
        }
    }
}
#[test]
fn natural_arrow_overlap_platform_bounces_and_tangent_control() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for character in [
            characters::Character::Explorer,
            characters::Character::Damsel,
        ] {
            // Only the two previously authored adjacent heights, not a coordinate search.
            let mut saw_overlap = false;
            for y in [224., 225.] {
                saw_overlap |= branch_contact(rules, character, true, true, y);
            }
            assert!(
                saw_overlap,
                "FIXTURE_TIMING neither fixed height supplied body-overlap+bottom-line while stunned {rules:?}/{character:?}"
            );
            assert!(
                branch_contact(rules, character, false, false, 224.),
                "FIXTURE_TIMING no non-overlapping tangent control {rules:?}/{character:?}"
            );
        }
    }
}

fn ordinary_case(rules: RulesPreset, character: characters::Character) {
    let mut g = room(rules, character, false, false, Some(224.));
    settle(&mut g);
    for _ in 0..8 {
        g.step(&InputFrame::default());
        assert!(!g.player.is_stunned());
        assert!(g.player.grounded);
        assert_eq!(g.player.vel, Vec2::default());
    }
    let mut air = false;
    for _ in 0..80 {
        g.step(&InputFrame {
            right: true,
            ..Default::default()
        });
        assert!(
            !g.player.is_stunned(),
            "CONTRACT no-trap ordinary control {}",
            diagnostic(&g)
        );
        if g.player.vel.y > 0. && !g.player.grounded {
            air = true;
        }
        if air && g.player.grounded && g.player.pos.y == 216. {
            assert_eq!(value(&g, "bounced"), 0.);
            // moveTo clips position at first contact without rewriting yVel.
            // The next ordinary controller samples that bottom contact and
            // clears the velocity; keep the zero-speed oracle at that phase.
            g.step(&InputFrame::default());
            assert!(!g.player.is_stunned());
            assert!(g.player.grounded);
            assert_eq!(g.player.pos.y, 216.);
            assert_eq!(g.player.vel.y, 0.);
            assert_eq!(value(&g, "bounced"), 0.);
            return;
        }
    }
    panic!(
        "FIXTURE_TIMING no ordinary airborne-to-platform control {}",
        diagnostic(&g)
    );
}

#[test]
fn no_trap_standing_air_and_platform_landing_controls() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for character in [
            characters::Character::Explorer,
            characters::Character::Damsel,
        ] {
            ordinary_case(rules, character);
        }
    }
}
