//! Authored initial geometry; Rope use and burning proceed only via Game::step.
//! Source RopeThrow inherits lava Destroy without exiting; RopeBurn does likewise.
use spelunky_core::{Area, Game, GameEvent, InputFrame, PlayerState, RulesPreset, Vec2, World};

fn room(rules: RulesPreset, x: f64) -> Game {
    room_with_floor_hole(rules, x, None)
}

fn room_with_floor_hole(rules: RulesPreset, x: f64, hole: Option<usize>) -> Game {
    let mut game = Game::shell(7, rules);
    game.world = World::new(24, 20, Area::Mines, 1);
    game.world.name = "rLevel".into();
    for column in 0..24 {
        if Some(column) != hole {
            game.world.set_tile(column, 10, "oHardBlock");
        }
    }
    game.player.pos = Vec2::new(x, 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.world.entrance = game.player.pos;
    game
}

fn quiet_step(game: &mut Game, input: InputFrame) {
    let rng = serde_json::to_value(&game.rng).unwrap();
    game.step(&input);
    assert_eq!(serde_json::to_value(&game.rng).unwrap(), rng);
    assert_eq!(game.player.health, 4);
}

fn live(game: &Game, object: &str) -> usize {
    game.world
        .entities
        .iter()
        .filter(|entity| entity.alive && entity.object == object)
        .count()
}

fn reserve_rope(game: &mut Game) -> u64 {
    quiet_step(
        game,
        InputFrame {
            cycle_pressed: true,
            ..Default::default()
        },
    );
    assert_eq!(
        game.world.entity(game.player.held.unwrap()).unwrap().object,
        "oBomb"
    );
    quiet_step(game, InputFrame::default());
    quiet_step(
        game,
        InputFrame {
            cycle_pressed: true,
            ..Default::default()
        },
    );
    let rope = game.player.held.unwrap();
    assert_eq!(game.world.entity(rope).unwrap().object, "oRopeThrow");
    assert!(game.world.entity(rope).unwrap().held);
    assert_eq!((game.player.bombs, game.player.ropes), (4, 3));
    quiet_step(game, InputFrame::default());
    rope
}

#[test]
fn natural_held_up_rope_lava_retirement_still_deploys_once() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for lava in [false, true] {
            let mut game = room(rules, 80.0);
            if lava {
                // Overhead Lava intersects the thrown rope, not Player's y+6.
                game.spawn("oLava", Vec2::new(80.0, 128.0));
            }
            let reserved = reserve_rope(&mut game);
            let held_origin = game.world.entity(reserved).unwrap().pos;
            assert_eq!(held_origin, Vec2::new(84.0, 154.0));
            let next_id = game.world.next_id;
            quiet_step(
                &mut game,
                InputFrame {
                    attack: true,
                    attack_pressed: true,
                    ..Default::default()
                },
            );
            assert!(game.player.held.is_none());
            assert_eq!((game.player.bombs, game.player.ropes), (4, 3));
            assert_eq!(
                game.events
                    .iter()
                    .filter(
                        |event| matches!(event, GameEvent::Sound { name, .. } if name == "sndThrow")
                    )
                    .count(),
                1
            );
            if lava {
                assert!(
                    game.world.entity(reserved).is_none(),
                    "inherited lava Destroy remains final"
                );
                assert_eq!(
                    (
                        live(&game, "oRopeThrow"),
                        live(&game, "oRopeTop"),
                        live(&game, "oRope")
                    ),
                    (0, 1, 1)
                );
                // Source moveTo(-12), sink .05, then move_snap(16,1).
                // px==80 picks +8; falling immediately extends another eight.
                let top = game.world.entity(next_id).unwrap();
                assert_eq!(top.object, "oRopeTop");
                assert_eq!(top.previous_pos, Vec2::new(88.0, 142.0));
                let segment = game.world.entity(next_id + 1).unwrap();
                assert_eq!(segment.object, "oRope");
                assert_eq!(segment.previous_pos, Vec2::new(80.0, 150.0));
                assert_eq!(game.world.next_id, next_id + 2);
                quiet_step(&mut game, InputFrame::default());
                assert!(game.world.entity(reserved).is_none());
                assert_eq!(
                    (live(&game, "oRopeTop"), live(&game, "oRope")),
                    (1, 1),
                    "retired receiver gets no second Step"
                );
            } else {
                assert!(game.world.entity(reserved).is_some());
                assert_eq!(
                    (live(&game, "oRopeTop"), live(&game, "oRope")),
                    (0, 0),
                    "live upward flight has not reached its apex"
                );
                assert_eq!(game.world.next_id, next_id);
            }
        }
    }
}

