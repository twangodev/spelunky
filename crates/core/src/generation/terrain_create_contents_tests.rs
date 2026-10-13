use super::*;
use crate::{Area, CollisionPrecision, Game, InputFrame, PlayerState, RulesPreset};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
};

// Independently transcribed scrGenerateItem Set2, in source switch order. The
// oracle uses the supplied port RNG, not a claim of original GM8 seed parity.
const SOURCE_SET2: [(&str, f64, f64); 19] = [
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

fn source_selector(
    rng: &mut Rng,
    temple: bool,
    city: bool,
    interior: bool,
) -> Option<(&'static str, f64, f64)> {
    let n = 1 + rng.range(100);
    if city || n < 30 || !interior {
        return None;
    }
    let sapphire = if temple { 60 } else { 80 };
    let emerald = if temple { 80 } else { 100 };
    let ruby = if temple { 100 } else { 120 };
    if 1 + rng.range(sapphire) == 1 {
        Some(("oSapphireBig", 0.0, 0.0))
    } else if 1 + rng.range(emerald) == 1 {
        Some(("oEmeraldBig", 0.0, 0.0))
    } else if 1 + rng.range(ruby) == 1 {
        Some(("oRubyBig", 0.0, 0.0))
    } else if 1 + rng.range(1200) == 1 {
        Some(SOURCE_SET2[rng.range(19) as usize])
    } else {
        None
    }
}

fn fixtures(temple: bool) -> BTreeMap<&'static str, u32> {
    let mut found = BTreeMap::new();
    // Select ONLY on literal source conditions, before exercising production.
    for seed in 0..500_000 {
        if let Some((object, _, _)) = source_selector(&mut Rng::new(seed), temple, false, true) {
            found.entry(object).or_insert(seed);
        }
        if found.len() == 22 {
            return found;
        }
    }
    panic!(
        "bounded literal selector did not cover all 22 children: {}",
        found.len()
    );
}

fn one_parent(seed: u32, temple: bool, own_ruby: bool) -> (World, Rng, u64) {
    let mut world = World::new(20, 16, Area::Jungle, 5);
    world.set_tile(6, 6, if temple { "oTemple" } else { "oLush" });
    let parent = metadata(&mut world, 6, 6);
    if own_ruby {
        let e = world.entity_mut(parent).unwrap();
        e.data.insert("embedded_treasure".into(), 1.0);
        e.text.insert("treasure".into(), "Big Ruby".into());
    }
    let mut rng = Rng::new(seed);
    create_generated(&mut world, &mut rng);
    (world, rng, parent)
}

#[derive(Clone)]
pub(super) struct CreateTrace {
    object: String,
    origin: Vec2,
    before: Rng,
    after: Rng,
    city: bool,
    interior: bool,
    boundary: bool,
    first_child_id: u64,
    children: Vec<Entity>,
    metadata: Entity,
    cleanup: Vec<SourceCleanupPrediction>,
}

#[derive(Clone, Debug)]
struct SourceCleanupPrediction {
    child_id: u64,
    destroys: bool,
    point_gate: bool,
    first_precise: Option<(u64, String, bool)>,
    alive_after_cleanup: Option<bool>,
}
thread_local! {
    static TRACES: RefCell<Option<Vec<CreateTrace>>> = const { RefCell::new(None) };
}

pub(super) fn observe_before(
    world: &World,
    rng: &Rng,
    x: usize,
    y: usize,
    id: u64,
    custom: bool,
) -> Option<CreateTrace> {
    if custom
        || !matches!(
            world.tiles[y * world.width + x].object.as_str(),
            "oLush" | "oTemple"
        )
        || !TRACES.with(|traces| traces.borrow().is_some())
    {
        return None;
    }
    let parent = world.entity(id).unwrap().clone();
    Some(CreateTrace {
        object: world.tiles[y * world.width + x].object.clone(),
        origin: Vec2::new(x as f64 * 16.0, y as f64 * 16.0),
        before: rng.clone(),
        after: rng.clone(),
        city: world.name == "City of Gold",
        interior: x > 0 && y > 0 && x + 1 < world.width && y + 1 < world.height,
        boundary: parent.value("source_boundary") != 0.0,
        first_child_id: world.next_id,
        children: Vec::new(),
        metadata: parent,
        cleanup: Vec::new(),
    })
}

