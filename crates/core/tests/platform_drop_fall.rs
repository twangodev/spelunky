//! Input-only authored falls distinguish a fresh platform drop from an earlier
//! long fall. No Player state, velocity, health, or timer is injected.
use spelunky_core::*;

fn fall(game: &Game) -> f64 {
    game.player.data.get("fall_timer").copied().unwrap_or(0.0)
}

fn diagnostic(game: &Game) -> String {
    format!(
        "tick={} pos={:?} vel={:?} state={:?} grounded={} hp={} stun={} fall={}",
        game.tick,
        game.player.pos,
        game.player.vel,
        game.player.state,
        game.player.grounded,
        game.player.health,
        game.player.stun,
        fall(game)
    )
}

fn room(rules: RulesPreset, platform_top: u32) -> Game {
    let mut level = level::Level::default();
    level.set_tile(8, 4, '@').unwrap();
    for x in 6..=10 {
        level.set_tile(x, 5, '1').unwrap();
    }
    // Custom cell coordinates have the source one-cell room border. This
    // second landing is only64px below the one-way platform.
    let lower_row = ((platform_top + 64) / 16 - 1) as usize;
    for x in 0..40 {
        level.set_tile(x, lower_row, '1').unwrap();
    }
    let mut game = Game::shell(42, rules);
    game.load_custom(&level, "fresh-platform-drop", "");
    for x in 1..41 {
        game.spawn(
            "oTreeBranchTile",
            Vec2::new(x as f64 * 16.0, f64::from(platform_top)),
        );
    }
    game
}

fn first_contact(game: &mut Game, platform_top: u32) {
    for _ in 0..4 {
        game.step(&InputFrame::default());
    }
    assert_eq!(
        game.player.pos,
        Vec2::new(152.0, 88.0),
        "INITIAL: {}",
        diagnostic(game)
    );
    assert!(game.player.grounded, "INITIAL: {}", diagnostic(game));
    assert_eq!(fall(game), 0.0);
    // Source solid-floor Down remains a crouch, without starting a platform
    // drop. Release before walking, so only the later contact gets freshDown.
    game.step(&InputFrame {
        down: true,
        ..Default::default()
    });
    assert_eq!(
        game.player.state,
        PlayerState::Ducking,
        "SOLID_CONTROL: {}",
        diagnostic(game)
    );
    assert_eq!(game.player.pos.y, 88.0);
    game.step(&InputFrame::default());
    let mut left_support = false;
    for _ in 0..30 {
        game.step(&InputFrame {
            right: true,
            ..Default::default()
        });
        if !game.player.grounded && game.player.state == PlayerState::Falling {
            left_support = true;
            break;
        }
    }
    assert!(left_support, "WALK_OFF: {}", diagnostic(game));
    for _ in 0..60 {
        if game.player.grounded {
            break;
        }
        game.step(&InputFrame::default());
        assert_eq!(
            game.player.health,
            4,
            "BEFORE_CONTACT: {}",
            diagnostic(game)
        );
        assert!(
            !game.player.is_stunned(),
            "BEFORE_CONTACT: {}",
            diagnostic(game)
        );
    }
    assert!(game.player.grounded, "FIRST_CONTACT: {}", diagnostic(game));
    assert_eq!(
        game.player.pos.y + 8.0,
        f64::from(platform_top),
        "FIRST_CONTACT: {}",
        diagnostic(game)
    );
    assert_eq!(
        game.player.state,
        PlayerState::Falling,
        "FIRST_CONTACT: {}",
        diagnostic(game)
    );
    assert!(
        game.player.vel.y > 0.0,
        "FIRST_CONTACT: {}",
        diagnostic(game)
    );
    assert_eq!(game.player.health, 4);
    assert!(!game.player.is_stunned());
    assert!(fall(game) > 0.0);
    if platform_top == 288 {
        assert!(fall(game) > 16.0, "LONG_FIRST_FALL: {}", diagnostic(game));
    } else {
        assert!(
            fall(game) + 1.0 <= 16.0,
            "SHORT_FIRST_FALL: {}",
            diagnostic(game)
        );
    }
}

