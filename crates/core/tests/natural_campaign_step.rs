//! Seed 0, level 1: actual inputs can follow the generated Mines route from
//! its authored entrance through the first two room descents. This is partial
//! reachability evidence; it does not cover the level exit or campaign adapter.
use spelunky_core::{Game, InputFrame, PlayerState, RulesPreset, RunState};

fn room_center(col: usize) -> f64 {
    16.0 + col as f64 * 160.0 + 80.0
}

#[test]
fn seed_zero_mines_reaches_row_two_from_authored_entrance_with_game_step() {
    let mut game = Game::new(0, RulesPreset::Classic);
    assert_eq!(game.world.level, 1);
    assert_eq!(game.player.pos, game.world.entrance);
    let path = game.world.source_room_path.expect("source room path");
    let starting_health = game.player.health;
    let starting_inventory = (
        game.player.bombs,
        game.player.ropes,
        game.player.equipment.clone(),
    );
    let starting_money = game.player.money;
    let mut previous_x = game.player.pos.x;
    let mut stuck = 0_u8;
    let mut jump_ticks = 0_u8;
    let mut previous_jump = false;
    let mut reached_row_two = false;

    for tick in 0..120 {
        assert_eq!(game.state, RunState::Playing);
        let row = ((game.player.pos.y / 128.0).floor() as usize).min(3);
        if row >= 2 {
            reached_row_two = true;
            break;
        }
        let entry_col = path[row].iter().position(|mark| *mark == 3);
        let exit_col = path[row].iter().position(|mark| *mark == 2);
        // Follow the initial room's authored descent. Room 1 needs to be
        // crossed from its upper entry to the lower connector before moving
        // across the wall toward column 0.
        let target_x = if row == 0 {
            exit_col.map_or(game.world.exit.x, room_center)
        } else if row == 1 && game.player.pos.y < row as f64 * 128.0 + 64.0 {
            entry_col.map_or(game.world.exit.x, room_center)
        } else {
            exit_col.map_or(game.world.exit.x, room_center)
        };
        let dx = target_x - game.player.pos.x;
        let direction = if dx.abs() > 5.0 { dx.signum() } else { 0.0 };
        let blocked = direction != 0.0 && (game.player.pos.x - previous_x).abs() < 0.25;
        stuck = if blocked { stuck.saturating_add(1) } else { 0 };
        previous_x = game.player.pos.x;
        if stuck >= 3
            && jump_ticks == 0
            && (game.player.grounded || game.player.state == PlayerState::Hanging)
        {
            jump_ticks = if game.player.state == PlayerState::Hanging {
                1
            } else {
                8
            };
            stuck = 0;
        }
        let jump = jump_ticks > 0;
        let jump_pressed = jump && !previous_jump;
        let drop_from_hang = jump_pressed && game.player.state == PlayerState::Hanging;
        jump_ticks = jump_ticks.saturating_sub(1);
        previous_jump = jump;
        game.step(&InputFrame {
            left: direction < 0.0,
            right: direction > 0.0,
            down: drop_from_hang,
            run: true,
            jump,
            jump_pressed,
            ..Default::default()
        });
        assert_eq!(
            game.player.health, starting_health,
            "hazard damage at tick {tick}"
        );
        assert_eq!(game.world.level, 1);
        assert_eq!(game.state, RunState::Playing);
        assert_eq!(
            (
                game.player.bombs,
                game.player.ropes,
                game.player.equipment.clone(),
            ),
            starting_inventory,
            "the route does not spend bombs, ropes, or equipment"
        );
        reached_row_two = game.player.pos.y >= 256.0;
        if reached_row_two {
            break;
        }
    }

    assert!(
        reached_row_two,
        "actual Game::step inputs did not reach row two: pos {:?}, path {:?}",
        game.player.pos, path
    );
    assert_eq!(game.world.level, 1);
    assert_eq!(game.state, RunState::Playing);
    assert_eq!(
        game.player.money,
        starting_money + 1000,
        "natural pickup rewards along this seed-fixed route"
    );
}
