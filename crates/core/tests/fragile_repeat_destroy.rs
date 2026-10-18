//! Source-derived authored geometry and actors; only ordinary inputs after setup.
//! Explicit SourceClassicV2. No actor/velocity/equipment/latch/RNG midrun writes.
//! Native static proof permits the two explicit self-Destroy calls in one Step.
use spelunky_core::{
    Area, CollisionPrecision, Game, GameEvent, GenerationRevision, InputFrame, PlayerState, Rng,
    RngAlgorithm, RulesPreset, Vec2, World,
};

struct LiteralRng(u32);
impl LiteralRng {
    fn snapshot(rng: &Rng) -> Self {
        assert_eq!(rng.algorithm(), RngAlgorithm::Gm8Lcg32V1);
        Self(
            serde_json::to_value(rng).unwrap()["state"]
                .as_u64()
                .unwrap() as u32,
        )
    }
    fn raw(&mut self) -> u32 {
        self.0 = self.0.wrapping_mul(0x0808_8405).wrapping_add(1);
        self.0
    }
    fn range(&mut self, n: u32) -> u32 {
        ((u64::from(self.raw()) * u64::from(n)) >> 32) as u32
    }
    fn unit(&mut self) -> f64 {
        f64::from(self.raw()) / 4294967296.0
    }
}

#[derive(Debug)]
struct Birth {
    object: &'static str,
    pos: Vec2,
    vel: Vec2,
    bone_gravity: Option<f64>,
}

fn source_destroy(
    rng: &mut LiteralRng,
    object: &str,
    pos: Vec2,
    face: i8,
    wall: bool,
    births: &mut Vec<Birth>,
) {
    births.push(Birth {
        object: "oSmokePuff",
        pos,
        vel: Vec2::new(0.0, 0.1),
        bone_gravity: None,
    });
    let mut i = 0;
    // The original Skull for-condition redraws even for the failed test.
    while i < if object == "oJar" {
        3
    } else {
        1 + rng.range(2)
    } {
        let bone_gravity = if object == "oSkull" {
            // Synchronous Bone Create: three random() calls, then grav rand.
            let _create_vx = rng.unit() * 4.0 - rng.unit() * 4.0;
            let _create_vy = -1.0 - rng.unit() * 2.0;
            Some(f64::from(1 + rng.range(6)) * 0.1)
        } else {
            // RubbleSmall -> RubblePiece -> DrawnSprite Create is RNG-free.
            None
        };
        let vx = if wall {
            -f64::from(face) * f64::from(1 + rng.range(3))
        } else {
            f64::from(1 + rng.range(3)) - f64::from(1 + rng.range(3))
        };
        let vy = -f64::from(rng.range(4));
        births.push(Birth {
            object: if object == "oJar" {
                "oRubbleSmall"
            } else {
                "oBone"
            },
            pos: Vec2::new(pos.x - 2.0, pos.y - 2.0),
            vel: Vec2::new(vx, vy),
            bone_gravity,
        });
        i += 1;
    }
    if object == "oJar" {
        for (loot, width) in [
            ("oGoldChunk", 3),
            ("oGoldNugget", 6),
            ("oEmeraldBig", 12),
            ("oSapphireBig", 12),
            ("oRubyBig", 12),
            ("oSpider", 6),
            ("oSnake", 12),
        ] {
            if rng.range(width) == 0 {
                // Seed5's selected routes have no newborn enemies. This is a
                // mandatory fixture guard, not an omitted enemy-Collision oracle.
                assert!(!matches!(loot, "oSpider" | "oSnake"));
                births.push(Birth {
                    object: loot,
                    pos,
                    vel: Vec2::default(),
                    bone_gravity: None,
                });
                break;
            }
        }
    }
}

struct Fixture {
    game: Game,
    fragile: u64,
    attack_held: bool,
    face: i8,
    resources: (f64, f64, u32),
}
impl Fixture {
    fn step(&mut self, mut input: InputFrame) {
        assert!(!input.attack_pressed || (input.attack && !self.attack_held));
        input.attack_released = self.attack_held && !input.attack;
        self.attack_held = input.attack;
        self.game.step(&input);
        assert_eq!(
            self.game.player.health, 4.0,
            "FIXTURE Player must avoid lava"
        );
        assert_eq!(self.game.player.pos, Vec2::new(96.0, 168.0));
        assert_eq!(self.game.player.facing, self.face);
        assert_eq!(
            (
                self.game.player.bombs,
                self.game.player.ropes,
                self.game.player.money
            ),
            self.resources,
            "FIXTURE no resource changes"
        );
        assert_eq!(self.game.player.stun, 0);
        assert!(
            self.game
                .world
                .entities
                .iter()
                .all(|e| !spelunky_core::catalog::is_a(&e.object, "oEnemy")),
            "FIXTURE no creature impacts or loot-enemy interference"
        );
    }
}

