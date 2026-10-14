//! Tile decoding from Classic's scrRoomGen*, preserving area-dependent alphabets.
use crate::model::{Area, Rng, Vec2, World};

pub(super) fn stamp(
    world: &mut World,
    rng: &mut Rng,
    template: &str,
    room_x: usize,
    room_y: usize,
    is_start: bool,
    is_end: bool,
    shop_type: &str,
) {
    stamp_at(
        world,
        rng,
        template,
        1 + room_x * 10,
        1 + room_y * 8,
        is_start,
        is_end,
        shop_type,
    );
}

/// Versioned source selector globals, consumed at the actual instance births.
/// Historical callers retain the old adapter without an invented latch.
/// Source ground-Lush Create receives the actual campaign equipment context.
pub(super) fn stamp_with_source_context(
    world: &mut World,
    rng: &mut Rng,
    template: &str,
    room_x: usize,
    room_y: usize,
    is_start: bool,
    is_end: bool,
    shop_type: &str,
    damsel: bool,
    context: super::GenerationContext,
) {
    stamp_at_source_context(
        world,
        rng,
        template,
        1 + room_x * 10,
        1 + room_y * 8,
        is_start,
        is_end,
        shop_type,
        damsel,
        context,
    );
}
pub(super) fn stamp_at_source_context(
    world: &mut World,
    rng: &mut Rng,
    template: &str,
    origin_x: usize,
    origin_y: usize,
    is_start: bool,
    is_end: bool,
    shop_type: &str,
    damsel: bool,
    context: super::GenerationContext,
) {
    stamp_at_with_damsel(
        world,
        rng,
        template,
        origin_x,
        origin_y,
        is_start,
        is_end,
        shop_type,
        Some(damsel),
        context,
    );
}

pub(super) fn stamp_with_damsel(
    world: &mut World,
    rng: &mut Rng,
    template: &str,
    room_x: usize,
    room_y: usize,
    is_start: bool,
    is_end: bool,
    shop_type: &str,
    damsel: bool,
) {
    stamp_at_with_damsel(
        world,
        rng,
        template,
        1 + room_x * 10,
        1 + room_y * 8,
        is_start,
        is_end,
        shop_type,
        Some(damsel),
        super::GenerationContext::default(),
    );
}

pub(super) fn stamp_at(
    world: &mut World,
    rng: &mut Rng,
    template: &str,
    origin_x: usize,
    origin_y: usize,
    is_start: bool,
    is_end: bool,
    shop_type: &str,
) {
    stamp_at_with_damsel(
        world,
        rng,
        template,
        origin_x,
        origin_y,
        is_start,
        is_end,
        shop_type,
        None,
        super::GenerationContext::default(),
    );
}

/// SourceClassicV2 Boss and direct-source callers select the same explicit
/// source decoder mode; historical stamp_at retains its old symbol tape.
pub(super) fn stamp_at_source(
    world: &mut World,
    rng: &mut Rng,
    template: &str,
    origin_x: usize,
    origin_y: usize,
    is_start: bool,
    is_end: bool,
    shop_type: &str,
    damsel: bool,
) {
    stamp_at_with_damsel(
        world,
        rng,
        template,
        origin_x,
        origin_y,
        is_start,
        is_end,
        shop_type,
        Some(damsel),
        super::GenerationContext::default(),
    );
}

fn stamp_at_with_damsel(
    world: &mut World,
    rng: &mut Rng,
    template: &str,
    origin_x: usize,
    origin_y: usize,
    is_start: bool,
    _is_end: bool,
    shop_type: &str,
    source_damsel: Option<bool>,
    terrain_context: super::GenerationContext,
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
        if source_damsel.is_some() {
            stamp_source_tile(
                world,
                rng,
                symbol,
                tx,
                ty,
                p,
                is_start,
                shop_type,
                source_damsel.unwrap(),
                yeti,
                market,
                terrain_context,
            );
        } else {
            let occupied = solid_point(world, p);
            stamp_tile_body(
                world,
                rng,
                symbol,
                tx,
                ty,
                p,
                is_start,
                shop_type,
                None,
                yeti,
                market,
                occupied,
                false,
                terrain_context,
            );
        }
    }
}