pub(super) fn observe_after(world: &World, rng: &Rng, trace: Option<CreateTrace>) {
    if let Some(mut trace) = trace {
        trace.after = rng.clone();
        trace.children = world
            .entities
            .iter()
            .filter(|e| e.alive && e.id >= trace.first_child_id)
            .cloned()
            .collect();
        trace.metadata = world.entity(trace.metadata.id).unwrap().clone();
        TRACES.with(|traces| traces.borrow_mut().as_mut().unwrap().push(trace));
    }
}

// Independent literal scrInitLevel259–267 oracle. The generated room has
// never executed Level deactivation: source family entries are active and
// retain creation order across the tile/entity storage adapter. These probes
// use the shared source mask primitives, never init_cleanup::cleanup or its
// output as the expected decision.
pub(super) fn observe_init_cleanup(world: &World, after: bool) {
    TRACES.with(|traces| {
        let mut slot = traces.borrow_mut();
        let Some(traces) = slot.as_mut() else {
            return;
        };
        for trace in traces {
            if after {
                for predicted in &mut trace.cleanup {
                    let child = world
                        .entities
                        .iter()
                        .find(|e| e.id == predicted.child_id)
                        .expect("cleanup retains source identity as a live/dead record");
                    predicted.alive_after_cleanup = Some(child.alive);
                    assert_eq!(
                        child.alive, !predicted.destroys,
                        "literal cleanup phase decision: {predicted:?}"
                    );
                }
                continue;
            }
            assert!(
                trace.cleanup.is_empty(),
                "one pre-cleanup observation per room"
            );
            for birth in &trace.children {
                let child = world
                    .entity(birth.id)
                    .expect("source Create child must still be alive before final cleanup");
                assert_eq!(child.creation_serial, birth.creation_serial);
                assert_eq!(child.object, birth.object);
                let mut source_family = Vec::new();
                for (index, tile) in world.tiles.iter().enumerate() {
                    if !crate::catalog::is_a(&tile.object, "oSolid") {
                        continue;
                    }
                    assert!(
                        tile.engine_active,
                        "no source Level deactivation before cleanup"
                    );
                    let x = (index % world.width) as i32;
                    let y = (index / world.width) as i32;
                    let invincible = world.entities.iter().any(|e| {
                        e.alive
                            && e.object == "oTerrainMetadata"
                            && (e.pos.x / 16.).floor() as i32 == x
                            && (e.pos.y / 16.).floor() as i32 == y
                            && e.value("invincible") != 0.
                    });
                    source_family.push((
                        tile.creation_serial,
                        format!("tile:{index}:{}", tile.object),
                        invincible,
                        world.tile_collision_mask(x, y),
                    ));
                }
                for solid in world
                    .entities
                    .iter()
                    .filter(|e| e.alive && crate::catalog::is_a(&e.object, "oSolid"))
                {
                    assert!(
                        solid.engine_active,
                        "no source Level deactivation before cleanup"
                    );
                    source_family.push((
                        solid.creation_serial,
                        format!("entity:{}:{}", solid.id, solid.object),
                        solid.value("invincible") != 0.,
                        solid.collision_mask(),
                    ));
                }
                source_family.sort_by_key(|receiver| receiver.0);
                let point_gate = source_family.iter().any(|receiver| {
                    receiver
                        .3
                        .contains(child.pos, CollisionPrecision::BoundingBox)
                });
                let child_mask = child.collision_mask();
                let first_precise = source_family
                    .iter()
                    .find(|receiver| child_mask.intersects_mask(&receiver.3))
                    .map(|receiver| (receiver.0, receiver.1.clone(), receiver.2));
                let destroys = crate::catalog::is_a(&child.object, "oTreasure")
                    && point_gate
                    && first_precise.as_ref().is_some_and(|receiver| receiver.2);
                trace.cleanup.push(SourceCleanupPrediction {
                    child_id: child.id,
                    destroys,
                    point_gate,
                    first_precise,
                    alive_after_cleanup: None,
                });
            }
        }
    });
}

