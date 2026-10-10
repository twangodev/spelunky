//! Original extended statistics. Eligibility and timing remain at source producers.
use crate::Game;
use serde::{Deserialize, Serialize};

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
