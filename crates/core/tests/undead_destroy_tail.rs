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
    fn zombie_death(&mut self) {
        self.detritus_create(); // scrCreateBlood(1)
        for _ in 0..3 {
            self.detritus_create();
        } // synchronous Bone Creates
        self.rand(3);
        self.rand(4);
        self.rand(4); // Skull launch y, x-left, x-right
    }
}
fn count(game: &Game, object: &str) -> usize {
    game.world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == object)
        .count()
}
fn controlled_zombie(
    mode: Mode,
    rules: RulesPreset,
    state: i32,
    health: i32,
    floor: bool,
    near: bool,
) -> (Game, u64) {
    let mut game = mode.game(rules);
    game.world = World::new(30, 20, Area::Jungle, 5);
    game.world.name = "rLevel".into();
    if floor {
        for x in 0..30 {
            game.world.set_tile(x, 10, "oBlock");
        }
    }
    game.player.pos = Vec2::new(if near { 128.0 } else { 240.0 }, 152.0);
    game.player.previous_pos = game.player.pos;
    let id = game.spawn("oZombie", Vec2::new(96.0, 144.0));
    let actor = game.world.entity_mut(id).unwrap();
    actor.state = state;
    actor.health = health;
    actor.vel = Vec2::default();
    (game, id)
}
fn grounded_recovery_case(mode: Mode) {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, id) = controlled_zombie(mode, rules, 2, 0, true, false);
        let mut expected = SourceTape::at(&game);
        expected.zombie_death();
        expected.rand(61); // retired RECOVER reaches rand(40,100) after Destroy
        game.step(&InputFrame::default());
        assert!(game.world.entity(id).is_none());
        assert_eq!(
            (
                count(&game, "oBlood"),
                count(&game, "oBone"),
                count(&game, "oSkull")
            ),
            (1, 3, 1)
        );
        assert_eq!(game.data.get("zombies"), Some(&1.0));
        assert_eq!(game.data.get("kills"), Some(&1.0));
        assert_eq!(
            game.rng.state_bits(),
            expected.state,
            "POST_DESTROY_GROUNDED_RECOVERY_RNG"
        );
    }
}
fn idle_sound_case(mode: Mode) {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, id) = controlled_zombie(mode, rules, 0, 0, true, true);
        let mut expected = SourceTape::at(&game);
        expected.zombie_death();
        game.step(&InputFrame::default());
        assert!(game.world.entity(id).is_none());
        assert_eq!(
            game.events
                .iter()
                .filter(|event| matches!(event,GameEvent::Sound{name,..} if name=="sndZombie"))
                .count(),
            1,
            "POST_DESTROY_IDLE_BOUNCE_SOUND"
        );
        assert_eq!(game.rng.state_bits(), expected.state);
    }
}
fn recovery_controls_case(mode: Mode) {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut healthy, id) = controlled_zombie(mode, rules, 2, 1, true, false);
        let mut expected = SourceTape::at(&healthy);
        expected.rand(61);
        healthy.step(&InputFrame::default());
        let actor = healthy.world.entity(id).unwrap();
        assert_eq!(actor.state, 0);
        assert!((40..=100).contains(&actor.timer));
        assert_eq!(healthy.rng.state_bits(), expected.state);
        assert_eq!(
            (
                count(&healthy, "oBlood"),
                count(&healthy, "oBone"),
                count(&healthy, "oSkull")
            ),
            (0, 0, 0)
        );
        let (mut air, id) = controlled_zombie(mode, rules, 2, 0, false, false);
        let mut expected = SourceTape::at(&air);
        expected.zombie_death();
        air.step(&InputFrame::default());
        assert!(air.world.entity(id).is_none());
        assert_eq!(
            air.rng.state_bits(),
            expected.state,
            "AIRBORNE_RECOVERY_HAS_NO_FLOOR_DRAW"
        );
    }
}
fn ordinary_pistol_case(mode: Mode) {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (object, glyph, counter) in
            [("oZombie", 'z', "zombies"), ("oSkeleton", 'K', "skeletons")]
        {
            let mut level = level::Level::default();
            level.set_tile(4, 8, '@').unwrap();
            level.set_tile(5, 8, '-').unwrap();
            level.set_tile(14, 8, glyph).unwrap();
            for x in 6..=18 {
                level.set_tile(x, 7, '1').unwrap();
            }
            level.set_tile(18, 8, '1').unwrap();
            for x in 0..level::LEVEL_WIDTH {
                level.set_tile(x, 9, '1').unwrap();
            }
            let mut game = mode.game(rules);
            game.load_custom(&level, "undead-ordinary-pistol-death", "");
            let id = game
                .world
                .entities
                .iter()
                .find(|e| e.object == object)
                .unwrap()
                .id;
            assert_eq!(game.world.entity(id).unwrap().health, 1);
            for _ in 0..8 {
                game.step(&InputFrame::default());
            }
            for _ in 0..4 {
                game.step(&InputFrame {
                    right: true,
                    ..Default::default()
                });
            }
            game.step(&InputFrame {
                down: true,
                attack: true,
                attack_pressed: true,
                ..Default::default()
            });
            let pistol = game.player.held.expect("ACTUAL_AUTHORED_PISTOL_PICKUP");
            assert_eq!(game.world.entity(pistol).unwrap().object, "oPistol");
            for _ in 0..2 {
                game.step(&InputFrame::default());
            }
            let mut lethal = false;
            for tick in 0..240 {
                let fire = tick % 12 == 0;
                game.step(&InputFrame {
                    attack: fire,
                    attack_pressed: fire,
                    ..Default::default()
                });
                assert!(game.player.health > 0, "NATURAL_CONTROLLER_SURVIVES");
                let actor = game
                    .world
                    .entity(id)
                    .expect("WAIT_FOR_FATAL_BULLET_BEFORE_OWN_STEP");
                if actor.health <= 0 {
                    assert_eq!(actor.health, -3);
                    lethal = true;
                    break;
                }
            }
            assert!(lethal, "ORDINARY_PISTOL_REACHES_LETHAL_HP");
            game.step(&InputFrame::default());
            assert!(game.world.entity(id).is_none());
            assert_eq!((count(&game, "oBone"), count(&game, "oSkull")), (3, 1));
            assert_eq!(game.data.get(counter), Some(&1.0));
            assert_eq!(game.data.get("kills"), Some(&1.0));
        }
    }
}
#[test]
fn legacy_controlled_dead_grounded_recovery_keeps_post_destroy_rng() {
    grounded_recovery_case(Mode::Legacy)
}
#[test]
fn canonical_controlled_dead_grounded_recovery_keeps_post_destroy_rng() {
    grounded_recovery_case(Mode::Canonical)
}
#[test]
fn legacy_controlled_dead_idle_keeps_post_destroy_bounce_sound() {
    idle_sound_case(Mode::Legacy)
}
#[test]
fn canonical_controlled_dead_idle_keeps_post_destroy_bounce_sound() {
    idle_sound_case(Mode::Canonical)
}
#[test]
fn legacy_healthy_and_airborne_recovery_controls() {
    recovery_controls_case(Mode::Legacy)
}
#[test]
fn canonical_healthy_and_airborne_recovery_controls() {
    recovery_controls_case(Mode::Canonical)
}
#[test]
fn legacy_ordinary_pistol_reaches_both_undead_deaths() {
    ordinary_pistol_case(Mode::Legacy)
}
#[test]
fn canonical_ordinary_pistol_reaches_both_undead_deaths() {
    ordinary_pistol_case(Mode::Canonical)
}

