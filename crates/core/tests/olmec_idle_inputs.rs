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
            assert_eq!(game.player.health, 4);
            return (game, boss);
        }
        assert_eq!(game.player.pos, Vec2::new(24.0, 552.0));
        assert_eq!(game.player.health, 4);
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
        assert_eq!(game.player.health, 4);
    }
}
fn big_jump(game: &Game) -> bool {
    game.events
        .iter()
        .any(|event| matches!(event, GameEvent::Sound { name, .. } if name == "sndBigJump"))
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
        let origin = game.world.entity(boss).unwrap().pos;
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
        // The untouched intro slam stops with bottom570 inside the authored
        // Temple floor560..575. The first dual -4 move leaves bottom562, so
        // source isCollisionBottom(1) still selects grounded BOUNCE.
        assert_eq!(
            (actor.state, actor.timer),
            (1, 0),
            "first dual movement still has the source floor contact"
        );
        assert_eq!(actor.pos, Vec2::new(origin.x, origin.y - 8.0));
        assert_eq!(actor.vel, Vec2::new(0.0, -4.0));
        assert!(!big_jump(&game));
        game.step(&right());
        let actor = game.world.entity(boss).unwrap();
        assert_eq!(
            (actor.state, actor.timer),
            (2, 10),
            "next airborne BOUNCE enters RECOVER with source counter10"
        );
        // Source global and own moves now total -16; bottom554 is clear of
        // the same floor, so the source airborne branch emits BigJump.
        assert_eq!(actor.pos, Vec2::new(origin.x, origin.y - 16.0));
        assert!(big_jump(&game));
    }
}
