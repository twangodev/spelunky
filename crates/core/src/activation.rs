//! Room-controller activation from Classic's `oLevel` Step event.
//!
//! Deactivated instances do not participate in alarms, steps, or collision
//! queries. Temporarily removing them enforces that rule across every system.
use crate::{Area, Entity, Game, Rect, Tile, VIEW_HEIGHT, VIEW_WIDTH, Vec2, catalog};
use std::collections::BTreeMap;

const ALWAYS_ACTIVE: &[&str] = &[
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
    "oBoulder",
    "oOlmec",
];

/// Activation regions for the current source room controller. Authored hub
/// rooms have no oLevel, so their instances remain active.
#[derive(Debug, Clone)]
pub struct Regions {
    enabled: bool,
    rectangles: Vec<Rect>,
    stash_id: Option<u64>,
}

impl Regions {
    pub fn for_game(game: &Game) -> Self {
        let mut rectangles = vec![Rect::new(
            game.camera.x - 96.0,
            game.camera.y - 96.0,
            VIEW_WIDTH + 192.0,
            VIEW_HEIGHT + 192.0,
        )];
        // These last two arguments really are x+16 and y+16 in the source,
        // rather than 32 and 32. Preserve the original behavior.
        rectangles.push(Rect::new(
            game.player.pos.x - 16.0,
            game.player.pos.y - 16.0,
            game.player.pos.x + 16.0,
            game.player.pos.y + 16.0,
        ));
        // Object-qualified variables in the source select the first instance.
        for (object, offset, size) in [("oBoulder", 32.0, 64.0), ("oOlmec", 16.0, 96.0)] {
            if let Some(entity) = game
                .world
                .entities
                .iter()
                .find(|e| e.alive && catalog::is_a(&e.object, object))
            {
                rectangles.push(Rect::new(
                    entity.pos.x - offset,
                    entity.pos.y - offset,
                    size,
                    size,
                ));
            }
        }
        Self {
            enabled: !game.flags.contains("hub"),
            rectangles,
            stash_id: game
                .player
                .data
                .get("stash_id")
                .copied()
                .filter(|id| id.is_finite() && *id > 0.0)
                .map(|id| id as u64),
        }
    }

    pub fn bounds_active(&self, bounds: Rect) -> bool {
        !self.enabled
            || self
                .rectangles
                .iter()
                .any(|region| region.intersects(bounds))
    }

    /// Sprite-less bookkeeping objects use their origin as an activation point.
    /// Exact GM8 behavior at empty masks and rectangle edges remains unverified.
    pub fn point_active(&self, point: Vec2) -> bool {
        !self.enabled || self.rectangles.iter().any(|region| region.contains(point))
    }

    fn instance_active(&self, bounds: Rect, origin: Vec2) -> bool {
        if bounds.is_valid() {
            self.bounds_active(bounds)
        } else {
            self.point_active(origin)
        }
    }

    pub fn entity_active(&self, entity: &Entity) -> bool {
        // Inventory cycling represents Classic's pickupItemType value with a
        // hidden, held entity. Keep only the currently referenced bookkeeping
        // instance available to restore/carry logic; its held state and hidden
        // position already exclude it from physics, rendering and interactions.
        (self.stash_id == Some(entity.id) && entity.held && entity.value("stashed") > 0.0)
            || ALWAYS_ACTIVE
                .iter()
                .any(|parent| catalog::is_a(&entity.object, parent))
            || self.instance_active(entity.sprite_bounds(), entity.pos)
    }
}

/// Instances withheld during one authoritative step. Pass this token to
/// [`finish`] before rendering or serializing the world.
#[must_use = "inactive instances must be restored with activation::finish"]
pub struct FrozenEntities {
    entities: Vec<Entity>,
    tiles: Vec<(usize, Tile)>,
    original_order: BTreeMap<u64, usize>,
    level_tick: u64,
    room: (String, usize, usize, Area, u32),
}

