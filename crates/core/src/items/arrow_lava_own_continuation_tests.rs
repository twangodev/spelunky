//! Configured own-Step boundary: source Destroy does not abort Arrow's
//! remaining statements. These are not stock-room or full-frame witnesses.
use super::*;

fn fixture(rules: RulesPreset, algorithm: RngAlgorithm, lava: bool, room: &str) -> (Game, u64) {
    let mut game = Game::shell_with_algorithm(42, rules, algorithm).unwrap();
    game.world = World::new(40, 24, Area::Mines, 1);
    game.world.name = room.into();
    game.player.pos = Vec2::new(24.0, 200.0);
    game.player.previous_pos = game.player.pos;
    if lava {
        let id = game.spawn("oLava", Vec2::new(160.0, 80.0));
        let mask = game.world.entity(id).unwrap();
        assert!(mask.mask_contains(Vec2::new(162.0, 95.0), CollisionPrecision::BoundingBox));
        assert!(!mask.mask_intersects(
            Rect::new(159.0, 97.0, 7.0, 7.0),
            CollisionPrecision::BoundingBox
        ));
    }
    let id = game.spawn("oArrow", Vec2::new(159.0, 100.0));
    let arrow = game.world.entity_mut(id).unwrap();
    // Initial source-reachable projectile launch; no writes follow Step entry.
    arrow.vel = Vec2::new(3.0, 0.0);
    arrow.data.insert("rotation".into(), 1.0);
    assert_eq!(arrow.value("gravity"), 0.2);
    assert!(!arrow.held);
    assert_eq!(game.engine_time(), 1);
    assert!(super::super::item_body_active(
        &game,
        game.world.entity(id).unwrap()
    ));
    (game, id)
}

fn raw(game: &Game, id: u64) -> &Entity {
    game.world.entities.iter().find(|e| e.id == id).unwrap()
}

#[test]
fn inherited_lava_destroy_keeps_arrow_angle_and_does_not_reenter_dead_step() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for algorithm in [RngAlgorithm::LegacySplitMix64V1, RngAlgorithm::Gm8Lcg32V1] {
            for lava in [false, true] {
                let (mut game, id) = fixture(rules, algorithm, lava, "rLevel");
                let rng = game.rng.state_bits();
                step_instance(&mut game, id);
                let arrow = raw(&game, id);
                assert_eq!(arrow.alive, !lava);
                assert_eq!(arrow.pos, Vec2::new(162.0, 100.0));
                assert_eq!(arrow.vel, Vec2::new(3.0, 0.2));
                assert_eq!(arrow.value("gravity"), 0.6);
                // Literal source positive/positive quadrant converted to port radians.
                assert_eq!(
                    arrow.value("rotation"),
                    -(0.2_f64 / 3.0).atan(),
                    "ARROW_CHILD_AFTER_INHERITED_DESTROY {rules:?} {algorithm:?}"
                );
                assert_eq!(game.rng.state_bits(), rng);
                if lava {
                    assert!(game.world.entity(id).is_none());
                    let pos = arrow.pos;
                    let rotation = arrow.value("rotation");
                    step_instance(&mut game, id);
                    assert_eq!(
                        raw(&game, id).pos,
                        pos,
                        "dead receiver cannot enter a new Step"
                    );
                    assert_eq!(raw(&game, id).value("rotation"), rotation);
                    assert!(!raw(&game, id).alive);
                    assert_eq!(game.rng.state_bits(), rng);
                }
            }
        }
    }
}

#[test]
fn retired_arrow_moon_tail_keeps_target_existence_and_strict_y_guards() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for algorithm in [RngAlgorithm::LegacySplitMix64V1, RngAlgorithm::Gm8Lcg32V1] {
            // Missing, inactive and destroyed targets all fail instance_exists.
            for (room, target_mode, target_y, clamps) in [
                ("rMoon", 0, 100.0, false),
                ("rMoon", 1, 100.0, true),
                ("rMoon", 2, 100.0, false),
                ("rMoon", 3, 100.0, false),
                ("rMoon", 1, 84.0, false),
                ("rMoon", 1, 116.0, false),
                ("rLevel", 1, 100.0, false),
            ] {
                let (mut game, id) = fixture(rules, algorithm, true, room);
                if target_mode != 0 {
                    let target = game.spawn("oTarget", Vec2::new(160.0, target_y));
                    let target = game.world.entity_mut(target).unwrap();
                    target.engine_active = target_mode != 2;
                    target.alive = target_mode != 3;
                }
                let rng = game.rng.state_bits(); // Target's own Create draw is already complete.
                step_instance(&mut game, id);
                let arrow = raw(&game, id);
                assert!(!arrow.alive);
                assert!(game.world.entity(id).is_none());
                assert_eq!(
                    arrow.pos,
                    Vec2::new(if clamps { 164.0 } else { 162.0 }, 100.0),
                    "MOON_TAIL_AFTER_INHERITED_DESTROY {rules:?} {algorithm:?}"
                );
                assert_eq!(
                    arrow.vel,
                    if clamps {
                        Vec2::default()
                    } else {
                        Vec2::new(3.0, 0.2)
                    }
                );
                assert_eq!(
                    arrow.value("rotation"),
                    -(0.2_f64 / 3.0).atan(),
                    "angle precedes the Moon velocity reset"
                );
                assert_eq!(game.rng.state_bits(), rng);
            }
        }
    }
}

#[test]
fn retired_arrow_moon_uses_first_origin_but_whole_family_distance() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for algorithm in [RngAlgorithm::LegacySplitMix64V1, RngAlgorithm::Gm8Lcg32V1] {
            let (mut game, id) = fixture(rules, algorithm, true, "rMoon");
            game.spawn("oTarget", Vec2::new(100.0, 100.0));
            game.spawn("oTarget", Vec2::new(160.0, 100.0));
            let rng = game.rng.state_bits();
            step_instance(&mut game, id);
            let arrow = raw(&game, id);
            assert!(!arrow.alive);
            assert_eq!(
                arrow.pos,
                Vec2::new(104.0, 100.0),
                "first qualified object origin, not nearest target origin"
            );
            assert_eq!(arrow.vel, Vec2::default());
            assert_eq!(arrow.value("rotation"), -(0.2_f64 / 3.0).atan());
            assert_eq!(game.rng.state_bits(), rng);
        }
    }
}
