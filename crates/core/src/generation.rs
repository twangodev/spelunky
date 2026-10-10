//! Classic room-path construction and the original 10×8 room/obstacle templates.
//! The checked-in template archive is extracted from the licensed GML source.
use crate::model::{Area, Rng, Vec2, World};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

#[path = "generation/population.rs"]
mod population;
#[path = "generation/special.rs"]
mod special;
#[path = "generation/terrain.rs"]
mod terrain;
#[path = "generation/tiles.rs"]
mod tiles;
#[cfg(test)]
#[path = "generation/traversal.rs"]
mod traversal;
#[path = "generation/water.rs"]
mod water;

#[derive(Deserialize)]
struct Archive {
    scripts: BTreeMap<String, Script>,
}
#[derive(Deserialize)]
struct Script {
    records: Vec<Record>,
}
#[derive(Deserialize)]
struct Record {
    variable: String,
    value: String,
    kind: String,
    #[serde(default)]
    case: Option<u32>,
    #[serde(default)]
    context: Vec<String>,
}
fn archive() -> &'static Archive {
    static DATA: OnceLock<Archive> = OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!("../../../assets/generation.json"))
            .expect("validated Classic template archive")
    })
}
fn rand(rng: &mut Rng, low: u32, high: u32) -> u32 {
    low + rng.range(high - low + 1)
}
fn has(flags: &BTreeSet<String>, name: &str) -> bool {
    flags.contains(name)
        || match name {
            "black_market" => flags.contains("blackMarket") || flags.contains("genBlackMarket"),
            "city_of_gold" => flags.contains("cityOfGold"),
            "snake_pit" => flags.contains("snakePit"),
            "alien_ship" => flags.contains("alienCraft"),
            "yeti_lair" => flags.contains("yetiLair"),
            "cemetery" => flags.contains("cemetary"),
            "sacrifice_pit" => flags.contains("sacrificePit"),
            "made_moai" => flags.contains("madeMoai"),
            "dark" => flags.contains("darkLevel"),
            _ => false,
        }
}

