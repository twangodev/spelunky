//! Olmec's invisible, live slam mask and synchronous Solid destruction.
//! The generated intro uses only ordinary Right input. Configured impact
//! controls choose a source SLAM pose before the first Step; no mid-run writes.
use spelunky_core::*;
use std::collections::BTreeSet;

fn modes() -> impl Iterator<Item = (RulesPreset, RngAlgorithm)> {
    [RulesPreset::Classic, RulesPreset::Assisted]
        .into_iter()
        .flat_map(|rules| {
            [RngAlgorithm::Gm8Lcg32V1, RngAlgorithm::LegacySplitMix64V1]
                .into_iter()
                .map(move |algorithm| (rules, algorithm))
        })
}
fn sound_count(game: &Game) -> usize {
    game.events
        .iter()
        .filter(|event| matches!(event, GameEvent::Sound { name, .. } if name == "sndSlam"))
        .count()
}
fn initial_ids(game: &Game) -> BTreeSet<u64> {
    game.world.entities.iter().map(|e| e.id).collect()
}
fn fresh_slam<'a>(game: &'a Game, prior: &BTreeSet<u64>) -> &'a Entity {
    let fresh = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oOlmecSlam" && !prior.contains(&e.id))
        .collect::<Vec<_>>();
    assert_eq!(fresh.len(), 1, "ONE_FRESH_OWN_STEP_SLAM");
    fresh[0]
}
fn literal_mask_edges(slam: &Entity) {
    let p = slam.pos;
    assert_eq!(slam.sprite, "sOlmecSlam");
    assert_eq!(slam.sprite_bounds(), Rect::new(p.x + 2.0, p.y, 59.0, 8.0));
    for q in [Vec2::new(p.x + 2.0, p.y), Vec2::new(p.x + 60.0, p.y + 7.0)] {
        assert!(slam.mask_contains(q, CollisionPrecision::Precise));
    }
    for q in [
        Vec2::new(p.x + 1.0, p.y),
        Vec2::new(p.x + 61.0, p.y),
        Vec2::new(p.x + 2.0, p.y + 8.0),
    ] {
        assert!(!slam.mask_contains(q, CollisionPrecision::Precise));
    }
    assert!(!catalog::object("oOlmecSlam").unwrap().visible);
}
#[test]
fn source_slam_create_keeps_live_invisible_mask_and_alarm_retires_before_next_collision() {
    for (rules, algorithm) in modes() {
        let mut game = Game::shell_with_algorithm(42, rules, algorithm).unwrap();
        let prior_rng = game.rng.state_bits();
        let id = game.spawn("oOlmecSlam", Vec2::new(80.0, 80.0));
        let slam = game.world.entity(id).unwrap();
        literal_mask_edges(slam);
        assert_eq!(slam.alarms[0], 1);
        assert_eq!(sound_count(&game), 1, "ONE_SYNCHRONOUS_CREATE_SOUND");
        assert_eq!(
            game.rng.state_bits(),
            prior_rng,
            "Slam Create has no random calls"
        );
        game.step(&InputFrame::default());
        assert!(
            game.world.entity(id).is_none(),
            "ALARM_ZERO_RETIRES_BEFORE_COLLISION"
        );
        assert_eq!(
            sound_count(&game),
            0,
            "Create cannot replay at bootstrap/Alarm"
        );
        assert_eq!(game.rng.state_bits(), prior_rng);
        assert_eq!(game.player.health, 4);
    }
}

