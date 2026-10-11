//! Public pickup and pistol inputs expose normal Spider's post-Destroy recovery roll.
//! Literal draw order uses the port stream; equal GameMaker seed values are not claimed.
use spelunky_core::*;

struct LiteralRng(u64);
impl LiteralRng {
    fn at(game: &Game) -> Self {
        Self(
            serde_json::to_value(&game.rng).unwrap()["state"]
                .as_u64()
                .unwrap(),
        )
    }
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut n = self.0;
        n = (n ^ (n >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        n = (n ^ (n >> 27)).wrapping_mul(0x94d049bb133111eb);
        n ^ (n >> 31)
    }
    fn range(&mut self, n: u64) -> u64 {
        self.next() % n
    }
    fn random(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1_u64 << 53) as f64
    }
    fn blood(&mut self) -> (Vec2, f64) {
        (
            Vec2::new(
                self.random() * 4.0 - self.random() * 4.0,
                -1.0 - self.random() * 2.0,
            ),
            (1 + self.range(6)) as f64 * 0.1,
        )
    }
}

fn room(rules: RulesPreset, spider_x: f64) -> (Game, u64, u64) {
    let mut level = level::Level::default();
    level.set_tile(4, 8, '@').unwrap();
    for x in 0..40 {
        level.set_tile(x, 7, '1').unwrap();
        level.set_tile(x, 9, '1').unwrap();
    }
    let mut game = Game::legacy_shell(2, rules);
    game.load_custom(&level, "spider-post-destroy", "");
    let pistol = game.spawn("oPistol", game.player.pos);
    let spider = game.spawn("oSpider", Vec2::new(spider_x, 144.0));
    assert_eq!(game.player.pos, Vec2::new(88.0, 152.0));
    assert_eq!(game.world.entity(spider).unwrap().state, 0);
    (game, pistol, spider)
}

