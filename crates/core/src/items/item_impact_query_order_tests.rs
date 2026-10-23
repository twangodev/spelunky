//! Configured inherited Item query contracts; no stock campaign claim.
//! Nested-Create probe changes family membership to test callback visibility,
//! not to claim that original Leaf Create itself spawns Damsels.
use super::install;
use crate::dispatcher::{self, Instance};
use crate::model::*;
use std::{cell::Cell, rc::Rc};

fn scene(rules: RulesPreset, algorithm: RngAlgorithm) -> Game {
    let mut g = Game::shell_with_algorithm(42, rules, algorithm).unwrap();
    g.world = World::new(40, 24, Area::Mines, 1);
    g.player.pos = Vec2::new(24.0, 200.0);
    g.player.previous_pos = g.player.pos;
    g
}
fn shot(g: &mut Game) -> u64 {
    let id = g.spawn("oRock", Vec2::new(168.0, 168.0));
    g.world.entity_mut(id).unwrap().vel = Vec2::new(3.0, 0.0);
    id
}
fn overlap(g: &Game, id: u64) -> bool {
    g.world
        .entity(id)
        .unwrap()
        .collision_mask()
        .intersects_rectangle_bbox(Vec2::new(166.0, 166.0), Vec2::new(170.0, 170.0))
}
fn consume_blood(rng: &mut Rng) -> (Vec2, f64) {
    // Literal Detritus child Create: two horizontal units, vertical unit,
    // then rand(1,6). No production Blood helper is used in this oracle.
    let velocity = Vec2::new(rng.unit() * 4.0 - rng.unit() * 4.0, -1.0 - rng.unit() * 2.0);
    let gravity = f64::from(1 + rng.range(6)) * 0.1;
    (velocity, gravity)
}
fn births(g: &Game) -> Vec<&Entity> {
    let mut blood: Vec<_> = g
        .world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oBlood")
        .collect();
    blood.sort_by_key(|e| e.creation_serial);
    blood
}

#[test]
fn item_enemy_queries_ignore_inactive_storage_and_keep_active_nearest_tie_order() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for algorithm in [RngAlgorithm::LegacySplitMix64V1, RngAlgorithm::Gm8Lcg32V1] {
            // Initial inactive storage must not qualify the Enemy rectangle.
            let mut g = scene(rules, algorithm);
            let snake = g.spawn("oSnake", Vec2::new(160.0, 160.0));
            assert!(overlap(&g, snake));
            g.world.entity_mut(snake).unwrap().engine_active = false;
            let item = shot(&mut g);
            let rng = g.rng.state_bits();
            crate::enemies::inherited_item_step_impacts(&mut g, item);
            assert_eq!(
                g.world.entity(snake).unwrap().health,
                1,
                "INACTIVE_ENEMY_RECTANGLE"
            );
            assert!(births(&g).is_empty());
            assert_eq!(g.rng.state_bits(), rng);

            for blocked in [false, true] {
                let mut g = scene(rules, algorithm);
                let a = g.spawn("oSnake", Vec2::new(160.0, 160.0));
                let b = g.spawn("oSnake", Vec2::new(176.0, 160.0));
                let item = shot(&mut g);
                assert!(overlap(&g, a));
                assert!(
                    !overlap(&g, b),
                    "rectangle only gates the separate whole-family nearest query"
                );
                g.world.entity_mut(a).unwrap().engine_active = false;
                dispatcher::activate_region(&mut g, Rect::new(0.0, 0.0, 1.0, 1.0));
                dispatcher::activate_object(&mut g, "oSnake");
                let order: Vec<_> = dispatcher::descendant_instances(&g, "oEnemy")
                    .into_iter()
                    .filter_map(|i| match i {
                        Instance::Entity(id) => Some(id),
                        _ => None,
                    })
                    .collect();
                assert_eq!(order, vec![b, a], "reactivation appends A after B");
                if blocked {
                    g.world
                        .entity_mut(b)
                        .unwrap()
                        .data
                        .insert("invincible".into(), 1.0);
                }
                g.world.entities.reverse();
                assert!(
                    g.world.entities.iter().position(|e| e.id == a)
                        < g.world.entities.iter().position(|e| e.id == b)
                );
                let mut rng = g.rng.clone();
                let expected = (!blocked).then(|| consume_blood(&mut rng));
                crate::enemies::inherited_item_step_impacts(&mut g, item);
                assert_eq!(g.world.entity(a).unwrap().health, 1);
                assert_eq!(
                    g.world.entity(b).unwrap().health,
                    if blocked { 1 } else { 0 },
                    "ACTIVE_ENEMY_NEAREST_TIE"
                );
                let blood = births(&g);
                assert_eq!(blood.len(), usize::from(!blocked));
                if let Some((vel, gravity)) = expected {
                    assert_eq!(blood[0].pos, Vec2::new(184.0, 168.0));
                    assert_eq!(blood[0].vel, vel);
                    assert_eq!(blood[0].value("gravity"), gravity);
                }
                assert_eq!(g.rng.state_bits(), rng.state_bits());
            }
        }
    }
}

