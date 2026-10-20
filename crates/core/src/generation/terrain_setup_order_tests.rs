//! Represented-parent SetupWalls order controls. These execute real Create
//! first, then compare only Setup against a literal GM8 integer RNG oracle.
//! Existing exterior/overlap parents are included; lost births and complete
//! generation streams remain excluded.
use super::*;
use crate::{GenerationRevision, model::Area};

type Emission = (String, Vec2, f64);
fn rand(state: &mut u32, width: u32) -> u32 {
    *state = state.wrapping_mul(0x0808_8405).wrapping_add(1);
    1 + ((u64::from(*state) * u64::from(width)) >> 32) as u32
}
fn top(result: &mut Vec<Emission>, background: &str, origin: Vec2, offset: f64) {
    result.push((
        background.into(),
        Vec2::new(origin.x, origin.y - 16.),
        offset,
    ));
}
fn lush(state: &mut u32, result: &mut Vec<Emission>, origin: Vec2) {
    let offset = if rand(state, 8) == 1 {
        32.
    } else if rand(state, 3) < 3 {
        0.
    } else {
        16.
    };
    top(result, "bgCaveTop2", origin, offset);
    // The actual HardBlock below prevents the separate lower fringe branch.
}
fn temple(state: &mut u32, result: &mut Vec<Emission>, origin: Vec2) {
    if rand(state, 4) == 1 {
        top(result, "bgCaveTop4", origin, 0.);
    } else if rand(state, 4) == 1 {
        top(result, "bgCaveTop4", origin, 16.);
    }
}
fn emissions(world: &World, first: u64) -> Vec<Emission> {
    world
        .entities
        .iter()
        .filter(|e| e.id >= first)
        .map(|e| {
            if e.object == "oGenerationDecoration" {
                assert_eq!(e.value("source_w"), 16.);
                assert_eq!(e.value("source_h"), 16.);
                assert_eq!(e.value("depth"), 3.);
                (e.sprite.clone(), e.pos, e.value("source_x"))
            } else {
                assert_eq!(e.object, "oIceBottom", "unexpected Setup child");
                (e.object.clone(), e.pos, e.value("drip_alarm"))
            }
        })
        .collect()
}
fn mixed(revision: GenerationRevision, source_order: bool) {
    // scrRoomGen2 and scrRoomGenMarket can both create Temple and Lush.
    for seed in 0..64 {
        let mut world = World::new(8, 8, Area::Jungle, 5);
        world.set_tile(1, 1, "oTemple");
        world.set_tile(5, 3, "oLush");
        world.set_tile(5, 4, "oHardBlock");
        let mut rng = Rng::new(seed);
        create_generated(&mut world, &mut rng);
        let first = world.next_id;
        let mut state = rng.state_bits() as u32;
        let mut expected = Vec::new();
        if source_order {
            lush(&mut state, &mut expected, Vec2::new(80., 48.));
            temple(&mut state, &mut expected, Vec2::new(16., 16.));
        } else {
            temple(&mut state, &mut expected, Vec2::new(16., 16.));
            lush(&mut state, &mut expected, Vec2::new(80., 48.));
        }
        setup_generated_with_revision(&mut world, &mut rng, true, revision, false);
        assert_eq!(emissions(&world, first), expected, "seed {seed}");
        assert_eq!(rng.state_bits(), u64::from(state), "seed {seed}");
    }
}
#[test]
fn source_setup_finishes_lush_family_before_temple() {
    mixed(GenerationRevision::SourceClassicV2, true);
}
#[test]
fn historical_setup_retains_row_major_mixed_family_order() {
    mixed(GenerationRevision::HistoricalV1, false);
}
#[test]
fn source_setup_uses_same_family_parent_birth_order_not_coordinates() {
    for seed in 0..64 {
        let mut world = World::new(8, 8, Area::Mines, 1);
        world.set_tile(5, 3, "oBrick");
        world.set_tile(1, 1, "oBrick");
        assert!(
            world.tile(5, 3).unwrap().creation_serial < world.tile(1, 1).unwrap().creation_serial
        );
        let mut rng = Rng::new(seed);
        create_generated(&mut world, &mut rng);
        let first = world.next_id;
        let mut state = rng.state_bits() as u32;
        let mut expected = Vec::new();
        for origin in [Vec2::new(80., 48.), Vec2::new(16., 16.)] {
            let offset = if rand(&mut state, 3) < 3 { 0. } else { 16. };
            top(&mut expected, "bgCaveTop", origin, offset);
        }
        setup_generated_with_revision(
            &mut world,
            &mut rng,
            true,
            GenerationRevision::SourceClassicV2,
            false,
        );
        assert_eq!(emissions(&world, first), expected, "seed {seed}");
        assert_eq!(rng.state_bits(), u64::from(state), "seed {seed}");
    }
}
#[test]
fn source_setup_ice_roll_and_actual_drip_birth_precede_temple_fringe() {
    let mut observed_drip = false;
    for seed in 0..128 {
        let mut world = World::new(8, 8, Area::Ice, 9);
        world.set_tile(1, 1, "oTemple");
        world.set_tile(5, 3, "oIce");
        let mut rng = Rng::new(seed);
        create_generated(&mut world, &mut rng);
        let first = world.next_id;
        let mut state = rng.state_bits() as u32;
        let mut expected = Vec::new();
        if rand(&mut state, 20) == 1 {
            observed_drip = true;
            let alarm = 19 + rand(&mut state, 381); // rand(20,400), child own Create
            expected.push(("oIceBottom".into(), Vec2::new(80., 64.), f64::from(alarm)));
        }
        temple(&mut state, &mut expected, Vec2::new(16., 16.));
        setup_generated_with_revision(
            &mut world,
            &mut rng,
            true,
            GenerationRevision::SourceClassicV2,
            false,
        );
        if let Some(child) = world.entities.iter().find(|e| e.object == "oIceBottom") {
            assert_eq!(child.text.get("type").map(String::as_str), Some(""));
            assert_eq!(child.value("blinkToggle"), 0.0);
        }
        assert_eq!(emissions(&world, first), expected, "seed {seed}");
        assert_eq!(rng.state_bits(), u64::from(state), "seed {seed}");
    }
    assert!(observed_drip, "fixed seed domain must exercise child birth");
}

