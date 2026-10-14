//! Shared entity-local Create code for generated and runtime instances.
//!
//! This context deliberately covers only Create bodies whose dependencies are
//! the instance, the destination world, and the gameplay RNG. Player/global
//! state, sounds, and other Game-dependent effects remain in runtime Create,
//! except the explicit Spectacles snapshot for terrain's RNG-free item children.
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
    spawn_generated_with_item_context(world, rng, object, pos, None, false)
}

/// Complete the selected RNG-free item Create with actual generation globals
/// before its producer overwrites Underground-set price/sale fields.
pub(crate) fn spawn_generated_terrain_child(
    world: &mut World,
    rng: &mut Rng,
    object: &str,
    pos: Vec2,
    has_spectacles: bool,
) -> u64 {
    spawn_generated_with_item_context(world, rng, object, pos, Some(has_spectacles), false)
}

/// Versioned cemetery children extend the finite native constructor set by
/// Gold Nugget without changing historical runtime/generation state bytes.
pub(crate) fn spawn_generated_jungle_child(
    world: &mut World,
    rng: &mut Rng,
    object: &str,
    pos: Vec2,
    has_spectacles: bool,
) -> u64 {
    spawn_generated_with_item_context(world, rng, object, pos, Some(has_spectacles), true)
}

fn spawn_generated_with_item_context(
    world: &mut World,
    rng: &mut Rng,
    object: &str,
    pos: Vec2,
    has_spectacles: Option<bool>,
    source_jungle: bool,
) -> u64 {
    let id = world.spawn(object, pos);
    #[cfg(test)]
    let rng_before =
        GENERATED_CREATE_TRACES.with(|traces| traces.borrow().as_ref().map(|_| rng.clone()));
    initialize_entity_local(world, rng, id);
    if let Some(has_spectacles) = has_spectacles {
        if source_jungle {
            let entity = world.entity_mut(id).unwrap();
            crate::items::initialize_source_jungle_child(entity, has_spectacles);
            crate::traps::initialize_source_jungle_spear(entity);
            initialize_source_jungle_market(entity);
        } else {
            crate::items::initialize_terrain_child(world.entity_mut(id).unwrap(), has_spectacles);
        }
    }
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
                    after_cemetery_prepass: None,
                });
            }
        });
    }
    id
}

/// oXMarket inherits the RNG-free oExit Create, then changes its type.
/// Shared with runtime Create; producer writes must survive bootstrap reentry.
pub(crate) fn initialize_source_jungle_market(entity: &mut crate::Entity) {
    if entity.object == "oXMarket" && entity.value("source_jungle_market_create_initialized") == 0.0
    {
        entity.text.insert("leadsTo".into(), String::new());
        entity.text.insert("type".into(), "Market Exit".into());
        entity
            .data
            .insert("source_jungle_market_create_initialized".into(), 1.0);
    }
}

pub(crate) fn initialize_entity_local(world: &mut World, rng: &mut Rng, id: u64) {
    let Some(index) = world.entities.iter().position(|e| e.id == id && e.alive) else {
        return;
    };
    // Runtime Create likewise detaches the receiver while its body queries the
    // world. The local context cannot mutate storage, so the original slot is
    // still authoritative when the body returns.
    let mut entity = world.entities.remove(index);
    if entity.object == "oFrozenCaveman" && entity.value("frozen_create_initialized") == 0.0 {
        // No own Create: it inherits oDrawnSprite's empty type and blinkToggle.
        // Its live Step/Destroy receiver is created at the Ice parent's origin.
        entity.text.insert("type".into(), String::new());
        entity.data.insert("blinkToggle".into(), 0.0);
        entity.data.insert("frozen_create_initialized".into(), 1.0);
    }
    let mut context = EntityCreateContext { world, rng };
    crate::enemies::create_local(&mut context, &mut entity);
    crate::traps::create_local(&mut context, &mut entity);
    crate::world_events::create_local(&mut context, &mut entity);
    world.entities.insert(index, entity);
}

#[cfg(test)]
pub(crate) struct GeneratedCreateTrace {
    pub level: u32,
    pub rng_before: Rng,
    pub entity_after: Entity,
    pub after_cemetery_prepass: Option<Entity>,
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
pub(crate) fn capture_generated_cemetery_prepass(world: &World) {
    GENERATED_CREATE_TRACES.with(|traces| {
        if let Some(traces) = traces.borrow_mut().as_mut() {
            for trace in traces
                .iter_mut()
                .filter(|trace| trace.entity_after.object == "oGrave")
            {
                trace.after_cemetery_prepass = world.entity(trace.entity_after.id).cloned();
            }
        }
    });
}

#[cfg(test)]
pub(crate) fn take_generated_create_traces() -> Vec<GeneratedCreateTrace> {
    GENERATED_CREATE_TRACES.with(|traces| traces.borrow_mut().take().unwrap_or_default())
}

#[cfg(test)]
mod grave_create_tests {
    use super::*;
    use crate::{Game, InputFrame, RulesPreset};
    use std::collections::BTreeSet;

