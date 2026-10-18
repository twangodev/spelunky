//! Authored initial shop walls and raw keeper guard controls, followed only by
//! real Mattock pickup/attack/AnimationEnd inputs. No mid-run actor or RNG writes.
//! scrShopkeeperAnger runs once per active keeper caller, selects nearest BEFORE
//! dead/angered guards, and only ATTACK ownStep latches angered afterward.
use spelunky_core::*;

#[derive(Clone, Copy, Debug)]
enum Storage {
    Dynamic,
    Terrain,
}
const STORAGES: [Storage; 2] = [Storage::Dynamic, Storage::Terrain];

fn modes() -> impl Iterator<Item = (RulesPreset, RngAlgorithm)> {
    [RulesPreset::Classic, RulesPreset::Assisted]
        .into_iter()
        .flat_map(|preset| {
            [RngAlgorithm::Gm8Lcg32V1, RngAlgorithm::LegacySplitMix64V1]
                .into_iter()
                .map(move |algorithm| (preset, algorithm))
        })
}

#[derive(Clone, Copy)]
enum Guard {
    None,
    Dead,
    Angered,
}
struct Scene {
    game: Game,
    storage: Storage,
    target: u64,
    mattock: u64,
    keepers: Vec<u64>,
    inactive: u64,
}

