//! Initial authored Spectacles/Mattock/altar, followed only by normal inputs.
use spelunky_core::{Area, Game, GameEvent, InputFrame, PlayerState, RulesPreset, Vec2, World};
#[test]
fn ordinary_mattock_altar_destroy_replaces_prior_pickup_dialogue_with_both_source_lines() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = Game::shell(29, rules);
        game.world = World::new(42, 18, Area::Mines, 1);
        game.world.name = "rLevel".into();
        for x in 0..42 {
            game.world.set_tile(x, 10, "oHardBlock");
        }
        game.player.pos = Vec2::new(136., 152.);
        game.player.previous_pos = game.player.pos;
        game.player.state = PlayerState::Standing;
        game.player.grounded = true;
        game.player.facing = 1;
        let altar = game.spawn("oSacAltarLeft", Vec2::new(144., 144.));
        let spectacles = game.spawn("oSpectacles", Vec2::new(137., 152.));
        let mattock = game.spawn("oMattock", Vec2::new(140., 152.));
        for id in [spectacles, mattock] {
            let e = game.world.entity_mut(id).unwrap();
            e.data.insert("cost".into(), 0.);
            e.data.insert("forSale".into(), 0.);
        }
        let pickup = InputFrame {
            down: true,
            attack: true,
            attack_pressed: true,
            ..Default::default()
        };
        game.step(&pickup);
        assert!(
            game.player.has("Spectacles"),
            "NATURAL_PRIOR_MESSAGE_PICKUP"
        );
        assert!(game.events.iter().any(
            |e| matches!(e,GameEvent::Dialogue(m) if m.second=="YOUR EYESIGHT SEEMS IMPROVED...")
        ));
        game.step(&InputFrame::default());
        game.step(&pickup);
        assert_eq!(game.player.held, Some(mattock), "NATURAL_MATTOCK_PICKUP");
        game.step(&InputFrame::default());
        assert!(
            game.data["message_timer"] > 0.,
            "PRIOR_SOURCE_MESSAGE_LIVE_BEFORE_DEFILE"
        );
        assert!(game.world.entity(altar).is_some());
        game.step(&InputFrame {
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        let mut destroyed = false;
        for _ in 0..60 {
            game.step(&InputFrame::default());
            if game.world.entity(altar).is_none() {
                destroyed = true;
                break;
            }
        }
        assert!(destroyed, "ORDINARY_MATTOCK_ANIMATION_END_DEFILES_ALTAR");
        let message = game
            .events
            .iter()
            .find_map(|e| {
                if let GameEvent::Dialogue(m) = e {
                    (m.first == "YOU DARE DEFILE MY ALTAR?").then_some(m)
                } else {
                    None
                }
            })
            .expect("ALTAR_DESTROY_TYPED_TWO_LINE_DIALOGUE_REQUIRED");
        assert_eq!(message.second, "I WILL PUNISH YOU!");
        assert_eq!(message.ticks, 200);
        // MattockHit AnimationEnd follows Draw, so this new200 is not consumed yet.
        assert_eq!(game.data["message_timer"], 200.);
        assert_eq!(game.data["favor"], -16.);
        assert_eq!(game.data["kali_punish"], 1.);
        assert!(
            !game
                .events
                .iter()
                .any(|e| matches!(e,GameEvent::Message(s) if s.contains("DEFILE MY ALTAR")))
        );
        game.step(&InputFrame::default());
        assert_eq!(game.presentation().data["source_message_begin_timer"], 200.);
        assert_eq!(game.presentation().data["source_message_draw_timer"], 199.);
        assert_eq!(game.data["message_timer"], 198.);
    }
}
