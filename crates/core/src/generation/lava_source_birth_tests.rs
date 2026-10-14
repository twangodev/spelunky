//! Cold source Lava birth prefix only. Earlier default generation / numeric
//! query geometry / all-phase family streams are not certified by this test.
use super::*;
use crate::Area;
use std::cell::RefCell;

pub(in crate::generation) struct Before {
    rng: u64,
    next_id: u64,
    serial: u64,
    origin: Vec2,
    source: bool,
}
struct Birth {
    before: Before,
    parent: Entity,
    after: u64,
    live_lava_at_return: usize,
}
thread_local! {
    static TRACE: RefCell<Option<Vec<Birth>>> = const { RefCell::new(None) };
}
pub(in crate::generation) fn before_birth(
    world: &World,
    rng: &Rng,
    origin: Vec2,
    source: bool,
) -> Option<Before> {
    TRACE.with(|trace| {
        trace.borrow().is_some().then(|| Before {
            rng: rng.state_bits(),
            next_id: world.next_id,
            serial: world.creation_serial,
            origin,
            source,
        })
    })
}
pub(in crate::generation) fn after_birth(
    world: &World,
    rng: &Rng,
    before: Option<Before>,
    id: u64,
) {
    if let Some(before) = before {
        let parent = world
            .entity(id)
            .expect("actual returned Lava parent")
            .clone();
        let live_lava_at_return = world
            .entities
            .iter()
            .filter(|e| e.alive && e.object == "oLava")
            .count();
        TRACE.with(|trace| {
            trace.borrow_mut().as_mut().unwrap().push(Birth {
                before,
                parent,
                after: rng.state_bits(),
                live_lava_at_return,
            })
        });
    }
}
fn begin() {
    TRACE.with(|trace| {
        assert!(trace.borrow().is_none());
        *trace.borrow_mut() = Some(Vec::new());
    });
}
fn finish() -> Vec<Birth> {
    TRACE.with(|trace| trace.borrow_mut().take().unwrap())
}
fn literal_rand(state: &mut u32, width: u32) -> u32 {
    *state = state.wrapping_mul(0x0808_8405).wrapping_add(1);
    ((u64::from(*state) * u64::from(width)) >> 32) as u32
}
#[test]
fn stock_seed_zero_temple_lava_finishes_create_before_next_source_cell() {
    // One fixed ordinary stock case, chosen before any execution. No forced
    // Lava template, seed search, actor insertion, RNG splice or rescue.
    begin();
    let mut rng = Rng::new(0);
    let world = crate::generation::generate(&mut rng, 13, &Default::default());
    let births = finish();
    assert_eq!(world.area, Area::Temple);
    assert_eq!(world.level, 13);
    assert!(
        !births.is_empty(),
        "the fixed stock case must naturally reach Lava; never change seed to repair this premise"
    );
    let first = &births[0];
    assert!(first.before.source);
    assert_eq!(first.live_lava_at_return, 1);
    assert_eq!(first.parent.object, "oLava");
    assert_eq!(first.parent.id, first.before.next_id);
    assert_eq!(first.parent.creation_serial, first.before.serial + 1);
    assert_eq!(first.parent.pos, first.before.origin);
    assert_eq!(first.parent.previous_pos, first.before.origin);
    assert_eq!(first.parent.sprite, "sLava");
    assert!(first.parent.engine_active);
    assert_eq!(first.parent.animation, 0.);
    let mut state = u32::try_from(first.before.rng).unwrap();
    let expected_time = 100 + literal_rand(&mut state, 201);
    eprintln!(
        "STOCK_LAVA_BIRTH seed=0 level=13 id={} serial={} origin={:?} incoming={} expected_after={state} actual_after={} expected_spurt_time={expected_time} actual_spurt_time={:?} type_at_return={:?} lava_initialized={}",
        first.parent.id,
        first.parent.creation_serial,
        first.parent.pos,
        first.before.rng,
        first.after,
        first.parent.data.get("spurt_time"),
        first.parent.text.get("type"),
        first.parent.value("lava_initialized")
    );
    assert_eq!(
        first.after,
        u64::from(state),
        "own Lava Create rand(100,300) must complete at birth, before the next source room cell"
    );
    assert_eq!(
        first.parent.text.get("type").map(String::as_str),
        Some("Lava")
    );
    assert_eq!(first.parent.data.get("spurt"), Some(&0.));
    assert_eq!(first.parent.data.get("randNum"), Some(&201.));
    assert_eq!(
        first.parent.data.get("spurt_time"),
        Some(&f64::from(expected_time))
    );
    assert_eq!(
        first.parent.data.get("spurt_counter"),
        Some(&f64::from(expected_time))
    );
    assert_eq!(first.parent.value("lava_initialized"), 1.);
    assert_eq!(first.parent.value("animation_speed"), 0.4);
}
#[test]
fn source_lava_constructor_is_complete_before_caller_and_late_create_cannot_reroll() {
    for seed in [0, 1, 42] {
        let mut world = World::new(8, 8, Area::Temple, 13);
        let origin = Vec2::new(32., 32.);
        let id = world.spawn("oLava", origin);
        let serial = world.entity(id).unwrap().creation_serial;
        let mut actual = Rng::new(seed);
        let mut state = seed;
        let time = 100 + literal_rand(&mut state, 201);
        create_source_room_lava(&mut world, &mut actual, id);
        let parent = world.entity(id).unwrap();
        assert_eq!(actual.state_bits(), u64::from(state));
        assert_eq!(
            (parent.id, parent.creation_serial, parent.pos),
            (id, serial, origin)
        );
        assert_eq!(parent.text.get("type").map(String::as_str), Some("Lava"));
        assert_eq!(parent.value("randNum"), 201.);
        assert_eq!(parent.value("spurt"), 0.);
        assert_eq!(parent.value("spurt_time"), f64::from(time));
        assert_eq!(parent.value("spurt_counter"), f64::from(time));
        assert_eq!(parent.value("animation_speed"), 0.4);
        let before = actual.state_bits();
        create_generated(&mut world, &mut actual);
        assert_eq!(
            actual.state_bits(),
            before,
            "late generation pass cannot repeat completed Create"
        );
        assert_eq!(
            world.entity(id).unwrap().value("spurt_time"),
            f64::from(time)
        );
    }
}
#[test]
fn historical_none_room_context_keeps_deferred_lava_create_tape() {
    for mut actual in [Rng::new(42), Rng::legacy(42)] {
        let mut world = World::new(8, 8, Area::Temple, 13);
        let before = actual.state_bits();
        begin();
        super::super::tiles::stamp_at(&mut world, &mut actual, "w", 2, 2, false, false, "General");
        let births = finish();
        assert_eq!(births.len(), 1);
        assert!(!births[0].before.source);
        assert_eq!(births[0].before.rng, before);
        assert_eq!(births[0].after, before);
        assert!(!births[0].parent.data.contains_key("spurt_time"));
        assert_eq!(births[0].parent.value("lava_initialized"), 0.);
        let mut expected = actual.clone();
        let time = 100 + expected.range(201);
        create_generated(&mut world, &mut actual);
        assert_eq!(actual.state_bits(), expected.state_bits());
        assert_eq!(
            world
                .entity(births[0].parent.id)
                .unwrap()
                .value("spurt_time"),
            f64::from(time)
        );
    }
}