fn fresh_down(game: &mut Game, platform_top: u32) {
    game.step(&InputFrame {
        down: true,
        ..Default::default()
    });
    assert_eq!(
        game.player.state,
        PlayerState::Falling,
        "ACTUAL_DROP: {}",
        diagnostic(game)
    );
    assert!(
        game.player.pos.y + 8.0 > f64::from(platform_top),
        "ACTUAL_DROP: {}",
        diagnostic(game)
    );
    assert!(game.player.vel.y > 0.0);
    assert_eq!(game.player.health, 4);
    assert!(!game.player.is_stunned());
}

fn lower_landing(game: &mut Game, platform_top: u32) {
    let lower_top = f64::from(platform_top + 64);
    for _ in 0..60 {
        game.step(&InputFrame::default());
        if game.player.health < 4 || game.player.is_stunned() {
            break;
        }
        if game.player.grounded && game.player.vel.y == 0.0 && fall(game) == 0.0 {
            break;
        }
    }
    assert_eq!(
        game.player.health,
        4,
        "SHORT_DROP_HEALTH: {}",
        diagnostic(game)
    );
    assert!(
        !game.player.is_stunned(),
        "SHORT_DROP_STUN: {}",
        diagnostic(game)
    );
    assert!(game.player.grounded, "LOWER_LANDING: {}", diagnostic(game));
    assert_eq!(
        game.player.pos.y + 8.0,
        lower_top,
        "LOWER_LANDING: {}",
        diagnostic(game)
    );
    assert_eq!(fall(game), 0.0, "LOWER_SETTLED: {}", diagnostic(game));
    let y = game.player.pos.y;
    game.step(&InputFrame {
        jump: true,
        jump_pressed: true,
        ..Default::default()
    });
    assert!(
        game.player.vel.y < 0.0 && game.player.pos.y < y,
        "FRESH_JUMP: {}",
        diagnostic(game)
    );
    assert_eq!(game.player.health, 4);
}

#[test]
fn natural_first_platform_contact_down_resets_fall_before_a_fresh_short_drop() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for top in [288, 176] {
            let mut game = room(rules, top);
            first_contact(&mut game, top);
            let previous = fall(&game);
            fresh_down(&mut game, top);
            assert_eq!(
                fall(&game),
                0.0,
                "DROP_RESET previous={previous}: {}",
                diagnostic(&game)
            );
            lower_landing(&mut game, top);
        }
    }
}

#[test]
fn natural_short_platform_drop_does_not_inherit_the_previous_long_fall_injury() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        // Do not use the immediate timer assertion here: follow the complete
        // route to expose incorrect health/stun at the later short landing.
        let mut game = room(rules, 288);
        first_contact(&mut game, 288);
        fresh_down(&mut game, 288);
        lower_landing(&mut game, 288);
        // A long fall settled on the upper platform withoutDown still incurs
        // Classic's real one-health/60-tick injury; the fix cannot suppress it.
        let mut control = room(rules, 288);
        first_contact(&mut control, 288);
        control.step(&InputFrame::default());
        assert_eq!(
            control.player.health,
            4,
            "LANDING_CONTROL: {}",
            diagnostic(&control)
        );
        assert!(fall(&control) > 16.0);
        assert!(
            fall(&control) <= 32.0,
            "ONE_HEALTH_CONTROL: {}",
            diagnostic(&control)
        );
        control.step(&InputFrame::default());
        assert_eq!(
            control.player.health,
            3,
            "LONG_FALL_CONTROL: {}",
            diagnostic(&control)
        );
        assert!(control.player.is_stunned());
        assert_eq!(control.player.stun, 60);
    }
}
