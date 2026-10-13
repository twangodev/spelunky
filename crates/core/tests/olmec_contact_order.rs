//! Fresh source Boss Create, authored contacts, and idle Game::step inputs.
//! The one-step scenes test own-Step contact correction, not campaign reachability.
use spelunky_core::*;

fn room(canonical: bool, rules: RulesPreset) -> Game {
    let mut game = if canonical {
        Game::shell(42, rules)
    } else {
        Game::legacy_shell(1, rules)
    };
    game.world = World::new(40, 24, Area::Boss, 16);
    game.world.name = "rLevel".into();
    game.player.pos = Vec2::new(500.0, 216.0);
    game.player.previous_pos = game.player.pos;
    game.player.data.insert("active".into(), 0.0);
    game.world.set_tile(31, 14, "oBlock");
    game
}

#[test]
fn fresh_boss_backs_away_from_each_authored_wall_without_directional_input() {
    for canonical in [false, true] {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            for (start, expected) in [(338.0, 337.0), (414.0, 415.0)] {
                let mut game = room(canonical, rules);
                for y in 0..24 {
                    game.world.set_tile(25, y, "oHardBlock");
                }
                let id = game.spawn("oOlmec", Vec2::new(start, 96.0));
                let born = game.world.entity(id).unwrap();
                assert_eq!(born.state, -1);
                assert_eq!(born.vel, Vec2::default());
                game.step(&InputFrame::default());
                let boss = game.world.entity(id).unwrap();
                assert_eq!(
                    boss.pos.x, expected,
                    "source post-move full-height contact retreats one pixel"
                );
                assert_eq!(boss.vel.x, 0.0);
                assert_eq!(game.player.health, 4);
                assert_eq!(game.world.tile(25, 6).unwrap().object, "oHardBlock");
            }
        }
    }
}

#[test]
fn fresh_boss_with_one_more_pixel_of_wall_clearance_is_not_corrected() {
    for canonical in [false, true] {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            for start in [337.0, 415.0] {
                let mut game = room(canonical, rules);
                for y in 0..24 {
                    game.world.set_tile(25, y, "oHardBlock");
                }
                let id = game.spawn("oOlmec", Vec2::new(start, 96.0));
                game.step(&InputFrame::default());
                assert_eq!(game.world.entity(id).unwrap().pos.x, start);
                assert_eq!(game.player.health, 4);
            }
        }
    }
}

#[test]
fn fresh_boss_at_rest_under_ceiling_emits_slam_before_downward_correction() {
    for canonical in [false, true] {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            let mut game = room(canonical, rules);
            for x in 5..9 {
                game.world.set_tile(x, 5, "oBlock");
            }
            let id = game.spawn("oOlmec", Vec2::new(80.0, 96.0));
            assert_eq!(game.world.entity(id).unwrap().vel, Vec2::default());
            game.step(&InputFrame::default());
            let boss = game.world.entity(id).unwrap();
            assert_eq!(boss.pos, Vec2::new(80.0, 97.0));
            assert!((boss.vel.y - 0.2).abs() < 1e-12);
            let slams: Vec<_> = game
                .world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == "oOlmecSlam")
                .collect();
            assert_eq!(slams.len(), 1, "contact is independent of upward velocity");
            assert_eq!(slams[0].pos, Vec2::new(80.0, 80.0));
            for x in 5..9 {
                assert!(game.world.tile(x, 5).unwrap().object.is_empty());
            }
            assert_eq!(game.player.health, 4);
        }
    }
}
