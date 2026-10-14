//! Complete controlled Lush-ground event path; source selectors choose
//! constructor inputs before production, never on a generated route outcome.
use super::super::super::{GenerationContext, tiles};
use super::*;

// Independent literal source rand arithmetic and oLush/Create decision order.
// Fixture seeds are selected on constructor inputs, never generated outcomes.
fn literal_range(state: &mut u32, width: u32) -> u32 {
    *state = state.wrapping_mul(0x0808_8405).wrapping_add(1);
    ((u64::from(*state) * u64::from(width)) >> 32) as u32
}
fn literal_lush_create(state: &mut u32) -> (u32, Option<(&'static str, f64, f64)>) {
    let n = 1 + literal_range(state, 100);
    let child = if n < 30 {
        None
    } else if literal_range(state, 80) == 0 {
        Some(("oSapphireBig", 0., 0.))
    } else if literal_range(state, 100) == 0 {
        Some(("oEmeraldBig", 0., 0.))
    } else if literal_range(state, 120) == 0 {
        Some(("oRubyBig", 0., 0.))
    } else if literal_range(state, 1200) == 0 {
        Some(SOURCE_SET2[literal_range(state, 19) as usize])
    } else {
        None
    };
    (n, child)
}

#[test]
fn source_ground_lush_all_literal_children_finish_before_next_glyph_and_no_late_replay() {
    let seeds = fixtures(false);
    assert_eq!(seeds.len(), 22);
    for has_spectacles in [false, true] {
        for (object, seed) in &seeds {
            let context = GenerationContext { has_spectacles };
            let mut expected_state = *seed;
            let (n, selected) = literal_lush_create(&mut expected_state);
            let (selected_object, dx, dy) = selected.unwrap();
            assert_eq!(selected_object, *object);
            // Own children are RNG-free. Source then reaches the independent
            // tail's random7 predicate, and the next L glyph's random2 test.
            literal_range(&mut expected_state, 3);
            literal_range(&mut expected_state, 2);
            let mut world = World::new(20, 16, Area::Jungle, 5);
            world.name = "Jungle".into();
            let mut actual_rng = Rng::new(*seed);
            crate::creation::begin_generated_create_trace();
            tiles::stamp_at_source_context(
                &mut world,
                &mut actual_rng,
                "1L",
                6,
                6,
                false,
                false,
                "General",
                false,
                context,
            );
            let constructors = crate::creation::take_generated_create_traces();
            assert_eq!(constructors.len(), 1, "{object}/{has_spectacles}");
            assert_eq!(constructors[0].entity_after.object, *object);
            let child = world.entity(constructors[0].entity_after.id).unwrap();
            assert_eq!(child.pos, Vec2::new(104. + dx, 104. + dy));
            assert!(
                child.creation_serial < world.tile(7, 6).unwrap().creation_serial,
                "real child before next glyph"
            );
            assert_eq!(world.tile(7, 6).unwrap().object, "oVine");
            assert_eq!(child.age, 0);
            assert_eq!(child.vel, Vec2::default());
            assert_eq!(child.value("source_pickup_create_initialized"), 1.);
            assert_eq!(child.value("myGrav"), 0.6);
            assert_eq!(child.value("xVel"), 0.);
            assert_eq!(child.value("yVel"), 0.);
            if crate::catalog::is_a(object, "oItem") {
                assert_eq!(child.value("cost"), 0.);
                assert_eq!(child.value("forSale"), 0.);
                assert_eq!(
                    child.value("depth"),
                    if has_spectacles { 51. } else { 101. }
                );
                assert!(
                    constructors[0].entity_after.value("cost") > 0.,
                    "Set2 override follows constructor"
                );
                assert_eq!(
                    constructors[0].entity_after.value("depth"),
                    child.value("depth")
                );
                assert_eq!(child.value("canPickUp"), 1.);
            } else {
                assert_eq!(child.state, 1);
                assert_eq!(child.alarms[0], 20);
                assert_eq!(child.value("canCollect"), 0.);
                assert_eq!(
                    child.value("value"),
                    match *object {
                        "oSapphireBig" => 1200.,
                        "oEmeraldBig" => 800.,
                        "oRubyBig" => 1600.,
                        _ => unreachable!(),
                    }
                );
            }
            let parent = world
                .entities
                .iter()
                .find(|e| e.alive && e.object == "oTerrainMetadata")
                .unwrap();
            assert_eq!(parent.pos, Vec2::new(104., 104.));
            assert_eq!(parent.value("n"), f64::from(n));
            assert_eq!(parent.value("terrain_initialized"), 1.);
            assert_eq!(parent.value("source_ground_lush_created"), 1.);
            assert_eq!(parent.value("embedded_treasure"), 0.);
            assert_eq!(parent.value("invincible"), 0.);
            assert_eq!(parent.value("shopWall"), 0.);
            assert_eq!(parent.value("cleanDeath"), 0.);
            assert_eq!(parent.text.get("treasure").map(String::as_str), Some(""));
            assert_eq!(actual_rng.state_bits(), u64::from(expected_state));
            let id = child.id;
            world
                .entity_mut(id)
                .unwrap()
                .data
                .insert("constructor_replay_probe".into(), 17.);
            let before = serde_json::to_vec(&world).unwrap();
            let rng_before = actual_rng.state_bits();
            create_generated_with_context(&mut world, &mut actual_rng, context);
            assert_eq!(
                actual_rng.state_bits(),
                rng_before,
                "late phase must not charge Create twice"
            );
            assert_eq!(
                serde_json::to_vec(&world).unwrap(),
                before,
                "child identity/state and parent unchanged"
            );
        }
    }
}

#[test]
fn source_ground_lush_gold_and_conditional_two_complete_at_actual_branch() {
    for name in ["Jungle", "Black Market"] {
        for glyph in ["1", "2"] {
            let mut world = World::new(20, 16, Area::Jungle, 5);
            world.name = name.into();
            let mut expected = 0_u32;
            if glyph == "2" {
                assert_eq!(literal_range(&mut expected, 2), 0);
            }
            let (n, child) = literal_lush_create(&mut expected);
            assert!(child.is_none());
            assert!(
                n < 30,
                "fixed literal prefix gives a gold constructor, no seed search"
            );
            literal_range(&mut expected, 3);
            let mut actual = Rng::new(0);
            tiles::stamp_at_source_context(
                &mut world,
                &mut actual,
                glyph,
                6,
                6,
                false,
                false,
                "General",
                false,
                GenerationContext::default(),
            );
            assert_eq!(actual.state_bits(), u64::from(expected));
            let parent = world
                .entities
                .iter()
                .find(|e| e.alive && e.object == "oTerrainMetadata")
                .unwrap();
            assert_eq!(parent.value("n"), f64::from(n));
            assert_eq!(
                parent.text.get("terrain_sprite").map(String::as_str),
                Some(if n < 20 { "sLushGold" } else { "sLushGoldBig" })
            );
            assert_eq!(parent.value("gold_chunks"), 3.);
            assert_eq!(parent.value("gold_nuggets"), if n < 20 { 0. } else { 1. });
            assert_eq!(world.entities.iter().filter(|e| e.alive).count(), 1);
        }
    }
}

#[test]
fn historical_ground_lush_retains_deferred_constructor_and_old_two_guard_tapes() {
    for canonical in [false, true] {
        for glyph in ["1", "2"] {
            let mut world = World::new(20, 16, Area::Jungle, 5);
            let mut actual = if canonical {
                Rng::new(0)
            } else {
                Rng::legacy(0)
            };
            let mut expected = actual.clone();
            let placed = glyph == "1" || expected.chance(1, 2);
            tiles::stamp_at(
                &mut world,
                &mut actual,
                glyph,
                6,
                6,
                false,
                false,
                "General",
            );
            assert_eq!(actual.state_bits(), expected.state_bits());
            assert_eq!(
                world.tile(6, 6).unwrap().object,
                if placed { "oLush" } else { "" }
            );
            assert!(
                world.entities.is_empty(),
                "historical does not enter synchronous source Create"
            );
        }
    }
}
