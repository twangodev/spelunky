//! Fish own-Step post-Destroy regressions. Natural authored-room input routes
//! and explicitly configured cap controls; no original-game runtime claim.
use spelunky_core::*;

#[derive(Clone)]
struct LiteralRng(u64, bool);

impl LiteralRng {
    fn at(game: &Game) -> Self {
        let wire = serde_json::to_value(&game.rng).unwrap();
        let state = wire["state"].as_u64().unwrap();
        let canonical = match wire.get("algorithm").and_then(|value| value.as_str()) {
            None => false,
            Some("gm8_lcg32_v1") => true,
            other => panic!("unsupported literal RNG identity: {other:?}"),
        };
        assert!(!canonical || state <= u64::from(u32::MAX));
        Self(state, canonical)
    }

    fn next(&mut self) -> u64 {
        if self.1 {
            self.0 = u64::from((self.0 as u32).wrapping_mul(0x0808_8405).wrapping_add(1));
            return self.0;
        }
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut value = self.0;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
        value ^ (value >> 31)
    }

    fn range(&mut self, upper: u64) -> u64 {
        assert!((1..=u64::from(u32::MAX)).contains(&upper));
        let raw = self.next();
        if self.1 {
            (raw * upper) >> 32
        } else {
            raw % upper
        }
    }

    fn random(&mut self) -> f64 {
        let raw = self.next();
        if self.1 {
            raw as f64 / 4_294_967_296.0
        } else {
            (raw >> 11) as f64 / (1_u64 << 53) as f64
        }
    }

    fn blood_create(&mut self) -> (Vec2, f64) {
        // oBlood own Create, after fixed DrawnSprite/Detritus initialization.
        let velocity = Vec2::new(
            self.random() * 4.0 - self.random() * 4.0,
            -1.0 - self.random() * 2.0,
        );
        (velocity, (1 + self.range(6)) as f64 * 0.1)
    }
}

fn room(rules: RulesPreset, canonical: bool, shallow: bool) -> Game {
    let mut level = level::Level::default();
    level.set_tile(4, 8, '@').unwrap();
    for x in 0..if shallow { 40 } else { 8 } {
        level.set_tile(x, 9, '1').unwrap();
    }
    for x in 8..=20 {
        level.set_tile(x, 8, 'w').unwrap();
    }
    let mut game = if canonical {
        Game::shell(42, rules)
    } else {
        Game::legacy_shell(42, rules)
    };
    game.load_custom(&level, "fish-own-destroy-tail", "");
    game
}
fn prior_ids(game: &Game) -> std::collections::BTreeSet<u64> {
    game.world.entities.iter().map(|e| e.id).collect()
}
fn born<'a>(
    game: &'a Game,
    prior: &std::collections::BTreeSet<u64>,
    object: &str,
) -> Vec<&'a Entity> {
    let mut entities: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == object && !prior.contains(&e.id))
        .collect();
    entities.sort_by_key(|e| e.id);
    entities
}
fn assert_rng(game: &Game, expected: &LiteralRng, label: &str) {
    assert_eq!(LiteralRng::at(game).0, expected.0, "{label}");
}
fn assert_credit(game: &Game, category: &str) {
    assert_eq!(game.data.get(category), Some(&1.0));
    assert_eq!(game.data.get("kills"), Some(&1.0));
}
fn assert_blood(
    game: &Game,
    prior: &std::collections::BTreeSet<u64>,
    origin: Vec2,
    expected: &[(Vec2, f64)],
) {
    let blood = born(game, prior, "oBlood");
    assert_eq!(blood.len(), expected.len(), "EXACT_DEATH_BLOOD_BIRTHS");
    for (drop, (velocity, gravity)) in blood.iter().zip(expected) {
        assert_eq!(drop.pos, origin);
        assert_eq!(drop.vel, *velocity);
        assert_eq!(drop.value("gravity"), *gravity);
        assert_eq!((drop.alarms[0], drop.alarms[1], drop.alarms[2]), (1, 1, 5));
    }
}