fn stamp_tile_body(
    world: &mut World,
    rng: &mut Rng,
    symbol: u8,
    tx: usize,
    ty: usize,
    p: Vec2,
    is_start: bool,
    shop_type: &str,
    source_damsel: Option<bool>,
    yeti: bool,
    market: bool,
    occupied: bool,
    predicates_prechecked: bool,
    terrain_context: super::GenerationContext,
) {
    let source_context = predicates_prechecked.then_some(terrain_context);
    match symbol {
        b'1' | b'2' => {
            if symbol == b'2' && !predicates_prechecked && !rng.chance(1, 2) {
                return;
            }
            if !occupied {
                let object = ground(world.area, yeti, symbol == b'1', rng);
                room_terrain(world, rng, tx, ty, object, source_context);
            }
        }
        b'9' => {
            let floor = match world.area {
                Area::Mines => "oBrick",
                Area::Jungle => "oLush",
                Area::Ice => "oDark",
                _ => "oTemple",
            };
            // scrRoomGen5 uniquely births the door BEFORE its support.
            let door_first = source_context.is_some() && world.area == Area::Boss;
            if !door_first {
                room_terrain(world, rng, tx, ty + 1, floor, source_context);
            }
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
            if door_first {
                room_terrain(world, rng, tx, ty + 1, floor, source_context);
            }
            if !is_start || world.area == Area::Boss {
                terrain_property(world, tx, ty + 1, "invincible", 1.0);
            }
        }
        b'L' | b'P' => {
            let object = match (world.area, yeti, symbol) {
                (Area::Jungle, _, b'L') | (Area::Ice, false, _) | (Area::Boss, _, b'L') => "oVine",
                (Area::Jungle, _, _) | (Area::Boss, _, _) => "oVineTop",
                (_, _, b'L') => "oLadderOrange",
                _ => "oLadderTop",
            };
            climbable(world, rng, tx, ty, object, p);
        }
        b'7' => {
            if predicates_prechecked || (world.area == Area::Ice && !yeti) || rng.chance(1, 3) {
                spawn(world, rng, "oSpikes", p);
            }
        }
        b'4' => {
            if predicates_prechecked || world.area == Area::Jungle || rng.chance(1, 4) {
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
                room_terrain(world, rng, tx, ty, object, source_context);
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
            room_terrain(world, rng, tx, ty, tile, source_context);
            let id = terrain_property(world, tx, ty, "shopWall", 1.0);
            let sprite = match world.area {
                Area::Jungle => "sLushSmooth",
                Area::Ice => "sDarkSmooth",
                Area::Temple => "sTemple",
                _ => "sCaveSmooth",
            };
            // scrRoomGen4's b caller assigns only shopWall. Its own Temple
            // variant (including CityGold) survives; Historical keeps the adapter.
            if !(source_context.is_some() && world.area == Area::Temple) {
                world
                    .entity_mut(id)
                    .unwrap()
                    .text
                    .insert("terrain_sprite".into(), sprite.into());
            }
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
            Area::Mines => mines(world, rng, symbol, tx, ty, p, source_context),
            Area::Jungle => jungle(
                world,
                rng,
                symbol,
                tx,
                ty,
                p,
                market,
                occupied,
                source_context,
            ),
            Area::Ice => ice(
                world,
                rng,
                symbol,
                tx,
                ty,
                p,
                yeti,
                occupied,
                predicates_prechecked,
                source_context,
            ),
            Area::Temple => temple(world, rng, symbol, tx, ty, p, occupied, source_context),
            Area::Boss => boss(world, rng, symbol, tx, ty, p, source_context),
        },
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TileDialect {
    Mines,
    Jungle,
    Ice,
    Temple,
    Boss,
    Market,
    Yeti,
}
fn tile_dialect(area: Area, yeti: bool, market: bool) -> TileDialect {
    match area {
        Area::Mines => TileDialect::Mines,
        Area::Jungle if market => TileDialect::Market,
        Area::Jungle => TileDialect::Jungle,
        Area::Ice if yeti => TileDialect::Yeti,
        Area::Ice => TileDialect::Ice,
        Area::Temple => TileDialect::Temple,
        Area::Boss => TileDialect::Boss,
    }
}
fn source_statement_groups(dialect: TileDialect) -> &'static [&'static [u8]] {
    match dialect {
        TileDialect::Mines => &[b"12LP749AxaIBQq+W.blKkidDsSTM"],
        TileDialect::Jungle => &[b"12", b"t", b"r3LP7s49cdwv,JIC.Qq+WblKkiuDBxpT"],
        TileDialect::Ice => &[b"12LP749aI.Qq+W:lKk$u!YABbCDEGXxTtcMmijfw"],
        TileDialect::Temple => &[b"123RLP749actxXI;BCDA?.Qq+WblpKkiwudeT"],
        TileDialect::Boss => &[b"12LP749aTtICDw"],
        TileDialect::Market => &[b"12", b"t3LPGH7s49cdwI.+qQWblKkiAzBpT"],
        TileDialect::Yeti => &[b"1", b"2", b"tLP749AaIB.Qq+WblKk$dDsSTijYy"],
    }
}
fn source_random_predicate(dialect: TileDialect, symbol: u8) -> Option<u32> {
    match symbol {
        b'2' => Some(2),
        b'7' if dialect != TileDialect::Ice => Some(3),
        b'4' if !matches!(dialect, TileDialect::Jungle | TileDialect::Market) => Some(4),
        b'j' if matches!(dialect, TileDialect::Ice | TileDialect::Yeti) => Some(2),
        _ => None,
    }
}
fn source_clear_predicate(dialect: TileDialect, symbol: u8) -> bool {
    match symbol {
        b'1' | b'2' | b'.' => true,
        b't' => matches!(
            dialect,
            TileDialect::Jungle | TileDialect::Market | TileDialect::Yeti
        ),
        b'r' => dialect == TileDialect::Jungle,
        b'3' => matches!(
            dialect,
            TileDialect::Jungle | TileDialect::Market | TileDialect::Temple
        ),
        b'R' => dialect == TileDialect::Temple,
        b'M' | b'm' => dialect == TileDialect::Ice,
        _ => false,
    }
}

// Each reached Boolean predicate evaluates its random operand before deciding
// whether the symbol/clearance guards pass. Successful branches end only their
// own elseif chain; independent later if statements still execute. Do not move
// all draws ahead of bodies: Yeti's first body draws before the separate 2 test.
fn stamp_source_tile(
    world: &mut World,
    rng: &mut Rng,
    symbol: u8,
    tx: usize,
    ty: usize,
    p: Vec2,
    is_start: bool,
    shop_type: &str,
    damsel: bool,
    yeti: bool,
    market: bool,
    terrain_context: super::GenerationContext,
) {
    let dialect = tile_dialect(world.area, yeti, market);
    for group in source_statement_groups(dialect) {
        for &candidate in *group {
            let random_pass = source_random_predicate(dialect, candidate)
                .is_none_or(|width| rng.range(width) == 0);
            // Evaluate each reached source collision operand, even when the
            // random/symbol operand failed; plain clauses have no such query.
            let clear_pass = !source_clear_predicate(dialect, candidate) || !solid_point(world, p);
            if candidate == symbol && random_pass && clear_pass {
                let occupied = solid_point(world, p);
                stamp_tile_body(
                    world,
                    rng,
                    symbol,
                    tx,
                    ty,
                    p,
                    is_start,
                    shop_type,
                    Some(damsel),
                    yeti,
                    market,
                    occupied,
                    true,
                    terrain_context,
                );
                break;
            }
        }
    }
}

#[cfg(test)]
#[path = "source_tile_predicate_tests.rs"]
mod source_tile_predicate_tests;

/// Complete represented source room-terrain Create before returning to its
/// literal caller. Historical decoding retains its deferred constructor tape.
fn room_terrain(
    world: &mut World,
    rng: &mut Rng,
    x: usize,
    y: usize,
    object: &str,
    source_context: Option<super::GenerationContext>,
) {
    world.set_tile(x, y, object);
    if let Some(context) = source_context
        && matches!(
            object,
            "oBrick" | "oLush" | "oDark" | "oTemple" | "oIce" | "oBlock"
        )
        && x < world.width
        && y < world.height
    {
        super::terrain::create_source_room_terrain(world, rng, x, y, context);
    }
}

fn ground(area: Area, yeti: bool, certain: bool, rng: &mut Rng) -> &'static str {
    match area {
        Area::Mines => {
            if rng.chance(1, 10) {
                "oBlock"
            } else {
                "oBrick"
            }
        }
        Area::Jungle => "oLush",
        Area::Ice if yeti => {
            if rng.chance(1, 6) {
                "oDark"
            } else {
                "oIce"
            }
        }
        Area::Ice => {
            if rng.chance(1, 10) {
                "oIce"
            } else {
                "oDark"
            }
        }
        Area::Temple if certain && rng.chance(1, 100) => "oLush",
        _ => {
            if rng.chance(1, 10) {
                "oBlock"
            } else {
                "oTemple"
            }
        }
    }
}

fn mines(
    w: &mut World,
    r: &mut Rng,
    c: u8,
    x: usize,
    y: usize,
    p: Vec2,
    source_context: Option<super::GenerationContext>,
) {
    match c {
        b'A' => altar(w, r, p, false),
        b'a' => {
            spawn(w, r, "oChest", p);
        }
        b'B' => {
            spawn(w, r, "oGiantTikiHead", offset(p, 16.0, 12.0));
            let body = decoration(w, r, "bgTiki", offset(p, 0.0, 32.0));
            w.entity_mut(body)
                .unwrap()
                .data
                .insert("depth".into(), 10001.0);
            for (dx, source_y) in [(32.0, 0.0), (-16.0, 16.0)] {
                let source_x = f64::from(r.range(3)) * 16.0;
                let arm = decoration(w, r, "bgTikiArms", offset(p, dx, 32.0));
                let arm = w.entity_mut(arm).unwrap();
                for (key, value) in [
                    ("depth", 10001.0),
                    ("source_x", source_x),
                    ("source_y", source_y),
                    ("source_w", 16.0),
                    ("source_h", 16.0),
                ] {
                    arm.data.insert(key.into(), value);
                }
            }
        }
        b's' => {
            if r.chance(1, 10) {
                spawn(w, r, "oSnake", p);
            } else if r.chance(1, 2) {
                room_terrain(w, r, x, y, "oBrick", source_context);
            }
        }
        b'S' => {
            spawn(w, r, "oSnake", p);
        }
        b'T' => {
            spawn(w, r, "oRubyBig", offset(p, 8.0, 8.0));
        }
        b'M' => {
            room_terrain(w, r, x, y, "oBrick", source_context);
            free_item(w, r, "oMattock", offset(p, 8.0, 8.0));
        }
        _ => {}
    }
}

fn jungle(
    w: &mut World,
    r: &mut Rng,
    c: u8,
    x: usize,
    y: usize,
    p: Vec2,
    market: bool,
    occupied: bool,
    source_context: Option<super::GenerationContext>,
) {
    match c {
        b't' if !occupied => room_terrain(w, r, x, y, "oTemple", source_context),
        b'r' if !market && !occupied => {
            let object = if r.chance(1, 2) { "oTemple" } else { "oLush" };
            room_terrain(w, r, x, y, object, source_context);
        }
        b'3' if !occupied => {
            if r.chance(1, 2) {
                spawn(w, r, "oWaterSwim", p);
            } else {
                room_terrain(w, r, x, y, "oLush", source_context);
            }
        }
        b's' => {
            spawn(w, r, "oSpikes", p);
        }
        b'G' if market => climbable(w, r, x, y, "oLadderOrange", p),
        b'H' if market => climbable(w, r, x, y, "oLadderTop", p),
        b'c' => {
            spawn(w, r, "oChest", p);
        }
        b'd' => {
            spawn(w, r, "oWaterSwim", p);
            spawn(w, r, "oChest", p);
        }
        b'w' => {
            spawn(w, r, "oWaterSwim", p);
        }
        b'v' if !market => {
            spawn(w, r, "oWaterSwim", p);
            room_terrain(w, r, x, y, "oLush", source_context);
        }
        b',' if !market => {
            spawn(w, r, "oWaterSwim", p);
            if r.chance(1, 2) {
                room_terrain(w, r, x, y, "oLush", source_context);
            }
        }
        b'J' if !market => {
            spawn(w, r, "oWaterSwim", p);
            spawn(w, r, "oJaws", p);
        }
        b'C' if !market => {
            spawn(w, r, "oCrystalSkull", offset(p, 16.0, 12.0));
        }
        b'A' if market => {
            let id = spawn(w, r, "oAnkh", offset(p, 8.0, 8.0));
            w.entity_mut(id)
                .unwrap()
                .data
                .insert("cost".into(), 50000.0);
            w.entity_mut(id).unwrap().data.insert("forSale".into(), 1.0);
        }
        b'B' => {
            let id = spawn(w, r, "oTrapBlock", p);
            if !market {
                let idol = w
                    .entities
                    .iter()
                    .find(|e| e.object == "oGoldIdol")
                    .map_or(p.x, |e| e.pos.x);
                w.entity_mut(id).unwrap().data.insert(
                    "deathTimer".into(),
                    (40.0 - (p.x - (idol - 8.0)).abs()).max(0.0),
                );
            }
        }
        // rand(1,2) is always nonzero in the original condition: jars are unreachable here.
        b'p' => {
            r.range(2);
            spawn(w, r, "oFakeBones", p);
        }
        b'T' => tree(w, r, x, y, p),
        _ => {}
    }
}

fn ice(
    w: &mut World,
    r: &mut Rng,
    c: u8,
    x: usize,
    y: usize,
    p: Vec2,
    yeti: bool,
    occupied: bool,
    predicates_prechecked: bool,
    source_context: Option<super::GenerationContext>,
) {
    if yeti {
        match c {
            b't' if !occupied => {
                spawn(w, r, "oThinIce", p);
            }
            b'A' => altar(w, r, p, false),
            b'a' => {
                spawn(w, r, "oChest", p);
            }
            b'B' => {
                spawn(w, r, "oGiantTikiHead", offset(p, 16.0, 12.0));
            }
            b's' => {
                spawn(w, r, "oSpikes", p);
            }
            b'S' => {
                spawn(w, r, "oSnake", p);
            }
            b'T' => {
                spawn(w, r, "oRubyBig", offset(p, 8.0, 8.0));
            }
            b'i' => room_terrain(w, r, x, y, "oIce", source_context),
            b'j' if predicates_prechecked || r.chance(1, 2) => {
                room_terrain(w, r, x, y, "oIce", source_context)
            }
            b'Y' => {
                spawn(w, r, "oYetiKing", p);
            }
            b'y' => {
                spawn(w, r, "oYeti", p);
            }
            _ => {}
        }
        return;
    }
    match c {
        b'a' => {
            r.range(1);
            spawn(w, r, "oChest", p);
        }
        b'Y' => {
            spawn(w, r, "oYeti", p);
        }
        b'A' => ship(w, r, x, y, p, "sAlienTop"),
        b'B' => ship(w, r, x, y, p, "sAlienFloor"),
        b'b' => {
            if r.chance(1, 2) {
                let obj = ground(Area::Ice, false, true, r);
                room_terrain(w, r, x, y, obj, source_context);
            } else {
                ship(w, r, x, y, p, "sAlienFloor");
            }
        }
        b'C' => {
            ship(w, r, x, y, p, "sAlienFront");
            ship(w, r, x, y + 1, offset(p, 0.0, 16.0), "sAlienFront");
            ship(w, r, x, y + 2, offset(p, 0.0, 32.0), "sAlienFront2");
        }
        b'D' => ship(w, r, x, y, p, "sAlienFront3"),
        b'E' => {
            let id = decoration(w, r, "bgAlienShip", p);
            let frame = if r.chance(1, 3) { 1.0 } else { 0.0 };
            w.entity_mut(id)
                .unwrap()
                .data
                .insert("variant".into(), frame);
        }
        b'G' => {
            decoration(w, r, "bgAlienShip2", p);
        }
        b'X' => {
            spawn(w, r, "oAlienBoss", p);
        }
        b'T' => {
            room_terrain(w, r, x, y, "oDark", source_context);
            free_item(w, r, "oJetpack", offset(p, 8.0, 8.0));
        }
        b't' => {
            spawn(w, r, "oBarrierEmitter", p);
        }
        b'c' => {
            spawn(w, r, "oThinIce", p);
        }
        b'M' if !occupied => {
            for (dx, object) in [(0, "oMoai"), (1, "oMoai2"), (2, "oMoai3")] {
                let id = spawn(w, r, object, offset(p, dx as f64 * 16.0, 0.0));
                w.entity_mut(id)
                    .unwrap()
                    .data
                    .insert("invincible".into(), 1.0);
            }
            let id = spawn(w, r, "oMoaiInside", offset(p, 16.0, 16.0));
            w.entity_mut(id)
                .unwrap()
                .data
                .insert("invincible".into(), 1.0);
            spawn(w, r, "oThinIce", offset(p, 16.0, 32.0));
            let id = spawn(w, r, "oExit", offset(p, 16.0, 48.0));
            w.entity_mut(id)
                .unwrap()
                .text
                .insert("type".into(), "Moai Exit".into());
            free_item(w, r, "oCrown", offset(p, 24.0, 56.0));
        }
        b'm' if !occupied => {
            room_terrain(w, r, x, y, "oDark", source_context);
            terrain_property(w, x, y, "invincible", 1.0);
        }
        b'i' => room_terrain(w, r, x, y, "oIce", source_context),
        b'j' if predicates_prechecked || r.chance(1, 2) => {
            room_terrain(w, r, x, y, "oIce", source_context)
        }
        b'f' => {
            spawn(w, r, "oDarkFall", p);
        }
        b'w' => {
            spawn(w, r, "oWaterSwim", p);
        }
        _ => {}
    }
}

fn temple(
    w: &mut World,
    r: &mut Rng,
    c: u8,
    x: usize,
    y: usize,
    p: Vec2,
    occupied: bool,
    source_context: Option<super::GenerationContext>,
) {
    match c {
        b'3' if !occupied => {
            if r.chance(1, 2) {
                spawn(w, r, "oLava", p);
            } else {
                room_terrain(w, r, x, y, "oTemple", source_context);
            }
        }
        b'R' if !occupied => {
            room_terrain(w, r, x, y, "oTemple", source_context);
            terrain_treasure(w, x, y, "Big Ruby");
        }
        b'a' => {
            spawn(w, r, "oChest", offset(p, 8.0, 8.0));
        }
        b'c' => {
            let object = if r.chance(1, 2) { "oChest" } else { "oCrate" };
            spawn(w, r, object, offset(p, 8.0, 8.0));
        }
        b't' => {
            let object = if r.chance(1, 120) {
                "oRubyBig"
            } else if r.chance(1, 80) {
                "oSapphireBig"
            } else if r.chance(1, 60) {
                "oEmeraldBig"
            } else {
                "oGoldBars"
            };
            spawn(
                w,
                r,
                object,
                offset(p, 8.0, if object == "oGoldBars" { 8.0 } else { 12.0 }),
            );
        }
        b'X' => {
            for dy in 0..6 {
                for dx in 0..5 {
                    w.set_tile(x + dx, y + dy, "oXocBlock");
                    let treasure = match (dx, dy) {
                        (2, 1) => "Diamond",
                        (1 | 3, 2) => "Sapphire",
                        (0 | 4, 3) => "Emerald",
                        (2, 4) => "Ruby",
                        _ => "",
                    };
                    if !treasure.is_empty() {
                        terrain_treasure(w, x + dx, y + dy, treasure);
                    }
                    let id = decoration(
                        w,
                        r,
                        "bgLadyXoc",
                        offset(p, dx as f64 * 16.0, dy as f64 * 16.0),
                    );
                    let mural = w.entity_mut(id).unwrap();
                    for (key, value) in [
                        ("source_x", dx as f64 * 16.0),
                        ("source_y", dy as f64 * 16.0),
                        ("source_w", 16.0),
                        ("source_h", 16.0),
                        ("depth", 99.0),
                    ] {
                        mural.data.insert(key.into(), value);
                    }
                }
            }
        }
        b';' => {
            free_item(w, r, "oDamsel", offset(p, 8.0, 8.0));
            spawn(w, r, "oGoldIdol", offset(p, 24.0, 12.0));
        }
        b'B' => {
            r.range(1);
            spawn(w, r, "oTrapBlock", p);
        }
        b'C' => {
            spawn(w, r, "oCeilingTrap", p);
        }
        b'D' => fake_door(w, r, p),
        b'?' => {
            spawn(w, r, "oTombLord", p);
        }
        b'p' => {
            spawn(w, r, "oLampRed", p);
        }
        b'w' => {
            spawn(w, r, "oLava", p);
        }
        b'd' => room_terrain(w, r, x, y, "oLush", source_context),
        b'e' if r.chance(1, 2) => room_terrain(w, r, x, y, "oLush", source_context),
        b'T' => tree(w, r, x, y, p),
        _ => {}
    }
}

fn boss(
    w: &mut World,
    r: &mut Rng,
    c: u8,
    x: usize,
    y: usize,
    p: Vec2,
    source_context: Option<super::GenerationContext>,
) {
    match c {
        b'a' => {
            r.range(1);
            spawn(w, r, "oChest", offset(p, 8.0, 8.0));
        }
        b'T' => {
            if r.chance(1, 15) {
                spawn(w, r, "oChest", offset(p, 8.0, 8.0));
            } else if r.chance(1, 6) {
                spawn(w, r, "oGoldBars", offset(p, 8.0, 8.0));
            } else if r.chance(1, 6) {
                spawn(w, r, "oEmeraldBig", offset(p, 8.0, 12.0));
            } else if r.chance(1, 8) {
                spawn(w, r, "oSapphireBig", offset(p, 8.0, 12.0));
            } else if r.chance(1, 10) {
                spawn(w, r, "oRubyBig", offset(p, 8.0, 12.0));
            } else if r.chance(1, 10) {
                spawn(w, r, "oCrate", offset(p, 8.0, 8.0));
            } else if r.chance(1, 10) {
                room_terrain(w, r, x, y, "oBlock", source_context);
            } else {
                room_terrain(w, r, x, y, "oTemple", source_context);
            }
        }
        b't' => {
            spawn(w, r, "oThwompTrap", p);
        }
        b'C' => {
            spawn(w, r, "oCeilingTrap", p);
        }
        b'D' => fake_door(w, r, p),
        b'w' => {
            spawn(w, r, "oWaterSwim", p);
        }
        _ => {}
    }
}

fn offset(p: Vec2, x: f64, y: f64) -> Vec2 {
    Vec2::new(p.x + x, p.y + y)
}
fn spawn(w: &mut World, r: &mut Rng, object: &str, p: Vec2) -> u64 {
    let id = crate::creation::spawn_generated_entity(w, r, object, p);
    let e = w.entity_mut(id).unwrap();
    let sprite = match object {
        "oTreeBranch" => "sTreeBranchRight",
        "oCeilingTrap" => "sBlock",
        "oTempleFake" => "sTemple",
        "oWaterSwim" => "sWater",
        "oKaliHead" => "sKaliHead1",
        "oThinIce" => "sThinIce1",
        "oDice" => "sDice1",
        "oBow" => "sBowRight",
        "oMattock" => "sMattockRight",
        "oPistol" => "sPistolRight",
        "oShotgun" => "sShotgunRight",
        "oMachete" => "sMacheteRight",
        "oWebCannon" => "sWebCannonR",
        "oDamsel" => "sDamselLeft",
        "oShopkeeper" => "sShopLeft",
        "oTrapBlock" => "sSkullBlock",
        "oGloves" => "sGlove",
        "oFakeBones" => "sFakeBonesLeft",
        "oCapePickup" => "sCape",
        "oJetpack" => "sJetpackRight",
        "oYeti" => "sYetiLeft",
        "oTombLord" => "sTombLordLeft",
        "oSnake" => "sSnakeLeft",
        "oYetiKing" => "sYetiKingLeft",
        "oJaws" => "sJawsLeft",
        _ => "",
    };
    if !sprite.is_empty() {
        e.sprite = sprite.into();
    }
    // Entity::new derives collision membership from the original parent chain.
    // In particular, oTreeBranch is a one-way oPlatform and oTempleFake is
    // decoration while its door remains; neither belongs to the oSolid family.
    if object == "oDarkFall" {
        for (key, value) in [
            ("collision_left", 0.0),
            ("collision_top", 0.0),
            ("collision_right", 16.0),
            ("collision_bottom", 8.0),
        ] {
            e.data.insert(key.into(), value);
        }
    }
    id
}

/// The source collision_point query includes previously created solid
/// instances, including moving blocks and the full four-tile Moai side masks.
/// Room edges themselves have no implicit source collision surface.
fn solid_point(world: &World, point: Vec2) -> bool {
    world.solid_point_except(point, 0)
}

fn climbable(world: &mut World, rng: &mut Rng, x: usize, y: usize, object: &str, point: Vec2) {
    // GameMaker can place a ladder atop an already created door support. Keep
    // both collision layers rather than replacing the support with a ladder.
    if world
        .tile(x as i32, y as i32)
        .is_some_and(|tile| tile.is_solid())
    {
        spawn(world, rng, object, point);
    } else {
        world.set_tile(x, y, object);
    }
}
fn free_item(w: &mut World, r: &mut Rng, object: &str, p: Vec2) {
    let id = spawn(w, r, object, p);
    let e = w.entity_mut(id).unwrap();
    e.data.insert("cost".into(), 0.0);
    e.data.insert("forSale".into(), 0.0);
}
fn sale_damsel(w: &mut World, r: &mut Rng, p: Vec2) {
    let cost = 15000.0 * w.level as f64;
    let id = spawn(w, r, "oDamsel", offset(p, 8.0, 8.0));
    let e = w.entity_mut(id).unwrap();
    e.data.insert("forSale".into(), 1.0);
    e.data.insert("status".into(), 5.0);
    e.state = 5;
    e.data.insert("cost".into(), cost);
}
fn fake_door(w: &mut World, r: &mut Rng, p: Vec2) {
    spawn(w, r, "oDoor", p);
    spawn(w, r, "oTempleFake", p);
    spawn(w, r, "oTempleFake", offset(p, 0.0, 16.0));
}
fn decoration(w: &mut World, r: &mut Rng, bg: &str, p: Vec2) -> u64 {
    let id = spawn(w, r, "oGenerationDecoration", p);
    let e = w.entity_mut(id).unwrap();
    e.sprite = bg.into();
    e.data.insert("decoration".into(), 1.0);
    id
}
fn altar(w: &mut World, r: &mut Rng, p: Vec2, sacrifice: bool) {
    let (left, right) = if sacrifice {
        ("oSacAltarLeft", "oSacAltarRight")
    } else {
        ("oAltarLeft", "oAltarRight")
    };
    for (object, dx) in [(left, 0.0), (right, 16.0)] {
        let q = offset(p, dx, 0.0);
        w.set_tile((q.x / 16.0) as usize, (q.y / 16.0) as usize, object);
    }
    if sacrifice {
        decoration(w, r, "bgKaliBody", offset(p, -16.0, -48.0));
        let head = spawn(w, r, "oKaliHead", offset(p, 16.0, -64.0));
        crate::traps::initialize_kali_head(w, r, head);
    }
}

// World tiles currently carry names only. Invisible overlays preserve source per-instance properties.
pub(super) fn terrain_property(w: &mut World, x: usize, y: usize, key: &str, value: f64) -> u64 {
    let p = Vec2::new(x as f64 * 16.0 + 8.0, y as f64 * 16.0 + 8.0);
    let existing = w
        .entities
        .iter()
        .find(|e| e.object == "oTerrainMetadata" && e.pos == p)
        .map(|e| e.id);
    let id = existing.unwrap_or_else(|| {
        let id = w.spawn("oTerrainMetadata", p);
        w.entity_mut(id).unwrap().sprite.clear();
        id
    });
    w.entity_mut(id).unwrap().data.insert(key.into(), value);
    id
}
fn terrain_treasure(w: &mut World, x: usize, y: usize, treasure: &str) {
    let id = terrain_property(w, x, y, "embedded_treasure", 1.0);
    w.entity_mut(id)
        .unwrap()
        .text
        .insert("treasure".into(), treasure.into());
}
fn ship(w: &mut World, r: &mut Rng, x: usize, y: usize, p: Vec2, sprite: &str) {
    w.set_tile(x, y, "oAlienShip");
    let id = terrain_property(w, x, y, "alien_ship", 1.0);
    w.entity_mut(id)
        .unwrap()
        .text
        .insert("terrain_sprite".into(), sprite.into());
    decoration(w, r, "bgAlienShip3", p);
}

fn tree(w: &mut World, r: &mut Rng, x: usize, y: usize, p: Vec2) {
    w.set_tile(x, y, "oTree");
    crate::world_events::initialize_tree(w, x, y);
    let mut ty = y as i32 - 1;
    let mut right = false;
    let mut left = false;
    for m in 0..5 {
        if r.range(m + 1) > 2 {
            break;
        }
        if [-1, 0, 1].iter().any(|dx| {
            solid_point(
                w,
                Vec2::new((x as i32 + dx) as f64 * 16.0, (ty - 1) as f64 * 16.0),
            )
        }) {
            break;
        }
        if ty < 0 {
            break;
        }
        w.set_tile(x, ty as usize, "oTree");
        crate::world_events::initialize_tree(w, x, ty as usize);
        if m < 4 {
            if r.range(5) < 3 && !right {
                spawn(w, r, "oTreeBranch", Vec2::new(p.x + 16.0, ty as f64 * 16.0));
                right = true;
            } else if right {
                right = false;
            }
            if r.range(5) < 3 && !left {
                spawn(w, r, "oTreeBranch", Vec2::new(p.x - 16.0, ty as f64 * 16.0));
                left = true;
            } else if left {
                left = false;
            }
        }
        ty -= 1;
    }
    let top = Vec2::new(p.x, ty as f64 * 16.0 + 16.0);
    for dx in [-16.0, 16.0] {
        let id = spawn(w, r, "oLeaves", offset(top, dx, 0.0));
        // Create chooses its orientation immediately, using only instances
        // already present. The coordinator applies the cemetery sprite variant
        // before the first frame, without changing these equal-sized masks.
        crate::world_events::initialize_leaves(w, id, false);
    }
}

fn high_end(r: &mut Rng) -> &'static str {
    for (object, denominator) in [
        ("oJetpack", 40),
        ("oCapePickup", 25),
        ("oShotgun", 20),
        ("oGloves", 10),
        ("oTeleporter", 10),
        ("oMattock", 8),
        ("oPaste", 8),
        ("oSpringShoes", 8),
        ("oSpikeShoes", 8),
        ("oCompass", 8),
        ("oPistol", 8),
        ("oMachete", 8),
    ] {
        if r.chance(1, denominator) {
            return object;
        }
    }
    "oBombBox"
}