fn metadata_tile(entity: &Entity, width: usize, height: usize) -> Option<usize> {
    if entity.object != "oTerrainMetadata" || !entity.pos.x.is_finite() || !entity.pos.y.is_finite()
    {
        return None;
    }
    let x = (entity.pos.x / 16.0).floor();
    let y = (entity.pos.y / 16.0).floor();
    (x >= 0.0 && y >= 0.0 && x < width as f64 && y < height as f64)
        .then(|| y as usize * width + x as usize)
}

/// Called before instance initialization, age, alarms and all simulation phases.
pub fn begin(game: &mut Game) -> FrozenEntities {
    let regions = Regions::for_game(game);
    let original_order = game
        .world
        .entities
        .iter()
        .enumerate()
        .map(|(index, e)| (e.id, index))
        .collect();
    let mut frozen = FrozenEntities {
        entities: Vec::new(),
        tiles: Vec::new(),
        original_order,
        level_tick: game.level_tick,
        room: (
            game.world.name.clone(),
            game.world.width,
            game.world.height,
            game.world.area,
            game.world.level,
        ),
    };
    if !regions.enabled {
        return frozen;
    }

    // Resolve all sprite overrides in one pass before partitioning. The first
    // live override wins, matching World::tile_bounds without scanning the
    // complete entity vector for every occupied terrain cell.
    let mut overrides = vec![None; game.world.tiles.len()];
    for entity in &game.world.entities {
        if entity.alive
            && let Some(index) = metadata_tile(entity, game.world.width, game.world.height)
            && let Some(sprite) = entity.text.get("terrain_sprite")
            && overrides[index].is_none()
        {
            overrides[index] = Some((sprite.as_str(), entity.animation));
        }
    }
    let mut terrain_active = vec![None; game.world.tiles.len()];
    for index in 0..game.world.tiles.len() {
        let tile = &game.world.tiles[index];
        if tile.object.is_empty() {
            continue;
        }
        let x = (index % game.world.width) as i32;
        let y = (index / game.world.width) as i32;
        let origin = Vec2::new(f64::from(x) * 16.0, f64::from(y) * 16.0);
        let (sprite, animation) =
            overrides[index].unwrap_or_else(|| (catalog::sprite(&tile.object).unwrap_or(""), 0.0));
        let bounds = catalog::instance_sprite_bounds(
            &tile.object,
            sprite,
            origin,
            animation,
            Vec2::new(1.0, 1.0),
            0.0,
        );
        let active = regions.instance_active(bounds, origin);
        terrain_active[index] = Some(active);
        if !active {
            frozen
                .tiles
                .push((index, std::mem::take(&mut game.world.tiles[index])));
        }
    }
    let entities = std::mem::take(&mut game.world.entities);
    for entity in entities {
        // The tile and its metadata are one original instance. Its visual and
        // collision override must follow the tile mask, not the helper's
        // centered origin (which may lie just outside an activation edge).
        let active = metadata_tile(&entity, game.world.width, game.world.height)
            .and_then(|index| terrain_active[index])
            .unwrap_or_else(|| regions.entity_active(&entity));
        if active {
            game.world.entities.push(entity);
        } else {
            frozen.entities.push(entity);
        }
    }
    frozen
}

/// Restore inactive instances without advancing any of their state. Surviving
/// original instances retain creation order, followed by this frame's spawns.
/// A room transition resets level_tick and discards the old room's token.
pub fn finish(game: &mut Game, frozen: FrozenEntities) {
    let room = (
        &game.world.name,
        game.world.width,
        game.world.height,
        game.world.area,
        game.world.level,
    );
    if game.level_tick != frozen.level_tick.wrapping_add(1)
        || room
            != (
                &frozen.room.0,
                frozen.room.1,
                frozen.room.2,
                frozen.room.3,
                frozen.room.4,
            )
    {
        return;
    }
    for (index, tile) in frozen.tiles {
        game.world.tiles[index] = tile;
    }
    game.world.entities.extend(frozen.entities);
    game.world.entities.sort_by_key(|entity| {
        frozen
            .original_order
            .get(&entity.id)
            .copied()
            .unwrap_or(usize::MAX)
    });
}
