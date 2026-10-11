//! Authored sign metadata then only ordinary pickup/walking Game.step inputs.
use spelunky_core::{
    Area, Game, GameEvent, InputFrame, PlayerState, RulesPreset, SourceMessage, Vec2, World,
};
fn fixture(rules: RulesPreset, pickup: bool) -> Game {
    let mut game = Game::shell(29, rules);
    game.world = World::new(42, 18, Area::Mines, 1);
    game.world.name = "rLevel".into();
    for x in 0..42 {
        game.world.set_tile(x, 8, "oHardBlock");
    }
    game.player.pos = Vec2::new(72., 120.);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    for (x, second) in [(64., "FIRST SECOND"), (112., "SECOND SECOND")] {
        let id = game.spawn("oMsgSign", Vec2::new(x, 112.));
        let sign = game.world.entity_mut(id).unwrap();
        sign.text.insert("message".into(), String::new());
        sign.text.insert("message2".into(), second.into());
    }
    if pickup {
        let id = game.spawn("oSpectacles", Vec2::new(73., 120.));
        let item = game.world.entity_mut(id).unwrap();
        item.data.insert("cost".into(), 0.);
        item.data.insert("forSale".into(), 0.);
    }
    game
}
fn messages(game: &Game) -> Vec<&SourceMessage> {
    game.events
        .iter()
        .filter_map(|e| {
            if let GameEvent::Dialogue(m) = e {
                Some(m)
            } else {
                None
            }
        })
        .collect()
}
#[test]
fn natural_pickup_then_sign_collision_assigns_both_lines_and_repeats_without_cooldown() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = fixture(rules, true);
        game.step(&InputFrame {
            down: true,
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert!(
            game.player.has("Spectacles"),
            "REAL_PICKUP_BEFORE_SIGN_COLLISION"
        );
        let assigned = messages(&game);
        assert_eq!(
            assigned.len(),
            2,
            "SOURCE_PICKUP_THEN_SIGN_TYPED_ASSIGNMENTS"
        );
        assert_eq!(assigned[0].ticks, 120);
        assert_eq!(assigned[0].second, "YOUR EYESIGHT SEEMS IMPROVED...");
        assert_eq!(
            assigned[1],
            &SourceMessage {
                first: String::new(),
                second: "FIRST SECOND".into(),
                ticks: 200
            }
        );
        assert_eq!(game.presentation().data["source_message_draw_timer"], 200.);
        assert_eq!(game.data["message_timer"], 199.);
        assert!(
            !game
                .events
                .iter()
                .any(|e| matches!(e, GameEvent::Message(_)))
        );
        for _ in 0..3 {
            game.step(&InputFrame::default());
            assert_eq!(
                messages(&game),
                vec![&SourceMessage {
                    first: String::new(),
                    second: "FIRST SECOND".into(),
                    ticks: 200
                }],
                "EACH_SOURCE_COLLISION_ASSIGNMENT_REQUIRED"
            );
            assert_eq!(game.data["message_timer"], 199.);
        }
    }
}
#[test]
fn ordinary_walk_to_other_sign_and_back_replaces_empty_first_line_without_timer_injection() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = fixture(rules, false);
        game.step(&InputFrame::default());
        assert_eq!(messages(&game).last().unwrap().second, "FIRST SECOND");
        let mut reached = false;
        for _ in 0..40 {
            game.step(&InputFrame {
                right: true,
                ..Default::default()
            });
            if messages(&game)
                .last()
                .is_some_and(|m| m.second == "SECOND SECOND")
            {
                reached = true;
                break;
            }
        }
        assert!(reached, "ORDINARY_WALK_REACHES_SECOND_SIGN");
        let mut returned = false;
        for _ in 0..40 {
            game.step(&InputFrame {
                left: true,
                ..Default::default()
            });
            if messages(&game)
                .last()
                .is_some_and(|m| m.second == "FIRST SECOND")
            {
                returned = true;
                break;
            }
        }
        assert!(returned, "ORDINARY_WALK_RETURNS_TO_FIRST_SIGN");
        let final_message = messages(&game).last().unwrap().clone();
        assert!(final_message.first.is_empty());
        assert_eq!(final_message.ticks, 200);
        assert_eq!(game.data["message_timer"], 199.);
    }
}
