use super::*;

// Independent literal RNG tape for the two supported source/replay algorithms.
struct Tape {
    state: u64,
    canonical: bool,
}
impl Tape {
    fn at(game: &Game) -> Self {
        Self {
            state: game.rng.state_bits(),
            canonical: game.rng.algorithm() == RngAlgorithm::Gm8Lcg32V1,
        }
    }
    fn next(&mut self) -> u64 {
        if self.canonical {
            self.state = u64::from((self.state as u32).wrapping_mul(0x08088405).wrapping_add(1));
            self.state
        } else {
            self.state = self.state.wrapping_add(0x9e3779b97f4a7c15);
            let mut z = self.state;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
            z ^ (z >> 31)
        }
    }
    fn unit(&mut self) -> f64 {
        if self.canonical {
            self.next() as f64 / 4294967296.0
        } else {
            (self.next() >> 11) as f64 / 9007199254740992.0
        }
    }
    fn range(&mut self, n: u32) -> u32 {
        if self.canonical {
            ((self.next() * u64::from(n)) >> 32) as u32
        } else {
            (self.next() % u64::from(n)) as u32
        }
    }
    fn blood(&mut self) -> (Vec2, f64) {
        (
            Vec2::new(
                self.unit() * 4.0 - self.unit() * 4.0,
                -1.0 - self.unit() * 2.0,
            ),
            f64::from(1 + self.range(6)) * 0.1,
        )
    }
}

fn scene(legacy: bool, rules: RulesPreset) -> Game {
    let mut game = if legacy {
        Game::legacy_shell(42, rules)
    } else {
        Game::shell(42, rules)
    };
    game.world = World::new(30, 20, Area::Mines, 1);
    game.world.name = "rLevel".into();
    game.player.pos = Vec2::new(240.0, 152.0);
    game
}

