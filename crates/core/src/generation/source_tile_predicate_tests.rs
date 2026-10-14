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
fn all_seven_literal_clause_orders_independent_statements_and_random_guards_match() {
    let mut clauses = 0;
    let mut predicates = 0;
    for (dialect, _, _, source) in sources() {
        let groups = literal_groups(source);
        assert_eq!(groups.len(), source_statement_groups(dialect).len());
        for (literal, actual) in groups.iter().zip(source_statement_groups(dialect)) {
            assert_eq!(
                literal.iter().map(|c| c.symbol).collect::<Vec<_>>(),
                *actual
            );
            for c in literal {
                assert_eq!(c.width, source_random_predicate(dialect, c.symbol));
                assert_eq!(c.vacant, source_clear_predicate(dialect, c.symbol));
                clauses += 1;
                if c.width.is_some() {
                    predicates += 1;
                }
            }
        }
    }
    // Inventory is literal top-level source, not the production match arms.
    assert_eq!(clauses, 216);
    assert_eq!(predicates, 20);
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
fn historical_decoder_retains_exact_world_and_rng_for_both_algorithms() {
    for (_, area, name, source) in sources() {
        let mut bytes = literal_groups(source)
            .iter()
            .flatten()
            .map(|c| c.symbol)
            .collect::<Vec<_>>();
        bytes.extend_from_slice(b"00Z");
        let template = String::from_utf8(bytes).unwrap();
        for canonical in [false, true] {
            for occupied in [false, true] {
                let mut actual_world = world(area, name, occupied);
                let mut expected_world = actual_world.clone();
                let mut actual_rng = if canonical {
                    Rng::new(37)
                } else {
                    Rng::legacy(37)
                };
                let mut expected_rng = actual_rng.clone();
                stamp_at(
                    &mut actual_world,
                    &mut actual_rng,
                    &template,
                    3,
                    3,
                    false,
                    false,
                    "General",
                );
                historical_decoder_before(
                    &mut expected_world,
                    &mut expected_rng,
                    &template,
                    3,
                    3,
                    false,
                    false,
                    "General",
                    None,
                );
                assert_eq!(
                    serde_json::to_vec(&actual_world).unwrap(),
                    serde_json::to_vec(&expected_world).unwrap()
                );
                assert_eq!(actual_rng.state_bits(), expected_rng.state_bits());
                assert_eq!(actual_rng.algorithm(), expected_rng.algorithm());
            }
        }
    }
}

// Sealed pre-change decoder. Only its name and the added false argument to
// ice's unchanged historical j bodies differ; no source dispatch is called.
fn historical_decoder_before(
    world: &mut World,
    rng: &mut Rng,
    template: &str,
    origin_x: usize,
    origin_y: usize,
    is_start: bool,
    _is_end: bool,
    shop_type: &str,
    source_damsel: Option<bool>,
) {
    let name = world.name.to_ascii_lowercase();
    let yeti = name.contains("yeti");
    let market = name.contains("market");
    for (index, symbol) in template.bytes().take(80).enumerate() {
        let tx = origin_x + index % 10;
        let ty = origin_y + index / 10;
        if tx >= world.width || ty >= world.height {
            continue;
        }
        // Positions are source instance origins; render/collision applies each sprite's origin.
        let p = Vec2::new(tx as f64 * 16.0, ty as f64 * 16.0);
        let occupied = solid_point(world, p);
        match symbol {
            b'1' | b'2' => {
                if symbol == b'2' && !rng.chance(1, 2) {
                    continue;
                }
                if !occupied {
                    let object = ground(world.area, yeti, symbol == b'1', rng);
                    world.set_tile(tx, ty, object);
                }
            }
            b'9' => {
                let floor = match world.area {
                    Area::Mines => "oBrick",
                    Area::Jungle => "oLush",
                    Area::Ice => "oDark",
                    _ => "oTemple",
                };
                world.set_tile(tx, ty + 1, floor);
                let id = spawn(world, rng, if is_start { "oEntrance" } else { "oExit" }, p);
                if is_start {
                    world.entrance = offset(p, 8.0, 8.0);
                } else {
                    world.exit = offset(p, 8.0, 8.0);
                }
                world
                    .entity_mut(id)
                    .unwrap()
                    .data
                    .insert("level_exit".into(), if is_start { 0.0 } else { 1.0 });
                if !is_start || world.area == Area::Boss {
                    terrain_property(world, tx, ty + 1, "invincible", 1.0);
                }
            }
            b'L' | b'P' => {
                let object = match (world.area, yeti, symbol) {
                    (Area::Jungle, _, b'L') | (Area::Ice, false, _) | (Area::Boss, _, b'L') => {
                        "oVine"
                    }
                    (Area::Jungle, _, _) | (Area::Boss, _, _) => "oVineTop",
                    (_, _, b'L') => "oLadderOrange",
                    _ => "oLadderTop",
                };
                climbable(world, rng, tx, ty, object, p);
            }
            b'7' => {
                if (world.area == Area::Ice && !yeti) || rng.chance(1, 3) {
                    spawn(world, rng, "oSpikes", p);
                }
            }
            b'4' => {
                if world.area == Area::Jungle || rng.chance(1, 4) {
                    spawn(world, rng, "oPushBlock", p);
                }
            }
            b'x' if world.area != Area::Boss => altar(world, rng, p, true),
            b'I' => {
                let dy = if world.area == Area::Boss {
                    rng.range(1);
                    0.0
                } else if yeti || market {
                    8.0
                } else {
                    12.0
                };
                spawn(world, rng, "oGoldIdol", offset(p, 16.0, dy));
            }
            b'.' if world.area != Area::Boss => {
                if !occupied {
                    let object = match world.area {
                        Area::Mines => {
                            if rng.chance(1, 10) {
                                "oBlock"
                            } else {
                                "oBrick"
                            }
                        }
                        Area::Jungle => "oLush",
                        Area::Ice => "oDark",
                        _ => "oTemple",
                    };
                    world.set_tile(tx, ty, object);
                    terrain_property(world, tx, ty, "shopWall", 1.0);
                }
            }
            b'+' if world.area != Area::Boss => {
                world.set_tile(tx, ty, "oSolid");
                let id = terrain_property(world, tx, ty, "shopWall", 1.0);
                world
                    .entity_mut(id)
                    .unwrap()
                    .text
                    .insert("terrain_sprite".into(), "sIceBlock".into());
            }
            b'K' if world.area != Area::Boss => {
                let id = spawn(world, rng, "oShopkeeper", p);
                world
                    .entity_mut(id)
                    .unwrap()
                    .text
                    .insert("style".into(), shop_type.into());
            }
            b'k' if world.area != Area::Boss => {
                let id = spawn(world, rng, "oSign", p);
                world.entity_mut(id).unwrap().sprite = format!("sSign{shop_type}");
            }
            b'l' if world.area != Area::Boss => {
                spawn(
                    world,
                    rng,
                    if source_damsel.unwrap_or(shop_type == "Kissing")
                        && world.area != Area::Temple
                        && !market
                    {
                        "oLampRed"
                    } else {
                        "oLamp"
                    },
                    p,
                );
            }
            b'Q' if shop_type == "Craps" && world.area != Area::Boss => {
                decoration(world, rng, "bgDiceSign", p);
            }
            // The coordinator applies run crime/character globals after room
            // stamping, retaining this exact template-authored position.
            b'W' => {
                let id = decoration(world, rng, "bgWanted", p);
                world
                    .entity_mut(id)
                    .unwrap()
                    .data
                    .insert("wanted_marker".into(), 1.0);
            }
            b'q' if world.area != Area::Boss => {
                rng.range(6); // Source's unused dice roll still consumes a draw.
                let object = high_end(rng);
                let id = spawn(world, rng, object, offset(p, 8.0, 8.0));
                world
                    .entity_mut(id)
                    .unwrap()
                    .data
                    .insert("inDiceHouse".into(), 1.0);
            }
            b'i' if world.area != Area::Ice && world.area != Area::Boss => {
                shop_item(world, rng, p, shop_type)
            }
            b'$' if world.area == Area::Ice => shop_item(world, rng, p, shop_type),
            b'd' if world.area == Area::Mines || yeti => {
                let id = spawn(world, rng, "oDice", offset(p, 8.0, 8.0));
                crate::items::initialize_dice(rng, world.entity_mut(id).unwrap());
            }
            b'u' if !yeti && matches!(world.area, Area::Jungle | Area::Ice | Area::Temple) => {
                let id = spawn(world, rng, "oDice", offset(p, 8.0, 8.0));
                crate::items::initialize_dice(rng, world.entity_mut(id).unwrap());
            }
            b'z' if market => {
                let id = spawn(world, rng, "oDice", offset(p, 8.0, 8.0));
                crate::items::initialize_dice(rng, world.entity_mut(id).unwrap());
            }
            b'D' if world.area == Area::Mines || yeti || world.area == Area::Jungle && !market => {
                sale_damsel(world, rng, p)
            }
            b'!' if world.area == Area::Ice && !yeti => sale_damsel(world, rng, p),
            b'A' if world.area == Area::Temple => sale_damsel(world, rng, p),
            b'b' if world.area != Area::Boss && (world.area != Area::Ice || yeti) => {
                let tile = if world.area == Area::Temple {
                    "oTemple"
                } else {
                    "oBrickSmooth"
                };
                world.set_tile(tx, ty, tile);
                let id = terrain_property(world, tx, ty, "shopWall", 1.0);
                let sprite = match world.area {
                    Area::Jungle => "sLushSmooth",
                    Area::Ice => "sDarkSmooth",
                    Area::Temple => "sTemple",
                    _ => "sCaveSmooth",
                };
                world
                    .entity_mut(id)
                    .unwrap()
                    .text
                    .insert("terrain_sprite".into(), sprite.into());
            }
            b':' if world.area == Area::Ice && !yeti => {
                world.set_tile(tx, ty, "oBrickSmooth");
                let id = terrain_property(world, tx, ty, "shopWall", 1.0);
                world
                    .entity_mut(id)
                    .unwrap()
                    .text
                    .insert("terrain_sprite".into(), "sDarkSmooth".into());
            }
            _ => match world.area {
                Area::Mines => mines(world, rng, symbol, tx, ty, p, None),
                Area::Jungle => jungle(world, rng, symbol, tx, ty, p, market, occupied, None),
                Area::Ice => ice(world, rng, symbol, tx, ty, p, yeti, occupied, false, None),
                Area::Temple => temple(world, rng, symbol, tx, ty, p, occupied, None),
                Area::Boss => boss(world, rng, symbol, tx, ty, p, None),
            },
        }
    }
}
