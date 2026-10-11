//! Authored room, source-default Yeti/Pistol, ordinary pickup/fire, no mid-run writes.
use spelunky_core::{catalog, level::Level, *};

fn room(rules: RulesPreset) -> (Game, u64, u64) {
    let mut level = Level::default();
    level.set_tile(6, 8, '@').unwrap();
    level.set_tile(34, 8, 'X').unwrap();
    level.set_tile(9, 8, 'y').unwrap();
    // The supported ledge stops WALK and establishes a real positive IDLE counter.
    for x in 0..=10 {
        level.set_tile(x, 9, '1').unwrap();
    }
    // A lower platform catches ordinary shot knockback beyond the upper ledge.
    for x in 0..=20 {
        level.set_tile(x, 11, '1').unwrap();
    }
    // Keep the real Pistol Bullet knockback and first landing inside
    // the player's active viewport; the wall is beyond the IDLE ledge.
    for y in 7..=10 {
        level.set_tile(18, y, '1').unwrap();
    }
    // Select only initial source preconditions, never a hit/result or later route.
    for seed in 0..128 {
        let mut game = Game::shell(seed, rules);
        game.load_custom(&level, "yeti-stun-landing", "");
        if game.rng.clone().range(2) != 1 {
            continue;
        }
        let enemies: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|e| e.alive && catalog::is_a(&e.object, "oEnemy"))
            .collect();
        if enemies.len() != 1 || enemies[0].object != "oYeti" {
            continue;
        }
        let yeti = enemies[0].id;
        let source = game.world.entity(yeti).unwrap();
        assert_eq!(source.health, 5);
        assert_eq!(source.state, 0);
        assert_eq!(source.vel, Vec2::new(2.5, 0.0));
        assert_eq!(source.value("counter"), 0.0);
        assert_eq!(source.value("bounced"), 0.0);
        let pistol = game.spawn("oPistol", game.player.pos);
        return (game, yeti, pistol);
    }
    panic!("bounded initial-only quiet/right-facing source fixture must exist");
}

