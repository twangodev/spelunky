//! Additional source point callers: default Rock (inherited Item) and default
//! Gold Bar (native Treasure). Legal fractional initial origin is authored;
//! one ordinary neutral Step, with healthy Player outside pickup/combat.
use spelunky_core::{
    Area, CollisionPrecision, Game, GameEvent, InputFrame, RulesPreset, Vec2, World,
};

fn default_birth_point_scene(object: &str, x: f64, rules: RulesPreset, expect_destroy: bool) {
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
    // Complete Lava's real Create before the first input, including its
    // own spurt RNG. Unchanged default spurt=false makes its Step RNG-free.
    let lava = game.spawn("oLava", Vec2::new(96.0, 96.0));
    let actor = game.spawn(object, Vec2::new(x, 116.0));
    let e = game.world.entity(actor).unwrap();
    assert_eq!(e.pos, Vec2::new(x, 116.0));
    assert_eq!(e.vel, Vec2::default());
    assert!(!e.held);
    assert!(e.engine_active);
    assert_eq!(e.value(gravity_field), 0.6);
    if object == "oGoldBar" {
        assert_eq!(e.state, 1, "native Treasure Create ACTIVE, not Item state");
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
    let point = Vec2::new(x, 111.0);
    assert_eq!(
        liquid.mask_contains(point, CollisionPrecision::BoundingBox),
        expect_destroy,
        "source literal point x,y-5 has no epsilon expansion"
    );
    assert!(
        !game.world.rectangle_family_except(
            Vec2::new(x - 3.0, 113.0),
            Vec2::new(x + 3.0, 119.0),
            "oLava",
            None,
            CollisionPrecision::BoundingBox
        ),
        "prior Lava rectangle must not zero gravity/velocity or add sink offset"
    );
    assert!(
        !game.world.rectangle_family_except(
            Vec2::new(x - 5.0, 111.0),
            Vec2::new(x + 5.0, 121.0),
            "oSolid",
            None,
            CollisionPrecision::BoundingBox
        ),
        "body and every one-pixel contact probe clear: no wall/ceiling/floor response"
    );
    assert!(
        !game.world.solid_at(x, 116.0),
        "generic Item origin not embedded"
    );
    let view = game.simulation_view_rect();
    assert!(
        x > view.x - 16.0
            && x < view.x + view.w + 16.0
            && 116.0 > view.y - 16.0
            && 116.0 < view.y + view.h + 16.0,
        "source own-Step viewport guard must admit the default receiver"
    );
    assert!(
        !game
            .player
            .sprite_bounds()
            .intersects(game.world.entity(actor).unwrap().sprite_bounds())
    );
    assert!(
        !game.world.rectangle_family_except(
            Vec2::new(192.0, 111.0),
            Vec2::new(208.0, 127.0),
            "oTreasure",
            None,
            CollisionPrecision::BoundingBox
        ),
        "Player treasure pickup gate is false before the receiver own Step"
    );
    assert!(!game.world.fluid_dirty);
    let first_birth = game.world.next_id;
    let rng = game.rng.state_bits();
    let before_pos = game.world.entity(actor).unwrap().pos;
    // All initial creation and authored placement ends above. No health,
    // actor/state/velocity/held/RNG/timer or environmental writes during play.
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
    assert_eq!(
        game.world.entity(actor).is_none(),
        expect_destroy,
        "SOURCE_OTHER_ITEM_TREASURE_POINT {object} point={point:?}"
    );
    if expect_destroy {
        assert!(
            !game.world.entities.iter().any(|e| e.id == actor),
            "real own/parent Destroy followed by complete frame retirement"
        );
    } else {
        let e = game.world.entity(actor).unwrap();
        assert_eq!(
            e.pos, before_pos,
            "default zero incoming velocity moves zero pixels"
        );
        assert_eq!(
            e.vel,
            Vec2::new(0.0, 0.6),
            "gravity is retained, not a Lava sink"
        );
        assert_eq!(e.value(gravity_field), 0.6);
        assert_eq!(e.value("colLeft"), 0.0);
        assert_eq!(e.value("colRight"), 0.0);
        assert_eq!(e.value("colTop"), 0.0);
        assert_eq!(e.value("colBot"), 0.0);
        assert!(!e.held);
        if object == "oGoldBar" {
            assert_eq!(e.state, 1);
            assert_eq!(e.value("canCollect"), 1.0);
            assert_eq!(e.value("value"), 500.0);
        }
    }
    assert_eq!(
        game.rng.state_bits(),
        rng,
        "neither literal point result consumes RNG"
    );
    assert_eq!(
        game.world.next_id, first_birth,
        "no debris, pickup or actor births"
    );
    assert!(!game.events.iter().any(|event| matches!(event,
        GameEvent::Sound { name, .. } if name == "sndBreak" || name == "sndCoin" || name == "sndHit")));
}

#[test]
fn default_general_item_keeps_exact_lava_point_and_clear_edge_control() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (x, destroy) in [(95.9995, false), (96.0, true)] {
            default_birth_point_scene("oRock", x, rules, destroy);
        }
    }
}

#[test]
fn default_native_treasure_keeps_exact_lava_point_and_clear_edge_control() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (x, destroy) in [(95.9995, false), (96.0, true)] {
            default_birth_point_scene("oGoldBar", x, rules, destroy);
        }
    }
}
