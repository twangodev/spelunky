use crate::*;

fn room() -> Game {
    let mut g = Game::legacy_shell(7, RulesPreset::Classic);
    g.world = World::new(20, 12, Area::Mines, 2);
    for x in 0..20 {
        g.world.set_tile(x, 9, "oBlock");
    }
    g.player.pos = Vec2::new(56.0, 136.0);
    g.player.previous_pos = g.player.pos;
    g
}
fn m() -> InputFrame {
    InputFrame {
        music_pressed: true,
        ..Default::default()
    }
}
fn play(name: &str, looping: bool, if_idle: bool) -> SourceMusicCommand {
    SourceMusicCommand::Play {
        name: name.into(),
        looping,
        if_idle,
    }
}
fn frequency(name: &str, hz: i64) -> SourceMusicCommand {
    SourceMusicCommand::Frequency {
        name: name.into(),
        hz,
    }
}
fn off(g: &mut Game) {
    g.step(&m());
    assert!(!g.source_music.enabled);
    assert_eq!(
        g.source_music.take_commands(),
        vec![SourceMusicCommand::StopAll]
    );
}

#[test]
fn natural_m_edges_stop_then_play_without_changing_simulation_or_frequency() {
    let mut g = room();
    g.step(&InputFrame::default());
    g.source_music.take_commands();
    let mut control = g.clone();
    control.step(&InputFrame::default());
    off(&mut g);
    assert_eq!(g.tick, control.tick);
    assert_eq!(g.player.pos, control.player.pos);
    assert_eq!(g.player.vel, control.player.vel);
    assert_eq!(g.rng.clone().next_u64(), control.rng.clone().next_u64());
    g.step(&m());
    assert!(g.source_music.enabled);
    assert_eq!(
        g.source_music.take_commands(),
        vec![play("musCave", true, true)]
    );
    assert_eq!(g.source_music.frequency("musCave"), 44_100);
    assert!(
        !g.events
            .iter()
            .any(|e| matches!(e, GameEvent::MusicFrequency { .. }))
    );
}

#[test]
fn natural_warning_fades_while_off_and_disabled_room_create_preserves_named_rate() {
    let mut g = room();
    for _ in 0..4000 {
        g.step(&InputFrame::default());
    }
    assert_eq!(g.data["xtime"], 120_000.0);
    g.source_music.take_commands();
    // Keyboard group precedes Game318 warning and Level553 fade on this frame.
    g.step(&m());
    assert!(!g.source_music.enabled);
    assert_eq!(
        g.source_music.take_commands(),
        vec![SourceMusicCommand::StopAll, frequency("musCave", 44_000)]
    );
    for n in 2..=100 {
        g.step(&InputFrame::default());
        assert_eq!(
            g.source_music.take_commands(),
            vec![frequency("musCave", 44_100 - n * 100)]
        );
    }
    assert_eq!(g.data["music_fade_timer"], 100.0);
    assert_eq!(g.source_music.frequency("musCave"), 34_100);
    g.next_level();
    assert_eq!(g.world.level, 3);
    assert_eq!(g.level_tick, 0);
    assert!(!g.source_music.enabled);
    assert!(g.source_music.take_commands().is_empty());
    assert_eq!(g.source_music.frequency("musCave"), 34_100);
    g.step(&m());
    assert_eq!(
        g.source_music.take_commands(),
        vec![play("musCave", true, true)]
    );
    assert_eq!(g.source_music.frequency("musCave"), 34_100);
}

#[test]
fn real_custom_lush_start_and_m_on_have_distinct_source_selection() {
    for name in ["music-toggle.lvl", "rescue.lvl", "rEnd", "rEndCustom"] {
        let mut g = room();
        let mut level = level::Level::default();
        level.music = "LUSH".into();
        g.load_custom(&level, name, "");
        assert_eq!(
            g.source_music.take_commands(),
            vec![play("musLush", true, true), frequency("musLush", 44_100)]
        );
        off(&mut g);
        g.step(&m());
        assert_eq!(g.world.area, Area::Mines);
        assert_eq!(g.data["custom_music"], 1.0);
        assert_eq!(
            g.source_music.take_commands(),
            vec![play("musCave", true, true)]
        );
    }
}

#[test]
fn custom_music_codes_match_source_case_sensitively() {
    let mut g = room();
    let mut level = level::Level::default();
    level.music = "lush".into();

    g.load_custom(&level, "lowercase-music.lvl", "");

    assert_eq!(g.data["custom_music"], 0.0);
    assert_eq!(
        g.source_music.take_commands(),
        vec![play("musCave", true, true), frequency("musCave", 44_100)]
    );
}

