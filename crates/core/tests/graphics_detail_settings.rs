use spelunky_core::{Game, RulesPreset, SourceMusicState};

#[test]
fn graphics_preference_is_applied_before_the_first_campaign_room_generates() {
    let high = Game::new_with_music_and_graphics(
        41,
        RulesPreset::Classic,
        SourceMusicState::default(),
        true,
    );
    let low = Game::new_with_music_and_graphics(
        41,
        RulesPreset::Classic,
        SourceMusicState::default(),
        false,
    );

    let extras = |game: &Game| {
        game.world
            .entities
            .iter()
            .filter(|entity| entity.sprite.starts_with("bgExtras"))
            .count()
    };
    assert!(extras(&high) >= 20, "Classic high detail includes bgExtras");
    assert_eq!(extras(&low), 0, "Classic low detail omits bgExtras");
    assert!(!high.flags.contains("low_graphics"));
    assert!(low.flags.contains("low_graphics"));

    let mut toggled = low;
    toggled.set_graphics_high(true);
    assert!(!toggled.flags.contains("low_graphics"));
    toggled.set_graphics_high(false);
    assert!(toggled.flags.contains("low_graphics"));
}
