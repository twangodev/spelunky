//! Shared real-input helpers for positioned source-lifecycle integration tests.
use spelunky_core::{
    Game, InputFrame, RulesPreset, RunState, Vec2,
    ending::{EndingKind, EndingStage},
};

/// Find a stable, generated Market entrance with destructible Lush cover.
/// Each candidate follows Game::new and real source level-generation
/// transitions through level eight; no world tile or campaign flag is staged.
pub fn generated_market_with_lush_cover() -> Game {
    for seed in [44, 45, 46, 47, 48, 49, 50, 51] {
        let mut game = Game::new(seed, RulesPreset::Classic);
        loop {
            if let Some(market) = game
                .world
                .entities
                .iter()
                .find(|entity| entity.alive && entity.object == "oXMarket")
            {
                let (x, y) = ((market.pos.x / 16.0) as i32, (market.pos.y / 16.0) as i32);
                if game
                    .world
                    .tile(x, y)
                    .is_some_and(|tile| tile.object == "oLush")
                {
                    return game;
                }
                break;
            }
            if game.world.level >= 8 {
                break;
            }
            game.next_level();
        }
    }
    panic!("bounded source-generation seed search found no covered Market entrance");
}

/// Reach the natural covered entrance from its generated level spawn with
/// movement and fresh jump inputs only. This is a deterministic test driver,
/// not a proof that every generated room layout admits this simple route.
pub fn approach_market_cover(game: &mut Game, market: Vec2) {
    let mut previous_x = game.player.pos.x;
    let mut stationary = 0;
    let mut jump_ticks = 0_u8;
    for _ in 0..500 {
        let dx = market.x + 24.0 - game.player.pos.x;
        if dx.abs() < 12.0 && (game.player.pos.y - (market.y - 8.0)).abs() < 24.0 {
            return;
        }
        if stationary >= 2 && jump_ticks == 0 {
            jump_ticks = 8;
            stationary = 0;
        }
        let jump = jump_ticks > 0;
        let jump_pressed = jump_ticks == 8;
        jump_ticks = jump_ticks.saturating_sub(1);
        let right = dx > 0.0;
        game.step(&InputFrame {
            left: !right,
            right,
            run: true,
            jump,
            jump_pressed,
            ..Default::default()
        });
        stationary = if (game.player.pos.x - previous_x).abs() < 0.5 {
            stationary + 1
        } else {
            0
        };
        previous_x = game.player.pos.x;
    }
    panic!(
        "input driver did not reach the generated Market cover: player={:?} market={market:?}",
        game.player.pos
    );
}

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
