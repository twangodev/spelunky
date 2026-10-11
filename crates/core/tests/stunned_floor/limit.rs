//! SOURCE-ONLY, UNRUN proposal; no compiler or game execution has validated
//! fixture timing. Intended unchanged in both arms of an authorized pair.
//! All actors precede Game.step. Only ordinary input changes after startup.
//! Three tests, 24 rooms, at most 3,448 Game.step calls.
use spelunky_core::*;

const FLOOR_TOP: f64 = 224.;
const HIT_BOUND: usize = 40;
const RECOVERY_BOUND: usize = 160;
const JUMP_STEPS: usize = 6;

fn value(game: &Game, key: &str) -> f64 {
    game.player.data.get(key).copied().unwrap_or(0.)
}

fn diagnostic(game: &Game) -> String {
    let actors: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.object == "oArrow" || e.object.starts_with("oArrowTrap"))
        .map(|e| {
            format!(
                "{}#{} pos={:?} vel={:?} fired={}",
                e.object,
                e.id,
                e.pos,
                e.vel,
                e.value("fired")
            )
        })
        .collect();
    format!(
        "tick={} pos={:?} vel={:?} state={:?} sprite={} health={} stun={} grounded={} fall={} limit={} left_steps={} right_steps={} actors={actors:?}",
        game.tick,
        game.player.pos,
        game.player.vel,
        game.player.state,
        game.player.sprite,
        game.player.health,
        game.player.stun,
        game.player.grounded,
        value(game, "fall_timer"),
        value(game, "velocity_limit_x"),
        value(game, "left_steps"),
        value(game, "right_steps")
    )
}

fn room(rules: RulesPreset, character: characters::Character, direction: i32, trap: bool) -> Game {
    assert!(direction == -1 || direction == 1);
    let mut level = level::Level::default();
    level.set_tile(20, 8, '@').unwrap();
    level.set_tile(20, 9, '1').unwrap();
    // Serialized row13 is world y224. Solid ground avoids retaining kDown
    // through a lower one-way platform. Its short drop avoids long-fall hurt.
    for x in 0..40 {
        level.set_tile(x, 13, '1').unwrap();
    }
    let mut game = Game::shell(42, rules);
    game.load_custom(&level, "stun-limit-initial-actors", "");
    characters::apply(&mut game, character);
    if trap {
        if direction < 0 {
            game.spawn("oArrowTrapLeft", Vec2::new(328., 192.));
        } else {
            game.spawn("oArrowTrapRight", Vec2::new(336., 192.));
        }
    }
    game
}

fn settle(game: &mut Game, trace: &mut Vec<String>) {
    for _ in 0..4 {
        game.step(&InputFrame::default());
        trace.push(diagnostic(game));
    }
    assert_eq!(game.state, RunState::Playing, "FIXTURE_TIMING: {trace:#?}");
    assert_eq!(game.player.health, 4, "FIXTURE_TIMING: {trace:#?}");
    assert_eq!(
        game.player.pos,
        Vec2::new(344., 152.),
        "FIXTURE_TIMING: {trace:#?}"
    );
    assert!(game.player.grounded, "FIXTURE_TIMING: {trace:#?}");
    assert_eq!(
        game.player.vel,
        Vec2::default(),
        "FIXTURE_TIMING: {trace:#?}"
    );
    assert!(!game.player.is_stunned(), "FIXTURE_TIMING: {trace:#?}");
    assert_eq!(
        value(game, "velocity_limit_x"),
        3.,
        "FIXTURE_TIMING: ordinary pre-hit cap: {trace:#?}"
    );
}