#[test]
fn live_terrain_create_all_three_gems_exist_before_population_and_do_not_reroll() {
    for temple in [false, true] {
        let seeds = fixtures(temple);
        for (object, value, kind) in [
            ("oSapphireBig", 1200.0, "Big Sapphire"),
            ("oEmeraldBig", 800.0, "Big Emerald"),
            ("oRubyBig", 1600.0, "Big Ruby"),
        ] {
            let seed = seeds[object];
            let mut expected = Rng::new(seed);
            assert_eq!(
                source_selector(&mut expected, temple, false, true),
                Some((object, 0.0, 0.0))
            );
            let (mut world, mut actual, parent) = one_parent(seed, temple, false);
            let child = world
                .entities
                .iter()
                .find(|e| e.alive && e.object == object)
                .expect("source Create child must already be live")
                .clone();
            assert_eq!(child.pos, Vec2::new(104.0, 104.0));
            assert_eq!(child.previous_pos, child.pos);
            assert_eq!(child.state, 1);
            assert_eq!(child.value("canCollect"), 0.0);
            assert_eq!(child.alarms[0], 20);
            assert_eq!(child.value("value"), value);
            assert_eq!(child.text["type"], kind);
            assert_eq!(child.value("yOff"), 4.0);
            assert!(crate::catalog::is_a(object, "oTreasure"));
            assert!(child.mask_contains(child.pos, CollisionPrecision::BoundingBox));
            assert_eq!(
                world.entity(parent).unwrap().value("embedded_treasure"),
                0.0
            );
            assert_eq!(
                actual.clone().next_u64(),
                expected.clone().next_u64(),
                "child constructor adds no draws"
            );
            let serial = child.creation_serial;
            create_generated(&mut world, &mut actual);
            assert_eq!(
                world
                    .entities
                    .iter()
                    .filter(|e| e.alive && e.object == object)
                    .count(),
                1
            );
            assert_eq!(world.entity(child.id).unwrap().creation_serial, serial);
            assert_eq!(
                actual.next_u64(),
                expected.next_u64(),
                "same parent cannot reroll"
            );
        }
    }
}

#[test]
fn live_terrain_create_all_nineteen_set2_items_have_source_offsets_free_prices_and_next_draw() {
    for temple in [false, true] {
        let seeds = fixtures(temple);
        for (object, dx, dy) in SOURCE_SET2 {
            let seed = seeds[object];
            let mut expected = Rng::new(seed);
            assert_eq!(
                source_selector(&mut expected, temple, false, true),
                Some((object, dx, dy))
            );
            let (world, mut actual, parent) = one_parent(seed, temple, false);
            let item = world
                .entities
                .iter()
                .find(|e| e.alive && e.object == object)
                .expect("selected source Set2 child must exist during Create");
            assert_eq!(item.pos, Vec2::new(104.0 + dx, 104.0 + dy), "{object}");
            assert!(crate::catalog::is_a(object, "oItem"));
            assert!(!crate::catalog::is_a(object, "oTreasure"));
            assert_eq!(item.value("cost"), 0.0);
            assert_eq!(item.value("forSale"), 0.0);
            assert_eq!(item.value("depth"), 101.0);
            assert_eq!(item.value("active"), 1.0);
            assert_eq!(item.value("new"), 1.0);
            assert_eq!(
                item.state, 0,
                "makeActive does not invent an Item ACTIVE state"
            );
            assert_eq!(
                world.entity(parent).unwrap().value("embedded_treasure"),
                0.0
            );
            assert_eq!(
                actual.next_u64(),
                expected.next_u64(),
                "{object}: fixed child Create has no RNG"
            );
        }
    }
}