fn ready(preset: RulesPreset, object: &str, face: i8, wall: bool, lava: bool) -> Fixture {
    let mut game = Game::shell(5, preset);
    game.generation_revision = GenerationRevision::SourceClassicV2;
    game.world = World::new(24, 20, Area::Mines, 1);
    game.world.name = "rLevel".into();
    for x in 0..24 {
        game.world.set_tile(x, 11, "oHardBlock");
    }
    game.player.pos = Vec2::new(96.0, 168.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.player.facing = face;
    game.world.entrance = game.player.pos;
    let mitt = game.spawn("oMitt", game.player.pos);
    let fragile = game.spawn(
        object,
        Vec2::new(96.0, if object == "oJar" { 170.0 } else { 172.0 }),
    );
    if wall {
        game.spawn(
            "oHardBlock",
            Vec2::new(if face > 0 { 118.0 } else { 58.0 }, 160.0),
        );
    }
    if lava {
        // Nonoverlapping pool + wall: right102..117/118..133;
        // left74..89/58..73. Original Lava Create does not snap its origin.
        game.spawn(
            "oLava",
            Vec2::new(if face > 0 { 102.0 } else { 74.0 }, 160.0),
        );
    }
    let initial_rng = serde_json::to_value(&game.rng).unwrap();
    let resources = (game.player.bombs, game.player.ropes, game.player.money);
    let mut f = Fixture {
        game,
        fragile,
        attack_held: false,
        face,
        resources,
    };
    // The ordinary inherited Mitt Step clears its source cost without a keeper.
    f.step(InputFrame::default());
    assert_eq!(f.game.world.entity(mitt).unwrap().value("cost"), 0.0);
    f.step(InputFrame {
        down: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert!(f.game.player.has("Mitt"), "FIXTURE actual equipment pickup");
    assert!(f.game.player.held.is_none());
    assert!(f.game.world.entity(mitt).is_none());
    f.step(InputFrame::default());
    f.step(InputFrame {
        down: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert_eq!(
        f.game.player.held,
        Some(fragile),
        "FIXTURE actual fragile pickup"
    );
    f.step(InputFrame::default());
    assert_eq!(f.game.player.state, PlayerState::Standing);
    let e = f.game.world.entity(fragile).unwrap();
    assert!(e.held);
    assert_eq!(e.pos, Vec2::new(96.0 + f64::from(face) * 4.0, 168.0));
    assert_eq!(e.vel, Vec2::default());
    assert_eq!(
        serde_json::to_value(&f.game.rng).unwrap(),
        initial_rng,
        "FIXTURE quiet prefix"
    );
    assert!(
        f.game
            .world
            .entities
            .iter()
            .all(|e| !matches!(e.object.as_str(), "oBone" | "oRubbleSmall" | "oSmokePuff"))
    );
    f
}

fn run(preset: RulesPreset, object: &str, face: i8, wall: bool, lava: bool) {
    let mut f = ready(preset, object, face, wall, lava);
    let last_id = f.game.world.entities.iter().map(|e| e.id).max().unwrap();
    let mut rng = LiteralRng::snapshot(&f.game.rng);
    f.step(InputFrame {
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert!(
        f.game.player.held.is_none(),
        "ordinary throw releases the item"
    );
    assert!(f.game.player.pickup_item_type.is_empty());
    // moveTo quantizes -.4 to -1 every round(1/.4)=2 source-clock frames.
    let y = 168.0
        - if f.game.engine_time().is_multiple_of(2) {
            1.0
        } else {
            0.0
        };
    let x = if wall {
        if face > 0 { 113.0 } else { 79.0 }
    } else {
        96.0 + f64::from(face) * 18.0
    };
    let pos = Vec2::new(x, y + if lava { 0.05 } else { 0.0 });
    // Re-evaluate the literal target predicates against the untouched initial
    // geometry, before checking callback counts. Full-height side probes
    // include center y; HardBlock has a rectangular precise mask.
    let contact_x = if face > 0 { x + 5.0 } else { x - 6.0 };
    assert_eq!(
        f.game.world.solid_at(contact_x, y),
        wall,
        "FIXTURE source side contact at pre-correction center"
    );
    let deep_lava = f.game.world.entities.iter().any(|e| {
        e.alive
            && e.object == "oLava"
            && e.mask_contains(
                Vec2::new(pos.x, pos.y - 5.0),
                CollisionPrecision::BoundingBox,
            )
    });
    assert_eq!(
        deep_lava, lava,
        "FIXTURE original precision0 deep-Lava point"
    );
    let callbacks = if wall && lava {
        2
    } else if wall || lava {
        1
    } else {
        0
    };
    let actual: Vec<_> = f
        .game
        .world
        .entities
        .iter()
        .filter(|e| e.id > last_id)
        .collect();
    if callbacks > 0 {
        assert!(
            f.game.world.entities.iter().all(|e| e.id != f.fragile),
            "source retirement remains retired"
        );
        let first = actual
            .first()
            .expect("FIXTURE first explicit Destroy must run");
        assert_eq!(first.object, "oSmokePuff");
        assert_eq!(
            first.pos, pos,
            "FIXTURE motion/lava location before callback-count oracle"
        );
        let piece = actual.get(1).expect("FIXTURE first Destroy debris");
        assert_eq!(piece.pos, Vec2::new(pos.x - 2.0, pos.y - 2.0));
        if wall {
            assert!(
                piece.vel.x * f64::from(face) < 0.0,
                "FIXTURE physical contact latch survives lava velocity zeroing"
            );
        }
    } else {
        let e = f
            .game
            .world
            .entity(f.fragile)
            .expect("clear throw remains alive");
        assert!(!e.held);
        assert_eq!(e.pos, pos);
        assert_eq!(e.vel, Vec2::new(f64::from(face) * 14.0, -0.4 + 0.1));
        assert_eq!(e.value("gravity"), 0.1);
    }
    let sounds: Vec<_> = f
        .game
        .events
        .iter()
        .filter_map(|e| {
            if let GameEvent::Sound { name, pos } = e {
                Some((name.as_str(), *pos))
            } else {
                None
            }
        })
        .collect();
    let mut expected_sounds = vec![("sndThrow", Vec2::new(96.0, 168.0))];
    for _ in 0..callbacks {
        expected_sounds.push(("sndBreak", pos));
    }
    assert_eq!(
        sounds, expected_sounds,
        "each explicit source self-Destroy dispatches its callback"
    );
    let mut expected = Vec::new();
    for _ in 0..callbacks {
        source_destroy(&mut rng, object, pos, face, wall, &mut expected);
    }
    assert_eq!(
        actual.len(),
        expected.len(),
        "exact complete ordered birth list"
    );
    for (e, b) in actual.iter().zip(&expected) {
        assert_eq!(e.object, b.object);
        assert_eq!(e.pos, b.pos);
        assert_eq!(e.vel, b.vel);
        assert!(e.alive);
        if let Some(gravity) = b.bone_gravity {
            assert_eq!(e.value("gravity"), gravity);
        }
    }
    assert!(actual.windows(2).all(|w| w[0].id < w[1].id));
    assert_eq!(
        LiteralRng::snapshot(&f.game.rng).0,
        rng.0,
        "literal callback/Create RNG order"
    );
}

fn cases(wall: bool, lava: bool) {
    for preset in [RulesPreset::Classic, RulesPreset::Assisted] {
        for object in ["oJar", "oSkull"] {
            for face in [-1, 1] {
                run(preset, object, face, wall, lava);
            }
        }
    }
}
#[test]
fn ordinary_mitt_jar_throw_wall_and_deep_lava_dispatches_both_explicit_destroys() {
    for preset in [RulesPreset::Classic, RulesPreset::Assisted] {
        for face in [-1, 1] {
            run(preset, "oJar", face, true, true);
        }
    }
}
#[test]
fn ordinary_mitt_skull_throw_wall_and_deep_lava_dispatches_both_explicit_destroys() {
    for preset in [RulesPreset::Classic, RulesPreset::Assisted] {
        for face in [-1, 1] {
            run(preset, "oSkull", face, true, true);
        }
    }
}
#[test]
fn ordinary_mitt_throw_physical_only_dispatches_one_destroy() {
    cases(true, false);
}
#[test]
fn ordinary_mitt_throw_deep_lava_without_physical_latch_dispatches_one_destroy() {
    cases(false, true);
}
#[test]
fn ordinary_mitt_throw_clear_path_retains_live_fragile_without_destroy_rng() {
    cases(false, false);
}
