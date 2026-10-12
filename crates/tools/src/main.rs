use serde_json::{Value, json};
#[cfg(test)]
use spelunky_core::InputFrame;
use spelunky_core::{Rng, RngAlgorithm, RulesPreset, World, generation, level, replay};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    path::Path,
    process::ExitCode,
};

const HELP: &str = "Usage: spelunky-tools inventory\n       spelunky-tools generate <seed:u32> <level:1..16> [special]\n       spelunky-tools replay <seed:u32> <ticks:0..100000> [classic|assisted]\n       spelunky-tools replay-legacy <seed:u64> <ticks:0..100000> [classic|assisted]\n       spelunky-tools replay-adaptive <seed:u32> <ticks:0..100000> <logical-width> <logical-height> [classic|assisted]\n       spelunky-tools validate-level <path>\n       spelunky-tools replay-file <path>";
const MAX_REPLAY_BYTES: u64 = replay::MAX_RECORDING_BYTES as u64;

fn main() -> ExitCode {
    match execute(env::args().skip(1).collect()) {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("{error}\n{HELP}");
            ExitCode::FAILURE
        }
    }
}

fn execute(args: Vec<String>) -> Result<String, String> {
    match args.first().map(String::as_str) {
        Some("inventory") if args.len() == 1 => {
            let path =
                Path::new(env!("CARGO_MANIFEST_DIR")).join("../../references/source-index.json");
            let index: Value = serde_json::from_str(&read_text(&path, 16 * 1024 * 1024)?)
                .map_err(|e| format!("Invalid source inventory: {e}"))?;
            print_json(json!({"source": index["source"], "counts": index["counts"]}))
        }
        Some("generate") if (3..=4).contains(&args.len()) => {
            let seed = number::<u32>(&args[1], "canonical seed (0..4294967295)")?;
            let level = number::<u32>(&args[2], "level")?;
            if !(1..=16).contains(&level) {
                return Err("Level must be between 1 and 16".into());
            }
            let mut flags = BTreeSet::new();
            if let Some(special) = args.get(3) {
                let required_area = match special.as_str() {
                    "dark" => None,
                    "snake_pit" => Some(0),
                    "cemetery" | "lake" | "black_market" => Some(1),
                    "yeti_lair" | "alien_ship" | "moai" => Some(2),
                    "sacrifice_pit" | "city_of_gold" => Some(3),
                    _ => return Err("Unknown special: use dark, snake_pit, cemetery, lake, black_market, yeti_lair, alien_ship, moai, sacrifice_pit, or city_of_gold".into()),
                };
                let area = match level {
                    1..=4 => 0,
                    5..=8 => 1,
                    9..=12 => 2,
                    13..=15 => 3,
                    _ => 4,
                };
                if required_area.is_some_and(|a| a != area) {
                    return Err(format!(
                        "Special {special} is incompatible with level {level}"
                    ));
                }
                flags.insert(special.clone());
            }
            let world = generation::generate(&mut Rng::new_canonical(seed), level, &flags);
            let summary = json!({"seed":seed.to_string(),"algorithm":RngAlgorithm::Gm8Lcg32V1,"level":level,"area":world.area,"width":world.width,"height":world.height,"entities":entity_counts(&world),"dark":world.dark,"entrance":world.entrance,"exit":world.exit});
            Ok(format!(
                "{}\n{}",
                serde_json::to_string(&summary).map_err(|e| e.to_string())?,
                ascii(&world)
            ))
        }
        Some("replay" | "replay-legacy") if (3..=4).contains(&args.len()) => {
            let seed = number::<u64>(&args[1], "seed")?;
            let ticks = number::<u32>(&args[2], "ticks")?;
            let rules = parse_rules(args.get(3).map(String::as_str).unwrap_or("classic"))?;
            let algorithm = if args[0] == "replay-legacy" {
                RngAlgorithm::LegacySplitMix64V1
            } else {
                RngAlgorithm::Gm8Lcg32V1
            };
            let result = replay::try_scripted_with_algorithm(seed, ticks, rules, algorithm)?;
            print_json(serde_json::to_value(result).map_err(|e| e.to_string())?)
        }
        Some("validate-level") if args.len() == 2 => {
            let text = read_text(Path::new(&args[1]), 4 * 1024 * 1024)?;
            let custom = level::Level::parse(&text)?;
            let world = custom.to_world();
            print_json(
                json!({"valid":true,"rows":level::LEVEL_HEIGHT,"columns":level::LEVEL_WIDTH,"author":custom.author,"music":custom.music,"life":custom.starting_life,"bombs":custom.starting_bombs,"ropes":custom.starting_ropes,"next_level":custom.next_level,"exit_targets":custom.exit_targets.len(),"signs":custom.signs.len(),"entities":entity_counts(&world)}),
            )
        }
        Some("replay-adaptive") if (5..=6).contains(&args.len()) => {
            let seed = number::<u32>(&args[1], "canonical seed (0..4294967295)")?;
            let ticks = number::<u32>(&args[2], "ticks")?;
            let width = args[3]
                .parse::<f64>()
                .map_err(|_| "Invalid logical width")?;
            let height = args[4]
                .parse::<f64>()
                .map_err(|_| "Invalid logical height")?;
            let view = replay::ReplayViewTimeline::fixed(width, height)?;
            let rules = parse_rules(args.get(5).map(String::as_str).unwrap_or("classic"))?;
            let result = replay::try_scripted_with_view_timeline(
                u64::from(seed),
                ticks,
                rules,
                RngAlgorithm::Gm8Lcg32V1,
                &view,
            )?;
            print_json(serde_json::to_value(result).map_err(|e| e.to_string())?)
        }
        Some("replay-file") if args.len() == 2 => {
            let text = read_text(Path::new(&args[1]), MAX_REPLAY_BYTES)?;
            let result = replay::from_recording_json(&text)?;
            print_json(serde_json::to_value(result).map_err(|e| e.to_string())?)
        }
        Some("--help" | "-h" | "help") if args.len() == 1 => Ok(HELP.into()),
        _ => Err("Unknown command or incorrect argument count".into()),
    }
}