fn hit(game: &mut Game, direction: i32, trace: &mut Vec<String>) {
    // Walk normally off the single solid cell. Down on its lip would enter
    // DuckToHang, so retain the desired cached input only after observed fall.
    // The detector/arrow sit below the initial solid and behind outward motion.
    // Jump is never held or pressed before the actual recovery boundary.
    let mut departed = false;
    for _ in 0..HIT_BOUND {
        if !departed
            && game.player.state == PlayerState::Falling
            && !game.player.grounded
            && game.player.vel.y > 0.
            && game.player.bounds().y + game.player.bounds().h > 160.
        {
            assert_eq!(
                game.player.health, 4,
                "FIXTURE_TIMING injury before departure: {trace:#?}"
            );
            assert!(
                !game.player.is_stunned(),
                "FIXTURE_TIMING stun before departure: {trace:#?}"
            );
            assert_eq!(
                value(game, "duck_to_hang"),
                0.,
                "FIXTURE_TIMING ledge departure: {trace:#?}"
            );
            departed = true;
        }
        // Release horizontal input once fully airborne, preserving its
        // source inertia while allowing the trailing trap Arrow to catch up.
        game.step(&InputFrame {
            down: direction < 0 && departed,
            left: direction < 0 && !departed,
            right: direction > 0 && !departed,
            ..Default::default()
        });
        trace.push(diagnostic(game));
        assert_eq!(game.state, RunState::Playing, "FIXTURE_TIMING: {trace:#?}");
        assert_eq!(
            value(game, "duck_to_hang"),
            0.,
            "FIXTURE_TIMING unexpected ledge transition: {trace:#?}"
        );
        if game.player.is_stunned() {
            assert!(
                departed,
                "FIXTURE_TIMING Arrow arrived before walkout: {trace:#?}"
            );
            assert!(
                !game.player.grounded,
                "FIXTURE_TIMING Arrow arrived after landing: {trace:#?}"
            );
            assert_eq!(
                value(game, "source_k_down"),
                f64::from(direction < 0),
                "FIXTURE_TIMING intended cached Down not sampled: {trace:#?}"
            );
            assert_eq!(
                value(game, "velocity_limit_x"),
                3.,
                "FIXTURE_TIMING pre-stun ordinary limit changed: {trace:#?}"
            );
            assert_eq!(
                game.player.health, 2,
                "FIXTURE_TIMING: natural Arrow damage2: {trace:#?}"
            );
            assert_eq!(
                game.player.stun, 20,
                "FIXTURE_TIMING: natural Arrow timer20: {trace:#?}"
            );
            assert!(
                game.player.vel.x * f64::from(direction) > 3.,
                "FIXTURE_TIMING: natural Arrow knock direction/speed: {trace:#?}"
            );
            return;
        }
        assert_eq!(
            game.player.health, 4,
            "FIXTURE_TIMING injury without expected Arrow stun: {trace:#?}"
        );
    }
    panic!("FIXTURE_TIMING: initial ArrowTrap produced no source hit: {trace:#?}");
}

/// Recovery is keyed to stationary stun art, independently of vertical bounce.
/// Run without an axis keeps runHeld=0 and preserves the existing speed cap:
/// it selects neither the running cap6 nor the ordinary neutral-ground cap3.
/// Down would select Ducking, which independently assigns cap3.
fn recover_and_land_without_ground_idle_reset(game: &mut Game, trace: &mut Vec<String>) -> f64 {
    let mut first_recovery_limit = None;
    for _ in 0..RECOVERY_BOUND {
        assert_eq!(game.state, RunState::Playing, "FIXTURE_TIMING: {trace:#?}");
        assert_eq!(
            game.player.health, 2,
            "FIXTURE_TIMING: unexpected secondary injury: {trace:#?}"
        );
        if first_recovery_limit.is_none() {
            assert!(
                game.player.is_stunned(),
                "FIXTURE_TIMING: recovery boundary missed: {trace:#?}"
            );
        } else {
            assert!(
                !game.player.is_stunned(),
                "FIXTURE_TIMING: new stun after recovery: {trace:#?}"
            );
        }
        let stationary = matches!(game.player.sprite.as_str(), "sStunL" | "sDamselStunL");
        let expires = first_recovery_limit.is_none() && game.player.stun == 1 && stationary;
        game.step(&InputFrame {
            run: true,
            ..Default::default()
        });
        trace.push(diagnostic(game));
        assert_eq!(game.state, RunState::Playing, "FIXTURE_TIMING: {trace:#?}");
        assert_eq!(
            game.player.health, 2,
            "FIXTURE_TIMING: unexpected injury: {trace:#?}"
        );
        if expires {
            assert_eq!(
                game.player.stun, 0,
                "CONTRACT: stationary-art timer1 expires on next Step: {trace:#?}"
            );
            assert!(
                !game.player.is_stunned(),
                "CONTRACT: stationary-art expiry admits controller: {trace:#?}"
            );
            assert_eq!(
                value(game, "bounced"),
                0.,
                "CONTRACT: first recovered controller clears bounce: {trace:#?}"
            );
            first_recovery_limit = Some(value(game, "velocity_limit_x"));
        }
        if first_recovery_limit.is_some() {
            assert_eq!(
                game.player.stun, 0,
                "FIXTURE_TIMING: new timer after recovery: {trace:#?}"
            );
            assert!(
                !game.player.is_stunned(),
                "FIXTURE_TIMING: new stun after recovery: {trace:#?}"
            );
            assert_eq!(
                value(game, "run_held"),
                0.,
                "FIXTURE_TIMING: direction-free Run must not accelerate: {trace:#?}"
            );
            assert_eq!(
                value(game, "source_k_down"),
                0.,
                "FIXTURE_TIMING: recovered controller samples no Down: {trace:#?}"
            );
            assert_eq!(value(game, "duck_to_hang"), 0., "{trace:#?}");
            if game.player.grounded
                && game.player.vel == Vec2::default()
                && value(game, "fall_timer") == 0.
            {
                // Keep true grounded/zero-speed/fall-free Jump prerequisites,
                // but observe them after source recovery instead of at timer1.
                assert_eq!(
                    game.player.pos.y + 8.,
                    FLOOR_TOP,
                    "FIXTURE_TIMING: wrong support: {trace:#?}"
                );
                return first_recovery_limit.unwrap();
            }
        }
    }
    panic!(
        "FIXTURE_TIMING: no actual recovery followed by settled grounded launch boundary: {trace:#?}"
    );
}

