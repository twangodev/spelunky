//! Literal oOlmecDebris Create and oOlmec Alarm1 producer order. The canonical
//! LCG/multiply-high oracle and explicit legacy compatibility oracle never
//! invoke the production Rng wrappers or constructor being checked.
use spelunky_core::{Area, Entity, Game, InputFrame, Rect, RngAlgorithm, RulesPreset, Vec2, World};

#[derive(Clone, Copy)]
struct LiteralRng {
    state: u64,
    algorithm: RngAlgorithm,
}
impl LiteralRng {
    fn raw(&mut self) -> u64 {
        match self.algorithm {
            RngAlgorithm::Gm8Lcg32V1 => {
                self.state =
                    u64::from((self.state as u32).wrapping_mul(0x08088405).wrapping_add(1));
                self.state
            }
            RngAlgorithm::LegacySplitMix64V1 => {
                self.state = self.state.wrapping_add(0x9e3779b97f4a7c15);
                let mut z = self.state;
                z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
                z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
                z ^ (z >> 31)
            }
        }
    }
    fn unit(&mut self) -> f64 {
        match self.algorithm {
            RngAlgorithm::Gm8Lcg32V1 => self.raw() as f64 * (1.0 / 4294967296.0),
            RngAlgorithm::LegacySplitMix64V1 => (self.raw() >> 11) as f64 / (1_u64 << 53) as f64,
        }
    }
    fn rand(&mut self, min: u32, max: u32) -> u32 {
        let raw = self.raw();
        let width = u64::from(max - min + 1);
        min + match self.algorithm {
            RngAlgorithm::Gm8Lcg32V1 => ((raw * width) >> 32) as u32,
            RngAlgorithm::LegacySplitMix64V1 => (raw % width) as u32,
        }
    }
    fn debris(&mut self) -> (Vec2, &'static str) {
        let velocity = Vec2::new(
            self.unit() * 4.0 - self.unit() * 4.0,
            -1.0 - self.unit() * 2.0,
        );
        let sprite = match self.rand(1, 3) {
            1 => "sOlmecDebris2",
            2 => "sOlmecDebris3",
            _ => "sOlmecDebris",
        };
        (velocity, sprite)
    }
}

fn fields(e: &Entity, sprite: &str) {
    assert_eq!(e.sprite, sprite);
    assert_eq!(e.image_speed(), 0.3);
    assert_eq!(e.text.get("type").map(String::as_str), Some(""));
    assert_eq!(e.data.get("blinkToggle"), Some(&0.0));
    assert_eq!(e.data.get("bounce"), Some(&1.0));
    assert_eq!(e.data.get("invincible"), Some(&1.0));
    assert_eq!(e.data.get("gravity"), Some(&0.6));
    assert_eq!(
        e.bounds(),
        Rect::new(e.pos.x - 4.0, e.pos.y - 4.0, 8.0, 8.0)
    );
    assert_eq!(e.value("started"), 1.0);
}

#[test]
fn direct_public_debris_create_finishes_literal_four_draws_and_all_birth_fields() {
    let mut seen = [false; 3];
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for algorithm in [RngAlgorithm::LegacySplitMix64V1, RngAlgorithm::Gm8Lcg32V1] {
            for seed in 0..32 {
                let mut g = Game::shell_with_algorithm(seed, rules, algorithm).unwrap();
                let mut source = LiteralRng {
                    state: g.rng.state_bits(),
                    algorithm,
                };
                let (velocity, sprite) = source.debris();
                seen[match sprite {
                    "sOlmecDebris2" => 0,
                    "sOlmecDebris3" => 1,
                    _ => 2,
                }] = true;
                let origin = Vec2::new(160.0, 96.0);
                let id = g.spawn("oOlmecDebris", origin);
                let e = g.world.entity(id).unwrap();
                fields(e, sprite);
                assert_eq!(e.pos, origin);
                assert_eq!(e.previous_pos, origin);
                assert_eq!(e.vel, velocity);
                assert_eq!(e.animation, 0.0);
                assert_eq!(e.age, 0);
                assert_eq!(g.world.entities.len(), 1);
                assert_eq!(g.rng.state_bits(), source.state, "four exact Create draws");
                assert_eq!(g.rng.clone().next_u64(), source.raw());
            }
        }
    }
    assert!(seen.into_iter().all(|variant| variant));
}

#[test]
fn direct_birth_caller_velocity_survives_real_first_step_without_create_replay() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for algorithm in [RngAlgorithm::LegacySplitMix64V1, RngAlgorithm::Gm8Lcg32V1] {
            let mut g = Game::shell_with_algorithm(42, rules, algorithm).unwrap();
            g.player.pos = Vec2::new(40.0, 152.0);
            g.player.data.insert("active".into(), 0.0);
            let origin = Vec2::new(160.0, 96.0);
            let mut source = LiteralRng {
                state: g.rng.state_bits(),
                algorithm,
            };
            let (_, sprite) = source.debris();
            let id = g.spawn("oOlmecDebris", origin);
            let launch = Vec2::new(3.0, -2.0);
            // Initial producer override after the actual completed Create.
            g.world.entity_mut(id).unwrap().vel = launch;
            for tick in 0..2 {
                g.step(&InputFrame::default());
                let e = g.world.entity(id).unwrap();
                fields(e, sprite);
                let expected_y = if tick == 0 {
                    origin.y - 2.0
                } else {
                    (origin.y - 2.0) + (-2.0 + 0.6)
                };
                assert_eq!(
                    e.pos,
                    Vec2::new(origin.x + 3.0 * f64::from(tick + 1), expected_y)
                );
                assert!((e.vel.y - (-2.0 + 0.6 * f64::from(tick + 1))).abs() < 1e-12);
                assert_eq!(e.vel.x, 3.0);
                assert_eq!(
                    g.rng.state_bits(),
                    source.state,
                    "no deferred/random Create replay"
                );
            }
        }
    }
}