#[test]
fn source_session_reaches_create_before_new_and_tutorial_generation() {
    let mut old = room();
    old.step(&InputFrame::default());
    old.source_music.take_commands();
    off(&mut old);
    let mut next = Game::new_with_music(42, RulesPreset::Classic, old.source_music.session_state());
    assert!(!next.source_music.enabled);
    assert!(next.source_music.take_commands().is_empty());
    let mut tutorial =
        Game::tutorial_with_music(42, RulesPreset::Classic, next.source_music.session_state());
    assert!(!tutorial.source_music.enabled);
    assert!(tutorial.source_music.take_commands().is_empty());
    let mut fresh = Game::new(42, RulesPreset::Classic);
    assert!(fresh.source_music.enabled);
    assert_eq!(
        fresh.source_music.take_commands(),
        vec![play("musCave", true, true), frequency("musCave", 44_100)]
    );
    // Adding music session state cannot change canonical generation draws/layout.
    assert_eq!(fresh.world.tiles.len(), next.world.tiles.len());
    assert_eq!(fresh.rng.clone().next_u64(), next.rng.clone().next_u64());
}

#[test]
fn natural_title_alarm_three_bypasses_disabled_play_music_gate() {
    let mut g = room();
    g.step(&InputFrame::default());
    g.source_music.take_commands();
    off(&mut g);
    // Select the authored first Title entrance before entering that room.
    g.flags.insert("title_start:0".into());
    g.load_hub(hub::HubKind::Title);
    assert!(g.source_music.take_commands().is_empty());
    let mut heard = false;
    for _ in 0..300 {
        g.step(&InputFrame::default());
        let commands = g.source_music.take_commands();
        if !commands.is_empty() {
            assert_eq!(commands, vec![play("musTitle", false, false)]);
            heard = true;
            break;
        }
    }
    assert!(
        heard,
        "natural Title alarm chain must reach literal playSound"
    );
    assert!(!g.source_music.enabled);
    g.step(&m());
    assert_eq!(
        g.source_music.take_commands(),
        vec![play("musTitle", true, true)]
    );
    off(&mut g);
}

#[test]
fn natural_challenge_m_on_waits_for_actual_music_alarm() {
    for kind in [hub::HubKind::Sun, hub::HubKind::Moon, hub::HubKind::Stars] {
        let mut g = room();
        g.step(&InputFrame::default());
        g.source_music.take_commands();
        off(&mut g);
        g.load_hub(kind);
        assert!(g.source_music.take_commands().is_empty());
        g.step(&m());
        assert!(g.source_music.enabled);
        assert!(g.source_music.take_commands().is_empty());
        for _ in 1..99 {
            g.step(&InputFrame::default());
            assert!(g.source_music.take_commands().is_empty());
        }
        g.step(&InputFrame::default());
        assert_eq!(g.level_tick, 100);
        assert_eq!(
            g.source_music.take_commands(),
            vec![play("musBoss", true, true), frequency("musBoss", 44_100)]
        );
    }
}

#[test]
fn natural_disabled_challenge_alarm_is_not_replayed_when_later_enabled() {
    let mut g = room();
    g.step(&InputFrame::default());
    g.source_music.take_commands();
    off(&mut g);
    g.load_hub(hub::HubKind::Sun);
    for _ in 0..100 {
        g.step(&InputFrame::default());
        assert!(g.source_music.take_commands().is_empty());
    }
    g.step(&m());
    assert!(g.source_music.enabled);
    assert!(g.source_music.take_commands().is_empty());
    g.step(&InputFrame::default());
    assert!(g.source_music.take_commands().is_empty());
}

#[test]
fn natural_olmec_unlock_obeys_enable_without_resetting_frequency() {
    let mut g = room();
    g.step(&InputFrame::default());
    g.source_music.take_commands();
    off(&mut g);
    g.start_from_hub(false, 16);
    assert!(g.source_music.take_commands().is_empty());
    assert_eq!(g.player.data["active"], 0.0);
    g.step(&m());
    assert!(g.source_music.enabled);
    assert!(g.source_music.take_commands().is_empty());
    let mut unlocked = false;
    for _ in 0..650 {
        g.step(&InputFrame::default());
        let commands = g.source_music.take_commands();
        if !commands.is_empty() {
            assert_eq!(commands, vec![play("musBoss", true, true)]);
            assert_eq!(g.player.data["active"], 1.0);
            unlocked = true;
            break;
        }
    }
    assert!(unlocked, "real Olmec intro must reach Alarm5");
}

#[test]
fn natural_custom_summary_has_no_o_game_key_handler_and_gates_victory() {
    let mut g = room();
    g.flags.insert("custom_level".into());
    g.step(&InputFrame::default());
    g.source_music.take_commands();
    off(&mut g);
    ending::begin_custom(&mut g);
    assert_eq!(g.world.name, "rEndCustom");
    assert!(!music::has_active_game(&g));
    g.step(&m());
    assert!(!g.source_music.enabled);
    assert!(g.source_music.take_commands().is_empty());
    for _ in 1..50 {
        g.step(&InputFrame::default());
        assert!(g.source_music.take_commands().is_empty());
    }
    assert_eq!(g.ending.as_ref().unwrap().music, "musVictory");
    assert!(!g.source_music.enabled);
}

