use spelunky_core::{
    Area, Game, InputFrame, PlayerState, RulesPreset, Vec2, World,
    activation::{self, Regions},
};

fn room() -> Game {
    let mut game = Game::shell(5, RulesPreset::Classic);
    game.world = World::new(80, 40, Area::Mines, 1);
    game.world.name = "activation fixture".into();
    game.player.pos = Vec2::new(40.0, 152.0);
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    for x in 0..80 {
        game.world.set_tile(x, 10, "oBrick");
    }
    game
}

#[test]
fn offcamera_bomb_fuse_and_age_freeze_then_resume_through_game_step() {
    let mut game = room();
    let id = game.world.spawn("oBomb", Vec2::new(600.0, 152.0));
    let bomb = game.world.entity_mut(id).unwrap();
    bomb.data.insert("armed".into(), 1.0);
    bomb.timer = 5;
    bomb.age = 7;
    for _ in 0..8 {
        game.step(&InputFrame::default());
    }
    let bomb = game
        .world
        .entity(id)
        .expect("an inactive bomb must not explode");
    assert_eq!((bomb.timer, bomb.age), (5, 7));
    game.camera = Vec2::new(400.0, 0.0);
    game.player.pos = Vec2::new(440.0, 152.0);
    game.step(&InputFrame::default());
    let bomb = game.world.entity(id).unwrap();
    assert_eq!((bomb.timer, bomb.age), (4, 8));
}

#[test]
fn deactivated_instances_are_absent_from_queries_and_return_in_original_order() {
    let mut game = room();
    let first = game.world.spawn("oRock", Vec2::new(600.0, 80.0));
    let second = game.world.spawn("oRock", Vec2::new(80.0, 80.0));
    let third = game.world.spawn("oRock", Vec2::new(640.0, 80.0));
    let frozen = activation::begin(&mut game);
    assert!(game.world.entity(first).is_none());
    assert!(game.world.entity(third).is_none());
    assert!(game.world.tile(37, 10).unwrap().object.is_empty());
    assert_eq!(game.world.tile(5, 10).unwrap().object, "oBrick");
    // Active destruction and new spawns must survive restoration.
    game.world.set_tile(5, 10, "");
    let new = game.world.spawn("oRock", Vec2::new(90.0, 80.0));
    game.world.entity_mut(second).unwrap().alive = false;
    game.world.entities.retain(|entity| entity.alive);
    game.level_tick += 1;
    activation::finish(&mut game, frozen);
    assert_eq!(
        game.world.entities.iter().map(|e| e.id).collect::<Vec<_>>(),
        [first, third, new]
    );
    assert_eq!(game.world.tile(37, 10).unwrap().object, "oBrick");
    assert!(game.world.tile(5, 10).unwrap().object.is_empty());
}

#[test]
fn source_activation_exemptions_include_descendants() {
    let objects = [
        "oPlayer1",
        "oRope",
        "oRopeThrow",
        "oRopeTop",
        "oExplosion",
        "oGhost",
        "oBoulder",
        "oOlmec",
        "oGame",
        "oGlobals",
        "oScreen",
        "oGamepad",
        "oFinalBoss",
    ];
    for object in objects {
        let mut game = room();
        let id = game.world.spawn(object, Vec2::new(900.0, 80.0));
        let ordinary = game.world.spawn("oBomb", Vec2::new(600.0, 80.0));
        let frozen = activation::begin(&mut game);
        assert!(game.world.entity(id).is_some(), "{object}");
        assert!(game.world.entity(ordinary).is_none(), "{object}");
        game.level_tick += 1;
        activation::finish(&mut game, frozen);
    }
    // A Damsel derives from oItem and a keeper from oEnemy, not oCharacter.
    for object in ["oDamsel", "oShopkeeper"] {
        let mut game = room();
        let id = game.world.spawn(object, Vec2::new(900.0, 80.0));
        let frozen = activation::begin(&mut game);
        assert!(game.world.entity(id).is_none(), "{object}");
        game.level_tick += 1;
        activation::finish(&mut game, frozen);
    }
}

#[test]
fn literal_player_region_and_large_actor_regions_are_preserved() {
    let mut game = room();
    game.player.pos = Vec2::new(300.0, 100.0);
    let id = game.world.spawn("oBomb", Vec2::new(560.0, 100.0));
    assert!(
        Regions::for_game(&game).entity_active(game.world.entity(id).unwrap()),
        "source width is player.x+16, not 32"
    );
    game.player.pos.x = 40.0;
    assert!(!Regions::for_game(&game).entity_active(game.world.entity(id).unwrap()));
    game.world.spawn("oBoulder", Vec2::new(550.0, 100.0));
    assert!(Regions::for_game(&game).entity_active(game.world.entity(id).unwrap()));
}

