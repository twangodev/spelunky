use crate::*;

impl Game {
    pub fn new(seed: u64, rules: RulesPreset) -> Self {
        let mut game = Self::shell(seed, rules);
        game.world = generation::generate(&mut game.rng, 1, &game.flags);
        game.reset_position();
        game
    }

    pub fn tutorial(seed: u64, rules: RulesPreset) -> Self {
        let mut game = Self::shell(seed, rules);
        game.world = generation::tutorial();
        game.flags.insert("tutorial".into());
        game.reset_position();
        game
    }

    pub fn load_custom(&mut self, level: &level::Level, name: &str, previous: &str) {
        self.world = level.to_world();
        self.world.name = name.to_owned();
        self.flags.insert("custom".into());
        self.flags.insert("custom_level".into());
        self.flags.remove("tutorial");
        self.flags
            .retain(|f| !f.starts_with("custom_level:") && !f.starts_with("default_next_custom:"));
        self.player.health = level.starting_life.clamp(1, 99);
        self.player.bombs = level.starting_bombs.clamp(0, 99);
        self.player.ropes = level.starting_ropes.clamp(0, 99);
        self.world.entrance = self
            .world
            .entities
            .iter()
            .find(|e| {
                e.object == "oEntrance" && e.text.get("leadsTo").is_some_and(|s| s == previous)
            })
            .map(|e| Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0))
            .unwrap_or(self.world.entrance);
        self.data.insert(
            "custom_music".into(),
            match level.music.to_uppercase().as_str() {
                "LUSH" => 1.0,
                "ICE" => 2.0,
                "TEMPLE" => 3.0,
                "BOSS" => 4.0,
                _ => 0.0,
            },
        );
        if !level.next_level.is_empty() && level.next_level != "NONE" {
            self.flags
                .insert(format!("default_next_custom:{}", level.next_level));
        }
        self.reset_position();
    }

    pub fn load_hub(&mut self, kind: hub::HubKind) {
        self.flags.retain(|flag| {
            !flag.starts_with("hub_")
                && !flag.starts_with("hub_destination:")
                && !flag.starts_with("custom_level:")
                && !flag.starts_with("default_next_custom:")
                && !matches!(flag.as_str(), "custom" | "custom_level" | "tutorial")
        });
        self.data.retain(|key, _| {
            !key.starts_with("hub_")
                && !key.starts_with("sun_")
                && !key.starts_with("moon_")
                && !key.starts_with("stars_")
        });
        self.world = hub::world(kind, &self.flags);
        self.reset_position();
        hub::initialize(self, kind);
        self.player.previous_pos = self.player.pos;
        self.update_camera();
        self.previous_camera = self.camera;
    }

    fn reset_position(&mut self) {
        self.player.pos = self.world.entrance;
        self.player.previous_pos = self.player.pos;
        self.player.vel = Vec2::default();
        self.player.state = PlayerState::Falling;
        self.player.grounded = false;
        self.player.held = None;
        self.player.stun = 0;
        self.player.attack_timer = 0;
        self.player.jump_time = 0;
        self.player.coyote_ticks = 0;
        self.player.buffered_jump_ticks = 0;
        self.player.animation = 0.0;
        self.player.data.clear();
        self.player.invincible = 0;
        self.data.retain(|key, _| {
            !key.starts_with("hazard:") && !key.starts_with("altar:") && !key.starts_with("tile:")
        });
        self.level_tick = 0;
        self.state = RunState::Playing;
        self.events.clear();
        for e in &mut self.world.entities {
            enemies::init_entity(e);
            items::init_entity(e);
            e.previous_pos = e.pos;
        }
        self.update_camera();
        self.previous_camera = self.camera;
    }

    /// Advance exactly one original 1/30-second tick. Rendering never calls RNG.
    pub fn step(&mut self, input: &InputFrame) {
        if self.state != RunState::Playing {
            return;
        }
        self.events.clear();
        self.flags.remove("render_discontinuity");
        self.player.previous_pos = self.player.pos;
        self.previous_camera = self.camera;
        for e in &mut self.world.entities {
            enemies::init_entity(e);
            items::init_entity(e);
            e.previous_pos = e.pos;
            e.age = e.age.saturating_add(1);
        }
        self.tick += 1;
        self.level_tick += 1;
        traps::pre_tick(self);
        player::tick(self, input);
        // Classic's dedicated inventory keys are gated by isLevel(), while
        // attack and pickup remain available in the authored hub/challenges.
        let mut item_input = *input;
        if self.flags.contains("hub") {
            item_input.cycle_pressed = false;
            item_input.bomb_pressed = false;
            item_input.rope_pressed = false;
        }
        items::tick(self, &item_input);
        enemies::tick(self);
        traps::tick(self);
        world_events::tick(self, input);
        hub::tick(self, input);
        if self.player.pos.y > self.world.height as f64 * 16.0 + 16.0 {
            self.player.health = 0;
        }
        economy::tick(self, input);
        progression::tick(self, input);
        if self.player.health <= 0 {
            self.player.health = 0;
            self.player.state = PlayerState::Dead;
            self.state = RunState::Dead;
            self.events.push(GameEvent::Died);
            self.sound("sndDie", self.player.pos);
        }
        self.world.entities.retain(|e| e.alive);
        self.update_camera();
    }

    pub fn next_level(&mut self) {
        if self.flags.contains("tutorial") {
            self.state = RunState::Won;
            self.events.push(GameEvent::Won);
            return;
        }
        let carry = self
            .flags
            .iter()
            .find_map(|f| f.strip_prefix("carry:").map(str::to_owned));
        let carried = carry.as_ref().and_then(|name| {
            self.world
                .entities
                .iter()
                .find(|e| e.object == *name && e.held)
                .cloned()
        });
        self.flags.retain(|f| !f.starts_with("carry:"));
        let rescued = self.flags.remove("damsel_rescued");
        let next = self
            .data
            .remove("next_level")
            .unwrap_or(f64::from(self.world.level + 1)) as u32;
        if next > 16 {
            self.state = RunState::Won;
            self.events.push(GameEvent::Won);
            return;
        }
        self.world = generation::generate(&mut self.rng, next, &self.flags);
        self.flags.retain(|f| {
            !matches!(
                f.as_str(),
                "black_market"
                    | "genBlackMarket"
                    | "blackMarket"
                    | "city_of_gold"
                    | "cityOfGold"
                    | "snake_pit"
                    | "snakePit"
                    | "cemetery"
                    | "cemetary"
                    | "lake"
                    | "dark"
                    | "darkLevel"
                    | "yeti_lair"
                    | "yetiLair"
                    | "alien_ship"
                    | "alienCraft"
                    | "sacrifice_pit"
                    | "sacrificePit"
            )
        });
        for e in &self.world.entities {
            match e.object.as_str() {
                "oLockedChest" => {
                    self.flags.insert("made_udjat_eye".into());
                }
                "oMoai" => {
                    self.flags.insert("made_moai".into());
                }
                "oXMarket" => {
                    self.flags.insert("made_market_entrance".into());
                }
                _ => {}
            }
        }
        self.reset_position();
        if rescued {
            self.player.health = (self.player.health + 1).min(99);
            self.sound("sndKiss", self.player.pos);
        }
        if let Some(object) = carry {
            let id = self.world.spawn(object, self.player.pos);
            if let Some(mut item) = carried {
                item.id = id;
                item.pos = self.player.pos;
                item.previous_pos = item.pos;
                item.vel = Vec2::default();
                item.alive = true;
                item.held = true;
                *self.world.entity_mut(id).expect("carried item") = item;
            } else if let Some(item) = self.world.entity_mut(id) {
                item.held = true;
            }
            self.player.held = Some(id);
        }
        if !self.flags.contains("wanted") && !self.flags.contains("murderer") {
            self.flags.remove("shopkeepers_angry");
        }
    }

    pub fn update_camera(&mut self) {
        if self.flags.contains("hub_title") {
            let target = if self.player.pos.x <= 320.0 {
                0.0
            } else {
                320.0
            };
            self.camera.x += (target - self.camera.x).clamp(-8.0, 8.0);
            self.camera.y = 0.0;
            return;
        }
        if self.flags.contains("hub") {
            self.camera = Vec2::default();
            return;
        }
        let relative = Vec2::new(
            self.player.pos.x - self.camera.x,
            self.player.pos.y - self.camera.y,
        );
        if relative.x < 128.0 {
            self.camera.x = self.player.pos.x - 128.0;
        } else if relative.x > VIEW_WIDTH - 128.0 {
            self.camera.x = self.player.pos.x - (VIEW_WIDTH - 128.0);
        }
        if relative.y < 96.0 {
            self.camera.y = self.player.pos.y - 96.0;
        } else if relative.y > VIEW_HEIGHT - 96.0 {
            self.camera.y = self.player.pos.y - (VIEW_HEIGHT - 96.0);
        }
        self.camera.x = self
            .camera
            .x
            .clamp(0.0, (self.world.width as f64 * 16.0 - VIEW_WIDTH).max(0.0));
        self.camera.y = self.camera.y.clamp(
            0.0,
            (self.world.height as f64 * 16.0 - VIEW_HEIGHT).max(0.0),
        );
    }

    /// Stable diagnostic digest of serialized authoritative state (not security).
    pub fn state_hash(&self) -> u64 {
        serde_json::to_vec(self)
            .expect("finite game state")
            .into_iter()
            .fold(0xcbf29ce484222325, |hash, byte| {
                (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn seeded_tick_replay_is_reproducible() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            let mut a = Game::new(123, rules);
            let mut b = Game::new(123, rules);
            for tick in 0..300 {
                let input = InputFrame {
                    right: tick % 80 < 40,
                    jump: tick % 30 < 8,
                    jump_pressed: tick % 30 == 0,
                    bomb_pressed: tick == 100,
                    ..InputFrame::default()
                };
                a.step(&input);
                b.step(&input);
                assert_eq!(a.state_hash(), b.state_hash());
            }
        }
    }
    #[test]
    fn transitions_reset_interpolation_and_preserve_inventory() {
        let mut game = Game::new(42, RulesPreset::Classic);
        game.player.money = 500;
        game.next_level();
        assert_eq!(game.world.level, 2);
        assert_eq!(game.player.money, 500);
        assert_eq!(game.player.pos, game.player.previous_pos);
        assert_eq!(game.camera, game.previous_camera);
    }
    fn flat_game() -> Game {
        let mut game = Game::shell(7, RulesPreset::Classic);
        for x in 0..game.world.width {
            game.world.set_tile(x, 3, "oBlock");
        }
        game.player.pos = Vec2::new(40.0, 40.0);
        game.player.previous_pos = game.player.pos;
        game
    }
    #[test]
    fn gem_collection_delay_advances_in_real_simulation() {
        let mut game = flat_game();
        let id = game.world.spawn("oRubyBig", Vec2::new(32.0, 32.0));
        game.world
            .entity_mut(id)
            .unwrap()
            .data
            .insert("gravity".into(), 0.0);
        for _ in 0..19 {
            game.step(&InputFrame::default());
        }
        assert!(game.world.entity(id).is_some());
        for _ in 0..5 {
            game.step(&InputFrame::default());
        }
        assert!(game.world.entity(id).is_none());
        assert!(game.data.get("collect").copied().unwrap_or(0.0) > 0.0 || game.player.money > 0);
    }
    #[test]
    fn abyss_death_and_ankh_revival_use_same_integrated_tick() {
        let mut game = flat_game();
        game.player.pos.y = game.world.height as f64 * 16.0 + 32.0;
        game.step(&InputFrame::default());
        assert_eq!(game.state, RunState::Dead);
        let mut game = flat_game();
        game.player.equipment.insert("Ankh".into());
        game.player.pos.y = game.world.height as f64 * 16.0 + 32.0;
        game.step(&InputFrame::default());
        assert_eq!(game.state, RunState::Playing);
        assert_eq!(game.player.health, 4);
        assert!(!game.player.has("Ankh"));
        assert_eq!(game.player.pos, game.player.previous_pos);
    }
    #[test]
    fn transition_restores_carried_weapon_and_rescue_health() {
        let mut game = Game::new(7, RulesPreset::Classic);
        let id = game.world.spawn("oShotgun", game.player.pos);
        game.world.entity_mut(id).unwrap().held = true;
        game.world
            .entity_mut(id)
            .unwrap()
            .data
            .insert("ammo".into(), 17.0);
        game.player.held = Some(id);
        game.flags.insert("carry:oShotgun".into());
        game.flags.insert("damsel_rescued".into());
        game.player.health = 2;
        game.next_level();
        assert_eq!(game.player.health, 3);
        let held = game.world.entity(game.player.held.unwrap()).unwrap();
        assert_eq!(held.object, "oShotgun");
        assert_eq!(held.value("ammo"), 17.0);
        assert!(held.held);
    }
}
