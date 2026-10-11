//! Source-derived initial-state fixtures; uncompiled and unexecuted.
//! Heading oracles use the selected port RNG's source draw shape, not an
//! original executable runtime capture or a naturally reached idle interval.
use spelunky_core::*;

fn room(seed: u32, rules: RulesPreset) -> Game {
    let mut game = Game::shell(seed, rules);
    game.world = World::new(30, 20, Area::Mines, 1);
    game.player.pos = Vec2::new(240.0, 128.0);
    game.player.previous_pos = game.player.pos;
    game
}

#[test]
fn actual_step_far_idle_scarab_uses_integer_degrees_and_negative_y_sine() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for seed in [0, 1, 5, 11, 42, 123] {
            let mut game = room(seed, rules);
            let id = game.spawn("oScarab", Vec2::new(96.0, 80.0));
            let scarab = game.world.entity_mut(id).unwrap();
            scarab.timer = 0;
            scarab.vel = Vec2::default();
            // All controlled writes precede the first authoritative Step.
            assert_eq!(scarab.state, 0);
            assert!(scarab.health > 0);
            assert!(scarab.pos.distance(game.player.pos) > 96.0);
            let mut expected = game.rng.clone();
            let degrees = expected.range(361);
            let radians = f64::from(degrees).to_radians();
            let velocity = Vec2::new(4.0 * radians.cos(), -4.0 * radians.sin());
            let counter = 10 + expected.range(21);

            game.step(&InputFrame::default());

            let scarab = game
                .world
                .entity(id)
                .expect("mandatory live Scarab witness");
            assert_eq!(scarab.pos, Vec2::new(96.0, 80.0));
            assert!(
                (scarab.vel.x - velocity.x).abs() < 1e-12,
                "seed={seed}, degrees={degrees}, actual={:?}",
                scarab.vel
            );
            assert!(
                (scarab.vel.y - velocity.y).abs() < 1e-12,
                "seed={seed}, degrees={degrees}, actual={:?}",
                scarab.vel
            );
            assert_eq!(scarab.timer, counter);
            assert_eq!(
                game.rng.clone().next_u64(),
                expected.next_u64(),
                "one source integer heading draw precedes the counter draw"
            );
        }
    }
}

#[test]
fn actual_step_idle_scarab_positive_counter_control_does_not_draw_a_heading() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = room(42, rules);
        let id = game.spawn("oScarab", Vec2::new(96.0, 80.0));
        game.world.entity_mut(id).unwrap().timer = 2;
        let mut expected = game.rng.clone();

        game.step(&InputFrame::default());

        let scarab = game.world.entity(id).unwrap();
        assert_eq!(scarab.timer, 1);
        assert_eq!(scarab.vel, Vec2::default());
        assert_eq!(game.rng.clone().next_u64(), expected.next_u64());
    }
}
