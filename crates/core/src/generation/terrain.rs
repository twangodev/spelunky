//! Original terrain Create-event treasure rolls and scrSetupWalls silhouettes.
//!
//! Tile instances are represented by invisible, centered metadata entities.
//! Their contents become available when the containing terrain is destroyed.
use crate::model::{Entity, Rng, Vec2, World};

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

fn metadata(world: &mut World, x: usize, y: usize) -> u64 {
    let p = Vec2::new(x as f64 * 16.0 + 8.0, y as f64 * 16.0 + 8.0);
    if let Some(id) = world
        .entities
        .iter()
        .find(|e| {
            e.alive
                && e.object == "oTerrainMetadata"
                && (e.pos.x / 16.0).floor() as usize == x
                && (e.pos.y / 16.0).floor() as usize == y
        })
        .map(|entity| entity.id)
    {
        let entity = world.entity_mut(id).unwrap();
        entity.pos = p;
        entity.previous_pos = p;
        return id;
    }
    let id = world.spawn("oTerrainMetadata", p);
    world.entity_mut(id).unwrap().sprite.clear();
    id
}

fn neighbors(world: &World, x: usize, y: usize, object: &str) -> (bool, bool, bool, bool) {
    let connected = |dx: i32, dy: i32| {
        let point = Vec2::new(
            (x as f64 + f64::from(dx)) * 16.0,
            (y as f64 + f64::from(dy)) * 16.0,
        );
        world.tile(x as i32 + dx, y as i32 + dy).is_some_and(|t| {
            t.object == object
                || (object == "oBrick" && t.object == "oHardBlock")
                || (object == "oTemple" && t.object == "oTempleFake")
        }) || (object == "oTemple"
            && world.entities.iter().any(|entity| {
                entity.alive
                    && entity.object == "oTempleFake"
                    && entity.sprite_bounds().contains(point)
            }))
    };
    let boundary = object != "oIce";
    let bottom = if world.area == crate::model::Area::Boss {
        864
    } else if world.name == "Lake" {
        656
    } else {
        528
    };
    let mut up = (boundary && y == 0) || connected(0, -1);
    if object == "oTemple" {
        // The original oTemple up check deliberately queries Fake *below*.
        up = (boundary && y == 0)
            || world
                .tile(x as i32, y as i32 - 1)
                .is_some_and(|t| t.object == "oTemple")
            || world
                .tile(x as i32, y as i32 + 1)
                .is_some_and(|t| t.object == "oTempleFake")
            || world.entities.iter().any(|entity| {
                entity.alive
                    && entity.object == "oTempleFake"
                    && entity
                        .sprite_bounds()
                        .contains(Vec2::new(x as f64 * 16.0, (y + 1) as f64 * 16.0))
            });
    }
    (
        up,
        (boundary && y * 16 >= bottom) || connected(0, 1),
        connected(-1, 0),
        connected(1, 0),
    )
}

fn edge_sprite(object: &str, city: bool, n: (bool, bool, bool, bool)) -> Option<String> {
    let (up, down, left, right) = n;
    let name = match object {
        "oBrick" => match (up, down) {
            (false, false) => "sCaveUp2",
            (false, true) => "sCaveUp",
            (true, false) => "sBrickDown",
            _ => return None,
        },
        "oLush" => match (up, down) {
            (false, false) => "sLushUp2",
            (false, true) => "sLushUp",
            (true, false) => "sLushDown",
            _ => return None,
        },
        "oDark" => match (up, down) {
            (false, false) => "sDarkUp2",
            (false, true) => "sDarkUp",
            (true, false) => "sDarkDown",
            _ => return None,
        },
        "oTemple" => {
            let suffix = if !up {
                match (left, right, down) {
                    (false, false, false) => "Up6",
                    (false, false, true) => "Up5",
                    (false, true, false) => "Up7",
                    (false, true, true) => "Up3",
                    (true, false, false) => "Up8",
                    (true, false, true) => "Up4",
                    (true, true, false) => "Up2",
                    (true, true, true) => "Up",
                }
            } else if !down {
                "Down"
            } else {
                return None;
            };
            return Some(format!(
                "{}{}",
                if city { "sGTemple" } else { "sTemple" },
                suffix
            ));
        }
        "oIce" => match (up, down, left, right) {
            (false, false, false, false) => "sIceBlock",
            (false, true, false, false) => "sIceULR",
            (true, false, false, false) => "sIceDLR",
            (true, true, false, false) => "sIceLR",
            (false, false, _, false) => "sIceUDR",
            (false, true, _, false) => "sIceUR",
            (true, false, _, false) => "sIceDR",
            (true, true, _, false) => "sIceRight",
            (false, false, false, true) => "sIceUDL",
            (false, true, false, true) => "sIceUL",
            (true, false, false, true) => "sIceDL",
            (true, true, false, true) => "sIceLeft",
            (false, false, true, true) => "sIceUp2",
            (false, true, true, true) => "sIceUp",
            (true, false, true, true) => "sIceDown",
            _ => return None,
        },
        _ => return None,
    };
    Some(name.into())
}