fn acquire_pistol(game: &mut Game, pistol: u64) {
    for _ in 0..4 {
        game.step(&InputFrame::default());
    }
    assert_eq!(game.world.entity(pistol).unwrap().value("cost"), 0.0);
    game.step(&InputFrame {
        down: true,
        ..Default::default()
    });
    assert_eq!(game.player.state, PlayerState::Ducking);
    game.step(&InputFrame {
        down: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert_eq!(game.player.held, Some(pistol), "ACTUAL_PISTOL_PICKUP");
    assert!(game.world.entity(pistol).unwrap().held);
    game.step(&InputFrame {
        attack_released: true,
        ..Default::default()
    });
    assert_eq!(game.player.state, PlayerState::Standing);
    assert!(game.player.grounded);
    assert_eq!(game.player.pos, Vec2::new(88.0, 152.0));
    assert_eq!(game.player.facing, 1);
}

#[test]
fn natural_pistol_kill_keeps_spider_idle_roll_after_blood_and_destroy() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, pistol, spider) = room(rules, 306.0);
        acquire_pistol(&mut game, pistol);
        let mut firing = None;
        // Select BEFORE firing from the source alarm/geometry, never by a
        // successful post-death outcome. No state, HP, timer or RNG is written.
        for _ in 0..96 {
            let actor = game.world.entity(spider).unwrap();
            assert_eq!(actor.health, 1, "LIVE_BEFORE_SHOT");
            let mut prediction = LiteralRng::at(&game);
            let speed = 6.0 + prediction.range(3) as f64;
            let muzzle = game.player.pos.x + 8.0;
            // Bullet's source mask right edge is origin+1. The flat corridor
            // keeps the recovering target stationary until its due Alarm0.
            let flight = ((actor.pos.x - (muzzle + 1.0)) / speed).ceil() as i32;
            if actor.state == 2
                && actor.pos.y == 144.0
                && actor.vel.x == 0.0
                && actor.alarms[0] == flight
                && flight > 0
            {
                let player = game.player.sprite_bounds();
                assert!(
                    actor.sprite_bounds().x - (player.x + player.w - 1.0) > 90.0,
                    "SOURCE_FAR_BOUNCE_FALLBACK"
                );
                firing = Some((speed, flight));
                break;
            }
            game.step(&InputFrame::default());
        }
        let (speed, flight) = firing.expect("NATURAL_RECOVERY_ALARM_FIRE_WINDOW");
        assert_eq!(game.player.vel.x, 0.0);
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
        assert_eq!(bullet.value("owner_player"), 1.0);
        let mut lethal = None;
        for tick in 1..=32 {
            let actor = game.world.entity(spider).expect("WAIT_FOR_REAL_BULLET_HIT");
            if actor.health <= 0 {
                lethal = Some(actor.clone());
                break;
            }
            game.step(&InputFrame {
                attack_released: tick == 1,
                ..Default::default()
            });
        }
        let actor = lethal.expect("NATURAL_LETHAL_PISTOL_HIT");
        assert_eq!(actor.health, -3);
        assert_eq!(
            actor.state, 0,
            "LITERAL_POST_ALARM_IDLE_DEATH_PRECONDITION flight={flight}"
        );
        assert!(actor.alarms[0] <= 0, "NO_NEXT_FRAME_ALARM_PREFIX");
        assert_eq!(actor.vel, Vec2::new(speed, -4.0));
        assert!(
            (139.0..=142.0).contains(&actor.pos.y),
            "LOW_CEILING_JUMP_ORIGIN"
        );
        assert_eq!(actor.value("swimming"), 0.0);
        assert_eq!(actor.value("burning"), 0.0);
        assert_eq!(actor.value("bloodless"), 0.0);
        assert!(!actor.held);
        assert!(
            game.world.entity(bullet_id).is_none(),
            "REAL_BULLET_RETIRED_ON_HIT"
        );
        assert_eq!(game.player.held, Some(pistol));
        assert_eq!(game.player.health, 4);
        assert!(!game.player.is_stunned());
        let prior: std::collections::BTreeSet<_> =
            game.world.entities.iter().map(|e| e.id).collect();
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == "oBlood")
                .count(),
            1,
            "CANONICAL_BULLET_HIT_BLOOD_PREFIX"
        );
        assert!(
            game.world
                .entities
                .iter()
                .filter(|e| e.alive && e.engine_active && catalog::is_a(&e.object, "oDetritus"))
                .count()
                <= 13
        );
        // Source moveTo adds integer horizontal Bullet knockback, and its
        // upward probe stops at y139 (bboxTop5, ceiling bottom143).
        let origin = Vec2::new(actor.pos.x + speed + 8.0, 147.0);
        let mut expected = LiteralRng::at(&game);
        let blood = [expected.blood(), expected.blood(), expected.blood()];
        let _new_recovery_alarm = 5 + expected.range(16);
        game.step(&InputFrame::default());
        assert!(game.world.entity(spider).is_none());
        let mut drops: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|e| e.alive && e.object == "oBlood" && !prior.contains(&e.id))
            .collect();
        drops.sort_by_key(|e| e.id);
        assert_eq!(drops.len(), 3);
        for (drop, (velocity, gravity)) in drops.into_iter().zip(blood) {
            assert_eq!(drop.previous_pos, origin, "DEATH_MOVE_THEN_BLOOD_ORIGIN");
            assert_eq!(drop.vel, velocity, "THREE_BLOOD_CREATE_ORDER");
            assert_eq!(drop.value("gravity"), gravity);
            // Spider366 precedes Blood370: newborns run their first own Step
            // this frame, unlike the accepted later-resource Snake552 burst.
            assert_eq!(drop.value("life"), 59.0, "SOURCE_NEWBORN_STEP_ORDER");
            assert_eq!((drop.alarms[0], drop.alarms[1], drop.alarms[2]), (1, 1, 5));
        }
        assert_eq!(game.data.get("spiders"), Some(&1.0));
        assert_eq!(game.data.get("kills"), Some(&1.0));
        assert_eq!(game.player.health, 4);
        assert_eq!(
            LiteralRng::at(&game).0,
            expected.0,
            "POST_DESTROY_IDLE_RNG: recovery16 follows all three Blood Creates"
        );
    }
}

