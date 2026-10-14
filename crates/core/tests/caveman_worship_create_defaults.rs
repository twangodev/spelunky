//! oCavemanWorship has no parent. Its sole own Create assignment is
//! image_speed=0.25; rOlmec instances 134462-134464 have empty creationCode.
use spelunky_core::{
    Entity, Game, InputFrame, Rect, Rng, RngAlgorithm, RulesPreset, Vec2, catalog, dispatcher,
    generation,
};
use std::collections::BTreeSet;

fn configurations(mut run: impl FnMut(RulesPreset, RngAlgorithm)) {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for algorithm in [RngAlgorithm::LegacySplitMix64V1, RngAlgorithm::Gm8Lcg32V1] {
            run(rules, algorithm);
        }
    }
}

fn assert_parentless(e: &Entity, origin: Vec2) {
    let def = catalog::object("oCavemanWorship").unwrap();
    assert_eq!(def.parent, "");
    assert_eq!(def.depth, 60);
    assert!(def.visible);
    assert!(!def.solid);
    assert_eq!(e.sprite, "sCavemanWorshipR");
    assert_eq!(e.pos, origin);
    assert_eq!(e.vel, Vec2::default());
    assert!(!e.solid);
    // These variables belong to Enemy's Create, which this object never inherits.
    for field in [
        "enemy_initialized",
        "enemy_context_created",
        "enemy",
        "counts_as_kill",
        "blood_left",
        "stun_time",
        "hit_cooldown",
        "size_width",
        "size_height",
        "heavy",
    ] {
        assert!(!e.data.contains_key(field), "invented parent field {field}");
    }
    // Source does not create hp here; the mandatory Rust field is an unchanged
    // Entity model placeholder, not an assertion that Original hp equals one.
    assert_eq!(e.health, Entity::new(0, "oCavemanWorship", origin).health);
}

#[test]
fn natural_caveman_statue_birth_runs_only_own_speed_assignment_without_rng_or_enemy_parent() {
    configurations(|rules, algorithm| {
        let mut g = Game::shell_with_algorithm(42, rules, algorithm).unwrap();
        let expected_rng = g.rng.clone().next_u64();
        let origin = Vec2::new(80.0, 80.0);
        let id = g.spawn("oCavemanWorship", origin);
        let e = g.world.entity(id).unwrap();
        assert_parentless(e, origin);
        assert_eq!(e.image_speed(), 0.25);
        assert_eq!(e.animation, 0.0);
        assert_eq!(e.sprite_bounds(), Rect::new(80.0, 81.0, 12.0, 15.0)); // literal frame zero
        assert_eq!(g.rng.clone().next_u64(), expected_rng);
        assert_eq!(
            dispatcher::descendant_instances(&g, "oCavemanWorship"),
            vec![dispatcher::Instance::Entity(id)]
        );
        assert!(dispatcher::descendant_instances(&g, "oEnemy").is_empty());
    });
}

#[test]
fn statue_has_no_own_step_and_post_create_freeze_survives_bootstrap() {
    configurations(|rules, algorithm| {
        for frozen in [false, true] {
            let mut g = Game::shell_with_algorithm(42, rules, algorithm).unwrap();
            g.player.pos = Vec2::new(200.0, 120.0);
            g.player.data.insert("active".into(), 0.0);
            let origin = Vec2::new(80.0, 80.0);
            let id = g.spawn("oCavemanWorship", origin);
            if frozen {
                // Initial-only caller boundary of oOlmec Alarm1's with-body.
                // The existing natural intro test separately reaches that Alarm.
                let statue = g.world.entity_mut(id).unwrap();
                statue.animation = 0.0;
                statue.set_image_speed(0.0);
            }
            let expected_rng = g.rng.clone().next_u64();
            for tick in 1..=4 {
                g.step(&InputFrame::default());
                let e = g.world.entity(id).unwrap();
                assert_parentless(e, origin);
                assert_eq!(e.previous_pos, origin);
                assert_eq!(e.age, tick);
                assert_eq!(e.image_speed(), if frozen { 0.0 } else { 0.25 });
                assert_eq!(
                    e.animation,
                    if frozen { 0.0 } else { f64::from(tick) * 0.25 }
                );
                assert_eq!(g.rng.clone().next_u64(), expected_rng);
                assert!(dispatcher::descendant_instances(&g, "oEnemy").is_empty());
            }
        }
    });
}

#[test]
fn all_three_real_authored_olmec_statues_keep_own_create_without_enemy_fields() {
    let mut rng = Rng::new(42);
    let world = generation::generate(&mut rng, 16, &BTreeSet::new());
    let statues: Vec<_> = world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oCavemanWorship")
        .collect();
    assert_eq!(statues.len(), 3);
    let origins = [
        Vec2::new(264.0, 544.0),
        Vec2::new(248.0, 544.0),
        Vec2::new(232.0, 544.0),
    ];
    for (e, origin) in statues.iter().zip(origins) {
        assert_parentless(e, origin);
        assert_eq!(e.image_speed(), 0.25);
        assert_eq!(e.animation, 0.0);
    }
    assert!(
        statues
            .windows(2)
            .all(|pair| pair[0].creation_serial < pair[1].creation_serial)
    );
}

#[test]
fn actual_caveman_and_giant_hanging_spider_keep_complete_parent_and_own_births() {
    configurations(|rules, algorithm| {
        let mut g = Game::shell_with_algorithm(42, rules, algorithm).unwrap();
        let expected_rng = g.rng.clone().next_u64();
        let caveman = g.spawn("oCaveman", Vec2::new(80.0, 80.0));
        let giant = g.spawn("oGiantSpiderHang", Vec2::new(128.0, 80.0));
        let e = g.world.entity(caveman).unwrap();
        assert_eq!(e.health, 3);
        assert_eq!(e.image_speed(), 0.5);
        assert_eq!(e.vel, Vec2::new(2.5, 0.0));
        assert_eq!(e.value("enemy_initialized"), 1.0);
        let e = g.world.entity(giant).unwrap();
        assert_eq!(e.health, 10);
        assert_eq!(e.image_speed(), 0.4);
        assert_eq!(e.vel, Vec2::default());
        assert_eq!(e.bounds(), Rect::new(128.0, 80.0, 32.0, 16.0));
        assert_eq!(e.value("enemy_initialized"), 1.0);
        let webs: Vec<_> = g
            .world
            .entities
            .iter()
            .filter(|e| e.alive && e.object == "oWeb")
            .collect();
        assert_eq!(webs.len(), 2);
        assert_eq!(webs[0].pos, Vec2::new(128.0, 96.0));
        assert_eq!(webs[1].pos, Vec2::new(144.0, 96.0));
        assert!(e.creation_serial < webs[0].creation_serial);
        assert!(webs[0].creation_serial < webs[1].creation_serial);
        assert_eq!(
            dispatcher::descendant_instances(&g, "oEnemy"),
            vec![
                dispatcher::Instance::Entity(caveman),
                dispatcher::Instance::Entity(giant)
            ]
        );
        assert_eq!(g.rng.clone().next_u64(), expected_rng);
    });
}