/// Creation order is stable within this Rust port; it does not reproduce the
/// GameMaker interleaving of room creation and individual Create events.
#[cfg(test)]
pub(super) fn initialize(world: &mut World, rng: &mut Rng) {
    initialize_with_graphics(world, rng, true);
}

pub(super) fn initialize_custom(world: &mut World, rng: &mut Rng) {
    initialize_terrain(world, rng, true, false);
}

pub(super) fn initialize_with_graphics(world: &mut World, rng: &mut Rng, graphics_high: bool) {
    initialize_terrain(world, rng, false, graphics_high);
}

fn initialize_terrain(world: &mut World, rng: &mut Rng, custom: bool, graphics_high: bool) {
    let city = world.name == "City of Gold";
    for y in 0..world.height {
        for x in 0..world.width {
            let object = world.tiles[y * world.width + x].object.clone();
            if !matches!(
                object.as_str(),
                "oBrick" | "oLush" | "oDark" | "oTemple" | "oIce"
            ) {
                continue;
            }
            let id = metadata(world, x, y);
            if world.entity(id).unwrap().value("terrain_initialized") != 0.0 {
                continue;
            }
            let explicit_sprite = world
                .entity(id)
                .unwrap()
                .text
                .get("terrain_sprite")
                .cloned();
            let source_boundary = world.entity(id).unwrap().value("source_boundary") != 0.0;
            let mut sprite = if object == "oBrick" && !source_boundary && rng.chance(1, 10) {
                "sBrick2".into()
            } else {
                format!("s{}", &object[1..])
            };
            let mut gold = 0;
            let mut treasure = None;
            let mut frozen = false;
            if source_boundary {
                // scrInitLevel explicitly replaces boundary Create-event
                // variants with the area's base sprite. Preserve the port's
                // established interior RNG stream while applying SetupWalls
                // silhouettes below; exact GameMaker RNG remains unaudited.
                if object == "oTemple" && city {
                    sprite = "sGTemple".into();
                }
            } else if object == "oIce" {
                if rng.chance(1, 80) {
                    // FrozenCaveman Destroy releases this live caveman.
                    if custom {
                        release_custom_frozen(world, Vec2::new(x as f64 * 16.0, y as f64 * 16.0));
                    } else {
                        treasure = Some(("oCaveman", -8.0, -8.0));
                        frozen = true;
                    }
                }
            } else {
                let roll = rng.range(100) + 1;
                if object == "oTemple" && city {
                    sprite = "sGTemple".into();
                    gold = 2;
                } else if roll < 20 {
                    sprite = format!("s{}Gold", &object[1..]);
                    gold = 1;
                } else if roll < 30 {
                    sprite = format!("s{}GoldBig", &object[1..]);
                    gold = 2;
                } else if x > 0 && y > 0 && x + 1 < world.width && y + 1 < world.height {
                    let chances = match object.as_str() {
                        "oBrick" => [100, 120, 140],
                        "oLush" => [80, 100, 120],
                        "oDark" => [40, 60, 80],
                        _ => [60, 80, 100],
                    };
                    for (name, chance) in ["oSapphireBig", "oEmeraldBig", "oRubyBig"]
                        .into_iter()
                        .zip(chances)
                    {
                        if rng.chance(1, chance) {
                            treasure = Some((name, 0.0, 0.0));
                            break;
                        }
                    }
                    if treasure.is_none() && rng.chance(1, 1200) {
                        treasure = Some(UNDERGROUND[rng.range(19) as usize]);
                    }
                }
            }
            let sides = neighbors(world, x, y, &object);
            // SetupWalls overwrites gold sprites; their Destroy event then
            // releases no gold. City of Gold uses the global flag instead.
            if !custom && let Some(edge) = edge_sprite(&object, city, sides) {
                sprite = edge;
                if !(object == "oTemple" && city) {
                    gold = 0;
                }
            }
            if let Some(explicit) = explicit_sprite {
                sprite = explicit;
                if !(object == "oTemple" && city) {
                    gold = 0;
                }
            }
            if custom {
                sprite = format!("s{}", &object[1..]);
                gold = 0;
            }
            let e = world.entity_mut(id).unwrap();
            e.sprite.clear();
            e.data.insert("terrain_initialized".into(), 1.0);
            e.text.insert("terrain_sprite".into(), sprite);
            if gold > 0 {
                e.data.insert("gold_chunks".into(), 3.0);
                e.data
                    .insert("gold_nuggets".into(), if gold == 2 { 1.0 } else { 0.0 });
            }
            if frozen {
                e.data.insert("frozen_caveman".into(), 1.0);
            }
            // Explicit temple Big Ruby assignments from room templates win.
            if !e.text.contains_key("treasure") {
                if let Some((name, dx, dy)) = treasure {
                    e.data.insert("embedded_treasure".into(), 1.0);
                    e.data.insert("treasure_dx".into(), dx);
                    e.data.insert("treasure_dy".into(), dy);
                    e.text.insert("treasure".into(), name.into());
                }
            }
            if !custom && graphics_high {
                setup_fringe(
                    world,
                    rng,
                    &object,
                    Vec2::new(x as f64 * 16.0, y as f64 * 16.0),
                    sides,
                );
            }
            if !custom && object == "oIce" && !sides.1 && rng.chance(1, 20) {
                let below = Vec2::new(x as f64 * 16.0, (y + 1) as f64 * 16.0);
                if !world.solid_point_except(below, 0) {
                    let id = world.spawn("oIceBottom", below);
                    world
                        .entity_mut(id)
                        .unwrap()
                        .data
                        .insert("drip_alarm".into(), f64::from(20 + rng.range(381)));
                }
            }
        }
    }
    if !custom {
        setup_fake_temple(world, rng, city, graphics_high);
        super::water::setup(world, rng);
    }
    for y in 0..world.height {
        for x in 0..world.width {
            if world.tile(x as i32, y as i32).unwrap().object != "oLava" {
                continue;
            }
            let origin = Vec2::new(x as f64 * 16.0, y as f64 * 16.0);
            let exposed = !custom && lava_exposed(world, origin);
            let id = metadata(world, x, y);
            if custom {
                create_lava(world.entity_mut(id).unwrap(), rng);
            } else {
                setup_lava(world.entity_mut(id).unwrap(), rng, exposed, true);
            }
        }
    }
    let lava: Vec<_> = world
        .entities
        .iter()
        .filter(|entity| entity.alive && entity.object == "oLava")
        .map(|entity| entity.id)
        .collect();
    for id in lava {
        let origin = world.entity(id).unwrap().pos;
        if custom {
            create_lava(world.entity_mut(id).unwrap(), rng);
        } else {
            let exposed = lava_exposed(world, origin);
            setup_lava(world.entity_mut(id).unwrap(), rng, exposed, false);
        }
    }
    if !custom {
        super::water::propagate_surface(world);
    } else {
        initialize_custom_instances(world, rng);
    }
}

