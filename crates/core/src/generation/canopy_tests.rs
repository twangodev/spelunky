use super::*;
use crate::{Area, CollisionPrecision, GenerationRevision};

fn world() -> World {
    World::new(20, 20, Area::Jungle, 5)
}

fn metadata(world: &World, x: usize, y: usize) -> &crate::Entity {
    world
        .entities
        .iter()
        .find(|entity| {
            entity.alive
                && entity.object == "oTerrainMetadata"
                && entity.pos == Vec2::new(x as f64 * 16.0 + 8.0, y as f64 * 16.0 + 8.0)
        })
        .unwrap()
}

#[test]
fn vine_endpoints_change_the_real_mask_and_isolated_vine_prefers_source() {
    let mut world = world();
    for y in 2..=4 {
        world.set_tile(2, y, "oVine");
    }
    world.set_tile(5, 4, "oVine");
    vines(&mut world);
    assert_eq!(metadata(&world, 2, 2).text["terrain_sprite"], "sVineSource");
    assert!(!metadata(&world, 2, 3).text.contains_key("terrain_sprite"));
    assert_eq!(metadata(&world, 2, 4).text["terrain_sprite"], "sVineBottom");
    assert_eq!(metadata(&world, 5, 4).text["terrain_sprite"], "sVineSource");
    assert!(world.tiles_point(Vec2::new(32.0, 32.0), "oLadder"));
    assert!(!world.tiles_point(Vec2::new(33.0, 64.0), "oLadder"));
    assert!(world.tiles_point(Vec2::new(40.0, 64.0), "oLadder"));
}

#[test]
fn mixed_storage_uses_source_origins_and_ladder_family_but_not_vine_top_receivers() {
    let mut world = world();
    world.set_tile(3, 2, "oVineTop");
    let id = world.spawn("oVine", Vec2::new(48.25, 48.0));
    let untouched = world.spawn("oVineTop", Vec2::new(112.0, 48.0));
    vines(&mut world);
    assert_eq!(world.entity(id).unwrap().sprite, "sVineBottom");
    assert_eq!(world.entity(untouched).unwrap().sprite, "sVineTop");
    assert!(!world.entity(untouched).unwrap().data.contains_key("up"));
    assert!(
        !world
            .entities
            .iter()
            .any(|e| e.pos == Vec2::new(56.0, 40.0))
    );
}

#[test]
fn tree_top_sets_depth_and_dead_variant_without_mutating_tree_tile_siblings() {
    for cemetery in [false, true] {
        let mut world = world();
        world.set_tile(3, 3, "oTree");
        world.set_tile(3, 4, "oTree");
        world.set_tile(3, 2, "oTreeTile");
        let id = world.spawn("oTree", Vec2::new(48.0, 80.0));
        let sibling = world.spawn("oTreeTile", Vec2::new(144.0, 48.0));
        trees_and_branches(&mut world, &mut Rng::new_canonical(7), cemetery);
        let top = metadata(&world, 3, 3);
        assert_eq!(
            top.text["terrain_sprite"],
            if cemetery { "sTreeTopDead" } else { "sTreeTop" }
        );
        assert_eq!(top.value("depth"), 1.0);
        assert!(!metadata(&world, 3, 4).text.contains_key("terrain_sprite"));
        assert_eq!(world.entity(id).unwrap().sprite, "sTreeTrunk");
        assert_eq!(world.entity(id).unwrap().value("up"), 1.0);
        assert_eq!(world.entity(sibling).unwrap().sprite, "sTreeTrunk");
        assert!(!world.entity(sibling).unwrap().data.contains_key("up"));
    }
}

#[test]
fn surviving_branches_orient_left_only_when_tree_is_on_the_right() {
    for cemetery in [false, true] {
        let mut world = world();
        world.set_tile(4, 4, "oTree");
        let left = world.spawn("oTreeBranch", Vec2::new(48.0, 64.0));
        let right = world.spawn("oTreeBranch", Vec2::new(80.0, 64.0));
        if cemetery {
            world
                .entity_mut(left)
                .unwrap()
                .set_sprite("sTreeBranchDeadR");
            world
                .entity_mut(right)
                .unwrap()
                .set_sprite("sTreeBranchDeadR");
        }
        trees_and_branches(&mut world, &mut Rng::new_canonical(9), cemetery);
        assert_eq!(
            world.entity(left).unwrap().sprite,
            if cemetery {
                "sTreeBranchDeadL"
            } else {
                "sTreeBranchLeft"
            }
        );
        assert_eq!(
            world.entity(right).unwrap().sprite,
            if cemetery {
                "sTreeBranchDeadR"
            } else {
                "sTreeBranchRight"
            }
        );
    }
}

