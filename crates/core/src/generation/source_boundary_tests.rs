//! Independent literal controls for scrInitLevel's boundary phase only.
//! Interior Create batching, reached tile predicates, and exterior SetupWalls
//! remain separate debt; these controls claim neither whole streams nor routes.
use super::*;
use crate::catalog;
use std::cell::RefCell;

#[derive(Clone, Debug)]
struct Birth {
    id: u64,
    origin: Vec2,
    object: String,
    create_sprite: String,
    before: u64,
    after: u64,
}
#[derive(Default)]
struct Observation {
    births: Vec<Birth>,
    before_path: Option<(World, u64)>,
    capture_before_setup: bool,
    before_setup: Option<(World, Rng)>,
}
thread_local! {
    static OBSERVATION: RefCell<Option<Observation>> = const { RefCell::new(None) };
}
pub(super) fn observe_create(
    id: u64,
    origin: Vec2,
    object: &str,
    sprite: &str,
    before: u64,
    after: u64,
) {
    OBSERVATION.with(|slot| {
        if let Some(probe) = slot.borrow_mut().as_mut() {
            probe.births.push(Birth {
                id,
                origin,
                object: object.into(),
                create_sprite: sprite.into(),
                before,
                after,
            });
        }
    });
}
pub(super) fn observe_before_path(world: &World, rng: &Rng) {
    OBSERVATION.with(|slot| {
        if let Some(probe) = slot.borrow_mut().as_mut() {
            assert!(probe.before_path.is_none());
            probe.before_path = Some((world.clone(), rng.state_bits()));
        }
    });
}
fn begin() {
    OBSERVATION.with(|slot| {
        assert!(slot.borrow().is_none());
        *slot.borrow_mut() = Some(Observation::default());
    });
}
fn begin_setup_capture() {
    begin();
    OBSERVATION.with(|slot| {
        slot.borrow_mut().as_mut().unwrap().capture_before_setup = true;
    });
}
pub(super) fn observe_before_setup(world: &World, rng: &Rng) {
    OBSERVATION.with(|slot| {
        if let Some(probe) = slot.borrow_mut().as_mut()
            && probe.capture_before_setup
        {
            assert!(probe.before_setup.is_none());
            probe.before_setup = Some((world.clone(), rng.clone()));
        }
    });
}
fn finish() -> Observation {
    OBSERVATION.with(|slot| slot.borrow_mut().take().unwrap())
}

