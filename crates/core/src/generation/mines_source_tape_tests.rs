//! A bounded, source-derived Mines room tape. Expected values come from the
//! literal GM8 LCG and the referenced GML/Create statements, not a cloned
//! production `Rng`.
use super::*;

const MINES_BOUNDARY_CHECKPOINT: u32 = 16_145_268;
const MINES_ROUTE_CHECKPOINT: u32 = 2_549_583_129;
const MINES_FIRST_ROOM_CHECKPOINT: u32 = 3_445_532_030;
const MINES_AFTER_SELECTOR: u32 = 2_195_217_748;
const MINES_AFTER_FIRST_ROOM: u32 = 110_709_807;
const MINES_SIDE_ROOM_6: &str =
    "11111111110221111220002111120000022220000002222000002111120002211112201111111111";

#[derive(Clone, Copy)]
struct LiteralLcg {
    state: u32,
    draws: usize,
}

impl LiteralLcg {
    fn new(state: u32) -> Self {
        Self { state, draws: 0 }
    }

    fn rand(&mut self, low: u32, high: u32) -> u32 {
        self.state = self.state.wrapping_mul(0x0808_8405).wrapping_add(1);
        self.draws += 1;
        low + ((u64::from(self.state) * u64::from(high - low + 1)) >> 32) as u32
    }
}

#[derive(Debug, PartialEq, Eq)]
struct TerrainBirth {
    x: usize,
    y: usize,
    object: String,
}

