use spelunky_core::{Game, InputFrame, RulesPreset, RunState, Vec2, level::Level};

fn room(canonical: bool, rules: RulesPreset) -> Game {
    let mut level = Level::default();
    level.starting_bombs = 0;
    level.starting_ropes = 0;
    level.set_tile(6, 7, '@').unwrap();
    for x in 0..spelunky_core::level::LEVEL_WIDTH {
        level.set_tile(x, 8, '1').unwrap();
    }
    let mut game = if canonical {
        Game::shell(42, rules)
    } else {
        Game::legacy_shell(42, rules)
    };
    game.load_custom(&level, "monkey-depth", "");
    for _ in 0..8 {
        game.step(&InputFrame::default());
    }
    assert!(game.player.grounded);
    game
}

#[test]
fn fresh_monkey_step_replaces_inherited_depth_and_grab_uses_its_own_layer() {
    for canonical in [false, true] {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            let mut game = room(canonical, rules);
            let id = game.spawn(
                "oMonkey",
                Vec2::new(game.player.pos.x - 8.0, game.player.pos.y - 8.0),
            );
            assert_eq!(game.world.entity(id).unwrap().state, 5);
            game.step(&InputFrame::default());
            let monkey = game.world.entity(id).unwrap();
            assert_eq!(monkey.state, 7, "ordinary collision must acquire GRAB");
            assert!((40..=80).contains(&monkey.timer));
            assert_eq!(game.player.health, 4);
            assert_eq!(monkey.value("depth"), 40.0, "MONKEY_OWN_STEP_DEPTH");
            let mut released = false;
            for _ in 0..82 {
                let incoming = game.world.entity(id).unwrap().state;
                game.step(&InputFrame::default());
                assert_eq!(game.state, RunState::Playing);
                let monkey = game.world.entity(id).unwrap();
                assert_eq!(
                    monkey.value("depth"),
                    if incoming == 7 { 120.0 } else { 40.0 },
                    "MONKEY_SELECTED_ARM_DEPTH"
                );
                if incoming == 7 && monkey.state != 7 {
                    assert_eq!(
                        monkey.value("depth"),
                        120.0,
                        "release retains GRAB arm depth for this frame"
                    );
                    game.step(&InputFrame::default());
                    assert_eq!(
                        game.world.entity(id).unwrap().value("depth"),
                        40.0,
                        "next ordinary Step restores monkey depth"
                    );
                    released = true;
                    break;
                }
            }
            assert!(
                released,
                "natural countdown must release monkey without actor writes"
            );
        }
    }
}

#[test]
fn unrelated_enemy_and_offscreen_monkey_keep_inherited_depth_behavior() {
    for canonical in [false, true] {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            let mut game = room(canonical, rules);
            let frog = game.spawn("oFrog", Vec2::new(240.0, 128.0));
            let offscreen = game.spawn("oMonkey", Vec2::new(500.0, 112.0));
            let before = game
                .world
                .entity(offscreen)
                .unwrap()
                .data
                .get("depth")
                .copied();
            game.step(&InputFrame::default());
            assert_eq!(game.world.entity(frog).unwrap().value("depth"), 60.0);
            assert_eq!(
                game.world
                    .entity(offscreen)
                    .unwrap()
                    .data
                    .get("depth")
                    .copied(),
                before
            );
            assert_eq!(game.world.entity(offscreen).unwrap().state, 5);
        }
    }
}
