//! Source-derived COMPONENT/AUTHORED_ROOM fixture, not an
//! untouched generated campaign: retain one real generated dynamic LavaTop at
//! its original position, preserving all phase/transform/Create fields. Only
//! fresh identity bookkeeping changes before Step. Default Frog + neutral input.
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
    // Actual original producer phase, scale, angle and spurt fields are kept.
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
    // Authored initial grounded dry Player. No post-start pose/state changes.
    game.player.pos = Vec2::new(pos.x - 96.0, pos.y + 8.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.player.facing = 1;
    game.world.entrance = game.player.pos;
    game.camera = Vec2::new((pos.x - 160.0).max(0.0), (pos.y - 120.0).max(0.0));
    game.previous_camera = game.camera;
    let lava = if top {
        // Reserve a fresh identity, then keep the actual generated component
        // bytes except id/creation_serial. No sprite/frame/velocity edits.
        let id = game.world.spawn("oLava", pos);
        let serial = game.world.entity(id).unwrap().creation_serial;
        produced.id = id;
        produced.creation_serial = serial;
        *game.world.entity_mut(id).unwrap() = produced;
        id
    } else {
        // Matched full-box control uses native own Create, like custom tile 'l'.
        // It does not force the generated component back to a chosen sprite.
        game.spawn("oLava", pos)
    };
    let frog = game.spawn("oFrog", Vec2::new(pos.x, pos.y - 16.0));
    assert_eq!(game.world.entities.len(), 2);
    assert!(!game.world.fluid_dirty);
    let f = game.world.entity(frog).unwrap();
    assert_eq!((f.health, f.state, f.timer), (1, 0, 0));
    assert_eq!(f.vel, Vec2::default());
    assert_eq!(f.sprite, "sFrogLeft");
    assert_eq!(f.value("bloodless"), 0.0);
    assert_eq!(f.value("burning"), 0.0);
    assert_eq!(f.image_speed(), 0.4);
    println!(
        "INITIAL_COMPONENT rules={rules:?} canonical={canonical} top={top} attempt={attempt} origin={pos:?}"
    );
    (game, lava, frog)
}

fn admitted(game: &Game, lava: u64, frog: u64) {
    let f = game
        .world
        .entity(frog)
        .expect("natural Frog survives prerequisite Steps");
    let l = game
        .world
        .entity(lava)
        .expect("actual source-generated lava survives");
    assert!(f.alive && f.engine_active && !f.held);
    assert!(l.alive && l.engine_active);
    let view = game.simulation_view_rect();
    assert!(f.pos.x > view.x - 20.0 && f.pos.x < view.x + view.w + 4.0);
    assert!(f.pos.y > view.y - 20.0 && f.pos.y < view.y + view.h + 4.0);
    assert_eq!((f.health, f.state, f.timer), (1, 0, 0));
    assert_eq!(f.value("burning"), 0.0);
    assert_eq!(f.value("swimming"), 0.0);
    assert_eq!(f.value("inactive"), 0.0);
    assert_eq!(game.player.health, 4.0);
    assert_eq!(game.player.stun, 0);
    assert_eq!(game.player.state, PlayerState::Standing);
    assert!(game.player.held.is_none());
    assert_eq!(
        game.player.data.get("swimming").copied().unwrap_or(0.0),
        0.0
    );
    assert!((game.player.pos.x - f.pos.x).abs() >= 96.0);
    assert!(!game.world.fluid_dirty);
    assert!(l.value("spurt_counter") >= 90.0);
    assert!(
        game.world
            .entities
            .iter()
            .all(|e| e.id == lava || e.id == frog)
    );
}

