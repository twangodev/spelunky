use crate::*;

const SOURCE_CUSTOM_ROOM_LAYOUT_FLAGS: [&str; 10] = [
    "snake_pit",
    "snakePit",
    "sacrifice_pit",
    "sacrificePit",
    "alien_ship",
    "alienCraft",
    "yeti_lair",
    "yetiLair",
    "black_market",
    "blackMarket",
];

// Native 4BECA4: convert target origin with FISTP (nearest-even), apply
// border branches, then sequential room limits. Source room follow speeds
// are -1 (unlimited); this helper does not implement arbitrary speed settings.
// Inputs are the native finite valid signed64 conversion domain; keep the
// low32 word after rounding, as 403498 does (not a saturating f64-to-i32 cast).
fn source_follow_axis(old: f64, actor: f64, border: f64, extent: f64, room: f64) -> f64 {
    let (old, actor, border, extent, room) = (
        old.round_ties_even() as i64 as i32,
        actor.round_ties_even() as i64 as i32,
        border.round_ties_even() as i64 as i32,
        extent.round_ties_even() as i64 as i32,
        room.round_ties_even() as i64 as i32,
    );
    let mut desired = old;
    if border.wrapping_mul(2) >= extent {
        desired = actor.wrapping_sub(extent / 2);
    } else if actor.wrapping_sub(border) < old {
        desired = actor.wrapping_sub(border);
    } else if actor.wrapping_add(border) > old.wrapping_add(extent) {
        desired = actor.wrapping_add(border).wrapping_sub(extent);
    }
    if desired < 0 {
        desired = 0;
    }
    if room < desired.wrapping_add(extent) {
        desired = room.wrapping_sub(extent);
    }
    f64::from(desired)
}

fn campaign_level_type(level: u32) -> u32 {
    match level {
        5..=8 => 1,
        9..=12 => 2,
        13..=15 => 3,
        16 => 4,
        _ => 0,
    }
}

#[cfg(test)]
std::thread_local! {
    static CAMPAIGN_ROOM_CREATE_HEALTH: std::cell::Cell<Option<(u32, f64)>> =
        const { std::cell::Cell::new(None) };
}

impl Game {
    pub fn new(seed: u32, rules: RulesPreset) -> Self {
        Self::generate_first_room(Self::shell(seed, rules))
    }

    pub fn new_with_music(seed: u32, rules: RulesPreset, music: SourceMusicState) -> Self {
        Self::generate_first_room(Self::shell_with_music(seed, rules, music))
    }

    /// Build the initial campaign room with the source graphics preference in
    /// place before SetupWalls and background-decoration RNG is consumed.
    pub fn new_with_music_and_graphics(
        seed: u32,
        rules: RulesPreset,
        music: SourceMusicState,
        graphics_high: bool,
    ) -> Self {
        let mut game = Self::shell_with_music(seed, rules, music);
        game.set_graphics_high(graphics_high);
        Self::generate_first_room(game)
    }

    pub fn try_new(seed: u64, rules: RulesPreset) -> Result<Self, RngSeedError> {
        Ok(Self::generate_first_room(Self::try_shell(seed, rules)?))
    }

    pub fn legacy(seed: u64, rules: RulesPreset) -> Self {
        Self::generate_first_room(Self::legacy_shell(seed, rules))
    }

    /// Select identity before generation consumes any random draws.
    pub fn new_with_algorithm(
        seed: u64,
        rules: RulesPreset,
        algorithm: RngAlgorithm,
    ) -> Result<Self, RngSeedError> {
        Ok(Self::generate_first_room(Self::shell_with_algorithm(
            seed, rules, algorithm,
        )?))
    }

    /// Restore the pre-Jungle-revision contract before initial generation.
    pub fn new_historical_with_algorithm(
        seed: u64,
        rules: RulesPreset,
        algorithm: RngAlgorithm,
    ) -> Result<Self, RngSeedError> {
        let mut game = Self::shell_with_algorithm(seed, rules, algorithm)?;
        game.generation_revision = GenerationRevision::HistoricalV1;
        Ok(Self::generate_first_room(game))
    }

    fn generate_first_room(mut game: Self) -> Self {
        game.flags.insert("no_dark_level".into());
        game.generate_campaign_room(1);
        game.reset_position();
        traps::initialize_level(&mut game);
        game
    }

    pub fn tutorial_with_music(seed: u32, rules: RulesPreset, music: SourceMusicState) -> Self {
        Self::prepare_tutorial(Self::shell_with_music(seed, rules, music))
    }

    pub fn tutorial_with_music_and_graphics(
        seed: u32,
        rules: RulesPreset,
        music: SourceMusicState,
        graphics_high: bool,
    ) -> Self {
        let mut game = Self::shell_with_music(seed, rules, music);
        game.set_graphics_high(graphics_high);
        Self::prepare_tutorial(game)
    }

    pub fn tutorial(seed: u32, rules: RulesPreset) -> Self {
        Self::prepare_tutorial(Self::shell(seed, rules))
    }

    pub fn try_tutorial(seed: u64, rules: RulesPreset) -> Result<Self, RngSeedError> {
        Ok(Self::prepare_tutorial(Self::try_shell(seed, rules)?))
    }

    pub fn legacy_tutorial(seed: u64, rules: RulesPreset) -> Self {
        Self::prepare_tutorial(Self::legacy_shell(seed, rules))
    }

    fn prepare_tutorial(mut game: Self) -> Self {
        // Both direct constructors and the title/hub route enter rTutorial
        // with the same globals established by Classic's tutorial transition.
        game.data.insert("currLevel".into(), 0.0);
        game.data.insert("levelType".into(), 0.0);
        // oPlayer1's xSTART/xTUTORIAL animation-end branches set these before
        // room_goto(rTutorial); keep direct core constructors on that path too.
        game.player.bombs = 0.0;
        game.player.ropes = 2.0;
        game.world = game.generate_tutorial_world();
        game.flags.insert("tutorial".into());
        game.reset_position();
        traps::initialize_level(&mut game);
        game
    }

    fn generate_tutorial_world(&mut self) -> World {
        if self.generation_revision == GenerationRevision::SourceClassicV2 {
            generation::tutorial_with_context(
                &mut self.rng,
                generation::GenerationContext {
                    has_spectacles: self.player.has("Spectacles"),
                    has_cemetery: false,
                    has_city_of_gold: self.flags.contains("cityOfGold")
                        || self.flags.contains("city_of_gold"),
                },
                !self.flags.contains("low_graphics"),
            )
        } else {
            generation::tutorial()
        }
    }

    pub fn load_custom(&mut self, level: &level::Level, name: &str, previous: &str) {
        let starting_inventory = [
            f64::from(level.starting_life.clamp(1, 99)),
            f64::from(level.starting_bombs.clamp(0, 99)),
            f64::from(level.starting_ropes.clamp(0, 99)),
        ];
        self.load_custom_room(level, name, previous, Some(starting_inventory));
        self.finish_custom_room_init();
    }

    /// Editor test play uses real(starting inventory text) directly, before
    /// ordinary custom-level loading applies its floor and clamps.
    pub fn load_custom_editor_test(
        &mut self,
        level: &level::Level,
        name: &str,
        previous: &str,
        starting_inventory: [f64; 3],
    ) {
        self.load_custom_room(level, name, previous, Some(starting_inventory));
        self.finish_custom_room_init();
    }

    /// Linked custom rooms preserve the run's inventory, as scrLoadLevel(-1)
    /// does, while restoring the light item selected by the exit handler.
    pub fn continue_custom(&mut self, level: &level::Level, name: &str, previous: &str) {
        let carry = self.take_transition_carry();
        let kisses = self.take_pending_kisses();
        self.load_custom_room(level, name, previous, None);
        self.apply_kisses(kisses);
        self.restore_transition_carry(carry);
        self.finish_custom_room_init();
    }

    fn finish_custom_room_init(&mut self) {
        traps::initialize_level(self);
        if self.generation_revision == GenerationRevision::SourceClassicV2 {
            // scrLevelGen resets these room-local selectors on every custom
            // entry. Keep request and run-history flags such as genBlackMarket
            // and made_black_market intact across linked custom rooms.
            self.flags
                .retain(|flag| !SOURCE_CUSTOM_ROOM_LAYOUT_FLAGS.contains(&flag.as_str()));
        }
        generation::initialize_custom_source_setup(self);
        if self.generation_revision == GenerationRevision::SourceClassicV2 {
            // oGame.Create completes scrInitLevel before the first Draw. The
            // wanted-exit guards are part of that synchronous room setup, so
            // initialise them after SetupWalls/background generation here;
            // the first dispatcher tick remains an idempotent fallback.
            progression::initialize_level(self);
        }
    }

    /// Complete the authored custom-level transition reward when a run ends
    /// instead of loading another linked room.
    pub fn finish_custom_transition(&mut self) {
        let kisses = self.take_pending_kisses();
        self.apply_kisses(kisses);
    }

    fn load_custom_room(
        &mut self,
        level: &level::Level,
        name: &str,
        previous: &str,
        starting_inventory: Option<[f64; 3]>,
    ) {
        // scrInitLevel resets roomPath before scrLevelGen on each custom load.
        // Historical snapshots retain the old adapter grid because their
        // custom path predates the SourceClassic lifecycle.
        let historical_room_path = (self.generation_revision == GenerationRevision::HistoricalV1)
            .then(|| self.world.source_room_path.unwrap_or([[0; 4]; 5]));
        if self.generation_revision == GenerationRevision::SourceClassicV2 {
            // oLoadLevel Create clears global.cleanSolids before the custom
            // room is constructed. Exit animation sets it to suppress Destroy
            // drops while leaving the old room, so linked custom rooms must
            // clear it before their terrain and instance Creates run.
            self.data.insert("cleanSolids".into(), 0.0);
            self.flags.remove("cleanSolids");
        }
        self.world = level.to_world();
        self.custom_level_author = Some(level.author.clone());
        // rLoadLevel Create resets currLevel before oGame.Create runs
        // scrInitLevel. That room then derives levelType 0 for the custom
        // load, independently of any campaign that was open before it.
        self.data.insert("currLevel".into(), 1.0);
        self.data.insert("levelType".into(), 0.0);
        self.world.source_create_projection =
            self.generation_revision == GenerationRevision::SourceClassicV2;
        self.world.source_room_path = historical_room_path;
        generation::initialize_custom_terrain_with_context(
            &mut self.world,
            &mut self.rng,
            generation::GenerationContext {
                has_spectacles: self.player.has("Spectacles"),
                has_cemetery: false,
                has_city_of_gold: self.flags.contains("cityOfGold")
                    || self.flags.contains("city_of_gold"),
            },
        );
        self.world.name = name.to_owned();
        self.flags.retain(|flag| {
            flag != "hub" && !flag.starts_with("hub_") && !flag.starts_with("hub_destination:")
        });
        self.flags.insert("custom".into());
        self.flags.insert("custom_level".into());
        // rLoadLevel Create sets global.keepScore=false, and only oTitle Create
        // turns it back on. This flag survives the custom ending's return to
        // the score room and any challenge rooms entered from there.
        self.flags.insert("scores_disabled".into());
        self.flags.remove("tutorial");
        self.flags
            .retain(|f| !f.starts_with("custom_level:") && !f.starts_with("default_next_custom:"));
        if let Some([health, bombs, ropes]) = starting_inventory {
            self.player.health = health;
            self.player.bombs = bombs;
            self.player.ropes = ropes;
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
            // Classic startMusic compares these authored codes exactly.
            match level.music.as_str() {
                "LUSH" => 1.0,
                "ICE" => 2.0,
                "TEMPLE" => 3.0,
                "BOSS" => 4.0,
                _ => 0.0,
            },
        );
        if !level.next_level.is_empty() {
            self.flags
                .insert(format!("default_next_custom:{}", level.next_level));
        }
        self.reset_position();
    }

