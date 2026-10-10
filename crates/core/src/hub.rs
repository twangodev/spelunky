//! Authored Classic title, records, and challenge rooms.
//! Room instances come from the original XML; source controllers use 30 Hz alarms.
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HubKind {
    Title,
    Scores,
    Sun,
    Moon,
    Stars,
}
impl HubKind {
    pub fn key(self) -> &'static str {
        match self {
            Self::Title => "title",
            Self::Scores => "scores",
            Self::Sun => "sun",
            Self::Moon => "moon",
            Self::Stars => "stars",
        }
    }
    fn room(self) -> &'static str {
        match self {
            Self::Title => "rTitle",
            Self::Scores => "rHighscores",
            Self::Sun => "rSun",
            Self::Moon => "rMoon",
            Self::Stars => "rStars",
        }
    }
    pub fn challenge(self) -> bool {
        matches!(self, Self::Sun | Self::Moon | Self::Stars)
    }
}
#[derive(Deserialize)]
struct Instance {
    object: String,
    x: f64,
    y: f64,
}
#[derive(Deserialize)]
struct Room {
    width: usize,
    height: usize,
    instances: Vec<Instance>,
}
#[derive(Deserialize)]
struct Rooms {
    rooms: BTreeMap<String, Room>,
}
fn rooms() -> &'static Rooms {
    static ROOMS: OnceLock<Rooms> = OnceLock::new();
    ROOMS.get_or_init(|| {
        serde_json::from_str(include_str!("../../../assets/hub.json"))
            .expect("authored hub room data")
    })
}

/// Build authored geometry without consuming gameplay randomness.
pub fn world(kind: HubKind, unlocks: &BTreeSet<String>) -> World {
    let room = &rooms().rooms[kind.room()];
    let mut world = World::new(
        room.width.div_ceil(16),
        room.height.div_ceil(16),
        Area::Mines,
        0,
    );
    world.name = kind.room().into();
    world.entrance = Vec2::new(600.0, 184.0);
    for instance in &room.instances {
        let pos = Vec2::new(instance.x, instance.y);
        if instance.object == "oPlayer1" {
            world.entrance = pos;
            continue;
        }
        add_instance(&mut world, &instance.object, pos, kind.challenge());
    }
    if kind == HubKind::Title {
        // oTitle Create: initial completed-intro spawn and Tunnel Man shortcuts.
        world.spawn("oFlare", Vec2::new(600.0, 188.0));
        // This port enters the completed-intro spawn (source titleStart=2).
        // Fresh storage alone enables the source onboarding hand and barriers.
        if unlocks.contains("first_time") {
            let title_start = unlocks
                .iter()
                .find_map(|flag| {
                    flag.strip_prefix("title_start:")
                        .and_then(|value| value.parse::<u8>().ok())
                })
                .unwrap_or(2);
            if matches!(title_start, 1..=3) {
                let hand = world.spawn("oHintHand", Vec2::new(384.0, 144.0));
                world
                    .entity_mut(hand)
                    .unwrap()
                    .data
                    .insert("animation_speed".into(), 0.5);
            }
            for (x, y) in [(320.0, 160.0), (336.0, 160.0), (336.0, 176.0)] {
                add_instance(&mut world, "oBrick", Vec2::new(x, y), false);
            }
        }
        // Source tunnel1/tunnel2 are remaining donation balances, not stage
        // booleans. Stage two resets tunnel1 for stage three. Monotonic unlock
        // flags therefore map to ladder, level-nine door, level-thirteen door.
        let [stage1, stage2, stage3] = shortcut_stages(unlocks);
        if stage1 {
            for (object, y) in [
                ("oLadderOrange", 96.0),
                ("oLadderTop", 112.0),
                ("oLadderOrange", 128.0),
                ("oLadderOrange", 144.0),
                ("oLadderOrange", 160.0),
                ("oLadderOrange", 176.0),
            ] {
                add_instance(&mut world, object, Vec2::new(352.0, y), false);
            }
            if stage2 {
                world.spawn("oLevel9Sign", Vec2::new(128.0, 112.0));
                world.spawn("oXShortcut9", Vec2::new(144.0, 128.0));
                if stage3 {
                    world.spawn("oLevel13Sign", Vec2::new(192.0, 112.0));
                    world.spawn("oXShortcut13", Vec2::new(208.0, 128.0));
                }
            }
        } else {
            for x in [320.0, 336.0] {
                add_instance(&mut world, "oBrick", Vec2::new(x, 96.0), false);
            }
        }
        if unlocks.contains("unlock_tunnel_man") || all_gold(unlocks) {
            world.spawn("oMultiTrophy", Vec2::new(32.0, 112.0));
            world.spawn("oXChange2", Vec2::new(32.0, 128.0));
        } else {
            let id = world.spawn("oTunnelMan", Vec2::new(40.0, 136.0));
            world.entity_mut(id).unwrap().sprite = "sTunnelManRight".into();
        }
    }
    if kind == HubKind::Scores {
        for (category, flag, door, x, y) in [
            ("money", "unlock_sun", "oXSun", 32.0, 48.0),
            ("time", "unlock_moon", "oXMoon", 64.0, 80.0),
            ("kills", "unlock_stars", "oXStars", 32.0, 112.0),
            ("saves", "unlock_damsel", "oXChange", 64.0, 144.0),
        ] {
            let rank = if unlocks.contains(flag) {
                3
            } else {
                (1..=3)
                    .rev()
                    .find(|rank| unlocks.contains(&format!("trophy_{category}_{rank}")))
                    .unwrap_or(0)
            };
            if rank > 0 {
                let id = world.spawn("oTrophy", Vec2::new(x, y - 16.0));
                world.entity_mut(id).unwrap().sprite = match rank {
                    3 => "sGoldTrophy",
                    2 => "sSilverTrophy",
                    _ => "sBronzeTrophy",
                }
                .into();
            }
            if unlocks.contains(flag) {
                world.spawn(door, Vec2::new(x, y));
            }
        }
    }
    setup_wall_sprites(&mut world);
    world
}
fn shortcut_stages(flags: &BTreeSet<String>) -> [bool; 3] {
    let stage3 = flags.contains("shortcut_13") || flags.contains("tunnel3_complete");
    let stage2 = stage3 || flags.contains("shortcut_9") || flags.contains("tunnel2_complete");
    let stage1 = stage2 || flags.contains("shortcut_5") || flags.contains("tunnel1_complete");
    [stage1, stage2, stage3]
}

