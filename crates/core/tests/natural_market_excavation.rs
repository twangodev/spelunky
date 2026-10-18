//! The generated Black Market entrance is found, exposed, and entered through
//! ordinary gameplay. Every route action uses public `Game::step` input.
mod common;

use common::{approach_market_cover, complete_source_exit, generated_market_with_lush_cover};
use spelunky_core::{InputFrame, RunState};

#[test]
fn generated_market_cover_is_bombed_and_entered_with_real_inputs() {
    // The Mines source gate correction changes later RNG streams. Select the
    // first covered Market from a bounded deterministic sequence while keeping
    // the actual entrance generation path (`Game::new` + `next_level`).
    let mut game = generated_market_with_lush_cover();
    let market = game
        .world
        .entities
        .iter()
        .find(|entity| entity.alive && entity.object == "oXMarket")
        .expect("the selected generated level contains its Market exit");
    let market_id = market.id;
    let market_pos = market.pos;
    let wall_x = (market_pos.x / 16.0) as i32;
    let wall_y = (market_pos.y / 16.0) as i32;
    assert_eq!(game.world.tile(wall_x, wall_y).unwrap().object, "oLush");

    // No player or terrain coordinates are staged: this controller walks from
    // the natural spawn and jumps when the generated geometry blocks progress.
    approach_market_cover(&mut game, market_pos);
    assert!(game.player.health > 0.0);

    let bombs = game.player.bombs;
    game.step(&InputFrame {
        down: true,
        bomb_pressed: true,
        ..Default::default()
    });
    assert_eq!(game.player.bombs, bombs - 1.0);
    let bomb_id = game
        .world
        .entities
        .iter()
        .find(|entity| entity.alive && entity.object == "oBomb")
        .expect("Down+B creates the live bomb instance")
        .id;

    let escape_right = game.player.pos.x >= market_pos.x;
    for _ in 0..12 {
        game.step(&InputFrame {
            left: !escape_right,
            right: escape_right,
            run: true,
            ..Default::default()
        });
    }
    assert!((game.player.pos.x - market_pos.x).abs() > 48.0);

    for _ in 0..150 {
        if game.world.entity(bomb_id).is_none() {
            break;
        }
        game.step(&InputFrame::default());
    }

    assert!(
        game.world.entity(bomb_id).is_none(),
        "the real fuse completes"
    );
    assert!(game.world.tile(wall_x, wall_y).unwrap().object.is_empty());
    assert!(game.world.entity(market_id).is_some());
    assert!(game.player.health > 0.0);

    // Walk back to the generated portal and hold Up when inside its source
    // contact neighborhood. The exit checks still require the live grounded
    // controller state, which this input sequence naturally reaches.
    let mut previous_x = game.player.pos.x;
    let mut stuck = 0;
    let mut jump_ticks = 0_u8;
    for _ in 0..1200 {
        if game.state == RunState::Ending {
            break;
        }
        let dx = market_pos.x + 8.0 - game.player.pos.x;
        let horizontal = if dx.abs() < 5.0 {
            0
        } else {
            i8::from(dx > 0.0) * 2 - 1
        };
        let blocked = horizontal != 0 && (game.player.pos.x - previous_x).abs() < 0.5;
        stuck = if blocked { stuck + 1 } else { 0 };
        if stuck >= 2 && jump_ticks == 0 {
            jump_ticks = 8;
            stuck = 0;
        }
        let jump = jump_ticks > 0;
        let jump_pressed = jump_ticks == 8;
        jump_ticks = jump_ticks.saturating_sub(1);
        let near = dx.abs() < 30.0 && (game.player.pos.y - (market_pos.y + 8.0)).abs() < 32.0;
        game.step(&InputFrame {
            left: horizontal < 0,
            right: horizontal > 0,
            run: true,
            jump,
            jump_pressed,
            up: near,
            ..Default::default()
        });
        previous_x = game.player.pos.x;
    }
    assert_eq!(
        game.state,
        RunState::Ending,
        "the real Up input enters Market"
    );
    complete_source_exit(&mut game);
    assert!(game.flags.contains("genBlackMarket"));
    game.next_level();
    assert_eq!(game.world.name, "Black Market");
    assert_eq!(game.world.level, 8);
    assert!(
        game.world
            .entities
            .iter()
            .any(|entity| entity.object == "oAnkh")
    );
}

#[test]
fn generated_market_distance_rearms_udjat_blink_by_real_movement() {
    let mut game = generated_market_with_lush_cover();
    let market_pos = game
        .world
        .entities
        .iter()
        .find(|entity| entity.alive && entity.object == "oXMarket")
        .unwrap()
        .pos;
    game.step(&InputFrame::default());
    let initial_alarm = game.alarms[2];
    assert!(
        initial_alarm > 0,
        "the active generated Market arms Alarm 2"
    );

    approach_market_cover(&mut game, market_pos);
    let close_alarm = game.alarms[2];
    assert!(
        close_alarm < initial_alarm,
        "real movement shortens the Market alarm"
    );
    assert!(!game.flags.contains("udjat_blink"));

    // Alarm 2 toggles after the current source countdown; oGame keeps the
    // closest Market distance live while the player waits.
    let mut blinked = false;
    for _ in 0..(close_alarm.max(1) as usize + 20) {
        game.step(&InputFrame::default());
        if game.flags.contains("udjat_blink") {
            blinked = true;
            break;
        }
    }
    assert!(blinked, "the shortened real alarm reaches its blink edge");
}
