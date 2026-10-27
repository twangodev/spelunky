//! Source resource dispatch and independent engine activation membership.
use crate::{DeathStatsSnapshot, Entity, Game, InputFrame, Rect, Tile, Vec2, catalog};
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
    /// Conservatively include inactive storage when ruling out a family query.
    pub(crate) fn has_hidden_family(&self, family: &str) -> bool {
        self.hidden_entities
            .iter()
            .any(|entity| catalog::is_a(&entity.object, family))
            || self
                .hidden_tiles
                .iter()
                .any(|(_, tile)| catalog::is_a(&tile.object, family))
    }

    #[cfg(test)]
    pub(crate) fn hide_tile_for_query_test(&mut self, index: usize, tile: Tile) {
        self.hidden_tiles.push((index, tile));
    }

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CachedEntityIndex {
    Unique(usize),
    Duplicate,
    Missing,
}

/// An automatic-collision-local acceleration for identity lookups.
///
/// The cache stores vector indices rather than references, so callbacks may
/// mutate or append entities without invalidating a borrowed pointer. Every
/// hit validates the current ID at that index. Structural changes which can
/// be recognized cheaply are reconciled before lookup; otherwise a stale hit
/// falls back to the same ordered linear search used by the uncached path.
#[derive(Default)]
struct CollisionEntityIndex {
    by_id: HashMap<u64, usize>,
    duplicate_ids: HashSet<u64>,
    entity_len: usize,
    next_id: u64,
}

impl CollisionEntityIndex {
    fn new(world: &crate::World) -> Self {
        let mut cache = Self::default();
        cache.rebuild(world);
        cache
    }

    fn rebuild(&mut self, world: &crate::World) {
        self.by_id.clear();
        self.duplicate_ids.clear();
        for (index, entity) in world.entities.iter().enumerate() {
            self.register(index, entity.id);
        }
        self.entity_len = world.entities.len();
        self.next_id = world.next_id;
    }

    fn register(&mut self, index: usize, id: u64) {
        if self.duplicate_ids.contains(&id) {
            return;
        }
        if self.by_id.remove(&id).is_some() {
            self.duplicate_ids.insert(id);
        } else {
            self.by_id.insert(id, index);
        }
    }

    fn synchronize(&mut self, world: &crate::World) {
        let Some(appended) = world.entities.len().checked_sub(self.entity_len) else {
            self.rebuild(world);
            return;
        };
        let Some(spawned) = world.next_id.checked_sub(self.next_id) else {
            self.rebuild(world);
            return;
        };

        // Spawns append one entity and advance next_id once. A mismatch can
        // mean removals/reordering around births, so rebuild before using any
        // cached slot. Direct appenders with unchanged next_id are reconciled
        // from the new suffix below.
        if world.next_id != self.next_id && usize::try_from(spawned).ok() != Some(appended) {
            self.rebuild(world);
            return;
        }

        for (index, entity) in world.entities.iter().enumerate().skip(self.entity_len) {
            self.register(index, entity.id);
        }
        self.entity_len = world.entities.len();
        self.next_id = world.next_id;
    }

    fn locate(&mut self, world: &crate::World, id: u64) -> CachedEntityIndex {
        self.synchronize(world);
        if self.duplicate_ids.contains(&id) {
            return CachedEntityIndex::Duplicate;
        }
        if let Some(index) = self.by_id.get(&id).copied()
            && world
                .entities
                .get(index)
                .is_some_and(|entity| entity.id == id)
        {
            return CachedEntityIndex::Unique(index);
        }

        // A remove, sort, or other in-place edit can leave a stale index even
        // when vector length and next_id are unchanged. Recover using the
        // original first-match order and detect duplicates on this slow path.
        let mut found = None;
        for (index, entity) in world.entities.iter().enumerate() {
            if entity.id != id {
                continue;
            }
            if found.is_some() {
                self.by_id.remove(&id);
                self.duplicate_ids.insert(id);
                return CachedEntityIndex::Duplicate;
            }
            found = Some(index);
        }
        match found {
            Some(index) => {
                self.by_id.insert(id, index);
                CachedEntityIndex::Unique(index)
            }
            None => {
                self.by_id.remove(&id);
                CachedEntityIndex::Missing
            }
        }
    }
}

fn alive_cached(game: &Game, cache: &mut CollisionEntityIndex, instance: Instance) -> bool {
    match instance {
        Instance::Entity(id) => match cache.locate(&game.world, id) {
            CachedEntityIndex::Unique(index) => game.world.entities[index].alive,
            CachedEntityIndex::Duplicate => alive(game, Instance::Entity(id)),
            CachedEntityIndex::Missing => false,
        },
        _ => alive(game, instance),
    }
}

fn active_cached(game: &Game, cache: &mut CollisionEntityIndex, instance: Instance) -> bool {
    match instance {
        Instance::Entity(id) => match cache.locate(&game.world, id) {
            CachedEntityIndex::Unique(index) => game.world.entities[index].engine_active,
            CachedEntityIndex::Duplicate => is_active(game, Instance::Entity(id)),
            CachedEntityIndex::Missing => false,
        },
        Instance::Player => game.player.engine_active,
        Instance::Terrain(index) => game
            .world
            .tiles
            .get(index)
            .is_some_and(|tile| tile.engine_active && !tile.object.is_empty()),
    }
}

#[cfg(test)]
mod presentation_storage_tests {
    use super::*;
    use crate::{Area, PlayerState, RulesPreset, World};

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

/// Retire the player instance from the live engine lists while retaining the
/// Player model for host and ending state. `instance_destroy()` is synchronous:
/// the current Player Step can finish, but later phases must not visit it.
pub(crate) fn destroy_player_instance(game: &mut Game) {
    game.player.engine_active = false;
    update_membership(game);
}

fn activation_contains(region: Rect, p: Vec2) -> bool {
    p.x >= region.x && p.x <= region.x + region.w && p.y >= region.y && p.y <= region.y + region.h
}
struct RegionHits {
    player: bool,
    tiles: Vec<bool>,
    entities: Vec<bool>,
}
fn classify_region(game: &Game, region: Rect) -> RegionHits {
    let player_bounds = game.player.collision_mask().point_bounds();
    let player_hit = if player_bounds.is_valid() {
        crate::activation::source_region_overlaps_bbox(region, player_bounds)
    } else {
        activation_contains(region, game.player.pos)
    };
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
            let b = catalog::instance_mask(&t.object, sprite, p, frame, Vec2::new(1.0, 1.0), 0.0)
                .point_bounds();
            if b.is_valid() {
                crate::activation::source_region_overlaps_bbox(region, b)
            } else {
                activation_contains(region, p)
            }
        })
        .collect();
    let entity_hit: Vec<_> = game
        .world
        .entities
        .iter()
        .map(|e| {
            metadata_index(game, e)
                .map(|i| tile_hit[i])
                .unwrap_or_else(|| {
                    let b = e.collision_mask().point_bounds();
                    if b.is_valid() {
                        crate::activation::source_region_overlaps_bbox(region, b)
                    } else {
                        activation_contains(region, e.pos)
                    }
                })
        })
        .collect();
    RegionHits {
        player: player_hit,
        tiles: tile_hit,
        entities: entity_hit,
    }
}
fn apply_region_hits(
    game: &mut Game,
    hits: &RegionHits,
    activate: bool,
    outside: bool,
    entity_order: Option<&[usize]>,
) {
    if hits.player != outside {
        game.player.engine_active = activate;
    }
    for (i, t) in game.world.tiles.iter_mut().enumerate() {
        if hits.tiles[i] != outside {
            t.engine_active = activate;
        }
    }
    let stash = game.player.data.get("stash_id").copied().unwrap_or(0.0) as u64;
    let update_entity = |e: &mut Entity, hit: bool| {
        if hit != outside && !(e.id == stash && e.held && e.value("stashed") > 0.0) {
            e.engine_active = activate;
        }
    };
    if let Some(order) = entity_order {
        for (e, &original_index) in game.world.entities.iter_mut().zip(order) {
            update_entity(e, hits.entities[original_index]);
        }
    } else {
        for (e, &hit) in game.world.entities.iter_mut().zip(&hits.entities) {
            update_entity(e, hit);
        }
    }
}
fn region_change(game: &mut Game, region: Rect, activate: bool, outside: bool) {
    restore_hidden(game);
    sync_newborns(game);
    let hits = classify_region(game, region);
    apply_region_hits(game, &hits, activate, outside, None);
    update_membership(game);
}

