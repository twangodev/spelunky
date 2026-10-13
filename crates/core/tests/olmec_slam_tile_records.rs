//! Source tile_layer_find(3,x,y-16)/tile_delete for the invisible Slam callback.
//! The generated-fringe witness configures the initial grounded SLAM pose;
//! it is not a claim about an ordinary input-only Boss encounter.
use spelunky_core::*;
use std::collections::BTreeSet;

fn modes() -> impl Iterator<Item = (RulesPreset, RngAlgorithm)> {
    [RulesPreset::Classic, RulesPreset::Assisted]
        .into_iter()
        .flat_map(|rules| {
            [RngAlgorithm::Gm8Lcg32V1, RngAlgorithm::LegacySplitMix64V1]
                .into_iter()
                .map(move |algorithm| (rules, algorithm))
        })
}
fn source_tile(game: &mut Game, point: Vec2, depth: f64) -> u64 {
    let id = game.spawn("oGenerationDecoration", point);
    let e = game.world.entity_mut(id).unwrap();
    e.set_sprite("bgCaveTop3");
    for (key, value) in [
        ("decoration", 1.0),
        ("source_x", 0.0),
        ("source_y", 0.0),
        ("source_w", 16.0),
        ("source_h", 16.0),
        ("depth", depth),
    ] {
        e.data.insert(key.into(), value);
    }
    id
}
fn configure_impact(game: &mut Game, boss: u64, position: Vec2) {
    // These fields are authored only before the first real Step.
    let e = game.world.entity_mut(boss).unwrap();
    e.pos = position;
    e.previous_pos = position;
    e.state = 6;
    e.vel = Vec2::new(0.0, 5.0);
    e.set_sprite("sOlmec");
    e.data.insert("slammed".into(), 0.0);
}
fn generated_fixture(rules: RulesPreset, algorithm: RngAlgorithm) -> (Game, u64, usize, u64) {
    // Select by actual generated source-default geometry, never by behavior
    // under test. No injected fringe/terrain and no graphics override.
    for seed in 0..8 {
        let mut game = Game::shell_with_algorithm(seed, rules, algorithm).unwrap();
        game.world = generation::generate(&mut game.rng, 16, &BTreeSet::new());
        let selected = game.world.entities.iter().find_map(|tile| {
            if tile.object != "oGenerationDecoration"
                || tile.value("depth") != 3.0
                || tile.pos.y != 544.0
                || !(160.0..480.0).contains(&tile.pos.x)
            {
                return None;
            }
            let index = 35 * game.world.width + (tile.pos.x / 16.0) as usize;
            (game.world.tiles[index].object == "oTemple").then_some((tile.id, index, tile.pos))
        });
        let Some((tile, index, fringe)) = selected else {
            continue;
        };
        let boss = game
            .world
            .entities
            .iter()
            .find(|e| e.object == "oOlmec")
            .unwrap()
            .id;
        game.player.pos = game.world.entrance;
        game.player.previous_pos = game.player.pos;
        configure_impact(&mut game, boss, Vec2::new(fringe.x, 496.0));
        return (game, tile, index, boss);
    }
    panic!(
        "no source-default generated Boss floor fringe in seeds0..8 for {rules:?}/{algorithm:?}"
    );
}
#[test]
fn real_own_step_slam_removes_generated_default_fringe_and_matching_solid() {
    for (rules, algorithm) in modes() {
        let (mut game, tile, index, boss) = generated_fixture(rules, algorithm);
        let prior: BTreeSet<_> = game.world.entities.iter().map(|e| e.id).collect();
        let origin = Vec2::new((index % game.world.width) as f64 * 16.0, 560.0);
        let generated = game.world.entity(tile).unwrap();
        assert_eq!(generated.pos, Vec2::new(origin.x, origin.y - 16.0));
        assert_eq!(generated.sprite, "bgCaveTop4");
        assert_eq!(
            (generated.value("source_w"), generated.value("source_h")),
            (16.0, 16.0)
        );
        assert!(
            game.world
                .entities
                .iter()
                .filter(|e| e.object == "oGenerationDecoration" && e.value("depth") == 3.0)
                .count()
                > 4
        );
        let unrelated = game
            .world
            .entities
            .iter()
            .filter(|e| e.object == "oGenerationDecoration" && e.value("depth") != 3.0)
            .map(|e| e.id)
            .collect::<Vec<_>>();
        game.step(&InputFrame::default());
        let slam = game
            .world
            .entities
            .iter()
            .find(|e| e.alive && e.object == "oOlmecSlam" && !prior.contains(&e.id))
            .unwrap();
        assert!(slam.mask_contains(
            Vec2::new(origin.x + 2.0, 565.0),
            CollisionPrecision::Precise
        ));
        assert_eq!(slam.alarms[0], 1);
        assert_eq!(game.world.entity(boss).unwrap().value("slammed"), 1.0);
        assert!(
            game.world.tiles[index].object.is_empty(),
            "real Slam retired the generated matching Solid"
        );
        assert!(
            game.world.entity(tile).is_none(),
            "SLAM_MUST_DELETE_GENERATED_LAYER3_TILE"
        );
        assert!(
            game.world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oGenerationDecoration" && e.value("depth") == 3.0),
            "one impact must not erase the whole layer"
        );
        assert!(unrelated.iter().all(|id| game.world.entity(*id).is_some()));
    }
}
#[test]
fn real_slam_deletes_one_layer3_point_record_and_preserves_overlap_depth_edge_and_actor_controls() {
    for (rules, algorithm) in modes() {
        for terrain in [false, true] {
            let mut game = Game::shell_with_algorithm(42, rules, algorithm).unwrap();
            for x in 1..=3 {
                game.world.set_tile(x, 3, "oHardBlock");
            }
            let mut targets = Vec::new();
            for x in 6..=10 {
                if terrain {
                    game.world.set_tile(x, 10, "oBrickSmooth");
                } else {
                    targets.push(game.spawn("oBrickSmooth", Vec2::new(x as f64 * 16.0, 160.0)));
                }
            }
            let first = source_tile(&mut game, Vec2::new(96.0, 144.0), 3.0);
            let overlap = source_tile(&mut game, Vec2::new(96.0, 144.0), 3.0);
            let wrong_depth = source_tile(&mut game, Vec2::new(96.0, 144.0), 2.0);
            let point_outside = source_tile(&mut game, Vec2::new(96.0, 145.0), 3.0);
            let right_edge = source_tile(&mut game, Vec2::new(80.0, 144.0), 3.0);
            let other_layer3 = source_tile(&mut game, Vec2::new(256.0, 144.0), 3.0);
            // A real source actor at the tile query point is never a tile_add
            // record merely because it carries the same depth/sprite crop.
            let actor = game.spawn("oRubbleSmall", Vec2::new(96.0, 144.0));
            game.world
                .entity_mut(actor)
                .unwrap()
                .data
                .insert("depth".into(), 3.0);
            let boss = game.spawn("oOlmec", Vec2::new(107.0, 96.0));
            configure_impact(&mut game, boss, Vec2::new(107.0, 96.0));
            let prior: BTreeSet<_> = game.world.entities.iter().map(|e| e.id).collect();
            game.step(&InputFrame::default());
            assert!(
                game.world
                    .entities
                    .iter()
                    .any(|e| e.alive && e.object == "oOlmecSlam" && !prior.contains(&e.id)),
                "ordinary own Step must emit the actual Slam callback receiver"
            );
            for x in 6..=10 {
                if terrain {
                    assert!(game.world.tile(x, 10).unwrap().object.is_empty());
                }
            }
            assert!(targets.iter().all(|id| game.world.entity(*id).is_none()));
            assert!(
                game.world.entity(first).is_none(),
                "SLAM_MUST_DELETE_ONE_POINT_LAYER3_RECORD"
            );
            for id in [
                overlap,
                wrong_depth,
                point_outside,
                right_edge,
                other_layer3,
                actor,
            ] {
                assert!(
                    game.world.entity(id).is_some(),
                    "must preserve distinct tile/actor control {id}"
                );
            }
            // Alarm retires this Slam before a later collision can delete the
            // overlapping second record. No animation/state/timer authoring.
            game.step(&InputFrame::default());
            assert!(game.world.entity(overlap).is_some());
        }
    }
}
