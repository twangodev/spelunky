//! Ordinary authored walking isolates cached isCollisionLadder at rest.
//! The bounded route selector stops before Up and considers only geometry.
use spelunky_core::{Game, InputFrame, PlayerState, RulesPreset, Vec2, level::Level};

fn walk(rules: RulesPreset, ticks: u32, ladder: bool) -> Game {
    let mut level = Level::default();
    level.set_tile(9, 9, '@').unwrap();
    if ladder {
        level.set_tile(10, 9, 'L').unwrap();
    }
    for x in 0..40 {
        level.set_tile(x, 10, '1').unwrap();
    }
    let mut game = Game::shell(42, rules);
    game.load_custom(&level, "cached-ladder-side-looking", "");
    assert_eq!(game.player.pos, Vec2::new(168.0, 168.0));
    for _ in 0..4 {
        game.step(&InputFrame::default());
    }
    for _ in 0..ticks {
        game.step(&InputFrame {
            right: true,
            ..Default::default()
        });
    }
    for i in 0..32 {
        game.step(&InputFrame {
            right_released: i == 0,
            ..Default::default()
        });
    }
    assert_eq!(game.player.health, 4.0);
    assert!(!game.player.is_stunned());
    assert!(game.player.grounded);
    assert_eq!(game.player.vel, Vec2::default());
    assert_eq!(game.player.attack_timer, 0);
    assert_eq!(game.player.state, PlayerState::Standing);
    game
}

fn route(rules: RulesPreset) -> (u32, Game) {
    let mut trace = Vec::new();
    for ticks in 3..=16 {
        let game = walk(rules, ticks, true);
        trace.push((ticks, game.player.pos));
        // Literal source Ladder bbox [177,160,190,175]. Normal incoming
        // controller bounds (-5,-8,5,8) make the source rectangle x+3..x-3,y.
        // At192/193 the center is in empty neighboring tile, central row hits.
        if (192.0..=193.0).contains(&game.player.pos.x) && game.player.pos.y == 168.0 {
            assert_eq!(game.player.data.get("collision_half_width"), Some(&5.0));
            assert_eq!(game.player.data.get("collision_top"), Some(&-8.0));
            assert!(game.player.pos.x - 3.0 <= 190.0);
            assert!(game.player.pos.x > 190.0);
            assert!((game.player.pos.x - 184.0).abs() >= 4.0);
            println!(
                "QUALIFIED rules={rules:?} RightTicks={ticks} pos={:?} sourceRowLeft={} sourceLadderRight=190 centerMiss=true attachDistance={}",
                game.player.pos,
                game.player.pos.x - 3.0,
                (game.player.pos.x - 184.0).abs()
            );
            return (ticks, game);
        }
    }
    panic!("FIXTURE_GEOMETRY no qualifying natural route: {rules:?} {trace:?}");
}

#[test]
fn natural_rest_beside_ladder_uses_cached_rectangle_before_looking_up() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (_, mut game) = route(rules);
        let before = game.player.pos;
        game.step(&InputFrame {
            up: true,
            ..Default::default()
        });
        assert_eq!(
            game.player.pos, before,
            "No ladder snap outside source attachment distance"
        );
        assert_eq!(game.player.vel, Vec2::default());
        assert!(game.player.grounded && !game.player.is_stunned());
        assert_eq!(game.player.health, 4.0);
        assert_eq!(
            game.player.state,
            PlayerState::Standing,
            "SOURCE_CACHED_LADDER_RECTANGLE suppresses LookingUp despite missed center point; {rules:?} pos={:?}",
            game.player.pos
        );
        assert_eq!(game.player.sprite, "sStandLeft");
    }
}

#[test]
fn same_walk_without_ladder_retains_ordinary_looking_up() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (ticks, occupied) = route(rules);
        let mut game = walk(rules, ticks, false);
        assert_eq!(game.player.pos, occupied.player.pos);
        let before = game.player.pos;
        game.step(&InputFrame {
            up: true,
            ..Default::default()
        });
        assert_eq!(game.player.pos, before);
        assert_eq!(game.player.state, PlayerState::LookingUp);
        assert_eq!(game.player.sprite, "sLookLeft");
        assert_eq!(game.player.vel, Vec2::default());
        assert!(game.player.grounded && !game.player.is_stunned());
        assert_eq!(game.player.health, 4.0);
    }
}
