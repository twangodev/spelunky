//! Original extended statistics. Eligibility and timing remain at source producers.
use crate::Game;
use serde::{Deserialize, Serialize};
use std::fmt::Write as _;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct LifetimeStats {
    pub enemy_kills: [u64; 22],
    pub enemy_deaths: [u64; 24],
    pub level_deaths: [u64; 16],
    pub misc_deaths: [u64; 12],
    pub total_crates: u64,
    pub total_chests: u64,
    pub idols_grabbed: u64,
    pub idols_converted: u64,
    pub damsels_grabbed: u64,
    pub kisses_bought: u64,
    pub damsels_bought: u64,
    pub damsels_saved_total: u64,
    pub damsels_killed_total: u64,
    pub items_bought: u64,
    pub items_stolen: u64,
    pub dice_games_played: u64,
    pub dice_games_won: u64,
    pub dice_games_lost: u64,
    pub total_monkey_kills: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum EnemyKill {
    Bat = 0,
    Snake = 1,
    Spider = 2,
    GiantSpider = 3,
    Caveman = 4,
    Skeleton = 5,
    Zombie = 6,
    Vampire = 7,
    Frog = 8,
    FireFrog = 9,
    ManTrap = 10,
    Piranha = 11,
    Jaws = 12,
    Yeti = 13,
    YetiKing = 14,
    Alien = 15,
    Ufo = 16,
    AlienBoss = 17,
    Hawkman = 18,
    Shopkeeper = 19,
    TombLord = 20,
    Olmec = 21,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum EnemyDeath {
    Bat = 0,
    Snake = 1,
    Spider = 2,
    GiantSpider = 3,
    Caveman = 4,
    Skeleton = 5,
    Zombie = 6,
    Vampire = 7,
    Frog = 8,
    FireFrog = 9,
    ManTrap = 10,
    Piranha = 11,
    Jaws = 12,
    Yeti = 13,
    YetiKing = 14,
    Alien = 15,
    Ufo = 16,
    AlienBoss = 17,
    Hawkman = 18,
    Shopkeeper = 19,
    TombLord = 20,
    MagmaMan = 21,
    Olmec = 22,
    Ghost = 23,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum MiscDeath {
    Rock = 0,
    Explosion = 1,
    Crushed = 2,
    LongFall = 3,
    Spikes = 4,
    Boulder = 5,
    ArrowTrap = 6,
    SpearTrap = 7,
    SmashTrap = 8,
    CeilingTrap = 9,
    Pit = 10,
    Lava = 11,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifetimeCounter {
    TotalCrates,
    TotalChests,
    IdolsGrabbed,
    IdolsConverted,
    DamselsGrabbed,
    KissesBought,
    DamselsBought,
    DamselsSavedTotal,
    DamselsKilledTotal,
    ItemsBought,
    ItemsStolen,
    DiceGamesPlayed,
    DiceGamesWon,
    DiceGamesLost,
    TotalMonkeyKills,
}

impl LifetimeStats {
    pub fn increment(&mut self, counter: LifetimeCounter) {
        let value = match counter {
            LifetimeCounter::TotalCrates => &mut self.total_crates,
            LifetimeCounter::TotalChests => &mut self.total_chests,
            LifetimeCounter::IdolsGrabbed => &mut self.idols_grabbed,
            LifetimeCounter::IdolsConverted => &mut self.idols_converted,
            LifetimeCounter::DamselsGrabbed => &mut self.damsels_grabbed,
            LifetimeCounter::KissesBought => &mut self.kisses_bought,
            LifetimeCounter::DamselsBought => &mut self.damsels_bought,
            LifetimeCounter::DamselsSavedTotal => &mut self.damsels_saved_total,
            LifetimeCounter::DamselsKilledTotal => &mut self.damsels_killed_total,
            LifetimeCounter::ItemsBought => &mut self.items_bought,
            LifetimeCounter::ItemsStolen => &mut self.items_stolen,
            LifetimeCounter::DiceGamesPlayed => &mut self.dice_games_played,
            LifetimeCounter::DiceGamesWon => &mut self.dice_games_won,
            LifetimeCounter::DiceGamesLost => &mut self.dice_games_lost,
            LifetimeCounter::TotalMonkeyKills => &mut self.total_monkey_kills,
        };
        *value = value.saturating_add(1);
    }
    pub fn enemy_kill(&mut self, kind: EnemyKill) {
        let value = &mut self.enemy_kills[kind as usize];
        *value = value.saturating_add(1);
    }
    pub fn enemy_death(&mut self, kind: EnemyDeath) {
        let value = &mut self.enemy_deaths[kind as usize];
        *value = value.saturating_add(1);
    }
    pub fn misc_death(&mut self, kind: MiscDeath) {
        let value = &mut self.misc_deaths[kind as usize];
        *value = value.saturating_add(1);
    }
    pub fn level_death(&mut self, index: usize) {
        if let Some(value) = self.level_deaths.get_mut(index) {
            *value = value.saturating_add(1);
        }
    }
    /// Source stats.txt total excludes Damsels and includes its separate Monkey row.
    pub fn total_enemy_kills(&self) -> u64 {
        self.enemy_kills
            .iter()
            .fold(self.total_monkey_kills, |sum, n| sum.saturating_add(*n))
    }

    /// Parse the fixed-order text report written by Classic's scrWriteStats.
    ///
    /// The source reader treats each table row's position as its identity,
    /// because rows with a zero count are printed as `???`. We preserve that
    /// behavior and validate the report's headings, row counts, and integers.
    pub fn from_classic_report(report: &str) -> Result<Self, String> {
        let lines: Vec<_> = report.lines().collect();
        if lines.get(1).map(|line| line.trim()) != Some("|  SPELUNKY EXTENDED STATS  |") {
            return Err("Not a Classic extended stats report".into());
        }

        let mut stats = Self {
            total_crates: parse_source_scalar(&lines, "CRATES OPENED:")?,
            total_chests: parse_source_scalar(&lines, "CHESTS OPENED:")?,
            idols_grabbed: parse_source_scalar(&lines, "IDOLS GRABBED:")?,
            idols_converted: parse_source_scalar(&lines, "IDOLS CONVERTED:")?,
            damsels_grabbed: parse_source_scalar(&lines, "DAMSELS GRABBED:")?,
            kisses_bought: parse_source_scalar(&lines, "KISSES BOUGHT:")?,
            damsels_bought: parse_source_scalar(&lines, "DAMSELS BOUGHT:")?,
            damsels_saved_total: parse_source_scalar(&lines, "DAMSELS SAVED:")?,
            damsels_killed_total: parse_source_scalar(&lines, "DAMSELS KILLED:")?,
            items_bought: parse_source_scalar(&lines, "ITEMS BOUGHT:")?,
            items_stolen: parse_source_scalar(&lines, "ITEMS STOLEN:")?,
            dice_games_played: parse_source_scalar(&lines, "DICE GAMES PLAYED:")?,
            dice_games_won: parse_source_scalar(&lines, "DICE GAMES WON:")?,
            dice_games_lost: parse_source_scalar(&lines, "DICE GAMES LOST:")?,
            ..Self::default()
        };
        stats.level_deaths = parse_source_table(&lines, "LEVEL        DEATHS", 16)?
            .try_into()
            .map_err(|_| "Classic level-death table has the wrong size")?;
        stats.enemy_deaths = parse_source_table(&lines, "ENEMY        DEATHS", 24)?
            .try_into()
            .map_err(|_| "Classic enemy-death table has the wrong size")?;
        stats.misc_deaths = parse_source_table(&lines, "MISC         DEATHS", 12)?
            .try_into()
            .map_err(|_| "Classic miscellaneous-death table has the wrong size")?;
        let kills = parse_source_table(&lines, "ENEMY        KILLS", 23)?;
        stats.enemy_kills[..9].copy_from_slice(&kills[..9]);
        stats.total_monkey_kills = kills[9];
        stats.enemy_kills[9..].copy_from_slice(&kills[10..]);
        Ok(stats)
    }

    /// Recreate the readable `stats.txt` report produced by scrWriteStats.
    pub fn to_classic_report(&self) -> String {
        const ENEMIES: [&str; 24] = [
            " BAT        | ",
            " SNAKE      | ",
            " SPIDER     | ",
            " G. SPIDER  | ",
            " CAVEMAN    | ",
            " SKELETON   | ",
            " ZOMBIE     | ",
            " VAMPIRE    | ",
            " FROG       | ",
            " FIRE FROG  | ",
            " MANTRAP    | ",
            " PIRANHA    | ",
            " MEGAMOUTH  | ",
            " YETI       | ",
            " YETI KING  | ",
            " ALIEN      | ",
            " UFO        | ",
            " ALIEN BOSS | ",
            " HAWKMAN    | ",
            " SHOPKEEPER | ",
            " TOMB LORD  | ",
            " MAGMA MAN  | ",
            " OLMEC      | ",
            " GHOST      | ",
        ];
        const MISC: [&str; 12] = [
            " ROCK       | ",
            " EXPLOSION  | ",
            " CRUSHED    | ",
            " LONG FALL  | ",
            " SPIKES     | ",
            " BOULDER    | ",
            " ARROW TRAP | ",
            " SPEAR TRAP | ",
            " SMASH TRAP | ",
            " CEILING T. | ",
            " PIT        | ",
            " LAVA       | ",
        ];
        const KILLERS: [&str; 22] = [
            " BAT        | ",
            " SNAKE      | ",
            " SPIDER     | ",
            " G. SPIDER  | ",
            " CAVEMAN    | ",
            " SKELETON   | ",
            " ZOMBIE     | ",
            " VAMPIRE    | ",
            " FROG       | ",
            " FIRE FROG  | ",
            " MANTRAP    | ",
            " PIRANHA    | ",
            " MEGAMOUTH  | ",
            " YETI       | ",
            " YETI KING  | ",
            " ALIEN      | ",
            " UFO        | ",
            " ALIEN BOSS | ",
            " HAWKMAN    | ",
            " SHOPKEEPER | ",
            " TOMB LORD  | ",
            " OLMEC      | ",
        ];

        let mut report = String::new();
        report.push_str(
            "+---------------------------+\n\
             |  SPELUNKY EXTENDED STATS  |\n\
             +---------------------------+\n\n",
        );
        for (label, value) in [
            ("CRATES OPENED:      ", self.total_crates),
            ("CHESTS OPENED:      ", self.total_chests),
            ("IDOLS GRABBED:      ", self.idols_grabbed),
            ("IDOLS CONVERTED:    ", self.idols_converted),
            ("DAMSELS GRABBED:    ", self.damsels_grabbed),
            ("KISSES BOUGHT:      ", self.kisses_bought),
            ("DAMSELS BOUGHT:     ", self.damsels_bought),
            ("DAMSELS SAVED:      ", self.damsels_saved_total),
            ("DAMSELS KILLED:     ", self.damsels_killed_total),
            ("ITEMS BOUGHT:       ", self.items_bought),
            ("ITEMS STOLEN:       ", self.items_stolen),
            ("DICE GAMES PLAYED:  ", self.dice_games_played),
            ("DICE GAMES WON:     ", self.dice_games_won),
            ("DICE GAMES LOST:    ", self.dice_games_lost),
        ] {
            let _ = writeln!(report, "{label}{value}");
            if matches!(
                label,
                "CHESTS OPENED:      "
                    | "IDOLS CONVERTED:    "
                    | "DAMSELS KILLED:     "
                    | "ITEMS STOLEN:       "
            ) {
                report.push('\n');
            }
        }
        report.push('\n');

        report.push_str("LEVEL        DEATHS\n------------+------------\n");
        for (index, value) in self.level_deaths.iter().enumerate() {
            let _ = writeln!(report, " {:<11}| {value}", index + 1);
        }
        report.push_str("------------+------------\n");
        let level_deaths = self
            .level_deaths
            .iter()
            .fold(0_u64, |sum, n| sum.saturating_add(*n));
        let _ = writeln!(report, " TOTAL      | {level_deaths}\n");

        write_classic_table(
            &mut report,
            "ENEMY        DEATHS",
            &ENEMIES,
            &self.enemy_deaths,
        );
        write_classic_table(&mut report, "MISC         DEATHS", &MISC, &self.misc_deaths);

        report.push_str("ENEMY        KILLS\n------------+------------\n");
        let mut total_kills = 0_u64;
        for (index, (label, value)) in KILLERS.iter().zip(self.enemy_kills.iter()).enumerate() {
            let label = if *value == 0 { " ???        | " } else { label };
            let _ = writeln!(report, "{label}{value}");
            total_kills = total_kills.saturating_add(*value);
            if index == 8 {
                let monkey_label = if self.total_monkey_kills == 0 {
                    " ???        | "
                } else {
                    " MONKEY     | "
                };
                let _ = writeln!(report, "{monkey_label}{}", self.total_monkey_kills);
                total_kills = total_kills.saturating_add(self.total_monkey_kills);
            }
        }
        report.push_str("------------+------------\n");
        let _ = writeln!(report, " TOTAL      | {total_kills}");
        report
    }
}

fn parse_source_scalar(lines: &[&str], label: &str) -> Result<u64, String> {
    let line = lines
        .iter()
        .find(|line| line.starts_with(label))
        .ok_or_else(|| format!("Classic stats report is missing {label}"))?;
    parse_source_count(line[label.len()..].trim(), label)
}

fn parse_source_table(lines: &[&str], heading: &str, count: usize) -> Result<Vec<u64>, String> {
    let heading_index = lines
        .iter()
        .position(|line| *line == heading)
        .ok_or_else(|| format!("Classic stats report is missing {heading} table"))?;
    if lines.get(heading_index + 1) != Some(&"------------+------------") {
        return Err(format!("Classic {heading} table has an invalid divider"));
    }
    (0..count)
        .map(|index| {
            let line = lines
                .get(heading_index + 2 + index)
                .ok_or_else(|| format!("Classic {heading} table is truncated"))?;
            let (_, value) = line
                .split_once('|')
                .ok_or_else(|| format!("Classic {heading} row {} is invalid", index + 1))?;
            parse_source_count(value.trim(), heading)
        })
        .collect()
}

fn parse_source_count(value: &str, label: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|_| format!("Classic stats value for {label} is not an unsigned integer"))
}

fn write_classic_table<const N: usize>(
    report: &mut String,
    heading: &str,
    labels: &[&str; N],
    counts: &[u64; N],
) {
    let _ = writeln!(report, "{heading}\n------------+------------");
    for (label, value) in labels.iter().zip(counts) {
        let label = if *value == 0 { " ???        | " } else { label };
        let _ = writeln!(report, "{label}{value}");
    }
    report.push('\n');
}

/// Source isRealLevel is a room-resource check. World.name is user-facing in
/// this port, so use logical contexts, preserving source IDs in imported fixtures.
pub fn is_real_level(game: &Game) -> bool {
    game.is_real_level()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RulesPreset;
    #[test]
    fn source_index_layout_keeps_monkey_and_magma_distinctions() {
        let mut stats = LifetimeStats::default();
        stats.enemy_kill(EnemyKill::Olmec);
        stats.enemy_death(EnemyDeath::MagmaMan);
        stats.enemy_death(EnemyDeath::Olmec);
        stats.enemy_death(EnemyDeath::Ghost);
        stats.increment(LifetimeCounter::TotalMonkeyKills);
        stats.increment(LifetimeCounter::DamselsKilledTotal);
        assert_eq!(stats.enemy_kills[21], 1);
        assert_eq!(&stats.enemy_deaths[21..], &[1, 1, 1]);
        assert_eq!(stats.total_enemy_kills(), 2);
    }
    #[test]
    fn missing_and_partial_statistics_default_without_losing_known_fields() {
        let empty: LifetimeStats = serde_json::from_str("{}").unwrap();
        assert_eq!(empty, LifetimeStats::default());
        let partial: LifetimeStats = serde_json::from_str(r#"{"damsels_saved_total":9}"#).unwrap();
        assert_eq!(partial.damsels_saved_total, 9);
        assert_eq!(partial.enemy_kills, [0; 22]);
        assert_eq!(
            serde_json::from_str::<LifetimeStats>(&serde_json::to_string(&partial).unwrap())
                .unwrap(),
            partial
        );
    }

    fn source_report_fixture() -> String {
        let mut report = String::from(
            "+---------------------------+\n\
             |  SPELUNKY EXTENDED STATS  |\n\
             +---------------------------+\n\n\
             CRATES OPENED:      7\n\
             CHESTS OPENED:      8\n\
             \n\
             IDOLS GRABBED:      9\n\
             IDOLS CONVERTED:    10\n\
             \n\
             DAMSELS GRABBED:    11\n\
             KISSES BOUGHT:      12\n\
             DAMSELS BOUGHT:     13\n\
             DAMSELS SAVED:      14\n\
             DAMSELS KILLED:     15\n\
             \n\
             ITEMS BOUGHT:       16\n\
             ITEMS STOLEN:       17\n\
             \n\
             DICE GAMES PLAYED:  18\n\
             DICE GAMES WON:     19\n\
             DICE GAMES LOST:    20\n\
             \n\
             LEVEL        DEATHS\n\
             ------------+------------\n",
        );
        for index in 0..16 {
            let _ = writeln!(report, " {:<11}| {}", index + 1, u64::from(index == 0));
        }
        report.push_str("------------+------------\n TOTAL      | 1\n\nENEMY        DEATHS\n------------+------------\n");
        for index in 0..24 {
            let value = u64::from(index == 23);
            let label = if value == 0 {
                " ???        | "
            } else {
                " GHOST      | "
            };
            let _ = writeln!(report, "{label}{value}");
        }
        report.push_str("\nMISC         DEATHS\n------------+------------\n");
        for index in 0..12 {
            let value = u64::from(index == 11);
            let label = if value == 0 {
                " ???        | "
            } else {
                " LAVA       | "
            };
            let _ = writeln!(report, "{label}{value}");
        }
        report.push_str("\nENEMY        KILLS\n------------+------------\n");
        for index in 0..22 {
            let value = u64::from(index == 21);
            let label = if value == 0 {
                " ???        | "
            } else {
                " OLMEC      | "
            };
            let _ = writeln!(report, "{label}{value}");
            if index == 8 {
                report.push_str(" MONKEY     | 2\n");
            }
        }
        report.push_str("------------+------------\n TOTAL      | 3\n");
        report
    }

    #[test]
    fn imports_actual_classic_report_order_including_zero_rows_and_monkey_row() {
        let stats = LifetimeStats::from_classic_report(&source_report_fixture()).unwrap();

        assert_eq!(stats.total_crates, 7);
        assert_eq!(stats.dice_games_lost, 20);
        assert_eq!(stats.level_deaths[0], 1);
        assert_eq!(stats.enemy_deaths[23], 1);
        assert_eq!(stats.misc_deaths[11], 1);
        assert_eq!(stats.enemy_kills[8], 0);
        assert_eq!(stats.total_monkey_kills, 2);
        assert_eq!(stats.enemy_kills[21], 1);
        assert_eq!(stats.total_enemy_kills(), 3);
    }

    #[test]
    fn classic_report_formatter_matches_source_layout_and_round_trips() {
        let mut expected = LifetimeStats {
            total_crates: 7,
            total_chests: 8,
            idols_grabbed: 9,
            idols_converted: 10,
            damsels_grabbed: 11,
            kisses_bought: 12,
            damsels_bought: 13,
            damsels_saved_total: 14,
            damsels_killed_total: 15,
            items_bought: 16,
            items_stolen: 17,
            dice_games_played: 18,
            dice_games_won: 19,
            dice_games_lost: 20,
            ..LifetimeStats::default()
        };
        expected.level_deaths[0] = 1;
        expected.enemy_deaths[23] = 1;
        expected.misc_deaths[11] = 1;
        expected.total_monkey_kills = 2;
        expected.enemy_kills[21] = 1;

        let report = expected.to_classic_report();
        assert!(report.starts_with(
            "+---------------------------+\n|  SPELUNKY EXTENDED STATS  |\n+---------------------------+\n\n"
        ));
        assert!(report.contains("CRATES OPENED:      7\nCHESTS OPENED:      8\n\n"));
        assert!(report.contains(" 1          | 1\n"));
        assert!(report.contains(" ???        | 0\n"));
        assert!(report.contains(" MONKEY     | 2\n"));
        assert!(report.ends_with("------------+------------\n TOTAL      | 3\n"));
        assert_eq!(
            LifetimeStats::from_classic_report(&report).unwrap(),
            expected
        );
    }

    #[test]
    fn classic_stats_parser_rejects_invalid_and_truncated_reports() {
        assert!(LifetimeStats::from_classic_report("not a stats file").is_err());
        let report = source_report_fixture();
        let truncated = report.lines().take(35).collect::<Vec<_>>().join("\n");
        assert!(LifetimeStats::from_classic_report(&truncated).is_err());
        let invalid = report.replacen("CRATES OPENED:      7", "CRATES OPENED:      -7", 1);
        assert!(LifetimeStats::from_classic_report(&invalid).is_err());
    }
    #[test]
    fn real_room_eligibility_excludes_noncampaign_contexts() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        assert!(is_real_level(&game));
        for flag in ["custom_level", "tutorial", "hub", "hub_stars"] {
            game.flags.insert(flag.into());
            assert!(!is_real_level(&game), "{flag}");
            game.flags.remove(flag);
        }
        for name in ["rLevel", "rLevel2", "rLevel3", "rOlmec"] {
            game.world.name = name.into();
            assert!(is_real_level(&game));
        }
        for name in ["rTutorial", "rLoadLevel", "rTitle", "rTransition1", "rEnd"] {
            game.world.name = name.into();
            assert!(!is_real_level(&game));
        }
    }
}
