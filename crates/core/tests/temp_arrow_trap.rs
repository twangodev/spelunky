//! oTemp has no exported stock producer: configured rooms exercise its real Step.
use spelunky_core::*;

fn room(rules: RulesPreset, player: Vec2, trap: Vec2) -> (Game, u64) {
    let mut game = Game::shell(42, rules);
    game.world = World::new(40, 24, Area::Mines, 1);
    game.world.name = "rLevel".into();
    for x in 0..40 {
        game.world.set_tile(x, 7, "oHardBlock");
    }
    game.player.pos = player;
    game.player.previous_pos = player;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    let receiver = game.spawn("oTemp", trap);
    (game, receiver)
}
fn arrows(game: &Game) -> Vec<&Entity> {
    game.world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oArrow")
        .collect()
}
fn sounds(game: &Game) -> usize {
    game.events
        .iter()
        .filter(|event| matches!(event, GameEvent::Sound { name, .. } if name == "sndArrowTrap"))
        .count()
}

#[test]
fn ordinary_right_input_triggers_temp_once_after_stationary_prefix() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, trap) = room(rules, Vec2::new(72.0, 104.0), Vec2::new(192.0, 96.0));
        for _ in 0..4 {
            game.step(&InputFrame::default());
            assert_eq!(game.player.pos, Vec2::new(72.0, 104.0));
            assert_eq!(game.player.vel, Vec2::default());
            assert!(arrows(&game).is_empty());
            assert_eq!(sounds(&game), 0);
        }
        assert!(game.player.grounded);
        let health = game.player.health;
        for _ in 0..2 {
            game.step(&InputFrame {
                right: true,
                ..Default::default()
            });
            assert_eq!(game.player.pos, Vec2::new(72.0, 104.0));
            assert!(arrows(&game).is_empty());
        }
        game.step(&InputFrame {
            right: true,
            ..Default::default()
        });
        assert!(game.player.pos.x > 72.0 && game.player.pos.x < 200.0);
        assert_eq!(game.player.pos.y, 104.0);
        assert!(game.player.vel.x > 0.0);
        assert_eq!(game.player.health, health);
        assert_eq!(game.state, RunState::Playing);
        let children = arrows(&game);
        assert_eq!(
            children.len(),
            1,
            "oTemp own Step must fire after the moving Player prefix"
        );
        let arrow = children[0];
        assert_eq!(arrow.pos, Vec2::new(176.0, 100.0));
        assert_eq!(arrow.vel.x, -8.0);
        assert_eq!(arrow.value("gravity"), 0.2);
        assert_eq!(sounds(&game), 1);
        let arrow_id = arrow.id;
        assert_eq!(game.world.entity(trap).unwrap().value("fired"), 1.0);
        assert_eq!(
            game.world.entity(trap).unwrap().value("arrow"),
            arrow_id as f64
        );
        assert_eq!(
            game.world.entity(trap).unwrap().value("obj"),
            arrow_id as f64,
            "later nearest Item sees the same-frame child"
        );
        for _ in 0..4 {
            game.step(&InputFrame {
                right: true,
                ..Default::default()
            });
            assert_eq!(game.world.entity(trap).unwrap().value("fired"), 1.0);
            assert_eq!(sounds(&game), 0, "a fired trap does not emit again");
            assert!(
                game.world
                    .entities
                    .iter()
                    .all(|e| e.object != "oArrow" || e.id == arrow_id)
            );
        }
    }
}

#[test]
fn temp_create_inherits_solid_then_sets_own_defaults_before_overrides() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, id) = room(rules, Vec2::new(72.0, 104.0), Vec2::new(192.0, 96.0));
        let trap = game.world.entity(id).unwrap();
        assert_eq!(
            trap.text.get("type").map(String::as_str),
            Some("Arrow Trap")
        );
        assert_eq!(trap.text.get("treasure").map(String::as_str), Some(""));
        for field in [
            "blinkToggle",
            "invincible",
            "shopWall",
            "cleanDeath",
            "fired",
        ] {
            assert_eq!(trap.data.get(field), Some(&0.0));
        }
        let trap = game.world.entity_mut(id).unwrap();
        trap.data.insert("fired".into(), 7.0);
        trap.data.insert("invincible".into(), 9.0);
        game.step(&InputFrame::default());
        assert_eq!(game.world.entity(id).unwrap().value("fired"), 7.0);
        assert_eq!(game.world.entity(id).unwrap().value("invincible"), 9.0);
        assert!(arrows(&game).is_empty());
    }
}

#[test]
fn temp_alarm_fires_minus_five_without_increment_or_sound() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, id) = room(rules, Vec2::new(72.0, 104.0), Vec2::new(192.0, 96.0));
        // Authored initial overrides isolate Alarm from the independent normal
        // Step Item branch, which could otherwise see the Alarm's own Arrow.
        let trap = game.world.entity_mut(id).unwrap();
        trap.data.insert("fired".into(), 1.0);
        trap.alarms[0] = 1;
        game.step(&InputFrame::default());
        assert_eq!(game.player.health, 4.0);
        let children = arrows(&game);
        assert_eq!(
            children.len(),
            1,
            "oTemp Alarm 0 must emit independently of fired"
        );
        assert_eq!(children[0].vel.x, -5.0);
        assert_eq!(game.world.entity(id).unwrap().value("fired"), 1.0);
        assert_eq!(
            game.world.entity(id).unwrap().value("arrow"),
            children[0].id as f64
        );
        assert_eq!(sounds(&game), 0);
    }
}

#[test]
fn temp_ceil_tail_runs_even_when_fired_was_overridden() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, id) = room(rules, Vec2::new(72.0, 104.0), Vec2::new(192.25, 96.25));
        game.world
            .entity_mut(id)
            .unwrap()
            .data
            .insert("fired".into(), 7.0);
        game.step(&InputFrame::default());
        assert_eq!(game.player.pos, Vec2::new(72.0, 104.0));
        assert_eq!(
            game.world.entity(id).unwrap().pos,
            Vec2::new(193.0, 97.0),
            "oTemp independent ceil tail must run"
        );
        assert_eq!(game.world.entity(id).unwrap().value("fired"), 7.0);
        assert!(arrows(&game).is_empty());
        assert_eq!(sounds(&game), 0);
    }
}
