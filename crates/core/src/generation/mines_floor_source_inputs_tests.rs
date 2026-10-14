//! Source Mines floor guard: a fixed untouched stock seed and two query controls.
use super::*;
use crate::{Game, RngAlgorithm, RulesPreset};

struct Trace {
    before: World,
    origin: Vec2,
    rng_before: u32,
    rng_after: u32,
    first_id: u64,
    loot_id: u64,
    births: Vec<(String, Vec2)>,
}

pub(super) struct Pending(Trace);
thread_local! {
    static TRACE: std::cell::RefCell<Option<Option<Trace>>> = const { std::cell::RefCell::new(None) };
}
struct Capture;
impl Drop for Capture {
    fn drop(&mut self) {
        TRACE.with(|trace| *trace.borrow_mut() = None);
    }
}

pub(super) fn before(
    world: &World,
    rng: &Rng,
    source: bool,
    floor: bool,
    x: i32,
    y: i32,
    treasure_first_id: u64,
) -> Option<Pending> {
    let armed = TRACE.with(|trace| trace.borrow().as_ref().is_some_and(Option::is_none));
    if !armed || !source || !floor || y <= 1 || !occupied(world, x, y - 1) {
        return None;
    }
    let origin = Vec2::new(f64::from(x) * 16.0, f64::from(y) * 16.0);
    if source_point_family(world, Vec2::new(origin.x, origin.y - 16.0), "oSolid") {
        return None;
    }
    // First geometry qualifier only; no draw/birth success or final outcome filter.
    let loot = world.entities.iter().find(|e| {
        e.alive
            && e.engine_active
            && e.id >= treasure_first_id
            && matches!(
                e.object.as_str(),
                "oJar"
                    | "oRock"
                    | "oGoldBar"
                    | "oGoldBars"
                    | "oEmeraldBig"
                    | "oSapphireBig"
                    | "oRubyBig"
                    | "oDamsel"
            )
            && e.bounds().contains(at(x, y - 1))
    })?;
    assert_eq!(rng.algorithm(), RngAlgorithm::Gm8Lcg32V1);
    Some(Pending(Trace {
        before: world.clone(),
        origin,
        rng_before: rng.state_bits() as u32,
        rng_after: 0,
        first_id: world.next_id,
        loot_id: loot.id,
        births: Vec::new(),
    }))
}

pub(super) fn after(world: &World, rng: &Rng, pending: Option<Pending>) {
    let Some(Pending(mut trace)) = pending else {
        return;
    };
    trace.rng_after = rng.state_bits() as u32;
    trace.births = world
        .entities
        .iter()
        .filter(|e| e.id >= trace.first_id)
        .map(|e| (e.object.clone(), e.pos))
        .collect();
    TRACE.with(|slot| *slot.borrow_mut().as_mut().unwrap() = Some(trace));
}

// Independent finite-positive-width GM8 LCG transcription. Neither source
// Snake nor Caveman own/inherited Create has an additional random statement.
fn literal_roll(state: &mut u32, width: u32) -> u32 {
    *state = state.wrapping_mul(0x08088405).wrapping_add(1);
    ((u64::from(*state) * u64::from(width)) >> 32) as u32 + 1
}

#[test]
fn fixed_stock_seed0_mines_floor_draws_after_non_solid_loot_create() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        TRACE.with(|trace| *trace.borrow_mut() = Some(None));
        let _capture = Capture;
        let game = Game::new(0, rules);
        let trace = TRACE.with(|trace| trace.borrow_mut().take().unwrap())
            .expect("FIXTURE: fixed stock seed0 must contain a source-clear floor newly occupied by loot");
        assert_eq!(game.world.area, Area::Mines);
        assert_eq!(game.world.level, 1);
        assert_eq!(
            game.generation_revision,
            crate::GenerationRevision::SourceClassicV2
        );
        assert_eq!(game.rng.algorithm(), RngAlgorithm::Gm8Lcg32V1);
        assert_eq!(game.player.health, 4);
        assert_eq!(game.level_tick, 0);
        let x = (trace.origin.x / 16.0) as i32;
        let y = (trace.origin.y / 16.0) as i32;
        assert!(trace.origin.y > 16.0 && !in_shop(&trace.before, x, y));
        assert!(!start_room(&trace.before, x, y - 1));
        assert!(!solid(&trace.before, x, y - 1));
        let support = trace.before.tile(x, y).unwrap();
        assert!(support.engine_active && crate::catalog::is_a(&support.object, "oSolid"));
        assert!(
            trace
                .before
                .tiles
                .iter()
                .filter(|tile| crate::catalog::is_a(&tile.object, "oSolid"))
                .all(|tile| tile.engine_active)
        );
        assert!(
            trace
                .before
                .entities
                .iter()
                .filter(|e| e.alive && crate::catalog::is_a(&e.object, "oSolid"))
                .all(|e| e.engine_active),
            "this generation context has no inactive Solid query ambiguity"
        );
        assert!(!source_point_family(
            &trace.before,
            Vec2::new(trace.origin.x, trace.origin.y - 16.0),
            "oSolid"
        ));
        let loot = trace.before.entity(trace.loot_id).unwrap();
        assert!(loot.alive && loot.engine_active && !crate::catalog::is_a(&loot.object, "oSolid"));
        assert!(loot.bounds().contains(at(x, y - 1)));
        let mut state = trace.rng_before;
        let mut expected = Vec::new();
        let above = Vec2::new(trace.origin.x, trace.origin.y - 16.0);
        if literal_roll(&mut state, 60) == 1 {
            expected.push(("oSnake".to_owned(), above));
        } else if literal_roll(&mut state, 800) == 1 {
            expected.push(("oCaveman".to_owned(), above));
        }
        eprintln!(
            "QUALIFIED_MINES_FLOOR seed0 rules={rules:?} support={:?} loot={}@{:?} RNG={}=>{} expected={state}",
            trace.origin, loot.object, loot.pos, trace.rng_before, trace.rng_after
        );
        assert_eq!(
            trace.rng_after, state,
            "SOURCE_FLOOR_60_THEN_CONDITIONAL_800"
        );
        assert_eq!(
            trace.births, expected,
            "literal source branch/Create order and coordinates"
        );
    }
}

#[test]
fn source_floor_gate_allows_item_enemy_overlap_but_rejects_actual_solid_point() {
    for object in ["oJar", "oSnake"] {
        let mut world = World::new(20, 20, Area::Mines, 1);
        let pos = if object == "oJar" {
            Vec2::new(168.0, 154.0)
        } else {
            Vec2::new(160.0, 144.0)
        };
        let actor = world.spawn(object, pos);
        assert!(world.entity(actor).unwrap().bounds().contains(at(10, 9)));
        assert!(!crate::catalog::is_a(object, "oSolid"));
        assert!(
            mines_floor_clear(&world, true, true, 10, 10),
            "only Solid guards the source floor branch"
        );
        world.set_tile(10, 9, "oHardBlock");
        assert!(source_point_family(
            &world,
            Vec2::new(160.0, 144.0),
            "oSolid"
        ));
        assert!(!mines_floor_clear(&world, true, false, 10, 10));
    }
}

#[test]
fn historical_floor_gate_retains_its_prior_occupied_policy() {
    let mut world = World::new(20, 20, Area::Mines, 1);
    assert!(mines_floor_clear(&world, false, true, 10, 10));
    world.spawn("oJar", Vec2::new(168.0, 154.0));
    assert!(occupied(&world, 10, 9));
    assert!(!mines_floor_clear(&world, false, true, 10, 10));
    assert!(!mines_floor_clear(&world, false, false, 10, 10));
}
