//! PRIVATE UNRUN V2 proposal. No runtime/compiler execution has validated timing.
//! All initial actors are authored before any step; projectiles come only from
//! source trap Alarm/Collision events. Inputs are the sole post-start mutations.
use spelunky_core::*;

fn room(
    rules: RulesPreset,
    character: characters::Character,
    before_down: bool,
    trap: bool,
    lower_y: f64,
) -> Game {
    let mut level = level::Level::default();
    level.set_tile(6, 8, '@').unwrap();
    for x in 0..40 {
        level.set_tile(x, 19, '1').unwrap();
    }
    let mut game = Game::shell(42, rules);
    game.load_custom(&level, "cached-down-v2", "");
    characters::apply(&mut game, character);
    // A leftward Arrow hit must leave the upper support before the next
    // stunned bottom-line probe. Source oPlayer1 Step376 bounces even with
    // cached Down and body/platform overlap; an upstream branch at x104
    // catches that probe at (112,160) and prevents the lower-contact witness.
    let upper_supports: &[f64] = if before_down { &[120.] } else { &[104., 120.] };
    for &x in upper_supports {
        game.spawn("oTreeBranchTile", Vec2::new(x, 160.));
    }
    for column in 1..41 {
        game.spawn("oTreeBranchTile", Vec2::new(column as f64 * 16., lower_y));
    }
    if trap {
        if before_down {
            game.spawn("oArrowTrapLeft", Vec2::new(144., 160.));
        } else {
            // Behind a player walking right off the initial support. Its
            // natural Arrow knocks right, away from that support.
            game.spawn("oArrowTrapRight", Vec2::new(112., 160.));
        }
    }
    game
}

fn diagnostic(game: &Game) -> String {
    let actors: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| {
            e.object == "oArrow"
                || e.object == "oArrowTrapTest"
                || e.object.starts_with("oArrowTrap")
        })
        .map(|e| {
            format!(
                "{}#{} pos={:?} vel={:?} fired={} scale={}",
                e.object,
                e.id,
                e.pos,
                e.vel,
                e.value("fired"),
                e.value("image_xscale")
            )
        })
        .collect();
    format!(
        "tick={} pos={:?} vel={:?} state={:?} sprite={} health={} stun={} cachedDown={:?} actors={actors:?}",
        game.tick,
        game.player.pos,
        game.player.vel,
        game.player.state,
        game.player.sprite,
        game.player.health,
        game.player.stun,
        game.player.data.get("source_k_down")
    )
}

fn settle(game: &mut Game) {
    for _ in 0..4 {
        game.step(&InputFrame::default());
    }
    assert!(game.player.grounded, "{}", diagnostic(game));
    assert_eq!(
        game.player.pos,
        Vec2::new(120., 152.),
        "{}",
        diagnostic(game)
    );
}

fn hit(game: &mut Game, down: bool) -> Vec<String> {
    let health = game.player.health;
    let mut trace = Vec::new();
    for _ in 0..40 {
        game.step(&InputFrame {
            down,
            right: !down,
            ..Default::default()
        });
        trace.push(diagnostic(game));
        assert_eq!(game.state, RunState::Playing, "{trace:#?}");
        if game.player.is_stunned() {
            assert_eq!(
                game.player.health,
                health - 2.0,
                "real Arrow damage: {trace:#?}"
            );
            assert_eq!(game.player.stun, 20, "real Arrow stun: {trace:#?}");
            return trace;
        }
    }
    panic!("UNRUN initial trap fixture failed to produce its natural Arrow hit: {trace:#?}");
}

// Returns whether the first downward sweep penetrated the target platform.
// A source-valid exact tangent can bounce on the following Step, regardless
// of kDown, so do not assert complete traversal of the 11-pixel branch mask.
fn first_lower_contact(game: &mut Game, held_down: bool, top: f64, mut trace: Vec<String>) -> bool {
    // Both stun controls deliberately hold the opposite live input.
    let cached_down = f64::from(!held_down);
    assert_eq!(game.player.data.get("source_k_down"), Some(&cached_down));
    for _ in 0..60 {
        let previous_bottom = game.player.pos.y + 8.;
        game.step(&InputFrame {
            down: held_down,
            ..Default::default()
        });
        trace.push(diagnostic(game));
        assert_eq!(game.state, RunState::Playing, "{trace:#?}");
        assert!(
            game.player.is_stunned(),
            "must observe movement before ordinary input resumes: {trace:#?}"
        );
        assert_eq!(
            game.player.data.get("source_k_down"),
            Some(&cached_down),
            "stunned controller must preserve the preceding Down: {trace:#?}"
        );
        let bottom = game.player.pos.y + 8.;
        if previous_bottom < top && bottom >= top {
            return bottom > top;
        }
    }
    panic!("UNRUN fixture failed to reach lower platform before recovery: {trace:#?}");
}

#[test]
fn down_cached_before_arrow_survives_release_during_stun() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for character in [
            characters::Character::Explorer,
            characters::Character::Damsel,
        ] {
            let mut penetrated = Vec::new();
            // Fixed adjacent authored heights are a finite phase control.
            // A descending sweep of at least two pixels cannot end exactly
            // on both neighboring platform tops. One may legitimately be a
            // tangent; the other must expose the retained kDown=true sweep.
            for top in [224., 225.] {
                let mut game = room(rules, character, true, true, top);
                settle(&mut game);
                let trace = hit(&mut game, true);
                let result = first_lower_contact(&mut game, false, top, trace);
                penetrated.push((top, result));
            }
            assert!(
                penetrated.iter().any(|(_, yes)| *yes),
                "cached Down was discarded: {rules:?}/{character:?} {penetrated:?}"
            );
        }
    }
}

#[test]
fn down_pressed_only_during_stun_cannot_replace_cached_false() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for character in [
            characters::Character::Explorer,
            characters::Character::Damsel,
        ] {
            for top in [224., 225.] {
                let mut game = room(rules, character, false, true, top);
                settle(&mut game);
                let trace = hit(&mut game, false);
                assert!(
                    !first_lower_contact(&mut game, true, top, trace),
                    "live held Down replaced cached false: {rules:?}/{character:?}/top={top}"
                );
            }
        }
    }
}

#[test]
fn ordinary_down_still_controls_platform_drop_through() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for down in [false, true] {
            let mut game = room(rules, characters::Character::Explorer, down, false, 224.);
            settle(&mut game);
            let mut trace = Vec::new();
            let mut observed = false;
            for _ in 0..60 {
                game.step(&InputFrame {
                    down,
                    right: !down,
                    ..Default::default()
                });
                trace.push(diagnostic(&game));
                assert_eq!(game.state, RunState::Playing, "{trace:#?}");
                assert!(!game.player.is_stunned(), "{trace:#?}");
                if down && game.player.pos.y > 242.
                    || !down && game.player.grounded && game.player.pos.y == 216.
                {
                    observed = true;
                    break;
                }
            }
            assert!(observed, "ordinary Down regression: down={down} {trace:#?}");
        }
    }
}
