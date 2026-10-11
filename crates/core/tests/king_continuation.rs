//! Explicit initial King status/image/HP inputs, followed by actual Game.step.
//! These source branch controls are distinct from the ordinary Pistol witness.
use spelunky_core::*;

fn rand(rng: &mut Rng, width: u64) -> u32 {
    let algorithm = rng.algorithm();
    let raw = rng.next_u64();
    match algorithm {
        RngAlgorithm::Gm8Lcg32V1 => ((raw * width) >> 32) as u32,
        RngAlgorithm::LegacySplitMix64V1 => (raw % width) as u32,
    }
}

fn particle(rng: &mut Rng) {
    for _ in 0..3 {
        rng.next_u64();
    }
    rand(rng, 6);
}

fn death_prefix(rng: &mut Rng) {
    rand(rng, 5);
    rand(rng, 5);
    for _ in 0..3 {
        particle(rng);
    }
    for _ in 0..4 {
        rand(rng, 5);
        rand(rng, 7);
        particle(rng);
    }
    for _ in 0..4 {
        if rand(rng, 12) != 0 {
            rand(rng, 2);
        }
        rand(rng, 4);
        rand(rng, 4);
        rand(rng, 2);
    }
}

fn room(rules: RulesPreset, health: i32, frame: f64) -> (Game, u64, u64, u64) {
    room_with_seed(rules, health, frame, 42)
}

fn room_with_seed(rules: RulesPreset, health: i32, frame: f64, seed: u32) -> (Game, u64, u64, u64) {
    let mut game = Game::shell(seed, rules);
    game.world = World::new(60, 30, Area::Ice, 9);
    game.player.pos = Vec2::new(40.0, 152.0);
    game.player.previous_pos = game.player.pos;
    for x in 0..60 {
        game.world.set_tile(x, 10, "oBlock");
    }
    let king = game.spawn("oYetiKing", Vec2::new(96.0, 128.0));
    let receiver = game.world.entity_mut(king).unwrap();
    receiver.health = health;
    receiver.state = 3;
    receiver.animation = frame;
    receiver.vel = Vec2::default();
    receiver.set_sprite("sYetiKingYellR");
    let thin = game.spawn("oThinIce", Vec2::new(240.0, 64.0));
    let inactive_thin = game.spawn("oThinIce", Vec2::new(704.0, 64.0));
    (game, king, thin, inactive_thin)
}

#[test]
fn actual_step_king_own_death_continues_exact_yell_frame_window_and_active_thin_ice_pass() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for frame in [6.5, 7.0, 8.0, 12.0, 12.5] {
            let (mut game, king, thin, inactive_thin) = room(rules, 0, frame);
            let mut expected = game.rng.clone();
            death_prefix(&mut expected);
            let yelling = (7.0..=12.0).contains(&frame);

            game.step(&InputFrame::default());

            assert!(game.world.entity(king).is_none());
            assert_eq!(
                game.world.entity(thin).unwrap().value("thickness"),
                if yelling { 58.0 } else { 60.0 }
            );
            assert_eq!(
                game.world.entity(inactive_thin).unwrap().value("thickness"),
                60.0
            );
            assert!(!game.world.entity(inactive_thin).unwrap().engine_active);
            assert_eq!(
                game.events
                    .iter()
                    .filter(|event| matches!(event, GameEvent::SoundIfIdle { name, .. } if name == "sndYetiYell"))
                    .count(),
                usize::from(yelling)
            );
            assert_eq!(game.rng.clone().next_u64(), expected.clone().next_u64());
        }
    }
}

#[test]
fn actual_step_live_king_yell_control_keeps_thin_ice_effect_without_death_prefix() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, king, thin, _) = room(rules, 30, 8.0);
        let expected = game.rng.clone();
        game.step(&InputFrame::default());
        assert_eq!(game.world.entity(king).unwrap().health, 30);
        assert_eq!(game.world.entity(thin).unwrap().value("thickness"), 58.0);
        assert!(game.events.iter().any(
            |event| matches!(event, GameEvent::SoundIfIdle { name, .. } if name == "sndYetiYell")
        ));
        assert!(!game.world.entities.iter().any(|e| e.object == "oBone"));
        assert_eq!(game.rng.clone().next_u64(), expected.clone().next_u64());
    }
}

