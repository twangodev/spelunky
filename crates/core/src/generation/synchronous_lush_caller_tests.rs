//! All literal room-script Lush callers. Inputs select source branches before
//! production; no generated outcome, movement recording or successful run is searched.
use super::*;

#[derive(Clone, Copy, Debug)]
struct Caller {
    area: Area,
    name: &'static str,
    glyph: u8,
}
fn callers() -> Vec<Caller> {
    [
        (Area::Jungle, "Jungle", &b"12r39v,."[..]),
        (Area::Jungle, "Black Market", &b"1239."[..]),
        (Area::Temple, "Temple", &b"1de"[..]),
        // Lush own Create has no City-of-Gold override or child suppression.
        (Area::Temple, "City of Gold", &b"1de"[..]),
    ]
    .into_iter()
    .flat_map(|(area, name, glyphs)| {
        glyphs
            .iter()
            .map(move |&glyph| Caller { area, name, glyph })
    })
    .collect()
}
struct LiteralBirth {
    before_create: u32,
    after_create: u32,
    final_state: u32,
    n: u32,
    child: Option<(&'static str, f64, f64)>,
    y: usize,
    water: bool,
}
// Transcribed only for branches that really create Lush. Other terrain
// families' synchronous Create is separate debt and is not credited here.
fn literal_birth(caller: Caller, seed: u32) -> Option<LiteralBirth> {
    let mut state = seed;
    let c = caller.glyph;
    if caller.area == Area::Temple && c == b'1' {
        if literal_range(&mut state, 100) != 0 {
            return None;
        }
    } else if c == b'2' {
        if literal_range(&mut state, 2) != 0 {
            return None;
        }
    } else if c != b'1' {
        // The failed first independent statement still reaches random2.
        literal_range(&mut state, 2);
    }
    match c {
        b'r' | b'3' => {
            if literal_range(&mut state, 2) == 0 {
                return None;
            }
        }
        b'9' | b'v' | b',' | b'.' => {
            literal_range(&mut state, 3);
            if c == b',' && literal_range(&mut state, 2) != 0 {
                return None;
            }
        }
        b'd' | b'e' => {
            literal_range(&mut state, 3);
            literal_range(&mut state, 4);
            if c == b'e' && literal_range(&mut state, 2) != 0 {
                return None;
            }
        }
        _ => {}
    }
    let before_create = state;
    let (n, child) = literal_lush_create(&mut state);
    let after_create = state;
    if caller.area == Area::Jungle && matches!(c, b'1' | b'2') {
        // Separate statement after the completed parent/child constructor.
        literal_range(&mut state, 3);
    }
    // Following L glyph reaches random2, then creates Vine/Ladder without RNG.
    literal_range(&mut state, 2);
    Some(LiteralBirth {
        before_create,
        after_create,
        final_state: state,
        n,
        child,
        y: if c == b'9' { 7 } else { 6 },
        water: matches!(c, b'v' | b','),
    })
}

#[test]
fn all_literal_room_lush_calls_are_in_the_caller_inventory() {
    let sources = [
        (
            "Jungle",
            include_str!(
                "../../../../references/classic-1.1/Scripts/Level Generation/scrRoomGen2.gml"
            ),
        ),
        (
            "Black Market",
            include_str!(
                "../../../../references/classic-1.1/Scripts/Level Generation/scrRoomGenMarket.gml"
            ),
        ),
        (
            "Temple",
            include_str!(
                "../../../../references/classic-1.1/Scripts/Level Generation/scrRoomGen4.gml"
            ),
        ),
    ];
    let mut count = 0;
    for (name, source) in sources {
        let mut glyph = None;
        let mut found = Vec::new();
        for line in source
            .split("// Generate the tiles")
            .nth(1)
            .unwrap()
            .lines()
        {
            let t = line.trim();
            if t.starts_with("if (tile == \"") || t.starts_with("else if (tile == \"") {
                assert_eq!(line.len() - line.trim_start().len(), 8);
                glyph = Some(t.split('"').nth(1).unwrap().as_bytes()[0]);
            }
            if t.contains("instance_create") && t.contains("oLush") {
                found.push(glyph.unwrap());
            }
        }
        let expected: Vec<_> = callers()
            .iter()
            .filter(|c| c.name == name)
            .map(|c| c.glyph)
            .collect();
        assert_eq!(
            found, expected,
            "all actual Lush calls, including rare Temple ground"
        );
        count += found.len();
    }
    assert_eq!(count, 16);
}

fn check_caller(caller: Caller, seed: u32, spectacles: bool, start: bool) {
    let expected = literal_birth(caller, seed).unwrap();
    let mut world = World::new(
        20,
        16,
        caller.area,
        if caller.area == Area::Temple { 13 } else { 5 },
    );
    world.name = caller.name.into();
    let mut rng = Rng::new(seed);
    let context = GenerationContext {
        has_spectacles: spectacles,
        ..GenerationContext::default()
    };
    TRACES.with(|traces| *traces.borrow_mut() = Some(Vec::new()));
    crate::creation::begin_generated_create_trace();
    let template = String::from_utf8(vec![caller.glyph, b'L']).unwrap();
    tiles::stamp_at_source_context(
        &mut world, &mut rng, &template, 6, 6, start, false, "General", false, context,
    );
    let constructors = crate::creation::take_generated_create_traces();
    assert_eq!(
        constructors.len(),
        usize::from(expected.child.is_some())
            + usize::from(expected.water)
            + usize::from(caller.glyph == b'9')
    );
    let children: Vec<_> = constructors
        .iter()
        .filter(|trace| trace.entity_after.value("source_pickup_create_initialized") != 0.)
        .collect();
    let traces = TRACES.with(|traces| traces.borrow_mut().take().unwrap());
    assert_eq!(traces.len(), 1, "{caller:?}/{seed}");
    let trace = &traces[0];
    assert_eq!(trace.before.state_bits(), u64::from(expected.before_create));
    assert_eq!(trace.after.state_bits(), u64::from(expected.after_create));
    assert_eq!(rng.state_bits(), u64::from(expected.final_state));
    assert_eq!(trace.object, "oLush");
    assert_eq!(trace.origin, Vec2::new(96., expected.y as f64 * 16.));
    assert_eq!(trace.metadata.value("n"), f64::from(expected.n));
    assert_eq!(
        trace.metadata.value("invincible"),
        0.,
        "own Create before caller"
    );
    assert_eq!(
        trace.metadata.value("shopWall"),
        0.,
        "own Create before caller"
    );
    assert_eq!(world.tile(6, expected.y as i32).unwrap().object, "oLush");
    let parent = world.entity(trace.metadata.id).unwrap();
    assert_eq!(parent.value("source_ground_lush_created"), 1.);
    assert_eq!(parent.value("terrain_initialized"), 1.);
    assert_eq!(
        parent.value("invincible"),
        if caller.glyph == b'9' && !start {
            1.
        } else {
            0.
        }
    );
    assert_eq!(
        parent.value("shopWall"),
        if caller.glyph == b'.' { 1. } else { 0. }
    );
    assert!(parent.creation_serial < world.tile(7, 6).unwrap().creation_serial);
    if expected.water {
        let water = world
            .entities
            .iter()
            .find(|e| e.alive && e.object == "oWaterSwim")
            .unwrap();
        assert_eq!(water.pos, Vec2::new(96., 96.));
        assert!(
            water.creation_serial < parent.creation_serial,
            "water birth before parent"
        );
    }
    if caller.glyph == b'9' {
        let door = world
            .entities
            .iter()
            .find(|e| e.alive && e.object == if start { "oEntrance" } else { "oExit" })
            .unwrap();
        assert_eq!(door.pos, Vec2::new(96., 96.));
        assert!(
            parent.creation_serial < door.creation_serial,
            "parent before door"
        );
        assert!(
            trace
                .children
                .iter()
                .all(|e| e.creation_serial < door.creation_serial),
            "own children finish before door birth"
        );
    }
    match expected.child {
        None => {
            assert!(children.is_empty());
            assert!(trace.children.is_empty());
        }
        Some((object, dx, dy)) => {
            assert_eq!(children.len(), 1);
            assert_eq!(trace.children.len(), 1);
            let birth = &children[0].entity_after;
            let child = world.entity(birth.id).unwrap();
            assert_eq!(child.object, object);
            assert_eq!(
                child.pos,
                Vec2::new(104. + dx, expected.y as f64 * 16. + 8. + dy)
            );
            assert!(parent.creation_serial < child.creation_serial);
            assert!(child.creation_serial < world.tile(7, 6).unwrap().creation_serial);
            assert_eq!(child.value("source_pickup_create_initialized"), 1.);
            assert_eq!(child.age, 0);
            assert_eq!(child.vel, Vec2::default());
            if crate::catalog::is_a(object, "oItem") {
                assert!(
                    birth.value("cost") > 0.,
                    "constructor precedes Set2 overrides"
                );
                assert_eq!(child.value("cost"), 0.);
                assert_eq!(child.value("forSale"), 0.);
                assert_eq!(child.value("depth"), if spectacles { 51. } else { 101. });
            } else {
                assert_eq!(child.alarms[0], 20);
                assert_eq!(child.state, 1);
                assert_eq!(child.value("canCollect"), 0.);
            }
        }
    }
    let before = serde_json::to_vec(&world).unwrap();
    let rng_before = rng.state_bits();
    create_generated_with_context(&mut world, &mut rng, context);
    assert_eq!(
        rng.state_bits(),
        rng_before,
        "no late parent/child constructor replay"
    );
    assert_eq!(serde_json::to_vec(&world).unwrap(), before);
}

#[test]
fn all_source_lush_callers_complete_real_create_with_literal_prefix_and_suffix() {
    for caller in callers() {
        let mut births = 0;
        for seed in 0..128 {
            if literal_birth(caller, seed).is_some() {
                check_caller(caller, seed, false, false);
                births += 1;
            }
        }
        assert!(births > 0, "literal fixed domain must reach {caller:?}");
        // Select a child-producing constructor INPUT using only the literal
        // source arithmetic, before invoking production. Every caller gets a
        // real child, rather than an RNG-only prefix control.
        let seed = (0..500_000)
            .find(|&seed| literal_birth(caller, seed).is_some_and(|birth| birth.child.is_some()))
            .unwrap();
        check_caller(caller, seed, true, false);
        if caller.glyph == b'9' {
            check_caller(caller, seed, true, true);
        }
    }
}

#[test]
fn historical_all_lush_callers_keep_deferred_births_and_caller_properties() {
    for caller in callers() {
        for canonical in [false, true] {
            for seed in [0, 1, 37, 1234] {
                let mut world = World::new(20, 16, caller.area, 5);
                world.name = caller.name.into();
                let mut rng = if canonical {
                    Rng::new(seed)
                } else {
                    Rng::legacy(u64::from(seed))
                };
                tiles::stamp_at(
                    &mut world,
                    &mut rng,
                    &String::from_utf8(vec![caller.glyph]).unwrap(),
                    6,
                    6,
                    false,
                    false,
                    "General",
                );
                assert!(
                    !world
                        .entities
                        .iter()
                        .any(|e| e.value("source_ground_lush_created") != 0.)
                );
                assert!(
                    !world
                        .entities
                        .iter()
                        .any(|e| e.value("source_pickup_create_initialized") != 0.)
                );
                assert!(
                    world
                        .entities
                        .iter()
                        .filter(|e| e.alive && e.object == "oTerrainMetadata")
                        .all(|e| e.value("terrain_initialized") == 0.)
                );
            }
        }
    }
}
