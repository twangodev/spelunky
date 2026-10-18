use super::*;
use crate::{CollisionPrecision, Entity, Game, RulesPreset};
use std::{cell::RefCell, collections::BTreeMap};

pub(super) struct Trace {
    before: World,
    after: World,
    rng_before: Rng,
    rng_after: Rng,
}
thread_local! {
    static TRACE: RefCell<Option<Vec<Trace>>> = const { RefCell::new(None) };
}
pub(super) fn trace_before(world: &World, rng: &Rng) -> Option<(World, Rng)> {
    TRACE.with(|trace| {
        trace
            .borrow()
            .as_ref()
            .map(|_| (world.clone(), rng.clone()))
    })
}
pub(super) fn trace_after(world: &World, rng: &Rng, before: Option<(World, Rng)>) {
    if let Some((before, rng_before)) = before {
        TRACE.with(|trace| {
            if let Some(trace) = trace.borrow_mut().as_mut() {
                trace.push(Trace {
                    before,
                    after: world.clone(),
                    rng_before,
                    rng_after: rng.clone(),
                });
            }
        });
    }
}

// Independent statement transcription of scrEntityGen174–208. Clone the
// selected port RNG to check call order/table wiring, not canonical RNG math.
// Raw World.spawn is intentional: this oracle consumes the source Grave draw
// explicitly and the audited pickup constructors consume no random numbers.
fn literal_source(before: &World, rng: &mut Rng) -> Vec<Entity> {
    let mut world = before.clone();
    let first_id = world.next_id;
    let mut receivers = Vec::new();
    for (index, tile) in world.tiles.iter().enumerate() {
        if tile.engine_active && crate::catalog::is_a(&tile.object, "oLush") {
            receivers.push((
                tile.creation_serial,
                Vec2::new(
                    (index % world.width) as f64 * 16.0,
                    (index / world.width) as f64 * 16.0,
                ),
            ));
        }
    }
    for entity in &world.entities {
        if entity.alive && entity.engine_active && crate::catalog::is_a(&entity.object, "oLush") {
            receivers.push((entity.creation_serial, entity.pos));
        }
    }
    receivers.sort_by_key(|(serial, _)| *serial);
    for (_, origin) in receivers {
        let p = Vec2::new(origin.x, origin.y - 16.0);
        let clear = !named_point(&world, p, "oSolid")
            && !named_point(&world, p, "oEntrance")
            && !named_point(&world, p, "oExit");
        let selected = 1 + rng.range(20);
        if !(clear
            && selected == 1
            && origin.x != 160.0
            && origin.x != 176.0
            && origin.x != 320.0
            && origin.x != 336.0
            && origin.x != 480.0
            && origin.x != 496.0)
        {
            continue;
        }
        let grave = world.spawn("oGrave", p);
        world.entity_mut(grave).unwrap().sprite = format!("sGrave{}", 2 + rng.range(5));
        if 1 + rng.range(40) == 1 {
            world.entity_mut(grave).unwrap().sprite = "sGraveAsh".into();
            world.spawn("oShotgun", Vec2::new(origin.x + 8.0, origin.y + 8.0));
            // Original assigns local ashGrave, leaving the global false.
        } else if !named_point(
            &world,
            Vec2::new(origin.x + 8.0, origin.y + 8.0),
            "oTreasure",
        ) {
            let object = if 1 + rng.range(2) == 1 {
                Some("oGoldNugget")
            } else if 1 + rng.range(4) == 1 {
                Some("oSapphireBig")
            } else if 1 + rng.range(6) == 1 {
                Some("oEmeraldBig")
            } else if 1 + rng.range(8) == 1 {
                Some("oRubyBig")
            } else {
                None
            };
            if let Some(object) = object {
                world.spawn(object, Vec2::new(origin.x + 8.0, origin.y + 8.0));
            }
        }
    }
    world
        .entities
        .into_iter()
        .filter(|e| e.id >= first_id)
        .collect()
}