/// Convert independent best run records to the source trophy and door gates.
/// `seconds` must come from a completed winning run; absent/zero is not a medal.
pub fn record_unlocks(
    money: u32,
    seconds: Option<f64>,
    kills: u32,
    saves: u32,
) -> BTreeSet<String> {
    let mut flags = BTreeSet::new();
    let time = seconds.filter(|t| t.is_finite() && *t > 0.0);
    for (category, rank) in [
        (
            "money",
            if money >= 200000 {
                3
            } else if money >= 100000 {
                2
            } else if money >= 50000 {
                1
            } else {
                0
            },
        ),
        (
            "time",
            if time.is_some_and(|t| t <= 600.0) {
                3
            } else if time.is_some_and(|t| t <= 720.0) {
                2
            } else if time.is_some_and(|t| t < 960.0) {
                1
            } else {
                0
            },
        ),
        (
            "kills",
            if kills >= 120 {
                3
            } else if kills >= 100 {
                2
            } else if kills >= 80 {
                1
            } else {
                0
            },
        ),
        (
            "saves",
            if saves >= 8 {
                3
            } else if saves >= 6 {
                2
            } else if saves >= 4 {
                1
            } else {
                0
            },
        ),
    ] {
        if rank > 0 {
            flags.insert(format!("trophy_{category}_{rank}"));
        }
    }
    for (gate, condition) in [
        ("unlock_sun", money >= 200000),
        ("unlock_moon", time.is_some_and(|t| t <= 600.0)),
        ("unlock_stars", kills >= 120),
        ("unlock_damsel", saves >= 8),
    ] {
        if condition {
            flags.insert(gate.into());
        }
    }
    if all_gold(&flags) {
        flags.insert("unlock_tunnel_man".into());
    }
    flags
}
fn all_gold(flags: &BTreeSet<String>) -> bool {
    ["unlock_sun", "unlock_moon", "unlock_stars", "unlock_damsel"]
        .iter()
        .all(|key| flags.contains(*key))
}
pub(crate) fn add_instance(world: &mut World, object: &str, pos: Vec2, challenge: bool) {
    let static_tile = (crate::catalog::is_solid(object)
        && !crate::catalog::is_a(object, "oMovingSolid")
        && !crate::catalog::is_a(object, "oMoveableSolid"))
        || crate::catalog::is_a(object, "oLadder");
    if static_tile && pos.x >= 0.0 && pos.y >= 0.0 && pos.x % 16.0 == 0.0 && pos.y % 16.0 == 0.0 {
        let x = pos.x as usize / 16;
        let y = pos.y as usize / 16;
        // Some authored rooms layer rope and masonry at the same origin.
        if world.tile(x as i32, y as i32).is_some_and(|t| t.is_solid())
            && !crate::catalog::is_solid(object)
        {
            world.spawn(object, pos);
            return;
        }
        world.set_tile(x, y, object);
        if crate::catalog::is_solid(object) {
            let id = world.spawn("oTerrainMetadata", Vec2::new(pos.x + 8.0, pos.y + 8.0));
            let meta = world.entity_mut(id).unwrap();
            if matches!(object, "oHardBlock" | "oLavaSolid")
                || challenge
                    && object == "oBrick"
                    && (pos.x <= 16.0 || pos.x >= 288.0 || pos.y <= 16.0 || pos.y >= 208.0)
            {
                meta.data.insert("invincible".into(), 1.0);
            }
        }
    } else {
        world.spawn(object, pos);
    }
}
pub(crate) fn setup_wall_sprites(world: &mut World) {
    let overrides: Vec<_> = world
        .tiles
        .iter()
        .enumerate()
        .filter(|(_, t)| t.object == "oBrick")
        .map(|(i, _)| {
            let x = (i % world.width) as i32;
            let y = (i / world.width) as i32;
            let brick = |dy| {
                world
                    .tile(x, y + dy)
                    .is_some_and(|t| matches!(t.object.as_str(), "oBrick" | "oHardBlock"))
            };
            let up = y == 0 || brick(-1);
            let down = y * 16 >= 224 || brick(1);
            (
                Vec2::new(x as f64 * 16.0 + 8.0, y as f64 * 16.0 + 8.0),
                match (up, down) {
                    (false, false) => "sCaveUp2",
                    (false, true) => "sCaveUp",
                    (true, false) => "sBrickDown",
                    _ => "sBrick",
                },
            )
        })
        .collect();
    for (pos, sprite) in overrides {
        if let Some(meta) = world
            .entities
            .iter_mut()
            .find(|e| e.object == "oTerrainMetadata" && e.pos == pos)
        {
            meta.text.insert("terrain_sprite".into(), sprite.into());
        }
    }
}

pub(crate) fn initialize_brick_variants(game: &mut Game) {
    let ids: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| {
            e.object == "oTerrainMetadata"
                && game
                    .world
                    .tile((e.pos.x / 16.0) as i32, (e.pos.y / 16.0) as i32)
                    .is_some_and(|t| t.object == "oBrick")
        })
        .map(|e| e.id)
        .collect();
    for id in ids {
        let chipped = game.rng.range(10) == 0;
        let gold = game.rng.range(100) + 1;
        let metadata = game.world.entity_mut(id).unwrap();
        // scrSetupWalls overwrites the sprite of exposed edges, removing gold.
        if metadata
            .text
            .get("terrain_sprite")
            .is_some_and(|s| s != "sBrick")
        {
            continue;
        }
        let (sprite, grade) = if gold < 20 {
            ("sBrickGold", 1)
        } else if gold < 30 {
            ("sBrickGoldBig", 2)
        } else if chipped {
            ("sBrick2", 0)
        } else {
            ("sBrick", 0)
        };
        metadata.text.insert("terrain_sprite".into(), sprite.into());
        if grade > 0 {
            metadata.data.insert("gold_chunks".into(), 3.0);
            metadata
                .data
                .insert("gold_nuggets".into(), if grade == 2 { 1.0 } else { 0.0 });
        }
        // isLevel() is false in every hub and challenge room; no embedded gems
        // or underground items are generated by these original Create events.
    }
}

/// Call after assigning/resetting the authored world. Preserves persistent unlocks.
pub fn initialize(game: &mut Game, kind: HubKind) {
    game.flags.retain(|f| !f.starts_with("hub_") && f != "hub");
    game.flags.insert("hub".into());
    game.flags.insert(format!("hub_{}", kind.key()));
    game.data.retain(|key, _| {
        !key.starts_with("hub_")
            && !matches!(key.as_str(), "points" | "baskets" | "timer" | "stars_kills")
    });
    // Death returns clear run inventory in scrClearGlobals. load_hub resets
    // RunState before this hook, so health retains the evidence of that death.
    let reset_tutorial_run = kind == HubKind::Title && game.flags.remove("reset_run_on_title");
    let reset_dead_run = game.flags.remove("reset_run_on_hub");
    if game.player.health <= 0 || reset_tutorial_run || reset_dead_run {
        let selected = crate::characters::selected(game);
        crate::characters::apply(game, selected);
        game.player.money = 0;
        game.player.equipment.clear();
        game.flags.insert("no_dark_level".into());
        game.flags.retain(|flag| {
            !matches!(
                flag.as_str(),
                "wanted"
                    | "murderer"
                    | "shopkeepers_angry"
                    | "damsel_rescued"
                    | "sacrificed_damsel"
                    | "made_udjat_eye"
                    | "made_moai"
                    | "made_market_entrance"
                    | "made_black_market"
                    | "made_gold_entrance"
                    | "genUdjatEye"
                    | "genMarketEntrance"
                    | "genGoldEntrance"
                    | "marketEntrance"
                    | "goldEntrance"
                    | "ghost_exists"
                    | "ghost_warning"
                    | "udjat_blink"
                    | "olmec_dead"
                    | "boss_door_open"
                    | "won"
                    | "black_market"
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
            ) && !flag.starts_with("carry:")
        });
        for key in [
            "kills",
            "damsels",
            "collect",
            "collect_counter",
            "collectCounter",
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
            "xmoney",
            "xdamsels",
            "crapsPoint",
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
            "favor",
            "kaliGift",
            "kaliPunish",
            "kali_gift",
            "kali_punish",
            "arrows",
            "darkness",
            "bloodLevel",
            "thiefLevel",
            "pending_damsel_kisses",
            "time",
            "xtime",
        ] {
            game.data.remove(key);
        }
    }
    let source_start = game.flags.iter().find_map(|f| {
        f.strip_prefix("scores_start:")
            .and_then(|n| n.parse::<u8>().ok())
    });
    let title_start = game.flags.iter().find_map(|f| {
        f.strip_prefix("title_start:")
            .and_then(|n| n.parse::<u8>().ok())
    });
    game.flags
        .retain(|f| !f.starts_with("scores_start:") && !f.starts_with("title_start:"));
    // These Create-event resets apply on every visit, including living returns
    // from a challenge. They are independent of the death inventory reset.
    if kind == HubKind::Title {
        game.flags.remove("lake");
        game.flags.remove("cemetary");
        game.flags.remove("cemetery");
    }
    if kind == HubKind::Scores {
        for flag in ["dark", "darkLevel", "snake_pit", "snakePit"] {
            game.flags.remove(flag);
        }
        game.player.equipment.remove("Jordans");
        game.data.insert("arrows".into(), 0.0);
        for key in ["mini1", "mini2", "mini3"] {
            game.data.remove(key);
        }
        if let Some(pos) = match source_start {
            Some(1) => Some(Vec2::new(40.0, 56.0)),
            Some(2) => Some(Vec2::new(72.0, 88.0)),
            Some(3) => Some(Vec2::new(40.0, 120.0)),
            _ => None,
        } {
            game.player.pos = pos;
            game.player.previous_pos = pos;
        }
    }
    if kind == HubKind::Title {
        if let Some((pos, facing)) = match title_start {
            Some(1) => Some((Vec2::new(440.0, 184.0), 1)),
            Some(3) => Some((Vec2::new(344.0, 184.0), 1)),
            _ => None,
        } {
            game.player.pos = pos;
            game.player.previous_pos = pos;
            game.player.facing = facing;
        } else {
            game.player.facing = -1;
        }
    }
    initialize_brick_variants(game);
    if !game.flags.contains("low_graphics") {
        crate::generation::initialize_hub_fringe(&mut game.world, &mut game.rng);
    }
    game.data.insert("hub_draw_status".into(), 0.0);
    if kind.challenge() {
        game.player.health = 8;
        game.data.insert("points".into(), 0.0);
        game.data.insert("baskets".into(), 0.0);
        game.data.insert("stars_kills".into(), 0.0);
        let hazard = if kind == HubKind::Sun {
            200 + game.rng.range(101)
        } else {
            100
        };
        game.data.insert("hub_alarm0".into(), f64::from(hazard));
        game.data.insert(
            "hub_alarm11".into(),
            if kind == HubKind::Stars { 110.0 } else { 100.0 },
        );
        if kind == HubKind::Moon {
            game.data.insert("timer".into(), 60.0);
            game.data.insert("arrows".into(), 100.0);
            game.data.insert("hub_alarm11".into(), 101.0);
        }
    }
    // Selecting a source title spawn bypasses the purely cinematic first entrance.
    if kind == HubKind::Title {
        game.camera = Vec2::new(320.0, 0.0);
        game.previous_camera = game.camera;
    }
}
fn value(game: &Game, key: &str) -> f64 {
    game.data.get(key).copied().unwrap_or(0.0)
}
fn due(game: &mut Game, key: &str) -> bool {
    let alarm = game.data.entry(key.into()).or_default();
    if *alarm <= 0.0 {
        return false;
    }
    *alarm -= 1.0;
    *alarm == 0.0
}
fn record(game: &mut Game, key: &str, score: f64) {
    let score = score.clamp(0.0, 99.0);
    game.data.insert(key.into(), score);
    game.flags.insert("hub_challenge_finished".into());
    game.events.push(GameEvent::Message(format!(
        "Challenge score: {}",
        score as u32
    )));
}