fn shop_item(w: &mut World, r: &mut Rng, p: Vec2, style: &str) {
    let mut attempts = 20_u32;
    let object = loop {
        let candidate = match style {
            "Bomb" => {
                if r.chance(1, 5) {
                    "oPaste"
                } else if r.chance(1, 4) {
                    "oBombBox"
                } else {
                    "oBombBag"
                }
            }
            "Weapon" => {
                let n = r.range(4);
                if attempts == 0 {
                    "oBombBag"
                } else if r.chance(1, 12) {
                    "oWebCannon"
                } else if r.chance(1, 10) {
                    "oShotgun"
                } else if r.chance(1, 6) {
                    "oBombBox"
                } else {
                    ["oPistol", "oMachete", "oBombBag", "oBow"][n as usize]
                }
            }
            "Clothing" => {
                let n = r.range(6);
                if r.chance(1, attempts.max(1)) {
                    "oRopePile"
                } else {
                    [
                        "oSpringShoes",
                        "oSpectacles",
                        "oGloves",
                        "oMitt",
                        "oCapePickup",
                        "oSpikeShoes",
                    ][n as usize]
                }
            }
            "Rare" => {
                let n = r.range(11);
                if r.chance(1, attempts.max(1)) {
                    "oBombBox"
                } else {
                    [
                        "oSpringShoes",
                        "oCompass",
                        "oMattock",
                        "oSpectacles",
                        "oJetpack",
                        "oGloves",
                        "oMitt",
                        "oWebCannon",
                        "oCapePickup",
                        "oTeleporter",
                        "oSpikeShoes",
                    ][n as usize]
                }
            }
            _ => {
                let n = r.range(3);
                if r.chance(1, 20) {
                    "oMattock"
                } else if r.chance(1, 10) {
                    "oGloves"
                } else if r.chance(1, 10) {
                    "oCompass"
                } else {
                    ["oBombBag", "oRopePile", "oParaPickup"][n as usize]
                }
            }
        };
        let duplicate = w.entities.iter().any(|e| e.alive && e.object == candidate);
        if !duplicate
            || matches!(
                candidate,
                "oBombBag" | "oBombBox" | "oRopePile" | "oParaPickup"
            )
        {
            break candidate;
        }
        attempts = attempts.saturating_sub(1);
    };
    let dy = match object {
        "oBombBox" if style == "Weapon" => 10.0,
        "oGloves" | "oMitt" | "oJetpack" | "oBombBox" => 8.0,
        "oPistol" | "oMachete" | "oBow" | "oWebCannon" | "oShotgun" | "oTeleporter" => 12.0,
        "oRopePile" => 11.0,
        _ => 10.0,
    };
    let id = spawn(w, r, object, offset(p, 8.0, dy));
    let level = w.level;
    let e = w.entity_mut(id).unwrap();
    let base = match object {
        "oBombBag" | "oRopePile" => 2500.0,
        "oBombBox" | "oTeleporter" => 10000.0,
        "oPaste" | "oCompass" => 3000.0,
        "oSpringShoes" | "oPistol" => 5000.0,
        "oSpikeShoes" | "oMitt" => 4000.0,
        "oSpectacles" | "oGloves" | "oMattock" => 8000.0,
        "oJetpack" => 20000.0,
        "oCapePickup" => 12000.0,
        "oShotgun" => 15000.0,
        "oWebCannon" | "oParaPickup" => 2000.0,
        "oMachete" => 7000.0,
        "oBow" => 1000.0,
        _ => 0.0,
    };
    e.data.insert(
        "cost".into(),
        base * (1.0 + level.saturating_sub(2) as f64 * 0.1),
    );
    e.data.insert("forSale".into(), 1.0);
}

