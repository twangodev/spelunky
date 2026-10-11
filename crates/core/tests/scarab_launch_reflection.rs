//! Source-derived public Game.step fixtures. Natural Create/countdown controls
//! cover distance and edge reflection. The last two tests explicitly author
//! otherwise-unproven initial counter/health states, never post-start writes.
//! RNG expectations pin LegacySplitMix64V1, not original EXE execution.
use spelunky_core::{Game, InputFrame, RulesPreset, Vec2, level::Level};

fn arena(seed: u64, rules: RulesPreset, origin: Vec2, wall: bool, ceiling: bool) -> (Game, u64) {
    let mut level = Level::default();
    level.set_tile(14, 7, '@').unwrap();
    level.set_tile(34, 7, 'X').unwrap();
    for x in 0..spelunky_core::level::LEVEL_WIDTH {
        level.set_tile(x, 8, '1').unwrap();
    }
    if wall {
        level.set_tile(6, 4, '1').unwrap();
    }
    if ceiling {
        level.set_tile(5, 3, '1').unwrap();
    }
    let mut game = Game::legacy_shell(seed, rules);
    game.load_custom(&level, "scarab-launch-reflection", "");
    let id = game.spawn("oScarab", origin);
    assert_eq!(game.player.pos, Vec2::new(248.0, 136.0));
    (game, id)
}

fn reach_counter_one(game: &mut Game, id: u64) {
    let initial = game.world.entity(id).unwrap().timer;
    assert!((10..=30).contains(&initial));
    for _ in 1..initial {
        game.step(&InputFrame::default());
    }
    let actor = game
        .world
        .entity(id)
        .expect("natural stationary countdown survives");
    assert_eq!(actor.timer, 1);
    assert_eq!(actor.vel, Vec2::default());
}

fn close(actual: Vec2, expected: Vec2) {
    assert!(
        (actual.x - expected.x).abs() < 1e-12 && (actual.y - expected.y).abs() < 1e-12,
        "actual={actual:?}, expected={expected:?}"
    );
}

#[test]
fn natural_countdown_near_branch_measures_from_scarab_center() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        // Source center(192,136) is 56px from the player. The old origin
        // distance is sqrt(64^2+8^2), incorrectly choosing random direction.
        let (mut game, id) = arena(42, rules, Vec2::new(184.0, 128.0), false, false);
        reach_counter_one(&mut game, id);
        let mut expected = game.rng.clone();
        let counter = 10 + expected.range(21);
        game.step(&InputFrame::default());
        let actor = game
            .world
            .entity(id)
            .expect("near branch does not collect the Scarab");
        close(actor.vel, Vec2::new(-4.0, 0.0));
        assert_eq!(actor.timer, counter);
        assert_eq!(
            game.rng.clone().next_u64(),
            expected.next_u64(),
            "near branch draws only counter"
        );

        // Exactly64 is far, preserving the strict source threshold.
        let (mut game, id) = arena(42, rules, Vec2::new(176.0, 128.0), false, false);
        reach_counter_one(&mut game, id);
        let mut expected = game.rng.clone();
        let radians = f64::from(expected.range(361)).to_radians();
        let counter = 10 + expected.range(21);
        game.step(&InputFrame::default());
        let actor = game.world.entity(id).unwrap();
        close(
            actor.vel,
            Vec2::new(4.0 * radians.cos(), -4.0 * radians.sin()),
        );
        assert_eq!(actor.timer, counter);
        assert_eq!(game.rng.clone().next_u64(), expected.next_u64());
    }
}

fn displacement(v: f64, time: u64) -> f64 {
    let fraction = v.abs().fract();
    let extra = fraction > 0.0 && time % (1.0 / fraction).round_ties_even().max(1.0) as u64 == 0;
    (v.abs().floor() + f64::from(extra)) * v.signum()
}

fn edge_seed(rules: RulesPreset, vertical: bool, obstacle: bool) -> u64 {
    // Select an ordinary birth in the COMPLETE authored scene. Terrain Create
    // consumes RNG before Scarab Create, so each obstacle/control scene selects
    // its own seed. No actor or RNG fields are edited before or after birth.
    // The FIRST launched movement must land exactly at the authored edge.
    for seed in 0..512 {
        let (game, id) = arena(
            seed,
            rules,
            Vec2::new(96.0, 80.0),
            !vertical && obstacle,
            vertical && obstacle,
        );
        let counter = game.world.entity(id).unwrap().timer;
        let degrees = game.rng.clone().range(361);
        let angle = f64::from(degrees).to_radians();
        let velocity = Vec2::new(4.0 * angle.cos(), -4.0 * angle.sin());
        let time = u64::from(counter) + 2;
        if (!vertical && velocity.x > 3.8 && displacement(velocity.x, time) == 4.0)
            || (vertical && velocity.y < -3.8 && displacement(velocity.y, time) == -4.0)
        {
            return seed;
        }
    }
    panic!("mandatory ordinary birth witness absent from bounded seed set");
}

