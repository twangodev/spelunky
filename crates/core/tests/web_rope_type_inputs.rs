//! Source Web Item collision exempts the stored Rope type, including RopeThrow.
//! Authored Web geometry with an ordinary dedicated rope input; no stock route.
use spelunky_core::*;

#[test]
fn ordinary_rope_launch_keeps_rising_through_web_contact() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for web_present in [false, true] {
            let mut game = Game::shell(42, rules);
            game.world = World::new(30, 20, Area::Mines, 1);
            game.world.name = "rLevel".into();
            for x in 0..30 {
                game.world.set_tile(x, 10, "oHardBlock");
            }
            game.player.pos = Vec2::new(152.0, 152.0);
            game.player.previous_pos = game.player.pos;
            game.player.state = PlayerState::Standing;
            game.player.grounded = true;
            game.player.ropes = 4;
            let web = if web_present {
                Some(game.spawn("oWeb", Vec2::new(144.0, 128.0)))
            } else {
                None
            };
            let rope = game.world.next_id;
            game.step(&InputFrame {
                rope_pressed: true,
                ..Default::default()
            });
            assert_eq!(game.player.health, 4);
            assert_eq!(game.player.ropes, 3);
            let rope = game
                .world
                .entity(rope)
                .expect("ordinary dedicated input creates a live rope");
            assert_eq!(rope.object, "oRopeThrow");
            assert!(!rope.held);
            assert_eq!(rope.value("armed"), 1.0);
            assert_eq!(rope.value("falling"), 0.0);
            assert_eq!(rope.pos, Vec2::new(152.0, 140.0));
            if let Some(web) = web {
                let web = game.world.entity(web).expect("source Web persists");
                assert!(
                    rope.event_intersects(web),
                    "the real pair must reach the Web collision callback"
                );
            }
            assert!(
                rope.vel.y < -10.0,
                "source Rope type is exempt from Web velocity zeroing"
            );
        }
    }
}
