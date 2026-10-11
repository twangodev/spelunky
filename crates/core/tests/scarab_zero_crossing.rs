//! Ordinary source Create and neutral Game.step countdown: no timer/state
//! overrides. RNG assertions describe the port adapter and source draw order.
use spelunky_core::{Game, InputFrame, Rng, RngAlgorithm, RulesPreset, Vec2, level::Level};

fn arena(seed: u32, rules: RulesPreset) -> (Game, u64) {
    let mut level = Level::default();
    level.set_tile(14, 7, '@').unwrap();
    level.set_tile(34, 7, 'X').unwrap();
    for x in 0..spelunky_core::level::LEVEL_WIDTH {
        level.set_tile(x, 8, '1').unwrap();
    }
    let mut game = Game::shell(seed, rules);
    game.load_custom(&level, "scarab-natural-countdown", "");
    // Initial ordinary birth; source Create owns the random10..30 counter.
    let scarab = game.spawn("oScarab", Vec2::new(96.0, 80.0));
    assert_eq!(game.player.pos, Vec2::new(248.0, 136.0));
    assert_eq!(game.world.tile(15, 9).unwrap().object, "oBrick");
    (game, scarab)
}

// Independent arithmetic over one raw draw, not production range()/unit().
// Canonical mapping follows frozen EXE-derived math; Legacy retains historical
// SplitMix mapping. These controls are not original gameplay runtime captures.
fn rand(rng: &mut Rng, width: u64) -> u32 {
    assert!((1..=u64::from(u32::MAX)).contains(&width));
    let algorithm = rng.algorithm();
    let raw = rng.next_u64();
    match algorithm {
        RngAlgorithm::Gm8Lcg32V1 => ((raw * width) >> 32) as u32,
        RngAlgorithm::LegacySplitMix64V1 => (raw % width) as u32,
    }
}

fn source_displacement(velocity: f64, time: u64) -> f64 {
    let fraction = velocity.abs().fract();
    let extra = fraction > 0.0 && time % (1.0 / fraction).round_ties_even().max(1.0) as u64 == 0;
    (velocity.abs().floor() + f64::from(extra)) * velocity.signum()
}

fn source_decay(velocity: f64) -> f64 {
    let value = (velocity.abs() - 0.5).max(0.0) * velocity.signum();
    if value.abs() < 1.0 { 0.0 } else { value }
}

#[test]
fn ordinary_scarab_countdown_launches_on_zero_crossing_then_moves_next_step() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for seed in [0, 1, 5, 11, 42, 123] {
            let (mut game, id) = arena(seed, rules);
            let actor = game.world.entity(id).unwrap();
            let initial_counter = actor.timer;
            assert!((10..=30).contains(&initial_counter));
            assert_eq!(actor.state, 0);
            assert_eq!(actor.vel, Vec2::default());
            let origin = actor.pos;
            assert!(
                origin.distance(game.player.pos) > 96.0,
                "far branch avoids the separate near-character query contract"
            );
            let initial_rng = game.rng.clone();
            // Every earlier natural stationary tick is a no-launch/no-RNG
            // control. There are no writes to the actor after ordinary birth.
            for remaining in (2..=initial_counter).rev() {
                assert_eq!(game.world.entity(id).unwrap().timer, remaining);
                game.step(&InputFrame::default());
                let actor = game
                    .world
                    .entity(id)
                    .expect("mandatory live natural Scarab");
                assert_eq!(actor.pos, origin);
                assert_eq!(actor.vel, Vec2::default());
                assert_eq!(actor.timer, remaining - 1);
                assert_eq!(
                    game.rng.clone().next_u64(),
                    initial_rng.clone().next_u64(),
                    "positive remaining counter consumes no heading/counter RNG"
                );
            }
            assert_eq!(
                game.world.entity(id).unwrap().timer,
                1,
                "counter1 is reached by ordinary neutral Steps, never assigned"
            );
            let mut expected = game.rng.clone();
            let degrees = rand(&mut expected, 361);
            let angle = (degrees as f64).to_radians();
            let launch = Vec2::new(4.0 * angle.cos(), -4.0 * angle.sin());
            let counter = 10 + (rand(&mut expected, 21)) as u32;

            game.step(&InputFrame::default());

            let actor = game
                .world
                .entity(id)
                .expect("zero-crossing receiver stays live");
            assert_eq!(actor.pos, origin, "moveTo precedes this tick's launch");
            assert!(
                (actor.vel.x - launch.x).abs() < 1e-12 && (actor.vel.y - launch.y).abs() < 1e-12,
                "source counter1->0 must launch in the same Step: seed={seed}, degrees={degrees}, actual={:?}",
                actor.vel
            );
            assert_eq!(
                actor.timer, counter,
                "heading draw precedes fresh10..30 counter"
            );
            let mut actual_rng = game.rng.clone();
            let mut expected_rng = expected.clone();
            for _ in 0..8 {
                assert_eq!(
                    actual_rng.next_u64(),
                    expected_rng.next_u64(),
                    "exact two source draws at the zero crossing"
                );
            }
            // The next ordinary tick consumes the launch velocity in the
            // clear airborne area before applying the existing0.5 decay.
            let time = game.level_tick.wrapping_add(2) % 100_000_001;
            let next_position = Vec2::new(
                origin.x + source_displacement(launch.x, time),
                origin.y + source_displacement(launch.y, time),
            );
            let next_velocity = Vec2::new(source_decay(launch.x), source_decay(launch.y));
            assert_ne!(next_velocity, Vec2::default());
            game.step(&InputFrame::default());
            let actor = game.world.entity(id).expect("mandatory moving Scarab");
            assert_eq!(
                actor.pos, next_position,
                "launch takes effect on the next moveTo"
            );
            assert!(
                (actor.vel.x - next_velocity.x).abs() < 1e-12
                    && (actor.vel.y - next_velocity.y).abs() < 1e-12
            );
            assert_eq!(
                actor.timer, counter,
                "moving Scarab does not decrement idle counter"
            );
            assert_eq!(
                game.rng.clone().next_u64(),
                expected.next_u64(),
                "moving follow-up neither rerolls heading nor counter"
            );
        }
    }
}
