//! Regressions for source event boundaries, exercised through the public frame.
//!
//! Expectations combine preserved object events with the canonical executable's
//! resource-ordered Step, motion-after-Step, and draw-before-animation loops.
//! These are source-derived fixtures, not an original-executable runtime replay.
use spelunky_core::*;

fn flat_room() -> Game {
    let mut game = Game::shell(123, RulesPreset::Classic);
    game.world = World::new(40, 30, Area::Mines, 1);
    for x in 0..40 {
        game.world.set_tile(x, 10, "oBlock");
    }
    game.player.pos = Vec2::new(160.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.grounded = true;
    game.player.state = PlayerState::Standing;
    game
}

fn step(game: &mut Game) {
    game.step(&InputFrame::default());
}

fn died_events(game: &Game) -> usize {
    game.events
        .iter()
        .filter(|e| matches!(e, GameEvent::Died))
        .count()
}

#[test]
fn fatal_automatic_collision_waits_for_next_player_step_to_latch_death() {
    let mut game = flat_room();
    game.player.pos.x = 88.0;
    game.world.spawn("oBullet", Vec2::new(88.0, 149.0));

    // Bullet's automatic Collision follows Game318 and Player319 Normal Step.
    step(&mut game);
    assert_eq!(game.player.health, 0.0);
    assert_eq!(game.state, RunState::Playing);
    assert_ne!(game.player.state, PlayerState::Dead);
    assert_eq!(died_events(&game), 0);
    assert_eq!(game.data.get("time"), Some(&30.0));

    // Game318 still sees the old independent dead flag and counts this frame.
    step(&mut game);
    assert_eq!(game.state, RunState::Dead);
    assert_eq!(game.player.state, PlayerState::Dead);
    assert_eq!(died_events(&game), 1);
    assert_eq!(game.data.get("time"), Some(&60.0));
    assert_eq!(game.data.get("death_ticks"), Some(&0.0));

    step(&mut game);
    assert_eq!(game.data.get("time"), Some(&60.0));
    assert_eq!(game.data.get("death_ticks"), Some(&1.0));
    assert_eq!(died_events(&game), 0);
}

#[test]
fn fatal_collision_defers_ankh_consumption_and_teleport_to_next_player_step() {
    let mut game = flat_room();
    game.player.pos.x = 88.0;
    game.world.entrance = Vec2::new(40.0, 152.0);
    game.player.equipment.insert("Ankh".into());
    game.world.spawn("oBullet", Vec2::new(88.0, 149.0));

    step(&mut game);
    assert_eq!(game.player.health, 0.0);
    assert_eq!(game.player.pos.x, 88.0);
    assert!(game.player.has("Ankh"));
    assert_eq!(game.state, RunState::Playing);
    assert_eq!(died_events(&game), 0);

    step(&mut game);
    assert_eq!(game.player.health, 4.0);
    assert_eq!(game.player.pos.x, 40.0);
    assert!(!game.player.has("Ankh"));
    assert_eq!(game.state, RunState::Playing);
    assert_eq!(died_events(&game), 0);
    assert_eq!(game.data.get("time"), Some(&60.0));
}

#[test]
fn keeper_newborn_bullets_run_their_higher_resource_step_in_birth_frame() {
    let mut game = flat_room();
    let id = game.world.spawn("oShopkeeper", Vec2::new(80.0, 144.0));
    let keeper = game.world.entity_mut(id).unwrap();
    keeper.state = 2;
    keeper.facing = 1;
    keeper.data.insert("turn_timer".into(), 20.0);

    step(&mut game);
    let bullets: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oBullet")
        .collect();
    assert_eq!(bullets.len(), 6);
    for bullet in bullets {
        // oShopkeeper666 creates at x+12; oBullet669 explicitly adds xVel.
        assert!((9.0..=11.0).contains(&bullet.vel.x));
        assert_eq!(bullet.pos.x, 80.0 + 12.0 + bullet.vel.x);
    }
}

#[test]
fn newborn_enemy_sight_moves_in_builtin_motion_after_all_normal_steps() {
    let mut game = flat_room();
    let id = game.world.spawn("oCaveman", Vec2::new(80.0, 144.0));
    let caveman = game.world.entity_mut(id).unwrap();
    caveman.state = 0;
    caveman.facing = 1;
    caveman.vel = Vec2::default();
    caveman.timer = 100;

    step(&mut game);
    let sightings: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oEnemySight")
        .collect();
    assert_eq!(sightings.len(), 1);
    // Sight has no movement Step: its source speed becomes built-in hspeed.
    assert_eq!(sightings[0].pos.x, 90.0);
    assert_eq!(sightings[0].value("hspeed"), 10.0);
}

#[test]
fn damsel_kiss_observes_frame_seven_before_final_animation_advance() {
    let mut game = flat_room();
    let id = game.world.spawn("oDamsel", Vec2::new(80.0, 152.0));
    let damsel = game.world.entity_mut(id).unwrap();
    damsel.state = 6;
    damsel.sprite = "sDamselKissL".into();
    damsel.animation = 7.0;

    step(&mut game);
    assert_eq!(game.world.entity(id).unwrap().animation, 7.5);
    assert_eq!(
        game.world
            .entities
            .iter()
            .filter(|e| e.alive && e.object == "oHeart")
            .count(),
        1
    );
    assert_eq!(
        game.events
            .iter()
            .filter(|e| matches!(e, GameEvent::Sound { name, .. } if name == "sndKiss"))
            .count(),
        1
    );
}

#[test]
fn cross_resource_creation_order_does_not_change_step_rng_assignment() {
    fn run(reverse: bool) -> (Vec<(String, i8, i32, Vec2)>, u64) {
        let mut game = flat_room();
        let objects = if reverse {
            ["oSnake", "oCaveman"]
        } else {
            ["oCaveman", "oSnake"]
        };
        for object in objects {
            let x = if object == "oCaveman" { 32.0 } else { 96.0 };
            let id = game.world.spawn(object, Vec2::new(x, 144.0));
            let enemy = game.world.entity_mut(id).unwrap();
            enemy.state = 0;
            enemy.timer = 0;
            enemy.vel = Vec2::default();
        }
        step(&mut game);
        let states = ["oCaveman", "oSnake"]
            .into_iter()
            .map(|object| {
                let enemy = game
                    .world
                    .entities
                    .iter()
                    .find(|e| e.object == object)
                    .unwrap();
                (object.to_owned(), enemy.facing, enemy.state, enemy.pos)
            })
            .collect();
        (states, game.rng.next_u64())
    }

    // Caveman325 consumes its Step randomness before Snake552 in both rooms.
    assert_eq!(run(false), run(true));
}

#[test]
fn animation_callbacks_follow_depth_sorted_draw_order_and_exclude_newborns() {
    let mut game = flat_room();
    // Birth order deliberately opposes draw order. FakeBones657 depth900 must
    // animate before WebBall750 depth1, although the web was created first.
    let web_ball = game.world.spawn("oWebBall", Vec2::new(64.0, 112.0));
    let web_ball = game.world.entity_mut(web_ball).unwrap();
    web_ball.sprite = "sWebCreate".into();
    web_ball.animation = (catalog::sprite_frame_count("sWebCreate") - 1) as f64;
    web_ball.set_image_speed(1.0);
    web_ball.vel = Vec2::default();
    web_ball.data.insert("life".into(), 0.0);
    web_ball.data.insert("web_create_initialized".into(), 1.0);

    let bones = game.world.spawn("oFakeBones", Vec2::new(112.0, 144.0));
    let bones = game.world.entity_mut(bones).unwrap();
    bones.sprite = "sSkeletonCreateL".into();
    bones.animation = (catalog::sprite_frame_count("sSkeletonCreateL") - 1) as f64;
    bones.set_image_speed(1.0);

    step(&mut game);
    let skeletons: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oSkeleton")
        .collect();
    let webs: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oWeb")
        .collect();
    assert_eq!(skeletons.len(), 1);
    assert_eq!(webs.len(), 1);
    assert!(
        skeletons[0].id < webs[0].id,
        "depth900 FakeBones must finish before depth1 WebBall"
    );
    assert_eq!(skeletons[0].animation, 0.0);
    assert_eq!(webs[0].animation, 0.0);
}

#[test]
fn camera_activation_includes_its_exact_right_edge_but_excludes_next_pixel() {
    for (web_x, expected_active) in [(416.0, true), (417.0, false)] {
        let mut game = Game::shell(7, RulesPreset::Classic);
        game.player.pos = Vec2::new(40.0, 72.0);
        game.player.previous_pos = game.player.pos;
        game.player.state = PlayerState::Standing;
        game.player.grounded = true;
        for x in 0..game.world.width {
            game.world.set_tile(x, 5, "oBlock");
        }
        let id = game.world.spawn("oWeb", Vec2::new(web_x, 64.0));
        assert_eq!(game.world.entity(id).unwrap().sprite_bounds().x, web_x);

        // Level553 uses camera0 + viewport320 + margin96 = inclusive x416.
        step(&mut game);
        assert_eq!(
            game.world.entity(id).unwrap().engine_active,
            expected_active,
            "web mask starts at x{web_x}"
        );
    }
}

#[test]
fn replacing_registered_terrain_runs_new_create_and_replaces_resource_membership() {
    use dispatcher::Instance;

    let mut game = flat_room();
    let (x, y) = (6, 9);
    let index = y * game.world.width + x;
    let terrain = Instance::Terrain(index);
    let arrow_resource = catalog::object("oArrowTrapLeft").unwrap().id;
    let spear_resource = catalog::object("oSpearTrapTop").unwrap().id;
    game.world.set_tile(x, y, "oArrowTrapLeft");
    step(&mut game);
    assert!(game.dispatcher.resource_order[&arrow_resource].contains(&terrain));
    assert_eq!(
        game.world.tiles[index].alarms[1], 0,
        "ordinary ArrowTrap Create schedules Alarm1 for its first frame"
    );
    assert!(
        game.world
            .entities
            .iter()
            .any(|e| e.alive && e.object == "oArrowTrapTest")
    );
    assert!(
        !game
            .world
            .entities
            .iter()
            .any(|e| e.alive && e.object == "oSpearsLeft")
    );

    // A replacement is a fresh source instance even though its grid handle is
    // reused. SpearTrap detects the player on its right and extends immediately.
    game.world.set_tile(x, y, "oSpearTrapTop");
    step(&mut game);
    assert_eq!(game.world.tiles[index].object, "oSpearTrapTop");
    assert_eq!(game.world.tiles[index].alarms, [-1; 12]);
    assert!(
        !game
            .dispatcher
            .resource_order
            .get(&arrow_resource)
            .is_some_and(|instances| instances.contains(&terrain))
    );
    assert_eq!(
        game.dispatcher.resource_order[&spear_resource]
            .iter()
            .filter(|instance| **instance == terrain)
            .count(),
        1
    );
    let spears: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oSpearsLeft")
        .collect();
    assert_eq!(spears.len(), 1);
    assert_eq!(spears[0].pos, Vec2::new(112.0, 144.0));
    assert_eq!(spears[0].sprite, "sSpearsRight");

    // Returning to the first resource must run its Create/Alarm again rather
    // than resurrecting the first trap's exhausted Alarm1.
    let old_detector_ids: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.object == "oArrowTrapTest")
        .map(|e| e.id)
        .collect();
    game.world.set_tile(x, y, "oArrowTrapLeft");
    step(&mut game);
    assert_eq!(game.world.tiles[index].alarms[1], 0);
    assert!(
        game.world
            .entities
            .iter()
            .any(|e| e.alive && e.object == "oArrowTrapTest" && !old_detector_ids.contains(&e.id))
    );
    assert!(game.dispatcher.resource_order[&arrow_resource].contains(&terrain));
    assert!(
        !game
            .dispatcher
            .resource_order
            .get(&spear_resource)
            .is_some_and(|instances| instances.contains(&terrain))
    );
}