#[test]
fn both_source_room_callers_preserve_predicate_then_create_then_next_cell_order() {
    let mut world = World::new(8, 8, Area::Temple, 13);
    let mut actual = Rng::new(0);
    let mut state = 0_u32;
    // Original scrRoomGen4's elseif chain reaches the candidate tile2
    // predicate before tile3: its rand(1,2) is evaluated even for symbol3.
    literal_rand(&mut state, 2);
    // The selected tile3 body has its separate Lava/Temple selection roll.
    assert_eq!(1 + literal_rand(&mut state, 2), 1);
    let first_incoming = state;
    let first_time = 100 + literal_rand(&mut state, 201);
    let first_outgoing = state;
    // The next tilew reaches false-symbol tile2, tile7 and tile4 predicates
    // before its unconditional Lava birth. Each reached random operand runs.
    literal_rand(&mut state, 2);
    literal_rand(&mut state, 3);
    literal_rand(&mut state, 4);
    let second_incoming = state;
    let second_time = 100 + literal_rand(&mut state, 201);
    begin();
    super::super::tiles::stamp_at_source_context(
        &mut world,
        &mut actual,
        "3w",
        2,
        2,
        false,
        false,
        "General",
        false,
        crate::generation::GenerationContext::default(),
    );
    let births = finish();
    assert_eq!(births.len(), 2, "both literal cells must produce real Lava");
    assert_eq!(births[0].before.rng, u64::from(first_incoming));
    assert_eq!(births[0].after, u64::from(first_outgoing));
    assert_eq!(births[1].before.rng, u64::from(second_incoming));
    assert_eq!(births[1].after, u64::from(state));
    assert_eq!(actual.state_bits(), u64::from(state));
    for (index, (birth, time)) in births.iter().zip([first_time, second_time]).enumerate() {
        assert!(birth.before.source);
        assert_eq!(birth.live_lava_at_return, index + 1);
        assert_eq!(birth.parent.id, birth.before.next_id);
        assert_eq!(birth.parent.creation_serial, birth.before.serial + 1);
        assert_eq!(birth.parent.pos, Vec2::new((32 + index * 16) as f64, 32.));
        assert_eq!(birth.parent.value("spurt_time"), f64::from(time));
        assert_eq!(birth.parent.value("spurt_counter"), f64::from(time));
        assert_eq!(
            birth.parent.text.get("type").map(String::as_str),
            Some("Lava")
        );
        assert_eq!(birth.parent.value("lava_initialized"), 1.);
    }
    create_generated(&mut world, &mut actual);
    assert_eq!(
        actual.state_bits(),
        u64::from(state),
        "no later Lava rerolls"
    );
}

#[test]
fn historical_random_lava_branch_keeps_predicate_and_deferred_create_tape() {
    for mut actual in [Rng::new(0)] {
        let mut expected = actual.clone();
        assert!(expected.chance(1, 2), "fixed historical branch premise");
        let predicate_after = expected.state_bits();
        let mut world = World::new(8, 8, Area::Temple, 13);
        begin();
        super::super::tiles::stamp_at(&mut world, &mut actual, "3w", 2, 2, false, false, "General");
        let births = finish();
        assert_eq!(births.len(), 2);
        assert_eq!(actual.state_bits(), predicate_after);
        for birth in &births {
            assert!(!birth.before.source);
            assert_eq!(birth.before.rng, predicate_after);
            assert_eq!(birth.after, predicate_after);
            assert!(!birth.parent.data.contains_key("spurt_time"));
            assert_eq!(birth.parent.value("lava_initialized"), 0.);
        }
        let times = [100 + expected.range(201), 100 + expected.range(201)];
        create_generated(&mut world, &mut actual);
        assert_eq!(actual.state_bits(), expected.state_bits());
        for (birth, time) in births.iter().zip(times) {
            assert_eq!(
                world.entity(birth.parent.id).unwrap().value("spurt_time"),
                f64::from(time)
            );
        }
    }
}
