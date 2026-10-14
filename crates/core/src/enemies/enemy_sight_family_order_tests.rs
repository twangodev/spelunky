//! Source with-Caveman THEN with-Hawkman order, independent of World storage.
use super::*;
use crate::{Area, RulesPreset};
fn scene(rules: RulesPreset, legacy: bool) -> Game {
    let mut g = if legacy {
        Game::legacy_shell(123, rules)
    } else {
        Game::shell(123, rules)
    };
    g.world = World::new(40, 20, Area::Mines, 1);
    g.player.pos = Vec2::new(144.0, 72.0);
    g
}
fn invoke(g: &mut Game) {
    let mut sight = Entity::new(900_000, "oEnemySight", Vec2::new(128.0, 64.0));
    crate::enemies::init_entity(&mut sight);
    assert!(collision(
        g,
        &mut sight,
        crate::dispatcher::Instance::Player
    ));
}
fn alert_positions(g: &Game, first: usize) -> Vec<Vec2> {
    g.events[first..]
        .iter()
        .filter_map(|event| match event {
            crate::GameEvent::Sound { name, pos } if name == "sndAlert" => Some(*pos),
            _ => None,
        })
        .collect()
}
fn ids(g: &Game, object: &str) -> Vec<u64> {
    crate::dispatcher::descendant_instances(g, object)
        .into_iter()
        .filter_map(|i| match i {
            crate::dispatcher::Instance::Entity(id) => Some(id),
            _ => None,
        })
        .collect()
}
#[test]
fn sight_alerts_each_family_in_current_order_after_reactivation_not_storage_or_birth_order() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for legacy in [false, true] {
            let mut g = scene(rules, legacy);
            let hawk_a = g.spawn("oHawkman", Vec2::new(48.0, 64.0));
            let cav_a = g.spawn("oCaveman", Vec2::new(64.0, 64.0));
            let cav_b = g.spawn("oCaveman", Vec2::new(80.0, 64.0));
            let hawk_b = g.spawn("oHawkman", Vec2::new(112.0, 64.0));
            let cav_c = g.spawn("oCaveman", Vec2::new(96.0, 64.0));
            // These are initial source membership operations, not an injected
            // post-input actor-state witness. Reactivation changes family order.
            g.world.entity_mut(cav_a).unwrap().engine_active = false;
            crate::dispatcher::activate_region(&mut g, Rect::new(104.0, 56.0, 8.0, 32.0));
            g.world.entity_mut(cav_b).unwrap().engine_active = false;
            crate::dispatcher::activate_region(&mut g, Rect::new(104.0, 56.0, 8.0, 32.0));
            crate::dispatcher::activate_object(&mut g, "oCaveman");
            assert_eq!(ids(&g, "oCaveman"), vec![cav_c, cav_a, cav_b]);
            assert_eq!(ids(&g, "oHawkman"), vec![hawk_a, hawk_b]);
            // Deliberately keep storage different from both source arrays.
            g.world.entities.reverse();
            let registration = g.dispatcher.registration_order.clone();
            let first = g.events.len();
            let rng = g.rng.state_bits();
            invoke(&mut g);
            let expected = vec![
                Vec2::new(96.0, 64.0),
                Vec2::new(64.0, 64.0),
                Vec2::new(80.0, 64.0),
                Vec2::new(48.0, 64.0),
                Vec2::new(112.0, 64.0),
            ];
            assert_eq!(alert_positions(&g, first), expected);
            for id in [cav_a, cav_b, cav_c, hawk_a, hawk_b] {
                assert_eq!(g.world.entity(id).unwrap().state, 2);
            }
            assert_eq!(
                g.dispatcher.registration_order, registration,
                "query must not synchronize/rebuild source arrays"
            );
            assert_eq!(g.rng.state_bits(), rng);
            // Source status2 remains<98: another actual selected Collision
            // repeats the two passes, rather than adding a once-only latch.
            let first = g.events.len();
            invoke(&mut g);
            assert_eq!(alert_positions(&g, first), expected);
            assert_eq!(g.rng.state_bits(), rng);
        }
    }
}
#[test]
fn sight_pending_births_append_by_creation_serial_and_skip_ineligible_members() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for legacy in [false, true] {
            let mut g = scene(rules, legacy);
            let hawk = g.spawn("oHawkman", Vec2::new(48.0, 64.0));
            let cav = g.spawn("oCaveman", Vec2::new(64.0, 64.0));
            let dead = g.spawn("oCaveman", Vec2::new(80.0, 64.0));
            g.world.entity_mut(dead).unwrap().state = 98;
            let inactive = g.spawn("oHawkman", Vec2::new(96.0, 64.0));
            g.world.entity_mut(inactive).unwrap().engine_active = false;
            let retired = g.spawn("oCaveman", Vec2::new(104.0, 64.0));
            g.world.entity_mut(retired).unwrap().alive = false;
            let pending_cav_a = g.world.spawn("oCaveman", Vec2::new(112.0, 64.0));
            let pending_hawk = g.world.spawn("oHawkman", Vec2::new(120.0, 64.0));
            let pending_cav_b = g.world.spawn("oCaveman", Vec2::new(128.0, 64.0));
            assert!(!ids(&g, "oCaveman").contains(&pending_cav_a));
            g.world.entities.reverse();
            let registration = g.dispatcher.registration_order.clone();
            let first = g.events.len();
            let rng = g.rng.state_bits();
            invoke(&mut g);
            assert_eq!(
                alert_positions(&g, first),
                vec![
                    Vec2::new(64.0, 64.0),
                    Vec2::new(112.0, 64.0),
                    Vec2::new(128.0, 64.0),
                    Vec2::new(48.0, 64.0),
                    Vec2::new(120.0, 64.0)
                ]
            );
            for id in [cav, pending_cav_a, pending_cav_b, hawk, pending_hawk] {
                assert_eq!(g.world.entity(id).unwrap().state, 2);
            }
            assert_eq!(g.world.entity(dead).unwrap().state, 98);
            assert_eq!(g.world.entity(inactive).unwrap().state, 0);
            assert!(
                !g.world
                    .entities
                    .iter()
                    .find(|e| e.id == retired)
                    .unwrap()
                    .alive
            );
            assert_eq!(g.dispatcher.registration_order, registration);
            assert_eq!(g.rng.state_bits(), rng);
        }
    }
}
