//! Water's scrSetupWalls pass and scrInitLevel's horizontal surface propagation.
//! These are room-generation events; custom scrLoadLevel does not invoke them.
use crate::{Rect, Rng, Vec2, World, catalog};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cell {
    Tile(usize),
    Instance(u64),
}

fn cells(world: &World, active_only: bool) -> Vec<Cell> {
    world
        .tiles
        .iter()
        .enumerate()
        .filter(|(_, tile)| {
            (!active_only || tile.engine_active) && catalog::is_a(&tile.object, "oWater")
        })
        .map(|(index, _)| Cell::Tile(index))
        .chain(
            world
                .entities
                .iter()
                .filter(|entity| {
                    entity.alive
                        && (!active_only || entity.engine_active)
                        && catalog::is_a(&entity.object, "oWater")
                })
                .map(|entity| Cell::Instance(entity.id)),
        )
        .collect()
}

fn origin(world: &World, cell: Cell) -> Vec2 {
    match cell {
        Cell::Tile(index) => Vec2::new(
            (index % world.width) as f64 * 16.0,
            (index / world.width) as f64 * 16.0,
        ),
        Cell::Instance(id) => world.entity(id).unwrap().pos,
    }
}

fn lava(world: &World, cell: Cell) -> bool {
    let object = match cell {
        Cell::Tile(index) => &world.tiles[index].object,
        Cell::Instance(id) => &world.entity(id).unwrap().object,
    };
    catalog::is_a(object, "oLava")
}

fn bounds(world: &World, cell: Cell) -> Rect {
    match cell {
        Cell::Tile(index) => {
            world.tile_bounds((index % world.width) as i32, (index / world.width) as i32)
        }
        Cell::Instance(id) => world.entity(id).unwrap().sprite_bounds(),
    }
}

fn sprite(world: &World, cell: Cell) -> &str {
    match cell {
        Cell::Tile(index) => {
            let p = origin(world, cell);
            let center = Vec2::new(p.x + 8.0, p.y + 8.0);
            world
                .entities
                .iter()
                .find(|e| e.alive && e.object == "oTerrainMetadata" && e.pos == center)
                .and_then(|e| e.text.get("terrain_sprite"))
                .map(String::as_str)
                .unwrap_or_else(|| catalog::sprite(&world.tiles[index].object).unwrap_or(""))
        }
        Cell::Instance(id) => &world.entity(id).unwrap().sprite,
    }
}

fn set_sprite(world: &mut World, cell: Cell, sprite: &str) {
    match cell {
        Cell::Instance(id) => world.entity_mut(id).unwrap().sprite = sprite.into(),
        Cell::Tile(_) => {
            let p = origin(world, cell);
            let center = Vec2::new(p.x + 8.0, p.y + 8.0);
            let id = world
                .entities
                .iter()
                .find(|e| e.alive && e.object == "oTerrainMetadata" && e.pos == center)
                .map(|e| e.id)
                .unwrap_or_else(|| world.spawn("oTerrainMetadata", center));
            let metadata = world.entity_mut(id).unwrap();
            metadata.sprite.clear();
            metadata.text.insert("terrain_sprite".into(), sprite.into());
        }
    }
}

fn point(world: &World, pos: Vec2, family: &str, revision: crate::GenerationRevision) -> bool {
    if revision == crate::GenerationRevision::SourceClassicV2 {
        world.point_family_active(pos, family)
    } else {
        world.tiles_point(pos, family)
            || world.entities.iter().any(|entity| {
                entity.alive
                    && catalog::is_a(&entity.object, family)
                    && entity.sprite_bounds().contains(pos)
            })
    }
}

// instance_place moves the current instance mask to the supplied origin.
// It does not reduce the source's query to a tile lookup or center distance.
fn place(world: &World, source: Cell, destination: Vec2, active_only: bool) -> Option<Cell> {
    let old = origin(world, source);
    let r = bounds(world, source);
    let shifted = Rect::new(
        r.x + destination.x - old.x,
        r.y + destination.y - old.y,
        r.w,
        r.h,
    );
    cells(world, active_only)
        .into_iter()
        .find(|&candidate| shifted.intersects(bounds(world, candidate)))
}

/// scrSetupWalls.gml:360–398. Invoke before the separate lava setup loop.
#[cfg(test)]
pub(super) fn setup(world: &mut World, rng: &mut Rng) {
    setup_with_revision(world, rng, crate::GenerationRevision::HistoricalV1);
}

