//! Generated level16 introduction and ordinary Right input only. No running
//! actor/player/terrain/timer injection; these check source Step96-97 timing.
use spelunky_core::*;

fn modes() -> impl Iterator<Item = (RulesPreset, RngAlgorithm)> {
    [RulesPreset::Classic, RulesPreset::Assisted]
        .into_iter()
        .flat_map(|rules| {
            [RngAlgorithm::Gm8Lcg32V1, RngAlgorithm::LegacySplitMix64V1]
                .into_iter()
                .map(move |algorithm| (rules, algorithm))
        })
}
fn right() -> InputFrame {
    InputFrame {
        right: true,
        ..Default::default()
    }
}
fn natural_intro(rules: RulesPreset, algorithm: RngAlgorithm) -> (Game, u64) {
    let mut game = Game::new_with_algorithm(42, rules, algorithm).unwrap();
    // Initial campaign setup uses the public level transition; the encounter
    // starts from untouched generated defaults, not a fabricated boss state.
    while game.world.level < 16 {
        game.next_level();
    }
    assert_eq!(game.world.area, Area::Boss);
    assert_eq!(game.player.pos, Vec2::new(24.0, 552.0));
    assert_eq!(game.player.data.get("active"), Some(&0.0));
    let boss = game
        .world
        .entities
        .iter()
        .find(|e| e.object == "oOlmec")
        .unwrap()
        .id;
    let fresh = game.world.entity(boss).unwrap();
    assert_eq!((fresh.state, fresh.timer), (-1, 0));
    assert_eq!(fresh.vel, Vec2::default());
    let mut saw_bounce = false;
    let mut saw_prepare = false;
    let mut saw_slam = false;
    let mut saw_source_slam_counter = false;
    for tick in 1..=600 {
        let before = game.world.entity(boss).unwrap().clone();
        game.step(&right());
        let after = game.world.entity(boss).unwrap();
        eprintln!(
            "INTRO {tick} {rules:?}/{algorithm:?} state={} timer={} alarm5={} p={:?} hp={}",
            after.state, after.timer, after.alarms[5], game.player.pos, game.player.health
        );
        saw_bounce |= after.state == 1;
        saw_prepare |= after.state == 5;
        saw_slam |= after.state == 6;
        if before.state == 6 && after.state == 0 {
            assert_eq!(after.timer, 60, "NATURAL_SLAM_IDLE_COUNTER_SOURCE_199");
            assert_eq!(after.alarms[5], 50, "NATURAL_INACTIVE_SLAM_ALARM5");
            saw_source_slam_counter = true;
        }
        if game.player.data.get("active") == Some(&1.0) {
            assert!(saw_bounce && saw_prepare && saw_slam && saw_source_slam_counter);
            assert_eq!(
                before.alarms[5], 1,
                "ordinary Alarm5 expiration releases player"
            );
            assert_eq!(
                (after.state, after.timer),
                (0, 99),
                "Alarm5 sets100 before same-frame IDLE decrement"
            );
            assert_eq!(game.player.health, 4.0);
            return (game, boss);
        }
        assert_eq!(game.player.pos, Vec2::new(24.0, 552.0));
        assert_eq!(game.player.health, 4.0);
    }
    panic!("natural generated Olmec intro must release via Alarm5");
}
fn natural_countdown_to_one(game: &mut Game, boss: u64) {
    for timer in (1..=98).rev() {
        game.step(&right());
        let actor = game.world.entity(boss).unwrap();
        assert_eq!(
            (actor.state, actor.timer),
            (0, timer),
            "POSITIVE_IDLE_COUNTER_STAYS_IDLE"
        );
        assert_eq!(
            actor.vel,
            Vec2::default(),
            "grounded IDLE retains no motion"
        );
        assert_eq!(game.player.health, 4.0);
    }
}
fn big_jump(game: &Game) -> bool {
    game.events
        .iter()
        .any(|event| matches!(event, GameEvent::Sound { name, .. } if name == "sndBigJump"))
}
// Literal oOlmec controller geometry from Create/Platform Engine, independent
// of the displayed sprite and the historical penetrating-floor fixture.
fn source_bottom(game: &Game, actor: &Entity) -> bool {
    game.world.solid_line_except(
        Vec2::new(actor.pos.x + 2.0, actor.pos.y + 64.0),
        Vec2::new(actor.pos.x + 61.0, actor.pos.y + 64.0),
        actor.id,
    )
}
fn source_body_overlap(game: &Game, actor: &Entity) -> bool {
    game.world.rectangle_family_except(
        Vec2::new(actor.pos.x + 2.0, actor.pos.y),
        Vec2::new(actor.pos.x + 61.0, actor.pos.y + 63.0),
        "oSolid",
        Some(actor.id),
        CollisionPrecision::Precise,
    )
}
fn qualify_ascending_path_and_support(game: &Game, actor: &Entity) {
    assert_eq!(actor.vel, Vec2::default());
    assert!(
        !game.world.rectangle_family_except(
            Vec2::new(actor.pos.x + 2.0, actor.pos.y - 16.0),
            Vec2::new(actor.pos.x + 61.0, actor.pos.y + 63.0),
            "oSolid",
            Some(actor.id),
            CollisionPrecision::Precise,
        ),
        "SOURCE_SETTLED_BODY_AND_SIXTEEN_PIXEL_ASCENT_CLEAR"
    );
    assert!(source_bottom(game, actor), "SOURCE_SETTLED_FLOOR_CONTACT");
    let first = Vec2::new(actor.pos.x + 2.0, actor.pos.y + 64.0);
    let last = Vec2::new(actor.pos.x + 61.0, actor.pos.y + 64.0);
    let support = game
        .world
        .tiles
        .iter()
        .enumerate()
        .filter_map(|(i, tile)| {
            let x = (i % game.world.width) as i32;
            let y = (i / game.world.width) as i32;
            (tile.engine_active
                && tile.object == "oTemple"
                && game.world.tile_collision_mask(x, y).intersects_axis_line(
                    first,
                    last,
                    CollisionPrecision::Precise,
                ))
            .then_some(i)
        })
        .collect::<Vec<_>>();
    assert!(!support.is_empty(), "SOURCE_ACTIVE_AUTHORED_TEMPLE_SUPPORT");
    assert!(
        support
            .iter()
            .all(|i| matches!(i / game.world.width, 35 | 36))
    );
    // The first natural Slam removes its five original surface cells. Keep
    // that source consequence while observing the actual remaining support.
    for x in 17..=21 {
        assert!(
            game.world.tile(x, 35).unwrap().object.is_empty(),
            "SOURCE_INTRO_SLAM_REMOVED_ORIGINAL_SURFACE"
        );
    }
    eprintln!(
        "QUALIFIED_POST_SLAM_IDLE source_support={support:?} origin={:?}",
        actor.pos
    );
}
fn qualify_player_guard_phase(game: &Game) {
    assert_eq!(game.state, RunState::Playing);
    assert_eq!(game.player.health, 4.0);
    assert_eq!(game.player.stun, 0);
    assert!(
        !game.player.is_stunned(),
        "SOURCE_NO_LATE_STUNNED_STATE_WRITE"
    );
    assert_eq!(game.player.attack_timer, 0);
    assert_eq!(game.player.data.get("active"), Some(&1.0));
    assert!(
        !matches!(
            game.player.sprite.as_str(),
            "sDuckToHangL" | "sDamselDtHL" | "sTunnelDtHL"
        ),
        "SOURCE_RECOVER_PLAYER_PHASE_NO_DUCK_TO_HANG"
    );
}
#[test]
fn natural_alarm5_positive_countdown_remains_idle_until_one() {
    for (rules, algorithm) in modes() {
        let (mut game, boss) = natural_intro(rules, algorithm);
        natural_countdown_to_one(&mut game, boss);
        assert_eq!(
            (
                game.world.entity(boss).unwrap().state,
                game.world.entity(boss).unwrap().timer
            ),
            (0, 1)
        );
    }
}
#[test]
fn natural_alarm5_idle_one_to_zero_selects_bounce_same_step() {
    for (rules, algorithm) in modes() {
        let (mut game, boss) = natural_intro(rules, algorithm);
        natural_countdown_to_one(&mut game, boss);
        let settled = game.world.entity(boss).unwrap();
        let origin = settled.pos;
        // The restored invisible Slam has removed the original upper floor.
        // Qualify the remaining live support and complete ascent sweep rather
        // than retaining either historical penetration or a baked floor Y.
        qualify_ascending_path_and_support(&game, settled);
        qualify_player_guard_phase(&game);
        game.step(&right());
        let actor = game.world.entity(boss).unwrap();
        assert_eq!(actor.timer, 0);
        assert_eq!(
            actor.state, 1,
            "SOURCE_OLMEC_IDLE_COUNTER_ONE_SELECTS_BOUNCE_SAME_STEP"
        );
        assert_eq!(actor.pos, origin);
        assert_eq!(
            actor.vel,
            Vec2::default(),
            "state selection does not execute BOUNCE body in same Step"
        );
        assert!(!big_jump(&game));
        game.step(&right());
        let actor = game.world.entity(boss).unwrap();
        assert_eq!((actor.state, actor.timer), (1, 0));
        assert_eq!(actor.pos, origin);
        assert_eq!(
            actor.vel,
            Vec2::new(0.0, -4.0),
            "next ordinary Step executes grounded BOUNCE"
        );
        assert!(!big_jump(&game));
        game.step(&right());
        let actor = game.world.entity(boss).unwrap();
        // Global and own moveTo each move -4 from the qualified clear,
        // supported origin, leaving the actual remaining floor behind.
        // Gravity is applied before the airborne BOUNCE branch, which does not
        // overwrite yVel; this same Step selects RECOVER and emits BigJump.
        assert_eq!(
            (actor.state, actor.timer),
            (2, 10),
            "first dual movement leaves source floor contact and enters RECOVER"
        );
        assert_eq!(actor.pos, Vec2::new(origin.x, origin.y - 8.0));
        assert_eq!(actor.vel, Vec2::new(0.0, -4.0 + 0.2));
        assert!(!source_bottom(&game, actor));
        assert!(!source_body_overlap(&game, actor));
        assert!(big_jump(&game));
        assert_eq!(game.player.health, 4.0);
        let source_recover_x_vel = actor.vel.x;
        assert_eq!(source_recover_x_vel, 0.0);
        qualify_player_guard_phase(&game);
        game.step(&right());
        let actor = game.world.entity(boss).unwrap();
        // Keep the following ordinary Step, now executing source RECOVER.
        // moveTo(-3.8) samples -4 on every clock tick (round(1/.8) == 1),
        // so the two movement passes add another -8. The counter decrements
        // independently before the literal Player-under-boss PREPARE guard.
        assert_eq!(actor.pos, Vec2::new(origin.x, origin.y - 16.0));
        assert!(!source_bottom(&game, actor));
        assert!(!source_body_overlap(&game, actor));
        qualify_player_guard_phase(&game);
        let after_gravity = -4.0 + 0.2 + 0.2;
        let prepare = (game.player.data.get("active") == Some(&0.0) && after_gravity >= 0.0)
            || (game.player.pos.y > actor.pos.y
                && (game.player.pos.x - (actor.pos.x + 32.0)).abs() < 32.0
                && source_recover_x_vel > -1.0);
        let expected = if prepare {
            (5, 20, Vec2::default())
        } else {
            (2, 9, Vec2::new(0.0, after_gravity))
        };
        assert_eq!(
            (actor.state, actor.timer, actor.vel),
            expected,
            "SOURCE_RECOVER_COUNTER_THEN_LITERAL_PREPARE_GUARD"
        );
        assert!(!big_jump(&game));
    }
}