/// Source tile_add decorations use background pixel rectangles, not sprites.
pub(super) fn decoration(
    world: &mut World,
    background: &str,
    origin: Vec2,
    source_x: f64,
    size: f64,
    depth: f64,
) {
    let id = world.spawn("oGenerationDecoration", origin);
    let entity = world.entity_mut(id).unwrap();
    entity.sprite = background.into();
    for (key, value) in [
        ("decoration", 1.0),
        ("source_x", source_x),
        ("source_y", 0.0),
        ("source_w", size),
        ("source_h", size),
        ("depth", depth),
    ] {
        entity.data.insert(key.into(), value);
    }
}

fn setup_fringe(
    world: &mut World,
    rng: &mut Rng,
    object: &str,
    origin: Vec2,
    sides: (bool, bool, bool, bool),
) {
    let (up, down, _, _) = sides;
    if !up {
        let choice = match object {
            "oBrick" => Some(("bgCaveTop", if rng.range(3) < 2 { 0.0 } else { 16.0 })),
            "oLush" => Some((
                "bgCaveTop2",
                if rng.chance(1, 8) {
                    32.0
                } else if rng.range(3) < 2 {
                    0.0
                } else {
                    16.0
                },
            )),
            "oDark" => Some(("bgCaveTop3", if rng.range(3) < 2 { 0.0 } else { 16.0 })),
            "oTemple" => {
                if rng.chance(1, 4) {
                    Some(("bgCaveTop4", 0.0))
                } else if rng.chance(1, 4) {
                    Some(("bgCaveTop4", 16.0))
                } else {
                    None
                }
            }
            _ => None,
        };
        if let Some((background, source_x)) = choice {
            decoration(
                world,
                background,
                Vec2::new(origin.x, origin.y - 16.0),
                source_x,
                16.0,
                3.0,
            );
        }
    }
    if object == "oLush"
        && !down
        && !world.solid_point_except(Vec2::new(origin.x, origin.y + 16.0), 0)
    {
        let source_x = if rng.chance(1, 12) {
            Some(48.0)
        } else if rng.chance(1, 12) {
            Some(64.0)
        } else {
            None
        };
        if let Some(source_x) = source_x {
            decoration(
                world,
                "bgCaveTop2",
                Vec2::new(origin.x, origin.y + 16.0),
                source_x,
                16.0,
                3.0,
            );
        }
    }
}

