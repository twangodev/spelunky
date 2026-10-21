//! Public input routes with source-created initial bombs; no actor writes after
//! the first Game.step. These do not claim a natural FireFrog death producer.
use spelunky_core::{catalog, level::Level, *};

fn room(rules: RulesPreset, shop: bool, water: bool, trap: bool) -> (Game, u64) {
    let row = if trap { 9 } else { 8 };
    let mut level = Level::default();
    level.set_tile(6, row, '@').unwrap();
    level.set_tile(34, row, 'X').unwrap();
    for x in 0..spelunky_core::level::LEVEL_WIDTH {
        level.set_tile(x, row + 1, '1').unwrap();
    }
    if trap {
        level.set_tile(2, row, '>').unwrap();
    }
    if water {
        for x in 9..=11 {
            level.set_tile(x, 8, 'w').unwrap();
        }
    }
    let mut game = Game::shell(42, rules);
    game.load_custom(&level, "frog-bomb-own-step", "");
    if shop {
        let mut source_room_path = [[0; 4]; 5];
        source_room_path[1][0] = 4;
        game.world.source_room_path = Some(source_room_path);
        // Authored '!' means PATROL and independently attacks nearby players.
        // This initial source Create is IDLE; all later changes use real inputs.
        let keeper = game.spawn(
            "oShopkeeper",
            Vec2::new(160.0, if trap { 160.0 } else { 144.0 }),
        );
        assert_eq!(game.world.entity(keeper).unwrap().state, 0);
    }
    assert_eq!(
        game.player.pos,
        Vec2::new(120.0, if trap { 168.0 } else { 152.0 })
    );
    // Explicit source-default initial actor. The ordinary inputs below acquire,
    // carry and throw it; no authored FireFrog kill witness is claimed.
    let pos = game.player.pos;
    let bomb = game.spawn("oFireFrogBomb", pos);
    (game, bomb)
}

fn pickup(game: &mut Game, bomb: u64) {
    game.step(&InputFrame {
        down: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert_eq!(
        game.player.held,
        Some(bomb),
        "actual Down+Attack must pick up the source-created bomb"
    );
    assert!(game.world.entity(bomb).unwrap().held);
    assert_eq!(game.player.health, 4.0);
}

#[test]
fn ordinary_pickup_and_throw_preserve_the_unarmed_fire_frog_shop_control() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, bomb) = room(rules, true, false, false);
        let keeper = game
            .world
            .entities
            .iter()
            .find(|e| e.object == "oShopkeeper")
            .unwrap()
            .id;
        assert_eq!(game.world.entity(bomb).unwrap().value("armed"), 0.0);
        pickup(&mut game, bomb);
        game.step(&InputFrame::default());
        assert!(
            !game.flags.contains("wanted"),
            "fresh unarmed bomb must not warn shopkeepers while carried"
        );
        assert_eq!(game.data.get("thiefLevel").copied().unwrap_or(0.0), 0.0);
        assert_eq!(game.world.entity(bomb).unwrap().value("armed"), 0.0);
        game.step(&InputFrame {
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert_eq!(game.player.held, None);
        let actor = game.world.entity(bomb).unwrap();
        assert!(!actor.held);
        assert_eq!(
            actor.value("armed"),
            0.0,
            "source generic heavy-item throw does not set armed"
        );
        assert!(actor.alarms[1] > 0);
        assert!(!game.flags.contains("wanted"));
        assert_ne!(
            game.world.entity(keeper).unwrap().state,
            2,
            "peaceful authored keeper may start its source walking state"
        );
        assert_eq!(
            game.world.entity(keeper).unwrap().value("angered"),
            0.0,
            "ATTACK Step latches this on a later source visit"
        );
        assert_eq!(game.data.get("thiefLevel").copied().unwrap_or(0.0), 0.0);
        assert!(!game.events.iter().any(
            |event| matches!(event, GameEvent::Dialogue(message) if message.first == "TERRORIST!")
        ));
        assert!(!game.world.entities.iter().any(|e| e.object == "oExplosion"));
        assert_eq!(game.player.health, 4.0);
    }
}

fn on_water(game: &Game, pos: Vec2) -> bool {
    game.world
        .tiles_point_precision(pos, "oWaterSwim", CollisionPrecision::Precise)
        || game.world.entities.iter().any(|e| {
            e.alive
                && e.engine_active
                && catalog::is_a(&e.object, "oWaterSwim")
                && e.mask_contains(pos, CollisionPrecision::Precise)
        })
}

#[test]
fn ordinary_carry_enters_stays_exits_and_reenters_water_with_one_own_splash_per_entry() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, bomb) = room(rules, false, true, false);
        pickup(&mut game, bomb);
        let mut was_inside = false;
        let mut entries = 0;
        let mut saw_exit = false;
        let mut pause = 0;
        let mut stay_frames = 0;
        let mut own_births = 0;
        for tick in 0..105 {
            let input = if pause > 0 {
                pause -= 1;
                InputFrame::default()
            } else if saw_exit {
                InputFrame {
                    left: true,
                    ..Default::default()
                }
            } else {
                InputFrame {
                    right: true,
                    ..Default::default()
                }
            };
            let next_id = game.world.next_id;
            game.step(&input);
            let actor = game
                .world
                .entity(bomb)
                .expect("route must finish before the independent fuse expires");
            assert!(actor.held);
            assert_eq!(game.player.held, Some(bomb));
            assert_eq!(actor.value("armed"), 0.0);
            assert!(actor.alarms[1] > 0);
            let origin = actor.pos;
            let inside = on_water(&game, origin);
            assert_eq!(
                actor.value("swimming"),
                f64::from(inside),
                "own latch follows the carried heavy-item origin, tick={tick}"
            );
            let entered_now = inside && !was_inside;
            let newborns: Vec<_> = game
                .world
                .entities
                .iter()
                .filter(|e| e.id >= next_id && e.object == "oSplash" && e.pos == origin)
                .collect();
            // Player may independently splash at its own origin. Count the
            // bomb's source birth position, not the room's total Splash actors.
            assert_eq!(newborns.len(), usize::from(entered_now));
            let sounds = game.events.iter().filter(|event| matches!(event, GameEvent::Sound { name, pos } if name == "sndSplash" && *pos == origin)).count();
            assert_eq!(sounds, usize::from(entered_now));
            for splash in newborns {
                assert_eq!(splash.image_speed(), 0.6);
            }
            own_births += usize::from(entered_now);
            if entered_now {
                entries += 1;
                if entries == 1 {
                    pause = 3;
                }
            }
            if inside && was_inside {
                stay_frames += 1;
            }
            if entries == 1 && !inside && was_inside {
                assert!(
                    origin.x >= 208.0,
                    "must leave through the far pool edge before reversing"
                );
                saw_exit = true;
                assert_eq!(actor.value("swimming"), 0.0);
            }
            was_inside = inside;
            if entries == 2 {
                break;
            }
        }
        assert_eq!(
            entries, 2,
            "ordinary movement must establish both actual water entries"
        );
        assert!(saw_exit);
        assert!(stay_frames >= 3);
        assert_eq!(own_births, 2);
        assert_eq!(game.player.health, 4.0);
        assert!(!game.world.entities.iter().any(|e| e.object == "oExplosion"));
    }
}

