//! Controlled source selector predicates; not natural campaign witnesses.
use spelunky_core::*;

trait SourceTestMask {
    fn source_test_mask(&self) -> catalog::CollisionMask;
}
impl SourceTestMask for Entity {
    fn source_test_mask(&self) -> catalog::CollisionMask {
        assert_eq!(self.data.get("image_xscale").copied().unwrap_or(1.0), 1.0);
        assert_eq!(self.data.get("image_yscale").copied().unwrap_or(1.0), 1.0);
        assert_eq!(self.data.get("image_angle").copied().unwrap_or(0.0), 0.0);
        catalog::instance_mask(
            &self.object,
            &self.sprite,
            self.pos,
            self.animation,
            Vec2::new(1.0, 1.0),
            0.0,
        )
    }
}

#[test]
fn source_raw_rectangle_family_includes_held_skips_inactive_and_ignores_solid_flag() {
    let mut world = World::new(8, 8, Area::Mines, 1);
    world.set_tile(0, 0, "oBlock");
    let a = Vec2::new(2.0, 2.0);
    let b = Vec2::new(3.0, 3.0);
    assert!(world.rectangle_family_except(a, b, "oSolid", None, CollisionPrecision::Precise));
    world.tiles[0].engine_active = false;
    assert!(!world.rectangle_family_except(a, b, "oSolid", None, CollisionPrecision::Precise));
    world.tiles[0].engine_active = true;
    assert!(world.rectangle_family_except(b, a, "oSolid", None, CollisionPrecision::Precise));

    let block = world.spawn("oPushBlock", Vec2::new(48.5, 32.0));
    let a = Vec2::new(49.0, 33.0);
    let b = Vec2::new(50.0, 34.0);
    world.entity_mut(block).unwrap().held = true;
    world.entity_mut(block).unwrap().solid = false;
    assert!(
        world
            .entity(block)
            .unwrap()
            .source_test_mask()
            .intersects_rectangle(a, b, CollisionPrecision::Precise)
    );
    assert!(world.rectangle_family_except(a, b, "oSolid", None, CollisionPrecision::Precise));
    assert!(!world.rectangle_family_except(
        a,
        b,
        "oSolid",
        Some(block),
        CollisionPrecision::Precise
    ));
    world.entity_mut(block).unwrap().engine_active = false;
    assert!(!world.rectangle_family_except(a, b, "oSolid", None, CollisionPrecision::Precise));
    world.entity_mut(block).unwrap().engine_active = true;
    world.entity_mut(block).unwrap().alive = false;
    let rock = world.spawn("oRock", Vec2::new(50.0, 32.0));
    world.entity_mut(rock).unwrap().solid = true;
    assert!(
        world
            .entity(rock)
            .unwrap()
            .source_test_mask()
            .intersects_rectangle(a, b, CollisionPrecision::Precise)
    );
    assert!(!catalog::is_a("oRock", "oSolid"));
    assert!(!world.rectangle_family_except(a, b, "oSolid", None, CollisionPrecision::Precise));
}