#[derive(Debug)]
struct Path {
    rooms: [[u8; 4]; 5],
    start: usize,
    end: usize,
}
fn room_path(rng: &mut Rng, area: Area, level: u32, flags: &mut BTreeSet<String>) -> Path {
    let start = rng.range(4) as usize;
    let mut path = Path {
        rooms: [[0; 4]; 5],
        start,
        end: start,
    };
    if has(flags, "black_market") && area == Area::Jungle {
        path.start = 0;
        path.end = 3;
        path.rooms = [
            [2, 4, 4, 2],
            [2, 4, 4, 2],
            [2, 4, 5, 4],
            [3, 1, 1, 3],
            [0; 4],
        ];
        flags.insert("black_market".into());
        return path;
    }
    let mut x = start;
    let mut y = 0;
    path.rooms[y][x] = 1;
    // The source creates a sacrifice shaft before constructing the main path.
    if area == Area::Temple && (has(flags, "sacrifice_pit") || rng.chance(1, 8)) {
        let mut pit = rng.range(4) as usize;
        while pit == start {
            pit = rng.range(4) as usize;
        }
        for row in 0..4 {
            path.rooms[row][pit] = if row == 0 {
                7
            } else if row == 3 {
                9
            } else {
                8
            };
        }
        flags.insert("sacrifice_pit".into());
    }
    while y < 4 {
        let n = if x == 0 {
            rand(rng, 3, 5)
        } else if x == 3 {
            rand(rng, 5, 7)
        } else {
            rand(rng, 1, 5)
        };
        if n == 5 {
            if y < 3 {
                path.rooms[y][x] = 2;
                path.rooms[y + 1][x] = 3;
            } else {
                path.end = x;
            }
            y += 1;
        } else {
            let next = if n < 3 || n > 5 {
                x.saturating_sub(1)
            } else {
                (x + 1).min(3)
            };
            if path.rooms[y][next] == 0 {
                x = next;
            }
            path.rooms[y][x] = 1;
        }
    }
    if has(flags, "city_of_gold") {
        path.rooms[2][rng.range(4) as usize] = 6;
    }
    if area == Area::Mines {
        'pit: for row in 0..2 {
            for col in 0..4 {
                if (row..row + 3).all(|r| path.rooms[r][col] == 0)
                    && (has(flags, "snake_pit") || rng.chance(1, 8))
                {
                    path.rooms[row][col] = 7;
                    path.rooms[row + 1][col] = 8;
                    if row == 0 && path.rooms[3][col] == 0 {
                        path.rooms[2][col] = 8;
                        path.rooms[3][col] = 9;
                    } else {
                        path.rooms[row + 2][col] = 9;
                    }
                    flags.insert("snake_pit".into());
                    break 'pit;
                }
            }
        }
    }
    if has(flags, "lake") && area == Area::Jungle {
        path.rooms[3] = [8; 4];
        path.rooms[4] = [7; 4];
        let mut jaws = rng.range(4) as usize;
        while jaws == path.end {
            jaws = rng.range(4) as usize;
        }
        path.rooms[4][jaws] = 9;
    }
    if area == Area::Ice {
        let forced_other = has(flags, "alien_ship") || has(flags, "yeti_lair");
        let moai = has(flags, "moai")
            || (!forced_other
                && !has(flags, "made_moai")
                && match level {
                    9 => rng.chance(1, 4),
                    10 => rng.chance(1, 3),
                    11 => rng.chance(1, 2),
                    12 => true,
                    _ => false,
                });
        if moai {
            path.rooms[rand(rng, 1, 2) as usize][rng.range(4) as usize] = 6;
            flags.insert("moai".into());
        } else if has(flags, "alien_ship") || (!has(flags, "yeti_lair") && rng.chance(1, 10)) {
            let col = rng.range(3) as usize;
            let row = rand(rng, 1, 2) as usize;
            for x in col..4 {
                path.rooms[row][x] = if x == col {
                    7
                } else if x == 3 {
                    9
                } else {
                    8
                };
            }
            flags.insert("alien_ship".into());
        } else if has(flags, "yeti_lair") || rng.chance(1, 6) {
            flags.insert("yeti_lair".into());
        }
    }
    if level > 1 && rand(rng, 1, level) <= 2 && !has(flags, "made_black_market") {
        let mut candidates = Vec::new();
        for row in 0..4 {
            for col in 0..4 {
                if path.rooms[row][col] != 0 {
                    continue;
                }
                // Preserve Classic's right-neighbor preference and edge-only left fallback.
                if col < 3 && matches!(path.rooms[row][col + 1], 1 | 2) {
                    candidates.push((col, row, 4));
                } else if col == 3 && matches!(path.rooms[row][col - 1], 1 | 2) {
                    candidates.push((col, row, 5));
                }
            }
        }
        if !candidates.is_empty() {
            let (x, y, kind) = candidates[rng.range(candidates.len() as u32) as usize];
            path.rooms[y][x] = kind;
        }
    }
    path
}

#[derive(Default)]
struct Features {
    idol: bool,
    altar: bool,
    market_shops: BTreeSet<u32>,
}