// Literal source shape, arranged by the two wall columns versus other columns,
// rather than using the production boundary predicate or height inference.
fn literal_origins(k: usize, closed_bottom: bool, ice: bool) -> Vec<(usize, usize, bool)> {
    let mut result = Vec::new();
    for col in 0..43 {
        if col == 0 || col == 41 {
            for row in 0..=k {
                result.push((col, row, true));
            }
        } else {
            result.push((col, 0, true));
            if closed_bottom {
                result.push((col, k, true));
            }
        }
    }
    if ice {
        for col in 0..43 {
            result.push((col, 40, false));
        }
    }
    result
}
// Finite integer GM8 oracle: no production Rng/chance/range calls.
fn literal_rand(state: &mut u32, width: u32) -> u32 {
    *state = state.wrapping_mul(0x0808_8405).wrapping_add(1);
    1 + ((u64::from(*state) * u64::from(width)) >> 32) as u32
}
fn verify(
    world: &World,
    births: &[Birth],
    seed: u32,
    k: usize,
    closed_bottom: bool,
    ice: bool,
    object: &str,
    city: bool,
    preexisting_parents: usize,
    preexisting_tiles: usize,
) -> u64 {
    let expected = literal_origins(k, closed_bottom, ice);
    assert_eq!(births.len(), expected.len());
    // Real parents exist; a draw-only compensation or discarded exterior
    // birth cannot satisfy identities, ordered serials and this census.
    assert_eq!(world.entities.len(), expected.len() + preexisting_parents);
    assert_eq!(
        world.tiles.iter().filter(|t| !t.object.is_empty()).count(),
        expected
            .iter()
            .filter(|(x, y, _)| *x < 42 && *y < world.height)
            .count()
            + preexisting_tiles
    );
    let mut state = seed;
    let mut prior_serial = 0;
    for (birth, &(x, y, invincible)) in births.iter().zip(&expected) {
        let p = Vec2::new(x as f64 * 16., y as f64 * 16.);
        assert_eq!(birth.origin, p);
        assert_eq!(birth.object, object);
        assert_eq!(birth.before, u64::from(state));
        let mut sprite = format!("s{}", &object[1..]);
        if object == "oBrick" && literal_rand(&mut state, 10) == 1 {
            sprite = "sBrick2".into();
        }
        let n = literal_rand(&mut state, 100);
        if city && object == "oTemple" {
            sprite = "sGTemple".into();
        } else if n < 20 {
            sprite = format!("s{}Gold", &object[1..]);
        } else if n < 30 {
            sprite = format!("s{}GoldBig", &object[1..]);
        }
        assert_eq!(birth.create_sprite, sprite);
        assert_eq!(birth.after, u64::from(state));
        // Every actual boundary/floor coordinate is outside the strict
        // source interior child domain, independent of constructor outcomes.
        assert!(!(p.x > 1. && p.x < 656. && p.y > 1. && p.y < world.height as f64 * 16. - 16.));
        if invincible && !(city && object == "oTemple") {
            sprite = format!("s{}", &object[1..]);
        }
        let e = world
            .entity(birth.id)
            .expect("real boundary parent survives");
        assert!(e.creation_serial > prior_serial);
        prior_serial = e.creation_serial;
        assert_eq!(e.value("n"), f64::from(n));
        assert_eq!(e.value("invincible"), f64::from(invincible));
        assert_eq!(e.value("terrain_initialized"), 1.);
        assert_eq!(e.value("source_boundary_birth"), 1.);
        assert_eq!(e.value("shopWall"), 0.);
        assert_eq!(e.value("cleanDeath"), 0.);
        assert_eq!(e.text.get("type").map(String::as_str), Some("None"));
        assert_eq!(e.text.get("treasure").map(String::as_str), Some(""));
        if x < 42 && y < world.height {
            let tile = world.tile(x as i32, y as i32).unwrap();
            assert_eq!(tile.object, object);
            assert_eq!(tile.creation_serial + 1, e.creation_serial);
            assert_eq!(e.pos, Vec2::new(p.x + 8., p.y + 8.));
            assert_eq!(e.text.get("terrain_sprite"), Some(&sprite));
        } else {
            assert_eq!(e.object, object);
            assert_eq!(e.pos, p);
            assert_eq!(e.sprite, sprite);
        }
        let gold = sprite.ends_with("Gold") || sprite.ends_with("GoldBig") || sprite == "sGTemple";
        assert_eq!(e.value("gold_chunks"), if gold { 3. } else { 0. });
        assert_eq!(
            e.value("gold_nuggets"),
            if sprite.ends_with("GoldBig") || sprite == "sGTemple" {
                1.
            } else {
                0.
            }
        );
    }
    u64::from(state)
}

#[test]
fn literal_boundary_births_complete_create_then_caller_overrides_in_order() {
    for (area, height, k, bottom, ice, object, expected_births, draws) in [
        (Area::Mines, 34, 33, true, false, "oBrick", 150, 300),
        (Area::Jungle, 34, 33, true, false, "oLush", 150, 150),
        (Area::Jungle, 42, 41, true, false, "oLush", 166, 166),
        (Area::Ice, 38, 38, false, true, "oDark", 162, 162),
        (Area::Temple, 34, 33, true, false, "oTemple", 150, 150),
        (Area::Boss, 55, 54, false, false, "oTemple", 151, 151),
    ] {
        let mut world = World::new(42, height, area, 1);
        let mut rng = Rng::new(0);
        begin();
        source_boundaries(&mut world, &mut rng, height == 42, false);
        let observed = finish();
        assert_eq!(observed.births.len(), expected_births);
        let state = verify(
            &world,
            &observed.births,
            0,
            k,
            bottom,
            ice,
            object,
            false,
            0,
            0,
        );
        assert_eq!(rng.state_bits(), state);
        let mut count_state = 0;
        for _ in 0..draws {
            literal_rand(&mut count_state, 1);
        }
        assert_eq!(state, u64::from(count_state));
        assert!(
            world
                .entities
                .iter()
                .all(|e| !catalog::is_a(&e.object, "oTreasure")
                    && !catalog::is_a(&e.object, "oItem")),
            "strict literal bounds birth no children"
        );
        if area == Area::Boss {
            assert!(
                world.tile(1, 54).unwrap().object.is_empty(),
                "source Boss has no boundary bottom floor"
            );
        }
    }
}

