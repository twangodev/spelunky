//! Configured query/activation controls, not an ordinary campaign witness.
//! No candidate-only API: public model births and activation operations prepare
//! the source roster; existing private hurt entry points execute the query.
use crate::{
    dispatcher::{self, Instance},
    *,
};
const RULES: [RulesPreset; 2] = [RulesPreset::Classic, RulesPreset::Assisted];
fn scene(rules: RulesPreset) -> Game {
    let mut g = Game::shell(7, rules);
    g.world = World::new(40, 24, Area::Ice, 9);
    g.world.name = "rLevel".into();
    g.player.pos = Vec2::new(160., 160.);
    g.player.previous_pos = g.player.pos;
    g.player.health = 30.0;
    // Configured purity control: avoid legitimate Blood Create syncing pending
    // births. LaserExplode is the existing raw World-spawn deferred producer.
    g.player.data.insert("bloodless".into(), 1.);
    g
}
fn ids(g: &Game, obj: &str) -> Vec<u64> {
    dispatcher::descendant_instances(g, obj)
        .into_iter()
        .filter_map(|i| {
            if let Instance::Entity(id) = i {
                Some(id)
            } else {
                None
            }
        })
        .collect()
}
fn storage(g: &Game, obj: &str) -> Vec<u64> {
    g.world
        .entities
        .iter()
        .filter(|e| e.object == obj && e.alive && e.engine_active)
        .map(|e| e.id)
        .collect()
}
fn reordered(rules: RulesPreset, obj: &str, ax: f64, bx: f64) -> (Game, u64, u64, u64) {
    let mut g = scene(rules);
    let a = g.spawn(obj, Vec2::new(ax, 160.));
    let b = g.spawn(obj, Vec2::new(bx, 160.));
    let c = g.spawn(obj, Vec2::new(304., 160.));
    assert_eq!(ids(&g, obj), vec![a, b, c]);
    // Two separate membership operations produce inactive order A,B but
    // hidden storage B,A. Reactivation appends the former to the active family.
    g.world.entity_mut(a).unwrap().engine_active = false;
    dispatcher::activate_region(&mut g, Rect::new(296., 152., 32., 32.));
    assert_eq!(ids(&g, obj), vec![b, c]);
    g.world.entity_mut(b).unwrap().engine_active = false;
    dispatcher::activate_region(&mut g, Rect::new(296., 152., 32., 32.));
    assert_eq!(ids(&g, obj), vec![c]);
    assert_eq!(
        g.dispatcher
            .hidden_entities()
            .iter()
            .filter(|e| e.object == obj)
            .map(|e| e.id)
            .collect::<Vec<_>>(),
        vec![b, a]
    );
    dispatcher::activate_object(&mut g, obj);
    assert_eq!(ids(&g, obj), vec![c, a, b]);
    assert_eq!(storage(&g, obj), vec![c, b, a]);
    assert!(g.dispatcher.hidden_entities().is_empty());
    for id in [a, b] {
        let e = g.world.entity(id).unwrap();
        assert!(
            e.collision_mask()
                .intersects_rectangle_bbox(Vec2::new(152., 152.), Vec2::new(168., 168.))
        );
    }
    assert!(
        !g.world
            .entity(c)
            .unwrap()
            .collision_mask()
            .intersects_rectangle_bbox(Vec2::new(152., 152.), Vec2::new(168., 168.))
    );
    (g, a, b, c)
}
fn hurt(g: &mut Game, obj: &str, chosen: u64, marker: &str) {
    let target = g.world.entity(chosen).unwrap().clone();
    let ledger = (
        g.dispatcher.registration_order.clone(),
        g.dispatcher.active_order.clone(),
        g.dispatcher.inactive_order.clone(),
        g.dispatcher.resource_order.clone(),
    );
    let mut rng = g.rng.clone();
    let speed = if obj == "oExplosion" {
        4. + ((rng.next_u64() * 3) >> 32) as f64
    } else {
        2.
    };
    if obj == "oExplosion" {
        super::player_explosion_hurt(g)
    } else {
        crate::enemies::player_hurt(g)
    }
    let sign = if target.pos.x < 160. { 1. } else { -1. };
    assert_eq!(
        g.player.vel.x,
        sign * speed,
        "{marker} {obj}: chosen={chosen} target={:?}",
        target.pos
    );
    assert_eq!(g.player.vel.y, if obj == "oExplosion" { -6. } else { -4. });
    assert_eq!(
        g.player.health,
        30.0 - match obj {
            "oExplosion" => 10.0,
            "oLaser" => 3.0,
            _ => 1.0,
        }
    );
    assert_eq!(
        g.player.stun,
        match obj {
            "oExplosion" => 100,
            "oLaser" => 20,
            _ => 40,
        }
    );
    assert_eq!(
        g.rng.clone().next_u64(),
        rng.next_u64(),
        "selector never consumes RNG"
    );
    let mut expected_ledger = ledger;
    if obj == "oLaser" {
        // Classic's laser hit synchronously instance_creates oLaserExplode.
        // Its Create syncs pending raw births in creation-serial order too.
        // Those legitimate births join their families after the nearest query;
        // existing registrations must retain their order and membership.
        let registered: std::collections::BTreeSet<_> = expected_ledger
            .0
            .iter()
            .chain(&expected_ledger.1)
            .chain(&expected_ledger.2)
            .copied()
            .collect();
        let mut newborns: Vec<_> = g
            .world
            .entities
            .iter()
            .filter(|entity| !registered.contains(&Instance::Entity(entity.id)))
            .map(|entity| (entity.creation_serial, entity.id, entity.object.as_str()))
            .collect();
        newborns.sort_by_key(|(serial, _, _)| *serial);
        assert!(
            newborns
                .iter()
                .any(|(_, _, object)| *object == "oLaserExplode"),
            "laser hit creates its source explosion effect"
        );
        for (_, id, object) in newborns {
            let instance = Instance::Entity(id);
            expected_ledger.0.push(instance);
            expected_ledger.1.push(instance);
            expected_ledger
                .3
                .entry(crate::catalog::object(object).unwrap().id)
                .or_default()
                .push(instance);
        }
    }
    assert_eq!(
        (
            g.dispatcher.registration_order.clone(),
            g.dispatcher.active_order.clone(),
            g.dispatcher.inactive_order.clone(),
            g.dispatcher.resource_order.clone(),
        ),
        expected_ledger,
        "QUERY_DOES_NOT_REGISTER_OR_REPARTITION"
    );
    assert_eq!(g.level_tick, 0);
    assert!(!g.world.entities.iter().any(|e| e.object == "oBlood"));
    if obj == "oLaser" {
        assert!(g.world.entities.iter().any(|e| e.id == chosen && !e.alive));
        let effects: Vec<_> = g
            .world
            .entities
            .iter()
            .filter(|e| e.alive && e.object == "oLaserExplode")
            .collect();
        assert_eq!(effects.len(), 1);
        assert_eq!(effects[0].pos, target.pos);
    } else {
        assert!(g.world.entity(chosen).unwrap().alive);
    }
}
fn tie(obj: &str) {
    for rules in RULES {
        let (mut g, a, b, _) = reordered(rules, obj, 152., 168.);
        assert_eq!(g.world.entity(a).unwrap().pos.distance(g.player.pos), 8.);
        assert_eq!(g.world.entity(b).unwrap().pos.distance(g.player.pos), 8.);
        hurt(&mut g, obj, a, "FIRST_ACTIVE_FAMILY_NEAREST_TIE");
    }
}
#[test]
fn explosion_reactivation_tie_uses_first_current_family() {
    tie("oExplosion")
}
#[test]
fn laser_reactivation_tie_uses_first_current_family() {
    tie("oLaser")
}
#[test]
fn wave_reactivation_tie_uses_first_current_family() {
    tie("oPsychicWave")
}
#[test]
fn unequal_distance_preserves_closest_over_family_rank() {
    for rules in RULES {
        for obj in ["oExplosion", "oLaser", "oPsychicWave"] {
            let (mut g, a, b, _) = reordered(rules, obj, 152., 164.);
            assert!(
                g.world.entity(b).unwrap().pos.distance(g.player.pos)
                    < g.world.entity(a).unwrap().pos.distance(g.player.pos)
            );
            hurt(&mut g, obj, b, "UNIQUE_DISTANCE_BEATS_RANK");
        }
    }
}
#[test]
fn retired_and_inactive_do_not_win_nearest() {
    for rules in RULES {
        for obj in ["oExplosion", "oLaser", "oPsychicWave"] {
            for retired in [false, true] {
                let (mut g, a, b, _) = reordered(rules, obj, 152., 168.);
                if retired {
                    g.world.entity_mut(a).unwrap().alive = false;
                } else {
                    g.world.entity_mut(a).unwrap().engine_active = false;
                    dispatcher::activate_region(&mut g, Rect::new(296., 152., 32., 32.));
                    assert!(
                        g.dispatcher
                            .hidden_entities()
                            .iter()
                            .any(|e| e.id == a && !e.engine_active)
                    );
                    assert!(!ids(&g, obj).contains(&a));
                }
                hurt(&mut g, obj, b, "INELIGIBLE_FIRST_CANDIDATE_SKIPPED");
            }
        }
    }
}
#[test]
fn source_registered_newborn_is_visible_without_query_registration() {
    for rules in RULES {
        for obj in ["oExplosion", "oLaser", "oPsychicWave"] {
            let (mut g, _, _, _) = reordered(rules, obj, 152., 168.);
            let fresh = g.spawn(obj, Vec2::new(161., 160.));
            assert_eq!(ids(&g, obj).last(), Some(&fresh));
            hurt(&mut g, obj, fresh, "SOURCE_NEWBORN_VISIBLE");
        }
    }
}
#[test]
fn raw_deferred_newborn_keeps_unique_distance_visibility() {
    for rules in RULES {
        for obj in ["oExplosion", "oLaser", "oPsychicWave"] {
            let (mut g, _, _, _) = reordered(rules, obj, 152., 168.);
            let fresh = g.world.spawn(obj, Vec2::new(161., 160.));
            assert!(!ids(&g, obj).contains(&fresh));
            hurt(&mut g, obj, fresh, "DEFERRED_NEWBORN_VISIBLE");
        }
    }
}
#[test]
fn pending_birth_tie_uses_creation_order_not_configured_storage() {
    for rules in RULES {
        for obj in ["oExplosion", "oLaser", "oPsychicWave"] {
            let mut g = scene(rules);
            let c = g.spawn(obj, Vec2::new(304., 160.));
            let a = g.world.spawn(obj, Vec2::new(152., 160.));
            let b = g.world.spawn(obj, Vec2::new(168., 160.));
            assert!(
                g.world.entity(a).unwrap().creation_serial
                    < g.world.entity(b).unwrap().creation_serial
            );
            // Explicit configured representation permutation, not ordinary input.
            g.world.entities.swap(1, 2);
            assert_eq!(ids(&g, obj), vec![c]);
            assert_eq!(storage(&g, obj), vec![c, b, a]);
            hurt(&mut g, obj, a, "FIRST_PENDING_BIRTH_NEAREST_TIE");
        }
    }
}
