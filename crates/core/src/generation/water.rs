//! Water's scrSetupWalls pass and scrInitLevel's horizontal surface propagation.
//! These are room-generation events; custom scrLoadLevel does not invoke them.
use crate::{Rect, Rng, Vec2, World, catalog};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Cell {
    Tile(usize),
    Instance(u64),
}

fn cells(world: &World) -> Vec<Cell> {
    world
        .tiles
        .iter()
        .enumerate()
        .filter(|(_, tile)| catalog::is_a(&tile.object, "oWater"))
        .map(|(index, _)| Cell::Tile(index))
        .chain(
            world
                .entities
                .iter()
                .filter(|entity| entity.alive && catalog::is_a(&entity.object, "oWater"))
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

fn point(world: &World, pos: Vec2, family: &str) -> bool {
    world.tiles_point(pos, family)
        || world
            .entities
            .iter()
            .any(|e| e.alive && catalog::is_a(&e.object, family) && e.sprite_bounds().contains(pos))
}

// instance_place moves the current instance mask to the supplied origin.
// It does not reduce the source's query to a tile lookup or center distance.
fn place(world: &World, source: Cell, destination: Vec2) -> Option<Cell> {
    let old = origin(world, source);
    let r = bounds(world, source);
    let shifted = Rect::new(
        r.x + destination.x - old.x,
        r.y + destination.y - old.y,
        r.w,
        r.h,
    );
    cells(world)
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
    for cell in cells(world) {
        if lava(world, cell) {
            continue;
        }
        #[cfg(test)]
        let stock_prefix = stock_prefix_tests::observe_before(world, rng, cell);
        let p = origin(world, cell);
        let above = Vec2::new(p.x, p.y - 16.0);
        let below = Vec2::new(p.x, p.y + 16.0);
        let up_water = point(world, above, "oWater");
        let up = point(world, above, "oSolid");
        let down = point(world, below, "oSolid") && !point(world, below, "oWater");
        if !up && !up_water {
            set_sprite(world, cell, "sWaterTop");
        }
        let tall = if revision == crate::GenerationRevision::SourceClassicV2 {
            // Original AND evaluates each operand expression before its
            // operator truth checks, even when upWater/down is false.
            // Preserve query then rand order; the else-if roll is separate.
            let two_above = point(world, Vec2::new(p.x, p.y - 32.0), "oWater");
            let tall_roll = rng.chance(1, 4);
            up_water && two_above && down && tall_roll
        } else {
            // Explicit historical generation retains its earlier RNG tape.
            up_water
                && point(world, Vec2::new(p.x, p.y - 32.0), "oWater")
                && down
                && rng.chance(1, 4)
        };
        if tall {
            set_sprite(world, cell, "sWaterBottomTall2");
            if let Some(above_cell) = place(world, cell, above) {
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
pub(super) fn propagate_surface(world: &mut World) {
    for seed in cells(world) {
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
            let Some(next) = place(world, cell, destination) else {
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
        propagate_surface(&mut w);
        assert_eq!(w.entity(covered).unwrap().sprite, "sWaterTop");
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
        propagate_surface(&mut w);
        assert_eq!(w.entity(water).unwrap().sprite, "sWaterTop");
        assert_eq!(w.entity(lava).unwrap().sprite, "sLavaTop");
    }
}

#[cfg(test)]
#[path = "water_stock_prefix_tests.rs"]
mod stock_prefix_tests;