fn choose_room<'a>(
    script: &'a Script,
    script_name: &str,
    rng: &mut Rng,
    path: u8,
    above: u8,
    start: bool,
    end: bool,
    col: usize,
    row: usize,
    level: u32,
    flags: &BTreeSet<String>,
    features: &mut Features,
) -> (&'a str, &'static str) {
    let mines = script_name == "scrRoomGen";
    let jungle = script_name == "scrRoomGen2";
    let ice = script_name == "scrRoomGen3";
    let temple = script_name == "scrRoomGen4";
    let market = script_name == "scrRoomGenMarket";
    let yeti = script_name == "scrRoomGenYeti";
    let mut shop = "General";
    let (kind, number) = if script_name == "scrRoomGen5" {
        (
            if row < 3 { "boss_upper" } else { "boss_lower" }.to_string(),
            rand(rng, 1, 6),
        )
    } else if start {
        (
            "start".into(),
            if mines {
                if path == 2 {
                    rand(rng, 5, 8)
                } else {
                    rand(rng, 1, 4)
                }
            } else if ice || temple {
                if path == 2 { 2 } else { 1 }
            } else if path == 2 {
                rand(rng, 3, 4)
            } else {
                rand(rng, 1, 2)
            },
        )
    } else if end {
        (
            "end".into(),
            if jungle && has(flags, "lake") {
                5
            } else if mines {
                if above == 2 {
                    rand(rng, 2, 4)
                } else {
                    rand(rng, 3, 6)
                }
            } else if ice || temple {
                1
            } else if jungle || market {
                if above == 2 {
                    rand(rng, 1, 2)
                } else {
                    rand(rng, 3, 4)
                }
            } else if above == 2 {
                rand(rng, 1, 4)
            } else {
                rand(rng, 3, 6)
            },
        )
    } else if path == 0
        && (mines
            || (jungle && rng.chance(2, 3))
            || (ice && rng.chance(1, 2))
            || (temple && rng.chance(3, 4))
            || (yeti && rng.chance(3, 4)))
    {
        let n = if mines {
            if level > 1 && !features.altar && rng.chance(1, 16) {
                features.altar = true;
                11
            } else if features.idol || row == 3 {
                rand(rng, 1, 9)
            } else {
                let n = rand(rng, 1, 10);
                features.idol |= n == 10;
                n
            }
        } else if yeti {
            if features.idol || row == 3 {
                rand(rng, 1, 2)
            } else {
                1
            }
        } else if temple && has(flags, "city_of_gold") {
            let n = rand(rng, 1, 14);
            if n == 12 { 15 } else { n }
        } else if !features.altar && rng.chance(1, 12) {
            features.altar = true;
            if temple { 16 } else { 10 }
        } else if features.idol {
            rand(rng, 1, if temple { 11 } else { 8 })
        } else {
            let max = if temple { 12 } else { 9 };
            let n = rand(rng, 1, max);
            features.idol |= n == max;
            n
        };
        ("side".into(), n)
    } else if matches!(path, 4 | 5) {
        let mut n = if market && path == 5 {
            6
        } else if market {
            rand(rng, 1, 5)
        } else {
            rand(rng, 1, 7)
        };
        if market && path == 4 {
            for _ in 0..5 {
                if features.market_shops.insert(n) {
                    break;
                }
                n = n % 5 + 1;
            }
        }
        shop = if market && col == 3 && path == 4 {
            "Ankh"
        } else {
            match n {
                2 => "Bomb",
                3 => "Weapon",
                4 => "Rare",
                5 => "Clothing",
                6 => "Craps",
                7 => "Kissing",
                _ => "General",
            }
        };
        (
            if path == 4 { "shop_left" } else { "shop_right" }.into(),
            if market {
                0
            } else if n >= 6 {
                n
            } else {
                0
            },
        )
    } else if path >= 6 && !(path == 7 && mines) {
        let max = if path == 6 && ice {
            2
        } else if jungle && path == 7 {
            8
        } else if jungle && path == 8 {
            if above == 2 { 5 } else { 8 }
        } else {
            1
        };
        (format!("special_{path}"), rand(rng, 1, max))
    } else if ice {
        if path <= 2 && rng.chance(9, 10) {
            ("main".into(), rand(rng, 1, 9))
        } else {
            ("drop".into(), 1)
        }
    } else if matches!(path, 0 | 1) {
        (
            "main".into(),
            rand(
                rng,
                1,
                if mines || temple {
                    12
                } else if market {
                    8
                } else {
                    10
                },
            ),
        )
    } else if path == 3 {
        (
            "landing".into(),
            rand(
                rng,
                1,
                if yeti {
                    9
                } else if mines {
                    8
                } else if temple {
                    4
                } else {
                    7
                },
            ),
        )
    } else {
        let (lo, hi) = if mines || yeti {
            if path == 7 {
                (4, 12)
            } else if above != 2 {
                (1, 12)
            } else {
                (1, 8)
            }
        } else if temple {
            (1, 8)
        } else if above != 2 {
            (1, 6)
        } else {
            (1, 5)
        };
        ("drop".into(), rand(rng, lo, hi))
    };
    let mut choices: Vec<&Record> = script
        .records
        .iter()
        .filter(|r| r.variable == "strTemp" && r.kind == kind && (r.case.unwrap_or(0) == number))
        .collect();
    if choices.is_empty() {
        choices = script
            .records
            .iter()
            .filter(|r| r.variable == "strTemp" && r.kind == kind)
            .collect();
    }
    // The cemetery idol switches between the tomb and water trap in one case.
    if jungle && kind == "side" && number == 9 {
        choices.retain(|r| r.value.contains('C') == has(flags, "cemetery"));
    }
    if market && kind == "shop_left" {
        choices.retain(|r| r.context.iter().any(|c| c.contains("x == 496")) == (col == 3));
    }
    let chosen = choices
        .get(if choices.len() > 1 {
            rng.range(choices.len() as u32) as usize
        } else {
            0
        })
        .copied()
        .or_else(|| {
            script
                .records
                .iter()
                .find(|r| r.kind == "default" && r.variable == "strTemp")
        })
        .expect("every source generator has a default template");
    (&chosen.value, shop)
}

