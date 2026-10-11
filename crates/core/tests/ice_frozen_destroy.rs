//! Public Mattock Animation End callback witnesses. Initial terrain/instance
//! state is explicit; assertions inspect the complete synchronous callback.
//! The King continuation suite separately covers actual Game.step ordering.
use spelunky_core::*;

#[derive(Clone, Copy, Debug)]
enum Storage {
    Tile,
    Entity,
}

fn room(rules: RulesPreset) -> Game {
    let mut game = Game::shell(7, rules);
    game.world = World::new(40, 20, Area::Ice, 9);
    game.player.pos = Vec2::new(40.0, 40.0);
    // The source still draws the break roll; this prevents extra break debris.
    game.flags.insert("character_tunnelman".into());
    game
}

fn ice(game: &mut Game, storage: Storage, object: &str) {
    match storage {
        Storage::Tile => game.world.set_tile(5, 5, object),
        Storage::Entity => {
            game.spawn(object, Vec2::new(80.0, 80.0));
        }
    }
}

fn dig(game: &mut Game) {
    let hit = game.spawn("oMattockHit", Vec2::new(88.0, 88.0));
    items::weapons::animation_end(game, hit);
    assert!(game.world.entity(hit).is_none());
}

fn count(game: &Game, object: &str) -> usize {
    game.world
        .entities
        .iter()
        .filter(|entity| entity.alive && entity.object == object)
        .count()
}

fn cave(game: &Game, pos: Vec2) {
    let actors: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|entity| entity.alive && entity.object == "oCaveman")
        .collect();
    assert_eq!(actors.len(), 1);
    assert_eq!(actors[0].pos, pos);
    assert_eq!(actors[0].state, 98);
    assert_eq!(actors[0].timer, 200);
    assert_eq!(actors[0].value("invincible"), 20.0);
}

#[test]
fn public_mattock_ice_destroy_releases_frozen_synchronously_after_three_drips_once() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for storage in [Storage::Tile, Storage::Entity] {
            let mut game = room(rules);
            ice(&mut game, storage, "oIce");
            let frozen = game.spawn("oFrozenCaveman", Vec2::new(80.0, 80.0));
            // FrozenCaveman Destroy ignores its local cleanDeath flag.
            game.world
                .entity_mut(frozen)
                .unwrap()
                .data
                .insert("cleanDeath".into(), 1.0);
            let serial = game.world.creation_serial;
            let mut expected = game.rng.clone();
            let mut drips = Vec::new();
            for _ in 0..3 {
                drips.push(Vec2::new(
                    80.0 + f64::from(expected.range(17)),
                    80.0 + f64::from(expected.range(17)),
                ));
            }
            expected.range(20); // Mattock's source break test.

            dig(&mut game);

            assert!(game.world.entity(frozen).is_none(), "{storage:?}");
            cave(&game, Vec2::new(80.0, 80.0));
            let mut births: Vec<_> = game
                .world
                .entities
                .iter()
                .filter(|e| {
                    e.creation_serial > serial && matches!(e.object.as_str(), "oDrip" | "oCaveman")
                })
                .collect();
            births.sort_by_key(|e| e.creation_serial);
            assert_eq!(births.len(), 4);
            for (actual, pos) in births[..3].iter().zip(drips) {
                assert_eq!(actual.object, "oDrip");
                assert_eq!(actual.pos, pos);
            }
            assert_eq!(births[3].object, "oCaveman");
            assert_eq!(game.rng.clone().next_u64(), expected.next_u64());
            world_events::tick(&mut game, &InputFrame::default());
            world_events::tick(&mut game, &InputFrame::default());
            assert_eq!(count(&game, "oCaveman"), 1);
        }
    }
}