#[test]
fn item_enemy_queries_skip_retired_nearest_instances() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for algorithm in [RngAlgorithm::LegacySplitMix64V1, RngAlgorithm::Gm8Lcg32V1] {
            let mut g = scene(rules, algorithm);
            let retired = g.spawn("oSnake", Vec2::new(160.0, 160.0));
            let live = g.spawn("oSnake", Vec2::new(160.0, 160.0));
            g.world.entity_mut(retired).unwrap().alive = false;
            let item = shot(&mut g);
            let live_health = g.world.entity(live).unwrap().health;

            crate::enemies::inherited_item_step_impacts(&mut g, item);

            assert!(g.world.entity(retired).is_none());
            assert_eq!(g.world.entity(live).unwrap().health, live_health - 1);
        }
    }
}

#[test]
fn item_damsel_rectangle_selects_first_active_overlap_before_testing_eligibility() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for algorithm in [RngAlgorithm::LegacySplitMix64V1, RngAlgorithm::Gm8Lcg32V1] {
            for blocked in [false, true] {
                let mut g = scene(rules, algorithm);
                let a = g.spawn("oDamsel", Vec2::new(166.0, 168.0));
                let b = g.spawn("oDamsel", Vec2::new(166.0, 168.0));
                let item = shot(&mut g);
                assert!(overlap(&g, a) && overlap(&g, b));
                assert_eq!(
                    g.world.entity(b).unwrap().health,
                    4,
                    "source-default Damsel HP"
                );
                g.world.entity_mut(a).unwrap().engine_active = false;
                dispatcher::activate_region(&mut g, Rect::new(0.0, 0.0, 1.0, 1.0));
                dispatcher::activate_object(&mut g, "oDamsel");
                let ids: Vec<_> = dispatcher::descendant_instances(&g, "oDamsel")
                    .into_iter()
                    .filter_map(|i| match i {
                        Instance::Entity(id) => Some(id),
                        _ => None,
                    })
                    .collect();
                assert_eq!(ids, vec![b, a]);
                if blocked {
                    g.world
                        .entity_mut(b)
                        .unwrap()
                        .data
                        .insert("invincible".into(), 1.0);
                }
                g.world.entities.reverse();
                let mut rng = g.rng.clone();
                let expected = (!blocked).then(|| consume_blood(&mut rng));
                crate::enemies::inherited_item_step_impacts(&mut g, item);
                assert_eq!(g.world.entity(a).unwrap().health, 4);
                assert_eq!(
                    g.world.entity(b).unwrap().health,
                    if blocked { 4 } else { 3 },
                    "DAMSEL_FIRST_ACTIVE_RECTANGLE"
                );
                assert_eq!(births(&g).len(), usize::from(!blocked));
                if let Some((vel, gravity)) = expected {
                    let blood = births(&g)[0];
                    assert_eq!(blood.pos, Vec2::new(166.0, 168.0));
                    assert_eq!(blood.vel, vel);
                    assert_eq!(blood.value("gravity"), gravity);
                    assert_eq!(g.world.entity(b).unwrap().state, 2);
                    assert_eq!(g.world.entity(b).unwrap().timer, 120);
                }
                assert_eq!(g.rng.state_bits(), rng.state_bits());
            }
            let mut g = scene(rules, algorithm);
            let damsel = g.spawn("oDamsel", Vec2::new(166.0, 168.0));
            g.world.entity_mut(damsel).unwrap().engine_active = false;
            let item = shot(&mut g);
            let rng = g.rng.state_bits();
            crate::enemies::inherited_item_step_impacts(&mut g, item);
            assert_eq!(
                g.world.entity(damsel).unwrap().health,
                4,
                "INACTIVE_DAMSEL_RECTANGLE"
            );
            assert!(births(&g).is_empty());
            assert_eq!(g.rng.state_bits(), rng);
        }
    }
}

