//! Player Step78-86 resets hidden dead velocity;361-363 still adds bounced +1.
//! Real Ghost Collision creates invisibility; DIED belongs to the next own Step.
use spelunky_core::{
    Area, Game, InputFrame, PlayerState, RulesPreset, RunState, Vec2, World, characters,
};

fn matrix(mut run: impl FnMut(RulesPreset, characters::Character)) {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for character in [
            characters::Character::Explorer,
            characters::Character::Damsel,
            characters::Character::TunnelMan,
        ] {
            run(rules, character);
        }
    }
}

fn authored(rules: RulesPreset, character: characters::Character, supported: bool) -> Game {
    let mut game = Game::shell(42, rules);
    game.world = World::new(30, 30, Area::Mines, 1);
    characters::apply(&mut game, character);
    if supported {
        for x in 0..30 {
            game.world.set_tile(x, 6, "oBlock");
        }
    }
    game.player.pos = Vec2::new(104.0, if supported { 88.0 } else { 80.0 });
    game.player.previous_pos = game.player.pos;
    game
}

fn ghost_death(game: &mut Game) {
    game.spawn(
        "oGhost",
        Vec2::new(game.player.pos.x - 8.0, game.player.pos.y - 8.0),
    );
    game.step(&InputFrame::default());
    assert_eq!(game.player.health, -99, "actual Ghost Collision must occur");
    assert_eq!(game.player.data.get("visible"), Some(&0.0));
    assert_eq!(game.player.invincible, 9999);
    assert_eq!(game.state, RunState::Playing, "Collision does not run DIED");
    assert_ne!(game.player.state, PlayerState::Dead);
    let first_velocity = game.player.vel;
    let unsupported = !game.player.grounded;
    game.step(&InputFrame::default());
    assert_eq!(game.state, RunState::Dead);
    assert_eq!(game.player.state, PlayerState::Dead);
    assert_eq!(game.player.health, 0);
    if unsupported {
        assert_eq!(
            game.player.vel.y,
            first_velocity.y + 1.0,
            "first post-Ghost own Step is still the live controller before DIED"
        );
    }
}

#[test]
fn natural_ghost_hidden_dead_body_descends_one_pixel_per_tick() {
    matrix(|rules, character| {
        let mut game = authored(rules, character, false);
        ghost_death(&mut game);
        for _ in 0..6 {
            let previous = game.player.pos;
            game.step(&InputFrame::default());
            assert_eq!(
                game.player.pos,
                Vec2::new(previous.x, previous.y + 1.0),
                "hidden dead source reset then bounced +1: {rules:?} {character:?}"
            );
            assert_eq!(game.player.vel, Vec2::new(0.0, 1.0));
            assert_eq!(game.player.data.get("gravity"), Some(&0.0));
            assert_eq!(game.player.data.get("my_gravity"), Some(&0.0));
            assert_eq!(game.player.data.get("bounced"), Some(&1.0));
            assert_eq!(game.player.data.get("visible"), Some(&0.0));
            assert_eq!(game.state, RunState::Dead);
            assert!(!game.player.grounded);
        }
    });
}

#[test]
fn natural_supported_ghost_hidden_dead_body_remains_on_floor() {
    matrix(|rules, character| {
        let mut game = authored(rules, character, true);
        ghost_death(&mut game);
        for _ in 0..6 {
            game.step(&InputFrame::default());
            assert_eq!(game.player.pos, Vec2::new(104.0, 88.0));
            assert_eq!(game.player.vel, Vec2::default());
            assert!(game.player.grounded);
            assert_eq!(game.player.data.get("visible"), Some(&0.0));
            assert_eq!(game.state, RunState::Dead);
        }
    });
}

#[test]
fn natural_live_fall_without_ghost_keeps_accelerating() {
    matrix(|rules, character| {
        let mut game = authored(rules, character, false);
        let health = game.player.health;
        for speed in 1..=6 {
            let previous = game.player.pos;
            game.step(&InputFrame::default());
            assert_eq!(game.player.vel, Vec2::new(0.0, f64::from(speed)));
            assert_eq!(
                game.player.pos,
                Vec2::new(previous.x, previous.y + f64::from(speed))
            );
            assert_eq!(game.player.health, health);
            assert_eq!(game.state, RunState::Playing);
            assert!(!game.player.grounded);
            assert_ne!(game.player.data.get("visible"), Some(&0.0));
        }
    });
}