fn named_point(world: &World, point: Vec2, family: &str) -> bool {
    world.tiles_point(point, family)
        || world.entities.iter().any(|e| {
            e.alive
                && crate::catalog::is_a(&e.object, family)
                && e.mask_contains(point, CollisionPrecision::BoundingBox)
        })
}
fn fixture() -> World {
    let mut world = World::new(44, 40, Area::Jungle, 5);
    world.set_tile(6, 6, "oLush");
    // Existing metadata models the ordinary generated parent, not a new
    // authored birth during the tested cemetery phase.
    super::super::tiles::terrain_property(&mut world, 6, 6, "terrain_initialized", 1.0);
    world
}
fn newborns(world: &World, first: u64) -> Vec<Entity> {
    world
        .entities
        .iter()
        .filter(|e| e.id >= first && e.object != "oTerrainMetadata")
        .cloned()
        .collect()
}
fn check(before: &World, seed: u32) -> (World, Vec<Entity>) {
    let first = before.next_id;
    let mut source = Rng::new_canonical(seed);
    let expected = literal_source(before, &mut source);
    let mut actual = Rng::new_canonical(seed);
    let mut world = before.clone();
    TRACE.with(|trace| *trace.borrow_mut() = Some(Vec::new()));
    // This boundary control isolates the complete V2 cemetery producer.
    // The actual next_level test below also verifies its integration before
    // the primary Solid pass, using the same independently written oracle.
    let before_trace = trace_before(&world, &actual);
    cemetery_prepass(
        &mut world,
        &mut actual,
        super::super::GenerationContext::default(),
    );
    trace_after(&world, &actual, before_trace);
    let mut traces = TRACE.with(|trace| trace.borrow_mut().take().unwrap());
    assert_eq!(traces.len(), 1);
    let trace = traces.pop().unwrap();
    assert_eq!(
        trace.rng_before.clone().next_u64(),
        Rng::new_canonical(seed).next_u64()
    );
    let children = newborns(&trace.after, first);
    assert_births(&children, &expected, false);
    assert_eq!(
        trace.rng_after.clone().next_u64(),
        source.next_u64(),
        "seed={seed}"
    );
    (trace.after, children)
}
fn assert_births(actual: &[Entity], expected: &[Entity], spectacles: bool) {
    assert_eq!(actual.len(), expected.len());
    for (actual, expected) in actual.iter().zip(expected) {
        assert_eq!(
            (
                actual.id,
                actual.creation_serial,
                &actual.object,
                actual.pos
            ),
            (
                expected.id,
                expected.creation_serial,
                &expected.object,
                expected.pos
            )
        );
        assert_eq!(actual.previous_pos, actual.pos);
        match actual.object.as_str() {
            "oGrave" => assert_eq!(actual.sprite, expected.sprite),
            "oShotgun" => {
                assert_eq!(actual.text["type"], "Shotgun");
                assert_eq!(actual.value("depth"), if spectacles { 51.0 } else { 101.0 });
                assert_eq!(actual.value("cost"), 0.0);
                assert_eq!(actual.value("forSale"), 0.0);
            }
            object => {
                let (kind, value, collect, alarm) = match object {
                    "oGoldNugget" => ("Gold Nugget", 500.0, 1.0, -1),
                    "oSapphireBig" => ("Big Sapphire", 1200.0, 0.0, 20),
                    "oEmeraldBig" => ("Big Emerald", 800.0, 0.0, 20),
                    "oRubyBig" => ("Big Ruby", 1600.0, 0.0, 20),
                    _ => panic!("unexpected source child {object}"),
                };
                assert_eq!(actual.text["type"], kind);
                assert_eq!(actual.value("value"), value);
                assert_eq!(actual.value("canCollect"), collect);
                assert_eq!(actual.value("yOff"), 4.0);
                assert_eq!(actual.alarms[0], alarm);
                assert_eq!(actual.value("source_pickup_create_initialized"), 1.0);
            }
        }
    }
}

