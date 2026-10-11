//! Source qualified-owner witnesses. Natural routes use fresh bosses and only
//! ordinary inputs after setup. State7/activation/storage controls are labelled.
use spelunky_core::*;
fn shell(c: bool, r: RulesPreset, seed: u32) -> Game {
    let mut g = if c {
        Game::shell(seed, r)
    } else {
        Game::legacy_shell(u64::from(seed), r)
    };
    g.world = World::new(40, 24, Area::Boss, 16);
    g.world.name = "rLevel".into();
    g.player.pos = Vec2::new(500., 216.);
    g.player.previous_pos = g.player.pos;
    g.player.data.insert("active".into(), 0.);
    for x in 0..40 {
        g.world.set_tile(x, 14, "oBlock");
    }
    g
}
fn heading(e: &Entity, target: Vec2) {
    let p = e.previous_pos;
    let a = (target.y - p.y).atan2(target.x - p.x);
    let v = Vec2::new(2. * a.cos(), 2. * a.sin());
    assert!(
        (e.vel.x - v.x).abs() < 1e-12 && (e.vel.y - v.y).abs() < 1e-12,
        "FIRST_ACTIVE_QUALIFIED_OWNER_DIRECTION actual={:?} expected={:?}",
        e.vel,
        v
    );
    assert!((e.pos.x - p.x - v.x).abs() < 1e-12 && (e.pos.y - p.y - v.y).abs() < 1e-12);
}
fn prefix(e: &Entity) {
    assert_eq!(
        e.text.get("type").map(String::as_str),
        Some(""),
        "INHERITED_DRAWN_SPRITE_CREATE_TYPE"
    );
    assert_eq!(e.data.get("blinkToggle"), Some(&0.));
    assert_eq!(e.data.get("y_acc"), Some(&0.6));
    assert_eq!(e.data.get("y_vel"), Some(&0.0));
    assert_eq!(e.image_speed(), 0.4);
    assert_eq!(e.value("psychic_initialized"), 1.);
}
fn configured(c: bool, r: RulesPreset, mode: u8) {
    let mut g = shell(c, r, 42);
    let first = g.spawn("oOlmec", Vec2::new(80., 96.));
    let second = (mode != 2).then(|| g.spawn("oOlmec", Vec2::new(300., 96.)));
    let creator = if mode == 0 { second.unwrap() } else { first };
    let target = if mode == 3 {
        // Earlier boss leaves and rejoins the source active array after the later one.
        g.world.entity_mut(first).unwrap().engine_active = false;
        dispatcher::activate_region(&mut g, Rect::new(290., 80., 90., 90.));
        assert!(
            g.dispatcher
                .hidden_entities()
                .iter()
                .any(|e| e.id == first && !e.engine_active)
        );
        dispatcher::activate_object(&mut g, "oOlmec");
        assert_eq!(
            dispatcher::descendant_instances(&g, "oOlmec"),
            vec![
                dispatcher::Instance::Entity(second.unwrap()),
                dispatcher::Instance::Entity(first)
            ]
        );
        assert_eq!(
            g.dispatcher.registration_order, g.dispatcher.active_order,
            "EXISTING_FAMILY_API_TRACKS_REACTIVATED_ORDER"
        );
        Vec2::new(332., 112.)
    } else {
        Vec2::new(112., 112.)
    };
    g.world.entity_mut(creator).unwrap().state = 7;
    let origin = g.world.entity(creator).unwrap().pos;
    let mut rng = g.rng.clone();
    let positions: Vec<_> = (0..6)
        .map(|_| {
            Vec2::new(
                origin.x + 32. + f64::from(rng.range(33)) - f64::from(rng.range(33)),
                origin.y + 14. + f64::from(rng.range(33)) - f64::from(rng.range(33)),
            )
        })
        .collect();
    for _ in 0..3 {
        rng.unit();
        rng.range(4);
        rng.chance(1, 2);
    }
    g.step(&InputFrame::default());
    let mut effects: Vec<_> = g
        .world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oPsychicCreate2")
        .collect();
    effects.sort_by_key(|e| e.creation_serial);
    assert_eq!(effects.len(), 6);
    assert_eq!(g.world.entity(creator).unwrap().state, 0);
    for (e, p) in effects.into_iter().zip(positions) {
        assert_eq!(e.previous_pos, p);
        heading(e, target);
    }
    assert_eq!(g.rng.clone().next_u64(), rng.next_u64());
    assert_eq!(g.data.get("source_runtime_errors"), None);
}
fn later(c: bool, r: RulesPreset) {
    configured(c, r, 0)
}
fn first(c: bool, r: RulesPreset) {
    configured(c, r, 1)
}
fn single(c: bool, r: RulesPreset) {
    configured(c, r, 2)
}
fn reactivated(c: bool, r: RulesPreset) {
    configured(c, r, 3)
}
fn direct(c: bool, r: RulesPreset, mode: u8) {
    let mut g = shell(c, r, 42);
    let first = g.spawn("oOlmec", Vec2::new(80., 96.));
    let second = g.spawn("oOlmec", Vec2::new(300., 96.));
    let target = if mode == 0 {
        Vec2::new(112., 112.)
    } else {
        g.world.entity_mut(first).unwrap().engine_active = false;
        dispatcher::activate_region(&mut g, Rect::new(290., 80., 90., 90.));
        if mode == 1 {
            dispatcher::activate_object(&mut g, "oOlmec");
        }
        assert_eq!(
            dispatcher::descendant_instances(&g, "oOlmec")[0],
            dispatcher::Instance::Entity(second)
        );
        Vec2::new(332., 112.)
    };
    if mode == 1 {
        // Explicit storage-only perturbation leaves source family arrays unchanged.
        g.world.entities.reverse();
        assert_eq!(
            g.world
                .entities
                .iter()
                .find(|e| e.object == "oOlmec")
                .unwrap()
                .id,
            first
        );
    }
    let p = Vec2::new(220., 60.);
    let rng = g.rng.clone().next_u64();
    let id = g.spawn("oPsychicCreate2", p);
    let e = g.world.entity(id).unwrap();
    let a = (target.y - p.y).atan2(target.x - p.x);
    let v = Vec2::new(2. * a.cos(), 2. * a.sin());
    assert!(
        (e.vel.x - v.x).abs() < 1e-12 && (e.vel.y - v.y).abs() < 1e-12,
        "DIRECT_CREATE_USES_ACTIVE_FAMILY_NOT_WORLD_STORAGE"
    );
    prefix(e);
    let dir = e
        .data
        .get("dir")
        .copied()
        .expect("SOURCE_CREATE_ASSIGNS_NUMERIC_DIR");
    assert!((dir - (-a.to_degrees()).rem_euclid(360.0)).abs() < 1e-12);
    assert_eq!(e.pos, p);
    assert_eq!(g.rng.clone().next_u64(), rng);
}
fn direct_first(c: bool, r: RulesPreset) {
    direct(c, r, 0)
}
fn direct_reactivated(c: bool, r: RulesPreset) {
    direct(c, r, 1)
}
fn direct_inactive(c: bool, r: RulesPreset) {
    direct(c, r, 2)
}
fn missing(c: bool, r: RulesPreset) {
    let mut g = shell(c, r, 42);
    let p = Vec2::new(220., 60.);
    let id = g.spawn("oPsychicCreate2", p);
    assert_eq!(
        g.data.get("source_runtime_errors"),
        Some(&1.),
        "DIRECT_CREATE_MISSING_QUALIFIED_OWNER_ERROR_UNDER_IGNORE"
    );
    let e = g.world.entity(id).unwrap();
    prefix(e);
    assert!(!e.data.contains_key("dir"));
    assert_eq!(e.value("psychic_missing_dir"), 1.);
    g.step(&InputFrame::default());
    assert_eq!(g.data.get("source_runtime_errors"), Some(&2.));
    assert_eq!(g.world.entity(id).unwrap().pos, p);
}
fn configured_dir_override(c: bool, r: RulesPreset) {
    let mut g = shell(c, r, 42);
    g.spawn("oOlmec", Vec2::new(80.0, 96.0));
    let id = g.spawn("oPsychicCreate2", Vec2::new(220.0, 60.0));
    g.world
        .entity_mut(id)
        .unwrap()
        .data
        .insert("dir".into(), 90.0);
    g.step(&InputFrame::default());
    let effect = g.world.entity(id).unwrap();
    assert!(
        (effect.pos.x - 220.0).abs() < 1e-12 && (effect.pos.y - 58.0).abs() < 1e-12,
        "STEP_READS_STORED_DIR_RATHER_THAN_CONSTRUCTOR_MOVEMENT_CACHE"
    );
}
fn axis_directions(c: bool, r: RulesPreset) {
    // Literal native point_direction axes and coincident branch, before Step.
    for (pos, expected_dir, delta) in [
        (Vec2::new(82., 112.), 0., Vec2::new(2., 0.)),
        (Vec2::new(112., 82.), 270., Vec2::new(0., 2.)),
        (Vec2::new(142., 112.), 180., Vec2::new(-2., 0.)),
        (Vec2::new(112., 142.), 90., Vec2::new(0., -2.)),
        (Vec2::new(112., 112.), 0., Vec2::new(2., 0.)),
    ] {
        let mut g = shell(c, r, 42);
        g.spawn("oOlmec", Vec2::new(80., 96.));
        let id = g.spawn("oPsychicCreate2", pos);
        let effect = g.world.entity(id).unwrap();
        assert_eq!(
            effect.data.get("dir"),
            Some(&expected_dir),
            "LITERAL_SOURCE_AXIS_AND_COINCIDENT_DIR"
        );
        assert_eq!(effect.pos, pos);
        assert_eq!(effect.data.get("y_vel"), Some(&0.));
        // Preserve only literal Step movement expectation, independent of angle helper.
        g.step(&InputFrame::default());
        let effect = g.world.entity(id).unwrap();
        assert!((effect.pos.x - pos.x - delta.x).abs() < 1e-12);
        assert!((effect.pos.y - pos.y - delta.y).abs() < 1e-12);
    }
}
fn shop_wall(c: bool, r: RulesPreset) {
    let mut g = shell(c, r, 42);
    let id = g.spawn("oOlmec", Vec2::new(80., 96.));
    assert_eq!(
        g.world.entity(id).unwrap().data.get("shopWall"),
        Some(&0.),
        "OLMEC_OWN_CREATE_EXPLICIT_FALSE_SHOP_WALL"
    );
}
fn natural(c: bool, r: RulesPreset) {
    let mut g = shell(c, r, if c { 42 } else { 1 });
    for x in 0..40 {
        g.world.set_tile(x, 22, "oHardBlock");
    }
    for y in 0..24 {
        g.world.set_tile(0, y, "oHardBlock");
        g.world.set_tile(39, y, "oHardBlock");
    }
    for x in (5..9).chain(18..23) {
        g.world.set_tile(x, 10, "oBlock");
    }
    g.player.pos = Vec2::new(112., 216.);
    g.player.previous_pos = g.player.pos;
    let first = g.spawn("oOlmec", Vec2::new(300., 96.));
    let second = g.spawn("oOlmec", Vec2::new(80., 96.));
    for id in [first, second] {
        assert_eq!(g.world.entity(id).unwrap().state, -1);
        assert_eq!(g.world.entity(id).unwrap().vel, Vec2::default());
    }
    let mut flee = 0;
    let mut dir = 0;
    let mut preparing = false;
    for frame in 0..1600 {
        let a = g.world.entity(first).unwrap();
        let b = g.world.entity(second).unwrap();
        let danger = if b.state == 5 {
            Some(b.pos.x + 32.)
        } else if a.state == 5 {
            Some(a.pos.x + 32.)
        } else {
            None
        };
        if let Some(center) = danger {
            if !preparing {
                flee = 40;
                dir = if g.player.pos.x < 70. {
                    1
                } else if g.player.pos.x > 570. {
                    -1
                } else if g.player.pos.x < center {
                    -1
                } else {
                    1
                };
            }
        }
        preparing = danger.is_some();
        let create = b.state == 7;
        let serial = g.world.creation_serial;
        let input = InputFrame {
            left: flee > 0 && dir < 0,
            right: flee > 0 && dir > 0,
            ..InputFrame::default()
        };
        if flee > 0 {
            flee -= 1;
        }
        g.step(&input);
        assert_eq!(
            g.player.health, 4,
            "ORDINARY_CONTROLLER_SURVIVES_NO_HP_OR_STATE_INJECTION"
        );
        if create {
            assert!(frame > 250);
            assert_eq!(g.world.entity(second).unwrap().state, 0);
            let owner = g.world.entity(first).unwrap();
            assert!(owner.engine_active);
            let target = Vec2::new(owner.pos.x + 32., owner.pos.y + 16.);
            let mut effects: Vec<_> = g
                .world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == "oPsychicCreate2" && e.creation_serial > serial)
                .collect();
            effects.sort_by_key(|e| e.creation_serial);
            assert!(effects.len() >= 6);
            for e in effects.into_iter().rev().take(6) {
                heading(e, target);
            }
            return;
        }
    }
    panic!("FRESH_TWO_BOSS_ORDINARY_ROUTE_MUST_REACH_LATER_CREATE");
}
macro_rules! cases {
    ($m:ident,$f:ident) => {
        mod $m {
            use super::*;
            #[test]
            fn legacy_classic() {
                $f(false, RulesPreset::Classic)
            }
            #[test]
            fn legacy_assisted() {
                $f(false, RulesPreset::Assisted)
            }
            #[test]
            fn canonical_classic() {
                $f(true, RulesPreset::Classic)
            }
            #[test]
            fn canonical_assisted() {
                $f(true, RulesPreset::Assisted)
            }
        }
    };
}
cases!(natural_two_live, natural);
cases!(configured_later_creator, later);
cases!(positive_first_receiver, first);
cases!(positive_single_receiver, single);
cases!(configured_reactivated_receiver, reactivated);
cases!(direct_first_owner, direct_first);
cases!(direct_reactivated_storage_control, direct_reactivated);
cases!(direct_inactive_owner_control, direct_inactive);
cases!(direct_missing_owner_ignore, missing);
cases!(default_shop_wall_create, shop_wall);

cases!(configured_direction_override, configured_dir_override);

cases!(direct_axis_and_coincident, axis_directions);