#[cfg(test)]
mod tests {
    use super::*;
    fn room(w: &mut World, symbols: &str) {
        let template = format!("{symbols:0<80}");
        stamp(
            w,
            &mut Rng::new(42),
            &template,
            0,
            0,
            false,
            false,
            "General",
        );
    }

    #[test]
    fn generated_damsel_create_precedes_free_and_shop_producer_overrides() {
        let mut world = World::new(12, 12, Area::Jungle, 5);
        let mut rng = Rng::new(17);
        free_item(&mut world, &mut rng, "oDamsel", Vec2::new(40.0, 40.0));
        sale_damsel(&mut world, &mut rng, Vec2::new(64.0, 32.0));
        let free = &world.entities[0];
        let shop = &world.entities[1];
        for damsel in [free, shop] {
            assert_eq!(damsel.value("npc_local_created"), 1.0);
        }
        assert_eq!(free.value("cost"), 0.0);
        assert_eq!(free.value("forSale"), 0.0);
        assert_eq!(shop.value("cost"), 75000.0);
        assert_eq!(shop.value("forSale"), 1.0);
        assert_eq!(shop.state, 5);
        assert_eq!(rng.next_u64(), Rng::new(17).next_u64());
    }

    #[test]
    fn generated_tree_metadata_keeps_bottom_to_top_source_creation_order() {
        let mut world = World::new(42, 34, Area::Jungle, 5);
        tree(
            &mut world,
            &mut Rng::new(42),
            10,
            10,
            Vec2::new(160.0, 160.0),
        );
        let trunks: Vec<_> = world
            .entities
            .iter()
            .enumerate()
            .filter(|(_, e)| e.object == "oTerrainMetadata" && e.value("tree_initialized") != 0.0)
            .collect();
        assert!(
            trunks.len() > 2,
            "fixture must exercise interleaved trunk and branch creation"
        );
        assert!(world.entities.iter().any(|e| e.object == "oTreeBranch"));
        for pair in trunks.windows(2) {
            assert_eq!(pair[0].1.pos.y - pair[1].1.pos.y, 16.0);
            assert!(pair[0].0 < pair[1].0);
        }
        for (index, branch) in world
            .entities
            .iter()
            .enumerate()
            .filter(|(_, e)| e.object == "oTreeBranch")
        {
            let trunk = trunks
                .iter()
                .find(|(_, trunk)| trunk.pos.y == branch.pos.y + 8.0)
                .unwrap();
            assert!(
                trunk.0 < index,
                "Tree Step must precede its newly created branch"
            );
            if let Some(next) = trunks
                .iter()
                .find(|(_, next)| next.pos.y == trunk.1.pos.y - 16.0)
            {
                assert!(
                    index < next.0,
                    "branch belongs between consecutive source Tree creations"
                );
            }
        }
        let last_trunk = trunks.last().unwrap().0;
        assert!(
            world
                .entities
                .iter()
                .enumerate()
                .filter(|(_, e)| e.object == "oLeaves")
                .all(|(index, _)| index > last_trunk)
        );
    }

