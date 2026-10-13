//! Initial authored campaign-door contexts; all timing, exit input and history
//! after the first Step come from public APIs. This is not a generated route or
//! a natural Udjat/Market-unlock witness, nor an original RNG-stream comparison.
use spelunky_core::*;

fn initial_campaign_door(seed: u32, rules: RulesPreset, market: bool) -> Game {
    let mut level = level::Level::default();
    level.set_tile(6, 7, '@').unwrap();
    for x in 0..40 {
        level.set_tile(x, 8, '1').unwrap();
    }
    if !market {
        level.set_tile(7, 7, 'X').unwrap();
    }
    let mut game = Game::shell(seed, rules);
    game.load_custom(&level, "authored-dark-transition", "");
    // Initial source campaign context only. No Player/timer/RNG fields are set,
    // and no state/flag/world writes follow the first Step except next_level.
    game.flags.remove("custom");
    game.flags.remove("custom_level");
    game.flags.insert("no_dark_level".into());
    game.world.level = 5;
    game.world.area = Area::Jungle;
    if market {
        game.spawn("oXMarket", Vec2::new(128.0, 128.0));
    }
    assert_eq!(game.player.pos, Vec2::new(120.0, 136.0));
    assert!(!game.flags.contains("had_dark_level"));
    game
}

fn leave_with_inputs(game: &mut Game, neutral_ticks: usize, disabled: bool, market: bool) {
    for _ in 0..neutral_ticks {
        game.step(&InputFrame::default());
        assert_eq!(game.state, RunState::Playing);
        assert_eq!(game.player.health, 4);
        assert!(!game.player.is_stunned());
    }
    assert!(
        game.player.grounded,
        "source door needs genuine standing support"
    );
    for _ in 0..40 {
        game.step(&InputFrame {
            right: true,
            up: true,
            ..Default::default()
        });
        if game.state == RunState::Ending {
            break;
        }
    }
    assert_eq!(game.state, RunState::Ending, "ACTUAL_UP_DOOR");
    assert_eq!(
        game.flags.contains("genBlackMarket"),
        market,
        "actual door branch controls destination"
    );
    for _ in 0..160 {
        if game
            .ending
            .as_ref()
            .is_some_and(|ending| ending.stage == ending::EndingStage::Transition)
        {
            break;
        }
        game.step(&InputFrame::default());
    }
    let ending = game
        .ending
        .as_ref()
        .expect("actual exit animation creates Transition");
    assert_eq!(ending.stage, ending::EndingStage::Transition);
    assert_eq!(ending.kind, ending::EndingKind::Transition);
    let seconds = ending.level_time_ms / 1000 % 60;
    assert_eq!(
        seconds <= 20,
        disabled,
        "source remainder-seconds threshold must be reached by actual time"
    );
    for _ in 0..300 {
        if game.ending.as_ref().unwrap().draw_loot > -2 {
            break;
        }
        game.step(&InputFrame::default());
    }
    assert!(
        game.ending.as_ref().unwrap().draw_loot > -2,
        "actual transition reveals time before changing eligibility"
    );
    // One source Transition Step observes the revealed time.
    game.step(&InputFrame::default());
    assert_eq!(game.flags.contains("no_dark_level"), disabled);
    for tick in 0..300 {
        game.step(&InputFrame {
            start_pressed: tick % 2 == 0,
            ..Default::default()
        });
        if game.state == RunState::Transition {
            break;
        }
    }
    assert_eq!(
        game.state,
        RunState::Transition,
        "actual menu inputs finish the ordinary exit transition"
    );
    game.next_level();
    assert_eq!(game.state, RunState::Playing);
    assert_eq!(game.world.level, 6);
    assert_eq!(game.flags.contains("no_dark_level"), disabled);
}

#[test]
fn ordinary_exit_uses_real_fast_and_slow_time_to_control_dark_eligibility() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (ticks, disabled) in [(8, true), (700, false)] {
            let mut game = initial_campaign_door(42, rules, false);
            leave_with_inputs(&mut game, ticks, disabled, false);
            if disabled {
                assert!(
                    !game.world.dark,
                    "a fast ordinary exit forbids visible darkness"
                );
            }
            if game.world.dark {
                assert!(
                    game.flags.contains("had_dark_level"),
                    "generation commits history before the first new-room Step"
                );
            }
        }
    }
}

#[test]
fn slow_market_exit_records_selected_history_even_though_market_is_bright() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut selected = None;
        for seed in 0..64 {
            let mut game = initial_campaign_door(seed, rules, true);
            leave_with_inputs(&mut game, 700, false, true);
            assert_eq!(game.world.name, "Black Market");
            assert!(!game.world.dark, "Market overrides visibility only");
            if game.flags.contains("had_dark_level") {
                selected = Some(seed);
                break;
            }
        }
        assert!(
            selected.is_some(),
            "MARKET_INVISIBLE_SELECTION_HISTORY: an eligible stock-probability winning roll must survive the Market visibility override; rules={rules:?}"
        );
    }
}
