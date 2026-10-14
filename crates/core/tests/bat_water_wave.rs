//! SOURCE-ONLY UNCOMPILED/UNRUN. COMPONENT/AUTHORED_ROOM, not untouched
//! campaign. Preserve one real generated LavaTop's original position and fields;
//! only fresh identity bookkeeping. Native default Bat, then neutral inputs.
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
                && lava.pos.x < generated.width as f64 * 16.0 - 64.0
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
    // Authored initial dry, grounded Player48 right: clear of sidewall+16..31.
    game.player.pos = Vec2::new(pos.x + 48.0, pos.y + 8.0);
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
        // Real native custom-style Create gives full sLava, not forced shape.
        game.spawn("oLava", pos)
    };
    let before_bat = wire(&game);
    let bat = game.spawn("oBat", Vec2::new(pos.x, pos.y - 16.0));
    assert_eq!(
        wire(&game),
        before_bat,
        "default Bat Create consumes no RNG"
    );
    assert_eq!(game.world.entities.len(), 2);
    assert!(!game.world.fluid_dirty);
    let b = game.world.entity(bat).unwrap();
    assert_eq!((b.health, b.state), (1, 0));
    assert_eq!(b.vel, Vec2::default());
    assert_eq!(b.sprite, "sBatHang");
    assert_eq!(b.image_speed(), 0.5);
    assert_eq!(b.bounds(), Rect::new(pos.x + 2.0, pos.y - 14.0, 12.0, 12.0));
    assert_eq!(b.value("flying"), 1.0);
    assert_eq!(b.value("burning"), 0.0);
    println!(
        "INITIAL_BAT_COMPONENT rules={rules:?} canonical={canonical} top={top} attempt={attempt} origin={pos:?}"
    );
    (game, lava, bat)
}

fn admitted(game: &Game, lava: u64, bat: u64) {
    let b = game.world.entity(bat).unwrap();
    let l = game.world.entity(lava).unwrap();
    assert!(b.alive && b.engine_active && !b.held);
    assert!(l.alive && l.engine_active);
    assert_eq!(b.health, 1);
    assert_eq!(b.value("inactive"), 0.0);
    assert_eq!(b.value("burning"), 0.0);
    let view = game.simulation_view_rect();
    assert!(b.pos.x > view.x - 20.0 && b.pos.x < view.x + view.w + 4.0);
    assert!(b.pos.y > view.y - 20.0 && b.pos.y < view.y + view.h + 4.0);
    assert_eq!(game.player.health, 4);
    assert_eq!(game.player.stun, 0);
    assert_eq!(game.player.state, PlayerState::Standing);
    assert_eq!(game.player.vel, Vec2::default());
    assert_eq!(
        game.player.data.get("swimming").copied().unwrap_or(0.0),
        0.0
    );
    assert!(!water_at(&game.world, game.player.pos));
    assert!(game.player.held.is_none());
    assert!(!game.world.fluid_dirty);
    assert!(l.value("spurt_counter") >= 90.0);
    assert_eq!(game.world.entities.len(), 2, "no competing actors/effects");
    assert!(
        game.world
            .entities
            .iter()
            .all(|e| e.id == lava || e.id == bat)
    );
}

fn clear_body_queries(game: &Game, bat: u64) {
    let b = game.world.entity(bat).unwrap();
    let p = b.pos;
    assert!(
        !game
            .world
            .solid_point_except(Vec2::new(p.x + 8.0, p.y - 1.0), bat),
        "HANG ceiling empty"
    );
    assert!(
        !game
            .world
            .solid_point_except(Vec2::new(p.x + 8.0, p.y + 8.0), bat),
        "Bat crush point empty"
    );
    for (first, second) in [
        (
            Vec2::new(p.x + 1.0, p.y + 2.0),
            Vec2::new(p.x + 1.0, p.y + 13.0),
        ),
        (
            Vec2::new(p.x + 14.0, p.y + 2.0),
            Vec2::new(p.x + 14.0, p.y + 13.0),
        ),
        (
            Vec2::new(p.x + 2.0, p.y + 1.0),
            Vec2::new(p.x + 13.0, p.y + 1.0),
        ),
        (
            Vec2::new(p.x + 2.0, p.y + 14.0),
            Vec2::new(p.x + 13.0, p.y + 14.0),
        ),
    ] {
        assert!(
            !game.world.solid_line_except(first, second, bat),
            "no direction override from Solid contacts"
        );
    }
}

