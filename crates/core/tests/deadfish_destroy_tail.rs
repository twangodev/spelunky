//! Source-derived initial branch fixtures are explicitly controlled; the pistol
//! journey uses authored geometry and ordinary inputs without actor/RNG edits.
use spelunky_core::*;

#[derive(Clone, Copy)]
enum Mode {
    Legacy,
    Canonical,
}
impl Mode {
    fn game(self, rules: RulesPreset) -> Game {
        match self {
            Self::Legacy => Game::legacy_shell(42, rules),
            Self::Canonical => Game::shell(42, rules),
        }
    }
}
struct SourceTape {
    state: u64,
    canonical: bool,
}
impl SourceTape {
    fn at(game: &Game) -> Self {
        Self {
            state: game.rng.state_bits(),
            canonical: game.rng.algorithm() == RngAlgorithm::Gm8Lcg32V1,
        }
    }
    fn next(&mut self) -> u64 {
        if self.canonical {
            self.state = self.state.wrapping_mul(0x0808_8405).wrapping_add(1) & 0xffff_ffff;
            self.state
        } else {
            self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut value = self.state;
            value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            value ^ (value >> 31)
        }
    }
    fn rand(&mut self, width: u64) -> u64 {
        let raw = self.next();
        if self.canonical {
            (raw * width) >> 32
        } else {
            raw % width
        }
    }
    fn detritus_create(&mut self) {
        self.next();
        self.next();
        self.next();
        self.rand(6);
    }
}
fn count(game: &Game, object: &str) -> usize {
    game.world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == object)
        .count()
}

