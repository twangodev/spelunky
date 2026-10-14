//! Live oYellowBall Step hatching, not its fully commented Collision event.
//! Seeds are filtered only by the independent literal launch/hatch premise;
//! all eligible inputs execute the same ordinary frames with assertions intact.
use spelunky_core::{Game, InputFrame, RngAlgorithm, RulesPreset, Vec2};

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
}

fn source_prefix(seed: u64, algorithm: RngAlgorithm) -> (LiteralRng, Vec2, u32, i8) {
    let mut rng = LiteralRng {
        state: seed,
        algorithm,
    };
    let vy = -4.0 - rng.unit() * 3.0;
    let mut vx = f64::from(rng.rand(2, 5));
    if rng.rand(1, 2) == 1 {
        vx = -vx;
    }
    let n = if rng.rand(1, 2) == 1 {
        rng.rand(1, 4)
    } else {
        rng.rand(1, 5)
    };
    let facing = if rng.rand(0, 1) == 0 { -1 } else { 1 };
    (rng, Vec2::new(vx, vy), n, facing)
}

fn room(seed: u64, rules: RulesPreset, algorithm: RngAlgorithm, wall: bool) -> (Game, u64) {
    let mut g = Game::shell_with_algorithm(seed, rules, algorithm).unwrap();
    g.player.pos = Vec2::new(24.0, 152.0);
    g.player.data.insert("active".into(), 0.0);
    // A one-pixel initial gap to x96: every selected positive source launch
    // reaches the live Step's raw rectangle, without an initial overlap.
    if wall {
        g.world.set_tile(6, 5, "oBlock");
    }
    let ball = g.spawn("oYellowBall", Vec2::new(87.0, 80.0));
    (g, ball)
}

#[test]
fn ordinary_yellow_ball_step_hatch_completes_frog_and_fire_frog_create_before_parent_retirement() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for algorithm in [RngAlgorithm::LegacySplitMix64V1, RngAlgorithm::Gm8Lcg32V1] {
            let mut seen = [false; 2];
            for seed in 0..128 {
                let (source, velocity, kind, facing) = source_prefix(seed, algorithm);
                // Initial constructor/branch eligibility only; never run a
                // world to select successful physics or suppress a failure.
                if velocity.x <= 0.0 || !matches!(kind, 4 | 5) {
                    continue;
                }
                seen[(kind - 4) as usize] = true;
                let (mut g, ball) = room(seed, rules, algorithm, true);
                let parent = g.world.entity(ball).unwrap();
                assert_eq!(parent.pos, Vec2::new(87.0, 80.0));
                assert_eq!(parent.vel, velocity);
                assert!(!g.world.rectangle_family_except(
                    Vec2::new(79.0, 72.0),
                    Vec2::new(95.0, 88.0),
                    "oSolid",
                    None,
                    spelunky_core::CollisionPrecision::BoundingBox
                ));
                assert!(g.world.rectangle_family_except(
                    Vec2::new(79.0 + velocity.x, 72.0 + velocity.y),
                    Vec2::new(95.0 + velocity.x, 88.0 + velocity.y),
                    "oSolid",
                    None,
                    spelunky_core::CollisionPrecision::BoundingBox
                ));
                let parent_serial = parent.creation_serial;
                g.step(&InputFrame::default());
                assert!(g.world.entity(ball).is_none());
                let children: Vec<_> = g
                    .world
                    .entities
                    .iter()
                    .filter(|e| e.alive && matches!(e.object.as_str(), "oFrog" | "oFireFrog"))
                    .collect();
                assert_eq!(children.len(), 1);
                let child = children[0];
                assert_eq!(child.object, if kind == 4 { "oFrog" } else { "oFireFrog" });
                assert_eq!(
                    child.sprite,
                    if kind == 4 {
                        "sFrogLeft"
                    } else {
                        "sFireFrogJumpL"
                    }
                );
                assert_eq!(child.pos, Vec2::new(79.0, 72.0));
                assert_eq!(child.previous_pos, child.pos);
                // Frog554 precedes YellowBall579 and waits for next Step.
                // FireFrog882 follows it: its inherited/own Step runs now,
                // with zero initial motion followed by literal gravity .2.
                assert_eq!(
                    child.vel,
                    if kind == 4 {
                        Vec2::default()
                    } else {
                        Vec2::new(0.0, 0.2)
                    }
                );
                assert_eq!(child.state, 0);
                // Age increments at frame bootstrap, before either birth;
                // it does not prove the higher-resource child's Step waited.
                assert_eq!(child.age, 0);
                assert_eq!(child.facing, facing);
                assert_eq!(child.value("jungle_context_created"), 1.0);
                assert_eq!(child.value("enemy_context_created"), 1.0);
                assert_eq!(child.value("swimming"), 0.0);
                assert_eq!(child.image_speed(), 0.4);
                assert!(parent_serial < child.creation_serial);
                assert_eq!(
                    g.rng.state_bits(),
                    source.state,
                    "three launch + two hatch + one synchronous child facing draw {seed}/{kind}"
                );
                let child_id = child.id;
                g.step(&InputFrame::default());
                assert!(g.world.entity(child_id).is_some());
                assert_eq!(
                    g.rng.state_bits(),
                    source.state,
                    "next bootstrap cannot defer/replay own Create"
                );
            }
            assert!(seen.into_iter().all(|kind| kind));
        }
    }
}

#[test]
fn same_literal_launch_without_solid_keeps_ball_and_consumes_only_its_own_three_birth_draws() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for algorithm in [RngAlgorithm::LegacySplitMix64V1, RngAlgorithm::Gm8Lcg32V1] {
            let mut seen = [false; 2];
            for seed in 0..128 {
                let (_, velocity, kind, _) = source_prefix(seed, algorithm);
                if velocity.x <= 0.0 || !matches!(kind, 4 | 5) {
                    continue;
                }
                seen[(kind - 4) as usize] = true;
                let (mut g, ball) = room(seed, rules, algorithm, false);
                let mut source = LiteralRng {
                    state: seed,
                    algorithm,
                };
                source.unit();
                source.rand(2, 5);
                source.rand(1, 2);
                g.step(&InputFrame::default());
                let parent = g.world.entity(ball).unwrap();
                assert_eq!(parent.pos, Vec2::new(87.0 + velocity.x, 80.0 + velocity.y));
                assert_eq!(parent.vel.x, velocity.x);
                assert_eq!(parent.vel.y, velocity.y + 0.15);
                assert!(
                    !g.world
                        .entities
                        .iter()
                        .any(|e| e.alive && matches!(e.object.as_str(), "oFrog" | "oFireFrog"))
                );
                assert_eq!(
                    g.rng.state_bits(),
                    source.state,
                    "unreached hatch body never consumes its draws"
                );
            }
            assert!(seen.into_iter().all(|kind| kind));
        }
    }
}