fn run_case(rules: RulesPreset, canonical: bool, top: bool) {
    let (mut game, lava, bat) = scene(rules, canonical, top);
    let origin = game.world.entity(lava).unwrap().pos;
    let initial = game.world.entity(bat).unwrap().pos;
    let original_player = game.player.pos;
    let rng = wire(&game);
    admitted(&game, lava, bat);
    clear_body_queries(&game, bat);
    game.step(&InputFrame::default());
    admitted(&game, lava, bat);
    let b = game.world.entity(bat).unwrap();
    assert_eq!(
        (b.state, b.pos, b.vel),
        (1, initial, Vec2::default()),
        "native HANG->ATTACK without motion"
    );
    assert_eq!(b.sprite, "sBatHang");
    assert_eq!(game.player.pos, original_player);
    assert_eq!(wire(&game), rng);
    assert_eq!(
        game.events
            .iter()
            .filter(|e| matches!(e, GameEvent::Sound { name, .. } if name == "sndBat"))
            .count(),
        1
    );
    clear_body_queries(&game, bat);
    let l = game.world.entity(lava).unwrap();
    assert_eq!(l.pos, origin);
    assert_eq!(l.sprite, if top { "sLavaTop" } else { "sLava" });
    assert!((l.animation - 0.4).abs() < 1e-12);
    let lower = Vec2::new(initial.x + 8.0, initial.y + 16.0);
    let upper = Vec2::new(initial.x, initial.y + 12.0);
    assert_eq!(lower, Vec2::new(origin.x + 8.0, origin.y));
    assert!(
        l.mask_contains(lower, CollisionPrecision::BoundingBox),
        "source Water-family cached row included"
    );
    assert_eq!(
        l.sprite_bounds().contains(lower),
        !top,
        "actual LavaTop frame0 smooth row excluded"
    );
    assert!(
        !l.mask_contains(upper, CollisionPrecision::BoundingBox),
        "upper12 guard dry so source assigns new velocity"
    );
    let center = Vec2::new(initial.x + 8.0, initial.y + 8.0);
    assert!(center.distance(game.player.pos) < 160.0);
    assert!(game.player.pos.x > center.x && game.player.pos.y > center.y);
    println!(
        "QUALIFIED_BAT_WATER rules={rules:?} canonical={canonical} top={top} origin={origin:?} wave={} lower_native=true lower_smooth={} upper_native=false",
        l.animation, !top
    );
    game.step(&InputFrame::default());
    admitted(&game, lava, bat);
    let b = game.world.entity(bat).unwrap();
    assert_eq!((b.state, b.pos), (1, initial));
    assert_eq!(wire(&game), rng);
    // Source wet lower16 changes downward dir to90 before upper12 dry assign.
    // Native cos90 is tiny, not a claimed exact zero bit pattern.
    assert_eq!(b.vel.y, -1.0, "SOURCE_BAT_WET_AVOIDANCE_ASCENDS");
    assert!(b.vel.x.abs() < 1e-12, "source cos90 approximately zero");
    game.step(&InputFrame::default());
    admitted(&game, lava, bat);
    let b = game.world.entity(bat).unwrap();
    assert_eq!(
        b.pos,
        Vec2::new(initial.x, initial.y - 1.0),
        "next ordinary moveTo visibly ascends1"
    );
    assert_eq!(wire(&game), rng);
    assert!(
        !game
            .events
            .iter()
            .any(|e| matches!(e, GameEvent::Damage { .. }))
    );
}

#[test]
fn generated_dynamic_lava_wave_bat_uses_native_water_bbox_to_avoid_diving() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for canonical in [false, true] {
            run_case(rules, canonical, true);
        }
    }
}

#[test]
fn native_full_lava_water_query_bat_avoidance_agreement_control() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for canonical in [false, true] {
            run_case(rules, canonical, false);
        }
    }
}
