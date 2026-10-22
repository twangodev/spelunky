//! Literal terrain Create and room-caller controls, without generated-outcome
//! seed selection, controller tape changes, or a whole-generation parity claim.
use super::*;
use crate::{Area, Game, InputFrame, RulesPreset};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
};

pub(super) struct Trace {
    object: String,
    origin: Vec2,
    before: Rng,
    after: Rng,
    metadata: Entity,
    first_child: u64,
    children: Vec<Entity>,
}
thread_local! { static TRACES: RefCell<Option<Vec<Trace>>> = const {RefCell::new(None)}; }
pub(super) fn observe_before(world: &World, rng: &Rng, id: u64) -> Option<Trace> {
    if !TRACES.with(|t| t.borrow().is_some()) {
        return None;
    }
    let metadata = world.entity(id).unwrap().clone();
    let x = (metadata.pos.x / 16.).floor() as i32;
    let y = (metadata.pos.y / 16.).floor() as i32;
    Some(Trace {
        object: world.tile(x, y).unwrap().object.clone(),
        origin: Vec2::new(metadata.pos.x - 8., metadata.pos.y - 8.),
        before: rng.clone(),
        after: rng.clone(),
        metadata,
        first_child: world.next_id,
        children: Vec::new(),
    })
}
pub(super) fn observe_after(world: &World, rng: &Rng, trace: Option<Trace>) {
    if let Some(mut trace) = trace {
        trace.after = rng.clone();
        trace.metadata = world.entity(trace.metadata.id).unwrap().clone();
        trace.children = world
            .entities
            .iter()
            .filter(|e| e.alive && e.id >= trace.first_child)
            .cloned()
            .collect();
        TRACES.with(|t| t.borrow_mut().as_mut().unwrap().push(trace));
    }
}
// Independent scrGenerateItem Set2 switch order and offsets.
const SET2: [(&str, f64, f64); 19] = [
    ("oJetpack", 0., -2.),
    ("oCapePickup", 0., 0.),
    ("oShotgun", 0., 0.),
    ("oMattock", 0., 0.),
    ("oTeleporter", 0., 3.),
    ("oGloves", 0., -1.),
    ("oSpectacles", 0., 0.),
    ("oWebCannon", -2., 0.),
    ("oPistol", 0., 0.),
    ("oMitt", 0., -1.),
    ("oPaste", 0., 0.),
    ("oSpringShoes", 0., 0.),
    ("oSpikeShoes", 0., 0.),
    ("oMachete", 0., 0.),
    ("oBombBox", 0., -2.),
    ("oBow", 0., 0.),
    ("oCompass", 0., 0.),
    ("oParaPickup", 0., 0.),
    ("oRopePile", 0., 0.),
];
fn rand(state: &mut u32, width: u32) -> u32 {
    *state = state.wrapping_mul(0x0808_8405).wrapping_add(1);
    ((u64::from(*state) * u64::from(width)) >> 32) as u32
}
struct Expected {
    state: u32,
    sprite: String,
    n: Option<u32>,
    gold: u8,
    child: Option<(&'static str, f64, f64)>,
}
fn literal(object: &str, seed: u32, city: bool, intro: bool, interior_level: bool) -> Expected {
    let mut state = seed;
    let mut sprite = format!("s{}", &object[1..]);
    let mut n = None;
    let mut gold = 0;
    let mut child = None;
    if object == "oBlock" {
        if city {
            sprite = "sGoldBlock".into();
        }
    } else if object == "oIce" {
        if rand(&mut state, 80) == 0 {
            child = Some(("oFrozenCaveman", -8., -8.));
        }
    } else {
        if object == "oBrick" && rand(&mut state, 10) == 0 {
            sprite = "sBrick2".into();
        }
        let roll = 1 + rand(&mut state, 100);
        n = Some(roll);
        if object == "oTemple" && city {
            sprite = "sGTemple".into();
            gold = 2;
        } else if !(object == "oDark" && intro) {
            if roll < 20 {
                sprite = format!("s{}Gold", &object[1..]);
                gold = 1;
            } else if roll < 30 {
                sprite = format!("s{}GoldBig", &object[1..]);
                gold = 2;
            } else if interior_level {
                let widths = match object {
                    "oBrick" => [100, 120, 140],
                    "oDark" => [40, 60, 80],
                    "oTemple" => [60, 80, 100],
                    _ => unreachable!(),
                };
                for (object, width) in ["oSapphireBig", "oEmeraldBig", "oRubyBig"]
                    .into_iter()
                    .zip(widths)
                {
                    if rand(&mut state, width) == 0 {
                        child = Some((object, 0., 0.));
                        break;
                    }
                }
                if child.is_none() && rand(&mut state, 1200) == 0 {
                    child = Some(SET2[rand(&mut state, 19) as usize]);
                }
            }
        }
    }
    Expected {
        state,
        sprite,
        n,
        gold,
        child,
    }
}
fn assert_trace(trace: &Trace, city: bool, intro: bool, interior: bool, spectacles: bool) {
    let expected = literal(
        &trace.object,
        trace.before.state_bits() as u32,
        city,
        intro,
        interior,
    );
    assert_eq!(
        trace.after.state_bits(),
        u64::from(expected.state),
        "{} literal own draw order",
        trace.object
    );
    assert_eq!(
        trace.metadata.text.get("terrain_sprite"),
        Some(&expected.sprite)
    );
    if let Some(n) = expected.n {
        assert_eq!(trace.metadata.value("n"), f64::from(n));
    } else {
        assert!(!trace.metadata.data.contains_key("n"));
    }
    assert_eq!(trace.metadata.value("terrain_initialized"), 1.);
    assert_eq!(trace.metadata.value("source_room_terrain_created"), 1.);
    for name in [
        "blinkToggle",
        "invincible",
        "shopWall",
        "cleanDeath",
        "embedded_treasure",
        "frozen_caveman",
    ] {
        assert_eq!(trace.metadata.value(name), 0., "own {name} before caller");
    }
    assert_eq!(
        trace.metadata.text.get("type").map(String::as_str),
        Some("None")
    );
    assert_eq!(
        trace.metadata.text.get("treasure").map(String::as_str),
        Some("")
    );
    assert_eq!(
        trace.metadata.value("gold_chunks"),
        if expected.gold == 0 { 0. } else { 3. }
    );
    assert_eq!(
        trace.metadata.value("gold_nuggets"),
        if expected.gold == 2 { 1. } else { 0. }
    );
    match expected.child {
        None => assert!(trace.children.is_empty()),
        Some((object, dx, dy)) => {
            assert_eq!(trace.children.len(), 1);
            let child = &trace.children[0];
            assert_eq!(child.object, object);
            assert_eq!(
                child.pos,
                Vec2::new(trace.origin.x + 8. + dx, trace.origin.y + 8. + dy)
            );
            assert!(child.creation_serial > trace.metadata.creation_serial);
            assert_eq!(child.age, 0);
            assert_eq!(child.vel, Vec2::default());
            if object == "oFrozenCaveman" {
                assert_eq!(child.sprite, "sFrozenCaveman");
                assert!(!child.solid);
                assert!(child.alive && child.engine_active);
                assert_eq!(child.text.get("type").map(String::as_str), Some(""));
                assert_eq!(child.value("blinkToggle"), 0.);
                assert_eq!(child.value("frozen_create_initialized"), 1.);
                assert_eq!(child.state, 0);
                assert_eq!(child.timer, 0);
            } else {
                assert_eq!(child.value("source_pickup_create_initialized"), 1.);
                assert_eq!(child.value("myGrav"), 0.6);
                assert_eq!(child.value("xVel"), 0.);
                assert_eq!(child.value("yVel"), 0.);
                if crate::catalog::is_a(object, "oItem") {
                    assert_eq!(child.value("cost"), 0.);
                    assert_eq!(child.value("forSale"), 0.);
                    assert_eq!(child.value("depth"), if spectacles { 51. } else { 101. });
                    assert_eq!(child.value("canPickUp"), 1.);
                } else {
                    assert_eq!(child.state, 1);
                    assert_eq!(child.alarms[0], 20);
                    assert_eq!(child.value("canCollect"), 0.);
                }
            }
        }
    }
}
fn direct(
    object: &str,
    seed: u32,
    name: &str,
    level: u32,
    x: usize,
    y: usize,
    spectacles: bool,
) -> (World, Rng) {
    let mut world = World::new(20, 16, Area::for_level(level), level);
    world.name = name.into();
    world.set_tile(x, y, object);
    let mut rng = Rng::new(seed);
    TRACES.with(|t| *t.borrow_mut() = Some(Vec::new()));
    create_source_room_terrain(
        &mut world,
        &mut rng,
        x,
        y,
        super::super::GenerationContext {
            has_spectacles: spectacles,
            ..super::super::GenerationContext::default()
        },
    );
    let traces = TRACES.with(|t| t.borrow_mut().take().unwrap());
    assert_eq!(traces.len(), 1);
    let in_level = source_is_level(&world);
    assert_trace(
        &traces[0],
        name == "City of Gold",
        name == "rIntro",
        in_level && x > 0 && y > 0 && x + 1 < world.width && y + 1 < world.height,
        spectacles,
    );
    (world, rng)
}

#[test]
fn transition_room_lush_create_keeps_level_only_loot_guards() {
    let seed = (0..100)
        .find(|seed| {
            let mut rng = Rng::new(*seed);
            1 + rng.range(100) >= 30
        })
        .expect("a seed beyond the Lush gold variants");
    let mut expected = Rng::new(seed);
    let n = 1 + expected.range(100);
    let mut world = World::new(20, 16, Area::Jungle, 5);
    world.name = "rTransition1x".into();
    world.set_tile(6, 6, "oLush");
    let mut actual = Rng::new(seed);

    create_source_room_terrain(
        &mut world,
        &mut actual,
        6,
        6,
        super::super::GenerationContext::default(),
    );

    let metadata = world.tile_sprite_metadata(6, 6).unwrap();
    assert_eq!(metadata.value("n"), f64::from(n));
    assert_eq!(actual.state_bits(), expected.state_bits());
    assert!(
        world
            .entities
            .iter()
            .filter(|entity| entity.alive)
            .all(|entity| entity.object == "oTerrainMetadata")
    );
}

#[test]
fn all_three_room_parent_families_birth_all_literal_gems_and_set2_items_before_late_phase() {
    for (object, name, level) in [
        ("oBrick", "Mines", 1),
        ("oDark", "Ice Caves", 9),
        ("oTemple", "Temple", 13),
    ] {
        let mut seeds = BTreeMap::new();
        // Constructor inputs selected entirely on independent literal own predicates.
        for seed in 0..500_000 {
            if let Some((child, _, _)) = literal(object, seed, false, false, true).child {
                seeds.entry(child).or_insert(seed);
            }
            if seeds.len() == 22 {
                break;
            }
        }
        assert_eq!(seeds.len(), 22);
        for spectacles in [false, true] {
            for (child, seed) in &seeds {
                let (mut world, mut rng) = direct(object, *seed, name, level, 6, 6, spectacles);
                assert!(world.entities.iter().any(|e| e.alive && e.object == *child));
                let before = serde_json::to_vec(&world).unwrap();
                let rng_before = rng.state_bits();
                create_generated_with_context(
                    &mut world,
                    &mut rng,
                    super::super::GenerationContext {
                        has_spectacles: spectacles,
                        ..super::super::GenerationContext::default()
                    },
                );
                assert_eq!(rng.state_bits(), rng_before);
                assert_eq!(
                    serde_json::to_vec(&world).unwrap(),
                    before,
                    "no late re-roll or deferred rebirth"
                );
            }
        }
    }
}
#[test]
fn gold_intro_city_block_and_strict_bounds_follow_own_literal_guards() {
    for seed in [0, 1, 37, 1234] {
        for object in ["oBrick", "oDark", "oTemple", "oIce", "oBlock"] {
            direct(object, seed, "Mines", 1, 6, 6, false);
            direct(object, seed, "rTitle", 0, 6, 6, false);
            for (x, y) in [(0, 6), (19, 6), (6, 0), (6, 15)] {
                direct(object, seed, "Mines", 1, x, y, false);
            }
        }
        direct("oDark", seed, "rIntro", 0, 6, 6, false);
        direct("oTemple", seed, "City of Gold", 13, 6, 6, true);
        direct("oBlock", seed, "City of Gold", 13, 6, 6, true);
    }
}
#[test]
fn real_frozen_child_inherits_drawn_create_and_releases_once_on_support_loss() {
    for clean in [false, true] {
        let (world, _) = direct("oIce", 0, "Ice Caves", 9, 6, 6, false);
        let frozen = world
            .entities
            .iter()
            .find(|e| e.alive && e.object == "oFrozenCaveman")
            .unwrap()
            .id;
        assert!(
            world
                .entities
                .iter()
                .filter(|e| e.object == "oTerrainMetadata")
                .all(|e| e.value("frozen_caveman") == 0.)
        );
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.world = world;
        game.player.pos = Vec2::new(40., 40.);
        game.step(&InputFrame::default());
        assert!(game.world.entity(frozen).is_some());
        if clean {
            game.flags.insert("cleanSolids".into());
        }
        game.world.set_tile(6, 6, "");
        game.step(&InputFrame::default());
        assert!(game.world.entity(frozen).is_none());
        let cavemen: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|e| e.alive && e.object == "oCaveman")
            .collect();
        assert_eq!(cavemen.len(), if clean { 0 } else { 1 });
        if !clean {
            assert_eq!(cavemen[0].state, 98);
            assert_eq!(cavemen[0].timer, 200);
            assert_eq!(cavemen[0].value("invincible"), 20.);
        }
        game.step(&InputFrame::default());
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == "oCaveman")
                .count(),
            if clean { 0 } else { 1 }
        );
    }
}

