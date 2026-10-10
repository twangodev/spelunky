use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const TICK_RATE: u32 = 30;
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
    pub up: bool,
    pub down: bool,
    pub run: bool,
    pub jump: bool,
    pub jump_pressed: bool,
    pub jump_released: bool,
    pub attack: bool,
    pub attack_pressed: bool,
    #[serde(default)]
    pub start_pressed: bool,
    pub bomb_pressed: bool,
    pub rope_pressed: bool,
    pub flare_pressed: bool,
    pub cycle_pressed: bool,
    pub pay_pressed: bool,
    pub exit_pressed: bool,
}
impl InputFrame {
    pub fn clear_edges(&mut self) {
        self.jump_pressed = false;
        self.jump_released = false;
        self.attack_pressed = false;
        self.start_pressed = false;
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
    pub bombs: i32,
    pub ropes: i32,
    pub money: u32,
    pub equipment: BTreeSet<String>,
    pub held: Option<u64>,
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
            bombs: 4,
            ropes: 4,
            money: 0,
            equipment: BTreeSet::new(),
            held: None,
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
            data: BTreeMap::new(),
        }
    }
}
impl Player {
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
        self.stun > 0 || self.state == PlayerState::Stunned
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
        let (scale, angle) = self.collision_transform();
        crate::catalog::instance_mask(
            "oPlayer1",
            &self.sprite,
            self.pos,
            self.animation,
            scale,
            angle,
        )
    }
    fn collision_transform(&self) -> (Vec2, f64) {
        let xscale = self
            .data
            .get("image_xscale")
            .copied()
            .unwrap_or(if self.facing > 0 { -1.0 } else { 1.0 });
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
        let (scale, angle) = self.collision_transform();
        crate::catalog::instance_mask(
            &self.object,
            &self.sprite,
            self.pos,
            self.animation,
            scale,
            angle,
        )
    }
    fn collision_transform(&self) -> (Vec2, f64) {
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
                if matches!(self.object.as_str(), "oArrow" | "oFishBone") {
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
            data: BTreeMap::new(),
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct World {
    pub width: usize,
    pub height: usize,
    pub tiles: Vec<Tile>,
    pub entities: Vec<Entity>,
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
}
impl World {
    pub fn new(width: usize, height: usize, area: Area, level: u32) -> Self {
        Self {
            width,
            height,
            tiles: vec![Tile::default(); width * height],
            entities: Vec::new(),
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
        }
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

    pub fn tile_collision_mask(&self, x: i32, y: i32) -> crate::catalog::CollisionMask {
        let origin = Vec2::new(f64::from(x) * TILE_SIZE, f64::from(y) * TILE_SIZE);
        let Some(tile) = self.tile(x, y) else {
            return crate::catalog::instance_mask("", "", origin, 0.0, Vec2::new(1.0, 1.0), 0.0);
        };
        let sprite = self.entities.iter().find_map(|metadata| {
            (metadata.alive
                && metadata.object == "oTerrainMetadata"
                && (metadata.pos.x / TILE_SIZE).floor() as i32 == x
                && (metadata.pos.y / TILE_SIZE).floor() as i32 == y)
                .then(|| {
                    metadata
                        .text
                        .get("terrain_sprite")
                        .map(|sprite| (sprite.as_str(), metadata.animation))
                })
                .flatten()
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
        self.tiles_match(r, group, None, precision)
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
            precision,
        )
    }

    fn tiles_match(
        &self,
        r: Rect,
        group: &str,
        point: Option<Vec2>,
        precision: CollisionPrecision,
    ) -> bool {
        if !r.is_valid() || self.width == 0 {
            return false;
        }
        let matches = |mask: &crate::catalog::CollisionMask| {
            point.map_or_else(
                || mask.intersects(r, precision),
                |p| mask.contains(p, precision),
            )
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
            if !crate::catalog::is_a(&tile.object, group) {
                continue;
            }
            let mask = crate::catalog::instance_mask(
                &tile.object,
                sprite,
                Vec2::new(f64::from(x) * TILE_SIZE, f64::from(y) * TILE_SIZE),
                metadata.animation,
                Vec2::new(1.0, 1.0),
                0.0,
            );
            let near = x >= x0 && x <= x1 && y >= y0 && y <= y1;
            if !near && !matches(&mask) {
                continue;
            }
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rng {
    state: u64,
}
impl Rng {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }
    pub fn range(&mut self, upper: u32) -> u32 {
        if upper == 0 {
            0
        } else {
            (self.next_u64() % u64::from(upper)) as u32
        }
    }
    pub fn chance(&mut self, numerator: u32, denominator: u32) -> bool {
        self.range(denominator) < numerator
    }
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1_u64 << 53) as f64
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    /// Immutable state observed by the source Draw phase, before Animation.
    #[serde(skip)]
    pub draw_view: Option<Box<Game>>,
    pub seed: u64,
    pub tick: u64,
    pub level_tick: u64,
    pub rules: RulesPreset,
    pub player: Player,
    pub world: World,
    pub rng: Rng,
    pub events: Vec<GameEvent>,
    pub state: RunState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ending: Option<crate::ending::EndingState>,
    pub camera: Vec2,
    pub previous_camera: Vec2,
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
    pub fn presentation(&self) -> &Game {
        self.draw_view.as_deref().unwrap_or(self)
    }
    pub fn capture_draw_view(&mut self) {
        self.draw_view = None;
        self.draw_view = Some(Box::new(self.clone()));
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
    pub fn shell(seed: u64, rules: RulesPreset) -> Self {
        Self {
            draw_view: None,
            seed,
            tick: 0,
            level_tick: 0,
            rules,
            player: Player::default(),
            world: World::new(42, 34, Area::Mines, 1),
            rng: Rng::new(seed),
            events: Vec::new(),
            state: RunState::Playing,
            ending: None,
            opening: None,
            game_over: Default::default(),
            lifetime_stats: Default::default(),
            dispatcher: Default::default(),
            alarms: crate::dispatcher::unset_alarms(),
            camera: Vec2::default(),
            previous_camera: Vec2::default(),
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
    pub fn damage_player(&mut self, amount: i32) {
        if self.player.invincible == 0 && self.player.health > 0 {
            self.player.health -= amount;
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
            .unwrap_or(if p.facing > 0 { -1.0 } else { 1.0 });
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
    fn bounds_only_path_matches_original_current_frames_and_player_facing() {
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
