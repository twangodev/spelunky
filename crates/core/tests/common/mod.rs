//! Shared real-input helpers for positioned source-lifecycle integration tests.
use spelunky_core::{
    Game, InputFrame, RunState,
    ending::{EndingKind, EndingStage},
};

pub fn complete_source_exit(game: &mut Game) {
    let completed_level = game.world.level;
    assert_eq!(
        game.state,
        RunState::Ending,
        "seed {}, level {}",
        game.seed,
        completed_level
    );
    assert_eq!(game.ending.as_ref().unwrap().kind, EndingKind::Transition);
    assert_eq!(game.ending.as_ref().unwrap().stage, EndingStage::Exit);

    // The live room keeps ticking during the source exit animation. Do not
    // skip its hazards or replace the room before the animation completes.
    for _ in 0..100 {
        if game.ending.as_ref().unwrap().stage != EndingStage::Exit {
            break;
        }
        game.step(&InputFrame::default());
        assert_eq!(game.state, RunState::Ending);
    }
    assert_eq!(game.ending.as_ref().unwrap().stage, EndingStage::Transition);

    // Start accelerates the tally; it cannot immediately advance a freshly
    // entered transition. Subsequent distinct Attack presses also progress
    // any source Tunnel Man conversation before allowing room continuation.
    game.step(&InputFrame {
        start_pressed: true,
        ..Default::default()
    });
    assert_eq!(game.state, RunState::Ending);
    assert_eq!(game.ending.as_ref().unwrap().stage, EndingStage::Transition);
    for tick in 0..600 {
        if game.state == RunState::Transition {
            break;
        }
        assert_eq!(game.state, RunState::Ending);
        game.step(&InputFrame {
            attack: tick % 2 == 0,
            attack_pressed: tick % 2 == 0,
            ..Default::default()
        });
    }
    assert_eq!(
        game.state,
        RunState::Transition,
        "source tally did not finish: seed {}, level {}, ending {:?}",
        game.seed,
        completed_level,
        game.ending
    );
    let ending = game.ending.as_ref().unwrap();
    assert_eq!(ending.kind, EndingKind::Transition);
    assert_eq!(ending.stage, EndingStage::Complete);
    assert_eq!(ending.draw_loot, 2);
    assert_eq!(ending.money_count, ending.level_money());
    assert!(game.flags.contains("transition_cinematic_complete"));
}