    #[test]
    fn tree_creation_reuses_cell_metadata_without_losing_fields_or_identity() {
        let mut world = World::new(42, 34, Area::Jungle, 5);
        let reused = world.spawn("oTerrainMetadata", Vec2::new(168.0, 168.0));
        world
            .entity_mut(reused)
            .unwrap()
            .data
            .insert("author_marker".into(), 73.0);
        let prior = world.spawn("oRock", Vec2::new(32.0, 32.0));
        tree(
            &mut world,
            &mut Rng::new(42),
            10,
            10,
            Vec2::new(160.0, 160.0),
        );
        assert_eq!(world.entities[0].id, prior);
        assert_eq!(
            world.entities[1].id, reused,
            "reused metadata must move to the source Tree creation position"
        );
        let metadata = world.entity(reused).unwrap();
        assert_eq!(metadata.value("author_marker"), 73.0);
        assert_eq!(metadata.value("tree_initialized"), 1.0);
        assert_eq!(
            world
                .entities
                .iter()
                .filter(|e| e.object == "oTerrainMetadata" && e.pos == Vec2::new(168.0, 168.0))
                .count(),
            1
        );
    }

    #[test]
    fn kali_create_draws_before_following_prize_in_normal_area_alphabets() {
        for (area, name) in [
            (Area::Mines, "Mines"),
            (Area::Jungle, "Jungle"),
            (Area::Ice, "Ice Caves"),
            (Area::Temple, "Temple"),
        ] {
            // Each source scrRoomGen* creates the head synchronously for x;
            // its Create rand(1,3) precedes q's unused rand(1,6) and item roll.
            for seed in [1, 42, 123, 2026] {
                let mut world = World::new(42, 34, area, 2);
                world.name = name.into();
                let mut actual = Rng::new(seed);
                let mut source = actual.clone();
                let sprite = format!("sKaliHead{}", source.range(3) + 1);
                source.range(6);
                let prize = high_end(&mut source);
                stamp(
                    &mut world,
                    &mut actual,
                    &format!("xq{:0<78}", ""),
                    0,
                    0,
                    false,
                    false,
                    "Craps",
                );
                let head = world
                    .entities
                    .iter()
                    .find(|e| e.object == "oKaliHead")
                    .unwrap();
                assert_eq!(head.sprite, sprite, "{name}, seed {seed}");
                assert_eq!(head.value("kali_create_initialized"), 1.0);
                let head_id = head.id;
                let generated_prize = world
                    .entities
                    .iter()
                    .find(|e| e.value("inDiceHouse") != 0.0)
                    .unwrap();
                assert_eq!(generated_prize.object, prize, "{name}, seed {seed}");
                // The later dispatcher Create must retain this birth-time
                // choice and consume no additional generation RNG draw.
                crate::traps::initialize_kali_head(&mut world, &mut actual, head_id);
                assert_eq!(
                    actual.range(100000),
                    source.range(100000),
                    "{name}, seed {seed}"
                );
                assert_eq!(world.entity(head_id).unwrap().sprite, sprite);
            }
        }
    }