#[test]
fn natural_first_movement_reflects_at_right_edge_before_a_blocked_move() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for wall in [false, true] {
            let seed = edge_seed(rules, false, wall);
            let (mut game, id) = arena(seed, rules, Vec2::new(96.0, 80.0), wall, false);
            // Source bounds4..12 and native mask1..13 both clear the wall112..
            assert_eq!(game.world.entity(id).unwrap().bounds().x + 8.0, 108.0);
            reach_counter_one(&mut game, id);
            game.step(&InputFrame::default()); // ordinary random launch
            assert!(game.world.entity(id).unwrap().vel.x > 3.8);
            game.step(&InputFrame::default()); // first move, edge query now112
            let actor = game.world.entity(id).unwrap();
            assert_eq!(actor.pos.x, 100.0);
            assert_eq!(
                actor.vel.x < 0.0,
                wall,
                "ordinary birth selected for this authored wall scene"
            );
        }
    }
}

#[test]
fn natural_first_movement_reflects_at_ceiling_without_losing_upward_velocity() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for ceiling in [false, true] {
            let seed = edge_seed(rules, true, ceiling);
            let (mut game, id) = arena(seed, rules, Vec2::new(96.0, 80.0), false, ceiling);
            // Ceiling occupies y64..79. Initial native top82 and custom top84
            // are clear; first upward move leaves custom top80, query y79.
            reach_counter_one(&mut game, id);
            game.step(&InputFrame::default());
            assert!(game.world.entity(id).unwrap().vel.y < -3.8);
            game.step(&InputFrame::default());
            let actor = game.world.entity(id).unwrap();
            assert_eq!(actor.pos.y, 76.0);
            assert_eq!(
                actor.vel.y > 0.0,
                ceiling,
                "ordinary birth selected for this authored ceiling scene"
            );
        }
    }
}

#[test]
fn controlled_initial_negative_velocity_obeys_signed_launch_predicate() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, id) = arena(42, rules, Vec2::new(96.0, 80.0), false, false);
        // Explicit initial-condition control: counter0 while moving is not
        // claimed reachable from ordinary Create after the zero-crossing fix.
        let actor = game.world.entity_mut(id).unwrap();
        actor.timer = 0;
        actor.vel = Vec2::new(-2.0, -2.0);
        let mut expected = game.rng.clone();
        let radians = f64::from(expected.range(361)).to_radians();
        let counter = 10 + expected.range(21);
        game.step(&InputFrame::default());
        let actor = game.world.entity(id).unwrap();
        assert_eq!(actor.pos, Vec2::new(94.0, 78.0));
        close(
            actor.vel,
            Vec2::new(4.0 * radians.cos(), -4.0 * radians.sin()),
        );
        assert_eq!(actor.timer, counter);
        assert_eq!(game.rng.clone().next_u64(), expected.next_u64());
    }
}

#[test]
fn controlled_initial_lethal_counter_one_resumes_step_rng_after_six_sparks() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, id) = arena(42, rules, Vec2::new(96.0, 80.0), false, false);
        // Explicit initial lethal control. Ordinary Pistol and Explosion hits
        // impart velocity; neither proves this stationary counter1 death state.
        let actor = game.world.entity_mut(id).unwrap();
        actor.timer = 1;
        actor.health = 0;
        let mut expected = game.rng.clone();
        for _ in 0..3 {
            expected.range(15);
            expected.range(15);
            expected.range(3);
        }
        for _ in 0..3 {
            expected.range(5);
            expected.range(5);
        }
        expected.range(361); // far heading, after synchronous Destroy's3 sparks
        expected.range(21); // fresh10..30 counter, despite receiver destruction
        game.step(&InputFrame::default());
        assert!(game.world.entity(id).is_none());
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == "oFlareSpark")
                .count(),
            6
        );
        assert_eq!(
            game.rng.clone().next_u64(),
            expected.next_u64(),
            "Step continues after own Destroy"
        );
    }
}
