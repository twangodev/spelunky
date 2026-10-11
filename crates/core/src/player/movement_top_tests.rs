//! Source-only, UNCOMPILED/UNRUN natural-input regression proposals.
//! Only authored tiles and source-valid initial actors precede the first Step.
//! No player/actor/terrain writes occur after simulation starts. Collision top
//! is transient and resets during animate: assertions use actual displacement.
use super::*;

fn cell(level: &mut crate::level::Level, x: usize, y: usize, object: char) {
    level.set_tile(x - 1, y - 1, object).unwrap();
}

fn live(game: &Game) {
    assert_eq!(game.state, RunState::Playing);
    assert_eq!(game.player.health, 4);
    assert!(!game.player.is_stunned());
}

fn toward(direction: i32) -> InputFrame {
    InputFrame {
        left: direction < 0,
        right: direction > 0,
        ..Default::default()
    }
}

fn room(rules: RulesPreset, direction: i32, obstacle: bool, continuous_floor: bool) -> Game {
    let mut level = crate::level::Level::default();
    cell(&mut level, 8, 9, '@'); // Canonical entrance (136,152).
    for x in if continuous_floor { 6..=11 } else { 8..=9 } {
        cell(&mut level, x, 10, '1'); // Top160; departure floor spans128..159.
    }
    for x in 1..=40 {
        cell(&mut level, x, 14, '1');
    }
    let mut game = Game::shell(42, rules);
    game.load_custom(&level, "source-movement-top", "");
    if obstacle {
        // Nongrid fixed Solid is explicit initial geometry. It ends at y150;
        // first-departure source probe149..159 hits, shortened151..159 misses.
        // Left/right X differ because source movement bounds are asymmetric.
        game.spawn(
            "oBrick",
            Vec2::new(if direction > 0 { 170.0 } else { 101.0 }, 135.0),
        );
    }
    game
}

fn settle(game: &mut Game) {
    for _ in 0..4 {
        game.step(&InputFrame::default());
    }
    live(game);
    assert_eq!(game.player.pos, Vec2::new(136.0, 152.0));
    assert!(game.player.grounded);
    assert_eq!(game.player.vel, Vec2::default());
}

fn side_line(pos: Vec2, direction: i32, top: f64) -> Rect {
    // Literal getIdCollisionLeft/Right source endpoints, not current Player
    // data (characterSprite has restored top-8 after the preceding Step).
    let x = gm_round(pos.x + if direction > 0 { 5.0 } else { -6.0 });
    let y = gm_round(pos.y + top + 5.0);
    let last = gm_round(pos.y + 7.0);
    Rect::new(x, y, 1.0, last - y + 1.0)
}

#[test]
fn natural_walkoff_zero_incoming_vy_keeps_full_top_for_first_side_collision() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for direction in [-1, 1] {
            let mut wall = room(rules, direction, true, false);
            let mut open = room(rules, direction, false, false);
            settle(&mut wall);
            settle(&mut open);
            let input = toward(direction);
            let mut qualified = false;
            for _ in 0..40 {
                live(&wall);
                live(&open);
                assert_eq!(wall.player.pos, open.player.pos, "prefix differs");
                assert_eq!(wall.player.vel, open.player.vel, "prefix differs");
                if grounded_state(wall.player.state)
                    && !below(&wall.world, &wall.player)
                    && wall.player.vel.y == 0.0
                {
                    qualified = true;
                    break;
                }
                wall.step(&input);
                open.step(&input);
            }
            assert!(qualified, "FIXTURE: natural zero-vy departure not reached");
            let start = wall.player.pos;
            assert_eq!(start.y, 152.0);
            assert_eq!(start.x, if direction > 0 { 165.0 } else { 122.0 });
            assert!(!wall.player.grounded);
            assert!(!edge(&wall.world, &wall.player, Side::Top, 1.0));
            assert!(precise_line(
                &wall.world,
                side_line(start, direction, -8.0),
                "oSolid"
            ));
            assert!(!precise_line(
                &wall.world,
                side_line(start, direction, -6.0),
                "oSolid"
            ));
            assert!(!precise_line(
                &open.world,
                side_line(start, direction, -8.0),
                "oSolid"
            ));
            // Upper ledge clearance intentionally fails, so this is movement
            // collision, not an incidental automatic ledge grab.
            assert!(solid_point(
                &wall.world,
                Vec2::new(start.x + f64::from(direction) * 9.0, start.y - 9.0)
            ));
            wall.step(&input);
            open.step(&input);
            live(&wall);
            live(&open);
            assert_eq!(wall.player.state, PlayerState::Falling);
            assert_eq!(wall.player.vel.y, 1.0);
            assert_eq!(
                wall.player.pos.x, start.x,
                "CONTRACT: first fall uses top-8"
            );
            assert_eq!(wall.player.pos.y, 153.0);
            assert!(
                (open.player.pos.x - start.x) * f64::from(direction) > 0.0,
                "FIXTURE: open paired movement must actually advance"
            );
            // Positive incoming-vy control: the next actual falling controller
            // must shorten its probe; the same upper obstacle no longer stops it.
            let falling = wall.player.pos;
            assert!(wall.player.vel.y > 0.0 && !wall.player.grounded);
            assert!(!precise_line(
                &wall.world,
                side_line(falling, direction, -6.0),
                "oSolid"
            ));
            wall.step(&input);
            live(&wall);
            assert_eq!(wall.player.state, PlayerState::Falling);
            assert!(wall.player.vel.y > 1.0);
            assert!(
                (wall.player.pos.x - falling.x) * f64::from(direction) > 0.0,
                "CONTRACT: positive incoming-vy falling uses shortened top"
            );
        }
    }
}

