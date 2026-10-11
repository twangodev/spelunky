//! Source scrUseItem523–528: empty Bow replaces both message lines for80ticks.
//! Real pickup, six draw/release cycles and subsequent input reach the branch.
use spelunky_core::{
    Area, Game, GameEvent, InputFrame, PlayerState, RulesPreset, SourceMessage, Vec2, World,
};

const PRESETS: [RulesPreset; 2] = [RulesPreset::Classic, RulesPreset::Assisted];

fn attack() -> InputFrame {
    InputFrame {
        attack: true,
        attack_pressed: true,
        ..Default::default()
    }
}
fn release() -> InputFrame {
    InputFrame {
        attack_released: true,
        ..Default::default()
    }
}
fn dialogue(game: &Game) -> Option<&SourceMessage> {
    game.events.iter().find_map(|e| {
        if let GameEvent::Dialogue(m) = e {
            Some(m)
        } else {
            None
        }
    })
}

fn empty_bow(rules: RulesPreset) -> (Game, SourceMessage) {
    let mut game = Game::shell(29, rules);
    game.world = World::new(42, 18, Area::Mines, 1);
    game.world.name = "rLevel".into();
    game.player.pos = Vec2::new(72.0, 120.0);
    game.player.previous_pos = game.player.pos;
    game.player.grounded = true;
    game.player.state = PlayerState::Standing;
    game.player.facing = 1;
    for x in 0..42 {
        game.world.set_tile(x, 8, "oHardBlock");
    }
    // Initial authored free-item overrides are literal source editor semantics.
    // Spectacles is strictly nearer; no equal-distance nearest-family premise.
    let spectacles = game.spawn("oSpectacles", Vec2::new(73.0, 120.0));
    let bow = game.spawn("oBow", Vec2::new(77.0, 124.0));
    for id in [spectacles, bow] {
        let e = game.world.entity_mut(id).unwrap();
        e.data.insert("cost".into(), 0.0);
        e.data.insert("forSale".into(), 0.0);
    }
    game.step(&InputFrame {
        down: true,
        ..attack()
    });
    assert!(
        game.player.has("Spectacles"),
        "FIXTURE_REAL_SPECTACLES_PICKUP"
    );
    assert!(game.world.entity(spectacles).is_none());
    let previous = dialogue(&game).unwrap().clone();
    assert_eq!(previous.second, "YOUR EYESIGHT SEEMS IMPROVED...");
    game.step(&InputFrame {
        down: true,
        ..release()
    });
    game.step(&InputFrame {
        down: true,
        ..attack()
    });
    assert_eq!(game.player.held, Some(bow), "FIXTURE_REAL_BOW_PICKUP");
    assert_eq!(game.data["arrows"], 6.0);
    game.step(&release());
    for remaining in (0..6).rev() {
        for _ in 0..16 {
            if game
                .player
                .data
                .get("weapon_cooldown")
                .copied()
                .unwrap_or(0.0)
                == 0.0
            {
                break;
            }
            game.step(&InputFrame::default());
        }
        assert_eq!(
            game.player
                .data
                .get("weapon_cooldown")
                .copied()
                .unwrap_or(0.0),
            0.0
        );
        assert_eq!(game.player.held, Some(bow));
        game.step(&attack());
        assert_eq!(
            game.player.data.get("bow_armed"),
            Some(&1.0),
            "FIXTURE_REAL_BOW_ARM"
        );
        let first_birth = game.world.next_id;
        game.step(&release());
        assert_eq!(
            game.data["arrows"],
            f64::from(remaining),
            "FIXTURE_REAL_ARROW_SPEND"
        );
        assert!(
            game.world
                .entities
                .iter()
                .any(|e| e.id >= first_birth && e.object == "oArrow")
        );
    }
    for _ in 0..16 {
        if game
            .player
            .data
            .get("weapon_cooldown")
            .copied()
            .unwrap_or(0.0)
            == 0.0
        {
            break;
        }
        game.step(&InputFrame::default());
    }
    assert_eq!(
        game.player
            .data
            .get("weapon_cooldown")
            .copied()
            .unwrap_or(0.0),
        0.0
    );
    assert_eq!(game.data["arrows"], 0.0, "FIXTURE_NO_ARROW_RECOVERY");
    assert_eq!(game.player.held, Some(bow));
    assert_eq!(game.state, spelunky_core::RunState::Playing);
    assert!(!game.player.is_stunned());
    (game, previous)
}

fn assert_empty_message(game: &Game) {
    let message = dialogue(game).expect("BOW_EMPTY_SOURCE_DIALOGUE_REQUIRED");
    assert_eq!(message.first, "I'M OUT OF ARROWS!");
    assert_eq!(message.second, "", "BOW_EMPTY_CLEARS_SECOND_LINE");
    assert_eq!(message.ticks, 80);
    assert_eq!(
        game.data.get("message_timer"),
        Some(&80.0),
        "BOW_EMPTY_REFRESHES_80"
    );
    assert!(
        !game
            .events
            .iter()
            .any(|e| matches!(e, GameEvent::Message(text) if text == "I'M OUT OF ARROWS!"))
    );
    assert!(!game.events.iter().any(
        |e| matches!(e, GameEvent::Sound{name,..} if name == "sndBowPull" || name == "sndArrowTrap")
    ));
    assert_eq!(game.data["arrows"], 0.0);
}

#[test]
fn empty_bow_refreshes_live_source_message_and_clears_second_line() {
    for rules in PRESETS {
        let (mut game, previous) = empty_bow(rules);
        assert!(!previous.second.is_empty());
        let prior_timer = game.data.get("message_timer").copied().unwrap_or(0.0);
        assert!(
            prior_timer > 0.0 && prior_timer < 80.0,
            "FIXTURE_LIVE_OLD_DIALOGUE"
        );
        let first_birth = game.world.next_id;
        game.step(&attack());
        assert_empty_message(&game);
        assert!(
            !game
                .world
                .entities
                .iter()
                .any(|e| e.id >= first_birth && e.object == "oArrow")
        );
        game.step(&release());
        assert_eq!(game.data["message_timer"], 79.0);
        for _ in 0..78 {
            game.step(&InputFrame::default());
        }
        assert_eq!(game.data["message_timer"], 1.0);
        game.step(&InputFrame::default());
        assert_eq!(game.data["message_timer"], 0.0);
    }
}

#[test]
fn empty_bow_after_expiry_uses_simulation_timer_not_host_five_second_fallback() {
    for rules in PRESETS {
        let (mut game, _) = empty_bow(rules);
        for _ in 0..121 {
            if game.data.get("message_timer").copied().unwrap_or(0.0) == 0.0 {
                break;
            }
            game.step(&InputFrame::default());
        }
        assert_eq!(game.data.get("message_timer"), Some(&0.0));
        assert_eq!(game.data["arrows"], 0.0, "FIXTURE_NO_LATE_ARROW_RECOVERY");
        game.step(&attack());
        assert_empty_message(&game);
        game.step(&release());
        assert_eq!(game.data["message_timer"], 79.0);
        for _ in 0..78 {
            game.step(&InputFrame::default());
        }
        assert_eq!(game.data["message_timer"], 1.0);
        game.step(&InputFrame::default());
        assert_eq!(game.data["message_timer"], 0.0);
    }
}
