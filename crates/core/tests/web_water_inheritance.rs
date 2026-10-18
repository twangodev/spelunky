//! Ordinary WebCannon pickup/fire reaches inherited Water collision events.
use spelunky_core::*;

#[test]
fn naturally_fired_web_ball_expands_on_water_descendants_before_life_expiry() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for liquid in ["oWaterSwim", "oLava", ""] {
            let mut game = Game::shell(7, rules);
            game.world = World::new(30, 20, Area::Mines, 1);
            game.world.name = "rLevel".into();
            for x in 0..30 {
                game.world.set_tile(x, 10, "oHardBlock");
            }
            for y in 7..10 {
                game.world.set_tile(5, y, "oHardBlock");
                game.world.set_tile(20, y, "oHardBlock");
                if !liquid.is_empty() {
                    for x in 6..20 {
                        game.spawn(liquid, Vec2::new(x as f64 * 16.0, y as f64 * 16.0));
                    }
                }
            }
            game.player.pos = Vec2::new(88.0, 104.0);
            game.player.previous_pos = game.player.pos;
            let cannon = game.spawn("oWebCannon", Vec2::new(88.0, 108.0));
            game.step(&InputFrame {
                down: true,
                ..Default::default()
            });
            game.step(&InputFrame {
                down: true,
                attack: true,
                attack_pressed: true,
                ..Default::default()
            });
            assert_eq!(
                game.player.held,
                Some(cannon),
                "natural pickup {liquid}/{rules:?}"
            );
            game.step(&InputFrame {
                attack_released: true,
                ..Default::default()
            });
            game.step(&InputFrame {
                attack: true,
                attack_pressed: true,
                ..Default::default()
            });
            let id = game
                .world
                .entities
                .iter()
                .find(|e| e.alive && e.object == "oWebBall")
                .expect("ordinary Attack must fire")
                .id;
            let mut expanded = false;
            for _ in 0..18 {
                let ball = game
                    .world
                    .entity(id)
                    .expect("flight remains within room/lifetime");
                if ball.sprite == "sWebCreate" {
                    assert!(!liquid.is_empty(), "dry control has no collision target");
                    assert!(
                        ball.value("life") > 0.0,
                        "Water collision precedes life expiry"
                    );
                    assert_eq!(ball.vel, Vec2::default());
                    assert_eq!(ball.animation, 1.0);
                    assert!(
                        game.world.entities.iter().any(|water| water.alive
                            && water.object == liquid
                            && ball.event_intersects(water)),
                        "genuine water descendant contact"
                    );
                    expanded = true;
                    break;
                }
                game.step(&InputFrame::default());
            }
            assert_eq!(expanded, !liquid.is_empty(), "{liquid}/{rules:?}");
            assert_eq!(
                game.player.health, 4.0,
                "player stays above the left retaining wall"
            );
            if expanded {
                let pos = game.world.entity(id).unwrap().pos;
                for _ in 0..4 {
                    game.step(&InputFrame::default());
                }
                assert!(
                    game.world.entity(id).is_none(),
                    "source fifth expansion frame retires WebBall"
                );
                let web = game
                    .world
                    .entities
                    .iter()
                    .find(|e| e.alive && e.object == "oWeb")
                    .expect("Animation End births Web");
                assert_eq!(web.pos, Vec2::new(pos.x - 8.0, pos.y - 8.0));
                assert_eq!(web.value("dying"), 1.0);
            }
        }
    }
}
