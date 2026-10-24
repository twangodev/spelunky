//! Default Rock/Treasure births at a legal authored fractional origin, followed
//! by one ordinary neutral frame. Literal source Lava rectangle corners gate
//! the sink; a separate exact-edge control must still sink.
use spelunky_core::{
    Area, CollisionPrecision, Game, GameEvent, InputFrame, Rect, RulesPreset, Vec2, World,
};

fn default_birth_rectangle_scene(object: &str, x: f64, rules: RulesPreset, sink: bool) {
    let gravity_field = if object == "oGoldBar" {
        "myGrav"
    } else {
        "gravity"
    };
    let mut game = Game::shell(42, rules);
    game.world = World::new(30, 20, Area::Mines, 1);
    game.world.name = "rLevel".into();
    for col in 0..30 {
        game.spawn("oHardBlock", Vec2::new(f64::from(col) * 16.0, 127.0));
    }
    game.player.pos = Vec2::new(200.0, 119.0);
    game.player.previous_pos = game.player.pos;
    let lava = game.spawn("oLava", Vec2::new(96.0, 96.0));
    let actor = game.spawn(object, Vec2::new(x, 114.0));
    let e = game.world.entity(actor).unwrap();
    assert_eq!(e.pos, Vec2::new(x, 114.0));
    assert_eq!(e.vel, Vec2::default());
    assert!(!e.held);
    assert!(e.engine_active);
    assert_eq!(e.value(gravity_field), 0.6);
    if object == "oGoldBar" {
        assert_eq!(e.state, 1, "native Treasure Create ACTIVE");
        assert_eq!(e.value("canPickUp"), 0.0);
        assert_eq!(e.value("canCollect"), 1.0);
        assert_eq!(e.value("value"), 500.0);
    } else {
        assert_eq!(object, "oRock");
        assert_eq!(e.value("safe"), 0.0);
        assert_eq!(e.value("heavy"), 0.0);
    }
    let liquid = game.world.entity(lava).unwrap();
    assert!(liquid.alive && liquid.engine_active);
    assert_eq!(liquid.pos, Vec2::new(96.0, 96.0));
    assert_eq!(liquid.value("spurt"), 0.0);
    assert!(liquid.value("spurt_counter") >= 100.0);
    assert_eq!(
        game.world.rectangle_family_except(
            Vec2::new(x - 3.0, 111.0),
            Vec2::new(x + 3.0, 117.0),
            "oLava",
            None,
            CollisionPrecision::BoundingBox,
        ),
        sink,
        "SOURCE_LITERAL_RECTANGLE: raw max x+3 is below L96 or exactly L96",
    );
    assert!(
        Rect::new(x - 3.0, 111.0, 7.0, 7.0).intersects(liquid.bounds()),
        "both initial poses overlap the previous expanded smooth adapter",
    );
    for y in [109.0, 109.05] {
        assert!(
            !liquid.mask_contains(Vec2::new(x, y), CollisionPrecision::BoundingBox),
            "final literal point must stay left of Lava even after .05 sink",
        );
    }
    assert!(
        !game.world.rectangle_family_except(
            Vec2::new(x - 5.0, 109.0),
            Vec2::new(x + 5.0, 119.05),
            "oSolid",
            None,
            CollisionPrecision::BoundingBox,
        ),
        "zero-displacement body and all contact lines are Solid-clear",
    );
    assert!(!game.world.solid_at(x, 114.0), "Item is not embedded");
    let view = game.simulation_view_rect();
    assert!(
        x > view.x - 16.0
            && x < view.x + view.w + 16.0
            && 114.0 > view.y - 16.0
            && 114.0 < view.y + view.h + 16.0
    );
    assert!(!game.player.sprite_bounds().intersects(e.sprite_bounds()));
    assert!(
        !game.world.rectangle_family_except(
            Vec2::new(192.0, 111.0),
            Vec2::new(208.0, 127.0),
            "oTreasure",
            None,
            CollisionPrecision::BoundingBox,
        ),
        "Player collect gate is false before the receiver Step",
    );
    assert!(!game.world.fluid_dirty);
    let next_id = game.world.next_id;
    let rng = game.rng.state_bits();
    // Initial creation/placement ends above: only an ordinary input frame below.
    game.step(&InputFrame::default());
    assert_eq!(game.player.pos, Vec2::new(200.0, 119.0));
    assert_eq!(game.player.health, 4.0);
    assert_eq!(game.player.invincible, 0);
    assert_eq!(game.player.stun, 0);
    assert!(!game.player.is_stunned());
    assert_eq!(game.player.held, None);
    assert_eq!(game.player.money, 0);
    assert_eq!(game.player.bombs, 4.0);
    assert_eq!(game.player.ropes, 4.0);
    assert_eq!(game.data.get("collect").copied().unwrap_or(0.0), 0.0);
    assert_eq!(game.data.get("goldbar").copied().unwrap_or(0.0), 0.0);
    assert!(
        !game
            .events
            .iter()
            .any(|event| matches!(event, GameEvent::Collected { .. }))
    );
    assert!(!game.world.fluid_dirty);
    assert_eq!(game.world.entity(lava).unwrap().pos, Vec2::new(96.0, 96.0));
    let e = game
        .world
        .entity(actor)
        .expect("literal Lava point misses: survivor observable");
    assert_eq!(
        e.pos,
        Vec2::new(x, if sink { 114.0 + 0.05 } else { 114.0 }),
        "SOURCE_LITERAL_RECTANGLE_SINK {object}: only a raw-corner hit adds .05"
    );
    assert_eq!(e.vel, Vec2::new(0.0, if sink { 0.0 } else { 0.6 }));
    assert_eq!(e.value(gravity_field), if sink { 0.0 } else { 0.6 });
    for field in ["colLeft", "colRight", "colTop", "colBot"] {
        assert_eq!(e.value(field), 0.0, "source contact prerequisite {field}");
    }
    assert!(e.alive && e.engine_active && !e.held);
    if object == "oGoldBar" {
        assert_eq!(e.state, 1);
        assert_eq!(e.value("canCollect"), 1.0);
        assert_eq!(e.value("value"), 500.0);
    }
    assert_eq!(
        game.rng.state_bits(),
        rng,
        "neither Lava rectangle result consumes RNG"
    );
    assert_eq!(
        game.world.next_id, next_id,
        "no pickup/debris/effect/actor births"
    );
    assert!(!game.events.iter().any(|event| matches!(event,
        GameEvent::Sound { name, .. } if name == "sndBreak" || name == "sndCoin" || name == "sndHit")));
}

#[test]
fn default_general_item_uses_literal_lava_rectangle_and_exact_edge_control() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (x, sink) in [(92.9995, false), (93.0, true)] {
            default_birth_rectangle_scene("oRock", x, rules, sink);
        }
    }
}

#[test]
fn default_native_treasure_uses_literal_lava_rectangle_and_exact_edge_control() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (x, sink) in [(92.9995, false), (93.0, true)] {
            default_birth_rectangle_scene("oGoldBar", x, rules, sink);
        }
    }
}
