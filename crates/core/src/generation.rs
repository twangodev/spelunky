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
                    || (flags.contains("genBlackMarket") && !has(flags, "made_black_market"))
            }
            "made_black_market" => flags.contains("madeBlackMarket"),
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

/// `genBlackMarket` survives room transitions, but only becomes an active
/// Market while generating the Jungle. Keep explicit per-room test/host flags
/// compatible while scoping that run-persistent request to its source area.
fn source_black_market_active(flags: &BTreeSet<String>, area: Area) -> bool {
    flags.contains("black_market")
        || flags.contains("blackMarket")
        || (area == Area::Jungle
            && flags.contains("genBlackMarket")
            && !has(flags, "made_black_market"))
}

/// Mutable source globals that select optional campaign layouts. These values
/// live in `Game::data` in serialized snapshots, so SourceClassicV2 generation
/// reads them at each room boundary instead of treating the stock defaults as
/// constants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SourceProbabilities {
    pub dark_level: i32,
    pub snake_pit: i32,
    pub cemetery: i32,
    pub lake: i32,
    pub yeti_lair: i32,
    pub alien: i32,
    pub sacrifice_pit: i32,
}

const MAX_SOURCE_PROBABILITY_WIDTH: i64 = 1 << 21;

impl Default for SourceProbabilities {
    fn default() -> Self {
        Self {
            dark_level: 12,
            snake_pit: 8,
            cemetery: 10,
            lake: 10,
            yeti_lair: 6,
            alien: 10,
            sacrifice_pit: 8,
        }
    }
}

impl SourceProbabilities {
    pub(crate) fn from_globals(globals: &BTreeMap<String, f64>) -> Self {
        fn read(globals: &BTreeMap<String, f64>, name: &str, default: i32) -> i32 {
            let value = globals.get(name).copied().unwrap_or(f64::from(default));
            assert!(
                value.is_finite() && value.fract() == 0.0,
                "source probability {name} must be a finite integer"
            );
            assert!(
                (-MAX_SOURCE_PROBABILITY_WIDTH as f64..=MAX_SOURCE_PROBABILITY_WIDTH as f64)
                    .contains(&value),
                "source probability {name} is outside the supported source_rand width"
            );
            value as i32
        }

        let defaults = Self::default();
        Self {
            dark_level: read(globals, "probDarkLevel", defaults.dark_level),
            snake_pit: read(globals, "probSnakePit", defaults.snake_pit),
            cemetery: read(globals, "probCemetary", defaults.cemetery),
            lake: read(globals, "probLake", defaults.lake),
            yeti_lair: read(globals, "probYetiLair", defaults.yeti_lair),
            alien: read(globals, "probAlien", defaults.alien),
            sacrifice_pit: read(globals, "probSacPit", defaults.sacrifice_pit),
        }
    }

    fn validate(self) {
        for (name, value) in [
            ("probDarkLevel", self.dark_level),
            ("probSnakePit", self.snake_pit),
            ("probCemetary", self.cemetery),
            ("probLake", self.lake),
            ("probYetiLair", self.yeti_lair),
            ("probAlien", self.alien),
            ("probSacPit", self.sacrifice_pit),
        ] {
            assert!(
                i64::from(value).abs() <= MAX_SOURCE_PROBABILITY_WIDTH,
                "source probability {name} is outside the supported source_rand width"
            );
        }
    }
}

/// Evaluate the source expression `rand(1, global.prob*) == 1` while
/// consuming exactly one canonical RNG draw, including zero and signed widths.
fn source_probability_roll(rng: &mut Rng, denominator: i32) -> bool {
    rng.source_rand(1, denominator)
        .expect("source probability was validated before generation")
        == 1
}

#[derive(Debug)]
struct Path {
    rooms: [[u8; 4]; 5],
    start: usize,
    end: usize,
}
#[cfg(test)]
fn room_path(
    rng: &mut Rng,
    area: Area,
    level: u32,
    flags: &mut BTreeSet<String>,
    source_classic: bool,
) -> Path {
    room_path_with_probabilities(
        rng,
        area,
        level,
        flags,
        source_classic,
        SourceProbabilities::default(),
    )
}

fn room_path_with_probabilities(
    rng: &mut Rng,
    area: Area,
    level: u32,
    flags: &mut BTreeSet<String>,
    source_classic: bool,
    probabilities: SourceProbabilities,
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
            // Store Classic's roomPath[x, y] assignments in row-major [y][x]
            // order; keep this authored axis conversion explicit.
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
    let sacrifice_roll =
        source_classic && source_probability_roll(rng, probabilities.sacrifice_pit);
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
                let snake_roll =
                    source_classic && source_probability_roll(rng, probabilities.snake_pit);
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
        let alien_roll = source_classic && source_probability_roll(rng, probabilities.alien);
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
            let yeti_roll = source_classic && source_probability_roll(rng, probabilities.yeti_lair);
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
    /// Authored ending rooms retain this run global after leaving City of Gold.
    pub has_city_of_gold: bool,
}

