//! The Vine and Tree/TreeBranch passes of scrSetupWalls:438–513.
//! Called only for SourceClassicV2, around the existing Block pass.
use crate::{CollisionPrecision, Rng, Vec2, World, catalog};

#[derive(Clone, Copy)]
enum Instance {
    Tile(usize),
    Entity(u64),
}

#[derive(Clone, Copy)]
struct Receiver {
    instance: Instance,
    serial: u64,
    origin: Vec2,
}

fn receivers(world: &World, family: &str) -> Vec<Receiver> {
    let mut receivers: Vec<_> = world
        .tiles
        .iter()
        .enumerate()
        .filter(|(_, tile)| catalog::is_a(&tile.object, family))
        .map(|(index, tile)| Receiver {
            instance: Instance::Tile(index),
            serial: tile.creation_serial,
            origin: Vec2::new(
                (index % world.width) as f64 * 16.0,
                (index / world.width) as f64 * 16.0,
            ),
        })
        .chain(world.entities.iter().filter_map(|entity| {
            (entity.alive && catalog::is_a(&entity.object, family)).then_some(Receiver {
                instance: Instance::Entity(entity.id),
                serial: entity.creation_serial,
                origin: entity.pos,
            })
        }))
        .collect();
    // No body creates another receiver in its own family. Merge the two
    // storage forms by birth order, revalidating identity before each body.
    receivers.sort_by_key(|receiver| receiver.serial);
    receivers
}

fn live(world: &World, receiver: Receiver, family: &str) -> bool {
    match receiver.instance {
        Instance::Tile(index) => world.tiles.get(index).is_some_and(|tile| {
            tile.creation_serial == receiver.serial && catalog::is_a(&tile.object, family)
        }),
        Instance::Entity(id) => world.entity(id).is_some_and(|entity| {
            entity.creation_serial == receiver.serial && catalog::is_a(&entity.object, family)
        }),
    }
}

fn point(world: &World, origin: Vec2, family: &str) -> bool {
    world.tiles_point(origin, family)
        || world.entities.iter().any(|entity| {
            entity.alive
                && catalog::is_a(&entity.object, family)
                && entity.mask_contains(origin, CollisionPrecision::BoundingBox)
        })
}

fn properties(world: &mut World, receiver: Receiver) -> &mut crate::Entity {
    let id = match receiver.instance {
        Instance::Tile(index) => {
            let width = world.width;
            super::terrain::metadata(world, index % width, index / width)
        }
        Instance::Entity(id) => id,
    };
    world.entity_mut(id).expect("live SetupWalls receiver")
}

fn directions(world: &mut World, receiver: Receiver, sides: [bool; 4]) {
    let entity = properties(world, receiver);
    for (name, present) in ["up", "down", "left", "right"].into_iter().zip(sides) {
        entity
            .data
            .insert(name.into(), f64::from(u8::from(present)));
    }
}

fn sprite(world: &mut World, receiver: Receiver, next: &str) {
    let entity = properties(world, receiver);
    match receiver.instance {
        Instance::Tile(_) => {
            let current = entity.text.entry("terrain_sprite".into()).or_default();
            crate::animation::assign(current, &mut entity.animation, next);
        }
        Instance::Entity(_) => entity.set_sprite(next),
    }
}

pub(super) fn vines(world: &mut World) {
    for receiver in receivers(world, "oVine") {
        if !live(world, receiver, "oVine") {
            continue;
        }
        let p = receiver.origin;
        let up = point(world, Vec2::new(p.x + 8.0, p.y - 8.0), "oLadder");
        let down = point(world, Vec2::new(p.x + 8.0, p.y + 16.0), "oLadder");
        directions(world, receiver, [up, down, false, false]);
        if !up {
            sprite(world, receiver, "sVineSource");
        } else if !down {
            sprite(world, receiver, "sVineBottom");
        }
    }
}

pub(super) fn trees_and_branches(world: &mut World, rng: &mut Rng, cemetery: bool) {
    for receiver in receivers(world, "oTree") {
        if !live(world, receiver, "oTree") {
            continue;
        }
        let p = receiver.origin;
        let sides = [
            point(world, Vec2::new(p.x, p.y - 16.0), "oTree"),
            point(world, Vec2::new(p.x, p.y + 16.0), "oTree"),
            point(world, Vec2::new(p.x - 16.0, p.y), "oTree"),
            point(world, Vec2::new(p.x + 16.0, p.y), "oTree"),
        ];
        directions(world, receiver, sides);
        if !sides[0] {
            sprite(
                world,
                receiver,
                if cemetery { "sTreeTopDead" } else { "sTreeTop" },
            );
            properties(world, receiver).data.insert("depth".into(), 1.0);
        }
    }
    for receiver in receivers(world, "oTreeBranch") {
        if !live(world, receiver, "oTreeBranch") {
            continue;
        }
        let p = receiver.origin;
        let sides = [
            point(world, Vec2::new(p.x, p.y - 16.0), "oLeaves"),
            point(world, Vec2::new(p.x, p.y + 16.0), "oTreeBranch"),
            point(world, Vec2::new(p.x - 16.0, p.y), "oTreeBranch"),
            point(world, Vec2::new(p.x + 16.0, p.y), "oTree"),
        ];
        directions(world, receiver, sides);
        if sides[0] {
            // Own Destroy runs synchronously, before the later left-sprite
            // assignment. It reads the old sprite; dead branches emit nothing.
            let old_sprite = match receiver.instance {
                Instance::Tile(index) => {
                    let override_sprite = properties(world, receiver)
                        .text
                        .get("terrain_sprite")
                        .cloned();
                    override_sprite.unwrap_or_else(|| {
                        catalog::sprite(&world.tiles[index].object)
                            .unwrap_or("")
                            .into()
                    })
                }
                Instance::Entity(id) => world.entity(id).unwrap().sprite.clone(),
            };
            if !matches!(old_sprite.as_str(), "sTreeBranchDeadL" | "sTreeBranchDeadR") {
                let x = p.x + 8.0 + f64::from(rng.range(9)) - f64::from(rng.range(9));
                let y = p.y + 8.0 + f64::from(rng.range(9)) - f64::from(rng.range(9));
                crate::creation::spawn_generated_entity(world, rng, "oLeaf", Vec2::new(x, y));
            }
            match receiver.instance {
                Instance::Tile(index) => {
                    let id = properties(world, receiver).id;
                    let dirty = world.fluid_dirty;
                    world.set_tile(index % world.width, index / world.width, "");
                    world.fluid_dirty = dirty;
                    world.entity_mut(id).unwrap().alive = false;
                }
                Instance::Entity(id) => world.entity_mut(id).unwrap().alive = false,
            }
            // Source continues the script after destruction, but its remaining
            // sprite write targets the deleted receiver and cannot affect queries.
        } else if sides[3] {
            sprite(
                world,
                receiver,
                if cemetery {
                    "sTreeBranchDeadL"
                } else {
                    "sTreeBranchLeft"
                },
            );
        }
    }
}

#[cfg(test)]
#[path = "canopy_tests.rs"]
mod tests;
