//! Source resource dispatch and independent engine activation membership.
use crate::{Entity, Game, InputFrame, Rect, Tile, Vec2, catalog};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
mod collision_events;
pub(crate) fn collision_owner(this: &str, other: &str) -> Option<&'static str> {
    collision_events::owner(this, other)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Instance {
    Player,
    Entity(u64),
    Terrain(usize),
}
impl Instance {
    pub fn object(self, game: &Game) -> &str {
        match self {
            Self::Player => "oPlayer1",
            Self::Entity(id) => game
                .world
                .entities
                .iter()
                .chain(game.dispatcher.hidden_entities.iter())
                .find(|e| e.id == id)
                .map_or("", |e| &e.object),
            Self::Terrain(index) => game
                .dispatcher
                .hidden_tiles
                .iter()
                .find(|(i, _)| *i == index)
                .map(|(_, t)| t)
                .or_else(|| game.world.tiles.get(index))
                .map_or("", |t| &t.object),
        }
    }
    pub fn position(self, game: &Game) -> Vec2 {
        match self {
            Self::Player => game.player.pos,
            Self::Entity(id) => game
                .world
                .entities
                .iter()
                .chain(game.dispatcher.hidden_entities.iter())
                .find(|e| e.id == id)
                .map_or(Vec2::default(), |e| e.pos),
            Self::Terrain(i) => Vec2::new(
                (i % game.world.width) as f64 * 16.0,
                (i / game.world.width) as f64 * 16.0,
            ),
        }
    }
    pub fn alarms_mut(self, game: &mut Game) -> Option<&mut [i32; 12]> {
        match self {
            Self::Player => Some(&mut game.player.alarms),
            Self::Entity(id) => game
                .world
                .entities
                .iter_mut()
                .find(|e| e.id == id)
                .map(|e| &mut e.alarms),
            Self::Terrain(i) => game.world.tiles.get_mut(i).map(|t| &mut t.alarms),
        }
    }
}
pub const fn unset_alarms() -> [i32; 12] {
    [-1; 12]
}
pub const fn initially_active() -> bool {
    true
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Phase {
    #[default]
    Idle,
    BeginStep,
    Alarm(u8),
    Keyboard,
    NormalStep,
    BuiltinMotion,
    OutsideRoom,
    AutomaticCollision,
    EndStep,
    Draw,
    Animation,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct State {
    pub phase: Phase,
    pub active_order: Vec<Instance>,
    pub inactive_order: Vec<Instance>,
    #[serde(default)]
    pub resource_order: BTreeMap<i32, Vec<Instance>>,
    #[serde(default)]
    pub registration_order: Vec<Instance>,
    #[serde(skip)]
    hidden_entities: Vec<Entity>,
    #[serde(skip)]
    hidden_tiles: Vec<(usize, Tile)>,
    #[serde(skip)]
    storage_order: Vec<u64>,
    #[serde(skip)]
    pub running: bool,
    // Execution guards are distinct from native query-visible retirement.
    #[serde(skip)]
    pub(crate) destroying_solids: BTreeMap<(Instance, u64), u8>,
    #[serde(skip)]
    pub(crate) detached_actor_slots: Vec<(u64, usize)>,
    #[serde(default)]
    known_terrain: BTreeMap<usize, (u64, String)>,
    #[serde(skip)]
    synced_revision: Option<(u64, u64, u64)>,
    // Source xprevious is unchanged while inactive, but it must not replay
    // old movement when that actor becomes visible in a fresh Draw snapshot.
    #[serde(skip)]
    interpolation_starts: BTreeMap<u64, Vec2>,
}
impl State {
    pub(crate) fn prepare_draw_interpolation(&self, entities: &mut [Entity]) {
        if self.phase != Phase::Draw {
            return;
        }
        for entity in entities {
            if let Some(&origin) = self.interpolation_starts.get(&entity.id) {
                entity.previous_pos = origin;
            }
        }
    }

    /// Frozen instances omitted from world storage by source activation.
    /// Presentation callers should read this from `Game::presentation()`;
    /// borrowing the slice does not activate, register, or advance its actors.
    pub fn hidden_entities(&self) -> &[Entity] {
        &self.hidden_entities
    }

    /// Background tiles remain queryable when their instance adapter is hidden.
    pub(crate) fn remove_decoration_at(
        &mut self,
        entities: &mut [Entity],
        depth: f64,
        point: Vec2,
    ) {
        let selected = entities
            .iter()
            .chain(self.hidden_entities.iter())
            .filter(|tile| {
                tile.alive
                    && tile.object == "oGenerationDecoration"
                    && tile.value("depth") == depth
                    && Rect::new(
                        tile.pos.x,
                        tile.pos.y,
                        tile.data.get("source_w").copied().unwrap_or(16.0),
                        tile.data.get("source_h").copied().unwrap_or(16.0),
                    )
                    .contains(point)
            })
            .min_by_key(|tile| {
                (
                    self.storage_order
                        .iter()
                        .position(|id| *id == tile.id)
                        .unwrap_or(usize::MAX),
                    tile.creation_serial,
                )
            })
            .map(|tile| tile.id);
        if let Some(id) = selected {
            if let Some(tile) = entities
                .iter_mut()
                .chain(self.hidden_entities.iter_mut())
                .find(|tile| tile.id == id)
            {
                tile.alive = false;
            }
        }
    }

    /// Frozen terrain omitted from world storage, paired with its grid index.
    /// Read from `Game::presentation()` alongside that same snapshot's world.
    /// The storage order and identities are preserved without filtering.
    pub fn hidden_tiles(&self) -> &[(usize, Tile)] {
        &self.hidden_tiles
    }
}
/// Callback activation may move a still-live Destroy receiver out of World.
pub(crate) fn stored_entity_mut(game: &mut Game, id: u64) -> Option<&mut Entity> {
    game.world
        .entities
        .iter_mut()
        .chain(game.dispatcher.hidden_entities.iter_mut())
        .find(|entity| entity.id == id)
}

pub(crate) fn take_stored_entity(game: &mut Game, id: u64) -> Option<Entity> {
    if let Some(index) = game
        .world
        .entities
        .iter()
        .position(|entity| entity.id == id)
    {
        return Some(game.world.entities.remove(index));
    }
    let index = game
        .dispatcher
        .hidden_entities
        .iter()
        .position(|entity| entity.id == id)?;
    Some(game.dispatcher.hidden_entities.remove(index))
}

/// Retire this identity, preserving a replacement born at the same grid cell.
pub(crate) fn retire_solid_terrain(game: &mut Game, index: usize, serial: u64) {
    let same = game
        .world
        .tiles
        .get(index)
        .is_some_and(|tile| tile.creation_serial == serial);
    let hidden = game
        .dispatcher
        .hidden_tiles
        .iter()
        .position(|(i, tile)| *i == index && tile.creation_serial == serial);
    if let Some(hidden) = hidden {
        game.dispatcher.hidden_tiles.remove(hidden);
    }
    if same || (hidden.is_some() && game.world.tiles[index].object.is_empty()) {
        let water_was_dirty = game.world.fluid_dirty;
        game.world
            .set_tile(index % game.world.width, index / game.world.width, "");
        game.world.fluid_dirty = water_was_dirty;
    }
}

fn alive(game: &Game, instance: Instance) -> bool {
    match instance {
        Instance::Player => true,
        Instance::Entity(id) => game.world.entities.iter().any(|e| e.id == id && e.alive),
        Instance::Terrain(i) => game
            .world
            .tiles
            .get(i)
            .is_some_and(|t| !t.object.is_empty()),
    }
}

#[cfg(test)]
mod presentation_storage_tests {
    use super::*;
    use crate::{Area, PlayerState, RulesPreset, World};

    #[test]
    fn hidden_slice_accessors_borrow_exact_storage_including_duplicate_identities() {
        let mut state = State::default();
        state
            .hidden_entities
            .push(Entity::new(7, "oRock", Vec2::new(600.0, 80.0)));
        state
            .hidden_entities
            .push(Entity::new(7, "oBomb", Vec2::new(640.0, 80.0)));
        state.hidden_tiles.push((37, Tile::new("oBrick")));
        state.hidden_tiles.push((37, Tile::new("oLush")));
        let before = serde_json::to_vec(&state).unwrap();
        assert!(std::ptr::eq(
            state.hidden_entities(),
            state.hidden_entities.as_slice()
        ));
        assert!(std::ptr::eq(
            state.hidden_tiles(),
            state.hidden_tiles.as_slice()
        ));
        assert_eq!(
            state
                .hidden_entities()
                .iter()
                .map(|e| (e.id, e.object.as_str()))
                .collect::<Vec<_>>(),
            [(7, "oRock"), (7, "oBomb")]
        );
        assert_eq!(
            state
                .hidden_tiles()
                .iter()
                .map(|(i, t)| (*i, t.object.as_str()))
                .collect::<Vec<_>>(),
            [(37, "oBrick"), (37, "oLush")]
        );
        assert_eq!(serde_json::to_vec(&state).unwrap(), before);
    }

    #[test]
    fn actual_step_exposes_frozen_draw_storage_without_changing_source_state() {
        let mut game = Game::shell(5, RulesPreset::Classic);
        game.world = World::new(80, 40, Area::Mines, 1);
        game.world.name = "presentation storage fixture".into();
        game.player.pos = Vec2::new(40.0, 152.0);
        game.player.state = PlayerState::Standing;
        game.player.grounded = true;
        for x in 0..80 {
            game.world.set_tile(x, 10, "oBrick");
        }
        let id = game.world.spawn("oBomb", Vec2::new(600.0, 152.0));
        let bomb = game.world.entity_mut(id).unwrap();
        bomb.data.insert("armed".into(), 1.0);
        bomb.alarms[0] = 5;
        bomb.alarms[1] = -1;
        bomb.age = 7;
        for _ in 0..8 {
            game.step(&InputFrame::default());
        }
        let index = 10 * game.world.width + 37;
        let state = game.state_hash();
        let rng = game.rng.clone().next_u64();
        let camera = (game.previous_camera, game.camera);
        let draw = game.presentation();
        let draw_state = draw.state_hash();
        assert_eq!(draw.dispatcher.phase, Phase::Draw);
        assert!(draw.world.entity(id).is_none());
        assert!(draw.world.tiles[index].object.is_empty());
        let frozen = draw
            .dispatcher
            .hidden_entities()
            .iter()
            .find(|e| e.id == id)
            .unwrap();
        // Source alarms/bookkeeping run once before Level deactivation, then
        // the offcamera bomb freezes. Reading an expanded view must not tick it.
        assert_eq!((frozen.alarms[0], frozen.alarms[1], frozen.age), (4, -1, 8));
        assert!(!frozen.engine_active);
        let terrain = draw
            .dispatcher
            .hidden_tiles()
            .iter()
            .find(|(i, _)| *i == index)
            .unwrap();
        assert_eq!(terrain.1.object, "oBrick");
        assert!(!terrain.1.engine_active);
        assert_eq!(game.world.entity(id).unwrap().alarms[0], frozen.alarms[0]);
        for _ in 0..100 {
            std::hint::black_box(draw.dispatcher.hidden_entities());
            std::hint::black_box(draw.dispatcher.hidden_tiles());
        }
        assert_eq!(draw.state_hash(), draw_state);
        assert_eq!(game.state_hash(), state);
        assert_eq!(game.rng.clone().next_u64(), rng);
        assert_eq!((game.previous_camera, game.camera), camera);
    }
}
pub fn is_active(game: &Game, instance: Instance) -> bool {
    match instance {
        Instance::Player => game.player.engine_active,
        Instance::Entity(id) => game
            .world
            .entities
            .iter()
            .any(|e| e.id == id && e.engine_active),
        Instance::Terrain(i) => game
            .world
            .tiles
            .get(i)
            .is_some_and(|t| t.engine_active && !t.object.is_empty()),
    }
}
fn source_object(game: &Game, instance: Instance) -> String {
    match instance {
        Instance::Entity(id) => crate::world_events::source_object(game, id)
            .unwrap_or_else(|| instance.object(game).to_owned()),
        _ => instance.object(game).to_owned(),
    }
}
fn resource(game: &Game, instance: Instance) -> i32 {
    catalog::object(&source_object(game, instance)).map_or(i32::MAX, |o| o.id)
}
fn metadata_index(game: &Game, e: &Entity) -> Option<usize> {
    if e.object != "oTerrainMetadata" {
        return None;
    }
    let x = (e.pos.x / 16.0).floor() as i32;
    let y = (e.pos.y / 16.0).floor() as i32;
    (x >= 0 && y >= 0 && (x as usize) < game.world.width && (y as usize) < game.world.height)
        .then_some(y as usize * game.world.width + x as usize)
}
fn rebuild_resources(game: &mut Game) {
    // Activation has already restored storage and applied this call's flags.
    // Index that exact state locally; never retain an index across callbacks,
    // storage partitioning, births, or later activation calls.
    let entity_resources: HashMap<_, _> = game
        .world
        .entities
        .iter()
        .map(|e| {
            let object = if e.alive {
                metadata_index(game, e)
                    .and_then(|i| game.world.tiles.get(i))
                    .map(|t| t.object.as_str())
                    .filter(|object| matches!(*object, "oTree" | "oLava"))
                    .unwrap_or(&e.object)
            } else {
                &e.object
            };
            (e.id, catalog::object(object).map_or(i32::MAX, |o| o.id))
        })
        .collect();
    let mut map: BTreeMap<i32, Vec<Instance>> = BTreeMap::new();
    for &instance in &game.dispatcher.active_order {
        let key = match instance {
            Instance::Entity(id) => entity_resources
                .get(&id)
                .copied()
                .unwrap_or_else(|| resource(game, instance)),
            _ => resource(game, instance),
        };
        map.entry(key).or_default().push(instance);
    }
    game.dispatcher.resource_order = map;
    game.dispatcher.registration_order = game.dispatcher.active_order.clone();
}
fn remove_registration(game: &mut Game, instance: Instance) {
    game.dispatcher.active_order.retain(|i| *i != instance);
    game.dispatcher.inactive_order.retain(|i| *i != instance);
    game.dispatcher
        .registration_order
        .retain(|i| *i != instance);
    for instances in game.dispatcher.resource_order.values_mut() {
        instances.retain(|i| *i != instance);
    }
}
/// Register births without rebuilding or reordering existing source arrays.
pub(crate) fn register_instances(game: &mut Game) {
    sync_newborns(game);
}

fn sync_newborns(game: &mut Game) {
    let revision = (
        game.world.next_id,
        game.world.tile_revision,
        game.level_tick,
    );
    if game.dispatcher.synced_revision == Some(revision) {
        return;
    }
    game.dispatcher.synced_revision = Some(revision);
    let hidden: BTreeSet<_> = game
        .dispatcher
        .hidden_tiles
        .iter()
        .map(|(i, _)| *i)
        .collect();
    let changes: Vec<_> = game
        .dispatcher
        .known_terrain
        .iter()
        .filter_map(|(&i, (serial, object))| {
            if hidden.contains(&i) {
                return None;
            }
            let tile = game.world.tiles.get(i);
            (tile.is_none_or(|tile| {
                tile.object.is_empty() || tile.creation_serial != *serial || tile.object != *object
            }))
            .then_some(i)
        })
        .collect();
    for i in changes {
        remove_registration(game, Instance::Terrain(i));
        game.dispatcher.known_terrain.remove(&i);
        game.data.remove(&format!("traps_create_terrain:{i}"));
        let prefix = format!("hazard:{i}:");
        game.data.retain(|key, _| !key.starts_with(&prefix));
    }
    let known: BTreeSet<_> = game
        .dispatcher
        .active_order
        .iter()
        .chain(&game.dispatcher.inactive_order)
        .copied()
        .collect();
    let mut births = Vec::new();
    for (i, t) in game.world.tiles.iter().enumerate() {
        if !t.object.is_empty() && !known.contains(&Instance::Terrain(i)) {
            births.push((t.creation_serial, Instance::Terrain(i)));
        }
    }
    if !known.contains(&Instance::Player) {
        births.push((0, Instance::Player));
    }
    for e in &game.world.entities {
        if !known.contains(&Instance::Entity(e.id)) {
            births.push((e.creation_serial, Instance::Entity(e.id)));
        }
    }
    births.sort_by_key(|(serial, _)| *serial);
    for (_, instance) in births {
        if game.dispatcher.active_order.contains(&instance)
            || game.dispatcher.inactive_order.contains(&instance)
        {
            continue;
        }
        let key = resource(game, instance);
        game.dispatcher.active_order.push(instance);
        game.dispatcher.registration_order.push(instance);
        game.dispatcher
            .resource_order
            .entry(key)
            .or_default()
            .push(instance);
        if let Instance::Terrain(i) = instance {
            let t = &game.world.tiles[i];
            game.dispatcher
                .known_terrain
                .insert(i, (t.creation_serial, t.object.clone()));
            crate::traps::create(game, instance);
        }
    }
    // instance_change moves the existing identity into its new resource list.
    let migrations: Vec<_> = game
        .dispatcher
        .resource_order
        .iter()
        .flat_map(|(&key, list)| list.iter().filter_map(move |&i| Some((key, i))))
        .filter(|(key, i)| {
            !matches!(i,Instance::Entity(id) if !game.world.entities.iter().any(|e|e.id==*id))
                && resource(game, *i) != *key
        })
        .collect();
    for (old, i) in migrations {
        if let Some(list) = game.dispatcher.resource_order.get_mut(&old) {
            list.retain(|entry| *entry != i);
        }
        let key = resource(game, i);
        game.dispatcher
            .resource_order
            .entry(key)
            .or_default()
            .push(i);
    }
}
/// Notify the dispatcher of an explicit instance_change without a birth.
pub fn changed_instance(game: &mut Game) {
    game.dispatcher.synced_revision = None;
}

fn restore_hidden(game: &mut Game) {
    for (i, t) in std::mem::take(&mut game.dispatcher.hidden_tiles) {
        if i < game.world.tiles.len() {
            game.world.tiles[i] = t;
        }
    }
    game.world
        .entities
        .append(&mut game.dispatcher.hidden_entities);
}
fn partition(game: &mut Game) {
    let mut visible = Vec::with_capacity(game.world.entities.len());
    for e in std::mem::take(&mut game.world.entities) {
        if e.engine_active {
            visible.push(e)
        } else {
            game.dispatcher.hidden_entities.push(e)
        }
    }
    game.world.entities = visible;
    for i in 0..game.world.tiles.len() {
        if !game.world.tiles[i].engine_active && !game.world.tiles[i].object.is_empty() {
            game.dispatcher
                .hidden_tiles
                .push((i, std::mem::take(&mut game.world.tiles[i])));
        }
    }
}
fn update_membership(game: &mut Game) {
    // A membership operation may immediately rebuild source arrays. Resolve
    // identities once from its freshly mutated flags, preserving all list
    // traversal and append/compact order below.
    let active_entities: HashSet<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.engine_active)
        .map(|e| e.id)
        .collect();
    let player_active = game.player.engine_active;
    let tiles = &game.world.tiles;
    let active_now = |instance: Instance| match instance {
        Instance::Player => player_active,
        Instance::Entity(id) => active_entities.contains(&id),
        Instance::Terrain(i) => tiles
            .get(i)
            .is_some_and(|t| t.engine_active && !t.object.is_empty()),
    };
    let changed = game.dispatcher.active_order.iter().any(|i| !active_now(*i))
        || game
            .dispatcher
            .inactive_order
            .iter()
            .any(|i| active_now(*i));
    if !changed {
        partition(game);
        return;
    }
    let mut active = Vec::new();
    let mut inactive = Vec::new();
    // GM appends reactivated instances before compacting newly deactivated ones.
    for i in std::mem::take(&mut game.dispatcher.active_order) {
        if active_now(i) {
            active.push(i)
        } else {
            inactive.push(i)
        }
    }
    let mut old_inactive = std::mem::take(&mut game.dispatcher.inactive_order);
    old_inactive.retain(|i| {
        if active_now(*i) {
            active.push(*i);
            false
        } else {
            true
        }
    });
    old_inactive.extend(inactive);
    game.dispatcher.active_order = active;
    game.dispatcher.inactive_order = old_inactive;
    rebuild_resources(game);
    partition(game);
}
fn activation_contains(region: Rect, p: Vec2) -> bool {
    p.x >= region.x && p.x <= region.x + region.w && p.y >= region.y && p.y <= region.y + region.h
}
fn activation_intersects(region: Rect, b: Rect) -> bool {
    b.x <= region.x + region.w
        && b.x + b.w - 1.0 >= region.x
        && b.y <= region.y + region.h
        && b.y + b.h - 1.0 >= region.y
}
fn region_change(game: &mut Game, region: Rect, activate: bool, outside: bool) {
    restore_hidden(game);
    sync_newborns(game);
    let player_bounds = game.player.sprite_bounds();
    let player_hit = if player_bounds.is_valid() {
        activation_intersects(region, player_bounds)
    } else {
        activation_contains(region, game.player.pos)
    };
    if player_hit != outside {
        game.player.engine_active = activate;
    }
    let mut overrides = vec![None; game.world.tiles.len()];
    for e in &game.world.entities {
        if e.alive {
            if let Some(index) = metadata_index(game, e) {
                if overrides[index].is_none() {
                    if let Some(sprite) = e.text.get("terrain_sprite") {
                        overrides[index] = Some((sprite.as_str(), e.animation));
                    }
                }
            }
        }
    }
    let tile_hit: Vec<_> = game
        .world
        .tiles
        .iter()
        .enumerate()
        .map(|(i, t)| {
            let p = Instance::Terrain(i).position(game);
            let (sprite, frame) =
                overrides[i].unwrap_or_else(|| (catalog::sprite(&t.object).unwrap_or(""), 0.0));
            let b = catalog::instance_sprite_bounds(
                &t.object,
                sprite,
                p,
                frame,
                Vec2::new(1.0, 1.0),
                0.0,
            );
            if b.is_valid() {
                activation_intersects(region, b)
            } else {
                activation_contains(region, p)
            }
        })
        .collect();
    for (i, t) in game.world.tiles.iter_mut().enumerate() {
        if tile_hit[i] != outside {
            t.engine_active = activate;
        }
    }
    let entity_hit: Vec<_> = game
        .world
        .entities
        .iter()
        .map(|e| {
            metadata_index(game, e)
                .map(|i| tile_hit[i])
                .unwrap_or_else(|| {
                    let b = e.sprite_bounds();
                    if b.is_valid() {
                        activation_intersects(region, b)
                    } else {
                        activation_contains(region, e.pos)
                    }
                })
        })
        .collect();
    let stash = game.player.data.get("stash_id").copied().unwrap_or(0.0) as u64;
    for (e, hit) in game.world.entities.iter_mut().zip(entity_hit) {
        if hit != outside && !(e.id == stash && e.held && e.value("stashed") > 0.0) {
            e.engine_active = activate;
        }
    }
    update_membership(game);
}
pub fn activate_region(game: &mut Game, region: Rect) {
    region_change(game, region, true, false)
}
pub fn activate_object(game: &mut Game, object: &str) {
    restore_hidden(game);
    sync_newborns(game);
    if catalog::is_a("oPlayer1", object) {
        game.player.engine_active = true;
    }
    // The catalog and object names cannot change within these flag-only loops.
    // Resolve each distinct name once without changing either traversal order.
    let mut family_matches: HashMap<&str, bool> = HashMap::new();
    for e in &mut game.world.entities {
        if *family_matches
            .entry(e.object.as_str())
            .or_insert_with(|| catalog::is_a(&e.object, object))
        {
            e.engine_active = true;
        }
    }
    for t in &mut game.world.tiles {
        if *family_matches
            .entry(t.object.as_str())
            .or_insert_with(|| catalog::is_a(&t.object, object))
        {
            t.engine_active = true;
        }
    }
    drop(family_matches);
    update_membership(game);
}
pub fn descendant_instances(game: &Game, parent: &str) -> Vec<Instance> {
    game.dispatcher.registration_order.iter().copied().filter(|i| !matches!(i,Instance::Entity(id) if i.object(game)=="oTerrainMetadata" && game.world.entities.iter().any(|e|e.id==*id)) && catalog::is_a(&source_object(game,*i),parent)).collect()
}
fn level_step(game: &mut Game) {
    crate::progression::level_music_step(game);
    crate::camera_shake::level_step(game);
    let view = game.simulation_view_rect();
    let region = Rect::new(view.x - 96.0, view.y - 96.0, view.w + 192.0, view.h + 192.0);
    region_change(game, region, false, true);
    activate_region(game, region);
    for object in [
        "oCharacter",
        "oRope",
        "oRopeThrow",
        "oRopeTop",
        "oGame",
        "oGlobals",
        "oScreen",
        "oGamepad",
        "oExplosion",
        "oGhost",
        "oFinalBoss",
    ] {
        activate_object(game, object);
    }
    let p = game.player.pos;
    activate_region(
        game,
        Rect::new(p.x - 16.0, p.y - 16.0, p.x + 16.0, p.y + 16.0),
    );
    for (object, offset, size) in [("oBoulder", 32.0, 64.0), ("oOlmec", 16.0, 96.0)] {
        activate_object(game, object);
        if let Some(p) = game
            .world
            .entities
            .iter()
            .find(|e| e.alive && catalog::is_a(&e.object, object))
            .map(|e| e.pos)
        {
            activate_region(game, Rect::new(p.x - offset, p.y - offset, size, size));
        }
    }
    crate::world_events::lighting_tick(game);
}
/// Synchronous source Create dispatch; caller overrides happen after return.
pub(crate) fn create(game: &mut Game, id: u64) {
    #[cfg(test)]
    crate::items::explosives::lifecycle_tests::observe_create(game, id);
    crate::enemies::create_instance(game, id);
    crate::items::create_instance(game, id);
    crate::world_events::create_instance(game, id);
    crate::traps::create(game, Instance::Entity(id));
    sync_newborns(game);
}
fn visit_resources(game: &mut Game, mut callback: impl FnMut(&mut Game, Instance)) {
    let mut last = i32::MIN;
    loop {
        sync_newborns(game);
        let Some(key) = game
            .dispatcher
            .resource_order
            .range((std::ops::Bound::Excluded(last), std::ops::Bound::Unbounded))
            .next()
            .map(|(k, _)| *k)
        else {
            break;
        };
        let instances = game
            .dispatcher
            .resource_order
            .get(&key)
            .cloned()
            .unwrap_or_default();
        for i in instances {
            if alive(game, i) && is_active(game, i) {
                callback(game, i);
            }
        }
        last = key;
    }
}
fn player_step(game: &mut Game, input: &InputFrame) {
    game.player.health = game.player.health.min(99);
    game.player.bombs = game.player.bombs.min(99);
    game.player.ropes = game.player.ropes.min(99);
    crate::items::pre_player_tick(game);
    crate::world_events::sample_light_distances(game);
    crate::items::player_balance_step(game);
    crate::items::weapons::firing_tick(game);
    crate::hub::begin_title_exit(game);
    crate::player::tick(game, input);
    crate::items::player_attack_step(game, input);
    crate::hub::player_portal_actions(game, input);
    crate::economy::player_held_step(game);
    let container_consumed = crate::items::player_containers(game, input);
    // Game Over reads the physical edge independently of the consumed cached
    // action key. Containers execute before its possible room transition.
    crate::progression::player_actions(game, input);
    if matches!(
        game.state,
        crate::RunState::Transition | crate::RunState::Won
    ) {
        return;
    }
    let mut item_input = *input;
    if container_consumed {
        item_input.attack_pressed = false;
    }
    // oPlayer1 Step448–452 explicitly clears these cached edges at nonpositive
    // life, before inventory actions and the later Ankh revival action.
    if game.player.health <= 0 {
        item_input.bomb_pressed = false;
        item_input.rope_pressed = false;
    }
    if !game.is_level() {
        item_input.cycle_pressed = false;
        item_input.bomb_pressed = false;
        item_input.rope_pressed = false;
    }
    if game.player.data.get("active").copied().unwrap_or(1.0) == 0.0 {
        item_input = InputFrame::default();
    }
    crate::items::player_step(game, &item_input);
    crate::economy::player_pay_step(game, input);
    crate::traps::player_health_prelude(game);
    if crate::traps::player_hurt_enabled(game) {
        crate::traps::player_crush(game);
        crate::items::player_hurt(game);
        crate::enemies::player_hurt(game);
        crate::items::explosives::player_explosion_hurt(game);
        crate::traps::player_spears(game);
        crate::traps::player_moving_hurt(game);
        crate::traps::player_spikes(game);
    }
    crate::items::player_after_hurt(game);
    crate::player::late_hurt_step(game);
    crate::progression::player_revival(game);
    let protected = game.flags.contains("hub_title") || game.flags.contains("hub_scores");
    if game.player.health <= 0 && !protected {
        let first = game.state != crate::RunState::Dead;
        if first {
            game.player.health = 0;
        }
        crate::player::enter_dead(game);
        game.state = crate::RunState::Dead;
        if first {
            game.data.insert("death_ticks".into(), 0.0);
            game.events.push(crate::GameEvent::Died);
            game.sound("sndDie", game.player.pos);
        }
    }
    if game.state == crate::RunState::Dead
        && (game.is_level() || matches!(game.world.name.as_str(), "rSun" | "rMoon" | "rStars"))
    {
        crate::music::stop_all(game);
    }
    crate::player::finish_step(game);
    crate::items::player_collectibles(game);
    crate::progression::player_deposits(game);
}
fn normal_instance(game: &mut Game, i: Instance, input: &InputFrame) {
    match i {
        Instance::Player => player_step(game, input),
        Instance::Entity(id) => {
            let object = source_object(game, i);
            if crate::enemies::handles(&object) {
                crate::enemies::step_instance(game, id);
            } else if crate::traps::handles(&object) {
                crate::traps::step_instance(game, i);
            } else if crate::items::handles(&object) {
                crate::items::step_instance(game, id);
            } else if crate::world_events::handles(&object) {
                crate::world_events::step_instance(game, id);
            }
            if object == "oFinalBoss" {
                crate::progression::final_boss_step(game);
            }
            if object == "oTunnelMan" {
                crate::progression::tunnel_step(game, input);
            }
        }
        Instance::Terrain(_) => {
            if crate::traps::handles(i.object(game)) {
                crate::traps::step_instance(game, i);
            }
        }
    }
}
fn alarm(game: &mut Game, i: Instance, index: u8) {
    match i {
        Instance::Player => crate::player::alarm(game, index),
        Instance::Terrain(_) => crate::traps::alarm(game, i, index),
        Instance::Entity(id) => {
            let object = source_object(game, i);
            if crate::enemies::handles(&object) {
                crate::enemies::alarm(game, id, index);
            } else if crate::items::handles(&object) {
                crate::items::alarm(game, id, index);
            } else if crate::traps::handles(&object) {
                crate::traps::alarm(game, i, index);
            } else {
                crate::world_events::alarm(game, id, index);
            }
        }
    }
}
fn bounds(game: &Game, i: Instance) -> Rect {
    match i {
        Instance::Player => game.player.sprite_bounds(),
        Instance::Entity(id) => game
            .world
            .entities
            .iter()
            .find(|e| e.id == id)
            .map_or(Rect::new(0.0, 0.0, 0.0, 0.0), |e| e.sprite_bounds()),
        Instance::Terrain(n) => game
            .world
            .tile_bounds((n % game.world.width) as i32, (n / game.world.width) as i32),
    }
}
fn intersects(game: &Game, a: Instance, b: Instance) -> bool {
    match (a, b) {
        (Instance::Entity(x), Instance::Entity(y)) => game
            .world
            .entities
            .iter()
            .find(|e| e.id == x)
            .zip(game.world.entities.iter().find(|e| e.id == y))
            .is_some_and(|(a, b)| a.event_intersects(b)),
        (Instance::Player, Instance::Entity(id)) | (Instance::Entity(id), Instance::Player) => game
            .world
            .entities
            .iter()
            .find(|e| e.id == id)
            .is_some_and(|e| {
                game.player
                    .collision_mask()
                    .intersects_mask(&e.collision_mask())
            }),
        (Instance::Terrain(n), other) | (other, Instance::Terrain(n)) => {
            let mask = game
                .world
                .tile_collision_mask((n % game.world.width) as i32, (n / game.world.width) as i32);
            match other {
                Instance::Player => mask.intersects_mask(&game.player.collision_mask()),
                Instance::Entity(id) => game
                    .world
                    .entities
                    .iter()
                    .find(|e| e.id == id)
                    .is_some_and(|e| mask.intersects_mask(&e.collision_mask())),
                Instance::Terrain(m) => mask.intersects_mask(&game.world.tile_collision_mask(
                    (m % game.world.width) as i32,
                    (m / game.world.width) as i32,
                )),
            }
        }
        _ => false,
    }
}
fn collision(game: &mut Game, this: Instance, other: Instance) {
    // Resolve inherited handler existence before invoking the family adapter.
    if !collision_events::has_event(&source_object(game, this), &source_object(game, other)) {
        return;
    }
    match this {
        Instance::Player => crate::player::collision(game, other),
        Instance::Terrain(_) => crate::traps::collision(game, this, other),
        Instance::Entity(id) => {
            let object = source_object(game, this);
            if object == "oGoldDoor" && catalog::is_a(other.object(game), "oSceptre") {
                crate::progression::gold_door_collision(game);
                return;
            }
            if crate::enemies::handles(&object) {
                crate::enemies::collision(game, id, other);
            } else if crate::items::handles(&object) {
                crate::items::collision(game, id, other);
            } else if crate::traps::handles(&object) {
                crate::traps::collision(game, this, other);
            } else {
                crate::world_events::collision(game, id, other);
            }
        }
    }
}
fn source_solid(game: &Game, i: Instance) -> bool {
    let default = catalog::object(&source_object(game, i)).is_some_and(|o| o.solid);
    match i {
        Instance::Player => game
            .player
            .data
            .get("gm_solid")
            .map_or(default, |v| *v != 0.0),
        Instance::Entity(id) => game
            .world
            .entities
            .iter()
            .find(|e| e.id == id)
            .and_then(|e| e.data.get("gm_solid"))
            .map_or(default, |v| *v != 0.0),
        Instance::Terrain(_) => default,
    }
}
fn restore_previous(game: &mut Game, i: Instance) {
    match i {
        Instance::Player => game.player.pos = game.player.previous_pos,
        Instance::Entity(id) => {
            if let Some(e) = game.world.entities.iter_mut().find(|e| e.id == id) {
                e.pos = e.previous_pos;
            }
        }
        Instance::Terrain(_) => {}
    }
}
fn replay_motion(game: &mut Game, i: Instance) {
    match i {
        Instance::Player => {
            game.player.pos.x += game.player.data.get("hspeed").copied().unwrap_or(0.0);
            game.player.pos.y += game.player.data.get("vspeed").copied().unwrap_or(0.0);
        }
        Instance::Entity(id) => {
            if let Some(e) = game.world.entities.iter_mut().find(|e| e.id == id) {
                e.pos.x += e.value("hspeed");
                e.pos.y += e.value("vspeed");
            }
        }
        Instance::Terrain(_) => {}
    }
}
fn collision_pairs() -> &'static Vec<(i32, i32)> {
    static PAIRS: std::sync::OnceLock<Vec<(i32, i32)>> = std::sync::OnceLock::new();
    PAIRS.get_or_init(|| {
        let mut objects: Vec<_> = catalog::object_names()
            .filter_map(|name| catalog::object(name).map(|o| (o.id, name)))
            .collect();
        objects.sort_by_key(|(id, _)| *id);
        let mut pairs = Vec::new();
        for (index, &(aid, a)) in objects.iter().enumerate() {
            for &(bid, b) in &objects[index..] {
                if collision_events::has_event(a, b) || collision_events::has_event(b, a) {
                    pairs.push((aid, bid));
                }
            }
        }
        pairs
    })
}
fn auto_collisions(game: &mut Game) {
    sync_newborns(game);
    for &(aid, bid) in collision_pairs() {
        // No callback can create a participant while either list is empty.
        // Recheck at every pair entry so births in earlier pairs remain visible.
        if game
            .dispatcher
            .resource_order
            .get(&aid)
            .is_none_or(Vec::is_empty)
            || game
                .dispatcher
                .resource_order
                .get(&bid)
                .is_none_or(Vec::is_empty)
        {
            continue;
        }
        let first = game
            .dispatcher
            .resource_order
            .get(&aid)
            .cloned()
            .unwrap_or_default();
        for (ai, a) in first.into_iter().enumerate() {
            sync_newborns(game);
            let second = game
                .dispatcher
                .resource_order
                .get(&bid)
                .cloned()
                .unwrap_or_default();
            for (bi, b) in second.into_iter().enumerate() {
                if (aid == bid && bi < ai)
                    || a == b
                    || !alive(game, a)
                    || !alive(game, b)
                    || !is_active(game, a)
                    || !is_active(game, b)
                    || !intersects(game, a, b)
                {
                    continue;
                }
                if source_solid(game, a) || source_solid(game, b) {
                    restore_previous(game, a);
                    restore_previous(game, b);
                }
                collision(game, a, b);
                if matches!(
                    game.state,
                    crate::RunState::Transition | crate::RunState::Won
                ) {
                    return;
                }
                collision(game, b, a);
                if source_solid(game, a) || source_solid(game, b) {
                    replay_motion(game, a);
                    replay_motion(game, b);
                    if intersects(game, a, b) {
                        restore_previous(game, a);
                        restore_previous(game, b);
                    }
                }
            }
        }
    }
}
fn advance_one(game: &mut Game, i: Instance) {
    let ended = match i {
        Instance::Player => {
            let speed = game.player.image_speed();
            let count = catalog::sprite_frame_count(&game.player.sprite);
            crate::animation::advance(&mut game.player.animation, speed, count)
        }
        Instance::Entity(id) => {
            let Some(e) = game.world.entities.iter_mut().find(|e| e.id == id) else {
                return;
            };
            let sprite = e.text.get("terrain_sprite").unwrap_or(&e.sprite);
            let frames = catalog::sprite_frame_count(sprite);
            let speed = e.image_speed();
            crate::animation::advance(&mut e.animation, speed, frames)
        }
        Instance::Terrain(_) => false,
    };
    if !ended {
        return;
    }
    match i {
        Instance::Player => {
            crate::player::animation_end(game);
        }
        Instance::Entity(id) => {
            let object = source_object(game, i);
            if crate::enemies::handles(&object) {
                crate::enemies::animation_end(game, id);
            } else if crate::items::handles(&object) {
                crate::items::animation_end(game, id);
            } else if crate::traps::handles(&object) {
                crate::traps::animation_end(game, i);
            } else {
                crate::world_events::animation_end(game, id);
            }
        }
        Instance::Terrain(_) => crate::traps::animation_end(game, i),
    }
}
fn depth(game: &Game, i: Instance) -> f64 {
    match i {
        Instance::Player => game
            .player
            .data
            .get("depth")
            .copied()
            .unwrap_or_else(|| catalog::object("oPlayer1").map_or(0.0, |o| o.depth as f64)),
        Instance::Entity(id) => game
            .world
            .entities
            .iter()
            .find(|e| e.id == id)
            .and_then(|e| e.data.get("depth").copied())
            .unwrap_or_else(|| catalog::object(i.object(game)).map_or(0.0, |o| o.depth as f64)),
        Instance::Terrain(_) => catalog::object(i.object(game)).map_or(0.0, |o| o.depth as f64),
    }
}
fn sort_draw_order(game: &mut Game) {
    let mut order = std::mem::take(&mut game.dispatcher.active_order);
    order.sort_by(|a, b| depth(game, *b).total_cmp(&depth(game, *a)));
    game.dispatcher.active_order = order;
}
pub fn animate_entities(game: &mut Game) {
    let live: BTreeSet<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive)
        .map(|e| e.id)
        .collect();
    let removed = game
        .dispatcher
        .active_order
        .iter()
        .any(|i| matches!(i,Instance::Entity(id) if !live.contains(id)));
    game.dispatcher
        .active_order
        .retain(|i| !matches!(i,Instance::Entity(id) if !live.contains(id)));
    game.dispatcher
        .inactive_order
        .retain(|i| !matches!(i,Instance::Entity(id) if !live.contains(id)));
    if removed {
        rebuild_resources(game);
    }
    sync_newborns(game);
    sort_draw_order(game);
    let order = game.dispatcher.active_order.clone();
    for i in order {
        if matches!(i, Instance::Entity(_)) {
            advance_one(game, i);
        }
    }
}
fn finish_storage(game: &mut Game) {
    restore_hidden(game);
    let rank: BTreeMap<_, _> = game
        .dispatcher
        .storage_order
        .iter()
        .enumerate()
        .map(|(i, id)| (*id, i))
        .collect();
    game.world
        .entities
        .sort_by_key(|e| rank.get(&e.id).copied().unwrap_or(usize::MAX));
    game.dispatcher.running = false;
    game.dispatcher.phase = Phase::Idle;
}
/// The room's source phases. Storage order remains independent of engine lists.
pub(crate) fn step(game: &mut Game, input: &InputFrame) {
    crate::world_events::prepare_tick(game);
    crate::progression::initialize_level(game);
    if game.level_tick == 0 {
        crate::traps::initialize_level(game);
    }
    // Membership is read-only while this grid-ordered birth list is collected.
    // Use exact terrain identities; no index survives Create callbacks below.
    let registered_terrain: HashSet<_> = game
        .dispatcher
        .active_order
        .iter()
        .chain(&game.dispatcher.inactive_order)
        .filter_map(|instance| match instance {
            Instance::Terrain(index) => Some(*index),
            _ => None,
        })
        .collect();
    let new_terrain: Vec<_> = game
        .world
        .tiles
        .iter()
        .enumerate()
        .filter_map(|(i, t)| {
            (!t.object.is_empty() && !registered_terrain.contains(&i)).then_some(i)
        })
        .collect();
    drop(registered_terrain);
    for i in new_terrain {
        crate::traps::create(game, Instance::Terrain(i));
    }
    // Generation/import uses World::spawn for fixed Create defaults. Complete
    // context-dependent Create before the first alarm pass, retaining explicit
    // producer overrides through each family's idempotent Create markers.
    let mut raw_creates: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| {
            (crate::traps::handles(&e.object) && e.value("traps_create_initialized") == 0.0)
                || (crate::enemies::handles(&e.object) && e.value("enemy_context_created") == 0.0)
        })
        .map(|e| (e.creation_serial, e.id))
        .collect();
    raw_creates.sort_by_key(|&(serial, _)| serial);
    for (_, id) in raw_creates {
        crate::enemies::create_instance(game, id);
        crate::traps::create(game, Instance::Entity(id));
    }
    sync_newborns(game);
    game.dispatcher.interpolation_starts = game
        .world
        .entities
        .iter()
        .filter(|entity| !entity.engine_active && entity.previous_pos != entity.pos)
        .map(|entity| (entity.id, entity.pos))
        .collect();
    game.dispatcher.storage_order = game.world.entities.iter().map(|e| e.id).collect();
    game.dispatcher.running = true;
    partition(game);
    for i in game.dispatcher.active_order.clone() {
        match i {
            Instance::Player => game.player.previous_pos = game.player.pos,
            Instance::Entity(id) => {
                if let Some(e) = game.world.entity_mut(id) {
                    e.previous_pos = e.pos;
                    e.age = e.age.saturating_add(1);
                }
            }
            _ => {}
        }
    }
    game.previous_camera = game.camera;
    game.tick += 1;
    game.level_tick += 1;
    game.dispatcher.phase = Phase::BeginStep;
    let message_begin_timer = crate::world_events::begin_step(game);
    for index in 0..12u8 {
        game.dispatcher.phase = Phase::Alarm(index);
        let mut last = i32::MIN;
        let mut controller_pending = true;
        let mut challenge_pending = crate::hub::challenge_alarm_resource(game);
        loop {
            sync_newborns(game);
            let next = game
                .dispatcher
                .resource_order
                .range((std::ops::Bound::Excluded(last), std::ops::Bound::Unbounded))
                .next()
                .map(|(key, _)| *key);
            let Some(key) = next
                .into_iter()
                .chain(controller_pending.then_some(318))
                .chain(challenge_pending)
                .min()
            else {
                break;
            };
            if key == 318 && controller_pending {
                controller_pending = false;
                if game.alarms[index as usize] >= 0 {
                    game.alarms[index as usize] -= 1;
                    if game.alarms[index as usize] == 0 {
                        crate::progression::game_alarm(game, index as usize);
                    }
                }
            }
            if challenge_pending == Some(key) {
                challenge_pending = None;
                crate::hub::challenge_alarm(game, index);
            }
            let instances = game
                .dispatcher
                .resource_order
                .get(&key)
                .cloned()
                .unwrap_or_default();
            // A singleton keeps the original linear lookup path. For larger
            // batches only alarm-slot writes occur between callbacks, so index
            // the visible vector without crossing a callback, registration
            // pass, activation, or storage partition.
            let singleton = instances.len() == 1;
            let mut entity_indices: Option<HashMap<u64, (usize, bool, bool)>> = None;
            for i in instances {
                let alarms = if singleton {
                    if alive(game, i) && is_active(game, i) {
                        i.alarms_mut(game)
                    } else {
                        None
                    }
                } else if let Instance::Entity(id) = i {
                    let indices = entity_indices.get_or_insert_with(|| {
                        let mut indices = HashMap::with_capacity(game.world.entities.len());
                        for (position, entity) in game.world.entities.iter().enumerate() {
                            let entry =
                                indices.entry(entity.id).or_insert((position, false, false));
                            // Keep the old separate existential predicates even
                            // for public/deserialized vectors with duplicate IDs.
                            entry.1 |= entity.alive;
                            entry.2 |= entity.engine_active;
                        }
                        indices
                    });
                    indices
                        .get(&id)
                        .filter(|(_, any_alive, any_active)| *any_alive && *any_active)
                        .and_then(|&(position, _, _)| game.world.entities.get_mut(position))
                        .map(|entity| &mut entity.alarms)
                } else if alive(game, i) && is_active(game, i) {
                    i.alarms_mut(game)
                } else {
                    None
                };
                let due = alarms.is_some_and(|a| {
                    if a[index as usize] >= 0 {
                        a[index as usize] -= 1;
                        a[index as usize] == 0
                    } else {
                        false
                    }
                });
                if due {
                    // Source callbacks may remove/reinsert actors, create
                    // instances, or activate regions. No index survives one.
                    entity_indices = None;
                    alarm(game, i, index);
                }
            }
            last = key;
        }
    }
    game.dispatcher.phase = Phase::Keyboard;
    if input.music_pressed {
        crate::music::key_pressed(game);
    }
    game.dispatcher.phase = Phase::NormalStep;
    // Controllers participate in resource order alongside actual instances.
    let mut last = i32::MIN;
    let mut game_done = false;
    let mut level_done = !game.is_level();
    loop {
        sync_newborns(game);
        let next = game
            .dispatcher
            .resource_order
            .range((std::ops::Bound::Excluded(last), std::ops::Bound::Unbounded))
            .next()
            .map(|(k, _)| *k);
        let key = next
            .into_iter()
            .chain((!game_done).then_some(318))
            .chain((!level_done).then_some(553))
            .min();
        let Some(key) = key else { break };
        if key == 318 && !game_done {
            game_done = true;
            if game.is_level()
                && game.state != crate::RunState::Dead
                && game.player.state != crate::PlayerState::Dead
            {
                *game.data.entry("time".into()).or_default() += 30.0;
                *game.data.entry("xtime".into()).or_default() += 30.0;
            }
            if game.state == crate::RunState::Dead {
                *game.data.entry("death_ticks".into()).or_default() += 1.0;
            }
            crate::traps::pre_tick(game);
            crate::world_events::fluid_tick(game);
            crate::progression::game_step(game, input);
        }
        if key == 553 && !level_done {
            level_done = true;
            level_step(game);
        }
        let instances = game
            .dispatcher
            .resource_order
            .get(&key)
            .cloned()
            .unwrap_or_default();
        for i in instances {
            if alive(game, i) && is_active(game, i) {
                normal_instance(game, i, input);
                if matches!(
                    game.state,
                    crate::RunState::Transition | crate::RunState::Won
                ) {
                    finish_storage(game);
                    return;
                }
            }
        }
        last = key;
    }
    crate::hub::tick(game, input);
    if game
        .ending
        .as_ref()
        .is_some_and(|s| s.stage == crate::ending::EndingStage::Exit)
    {
        crate::ending::live_exit_step(game);
    }
    game.dispatcher.phase = Phase::BuiltinMotion;
    sync_newborns(game);
    for i in game.dispatcher.active_order.clone() {
        if !alive(game, i) {
            continue;
        }
        if let Instance::Entity(id) = i {
            if let Some(e) = game.world.entity_mut(id) {
                e.pos.x += e.value("hspeed");
                e.pos.y += e.value("vspeed");
            }
            let object = source_object(game, i);
            if crate::enemies::handles(&object) {
                crate::enemies::builtin_motion(game, id);
            } else if crate::items::handles(&object) {
                crate::items::builtin_motion(game, id);
            } else if crate::traps::handles(&object) {
                crate::traps::builtin_motion(game, i);
            } else {
                crate::world_events::builtin_motion(game, id);
            }
        }
    }
    game.dispatcher.phase = Phase::OutsideRoom;
    visit_resources(game, |game, i| {
        let b = bounds(game, i);
        let p = i.position(game);
        let outside = if b.is_valid() {
            b.x + b.w <= 0.0
                || b.x > game.world.width as f64 * 16.0
                || b.y + b.h <= 0.0
                || b.y > game.world.height as f64 * 16.0
        } else {
            p.x < 0.0
                || p.x > game.world.width as f64 * 16.0
                || p.y < 0.0
                || p.y > game.world.height as f64 * 16.0
        };
        if outside {
            if let Instance::Entity(id) = i {
                let object = source_object(game, i);
                if crate::enemies::handles(&object) {
                    crate::enemies::outside_room(game, id);
                } else if crate::traps::handles(&object) {
                    crate::traps::outside_room(game, i);
                } else if crate::items::handles(&object) {
                    crate::items::outside_room(game, id);
                } else {
                    crate::world_events::outside_room(game, id);
                }
            }
        }
    });
    game.dispatcher.phase = Phase::AutomaticCollision;
    auto_collisions(game);
    game.dispatcher.phase = Phase::EndStep;
    visit_resources(game, |game, i| match i {
        Instance::Player => crate::player::end_step(game),
        Instance::Entity(id) => crate::items::end_instance(game, id),
        Instance::Terrain(_) => {}
    });
    crate::enemies::finish_destroy_events(game);
    let destroyed = game.world.entities.iter().any(|e| !e.alive);
    game.world.entities.retain(|e| e.alive);
    game.dispatcher.active_order.retain(
        |i| !matches!(i,Instance::Entity(id) if !game.world.entities.iter().any(|e|e.id==*id)),
    );
    if destroyed {
        rebuild_resources(game);
    }
    sync_newborns(game);
    sort_draw_order(game);
    game.dispatcher.phase = Phase::Draw;
    for i in game.dispatcher.active_order.clone() {
        if i == Instance::Player
            && game.player.data.get("hidden").copied().unwrap_or(0.0) == 0.0
            && game.player.data.get("visible").copied().unwrap_or(1.0) != 0.0
        {
            crate::player::draw_event(game);
        }
        if let Instance::Entity(id) = i {
            if let Some(e) = game.world.entities.iter().find(|e| e.id == id) {
                let visible = e.data.get("visible").map_or_else(
                    || catalog::object(&e.object).is_some_and(|o| o.visible),
                    |v| *v != 0.0,
                );
                if e.alive && visible {
                    crate::enemies::draw_instance(game, id);
                    crate::items::draw_instance(game, id);
                }
            }
        }
    }
    let message_draw_timer = crate::world_events::message_draw(game);
    game.capture_draw_view();
    // Presentation-only phase records preserve timer1's final visible frame.
    // The authoritative global already contains both callback decrements.
    if let Some(view) = game.draw_view.as_mut() {
        view.data
            .insert("source_message_begin_timer".into(), message_begin_timer);
        view.data
            .insert("source_message_draw_timer".into(), message_draw_timer);
    }
    game.dispatcher.phase = Phase::Animation;
    let room_tick = game.level_tick;
    let order = game.dispatcher.active_order.clone();
    for i in order {
        advance_one(game, i);
        if game.level_tick != room_tick {
            break;
        }
    }
    if game.level_tick == room_tick {
        finish_storage(game);
    } else {
        game.dispatcher = Default::default();
        game.draw_view = None;
    }
}

#[cfg(test)]
mod skeleton_player_create_tests;
