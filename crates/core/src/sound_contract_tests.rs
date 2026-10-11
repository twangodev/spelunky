use crate::*;

fn room(area: Area, level: u32) -> Game {
    let mut g = Game::legacy_shell(7, RulesPreset::Classic);
    g.world = World::new(20, 12, area, level);
    for x in 0..20 {
        g.world.set_tile(x, 9, "oBlock");
    }
    g.player.pos = Vec2::new(56.0, 136.0);
    g
}

#[test]
fn actual_input_bow_release_stops_only_bow_pull_after_arrow_sound() {
    let mut g = room(Area::Mines, 2);
    let bow = g.spawn("oBow", g.player.pos);
    g.step(&InputFrame {
        down: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert_eq!(g.player.held, Some(bow), "real pickup must precede drawing");
    assert_eq!(
        g.data["arrows"], 6.0,
        "source first pickup grants six arrows"
    );
    g.step(&InputFrame {
        attack_released: true,
        ..Default::default()
    });
    g.step(&InputFrame {
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert!(
        g.events
            .iter()
            .any(|e| matches!(e,GameEvent::Sound{name,..} if name=="sndBowPull"))
    );
    for _ in 0..3 {
        g.step(&InputFrame {
            attack: true,
            ..Default::default()
        });
    }
    g.step(&InputFrame {
        attack_released: true,
        ..Default::default()
    });
    assert_eq!(g.data["arrows"], 5.0);
    assert!(g.world.entities.iter().any(|e| e.object == "oArrow"));
    let fire = g
        .events
        .iter()
        .position(|e| matches!(e,GameEvent::Sound{name,..} if name=="sndArrowTrap"))
        .unwrap();
    let stop = g
        .events
        .iter()
        .position(|e| matches!(e,GameEvent::SoundStop{name} if name=="sndBowPull"))
        .expect("fire must stop the long draw clip");
    assert!(fire < stop);
    assert_eq!(
        g.events
            .iter()
            .filter(|e| matches!(e, GameEvent::SoundStop { .. }))
            .count(),
        1
    );
}

#[test]
fn source_bow_missing_hold_cancel_retains_charging_gate() {
    // Local source event control: no held instance, armed draw at entry.
    for (held, strength, expected) in [(true, 0.0, true), (false, 0.0, false), (true, 12.0, false)]
    {
        let mut g = room(Area::Mines, 2);
        g.player.data.insert("bow_armed".into(), 1.0);
        g.player.data.insert("bow_strength".into(), strength);
        g.player.data.insert("attack_held".into(), f64::from(held));
        crate::items::weapons::player_step(&mut g);
        assert_eq!(
            g.events
                .iter()
                .any(|e| matches!(e,GameEvent::SoundStop{name} if name=="sndBowPull")),
            expected
        );
    }
}

#[test]
fn actual_damsel_help_sets_source_pan_before_sound_for_both_sides_and_equal() {
    for (player_x, expected) in [(80.0, -1.0), (160.0, 1.0), (120.0, 0.0)] {
        let mut g = room(Area::Mines, 2);
        g.player.pos.x = player_x;
        let damsel = g.spawn("oDamsel", Vec2::new(120.0, 136.0));
        let mut observed = false;
        for _ in 0..205 {
            g.step(&InputFrame::default());
            if let Some(i) = g
                .events
                .iter()
                .position(|e| matches!(e,GameEvent::Sound{name,..} if name=="sndDamsel"))
            {
                assert!(
                    matches!(g.events.get(i.wrapping_sub(1)),Some(GameEvent::SoundPan{name,pan}) if name=="sndDamsel" && *pan==expected)
                );
                assert_eq!(g.world.entity(damsel).unwrap().state, 3);
                observed = true;
                break;
            }
        }
        assert!(
            observed,
            "ordinary idle counter must naturally reach help call"
        );
    }
}

#[test]
fn actual_falling_damsel_centers_named_sound_before_thrown_call() {
    let mut g = room(Area::Mines, 2);
    let damsel = g.spawn("oDamsel", Vec2::new(120.0, 64.0));
    let mut observed = false;
    for _ in 0..25 {
        g.step(&InputFrame::default());
        if let Some(i) = g
            .events
            .iter()
            .position(|e| matches!(e,GameEvent::Sound{name,..} if name=="sndDamsel"))
        {
            assert!(
                matches!(g.events.get(i.wrapping_sub(1)),Some(GameEvent::SoundPan{name,pan}) if name=="sndDamsel" && *pan==0.0)
            );
            assert_eq!(g.world.entity(damsel).unwrap().state, 2);
            observed = true;
            break;
        }
    }
    assert!(observed, "natural gravity must trigger thrown sound");
}

#[test]
fn actual_level_clock_warns_then_lowers_only_cave_music_for_one_hundred_steps() {
    let mut g = room(Area::Mines, 2);
    for i in 1..=4000 {
        g.step(&InputFrame::default());
        assert!(
            !g.flags.contains("ghost_warning"),
            "warning before source threshold at {i}"
        );
    }
    assert_eq!(g.data["xtime"], 120000.0);
    for n in 1..=100 {
        let tick = g.tick;
        g.step(&InputFrame::default());
        assert_eq!(g.tick, tick + 1, "rate must not alter simulation time");
        assert_eq!(g.data["music_fade_timer"], f64::from(n));
        assert_eq!(
            g.events
                .iter()
                .filter_map(|e| if let GameEvent::MusicFrequency { name, hz } = e {
                    Some((name.as_str(), *hz))
                } else {
                    None
                })
                .collect::<Vec<_>>(),
            vec![("musCave", 44100 - n * 100)]
        );
    }
    for _ in 0..3 {
        g.step(&InputFrame::default());
        assert!(
            !g.events
                .iter()
                .any(|e| matches!(e, GameEvent::MusicFrequency { .. }))
        );
    }
    assert_eq!(g.data["music_frequency"], 34100.0);
}

#[test]
fn actual_warning_controls_and_room_reset_preserve_source_frequency_ownership() {
    for (area, level, music) in [
        (Area::Mines, 2, "musCave"),
        (Area::Jungle, 5, "musLush"),
        (Area::Ice, 9, "musIce"),
        (Area::Temple, 13, "musTemple"),
    ] {
        let mut g = room(area, level);
        // Initial authored clock is a boundary control; subsequent frames are ordinary.
        g.data.insert("xtime".into(), 120000.0);
        g.step(&InputFrame::default());
        let requests = g
            .events
            .iter()
            .filter_map(|e| {
                if let GameEvent::MusicFrequency { name, hz } = e {
                    Some((name.as_str(), *hz))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        assert_eq!(requests, vec![(music, 44100), (music, 44000)]);
        crate::progression::initialize_level(&mut g);
        assert_eq!(
            g.data["music_frequency"], 44000.0,
            "idempotent initialization cannot restart ramp"
        );
        g.next_level();
        g.step(&InputFrame::default());
        assert_eq!(g.data["music_frequency"], 44100.0);
        assert_eq!(g.data["music_fade_timer"], 0.0);
    }
    for control in ["crown", "first", "custom", "boss", "exit"] {
        let mut g = room(Area::Mines, 2);
        g.data.insert("xtime".into(), 120000.0);
        match control {
            "crown" => {
                g.player.equipment.insert("Crown".into());
            }
            "first" => g.world.level = 1,
            "custom" => {
                g.flags.insert("custom".into());
            }
            "boss" => g.world.area = Area::Boss,
            "exit" => g.player.set_sprite("sPExit"),
            _ => unreachable!(),
        }
        g.step(&InputFrame::default());
        assert!(!g.flags.contains("music_fade"), "{control}");
        assert!(
            !g.events
                .iter()
                .any(|e| matches!(e,GameEvent::MusicFrequency{hz,..} if *hz!=44100)),
            "{control}"
        );
    }
}
