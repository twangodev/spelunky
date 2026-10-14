//! Source-grounded authored-room input regression.
//! Off-grid integer placement distinguishes literal LadderTop from VineTop;
//! this is not a claim about an untouched campaign layout.
use spelunky_core::{Area, Game, InputFrame, PlayerState, RulesPreset, Vec2, World};

fn setup(rules: RulesPreset, top: &str) -> Game {
    let mut game = Game::shell(42, rules);
    game.world = World::new(20, 15, Area::Jungle, 5);
    game.world.name = "rLevel2".into();
    for x in 0..20 {
        game.world.set_tile(x, 10, "oHardBlock");
    }
    game.spawn("oLadderOrange", Vec2::new(80., 144.));
    game.spawn(top, Vec2::new(80., 148.));
    game.player.pos = Vec2::new(88., 152.);
    game.player.previous_pos = game.player.pos;
    game
}

fn healthy_step(game: &mut Game, input: InputFrame) {
    game.step(&input);
    assert_eq!(game.player.health, 4, "healthy ordinary authored route");
    assert!(!game.player.is_stunned());
}

#[test]
fn natural_down_on_vine_top_releases_to_standing_and_accepts_walking() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = setup(rules, "oVineTop");
        healthy_step(&mut game, InputFrame::default());
        assert_eq!(game.player.state, PlayerState::Standing);
        assert!(game.player.grounded);
        assert_eq!(game.player.pos, Vec2::new(88., 152.));
        healthy_step(
            &mut game,
            InputFrame {
                down: true,
                ..Default::default()
            },
        );
        let down_state = game.player.state;
        healthy_step(&mut game, InputFrame::default());
        let released_state = game.player.state;
        for _ in 0..3 {
            healthy_step(
                &mut game,
                InputFrame {
                    right: true,
                    ..Default::default()
                },
            );
        }
        // Do not assert the intermediate wrong state before exercising the
        // legal release/walk sequence: the baseline must expose its lost walk.
        assert!(
            (game.player.vel.x - 1.8).abs() < 1e-12,
            "SOURCE_VINE_IS_NOT_LADDER_TOP_WALK rules={rules:?} down={down_state:?} release={released_state:?} player={:?}",
            game.player
        );
        assert_eq!(game.player.pos, Vec2::new(90., 152.));
        assert_eq!(down_state, PlayerState::Ducking);
        assert_eq!(released_state, PlayerState::Standing);
        assert_eq!(game.player.state, PlayerState::Running);
        assert!(game.player.grounded);
    }
}

#[test]
fn natural_down_on_real_ladder_top_retains_the_source_climbing_attachment() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = setup(rules, "oLadderTop");
        healthy_step(&mut game, InputFrame::default());
        assert_eq!(game.player.state, PlayerState::Standing);
        assert!(game.player.grounded);
        assert_eq!(game.player.pos, Vec2::new(88., 152.));
        healthy_step(
            &mut game,
            InputFrame {
                down: true,
                ..Default::default()
            },
        );
        assert_eq!(game.player.state, PlayerState::Climbing);
        assert_eq!(game.player.pos, Vec2::new(88., 152.));
        healthy_step(&mut game, InputFrame::default());
        assert_eq!(game.player.state, PlayerState::Climbing);
        assert_eq!(game.player.vel, Vec2::default());
        assert_eq!(game.player.pos, Vec2::new(88., 152.));
    }
}