pub(super) fn setup_with_revision(
    world: &mut World,
    rng: &mut Rng,
    revision: crate::GenerationRevision,
) {
    let active_only = revision == crate::GenerationRevision::SourceClassicV2;
    for cell in cells(world, active_only) {
        if lava(world, cell) {
            continue;
        }
        #[cfg(test)]
        let stock_prefix = stock_prefix_tests::observe_before(world, rng, cell);
        let p = origin(world, cell);
        let above = Vec2::new(p.x, p.y - 16.0);
        let below = Vec2::new(p.x, p.y + 16.0);
        let up_water = point(world, above, "oWater", revision);
        let up = point(world, above, "oSolid", revision);
        let down =
            point(world, below, "oSolid", revision) && !point(world, below, "oWater", revision);
        if !up && !up_water {
            set_sprite(world, cell, "sWaterTop");
        }
        let tall = if revision == crate::GenerationRevision::SourceClassicV2 {
            // Original AND evaluates each operand expression before its
            // operator truth checks, even when upWater/down is false.
            // Preserve query then rand order; the else-if roll is separate.
            let two_above = point(world, Vec2::new(p.x, p.y - 32.0), "oWater", revision);
            let tall_roll = rng.chance(1, 4);
            up_water && two_above && down && tall_roll
        } else {
            // Explicit historical generation retains its earlier RNG tape.
            up_water
                && point(world, Vec2::new(p.x, p.y - 32.0), "oWater", revision)
                && down
                && rng.chance(1, 4)
        };
        if tall {
            set_sprite(world, cell, "sWaterBottomTall2");
            if let Some(above_cell) = place(world, cell, above, active_only) {
                set_sprite(world, above_cell, "sWaterBottomTall1");
            }
        } else if (up || up_water) && down {
            let selected = match rng.range(4) {
                0 => "sWaterBottom",
                1 => "sWaterBottom2",
                2 => "sWaterBottom3",
                _ => "sWaterBottom4",
            };
            set_sprite(world, cell, selected);
        }
        #[cfg(test)]
        stock_prefix_tests::observe_after(world, rng, stock_prefix);
    }
}