#[test]
fn ice_point_gate_and_translated_pair_query_can_select_different_single_targets() {
    let mut game = room(RulesPreset::Classic);
    ice(&mut game, Storage::Tile, "oIce");
    let selected = game.spawn("oFrozenCaveman", Vec2::new(96.0, 80.0));
    let gate = game.spawn("oFrozenCaveman", Vec2::new(80.0, 80.0));
    let mut translated = Entity::new(0, "oIce", Vec2::new(88.0, 88.0));
    assert!(
        !game
            .world
            .entity(selected)
            .unwrap()
            .mask_contains(translated.pos, CollisionPrecision::BoundingBox)
    );
    assert!(
        game.world
            .entity(gate)
            .unwrap()
            .mask_contains(translated.pos, CollisionPrecision::BoundingBox)
    );
    assert!(translated.event_intersects(game.world.entity(selected).unwrap()));
    translated.pos = Vec2::new(80.0, 80.0);
    assert!(!translated.event_intersects(game.world.entity(selected).unwrap()));
    // Physical Rust storage is deliberately independent of established source
    // family registration order; a raw world.entities scan chooses the wrong one.
    game.world.entities.reverse();

    dig(&mut game);

    assert!(game.world.entity(selected).is_none());
    assert!(game.world.entity(gate).is_some());
    cave(&game, Vec2::new(96.0, 80.0));
}

#[test]
fn translated_overlap_without_center_point_gate_does_not_release() {
    let mut game = room(RulesPreset::Classic);
    ice(&mut game, Storage::Tile, "oIce");
    let frozen = game.spawn("oFrozenCaveman", Vec2::new(96.0, 80.0));
    dig(&mut game);
    assert!(game.world.entity(frozen).is_some());
    assert_eq!(count(&game, "oCaveman"), 0);
    assert_eq!(count(&game, "oDrip"), 3);
}

#[test]
fn inactive_frozen_caveman_cannot_gate_or_win_the_single_target_query() {
    for inactive_only in [false, true] {
        let mut game = room(RulesPreset::Classic);
        ice(&mut game, Storage::Tile, "oIce");
        let inactive = game.spawn("oFrozenCaveman", Vec2::new(80.0, 80.0));
        game.world.entity_mut(inactive).unwrap().engine_active = false;
        let active = (!inactive_only).then(|| game.spawn("oFrozenCaveman", Vec2::new(80.0, 80.0)));
        dig(&mut game);
        assert!(game.world.entity(inactive).is_some());
        if let Some(active) = active {
            assert!(game.world.entity(active).is_none());
            cave(&game, Vec2::new(80.0, 80.0));
        } else {
            assert_eq!(count(&game, "oCaveman"), 0);
        }
    }
}

#[test]
fn iceblock_own_destroy_emits_drips_and_leaves_frozen_until_its_step() {
    let mut game = room(RulesPreset::Classic);
    ice(&mut game, Storage::Entity, "oIceBlock");
    let frozen = game.spawn("oFrozenCaveman", Vec2::new(80.0, 80.0));
    dig(&mut game);
    assert!(game.world.entity(frozen).is_some());
    assert_eq!(count(&game, "oCaveman"), 0);
    assert_eq!(count(&game, "oDrip"), 3);
    world_events::tick(&mut game, &InputFrame::default());
    assert!(game.world.entity(frozen).is_none());
    cave(&game, Vec2::new(80.0, 80.0));
}

#[test]
fn ice_local_clean_guard_defers_frozen_own_destroy_which_ignores_local_clean() {
    let mut game = room(RulesPreset::Classic);
    let ice = game.spawn("oIce", Vec2::new(80.0, 80.0));
    game.world
        .entity_mut(ice)
        .unwrap()
        .data
        .insert("cleanDeath".into(), 1.0);
    let frozen = game.spawn("oFrozenCaveman", Vec2::new(80.0, 80.0));
    game.world
        .entity_mut(frozen)
        .unwrap()
        .data
        .insert("cleanDeath".into(), 1.0);
    dig(&mut game);
    assert!(game.world.entity(frozen).is_some());
    assert_eq!(count(&game, "oDrip"), 0);
    assert_eq!(count(&game, "oCaveman"), 0);
    world_events::tick(&mut game, &InputFrame::default());
    assert!(game.world.entity(frozen).is_none());
    cave(&game, Vec2::new(80.0, 80.0));
}

