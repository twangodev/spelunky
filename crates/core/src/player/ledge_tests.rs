//! Controlled initial actors plus ordinary authored-entrance inputs.
//! No player pose/state/sprite, terrain, or actor writes after the first Step.
use super::*;

fn cell(level: &mut crate::level::Level, x: usize, y: usize, symbol: char) {
    // Custom files have the canonical extra border at world row/column zero.
    level.set_tile(x - 1, y - 1, symbol).unwrap();
}

fn authored(level: &crate::level::Level, rules: RulesPreset) -> Game {
    let mut game = Game::shell(42, rules);
    game.load_custom(level, "source-ledge-queries", "");
    game
}

fn quiet(game: &Game) {
    assert_eq!(game.player.health, 4);
    assert!(!game.player.is_stunned());
    assert_eq!(game.state, RunState::Playing);
}

fn shaft(rules: RulesPreset) -> Game {
    let mut level = crate::level::Level::default();
    cell(&mut level, 4, 5, '@'); // Ordinary entrance gives (72,88).
    cell(&mut level, 4, 6, '1');
    cell(&mut level, 5, 7, '1');
    for x in 1..=40 {
        cell(&mut level, x, 10, '1');
    }
    let mut game = authored(&level, rules);
    // Source instances may occupy nongrid positions. This controlled initial
    // geometry lets a natural walk enter the exact ten-pixel movement gap.
    game.spawn("oPushBlock", Vec2::new(83.0, 96.0));
    for y in [64.0, 80.0, 96.0] {
        // A fixed Solid prevents the ordinary pushing controller moving the wall.
        game.spawn("oBrick", Vec2::new(109.0, y));
    }
    game
}

#[test]
fn natural_walk_fall_both_keys_tries_valid_left_after_invalid_right() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut both = shaft(rules);
        let mut right = shaft(rules);
        for _ in 0..4 {
            both.step(&InputFrame::default());
            right.step(&InputFrame::default());
        }
        assert!(both.player.grounded);
        assert_eq!(both.player.pos, Vec2::new(72.0, 88.0));
        let toward = InputFrame {
            right: true,
            ..Default::default()
        };
        let mut qualified = false;
        let mut saw_fall = false;
        for _ in 0..80 {
            quiet(&both);
            quiet(&right);
            assert_eq!(both.player.pos, right.player.pos);
            saw_fall |= both.player.state == PlayerState::Falling && !both.player.grounded;
            if both.player.state == PlayerState::Falling
                && (101.0..=104.0).contains(&both.player.pos.y)
            {
                qualified = true;
                break;
            }
            both.step(&toward);
            right.step(&toward);
            assert_ne!(
                both.player.state,
                PlayerState::Hanging,
                "prefix grabbed prematurely"
            );
        }
        assert!(
            saw_fall && qualified,
            "entrance walk never reached the ledge predicate"
        );
        assert_eq!(both.player.pos.x, 104.0);
        assert!(edge(&both.world, &both.player, Side::Left, 1.0));
        assert!(edge(&both.world, &both.player, Side::Right, 1.0));
        assert!(!edge(&both.world, &both.player, Side::Top, 1.0));
        let y = both.player.pos.y;
        assert!(solid_point(&both.world, Vec2::new(113.0, y - 5.0)));
        assert!(solid_point(&both.world, Vec2::new(113.0, y - 9.0)));
        assert!(solid_point(&both.world, Vec2::new(95.0, y - 5.0)));
        assert!(!solid_point(&both.world, Vec2::new(95.0, y - 9.0)));
        assert!(!solid_point(&both.world, Vec2::new(104.0, y + 9.0)));
        both.step(&InputFrame {
            left: true,
            ..toward
        });
        right.step(&toward);
        quiet(&both);
        quiet(&right);
        assert_eq!(both.player.state, PlayerState::Hanging);
        assert_eq!(both.player.pos.y, 104.0);
        assert_eq!(both.player.vel.y, 0.0);
        assert_eq!(both.player.previous_pos, both.player.pos);
        assert_eq!(
            right.player.state,
            PlayerState::Falling,
            "right-only control must reject tall wall"
        );
        assert!(right.player.pos.y > y);
    }
}

