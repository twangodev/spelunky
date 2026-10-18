//! Classic room-path construction and the original 10×8 room/obstacle templates.
//! The checked-in template archive is extracted from the licensed GML source.
use crate::model::{Area, Rng, Vec2, World};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

#[path = "generation/blocks.rs"]
mod blocks;
#[path = "generation/canopy.rs"]
mod canopy;
#[path = "generation/init_cleanup.rs"]
mod init_cleanup;
#[path = "generation/population.rs"]
mod population;
#[path = "generation/selectors.rs"]
mod selectors;
#[cfg(test)]
#[path = "generation/selectors_tests.rs"]
mod selectors_tests;
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
            // `genBlackMarket` is a run-persistent request. Classic consumes
            // it for a layout only until `madeBlackMarket` has been set, but
            // keeps it alive to suppress Lake routes later in the Jungle.
            "black_market" => {
                flags.contains("blackMarket")
                    || (flags.contains("genBlackMarket") && !flags.contains("made_black_market"))
            }
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
fn room_path(
    rng: &mut Rng,
    area: Area,
    level: u32,
    flags: &mut BTreeSet<String>,
    source_classic: bool,
) -> Path {
    let start = rng.range(4) as usize;
    // scrLevelGen draws this candidate even outside Temple and before the
    // Black Market early return; a successful sacrifice pit reuses it.
    let mut pit = rng.range(4) as usize;
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
    // The source condition is eager even outside Temple, but the Market
    // early return above never reaches it. HistoricalV1 retains its old tape.
    let sacrifice_roll = source_classic && rng.chance(1, 8);
    if area == Area::Temple
        && (has(flags, "sacrifice_pit")
            || if source_classic {
                sacrifice_roll
            } else {
                rng.chance(1, 8)
            })
    {
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
        let mut n = if x == 0 {
            rand(rng, 3, 5)
        } else if x == 3 {
            rand(rng, 5, 7)
        } else {
            rand(rng, 1, 5)
        };
        if source_classic {
            // Preserve scrLevelGen's literal nested if/else binding. If the
            // requested neighbor is occupied, its inner `else if` tries the
            // opposite side; two occupied sides change n to 5 and descend.
            if n < 3 || n > 5 {
                if x > 0 {
                    if path.rooms[y][x - 1] == 0 {
                        x -= 1;
                    } else if x < 3 {
                        if path.rooms[y][x + 1] == 0 {
                            x += 1;
                        } else {
                            n = 5;
                        }
                    }
                }
            } else if n == 3 || n == 4 {
                if x < 3 {
                    if path.rooms[y][x + 1] == 0 {
                        x += 1;
                    } else if x > 0 {
                        if path.rooms[y][x - 1] == 0 {
                            x -= 1;
                        } else {
                            n = 5;
                        }
                    }
                }
            }
        } else if n != 5 {
            let next = if n < 3 || n > 5 {
                x.saturating_sub(1)
            } else {
                (x + 1).min(3)
            };
            if path.rooms[y][next] == 0 {
                x = next;
            }
        }
        if n == 5 {
            if y < 3 {
                path.rooms[y][x] = 2;
                path.rooms[y + 1][x] = 3;
            } else {
                path.end = x;
            }
            y += 1;
        } else {
            path.rooms[y][x] = 1;
        }
    }
    if has(flags, "city_of_gold") {
        path.rooms[2][rng.range(4) as usize] = 6;
    }
    if area == Area::Mines {
        'pit: for row in 0..2 {
            for col in 0..4 {
                // Every visited source condition evaluates rand, including
                // cells whose occupied room guards prevent a pit.
                let snake_roll = source_classic && rng.chance(1, 8);
                if (row..row + 3).all(|r| path.rooms[r][col] == 0)
                    && (has(flags, "snake_pit")
                        || if source_classic {
                            snake_roll
                        } else {
                            rng.chance(1, 8)
                        })
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
    let forced_other = has(flags, "alien_ship") || has(flags, "yeti_lair");
    let moai_eligible =
        area == Area::Ice && (has(flags, "moai") || (!forced_other && !has(flags, "made_moai")));
    // Entering the outer Moai body excludes both else-if statements, even
    // when this level's placement chance fails.
    if moai_eligible {
        let moai = has(flags, "moai")
            || if source_classic {
                let nine = rng.chance(1, 4);
                if level == 9 && nine {
                    true
                } else {
                    let ten = rng.chance(1, 3);
                    if level == 10 && ten {
                        true
                    } else {
                        let eleven = rng.chance(1, 2);
                        (level == 11 && eleven) || level == 12
                    }
                }
            } else {
                match level {
                    9 => rng.chance(1, 4),
                    10 => rng.chance(1, 3),
                    11 => rng.chance(1, 2),
                    12 => true,
                    _ => false,
                }
            };
        if moai {
            let col = rng.range(4) as usize;
            let row = rand(rng, 1, 2) as usize;
            path.rooms[row][col] = 6;
            flags.insert("moai".into());
        }
    } else {
        let alien_roll = source_classic && rng.chance(1, 10);
        if area == Area::Ice
            && (has(flags, "alien_ship")
                || (!has(flags, "yeti_lair")
                    && if source_classic {
                        alien_roll
                    } else {
                        rng.chance(1, 10)
                    }))
        {
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
        } else {
            let yeti_roll = source_classic && rng.chance(1, 6);
            if area == Area::Ice
                && (has(flags, "yeti_lair")
                    || if source_classic {
                        yeti_roll
                    } else {
                        rng.chance(1, 6)
                    })
            {
                flags.insert("yeti_lair".into());
            }
        }
    }
    // The port also uses this path builder for synthetic tutorial level 0;
    // canonical scrTutorialGen never calls the campaign rand(1, currLevel).
    if level != 0 && rand(rng, 1, level) <= 2 && level > 1 && !has(flags, "made_black_market") {
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

#[derive(Default, Clone, Debug, PartialEq, Eq)]
struct Features {
    idol: bool,
    altar: bool,
    damsel: bool,
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
    world: &World,
    source_classic: bool,
) -> (&'a str, &'static str) {
    let mines = script_name == "scrRoomGen";
    let jungle = script_name == "scrRoomGen2";
    let ice = script_name == "scrRoomGen3";
    let temple = script_name == "scrRoomGen4";
    let market = script_name == "scrRoomGenMarket";
    let yeti = script_name == "scrRoomGenYeti";
    let mut shop = "General";
    let (kind, number) = if source_classic {
        let selected = selectors::select(
            rng,
            selectors::Context {
                script_name,
                path,
                above,
                start,
                end,
                col,
                row,
                level,
                flags,
                world,
            },
            features,
        );
        shop = selected.shop;
        (selected.kind, selected.number)
    } else if script_name == "scrRoomGen5" {
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

/// Game state read by source Create bodies while a new room is generated.
/// Standalone generation uses fresh-run defaults; Game projects real equipment.
#[derive(Clone, Copy, Default)]
pub(crate) struct GenerationContext {
    pub has_spectacles: bool,
    /// The source `oTreeBranch` Create body reads global.cemetary directly.
    pub has_cemetery: bool,
}

/// Persistent campaign input/output for the versioned Jungle producer.
/// The signed range is the independently proved source_rand domain, not a
/// source global clamp. Historical generation never reads or writes it.
pub(crate) const MAX_JUNGLE_PRIMARY_RECEIVERS: i32 = 22_812;
pub(crate) const MIN_SUPPORTED_MARKET_CHANCE: i32 = -(1 << 21) + MAX_JUNGLE_PRIMARY_RECEIVERS;

#[derive(Debug, Clone, Copy)]
pub(crate) struct GenerationState {
    pub revision: crate::GenerationRevision,
    pub market_chance: i32,
    pub made_market_entrance: bool,
}
impl GenerationState {
    pub(crate) fn fresh(rng: &Rng) -> Self {
        Self {
            revision: match rng.algorithm() {
                crate::RngAlgorithm::LegacySplitMix64V1 => crate::GenerationRevision::HistoricalV1,
                crate::RngAlgorithm::Gm8Lcg32V1 => crate::GenerationRevision::SourceClassicV2,
            },
            market_chance: 160,
            made_market_entrance: false,
        }
    }
}

/// scrInitLevel resets dark history at each area entry, before selection.
pub(crate) fn reset_dark_history(flags: &mut BTreeSet<String>, level: u32) {
    if level <= 1 || matches!(level, 5 | 9 | 13) {
        flags.remove("had_dark_level");
    }
}

/// Source history is separate from visible darkness: Market clears only the latter.
pub(crate) struct GeneratedLevel {
    pub world: World,
    pub dark_roll_selected: bool,
}

fn select_dark_level(
    rng: &mut Rng,
    level: u32,
    area: Area,
    flags: &BTreeSet<String>,
) -> (bool, bool) {
    // scrInitLevel170: GM8 evaluates every Boolean operand, including this
    // rand when any earlier eligibility guard is false. Stock probability is12.
    let roll_won = rand(rng, 1, 12) == 1;
    let selected = !has(flags, "had_dark_level")
        && !has(flags, "no_dark_level")
        && level != 0
        && level != 1
        && area != Area::Ice
        && level != 16
        && roll_won;
    // Source177 clears darkLevel, retaining hadDarkLevel from a winning roll.
    // Explicit dark is a standalone fixture option, applied after the draw.
    let visible = !has(flags, "black_market") && (has(flags, "dark") || selected);
    (visible, selected)
}

fn select_cemetery(rng: &mut Rng, area: Area, forced: bool, source_classic: bool) -> bool {
    // scrInitLevel149 reaches this rand for every area. Explicit fixture flags
    // do not suppress that source call; HistoricalV1 retains its old tape.
    let roll = source_classic && rng.chance(1, 10);
    area == Area::Jungle
        && !forced
        && if source_classic {
            roll
        } else {
            rng.chance(1, 10)
        }
}

/// Generate a seeded Classic level with fresh-run Create context.
/// Flags select unlocked special destinations.
pub fn generate(rng: &mut Rng, level: u32, flags: &BTreeSet<String>) -> World {
    generate_with_context(rng, level, flags, GenerationContext::default())
}

pub(crate) fn generate_with_context(
    rng: &mut Rng,
    level: u32,
    flags: &BTreeSet<String>,
    context: GenerationContext,
) -> World {
    let mut state = GenerationState::fresh(rng);
    generate_with_campaign(rng, level, flags, context, &mut state)
}

pub(crate) fn generate_with_campaign(
    rng: &mut Rng,
    level: u32,
    flags: &BTreeSet<String>,
    context: GenerationContext,
    state: &mut GenerationState,
) -> World {
    generate_campaign_result(rng, level, flags, context, state).world
}

pub(crate) fn generate_campaign_result(
    rng: &mut Rng,
    level: u32,
    flags: &BTreeSet<String>,
    context: GenerationContext,
    state: &mut GenerationState,
) -> GeneratedLevel {
    // Validate before room construction, callbacks, or RNG draws. Historical
    // serialized state remains historical even if its RNG is canonical.
    if state.revision == crate::GenerationRevision::SourceClassicV2 {
        assert_eq!(
            rng.algorithm(),
            crate::RngAlgorithm::Gm8Lcg32V1,
            "SourceClassicV2 requires canonical RNG identity"
        );
        assert!(
            (MIN_SUPPORTED_MARKET_CHANCE..=(1 << 21)).contains(&state.market_chance),
            "marketChance history is outside the proved campaign domain with one-room draw headroom"
        );
    }
    let area = Area::for_level(level);
    let mut selected = flags.clone();
    if state.revision == crate::GenerationRevision::SourceClassicV2 {
        reset_dark_history(&mut selected, level);
    }
    if area == Area::Jungle
        && !has(&selected, "black_market")
        && !has(&selected, "lake")
        // oTransition's Lake route checks genBlackMarket independently of
        // madeBlackMarket, so a spent Market request still suppresses Lakes.
        && !selected.contains("genBlackMarket")
        && rng.chance(1, 10)
    {
        selected.insert("lake".into());
    }
    let mut world = World::new(42, 34, area, level);
    if special::generate_special(&mut world, rng, flags) {
        world.fluid_dirty = false;
        return GeneratedLevel {
            world,
            dark_roll_selected: false,
        };
    }
    let source_classic = state.revision == crate::GenerationRevision::SourceClassicV2;
    // rOlmec's static Temples, Olmec, and Lava instances finish their Create
    // events before oGame.Create reaches scrInitLevel/scrLevelGen. Preserve
    // that RNG prefix now, then attach its results to the authored instances.
    let boss_create_tape = if source_classic && area == Area::Boss {
        Some(special::create_source_boss_tape(
            rng,
            has(&selected, "city_of_gold"),
            context.has_spectacles,
        ))
    } else {
        None
    };
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
    if source_classic {
        world = World::new(42, height, area, level);
        if area == Area::Boss {
            // rOlmec's authored Temple instances and their child Creates run
            // before oGame.Create starts scrInitLevel. Keep their birth serials
            // ahead of controller boundaries and generated room terrain, so
            // the later SetupWalls family pass sees the source receiver order.
            special::create_source_boss_temples(
                &mut world,
                rng,
                boss_create_tape
                    .as_ref()
                    .expect("SourceClassic Boss captures authored Temple Creates"),
            );
        }
        // scrInitLevel completes real boundary Create calls and producer
        // overrides before scrLevelGen, including out-of-storage instances.
        source_boundaries(
            &mut world,
            rng,
            has(&selected, "lake"),
            has(&selected, "city_of_gold"),
        );
        #[cfg(test)]
        source_boundary_tests::observe_before_path(&world, rng);
    }
    let path = room_path(rng, area, level, &mut selected, source_classic);
    if source_classic && (1..=16).contains(&level) {
        // Preserve the selected grid before room Create/population callbacks.
        // Source isInShop reads these markers throughout gameplay, even after
        // a keeper moves, deactivates, or is destroyed. Synthetic level0 and
        // authored tutorial/custom/hub contexts do not establish this grid.
        world.source_room_path = Some(path.rooms);
    }
    if select_cemetery(rng, area, has(&selected, "cemetery"), source_classic) {
        selected.insert("cemetery".into());
    }
    let context = GenerationContext {
        has_cemetery: has(&selected, "cemetery"),
        ..context
    };
    if !source_classic {
        // Explicit HistoricalV1 retains its old construction order and draws.
        world = World::new(42, height, area, level);
    }
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
    if !source_classic {
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
    // scrLevelGen87 sets the room-generation latch before stamping the pit.
    // It is unrelated to the selected playable character.
    features.damsel = state.revision == crate::GenerationRevision::SourceClassicV2
        && has(&selected, "sacrifice_pit");
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
                &world,
                state.revision == crate::GenerationRevision::SourceClassicV2,
            );
            let expanded = expand(script, rng, template);
            if area == Area::Boss && row == 3 && source_classic {
                tiles::stamp_at_source(
                    &mut world,
                    rng,
                    &expanded,
                    1 + col * 10,
                    37,
                    false,
                    false,
                    shop,
                    features.damsel,
                );
            } else if area == Area::Boss && row == 3 {
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
            } else if state.revision == crate::GenerationRevision::SourceClassicV2 {
                tiles::stamp_with_source_context(
                    &mut world,
                    rng,
                    &expanded,
                    col,
                    row,
                    start,
                    end,
                    shop,
                    features.damsel,
                    context,
                );
            } else {
                tiles::stamp(&mut world, rng, &expanded, col, row, start, end, shop);
            }
        }
    }
    let dark_roll_selected = if state.revision == crate::GenerationRevision::SourceClassicV2 {
        let (visible, selected_roll) = select_dark_level(rng, level, area, &selected);
        world.dark = visible;
        selected_roll
    } else {
        // Preserve explicitly historical generation/replay streams.
        world.dark = !has(&selected, "black_market")
            && (has(&selected, "dark")
                || (!has(&selected, "had_dark_level")
                    && !has(&selected, "no_dark_level")
                    && level > 1
                    && area != Area::Ice
                    && area != Area::Boss
                    && rng.chance(1, 12)));
        false
    };
    special::apply(
        &mut world,
        rng,
        &selected,
        state.revision,
        boss_create_tape.as_ref(),
    );
    apply_wanted_posters(&mut world, &selected);
    let graphics_high = !has(&selected, "low_graphics");
    terrain::create_generated_with_context(&mut world, rng, context);
    // Construction/carving does not dispatch source Solid Destroy events.
    // Population does; preserve any real parent checkWater request it emits.
    world.fluid_dirty = false;
    population::populate_with_campaign(&mut world, rng, &selected, context, state, &path.rooms);
    #[cfg(test)]
    source_boundary_tests::observe_before_setup(&world, rng);
    terrain::setup_generated_with_revision(
        &mut world,
        rng,
        graphics_high,
        state.revision,
        has(&selected, "cemetery"),
    );
    if graphics_high {
        background_extras(&mut world, rng, source_classic);
    }
    #[cfg(test)]
    terrain::observe_init_cleanup(&world, false);
    init_cleanup::cleanup(&mut world, state.revision);
    #[cfg(test)]
    terrain::observe_init_cleanup(&world, true);
    terrain::finalize_generated(&mut world, state.revision);
    GeneratedLevel {
        world,
        dark_roll_selected,
    }
}

/// oBricks Create calls scrSetupWalls(224) after authored hub construction.
pub(crate) fn initialize_hub_fringe(world: &mut World, rng: &mut Rng) {
    let origins: Vec<_> = world
        .tiles
        .iter()
        .enumerate()
        .filter_map(|(index, tile)| {
            if tile.object != "oBrick" {
                return None;
            }
            let x = (index % world.width) as i32;
            let y = (index / world.width) as i32;
            let up = y == 0
                || world
                    .tile(x, y - 1)
                    .is_some_and(|tile| matches!(tile.object.as_str(), "oBrick" | "oHardBlock"));
            (!up).then_some(Vec2::new(f64::from(x * 16), f64::from((y - 1) * 16)))
        })
        .collect();
    for origin in origins {
        let source_x = if rng.range(3) < 2 { 0.0 } else { 16.0 };
        terrain::decoration(world, "bgCaveTop", origin, source_x, 16.0, 3.0);
    }
}

/// Run the source oBricks Create handler for an authored room. The authored
/// controller invokes scrSetupWalls(224); generated levels use a different
/// boundary argument and have their own setup order.
pub(crate) fn initialize_authored_setup_walls(
    world: &mut World,
    rng: &mut Rng,
    revision: crate::GenerationRevision,
    graphics_high: bool,
    city: bool,
    cemetery: bool,
) -> bool {
    if revision != crate::GenerationRevision::SourceClassicV2
        || !world
            .entities
            .iter()
            .any(|entity| entity.alive && entity.object == "oBricks")
    {
        return false;
    }
    terrain::setup_authored_walls(world, rng, graphics_high, city, cemetery);
    true
}

/// The only authored oBricks room with this family is rSun (12 Vines).
/// Its SetupWalls Vine pass runs even when the Brick fringe is disabled.
pub(crate) fn initialize_hub_canopy(world: &mut World, revision: crate::GenerationRevision) {
    if revision == crate::GenerationRevision::SourceClassicV2 {
        canopy::vines(world);
    }
}

fn background_extras(world: &mut World, rng: &mut Rng, source_classic: bool) {
    // scrInitLevel:228–240, after SetupWalls and before entity generation.
    for _ in 0..20 {
        let background = if source_classic {
            // Boolean operands are eager; else-if statement reach is not.
            let lush = rng.range(3) < 2;
            if world.area == Area::Jungle && lush {
                "bgExtrasLush"
            } else {
                let ice = rng.range(3) < 2;
                if world.area == Area::Ice && ice {
                    "bgExtrasIce"
                } else {
                    let temple = rng.range(3) < 2;
                    if world.area == Area::Temple && temple {
                        "bgExtrasTemple"
                    } else {
                        "bgExtras"
                    }
                }
            }
        } else {
            match world.area {
                Area::Jungle if rng.range(3) < 2 => "bgExtrasLush",
                Area::Ice if rng.range(3) < 2 => "bgExtrasIce",
                Area::Temple if rng.range(3) < 2 => "bgExtrasTemple",
                _ => "bgExtras",
            }
        };
        let source_x = f64::from(rng.range(2) * 32);
        let origin = Vec2::new(
            f64::from((1 + rng.range(42)) * 16),
            f64::from((1 + rng.range(33)) * 16),
        );
        terrain::decoration(world, background, origin, source_x, 32.0, 10002.0);
    }
}

fn apply_wanted_posters(world: &mut World, flags: &BTreeSet<String>) {
    let wanted = flags.contains("murderer") || flags.contains("wanted");
    let source_x = if flags.contains("character_damsel") {
        32.0
    } else if flags.contains("character_tunnelman") {
        64.0
    } else {
        0.0
    };
    for poster in world
        .entities
        .iter_mut()
        .filter(|entity| entity.value("wanted_marker") != 0.0)
    {
        if !wanted {
            poster.alive = false;
            continue;
        }
        for (key, value) in [
            ("depth", 9004.0),
            ("source_x", source_x),
            ("source_y", 0.0),
            ("source_w", 32.0),
            ("source_h", 32.0),
        ] {
            poster.data.insert(key.into(), value);
        }
    }
    world.entities.retain(|entity| entity.alive);
}

// scrInitLevel:75-140. The loop is column-major, includes x=672, and
// creates its Ice floor below (not instead of) rLevel2's falling threshold.
fn source_boundaries(world: &mut World, rng: &mut Rng, lake: bool, city: bool) {
    let k = if world.area == Area::Boss {
        54
    } else if world.area == Area::Ice {
        38
    } else if lake {
        41
    } else {
        33
    };
    let object = match world.area {
        Area::Ice => "oDark",
        Area::Boss => "oTemple",
        _ if lake => "oLush",
        Area::Mines => "oBrick",
        Area::Jungle => "oLush",
        Area::Temple => "oTemple",
    };
    for i in 0..=42 {
        for j in 0..=k {
            let side_or_top = i == 0 || i == 41 || j == 0;
            let floor = match world.area {
                Area::Ice | Area::Boss => false,
                _ if lake => j >= 41,
                _ => j >= 33,
            };
            if side_or_top || floor {
                terrain::create_source_boundary(world, rng, i, j, object, city, true);
            }
        }
    }
    if world.area == Area::Ice {
        for i in 0..=42 {
            terrain::create_source_boundary(world, rng, i, 40, "oDark", city, false);
        }
    }
}

#[cfg(test)]
#[path = "generation/source_boundary_tests.rs"]
mod source_boundary_tests;

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

    // Independent column-major transcription of scrLevelGen.gml. This uses
    // only the RNG primitive, not room_path/rand/has or its candidate builder.
    fn source_room_path(
        rng: &mut Rng,
        area: Area,
        level: u32,
        made_moai: bool,
        made_market: bool,
    ) -> (Path, BTreeSet<String>) {
        let mut rooms = [[0_u8; 5]; 4];
        let start = rng.range(4) as usize;
        let mut n = rng.range(4) as usize;
        let (mut x, mut y) = (start, 0_usize);
        let mut end = start;
        let mut selected = BTreeSet::new();
        if made_moai {
            selected.insert("made_moai".into());
        }
        if made_market {
            selected.insert("made_black_market".into());
        }
        rooms[x][y] = 1;
        let sacrifice = 1 + rng.range(8);
        if area == Area::Temple && sacrifice == 1 {
            while n == x {
                n = rng.range(4) as usize;
            }
            rooms[n] = [7, 8, 8, 9, 0];
            selected.insert("sacrifice_pit".into());
        }
        while y < 4 {
            let mut draw = match x {
                0 => 3 + rng.range(3),
                3 => 5 + rng.range(3),
                _ => 1 + rng.range(5),
            };
            let (previous_x, previous_y) = (x, y);
            // Keep the source's unbraced nested if/else structure: when the
            // requested horizontal neighbor is occupied, its `else if`
            // checks the opposite side; if that side is occupied too, source
            // changes n to 5 and descends instead of staying in this room.
            if draw < 3 || draw > 5 {
                if x > 0 {
                    if rooms[x - 1][y] == 0 {
                        x -= 1;
                    } else if x < 3 {
                        if rooms[x + 1][y] == 0 {
                            x += 1;
                        } else {
                            draw = 5;
                        }
                    }
                }
            } else if draw == 3 || draw == 4 {
                if x < 3 {
                    if rooms[x + 1][y] == 0 {
                        x += 1;
                    } else if x > 0 {
                        if rooms[x - 1][y] == 0 {
                            x -= 1;
                        } else {
                            draw = 5;
                        }
                    }
                }
            }
            if draw == 5 {
                y += 1;
                if y < 4 {
                    rooms[previous_x][previous_y] = 2;
                    rooms[x][y] = 3;
                } else {
                    end = x;
                }
            } else {
                rooms[x][y] = 1;
            }
        }
        if area == Area::Mines {
            'snake: for row in 0..2 {
                for col in 0..4 {
                    let snake = 1 + rng.range(8);
                    if rooms[col][row] == 0
                        && rooms[col][row + 1] == 0
                        && rooms[col][row + 2] == 0
                        && snake == 1
                    {
                        rooms[col][row] = 7;
                        rooms[col][row + 1] = 8;
                        if row == 0 && rooms[col][3] == 0 {
                            rooms[col][2] = 8;
                            rooms[col][3] = 9;
                        } else {
                            rooms[col][row + 2] = 9;
                        }
                        selected.insert("snake_pit".into());
                        break 'snake;
                    }
                }
            }
        }
        if !made_moai && area == Area::Ice {
            let nine = 1 + rng.range(4);
            let placed = if level == 9 && nine == 1 {
                true
            } else {
                let ten = 1 + rng.range(3);
                if level == 10 && ten == 1 {
                    true
                } else {
                    let eleven = 1 + rng.range(2);
                    (level == 11 && eleven == 1) || level == 12
                }
            };
            if placed {
                let col = rng.range(4) as usize;
                let row = 1 + rng.range(2) as usize;
                rooms[col][row] = 6;
                selected.insert("moai".into());
            }
        } else {
            let alien = 1 + rng.range(10);
            if area == Area::Ice && alien == 1 {
                let col = rng.range(3) as usize;
                let row = 1 + rng.range(2) as usize;
                for i in col..4 {
                    rooms[i][row] = if i == col {
                        7
                    } else if i == 3 {
                        9
                    } else {
                        8
                    };
                }
                selected.insert("alien_ship".into());
            } else {
                let yeti = 1 + rng.range(6);
                if area == Area::Ice && yeti == 1 {
                    selected.insert("yeti_lair".into());
                }
            }
        }
        let shop_draw = 1 + rng.range(level);
        if shop_draw <= 2 && level > 1 && !made_market {
            let mut possibilities = [[0_u8; 4]; 4];
            let mut count = 0;
            for row in 0..4 {
                for col in 0..4 {
                    if rooms[col][row] == 0 {
                        if col < 3 {
                            if rooms[col + 1][row] == 1 || rooms[col + 1][row] == 2 {
                                possibilities[col][row] = 4;
                                count += 1;
                            }
                        } else if rooms[col - 1][row] == 1 || rooms[col - 1][row] == 2 {
                            possibilities[col][row] = 5;
                            count += 1;
                        }
                    }
                }
            }
            if count > 0 {
                let mut choice = rng.range(count);
                'shop: for row in 0..4 {
                    for col in 0..4 {
                        if possibilities[col][row] != 0 {
                            if choice == 0 {
                                rooms[col][row] = possibilities[col][row];
                                break 'shop;
                            }
                            choice -= 1;
                        }
                    }
                }
            }
        }
        (
            Path {
                rooms: std::array::from_fn(|row| std::array::from_fn(|col| rooms[col][row])),
                start,
                end,
            },
            selected,
        )
    }

    fn historical_room_path_oracle(
        rng: &mut Rng,
        area: Area,
        level: u32,
        made_moai: bool,
        made_market: bool,
    ) -> (Path, BTreeSet<String>) {
        let mut rooms = [[0_u8; 5]; 4];
        let start = rng.range(4) as usize;
        let mut n = rng.range(4) as usize;
        let (mut x, mut y) = (start, 0_usize);
        let mut end = start;
        let mut selected = BTreeSet::new();
        if made_moai {
            selected.insert("made_moai".into());
        }
        if made_market {
            selected.insert("made_black_market".into());
        }
        rooms[x][y] = 1;
        if area == Area::Temple && 1 + rng.range(8) == 1 {
            while n == x {
                n = rng.range(4) as usize;
            }
            rooms[n] = [7, 8, 8, 9, 0];
            selected.insert("sacrifice_pit".into());
        }
        while y < 4 {
            let draw = match x {
                0 => 3 + rng.range(3),
                3 => 5 + rng.range(3),
                _ => 1 + rng.range(5),
            };
            let (previous_x, previous_y) = (x, y);
            if draw < 3 || draw > 5 {
                if x > 0 && rooms[x - 1][y] == 0 {
                    x -= 1;
                }
            } else if draw == 3 || draw == 4 {
                if x < 3 && rooms[x + 1][y] == 0 {
                    x += 1;
                }
            }
            if draw == 5 {
                y += 1;
                if y < 4 {
                    rooms[previous_x][previous_y] = 2;
                    rooms[x][y] = 3;
                } else {
                    end = x;
                }
            } else {
                rooms[x][y] = 1;
            }
        }
        if area == Area::Mines {
            'snake: for row in 0..2 {
                for col in 0..4 {
                    if rooms[col][row] == 0
                        && rooms[col][row + 1] == 0
                        && rooms[col][row + 2] == 0
                        && 1 + rng.range(8) == 1
                    {
                        rooms[col][row] = 7;
                        rooms[col][row + 1] = 8;
                        if row == 0 && rooms[col][3] == 0 {
                            rooms[col][2] = 8;
                            rooms[col][3] = 9;
                        } else {
                            rooms[col][row + 2] = 9;
                        }
                        selected.insert("snake_pit".into());
                        break 'snake;
                    }
                }
            }
        }
        if !made_moai && area == Area::Ice {
            let placed = match level {
                9 => 1 + rng.range(4) == 1,
                10 => 1 + rng.range(3) == 1,
                11 => 1 + rng.range(2) == 1,
                12 => true,
                _ => false,
            };
            if placed {
                let col = rng.range(4) as usize;
                let row = 1 + rng.range(2) as usize;
                rooms[col][row] = 6;
                selected.insert("moai".into());
            }
        } else if area == Area::Ice && 1 + rng.range(10) == 1 {
            let col = rng.range(3) as usize;
            let row = 1 + rng.range(2) as usize;
            for i in col..4 {
                rooms[i][row] = if i == col {
                    7
                } else if i == 3 {
                    9
                } else {
                    8
                };
            }
            selected.insert("alien_ship".into());
        } else if area == Area::Ice && 1 + rng.range(6) == 1 {
            selected.insert("yeti_lair".into());
        }
        let shop_draw = 1 + rng.range(level);
        if shop_draw <= 2 && level > 1 && !made_market {
            let mut possibilities = [[0_u8; 4]; 4];
            let mut count = 0;
            for row in 0..4 {
                for col in 0..4 {
                    if rooms[col][row] == 0 {
                        if col < 3 {
                            if rooms[col + 1][row] == 1 || rooms[col + 1][row] == 2 {
                                possibilities[col][row] = 4;
                                count += 1;
                            }
                        } else if rooms[col - 1][row] == 1 || rooms[col - 1][row] == 2 {
                            possibilities[col][row] = 5;
                            count += 1;
                        }
                    }
                }
            }
            if count > 0 {
                let mut choice = rng.range(count);
                'shop: for row in 0..4 {
                    for col in 0..4 {
                        if possibilities[col][row] != 0 {
                            if choice == 0 {
                                rooms[col][row] = possibilities[col][row];
                                break 'shop;
                            }
                            choice -= 1;
                        }
                    }
                }
            }
        }
        (
            Path {
                rooms: std::array::from_fn(|row| std::array::from_fn(|col| rooms[col][row])),
                start,
                end,
            },
            selected,
        )
    }

    #[test]
    fn source_room_path_matches_literal_branch_draw_order_and_next_rng() {
        for seed in 0..128 {
            for level in [1, 2, 5, 9, 10, 11, 12, 13, 14] {
                for made_moai in [false, true] {
                    for made_market in [false, true] {
                        let mut expected_rng = Rng::new(seed);
                        let (expected, expected_flags) = source_room_path(
                            &mut expected_rng,
                            Area::for_level(level),
                            level,
                            made_moai,
                            made_market,
                        );
                        let mut selected = BTreeSet::new();
                        if made_moai {
                            selected.insert("made_moai".into());
                        }
                        if made_market {
                            selected.insert("made_black_market".into());
                        }
                        let mut actual_rng = Rng::new(seed);
                        let actual = room_path(
                            &mut actual_rng,
                            Area::for_level(level),
                            level,
                            &mut selected,
                            true,
                        );
                        assert_eq!(
                            actual.rooms, expected.rooms,
                            "seed{seed} level{level} madeMoai{made_moai} madeMarket{made_market}"
                        );
                        assert_eq!((actual.start, actual.end), (expected.start, expected.end));
                        assert_eq!(selected, expected_flags);
                        assert_eq!(
                            actual_rng.next_u64(),
                            expected_rng.next_u64(),
                            "next RNG seed{seed} level{level} madeMoai{made_moai} madeMarket{made_market}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn source_black_market_return_consumes_both_initial_draws() {
        for seed in 0..32 {
            let mut expected = Rng::new(seed);
            expected.range(4); // startRoomX
            expected.range(4); // n, before the Black Market return
            let mut actual = Rng::new(seed);
            let mut flags = BTreeSet::from(["black_market".into()]);
            let path = room_path(&mut actual, Area::Jungle, 5, &mut flags, true);
            assert_eq!((path.start, path.end), (0, 3));
            assert_eq!(actual.next_u64(), expected.next_u64(), "seed{seed}");
        }
    }

    #[test]
    fn spent_black_market_request_suppresses_lake_without_repeating_market() {
        // Seed zero's first canonical draw selects the source Lake branch.
        let mut lake_rng = Rng::new(0);
        assert!(lake_rng.chance(1, 10));
        let ordinary = generate_campaign_result(
            &mut Rng::new(0),
            7,
            &BTreeSet::from(["made_black_market".into()]),
            GenerationContext::default(),
            &mut GenerationState::fresh(&Rng::new(0)),
        );
        assert_eq!(ordinary.world.name, "Lake");

        // Classic leaves genBlackMarket set for the run after its one Market
        // layout. It prevents later Jungle Lake routes even though
        // madeBlackMarket prevents the special layout from repeating.
        let flags = BTreeSet::from(["genBlackMarket".into(), "made_black_market".into()]);
        let persisted = generate_campaign_result(
            &mut Rng::new(0),
            7,
            &flags,
            GenerationContext::default(),
            &mut GenerationState::fresh(&Rng::new(0)),
        );
        assert_eq!(persisted.world.name, "Jungle");
    }

    #[test]
    fn source_moai_eligibility_does_not_fall_through_to_other_ice_events() {
        for level in 9..=12 {
            let mut placed = false;
            let mut absent = false;
            for seed in 0..128 {
                let mut selected = BTreeSet::new();
                room_path(&mut Rng::new(seed), Area::Ice, level, &mut selected, true);
                placed |= selected.contains("moai");
                absent |= !selected.contains("moai");
                assert!(!selected.contains("alien_ship") && !selected.contains("yeti_lair"));
            }
            assert!(placed);
            assert_eq!(absent, level != 12);
        }
    }

    #[test]
    fn source_path_choices_reach_actual_natural_generation() {
        for (level, made, feature) in [
            (9, false, "moai"),
            (9, false, "ordinary"),
            (12, false, "moai"),
            (9, true, "alien_ship"),
            (9, true, "yeti_lair"),
            (13, false, "sacrifice_pit"),
        ] {
            let seed = (0..128)
                .find(|seed| {
                    // Literal scrInitLevel boundary prefix before scrLevelGen:
                    // Ice:119 side/top Dark +43 ordinary y640 Dark births;
                    // Temple:150 boundary Temple births. All own Creates draw
                    // once; strict source interior child bounds exclude them.
                    // Independent finite GM8 LCG, not generated-world outcome
                    // selection or a call to the production boundary helper.
                    let calls = match level {
                        9 | 12 => 162,
                        13 => 150,
                        _ => unreachable!("six fixed Ice/Temple branch cases"),
                    };
                    let mut boundary_state: u32 = *seed;
                    for _ in 0..calls {
                        boundary_state = boundary_state.wrapping_mul(0x0808_8405).wrapping_add(1);
                    }
                    let (_, selected) = source_room_path(
                        &mut Rng::new(boundary_state),
                        Area::for_level(level),
                        level,
                        made,
                        false,
                    );
                    if feature == "ordinary" {
                        !selected.contains("moai")
                    } else {
                        selected.contains(feature)
                    }
                })
                .expect("bounded natural source event fixture");
            let flags = if made {
                BTreeSet::from(["made_moai".into()])
            } else {
                BTreeSet::new()
            };
            let world = generate(&mut Rng::new(seed), level, &flags);
            let has_object = |object| world.entities.iter().any(|e| e.object == object);
            match feature {
                "moai" => assert!(has_object("oMoai"), "seed{seed} level{level}"),
                "ordinary" => {
                    assert!(!has_object("oMoai") && !has_object("oAlienBoss"));
                    assert_eq!(world.name, "Ice Caves");
                }
                "alien_ship" => {
                    assert!(has_object("oAlienBoss"));
                    assert_eq!(world.name, "Alien Ship");
                }
                "yeti_lair" => assert_eq!(world.name, "Yeti Lair"),
                "sacrifice_pit" => assert_eq!(world.name, "Sacrifice Pit"),
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn synthetic_level_zero_and_authored_tutorial_remain_valid() {
        let path = room_path(
            &mut Rng::new(0),
            Area::Mines,
            0,
            &mut BTreeSet::new(),
            false,
        );
        assert!(path.start < 4 && path.end < 4);
        let world = tutorial();
        assert_eq!((world.width, world.height), (42, 19));
        assert!(world.entities.iter().any(|e| e.object == "oExit"));
    }
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
            let path = room_path(
                &mut Rng::new(seed),
                Area::Mines,
                1,
                &mut BTreeSet::new(),
                true,
            );
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
                for (name, pos) in [("entrance", w.entrance), ("exit", w.exit)] {
                    assert!(
                        !w.tiles_point(pos, "oSolid"),
                        "{name} terrain at {pos:?}, seed {seed}, level {level}"
                    );
                }
                // scrEntityGen:453 excludes the start room from Temple floor
                // enemies. It does not exclude the exit room: mobile SmashTraps
                // can initially occupy an exit without embedding it in terrain.
                assert!(
                    !w.solid_at(w.entrance.x, w.entrance.y),
                    "entrance live solid at {:?}, seed {seed}, level {level}",
                    w.entrance
                );
            }
        }
    }

    #[test]
    fn temple_exit_smash_moves_away_and_allows_a_local_exit_interaction() {
        use crate::model::{Game, InputFrame, RulesPreset, RunState};

        // A bounded canonical search selected seed 118 solely for its natural
        // exit-origin LEFT SmashTrap and clear, supported local approach,
        // before any player or trap steps. The approach is checked below.
        let mut game = Game::shell(118, RulesPreset::Classic);
        // This fixed geometry recording predates the SourceClassicV2 selectors.
        game.generation_revision = crate::GenerationRevision::HistoricalV1;
        // Retain the natural exit-trap layout with both features made earlier:
        // scrInitLevel:180,188 skips each scheduling chain once already made.
        game.flags.insert("made_udjat_eye".into());
        game.flags.insert("made_market_entrance".into());
        let mut generation_state = GenerationState::fresh(&game.rng);
        generation_state.revision = game.generation_revision;
        game.world = generate_with_campaign(
            &mut game.rng,
            13,
            &game.flags,
            GenerationContext::default(),
            &mut generation_state,
        );
        // The static canonical discovery pins this natural layout before
        // any movement. Source doors are created at a tile origin and the
        // logical interaction position is its center.
        assert_eq!(game.world.exit, Vec2::new(584.0, 472.0));
        assert_eq!(game.world.exit.x % 16.0, 8.0);
        assert_eq!(game.world.exit.y % 16.0, 8.0);
        let door_origin = Vec2::new(game.world.exit.x - 8.0, game.world.exit.y - 8.0);
        let trap = game
            .world
            .entities
            .iter()
            .find(|e| e.alive && e.object == "oSmashTrap" && e.pos == door_origin)
            .expect("scrEntityGen:470-471 permits a SmashTrap at the raw exit origin");
        let trap_id = trap.id;
        assert_eq!(trap.data.get("direction"), Some(&2.0));
        assert!(game.world.solid_at(door_origin.x, door_origin.y));
        assert!(!game.world.tiles_point(door_origin, "oSolid"));
        assert!(!game.world.collides_except(
            crate::model::Rect::new(game.world.exit.x - 38., game.world.exit.y - 40., 30., 48.,),
            trap_id,
        ));
        for offset in -37..=-8 {
            assert!(game.world.tiles_point(
                Vec2::new(
                    game.world.exit.x + f64::from(offset),
                    game.world.exit.y + 8.
                ),
                "oSolid",
            ));
        }

        // This is a focused interaction fixture near the generated exit, not
        // an entrance-to-exit navigation replay. Retain every generated object
        // and the post-generation RNG; only ordinary inputs follow this pose.
        game.player.pos = Vec2::new(game.world.exit.x - 32.0, game.world.exit.y);
        game.player.previous_pos = game.player.pos;
        let initial_health = game.player.health;
        let mut vacated = false;
        for tick in 0..40 {
            game.step(&InputFrame {
                jump_pressed: tick == 2,
                jump: (2..10).contains(&tick),
                right: tick >= 2 && game.player.pos.x < game.world.exit.x - 2.0,
                up: true,
                ..InputFrame::default()
            });
            let trap = game.world.entity(trap_id).expect("generated trap persists");
            assert!(trap.alive);
            if !game.world.solid_at(door_origin.x, door_origin.y) {
                assert!(trap.pos.x < door_origin.x);
                vacated = true;
            }
            if game.state != RunState::Playing {
                break;
            }
        }
        assert!(
            vacated,
            "ordinary SmashTrap movement must clear the exit gate"
        );
        assert_eq!(game.player.health, initial_health);
        assert_eq!(game.state, RunState::Ending);
    }
    #[test]
    fn historical_room_path_keeps_its_prior_tape_for_both_rng_algorithms() {
        for seed in 0..64 {
            for level in [1, 2, 5, 9, 10, 11, 12, 13, 14, 16] {
                for made_moai in [false, true] {
                    for made_market in [false, true] {
                        for canonical_rng in [false, true] {
                            let mut actual = if canonical_rng {
                                Rng::new(seed)
                            } else {
                                Rng::legacy(u64::from(seed))
                            };
                            let mut expected = actual.clone();
                            let mut flags = BTreeSet::new();
                            if made_moai {
                                flags.insert("made_moai".into());
                            }
                            if made_market {
                                flags.insert("made_black_market".into());
                            }
                            let (oracle, oracle_flags) = historical_room_path_oracle(
                                &mut expected,
                                Area::for_level(level),
                                level,
                                made_moai,
                                made_market,
                            );
                            let path = room_path(
                                &mut actual,
                                Area::for_level(level),
                                level,
                                &mut flags,
                                false,
                            );
                            assert_eq!(path.rooms, oracle.rooms);
                            assert_eq!((path.start, path.end), (oracle.start, oracle.end));
                            assert_eq!(flags, oracle_flags);
                            assert_eq!(actual.state_bits(), expected.state_bits());
                            assert_eq!(actual.next_u64(), expected.next_u64());
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn cemetery_predicate_draws_on_wrong_areas_but_historical_tape_stays_conditional() {
        for seed in 0..64 {
            for area in [
                Area::Mines,
                Area::Jungle,
                Area::Ice,
                Area::Temple,
                Area::Boss,
            ] {
                for forced in [false, true] {
                    let mut actual = Rng::new(seed);
                    let mut source = actual.clone();
                    let chance = 1 + source.range(10);
                    assert_eq!(
                        select_cemetery(&mut actual, area, forced, true),
                        area == Area::Jungle && !forced && chance == 1
                    );
                    assert_eq!(actual.next_u64(), source.next_u64());
                    let mut old = Rng::new(seed);
                    let mut old_oracle = old.clone();
                    let expected = area == Area::Jungle && !forced && old_oracle.chance(1, 10);
                    assert_eq!(select_cemetery(&mut old, area, forced, false), expected);
                    assert_eq!(old.next_u64(), old_oracle.next_u64());
                }
            }
        }
    }

    #[test]
    fn background_extras_match_each_reached_source_predicate_and_three_tile_draws() {
        for seed in 0..32 {
            for area in [
                Area::Mines,
                Area::Jungle,
                Area::Ice,
                Area::Temple,
                Area::Boss,
            ] {
                for source_classic in [false, true] {
                    let mut world = World::new(42, 34, area, 1);
                    let mut actual = Rng::new(seed);
                    let mut expected = actual.clone();
                    let mut decorations = Vec::new();
                    for _ in 0..20 {
                        let sprite = if source_classic {
                            let lush_roll = 1 + expected.range(3);
                            if area == Area::Jungle && lush_roll < 3 {
                                "bgExtrasLush"
                            } else {
                                let ice_roll = 1 + expected.range(3);
                                if area == Area::Ice && ice_roll < 3 {
                                    "bgExtrasIce"
                                } else {
                                    let temple_roll = 1 + expected.range(3);
                                    if area == Area::Temple && temple_roll < 3 {
                                        "bgExtrasTemple"
                                    } else {
                                        "bgExtras"
                                    }
                                }
                            }
                        } else {
                            match area {
                                Area::Jungle if expected.range(3) < 2 => "bgExtrasLush",
                                Area::Ice if expected.range(3) < 2 => "bgExtrasIce",
                                Area::Temple if expected.range(3) < 2 => "bgExtrasTemple",
                                _ => "bgExtras",
                            }
                        };
                        let source_x = f64::from(expected.range(2) * 32);
                        let pos = Vec2::new(
                            f64::from((1 + expected.range(42)) * 16),
                            f64::from((1 + expected.range(33)) * 16),
                        );
                        decorations.push((sprite, pos, source_x));
                    }
                    background_extras(&mut world, &mut actual, source_classic);
                    assert_eq!(world.entities.len(), 20);
                    for (entity, (sprite, pos, source_x)) in world.entities.iter().zip(decorations)
                    {
                        assert_eq!(entity.object, "oGenerationDecoration");
                        assert_eq!(entity.sprite, sprite);
                        assert_eq!(entity.pos, pos);
                        assert_eq!(entity.value("source_x"), source_x);
                        assert_eq!(entity.value("source_w"), 32.);
                        assert_eq!(entity.value("depth"), 10002.);
                    }
                    assert_eq!(actual.state_bits(), expected.state_bits());
                    assert_eq!(actual.next_u64(), expected.next_u64());
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "generation/darkness_tests.rs"]
mod darkness_tests;