fn expand(script: &Script, rng: &mut Rng, template: &str) -> String {
    let mut bytes = template.as_bytes().to_vec();
    for index in 0..80 {
        let marker = bytes[index] as char;
        let kind = format!("obstacle_{marker}");
        let options: Vec<&Record> = script
            .records
            .iter()
            .filter(|r| r.kind == kind && r.variable == "strObs1")
            .collect();
        if options.is_empty() {
            continue;
        }
        let special_ground = marker == '5' && options.iter().any(|r| r.case == Some(100));
        let case = if special_ground {
            if rng.chance(1, 8) {
                rand(rng, 100, 102)
            } else {
                rand(rng, 1, 2)
            }
        } else {
            // Read the source switch bound: boss air deliberately has two empty cases.
            let max = options.iter().filter_map(|r| r.case).max().unwrap_or(1);
            let bound = options
                .first()
                .and_then(|r| r.context.iter().find(|c| c.starts_with("switch(rand(1,")))
                .and_then(|c| c.strip_prefix("switch(rand(1,"))
                .and_then(|s| s.split(')').next())
                .and_then(|s| s.parse::<u32>().ok())
                .unwrap_or(max);
            rand(rng, 1, bound)
        };
        let width = if marker == 'F' {
            3
        } else if marker == 'r' {
            4
        } else {
            5
        };
        for row in 0..if marker == 'V' { 4 } else { 3 } {
            let var = format!("strObs{}", row + 1);
            let value = script
                .records
                .iter()
                .find(|r| r.kind == kind && r.variable == var && r.case == Some(case))
                .map(|r| r.value.as_bytes());
            for col in 0..width {
                let at = index + row * 10 + col;
                if at < 80 {
                    bytes[at] = value.and_then(|v| v.get(col)).copied().unwrap_or(b'0');
                }
            }
        }
    }
    String::from_utf8(bytes).expect("ASCII source templates")
}

/// Generate a seeded Classic level. Flags select unlocked special destinations.
pub fn generate(rng: &mut Rng, level: u32, flags: &BTreeSet<String>) -> World {
    let area = Area::for_level(level);
    let mut selected = flags.clone();
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
        return world;
    }
    let path = room_path(rng, area, level, &mut selected);
    if area == Area::Jungle && !has(&selected, "cemetery") && rng.chance(1, 10) {
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
            } else {
                tiles::stamp(&mut world, rng, &expanded, col, row, start, end, shop);
            }
        }
    }
    world.dark = !has(&selected, "black_market")
        && (has(&selected, "dark")
            || (!has(&selected, "had_dark_level")
                && !has(&selected, "no_dark_level")
                && level > 1
                && area != Area::Ice
                && area != Area::Boss
                && rng.chance(1, 12)));
    special::apply(&mut world, rng, &selected);
    terrain::initialize(&mut world, rng);
    population::populate(&mut world, rng, &selected);
    // Construction/carving does not dispatch source Solid Destroy events.
    world.fluid_dirty = false;
    world
}

