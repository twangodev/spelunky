use crate::{Game, GameEvent, RulesPreset, Vec2};

fn room() -> Game {
    let mut game = Game::shell(17, RulesPreset::Classic);
    game.player.pos = Vec2::new(80.0, 80.0);
    game.player.health = 4;
    game
}

#[test]
fn player_spike_query_ignores_inactive_entity_and_terrain() {
    for terrain in [false, true] {
        let mut game = room();
        game.player.vel.y = 1.0;
        game.player.data.insert("fall_ticks".into(), 5.0);
        if terrain {
            game.world.set_tile(5, 5, "oSpikes");
            game.world.tiles[5 * game.world.width + 5].engine_active = false;
        } else {
            let id = game.spawn("oSpikes", Vec2::new(80.0, 80.0));
            game.world.entity_mut(id).unwrap().engine_active = false;
        }
        crate::dispatcher::register_instances(&mut game);
        if !terrain {
            let id = game
                .world
                .entities
                .iter()
                .find(|e| e.object == "oSpikes")
                .unwrap()
                .id;
            game.world.entity_mut(id).unwrap().engine_active = false;
        }

        super::player_spikes(&mut game);

        assert_eq!(game.player.health, 4, "terrain={terrain}");
        assert!(!game.events.contains(&GameEvent::Damage { amount: 99 }));
    }
}

#[test]
fn spear_player_query_ignores_inactive_instances() {
    let mut game = room();
    let id = game.spawn("oSpearsLeft", Vec2::new(72.0, 80.0));
    let spear = game.world.entity_mut(id).unwrap();
    spear.animation = 21.0;
    spear.engine_active = false;
    crate::dispatcher::register_instances(&mut game);
    game.world.entity_mut(id).unwrap().engine_active = false;

    crate::traps::detectors::player_hurt(&mut game);

    assert_eq!(game.player.health, 4);
    assert!(!game.events.contains(&GameEvent::Damage { amount: 4 }));
}

#[test]
fn spear_player_query_uses_first_overlapping_instance_in_source_order() {
    let mut game = room();
    let damaging = game.spawn("oSpearsLeft", Vec2::new(72.0, 80.0));
    game.world.entity_mut(damaging).unwrap().animation = 21.0;
    let safe = game.spawn("oSpearsLeft", Vec2::new(72.0, 80.0));
    game.world.entity_mut(safe).unwrap().animation = 19.0;
    crate::dispatcher::register_instances(&mut game);
    let order = &mut game.dispatcher.registration_order;
    let damaging_order = order
        .iter()
        .position(|instance| *instance == crate::dispatcher::Instance::Entity(damaging))
        .unwrap();
    let safe_order = order
        .iter()
        .position(|instance| *instance == crate::dispatcher::Instance::Entity(safe))
        .unwrap();
    order.swap(damaging_order, safe_order);

    crate::traps::detectors::player_hurt(&mut game);

    assert_eq!(game.player.health, 4);
    assert!(!game.events.contains(&GameEvent::Damage { amount: 4 }));
}

#[test]
fn ceiling_query_ignores_inactive_instances() {
    let mut game = room();
    let id = game.spawn("oCeilingTrap", Vec2::new(78.0, 64.0));
    let trap = game.world.entity_mut(id).unwrap();
    trap.state = 1;
    trap.engine_active = false;
    crate::dispatcher::register_instances(&mut game);
    game.world.entity_mut(id).unwrap().engine_active = false;

    crate::traps::moving::player_hurt(&mut game);

    assert_eq!(game.player.health, 4);
    assert!(!game.events.contains(&GameEvent::Damage { amount: 10 }));
}

#[test]
fn ceiling_query_uses_first_overlapping_instance_in_source_order() {
    let mut game = room();
    let damaging = game.spawn("oCeilingTrap", Vec2::new(78.0, 64.0));
    game.world.entity_mut(damaging).unwrap().state = 1;
    let safe = game.spawn("oCeilingTrap", Vec2::new(78.0, 64.0));
    game.world.entity_mut(safe).unwrap().state = 0;
    crate::dispatcher::register_instances(&mut game);
    let order = &mut game.dispatcher.registration_order;
    let damaging_order = order
        .iter()
        .position(|instance| *instance == crate::dispatcher::Instance::Entity(damaging))
        .unwrap();
    let safe_order = order
        .iter()
        .position(|instance| *instance == crate::dispatcher::Instance::Entity(safe))
        .unwrap();
    order.swap(damaging_order, safe_order);

    crate::traps::moving::player_hurt(&mut game);

    assert_eq!(game.player.health, 4);
    assert!(!game.events.contains(&GameEvent::Damage { amount: 10 }));
}