#[test]
fn live_terrain_create_runtime_child_context_precedes_caller_overrides_and_is_idempotent() {
    // Independent source base prices. Set2 overwrites these after instance_create.
    let prices = [
        20000.0, 12000.0, 15000.0, 8000.0, 10000.0, 8000.0, 8000.0, 2000.0, 5000.0, 4000.0, 3000.0,
        5000.0, 4000.0, 7000.0, 10000.0, 1000.0, 3000.0, 2000.0, 2500.0,
    ];
    for spectacles in [false, true] {
        let mut game = Game::shell(9, RulesPreset::Classic);
        if spectacles {
            game.player.equipment.insert("Spectacles".into());
        }
        for ((object, _, _), price) in SOURCE_SET2.into_iter().zip(prices) {
            let before = game.rng.clone().next_u64();
            let id = game.spawn(object, Vec2::new(104.0, 104.0));
            let item = game.world.entity_mut(id).unwrap();
            assert_eq!(
                item.value("cost"),
                price,
                "{object}: own Create before producer override"
            );
            assert_eq!(item.value("depth"), if spectacles { 51.0 } else { 101.0 });
            assert_eq!(item.value("new"), 1.0);
            assert_eq!(item.value("active"), 1.0);
            assert_eq!(item.value("forSale"), 0.0);
            item.data.insert("cost".into(), 0.0);
            item.data.insert("forSale".into(), 1.0);
            item.held = true;
            item.text.insert("type".into(), "caller override".into());
            crate::items::create_instance(&mut game, id);
            let item = game.world.entity(id).unwrap();
            assert_eq!(item.value("cost"), 0.0);
            assert_eq!(item.value("forSale"), 1.0);
            assert!(item.held);
            assert_eq!(item.text["type"], "caller override");
            assert_eq!(game.rng.clone().next_u64(), before);
        }
    }
}

#[test]
fn live_terrain_create_own_ruby_label_coexists_and_clean_support_removal_keeps_child_identity() {
    for temple in [false, true] {
        let seed = fixtures(temple)["oSapphireBig"];
        let (mut world, rng, parent) = one_parent(seed, temple, temple);
        let child = world
            .entities
            .iter()
            .find(|e| e.alive && e.object == "oSapphireBig")
            .expect("Create birth must coexist with independent Destroy label")
            .clone();
        if temple {
            assert_eq!(world.entity(parent).unwrap().text["treasure"], "Big Ruby");
        }
        let source = {
            let meta = world.entity_mut(parent).unwrap();
            meta.data.insert("cleanDeath".into(), 1.0);
            let mut source = Entity::new(
                0,
                if temple { "oTemple" } else { "oLush" },
                Vec2::new(96.0, 96.0),
            );
            source.data = meta.data.clone();
            source.text = meta.text.clone();
            source
        };
        let mut game = Game::shell(0, RulesPreset::Classic);
        game.world = world;
        game.rng = rng;
        // Exercise the existing generation clean-Own-Destroy adapter, then
        // exact original support retirement. This is not a whole spear-phase test.
        let before = game.rng.clone().next_u64();
        crate::items::explosives::destroy_temple_support_clean(
            &mut game.world,
            &mut game.rng,
            &source,
        );
        game.world.set_tile(6, 6, "");
        assert_eq!(game.rng.clone().next_u64(), before);
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == "oSapphireBig")
                .count(),
            1
        );
        let survivor = game.world.entity(child.id).unwrap();
        assert_eq!(survivor.alarms, child.alarms);
        assert_eq!(survivor.state, child.state);
        assert_eq!(survivor.creation_serial, child.creation_serial);
        assert!(
            !game
                .world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oRubyBig")
        );
    }
}

#[test]
fn live_terrain_create_ordinary_game_step_support_explosion_does_not_clone_or_reset_child() {
    for temple in [false, true] {
        let seed = fixtures(temple)["oSapphireBig"];
        let (mut world, rng, parent) = one_parent(seed, temple, temple);
        let child = world
            .entities
            .iter()
            .find(|e| e.alive && e.object == "oSapphireBig")
            .expect("live child before ordinary removal")
            .clone();
        world
            .entity_mut(parent)
            .unwrap()
            .data
            .insert("cleanDeath".into(), 1.0);
        for x in 0..20 {
            world.set_tile(x, 13, "oHardBlock");
        }
        let mut game = Game::shell(0, RulesPreset::Classic);
        game.world = world;
        game.rng = rng;
        game.player.pos = Vec2::new(40.0, 200.0);
        game.player.previous_pos = game.player.pos;
        game.player.state = PlayerState::Standing;
        game.player.grounded = true;
        game.spawn("oExplosion", Vec2::new(104.0, 104.0));
        game.step(&InputFrame::default());
        assert!(
            game.world.tile(6, 6).unwrap().object.is_empty(),
            "ordinary Collision destroys original support"
        );
        let survivor = game
            .world
            .entity(child.id)
            .expect("same source Create identity survives support removal");
        assert_eq!(survivor.creation_serial, child.creation_serial);
        assert_eq!(
            survivor.alarms[0], 19,
            "Create alarm advances; removal cannot restart it"
        );
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == "oSapphireBig")
                .count(),
            1
        );
        assert!(
            !game
                .world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oRubyBig")
        );
    }
}

