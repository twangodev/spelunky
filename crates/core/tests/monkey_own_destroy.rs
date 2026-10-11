//! Monkey own-Step post-Destroy continuation, using public Game.step.
//! Explicit configured branch fixtures are labelled; no original-runtime claim.
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

    fn walk_tail(&mut self) -> Option<u32> {
        // Monkey Step: rand(1,100), then rand(20,50) ONLY on success.
        if self.range(100) == 0 {
            Some(20 + self.range(31) as u32)
        } else {
            None
        }
    }
}

fn room(rules: RulesPreset, canonical: bool, pos: Vec2) -> (Game, u64) {
    let mut level = level::Level::default();
    level.set_tile(4, 8, '@').unwrap();
    for x in 0..40 {
        level.set_tile(x, 9, '1').unwrap();
    }
    let mut game = if canonical {
        Game::shell(42, rules)
    } else {
        Game::legacy_shell(42, rules)
    };
    game.load_custom(&level, "monkey-own-destroy-tail", "");
    let id = game.spawn("oMonkey", pos);
    assert_eq!(
        (
            game.world.entity(id).unwrap().health,
            game.world.entity(id).unwrap().state
        ),
        (1, 5)
    );
    (game, id)
}

fn sound_count(game: &Game, sound: &str) -> usize {
    game.events
        .iter()
        .filter(|event| matches!(event, GameEvent::Sound { name, .. } if name == sound))
        .count()
}

fn death_step(game: &mut Game, id: u64, tail: impl FnOnce(&mut LiteralRng)) {
    let actor = game.world.entity(id).unwrap().clone();
    assert!(actor.health <= 0);
    assert_eq!(actor.vel, Vec2::default(), "NO_MOTION_DEATH_ORIGIN");
    assert_eq!(actor.value("burning"), 0.0);
    assert_eq!(actor.value("bloodless"), 0.0);
    let prior_ids: std::collections::BTreeSet<_> =
        game.world.entities.iter().map(|e| e.id).collect();
    assert!(
        game.world
            .entities
            .iter()
            .filter(|e| e.alive && e.engine_active && catalog::is_a(&e.object, "oDetritus"))
            .count()
            <= 13
    );
    let mut expected = LiteralRng::at(game);
    let blood = [
        expected.blood_create(),
        expected.blood_create(),
        expected.blood_create(),
    ];
    tail(&mut expected);
    let health = game.player.health;
    game.step(&InputFrame::default());
    assert!(game.world.entity(id).is_none());
    assert_eq!(game.player.health, health);
    assert_eq!(game.data.get("monkeys"), Some(&1.0));
    assert_eq!(game.data.get("kills"), Some(&1.0));
    let mut born: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oBlood" && !prior_ids.contains(&e.id))
        .collect();
    born.sort_by_key(|e| e.id);
    assert_eq!(born.len(), 3, "EXACT_DEATH_BLOOD_BIRTHS");
    for (drop, (velocity, gravity)) in born.into_iter().zip(blood) {
        assert_eq!(drop.pos, Vec2::new(actor.pos.x + 8.0, actor.pos.y + 8.0));
        assert_eq!(drop.vel, velocity, "SYNCHRONOUS_BLOOD_CREATE_ORDER");
        assert_eq!(drop.value("gravity"), gravity);
        assert_eq!((drop.alarms[0], drop.alarms[1], drop.alarms[2]), (1, 1, 5));
    }
    assert_eq!(
        LiteralRng::at(game).0,
        expected.0,
        "MONKEY_POST_DESTROY_TAIL_RNG"
    );
}

#[test]
fn configured_dead_walk_keeps_roll_after_death_blood_and_destroy() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for canonical in [false, true] {
            let (mut game, id) = room(rules, canonical, Vec2::new(200.0, 144.0));
            // Explicit branch fixture: no natural WALK death producer is claimed.
            let actor = game.world.entity_mut(id).unwrap();
            actor.state = 3;
            actor.health = 0;
            death_step(&mut game, id, |rng| {
                rng.walk_tail();
            });
            assert_eq!(sound_count(&game, "sndMonkey"), 0);
            game.step(&InputFrame::default());
            assert_eq!(
                game.data.get("kills"),
                Some(&1.0),
                "RETIRED_MONKEY_NEVER_STEPS_AGAIN"
            );
        }
    }
}

#[test]
fn configured_dead_airborne_bounce_keeps_one_monkey_sound() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for canonical in [false, true] {
            let (mut game, id) = room(rules, canonical, Vec2::new(200.0, 96.0));
            // Explicit branch fixture: ordinary dispatcher, configured death/state.
            let actor = game.world.entity_mut(id).unwrap();
            actor.state = 1;
            actor.health = 0;
            death_step(&mut game, id, |_| {});
            assert_eq!(
                sound_count(&game, "sndMonkey"),
                1,
                "POST_DESTROY_AIRBORNE_BOUNCE_SOUND"
            );
            game.step(&InputFrame::default());
            assert_eq!(sound_count(&game, "sndMonkey"), 0);
            assert_eq!(game.data.get("kills"), Some(&1.0));
        }
    }
}