/// scrInitLevel.gml:269 and scrCheckWaterTop.gml:26–50. Explicit stack avoids
/// overflowing Rust's call stack for long connected rows, preserving left-first
/// recursive visitation and the source's current-mask instance_place queries.
pub(super) fn propagate_surface(world: &mut World, active_only: bool) {
    for seed in cells(world, active_only) {
        if !matches!(sprite(world, seed), "sWaterTop" | "sLavaTop") {
            continue;
        }
        let mut stack = vec![(seed, 0)];
        while let Some((cell, side)) = stack.pop() {
            if side > 1 {
                continue;
            }
            stack.push((cell, side + 1));
            let p = origin(world, cell);
            let destination = Vec2::new(p.x + if side == 0 { -16.0 } else { 16.0 }, p.y);
            let Some(next) = place(world, cell, destination, active_only) else {
                continue;
            };
            if matches!(sprite(world, next), "sWaterTop" | "sLavaTop") {
                continue;
            }
            let top = if lava(world, next) {
                "sLavaTop"
            } else {
                "sWaterTop"
            };
            set_sprite(world, next, top);
            stack.push((next, 0));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Area;

    #[test]
    fn covered_neighbor_gets_surface_from_source_horizontal_recursion() {
        let mut w = World::new(12, 12, Area::Jungle, 5);
        let open = w.spawn("oWaterSwim", Vec2::new(64.0, 80.0));
        let covered = w.spawn("oWaterSwim", Vec2::new(80.0, 80.0));
        w.set_tile(5, 4, "oLush");
        setup(&mut w, &mut Rng::new(1));
        assert_eq!(w.entity(open).unwrap().sprite, "sWaterTop");
        assert_eq!(w.entity(covered).unwrap().sprite, "sWater");
        propagate_surface(&mut w, false);
        assert_eq!(w.entity(covered).unwrap().sprite, "sWaterTop");
    }

    #[test]
    fn source_setup_walls_skips_inactive_water_receivers_but_history_is_retained() {
        for dynamic in [false, true] {
            for revision in [
                crate::GenerationRevision::HistoricalV1,
                crate::GenerationRevision::SourceClassicV2,
            ] {
                let mut world = World::new(12, 12, Area::Jungle, 5);
                let before = if dynamic {
                    let id = world.spawn("oWaterSwim", Vec2::new(64.0, 80.0));
                    world.entity_mut(id).unwrap().engine_active = false;
                    world.entity(id).unwrap().sprite.clone()
                } else {
                    world.set_tile(4, 5, "oWater");
                    world.tiles[5 * world.width + 4].engine_active = false;
                    String::new()
                };

                setup_with_revision(&mut world, &mut Rng::new(1), revision);

                let source_skips = revision == crate::GenerationRevision::SourceClassicV2;
                if dynamic {
                    let water = world
                        .entities
                        .iter()
                        .find(|entity| entity.object == "oWaterSwim")
                        .unwrap();
                    assert_eq!(water.sprite == before, source_skips, "{revision:?}");
                } else {
                    let metadata = world.entities.iter().find(|entity| {
                        entity.object == "oTerrainMetadata" && entity.pos == Vec2::new(72.0, 88.0)
                    });
                    assert_eq!(metadata.is_none(), source_skips, "{revision:?}");
                }
            }
        }
    }

    #[test]
    fn literal_origin_and_dynamic_solid_mask_control_surface() {
        let mut w = World::new(12, 12, Area::Jungle, 5);
        let water = w.spawn("oWaterSwim", Vec2::new(65.0, 80.0));
        // The translated source mask extends left of its origin. A cell-only
        // lookup at the water's center would miss this overhead solid.
        let ceiling = w.spawn("oBlock", Vec2::new(50.0, 64.0));
        setup(&mut w, &mut Rng::new(1));
        assert_eq!(w.entity(water).unwrap().sprite, "sWater");
        w.entity_mut(ceiling).unwrap().alive = false;
        setup(&mut w, &mut Rng::new(1));
        assert_eq!(w.entity(water).unwrap().sprite, "sWaterTop");
    }

    #[test]
    fn source_buried_pool_uses_bottom_variants_and_tall_pairs() {
        let mut saw_tall = false;
        for seed in 1..=32 {
            let mut w = World::new(12, 12, Area::Jungle, 5);
            w.spawn("oWaterSwim", Vec2::new(80.0, 48.0));
            let middle = w.spawn("oWaterSwim", Vec2::new(80.0, 64.0));
            let bottom = w.spawn("oWaterSwim", Vec2::new(80.0, 80.0));
            w.set_tile(5, 6, "oLush");
            setup(&mut w, &mut Rng::new(seed));
            let e = w.entity(bottom).unwrap();
            assert!(e.sprite.starts_with("sWaterBottom"));
            if e.sprite == "sWaterBottomTall2" {
                saw_tall = true;
                assert_eq!(w.entity(middle).unwrap().sprite, "sWaterBottomTall1");
            }
        }
        assert!(saw_tall);
    }

    #[test]
    fn lava_seed_propagates_to_water_and_preserves_lava_type() {
        let mut w = World::new(12, 12, Area::Jungle, 5);
        let lava = w.spawn("oLava", Vec2::new(64.0, 80.0));
        let water = w.spawn("oWaterSwim", Vec2::new(80.0, 80.0));
        w.entity_mut(lava).unwrap().sprite = "sLavaTop".into();
        propagate_surface(&mut w, false);
        assert_eq!(w.entity(water).unwrap().sprite, "sWaterTop");
        assert_eq!(w.entity(lava).unwrap().sprite, "sLavaTop");
    }

    #[test]
    fn source_surface_propagation_ignores_inactive_water_instances() {
        for active_only in [false, true] {
            let mut world = World::new(12, 12, Area::Jungle, 5);
            let lava = world.spawn("oLava", Vec2::new(64.0, 80.0));
            let water = world.spawn("oWaterSwim", Vec2::new(80.0, 80.0));
            world.entity_mut(lava).unwrap().sprite = "sLavaTop".into();
            world.entity_mut(water).unwrap().engine_active = false;
            let original = world.entity(water).unwrap().sprite.clone();

            propagate_surface(&mut world, active_only);

            assert_eq!(
                world.entity(water).unwrap().sprite == original,
                active_only,
                "active_only={active_only}"
            );
        }
    }
}

#[cfg(test)]
#[path = "water_stock_prefix_tests.rs"]
mod stock_prefix_tests;