fn setup_fake_temple(world: &mut World, rng: &mut Rng, city: bool, graphics_high: bool) {
    let ids: Vec<_> = world
        .entities
        .iter()
        .filter(|entity| {
            entity.alive
                && entity.object == "oTempleFake"
                && entity.value("fake_setup_initialized") == 0.0
        })
        .map(|entity| entity.id)
        .collect();
    for id in ids {
        let origin = world.entity(id).unwrap().pos;
        let connected = |point: Vec2| {
            world.tiles_point(point, "oTemple")
                || world.tiles_point(point, "oTempleFake")
                || world.entities.iter().any(|entity| {
                    entity.alive
                        && matches!(entity.object.as_str(), "oTemple" | "oTempleFake")
                        && entity.sprite_bounds().contains(point)
                })
        };
        let up = origin.y == 0.0 || connected(Vec2::new(origin.x, origin.y - 16.0));
        let down = connected(Vec2::new(origin.x, origin.y + 16.0));
        let left = connected(Vec2::new(origin.x - 16.0, origin.y));
        let right = connected(Vec2::new(origin.x + 16.0, origin.y));
        if city && graphics_high {
            setup_fringe(world, rng, "oTemple", origin, (up, down, left, right));
        }
        let sprite = edge_sprite("oTemple", city, (up, down, left, right));
        let entity = world.entity_mut(id).unwrap();
        if let Some(sprite) = sprite {
            entity.sprite = sprite;
        }
        entity.data.insert("fake_setup_initialized".into(), 1.0);
    }
}

fn release_custom_frozen(world: &mut World, origin: Vec2) {
    // scrCreateTileObj destroys its freshly created FrozenCaveman. In
    // rLoadLevel cleanSolids is false, so that Destroy event releases a live,
    // stunned caveman; there is no frozen overlay left in the custom room.
    let id = world.spawn("oCaveman", origin);
    let caveman = world.entity_mut(id).unwrap();
    caveman.data.insert("invincible".into(), 20.0);
    caveman.state = 98;
    caveman.timer = caveman.value("stun_time") as u32;
}

