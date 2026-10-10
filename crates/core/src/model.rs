use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const TICK_RATE: u32 = 30;
pub const TILE_SIZE: f64 = 16.0;
pub const VIEW_WIDTH: f64 = 320.0;
pub const VIEW_HEIGHT: f64 = 240.0;

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
}
impl Default for Player {
    fn default() -> Self {
        Self {
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
    /// Current source sprite mask for collision events, separate from the
    /// platform engine's custom rectangle returned by `bounds()`.
    pub fn sprite_bounds(&self) -> Rect {
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
        crate::catalog::instance_sprite_bounds(
            "oPlayer1",
            &self.sprite,
            self.pos,
            self.animation,
            Vec2::new(xscale, yscale),
            angle,
        )
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
}
impl Entity {
    /// Collision-event mask from the current sprite/object mask. Movement
    /// overrides in `data["collision_*"]` deliberately do not affect this.
    pub fn sprite_bounds(&self) -> Rect {
        // Unlike enemy draw_sprite_ext mirroring, Damsel's Draw event writes
        // the actual image_xscale property and therefore changes her mask.
        let default_xscale = if matches!(
            self.object.as_str(),
            "oDamsel" | "oPDummy" | "oPDummy2" | "oPDummy3" | "oPDummy6"
        ) && self.facing > 0
        {
            -1.0
        } else {
            1.0
        };
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
        crate::catalog::instance_sprite_bounds(
            &self.object,
            &self.sprite,
            self.pos,
            self.animation,
            Vec2::new(xscale, yscale),
            angle,
        )
    }
    pub fn new(id: u64, object: impl Into<String>, pos: Vec2) -> Self {
        let object = object.into();
        let sprite = crate::catalog::sprite(&object)
            .map(str::to_owned)
            .unwrap_or_else(|| format!("s{}", object.strip_prefix('o').unwrap_or(&object)));
        let solid = crate::catalog::is_solid(&object);
        Self {
            id,
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

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tile {
    pub object: String,
}
impl Tile {
    pub fn new(object: impl Into<String>) -> Self {
        Self {
            object: object.into(),
        }
    }
    pub fn is_solid(&self) -> bool {
        crate::catalog::is_solid(&self.object)
    }
    pub fn is_ladder(&self) -> bool {
        matches!(
            self.object.as_str(),
            "oLadder"
                | "oLadderOrange"
                | "oLadderTop"
                | "oVine"
                | "oVineTop"
                | "oRope"
                | "oRopeTop"
                | "oRopeSegment"
        )
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
            self.tiles[y * self.width + x] = Tile::new(object)
        }
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
        self.tile(
            (x / TILE_SIZE).floor() as i32,
            (y / TILE_SIZE).floor() as i32,
        )
        .is_some_and(Tile::is_solid)
            || self.entities.iter().any(|e| {
                e.id != excluded
                    && e.alive
                    && e.solid
                    && !e.held
                    && e.sprite_bounds().contains(Vec2::new(x, y))
            })
    }
    /// Tile-only rectangle query. Use collides_except for dynamic solids too.
    pub fn collides(&self, r: Rect) -> bool {
        if !r.is_valid() {
            return false;
        }
        // Only authored cells can collide. Clamp scans to room storage so a
        // large valid probe through open space does not traverse billions of
        // nonexistent cells.
        let x0 = ((r.x / 16.0).floor() as i32).max(0);
        let y0 = ((r.y / 16.0).floor() as i32).max(0);
        let x1 = (((r.x + r.w - 0.001) / 16.0).floor() as i32)
            .min((self.width as i32).saturating_sub(1));
        let y1 = (((r.y + r.h - 0.001) / 16.0).floor() as i32)
            .min((self.height as i32).saturating_sub(1));
        for y in y0..=y1 {
            for x in x0..=x1 {
                if self.tile(x, y).is_some_and(Tile::is_solid) {
                    return true;
                }
            }
        }
        false
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
        crate::enemies::init_entity(&mut e);
        crate::items::init_entity(&mut e);
        self.entities.push(e);
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
    Sound { name: String, pos: Vec2 },
    Damage { amount: i32 },
    Collected { object: String, value: u32 },
    Explosion { pos: Vec2, radius: f64 },
    Message(String),
    LevelComplete,
    Died,
    Won,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunState {
    Playing,
    Dead,
    Transition,
    Won,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub seed: u64,
    pub tick: u64,
    pub level_tick: u64,
    pub rules: RulesPreset,
    pub player: Player,
    pub world: World,
    pub rng: Rng,
    pub events: Vec<GameEvent>,
    pub state: RunState,
    pub camera: Vec2,
    pub previous_camera: Vec2,
    pub flags: BTreeSet<String>,
    pub data: BTreeMap<String, f64>,
}
impl Game {
    /// The oGame source clock starts at 1 on every room entry, then
    /// increments before simulation and wraps after 100,000,000.
    pub fn engine_time(&self) -> u64 {
        self.level_tick.wrapping_add(1) % 100_000_001
    }
    pub fn shell(seed: u64, rules: RulesPreset) -> Self {
        Self {
            seed,
            tick: 0,
            level_tick: 0,
            rules,
            player: Player::default(),
            world: World::new(42, 34, Area::Mines, 1),
            rng: Rng::new(seed),
            events: Vec::new(),
            state: RunState::Playing,
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
}