#[test]
fn authored_hubs_do_not_have_a_level_activation_controller() {
    let mut game = room();
    game.flags.insert("hub".into());
    let id = game.world.spawn("oBomb", Vec2::new(600.0, 80.0));
    let frozen = activation::begin(&mut game);
    assert!(game.world.entity(id).is_some());
    assert_eq!(game.world.tile(37, 10).unwrap().object, "oBrick");
    game.level_tick += 1;
    activation::finish(&mut game, frozen);
}

#[test]
fn tutorial_and_custom_rooms_share_campaign_activation() {
    for flag in ["tutorial", "custom_level"] {
        let mut game = room();
        game.flags.insert(flag.into());
        let id = game.world.spawn("oBomb", Vec2::new(600.0, 80.0));
        let frozen = activation::begin(&mut game);
        assert!(game.world.entity(id).is_none(), "{flag}");
        game.level_tick += 1;
        activation::finish(&mut game, frozen);
    }
}

#[test]
fn room_transition_does_not_restore_old_entities_or_terrain() {
    let mut game = room();
    game.world.spawn("oBomb", Vec2::new(600.0, 80.0));
    let frozen = activation::begin(&mut game);
    game.world = World::new(42, 34, Area::Jungle, 5);
    game.level_tick = 0;
    activation::finish(&mut game, frozen);
    assert!(game.world.entities.is_empty());
    assert!(game.world.tiles.iter().all(|tile| tile.object.is_empty()));
}

#[test]
fn activation_uses_sprite_overlap_not_actor_origin_or_narrow_ai_viewport() {
    let mut game = room();
    let id = game.world.spawn("oSnake", Vec2::new(410.0, 80.0));
    assert!(Regions::for_game(&game).entity_active(game.world.entity(id).unwrap()));
    game.world.entity_mut(id).unwrap().pos.x = 420.0;
    assert!(!Regions::for_game(&game).entity_active(game.world.entity(id).unwrap()));
    game.world.entity_mut(id).unwrap().pos.x = 370.0;
    assert!(Regions::for_game(&game).entity_active(game.world.entity(id).unwrap()));
}

#[test]
fn hidden_inventory_bridge_requires_the_live_stash_reference_and_marker() {
    let mut game = room();
    let id = game
        .world
        .spawn("oShotgun", Vec2::new(-100_000.0, -100_000.0));
    game.world.entity_mut(id).unwrap().held = true;
    assert!(!Regions::for_game(&game).entity_active(game.world.entity(id).unwrap()));
    game.world
        .entity_mut(id)
        .unwrap()
        .data
        .insert("stashed".into(), 1.0);
    assert!(!Regions::for_game(&game).entity_active(game.world.entity(id).unwrap()));
    game.player.data.insert("stash_id".into(), id as f64);
    assert!(Regions::for_game(&game).entity_active(game.world.entity(id).unwrap()));
    game.world.entity_mut(id).unwrap().held = false;
    assert!(!Regions::for_game(&game).entity_active(game.world.entity(id).unwrap()));
}

#[test]
fn actual_inventory_cycle_idle_and_exit_preserve_the_hidden_weapon() {
    let mut game = room();
    game.player.pos = Vec2::new(85.0, 56.0);
    for x in 0..game.world.width {
        game.world.set_tile(x, 4, "oBrick");
    }
    game.world.spawn("oExit", Vec2::new(80.0, 48.0));
    let weapon = game.world.spawn("oShotgun", game.player.pos);
    game.world.entity_mut(weapon).unwrap().held = true;
    game.player.held = Some(weapon);
    game.step(&InputFrame {
        cycle_pressed: true,
        ..Default::default()
    });
    assert_eq!(game.player.data["stash_id"], weapon as f64);
    for _ in 0..10 {
        game.step(&InputFrame::default());
    }
    let stashed = game.world.entity(weapon).unwrap();
    assert!(stashed.held);
    assert_eq!(stashed.pos, Vec2::new(-100_000.0, -100_000.0));
    assert_eq!(stashed.vel, Vec2::default());
    game.step(&InputFrame {
        up: true,
        ..Default::default()
    });
    assert!(game.flags.contains("carry:oShotgun"));
    assert!(!game.player.data.contains_key("stash_id"));
    game.next_level();
    let weapon = game.world.entity(game.player.held.unwrap()).unwrap();
    assert_eq!(weapon.object, "oShotgun");
    assert_eq!(weapon.value("stashed"), 0.0);
}