// Independent literal random-width tape. Canonical is the accepted u32 GM8
// integer-width contract; Legacy is explicitly the historical port algorithm.
struct Tape {
    algorithm: RngAlgorithm,
    state: u64,
}
impl Tape {
    fn draw(&mut self, width: u32) -> u32 {
        match self.algorithm {
            RngAlgorithm::Gm8Lcg32V1 => {
                self.state =
                    u64::from((self.state as u32).wrapping_mul(0x08088405).wrapping_add(1));
                ((self.state * u64::from(width)) >> 32) as u32
            }
            RngAlgorithm::LegacySplitMix64V1 => {
                self.state = self.state.wrapping_add(0x9e3779b97f4a7c15);
                let mut z = self.state;
                z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
                z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
                ((z ^ (z >> 31)) % u64::from(width)) as u32
            }
        }
    }
}
#[test]
fn own_step_slam_destroys_invincible_solids_with_clean_override_and_literal_rubble_tape() {
    for (rules, algorithm) in modes() {
        for terrain in [false, true] {
            let mut game = Game::shell_with_algorithm(42, rules, algorithm).unwrap();
            // Healthy default Player at40,40 stands on an initial authored floor.
            for x in 1..=3 {
                game.world.set_tile(x, 3, "oHardBlock");
            }
            let mut targets = Vec::new();
            for x in 6..=10 {
                let id = if terrain {
                    game.world.set_tile(x, 10, "oBrickSmooth");
                    // Explicit initial terrain descriptor carries source
                    // instance fields; World::set_tile itself creates no proxy.
                    game.world
                        .spawn("oTerrainMetadata", Vec2::new(x as f64 * 16.0 + 8.0, 168.0))
                } else {
                    game.spawn("oBrickSmooth", Vec2::new(x as f64 * 16.0, 160.0))
                };
                let target = game.world.entity_mut(id).unwrap();
                target.data.insert("invincible".into(), 1.0);
                target.data.insert("cleanDeath".into(), 1.0);
                targets.push(id);
            }
            let boss = game.spawn("oOlmec", Vec2::new(107.0, 96.0));
            // Explicit initial source-compatible ground impact, not a claimed
            // generated encounter. These are the only actor overrides.
            let actor = game.world.entity_mut(boss).unwrap();
            actor.state = 6;
            actor.vel = Vec2::new(0.0, 5.0);
            actor.set_sprite("sOlmec");
            actor.data.insert("slammed".into(), 0.0);
            game.world.fluid_dirty = false;
            let prior = initial_ids(&game);
            let mut tape = Tape {
                algorithm,
                state: game.rng.state_bits(),
            };
            let mut expected = Vec::new();
            for x in 6..=10 {
                for (object, sprite) in [
                    ("oRubble", "sRubbleTan"),
                    ("oRubbleSmall", "sRubbleTanSmall"),
                    ("oRubbleSmall", "sRubbleTanSmall"),
                ] {
                    let px =
                        x as f64 * 16.0 + 8.0 + f64::from(tape.draw(9)) - f64::from(tape.draw(9));
                    let py = 168.0 + f64::from(tape.draw(9)) - f64::from(tape.draw(9));
                    expected.push((object.to_owned(), sprite.to_owned(), Vec2::new(px, py)));
                }
            }
            assert_eq!(game.player.health, 4);
            assert_eq!(game.player.invincible, 0);
            assert_eq!(game.player.stun, 0);
            game.step(&InputFrame::default());
            let slam = fresh_slam(&game, &prior);
            let slam_id = slam.id;
            assert_eq!(
                slam.pos,
                Vec2::new(107.0, 165.0),
                "GLOBAL_FIVE_THEN_BLOCKED_OWN_MOVEMENT"
            );
            assert_eq!(
                slam.alarms[0], 1,
                "birth occurs after this tick's Alarm phase"
            );
            eprintln!(
                "QUALIFIED_AUTHORED_IMPACT {rules:?}/{algorithm:?} terrain={terrain} slam={:?} sprite={:?}",
                slam.pos, slam.sprite
            );
            for (x, id) in (6..=10).zip(targets) {
                if terrain {
                    assert!(
                        game.world.tile(x, 10).unwrap().object.is_empty(),
                        "SLAM_MUST_DESTROY_ACTIVE_SOLID_CALLBACK"
                    );
                }
                assert!(
                    game.world.entity(id).is_none(),
                    "SLAM_MUST_RETIRE_TARGET_AND_METADATA"
                );
            }
            assert!(
                game.world.fluid_dirty,
                "SOURCE_INHERITED_SOLID_REFRESHES_WATER"
            );
            let actual = game
                .world
                .entities
                .iter()
                .filter(|e| {
                    !prior.contains(&e.id)
                        && matches!(e.object.as_str(), "oRubble" | "oRubbleSmall")
                })
                .map(|e| (e.object.clone(), e.sprite.clone(), e.pos))
                .collect::<Vec<_>>();
            assert_eq!(
                actual, expected,
                "SYNCHRONOUS_PARENT_THEN_THREE_RUBBLE_PER_TARGET"
            );
            assert_eq!(
                game.rng.state_bits(),
                tape.state,
                "FIVE_TARGETS_TWELVE_DRAWS_EACH"
            );
            assert_eq!(
                sound_count(&game),
                1,
                "local producer cannot double emit Create sound"
            );
            literal_mask_edges(game.world.entity(slam_id).unwrap());
            assert_eq!(game.player.health, 4);
            let after_rng = game.rng.state_bits();
            game.step(&InputFrame::default());
            assert!(game.world.entity(slam_id).is_none());
            assert_eq!(sound_count(&game), 0);
            assert_eq!(
                game.rng.state_bits(),
                after_rng,
                "Alarm cannot repeat target destruction"
            );
            assert_eq!(game.player.health, 4);
        }
    }
}