#[test]
fn actual_step_last_retired_king_still_weakens_thin_ice_without_ice_rng_or_births() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, king, thin, _) = room(rules, 0, 8.0);
        let ice = game.spawn("oIce", Vec2::new(176.0, 128.0));
        let frozen = game.spawn("oFrozenCaveman", Vec2::new(176.0, 128.0));
        let mut expected = game.rng.clone();
        death_prefix(&mut expected);

        game.step(&InputFrame::default());

        assert!(game.world.entity(king).is_none());
        assert!(game.world.entity(ice).is_some());
        assert!(game.world.entity(frozen).is_some());
        assert!(
            !game
                .world
                .entities
                .iter()
                .any(|e| matches!(e.object.as_str(), "oIceBlock" | "oDrip" | "oCaveman"))
        );
        assert_eq!(game.world.entity(thin).unwrap().value("thickness"), 58.0);
        assert!(game.events.iter().any(
            |event| matches!(event, GameEvent::SoundIfIdle { name, .. } if name == "sndYetiYell")
        ));
        assert_eq!(game.rng.clone().next_u64(), expected.clone().next_u64());
    }
}

#[test]
fn actual_step_retired_king_yell_uses_first_surviving_king_then_completes_ice_frozen_births() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut selected = None;
        for seed in 0..128 {
            let (mut game, king, thin, _) = room_with_seed(rules, 0, 8.0, seed);
            let other = game.spawn("oYetiKing", Vec2::new(304.0, 128.0));
            let live = game.world.entity_mut(other).unwrap();
            live.state = 0;
            live.data.insert("counter".into(), 999.0);
            let ice = game.spawn("oIce", Vec2::new(240.0, 128.0));
            let frozen = game.spawn("oFrozenCaveman", Vec2::new(240.0, 128.0));
            let mut expected = game.rng.clone();
            death_prefix(&mut expected);
            if rand(&mut expected, 60) != 0 {
                continue;
            }
            let mut drips = Vec::new();
            for _ in 0..3 {
                drips.push(Vec2::new(
                    240.0 + f64::from(rand(&mut expected, 17)),
                    128.0 + f64::from(rand(&mut expected, 17)),
                ));
            }
            selected = Some((game, king, other, thin, ice, frozen, expected, drips));
            break;
        }
        let (mut game, king, other, thin, ice, frozen, expected, drips) =
            selected.expect("bounded initial-only successful source Ice roll");
        let serial = game.world.creation_serial;

        game.step(&InputFrame::default());

        assert!(game.world.entity(king).is_none());
        assert!(game.world.entity(other).is_some());
        assert!(
            game.world.entity(ice).is_none(),
            "qualified query selects the surviving King, not the retired event receiver"
        );
        assert!(
            game.world.entity(frozen).is_none(),
            "Ice Destroy finishes the selected FrozenCaveman Destroy synchronously"
        );
        assert_eq!(game.world.entity(thin).unwrap().value("thickness"), 58.0);
        let mut births: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|e| {
                e.creation_serial > serial
                    && matches!(e.object.as_str(), "oIceBlock" | "oDrip" | "oCaveman")
            })
            .collect();
        births.sort_by_key(|e| e.creation_serial);
        assert_eq!(births.len(), 5);
        assert_eq!(births[0].object, "oIceBlock");
        for (actual, pos) in births[1..4].iter().zip(drips) {
            assert_eq!(actual.object, "oDrip");
            assert_eq!(actual.pos, pos);
        }
        assert_eq!(births[4].object, "oCaveman");
        assert_eq!(births[4].pos, Vec2::new(240.0, 128.0));
        assert_eq!(births[4].state, 98);
        assert_eq!(births[4].timer, 200);
        assert_eq!(births[4].value("invincible"), 20.0);
        assert_eq!(game.rng.clone().next_u64(), expected.clone().next_u64());
    }
}
