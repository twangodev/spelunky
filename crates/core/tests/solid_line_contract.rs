//! Controlled public query contract, not natural campaign reachability.
use spelunky_core::*;

#[test]
fn precise_solid_line_respects_source_family_activity_self_and_held() {
    let mut world = World::new(20, 20, Area::Mines, 1);
    let id = world.spawn("oBlock", Vec2::new(80.0, 80.0));
    let a = Vec2::new(80.0, 80.0);
    let b = Vec2::new(95.0, 80.0);
    assert!(world.solid_line_except(a, b, 0));
    world.entity_mut(id).unwrap().held = true;
    assert!(
        world.solid_line_except(a, b, 0),
        "source family line includes held Solid"
    );
    assert!(!world.solid_line_except(a, b, id));
    world.entity_mut(id).unwrap().engine_active = false;
    assert!(!world.solid_line_except(a, b, 0));
    world.entity_mut(id).unwrap().engine_active = true;
    world.entity_mut(id).unwrap().alive = false;
    assert!(!world.solid_line_except(a, b, 0));
    let other = world.spawn("oGoldNugget", Vec2::new(80.0, 80.0));
    world.entity_mut(other).unwrap().solid = true;
    assert!(
        !world.solid_line_except(a, b, 0),
        "engine flag does not change the oSolid family"
    );
    let solid = world.spawn("oBlock", Vec2::new(80.0, 80.0));
    world.entity_mut(solid).unwrap().solid = false;
    assert!(
        world.solid_line_except(a, b, 0),
        "family ancestry remains authoritative"
    );
}

#[test]
fn active_tile_filter_is_narrow_to_the_new_source_line_query() {
    for metadata_override in [false, true] {
        let mut world = World::new(20, 20, Area::Mines, 1);
        world.set_tile(5, 5, "oBlock");
        if metadata_override {
            let id = world.spawn("oTerrainMetadata", Vec2::new(80.0, 80.0));
            world
                .entity_mut(id)
                .unwrap()
                .text
                .insert("terrain_sprite".into(), "sBlock".into());
        }
        let a = Vec2::new(80.0, 80.0);
        let b = Vec2::new(95.0, 80.0);
        assert!(world.solid_line_except(a, b, 0));
        world.tiles[5 * world.width + 5].engine_active = false;
        assert!(
            !world.solid_line_except(a, b, 0),
            "inactive tile source line, override={metadata_override}"
        );
        assert!(
            world.tiles_line_precision(a, b, "oSolid", CollisionPrecision::Precise),
            "existing generic tile API behavior unchanged"
        );
        world.tiles[5 * world.width + 5].engine_active = true;
        assert!(world.solid_line_except(a, b, 0));
    }
}
