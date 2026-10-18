//! A ladder through a one-way platform must permit downward climbing.
//! Geometry is authored initially; later changes use ordinary Game.step input.
use spelunky_core::*;

fn room(rules: RulesPreset, solid_support: bool) -> Game {
    let mut level = level::Level::default();
    level.set_tile(8, 4, '@').unwrap();
    for x in 0..40 {
        level.set_tile(x, 9, '1').unwrap();
    }
    if solid_support {
        level.set_tile(8, 5, '1').unwrap();
    }
    let mut game = Game::shell(42, rules);
    game.load_custom(&level, "ladder-descent-support", "");
    if !solid_support {
        game.spawn("oTreeBranchTile", Vec2::new(144.0, 96.0));
    }
    game.spawn("oLadderOrange", Vec2::new(144.0, 96.0));
    game
}

fn safe(game: &Game) {
    assert_eq!(game.state, RunState::Playing);
    assert_eq!(game.player.health, 4.0);
    assert!(!game.player.is_stunned());
    assert!(!game.world.solid_at(game.player.pos.x, game.player.pos.y));
}

#[test]
fn natural_climbing_down_leaves_platform_without_solid_stop() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = room(rules, false);
        for _ in 0..4 {
            game.step(&InputFrame::default());
        }
        safe(&game);
        assert_eq!(game.player.pos, Vec2::new(152.0, 88.0));
        assert!(game.player.grounded);
        assert!(!game.world.solid_at(152.0, 96.0));
        game.step(&InputFrame {
            down: true,
            down_pressed: true,
            ..Default::default()
        });
        safe(&game);
        assert_eq!(game.player.state, PlayerState::Climbing);
        assert_eq!(game.player.pos, Vec2::new(152.0, 88.0));
        let mut descended = false;
        for _ in 0..16 {
            game.step(&InputFrame {
                down: true,
                ..Default::default()
            });
            safe(&game);
            assert_eq!(game.player.pos.x, 152.0);
            if game.player.pos.y > 88.0 {
                assert_eq!(game.player.state, PlayerState::Climbing);
                descended = true;
                break;
            }
        }
        assert!(
            descended,
            "source216 checks Solid-only; platform contact must not repeatedly reattach without descent: {:?}",
            game.player.pos
        );
    }
}

#[test]
fn natural_solid_support_keeps_grounded_down_ducking() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = room(rules, true);
        for _ in 0..4 {
            game.step(&InputFrame::default());
        }
        safe(&game);
        assert_eq!(game.player.pos, Vec2::new(152.0, 88.0));
        assert!(game.world.solid_at(152.0, 96.0));
        for tick in 0..16 {
            game.step(&InputFrame {
                down: true,
                down_pressed: tick == 0,
                ..Default::default()
            });
            safe(&game);
            assert_eq!(game.player.pos, Vec2::new(152.0, 88.0));
            assert_eq!(game.player.state, PlayerState::Ducking);
        }
    }
}
