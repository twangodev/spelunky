use super::*;

fn graphic(x: f64, depth: f64) -> GeneratedGraphicTileSpec {
    GeneratedGraphicTileSpec {
        background: "bgCaveTop".into(),
        source: [16.0, 0.0, 16.0, 16.0],
        position: Vec2::new(x, 20.0),
        depth,
        scale: Vec2::new(1.0, 1.0),
        color: 0xffffff,
        alpha: 1.0,
        visible: true,
    }
}

#[test]
fn graphics_between_real_births_leave_actor_and_grid_chronology_untouched() {
    let mut world = World::new(2, 2, Area::Mines, 1);
    world.set_tile(0, 0, "oBrick");
    let first = world.spawn("oSnake", Vec2::new(40.0, 40.0));
    let before = (world.next_id, world.creation_serial, world.tile_revision);
    let tiles = serde_json::to_value(&world.tiles).unwrap();
    let entities = serde_json::to_value(&world.entities).unwrap();
    let a = world
        .add_generated_graphic_tile(graphic(10.0, 3.0))
        .unwrap();
    let b = world
        .add_generated_graphic_tile(graphic(20.0, 2.0))
        .unwrap();
    assert_ne!(a, b);
    assert_eq!(
        (world.next_id, world.creation_serial, world.tile_revision),
        before
    );
    assert_eq!(serde_json::to_value(&world.tiles).unwrap(), tiles);
    assert_eq!(serde_json::to_value(&world.entities).unwrap(), entities);
    let next = world.spawn("oBat", Vec2::new(50.0, 40.0));
    assert_eq!(next, before.0);
    assert_eq!(world.entity(next).unwrap().creation_serial, before.1 + 1);
    assert!(world.entity(first).is_some());
    assert_eq!(
        world.generated_graphic_tile(a).unwrap().graphic,
        graphic(10.0, 3.0)
    );
}

#[test]
fn removal_retains_source_array_order_and_does_not_reuse_graphic_handles() {
    let mut world = World::new(1, 1, Area::Mines, 1);
    let a = world.add_generated_graphic_tile(graphic(1.0, 0.0)).unwrap();
    let b = world.add_generated_graphic_tile(graphic(2.0, 3.0)).unwrap();
    let c = world
        .add_generated_graphic_tile(graphic(3.0, -0.0))
        .unwrap();
    let d = world.add_generated_graphic_tile(graphic(4.0, 2.0)).unwrap();
    assert!(world.remove_generated_graphic_tile(b));
    assert!(!world.remove_generated_graphic_tile(b));
    assert_eq!(world.remove_generated_graphics_at_depth(0.0), 2);
    assert_eq!(
        world
            .generated_graphic_tiles
            .iter()
            .map(|tile| tile.handle)
            .collect::<Vec<_>>(),
        [d]
    );
    assert!(world.generated_graphic_tile(a).is_none());
    assert!(world.generated_graphic_tile(c).is_none());
    world.generated_graphic_tiles.clear();
    let e = world.add_generated_graphic_tile(graphic(5.0, 2.0)).unwrap();
    assert_ne!(d, e);
    assert_eq!(world.creation_serial, 0);
    assert_eq!(world.next_id, 1);
}

#[test]
fn serialization_keeps_historical_empty_shape_and_resumes_nonempty_graphics() {
    let mut world = World::new(1, 1, Area::Mines, 1);
    let historical = serde_json::to_value(&world).unwrap();
    assert!(historical.get("generated_graphic_tiles").is_none());
    assert!(historical.get("generated_graphic_serial").is_none());
    let imported: World = serde_json::from_value(historical.clone()).unwrap();
    assert_eq!(serde_json::to_value(imported).unwrap(), historical);
    let first = world.add_generated_graphic_tile(graphic(5.0, 3.5)).unwrap();
    let mut restored: World =
        serde_json::from_value(serde_json::to_value(&world).unwrap()).unwrap();
    assert_eq!(
        restored.generated_graphic_tile(first).unwrap().graphic,
        graphic(5.0, 3.5)
    );
    let next = restored
        .add_generated_graphic_tile(graphic(6.0, 3.5))
        .unwrap();
    assert_ne!(first, next);
    assert_eq!(restored.generated_graphic_tiles.len(), 2);
    assert_eq!(restored.creation_serial, world.creation_serial);
    assert_eq!(restored.next_id, world.next_id);
}

#[test]
fn exhausted_graphic_sequence_errors_before_partial_append_or_actor_changes() {
    let mut world = World::new(1, 1, Area::Mines, 1);
    world.generated_graphic_serial = u64::MAX;
    let before = serde_json::to_value(&world).unwrap();
    assert_eq!(
        world.add_generated_graphic_tile(graphic(5.0, 3.0)),
        Err(GraphicTileAllocationError::HandleExhausted)
    );
    assert_eq!(serde_json::to_value(&world).unwrap(), before);
}