#[test]
fn natural_grounded_wall_control_still_stops_at_the_full_side_probe() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for direction in [-1, 1] {
            let mut game = room(rules, direction, true, true);
            settle(&mut game);
            for _ in 0..32 {
                game.step(&toward(direction));
                live(&game);
            }
            assert!(game.player.grounded);
            assert_eq!(game.player.pos.y, 152.0);
            assert_eq!(game.player.vel.y, 0.0);
            assert_eq!(game.player.pos.x, if direction > 0 { 165.0 } else { 122.0 });
        }
    }
}

#[test]
fn natural_rising_head_collision_keeps_full_top_and_open_control_rises() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut ceiling = room(rules, 1, false, false);
        let mut open = room(rules, 1, false, false);
        // Initial nongrid ceiling ends137. The first jump moves148 normally;
        // its next negative-incoming-vy movement must stop at center146.
        ceiling.spawn("oBrick", Vec2::new(128.0, 122.0));
        settle(&mut ceiling);
        settle(&mut open);
        assert!(!edge(&ceiling.world, &ceiling.player, Side::Top, 1.0));
        assert!(!edge(&open.world, &open.player, Side::Top, 1.0));
        let jump = InputFrame {
            jump: true,
            jump_pressed: true,
            ..Default::default()
        };
        ceiling.step(&jump);
        open.step(&jump);
        live(&ceiling);
        live(&open);
        // Source normal-jump branch sets FALLING before integration; the next
        // controller changes it to JUMPING from actual negative incoming vy.
        assert_eq!(ceiling.player.state, PlayerState::Falling);
        assert!(ceiling.player.vel.y < 0.0);
        assert_eq!(ceiling.player.pos.y, 148.0);
        assert_eq!(ceiling.player.pos, open.player.pos);
        assert!(!edge(&ceiling.world, &ceiling.player, Side::Top, 1.0));
        // Next Step enters truly airborne with negative yVel; clipping rather
        // than a cached initial head contact supplies the physical oracle.
        ceiling.step(&InputFrame {
            jump: true,
            ..Default::default()
        });
        open.step(&InputFrame {
            jump: true,
            ..Default::default()
        });
        live(&ceiling);
        live(&open);
        assert_eq!(ceiling.player.state, PlayerState::Jumping);
        assert!(ceiling.player.vel.y < 0.0);
        assert_eq!(
            ceiling.player.pos.y, 146.0,
            "CONTRACT: rising full-top movement hits the ceiling"
        );
        assert!(
            open.player.pos.y < 146.0,
            "FIXTURE: actual open rising movement required"
        );
    }
}

#[test]
fn natural_climb_grab_hanging_control_keeps_its_supported_position() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut level = crate::level::Level::default();
        cell(&mut level, 6, 9, '@');
        for y in 6..=8 {
            cell(&mut level, 6, y, 'L');
        }
        cell(&mut level, 7, 7, '1');
        for x in 1..=40 {
            cell(&mut level, x, 10, '1');
        }
        let mut game = Game::shell(42, rules);
        game.load_custom(&level, "movement-top-hanging-control", "");
        game.spawn("oLadderOrange", Vec2::new(96.0, 144.0));
        game.spawn("oPushBlock", Vec2::new(109.0, 96.0));
        for _ in 0..4 {
            game.step(&InputFrame::default());
        }
        live(&game);
        assert!(game.player.grounded);
        assert_eq!(game.player.pos, Vec2::new(104.0, 152.0));
        let up = InputFrame {
            up: true,
            ..Default::default()
        };
        let mut climbed = false;
        for _ in 0..180 {
            live(&game);
            if game.player.state == PlayerState::Climbing
                && (101.0..=104.0).contains(&game.player.pos.y)
            {
                climbed = true;
                break;
            }
            game.step(&up);
        }
        assert!(climbed, "FIXTURE: actual ladder climb required");
        game.step(&InputFrame { right: true, ..up });
        live(&game);
        assert_eq!(
            game.player.state,
            PlayerState::Hanging,
            "FIXTURE: actual dynamic-Solid grip required"
        );
        let hanging = game.player.pos;
        assert_eq!(hanging, Vec2::new(104.0, 104.0));
        assert_eq!(game.player.vel.y, 0.0);
        game.step(&InputFrame::default());
        live(&game);
        assert_eq!(game.player.state, PlayerState::Hanging);
        assert_eq!(game.player.pos, hanging);
        assert_eq!(game.player.vel.y, 0.0);
    }
}
