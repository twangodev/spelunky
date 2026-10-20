//! Authored Classic title, records, and challenge rooms.
//! Room instances come from the original XML; source controllers use 30 Hz alarms.
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque};
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HubKind {
    Title,
    Scores,
    Sun,
    Moon,
    Stars,
}
/// Source oPlayer1 pExit values for authored start-door descendants.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HubPortal {
    Start,
    End,
    Scores,
    Tutorial,
    Title,
    Shortcut5,
    Shortcut9,
    Shortcut13,
    Sun,
    Moon,
    Stars,
    Change,
    Change2,
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
struct RoomTile {
    background: String,
    background_x: f64,
    background_y: f64,
    depth: f64,
    height: f64,
    width: f64,
    x: f64,
    y: f64,
}
#[derive(Deserialize)]
struct Room {
    width: usize,
    height: usize,
    instances: Vec<Instance>,
    #[serde(default)]
    tiles: Vec<RoomTile>,
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

pub(crate) fn authored_world(name: &str) -> World {
    let room = &rooms().rooms[name];
    let mut world = World::new(
        room.width.div_ceil(16),
        room.height.div_ceil(16),
        Area::Mines,
        0,
    );
    world.name = name.into();
    world.entrance = Vec2::new(600., 184.);
    for instance in &room.instances {
        let pos = Vec2::new(instance.x, instance.y);
        if instance.object == "oPlayer1" {
            world.entrance = pos;
            continue;
        }
        add_instance(
            &mut world,
            &instance.object,
            pos,
            matches!(name, "rSun" | "rMoon" | "rStars"),
        );
    }
    for tile in &room.tiles {
        world
            .add_generated_graphic_tile(GeneratedGraphicTileSpec {
                background: tile.background.clone(),
                source: [
                    tile.background_x,
                    tile.background_y,
                    tile.width,
                    tile.height,
                ],
                position: Vec2::new(tile.x, tile.y),
                depth: tile.depth,
                scale: Vec2::new(1.0, 1.0),
                color: 0x00ff_ffff,
                alpha: 1.0,
                visible: true,
            })
            .expect("authored room graphic tile handle is available");
    }
    world
}

/// Build authored geometry without consuming gameplay randomness.
pub fn world(kind: HubKind, unlocks: &BTreeSet<String>) -> World {
    let mut world = authored_world(kind.room());
    if kind == HubKind::Title {
        let title_start = unlocks
            .iter()
            .find_map(|flag| {
                flag.strip_prefix("title_start:")
                    .and_then(|value| value.parse::<u8>().ok())
            })
            .unwrap_or(2);
        // Start zero has no source Flare/player until the introduction alarms.
        if title_start != 0 {
            world.spawn("oFlare", Vec2::new(600.0, 188.0));
        }
        // Fresh storage alone enables the source onboarding hand and barriers.
        if unlocks.contains("first_time") {
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
            world.entity_mut(id).unwrap().set_sprite("sTunnelManRight");
        }
    }
    if kind == HubKind::Scores {
        for (flag, source_flag, y) in [
            ("new_money", "newMoney", 48.),
            ("new_kills", "newKills", 64.),
            ("new_saves", "newSaves", 80.),
            ("new_time", "newTime", 96.),
        ] {
            if unlocks.contains(flag) || unlocks.contains(source_flag) {
                let id = world.spawn("oNew", Vec2::new(272., y));
                world
                    .entity_mut(id)
                    .unwrap()
                    .data
                    .insert("visible".into(), 1.);
            }
        }
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
                world.entity_mut(id).unwrap().set_sprite(match rank {
                    3 => "sGoldTrophy",
                    2 => "sSilverTrophy",
                    _ => "sBronzeTrophy",
                });
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

/// Run room-instance Create callbacks in the order stored by the authored
/// room before SetupWalls and the first Step. `authored_world` represents
/// static instances with terrain tiles plus metadata entities, so the source
/// room list pairs those proxies back to each original room instance.
fn initialize_authored_room_creates(game: &mut Game, kind: HubKind) -> bool {
    let room = &rooms().rooms[kind.room()];
    let mut entities_by_instance: HashMap<(String, u64, u64), Vec<_>> = HashMap::new();
    let mut initial_entities = Vec::new();
    for entity in game
        .world
        .entities
        .iter()
        .filter(|entity| entity.alive && entity.object != "oTerrainMetadata")
    {
        initial_entities.push((entity.creation_serial, entity.id));
        entities_by_instance
            .entry((
                entity.object.clone(),
                entity.pos.x.to_bits(),
                entity.pos.y.to_bits(),
            ))
            .or_default()
            .push((entity.creation_serial, entity.id));
    }
    for instances in entities_by_instance.values_mut() {
        instances.sort_unstable();
    }
    let mut entities_by_instance: HashMap<_, VecDeque<_>> = entities_by_instance
        .into_iter()
        .map(|(key, instances)| (key, instances.into()))
        .collect();
    let mut metadata_by_origin: HashMap<(u64, u64), Vec<_>> = HashMap::new();
    let mut initial_metadata = Vec::new();
    for entity in game
        .world
        .entities
        .iter()
        .filter(|entity| entity.alive && entity.object == "oTerrainMetadata")
    {
        initial_metadata.push((entity.creation_serial, entity.id));
        metadata_by_origin
            .entry((
                (entity.pos.x - 8.0).to_bits(),
                (entity.pos.y - 8.0).to_bits(),
            ))
            .or_default()
            .push((entity.creation_serial, entity.id));
    }
    for instances in metadata_by_origin.values_mut() {
        instances.sort_unstable();
    }
    let mut metadata_by_origin: HashMap<_, VecDeque<_>> = metadata_by_origin
        .into_iter()
        .map(|(key, instances)| (key, instances.into()))
        .collect();

    let mut initialized_entities = HashSet::new();
    let mut initialized_metadata = HashSet::new();
    let mut authored_walls = false;
    for instance in &room.instances {
        if instance.object == "oPlayer1" {
            continue;
        }
        let key = (
            instance.object.clone(),
            instance.x.to_bits(),
            instance.y.to_bits(),
        );
        let entity_id = entities_by_instance
            .get_mut(&key)
            .and_then(VecDeque::pop_front)
            .map(|(_, id)| id);
        if let Some(id) = entity_id {
            initialize_authored_entity_create(game, id);
            initialized_entities.insert(id);
            if instance.object == "oTitle"
                && kind == HubKind::Title
                && game.generation_revision == GenerationRevision::SourceClassicV2
            {
                initialize_title_created_bricks(
                    game,
                    &mut metadata_by_origin,
                    &mut initialized_metadata,
                );
            }
            if instance.object == "oBricks"
                && game.generation_revision == GenerationRevision::SourceClassicV2
            {
                authored_walls = crate::generation::initialize_authored_setup_walls(
                    &mut game.world,
                    &mut game.rng,
                    game.generation_revision,
                    !game.flags.contains("low_graphics"),
                    false,
                    false,
                );
            }
            if instance.object == "oSunRoom"
                && game.generation_revision == GenerationRevision::SourceClassicV2
            {
                game.data
                    .insert("hub_alarm0".into(), f64::from(200 + game.rng.range(101)));
            }
            continue;
        }

        let metadata_id = metadata_by_origin
            .get_mut(&(instance.x.to_bits(), instance.y.to_bits()))
            .and_then(VecDeque::pop_front)
            .map(|(_, id)| id);
        if let Some(metadata_id) = metadata_id {
            initialize_authored_tile_create(game, &instance.object, metadata_id);
            initialized_metadata.insert(metadata_id);
        }
        let x = (instance.x / 16.0) as usize;
        let y = (instance.y / 16.0) as usize;
        if x < game.world.width
            && y < game.world.height
            && game
                .world
                .tile(x as i32, y as i32)
                .is_some_and(|tile| tile.object == instance.object)
        {
            crate::traps::create(
                game,
                crate::dispatcher::Instance::Terrain(y * game.world.width + x),
            );
        }
    }

    // Runtime unlocks and first-time title additions are appended after the
    // authored room's Create sequence. They still need their own Create state.
    let mut remaining_metadata: Vec<_> = initial_metadata
        .into_iter()
        .filter(|(_, id)| !initialized_metadata.contains(id))
        .collect();
    remaining_metadata.sort_unstable();
    for (_, id) in remaining_metadata {
        let Some(entity) = game.world.entity(id) else {
            continue;
        };
        let x = ((entity.pos.x - 8.0) / 16.0) as i32;
        let y = ((entity.pos.y - 8.0) / 16.0) as i32;
        if let Some(object) = game.world.tile(x, y).map(|tile| tile.object.clone()) {
            initialize_authored_tile_create(game, &object, id);
        }
    }
    let mut remaining_entities: Vec<_> = initial_entities
        .into_iter()
        .filter(|(_, id)| !initialized_entities.contains(id))
        .collect();
    remaining_entities.sort_unstable();
    for (_, id) in remaining_entities {
        initialize_authored_entity_create(game, id);
    }
    authored_walls
}

/// `oTitle.Create` creates these barrier bricks synchronously before the
/// authored `oBricks` instance runs SetupWalls. They are represented as
/// appended tile metadata in `hub::world`, so initialize them at the title
/// controller's source-order slot rather than after the room loop.
fn initialize_title_created_bricks(
    game: &mut Game,
    metadata_by_origin: &mut HashMap<(u64, u64), VecDeque<(u64, u64)>>,
    initialized_metadata: &mut HashSet<u64>,
) {
    let [shortcut_stage1, _, _] = shortcut_stages(&game.flags);
    let mut origins: Vec<(f64, f64)> = Vec::new();
    if !shortcut_stage1 {
        origins.extend([(320.0, 96.0), (336.0, 96.0)]);
    }
    if game.flags.contains("first_time") {
        origins.extend([(320.0, 160.0), (336.0, 160.0), (336.0, 176.0)]);
    }
    for (x, y) in origins {
        let key = (x.to_bits(), y.to_bits());
        let Some((_, metadata_id)) = metadata_by_origin
            .get_mut(&key)
            .and_then(VecDeque::pop_front)
        else {
            continue;
        };
        if !game
            .world
            .tile((x / 16.0) as i32, (y / 16.0) as i32)
            .is_some_and(|tile| tile.object == "oBrick")
        {
            continue;
        }
        initialize_authored_tile_create(game, "oBrick", metadata_id);
        initialized_metadata.insert(metadata_id);
    }
}

fn initialize_authored_entity_create(game: &mut Game, id: u64) {
    crate::enemies::create_instance(game, id);
    crate::items::create_instance(game, id);
    crate::world_events::create_instance(game, id);
    crate::traps::create(game, crate::dispatcher::Instance::Entity(id));
}

fn initialize_authored_tile_create(game: &mut Game, object: &str, metadata_id: u64) {
    match object {
        "oBrick" => {
            let chipped = game.rng.range(10) == 0;
            let roll = 1 + game.rng.range(100);
            if let Some(metadata) = game.world.entity_mut(metadata_id) {
                metadata.data.insert("n".into(), f64::from(roll));
                if metadata
                    .text
                    .get("terrain_sprite")
                    .is_some_and(|sprite| sprite != "sBrick")
                {
                    return;
                }
                let (sprite, grade) = if roll < 20 {
                    (Some("sBrickGold"), 1)
                } else if roll < 30 {
                    (Some("sBrickGoldBig"), 2)
                } else if chipped {
                    (Some("sBrick2"), 0)
                } else {
                    (None, 0)
                };
                if let Some(sprite) = sprite {
                    metadata.text.insert("terrain_sprite".into(), sprite.into());
                }
                if grade > 0 {
                    metadata.data.insert("gold_chunks".into(), 3.0);
                    metadata
                        .data
                        .insert("gold_nuggets".into(), if grade == 2 { 1.0 } else { 0.0 });
                }
            }
        }
        "oDark" => {
            let roll = 1 + game.rng.range(100);
            let in_intro = game.world.name == "rIntro";
            if let Some(metadata) = game.world.entity_mut(metadata_id) {
                metadata.data.insert("n".into(), f64::from(roll));
                if in_intro {
                    return;
                }
                let (sprite, grade) = if roll < 20 {
                    (Some("sDarkGold"), 1)
                } else if roll < 30 {
                    (Some("sDarkGoldBig"), 2)
                } else {
                    (None, 0)
                };
                if let Some(sprite) = sprite {
                    metadata.text.insert("terrain_sprite".into(), sprite.into());
                }
                if grade > 0 {
                    metadata.data.insert("gold_chunks".into(), 3.0);
                    metadata
                        .data
                        .insert("gold_nuggets".into(), if grade == 2 { 1.0 } else { 0.0 });
                }
            }
        }
        "oLava" => crate::generation::initialize_authored_lava_create(
            &mut game.world,
            &mut game.rng,
            metadata_id,
        ),
        _ => {}
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
    if game.player.health <= 0.0 || reset_tutorial_run || reset_dead_run {
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
        if reset_dead_run {
            // Player Step calls scrClearGlobals before routing an acknowledged
            // death to rHighscores. Mirror that whole run reset here as the
            // host's room-transition boundary, after the hub's world exists.
            game.clear_run_globals();
        }
    }
    if reset_tutorial_run {
        // The tutorial's manual xTITLE return calls scrClearGlobals before
        // setting these room-specific values. Match that full run reset here
        // because load_hub is where the destination room is assembled.
        game.clear_run_globals();
        game.flags.retain(|flag| !flag.starts_with("title_start:"));
        game.flags.insert("title_start:3".into());
        game.data.insert("message_timer".into(), 0.0);
        for key in ["mini1", "mini2", "mini3"] {
            game.data.insert(key.into(), 0.0);
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
        for flag in [
            "new_money",
            "new_time",
            "new_kills",
            "new_saves",
            "newMoney",
            "newTime",
            "newKills",
            "newSaves",
        ] {
            game.flags.remove(flag);
        }
    }
    if kind == HubKind::Scores {
        for flag in ["dark", "darkLevel", "snake_pit", "snakePit"] {
            game.flags.remove(flag);
        }
        // oHighscores Create resets these globals on every entry, including
        // a return from a challenge or a completed campaign. In particular,
        // stale campaign levels must not survive into a score-room session.
        game.data.insert("currLevel".into(), 1.0);
        game.data.insert("message_timer".into(), 0.0);
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
    let authored_walls = if game.generation_revision == GenerationRevision::SourceClassicV2 {
        initialize_authored_room_creates(game, kind)
    } else {
        initialize_brick_variants(game);
        crate::generation::initialize_authored_setup_walls(
            &mut game.world,
            &mut game.rng,
            game.generation_revision,
            !game.flags.contains("low_graphics"),
            false,
            false,
        )
    };
    if !authored_walls {
        if !game.flags.contains("low_graphics") {
            crate::generation::initialize_hub_fringe(&mut game.world, &mut game.rng);
        }
        crate::generation::initialize_hub_canopy(&mut game.world, game.generation_revision);
    }
    game.data.insert("hub_draw_status".into(), 0.0);
    if kind.challenge() {
        game.data.insert("source_hub_music_alarm".into(), 100.0);
        game.data.insert("challenge_highscore".into(), 0.);
        game.player.health = 8.0;
        game.data.insert("points".into(), 0.0);
        game.data.insert("baskets".into(), 0.0);
        game.data.insert("stars_kills".into(), 0.0);
        let hazard = if kind == HubKind::Sun {
            game.data
                .get("hub_alarm0")
                .copied()
                .unwrap_or_else(|| f64::from(200 + game.rng.range(101)))
        } else {
            100.0
        };
        game.data.insert("hub_alarm0".into(), hazard);
        game.data.insert(
            "hub_alarm11".into(),
            if kind == HubKind::Stars { 110.0 } else { 100.0 },
        );
        if kind == HubKind::Moon {
            game.data.insert("timer".into(), 60.0);
            game.data.insert("arrows".into(), 100.0);
            game.data.insert("hub_alarm11".into(), -1.0);
            game.data.insert("hub_alarm10".into(), -1.0);
            game.data.insert("hub_alarm1".into(), 30.0);
            game.data.insert("hub_alarm2".into(), -1.0);
            game.data.insert("hub_alarm3".into(), -1.0);
        }
    }
    // Selecting a source title spawn bypasses the purely cinematic first entrance.
    if kind == HubKind::Title {
        game.camera = Vec2::new(320.0, 0.0);
        game.previous_camera = game.camera;
        crate::opening::begin_title(game, title_start.unwrap_or(2));
    }
    if game.generation_revision == GenerationRevision::SourceClassicV2 {
        // Authored room entities enter through World::spawn rather than
        // Game::spawn, so apply the inherited oItem Create depth after the
        // room's own setup while retaining any explicit producer override.
        let has_spectacles = game.player.has("Spectacles");
        for entity in &mut game.world.entities {
            crate::items::initialize_inherited_depth(entity, has_spectacles);
        }
    }
}
fn value(game: &Game, key: &str) -> f64 {
    game.data.get(key).copied().unwrap_or(0.0)
}
fn record(game: &mut Game, key: &str, score: f64) {
    let score = score.clamp(0.0, 99.0);
    let best = value(game, &format!("best_{key}"));
    game.data
        .insert("challenge_highscore".into(), f64::from(score > best));
    game.data.insert(key.into(), score);
    game.flags.insert("hub_challenge_finished".into());
    game.events.push(GameEvent::Message(format!(
        "Challenge score: {}",
        score as u32
    )));
}

/// oPlayer1's start-door branch, before inventory actions and hurt queries.
pub(crate) fn player_portal_actions(game: &mut Game, input: &InputFrame) {
    if (game.flags.contains("hub") || game.flags.contains("tutorial"))
        && (input.up || input.exit_pressed)
    {
        activate_portal(game);
    }
}

pub fn tick(game: &mut Game, input: &InputFrame) {
    if !game.flags.contains("hub") {
        // The authored tutorial also contains an oXStart-derived return door.
        // It uses the same portal input branch as the title and records rooms.
        if !game.dispatcher.running {
            player_portal_actions(game, input);
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
    if kind != HubKind::Moon || !game.dispatcher.running {
        game.data.insert("hub_draw_status".into(), draw);
    }
    match kind {
        HubKind::Sun => sun(game),
        HubKind::Moon => moon(game),
        HubKind::Stars => stars(game),
        _ => {}
    }
    if kind == HubKind::Scores {
        scores_step(game);
    }
    if !game.dispatcher.running {
        basketball(game);
    }
    if !game.dispatcher.running {
        static_targets(game);
    }
    if !game.dispatcher.running {
        player_portal_actions(game, input);
    }
}

fn clear_highscores(game: &mut Game) {
    game.flags.retain(|flag| {
        !flag.starts_with("trophy_")
            && !matches!(
                flag.as_str(),
                "unlock_sun"
                    | "unlock_moon"
                    | "unlock_stars"
                    | "unlock_damsel"
                    | "unlock_tunnel_man"
                    | "shortcut_5"
                    | "shortcut_9"
                    | "shortcut_13"
            )
    });
    for key in [
        "best_money",
        "best_time",
        "best_kills",
        "best_saves",
        "best_mini1",
        "best_mini2",
        "best_mini3",
        "total_plays",
        "total_wins",
        "total_deaths",
    ] {
        game.data.insert(key.into(), 0.);
    }
    game.data.insert("tunnel1_remaining".into(), 100001.);
    game.data.insert("tunnel2_remaining".into(), 200001.);
    game.data.remove("tunnel3_remaining");
    game.flags.insert("highscores_clear_requested".into());
    game.data.insert("shake".into(), 60.);
}
fn scores_step(game: &mut Game) {
    let ids: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oButtonHighscore")
        .map(|e| e.id)
        .collect();
    for id in ids {
        let mut button = game.world.entity(id).unwrap().clone();
        // Source collision_rectangle(x+2,y+11,x+13,y+15,oSolid,0,0)
        // includes both endpoint pixels and queries active family members.
        let pressed = game.world.rectangle_family_except(
            Vec2::new(button.pos.x + 2.0, button.pos.y + 11.0),
            Vec2::new(button.pos.x + 13.0, button.pos.y + 15.0),
            "oSolid",
            None,
            CollisionPrecision::BoundingBox,
        );
        if pressed && button.value("pushed") == 0. {
            button.data.insert("counter".into(), 20.);
            game.sound("sndClick", button.pos);
        }
        button.data.insert("pushed".into(), f64::from(pressed));
        if pressed {
            let counter = button.value("counter");
            if counter > 0. {
                button.data.insert("counter".into(), counter - 1.);
            }
            if button.value("counter") == 1. {
                clear_highscores(game);
                game.sound("sndThump", button.pos);
            }
            for e in &mut game.world.entities {
                if matches!(
                    e.object.as_str(),
                    "oTrophy" | "oXSun" | "oXMoon" | "oXStars" | "oXChange"
                ) {
                    e.alive = false;
                }
            }
            button.set_sprite("sButtonPushed");
            for key in [
                "hub_record_money",
                "hub_record_time",
                "hub_record_kills",
                "hub_record_saves",
            ] {
                game.data.insert(key.into(), 0.);
            }
            game.flags.insert("hub_records_erased".into());
        } else {
            button.set_sprite("sButton");
        }
        *game.world.entity_mut(id).unwrap() = button;
    }
    // Source Draw mutates this visibility latch only in these two branches.
    // Keeping it authoritative makes arbitrary presentation rates read-only.
    let visible = if game.player.pos.y >= 156. {
        Some(1.)
    } else if game.player.pos.y >= 128. {
        Some(0.)
    } else {
        None
    };
    if let Some(visible) = visible {
        for e in &mut game.world.entities {
            if e.object == "oNew" {
                e.data.insert("visible".into(), visible);
            }
        }
    }
}

/// The player's Title Step replaces the climbing controller before input can
/// release the rope. The dummy then ascends and fades into the exit credits.
pub(crate) fn begin_title_exit(game: &mut Game) -> bool {
    if !game.flags.contains("hub_title")
        || !crate::player::source_climbing(game)
        || game.player.pos.y >= 32.
        || game.state != RunState::Playing
    {
        return false;
    }
    if let Some(id) = game.player.held.take() {
        game.player.pickup_item_type.clear();
        if let Some(item) = game.world.entity_mut(id) {
            item.held = false;
        }
    }
    crate::ending::begin_title_credits(game);
    true
}

fn activate_portal(game: &mut Game) {
    if !crate::player::source_platform_grounded(game)
        || game.player.health <= 0.0
        || game.player.state == PlayerState::Dead
        || game.state == RunState::Dead
        || game.player.is_stunned()
        || game.player.attack_timer > 0
        || game.player.is_exiting()
        || game.state != RunState::Playing
    {
        return;
    }

    // Player Step first gates on collision_point(x, y, oXStart), then resolves
    // `instance_place(x, y, oXStart)` separately. Both source queries see only
    // active members and preserve the registered family order; the latter
    // uses the Player's full sprite mask rather than its origin point.
    crate::dispatcher::register_instances(game);
    let doors: Vec<_> = crate::dispatcher::descendant_instances(game, "oXStart")
        .into_iter()
        .filter_map(|instance| match instance {
            crate::dispatcher::Instance::Entity(id) => game.world.entity(id),
            _ => None,
        })
        .filter(|door| door.alive && door.engine_active)
        .cloned()
        .collect();
    if !doors
        .iter()
        .any(|door| door.mask_contains(game.player.pos, crate::CollisionPrecision::BoundingBox))
    {
        return;
    }
    let player_mask = game.player.collision_mask();
    let receiver = doors
        .iter()
        .find(|door| player_mask.intersects_mask(&door.collision_mask()));
    if let Some(receiver) = receiver {
        game.player.pos.x = receiver.pos.x + 8.0;
    }

    // pExit is resolved after the receiver snap, using the source's explicit
    // subtype precedence rather than assuming it is the same instance that
    // `instance_place` selected.
    let portal = [
        ("oXScores", HubPortal::Scores),
        ("oXTutorial", HubPortal::Tutorial),
        ("oXTitle", HubPortal::Title),
        ("oXEnd", HubPortal::End),
        ("oXShortcut5", HubPortal::Shortcut5),
        ("oXShortcut9", HubPortal::Shortcut9),
        ("oXShortcut13", HubPortal::Shortcut13),
        ("oXSun", HubPortal::Sun),
        ("oXMoon", HubPortal::Moon),
        ("oXStars", HubPortal::Stars),
        ("oXChange", HubPortal::Change),
        ("oXChange2", HubPortal::Change2),
    ]
    .into_iter()
    .find_map(|(object, portal)| {
        doors
            .iter()
            .any(|door| {
                door.object == object
                    && door.mask_contains(game.player.pos, crate::CollisionPrecision::BoundingBox)
            })
            .then_some(portal)
    })
    .unwrap_or(HubPortal::Start);
    let stages = shortcut_stages(&game.flags);
    // The level-five shortcut is authored into rTitle and the source player
    // accepts it unconditionally. Only the later doors appear after their
    // tunnel donations; those rooms are already guarded by their creation
    // conditions, with this check protecting stray/custom instances too.
    if matches!(portal, HubPortal::Shortcut9) && !stages[1]
        || matches!(portal, HubPortal::Shortcut13) && !stages[2]
    {
        return;
    }
    if let Some(id) = game.player.held.take() {
        if let Some(item) = game.world.entity_mut(id) {
            item.held = false;
        }
    }
    game.player.pickup_item_type.clear();
    crate::ending::begin_hub_exit(game, portal);
    if portal != HubPortal::Change2 {
        crate::music::stop_all(game);
    }
    game.sound("sndSteps", game.player.pos);
}

/// Animation End commits the source pExit branch after common collect payout.
pub fn complete_portal(game: &mut Game, portal: HubPortal) {
    if matches!(portal, HubPortal::Change | HubPortal::Change2) {
        let (selected, other) = if portal == HubPortal::Change {
            ("character_damsel", "character_tunnelman")
        } else {
            ("character_tunnelman", "character_damsel")
        };
        if !game.flags.remove(selected) {
            game.flags.remove(other);
            game.flags.insert(selected.into());
        }
        let character = crate::characters::selected(game);
        let image_index = game.player.animation;
        crate::characters::apply(game, character);
        // Source character switches mutate inventory, not image_index. Its
        // automatic animation pass has already wrapped the Exit index.
        game.player.animation = image_index;
        crate::player::assign_platform(game, PlayerState::Standing);
        game.player.vel.x = 0.;
        game.player.invincible = 0;
        game.player.data.insert("active".into(), 1.);
        game.player.data.insert("depth".into(), 50.);
        game.player.facing = if portal == HubPortal::Change { -1 } else { 1 };
        game.player.set_sprite(match character {
            crate::characters::Character::Damsel => "sDamselLeft",
            crate::characters::Character::TunnelMan => "sTunnelLeft",
            crate::characters::Character::Explorer => "sStandLeft",
        });
        game.state = RunState::Playing;
        return;
    }
    let destination = match portal {
        HubPortal::Scores => "scores",
        HubPortal::End => unreachable!("xEND enters the campaign chamber in Animation End"),
        HubPortal::Tutorial => "tutorial",
        HubPortal::Start if game.flags.remove("first_time") => {
            game.flags.insert("onboarding_consumed".into());
            "tutorial"
        }
        HubPortal::Title => "title",
        HubPortal::Shortcut5 => "shortcut5",
        HubPortal::Shortcut9 => "shortcut9",
        HubPortal::Shortcut13 => "shortcut13",
        HubPortal::Sun => "sun",
        HubPortal::Moon => "moon",
        HubPortal::Stars => "stars",
        _ => "campaign",
    };
    if destination == "tutorial" {
        // Both xSTART's first-use branch and xTUTORIAL use these inventory
        // values. Only the former consumes the firstTime sentinel.
        game.world.level = 0;
        game.player.bombs = 0.0;
        game.player.ropes = 2.0;
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
    game.state = RunState::Transition;
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
/// Source controller resources keep their persisted alarms in Game::data.
/// The dispatcher visits them in its existing alarm-number/resource passes.
pub(crate) fn challenge_alarm_resource(game: &Game) -> Option<i32> {
    if !game.flags.contains("hub") {
        return None;
    }
    if game.flags.contains("hub_sun") {
        Some(721)
    } else if game.flags.contains("hub_moon") {
        Some(715)
    } else if game.flags.contains("hub_stars") {
        Some(701)
    } else {
        None
    }
}

pub(crate) fn challenge_alarm(game: &mut Game, index: u8) {
    let Some(resource) = challenge_alarm_resource(game) else {
        return;
    };
    let music_index = if resource == 715 { 9 } else { 10 };
    if index == music_index {
        let timer = game
            .data
            .entry("source_hub_music_alarm".into())
            .or_insert(-1.0);
        if *timer > 0.0 {
            *timer -= 1.0;
            if *timer == 0.0 {
                crate::music::start_music(game);
            }
        }
    }
    let key = match index {
        0 => "hub_alarm0",
        1 if resource == 715 => "hub_alarm1",
        2 if resource == 715 => "hub_alarm2",
        3 if resource == 715 => "hub_alarm3",
        10 if resource == 715 => "hub_alarm10",
        11 => "hub_alarm11",
        _ => return,
    };
    let timer = game.data.entry(key.into()).or_insert(-1.0);
    if *timer < 0.0 {
        return;
    }
    *timer -= 1.0;
    if *timer != 0.0 {
        return;
    }
    // oMoonRoom has no dead guard in these Alarm callbacks. Alarm 0 can
    // arm 11 later in this same alarm-number pass; 11 schedules 10 next frame.
    if resource == 715 {
        match index {
            0 if value(game, "timer") == 60.0 => {
                game.data.insert("hub_alarm11".into(), 1.0);
            }
            1 => {
                game.data.insert("hub_draw_status".into(), 1.0);
                game.data.insert("hub_alarm2".into(), 30.0);
            }
            2 => {
                game.data.insert("hub_draw_status".into(), 2.0);
                game.data.insert("hub_alarm3".into(), 10.0);
            }
            3 => {
                game.data.insert("hub_draw_status".into(), 3.0);
                for (x, y) in [(160.0, -16.0), (208.0, -64.0), (256.0, -128.0)] {
                    spawn_target(game, x, y);
                }
            }
            10 if value(game, "timer") < 0.0 => {
                game.game_over.draw_status = game.game_over.draw_status.max(3);
                game.flags.insert("source_music_stopped".into());
                crate::music::stop_all(game);
            }
            11 => moon_alarm11(game),
            _ => {}
        }
        return;
    }
    // oStarsRoom Alarm 11 evaluates rand(1,100) even when dead. Keep
    // historical snapshots' skipped draw; alive callbacks still roll below.
    if resource == 701
        && index == 11
        && game.generation_revision == GenerationRevision::SourceClassicV2
        && crate::items::player_dead(game)
    {
        game.rng.range(100);
    }
    if crate::items::player_dead(game) {
        return;
    }
    // Source tests dead, not global life: Player Step can observe fatal life
    // later in this same frame, after every Alarm callback has completed.
    match (resource, index) {
        (721, 0) => sun_alarm0(game),
        (721, 11) => {
            let score = (value(game, "points") + 1.0).min(99.0);
            game.data.insert("points".into(), score);
            game.data.insert("hub_alarm11".into(), 30.0);
        }
        (701, 0) => stars_alarm0(game),
        (701, 11) => {
            if f64::from(1 + game.rng.range(100)) < value(game, "stars_kills") {
                let x = 32.0 + 80.0 * f64::from(game.rng.range(4));
                game.spawn("oSpider", Vec2::new(x, 0.0));
            }
            game.data.insert("hub_alarm11".into(), 20.0);
        }
        _ => {}
    }
}

fn sun(game: &mut Game) {
    if value(game, "hub_draw_status") == 0.0
        && game
            .world
            .entities
            .iter()
            .all(|e| e.object != "oDamsel" || !e.alive || e.health < 1)
    {
        game.player.health = 0.0;
    }
    // oSunRoom Step only sets global.plife to zero when the intro damsel is
    // missing. The later oPlayer1 Step observes that value and performs death
    // or Ankh revival; the challenge score is recorded from that Player event.
    // Recording on health alone here would finish the challenge one tick early
    // and incorrectly keep it finished if the Player revives.
    if game.player.state == PlayerState::Dead || game.state == RunState::Dead {
        if !game.flags.contains("hub_challenge_finished") {
            record(game, "mini1", value(game, "points"));
        }
    }
}
fn sun_alarm0(game: &mut Game) {
    let x = game.player.pos.x.clamp(40.0, 280.0);
    if game
        .world
        .solid_point_active(Vec2::new(x, 32.0), CollisionPrecision::BoundingBox, None)
    {
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
            game.spawn(
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
        let id = game.spawn("oVampire", Vec2::new(152.0, 32.0));
        game.world.entity_mut(id).unwrap().state = 7;
        poof_pair(game, Vec2::new(160.0, 40.0), true);
    } else {
        let id = game.spawn("oBomb", Vec2::new(x, 32.0));
        let e = game.world.entity_mut(id).unwrap();
        e.set_sprite("sBombArmed");
        e.timer = 80;
        e.alarms[0] = 80;
        e.alarms[2] = 10;
        e.set_image_speed(0.2);
        e.data.insert("armed".into(), 1.0);
        poof_pair(game, Vec2::new(x, 32.0), true);
    }
    let delay = 100 + game.rng.range(101);
    game.data.insert("hub_alarm0".into(), f64::from(delay));
}
fn stars(game: &mut Game) {
    if game.player.health <= 0.0
        || game.player.state == PlayerState::Dead
        || game.state == RunState::Dead
    {
        if !game.flags.contains("hub_challenge_finished") {
            record(game, "mini3", value(game, "stars_kills"));
        }
    }
}
fn stars_alarm0(game: &mut Game) {
    let kills = value(game, "stars_kills");
    let x = 32.0 + 80.0 * f64::from(game.rng.range(4));
    let id = game.spawn("oShopkeeper2", Vec2::new(x, 0.0));
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
fn spawn_target(game: &mut Game, x: f64, y: f64) {
    game.spawn("oBatTarget", Vec2::new(x, y));
}

/// Root object: this Create has no Item/Enemy inheritance and no RNG draws.
pub(crate) fn create_bat_target(game: &mut Game, id: u64) {
    let ufo = value(game, "timer") <= 20.0;
    let Some(e) = game.world.entity_mut(id) else {
        return;
    };
    if e.value("bat_target_create_initialized") != 0.0 {
        return;
    }
    let difficulty = if e.pos.x == 208.0 {
        2.0
    } else if e.pos.x == 256.0 {
        4.0
    } else {
        1.0
    };
    for (name, field) in [
        // Explicit builtin default bypasses Scene's directional-name fallback;
        // this parentless object's source Create never flips the sprite.
        ("image_xscale", 1.0),
        ("bat_target_create_initialized", 1.0),
        ("bloodless", 0.0),
        ("UP", 0.0),
        ("DOWN", 1.0),
        ("dir", 1.0),
        ("difficulty", difficulty + if ufo { 2.0 } else { 0.0 }),
        ("xMid", e.pos.x),
        ("xDiff", 0.0),
    ] {
        e.data.insert(name.into(), field);
    }
    if ufo {
        e.set_sprite("sUFO");
    }
}

pub(crate) fn bat_target_step(game: &mut Game, id: u64) {
    let timer = value(game, "timer");
    let Some(e) = game.world.entity_mut(id) else {
        return;
    };
    if e.value("dir") == e.value("UP") {
        e.pos.y -= e.value("difficulty");
        if e.pos.y <= 64.0 {
            e.data.insert("dir".into(), e.value("DOWN"));
        }
    } else if e.value("dir") == e.value("DOWN") {
        e.pos.y += e.value("difficulty");
        if e.pos.y >= 160.0 {
            e.data.insert("dir".into(), e.value("UP"));
        }
    }
    if timer <= 20.0 && e.pos.y > 64.0 {
        e.pos.x = e.value("xMid") - (e.value("xDiff").sin() * 32.0).abs();
        e.data.insert("xDiff".into(), e.value("xDiff") + 0.01);
    }
}

/// Source Collision supplies current precise mask overlap and active membership.
pub(crate) fn bat_target_collision(game: &mut Game, id: u64, other: crate::dispatcher::Instance) {
    let Some(target) = game.world.entity(id).cloned() else {
        return;
    };
    match other {
        crate::dispatcher::Instance::Entity(arrow)
            if crate::catalog::is_a(other.object(game), "oArrow") =>
        {
            game.sound("sndHit", target.pos);
            game.sound("sndCoin", target.pos);
            *game.data.entry("baskets".into()).or_default() += target.value("difficulty");
            // Inherited Item Destroy uses held alone; pickupItemType survives.
            if game.world.entity(arrow).is_some_and(|e| e.held) {
                game.player.held = None;
            }
            if let Some(e) = game.world.entity_mut(arrow) {
                e.alive = false;
            }
            spawn_target(game, target.value("xMid"), -64.0);
            if target.sprite == "sUFO" {
                game.spawn("oUFOCrash", target.pos);
            } else {
                crate::enemies::create_blood(
                    game,
                    &target,
                    Vec2::new(target.pos.x + 8.0, target.pos.y + 8.0),
                    3,
                );
            }
            game.world.entity_mut(id).unwrap().alive = false;
        }
        // oPlayer1 is the source corpus's sole oCharacter descendant.
        crate::dispatcher::Instance::Player => {
            if (game.player.pos.x - (target.pos.x + 8.0)).abs() > 12.0 {
                return;
            }
            let stomp = !crate::items::player_dead(game)
                && crate::player::source_airborne(game)
                && game.player.pos.y < target.pos.y + 8.0
                && game.player.data.get("swimming").copied().unwrap_or(0.0) == 0.0;
            if stomp {
                game.player.vel.y = -6.0 - 0.2 * game.player.vel.y;
                game.sound("sndHit", target.pos);
                game.sound("sndCoin", target.pos);
                *game.data.entry("baskets".into()).or_default() += target.value("difficulty");
                spawn_target(game, target.pos.x, -64.0);
                crate::enemies::create_blood(
                    game,
                    &target,
                    Vec2::new(target.pos.x + 8.0, target.pos.y + 8.0),
                    3,
                );
                game.world.entity_mut(id).unwrap().alive = false;
            } else if game.player.invincible == 0 {
                game.player.data.insert("blink".into(), 30.0);
                game.player.invincible = 30;
                game.player.vel.x = if game.player.pos.x < target.pos.x {
                    -6.0
                } else {
                    6.0
                };
                if game.player.health > 0.0 {
                    game.player.lose_health(1);
                    game.events.push(GameEvent::Damage { amount: 1 });
                }
                crate::enemies::create_blood(
                    game,
                    &target,
                    Vec2::new(target.pos.x + 4.0, target.pos.y + 4.0),
                    1,
                );
                game.sound("sndHurt", game.player.pos);
            }
        }
        _ => {}
    }
}
// Literal oMoonRoom Alarm 11: record/reveal, targets in creation order,
// then entrance replacement. New instances run Create before producer writes.
fn moon_alarm11(game: &mut Game) {
    let timer = value(game, "timer") - 1.0;
    game.data.insert("timer".into(), timer);
    if timer != 0.0 {
        game.data.insert("hub_alarm11".into(), 30.0);
        return;
    }
    record(game, "mini2", value(game, "baskets"));
    game.data.insert("timer".into(), -1.0);
    game.game_over.draw_status = 1;
    let mut targets: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && e.engine_active && e.object == "oBatTarget")
        .map(|e| (e.creation_serial, e.id, e.pos))
        .collect();
    targets.sort_by_key(|(serial, _, _)| *serial);
    for (_, id, pos) in targets {
        for y_vel in [-1.0, 1.0] {
            let poof = game.spawn("oPoof", Vec2::new(pos.x + 8.0, pos.y));
            game.world.entity_mut(poof).unwrap().vel = Vec2::new(0.0, y_vel);
        }
        game.world.entity_mut(id).unwrap().alive = false;
    }
    let mut doors: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oEntrance")
        .map(|e| (e.creation_serial, e.id, e.pos))
        .collect();
    doors.sort_by_key(|(serial, _, _)| *serial);
    for (_, id, pos) in doors {
        game.spawn("oXScores", pos);
        game.world.entity_mut(id).unwrap().alive = false;
    }
    game.data.insert("hub_alarm10".into(), 30.0);
}

fn moon(game: &mut Game) {
    if !game.dispatcher.running && game.level_tick == 70 {
        for (x, y) in [(160.0, -16.0), (208.0, -64.0), (256.0, -128.0)] {
            spawn_target(game, x, y);
        }
    }
    let died = game.player.health <= 0.0
        || game.player.state == PlayerState::Dead
        || game.state == RunState::Dead;
    // Player's first fatal Step records Moon and schedules its own 30-tick
    // reveal, but does not run the living-timeout target/entrance replacement.
    if died && value(game, "moon_death_recorded") == 0.0 {
        record(game, "mini2", value(game, "baskets"));
        game.data.insert("timer".into(), -1.0);
        game.data.insert("hub_alarm10".into(), 30.0);
        game.data.insert("moon_death_recorded".into(), 1.0);
    }
    // Live BatTarget Steps/Collisions are source-dispatched; preserve the
    // direct nonrunning adapter below for historical subsystem fixtures only.
    if game.dispatcher.running {
        return;
    }
    // Score/reveal bookkeeping does not disable the targets' independent
    // Step and collision callbacks. Timeout Alarm 11 destroys them explicitly.
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
        } else if e.value("dir") == 1.0 {
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
            && game.player.state != PlayerState::Dead
            && game.state != RunState::Dead
            && crate::player::source_airborne(game)
            && game.player.data.get("swimming").copied().unwrap_or(0.0) == 0.0
            && game.player.pos.y < target.pos.y + 8.0;
        if let Some(arrow) = arrow {
            // oBatTarget Arrow collision: sounds, credit, other Destroy,
            // replacement Create, crash/blood, then this target's Destroy.
            game.sound("sndHit", target.pos);
            game.sound("sndCoin", target.pos);
            *game.data.entry("baskets".into()).or_default() += speed;
            if game.world.entity(arrow).unwrap().held {
                // Arrow inherits oItem Destroy; the held flag alone controls
                // its synchronous write, independently of the hand backlink.
                game.player.held = None;
            }
            game.world.entity_mut(arrow).unwrap().alive = false;
            spawn_target(game, target.value("xMid"), -64.0);
            if target.sprite == "sUFO" {
                game.spawn("oUFOCrash", target.pos);
            } else {
                crate::enemies::create_blood(
                    game,
                    &target,
                    Vec2::new(target.pos.x + 8.0, target.pos.y + 8.0),
                    3,
                );
            }
            game.world.entity_mut(id).unwrap().alive = false;
        } else if stomp {
            // The Character stomp has its own source ordering and uses the
            // target's current X, including its timer-driven oscillation.
            game.player.vel.y = -6.0 - 0.2 * game.player.vel.y;
            game.sound("sndHit", target.pos);
            game.sound("sndCoin", target.pos);
            *game.data.entry("baskets".into()).or_default() += speed;
            spawn_target(game, target.pos.x, -64.0);
            crate::enemies::create_blood(
                game,
                &target,
                Vec2::new(target.pos.x + 8.0, target.pos.y + 8.0),
                3,
            );
            game.world.entity_mut(id).unwrap().alive = false;
        } else if touching && game.player.invincible == 0 {
            // Source's final Character branch tests invincibility alone;
            // dead characters retain this callback, with life clamped at zero.
            game.player.data.insert("blink".into(), 30.);
            game.player.invincible = 30;
            game.player.vel.x = if game.player.pos.x < target.pos.x {
                -6.0
            } else {
                6.0
            };
            if game.player.health > 0.0 {
                game.player.lose_health(1);
                game.events.push(GameEvent::Damage { amount: 1 });
            }
            crate::enemies::create_blood(
                game,
                &target,
                Vec2::new(target.pos.x + 4.0, target.pos.y + 4.0),
                1,
            );
            game.sound("sndHurt", game.player.pos);
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
            && (ball.vel.y > 0.0 || ball.held && crate::player::source_falling(game))
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
                basket.set_sprite("sBasketSwoosh");
                basket.animation = 0.0;
            }
        } else if !inside {
            ball.data.insert("goingIn".into(), 0.0);
        }
        ball.set_sprite(
            if ball.held
                && game.world.solid_point_active(
                    Vec2::new(ball.pos.x, ball.pos.y + 6.0),
                    CollisionPrecision::BoundingBox,
                    None,
                )
            {
                "sDribble"
            } else {
                "sBasketball"
            },
        );
        game.world.entities[index] = ball;
    }
}
/// Literal oTarget Create; normal Game.spawn invokes this synchronously.
pub(crate) fn create_target(game: &mut Game, id: u64) {
    if !game
        .world
        .entity(id)
        .is_some_and(|e| e.value("hub_initialized") == 0.0)
    {
        return;
    }
    let direction = game.rng.range(2);
    let e = game.world.entity_mut(id).unwrap();
    for (name, value) in [
        ("hub_initialized", 1.0),
        ("life", 20.0),
        ("dying", 0.0),
        ("UP", 0.0),
        ("DOWN", 1.0),
        ("dir", f64::from(direction)),
        ("moveOff", 32.0),
        ("hard", f64::from(e.pos.x > 208.0)),
        ("xDiff", 0.0),
    ] {
        e.data.insert(name.into(), value);
    }
}

pub(crate) fn target_step(game: &mut Game, id: u64) {
    let timer = value(game, "timer");
    let Some(e) = game.world.entity_mut(id) else {
        return;
    };
    if e.value("dying") != 0.0 {
        if e.value("life") > 0.0 {
            e.data.insert("life".into(), e.value("life") - 1.0);
        } else {
            e.data.insert("life".into(), 20.0);
            e.data.insert("dying".into(), 0.0);
        }
    } else {
        if e.value("dir") == 0.0 || e.value("dir") == 1.0 {
            let up = e.value("dir") == 0.0;
            let speed = if e.value("hard") != 0.0 { 2.0 } else { 1.0 };
            e.pos.y += if up { -speed } else { speed };
            e.data.insert("moveOff".into(), e.value("moveOff") + 1.0);
            if up && e.pos.y <= 64.0 || !up && e.pos.y >= 160.0 || e.value("moveOff") > 64.0 {
                e.data.insert("dir".into(), f64::from(up));
                e.data.insert("moveOff".into(), 0.0);
            }
        }
        if timer <= 30.0 {
            e.pos.x = 240.0 - (e.value("xDiff").sin() * 64.0).abs();
            e.data.insert("xDiff".into(), e.value("xDiff") + 0.01);
        }
    }
}

pub(crate) fn target_collision(game: &mut Game, id: u64, other: crate::dispatcher::Instance) {
    let crate::dispatcher::Instance::Entity(arrow) = other else {
        return;
    };
    let Some(a) = game.world.entity(arrow) else {
        return;
    };
    if !crate::catalog::is_a(&a.object, "oArrow") {
        return;
    }
    let arrow_y = a.pos.y;
    game.world.entity_mut(arrow).unwrap().vel = Vec2::default();
    let Some(target) = game.world.entity(id).cloned() else {
        return;
    };
    if target.value("dying") == 0.0 {
        game.world
            .entity_mut(id)
            .unwrap()
            .data
            .insert("dying".into(), 1.0);
        let distance = (arrow_y - target.pos.y).abs();
        let points = if distance <= 2.0 {
            game.sound("sndCoin", target.pos);
            5.0
        } else if (3.0..=6.0).contains(&distance) {
            3.0
        } else if (7.0..=10.0).contains(&distance) {
            2.0
        } else {
            1.0
        };
        *game.data.entry("baskets".into()).or_default() += points;
        game.sound("sndHit", target.pos);
    }
    if target.value("life") <= 2.0 {
        // Arrow inherits only oItem's held-hand cleanup in Destroy.
        if game.world.entity(arrow).is_some_and(|e| e.held) {
            game.player.held = None;
        }
        game.world.entity_mut(arrow).unwrap().alive = false;
    }
}

/// Compatibility for direct hub::tick callers without the engine dispatcher.
/// Live source Steps and precise Collision events use the callbacks above.
fn static_targets(game: &mut Game) {
    let ids: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && e.engine_active && e.object == "oTarget")
        .map(|e| e.id)
        .collect();
    for id in ids {
        create_target(game, id);
        target_step(game, id);
        let target = game.world.entity(id).unwrap().clone();
        let arrows: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|a| {
                a.alive
                    && a.engine_active
                    && crate::catalog::is_a(&a.object, "oArrow")
                    && a.event_intersects(&target)
            })
            .map(|a| a.id)
            .collect();
        for arrow in arrows {
            target_collision(game, id, crate::dispatcher::Instance::Entity(arrow));
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
    fn authored_hubs_preserve_room_graphic_tiles() {
        let expected = [
            (
                "rTitle",
                "bgTitle",
                [64.0, 0.0, 16.0, 16.0],
                Vec2::new(32.0, 128.0),
            ),
            (
                "rHighscores",
                "bgTitle",
                [16.0, 0.0, 16.0, 16.0],
                Vec2::new(32.0, 48.0),
            ),
            (
                "rHighscores",
                "bgTitle",
                [32.0, 0.0, 16.0, 16.0],
                Vec2::new(64.0, 80.0),
            ),
            (
                "rHighscores",
                "bgTitle",
                [48.0, 0.0, 16.0, 16.0],
                Vec2::new(32.0, 112.0),
            ),
            (
                "rHighscores",
                "bgTitle",
                [64.0, 0.0, 16.0, 16.0],
                Vec2::new(64.0, 144.0),
            ),
            (
                "rSun",
                "bgSky",
                [0.0, 0.0, 32.0, 32.0],
                Vec2::new(144.0, 48.0),
            ),
            (
                "rMoon",
                "bgSky",
                [32.0, 0.0, 32.0, 32.0],
                Vec2::new(48.0, 48.0),
            ),
            (
                "rStars",
                "bgSky",
                [64.0, 0.0, 32.0, 32.0],
                Vec2::new(144.0, 64.0),
            ),
        ];
        for room in ["rTitle", "rHighscores", "rSun", "rMoon", "rStars"] {
            let world = authored_world(room);
            let room_expected: Vec<_> = expected.iter().filter(|entry| entry.0 == room).collect();
            assert_eq!(
                world.generated_graphic_tiles.len(),
                room_expected.len(),
                "{room}"
            );
            for (actual, (_, background, source, position)) in
                world.generated_graphic_tiles.iter().zip(room_expected)
            {
                assert_eq!(actual.graphic.background, *background, "{room}");
                assert_eq!(actual.graphic.source, *source, "{room}");
                assert_eq!(actual.graphic.position, *position, "{room}");
                assert_eq!(actual.graphic.depth, 1_000_000.0, "{room}");
                assert_eq!(actual.graphic.scale, Vec2::new(1.0, 1.0), "{room}");
                assert_eq!(actual.graphic.color, 0x00ff_ffff, "{room}");
                assert_eq!(actual.graphic.alpha, 1.0, "{room}");
                assert!(actual.graphic.visible, "{room}");
            }
        }
    }

    #[test]
    fn source_sun_room_runs_authored_create_rng_before_the_first_step() {
        let seed = 19;
        let mut expected = Game::try_shell(seed, RulesPreset::Classic).unwrap();
        expected.world = world(HubKind::Sun, &expected.flags);
        for instance in &rooms().rooms["rSun"].instances {
            let center = Vec2::new(instance.x + 8.0, instance.y + 8.0);
            let origin = Vec2::new(instance.x, instance.y);
            match instance.object.as_str() {
                "oArrowRepeaterL" | "oArrowRepeaterR" | "oSmashTrap" => {
                    let x = (instance.x / 16.0) as usize;
                    let y = (instance.y / 16.0) as usize;
                    let terrain = expected
                        .world
                        .tile(x as i32, y as i32)
                        .is_some_and(|tile| tile.object == instance.object);
                    let receiver = if terrain {
                        crate::dispatcher::Instance::Terrain(y * expected.world.width + x)
                    } else {
                        let id = expected
                            .world
                            .entities
                            .iter()
                            .find(|entity| {
                                entity.alive
                                    && entity.object == instance.object
                                    && entity.pos == origin
                            })
                            .expect("authored trap instance")
                            .id;
                        crate::dispatcher::Instance::Entity(id)
                    };
                    crate::traps::create(&mut expected, receiver);
                }
                "oBrick" | "oDark" => {
                    let id = expected
                        .world
                        .entities
                        .iter()
                        .find(|entity| {
                            entity.alive
                                && entity.object == "oTerrainMetadata"
                                && entity.pos == center
                        })
                        .unwrap_or_else(|| panic!("{} metadata", instance.object))
                        .id;
                    initialize_authored_tile_create(&mut expected, &instance.object, id);
                }
                "oLava" => {
                    let id = expected
                        .world
                        .entities
                        .iter()
                        .find(|entity| {
                            entity.alive && entity.object == "oLava" && entity.pos == origin
                        })
                        .expect("authored Lava instance")
                        .id;
                    crate::world_events::create_instance(&mut expected, id);
                }
                "oBricks" => {
                    assert!(crate::generation::initialize_authored_setup_walls(
                        &mut expected.world,
                        &mut expected.rng,
                        expected.generation_revision,
                        true,
                        false,
                        false,
                    ));
                }
                "oSunRoom" => {
                    expected.data.insert(
                        "hub_alarm0".into(),
                        f64::from(200 + expected.rng.range(101)),
                    );
                }
                _ => {}
            }
        }

        let mut actual = Game::try_shell(seed, RulesPreset::Classic).unwrap();
        actual.load_hub(HubKind::Sun);

        for instance in &rooms().rooms["rSun"].instances {
            let center = Vec2::new(instance.x + 8.0, instance.y + 8.0);
            let origin = Vec2::new(instance.x, instance.y);
            match instance.object.as_str() {
                "oArrowRepeaterL" | "oArrowRepeaterR" => {
                    let x = (instance.x / 16.0) as usize;
                    let y = (instance.y / 16.0) as usize;
                    let key = if actual
                        .world
                        .tile(x as i32, y as i32)
                        .is_some_and(|tile| tile.object == instance.object)
                    {
                        format!("hazard:{}:fired", y * actual.world.width + x)
                    } else {
                        let id = actual
                            .world
                            .entities
                            .iter()
                            .find(|entity| {
                                entity.alive
                                    && entity.object == instance.object
                                    && entity.pos == origin
                            })
                            .expect("authored repeater")
                            .id;
                        format!("hazard_entity:{id}:fired")
                    };
                    assert_eq!(actual.data.get(&key), expected.data.get(&key));
                }
                "oSmashTrap" => {
                    let actual_trap = actual
                        .world
                        .entities
                        .iter()
                        .find(|entity| {
                            entity.alive && entity.object == instance.object && entity.pos == origin
                        })
                        .expect("authored Smash Trap");
                    let expected_trap = expected
                        .world
                        .entities
                        .iter()
                        .find(|entity| {
                            entity.alive && entity.object == instance.object && entity.pos == origin
                        })
                        .expect("expected Smash Trap");
                    assert_eq!(
                        actual_trap.data.get("direction"),
                        expected_trap.data.get("direction")
                    );
                }
                "oBrick" | "oDark" => {
                    let actual_terrain = actual
                        .world
                        .entities
                        .iter()
                        .find(|entity| {
                            entity.alive
                                && entity.object == "oTerrainMetadata"
                                && entity.pos == center
                        })
                        .unwrap_or_else(|| panic!("{} metadata", instance.object));
                    let expected_terrain = expected
                        .world
                        .entities
                        .iter()
                        .find(|entity| {
                            entity.alive
                                && entity.object == "oTerrainMetadata"
                                && entity.pos == center
                        })
                        .expect("expected terrain metadata");
                    assert_eq!(
                        actual_terrain.data.get("n"),
                        expected_terrain.data.get("n"),
                        "{} at ({}, {})",
                        instance.object,
                        instance.x,
                        instance.y
                    );
                }
                "oLava" => {
                    let actual_lava = actual
                        .world
                        .entities
                        .iter()
                        .find(|entity| {
                            entity.alive && entity.object == "oLava" && entity.pos == origin
                        })
                        .expect("authored Lava");
                    let expected_lava = expected
                        .world
                        .entities
                        .iter()
                        .find(|entity| {
                            entity.alive && entity.object == "oLava" && entity.pos == origin
                        })
                        .expect("expected Lava");
                    assert_eq!(
                        actual_lava.data.get("spurt_time"),
                        expected_lava.data.get("spurt_time"),
                        "Lava at ({}, {})",
                        instance.x,
                        instance.y
                    );
                    assert_eq!(actual_lava.value("randNum"), 201.0);
                    assert_eq!(
                        actual_lava.text.get("type").map(String::as_str),
                        Some("Lava")
                    );
                    assert_eq!(actual_lava.image_speed(), 0.4);
                }
                _ => {}
            }
        }
        assert_eq!(
            actual.data.get("hub_alarm0"),
            expected.data.get("hub_alarm0")
        );
        assert_eq!(actual.rng.state_bits(), expected.rng.state_bits());

        let directions: Vec<_> = actual
            .world
            .entities
            .iter()
            .filter(|entity| entity.alive && entity.object == "oSmashTrap")
            .map(|entity| (entity.id, entity.value("direction")))
            .collect();
        let lava_times: Vec<_> = actual
            .world
            .entities
            .iter()
            .filter(|entity| entity.alive && entity.object == "oLava")
            .map(|entity| (entity.id, entity.value("spurt_time")))
            .collect();
        actual.step(&InputFrame::default());
        for (id, direction) in directions {
            assert_eq!(
                actual.world.entity(id).unwrap().value("direction"),
                direction
            );
        }
        for (id, time) in lava_times {
            assert_eq!(actual.world.entity(id).unwrap().value("spurt_time"), time);
        }
    }

    #[test]
    fn source_title_controller_bricks_run_before_setup_walls() {
        let seed = 29;
        for flags in [
            BTreeSet::new(),
            BTreeSet::from(["first_time".into()]),
            BTreeSet::from(["shortcut_5".into()]),
            BTreeSet::from(["first_time".into(), "shortcut_5".into()]),
        ] {
            let mut expected = Game::try_shell(seed, RulesPreset::Classic).unwrap();
            expected.flags = flags.clone();
            expected.world = world(HubKind::Title, &flags);

            let mut entities_by_instance: HashMap<(String, u64, u64), VecDeque<u64>> =
                HashMap::new();
            for entity in expected
                .world
                .entities
                .iter()
                .filter(|entity| entity.alive && entity.object != "oTerrainMetadata")
            {
                entities_by_instance
                    .entry((
                        entity.object.clone(),
                        entity.pos.x.to_bits(),
                        entity.pos.y.to_bits(),
                    ))
                    .or_default()
                    .push_back(entity.id);
            }
            let mut metadata_by_origin: HashMap<(u64, u64), VecDeque<u64>> = HashMap::new();
            for entity in expected
                .world
                .entities
                .iter()
                .filter(|entity| entity.alive && entity.object == "oTerrainMetadata")
            {
                metadata_by_origin
                    .entry((
                        (entity.pos.x - 8.0).to_bits(),
                        (entity.pos.y - 8.0).to_bits(),
                    ))
                    .or_default()
                    .push_back(entity.id);
            }

            for instance in &rooms().rooms["rTitle"].instances {
                if instance.object == "oPlayer1" {
                    continue;
                }
                let entity_key = (
                    instance.object.clone(),
                    instance.x.to_bits(),
                    instance.y.to_bits(),
                );
                if let Some(id) = entities_by_instance
                    .get_mut(&entity_key)
                    .and_then(VecDeque::pop_front)
                {
                    initialize_authored_entity_create(&mut expected, id);
                    if instance.object == "oTitle" {
                        let [shortcut_stage1, _, _] = shortcut_stages(&flags);
                        let mut origins: Vec<(f64, f64)> = Vec::new();
                        if !shortcut_stage1 {
                            origins.extend([(320.0, 96.0), (336.0, 96.0)]);
                        }
                        if flags.contains("first_time") {
                            origins.extend([(320.0, 160.0), (336.0, 160.0), (336.0, 176.0)]);
                        }
                        for (x, y) in origins {
                            let id = metadata_by_origin
                                .get_mut(&(x.to_bits(), y.to_bits()))
                                .and_then(VecDeque::pop_front)
                                .expect("controller-created title brick metadata");
                            initialize_authored_tile_create(&mut expected, "oBrick", id);
                        }
                    }
                    if instance.object == "oBricks" {
                        assert!(crate::generation::initialize_authored_setup_walls(
                            &mut expected.world,
                            &mut expected.rng,
                            expected.generation_revision,
                            true,
                            false,
                            false,
                        ));
                    }
                    continue;
                }

                let metadata_key = (instance.x.to_bits(), instance.y.to_bits());
                if let Some(id) = metadata_by_origin
                    .get_mut(&metadata_key)
                    .and_then(VecDeque::pop_front)
                {
                    initialize_authored_tile_create(&mut expected, &instance.object, id);
                    continue;
                }

                let x = (instance.x / 16.0) as usize;
                let y = (instance.y / 16.0) as usize;
                if x < expected.world.width
                    && y < expected.world.height
                    && expected
                        .world
                        .tile(x as i32, y as i32)
                        .is_some_and(|tile| tile.object == instance.object)
                {
                    let index = y * expected.world.width + x;
                    crate::traps::create(
                        &mut expected,
                        crate::dispatcher::Instance::Terrain(index),
                    );
                }
            }

            let mut actual = Game::try_shell(seed, RulesPreset::Classic).unwrap();
            actual.flags = flags.clone();
            actual.load_hub(HubKind::Title);
            assert_eq!(
                actual.rng.state_bits(),
                expected.rng.state_bits(),
                "{flags:?}"
            );

            let mut dynamic_bricks = Vec::new();
            if !shortcut_stages(&flags)[0] {
                dynamic_bricks.extend([(320.0, 96.0), (336.0, 96.0)]);
            }
            if flags.contains("first_time") {
                dynamic_bricks.extend([(320.0, 160.0), (336.0, 160.0), (336.0, 176.0)]);
            }
            for (x, y) in dynamic_bricks {
                let origin = Vec2::new(x + 8.0, y + 8.0);
                let actual_metadata = actual
                    .world
                    .entities
                    .iter()
                    .find(|entity| {
                        entity.alive && entity.object == "oTerrainMetadata" && entity.pos == origin
                    })
                    .expect("actual title brick metadata");
                let expected_metadata = expected
                    .world
                    .entities
                    .iter()
                    .find(|entity| {
                        entity.alive && entity.object == "oTerrainMetadata" && entity.pos == origin
                    })
                    .expect("expected title brick metadata");
                assert_eq!(
                    actual_metadata.data.get("n"),
                    expected_metadata.data.get("n"),
                    "brick at ({x}, {y}), flags {flags:?}"
                );
                assert_eq!(
                    actual_metadata.text.get("terrain_sprite"),
                    expected_metadata.text.get("terrain_sprite"),
                    "brick sprite at ({x}, {y}), flags {flags:?}"
                );
                assert_eq!(
                    actual_metadata.data.get("gold_chunks"),
                    expected_metadata.data.get("gold_chunks"),
                    "brick gold at ({x}, {y}), flags {flags:?}"
                );
            }
        }
    }

    #[test]
    fn authored_challenge_items_keep_inherited_source_create_depth() {
        for (kind, object) in [(HubKind::Moon, "oBow"), (HubKind::Stars, "oShotgun")] {
            for (has_spectacles, expected_depth) in [(false, 101.0), (true, 51.0)] {
                let mut game = Game::shell(7, RulesPreset::Classic);
                if has_spectacles {
                    game.player.equipment.insert("Spectacles".into());
                }
                game.load_hub(kind);

                let item = game
                    .world
                    .entities
                    .iter()
                    .find(|entity| entity.object == object)
                    .unwrap_or_else(|| panic!("{kind:?} has no authored {object}"));
                assert_eq!(item.data.get("depth"), Some(&expected_depth));
            }

            let mut historical = Game::shell(7, RulesPreset::Classic);
            historical.generation_revision = GenerationRevision::HistoricalV1;
            historical.load_hub(kind);
            let item = historical
                .world
                .entities
                .iter()
                .find(|entity| entity.object == object)
                .unwrap_or_else(|| panic!("{kind:?} has no authored {object}"));
            assert!(!item.data.contains_key("depth"));
        }
    }

    fn portal_grounding_fixture(floor_end: usize) -> Game {
        let mut g = Game::shell(31, RulesPreset::Classic);
        g.world = World::new(40, 20, Area::Mines, 1);
        g.flags.insert("hub".into());
        g.flags.insert("hub_title".into());
        for x in 0..floor_end {
            g.world.set_tile(x, 12, "oTemple");
        }
        g.world.spawn("oXStart", Vec2::new(384., 176.));
        g
    }
    fn finish_portal(g: &mut Game) {
        for _ in 0..40 {
            if g.state != RunState::Ending {
                return;
            }
            g.step(&InputFrame::default());
        }
        panic!("source portal animation did not finish");
    }
    #[test]
    fn challenge_record_snapshot_uses_strict_improvement_after_score_cap() {
        for (score, best, expected) in [
            (0., 0., 0.),
            (12., 12., 0.),
            (13., 12., 1.),
            (100., 99., 0.),
        ] {
            let mut g = loaded(HubKind::Moon);
            g.data.insert("best_mini2".into(), best);
            record(&mut g, "mini2", score);
            assert_eq!(g.data["challenge_highscore"], expected);
            assert_eq!(g.data["mini2"], score.min(99.));
        }
    }
    #[test]
    fn authored_scores_block_inputs_clear_only_after_nineteen_contact_ticks() {
        let mut g = Game::shell(13, RulesPreset::Classic);
        g.flags.extend(record_unlocks(200000, Some(600.), 120, 8));
        for flag in [
            "shortcut_5",
            "shortcut_9",
            "shortcut_13",
            "new_money",
            "character_damsel",
            "first_time",
        ] {
            g.flags.insert(flag.into());
        }
        g.load_hub(HubKind::Scores);
        g.data.insert("best_money".into(), 200000.);
        g.data.insert("best_mini2".into(), 42.);
        let block = g
            .world
            .entities
            .iter()
            .find(|e| e.object == "oPushBlock")
            .unwrap()
            .id;
        let button = g
            .world
            .entities
            .iter()
            .find(|e| e.object == "oButtonHighscore")
            .unwrap()
            .id;
        let mut contacts = 0;
        let mut clicks = 0;
        let mut thumps = 0;
        let mut warning_position_seen = false;
        for _ in 0..500 {
            g.step(&InputFrame {
                right: true,
                ..Default::default()
            });
            clicks += g
                .events
                .iter()
                .filter(|event| matches!(event,GameEvent::Sound{name,..} if name=="sndClick"))
                .count();
            thumps += g
                .events
                .iter()
                .filter(|event| matches!(event,GameEvent::Sound{name,..} if name=="sndThump"))
                .count();
            let pressed = g.world.entity(button).unwrap().value("pushed") != 0.;
            warning_position_seen |= g.world.entity(block).unwrap().pos.x > 160. && !pressed;
            if pressed {
                contacts += 1;
                assert!(!g.world.entities.iter().any(|e| e.alive
                    && matches!(
                        e.object.as_str(),
                        "oTrophy" | "oXSun" | "oXMoon" | "oXStars" | "oXChange"
                    )));
                assert_eq!(g.world.entity(button).unwrap().sprite, "sButtonPushed");
                if contacts < 19 {
                    assert!(!g.flags.contains("highscores_clear_requested"));
                    assert_eq!(g.data["best_money"], 200000.);
                    assert!(g.flags.contains("unlock_damsel"));
                }
            } else {
                contacts = 0;
            }
            if g.flags.contains("highscores_clear_requested") {
                break;
            }
        }
        assert!(
            warning_position_seen,
            "block must cross source warning threshold before button"
        );
        assert!(
            g.flags.contains("highscores_clear_requested"),
            "natural block route stalled player={:?} block={:?} bounds={:?} solid={} button={:?} counter={}",
            g.player.pos,
            g.world.entity(block).unwrap().pos,
            g.world.entity(block).unwrap().sprite_bounds(),
            g.world.entity(block).unwrap().solid,
            g.world.entity(button).unwrap().pos,
            g.world.entity(button).unwrap().value("counter")
        );
        assert_eq!(contacts, 19);
        assert_eq!((clicks, thumps), (1, 1));
        assert_eq!(g.data["shake"], 60.);
        assert_eq!(g.data["best_money"], 0.);
        assert_eq!(g.data["best_mini2"], 0.);
        assert_eq!(g.data["tunnel1_remaining"], 100001.);
        assert_eq!(g.data["tunnel2_remaining"], 200001.);
        assert!(g.flags.contains("character_damsel"));
        assert!(g.flags.contains("first_time"));
        assert!(g.flags.contains("new_money"));
        assert!(
            g.world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oNew")
        );
        assert!(!g.flags.contains("shortcut_5"));
        for _ in 0..40 {
            g.step(&InputFrame::default());
            assert!(
                !g.events
                    .iter()
                    .any(|event| matches!(event,GameEvent::Sound{name,..} if name=="sndThump"))
            );
        }
    }
    #[test]
    fn scores_button_ignores_inactive_solid_collision_targets() {
        for inactive_entity in [false, true] {
            let mut g = Game::shell(13, RulesPreset::Classic);
            g.world = World::new(20, 20, Area::Mines, 0);
            let button = g.world.spawn("oButtonHighscore", Vec2::new(80.0, 80.0));
            if inactive_entity {
                let target = g.world.spawn("oBlock", Vec2::new(80.0, 80.0));
                g.world.entity_mut(target).unwrap().engine_active = false;
            } else {
                g.world.set_tile(5, 5, "oBlock");
                g.world.tiles[5 * g.world.width + 5].engine_active = false;
            }

            scores_step(&mut g);

            assert_eq!(
                g.world.entity(button).unwrap().value("pushed"),
                0.0,
                "source collision_rectangle must ignore inactive target; entity={inactive_entity}"
            );
        }
    }

    #[test]
    fn scores_new_markers_match_source_flags_and_title_clears_them() {
        let mut g = Game::shell(14, RulesPreset::Classic);
        for flag in ["new_money", "new_kills", "new_saves", "new_time"] {
            g.flags.insert(flag.into());
        }
        g.load_hub(HubKind::Scores);
        let positions: Vec<_> = g
            .world
            .entities
            .iter()
            .filter(|e| e.object == "oNew")
            .map(|e| e.pos)
            .collect();
        assert_eq!(
            positions,
            vec![
                Vec2::new(272., 48.),
                Vec2::new(272., 64.),
                Vec2::new(272., 80.),
                Vec2::new(272., 96.)
            ]
        );
        assert!(
            g.world
                .entities
                .iter()
                .filter(|e| e.object == "oNew")
                .all(|e| e.value("visible") == 1.)
        );
        g.load_hub(HubKind::Title);
        assert!(!g.flags.iter().any(|f| f.starts_with("new_")));
        g.load_hub(HubKind::Scores);
        assert!(!g.world.entities.iter().any(|e| e.object == "oNew"));
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
        assert!(g.player.is_exiting());
        assert!(!g.flags.contains("hub_destination:campaign"));
        finish_portal(&mut g);
        assert!(
            g.flags.contains("hub_destination:campaign"),
            "pos {:?} state {:?}",
            g.player.pos,
            g.player.state
        );
    }
    #[test]
    fn natural_title_doors_keep_live_world_and_defer_destination_for_every_character() {
        for character in [
            crate::characters::Character::Explorer,
            crate::characters::Character::Damsel,
            crate::characters::Character::TunnelMan,
        ] {
            for (target, destination) in [(394., "campaign"), (440., "scores")] {
                let mut g = Game::shell(9, RulesPreset::Classic);
                crate::characters::apply(&mut g, character);
                g.load_hub(HubKind::Title);
                for _ in 0..200 {
                    if g.player.pos.x <= target {
                        break;
                    }
                    g.step(&InputFrame {
                        left: true,
                        ..Default::default()
                    });
                }
                assert!(
                    (g.player.pos.x - target).abs() <= 3.,
                    "natural route {character:?} {:?}",
                    g.player.pos
                );
                g.step(&InputFrame {
                    up: true,
                    ..Default::default()
                });
                assert!(g.player.is_exiting());
                assert_eq!(g.state, RunState::Ending);
                assert!(!g.flags.iter().any(|f| f.starts_with("hub_destination:")));
                let exit_x = g.player.pos.x;
                let entered = g.level_tick;
                let flare = g
                    .world
                    .entities
                    .iter()
                    .find(|e| e.object == "oFlare")
                    .unwrap()
                    .id;
                let age = g.world.entity(flare).unwrap().age;
                g.step(&InputFrame {
                    right: true,
                    jump: true,
                    jump_pressed: true,
                    attack: true,
                    attack_pressed: true,
                    ..Default::default()
                });
                assert!(g.player.is_exiting());
                assert_eq!(g.player.pos.x, exit_x);
                assert_eq!(g.player.attack_timer, 0);
                assert_eq!(g.world.name, "rTitle");
                assert!(g.world.entity(flare).unwrap().age > age);
                finish_portal(&mut g);
                assert!((2..=32).contains(&(g.level_tick - entered)));
                assert!(g.flags.contains(&format!("hub_destination:{destination}")));
                assert_eq!(g.state, RunState::Transition);
                assert_eq!(g.world.name, "rTitle");
                assert!(g.ending.is_none());
            }
        }
    }

    #[test]
    fn game_step_resolves_portal_receiver_in_active_family_order() {
        let mut g = Game::shell(9, RulesPreset::Classic);
        g.world = World::new(40, 20, Area::Mines, 1);
        g.flags.insert("hub".into());
        g.flags.insert("hub_title".into());
        for x in 0..g.world.width {
            g.world.set_tile(x, 12, "oTemple");
        }

        // The origin point is inside only the generic door at x=384, while
        // the Player's full mask also overlaps Scores at x=396. Create Scores
        // first so source family order chooses it for instance_place.
        g.world.spawn("oXScores", Vec2::new(396.0, 176.0));
        g.world.spawn("oXStart", Vec2::new(384.0, 176.0));
        g.player.pos = Vec2::new(392.0, 184.0);
        g.player.previous_pos = g.player.pos;
        g.player.grounded = true;
        g.player.state = PlayerState::Standing;

        g.step(&InputFrame {
            up: true,
            ..Default::default()
        });

        assert!(g.player.is_exiting());
        assert_eq!(
            g.ending.as_ref().and_then(|ending| ending.portal),
            Some(HubPortal::Scores),
            "instance_place uses the active source-family receiver order"
        );
    }

    #[test]
    fn game_step_title_portal_accepts_classic_ground_state_after_walkoff() {
        let mut g = portal_grounding_fixture(24);
        g.player.pos = Vec2::new(372., 184.);
        g.player.previous_pos = g.player.pos;
        g.player.state = PlayerState::Standing;
        g.player.grounded = true;

        let mut entered_at_walkoff = false;
        for _ in 0..40 {
            let mut probe = g.clone();
            probe.step(&InputFrame {
                right: true,
                ..Default::default()
            });
            if !probe.player.grounded && crate::player::source_platform_grounded(&probe) {
                g.step(&InputFrame {
                    right: true,
                    up: true,
                    ..Default::default()
                });
                assert!(!g.player.grounded);
                assert!(crate::player::source_platform_grounded(&g));
                assert!((384. ..400.).contains(&g.player.pos.x));
                assert_eq!(g.state, RunState::Ending);
                assert!(g.player.is_exiting());
                entered_at_walkoff = true;
                break;
            }
            g = probe;
        }
        assert!(
            entered_at_walkoff,
            "fixture must walk off inside the portal"
        );
    }

    #[test]
    fn game_step_title_portal_waits_for_classic_landing_state() {
        let mut g = portal_grounding_fixture(40);
        g.player.pos = Vec2::new(392., 183.);
        g.player.previous_pos = g.player.pos;
        g.player.vel = Vec2::new(0., 1.);
        g.player.state = PlayerState::Falling;

        g.step(&InputFrame {
            up: true,
            ..Default::default()
        });

        assert!(
            g.player.grounded,
            "the post-move support probe sees the floor"
        );
        assert!(!crate::player::source_platform_grounded(&g));
        assert_eq!(g.player.state, PlayerState::Falling);
        assert_eq!(g.state, RunState::Playing);
        assert!(!g.player.is_exiting());
    }

    #[test]
    fn game_step_routes_xend_portal_into_campaign_ending_room() {
        let mut g = Game::shell(9, RulesPreset::Classic);
        g.world = World::new(40, 20, Area::Mines, 1);
        g.flags.insert("hub".into());
        g.flags.insert("hub_title".into());
        for x in 0..g.world.width {
            g.world.set_tile(x, 12, "oTemple");
        }
        g.world.spawn("oXEnd", Vec2::new(384.0, 176.0));
        g.player.pos = Vec2::new(392.0, 184.0);
        g.player.previous_pos = g.player.pos;
        g.player.grounded = true;
        g.player.state = PlayerState::Standing;

        g.step(&InputFrame {
            up: true,
            ..Default::default()
        });

        assert!(g.player.is_exiting());
        assert_eq!(
            g.ending.as_ref().unwrap().kind,
            crate::ending::EndingKind::HubPortal
        );
        assert_eq!(g.ending.as_ref().unwrap().portal, Some(HubPortal::End));
        for _ in 0..40 {
            if g.world.name == "rEnd" {
                break;
            }
            g.step(&InputFrame::default());
        }
        assert_eq!(g.world.name, "rEnd");
        assert_eq!(
            g.ending.as_ref().unwrap().stage,
            crate::ending::EndingStage::Chamber
        );
    }

    #[test]
    fn inactive_authored_title_portal_does_not_activate_on_game_step() {
        let mut g = Game::shell(9, RulesPreset::Classic);
        g.load_hub(HubKind::Title);
        for _ in 0..200 {
            if g.player.pos.x <= 440.0 {
                break;
            }
            g.step(&InputFrame {
                left: true,
                ..Default::default()
            });
        }
        assert!((g.player.pos.x - 440.0).abs() <= 3.0);

        let scores = g
            .world
            .entities
            .iter()
            .find(|entity| entity.object == "oXScores")
            .unwrap()
            .id;
        g.world.entity_mut(scores).unwrap().engine_active = false;
        g.step(&InputFrame {
            up: true,
            ..Default::default()
        });

        assert_eq!(g.state, RunState::Playing);
        assert!(!g.player.is_exiting());
        assert!(!g.flags.contains("hub_destination:scores"));
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
        assert_eq!(g.player.bombs, 3.0);
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
            assert!(g.player.health > 0.0, "{kind:?}");
            assert!(g.player.grounded, "{kind:?} pos {:?}", g.player.pos);
            assert_eq!(g.player.pos, spawn, "{kind:?}");
        }
    }
    #[test]
    fn dead_challenge_can_return_to_playable_title_and_scores() {
        for destination in [HubKind::Title, HubKind::Scores] {
            let mut g = loaded(HubKind::Stars);
            g.player.health = 0.0;
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
            assert_eq!(g.player.health, 4.0);
            assert!(g.player.equipment.is_empty());
            assert_eq!(g.player.money, 0);
            assert!(!g.flags.contains("murderer"));
        }
    }
    #[test]
    fn changing_doors_restore_source_standing_state_and_drop_held_items() {
        for (kind, flag, door, facing, health) in [
            (HubKind::Scores, "unlock_damsel", "oXChange", -1, 4.0),
            (HubKind::Title, "unlock_tunnel_man", "oXChange2", 1, 2.0),
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
            assert!(g.player.is_exiting());
            assert!(
                !g.flags.contains("character_damsel") && !g.flags.contains("character_tunnelman")
            );
            finish_portal(&mut g);
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
        assert_eq!(g.player.health, 8.0);
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
        assert_eq!(g.player.health, 7.0);
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
        g.player.health = 0.0;
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
        // The room controller follows Player Step in source object order.
        // Losing the damsel zeros health now; Player observes death next Step.
        assert_eq!(sun.player.health, 0.0);
        assert_eq!(sun.state, RunState::Playing);
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
        finish_portal(&mut g);
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
    fn later_shortcut_portals_require_their_completed_stage() {
        for (object, destination, flag) in [
            ("oXShortcut9", "shortcut9", "shortcut_9"),
            ("oXShortcut13", "shortcut13", "shortcut_13"),
        ] {
            for unlocked in [false, true] {
                let mut g = challenge(HubKind::Title);
                let id = g.world.spawn(object, Vec2::new(600.0, 64.0));
                let bounds = g.world.entity(id).unwrap().bounds();
                g.player.pos = Vec2::new(bounds.x + bounds.w / 2.0, bounds.y + bounds.h / 2.0);
                g.player.grounded = true;
                g.player.state = PlayerState::Standing;
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
                finish_portal(&mut g);
                assert_eq!(
                    g.flags.contains(&format!("hub_destination:{destination}")),
                    unlocked,
                    "{object} unlocked={unlocked}"
                );
            }
        }
    }

    #[test]
    fn authored_level_five_shortcut_works_without_a_tunnel_unlock() {
        let mut g = challenge(HubKind::Title);
        assert!(!shortcut_stages(&g.flags)[0]);
        assert!(g.world.entities.iter().any(|e| e.object == "oXShortcut5"));

        // The source room's fixed level-five door is at (80,128), above the
        // authored floor. Exercise the same hub input controller used by the
        // Game::step dispatcher without bypassing portal eligibility.
        g.player.pos = Vec2::new(88.0, 136.0);
        g.player.previous_pos = g.player.pos;
        g.player.grounded = true;
        g.player.state = PlayerState::Standing;
        tick(
            &mut g,
            &InputFrame {
                up: true,
                ..InputFrame::default()
            },
        );
        assert!(g.player.is_exiting());
        finish_portal(&mut g);
        assert!(g.flags.contains("hub_destination:shortcut5"));
        g.start_from_hub(false, 5);
        assert_eq!(g.world.level, 5);
        assert!(g.flags.contains("used_shortcut"));
        assert_eq!(g.state, RunState::Playing);
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
        g.player.state = PlayerState::Standing;
        tick(
            &mut g,
            &InputFrame {
                up: true,
                ..InputFrame::default()
            },
        );
        finish_portal(&mut g);
        assert!(g.flags.contains("hub_destination:campaign"));
        assert_eq!(g.world.level, 0);
    }
    #[test]
    fn sun_intro_damsel_loss_records_when_player_enters_death() {
        let mut g = loaded(HubKind::Sun);
        g.data.insert("points".into(), 42.0);
        g.world
            .entities
            .iter_mut()
            .find(|e| e.object == "oDamsel")
            .unwrap()
            .health = 0;

        g.step(&InputFrame::default());
        assert_eq!(g.player.health, 0.0);
        assert_eq!(g.state, RunState::Playing);
        assert!(!g.data.contains_key("mini1"));
        assert!(!g.flags.contains("hub_challenge_finished"));

        g.step(&InputFrame::default());
        assert_eq!(g.state, RunState::Dead);
        assert_eq!(g.data["mini1"], 42.0);
        assert!(g.flags.contains("hub_challenge_finished"));
    }

    #[test]
    fn sun_intro_damsel_loss_does_not_finish_before_ankh_revival() {
        let mut g = loaded(HubKind::Sun);
        g.data.insert("points".into(), 42.0);
        g.player.equipment.insert("Ankh".into());
        g.world
            .entities
            .iter_mut()
            .find(|e| e.object == "oDamsel")
            .unwrap()
            .health = 0;

        // The room Step zeros life after Player Step, so no score is committed
        // until the next Player Step handles the fatal life value.
        g.step(&InputFrame::default());
        assert_eq!(g.player.health, 0.0);
        assert_eq!(g.state, RunState::Playing);
        assert!(!g.data.contains_key("mini1"));
        assert!(!g.flags.contains("hub_challenge_finished"));

        // Ankh revives in Player Step, then the still-running intro room Step
        // zeros life again. This must not have recorded a completed challenge.
        g.step(&InputFrame::default());
        assert_eq!(g.player.health, 0.0);
        assert!(!g.player.has("Ankh"));
        assert_eq!(g.state, RunState::Playing);
        assert!(!g.data.contains_key("mini1"));
        assert!(!g.flags.contains("hub_challenge_finished"));

        g.step(&InputFrame::default());
        assert_eq!(g.state, RunState::Dead);
        assert_eq!(g.data["mini1"], 42.0);
        assert!(g.flags.contains("hub_challenge_finished"));
    }
    #[test]
    fn sun_damsel_loss_after_intro_does_not_fail_challenge() {
        let mut g = loaded(HubKind::Sun);
        for _ in 0..70 {
            g.step(&InputFrame::default());
        }
        assert_eq!(g.data["hub_draw_status"], 3.0);
        for e in &mut g.world.entities {
            if e.object == "oDamsel" {
                e.health = 0;
            }
        }

        g.step(&InputFrame::default());

        assert!(g.player.health > 0.0);
        assert_eq!(g.state, RunState::Playing);
        assert!(!g.flags.contains("hub_challenge_finished"));
        assert!(!g.data.contains_key("mini1"));
    }
    #[test]
    fn sun_setup_walls_initializes_authored_vines_even_with_low_graphics() {
        for low_graphics in [false, true] {
            for revision in [
                GenerationRevision::HistoricalV1,
                GenerationRevision::SourceClassicV2,
            ] {
                let mut game = Game::shell(7, RulesPreset::Classic);
                game.generation_revision = revision;
                if low_graphics {
                    game.flags.insert("low_graphics".into());
                }
                game.load_hub(HubKind::Sun);
                assert_eq!(
                    game.world
                        .tiles
                        .iter()
                        .filter(|t| t.object == "oVine")
                        .count(),
                    12
                );
                for x in [7, 12] {
                    for (y, expected) in [(2, "sVineSource"), (7, "sVineBottom")] {
                        let sprite = game.world.entities.iter().find_map(|e| {
                            (e.alive
                                && e.object == "oTerrainMetadata"
                                && e.pos == Vec2::new(x as f64 * 16.0 + 8.0, y as f64 * 16.0 + 8.0))
                            .then(|| e.text.get("terrain_sprite").map(String::as_str))
                            .flatten()
                        });
                        assert_eq!(
                            sprite,
                            (revision == GenerationRevision::SourceClassicV2).then_some(expected)
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn sun_setup_walls_spur_lava_emits_particles_through_game_steps() {
        let mut game = Game::shell(7, RulesPreset::Classic);
        game.load_hub(HubKind::Sun);

        let lava = game
            .world
            .entities
            .iter()
            .filter(|entity| entity.alive && entity.object == "oLava")
            .collect::<Vec<_>>();
        let top_lava = lava
            .iter()
            .filter(|entity| entity.sprite == "sLavaTop")
            .count();
        assert!(top_lava > 0, "scrSetupWalls exposes the upper lava row");
        let spurter = lava
            .iter()
            .find(|entity| entity.value("spurt") == 1.0)
            .expect("one exposed lava tile takes the source one-in-four spurt roll");
        let spurt_time = spurter.value("spurt_time");
        assert!(spurt_time >= 100.0 && spurt_time <= 300.0);

        let particles_before = game
            .world
            .entities
            .iter()
            .filter(|entity| matches!(entity.object.as_str(), "oMagma" | "oFlame"))
            .count();
        for _ in 0..=(spurt_time as usize + 1) {
            game.step(&InputFrame::default());
        }
        let particles_after = game
            .world
            .entities
            .iter()
            .filter(|entity| matches!(entity.object.as_str(), "oMagma" | "oFlame"))
            .count();
        assert!(particles_after > particles_before);
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
        let mut g = loaded(HubKind::Sun);
        for _ in 0..100 {
            g.step(&InputFrame::default());
        }
        assert_eq!(g.data["points"], 1.0);
        assert_eq!(g.player.health, 8.0);
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
        g.step(&InputFrame::default());
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
        g.flags.insert("darkLevel".into());
        g.flags.insert("snakePit".into());
        g.player.money = 12345;
        g.data.insert("kali_punish".into(), 1.);
        g.data.insert("kali_gift".into(), 1.);
        g.data.insert("currLevel".into(), 16.);
        g.data.insert("shake_toggle".into(), 1.);
        g.data.insert("message_timer".into(), 100.);
        g.data.insert("mini1".into(), 21.);
        g.data.insert("mini2".into(), 22.);
        g.data.insert("mini3".into(), 23.);
        g.data.insert("arrows".into(), 9.);
        g.player.bombs = 33.0;
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
        finish_portal(&mut g);
        assert!(g.flags.contains("hub_destination:title"));
        assert!(g.flags.contains("title_start:3"));
        g.load_hub(HubKind::Title);
        assert_eq!(g.player.pos, Vec2::new(344., 184.));
        assert_eq!(
            (g.player.health, g.player.bombs, g.player.ropes),
            (4.0, 4.0, 4.0)
        );
        assert_eq!(g.player.money, 0);
        assert!(g.player.equipment.is_empty());
        assert_eq!(g.data["currLevel"], 1.);
        assert_eq!(g.data["shake_toggle"], 0.);
        assert_eq!(g.data["shakeToggle"], 0.);
        assert_eq!(g.data["message_timer"], 0.);
        assert_eq!(
            [g.data["mini1"], g.data["mini2"], g.data["mini3"]],
            [0., 0., 0.]
        );
        assert_eq!(g.data["arrows"], 0.);
        assert!(!g.flags.contains("darkLevel"));
        assert!(!g.flags.contains("snakePit"));
        assert!(!g.flags.contains("wanted"));
        assert!(g.flags.contains("shortcut_5"));
        assert_eq!(g.data["kali_punish"], 0.);
        assert_eq!(g.data["kali_gift"], 0.);
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
        g.data.insert("currLevel".into(), 16.0);
        g.data.insert("message_timer".into(), 100.0);
        g.player.equipment.insert("Jordans".into());
        g.player.equipment.insert("Compass".into());
        g.load_hub(HubKind::Scores);
        for flag in ["dark", "darkLevel", "snake_pit", "snakePit"] {
            assert!(!g.flags.contains(flag));
        }
        assert!(!g.player.has("Jordans"));
        assert_eq!(g.data["currLevel"], 1.0);
        assert_eq!(g.data["message_timer"], 0.0);
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
        g.player.health = 1.0;
        g.player.state = PlayerState::Dead;
        g.state = RunState::Dead;
        g.player.money = 10000;
        g.player.equipment.insert("Kapala".into());
        activate_portal(&mut g);
        assert!(!g.flags.iter().any(|f| f.starts_with("hub_destination:")));
        g.load_hub(HubKind::Scores);
        assert_eq!(g.player.health, 4.0);
        assert_eq!(g.player.money, 0);
        assert!(g.player.equipment.is_empty());
    }

    #[test]
    fn acknowledged_level_death_clears_source_run_globals_before_scores_room() {
        let mut g = Game::shell(41, RulesPreset::Classic);
        g.data.insert("marketChance".into(), 73.0);
        g.data.insert("goldChance".into(), 29.0);
        g.data.insert("probDarkLevel".into(), 3.0);
        g.data.insert("currLevel".into(), 12.0);
        g.data.insert("levelType".into(), 3.0);
        g.data.insert("time".into(), 120_000.0);
        g.data.insert("kills".into(), 7.0);
        g.flags.extend([
            "made_market_entrance".into(),
            "made_gold_entrance".into(),
            "ghost_exists".into(),
            "murderer".into(),
        ]);
        g.player.money = 1_000;
        g.player.equipment.insert("Jetpack".into());
        g.world.spawn("oExplosion", g.player.pos);

        g.step(&InputFrame::default());
        assert_eq!(g.state, RunState::Dead);
        g.step(&InputFrame {
            attack_pressed: true,
            ..Default::default()
        });
        assert_eq!(g.state, RunState::Dead, "first edge skips the reveal");
        g.step(&InputFrame {
            attack_pressed: true,
            ..Default::default()
        });
        assert_eq!(g.state, RunState::Transition);
        assert_eq!(g.lifetime_stats.level_deaths[0], 1);

        // The host completes the acknowledged room transition here, as source
        // room_goto(rHighscores) does after scrClearGlobals in Player Step.
        g.load_hub(HubKind::Scores);

        assert_eq!(g.data["marketChance"], 160.0);
        assert_eq!(g.data["goldChance"], 160.0);
        assert_eq!(g.data["probDarkLevel"], 12.0);
        assert_eq!(g.data["currLevel"], 1.0);
        assert_eq!(g.data["levelType"], 0.0);
        assert_eq!(g.data["time"], 0.0);
        assert_eq!(g.data["kills"], 0.0);
        for flag in [
            "made_market_entrance",
            "made_gold_entrance",
            "ghost_exists",
            "murderer",
        ] {
            assert!(!g.flags.contains(flag), "run flag {flag} survived death");
        }
        assert!(g.flags.contains("no_dark_level"));
        assert_eq!(g.player.health, 4.0);
        assert_eq!((g.player.bombs, g.player.ropes), (4.0, 4.0));
        assert_eq!(g.player.money, 0);
        assert!(g.player.equipment.is_empty());
        assert_eq!(g.lifetime_stats.level_deaths[0], 1);
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
        finish_portal(&mut g);
        assert!(g.flags.contains("hub_destination:tutorial"));
        assert!(g.flags.contains("onboarding_consumed"));
        assert!(!g.flags.contains("first_time"));
        assert_eq!(
            (g.world.level, g.player.bombs, g.player.ropes),
            (0, 0.0, 2.0)
        );
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
        g.player.state = PlayerState::Standing;
        g.step(&InputFrame {
            up: true,
            ..Default::default()
        });
        finish_portal(&mut g);
        assert!(g.flags.contains("hub_destination:tutorial"));
        assert!(g.flags.contains("first_time"));
        assert!(!g.flags.contains("onboarding_consumed"));
        assert_eq!((g.player.bombs, g.player.ropes), (0.0, 2.0));
    }
    #[test]
    fn explicit_unskipped_intro_has_barriers_but_no_early_hint_hand() {
        let flags = BTreeSet::from(["first_time".into(), "title_start:0".into()]);
        let title = world(HubKind::Title, &flags);
        assert!(!title.entities.iter().any(|e| e.object == "oHintHand"));
        assert_eq!(title.tile(21, 11).unwrap().object, "oBrick");
    }
}

#[cfg(test)]
#[path = "stars_alarm11_eager_tests.rs"]
mod stars_alarm11_eager_tests;
