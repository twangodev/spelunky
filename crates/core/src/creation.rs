//! Shared entity-local Create code for generated and runtime instances.
//!
//! This context deliberately covers only Create bodies whose dependencies are
//! the instance, the destination world, and the gameplay RNG. Player/global
//! state, sounds, and other Game-dependent effects remain in runtime Create.
#[cfg(test)]
use crate::Entity;
use crate::{Rng, Vec2, World};

pub(crate) struct EntityCreateContext<'a> {
    pub world: &'a World,
    pub rng: &'a mut Rng,
}

/// Fixed defaults, then the shared local Create body, then producer overrides.
pub(crate) fn spawn_generated_entity(
    world: &mut World,
    rng: &mut Rng,
    object: &str,
    pos: Vec2,
) -> u64 {
    let id = world.spawn(object, pos);
    #[cfg(test)]
    let rng_before =
        GENERATED_CREATE_TRACES.with(|traces| traces.borrow().as_ref().map(|_| rng.clone()));
    initialize_entity_local(world, rng, id);
    #[cfg(test)]
    if let Some(rng_before) = rng_before
        && let Some(entity) = world.entity(id)
    {
        GENERATED_CREATE_TRACES.with(|traces| {
            if let Some(traces) = traces.borrow_mut().as_mut() {
                traces.push(GeneratedCreateTrace {
                    level: world.level,
                    rng_before,
                    entity_after: entity.clone(),
                });
            }
        });
    }
    id
}

pub(crate) fn initialize_entity_local(world: &mut World, rng: &mut Rng, id: u64) {
    let Some(index) = world.entities.iter().position(|e| e.id == id && e.alive) else {
        return;
    };
    // Runtime Create likewise detaches the receiver while its body queries the
    // world. The local context cannot mutate storage, so the original slot is
    // still authoritative when the body returns.
    let mut entity = world.entities.remove(index);
    crate::enemies::create_local(&mut EntityCreateContext { world, rng }, &mut entity);
    world.entities.insert(index, entity);
}

#[cfg(test)]
pub(crate) struct GeneratedCreateTrace {
    pub level: u32,
    pub rng_before: Rng,
    pub entity_after: Entity,
}

#[cfg(test)]
thread_local! {
    static GENERATED_CREATE_TRACES: std::cell::RefCell<Option<Vec<GeneratedCreateTrace>>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
pub(crate) fn begin_generated_create_trace() {
    GENERATED_CREATE_TRACES.with(|traces| *traces.borrow_mut() = Some(Vec::new()));
}

#[cfg(test)]
pub(crate) fn take_generated_create_traces() -> Vec<GeneratedCreateTrace> {
    GENERATED_CREATE_TRACES.with(|traces| traces.borrow_mut().take().unwrap_or_default())
}