fn directional_jump(direction: i32, pressed: bool) -> InputFrame {
    InputFrame {
        left: direction < 0,
        right: direction > 0,
        jump: true,
        jump_pressed: pressed,
        ..Default::default()
    }
}

fn jump_and_observe(
    game: &mut Game,
    direction: i32,
    expected_cap: f64,
    recovering: bool,
    trace: &mut Vec<String>,
) {
    let expected_health = if recovering { 2 } else { game.player.health };
    let mut maximum = 0_f64;
    let mut first_limit = None;
    for step in 0..JUMP_STEPS {
        game.step(&directional_jump(direction, step == 0));
        trace.push(diagnostic(game));
        assert_eq!(game.state, RunState::Playing, "{trace:#?}");
        assert_eq!(game.player.health, expected_health, "{trace:#?}");
        assert_eq!(game.player.stun, 0, "{trace:#?}");
        assert!(
            !game.player.is_stunned(),
            "recovery Step must admit controller: {trace:#?}"
        );
        assert!(
            !game.player.grounded,
            "FIXTURE_TIMING: jump sweep landed/blocked early: {trace:#?}"
        );
        assert!(
            matches!(
                game.player.state,
                PlayerState::Jumping | PlayerState::Falling
            ),
            "{trace:#?}"
        );
        if step == 0 {
            assert!(
                game.player.vel.y < 0.,
                "first rising Jump edge must launch: {trace:#?}"
            );
            first_limit = Some(value(game, "velocity_limit_x"));
        }
        let signed_speed = game.player.vel.x * f64::from(direction);
        assert!(
            signed_speed >= -1e-9,
            "wrong horizontal direction: {trace:#?}"
        );
        maximum = maximum.max(signed_speed);
    }
    // Physical sweep is primary: inherited old cap3 cannot exceed speed3.
    // Six held-Jump ticks accommodate the ordinary two-tick direction delay.
    // The source cap itself is checked independently on the recovery Step;
    // this fixture does not need to reach exactly speed10 before landing.
    if expected_cap == 10. {
        assert!(
            maximum > 3. && maximum <= 10. + 1e-9,
            "CONTRACT: physical maximum={maximum} must exceed3 with source cap10, recovery_first_limit={first_limit:?}: {trace:#?}"
        );
    } else {
        assert!(
            maximum > 0. && maximum <= 3. + 1e-9,
            "CONTRACT: ordinary physical maximum={maximum} exceeded cap3: {trace:#?}"
        );
    }
    assert_eq!(
        first_limit,
        Some(expected_cap),
        "CONTRACT: first controller Step retained wrong cap: {trace:#?}"
    );
    assert_eq!(value(game, "velocity_limit_x"), expected_cap, "{trace:#?}");
}

#[test]
fn actual_arrow_stun_retains_ten_on_first_grounded_jump_after_recovery_in_both_directions() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for character in [
            characters::Character::Explorer,
            characters::Character::Damsel,
        ] {
            for direction in [-1, 1] {
                let mut game = room(rules, character, direction, true);
                let mut trace = Vec::new();
                settle(&mut game, &mut trace);
                hit(&mut game, direction, &mut trace);
                let first_recovery_limit =
                    recover_and_land_without_ground_idle_reset(&mut game, &mut trace);
                assert_eq!(
                    first_recovery_limit, 10.,
                    "CONTRACT: first actual recovered controller retains stun cap10: {trace:#?}"
                );
                jump_and_observe(&mut game, direction, 10., true, &mut trace);
            }
        }
    }
}

#[test]
fn neutral_ground_recovery_legitimately_replaces_ten_with_three_before_jump() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for character in [
            characters::Character::Explorer,
            characters::Character::Damsel,
        ] {
            for direction in [-1, 1] {
                let mut game = room(rules, character, direction, true);
                let mut trace = Vec::new();
                settle(&mut game, &mut trace);
                hit(&mut game, direction, &mut trace);
                recover_and_land_without_ground_idle_reset(&mut game, &mut trace);
                game.step(&InputFrame::default());
                trace.push(diagnostic(&game));
                assert!(!game.player.is_stunned(), "{trace:#?}");
                assert_eq!(game.player.stun, 0, "{trace:#?}");
                assert_eq!(game.player.health, 2, "{trace:#?}");
                assert!(game.player.grounded, "{trace:#?}");
                assert_eq!(
                    value(&game, "velocity_limit_x"),
                    3.,
                    "ordinary no-Jump ground controller cap: {trace:#?}"
                );
                jump_and_observe(&mut game, direction, 3., true, &mut trace);
            }
        }
    }
}

#[test]
fn ordinary_ground_jump_preserves_the_existing_three_limit() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for character in [
            characters::Character::Explorer,
            characters::Character::Damsel,
        ] {
            for direction in [-1, 1] {
                let mut game = room(rules, character, direction, false);
                let mut trace = Vec::new();
                settle(&mut game, &mut trace);
                jump_and_observe(&mut game, direction, 3., false, &mut trace);
            }
        }
    }
}
