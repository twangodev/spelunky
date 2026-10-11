//! Source SpringTrap Item eligibility after natural rescue reaches a collidable exit frame.
use spelunky_core::*;

fn room(rules: RulesPreset) -> Game {
    let mut game = Game::legacy_shell(123, rules);
    game.world = World::new(30, 20, Area::Mines, 1);
    for x in 0..game.world.width {
        game.world.set_tile(x, 10, "oBlock");
    }
    game.player.pos = Vec2::new(32.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game
}

#[test]
fn spring_does_not_launch_naturally_rescued_damsel_on_collidable_exit_frame() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = room(rules);
        game.spawn("oExit", Vec2::new(128.0, 144.0));
        let damsel = game.spawn("oDamsel", Vec2::new(132.0, 152.0));
        let spring = game.spawn("oSpringTrap", Vec2::new(128.0, 144.0));

        game.step(&InputFrame::default());

        let damsel_id = damsel;
        let spring_id = spring;
        let damsel = game.world.entity(damsel_id).unwrap();
        let spring = game.world.entity(spring_id).unwrap();
        assert_eq!(damsel.state, 4, "the default actor naturally entered Exit");
        assert_eq!(damsel.sprite, "sDamselExit2");
        assert_eq!(damsel.value("inactive"), 1.0);
        assert_eq!(game.data.get("pending_damsel_kisses"), Some(&1.0));
        assert_eq!(damsel.pos, Vec2::new(136.0, 152.0));
        assert_eq!(damsel.vel, Vec2::default());
        // Original separate RECTANGLE mask: sDamselExit2 frame 0 is empty.
        // Aggregate sprite bounds alone do not establish Collision contact.
        assert!(!damsel.event_intersects(spring));
        assert_eq!((spring.state, spring.timer), (0, 0));
        assert!(!game.events.iter().any(|event| matches!(event,
            GameEvent::Sound { name, .. } if name == "sndBoing")));
        assert_eq!(game.player.health, 4);

        // Only ordinary endframe animation makes this rescued actor collidable.
        // Stop before the next Collision pass once its native mask overlaps.
        for _ in 0..16 {
            let damsel = game.world.entity(damsel_id).unwrap();
            let spring = game.world.entity(spring_id).unwrap();
            if damsel.event_intersects(spring) {
                break;
            }
            assert_eq!(damsel.state, 4);
            assert_eq!(damsel.value("inactive"), 1.0);
            assert_eq!(damsel.pos, Vec2::new(136.0, 152.0));
            assert_eq!(damsel.vel, Vec2::default());
            assert_eq!(game.data.get("pending_damsel_kisses"), Some(&1.0));
            assert_eq!((spring.state, spring.timer), (0, 0));
            assert!(!game.events.iter().any(|event| matches!(event,
                GameEvent::Sound { name, .. } if name == "sndBoing")));
            assert_eq!(game.player.health, 4);
            game.step(&InputFrame::default());
        }
        assert!(
            game.world
                .entity(damsel_id)
                .unwrap()
                .event_intersects(game.world.entity(spring_id).unwrap()),
            "ordinary animation must reach a collidable exit frame within 16 steps"
        );

        // Source other.active is already false from rescue. A genuine native
        // contact must not launch it, even though its exit frame is now visible.
        game.step(&InputFrame::default());
        let damsel = game.world.entity(damsel_id).unwrap();
        let spring = game.world.entity(spring_id).unwrap();
        assert_eq!(damsel.pos, Vec2::new(136.0, 152.0));
        assert_eq!(damsel.vel, Vec2::default());
        assert_eq!(damsel.state, 4);
        assert_eq!(damsel.sprite, "sDamselExit2");
        assert_eq!(damsel.value("inactive"), 1.0);
        assert_eq!(game.data.get("pending_damsel_kisses"), Some(&1.0));
        assert_eq!((spring.state, spring.timer), (0, 0));
        assert!(!game.events.iter().any(|event| matches!(event,
            GameEvent::Sound { name, .. } if name == "sndBoing")));
        assert_eq!(game.player.health, 4);
    }
}

#[test]
fn spring_still_launches_a_default_active_unheld_item() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = room(rules);
        let rock = game.spawn("oRock", Vec2::new(136.0, 154.0));
        let spring = game.spawn("oSpringTrap", Vec2::new(128.0, 144.0));
        assert!(
            game.world
                .entity(rock)
                .unwrap()
                .event_intersects(game.world.entity(spring).unwrap())
        );

        game.step(&InputFrame::default());

        let rock = game.world.entity(rock).unwrap();
        let spring = game.world.entity(spring).unwrap();
        assert!(!rock.held);
        assert_eq!(rock.pos, Vec2::new(136.0, 130.0));
        assert_eq!(rock.vel, Vec2::new(0.0, -8.0));
        assert_eq!((spring.state, spring.timer), (1, 10));
        assert_eq!(spring.sprite, "sSpringTrapSprung");
        assert_eq!(
            game.events
                .iter()
                .filter(|event| matches!(event,
            GameEvent::Sound { name, .. } if name == "sndBoing"))
                .count(),
            1
        );
        assert_eq!(game.player.health, 4);
    }
}