    // Independent transcription of oGrave's five explicit source switch arms.
    // The oracle uses the port RNG; it does not establish GM8 seed equivalence.
    const SOURCE_GRAVE_SPRITES: [&str; 5] = ["sGrave2", "sGrave3", "sGrave4", "sGrave5", "sGrave6"];

    #[test]
    fn grave_create_custom_game_birth_has_all_five_variants_and_does_not_reroll() {
        let mut variants = BTreeSet::new();
        for seed in 0..64 {
            let mut game = Game::shell(seed, RulesPreset::Classic);
            // Grave has no Classic custom tile symbol. Exercise an actual
            // runtime birth in a loaded custom room through the public API.
            let mut level = crate::level::Level::default();
            level.set_tile(7, 4, '1').unwrap(); // Support at source origin (128, 80).
            game.load_custom(&level, "grave-create.lvl", "");
            let mut source = game.rng.clone();
            let expected = SOURCE_GRAVE_SPRITES[source.range(5) as usize];
            let id = game.spawn("oGrave", Vec2::new(128.0, 64.0));
            let grave = game.world.entity(id).unwrap();
            assert_eq!(grave.sprite, expected, "seed {seed}");
            assert_eq!(grave.value("invincible"), 0.0);
            assert_eq!(grave.value("shopWall"), 0.0);
            assert_eq!(grave.value("cleanDeath"), 0.0);
            assert_eq!(grave.value("blinkToggle"), 0.0);
            assert_eq!(grave.text.get("type").map(String::as_str), Some("None"));
            assert_eq!(grave.text.get("treasure").map(String::as_str), Some(""));
            variants.insert(grave.sprite.clone());
            assert_eq!(game.rng.clone().next_u64(), source.clone().next_u64());

            // The cemetery caller can replace the variant after Create. Shared
            // initialization and later real ticks must preserve that assignment.
            let grave = game.world.entity_mut(id).unwrap();
            grave.set_sprite("sGraveAsh");
            grave.data.insert("invincible".into(), 1.0);
            grave.text.insert("treasure".into(), "Big Ruby".into());
            for _ in 0..2 {
                initialize_entity_local(&mut game.world, &mut game.rng, id);
                crate::world_events::create_instance(&mut game, id);
            }
            for _ in 0..3 {
                game.step(&InputFrame::default());
            }
            let grave = game.world.entity(id).unwrap();
            assert_eq!(grave.sprite, "sGraveAsh");
            assert_eq!(grave.value("invincible"), 1.0);
            assert_eq!(
                grave.text.get("treasure").map(String::as_str),
                Some("Big Ruby")
            );
            assert_eq!(game.rng.next_u64(), source.next_u64(), "seed {seed}");
        }
        assert_eq!(
            variants,
            SOURCE_GRAVE_SPRITES.into_iter().map(String::from).collect()
        );
    }

    #[test]
    fn grave_create_generated_cemetery_variants_precede_caller_shotgun_roll() {
        let mut graves = 0;
        let mut survivors = 0;
        let mut retired = 0;
        for seed in 0..8 {
            let mut game = Game::shell(seed, RulesPreset::Classic);
            assert_eq!(
                game.generation_revision,
                crate::GenerationRevision::SourceClassicV2
            );
            game.flags.extend([
                "cemetery".into(),
                "no_dark_level".into(),
                "low_graphics".into(),
            ]);
            game.data.insert("next_level".into(), 5.0);
            begin_generated_create_trace();
            game.next_level();
            let traces = take_generated_create_traces();
            assert_eq!(game.world.level, 5);
            for trace in traces
                .into_iter()
                .filter(|trace| trace.entity_after.object == "oGrave")
            {
                graves += 1;
                let mut source = trace.rng_before.clone();
                let expected = SOURCE_GRAVE_SPRITES[source.range(5) as usize];
                assert_eq!(trace.entity_after.sprite, expected, "seed {seed}");
                // Check the completed cemetery caller before the later Solid
                // pass can legitimately destroy this Grave or its support.
                let caller = trace
                    .after_cemetery_prepass
                    .expect("Grave survives its cemetery caller");
                assert!(caller.alive && caller.engine_active);
                assert_eq!(caller.id, trace.entity_after.id);
                assert_eq!(caller.creation_serial, trace.entity_after.creation_serial);
                assert_eq!(caller.pos, trace.entity_after.pos);
                // The source writes receiver-local ashGrave; global.ashGrave
                // remains false, so every Grave has the immediate next roll.
                let expected_after_caller = if source.range(40) == 0 {
                    "sGraveAsh"
                } else {
                    expected
                };
                assert_eq!(caller.sprite, expected_after_caller, "seed {seed}");
                if let Some(final_grave) = game.world.entity(caller.id) {
                    assert_eq!(final_grave.sprite, caller.sprite, "seed {seed}");
                    survivors += 1;
                } else {
                    retired += 1;
                }
            }
        }
        assert!(graves > 0, "exercise natural generated cemetery births");
        assert!(
            survivors > 0,
            "check variants retained through later phases"
        );
        assert!(
            retired > 0,
            "exercise the later source Solid-pass lifecycle"
        );
    }
}