#[test]
fn source_jungle_cemetery_v2_prepass_all_variants_and_conditional_loot_draws() {
    let world = fixture();
    let mut cases = BTreeMap::new();
    // Choose fixtures using only the independently transcribed source body;
    // production outcomes and test success never participate in selection.
    for seed in 0..4096 {
        let children = literal_source(&world, &mut Rng::new_canonical(seed));
        if let Some(grave) = children.first() {
            cases
                .entry(format!("variant:{}", grave.sprite))
                .or_insert(seed);
            cases
                .entry(format!(
                    "loot:{}",
                    children.get(1).map_or("none", |e| e.object.as_str())
                ))
                .or_insert(seed);
        }
    }
    let mut variants = BTreeSet::new();
    let mut outcomes = BTreeSet::new();
    for seed in BTreeSet::from_iter(cases.values().copied()) {
        let (_, children) = check(&world, seed);
        if let Some(grave) = children.first() {
            variants.insert(grave.sprite.clone());
            outcomes.insert(
                children
                    .get(1)
                    .map_or("none", |e| e.object.as_str())
                    .to_owned(),
            );
        }
    }
    assert_eq!(
        variants,
        BTreeSet::from_iter(
            [
                "sGrave2",
                "sGrave3",
                "sGrave4",
                "sGrave5",
                "sGrave6",
                "sGraveAsh"
            ]
            .map(str::to_owned)
        )
    );
    assert_eq!(
        outcomes,
        BTreeSet::from_iter(
            [
                "none",
                "oGoldNugget",
                "oSapphireBig",
                "oEmeraldBig",
                "oRubyBig",
                "oShotgun"
            ]
            .map(str::to_owned)
        )
    );
}

#[test]
fn source_jungle_cemetery_v2_prepass_eager_guards_raw_bans_and_live_mixed_receivers() {
    let mut world = fixture();
    for (column, object) in [(8, "oTemple"), (12, "oEntrance"), (14, "oExit")] {
        world.set_tile(column, 6, "oLush");
        super::super::tiles::terrain_property(&mut world, column, 6, "terrain_initialized", 1.0);
        world.spawn(object, Vec2::new(column as f64 * 16.0, 80.0));
        assert!(named_point(
            &world,
            Vec2::new(column as f64 * 16.0, 80.0),
            if object == "oTemple" {
                "oSolid"
            } else {
                object
            }
        ));
    }
    for x in [16.0, 160.0, 176.0, 320.0, 336.0, 480.0, 496.0, 656.0] {
        world.spawn("oLush", Vec2::new(x, 192.0));
    }
    let inactive = world.spawn("oLush", Vec2::new(80.0, 240.0));
    world.entity_mut(inactive).unwrap().engine_active = false;
    let dead = world.spawn("oLush", Vec2::new(96.0, 240.0));
    world.entity_mut(dead).unwrap().alive = false;
    // A later tile follows earlier dynamic receivers; row-major regrouping is wrong.
    world.set_tile(7, 7, "oLush");
    super::super::tiles::terrain_property(&mut world, 7, 7, "terrain_initialized", 1.0);
    let mut observed = BTreeSet::new();
    for seed in 0..256 {
        let (_, children) = check(&world, seed);
        for grave in children.iter().filter(|e| e.object == "oGrave") {
            assert!(![160.0, 176.0, 320.0, 336.0, 480.0, 496.0].contains(&grave.pos.x));
            observed.insert(grave.pos.x as i32);
        }
    }
    assert!(
        observed.contains(&16) && observed.contains(&656),
        "raw x exclusions do not repeat each room"
    );
}

#[test]
fn source_jungle_cemetery_v2_prepass_treasure_masks_suppress_loot_but_equipment_does_not() {
    let seed = (0..4096)
        .find(|seed| {
            let children = literal_source(&fixture(), &mut Rng::new_canonical(*seed));
            children.len() == 2 && children[1].object == "oGoldNugget"
        })
        .expect("literal guard selector");
    for equipment in [false, true] {
        let mut world = fixture();
        world.spawn(
            if equipment {
                "oShotgun"
            } else {
                "oSapphireBig"
            },
            Vec2::new(104.0, 104.0),
        );
        let (_, children) = check(&world, seed);
        assert_eq!(children.len(), if equipment { 2 } else { 1 });
        assert_eq!(children[0].object, "oGrave");
        if equipment {
            assert_eq!(children[1].object, "oGoldNugget");
        }
    }
}

