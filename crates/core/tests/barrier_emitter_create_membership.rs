//! Original Emitter Create uses instance_exists(oBarrier), which sees active
//! instances. Inactive retained initial state is configured explicitly here;
//! this does not claim a recorded ordinary gameplay deactivation route.
use spelunky_core::{Area, Game, InputFrame, PlayerState, RngAlgorithm, RulesPreset, Vec2, World};

fn configurations(mut run: impl FnMut(RulesPreset, RngAlgorithm)) {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for algorithm in [RngAlgorithm::LegacySplitMix64V1, RngAlgorithm::Gm8Lcg32V1] {
            run(rules, algorithm);
        }
    }
}

#[test]
fn inactive_retained_barrier_does_not_suppress_real_emitter_create_child() {
    configurations(|rules, algorithm| {
        let mut g = Game::shell_with_algorithm(42, rules, algorithm).unwrap();
        let old = g.spawn("oBarrier", Vec2::new(512.0, 96.0));
        // Initial source inactive-state boundary before a synchronous Create.
        g.world.entity_mut(old).unwrap().engine_active = false;
        let expected_rng = g.rng.clone().next_u64();
        let emitter = g.spawn("oBarrierEmitter", Vec2::new(160.0, 80.0));
        let children: Vec<_> = g
            .world
            .entities
            .iter()
            .filter(|e| e.alive && e.engine_active && e.object == "oBarrier")
            .collect();
        assert_eq!(children.len(), 1);
        let child = children[0];
        assert_ne!(child.id, old);
        assert_eq!(child.pos, Vec2::new(160.0, 96.0));
        assert_eq!(child.previous_pos, child.pos);
        assert_eq!(child.vel, Vec2::default());
        assert_eq!(child.image_speed(), 1.0);
        assert_eq!(child.animation, 0.0);
        let parent = g.world.entity(emitter).unwrap();
        assert_eq!(parent.image_speed(), 0.5);
        assert!(parent.creation_serial < child.creation_serial);
        let old = g.world.entity(old).unwrap();
        assert!(!old.engine_active);
        assert_eq!(old.pos, Vec2::new(512.0, 96.0));
        assert!(old.creation_serial < parent.creation_serial);
        assert_eq!(g.rng.clone().next_u64(), expected_rng);
    });
}

#[test]
fn active_global_barrier_suppresses_child_but_retired_identity_does_not() {
    configurations(|rules, algorithm| {
        for alive in [true, false] {
            let mut g = Game::shell_with_algorithm(42, rules, algorithm).unwrap();
            // Existence is global over active instances, not an emitter-local
            // rectangle or view gate: this old Barrier is far from the new birth.
            let old = g.spawn("oBarrier", Vec2::new(512.0, 96.0));
            g.world
                .entities
                .iter_mut()
                .find(|e| e.id == old)
                .unwrap()
                .alive = alive;
            let expected_rng = g.rng.clone().next_u64();
            let emitter = g.spawn("oBarrierEmitter", Vec2::new(160.0, 80.0));
            let barriers: Vec<_> = g
                .world
                .entities
                .iter()
                .filter(|e| e.alive && e.engine_active && e.object == "oBarrier")
                .collect();
            assert_eq!(barriers.len(), 1);
            if alive {
                assert_eq!(barriers[0].id, old);
                assert_eq!(barriers[0].pos, Vec2::new(512.0, 96.0));
                assert!(
                    barriers[0].creation_serial < g.world.entity(emitter).unwrap().creation_serial
                );
            } else {
                assert_ne!(barriers[0].id, old);
                assert_eq!(barriers[0].pos, Vec2::new(160.0, 96.0));
                assert!(
                    g.world.entity(emitter).unwrap().creation_serial < barriers[0].creation_serial
                );
            }
            assert_eq!(g.rng.clone().next_u64(), expected_rng);
        }
    });
}

#[test]
fn new_child_from_inactive_gate_has_real_supported_step_and_builtin_animation() {
    configurations(|rules, algorithm| {
        let mut g = Game::shell_with_algorithm(42, rules, algorithm).unwrap();
        g.world = World::new(40, 30, Area::Ice, 9);
        for x in 0..40 {
            g.world.set_tile(x, 10, "oBlock");
        }
        g.world.set_tile(10, 4, "oBlock"); // initial source support above Emitter
        g.player.pos = Vec2::new(40.0, 152.0);
        g.player.previous_pos = g.player.pos;
        g.player.grounded = true;
        g.player.state = PlayerState::Standing;
        let old = g.spawn("oBarrier", Vec2::new(512.0, 96.0));
        g.world.entity_mut(old).unwrap().engine_active = false;
        let emitter = g.spawn("oBarrierEmitter", Vec2::new(160.0, 80.0));
        let child = g
            .world
            .entities
            .iter()
            .find(|e| e.alive && e.engine_active && e.object == "oBarrier")
            .expect("Source inactive gate births an active child")
            .id;
        let expected_rng = g.rng.clone().next_u64();
        for (tick, frame) in [1.0, 2.0, 0.0, 1.0].into_iter().enumerate() {
            g.step(&InputFrame::default());
            let e = g.world.entity(child).unwrap();
            assert_eq!(e.pos, Vec2::new(160.0, 96.0));
            assert_eq!(e.image_speed(), 1.0);
            assert_eq!(e.animation, frame);
            assert_eq!(
                g.world.entity(emitter).unwrap().animation,
                (tick + 1) as f64 * 0.5
            );
            assert_eq!(g.player.health, 4);
            assert_eq!(g.rng.clone().next_u64(), expected_rng);
            let retained = g
                .world
                .entities
                .iter()
                .chain(g.dispatcher.hidden_entities())
                .find(|e| e.id == old)
                .unwrap();
            assert!(retained.alive);
            assert!(!retained.engine_active);
            assert_eq!(retained.pos, Vec2::new(512.0, 96.0));
        }
    });
}
