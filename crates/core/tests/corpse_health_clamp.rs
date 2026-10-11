use spelunky_core::{Area, Game, GameEvent, InputFrame, RulesPreset, RunState, Vec2, World};

fn lava_room(rules: RulesPreset) -> Game {
    let mut game = Game::shell(7, rules);
    game.world = World::new(30, 20, Area::Mines, 1);
    game.world.name = "rLevel".into();
    for x in 0..30 { game.world.set_tile(x, 10, "oHardBlock"); }
    // The initial solid sides and floor retain the source fluid cell.
    game.world.set_tile(4, 9, "oHardBlock");
    game.world.set_tile(6, 9, "oHardBlock");
    game.world.set_tile(5, 9, "oLava");
    game.player.pos = Vec2::new(88.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.grounded = true;
    game
}

#[test]
fn actual_lava_first_death_clamps_once_and_next_corpse_step_keeps_damage() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = lava_room(rules);
        game.step(&InputFrame::default());
        assert!(game.events.contains(&GameEvent::Damage { amount: 99 }));
        assert_eq!(game.player.health, 0, "first DIED action still sets life zero");
        assert_eq!(game.state, RunState::Dead);
        assert_eq!(game.events.iter().filter(|e| **e == GameEvent::Died).count(), 1);
        assert_eq!(game.player.data.get("hidden"), None);
        assert_eq!(game.player.pos, Vec2::new(88.0, 152.0));
        game.step(&InputFrame::default());
        assert!(game.events.contains(&GameEvent::Damage { amount: 99 }));
        assert_eq!(game.player.health, -99, "already-dead DIED action cannot erase Lava damage");
        assert_eq!(game.state, RunState::Dead);
        assert!(!game.events.contains(&GameEvent::Died));
        assert!(!game.events.iter().any(|e| matches!(e, GameEvent::Sound { name, .. } if name == "sndDie")));
        assert_eq!(game.data.get("death_ticks"), Some(&1.0));
        assert_eq!(game.player.data.get("hidden"), None, "exact -99 remains visible");
    }
}

#[test]
fn actual_first_fatal_lava_preserves_initial_carried_ankh_revival_before_dead_latch() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = lava_room(rules);
        // Controlled source-legal initial carried equipment; no pickup claim.
        game.player.equipment.insert("Ankh".into());
        game.world.entrance = Vec2::new(40.0, 152.0);
        game.step(&InputFrame::default());
        assert!(game.events.contains(&GameEvent::Damage { amount: 99 }));
        assert_eq!(game.player.health, 4);
        assert_eq!(game.state, RunState::Playing);
        assert_eq!(game.player.pos, game.world.entrance);
        assert_eq!(game.player.vel, Vec2::default());
        assert!(!game.player.has("Ankh"));
        assert_eq!(game.player.data.get("active"), Some(&1.0));
        assert_eq!(game.player.data.get("hidden"), None);
        assert!(!game.events.contains(&GameEvent::Died));
        assert!(game.events.iter().any(|e| matches!(e, GameEvent::Sound { name, .. } if name == "sndTeleport")));
    }
}