    pub fn load_hub(&mut self, kind: hub::HubKind) {
        let completed_campaign = kind == hub::HubKind::Scores && self.state == RunState::Won;
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
        if completed_campaign {
            // Host persistence snapshots the finished run before routing to
            // Scores. Classic then calls scrClearGlobals before room_goto.
            self.clear_run_globals();
            self.flags.remove("reset_run_on_hub");
        }
        if kind == hub::HubKind::Title {
            // oTitle Create restores global.keepScore after custom play.
            self.flags.remove("scores_disabled");
        }
        self.data.retain(|key, _| {
            !key.starts_with("hub_")
                && !key.starts_with("sun_")
                && !key.starts_with("moon_")
                && !key.starts_with("stars_")
        });
        self.world = hub::world(kind, &self.flags);
        self.reset_position();
        hub::initialize(self, kind);
        if kind == hub::HubKind::Scores {
            // A room change starts a fresh live room even when its caller was
            // in the Won or Transition state used to leave the prior screen.
            self.state = RunState::Playing;
        }
        traps::initialize_level(self);
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
        // oLevel Create clears duration, but not global.shakeToggle.
        self.data.insert("shake".into(), 0.);
        // Authored room defaults; runtime Create can select another target
        // before the first Level Step. Do not infer it from Player.active.
        self.view_target = ViewTarget::Player;
        self.data.remove("camera_hborder");
        self.data.remove("camera_vborder");
        self.player.pos = self.world.entrance;
        self.player.previous_pos = self.player.pos;
        self.player.vel = Vec2::default();
        // Each room creates a nonpersistent Player; later hub overrides still run.
        self.player.facing = 1;
        self.player.state = PlayerState::Falling;
        self.player.grounded = false;
        self.player.held = None;
        self.player.pickup_item_type.clear();
        self.player.stun = 0;
        self.player.attack_timer = 0;
        // characterCreateEvent initializes jumpTime to jumpTimeTotal. This
        // preserves full gravity on the initial falling ticks; an actual jump
        // resets the counter to zero before variable-height gravity begins.
        self.player.jump_time = 10;
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
                && key != "source_music_room_created"
                && key != "curse_level_create_initialized"
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
        self.flags.remove("source_game_inactive");
        music::room_create(self);
        player::initialize_level_alarms(self);
        // The deferred Olmec Create must receive the initial room view unchanged.
        // Ordinary rooms retain the host's startup entrance projection.
        if self.world.area != Area::Boss || self.flags.contains("custom") {
            self.update_camera();
        }
        self.previous_camera = self.camera;
    }

    /// Retire adapter-held history when a host discards gameplay controls.
    /// Source controller keys stay cached until their next actual sample.
    pub fn clear_direction_input_history(&mut self) {
        self.data.remove("input_frame_left");
        self.data.remove("input_frame_right");
        self.data.remove("input_frame_attack");
        self.data.remove("input_frame_up");
        self.data.remove("input_frame_down");
    }

    /// Normalize held-only callers against actual input frames, independently
    /// of whether the Player controller is active. Explicit device edges win.
    /// Repeating this on an already normalized frame is idempotent.
    pub(crate) fn normalize_direction_releases(&mut self, input: &InputFrame) -> InputFrame {
        let mut frame = *input;
        frame.left_released |= self.data.get("input_frame_left") == Some(&1.0) && !input.left;
        frame.right_released |= self.data.get("input_frame_right") == Some(&1.0) && !input.right;
        for (key, held) in [
            ("input_frame_left", input.left),
            ("input_frame_right", input.right),
        ] {
            if held {
                self.data.insert(key.into(), 1.0);
            } else {
                self.data.remove(key);
            }
        }
        frame
    }

    /// Held-only action compatibility belongs to admitted engine frames,
    /// never to a Player or cinematic callback cache. Explicit device edges
    /// remain independently authoritative, including short taps and OR holds.
    pub(crate) fn normalize_action_edges(&mut self, input: &InputFrame) -> InputFrame {
        let mut frame = *input;
        frame.attack_released |= self.data.get("input_frame_attack") == Some(&1.0) && !input.attack;
        frame.up_pressed |= input.up && self.data.get("input_frame_up") != Some(&1.0);
        frame.down_pressed |= input.down && self.data.get("input_frame_down") != Some(&1.0);
        for (key, held) in [
            ("input_frame_attack", input.attack),
            ("input_frame_up", input.up),
            ("input_frame_down", input.down),
        ] {
            if held {
                self.data.insert(key.into(), 1.0);
            } else {
                self.data.remove(key);
            }
        }
        frame
    }

    /// Advance exactly one original 1/30-second tick.
    pub fn step(&mut self, input: &InputFrame) {
        let frame = self.normalize_direction_releases(input);
        let frame = self.normalize_action_edges(&frame);
        let input = &frame;
        self.capture_render_history();
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
                2.0
            } else {
                4.0
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
        items::explosives::migrate_legacy_stash(self);
        dispatcher::step(self, input);
        self.update_camera();
        // Draw freezes actors before Animation callbacks. Keep camera following
        // in that same phase when Animation End repositions the live player.
        // The authoritative camera and collision history remain untouched.
        if let Some(draw) = self.draw_view.as_mut() {
            if draw.player.pos != self.player.pos {
                draw.update_camera();
            } else {
                draw.previous_camera = self.previous_camera;
                draw.camera = self.camera;
            }
        }
    }

    /// scrClearGlobals changes run globals without replacing the current room,
    /// actor callbacks, selected character, profile, or random stream.
    pub(crate) fn clear_run_globals(&mut self) {
        // The lethal-step counters have been consumed by the score boundary
        // before Classic clears run globals on Game Over or a hub return.
        self.death_stats_snapshot = None;
        let tunnel = characters::selected(self) == characters::Character::TunnelMan;
        self.player.health = if tunnel { 2.0 } else { 4.0 };
        self.player.bombs = if tunnel { 0.0 } else { 4.0 };
        self.player.ropes = if tunnel { 0.0 } else { 4.0 };
        self.player.money = 0;
        self.player.equipment.clear();
        for flag in [
            "ghost_exists",
            "ghostExists",
            "ghost_warning",
            "gameStart",
            "entityGen",
            "drawHUD",
            "check_water",
            "checkWater",
            "wanted",
            "murderer",
            "shopkeepers_angry",
            "made_udjat_eye",
            "madeUdjatEye",
            "genUdjatEye",
            "made_market_entrance",
            "madeMarketEntrance",
            "genMarketEntrance",
            "marketEntrance",
            "made_black_market",
            "madeBlackMarket",
            "genBlackMarket",
            "made_moai",
            "madeMoai",
            "made_gold_entrance",
            "madeGoldEntrance",
            "genGoldEntrance",
            "goldEntrance",
            "city_of_gold",
            "cityOfGold",
            "damsel_rescued",
            "sacrificed_damsel",
            "udjat_blink",
        ] {
            self.flags.remove(flag);
        }
        self.flags
            .retain(|flag| !flag.starts_with("carry:") && !flag.starts_with("title_start:"));
        self.flags.insert("title_start:0".into());
        self.flags.insert("no_dark_level".into());
        for key in [
            "collect",
            "collectCounter",
            "shake",
            "shakeToggle",
            "yviewPrev",
            "xview",
            "yview",
            "waterCounter",
            "crapsPoint",
            "bloodLevel",
            "thiefLevel",
            "favor",
            "kaliGift",
            "kaliPunish",
            "exitX",
            "exitY",
            "arrows",
            "time",
            "kills",
            "damsels",
            "gold",
            "goldbar",
            "goldbars",
            "nuggets",
            "rubies",
            "bigrubies",
            "sapphires",
            "bigsapphires",
            "emeralds",
            "bigemeralds",
            "diamonds",
            "scarabs",
            "idols",
            "skulls",
            "xdamsels",
            "xmoney",
            "xtime",
            "bats",
            "snakes",
            "spiders",
            "skeletons",
            "frogs",
            "firefrogs",
            "piranhas",
            "mantraps",
            "yetis",
            "aliens",
            "ufos",
            "cavemen",
            "hawkmen",
            "monkeys",
            "zombies",
            "vampires",
            "deadfish",
            "alienbosses",
            "giantspiders",
            "yetikings",
            "megamouths",
            "tomblords",
            "shopkeepers",
            "damselsKilled",
            "levelType",
            "collect_counter",
            "shake_toggle",
            "water_counter",
            "kali_gift",
            "kali_punish",
            "pending_damsel_kisses",
        ] {
            self.data.insert(key.into(), 0.);
        }
        for (key, value) in [
            ("probDarkLevel", 12.),
            ("probSnakePit", 8.),
            ("probCemetary", 10.),
            ("probLake", 10.),
            ("probYetiLair", 6.),
            ("probAlien", 10.),
            ("probSacPit", 8.),
            ("marketChance", 160.),
            ("goldChance", 160.),
            ("flares", 3.),
            ("arrows", 0.),
            ("currLevel", 1.),
            ("levelType", 0.),
        ] {
            self.data.insert(key.into(), value);
        }
    }

