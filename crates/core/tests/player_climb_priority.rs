//! characterStepEvent199-217: Up has priority even if the above-ladder probe fails.
//! Authored entrance, initial ladder overlap, then ordinary held inputs only.
use spelunky_core::{
    Game, InputFrame, PlayerState, RulesPreset, RunState, Vec2, characters, level::Level,
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

fn room(rules: RulesPreset, character: characters::Character) -> Game {
    let mut level = Level::default();
    level.set_tile(5, 8, '@').unwrap(); // Source entrance (104,152).
    for y in 4..=8 {
        level.set_tile(5, y - 1, 'L').unwrap();
    }
    for x in 0..40 {
        level.set_tile(x, 9, '1').unwrap();
    }
    let mut game = Game::shell(42, rules);
    characters::apply(&mut game, character);
    game.load_custom(&level, "source-climbing-priority", "");
    // Native rooms can author a ladder and Player at the same origin; the
    // custom text format uses one cell for its entrance, so add this initial actor.
    game.spawn("oLadderOrange", Vec2::new(96.0, 144.0));
    for _ in 0..4 {
        game.step(&InputFrame::default());
    }
    assert_eq!(game.player.pos, Vec2::new(104.0, 152.0));
    assert!(game.player.grounded);
    assert_eq!(game.state, RunState::Playing);
    game
}

fn top(rules: RulesPreset, character: characters::Character) -> Game {
    let mut game = room(rules, character);
    for _ in 0..240 {
        game.step(&InputFrame {
            up: true,
            ..Default::default()
        });
        if game.player.state == PlayerState::Climbing
            && game.player.vel == Vec2::default()
            && (64.0..72.0).contains(&game.player.pos.y)
        {
            // The uppermost segment occupies y64..79. Current origin is on
            // it, y-8 is above it, and y+8 remains on this or the next segment.
            assert_eq!(game.player.pos.x, 104.0);
            assert!(!game.player.grounded);
            return game;
        }
    }
    panic!("ordinary Up never reached a settled ladder top");
}

fn quiet(game: &Game, health: f64) {
    assert_eq!(game.player.health, health);
    assert_eq!(game.state, RunState::Playing);
    assert!(!game.player.is_stunned());
    assert_eq!(game.player.state, PlayerState::Climbing);
}

#[test]
fn natural_up_and_down_at_ladder_top_retains_up_priority() {
    matrix(|rules, character| {
        let mut both = top(rules, character);
        let mut up_only = both.clone();
        let origin = both.player.pos;
        let health = both.player.health;
        for _ in 0..12 {
            both.step(&InputFrame {
                up: true,
                down: true,
                ..Default::default()
            });
            up_only.step(&InputFrame {
                up: true,
                ..Default::default()
            });
            assert_eq!(
                both.player.vel,
                Vec2::default(),
                "Up must retain priority at ladder top: {rules:?} {character:?}"
            );
            assert_eq!(both.player.pos, origin);
            assert_eq!(both.player.pos, up_only.player.pos);
            assert_eq!(both.player.vel, up_only.player.vel);
            quiet(&both, health);
            quiet(&up_only, health);
        }
    });
}

#[test]
fn natural_down_alone_from_ladder_top_still_descends() {
    matrix(|rules, character| {
        let mut down = top(rules, character);
        let origin = down.player.pos;
        let health = down.player.health;
        for _ in 0..12 {
            down.step(&InputFrame {
                down: true,
                ..Default::default()
            });
            quiet(&down, health);
        }
        assert_eq!(down.player.pos.x, origin.x);
        assert!(down.player.pos.y > origin.y + 4.0);
        assert!(down.player.vel.y > 0.5);
    });
}

#[test]
fn natural_up_and_down_inside_ladder_retains_ordinary_ascent() {
    matrix(|rules, character| {
        let mut both = room(rules, character);
        for _ in 0..20 {
            both.step(&InputFrame {
                up: true,
                ..Default::default()
            });
        }
        assert_eq!(both.player.state, PlayerState::Climbing);
        assert!(both.player.pos.y > 96.0 && both.player.pos.y < 144.0);
        let origin = both.player.pos;
        let mut up_only = both.clone();
        let health = both.player.health;
        for _ in 0..12 {
            both.step(&InputFrame {
                up: true,
                down: true,
                ..Default::default()
            });
            up_only.step(&InputFrame {
                up: true,
                ..Default::default()
            });
            assert_eq!(both.player.pos, up_only.player.pos);
            assert_eq!(both.player.vel, up_only.player.vel);
            quiet(&both, health);
            quiet(&up_only, health);
        }
        assert!(both.player.pos.y < origin.y);
    });
}