#[test]
fn city_gold_boundary_still_draws_and_retains_create_gold_after_invincibility_override() {
    let mut world = World::new(42, 34, Area::Temple, 14);
    let mut rng = Rng::new(0);
    begin();
    source_boundaries(&mut world, &mut rng, false, true);
    let observed = finish();
    assert_eq!(
        verify(
            &world,
            &observed.births,
            0,
            33,
            true,
            false,
            "oTemple",
            true,
            0,
            0,
        ),
        2_892_300_850
    );
    assert_eq!(rng.state_bits(), 2_892_300_850);
}

#[test]
fn generated_source_boundary_rng_and_real_world_exist_before_path_selection() {
    for (level, k, bottom, ice, object, pinned_state, first_col) in [
        (1, 33, true, false, "oBrick", 16_145_268, 2),
        (9, 38, false, true, "oDark", 987_897_958, 3),
    ] {
        let mut rng = Rng::new(0);
        begin();
        let _world = generate(&mut rng, level, &BTreeSet::new());
        let observed = finish();
        let (before_path, state) = observed
            .before_path
            .expect("real generation checkpoint before scrLevelGen");
        assert_eq!(state, pinned_state);
        assert_eq!(
            verify(
                &before_path,
                &observed.births,
                0,
                k,
                bottom,
                ice,
                object,
                false,
                0,
                0,
            ),
            state
        );
        let mut reference = state as u32;
        assert_eq!(literal_rand(&mut reference, 4) - 1, first_col);
    }
}

