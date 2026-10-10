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
        self.load_custom_room(level, name, previous, true);
    }

    /// Linked custom rooms preserve the run's inventory, as scrLoadLevel(-1)
    /// does, while restoring the light item selected by the exit handler.
    pub fn continue_custom(&mut self, level: &level::Level, name: &str, previous: &str) {
        let carry = self.take_transition_carry();
        let kisses = self.take_pending_kisses();
        self.load_custom_room(level, name, previous, false);
        self.apply_kisses(kisses);
        self.restore_transition_carry(carry);
    }

    fn load_custom_room(
        &mut self,
        level: &level::Level,
        name: &str,
        previous: &str,
        initialize_inventory: bool,
    ) {
        self.world = level.to_world();
        generation::initialize_custom_terrain(&mut self.world, &mut self.rng);
        self.world.name = name.to_owned();
        self.flags.retain(|flag| {
            flag != "hub" && !flag.starts_with("hub_") && !flag.starts_with("hub_destination:")
        });
        self.flags.insert("custom".into());
        self.flags.insert("custom_level".into());
        self.flags.remove("tutorial");
        self.flags
            .retain(|f| !f.starts_with("custom_level:") && !f.starts_with("default_next_custom:"));
        if initialize_inventory {
            self.player.health = level.starting_life.clamp(1, 99);
            self.player.bombs = level.starting_bombs.clamp(0, 99);
            self.player.ropes = level.starting_ropes.clamp(0, 99);
        }
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
        if self.state == RunState::Dead || self.player.state == PlayerState::Dead {
            self.flags.insert("reset_run_on_hub".into());
        }
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
            !key.starts_with("hazard:")
                && !key.starts_with("hazard_entity:")
                && !key.starts_with("altar:")
                && !key.starts_with("altar_tile:")
                && !key.starts_with("tile:")
                && key != "death_ticks"
                && key != "water_counter"
                && key != "water_loop_safety"
                && key != "progression_initialized"
                && key != "progression_entrance_x"
                && key != "progression_entrance_y"
        });
        self.level_tick = 0;
        self.state = RunState::Playing;
        self.events.clear();
        for e in &mut self.world.entities {
            enemies::init_entity(e);
            items::init_entity(e);
            e.previous_pos = e.pos;
        }
        // oPlayer1 Create disables its controller only in the authored rOlmec
        // room. oOlmec Create starts the zero-border boss camera at Y=400.
        if self.world.area == Area::Boss && !self.flags.contains("custom") {
            self.player.data.insert("active".into(), 0.0);
            self.camera = Vec2::new(0.0, 400.0);
        }
        self.update_camera();
        self.previous_camera = self.camera;
    }

    /// Advance exactly one original 1/30-second tick. Rendering never calls RNG.
    pub fn step(&mut self, input: &InputFrame) {
        if !matches!(self.state, RunState::Playing | RunState::Dead) {
            return;
        }
        let was_dead = self.state == RunState::Dead;
        if was_dead {
            let ticks = self.data.entry("death_ticks".into()).or_default();
            *ticks = if input.attack_pressed {
                100.0
            } else {
                *ticks + 1.0
            };
        }
        self.events.clear();
        self.flags.remove("render_discontinuity");
        self.player.previous_pos = self.player.pos;
        self.previous_camera = self.camera;
        world_events::fluid_tick(self);
        progression::initialize_level(self);
        let solids_before: Vec<_> = self
            .world
            .entities
            .iter()
            .filter(|entity| entity.alive && catalog::is_a(&entity.object, "oSolid"))
            .map(|entity| entity.id)
            .collect();
        let room_tick_before = self.level_tick;
        let frozen = activation::begin(self);
        for e in &mut self.world.entities {
            enemies::init_entity(e);
            items::init_entity(e);
            e.previous_pos = e.pos;
            e.age = e.age.saturating_add(1);
        }
        self.tick += 1;
        self.level_tick += 1;
        enemies::olmec_intro_alarms(self);
        traps::pre_tick(self);
        player::tick(self, input);
        // Inactive players retain world collisions and the hurt/dead physics
        // controller, but cannot perform inventory actions during the intro.
        let inactive_input = InputFrame::default();
        let input = if self.player.data.get("active").copied().unwrap_or(1.0) != 0.0 {
            input
        } else {
            &inactive_input
        };
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
        items::explosives::end_tick(self);
        hub::tick(self, input);
        if self.player.pos.y > self.world.height as f64 * 16.0 + 16.0 {
            self.player.health = 0;
        }
        economy::tick(self, input);
        progression::tick(self, input);
        if self.player.health <= 0 || was_dead {
            self.player.health = self.player.health.max(0);
            self.player.state = PlayerState::Dead;
            self.state = RunState::Dead;
            if !was_dead {
                self.data.insert("death_ticks".into(), 0.0);
                self.events.push(GameEvent::Died);
                self.sound("sndDie", self.player.pos);
            }
        }
        self.world.entities.retain(|e| e.alive);
        activation::finish(self, frozen);
        if self.level_tick == room_tick_before + 1
            && solids_before
                .iter()
                .any(|id| self.world.entity(*id).is_none())
        {
            self.world.fluid_dirty = true;
        }
        self.update_camera();
    }

    pub fn next_level(&mut self) {
        if self.flags.contains("tutorial") {
            self.state = RunState::Won;
            self.events.push(GameEvent::Won);
            return;
        }
        let carry = self.take_transition_carry();
        let kisses = self.take_pending_kisses();
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
        self.apply_kisses(kisses);
        self.restore_transition_carry(carry);
        if !self.flags.contains("wanted") && !self.flags.contains("murderer") {
            self.flags.remove("shopkeepers_angry");
        }
    }

    fn take_pending_kisses(&mut self) -> i32 {
        let rescued = self.flags.remove("damsel_rescued");
        self.data
            .remove("pending_damsel_kisses")
            .map(|count| count.clamp(0.0, 99.0) as i32)
            .unwrap_or(i32::from(rescued))
    }

    fn apply_kisses(&mut self, kisses: i32) {
        if kisses > 0 {
            self.player.health = (self.player.health + kisses).min(99);
            self.sound("sndKiss", self.player.pos);
        }
    }

    fn take_transition_carry(&mut self) -> Option<(String, Option<Entity>)> {
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
        carry.map(|object| (object, carried))
    }

    fn restore_transition_carry(&mut self, carry: Option<(String, Option<Entity>)>) {
        if let Some((object, carried)) = carry {
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
        let boss_room = self.world.area == Area::Boss && !self.flags.contains("custom");
        if boss_room && self.player.data.get("active") == Some(&0.0) {
            // Zero borders let the scripted pan persist while Olmec remains
            // inside the view; follow him only when he leaves its edges.
            if let Some(boss) = self
                .world
                .entities
                .iter()
                .find(|e| e.alive && e.object == "oOlmec")
            {
                self.camera.x = self.camera.x.clamp(boss.pos.x - VIEW_WIDTH, boss.pos.x);
                self.camera.y = self.camera.y.clamp(boss.pos.y - VIEW_HEIGHT, boss.pos.y);
            }
            return;
        }
        let vertical_border = if boss_room { 64.0 } else { 96.0 };
        let relative = Vec2::new(
            self.player.pos.x - self.camera.x,
            self.player.pos.y - self.camera.y,
        );
        if relative.x < 128.0 {
            self.camera.x = self.player.pos.x - 128.0;
        } else if relative.x > VIEW_WIDTH - 128.0 {
            self.camera.x = self.player.pos.x - (VIEW_WIDTH - 128.0);
        }
        if relative.y < vertical_border {
            self.camera.y = self.player.pos.y - vertical_border;
        } else if relative.y > VIEW_HEIGHT - vertical_border {
            self.camera.y = self.player.pos.y - (VIEW_HEIGHT - vertical_border);
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

    #[test]
    fn linked_custom_exit_preserves_resources_and_the_carried_weapon() {
        let mut level = level::Level::default();
        level.set_tile(3, 3, '@').unwrap();
        level.set_tile(5, 3, 'X').unwrap();
        for x in 0..level::LEVEL_WIDTH {
            level.set_tile(x, 4, '1').unwrap();
        }
        let mut game = Game::shell(9, RulesPreset::Classic);
        game.load_custom(&level, "first", "");
        game.player.pos = Vec2::new(104.0, 72.0);
        game.player.health = 2;
        game.player.bombs = 1;
        game.player.ropes = 0;
        game.player.money = 700;
        game.player.equipment.insert("Cape".into());
        let id = game.world.spawn("oShotgun", game.player.pos);
        let gun = game.world.entity_mut(id).unwrap();
        gun.held = true;
        gun.data.insert("ammo".into(), 11.0);
        game.player.held = Some(id);
        for _ in 0..3 {
            game.step(&InputFrame {
                up: true,
                ..InputFrame::default()
            });
        }
        assert_eq!(game.state, RunState::Transition);
        assert!(game.flags.contains("carry:oShotgun"));
        let mut next = level.clone();
        next.starting_life = 99;
        next.starting_bombs = 99;
        next.starting_ropes = 99;
        game.continue_custom(&next, "second", "first");
        assert_eq!(
            (game.player.health, game.player.bombs, game.player.ropes),
            (2, 1, 0)
        );
        assert_eq!(game.player.money, 700);
        assert!(game.player.has("Cape"));
        let carried = game.world.entity(game.player.held.unwrap()).unwrap();
        assert_eq!(carried.object, "oShotgun");
        assert_eq!(carried.value("ammo"), 11.0);
        assert!(!game.flags.contains("carry:oShotgun"));
    }

    #[test]
    fn linked_rooms_do_not_reuse_an_old_entity_traps_fired_state() {
        let mut level = level::Level::default();
        level.set_tile(4, 4, '@').unwrap();
        level.set_tile(2, 4, '>').unwrap();
        for x in 0..level::LEVEL_WIDTH {
            level.set_tile(x, 5, '1').unwrap();
        }
        let mut game = Game::shell(10, RulesPreset::Classic);
        game.load_custom(&level, "first", "");
        game.player.vel.x = 1.0;
        game.step(&InputFrame::default());
        let fired_key = game
            .data
            .iter()
            .find_map(|(key, value)| {
                (key.starts_with("hazard_entity:") && key.ends_with(":fired") && *value == 1.0)
                    .then(|| key.clone())
            })
            .expect("first room's aligned arrow trap fires");
        game.continue_custom(&level, "second", "first");
        assert!(!game.data.contains_key(&fired_key));
        game.player.vel.x = 1.0;
        game.step(&InputFrame::default());
        assert_eq!(game.data.get(&fired_key), Some(&1.0));
    }

    #[test]
    fn linked_custom_continuation_consumes_each_rescued_damsels_kiss_once() {
        let mut level = level::Level::default();
        level.set_tile(3, 3, '@').unwrap();
        level.set_tile(5, 3, 'X').unwrap();
        for x in 0..level::LEVEL_WIDTH {
            level.set_tile(x, 4, '1').unwrap();
        }
        let mut game = Game::shell(11, RulesPreset::Classic);
        game.load_custom(&level, "first", "");
        game.player.pos = Vec2::new(104.0, 72.0);
        game.player.health = 2;
        for _ in 0..2 {
            let damsel = game.world.spawn("oDamsel", game.player.pos);
            game.world.entity_mut(damsel).unwrap().held = true;
            game.player.held = Some(damsel);
            game.step(&InputFrame::default());
        }
        assert_eq!(game.data["pending_damsel_kisses"], 2.0);
        game.step(&InputFrame {
            up: true,
            ..Default::default()
        });
        assert_eq!(game.state, RunState::Transition);
        game.continue_custom(&level, "second", "first");
        assert_eq!(game.player.health, 4);
        assert!(!game.data.contains_key("pending_damsel_kisses"));
        game.continue_custom(&level, "third", "second");
        assert_eq!(game.player.health, 4);
    }

    #[test]
    fn lethal_contact_keeps_corpse_physics_and_actor_clocks_running() {
        let mut game = flat_game();
        game.player.health = 1;
        let arrow = game.world.spawn("oArrow", Vec2::new(24.0, 40.0));
        game.world.entity_mut(arrow).unwrap().vel.x = 8.0;
        let rock = game.world.spawn("oRock", Vec2::new(160.0, 24.0));
        for _ in 0..4 {
            game.step(&InputFrame::default());
            if game.state == RunState::Dead {
                break;
            }
        }
        assert_eq!(game.state, RunState::Dead);
        assert_eq!(
            game.events
                .iter()
                .filter(|event| matches!(event, GameEvent::Died))
                .count(),
            1
        );
        let dead_pos = game.player.pos;
        let age = game.world.entity(rock).unwrap().age;
        let tick = game.tick;
        for _ in 0..8 {
            game.step(&InputFrame::default());
            assert!(
                !game
                    .events
                    .iter()
                    .any(|event| matches!(event, GameEvent::Died))
            );
            assert!(
                !game.events.iter().any(
                    |event| matches!(event, GameEvent::Sound { name, .. } if name == "sndDie")
                )
            );
        }
        assert_ne!(game.player.pos, dead_pos);
        assert_eq!(game.world.entity(rock).unwrap().age, age + 8);
        assert_eq!(game.tick, tick + 8);
        assert_eq!(game.data.get("death_ticks"), Some(&8.0));
        game.step(&InputFrame {
            attack_pressed: true,
            ..InputFrame::default()
        });
        assert_eq!(game.data.get("death_ticks"), Some(&100.0));
    }
}
