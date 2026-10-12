//! Controlled initial room; every curse, movement and Draw after that uses inputs.
use spelunky_core::{Area, Game, InputFrame, PlayerState, RulesPreset, Vec2, World};

fn room(rules: RulesPreset) -> (Game, u64, u64) {
    let mut g = Game::shell(29, rules);
    g.world = World::new(72, 18, Area::Mines, 1);
    g.world.name = "rLevel".into();
    for x in 0..72 {
        g.world.set_tile(x, 10, "oHardBlock");
    }
    for y in 0..10 {
        g.world.set_tile(0, y, "oHardBlock");
    }
    g.player.pos = Vec2::new(136., 152.);
    g.player.previous_pos = g.player.pos;
    g.player.state = PlayerState::Standing;
    g.player.grounded = true;
    g.player.facing = 1;
    let first = g.spawn("oSacAltarLeft", Vec2::new(144., 144.));
    let second = g.spawn("oSacAltarLeft", Vec2::new(560., 144.));
    (g, first, second)
}

fn defile_with_bomb(g: &mut Game, altar: u64, expected: f64) {
    let bombs = g.player.bombs;
    g.step(&InputFrame {
        down: true,
        bomb_pressed: true,
        ..Default::default()
    });
    assert_eq!(g.player.bombs, bombs - 1, "ORDINARY_BOMB_BUTTON_PRODUCER");
    let mut destroyed = false;
    for tick in 0..180 {
        g.step(&InputFrame {
            left: tick < 45,
            ..Default::default()
        });
        assert!(g.player.health > 0, "BOMB_RETREAT_SURVIVES");
        if g.world.entity(altar).is_none() {
            destroyed = true;
            break;
        }
    }
    assert!(destroyed, "ORDINARY_BOMB_EXPLOSION_DEFILES_SUPPORTED_ALTAR");
    assert_eq!(g.data["kali_punish"], expected);
}

// Literal original aggregate envelopes: sBall[3,6,12,15]/origin[8,10],
// sRunLeft[1,0,14,15]/origin[8,8]. Right Draw writes scale=-1. These
// scalar endpoints are independent of the production mask/distance helpers.
fn right_run_distance(player: Vec2, ball: Vec2) -> f64 {
    let player_left = (player.x - 8.).round_ties_even();
    let player_right = (player.x + 7.).round_ties_even();
    let ball_left = (ball.x - 5.).round_ties_even();
    let ball_right = (ball.x + 4.).round_ties_even();
    let dx = (player_left - ball_right)
        .max(ball_left - player_right)
        .max(0.);
    let dy = ((player.y - 8.).round_ties_even() - (ball.y + 5.).round_ties_even())
        .max((ball.y - 4.).round_ties_even() - (player.y + 7.).round_ties_even())
        .max(0.);
    dx.hypot(dy)
}

#[test]
fn ordinary_two_altar_bombs_then_right_walk_preserves_native_tether_threshold() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut g, first, second) = room(rules);
        g.step(&InputFrame::default());
        assert!(
            !g.world.entity(second).unwrap().engine_active,
            "SECOND_ALTAR_INITIAL_VIEW_INACTIVE"
        );
        defile_with_bomb(&mut g, first, 1.);
        assert!(
            g.world.entity(second).is_some(),
            "FIRST_DEFILE_ACTIVE_FAMILY_CASCADE_LEAVES_DISTANT_ALTAR"
        );
        assert!(
            !g.world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oBall")
        );
        for _ in 0..240 {
            if g.player.pos.x >= 550. {
                break;
            }
            g.step(&InputFrame {
                right: true,
                ..Default::default()
            });
            assert!(g.player.health > 0);
        }
        assert!(
            (550. ..560.).contains(&g.player.pos.x),
            "ORDINARY_WALK_REACHES_SECOND_ALTAR"
        );
        assert!(g.world.entity(second).unwrap().engine_active);
        defile_with_bomb(&mut g, second, 2.);
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == "oBall")
                .count(),
            1
        );
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == "oChain")
                .count(),
            4
        );
        let ball = g
            .world
            .entities
            .iter()
            .find(|e| e.alive && e.object == "oBall")
            .unwrap()
            .id;
        let mut witnessed = false;
        for _ in 0..180 {
            let before_ball = g.world.entity(ball).unwrap().clone();
            let before_scale = g.player.data.get("image_xscale").copied().unwrap_or(1.);
            let before_player = g.player.pos;
            g.step(&InputFrame {
                right: true,
                ..Default::default()
            });
            let view = g.presentation();
            let current = view.world.entity(ball).expect("LIVE_CURSE_BALL");
            // A stopped, supported Ball has no horizontal inherited motion;
            // the source follow decision sees this same post-Player origin.
            if before_scale == -1.
                && before_ball.vel.x == 0.
                && current.pos.x == before_ball.pos.x
                && view.player.sprite == "sRunLeft"
                && right_run_distance(view.player.pos, current.pos) == 23.
                && view.player.pos.x - current.pos.x >= 35.
            {
                assert_eq!(
                    view.player.data.get("image_xscale"),
                    Some(&-1.),
                    "PREVIOUS_DRAW_TRANSFORM_IS_THE_COLLISION_INPUT"
                );
                assert!(
                    view.player.pos.x > before_player.x,
                    "ORDINARY_RIGHT_INPUT_MOVES_BEFORE_TETHER"
                );
                assert!(view.player.vel.x > 0., "SOURCE_GAP23_MUST_NOT_CLAMP_PLAYER");
                assert_eq!(current.vel.x, 0., "SOURCE_GAP23_MUST_NOT_START_BALL_PULL");
                witnessed = true;
                break;
            }
            assert!(g.player.health > 0);
        }
        assert!(
            witnessed,
            "MANDATORY_NATIVE_RIGHT_MIRROR_GAP23_INPUT_WITNESS"
        );
    }
}