    #[test]
    fn dice_create_draws_before_following_prize_in_each_source_alphabet() {
        for (area, name, symbol) in [
            (Area::Mines, "Mines", 'd'),
            (Area::Ice, "Yeti Lair", 'd'),
            (Area::Jungle, "Jungle", 'u'),
            (Area::Ice, "Ice Caves", 'u'),
            (Area::Temple, "Temple", 'u'),
            (Area::Jungle, "Black Market", 'z'),
        ] {
            let mut world = World::new(42, 34, area, 2);
            world.name = name.into();
            let mut actual = Rng::new(123);
            let mut source = actual.clone();
            let values = [1 + source.range(6), 1 + source.range(6)];
            source.range(6); // q's source-local unused dice roll.
            let prize = high_end(&mut source);
            stamp(
                &mut world,
                &mut actual,
                &format!("{symbol}{symbol}q{:0<77}", ""),
                0,
                0,
                false,
                false,
                "Craps",
            );
            let dice: Vec<_> = world
                .entities
                .iter()
                .filter(|e| e.object == "oDice")
                .collect();
            assert_eq!(dice.len(), 2, "{name}");
            for (die, expected) in dice.into_iter().zip(values) {
                assert_eq!(die.value("value"), f64::from(expected), "{name}");
                assert_eq!(die.value("dice_create_initialized"), 1.0);
            }
            let generated_prize = world
                .entities
                .iter()
                .find(|e| e.value("inDiceHouse") != 0.0)
                .unwrap();
            assert_eq!(generated_prize.object, prize, "{name}");
            assert_eq!(actual.range(100000), source.range(100000), "{name}");
        }
    }