// The earlier independently documented exterior omission is now a selected
// regression against the real synchronous boundary constructor. UNRUN.
#[test]
fn source_setup_visits_actual_exterior_boundary_parent() {
    let mut world = World::new(42, 34, Area::Mines, 1);
    let mut rng = Rng::new(42);
    // scrInitLevel's i=42,j=0 birth: x=672 outside 42-column tile storage.
    // Use its real synchronous constructor and caller overrides, no manual
    // instance identity, sprite, initialization, or source RNG assignment.
    create_source_boundary(&mut world, &mut rng, 42, 0, "oBrick", false, true);
    let parent = world
        .entities
        .iter()
        .find(|e| e.object == "oBrick")
        .unwrap()
        .id;
    assert!(world.tile(42, 0).is_none());
    assert_eq!(world.entity(parent).unwrap().pos, Vec2::new(672., 0.));
    assert_eq!(world.entity(parent).unwrap().sprite, "sBrick");
    let before = rng.state_bits();
    setup_generated_with_revision(
        &mut world,
        &mut rng,
        true,
        GenerationRevision::SourceClassicV2,
        false,
    );
    // Source: y==0 makes up true; no Brick/HardBlock below and y<528 makes
    // down false. No top fringe RNG executes. The same parent becomes Down.
    let actual = world.entity(parent).unwrap();
    assert_eq!(actual.sprite, "sBrickDown");
    assert_eq!(actual.value("invincible"), 1.);
    assert_eq!(actual.value("terrain_setup_initialized"), 1.);
    assert_eq!(rng.state_bits(), before);
}