#[test]
fn inherited_attack_descendants_run_literal_enemy_blood_and_exhaustion_callback() {
    // Controlled initial callback states: this isolates the selected source
    // event from later AI/death outputs. It is not a natural gameplay witness.
    let cases = [
        (1, 4.0, 0.0, 0, 0.0),
        (3, 4.0, 0.0, 0, 0.0),
        (2, 1.0, 0.0, 0, 0.0),
        (1, 4.0, 2.0, 0, 0.0),
        (1, 4.0, 0.0, 15, 0.0),
        (1, 4.0, 0.0, 16, 0.0),
        (1, 0.0, 0.0, 0, 0.0),
        (1, 1.0, 0.0, 0, 1.0),
    ];
    for legacy in [false, true] {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            for object in ["oSnake", "oBat", "oSpider", "oFrog", "oPiranha"] {
                for weapon in ["oSlash", "oMattockHit", "oMachetePre", "oMattockPre"] {
                    for (hp, left, bloodless, cap, extra_gates) in cases {
                        let mut game = scene(legacy, rules);
                        let target = game.spawn(object, Vec2::new(80.0, 80.0));
                        let attack = game.spawn(weapon, Vec2::new(96.0, 80.0));
                        // The real normal-input caller also supplies this
                        // source Create damage; this direct fixture sets it at birth.
                        game.world
                            .entity_mut(attack)
                            .unwrap()
                            .data
                            .insert("damage".into(), 2.0);
                        let receiver = game.world.entity_mut(target).unwrap();
                        receiver.health = hp;
                        receiver.data.insert("blood_left".into(), left);
                        receiver.data.insert("bloodless".into(), bloodless);
                        receiver.data.insert("invincible".into(), extra_gates);
                        receiver.data.insert("no_whip".into(), extra_gates);
                        set_counts_as_kill(receiver, false);
                        let before = receiver.clone();
                        let (w, h) = crate::catalog::sprite_dimensions(&before.sprite).unwrap();
                        let position = Vec2::new(
                            before.pos.x + f64::from(w) / 2.0,
                            before.pos.y + f64::from(h) / 2.0,
                        );
                        for _ in 0..cap {
                            game.spawn("oBone", Vec2::new(272.0, 64.0));
                        }
                        assert_eq!(
                            crate::dispatcher::collision_owner(object, weapon),
                            Some("oEnemy")
                        );
                        let mut tape = Tape::at(&game);
                        let admitted = left > 0.0 && bloodless == 0.0 && cap < 16;
                        let expected = admitted.then(|| tape.blood());

                        collision(&mut game, target, Instance::Entity(attack));

                        let blood: Vec<_> = game
                            .world
                            .entities
                            .iter()
                            .filter(|e| e.object == "oBlood")
                            .collect();
                        assert_eq!(
                            blood.len(),
                            usize::from(admitted),
                            "INHERITED_DESCENDANT_ENEMY_BLOOD {object}/{weapon}"
                        );
                        let receiver = game.world.entity(target).unwrap();
                        assert_eq!(receiver.health, hp - 2);
                        assert!(counts_as_kill(receiver));
                        assert_eq!(
                            receiver.value("blood_left"),
                            left - f64::from(left > 0.0 && hp - 2 < 0)
                        );
                        assert_eq!(receiver.state, before.state);
                        assert_eq!(receiver.vel, before.vel);
                        assert_eq!(receiver.timer, before.timer);
                        assert_eq!(receiver.value("assaulted"), before.value("assaulted"));
                        assert_eq!(
                            receiver.value("killed_by_player"),
                            before.value("killed_by_player")
                        );
                        assert_eq!(game.rng.state_bits(), tape.state);
                        if let Some((velocity, gravity)) = expected {
                            assert_eq!(blood[0].pos, position);
                            assert_eq!(blood[0].vel, velocity);
                            assert_eq!(blood[0].value("gravity"), gravity);
                            assert_eq!(blood[0].value("life"), 60.0);
                            assert_eq!(blood[0].alarms[..3], [1, 1, 5]);
                        }
                        assert!(game.world.entity(attack).is_some());
                        assert_eq!(
                            game.events
                                .iter()
                                .filter(|event| matches!(event,
                            GameEvent::Sound { name, .. } if name == "sndHit"))
                                .count(),
                            1
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn selected_specific_front_override_keeps_its_direct_blood_and_single_damage() {
    for legacy in [false, true] {
        let mut game = scene(legacy, RulesPreset::Classic);
        let target = game.spawn("oGiantSpider", Vec2::new(80.0, 80.0));
        let attack = game.spawn("oSlash", Vec2::new(96.0, 80.0));
        game.world
            .entity_mut(attack)
            .unwrap()
            .data
            .insert("damage".into(), 2.0);
        let receiver = game.world.entity_mut(target).unwrap();
        receiver.data.insert("hit_cooldown".into(), 0.0);
        receiver.data.insert("bloodless".into(), 2.0);
        set_counts_as_kill(receiver, false);
        for _ in 0..16 {
            game.spawn("oBone", Vec2::new(272.0, 64.0));
        }
        assert_eq!(
            crate::dispatcher::collision_owner("oGiantSpider", "oSlash"),
            Some("oGiantSpider")
        );
        assert_eq!(
            game.world.entity(target).unwrap().health,
            1,
            "source GiantSpider Create hp=1"
        );
        let mut tape = Tape::at(&game);
        let (velocity, gravity) = tape.blood();
        collision(&mut game, target, Instance::Entity(attack));
        let receiver = game.world.entity(target).unwrap();
        assert_eq!(
            receiver.health, 0,
            "the source override ignores other.damage=2"
        );
        assert!(!counts_as_kill(receiver));
        assert_eq!(receiver.value("hit_cooldown"), 10.0);
        let blood: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|e| e.object == "oBlood")
            .collect();
        assert_eq!(
            blood.len(),
            1,
            "direct override bypasses script bloodless/cap16"
        );
        assert_eq!(blood[0].pos, Vec2::new(96.0, 104.0));
        assert_eq!(blood[0].vel, velocity);
        assert_eq!(blood[0].value("gravity"), gravity);
        assert_eq!(game.rng.state_bits(), tape.state);
    }
}

#[test]
fn inherited_whip_blood_uses_signed_instance_sprite_dimensions() {
    // Configured Collision-event receivers, not a stock gameplay scale route.
    // Native sprite_width/height multiply resource dimensions by signed scales.
    for legacy in [false, true] {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            for object in ["oBat", "oPiranha"] {
                for weapon in ["oWhip", "oWhipPre", "oSlash", "oMattockHit"] {
                    for (xs, ys) in [(1.0, 1.0), (-1.0, 1.0), (0.5, 2.0), (2.0, -0.5)] {
                        for (cap, bloodless) in [(0, 0.0), (15, 0.0), (16, 0.0), (0, 2.0)] {
                            let mut game = scene(legacy, rules);
                            let target = game.spawn(object, Vec2::new(80.0, 80.0));
                            let attack = game.spawn(weapon, Vec2::new(96.0, 80.0));
                            game.world
                                .entity_mut(attack)
                                .unwrap()
                                .data
                                .insert("damage".into(), 2.0);
                            let receiver = game.world.entity_mut(target).unwrap();
                            receiver.health = 1;
                            receiver.data.insert("blood_left".into(), 3.0);
                            receiver.data.insert("bloodless".into(), bloodless);
                            receiver.data.insert("image_xscale".into(), xs);
                            receiver.data.insert("image_yscale".into(), ys);
                            let before = receiver.clone();
                            let (width, height) =
                                crate::catalog::sprite_dimensions(&before.sprite).unwrap();
                            // Literal native metric getters, independent of the production helper.
                            let expected_pos = Vec2::new(
                                80.0 + f64::from(width) * xs / 2.0,
                                80.0 + f64::from(height) * ys / 2.0,
                            );
                            for _ in 0..cap {
                                game.spawn("oBone", Vec2::new(272.0, 64.0));
                            }
                            assert_eq!(
                                crate::dispatcher::collision_owner(object, weapon),
                                Some("oEnemy")
                            );
                            let mut tape = Tape::at(&game);
                            let admitted = cap < 16 && bloodless == 0.0;
                            let expected_blood = admitted.then(|| tape.blood());
                            collision(&mut game, target, Instance::Entity(attack));
                            let drops: Vec<_> = game
                                .world
                                .entities
                                .iter()
                                .filter(|e| e.object == "oBlood")
                                .collect();
                            assert_eq!(drops.len(), usize::from(admitted));
                            if let Some((velocity, gravity)) = expected_blood {
                                assert_eq!(
                                    drops[0].pos, expected_pos,
                                    "SIGNED_INSTANCE_METRIC_BLOOD_ORIGIN {object}/{weapon} scale={xs},{ys}"
                                );
                                assert_eq!(drops[0].vel, velocity);
                                assert_eq!(drops[0].value("gravity"), gravity);
                                assert_eq!(drops[0].alarms[..3], [1, 1, 5]);
                            }
                            assert_eq!(game.rng.state_bits(), tape.state);
                            let receiver = game.world.entity(target).unwrap();
                            assert_eq!(receiver.health, -1);
                            assert_eq!(receiver.value("blood_left"), 2.0);
                            assert_eq!(receiver.state, before.state);
                            assert_eq!(receiver.vel, before.vel);
                            assert_eq!(receiver.timer, before.timer);
                            assert!(counts_as_kill(receiver));
                            assert!(game.world.entity(attack).is_some());
                            assert_eq!(game.events.iter().filter(|event| matches!(event, GameEvent::Sound { name, .. } if name == "sndHit")).count(), 1);
                        }
                    }
                }
            }
        }
    }
}
