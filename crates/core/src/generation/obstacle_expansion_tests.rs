//! Literal source oracle for every main room generator's obstacle expansion
//! loop. Case rows and RNG draws come from the preserved Classic GML scripts.
//! The source case rows are parsed independently from the preserved GML files;
//! expansion and the GM8 LCG step are transcribed here rather than using the
//! production asset records or `expand` implementation.
use super::{archive, expand};
use crate::Rng;
use std::collections::{BTreeMap, BTreeSet};

const JUNGLE: &str =
    include_str!("../../../../references/classic-1.1/Scripts/Level Generation/scrRoomGen2.gml");
const MARKET: &str = include_str!(
    "../../../../references/classic-1.1/Scripts/Level Generation/scrRoomGenMarket.gml"
);
const MINES: &str =
    include_str!("../../../../references/classic-1.1/Scripts/Level Generation/scrRoomGen.gml");
const ICE: &str =
    include_str!("../../../../references/classic-1.1/Scripts/Level Generation/scrRoomGen3.gml");
const TEMPLE: &str =
    include_str!("../../../../references/classic-1.1/Scripts/Level Generation/scrRoomGen4.gml");
const BOSS: &str =
    include_str!("../../../../references/classic-1.1/Scripts/Level Generation/scrRoomGen5.gml");
const YETI: &str =
    include_str!("../../../../references/classic-1.1/Scripts/Level Generation/scrRoomGenYeti.gml");
const SOURCE_FIXTURES: [(&str, &str); 7] = [
    ("scrRoomGen", MINES),
    ("scrRoomGen2", JUNGLE),
    ("scrRoomGen3", ICE),
    ("scrRoomGen4", TEMPLE),
    ("scrRoomGen5", BOSS),
    ("scrRoomGenMarket", MARKET),
    ("scrRoomGenYeti", YETI),
];

#[derive(Clone)]
struct ObstacleCase {
    rows: [String; 4],
    assigned: [bool; 4],
}

type Cases = BTreeMap<(u8, u32), ObstacleCase>;

#[derive(Clone)]
struct SourceRng {
    state: u32,
}

impl SourceRng {
    fn new(seed: u32) -> Self {
        Self { state: seed }
    }

    fn rand(&mut self, min: u32, max: u32) -> u32 {
        self.state = self.state.wrapping_mul(0x0808_8405).wrapping_add(1);
        let width = u64::from(max - min + 1);
        min + ((u64::from(self.state) * width) >> 32) as u32
    }
}

fn obstacle_source(source: &str) -> &str {
    source
        .split("// Add obstacles")
        .nth(1)
        .expect("source obstacle section")
        .split("// Generate the tiles")
        .next()
        .expect("source tile-generation boundary")
}

fn source_room_templates(source: &str) -> Vec<String> {
    let selectors = source
        .split("// Add obstacles")
        .next()
        .expect("source room selector section");
    let mut templates = BTreeSet::new();
    for line in selectors.lines() {
        let code = line.split("//").next().unwrap_or(line);
        let mut remainder = code;
        while let Some(assignment) = remainder.find("strTemp = \"") {
            remainder = &remainder[assignment + "strTemp = \"".len()..];
            let Some(end) = remainder.find('"') else {
                break;
            };
            let value = &remainder[..end];
            if value.len() == 80 && value.is_ascii() {
                templates.insert(value.to_owned());
            }
            remainder = &remainder[end + 1..];
        }
    }
    templates.into_iter().collect()
}

fn marker_on_line(line: &str) -> Option<u8> {
    let line = line.trim();
    [b'8', b'5', b'6', b'V', b'F', b'r']
        .into_iter()
        .find(|marker| {
            let prefix = format!("tile == \"{}\"", char::from(*marker));
            (line.starts_with("if (") || line.starts_with("else if ("))
                && line.contains(&prefix)
                && line.contains(')')
                && !line.contains(" or ")
        })
}