#[test]
fn natural_down_rope_lava_retirement_still_extends_existing_anchor() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for lava in [false, true] {
            // Author Player's tile4 support and adjacent gap from the start.
            // Removing a Solid afterward requests the source fluid drain pass.
            let mut game = room_with_floor_hole(rules, 72.0, Some(5));
            if lava {
                game.spawn("oLava", Vec2::new(80.0, 144.0));
            }
            quiet_step(&mut game, InputFrame::default());
            let attempt_id = game.world.next_id;
            quiet_step(
                &mut game,
                InputFrame {
                    down: true,
                    rope_pressed: true,
                    ..Default::default()
                },
            );
            assert_eq!((game.player.bombs, game.player.ropes), (4, 3));
            assert_eq!((live(&game, "oRopeTop"), live(&game, "oRope")), (1, 1));
            let top = game.world.entity(attempt_id + 1).unwrap();
            assert_eq!(top.object, "oRopeTop");
            assert_eq!(top.previous_pos, Vec2::new(88.0, 152.0));
            let segment = game.world.entity(attempt_id + 2).unwrap();
            assert_eq!(segment.object, "oRope");
            assert_eq!(segment.previous_pos.x, 80.0);
            assert!(
                (segment.previous_pos.y - (160.0 + if lava { 0.05 } else { 0.0 })).abs() < 1e-9
            );
            assert_eq!(game.world.next_id, attempt_id + 3);
            if lava {
                assert!(game.world.entity(attempt_id).is_none());
                quiet_step(&mut game, InputFrame::default());
                assert_eq!((live(&game, "oRopeTop"), live(&game, "oRope")), (1, 1));
                assert!(game.world.entity(attempt_id).is_none());
            } else {
                let throwing = game.world.entity(attempt_id).unwrap();
                assert_eq!(throwing.value("falling"), 1.0);
                assert_eq!(throwing.value("fall_count"), 1.0);
            }
        }
    }
}

#[test]
fn natural_lava_burn_marks_nearest_rope_even_after_solid_retirement() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for solid in [false, true] {
            let mut game = room(rules, 40.0);
            // Initial authored climbable segments. No authored flame, timer,
            // velocity or callback phase: the ordinary Rope Step births fire.
            let touching = game.spawn("oRope", Vec2::new(80.0, 96.0));
            let nearest = game.spawn("oRope", Vec2::new(85.0, 95.0));
            game.spawn("oLava", Vec2::new(88.0, 96.0));
            if solid {
                game.spawn("oHardBlock", Vec2::new(88.0, 88.0));
            }
            let flame_id = game.world.next_id;
            quiet_step(&mut game, InputFrame::default());
            assert_eq!(
                game.world.next_id,
                flame_id + 1,
                "one fire born from the touching segment"
            );
            // sRope bbox is [x+6,x+9,y-4,y+3]. Flame moves to (88,95):
            // touching contains it, nearest does not, but has the nearer origin.
            assert_eq!(
                game.world.entity(touching).unwrap().value("burn_timer"),
                0.0
            );
            assert_eq!(game.world.entity(nearest).unwrap().value("burn_timer"), 1.0);
            if solid {
                assert!(
                    game.world.entity(flame_id).is_none(),
                    "Solid still destroys the flame"
                );
                assert_eq!(live(&game, "oRopeBurn"), 0);
            } else {
                let flame = game.world.entity(flame_id).unwrap();
                assert_eq!(flame.object, "oRopeBurn");
                assert_eq!(flame.previous_pos, Vec2::new(88.0, 96.0));
                assert_eq!(flame.pos, Vec2::new(88.0, 95.0));
            }
            quiet_step(&mut game, InputFrame::default());
            assert!(
                game.world.entity(nearest).is_none(),
                "source timer1 removes the selected rope next Step"
            );
            if solid {
                assert!(game.world.entity(flame_id).is_none(), "no resurrection");
            }
        }
    }
}

#[test]
fn natural_lava_burn_solid_retirement_still_marks_nearest_rope() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = room(rules, 40.0);
        // Initial authored geometry; ordinary neutral Steps birth and retire
        // the flame. This separate test always reaches the Solid-only route.
        let touching = game.spawn("oRope", Vec2::new(80.0, 96.0));
        let nearest = game.spawn("oRope", Vec2::new(85.0, 95.0));
        game.spawn("oLava", Vec2::new(88.0, 96.0));
        game.spawn("oHardBlock", Vec2::new(88.0, 88.0));
        let flame_id = game.world.next_id;
        quiet_step(&mut game, InputFrame::default());
        assert_eq!(
            game.world.next_id,
            flame_id + 1,
            "one fire born from the touching segment"
        );
        assert!(
            game.world.entity(flame_id).is_none(),
            "Solid still destroys the flame"
        );
        assert_eq!(live(&game, "oRopeBurn"), 0);
        // At (88,95), only touching contains the point, but nearest has
        // the closer origin. The independent query follows Solid Destroy.
        assert_eq!(
            game.world.entity(touching).unwrap().value("burn_timer"),
            0.0
        );
        assert_eq!(game.world.entity(nearest).unwrap().value("burn_timer"), 1.0);
        quiet_step(&mut game, InputFrame::default());
        assert!(
            game.world.entity(nearest).is_none(),
            "source timer1 removes the selected rope next Step"
        );
        assert!(game.world.entity(flame_id).is_none(), "no resurrection");
    }
}
