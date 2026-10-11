//! Authored source-crush fixtures for Bat own post-Destroy sound continuation.
//! Neutral public Game.step executes source damage; no HP/state injection.
//! These are bounded initial-contact fixtures, not natural generated routes.
use spelunky_core::*;

fn game(rules: RulesPreset) -> Game {
    let mut game = Game::legacy_shell(42, rules);
    game.player.pos = Vec2::new(216.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game.world.set_tile(13, 10, "oBlock");
    game
}
fn blood_draws(rng: &mut Rng, count: usize) {
    for _ in 0..count {
        let _x1 = rng.unit();
        let _x2 = rng.unit();
        let _y = rng.unit();
        let _gravity = rng.range(6);
    }
}
fn sound_count(game: &Game, sound: &str) -> usize {
    game.events
        .iter()
        .filter(|event| {
            matches!(event,
        GameEvent::Sound { name, .. } if name == sound)
        })
        .count()
}

#[test]
fn fresh_bat_source_crush_still_wakes_after_destroy_if_ceiling_absent() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = game(rules);
        game.world.set_tile(4, 4, "oBlock");
        let bat = game.spawn("oBat", Vec2::new(64.0, 64.0));
        assert_eq!(
            (
                game.world.entity(bat).unwrap().health,
                game.world.entity(bat).unwrap().state
            ),
            (1, 0)
        );
        assert!(game.world.solid_at(72.0, 72.0));
        assert!(!game.world.solid_at(72.0, 63.0));
        let mut expected = game.rng.clone();
        blood_draws(&mut expected, 3);
        game.step(&InputFrame::default());
        assert!(game.world.entity(bat).is_none());
        assert_eq!(game.data.get("bats"), Some(&1.0));
        assert_eq!(game.data.get("kills"), Some(&1.0));
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == "oBlood")
                .count(),
            3
        );
        assert_eq!(
            sound_count(&game, "sndBat"),
            1,
            "POST_DESTROY_HANG_WAKE_SOUND"
        );
        assert_eq!(
            game.rng.clone().next_u64(),
            expected.next_u64(),
            "BAT_TAIL_ADDS_NO_RANDOM_DRAWS"
        );
        game.step(&InputFrame::default());
        assert_eq!(
            sound_count(&game, "sndBat"),
            0,
            "RETIRED_BAT_NEVER_STEPS_AGAIN"
        );
        assert_eq!(game.data.get("kills"), Some(&1.0));
        assert_eq!(game.player.health, 4);
    }
}
#[test]
fn crushed_bat_with_ceiling_and_nonqualifying_player_has_no_wake_sound() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = game(rules);
        game.world.set_tile(4, 3, "oBlock");
        game.world.set_tile(4, 4, "oBlock");
        let bat = game.spawn("oBat", Vec2::new(64.0, 64.0));
        assert!(game.world.solid_at(72.0, 63.0));
        assert!(Vec2::new(72.0, 72.0).distance(game.player.pos) > 90.0);
        let mut expected = game.rng.clone();
        blood_draws(&mut expected, 3);
        game.step(&InputFrame::default());
        assert!(game.world.entity(bat).is_none());
        assert_eq!(sound_count(&game, "sndBat"), 0);
        assert_eq!(game.data.get("kills"), Some(&1.0));
        assert_eq!(game.rng.clone().next_u64(), expected.next_u64());
    }
}
#[test]
fn fresh_live_bat_without_ceiling_keeps_existing_wake_behavior() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = game(rules);
        let bat = game.spawn("oBat", Vec2::new(64.0, 64.0));
        let mut expected = game.rng.clone();
        game.step(&InputFrame::default());
        let actor = game.world.entity(bat).unwrap();
        assert_eq!((actor.health, actor.state), (1, 1));
        assert_eq!(sound_count(&game, "sndBat"), 1);
        assert!(
            !game
                .world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oBlood")
        );
        assert_eq!(game.data.get("kills"), None);
        assert_eq!(game.rng.clone().next_u64(), expected.next_u64());
    }
}