#[test]
fn ordinary_whip_kill_keeps_piranha_attack_jitter_after_death() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for canonical in [false, true] {
            let mut game = room(rules, canonical, true);
            let id = game.spawn("oPiranha", Vec2::new(144.0, 144.0));
            assert_eq!(
                (
                    game.world.entity(id).unwrap().health,
                    game.world.entity(id).unwrap().state
                ),
                (1, 0)
            );
            let mut fired = false;
            for _ in 0..96 {
                let fish = game.world.entity(id).expect("LIVE_BEFORE_WHIP");
                assert_eq!(fish.health, 1);
                // Select before attack from naturally reached live ATTACK,
                // player swimming and front-whip geometry, never death output.
                let fire = fish.state == 1
                    && game.player.data.get("swimming").copied().unwrap_or(0.0) != 0.0
                    && fish.pos.x + 4.0 > game.player.pos.x
                    && fish.pos.x + 4.0 - game.player.pos.x < 32.0;
                game.step(&InputFrame {
                    right: true,
                    attack: fire,
                    attack_pressed: fire,
                    ..Default::default()
                });
                if fire {
                    fired = true;
                    break;
                }
            }
            assert!(fired, "NATURAL_SWIMMING_ATTACK_WHIP_WINDOW");
            let mut lethal = false;
            let mut saw_whip = false;
            for _ in 0..16 {
                saw_whip |= game
                    .world
                    .entities
                    .iter()
                    .any(|e| e.alive && catalog::is_a(&e.object, "oWhip"));
                let fish = game.world.entity(id).expect("WAIT_FOR_REAL_WHIP_HIT");
                if fish.health <= 0 {
                    lethal = true;
                    break;
                }
                game.step(&InputFrame::default());
            }
            assert!(saw_whip && lethal, "NATURAL_WHIP_DEATH_PRODUCER");
            let fish = game.world.entity(id).unwrap().clone();
            assert_eq!(fish.state, 1, "LIVE_ATTACK_STATE_SURVIVES_WHIP_HIT");
            assert_eq!(fish.vel, Vec2::default());
            assert!(fish.value("bubble_timer") > 0.0);
            assert!(game.player.data.get("swimming").copied().unwrap_or(0.0) != 0.0);
            assert_eq!(game.player.health, 4.0);
            assert!(!game.player.is_stunned());
            assert!(Vec2::new(fish.pos.x + 4.0, fish.pos.y + 4.0).distance(game.player.pos) < 90.0);
            let prior = prior_ids(&game);
            let mut expected = LiteralRng::at(&game);
            let blood = [
                expected.blood_create(),
                expected.blood_create(),
                expected.blood_create(),
            ];
            expected.range(2);
            expected.range(2); // source point_direction jitter
            game.step(&InputFrame::default());
            assert!(game.world.entity(id).is_none());
            assert_credit(&game, "piranhas");
            assert_blood(
                &game,
                &prior,
                Vec2::new(fish.pos.x + 4.0, fish.pos.y + 4.0),
                &blood,
            );
            assert!(born(&game, &prior, "oBubble").is_empty());
            assert!(born(&game, &prior, "oFishBone").is_empty());
            assert_eq!(game.player.health, 4.0);
            assert_rng(&game, &expected, "POST_DESTROY_PIRANHA_ATTACK_JITTER");
            game.step(&InputFrame::default());
            assert_credit(&game, "piranhas");
        }
    }
}

fn jaws_death_oracle(
    rng: &mut LiteralRng,
    origin: Vec2,
    blood_count: usize,
) -> (Vec2, Vec<(Vec2, f64)>, Vec<(Vec2, Vec2, f64)>, Vec<Vec2>) {
    // Jaws draws the shared blood origin even when script Blood is capped.
    let blood_pos = Vec2::new(
        origin.x + 22.0 + rng.range(5) as f64,
        origin.y + 14.0 + rng.range(5) as f64,
    );
    let blood = (0..blood_count).map(|_| rng.blood_create()).collect();
    // All four synchronous Bone Creates precede ANY crate velocity roll.
    let bones = (0..4)
        .map(|_| {
            let pos = Vec2::new(
                origin.x + 22.0 + rng.range(5) as f64,
                origin.y + 14.0 + rng.range(7) as f64,
            );
            let (vel, gravity) = rng.blood_create();
            (pos, vel, gravity)
        })
        .collect();
    let crates = (0..4)
        .map(|_| {
            Vec2::new(
                rng.range(4) as f64 - rng.range(4) as f64,
                -1.0 - rng.range(2) as f64,
            )
        })
        .collect();
    (blood_pos, blood, bones, crates)
}

