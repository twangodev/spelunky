//! Fixed incoming generation seed, ordinary stock level/empty flags. No seed
//! search, synthetic cells, forced destination or source RNG replacement.
//! The captured prefix is local to Water Setup; earlier generator parity and
//! complete Game bootstrap/campaign history are expressly not certified.
use super::*;
use crate::Area;
use std::cell::RefCell;

pub(super) struct First {
    before: World,
    rng_before: u64,
    cell: Cell,
    rng_after: u64,
    sprite_after: String,
}
thread_local! {
    static CAPTURE: RefCell<Option<Option<First>>> = const { RefCell::new(None) };
}
pub(super) fn observe_before(world: &World, rng: &Rng, cell: Cell) -> Option<(World, u64, Cell)> {
    CAPTURE.with(|capture| {
        matches!(*capture.borrow(), Some(None)).then(|| (world.clone(), rng.state_bits(), cell))
    })
}
pub(super) fn observe_after(world: &World, rng: &Rng, before: Option<(World, u64, Cell)>) {
    if let Some((before, rng_before, cell)) = before {
        let first = First {
            before,
            rng_before,
            cell,
            rng_after: rng.state_bits(),
            sprite_after: sprite(world, cell).into(),
        };
        CAPTURE.with(|capture| *capture.borrow_mut() = Some(Some(first)));
    }
}
fn draw(state: &mut u32, width: u32) -> u32 {
    *state = state.wrapping_mul(0x0808_8405).wrapping_add(1);
    1 + ((u64::from(*state) * u64::from(width)) >> 32) as u32
}
#[test]
fn stock_seed_zero_lake_first_water_executes_rand_even_when_up_water_is_false() {
    // Preselected seed 0, not chosen by inspecting a generated outcome:
    // literal stock Transition lake chance is rand(1,10), which draws state1
    // and returns1. SourceClassic public generation projects that first roll.
    let mut first_roll_state = 0;
    assert_eq!(draw(&mut first_roll_state, 10), 1);
    assert_eq!(first_roll_state, 1);
    CAPTURE.with(|capture| {
        assert!(capture.borrow().is_none());
        *capture.borrow_mut() = Some(None);
    });
    let mut rng = Rng::new(0);
    let generated = crate::generation::generate(&mut rng, 5, &Default::default());
    let first = CAPTURE.with(|capture| {
        capture
            .borrow_mut()
            .take()
            .unwrap()
            .expect("stock Lake must reach Water Setup")
    });
    assert_eq!(generated.area, Area::Jungle);
    assert_eq!(generated.level, 5);
    assert_eq!(generated.name, "Lake");
    assert_eq!((generated.width, generated.height), (42, 42));
    assert_eq!(first.before.name, "Lake");
    assert!(
        first
            .before
            .tiles
            .iter()
            .all(|t| !catalog::is_a(&t.object, "oWater")),
        "stock room scripts create concrete WaterSwim actors, not water cells"
    );
    let family: Vec<_> = first
        .before
        .entities
        .iter()
        .filter(|e| e.alive && catalog::is_a(&e.object, "oWater"))
        .collect();
    assert!(!family.is_empty());
    assert!(family.iter().all(|e| e.engine_active));
    assert!(
        family
            .windows(2)
            .all(|pair| pair[0].creation_serial < pair[1].creation_serial)
    );
    let Cell::Instance(id) = first.cell else {
        panic!("stock Water receiver must retain actual actor identity")
    };
    assert_eq!(
        family[0].id, id,
        "first captured receiver is earliest actual Water-family birth"
    );
    assert_eq!(family[0].object, "oWaterSwim");
    let p = origin(&first.before, first.cell);
    let up_water = point(&first.before, Vec2::new(p.x, p.y - 16.), "oWater");
    let up = point(&first.before, Vec2::new(p.x, p.y - 16.), "oSolid");
    let down = point(&first.before, Vec2::new(p.x, p.y + 16.), "oSolid")
        && !point(&first.before, Vec2::new(p.x, p.y + 16.), "oWater");
    assert!(
        !up_water,
        "first topmost stock Water receiver premise; never replace the seed to repair this premise"
    );
    assert!(!up);
    // Correct source room routing changes downstream receiver identity and
    // population. Keep this fixed seed and verify the source-local RNG prefix
    // from the actual first receiver rather than old map-specific IDs.
    // Packaged Script212's left-folded AND still evaluates rand last. Its
    // operator may skip RHS TRUTH comparison, not RHS CALL evaluation.
    // False upWater rules out Tall, but Classic still evaluates rand at the
    // end of that left-folded condition before choosing the top sprite.
    let mut expected = u32::try_from(first.rng_before).unwrap();
    let tall_roll = draw(&mut expected, 4);
    let expected_sprite = if (up || up_water) && down {
        match draw(&mut expected, 4) {
            1 => "sWaterBottom",
            2 => "sWaterBottom2",
            3 => "sWaterBottom3",
            _ => "sWaterBottom4",
        }
    } else if !up && !up_water {
        "sWaterTop"
    } else {
        sprite(&first.before, first.cell)
    };
    eprintln!(
        "STOCK_WATER_PREFIX seed=0 level=5 mode=Lake receiver_id={id} serial={} origin={p:?} water_count={} upWater={up_water} up={up} down={down} rng_before={} source_tall_roll={tall_roll} source_rng_after={expected} actual_rng_after={} source_sprite={expected_sprite} actual_sprite={}",
        family[0].creation_serial,
        family.len(),
        first.rng_before,
        first.rng_after,
        first.sprite_after
    );
    assert_eq!(
        first.rng_after,
        u64::from(expected),
        "first Water receiver omits the required source RHS rand; this is a local prefix assertion, not whole-generation parity"
    );
    assert_eq!(first.sprite_after, expected_sprite);
}

