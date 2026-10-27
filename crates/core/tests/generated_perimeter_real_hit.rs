//! A real trap hit against the generated Mines wall must leave controls usable.
use spelunky_core::{Game, InputFrame, RulesPreset, RunState, Vec2};

fn step(game: &mut Game, mut input: InputFrame, previous: &mut InputFrame) {
    input.left_released = previous.left && !input.left;
    input.right_released = previous.right && !input.right;
    input.jump_pressed = input.jump && !previous.jump;
    input.jump_released = previous.jump && !input.jump;
    game.step(&input);
    *previous = input;
    assert_eq!(game.state, RunState::Playing);
    assert!(game.player.pos.x >= 16.0 && game.player.pos.x < 656.0);
    assert!(!game.world.solid_at(game.player.pos.x, game.player.pos.y));
}

#[test]
fn real_arrow_stun_at_generated_perimeter_recovers_for_running_and_jumping() {
    // Keep the generated room and actors intact. This seed's entrance is
    // beside the left perimeter; the added trap supplies an airborne hit.
    let mut game = Game::new(2036, RulesPreset::Classic);
    let entrance = game.player.pos;
    game.spawn(
        "oArrowTrapLeft",
        Vec2::new(entrance.x + 24.0, entrance.y - 24.0),
    );
    let mut previous = InputFrame::default();
    for _ in 0..4 {
        step(&mut game, InputFrame::default(), &mut previous);
    }
    assert_eq!(game.player.health, 4.0);
    assert!(game.player.grounded && !game.player.is_stunned());

    let mut saw_arrow = false;
    let mut hit = false;
    for _ in 0..32 {
        step(
            &mut game,
            InputFrame {
                left: true,
                jump: true,
                ..Default::default()
            },
            &mut previous,
        );
        saw_arrow |= game
            .world
            .entities
            .iter()
            .any(|entity| entity.alive && entity.object == "oArrow" && entity.vel.x < -3.0);
        if game.player.health < 4.0 {
            assert!(saw_arrow, "the hit must follow a live trap Arrow");
            assert_eq!(game.player.health, 2.0);
            assert!(game.player.is_stunned() && game.player.stun > 0);
            assert!(!game.player.grounded);
            hit = true;
            break;
        }
    }
    assert!(hit, "the trap must actually stun the player");

    let mut wall_contact = false;
    for _ in 0..160 {
        wall_contact |= game.player.bounds().x <= 16.0 && game.player.is_stunned();
        step(&mut game, InputFrame::default(), &mut previous);
        assert_eq!(game.player.health, 2.0);
        if !game.player.is_stunned() && game.player.grounded && game.player.vel == Vec2::default() {
            break;
        }
    }
    assert!(
        wall_contact,
        "the stunned player must reach the generated wall"
    );
    assert!(
        !game.player.is_stunned() && game.player.grounded && game.player.vel == Vec2::default(),
        "player failed to recover: {:?}",
        game.player
    );

    let recovered_x = game.player.pos.x;
    for _ in 0..20 {
        step(
            &mut game,
            InputFrame {
                right: true,
                run: true,
                ..Default::default()
            },
            &mut previous,
        );
    }
    assert!(
        game.player.pos.x >= recovered_x + 8.0,
        "running must resume"
    );
    for _ in 0..12 {
        step(&mut game, InputFrame::default(), &mut previous);
    }

    // Walk clear of the added trap's ceiling before testing a fresh jump.
    for _ in 0..48 {
        if game.player.pos.x <= entrance.x - 8.0 {
            break;
        }
        step(
            &mut game,
            InputFrame {
                left: true,
                ..Default::default()
            },
            &mut previous,
        );
    }
    assert!(game.player.pos.x <= entrance.x - 8.0);
    for _ in 0..12 {
        step(&mut game, InputFrame::default(), &mut previous);
    }
    assert!(game.player.grounded && !game.player.is_stunned());
    let before_jump = game.player.pos.y;
    step(
        &mut game,
        InputFrame {
            left: true,
            jump: true,
            ..Default::default()
        },
        &mut previous,
    );
    assert!(
        game.player.vel.y < 0.0 && game.player.pos.y < before_jump,
        "jumping must resume"
    );
    assert_eq!(game.player.health, 2.0);
}
