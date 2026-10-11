//! Literal own-Step controls; all actor-state setup precedes the first callback.
//! Callback RNG checks exclude Game.step's unrelated alarm/AI/animation phases.
use super::*;

fn fixture() -> Game {
    let mut game = Game::shell(42, RulesPreset::Classic);
    game.world = World::new(30, 20, Area::Jungle, 5);
    game.player.pos = Vec2::new(240.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game
}

fn splashes(game: &Game) -> Vec<&Entity> {
    game.world
        .entities
        .iter()
        .filter(|e| e.object == "oSplash")
        .collect()
}

fn splash_sounds(game: &Game) -> Vec<Vec2> {
    game.events
        .iter()
        .filter_map(|event| match event {
            GameEvent::Sound { name, pos } if name == "sndSplash" => Some(*pos),
            _ => None,
        })
        .collect()
}

#[test]
fn frog_bomb_fresh_create_is_unarmed_with_an_independent_running_fuse() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = fixture();
        game.rules = rules;
        let mut expected = game.rng.clone();
        let id = game.spawn("oFireFrogBomb", Vec2::new(88.0, 88.0));
        let bomb = game.world.entity(id).unwrap();
        assert_eq!(bomb.value("armed"), 0.0);
        assert_eq!(bomb.value("swimming"), 0.0);
        assert_eq!(bomb.value("heavy"), 1.0);
        assert_eq!(bomb.alarms[1], 120);
        assert_eq!(bomb.timer, 120);
        assert_eq!(
            game.rng.clone().next_u64(),
            expected.next_u64(),
            "Create has no random calls"
        );
        game.step(&InputFrame::default());
        let bomb = game.world.entity(id).unwrap();
        assert_eq!(bomb.value("armed"), 0.0);
        assert_eq!(bomb.alarms[1], 119, "Alarm advances independently of armed");
        assert!(!game.world.entities.iter().any(|e| e.object == "oExplosion"));
    }
}

#[test]
fn frog_bomb_water_point_uses_active_family_raw_edges_and_a_per_entry_latch() {
    let mut cases = 0;
    for terrain in [false, true] {
        for (object, active, x, inside) in [
            ("oWaterSwim", true, 79.75, false),
            ("oWaterSwim", true, 80.0, true),
            ("oWaterSwim", true, 88.25, true),
            ("oWaterSwim", true, 95.0, true),
            ("oWaterSwim", true, 96.0, false),
            ("oWaterSwim", false, 88.25, false),
            ("oWater", true, 88.25, false),
        ] {
            for already_swimming in [false, true] {
                for ducking in [false, true] {
                    for facing in [-1, 1] {
                        let mut game = fixture();
                        if terrain {
                            game.world.set_tile(5, 5, object);
                            game.world.tiles[5 * game.world.width + 5].engine_active = active;
                        } else {
                            let water = game.spawn(object, Vec2::new(80.0, 80.0));
                            game.world.entity_mut(water).unwrap().engine_active = active;
                        }
                        let origin = Vec2::new(x, 88.0);
                        game.player.pos = Vec2::new(
                            x - 4.0 * f64::from(facing),
                            88.0 + if ducking { 2.0 } else { 4.0 },
                        );
                        game.player.previous_pos = game.player.pos;
                        game.player.facing = facing;
                        game.player.state = if ducking {
                            PlayerState::Ducking
                        } else {
                            PlayerState::Standing
                        };
                        let id = game.spawn("oFireFrogBomb", origin);
                        let bomb = game.world.entity_mut(id).unwrap();
                        bomb.held = true;
                        bomb.data
                            .insert("swimming".into(), f64::from(already_swimming));
                        game.player.held = Some(id);
                        let mut expected = game.rng.clone();
                        crate::items::step_instance(&mut game, id);
                        let bomb = game.world.entity(id).unwrap();
                        assert_eq!(bomb.pos, origin, "source inherited heavy Item alignment");
                        assert_eq!(
                            bomb.value("swimming"),
                            f64::from(inside),
                            "terrain={terrain}, object={object}, active={active}, x={x}"
                        );
                        let births = usize::from(inside && !already_swimming);
                        assert_eq!(splashes(&game).len(), births);
                        assert_eq!(splash_sounds(&game), vec![origin; births]);
                        for splash in splashes(&game) {
                            assert_eq!(splash.pos, origin);
                            assert_eq!(splash.sprite, "sSplash");
                            assert_eq!(splash.image_speed(), 0.6);
                            assert_eq!(splash.vel, Vec2::default());
                            assert!(!crate::catalog::is_a(&splash.object, "oDetritus"));
                        }
                        assert_eq!(
                            game.rng.clone().next_u64(),
                            expected.next_u64(),
                            "Splash Create uses zero RNG and no particle-cap script"
                        );
                        let mut expected = game.rng.clone();
                        crate::items::step_instance(&mut game, id);
                        assert_eq!(
                            splashes(&game).len(),
                            births,
                            "remaining inside is not another entry"
                        );
                        assert_eq!(splash_sounds(&game).len(), births);
                        assert_eq!(game.rng.clone().next_u64(), expected.next_u64());
                        cases += 1;
                    }
                }
            }
        }
    }
    assert_eq!(cases, 112);
}

