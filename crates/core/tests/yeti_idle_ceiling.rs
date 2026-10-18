//! Source oYeti Step80–82: IDLE wall jump checks its ceiling immediately.
//! Authored terrain and default Create only; no writes after the first Step.
use spelunky_core::{catalog, level::Level, *};

fn room(roof: bool, wall: bool, rules: RulesPreset, algorithm: RngAlgorithm) -> (Game, u64) {
    let mut level = Level::default();
    level.set_tile(1, 8, '@').unwrap();
    level.set_tile(17, 8, 'X').unwrap();
    level.set_tile(5, 8, 'y').unwrap();
    for x in 0..40 {
        level.set_tile(x, 9, '1').unwrap();
    }
    if roof {
        level.set_tile(5, 7, '1').unwrap();
    }
    if wall {
        level.set_tile(6, 8, '1').unwrap();
    }
    let mut game = Game::shell_with_algorithm(7, rules, algorithm).unwrap();
    game.load_custom(&level, "yeti-idle-ceiling", "");
    let enemies: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && catalog::is_a(&e.object, "oEnemy"))
        .collect();
    assert_eq!(enemies.len(), 1, "SOURCE_DEFAULT_SINGLE_YETI_SETUP");
    let actor = enemies[0];
    assert_eq!(actor.object, "oYeti");
    assert_eq!(actor.pos, Vec2::new(96.0, 144.0));
    assert_eq!(actor.vel, Vec2::new(2.5, 0.0));
    assert_eq!(actor.state, 0);
    assert_eq!(actor.health, 5);
    assert!(!actor.held);
    let id = actor.id;
    (game, id)
}
fn variants(mut check: impl FnMut(RulesPreset, RngAlgorithm)) {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for algorithm in [RngAlgorithm::Gm8Lcg32V1, RngAlgorithm::LegacySplitMix64V1] {
            check(rules, algorithm);
        }
    }
}
#[test]
fn natural_idle_wall_jump_cancels_upward_speed_at_touching_ceiling() {
    variants(|rules, algorithm| {
        let (mut game, id) = room(true, true, rules, algorithm);
        game.step(&InputFrame::default());
        let e = game.world.entity(id).unwrap();
        eprintln!(
            "CEILING {rules:?}/{algorithm:?} pos={:?} vel={:?} state={} health={}",
            e.pos, e.vel, e.state, e.health
        );
        assert!(e.alive);
        assert_eq!(e.health, 5);
        assert_eq!(game.player.health, 4.0);
        assert_eq!(e.pos.y, 144.0, "SOURCE_MOVE_PRECEDES_IDLE_JUMP");
        assert_eq!(e.vel.y, 0.0, "SOURCE_IDLE_WALL_JUMP_CEILING_CLAMP");
    });
}
#[test]
fn natural_idle_wall_jump_without_roof_keeps_upward_speed() {
    variants(|rules, algorithm| {
        let (mut game, id) = room(false, true, rules, algorithm);
        game.step(&InputFrame::default());
        let e = game.world.entity(id).unwrap();
        assert!(e.alive);
        assert_eq!(e.health, 5);
        assert_eq!(game.player.health, 4.0);
        assert_eq!(e.pos.y, 144.0);
        assert_eq!(e.vel.y, -6.0, "SOURCE_OPEN_CEILING_WALL_JUMP_CONTROL");
    });
}
#[test]
fn natural_idle_touching_roof_without_wall_does_not_jump() {
    variants(|rules, algorithm| {
        let (mut game, id) = room(true, false, rules, algorithm);
        game.step(&InputFrame::default());
        let e = game.world.entity(id).unwrap();
        assert!(e.alive);
        assert_eq!(e.health, 5);
        assert_eq!(game.player.health, 4.0);
        assert_eq!(e.pos.y, 144.0);
        assert_eq!(e.vel.y, 0.0, "SOURCE_NO_WALL_NO_IDLE_JUMP_CONTROL");
    });
}