pub fn tick(game: &mut Game, input: &InputFrame) {
    if !game.flags.contains("hub") {
        // The authored tutorial also contains an oXStart-derived return door.
        // It uses the same portal input branch as the title and records rooms.
        if game.flags.contains("tutorial") && (input.up || input.exit_pressed) {
            activate_portal(game);
        }
        return;
    }
    let kind = if game.flags.contains("hub_sun") {
        HubKind::Sun
    } else if game.flags.contains("hub_moon") {
        HubKind::Moon
    } else if game.flags.contains("hub_stars") {
        HubKind::Stars
    } else if game.flags.contains("hub_scores") {
        HubKind::Scores
    } else {
        HubKind::Title
    };
    let draw = match game.level_tick {
        0..=29 => 0.0,
        30..=59 => 1.0,
        60..=69 => 2.0,
        _ => 3.0,
    };
    game.data.insert("hub_draw_status".into(), draw);
    match kind {
        HubKind::Sun => sun(game),
        HubKind::Moon => moon(game),
        HubKind::Stars => stars(game),
        _ => {}
    }
    basketball(game);
    static_targets(game);
    if input.up || input.exit_pressed {
        activate_portal(game);
    }
}

/// The player's Title Step replaces the climbing controller before input can
/// release the rope. The dummy then ascends and fades into the exit credits.
pub(crate) fn begin_title_exit(game: &mut Game) -> bool {
    if !game.flags.contains("hub_title")
        || game.player.state != PlayerState::Climbing
        || game.player.pos.y >= 32.
        || game.state != RunState::Playing
    {
        return false;
    }
    if let Some(id) = game.player.held.take() {
        if let Some(item) = game.world.entity_mut(id) {
            item.held = false;
        }
    }
    crate::ending::begin_title_credits(game);
    true
}

