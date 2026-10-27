//! Ordinary Jungle frog behavior through the production event dispatcher.
use spelunky_core::{Area, Game, GameEvent, InputFrame, RulesPreset, Vec2, World};

#[test]
fn nearby_player_triggers_frog_bounce_and_recovery() {
    for (object, player_x, direction) in [
        ("oFrog", 40.0, -1.0),
        ("oFrog", 152.0, 1.0),
        ("oFireFrog", 40.0, -1.0),
        ("oFireFrog", 152.0, 1.0),
    ] {
        let mut game = Game::shell(123, RulesPreset::Classic);
        game.world = World::new(40, 30, Area::Jungle, 5);
        for x in 0..40 {
            game.world.set_tile(x, 10, "oBlock");
        }
        game.player.pos = Vec2::new(player_x, 152.0);
        game.player.previous_pos = game.player.pos;
        let frog = game.spawn(object, Vec2::new(96.0, 144.0));

        game.step(&InputFrame::default());
        let e = game.world.entity(frog).unwrap();
        assert_eq!(e.state, 1, "{object} detects the nearby player");
        assert!(
            game.events
                .iter()
                .any(|event| matches!(event, GameEvent::Sound { name, .. } if name == "sndFrog"))
        );

        game.step(&InputFrame::default());
        let e = game.world.entity(frog).unwrap();
        assert_eq!(e.state, 1);
        assert_eq!(e.vel.x, 3.0 * direction);
        assert!((-4.0..=-2.0).contains(&e.vel.y));

        game.step(&InputFrame::default());
        let e = game.world.entity(frog).unwrap();
        assert_eq!(
            e.state, 2,
            "{object} enters recovery after leaving the floor"
        );
        assert!(e.pos.y < 144.0);

        for _ in 0..100 {
            game.step(&InputFrame {
                left: direction < 0.0,
                right: direction > 0.0,
                ..InputFrame::default()
            });
            let e = game.world.entity(frog).expect("frog survives its jump");
            if e.state == 0 {
                assert!(
                    (10..=40).contains(&e.timer),
                    "{object} cooldown: {}",
                    e.timer
                );
                assert_eq!(e.vel, Vec2::default());
                break;
            }
        }
        let e = game.world.entity(frog).unwrap();
        assert_eq!(e.state, 0, "{object} lands and returns to idle");
    }
}

#[test]
fn distant_player_does_not_trigger_idle_frog() {
    for object in ["oFrog", "oFireFrog"] {
        let mut game = Game::shell(123, RulesPreset::Classic);
        game.world = World::new(40, 30, Area::Jungle, 5);
        for x in 0..40 {
            game.world.set_tile(x, 10, "oBlock");
        }
        game.player.pos = Vec2::new(200.0, 152.0);
        game.player.previous_pos = game.player.pos;
        let frog = game.spawn(object, Vec2::new(96.0, 144.0));

        for _ in 0..5 {
            game.step(&InputFrame::default());
            let e = game.world.entity(frog).unwrap();
            assert_eq!(e.state, 0, "{object} stays idle");
            assert_eq!(e.vel.x, 0.0);
            assert!(
                !game.events.iter().any(
                    |event| matches!(event, GameEvent::Sound { name, .. } if name == "sndFrog")
                )
            );
        }
    }
}