/// Reuse the expensive hit classification for the adjacent source deactivate
/// and activate calls in oLevel Step. Keep both membership updates: the source
/// engine appends reactivated instances and compacts deactivated ones between
/// these calls, which affects later event order.
fn level_region_change(game: &mut Game, region: Rect) {
    restore_hidden(game);
    sync_newborns(game);

    // Multiple metadata overlays can compete to provide a tile's mask sprite.
    // Partition/restore may reorder those overlays between source calls, so
    // retain the original independent classifications for that edge case.
    let mut metadata_tiles = HashSet::new();
    let single_overlay_per_tile = game.world.entities.iter().all(|entity| {
        metadata_index(game, entity).is_none_or(|index| metadata_tiles.insert(index))
    });
    if !single_overlay_per_tile {
        region_change(game, region, false, true);
        region_change(game, region, true, false);
        return;
    }

    let hits = classify_region(game, region);
    let stash = game.player.data.get("stash_id").copied().unwrap_or(0.0) as u64;
    let first_active: Vec<_> = game
        .world
        .entities
        .iter()
        .zip(&hits.entities)
        .map(|(entity, &hit)| {
            if !hit && !(entity.id == stash && entity.held && entity.value("stashed") > 0.0) {
                false
            } else {
                entity.engine_active
            }
        })
        .collect();
    let mut entity_order = Vec::with_capacity(first_active.len());
    entity_order.extend(
        first_active
            .iter()
            .enumerate()
            .filter_map(|(index, &active)| active.then_some(index)),
    );
    entity_order.extend(
        first_active
            .iter()
            .enumerate()
            .filter_map(|(index, &active)| (!active).then_some(index)),
    );

    apply_region_hits(game, &hits, false, true, None);
    update_membership(game);
    restore_hidden(game);
    sync_newborns(game);
    apply_region_hits(game, &hits, true, false, Some(&entity_order));
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
/// Reactivate every room instance, matching GameMaker's `instance_activate_all`.
/// Pause resume calls this before the next Alarm phase; the level controller
/// can cull offscreen instances again later in that same tick.
pub fn activate_all(game: &mut Game) {
    restore_hidden(game);
    sync_newborns(game);
    game.player.engine_active = true;
    for entity in &mut game.world.entities {
        entity.engine_active = true;
    }
    for tile in &mut game.world.tiles {
        if !tile.object.is_empty() {
            tile.engine_active = true;
        }
    }
    update_membership(game);
}
pub fn descendant_instances(game: &Game, parent: &str) -> Vec<Instance> {
    // The query has no callbacks: index this immutable storage once instead of
    // searching all actors for every registered identity. Retain both first
    // physical and first live matches, as the existing lookup APIs do.
    let mut entities: HashMap<u64, (&Entity, Option<&Entity>, bool)> = HashMap::new();
    for entity in &game.world.entities {
        let entry = entities.entry(entity.id).or_insert((entity, None, true));
        if entry.1.is_none() && entity.alive {
            entry.1 = Some(entity);
        }
    }
    for entity in &game.dispatcher.hidden_entities {
        entities.entry(entity.id).or_insert((entity, None, false));
    }
    game.dispatcher
        .registration_order
        .iter()
        .copied()
        .filter(|instance| {
            let object = match instance {
                Instance::Entity(id) => {
                    let Some((first, live, physical)) = entities.get(id) else {
                        return catalog::is_a("", parent);
                    };
                    if first.object == "oTerrainMetadata" && *physical {
                        return false;
                    }
                    live.and_then(|entity| {
                        if entity.object != "oTerrainMetadata" {
                            return Some(entity.object.as_str());
                        }
                        game.world
                            .tile(
                                (entity.pos.x / 16.0).floor() as i32,
                                (entity.pos.y / 16.0).floor() as i32,
                            )
                            .map(|tile| tile.object.as_str())
                            .filter(|object| matches!(*object, "oTree" | "oLava"))
                    })
                    .unwrap_or(first.object.as_str())
                }
                _ => instance.object(game),
            };
            catalog::is_a(object, parent)
        })
        .collect()
}
#[cfg(test)]
mod descendant_query_tests {
    use super::*;
    use crate::{Area, RulesPreset, World};

    // Retain the pre-index lookup as an independent ordering/identity oracle.
    fn original(game: &Game, parent: &str) -> Vec<Instance> {
        game.dispatcher
            .registration_order
            .iter()
            .copied()
            .filter(|i| {
                !matches!(i, Instance::Entity(id) if i.object(game) == "oTerrainMetadata"
                && game.world.entities.iter().any(|e| e.id == *id))
                    && catalog::is_a(&source_object(game, *i), parent)
            })
            .collect()
    }

    #[test]
    fn indexed_family_queries_preserve_order_hidden_storage_and_duplicate_identities() {
        let mut game = Game::shell(7, RulesPreset::Classic);
        game.world = World::new(16, 12, Area::Mines, 1);
        game.world.set_tile(2, 3, "oLava");
        game.world.set_tile(3, 3, "oTree");
        game.world.entities = vec![
            Entity::new(1, "oBomb", Vec2::new(40.0, 56.0)),
            Entity::new(2, "oTerrainMetadata", Vec2::new(40.0, 56.0)),
            Entity::new(3, "oRock", Vec2::default()),
            Entity::new(3, "oTerrainMetadata", Vec2::new(56.0, 56.0)),
            Entity::new(4, "oSnake", Vec2::default()),
        ];
        game.world.entities[2].alive = false;
        game.dispatcher.hidden_entities = vec![
            Entity::new(1, "oSnake", Vec2::default()),
            Entity::new(5, "oTerrainMetadata", Vec2::default()),
            Entity::new(6, "oBomb", Vec2::default()),
        ];
        game.dispatcher.registration_order = vec![
            Instance::Entity(6),
            Instance::Player,
            Instance::Entity(3),
            Instance::Terrain(50),
            Instance::Entity(2),
            Instance::Entity(1),
            Instance::Entity(4),
            Instance::Entity(5),
            Instance::Entity(999),
            Instance::Entity(1),
        ];
        for reversed in [false, true] {
            if reversed {
                game.world.entities.reverse();
            }
            for parent in [
                "oItem",
                "oEnemy",
                "oSolid",
                "oTree",
                "oLava",
                "oTerrainMetadata",
                "oPlayer1",
                "",
                "unknown",
            ] {
                assert_eq!(
                    descendant_instances(&game, parent),
                    original(&game, parent),
                    "family={parent}, reversed={reversed}"
                );
            }
        }
    }
}

fn level_step(game: &mut Game) {
    crate::progression::level_music_step(game);
    crate::camera_shake::level_step(game);
    let view = game.simulation_view_rect();
    let region = Rect::new(view.x - 96.0, view.y - 96.0, view.w + 192.0, view.h + 192.0);
    level_region_change(game, region);
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
    game.player.health = game.player.health.min(99.0);
    game.player.bombs = game.player.bombs.min(99.0);
    game.player.ropes = game.player.ropes.min(99.0);
    // oPlayer1 Step creates its independent Cape actor before the controller
    // runs. Its resource (741) follows Player (319), so the newborn receives
    // its own Step later in this same resource-ordered pass.
    if (game.player.has("Cape") || game.player.has("cape"))
        && !game
            .world
            .entities
            .iter()
            .any(|entity| entity.alive && entity.object == "oCape")
    {
        game.spawn("oCape", game.player.pos);
    }
    crate::items::pre_player_tick(game);
    crate::world_events::sample_light_distances(game);
    crate::items::player_balance_step(game);
    crate::items::weapons::firing_tick(game);
    crate::hub::begin_title_exit(game);
    crate::player::tick(game, input);
    crate::items::player_attack_step(game, input);
    crate::economy::player_held_step(game);
    let container_consumed = crate::items::player_containers(game, input);
    // Classic resolves held-item theft and chest/crate interaction at the
    // Player's current origin before its oXStart portal branch snaps the actor
    // into the exit animation. Keep the authored hub/tutorial portal after
    // those source actions, just like the ordinary oExit branch below.
    crate::hub::player_portal_actions(game);
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
    // Classic's Player Action code reads the controller variables sampled by
    // characterStepEvent. DUCKTOHANG and exit animations skip that controller,
    // so presses during them must not bypass the retained source values.
    let cached_key = |name: &str| game.player.data.get(name).copied().unwrap_or(0.0) >= 0.5;
    item_input.attack = cached_key("source_k_attack");
    item_input.attack_pressed = cached_key("source_k_attack_pressed");
    item_input.attack_released = cached_key("source_k_attack_released");
    item_input.cycle_pressed = cached_key("source_k_item_pressed");
    item_input.down = cached_key("source_k_down");
    item_input.up = cached_key("source_k_up");
    if container_consumed {
        item_input.attack_pressed = false;
    }
    // oPlayer1 Step448–452 explicitly clears these cached edges at nonpositive
    // life, before inventory actions and the later Ankh revival action.
    if game.player.health <= 0.0 {
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
    let player_action_aborted = crate::items::player_actions_before_pay(game, &item_input);
    if player_action_aborted {
        // Classic's Player Step is one code action. A runtime error in the
        // pickup branch aborts the rest of that event, while the dispatcher
        // still advances later object resources in the outer engine tick.
        return;
    }
    crate::economy::player_pay_step(game, input);
    crate::items::player_step_after_pay(game, &item_input);
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
    if game.player.health < 1.0 && !protected {
        let first = game.state != crate::RunState::Dead;
        if first {
            game.player.health = 0.0;
            crate::hub::player_death_step(game);
            game.death_stats_snapshot = Some(DeathStatsSnapshot {
                money: game.player.money,
                kills: game.data.get("kills").copied().unwrap_or(0.0).max(0.0) as u32,
                saves: game.data.get("damsels").copied().unwrap_or(0.0).max(0.0) as u32,
            });
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
                crate::progression::final_boss_step(game, Some(id));
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
                // oDamsel is dispatched through the NPC adapter, but its
                // source parent is oItem and it inherits oItem Alarm 2.
                if catalog::is_a(&object, "oItem") {
                    crate::items::alarm(game, id, index);
                }
            } else if crate::items::handles(&object) {
                crate::items::alarm(game, id, index);
            } else if crate::traps::handles(&object) {
                crate::traps::alarm(game, i, index);
                // oBall/oBall2 use the curse adapter but retain their oItem
                // Alarm events through their source parent relationship.
                if catalog::is_a(&object, "oItem") {
                    crate::items::alarm(game, id, index);
                }
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
#[cfg(test)]
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
fn cached_entity<'a>(
    world: &'a crate::World,
    lookup: CachedEntityIndex,
    id: u64,
) -> Option<&'a Entity> {
    match lookup {
        CachedEntityIndex::Unique(index) => {
            world.entities.get(index).filter(|entity| entity.id == id)
        }
        CachedEntityIndex::Duplicate => world.entities.iter().find(|entity| entity.id == id),
        CachedEntityIndex::Missing => None,
    }
}
fn intersects_cached(
    game: &Game,
    a: Instance,
    b: Instance,
    cache: &mut CollisionEntityIndex,
) -> bool {
    match (a, b) {
        (Instance::Entity(x), Instance::Entity(y)) => {
            let x_lookup = cache.locate(&game.world, x);
            let y_lookup = cache.locate(&game.world, y);
            cached_entity(&game.world, x_lookup, x)
                .zip(cached_entity(&game.world, y_lookup, y))
                .is_some_and(|(left, right)| left.event_intersects(right))
        }
        (Instance::Player, Instance::Entity(id)) | (Instance::Entity(id), Instance::Player) => {
            let lookup = cache.locate(&game.world, id);
            cached_entity(&game.world, lookup, id).is_some_and(|entity| {
                game.player
                    .collision_mask()
                    .intersects_mask(&entity.collision_mask())
            })
        }
        (Instance::Terrain(index), other) | (other, Instance::Terrain(index)) => {
            let mask = game.world.tile_collision_mask(
                (index % game.world.width) as i32,
                (index / game.world.width) as i32,
            );
            match other {
                Instance::Player => mask.intersects_mask(&game.player.collision_mask()),
                Instance::Entity(id) => {
                    let lookup = cache.locate(&game.world, id);
                    cached_entity(&game.world, lookup, id)
                        .is_some_and(|entity| mask.intersects_mask(&entity.collision_mask()))
                }
                Instance::Terrain(other_index) => {
                    mask.intersects_mask(&game.world.tile_collision_mask(
                        (other_index % game.world.width) as i32,
                        (other_index / game.world.width) as i32,
                    ))
                }
            }
        }
        _ => false,
    }
}
fn collision_resolved(
    game: &mut Game,
    this: Instance,
    other: Instance,
    this_object: &str,
    other_object: &str,
) {
    // Resolve inherited handler existence before invoking the family adapter.
    if !collision_events::has_event(this_object, other_object) {
        return;
    }
    match this {
        Instance::Player => crate::player::collision(game, other),
        Instance::Terrain(_) => crate::traps::collision(game, this, other),
        Instance::Entity(id) => {
            if this_object == "oGoldDoor" && catalog::is_a(other.object(game), "oSceptre") {
                crate::progression::gold_door_collision(game, id, other);
                return;
            }
            if crate::enemies::handles(this_object) {
                crate::enemies::collision(game, id, other);
            } else if crate::items::handles(this_object) {
                crate::items::collision(game, id, other);
            } else if crate::traps::handles(this_object) {
                crate::traps::collision(game, this, other);
            } else {
                crate::world_events::collision(game, id, other);
            }
        }
    }
}
fn automatic_collision_pair(
    game: &mut Game,
    a: Instance,
    b: Instance,
    entity_indices: &mut CollisionEntityIndex,
) {
    let a_object = source_object(game, a);
    let b_object = source_object(game, b);
    if !collision_events::has_event(&a_object, &b_object)
        && !collision_events::has_event(&b_object, &a_object)
    {
        return;
    }
    let solid_before_events = source_solid_with_object(game, a, &a_object, entity_indices)
        || source_solid_with_object(game, b, &b_object, entity_indices);
    if solid_before_events {
        // The source collision manager rewinds both participants before it
        // dispatches either directional event when either member is solid.
        restore_previous(game, a);
        restore_previous(game, b);
    }

    // Some source classes are derived from the instance's current position
    // (notably oTerrainMetadata over a tree or lava tile). Reuse the original
    // names only when there was no rewind; after a rewind, Classic resolves
    // the directional event against the restored positions.
    let (a_before_first, b_before_first) = if solid_before_events {
        (source_object(game, a), source_object(game, b))
    } else {
        (a_object, b_object)
    };

    // Both callbacks run in pair order against the same rewound positions.
    // The second callback observes any changes made by the first, including
    // synchronous destroy-event effects.
    collision_resolved(game, a, b, &a_before_first, &b_before_first);
    let b_after_first = source_object(game, b);
    let a_after_first = source_object(game, a);
    collision_resolved(game, b, a, &b_after_first, &a_after_first);

    // Classic re-reads solidity after both callbacks. Its built-in response
    // advances each surviving participant by its current hspeed/vspeed, then
    // restores both previous positions if the solid pair still overlaps.
    let a_after_events = source_object(game, a);
    let solid_after_events = source_solid_with_object(game, a, &a_after_events, entity_indices)
        || {
            let b_after_events = source_object(game, b);
            source_solid_with_object(game, b, &b_after_events, entity_indices)
        };
    if solid_after_events {
        replay_motion(game, a);
        replay_motion(game, b);
        if alive_cached(game, entity_indices, a)
            && alive_cached(game, entity_indices, b)
            && intersects_cached(game, a, b, entity_indices)
        {
            restore_previous(game, a);
            restore_previous(game, b);
        }
    }
}
#[cfg(test)]
fn source_solid(game: &Game, i: Instance) -> bool {
    let object = source_object(game, i);
    let gm_solid = match i {
        Instance::Player => game.player.data.get("gm_solid").copied(),
        Instance::Entity(id) => game
            .world
            .entities
            .iter()
            .find(|e| e.id == id)
            .and_then(|e| e.data.get("gm_solid").copied()),
        Instance::Terrain(_) => None,
    };
    source_solid_value(&object, gm_solid)
}
fn source_solid_with_object(
    game: &Game,
    i: Instance,
    object: &str,
    entity_indices: &mut CollisionEntityIndex,
) -> bool {
    let gm_solid = match i {
        Instance::Player => game.player.data.get("gm_solid").copied(),
        Instance::Entity(id) => {
            let lookup = entity_indices.locate(&game.world, id);
            cached_entity(&game.world, lookup, id).and_then(|e| e.data.get("gm_solid").copied())
        }
        Instance::Terrain(_) => None,
    };
    source_solid_value(object, gm_solid)
}
fn source_solid_value(object: &str, gm_solid: Option<f64>) -> bool {
    let default = catalog::object(object).is_some_and(|o| o.solid);
    gm_solid.map_or(default, |v| v != 0.0)
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
    let mut entity_indices = CollisionEntityIndex::new(&game.world);
    for &(aid, bid) in collision_pairs() {
        // Classic captures the outer class count once for this pair, then
        // queries the inner class count again before each outer instance's
        // loop. Reconcile prior-pair births here and births from earlier
        // outer instances before building each inner list.
        sync_newborns(game);
        let first = game
            .dispatcher
            .resource_order
            .get(&aid)
            .cloned()
            .unwrap_or_default();
        if first.is_empty() {
            continue;
        }
        let mut second = Vec::new();
        for (ai, a) in first.into_iter().enumerate() {
            // GameMaker re-reads instance_number(B) for each outer A, so a
            // collision callback that creates B participates from the next
            // outer iteration onward, but never extends this inner loop. Keep
            // this identity snapshot stable while callbacks mutate and
            // compact resource_order; reuse its allocation between A values.
            sync_newborns(game);
            second.clear();
            if let Some(instances) = game.dispatcher.resource_order.get(&bid) {
                second.extend_from_slice(instances);
            }
            for (bi, b) in second.iter().copied().enumerate() {
                if (aid == bid && bi < ai)
                    || a == b
                    || !alive_cached(game, &mut entity_indices, a)
                    || !alive_cached(game, &mut entity_indices, b)
                    || !active_cached(game, &mut entity_indices, a)
                    || !active_cached(game, &mut entity_indices, b)
                    || !intersects_cached(game, a, b, &mut entity_indices)
                {
                    continue;
                }
                automatic_collision_pair(game, a, b, &mut entity_indices);
                if matches!(
                    game.state,
                    crate::RunState::Transition | crate::RunState::Won
                ) {
                    return;
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
    // Classic sorts the global active list in place before Draw. Object event
    // arrays remain independent until an activation change rebuilds them from
    // this newly sorted global order.
    let mut order = std::mem::take(&mut game.dispatcher.active_order);
    order.sort_by(|a, b| depth(game, *b).total_cmp(&depth(game, *a)));
    game.dispatcher.active_order = order;
}

fn animation_end_order(game: &Game) -> Vec<Instance> {
    // Classic runs Animation End against the same manager array it sorted by
    // depth for Draw. The registration ledger is creation/activation order and
    // can differ after that in-place sort.
    game.dispatcher.active_order.clone()
}

pub fn animate_entities(game: &mut Game) {
    let live: BTreeSet<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive)
        .map(|e| e.id)
        .collect();
    let removed: Vec<_> = game
        .dispatcher
        .active_order
        .iter()
        .chain(&game.dispatcher.inactive_order)
        .filter_map(|instance| match instance {
            Instance::Entity(id) if !live.contains(id) => Some(*id),
            _ => None,
        })
        .collect();
    for id in &removed {
        remove_registration(game, Instance::Entity(*id));
    }
    sync_newborns(game);
    // Opening callers normally discard dead storage before reaching Draw, but
    // keep this boundary correct for any instance already registered as dead.
    for id in removed {
        remove_registration(game, Instance::Entity(id));
    }
    sort_draw_order(game);
    for i in game.dispatcher.active_order.clone() {
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
            crate::progression::game_step(game, input);
            crate::world_events::fluid_tick(game);
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
    let destroyed: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|entity| !entity.alive)
        .map(|entity| entity.id)
        .collect();
    game.world.entities.retain(|e| e.alive);
    for id in destroyed {
        remove_registration(game, Instance::Entity(id));
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
    // Draw has depth-sorted active_order in place, and Classic's Animation End
    // pass traverses that same active manager array.
    let animation_order = animation_end_order(game);
    for i in animation_order {
        if !game.dispatcher.active_order.contains(&i) {
            continue;
        }
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

#[cfg(test)]
mod automatic_collision_tests {
    use super::*;
    use crate::{Area, PlayerState, RulesPreset, World};

    #[test]
    fn collision_pair_refreshes_inner_class_before_each_outer_instance() {
        let mut game = Game::shell(5, RulesPreset::Classic);
        game.world = World::new(24, 18, Area::Mines, 1);
        game.data.insert("timer".into(), 70.0);
        game.data.insert("baskets".into(), 0.0);

        // The source Moon target's Arrow Collision creates a replacement at
        // (xMid, -64). A second Arrow is already waiting at that destination.
        // Classic captures the arrow count once, then refreshes the target
        // count for the second arrow. The replacement created by the first
        // collision is therefore hit during this same resource pair.
        let target = game.spawn("oBatTarget", Vec2::new(120.0, 120.0));
        let first_arrow = game.spawn("oArrow", Vec2::new(122.0, 124.0));
        let later_arrow = game.spawn("oArrow", Vec2::new(122.0, -60.0));
        game.dispatcher = State::default();
        register_instances(&mut game);
        let arrow_class = catalog::object("oArrow").unwrap().id;
        let target_class = catalog::object("oBatTarget").unwrap().id;
        assert!(collision_pairs().contains(&(arrow_class, target_class)));
        assert!(is_active(&game, Instance::Entity(first_arrow)));
        assert!(intersects(
            &game,
            Instance::Entity(first_arrow),
            Instance::Entity(target)
        ));

        auto_collisions(&mut game);

        assert_eq!(game.data["baskets"], 2.0);
        assert!(game.world.entity(target).is_none());
        assert!(game.world.entity(first_arrow).is_none());
        assert!(game.world.entity(later_arrow).is_none());
        let live_targets: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|entity| entity.alive && entity.object == "oBatTarget")
            .collect();
        assert_eq!(live_targets.len(), 1);
        assert_eq!(live_targets[0].pos, Vec2::new(120.0, -64.0));
    }

    #[test]
    fn half_turn_arrow_triggers_web_ball_item_collision() {
        let mut game = Game::shell(5, RulesPreset::Classic);
        game.world = World::new(24, 18, Area::Mines, 1);
        let web_ball = game.spawn("oWebBall", Vec2::new(94.0, 103.0));
        let arrow = game.spawn("oArrow", Vec2::new(100.0, 100.0));
        // oArrow Step's xVel<0, yVel=0 branch sets image_angle=180.
        game.world
            .entity_mut(arrow)
            .unwrap()
            .data
            .insert("image_angle".into(), 180.0);
        game.dispatcher = State::default();
        register_instances(&mut game);

        assert!(collision_events::has_event("oWebBall", "oItem"));
        assert!(intersects(
            &game,
            Instance::Entity(arrow),
            Instance::Entity(web_ball)
        ));
        auto_collisions(&mut game);

        let web_ball = game.world.entity(web_ball).unwrap();
        assert_eq!(web_ball.sprite, "sWebCreate");
        assert_eq!(web_ball.state, 1);
    }

    #[test]
    fn one_sided_collision_with_solid_pair_rewinds_both_participants() {
        let mut game = Game::shell(5, RulesPreset::Classic);
        game.world = World::new(24, 18, Area::Mines, 1);
        game.player.pos = Vec2::new(24.0, 24.0);
        game.player.state = PlayerState::Standing;

        // oRuby is solid and oWeb owns the one-sided Collision event. Classic
        // Classic rewinds both objects before Web's one-sided collision event.
        // The event zeros Ruby's velocity, then the pair-level solid response
        // restores both objects when they still overlap.
        let web = game.spawn("oWeb", Vec2::new(120.0, 120.0));
        let ruby = game.spawn("oRuby", Vec2::new(120.0, 120.0));
        game.world.entity_mut(ruby).unwrap().vel = Vec2::new(1.0, 0.0);
        game.world
            .entity_mut(ruby)
            .unwrap()
            .data
            .insert("xVel".into(), 1.0);
        game.world
            .entity_mut(ruby)
            .unwrap()
            .data
            .insert("yVel".into(), 0.0);
        assert!(collision_events::has_event("oWeb", "oRuby"));
        assert!(!collision_events::has_event("oRuby", "oWeb"));

        game.step(&InputFrame::default());

        let ruby = game.world.entity(ruby).unwrap();
        assert!(game.world.entity(web).is_some());
        assert_eq!(
            ruby.vel,
            Vec2::default(),
            "oWeb's source Collision zeros other velocity"
        );
        assert_eq!(ruby.value("xVel"), 0.0);
        assert_eq!(ruby.value("yVel"), 0.0);
        assert_eq!(ruby.pos.x, 120.0, "the solid pair restores both positions");
        assert_eq!(
            game.world.entity(web).unwrap().pos.x,
            120.0,
            "the solid pair restores the Collision receiver too"
        );
    }

    #[test]
    fn solid_large_gem_with_nonsolid_ghost_creates_diamond_at_rewound_position() {
        let mut game = Game::shell(5, RulesPreset::Classic);
        game.world = World::new(24, 18, Area::Mines, 1);
        game.player.pos = Vec2::new(24.0, 24.0);
        game.player.state = PlayerState::Standing;

        // Big gems are solid; Ghost is not. The pair rewinds both objects
        // before the gem's one-sided Collision callback creates its Diamond.
        let ghost = game.spawn("oGhost", Vec2::new(120.0, 120.0));
        game.world.entity_mut(ghost).unwrap().state = 0;
        let gem = game.spawn("oRubyBig", Vec2::new(128.0, 128.0));
        game.world.entity_mut(gem).unwrap().vel = Vec2::new(1.0, 0.0);
        assert!(collision_events::has_event("oRubyBig", "oGhost"));
        assert!(!collision_events::has_event("oGhost", "oRubyBig"));

        game.step(&InputFrame::default());

        assert!(game.world.entity(gem).is_none());
        let diamond = game
            .world
            .entities
            .iter()
            .find(|entity| entity.alive && entity.object == "oDiamond")
            .unwrap();
        assert_eq!(
            diamond.pos.x, 128.0,
            "the callback creates the Diamond at the pair's rewound position"
        );
    }

    #[test]
    fn nonsolid_pair_keeps_post_step_position_after_collision_callback() {
        let mut game = Game::shell(5, RulesPreset::Classic);
        game.world = World::new(24, 18, Area::Mines, 1);
        game.player.pos = Vec2::new(24.0, 24.0);
        game.player.state = PlayerState::Standing;

        let web = game.spawn("oWeb", Vec2::new(120.0, 120.0));
        let arrow_id = game.spawn("oArrow", Vec2::new(120.0, 120.0));
        let arrow = game.world.entity_mut(arrow_id).unwrap();
        arrow.vel = Vec2::new(1.0, 0.0);
        arrow.data.insert("xVel".into(), 1.0);
        arrow.data.insert("yVel".into(), 0.0);
        assert!(!source_solid(&game, Instance::Entity(web)));
        assert!(!source_solid(&game, Instance::Entity(arrow_id)));
        assert!(collision_events::has_event("oWeb", "oArrow"));

        game.step(&InputFrame::default());

        let arrow = game.world.entity(arrow_id).unwrap();
        assert_eq!(arrow.pos.x, 121.0);
        assert_eq!(
            arrow.value("xVel"),
            0.0,
            "the Web collision stops the Arrow"
        );
    }
}

#[cfg(test)]
mod source_activation_bbox_tests {
    use super::*;
    use crate::{Area, RulesPreset, World};

    fn activation_fixture(duplicate_metadata: bool) -> Game {
        let mut game = Game::shell(42, RulesPreset::Classic);
        game.world = World::new(80, 40, Area::Mines, 1);
        game.player.pos = Vec2::new(160.0, 120.0);
        game.world.set_tile(6, 10, "oBrick");
        game.world.set_tile(60, 10, "oRock");

        let inside_inactive = game.spawn("oBomb", Vec2::new(96.0, 80.0));
        game.world
            .entity_mut(inside_inactive)
            .unwrap()
            .engine_active = false;
        let outside_active = game.spawn("oRock", Vec2::new(720.0, 80.0));
        let outside_inactive = game.spawn("oBomb", Vec2::new(752.0, 80.0));
        game.world
            .entity_mut(outside_inactive)
            .unwrap()
            .engine_active = false;
        let inside_active = game.spawn("oCrate", Vec2::new(240.0, 80.0));
        let stash = game.spawn("oShotgun", Vec2::new(900.0, 80.0));
        let stored = game.world.entity_mut(stash).unwrap();
        stored.held = true;
        stored.data.insert("stashed".into(), 1.0);
        game.player.data.insert("stash_id".into(), stash as f64);

        let metadata = game.spawn("oTerrainMetadata", Vec2::new(104.0, 168.0));
        let overlay = game.world.entity_mut(metadata).unwrap();
        overlay
            .text
            .insert("terrain_sprite".into(), "sBrick".into());
        overlay.animation = 1.5;
        if duplicate_metadata {
            let duplicate = game.spawn("oTerrainMetadata", Vec2::new(104.0, 168.0));
            let overlay = game.world.entity_mut(duplicate).unwrap();
            overlay.text.insert("terrain_sprite".into(), "sRock".into());
            overlay.animation = 2.5;
        }

        game.world.tiles[10 * game.world.width + 60].engine_active = false;
        assert!(game.world.entity(outside_active).unwrap().engine_active);
        assert!(game.world.entity(inside_active).unwrap().engine_active);
        game
    }

    fn assert_same_activation_state(expected: &Game, actual: &Game) {
        assert_eq!(expected.state_hash(), actual.state_hash());
        assert_eq!(
            expected.dispatcher.active_order,
            actual.dispatcher.active_order
        );
        assert_eq!(
            expected.dispatcher.inactive_order,
            actual.dispatcher.inactive_order
        );
        assert_eq!(
            expected.dispatcher.resource_order,
            actual.dispatcher.resource_order
        );
        assert_eq!(
            expected.dispatcher.registration_order,
            actual.dispatcher.registration_order
        );
        let hidden_entities = |game: &Game| {
            game.dispatcher
                .hidden_entities()
                .iter()
                .map(|e| (e.id, e.object.clone(), e.pos, e.engine_active, e.alive))
                .collect::<Vec<_>>()
        };
        assert_eq!(hidden_entities(expected), hidden_entities(actual));
        let hidden_tiles = |game: &Game| {
            game.dispatcher
                .hidden_tiles()
                .iter()
                .map(|(i, tile)| {
                    (
                        *i,
                        tile.object.clone(),
                        tile.engine_active,
                        tile.creation_serial,
                    )
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(hidden_tiles(expected), hidden_tiles(actual));
    }

    #[test]
    fn level_controller_keeps_rounded_cached_bbox_touching_region_edge_active() {
        let mut game = Game::shell(42, RulesPreset::Classic);
        game.world = World::new(42, 34, Area::Mines, 1);
        game.player.pos = Vec2::new(128.0, 128.0);
        game.camera = Vec2::new(0.0, 17.0);
        let id = game.spawn("oPushBlock", Vec2::new(160.0, 352.55));
        let block = game.world.entity(id).unwrap();
        let region = Rect::new(-96.0, -79.0, 512.0, 432.0);

        assert_eq!(region.y + region.h, 353.0);
        assert_eq!(block.sprite_bounds().y, 352.55);
        assert_eq!(block.collision_mask().point_bounds().y, 353.0);
        assert!(region.intersects(block.sprite_bounds()));
        assert!(!region.intersects(block.collision_mask().point_bounds()));
        assert!(
            crate::activation::Regions::for_game(&game).entity_active(block),
            "GM8's cached bbox touches the inclusive region edge"
        );

        level_step(&mut game);

        assert!(game.world.entity(id).is_some());
        assert!(
            !game
                .dispatcher
                .hidden_entities()
                .iter()
                .any(|entity| { entity.id == id && !entity.engine_active })
        );
    }

    #[test]
    fn paired_level_region_cache_matches_two_source_calls_and_keeps_membership_order() {
        for duplicate_metadata in [false, true] {
            let mut expected = activation_fixture(duplicate_metadata);
            let mut actual = expected.clone();
            let region = Rect::new(0.0, 0.0, 320.0, 240.0);

            region_change(&mut expected, region, false, true);
            region_change(&mut expected, region, true, false);
            level_region_change(&mut actual, region);

            assert_same_activation_state(&expected, &actual);
        }
    }
}

#[cfg(test)]
mod pause_resume_activation_tests {
    use super::*;
    use crate::{Area, PlayerState, RulesPreset, World};

    #[test]
    fn resumed_offscreen_spider_runs_its_due_alarm_before_level_reculls_it() {
        let mut game = Game::shell(42, RulesPreset::Classic);
        game.world = World::new(80, 34, Area::Mines, 1);
        game.world.name = "rLevel".into();
        game.player.pos = Vec2::new(200.0, 120.0);
        game.player.state = PlayerState::Standing;
        game.camera = Vec2::default();
        game.set_simulation_view_extent(320.0, 240.0).unwrap();

        let spider = game.spawn("oSpider", Vec2::new(1000.0, 120.0));
        let entity = game.world.entity_mut(spider).unwrap();
        entity.state = 2;
        entity.alarms[0] = 1;

        // The level controller culls the distant actor after its Alarm phase.
        // The due alarm is one frame away, as if the player paused after that
        // tick's Alarm phase and Level controller cull.
        level_step(&mut game);
        let frozen = game
            .dispatcher
            .hidden_entities()
            .iter()
            .find(|entity| entity.id == spider)
            .expect("offscreen spider should be stored as inactive");
        assert!(!frozen.engine_active);
        assert_eq!(frozen.alarms[0], 1);

        activate_all(&mut game);
        assert!(is_active(&game, Instance::Entity(spider)));

        game.step(&InputFrame::default());

        // Spider Alarm 0 sets state 1 before its ordinary Step, then the
        // later level resource deactivates it again because it is still far.
        let resumed = game
            .world
            .entity(spider)
            .or_else(|| {
                game.dispatcher
                    .hidden_entities()
                    .iter()
                    .find(|entity| entity.id == spider)
            })
            .expect("spider remains present after the resumed tick");
        assert_eq!(resumed.state, 1);
        assert_eq!(resumed.alarms[0], 0);
        assert!(!resumed.engine_active);
    }
}

#[cfg(test)]
mod destroyed_resource_order_tests {
    use super::*;
    use crate::{Area, RulesPreset, World};

    fn depth_sorted_scarabs() -> (Game, u64, u64, u64, i32) {
        let mut game = Game::shell(31, RulesPreset::Classic);
        game.world = World::new(42, 34, Area::Mines, 1);
        game.player.pos = Vec2::new(320.0, 240.0);
        let first = game.spawn("oScarab", Vec2::new(352.0, 240.0));
        let second = game.spawn("oScarab", Vec2::new(384.0, 240.0));
        let unrelated = game.spawn("oBackdrop", Vec2::new(416.0, 240.0));
        game.world
            .entity_mut(first)
            .unwrap()
            .data
            .insert("depth".into(), 10.0);
        game.world
            .entity_mut(second)
            .unwrap()
            .data
            .insert("depth".into(), 100.0);
        let resource = resource(&game, Instance::Entity(first));

        game.step(&InputFrame::default());
        assert_eq!(
            game.dispatcher.resource_order[&resource],
            vec![Instance::Entity(first), Instance::Entity(second)],
            "depth sorting must not reorder the source oScarab resource list"
        );
        (game, first, second, unrelated, resource)
    }

    fn assert_scarab_order(game: &Game, first: u64, second: u64, resource: i32) {
        assert_eq!(
            game.dispatcher.resource_order[&resource],
            vec![Instance::Entity(first), Instance::Entity(second)]
        );
    }

    #[test]
    fn unrelated_destroy_prunes_only_that_identity_from_resource_events() {
        let (mut game, first, second, unrelated, resource) = depth_sorted_scarabs();
        game.world.entity_mut(unrelated).unwrap().alive = false;

        game.step(&InputFrame::default());

        assert_scarab_order(&game, first, second, resource);
    }

    #[test]
    fn animation_cleanup_prunes_dead_identity_without_rebuilding_resource_order() {
        let (mut game, first, second, unrelated, resource) = depth_sorted_scarabs();
        game.world.entities.retain(|entity| entity.id != unrelated);

        animate_entities(&mut game);

        assert_scarab_order(&game, first, second, resource);
    }
}

#[cfg(test)]
mod animation_end_order_tests {
    use super::*;
    use crate::{Area, RulesPreset, World};

    #[test]
    fn animation_end_callbacks_spawn_in_the_source_depth_sorted_order() {
        let mut game = Game::shell(38, RulesPreset::Classic);
        game.world = World::new(42, 34, Area::Mines, 1);
        game.world.name = "rLevel".into();
        game.state = crate::RunState::Playing;
        game.player.pos = Vec2::new(160.0, 160.0);

        // Register the Web Ball first, but give the Fake Bones the higher
        // depth so Classic's in-place active-list sort visits it first.
        let web_ball = game.spawn("oWebBall", Vec2::new(96.0, 160.0));
        let fake_bones = game.spawn("oFakeBones", Vec2::new(80.0, 160.0));
        for (id, sprite, depth) in [
            (web_ball, "sWebCreate", 0.0),
            (fake_bones, "sSkeletonCreateL", 100.0),
        ] {
            let entity = game.world.entity_mut(id).unwrap();
            entity.set_sprite(sprite);
            entity.animation = crate::catalog::sprite_frame_count(sprite) as f64 - 1.0;
            entity.set_image_speed(1.0);
            entity.data.insert("depth".into(), depth);
        }
        game.world
            .entity_mut(web_ball)
            .unwrap()
            .data
            .insert("life".into(), 100.0);

        game.step(&crate::InputFrame::default());

        let spawned_after_callbacks: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|entity| matches!(entity.object.as_str(), "oSkeleton" | "oWeb"))
            .map(|entity| (entity.creation_serial, entity.object.as_str()))
            .collect();
        assert_eq!(
            spawned_after_callbacks
                .iter()
                .map(|(_, object)| *object)
                .collect::<Vec<_>>(),
            ["oSkeleton", "oWeb"]
        );
        assert!(spawned_after_callbacks[0].0 < spawned_after_callbacks[1].0);
    }
}

#[cfg(test)]
mod source_action_abort_dispatch_tests {
    use super::*;
    use crate::{Area, PlayerState, RulesPreset, World};

    #[test]
    fn temple_idol_missing_door_aborts_the_remaining_player_step_event() {
        let mut game = Game::shell(41, RulesPreset::Classic);
        game.world = World::new(42, 34, Area::Temple, 1);
        game.world.name = "rLevel".into();
        for x in 0..10 {
            game.world.set_tile(x, 5, "oBlock");
        }
        game.player.pos = Vec2::new(80.0, 72.0);
        game.player.previous_pos = game.player.pos;
        game.player.state = PlayerState::Ducking;
        game.player.set_sprite("sDuckLeft");
        game.player.grounded = true;
        game.player.money = 5000;
        let ceiling = game.spawn("oCeilingTrap", Vec2::new(96.0, 64.0));
        let keeper = game.spawn("oShopkeeper", game.player.pos);
        game.world
            .entity_mut(keeper)
            .unwrap()
            .text
            .insert("style".into(), "Craps".into());
        let idol = game.spawn("oGoldIdol", game.player.pos);
        let nugget = game.spawn("oGoldNugget", game.player.pos);

        player_step(
            &mut game,
            &InputFrame {
                down: true,
                attack: true,
                attack_pressed: true,
                pay_pressed: true,
                ..Default::default()
            },
        );

        assert_eq!(game.player.held, Some(idol));
        assert_eq!(game.world.entity(idol).unwrap().value("trigger"), 1.0);
        assert_eq!(game.data["idols_grabbed"], 1.0);
        assert_eq!(game.world.entity(ceiling).unwrap().state, 1);
        assert_eq!(game.world.entity(ceiling).unwrap().vel.y, 0.5);
        assert_eq!(game.data["shake"], 20.0);
        assert_eq!(game.data["source_runtime_errors"], 1.0);
        assert_eq!(
            game.player.money, 5000,
            "the same Action's pay tail is skipped"
        );
        assert_eq!(game.player.data.get("bet").copied().unwrap_or(0.0), 0.0);
        assert!(
            game.world.entity(nugget).is_some(),
            "the later Player Step collectible action is skipped"
        );
        assert_eq!(game.data.get("nuggets").copied().unwrap_or(0.0), 0.0);
    }
}

#[cfg(test)]
mod automatic_collision_index_tests {
    use super::*;
    use crate::{Area, GameEvent, InputFrame, RulesPreset, Vec2, World};

    #[test]
    fn collision_entity_index_repairs_stale_slots_and_preserves_duplicate_fallbacks() {
        let mut game = Game::shell(71, RulesPreset::Classic);
        game.world = World::new(24, 18, Area::Mines, 1);
        let first = game.world.spawn("oCrate", Vec2::new(80.0, 80.0));
        let second = game.world.spawn("oJar", Vec2::new(80.0, 80.0));
        let mut cache = CollisionEntityIndex::new(&game.world);

        assert_eq!(
            cache.locate(&game.world, first),
            CachedEntityIndex::Unique(0)
        );
        assert_eq!(
            cache.locate(&game.world, second),
            CachedEntityIndex::Unique(1)
        );

        // A same-length reorder leaves the cache's old indices stale. The
        // validated lookup must fall back and repair them in source order.
        game.world.entities.swap(0, 1);
        assert_eq!(
            cache.locate(&game.world, first),
            CachedEntityIndex::Unique(1)
        );
        assert_eq!(
            cache.locate(&game.world, second),
            CachedEntityIndex::Unique(0)
        );

        // Appended duplicate IDs are detected before cached fast hits. Keep a
        // dead/inactive first match and a live/active later match to ensure
        // the fallback delegates to the original alive/active semantics.
        let mut later_duplicate = game.world.entities[0].clone();
        later_duplicate.pos = Vec2::new(400.0, 400.0);
        later_duplicate.alive = true;
        later_duplicate.engine_active = true;
        game.world.entities[0].alive = false;
        game.world.entities[0].engine_active = false;
        game.world.entities.push(later_duplicate);

        assert_eq!(
            cache.locate(&game.world, second),
            CachedEntityIndex::Duplicate
        );
        assert_eq!(
            alive_cached(&game, &mut cache, Instance::Entity(second)),
            alive(&game, Instance::Entity(second))
        );
        assert!(alive_cached(&game, &mut cache, Instance::Entity(second)));
        assert_eq!(
            active_cached(&game, &mut cache, Instance::Entity(second)),
            is_active(&game, Instance::Entity(second))
        );
        assert!(active_cached(&game, &mut cache, Instance::Entity(second)));
        assert_eq!(
            intersects_cached(
                &game,
                Instance::Entity(second),
                Instance::Entity(first),
                &mut cache,
            ),
            intersects(&game, Instance::Entity(second), Instance::Entity(first)),
            "duplicate intersects lookup must retain the first matching mask"
        );
        assert!(intersects_cached(
            &game,
            Instance::Entity(second),
            Instance::Entity(first),
            &mut cache,
        ));
    }

    #[test]
    fn alarm_bomb_overload_preserves_every_ordered_item_callback() {
        const BOMBS: usize = 80;
        const CRATES: usize = 24;

        let mut game = Game::shell(17, RulesPreset::Classic);
        game.player.pos = Vec2::new(240.0, 200.0);
        game.flags.insert("low_graphics".into());
        let mut crates = Vec::with_capacity(CRATES);
        for _ in 0..CRATES {
            let id = game.spawn("oCrate", Vec2::new(88.0, 88.0));
            game.world.entity_mut(id).unwrap().vel = Vec2::new(0.0, 0.0);
            crates.push(id);
        }
        let mut bombs = Vec::with_capacity(BOMBS);
        for _ in 0..BOMBS {
            let id = game.spawn("oBomb", Vec2::new(88.0, 88.0));
            let bomb = game.world.entity_mut(id).unwrap();
            bomb.data.insert("armed".into(), 1.0);
            bomb.alarms[0] = -1;
            bomb.alarms[1] = 1;
            bomb.timer = 100;
            bombs.push(id);
        }

        // Explosion class/resource order is outermost; Crate instance order
        // is the inner list. The generic item event draws one rand(4,6) per
        // pair, producing the exact velocity and RNG oracle for 1,920 calls.
        let mut expected_rng = game.rng.clone();
        let mut expected_velocities = vec![0.0; CRATES];
        for _ in 0..BOMBS {
            for velocity in &mut expected_velocities {
                *velocity += 4.0 + f64::from(expected_rng.range(3));
            }
        }

        game.step(&InputFrame::default());
        let final_state_hash = game.state_hash();
        assert_eq!(
            final_state_hash, 0x6386_7aac_3816_a58d,
            "simulation state/effects must match the pre-cache baseline"
        );
        assert_eq!(game.rng.next_u64(), expected_rng.next_u64());
        assert_eq!(
            bombs
                .iter()
                .filter(|&&id| game.world.entity(id).is_some())
                .count(),
            0
        );
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|entity| entity.object == "oExplosion" && entity.alive)
                .count(),
            BOMBS
        );
        assert_eq!(
            crates
                .iter()
                .map(|&id| game.world.entity(id).unwrap().vel.x)
                .collect::<Vec<_>>(),
            expected_velocities
        );
        assert_eq!(
            game.events
                .iter()
                .filter(|event| matches!(event, GameEvent::Explosion { .. }))
                .count(),
            BOMBS
        );
        assert_eq!(
            game.events
                .iter()
                .filter(
                    |event| matches!(event, GameEvent::Sound { name, .. } if name == "sndExplosion")
                )
                .count(),
            BOMBS
        );
    }
}

#[cfg(test)]
mod source_price_marker_draw_tests {
    use super::*;
    use crate::{Area, Game, RulesPreset, Vec2, World};

    fn priced_game() -> (Game, u64) {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.world = World::new(40, 20, Area::Mines, 1);
        game.world.name = "rLevel".into();
        for x in 0..game.world.width {
            game.world.set_tile(x, 10, "oBrick");
        }
        game.player.pos = Vec2::new(32.0, 152.0);
        game.player.previous_pos = game.player.pos;
        game.spawn("oShopkeeper", Vec2::new(280.0, 144.0));
        let id = game.spawn("oBombBag", Vec2::new(120.0, 144.0));
        game.world
            .entity_mut(id)
            .unwrap()
            .data
            .insert("cost".into(), 5000.0);
        (game, id)
    }

    #[test]
    fn paid_item_source_draw_samples_before_advancing_once_per_fixed_tick() {
        let (mut game, id) = priced_game();
        let key = {
            let item = game.world.entity(id).unwrap();
            (item.id, item.creation_serial)
        };

        for tick in 1..=30 {
            game.step(&InputFrame::default());
            let item = game
                .world
                .entity(id)
                .expect("shop stock remains in the room");
            assert_eq!(game.tick, tick);
            assert_eq!(item.value("cost"), 5000.0);
            assert_eq!(item.value("cimg"), (tick % 10) as f64);
            assert_eq!(
                game.presentation().render_history.price_marker_frames[&key],
                ((tick - 1) % 10) as f64,
                "Draw displays cimg before incrementing it"
            );
            assert_eq!(
                game.presentation().render_history.price_marker_tick,
                Some(tick)
            );
        }
    }

    #[test]
    fn newly_created_paid_item_draws_frame_zero_in_its_first_source_draw() {
        let (mut game, _) = priced_game();
        game.step(&InputFrame::default());
        let id = game.spawn("oBombBag", Vec2::new(152.0, 144.0));
        game.world
            .entity_mut(id)
            .unwrap()
            .data
            .insert("cost".into(), 5000.0);
        let key = {
            let item = game.world.entity(id).unwrap();
            (item.id, item.creation_serial)
        };
        assert_eq!(game.world.entity(id).unwrap().value("cimg"), 0.0);

        game.step(&InputFrame::default());

        assert_eq!(game.world.entity(id).unwrap().value("cimg"), 1.0);
        assert_eq!(
            game.presentation().render_history.price_marker_frames[&key],
            0.0,
            "a newborn paid item starts at Create's cimg=0"
        );
        assert_eq!(
            game.presentation().render_history.price_marker_tick,
            Some(2)
        );
    }
}