#[test]
fn live_terrain_create_actual_next_level_births_follow_literal_parent_oracle_and_player_context() {
    let mut observed = BTreeSet::new();
    // No numeric seed claims: inspect the actual incoming port RNG at each
    // parent Create. Coverage requires natural gems AND Set2 items in both
    // context arms, rather than filtering by production test success.
    for spectacles in [false, true] {
        let mut kinds = BTreeSet::new();
        for seed in 0..96 {
            let mut game = Game::shell(seed, RulesPreset::Classic);
            game.flags.insert("cemetery".into());
            game.flags.insert("no_dark_level".into());
            game.data.insert("next_level".into(), 5.0);
            if spectacles {
                game.player.equipment.insert("Spectacles".into());
            }
            TRACES.with(|traces| *traces.borrow_mut() = Some(Vec::new()));
            game.next_level();
            let traces = TRACES.with(|traces| traces.borrow_mut().take().unwrap());
            let mut prior_child_serial = 0;
            for trace in traces {
                // Boundary Create draws remain an explicit separate port gap.
                if trace.boundary {
                    continue;
                }
                let mut source = trace.before.clone();
                let selected = source_selector(
                    &mut source,
                    trace.object == "oTemple",
                    trace.city,
                    trace.interior,
                );
                assert_eq!(trace.after.clone().next_u64(), source.next_u64());
                if let Some((object, dx, dy)) = selected {
                    assert_eq!(
                        trace.children.len(),
                        1,
                        "{object} selected at {:?} before cemetery",
                        trace.origin
                    );
                    let child = &trace.children[0];
                    assert_eq!(child.object, object);
                    assert_eq!(
                        child.pos,
                        Vec2::new(trace.origin.x + 8.0 + dx, trace.origin.y + 8.0 + dy)
                    );
                    assert!(child.creation_serial > prior_child_serial);
                    prior_child_serial = child.creation_serial;
                    if crate::catalog::is_a(object, "oItem") {
                        assert_eq!(child.value("depth"), if spectacles { 51.0 } else { 101.0 });
                        assert_eq!(child.value("cost"), 0.0);
                        assert_eq!(child.value("forSale"), 0.0);
                        kinds.insert("item");
                    } else {
                        assert!(crate::catalog::is_a(object, "oTreasure"));
                        assert!(child.mask_contains(
                            Vec2::new(trace.origin.x + 8.0, trace.origin.y + 8.0),
                            CollisionPrecision::BoundingBox
                        ));
                        assert_eq!(child.alarms[0], 20);
                        kinds.insert("gem");
                    }
                    let cleanup = trace
                        .cleanup
                        .iter()
                        .find(|prediction| prediction.child_id == child.id)
                        .expect("selected child has an independent pre-cleanup source prediction");
                    assert_eq!(
                        cleanup.alive_after_cleanup,
                        Some(!cleanup.destroys),
                        "source cleanup phase must be observed: {cleanup:?}"
                    );
                    assert_eq!(
                        game.world.entity(child.id).is_some(),
                        !cleanup.destroys,
                        "completed initialization preserves the literal cleanup outcome: {cleanup:?}"
                    );
                    if let Some(survivor) = game.world.entity(child.id) {
                        assert_eq!(survivor.creation_serial, child.creation_serial);
                        assert_eq!(survivor.object, child.object);
                        assert_eq!(survivor.pos, child.pos);
                    }
                    observed.insert(object.to_owned());
                } else {
                    assert!(trace.children.is_empty());
                }
            }
            // Stop on source-oracle coverage, never on production success or
            // caught assertion outcomes. All births of every visited room were
            // checked above; no scene/actor mutation selects a convenient level.
            if kinds.len() == 2 && observed.len() >= 4 {
                break;
            }
        }
        assert_eq!(kinds, BTreeSet::from(["gem", "item"]));
    }
    assert!(
        observed.len() >= 4,
        "natural coverage must include several independently selected children"
    );
}