#[test]
fn unchanged_generated_intro_right_input_reaches_slam_that_removes_literal_temple_mask_targets() {
    for (rules, algorithm) in modes() {
        let mut game = Game::new_with_algorithm(42, rules, algorithm).unwrap();
        while game.world.level < 16 {
            game.next_level();
        }
        let boss = game
            .world
            .entities
            .iter()
            .find(|e| e.object == "oOlmec")
            .unwrap()
            .id;
        assert_eq!(game.player.pos, Vec2::new(24.0, 552.0));
        assert_eq!(game.player.health, 4);
        let mut saw_prepare = false;
        let mut saw_slam = false;
        let mut qualified = false;
        for _ in 0..600 {
            let before = game.world.entity(boss).unwrap().clone();
            let prior = initial_ids(&game);
            let before_tiles = game.world.tiles.clone();
            game.step(&InputFrame {
                right: true,
                ..Default::default()
            });
            assert_eq!(game.player.health, 4);
            saw_prepare |= game.world.entity(boss).unwrap().state == 5;
            saw_slam |= game.world.entity(boss).unwrap().state == 6;
            let slams = game
                .world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == "oOlmecSlam" && !prior.contains(&e.id))
                .collect::<Vec<_>>();
            if before.state != 6 || before.value("slammed") != 0.0 || slams.is_empty() {
                continue;
            }
            assert!(
                saw_prepare && saw_slam,
                "MANDATORY_NATURAL_PREPARE_THEN_SLAM"
            );
            assert_eq!(slams.len(), 1, "ONE_FIRST_GROUNDED_FOOT_SLAM");
            let slam = slams[0];
            assert_eq!(slam.alarms[0], 1);
            let (left, top, right, bottom) = (
                slam.pos.x + 2.0,
                slam.pos.y,
                slam.pos.x + 60.0,
                slam.pos.y + 7.0,
            );
            // Original sTemple is RECTANGLE/AUTO [0,0,15,15]; these are
            // literal source pixel intersections, independent of actualSlam mask.
            let expected = before_tiles
                .iter()
                .enumerate()
                .filter_map(|(i, tile)| {
                    let x = (i % game.world.width) as f64 * 16.0;
                    let y = (i / game.world.width) as f64 * 16.0;
                    (tile.object == "oTemple"
                        && x <= right
                        && x + 15.0 >= left
                        && y <= bottom
                        && y + 15.0 >= top)
                        .then_some(i)
                })
                .collect::<Vec<_>>();
            assert!(
                expected.len() >= 4,
                "MANDATORY_AUTHORED_FLOOR_SOLIDS_UNDER_SOURCE_MASK"
            );
            assert!(expected.iter().all(|&i| before_tiles[i].engine_active));
            eprintln!(
                "QUALIFIED_NATURAL_FOOT_SLAM {rules:?}/{algorithm:?} tick={} pos={:?} source_tiles={expected:?} current_sprite={:?}",
                game.tick, slam.pos, slam.sprite
            );
            for i in expected {
                assert!(
                    game.world.tiles[i].object.is_empty(),
                    "NATURAL_SOURCE_SLAM_MASK_MUST_REMOVE_TEMPLE_FLOOR"
                );
            }
            assert_eq!(sound_count(&game), 1);
            literal_mask_edges(slam);
            assert_eq!(game.player.data.get("active"), Some(&0.0));
            qualified = true;
            break;
        }
        assert!(qualified, "NATURAL_GENERATED_FOOT_IMPACT_PREREQUISITE");
    }
}