#[test]
fn configured_live_walk_keeps_existing_single_roll_and_no_death_effects() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for canonical in [false, true] {
            let (mut game, id) = room(rules, canonical, Vec2::new(200.0, 144.0));
            game.world.entity_mut(id).unwrap().state = 3;
            let mut expected = LiteralRng::at(&game);
            let counter = expected.walk_tail();
            game.step(&InputFrame::default());
            let actor = game.world.entity(id).unwrap();
            assert_eq!(actor.health, 1);
            assert_eq!(
                (actor.state, actor.timer),
                counter.map_or((3, 0), |n| (0, n))
            );
            assert_eq!(LiteralRng::at(&game).0, expected.0);
            assert_eq!(sound_count(&game, "sndMonkey"), 0);
            assert_eq!(game.data.get("kills"), None);
            assert!(!game.world.entities.iter().any(|e| e.object == "oBlood"));
        }
    }
}

#[test]
fn ordinary_whip_kill_at_natural_grab_expiry_keeps_theft_and_release_tail() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for canonical in [false, true] {
            let (mut game, id) = room(rules, canonical, Vec2::new(96.0, 144.0));
            // Select a live, source-created GRAB timeout before any attack.
            // After initial spawn, every state/HP/timer/position change comes
            // from ordinary neutral/Right+Attack inputs and real collisions.
            let mut ready = false;
            for _ in 0..96 {
                let actor = game.world.entity(id).expect("LIVE_UNTIL_WHIP");
                assert_eq!(actor.health, 1);
                if actor.state == 7 && actor.timer == 6 {
                    ready = true;
                    break;
                }
                game.step(&InputFrame::default());
            }
            assert!(ready, "NATURAL_GRAB_SIX_TICK_FIRE_WINDOW");
            game.step(&InputFrame {
                right: true,
                attack: true,
                attack_pressed: true,
                ..Default::default()
            });
            let mut saw_whip = false;
            let mut lethal = false;
            for _ in 0..12 {
                saw_whip |= game
                    .world
                    .entities
                    .iter()
                    .any(|e| e.alive && catalog::is_a(&e.object, "oWhip"));
                let actor = game.world.entity(id).expect("WAIT_FOR_ACTUAL_WHIP_HIT");
                if actor.health <= 0 {
                    lethal = true;
                    break;
                }
                game.step(&InputFrame {
                    right: true,
                    ..Default::default()
                });
            }
            assert!(saw_whip && lethal, "NATURAL_WHIP_DEATH_PRODUCER");
            let actor = game.world.entity(id).unwrap();
            assert_eq!(
                (actor.state, actor.timer),
                (7, 0),
                "NATURAL_GRAB_EXPIRY_DEATH_BRANCH"
            );
            assert_eq!(game.player.money, 0);
            assert_eq!((game.player.ropes, game.player.bombs), (4, 4));
            assert!(!game.player.is_stunned());
            assert_eq!(game.player.held, None);
            let mut trip = false;
            let mut thrown = None;
            death_step(&mut game, id, |rng| {
                // Source GRAB: eager money-and-rand consumes the second roll
                // even with money0. Rope/Bomb Creates have no random draw.
                if rng.range(4) == 0 {
                    trip = true;
                } else {
                    rng.range(10);
                    if rng.range(2) == 0 {
                        thrown = Some("oRopeThrow");
                    } else {
                        thrown = Some("oBomb");
                        rng.range(10); // armed-Bomb conditional
                    }
                    rng.range(3); // launch x first rand(1,3)
                    rng.range(3); // launch x second rand(1,3)
                    rng.range(2); // launch y rand(3,4)
                }
                rng.range(3); // release_monkey rand(2,4), always after theft
            });
            assert_eq!(
                sound_count(&game, "sndHit"),
                usize::from(trip),
                "POST_DESTROY_TRIP_SOUND"
            );
            assert_eq!(
                sound_count(&game, "sndThrow"),
                usize::from(thrown.is_some()),
                "POST_DESTROY_THEFT_SOUND"
            );
            assert_eq!(
                game.player.ropes,
                4 - i32::from(thrown == Some("oRopeThrow"))
            );
            assert_eq!(game.player.bombs, 4 - i32::from(thrown == Some("oBomb")));
            assert_eq!(game.player.money, 0);
            if let Some(object) = thrown {
                assert_eq!(
                    game.world
                        .entities
                        .iter()
                        .filter(|e| e.alive && e.object == object)
                        .count(),
                    1
                );
            }
            if trip {
                assert_eq!(game.player.state, PlayerState::Stunned);
                assert_eq!(game.player.stun, 40);
                assert_eq!(game.player.vel, Vec2::new(3.0, -3.0));
            }
            game.step(&InputFrame::default());
            assert_eq!(game.data.get("kills"), Some(&1.0));
            assert_eq!(sound_count(&game, "sndThrow"), 0);
        }
    }
}
