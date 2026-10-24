//! Literal source oracle for the Jungle and Black Market obstacle expansion
//! loops (`scrRoomGen2.gml:246-318`, `scrRoomGenMarket.gml:174-236`).
//! The source case rows are parsed independently from the preserved GML files;
//! expansion and the GM8 LCG step are transcribed here rather than using the
//! production asset records or `expand` implementation.
use super::{archive, expand};
use crate::Rng;
use std::collections::BTreeMap;

const JUNGLE: &str =
    include_str!("../../../../references/classic-1.1/Scripts/Level Generation/scrRoomGen2.gml");
const MARKET: &str = include_str!(
    "../../../../references/classic-1.1/Scripts/Level Generation/scrRoomGenMarket.gml"
);

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

fn marker_on_line(line: &str) -> Option<u8> {
    let line = line.trim();
    [b'8', b'5', b'6', b'V'].into_iter().find(|marker| {
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
            assert_eq!(value.len(), 5, "five-column source obstacle row");
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

fn assert_source_contract(source: &str) -> Cases {
    let body = obstacle_source(source);
    for (marker, snippets) in [
        (b'8', &["n = rand(1,1);"][..]),
        (
            b'5',
            &[
                "if (rand(1,8) == 1) n = rand(100,102);",
                "else n = rand(1,2);",
            ][..],
        ),
        (b'6', &["n = rand(1,4);"][..]),
        (b'V', &["n = rand(1,3);"][..]),
    ] {
        for &snippet in snippets.iter() {
            let mut current_marker = None;
            let found = body.lines().any(|line| {
                if let Some(next_marker) = marker_on_line(line) {
                    current_marker = Some(next_marker);
                }
                current_marker == Some(marker) && line.contains(snippet)
            });
            assert!(found, "source draw contract for {marker:?}: {snippet}");
        }
    }

    let cases = parse_source_cases(source);
    let expected: &[(u8, &[u32])] = &[
        (b'8', &[1]),
        (b'5', &[1, 2, 100, 101, 102]),
        (b'6', &[1, 2, 3, 4]),
        (b'V', &[1, 2, 3]),
    ];
    for (marker, numbers) in expected {
        let actual: Vec<_> = cases
            .keys()
            .filter_map(|(found, number)| (found == marker).then_some(*number))
            .collect();
        assert_eq!(
            actual.as_slice(),
            *numbers,
            "source obstacle cases for {marker:?}"
        );
        for number in *numbers {
            let rows = &cases[&(*marker, *number)];
            let row_count = if *marker == b'V' { 4 } else { 3 };
            assert!(rows.rows[..row_count].iter().all(|row| row.len() == 5));
            assert!(rows.assigned[..row_count].iter().all(|assigned| *assigned));
            assert!(rows.assigned[row_count..].iter().all(|assigned| !assigned));
        }
    }
    cases
}

fn source_case(marker: u8, rng: &mut SourceRng) -> u32 {
    match marker {
        b'8' => rng.rand(1, 1),
        b'5' => {
            if rng.rand(1, 8) == 1 {
                rng.rand(100, 102)
            } else {
                rng.rand(1, 2)
            }
        }
        b'6' => rng.rand(1, 4),
        b'V' => rng.rand(1, 3),
        _ => panic!("unexpected Jungle/Market obstacle marker {marker:?}"),
    }
}

fn expand_source(template: &str, cases: &Cases, rng: &mut SourceRng) -> String {
    let mut bytes = template.as_bytes().to_vec();
    assert_eq!(bytes.len(), 80);
    for index in 0..80 {
        let marker = bytes[index];
        if !matches!(marker, b'8' | b'5' | b'6' | b'V') {
            continue;
        }
        let choice = source_case(marker, rng);
        let rows = &cases[&(marker, choice)].rows;
        let height = if marker == b'V' { 4 } else { 3 };
        for (row, text) in rows.iter().take(height).enumerate() {
            let start = index + row * 10;
            for (column, byte) in text.bytes().enumerate() {
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

fn first_seed_for_case(marker: u8, wanted: u32) -> u32 {
    (0..100_000)
        .find(|seed| source_case(marker, &mut SourceRng::new(*seed)) == wanted)
        .unwrap_or_else(|| panic!("no seed found for marker {marker:?}, case {wanted}"))
}

fn compare_expansion(script_name: &str, template: &str, seed: u32, cases: &Cases) {
    let mut source_rng = SourceRng::new(seed);
    let expected = expand_source(template, cases, &mut source_rng);

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
fn jungle_and_market_obstacle_cases_match_source_rows_and_rng_continuation() {
    for (script_name, source) in [("scrRoomGen2", JUNGLE), ("scrRoomGenMarket", MARKET)] {
        let cases = assert_source_contract(source);
        for marker in [b'8', b'5', b'6', b'V'] {
            let numbers: &[u32] = match marker {
                b'8' => &[1],
                b'5' => &[1, 2, 100, 101, 102],
                b'6' => &[1, 2, 3, 4],
                b'V' => &[1, 2, 3],
                _ => unreachable!(),
            };
            for &number in numbers {
                let seed = first_seed_for_case(marker, number);
                compare_expansion(script_name, &template_with_marker(marker), seed, &cases);
            }
        }

        // Check the source loop's sequential draw order across all four
        // marker types when their expanded rows partially border one another.
        let mut bytes = vec![b'0'; 80];
        for (index, marker) in [(0, b'8'), (15, b'6'), (32, b'5'), (57, b'V')] {
            bytes[index] = marker;
        }
        let combined = String::from_utf8(bytes).unwrap();
        for seed in 0..32 {
            compare_expansion(script_name, &combined, seed, &cases);
        }
    }
}
