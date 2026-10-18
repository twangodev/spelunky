//! SOURCE-ONLY, UNCOMPILED/UNRUN. Authored component room, not an untouched
//! generated campaign: retain a real generated LavaTop component unchanged
//! except initial identity bookkeeping, then use a default healthy Damsel.
//! Neutral input only; no state/frame/velocity/HP/scale changes after birth.
use spelunky_core::*;
use std::collections::BTreeSet;

fn wire(game: &Game) -> serde_json::Value {
    serde_json::to_value(&game.rng).unwrap()
}

fn water_at(world: &World, point: Vec2) -> bool {
    world.tiles.iter().enumerate().any(|(index, tile)| {
        tile.engine_active
            && catalog::is_a(&tile.object, "oWater")
            && world
                .tile_collision_mask((index % world.width) as i32, (index / world.width) as i32)
                .contains(point, CollisionPrecision::BoundingBox)
    }) || world.entities.iter().any(|e| {
        e.alive
            && e.engine_active
            && catalog::is_a(&e.object, "oWater")
            && e.mask_contains(point, CollisionPrecision::BoundingBox)
    })
}

fn component(game: &mut Game) -> (Entity, usize, usize, usize) {
    // Bounded INITIAL producer selection reads shape/exposure only; never
    // runs simulation or selects by future damage/blood/effect outcomes.
    for attempt in 0..16 {
        let generated = generation::generate(&mut game.rng, 13, &BTreeSet::new());
        for lava in &generated.entities {
            if lava.alive
                && lava.object == "oLava"
                && lava.sprite == "sLavaTop"
                && lava.pos.x >= 128.0
                && lava.pos.x < generated.width as f64 * 16.0 - 32.0
                && lava.pos.y >= 64.0
                && lava.pos.y < generated.height as f64 * 16.0 - 32.0
                && !generated.solid_point_except(Vec2::new(lava.pos.x, lava.pos.y - 16.0), lava.id)
                && !water_at(&generated, Vec2::new(lava.pos.x, lava.pos.y - 16.0))
                && lava.pos.x.rem_euclid(16.0) == 0.0
                && lava.pos.y.rem_euclid(16.0) == 0.0
            {
                return (lava.clone(), generated.width, generated.height, attempt);
            }
        }
    }
    panic!("UNQUALIFIED_COMPONENT: no source-generated dynamic exposed LavaTop in 16 generations");
}