#[test]
fn ordinary_olmec_alarm_dispatch_interleaves_positions_create_and_launch_draws() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for algorithm in [RngAlgorithm::LegacySplitMix64V1, RngAlgorithm::Gm8Lcg32V1] {
            let mut g = Game::shell_with_algorithm(42, rules, algorithm).unwrap();
            g.world = World::new(40, 30, Area::Mines, 1);
            g.player.pos = Vec2::new(48.0, 152.0);
            g.player.data.insert("active".into(), 0.0);
            for x in 0..40 {
                g.world.set_tile(x, 10, "oBlock");
            }
            let boss_origin = Vec2::new(96.0, 64.0);
            let boss = g.spawn("oOlmec", boss_origin);
            // Disclosed initial source Alarm1 boundary; all emissions below use
            // actual automatic alarm dispatch and the unchanged source producer.
            let e = g.world.entity_mut(boss).unwrap();
            e.state = -2;
            e.alarms[1] = 1;
            let mut source = LiteralRng {
                state: g.rng.state_bits(),
                algorithm,
            };
            let expected: Vec<_> = (0..6)
                .map(|_| {
                    let origin = Vec2::new(
                        boss_origin.x + 32.0 + f64::from(source.rand(0, 32)),
                        boss_origin.y + f64::from(source.rand(0, 32)),
                    );
                    let (_, sprite) = source.debris();
                    let launch =
                        Vec2::new(f64::from(source.rand(1, 4)), -f64::from(source.rand(1, 3)));
                    (origin, sprite, launch)
                })
                .collect();
            g.step(&InputFrame::default());
            let fragments: Vec<_> = g
                .world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == "oOlmecDebris")
                .collect();
            assert_eq!(fragments.len(), 6);
            for (e, (origin, sprite, launch)) in fragments.iter().zip(expected) {
                fields(e, sprite);
                assert_eq!(e.previous_pos, origin);
                assert_eq!(e.pos, Vec2::new(origin.x + launch.x, origin.y + launch.y));
                assert_eq!(e.vel.x, launch.x);
                assert!((e.vel.y - (launch.y + 0.6)).abs() < 1e-12);
                assert_eq!(e.age, 0); // born after the frame's age increment
            }
            assert!(
                fragments
                    .windows(2)
                    .all(|p| p[0].creation_serial < p[1].creation_serial)
            );
            assert_eq!(g.world.entity(boss).unwrap().value("intro_stage"), 1.0);
            assert_eq!(
                g.rng.state_bits(),
                source.state,
                "six exact eight-draw producer segments"
            );
            g.step(&InputFrame::default());
            assert_eq!(
                g.rng.state_bits(),
                source.state,
                "next bootstrap cannot rerun child Create"
            );
        }
    }
}

#[test]
fn real_authored_intro_keeps_literal_first_alarm_birth_prefix_and_caller_launches() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut g = Game::shell(42, rules);
        // Initial campaign destination, then the real rOlmec source construction.
        g.data.insert("next_level".into(), 16.0);
        g.next_level();
        let boss = g
            .world
            .entities
            .iter()
            .find(|e| e.alive && e.object == "oOlmec")
            .unwrap()
            .id;
        let mut reached = false;
        for _ in 0..260 {
            let before = g.world.entity(boss).unwrap();
            let firing = before.alarms[1] == 1;
            let origin = before.pos;
            let mut source = LiteralRng {
                state: g.rng.state_bits(),
                algorithm: RngAlgorithm::Gm8Lcg32V1,
            };
            let expected: Vec<_> = if firing {
                (0..6)
                    .map(|_| {
                        let born = Vec2::new(
                            origin.x + 32.0 + f64::from(source.rand(0, 32)),
                            origin.y + f64::from(source.rand(0, 32)),
                        );
                        let (_, sprite) = source.debris();
                        let launch =
                            Vec2::new(f64::from(source.rand(1, 4)), -f64::from(source.rand(1, 3)));
                        (born, sprite, launch)
                    })
                    .collect()
            } else {
                Vec::new()
            };
            g.step(&InputFrame::default());
            if !firing {
                continue;
            }
            let fragments: Vec<_> = g
                .world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == "oOlmecDebris" && e.age == 0)
                .collect();
            assert_eq!(fragments.len(), 6);
            for (e, (born, sprite, launch)) in fragments.iter().zip(expected) {
                fields(e, sprite);
                assert_eq!(e.previous_pos, born);
                assert_eq!(e.pos, Vec2::new(born.x + launch.x, born.y + launch.y));
                assert_eq!(e.vel.x, launch.x);
                assert!((e.vel.y - (launch.y + 0.6)).abs() < 1e-12);
            }
            assert!(
                fragments
                    .windows(2)
                    .all(|p| p[0].creation_serial < p[1].creation_serial)
            );
            // Other real room events may draw after this alarm; the exact full
            // producer stream is protected in the isolated ordinary-dispatch test.
            reached = true;
            break;
        }
        assert!(reached, "real unchanged rOlmec intro must dispatch Alarm1");
    }
}
