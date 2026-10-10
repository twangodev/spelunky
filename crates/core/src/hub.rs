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
            world.spawn("oTunnelMan", Vec2::new(40.0, 136.0));
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
fn add_instance(world: &mut World, object: &str, pos: Vec2, challenge: bool) {
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
fn setup_wall_sprites(world: &mut World) {
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

/// Call after assigning/resetting the authored world. Preserves persistent unlocks.
pub fn initialize(game: &mut Game, kind: HubKind) {
    game.flags.retain(|f| !f.starts_with("hub_") && f != "hub");
    game.flags.insert("hub".into());
    game.flags.insert(format!("hub_{}", kind.key()));
    game.data.retain(|key, _| {
        !key.starts_with("hub_")
            && !matches!(key.as_str(), "points" | "baskets" | "timer" | "stars_kills")
    });
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
fn activate_portal(game: &mut Game) {
    if !game.player.grounded
        || game.player.health <= 0
        || game.player.stun > 0
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
                && e.bounds().contains(game.player.pos)
        })
        .map(|e| e.object.clone());
    let Some(portal) = portal else {
        return;
    };
    let stages = shortcut_stages(&game.flags);
    if matches!(portal.as_str(), "oXShortcut5") && !stages[0]
        || matches!(portal.as_str(), "oXShortcut9") && !stages[1]
        || matches!(portal.as_str(), "oXShortcut13") && !stages[2]
    {
        return;
    }
    let destination = match portal.as_str() {
        "oXScores" => "scores",
        "oXTutorial" => "tutorial",
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
            game.sound("sndSteps", game.player.pos);
            return;
        }
        _ => "campaign",
    };
    game.flags.retain(|f| !f.starts_with("hub_destination:"));
    game.flags.insert(format!("hub_destination:{destination}"));
    game.sound("sndSteps", game.player.pos);
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
    if game.player.health <= 0 {
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
        } else {
            let id = game.world.spawn("oBomb", Vec2::new(x, 32.0));
            let e = game.world.entity_mut(id).unwrap();
            e.sprite = "sBombArmed".into();
            e.timer = 80;
            e.data.insert("armed".into(), 1.0);
            e.data.insert("safe_ticks".into(), 10.0);
        }
        let delay = 100 + game.rng.range(101);
        game.data.insert("hub_alarm0".into(), f64::from(delay));
    }
}
fn stars(game: &mut Game) {
    if game.player.health <= 0 {
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
    if !game.flags.contains("hub_challenge_finished")
        && (game.player.health <= 0
            || due(game, "hub_alarm11") && {
                let timer = value(game, "timer") - 1.0;
                game.data.insert("timer".into(), timer);
                game.data.insert("hub_alarm11".into(), 30.0);
                timer == 0.0
            })
    {
        record(game, "mini2", value(game, "baskets"));
        game.data.insert("timer".into(), -1.0);
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
        for e in &mut game.world.entities {
            if e.object == "oBatTarget" {
                e.alive = false;
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
            .find(|a| a.alive && a.object == "oArrow" && swept(a).intersects(target.bounds()))
            .map(|a| a.id);
        let stomp = game.player.bounds().intersects(target.bounds())
            && game.player.vel.y > 0.0
            && game.player.pos.y < target.pos.y + 8.0
            && (game.player.pos.x - target.pos.x - 8.0).abs() <= 12.0;
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
            game.sound("sndHit", target.pos);
            game.sound("sndCoin", target.pos);
        } else if game.player.bounds().intersects(target.bounds()) && game.player.invincible == 0 {
            game.damage_player(1);
            game.player.invincible = 30;
            game.player.vel.x = if game.player.pos.x < target.pos.x {
                -6.0
            } else {
                6.0
            };
        }
    }
}
fn swept(e: &Entity) -> Rect {
    let now = e.bounds();
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
            .filter(|a| a.alive && a.object == "oArrow" && swept(a).intersects(target.bounds()))
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
        assert!(record_unlocks(199999, Some(601.0), 119, 7)
            .iter()
            .all(|f| !f.starts_with("unlock_")));
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
        assert!(!record_unlocks(0, Some(960.0), 0, 0)
            .iter()
            .any(|f| f.starts_with("trophy_time_")));
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
        assert!(sun
            .entities
            .iter()
            .any(|e| e.object == "oTerrainMetadata" && e.value("invincible") == 1.0));
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
        assert!(g
            .world
            .entities
            .iter()
            .any(|e| e.alive && e.object == "oXScores"));
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
}
