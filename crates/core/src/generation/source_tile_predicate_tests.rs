//! Reached tile-predicate controls. The independent expected dispatcher reads
//! literal Original clauses; it shares only unchanged branch bodies, so this
//! does not claim independent interior Create/whole-generator stream parity.
use super::*;

#[derive(Debug)]
struct Clause {
    symbol: u8,
    width: Option<u32>,
    vacant: bool,
}
fn literal_groups(source: &str) -> Vec<Vec<Clause>> {
    let decoder = source.split("// Generate the tiles").nth(1).unwrap();
    let mut groups: Vec<Vec<Clause>> = Vec::new();
    for line in decoder.lines() {
        let t = line.trim();
        let independent = t.starts_with("if (tile == \"");
        if !independent && !t.starts_with("else if (tile == \"") {
            continue;
        }
        assert_eq!(
            line.len() - line.trim_start().len(),
            8,
            "decoder top-level statement"
        );
        if independent {
            groups.push(Vec::new());
        }
        let symbol = t.split('"').nth(1).unwrap().as_bytes()[0];
        let width = t
            .split("rand(1,")
            .nth(1)
            .map(|s| s.split(')').next().unwrap().parse().unwrap());
        groups.last_mut().unwrap().push(Clause {
            symbol,
            width,
            vacant: t.contains("not collision_point"),
        });
    }
    groups
}
fn sources() -> [(TileDialect, Area, &'static str, &'static str); 7] {
    [
        (
            TileDialect::Mines,
            Area::Mines,
            "Mines",
            include_str!(
                "../../../../references/classic-1.1/Scripts/Level Generation/scrRoomGen.gml"
            ),
        ),
        (
            TileDialect::Jungle,
            Area::Jungle,
            "Jungle",
            include_str!(
                "../../../../references/classic-1.1/Scripts/Level Generation/scrRoomGen2.gml"
            ),
        ),
        (
            TileDialect::Ice,
            Area::Ice,
            "Ice Caves",
            include_str!(
                "../../../../references/classic-1.1/Scripts/Level Generation/scrRoomGen3.gml"
            ),
        ),
        (
            TileDialect::Temple,
            Area::Temple,
            "Temple",
            include_str!(
                "../../../../references/classic-1.1/Scripts/Level Generation/scrRoomGen4.gml"
            ),
        ),
        (
            TileDialect::Boss,
            Area::Boss,
            "Olmec's Lair",
            include_str!(
                "../../../../references/classic-1.1/Scripts/Level Generation/scrRoomGen5.gml"
            ),
        ),
        (
            TileDialect::Market,
            Area::Jungle,
            "Black Market",
            include_str!(
                "../../../../references/classic-1.1/Scripts/Level Generation/scrRoomGenMarket.gml"
            ),
        ),
        (
            TileDialect::Yeti,
            Area::Ice,
            "Yeti Lair",
            include_str!(
                "../../../../references/classic-1.1/Scripts/Level Generation/scrRoomGenYeti.gml"
            ),
        ),
    ]
}
fn rand_literal(state: &mut u32, width: u32) -> u32 {
    *state = state.wrapping_mul(0x0808_8405).wrapping_add(1);
    1 + ((u64::from(*state) * u64::from(width)) >> 32) as u32
}
fn world(area: Area, name: &str, occupied: bool) -> World {
    let mut w = World::new(32, 32, area, 9);
    w.name = name.into();
    if occupied {
        w.set_tile(3, 3, "oBrick");
    }
    w
}
#[test]
fn wrong_symbol_zero_reaches_every_random_predicate_without_births() {
    for (dialect, area, name, source) in sources() {
        for seed in [0, 1, 37, u32::MAX] {
            let mut expected = seed;
            let mut count = 0;
            for group in literal_groups(source) {
                for c in group {
                    if let Some(width) = c.width {
                        rand_literal(&mut expected, width);
                        count += 1;
                    }
                    assert_ne!(c.symbol, b'0');
                }
            }
            assert_eq!(
                count,
                match dialect {
                    TileDialect::Jungle | TileDialect::Market => 2,
                    TileDialect::Yeti => 4,
                    _ => 3,
                }
            );
            let mut actual_world = world(area, name, false);
            let mut actual_rng = Rng::new(seed);
            stamp_at_source(
                &mut actual_world,
                &mut actual_rng,
                "0",
                3,
                3,
                false,
                false,
                "General",
                false,
            );
            assert_eq!(actual_rng.state_bits(), u64::from(expected));
            assert!(actual_world.entities.is_empty());
            assert!(actual_world.tiles.iter().all(|t| t.object.is_empty()));
        }
    }
}
#[test]
fn reached_predicates_preserve_body_order_clearance_failures_and_independent_chains() {
    for (dialect, area, name, source) in sources() {
        let groups = literal_groups(source);
        let mut alphabet = groups
            .iter()
            .flatten()
            .map(|c| c.symbol)
            .collect::<Vec<_>>();
        alphabet.extend_from_slice(b"0Z");
        alphabet.sort_unstable();
        alphabet.dedup();
        for symbol in alphabet {
            for seed in 0..8 {
                for occupied in [false, true] {
                    let mut expected_world = world(area, name, occupied);
                    let mut state = seed;
                    for group in &groups {
                        for clause in group {
                            let win = clause
                                .width
                                .is_none_or(|width| rand_literal(&mut state, width) == 1);
                            // Re-evaluate after earlier independent bodies;
                            // never short-circuit a reached random on symbol.
                            let blocked = solid_point(&expected_world, Vec2::new(48., 48.));
                            if clause.symbol == symbol && win && (!clause.vacant || !blocked) {
                                let mut body_rng = Rng::new(state);
                                // Preserved branch bodies are separate from the
                                // independently parsed predicate/LCG oracle.
                                stamp_tile_body(
                                    &mut expected_world,
                                    &mut body_rng,
                                    symbol,
                                    3,
                                    3,
                                    Vec2::new(48., 48.),
                                    false,
                                    "General",
                                    Some(false),
                                    dialect == TileDialect::Yeti,
                                    dialect == TileDialect::Market,
                                    blocked,
                                    true,
                                    super::super::GenerationContext::default(),
                                );
                                state = body_rng.state_bits() as u32;
                                break;
                            }
                        }
                    }
                    let mut actual_world = world(area, name, occupied);
                    let mut actual_rng = Rng::new(seed);
                    let template = String::from_utf8(vec![symbol]).unwrap();
                    stamp_at_source(
                        &mut actual_world,
                        &mut actual_rng,
                        &template,
                        3,
                        3,
                        false,
                        false,
                        "General",
                        false,
                    );
                    assert_eq!(
                        actual_rng.state_bits(),
                        u64::from(state),
                        "{dialect:?}/{symbol}/{seed}/{occupied}"
                    );
                    assert_eq!(
                        serde_json::to_vec(&actual_world).unwrap(),
                        serde_json::to_vec(&expected_world).unwrap(),
                        "{dialect:?}/{symbol}/{seed}/{occupied}"
                    );
                }
            }
        }
    }
}

#[test]
fn source_tile_solid_point_uses_active_o_solid_family_membership() {
    let point = Vec2::new(48.0, 48.0);
    let mut held_world = world(Area::Mines, "Mines", false);
    let held_crate = held_world.spawn("oPushBlock", point);
    assert!(crate::catalog::is_a("oPushBlock", "oSolid"));
    held_world.entity_mut(held_crate).unwrap().held = true;
    assert!(source_solid_point(&held_world, point));
    stamp_at_source(
        &mut held_world,
        &mut Rng::new(17),
        "1",
        3,
        3,
        false,
        false,
        "General",
        false,
    );
    assert!(held_world.tile(3, 3).unwrap().object.is_empty());

    let mut inactive_world = world(Area::Mines, "Mines", false);
    let inactive_crate = inactive_world.spawn("oPushBlock", point);
    inactive_world
        .entity_mut(inactive_crate)
        .unwrap()
        .engine_active = false;
    assert!(!source_solid_point(&inactive_world, point));
    stamp_at_source(
        &mut inactive_world,
        &mut Rng::new(17),
        "1",
        3,
        3,
        false,
        false,
        "General",
        false,
    );
    assert!(!inactive_world.tile(3, 3).unwrap().object.is_empty());
}