fn ladder_room(rules: RulesPreset) -> Game {
    let mut level = crate::level::Level::default();
    cell(&mut level, 6, 9, '@'); // (104,152), above the ordinary floor.
    for y in 6..=8 {
        cell(&mut level, 6, y, 'L');
    }
    cell(&mut level, 7, 7, '1');
    for x in 1..=40 {
        cell(&mut level, x, 10, '1');
    }
    let mut game = authored(&level, rules);
    game.spawn("oLadderOrange", Vec2::new(96.0, 144.0));
    game.spawn("oPushBlock", Vec2::new(109.0, 96.0));
    game
}

#[test]
fn natural_ground_to_ladder_to_dynamic_solid_grab_can_jump_and_land_above() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut grab = ladder_room(rules);
        let mut climb = ladder_room(rules);
        for _ in 0..4 {
            grab.step(&InputFrame::default());
            climb.step(&InputFrame::default());
        }
        assert_eq!(grab.player.pos, Vec2::new(104.0, 152.0));
        assert!(grab.player.grounded);
        let up = InputFrame {
            up: true,
            ..Default::default()
        };
        let mut saw_climbing = false;
        let mut qualified = false;
        for _ in 0..180 {
            quiet(&grab);
            quiet(&climb);
            assert_eq!(grab.player.pos, climb.player.pos);
            saw_climbing |= grab.player.state == PlayerState::Climbing;
            if grab.player.state == PlayerState::Climbing
                && (101.0..=104.0).contains(&grab.player.pos.y)
            {
                qualified = true;
                break;
            }
            grab.step(&up);
            climb.step(&up);
        }
        assert!(
            saw_climbing && qualified,
            "ordinary Up never reached eligible ledge while climbing"
        );
        assert!(!grab.player.grounded);
        assert!(edge(&grab.world, &grab.player, Side::Right, 1.0));
        assert!(!edge(&grab.world, &grab.player, Side::Top, 1.0));
        let y = grab.player.pos.y;
        assert!(solid_point(&grab.world, Vec2::new(113.0, y - 5.0)));
        assert!(!solid_point(&grab.world, Vec2::new(113.0, y - 9.0)));
        assert!(!solid_point(&grab.world, Vec2::new(104.0, y + 9.0)));
        grab.step(&InputFrame { right: true, ..up });
        climb.step(&up);
        assert_eq!(grab.player.state, PlayerState::Hanging);
        assert_eq!(grab.player.pos.y, 104.0);
        assert_eq!(grab.player.previous_pos, grab.player.pos);
        assert_eq!(climb.player.state, PlayerState::Climbing);
        grab.step(&InputFrame {
            right: true,
            jump: true,
            jump_pressed: true,
            ..Default::default()
        });
        assert_eq!(
            grab.player.vel.y, -5.0,
            "detach includes the source side-support impulse"
        );
        let mut landed = false;
        for _ in 0..30 {
            quiet(&grab);
            grab.step(&InputFrame {
                right: true,
                jump: true,
                ..Default::default()
            });
            if grab.player.grounded && grab.player.pos.y == 88.0 {
                landed = true;
                break;
            }
        }
        assert!(
            landed,
            "ledge jump did not land above the original dynamic Solid"
        );
    }
}

