//! Initially fractional authored loot; subsequent controls are ordinary inputs.
//! Lava's source +0.05 sink is a fractional producer, but this fixture does not
//! claim a generated lava-to-pickup trajectory.
use spelunky_core::{Game, GameEvent, InputFrame, RulesPreset, Vec2, catalog, level::Level};

fn room(rules: RulesPreset) -> Game {
    let mut level = Level::default();
    level.set_tile(5, 6, '@').unwrap();
    for x in 0..40 {
        level.set_tile(x, 7, '1').unwrap();
    }
    let mut game = Game::shell(42, rules);
    game.load_custom(&level, "treasure-cached-contact", "");
    for _ in 0..8 {
        game.step(&InputFrame::default());
    }
    assert_eq!(game.player.pos, Vec2::new(104.0, 120.0));
    assert!(game.player.grounded);
    assert_eq!(game.player.health, 4.0);
    assert!(!game.player.is_stunned());
    game
}

#[test]
fn authored_fractional_gold_touch_uses_cached_integer_bbox_and_complete_accounting() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        // sGoldChunk origin(2,2), aggregate bbox(0,0..3,3): left is
        // RTE(x-2). Player's literal source query has maximum x112.
        for (x, touches) in [(114.25, true), (114.75, false), (115.25, false)] {
            let mut game = room(rules);
            let start_money = game.player.money;
            let id = game.spawn("oGoldChunk", Vec2::new(x, 126.0));
            let item = game.world.entity(id).unwrap();
            assert!(item.engine_active);
            assert_eq!(item.value("canCollect"), 1.0);
            assert_eq!(item.value("value"), 100.0);
            assert!(!item.held);
            assert_eq!(item.sprite, "sGoldChunk");
            let mask = catalog::instance_mask(
                &item.object,
                &item.sprite,
                item.pos,
                item.animation,
                Vec2::new(1.0, 1.0),
                0.0,
            );
            assert_eq!(
                mask.intersects_rectangle_bbox(Vec2::new(96.0, 112.0), Vec2::new(112.0, 128.0)),
                touches
            );
            let before_rng = game.rng.state_bits();
            game.step(&InputFrame::default());
            assert_eq!(game.player.pos, Vec2::new(104.0, 120.0));
            assert_eq!(game.player.health, 4.0);
            assert!(!game.player.is_stunned());
            assert_eq!(game.rng.state_bits(), before_rng);
            assert_eq!(
                game.world.entity(id).is_none(),
                touches,
                "SOURCE_CACHED_TREASURE_TOUCH {rules:?} x{x}"
            );
            assert_eq!(
                game.data.get("collect").copied().unwrap_or(0.0),
                if touches { 100.0 } else { 0.0 }
            );
            assert_eq!(
                game.data.get("collectCounter").copied().unwrap_or(0.0),
                if touches { 20.0 } else { 0.0 }
            );
            assert_eq!(
                game.data.get("gold").copied().unwrap_or(0.0),
                if touches { 1.0 } else { 0.0 }
            );
            assert_eq!(
                game.data
                    .get("collected:oGoldChunk")
                    .copied()
                    .unwrap_or(0.0),
                if touches { 1.0 } else { 0.0 }
            );
            assert_eq!(game.player.money, start_money);
            assert_eq!(
                game.events
                    .iter()
                    .filter(|e| matches!(e,
                GameEvent::Sound {name,..} if name=="sndCoin"))
                    .count(),
                usize::from(touches)
            );
            assert_eq!(
                game.events
                    .iter()
                    .filter(|e| matches!(e,
                GameEvent::Collected {object,value} if object=="oGoldChunk" && *value==100))
                    .count(),
                usize::from(touches)
            );
            assert!(!game.events.iter().any(|e| matches!(e,
                GameEvent::Sound {name,..} if name=="sndBreak" || name=="sndGem")));
            if touches {
                for _ in 0..20 {
                    game.step(&InputFrame::default());
                }
                assert_eq!(game.player.money, start_money);
                game.step(&InputFrame::default());
                assert_eq!(game.player.money, start_money + 100);
                assert_eq!(game.data["xmoney"], 100.0);
                assert_eq!(game.data["collect"], 0.0);
                assert_eq!(game.data["gold"], 1.0);
            }
        }
    }
}