// Authored hazard geometry; actor Create and every AI/damage transition remain
// ordinary. This is neither a pre-set death branch nor generated-level coverage.
fn ordinary_lava_landing_case(mode: Mode) {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = mode.game(rules);
        game.world = World::new(30, 20, Area::Jungle, 5);
        game.world.name = "rLevel".into();
        for x in 0..30 {
            game.world.set_tile(x, 10, "oBlock");
        }
        game.player.pos = Vec2::new(128.0, 152.0);
        game.player.previous_pos = game.player.pos;
        let zombie = game.spawn("oZombie", Vec2::new(96.0, 80.0));
        game.spawn("oLava", Vec2::new(96.0, 158.0));
        assert_eq!(
            (
                game.world.entity(zombie).unwrap().health,
                game.world.entity(zombie).unwrap().state
            ),
            (1, 0)
        );
        let mut saw_bounce = false;
        let mut saw_air_recover = false;
        let mut death = None;
        for tick in 0..60 {
            let before = game
                .world
                .entity(zombie)
                .expect("ORDINARY_LAVA_JOURNEY_LIVE_RECEIVER")
                .clone();
            assert_eq!(before.health, 1, "NO_HEALTH_WRITES_OR_PRIOR_DAMAGE");
            saw_bounce |= before.state == 1;
            saw_air_recover |= before.state == 2 && before.pos.y < 144.0;
            game.step(&InputFrame::default());
            if game.world.entity(zombie).is_none() {
                death = Some((tick, before));
                break;
            }
        }
        let (tick, before) = death.expect("STOCK_LAVA_LOWER_POINT_RETIRES_ZOMBIE");
        assert_eq!(tick, 26, "STOCK_GRAVITY_LANDING_THEN_LOWER_LAVA_PROBE");
        assert!(
            saw_bounce && saw_air_recover,
            "ORDINARY_AI_PRODUCES_RECOVERY_AND_LANDING"
        );
        assert_eq!((before.state, before.pos), (0, Vec2::new(96.0, 144.0)));
        assert!(
            (40..=100).contains(&before.timer),
            "LIVING_RECOVER_PRODUCED_IDLE_COUNTER"
        );
        assert_eq!(
            game.events
                .iter()
                .filter(
                    |event| matches!(event, GameEvent::Sound { name, .. } if name == "sndZombie")
                )
                .count(),
            1,
            "ORDINARY_LAVA_DEATH_CONTINUES_NEAR_IDLE_BOUNCE_SOUND"
        );
        assert_eq!(
            (
                count(&game, "oBlood"),
                count(&game, "oBone"),
                count(&game, "oSkull")
            ),
            (1, 3, 1)
        );
        assert_eq!(
            game.data.get("kills").copied().unwrap_or(0.0),
            0.0,
            "SOURCE_LAVA_DISALLOWS_KILL_CREDIT"
        );
        assert_eq!(game.player.health, 4, "PLAYER_STAYS_CLEAR_OF_AUTHORED_LAVA");
    }
}
#[test]
fn legacy_ordinary_lava_landing_keeps_post_destroy_idle_sound() {
    ordinary_lava_landing_case(Mode::Legacy);
}
#[test]
fn canonical_ordinary_lava_landing_keeps_post_destroy_idle_sound() {
    ordinary_lava_landing_case(Mode::Canonical);
}