/// Run the Create body for the Temple that oFinalBoss may create when it
/// opens the exit. Unlike rOlmec's authored Temple instances, this birth
/// occurs during gameplay and consumes the current room RNG immediately.
pub(crate) fn create_runtime_boss_temple(game: &mut crate::model::Game, x: usize, y: usize) {
    let city_of_gold = game.world.name == "City of Gold"
        || game.flags.contains("cityOfGold")
        || game.flags.contains("city_of_gold");
    let has_spectacles = game.player.has("Spectacles");
    special::create_source_runtime_boss_temple(
        &mut game.world,
        &mut game.rng,
        x,
        y,
        city_of_gold,
        has_spectacles,
    );
}

/// Persistent campaign input/output for the versioned Jungle producer.
/// The signed range is the independently proved source_rand domain, not a
/// source global clamp. Historical generation never reads or writes it.
pub(crate) const MAX_JUNGLE_PRIMARY_RECEIVERS: i32 = 22_812;
pub(crate) const MIN_SUPPORTED_SOURCE_CHANCE: i32 = -(1 << 21) + MAX_JUNGLE_PRIMARY_RECEIVERS;
pub(crate) const MIN_SUPPORTED_MARKET_CHANCE: i32 = MIN_SUPPORTED_SOURCE_CHANCE;

