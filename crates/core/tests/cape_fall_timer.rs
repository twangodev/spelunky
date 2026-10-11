//! Tests-only SOURCE proposal, UNCOMPILED/UNRUN.
//! Initial authored room + one real CapePickup; subsequent changes are solely
//! ordinary Game.step(inputs). No equipment/state/velocity/RNG injection.
use spelunky_core::*;

fn value(game: &Game, key: &str) -> f64 {
    game.player.data.get(key).copied().unwrap_or(0.0)
}

fn live_fall(game: &Game) {
    assert_eq!(game.state, RunState::Playing);
    assert_eq!(game.player.health, 4);
    assert!(!game.player.is_stunned());
    assert_eq!(game.player.state, PlayerState::Falling);
    assert!(!game.player.grounded);
    assert!(game.player.vel.y > 0.0);
    assert_ne!(value(game, "jumped"), 0.0);
    assert!(game.player.has("Cape"));
}

fn authored_cape(rules: RulesPreset) -> (Game, u64) {
    let mut level = level::Level::default();
    level.set_tile(7, 4, '@').unwrap(); // Extra border gives entrance136,88.
    for x in 7..=8 {
        level.set_tile(x, 5, '1').unwrap(); // Floor128..159, top96.
    }
    for x in 0..40 {
        level.set_tile(x, 24, '1').unwrap(); // Lower floor400, ample fall space.
    }
    let mut game = Game::shell(42, rules);
    game.load_custom(&level, "natural-cape-fall-counter", "");
    let cape = game.spawn("oCapePickup", Vec2::new(136.0, 90.0));
    assert_eq!(game.world.entity(cape).unwrap().value("cost"), 12000.0);
    assert!(!game.player.has("Cape"));
    (game, cape)
}

fn acquire_grounded(game: &mut Game, cape: u64) {
    // Actual inherited item Steps clear the source shop cost in this empty
    // authored room. No mutation forces the pickup free.
    for _ in 0..4 {
        game.step(&InputFrame::default());
    }
    assert_eq!(game.state, RunState::Playing);
    assert_eq!(game.player.health, 4);
    assert_eq!(game.player.pos, Vec2::new(136.0, 88.0));
    assert!(game.player.grounded);
    assert_eq!(game.world.entity(cape).unwrap().value("cost"), 0.0);
    assert!(!game.player.has("Cape"));
    game.step(&InputFrame {
        down: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert!(
        game.player.has("Cape"),
        "FIXTURE: actual free Cape pickup required"
    );
    assert!(
        game.world.entity(cape).is_none(),
        "FIXTURE: pickup consumed"
    );
    assert!(game.events.iter().any(|event| {
        matches!(event, GameEvent::Collected { object, .. } if object == "oCapePickup")
    }));
    assert_eq!(value(game, "cape_open"), 0.0);
    assert!(game.player.grounded);
    game.step(&InputFrame::default());
    assert!(game.player.grounded);
    assert_eq!(game.player.health, 4);
}

#[test]
fn natural_free_cape_first_open_keeps_descent_count_then_prior_open_resets_before_increment() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut toggle, cape) = authored_cape(rules);
        let (mut closed, closed_cape) = authored_cape(rules);
        acquire_grounded(&mut toggle, cape);
        acquire_grounded(&mut closed, closed_cape);
        let mut qualified = false;
        for _ in 0..60 {
            assert_eq!(toggle.player.pos, closed.player.pos);
            assert_eq!(toggle.player.vel, closed.player.vel);
            assert_eq!(value(&toggle, "fall_timer"), value(&closed, "fall_timer"));
            assert_eq!(value(&toggle, "cape_open"), 0.0);
            if toggle.player.state == PlayerState::Falling
                && !toggle.player.grounded
                && toggle.player.vel.y > 0.0
                && value(&toggle, "jumped") != 0.0
                && value(&toggle, "fall_timer") > 4.0
            {
                qualified = true;
                break;
            }
            let right = InputFrame {
                right: true,
                ..Default::default()
            };
            toggle.step(&right);
            closed.step(&right);
        }
        assert!(
            qualified,
            "FIXTURE: natural jumped descent with fall>4 required"
        );
        live_fall(&toggle);
        live_fall(&closed);
        let prior = value(&toggle, "fall_timer");
        let opening = InputFrame {
            jump: true,
            jump_pressed: true,
            ..Default::default()
        };
        toggle.step(&opening);
        closed.step(&InputFrame::default());
        live_fall(&toggle);
        live_fall(&closed);
        assert_eq!(
            value(&toggle, "cape_open"),
            1.0,
            "FIXTURE: actual Jump toggle required"
        );
        assert_eq!(value(&closed, "cape_open"), 0.0);
        assert_eq!(
            value(&toggle, "fall_timer"),
            prior + 1.0,
            "CONTRACT: first opening retains the earlier active-prelude increment"
        );
        assert_eq!(value(&toggle, "fall_ticks"), prior + 1.0);
        assert_eq!(value(&closed, "fall_timer"), prior + 1.0);
        assert!(
            toggle.player.vel.y < closed.player.vel.y,
            "FIXTURE: actual Cape drag required"
        );

        // Previous-open reset runs before descent increments on this release
        // frame. The controller's later drag must preserve the resulting1.
        toggle.step(&InputFrame {
            jump_released: true,
            ..Default::default()
        });
        closed.step(&InputFrame::default());
        live_fall(&toggle);
        live_fall(&closed);
        assert_eq!(value(&toggle, "cape_open"), 1.0);
        assert_eq!(value(&toggle, "fall_timer"), 1.0);
        assert_eq!(value(&toggle, "fall_ticks"), 1.0);
        assert_eq!(value(&closed, "fall_timer"), prior + 2.0);

        // Fresh Jump closes after the same early previous-open reset. Closing
        // this frame cannot undo the already-reset/incremented source counter.
        toggle.step(&opening);
        closed.step(&InputFrame::default());
        live_fall(&toggle);
        live_fall(&closed);
        assert_eq!(
            value(&toggle, "cape_open"),
            0.0,
            "FIXTURE: actual fresh-Jump close required"
        );
        assert_eq!(value(&toggle, "fall_timer"), 1.0);
        assert_eq!(value(&closed, "fall_timer"), prior + 3.0);

        toggle.step(&InputFrame {
            jump_released: true,
            ..Default::default()
        });
        closed.step(&InputFrame::default());
        live_fall(&toggle);
        live_fall(&closed);
        assert_eq!(value(&toggle, "cape_open"), 0.0);
        assert_eq!(value(&toggle, "fall_timer"), 2.0);
        assert_eq!(value(&toggle, "fall_ticks"), 2.0);
        assert_eq!(value(&closed, "fall_timer"), prior + 4.0);
    }
}