fn activate_portal(game: &mut Game) {
    if !game.player.grounded
        || game.player.health <= 0
        || game.player.state == PlayerState::Dead
        || game.state == RunState::Dead
        || game.player.is_stunned()
        || game.player.attack_timer > 0
    {
        return;
    }
    let portal = game
        .world
        .entities
        .iter()
        .find(|e| {
            e.alive
                && crate::catalog::is_a(&e.object, "oXStart")
                && e.sprite_bounds().contains(game.player.pos)
        })
        .map(|e| (e.object.clone(), e.pos));
    let Some((portal, portal_pos)) = portal else {
        return;
    };
    let stages = shortcut_stages(&game.flags);
    if matches!(portal.as_str(), "oXShortcut5") && !stages[0]
        || matches!(portal.as_str(), "oXShortcut9") && !stages[1]
        || matches!(portal.as_str(), "oXShortcut13") && !stages[2]
    {
        return;
    }
    game.player.pos.x = portal_pos.x + 8.0;
    if let Some(id) = game.player.held.take() {
        if let Some(item) = game.world.entity_mut(id) {
            item.held = false;
        }
    }
    let destination = match portal.as_str() {
        "oXScores" => "scores",
        "oXTutorial" => "tutorial",
        "oXStart" if game.flags.remove("first_time") => {
            game.flags.insert("onboarding_consumed".into());
            "tutorial"
        }
        "oXTitle" => "title",
        "oXShortcut5" => "shortcut5",
        "oXShortcut9" => "shortcut9",
        "oXShortcut13" => "shortcut13",
        "oXSun" => "sun",
        "oXMoon" => "moon",
        "oXStars" => "stars",
        "oXChange" | "oXChange2" => {
            // Debounce held up: source exit animation temporarily disables control.
            if value(game, "hub_character_cooldown") > game.level_tick as f64 {
                return;
            }
            game.data.insert(
                "hub_character_cooldown".into(),
                game.level_tick as f64 + 20.0,
            );
            let (selected, other) = if portal == "oXChange" {
                ("character_damsel", "character_tunnelman")
            } else {
                ("character_tunnelman", "character_damsel")
            };
            if !game.flags.remove(selected) {
                game.flags.remove(other);
                game.flags.insert(selected.into());
            }
            let character = if game.flags.contains("character_tunnelman") {
                crate::characters::Character::TunnelMan
            } else if game.flags.contains("character_damsel") {
                crate::characters::Character::Damsel
            } else {
                crate::characters::Character::Explorer
            };
            crate::characters::apply(game, character);
            game.player.state = PlayerState::Standing;
            game.player.vel = Vec2::default();
            game.player.invincible = 0;
            game.player.facing = if portal == "oXChange" { -1 } else { 1 };
            game.sound("sndSteps", game.player.pos);
            return;
        }
        _ => "campaign",
    };
    if destination == "tutorial" {
        // Both xSTART's first-use branch and xTUTORIAL use these inventory
        // values. Only the former consumes the firstTime sentinel.
        game.world.level = 0;
        game.player.bombs = 0;
        game.player.ropes = 2;
    }
    if destination == "scores" {
        let start = if game.flags.contains("hub_sun") {
            1
        } else if game.flags.contains("hub_moon") {
            2
        } else if game.flags.contains("hub_stars") {
            3
        } else {
            0
        };
        game.flags.retain(|f| !f.starts_with("scores_start:"));
        game.flags.insert(format!("scores_start:{start}"));
    }
    if destination == "title" {
        if game.flags.contains("hub_scores") || game.flags.contains("tutorial") {
            let start = if game.flags.contains("tutorial") {
                3
            } else {
                1
            };
            if game.flags.contains("tutorial") {
                game.flags.insert("reset_run_on_title".into());
            }
            game.flags.retain(|f| !f.starts_with("title_start:"));
            game.flags.insert(format!("title_start:{start}"));
        }
    }
    game.flags.retain(|f| !f.starts_with("hub_destination:"));
    game.flags.insert(format!("hub_destination:{destination}"));
    game.sound("sndSteps", game.player.pos);
}
fn poof_pair(game: &mut Game, pos: Vec2, horizontal: bool) {
    for sign in [-1.0, 1.0] {
        let id = game.world.spawn("oPoof", pos);
        game.world.entity_mut(id).unwrap().vel = if horizontal {
            Vec2::new(sign, 0.0)
        } else {
            Vec2::new(0.0, sign)
        };
    }
}
fn blood(game: &mut Game, pos: Vec2, count: usize) {
    for _ in 0..count {
        if game
            .world
            .entities
            .iter()
            .filter(|e| e.alive && crate::catalog::is_a(&e.object, "oDetritus"))
            .count()
            < 16
        {
            game.world.spawn("oBlood", pos);
        }
    }
}
fn sun(game: &mut Game) {
    if game
        .world
        .entities
        .iter()
        .all(|e| e.object != "oDamsel" || !e.alive || e.health < 1)
    {
        game.player.health = 0;
    }
    if game.player.health <= 0
        || game.player.state == PlayerState::Dead
        || game.state == RunState::Dead
    {
        if !game.flags.contains("hub_challenge_finished") {
            record(game, "mini1", value(game, "points"));
        }
        return;
    }
    if due(game, "hub_alarm11") {
        let score = (value(game, "points") + 1.0).min(99.0);
        game.data.insert("points".into(), score);
        game.data.insert("hub_alarm11".into(), 30.0);
    }
    if due(game, "hub_alarm0") {
        let x = game.player.pos.x.clamp(40.0, 280.0);
        if game.world.solid_at(x, 32.0) {
            game.data.insert("hub_alarm0".into(), 1.0);
            return;
        }
        let score = value(game, "points");
        if score >= 80.0 {
            if !game
                .world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oGhost")
            {
                game.world.spawn(
                    "oGhost",
                    Vec2::new(
                        if game.player.pos.x > 160.0 {
                            -32.0
                        } else {
                            320.0
                        },
                        104.0,
                    ),
                );
            }
        } else if score >= 50.0 {
            let id = game.world.spawn("oVampire", Vec2::new(152.0, 32.0));
            game.world.entity_mut(id).unwrap().state = 7;
            poof_pair(game, Vec2::new(160.0, 40.0), true);
        } else {
            let id = game.world.spawn("oBomb", Vec2::new(x, 32.0));
            let e = game.world.entity_mut(id).unwrap();
            e.sprite = "sBombArmed".into();
            e.timer = 80;
            e.data.insert("armed".into(), 1.0);
            e.data.insert("safe_ticks".into(), 10.0);
            poof_pair(game, Vec2::new(x, 32.0), true);
        }
        let delay = 100 + game.rng.range(101);
        game.data.insert("hub_alarm0".into(), f64::from(delay));
    }
}
fn stars(game: &mut Game) {
    if game.player.health <= 0
        || game.player.state == PlayerState::Dead
        || game.state == RunState::Dead
    {
        if !game.flags.contains("hub_challenge_finished") {
            record(game, "mini3", value(game, "stars_kills"));
        }
        return;
    }
    let kills = value(game, "stars_kills");
    if due(game, "hub_alarm0") {
        let x = 32.0 + 80.0 * f64::from(game.rng.range(4));
        let id = game.world.spawn("oShopkeeper2", Vec2::new(x, 0.0));
        let e = game.world.entity_mut(id).unwrap();
        e.text.insert("source_object".into(), "oShopkeeper2".into());
        e.state = 2;
        e.data.insert("hasGun".into(), 0.0);
        e.data.insert("has_gun".into(), 0.0);
        e.data.insert("deathTimer".into(), 200.0);
        game.data.insert(
            "hub_alarm0".into(),
            match kills as u32 {
                0..=9 => 200.0,
                10..=19 => 175.0,
                20..=29 => 150.0,
                30..=39 => 125.0,
                _ => 100.0,
            },
        );
    }
    if due(game, "hub_alarm11") {
        if f64::from(1 + game.rng.range(100)) < kills {
            let x = 32.0 + 80.0 * f64::from(game.rng.range(4));
            game.world.spawn("oSpider", Vec2::new(x, 0.0));
        }
        game.data.insert("hub_alarm11".into(), 20.0);
    }
}
fn spawn_target(game: &mut Game, x: f64, y: f64) {
    let difficulty = if x == 208.0 {
        2.0
    } else if x == 256.0 {
        4.0
    } else {
        1.0
    };
    let ufo = value(game, "timer") <= 20.0;
    let id = game.world.spawn("oBatTarget", Vec2::new(x, y));
    let e = game.world.entity_mut(id).unwrap();
    e.data.insert(
        "difficulty".into(),
        difficulty + if ufo { 2.0 } else { 0.0 },
    );
    e.data.insert("xMid".into(), x);
    e.data.insert("dir".into(), 1.0);
    if ufo {
        e.sprite = "sUFO".into();
    }
}
fn moon(game: &mut Game) {
    if game.level_tick == 70 {
        for (x, y) in [(160.0, -16.0), (208.0, -64.0), (256.0, -128.0)] {
            spawn_target(game, x, y);
        }
    }
    let died = game.player.health <= 0
        || game.player.state == PlayerState::Dead
        || game.state == RunState::Dead;
    if !game.flags.contains("hub_challenge_finished")
        && (died
            || due(game, "hub_alarm11") && {
                let timer = value(game, "timer") - 1.0;
                game.data.insert("timer".into(), timer);
                game.data.insert("hub_alarm11".into(), 30.0);
                timer == 0.0
            })
    {
        record(game, "mini2", value(game, "baskets"));
        game.data.insert("timer".into(), -1.0);
        if !died {
            let doors: Vec<_> = game
                .world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == "oEntrance")
                .map(|e| (e.id, e.pos))
                .collect();
            for (id, pos) in doors {
                game.world.entity_mut(id).unwrap().alive = false;
                game.world.spawn("oXScores", pos);
            }
            let targets: Vec<_> = game
                .world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == "oBatTarget")
                .map(|e| (e.id, e.pos))
                .collect();
            for (id, pos) in targets {
                game.world.entity_mut(id).unwrap().alive = false;
                poof_pair(game, Vec2::new(pos.x + 8.0, pos.y), false);
            }
        }
    }
    if game.flags.contains("hub_challenge_finished") {
        return;
    }
    let ids: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oBatTarget")
        .map(|e| e.id)
        .collect();
    for id in ids {
        let timer = value(game, "timer");
        let e = game.world.entity_mut(id).unwrap();
        let speed = e.value("difficulty");
        if e.value("dir") == 0.0 {
            e.pos.y -= speed;
            if e.pos.y <= 64.0 {
                e.data.insert("dir".into(), 1.0);
            }
        } else {
            e.pos.y += speed;
            if e.pos.y >= 160.0 {
                e.data.insert("dir".into(), 0.0);
            }
        }
        if timer <= 20.0 && e.pos.y > 64.0 {
            e.pos.x = e.value("xMid") - (e.value("xDiff").sin() * 32.0).abs();
            e.data.insert("xDiff".into(), e.value("xDiff") + 0.01);
        }
        let target = e.clone();
        let arrow = game
            .world
            .entities
            .iter()
            .find(|a| {
                a.alive && a.object == "oArrow" && swept(a).intersects(target.sprite_bounds())
            })
            .map(|a| a.id);
        let aligned = (game.player.pos.x - target.pos.x - 8.0).abs() <= 12.0;
        let touching = game
            .player
            .sprite_bounds()
            .intersects(target.sprite_bounds())
            && aligned;
        let stomp = touching
            && matches!(
                game.player.state,
                PlayerState::Jumping | PlayerState::Falling
            )
            && game.player.data.get("swimming").copied().unwrap_or(0.0) == 0.0
            && game.player.pos.y < target.pos.y + 8.0;
        if arrow.is_some() || stomp {
            if let Some(arrow) = arrow {
                game.world.entity_mut(arrow).unwrap().alive = false;
            }
            if stomp {
                game.player.vel.y = -6.0 - 0.2 * game.player.vel.y;
            }
            *game.data.entry("baskets".into()).or_default() += speed;
            game.world.entity_mut(id).unwrap().alive = false;
            spawn_target(
                game,
                if stomp {
                    target.pos.x
                } else {
                    target.value("xMid")
                },
                -64.0,
            );
            if !stomp && target.sprite == "sUFO" {
                game.world.spawn("oUFOCrash", target.pos);
            } else {
                blood(game, Vec2::new(target.pos.x + 8.0, target.pos.y + 8.0), 3);
            }
            game.sound("sndHit", target.pos);
            game.sound("sndCoin", target.pos);
        } else if touching
            && game.player.invincible == 0
            && game.player.state != PlayerState::Dead
            && game.state != RunState::Dead
        {
            // oBatTarget contact sets blink/invulnerability, not stunned.
            game.player.health -= 1;
            game.events.push(GameEvent::Damage { amount: 1 });
            game.sound("sndHurt", game.player.pos);
            blood(game, Vec2::new(target.pos.x + 4.0, target.pos.y + 4.0), 1);
            game.player.invincible = 30;
            game.player.data.insert("blink".into(), 30.);
            game.player.vel.x = if game.player.pos.x < target.pos.x {
                -6.0
            } else {
                6.0
            };
        }
    }
}
fn swept(e: &Entity) -> Rect {
    let now = e.sprite_bounds();
    Rect::new(
        now.x.min(now.x + e.previous_pos.x - e.pos.x),
        now.y.min(now.y + e.previous_pos.y - e.pos.y),
        now.w + (e.pos.x - e.previous_pos.x).abs(),
        now.h + (e.pos.y - e.previous_pos.y).abs(),
    )
}

