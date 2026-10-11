//! Configured callback controls for terrain frame, mixed families and activation.
use super::*;
fn scene() -> Game {
    let mut game = Game::shell(7, RulesPreset::Classic);
    game.world = World::new(20, 20, Area::Mines, 1);
    game.player.pos = Vec2::new(200., 120.);
    game
}
#[test]
fn terrain_receiver_retains_metadata_current_frame_for_translated_query() {
    let mut g = scene();
    g.world.set_tile(6, 9, "oBrick");
    let meta = g.spawn("oTerrainMetadata", Vec2::new(104., 152.));
    {
        let m = g.world.entity_mut(meta).unwrap();
        m.data.insert("cleanDeath".into(), 1.);
        m.text.insert("terrain_sprite".into(), "sLamp".into());
        m.animation = 1.;
    }
    let first = g.spawn("oLamp", Vec2::new(91., 160.));
    {
        let lamp = g.world.entity_mut(first).unwrap();
        lamp.animation = 1.;
        lamp.set_image_speed(0.);
    }
    let second = g.spawn("oLamp", Vec2::new(112., 160.));
    // Current terrain frame1 includes x105 after translation; frame0 starts106.
    assert!(destroy_terrain_self(&mut g, 6, 9));
    assert!(g.world.entity(first).is_none());
    assert!(g.world.entity(second).is_some());
    let item = g
        .world
        .entities
        .iter()
        .find(|e| e.object == "oLampItem")
        .unwrap();
    assert_eq!(item.pos, Vec2::new(99., 172.));
    assert!(g.world.fluid_dirty);
}
#[test]
fn mixed_upper_family_selects_terrain_overlap_instead_of_dynamic_gate_owner() {
    let mut g = scene();
    let source = g.spawn("oThinIce", Vec2::new(80., 80.));
    g.world.set_tile(6, 5, "oSpikes");
    let gate = g.spawn("oSpikes", Vec2::new(80., 64.));
    assert!(destroy_solid_self(&mut g, source));
    assert!(g.world.tile(6, 5).unwrap().object.is_empty());
    assert!(g.world.entity(gate).is_some());
    assert!(g.world.fluid_dirty);
}
#[test]
fn inactive_point_gate_cannot_enable_a_query_on_active_adjacent_lamp() {
    let mut g = scene();
    let source = g.spawn("oThinIce", Vec2::new(80., 80.));
    let first = g.spawn("oLamp", Vec2::new(96., 96.));
    let gate = g.spawn("oLamp", Vec2::new(80., 96.));
    g.world.entity_mut(gate).unwrap().engine_active = false;
    assert!(destroy_solid_self(&mut g, source));
    assert!(g.world.entity(first).is_some());
    assert!(g.world.entity(gate).is_some());
    assert!(!g.world.entities.iter().any(|e| e.object == "oLampItem"));
    assert!(g.world.fluid_dirty);
}