#[test]
fn active_terrain_keeps_its_metadata_when_the_metadata_origin_is_offscreen() {
    let mut game = room();
    game.camera = Vec2::new(2.0, 0.0);
    game.player.pos = Vec2::new(130.0, 152.0);
    game.world.set_tile(26, 5, "oSolid");
    let metadata = game.world.spawn("oTerrainMetadata", Vec2::new(424.0, 88.0));
    let entity = game.world.entity_mut(metadata).unwrap();
    entity.sprite.clear();
    entity
        .text
        .insert("terrain_sprite".into(), "sIceBlock".into());
    let bullet = game.world.spawn("oBullet", Vec2::new(414.0, 81.0));
    game.world.entity_mut(bullet).unwrap().vel.x = 2.0;
    game.step(&InputFrame::default());
    assert!(
        game.world.entity(bullet).is_none(),
        "bullet must hit the still-active ice wall"
    );
    assert_eq!(
        game.world.entity(metadata).unwrap().text["terrain_sprite"],
        "sIceBlock"
    );
    assert!(game.world.tile_bounds(26, 5).is_valid());
}

#[test]
fn first_step_room_initialization_does_not_duplicate_a_frozen_exit_guard() {
    let mut game = room();
    game.player.pos = Vec2::new(80.0, 152.0);
    game.world.exit = Vec2::new(600.0, 152.0);
    game.flags.insert("murderer".into());
    let guard = game.world.spawn("oShopkeeper", Vec2::new(592.0, 144.0));
    game.world.entity_mut(guard).unwrap().state = 4;
    game.step(&InputFrame::default());
    assert_eq!(
        game.world
            .entities
            .iter()
            .filter(|e| e.object == "oShopkeeper")
            .count(),
        1
    );
    assert!(game.world.entity(guard).is_some());
}

#[test]
fn generated_terrain_masks_are_unchanged_by_atomic_partitioning() {
    for seed in [0, 33, 123] {
        let mut game = Game::new(seed, RulesPreset::Classic);
        let before: Vec<_> = (0..game.world.tiles.len())
            .map(|index| {
                game.world.tile_bounds(
                    (index % game.world.width) as i32,
                    (index / game.world.width) as i32,
                )
            })
            .collect();
        let frozen = activation::begin(&mut game);
        for (index, tile) in game.world.tiles.iter().enumerate() {
            if !tile.object.is_empty() {
                assert_eq!(
                    game.world.tile_bounds(
                        (index % game.world.width) as i32,
                        (index / game.world.width) as i32
                    ),
                    before[index],
                    "seed {seed}, tile {index}"
                );
            }
        }
        game.level_tick += 1;
        activation::finish(&mut game, frozen);
        for (index, bounds) in before.iter().enumerate() {
            assert_eq!(
                game.world.tile_bounds(
                    (index % game.world.width) as i32,
                    (index / game.world.width) as i32
                ),
                *bounds,
                "restored seed {seed}, tile {index}"
            );
        }
    }
}

#[test]
fn animated_terrain_activation_uses_the_current_override_frame() {
    let mut game = room();
    // Fractional camera avoids depending on disputed GM rectangle-edge
    // inclusion: frame1 overlaps the activation region by half a pixel,
    // whereas frame0 starts half a pixel beyond it.
    game.camera.y = 0.5;
    game.world.set_tile(0, 21, "oLava");
    let metadata = game.world.spawn("oTerrainMetadata", Vec2::new(8.0, 344.0));
    let entity = game.world.entity_mut(metadata).unwrap();
    entity.sprite.clear();
    entity.animation = 1.0;
    entity
        .text
        .insert("terrain_sprite".into(), "sLavaTop".into());
    let bounds = game.world.tile_bounds(0, 21);
    assert_eq!(bounds.y, 336.0);
    let frozen = activation::begin(&mut game);
    assert_eq!(game.world.tile(0, 21).unwrap().object, "oLava");
    assert!(game.world.entity(metadata).is_some());
    assert_eq!(game.world.tile_bounds(0, 21), bounds);
    game.level_tick += 1;
    activation::finish(&mut game, frozen);
}