    #[test]
    fn origins_match_source_instead_of_unconditionally_centering_entities() {
        let mut w = World::new(42, 34, Area::Mines, 1);
        room(&mut w, "SaITM");
        for (object, p) in [
            ("oSnake", Vec2::new(16.0, 16.0)),
            ("oChest", Vec2::new(32.0, 16.0)),
            ("oGoldIdol", Vec2::new(64.0, 28.0)),
            ("oRubyBig", Vec2::new(72.0, 24.0)),
            ("oMattock", Vec2::new(88.0, 24.0)),
        ] {
            assert_eq!(
                w.entities.iter().find(|e| e.object == object).unwrap().pos,
                p,
                "{object}"
            );
        }
    }

    #[test]
    fn doors_have_centered_world_markers_and_original_instance_origins() {
        let mut w = World::new(42, 55, Area::Boss, 16);
        stamp_at(
            &mut w,
            &mut Rng::new(4),
            &format!("{:0<80}", "9"),
            1,
            37,
            true,
            false,
            "General",
        );
        assert_eq!(w.entrance, Vec2::new(24.0, 600.0));
        assert_eq!(
            w.entities
                .iter()
                .find(|e| e.object == "oEntrance")
                .unwrap()
                .pos,
            Vec2::new(16.0, 592.0)
        );
        assert_eq!(w.tile(1, 38).unwrap().object, "oTemple");
        assert_eq!(
            w.entities
                .iter()
                .find(|e| e.object == "oTerrainMetadata")
                .unwrap()
                .value("invincible"),
            1.0
        );
    }

    #[test]
    fn ice_and_yeti_alphabets_are_distinct() {
        let mut normal = World::new(42, 34, Area::Ice, 10);
        room(&mut normal, "YTiL");
        assert!(normal.entities.iter().any(|e| e.object == "oYeti"));
        assert!(normal.entities.iter().any(|e| e.object == "oJetpack"));
        assert_eq!(normal.tile(4, 1).unwrap().object, "oVine");
        let mut yeti = World::new(42, 34, Area::Ice, 10);
        yeti.name = "Yeti Lair".into();
        room(&mut yeti, "YTiL");
        assert!(yeti.entities.iter().any(|e| e.object == "oYetiKing"));
        assert!(yeti.entities.iter().any(|e| e.object == "oRubyBig"));
        assert_eq!(yeti.tile(4, 1).unwrap().object, "oLadderOrange");
    }

    #[test]
    fn market_ankh_and_swimming_chests_keep_source_offsets() {
        let mut w = World::new(42, 34, Area::Jungle, 6);
        w.name = "Black Market".into();
        room(&mut w, "AdGH");
        let ankh = w.entities.iter().find(|e| e.object == "oAnkh").unwrap();
        assert_eq!(ankh.pos, Vec2::new(24.0, 24.0));
        assert_eq!(ankh.value("cost"), 50000.0);
        assert_eq!(
            w.entities
                .iter()
                .find(|e| e.object == "oChest")
                .unwrap()
                .pos,
            Vec2::new(32.0, 16.0)
        );
        assert_eq!(w.tile(3, 1).unwrap().object, "oLadderOrange");
        assert_eq!(w.tile(4, 1).unwrap().object, "oLadderTop");
    }

