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
) -> u64 {
    let expected = literal_origins(k, closed_bottom, ice);
    assert_eq!(births.len(), expected.len());
    // Real parents exist; a draw-only compensation or discarded exterior
    // birth cannot satisfy identities, ordered serials and this census.
    assert_eq!(world.entities.len(), expected.len());
    assert_eq!(
        world.tiles.iter().filter(|t| !t.object.is_empty()).count(),
        expected
            .iter()
            .filter(|(x, y, _)| *x < 42 && *y < world.height)
            .count()
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
        let state = verify(&world, &observed.births, 0, k, bottom, ice, object, false);
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
            true
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
                false
            ),
            state
        );
        let mut reference = state as u32;
        assert_eq!(literal_rand(&mut reference, 4) - 1, first_col);
    }
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
    special::apply(&mut world, rng, &selected, state.revision);
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
    terrain::finalize_generated(&mut world);
    GeneratedLevel {
        world,
        dark_roll_selected,
    }
}