#[test]
fn live_terrain_create_city_gold_and_bounds_retain_literal_parent_draw_guards() {
    for temple in [false, true] {
        for seed in 0..64 {
            for (city, x, y) in [
                (false, 0, 6),
                (false, 6, 0),
                (false, 19, 6),
                (false, 6, 15),
                (temple, 6, 6),
            ] {
                if !temple && x == 6 && y == 6 {
                    continue;
                }
                let mut world = World::new(20, 16, Area::Jungle, 5);
                if city {
                    world.name = "City of Gold".into();
                }
                world.set_tile(x, y, if temple { "oTemple" } else { "oLush" });
                let mut expected = Rng::new(seed);
                assert!(
                    source_selector(
                        &mut expected,
                        temple,
                        city,
                        x > 0 && y > 0 && x + 1 < 20 && y + 1 < 16
                    )
                    .is_none()
                );
                let mut actual = Rng::new(seed);
                create_generated(&mut world, &mut actual);
                assert_eq!(actual.next_u64(), expected.next_u64());
                assert!(!world.entities.iter().any(|e| e.alive
                    && (crate::catalog::is_a(&e.object, "oItem")
                        || crate::catalog::is_a(&e.object, "oTreasure"))));
            }
        }
        let seed = (0..128)
            .find(|&seed| 1 + Rng::new(seed).range(100) < 30)
            .unwrap();
        let (world, mut actual, _) = one_parent(seed, temple, false);
        let mut expected = Rng::new(seed);
        expected.range(100);
        assert_eq!(actual.next_u64(), expected.next_u64());
        assert!(!world.entities.iter().any(|e| e.alive
            && (crate::catalog::is_a(&e.object, "oItem")
                || crate::catalog::is_a(&e.object, "oTreasure"))));
    }
}

#[test]
fn live_terrain_create_template_normal_destroy_adds_own_ruby_without_recreating_create_child() {
    let seed = fixtures(true)["oSapphireBig"];
    let (world, rng, _) = one_parent(seed, true, true);
    let child = world
        .entities
        .iter()
        .find(|e| e.alive && e.object == "oSapphireBig")
        .expect("live independent Create child")
        .clone();
    let mut game = Game::shell(0, RulesPreset::Classic);
    game.world = world;
    game.rng = rng;
    assert!(crate::items::explosives::destroy_terrain_self(
        &mut game, 6, 6
    ));
    assert_eq!(
        game.world
            .entities
            .iter()
            .filter(|e| e.alive && e.object == "oSapphireBig")
            .count(),
        1
    );
    assert!(game.world.entity(child.id).is_some());
    assert_eq!(
        game.world
            .entities
            .iter()
            .filter(|e| e.alive && e.object == "oRubyBig")
            .count(),
        1,
        "OWN Destroy label is independent additional loot"
    );
    assert!(!crate::items::explosives::destroy_terrain_self(
        &mut game, 6, 6
    ));
    assert_eq!(
        game.world
            .entities
            .iter()
            .filter(|e| e.alive && e.object == "oRubyBig")
            .count(),
        1
    );
}

#[test]
fn live_terrain_create_legacy_deferred_and_custom_paths_stay_once_only() {
    let seed = fixtures(false)["oSapphireBig"];
    let mut world = World::new(20, 16, Area::Jungle, 5);
    world.set_tile(6, 6, "oLush");
    let mut rng = Rng::new(seed);
    initialize_custom(&mut world, &mut rng);
    assert!(
        !world
            .entities
            .iter()
            .any(|e| e.alive && e.object == "oSapphireBig"),
        "custom tile adapter is explicitly unchanged"
    );
    let parent = metadata(&mut world, 6, 6);
    assert_eq!(
        world.entity(parent).unwrap().text["treasure"],
        "oSapphireBig"
    );
    let mut game = Game::shell(0, RulesPreset::Classic);
    game.world = world;
    game.rng = rng;
    assert!(crate::items::explosives::destroy_terrain_self(
        &mut game, 6, 6
    ));
    assert_eq!(
        game.world
            .entities
            .iter()
            .filter(|e| e.alive && e.object == "oSapphireBig")
            .count(),
        1
    );
    assert!(!crate::items::explosives::destroy_terrain_self(
        &mut game, 6, 6
    ));
    assert_eq!(
        game.world
            .entities
            .iter()
            .filter(|e| e.alive && e.object == "oSapphireBig")
            .count(),
        1
    );
}

