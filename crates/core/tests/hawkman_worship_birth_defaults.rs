//! rOlmec instance 134465 at (280,544) has empty creation code. Its object
//! has no parent/events: builtin animation is distinct from a real Hawkman.
use spelunky_core::{
    Entity, Game, InputFrame, Rect, Rng, RngAlgorithm, RulesPreset, Vec2, catalog, dispatcher,
    enemies, generation,
};
use std::collections::BTreeSet;

fn configurations(mut run: impl FnMut(RulesPreset, RngAlgorithm)) {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for algorithm in [RngAlgorithm::LegacySplitMix64V1, RngAlgorithm::Gm8Lcg32V1] {
            run(rules, algorithm);
        }
    }
}

fn assert_statue(e: &Entity, origin: Vec2) {
    let definition = catalog::object("oHawkmanWorship").unwrap();
    assert_eq!(definition.parent, "");
    assert_eq!(definition.sprite, "sHawkLeft");
    assert_eq!(definition.depth, 0);
    assert!(definition.visible);
    assert!(!definition.solid);
    assert_eq!(e.object, "oHawkmanWorship");
    assert_eq!(e.sprite, "sHawkLeft");
    assert_eq!(e.pos, origin);
    assert_eq!(e.vel, Vec2::default());
    assert!(!e.solid);
    assert!(e.engine_active);
    assert_eq!(e.image_speed(), 1.0); // runner builtin, no own Create assignment
    assert!(!e.data.contains_key("animation_speed"));
    assert!(!e.data.contains_key("depth")); // Draw ordering falls back to object depth 0
    for unsupported in [
        "enemy_initialized",
        "enemy_context_created",
        "enemy",
        "heavy",
        "counts_as_kill",
        "stun_time",
        "contact_damage",
        "collision_left",
        "collision_top",
        "collision_right",
        "collision_bottom",
    ] {
        assert!(
            !e.data.contains_key(unsupported),
            "invented enemy birth field {unsupported}"
        );
    }
    assert_eq!(e.bounds(), Rect::new(origin.x, origin.y, 16.0, 16.0));
    assert_eq!(e.sprite_bounds(), Rect::new(origin.x, origin.y, 16.0, 16.0));
    // Source has no hp variable here. Entity's mandatory Rust health field is
    // only a model placeholder; check it remains untouched, not source hp=1.
    assert_eq!(e.health, Entity::new(0, "oHawkmanWorship", origin).health);
}

#[test]
fn parentless_runtime_statue_retains_builtin_defaults_and_consumes_no_create_rng() {
    configurations(|rules, algorithm| {
        let mut g = Game::shell_with_algorithm(42, rules, algorithm).unwrap();
        let expected_rng = g.rng.clone().next_u64();
        let origin = Vec2::new(80.0, 80.0);
        let id = g.spawn("oHawkmanWorship", origin);
        let e = g.world.entity(id).unwrap();
        assert_statue(e, origin);
        assert_eq!(e.animation, 0.0);
        assert_eq!(g.rng.clone().next_u64(), expected_rng);
        assert_eq!(
            dispatcher::descendant_instances(&g, "oHawkmanWorship"),
            vec![dispatcher::Instance::Entity(id)]
        );
        assert!(dispatcher::descendant_instances(&g, "oEnemy").is_empty());
        assert!(!enemies::is_enemy("oHawkmanWorship"));
    });
}

#[test]
fn ordinary_steps_preserve_static_statue_and_builtin_single_frame_animation() {
    configurations(|rules, algorithm| {
        let mut g = Game::shell_with_algorithm(42, rules, algorithm).unwrap();
        g.player.pos = Vec2::new(200.0, 120.0);
        g.player.data.insert("active".into(), 0.0);
        g.camera = Vec2::new(160.0, 120.0);
        let origin = Vec2::new(80.0, 80.0);
        let id = g.spawn("oHawkmanWorship", origin);
        assert_eq!(catalog::sprite_frame_count("sHawkLeft"), 1);
        let expected_rng = g.rng.clone().next_u64();
        for tick in 1..=4 {
            g.step(&InputFrame::default());
            let e = g.world.entity(id).unwrap();
            assert_statue(e, origin);
            assert_eq!(e.previous_pos, origin);
            assert_eq!(e.age, tick);
            assert_eq!(
                e.animation, 0.0,
                "builtin speed 1 wraps the one-frame sprite each Step"
            );
            assert_eq!(g.rng.clone().next_u64(), expected_rng);
            assert_eq!(
                dispatcher::descendant_instances(&g, "oHawkmanWorship"),
                vec![dispatcher::Instance::Entity(id)]
            );
            assert!(dispatcher::descendant_instances(&g, "oEnemy").is_empty());
        }
    });
}

#[test]
fn actual_authored_olmec_room_statue_has_empty_create_defaults_at_source_origin() {
    let mut rng = Rng::new(42);
    let world = generation::generate(&mut rng, 16, &BTreeSet::new());
    let statues: Vec<_> = world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oHawkmanWorship")
        .collect();
    assert_eq!(statues.len(), 1);
    let statue = statues[0];
    assert_statue(statue, Vec2::new(280.0, 544.0));
    assert_eq!(statue.animation, 0.0);
    // The authored birth follows the three Caveman statues in rOlmec.
    for origin in [
        Vec2::new(264.0, 544.0),
        Vec2::new(248.0, 544.0),
        Vec2::new(232.0, 544.0),
    ] {
        let prior = world
            .entities
            .iter()
            .find(|e| e.alive && e.object == "oCavemanWorship" && e.pos == origin)
            .expect("earlier authored Caveman statue");
        assert!(prior.creation_serial < statue.creation_serial);
    }
}

#[test]
fn proper_hawkman_still_inherits_enemy_create_and_owns_its_source_defaults() {
    configurations(|rules, algorithm| {
        let mut g = Game::shell_with_algorithm(42, rules, algorithm).unwrap();
        let origin = Vec2::new(80.0, 80.0);
        let expected_rng = g.rng.clone().next_u64();
        let statue = g.spawn("oHawkmanWorship", origin);
        let hawk = g.spawn("oHawkman", Vec2::new(112.0, 80.0));
        let e = g.world.entity(hawk).unwrap();
        assert_eq!(e.sprite, "sHawkLeft");
        assert_eq!(e.image_speed(), 0.5);
        assert_eq!(e.health, 4);
        assert_eq!(e.vel, Vec2::new(2.5, 0.0));
        assert_eq!(e.bounds(), Rect::new(114.0, 80.0, 12.0, 16.0));
        assert_eq!(e.value("enemy"), 1.0);
        assert_eq!(e.value("enemy_initialized"), 1.0);
        assert_eq!(
            dispatcher::descendant_instances(&g, "oEnemy"),
            vec![dispatcher::Instance::Entity(hawk)]
        );
        assert_eq!(
            dispatcher::descendant_instances(&g, "oHawkmanWorship"),
            vec![dispatcher::Instance::Entity(statue)]
        );
        assert_eq!(g.rng.clone().next_u64(), expected_rng);
    });
}