#[test]
fn source_setup_merges_actual_overlapping_entities_and_tile_births() {
    // Incoming concrete parents use actual World::spawn. This control scopes
    // Setup only: it does not certify those actors' earlier own Create body.
    for seed in 0..64 {
        let mut world = World::new(8, 8, Area::Mines, 1);
        let first_entity = world.spawn("oBrick", Vec2::new(80., 48.));
        world.set_tile(1, 1, "oBrick");
        let duplicate = world.spawn("oBrick", Vec2::new(16., 16.));
        let mut rng = Rng::new(seed);
        create_generated(&mut world, &mut rng);
        let stored = metadata(&mut world, 1, 1);
        let identities: Vec<_> = [first_entity, duplicate, stored]
            .into_iter()
            .map(|id| {
                let entity = world.entity(id).unwrap();
                (
                    entity.id,
                    entity.creation_serial,
                    entity.pos,
                    entity.previous_pos,
                    entity.alarms,
                )
            })
            .collect();
        let first_child = world.next_id;
        let mut state = rng.state_bits() as u32;
        let mut expected = Vec::new();
        for origin in [
            Vec2::new(80., 48.),
            Vec2::new(16., 16.),
            Vec2::new(16., 16.),
        ] {
            let offset = if rand(&mut state, 3) < 3 { 0. } else { 16. };
            top(&mut expected, "bgCaveTop", origin, offset);
        }
        setup_generated_with_revision(
            &mut world,
            &mut rng,
            true,
            GenerationRevision::SourceClassicV2,
            false,
        );
        assert_eq!(emissions(&world, first_child), expected, "seed {seed}");
        assert_eq!(rng.state_bits(), u64::from(state));
        for (id, serial, pos, previous, alarms) in identities {
            let actual = world.entity(id).unwrap();
            assert_eq!(
                (
                    actual.id,
                    actual.creation_serial,
                    actual.pos,
                    actual.previous_pos,
                    actual.alarms
                ),
                (id, serial, pos, previous, alarms)
            );
            assert_eq!(actual.value("terrain_setup_initialized"), 1.);
        }
        for id in [first_entity, duplicate] {
            let actual = world.entity(id).unwrap();
            assert_eq!(actual.sprite, "sCaveUp2");
            assert!(actual.mask_contains(actual.pos, crate::CollisionPrecision::BoundingBox));
            assert!(!actual.mask_contains(
                Vec2::new(actual.pos.x, actual.pos.y + 16.),
                crate::CollisionPrecision::BoundingBox
            ));
        }
        assert_eq!(
            world.entity(stored).unwrap().text["terrain_sprite"],
            "sCaveUp2"
        );
        assert_eq!(
            world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == "oTerrainMetadata")
                .count(),
            1
        );
        assert_eq!(
            world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == "oBrick")
                .count(),
            2
        );
    }
}

#[test]
fn source_setup_stored_parent_observes_concrete_family_neighbor_masks() {
    let mut world = World::new(8, 8, Area::Mines, 1);
    world.set_tile(2, 2, "oBrick");
    let neighbor = world.spawn("oBrick", Vec2::new(32., 16.));
    let mut rng = Rng::new(1);
    create_generated(&mut world, &mut rng);
    let stored = metadata(&mut world, 2, 2);
    setup_generated_with_revision(
        &mut world,
        &mut rng,
        false,
        GenerationRevision::SourceClassicV2,
        false,
    );
    let actual = world.entity(stored).unwrap();
    assert_eq!(actual.value("up"), 1.);
    assert_eq!(actual.value("down"), 0.);
    assert_eq!(actual.text["terrain_sprite"], "sBrickDown");
    // Earlier stored receiver then later concrete receiver both retain the
    // source family connection as the real neighbor's sprite is updated.
    assert_eq!(world.entity(neighbor).unwrap().sprite, "sCaveUp");
}

