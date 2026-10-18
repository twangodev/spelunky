//! Source Damsel Step307: raw Web rectangle, independent from sprite contact.
//! Source Item Step56 attaches at Hero.x−4, so a Left throw from168 reaches160.
//! All World/actor writes precede the first step.
use spelunky_core::*;
const PRESETS: [RulesPreset; 2] = [RulesPreset::Classic, RulesPreset::Assisted];
fn sound(g: &Game, wanted: &str) -> bool {
    g.events
        .iter()
        .any(|event| matches!(event,GameEvent::Sound{name,..} if name==wanted))
}
fn safe_step(g: &mut Game, input: &InputFrame) {
    g.step(input);
    assert_eq!(g.player.health, 4.0, "FIXTURE_HEALTHY_HERO");
    assert_eq!(g.player.stun, 0, "FIXTURE_NO_STUN_COUNTDOWN");
    assert!(!g.player.is_stunned(), "FIXTURE_NO_STUN_LATCH");
    assert!(
        !g.events
            .iter()
            .any(|event| matches!(event, GameEvent::Damage { .. })),
        "FIXTURE_NO_DAMAGE_EVENT"
    );
}
fn source_bottom(g: &Game, e: &Entity, offset: f64) -> bool {
    let b = e.bounds();
    let y = (b.y + b.h + offset - 1.0).round_ties_even();
    g.world.solid_line_except(
        Vec2::new(b.x.round_ties_even(), y),
        Vec2::new((b.x + b.w - 1.0).round_ties_even(), y),
        e.id,
    )
}
fn throw_at_web(rules: RulesPreset, web_x: f64, expected_timer: u32, source_inside: bool) {
    let mut g = Game::shell(7, rules);
    g.world = World::new(30, 20, Area::Mines, 1);
    g.world.name = "rLevel".into();
    for x in 0..30 {
        g.world.set_tile(x, 10, "oBlock");
    }
    g.player.pos = Vec2::new(168.0, 152.0);
    g.player.previous_pos = g.player.pos;
    g.player.state = PlayerState::Standing;
    g.player.grounded = true;
    g.player.facing = 1;
    g.world.entrance = g.player.pos;
    let damsel = g.spawn("oDamsel", Vec2::new(180.0, 152.0));
    // Legal initial free price metadata, not a paid-shop/campaign claim.
    g.world
        .entity_mut(damsel)
        .unwrap()
        .data
        .insert("cost".into(), 0.0);
    let web = g.spawn("oWeb", Vec2::new(web_x, 136.0));
    assert_eq!(catalog::object("oDamsel").unwrap().id, 556);
    assert_eq!(catalog::object("oWeb").unwrap().id, 751);
    assert!(damsel < web, "FIXTURE_EXPLICIT_BIRTH_ORDER");
    let target = g.world.entity(damsel).unwrap();
    assert_eq!(target.health, 4);
    assert_eq!(target.state, 0);
    assert_eq!(target.value("heavy"), 1.0);
    let target_web = g.world.entity(web).unwrap();
    assert_eq!(target_web.sprite, "sWeb");
    assert_eq!(target_web.pos, Vec2::new(web_x, 136.0));
    assert_eq!(target_web.value("life"), 12.0);
    assert_eq!(target_web.value("dying"), 0.0);
    assert!(!target_web.solid);
    assert!(target_web.engine_active);
    safe_step(
        &mut g,
        &InputFrame {
            down: true,
            attack_pressed: true,
            ..Default::default()
        },
    );
    assert_eq!(g.player.held, Some(damsel), "FIXTURE_REAL_HEALTHY_PICKUP");
    assert!(g.world.entity(damsel).unwrap().held);
    safe_step(
        &mut g,
        &InputFrame {
            left: true,
            ..Default::default()
        },
    );
    for _ in 0..8 {
        safe_step(&mut g, &InputFrame::default());
        assert_eq!(g.player.held, Some(damsel));
        assert_eq!(g.world.entity(damsel).unwrap().health, 4);
        assert_eq!(g.world.entity(web).unwrap().pos, Vec2::new(web_x, 136.0));
        assert_eq!(g.world.entity(web).unwrap().value("life"), 12.0);
    }
    assert_eq!(
        g.player.pos,
        Vec2::new(168.0, 152.0),
        "FIXTURE_FIXED_THROW_POSE"
    );
    assert_eq!(g.player.vel, Vec2::default());
    assert!(g.player.grounded);
    assert_eq!(g.player.state, PlayerState::Standing);
    assert_eq!(g.player.facing, -1);
    assert!(!g.player.has("Mitt"));
    let before = g.world.entity(damsel).unwrap().clone();
    assert!(before.held);
    assert_eq!(before.state, 0);
    assert_eq!(before.health, 4);
    assert_eq!(
        before.pos,
        Vec2::new(164.0, 152.0),
        "FIXTURE_SOURCE_PARENT_ATTACHMENT"
    );
    assert_eq!(before.vel, Vec2::default());
    assert_eq!(
        before.data.get("image_xscale").copied().unwrap_or(1.0),
        1.0,
        "FIXTURE_PRE_COLLISION_LEFT_DRAW_SCALE"
    );
    assert_eq!(before.value("dead"), 0.0);
    let saved = g.lifetime_stats.damsels_saved_total;
    let killed = g.lifetime_stats.damsels_killed_total;
    let grabbed = g.lifetime_stats.damsels_grabbed;
    let body_tick = g.level_tick;
    let rng = g.rng.state_bits();
    safe_step(
        &mut g,
        &InputFrame {
            attack_pressed: true,
            ..Default::default()
        },
    );
    assert_eq!(
        g.level_tick,
        body_tick + 1,
        "FIXTURE_EXACT_ONE_THROW_CALLBACK_TICK"
    );
    assert_eq!(g.player.held, None, "FIXTURE_REAL_THROW_RELEASE");
    assert!(sound(&g, "sndThrow"));
    assert!(sound(&g, "sndDamsel"));
    let e = g.world.entity(damsel).unwrap();
    assert!(e.alive && !e.held);
    assert_eq!(e.health, 4);
    assert_eq!(e.state, 2);
    assert_eq!(e.value("dead"), 0.0);
    assert_eq!(e.value("bounced"), 0.0);
    assert_eq!(e.alarms[2], 10, "FIXTURE_REAL_THROW_SAFE_ALARM");
    // Actual movement proves the parent's heavy−4,−2 displacement after the
    // throw's own y−4 shift. Web collision occurs later and changes no position.
    assert_eq!(
        e.pos,
        Vec2::new(before.pos.x - 4.0, before.pos.y - 6.0),
        "FIXTURE_REAL_PARENT_THROW_DISPLACEMENT"
    );
    assert_eq!(e.pos, Vec2::new(160.0, 146.0));
    assert_eq!(
        e.value("simulation_tick"),
        g.engine_time() as f64,
        "FIXTURE_OWN_STEP_BODY_TIME"
    );
    assert!(!source_bottom(&g, e, 1.0), "FIXTURE_SOURCE_BOTTOM1_FALSE");
    assert!(!source_bottom(&g, e, 2.0), "FIXTURE_SOURCE_BOTTOM2_FALSE");
    let shown = g.presentation();
    let shown_e = shown.world.entity(damsel).unwrap();
    let shown_web = shown.world.entity(web).unwrap();
    assert_eq!(shown_e.pos, e.pos);
    assert_eq!(shown_e.sprite, "sDamselDieLL");
    assert_eq!(shown_e.animation, 0.0, "FIXTURE_PRE_ANIMATION_THROW_FRAME");
    assert_eq!(
        shown_e.data.get("image_xscale").copied().unwrap_or(1.0),
        1.0
    );
    assert_eq!(shown_web.pos, Vec2::new(web_x, 136.0));
    assert_eq!(shown_web.sprite, "sWeb");
    assert_eq!(shown_web.animation, 0.0);
    assert!(shown_web.engine_active && shown_web.alive);
    assert_eq!(shown_web.value("life"), 12.0);
    assert_eq!(shown_web.value("dying"), 0.0);
    assert_eq!(
        shown_web.sprite_bounds(),
        Rect::new(web_x, 136.0, 16.0, 16.0),
        "FIXTURE_WEB_DEFAULT_SOURCE_BBOX"
    );
    assert_eq!(
        shown_e.sprite_bounds(),
        Rect::new(154.0, 138.0, 13.0, 15.0),
        "FIXTURE_THROW_SELECTED_SPRITE_BBOX"
    );
    assert!(
        shown_e
            .sprite_bounds()
            .intersects(shown_web.sprite_bounds()),
        "FIXTURE_VISUAL_MARGIN_OVERLAP"
    );
    let first = Vec2::new(e.pos.x - 4.0, e.pos.y - 6.0);
    let second = Vec2::new(e.pos.x + 4.0, e.pos.y + 8.0);
    assert_eq!(first, Vec2::new(156.0, 140.0));
    assert_eq!(second, Vec2::new(164.0, 154.0));
    assert_eq!(
        g.world.rectangle_family_except(
            first,
            second,
            "oWeb",
            None,
            CollisionPrecision::BoundingBox
        ),
        source_inside,
        "FIXTURE_SOURCE_RAW_WEB_RECTANGLE"
    );
    if source_inside {
        assert_eq!(shown_web.pos.x, second.x, "FIXTURE_TRUE_INCLUSIVE_BOUNDARY");
    } else {
        assert_eq!(
            shown_web.pos.x,
            second.x + 1.0,
            "FIXTURE_ONE_PIXEL_OUTSIDE_RAW_BOUNDARY"
        );
    }
    // Source Web own Item callback may reset velocity after Damsel's timer
    // decision. It never writes counter; exact native mask contact distinguishes
    // a stopped body from the preserved heavy-throw/gravity result.
    if shown_e.event_intersects(shown_web) {
        assert_eq!(
            e.vel,
            Vec2::default(),
            "FIXTURE_ACTUAL_WEB_ITEM_COLLISION_STOPPED_BODY"
        );
    } else {
        assert!(
            (e.vel.x + 4.0).abs() < 1e-9 && (e.vel.y + 1.4).abs() < 1e-9,
            "FIXTURE_UNCONTACTED_THROW_VELOCITY"
        );
    }
    assert_eq!(g.rng.state_bits(), rng, "FIXTURE_THROW_WEB_RECOVERY_NO_RNG");
    assert_eq!(g.lifetime_stats.damsels_saved_total, saved);
    assert_eq!(g.lifetime_stats.damsels_killed_total, killed);
    assert_eq!(
        g.lifetime_stats.damsels_grabbed,
        grabbed + 1,
        "FIXTURE_ONE_REAL_THROWN_ENTRY"
    );
    assert_eq!(
        e.timer, expected_timer,
        "DAMSEL_RECOVERY_USES_SOURCE_RAW_WEB_RECTANGLE"
    );
}
#[test]
fn ordinary_damsel_throw_visual_web_margin_does_not_advance_recovery() {
    for rules in PRESETS {
        throw_at_web(rules, 165.0, 120, false);
    }
}
#[test]
fn ordinary_damsel_throw_inside_raw_inclusive_web_boundary_advances_recovery() {
    for rules in PRESETS {
        throw_at_web(rules, 164.0, 119, true);
    }
}