#[test]
fn live_terrain_create_shared_birth_trace_finishes_constructor_before_set2_override() {
    for (object, price) in [("oShotgun", 15000.0), ("oJetpack", 20000.0)] {
        let seed = fixtures(false)[object];
        let mut source = Rng::new(seed);
        let selected = source_selector(&mut source, false, false, true).unwrap();
        assert_eq!(selected.0, object);
        crate::creation::begin_generated_create_trace();
        let (world, actual, _) = one_parent(seed, false, false);
        let traces = crate::creation::take_generated_create_traces();
        assert_eq!(traces.len(), 1);
        let birth = &traces[0];
        assert_eq!(
            birth.rng_before.clone().next_u64(),
            source.clone().next_u64()
        );
        assert_eq!(actual.clone().next_u64(), source.next_u64());
        assert_eq!(birth.entity_after.object, object);
        assert_eq!(
            birth.entity_after.value("cost"),
            price,
            "constructor finishes before caller writes"
        );
        assert_eq!(birth.entity_after.value("depth"), 101.0);
        assert_eq!(
            birth.entity_after.value("source_pickup_create_initialized"),
            1.0
        );
        let final_child = world.entity(birth.entity_after.id).unwrap();
        assert_eq!(final_child.value("cost"), 0.0);
        assert_eq!(final_child.value("forSale"), 0.0);
        assert_eq!(
            final_child.creation_serial,
            birth.entity_after.creation_serial
        );
    }
}

#[test]
fn live_terrain_create_retained_concrete_item_debt_applies_set2_overrides_after_constructor() {
    for object in ["oShotgun", "oJetpack"] {
        let mut source = Entity::new(0, "oLush", Vec2::new(96.0, 96.0));
        source.data.insert("cleanDeath".into(), 1.0);
        source.data.insert("embedded_treasure".into(), 1.0);
        source.text.insert("treasure".into(), object.into());
        for adapter in 0..3 {
            let mut game = Game::shell(19, RulesPreset::Classic);
            game.world = World::new(20, 16, Area::Jungle, 5);
            let before = game.rng.clone().next_u64();
            match adapter {
                0 => {
                    game.world.set_tile(6, 6, "oLush");
                    let parent = metadata(&mut game.world, 6, 6);
                    game.world.entity_mut(parent).unwrap().data = source.data.clone();
                    game.world.entity_mut(parent).unwrap().text = source.text.clone();
                    assert!(crate::items::explosives::destroy_terrain_self(
                        &mut game, 6, 6
                    ));
                    assert!(!crate::items::explosives::destroy_terrain_self(
                        &mut game, 6, 6
                    ));
                }
                1 => crate::items::explosives::destroy_temple_support_clean(
                    &mut game.world,
                    &mut game.rng,
                    &source,
                ),
                _ => {
                    let mut block = source.clone();
                    block.object = "oBlock".into();
                    crate::items::explosives::destroy_generated_block_own(
                        &mut game.world,
                        &mut game.rng,
                        &block,
                        false,
                    );
                }
            }
            let children: Vec<_> = game
                .world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == object)
                .collect();
            assert_eq!(children.len(), 1, "legacy adapter {adapter}/{object}");
            assert_eq!(children[0].value("cost"), 0.0);
            assert_eq!(children[0].value("forSale"), 0.0);
            assert_eq!(children[0].pos, Vec2::new(104.0, 104.0));
            let id = children[0].id;
            crate::items::create_instance(&mut game, id);
            assert_eq!(
                game.world.entity(id).unwrap().value("cost"),
                0.0,
                "legacy caller overrides survive shared reentry"
            );
            assert_eq!(game.world.entity(id).unwrap().value("forSale"), 0.0);
            assert_eq!(
                game.rng.clone().next_u64(),
                before,
                "clean adapter fixed child has no RNG"
            );
        }
    }
}