    /// Start a destination using the existing source RNG and persistent profile.
    pub fn start_from_hub(&mut self, tutorial: bool, level: u32) {
        // Reusing the Game for a fresh adventure must not let the previous
        // run's lethal-step counters override a later win record.
        self.death_stats_snapshot = None;
        let character = characters::selected(self);
        self.flags.retain(|f| {
            f.starts_with("unlock_")
                || f.starts_with("character_")
                || f.starts_with("shortcut")
                || f == "first_time"
                || f == "low_graphics"
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
            // The authored xSTART/xTUTORIAL branches set currLevel to zero
            // before entering rTutorial. This room bypasses scrInitLevel, so
            // publish the matching source globals here instead of leaving
            // them absent after the run-data reset above.
            self.data.insert("currLevel".into(), 0.0);
            self.data.insert("levelType".into(), 0.0);
            self.player.bombs = 0.0;
            self.player.ropes = 2.0;
            self.world = self.generate_tutorial_world();
        } else {
            if level > 1 {
                self.flags.insert("used_shortcut".into());
            }
            self.flags.insert("no_dark_level".into());
            // Shortcut portals call `room_goto(rLevel)` directly. Classic only
            // evaluates the Jungle Lake lottery from `oTransition`.
            self.generate_campaign_room_for_route(level.clamp(1, 16), false);
        }
        self.reset_position();
        traps::initialize_level(self);
    }

    /// Start a title-menu shortcut using Classic's direct destination-room
    /// route. The title portal stops all music before entering its selected
    /// room (oPlayer1 Step.xml:793; Animation end.xml:152-171).
    pub fn start_from_title_shortcut(&mut self, level: u32) {
        music::stop_all(self);
        self.start_from_hub(false, level);
        // reset_position clears the room event log while preserving the music
        // outbox. Restore the stop event ahead of the destination room's play.
        self.events.insert(0, GameEvent::MusicStopAll);
    }

    pub fn simulation_version(&self) -> u32 {
        match self.generation_revision {
            GenerationRevision::HistoricalV1 => HISTORICAL_SIMULATION_VERSION,
            GenerationRevision::SourceClassicV2 => SIMULATION_VERSION,
        }
    }

    fn generate_campaign_room(&mut self, level: u32) {
        self.generate_campaign_room_for_route(level, true);
    }

    fn generate_campaign_room_for_route(&mut self, level: u32, source_transition: bool) {
        // scrInitLevel publishes these source globals before generating the
        // room. Keep the serialized source namespace aligned with `World`,
        // including direct hub starts and ordinary transition loads.
        self.data.insert("currLevel".into(), f64::from(level));
        self.data
            .insert("levelType".into(), f64::from(campaign_level_type(level)));
        let context = generation::GenerationContext {
            has_spectacles: self.player.has("Spectacles"),
            has_cemetery: false,
            has_city_of_gold: self.flags.contains("cityOfGold")
                || self.flags.contains("city_of_gold"),
        };
        let historical = self.generation_revision == GenerationRevision::HistoricalV1;
        let market = if historical {
            160.0
        } else {
            self.data.get("marketChance").copied().unwrap_or(160.0)
        };
        let gold = if historical {
            160.0
        } else {
            self.data.get("goldChance").copied().unwrap_or(160.0)
        };
        let probabilities = if historical {
            generation::SourceProbabilities::default()
        } else {
            generation::SourceProbabilities::from_globals(&self.data)
        };
        // Invalid edited/synthetic histories are rejected BEFORE procedural
        // generation starts; neither signed state nor random draws are clamped.
        assert!(
            historical
                || (market.is_finite()
                    && market.fract() == 0.0
                    && (f64::from(generation::MIN_SUPPORTED_MARKET_CHANCE)..=f64::from(1 << 21))
                        .contains(&market)),
            "marketChance history is outside the proved integer campaign domain with one-room draw headroom"
        );
        assert!(
            historical
                || (gold.is_finite()
                    && gold.fract() == 0.0
                    && (f64::from(generation::MIN_SUPPORTED_SOURCE_CHANCE)..=f64::from(1 << 21))
                        .contains(&gold)),
            "goldChance history is outside the proved integer campaign domain with one-room draw headroom"
        );
        let mut state = generation::GenerationState {
            revision: self.generation_revision,
            market_chance: market as i32,
            gold_chance: gold as i32,
            made_market_entrance: self.flags.contains("made_market_entrance")
                || self.flags.contains("madeMarketEntrance"),
            probabilities,
        };
        #[cfg(test)]
        CAMPAIGN_ROOM_CREATE_HEALTH.with(|observed| {
            if observed.get().is_some() {
                observed.set(Some((level, self.player.health)));
            }
        });
        if !historical {
            generation::reset_dark_history(&mut self.flags, level);
        }
        let generated = generation::generate_campaign_result_for_route(
            &mut self.rng,
            level,
            &self.flags,
            context,
            &mut state,
            source_transition,
        );
        if generated.dark_roll_selected {
            self.flags.insert("had_dark_level".into());
        }
        if !historical {
            // scrInitLevel resets global.cemetary for the new room, then sets
            // it when the Jungle cemetery roll wins. Runtime Create callbacks
            // (leaves, branches, and idol traps) read this global after level
            // generation, so project the selected result out of the generator.
            self.flags.remove("cemetary");
            self.flags.remove("cemetery");
            if generated.cemetery_selected {
                self.flags.insert("cemetary".into());
            }
            // scrLevelGen resets the per-room sacrifice latch, selects the
            // pit before the City of Gold path override, and leaves the latch
            // set even if that override replaces one shaft cell.
            self.flags.remove("sacrifice_pit");
            self.flags.remove("sacrificePit");
            if generated.sacrifice_pit_selected {
                self.flags.insert("sacrifice_pit".into());
            }
        }
        self.world = generated.world;
        if !historical {
            // scrInitLevel runs scrEntityGen after terrain setup. Its final
            // statement clears global.cleanSolids after population, so the
            // exit-animation setting cannot leak into the newly loaded room.
            self.data.insert("cleanSolids".into(), 0.0);
        }
        // scrLevelGen sets global.madeMoai as soon as it schedules a Moai
        // room. Persist that source side effect with the generated room so a
        // hub start at level 9 suppresses another Moai on level 10.
        if !historical
            && self
                .world
                .entities
                .iter()
                .any(|entity| entity.alive && entity.object == "oMoai")
        {
            self.flags.insert("made_moai".into());
        }
        // `moai` only forces the room being generated. Once that request has
        // produced its terrain, consume it so it cannot override the
        // persistent `made_moai` guard on a later Ice level.
        self.flags.remove("moai");
        if !historical {
            self.data
                .insert("marketChance".into(), f64::from(state.market_chance));
            self.data
                .insert("goldChance".into(), f64::from(state.gold_chance));
            if state.made_market_entrance {
                self.flags.insert("made_market_entrance".into());
            }
        }
        // scrLevelGen sets madeBlackMarket before returning from the forced
        // layout. Keep that lifecycle edge synchronous with generation so a
        // persistent genBlackMarket cannot select the room again.
        if self.world.name == "Black Market" {
            self.flags.insert("made_black_market".into());
        }
    }

    pub fn next_level(&mut self) {
        crate::progression::consume_tunnel_first_encounter(self);
        let carry = self.take_transition_carry();
        let kisses = self.take_pending_kisses();
        let next = self
            .data
            .remove("next_level")
            .unwrap_or(f64::from(self.world.level + 1)) as u32;
        // The transition room's Room End event grants the rescue kiss before
        // leaving for either another level or the campaign ending.
        self.apply_kisses(kisses);
        if next > 16 {
            self.state = RunState::Won;
            self.events.push(GameEvent::Won);
            return;
        }
        self.flags.retain(|f| {
            !matches!(
                f.as_str(),
                "black_market"
                    | "blackMarket"
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
        // Classic resets the per-room special layout globals at the start of
        // scrLevelGen, before it selects the next room. Clear their previous
        // room values before passing flags into generation so they cannot
        // force the same special layout to repeat.
        if self.generation_revision == GenerationRevision::SourceClassicV2 {
            // oTransition Draw updates noDarkLevel from the completed room's
            // elapsed seconds before the player can confirm into the next
            // room. The source uses floor(xtime / 1000), then wraps its
            // displayed seconds at 60; only seconds 0 through 20 suppress the
            // next room's dark-level roll.
            let seconds = (self.data.get("xtime").copied().unwrap_or(0.0) / 1000.0).floor();
            let seconds = if seconds > 59.0 {
                seconds % 60.0
            } else {
                seconds
            };
            if seconds <= 20.0 {
                self.flags.insert("no_dark_level".into());
            } else {
                self.flags.remove("no_dark_level");
            }
        }
        self.generate_campaign_room(next);
        for e in &self.world.entities {
            match e.object.as_str() {
                "oKey"
                    if e.alive
                        && self.generation_revision == GenerationRevision::SourceClassicV2 =>
                {
                    self.flags.insert("made_udjat_eye".into());
                }
                "oLockedChest" if self.generation_revision == GenerationRevision::HistoricalV1 => {
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
        self.restore_transition_carry(carry);
        traps::initialize_level(self);
        if !self.flags.contains("wanted") && !self.flags.contains("murderer") {
            self.flags.remove("shopkeepers_angry");
        }
        if self.generation_revision == GenerationRevision::SourceClassicV2 {
            // Room Create completes during the source transition. This runs
            // after generation and reset_position, with exits and player
            // placement finalized, so wanted exit keepers exist before Draw.
            progression::initialize_level(self);
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
            self.player.health = (self.player.health + 1.0).min(99.0);
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
        if let Some(item_type) = carry.as_deref().and_then(items::explosives::object_type) {
            items::explosives::hold_item_type(self, item_type);
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
        if self.flags.contains("hub_scores") {
            self.camera = Vec2::new(
                self.data.get("xview").copied().unwrap_or(0.0),
                self.data.get("scores_yview").copied().unwrap_or(0.0),
            );
            return;
        }
        if self.flags.contains("hub") {
            self.camera = Vec2::default();
            return;
        }
        // Native pre-Draw follow selects the first unmarked current active
        // descendant. This lookup must not register or create any instances.
        let target = match &self.view_target {
            // The physical Game adapter has one implicit Player (except the
            // guarded opening above). This also retains its pre-frame entrance
            // projection; normal level frames activate that Player explicitly.
            ViewTarget::Player => Some(self.player.pos),
            ViewTarget::Object(object) => dispatcher::descendant_instances(self, object)
                .into_iter()
                .find(|instance| match instance {
                    dispatcher::Instance::Player => true,
                    dispatcher::Instance::Entity(id) => self
                        .world
                        .entities
                        .iter()
                        .chain(self.dispatcher.hidden_entities().iter())
                        .any(|entity| entity.id == *id && entity.alive),
                    dispatcher::Instance::Terrain(index) => self
                        .world
                        .tiles
                        .get(*index)
                        .is_some_and(|tile| !tile.object.is_empty()),
                })
                .map(|instance| instance.position(self)),
            ViewTarget::None => None,
        };
        let Some(target) = target else {
            // No active receiver means no coordinate writes, including clamps.
            return;
        };
        let boss_room = self.world.area == Area::Boss && !self.flags.contains("custom");
        let hborder = self.data.get("camera_hborder").copied().unwrap_or(128.0);
        let vborder = self
            .data
            .get("camera_vborder")
            .copied()
            .unwrap_or(if boss_room { 64.0 } else { 96.0 });
        self.camera = Vec2::new(
            source_follow_axis(
                self.camera.x,
                target.x,
                hborder,
                VIEW_WIDTH,
                self.world.width as f64 * 16.0,
            ),
            source_follow_axis(
                self.camera.y,
                target.y,
                vborder,
                VIEW_HEIGHT,
                self.world.height as f64 * 16.0,
            ),
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
mod damsel_room_end_order_tests {
    use super::*;

    #[test]
    fn room_end_kiss_is_applied_before_campaign_room_create() {
        let mut game = Game::shell(11, RulesPreset::Classic);
        game.player.health = 2.0;
        game.data.insert("pending_damsel_kisses".into(), 2.0);
        game.data.insert("next_level".into(), 2.0);
        CAMPAIGN_ROOM_CREATE_HEALTH.with(|observed| {
            observed.set(Some((0, f64::NAN)));
        });

        game.next_level();

        let health_seen_by_create = CAMPAIGN_ROOM_CREATE_HEALTH.with(|observed| {
            let value = observed.get();
            observed.set(None);
            value
        });
        assert_eq!(health_seen_by_create, Some((2, 3.0)));
        assert_eq!(game.player.health, 3.0);
    }

    #[test]
    fn room_end_kiss_is_applied_before_campaign_win_returns() {
        let mut game = Game::shell(11, RulesPreset::Classic);
        game.world.level = 16;
        game.player.health = 2.0;
        game.data.insert("pending_damsel_kisses".into(), 1.0);
        game.data.insert("next_level".into(), 17.0);

        game.next_level();

        assert_eq!(game.state, RunState::Won);
        assert!(game.events.contains(&GameEvent::Won));
        assert_eq!(game.player.health, 3.0);
        assert!(!game.data.contains_key("pending_damsel_kisses"));
    }
}

#[cfg(test)]
mod classic_score_eligibility_tests {
    use crate::{Game, RulesPreset, hub::HubKind, level::Level};

    #[test]
    fn custom_score_suppression_survives_scores_until_title_create() {
        let mut game = Game::shell(7, RulesPreset::Classic);
        game.load_custom(&Level::default(), "custom", "");
        assert!(game.flags.contains("scores_disabled"));

        game.load_hub(HubKind::Scores);
        assert!(game.flags.contains("scores_disabled"));

        game.load_hub(HubKind::Title);
        assert!(!game.flags.contains("scores_disabled"));
    }
}

#[cfg(test)]
mod scores_hub_terminal_entry_tests {
    use crate::{Game, InputFrame, PlayerState, RulesPreset, RunState, hub::HubKind};

    #[test]
    fn campaign_win_clears_run_state_and_scores_room_resumes_simulation() {
        let mut game = Game::shell(7, RulesPreset::Classic);
        game.state = RunState::Won;
        game.player.money = 123_456;
        game.player.equipment.insert("Cape".into());
        game.data.insert("kills".into(), 37.0);
        game.data.insert("time".into(), 54_000.0);
        game.flags.insert("won".into());
        game.flags.insert("reset_run_on_hub".into());

        game.load_hub(HubKind::Scores);

        assert_eq!(game.state, RunState::Playing);
        assert_ne!(game.player.state, PlayerState::Dead);
        assert_eq!(game.player.money, 0);
        assert!(game.player.equipment.is_empty());
        assert_eq!(game.data["kills"], 0.0);
        assert_eq!(game.data["time"], 0.0);
        assert!(!game.flags.contains("reset_run_on_hub"));
        assert!(game.flags.contains("hub_scores"));

        let tick = game.level_tick;
        game.step(&InputFrame::default());
        assert_eq!(game.level_tick, tick + 1, "Scores must continue stepping");
    }

    #[test]
    fn transition_into_scores_hub_resumes_simulation_after_death_reset() {
        let mut game = Game::shell(7, RulesPreset::Classic);
        game.state = RunState::Transition;
        game.player.state = PlayerState::Dead;
        game.flags.insert("reset_run_on_hub".into());

        game.load_hub(HubKind::Scores);

        assert_eq!(game.state, RunState::Playing);
        assert_eq!(game.player.health, 4.0);
        assert!(game.player.equipment.is_empty());
        let tick = game.level_tick;
        game.step(&InputFrame::default());
        assert_eq!(game.level_tick, tick + 1, "Scores must continue stepping");
    }
}

#[cfg(test)]
mod campaign_cemetery_runtime_flag_tests {
    use crate::{Game, RulesPreset, Vec2};

    #[test]
    fn naturally_selected_cemetery_reaches_runtime_create_and_idol_callbacks() {
        let mut game = Game::shell(7, RulesPreset::Classic);
        game.data.insert("next_level".into(), 5.0);
        game.next_level();

        assert_eq!(game.world.name, "Cemetery");
        assert!(game.flags.contains("cemetary"));
        assert!(!game.flags.contains("cemetery"));

        // oLeaves Create reads global.cemetary directly. At the left boundary,
        // no tree/leaf neighbor changes the right-facing sprite variant.
        let leaves = game.spawn("oLeaves", Vec2::new(16.0, 16.0));
        assert_eq!(game.world.entity(leaves).unwrap().sprite, "sLeavesDead");

        // The Jungle idol callback also reads the same global to choose its
        // cemetery ghost response.
        let idol = game.spawn("oGoldIdol", Vec2::new(320.0, 320.0));
        crate::traps::altar::idol_pickup(&mut game, idol);
        assert!(
            game.world
                .entities
                .iter()
                .any(|entity| { entity.alive && entity.object == "oGhost" })
        );
    }
}

#[cfg(test)]
mod source_probability_global_tests {
    use crate::{Game, RulesPreset};

    #[test]
    fn serialized_campaign_probability_override_controls_the_next_room() {
        let mut game = Game::shell(0, RulesPreset::Classic);
        game.world.level = 6;
        game.data.insert("probLake".into(), 1.0);

        let serialized = serde_json::to_value(&game).unwrap();
        let mut restored: Game = serde_json::from_value(serialized).unwrap();
        restored.next_level();

        assert_eq!(restored.world.level, 7);
        assert_eq!(restored.world.name, "Lake");
        assert_eq!(restored.data.get("probLake"), Some(&1.0));
    }
}

#[cfg(test)]
#[path = "source_gold_chance_tests.rs"]
mod source_gold_chance_tests;

#[cfg(test)]
mod boss_terrain_spectacles_context_tests {
    use crate::{Game, GenerationRevision, RulesPreset, Vec2};

    fn generated_underground_spring_shoes(has_spectacles: bool) -> crate::Entity {
        let mut game = Game::shell(0, RulesPreset::Classic);
        game.world.level = 15;
        game.data.insert("next_level".into(), 16.0);
        if has_spectacles {
            game.player.equipment.insert("Spectacles".into());
        }

        // Exercise the real campaign transition and its SourceClassicV2
        // generation context rather than calling the generator directly.
        game.next_level();

        assert_eq!(game.world.level, 16);
        assert_eq!(
            game.generation_revision,
            GenerationRevision::SourceClassicV2
        );
        game.world
            .entities
            .into_iter()
            .find(|entity| {
                entity.alive
                    && entity.object == "oSpringShoes"
                    && entity.pos == Vec2::new(344.0, 664.0)
                    && entity.value("source_pickup_create_initialized") == 1.0
            })
            .expect("fixed stock level-16 fixture must generate buried Spring Shoes")
    }

    #[test]
    fn boss_bottom_row_terrain_create_receives_campaign_spectacles_context() {
        let without_spectacles = generated_underground_spring_shoes(false);
        let with_spectacles = generated_underground_spring_shoes(true);

        // The source oItem Create puts Underground items at depth 101 normally,
        // but at depth 51 when global.spectacles is active.
        assert_eq!(without_spectacles.value("depth"), 101.0);
        assert_eq!(with_spectacles.value("depth"), 51.0);
    }
}

#[cfg(test)]
mod tests {
    // Author a legal room, then use only actual tick inputs. No midrun player,
    // image-index, animation-speed or controller-counter assignments.
    fn natural_duck_to_hang_callback(right: bool) -> Game {
        let mut level = crate::level::Level::default();
        level
            .set_tile(if right { 12 } else { 14 }, 18, '@')
            .unwrap();
        let platform = if right { 3..=13 } else { 14..=24 };
        for x in platform {
            level.set_tile(x, 19, '1').unwrap();
        }
        let mut game = Game::shell(42, crate::RulesPreset::Classic);
        game.load_custom(&level, "natural-duck-to-hang", "");
        for _ in 0..4 {
            game.step(&crate::InputFrame::default());
        }
        assert!(game.player.grounded, "authored entrance did not settle");
        let mut entered = false;
        for _ in 0..80 {
            game.step(&crate::InputFrame {
                down: true,
                right,
                left: !right,
                ..Default::default()
            });
            entered |= game.player.data.get("duck_to_hang") == Some(&1.);
            if entered && game.presentation().player.pos != game.player.pos {
                assert_eq!(game.presentation().player.sprite, "sDuckToHangL");
                assert_eq!(game.player.state, crate::PlayerState::Hanging);
                return game;
            }
        }
        panic!(
            "natural crawl failed to finish duck-to-hang: right={right}, state={:?}, pos={:?}, sprite={}, animation={}",
            game.player.state, game.player.pos, game.player.sprite, game.player.animation
        );
    }
    fn natural_duck_to_hang_animation_end_keeps_draw_camera_in_actor_phase(right: bool) {
        {
            let mut game = natural_duck_to_hang_callback(right);
            let drawn = game.presentation();
            // Animation End moves the live actor, but the source last Draw
            // frame remains frozen at its original position/image index.
            assert_eq!(game.player.pos.y - drawn.player.pos.y, 16.);
            assert_eq!(
                game.player.pos.x - drawn.player.pos.x,
                if right { 6. } else { -5. }
            );
            assert_eq!(game.player.previous_pos, game.player.pos);
            assert_eq!(drawn.player.previous_pos, drawn.player.pos);
            assert_eq!(drawn.player.sprite, "sDuckToHangL");
            assert_eq!(
                drawn.player.animation.floor() as usize,
                catalog::sprite_frame_count("sDuckToHangL") - 1
            );
            assert!(!drawn.flags.contains("render_discontinuity"));
            assert_eq!(
                game.camera.y,
                game.player.pos.y - 144.,
                "authoritative camera changed"
            );
            assert_eq!(
                drawn.previous_camera.y,
                drawn.player.pos.y - 144.,
                "captured previous camera changed"
            );
            assert_eq!(
                drawn.camera.y,
                drawn.player.pos.y - 144.,
                "Draw camera followed post-Animation actor"
            );
            let previous_draw_end_y = drawn.player.pos.y - drawn.camera.y;
            let live_position = game.player.pos;
            let live_camera = game.camera;
            game.step(&InputFrame::default());
            assert_eq!(
                game.player.pos, live_position,
                "source hanging actor unexpectedly moved"
            );
            assert_eq!(
                game.camera, live_camera,
                "source hanging camera unexpectedly moved"
            );
            assert_eq!(game.presentation().player.sprite, "sHangLeft");
            let next = game.presentation();
            assert_eq!(
                next.player.previous_pos.y - next.previous_camera.y,
                previous_draw_end_y,
                "joined tick boundary displaced the player on screen"
            );
        }
    }
    #[test]
    fn natural_duck_to_hang_left_camera_in_actor_phase() {
        natural_duck_to_hang_animation_end_keeps_draw_camera_in_actor_phase(false);
    }
    #[test]
    fn natural_duck_to_hang_right_camera_in_actor_phase() {
        natural_duck_to_hang_animation_end_keeps_draw_camera_in_actor_phase(true);
    }

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

    #[test]
    fn ordinary_jungle_transition_keeps_source_lake_lottery() {
        // With this exact incoming RNG state, source rand(1, probLake) wins.
        let mut game = Game::shell(0, RulesPreset::Classic);
        game.world.level = 4;
        game.data.insert("next_level".into(), 5.0);

        game.next_level();

        assert_eq!(game.world.level, 5);
        assert_eq!(game.world.name, "Lake");
    }

    #[test]
    fn transition_clears_prior_ice_special_before_generating_next_level() {
        let mut start = Game::shell(0, RulesPreset::Classic);
        start.start_from_hub(false, 9);
        let mut expected = start.clone();
        let mut actual = start;
        expected.data.insert("next_level".into(), 10.0);
        actual.data.insert("next_level".into(), 10.0);
        actual.flags.insert("yeti_lair".into());

        expected.next_level();
        actual.next_level();

        assert_eq!(expected.world.name, "Ice Caves");
        assert_eq!(
            actual.world.name, expected.world.name,
            "the previous level's Yeti flag must be cleared before new room generation"
        );
    }
    #[test]
    fn campaign_level_type_matches_authored_scr_init_level_ranges() {
        for (level, expected_type) in [
            (1, 0),
            (2, 0),
            (4, 0),
            (5, 1),
            (8, 1),
            (9, 2),
            (12, 2),
            (13, 3),
            (15, 3),
            (16, 4),
        ] {
            assert_eq!(campaign_level_type(level), expected_type, "level {level}");
        }
    }

    #[test]
    fn direct_campaign_room_setup_publishes_source_level_globals() {
        let mut game = Game::shell(17, RulesPreset::Classic);
        for (level, expected_type) in [(2, 0.), (5, 1.), (9, 2.), (13, 3.), (16, 4.)] {
            game.start_from_hub(false, level);
            assert_eq!(game.world.level, level);
            assert_eq!(game.data["currLevel"], f64::from(level));
            assert_eq!(game.data["levelType"], expected_type, "level {level}");
        }
    }

    #[test]
    fn title_shortcut_matches_direct_classic_destination_and_replaces_title_music() {
        // Classic oPlayer1 Animation end.xml:152-171 assigns currLevel 5/9/13
        // and enters the selected destination directly; Step.xml:793 stops
        // title music before that room starts its own track.
        let music = SourceMusicState::default();
        for (level, track) in [(5, "musLush")] {
            let seed = 0x51_000 + level;
            let mut actual = Game::shell_with_music(seed, RulesPreset::Classic, music.clone());
            actual.set_graphics_high(true);
            actual.start_from_title_shortcut(level);

            let mut direct = Game::shell_with_music(seed, RulesPreset::Classic, music.clone());
            direct.set_graphics_high(true);
            direct.start_from_hub(false, level);

            assert_eq!(
                serde_json::to_value(&actual.world).unwrap(),
                serde_json::to_value(&direct.world).unwrap(),
                "level {level} must use the direct destination generation route"
            );
            assert_eq!(actual.data, direct.data, "level {level}");
            assert_eq!(actual.flags, direct.flags, "level {level}");
            assert_eq!(
                serde_json::to_value(&actual.player).unwrap(),
                serde_json::to_value(&direct.player).unwrap(),
                "level {level}"
            );
            assert_eq!(
                actual.rng.state_bits(),
                direct.rng.state_bits(),
                "level {level}"
            );
            assert_eq!(actual.world.level, level);
            assert!(actual.flags.contains("used_shortcut"));
            assert_eq!(
                actual.source_music.take_commands(),
                [
                    SourceMusicCommand::StopAll,
                    SourceMusicCommand::Play {
                        name: track.into(),
                        looping: true,
                        if_idle: true,
                    },
                    SourceMusicCommand::Frequency {
                        name: track.into(),
                        hz: 44_100,
                    },
                ],
                "level {level} must stop title music before starting its room track"
            );
            assert!(matches!(
                &actual.events[..2],
                [
                    GameEvent::MusicStopAll,
                    GameEvent::MusicPlay { name, looping: true, if_idle: true }
                ] if name == track
            ));
        }
    }

    #[test]
    fn direct_tutorial_constructor_enters_classics_zero_level_context() {
        let game = Game::tutorial(17, RulesPreset::Classic);

        assert_eq!(game.world.name, "Tutorial Cave");
        assert_eq!(game.world.level, 0);
        assert_eq!(game.data["currLevel"], 0.0);
        assert_eq!(game.data["levelType"], 0.0);
        assert_eq!(game.player.bombs, 0.0);
        assert_eq!(game.player.ropes, 2.0);
    }

    #[test]
    fn tutorial_start_publishes_zero_level_globals_after_clearing_run_data() {
        let mut game = Game::shell(17, RulesPreset::Classic);
        game.data.insert("currLevel".into(), 13.0);
        game.data.insert("levelType".into(), 3.0);
        game.flags.insert("low_graphics".into());
        game.start_from_hub(true, 1);

        assert_eq!(game.world.name, "Tutorial Cave");
        assert!(game.flags.contains("tutorial"));
        assert_eq!(game.world.level, 0);
        assert_eq!(game.data["currLevel"], 0.0);
        assert_eq!(game.data["levelType"], 0.0);
        assert!(game.flags.contains("low_graphics"));
        assert!(
            !game
                .world
                .entities
                .iter()
                .any(|e| { e.object == "oGenerationDecoration" && e.value("decoration") != 0.0 })
        );
    }

    #[test]
    fn run_resets_discard_the_previous_lethal_stats_snapshot() {
        let mut game = Game::shell(17, RulesPreset::Classic);
        let previous = crate::DeathStatsSnapshot {
            money: 50,
            kills: 2,
            saves: 3,
        };
        game.death_stats_snapshot = Some(previous);

        game.clear_run_globals();
        assert_eq!(game.death_stats_snapshot, None);

        game.death_stats_snapshot = Some(previous);
        game.start_from_hub(false, 1);
        assert_eq!(game.death_stats_snapshot, None);
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
    fn camera_follow_reaches_draw_snapshot_without_advancing_frozen_actor() {
        let mut game = flat_game();
        game.player.pos.x = 224.0;
        game.player.previous_pos = game.player.pos;
        game.player.state = PlayerState::Running;
        game.player.grounded = true;
        game.player.vel.x = 3.0;
        game.camera.x = 32.0;
        game.previous_camera = game.camera;
        let before_camera = game.camera;
        game.step(&InputFrame {
            right: true,
            ..Default::default()
        });
        assert!(game.camera.x > before_camera.x);
        let drawn = game.presentation();
        assert_eq!(drawn.previous_camera, before_camera);
        assert_eq!(drawn.camera, game.camera);
        assert_eq!(drawn.player.pos, game.player.pos);
        assert!(drawn.player.image_speed() > 0.0);
        assert_ne!(
            drawn.player.animation, game.player.animation,
            "following the camera must not replace the pre-Animation actor"
        );
    }

    #[test]
    fn held_camera_look_survives_follow_update_and_restarts_after_release() {
        let mut game = flat_game();
        for x in 0..game.world.width {
            game.world.set_tile(x, 13, "oBlock");
        }
        game.player.pos = Vec2::new(40.0, 200.0);
        game.player.previous_pos = game.player.pos;
        game.player.vel = Vec2::default();
        game.player.state = PlayerState::Standing;
        game.player.grounded = true;
        game.camera.y = 100.0;
        game.previous_camera = game.camera;

        let up = InputFrame {
            up: true,
            ..Default::default()
        };
        for _ in 0..31 {
            game.step(&up);
        }
        assert_eq!(game.camera.y, 100.0, "source waits through 31 held ticks");
        game.step(&up);
        assert_eq!(
            game.camera.y, 96.0,
            "the 32nd held tick moves the camera and camera follow keeps it"
        );

        game.step(&InputFrame::default());
        assert_eq!(game.player.data.get("view_count"), None);
        let down = InputFrame {
            down: true,
            ..Default::default()
        };
        for _ in 0..31 {
            game.step(&down);
        }
        assert_eq!(game.camera.y, 96.0);
        game.step(&down);
        assert_eq!(game.camera.y, 100.0);
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
    fn lethal_step_snapshots_campaign_stats_before_posthumous_collection() {
        let mut game = flat_game();
        game.player.health = 0.0;
        game.player.money = 50;
        game.data.insert("collect".into(), 150.0);
        game.data.insert("collectCounter".into(), 0.0);
        game.data.insert("kills".into(), 2.0);
        game.data.insert("damsels".into(), 3.0);

        game.step(&InputFrame::default());

        assert_eq!(game.state, RunState::Dead);
        assert_eq!(
            game.player.money, 150,
            "Classic still pays the collection later in this Step"
        );
        assert_eq!(
            game.death_stats_snapshot,
            Some(crate::DeathStatsSnapshot {
                money: 50,
                kills: 2,
                saves: 3,
            })
        );
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
        assert_eq!(game.player.health, 4.0);
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
        game.player.health = 2.0;
        game.next_level();
        assert_eq!(game.player.health, 3.0);
        let held = game.world.entity(game.player.held.unwrap()).unwrap();
        assert_eq!(held.object, "oShotgun");
        assert!(!held.data.contains_key("ammo"));
        assert_eq!(held.value("cost"), 0.);
        assert_eq!(held.value("new"), 0.);
        assert!(held.held);
    }

    #[test]
    fn custom_default_next_level_preserves_none_for_catalog_lookup() {
        let mut game = Game::shell(9, RulesPreset::Classic);
        let level = level::Level::default();
        assert_eq!(level.next_level, "NONE");

        game.load_custom(&level, "first", "");

        assert!(game.flags.contains("default_next_custom:NONE"));
    }

    #[test]
    fn linked_custom_room_clears_exit_clean_solids_before_destroy_callbacks() {
        let level = level::Level::default();
        let mut game = Game::shell(9, RulesPreset::Classic);
        game.load_custom(&level, "first", "");

        // The previous room's exit Animation End sets this global before the
        // host loads the linked custom room.
        game.data.insert("cleanSolids".into(), 1.0);
        game.flags.insert("cleanSolids".into());
        game.continue_custom(&level, "second", "first");

        assert_eq!(game.data.get("cleanSolids"), Some(&0.0));
        assert!(!game.flags.contains("cleanSolids"));

        let frozen = game.world.spawn("oFrozenCaveman", Vec2::new(80.0, 64.0));
        world_events::destroy_frozen_caveman(&mut game, frozen);
        assert!(game.world.entities.iter().any(|entity| {
            entity.alive && entity.object == "oCaveman" && entity.pos == Vec2::new(80.0, 64.0)
        }));
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
        game.player.health = 2.0;
        game.player.bombs = 1.0;
        game.player.ropes = 0.0;
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
            (2.0, 1.0, 0.0)
        );
        assert_eq!(game.player.money, 700);
        assert!(game.player.has("Cape"));
        let carried = game.world.entity(game.player.held.unwrap()).unwrap();
        assert_eq!(carried.object, "oShotgun");
        assert!(!carried.data.contains_key("ammo"));
        assert!(!game.flags.contains("carry:oShotgun"));
    }

    #[test]
    fn custom_room_terrain_and_enemy_creates_follow_source_birth_order() {
        let mut game = Game::shell(0, RulesPreset::Classic);
        let mut level = level::Level::default();
        level.set_tile(2, 2, 'p').unwrap();
        level.set_tile(3, 2, '1').unwrap();

        // The Piranha is born in the first map cell and draws its facing before
        // the next cell's Brick runs its variant/content Create rolls.
        let mut expected = game.rng.clone();
        let expected_facing = if expected.range(2) == 0 { -1 } else { 1 };
        expected.chance(1, 10);
        let brick_roll = expected.range(100) + 1;
        if brick_roll >= 30 {
            let mut treasure = false;
            for denominator in [100, 120, 140] {
                if expected.chance(1, denominator) {
                    treasure = true;
                    break;
                }
            }
            if !treasure && expected.chance(1, 1200) {
                expected.range(19);
            }
        }

        game.load_custom(&level, "create-order", "");
        let piranha = game
            .world
            .entities
            .iter()
            .find(|entity| entity.alive && entity.object == "oPiranha")
            .expect("the custom p tile creates a Piranha");
        assert_eq!(piranha.facing, expected_facing);
        assert_ne!(
            game.rng.state_bits(),
            expected.state_bits(),
            "scrInitLevel continues with boundaries, path generation, and wall setup"
        );

        // Its first Step then creates a Bubble before choosing the next timer,
        // continuing after custom-room SetupWalls and background setup.
        expected = game.rng.clone();
        expected.range(3);
        let expected_bubble_timer = 30 + expected.range(21);
        game.step(&InputFrame::default());
        let piranha = game
            .world
            .entities
            .iter()
            .find(|entity| entity.alive && entity.object == "oPiranha")
            .expect("the Piranha remains in the pool");
        assert_eq!(
            piranha.value("bubble_timer"),
            f64::from(expected_bubble_timer)
        );
        assert_eq!(game.rng.state_bits(), expected.state_bits());
    }

    #[test]
    fn source_custom_setup_builds_boundary_walls_and_autotiles_authored_bricks() {
        let mut game = Game::try_shell(73, RulesPreset::Classic).unwrap();
        game.set_graphics_high(false);
        let mut level = level::Level::default();
        level.set_tile(5, 5, '1').unwrap();

        game.load_custom(&level, "wall-fixture", "");

        let width = game.world.width;
        let brick_index = 6 * width + 6;
        assert_eq!(game.world.tiles[brick_index].object, "oBrick");
        let brick_pos = Vec2::new(6.0 * 16.0 + 8.0, 6.0 * 16.0 + 8.0);
        let brick = game
            .world
            .entities
            .iter()
            .find(|entity| {
                entity.alive && entity.object == "oTerrainMetadata" && entity.pos == brick_pos
            })
            .expect("authored Brick retains terrain metadata");
        assert_eq!(brick.value("terrain_setup_initialized"), 1.0);
        assert_ne!(
            brick.text.get("terrain_sprite").map(String::as_str),
            Some("sBrick"),
            "SetupWalls assigns an edge sprite to an isolated authored Brick"
        );
        assert_eq!(game.world.tiles[0].object, "oBrick");
        assert!(game.world.entities.iter().any(|entity| {
            entity.alive
                && entity.object == "oTerrainMetadata"
                && entity.value("source_boundary") == 1.0
                && entity.pos == Vec2::new(8.0, 8.0)
        }));

        let mut city_named_file = Game::try_shell(74, RulesPreset::Classic).unwrap();
        city_named_file.set_graphics_high(false);
        let mut temple_level = level::Level::default();
        temple_level.set_tile(5, 5, '4').unwrap();
        city_named_file.load_custom(&temple_level, "City of Gold", "");
        let temple_pos = Vec2::new(6.0 * 16.0 + 8.0, 6.0 * 16.0 + 8.0);
        let temple = city_named_file
            .world
            .entities
            .iter()
            .find(|entity| {
                entity.alive && entity.object == "oTerrainMetadata" && entity.pos == temple_pos
            })
            .expect("authored Temple retains terrain metadata");
        assert!(
            temple
                .text
                .get("terrain_sprite")
                .is_some_and(|sprite| sprite.starts_with("sTemple")),
            "custom filenames do not grant the City of Gold world flag"
        );
    }

    #[test]
    fn source_custom_room_projects_create_fields_before_custom_overrides() {
        let mut game = Game::try_shell(17, crate::RulesPreset::Classic).unwrap();
        let mut level = crate::level::Level::default();
        level.set_tile(2, 2, 'c').unwrap();
        level.set_tile(5, 2, 'X').unwrap();
        level.set_tile(8, 2, '!').unwrap();
        level.exit_targets = vec!["next-room".into()];

        game.load_custom(&level, "source-custom-create", "previous-room");

        let chest = game
            .world
            .entities
            .iter()
            .find(|entity| entity.alive && entity.object == "oChest")
            .expect("custom chest was created");
        assert_eq!(chest.text.get("type").map(String::as_str), Some("Chest"));
        assert_eq!(chest.value("active"), 1.0);
        assert_eq!(chest.value("collision_left"), -6.0);
        assert_eq!(chest.value("collision_right"), 6.0);
        assert_eq!(chest.value("yAcc"), 0.2);

        let exit = game
            .world
            .entities
            .iter()
            .find(|entity| entity.alive && entity.object == "oExit")
            .expect("custom exit was created");
        assert_eq!(exit.text.get("type").map(String::as_str), Some("Exit"));
        assert_eq!(
            exit.text.get("leadsTo").map(String::as_str),
            Some("next-room")
        );

        let keeper = game
            .world
            .entities
            .iter()
            .find(|entity| entity.alive && entity.object == "oShopkeeper")
            .expect("custom shopkeeper was created");
        assert_eq!(
            keeper.text.get("type").map(String::as_str),
            Some("Shopkeeper")
        );
        assert_eq!(keeper.value("collision_left"), 2.0);
        assert_eq!(keeper.value("collision_top"), 0.0);
        assert_eq!(keeper.value("collision_right"), 14.0);
        assert_eq!(keeper.value("collision_bottom"), 16.0);
    }

    #[test]
    fn editor_test_load_preserves_real_inventory_while_regular_load_clamps() {
        let mut level = level::Level::default();
        level.author = "UNSAVED AUTHOR".into();
        level.starting_life = 0;
        level.starting_bombs = 120;
        level.starting_ropes = -3;

        let mut regular = Game::shell(4, RulesPreset::Classic);
        regular.load_custom(&level, "regular", "");
        assert_eq!(
            (
                regular.player.health,
                regular.player.bombs,
                regular.player.ropes
            ),
            (1.0, 99.0, 0.0),
            "ordinary loading applies scrLoadLevel inventory clamps"
        );

        let mut editor = Game::shell(4, RulesPreset::Classic);
        editor.load_custom_editor_test(&level, "editor", "", [0.75, 12.9, 1000.25]);
        assert_eq!(
            editor.custom_level_author.as_deref(),
            Some("UNSAVED AUTHOR"),
            "F5 loads author metadata from the editor's test.tmp snapshot"
        );
        assert_eq!(
            (
                editor.player.health,
                editor.player.bombs,
                editor.player.ropes
            ),
            (0.75, 12.9, 1000.25),
            "editor test room assigns real(raw inventory text)"
        );
        editor.damage_player(1);
        assert_eq!(
            editor.player.health, -0.25,
            "real health loses integer damage"
        );
    }

    #[test]
    fn editor_test_fractional_life_below_one_dies_on_player_step() {
        let level = level::Level::default();
        let mut game = Game::shell(4, RulesPreset::Classic);
        game.load_custom_editor_test(&level, "editor", "", [0.75, 4.0, 4.0]);
        game.flags.insert("editor_test".into());

        game.step(&InputFrame::default());

        assert_eq!(game.state, RunState::Dead);
        assert_eq!(game.player.health, 0.0);
        assert!(
            game.events
                .iter()
                .any(|event| matches!(event, GameEvent::Died))
        );
    }

    #[test]
    fn editor_test_fractional_life_below_one_uses_ankh_before_death() {
        let level = level::Level::default();
        let mut game = Game::shell(4, RulesPreset::Classic);
        game.load_custom_editor_test(&level, "editor", "", [0.75, 4.0, 4.0]);
        game.flags.insert("editor_test".into());
        game.player.equipment.insert("Ankh".into());

        game.step(&InputFrame::default());

        assert_eq!(game.state, RunState::Playing);
        assert_eq!(game.player.health, 4.0);
        assert!(!game.player.has("Ankh"));
    }

    #[test]
    fn editor_test_starting_life_of_one_does_not_trip_fractional_death_threshold() {
        let level = level::Level::default();
        let mut game = Game::shell(4, RulesPreset::Classic);
        game.load_custom_editor_test(&level, "editor", "", [1.0, 4.0, 4.0]);
        game.flags.insert("editor_test".into());

        game.step(&InputFrame::default());

        assert_eq!(game.state, RunState::Playing);
        assert_eq!(game.player.health, 1.0);
    }

    #[test]
    fn completed_exit_recreates_source_controller_and_moves_in_linked_room() {
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
                    // Keep the exact original seed. Generated campaign rooms
                    // need a fresh character controller, but do not promise
                    // the authored linked room's broad movement corridor.
                    let mut game = Game::shell(8, rules);
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
                    let inventory = (
                        game.player.health,
                        game.player.bombs,
                        game.player.ropes,
                        game.player.money,
                        game.player.equipment.clone(),
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
                    // Literal characterCreateEvent + oPlayer1 Create/reset
                    // contract, independent of collision-scene headroom.
                    assert_eq!(game.state, RunState::Playing);
                    assert!(game.ending.is_none());
                    assert_eq!(game.level_tick, 0);
                    assert_eq!(game.player.pos, game.world.entrance);
                    assert_eq!(game.player.previous_pos, game.world.entrance);
                    assert_eq!(game.player.vel, Vec2::default());
                    assert_eq!(game.player.state, crate::PlayerState::Falling);
                    assert_eq!(game.player.facing, 1);
                    assert!(!game.player.grounded);
                    assert!(game.player.engine_active);
                    assert_eq!(game.player.data.get("active"), Some(&1.));
                    assert!(!game.player.is_exiting());
                    assert!(!game.player.is_stunned());
                    assert_eq!(game.player.stun, 0);
                    assert_eq!(game.player.attack_timer, 0);
                    assert_eq!(game.player.invincible, 0);
                    assert_eq!(game.player.animation, 0.);
                    assert!(game.player.held.is_none());
                    assert!(game.player.pickup_item_type.is_empty());
                    assert_eq!(
                        game.player.sprite,
                        match character {
                            Character::Explorer => "sStandLeft",
                            Character::Damsel => "sDamselLeft",
                            Character::TunnelMan => "sTunnelLeft",
                        }
                    );
                    assert_eq!(
                        game.player.bounds(),
                        Rect::new(
                            game.world.entrance.x - 5.,
                            game.world.entrance.y - 8.,
                            10.,
                            16.
                        )
                    );
                    assert_eq!(
                        (
                            game.player.health,
                            game.player.bombs,
                            game.player.ropes,
                            game.player.money,
                            game.player.equipment.clone()
                        ),
                        inventory
                    );
                    if !custom {
                        // Current campaign construction/traversal has its own
                        // proof. Do not choose a passing seed/alter geometry to
                        // force this controller-reset test's old corridor.
                        continue;
                    }
                    // characterCreateEvent sets the upright platform mask to
                    // (-5,-8,5,8). This corridor covers its six-tick held-jump
                    // sweep; select and assert geometry before exercising the
                    // controller, rather than selecting on movement outcomes.
                    let entrance = game.world.entrance;
                    assert!(
                        !game.world.collides_except(
                            Rect::new(entrance.x - 6., entrance.y - 40., 40., 48.),
                            0,
                        ),
                        "continued entrance headroom: {rules:?}/{character:?}/{custom}"
                    );
                    for offset in -5..=33 {
                        assert!(
                            game.world.tiles_point(
                                Vec2::new(entrance.x + f64::from(offset), entrance.y + 8.),
                                "oSolid",
                            ),
                            "continued entrance floor: {rules:?}/{character:?}/{custom}/{offset}"
                        );
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
        game.player.health = 2.0;
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
        assert_eq!(game.player.health, 3.0);
        assert!(!game.data.contains_key("pending_damsel_kisses"));
        game.continue_custom(&level, "third", "second");
        assert_eq!(game.player.health, 3.0);
    }

    #[test]
    fn lethal_contact_keeps_corpse_physics_and_actor_clocks_running() {
        let mut game = flat_game();
        game.player.health = 1.0;
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
                game.player.health = 0.0;
                game.player.state = PlayerState::Dead;
                game.state = RunState::Dead;
                game.player.equipment.insert("Ankh".into());
                game.data.insert("death_ticks".into(), 100.);
                game.step(&InputFrame::default());
                assert_eq!(game.state, RunState::Playing, "{room:?}/{character:?}");
                assert_eq!(
                    game.player.health,
                    if character == characters::Character::TunnelMan {
                        2.0
                    } else {
                        4.0
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
        assert!(!game.player.engine_active);
        assert!(
            !game
                .dispatcher
                .active_order
                .contains(&dispatcher::Instance::Player)
        );

        for _ in 0..120 {
            if game.world.name == "rCredits1" {
                break;
            }
            game.step(&InputFrame::default());
        }
        assert_eq!(
            game.world.name, "rCredits1",
            "natural title rope ascent must continue through the fade into credits"
        );
        assert_eq!(
            game.ending.as_ref().unwrap().stage,
            ending::EndingStage::Credits
        );
    }

    #[test]
    fn source_record_clock_uses_source_units_and_excludes_hubs_and_dead_ticks() {
        let mut game = flat_game();
        for _ in 0..34 {
            game.step(&InputFrame::default());
        }
        assert_eq!(game.source_elapsed_ms(), 1020);
        assert_eq!(game.source_elapsed_seconds(), 1);
        game.player.health = 0.0;
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
    fn source_transition_recomputes_no_dark_from_completed_room_seconds() {
        for (elapsed_ms, expected_no_dark) in [
            (20_999.0, true),
            (21_000.0, false),
            (60_000.0, true),
            (80_999.0, true),
            (81_000.0, false),
        ] {
            let mut game = Game::new(17, RulesPreset::Classic);
            game.data.insert("xtime".into(), elapsed_ms);

            game.next_level();

            assert_eq!(
                game.flags.contains("no_dark_level"),
                expected_no_dark,
                "completed room xtime={elapsed_ms}ms"
            );
            assert_eq!(game.data["xtime"], 0.0, "oLevel Create resets xtime");
        }
    }

    #[test]
    fn editor_playtest_returns_after_exit_animation_without_transition_or_bonus() {
        let mut game = flat_game();
        game.flags.insert("custom".into());
        game.flags.insert("custom_level".into());
        game.flags.insert("editor_test".into());
        game.flags.insert("damsel_rescued".into());
        game.flags.insert("unlock_tunnel_man".into());
        game.data.insert("pending_damsel_kisses".into(), 1.);
        game.data.insert("damsels".into(), 1.);
        game.data.insert("xdamsels".into(), 1.);
        game.player.health = 2.0;
        game.player.bombs = 1.0;
        game.player.ropes = 2.0;
        game.player.money = 125;
        game.player.equipment.insert("Jetpack".into());
        game.world.name = "editor fixture".into();
        let rng = game.rng.state_bits();
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
        assert_eq!(game.rng.state_bits(), rng);
        assert_eq!(
            (game.player.health, game.player.bombs, game.player.ropes),
            (4., 4., 4.)
        );
        assert_eq!(game.player.money, 0);
        assert!(game.player.equipment.is_empty());
        assert_eq!(game.data["pending_damsel_kisses"], 0.);
        assert_eq!(game.data["damsels"], 0.);
        assert_eq!(game.data["xdamsels"], 0.);
        assert!(!game.flags.contains("damsel_rescued"));
        assert!(game.flags.contains("editor_test"));
        assert!(game.flags.contains("unlock_tunnel_man"));
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

    #[test]
    fn authored_tutorial_exit_clears_run_at_transition_entry_then_starts_level_one() {
        for character in [
            characters::Character::Explorer,
            characters::Character::Damsel,
            characters::Character::TunnelMan,
        ] {
            let mut game = Game::shell(71, RulesPreset::Classic);
            characters::apply(&mut game, character);
            game.start_from_hub(true, 1);
            game.flags.extend([
                "first_time".into(),
                "unlock_sun".into(),
                "shortcut_5".into(),
            ]);
            game.data.insert("tunnel1_remaining".into(), 12345.);
            game.data.insert("best_mini1".into(), 47.);
            game.lifetime_stats.total_chests = 17;
            game.flags.insert("used_shortcut".into());
            game.flags.insert("low_graphics".into());
            game.player.health = 1.0;
            game.player.money = 1234;
            game.player.equipment.insert("Jetpack".into());
            game.data.insert("kills".into(), 3.);
            game.data.insert("kali_punish".into(), 2.);
            game.data.insert("pending_damsel_kisses".into(), 2.);
            // Controlled placement on the original ordinary exit's supported
            // floor; its authored door, terrain, and animation are untouched.
            game.player.pos = Vec2::new(56., 248.);
            game.player.previous_pos = game.player.pos;
            for _ in 0..5 {
                game.step(&InputFrame::default());
            }
            assert!(game.player.grounded);
            game.step(&InputFrame {
                up: true,
                ..Default::default()
            });
            assert_eq!(game.state, RunState::Ending);
            assert_eq!(
                game.ending.as_ref().unwrap().stage,
                ending::EndingStage::Exit
            );
            assert_eq!(game.player.money, 1234, "reset must wait for Animation End");
            assert!(game.flags.contains("tutorial"));
            for _ in 0..100 {
                game.step(&InputFrame::default());
                if game.world.name == "rTransition1" {
                    break;
                }
            }
            assert_eq!(game.world.name, "rTransition1");
            let transition = game.ending.as_ref().unwrap();
            assert_eq!(transition.stage, ending::EndingStage::Transition);
            assert_eq!(transition.completed_level, 0);
            assert_eq!(transition.source_time_ms, 0);
            assert_eq!(transition.level_time_ms, 0);
            assert_eq!(transition.level_money(), 0);
            assert_eq!(game.data["next_level"], 1.);
            assert_eq!(game.data["time"], 0.);
            assert_eq!(game.data["kills"], 0.);
            assert_eq!(game.player.money, 0);
            assert!(game.player.equipment.is_empty());
            let inventory = if character == characters::Character::TunnelMan {
                (2.0, 0.0, 0.0)
            } else {
                (4.0, 4.0, 4.0)
            };
            assert_eq!(
                (game.player.health, game.player.bombs, game.player.ropes),
                inventory
            );
            assert_eq!(characters::selected(&game), character);
            assert!(!game.flags.contains("tutorial"));
            assert!(!game.flags.contains("first_time"));
            assert!(game.flags.contains("onboarding_consumed"));
            assert!(
                game.flags.contains("used_shortcut"),
                "scrClearGlobals preserves this independent flag"
            );
            assert!(game.flags.contains("low_graphics"));
            assert!(game.flags.contains("unlock_sun"));
            assert!(game.flags.contains("shortcut_5"));
            assert_eq!(game.data["tunnel1_remaining"], 12345.);
            assert_eq!(game.data["best_mini1"], 47.);
            assert_eq!(game.lifetime_stats.total_chests, 17);
            assert!(
                !game
                    .world
                    .entities
                    .iter()
                    .any(|e| matches!(e.object.as_str(), "oDamselKiss" | "oBall2" | "oChain2"))
            );
            // Original Attack first hurries, then a new edge continues after
            // the source tally completes. No injected Ending state is used.
            for tick in 0..300 {
                game.step(&InputFrame {
                    attack_pressed: tick % 2 == 0,
                    ..Default::default()
                });
                if game.state == RunState::Transition {
                    break;
                }
            }
            assert_eq!(game.state, RunState::Transition);
            assert!(
                !game
                    .flags
                    .iter()
                    .any(|flag| flag.starts_with("hub_destination:"))
            );
            game.next_level();
            assert_eq!(game.world.level, 1);
            assert_eq!(game.state, RunState::Playing);
            assert!(!game.flags.contains("hub"));
            assert_eq!(
                (game.player.health, game.player.bombs, game.player.ropes),
                inventory
            );
        }
    }

    #[test]
    fn authored_tutorial_manual_return_door_still_selects_title() {
        let mut game = Game::tutorial(72, RulesPreset::Classic);
        for _ in 0..5 {
            game.step(&InputFrame::default());
        }
        assert_eq!(game.player.pos, Vec2::new(24., 72.));
        assert!(game.player.grounded);
        game.step(&InputFrame {
            up: true,
            ..Default::default()
        });
        for _ in 0..100 {
            game.step(&InputFrame::default());
            if game.state == RunState::Transition {
                break;
            }
        }
        assert!(game.flags.contains("hub_destination:title"));
        assert!(game.flags.contains("title_start:3"));
        assert_ne!(game.world.name, "rTransition1");
        game.load_hub(hub::HubKind::Title);
        assert_eq!(game.world.name, "rTitle");
        assert_eq!(game.player.pos, Vec2::new(344., 184.));
    }

    #[test]
    fn hub_started_moai_suppresses_a_second_moai_at_level_ten() {
        // scrLevelGen.gml:214-221 sets madeMoai during level generation;
        // GameMaker shortcuts can enter level 9 directly from the hub.
        let mut game = Game::shell(11, RulesPreset::Classic);
        game.start_from_hub(false, 9);
        assert_eq!(game.world.level, 9);
        assert!(
            game.world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oMoai")
        );
        assert!(game.flags.contains("made_moai"));

        game.data.insert("next_level".into(), 10.0);
        game.next_level();
        assert_eq!(game.world.level, 10);
        assert!(
            !game
                .world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oMoai")
        );
    }

    #[test]
    fn forced_moai_request_is_consumed_after_its_room_generation() {
        let mut game = Game::shell(0, RulesPreset::Classic);
        game.data.insert("next_level".into(), 9.0);
        game.flags.insert("moai".into());

        game.next_level();

        assert_eq!(game.world.level, 9);
        assert!(
            game.world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oMoai")
        );
        assert!(game.flags.contains("made_moai"));
        assert!(!game.flags.contains("moai"));

        game.data.insert("next_level".into(), 10.0);
        game.next_level();

        assert_eq!(game.world.level, 10);
        assert!(
            !game
                .world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oMoai")
        );
    }

    #[test]
    fn city_of_gold_transition_preserves_selected_sacrifice_alarm_chain() {
        let mut game = Game::new(3, RulesPreset::Classic);
        game.world.level = 14;
        game.flags.insert("city_of_gold".into());
        // This is the outgoing room's per-room value. next_level must clear it
        // before generating the target, then publish the target's fresh roll.
        game.flags.insert("sacrificePit".into());

        game.next_level();

        assert_eq!(game.world.level, 15);
        assert_eq!(game.world.name, "City of Gold");
        assert_eq!(
            game.world.source_room_path,
            Some([
                [2, 1, 0, 7],
                [2, 0, 0, 8],
                [3, 1, 2, 6],
                [0, 0, 3, 9],
                [0, 0, 0, 0],
            ])
        );
        assert!(game.flags.contains("sacrifice_pit"));
        assert!(!game.flags.contains("sacrificePit"));
        assert_eq!(game.player.alarms[0], 10);

        player::alarm(&mut game, 0);
        assert_eq!(game.player.alarms[1], 210);
        player::alarm(&mut game, 1);
        assert_eq!(game.player.alarms[4], 210);
    }
}

#[cfg(test)]
mod source_room_entry_tests {
    use super::*;

    fn custom_room_with_wanted_exit(wanted: &str, linked: bool) -> Game {
        let mut game = Game::shell(1, RulesPreset::Classic);
        match wanted {
            "murderer" => {
                game.flags.insert("murderer".into());
            }
            "thief" => {
                game.data.insert("thiefLevel".into(), 1.0);
            }
            _ => unreachable!(),
        }

        let mut level = level::Level::default();
        level.set_tile(10, 8, 'X').unwrap();
        level.set_tile(30, 8, 'X').unwrap();
        if linked {
            game.load_custom(&level::Level::default(), "first", "");
            game.continue_custom(&level, "second", "first");
        } else {
            game.load_custom(&level, "wanted-exits", "");
        }
        game
    }

    #[test]
    fn linked_custom_room_clears_only_scrlevelgen_layout_flags_before_path_setup() {
        let mut fixture = None;
        for seed in 0..256 {
            let mut first = Game::shell(seed, RulesPreset::Classic);
            first.flags.extend(
                ["murderer", "lake", "genBlackMarket", "made_black_market"]
                    .into_iter()
                    .map(str::to_owned),
            );
            first.load_custom(&level::Level::default(), "first", "");
            // Use an actually selected prior-room flag so this exercises the
            // linked carryover that scrLevelGen resets on every invocation.
            if !first.flags.contains("snake_pit") {
                continue;
            }

            let mut expected = first.clone();
            for flag in SOURCE_CUSTOM_ROOM_LAYOUT_FLAGS {
                expected.flags.remove(flag);
            }
            expected.continue_custom(&level::Level::default(), "second", "first");
            // Avoid a seed where the next room independently rolls a new
            // special layout, so the comparison isolates the stale flag.
            if SOURCE_CUSTOM_ROOM_LAYOUT_FLAGS
                .iter()
                .any(|flag| expected.flags.contains(*flag))
            {
                continue;
            }
            fixture = Some((first, expected.world.source_room_path));
            break;
        }
        let (mut linked, expected_path) = fixture.expect("find linked source-path fixture");
        for flag in SOURCE_CUSTOM_ROOM_LAYOUT_FLAGS {
            linked.flags.insert(flag.into());
        }

        linked.continue_custom(&level::Level::default(), "second", "first");

        assert_eq!(linked.world.source_room_path, expected_path);
        assert!(
            SOURCE_CUSTOM_ROOM_LAYOUT_FLAGS
                .iter()
                .all(|flag| !linked.flags.contains(*flag))
        );
        for persistent in ["murderer", "lake", "genBlackMarket", "made_black_market"] {
            assert!(linked.flags.contains(persistent), "lost {persistent}");
        }
    }

    #[test]
    fn wanted_exit_keepers_exist_before_first_custom_room_draw_and_do_not_duplicate() {
        for wanted in ["murderer", "thief"] {
            for linked in [false, true] {
                let mut game = custom_room_with_wanted_exit(wanted, linked);
                let mut guards: Vec<_> = game
                    .world
                    .entities
                    .iter()
                    .filter(|entity| entity.alive && entity.object == "oShopkeeper")
                    .collect();
                guards.sort_by_key(|entity| entity.creation_serial);
                assert_eq!(
                    guards.iter().map(|entity| entity.pos).collect::<Vec<_>>(),
                    [Vec2::new(176.0, 144.0), Vec2::new(496.0, 144.0)],
                    "{wanted}, linked={linked}"
                );
                assert!(guards.iter().all(|guard| {
                    guard.state == 4 && guard.value("enemy_context_created") == 1.0
                }));

                game.step(&InputFrame::default());
                assert_eq!(
                    game.world
                        .entities
                        .iter()
                        .filter(|entity| entity.alive && entity.object == "oShopkeeper")
                        .count(),
                    2,
                    "first dispatcher tick must not repeat room Create"
                );
            }
        }
    }

    #[test]
    fn wanted_campaign_exit_keepers_exist_after_next_level_before_first_step() {
        for wanted in ["murderer", "thief"] {
            let mut game = Game::shell(8, RulesPreset::Classic);
            if wanted == "murderer" {
                game.flags.insert("murderer".into());
            } else {
                game.data.insert("thiefLevel".into(), 1.0);
            }
            game.data.insert("next_level".into(), 2.0);

            game.next_level();

            let mut exit_positions: Vec<_> = game
                .world
                .entities
                .iter()
                .filter(|entity| {
                    entity.alive
                        && crate::catalog::is_a(&entity.object, "oExit")
                        && entity
                            .text
                            .get("type")
                            .map_or(entity.object != "oXMarket", |kind| kind == "Exit")
                })
                .map(|entity| entity.pos)
                .collect();
            exit_positions.sort_by(|a, b| a.x.total_cmp(&b.x).then(a.y.total_cmp(&b.y)));
            assert!(!exit_positions.is_empty(), "generated room has an exit");

            let mut guards: Vec<_> = game
                .world
                .entities
                .iter()
                .filter(|entity| entity.alive && entity.object == "oShopkeeper")
                .collect();
            guards.sort_by(|a, b| {
                a.pos
                    .x
                    .total_cmp(&b.pos.x)
                    .then(a.pos.y.total_cmp(&b.pos.y))
            });
            assert_eq!(
                guards.iter().map(|guard| guard.pos).collect::<Vec<_>>(),
                exit_positions,
                "{wanted} guard placement before the first simulation step"
            );
            assert!(
                guards.iter().all(|guard| {
                    guard.state == 4 && guard.value("enemy_context_created") == 1.0
                })
            );

            game.step(&InputFrame::default());
            assert_eq!(
                game.world
                    .entities
                    .iter()
                    .filter(|entity| entity.alive && entity.object == "oShopkeeper")
                    .count(),
                exit_positions.len(),
                "first dispatcher tick must not repeat room Create"
            );
        }
    }

    #[test]
    fn historical_custom_entry_keeps_legacy_first_step_progression_timing() {
        let mut game = Game::legacy_shell(1, RulesPreset::Classic);
        game.flags.insert("murderer".into());
        let mut level = level::Level::default();
        level.set_tile(10, 8, 'X').unwrap();

        game.load_custom(&level, "historical", "");

        assert!(!game.data.contains_key("progression_initialized"));
        game.step(&InputFrame::default());
        assert!(game.data.contains_key("progression_initialized"));
    }
}

// Append this separate test module to crates/core/src/game.rs. Prepared without
// compilation or execution; these tests exercise version/transport contracts.
#[cfg(test)]
mod jungle_version_contract_tests {
    use crate::{
        Game, GenerationRevision, HISTORICAL_SIMULATION_VERSION, InputFrame, RngAlgorithm,
        RulesPreset, SIMULATION_VERSION, generation, replay,
    };
    use std::panic::{AssertUnwindSafe, catch_unwind};

    #[test]
    fn jungle_contract_missing_revision_keeps_canonical_snapshot_historical() {
        let shell = Game::shell(42, RulesPreset::Classic);
        let mut old_snapshot = serde_json::to_value(&shell).unwrap();
        old_snapshot
            .as_object_mut()
            .unwrap()
            .remove("generation_revision");
        let mut restored: Game = serde_json::from_value(old_snapshot.clone()).unwrap();
        assert_eq!(restored.rng.algorithm(), RngAlgorithm::Gm8Lcg32V1);
        assert_eq!(
            restored.generation_revision,
            GenerationRevision::HistoricalV1
        );
        assert_eq!(restored.simulation_version(), HISTORICAL_SIMULATION_VERSION);
        assert_eq!(serde_json::to_value(&restored).unwrap(), old_snapshot);

        // A historical canonical snapshot must not begin reading the new
        // campaign projection merely because its RNG is canonical.
        restored.data.insert("marketChance".into(), -9_999_999.0);
        restored.generate_campaign_room(1);
        assert_eq!(restored.data["marketChance"], -9_999_999.0);
        assert_eq!(
            restored.generation_revision,
            GenerationRevision::HistoricalV1
        );
        assert!(
            serde_json::to_value(&restored)
                .unwrap()
                .get("generation_revision")
                .is_none()
        );
    }

    #[test]
    fn jungle_contract_fresh_canonical_revision_survives_serialization() {
        let fresh = Game::new(42, RulesPreset::Classic);
        assert_eq!(
            fresh.generation_revision,
            GenerationRevision::SourceClassicV2
        );
        assert_eq!(fresh.simulation_version(), SIMULATION_VERSION);
        assert_eq!(fresh.data["marketChance"], 160.0);
        let json = serde_json::to_value(&fresh).unwrap();
        assert_eq!(json["generation_revision"], "SourceClassicV2");
        let restored: Game = serde_json::from_value(json.clone()).unwrap();
        assert_eq!(
            restored.generation_revision,
            GenerationRevision::SourceClassicV2
        );
        assert_eq!(serde_json::to_value(&restored).unwrap(), json);
    }

    #[test]
    fn jungle_contract_historical_constructor_selects_revision_before_generation() {
        for algorithm in [RngAlgorithm::LegacySplitMix64V1, RngAlgorithm::Gm8Lcg32V1] {
            let historical =
                Game::new_historical_with_algorithm(42, RulesPreset::Classic, algorithm).unwrap();
            assert_eq!(
                historical.generation_revision,
                GenerationRevision::HistoricalV1
            );
            assert_eq!(historical.rng.algorithm(), algorithm);
            assert_eq!(
                historical.simulation_version(),
                HISTORICAL_SIMULATION_VERSION
            );
            assert!(!historical.data.contains_key("marketChance"));
            assert!(
                serde_json::to_value(&historical)
                    .unwrap()
                    .get("generation_revision")
                    .is_none()
            );
        }
        for seed in [0, u64::MAX] {
            let legacy = Game::legacy(seed, RulesPreset::Classic);
            let explicit = Game::new_historical_with_algorithm(
                seed,
                RulesPreset::Classic,
                RngAlgorithm::LegacySplitMix64V1,
            )
            .unwrap();
            assert_eq!(
                serde_json::to_value(legacy).unwrap(),
                serde_json::to_value(explicit).unwrap()
            );
        }
        assert!(
            Game::new_historical_with_algorithm(
                u64::from(u32::MAX) + 1,
                RulesPreset::Classic,
                RngAlgorithm::Gm8Lcg32V1,
            )
            .is_err()
        );
    }

    #[test]
    fn jungle_contract_versioned_replays_route_all_three_supported_pairs() {
        let frames = [
            InputFrame::default(),
            InputFrame {
                right: true,
                ..InputFrame::default()
            },
            InputFrame {
                jump: true,
                jump_pressed: true,
                ..InputFrame::default()
            },
        ];
        for (algorithm, version) in [
            (
                RngAlgorithm::LegacySplitMix64V1,
                HISTORICAL_SIMULATION_VERSION,
            ),
            (RngAlgorithm::Gm8Lcg32V1, HISTORICAL_SIMULATION_VERSION),
            (RngAlgorithm::Gm8Lcg32V1, SIMULATION_VERSION),
        ] {
            let mut manual = if version == HISTORICAL_SIMULATION_VERSION {
                Game::new_historical_with_algorithm(42, RulesPreset::Classic, algorithm).unwrap()
            } else {
                Game::new_with_algorithm(42, RulesPreset::Classic, algorithm).unwrap()
            };
            for frame in &frames {
                manual.step(frame);
            }
            let result = replay::from_frames_with_simulation_version(
                42,
                RulesPreset::Classic,
                &frames,
                algorithm,
                version,
            )
            .unwrap();
            assert_eq!(result.simulation_version, version);
            assert_eq!(result.algorithm, algorithm);
            assert_eq!(result.hash, format!("{:016x}", manual.state_hash()));
            assert_eq!(result.ticks, manual.tick);
        }
        let historical = replay::from_frames_with_simulation_version(
            42,
            RulesPreset::Classic,
            &frames,
            RngAlgorithm::Gm8Lcg32V1,
            HISTORICAL_SIMULATION_VERSION,
        )
        .unwrap();
        let fresh = replay::from_frames(42, RulesPreset::Classic, &frames).unwrap();
        assert_eq!(fresh.simulation_version, SIMULATION_VERSION);
        assert_ne!(
            historical.hash, fresh.hash,
            "the serialized contracts are distinct"
        );
        let existing_legacy_api = replay::from_frames_with_algorithm(
            u64::MAX,
            RulesPreset::Classic,
            &[],
            RngAlgorithm::LegacySplitMix64V1,
        )
        .unwrap();
        assert_eq!(
            existing_legacy_api.simulation_version,
            HISTORICAL_SIMULATION_VERSION
        );
    }

    #[test]
    fn jungle_contract_unsupported_replay_pair_rejected_before_constructor_validation() {
        for version in [0, SIMULATION_VERSION, u32::MAX] {
            assert!(
                replay::validate_simulation_version(RngAlgorithm::LegacySplitMix64V1, version)
                    .is_err()
            );
            assert!(
                replay::from_frames_with_simulation_version(
                    42,
                    RulesPreset::Classic,
                    &[],
                    RngAlgorithm::LegacySplitMix64V1,
                    version,
                )
                .unwrap_err()
                .contains("Unsupported replay simulation version")
            );
        }
        // This seed would fail canonical construction. Contract validation
        // must win first, rather than constructing any procedural game.
        let error = replay::from_frames_with_simulation_version(
            u64::MAX,
            RulesPreset::Classic,
            &[],
            RngAlgorithm::Gm8Lcg32V1,
            u32::MAX,
        )
        .unwrap_err();
        assert!(error.contains("Unsupported replay simulation version"));
    }

    #[test]
    fn jungle_contract_market_state_carries_between_rooms_and_resets_for_new_run() {
        let mut game = Game::shell(42, RulesPreset::Classic);
        game.data.insert("marketChance".into(), -37.0);
        game.flags.insert("madeMarketEntrance".into());
        game.generate_campaign_room(1);
        assert_eq!(game.data["marketChance"], -37.0);
        assert!(game.flags.contains("made_market_entrance"));
        game.data.insert("next_level".into(), 2.0);
        game.next_level();
        assert_eq!(game.world.level, 2);
        assert_eq!(game.data["marketChance"], -37.0);
        assert!(game.flags.contains("made_market_entrance"));

        let before_clear_rng = game.rng.state_bits();
        game.clear_run_globals();
        assert_eq!(game.data["marketChance"], 160.0);
        assert!(!game.flags.contains("made_market_entrance"));
        assert_eq!(game.rng.state_bits(), before_clear_rng);

        game.data.insert("marketChance".into(), -37.0);
        game.flags.insert("made_market_entrance".into());
        game.start_from_hub(false, 1);
        assert_eq!(game.data["marketChance"], 160.0);
        assert!(!game.flags.contains("made_market_entrance"));
        assert_eq!(
            game.generation_revision,
            GenerationRevision::SourceClassicV2
        );
    }

    #[test]
    fn jungle_contract_invalid_market_projection_rejected_before_procedural_rng() {
        for invalid in [
            f64::NAN,
            f64::INFINITY,
            1.5,
            f64::from(generation::MIN_SUPPORTED_MARKET_CHANCE) - 1.0,
            f64::from(1 << 21) + 1.0,
        ] {
            let mut game = Game::shell(42, RulesPreset::Classic);
            game.data.insert("marketChance".into(), invalid);
            let before_rng = game.rng.state_bits();
            let before_world = serde_json::to_value(&game.world).unwrap();
            assert!(catch_unwind(AssertUnwindSafe(|| game.generate_campaign_room(5))).is_err());
            assert_eq!(game.rng.state_bits(), before_rng);
            assert_eq!(serde_json::to_value(&game.world).unwrap(), before_world);
        }
    }
}

#[cfg(test)]
#[path = "jungle_public_contract_tests.rs"]
mod jungle_public_contract_tests;

#[cfg(test)]
#[path = "selectors_public_tests.rs"]
mod selectors_public_tests;

// Exercise admitted action frames while the Player callback is skipped.
#[cfg(test)]
mod admitted_action_frame_controls {
    use super::*;
    #[test]
    fn held_only_and_independent_edges_normalize_against_every_admitted_frame() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            let mut game = Game::shell(7, rules);
            // Transition/Won cannot run the ordinary Player controller, but
            // Game.step is still an admitted input-frame boundary.
            game.state = RunState::Transition;
            game.step(&InputFrame {
                attack: true,
                up: true,
                ..Default::default()
            });
            assert_eq!(game.data["input_frame_attack"], 1.);
            assert_eq!(game.data["input_frame_up"], 1.);
            game.step(&InputFrame {
                up: true,
                attack_released: true,
                ..Default::default()
            });
            assert!(!game.data.contains_key("input_frame_attack"));
            // The release was already consumed; first later receiver must not
            // infer another one from a stale Player-local controller cache.
            let ordinary = game.normalize_action_edges(&InputFrame {
                up: true,
                ..Default::default()
            });
            assert!(!ordinary.attack_released && !ordinary.up_pressed);
            let independent = game.normalize_action_edges(&InputFrame {
                up: true,
                up_pressed: true,
                attack: true,
                attack_released: true,
                down_pressed: true,
                ..Default::default()
            });
            assert!(
                independent.up_pressed && independent.down_pressed && independent.attack_released
            );
            assert_eq!(game.normalize_action_edges(&independent), independent);
            let falling = game.normalize_action_edges(&InputFrame::default());
            assert!(falling.attack_released);
            assert!(
                !game
                    .normalize_action_edges(&InputFrame::default())
                    .attack_released
            );
        }
    }
    #[test]
    fn discarded_host_controls_clear_action_history_without_adding_jump_edges() {
        let mut game = Game::shell(7, RulesPreset::Classic);
        let first = game.normalize_action_edges(&InputFrame {
            attack: true,
            up: true,
            down: true,
            jump: true,
            ..Default::default()
        });
        assert!(first.up_pressed && first.down_pressed);
        assert!(!first.jump_pressed);
        game.clear_direction_input_history();
        let reset = game.normalize_action_edges(&InputFrame::default());
        assert!(!reset.attack_released && !reset.up_pressed && !reset.down_pressed);
        assert!(!reset.jump_pressed && !reset.jump_released);
        assert!(
            game.normalize_action_edges(&InputFrame {
                up: true,
                down: true,
                ..Default::default()
            })
            .up_pressed
        );
    }
}

#[cfg(test)]
mod public_action_entry_compatibility {
    use super::*;
    #[test]
    fn public_player_tick_updates_action_history_before_its_inactive_guard() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            let mut game = Game::shell(7, rules);
            game.world = World::new(20, 20, Area::Mines, 1);
            game.player.pos = Vec2::new(88., 152.);
            for x in 0..20 {
                game.world.set_tile(x, 10, "oBlock");
            }
            game.player.data.insert("active".into(), 0.);
            let held = InputFrame {
                attack: true,
                up: true,
                ..Default::default()
            };
            crate::player::tick(&mut game, &held);
            assert_eq!(game.data["input_frame_attack"], 1.);
            assert_eq!(game.data["input_frame_up"], 1.);
            crate::player::tick(
                &mut game,
                &InputFrame {
                    up: true,
                    ..Default::default()
                },
            );
            assert!(!game.data.contains_key("input_frame_attack"));
            let next = game.normalize_action_edges(&InputFrame {
                up: true,
                ..Default::default()
            });
            assert!(!next.attack_released && !next.up_pressed);
            // Direct controller normalizes only its own input boundary; no
            // implicit extra item event is added to this public Player API.
            assert_eq!(game.player.data["active"], 0.);
        }
    }
    #[test]
    fn game_step_retains_held_only_bow_release_without_extra_item_events() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            let mut game = Game::shell(7, rules);
            game.world = World::new(20, 20, Area::Mines, 1);
            for x in 0..20 {
                game.world.set_tile(x, 10, "oBlock");
            }
            game.player.pos = Vec2::new(88., 152.);
            game.player.previous_pos = game.player.pos;
            game.player.state = PlayerState::Standing;
            game.player.grounded = true;
            game.data.insert("arrows".into(), 3.);
            let bow = game.world.spawn("oBow", game.player.pos);
            game.player.held = Some(bow);
            game.world.entity_mut(bow).unwrap().held = true;
            // Ordinary full dispatcher frames arm/charge the initial held Bow.
            game.step(&InputFrame {
                attack: true,
                attack_pressed: true,
                ..Default::default()
            });
            assert_eq!(game.player.data["bow_armed"], 1.);
            let ammo = game.data["arrows"];
            game.step(&InputFrame {
                attack: true,
                ..Default::default()
            });
            assert_eq!(game.data["arrows"], ammo);
            game.step(&InputFrame::default());
            assert_eq!(game.data["arrows"], ammo - 1.);
            assert_eq!(game.player.data["bow_armed"], 0.);
            game.step(&InputFrame::default());
            assert_eq!(game.data["arrows"], ammo - 1.);
        }
    }
}