#[test]
fn damsel_collision_uses_previous_draw_scale_before_current_draw_updates_it() {
    // This asymmetric pair overlaps only with the stored +1 mirror. A current
    // facing-derived -1 collision mask would miss the source WebBall event.
    let mut old_draw = Entity::new(1, "oDamsel", Vec2::new(80.0, 72.0));
    old_draw.facing = 1;
    old_draw.data.insert("image_xscale".into(), 1.0);
    let mut current_facing = old_draw.clone();
    current_facing.data.insert("image_xscale".into(), -1.0);
    let ball_mask = Entity::new(2, "oWebBall", Vec2::new(73.0, 61.0));
    assert!(old_draw.event_intersects(&ball_mask));
    assert!(!current_facing.event_intersects(&ball_mask));

    let mut game = Game::shell(7, RulesPreset::Classic);
    game.world = World::new(80, 40, Area::Mines, 1);
    game.world.name = "draw mirror fixture".into();
    game.player.pos = Vec2::new(240.0, 72.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    for x in 0..game.world.width {
        game.world.set_tile(x, 5, "oBrick");
    }
    let damsel_id = game.spawn("oDamsel", Vec2::new(80.0, 72.0));
    let damsel = game.world.entity_mut(damsel_id).unwrap();
    damsel.state = 5; // SLAVE turns toward the player during Normal Step.
    damsel.facing = -1;
    damsel.data.insert("image_xscale".into(), 1.0);
    let ball_id = game.spawn("oWebBall", Vec2::new(73.0, 61.0));
    let ball = game.world.entity_mut(ball_id).unwrap();
    ball.vel = Vec2::default();
    ball.data.insert("life".into(), 99.0);

    step(&mut game);
    let damsel = game.world.entity(damsel_id).unwrap();
    let ball = game.world.entity(ball_id).unwrap();
    assert_eq!(damsel.facing, 1);
    assert_eq!(
        ball.state, 1,
        "Collision must observe the +1 scale stored by the preceding Draw"
    );
    assert_eq!(ball.sprite, "sWebCreate");
    assert_eq!(
        damsel.value("image_xscale"),
        -1.0,
        "current Draw applies the new facing only after automatic Collision"
    );
}

#[test]
fn nonpositive_life_blocks_fresh_bomb_and_rope_before_death_or_ankh_revival() {
    for life in [0, -99] {
        for ankh in [false, true] {
            for bomb in [false, true] {
                let mut game = flat_room();
                game.player.health = f64::from(life);
                if ankh {
                    game.player.equipment.insert("Ankh".into());
                }
                let resources = (game.player.bombs, game.player.ropes);
                game.step(&InputFrame {
                    bomb_pressed: bomb,
                    rope_pressed: !bomb,
                    ..Default::default()
                });
                assert_eq!(
                    (game.player.bombs, game.player.ropes),
                    resources,
                    "life={life}/ankh={ankh}/bomb={bomb}"
                );
                assert!(!game.world.entities.iter().any(|entity| entity.alive
                    && matches!(entity.object.as_str(), "oBomb" | "oRopeThrow")));
                assert!(!game.events.iter().any(
                    |event| matches!(event, GameEvent::Sound {name,..} if name == "sndThrow")
                ));
                assert_eq!(
                    game.state,
                    if ankh {
                        RunState::Playing
                    } else {
                        RunState::Dead
                    }
                );
                assert_eq!(game.player.health, if ankh { 4.0 } else { 0.0 });
                assert!(!game.player.has("Ankh"));
                assert_eq!(died_events(&game), usize::from(!ankh));
                assert_eq!(game.events.iter().filter(|event| matches!(event, GameEvent::Sound {name,..} if name == "sndTeleport")).count(), usize::from(ankh));
            }
        }
    }
}

#[test]
fn positive_life_keeps_fresh_bomb_and_rope_actions_and_their_source_births() {
    for bomb in [false, true] {
        let mut game = flat_room();
        let resources = (game.player.bombs, game.player.ropes);
        game.step(&InputFrame {
            bomb_pressed: bomb,
            rope_pressed: !bomb,
            ..Default::default()
        });
        assert_eq!(game.player.bombs, resources.0 - f64::from(bomb));
        assert_eq!(game.player.ropes, resources.1 - f64::from(!bomb));
        assert!(game.world.entities.iter().any(
            |entity| entity.alive && entity.object == if bomb { "oBomb" } else { "oRopeThrow" }
        ));
        assert!(
            game.events
                .iter()
                .any(|event| matches!(event, GameEvent::Sound {name,..} if name == "sndThrow"))
        );
        assert_eq!(game.state, RunState::Playing);
        assert_eq!(game.player.health, 4.0);
    }
}