fn parse_source_cases(source: &str) -> Cases {
    let mut cases = Cases::new();
    let mut marker = None;
    let mut case = None;

    for line in obstacle_source(source).lines() {
        if let Some(next_marker) = marker_on_line(line) {
            marker = Some(next_marker);
            case = None;
        }

        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("case ") {
            let number = rest
                .split(':')
                .next()
                .expect("case number")
                .parse::<u32>()
                .expect("numeric source case");
            case = Some(number);
        }

        let (Some(marker), Some(case)) = (marker, case) else {
            continue;
        };
        for assignment in line.split("strObs").skip(1) {
            let Some(row) = assignment.as_bytes().first().copied() else {
                continue;
            };
            if !(b'1'..=b'4').contains(&row) {
                continue;
            }
            let row = u32::from(row - b'0');
            let Some(rhs) = assignment[1..].trim_start().strip_prefix('=') else {
                // Ignore reads such as string_insert(strObs1, ...), plus
                // comparisons such as `strObs1 == ...`; only assignments in
                // the source case table define obstacle rows.
                continue;
            };
            let rhs = rhs.trim_start();
            let Some(literal) = rhs.strip_prefix('\"') else {
                continue;
            };
            let Some(end) = literal.find('\"') else {
                continue;
            };
            let value = &literal[..end];
            let width = match marker {
                b'F' => 3,
                b'r' => 4,
                _ => 5,
            };
            assert_eq!(value.len(), width, "source obstacle row width");
            let row_index = (row as usize) - 1;
            let obstacle_case = cases.entry((marker, case)).or_insert_with(|| ObstacleCase {
                rows: std::array::from_fn(|_| "00000".to_owned()),
                assigned: [false; 4],
            });
            obstacle_case.rows[row_index] = value.to_owned();
            obstacle_case.assigned[row_index] = true;
        }
    }

    cases
}

fn obstacle_markers(source: &str) -> Vec<u8> {
    let mut markers = Vec::new();
    for line in obstacle_source(source).lines() {
        if let Some(marker) = marker_on_line(line)
            && !markers.contains(&marker)
        {
            markers.push(marker);
        }
    }
    markers
}

fn obstacle_case_bound(source: &str, marker: u8) -> u32 {
    let mut in_marker = false;
    for line in obstacle_source(source).lines() {
        if let Some(found) = marker_on_line(line) {
            if found == marker {
                in_marker = true;
            } else if in_marker {
                break;
            }
        }
        if !in_marker {
            continue;
        }
        if let Some(rest) = line.split("rand(1,").nth(1) {
            return rest
                .split(')')
                .next()
                .expect("source rand upper bound")
                .parse()
                .expect("integer source rand upper bound");
        }
    }
    panic!(
        "source marker {:?} has no rand(1, N) case selector",
        marker as char
    )
}

fn assert_source_contract(source: &str) -> Cases {
    let body = obstacle_source(source);
    let cases = parse_source_cases(source);
    let has_special_ground = cases
        .keys()
        .any(|(marker, case)| *marker == b'5' && *case >= 100);
    if has_special_ground {
        assert!(body.contains("if (rand(1,8) == 1) n = rand(100,102);"));
        assert!(body.contains("else n = rand(1,2);"));
    }
    for marker in obstacle_markers(source) {
        let actual: Vec<_> = cases
            .keys()
            .filter_map(|(found, number)| (found == &marker).then_some(*number))
            .collect();
        let expected: Vec<u32> = if marker == b'5' && has_special_ground {
            vec![1, 2, 100, 101, 102]
        } else {
            (1..=obstacle_case_bound(source, marker)).collect()
        };
        assert!(
            actual.iter().all(|number| expected.contains(number)),
            "source obstacle cases exceed the selector for {:?}",
            marker as char
        );
        assert!(
            !actual.is_empty(),
            "source has no explicit cases for {marker:?}"
        );
        for number in actual {
            let rows = &cases[&(marker, number)];
            let row_count = if marker == b'V' { 4 } else { 3 };
            let width = match marker {
                b'F' => 3,
                b'r' => 4,
                _ => 5,
            };
            assert!(rows.rows[..row_count].iter().all(|row| row.len() == width));
            assert!(rows.assigned[..row_count].iter().all(|assigned| *assigned));
            assert!(rows.assigned[row_count..].iter().all(|assigned| !assigned));
        }
    }
    cases
}

fn source_case(source: &str, marker: u8, rng: &mut SourceRng) -> u32 {
    let special_ground = marker == b'5'
        && parse_source_cases(source)
            .keys()
            .any(|(found, case)| *found == b'5' && *case >= 100);
    if special_ground {
        if rng.rand(1, 8) == 1 {
            rng.rand(100, 102)
        } else {
            rng.rand(1, 2)
        }
    } else {
        rng.rand(1, obstacle_case_bound(source, marker))
    }
}