#[test]
fn item_damsel_query_observes_enemy_nested_create_birth_and_per_birth_blood_cap() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for algorithm in [RngAlgorithm::LegacySplitMix64V1, RngAlgorithm::Gm8Lcg32V1] {
            for cap in [0, 15, 16] {
                let mut g = scene(rules, algorithm);
                let trap = g.spawn("oManTrap", Vec2::new(160.0, 160.0));
                let hp = g.world.entity(trap).unwrap().health;
                let item = shot(&mut g);
                for _ in 0..cap {
                    g.spawn("oBone", Vec2::new(240.0, 80.0));
                }
                assert!(overlap(&g, trap));
                assert_ne!(g.world.entity(trap).unwrap().state, 98);
                assert!(dispatcher::descendant_instances(&g, "oDamsel").is_empty());
                let newborn = Rc::new(Cell::new(None));
                let observed = newborn.clone();
                let _probe = install(move |game, child| {
                    if game.world.entity(child).unwrap().object == "oLeaf"
                        && observed.get().is_none()
                    {
                        // Deliberately configured nested Create callback. The
                        // literal original Leaf Create does not spawn Damsels.
                        let id = game.spawn("oDamsel", Vec2::new(166.0, 168.0));
                        assert_eq!(game.world.entity(id).unwrap().health, 4);
                        observed.set(Some(id));
                    }
                });
                let mut rng = g.rng.clone();
                let leaf_pos = Vec2::new(
                    160.0 + f64::from(rng.range(17)),
                    152.0 + f64::from(rng.range(17)),
                );
                let count = usize::from(cap < 16);
                let expected: Vec<_> = (0..count).map(|_| consume_blood(&mut rng)).collect();
                crate::enemies::inherited_item_step_impacts(&mut g, item);
                assert_eq!(g.world.entity(trap).unwrap().health, hp - 1);
                assert_eq!(g.world.entity(trap).unwrap().state, 98);
                let leaf: Vec<_> = g
                    .world
                    .entities
                    .iter()
                    .filter(|e| e.alive && e.object == "oLeaf")
                    .collect();
                assert_eq!(leaf.len(), 1);
                assert_eq!(leaf[0].pos, leaf_pos);
                let blood = births(&g);
                assert_eq!(blood.len(), count);
                for (e, (vel, gravity)) in blood.iter().zip(expected) {
                    assert_eq!(e.vel, vel);
                    assert_eq!(e.value("gravity"), gravity);
                }
                let target = newborn
                    .get()
                    .expect("Enemy Leaf Create synchronously ran the controlled probe");
                assert!(
                    dispatcher::descendant_instances(&g, "oDamsel")
                        .contains(&Instance::Entity(target))
                );
                assert_eq!(
                    g.world.entity(target).unwrap().health,
                    3,
                    "DAMSEL_QUERY_AFTER_ENEMY_CALLBACK"
                );
                assert_eq!(g.world.entity(target).unwrap().state, 2);
                assert_eq!(g.world.entity(target).unwrap().value("hit_cooldown"), 10.0);
                if cap < 16 {
                    assert_eq!(blood[0].pos, Vec2::new(166.0, 168.0));
                }
                assert_eq!(g.rng.state_bits(), rng.state_bits());
                assert_eq!(g.player.health, 4.0);
            }
        }
    }
}

#[test]
fn finite_native_nearest_sentinel_fault_stops_the_later_damsel_statement() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for algorithm in [RngAlgorithm::LegacySplitMix64V1, RngAlgorithm::Gm8Lcg32V1] {
            let mut g = scene(rules, algorithm);
            let snake = g.spawn("oSnake", Vec2::new(100168.0, 168.0));
            // Configured huge signed source scale makes the rectangle gate
            // true while the raw-origin squared distance reaches native1e10.
            g.world
                .entity_mut(snake)
                .unwrap()
                .data
                .insert("image_xscale".into(), -10000.0);
            // Snake's source bbox starts four pixels below its raw origin.
            // Initial Y scale1/8 keeps raw origins unchanged but brings its
            // cached integer bbox to y168..169 (168.5 rounds nearest-even).
            g.world
                .entity_mut(snake)
                .unwrap()
                .data
                .insert("image_yscale".into(), 0.125);
            assert_eq!(
                g.world
                    .entity(snake)
                    .unwrap()
                    .collision_mask()
                    .point_bounds(),
                Rect::new(-49833.0, 168.0, 140002.0, 2.0)
            );
            let damsel = g.spawn("oDamsel", Vec2::new(166.0, 168.0));
            let item = shot(&mut g);
            assert!(overlap(&g, snake) && overlap(&g, damsel));
            let dx = g.world.entity(snake).unwrap().pos.x - g.world.entity(item).unwrap().pos.x;
            assert_eq!(dx * dx, 10_000_000_000.0);
            let dy = g.world.entity(snake).unwrap().pos.y - g.world.entity(item).unwrap().pos.y;
            assert_eq!(dy, 0.0);
            assert_eq!(dx * dx + dy * dy, 10_000_000_000.0);
            let rng = g.rng.state_bits();
            crate::enemies::inherited_item_step_impacts(&mut g, item);
            assert_eq!(g.world.entity(snake).unwrap().health, 1);
            assert_eq!(
                g.world.entity(damsel).unwrap().health,
                4,
                "error precedes the later Damsel query"
            );
            assert_eq!(
                g.data.get("source_runtime_errors").copied().unwrap_or(0.0),
                1.0
            );
            assert!(g.events.iter().any(|e|matches!(e,GameEvent::Message(s)if s.contains("oItem Step: obj.invincible")&&s.contains("code block aborted"))));
            assert!(births(&g).is_empty());
            assert_eq!(g.rng.state_bits(), rng);
        }
    }
}