#[test]
fn signed_source_fade_boundary_keeps_zero_and_negative_requests() {
    // Explicit initial contract fixture, not a natural historical frequency trajectory.
    let mut g = room();
    g.source_music.enabled = false;
    g.source_music.frequencies.insert("musCave".into(), 100);
    g.data.insert("xtime".into(), 120_000.0);
    g.step(&InputFrame::default());
    assert_eq!(
        g.source_music.take_commands(),
        vec![frequency("musCave", 0)]
    );
    assert_eq!(g.source_music.frequency("musCave"), 0);
    assert!(
        !g.events
            .iter()
            .any(|e| matches!(e, GameEvent::MusicFrequency { .. }))
    );
    g.step(&InputFrame::default());
    assert_eq!(
        g.source_music.take_commands(),
        vec![frequency("musCave", -100)]
    );
    assert_eq!(g.source_music.frequency("musCave"), -100);
}

#[test]
fn inactive_receiver_drops_m_and_source_edge_clear_cannot_replay_it() {
    // Core guard only. Actual continuously paused transport is covered by host tests.
    let mut g = room();
    g.flags.insert("source_game_inactive".into());
    let mut input = m();
    music::key_pressed(&mut g);
    input.clear_edges();
    assert!(g.source_music.enabled);
    assert!(g.source_music.take_commands().is_empty());
    assert!(!input.music_pressed);
    g.flags.remove("source_game_inactive");
    assert!(music::has_active_game(&g));
    assert!(g.source_music.enabled);
    assert!(g.source_music.take_commands().is_empty());
}

#[test]
fn durable_commands_survive_event_clear_and_session_handoff_never_replays_them() {
    // Additive outbox contract; both semantic arms share this API test.
    let mut g = room();
    music::key_pressed(&mut g);
    g.events.clear();
    assert_eq!(
        g.source_music.take_commands(),
        vec![SourceMusicCommand::StopAll]
    );
    assert!(g.source_music.take_commands().is_empty());
    let mut next = Game::shell_with_music(42, RulesPreset::Classic, g.source_music.session_state());
    assert!(!next.source_music.enabled);
    assert!(next.source_music.take_commands().is_empty());
}

#[test]
fn natural_exit_stops_music_before_the_exit_animation() {
    let mut g = room();
    g.spawn("oExit", Vec2::new(48.0, 128.0));
    for _ in 0..3 {
        g.step(&InputFrame::default());
    }
    g.source_music.take_commands();
    g.step(&InputFrame {
        up: true,
        ..Default::default()
    });
    assert!(g.player.is_exiting());
    assert_eq!(
        g.source_music.take_commands(),
        vec![SourceMusicCommand::StopAll]
    );
    assert!(
        g.events
            .iter()
            .any(|e| matches!(e, GameEvent::Sound { name, .. } if name == "sndSteps"))
    );
}

#[test]
fn natural_lava_death_repeats_source_stop_after_m_on_without_ankh() {
    let mut g = room();
    // Authored initial hazard; no life, state or velocity injection.
    g.spawn("oLava", Vec2::new(48.0, 128.0));
    g.step(&InputFrame::default());
    assert_eq!(g.state, RunState::Dead);
    assert_eq!(
        g.source_music.take_commands(),
        vec![
            play("musCave", true, true),
            frequency("musCave", 44_100),
            SourceMusicCommand::StopAll
        ]
    );
    g.step(&m());
    assert!(!g.source_music.enabled);
    assert_eq!(
        g.source_music.take_commands(),
        vec![SourceMusicCommand::StopAll, SourceMusicCommand::StopAll]
    );
    g.step(&m());
    assert!(g.source_music.enabled);
    assert_eq!(
        g.source_music.take_commands(),
        vec![play("musCave", true, true), SourceMusicCommand::StopAll]
    );
}

#[test]
fn natural_change_portal_stops_but_change_two_keeps_music() {
    for (kind, unlock, object, stop) in [
        (hub::HubKind::Scores, "unlock_damsel", "oXChange", true),
        (hub::HubKind::Title, "unlock_tunnel_man", "oXChange2", false),
    ] {
        let mut g = Game::legacy_shell(7, RulesPreset::Classic);
        g.flags.insert("title_start:1".into());
        g.flags.insert(unlock.into());
        g.load_hub(kind);
        // Controlled initial authored room position, before any input/Step.
        let door = g
            .world
            .entities
            .iter()
            .find(|e| e.alive && e.object == object)
            .unwrap()
            .pos;
        g.player.pos = Vec2::new(door.x + 8.0, door.y + 8.0);
        g.player.previous_pos = g.player.pos;
        g.player.grounded = true;
        g.source_music.take_commands();
        g.step(&InputFrame {
            up: true,
            ..Default::default()
        });
        assert!(g.player.is_exiting());
        let expected = if stop {
            vec![SourceMusicCommand::StopAll]
        } else {
            vec![]
        };
        assert_eq!(g.source_music.take_commands(), expected, "{object}");
    }
}