#[test]
fn frog_bomb_own_water_tail_uses_post_item_motion_and_continues_after_lava_destroy() {
    for lava in [false, true] {
        let mut game = fixture();
        game.world.set_tile(5, 5, "oWaterSwim");
        if lava {
            game.spawn("oLava", Vec2::new(80.0, 80.0));
        }
        let start = if lava {
            Vec2::new(88.0, 88.0)
        } else {
            Vec2::new(76.0, 88.0)
        };
        let id = game.spawn("oFireFrogBomb", start);
        if !lava {
            game.world.entity_mut(id).unwrap().vel.x = 8.0;
        }
        let mut expected = game.rng.clone();
        crate::items::step_instance(&mut game, id);
        let bomb = game.world.entities.iter().find(|e| e.id == id).unwrap();
        assert_eq!(bomb.alive, !lava);
        let pos = if lava {
            Vec2::new(88.0, 88.05)
        } else {
            Vec2::new(84.0, 88.0)
        };
        assert!((bomb.pos.x - pos.x).abs() < 1e-9 && (bomb.pos.y - pos.y).abs() < 1e-9);
        assert_eq!(bomb.value("swimming"), 1.0);
        assert_eq!(
            splashes(&game).len(),
            1,
            "child event still executes after inherited selfDestroy"
        );
        assert_eq!(splashes(&game)[0].pos, bomb.pos);
        assert_eq!(splash_sounds(&game), vec![bomb.pos]);
        assert!(
            !game.world.entities.iter().any(|e| e.object == "oExplosion"),
            "inherited lava explosion belongs to type Bomb only"
        );
        assert_eq!(game.rng.clone().next_u64(), expected.next_u64());
    }
}

#[test]
fn frog_bomb_armed_shop_guard_is_strict_bbox_distance_with_source_status_controls() {
    let mut cases = 0;
    for object in ["oShopkeeper", "oShopkeeper2"] {
        for (armed, active, alive, dead, angered, keeper_x, should_anger) in [
            (false, true, true, false, false, 124.0, false),
            (true, true, true, false, false, 124.0, true), // bbox gap95
            (true, true, true, false, false, 125.0, false), // bbox gap96
            (true, false, true, false, false, 124.0, false),
            (true, true, false, false, false, 124.0, false),
            (true, true, true, true, false, 124.0, false),
            (true, true, true, false, true, 124.0, false),
            (true, true, true, false, false, 176.0, false), // other source room
        ] {
            let mut game = fixture();
            let keeper = game.spawn(object, Vec2::new(keeper_x, 80.0));
            let actor = game.world.entity_mut(keeper).unwrap();
            actor.engine_active = active;
            actor.alive = alive;
            actor.state = 4;
            actor.data.insert("dead".into(), f64::from(dead));
            actor.data.insert("angered".into(), f64::from(angered));
            let bomb_x = if keeper_x == 176.0 { 160.0 } else { 24.0 };
            let id = game.spawn("oFireFrogBomb", Vec2::new(bomb_x, 88.0));
            assert_eq!(
                game.world
                    .entity(id)
                    .unwrap()
                    .collision_mask()
                    .point_bounds(),
                Rect::new(bomb_x - 8.0, 84.0, 16.0, 12.0)
            );
            assert_eq!(
                game.world
                    .entities
                    .iter()
                    .find(|e| e.id == keeper)
                    .unwrap()
                    .collision_mask()
                    .point_bounds(),
                Rect::new(keeper_x + 2.0, 81.0, 12.0, 15.0)
            );
            game.world
                .entity_mut(id)
                .unwrap()
                .data
                .insert("armed".into(), f64::from(armed));
            let mut expected = game.rng.clone();
            crate::items::step_instance(&mut game, id);
            let actor = game.world.entities.iter().find(|e| e.id == keeper).unwrap();
            assert_eq!(actor.state, if should_anger { 2 } else { 4 });
            assert_eq!(
                actor.value("angered"),
                f64::from(angered),
                "anger script does not latch flag"
            );
            assert_eq!(game.flags.contains("wanted"), should_anger);
            assert_eq!(
                game.data.get("thiefLevel").copied().unwrap_or(0.0),
                if should_anger { 2.0 } else { 0.0 }
            );
            if should_anger {
                assert!(game.events.iter().any(
                    |e| matches!(e, GameEvent::Dialogue(message) if message.first == "TERRORIST!")
                ));
            }
            assert_eq!(game.rng.clone().next_u64(), expected.next_u64());
            cases += 1;
        }
    }
    assert_eq!(cases, 16);
}