#[test]
fn covered_branches_destroy_in_birth_order_across_storage_and_keep_destroy_rng() {
    let mut world = world();
    // Entity is born before tile: iterating tiles first would exchange Leaf positions.
    let first = world.spawn("oTreeBranch", Vec2::new(64.0, 64.0));
    world.set_tile(7, 4, "oTreeBranch");
    for x in [4, 7] {
        world.spawn("oLeaves", Vec2::new(x as f64 * 16.0, 48.0));
        world.set_tile(x + 1, 4, "oTree");
    }
    world.fluid_dirty = true;
    let mut rng = Rng::new_canonical(13);
    let mut expected = rng.clone();
    let expected_pos: Vec<_> = [64.0, 112.0]
        .into_iter()
        .map(|x| {
            Vec2::new(
                x + 8.0 + f64::from(expected.range(9)) - f64::from(expected.range(9)),
                72.0 + f64::from(expected.range(9)) - f64::from(expected.range(9)),
            )
        })
        .collect();
    trees_and_branches(&mut world, &mut rng, false);
    assert!(world.entity(first).is_none());
    assert!(world.tile(7, 4).unwrap().object.is_empty());
    assert!(world.fluid_dirty);
    let leaves: Vec<_> = world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oLeaf")
        .collect();
    assert_eq!(
        leaves.iter().map(|e| e.pos).collect::<Vec<_>>(),
        expected_pos
    );
    for leaf in leaves {
        assert_eq!(leaf.vel.y, 0.4);
        assert_eq!(leaf.value("gravity"), 0.01);
        assert_eq!(leaf.image_speed(), 0.2);
    }
    assert_eq!(rng.state_bits(), expected.state_bits());
    assert_eq!(rng.next_u64(), expected.next_u64());
}

#[test]
fn covered_dead_branches_do_not_emit_leaves_even_with_clean_death_unset() {
    let mut world = world();
    let dead = world.spawn("oTreeBranch", Vec2::new(80.0, 80.0));
    world
        .entity_mut(dead)
        .unwrap()
        .set_sprite("sTreeBranchDeadR");
    world.spawn("oLeaves", Vec2::new(80.0, 64.0));
    let mut rng = Rng::new_canonical(42);
    let before = rng.state_bits();
    trees_and_branches(&mut world, &mut rng, true);
    assert!(world.entity(dead).is_none());
    assert!(!world.entities.iter().any(|e| e.object == "oLeaf"));
    assert_eq!(rng.state_bits(), before);
}

#[test]
fn bounding_box_canopy_probe_does_not_use_precise_alpha_or_viewport_culling() {
    let mut world = world();
    let branch = world.spawn("oTreeBranch", Vec2::new(240.0, 254.0));
    let leaves = world.spawn("oLeaves", Vec2::new(240.0, 224.0));
    let probe = Vec2::new(240.0, 238.0);
    assert!(
        world
            .entity(leaves)
            .unwrap()
            .mask_contains(probe, CollisionPrecision::BoundingBox)
    );
    // Source queries precise=false: the transparent pixel (0,14) inside the bbox is enough.
    assert!(
        !world
            .entity(leaves)
            .unwrap()
            .mask_contains(probe, CollisionPrecision::Precise)
    );
    trees_and_branches(&mut world, &mut Rng::new_canonical(10), false);
    assert!(world.entity(branch).is_none());
}

#[test]
fn inactive_vine_receivers_do_not_run_the_setup_walls_body() {
    for dynamic in [false, true] {
        let mut world = world();
        let id = if dynamic {
            let id = world.spawn("oVine", Vec2::new(64.0, 64.0));
            world.entity_mut(id).unwrap().engine_active = false;
            Some(id)
        } else {
            world.set_tile(4, 4, "oVine");
            world.tiles[4 * world.width + 4].engine_active = false;
            None
        };

        vines(&mut world);

        if let Some(id) = id {
            let vine = world.entity(id).unwrap();
            assert_eq!(vine.sprite, "sVine");
            assert!(!vine.data.contains_key("up"));
        } else {
            assert!(
                world
                    .entities
                    .iter()
                    .all(|entity| entity.object != "oTerrainMetadata")
            );
        }
    }
}

#[test]
fn explicit_revision_not_rng_algorithm_controls_setup_including_imported_canonical() {
    for revision in [
        GenerationRevision::HistoricalV1,
        GenerationRevision::SourceClassicV2,
    ] {
        let mut world = world();
        world.set_tile(4, 4, "oVine");
        let branch = world.spawn("oTreeBranch", Vec2::new(96.0, 96.0));
        world.spawn("oLeaves", Vec2::new(96.0, 80.0));
        let mut rng = Rng::new_canonical(17);
        super::super::terrain::setup_generated_with_revision(
            &mut world, &mut rng, false, revision, false,
        );
        if revision == GenerationRevision::HistoricalV1 {
            assert!(world.entity(branch).is_some());
            assert!(
                !world
                    .entities
                    .iter()
                    .any(|e| e.object == "oTerrainMetadata")
            );
            assert_eq!(rng.state_bits(), Rng::new_canonical(17).state_bits());
        } else {
            assert!(world.entity(branch).is_none());
            assert_eq!(metadata(&world, 4, 4).text["terrain_sprite"], "sVineSource");
        }
    }
}