fn initialize_custom_instances(world: &mut World, rng: &mut Rng) {
    // Custom compound symbols can overlap static terrain with pools or room
    // borders. Those additional terrain instances live outside tile storage
    // but still execute the same source Create events.
    let ids: Vec<_> = world
        .entities
        .iter()
        .filter(|entity| {
            entity.alive
                && matches!(
                    entity.object.as_str(),
                    "oBrick" | "oLush" | "oDark" | "oTemple" | "oIce"
                )
                && entity.value("terrain_initialized") == 0.0
        })
        .map(|entity| entity.id)
        .collect();
    for id in ids {
        let entity = world.entity(id).unwrap();
        let object = entity.object.clone();
        let origin = entity.pos;
        let mut treasure = None;
        if object == "oIce" {
            if rng.chance(1, 80) {
                release_custom_frozen(world, origin);
            }
        } else {
            if object == "oBrick" {
                rng.range(10);
            }
            let roll = rng.range(100) + 1;
            if roll >= 30
                && origin.x > 1.0
                && origin.x < (world.width - 1) as f64 * 16.0
                && origin.y > 1.0
                && origin.y < (world.height - 1) as f64 * 16.0
            {
                let chances = match object.as_str() {
                    "oBrick" => [100, 120, 140],
                    "oLush" => [80, 100, 120],
                    "oDark" => [40, 60, 80],
                    _ => [60, 80, 100],
                };
                for (name, chance) in ["oSapphireBig", "oEmeraldBig", "oRubyBig"]
                    .into_iter()
                    .zip(chances)
                {
                    if rng.chance(1, chance) {
                        treasure = Some((name, 0.0, 0.0));
                        break;
                    }
                }
                if treasure.is_none() && rng.chance(1, 1200) {
                    treasure = Some(UNDERGROUND[rng.range(19) as usize]);
                }
            }
        }
        if let Some((object, dx, dy)) = treasure {
            world.spawn(object, Vec2::new(origin.x + 8.0 + dx, origin.y + 8.0 + dy));
        }
        let entity = world.entity_mut(id).unwrap();
        entity.data.insert("terrain_initialized".into(), 1.0);
        entity.sprite = format!("s{}", &object[1..]);
    }
}

fn lava_exposed(world: &World, origin: Vec2) -> bool {
    // scrSetupWalls probes the original instance origin, not its tile center.
    // oLava itself inherits oWater, so this also suppresses buried lava layers.
    let above = Vec2::new(origin.x, origin.y - 16.0);
    !world.solid_point_except(above, 0)
        && !world.tiles_point(above, "oWater")
        && !world.entities.iter().any(|entity| {
            entity.alive
                && crate::catalog::is_a(&entity.object, "oWater")
                && entity.sprite_bounds().contains(above)
        })
}

fn setup_lava(entity: &mut Entity, rng: &mut Rng, exposed: bool, metadata: bool) {
    if entity.value("lava_setup_initialized") != 0.0 {
        return;
    }
    let explicit_spurt = entity.data.contains_key("spurt");
    create_lava(entity, rng);
    if exposed && !explicit_spurt && rng.chance(1, 4) {
        entity.data.insert("spurt".into(), 1.0);
    }
    let sprite = if exposed { "sLavaTop" } else { "sLava" };
    if metadata {
        entity.text.insert("terrain_sprite".into(), sprite.into());
    } else {
        entity.sprite = sprite.into();
    }
    entity.data.insert("lava_setup_initialized".into(), 1.0);
}