fn arrow_fixture(rules: RulesPreset, case: &str) -> Game {
    let mut level = crate::level::Level::default();
    cell(&mut level, 6, 8, '@'); // Initial ordinary FALLING at (104,136).
    cell(&mut level, 7, 9, '1');
    for x in 1..=40 {
        cell(&mut level, x, 10, '1');
    }
    let mut game = authored(&level, rules);
    game.spawn("oPushBlock", Vec2::new(109.0, 128.0));
    let primary = game.spawn(
        "oArrow",
        Vec2::new(
            if case == "horizontal_miss" {
                111.0
            } else {
                105.0
            },
            131.0,
        ),
    );
    // Explicit controlled INITIAL embedded-arrow context. The separate Bow
    // regression below proves natural acquisition, emission and wall embedding.
    game.world.entity_mut(primary).unwrap().data.insert(
        "stuck".into(),
        if case == "tie_unstuck_first" {
            0.0
        } else {
            1.0
        },
    );
    match case {
        "upper_blocked" => {
            game.spawn("oArrow", Vec2::new(104.0, 127.0));
        }
        "lower_blocked" => {
            game.spawn("oArrow", Vec2::new(104.0, 145.0));
        }
        "nearest_unstuck" => {
            game.spawn("oArrow", Vec2::new(104.0, 131.25));
        }
        "tie_stuck_first" | "tie_unstuck_first" => {
            let second = game.spawn("oArrow", Vec2::new(103.0, 131.0));
            if case == "tie_unstuck_first" {
                game.world
                    .entity_mut(second)
                    .unwrap()
                    .data
                    .insert("stuck".into(), 1.0);
            }
        }
        "positive" | "horizontal_miss" => {}
        _ => panic!("unknown controlled fixture"),
    }
    game
}

#[test]
fn actual_step_arrow_exact_points_and_nearest_all_reject_tolerance_false_grabs() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for case in [
            "positive",
            "horizontal_miss",
            "upper_blocked",
            "lower_blocked",
            "nearest_unstuck",
            "tie_stuck_first",
            "tie_unstuck_first",
        ] {
            let mut game = arrow_fixture(rules, case);
            assert_eq!(game.player.pos, Vec2::new(104.0, 136.0));
            assert_eq!(game.player.state, PlayerState::Falling);
            assert!(!below(&game.world, &game.player));
            assert!(!edge(&game.world, &game.player, Side::Top, 1.0));
            assert!(edge(&game.world, &game.player, Side::Right, 2.0));
            let arrows: Vec<_> = game
                .world
                .entities
                .iter()
                .filter(|e| e.object == "oArrow")
                .collect();
            let point = |y| {
                arrows
                    .iter()
                    .any(|e| e.mask_contains(Vec2::new(104.0, y), CollisionPrecision::BoundingBox))
            };
            assert_eq!(point(131.0) || point(130.0), case != "horizontal_miss");
            assert_eq!(point(127.0), case == "upper_blocked");
            assert_eq!(point(145.0), case == "lower_blocked");
            if case == "nearest_unstuck" {
                assert_eq!(arrows[0].pos, Vec2::new(105.0, 131.0));
                assert_eq!(arrows[0].value("stuck"), 1.0);
                assert_eq!(arrows[1].pos, Vec2::new(104.0, 131.25));
                assert_eq!(arrows[1].value("stuck"), 0.0);
                assert!(
                    arrows[1].pos.distance(Vec2::new(104.0, 131.0))
                        < arrows[0].pos.distance(Vec2::new(104.0, 131.0))
                );
            }
            if case.starts_with("tie_") {
                assert_eq!(arrows[0].pos, Vec2::new(105.0, 131.0));
                assert_eq!(arrows[1].pos, Vec2::new(103.0, 131.0));
                assert_eq!(
                    arrows[0].pos.distance(Vec2::new(104.0, 131.0)),
                    arrows[1].pos.distance(Vec2::new(104.0, 131.0))
                );
                assert_eq!(
                    arrows[0].value("stuck"),
                    if case == "tie_stuck_first" { 1.0 } else { 0.0 }
                );
                assert_eq!(
                    arrows[1].value("stuck"),
                    if case == "tie_stuck_first" { 0.0 } else { 1.0 }
                );
            }
            game.step(&InputFrame::default());
            quiet(&game);
            if case == "positive" || case == "tie_stuck_first" {
                assert_eq!(game.player.state, PlayerState::Hanging);
                assert_eq!(
                    game.player.pos,
                    Vec2::new(104.0, 136.0),
                    "Arrow grabs never snap the player"
                );
                assert_eq!(game.player.data.get("hanging_arrow"), Some(&1.0));
            } else {
                assert_eq!(game.player.state, PlayerState::Falling, "{rules:?}/{case}");
                assert_eq!(
                    game.player.pos,
                    Vec2::new(104.0, 137.0),
                    "false Arrow contact must preserve ordinary gravity: {case}"
                );
                assert_eq!(game.player.data.get("hang_count"), Some(&0.0));
            }
        }
    }
}

