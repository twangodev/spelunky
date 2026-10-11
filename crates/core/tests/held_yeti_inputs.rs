//! Ordinary Rock pickup/throw, default Yeti stun, chase/pickup, carry and release.
//! Initial authored room/seed selection only: no mid-run state/pose/HP/RNG writes.
use spelunky_core::{catalog, level::Level, *};

fn room(rules: RulesPreset, algorithm: RngAlgorithm) -> (Game, u64, u64) {
    let mut level = Level::default();
    level.set_tile(6, 8, '@').unwrap();
    level.set_tile(34, 8, 'X').unwrap();
    level.set_tile(9, 8, 'y').unwrap();
    for x in 0..=10 {
        level.set_tile(x, 9, '1').unwrap();
    }
    for x in 0..=20 {
        level.set_tile(x, 11, '1').unwrap();
    }
    for y in 7..=10 {
        level.set_tile(18, y, '1').unwrap();
    }
    for seed in 0..128 {
        let mut g = Game::shell_with_algorithm(seed, rules, algorithm).unwrap();
        g.load_custom(&level, "held-yeti-gravity", "");
        if g.rng.clone().range(2) != 1 {
            continue;
        }
        let enemies: Vec<_> = g
            .world
            .entities
            .iter()
            .filter(|e| e.alive && catalog::is_a(&e.object, "oEnemy"))
            .collect();
        if enemies.len() != 1 || enemies[0].object != "oYeti" {
            continue;
        }
        let id = enemies[0].id;
        let actor = g.world.entity(id).unwrap();
        assert_eq!(actor.health, 5);
        assert_eq!(actor.state, 0);
        assert_eq!(actor.vel, Vec2::new(2.5, 0.0));
        let rock = g.spawn("oRock", g.player.pos);
        return (g, id, rock);
    }
    panic!("initial-only source-default right-facing fixture unavailable");
}
fn pickup_rock_and_pause(g: &mut Game, yeti: u64, rock: u64) {
    g.step(&InputFrame {
        down: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert_eq!(g.player.held, Some(rock), "SOURCE_ROCK_PICKUP");
    let mut walked = false;
    for _ in 0..100 {
        let e = g.world.entity(yeti).unwrap();
        assert_eq!(e.health, 5);
        assert_eq!(g.player.health, 4);
        walked |= e.state == 1;
        if e.state == 0 && e.value("counter") >= 20.0 {
            assert!(walked);
            return;
        }
        g.step(&InputFrame::default());
    }
    panic!("SOURCE_ORDINARY_IDLE_PAUSE");
}
fn throw_rock_and_stun(g: &mut Game, yeti: u64, rock: u64) {
    pickup_rock_and_pause(g, yeti, rock);
    // A crouch-thrown Rock lands and applies source floor friction quickly.
    // Approach through real movement before throwing; remain outside grab reach.
    let mut near = false;
    for _ in 0..32 {
        let actor = g.world.entity(yeti).unwrap();
        let gap = actor.pos.x + 8.0 - g.player.pos.x;
        assert_eq!(actor.health, 5);
        assert_eq!(g.player.health, 4);
        if gap <= 28.0 {
            assert!(gap > 16.0);
            near = true;
            break;
        }
        g.step(&InputFrame {
            right: true,
            ..Default::default()
        });
    }
    assert!(near, "SOURCE_ORDINARY_APPROACH_BEFORE_THROW");
    g.step(&InputFrame {
        down: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert_eq!(g.player.held, None, "ordinary crouch throw frees hand");
    assert!(!g.world.entity(rock).unwrap().held);
    let mut hit = false;
    for _ in 0..64 {
        let e = g.world.entity(yeti).unwrap();
        assert_eq!(g.player.health, 4);
        if e.health == 4 {
            assert_eq!(e.state, 98);
            assert!(e.timer >= 190);
            assert!(!e.held);
            hit = true;
            break;
        }
        assert_eq!(e.health, 5, "only one source Rock damage point");
        g.step(&InputFrame::default());
    }
    assert!(hit, "SOURCE_ORDINARY_ROCK_STUN");
}
#[test]
fn ordinary_rock_stun_then_enemy_pickup_carry_and_release_applies_gravity_only_unheld() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for algorithm in [RngAlgorithm::Gm8Lcg32V1, RngAlgorithm::LegacySplitMix64V1] {
            let (mut g, yeti, rock) = room(rules, algorithm);
            throw_rock_and_stun(&mut g, yeti, rock);
            // Positive control: one unobstructed airborne unheld Step retains gravity.
            let before = g.world.entity(yeti).unwrap().clone();
            g.step(&InputFrame::default());
            let e = g.world.entity(yeti).unwrap();
            assert!(!e.held);
            assert_eq!(e.state, 98);
            assert_eq!(e.vel.y, before.vel.y + 0.6, "SOURCE_UNHELD_GRAVITY_CONTROL");
            let mut picked = false;
            let mut cleared_rock = false;
            for _ in 0..160 {
                let e = g.world.entity(yeti).unwrap();
                assert_eq!(e.health, 4);
                assert_eq!(
                    e.state, 98,
                    "stun must remain positive until ordinary pickup"
                );
                assert_eq!(g.player.health, 4);
                let dx = e.pos.x + 8.0 - g.player.pos.x;
                let near = dx.abs() <= 7.0 && (g.player.pos.y - (e.pos.y + 8.0)).abs() <= 10.0;
                let input = if near {
                    InputFrame {
                        down: true,
                        attack: true,
                        attack_pressed: true,
                        ..Default::default()
                    }
                } else {
                    InputFrame {
                        left: dx < -3.0,
                        right: dx > 3.0,
                        ..Default::default()
                    }
                };
                g.step(&input);
                if g.player.held == Some(yeti) {
                    picked = true;
                    break;
                }
                if let Some(held) = g.player.held {
                    assert_eq!(
                        held, rock,
                        "the sole nearby source Item has pickup precedence"
                    );
                    assert!(
                        !cleared_rock,
                        "the thrown-away Rock must no longer obstruct Enemy pickup"
                    );
                    cleared_rock = true;
                    // Item pickup precedes Enemy pickup. Face away from the stunned
                    // Yeti, then use the ordinary standing throw to clear that Item.
                    g.step(&InputFrame {
                        right: true,
                        ..Default::default()
                    });
                    assert_eq!(g.player.facing, 1);
                    g.step(&InputFrame {
                        attack: true,
                        attack_pressed: true,
                        ..Default::default()
                    });
                    assert_eq!(g.player.held, None);
                    assert!(g.world.entity(rock).unwrap().vel.x > 2.0);
                    assert_eq!(g.world.entity(yeti).unwrap().health, 4);
                }
            }
            assert!(picked, "SOURCE_ORDINARY_YETI_PICKUP");
            let e = g.world.entity(yeti).unwrap();
            assert!(e.held);
            assert_eq!(e.state, 98);
            assert!(e.timer > 0);
            assert_eq!(e.health, 4);
            assert_eq!(g.player.pickup_item_type, "Yeti");
            assert_eq!(
                e.vel,
                Vec2::default(),
                "SOURCE_HELD_POSITIVE_COUNTER_HAS_NO_GRAVITY"
            );
            for frame in 0..8 {
                let before = g.world.entity(yeti).unwrap().clone();
                let input = InputFrame {
                    right: frame < 3,
                    jump: frame >= 3,
                    jump_pressed: frame == 3,
                    ..Default::default()
                };
                g.step(&input);
                let e = g.world.entity(yeti).unwrap();
                assert!(e.held);
                assert_eq!(g.player.held, Some(yeti));
                assert_eq!(e.health, 4);
                assert_eq!((e.state, e.timer), (98, before.timer - 1));
                assert_eq!(
                    e.vel,
                    Vec2::default(),
                    "source carrying never adds gravity, including player jump"
                );
                assert_eq!(
                    e.pos,
                    Vec2::new(
                        g.player.pos.x + if g.player.facing < 0 { -12.0 } else { -4.0 },
                        g.player.pos.y - 12.0
                    )
                );
                assert_eq!(g.player.health, 4);
            }
            assert!(
                g.player.vel.y < 0.0,
                "ordinary carry jump is genuinely airborne"
            );
            g.step(&InputFrame {
                attack: true,
                attack_pressed: true,
                ..Default::default()
            });
            let e = g.world.entity(yeti).unwrap();
            assert!(!e.held);
            assert_eq!(g.player.held, None);
            assert_eq!(e.state, 98);
            // oEnemy Create sets heavy=true; scrUseItem629 gives -2 before own +.6.
            assert_eq!(e.value("heavy"), 1.0);
            assert_eq!(e.vel.y, -1.4, "SOURCE_RELEASE_RESTORES_GRAVITY_CONTROL");
        }
    }
}
