//! Explicit, session-only helpers for sandbox play.
use crate::{Game, PlayerState, RunState, Vec2, catalog};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct ShopWorldScope {
    seed: u64,
    level: u32,
    area: u8,
    width: usize,
    height: usize,
    name: String,
}

#[derive(Debug, Clone, Copy, Default)]
struct SaleFields {
    cost: Option<f64>,
    for_sale: Option<f64>,
    acquired: bool,
}

type ShopStockKey = (ShopWorldScope, u64, u64);

#[derive(Debug, Clone, Default)]
pub struct Cheats {
    pub infinite_health: bool,
    pub infinite_bombs: bool,
    pub infinite_ropes: bool,
    pub infinite_money: bool,
    pub free_shops: bool,
    original_shop_sales: BTreeMap<ShopStockKey, SaleFields>,
}

impl Cheats {
    /// Called on both sides of a simulation tick and immediately after editing.
    pub fn apply(&mut self, game: &mut Game) {
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
            game.player.health = 99.0;
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
            game.player.bombs = 99.0;
        }
        if self.infinite_ropes {
            game.player.ropes = 99.0;
        }
        if self.infinite_money {
            game.player.money = 999_999;
        }
        self.apply_free_shops(game);
    }

    fn apply_free_shops(&mut self, game: &mut Game) {
        let scope = ShopWorldScope {
            seed: game.seed,
            level: game.world.level,
            area: game.world.area as u8,
            width: game.world.width,
            height: game.world.height,
            name: game.world.name.clone(),
        };
        let stock_acquired: BTreeMap<_, _> = game
            .world
            .entities
            .iter()
            .filter(|entity| entity.alive && catalog::is_a(&entity.object, "oItem"))
            .map(|entity| {
                (
                    (entity.id, entity.creation_serial),
                    stock_is_acquired(game, entity),
                )
            })
            .collect();

        if self.free_shops {
            for entity in &mut game.world.entities {
                if !entity.alive || !catalog::is_a(&entity.object, "oItem") {
                    continue;
                }

                let key = (scope.clone(), entity.id, entity.creation_serial);
                if let Some(original) = self.original_shop_sales.get_mut(&key) {
                    original.acquired |= stock_acquired
                        .get(&(entity.id, entity.creation_serial))
                        .copied()
                        .unwrap_or(false);
                    // Keep the cheat active if game logic updates a tracked
                    // sale item while the menu toggle remains on.
                    entity.data.insert("cost".into(), 0.0);
                    entity.data.insert("forSale".into(), 0.0);
                    continue;
                }

                // The sale flag identifies live shop stock. Items that
                // merely carry a cost field are not made free.
                if entity.value("forSale") == 0.0 {
                    continue;
                }
                self.original_shop_sales.insert(
                    key,
                    SaleFields {
                        cost: entity.data.get("cost").copied(),
                        for_sale: entity.data.get("forSale").copied(),
                        acquired: stock_acquired
                            .get(&(entity.id, entity.creation_serial))
                            .copied()
                            .unwrap_or(false),
                    },
                );
                entity.data.insert("cost".into(), 0.0);
                entity.data.insert("forSale".into(), 0.0);
            }
        } else {
            for entity in &mut game.world.entities {
                let key = (scope.clone(), entity.id, entity.creation_serial);
                if let Some(original) = self.original_shop_sales.remove(&key) {
                    let acquired = stock_acquired
                        .get(&(entity.id, entity.creation_serial))
                        .copied()
                        .unwrap_or(false);
                    if original.acquired || acquired {
                        // Picked-up or shop-exited stock is now player-owned
                        // (or stolen), so disabling the cheat must not restore
                        // its old sale price or availability.
                        entity.data.insert("cost".into(), 0.0);
                        entity.data.insert("forSale".into(), 0.0);
                    } else {
                        restore_field(&mut entity.data, "cost", original.cost);
                        restore_field(&mut entity.data, "forSale", original.for_sale);
                    }
                }
            }
            // Snapshots from earlier rooms refer to entities that no longer
            // exist; discard them without applying them to this room's IDs.
            self.original_shop_sales.clear();
        }
    }

    pub fn refill(game: &mut Game) {
        game.flags.insert("cheats_used".into());
        game.player.health = 99.0;
        game.player.bombs = 99.0;
        game.player.ropes = 99.0;
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

fn stock_is_acquired(game: &Game, entity: &crate::Entity) -> bool {
    entity.held || game.player.held == Some(entity.id) || !crate::economy::in_shop(game, entity.pos)
}

fn restore_field(data: &mut BTreeMap<String, f64>, key: &str, value: Option<f64>) {
    if let Some(value) = value {
        data.insert(key.into(), value);
    } else {
        data.remove(key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Area, InputFrame, PlayerState, RulesPreset, World};

    #[test]
    fn cheats_disabled_preserve_normal_damage_and_serialization() {
        let mut game = Game::new(7, RulesPreset::Classic);
        let before = serde_json::to_value(&game).unwrap();
        Cheats::default().apply(&mut game);
        assert_eq!(serde_json::to_value(&game).unwrap(), before);
        game.damage_player(1);
        assert_eq!(game.player.health, 3.0);
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
        assert_eq!(game.player.health, 99.0);
        assert_eq!(game.player.stun, 0);
        cheats.infinite_health = false;
        cheats.apply(&mut game);
        game.damage_player(1);
        assert_eq!(game.player.health, 98.0);
        assert!(game.flags.contains("cheats_used"));
    }

    #[test]
    fn infinite_bombs_and_ropes_refill_after_actual_use() {
        let mut game = Game::new(7, RulesPreset::Classic);
        let mut cheats = Cheats {
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
        assert_eq!(game.player.bombs, 99.0);
        game.step(&InputFrame {
            rope_pressed: true,
            ..Default::default()
        });
        cheats.apply(&mut game);
        assert_eq!(game.player.ropes, 99.0);
    }

    #[test]
    fn free_shops_restore_stock_fields_on_disable_and_repeated_cycles() {
        let mut game = Game::shell(7, RulesPreset::Classic);
        game.world = World::new(30, 20, Area::Mines, 1);
        game.world.name = "rLevel".into();
        game.world.spawn("oShopkeeper", Vec2::new(120.0, 100.0));
        let item = game.world.spawn("oBombBag", Vec2::new(100.0, 100.0));
        let item_data = &mut game.world.entity_mut(item).unwrap().data;
        item_data.insert("cost".into(), 2500.0);
        item_data.insert("forSale".into(), 1.0);

        let stock_without_original_cost = game.world.spawn("oRopePile", Vec2::new(120.0, 100.0));
        let stock_data = &mut game
            .world
            .entity_mut(stock_without_original_cost)
            .unwrap()
            .data;
        stock_data.remove("cost");
        stock_data.insert("forSale".into(), 2.5);

        let priced_but_not_for_sale = game.world.spawn("oBombBag", Vec2::new(140.0, 100.0));
        let priced_item_data = &mut game.world.entity_mut(priced_but_not_for_sale).unwrap().data;
        priced_item_data.insert("cost".into(), 777.0);
        priced_item_data.remove("forSale");

        let non_item_sale_entity = game.world.spawn("oShopkeeper", Vec2::new(152.0, 100.0));
        let keeper_data = &mut game.world.entity_mut(non_item_sale_entity).unwrap().data;
        keeper_data.insert("cost".into(), 888.0);
        keeper_data.insert("forSale".into(), 1.0);

        let mut cheats = Cheats {
            free_shops: true,
            ..Default::default()
        };
        cheats.apply(&mut game);
        assert_eq!(game.world.entity(item).unwrap().value("cost"), 0.0);
        assert_eq!(game.world.entity(item).unwrap().value("forSale"), 0.0);
        assert_eq!(
            game.world
                .entity(stock_without_original_cost)
                .unwrap()
                .value("cost"),
            0.0
        );
        assert_eq!(
            game.world
                .entity(stock_without_original_cost)
                .unwrap()
                .value("forSale"),
            0.0
        );

        // Reapplying must not replace the saved original values with zeroes.
        cheats.apply(&mut game);
        let late_stock = game.world.spawn("oBombBag", Vec2::new(148.0, 100.0));
        let late_stock_data = &mut game.world.entity_mut(late_stock).unwrap().data;
        late_stock_data.insert("cost".into(), 3500.0);
        late_stock_data.insert("forSale".into(), 1.0);
        cheats.apply(&mut game);
        assert_eq!(game.world.entity(late_stock).unwrap().value("cost"), 0.0);
        assert_eq!(game.world.entity(late_stock).unwrap().value("forSale"), 0.0);

        cheats.free_shops = false;
        cheats.apply(&mut game);
        assert_eq!(game.world.entity(item).unwrap().value("cost"), 2500.0);
        assert_eq!(game.world.entity(item).unwrap().value("forSale"), 1.0);
        assert!(
            !game
                .world
                .entity(stock_without_original_cost)
                .unwrap()
                .data
                .contains_key("cost")
        );
        assert_eq!(
            game.world
                .entity(stock_without_original_cost)
                .unwrap()
                .value("forSale"),
            2.5
        );
        assert_eq!(game.world.entity(late_stock).unwrap().value("cost"), 3500.0);
        assert_eq!(game.world.entity(late_stock).unwrap().value("forSale"), 1.0);
        assert_eq!(
            game.world
                .entity(priced_but_not_for_sale)
                .unwrap()
                .value("cost"),
            777.0
        );
        assert!(
            !game
                .world
                .entity(priced_but_not_for_sale)
                .unwrap()
                .data
                .contains_key("forSale")
        );
        assert_eq!(
            game.world
                .entity(non_item_sale_entity)
                .unwrap()
                .value("cost"),
            888.0
        );
        assert_eq!(
            game.world
                .entity(non_item_sale_entity)
                .unwrap()
                .value("forSale"),
            1.0
        );

        // A later enable cycle captures the then-current values afresh.
        game.world
            .entity_mut(item)
            .unwrap()
            .data
            .insert("cost".into(), 4321.0);
        game.world
            .entity_mut(item)
            .unwrap()
            .data
            .insert("forSale".into(), 3.0);
        cheats.free_shops = true;
        cheats.apply(&mut game);
        cheats.apply(&mut game);
        cheats.free_shops = false;
        cheats.apply(&mut game);
        assert_eq!(game.world.entity(item).unwrap().value("cost"), 4321.0);
        assert_eq!(game.world.entity(item).unwrap().value("forSale"), 3.0);
    }

    #[test]
    fn free_shops_do_not_reprice_held_or_removed_stock_when_disabled() {
        let mut game = Game::shell(7, RulesPreset::Classic);
        game.world = World::new(30, 20, Area::Mines, 1);
        game.world.name = "rLevel".into();
        for x in 0..game.world.width {
            game.world.set_tile(x, 10, "oBrick");
        }
        game.player.pos = Vec2::new(96.0, 152.0);
        game.player.previous_pos = game.player.pos;
        game.player.state = PlayerState::Standing;
        game.player.grounded = true;
        game.world.spawn("oShopkeeper", Vec2::new(120.0, 152.0));

        let held_stock = game.spawn("oShotgun", game.player.pos);
        let item = game.world.entity_mut(held_stock).unwrap();
        item.data.insert("cost".into(), 5_000.0);
        item.data.insert("forSale".into(), 1.0);

        let removed_stock = game.spawn("oShotgun", Vec2::new(144.0, 152.0));
        let item = game.world.entity_mut(removed_stock).unwrap();
        item.data.insert("cost".into(), 6_000.0);
        item.data.insert("forSale".into(), 1.0);

        let mut cheats = Cheats {
            free_shops: true,
            ..Default::default()
        };
        cheats.apply(&mut game);
        game.step(&InputFrame {
            down: true,
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert_eq!(game.player.held, Some(held_stock));

        // The next host apply records that this sale item was acquired while
        // free. The other item is moved out of the source shop room.
        cheats.apply(&mut game);
        game.world.entity_mut(removed_stock).unwrap().pos = Vec2::new(260.0, 64.0);
        cheats.apply(&mut game);
        assert_eq!(game.world.entity(held_stock).unwrap().value("cost"), 0.0);
        assert_eq!(game.world.entity(removed_stock).unwrap().value("cost"), 0.0);

        cheats.free_shops = false;
        cheats.apply(&mut game);
        for id in [held_stock, removed_stock] {
            assert_eq!(game.world.entity(id).unwrap().value("cost"), 0.0);
            assert_eq!(game.world.entity(id).unwrap().value("forSale"), 0.0);
        }
    }

    #[test]
    fn calm_shopkeepers_clears_bounty() {
        let mut game = Game::new(7, RulesPreset::Classic);
        let keeper = game.world.spawn("oShopkeeper", Vec2::new(120.0, 100.0));
        game.world.entity_mut(keeper).unwrap().state = 2;
        game.world
            .entity_mut(keeper)
            .unwrap()
            .data
            .insert("angry".into(), 1.0);
        game.flags.insert("wanted".into());
        game.data.insert("thiefLevel".into(), 5.0);
        Cheats::calm_shopkeepers(&mut game);
        assert!(!game.flags.contains("wanted"));
        assert_eq!(game.data["thiefLevel"], 0.0);
        assert_eq!(game.world.entity(keeper).unwrap().state, 0);
        assert_eq!(game.world.entity(keeper).unwrap().value("angry"), 0.0);
    }
}
