//! Explicit, session-only helpers for sandbox play.
use crate::{Game, PlayerState, RunState, Vec2, catalog};

#[derive(Debug, Clone, Copy, Default)]
pub struct Cheats {
    pub infinite_health: bool,
    pub infinite_bombs: bool,
    pub infinite_ropes: bool,
    pub infinite_money: bool,
    pub free_shops: bool,
}

impl Cheats {
    /// Called on both sides of a simulation tick and immediately after editing.
    pub fn apply(&self, game: &mut Game) {
        game.player.cheat_invulnerable = self.infinite_health;
        if self.infinite_health
            || self.infinite_bombs
            || self.infinite_ropes
            || self.infinite_money
            || self.free_shops
        {
            game.flags.insert("cheats_used".into());
        }
        if self.infinite_health && game.state == RunState::Playing {
            game.player.health = 99;
            // A bottomless pit has no landing to recover on. Return to the
            // actual room entrance without replaying room creation.
            if game.player.pos.y > game.world.height as f64 * 16.0 + 16.0 {
                game.player.pos = game.world.entrance;
                game.player.previous_pos = game.player.pos;
                game.player.vel = Vec2::default();
                game.player.state = PlayerState::Falling;
                game.player.stun = 0;
                game.player.data.insert("source_stunned".into(), 0.0);
                game.flags.insert("render_discontinuity".into());
            }
        }
        if self.infinite_bombs {
            game.player.bombs = 99;
        }
        if self.infinite_ropes {
            game.player.ropes = 99;
        }
        if self.infinite_money {
            game.player.money = 999_999;
        }
        if self.free_shops {
            for entity in &mut game.world.entities {
                if entity.alive && (entity.value("cost") > 0.0 || entity.value("forSale") != 0.0) {
                    entity.data.insert("cost".into(), 0.0);
                    entity.data.insert("forSale".into(), 0.0);
                }
            }
        }
    }

    pub fn refill(game: &mut Game) {
        game.flags.insert("cheats_used".into());
        game.player.health = 99;
        game.player.bombs = 99;
        game.player.ropes = 99;
        game.player.money = 999_999;
    }

    pub fn calm_shopkeepers(game: &mut Game) {
        game.flags.insert("cheats_used".into());
        for flag in ["wanted", "murderer", "thief", "shopkeepers_angry"] {
            game.flags.remove(flag);
        }
        for key in ["thiefLevel", "thief_level"] {
            game.data.insert(key.into(), 0.0);
        }
        for entity in &mut game.world.entities {
            if entity.alive && entity.health > 0 && catalog::is_a(&entity.object, "oShopkeeper") {
                entity.state = 0;
                entity.vel.x = 0.0;
                for key in ["angry", "angered", "running", "firing", "counter"] {
                    entity.data.insert(key.into(), 0.0);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{InputFrame, RulesPreset};

    #[test]
    fn cheats_disabled_preserve_normal_damage_and_serialization() {
        let mut game = Game::new(7, RulesPreset::Classic);
        let before = serde_json::to_value(&game).unwrap();
        Cheats::default().apply(&mut game);
        assert_eq!(serde_json::to_value(&game).unwrap(), before);
        game.damage_player(1);
        assert_eq!(game.player.health, 3);
    }

    #[test]
    fn infinite_health_blocks_contact_and_direct_lethal_damage_then_can_be_disabled() {
        let mut game = Game::new(7, RulesPreset::Classic);
        let mut cheats = Cheats {
            infinite_health: true,
            ..Default::default()
        };
        cheats.apply(&mut game);
        game.damage_player(1000);
        game.player.lose_health(99);
        assert_eq!(game.player.health, 99);
        assert_eq!(game.player.stun, 0);
        cheats.infinite_health = false;
        cheats.apply(&mut game);
        game.damage_player(1);
        assert_eq!(game.player.health, 98);
        assert!(game.flags.contains("cheats_used"));
    }

    #[test]
    fn infinite_bombs_and_ropes_refill_after_actual_use() {
        let mut game = Game::new(7, RulesPreset::Classic);
        let cheats = Cheats {
            infinite_bombs: true,
            infinite_ropes: true,
            ..Default::default()
        };
        cheats.apply(&mut game);
        game.step(&InputFrame {
            bomb_pressed: true,
            ..Default::default()
        });
        assert!(game.world.entities.iter().any(|e| e.object == "oBomb"));
        cheats.apply(&mut game);
        assert_eq!(game.player.bombs, 99);
        game.step(&InputFrame {
            rope_pressed: true,
            ..Default::default()
        });
        cheats.apply(&mut game);
        assert_eq!(game.player.ropes, 99);
    }

    #[test]
    fn free_shops_clear_actual_sale_fields_and_calm_clears_bounty() {
        let mut game = Game::new(7, RulesPreset::Classic);
        let item = game.world.spawn("oBombBag", Vec2::new(100.0, 100.0));
        game.world
            .entity_mut(item)
            .unwrap()
            .data
            .insert("cost".into(), 2500.0);
        game.world
            .entity_mut(item)
            .unwrap()
            .data
            .insert("forSale".into(), 1.0);
        let keeper = game.world.spawn("oShopkeeper", Vec2::new(120.0, 100.0));
        game.world.entity_mut(keeper).unwrap().state = 2;
        game.world
            .entity_mut(keeper)
            .unwrap()
            .data
            .insert("angry".into(), 1.0);
        game.flags.insert("wanted".into());
        game.data.insert("thiefLevel".into(), 5.0);
        Cheats {
            free_shops: true,
            ..Default::default()
        }
        .apply(&mut game);
        assert_eq!(game.world.entity(item).unwrap().value("cost"), 0.0);
        assert_eq!(game.world.entity(item).unwrap().value("forSale"), 0.0);
        Cheats::calm_shopkeepers(&mut game);
        assert!(!game.flags.contains("wanted"));
        assert_eq!(game.data["thiefLevel"], 0.0);
        assert_eq!(game.world.entity(keeper).unwrap().state, 0);
        assert_eq!(game.world.entity(keeper).unwrap().value("angry"), 0.0);
    }
}