#[test]
fn olmec_create_tape_precedes_boundaries_and_route_and_is_reused_by_authored_instances() {
    const SEED: u32 = 0;
    const EXPECTED_AFTER_TEMPLE_CREATE: u32 = 964_265_884;
    const EXPECTED_AFTER_STATIC_CREATE: u32 = 62_565_469;
    const EXPECTED_BEFORE_ROUTE: u32 = 1_078_910_996;
    const EXPECTED_ROUTE_START: usize = 2;
    const EXPECTED_TIMER_DIGEST: u64 = 2_439_870_408_187_396_080;
    // Independent literal room-resource order: row 0/1 is y=560/576, and
    // column 0 is x=16. Kept separate from the production tape constant.
    const SOURCE_TEMPLE_ORDER: [u8; 80] = [
        0, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31,
        32, 33, 34, 35, 36, 37, 38, 39, 79, 78, 77, 76, 75, 74, 73, 72, 71, 70, 69, 68, 67, 66, 65,
        64, 63, 62, 61, 60, 59, 58, 57, 56, 55, 54, 53, 52, 51, 50, 5, 6, 7, 9, 8, 49, 48, 47, 46,
        45, 4, 3, 2, 1, 40, 41, 42, 43, 44,
    ];
    assert_eq!(
        super::special::BOSS_TEMPLE_CREATE_ORDER,
        SOURCE_TEMPLE_ORDER
    );

    // Independent literal GM8 draws for all authored pre-oGame Create events.
    let mut source_state = SEED;
    let mut temple_results = Vec::new();
    for &position in &SOURCE_TEMPLE_ORDER {
        let roll = literal_rand(&mut source_state, 100);
        let treasure = if roll < 30 {
            None
        } else if literal_rand(&mut source_state, 60) == 1 {
            Some(("oSapphireBig", 0.0, 0.0))
        } else if literal_rand(&mut source_state, 80) == 1 {
            Some(("oEmeraldBig", 0.0, 0.0))
        } else if literal_rand(&mut source_state, 100) == 1 {
            Some(("oRubyBig", 0.0, 0.0))
        } else if literal_rand(&mut source_state, 1200) == 1 {
            const UNDERGROUND: [(&str, f64, f64); 19] = [
                ("oJetpack", 0.0, -2.0),
                ("oCapePickup", 0.0, 0.0),
                ("oShotgun", 0.0, 0.0),
                ("oMattock", 0.0, 0.0),
                ("oTeleporter", 0.0, 3.0),
                ("oGloves", 0.0, -1.0),
                ("oSpectacles", 0.0, 0.0),
                ("oWebCannon", -2.0, 0.0),
                ("oPistol", 0.0, 0.0),
                ("oMitt", 0.0, -1.0),
                ("oPaste", 0.0, 0.0),
                ("oSpringShoes", 0.0, 0.0),
                ("oSpikeShoes", 0.0, 0.0),
                ("oMachete", 0.0, 0.0),
                ("oBombBox", 0.0, -2.0),
                ("oBow", 0.0, 0.0),
                ("oCompass", 0.0, 0.0),
                ("oParaPickup", 0.0, 0.0),
                ("oRopePile", 0.0, 0.0),
            ];
            Some(UNDERGROUND[(literal_rand(&mut source_state, 19) - 1) as usize])
        } else {
            None
        };
        temple_results.push((position, roll, treasure));
    }
    assert_eq!(source_state, EXPECTED_AFTER_TEMPLE_CREATE);
    assert_eq!(
        temple_results
            .iter()
            .filter_map(|(position, _, treasure)| treasure.map(|(object, _, _)| (*position, object)))
            .collect::<Vec<_>>(),
        vec![(23, "oRubyBig"), (30, "oSapphireBig")]
    );
    let olmec_roll = literal_rand(&mut source_state, 2) - 1;
    let expected_facing = if olmec_roll == 0 { -1 } else { 1 };
    let expected_timers: Vec<u16> = (0..240)
        .map(|_| (100 + literal_rand(&mut source_state, 201) - 1) as u16)
        .collect();
    assert_eq!(source_state, EXPECTED_AFTER_STATIC_CREATE);
    let mut timer_digest = 0xcbf2_9ce4_8422_2325_u64;
    for (birth, timer) in expected_timers.iter().copied().enumerate() {
        for byte in [birth as u8, timer as u8, (timer >> 8) as u8] {
            timer_digest ^= u64::from(byte);
            timer_digest = timer_digest.wrapping_mul(0x100_0000_01b3);
        }
    }
    assert_eq!(timer_digest, EXPECTED_TIMER_DIGEST);

    // oGame then creates the 151 Boss boundary instances before scrLevelGen.
    for _ in 0..151 {
        literal_rand(&mut source_state, 100);
    }
    assert_eq!(source_state, EXPECTED_BEFORE_ROUTE);
    let expected_route_start = (literal_rand(&mut source_state, 4) - 1) as usize;
    assert_eq!(expected_route_start, EXPECTED_ROUTE_START);

    let mut positions = super::special::BOSS_LAVA_CREATE_ORDER;
    positions.sort_unstable();
    assert_eq!(positions, std::array::from_fn(|index| index as u8));

    let mut actual_rng = Rng::new(SEED);
    let flags = BTreeSet::from(["low_graphics".into(), "no_dark_level".into()]);
    begin();
    let world = generate(&mut actual_rng, 16, &flags);
    let observed = finish();
    let (before_path, before_path_rng) = observed
        .before_path
        .expect("Boss source generation checkpoints immediately before scrLevelGen");
    assert_eq!(before_path_rng, u64::from(EXPECTED_BEFORE_ROUTE));
    assert_eq!(
        verify(
            &before_path,
            &observed.births,
            EXPECTED_AFTER_STATIC_CREATE,
            54,
            false,
            false,
            "oTemple",
            false,
            82,
            80,
        ),
        u64::from(EXPECTED_BEFORE_ROUTE)
    );
    let route = world
        .source_room_path
        .as_ref()
        .expect("SourceClassic level 16 retains its selected room route");
    assert_eq!(
        route[0].iter().position(|&room| room == 1),
        Some(EXPECTED_ROUTE_START)
    );

    for &(position, roll, treasure) in &temple_results {
        let x = usize::from(position) % 40 + 1;
        let y = usize::from(position) / 40 + 35;
        let metadata = world
            .tile_sprite_metadata(x as i32, y as i32)
            .expect("authored Temple Create result is attached to its tile");
        assert_eq!(metadata.value("n"), f64::from(roll));
        if let Some((object, dx, dy)) = treasure {
            let center = Vec2::new(x as f64 * 16.0 + 8.0, y as f64 * 16.0 + 8.0);
            assert!(
                world.entities.iter().any(|entity| {
                    entity.alive
                        && entity.object == object
                        && entity.pos == Vec2::new(center.x + dx, center.y + dy)
                }),
                "source Temple treasure {object} at ({x}, {y})"
            );
        }
    }

    let olmec = world
        .entities
        .iter()
        .find(|entity| entity.object == "oOlmec")
        .expect("authored Olmec instance");
    assert_eq!(olmec.facing, expected_facing);
    assert_eq!(olmec.value("olmec_facing_created"), 1.0);

    let lava: Vec<_> = world
        .entities
        .iter()
        .filter(|entity| entity.object == "oLava")
        .collect();
    assert_eq!(lava.len(), 240);
    for (birth, (entity, &position)) in lava
        .iter()
        .zip(&super::special::BOSS_LAVA_CREATE_ORDER)
        .enumerate()
    {
        let position = usize::from(position);
        let expected_pos = Vec2::new(
            ((position % 40) + 1) as f64 * 16.0,
            ((position / 40) + 48) as f64 * 16.0,
        );
        assert_eq!(entity.pos, expected_pos, "source Lava birth {birth}");
        assert_eq!(
            entity.value("spurt_time"),
            f64::from(expected_timers[birth]),
            "source Lava timer at {expected_pos:?}"
        );
        assert_eq!(
            entity.value("spurt_counter"),
            f64::from(expected_timers[birth])
        );
        assert_eq!(entity.text.get("type").map(String::as_str), Some("Lava"));
        assert_eq!(entity.value("randNum"), 201.0);
        assert_eq!(entity.image_speed(), 0.4);
        assert_eq!(entity.value("lava_initialized"), 1.0);
    }
}

