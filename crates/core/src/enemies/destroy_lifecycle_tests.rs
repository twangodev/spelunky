//! Direct source-callback controls, separate from the public simulation witnesses.
//! Receiver deletion/held flags below are explicit callback inputs, not claims
//! of a naturally reached pickup or collision trajectory.
use super::*;

#[test]
fn enemy_destroy_preserves_receiver_held_and_releases_backlinks_only_once() {
    for object in ["oTombLord", "oShopkeeper", "oShopkeeper2", "oCaveman"] {
        let mut game = Game::shell(17, RulesPreset::Classic);
        let actor_id = game.spawn(object, Vec2::new(80.0, 80.0));
        let bomb_id = game.spawn("oBomb", Vec2::new(88.0, 88.0));
        game.world
            .entity_mut(bomb_id)
            .unwrap()
            .data
            .insert("attached_id".into(), actor_id as f64);
        let index = game
            .world
            .entities
            .iter()
            .position(|e| e.id == actor_id)
            .unwrap();
        let mut actor = game.world.entities.remove(index);
        actor.held = true;
        actor.alive = false;
        actor.data.insert("bomb_id".into(), bomb_id as f64);
        game.player.held = Some(actor_id);
        game.player.pickup_item_type = "Rock".into();

        destroyed(&mut game, &mut actor);

        assert!(!actor.alive, "Destroy never revives its receiver");
        assert!(
            actor.held,
            "source Enemy Destroy leaves receiver.held unchanged: {object}"
        );
        assert_eq!(game.player.held, None);
        assert_eq!(
            game.player.pickup_item_type, "Rock",
            "pickupItem and pickupItemType are distinct"
        );
        assert_eq!(
            game.world.entity(bomb_id).unwrap().value("attached_id"),
            0.0
        );
        assert_eq!(actor.value("enemy_destroyed"), 1.0);

        // A repeated adapter/finalizer visit must not clear a subsequent hold.
        let next = game.spawn("oRock", Vec2::new(200.0, 80.0));
        game.player.held = Some(next);
        let mut expected = game.rng.clone();
        destroyed(&mut game, &mut actor);
        game.world.entities.push(actor);
        finish_destroy_events(&mut game);
        assert_eq!(
            game.player.held,
            Some(next),
            "Destroy is not dispatched twice: {object}"
        );
        assert_eq!(game.rng.clone().next_u64(), expected.next_u64());
    }
}

#[test]
fn inherited_top_lava_finishes_destroy_before_the_following_parent_statements() {
    let mut game = Game::shell(42, RulesPreset::Classic);
    game.world = World::new(24, 20, Area::Temple, 13);
    game.player.pos = Vec2::new(136.0, 152.0);
    game.spawn("oLava", Vec2::new(128.0, 128.0));
    let actor_id = game.spawn("oCaveman", Vec2::new(132.0, 140.0));
    let bomb_id = game.spawn("oBomb", Vec2::new(136.0, 136.0));
    game.world
        .entity_mut(bomb_id)
        .unwrap()
        .data
        .insert("attached_id".into(), actor_id as f64);
    let index = game
        .world
        .entities
        .iter()
        .position(|e| e.id == actor_id)
        .unwrap();
    let mut actor = game.world.entities.remove(index);
    actor.held = true;
    actor.state = 98;
    actor.timer = 200;
    actor.data.insert("bomb_id".into(), bomb_id as f64);
    game.player.held = Some(actor_id);
    let mut expected = game.rng.clone();

    inherited::tick(&mut game, &mut actor);

    assert!(!actor.alive);
    assert!(
        actor.held,
        "source parent checks the old hold before its Destroy statement"
    );
    assert_eq!(
        actor.health, 3,
        "top-only Lava does not run the foot-Lava HP assignment"
    );
    assert_eq!(
        game.player.held, None,
        "callback must finish inside the parent, not at with_actor return"
    );
    assert_eq!(
        game.world.entity(bomb_id).unwrap().value("attached_id"),
        0.0
    );
    assert_eq!(actor.value("enemy_destroyed"), 1.0);
    assert_eq!(game.rng.clone().next_u64(), expected.next_u64());
}

