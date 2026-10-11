//! Source Player End Step contracts exercised through actual authored-room input.
use spelunky_core::*;

fn ledge_room(rules: RulesPreset, character: characters::Character, yeti: bool) -> Game {
    let mut level = level::Level::default();
    level.set_tile(13, 18, '@').unwrap();
    for x in 3..=13 {
        level.set_tile(x, 19, '1').unwrap();
    }
    for x in 0..40 {
        level.set_tile(x, 22, '1').unwrap();
    }
    let mut game = Game::shell(42, rules);
    game.load_custom(&level, "interrupted-ledge", "");
    characters::apply(&mut game, character);
    if yeti {
        game.spawn("oYeti", Vec2::new(197.0, 200.0));
    }
    game
}

fn duck_to_hang(game: &Game) -> bool {
    matches!(
        game.player.sprite.as_str(),
        "sDuckToHangL" | "sDamselDtHL" | "sTunnelDtHL"
    )
}

fn begin_crawl(game: &mut Game) {
    for _ in 0..4 {
        game.step(&InputFrame::default());
    }
    assert!(game.player.grounded);
    for _ in 0..80 {
        game.step(&InputFrame {
            down: true,
            right: true,
            ..Default::default()
        });
        if game.player.data.get("duck_to_hang") == Some(&1.0) {
            assert!(duck_to_hang(game));
            return;
        }
        assert!(
            !game.player.is_stunned(),
            "initial actor hit before the ledge transition"
        );
    }
    panic!("natural crawl never entered DuckToHang");
}

#[test]
fn uninterrupted_natural_ledge_transition_keeps_its_animation_end() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for character in [
            characters::Character::Explorer,
            characters::Character::Damsel,
            characters::Character::TunnelMan,
        ] {
            let mut game = ledge_room(rules, character, false);
            begin_crawl(&mut game);
            let start = game.player.pos;
            let mut finished = false;
            for _ in 0..20 {
                game.step(&InputFrame::default());
                if game.player.state == PlayerState::Hanging {
                    finished = true;
                    break;
                }
                assert_eq!(game.player.data.get("duck_to_hang"), Some(&1.0));
                assert!(duck_to_hang(&game));
                assert_eq!(game.player.pos, start);
            }
            assert!(finished);
            assert_eq!(game.player.pos, Vec2::new(start.x + 6.0, start.y + 16.0));
            assert_eq!(game.player.data.get("duck_to_hang"), Some(&0.0));
        }
    }
}

#[test]
fn initial_yeti_throw_interrupts_natural_ledge_animation_and_player_recovers() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for character in [
            characters::Character::Explorer,
            characters::Character::Damsel,
            characters::Character::TunnelMan,
        ] {
            let mut game = ledge_room(rules, character, true);
            let health = game.player.health;
            begin_crawl(&mut game);
            let mut thrown_during_transition = false;
            for _ in 0..20 {
                game.step(&InputFrame::default());
                if game.player.is_stunned() {
                    assert!(duck_to_hang(&game));
                    assert_eq!(game.player.data.get("duck_to_hang"), Some(&1.0));
                    thrown_during_transition = true;
                    break;
                }
                assert!(
                    duck_to_hang(&game),
                    "Yeti arrived after the ledge animation"
                );
                assert_eq!(game.player.data.get("duck_to_hang"), Some(&1.0));
            }
            assert!(
                thrown_during_transition,
                "initial Yeti must throw during the live ledge animation"
            );
            // Collision assigned only the independent stun flag. The following
            // Player Step changes the sprite; its own End Step must repair the
            // pending platform state before that interrupted animation can end.
            game.step(&InputFrame::default());
            assert!(!duck_to_hang(&game));
            assert_eq!(game.player.health, health);
            assert_eq!(
                game.player.stun, 0,
                "source Yeti throw assigns no stun timer"
            );
            assert_eq!(game.player.data.get("duck_to_hang"), Some(&0.0));
            assert!(
                game.player.is_stunned(),
                "End Step must preserve the independent stun flag"
            );
            let mut jumped = false;
            for tick in 0..160 {
                let jump = tick % 2 == 0;
                game.step(&InputFrame {
                    right: true,
                    jump,
                    jump_pressed: jump,
                    jump_released: !jump,
                    ..Default::default()
                });
                if !game.player.is_stunned()
                    && game.player.state == PlayerState::Jumping
                    && game.player.vel.y < 0.0
                {
                    jumped = true;
                    break;
                }
                assert_eq!(game.state, RunState::Playing);
            }
            assert!(
                jumped,
                "recovered player must accept a real jump after interruption"
            );
        }
    }
}

#[test]
fn natural_equipment_pickup_and_held_rock_receive_end_step_ladder_depth() {
    for (pickup_object, equipment) in [("oCapePickup", "Cape"), ("oJetpack", "Jetpack")] {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            let mut level = level::Level::default();
            level.set_tile(4, 8, '@').unwrap();
            for y in 4..8 {
                level.set_tile(4, y, 'L').unwrap();
            }
            for x in 0..40 {
                level.set_tile(x, 9, '1').unwrap();
            }
            let mut game = Game::shell(7, rules);
            game.load_custom(&level, "held-climbing-depth", "");
            let start = game.player.pos;
            // The entrance occupies its serialized cell; author its overlapping
            // ladder segment as another initial room actor.
            game.spawn("oLadderOrange", Vec2::new(start.x - 8.0, start.y - 8.0));
            game.spawn(pickup_object, Vec2::new(start.x, start.y + 2.0));
            let rock = game.spawn("oRock", Vec2::new(start.x + 8.0, start.y + 2.0));
            for _ in 0..4 {
                game.step(&InputFrame::default());
            }
            let pickup = InputFrame {
                down: true,
                attack_pressed: true,
                ..Default::default()
            };
            game.step(&pickup);
            assert!(game.player.has(equipment));
            game.step(&InputFrame::default());
            game.step(&pickup);
            assert_eq!(game.player.held, Some(rock));
            assert_eq!(game.world.entity(rock).unwrap().value("depth"), 0.0);
            for _ in 0..8 {
                game.step(&InputFrame {
                    up: true,
                    ..Default::default()
                });
                if game.player.state == PlayerState::Climbing {
                    break;
                }
            }
            assert_eq!(game.player.state, PlayerState::Climbing);
            assert_eq!(game.player.held, Some(rock));
            assert_eq!(game.world.entity(rock).unwrap().value("depth"), 51.0);
            assert_eq!(
                game.presentation()
                    .world
                    .entity(rock)
                    .unwrap()
                    .value("depth"),
                51.0
            );
            game.step(&InputFrame {
                jump: true,
                jump_pressed: true,
                right: true,
                ..Default::default()
            });
            assert_ne!(game.player.state, PlayerState::Climbing);
            assert_eq!(game.player.held, Some(rock));
            assert_eq!(game.world.entity(rock).unwrap().value("depth"), 0.0);
        }
    }
}