#[test]
fn source_jungle_cemetery_v2_prepass_local_ash_typo_allows_multiple_free_shotguns() {
    let mut world = World::new(44, 40, Area::Jungle, 5);
    for row in 0..10 {
        for column in 0..10 {
            world.spawn(
                "oLush",
                Vec2::new(32.0 + 32.0 * column as f64, 64.0 + 32.0 * row as f64),
            );
        }
    }
    let seed = (0..8192)
        .find(|seed| {
            literal_source(&world, &mut Rng::new_canonical(*seed))
                .iter()
                .filter(|e| e.object == "oShotgun")
                .count()
                >= 2
        })
        .expect("literal multi-Ash guard selector");
    let (after, children) = check(&world, seed);
    assert!(children.iter().filter(|e| e.object == "oShotgun").count() >= 2);
    for pair in children
        .windows(2)
        .filter(|pair| pair[0].sprite == "sGraveAsh")
    {
        assert_eq!(pair[1].object, "oShotgun");
        let parent = after
            .entities
            .iter()
            .find(|e| {
                e.object == "oLush" && e.pos == Vec2::new(pair[0].pos.x, pair[0].pos.y + 16.0)
            })
            .unwrap();
        assert_eq!(parent.value("ashGrave"), 1.0);
        assert_eq!(parent.value("obj"), pair[1].id as f64);
    }
}

#[test]
fn source_jungle_cemetery_v2_gold_nugget_runtime_constructor_no_alarm_or_rng_and_idempotent() {
    let mut game = Game::shell(17, RulesPreset::Classic);
    let next = game.rng.clone().next_u64();
    let id = game.spawn("oGoldNugget", Vec2::new(104.0, 104.0));
    let e = game.world.entity_mut(id).unwrap();
    assert_eq!(e.text["type"], "Gold Nugget");
    assert_eq!(e.value("canCollect"), 1.0);
    assert_eq!(e.value("yOff"), 4.0);
    assert_eq!(e.alarms[0], -1);
    e.alarms[0] = 9;
    e.text.insert("type".into(), "caller override".into());
    crate::items::create_instance(&mut game, id);
    let e = game.world.entity(id).unwrap();
    assert_eq!(e.alarms[0], 9);
    assert_eq!(e.text["type"], "caller override");
    assert_eq!(game.rng.next_u64(), next);
}

#[test]
fn source_jungle_cemetery_v2_prepass_actual_next_level_completes_before_solid_pass() {
    let mut natural_levels = [false; 2];
    // next_level clears room-local special flags; find naturally selected
    // Jungle cemeteries instead of forcing a flag that the transition removes.
    for seed in 0..64 {
        for (index, spectacles) in [false, true].into_iter().enumerate() {
            if natural_levels[index] {
                continue;
            }
            let mut game = Game::shell(seed, RulesPreset::Classic);
            game.flags.insert("no_dark_level".into());
            game.data.insert("next_level".into(), 5.0);
            if spectacles {
                game.player.equipment.insert("Spectacles".into());
            }
            TRACE.with(|trace| *trace.borrow_mut() = Some(Vec::new()));
            game.next_level();
            let traces = TRACE.with(|trace| trace.borrow_mut().take().unwrap());
            if traces.is_empty() {
                continue;
            }
            assert_eq!(traces.len(), 1, "one complete cemetery phase");
            assert_eq!(game.world.name, "Cemetery", "seed={seed}");
            let trace = &traces[0];
            let mut source = trace.rng_before.clone();
            let expected = literal_source(&trace.before, &mut source);
            let actual = newborns(&trace.after, trace.before.next_id);
            assert_births(&actual, &expected, spectacles);
            assert_eq!(trace.rng_after.clone().next_u64(), source.next_u64());
            natural_levels[index] = true;
        }
        if natural_levels == [true, true] {
            break;
        }
    }
    assert_eq!(
        natural_levels,
        [true, true],
        "find natural cemetery selections"
    );
}