fn boundary_tile(world: &mut World, x: usize, y: usize, object: &str) {
    world.set_tile(x, y, object);
    let id = world.spawn(
        "oTerrainMetadata",
        Vec2::new(x as f64 * 16.0 + 8.0, y as f64 * 16.0 + 8.0),
    );
    let metadata = world.entity_mut(id).unwrap();
    metadata.sprite.clear();
    metadata.data.insert("invincible".into(), 1.0);
    metadata.data.insert("source_boundary".into(), 1.0);
}

pub fn tutorial() -> World {
    let mut flags = BTreeSet::new();
    flags.insert("tutorial".into());
    generate(&mut Rng::new(0), 0, &flags)
}

/// Run custom-room terrain Create events without generated-room SetupWalls.
/// scrCreateTileObj subsequently forces base sprites, while buried contents
/// created before those assignments remain present.
pub fn initialize_custom_terrain(world: &mut World, rng: &mut Rng) {
    terrain::initialize_custom(world, rng);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn campaign_perimeters_have_concrete_source_masks_and_invincible_metadata() {
        let source = include_str!(
            "../../../references/classic-1.1/Scripts/Level Generation/scrInitLevel.gml"
        );
        for object in ["oBrick", "oLush", "oDark", "oTemple"] {
            assert!(source.contains(&format!("instance_create(i*16, j*16, {object})")));
        }
        for (level, flags, object) in [
            (1, BTreeSet::new(), "oBrick"),
            (5, BTreeSet::new(), "oLush"),
            (5, BTreeSet::from(["lake".into()]), "oLush"),
            (5, BTreeSet::from(["black_market".into()]), "oLush"),
            (9, BTreeSet::from(["yeti_lair".into()]), "oDark"),
            (13, BTreeSet::from(["city_of_gold".into()]), "oTemple"),
            (16, BTreeSet::new(), "oTemple"),
        ] {
            let world = generate(&mut Rng::new(42), level, &flags);
            for y in 0..world.height {
                for x in [0, world.width - 1] {
                    assert_eq!(world.tile(x as i32, y as i32).unwrap().object, object);
                    assert!(
                        crate::catalog::bounds(object, Vec2::new(x as f64 * 16.0, y as f64 * 16.0))
                            .is_valid()
                    );
                    let center = Vec2::new(x as f64 * 16.0 + 8.0, y as f64 * 16.0 + 8.0);
                    let metadata = world
                        .entities
                        .iter()
                        .find(|entity| entity.object == "oTerrainMetadata" && entity.pos == center)
                        .unwrap();
                    assert_eq!(metadata.value("invincible"), 1.0);
                }
            }
        }
    }

    #[test]
    fn source_templates_are_complete_ascii_rooms() {
        for script in archive().scripts.values() {
            for r in &script.records {
                if r.variable == "strTemp" {
                    assert_eq!(r.value.len(), 80);
                    assert!(r.value.is_ascii());
                }
            }
        }
    }
    #[test]
    fn room_path_has_one_continuous_descent() {
        for seed in 0..512 {
            let path = room_path(&mut Rng::new(seed), Area::Mines, 1, &mut BTreeSet::new());
            let mut x = path.start;
            for y in 0..3 {
                let down = (0..4).find(|&c| path.rooms[y][c] == 2).expect("down room");
                for col in x.min(down)..=x.max(down) {
                    assert_ne!(path.rooms[y][col], 0);
                }
                assert_ne!(path.rooms[y + 1][down], 0);
                x = down;
            }
            for col in x.min(path.end)..=x.max(path.end) {
                assert_ne!(path.rooms[3][col], 0);
            }
        }
    }
    #[test]
    fn repeat_seeds_reproduce_every_area() {
        for level in [1, 4, 5, 8, 9, 12, 13, 15, 16] {
            let a = generate(&mut Rng::new(1234), level, &BTreeSet::new());
            let b = generate(&mut Rng::new(1234), level, &BTreeSet::new());
            assert_eq!(
                serde_json::to_string(&a).unwrap(),
                serde_json::to_string(&b).unwrap()
            );
            assert_eq!(a.width, 42);
            assert!(a.entities.iter().all(|e| e.id < a.next_id));
        }
    }
    #[test]
    fn forced_special_destinations_have_their_source_landmarks() {
        for (flag, level, height, object) in [
            ("black_market", 5, 34, "oAnkh"),
            ("lake", 5, 42, "oJaws"),
            ("alien_ship", 9, 38, "oAlienBoss"),
            ("yeti_lair", 9, 38, "oIce"),
            ("moai", 9, 38, "oMoai"),
            ("city_of_gold", 14, 34, "oXocBlock"),
            ("sacrifice_pit", 13, 34, "oDamsel"),
        ] {
            let flags = BTreeSet::from([flag.to_string()]);
            let world = generate(&mut Rng::new(77), level, &flags);
            assert_eq!(world.height, height, "{flag}");
            assert!(
                world.entities.iter().any(|e| e.object == object)
                    || world.tiles.iter().any(|t| t.object == object),
                "missing {object} in {flag}"
            );
        }
    }
    #[test]
    fn generated_room_extents_match_the_source_room_routes_and_ice_abyss() {
        // oTransition Step routes Ice to rLevel2 and lake to rLevel3. Their
        // similar names must not turn the Ice abyss into a lake-sized room.
        let ordinary = include_str!("../../../references/classic-1.1/Rooms/rLevel.xml");
        let ice = include_str!("../../../references/classic-1.1/Rooms/rLevel2.xml");
        let lake = include_str!("../../../references/classic-1.1/Rooms/rLevel3.xml");
        let boss = include_str!("../../../references/classic-1.1/Rooms/rOlmec.xml");
        for (level, flags, source) in [
            (1, BTreeSet::new(), ordinary),
            (5, BTreeSet::from(["black_market".into()]), ordinary),
            (9, BTreeSet::new(), ice),
            (5, BTreeSet::from(["lake".into()]), lake),
            (16, BTreeSet::new(), boss),
        ] {
            let size = source
                .split("<size ")
                .nth(1)
                .unwrap()
                .split("/>")
                .next()
                .unwrap();
            let source_height = size
                .split("height=\"")
                .nth(1)
                .unwrap()
                .split('"')
                .next()
                .unwrap()
                .parse::<usize>()
                .unwrap();
            let world = generate(&mut Rng::new(0), level, &flags);
            assert_eq!(
                world.height * 16,
                source_height,
                "source room for level {level}"
            );
            if world.area == Area::Ice {
                for x in 1..world.width - 1 {
                    assert!(
                        !world
                            .tile(x as i32, world.height as i32 - 1)
                            .unwrap()
                            .is_solid(),
                        "Ice room bottom is an abyss, not an invincible floor"
                    );
                }
            }
        }
    }
    #[test]
    fn borders_and_door_clearance() {
        for seed in 0..64 {
            for level in [1, 5, 9, 13] {
                let w = generate(&mut Rng::new(seed), level, &BTreeSet::new());
                for y in 0..w.height {
                    assert!(w.tile(0, y as i32).unwrap().is_solid());
                    assert!(w.tile(41, y as i32).unwrap().is_solid());
                }
                for pos in [w.entrance, w.exit] {
                    assert!(!w.solid_at(pos.x, pos.y), "seed {seed}, level {level}");
                }
            }
        }
    }
}