fn scene(
    preset: RulesPreset,
    algorithm: RngAlgorithm,
    storage: Storage,
    object: &str,
    shop_wall: bool,
    guards: &[Guard],
    colocated: bool,
    global_clean: bool,
) -> Scene {
    let mut game = Game::shell_with_algorithm(23, preset, algorithm).unwrap();
    game.world = World::new(80, 30, Area::Mines, 1);
    game.world.name = "rLevel".into();
    game.player.pos = Vec2::new(136.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.player.facing = 1;
    game.world.entrance = game.player.pos;
    for x in 0..80 {
        game.world.set_tile(x, 10, "oHardBlock");
    }
    let target = match storage {
        Storage::Dynamic => game.spawn(object, Vec2::new(144.0, 144.0)),
        Storage::Terrain => {
            game.world.set_tile(9, 9, object);
            game.spawn("oTerrainMetadata", Vec2::new(152.0, 152.0))
        }
    };
    // Explicit configured initial fields AFTER Create, which resets shopWall.
    let wall = game.world.entity_mut(target).unwrap();
    wall.data.insert("shopWall".into(), f64::from(shop_wall));
    wall.data
        .insert("cleanDeath".into(), f64::from(!global_clean));
    if global_clean {
        game.flags.insert("cleanSolids".into());
    }
    let mattock = game.spawn("oMattock", game.player.pos);
    // Authored free floor weapon; source crate/free-item producers clear cost.
    // A default positive price would trigger unrelated held-item theft first.
    let free_mattock = game.world.entity_mut(mattock).unwrap();
    free_mattock.data.insert("cost".into(), 0.0);
    free_mattock.data.insert("forSale".into(), 0.0);
    let mut keepers = Vec::new();
    for (index, guard) in guards.iter().enumerate() {
        let x = if colocated {
            256.0
        } else {
            256.0 + 48.0 * index as f64
        };
        let id = game.spawn("oShopkeeper", Vec2::new(x, 144.0));
        let keeper = game.world.entity_mut(id).unwrap();
        assert_eq!(keeper.health, 20);
        assert_eq!(keeper.state, 0);
        assert_eq!(keeper.value("dead"), 0.0);
        assert_eq!(keeper.value("angered"), 0.0);
        match guard {
            Guard::None => {}
            Guard::Dead => {
                // Raw source dead guard, not a claim of an earned corpse.
                keeper.data.insert("dead".into(), 1.0);
            }
            Guard::Angered => {
                keeper.data.insert("angered".into(), 1.0);
            }
        }
        keepers.push(id);
    }
    // Actual oLevel camera activation will freeze this default third keeper.
    let inactive = game.spawn("oShopkeeper", Vec2::new(800.0, 144.0));
    assert_eq!(game.data.get("thiefLevel").copied().unwrap_or(0.0), 0.0);
    assert!(!game.flags.contains("wanted"));
    assert!(!game.flags.contains("shopkeepers_angry"));
    Scene {
        game,
        storage,
        target,
        mattock,
        keepers,
        inactive,
    }
}

fn wall_alive(s: &Scene) -> bool {
    match s.storage {
        Storage::Dynamic => s.game.world.entity(s.target).is_some_and(|e| e.alive),
        Storage::Terrain => s.game.world.tile(9, 9).is_some_and(Tile::is_solid),
    }
}

fn vandal_messages(game: &Game) -> Vec<&SourceMessage> {
    game.events
        .iter()
        .filter_map(|event| match event {
            GameEvent::Dialogue(message) if message.first == "DIE, YOU VANDAL!" => Some(message),
            _ => None,
        })
        .collect()
}

fn drive_to_dig(s: &mut Scene) -> Rng {
    s.game.step(&InputFrame {
        down: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert_eq!(s.game.player.held, Some(s.mattock), "actual Mattock pickup");
    s.game.step(&InputFrame::default());
    assert_eq!(s.game.player.state, PlayerState::Standing);
    assert!(s.game.player.grounded);
    assert!(wall_alive(s));
    assert!(!s.game.world.entity(s.inactive).unwrap().engine_active);
    for id in &s.keepers {
        assert!(s.game.world.entity(*id).unwrap().engine_active);
    }
    assert_eq!(s.game.data.get("thiefLevel").copied().unwrap_or(0.0), 0.0);
    s.game.step(&InputFrame {
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    for _ in 0..40 {
        let hit = s
            .game
            .world
            .entities
            .iter()
            .find(|e| e.alive && e.object == "oMattockHit")
            .cloned();
        let before_rng = s.game.rng.clone();
        s.game.step(&InputFrame::default());
        if !wall_alive(s) {
            let hit = hit.expect("real MattockHit must precede terminal AnimationEnd");
            assert_eq!(hit.pos, Vec2::new(152.0, 152.0));
            assert!(
                hit.animation + hit.image_speed()
                    >= catalog::sprite_frame_count(&hit.sprite) as f64
            );
            assert!(s.game.world.entity(hit.id).is_none());
            assert_eq!(s.game.player.health, 4.0);
            let inactive = s.game.world.entity(s.inactive).unwrap();
            assert!(!inactive.engine_active);
            assert_eq!(inactive.state, 0);
            assert_eq!(inactive.value("angered"), 0.0);
            assert_eq!(inactive.pos, Vec2::new(800.0, 144.0));
            assert_eq!(inactive.health, 20);
            // cleanDeath OR global.cleanSolids suppresses only child rubble;
            // the successful Mattock tail still consumes rand(1,20).
            let mut expected = before_rng.clone();
            let _ = expected.range(20);
            assert_eq!(
                s.game.rng.clone().next_u64(),
                expected.next_u64(),
                "anger itself has no RNG; no keeper gun/jump/welcome in this scene"
            );
            assert!(
                !s.game
                    .world
                    .entities
                    .iter()
                    .any(|e| catalog::is_a(&e.object, "oRubblePiece"))
            );
            let sounds: Vec<_> = s
                .game
                .events
                .iter()
                .filter_map(|event| match event {
                    GameEvent::Sound { name, .. } | GameEvent::SoundIfIdle { name, .. } => {
                        Some(name.as_str())
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(
                sounds.len(),
                1,
                "only Mattock durability-result sound follows silent anger script"
            );
            assert!(matches!(sounds[0], "sndCrunch" | "sndMattockBreak"));
            return before_rng;
        }
        assert_eq!(
            s.game.data.get("thiefLevel").copied().unwrap_or(0.0),
            0.0,
            "crime occurs at the actual wall Destroy"
        );
        assert!(vandal_messages(&s.game).is_empty());
    }
    panic!("ordinary pickup and attack must destroy the configured shop wall");
}

fn assert_crime(s: &Scene, calls: usize, expected_thief: f64) {
    let messages = vandal_messages(&s.game);
    assert_eq!(
        s.game.data.get("thiefLevel").copied().unwrap_or(0.0),
        expected_thief,
        "source calls nearest anger once from EACH active keeper origin; keepers={:?}, messages={messages:?}",
        s.keepers
            .iter()
            .map(|id| {
                let e = s.game.world.entity(*id).unwrap();
                (e.state, e.value("dead"), e.value("angered"))
            })
            .collect::<Vec<_>>()
    );
    assert_eq!(
        messages.len(),
        calls,
        "port Dialogue calls witness repeated source message assignments, not multiple original displayed messages"
    );
    for message in messages {
        assert_eq!(message.second, "");
        assert_eq!(message.ticks, 80);
    }
    assert_eq!(s.game.flags.contains("wanted"), calls > 0);
    assert!(
        !s.game.flags.contains("shopkeepers_angry"),
        "Solid Destroy does not write a separate unconditional alert"
    );
    if calls > 0 {
        assert_eq!(s.game.data.get("message_timer").copied(), Some(80.0));
    } else {
        assert!(
            s.game
                .events
                .iter()
                .all(|event| !matches!(event, GameEvent::Dialogue(_)))
        );
        assert_eq!(
            s.game.data.get("message_timer").copied().unwrap_or(0.0),
            0.0
        );
    }
    assert!(
        s.game
            .events
            .iter()
            .all(|event| !matches!(event, GameEvent::Sound { name, .. } if name=="sndShotgun")),
        "anger script itself does not play a sound"
    );
}

#[test]
fn ordinary_mattock_destroys_dynamic_and_terrain_shopwall_and_calls_both_active_keepers() {
    for (preset, algorithm) in modes() {
        for storage in STORAGES {
            let mut s = scene(
                preset,
                algorithm,
                storage,
                "oBrickSmooth",
                true,
                &[Guard::None, Guard::None],
                false,
                false,
            );
            drive_to_dig(&mut s);
            assert_crime(&s, 2, 5.0);
            for id in &s.keepers {
                let keeper = s.game.world.entity(*id).unwrap();
                assert_eq!(keeper.state, 2);
                assert_eq!(
                    keeper.value("angered"),
                    0.0,
                    "AnimationEnd follows ownStep; script only assigns status"
                );
            }
            s.game.step(&InputFrame::default());
            for id in &s.keepers {
                assert_eq!(s.game.world.entity(*id).unwrap().value("angered"), 1.0);
            }
        }
    }
}

#[test]
fn configured_global_clean_solids_still_runs_unguarded_parent_shopwall_anger() {
    for (preset, algorithm) in modes() {
        for storage in STORAGES {
            let mut s = scene(
                preset,
                algorithm,
                storage,
                "oBrickSmooth",
                true,
                &[Guard::None, Guard::None],
                false,
                true,
            );
            drive_to_dig(&mut s);
            assert_crime(&s, 2, 5.0);
        }
    }
}

#[test]
fn configured_nearest_dead_or_already_angered_keeper_does_not_hide_distinct_eligible_caller() {
    for (preset, algorithm) in modes() {
        for storage in STORAGES {
            for guard in [Guard::Dead, Guard::Angered] {
                let mut s = scene(
                    preset,
                    algorithm,
                    storage,
                    "oBrickSmooth",
                    true,
                    &[guard, Guard::None],
                    false,
                    false,
                );
                drive_to_dig(&mut s);
                assert_crime(&s, 1, 2.0);
                let eligible = s.game.world.entity(s.keepers[1]).unwrap();
                assert_eq!(eligible.state, 2);
                assert_eq!(eligible.value("angered"), 0.0);
            }
        }
    }
}

#[test]
fn configured_colocated_callers_repeat_first_nearest_before_ownstep_angered_latch() {
    for (preset, algorithm) in modes() {
        for storage in STORAGES {
            let mut s = scene(
                preset,
                algorithm,
                storage,
                "oBrickSmooth",
                true,
                &[Guard::None, Guard::None],
                true,
                false,
            );
            drive_to_dig(&mut s);
            assert_crime(&s, 2, 5.0);
            let first = s.game.world.entity(s.keepers[0]).unwrap();
            let second = s.game.world.entity(s.keepers[1]).unwrap();
            assert_eq!((first.state, first.value("angered")), (2, 0.0));
            assert_eq!(
                (second.state, second.value("angered")),
                (0, 0.0),
                "second caller selected first keeper too; no immediate status write to second"
            );
            s.game.step(&InputFrame::default());
            let first = s.game.world.entity(s.keepers[0]).unwrap();
            let second = s.game.world.entity(s.keepers[1]).unwrap();
            assert_eq!((first.state, first.value("angered")), (2, 1.0));
            assert_eq!((second.state, second.value("angered")), (4, 0.0));
        }
    }
}

#[test]
fn configured_colocated_dead_or_angered_first_nearest_blocks_both_without_fallback() {
    for (preset, algorithm) in modes() {
        for storage in STORAGES {
            for guard in [Guard::Dead, Guard::Angered] {
                let mut s = scene(
                    preset,
                    algorithm,
                    storage,
                    "oBrickSmooth",
                    true,
                    &[guard, Guard::None],
                    true,
                    false,
                );
                drive_to_dig(&mut s);
                assert_crime(&s, 0, 0.0);
                assert_eq!(
                    s.game.world.entity(s.keepers[1]).unwrap().value("angered"),
                    0.0
                );
            }
        }
    }
}

#[test]
fn configured_no_active_eligible_keeper_leaves_global_alerts_and_messages_untouched() {
    for (preset, algorithm) in modes() {
        for storage in STORAGES {
            for guards in [vec![], vec![Guard::Dead, Guard::Angered]] {
                let mut s = scene(
                    preset,
                    algorithm,
                    storage,
                    "oBrickSmooth",
                    true,
                    &guards,
                    false,
                    false,
                );
                drive_to_dig(&mut s);
                assert_crime(&s, 0, 0.0);
            }
        }
    }
}

#[test]
fn ordinary_mattock_non_shopwall_control_has_no_keeper_crime() {
    for (preset, algorithm) in modes() {
        for storage in STORAGES {
            let mut s = scene(
                preset,
                algorithm,
                storage,
                "oBrickSmooth",
                false,
                &[Guard::None, Guard::None],
                false,
                false,
            );
            drive_to_dig(&mut s);
            assert_crime(&s, 0, 0.0);
            assert!(
                s.keepers
                    .iter()
                    .all(|id| s.game.world.entity(*id).unwrap().state == 0)
            );
        }
    }
}

#[test]
fn configured_shopwall_on_noninheriting_block_does_not_invoke_solid_parent_crime() {
    for (preset, algorithm) in modes() {
        for storage in STORAGES {
            let mut s = scene(
                preset,
                algorithm,
                storage,
                "oBlock",
                true,
                &[Guard::None, Guard::None],
                false,
                false,
            );
            drive_to_dig(&mut s);
            assert_crime(&s, 0, 0.0);
            assert!(
                s.keepers
                    .iter()
                    .all(|id| s.game.world.entity(*id).unwrap().state == 0)
            );
        }
    }
}