#[test]
fn scarab_destroy_override_does_not_run_inherited_enemy_held_cleanup() {
    let mut game = Game::shell(7, RulesPreset::Classic);
    let id = game.spawn("oScarab", Vec2::new(80.0, 80.0));
    let index = game.world.entities.iter().position(|e| e.id == id).unwrap();
    let mut scarab = game.world.entities.remove(index);
    scarab.alive = false;
    scarab.held = true;
    game.player.held = Some(id);

    destroyed(&mut game, &mut scarab);

    assert!(scarab.held);
    assert_eq!(
        game.player.held,
        Some(id),
        "Scarab's own Destroy has no action_inherited"
    );
    assert_eq!(scarab.value("enemy_destroyed"), 0.0);
    assert_eq!(
        game.world
            .entities
            .iter()
            .filter(|e| e.object == "oFlareSpark")
            .count(),
        3
    );
    let mut expected = game.rng.clone();
    destroyed(&mut game, &mut scarab);
    game.world.entities.push(scarab);
    finish_destroy_events(&mut game);
    assert_eq!(game.player.held, Some(id));
    assert_eq!(
        game.world
            .entities
            .iter()
            .filter(|e| e.object == "oFlareSpark")
            .count(),
        3
    );
    assert_eq!(game.rng.clone().next_u64(), expected.next_u64());
}

#[test]
fn inherited_sacrifice_finishes_destroy_before_parent_returns_with_countdown_control() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for count in [0.0, 1.0] {
            let mut game = Game::shell(17, rules);
            game.world.set_tile(4, 4, "oSacAltarLeft");
            let id = game.spawn("oCaveman", Vec2::new(64.0, 48.0));
            let bomb_id = game.spawn("oBomb", Vec2::new(72.0, 56.0));
            game.world
                .entity_mut(bomb_id)
                .unwrap()
                .data
                .insert("attached_id".into(), id as f64);
            let index = game.world.entities.iter().position(|e| e.id == id).unwrap();
            let mut actor = game.world.entities.remove(index);
            // Explicit direct-parent callback input, not a natural sacrifice trajectory.
            actor.state = 98;
            actor.timer = 200;
            actor.vel = Vec2::default();
            actor.data.insert("sacrifice_count".into(), count);
            actor.data.insert("bomb_id".into(), bomb_id as f64);

            inherited::tick(&mut game, &mut actor);

            assert!(!actor.held, "an unheld sacrifice stays unheld");
            assert_eq!(actor.alive, count > 0.0);
            assert_eq!(actor.value("enemy_destroyed"), f64::from(count == 0.0));
            assert_eq!(
                game.world.entity(bomb_id).unwrap().value("attached_id"),
                if count == 0.0 { 0.0 } else { id as f64 },
                "source Destroy unlinks the Bomb at the sacrifice statement, before returning"
            );
            assert_eq!(
                game.world
                    .entities
                    .iter()
                    .filter(|e| e.object == "oBlood")
                    .count(),
                if count == 0.0 { 3 } else { 0 }
            );
            if count == 0.0 {
                assert_eq!(game.data["favor"], 2.0);
                assert_eq!(game.data["shake"], 10.0);
                assert_eq!(game.data["message_timer"], 200.0);
                let mut expected = game.rng.clone();
                let births = game.world.entities.len();
                destroyed(&mut game, &mut actor);
                assert_eq!(game.world.entities.len(), births);
                assert_eq!(game.rng.clone().next_u64(), expected.next_u64());
            } else {
                assert_eq!(actor.value("sacrifice_count"), 0.0);
                assert_eq!(game.data.get("favor").copied().unwrap_or(0.0), 0.0);
            }
        }
    }
}
