//! Local Ice ceiling statement oracle. The public seed/level/flags are fixed;
//! the incoming prefix is captured, not guessed or replaced. This does not
//! certify earlier generation, native with-family order, or Game campaign history.
use super::*;
use crate::{CollisionPrecision, GenerationRevision, RngAlgorithm};
use std::cell::RefCell;

type Before = (World, u64, i32, i32, Option<GenerationRevision>);
pub(super) struct First {
    before: World,
    rng_before: u64,
    x: i32,
    y: i32,
    revision: Option<GenerationRevision>,
    rng_after: u64,
    ids_after: Vec<u64>,
}
thread_local! {
    static CAPTURE: RefCell<Option<Option<First>>> = const { RefCell::new(None) };
}
pub(super) fn observe_before(
    world: &World,
    rng: &Rng,
    x: i32,
    y: i32,
    revision: Option<GenerationRevision>,
) -> Option<Before> {
    CAPTURE.with(|capture| {
        matches!(*capture.borrow(), Some(None))
            .then(|| (world.clone(), rng.state_bits(), x, y, revision))
    })
}
pub(super) fn observe_after(world: &World, rng: &Rng, before: Option<Before>) {
    if let Some((before, rng_before, x, y, revision)) = before {
        CAPTURE.with(|capture| {
            *capture.borrow_mut() = Some(Some(First {
                before,
                rng_before,
                x,
                y,
                revision,
                rng_after: rng.state_bits(),
                ids_after: world.entities.iter().map(|entity| entity.id).collect(),
            }));
        });
    }
}
fn begin() {
    CAPTURE.with(|capture| {
        assert!(capture.borrow().is_none());
        *capture.borrow_mut() = Some(None);
    });
}
fn finish() -> First {
    CAPTURE.with(|capture| {
        capture.borrow_mut().take().unwrap().expect(
            "fixed Ice fixture must reach an eligible ceiling; do not repair by changing the seed",
        )
    })
}
fn source_draw_40(state: &mut u32) -> u32 {
    // Canonical integer rand(1,40): independent wrapping LCG and multiply-high.
    // No production Rng method participates in the expected prefix.
    *state = state.wrapping_mul(0x0808_8405).wrapping_add(1);
    1 + ((u64::from(*state) * 40) >> 32) as u32
}
fn bbox_point(world: &World, point: Vec2, family: &str) -> bool {
    world.tiles_point(point, family)
        || world.entities.iter().any(|entity| {
            entity.alive
                && entity.engine_active
                && crate::catalog::is_a(&entity.object, family)
                && entity.mask_contains(point, CollisionPrecision::BoundingBox)
        })
}
fn bright_ceiling_premises(first: &First) {
    let world = &first.before;
    assert_eq!((world.area, world.level), (Area::Ice, 9));
    assert_eq!((world.width, world.height), (42, 38));
    assert!(
        !world.dark,
        "no dark fixture flag or post-admission override"
    );
    let tile = world.tile(first.x, first.y).unwrap();
    assert!(tile.engine_active && crate::catalog::is_a(&tile.object, "oSolid"));
    assert!(tile.creation_serial > 0);
    assert!(!in_shop(world, first.x, first.y));
    assert!(!start_room(world, first.x, first.y - 1));
    assert!(first.y < world.height as i32 - 4);
    let water = WaterMasks::new(world);
    for delta in [1, 2] {
        assert!(!solid(world, first.x, first.y + delta));
        assert!(!water.wet(first.x, first.y + delta));
    }
    // Independently qualify the literal source receiver origin and point guards,
    // rather than claiming the existing tile-center adapter is generally exact.
    let origin = Vec2::new(f64::from(first.x) * 16.0, f64::from(first.y) * 16.0);
    assert!(origin.y < world.height as f64 * 16.0 - 64.0);
    assert_ne!(
        source_room(Vec2::new(origin.x, origin.y - 16.0)),
        source_room(world.entrance)
    );
    if let Some(path) = world.source_room_path {
        let (rx, ry) = source_room(origin);
        assert!(!matches!(path[ry as usize][rx as usize], 4 | 5));
    }
    for delta in [16.0, 32.0] {
        let point = Vec2::new(origin.x, origin.y + delta);
        assert!(!bbox_point(world, point, "oSolid"));
        assert!(!bbox_point(world, point, "oWater"));
    }
    // `with oSolid` visits every live descendant in creation order, including
    // top-border instances. Qualify the first source receiver directly instead
    // of reusing the historical interior-tile scan.
    let first_eligible = population_supports(world, true)
        .into_iter()
        .find(|support| {
            !campaign_shop_membership(world, support.origin)
                .unwrap_or_else(|| in_shop(world, support.x, support.y))
                && !source_start_room(world, support.origin)
                && support.origin.y < world.height as f64 * 16.0 - 64.0
                && !source_point_family(
                    world,
                    Vec2::new(support.origin.x, support.origin.y + 16.0),
                    "oSolid",
                )
                && !source_point_family(
                    world,
                    Vec2::new(support.origin.x, support.origin.y + 32.0),
                    "oSolid",
                )
                && !source_point_family(
                    world,
                    Vec2::new(support.origin.x, support.origin.y + 16.0),
                    "oWater",
                )
                && !source_point_family(
                    world,
                    Vec2::new(support.origin.x, support.origin.y + 32.0),
                    "oWater",
                )
        })
        .map(|support| (support.x, support.y));
    assert_eq!(first_eligible, Some((first.x, first.y)));
    let before_ids: Vec<_> = world.entities.iter().map(|entity| entity.id).collect();
    assert_eq!(
        first.ids_after, before_ids,
        "bright ceiling must not birth a Scarab or another actor"
    );
    eprintln!(
        "QUALIFIED_ICE_CEILING x={} y={} object={} serial={} revision={:?} dark={} rng_before={} rng_after={}",
        first.x,
        first.y,
        tile.object,
        tile.creation_serial,
        first.revision,
        world.dark,
        first.rng_before,
        first.rng_after
    );
}
#[test]
fn stock_seed_zero_level_nine_bright_first_ceiling_executes_source_rand40() {
    begin();
    let mut rng = Rng::new(0);
    assert_eq!(rng.algorithm(), RngAlgorithm::Gm8Lcg32V1);
    assert_eq!(rng.state_bits(), 0);
    let flags = BTreeSet::new();
    let generated = crate::generation::generate(&mut rng, 9, &flags);
    let first = finish();
    assert_eq!((generated.area, generated.level), (Area::Ice, 9));
    assert!(!generated.dark);
    assert!(generated.source_room_path.is_some());
    assert_eq!(first.revision, Some(GenerationRevision::SourceClassicV2));
    bright_ceiling_premises(&first);
    let mut expected = u32::try_from(first.rng_before).unwrap();
    let roll = source_draw_40(&mut expected);
    assert!((1..=40).contains(&roll));
    eprintln!("SOURCE_RAND40 value={roll} expected_state={expected}");
    assert_eq!(
        first.rng_after,
        u64::from(expected),
        "source RHS rand40 executes after false darkLevel; local prefix only"
    );
}
fn configured_bright_ceiling() -> World {
    // Compatibility controls use initial authored geometry, not a stock claim.
    let mut world = World::new(42, 38, Area::Ice, 9);
    world.name = "Ice Caves".into();
    world.entrance = at(3, 3);
    world.set_tile(20, 5, "oDark");
    world
}
#[test]
fn none_context_bright_ice_ceiling_preserves_its_prior_rng_stream() {
    let mut world = configured_bright_ceiling();
    let mut rng = Rng::new(0);
    begin();
    populate(&mut world, &mut rng, &BTreeSet::new());
    let first = finish();
    assert_eq!(first.revision, None);
    bright_ceiling_premises(&first);
    assert_eq!(first.rng_after, first.rng_before);
}
#[test]
fn historical_context_bright_ice_ceiling_preserves_its_prior_rng_stream() {
    // Historical revision with canonical RNG is legitimate serialized context;
    // the policy follows revision, not RNG algorithm. Exercise both dispatches.
    for direct in [false, true] {
        let mut world = configured_bright_ceiling();
        let mut rng = Rng::new(0);
        let mut campaign = super::super::GenerationState::fresh(&rng);
        campaign.revision = GenerationRevision::HistoricalV1;
        let rooms = [[0; 4]; 5];
        begin();
        if direct {
            populate_internal(
                &mut world,
                &mut rng,
                &BTreeSet::new(),
                super::super::GenerationContext::default(),
                Some(&mut campaign),
                Some(&rooms),
            );
        } else {
            populate_with_campaign(
                &mut world,
                &mut rng,
                &BTreeSet::new(),
                super::super::GenerationContext::default(),
                &mut campaign,
                &rooms,
            );
        }
        let first = finish();
        assert_eq!(
            first.revision,
            direct.then_some(GenerationRevision::HistoricalV1)
        );
        bright_ceiling_premises(&first);
        assert_eq!(first.rng_after, first.rng_before);
    }
}
