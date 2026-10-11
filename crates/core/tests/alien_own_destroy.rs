//! Alien own-Step death prefix and post-Destroy continuation.
//! Crush/cap/WALK fixtures are controlled initial contacts, not generated routes.
//! The Pistol journey uses authored terrain and ordinary public inputs only.
use spelunky_core::*;

#[derive(Clone)]
struct Tape {
    algorithm: RngAlgorithm,
    state: u64,
}
impl Tape {
    fn at(game: &Game) -> Self {
        Self {
            algorithm: game.rng.algorithm(),
            state: game.rng.state_bits(),
        }
    }
    fn next(&mut self) -> u64 {
        match self.algorithm {
            RngAlgorithm::Gm8Lcg32V1 => {
                self.state = u64::from(
                    (self.state as u32)
                        .wrapping_mul(0x0808_8405)
                        .wrapping_add(1),
                );
                self.state
            }
            RngAlgorithm::LegacySplitMix64V1 => {
                self.state = self.state.wrapping_add(0x9e3779b97f4a7c15);
                let mut v = self.state;
                v = (v ^ (v >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
                v = (v ^ (v >> 27)).wrapping_mul(0x94d049bb133111eb);
                v ^ (v >> 31)
            }
        }
    }
    fn rand(&mut self, width: u32) -> u32 {
        let value = self.next();
        match self.algorithm {
            RngAlgorithm::Gm8Lcg32V1 => ((value * u64::from(width)) >> 32) as u32,
            RngAlgorithm::LegacySplitMix64V1 => (value % u64::from(width)) as u32,
        }
    }
    fn random(&mut self) -> f64 {
        let value = self.next();
        match self.algorithm {
            RngAlgorithm::Gm8Lcg32V1 => value as f64 / 4_294_967_296.0,
            RngAlgorithm::LegacySplitMix64V1 => (value >> 11) as f64 / (1_u64 << 53) as f64,
        }
    }
    fn blood(&mut self) -> (Vec2, f64) {
        let vel = Vec2::new(
            self.random() * 4.0 - self.random() * 4.0,
            -1.0 - self.random() * 2.0,
        );
        let gravity = f64::from(1 + self.rand(6)) * 0.1;
        (vel, gravity)
    }
}
fn shell(legacy: bool, rules: RulesPreset) -> Game {
    if legacy {
        Game::legacy_shell(42, rules)
    } else {
        Game::shell(42, rules)
    }
}
fn room(legacy: bool, rules: RulesPreset, crush: bool, cap: usize, walk: bool) -> (Game, u64) {
    let mut game = shell(legacy, rules);
    game.world = World::new(30, 20, Area::Ice, 9);
    game.world.name = "rLevel".into();
    for x in 0..30 {
        game.world.set_tile(x, 10, "oBlock");
    }
    game.player.pos = Vec2::new(240.0, 152.0);
    game.player.previous_pos = game.player.pos;
    if crush {
        game.world.set_tile(6, 9, "oBlock");
    }
    for _ in 0..cap {
        game.spawn("oBlood", Vec2::new(240.0, 64.0));
    }
    let id = game.spawn("oAlien", Vec2::new(96.0, 144.0));
    let actor = game.world.entity_mut(id).unwrap();
    assert_eq!((actor.health, actor.state), (1, 0));
    if walk {
        actor.state = 1;
    } // explicit controlled source-reachable branch
    (game, id)
}
fn blood(game: &Game) -> Vec<&Entity> {
    let mut result: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oBlood")
        .collect();
    result.sort_by_key(|e| e.creation_serial);
    result
}
fn crush_blood_prefix_case(legacy: bool) {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, id) = room(legacy, rules, true, 0, false);
        let mut tape = Tape::at(&game);
        let launch: Vec<_> = (0..3).map(|_| tape.blood()).collect();
        tape.rand(2); // original independent IDLE tail after instance_destroy
        game.step(&InputFrame::default());
        assert!(game.world.entity(id).is_none());
        assert_eq!(game.data.get("aliens"), Some(&1.0));
        assert_eq!(game.data.get("kills"), Some(&1.0));
        assert_eq!(game.lifetime_stats.enemy_kills[15], 1);
        let births = blood(&game);
        assert_eq!(births.len(), 3, "ALIEN_DEATH_BLOOD_PREFIX");
        for (particle, (velocity, gravity)) in births.into_iter().zip(launch) {
            assert_eq!(particle.pos, Vec2::new(104.0, 152.0));
            assert_eq!(particle.vel, velocity);
            assert_eq!(particle.value("gravity"), gravity);
            assert_eq!(
                particle.value("life"),
                60.0,
                "BLOOD_RESOURCE_ALREADY_PASSED"
            );
        }
        assert_eq!(
            Tape::at(&game).state,
            tape.state,
            "BLOOD_THEN_POST_DESTROY_IDLE_DRAW"
        );
        game.step(&InputFrame::default());
        assert_eq!(game.data.get("kills"), Some(&1.0));
        assert_eq!(game.data.get("aliens"), Some(&1.0));
        assert_eq!(game.player.health, 4);
    }
}
fn capped_tail_case(legacy: bool, walk: bool) {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, id) = room(legacy, rules, true, 16, walk);
        let mut tape = Tape::at(&game);
        if walk {
            if tape.rand(100) == 0 {
                tape.rand(31);
            }
        } else {
            tape.rand(2);
        }
        game.step(&InputFrame::default());
        assert!(game.world.entity(id).is_none());
        assert_eq!(blood(&game).len(), 16, "FULL_CAP_CREATES_NO_DEATH_BLOOD");
        assert_eq!(
            Tape::at(&game).state,
            tape.state,
            "CAPPED_DEATH_STILL_RUNS_AI_DRAW"
        );
        assert_eq!(game.data.get("kills"), Some(&1.0));
        assert_eq!(game.player.health, 4);
    }
}
fn live_control_case(legacy: bool) {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, id) = room(legacy, rules, false, 0, false);
        let mut tape = Tape::at(&game);
        let facing = if tape.rand(2) == 0 { -1 } else { 1 };
        game.step(&InputFrame::default());
        let actor = game.world.entity(id).unwrap();
        assert_eq!((actor.health, actor.state, actor.facing), (1, 1, facing));
        assert_eq!(Tape::at(&game).state, tape.state);
        assert!(blood(&game).is_empty());
        assert_eq!(game.data.get("kills"), None);
        assert_eq!(game.player.health, 4);
    }
}
fn ordinary_pistol_case(legacy: bool) {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut level = level::Level::default();
        level.set_tile(4, 8, '@').unwrap();
        level.set_tile(5, 8, '-').unwrap();
        level.set_tile(14, 8, 'a').unwrap();
        for x in 6..=18 {
            level.set_tile(x, 7, '1').unwrap();
        }
        level.set_tile(18, 8, '1').unwrap();
        for x in 0..level::LEVEL_WIDTH {
            level.set_tile(x, 9, '1').unwrap();
        }
        let mut game = shell(legacy, rules);
        game.load_custom(&level, "alien-ordinary-pistol", "");
        let id = game
            .world
            .entities
            .iter()
            .find(|e| e.object == "oAlien")
            .unwrap()
            .id;
        let geometry: Vec<_> = game.world.tiles.iter().map(|t| t.object.clone()).collect();
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
        let gun = game.player.held.expect("ORDINARY_AUTHORED_PISTOL_PICKUP");
        assert_eq!(game.world.entity(gun).unwrap().object, "oPistol");
        for _ in 0..2 {
            game.step(&InputFrame::default());
        }
        assert_eq!(game.player.facing, 1);
        let mut lethal = false;
        for tick in 0..240 {
            let fire = tick % 12 == 0;
            game.step(&InputFrame {
                attack: fire,
                attack_pressed: fire,
                ..Default::default()
            });
            assert!(game.player.health > 0, "ORDINARY_CONTROLLER_SURVIVES");
            if let Some(actor) = game.world.entity(id) {
                if actor.health <= 0 {
                    lethal = true;
                    break;
                }
            } else {
                panic!("WAIT_FOR_FATAL_BULLET_BEFORE_NEXT_ALIEN_STEP");
            }
        }
        assert!(lethal, "ORDINARY_BULLET_REACHES_FATAL_HP");
        let actor = game.world.entity(id).unwrap();
        assert!(!actor.held && actor.value("burning") == 0.0);
        assert!(!game.world.solid_at(actor.pos.x + 8.0, actor.pos.y + 8.0));
        let existing: Vec<_> = blood(&game).iter().map(|e| e.id).collect();
        game.step(&InputFrame::default());
        assert!(game.world.entity(id).is_none());
        assert_eq!(
            blood(&game)
                .iter()
                .filter(|e| !existing.contains(&e.id))
                .count(),
            3,
            "ORDINARY_FATAL_DAMAGE_REACHES_OWN_DEATH_BLOOD"
        );
        assert_eq!(game.data.get("kills"), Some(&1.0));
        assert_eq!(game.data.get("aliens"), Some(&1.0));
        assert_eq!(
            game.world
                .tiles
                .iter()
                .map(|t| t.object.clone())
                .collect::<Vec<_>>(),
            geometry
        );
    }
}
#[test]
fn canonical_crush_blood_prefix() {
    crush_blood_prefix_case(false);
}
#[test]
fn legacy_crush_blood_prefix() {
    crush_blood_prefix_case(true);
}
#[test]
fn canonical_capped_idle_tail() {
    capped_tail_case(false, false);
}
#[test]
fn legacy_capped_idle_tail() {
    capped_tail_case(true, false);
}
#[test]
fn canonical_capped_walk_tail() {
    capped_tail_case(false, true);
}
#[test]
fn legacy_capped_walk_tail() {
    capped_tail_case(true, true);
}
#[test]
fn canonical_live_control() {
    live_control_case(false);
}
#[test]
fn legacy_live_control() {
    live_control_case(true);
}
#[test]
fn canonical_ordinary_pistol_death() {
    ordinary_pistol_case(false);
}
#[test]
fn legacy_ordinary_pistol_death() {
    ordinary_pistol_case(true);
}
