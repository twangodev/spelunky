//! Classic scrUseItem37-120 and oPlayer1 Step1087-1173 Rope lifecycle.
//! Only authored initial geometry/inventory; acquisition and actions use Game::step.
use spelunky_core::{Area, Game, GameEvent, InputFrame, PlayerState, RulesPreset, Vec2, World};

fn cases() -> [(RulesPreset, i8); 4] {
    [
        (RulesPreset::Classic, -1),
        (RulesPreset::Classic, 1),
        (RulesPreset::Assisted, -1),
        (RulesPreset::Assisted, 1),
    ]
}

fn room(rules: RulesPreset, facing: i8) -> Game {
    let mut game = Game::shell(42, rules);
    game.world = World::new(30, 20, Area::Mines, 1);
    game.world.name = "rLevel".into();
    for x in 0..game.world.width {
        game.world.set_tile(x, 10, "oBrickSmooth");
    }
    game.player.pos = Vec2::new(152.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.player.facing = facing;
    game.player.bombs = 4.0;
    game.player.ropes = 4.0;
    game.world.entrance = game.player.pos;
    game
}

fn live(game: &Game, object: &str) -> usize {
    game.world
        .entities
        .iter()
        .filter(|entity| entity.alive && entity.object == object)
        .count()
}

fn throw_sounds(game: &Game) -> usize {
    game.events
        .iter()
        .filter(|event| matches!(event, GameEvent::Sound { name, .. } if name == "sndThrow"))
        .count()
}

fn quiet_step(game: &mut Game, input: InputFrame) {
    let mut expected_rng = game.rng.clone();
    game.step(&input);
    let mut actual_rng = game.rng.clone();
    assert_eq!(
        actual_rng.next_u64(),
        expected_rng.next_u64(),
        "Rope/Bomb reservation and Rope Create consume no RNG in this quiet room"
    );
}

fn reserve_rope(game: &mut Game) -> u64 {
    quiet_step(
        game,
        InputFrame {
            cycle_pressed: true,
            ..Default::default()
        },
    );
    let bomb = game.player.held.expect("empty/Rock -> reserved Bomb");
    let entity = game.world.entity(bomb).unwrap();
    assert_eq!(entity.object, "oBomb");
    assert!(entity.held);
    assert_eq!(entity.value("armed"), 0.0);
    assert_eq!((game.player.bombs, game.player.ropes), (3.0, 4.0));
    quiet_step(game, InputFrame::default());
    quiet_step(
        game,
        InputFrame {
            cycle_pressed: true,
            ..Default::default()
        },
    );
    assert!(game.world.entity(bomb).is_none());
    let rope = game.player.held.expect("refund Bomb -> reserve Rope");
    assert_ne!(rope, bomb);
    let entity = game.world.entity(rope).unwrap();
    assert_eq!(entity.object, "oRopeThrow");
    assert!(entity.held);
    assert_eq!(entity.value("armed"), 0.0);
    assert_eq!((game.player.bombs, game.player.ropes), (4.0, 3.0));
    quiet_step(game, InputFrame::default());
    rope
}

fn held_use(down: bool) -> InputFrame {
    InputFrame {
        down,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    }
}

fn dedicated_use(down: bool) -> InputFrame {
    InputFrame {
        down,
        rope_pressed: true,
        ..Default::default()
    }
}

fn open_ledge(game: &mut Game, facing: i8) {
    // Keep tile9 supporting the player. The adjacent strip can receive RopeTop.
    game.world.set_tile(if facing < 0 { 8 } else { 10 }, 10, "");
}

fn source_fresh_rope_defaults(game: &Game, id: u64) {
    let rope = game
        .world
        .entity(id)
        .expect("fresh Rope actor survives this frame");
    assert_eq!(rope.object, "oRopeThrow");
    assert!(!rope.held);
    assert_eq!(rope.value("px"), 0.0);
    assert_eq!(rope.value("py"), 0.0);
    assert_eq!(rope.value("safe"), 0.0);
    assert_eq!(rope.value("cost"), 0.0);
    assert_eq!(rope.alarms, [-1; 12]);
}

#[test]
fn natural_two_edge_cycle_reserves_rope_and_refunds_bomb_without_rng() {
    for (rules, facing) in cases() {
        let mut game = room(rules, facing);
        let rope = reserve_rope(&mut game);
        assert_eq!(game.player.held, Some(rope));
        assert!(game.player.pickup_item_type.is_empty());
        assert_eq!((live(&game, "oBomb"), live(&game, "oRopeThrow")), (0, 1));
        assert_eq!(throw_sounds(&game), 0);
    }
}

#[test]
fn natural_held_up_launch_keeps_the_reserved_actor_and_origin_without_second_charge() {
    for (rules, facing) in cases() {
        let mut game = room(rules, facing);
        let reserved = reserve_rope(&mut game);
        let origin = game.player.pos;
        let next_id = game.world.next_id;
        quiet_step(&mut game, held_use(false));
        assert!(game.player.held.is_none());
        assert_eq!((game.player.bombs, game.player.ropes), (4.0, 3.0));
        assert_eq!(game.world.next_id, next_id, "up launches the SAME actor");
        let rope = game
            .world
            .entity(reserved)
            .expect("source retains reserved ID");
        assert!(!rope.held);
        assert_eq!(rope.value("armed"), 1.0);
        assert_eq!(rope.value("falling"), 0.0);
        assert_eq!(rope.value("px"), origin.x);
        assert_eq!(rope.value("py"), origin.y);
        assert!(
            rope.vel.y < 0.0,
            "later Rope Step may advance launch motion"
        );
        assert_eq!(live(&game, "oRopeThrow"), 1);
        assert_eq!(throw_sounds(&game), 1);
    }
}

#[test]
fn natural_held_down_blocked_retires_old_and_attempt_before_fresh_fallback_survives() {
    for (rules, facing) in cases() {
        let mut game = room(rules, facing);
        let reserved = reserve_rope(&mut game);
        let attempt = game.world.next_id;
        quiet_step(&mut game, held_use(true));
        assert!(
            game.world.entity(reserved).is_none(),
            "old reserved actor is destroyed"
        );
        assert!(
            game.world.entity(attempt).is_none(),
            "attempt is created then destroyed"
        );
        assert_eq!(
            game.world.next_id,
            attempt + 2,
            "attempt then fresh fallback Create"
        );
        let fresh = attempt + 1;
        assert_ne!(fresh, reserved);
        source_fresh_rope_defaults(&game, fresh);
        let rope = game.world.entity(fresh).unwrap();
        assert_eq!(rope.value("armed"), 0.0);
        assert_eq!(rope.value("falling"), 0.0);
        assert_eq!(rope.value("fall_count"), 0.0);
        // The later inherited motion may apply ground friction; preserve direction.
        assert!(rope.vel.x * f64::from(facing) > 0.0);
        assert_eq!(live(&game, "oRopeThrow"), 1);
        assert_eq!(live(&game, "oRopeTop"), 0);
        assert!(game.player.held.is_none());
        assert_eq!((game.player.bombs, game.player.ropes), (4.0, 3.0));
        assert_eq!(throw_sounds(&game), 1);
    }
}

#[test]
fn natural_held_down_clear_creates_fresh_attempt_then_top_without_second_charge() {
    for (rules, facing) in cases() {
        let mut game = room(rules, facing);
        let reserved = reserve_rope(&mut game);
        open_ledge(&mut game, facing);
        let attempt = game.world.next_id;
        quiet_step(&mut game, held_use(true));
        assert!(game.world.entity(reserved).is_none());
        source_fresh_rope_defaults(&game, attempt);
        let rope = game.world.entity(attempt).unwrap();
        assert_eq!(rope.value("armed"), 0.0);
        assert_eq!(rope.value("falling"), 1.0);
        assert_eq!(game.world.entity(attempt + 1).unwrap().object, "oRopeTop");
        // Rope469 can extend once later in this same NormalStep; do not fix a
        // pre-Step position, velocity or actor count as the full-frame oracle.
        assert!(game.world.next_id >= attempt + 2);
        assert_eq!(live(&game, "oRopeThrow"), 1);
        assert_eq!(live(&game, "oRopeTop"), 1);
        assert!(game.player.held.is_none());
        assert_eq!((game.player.bombs, game.player.ropes), (4.0, 3.0));
        assert_eq!(throw_sounds(&game), 1);
    }
}

#[test]
fn naturally_picked_rock_is_recreated_once_after_each_held_rope_use() {
    for (rules, facing) in cases() {
        for (down, clear) in [(false, false), (true, false), (true, true)] {
            let mut game = room(rules, facing);
            let old_rock = game.spawn("oRock", game.player.pos);
            quiet_step(&mut game, held_use(true));
            assert_eq!(
                game.player.held,
                Some(old_rock),
                "ordinary Down+Attack pickup"
            );
            quiet_step(&mut game, InputFrame::default());
            let reserved = reserve_rope(&mut game);
            assert!(game.world.entity(old_rock).is_none());
            assert_eq!(game.player.pickup_item_type, "Rock");
            if clear {
                open_ledge(&mut game, facing);
            }
            let first_new = game.world.next_id;
            quiet_step(&mut game, held_use(down));
            let fresh_rock = game
                .player
                .held
                .expect("scrHoldItem restores remembered Rock");
            assert_ne!(fresh_rock, old_rock);
            assert_eq!(fresh_rock, if down { first_new + 2 } else { first_new });
            let rock = game.world.entity(fresh_rock).unwrap();
            assert_eq!(rock.object, "oRock");
            assert!(rock.held);
            assert_eq!(rock.value("new"), 0.0);
            assert_eq!(rock.value("cost"), 0.0);
            assert_eq!(game.player.pickup_item_type, "Rock");
            assert_eq!(live(&game, "oRock"), 1);
            assert_eq!(game.world.entity(reserved).is_some(), !down);
            assert_eq!((game.player.bombs, game.player.ropes), (4.0, 3.0));
            assert_eq!(throw_sounds(&game), 1);
        }
    }
}

#[test]
fn settled_ceiling_keeps_held_rope_upward_but_allows_downward_use() {
    for (rules, facing) in cases() {
        let mut game = room(rules, facing);
        game.world.set_tile(9, 8, "oBrickSmooth");
        let reserved = reserve_rope(&mut game);
        let next_id = game.world.next_id;
        quiet_step(&mut game, held_use(false));
        assert_eq!(game.player.held, Some(reserved));
        assert_eq!(game.world.next_id, next_id);
        assert!(game.world.entity(reserved).unwrap().held);
        assert_eq!(game.world.entity(reserved).unwrap().value("armed"), 0.0);
        assert_eq!((game.player.bombs, game.player.ropes), (4.0, 3.0));
        assert_eq!(throw_sounds(&game), 0);
        quiet_step(&mut game, InputFrame::default());
        let attempt = game.world.next_id;
        quiet_step(&mut game, held_use(true));
        assert!(game.world.entity(reserved).is_none());
        assert!(game.world.entity(attempt).is_none());
        assert_eq!(game.world.next_id, attempt + 2);
        source_fresh_rope_defaults(&game, attempt + 1);
        assert!(game.player.held.is_none());
        assert_eq!((game.player.bombs, game.player.ropes), (4.0, 3.0));
        assert_eq!(throw_sounds(&game), 1);
    }
}

#[test]
fn dedicated_rope_creates_fresh_up_and_down_but_blocked_down_only_consumes_attempt_id() {
    for (rules, facing) in cases() {
        for (down, clear, ceiling) in [
            (false, false, false),
            (true, true, false),
            (true, false, false),
            (false, false, true),
        ] {
            let mut game = room(rules, facing);
            if clear {
                open_ledge(&mut game, facing);
            }
            if ceiling {
                game.world.set_tile(9, 8, "oBrickSmooth");
            }
            let origin = game.player.pos;
            let attempt = game.world.next_id;
            quiet_step(&mut game, dedicated_use(down));
            assert!(game.player.held.is_none());
            assert_eq!(game.player.bombs, 4.0);
            if ceiling {
                assert_eq!(game.world.next_id, attempt);
                assert_eq!(game.player.ropes, 4.0);
                assert_eq!(live(&game, "oRopeThrow"), 0);
                assert_eq!(throw_sounds(&game), 0);
            } else if down && !clear {
                assert_eq!(game.world.next_id, attempt + 1);
                assert!(game.world.entity(attempt).is_none());
                assert_eq!(game.player.ropes, 4.0);
                assert_eq!(live(&game, "oRopeThrow"), 0);
                assert_eq!(live(&game, "oRopeTop"), 0);
                assert_eq!(throw_sounds(&game), 0);
            } else if down {
                source_fresh_rope_defaults(&game, attempt);
                assert_eq!(game.world.entity(attempt).unwrap().value("falling"), 1.0);
                assert_eq!(game.world.entity(attempt + 1).unwrap().object, "oRopeTop");
                assert_eq!(game.player.ropes, 3.0);
                assert_eq!(throw_sounds(&game), 1);
            } else {
                assert_eq!(game.world.next_id, attempt + 1);
                let rope = game.world.entity(attempt).unwrap();
                assert!(!rope.held);
                assert_eq!(rope.value("armed"), 1.0);
                assert_eq!(rope.value("falling"), 0.0);
                assert_eq!(rope.value("px"), origin.x);
                assert_eq!(rope.value("py"), origin.y);
                assert_eq!(game.player.ropes, 3.0);
                assert_eq!(throw_sounds(&game), 1);
            }
        }
    }
}

#[test]
fn dedicated_up_uses_cached_ceiling_contact_when_player_crosses_either_edge() {
    for (rules, facing) in cases() {
        for leaving in [true, false] {
            let mut game = room(rules, facing);
            let start_x = match (facing, leaving) {
                (1, true) => 163.0,
                (-1, true) => 140.0,
                (1, false) => 138.0,
                (-1, false) => 165.0,
                _ => unreachable!(),
            };
            game.player.pos = Vec2::new(start_x, 152.0);
            game.player.previous_pos = game.player.pos;
            game.player.vel.x = 6.0 * f64::from(facing);
            game.world.set_tile(9, 8, "oBrickSmooth");
            let next_id = game.world.next_id;
            quiet_step(
                &mut game,
                InputFrame {
                    left: facing < 0,
                    right: facing > 0,
                    rope_pressed: true,
                    ..Default::default()
                },
            );
            let bounds = game.player.bounds();
            let left = bounds.x.round_ties_even();
            let right = (bounds.x + bounds.w - 1.0).round_ties_even();
            let post_movement_overlap = left <= 159.0 && right >= 144.0;
            assert_eq!(
                post_movement_overlap, !leaving,
                "fixture must cross the ceiling edge"
            );
            if leaving {
                assert_eq!(
                    game.world.next_id, next_id,
                    "earlier colTop still blocks launch"
                );
                assert_eq!(game.player.ropes, 4.0);
                assert_eq!(live(&game, "oRopeThrow"), 0);
                assert_eq!(throw_sounds(&game), 0);
            } else {
                assert!(
                    game.world.next_id > next_id,
                    "earlier clear colTop permits launch"
                );
                assert_eq!(game.player.ropes, 3.0);
                assert_eq!(throw_sounds(&game), 1);
            }
            assert_eq!(game.player.bombs, 4.0);
            assert_eq!(game.player.data["colTop"], f64::from(leaving));
        }
    }
}

#[test]
fn held_down_uses_two_pixel_clearance_while_dedicated_uses_eight() {
    for (rules, facing) in cases() {
        for held in [true, false] {
            let mut game = room(rules, facing);
            game.player.pos.x = if facing > 0 { 156.0 } else { 148.0 };
            game.player.previous_pos = game.player.pos;
            // Adjacent wall blocks +/-8 but leaves the source held +/-2 clear.
            game.world
                .set_tile(if facing > 0 { 10 } else { 8 }, 9, "oBrickSmooth");
            game.world.set_tile(if facing > 0 { 11 } else { 7 }, 10, "");
            let reserved = held.then(|| reserve_rope(&mut game));
            let attempt = game.world.next_id;
            quiet_step(
                &mut game,
                if held {
                    held_use(true)
                } else {
                    dedicated_use(true)
                },
            );
            if let Some(reserved) = reserved {
                assert!(game.world.entity(reserved).is_none());
                let top = game
                    .world
                    .entity(attempt + 1)
                    .expect("held alternate strip succeeds");
                assert_eq!(top.object, "oRopeTop");
                assert_eq!(top.pos.x, if facing > 0 { 184.0 } else { 120.0 });
                assert_eq!(game.player.ropes, 3.0);
                assert_eq!(throw_sounds(&game), 1);
            } else {
                assert_eq!(game.world.next_id, attempt + 1);
                assert!(game.world.entity(attempt).is_none());
                assert_eq!(live(&game, "oRopeTop"), 0);
                assert_eq!(game.player.ropes, 4.0);
                assert_eq!(throw_sounds(&game), 0);
            }
        }
    }
}