#[test]
fn source_setup_receiver_specific_hard_boundaries_use_raw_coordinates() {
    for (family, y, sprite, up, down) in [
        ("oBrick", 0., "sBrickDown", 1., 0.),
        ("oBrick", 528., "sCaveUp", 0., 1.),
        ("oLush", 528., "sLushUp", 0., 1.),
        ("oDark", 528., "sDarkUp", 0., 1.),
        ("oTemple", 528., "sTempleUp5", 0., 1.),
        ("oIce", 0., "sIceBlock", 0., 0.),
        ("oIce", 528., "sIceBlock", 0., 0.),
    ] {
        let mut world = World::new(42, 34, Area::Mines, 1);
        let id = world.spawn(family, Vec2::new(672., y));
        let mut rng = Rng::new(42);
        setup_generated_with_revision(
            &mut world,
            &mut rng,
            false,
            GenerationRevision::SourceClassicV2,
            false,
        );
        let actual = world.entity(id).unwrap();
        assert_eq!(actual.sprite, sprite, "{family}/{y}");
        assert_eq!(
            (actual.value("up"), actual.value("down")),
            (up, down),
            "{family}/{y}"
        );
        assert_eq!(actual.pos, Vec2::new(672., y));
    }
    // y=-0.5 remains distinct from source y==0: no flooring to a tile cell.
    let mut world = World::new(42, 34, Area::Mines, 1);
    let id = world.spawn("oBrick", Vec2::new(672.25, -0.5));
    setup_generated_with_revision(
        &mut world,
        &mut Rng::new(42),
        false,
        GenerationRevision::SourceClassicV2,
        false,
    );
    assert_eq!(world.entity(id).unwrap().sprite, "sCaveUp2");
    assert_eq!(world.entity(id).unwrap().pos, Vec2::new(672.25, -0.5));
}

#[test]
fn source_setup_brick_hardblock_and_temple_fake_probe_are_family_specific() {
    for family in ["oBrick", "oDark", "oLush"] {
        let mut world = World::new(8, 8, Area::Mines, 1);
        let id = world.spawn(family, Vec2::new(32., 32.));
        world.spawn("oHardBlock", Vec2::new(32., 16.));
        setup_generated_with_revision(
            &mut world,
            &mut Rng::new(42),
            false,
            GenerationRevision::SourceClassicV2,
            false,
        );
        assert_eq!(
            world.entity(id).unwrap().value("up"),
            if family == "oBrick" { 1. } else { 0. }
        );
    }
    let mut world = World::new(8, 8, Area::Temple, 13);
    let id = world.spawn("oTemple", Vec2::new(32., 32.));
    world.spawn("oTempleFake", Vec2::new(32., 48.));
    let mut rng = Rng::new(42);
    let before = rng.state_bits();
    setup_generated_with_revision(
        &mut world,
        &mut rng,
        false,
        GenerationRevision::SourceClassicV2,
        false,
    );
    let actual = world.entity(id).unwrap();
    assert_eq!((actual.value("up"), actual.value("down")), (1., 1.));
    assert_eq!(
        actual.sprite, "sTemple",
        "the literal below-Fake up check leaves the covered parent sprite"
    );
    assert_eq!(rng.state_bits(), before);
}

#[test]
fn source_setup_skips_inactive_concrete_parents_and_neighbor_queries() {
    let mut world = World::new(8, 8, Area::Mines, 1);
    let parent = world.spawn("oBrick", Vec2::new(32., 32.));
    let neighbor = world.spawn("oBrick", Vec2::new(32., 16.));
    world.entity_mut(neighbor).unwrap().engine_active = false;
    setup_generated_with_revision(
        &mut world,
        &mut Rng::new(42),
        false,
        GenerationRevision::SourceClassicV2,
        false,
    );
    assert_eq!(world.entity(parent).unwrap().sprite, "sCaveUp2");
    assert_eq!(world.entity(neighbor).unwrap().sprite, "sBrick");
    assert_eq!(
        world
            .entity(neighbor)
            .unwrap()
            .value("terrain_setup_initialized"),
        0.
    );
}
