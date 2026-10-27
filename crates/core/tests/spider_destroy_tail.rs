//! Spider idle timing and post-Destroy water continuation controls.
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
        assert_eq!(game.player.health, 4.0);
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