fn controlled_fish(
    mode: Mode,
    rules: RulesPreset,
    dead: bool,
    wet: bool,
    pause: bool,
) -> (Game, u64) {
    let mut game = mode.game(rules);
    game.world = World::new(30, 20, Area::Jungle, 5);
    game.world.name = "rLevel".into();
    game.player.pos = Vec2::new(240.0, 152.0);
    game.player.previous_pos = game.player.pos;
    if wet {
        for y in 4..9 {
            for x in 4..10 {
                game.world.set_tile(x, y, "oWaterSwim");
            }
        }
    }
    let id = game.spawn("oDeadFish", Vec2::new(96.0, 96.0));
    let fish = game.world.entity_mut(id).unwrap();
    if dead {
        fish.health = 0;
    }
    if pause {
        fish.state = 2;
        fish.timer = 0;
    }
    (game, id)
}
fn dead_dry_case(mode: Mode) {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, id) = controlled_fish(mode, rules, true, false, false);
        let mut expected = SourceTape::at(&game);
        for _ in 0..3 {
            expected.detritus_create();
        }
        game.step(&InputFrame::default());
        assert!(game.world.entity(id).is_none());
        assert_eq!(count(&game, "oBone"), 3);
        assert_eq!(
            count(&game, "oFishBone"),
            1,
            "DEATH_CONTINUES_THROUGH_FINAL_DRY_FISHBONE"
        );
        assert_eq!(game.data.get("deadfish"), Some(&1.0));
        assert_eq!(game.data.get("kills"), Some(&1.0));
        assert_eq!(game.rng.state_bits(), expected.state);
    }
}
fn dead_pause_case(mode: Mode) {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, id) = controlled_fish(mode, rules, true, true, true);
        let mut expected = SourceTape::at(&game);
        for _ in 0..3 {
            expected.detritus_create();
        }
        expected.rand(2);
        game.step(&InputFrame::default());
        assert!(game.world.entity(id).is_none());
        assert_eq!(count(&game, "oBone"), 3);
        assert_eq!(count(&game, "oFishBone"), 0);
        assert_eq!(
            game.rng.state_bits(),
            expected.state,
            "DEATH_CONTINUES_PAUSE_ZERO_FACING_RNG"
        );
    }
}
fn healthy_dry_pause_case(mode: Mode) {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, id) = controlled_fish(mode, rules, false, false, true);
        let mut expected = SourceTape::at(&game);
        expected.rand(2);
        game.step(&InputFrame::default());
        assert!(game.world.entity(id).is_none());
        assert_eq!((count(&game, "oBone"), count(&game, "oFishBone")), (0, 1));
        assert_eq!(
            game.rng.state_bits(),
            expected.state,
            "HEALTHY_PAUSE_RNG_PRECEDES_DRY_EPILOGUE"
        );
        assert_eq!(game.data.get("kills").copied().unwrap_or(0.0), 0.0);
    }
}
fn positive_controls_case(mode: Mode) {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut live, id) = controlled_fish(mode, rules, false, true, true);
        let mut expected = SourceTape::at(&live);
        expected.rand(2);
        live.step(&InputFrame::default());
        assert_eq!(live.world.entity(id).unwrap().state, 0);
        assert_eq!(live.rng.state_bits(), expected.state);
        assert_eq!((count(&live, "oBone"), count(&live, "oFishBone")), (0, 0));
        let (mut dead, id) = controlled_fish(mode, rules, true, true, false);
        let mut expected = SourceTape::at(&dead);
        for _ in 0..3 {
            expected.detritus_create();
        }
        dead.step(&InputFrame::default());
        assert!(dead.world.entity(id).is_none());
        assert_eq!(count(&dead, "oBone"), 3);
        assert_eq!(count(&dead, "oFishBone"), 0);
        assert_eq!(
            dead.rng.state_bits(),
            expected.state,
            "WET_IDLE_TAIL_HAS_NO_RANDOM_DRAW"
        );
    }
}
// Authored floor/water, stock fish and ordinary pickup/walk/fire inputs. No
// actor HP/state/timer/position or RNG edits after Create. This journey isolates
// pause/death continuation; attack steering has its own source trace.
fn ordinary_pistol_pause_zero_case(mode: Mode) {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = mode.game(rules);
        game.world = World::new(30, 20, Area::Jungle, 5);
        game.world.name = "rLevel".into();
        for x in 0..30 {
            game.world.set_tile(x, 10, "oBlock");
        }
        for y in 7..10 {
            for x in 9..13 {
                game.world.set_tile(x, y, "oWaterSwim");
            }
        }
        game.player.pos = Vec2::new(88.0, 152.0);
        game.player.previous_pos = game.player.pos;
        let pistol = game.spawn("oPistol", game.player.pos);
        let fish = game.spawn("oDeadFish", Vec2::new(176.0, 148.0));
        for _ in 0..4 {
            game.step(&InputFrame::default());
        }
        game.step(&InputFrame {
            down: true,
            ..Default::default()
        });
        game.step(&InputFrame {
            down: true,
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert_eq!(game.player.held, Some(pistol), "ACTUAL_PISTOL_PICKUP");
        game.step(&InputFrame {
            attack_released: true,
            ..Default::default()
        });
        for _ in 0..24 {
            game.step(&InputFrame {
                right: true,
                ..Default::default()
            });
        }
        assert_eq!(game.player.data.get("swimming"), Some(&1.0));
        assert_eq!(
            game.world.entity(fish).unwrap().state,
            1,
            "ORDINARY_WATER_ENTRY_PRODUCES_ATTACK"
        );
        for _ in 0..12 {
            game.step(&InputFrame {
                left: true,
                ..Default::default()
            });
        }
        game.step(&InputFrame {
            right: true,
            ..Default::default()
        });
        assert_eq!(game.player.data.get("swimming"), Some(&0.0));
        assert_eq!(
            game.world.entity(fish).unwrap().state,
            2,
            "ORDINARY_WATER_EXIT_PRODUCES_PAUSE"
        );
        let mut firing = None;
        for _ in 0..90 {
            let actor = game.world.entity(fish).unwrap();
            assert_eq!(actor.health, 1);
            let mut prediction = SourceTape::at(&game);
            let speed = 6.0 + prediction.rand(3) as f64;
            let flight = ((actor.pos.x - (game.player.pos.x + 9.0)) / speed).ceil() as u32;
            if actor.state == 2
                && actor.timer == flight
                && flight > 0
                && game.player.vel.x == 0.0
                && game.player.facing == 1
            {
                firing = Some((speed, flight));
                break;
            }
            game.step(&InputFrame::default());
        }
        let (speed, flight) = firing.expect("PAUSE_ALARM_GEOMETRY_FIRE_WINDOW");
        game.step(&InputFrame {
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        let bullet = game
            .world
            .entities
            .iter()
            .find(|e| e.alive && e.object == "oBullet")
            .expect("REAL_PISTOL_BULLET");
        let bullet_id = bullet.id;
        assert_eq!(bullet.vel, Vec2::new(speed, 0.0));
        let mut lethal = None;
        for tick in 0..32 {
            let actor = game
                .world
                .entity(fish)
                .expect("LETHAL_BULLET_BEFORE_OWN_DEATH_STEP");
            if actor.health <= 0 {
                lethal = Some(actor.clone());
                break;
            }
            game.step(&InputFrame {
                attack_released: tick == 0,
                ..Default::default()
            });
        }
        let actor = lethal.expect("NATURAL_LETHAL_PISTOL_HIT");
        assert_eq!(actor.health, -3);
        assert_eq!(
            (actor.state, actor.timer),
            (2, 0),
            "ORDINARY_PAUSE_ZERO_ON_LETHAL_HIT"
        );
        assert!(game.world.entity(bullet_id).is_none());
        assert!(flight > 0);
        assert_eq!(count(&game, "oBlood"), 0, "BLOODLESS_SOURCE_DEATH_PREFIX");
        let mut expected = SourceTape::at(&game);
        for _ in 0..3 {
            expected.detritus_create();
        }
        expected.rand(2);
        game.step(&InputFrame::default());
        assert!(game.world.entity(fish).is_none());
        assert_eq!(count(&game, "oBone"), 3);
        assert_eq!(count(&game, "oFishBone"), 0);
        assert_eq!(game.data.get("deadfish"), Some(&1.0));
        assert_eq!(game.data.get("kills"), Some(&1.0));
        assert_eq!(game.player.health, 4.0);
        assert_eq!(
            game.rng.state_bits(),
            expected.state,
            "ORDINARY_PISTOL_DEATH_CONTINUES_PAUSE_RNG"
        );
    }
}
#[test]
fn legacy_controlled_dead_dry_keeps_final_fishbone() {
    dead_dry_case(Mode::Legacy)
}
#[test]
fn canonical_controlled_dead_dry_keeps_final_fishbone() {
    dead_dry_case(Mode::Canonical)
}
#[test]
fn legacy_controlled_dead_pause_zero_keeps_facing_rng() {
    dead_pause_case(Mode::Legacy)
}
#[test]
fn canonical_controlled_dead_pause_zero_keeps_facing_rng() {
    dead_pause_case(Mode::Canonical)
}
#[test]
fn legacy_controlled_healthy_dry_pause_runs_rng_before_dry_epilogue() {
    healthy_dry_pause_case(Mode::Legacy)
}
#[test]
fn canonical_controlled_healthy_dry_pause_runs_rng_before_dry_epilogue() {
    healthy_dry_pause_case(Mode::Canonical)
}
#[test]
fn legacy_healthy_wet_and_dead_wet_idle_controls() {
    positive_controls_case(Mode::Legacy)
}
#[test]
fn canonical_healthy_wet_and_dead_wet_idle_controls() {
    positive_controls_case(Mode::Canonical)
}
#[test]
fn legacy_ordinary_pistol_death_keeps_pause_zero_facing_rng() {
    ordinary_pistol_pause_zero_case(Mode::Legacy)
}
#[test]
fn canonical_ordinary_pistol_death_keeps_pause_zero_facing_rng() {
    ordinary_pistol_pause_zero_case(Mode::Canonical)
}

// Controlled query geometry: source false precision uses the cached integer
// bbox and all oWater descendants. DeadFish owns Step without Enemy inheritance.
fn lava_family_query_case(mode: Mode) {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for terrain in [false, true] {
            let mut game = mode.game(rules);
            game.world = World::new(30, 20, Area::Jungle, 5);
            game.player.pos = Vec2::new(240.0, 152.0);
            game.player.previous_pos = game.player.pos;
            if terrain {
                game.world.set_tile(6, 6, "oLava");
            } else {
                game.spawn("oLava", Vec2::new(96.0, 96.0));
            }
            let fish = game.spawn("oDeadFish", Vec2::new(96.0, 96.0));
            game.step(&InputFrame::default());
            let actor = game
                .world
                .entity(fish)
                .expect("LAVA_IS_WATER_FAMILY_NOT_DRY_FISHBONE");
            assert_eq!(actor.health, 1, "DEADFISH_DOES_NOT_INHERIT_ENEMY_LAVA_STEP");
            assert_eq!(count(&game, "oFishBone"), 0);
        }
    }
}
fn fractional_bbox_case(mode: Mode) {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for active in [true, false] {
            let mut game = mode.game(rules);
            game.world = World::new(30, 20, Area::Jungle, 5);
            game.player.pos = Vec2::new(240.0, 152.0);
            game.player.previous_pos = game.player.pos;
            let water_id = game.spawn("oWaterSwim", Vec2::new(96.0, 96.25));
            let water = game.world.entity_mut(water_id).unwrap();
            water.engine_active = active;
            let point = Vec2::new(96.0, 96.1);
            assert!(water.mask_contains(point, CollisionPrecision::BoundingBox));
            assert!(
                !water.bounds().contains(point),
                "OLD_CONTINUOUS_BODY_BOUNDS_MISS_FRACTIONAL_POINT"
            );
            let fish = game.spawn("oDeadFish", Vec2::new(92.0, 92.1));
            game.step(&InputFrame::default());
            assert!(
                game.world.entity(water_id).unwrap().engine_active,
                "LEVEL_REACTIVATES_NEARBY_WATER_BEFORE_DEADFISH_STEP"
            );
            assert!(
                game.world.entity(fish).is_some(),
                "ACTIVE_WATER_FAMILY_CACHED_BBOX_QUERY"
            );
            assert_eq!(count(&game, "oFishBone"), 0);
        }
    }
}
fn transparent_bbox_case(mode: Mode) {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = mode.game(rules);
        game.world = World::new(30, 20, Area::Jungle, 5);
        game.player.pos = Vec2::new(240.0, 152.0);
        game.player.previous_pos = game.player.pos;
        let water = game.spawn("oWaterSwim", Vec2::new(96.0, 96.0));
        let water = game.world.entity_mut(water).unwrap();
        // Explicit controlled sprite override creates a transparent top-left
        // pixel. Integer origin permits the independently supported precise
        // sampler to distinguish it from source false-precision bbox contact.
        water.set_sprite("sDarkFall");
        let point = Vec2::new(96.0, 96.0);
        assert!(water.mask_contains(point, CollisionPrecision::BoundingBox));
        assert!(
            !water.mask_contains(point, CollisionPrecision::Precise),
            "TRANSPARENT_POINT_SOURCE_FALSE_PRECISION_CONTROL"
        );
        let fish = game.spawn("oDeadFish", Vec2::new(92.0, 92.0));
        game.step(&InputFrame::default());
        assert!(
            game.world.entity(fish).is_some(),
            "FALSE_PRECISION_RETAINS_TRANSPARENT_BBOX_CONTACT"
        );
        assert_eq!(count(&game, "oFishBone"), 0);
    }
}

#[test]
fn legacy_lava_entity_and_terrain_are_water_family_controls() {
    lava_family_query_case(Mode::Legacy);
}
#[test]
fn canonical_lava_entity_and_terrain_are_water_family_controls() {
    lava_family_query_case(Mode::Canonical);
}
#[test]
fn legacy_fractional_water_bbox_and_level_reactivation_control() {
    fractional_bbox_case(Mode::Legacy);
}
#[test]
fn canonical_fractional_water_bbox_and_level_reactivation_control() {
    fractional_bbox_case(Mode::Canonical);
}

#[test]
fn legacy_transparent_water_bbox_false_precision_control() {
    transparent_bbox_case(Mode::Legacy);
}
#[test]
fn canonical_transparent_water_bbox_false_precision_control() {
    transparent_bbox_case(Mode::Canonical);
}