#[test]
fn ordinary_drying_jaws_death_keeps_due_bubble_after_exact_loot_once() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for canonical in [false, true] {
            let mut game = room(rules, canonical, false);
            let id = game.spawn("oJaws", Vec2::new(200.0, 142.0));
            assert_eq!(game.world.entity(id).unwrap().health, 40);
            // The authored mouth is wet for exactly two neutral steps. Jaws
            // falls from it, then forty ordinary dry-mouth steps kill it.
            for _ in 0..2 {
                game.step(&InputFrame::default());
                assert_eq!(game.world.entity(id).unwrap().health, 40);
            }
            for _ in 0..39 {
                game.step(&InputFrame::default());
            }
            let actor = game
                .world
                .entity(id)
                .expect("NATURAL_DRYING_HP_ONE")
                .clone();
            assert_eq!(actor.health, 1);
            assert_eq!(actor.state, 2);
            assert!(actor.timer > 0);
            assert_eq!(actor.value("bubble_timer"), 0.0);
            assert_eq!(actor.pos, Vec2::new(244.0, 145.0));
            assert_eq!(game.player.health, 4.0);
            let prior = prior_ids(&game);
            let mut expected = LiteralRng::at(&game);
            let (blood_pos, blood, bones, crates) = jaws_death_oracle(&mut expected, actor.pos, 4);
            let bubble_velocity = -(1.0 + expected.range(3) as f64) * 0.1;
            game.step(&InputFrame::default());
            assert!(game.world.entity(id).is_none());
            assert_credit(&game, "megamouths");
            assert_blood(&game, &prior, blood_pos, &blood);
            let actual_bones = born(&game, &prior, "oBone");
            assert_eq!(actual_bones.len(), 4);
            for (bone, (pos, vel, gravity)) in actual_bones.iter().zip(bones) {
                assert_eq!(bone.pos, pos);
                assert_eq!(bone.vel, vel);
                assert_eq!(bone.value("gravity"), gravity);
                assert_eq!(bone.value("life"), 60.0);
            }
            let actual_crates = born(&game, &prior, "oCrate");
            assert_eq!(actual_crates.len(), 4, "SOLE_JAWS_DEATH_BLOCK");
            for (item, vel) in actual_crates.iter().zip(crates) {
                assert_eq!(item.pos, Vec2::new(actor.pos.x + 16.0, actor.pos.y + 16.0));
                assert_eq!(item.vel, vel);
            }
            let bubble = born(&game, &prior, "oBubble");
            assert_eq!(bubble.len(), 1, "POST_DESTROY_DUE_JAWS_BUBBLE");
            assert!(actual_crates.last().unwrap().id < bubble[0].id);
            assert_eq!(bubble[0].pos, Vec2::new(actor.pos.x, actor.pos.y + 16.0));
            assert_eq!(bubble[0].vel, Vec2::new(0.0, bubble_velocity));
            assert_rng(&game, &expected, "POST_DESTROY_JAWS_BUBBLE_CREATE_RNG");
            game.step(&InputFrame::default());
            assert_credit(&game, "megamouths");
        }
    }
}

#[test]
fn configured_dead_dry_piranha_keeps_pause_bubble_and_fishbone_with_blood_cap_controls() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for canonical in [false, true] {
            for (detritus, bloodless, blood_count) in [(0, false, 3), (16, false, 0), (0, true, 0)]
            {
                let mut game = room(rules, canonical, true);
                for _ in 0..detritus {
                    game.spawn("oBone", Vec2::new(300.0, 80.0));
                }
                let id = game.spawn("oPiranha", Vec2::new(200.0, 96.0));
                let fish = game.world.entity_mut(id).unwrap();
                // Explicit configured branch fixture; no natural dry PAUSE kill claim.
                fish.health = 0;
                fish.state = 2;
                fish.timer = 0;
                fish.data.insert("bloodless".into(), f64::from(bloodless));
                let prior = prior_ids(&game);
                let next_id = game.world.next_id;
                let mut expected = LiteralRng::at(&game);
                let blood: Vec<_> = (0..blood_count).map(|_| expected.blood_create()).collect();
                expected.range(2); // PAUSE expiry selects new dir
                let bubble_velocity = -(1.0 + expected.range(3) as f64) * 0.1;
                expected.range(21); // Piranha bubbleTimer jitter after Bubble Create
                game.step(&InputFrame::default());
                assert!(game.world.entity(id).is_none());
                assert_credit(&game, "piranhas");
                assert_blood(&game, &prior, Vec2::new(204.0, 100.0), &blood);
                // Bubble504 follows Piranha481, so this dry newborn runs its
                // own Step and retires immediately. FishBone482 also Steps,
                // and its identity proves Bubble was allocated before it.
                assert!(born(&game, &prior, "oBubble").is_empty());
                let bones = born(&game, &prior, "oFishBone");
                assert_eq!(bones.len(), 1, "POST_DESTROY_DRY_FISHBONE");
                assert_eq!(bones[0].id, next_id + blood_count as u64 + 1);
                assert_eq!(game.world.next_id, next_id + blood_count as u64 + 2);
                assert_eq!(bones[0].pos, Vec2::new(200.0, 96.0));
                assert!(bubble_velocity < 0.0);
                assert_rng(&game, &expected, "POST_DESTROY_PAUSE_BUBBLE_FISHBONE_RNG");
                assert_eq!(game.player.health, 4.0);
                game.step(&InputFrame::default());
                assert_credit(&game, "piranhas");
            }
        }
    }
}