fn bow_room(rules: RulesPreset) -> Game {
    let mut level = crate::level::Level::default();
    cell(&mut level, 8, 8, '@'); // (136,136), on the platform ending at x143.
    for x in 7..=8 {
        cell(&mut level, x, 9, '1');
    }
    for x in 1..=40 {
        cell(&mut level, x, 12, '1');
    }
    for y in 5..=11 {
        cell(&mut level, 10, y, '1');
    }
    let mut game = authored(&level, rules);
    game.spawn("oBow", Vec2::new(136.0, 140.0));
    game
}

#[test]
fn natural_bow_pickup_charge_embedding_and_walk_fall_arrow_grab_has_no_snap() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut fired = bow_room(rules);
        let mut held = bow_room(rules);
        for _ in 0..4 {
            fired.step(&InputFrame::default());
            held.step(&InputFrame::default());
        }
        assert!(fired.player.grounded);
        let pickup = InputFrame {
            down: true,
            attack: true,
            attack_pressed: true,
            ..Default::default()
        };
        fired.step(&pickup);
        held.step(&pickup);
        assert!(
            fired
                .player
                .held
                .and_then(|id| fired.world.entity(id))
                .is_some_and(|e| e.object == "oBow")
        );
        assert_eq!(fired.data.get("arrows"), Some(&6.0));
        fired.step(&InputFrame::default());
        held.step(&InputFrame::default());
        let charge = InputFrame {
            attack: true,
            attack_pressed: true,
            ..Default::default()
        };
        fired.step(&charge);
        held.step(&charge);
        for _ in 0..60 {
            let charge = InputFrame {
                attack: true,
                ..Default::default()
            };
            fired.step(&charge);
            held.step(&charge);
        }
        assert_eq!(fired.player.data.get("bow_strength"), Some(&12.0));
        fired.step(&InputFrame {
            attack_released: true,
            ..Default::default()
        });
        held.step(&InputFrame {
            attack: true,
            ..Default::default()
        });
        let arrow = fired
            .world
            .entities
            .iter()
            .find(|e| e.alive && e.object == "oArrow")
            .expect("Bow must naturally emit a real Arrow")
            .id;
        let mut embedded = false;
        for _ in 0..8 {
            let e = fired.world.entity(arrow).expect("emitted Arrow survived");
            if e.value("stuck") != 0.0 && e.vel == Vec2::default() {
                embedded = true;
                break;
            }
            fired.step(&InputFrame::default());
            held.step(&InputFrame {
                attack: true,
                ..Default::default()
            });
        }
        assert!(
            embedded,
            "fully charged Arrow never naturally embedded in the wall"
        );
        assert!(
            !held
                .world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oArrow")
        );
        assert_eq!(fired.data.get("arrows"), Some(&5.0));
        let mut saw_fall = false;
        let mut grabbed = false;
        for _ in 0..40 {
            quiet(&fired);
            quiet(&held);
            saw_fall |= fired.player.state == PlayerState::Falling && !fired.player.grounded;
            fired.step(&InputFrame {
                right: true,
                ..Default::default()
            });
            held.step(&InputFrame {
                right: true,
                attack: true,
                ..Default::default()
            });
            if fired.player.state == PlayerState::Hanging {
                grabbed = true;
                break;
            }
        }
        assert!(
            saw_fall && grabbed,
            "ordinary platform walk/fall never grabbed the emitted Arrow"
        );
        assert_eq!(fired.player.data.get("hanging_arrow"), Some(&1.0));
        assert_ne!(
            fired.player.pos.y % 8.0,
            0.0,
            "Arrow grab must retain its unsnapped height"
        );
        assert_eq!(
            held.player.state,
            PlayerState::Falling,
            "unchanged charged-Bow control has no Arrow support"
        );
        assert!(held.player.pos.y > fired.player.pos.y);
        quiet(&fired);
        quiet(&held);
    }
}
