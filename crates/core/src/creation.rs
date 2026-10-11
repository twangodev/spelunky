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
        let mut first_grave_controls = 0;
        for seed in 0..8 {
            let mut game = Game::shell(seed, RulesPreset::Classic);
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
            let mut shotgun_exists = false;
            let mut first_grave = true;
            for trace in traces {
                if trace.entity_after.object == "oShotgun" {
                    shotgun_exists = true;
                }
                if trace.entity_after.object != "oGrave" {
                    continue;
                }
                graves += 1;
                let mut source = trace.rng_before.clone();
                let expected = SOURCE_GRAVE_SPRITES[source.range(5) as usize];
                assert_eq!(trace.entity_after.sprite, expected, "seed {seed}");
                let final_grave = game.world.entity(trace.entity_after.id).unwrap();
                assert!(final_grave.sprite == expected || final_grave.sprite == "sGraveAsh");
                if first_grave && !shotgun_exists {
                    // scrEntityGen188 creates Grave synchronously; line189's
                    // next rand(1,40) follows its variant roll, before loot.
                    let expected_after_caller = if source.range(40) == 0 {
                        "sGraveAsh"
                    } else {
                        expected
                    };
                    assert_eq!(final_grave.sprite, expected_after_caller, "seed {seed}");
                    first_grave_controls += 1;
                }
                first_grave = false;
            }
        }
        assert!(graves > 0, "exercise natural generated cemetery births");
        assert!(
            first_grave_controls > 0,
            "exercise the source's next shotgun roll"
        );
    }
}