#[test]
fn configured_jaws_health_zero_dry_executes_one_loot_block_through_detritus_caps() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for canonical in [false, true] {
            for (detritus, bloodless, blood_count, bone_count) in [
                (0, false, 4, 4),
                (16, false, 0, 4),
                (32, false, 0, 0),
                (0, true, 0, 4),
            ] {
                let mut game = room(rules, canonical, true);
                for _ in 0..detritus {
                    game.spawn("oBone", Vec2::new(300.0, 80.0));
                }
                let id = game.spawn("oJaws", Vec2::new(200.0, 96.0));
                let actor = game.world.entity_mut(id).unwrap();
                actor.health = 0;
                actor.state = 2;
                actor.timer = 0;
                actor.data.insert("bloodless".into(), f64::from(bloodless));
                let prior = prior_ids(&game);
                let mut expected = LiteralRng::at(&game);
                let (blood_pos, blood, _, crates) =
                    jaws_death_oracle(&mut expected, Vec2::new(200.0, 96.0), blood_count);
                let bubble_velocity = -(1.0 + expected.range(3) as f64) * 0.1;
                game.step(&InputFrame::default());
                assert!(game.world.entity(id).is_none());
                assert_credit(&game, "megamouths");
                assert_blood(&game, &prior, blood_pos, &blood);
                assert_eq!(
                    born(&game, &prior, "oBone").len(),
                    bone_count,
                    "DIRECT_BONE_DETRITUS32_CAP"
                );
                let items = born(&game, &prior, "oCrate");
                assert_eq!(items.len(), 4, "NO_DOUBLE_DEATH_LOOT");
                for (item, vel) in items.iter().zip(crates) {
                    assert_eq!(item.pos, Vec2::new(216.0, 112.0));
                    assert_eq!(item.vel, vel);
                }
                let bubbles = born(&game, &prior, "oBubble");
                assert_eq!(bubbles.len(), 1, "POST_DESTROY_JAWS_BUBBLE_WITH_CAP");
                assert_eq!(bubbles[0].pos, Vec2::new(200.0, 112.0));
                assert_eq!(bubbles[0].vel, Vec2::new(0.0, bubble_velocity));
                assert_rng(
                    &game,
                    &expected,
                    "CAPPED_BONE_CREATE_STILL_CONSUMES_CHILD_RANDOM_DRAWS",
                );
                game.step(&InputFrame::default());
                assert_credit(&game, "megamouths");
            }
        }
    }
}

#[test]
fn ordinary_live_piranha_and_jaws_keep_first_bubble_without_death_effects() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for canonical in [false, true] {
            for object in ["oPiranha", "oJaws"] {
                let mut game = room(rules, canonical, true);
                let pos = if object == "oPiranha" {
                    Vec2::new(200.0, 146.0)
                } else {
                    Vec2::new(200.0, 128.0)
                };
                let id = game.spawn(object, pos);
                let prior = prior_ids(&game);
                let mut expected = LiteralRng::at(&game);
                let bubble_velocity = -(1.0 + expected.range(3) as f64) * 0.1;
                let timer = if object == "oPiranha" {
                    30.0 + expected.range(21) as f64
                } else {
                    40.0
                };
                game.step(&InputFrame::default());
                let actor = game.world.entity(id).unwrap();
                assert_eq!(actor.health, if object == "oPiranha" { 1 } else { 40 });
                assert_eq!(actor.value("bubble_timer"), timer);
                let bubbles = born(&game, &prior, "oBubble");
                assert_eq!(bubbles.len(), 1);
                assert_eq!(bubbles[0].vel.y, bubble_velocity);
                assert!(born(&game, &prior, "oBlood").is_empty());
                assert!(born(&game, &prior, "oCrate").is_empty());
                assert_eq!(game.data.get("kills"), None);
                assert_rng(&game, &expected, "LIVE_FISH_BUBBLE_CREATE_ORACLE");
            }
        }
    }
}