#[test]
fn olmec_setup_walls_begins_in_authored_room_temple_creation_order() {
    // rOlmec.xml lists these 80 authored oTemple instances before oGame.
    // Their Create events therefore finish before scrInitLevel creates the
    // controller boundaries and generated room terrain; scrSetupWalls runs
    // afterward and visits the same-family parents in birth order.
    const SOURCE_TEMPLE_ORDER: [u8; 80] = [
        0, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31,
        32, 33, 34, 35, 36, 37, 38, 39, 79, 78, 77, 76, 75, 74, 73, 72, 71, 70, 69, 68, 67, 66, 65,
        64, 63, 62, 61, 60, 59, 58, 57, 56, 55, 54, 53, 52, 51, 50, 5, 6, 7, 9, 8, 49, 48, 47, 46,
        45, 4, 3, 2, 1, 40, 41, 42, 43, 44,
    ];
    let expected: Vec<_> = SOURCE_TEMPLE_ORDER
        .iter()
        .map(|position| {
            let position = usize::from(*position);
            (1 + position % 40, 35 + position / 40)
        })
        .collect();

    let mut rng = Rng::new(0);
    let mut state = GenerationState::fresh(&rng);
    begin_setup_capture();
    let generated = generate_campaign_result(
        &mut rng,
        16,
        &BTreeSet::from(["no_dark_level".into()]),
        GenerationContext::default(),
        &mut state,
    );
    let observed = finish();
    let mut temples: Vec<_> = generated
        .world
        .tiles
        .iter()
        .enumerate()
        .filter(|(_, tile)| tile.object == "oTemple")
        .map(|(index, tile)| {
            (
                tile.creation_serial,
                (index % generated.world.width, index / generated.world.width),
            )
        })
        .collect();
    temples.sort_by_key(|(serial, _)| *serial);

    assert_eq!(
        temples
            .iter()
            .take(SOURCE_TEMPLE_ORDER.len())
            .map(|(_, position)| *position)
            .collect::<Vec<_>>(),
        expected,
        "scrSetupWalls must visit rOlmec's authored Temple parents before generated Temple walls"
    );

    // Replay SetupWalls from its captured real-generation input after assigning
    // the authored parents their literal earlier source order. This pins the
    // resulting RNG continuation, including later setup and background draws.
    let (source_input_world, source_input_rng) = observed
        .before_setup
        .expect("real Boss generation captures its pre-SetupWalls state");
    let authored: std::collections::BTreeSet<_> = expected.iter().copied().collect();
    let first_later_temple = source_input_world
        .tiles
        .iter()
        .enumerate()
        .filter(|(index, tile)| {
            tile.object == "oTemple"
                && !authored.contains(&(
                    index % source_input_world.width,
                    index / source_input_world.width,
                ))
        })
        .map(|(_, tile)| tile.creation_serial)
        .chain(
            source_input_world
                .entities
                .iter()
                .filter(|entity| entity.alive && entity.object == "oTemple")
                .map(|entity| entity.creation_serial),
        )
        .min()
        .expect("controller and generated Temple parents follow the authored arena");
    assert!(first_later_temple > SOURCE_TEMPLE_ORDER.len() as u64);

    let mut late_order_world = source_input_world.clone();
    let mut late_order_rng = source_input_rng.clone();
    let latest_parent = late_order_world
        .tiles
        .iter()
        .map(|tile| tile.creation_serial)
        .chain(
            late_order_world
                .entities
                .iter()
                .map(|entity| entity.creation_serial),
        )
        .max()
        .unwrap();
    for (serial, &(x, y)) in expected.iter().enumerate() {
        late_order_world.tiles[y * late_order_world.width + x].creation_serial =
            latest_parent + serial as u64 + 1;
    }
    terrain::setup_generated_with_revision(
        &mut late_order_world,
        &mut late_order_rng,
        true,
        crate::GenerationRevision::SourceClassicV2,
        false,
    );
    background_extras(&mut late_order_world, &mut late_order_rng, true);

    let mut source_order_world = source_input_world;
    let mut source_order_rng = source_input_rng;
    for (serial, &(x, y)) in expected.iter().enumerate() {
        let tile = &mut source_order_world.tiles[y * source_order_world.width + x];
        assert_eq!(tile.object, "oTemple");
        tile.creation_serial =
            first_later_temple - SOURCE_TEMPLE_ORDER.len() as u64 + serial as u64;
    }
    terrain::setup_generated_with_revision(
        &mut source_order_world,
        &mut source_order_rng,
        true,
        crate::GenerationRevision::SourceClassicV2,
        false,
    );
    background_extras(&mut source_order_world, &mut source_order_rng, true);
    assert_eq!(rng.state_bits(), source_order_rng.state_bits());
    assert_eq!(rng.next_u64(), source_order_rng.next_u64());
    assert_ne!(
        source_order_rng.state_bits(),
        late_order_rng.state_bits(),
        "seed 0 must expose the source-order SetupWalls RNG shift"
    );
}

