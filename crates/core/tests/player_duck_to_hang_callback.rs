//! Source Animation End family priority, reached through ordinary crawl inputs.
use spelunky_core::*;

#[derive(Clone, Copy, Debug)]
enum Arrangement {
    TopThenLadder,
    LadderThenTop,
    LadderOnly,
    TopOnly,
    MissedLadderThenTop,
}

fn room(
    rules: RulesPreset,
    character: characters::Character,
    arrangement: Arrangement,
) -> (Game, Vec<(u64, bool)>) {
    let mut level = level::Level::default();
    level.set_tile(13, 18, '@').unwrap();
    for x in 3..=13 {
        level.set_tile(x, 19, '1').unwrap();
    }
    for x in 0..40 {
        level.set_tile(x, 22, '1').unwrap();
    }
    let mut game = Game::shell(42, rules);
    game.load_custom(&level, "ledge-callback-families", "");
    characters::apply(&mut game, character);
    let ladder = ("oLadderOrange", Vec2::new(238.0, 320.0), true);
    let top = ("oLadderTop", Vec2::new(240.0, 320.0), true);
    // sLadder's left inset is one pixel: origin247 gives mask left248,
    // so the natural callback point247,328 must miss this active ladder.
    let missed = ("oLadderOrange", Vec2::new(247.0, 320.0), false);
    let actors = match arrangement {
        Arrangement::TopThenLadder => vec![top, ladder],
        Arrangement::LadderThenTop => vec![ladder, top],
        Arrangement::LadderOnly => vec![ladder],
        Arrangement::TopOnly => vec![top],
        Arrangement::MissedLadderThenTop => vec![missed, top],
    };
    let ids = actors
        .into_iter()
        .map(|(object, pos, hits)| (game.spawn(object, pos), hits))
        .collect();
    (game, ids)
}

fn run_case(
    rules: RulesPreset,
    character: characters::Character,
    arrangement: Arrangement,
    expected_x: f64,
) {
    let (mut game, actors) = room(rules, character, arrangement);
    let health = game.player.health;
    for _ in 0..4 {
        game.step(&InputFrame::default());
    }
    assert!(game.player.grounded, "FIXTURE_TIMING: initial support");
    let mut started = false;
    for _ in 0..80 {
        game.step(&InputFrame {
            down: true,
            right: true,
            ..Default::default()
        });
        assert!(!game.player.is_stunned(), "FIXTURE_TIMING: crawl stun");
        assert_eq!(game.player.health, health, "FIXTURE_TIMING: crawl damage");
        if game.player.data.get("duck_to_hang") == Some(&1.0) {
            started = true;
            break;
        }
    }
    assert!(
        started,
        "FIXTURE_TIMING: natural ledge onset {arrangement:?}"
    );
    let start = game.player.pos;
    assert_eq!(
        start,
        Vec2::new(239.0, 312.0),
        "FIXTURE_TIMING: onset geometry"
    );
    assert_eq!(game.player.facing, 1, "FIXTURE_TIMING: crawl facing");
    assert!(matches!(
        game.player.sprite.as_str(),
        "sDuckToHangL" | "sDamselDtHL" | "sTunnelDtHL"
    ));
    let point = Vec2::new(247.0, 328.0);
    for (id, hits) in &actors {
        let actor = game
            .world
            .entity(*id)
            .expect("FIXTURE_TIMING: authored actor missing");
        assert!(
            actor.alive && actor.engine_active,
            "FIXTURE_TIMING: actor inactive"
        );
        assert_eq!(
            actor.mask_contains(point, CollisionPrecision::BoundingBox),
            *hits,
            "FIXTURE_TIMING: authored point-mask overlap {arrangement:?} {}",
            actor.object
        );
    }
    let mut finished = false;
    for _ in 0..20 {
        game.step(&InputFrame::default());
        assert_eq!(
            game.player.health, health,
            "FIXTURE_TIMING: callback damage"
        );
        assert!(!game.player.is_stunned(), "FIXTURE_TIMING: callback stun");
        if game.player.data.get("duck_to_hang") == Some(&0.0) {
            finished = true;
            break;
        }
        assert_eq!(
            game.player.pos, start,
            "FIXTURE_TIMING: pending ledge moved"
        );
    }
    assert!(
        finished,
        "FIXTURE_TIMING: actual Animation End did not complete"
    );
    // Observe the first completed callback, before another controller Step.
    assert_eq!(
        game.player.state,
        PlayerState::Climbing,
        "CONTRACT: callback state {arrangement:?}"
    );
    assert_eq!(
        game.player.pos,
        Vec2::new(expected_x, 328.0),
        "CONTRACT: source oLadder-first callback selection {arrangement:?} {character:?} {rules:?}"
    );
    assert_eq!(game.player.previous_pos, game.player.pos);
    assert_eq!(game.player.facing, 1);
}

#[test]
fn natural_ledge_callback_prefers_ladder_family_in_both_birth_orders() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for character in [
            characters::Character::Explorer,
            characters::Character::Damsel,
            characters::Character::TunnelMan,
        ] {
            for arrangement in [Arrangement::TopThenLadder, Arrangement::LadderThenTop] {
                run_case(rules, character, arrangement, 246.0);
            }
        }
    }
}

#[test]
fn natural_ledge_callback_retains_single_family_and_point_miss_fallbacks() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for character in [
            characters::Character::Explorer,
            characters::Character::Damsel,
            characters::Character::TunnelMan,
        ] {
            for (arrangement, x) in [
                (Arrangement::LadderOnly, 246.0),
                (Arrangement::TopOnly, 248.0),
                (Arrangement::MissedLadderThenTop, 248.0),
            ] {
                run_case(rules, character, arrangement, x);
            }
        }
    }
}
