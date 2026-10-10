use serde_json::{Value, json};
use spelunky_core::{InputFrame, Rng, RulesPreset, World, generation, level, replay};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    path::Path,
    process::ExitCode,
};

const HELP: &str = "Usage: spelunky-tools inventory\n       spelunky-tools generate <seed> <level:1..16> [special]\n       spelunky-tools replay <seed> <ticks:0..100000> [classic|assisted]\n       spelunky-tools validate-level <path>\n       spelunky-tools replay-file <path>";
const MAX_REPLAY_BYTES: u64 = 64 * 1024 * 1024;

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
            let seed = number::<u64>(&args[1], "seed")?;
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
            let world = generation::generate(&mut Rng::new(seed), level, &flags);
            let summary = json!({"seed":seed,"level":level,"area":world.area,"width":world.width,"height":world.height,"entities":entity_counts(&world),"dark":world.dark,"entrance":world.entrance,"exit":world.exit});
            Ok(format!(
                "{}\n{}",
                serde_json::to_string(&summary).map_err(|e| e.to_string())?,
                ascii(&world)
            ))
        }
        Some("replay") if (3..=4).contains(&args.len()) => {
            let seed = number::<u64>(&args[1], "seed")?;
            let ticks = number::<u32>(&args[2], "ticks")?;
            let rules = parse_rules(args.get(3).map(String::as_str).unwrap_or("classic"))?;
            let result = replay::try_scripted(seed, ticks, rules)?;
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
        Some("replay-file") if args.len() == 2 => {
            let text = read_text(Path::new(&args[1]), MAX_REPLAY_BYTES)?;
            let (seed, rules, frames) = parse_replay(&text)?;
            let result = replay::from_frames(seed, rules, &frames)?;
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

fn parse_replay(text: &str) -> Result<(u64, RulesPreset, Vec<InputFrame>), String> {
    let value: Value =
        serde_json::from_str(text).map_err(|e| format!("Invalid replay JSON: {e}"))?;
    let object = value.as_object().ok_or("Replay must be an object")?;
    if object
        .keys()
        .any(|k| !matches!(k.as_str(), "seed" | "rules" | "frames"))
    {
        return Err("Unknown replay property; expected seed, rules, and frames".into());
    }
    let seed = object
        .get("seed")
        .and_then(Value::as_u64)
        .ok_or("Replay seed must be an unsigned integer")?;
    let rules = parse_rules(
        object
            .get("rules")
            .and_then(Value::as_str)
            .ok_or("Replay rules must be a string")?,
    )?;
    let frames = object
        .get("frames")
        .and_then(Value::as_array)
        .ok_or("Replay frames must be an array")?;
    if frames.len() > replay::MAX_REPLAY_TICKS as usize {
        return Err("Replay exceeds the 100000 tick limit".into());
    }
    let defaults = serde_json::to_value(InputFrame::default()).map_err(|e| e.to_string())?;
    let mut parsed = Vec::with_capacity(frames.len());
    for (i, frame) in frames.iter().enumerate() {
        let mut merged = defaults.clone();
        let fields = frame
            .as_object()
            .ok_or_else(|| format!("Frame {i} must be an object"))?;
        for (key, value) in fields {
            if merged.get(key).is_none() {
                return Err(format!("Frame {i} has unknown input {key}"));
            }
            if !value.is_boolean() {
                return Err(format!("Frame {i} input {key} must be boolean"));
            }
            merged[key] = value.clone();
        }
        parsed.push(serde_json::from_value(merged).map_err(|e| format!("Invalid frame {i}: {e}"))?);
    }
    Ok((seed, rules, parsed))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn replay_file_defaults_only_missing_input_fields() {
        let (seed, rules, frames) = parse_replay(
            r#"{"seed":9,"rules":"assisted","frames":[{}, {"right":true,"jump_pressed":true}]}"#,
        )
        .unwrap();
        assert_eq!(seed, 9);
        assert_eq!(rules, RulesPreset::Assisted);
        assert_eq!(frames[0], InputFrame::default());
        assert!(frames[1].right && frames[1].jump_pressed);
        assert!(!frames[1].left);
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
}
