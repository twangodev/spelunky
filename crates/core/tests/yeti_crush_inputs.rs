//! Natural default falling-block route plus explicitly controlled initial states.
//! All setup precedes the first ordinary Game.step; no later game-state writes.
use spelunky_core::*;

fn room(rules: RulesPreset, algorithm: RngAlgorithm) -> Game {
    let mut game = Game::shell_with_algorithm(42, rules, algorithm).unwrap();
    game.world = World::new(30, 20, Area::Ice, 9);
    game.player.pos = Vec2::new(24.0, 152.0);
    game.player.previous_pos = game.player.pos;
    for x in 0..30 {
        game.world.set_tile(x, 10, "oBlock");
    }
    game
}
fn modes() -> impl Iterator<Item = (RulesPreset, RngAlgorithm)> {
    [RulesPreset::Classic, RulesPreset::Assisted]
        .into_iter()
        .flat_map(|r| {
            [RngAlgorithm::Gm8Lcg32V1, RngAlgorithm::LegacySplitMix64V1]
                .into_iter()
                .map(move |a| (r, a))
        })
}
fn sound_count(game: &Game) -> usize {
    game.events
        .iter()
        .filter(|event| matches!(event,GameEvent::Sound{name,..} if name=="sndCavemanDie"))
        .count()
}
fn literal_random(rng: &mut Rng) -> f64 {
    let a = rng.algorithm();
    let n = rng.next_u64();
    match a {
        RngAlgorithm::Gm8Lcg32V1 => n as f64 / 4294967296.0,
        RngAlgorithm::LegacySplitMix64V1 => (n >> 11) as f64 / 9007199254740992.0,
    }
}
fn literal_rand(rng: &mut Rng, width: u64) -> u32 {
    let a = rng.algorithm();
    let n = rng.next_u64();
    match a {
        RngAlgorithm::Gm8Lcg32V1 => ((n * width) >> 32) as u32,
        RngAlgorithm::LegacySplitMix64V1 => (n % width) as u32,
    }
}
fn blood(rng: &mut Rng) -> (Vec2, f64) {
    let vx = literal_random(rng) * 4.0 - literal_random(rng) * 4.0;
    let vy = -1.0 - literal_random(rng) * 2.0;
    (Vec2::new(vx, vy), f64::from(1 + literal_rand(rng, 6)) * 0.1)
}
#[test]
fn ordinary_falling_push_block_crushes_default_yeti_with_three_blood_and_no_credit() {
    for (rules, algorithm) in modes() {
        let mut game = room(rules, algorithm);
        let yeti = game.spawn("oYeti", Vec2::new(96.0, 144.0));
        let block = game.spawn("oPushBlock", Vec2::new(104.0, 128.0));
        assert_eq!(game.world.entity(yeti).unwrap().health, 5);
        assert_eq!(game.world.entity(yeti).unwrap().vel, Vec2::new(2.5, 0.0));
        assert_eq!(game.world.entity(block).unwrap().vel, Vec2::default());
        let mut crushed = false;
        for frame in 0..32 {
            let actor = game.world.entity(yeti).unwrap().clone();
            let block_before = game.world.entity(block).unwrap().clone();
            let birth = game.world.next_id;
            assert_eq!(actor.health, 5, "natural live body, frame {frame}");
            assert!(actor.state < 98);
            game.step(&InputFrame::default());
            assert_eq!(game.player.health, 4.0);
            if game.world.entity(yeti).is_none() {
                let block_after = game.world.entity(block).unwrap();
                assert!(
                    block_after.pos.y > block_before.pos.y,
                    "ordinary block fall must cause contact"
                );
                let drops: Vec<_> = game
                    .world
                    .entities
                    .iter()
                    .filter(|e| e.id >= birth && e.object == "oBlood")
                    .collect();
                assert_eq!(sound_count(&game), 1, "own crush sound exactly once");
                assert_eq!(game.data.get("kills").copied().unwrap_or(0.0), 0.0);
                assert_eq!(game.data.get("yetis").copied().unwrap_or(0.0), 0.0);
                assert_eq!(drops.len(), 3, "SOURCE_LIVE_CRUSH_BLOOD3");
                for drop in drops {
                    let origin = drop.previous_pos;
                    assert!(
                        block_after.sprite_bounds().contains(origin),
                        "crush Blood center is inside the falling solid"
                    );
                    assert_eq!(origin.y, 152.0);
                }
                crushed = true;
                game.step(&InputFrame::default());
                assert_eq!(
                    sound_count(&game),
                    0,
                    "retired body cannot die twice next frame"
                );
                assert_eq!(game.data.get("kills").copied().unwrap_or(0.0), 0.0);
                break;
            }
        }
        assert!(crushed, "SOURCE_NATURAL_CRUSH_PREREQUISITE");
    }
}
#[test]
fn ordinary_falling_block_away_leaves_yeti_alive_without_crush_blood() {
    for (rules, algorithm) in modes() {
        let mut game = room(rules, algorithm);
        let yeti = game.spawn("oYeti", Vec2::new(96.0, 144.0));
        let block = game.spawn("oPushBlock", Vec2::new(224.0, 128.0));
        let mut fell = false;
        for _ in 0..12 {
            game.step(&InputFrame::default());
            fell |= game.world.entity(block).unwrap().pos.y > 128.0;
            assert_eq!(game.world.entity(yeti).unwrap().health, 5);
            assert_eq!(sound_count(&game), 0);
            assert!(!game.world.entities.iter().any(|e| e.object == "oBlood"));
            assert_eq!(game.data.get("kills").copied().unwrap_or(0.0), 0.0);
        }
        assert!(fell);
    }
}
#[test]
fn controlled_initial_live_crush_runs_destroy_then_idle_rng_and_sight_after_blood() {
    for (rules, algorithm) in modes() {
        let mut game = room(rules, algorithm);
        game.world.set_tile(6, 9, "oBlock");
        let yeti = game.spawn("oYeti", Vec2::new(96.0, 144.0));
        let first = game.world.next_id;
        let mut expected = game.rng.clone();
        let expected_blood: Vec<_> = (0..3).map(|_| blood(&mut expected)).collect();
        let _facing = literal_rand(&mut expected, 2);
        game.step(&InputFrame::default());
        assert!(game.world.entity(yeti).is_none());
        let births: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|e| e.id >= first)
            .collect();
        assert_eq!(
            births.iter().map(|e| e.object.as_str()).collect::<Vec<_>>(),
            ["oBlood", "oBlood", "oBlood"],
            "SOURCE_DESTROY_CRUSH_BLOOD_ORDER"
        );
        // Own IDLE creates a sight ray after the Blood. Its later automatic
        // Solid collision retires it in this same frame inside the block.
        assert_eq!(
            game.world.next_id,
            first + 4,
            "SOURCE_DESTROY_THEN_IDLE_SIGHT_BIRTH"
        );
        assert!(
            !game
                .world
                .entities
                .iter()
                .any(|e| e.id >= first && e.object == "oEnemySight")
        );
        for (drop, (vel, gravity)) in births.iter().take(3).zip(expected_blood) {
            assert_eq!(drop.previous_pos, Vec2::new(104.0, 152.0));
            assert_eq!(drop.vel, vel);
            assert_eq!(drop.value("gravity"), gravity);
        }
        assert_eq!(
            game.rng.clone().next_u64(),
            expected.next_u64(),
            "complete Blood Create then IDLE tape"
        );
        assert_eq!(sound_count(&game), 1);
        assert_eq!(game.data.get("kills").copied().unwrap_or(0.0), 0.0);
    }
}
#[test]
fn controlled_initial_crushed_dead_and_stunned_bodies_keep_source_credit_branch_and_caps() {
    for (rules, algorithm) in modes() {
        // state, health, already_dead, countable, preexisting_detritus, bloodless
        for (state, hp, dead, countable, count, bloodless) in [
            (99, 0, false, true, 0, false),
            (99, 0, true, true, 0, false),
            (99, 0, false, false, 0, false),
            (98, 1, false, true, 0, false),
            (98, 1, false, true, 14, false),
            (98, 1, false, true, 16, false),
            (98, 1, false, true, 0, true),
        ] {
            let mut game = room(rules, algorithm);
            game.world.set_tile(6, 9, "oBlock");
            for _ in 0..count {
                game.spawn("oBone", Vec2::new(240.0, 80.0));
            }
            let yeti = game.spawn("oYeti", Vec2::new(96.0, 144.0));
            let actor = game.world.entity_mut(yeti).unwrap();
            actor.state = state;
            actor.health = hp;
            actor.vel = Vec2::default();
            actor.timer = 2;
            actor.data.insert("dead".into(), f64::from(dead));
            actor
                .data
                .insert("counts_as_kill".into(), f64::from(countable));
            actor.data.insert("bloodless".into(), f64::from(bloodless));
            let first = game.world.next_id;
            let amount = if bloodless {
                0
            } else {
                (if state == 98 { 4 } else { 3 }).min(16 - count)
            };
            let mut expected = game.rng.clone();
            for _ in 0..amount {
                let _ = blood(&mut expected);
            }
            game.step(&InputFrame::default());
            assert!(game.world.entity(yeti).is_none());
            let drops: Vec<_> = game
                .world
                .entities
                .iter()
                .filter(|e| e.id >= first && e.object == "oBlood")
                .collect();
            assert_eq!(drops.len(), amount, "SOURCE_CRUSH_BEFORE_OWN_BRANCH_BLOOD");
            let credit = usize::from(state == 99 && !dead && countable);
            assert_eq!(
                game.data.get("kills").copied().unwrap_or(0.0),
                credit as f64,
                "SOURCE_DEAD_BRANCH_CREDIT_AFTER_DESTROY"
            );
            assert_eq!(
                game.data.get("yetis").copied().unwrap_or(0.0),
                credit as f64
            );
            assert_eq!(sound_count(&game), if state == 99 && !dead { 2 } else { 1 });
            assert_eq!(game.rng.clone().next_u64(), expected.next_u64());
            game.step(&InputFrame::default());
            assert_eq!(
                game.data.get("kills").copied().unwrap_or(0.0),
                credit as f64
            );
            assert_eq!(sound_count(&game), 0);
        }
    }
}