fn read_text(path: &Path, max_bytes: u64) -> Result<String, String> {
    let file = fs::File::open(path).map_err(|e| format!("Cannot open {}: {e}", path.display()))?;
    if !file.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("Input must be a regular file".into());
    }
    use std::io::Read;
    let mut bytes = Vec::new();
    file.take(max_bytes + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > max_bytes {
        return Err(format!("Input exceeds {max_bytes} bytes"));
    }
    String::from_utf8(bytes).map_err(|e| format!("Input is not UTF-8: {e}"))
}

fn number<T: std::str::FromStr>(value: &str, name: &str) -> Result<T, String> {
    value
        .parse()
        .map_err(|_| format!("Invalid {name}: expected an unsigned integer"))
}
fn parse_rules(value: &str) -> Result<RulesPreset, String> {
    match value {
        "classic" | "Classic" => Ok(RulesPreset::Classic),
        "assisted" | "Assisted" => Ok(RulesPreset::Assisted),
        _ => Err("Rules must be classic or assisted".into()),
    }
}
fn print_json(value: Value) -> Result<String, String> {
    serde_json::to_string(&value).map_err(|e| e.to_string())
}
fn entity_counts(world: &World) -> BTreeMap<&str, usize> {
    let mut counts = BTreeMap::new();
    for entity in &world.entities {
        *counts.entry(entity.object.as_str()).or_default() += 1;
    }
    counts
}
fn ascii(world: &World) -> String {
    let mut cells: Vec<char> = world
        .tiles
        .iter()
        .map(|t| {
            if t.object.is_empty() {
                ' '
            } else if t.is_ladder() {
                'H'
            } else if t.is_solid() {
                '#'
            } else {
                '.'
            }
        })
        .collect();
    for e in &world.entities {
        if matches!(
            e.object.as_str(),
            "oTerrainMetadata" | "oGenerationDecoration"
        ) {
            continue;
        }
        let x = (e.pos.x / 16.0).floor() as usize;
        let y = (e.pos.y / 16.0).floor() as usize;
        if x < world.width && y < world.height {
            cells[y * world.width + x] = match e.object.as_str() {
                "oEntrance" => '@',
                "oExit" | "oXMarket" | "oXGold" => 'X',
                "oDamsel" => 'D',
                "oSpikes" => '^',
                "oWaterSwim" => '~',
                "oLava" => '!',
                _ => 'e',
            };
        }
    }
    cells
        .chunks(world.width)
        .map(|row| row.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
fn parse_replay(text: &str) -> Result<replay::ReplayRecording, String> {
    replay::parse_recording_json(text)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn adaptive_recording() -> Value {
        json!({"version":2,"simulation_version":spelunky_core::SIMULATION_VERSION,
            "algorithm":"gm8_lcg32_v1","seed":"7","rules":"classic",
            "adaptive_view":{"policy":"gameplay_logical_extent_v1",
                "initial":{"width":480,"height":240},
                "changes":[{"before_frame":2,"extent":{"width":320,"height":640}}]},
            "frames":[{}, {"right":true}, {"jump_pressed":true,"jump":true}]})
    }

    #[test]
    fn adaptive_recording_round_trip_and_cli_preserve_explicit_resize_identity() {
        let recording = adaptive_recording();
        let text = serde_json::to_string(&recording).unwrap();
        let parsed = parse_replay(&text).unwrap();
        let view = parsed.adaptive_view.as_ref().unwrap();
        assert_eq!(view.initial.dimensions(), [480.0, 240.0]);
        assert_eq!(view.changes[0].before_frame, 2);
        assert_eq!(view.changes[0].extent.dimensions(), [320.0, 640.0]);
        assert_eq!(parsed.frames.len(), 3);
        let result = replay::from_frames_with_view_timeline(
            parsed.seed,
            parsed.rules,
            &parsed.frames,
            parsed.algorithm,
            parsed.simulation_version,
            view,
        )
        .unwrap();
        let mut manual = spelunky_core::Game::new(7, RulesPreset::Classic);
        for (index, input) in parsed.frames.iter().enumerate() {
            let (width, height) = if index < 2 {
                (480.0, 240.0)
            } else {
                (320.0, 640.0)
            };
            manual.set_simulation_view_extent(width, height).unwrap();
            manual.step(input);
        }
        assert_eq!(result.hash, format!("{:016x}", manual.state_hash()));
        assert_eq!(
            serde_json::to_value(&result).unwrap()["adaptive_view"],
            serde_json::to_value(view).unwrap()
        );
        let output = execute(
            vec!["replay-adaptive", "7", "0", "480", "240"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
        )
        .unwrap();
        let output: Value = serde_json::from_str(&output).unwrap();
        assert_eq!(output["version"], 2);
        assert_eq!(
            output["adaptive_view"]["initial"],
            json!({"width":480.0,"height":240.0})
        );
        let original = execute(
            vec!["replay", "7", "0"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
        )
        .unwrap();
        let original: Value = serde_json::from_str(&original).unwrap();
        assert_eq!(original["version"], 1);
        assert!(original.get("adaptive_view").is_none());
    }

    #[test]
    fn adaptive_recording_requires_version_two_valid_geometry_and_ordered_changes() {
        let record = adaptive_recording();
        for version in [json!(1), json!(3), Value::Null] {
            let mut invalid = record.clone();
            invalid["version"] = version;
            assert!(parse_replay(&invalid.to_string()).is_err());
        }
        let mut missing = record.clone();
        missing.as_object_mut().unwrap().remove("adaptive_view");
        assert!(parse_replay(&missing.to_string()).is_err());
        for view in [
            Value::Null,
            json!({}),
            json!({"policy":"other","initial":{"width":480,"height":240}}),
            json!({"policy":"gameplay_logical_extent_v1","initial":{"width":319,"height":240}}),
            json!({"policy":"gameplay_logical_extent_v1","initial":{"width":480,"height":240},"extra":true}),
        ] {
            let mut invalid = record.clone();
            invalid["adaptive_view"] = view;
            assert!(parse_replay(&invalid.to_string()).is_err());
        }
        for changes in [vec![0], vec![3], vec![1, 1], vec![2, 1]] {
            let mut invalid = record.clone();
            invalid["adaptive_view"]["changes"] = Value::Array(
                changes
                    .into_iter()
                    .map(|frame| json!({"before_frame":frame,"extent":{"width":480,"height":240}}))
                    .collect(),
            );
            assert!(parse_replay(&invalid.to_string()).is_err());
        }
        assert!(
            parse_replay(r#"{"seed":7,"rules":"classic","frames":[],"adaptive_view":null}"#)
                .is_err()
        );
        for dimensions in [
            ("NaN", "240"),
            ("480", "inf"),
            ("319", "240"),
            ("480", "239"),
        ] {
            assert!(
                execute(
                    vec!["replay-adaptive", "7", "0", dimensions.0, dimensions.1]
                        .into_iter()
                        .map(str::to_owned)
                        .collect()
                )
                .is_err()
            );
        }
    }

    #[test]
    fn replay_file_defaults_only_missing_input_fields() {
        let parsed = parse_replay(
            r#"{"seed":9,"rules":"assisted","frames":[{}, {"right":true,"jump_pressed":true}]}"#,
        )
        .unwrap();
        assert_eq!(parsed.seed, 9);
        assert_eq!(parsed.algorithm, RngAlgorithm::LegacySplitMix64V1);
        assert_eq!(parsed.rules, RulesPreset::Assisted);
        assert_eq!(parsed.frames[0], InputFrame::default());
        assert!(parsed.frames[1].right && parsed.frames[1].jump_pressed);
        assert!(!parsed.frames[1].left);
    }
    #[test]
    fn replay_file_rejects_mistyped_or_unknown_inputs() {
        for frame in [r#"{"right":1}"#, r#"{"jumpp":true}"#, "null"] {
            assert!(
                parse_replay(&format!(
                    r#"{{"seed":0,"rules":"classic","frames":[{frame}]}}"#
                ))
                .is_err()
            );
        }
    }
    #[test]
    fn cli_rejects_out_of_range_and_extra_arguments() {
        for args in [
            vec!["replay", "0", "100001"],
            vec!["generate", "0", "17"],
            vec!["generate", "0", "1", "black_market"],
            vec!["inventory", "unexpected"],
        ] {
            assert!(execute(args.into_iter().map(str::to_owned).collect()).is_err());
        }
    }
    #[test]
    fn historical_replay_keeps_full_seed_and_selects_legacy_before_generation() {
        let parsed =
            parse_replay(r#"{"seed":18446744073709551615,"rules":"classic","frames":[]}"#).unwrap();
        assert_eq!(parsed.seed, u64::MAX);
        assert_eq!(parsed.algorithm, RngAlgorithm::LegacySplitMix64V1);
        let result = replay::from_frames_with_algorithm(
            parsed.seed,
            parsed.rules,
            &parsed.frames,
            parsed.algorithm,
        )
        .unwrap();
        assert_eq!(
            result.hash,
            format!(
                "{:016x}",
                spelunky_core::Game::legacy(u64::MAX, RulesPreset::Classic).state_hash()
            )
        );
        assert_eq!(result.seed, "18446744073709551615");
    }

    #[test]
    fn historical_replay_file_nonempty_frames_match_manual_legacy_for_both_rules() {
        use spelunky_core::Game;
        use std::io::Write;

        // These literals intentionally contain neither version nor algorithm metadata.
        // They test the old numeric-u64 JSON contract, not frozen historical physics.
        let cases = [
            (
                RulesPreset::Classic,
                r#"{"seed":18446744073709551615,"rules":"classic","frames":[{}, {"right":true,"jump":true,"jump_pressed":true}, {"right":true,"jump":true}, {"jump_released":true}, {"bomb_pressed":true}, {"rope_pressed":true}, {"left":true,"attack_pressed":true}, {"down":true}]}"#,
            ),
            (
                RulesPreset::Assisted,
                r#"{"seed":18446744073709551615,"rules":"assisted","frames":[{}, {"right":true,"jump":true,"jump_pressed":true}, {"right":true,"jump":true}, {"jump_released":true}, {"bomb_pressed":true}, {"rope_pressed":true}, {"left":true,"attack_pressed":true}, {"down":true}]}"#,
            ),
        ];
        let expected_frames = [
            InputFrame::default(),
            InputFrame {
                right: true,
                jump: true,
                jump_pressed: true,
                ..InputFrame::default()
            },
            InputFrame {
                right: true,
                jump: true,
                ..InputFrame::default()
            },
            InputFrame {
                jump_released: true,
                ..InputFrame::default()
            },
            InputFrame {
                bomb_pressed: true,
                ..InputFrame::default()
            },
            InputFrame {
                rope_pressed: true,
                ..InputFrame::default()
            },
            InputFrame {
                left: true,
                attack_pressed: true,
                ..InputFrame::default()
            },
            InputFrame {
                down: true,
                ..InputFrame::default()
            },
        ];
        fn assert_same_continuation(actual: &Game, expected: &Game) {
            assert_eq!(actual.state_hash(), expected.state_hash());
            assert_eq!(actual.events, expected.events);
            assert_eq!(
                actual.rng.clone().next_u64(),
                expected.rng.clone().next_u64()
            );
        }

        for (rules, literal) in cases {
            let parsed = parse_replay(literal).unwrap();
            assert_eq!(parsed.seed, u64::MAX);
            assert_eq!(parsed.rules, rules);
            assert_eq!(parsed.algorithm, RngAlgorithm::LegacySplitMix64V1);
            assert_eq!(parsed.frames.as_slice(), expected_frames.as_slice());
            // The independent expected game and inputs do not use parser output.
            let mut expected = Game::legacy(u64::MAX, rules);
            let mut selected =
                Game::new_with_algorithm(parsed.seed, parsed.rules, parsed.algorithm).unwrap();
            assert_same_continuation(&selected, &expected);
            for (actual_input, expected_input) in parsed.frames.iter().zip(&expected_frames) {
                selected.step(actual_input);
                expected.step(expected_input);
                assert_same_continuation(&selected, &expected);
            }

            // Also cover the actual replay-file entry point, including file ingestion
            // and its selected-algorithm replay runner and JSON diagnostic envelope.
            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = env::temp_dir().join(format!(
                "spelunky-historical-replay-{}-{nonce}-{rules:?}.json",
                std::process::id()
            ));
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
                .unwrap();
            file.write_all(literal.as_bytes()).unwrap();
            drop(file);
            let output = execute(vec![
                "replay-file".to_owned(),
                path.to_str().unwrap().to_owned(),
            ]);
            fs::remove_file(&path).unwrap();
            let actual: Value = serde_json::from_str(&output.unwrap()).unwrap();
            assert_eq!(
                actual,
                json!({
                    "version": replay::REPLAY_VERSION,
                    "simulation_version": spelunky_core::HISTORICAL_SIMULATION_VERSION,
                    "algorithm": RngAlgorithm::LegacySplitMix64V1,
                    "seed": "18446744073709551615",
                    "rules": rules,
                    "requested_ticks": expected_frames.len(),
                    "ticks": expected.tick,
                    "hash": format!("{:016x}", expected.state_hash()),
                    "score": expected.player.money,
                    "health": expected.player.health,
                    "level": expected.world.level,
                    "entities": expected.world.entities.len(),
                    "state": expected.state,
                })
            );
        }
    }

    #[test]
    fn versioned_replays_require_explicit_valid_metadata_and_checked_seeds() {
        let canonical = parse_replay(r#"{"simulation_version":1,"version":1,"algorithm":"gm8_lcg32_v1","seed":"4294967295","rules":"classic","frames":[]}"#).unwrap();
        assert_eq!(canonical.seed, u64::from(u32::MAX));
        assert_eq!(canonical.algorithm, RngAlgorithm::Gm8Lcg32V1);
        let legacy = parse_replay(r#"{"simulation_version":1,"version":1,"algorithm":"legacy_splitmix64_v1","seed":"18446744073709551615","rules":"classic","frames":[]}"#).unwrap();
        assert_eq!(legacy.seed, u64::MAX);
        for metadata in [
            r#""version":1"#,
            r#""algorithm":"gm8_lcg32_v1"#,
            r#""version":null,"algorithm":"gm8_lcg32_v1"#,
            r#""version":2,"algorithm":"gm8_lcg32_v1"#,
            r#""version":1,"algorithm":null"#,
            r#""version":1,"algorithm":"unknown"#,
        ] {
            assert!(parse_replay(&format!(r#"{{"simulation_version":1,{metadata},"seed":"1","rules":"classic","frames":[]}}"#)).is_err());
        }
        for seed in [
            "4294967296",
            "9007199254740993",
            "18446744073709551615",
            "+1",
            "-1",
            "1.0",
            "",
        ] {
            assert!(parse_replay(&format!(r#"{{"simulation_version":1,"version":1,"algorithm":"gm8_lcg32_v1","seed":"{seed}","rules":"classic","frames":[]}}"#)).is_err());
        }
        assert!(parse_replay(r#"{"simulation_version":1,"version":1,"algorithm":"gm8_lcg32_v1","seed":1,"rules":"classic","frames":[]}"#).is_err());
        assert!(parse_replay(r#"{"seed":"1","rules":"classic","frames":[]}"#).is_err());
        for simulation in ["null", "0", "3", "\"1\""] {
            assert!(parse_replay(&format!(r#"{{"version":1,"algorithm":"gm8_lcg32_v1","simulation_version":{simulation},"seed":"1","rules":"classic","frames":[]}}"#)).is_err());
        }
        assert!(parse_replay(r#"{"version":1,"algorithm":"gm8_lcg32_v1","seed":"1","rules":"classic","frames":[]}"#).is_err());
    }

    #[test]
    fn cli_new_runs_are_checked_and_legacy_diagnostics_remain_explicit() {
        for command in ["generate", "replay"] {
            let args = [
                command,
                "4294967296",
                if command == "generate" { "1" } else { "0" },
            ];
            assert!(execute(args.into_iter().map(str::to_owned).collect()).is_err());
        }
        let output = execute(
            ["replay-legacy", "18446744073709551615", "0"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
        )
        .unwrap();
        let value: Value = serde_json::from_str(&output).unwrap();
        assert_eq!(value["seed"], "18446744073709551615");
        assert_eq!(value["algorithm"], "legacy_splitmix64_v1");
        let canonical = execute(
            ["replay", "4294967295", "0"]
                .into_iter()
                .map(str::to_owned)
                .collect(),
        )
        .unwrap();
        let value: Value = serde_json::from_str(&canonical).unwrap();
        assert_eq!(value["algorithm"], "gm8_lcg32_v1");
    }
}
