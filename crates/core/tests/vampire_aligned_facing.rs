//! Default Vampire birth followed by ordinary neutral Steps, with no AI edits.
use spelunky_core::{Game, InputFrame, RulesPreset, Vec2};

#[test]
fn default_idle_vampire_preserves_random_facing_when_player_is_exactly_above() {
    for canonical in [false, true] {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            let mut saw_left = false;
            let mut saw_right = false;
            for seed in 0..32 {
                let mut game = if canonical {
                    Game::shell(seed, rules)
                } else {
                    Game::legacy_shell(u64::from(seed), rules)
                };
                for x in 0..game.world.width {
                    game.world.set_tile(x, 6, "oBlock");
                }
                // The first normal Step uses clock2: default xVel2.5 moves
                // from80 to83. Player x91 aligns with its new center. Vertical
                // separation keeps this AI assertion outside contact events.
                game.player.pos = Vec2::new(91.0, 24.0);
                game.player.previous_pos = game.player.pos;
                let id = game.spawn("oVampire", Vec2::new(80.0, 80.0));
                let before = game.world.entity(id).unwrap();
                assert_eq!((before.state, before.timer, before.facing), (0, 0, 1));
                assert_eq!(before.vel.x, 2.5);
                let mut source_rng = game.rng.clone();
                // Source counter0 chooses random facing before the strict
                // less-than / greater-than target tests; equality keeps it.
                let expected = if source_rng.range(2) == 0 { -1 } else { 1 };
                game.step(&InputFrame::default());
                let actor = game.world.entity(id).unwrap();
                assert_eq!(game.player.pos.x, actor.pos.x + 8.0);
                assert_eq!(actor.state, 2, "default IDLE selects ATTACK");
                assert_eq!(actor.facing, expected, "SOURCE_VAMPIRE_EQUAL_X_FACING");
                assert_eq!(game.player.health, 4);
                assert_eq!(game.rng.clone().next_u64(), source_rng.next_u64());
                game.step(&InputFrame::default());
                let actor = game.world.entity(id).unwrap();
                assert!((actor.vel.x - f64::from(expected) * 3.9).abs() < 1e-12);
                saw_left |= expected < 0;
                saw_right |= expected > 0;
            }
            assert!(saw_left && saw_right, "exercise both random facing results");
        }
    }
}
