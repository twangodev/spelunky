//! scrInitLevel's post-SetupWalls treasure cleanup, before water propagation.
use crate::{CollisionPrecision, GenerationRevision, World};

pub(super) fn cleanup(world: &mut World, revision: GenerationRevision) {
    if revision != GenerationRevision::SourceClassicV2 {
        return;
    }
    let doomed: Vec<_> = world
        .entities
        .iter()
        .filter(|treasure| {
            if !treasure.alive || !crate::catalog::is_a(&treasure.object, "oTreasure") {
                return false;
            }
            // The source first asks collision_point(..., precise=false), then
            // instance_place using the receiver's current sprite collision mask.
            let mut point_hit = false;
            let mut first = None;
            let mask = treasure.collision_mask();
            for (index, tile) in world.tiles.iter().enumerate() {
                if !crate::catalog::is_a(&tile.object, "oSolid") {
                    continue;
                }
                let x = (index % world.width) as i32;
                let y = (index / world.width) as i32;
                let solid = world.tile_collision_mask(x, y);
                point_hit |= solid.contains(treasure.pos, CollisionPrecision::BoundingBox);
                if mask.intersects_mask(&solid)
                    && first.is_none_or(|(serial, _)| tile.creation_serial < serial)
                {
                    let invincible = world.entities.iter().any(|metadata| {
                        metadata.alive
                            && metadata.object == "oTerrainMetadata"
                            && (metadata.pos.x / 16.).floor() as i32 == x
                            && (metadata.pos.y / 16.).floor() as i32 == y
                            && metadata.value("invincible") != 0.
                    });
                    first = Some((tile.creation_serial, invincible));
                }
            }
            for solid in world
                .entities
                .iter()
                .filter(|solid| solid.alive && crate::catalog::is_a(&solid.object, "oSolid"))
            {
                point_hit |= solid.mask_contains(treasure.pos, CollisionPrecision::BoundingBox);
                if mask.intersects_mask(&solid.collision_mask())
                    && first.is_none_or(|(serial, _)| solid.creation_serial < serial)
                {
                    first = Some((solid.creation_serial, solid.value("invincible") != 0.));
                }
            }
            point_hit && first.is_some_and(|(_, invincible)| invincible)
        })
        .map(|treasure| treasure.id)
        .collect();
    // No oTreasure descendant or ancestor defines a Destroy callback.
    for id in doomed {
        world.entity_mut(id).unwrap().alive = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Area, Rng, Vec2};

    #[test]
    fn exit_support_create_gem_is_removed_only_for_source_classic() {
        let mut witnessed = 0;
        for seed in 0..512 {
            let mut world = World::new(14, 12, Area::Jungle, 5);
            let mut rng = Rng::new(seed);
            // Actual room '9' producer creates the exit's invincible support.
            super::super::tiles::stamp_at(&mut world, &mut rng, "9", 3, 3, false, true, "");
            super::super::terrain::create_generated_with_context(
                &mut world,
                &mut rng,
                super::super::GenerationContext::default(),
            );
            let ids: Vec<_> = world
                .entities
                .iter()
                .filter(|e| {
                    e.alive
                        && crate::catalog::is_a(&e.object, "oTreasure")
                        && e.pos == Vec2::new(56., 72.)
                })
                .map(|e| e.id)
                .collect();
            if ids.is_empty() {
                continue;
            }
            witnessed += ids.len();
            let mut historical = world.clone();
            cleanup(&mut historical, GenerationRevision::HistoricalV1);
            assert!(ids.iter().all(|id| historical.entity(*id).unwrap().alive));
            let state = rng.state_bits();
            cleanup(&mut world, GenerationRevision::SourceClassicV2);
            assert!(
                ids.iter()
                    .all(|id| !world.entities.iter().find(|e| e.id == *id).unwrap().alive)
            );
            assert_eq!(rng.state_bits(), state);
        }
        assert!(
            witnessed > 0,
            "fixed actual producer sweep must birth a gem"
        );
    }

    #[test]
    fn first_solid_controls_invincibility_and_entities_keep_birth_order() {
        for first_invincible in [false, true] {
            let mut world = World::new(8, 8, Area::Jungle, 5);
            let first = world.spawn("oLush", Vec2::new(32., 32.));
            world
                .entity_mut(first)
                .unwrap()
                .data
                .insert("invincible".into(), if first_invincible { 1. } else { 0. });
            let second = world.spawn("oLush", Vec2::new(32., 32.));
            world
                .entity_mut(second)
                .unwrap()
                .data
                .insert("invincible".into(), if !first_invincible { 1. } else { 0. });
            let treasure = world.spawn("oRubyBig", Vec2::new(40., 40.));
            world.entities.reverse();
            cleanup(&mut world, GenerationRevision::SourceClassicV2);
            assert_eq!(
                world
                    .entities
                    .iter()
                    .find(|e| e.id == treasure)
                    .unwrap()
                    .alive,
                !first_invincible
            );
        }
    }

    #[test]
    fn cleanup_requires_origin_point_and_treasure_family() {
        let mut world = World::new(8, 8, Area::Jungle, 5);
        let solid = world.spawn("oLush", Vec2::new(32., 32.));
        world
            .entity_mut(solid)
            .unwrap()
            .data
            .insert("invincible".into(), 1.);
        let overlapping = world.spawn("oRubyBig", Vec2::new(30., 40.));
        let item = world.spawn("oBombBag", Vec2::new(40., 40.));
        let outside = world.spawn("oRubyBig", Vec2::new(80., 80.));
        assert!(
            world
                .entity(overlapping)
                .unwrap()
                .event_intersects(world.entity(solid).unwrap())
        );
        cleanup(&mut world, GenerationRevision::SourceClassicV2);
        for id in [overlapping, item, outside] {
            assert!(world.entity(id).unwrap().alive);
        }
    }
}