/// Legacy basketball objects are supported even though Classic 1.1's Moon room
/// has no basketball instances (its spawning controller code is commented out).
fn basketball(game: &mut Game) {
    let rims: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oRim")
        .map(Entity::bounds)
        .collect();
    let deflectors: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oRimDeflect")
        .cloned()
        .collect();
    for index in 0..game.world.entities.len() {
        let mut ball = game.world.entities[index].clone();
        if !ball.alive || ball.object != "oBasketball" {
            continue;
        }
        ball.data.insert("bboxLeft".into(), -4.0);
        ball.data.insert("bboxRight".into(), 4.0);
        ball.data.insert("bboxTop".into(), -4.0);
        ball.data.insert("bboxBottom".into(), 4.0);
        ball.data.insert("bounceFactor".into(), 0.7);
        ball.data.insert("frictionFactor".into(), 0.6);
        if ball.previous_pos.x != ball.pos.x
            && ball.vel.x * (ball.pos.x - ball.previous_pos.x) < 0.0
        {
            ball.data.insert("wallBounce".into(), 1.0);
        }
        if ball.previous_pos.y < ball.pos.y && ball.vel.y <= 0.0 {
            ball.data.insert("wallBounce".into(), 0.0);
        }
        if ball.value("goingIn") == 0.0 && ball.value("bounces") < 40.0 {
            let contact = Rect::new(ball.pos.x - 1.0, ball.pos.y + 4.0, 2.0, 1.0);
            if let Some(rim) = deflectors
                .iter()
                .filter(|rim| rim.bounds().intersects(contact))
                .min_by(|a, b| {
                    a.pos
                        .distance(ball.pos)
                        .total_cmp(&b.pos.distance(ball.pos))
                })
            {
                ball.vel.y *= -0.8;
                let center = rim.pos.x + 8.0;
                let distance = (center - ball.pos.x).abs();
                if ball.pos.x < center && distance > 7.0 {
                    ball.vel = Vec2::new(-2.0, -0.5);
                } else if ball.pos.x > center && distance > 7.0 {
                    ball.vel = Vec2::new(2.0, -0.5);
                } else if distance < 4.0 {
                    ball.vel = Vec2::new(if ball.pos.x < center { 1.0 } else { -0.2 }, -0.5);
                }
                ball.data
                    .insert("bounces".into(), ball.value("bounces") + 10.0);
            }
        }
        if ball.value("bounces") > 0.0 {
            ball.data
                .insert("bounces".into(), ball.value("bounces") - 1.0);
        }
        let inside = rims.iter().any(|r| r.contains(ball.pos));
        if inside
            && ball.value("goingIn") == 0.0
            && (ball.vel.y > 0.0 || ball.held && game.player.state == PlayerState::Falling)
        {
            let points = if ball.held {
                1.0
            } else if ball.value("wallBounce") != 0.0 {
                3.0
            } else {
                2.0
            };
            *game.data.entry("baskets".into()).or_default() += points;
            ball.data.insert("goingIn".into(), 1.0);
            ball.vel = Vec2::default();
            ball.held = false;
            if game.player.held == Some(ball.id) {
                game.player.held = None;
            }
            game.data.insert("hub_alarm0".into(), 20.0);
            if let Some(basket) = game
                .world
                .entities
                .iter_mut()
                .filter(|e| e.object == "oBasket")
                .min_by(|a, b| {
                    a.pos
                        .distance(ball.pos)
                        .total_cmp(&b.pos.distance(ball.pos))
                })
            {
                basket.sprite = "sBasketSwoosh".into();
                basket.animation = 0.0;
            }
        } else if !inside {
            ball.data.insert("goingIn".into(), 0.0);
        }
        ball.sprite = if ball.held && game.world.solid_at(ball.pos.x, ball.pos.y + 6.0) {
            "sDribble"
        } else {
            "sBasketball"
        }
        .into();
        game.world.entities[index] = ball;
    }
}
fn static_targets(game: &mut Game) {
    let ids: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oTarget")
        .map(|e| e.id)
        .collect();
    for id in ids {
        let timer = value(game, "timer");
        let initial_dir = if game.world.entity(id).unwrap().value("hub_initialized") == 0.0 {
            game.rng.range(2)
        } else {
            0
        };
        let e = game.world.entity_mut(id).unwrap();
        if e.value("hub_initialized") == 0.0 {
            e.data.insert("hub_initialized".into(), 1.0);
            e.data.insert("life".into(), 20.0);
            e.data.insert("dir".into(), f64::from(initial_dir));
            e.data.insert("moveOff".into(), 32.0);
            e.data.insert("hard".into(), f64::from(e.pos.x > 208.0));
        }
        if e.value("dying") != 0.0 {
            if e.value("life") > 0.0 {
                e.data.insert("life".into(), e.value("life") - 1.0);
            } else {
                e.data.insert("life".into(), 20.0);
                e.data.insert("dying".into(), 0.0);
            }
        } else {
            let up = e.value("dir") == 0.0;
            let speed = if e.value("hard") != 0.0 { 2.0 } else { 1.0 };
            e.pos.y += if up { -speed } else { speed };
            e.data.insert("moveOff".into(), e.value("moveOff") + 1.0);
            if up && e.pos.y <= 64.0 || !up && e.pos.y >= 160.0 || e.value("moveOff") > 64.0 {
                e.data.insert("dir".into(), f64::from(up));
                e.data.insert("moveOff".into(), 0.0);
            }
            if timer <= 30.0 {
                e.pos.x = 240.0 - (e.value("xDiff").sin() * 64.0).abs();
                e.data.insert("xDiff".into(), e.value("xDiff") + 0.01);
            }
        }
        let target = e.clone();
        let arrows: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|a| {
                a.alive && a.object == "oArrow" && swept(a).intersects(target.sprite_bounds())
            })
            .map(|a| (a.id, a.pos.y))
            .collect();
        for (arrow, y) in arrows {
            let a = game.world.entity_mut(arrow).unwrap();
            a.vel = Vec2::default();
            if target.value("life") <= 2.0 {
                a.alive = false;
            }
            if game.world.entity(id).unwrap().value("dying") == 0.0 {
                game.world
                    .entity_mut(id)
                    .unwrap()
                    .data
                    .insert("dying".into(), 1.0);
                let d = (y - target.pos.y).abs();
                let points = if d <= 2.0 {
                    5.0
                } else if (3.0..=6.0).contains(&d) {
                    3.0
                } else if (7.0..=10.0).contains(&d) {
                    2.0
                } else {
                    1.0
                };
                *game.data.entry("baskets".into()).or_default() += points;
                game.sound("sndHit", target.pos);
                if points == 5.0 {
                    game.sound("sndCoin", target.pos);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn challenge(kind: HubKind) -> Game {
        let mut g = Game::shell(7, RulesPreset::Classic);
        g.world = world(kind, &g.flags);
        g.player.pos = g.world.entrance;
        initialize(&mut g, kind);
        g
    }
    fn loaded(kind: HubKind) -> Game {
        let mut g = Game::shell(7, RulesPreset::Classic);
        g.load_hub(kind);
        g
    }
    #[test]
    fn campaign_door_is_reachable_from_source_title_spawn() {
        let mut g = loaded(HubKind::Title);
        for _ in 0..200 {
            if (384.0..400.0).contains(&g.player.pos.x) {
                break;
            }
            g.step(&InputFrame {
                left: true,
                ..InputFrame::default()
            });
        }
        assert!(
            (384.0..400.0).contains(&g.player.pos.x),
            "title movement stopped at {:?}",
            g.player.pos
        );
        g.step(&InputFrame {
            up: true,
            ..InputFrame::default()
        });
        assert!(
            g.flags.contains("hub_destination:campaign"),
            "pos {:?} state {:?}",
            g.player.pos,
            g.player.state
        );
    }
    #[test]
    fn unlocked_title_ladder_can_be_climbed_in_integrated_steps() {
        let mut g = Game::shell(7, RulesPreset::Classic);
        g.flags.insert("shortcut_5".into());
        g.load_hub(HubKind::Title);
        g.player.pos = Vec2::new(360.0, 184.0);
        g.player.previous_pos = g.player.pos;
        for _ in 0..100 {
            g.step(&InputFrame {
                up: true,
                ..InputFrame::default()
            });
        }
        assert_eq!(g.state, RunState::Playing);
        assert_eq!(g.player.state, PlayerState::Climbing);
        assert!(
            g.player.pos.y < 112.0,
            "climb stopped at {:?}",
            g.player.pos
        );
    }
    #[test]
    fn hub_brick_creation_preserves_covered_gold_and_seeded_replay() {
        let a = loaded(HubKind::Title);
        let b = loaded(HubKind::Title);
        assert_eq!(a.state_hash(), b.state_hash());
        assert!(
            a.world
                .entities
                .iter()
                .any(|e| e.object == "oTerrainMetadata" && e.value("gold_chunks") == 3.0)
        );
        for e in a
            .world
            .entities
            .iter()
            .filter(|e| e.object == "oTerrainMetadata")
        {
            if e.text
                .get("terrain_sprite")
                .is_some_and(|s| matches!(s.as_str(), "sCaveUp" | "sCaveUp2" | "sBrickDown"))
            {
                assert_eq!(e.value("gold_chunks"), 0.0);
            }
            assert!(!e.text.contains_key("treasure"));
        }
    }
    #[test]
    fn integrated_hub_inventory_controls_are_disabled_but_campaign_bombs_work() {
        for kind in [
            HubKind::Title,
            HubKind::Scores,
            HubKind::Sun,
            HubKind::Moon,
            HubKind::Stars,
        ] {
            let mut g = loaded(kind);
            let inventory = (g.player.bombs, g.player.ropes);
            g.step(&InputFrame {
                cycle_pressed: true,
                bomb_pressed: true,
                rope_pressed: true,
                ..InputFrame::default()
            });
            assert_eq!((g.player.bombs, g.player.ropes), inventory, "{kind:?}");
            assert!(g.player.held.is_none(), "{kind:?}: inventory cycle");
            assert!(
                !g.world
                    .entities
                    .iter()
                    .any(|e| matches!(e.object.as_str(), "oBomb" | "oRopeThrow")),
                "{kind:?}: dedicated inventory action"
            );
        }
        let mut g = Game::shell(7, RulesPreset::Classic);
        for x in 0..g.world.width {
            g.world.set_tile(x, 3, "oBlock");
        }
        g.step(&InputFrame {
            bomb_pressed: true,
            ..InputFrame::default()
        });
        assert_eq!(g.player.bombs, 3);
        assert!(
            g.world
                .entities
                .iter()
                .any(|e| e.object == "oBomb" && e.value("armed") != 0.0)
        );
    }
    #[test]
    fn authored_spawns_survive_and_settle_in_integrated_steps() {
        for (kind, spawn) in [
            (HubKind::Title, Vec2::new(600.0, 184.0)),
            (HubKind::Scores, Vec2::new(40.0, 184.0)),
            (HubKind::Sun, Vec2::new(56.0, 168.0)),
            (HubKind::Moon, Vec2::new(40.0, 88.0)),
            (HubKind::Stars, Vec2::new(40.0, 200.0)),
        ] {
            let mut g = loaded(kind);
            assert_eq!(g.player.pos, spawn);
            for _ in 0..10 {
                g.step(&InputFrame::default());
            }
            assert_eq!(
                g.state,
                RunState::Playing,
                "{kind:?} pos {:?}",
                g.player.pos
            );
            assert!(g.player.health > 0, "{kind:?}");
            assert!(g.player.grounded, "{kind:?} pos {:?}", g.player.pos);
            assert_eq!(g.player.pos, spawn, "{kind:?}");
        }
    }
    #[test]
    fn dead_challenge_can_return_to_playable_title_and_scores() {
        for destination in [HubKind::Title, HubKind::Scores] {
            let mut g = loaded(HubKind::Stars);
            g.player.health = 0;
            g.step(&InputFrame::default());
            assert_eq!(g.state, RunState::Dead);
            g.player.equipment.insert("Jetpack".into());
            g.player.money = 5000;
            g.flags.insert("murderer".into());
            g.load_hub(destination);
            for _ in 0..10 {
                g.step(&InputFrame::default());
            }
            assert_eq!(g.state, RunState::Playing);
            assert_eq!(g.player.health, 4);
            assert!(g.player.equipment.is_empty());
            assert_eq!(g.player.money, 0);
            assert!(!g.flags.contains("murderer"));
        }
    }
    #[test]
    fn changing_doors_restore_source_standing_state_and_drop_held_items() {
        for (kind, flag, door, facing, health) in [
            (HubKind::Scores, "unlock_damsel", "oXChange", -1, 4),
            (HubKind::Title, "unlock_tunnel_man", "oXChange2", 1, 2),
        ] {
            let mut g = Game::shell(7, RulesPreset::Classic);
            g.flags.insert(flag.into());
            g.load_hub(kind);
            let pos = g
                .world
                .entities
                .iter()
                .find(|e| e.object == door)
                .unwrap()
                .pos;
            g.player.pos = Vec2::new(pos.x + 8.0, pos.y + 8.0);
            g.player.previous_pos = g.player.pos;
            g.player.state = PlayerState::Standing;
            g.player.vel.x = 1.0;
            let id = g.world.spawn("oRock", g.player.pos);
            g.world.entity_mut(id).unwrap().held = true;
            g.player.held = Some(id);
            g.step(&InputFrame {
                up: true,
                ..InputFrame::default()
            });
            assert_eq!(g.player.health, health, "{kind:?}");
            assert_eq!(g.player.pos.x, pos.x + 8.0);
            assert_eq!(g.player.facing, facing);
            assert_eq!(g.player.state, PlayerState::Standing);
            assert_eq!(g.player.vel, Vec2::default());
            assert!(g.player.held.is_none());
            assert!(!g.world.entity(id).unwrap().held);
        }
    }
    #[test]
    fn moon_contact_damage_does_not_invent_a_stun() {
        let mut g = loaded(HubKind::Moon);
        for _ in 0..70 {
            g.step(&InputFrame::default());
        }
        assert_eq!(g.player.health, 8);
        let target = g
            .world
            .entities
            .iter()
            .find(|e| e.alive && e.object == "oBatTarget")
            .unwrap()
            .id;
        for e in &mut g.world.entities {
            if e.object == "oBatTarget" && e.id != target {
                e.alive = false;
            }
        }
        let e = g.world.entity_mut(target).unwrap();
        e.pos = Vec2::new(176.0, 116.0);
        e.previous_pos = e.pos;
        g.player.pos = Vec2::new(184.0, 128.0);
        g.player.previous_pos = g.player.pos;
        g.player.vel = Vec2::default();
        g.player.invincible = 0;
        g.step(&InputFrame::default());
        assert_eq!(g.player.health, 7);
        assert_eq!(g.player.stun, 0);
        assert_eq!(g.player.invincible, 30);
        assert_eq!(g.player.data["blink"], 30.);
        assert_eq!(g.data["baskets"], 0.0);
    }
    #[test]
    fn moon_arrow_hit_scores_and_replaces_the_authored_target_in_real_step() {
        let mut g = loaded(HubKind::Moon);
        for _ in 0..70 {
            g.step(&InputFrame::default());
        }
        let id = g
            .world
            .entities
            .iter()
            .find(|e| e.object == "oBatTarget")
            .unwrap()
            .id;
        let e = g.world.entity_mut(id).unwrap();
        e.pos = Vec2::new(160.0, 100.0);
        e.previous_pos = e.pos;
        e.data.insert("dir".into(), 1.0);
        let arrow = g.world.spawn("oArrow", Vec2::new(144.0, 106.0));
        g.world.entity_mut(arrow).unwrap().vel = Vec2::new(20.0, 0.0);
        g.step(&InputFrame::default());
        assert!(g.world.entity(id).is_none());
        assert!(g.world.entity(arrow).is_none());
        assert_eq!(g.data["baskets"], 1.0);
        assert!(
            g.world
                .entities
                .iter()
                .any(|e| e.object == "oBatTarget" && e.pos == Vec2::new(160.0, -64.0))
        );
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| e.object == "oBlood")
                .count(),
            3
        );
    }
    #[test]
    fn moon_death_stops_timer_without_timeout_door_replacement() {
        let mut g = loaded(HubKind::Moon);
        for _ in 0..70 {
            g.step(&InputFrame::default());
        }
        g.data.insert("baskets".into(), 23.0);
        g.player.health = 0;
        g.step(&InputFrame::default());
        assert_eq!(g.state, RunState::Dead);
        assert_eq!(g.data["mini2"], 23.0);
        assert_eq!(g.data["timer"], -1.0);
        assert!(g.world.entities.iter().any(|e| e.object == "oEntrance"));
        assert!(!g.world.entities.iter().any(|e| e.object == "oXScores"));
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| e.object == "oBatTarget")
                .count(),
            3
        );
    }
    #[test]
    fn integrated_sun_protection_and_stars_kill_recording_match_source() {
        let mut sun = loaded(HubKind::Sun);
        sun.data.insert("points".into(), 42.0);
        sun.world
            .entities
            .iter_mut()
            .find(|e| e.object == "oDamsel")
            .unwrap()
            .health = 0;
        sun.step(&InputFrame::default());
        assert_eq!(sun.state, RunState::Dead);
        assert_eq!(sun.data["mini1"], 42.0);
        let mut stars = loaded(HubKind::Stars);
        for _ in 0..100 {
            stars.step(&InputFrame::default());
        }
        let shop = stars
            .world
            .entities
            .iter()
            .find(|e| e.object == "oShopkeeper2")
            .unwrap()
            .id;
        assert_eq!(stars.world.entity(shop).unwrap().health, 20);
        stars.world.entity_mut(shop).unwrap().health = 0;
        stars.step(&InputFrame::default());
        assert_eq!(stars.data["stars_kills"], 1.0);
        assert_eq!(stars.data.get("kills").copied().unwrap_or(0.0), 0.0);
        assert!(!stars.flags.contains("murderer"));
    }
    #[test]
    fn moon_timeout_return_restores_source_scores_door_position() {
        let mut g = loaded(HubKind::Moon);
        g.data.insert("timer".into(), 1.0);
        g.data.insert("hub_alarm11".into(), 1.0);
        g.data.insert("baskets".into(), 42.0);
        g.step(&InputFrame::default());
        assert_eq!(g.data["mini2"], 42.0);
        assert_eq!(g.state, RunState::Playing);
        g.step(&InputFrame {
            up: true,
            ..InputFrame::default()
        });
        assert!(g.flags.contains("hub_destination:scores"));
        assert!(g.flags.contains("scores_start:2"));
        g.flags.insert("unlock_moon".into());
        g.load_hub(HubKind::Scores);
        assert_eq!(g.player.pos, Vec2::new(72.0, 88.0));
        assert_eq!(g.player.pos, g.player.previous_pos);
        assert_eq!(g.data["arrows"], 0.0);
        for _ in 0..5 {
            g.step(&InputFrame::default());
        }
        assert!(g.player.grounded);
        assert_eq!(g.state, RunState::Playing);
    }
    #[test]
    fn title_shortcut_geometry_matches_all_donation_stages() {
        for (stage, flag) in [
            (0, None),
            (1, Some("shortcut_5")),
            (2, Some("shortcut_9")),
            (3, Some("shortcut_13")),
        ] {
            let mut flags = BTreeSet::new();
            if let Some(flag) = flag {
                flags.insert(flag.into());
            }
            let w = world(HubKind::Title, &flags);
            let ladder = w
                .tile(22, 6)
                .is_some_and(|tile| tile.object == "oLadderOrange");
            assert_eq!(ladder, stage >= 1, "stage {stage}: ladder");
            assert_eq!(
                w.tile(20, 6).is_some_and(|tile| tile.object == "oBrick"),
                stage == 0,
                "stage {stage}: blocking brick 320,96"
            );
            assert_eq!(
                w.tile(21, 6).is_some_and(|tile| tile.object == "oBrick"),
                stage == 0,
                "stage {stage}: blocking brick 336,96"
            );
            assert!(
                w.entities.iter().any(|e| e.object == "oXShortcut5"),
                "source-authored first shortcut remains"
            );
            assert_eq!(
                w.entities.iter().any(|e| e.object == "oXShortcut9"),
                stage >= 2,
                "stage {stage}: level9 door"
            );
            assert_eq!(
                w.entities.iter().any(|e| e.object == "oLevel9Sign"),
                stage >= 2,
                "stage {stage}: level9 sign"
            );
            assert_eq!(
                w.entities.iter().any(|e| e.object == "oXShortcut13"),
                stage >= 3,
                "stage {stage}: level13 door"
            );
            assert_eq!(
                w.entities.iter().any(|e| e.object == "oLevel13Sign"),
                stage >= 3,
                "stage {stage}: level13 sign"
            );
        }
    }
    #[test]
    fn each_shortcut_portal_requires_its_completed_stage() {
        for (object, destination, flag) in [
            ("oXShortcut5", "shortcut5", "shortcut_5"),
            ("oXShortcut9", "shortcut9", "shortcut_9"),
            ("oXShortcut13", "shortcut13", "shortcut_13"),
        ] {
            for unlocked in [false, true] {
                let mut g = challenge(HubKind::Title);
                let id = g.world.spawn(object, Vec2::new(600.0, 64.0));
                let bounds = g.world.entity(id).unwrap().bounds();
                g.player.pos = Vec2::new(bounds.x + bounds.w / 2.0, bounds.y + bounds.h / 2.0);
                g.player.grounded = true;
                if unlocked {
                    g.flags.insert(flag.into());
                }
                tick(
                    &mut g,
                    &InputFrame {
                        up: true,
                        ..InputFrame::default()
                    },
                );
                assert_eq!(
                    g.flags.contains(&format!("hub_destination:{destination}")),
                    unlocked,
                    "{object} unlocked={unlocked}"
                );
            }
        }
    }
    #[test]
    fn record_thresholds_and_valid_win_time() {
        assert!(
            record_unlocks(199999, Some(601.0), 119, 7)
                .iter()
                .all(|f| !f.starts_with("unlock_"))
        );
        let flags = record_unlocks(200000, Some(600.0), 120, 8);
        for key in [
            "unlock_sun",
            "unlock_moon",
            "unlock_stars",
            "unlock_damsel",
            "unlock_tunnel_man",
        ] {
            assert!(flags.contains(key));
        }
        assert!(!record_unlocks(0, Some(0.0), 0, 0).contains("unlock_moon"));
        assert!(
            !record_unlocks(0, Some(960.0), 0, 0)
                .iter()
                .any(|f| f.starts_with("trophy_time_"))
        );
        assert!(record_unlocks(50000, Some(959.0), 80, 4).contains("trophy_money_1"));
    }
    #[test]
    fn source_portal_requests_destination_without_advancing_campaign() {
        let mut g = challenge(HubKind::Title);
        let door = g
            .world
            .entities
            .iter()
            .find(|e| e.object == "oXStart")
            .unwrap()
            .bounds();
        g.player.pos = Vec2::new(door.x + door.w / 2.0, door.y + door.h / 2.0);
        g.player.grounded = true;
        tick(
            &mut g,
            &InputFrame {
                up: true,
                ..InputFrame::default()
            },
        );
        assert!(g.flags.contains("hub_destination:campaign"));
        assert_eq!(g.world.level, 0);
    }
    #[test]
    fn sun_damsel_loss_finishes_source_counter() {
        let mut g = challenge(HubKind::Sun);
        g.data.insert("points".into(), 42.0);
        for e in &mut g.world.entities {
            if e.object == "oDamsel" {
                e.health = 0;
            }
        }
        tick_hub(&mut g);
        assert_eq!(g.player.health, 0);
        assert_eq!(g.data["mini1"], 42.0);
        assert!(g.flags.contains("hub_challenge_finished"));
    }
    #[test]
    fn authored_rooms_have_source_dimensions_and_spawns() {
        let flags = BTreeSet::new();
        let title = world(HubKind::Title, &flags);
        assert_eq!((title.width, title.height), (40, 15));
        assert_eq!(title.entrance, Vec2::new(600.0, 184.0));
        let sun = world(HubKind::Sun, &flags);
        assert_eq!((sun.width, sun.height), (20, 16));
        assert_eq!(sun.entrance, Vec2::new(56.0, 168.0));
        assert!(sun.entities.iter().any(|e| e.object == "oDamsel"));
        assert!(
            sun.entities
                .iter()
                .any(|e| e.object == "oTerrainMetadata" && e.value("invincible") == 1.0)
        );
    }
    #[test]
    fn gold_gates_are_independent() {
        let mut flags = BTreeSet::new();
        flags.insert("unlock_moon".into());
        let w = world(HubKind::Scores, &flags);
        assert!(w.entities.iter().any(|e| e.object == "oXMoon"));
        assert!(!w.entities.iter().any(|e| e.object == "oXSun"));
    }
    #[test]
    fn sun_first_point_at_tick100() {
        let mut g = challenge(HubKind::Sun);
        for tick in 1..=100 {
            g.level_tick = tick;
            tick_hub(&mut g);
        }
        assert_eq!(g.data["points"], 1.0);
        assert_eq!(g.player.health, 8);
    }
    fn tick_hub(g: &mut Game) {
        tick(g, &InputFrame::default())
    }
    #[test]
    fn moon_targets_and_timeout_follow_source_alarms() {
        let mut g = challenge(HubKind::Moon);
        for t in 1..=70 {
            g.level_tick = t;
            tick_hub(&mut g);
        }
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == "oBatTarget")
                .count(),
            3
        );
        g.data.insert("hub_alarm11".into(), 1.0);
        g.data.insert("timer".into(), 1.0);
        g.data.insert("baskets".into(), 105.0);
        tick_hub(&mut g);
        assert_eq!(g.data["mini2"], 99.0);
        assert_eq!(g.data["timer"], -1.0);
        assert!(
            g.world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oXScores")
        );
    }
    #[test]
    fn basketball_scores_dunk_shot_and_wall_bounce() {
        for (held, wall, score) in [(true, false, 1.0), (false, false, 2.0), (false, true, 3.0)] {
            let mut g = challenge(HubKind::Moon);
            let rim = g.world.spawn("oRim", Vec2::new(100.0, 100.0));
            let pos = {
                let r = g.world.entity(rim).unwrap().bounds();
                Vec2::new(r.x + r.w / 2.0, r.y + r.h / 2.0)
            };
            let id = g.world.spawn("oBasketball", pos);
            let b = g.world.entity_mut(id).unwrap();
            b.held = held;
            b.vel.y = 1.0;
            b.data.insert("wallBounce".into(), f64::from(wall));
            g.player.state = PlayerState::Falling;
            basketball(&mut g);
            assert_eq!(g.data["baskets"], score);
            basketball(&mut g);
            assert_eq!(g.data["baskets"], score);
        }
    }
    #[test]
    fn actual_tutorial_return_door_preserves_unlocks_and_clears_run() {
        let mut g = Game::tutorial(11, RulesPreset::Classic);
        g.flags.insert("shortcut_5".into());
        g.flags.insert("wanted".into());
        g.player.money = 12345;
        g.data.insert("kali_punish".into(), 1.);
        g.data.insert("kali_gift".into(), 1.);
        g.player.bombs = 33;
        g.player.equipment.insert("Jetpack".into());
        for _ in 0..5 {
            g.step(&InputFrame::default());
        }
        assert!(g.player.grounded);
        assert_eq!(g.player.pos, Vec2::new(24., 72.));
        g.step(&InputFrame {
            up: true,
            ..Default::default()
        });
        assert!(g.flags.contains("hub_destination:title"));
        assert!(g.flags.contains("title_start:3"));
        g.load_hub(HubKind::Title);
        assert_eq!(g.player.pos, Vec2::new(344., 184.));
        assert_eq!((g.player.health, g.player.bombs, g.player.ropes), (4, 4, 4));
        assert_eq!(g.player.money, 0);
        assert!(g.player.equipment.is_empty());
        assert!(!g.flags.contains("wanted"));
        assert!(g.flags.contains("shortcut_5"));
        assert!(!g.data.contains_key("kali_punish"));
        assert!(!g.data.contains_key("kali_gift"));
        g.step(&InputFrame::default());
        assert!(
            !g.world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oBall")
        );
    }

    #[test]
    fn living_hub_entries_apply_room_specific_source_resets() {
        let mut g = Game::shell(3, RulesPreset::Classic);
        for flag in ["lake", "cemetary", "cemetery"] {
            g.flags.insert(flag.into());
        }
        g.load_hub(HubKind::Title);
        for flag in ["lake", "cemetary", "cemetery"] {
            assert!(!g.flags.contains(flag));
        }
        for flag in ["dark", "darkLevel", "snake_pit", "snakePit"] {
            g.flags.insert(flag.into());
        }
        g.player.equipment.insert("Jordans".into());
        g.player.equipment.insert("Compass".into());
        g.load_hub(HubKind::Scores);
        for flag in ["dark", "darkLevel", "snake_pit", "snakePit"] {
            assert!(!g.flags.contains(flag));
        }
        assert!(!g.player.has("Jordans"));
        assert!(
            g.player.has("Compass"),
            "room reset must not wipe unrelated equipment"
        );
    }
    #[test]
    fn healed_dead_corpse_cannot_use_portals_and_hub_return_resets_inventory() {
        let mut g = loaded(HubKind::Title);
        let portal = g
            .world
            .entities
            .iter()
            .find(|e| crate::catalog::is_a(&e.object, "oXStart"))
            .unwrap()
            .pos;
        g.player.pos = Vec2::new(portal.x + 8., portal.y + 8.);
        g.player.grounded = true;
        g.player.health = 1;
        g.player.state = PlayerState::Dead;
        g.state = RunState::Dead;
        g.player.money = 10000;
        g.player.equipment.insert("Kapala".into());
        activate_portal(&mut g);
        assert!(!g.flags.iter().any(|f| f.starts_with("hub_destination:")));
        g.load_hub(HubKind::Scores);
        assert_eq!(g.player.health, 4);
        assert_eq!(g.player.money, 0);
        assert!(g.player.equipment.is_empty());
    }
    #[test]
    fn fresh_title_has_source_hint_and_barriers_then_walks_to_first_start_tutorial() {
        let mut g = Game::shell(3, RulesPreset::Classic);
        assert!(
            !g.flags.contains("first_time"),
            "raw core shell has no profile policy"
        );
        g.flags.insert("first_time".into());
        g.load_hub(HubKind::Title);
        let hands: Vec<_> = g
            .world
            .entities
            .iter()
            .filter(|e| e.object == "oHintHand")
            .collect();
        assert_eq!(hands.len(), 1);
        assert_eq!(hands[0].pos, Vec2::new(384., 144.));
        for (x, y) in [(20, 10), (21, 10), (21, 11)] {
            assert_eq!(g.world.tile(x, y).unwrap().object, "oBrick");
        }
        // Reach the authored start door by actual controller steps from the
        // source completed-intro position rather than teleporting into it.
        for _ in 0..150 {
            if g.player.pos.x <= 394. {
                break;
            }
            g.step(&InputFrame {
                left: true,
                ..Default::default()
            });
        }
        assert!(
            (384. ..400.).contains(&g.player.pos.x),
            "player must reach start door"
        );
        g.step(&InputFrame {
            up: true,
            ..Default::default()
        });
        assert!(g.flags.contains("hub_destination:tutorial"));
        assert!(g.flags.contains("onboarding_consumed"));
        assert!(!g.flags.contains("first_time"));
        assert_eq!((g.world.level, g.player.bombs, g.player.ropes), (0, 0, 2));
        g.load_hub(HubKind::Title);
        assert!(!g.world.entities.iter().any(|e| e.object == "oHintHand"));
        assert_eq!(g.world.tile(21, 11).unwrap().object, "");
    }

    #[test]
    fn manual_authored_tutorial_does_not_consume_first_time() {
        let mut g = Game::shell(4, RulesPreset::Classic);
        g.flags.insert("first_time".into());
        g.load_hub(HubKind::Title);
        let portal = g
            .world
            .entities
            .iter()
            .find(|e| e.object == "oXTutorial")
            .unwrap()
            .pos;
        // The fresh-profile barrier covers this door in the authored title.
        // Isolate the portal policy without weakening the geometry assertion
        // in the natural first-start test above.
        g.world.set_tile(21, 11, "");
        g.player.pos = Vec2::new(portal.x + 8., portal.y + 8.);
        g.player.previous_pos = g.player.pos;
        g.player.vel = Vec2::default();
        g.player.grounded = true;
        g.step(&InputFrame {
            up: true,
            ..Default::default()
        });
        assert!(g.flags.contains("hub_destination:tutorial"));
        assert!(g.flags.contains("first_time"));
        assert!(!g.flags.contains("onboarding_consumed"));
        assert_eq!((g.player.bombs, g.player.ropes), (0, 2));
    }
    #[test]
    fn explicit_unskipped_intro_has_barriers_but_no_early_hint_hand() {
        let flags = BTreeSet::from(["first_time".into(), "title_start:0".into()]);
        let title = world(HubKind::Title, &flags);
        assert!(!title.entities.iter().any(|e| e.object == "oHintHand"));
        assert_eq!(title.tile(21, 11).unwrap().object, "oBrick");
    }
}
