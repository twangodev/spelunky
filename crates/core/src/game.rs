use crate::*;

impl Game {
    pub fn new(seed: u64, rules: RulesPreset) -> Self {
        let mut game = Self::shell(seed, rules);
        game.flags.insert("no_dark_level".into());
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
        self.ending = None;
        self.draw_view = None;
        self.opening = None;
        self.dispatcher = Default::default();
        self.alarms = dispatcher::unset_alarms();
        self.game_over = Default::default();
        self.player.alarms = dispatcher::unset_alarms();
        self.player.engine_active = true;
        self.data.insert("xmoney".into(), 0.);
        self.data.insert("xtime".into(), 0.);
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
        // Recreate the selected character's initial sprite so the previous
        // room's exit animation cannot keep the new controller disabled.
        self.player.set_sprite(match characters::selected(self) {
            characters::Character::Explorer => "sStandLeft",
            characters::Character::Damsel => "sDamselLeft",
            characters::Character::TunnelMan => "sTunnelLeft",
        });
        self.player.data.clear();
        self.player.data.insert("active".into(), 1.0);
        self.player.invincible = 0;
        self.data
            .insert("darkness".into(), if self.world.dark { 1.0 } else { 0.0 });
        self.data.insert("darkness_lerp".into(), 0.0);
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
            world_events::init_entity(e);
            e.previous_pos = e.pos;
        }
        // oPlayer1 Create disables its controller only in the authored rOlmec
        // room. oOlmec Create starts the zero-border boss camera at Y=400.
        if self.world.area == Area::Boss && !self.flags.contains("custom") {
            self.player.data.insert("active".into(), 0.0);
            self.camera = Vec2::new(0.0, 400.0);
        }
        player::initialize_level_alarms(self);
        self.update_camera();
        self.previous_camera = self.camera;
    }

    /// Advance exactly one original 1/30-second tick.
    pub fn step(&mut self, input: &InputFrame) {
        self.draw_view = None;
        if self.opening.is_some() {
            opening::tick(self, input);
            return;
        }
        let live_exit = self.ending.as_ref().is_some_and(|s| {
            s.stage == ending::EndingStage::Exit && s.kind != ending::EndingKind::TitleCredits
        });
        if self.state == RunState::Ending && !live_exit {
            ending::tick(self, input);
            return;
        }
        if !matches!(self.state, RunState::Playing | RunState::Dead) && !live_exit {
            return;
        }
        if self.flags.contains("hub_title") || self.flags.contains("hub_scores") {
            self.player.health = if characters::selected(self) == characters::Character::TunnelMan {
                2
            } else {
                4
            };
            if self.state == RunState::Dead {
                self.state = RunState::Playing;
                self.data.remove("death_ticks");
            }
            if self.player.state == PlayerState::Dead {
                self.player.state = PlayerState::Falling;
            }
        }
        self.events.clear();
        self.flags.remove("render_discontinuity");
        dispatcher::step(self, input);
        self.update_camera();
    }

    /// Start a destination using the existing source RNG and persistent profile.
    pub fn start_from_hub(&mut self, tutorial: bool, level: u32) {
        let character = characters::selected(self);
        self.flags.retain(|f| {
            f.starts_with("unlock_")
                || f.starts_with("character_")
                || f.starts_with("shortcut")
                || f == "first_time"
        });
        self.data
            .retain(|key, _| key.starts_with("tunnel") || key.starts_with("shortcut"));
        self.ending = None;
        self.opening = None;
        self.tick = 0;
        characters::apply(self, character);
        self.player.money = 0;
        self.player.equipment.clear();
        if tutorial {
            self.flags.insert("tutorial".into());
            self.player.bombs = 0;
            self.player.ropes = 2;
            self.world = generation::tutorial();
        } else {
            if level > 1 {
                self.flags.insert("used_shortcut".into());
            }
            self.flags.insert("no_dark_level".into());
            self.world = generation::generate(&mut self.rng, level.clamp(1, 16), &self.flags);
        }
        self.reset_position();
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
            // Transition Create spawns one kiss actor when xdamsels > 0,
            // independently of the number of rescue icons in its tally.
            self.player.health = (self.player.health + 1).min(99);
        }
    }

    fn take_transition_carry(&mut self) -> Option<String> {
        let carry = self
            .flags
            .iter()
            .find_map(|f| f.strip_prefix("carry:").map(str::to_owned));
        self.flags.retain(|f| !f.starts_with("carry:"));
        carry
    }

    fn restore_transition_carry(&mut self, carry: Option<String>) {
        if let Some(object) = carry.filter(|object| {
            matches!(
                object.as_str(),
                "oRock"
                    | "oJar"
                    | "oSkull"
                    | "oFishBone"
                    | "oArrow"
                    | "oMachete"
                    | "oMattock"
                    | "oMattockHead"
                    | "oPistol"
                    | "oWebCannon"
                    | "oTeleporter"
                    | "oShotgun"
                    | "oBow"
                    | "oFlare"
                    | "oSceptre"
                    | "oKey"
            )
        }) {
            let id = self.world.spawn(object, self.player.pos);
            let item = self.world.entity_mut(id).expect("fresh carried item");
            // scrHoldItem recreates the type, then clears purchase/new flags.
            // Bow ammunition belongs to the surviving global run data.
            items::init_entity(item);
            item.held = true;
            item.data.insert("cost".into(), 0.);
            item.data.insert("forSale".into(), 0.);
            item.data.insert("new".into(), 0.);
            self.player.held = Some(id);
        }
    }

    pub fn update_camera(&mut self) {
        if self
            .opening
            .as_ref()
            .is_some_and(|opening| !opening.player_present)
        {
            return;
        }
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

    /// Classic's recorded clock adds room_speed (30) per live level tick,
    /// then floors the accumulated source units by 1000 at record time.
    pub fn source_elapsed_ms(&self) -> u64 {
        self.ending
            .as_ref()
            .filter(|ending| ending.stage != crate::ending::EndingStage::Exit)
            .map(|ending| ending.source_time_ms)
            .unwrap_or_else(|| self.data.get("time").copied().unwrap_or(0.).max(0.) as u64)
    }

    pub fn source_elapsed_seconds(&self) -> u64 {
        self.source_elapsed_ms() / 1000
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
    fn transition_recreates_carried_weapon_and_applies_rescue_health() {
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
        assert!(!held.data.contains_key("ammo"));
        assert_eq!(held.value("cost"), 0.);
        assert_eq!(held.value("new"), 0.);
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
        assert_eq!(game.state, RunState::Ending);
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
        assert!(!carried.data.contains_key("ammo"));
        assert!(!game.flags.contains("carry:oShotgun"));
    }

    #[test]
    fn completed_exit_recreates_a_moving_player_in_linked_and_campaign_rooms() {
        use characters::Character;
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            for character in [Character::Explorer, Character::Damsel, Character::TunnelMan] {
                for custom in [true, false] {
                    let mut first = level::Level::default();
                    first.set_tile(6, 7, '@').unwrap();
                    first.set_tile(7, 7, 'X').unwrap();
                    for x in 0..level::LEVEL_WIDTH {
                        first.set_tile(x, 8, '1').unwrap();
                    }
                    let mut game = Game::shell(7, rules);
                    characters::apply(&mut game, character);
                    game.load_custom(&first, "first", "");
                    if !custom {
                        game.flags.remove("custom");
                        game.flags.remove("custom_level");
                    }
                    for _ in 0..60 {
                        game.step(&InputFrame {
                            right: true,
                            up: true,
                            ..Default::default()
                        });
                        if game.state == RunState::Ending {
                            break;
                        }
                    }
                    assert!(game.player.is_exiting(), "{rules:?}/{character:?}/{custom}");
                    for _ in 0..1_000 {
                        if game.state == RunState::Transition {
                            break;
                        }
                        game.step(&InputFrame {
                            start_pressed: true,
                            ..Default::default()
                        });
                    }
                    assert_eq!(game.state, RunState::Transition);
                    assert_eq!(
                        game.ending.as_ref().unwrap().stage,
                        ending::EndingStage::Complete
                    );
                    if custom {
                        let mut next = first.clone();
                        next.set_tile(6, 7, '0').unwrap();
                        next.set_tile(7, 7, '0').unwrap();
                        next.set_tile(15, 7, '@').unwrap();
                        next.exit_targets = vec!["first".into()];
                        game.continue_custom(&next, "second", "first");
                    } else {
                        game.next_level();
                    }
                    for _ in 0..60 {
                        game.step(&InputFrame::default());
                        if game.player.grounded {
                            break;
                        }
                    }
                    assert!(game.player.grounded, "{rules:?}/{character:?}/{custom}");
                    let start = game.player.pos;
                    for tick in 0..6 {
                        game.step(&InputFrame {
                            right: true,
                            jump: true,
                            jump_pressed: tick == 0,
                            ..Default::default()
                        });
                    }
                    assert!(
                        game.player.pos.x > start.x + 4.,
                        "continued player did not move: {rules:?}/{character:?}/{custom}: {:?}",
                        game.player
                    );
                    assert!(
                        game.player.vel.y < 0. && game.player.pos.y < start.y,
                        "continued player did not jump: {rules:?}/{character:?}/{custom}"
                    );
                }
            }
        }
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
    fn linked_custom_continuation_applies_one_kiss_for_multiple_rescues() {
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
        assert_eq!(game.state, RunState::Ending);
        game.continue_custom(&level, "second", "first");
        assert_eq!(game.player.health, 3);
        assert!(!game.data.contains_key("pending_damsel_kisses"));
        game.continue_custom(&level, "third", "second");
        assert_eq!(game.player.health, 3);
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
        // Source game-over reveal is an independent controller, not the
        // elapsed corpse-physics clock. The first action skips its reveal.
        assert_eq!(game.data.get("death_ticks"), Some(&9.0));
        assert_eq!(game.game_over.draw_status, 3);
    }

    #[test]
    fn authored_title_and_scores_restore_selected_character_life_without_using_ankh() {
        for room in [hub::HubKind::Title, hub::HubKind::Scores] {
            for character in [
                characters::Character::Explorer,
                characters::Character::Damsel,
                characters::Character::TunnelMan,
            ] {
                let mut game = Game::shell(7, RulesPreset::Classic);
                characters::apply(&mut game, character);
                game.load_hub(room);
                game.player.health = 0;
                game.player.state = PlayerState::Dead;
                game.state = RunState::Dead;
                game.player.equipment.insert("Ankh".into());
                game.data.insert("death_ticks".into(), 100.);
                game.step(&InputFrame::default());
                assert_eq!(game.state, RunState::Playing, "{room:?}/{character:?}");
                assert_eq!(
                    game.player.health,
                    if character == characters::Character::TunnelMan {
                        2
                    } else {
                        4
                    }
                );
                assert_ne!(game.player.state, PlayerState::Dead);
                assert!(game.player.has("Ankh"));
                assert!(!game.data.contains_key("death_ticks"));
            }
        }
    }

    #[test]
    fn challenge_death_attack_reveals_then_routes_to_its_scores_door() {
        for (kind, start) in [
            (hub::HubKind::Sun, 1),
            (hub::HubKind::Moon, 2),
            (hub::HubKind::Stars, 3),
        ] {
            let mut game = Game::shell(7, RulesPreset::Classic);
            game.load_hub(kind);
            // A controlled terminal explosion supplies the same fatal hazard
            // path without depending on a seed's later challenge spawn RNG.
            let blast = game.world.spawn("oExplosion", game.player.pos);
            items::init_entity(game.world.entity_mut(blast).unwrap());
            game.world.entity_mut(blast).unwrap().timer = 1;
            game.step(&InputFrame::default());
            assert_eq!(game.state, RunState::Dead, "{kind:?}");
            let action = InputFrame {
                attack_pressed: true,
                ..Default::default()
            };
            game.step(&action);
            assert_eq!(game.state, RunState::Dead);
            assert_eq!(game.game_over.draw_status, 3);
            assert_eq!(game.game_over.money_count, game.player.money);
            game.step(&action);
            assert_eq!(game.state, RunState::Transition);
            assert!(game.flags.contains(&format!("scores_start:{start}")));
            assert_eq!(
                game.game_over.destination,
                Some(progression::GameOverDestination::Scores)
            );
            assert!(game.flags.contains("reset_run_on_hub"));
        }
    }

    #[test]
    fn authored_title_rope_exit_begins_climbing_dummy_before_credits() {
        let mut game = Game::shell(7, RulesPreset::Classic);
        game.load_hub(hub::HubKind::Title);
        game.step(&InputFrame {
            up: true,
            jump: true,
            jump_pressed: true,
            ..Default::default()
        });
        for _ in 0..220 {
            game.step(&InputFrame {
                up: true,
                ..Default::default()
            });
            if game.state == RunState::Ending {
                break;
            }
        }
        let ending = game.ending.as_ref().expect("authored title rope can exit");
        assert_eq!(ending.kind, ending::EndingKind::TitleCredits);
        assert_eq!(ending.stage, ending::EndingStage::Exit);
        assert_eq!(game.world.name, "rTitle");
        let dummy = game
            .world
            .entities
            .iter()
            .find(|entity| entity.object == "oPDummy5")
            .unwrap();
        assert!(dummy.pos.y < 32.);
        assert_eq!(dummy.vel.y, -1.);
        assert_eq!(game.player.data["active"], 0.);
    }

    #[test]
    fn source_record_clock_uses_source_units_and_excludes_hubs_and_dead_ticks() {
        let mut game = flat_game();
        for _ in 0..34 {
            game.step(&InputFrame::default());
        }
        assert_eq!(game.source_elapsed_ms(), 1020);
        assert_eq!(game.source_elapsed_seconds(), 1);
        game.player.health = 0;
        game.player.state = PlayerState::Dead;
        game.state = RunState::Dead;
        for _ in 0..8 {
            game.step(&InputFrame::default());
        }
        assert_eq!(game.source_elapsed_ms(), 1020);
        game.load_hub(hub::HubKind::Title);
        let before = game.source_elapsed_ms();
        for _ in 0..8 {
            game.step(&InputFrame::default());
        }
        assert_eq!(game.source_elapsed_ms(), before);
    }

    #[test]
    fn source_clock_counts_alive_start_fatal_frame_then_freezes_during_dead_world_ticks() {
        let mut game = flat_game();
        for _ in 0..3 {
            game.step(&InputFrame::default());
        }
        assert_eq!(game.source_elapsed_ms(), 90);
        let blast = game.world.spawn("oExplosion", game.player.pos);
        items::init_entity(game.world.entity_mut(blast).unwrap());
        game.world.entity_mut(blast).unwrap().timer = 1;
        game.step(&InputFrame::default());
        assert_eq!(game.state, RunState::Dead);
        assert_eq!(game.data["time"], 120.);
        assert_eq!(game.data["xtime"], 120.);
        let run_time = game.data["time"];
        let room_time = game.data["xtime"];
        let tick = game.tick;
        for _ in 0..5 {
            game.step(&InputFrame::default());
        }
        assert_eq!(game.data["time"], run_time);
        assert_eq!(game.data["xtime"], room_time);
        assert_eq!(game.tick, tick + 5);
    }

    #[test]
    fn editor_playtest_returns_after_exit_animation_without_transition_or_bonus() {
        let mut game = flat_game();
        game.flags.insert("custom".into());
        game.flags.insert("custom_level".into());
        game.flags.insert("editor_test".into());
        game.world.name = "editor fixture".into();
        game.world.spawn("oExit", Vec2::new(32., 32.));
        game.step(&InputFrame {
            up: true,
            ..Default::default()
        });
        assert_eq!(game.state, RunState::Ending);
        for _ in 0..32 {
            game.step(&InputFrame::default());
            if game.state != RunState::Ending {
                break;
            }
        }
        assert_eq!(game.state, RunState::Transition);
        assert!(game.ending.is_none());
        assert_eq!(game.world.name, "editor fixture");
        assert_eq!(game.player.money, 0);
        assert!(!game.events.contains(&GameEvent::Won));
    }

    #[test]
    fn standing_up_ending_advances_on_entry_frame_and_freezes_at_source_960ms() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            for character in [
                characters::Character::Explorer,
                characters::Character::Damsel,
                characters::Character::TunnelMan,
            ] {
                let mut game = flat_game();
                game.rules = rules;
                characters::apply(&mut game, character);
                game.world.spawn("oXEnd", Vec2::new(32., 32.));
                game.step(&InputFrame {
                    up: true,
                    ..Default::default()
                });
                assert_eq!(game.state, RunState::Ending);
                assert_eq!(game.player.animation, 0.5);
                assert_eq!(game.ending.as_ref().unwrap().stage_tick, 1);
                assert_eq!(game.source_elapsed_ms(), 30);
                for _ in 1..31 {
                    game.step(&InputFrame::default());
                    assert_eq!(
                        game.ending.as_ref().unwrap().stage,
                        ending::EndingStage::Exit
                    );
                }
                game.step(&InputFrame::default());
                assert_eq!(
                    game.ending.as_ref().unwrap().stage,
                    ending::EndingStage::Chamber
                );
                assert_eq!(game.tick, 32);
                assert_eq!(game.source_elapsed_ms(), 960);
                game.step(&InputFrame::default());
                assert_eq!(game.source_elapsed_ms(), 960);
            }
        }
    }
    #[test]
    fn exit_animation_keeps_live_world_and_source_clock_running_then_changes_rooms() {
        let mut game = flat_game();
        game.world.spawn("oExit", Vec2::new(32., 32.));
        let rock = game.world.spawn("oRock", Vec2::new(88., 16.));
        game.step(&InputFrame {
            up: true,
            ..Default::default()
        });
        assert_eq!(game.state, RunState::Ending);
        assert_eq!(
            game.ending.as_ref().unwrap().stage,
            ending::EndingStage::Exit
        );
        let player_pos = game.player.pos;
        let world_pos = game.world.entity(rock).unwrap().pos;
        let world_age = game.world.entity(rock).unwrap().age;
        let tick = game.tick;
        let elapsed = game.source_elapsed_ms();
        let sprite_phase = game.player.animation;
        for _ in 0..3 {
            game.step(&InputFrame {
                left: true,
                jump: true,
                attack_pressed: true,
                ..Default::default()
            });
        }
        assert_eq!(
            game.player.pos, player_pos,
            "inactive exit controller ignores movement"
        );
        assert_ne!(
            game.world.entity(rock).unwrap().pos,
            world_pos,
            "source world remains live"
        );
        assert_eq!(game.world.entity(rock).unwrap().age, world_age + 3);
        assert_eq!(game.tick, tick + 3, "one source tick per live exit step");
        assert_eq!(game.source_elapsed_ms(), elapsed + 90);
        assert_eq!(game.player.animation, sprite_phase + 1.5);
        for _ in 0..32 {
            game.step(&InputFrame::default());
            if game.ending.as_ref().unwrap().stage != ending::EndingStage::Exit {
                break;
            }
        }
        assert_eq!(
            game.ending.as_ref().unwrap().stage,
            ending::EndingStage::Transition
        );
        assert_eq!(game.world.name, "rTransition1");
        assert!(
            game.world
                .entity(rock)
                .is_none_or(|entity| entity.object != "oRock"),
            "activation must not restore old level instances into its successor room"
        );
    }
}