#[test]
fn global_clean_suppresses_both_callbacks_for_flag_and_numeric_state() {
    for numeric in [false, true] {
        let mut game = room(RulesPreset::Classic);
        ice(&mut game, Storage::Tile, "oIce");
        let frozen = game.spawn("oFrozenCaveman", Vec2::new(80.0, 80.0));
        if numeric {
            game.data.insert("cleanSolids".into(), 1.0);
        } else {
            game.flags.insert("cleanSolids".into());
        }
        dig(&mut game);
        assert!(game.world.entity(frozen).is_some());
        assert_eq!(count(&game, "oDrip"), 0);
        assert_eq!(count(&game, "oCaveman"), 0);
        world_events::tick(&mut game, &InputFrame::default());
        assert!(game.world.entity(frozen).is_none());
        assert_eq!(count(&game, "oCaveman"), 0);
    }
}

#[test]
fn explicit_and_metadata_occupants_share_one_release_but_adjacent_metadata_does_not() {
    for same_cell in [false, true] {
        let mut game = room(RulesPreset::Classic);
        ice(&mut game, Storage::Tile, "oIce");
        let metadata = game.spawn(
            "oTerrainMetadata",
            Vec2::new(if same_cell { 88.0 } else { 72.0 }, 88.0),
        );
        game.world
            .entity_mut(metadata)
            .unwrap()
            .data
            .insert("frozen_caveman".into(), 1.0);
        let frozen = game.spawn("oFrozenCaveman", Vec2::new(80.0, 80.0));
        dig(&mut game);
        assert!(game.world.entity(frozen).is_none());
        cave(&game, Vec2::new(80.0, 80.0));
        world_events::tick(&mut game, &InputFrame::default());
        assert_eq!(count(&game, "oCaveman"), 1);
    }
}

#[test]
fn locally_clean_metadata_cannot_suppress_the_later_explicit_frozen_destroy() {
    let mut game = room(RulesPreset::Classic);
    ice(&mut game, Storage::Tile, "oIce");
    let metadata = game.spawn("oTerrainMetadata", Vec2::new(88.0, 88.0));
    let stored = game.world.entity_mut(metadata).unwrap();
    stored.data.insert("frozen_caveman".into(), 1.0);
    stored.data.insert("cleanDeath".into(), 1.0);
    let frozen = game.spawn("oFrozenCaveman", Vec2::new(80.0, 80.0));
    dig(&mut game);
    assert!(game.world.entity(frozen).is_some());
    assert!(game.world.entity(metadata).is_none());
    assert_eq!(count(&game, "oDrip"), 0);
    assert_eq!(count(&game, "oCaveman"), 0);
    world_events::tick(&mut game, &InputFrame::default());
    assert!(game.world.entity(frozen).is_none());
    cave(&game, Vec2::new(80.0, 80.0));
}

#[test]
fn unconsumed_metadata_without_a_terrain_release_cannot_suppress_entity_ice_destroy() {
    let mut game = room(RulesPreset::Classic);
    ice(&mut game, Storage::Entity, "oIce");
    let metadata = game.spawn("oTerrainMetadata", Vec2::new(88.0, 88.0));
    game.world
        .entity_mut(metadata)
        .unwrap()
        .data
        .insert("frozen_caveman".into(), 1.0);
    let frozen = game.spawn("oFrozenCaveman", Vec2::new(80.0, 80.0));
    dig(&mut game);
    assert!(game.world.entity(metadata).is_some());
    assert!(game.world.entity(frozen).is_none());
    cave(&game, Vec2::new(80.0, 80.0));
}

#[test]
fn completed_metadata_receipt_suppresses_a_remaining_explicit_representation() {
    let mut game = room(RulesPreset::Classic);
    ice(&mut game, Storage::Tile, "oIce");
    let metadata = game.spawn("oTerrainMetadata", Vec2::new(88.0, 88.0));
    game.world
        .entity_mut(metadata)
        .unwrap()
        .data
        .insert("frozen_caveman".into(), 1.0);
    let first = game.spawn("oFrozenCaveman", Vec2::new(80.0, 80.0));
    let remaining = game.spawn("oFrozenCaveman", Vec2::new(80.0, 80.0));
    dig(&mut game);
    assert!(game.world.entity(first).is_none());
    assert!(game.world.entity(remaining).is_some());
    cave(&game, Vec2::new(80.0, 80.0));
    world_events::tick(&mut game, &InputFrame::default());
    assert!(game.world.entity(remaining).is_none());
    cave(&game, Vec2::new(80.0, 80.0));
}