fn pickup_and_wait_for_idle(game: &mut Game, yeti: u64, pistol: u64) {
    game.step(&InputFrame {
        down: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert_eq!(game.player.held, Some(pistol), "actual pickup input");
    let mut saw_walk = false;
    for _ in 0..100 {
        let actor = game.world.entity(yeti).unwrap();
        saw_walk |= actor.state == 1;
        assert_eq!(actor.health, 5);
        assert_eq!(game.player.health, 4);
        assert!(!game.world.entities.iter().any(|e| e.object == "oBlood"));
        if actor.state == 0 && actor.value("counter") >= 20.0 {
            assert!(saw_walk);
            assert_eq!(actor.vel.x, 0.0);
            return;
        }
        game.step(&InputFrame::default());
    }
    panic!("ordinary default Yeti must naturally enter a positive-counter IDLE pause");
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
fn random(rng: &mut Rng) -> f64 {
    let algorithm = rng.algorithm();
    let raw = rng.next_u64();
    match algorithm {
        RngAlgorithm::Gm8Lcg32V1 => raw as f64 / 4_294_967_296.0,
        RngAlgorithm::LegacySplitMix64V1 => (raw >> 11) as f64 / 9_007_199_254_740_992.0,
    }
}

fn literal_blood(rng: &mut Rng) -> (Vec2, f64) {
    let x = random(rng) * 4.0 - random(rng) * 4.0;
    let y = -1.0 - random(rng) * 2.0;
    let gravity = f64::from(1 + rand(rng, 6)) * 0.1;
    (Vec2::new(x, y), gravity)
}

fn shoot_paused_yeti(game: &mut Game, yeti: u64, pistol: u64) {
    pickup_and_wait_for_idle(game, yeti, pistol);
    game.step(&InputFrame {
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    let mut saw_bullet = game
        .world
        .entities
        .iter()
        .any(|e| e.object == "oBullet" && e.vel.x > 0.0);
    let mut hit = false;
    for _ in 0..24 {
        let actor = game.world.entity(yeti).unwrap();
        if actor.health < 5 {
            assert_eq!(
                actor.health, 1,
                "one actual pistol Bullet deals source four damage"
            );
            assert_eq!(actor.state, 98);
            assert_eq!(actor.timer, 20);
            assert!(
                actor.value("counter") > 0.0,
                "real pre-hit idle pause remains in port alias"
            );
            assert_eq!(actor.value("bounced"), 0.0);
            assert!(saw_bullet);
            hit = true;
            break;
        }
        game.step(&InputFrame::default());
        saw_bullet |= game
            .world
            .entities
            .iter()
            .any(|e| e.object == "oBullet" && e.vel.x > 0.0);
    }
    assert!(
        hit,
        "ordinary pickup/fire must genuinely hit the naturally paused Yeti"
    );
}

#[test]
fn ordinary_pistol_hit_yeti_bleeds_once_on_landing_and_recovers_without_stale_idle_pause() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, yeti, pistol) = room(rules);
        shoot_paused_yeti(&mut game, yeti, pistol);
        let mut landings = 0;
        let mut recovered = false;
        for _ in 0..240 {
            let before = game.world.entity(yeti).unwrap().clone();
            assert_eq!(before.state, 98);
            let next_id = game.world.next_id;
            let mut expected = game.rng.clone();
            game.step(&InputFrame::default());
            let actor = game.world.entity(yeti).unwrap();
            let first_landing = before.value("bounced") == 0.0 && actor.value("bounced") != 0.0;
            let drops: Vec<_> = game
                .world
                .entities
                .iter()
                .filter(|e| e.id >= next_id && e.object == "oBlood")
                .collect();
            assert_eq!(
                drops.len(),
                usize::from(first_landing),
                "source first-supported stun birth only"
            );
            if first_landing {
                landings += 1;
                let (velocity, gravity) = literal_blood(&mut expected);
                assert_eq!(
                    drops[0].previous_pos,
                    Vec2::new(actor.pos.x + 8.0, actor.pos.y + 8.0)
                );
                assert_eq!(
                    drops[0].vel, velocity,
                    "newborn bounce alarm cannot run before next frame"
                );
                assert_eq!(drops[0].value("gravity"), gravity);
                assert_eq!(drops[0].value("bounce"), 0.0);
                assert!(
                    actor.timer < before.timer,
                    "landing Blood precedes this supported counter decrement"
                );
            }
            // During this quiet STUNNED phase, existing Blood/Trail and sight
            // movement have no RNG. Literal Blood Create is the complete frame tape.
            assert_eq!(game.rng.clone().next_u64(), expected.next_u64());
            assert_eq!(actor.health, 1);
            assert_eq!(game.player.health, 4);
            if actor.state == 0 {
                assert_eq!(
                    before.timer, 0,
                    "source else-if recovery occurs after the zero decrement tick"
                );
                assert_eq!(actor.timer, 0);
                assert_eq!(
                    actor.value("counter"),
                    0.0,
                    "expired stun cannot revive old idle pause"
                );
                assert_eq!(landings, 1);
                recovered = true;
                break;
            }
        }
        assert!(
            recovered,
            "natural supported stun must recover within its bounded trajectory"
        );
        let mut expected = game.rng.clone();
        let facing = if rand(&mut expected, 2) == 0 { -1 } else { 1 };
        game.step(&InputFrame::default());
        let actor = game.world.entity(yeti).unwrap();
        assert_eq!(actor.state, 1, "next IDLE Step immediately returns to WALK");
        assert_eq!(actor.value("counter"), 0.0);
        assert_eq!(actor.value("bounced"), 0.0);
        assert_eq!(actor.facing, facing);
        assert_eq!(game.rng.clone().next_u64(), expected.next_u64());
        assert_eq!(game.player.health, 4);
        assert_eq!(game.player.held, Some(pistol));
    }
}

#[test]
fn ordinary_pistol_fired_away_from_yeti_does_not_stun_or_create_landing_blood() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, yeti, pistol) = room(rules);
        pickup_and_wait_for_idle(&mut game, yeti, pistol);
        game.step(&InputFrame {
            left: true,
            ..Default::default()
        });
        assert_eq!(game.player.facing, -1);
        game.step(&InputFrame {
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert!(
            game.world
                .entities
                .iter()
                .any(|e| e.object == "oBullet" && e.vel.x < 0.0)
        );
        for _ in 0..24 {
            game.step(&InputFrame::default());
            let actor = game.world.entity(yeti).unwrap();
            assert_eq!(actor.health, 5);
            assert_ne!(actor.state, 98);
            assert!(!game.world.entities.iter().any(|e| e.object == "oBlood"));
            assert_eq!(game.player.health, 4);
            assert_eq!(game.player.held, Some(pistol));
        }
    }
}

#[test]
fn ordinary_pistol_hit_yeti_exhausts_idle_counter_before_recovery() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, yeti, pistol) = room(rules);
        shoot_paused_yeti(&mut game, yeti, pistol);
        let mut recovered = false;
        for _ in 0..240 {
            let before = game.world.entity(yeti).unwrap().clone();
            assert_eq!(before.state, 98);
            game.step(&InputFrame::default());
            let actor = game.world.entity(yeti).unwrap();
            assert_eq!(actor.health, 1);
            assert_eq!(game.player.health, 4);
            if actor.state == 0 {
                assert_eq!(before.timer, 0);
                assert_eq!(actor.timer, 0);
                assert_eq!(
                    actor.value("counter"),
                    0.0,
                    "source one shared counter cannot resurrect the pre-hit idle pause"
                );
                recovered = true;
                break;
            }
        }
        assert!(recovered, "actual ordinary hit must settle and recover");
        let mut expected = game.rng.clone();
        let facing = if rand(&mut expected, 2) == 0 { -1 } else { 1 };
        game.step(&InputFrame::default());
        let actor = game.world.entity(yeti).unwrap();
        assert_eq!(actor.state, 1);
        assert_eq!(actor.facing, facing);
        assert_eq!(actor.value("counter"), 0.0);
        assert_eq!(actor.value("bounced"), 0.0);
        assert_eq!(game.rng.clone().next_u64(), expected.next_u64());
    }
}