#[derive(Debug, Clone, Copy)]
pub(crate) struct GenerationState {
    pub revision: crate::GenerationRevision,
    pub market_chance: i32,
    /// `goldChance` survives room transitions; Level 14 decrements it for
    /// every failed ordinary Gold Entrance candidate.
    pub gold_chance: i32,
    pub made_market_entrance: bool,
    pub probabilities: SourceProbabilities,
}
impl GenerationState {
    pub(crate) fn fresh(rng: &Rng) -> Self {
        Self {
            revision: match rng.algorithm() {
                crate::RngAlgorithm::LegacySplitMix64V1 => crate::GenerationRevision::HistoricalV1,
                crate::RngAlgorithm::Gm8Lcg32V1 => crate::GenerationRevision::SourceClassicV2,
            },
            market_chance: 160,
            gold_chance: 160,
            made_market_entrance: false,
            probabilities: SourceProbabilities::default(),
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
    /// `scrInitLevel` publishes its selected cemetery roll to the live global
    /// before gameplay Create callbacks run. Keep it alongside the room so
    /// Game can project that runtime state after generation returns.
    pub cemetery_selected: bool,
    /// `scrLevelGen` keeps the sacrifice-pit latch after City of Gold rewrites
    /// a roomPath cell. Project it even when that override changes the shaft.
    pub sacrifice_pit_selected: bool,
}

#[cfg(test)]
fn select_dark_level(
    rng: &mut Rng,
    level: u32,
    area: Area,
    flags: &BTreeSet<String>,
) -> (bool, bool) {
    select_dark_level_with_probability(rng, level, area, flags, 12)
}

fn select_dark_level_with_probability(
    rng: &mut Rng,
    level: u32,
    area: Area,
    flags: &BTreeSet<String>,
    probability: i32,
) -> (bool, bool) {
    // scrInitLevel170: GM8 evaluates every Boolean operand, including this
    // rand when any earlier eligibility guard is false. Stock probability is12.
    let roll_won = source_probability_roll(rng, probability);
    let selected = !has(flags, "had_dark_level")
        && !has(flags, "no_dark_level")
        && level != 0
        && level != 1
        && area != Area::Ice
        && level != 16
        && roll_won;
    // Source177 clears darkLevel, retaining hadDarkLevel from a winning roll.
    // Explicit dark is a standalone fixture option, applied after the draw.
    let visible = !source_black_market_active(flags, area) && (has(flags, "dark") || selected);
    (visible, selected)
}

#[cfg(test)]
fn select_cemetery(rng: &mut Rng, area: Area, forced: bool, source_classic: bool) -> bool {
    select_cemetery_with_probability(rng, area, forced, source_classic, 10)
}

fn select_cemetery_with_probability(
    rng: &mut Rng,
    area: Area,
    forced: bool,
    source_classic: bool,
    probability: i32,
) -> bool {
    // scrInitLevel149 reaches this rand for every area. Explicit fixture flags
    // do not suppress that source call; HistoricalV1 retains its old tape.
    let roll = source_classic && source_probability_roll(rng, probability);
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
    generate_campaign_result_for_route(rng, level, flags, context, state, true)
}

/// Generate a campaign room reached either from an ordinary source transition
/// or directly from a hub shortcut. Classic only rolls for the Jungle Lake in
/// `oTransition`; shortcuts enter `rLevel`/`scrInitLevel` directly.
pub(crate) fn generate_campaign_result_for_route(
    rng: &mut Rng,
    level: u32,
    flags: &BTreeSet<String>,
    context: GenerationContext,
    state: &mut GenerationState,
    source_transition: bool,
) -> GeneratedLevel {
    let area = Area::for_level(level);
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
        if area == Area::Temple && level == 14 {
            assert!(
                (MIN_SUPPORTED_SOURCE_CHANCE..=(1 << 21)).contains(&state.gold_chance),
                "goldChance history is outside the proved campaign domain with one-room draw headroom"
            );
        }
        state.probabilities.validate();
    }
    let source_classic = state.revision == crate::GenerationRevision::SourceClassicV2;
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
        && if source_classic {
            source_transition && source_probability_roll(rng, state.probabilities.lake)
        } else {
            rng.chance(1, 10)
        }
    {
        selected.insert("lake".into());
    }
    let mut world = World::new(42, 34, area, level);
    world.source_create_projection = source_classic;
    if special::generate_special(&mut world, rng, flags) {
        if source_classic {
            // The special room replaces the initial World. Restore its
            // generation revision so any generated children use the same
            // source Create projection as ordinary campaign rooms.
            world.source_create_projection = true;
            if has(flags, "tutorial") {
                return initialize_source_tutorial(world, rng, &selected, context, state);
            }
            for entity in &mut world.entities {
                crate::world_events::initialize_source_generated_create(entity);
            }
            initialize_generated_item_depths(&mut world, context.has_spectacles);
            initialize_generated_source_exit_creates(&mut world);
        }
        world.fluid_dirty = false;
        return GeneratedLevel {
            world,
            dark_roll_selected: false,
            cemetery_selected: false,
            sacrifice_pit_selected: false,
        };
    }
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
        world.source_create_projection = true;
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
    let path = room_path_with_probabilities(
        rng,
        area,
        level,
        &mut selected,
        source_classic,
        state.probabilities,
    );
    if source_classic && (1..=16).contains(&level) {
        // Preserve the selected grid before room Create/population callbacks.
        // Source isInShop reads these markers throughout gameplay, even after
        // a keeper moves, deactivates, or is destroyed. Synthetic level0 and
        // authored tutorial/custom/hub contexts do not establish this grid.
        world.source_room_path = Some(path.rooms);
    }
    if select_cemetery_with_probability(
        rng,
        area,
        has(&selected, "cemetery"),
        source_classic,
        state.probabilities.cemetery,
    ) {
        selected.insert("cemetery".into());
    }
    let context = GenerationContext {
        has_cemetery: has(&selected, "cemetery"),
        has_city_of_gold: has(&selected, "city_of_gold"),
        ..context
    };
    if !source_classic {
        // Explicit HistoricalV1 retains its old construction order and draws.
        world = World::new(42, height, area, level);
    }
    let black_market = if source_classic {
        source_black_market_active(&selected, area)
    } else {
        has(&selected, "black_market")
    };
    world.name = if black_market {
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
    let room_order = if source_classic && area == Area::Boss {
        // rOlmec.xml's static oRoom instance order is not spatial row-major:
        // its two interior top rooms are declared after rows 1 and 2. Since
        // each scrRoomGen5 callback draws and creates synchronously, dispatch
        // them in resource order before continuing the shared RNG stream.
        vec![
            (0, 0),
            (0, 3),
            (1, 0),
            (1, 1),
            (1, 2),
            (1, 3),
            (2, 0),
            (2, 1),
            (2, 2),
            (2, 3),
            (0, 1),
            (0, 2),
            (3, 0),
            (3, 1),
            (3, 2),
            (3, 3),
        ]
    } else {
        (0..if has(&selected, "lake") { 5 } else { 4 })
            .flat_map(|row| (0..4).map(move |col| (row, col)))
            .collect()
    };
    for (row, col) in room_order {
        #[cfg(test)]
        source_room_dispatch_tests::observe(row, col);
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
            tiles::stamp_at_source_context(
                &mut world,
                rng,
                &expanded,
                1 + col * 10,
                37,
                false,
                false,
                shop,
                features.damsel,
                context,
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
    let dark_roll_selected = if state.revision == crate::GenerationRevision::SourceClassicV2 {
        let (visible, selected_roll) = select_dark_level_with_probability(
            rng,
            level,
            area,
            &selected,
            state.probabilities.dark_level,
        );
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
    if source_classic {
        initialize_generated_item_depths(&mut world, context.has_spectacles);
        initialize_generated_source_exit_creates(&mut world);
    }
    GeneratedLevel {
        world,
        dark_roll_selected,
        cemetery_selected: has(&selected, "cemetery"),
        sacrifice_pit_selected: has(&selected, "sacrifice_pit"),
    }
}

/// Finish the same `scrInitLevel` phases that `rTutorial` reaches in Classic.
/// It creates its authored room first, then the controller boundaries, path
/// and global rolls, SetupWalls(528), optional detail tiles, and cleanup. It
/// deliberately skips generated-room population just as `scrEntityGen` does.
fn initialize_source_tutorial(
    mut world: World,
    rng: &mut Rng,
    flags: &BTreeSet<String>,
    context: GenerationContext,
    state: &GenerationState,
) -> GeneratedLevel {
    for entity in &mut world.entities {
        crate::world_events::initialize_source_generated_create(entity);
    }
    special::create_tutorial_source_terrain(&mut world, rng, context);
    source_tutorial_boundaries(&mut world, rng);

    let mut selected = flags.clone();
    let _path = room_path_with_probabilities(
        rng,
        Area::Mines,
        0,
        &mut selected,
        true,
        state.probabilities,
    );
    // scrLevelGen evaluates rand(1,currLevel) before its currLevel > 1 guard;
    // tutorial currLevel is zero, so this is a source zero-width draw only.
    let _ = rng
        .source_rand(1, 0)
        .expect("tutorial shop predicate uses the canonical zero-width source draw");

    if select_cemetery_with_probability(
        rng,
        Area::Mines,
        has(&selected, "cemetery"),
        true,
        state.probabilities.cemetery,
    ) {
        selected.insert("cemetery".into());
    }
    let (dark, dark_roll_selected) = select_dark_level_with_probability(
        rng,
        0,
        Area::Mines,
        &selected,
        state.probabilities.dark_level,
    );
    debug_assert!(
        !dark,
        "the source tutorial suppresses dark levels at level 0"
    );
    population::initialize_source_schedules(rng, 0, &selected);

    let graphics_high = !has(&selected, "low_graphics");
    terrain::setup_generated_with_revision(
        &mut world,
        rng,
        graphics_high,
        state.revision,
        has(&selected, "cemetery"),
    );
    if graphics_high {
        background_extras(&mut world, rng, true);
    }
    init_cleanup::cleanup(&mut world, state.revision);
    terrain::finalize_generated(&mut world, state.revision);
    initialize_generated_item_depths(&mut world, context.has_spectacles);
    initialize_generated_source_exit_creates(&mut world);
    world.fluid_dirty = false;
    GeneratedLevel {
        world,
        dark_roll_selected,
        cemetery_selected: has(&selected, "cemetery"),
        sacrifice_pit_selected: false,
    }
}

/// Generated worlds run local enemy/trap Creates as their producers execute,
/// but are assembled outside `Game::spawn`, so inherited item Create depth
/// must be projected from the same room context before the first Draw.
fn initialize_generated_item_depths(world: &mut World, has_spectacles: bool) {
    for entity in &mut world.entities {
        crate::items::initialize_inherited_depth(entity, has_spectacles);
    }
}

/// Generated worlds bypass `Game::spawn`, so project the source portal Create
/// fields before the completed room becomes playable. Existing producer
/// overrides (for example the Moai Exit type) remain authoritative.
fn initialize_generated_source_exit_creates(world: &mut World) {
    for entity in &mut world.entities {
        crate::creation::initialize_source_exit_create(entity);
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
    if revision != crate::GenerationRevision::SourceClassicV2 {
        return false;
    }
    let Some(bricks_id) = world
        .entities
        .iter()
        .find(|entity| entity.alive && entity.object == "oBricks")
        .map(|entity| entity.id)
    else {
        return false;
    };
    terrain::setup_authored_walls(world, rng, graphics_high, city, cemetery);
    // oBricks.Create immediately calls instance_destroy() after SetupWalls.
    world.entity_mut(bricks_id).unwrap().alive = false;
    true
}

/// Run one source terrain parent Create at its authored room-instance slot.
pub(crate) fn initialize_source_room_terrain(
    world: &mut World,
    rng: &mut Rng,
    x: usize,
    y: usize,
    object: &str,
    context: GenerationContext,
) {
    terrain::create_source_room_terrain_for_object(world, rng, x, y, object, context);
}

/// Run one authored room's `oLava.Create` in its source instance-order slot.
/// The later SetupWalls lava pass sees the initialized marker and does not
/// consume the Create RNG a second time.
pub(crate) fn initialize_authored_lava_create(world: &mut World, rng: &mut Rng, id: u64) {
    terrain::initialize_authored_lava_create(world, rng, id);
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

/// `rTutorial` already contains authored terrain under most of its controller
/// boundary positions. Classic creates a second invincible object there, so
/// retain those room instances and add the boundary as a separate entity.
fn source_tutorial_boundaries(world: &mut World, rng: &mut Rng) {
    for i in 0..=42 {
        for j in 0..=33 {
            if i == 0 || i == 41 || j == 0 || j >= 33 {
                terrain::create_source_boundary_instance(world, rng, i, j, "oBrick", false, true);
            }
        }
    }
}

#[cfg(test)]
#[path = "generation/source_boundary_tests.rs"]
mod source_boundary_tests;

#[cfg(test)]
#[path = "generation/source_room_dispatch_tests.rs"]
mod source_room_dispatch_tests;

#[cfg(test)]
#[path = "generation/mines_source_tape_tests.rs"]
mod mines_source_tape_tests;

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
    tutorial_with_context(&mut Rng::new(0), GenerationContext::default(), true)
}

pub(crate) fn tutorial_with_context(
    rng: &mut Rng,
    context: GenerationContext,
    graphics_high: bool,
) -> World {
    let mut flags = BTreeSet::new();
    flags.insert("tutorial".into());
    if !graphics_high {
        flags.insert("low_graphics".into());
    }
    generate_with_context(rng, 0, &flags, context)
}

/// Run custom-room terrain Create events without generated-room SetupWalls.
/// scrCreateTileObj subsequently forces base sprites, while buried contents
/// created before those assignments remain present.
pub fn initialize_custom_terrain(world: &mut World, rng: &mut Rng) {
    terrain::initialize_custom(world, rng);
}

pub(crate) fn initialize_custom_terrain_with_context(
    world: &mut World,
    rng: &mut Rng,
    context: GenerationContext,
) {
    terrain::initialize_custom_with_context(world, rng, context);
}

/// Finish the `scrInitLevel` work that still runs after Classic loads a custom
/// room. The loader has already created its authored instances row-major;
/// `oGame.Create` then creates boundaries, builds `roomPath`, runs the
/// post-path selectors and SetupWalls, places high-graphics details, and
/// performs the ordinary room cleanup.
pub(crate) fn initialize_custom_source_setup(game: &mut crate::Game) {
    if game.generation_revision != crate::GenerationRevision::SourceClassicV2 {
        return;
    }

    // oLoadLevel Create resets these source globals on every custom-room
    // entry; scrInitLevel then derives levelType zero and clears dark history
    // for the reset currLevel one before building boundaries and roomPath.
    game.data.insert("currLevel".into(), 1.0);
    game.data.insert("levelType".into(), 0.0);
    reset_dark_history(&mut game.flags, 1);

    // `Level::to_world` uses hard blocks as a temporary closed perimeter.
    // Classic instead creates invincible terrain objects in scrInitLevel;
    // remove only that adapter perimeter before replaying the source births.
    for y in 0..game.world.height {
        for x in 0..game.world.width {
            if (x == 0 || y == 0 || x + 1 == game.world.width || y + 1 == game.world.height)
                && game.world.tiles[y * game.world.width + x].object == "oHardBlock"
            {
                game.world.set_tile(x, y, "");
            }
        }
    }
    source_boundaries(&mut game.world, &mut game.rng, false, false);

    let probabilities = SourceProbabilities::from_globals(&game.data);
    let mut source_flags = game.flags.clone();
    let path = room_path_with_probabilities(
        &mut game.rng,
        game.world.area,
        game.world.level,
        &mut source_flags,
        true,
        probabilities,
    );
    game.flags = source_flags;
    game.world.source_room_path = Some(path.rooms);

    initialize_custom_source_predicates(game, probabilities);

    let graphics_high = !has(&game.flags, "low_graphics");
    terrain::setup_custom_source(
        &mut game.world,
        &mut game.rng,
        graphics_high,
        has(&game.flags, "cemetery"),
    );
    if graphics_high {
        background_extras(&mut game.world, &mut game.rng, true);
    }
    init_cleanup::cleanup(&mut game.world, game.generation_revision);
    terrain::finalize_generated(&mut game.world, game.generation_revision);
    game.world.fluid_dirty = false;
}

/// Apply the selectors that `scrInitLevel` reaches between `scrLevelGen` and
/// `scrSetupWalls` for rLoadLevel's custom rooms. Keep these draws in one
/// production helper so the level-one tape can be audited independently.
fn initialize_custom_source_predicates(
    game: &mut crate::Game,
    probabilities: SourceProbabilities,
) -> bool {
    game.flags
        .retain(|flag| !matches!(flag.as_str(), "cemetery" | "cemetary"));
    if select_cemetery_with_probability(
        &mut game.rng,
        game.world.area,
        false,
        true,
        probabilities.cemetery,
    ) {
        game.flags.insert("cemetery".into());
    }

    game.flags
        .retain(|flag| !matches!(flag.as_str(), "dark" | "darkLevel"));
    let (dark, dark_roll_selected) = select_dark_level_with_probability(
        &mut game.rng,
        1,
        game.world.area,
        &game.flags,
        probabilities.dark_level,
    );
    game.world.dark = dark;
    if dark_roll_selected {
        game.flags.insert("had_dark_level".into());
    }

    game.flags.retain(|flag| {
        !matches!(
            flag.as_str(),
            "gen_udjat_eye" | "genUdjatEye" | "gen_market_entrance" | "genMarketEntrance"
        )
    });
    population::initialize_source_schedules(&mut game.rng, 1, &game.flags);
    dark
}

#[cfg(test)]
fn initialize_custom_source_predicates_for_test(
    game: &mut crate::Game,
    probabilities: SourceProbabilities,
) -> bool {
    initialize_custom_source_predicates(game, probabilities)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_source_init_keeps_all_level_one_selector_draws() {
        let mut game = crate::Game::shell(0, crate::RulesPreset::Classic);
        game.world = World::new(42, 34, Area::Mines, 1);
        let probabilities = SourceProbabilities::default();
        game.flags.extend([
            "cemetery".into(),
            "cemetary".into(),
            "dark".into(),
            "darkLevel".into(),
            "gen_udjat_eye".into(),
            "genUdjatEye".into(),
            "gen_market_entrance".into(),
            "genMarketEntrance".into(),
            "had_dark_level".into(),
        ]);
        reset_dark_history(&mut game.flags, 1);

        let mut expected = game.rng.clone();
        for denominator in [probabilities.cemetery, probabilities.dark_level, 3, 2, 3, 2] {
            expected.source_rand(1, denominator).unwrap();
        }
        assert!(!super::initialize_custom_source_predicates_for_test(
            &mut game,
            probabilities
        ));
        assert_eq!(game.rng.state_bits(), expected.state_bits());
        assert!(!game.world.dark);
        assert!(!game.flags.contains("cemetery"));
        assert!(!game.flags.contains("cemetary"));
        assert!(!game.flags.contains("dark"));
        assert!(!game.flags.contains("darkLevel"));
        assert!(!game.flags.contains("had_dark_level"));
        assert!(!game.flags.contains("gen_udjat_eye"));
        assert!(!game.flags.contains("genUdjatEye"));
        assert!(!game.flags.contains("gen_market_entrance"));
        assert!(!game.flags.contains("genMarketEntrance"));

        let mut made = crate::Game::shell(0, crate::RulesPreset::Classic);
        made.world = World::new(42, 34, Area::Mines, 1);
        made.flags.insert("made_udjat_eye".into());
        made.flags.insert("made_market_entrance".into());
        made.flags.insert("gen_udjat_eye".into());
        made.flags.insert("genUdjatEye".into());
        made.flags.insert("gen_market_entrance".into());
        made.flags.insert("genMarketEntrance".into());
        reset_dark_history(&mut made.flags, 1);
        let mut expected = made.rng.clone();
        for denominator in [probabilities.cemetery, probabilities.dark_level] {
            expected.source_rand(1, denominator).unwrap();
        }
        assert!(!super::initialize_custom_source_predicates_for_test(
            &mut made,
            probabilities
        ));
        assert_eq!(made.rng.state_bits(), expected.state_bits());
        assert!(made.flags.contains("made_udjat_eye"));
        assert!(made.flags.contains("made_market_entrance"));
        assert!(!made.flags.contains("gen_udjat_eye"));
        assert!(!made.flags.contains("genUdjatEye"));
        assert!(!made.flags.contains("gen_market_entrance"));
        assert!(!made.flags.contains("genMarketEntrance"));
        assert!(!made.world.dark);
    }

    #[test]
    fn authored_setup_walls_destroys_its_source_controller_after_setup() {
        let mut world = World::new(42, 34, Area::Mines, 1);
        let bricks = world.spawn("oBricks", Vec2::default());
        let mut rng = Rng::new(7);

        assert!(super::initialize_authored_setup_walls(
            &mut world,
            &mut rng,
            crate::GenerationRevision::SourceClassicV2,
            true,
            false,
            false,
        ));
        assert!(
            !world
                .entities
                .iter()
                .find(|entity| entity.id == bricks)
                .unwrap()
                .alive
        );
        assert!(!super::initialize_authored_setup_walls(
            &mut world,
            &mut rng,
            crate::GenerationRevision::SourceClassicV2,
            true,
            false,
            false,
        ));
    }

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
            assert_eq!(
                path.rooms,
                [
                    [2, 4, 4, 2],
                    [2, 4, 4, 2],
                    [2, 4, 5, 4],
                    [3, 1, 1, 3],
                    [0; 4],
                ],
                "Classic roomPath[x, y] must be stored row-major as [y][x]"
            );
            assert_eq!(actual.next_u64(), expected.next_u64(), "seed{seed}");
        }
    }

    #[test]
    fn source_probability_globals_drive_each_optional_campaign_branch() {
        let globals = BTreeMap::from([
            ("probDarkLevel".into(), 1.0),
            ("probSnakePit".into(), 1.0),
            ("probCemetary".into(), 1.0),
            ("probLake".into(), 1.0),
            ("probYetiLair".into(), 1.0),
            ("probAlien".into(), 1.0),
            ("probSacPit".into(), 1.0),
        ]);
        let probabilities = SourceProbabilities::from_globals(&globals);
        assert_eq!(
            probabilities,
            SourceProbabilities {
                dark_level: 1,
                snake_pit: 1,
                cemetery: 1,
                lake: 1,
                yeti_lair: 1,
                alien: 1,
                sacrifice_pit: 1,
            }
        );
        assert_eq!(
            SourceProbabilities::from_globals(&BTreeMap::new()),
            SourceProbabilities::default()
        );

        let (dark, dark_roll_selected) = select_dark_level_with_probability(
            &mut Rng::new(0),
            2,
            Area::Mines,
            &BTreeSet::new(),
            probabilities.dark_level,
        );
        assert!(dark && dark_roll_selected);
        assert!(select_cemetery_with_probability(
            &mut Rng::new(0),
            Area::Jungle,
            false,
            true,
            probabilities.cemetery,
        ));

        let mut lake_state = GenerationState::fresh(&Rng::new(0));
        lake_state.probabilities = probabilities;
        let lake = generate_campaign_result(
            &mut Rng::new(0),
            7,
            &BTreeSet::from(["made_black_market".into()]),
            GenerationContext::default(),
            &mut lake_state,
        );
        assert_eq!(lake.world.name, "Lake");

        let mut found_snake_pit = false;
        for seed in 0..128 {
            let mut flags = BTreeSet::new();
            room_path_with_probabilities(
                &mut Rng::new(seed),
                Area::Mines,
                2,
                &mut flags,
                true,
                probabilities,
            );
            if flags.contains("snake_pit") {
                found_snake_pit = true;
                break;
            }
        }
        assert!(
            found_snake_pit,
            "probSnakePit=1 should select an available pit"
        );

        let mut temple_flags = BTreeSet::new();
        room_path_with_probabilities(
            &mut Rng::new(0),
            Area::Temple,
            13,
            &mut temple_flags,
            true,
            probabilities,
        );
        assert!(temple_flags.contains("sacrifice_pit"));

        let made_moai = BTreeSet::from(["made_moai".into()]);
        let mut alien_flags = made_moai.clone();
        room_path_with_probabilities(
            &mut Rng::new(0),
            Area::Ice,
            9,
            &mut alien_flags,
            true,
            probabilities,
        );
        assert!(alien_flags.contains("alien_ship"));

        let mut yeti_probabilities = probabilities;
        yeti_probabilities.alien = 1 << 21;
        let found_yeti = (0..128).any(|seed| {
            let mut flags = made_moai.clone();
            room_path_with_probabilities(
                &mut Rng::new(seed),
                Area::Ice,
                9,
                &mut flags,
                true,
                yeti_probabilities,
            );
            flags.contains("yeti_lair")
        });
        assert!(
            found_yeti,
            "probYetiLair=1 should select when Alien Ship does not"
        );
    }

    #[test]
    fn invalid_source_probability_is_rejected_before_generation_draws() {
        let mut rng = Rng::new(42);
        let before = rng.state_bits();
        let mut state = GenerationState::fresh(&rng);
        state.probabilities.dark_level = (1 << 21) + 1;

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            generate_campaign_result(
                &mut rng,
                2,
                &BTreeSet::new(),
                GenerationContext::default(),
                &mut state,
            )
        }));

        assert!(result.is_err());
        assert_eq!(rng.state_bits(), before);
    }

    #[test]
    fn pending_black_market_request_only_activates_in_the_jungle() {
        let mut game = crate::Game::new_with_algorithm(
            4,
            crate::RulesPreset::Classic,
            crate::RngAlgorithm::Gm8Lcg32V1,
        )
        .unwrap();
        game.flags.insert("genBlackMarket".into());
        // The transition Draw clears noDarkLevel once the completed room
        // reaches 21 source seconds; this fixture needs that source state in
        // order to exercise the level-two darkness branch.
        game.data.insert("xtime".into(), 21_000.0);
        game.data.insert("next_level".into(), 2.0);

        game.next_level();

        assert_eq!(game.world.area, Area::Mines);
        assert_eq!(game.world.name, "Mines");
        assert!(
            game.world.dark,
            "the pending request must not suppress Mines darkness"
        );
        assert!(
            !game.flags.contains("made_black_market"),
            "a non-Jungle level must not consume the run-persistent request"
        );

        game.data.insert("next_level".into(), 5.0);
        game.next_level();
        assert_eq!(game.world.area, Area::Jungle);
        assert_eq!(game.world.name, "Black Market");
        assert!(game.flags.contains("made_black_market"));
    }

    #[test]
    fn source_generated_chests_have_inherited_depth_before_the_first_draw() {
        for (has_spectacles, expected_depth) in [(false, 101.0), (true, 51.0)] {
            let mut rng = Rng::new(0);
            let mut state = GenerationState::fresh(&rng);
            let generated = generate_campaign_result(
                &mut rng,
                1,
                &BTreeSet::new(),
                GenerationContext {
                    has_spectacles,
                    ..GenerationContext::default()
                },
                &mut state,
            );
            let chest = generated
                .world
                .entities
                .iter()
                .find(|entity| entity.alive && entity.object == "oChest")
                .expect("seed-zero Mines level has a generated chest");
            assert_eq!(chest.data.get("depth"), Some(&expected_depth));
        }
    }

    #[test]
    fn source_generated_portals_have_their_classic_create_fields() {
        let mut rng = Rng::new(0);
        let mut state = GenerationState::fresh(&rng);
        let generated = generate_campaign_result(
            &mut rng,
            1,
            &BTreeSet::new(),
            GenerationContext::default(),
            &mut state,
        );
        let portals: Vec<_> = generated
            .world
            .entities
            .iter()
            .filter(|entity| {
                entity.object == "oEntrance" || crate::catalog::is_a(&entity.object, "oExit")
            })
            .collect();

        assert!(portals.iter().any(|entity| entity.object == "oEntrance"));
        assert!(portals.iter().any(|entity| entity.object == "oExit"));
        for portal in portals {
            assert_eq!(portal.text.get("type").map(String::as_str), Some("Exit"));
            assert_eq!(portal.text.get("leadsTo").map(String::as_str), Some(""));
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
        let mut persisted_rng = Rng::new(0);
        let persisted = generate_campaign_result(
            &mut persisted_rng,
            7,
            &flags,
            GenerationContext::default(),
            &mut GenerationState::fresh(&Rng::new(0)),
        );
        assert_eq!(persisted.world.name, "Jungle");

        let source_named_flags =
            BTreeSet::from(["genBlackMarket".into(), "madeBlackMarket".into()]);
        let mut source_named_rng = Rng::new(0);
        let source_named = generate_campaign_result(
            &mut source_named_rng,
            7,
            &source_named_flags,
            GenerationContext::default(),
            &mut GenerationState::fresh(&Rng::new(0)),
        );
        assert_eq!(
            source_named.world.name, "Jungle",
            "Classic's madeBlackMarket flag must suppress another forced Market layout"
        );
        assert_eq!(source_named_rng.state_bits(), persisted_rng.state_bits());
    }

    #[test]
    fn natural_level_nine_moai_keeps_its_authored_wall_decorations() {
        let mut game = crate::Game::shell(4, crate::RulesPreset::Classic);
        game.world.level = 8;
        game.flags.insert("no_dark_level".into());
        game.next_level();

        assert_eq!(game.world.level, 9);
        assert!(
            game.world
                .entities
                .iter()
                .any(|entity| entity.object == "oMoai")
        );
        let inside = game
            .world
            .entities
            .iter()
            .find(|entity| entity.object == "oMoaiInside")
            .expect("natural level-nine Moai room");
        let decorations: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|entity| {
                entity.object == "oGenerationDecoration"
                    && entity.sprite == "bgAlienShip3"
                    && entity.value("depth") == 150.0
            })
            .collect();
        assert_eq!(decorations.len(), 2);
        for (decoration, y_offset) in decorations.iter().zip([0.0, 16.0]) {
            assert_eq!(
                decoration.pos,
                Vec2::new(inside.pos.x, inside.pos.y + y_offset)
            );
            assert_eq!(decoration.value("source_x"), 0.0);
            assert_eq!(decoration.value("source_y"), 0.0);
            assert_eq!(decoration.value("source_w"), 16.0);
            assert_eq!(decoration.value("source_h"), 16.0);
        }
        let thin_ice = game
            .world
            .entities
            .iter()
            .find(|entity| entity.object == "oThinIce" && entity.pos == decorations[1].pos)
            .expect("source births the second decoration before the ThinIce at that origin");
        let moai_exit = game
            .world
            .entities
            .iter()
            .find(|entity| {
                entity.object == "oExit"
                    && entity
                        .text
                        .get("type")
                        .is_some_and(|kind| kind == "Moai Exit")
            })
            .expect("Moai room's authored exit");
        let crown = game
            .world
            .entities
            .iter()
            .find(|entity| entity.object == "oCrown")
            .expect("Moai room crown");
        assert!(inside.creation_serial < decorations[0].creation_serial);
        assert!(decorations[0].creation_serial < decorations[1].creation_serial);
        assert!(decorations[1].creation_serial < thin_ice.creation_serial);
        assert!(thin_ice.creation_serial < moai_exit.creation_serial);
        assert!(moai_exit.creation_serial < crown.creation_serial);
        assert_eq!(moai_exit.text.get("leadsTo").map(String::as_str), Some(""));
        assert_eq!(game.rng.state_bits(), 2_518_181_788);
    }

    #[test]
    fn natural_level_nine_alien_front_uses_authored_background_depths() {
        let mut game = crate::Game::shell(5, crate::RulesPreset::Classic);
        game.world.level = 8;
        game.flags.insert("made_moai".into());
        game.flags.insert("no_dark_level".into());
        game.next_level();

        assert_eq!(game.world.level, 9);
        assert_eq!(game.world.name, "Alien Ship");
        let mut front = None;
        'rows: for y in 0..game.world.height as i32 - 2 {
            for x in 0..game.world.width as i32 {
                let sprite = |y| {
                    game.world
                        .tile_sprite_metadata(x, y)
                        .and_then(|entity| entity.text.get("terrain_sprite"))
                        .map(String::as_str)
                };
                if sprite(y) == Some("sAlienFront")
                    && sprite(y + 1) == Some("sAlienFront")
                    && sprite(y + 2) == Some("sAlienFront2")
                {
                    front = Some((x, y));
                    break 'rows;
                }
            }
        }
        let (x, y) = front.expect("natural Alien Ship has a three-tile front");
        let depth_at = |y| {
            let origin = Vec2::new(f64::from(x * 16), f64::from(y * 16));
            game.world
                .entities
                .iter()
                .find(|entity| {
                    entity.object == "oGenerationDecoration"
                        && entity.sprite == "bgAlienShip3"
                        && entity.pos == origin
                })
                .map(|entity| entity.value("depth"))
        };
        assert_eq!(
            [depth_at(y), depth_at(y + 1), depth_at(y + 2)],
            [Some(9005.0), Some(160.0), Some(160.0)]
        );
        for row in [y, y + 1, y + 2] {
            let origin = Vec2::new(f64::from(x * 16), f64::from(row * 16));
            let decoration = game
                .world
                .entities
                .iter()
                .find(|entity| {
                    entity.object == "oGenerationDecoration"
                        && entity.sprite == "bgAlienShip3"
                        && entity.pos == origin
                })
                .unwrap();
            assert_eq!(decoration.value("source_x"), 0.0);
            assert_eq!(decoration.value("source_y"), 0.0);
            assert_eq!(decoration.value("source_w"), 16.0);
            assert_eq!(decoration.value("source_h"), 16.0);
        }
        // Successful Ice spring rolls assign sDark before their guards, so
        // they now spawn the trap and skip the source else-if Yeti draw.
        assert_eq!(game.rng.state_bits(), 1_532_323_483);
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

#[cfg(test)]
#[path = "generation/obstacle_expansion_tests.rs"]
mod obstacle_expansion_tests;