fn literal_mines_l1_path(source: &mut LiteralLcg) -> (Path, bool) {
    // scrLevelGen:26,34. The candidate `n` is consumed before the source's
    // eager Temple-only sacrifice condition.
    let start = source.rand(0, 3) as usize;
    let _candidate_n = source.rand(0, 3) as usize;
    let mut room_path = [[0_u8; 5]; 4]; // source indexes [x][y]
    let (mut x, mut y) = (start, 0_usize);
    let mut end = start;
    room_path[x][y] = 1;

    // GM8 computes both Boolean operands, so this draw occurs in Mines too.
    let _sacrifice_roll = source.rand(1, 8);

    while y < 4 {
        let mut draw = if x == 0 {
            source.rand(3, 5)
        } else if x == 3 {
            source.rand(5, 7)
        } else {
            source.rand(1, 5)
        };
        let (old_x, old_y) = (x, y);

        // Keep the Original's unbraced nested if/else binding from lines
        // 97-112; the inner fallback can turn a blocked move into descent.
        if draw < 3 || draw > 5 {
            if x > 0 {
                if room_path[x - 1][y] == 0 {
                    x -= 1;
                } else if x < 3 {
                    if room_path[x + 1][y] == 0 {
                        x += 1;
                    } else {
                        draw = 5;
                    }
                }
            }
        } else if draw == 3 || draw == 4 {
            if x < 3 {
                if room_path[x + 1][y] == 0 {
                    x += 1;
                } else if x > 0 {
                    if room_path[x - 1][y] == 0 {
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
                room_path[old_x][old_y] = 2;
                room_path[x][y] = 3;
            } else {
                end = x;
            }
        } else {
            room_path[x][y] = 1;
        }
    }

    // scrLevelGen:149 evaluates the final rand for every visited coordinate,
    // even if an earlier empty-room guard is false.
    let mut snake_pit = false;
    'snake: for row in 0..2 {
        for col in 0..4 {
            let roll = source.rand(1, 8);
            if room_path[col][row] == 0
                && room_path[col][row + 1] == 0
                && room_path[col][row + 2] == 0
                && roll == 1
            {
                room_path[col][row] = 7;
                room_path[col][row + 1] = 8;
                if row == 0 && room_path[col][3] == 0 {
                    room_path[col][2] = 8;
                    room_path[col][3] = 9;
                } else {
                    room_path[col][row + 2] = 9;
                }
                snake_pit = true;
                break 'snake;
            }
        }
    }

    // For Mines, both Ice alternatives still evaluate their random operands.
    source.rand(1, 10); // probAlien
    source.rand(1, 6); // probYetiLair
    source.rand(1, 1); // shop lottery, before currLevel > 1

    let rooms = std::array::from_fn(|row| std::array::from_fn(|col| room_path[col][row]));
    (Path { rooms, start, end }, snake_pit)
}

fn literal_mines_first_room(source: &mut LiteralLcg, template: &str) -> Vec<TerrainBirth> {
    let mut births = Vec::new();

    for row in 0..8 {
        for col in 0..10 {
            let symbol = template.as_bytes()[row * 10 + col];
            let mut create_ground = |source: &mut LiteralLcg| {
                if source.rand(1, 10) == 1 {
                    births.push(TerrainBirth {
                        x: col + 1,
                        y: row + 1,
                        object: "oBlock".into(),
                    });
                    return;
                }

                births.push(TerrainBirth {
                    x: col + 1,
                    y: row + 1,
                    object: "oBrick".into(),
                });

                // oBrick.events/Create.xml runs after inherited oSolid Create.
                source.rand(1, 10); // random sprite variant
                let n = source.rand(1, 100);
                if n < 30 {
                    return;
                }

                // Every selected cell is inside rLevel's precious-item
                // domain. The fixed seed misses all four Create branches;
                // if the rare item branch changes, this fixture must be
                // expanded with scrGenerateItem's own source tape.
                for width in [100, 120, 140, 1200] {
                    if source.rand(1, width) == 1 {
                        assert_ne!(width, 1200, "fixture reached scrGenerateItem");
                        break;
                    }
                }
            };

            match symbol {
                b'1' => create_ground(source),
                b'2' => {
                    if source.rand(1, 2) == 1 {
                        create_ground(source);
                    } else {
                        // The following else-if predicates remain reached.
                        source.rand(1, 3); // tile == "7"
                        source.rand(1, 4); // tile == "4"
                    }
                }
                b'0' => {
                    // Eager `and` operands in the tile-2, tile-7 and tile-4
                    // predicates consume these even for the wrong symbol.
                    source.rand(1, 2);
                    source.rand(1, 3);
                    source.rand(1, 4);
                }
                _ => panic!("unexpected symbol in fixed side-room template: {symbol}"),
            }
        }
    }

    births
}

fn mines_world_with_source_boundaries() -> World {
    let mut world = World::new(42, 34, Area::Mines, 1);
    world.name = "Mines".into();
    world.source_create_projection = true;
    // scrInitLevel's 150 source oBrick boundaries have already completed
    // Create at MINES_BOUNDARY_CHECKPOINT. The fixture needs their exact masks
    // for source collision_point queries, not their earlier random state.
    for y in 0..world.height {
        for x in 0..world.width {
            if x == 0 || x == 41 || y == 0 || y == 33 {
                world.set_tile(x, y, "oBrick");
            }
        }
    }
    world
}

#[test]
fn source_seed0_mines_route_first_room_and_create_tape_match_fixed_values() {
    let mut source = LiteralLcg::new(0);

    // scrInitLevel's Mines boundary loop creates 150 oBrick instances.
    // Each runs the two unconditional oBrick Create rolls; all are outside
    // the interior precious-item gate. This independently derives the fixed
    // pre-scrLevelGen checkpoint instead of obtaining it from production.
    for _ in 0..150 {
        source.rand(1, 10);
        source.rand(1, 100);
    }
    assert_eq!(source.state, MINES_BOUNDARY_CHECKPOINT);
    assert_eq!(source.draws, 300);

    let (expected_path, expected_snake_pit) = literal_mines_l1_path(&mut source);
    assert_eq!(expected_path.start, 2);
    assert_eq!(expected_path.end, 3);
    assert!(!expected_snake_pit);
    assert_eq!(
        expected_path.rooms,
        [
            [0, 2, 1, 0],
            [2, 3, 0, 0],
            [3, 1, 1, 2],
            [0, 0, 0, 3],
            [0; 4],
        ]
    );
    assert_eq!(source.state, MINES_ROUTE_CHECKPOINT);
    assert_eq!(source.draws, 329);

    // scrInitLevel149 consumes this draw outside Jungle as well.
    let cemetery_roll = source.rand(1, 10);
    assert_eq!(cemetery_roll, 9);
    assert_eq!(source.state, MINES_FIRST_ROOM_CHECKPOINT);
    assert_eq!(source.draws, 330);

    // The first static oRoom in rLevel.xml is (16,16), row 0 / column 0.
    // Its path is 0; scrRoomGen consumes the altar guard and room choice.
    let altar_roll = source.rand(1, 16);
    assert_eq!(altar_roll, 15);
    let source_room_number = source.rand(1, 10);
    assert_eq!(source_room_number, 6);
    assert_eq!(source.state, MINES_AFTER_SELECTOR);
    assert_eq!(source.draws, 332);

    let mut production_rng = Rng::new_canonical(MINES_BOUNDARY_CHECKPOINT);
    let mut selected_flags = BTreeSet::new();
    let production_path = room_path_with_probabilities(
        &mut production_rng,
        Area::Mines,
        1,
        &mut selected_flags,
        true,
        SourceProbabilities::default(),
    );
    assert_eq!(production_path.rooms, expected_path.rooms);
    assert_eq!((production_path.start, production_path.end), (2, 3));
    assert!(selected_flags.is_empty());
    assert_eq!(
        production_rng.state_bits(),
        u64::from(MINES_ROUTE_CHECKPOINT)
    );
    assert!(!select_cemetery_with_probability(
        &mut production_rng,
        Area::Mines,
        false,
        true,
        10,
    ));
    assert_eq!(
        production_rng.state_bits(),
        u64::from(MINES_FIRST_ROOM_CHECKPOINT)
    );

    let mut world = mines_world_with_source_boundaries();
    let mut features = Features::default();
    let script = &archive().scripts["scrRoomGen"];
    let (template, shop) = choose_room(
        script,
        "scrRoomGen",
        &mut production_rng,
        production_path.rooms[0][0],
        0,
        false,
        false,
        0,
        0,
        1,
        &selected_flags,
        &mut features,
        &world,
        true,
    );
    assert_eq!(template, MINES_SIDE_ROOM_6);
    assert_eq!(shop, "General");
    assert_eq!(features, Features::default());
    assert_eq!(production_rng.state_bits(), u64::from(MINES_AFTER_SELECTOR));

    let expanded = expand(script, &mut production_rng, template);
    assert_eq!(expanded, MINES_SIDE_ROOM_6);
    assert_eq!(production_rng.state_bits(), u64::from(MINES_AFTER_SELECTOR));
    assert!(expanded.bytes().all(|symbol| !b"568".contains(&symbol)));

    let expected_births = literal_mines_first_room(&mut source, MINES_SIDE_ROOM_6);
    assert_eq!(source.state, MINES_AFTER_FIRST_ROOM);
    assert_eq!(source.draws, 691);

    tiles::stamp_with_source_context(
        &mut world,
        &mut production_rng,
        &expanded,
        0,
        0,
        false,
        false,
        shop,
        false,
        GenerationContext::default(),
    );
    assert_eq!(
        production_rng.state_bits(),
        u64::from(MINES_AFTER_FIRST_ROOM)
    );

    let actual_births: Vec<_> = world
        .entities
        .iter()
        .filter(|entity| {
            entity.object == "oTerrainMetadata"
                && entity.value("source_room_terrain_created") == 1.0
        })
        .map(|entity| {
            let x = (entity.pos.x / 16.0).floor() as i32;
            let y = (entity.pos.y / 16.0).floor() as i32;
            TerrainBirth {
                x: x as usize,
                y: y as usize,
                object: world.tile(x, y).unwrap().object.clone(),
            }
        })
        .collect();
    assert_eq!(actual_births, expected_births);
    assert_eq!(actual_births.len(), 49);
    assert_eq!(
        actual_births
            .iter()
            .filter(|birth| birth.object == "oBlock")
            .count(),
        5
    );
    assert_eq!(
        actual_births
            .iter()
            .filter(|birth| birth.object == "oBrick")
            .count(),
        44
    );
    assert!(
        world
            .entities
            .iter()
            .all(|entity| entity.object == "oTerrainMetadata"),
        "the fixed source stream spawns no gem or generated item"
    );
}