#[test]
fn explicit_historical_revision_matches_sealed_before_function_world_and_rng() {
    for canonical in [false, true] {
        for (level, flags) in [
            (1, BTreeSet::new()),
            (9, BTreeSet::new()),
            (16, BTreeSet::new()),
            (5, BTreeSet::from(["lake".into()])),
            (14, BTreeSet::from(["city_of_gold".into()])),
        ] {
            let mut actual_rng = if canonical {
                Rng::new(0)
            } else {
                Rng::legacy(0)
            };
            let mut reference_rng = actual_rng.clone();
            let mut actual_state = GenerationState::fresh(&actual_rng);
            actual_state.revision = crate::GenerationRevision::HistoricalV1;
            let mut reference_state = actual_state;
            begin();
            let actual = generate_campaign_result(
                &mut actual_rng,
                level,
                &flags,
                GenerationContext::default(),
                &mut actual_state,
            );
            assert!(
                finish().births.is_empty(),
                "historical never enters source boundary constructor"
            );
            let reference = historical_before_boundary_correction(
                &mut reference_rng,
                level,
                &flags,
                GenerationContext::default(),
                &mut reference_state,
            );
            assert_eq!(
                serde_json::to_vec(&actual.world).unwrap(),
                serde_json::to_vec(&reference.world).unwrap()
            );
            assert_eq!(actual.dark_roll_selected, reference.dark_roll_selected);
            assert_eq!(actual_rng.state_bits(), reference_rng.state_bits());
            assert_eq!(actual_state.market_chance, reference_state.market_chance);
            assert_eq!(
                actual_state.made_market_entrance,
                reference_state.made_market_entrance
            );
        }
    }
}