fn create_lava(entity: &mut Entity, rng: &mut Rng) {
    if entity.value("lava_initialized") == 0.0 {
        let time = entity
            .data
            .get("spurt_time")
            .copied()
            .unwrap_or_else(|| 100.0 + f64::from(rng.range(201)));
        entity.data.entry("spurt_time".into()).or_insert(time);
        entity.data.entry("spurt_counter".into()).or_insert(time);
        entity.data.entry("spurt".into()).or_insert(0.0);
        entity.data.insert("lava_initialized".into(), 1.0);
        entity.animation = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Area;

    #[test]
    fn high_graphics_fringe_choices_follow_source_short_circuit_draw_order() {
        for seed in 0..64 {
            for object in ["oBrick", "oLush", "oDark", "oTemple"] {
                let mut expected = Rng::new(seed);
                let mut choices = Vec::new();
                let upper = match object {
                    "oBrick" | "oDark" => Some(if expected.range(3) < 2 { 0.0 } else { 16.0 }),
                    "oLush" => Some(if expected.chance(1, 8) {
                        32.0
                    } else if expected.range(3) < 2 {
                        0.0
                    } else {
                        16.0
                    }),
                    _ => {
                        if expected.chance(1, 4) {
                            Some(0.0)
                        } else if expected.chance(1, 4) {
                            Some(16.0)
                        } else {
                            None
                        }
                    }
                };
                if let Some(x) = upper {
                    choices.push((x, 16.0));
                }
                if object == "oLush" {
                    let lower = if expected.chance(1, 12) {
                        Some(48.0)
                    } else if expected.chance(1, 12) {
                        Some(64.0)
                    } else {
                        None
                    };
                    if let Some(x) = lower {
                        choices.push((x, 48.0));
                    }
                }
                let mut world = World::new(4, 4, Area::Mines, 1);
                let mut actual = Rng::new(seed);
                setup_fringe(
                    &mut world,
                    &mut actual,
                    object,
                    Vec2::new(32.0, 32.0),
                    (false, false, false, false),
                );
                let emitted: Vec<_> = world
                    .entities
                    .iter()
                    .map(|e| (e.value("source_x"), e.pos.y))
                    .collect();
                assert_eq!(emitted, choices, "{object} seed {seed}");
                assert_eq!(
                    actual.range(100000),
                    expected.range(100000),
                    "{object} seed {seed}"
                );
            }
        }
    }

    #[test]
    fn fake_temple_keeps_source_gold_edge_and_covered_base_sprite_rules() {
        for city in [false, true] {
            for covered in [false, true] {
                let mut world = World::new(5, 5, Area::Temple, 13);
                let id = world.spawn("oTempleFake", Vec2::new(32.0, 32.0));
                if covered {
                    world.set_tile(2, 1, "oTemple");
                    world.set_tile(2, 3, "oTemple");
                }
                let mut actual = Rng::new(42);
                let mut expected = actual.clone();
                setup_fake_temple(&mut world, &mut actual, city, false);
                assert_eq!(
                    actual.range(100000),
                    expected.range(100000),
                    "low graphics must consume no fringe RNG"
                );
                let entity = world.entity(id).unwrap();
                assert_eq!(
                    entity.sprite,
                    if covered {
                        "sTemple"
                    } else if city {
                        "sGTempleUp6"
                    } else {
                        "sTempleUp6"
                    }
                );
                assert!(!entity.solid);
            }
        }
        // Normal fake temple never adds fringes, even at high graphics.
        let mut world = World::new(5, 5, Area::Temple, 13);
        world.spawn("oTempleFake", Vec2::new(32.0, 32.0));
        let mut actual = Rng::new(42);
        let mut expected = actual.clone();
        setup_fake_temple(&mut world, &mut actual, false, true);
        assert_eq!(actual.range(100000), expected.range(100000));
        assert!(!world.entities.iter().any(|e| e.sprite == "bgCaveTop4"));
    }

    #[test]
    fn lava_setup_uses_source_origins_water_ancestry_and_dynamic_solid_masks() {
        for blocker in ["oBrick", "oWaterSwim", "oPushBlock", "oLava"] {
            let mut world = World::new(4, 4, Area::Temple, 13);
            let lava = world.spawn("oLava", Vec2::new(16.0, 32.0));
            if matches!(blocker, "oBrick" | "oWaterSwim") {
                world.set_tile(1, 1, blocker);
            } else {
                world.spawn(blocker, Vec2::new(16.0, 16.0));
            }
            initialize(&mut world, &mut Rng::new(42));
            let lava = world.entity(lava).unwrap();
            assert_eq!(lava.sprite, "sLava", "blocker {blocker}");
            assert_eq!(lava.value("spurt"), 0.0);
            assert!((100.0..=300.0).contains(&lava.value("spurt_time")));
            assert_eq!(lava.value("spurt_counter"), lava.value("spurt_time"));
        }
    }

    #[test]
    fn exposed_lava_uses_source_spurt_roll_and_preserves_explicit_state() {
        let mut spurts = 0;
        for seed in 0..64 {
            let mut world = World::new(4, 4, Area::Temple, 13);
            let id = world.spawn("oLava", Vec2::new(16.0, 32.0));
            let mut expected = Rng::new(seed);
            let time = 100.0 + f64::from(expected.range(201));
            let spurt = expected.chance(1, 4);
            initialize(&mut world, &mut Rng::new(seed));
            let lava = world.entity(id).unwrap();
            assert_eq!(lava.sprite, "sLavaTop");
            assert_eq!(lava.value("spurt_time"), time);
            assert_eq!(lava.value("spurt"), f64::from(spurt));
            spurts += usize::from(spurt);
            let previous = serde_json::to_string(&world).unwrap();
            initialize(&mut world, &mut Rng::new(999));
            assert_eq!(serde_json::to_string(&world).unwrap(), previous);
        }
        assert!(spurts > 0 && spurts < 64);
        let mut world = World::new(4, 4, Area::Temple, 13);
        world.set_tile(1, 2, "oLava");
        let id = metadata(&mut world, 1, 2);
        let lava = world.entity_mut(id).unwrap();
        lava.data.insert("spurt".into(), 1.0);
        lava.data.insert("spurt_time".into(), 123.0);
        lava.data.insert("spurt_counter".into(), 7.0);
        initialize(&mut world, &mut Rng::new(42));
        let lava = world.entity(id).unwrap();
        assert_eq!(lava.text["terrain_sprite"], "sLavaTop");
        assert_eq!(lava.value("spurt"), 1.0);
        assert_eq!(lava.value("spurt_time"), 123.0);
        assert_eq!(lava.value("spurt_counter"), 7.0);
    }

    #[test]
    fn generated_olmec_lava_only_enables_exposed_surface_spurts() {
        let world = crate::generation::generate(&mut Rng::new(42), 16, &Default::default());
        let lava: Vec<_> = world
            .entities
            .iter()
            .filter(|entity| entity.object == "oLava")
            .collect();
        assert_eq!(lava.len(), 240);
        assert!(
            lava.iter()
                .filter(|entity| entity.pos.y == 768.0)
                .any(|entity| entity.value("spurt") == 1.0)
        );
        for entity in lava {
            assert_eq!(entity.value("lava_initialized"), 1.0);
            assert_eq!(
                entity.sprite,
                if entity.pos.y == 768.0 {
                    "sLavaTop"
                } else {
                    "sLava"
                }
            );
            if entity.pos.y > 768.0 {
                assert_eq!(entity.value("spurt"), 0.0);
            }
        }
    }

    #[test]
    fn ice_and_temple_silhouettes_match_source_precedence() {
        assert_eq!(
            edge_sprite("oIce", false, (false, false, false, false)).as_deref(),
            Some("sIceBlock")
        );
        assert_eq!(
            edge_sprite("oIce", false, (false, false, true, false)).as_deref(),
            Some("sIceUDR")
        );
        assert_eq!(
            edge_sprite("oTemple", true, (false, false, false, true)).as_deref(),
            Some("sGTempleUp7")
        );
        assert_eq!(
            edge_sprite("oBrick", false, (true, true, false, false)),
            None
        );
    }

    #[test]
    fn city_edges_keep_gold_and_existing_treasure_metadata() {
        let mut w = World::new(3, 3, Area::Temple, 15);
        w.name = "City of Gold".into();
        w.set_tile(1, 1, "oTemple");
        let id = metadata(&mut w, 1, 1);
        w.entity_mut(id)
            .unwrap()
            .text
            .insert("treasure".into(), "oRubyBig".into());
        initialize(&mut w, &mut Rng::new(0));
        let e = w.entity(id).unwrap();
        assert_eq!(e.value("gold_chunks"), 3.0);
        assert_eq!(e.value("gold_nuggets"), 1.0);
        assert_eq!(e.text["treasure"], "oRubyBig");
        assert_eq!(e.text["terrain_sprite"], "sGTempleUp6");
        assert_eq!(
            w.entities
                .iter()
                .filter(|e| e.object == "oTerrainMetadata")
                .count(),
            1
        );
        let previous = serde_json::to_string(&w).unwrap();
        initialize(&mut w, &mut Rng::new(100));
        assert_eq!(serde_json::to_string(&w).unwrap(), previous);
    }

    #[test]
    fn covered_bricks_roll_both_gold_classes_but_edges_do_not() {
        let mut classes = std::collections::BTreeSet::new();
        for seed in 0..128 {
            let mut w = World::new(3, 3, Area::Mines, 1);
            for tile in &mut w.tiles {
                tile.object = "oBrick".into();
            }
            initialize(&mut w, &mut Rng::new(seed));
            let middle = w
                .entities
                .iter()
                .find(|e| e.pos == Vec2::new(24.0, 24.0))
                .unwrap();
            if middle.value("gold_chunks") == 3.0 {
                classes.insert(middle.value("gold_nuggets") as u8);
            }
        }
        assert_eq!(classes, [0, 1].into_iter().collect());
        let mut w = World::new(3, 3, Area::Mines, 1);
        w.set_tile(1, 1, "oBrick");
        initialize(&mut w, &mut Rng::new(4));
        assert_eq!(w.entities[0].value("gold_chunks"), 0.0);
    }
}
