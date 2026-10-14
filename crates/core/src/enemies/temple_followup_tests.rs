//! Source-qualified constructor checks; these do not certify whole generation RNG.
use crate::{Game, RulesPreset, Vec2};

fn game(canonical: bool, seed: u32, rules: RulesPreset) -> Game {
    if canonical {
        Game::shell(seed, rules)
    } else {
        Game::legacy_shell(u64::from(seed), rules)
    }
}

#[test]
fn olmec_runtime_create_consumes_its_facing_draw_before_caller_overrides() {
    for canonical in [false, true] {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            let mut saw_left = false;
            let mut saw_right = false;
            for seed in 0..32 {
                let mut game = game(canonical, seed, rules);
                // Literal own Create: facing=rand(0,1). No other own Create
                // statement consumes random values before returning to caller.
                let mut source_rng = game.rng.clone();
                let expected = if source_rng.range(2) == 0 { -1 } else { 1 };
                let id = game.spawn("oOlmec", Vec2::new(160.0, 96.0));
                assert_eq!(game.world.entity(id).unwrap().facing, expected);
                assert_eq!(game.rng.clone().next_u64(), source_rng.next_u64());
                assert_eq!(game.camera.y, 400.0);
                saw_left |= expected < 0;
                saw_right |= expected > 0;
            }
            assert!(saw_left && saw_right, "exercise both source facing results");
        }
    }
}

#[test]
fn generated_olmec_create_runs_once_and_runtime_preserves_producer_facing() {
    for canonical in [false, true] {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            let mut game = game(canonical, 42, rules);
            let mut source_rng = game.rng.clone();
            let expected = if source_rng.range(2) == 0 { -1 } else { 1 };
            let id = crate::creation::spawn_generated_entity(
                &mut game.world,
                &mut game.rng,
                "oOlmec",
                Vec2::new(304.0, 496.0),
            );
            assert_eq!(game.world.entity(id).unwrap().facing, expected);
            assert_eq!(game.rng.clone().next_u64(), source_rng.clone().next_u64());
            // Source caller fields follow synchronous Create. Simulate that
            // producer override, then exercise the actual bootstrap hooks.
            game.world.entity_mut(id).unwrap().facing = -expected;
            for _ in 0..2 {
                crate::creation::initialize_entity_local(&mut game.world, &mut game.rng, id);
                crate::enemies::create_instance(&mut game, id);
            }
            assert_eq!(game.world.entity(id).unwrap().facing, -expected);
            assert_eq!(game.rng.next_u64(), source_rng.next_u64());
            assert_eq!(game.camera.y, 400.0);
        }
    }
}