#[test]
fn frog_bomb_shop_with_loop_uses_every_active_keeper_origin_and_preserves_ties() {
    for coincident in [false, true] {
        let mut game = fixture();
        let first = game.spawn("oShopkeeper", Vec2::new(80.0, 80.0));
        let second_pos = if coincident {
            Vec2::new(80.0, 80.0)
        } else {
            Vec2::new(240.0, 80.0)
        };
        let second = game.spawn("oShopkeeper2", second_pos);
        let inactive = game.spawn("oShopkeeper", Vec2::new(96.0, 80.0));
        game.world.entity_mut(inactive).unwrap().engine_active = false;
        for id in [first, second, inactive] {
            game.world.entity_mut(id).unwrap().state = 4;
        }
        let bomb = game.spawn("oFireFrogBomb", Vec2::new(64.0, 88.0));
        game.world
            .entity_mut(bomb)
            .unwrap()
            .data
            .insert("armed".into(), 1.0);
        let mut expected = game.rng.clone();
        crate::items::step_instance(&mut game, bomb);
        assert_eq!(game.world.entity(first).unwrap().state, 2);
        assert_eq!(
            game.world.entity(second).unwrap().state,
            if coincident { 4 } else { 2 }
        );
        assert_eq!(game.world.entity(inactive).unwrap().state, 4);
        assert_eq!(
            game.data["thiefLevel"], 5.0,
            "source with-loop executes twice, including coincident nearest ties"
        );
        assert_eq!(game.world.entity(first).unwrap().value("angered"), 0.0);
        assert_eq!(game.world.entity(second).unwrap().value("angered"), 0.0);
        assert_eq!(game.rng.clone().next_u64(), expected.next_u64());
    }
}

#[test]
fn frog_bomb_splash_has_no_detritus_cap_or_create_rng() {
    let mut game = fixture();
    game.world.set_tile(5, 5, "oWaterSwim");
    for _ in 0..32 {
        game.spawn("oBone", Vec2::new(240.0, 80.0));
    }
    assert_eq!(
        game.world
            .entities
            .iter()
            .filter(|e| e.alive && e.engine_active && crate::catalog::is_a(&e.object, "oDetritus"))
            .count(),
        32
    );
    let bomb = game.spawn("oFireFrogBomb", Vec2::new(88.0, 88.0));
    let mut expected = game.rng.clone();
    crate::items::step_instance(&mut game, bomb);
    assert_eq!(splashes(&game).len(), 1);
    assert!(splashes(&game)[0].alive);
    assert_eq!(splashes(&game)[0].image_speed(), 0.6);
    assert_eq!(game.world.entity(bomb).unwrap().value("swimming"), 1.0);
    assert_eq!(game.rng.clone().next_u64(), expected.next_u64());
}

#[test]
fn frog_bomb_own_splash_follows_inherited_creature_hit_and_its_full_blood_create() {
    let mut game = fixture();
    game.world.set_tile(5, 5, "oWaterSwim");
    let snake = game.spawn("oSnake", Vec2::new(80.0, 80.0));
    let bomb = game.spawn("oFireFrogBomb", Vec2::new(76.0, 88.0));
    game.world.entity_mut(bomb).unwrap().vel.x = 8.0;
    let mut expected = game.rng.clone();
    // Literal source Blood Create, not a call to the tested particle helper.
    let velocity = Vec2::new(
        expected.unit() * 4.0 - expected.unit() * 4.0,
        -1.0 - expected.unit() * 2.0,
    );
    let gravity = f64::from(1 + expected.range(6)) * 0.1;
    crate::items::step_instance(&mut game, bomb);
    assert_eq!(game.world.entity(snake).unwrap().health, 0);
    let drops: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.object == "oBlood")
        .collect();
    assert_eq!(drops.len(), 1);
    assert_eq!(drops[0].pos, Vec2::new(88.0, 88.0));
    assert_eq!(drops[0].vel, velocity);
    assert_eq!(drops[0].value("gravity"), gravity);
    assert_eq!(splashes(&game).len(), 1);
    assert_eq!(splashes(&game)[0].pos, Vec2::new(84.0, 88.0));
    assert!(
        drops[0].creation_serial < splashes(&game)[0].creation_serial,
        "complete inherited Item callback precedes child Splash Create"
    );
    let hit = game
        .events
        .iter()
        .position(|event| matches!(event, GameEvent::Sound { name, .. } if name == "sndHit"))
        .unwrap();
    let splash = game
        .events
        .iter()
        .position(|event| matches!(event, GameEvent::Sound { name, .. } if name == "sndSplash"))
        .unwrap();
    assert!(hit < splash);
    assert_eq!(game.rng.clone().next_u64(), expected.next_u64());
}