// Exact pre-correction campaign function, sealed in preparation. Only its
// name/visibility and an explicit HistoricalV1 precondition differ. Existing
// helpers are unchanged by this patch, so this checks the complete historical
// producer result/creation order/masks/metadata/RNG rather than counts alone.
fn historical_before_boundary_correction(
    rng: &mut Rng,
    level: u32,
    flags: &BTreeSet<String>,
    context: GenerationContext,
    state: &mut GenerationState,
) -> GeneratedLevel {
    assert_eq!(state.revision, crate::GenerationRevision::HistoricalV1);
    // Validate before room construction, callbacks, or RNG draws. Historical
    // serialized state remains historical even if its RNG is canonical.
    if state.revision == crate::GenerationRevision::SourceClassicV2 {
        assert_eq!(
            rng.algorithm(),
            crate::RngAlgorithm::Gm8Lcg32V1,
            "SourceClassicV2 requires canonical RNG identity"
        );
        assert!(
            (MIN_SUPPORTED_MARKET_CHANCE..=(1 << 21)).contains(&state.market_chance),
            "marketChance history is outside the proved campaign domain with one-room draw headroom"
        );
    }
    let area = Area::for_level(level);
    let mut selected = flags.clone();
    if state.revision == crate::GenerationRevision::SourceClassicV2 {
        reset_dark_history(&mut selected, level);
    }
    if area == Area::Jungle
        && !has(&selected, "black_market")
        && !has(&selected, "lake")
        && rng.chance(1, 10)
    {
        selected.insert("lake".into());
    }
    let mut world = World::new(42, 34, area, level);
    if special::generate_special(&mut world, rng, flags) {
        world.fluid_dirty = false;
        return GeneratedLevel {
            world,
            dark_roll_selected: false,
            cemetery_selected: false,
            sacrifice_pit_selected: false,
        };
    }
    let source_classic = state.revision == crate::GenerationRevision::SourceClassicV2;
    let path = room_path(rng, area, level, &mut selected, source_classic);
    if select_cemetery(rng, area, has(&selected, "cemetery"), source_classic) {
        selected.insert("cemetery".into());
    }
    let height = if area == Area::Boss {
        55
    } else if area == Area::Ice {
        // Ice uses rLevel2 (608 px); rLevel3 is the 672 px Jungle lake.
        // The source creates a dark floor at y=640 outside the Ice room,
        // below its y>room_height+16 falling-death threshold.
        38
    } else if has(&selected, "lake") {
        42
    } else {
        34
    };
    world = World::new(42, height, area, level);
    world.name = if has(&selected, "black_market") {
        "Black Market"
    } else if has(&selected, "city_of_gold") {
        "City of Gold"
    } else if has(&selected, "yeti_lair") {
        "Yeti Lair"
    } else if has(&selected, "alien_ship") {
        "Alien Ship"
    } else if has(&selected, "lake") {
        "Lake"
    } else if has(&selected, "cemetery") {
        "Cemetery"
    } else if has(&selected, "snake_pit") {
        "Snake Pit"
    } else if has(&selected, "sacrifice_pit") {
        "Sacrifice Pit"
    } else {
        match area {
            Area::Mines => "Mines",
            Area::Jungle => "Jungle",
            Area::Ice => "Ice Caves",
            Area::Temple => "Temple",
            Area::Boss => "Olmec's Lair",
        }
    }
    .into();
    // scrInitLevel creates concrete, invincible terrain, never the sprite-less
    // oSolid parent. The controller queries the source sprite mask, so a bare
    // oSolid cell is not a substitute for these authored wall instances.
    let boundary_object = match area {
        Area::Mines => "oBrick",
        Area::Jungle => "oLush",
        Area::Ice => "oDark",
        Area::Temple | Area::Boss => "oTemple",
    };
    for y in 0..height {
        boundary_tile(&mut world, 0, y, boundary_object);
        boundary_tile(&mut world, 41, y, boundary_object);
    }
    for x in 1..41 {
        boundary_tile(&mut world, x, 0, boundary_object);
        if area != Area::Ice {
            boundary_tile(&mut world, x, height - 1, boundary_object);
        }
    }
    if area == Area::Ice {
        // Source j<=38 also authors the two side-wall instances at y=608,
        // beyond rLevel2's storage height. Keep that last wall segment without
        // creating a floor across the open abyss or moving its death threshold.
        for x in [0.0, 656.0] {
            let id = world.spawn("oDark", Vec2::new(x, 608.0));
            world
                .entity_mut(id)
                .unwrap()
                .data
                .insert("invincible".into(), 1.0);
        }
    }
    let script_name = match area {
        Area::Mines => "scrRoomGen",
        Area::Jungle => {
            if has(&selected, "black_market") {
                "scrRoomGenMarket"
            } else {
                "scrRoomGen2"
            }
        }
        Area::Ice => {
            if has(&selected, "yeti_lair") {
                "scrRoomGenYeti"
            } else {
                "scrRoomGen3"
            }
        }
        Area::Temple => "scrRoomGen4",
        Area::Boss => "scrRoomGen5",
    };
    let script = &archive().scripts[script_name];
    let mut features = Features::default();
    features.idol = has(&selected, "sacrifice_pit");
    // scrLevelGen87 sets the room-generation latch before stamping the pit.
    // It is unrelated to the selected playable character.
    features.damsel = state.revision == crate::GenerationRevision::SourceClassicV2
        && has(&selected, "sacrifice_pit");
    for row in 0..if has(&selected, "lake") { 5 } else { 4 } {
        for col in 0..4 {
            let start = row == 0 && col == path.start;
            let end = row == 3 && col == path.end;
            let (template, shop) = choose_room(
                script,
                script_name,
                rng,
                path.rooms[row][col],
                if row > 0 { path.rooms[row - 1][col] } else { 0 },
                start,
                end,
                col,
                row,
                level,
                &selected,
                &mut features,
                &world,
                state.revision == crate::GenerationRevision::SourceClassicV2,
            );
            let expanded = expand(script, rng, template);
            if area == Area::Boss && row == 3 {
                tiles::stamp_at(
                    &mut world,
                    rng,
                    &expanded,
                    1 + col * 10,
                    37,
                    false,
                    false,
                    shop,
                );
            } else if state.revision == crate::GenerationRevision::SourceClassicV2 {
                tiles::stamp_with_damsel(
                    &mut world,
                    rng,
                    &expanded,
                    col,
                    row,
                    start,
                    end,
                    shop,
                    features.damsel,
                );
            } else {
                tiles::stamp(&mut world, rng, &expanded, col, row, start, end, shop);
            }
        }
    }
    let dark_roll_selected = if state.revision == crate::GenerationRevision::SourceClassicV2 {
        let (visible, selected_roll) = select_dark_level(rng, level, area, &selected);
        world.dark = visible;
        selected_roll
    } else {
        // Preserve explicitly historical generation/replay streams.
        world.dark = !has(&selected, "black_market")
            && (has(&selected, "dark")
                || (!has(&selected, "had_dark_level")
                    && !has(&selected, "no_dark_level")
                    && level > 1
                    && area != Area::Ice
                    && area != Area::Boss
                    && rng.chance(1, 12)));
        false
    };
    special::apply(&mut world, rng, &selected, state.revision, None);
    apply_wanted_posters(&mut world, &selected);
    let graphics_high = !has(&selected, "low_graphics");
    terrain::create_generated_with_context(&mut world, rng, context);
    // Construction/carving does not dispatch source Solid Destroy events.
    // Population does; preserve any real parent checkWater request it emits.
    world.fluid_dirty = false;
    population::populate_with_campaign(&mut world, rng, &selected, context, state, &path.rooms);
    terrain::setup_generated_with_revision(
        &mut world,
        rng,
        graphics_high,
        state.revision,
        has(&selected, "cemetery"),
    );
    if graphics_high {
        background_extras(&mut world, rng, source_classic);
    }
    #[cfg(test)]
    terrain::observe_init_cleanup(&world, false);
    init_cleanup::cleanup(&mut world, state.revision);
    #[cfg(test)]
    terrain::observe_init_cleanup(&world, true);
    terrain::finalize_generated(&mut world, state.revision);
    GeneratedLevel {
        world,
        dark_roll_selected,
        cemetery_selected: has(&selected, "cemetery"),
        sacrifice_pit_selected: has(&selected, "sacrifice_pit"),
    }
}
