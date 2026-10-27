//! A real input-only route through generated Mines to its exit.
//!
//! This composes level generation, movement, ledge jumps, hazards, the door,
//! and the transition controller without directly moving the player or
//! modifying the generated world.
use spelunky_core::ending::{EndingKind, EndingStage};
use spelunky_core::{Game, InputFrame, RulesPreset, RunState, Vec2};

const SEED_ZERO_MINES_EXIT_TAPE: &str = "2:11,1:71,7:31,23:11,2:10,5:12,1:10,28:11,24:12,1:72,7:32,1:12,1:72,31:12,1:72,9:12,1:72,7:32,1:12,1:72,7:32,1:12,1:72,38:12,1:10,1:12,4:10,4:11,2:10,3:14,1:16";

fn input_from_mask(mask: u8) -> InputFrame {
    InputFrame {
        left: mask & 1 != 0,
        right: mask & 2 != 0,
        up: mask & 4 != 0,
        down: mask & 8 != 0,
        run: mask & 16 != 0,
        jump: mask & 32 != 0,
        jump_pressed: mask & 64 != 0,
        ..Default::default()
    }
}

#[test]
fn seed_zero_reaches_generated_mines_exit_with_natural_inputs() {
    let mut game = Game::new(0, RulesPreset::Classic);
    assert_eq!(game.world.level, 1);
    assert_eq!(game.player.pos, game.world.entrance);
    let starting_health = game.player.health;
    let mut frames = 0;

    for token in SEED_ZERO_MINES_EXIT_TAPE.split(',') {
        let (count, mask) = token.split_once(':').expect("count:mask input run");
        let count = count.parse::<usize>().expect("frame count");
        let mask = u8::from_str_radix(mask, 16).expect("hex input mask");
        for _ in 0..count {
            assert_eq!(game.state, RunState::Playing, "before frame {frames}");
            game.step(&input_from_mask(mask));
            frames += 1;
        }
    }

    assert_eq!(frames, 217, "the tape must remain complete");
    assert_eq!(game.world.level, 1);
    assert_eq!(game.state, RunState::Ending);
    assert_eq!(game.player.pos, Vec2::new(584.0, 504.0));
    assert!(game.player.health > 0.0 && game.player.health <= starting_health);
    let ending = game
        .ending
        .as_ref()
        .expect("the generated exit starts its transition");
    assert_eq!(ending.kind, EndingKind::Transition);
    assert_eq!(ending.stage, EndingStage::Exit);
    assert_eq!(ending.completed_level, 1);
}