#[test]
fn controlled_initial_fire_frog_death_creates_an_unarmed_bomb_without_changing_recover_rng() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = Game::shell(42, rules);
        game.world = World::new(30, 20, Area::Jungle, 5);
        game.player.pos = Vec2::new(240.0, 120.0);
        game.player.previous_pos = game.player.pos;
        for x in 0..30 {
            game.world.set_tile(x, 8, "oBlock");
        }
        let frog = game.spawn("oFireFrog", Vec2::new(96.0, 112.0));
        let actor = game.world.entity_mut(frog).unwrap();
        actor.health = 0;
        actor.state = 2; // Source RECOVER, initial-state continuation control.
        actor.vel = Vec2::default();
        let mut expected = game.rng.clone();
        expected.range(31); // Source RECOVER floor roll after child Destroy.
        game.step(&InputFrame::default());
        assert!(game.world.entity(frog).is_none());
        let bombs: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|e| e.alive && e.object == "oFireFrogBomb")
            .collect();
        assert_eq!(bombs.len(), 1);
        assert_eq!(
            bombs[0].value("armed"),
            0.0,
            "literal FireFrog death caller does not override armed"
        );
        assert_eq!(bombs[0].value("swimming"), 0.0);
        assert_eq!(
            bombs[0].alarms[1], 120,
            "newborn follows this frame's alarm pass"
        );
        assert_eq!(game.rng.clone().next_u64(), expected.next_u64());
    }
}

#[test]
fn ordinary_carry_and_trap_hit_arm_the_forced_drop_before_its_shop_warning() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, bomb) = room(rules, true, false, true);
        let keeper = game
            .world
            .entities
            .iter()
            .find(|e| e.object == "oShopkeeper")
            .unwrap()
            .id;
        pickup(&mut game, bomb);
        assert_eq!(game.world.entity(bomb).unwrap().value("armed"), 0.0);
        assert!(!game.flags.contains("wanted"));
        let mut saw_arrow = game
            .world
            .entities
            .iter()
            .any(|e| e.alive && e.object == "oArrow");
        let mut hit = false;
        for _ in 0..24 {
            // Move only until the detector fires, then stay in its original row.
            let input = InputFrame {
                right: !saw_arrow,
                ..Default::default()
            };
            game.step(&input);
            saw_arrow |= game
                .world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oArrow");
            if game.player.health < 4.0 {
                assert!(saw_arrow, "real trap birth must precede hurt");
                assert_eq!(game.player.health, 2.0, "first ordinary Arrow hit only");
                assert!(game.player.is_stunned());
                assert_eq!(game.player.held, None);
                let actor = game.world.entity(bomb).unwrap();
                assert!(!actor.held);
                assert_eq!(
                    actor.value("armed"),
                    1.0,
                    "late source hurt release sets armed"
                );
                assert!(actor.alarms[1] > 0);
                assert!(game.flags.contains("wanted"));
                assert_eq!(
                    game.world.entity(keeper).unwrap().health,
                    20,
                    "warning must not be confounded with an inherited thrown-item hit"
                );
                assert_eq!(game.world.entity(keeper).unwrap().state, 2);
                assert_eq!(game.world.entity(keeper).unwrap().value("angered"), 0.0);
                assert_eq!(game.data["thiefLevel"], 2.0);
                assert!(game.events.iter().any(|event| matches!(event, GameEvent::Dialogue(message) if message.first == "TERRORIST!" && message.ticks == 80)));
                assert!(!game.world.entities.iter().any(|e| e.object == "oExplosion"));
                hit = true;
                break;
            }
            assert_eq!(game.player.held, Some(bomb));
            assert_eq!(game.world.entity(bomb).unwrap().value("armed"), 0.0);
            assert!(!game.flags.contains("wanted"));
        }
        assert!(
            hit,
            "ordinary horizontal movement must trigger and receive the authored Arrow; runtime route remains a required precondition"
        );
    }
}
