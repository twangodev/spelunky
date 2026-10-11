//! Initial authored floor/free Spectacles; all subsequent state via Game.step.
use spelunky_core::{Area, Game, GameEvent, InputFrame, PlayerState, RulesPreset, Vec2, World};
fn producer(dark: bool, rules: RulesPreset) -> Game {
    let mut game = Game::shell(29, rules);
    game.world = World::new(42, 18, Area::Mines, 1);
    game.world.name = "rLevel".into();
    game.world.dark = dark;
    for x in 0..42 {
        game.world.set_tile(x, 8, "oHardBlock");
    }
    game.player.pos = Vec2::new(72., 120.);
    game.player.previous_pos = game.player.pos;
    game.player.grounded = true;
    game.player.state = PlayerState::Standing;
    let item = game.spawn("oSpectacles", Vec2::new(73., 120.));
    let item = game.world.entity_mut(item).unwrap();
    item.data.insert("cost".into(), 0.);
    item.data.insert("forSale".into(), 0.);
    game.step(&InputFrame {
        down: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert!(game.player.has("Spectacles"));
    assert!(game.events.iter().any(|e| matches!(e, GameEvent::Dialogue(m) if m.ticks==120 && m.second=="YOUR EYESIGHT SEEMS IMPROVED...")));
    game
}
#[test]
fn real_pickup_bright_message_has_two_source_consumers_and_final_begin_frame() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = producer(false, rules);
        assert_eq!(
            game.data["message_timer"], 119.,
            "BRIGHT_DRAW_CONSUMES_AFTER_PRODUCER"
        );
        assert_eq!(
            game.presentation().data["source_message_begin_timer"],
            0.,
            "BEGIN_PRECEDES_PICKUP"
        );
        assert_eq!(game.presentation().data["source_message_draw_timer"], 120.);
        for n in 1..=59 {
            game.step(&InputFrame::default());
            assert_eq!(game.data["message_timer"], 119. - f64::from(n) * 2.);
        }
        assert_eq!(game.data["message_timer"], 1.);
        game.step(&InputFrame::default());
        assert_eq!(game.data["message_timer"], 0.);
        assert_eq!(
            game.presentation().data["source_message_begin_timer"],
            1.,
            "FINAL_BEGIN_REFRESH_SURVIVES_ZERO_GLOBAL"
        );
        assert_eq!(game.presentation().data["source_message_draw_timer"], 0.);
        game.step(&InputFrame::default());
        assert_eq!(game.presentation().data["source_message_begin_timer"], 0.);
    }
}
#[test]
fn real_pickup_dark_message_has_only_begin_consumer_and_render_reads_are_inert() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = producer(true, rules);
        assert_eq!(game.data["message_timer"], 120.);
        assert_eq!(game.presentation().data["source_message_begin_timer"], 0.);
        for n in 1..=120 {
            game.step(&InputFrame::default());
            assert_eq!(game.data["message_timer"], 120. - f64::from(n));
            assert_eq!(
                game.presentation().data["source_message_begin_timer"],
                121. - f64::from(n)
            );
            assert_eq!(game.presentation().data["source_message_draw_timer"], 0.);
        }
        let hash = game.state_hash();
        let rng = game.rng.clone().next_u64();
        for _ in 0..240 {
            assert_eq!(game.presentation().data["source_message_begin_timer"], 1.);
        }
        assert_eq!(game.state_hash(), hash);
        assert_eq!(game.rng.clone().next_u64(), rng);
        game.step(&InputFrame::default());
        assert_eq!(game.presentation().data["source_message_begin_timer"], 0.);
    }
}