fn run_case(rules: RulesPreset, canonical: bool, top: bool) {
    let (mut game, lava, frog) = scene(rules, canonical, top);
    let origin = game.world.entity(lava).unwrap().pos;
    let prior_rng = wire(&game);
    // Source oGame time starts1, increments before Step: first five clocks2..6.
    // Default dry Frog gravity.2 and source moveTo quantization give these y's.
    for (tick, delta) in [0.0, 0.0, 1.0, 1.0, 2.0].into_iter().enumerate() {
        admitted(&game, lava, frog);
        game.step(&InputFrame::default());
        admitted(&game, lava, frog);
        let f = game.world.entity(frog).unwrap();
        assert_eq!(game.engine_time(), (tick + 2) as u64);
        assert_eq!(f.pos, Vec2::new(origin.x, origin.y - 16.0 + delta));
        assert!((f.vel.y - 0.2 * (tick + 1) as f64).abs() < 1e-12);
        assert_eq!(f.vel.x, 0.0);
        assert_eq!(
            wire(&game),
            prior_rng,
            "no competing first-five-Step random producer"
        );
    }
    let before = game.world.entity(frog).unwrap().clone();
    let l = game.world.entity(lava).unwrap();
    assert_eq!(l.pos, origin);
    // Source endframe adds .4 and subtracts frame_count once on wrap:
    // seven-frame LavaTop reaches2; single-frame sLava wraps twice to≈0.
    let expected_phase = if top { 2.0 } else { 0.0 };
    assert!((l.animation - expected_phase).abs() < 1e-12);
    assert_eq!(l.sprite, if top { "sLavaTop" } else { "sLava" });
    assert_eq!(catalog::sprite_dimensions(&before.sprite), Some((16, 16)));
    let probe = Vec2::new(before.pos.x + 8.0, before.pos.y + 14.0);
    assert_eq!(probe, Vec2::new(origin.x + 8.0, origin.y));
    // Current public native-cache point API reproduces Source prec0's bbox.
    // No geometry/state of either actor is injected to establish this premise.
    assert!(
        l.mask_contains(probe, CollisionPrecision::BoundingBox),
        "NATIVE_CACHED_SURFACE_ROW_INCLUDED"
    );
    assert_eq!(
        l.sprite_bounds().contains(probe),
        !top,
        "ACTUAL_NATURAL_RAW_VS_SMOOTH_ROW_PREMISE"
    );
    assert!(
        !game
            .world
            .solid_point_except(Vec2::new(before.pos.x + 8.0, before.pos.y + 8.0), frog)
    );
    assert!(!game.world.solid_point_except(probe, frog));
    println!(
        "QUALIFIED_SURFACE rules={rules:?} canonical={canonical} top={top} frog={:?} wave={} native=true smooth={} clock={}",
        before.pos,
        l.animation,
        !top,
        game.engine_time()
    );
    let mut expected_rng = game.rng.clone();
    let blood: Vec<_> = (0..3)
        .map(|_| {
            let velocity = Vec2::new(
                expected_rng.unit() * 4.0 - expected_rng.unit() * 4.0,
                -1.0 - expected_rng.unit() * 2.0,
            );
            let gravity = f64::from(1 + expected_rng.range(6)) * 0.1;
            (velocity, gravity)
        })
        .collect();
    game.step(&InputFrame::default());
    assert_eq!(game.player.health, 4.0);
    assert_eq!(game.player.stun, 0);
    let born: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oBlood")
        .collect();
    assert_eq!(born.len(), 3, "SOURCE_FROG_LAVA_ANIMAL_DEATH_EXACT_3_BLOOD");
    assert_eq!(
        wire(&game),
        serde_json::to_value(&expected_rng).unwrap(),
        "SOURCE_LOWER_LAVA_FROG_DEATH_12_BLOOD_CREATE_DRAWS"
    );
    assert!(
        game.world.entity(frog).is_none(),
        "SOURCE_LOWER_LAVA_SURFACE_KILLS_DEFAULT_FROG"
    );
    for (drop, (velocity, gravity)) in born.into_iter().zip(blood) {
        // Blood370 was already passed when Frog554 creates it: no ownStep.
        assert_eq!(drop.pos, Vec2::new(before.pos.x + 8.0, before.pos.y + 8.0));
        assert_eq!(drop.vel, velocity);
        assert_eq!(drop.value("gravity"), gravity);
    }
    assert_eq!(
        game.data.get("frogs").copied().unwrap_or(0.0),
        0.0,
        "environment death uncredited"
    );
}

#[test]
fn source_generated_dynamic_lava_wave_uses_cached_bbox_for_default_frog_death() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for canonical in [false, true] {
            run_case(rules, canonical, true);
        }
    }
}

#[test]
fn native_custom_lava_full_box_agrees_on_same_default_frog_fall_death() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for canonical in [false, true] {
            run_case(rules, canonical, false);
        }
    }
}
