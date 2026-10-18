use super::*;
use crate::dispatcher::Instance;
use crate::{Area, RulesPreset, World};

fn room() -> Game {
    let mut game = Game::shell(13, RulesPreset::Classic);
    game.world = World::new(40, 30, Area::Mines, 1);
    game
}

#[test]
fn boulder_collision_selects_active_spike_in_source_instance_order() {
    let mut game = room();
    let inactive = game.world.spawn("oSpikes", Vec2::new(80.0, 64.0));
    game.world.entity_mut(inactive).unwrap().engine_active = false;
    let active = game.world.spawn("oSpikes", Vec2::new(80.0, 64.0));
    let boulder = game.world.spawn("oBoulder", Vec2::new(80.0, 80.0));
    game.world.entity_mut(boulder).unwrap().vel.x = 2.0;
    let solid = game.world.spawn("oBlock", Vec2::new(80.0, 80.0));
    crate::dispatcher::register_instances(&mut game);

    collision(
        &mut game,
        Instance::Entity(boulder),
        Instance::Entity(solid),
    );

    assert!(
        game.world
            .entities
            .iter()
            .find(|entity| entity.id == inactive)
            .unwrap()
            .alive
    );
    assert!(
        !game
            .world
            .entities
            .iter()
            .find(|entity| entity.id == active)
            .unwrap()
            .alive
    );
}

#[test]
fn boulder_collision_can_find_authored_terrain_spikes() {
    let mut game = room();
    game.world.set_tile(5, 4, "oSpikes");
    let boulder = game.world.spawn("oBoulder", Vec2::new(80.0, 80.0));
    game.world.entity_mut(boulder).unwrap().vel.x = 2.0;
    let solid = game.world.spawn("oBlock", Vec2::new(80.0, 80.0));
    crate::dispatcher::register_instances(&mut game);
    let spike_point = Vec2::new(88.0, 79.0);
    let spike_index = 4 * game.world.width + 5;
    assert!(
        crate::dispatcher::descendant_instances(&game, "oSpikes")
            .contains(&Instance::Terrain(spike_index))
    );
    assert!(crate::dispatcher::is_active(
        &game,
        Instance::Terrain(spike_index)
    ));
    assert!(
        game.world
            .tile_collision_mask(5, 4)
            .contains(spike_point, CollisionPrecision::BoundingBox)
    );

    collision(
        &mut game,
        Instance::Entity(boulder),
        Instance::Entity(solid),
    );

    assert!(game.world.tiles[4 * game.world.width + 5].object.is_empty());
}

#[test]
fn platform_collision_queries_ignore_inactive_solids() {
    let mut game = room();
    let inactive = game.world.spawn("oBlock", Vec2::new(80.0, 80.0));
    let solid = game.world.entity_mut(inactive).unwrap();
    solid.solid = true;
    solid.engine_active = false;
    let probe = Rect::new(80.0, 80.0, 1.0, 1.0);

    assert!(!collides_except(&game, probe, 0));
    assert!(!precise_collides_except(&game, probe, 0));
}

#[test]
fn platform_collision_queries_ignore_inactive_terrain_solids() {
    let mut game = room();
    game.world.set_tile(5, 5, "oBrick");
    game.world.tiles[5 * game.world.width + 5].engine_active = false;
    let probe = Rect::new(80.0, 80.0, 16.0, 16.0);

    assert!(!collides_except(&game, probe, 0));
    assert!(!precise_collides_except(&game, probe, 0));
}

#[test]
fn smash_lava_queries_ignore_inactive_entity_and_terrain() {
    let mut game = room();
    let inactive = game.world.spawn("oLava", Vec2::new(80.0, 80.0));
    game.world.entity_mut(inactive).unwrap().engine_active = false;
    let probe = Rect::new(80.0, 80.0, 16.0, 16.0);
    assert!(!touches_lava(&game, probe));

    game.world.set_tile(5, 5, "oLava");
    game.world.tiles[5 * game.world.width + 5].engine_active = false;
    assert!(!touches_lava(&game, probe));
}

#[test]
fn smash_step_ignores_inactive_tomb_lords() {
    for (active, expected_state) in [(false, 1), (true, 0)] {
        let mut game = room();
        let tomb_lord = game.world.spawn("oTombLord", Vec2::new(80.0, 80.0));
        game.world.entity_mut(tomb_lord).unwrap().engine_active = active;
        let mut trap = Entity::new(900, "oSmashTrap", Vec2::new(80.0, 80.0));
        trap.state = 1;
        trap.data.insert("direction".into(), 0.0);
        trap.data.insert("xv".into(), 1.0);
        trap.data.insert("yv".into(), 0.0);
        trap.data.insert("xa".into(), 0.0);
        trap.data.insert("ya".into(), 0.0);

        smash(&mut game, &mut trap);

        assert_eq!(trap.state, expected_state, "active={active}");
    }
}