fn expand_source_for_source(
    source: &str,
    template: &str,
    cases: &Cases,
    rng: &mut SourceRng,
) -> String {
    let mut bytes = template.as_bytes().to_vec();
    assert_eq!(bytes.len(), 80);
    let markers = obstacle_markers(source);
    for index in 0..80 {
        let marker = bytes[index];
        if !markers.contains(&marker) {
            continue;
        }
        let choice = source_case(source, marker, rng);
        let default_rows = std::array::from_fn(|_| "00000".to_owned());
        let rows = cases
            .get(&(marker, choice))
            .map(|case| &case.rows)
            .unwrap_or(&default_rows);
        let height = if marker == b'V' { 4 } else { 3 };
        let width = match marker {
            b'F' => 3,
            b'r' => 4,
            _ => 5,
        };
        for (row, text) in rows.iter().take(height).enumerate() {
            let start = index + row * 10;
            for (column, byte) in text.bytes().take(width).enumerate() {
                if start + column < 80 {
                    bytes[start + column] = byte;
                }
            }
        }
    }
    String::from_utf8(bytes).expect("source obstacle templates are ASCII")
}

fn template_with_marker(marker: u8) -> String {
    let mut bytes = vec![b'0'; 80];
    bytes[0] = marker;
    String::from_utf8(bytes).unwrap()
}

fn first_seed_for_case(source: &str, marker: u8, wanted: u32) -> u32 {
    (0..100_000)
        .find(|seed| source_case(source, marker, &mut SourceRng::new(*seed)) == wanted)
        .unwrap_or_else(|| panic!("no seed found for marker {marker:?}, case {wanted}"))
}

fn compare_expansion(script_name: &str, source: &str, template: &str, seed: u32, cases: &Cases) {
    let mut source_rng = SourceRng::new(seed);
    let expected = expand_source_for_source(source, template, cases, &mut source_rng);

    let mut actual_rng = Rng::new_canonical(seed);
    let actual = expand(&archive().scripts[script_name], &mut actual_rng, template);
    assert_eq!(actual, expected, "script={script_name}, seed={seed}");
    assert_eq!(
        actual_rng.state_bits(),
        u64::from(source_rng.state),
        "source state after expansion, script={script_name}, seed={seed}"
    );

    let expected_next = source_rng.rand(1, 257);
    let actual_next = 1 + actual_rng.range(257);
    assert_eq!(
        actual_next, expected_next,
        "next draw after expansion, script={script_name}, seed={seed}"
    );
    assert_eq!(actual_rng.state_bits(), u64::from(source_rng.state));
}

#[test]
fn every_generator_obstacle_case_matches_source_rows_and_rng_continuation() {
    for (script_name, source) in SOURCE_FIXTURES {
        let cases = assert_source_contract(source);
        for marker in obstacle_markers(source) {
            for &(case_marker, number) in cases.keys() {
                if marker == case_marker {
                    let seed = first_seed_for_case(source, marker, number);
                    compare_expansion(
                        script_name,
                        source,
                        &template_with_marker(marker),
                        seed,
                        &cases,
                    );
                }
            }
        }

        // Check the source loop's sequential draw order for each family with
        // overlapping chunk rows. Put the last marker at row 4 so every
        // four-row vine expansion remains within the 80-byte room.
        let mut bytes = vec![b'0'; 80];
        for (index, marker) in [0, 15, 32, 47].into_iter().zip(obstacle_markers(source)) {
            bytes[index] = marker;
        }
        let combined = String::from_utf8(bytes).unwrap();
        for seed in 0..32 {
            compare_expansion(script_name, source, &combined, seed, &cases);
        }

        // Exercise every 80-byte room literal assigned by the source selector,
        // so overlapping obstacle rows are checked in authentic templates as
        // well as the synthetic marker matrix above.
        let templates = source_room_templates(source);
        assert!(!templates.is_empty(), "source templates for {script_name}");
        for template in templates {
            for seed in [0, 1, 0x1234_5678, u32::MAX] {
                compare_expansion(script_name, source, &template, seed, &cases);
            }
        }
    }
}