#[test]
fn source_top_water_commits_one_rand_before_false_guard_finishes() {
    for seed in [0, 1, 42] {
        let mut world = World::new(8, 8, Area::Jungle, 5);
        let water = world.spawn("oWaterSwim", Vec2::new(32., 32.));
        let mut actual = Rng::new(seed);
        let mut expected = seed;
        draw(&mut expected, 4);
        setup_with_revision(
            &mut world,
            &mut actual,
            crate::GenerationRevision::SourceClassicV2,
        );
        assert_eq!(actual.state_bits(), u64::from(expected));
        assert_eq!(world.entity(water).unwrap().sprite, "sWaterTop");
    }
}
#[test]
fn historical_top_water_retains_no_draw_for_both_rng_algorithms() {
    for mut actual in [Rng::new(42), Rng::legacy(42)] {
        let before = actual.clone();
        let mut world = World::new(8, 8, Area::Jungle, 5);
        let water = world.spawn("oWaterSwim", Vec2::new(32., 32.));
        setup_with_revision(
            &mut world,
            &mut actual,
            crate::GenerationRevision::HistoricalV1,
        );
        assert_eq!(actual.state_bits(), before.state_bits());
        assert_eq!(actual.algorithm(), before.algorithm());
        assert_eq!(world.entity(water).unwrap().sprite, "sWaterTop");
    }
}
#[test]
fn source_two_row_pool_consumes_guard_then_distinct_bottom_variant_roll() {
    for seed in [0, 1, 42] {
        let mut world = World::new(8, 8, Area::Jungle, 5);
        let top = world.spawn("oWaterSwim", Vec2::new(32., 32.));
        let bottom = world.spawn("oWaterSwim", Vec2::new(32., 48.));
        world.set_tile(2, 4, "oLush");
        let mut actual = Rng::new(seed);
        let mut expected = seed;
        draw(&mut expected, 4); // top guard, false upWater
        draw(&mut expected, 4); // bottom guard, false water two rows above
        let variant = draw(&mut expected, 4); // reached else-if has its OWN call
        let expected_sprite = [
            "sWaterBottom",
            "sWaterBottom2",
            "sWaterBottom3",
            "sWaterBottom4",
        ][(variant - 1) as usize];
        setup_with_revision(
            &mut world,
            &mut actual,
            crate::GenerationRevision::SourceClassicV2,
        );
        assert_eq!(actual.state_bits(), u64::from(expected));
        assert_eq!(world.entity(top).unwrap().sprite, "sWaterTop");
        assert_eq!(world.entity(bottom).unwrap().sprite, expected_sprite);
    }
}
#[test]
fn source_three_row_pool_has_one_guard_per_receiver_and_conditional_variant() {
    let mut observed_tall = false;
    let mut observed_short = false;
    // Fixed raw input census, not selected by a generated/controller outcome.
    for seed in 0..16 {
        let mut world = World::new(8, 8, Area::Jungle, 5);
        let top = world.spawn("oWaterSwim", Vec2::new(32., 32.));
        let middle = world.spawn("oWaterSwim", Vec2::new(32., 48.));
        let bottom = world.spawn("oWaterSwim", Vec2::new(32., 64.));
        world.set_tile(2, 5, "oLush");
        let mut actual = Rng::new(seed);
        let mut expected = seed;
        draw(&mut expected, 4); // top
        draw(&mut expected, 4); // middle: down false
        let tall = draw(&mut expected, 4) == 1; // bottom, all earlier values true
        let expected_sprite = if tall {
            observed_tall = true;
            "sWaterBottomTall2"
        } else {
            observed_short = true;
            let variant = draw(&mut expected, 4);
            [
                "sWaterBottom",
                "sWaterBottom2",
                "sWaterBottom3",
                "sWaterBottom4",
            ][(variant - 1) as usize]
        };
        setup_with_revision(
            &mut world,
            &mut actual,
            crate::GenerationRevision::SourceClassicV2,
        );
        assert_eq!(actual.state_bits(), u64::from(expected));
        assert_eq!(world.entity(top).unwrap().sprite, "sWaterTop");
        assert_eq!(world.entity(bottom).unwrap().sprite, expected_sprite);
        assert_eq!(
            world.entity(middle).unwrap().sprite,
            if tall { "sWaterBottomTall1" } else { "sWater" }
        );
    }
    assert!(observed_tall && observed_short);
}
#[test]
fn source_water_pass_leaves_lava_for_its_separate_source_pass() {
    let mut world = World::new(8, 8, Area::Temple, 13);
    let lava = world.spawn("oLava", Vec2::new(32., 32.));
    let before_sprite = world.entity(lava).unwrap().sprite.clone();
    let mut actual = Rng::new(42);
    setup_with_revision(
        &mut world,
        &mut actual,
        crate::GenerationRevision::SourceClassicV2,
    );
    assert_eq!(actual.state_bits(), 42);
    assert_eq!(world.entity(lava).unwrap().sprite, before_sprite);
}