fn scene(rules: RulesPreset, canonical: bool, top: bool) -> (Game, u64, u64) {
    let mut game = if canonical {
        Game::shell(42, rules)
    } else {
        Game::legacy_shell(42, rules)
    };
    let (mut produced, width, height, attempt) = component(&mut game);
    let pos = produced.pos;
    assert_eq!(produced.animation, 0.0);
    assert_eq!(produced.image_speed(), 0.4);
    assert_eq!(
        produced.data.get("image_xscale").copied().unwrap_or(1.0),
        1.0
    );
    assert_eq!(
        produced.data.get("image_yscale").copied().unwrap_or(1.0),
        1.0
    );
    assert_eq!(produced.value("image_angle"), 0.0);
    assert_eq!(produced.value("lava_initialized"), 1.0);
    assert_eq!(produced.value("lava_setup_initialized"), 1.0);
    assert!(produced.value("spurt_counter") >= 100.0);
    assert_eq!(produced.vel, Vec2::default());
    game.world = World::new(width, height, Area::Temple, 13);
    game.world.name = "rLevel".into();
    let tx = (pos.x / 16.0) as usize;
    let ty = (pos.y / 16.0) as usize;
    for x in 0..width {
        game.world.set_tile(x, ty + 1, "oHardBlock");
    }
    game.world.set_tile(tx - 1, ty, "oHardBlock");
    game.world.set_tile(tx + 1, ty, "oHardBlock");
    game.player.pos = Vec2::new(pos.x - 96.0, pos.y + 8.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.player.facing = 1;
    game.world.entrance = game.player.pos;
    game.camera = Vec2::new((pos.x - 160.0).max(0.0), (pos.y - 120.0).max(0.0));
    game.previous_camera = game.camera;
    let lava = if top {
        let id = game.world.spawn("oLava", pos);
        let serial = game.world.entity(id).unwrap().creation_serial;
        produced.id = id;
        produced.creation_serial = serial;
        *game.world.entity_mut(id).unwrap() = produced;
        id
    } else {
        // Matched full-box control is native own Create; no sprite replacement.
        game.spawn("oLava", pos)
    };
    // Authored initial six-pixel-above-surface origin, stock Create/healthy pose.
    // The parent body does not move with initial zero velocity. Its center/sink
    // rectangle and y-5 Destroy point stay above the fluid before own foot+6.
    let damsel = game.spawn("oDamsel", Vec2::new(pos.x + 8.0, pos.y - 6.0));
    assert_eq!(game.world.entities.len(), 2);
    assert!(!game.world.fluid_dirty);
    let d = game.world.entity(damsel).unwrap();
    assert_eq!((d.health, d.state, d.timer), (4, 0, 200));
    assert_eq!(d.vel, Vec2::default());
    assert_eq!(d.sprite, "sDamselLeft");
    assert_eq!(d.facing, -1);
    assert_eq!(d.image_speed(), 0.5);
    assert_eq!(d.value("cost"), 195000.0);
    assert_eq!(d.value("forSale"), 0.0);
    for field in [
        "invincible",
        "dead",
        "death_recorded",
        "burning",
        "swimming",
        "inactive",
        "hit_cooldown",
    ] {
        assert_eq!(d.value(field), 0.0, "native initial {field}");
    }
    assert!(!d.held && d.alive && d.engine_active);
    assert_eq!(game.lifetime_stats.damsels_killed_total, 0);
    println!(
        "INITIAL_DAMSEL_COMPONENT rules={rules:?} canonical={canonical} top={top} attempt={attempt} origin={pos:?}"
    );
    (game, lava, damsel)
}

fn run_case(rules: RulesPreset, canonical: bool, top: bool) {
    let (mut game, lava, damsel) = scene(rules, canonical, top);
    let d = game.world.entity(damsel).unwrap().clone();
    let l = game.world.entity(lava).unwrap();
    let origin = l.pos;
    let probe = Vec2::new(d.pos.x, d.pos.y + 6.0);
    assert_eq!(probe, Vec2::new(origin.x + 8.0, origin.y));
    assert_eq!(l.animation, 0.0);
    assert_eq!(l.sprite, if top { "sLavaTop" } else { "sLava" });
    assert!(l.alive && l.engine_active);
    assert!(
        l.mask_contains(probe, CollisionPrecision::BoundingBox),
        "NATIVE_CACHED_DAMSEL_SURFACE_ROW_INCLUDED"
    );
    assert_eq!(
        l.sprite_bounds().contains(probe),
        !top,
        "SOURCE_GENERATED_NATIVE_VS_SMOOTH_DAMSEL_ROW"
    );
    assert!(!l.mask_contains(
        Vec2::new(probe.x, probe.y - 1.0),
        CollisionPrecision::BoundingBox
    ));
    assert!(l.mask_contains(
        Vec2::new(probe.x, probe.y + 1.0),
        CollisionPrecision::BoundingBox
    ));
    assert!(
        l.sprite_bounds()
            .contains(Vec2::new(probe.x, probe.y + 1.0))
    );
    assert_eq!(catalog::sprite_dimensions(&d.sprite), Some((16, 16)));
    let view = game.simulation_view_rect();
    assert!(
        d.pos.x > view.x - 16.0 && d.pos.x < view.x + view.w + 16.0,
        "inherited Item x admission"
    );
    assert!(
        d.pos.y > view.y - 16.0 && d.pos.y < view.y + view.h + 16.0,
        "inherited Item y admission"
    );
    assert!((game.player.pos.x - d.pos.x).abs() >= 96.0);
    assert!(!game.world.solid_point_except(d.pos, damsel));
    assert!(!game.world.solid_point_except(probe, damsel));
    assert!(
        !l.mask_contains(
            Vec2::new(d.pos.x, d.pos.y - 5.0),
            CollisionPrecision::BoundingBox
        ),
        "parent item Destroy remains absent"
    );
    assert!(
        d.pos.y + 3.0 < origin.y,
        "parent item Lava sink rectangle remains above"
    );
    assert!(
        !game
            .world
            .tiles
            .iter()
            .any(|tile| catalog::is_a(&tile.object, "oLava")),
        "no duplicate tile can conceal dynamic query gap"
    );
    assert_eq!(game.player.health, 4.0);
    assert_eq!(game.player.stun, 0);
    assert_eq!(game.player.state, PlayerState::Standing);
    assert!(game.player.grounded && game.player.held.is_none());
    assert_eq!(
        game.player.data.get("swimming").copied().unwrap_or(0.0),
        0.0
    );
    let prior_rng = wire(&game);
    println!(
        "QUALIFIED_DAMSEL_SOURCE_SURFACE rules={rules:?} canonical={canonical} top={top} pos={:?} probe={probe:?} clock={}",
        d.pos,
        game.engine_time()
    );
    game.step(&InputFrame::default());
    // Source admitted oItem precedes own Water→anger→burn→Lava. Default zero
    // motion and the initial clear sink/solid probes preserve this origin.
    let after = game
        .world
        .entity(damsel)
        .expect("source Lava leaves live DEAD receiver");
    assert_eq!(
        after.pos, d.pos,
        "parent zero-motion pose retained at child hazard"
    );
    assert!(after.alive && after.engine_active && !after.held);
    assert_eq!(
        after.value("cost"),
        0.0,
        "inherited item economics naturally clears unowned price"
    );
    assert_eq!(after.value("swimming"), 0.0);
    assert_eq!(after.value("invincible"), 0.0);
    assert_eq!(
        after.health, 0,
        "SOURCE_DAMSEL_OWN_PREC0_LAVA_SURFACE_HP_ZERO"
    );
    assert_eq!(after.state, 99, "source own lower Lava sets DEAD");
    assert_eq!(after.timer, 200, "DEAD path skips normal IDLE decrement");
    assert_eq!(after.value("dead"), 1.0);
    assert_eq!(after.value("death_recorded"), 1.0);
    assert_eq!(
        after.value("burning"),
        1.0,
        "burn branch precedes new Lava burning assignment"
    );
    assert_eq!(after.value("gravity"), 0.0);
    assert_eq!(after.vel, Vec2::new(0.0, 0.1));
    assert_eq!(after.value("depth"), 999.0);
    assert_eq!(
        after.sprite, "sDamselLeft",
        "Lava has no spike death-sprite assignment"
    );
    assert_eq!(after.image_speed(), 0.5);
    assert_eq!(after.animation, 0.5);
    assert_eq!(
        game.lifetime_stats.damsels_killed_total, 1,
        "own Lava lifetime death only"
    );
    for counter in ["kills", "damselsKilled", "damsels", "xdamsels"] {
        assert_eq!(
            game.data.get(counter).copied().unwrap_or(0.0),
            0.0,
            "uncredited {counter}"
        );
    }
    assert_eq!(game.lifetime_stats.damsels_saved_total, 0);
    assert_eq!(game.lifetime_stats.damsels_grabbed, 0);
    assert_eq!(
        wire(&game),
        prior_rng,
        "no same-Step burn RNG: old burning was zero"
    );
    assert!(
        game.world
            .entities
            .iter()
            .all(|e| e.id == lava || e.id == damsel),
        "no competing Blood/Burn/Splash/exit producer"
    );
    assert!(!game.events.iter().any(|e| matches!(e, GameEvent::Sound { name, .. } if ["sndDamsel", "sndHit", "sndHurt", "sndSplash"].contains(&name.as_str()))));
    assert_eq!(game.player.health, 4.0);
    assert_eq!(game.player.stun, 0);
    let shown = game.presentation();
    let draw = shown
        .world
        .entity(damsel)
        .expect("visible source DEAD receiver drawn");
    assert_eq!(draw.pos, d.pos);
    assert_eq!((draw.health, draw.state), (0, 99));
    assert_eq!(draw.value("depth"), 999.0);
    assert_eq!(draw.value("image_xscale"), 1.0, "native LEFT Draw scale");
    assert_eq!(draw.sprite, "sDamselLeft");
    assert_eq!(
        draw.animation, 0.0,
        "Draw snapshot precedes builtin animation .5"
    );
    assert_eq!(draw.value("burning"), 1.0);
    let live_lava = game.world.entity(lava).unwrap();
    assert_eq!(live_lava.pos, origin);
    assert!((live_lava.animation - 0.4).abs() < 1e-12);
    assert!(live_lava.value("spurt_counter") >= 99.0);
    assert!(!game.world.fluid_dirty);
}

#[test]
fn source_generated_lava_wave_kills_default_healthy_damsel_at_own_surface_probe() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for canonical in [false, true] {
            run_case(rules, canonical, true);
        }
    }
}

#[test]
fn native_plain_lava_full_box_control_kills_same_default_healthy_damsel() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for canonical in [false, true] {
            run_case(rules, canonical, false);
        }
    }
}
