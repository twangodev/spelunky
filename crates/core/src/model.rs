use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const TICK_RATE: u32 = 30;
/// Versioned replay/record identity for this simulation contract.
pub const SIMULATION_VERSION: u32 = 2;
pub const HISTORICAL_SIMULATION_VERSION: u32 = 1;
pub const TILE_SIZE: f64 = 16.0;
pub const VIEW_WIDTH: f64 = 320.0;
pub const VIEW_HEIGHT: f64 = 240.0;

/// The source collision functions explicitly select either bounding boxes or
/// the target sprite's precise mask. Rectangle sprites use their box in both.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum CollisionPrecision {
    #[default]
    BoundingBox,
    Precise,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}
impl Vec2 {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
    pub fn distance(self, other: Self) -> f64 {
        (self.x - other.x).hypot(self.y - other.y)
    }
    pub fn lerp(self, other: Self, alpha: f64) -> Self {
        Self::new(
            self.x + (other.x - self.x) * alpha,
            self.y + (other.y - self.y) * alpha,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}
impl Rect {
    pub const fn new(x: f64, y: f64, w: f64, h: f64) -> Self {
        Self { x, y, w, h }
    }
    pub fn intersects(self, other: Self) -> bool {
        self.is_valid()
            && other.is_valid()
            && self.x < other.x + other.w
            && self.x + self.w > other.x
            && self.y < other.y + other.h
            && self.y + self.h > other.y
    }
    pub fn contains(self, point: Vec2) -> bool {
        self.is_valid()
            && point.x >= self.x
            && point.x < self.x + self.w
            && point.y >= self.y
            && point.y < self.y + self.h
    }
    /// Empty, inverted and nonfinite geometry must never collide.
    pub fn is_valid(self) -> bool {
        self.x.is_finite()
            && self.y.is_finite()
            && self.w.is_finite()
            && self.h.is_finite()
            && self.w > 0.0
            && self.h > 0.0
            && (self.x + self.w).is_finite()
            && (self.y + self.h).is_finite()
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RulesPreset {
    #[default]
    Classic,
    Assisted,
}

/// Edges are retained by the host until one authoritative tick consumes them.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputFrame {
    pub left: bool,
    pub right: bool,
    /// Independent device edges, ORed by the host like Classic's control scripts.
    #[serde(default)]
    pub left_released: bool,
    #[serde(default)]
    pub right_released: bool,
    pub up: bool,
    #[serde(default)]
    pub up_pressed: bool,
    #[serde(default)]
    pub down_pressed: bool,
    pub down: bool,
    pub run: bool,
    pub jump: bool,
    pub jump_pressed: bool,
    pub jump_released: bool,
    pub attack: bool,
    pub attack_pressed: bool,
    #[serde(default)]
    pub attack_released: bool,
    #[serde(default)]
    pub start_pressed: bool,
    /// Fresh physical M, independent from remappable player actions.
    #[serde(default)]
    pub music_pressed: bool,
    pub bomb_pressed: bool,
    pub rope_pressed: bool,
    pub flare_pressed: bool,
    pub cycle_pressed: bool,
    pub pay_pressed: bool,
    pub exit_pressed: bool,
}
impl InputFrame {
    pub fn clear_edges(&mut self) {
        self.left_released = false;
        self.right_released = false;
        self.up_pressed = false;
        self.down_pressed = false;
        self.attack_released = false;
        self.jump_pressed = false;
        self.jump_released = false;
        self.attack_pressed = false;
        self.start_pressed = false;
        self.music_pressed = false;
        self.bomb_pressed = false;
        self.rope_pressed = false;
        self.flare_pressed = false;
        self.cycle_pressed = false;
        self.pay_pressed = false;
        self.exit_pressed = false;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlayerState {
    Standing,
    Running,
    Ducking,
    LookingUp,
    Climbing,
    Jumping,
    Falling,
    Hanging,
    Stunned,
    Dead,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    #[serde(default = "crate::dispatcher::initially_active")]
    pub engine_active: bool,
    pub pos: Vec2,
    pub previous_pos: Vec2,
    pub vel: Vec2,
    pub facing: i8,
    pub state: PlayerState,
    pub grounded: bool,
    pub health: i32,
    /// Session-only opt-in cheat. Imported games always start without it.
    #[serde(skip)]
    pub cheat_invulnerable: bool,
    pub bombs: i32,
    pub ropes: i32,
    pub money: u32,
    pub equipment: BTreeSet<String>,
    pub held: Option<u64>,
    /// Source pickupItemType: inventory cycling stores a type, never an actor.
    #[serde(default)]
    pub pickup_item_type: String,
    pub invincible: u32,
    pub stun: u32,
    pub animation: f64,
    pub sprite: String,
    pub attack_timer: u32,
    pub jump_time: u32,
    pub coyote_ticks: u32,
    pub buffered_jump_ticks: u32,
    pub selected: u8,
    pub data: BTreeMap<String, f64>,
    #[serde(default = "crate::dispatcher::unset_alarms")]
    pub alarms: [i32; 12],
}
impl Default for Player {
    fn default() -> Self {
        Self {
            engine_active: true,
            pos: Vec2::new(40.0, 40.0),
            previous_pos: Vec2::new(40.0, 40.0),
            vel: Vec2::default(),
            facing: 1,
            state: PlayerState::Falling,
            grounded: false,
            health: 4,
            cheat_invulnerable: false,
            bombs: 4,
            ropes: 4,
            money: 0,
            equipment: BTreeSet::new(),
            held: None,
            pickup_item_type: String::new(),
            invincible: 0,
            stun: 0,
            animation: 0.0,
            alarms: crate::dispatcher::unset_alarms(),
            sprite: "sStandLeft".into(),
            attack_timer: 0,
            jump_time: 0,
            coyote_ticks: 0,
            buffered_jump_ticks: 0,
            selected: 0,
            data: BTreeMap::from([("whoaTimer".into(), 0.0), ("whoaTimerMax".into(), 30.0)]),
        }
    }
}
impl Player {
    pub(crate) fn lose_health(&mut self, amount: i32) {
        if !self.cheat_invulnerable {
            self.health -= amount;
        }
    }

    pub fn set_sprite(&mut self, next: &str) {
        crate::animation::assign(&mut self.sprite, &mut self.animation, next);
    }
    pub fn image_speed(&self) -> f64 {
        self.data.get("animation_speed").copied().unwrap_or(1.0)
    }
    pub fn set_image_speed(&mut self, speed: f64) {
        self.data.insert("animation_speed".into(), speed);
    }
    /// oPlayer1's source hurt block excludes every character's exit sprite,
    /// independently of the invincibility countdown used by contact damage.
    pub fn is_exiting(&self) -> bool {
        matches!(
            self.sprite.as_str(),
            "sPExit" | "sDamselExit" | "sTunnelExit"
        )
    }

    /// Classic stores the stunned flag independently from its countdown.
    /// Throws can remain stunned in flight after the countdown reaches zero.
    pub fn is_stunned(&self) -> bool {
        self.stun > 0
            || self.state == PlayerState::Stunned
            || self.data.get("source_stunned").copied().unwrap_or(0.0) != 0.0
    }
    /// Current source sprite mask for collision events, separate from the
    /// platform engine's custom rectangle returned by `bounds()`.
    pub fn sprite_bounds(&self) -> Rect {
        let (scale, angle) = self.collision_transform();
        crate::catalog::instance_sprite_bounds(
            "oPlayer1",
            &self.sprite,
            self.pos,
            self.animation,
            scale,
            angle,
        )
    }
    pub fn mask_contains(&self, point: Vec2, precision: CollisionPrecision) -> bool {
        self.collision_mask().contains(point, precision)
    }
    pub fn mask_intersects(&self, rect: Rect, precision: CollisionPrecision) -> bool {
        self.collision_mask().intersects(rect, precision)
    }
    pub(crate) fn collision_mask(&self) -> crate::catalog::CollisionMask {
        self.collision_mask_at(self.pos)
    }
    /// instance_place changes only the queried origin, retaining the current
    /// loaded sprite mask, animation frame, and source Draw transform.
    pub(crate) fn collision_mask_at(&self, pos: Vec2) -> crate::catalog::CollisionMask {
        let (scale, angle) = self.collision_transform();
        crate::catalog::instance_mask("oPlayer1", &self.sprite, pos, self.animation, scale, angle)
    }
    fn collision_transform(&self) -> (Vec2, f64) {
        let xscale = self.data.get("image_xscale").copied().unwrap_or(1.0);
        let yscale = self.data.get("image_yscale").copied().unwrap_or(1.0);
        let angle = -self
            .data
            .get("image_angle")
            .copied()
            .unwrap_or(0.0)
            .to_radians();
        (Vec2::new(xscale, yscale), angle)
    }
    pub fn bounds(&self) -> Rect {
        let half = self
            .data
            .get("collision_half_width")
            .copied()
            .unwrap_or(5.0);
        let top = self.data.get("collision_top").copied().unwrap_or(-8.0);
        Rect::new(self.pos.x - half, self.pos.y + top, half * 2.0, 8.0 - top)
    }
    pub fn has(&self, name: &str) -> bool {
        self.equipment.contains(name)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub id: u64,
    #[serde(default)]
    pub creation_serial: u64,
    pub object: String,
    pub sprite: String,
    pub pos: Vec2,
    pub previous_pos: Vec2,
    pub vel: Vec2,
    pub facing: i8,
    pub health: i32,
    pub state: i32,
    pub timer: u32,
    pub age: u32,
    pub animation: f64,
    pub alive: bool,
    pub solid: bool,
    pub held: bool,
    pub data: BTreeMap<String, f64>,
    pub text: BTreeMap<String, String>,
    #[serde(default = "crate::dispatcher::unset_alarms")]
    pub alarms: [i32; 12],
    #[serde(default = "crate::dispatcher::initially_active")]
    pub engine_active: bool,
}
impl Entity {
    pub fn set_sprite(&mut self, next: &str) {
        crate::animation::assign(&mut self.sprite, &mut self.animation, next);
    }
    pub fn image_speed(&self) -> f64 {
        self.data.get("animation_speed").copied().unwrap_or(1.0)
    }
    pub fn set_image_speed(&mut self, speed: f64) {
        self.data.insert("animation_speed".into(), speed);
    }
    /// Collision-event mask from the current sprite/object mask. Movement
    /// overrides in `data["collision_*"]` deliberately do not affect this.
    pub fn sprite_bounds(&self) -> Rect {
        let (scale, angle) = self.collision_transform();
        crate::catalog::instance_sprite_bounds(
            &self.object,
            &self.sprite,
            self.pos,
            self.animation,
            scale,
            angle,
        )
    }
    pub fn mask_contains(&self, point: Vec2, precision: CollisionPrecision) -> bool {
        self.collision_mask().contains(point, precision)
    }
    pub fn mask_intersects(&self, rect: Rect, precision: CollisionPrecision) -> bool {
        self.collision_mask().intersects(rect, precision)
    }
    /// Source Collision event overlap, distinct from explicit rectangle/point
    /// query precision and from the platform engine's movement dimensions.
    pub fn event_intersects(&self, other: &Entity) -> bool {
        self.collision_mask()
            .intersects_mask(&other.collision_mask())
    }
    pub(crate) fn collision_mask(&self) -> crate::catalog::CollisionMask {
        self.collision_mask_with_geometry(&self.object, &self.sprite, self.pos)
    }
    /// Same authoritative mask construction with a borrowed canonical parent
    /// projection. Data/frame/transform remain this actual allocation's fields.
    pub(crate) fn collision_mask_with_geometry(
        &self,
        object: &str,
        sprite: &str,
        pos: Vec2,
    ) -> crate::catalog::CollisionMask {
        let (scale, angle) = self.collision_transform_for_object(object);
        crate::catalog::instance_mask(object, sprite, pos, self.animation, scale, angle)
    }
    fn collision_transform(&self) -> (Vec2, f64) {
        self.collision_transform_for_object(&self.object)
    }
    fn collision_transform_for_object(&self, object: &str) -> (Vec2, f64) {
        // Source image_xscale is an instance property. Damsel/dummy Draw
        // writes it after Collision; a Step-facing change cannot alter it yet.
        let default_xscale = 1.0;
        let xscale = self
            .data
            .get("image_xscale")
            .copied()
            .unwrap_or(default_xscale);
        let yscale = self.data.get("image_yscale").copied().unwrap_or(1.0);
        let angle = self
            .data
            .get("image_angle")
            .map(|angle| -angle.to_radians())
            .unwrap_or_else(|| {
                if matches!(object, "oArrow" | "oFishBone") {
                    self.value("rotation")
                } else {
                    0.0
                }
            });
        (Vec2::new(xscale, yscale), angle)
    }
    pub fn new(id: u64, object: impl Into<String>, pos: Vec2) -> Self {
        let object = object.into();
        let sprite = crate::catalog::sprite(&object)
            .map(str::to_owned)
            .unwrap_or_else(|| format!("s{}", object.strip_prefix('o').unwrap_or(&object)));
        let solid = crate::catalog::is_solid(&object);
        // oHawkmanWorship is parentless and has no Draw override. Every birth
        // keeps the engine's default positive sprite scale; port-facing is not
        // a source Draw property. Explicit later scale overrides remain valid.
        let data = if object == "oHawkmanWorship" {
            BTreeMap::from([("image_xscale".into(), 1.0)])
        } else {
            BTreeMap::new()
        };
        Self {
            id,
            creation_serial: 0,
            object,
            sprite,
            pos,
            previous_pos: pos,
            vel: Vec2::default(),
            facing: 1,
            health: 1,
            state: 0,
            timer: 0,
            age: 0,
            animation: 0.0,
            alive: true,
            solid,
            held: false,
            data,
            text: BTreeMap::new(),
            alarms: crate::dispatcher::unset_alarms(),
            engine_active: true,
        }
    }
    pub fn bounds(&self) -> Rect {
        if let (Some(l), Some(t), Some(r), Some(b)) = (
            self.data.get("collision_left"),
            self.data.get("collision_top"),
            self.data.get("collision_right"),
            self.data.get("collision_bottom"),
        ) {
            Rect::new(self.pos.x + l, self.pos.y + t, r - l, b - t)
        } else {
            crate::catalog::bounds(&self.object, self.pos)
        }
    }
    pub fn value(&self, key: &str) -> f64 {
        self.data.get(key).copied().unwrap_or(0.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tile {
    pub object: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_metadata_id: Option<u64>,
    #[serde(default)]
    pub creation_serial: u64,
    #[serde(default = "crate::dispatcher::unset_alarms")]
    pub alarms: [i32; 12],
    #[serde(default = "crate::dispatcher::initially_active")]
    pub engine_active: bool,
}
impl Default for Tile {
    fn default() -> Self {
        Self {
            object: String::new(),
            source_metadata_id: None,
            creation_serial: 0,
            alarms: crate::dispatcher::unset_alarms(),
            engine_active: true,
        }
    }
}
impl Tile {
    pub fn new(object: impl Into<String>) -> Self {
        Self {
            object: object.into(),
            ..Self::default()
        }
    }
    pub fn is_solid(&self) -> bool {
        crate::catalog::is_solid(&self.object)
    }
    pub fn is_ladder(&self) -> bool {
        crate::catalog::is_a(&self.object, "oLadder")
            || crate::catalog::is_a(&self.object, "oLadderTop")
    }
    pub fn is_destructible(&self) -> bool {
        self.is_solid() && !matches!(self.object.as_str(), "oSolid" | "oHardBlock")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Area {
    Mines,
    Jungle,
    Ice,
    Temple,
    Boss,
}
impl Area {
    pub fn for_level(level: u32) -> Self {
        match level {
            0..=4 => Self::Mines,
            5..=8 => Self::Jungle,
            9..=12 => Self::Ice,
            13..=15 => Self::Temple,
            _ => Self::Boss,
        }
    }
}

/// An adapter-local graphic handle. This is neither an actor ID nor the
/// original engine's numeric tile_add result/counter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GeneratedGraphicTileHandle(u64);

/// Fully supplied graphics state, independent of solid grid/actor storage.
/// The source setup sink supplies converted integral crop coordinates; retaining
/// f64 here does not implement the native conversion/Variant protocol.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneratedGraphicTileSpec {
    pub background: String,
    pub source: [f64; 4],
    pub position: Vec2,
    pub depth: f64,
    pub scale: Vec2,
    pub color: u32,
    pub alpha: f64,
    pub visible: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GeneratedGraphicTile {
    pub handle: GeneratedGraphicTileHandle,
    pub graphic: GeneratedGraphicTileSpec,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphicTileAllocationError {
    HandleExhausted,
}
impl std::fmt::Display for GraphicTileAllocationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("generated graphic adapter handle space exhausted")
    }
}
impl std::error::Error for GraphicTileAllocationError {}

fn generated_graphic_counter_is_zero(value: &u64) -> bool {
    *value == 0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct World {
    pub width: usize,
    pub height: usize,
    pub tiles: Vec<Tile>,
    pub entities: Vec<Entity>,
    /// Source-generated tile graphics, never registered actors or collision cells.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub generated_graphic_tiles: Vec<GeneratedGraphicTile>,
    /// World-scoped adapter sequence; unrelated to any native per-room counter.
    #[serde(default, skip_serializing_if = "generated_graphic_counter_is_zero")]
    generated_graphic_serial: u64,
    pub entrance: Vec2,
    pub exit: Vec2,
    pub area: Area,
    pub level: u32,
    pub dark: bool,
    pub name: String,
    pub next_id: u64,
    #[serde(default)]
    pub creation_serial: u64,
    #[serde(default)]
    pub tile_revision: u64,
    /// A destroyed solid requests the source global water/lava drainage pass.
    #[serde(default)]
    pub fluid_dirty: bool,
    /// Campaign room-path markers captured by SourceClassic generation.
    /// Missing historical/custom context retains its prior shop adapter; no
    /// keeper-derived grid is reconstructed or serialized on those worlds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_room_path: Option<[[u8; 4]; 5]>,
}
impl World {
    pub fn new(width: usize, height: usize, area: Area, level: u32) -> Self {
        Self {
            width,
            height,
            tiles: vec![Tile::default(); width * height],
            entities: Vec::new(),
            generated_graphic_tiles: Vec::new(),
            generated_graphic_serial: 0,
            entrance: Vec2::new(40.0, 40.0),
            exit: Vec2::new(616.0, 504.0),
            area,
            level,
            dark: false,
            name: String::new(),
            next_id: 1,
            creation_serial: 0,
            tile_revision: 0,
            fluid_dirty: false,
            source_room_path: None,
        }
    }
    /// Append in source emission order. Only the independent graphics counter
    /// advances: no actor reservation/Create, terrain replacement, RNG or event.
    pub fn add_generated_graphic_tile(
        &mut self,
        graphic: GeneratedGraphicTileSpec,
    ) -> Result<GeneratedGraphicTileHandle, GraphicTileAllocationError> {
        let next = self
            .generated_graphic_serial
            .checked_add(1)
            .ok_or(GraphicTileAllocationError::HandleExhausted)?;
        let handle = GeneratedGraphicTileHandle(next);
        self.generated_graphic_tiles
            .push(GeneratedGraphicTile { handle, graphic });
        self.generated_graphic_serial = next;
        Ok(handle)
    }

    pub fn generated_graphic_tile(
        &self,
        handle: GeneratedGraphicTileHandle,
    ) -> Option<&GeneratedGraphicTile> {
        self.generated_graphic_tiles
            .iter()
            .find(|tile| tile.handle == handle)
    }

    /// Adapter issuer-range validation, also after deletion. This does not
    /// expose or reconstruct an Original numeric tile ID/counter.
    pub(crate) fn generated_graphic_handle_was_issued(
        &self,
        handle: GeneratedGraphicTileHandle,
    ) -> bool {
        handle.0 > 0 && handle.0 <= self.generated_graphic_serial
    }

    /// Physical graphics removal only; it never dispatches actor Destroy.
    pub fn remove_generated_graphic_tile(&mut self, handle: GeneratedGraphicTileHandle) -> bool {
        let before = self.generated_graphic_tiles.len();
        self.generated_graphic_tiles
            .retain(|tile| tile.handle != handle);
        before != self.generated_graphic_tiles.len()
    }

    /// Exact source depth equality, including equal signed zero. Remaining
    /// records retain their current order; the host graphics counter persists.
    pub fn remove_generated_graphics_at_depth(&mut self, depth: f64) -> usize {
        let before = self.generated_graphic_tiles.len();
        self.generated_graphic_tiles
            .retain(|tile| tile.graphic.depth != depth);
        before - self.generated_graphic_tiles.len()
    }

    pub fn tile(&self, x: i32, y: i32) -> Option<&Tile> {
        if x < 0 || y < 0 || x as usize >= self.width || y as usize >= self.height {
            None
        } else {
            self.tiles.get(y as usize * self.width + x as usize)
        }
    }
    pub fn set_tile(&mut self, x: usize, y: usize, object: impl Into<String>) {
        if x < self.width && y < self.height {
            self.creation_serial = self.creation_serial.saturating_add(1);
            self.tile_revision = self.tile_revision.wrapping_add(1);
            let mut replacement = Tile::new(object);
            replacement.creation_serial = self.creation_serial;
            let previous = &self.tiles[y * self.width + x];
            if previous.object != replacement.object
                && crate::catalog::is_a(&previous.object, "oSolid")
            {
                self.fluid_dirty = true;
            }
            self.tiles[y * self.width + x] = replacement;
        }
    }
    /// Collision mask of the instance stored at this tile origin. Tiles retain
    /// source origins and sprite footprints; a storage cell is not a collider.
    /// An explicit terrain sprite corresponds to source `sprite_index` writes
    /// (notably the otherwise sprite-less oSolid shop ice blocks).
    pub fn tile_bounds(&self, x: i32, y: i32) -> Rect {
        self.tile_collision_mask(x, y).bounds()
    }

    /// Exact metadata allocation supplying a tile's explicit sprite/frame.
    /// A preferred Source binding excludes coordinate aliases. With no binding,
    /// the first live coordinate provider with terrain_sprite retains authority.
    pub(crate) fn tile_sprite_metadata(&self, x: i32, y: i32) -> Option<&Entity> {
        let tile = self.tile(x, y)?;
        self.entities.iter().find(|metadata| {
            metadata.alive
                && metadata.object == "oTerrainMetadata"
                && tile.source_metadata_id.is_none_or(|id| metadata.id == id)
                && (metadata.pos.x / TILE_SIZE).floor() as i32 == x
                && (metadata.pos.y / TILE_SIZE).floor() as i32 == y
                && metadata.text.contains_key("terrain_sprite")
        })
    }

    pub fn tile_collision_mask(&self, x: i32, y: i32) -> crate::catalog::CollisionMask {
        let origin = Vec2::new(f64::from(x) * TILE_SIZE, f64::from(y) * TILE_SIZE);
        let Some(tile) = self.tile(x, y) else {
            return crate::catalog::instance_mask("", "", origin, 0.0, Vec2::new(1.0, 1.0), 0.0);
        };
        let sprite = self.tile_sprite_metadata(x, y).and_then(|metadata| {
            metadata
                .text
                .get("terrain_sprite")
                .map(|sprite| (sprite.as_str(), metadata.animation))
        });
        crate::catalog::instance_mask(
            &tile.object,
            sprite
                .map(|(sprite, _)| sprite)
                .or_else(|| crate::catalog::sprite(&tile.object))
                .unwrap_or(""),
            origin,
            sprite.map_or(0.0, |(_, animation)| animation),
            Vec2::new(1.0, 1.0),
            0.0,
        )
    }

    /// Source group query against stored tile instances. It inspects complete
    /// masks, including portions extending beyond their storage cells. Dynamic
    /// instances are handled separately by callers that need them.
    pub fn tiles_collide(&self, r: Rect, group: &str) -> bool {
        self.tiles_collide_precision(r, group, CollisionPrecision::BoundingBox)
    }

    pub fn tiles_collide_precision(
        &self,
        r: Rect,
        group: &str,
        precision: CollisionPrecision,
    ) -> bool {
        self.tiles_match(r, group, None, None, precision, false)
    }

    /// Explicit source collision_rectangle over active stored family members.
    /// Corners remain raw inclusive arguments; held targets stay eligible.
    /// The optional exclusion denotes a dynamic caller, never a numeric sentinel.
    /// This is a geometric hit predicate; signed native handle truth is separate.
    pub fn rectangle_family_except(
        &self,
        first: Vec2,
        second: Vec2,
        family: &str,
        excluded: Option<u64>,
        precision: CollisionPrecision,
    ) -> bool {
        self.tiles.iter().enumerate().any(|(index, tile)| {
            tile.engine_active
                && self.width > 0
                && crate::catalog::is_a(&tile.object, family)
                && self
                    .tile_collision_mask((index % self.width) as i32, (index / self.width) as i32)
                    .intersects_rectangle(first, second, precision)
        }) || self.entities.iter().any(|entity| {
            Some(entity.id) != excluded
                && entity.alive
                && entity.engine_active
                && crate::catalog::is_a(&entity.object, family)
                && entity
                    .collision_mask()
                    .intersects_rectangle(first, second, precision)
        })
    }

    /// Explicit source axis-aligned collision_line query over tile instances.
    pub fn tiles_line_precision(
        &self,
        first: Vec2,
        second: Vec2,
        group: &str,
        precision: CollisionPrecision,
    ) -> bool {
        self.tiles_line_precision_with_activity(first, second, group, precision, false)
    }

    fn tiles_line_precision_with_activity(
        &self,
        first: Vec2,
        second: Vec2,
        group: &str,
        precision: CollisionPrecision,
        active_only: bool,
    ) -> bool {
        if !first.x.is_finite()
            || !first.y.is_finite()
            || !second.x.is_finite()
            || !second.y.is_finite()
            || (first.x != second.x && first.y != second.y)
        {
            return false;
        }
        self.tiles_match(
            Rect::new(
                first.x.min(second.x),
                first.y.min(second.y),
                (first.x - second.x).abs() + 1.0,
                (first.y - second.y).abs() + 1.0,
            ),
            group,
            None,
            Some((first, second)),
            precision,
            active_only,
        )
    }

    /// Source collision_line(..., oSolid, true, true) for axis-aligned script
    /// probes. Explicit family queries include held solids and skip inactive
    /// instances; the engine solid flag does not select the queried family.
    /// Callers round each complete source endpoint expression before querying.
    pub fn solid_line_except(&self, first: Vec2, second: Vec2, excluded: u64) -> bool {
        self.tiles_line_precision_with_activity(
            first,
            second,
            "oSolid",
            CollisionPrecision::Precise,
            true,
        ) || self.entities.iter().any(|entity| {
            entity.id != excluded
                && entity.alive
                && entity.engine_active
                && crate::catalog::is_a(&entity.object, "oSolid")
                && entity.collision_mask().intersects_axis_line(
                    first,
                    second,
                    CollisionPrecision::Precise,
                )
        })
    }

    /// Point form of the same tile-instance mask query, with exact half-open
    /// containment rather than a tiny rectangle that can touch a nearby edge.
    pub fn tiles_point(&self, point: Vec2, group: &str) -> bool {
        self.tiles_point_precision(point, group, CollisionPrecision::BoundingBox)
    }

    pub fn tiles_point_precision(
        &self,
        point: Vec2,
        group: &str,
        precision: CollisionPrecision,
    ) -> bool {
        self.tiles_match(
            Rect::new(point.x, point.y, 0.001, 0.001),
            group,
            Some(point),
            None,
            precision,
            false,
        )
    }

    /// Source collision_point(..., family, 0, 0) only sees active instances.
    /// Retain the sparse mask broadphase used by other tile queries.
    pub(crate) fn tiles_point_active(&self, point: Vec2, group: &str) -> bool {
        self.tiles_match(
            Rect::new(point.x, point.y, 0.001, 0.001),
            group,
            Some(point),
            None,
            CollisionPrecision::BoundingBox,
            true,
        )
    }

    fn tiles_match(
        &self,
        r: Rect,
        group: &str,
        point: Option<Vec2>,
        line: Option<(Vec2, Vec2)>,
        precision: CollisionPrecision,
        active_only: bool,
    ) -> bool {
        if !r.is_valid() || self.width == 0 {
            return false;
        }
        let matches = |mask: &crate::catalog::CollisionMask| {
            if let Some((first, second)) = line {
                mask.intersects_axis_line(first, second, precision)
            } else {
                point.map_or_else(
                    || mask.intersects(r, precision),
                    |p| mask.contains(p, precision),
                )
            }
        };
        let envelope = crate::catalog::group_bounds(group).unwrap_or(Rect::new(0.0, 0.0, 0.0, 0.0));
        let x0 = (((r.x - envelope.x - envelope.w) / TILE_SIZE).floor() as i32).max(0);
        let y0 = (((r.y - envelope.y - envelope.h) / TILE_SIZE).floor() as i32).max(0);
        let x1 = (((r.x + r.w - envelope.x) / TILE_SIZE).floor() as i32)
            .min((self.width as i32).saturating_sub(1));
        let y1 = (((r.y + r.h - envelope.y) / TILE_SIZE).floor() as i32)
            .min((self.height as i32).saturating_sub(1));
        // Build the sparse per-instance overrides once, rather than performing
        // a linear metadata lookup for every solid in every collision query.
        let mut overrides = vec![0_u64; self.tiles.len().div_ceil(64)];
        for metadata in &self.entities {
            if !metadata.alive || metadata.object != "oTerrainMetadata" {
                continue;
            }
            let Some(sprite) = metadata.text.get("terrain_sprite") else {
                continue;
            };
            let x = (metadata.pos.x / TILE_SIZE).floor() as i32;
            let y = (metadata.pos.y / TILE_SIZE).floor() as i32;
            let Some(tile) = self.tile(x, y) else {
                continue;
            };
            let index = y as usize * self.width + x as usize;
            let bit = 1_u64 << (index % 64);
            if overrides[index / 64] & bit != 0 {
                continue;
            }
            overrides[index / 64] |= bit;
            // Most point queries target a sparse family such as oWater. Reject
            // unrelated terrain before resolving frame masks or pixel data.
            if (active_only && !tile.engine_active) || !crate::catalog::is_a(&tile.object, group) {
                continue;
            }
            let origin = Vec2::new(f64::from(x) * TILE_SIZE, f64::from(y) * TILE_SIZE);
            // Overrides can extend beyond the object's default envelope. Use
            // this override's aggregate sprite bounds to reject distant tiles
            // before constructing/sampling its precise mask. Separate masks
            // need the all-frame envelope, not their first frame's bounds.
            let sprite_envelope =
                crate::catalog::terrain_mask_envelope(&tile.object, sprite, origin);
            if !sprite_envelope.intersects(r) {
                continue;
            }
            let mask = crate::catalog::instance_mask(
                &tile.object,
                sprite,
                origin,
                metadata.animation,
                Vec2::new(1.0, 1.0),
                0.0,
            );
            if matches(&mask) {
                return true;
            }
        }
        for y in y0..=y1 {
            for x in x0..=x1 {
                let index = y as usize * self.width + x as usize;
                if overrides[index / 64] & (1_u64 << (index % 64)) != 0 {
                    continue;
                }
                if let Some(tile) = self.tile(x, y)
                    && (!active_only || tile.engine_active)
                    && crate::catalog::is_a(&tile.object, group)
                    && matches(&crate::catalog::instance_mask(
                        &tile.object,
                        crate::catalog::sprite(&tile.object).unwrap_or(""),
                        Vec2::new(f64::from(x) * TILE_SIZE, f64::from(y) * TILE_SIZE),
                        0.0,
                        Vec2::new(1.0, 1.0),
                        0.0,
                    ))
                {
                    return true;
                }
            }
        }
        false
    }
    pub fn solid_at(&self, x: f64, y: f64) -> bool {
        self.solid_at_except(x, y, 0)
    }
    pub fn solid_point_except(&self, pos: Vec2, excluded: u64) -> bool {
        self.solid_at_except(pos.x, pos.y, excluded)
    }
    /// Source collision_point(oSolid): authored tiles and alive, nonheld
    /// dynamic solids. Outside-room space is open unless explicitly authored.
    pub fn solid_at_except(&self, x: f64, y: f64, excluded: u64) -> bool {
        if !x.is_finite() || !y.is_finite() {
            return false;
        }
        self.tiles_point(Vec2::new(x, y), "oSolid")
            || self.entities.iter().any(|e| {
                e.id != excluded
                    && e.alive
                    && e.solid
                    && !e.held
                    && e.mask_contains(Vec2::new(x, y), CollisionPrecision::BoundingBox)
            })
    }
    /// Tile-only rectangle query. Use collides_except for dynamic solids too.
    pub fn collides(&self, r: Rect) -> bool {
        self.tiles_collide(r, "oSolid")
    }
    pub fn collides_except(&self, r: Rect, excluded: u64) -> bool {
        self.collides(r)
            || self.entities.iter().any(|e| {
                e.id != excluded && e.alive && e.solid && !e.held && e.sprite_bounds().intersects(r)
            })
    }
    pub fn spawn(&mut self, object: impl Into<String>, pos: Vec2) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        let mut e = Entity::new(id, object, pos);
        self.creation_serial = self.creation_serial.saturating_add(1);
        e.creation_serial = self.creation_serial;
        crate::enemies::init_entity(&mut e);
        crate::items::init_entity(&mut e);
        crate::world_events::init_entity(&mut e);
        crate::traps::init_entity(&mut e);
        self.entities.push(e);
        crate::enemies::on_spawn(self, id);
        id
    }
    pub fn entity(&self, id: u64) -> Option<&Entity> {
        self.entities.iter().find(|e| e.id == id && e.alive)
    }
    pub fn entity_mut(&mut self, id: u64) -> Option<&mut Entity> {
        self.entities.iter_mut().find(|e| e.id == id && e.alive)
    }
}

/// Algorithm identity is explicit for canonical state and versioned replays.
/// Missing historical serialized identity means LegacySplitMix64V1, regardless
/// of the algorithm selected by a new-game constructor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RngAlgorithm {
    #[serde(rename = "legacy_splitmix64_v1")]
    LegacySplitMix64V1,
    #[serde(rename = "gm8_lcg32_v1")]
    Gm8Lcg32V1,
}

impl RngAlgorithm {
    fn historical_wire_default() -> Self {
        Self::LegacySplitMix64V1
    }
}

#[derive(Debug, Clone, Copy)]
enum RngState {
    Legacy(u64),
    Canonical(u32),
}

#[derive(Debug, Clone)]
pub struct Rng {
    state: RngState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RngSeedError {
    pub seed: u64,
}
impl std::fmt::Display for RngSeedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Canonical seed {} exceeds {}", self.seed, u32::MAX)
    }
}
impl std::error::Error for RngSeedError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceRandError {
    RequiresCanonicalAlgorithm,
    UnsupportedWidth(i64),
}
impl std::fmt::Display for SourceRandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RequiresCanonicalAlgorithm => {
                f.write_str("Signed source rand requires the canonical RNG")
            }
            Self::UnsupportedWidth(width) => {
                write!(
                    f,
                    "Source rand width {width} exceeds the exact supported domain"
                )
            }
        }
    }
}
impl std::error::Error for SourceRandError {}

impl Serialize for Rng {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        match self.state {
            RngState::Legacy(state) => {
                // Preserve historical Game serialization bytes and state hashes.
                // Legacy identity is unambiguous for this old wire shape.
                let mut wire = serializer.serialize_struct("Rng", 1)?;
                wire.serialize_field("state", &state)?;
                wire.end()
            }
            RngState::Canonical(state) => {
                let mut wire = serializer.serialize_struct("Rng", 2)?;
                wire.serialize_field("algorithm", &RngAlgorithm::Gm8Lcg32V1)?;
                wire.serialize_field("state", &state)?;
                wire.end()
            }
        }
    }
}
// Compatibility covers the observed JSON-object save/replay contract. The
// historical derived struct's sequence/binary forms are not a versioned public
// wire format here; this adapter does not claim to preserve those forms.
impl<'de> Deserialize<'de> for Rng {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Wire {
            #[serde(default = "RngAlgorithm::historical_wire_default")]
            algorithm: RngAlgorithm,
            state: u64,
        }
        let wire = Wire::deserialize(deserializer)?;
        match wire.algorithm {
            RngAlgorithm::LegacySplitMix64V1 => Ok(Self::legacy(wire.state)),
            RngAlgorithm::Gm8Lcg32V1 => {
                Self::try_canonical(wire.state).map_err(serde::de::Error::custom)
            }
        }
    }
}

impl Rng {
    /// New runs use canonical raw u32 seeds. External u64 inputs use
    /// try_canonical(); historical state/replays use legacy() explicitly.
    pub fn new(seed: u32) -> Self {
        Self::new_canonical(seed)
    }
    pub fn legacy(seed: u64) -> Self {
        Self {
            state: RngState::Legacy(seed),
        }
    }
    pub fn new_canonical(seed: u32) -> Self {
        Self {
            state: RngState::Canonical(seed),
        }
    }
    /// External canonical seed input is checked, never silently narrowed.
    pub fn try_canonical(seed: u64) -> Result<Self, RngSeedError> {
        u32::try_from(seed)
            .map(Self::new_canonical)
            .map_err(|_| RngSeedError { seed })
    }
    pub fn algorithm(&self) -> RngAlgorithm {
        match self.state {
            RngState::Legacy(_) => RngAlgorithm::LegacySplitMix64V1,
            RngState::Canonical(_) => RngAlgorithm::Gm8Lcg32V1,
        }
    }
    /// Current state bits, with no draw. Canonical state occupies the low32 bits.
    pub fn state_bits(&self) -> u64 {
        match self.state {
            RngState::Legacy(state) => state,
            RngState::Canonical(state) => u64::from(state),
        }
    }
    /// One diagnostic raw draw. Canonical returns one new u32 state widened to
    /// u64; it does not combine two draws. This is not a GML random wrapper.
    pub fn next_u64(&mut self) -> u64 {
        match &mut self.state {
            RngState::Legacy(state) => {
                *state = state.wrapping_add(0x9e3779b97f4a7c15);
                let mut z = *state;
                z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
                z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
                z ^ (z >> 31)
            }
            RngState::Canonical(state) => {
                *state = state.wrapping_mul(0x08088405).wrapping_add(1);
                u64::from(*state)
            }
        }
    }
    /// Canonical selection uses unsigned multiply-high and always draws,
    /// including upper0. It equals source floor(random(upper)) for integer
    /// widths <=2^21. For larger widths this is the integer-core contract;
    /// arbitrary real-wrapper rounding is not claimed. Legacy keeps modulo
    /// selection and its historical no-draw upper0 behavior.
    pub fn range(&mut self, upper: u32) -> u32 {
        match self.algorithm() {
            RngAlgorithm::LegacySplitMix64V1 => {
                if upper == 0 {
                    0
                } else {
                    (self.next_u64() % u64::from(upper)) as u32
                }
            }
            RngAlgorithm::Gm8Lcg32V1 => ((self.next_u64() * u64::from(upper)) >> 32) as u32,
        }
    }
    /// Exact finite-integer subset of source floor(random(max-min+1))+min.
    /// Both endpoints are i32; signed width magnitude must be <=2^21. This
    /// includes zero and negative widths, each consuming exactly one draw.
    /// Unsupported calls return an error before drawing rather than inventing
    /// endpoint swapping, clamping, or arbitrary x87 floating semantics.
    pub fn source_rand(&mut self, min: i32, max: i32) -> Result<i64, SourceRandError> {
        if self.algorithm() != RngAlgorithm::Gm8Lcg32V1 {
            return Err(SourceRandError::RequiresCanonicalAlgorithm);
        }
        let width = i64::from(max) - i64::from(min) + 1;
        if width.abs() > (1_i64 << 21) {
            return Err(SourceRandError::UnsupportedWidth(width));
        }
        let state = self.next_u64() as i64;
        Ok((state * width).div_euclid(1_i64 << 32) + i64::from(min))
    }
    pub fn chance(&mut self, numerator: u32, denominator: u32) -> bool {
        self.range(denominator) < numerator
    }
    pub fn unit(&mut self) -> f64 {
        match self.algorithm() {
            RngAlgorithm::LegacySplitMix64V1 => {
                (self.next_u64() >> 11) as f64 / (1_u64 << 53) as f64
            }
            RngAlgorithm::Gm8Lcg32V1 => self.next_u64() as f64 * (1.0 / 4294967296.0),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GameEvent {
    Sound {
        name: String,
        pos: Vec2,
    },
    /// Source SS_IsSoundPlaying gate: the audio host starts this clip only
    /// after its previous playback has finished.
    SoundIfIdle {
        name: String,
        pos: Vec2,
    },
    /// Stop the named effect resource, including playback waiting for decode.
    SoundStop {
        name: String,
    },
    /// Persistent source resource pan; ordinary play calls retain this setting.
    SoundPan {
        name: String,
        pan: f32,
    },
    MusicPlay {
        name: String,
        looping: bool,
        if_idle: bool,
    },
    MusicStopAll,
    /// Source music frequency in Hz. This never changes the simulation clock.
    MusicFrequency {
        name: String,
        hz: u32,
    },
    Damage {
        amount: i32,
    },
    Collected {
        object: String,
        value: u32,
    },
    Explosion {
        pos: Vec2,
        radius: f64,
    },
    Message(String),
    Dialogue(SourceMessage),
    LevelComplete,
    Died,
    Won,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceMusicCommand {
    Play {
        name: String,
        looping: bool,
        if_idle: bool,
    },
    StopAll,
    Frequency {
        name: String,
        hz: i64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceMusicState {
    pub enabled: bool,
    #[serde(default)]
    pub frequencies: BTreeMap<String, i64>,
    #[serde(skip)]
    pub pending: Vec<SourceMusicCommand>,
}
impl Default for SourceMusicState {
    fn default() -> Self {
        Self {
            enabled: true,
            frequencies: BTreeMap::new(),
            pending: Vec::new(),
        }
    }
}
impl SourceMusicState {
    pub fn frequency(&self, name: &str) -> i64 {
        self.frequencies.get(name).copied().unwrap_or(44_100)
    }
    pub fn take_commands(&mut self) -> Vec<SourceMusicCommand> {
        std::mem::take(&mut self.pending)
    }
    /// Drain the old Game before carrying these globals into a replacement.
    pub fn session_state(&self) -> Self {
        Self {
            enabled: self.enabled,
            frequencies: self.frequencies.clone(),
            pending: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceMessage {
    pub first: String,
    pub second: String,
    pub ticks: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunState {
    Playing,
    Dead,
    Transition,
    Ending,
    Won,
}

/// Generation behavior is independent of RNG identity. Missing historical
/// snapshots retain their original behavior, including old canonical snapshots.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GenerationRevision {
    #[default]
    HistoricalV1,
    SourceClassicV2,
}
impl GenerationRevision {
    fn is_historical(&self) -> bool {
        *self == Self::HistoricalV1
    }
}

fn classic_down_to_run() -> bool {
    true
}

/// Source view_object[0], independent of the followed actor's controller state.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ViewTarget {
    #[default]
    Player,
    Object(String),
    None,
}
impl ViewTarget {
    fn is_player(&self) -> bool {
        *self == Self::Player
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    /// Persistent source globals plus an outbox that survives event clearing.
    #[serde(default)]
    pub source_music: SourceMusicState,
    /// Immutable state observed by the source Draw phase, before Animation.
    #[serde(skip)]
    pub draw_view: Option<Box<Game>>,
    pub seed: u64,
    pub tick: u64,
    pub level_tick: u64,
    pub rules: RulesPreset,
    /// scrInit defaults downToRun to true; settings.cfg can disable it.
    #[serde(default = "classic_down_to_run")]
    pub down_to_run: bool,
    pub player: Player,
    pub world: World,
    pub rng: Rng,
    #[serde(default, skip_serializing_if = "GenerationRevision::is_historical")]
    pub generation_revision: GenerationRevision,
    pub events: Vec<GameEvent>,
    pub state: RunState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ending: Option<crate::ending::EndingState>,
    #[serde(default, skip_serializing_if = "ViewTarget::is_player")]
    pub view_target: ViewTarget,
    pub camera: Vec2,
    pub previous_camera: Vec2,
    /// Original is omitted so historical snapshots and hashes remain unchanged.
    #[serde(default, skip_serializing_if = "crate::SimulationView::is_original")]
    pub simulation_view: crate::SimulationView,
    pub flags: BTreeSet<String>,
    pub data: BTreeMap<String, f64>,
    #[serde(default)]
    pub lifetime_stats: crate::statistics::LifetimeStats,
    #[serde(default)]
    pub dispatcher: crate::dispatcher::State,
    #[serde(default = "crate::dispatcher::unset_alarms")]
    pub alarms: [i32; 12],
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opening: Option<crate::opening::OpeningState>,
    #[serde(default)]
    pub game_over: crate::progression::GameOverState,
}
impl Game {
    pub fn set_simulation_view_extent(
        &mut self,
        width: f64,
        height: f64,
    ) -> Result<(), crate::SimulationViewError> {
        self.simulation_view = crate::SimulationView::from_extent(width, height)?;
        Ok(())
    }
    /// Evaluate at each source callback; do not cache the interpolated Draw camera.
    pub fn simulation_view_rect(&self) -> Rect {
        let policy = if !self.is_level() || self.opening.is_some() || self.ending.is_some() {
            crate::SimulationView::Original
        } else {
            self.simulation_view
        };
        policy.rect(
            self.camera,
            self.world.width as f64 * 16.0,
            self.world.height as f64 * 16.0,
        )
    }
    pub fn presentation(&self) -> &Game {
        self.draw_view.as_deref().unwrap_or(self)
    }
    pub fn capture_draw_view(&mut self) {
        self.draw_view = None;
        let mut draw = Box::new(self.clone());
        if !self.flags.contains("render_discontinuity") {
            self.dispatcher
                .prepare_draw_interpolation(&mut draw.world.entities);
        }
        self.draw_view = Some(draw);
    }
    /// Create callbacks run synchronously before producer assignments.
    pub fn spawn(&mut self, object: impl Into<String>, pos: Vec2) -> u64 {
        let id = self.world.spawn(object, pos);
        crate::dispatcher::create(self, id);
        id
    }
    pub fn is_level(&self) -> bool {
        if [
            "hub",
            "hub_title",
            "hub_scores",
            "hub_sun",
            "hub_moon",
            "hub_stars",
        ]
        .iter()
        .any(|flag| self.flags.contains(*flag))
            || self.opening.is_some()
        {
            return false;
        }
        if self
            .ending
            .as_ref()
            .is_some_and(|ending| ending.stage != crate::ending::EndingStage::Exit)
        {
            return false;
        }
        if self.flags.contains("tutorial")
            || self.flags.contains("custom_level")
            || self.flags.contains("custom")
        {
            return true;
        }
        if self.world.name.starts_with('r') {
            return matches!(
                self.world.name.as_str(),
                "rTutorial" | "rLoadLevel" | "rLevel" | "rLevel2" | "rLevel3" | "rOlmec"
            );
        }
        (1..=16).contains(&self.world.level)
    }
    pub fn is_load_level(&self) -> bool {
        self.world.name == "rLoadLevel"
            || self.flags.contains("custom")
            || self.flags.contains("custom_level")
    }
    pub fn is_real_level(&self) -> bool {
        self.is_level()
            && !self.flags.contains("tutorial")
            && !self.flags.contains("custom")
            && !self.flags.contains("custom_level")
            && !matches!(self.world.name.as_str(), "rTutorial" | "rLoadLevel")
    }
    /// The oGame source clock starts at 1 on every room entry, then
    /// increments before simulation and wraps after 100,000,000.
    pub fn engine_time(&self) -> u64 {
        self.level_tick.wrapping_add(1) % 100_000_001
    }
    pub fn shell(seed: u32, rules: RulesPreset) -> Self {
        Self::shell_from_rng(u64::from(seed), rules, Rng::new(seed))
    }
    pub fn shell_with_music(seed: u32, rules: RulesPreset, music: SourceMusicState) -> Self {
        let mut game = Self::shell(seed, rules);
        game.source_music = music;
        game
    }
    pub fn try_shell(seed: u64, rules: RulesPreset) -> Result<Self, RngSeedError> {
        Ok(Self::shell_from_rng(seed, rules, Rng::try_canonical(seed)?))
    }
    pub fn legacy_shell(seed: u64, rules: RulesPreset) -> Self {
        Self::shell_from_rng(seed, rules, Rng::legacy(seed))
    }
    pub fn shell_with_algorithm(
        seed: u64,
        rules: RulesPreset,
        algorithm: RngAlgorithm,
    ) -> Result<Self, RngSeedError> {
        match algorithm {
            RngAlgorithm::Gm8Lcg32V1 => Self::try_shell(seed, rules),
            RngAlgorithm::LegacySplitMix64V1 => Ok(Self::legacy_shell(seed, rules)),
        }
    }
    fn shell_from_rng(seed: u64, rules: RulesPreset, rng: Rng) -> Self {
        let generation_revision = match rng.algorithm() {
            RngAlgorithm::LegacySplitMix64V1 => GenerationRevision::HistoricalV1,
            RngAlgorithm::Gm8Lcg32V1 => GenerationRevision::SourceClassicV2,
        };
        Self {
            draw_view: None,
            seed,
            tick: 0,
            level_tick: 0,
            rules,
            down_to_run: true,
            source_music: SourceMusicState::default(),
            player: Player::default(),
            world: World::new(42, 34, Area::Mines, 1),
            rng,
            generation_revision,
            events: Vec::new(),
            state: RunState::Playing,
            ending: None,
            opening: None,
            game_over: Default::default(),
            lifetime_stats: Default::default(),
            dispatcher: Default::default(),
            alarms: crate::dispatcher::unset_alarms(),
            view_target: ViewTarget::Player,
            camera: Vec2::default(),
            previous_camera: Vec2::default(),
            simulation_view: crate::SimulationView::Original,
            flags: BTreeSet::new(),
            data: BTreeMap::new(),
        }
    }
    pub fn sound(&mut self, name: impl Into<String>, pos: Vec2) {
        self.events.push(GameEvent::Sound {
            name: name.into(),
            pos,
        })
    }
    pub fn sound_if_idle(&mut self, name: impl Into<String>, pos: Vec2) {
        self.events.push(GameEvent::SoundIfIdle {
            name: name.into(),
            pos,
        });
    }
    pub fn stop_sound(&mut self, name: impl Into<String>) {
        self.events.push(GameEvent::SoundStop { name: name.into() });
    }
    pub fn set_sound_pan(&mut self, name: impl Into<String>, pan: f32) {
        self.events.push(GameEvent::SoundPan {
            name: name.into(),
            pan,
        });
    }
    pub fn set_music_frequency(&mut self, name: impl Into<String>, hz: u32) {
        let name = name.into();
        self.set_source_music_frequency(name, i64::from(hz));
    }
    pub fn set_source_music_frequency(&mut self, name: impl Into<String>, hz: i64) {
        let name = name.into();
        self.source_music.frequencies.insert(name.clone(), hz);
        self.source_music
            .pending
            .push(SourceMusicCommand::Frequency {
                name: name.clone(),
                hz,
            });
        if let Ok(positive) = u32::try_from(hz) {
            if positive > 0 {
                self.events
                    .push(GameEvent::MusicFrequency { name, hz: positive });
            }
        }
    }
    pub fn damage_player(&mut self, amount: i32) {
        if !self.player.cheat_invulnerable && self.player.invincible == 0 && self.player.health > 0
        {
            self.player.lose_health(amount);
            self.player.invincible = 60;
            self.player.stun = 20;
            self.events.push(GameEvent::Damage { amount });
            self.sound("sndHurt", self.player.pos);
        }
    }
}

#[cfg(test)]
mod geometry_tests {
    use super::*;

    // Preserve the pre-optimization argument extraction independently from
    // collision_transform(), so a changed default cannot make both paths agree.
    fn original_entity_bounds(e: &Entity) -> Rect {
        let xscale = e.data.get("image_xscale").copied().unwrap_or(1.0);
        let yscale = e.data.get("image_yscale").copied().unwrap_or(1.0);
        let angle = e
            .data
            .get("image_angle")
            .map(|angle| -angle.to_radians())
            .unwrap_or_else(|| {
                if matches!(e.object.as_str(), "oArrow" | "oFishBone") {
                    e.value("rotation")
                } else {
                    0.0
                }
            });
        crate::catalog::instance_mask(
            &e.object,
            &e.sprite,
            e.pos,
            e.animation,
            Vec2::new(xscale, yscale),
            angle,
        )
        .bounds()
    }

    fn original_player_bounds(p: &Player) -> Rect {
        let xscale = p
            .data
            .get("image_xscale")
            .copied()
            // Source Create starts at +1; characterDrawEvent persists facing later.
            .unwrap_or(1.0);
        let yscale = p.data.get("image_yscale").copied().unwrap_or(1.0);
        let angle = -p
            .data
            .get("image_angle")
            .copied()
            .unwrap_or(0.0)
            .to_radians();
        crate::catalog::instance_mask(
            "oPlayer1",
            &p.sprite,
            p.pos,
            p.animation,
            Vec2::new(xscale, yscale),
            angle,
        )
        .bounds()
    }

    fn assert_bounds_bits(actual: Rect, expected: Rect, context: &str) {
        let bits = |r: Rect| [r.x.to_bits(), r.y.to_bits(), r.w.to_bits(), r.h.to_bits()];
        assert_eq!(bits(actual), bits(expected), "{context}");
    }

    fn bounds_transform_cases() -> Vec<(Vec2, BTreeMap<String, f64>)> {
        [
            (Vec2::new(32.0, 48.0), vec![]),
            (Vec2::new(32.5, -48.5), vec![]),
            (Vec2::new(-0.0, 0.0), vec![]),
            (Vec2::new(32.0, 48.0), vec![("image_xscale", -1.0)]),
            (Vec2::new(32.0, 48.0), vec![("image_yscale", -1.0)]),
            (Vec2::new(32.5, -48.5), vec![("image_xscale", 4.0)]),
            (
                Vec2::new(32.0, 48.0),
                vec![("image_xscale", 2.0), ("image_yscale", 3.0)],
            ),
            (Vec2::new(32.5, -48.5), vec![("image_angle", 90.0)]),
            (Vec2::new(32.0, 48.0), vec![("image_angle", 1.5)]),
            (Vec2::new(32.0, 48.0), vec![("rotation", 0.75)]),
            (
                Vec2::new(32.0, 48.0),
                vec![("rotation", 0.75), ("image_angle", -45.0)],
            ),
            (Vec2::new(32.0, 48.0), vec![("image_xscale", 0.0)]),
            (Vec2::new(32.0, 48.0), vec![("image_yscale", 0.0)]),
            (Vec2::new(32.0, 48.0), vec![("image_xscale", f64::NAN)]),
            (Vec2::new(32.0, 48.0), vec![("image_angle", f64::NAN)]),
            (Vec2::new(f64::NAN, 48.0), vec![]),
            (Vec2::new(32.0, f64::INFINITY), vec![]),
        ]
        .into_iter()
        .map(|(pos, data)| {
            (
                pos,
                data.into_iter()
                    .map(|(key, value)| (key.into(), value))
                    .collect(),
            )
        })
        .collect()
    }

    #[test]
    fn bounds_only_path_matches_original_masks_for_every_object_and_transform() {
        let cases = bounds_transform_cases();
        for name in crate::catalog::object_names().chain(["missing", ""]) {
            let mut e = Entity::new(1, name, Vec2::default());
            for (pos, data) in &cases {
                e.pos = *pos;
                e.data = data.clone();
                for animation in [0.0, -0.0, 2.75, -0.5, f64::NAN, f64::INFINITY] {
                    e.animation = animation;
                    let expected = original_entity_bounds(&e);
                    assert_bounds_bits(e.sprite_bounds(), expected, name);
                    assert_bounds_bits(e.collision_mask().bounds(), expected, name);
                }
            }
        }
    }

    #[test]
    fn bounds_only_path_matches_source_frames_and_persisted_draw_scale() {
        let atlas: serde_json::Value =
            serde_json::from_str(include_str!("../../../assets/atlas.json")).unwrap();
        let sprites = atlas["sprites"].as_object().unwrap();
        let cases = bounds_transform_cases();
        let mut e = Entity::new(1, "oSnake", Vec2::default());
        let mut p = Player::default();
        for sprite in sprites.keys().map(String::as_str).chain(["missing", ""]) {
            e.sprite = sprite.into();
            p.sprite = sprite.into();
            for frame in 0..crate::catalog::sprite_frame_count(sprite).max(1) {
                for (pos, data) in &cases {
                    e.pos = *pos;
                    e.data = data.clone();
                    e.animation = frame as f64 + 0.5;
                    assert_bounds_bits(e.sprite_bounds(), original_entity_bounds(&e), sprite);
                    p.pos = *pos;
                    p.data = data.clone();
                    p.animation = e.animation;
                    for facing in [-1, 0, 1] {
                        p.facing = facing;
                        let expected = original_player_bounds(&p);
                        assert_bounds_bits(p.sprite_bounds(), expected, sprite);
                        assert_bounds_bits(p.collision_mask().bounds(), expected, sprite);
                        assert_bounds_bits(p.collision_mask_at(p.pos).bounds(), expected, sprite);
                    }
                }
            }
        }
    }

    #[test]
    fn collision_events_use_current_sprite_not_movement_override() {
        let mut e = Entity::new(1, "oSnake", Vec2::new(32.0, 48.0));
        e.sprite = "sRunLeft".into();
        e.animation = 2.0;
        for (name, value) in [
            ("collision_left", -2.0),
            ("collision_top", -3.0),
            ("collision_right", 4.0),
            ("collision_bottom", 5.0),
        ] {
            e.data.insert(name.into(), value);
        }
        assert_eq!(e.bounds(), Rect::new(30.0, 45.0, 6.0, 8.0));
        assert_eq!(e.sprite_bounds(), Rect::new(26.0, 42.0, 12.0, 14.0));
        e.facing = -1;
        assert_eq!(
            e.sprite_bounds(),
            Rect::new(26.0, 42.0, 12.0, 14.0),
            "enemy draw mirroring does not write image_xscale"
        );
        e.data.insert("image_xscale".into(), -1.0);
        assert_eq!(e.sprite_bounds(), Rect::new(27.0, 42.0, 12.0, 14.0));
    }

    #[test]
    fn player_sprite_changes_and_source_mirroring_are_separate_from_movement() {
        let mut p = Player {
            pos: Vec2::new(32.0, 48.0),
            facing: -1,
            ..Player::default()
        };
        assert_eq!(p.bounds(), Rect::new(27.0, 40.0, 10.0, 16.0));
        assert_eq!(p.sprite_bounds(), Rect::new(26.0, 40.0, 12.0, 16.0));
        p.facing = 1;
        assert_eq!(
            p.sprite_bounds(),
            Rect::new(26.0, 40.0, 12.0, 16.0),
            "changing facing does not execute the source Draw event"
        );
        p.data.insert("image_xscale".into(), -1.0);
        assert_eq!(p.sprite_bounds(), Rect::new(27.0, 40.0, 12.0, 16.0));
        p.sprite = "sDuckLeft".into();
        assert_eq!(p.sprite_bounds(), Rect::new(25.0, 47.0, 15.0, 9.0));
        assert_eq!(p.bounds(), Rect::new(27.0, 40.0, 10.0, 16.0));
    }

    #[test]
    fn empty_or_nonfinite_rectangles_never_collide() {
        let occupied = Rect::new(-10.0, -10.0, 20.0, 20.0);
        for invalid in [
            Rect::new(0.0, 0.0, 0.0, 1.0),
            Rect::new(0.0, 0.0, 1.0, 0.0),
            Rect::new(0.0, 0.0, -1.0, 2.0),
            Rect::new(0.0, 0.0, f64::INFINITY, 1.0),
            Rect::new(f64::NAN, 0.0, 1.0, 1.0),
        ] {
            assert!(!invalid.intersects(occupied));
            assert!(!occupied.intersects(invalid));
            assert!(!invalid.contains(Vec2::default()));
        }
    }

    #[test]
    fn solid_queries_include_live_dynamic_masks_and_open_room_void() {
        let mut world = World::new(4, 4, Area::Ice, 9);
        assert!(!world.solid_at(-1.0, 10.0));
        assert!(!world.solid_at(65.0, 10.0));
        assert!(!world.collides(Rect::new(-8.0, -8.0, 8.0, 8.0)));
        world.set_tile(0, 0, "oHardBlock");
        assert!(world.solid_at(1.0, 1.0));
        let mut block = Entity::new(11, "oPushBlock", Vec2::new(32.0, 32.0));
        // The probe must use the source sprite mask rather than this unrelated
        // platform movement override.
        for (name, value) in [
            ("collision_left", 50.0),
            ("collision_top", 50.0),
            ("collision_right", 51.0),
            ("collision_bottom", 51.0),
        ] {
            block.data.insert(name.into(), value);
        }
        world.entities.push(block);
        let probe = Vec2::new(35.0, 35.0);
        assert!(world.solid_at(probe.x, probe.y));
        assert!(!world.solid_point_except(probe, 11));
        assert!(!world.collides(Rect::new(35.0, 35.0, 1.0, 1.0)));
        assert!(world.collides_except(Rect::new(35.0, 35.0, 1.0, 1.0), 0));
        world.entities[0].held = true;
        assert!(!world.solid_at(probe.x, probe.y));
        world.entities[0].held = false;
        world.entities[0].solid = false;
        assert!(!world.solid_at(probe.x, probe.y));
        world.entities[0].solid = true;
        world.entities[0].alive = false;
        assert!(!world.solid_at(probe.x, probe.y));
        assert!(Tile::new("oDark").is_destructible());
        assert!(!Tile::new("oHardBlock").is_destructible());
    }

    #[test]
    fn engine_clock_starts_per_room_and_wraps_like_source() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        assert_eq!(game.engine_time(), 1);
        game.level_tick = 1;
        assert_eq!(game.engine_time(), 2);
        game.tick = 999;
        game.level_tick = 0;
        assert_eq!(game.engine_time(), 1);
        assert_eq!(game.tick, 999, "room clock does not modify replay lifetime");
        game.level_tick = 99_999_999;
        assert_eq!(game.engine_time(), 100_000_000);
        game.level_tick = 100_000_000;
        assert_eq!(game.engine_time(), 0);
        game.level_tick = 100_000_001;
        assert_eq!(game.engine_time(), 1);
    }

    #[test]
    fn source_shop_ice_sprite_blocks_player_in_actual_step() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.world = World::new(16, 12, Area::Mines, 1);
        for x in 0..16 {
            game.world.set_tile(x, 8, "oBrick");
        }
        game.world.set_tile(5, 7, "oSolid");
        let id = game.world.spawn("oTerrainMetadata", Vec2::new(88.0, 120.0));
        game.world
            .entity_mut(id)
            .unwrap()
            .text
            .insert("terrain_sprite".into(), "sIceBlock".into());
        game.player.pos = Vec2::new(75.0, 120.0);
        game.player.previous_pos = game.player.pos;
        game.player.state = PlayerState::Standing;
        game.player.grounded = true;
        for _ in 0..30 {
            game.step(&InputFrame {
                right: true,
                ..InputFrame::default()
            });
            assert!(
                game.player.pos.x <= 75.0,
                "shop wall escaped at tick {}: {:?}",
                game.tick,
                game.player.pos
            );
        }
    }

    #[test]
    fn wide_authored_solid_supports_player_beyond_its_storage_cell() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.world = World::new(16, 12, Area::Mines, 1);
        game.world.set_tile(5, 7, "oTitleLogo");
        game.player.pos = Vec2::new(200.0, 104.0);
        game.player.previous_pos = game.player.pos;
        game.player.state = PlayerState::Standing;
        game.player.grounded = true;
        for _ in 0..30 {
            game.step(&InputFrame::default());
            assert_eq!(
                game.player.pos.y, 104.0,
                "wide source solid lost support at tick {}",
                game.tick
            );
        }
    }

    #[test]
    fn tile_queries_use_source_thickness_extents_and_explicit_sprites() {
        let mut world = World::new(16, 12, Area::Mines, 1);
        world.set_tile(2, 3, "oThinIce");
        assert!(world.solid_at(35.0, 56.0));
        assert!(!world.solid_at(35.0, 57.0));
        assert!(!world.collides(Rect::new(35.0, 57.0, 1.0, 1.0)));
        world.set_tile(4, 3, "oMoai");
        assert!(world.solid_at(70.0, 100.0));
        assert!(world.collides(Rect::new(70.0, 100.0, 1.0, 1.0)));
        world.set_tile(10, 3, "oSolid");
        assert!(
            !world.solid_at(165.0, 50.0),
            "an abstract object has no implicit mask"
        );
        let id = world.spawn("oTerrainMetadata", Vec2::new(168.0, 56.0));
        world
            .entity_mut(id)
            .unwrap()
            .text
            .insert("terrain_sprite".into(), "sIceBlock".into());
        assert!(world.solid_at(165.0, 50.0));
        world.entity_mut(id).unwrap().alive = false;
        assert!(!world.solid_at(165.0, 50.0));
    }

    #[test]
    fn explicit_tile_sprite_can_narrow_or_extend_beyond_family_defaults() {
        let mut world = World::new(40, 12, Area::Mines, 1);
        world.set_tile(2, 3, "oBlock");
        let id = world.spawn("oTerrainMetadata", Vec2::new(40.0, 56.0));
        world
            .entity_mut(id)
            .unwrap()
            .text
            .insert("terrain_sprite".into(), "sThinIce1".into());
        assert!(world.tiles_point(Vec2::new(35.0, 56.0), "oSolid"));
        assert!(!world.tiles_point(Vec2::new(35.0, 57.0), "oSolid"));
        assert!(!world.collides(Rect::new(35.0, 57.0, 1.0, 1.0)));
        world
            .entity_mut(id)
            .unwrap()
            .text
            .insert("terrain_sprite".into(), "sCopy".into());
        // sCopy's 144-pixel width exceeds the oSolid family's widest default
        // mask (sTitle at 128 pixels). Broad-phase bounds cannot truncate it.
        assert!(world.tiles_point(Vec2::new(170.0, 50.0), "oSolid"));
        assert!(world.collides(Rect::new(170.0, 50.0, 1.0, 1.0)));
        assert!(!world.tiles_point(Vec2::new(31.9995, 50.0), "oSolid"));
        assert!(!world.tiles_point(Vec2::new(f64::NAN, 50.0), "oSolid"));
    }

    #[test]
    fn terrain_broad_phase_preserves_animated_and_oversized_override_queries() {
        let mut world = World::new(40, 12, Area::Mines, 1);
        world.set_tile(2, 3, "oSolid");
        let id = world.spawn("oTerrainMetadata", Vec2::new(40.0, 56.0));
        for sprite in ["sThinIce1", "sExplosion", "sCopy"] {
            world
                .entity_mut(id)
                .unwrap()
                .text
                .insert("terrain_sprite".into(), sprite.into());
            for frame in [0.0, 1.0, 4.0, 9.0] {
                world.entity_mut(id).unwrap().animation = frame;
                let mask = world.tile_collision_mask(2, 3);
                for x in [15.9995, 16.0, 31.9995, 32.0, 48.0, 64.0, 96.0, 170.0, 192.0] {
                    for y in [39.0, 40.0, 47.9995, 48.0, 55.0, 56.0, 57.0, 64.0, 80.0] {
                        for precision in
                            [CollisionPrecision::BoundingBox, CollisionPrecision::Precise]
                        {
                            let point = Vec2::new(x, y);
                            assert_eq!(
                                world.tiles_point_precision(point, "oSolid", precision),
                                mask.contains(point, precision),
                                "point {sprite}/{frame}/{point:?}"
                            );
                            let end = Vec2::new(x + 3.0, y);
                            assert_eq!(
                                world.tiles_line_precision(point, end, "oSolid", precision),
                                mask.intersects_axis_line(point, end, precision),
                                "line {sprite}/{frame}/{point:?}"
                            );
                            let rect = Rect::new(x, y, 3.0, 2.0);
                            assert_eq!(
                                world.tiles_collide_precision(rect, "oSolid", precision),
                                mask.intersects(rect, precision),
                                "rect {sprite}/{frame}/{point:?}"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn stun_status_retains_source_boolean_after_throw_countdown() {
        let mut player = Player::default();
        assert!(!player.is_stunned());
        player.stun = 1;
        assert!(player.is_stunned());
        player.stun = 0;
        player.state = PlayerState::Stunned;
        assert!(player.is_stunned());
        player.state = PlayerState::Standing;
        assert!(!player.is_stunned());
    }

    #[test]
    fn ladder_family_excludes_decorative_rope_cap() {
        for name in ["oLadderOrange", "oLadderTop", "oVine", "oVineTop", "oRope"] {
            assert!(Tile::new(name).is_ladder(), "{name}");
        }
        assert!(!Tile::new("oRopeTop").is_ladder());
        assert!(!Tile::new("oRopeSegment").is_ladder());
    }

    #[test]
    fn tile_queries_follow_authoritative_terrain_animation_frame() {
        let mut world = World::new(12, 12, Area::Boss, 16);
        world.set_tile(5, 5, "oLavaSolid");
        let id = world.spawn("oTerrainMetadata", Vec2::new(88.0, 88.0));
        world
            .entity_mut(id)
            .unwrap()
            .text
            .insert("terrain_sprite".into(), "sLavaTop".into());
        assert_eq!(world.tile_bounds(5, 5).y, 81.0);
        // collision_point uses the aggregate cached integer bbox even when
        // this frame's visual/rectangle bounds begin one pixel lower.
        assert!(world.tiles_point(Vec2::new(80.0, 80.0), "oSolid"));
        world.entity_mut(id).unwrap().animation = 1.0;
        assert_eq!(world.tile_bounds(5, 5).y, 80.0);
        assert!(world.tiles_point(Vec2::new(80.0, 80.0), "oSolid"));
        assert!(world.tiles_collide(Rect::new(80.0, 80.0, 1.0, 1.0), "oSolid"));
    }

    #[test]
    fn shared_world_queries_keep_explicit_source_precision() {
        let mut world = World::new(12, 12, Area::Ice, 9);
        world.set_tile(5, 5, "oDarkFall");
        let p = Vec2::new(80.0, 80.0);
        assert!(world.tiles_point(p, "oSolid"));
        assert!(!world.tiles_point_precision(p, "oSolid", CollisionPrecision::Precise));
        assert!(world.tiles_point_precision(
            Vec2::new(81.0, 80.0),
            "oSolid",
            CollisionPrecision::Precise
        ));
        assert!(!world.tiles_collide_precision(
            Rect::new(80.0, 80.0, 1.0, 1.0),
            "oSolid",
            CollisionPrecision::Precise
        ));
        let entity = Entity::new(1, "oDarkFall", p);
        assert!(entity.mask_contains(p, CollisionPrecision::BoundingBox));
        assert!(!entity.mask_contains(p, CollisionPrecision::Precise));
    }

    #[test]
    fn game_step_fractional_webball_events_preserve_transparent_and_opaque_contacts() {
        // Canonical executable dispatch -> 0x4AD1A0 -> 0x48A4F0, independently
        // evaluated with original Rock/WebBall mask bytes: all sixteen corner
        // placements miss, and all sixteen interior controls hit.
        for (rock_pos, expected_state) in [(72.0, 0), (76.0, 1)] {
            for qx in [0.0, 0.25, 0.5, 0.75] {
                for qy in [0.0, 0.25, 0.5, 0.75] {
                    let mut game = Game::shell(1, RulesPreset::Classic);
                    game.world = World::new(16, 12, Area::Mines, 1);
                    game.player.pos = Vec2::new(24.0, 24.0);
                    game.player.previous_pos = game.player.pos;
                    game.world.spawn("oRock", Vec2::new(rock_pos, rock_pos));
                    let id = game.world.spawn("oWebBall", Vec2::new(79.0 + qx, 82.0));
                    let web = game.world.entity_mut(id).unwrap();
                    web.vel = Vec2::new(1.0, -2.0 + qy);
                    web.data.insert("life".into(), 50.0);

                    game.step(&InputFrame::default());

                    let web = game.world.entity(id).unwrap();
                    assert_eq!(web.pos, Vec2::new(80.0 + qx, 80.0 + qy));
                    assert_eq!(
                        web.state, expected_state,
                        "rock={rock_pos}, qx={qx}, qy={qy}"
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod canonical_rng_stage_one_tests {
    use super::*;

    // Frozen independently from executable bytes plus Python arithmetic.
    // These are static predictions, not original-runtime captures.
    const CANONICAL: [(u32, [(u32, u64, u32); 8]); 5] = [
        (
            0,
            [
                (1, 0x3df0000000000000, 0),
                (134775814, 0x3fa011080c000000, 1),
                (3698175007, 0x3feb8db583e00000, 51),
                (870078620, 0x3fc9ee2c4e000000, 12),
                (1172187917, 0x3fd1778ac3400000, 16),
                (2884733762, 0x3fe57e3168400000, 40),
                (1368768587, 0x3fd4657012c00000, 19),
                (694906232, 0x3fc4b5b6bc000000, 9),
            ],
        ),
        (
            1,
            [
                (134775814, 0x3fa011080c000000, 1),
                (3698175007, 0x3feb8db583e00000, 51),
                (870078620, 0x3fc9ee2c4e000000, 12),
                (1172187917, 0x3fd1778ac3400000, 16),
                (2884733762, 0x3fe57e3168400000, 40),
                (1368768587, 0x3fd4657012c00000, 19),
                (694906232, 0x3fc4b5b6bc000000, 9),
                (1598751577, 0x3fd7d2c0d6400000, 22),
            ],
        ),
        (
            42,
            [
                (1365616851, 0x3fd4596a34c00000, 19),
                (3768653856, 0x3fec142304000000, 52),
                (882243745, 0x3fca4afc50800000, 12),
                (673767206, 0x3fc4146f93000000, 9),
                (84669375, 0x3f942fcefc000000, 1),
                (3638705852, 0x3feb1c47d7800000, 50),
                (3131320749, 0x3fe7548535a00000, 43),
                (1705058, 0x3f3a046200000000, 0),
            ],
        ),
        (
            2147483648,
            [
                (2147483649, 0x3fe0000000200000, 30),
                (2282259462, 0x3fe1011080c00000, 31),
                (1550691359, 0x3fd71b6b07c00000, 21),
                (3017562268, 0x3fe67b8b13800000, 42),
                (3319671565, 0x3fe8bbc561a00000, 46),
                (737250114, 0x3fc5f8c5a1000000, 10),
                (3516252235, 0x3fea32b809600000, 49),
                (2842389880, 0x3fe52d6daf000000, 39),
            ],
        ),
        (
            4294967295,
            [
                (4160191484, 0x3feefeef7f800000, 58),
                (866343917, 0x3fc9d1adf6800000, 12),
                (2231304098, 0x3fe09fdff4400000, 31),
                (567969323, 0x3fc0ed4315800000, 7),
                (3754609368, 0x3febf9595b000000, 52),
                (105731641, 0x3f993558e4000000, 1),
                (2042630942, 0x3fde7004c7800000, 28),
                (4086028183, 0x3fee717af2e00000, 57),
            ],
        ),
    ];

    #[test]
    fn canonical_raw_unit_and_range_match_frozen_independent_vectors() {
        for (seed, draws) in CANONICAL {
            let mut raw = Rng::new_canonical(seed);
            let mut unit = Rng::new_canonical(seed);
            let mut range = Rng::new_canonical(seed);
            for (state, unit_bits, selected) in draws {
                assert_eq!(raw.next_u64(), u64::from(state));
                assert_eq!(unit.unit().to_bits(), unit_bits);
                assert_eq!(range.range(60), selected);
                for rng in [&raw, &unit, &range] {
                    assert_eq!(rng.state_bits(), u64::from(state));
                    assert_eq!(rng.algorithm(), RngAlgorithm::Gm8Lcg32V1);
                }
            }
        }
    }

    #[test]
    fn canonical_zero_width_draws_while_legacy_zero_width_keeps_old_behavior() {
        let mut canonical = Rng::new_canonical(0);
        assert_eq!(canonical.range(0), 0);
        assert_eq!(canonical.state_bits(), 1);
        let mut zero_source = Rng::new_canonical(0);
        assert_eq!(zero_source.source_rand(10, 9), Ok(10));
        assert_eq!(zero_source.state_bits(), 1);
        let mut zero_chance = Rng::new_canonical(0);
        assert!(!zero_chance.chance(0, 0));
        assert_eq!(zero_chance.state_bits(), 1);
        let mut legacy = Rng::legacy(u64::MAX);
        assert_eq!(legacy.range(0), 0);
        assert_eq!(legacy.state_bits(), u64::MAX);
    }

    #[test]
    fn signed_source_widths_match_independent_floor_vectors() {
        let vectors: &[(u32, i32, i64, u32)] = &[
            (0, -99, -1, 1),
            (0, -1, -1, 1),
            (0, 0, 0, 1),
            (0, 1, 0, 1),
            (0, 2, 0, 1),
            (0, 60, 0, 1),
            (0, 901, 0, 1),
            (0, 1200, 0, 1),
            (0, 2097152, 0, 1),
            (1, -99, -4, 134775814),
            (1, -1, -1, 134775814),
            (1, 0, 0, 134775814),
            (1, 1, 0, 134775814),
            (1, 2, 0, 134775814),
            (1, 60, 1, 134775814),
            (1, 901, 28, 134775814),
            (1, 1200, 37, 134775814),
            (1, 2097152, 65808, 134775814),
            (42, -99, -32, 1365616851),
            (42, -1, -1, 1365616851),
            (42, 0, 0, 1365616851),
            (42, 1, 0, 1365616851),
            (42, 2, 0, 1365616851),
            (42, 60, 19, 1365616851),
            (42, 901, 286, 1365616851),
            (42, 1200, 381, 1365616851),
            (42, 2097152, 666805, 1365616851),
            (2147483648, -99, -50, 2147483649),
            (2147483648, -1, -1, 2147483649),
            (2147483648, 0, 0, 2147483649),
            (2147483648, 1, 0, 2147483649),
            (2147483648, 2, 1, 2147483649),
            (2147483648, 60, 30, 2147483649),
            (2147483648, 901, 450, 2147483649),
            (2147483648, 1200, 600, 2147483649),
            (2147483648, 2097152, 1048576, 2147483649),
            (4294967295, -99, -96, 4160191484),
            (4294967295, -1, -1, 4160191484),
            (4294967295, 0, 0, 4160191484),
            (4294967295, 1, 0, 4160191484),
            (4294967295, 2, 1, 4160191484),
            (4294967295, 60, 58, 4160191484),
            (4294967295, 901, 872, 4160191484),
            (4294967295, 1200, 1162, 4160191484),
            (4294967295, 2097152, 2031343, 4160191484),
        ];
        for &(seed, width, expected, state) in vectors {
            let mut rng = Rng::new_canonical(seed);
            assert_eq!(rng.source_rand(0, width - 1), Ok(expected));
            assert_eq!(rng.state_bits(), u64::from(state));
        }
        let mut inclusive = Rng::new_canonical(42);
        assert_eq!(inclusive.source_rand(1, 60), Ok(20));
        let mut reversed = Rng::new_canonical(42);
        assert_eq!(reversed.source_rand(100, 0), Ok(68));
        let mut max_endpoint = Rng::new_canonical(42);
        assert_eq!(
            max_endpoint.source_rand(i32::MAX, i32::MAX),
            Ok(i64::from(i32::MAX))
        );
        let mut min_endpoint = Rng::new_canonical(42);
        assert_eq!(
            min_endpoint.source_rand(i32::MIN, i32::MIN),
            Ok(i64::from(i32::MIN))
        );
    }

    #[test]
    fn unsupported_source_widths_and_legacy_calls_do_not_draw() {
        let mut rng = Rng::new_canonical(42);
        assert_eq!(
            rng.source_rand(0, 1 << 21),
            Err(SourceRandError::UnsupportedWidth((1 << 21) + 1))
        );
        assert_eq!(
            rng.source_rand(0, -(1 << 21) - 2),
            Err(SourceRandError::UnsupportedWidth(-(1 << 21) - 1))
        );
        assert_eq!(rng.state_bits(), 42);
        let mut legacy = Rng::legacy(u64::MAX);
        assert_eq!(
            legacy.source_rand(0, 0),
            Err(SourceRandError::RequiresCanonicalAlgorithm)
        );
        assert_eq!(legacy.state_bits(), u64::MAX);
    }

    #[test]
    fn canonical_seed_validation_rejects_full_width_legacy_seeds_without_truncation() {
        assert_eq!(
            Rng::try_canonical(u64::from(u32::MAX))
                .unwrap()
                .state_bits(),
            u64::from(u32::MAX)
        );
        for seed in [1_u64 << 32, (1_u64 << 53) + 1, u64::MAX] {
            assert_eq!(Rng::try_canonical(seed).unwrap_err(), RngSeedError { seed });
            assert_eq!(Rng::legacy(seed).state_bits(), seed);
        }
    }

    #[test]
    fn legacy_raw_draws_match_frozen_port_vectors_for_all_64_seed_bits() {
        let cases: &[(u64, [u64; 3])] = &[
            (
                0,
                [
                    16294208416658607535,
                    7960286522194355700,
                    487617019471545679,
                ],
            ),
            (
                1,
                [
                    10451216379200822465,
                    13757245211066428519,
                    17911839290282890590,
                ],
            ),
            (
                42,
                [
                    13679457532755275413,
                    2949826092126892291,
                    5139283748462763858,
                ],
            ),
            (
                4294967295,
                [
                    8336509955162079680,
                    6998667510010663860,
                    17170758627551043187,
                ],
            ),
            (
                4294967296,
                [
                    14135772400868000056,
                    4011968247314693437,
                    12634733894835514069,
                ],
            ),
            (
                9007199254740993,
                [
                    386430782533759031,
                    13113306883110016354,
                    5451226349904013435,
                ],
            ),
            (
                18446744073709551615,
                [
                    16490336266968443936,
                    16834447057089888969,
                    4048727598324417001,
                ],
            ),
        ];
        for &(seed, words) in cases {
            let mut rng = Rng::legacy(seed);
            for expected in words {
                assert_eq!(rng.next_u64(), expected);
            }
            assert_eq!(rng.algorithm(), RngAlgorithm::LegacySplitMix64V1);
        }
    }

    #[test]
    fn historical_untagged_and_explicit_tagged_legacy_imports_preserve_state() {
        for encoded in [
            r#"{"state":18446744073709551615}"#,
            r#"{"algorithm":"legacy_splitmix64_v1","state":18446744073709551615}"#,
        ] {
            let mut rng: Rng = serde_json::from_str(encoded).unwrap();
            assert_eq!(rng.algorithm(), RngAlgorithm::LegacySplitMix64V1);
            assert_eq!(rng.state_bits(), u64::MAX);
            assert_eq!(
                serde_json::to_string(&rng).unwrap(),
                r#"{"state":18446744073709551615}"#
            );
            assert_eq!(rng.next_u64(), 16490336266968443936);
        }
    }

    #[test]
    fn canonical_tagged_state_round_trips_without_changing_next_draw() {
        for state in [0, 1 << 31, u32::MAX] {
            let rng = Rng::new_canonical(state);
            let encoded = serde_json::to_string(&rng).unwrap();
            assert_eq!(
                encoded,
                format!(r#"{{"algorithm":"gm8_lcg32_v1","state":{state}}}"#)
            );
            let mut decoded: Rng = serde_json::from_str(&encoded).unwrap();
            assert_eq!(decoded.algorithm(), RngAlgorithm::Gm8Lcg32V1);
            assert_eq!(decoded.state_bits(), u64::from(state));
            assert_eq!(decoded.next_u64(), rng.clone().next_u64());
        }
    }

    #[test]
    fn malformed_or_unknown_rng_wire_state_is_rejected() {
        for encoded in [
            r#"{"algorithm":"gm8_lcg32_v1","state":4294967296}"#,
            r#"{"algorithm":"gm8_lcg32_v1","state":-1}"#,
            r#"{"algorithm":"gm8_lcg32_v1","state":1.5}"#,
            r#"{"algorithm":"unknown","state":42}"#,
            r#"{"algorithm":null,"state":42}"#,
            r#"{"algorithm":"gm8_lcg32_v1"}"#,
            r#"{"state":42,"state":43}"#,
        ] {
            assert!(serde_json::from_str::<Rng>(encoded).is_err(), "{encoded}");
        }
    }

    #[test]
    fn legacy_rng_wire_bytes_preserve_the_historical_envelope_hash_recipe() {
        #[derive(Serialize)]
        struct Envelope {
            seed: u64,
            rng: Rng,
            after: u32,
        }
        let old = Envelope {
            seed: u64::MAX,
            rng: Rng::legacy(u64::MAX),
            after: 7,
        };
        let encoded = serde_json::to_vec(&old).unwrap();
        assert_eq!(
            encoded,
            br#"{"seed":18446744073709551615,"rng":{"state":18446744073709551615},"after":7}"#
        );
        let hash = encoded
            .into_iter()
            .fold(0xcbf29ce484222325_u64, |hash, byte| {
                (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
            });
        // Independently calculated from the literal historical bytes in Python.
        assert_eq!(hash, 0x0a36895396092cd3);
    }
}

#[cfg(test)]
#[path = "model_generated_graphics_tests.rs"]
mod generated_graphics_tests;