fn scripts() -> Vec<(&'static str, Area, &'static str, &'static str)> {
    vec![
        (
            "Mines",
            Area::Mines,
            "129.sM",
            include_str!(
                "../../../../references/classic-1.1/Scripts/Level Generation/scrRoomGen.gml"
            ),
        ),
        (
            "Jungle",
            Area::Jungle,
            "tr",
            include_str!(
                "../../../../references/classic-1.1/Scripts/Level Generation/scrRoomGen2.gml"
            ),
        ),
        (
            "Black Market",
            Area::Jungle,
            "t",
            include_str!(
                "../../../../references/classic-1.1/Scripts/Level Generation/scrRoomGenMarket.gml"
            ),
        ),
        (
            "Ice Caves",
            Area::Ice,
            "129.bTmij",
            include_str!(
                "../../../../references/classic-1.1/Scripts/Level Generation/scrRoomGen3.gml"
            ),
        ),
        (
            "Yeti Lair",
            Area::Ice,
            "129.ij",
            include_str!(
                "../../../../references/classic-1.1/Scripts/Level Generation/scrRoomGenYeti.gml"
            ),
        ),
        (
            "Temple",
            Area::Temple,
            "123R9.b",
            include_str!(
                "../../../../references/classic-1.1/Scripts/Level Generation/scrRoomGen4.gml"
            ),
        ),
        (
            "Olmec's Lair",
            Area::Boss,
            "129T",
            include_str!(
                "../../../../references/classic-1.1/Scripts/Level Generation/scrRoomGen5.gml"
            ),
        ),
    ]
}
#[test]
fn all_literal_remaining_terrain_call_sites_are_observed_and_caller_order_is_preserved() {
    let mut literal_sites = BTreeSet::new();
    let mut count = 0;
    for (name, _, glyphs, source) in scripts() {
        let mut glyph = None;
        let mut found = BTreeSet::new();
        for line in source
            .split("// Generate the tiles")
            .nth(1)
            .unwrap()
            .lines()
        {
            let t = line.trim();
            if t.starts_with("if (tile == \"") || t.starts_with("else if (tile == \"") {
                glyph = Some(t.split('"').nth(1).unwrap().as_bytes()[0]);
            }
            for object in ["oBrick", "oDark", "oTemple", "oIce", "oBlock"] {
                if t.contains("instance_create") && t.contains(&format!(", {object})")) {
                    found.insert(glyph.unwrap());
                    literal_sites.insert((name, glyph.unwrap(), object));
                    count += 1;
                }
            }
        }
        assert_eq!(
            found,
            glyphs.bytes().collect::<BTreeSet<_>>(),
            "complete literal room call-site glyph inventory"
        );
    }
    assert_eq!(count, 48);
    assert_eq!(literal_sites.len(), 48);
    let mut observed = BTreeSet::new();
    for (name, area, glyphs, _) in scripts() {
        for glyph in glyphs.bytes() {
            for seed in 0..128 {
                let mut world = World::new(
                    40,
                    30,
                    area,
                    match area {
                        Area::Mines => 1,
                        Area::Jungle => 5,
                        Area::Ice => 9,
                        Area::Temple => 13,
                        Area::Boss => 16,
                    },
                );
                world.name = name.into();
                let mut rng = Rng::new(seed);
                TRACES.with(|t| *t.borrow_mut() = Some(Vec::new()));
                super::super::tiles::stamp_at_source_context(
                    &mut world,
                    &mut rng,
                    &String::from_utf8(vec![glyph, b'L']).unwrap(),
                    6,
                    6,
                    false,
                    false,
                    "General",
                    false,
                    super::super::GenerationContext {
                        has_spectacles: true,
                        ..super::super::GenerationContext::default()
                    },
                );
                let traces = TRACES.with(|t| t.borrow_mut().take().unwrap());
                for trace in traces {
                    let key = (name, glyph, trace.object.as_str());
                    assert!(literal_sites.contains(&key));
                    observed.insert((name, glyph, trace.object.clone()));
                    assert_trace(&trace, false, false, true, true);
                    let parent = world.entity(trace.metadata.id).unwrap();
                    assert!(parent.creation_serial < world.tile(7, 6).unwrap().creation_serial);
                    for child in &trace.children {
                        assert!(child.creation_serial < world.tile(7, 6).unwrap().creation_serial);
                        assert!(world.entity(child.id).is_some());
                    }
                    if glyph == b'9' {
                        let door = world
                            .entities
                            .iter()
                            .find(|e| e.alive && e.object == "oExit")
                            .unwrap();
                        if area == Area::Boss {
                            assert!(door.creation_serial < parent.creation_serial);
                        } else {
                            assert!(parent.creation_serial < door.creation_serial);
                            assert!(
                                trace
                                    .children
                                    .iter()
                                    .all(|c| c.creation_serial < door.creation_serial)
                            );
                        }
                        assert_eq!(trace.metadata.value("invincible"), 0.);
                        assert_eq!(parent.value("invincible"), 1.);
                    }
                    if glyph == b'.' || glyph == b'b' && area == Area::Temple {
                        assert_eq!(parent.value("shopWall"), 1.);
                        assert_eq!(
                            parent.text.get("terrain_sprite"),
                            trace.metadata.text.get("terrain_sprite")
                        );
                    }
                    if glyph == b'R' {
                        assert_eq!(
                            parent.text.get("treasure").map(String::as_str),
                            Some("Big Ruby")
                        );
                        assert_eq!(parent.value("embedded_treasure"), 1.);
                    }
                    if glyph == b'm' && area == Area::Ice {
                        assert_eq!(parent.value("invincible"), 1.);
                    }
                    if glyph == b'M' && area == Area::Mines || glyph == b'T' && area == Area::Ice {
                        let gift = world
                            .entities
                            .iter()
                            .filter(|e| {
                                e.alive
                                    && e.object
                                        == if glyph == b'M' {
                                            "oMattock"
                                        } else {
                                            "oJetpack"
                                        }
                            })
                            .max_by_key(|e| e.creation_serial)
                            .unwrap();
                        assert!(
                            trace
                                .children
                                .iter()
                                .all(|child| child.creation_serial < gift.creation_serial)
                        );
                        assert!(parent.creation_serial < gift.creation_serial);
                    }
                }
            }
        }
    }
    let expected: BTreeSet<_> = literal_sites
        .into_iter()
        .map(|(name, c, obj)| (name, c, obj.to_owned()))
        .collect();
    assert_eq!(
        observed, expected,
        "all48 actual parent-type sites reached in fixed source input domain"
    );
}
