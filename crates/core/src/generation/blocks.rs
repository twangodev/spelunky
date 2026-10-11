//! The oBlock family pass in scrSetupWalls:458–472.
use crate::{CollisionPrecision, Entity, Rng, Vec2, World, catalog};

#[derive(Clone, Copy)]
enum Instance {
    Tile(usize),
    Entity(u64),
}

struct Receiver {
    instance: Instance,
    serial: u64,
    source: Entity,
    metadata: Option<u64>,
}

fn point(world: &World, origin: Vec2, family: &str) -> bool {
    world.tiles_point(origin, family)
        || world.entities.iter().any(|entity| {
            entity.alive
                && catalog::is_a(&entity.object, family)
                && entity.mask_contains(origin, CollisionPrecision::BoundingBox)
        })
}

fn receivers(world: &World) -> Vec<(u64, Instance)> {
    let mut instances: Vec<_> = world
        .tiles
        .iter()
        .enumerate()
        .filter(|(_, tile)| tile.object == "oBlock")
        .map(|(index, tile)| (tile.creation_serial, Instance::Tile(index)))
        .chain(
            world
                .entities
                .iter()
                .filter(|entity| entity.alive && entity.object == "oBlock")
                .map(|entity| (entity.creation_serial, Instance::Entity(entity.id))),
        )
        .collect();
    // Canonical oBlock has no descendants. Generation's family arrays append
    // births across storage forms; no body here creates another Block, changes
    // activation, or deletes any other Block. Identity snapshots preserve that
    // bounded traversal, without claiming earlier port births match the source.
    instances.sort_by_key(|(serial, _)| *serial);
    instances
}

fn receiver(world: &World, serial: u64, instance: Instance) -> Option<Receiver> {
    let (source, metadata) = match instance {
        Instance::Tile(index) => {
            let tile = world.tiles.get(index)?;
            if tile.object != "oBlock" || tile.creation_serial != serial {
                return None;
            }
            let origin = Vec2::new(
                (index % world.width) as f64 * 16.0,
                (index / world.width) as f64 * 16.0,
            );
            let metadata = world.entities.iter().find(|entity| {
                entity.alive
                    && entity.object == "oTerrainMetadata"
                    && entity.pos == Vec2::new(origin.x + 8.0, origin.y + 8.0)
            });
            let mut source = Entity::new(0, "oBlock", origin);
            source.creation_serial = serial;
            if let Some(metadata) = metadata {
                source.data = metadata.data.clone();
                source.text = metadata.text.clone();
            }
            (source, metadata.map(|entity| entity.id))
        }
        Instance::Entity(id) => {
            let entity = world.entity(id)?;
            if entity.object != "oBlock" || entity.creation_serial != serial {
                return None;
            }
            (entity.clone(), None)
        }
    };
    Some(Receiver {
        instance,
        serial,
        source,
        metadata,
    })
}

fn retire(world: &mut World, rng: &mut Rng, receiver: Receiver) {
    if self::receiver(world, receiver.serial, receiver.instance).is_none() {
        return;
    }
    // The new PushBlock is already alive during the old Block's own Destroy.
    // This non-inheriting event never writes checkWater or inspects attached
    // fixtures. Preserve cleanDeath from EntityGen's preceding Temple pass.
    let city = world.name == "City of Gold";
    crate::items::explosives::destroy_generated_block_literal(world, rng, &receiver.source, city);
    match receiver.instance {
        Instance::Tile(index) => {
            let was_dirty = world.fluid_dirty;
            world.set_tile(index % world.width, index / world.width, "");
            world.fluid_dirty = was_dirty;
            if let Some(id) = receiver.metadata
                && let Some(metadata) = world.entity_mut(id)
            {
                metadata.alive = false;
            }
        }
        Instance::Entity(id) => {
            if let Some(entity) = world.entity_mut(id) {
                entity.alive = false;
            }
        }
    }
}

pub(super) fn setup(world: &mut World, rng: &mut Rng) {
    #[cfg(test)]
    let trace = TRACE.with(|trace| {
        trace.borrow().as_ref().map(|_| SetupTrace {
            before: world.clone(),
            rng_before: rng.clone(),
            rng_after: rng.clone(),
        })
    });
    for (serial, instance) in receivers(world) {
        let Some(receiver) = receiver(world, serial, instance) else {
            continue;
        };
        let origin = receiver.source.pos;
        let below = Vec2::new(origin.x, origin.y + 16.0);
        let mut down = point(world, below, "oBrick")
            || point(world, below, "oTemple")
            || point(world, below, "oHardBlock");
        if point(world, Vec2::new(origin.x - 16.0, origin.y), "oLava")
            || point(world, Vec2::new(origin.x + 16.0, origin.y), "oLava")
        {
            down = false;
        }
        // Source and/or are eager: every live Block consumes this roll,
        // including unsupported receivers and those beside lava.
        let promotes = rng.range(4) == 0;
        if down && promotes {
            crate::creation::spawn_generated_entity(world, rng, "oPushBlock", origin);
            retire(world, rng, receiver);
        }
    }
    #[cfg(test)]
    if let Some(mut trace) = trace {
        trace.rng_after = rng.clone();
        TRACE.with(|traces| {
            if let Some(traces) = traces.borrow_mut().as_mut() {
                traces.push(trace);
            }
        });
    }
}

#[cfg(test)]
pub(super) struct SetupTrace {
    pub(super) before: World,
    pub(super) rng_before: Rng,
    pub(super) rng_after: Rng,
}

#[cfg(test)]
thread_local! {
    pub(super) static TRACE: std::cell::RefCell<Option<Vec<SetupTrace>>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
#[path = "blocks_tests.rs"]
pub(super) mod tests;
