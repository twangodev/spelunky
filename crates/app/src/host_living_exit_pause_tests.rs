//! Source screen eligibility around a real, ordinary door-exit animation.
use super::*;
use spelunky_core::{InputFrame, PlayerState, RulesPreset, ending::EndingStage, level::Level};

fn natural_door_exit(rules: RulesPreset) -> Game {
    let mut level = Level::default();
    for x in 0..40 {
        level.set_tile(x, 9, '1').unwrap();
    }
    level.set_tile(4, 8, '@').unwrap();
    level.set_tile(8, 8, 'X').unwrap();
    level.next_level = "NEXT.LVL".into();
    level.exit_targets = vec!["NONE".into(), "NEXT.LVL".into()];
    let authored = Level::parse(&level.to_text()).unwrap();
    let mut game = Game::shell(1137, rules);
    game.load_custom(&authored, "living-exit-pause", "");
    assert_eq!(game.player.pos, spelunky_core::Vec2::new(88., 152.));
    for _ in 0..6 {
        game.step(&InputFrame::default());
    }
    assert!(game.player.grounded);
    assert!(!ending_uses_cinematic_controls(&game));
    let door = game
        .world
        .entities
        .iter()
        .find(|e| e.object == "oExit")
        .unwrap()
        .pos;
    for _ in 0..80 {
        if game.player.pos.x >= door.x + 4. {
            break;
        }
        game.step(&InputFrame {
            right: true,
            ..Default::default()
        });
    }
    assert!((game.player.pos.x - (door.x + 8.)).abs() <= 8.);
    for _ in 0..16 {
        game.step(&InputFrame {
            up: true,
            ..Default::default()
        });
        if game.state == RunState::Ending {
            break;
        }
    }
    assert_eq!(
        game.state,
        RunState::Ending,
        "ordinary Up must enter the authored door"
    );
    assert_eq!(game.ending.as_ref().unwrap().stage, EndingStage::Exit);
    assert!(game.player.is_exiting());
    assert!(game.player.engine_active);
    assert_ne!(game.player.state, PlayerState::Dead);
    assert_eq!(
        game.player.data["active"], 0.,
        "source controller variable is not engine deactivation"
    );
    game
}

#[test]
fn natural_door_exit_allows_pause_and_resume_before_animation_end() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = natural_door_exit(rules);
        assert!(
            !ending_uses_cinematic_controls(&game),
            "keyboard and controller share this actual predicate"
        );
        let settings = crate::storage::Settings::default();
        assert!(configured_pause_key(&settings.bindings, KeyCode::Escape));
        assert_eq!(
            source_paused_control(Some(&game), &InputFrame::default(), true),
            Some(SourcePausedControl::Resume)
        );
        let before = game.state_hash();
        let image = game.player.animation;
        let mut timing = HostTiming::default();
        timing.reset(0.);
        // Actual shared host gate blocks Game.step on Pause. Sampling the
        // frozen presentation does not advance the exit frame or game RNG.
        for frame in 1..=120 {
            assert!(!render_clock_running(
                Screen::Pause,
                true,
                false,
                Some(&game)
            ));
            let _ = timing.render_alpha(f64::from(frame) / 240., false);
        }
        assert_eq!(game.state_hash(), before);
        assert_eq!(game.player.animation, image);
        assert!(render_clock_running(
            Screen::Playing,
            true,
            false,
            Some(&game)
        ));
        for _ in 0..60 {
            game.step(&InputFrame::default());
            if game.ending.as_ref().unwrap().stage != EndingStage::Exit {
                break;
            }
        }
        assert_eq!(game.ending.as_ref().unwrap().stage, EndingStage::Transition);
        assert_eq!(game.world.name, "rTransition1");
        assert!(
            ending_uses_cinematic_controls(&game),
            "later no-Player cinematic keeps skip/start controls"
        );
    }
}

#[test]
fn natural_paused_exit_preserves_bomb_rope_start_priority() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let game = natural_door_exit(rules);
        assert!(game.is_level());
        let settings = crate::storage::Settings::default();
        for (keys, expected) in [
            (
                vec![KeyCode::KeyB, KeyCode::KeyR],
                SourcePausedControl::Suicide,
            ),
            (vec![KeyCode::KeyR], SourcePausedControl::Quit),
            (vec![], SourcePausedControl::Resume),
        ] {
            let mut input = InputState::default();
            input.set_bindings(&settings.bindings);
            for key in keys {
                input.key(key, true, false);
            }
            assert_eq!(
                source_paused_control(Some(&game), &input.take_tick(), true),
                Some(expected)
            );
        }
    }
}

#[test]
fn only_living_active_exit_player_bypasses_cinematic_controls() {
    use spelunky_core::ending::{EndingKind, EndingState};
    // Initial configured negative boundaries; no claim these are natural
    // death producers. The two tests above retain a fully ordinary exit path.
    for stage in [
        EndingStage::Chamber,
        EndingStage::Volcano,
        EndingStage::Landing,
        EndingStage::Transition,
        EndingStage::Summary,
        EndingStage::Credits,
        EndingStage::Complete,
    ] {
        let mut game = Game::shell(1137, RulesPreset::Classic);
        game.state = RunState::Ending;
        game.ending = Some(EndingState::new(&game, EndingKind::Transition, stage));
        assert!(ending_uses_cinematic_controls(&game));
    }
    for (dead, active, sprite) in [
        (true, true, "sPExit"),
        (false, false, "sPExit"),
        (false, true, "sStandRight"),
    ] {
        let mut game = Game::shell(1137, RulesPreset::Classic);
        game.state = RunState::Ending;
        game.player.state = if dead {
            PlayerState::Dead
        } else {
            PlayerState::Standing
        };
        game.player.engine_active = active;
        game.player.sprite = sprite.into();
        game.ending = Some(EndingState::new(
            &game,
            EndingKind::Transition,
            EndingStage::Exit,
        ));
        assert!(ending_uses_cinematic_controls(&game));
    }
}
