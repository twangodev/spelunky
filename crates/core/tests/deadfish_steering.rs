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
impl Mode {
    fn seeded_game(self, seed: u32, rules: RulesPreset) -> Game {
        match self {
            Self::Legacy => Game::legacy_shell(u64::from(seed), rules),
            Self::Canonical => Game::shell(seed, rules),
        }
    }
}
fn attack_room(mode: Mode, seed: u32, rules: RulesPreset, swim: bool) -> (Game, u64) {
    let mut game = mode.seeded_game(seed, rules);
    game.world = World::new(30, 20, Area::Jungle, 5);
    game.world.name = "rLevel".into();
    for x in 0..30 {
        game.world.set_tile(x, 10, "oBlock");
    }
    for y in 6..10 {
        for x in 5..if swim { 12 } else { 9 } {
            game.world.set_tile(x, y, "oWaterSwim");
        }
    }
    if !swim {
        // The bare Water parent has no sprite. This explicit authored sprite
        // supplies legal Water-family contact without being WaterSwim, so
        // Player's ordinary prelude leaves its swimming flag clear.
        let water = game.spawn("oWater", Vec2::new(136.0, 144.0));
        game.world.entity_mut(water).unwrap().set_sprite("sWater");
    }
    game.player.pos = Vec2::new(144.0, 152.0);
    game.player.previous_pos = game.player.pos;
    let fish = game.spawn("oDeadFish", Vec2::new(96.0, 120.0));
    (game, fish)
}
fn source_displacement(v: f64, time: u64) -> f64 {
    let fraction = v.abs().fract();
    let extra =
        fraction != 0.0 && time.is_multiple_of((1.0 / fraction).round_ties_even().max(1.0) as u64);
    (v.abs().floor() + f64::from(extra)) * v.signum()
}
// Natural stock Create→ordinary Player water-entry flag→IDLE→ATTACK. No fish
// state/HP/timer/direction or RNG edits occur after setup.
fn natural_attack_case(mode: Mode) {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for seed in [1, 2, 42, 129] {
            let (mut game, id) = attack_room(mode, seed, rules, true);
            assert_eq!(game.world.entity(id).unwrap().state, 0);
            game.step(&InputFrame::default());
            assert_eq!(game.player.data.get("swimming"), Some(&1.0));
            let before = game.world.entity(id).unwrap().clone();
            assert_eq!(before.state, 1, "ORDINARY_IDLE_ENTERS_ATTACK");
            assert_eq!(game.player.pos, Vec2::new(144.0, 152.0));
            for tick in 0..12 {
                let before = game.world.entity(id).unwrap().clone();
                let target = game.player.pos;
                let mut expected = SourceTape::at(&game);
                let jitter = expected.rand(2) as f64 - expected.rand(2) as f64;
                let direction = (before.pos.y - target.y)
                    .atan2(target.x - before.pos.x)
                    .to_degrees()
                    .rem_euclid(360.0)
                    + jitter;
                game.step(&InputFrame::default());
                assert_eq!(
                    game.rng.state_bits(),
                    expected.state,
                    "ATTACK_TWO_ORDERED_JITTER_DRAWS tick={tick}"
                );
                let actor = game.world.entity(id).unwrap();
                assert!(
                    (actor.value("dir") - direction).abs() < 1e-12,
                    "SOURCE_ORIGIN_POINT_DIRECTION_AND_JITTER tick={tick}"
                );
                let angle = direction.to_radians();
                let time = game.engine_time();
                assert_eq!(
                    actor.pos,
                    Vec2::new(
                        before.pos.x + source_displacement(angle.cos(), time),
                        before.pos.y + source_displacement(-angle.sin(), time)
                    ),
                    "SOURCE_ARGUMENT_MOVE_TO_DISPLACEMENT tick={tick}"
                );
                assert_eq!(
                    actor.vel,
                    Vec2::default(),
                    "MOVE_TO_ARGUMENTS_PRESERVE_SOURCE_XVEL_YVEL"
                );
                assert_eq!(
                    actor.facing,
                    if direction > 90.0 && direction < 270.0 {
                        -1
                    } else {
                        1
                    }
                );
            }
        }
    }
}
fn water_without_swimming_flag_case(mode: Mode) {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, id) = attack_room(mode, 42, rules, false);
        let expected = game.rng.state_bits();
        game.step(&InputFrame::default());
        assert_eq!(
            game.player.data.get("swimming").copied().unwrap_or(0.0),
            0.0,
            "ORDINARY_WATER_PARENT_DOES_NOT_SET_WATERSWIM_FLAG"
        );
        assert_eq!(
            game.world.entity(id).unwrap().state,
            0,
            "IDLE_GATE_READS_CHARACTER_SWIMMING_FLAG"
        );
        assert_eq!(game.rng.state_bits(), expected);
    }
}
fn controlled_dead_attack_case(mode: Mode) {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, id) = attack_room(mode, 42, rules, true);
        let fish = game.world.entity_mut(id).unwrap();
        fish.state = 1;
        fish.health = 0;
        let mut expected = SourceTape::at(&game);
        expected.rand(3); // Player's first water Bubble Create
        for _ in 0..3 {
            expected.detritus_create();
        }
        expected.rand(2);
        expected.rand(2);
        game.step(&InputFrame::default());
        assert!(game.world.entity(id).is_none());
        assert_eq!((count(&game, "oBone"), count(&game, "oFishBone")), (3, 0));
        assert_eq!(
            game.rng.state_bits(),
            expected.state,
            "RETIRED_ATTACK_JITTER_FOLLOWS_ALL_BONE_CREATES"
        );
    }
}
fn controlled_pause_direction_case(mode: Mode) {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut saw_right = false;
        let mut saw_left = false;
        for seed in 0..48 {
            let mut game = mode.seeded_game(seed, rules);
            game.world = World::new(30, 20, Area::Jungle, 5);
            game.player.pos = Vec2::new(240.0, 152.0);
            game.player.previous_pos = game.player.pos;
            for y in 4..9 {
                for x in 4..10 {
                    game.world.set_tile(x, y, "oWaterSwim");
                }
            }
            let id = game.spawn("oDeadFish", Vec2::new(96.0, 96.0));
            let fish = game.world.entity_mut(id).unwrap();
            fish.state = 2;
            fish.timer = 0;
            let mut expected = SourceTape::at(&game);
            let choice = expected.rand(2);
            let facing = if choice == 0 { 1 } else { -1 };
            game.step(&InputFrame::default());
            let actor = game.world.entity(id).unwrap();
            assert_eq!(actor.facing, facing, "PAUSE_RAND_ZERO_POINTS_RIGHT");
            assert_eq!(actor.value("dir"), choice as f64 * 180.0);
            assert_eq!(actor.state, 0);
            assert_eq!(game.rng.state_bits(), expected.state);
            saw_right |= choice == 0;
            saw_left |= choice == 1;
        }
        assert!(saw_right && saw_left, "BOTH_SOURCE_PAUSE_DIRECTION_CHOICES");
    }
}
fn controlled_character_reactivation_case(mode: Mode) {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = mode.game(rules);
        game.world = World::new(30, 20, Area::Jungle, 5);
        game.player.pos = Vec2::new(240.0, 152.0);
        game.player.previous_pos = game.player.pos;
        game.player.engine_active = false;
        for y in 4..9 {
            for x in 4..10 {
                game.world.set_tile(x, y, "oWaterSwim");
            }
        }
        let id = game.spawn("oDeadFish", Vec2::new(96.0, 96.0));
        game.world.entity_mut(id).unwrap().state = 1;
        let mut expected = SourceTape::at(&game);
        let timer = 20 + expected.rand(21) as u32;
        game.step(&InputFrame::default());
        assert!(
            game.player.engine_active,
            "LEVEL_REACTIVATES_CHARACTER_BEFORE_DEADFISH"
        );
        let actor = game.world.entity(id).unwrap();
        assert_eq!(
            (actor.state, actor.timer),
            (2, timer),
            "REACTIVATED_DRY_CHARACTER_TAKES_PRESENT_ATTACK_FALLBACK"
        );
        assert_eq!(game.rng.state_bits(), expected.state);
    }
}
#[test]
fn legacy_natural_attack_uses_origin_heading_and_two_jitter_draws() {
    natural_attack_case(Mode::Legacy)
}
#[test]
fn canonical_natural_attack_uses_origin_heading_and_two_jitter_draws() {
    natural_attack_case(Mode::Canonical)
}
#[test]
fn legacy_authored_water_parent_mask_without_swimming_flag_stays_idle() {
    water_without_swimming_flag_case(Mode::Legacy)
}
#[test]
fn canonical_authored_water_parent_mask_without_swimming_flag_stays_idle() {
    water_without_swimming_flag_case(Mode::Canonical)
}
#[test]
fn legacy_controlled_dead_attack_keeps_bone_then_jitter_order() {
    controlled_dead_attack_case(Mode::Legacy)
}
#[test]
fn canonical_controlled_dead_attack_keeps_bone_then_jitter_order() {
    controlled_dead_attack_case(Mode::Canonical)
}
#[test]
fn legacy_controlled_pause_zero_maps_source_directions() {
    controlled_pause_direction_case(Mode::Legacy)
}
#[test]
fn canonical_controlled_pause_zero_maps_source_directions() {
    controlled_pause_direction_case(Mode::Canonical)
}
#[test]
fn legacy_controlled_level_reactivates_character_before_attack_branch() {
    controlled_character_reactivation_case(Mode::Legacy)
}
#[test]
fn canonical_controlled_level_reactivates_character_before_attack_branch() {
    controlled_character_reactivation_case(Mode::Canonical)
}
fn positive_pause_controls_case(mode: Mode) {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for status in [1, 2] {
            let mut game = mode.game(rules);
            game.world = World::new(30, 20, Area::Jungle, 5);
            game.player.pos = Vec2::new(240.0, 152.0);
            game.player.previous_pos = game.player.pos;
            for y in 4..9 {
                for x in 4..10 {
                    game.world.set_tile(x, y, "oWaterSwim");
                }
            }
            let id = game.spawn("oDeadFish", Vec2::new(96.0, 96.0));
            let fish = game.world.entity_mut(id).unwrap();
            fish.state = status;
            fish.timer = 1;
            let mut expected = SourceTape::at(&game);
            let timer = if status == 1 {
                20 + expected.rand(21) as u32
            } else {
                0
            };
            game.step(&InputFrame::default());
            let actor = game.world.entity(id).unwrap();
            assert_eq!((actor.state, actor.timer), (2, timer));
            assert_eq!(game.rng.state_bits(), expected.state);
            assert_eq!((count(&game, "oBone"), count(&game, "oFishBone")), (0, 0));
        }
    }
}
#[test]
fn legacy_pause_countdown_and_present_dry_character_controls() {
    positive_pause_controls_case(Mode::Legacy)
}
#[test]
fn canonical_pause_countdown_and_present_dry_character_controls() {
    positive_pause_controls_case(Mode::Canonical)
}