    #[test]
    fn shop_inventory_prices_use_original_bases_and_level_scaling() {
        for style in ["Bomb", "Weapon", "Clothing", "Rare", "General"] {
            let mut w = World::new(42, 34, Area::Mines, 4);
            stamp(
                &mut w,
                &mut Rng::new(9),
                &format!("{:0<80}", "Kkiiii"),
                0,
                0,
                false,
                false,
                style,
            );
            assert_eq!(
                w.entities
                    .iter()
                    .find(|e| e.object == "oShopkeeper")
                    .unwrap()
                    .text
                    .get("style")
                    .unwrap(),
                style
            );
            assert_eq!(
                w.entities
                    .iter()
                    .find(|e| e.object == "oSign")
                    .unwrap()
                    .sprite,
                format!("sSign{style}")
            );
            let goods: Vec<_> = w
                .entities
                .iter()
                .filter(|e| e.value("forSale") == 1.0)
                .collect();
            assert_eq!(goods.len(), 4);
            assert!(
                goods
                    .iter()
                    .all(|e| e.value("cost") >= 1200.0 && e.pos.x % 16.0 == 8.0)
            );
        }
    }

    #[test]
    fn temple_specials_preserve_embedded_treasure_and_free_rescue() {
        let mut w = World::new(42, 34, Area::Temple, 14);
        room(&mut w, "R;DX");
        assert!(
            w.entities
                .iter()
                .any(|e| e.text.get("treasure").is_some_and(|t| t == "Big Ruby"))
        );
        let damsel = w.entities.iter().find(|e| e.object == "oDamsel").unwrap();
        assert_eq!(damsel.pos, Vec2::new(40.0, 24.0));
        assert_eq!(damsel.value("forSale"), 0.0);
        assert_eq!(
            w.entities
                .iter()
                .filter(|e| e.object == "oTempleFake")
                .count(),
            2
        );
        assert!(w.tiles.iter().any(|t| t.object == "oXocBlock"));
    }

    #[test]
    fn all_seven_source_decoders_preserve_ladder_and_platform_families() {
        for (area, name, ladder, top) in [
            (Area::Mines, "", "oLadderOrange", "oLadderTop"),
            (Area::Jungle, "", "oVine", "oVineTop"),
            (Area::Ice, "", "oVine", "oVine"),
            (Area::Temple, "", "oLadderOrange", "oLadderTop"),
            (Area::Boss, "", "oVine", "oVineTop"),
            (Area::Ice, "Yeti Lair", "oLadderOrange", "oLadderTop"),
            (Area::Jungle, "Black Market", "oVine", "oVineTop"),
        ] {
            let mut w = World::new(42, 55, area, 1);
            w.name = name.into();
            room(&mut w, "LP");
            assert_eq!(w.tile(1, 1).unwrap().object, ladder, "{area:?} {name}");
            assert_eq!(w.tile(2, 1).unwrap().object, top, "{area:?} {name}");
            assert!(!w.tile(1, 1).unwrap().is_solid());
            assert!(!w.tile(2, 1).unwrap().is_solid());
            assert!(w.tile(1, 1).unwrap().is_ladder());
            assert!(w.tile(2, 1).unwrap().is_ladder());
            assert_eq!(crate::catalog::is_a(top, "oPlatform"), top == "oLadderTop");
        }
    }

    #[test]
    fn source_collision_point_counts_dynamic_solids_but_ignores_platforms() {
        for (object, should_block) in [
            ("oPushBlock", true),
            ("oTrapBlock", true),
            ("oCeilingTrap", true),
            ("oTreeBranch", false),
            ("oLeaves", false),
            ("oTempleFake", false),
        ] {
            let mut w = World::new(42, 34, Area::Mines, 1);
            spawn(&mut w, &mut Rng::new(0), object, Vec2::new(16.0, 16.0));
            room(&mut w, "1");
            assert_eq!(
                w.tile(1, 1).unwrap().object.is_empty(),
                should_block,
                "{object}"
            );
        }
    }

    #[test]
    fn ladder_stamping_keeps_the_original_door_support_collision_layer() {
        let mut w = World::new(42, 34, Area::Mines, 1);
        let mut template = vec![b'0'; 80];
        template[0] = b'9';
        template[10] = b'L';
        room(&mut w, std::str::from_utf8(&template).unwrap());
        assert_eq!(w.tile(1, 2).unwrap().object, "oBrick");
        let ladder = w
            .entities
            .iter()
            .find(|e| e.object == "oLadderOrange")
            .unwrap();
        assert_eq!(ladder.pos, Vec2::new(16.0, 32.0));
        assert!(!ladder.solid);
    }

    #[test]
    fn moai_side_masks_span_four_rows_and_suppress_overlapping_terrain() {
        let mut w = World::new(42, 34, Area::Ice, 10);
        room(&mut w, "M");
        let left = w.entities.iter().find(|e| e.object == "oMoai").unwrap();
        assert_eq!(
            left.bounds(),
            crate::model::Rect::new(16.0, 16.0, 16.0, 64.0)
        );
        assert!(left.solid);
        assert_eq!(left.value("invincible"), 1.0);
        for row in 1..=4 {
            stamp_at(
                &mut w,
                &mut Rng::new(0),
                "1",
                1,
                row,
                false,
                false,
                "General",
            );
            assert!(w.tile(1, row as i32).unwrap().object.is_empty());
        }
        assert_eq!(
            w.entities
                .iter()
                .filter(|e| matches!(
                    e.object.as_str(),
                    "oMoai" | "oMoai2" | "oMoai3" | "oMoaiInside"
                ))
                .count(),
            4
        );
    }

    #[test]
    fn generated_tree_branch_is_jump_through_and_supports_falling_player() {
        use crate::model::{Game, InputFrame, PlayerState, RulesPreset};
        let mut game = Game::shell(0, RulesPreset::Classic);
        game.world = World::new(20, 20, Area::Jungle, 5);
        let id = spawn(
            &mut game.world,
            &mut game.rng,
            "oTreeBranch",
            Vec2::new(80.0, 128.0),
        );
        assert!(!game.world.entity(id).unwrap().solid);
        game.player.pos = Vec2::new(88.0, 148.0);
        game.player.state = PlayerState::Jumping;
        game.player.vel.y = -6.0;
        game.tick = 1;
        crate::player::tick(&mut game, &InputFrame::default());
        assert!(
            game.player.pos.y < 148.0,
            "a branch must not block upward movement"
        );
        assert!(game.player.vel.y < 0.0);
        game.player.pos = Vec2::new(88.0, 118.0);
        game.player.state = PlayerState::Falling;
        game.player.vel.y = 3.0;
        game.tick += 1;
        crate::player::tick(&mut game, &InputFrame::default());
        assert_eq!(
            game.player.pos.y, 120.0,
            "falling must land on the platform top"
        );
    }

    #[test]
    fn falling_ice_and_spike_masks_use_source_origin_and_collision_bounds() {
        let mut w = World::new(42, 34, Area::Ice, 9);
        room(&mut w, "f7");
        let falling = w.entities.iter().find(|e| e.object == "oDarkFall").unwrap();
        assert_eq!(
            falling.bounds(),
            crate::model::Rect::new(16.0, 16.0, 16.0, 8.0)
        );
        // collision_point uses the sprite's inclusive mask, independent of the
        // eight-pixel custom rectangle used to move the falling block.
        assert!(!falling.bounds().contains(Vec2::new(16.0, 24.0)));
        assert!(solid_point(&w, Vec2::new(16.0, 24.0)));
        let spikes = w.entities.iter().find(|e| e.object == "oSpikes").unwrap();
        assert_eq!(
            spikes.bounds(),
            crate::model::Rect::new(32.0, 19.0, 16.0, 13.0)
        );
        assert!(!spikes.solid);
    }
}