#[test]
fn natural_healthy_spider_keeps_idle_then_recovery_draw_boundaries() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, _, spider) = room(rules, 192.0);
        let mut expected = LiteralRng::at(&game);
        let alarm = (5 + expected.range(16)) as i32;
        game.step(&InputFrame::default());
        let actor = game.world.entity(spider).unwrap();
        assert_eq!((actor.health, actor.state, actor.alarms[0]), (1, 2, alarm));
        assert_eq!(LiteralRng::at(&game).0, expected.0, "LIVE_IDLE_SINGLE_DRAW");
        game.step(&InputFrame::default());
        let actor = game.world.entity(spider).unwrap();
        assert_eq!(
            (actor.health, actor.state, actor.alarms[0]),
            (1, 2, alarm - 1)
        );
        assert_eq!(
            LiteralRng::at(&game).0,
            expected.0,
            "RECOVERY_NO_RANDOM_DRAW"
        );
        assert!(!game.world.entities.iter().any(|e| e.object == "oBlood"));
        assert_eq!(game.data.get("spiders"), None);
        assert_eq!(game.player.health, 4);
    }
}

// CONTROLLED initial-condition witness; this is not a natural combat wet kill.
#[test]
fn controlled_dead_spider_continues_second_water_action_once() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for wet in [false, true] {
            let mut game = Game::legacy_shell(2, rules);
            if wet {
                // Explicit authored mask on abstract Water: no Swim Create splash.
                let water = game.spawn("oWater", Vec2::new(160.0, 128.0));
                game.world.entity_mut(water).unwrap().set_sprite("sWater");
            }
            let spider = game.spawn("oSpider", Vec2::new(160.0, 128.0));
            // Authored health-zero entry condition, before the first Step.
            game.world.entity_mut(spider).unwrap().health = 0;
            game.step(&InputFrame::default());
            assert!(
                game.world.entity(spider).is_none(),
                "CONTROLLED_DEATH_RETIRED"
            );
            assert_eq!(game.data.get("spiders"), Some(&1.0));
            assert_eq!(game.data.get("kills"), Some(&1.0));
            assert_eq!(
                game.world
                    .entities
                    .iter()
                    .filter(|e| e.alive && e.object == "oBlood")
                    .count(),
                3,
                "CONTROLLED_THREE_DEATH_BLOOD"
            );
            let splashes: Vec<_> = game
                .world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == "oSplash")
                .collect();
            assert_eq!(
                splashes.len(),
                usize::from(wet),
                "POST_DESTROY_WATER_SPLASH"
            );
            if wet {
                assert_eq!(
                    splashes[0].pos,
                    Vec2::new(168.0, 128.0),
                    "SOURCE_WATER_SPLASH_ORIGIN"
                );
            }
            assert_eq!(
                game.events
                    .iter()
                    .filter(|e| matches!(e, GameEvent::Sound { name, .. } if name == "sndSplash"))
                    .count(),
                usize::from(wet),
                "SOURCE_WATER_SPLASH_SOUND_ONCE"
            );
            game.step(&InputFrame::default());
            assert_eq!(game.data.get("spiders"), Some(&1.0));
            assert_eq!(game.data.get("kills"), Some(&1.0));
            assert!(
                !game
                    .events
                    .iter()
                    .any(|e| matches!(e, GameEvent::Sound { name, .. } if name == "sndSplash")),
                "RETIRED_SPIDER_DOES_NOT_REPEAT_WATER_ACTION"
            );
        }
    }
}
