//! Authored default Chest/Ghost/floor, then ordinary inputs only.
//! Original Player Step443 refreshes Attack when dead; kUp stays cached from
//! characterStepEvent. Containers603–710 precede the dead-action gate992.
use spelunky_core::*;

fn ready(rules: RulesPreset, ghost: bool) -> (Game, u64) {
    let mut game = Game::shell(7, rules);
    game.world = World::new(30, 20, Area::Mines, 1);
    game.world.name = "rLevel".into();
    for column in 0..30 {
        game.world.set_tile(column, 10, "oHardBlock");
    }
    game.player.pos = Vec2::new(104.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.world.entrance = game.player.pos;
    let chest = game.spawn("oChest", game.player.pos);
    if ghost {
        game.spawn("oGhost", Vec2::new(128.0, 136.0));
    }
    assert_eq!(game.world.entity(chest).unwrap().sprite, "sChest");
    assert_eq!(game.player.health, 4.0);
    assert!(game.player.engine_active);
    (game, chest)
}

fn real_hidden_death(game: &mut Game, chest: u64, cached_up: bool) {
    let mut saw_ghost_kill = false;
    for _ in 0..132 {
        game.step(&InputFrame {
            up: cached_up,
            ..Default::default()
        });
        assert_eq!(game.world.entity(chest).unwrap().sprite, "sChest");
        assert_eq!(game.player.attack_timer, 0);
        let visible = game.player.data.get("visible").copied().unwrap_or(1.0) != 0.0;
        if !saw_ghost_kill && !visible {
            assert_eq!(game.player.health, -99.0, "FIXTURE actual Ghost collision");
            assert_eq!(game.player.invincible, 9999);
            assert!(game.events.iter().any(|event| matches!(event,
                GameEvent::Sound { name, .. } if name == "sndGhost"
            )));
            assert_eq!(game.player.data.get("active").copied().unwrap_or(1.0), 1.0);
            saw_ghost_kill = true;
        }
        if game.state == RunState::Dead {
            assert!(saw_ghost_kill, "FIXTURE Ghost caused the real death");
            assert!(!visible);
            assert_eq!(game.player.health, 0.0);
            assert_eq!(game.player.state, PlayerState::Dead);
            assert!(game.player.engine_active);
            assert_eq!(game.player.data.get("active").copied().unwrap_or(1.0), 1.0);
            assert_eq!(
                game.player.data.get("source_k_up").copied().unwrap_or(0.0),
                f64::from(cached_up)
            );
            assert!(
                game.game_over.draw_status < 3,
                "FIXTURE first Game Over edge must reveal, not change room"
            );
            assert!(!game.game_over.acknowledged);
            assert!(game.world.entity(chest).unwrap().engine_active);
            assert!(
                game.world
                    .entity(chest)
                    .unwrap()
                    .mask_contains(game.player.pos, CollisionPrecision::BoundingBox),
                "FIXTURE real corpse remains at closed Chest point"
            );
            assert_eq!(game.lifetime_stats.total_chests, 0);
            return;
        }
    }
    panic!(
        "FIXTURE actual Ghost death timeout: tick={} hp={} state={:?} pos={:?}",
        game.tick, game.player.health, game.state, game.player.pos
    );
}

#[test]
fn real_ghost_death_retains_cached_up_and_opens_chest_on_first_attack_edge() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, chest) = ready(rules, true);
        real_hidden_death(&mut game, chest, true);
        let before_id = game
            .world
            .entities
            .iter()
            .map(|actor| actor.id)
            .max()
            .unwrap();
        // Physical Up is now released, but dead Player does not run the
        // controller that overwrites kUp. Its fresh Attack edge is sampled.
        game.step(&InputFrame {
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert_eq!(game.state, RunState::Dead);
        assert!(!game.game_over.acknowledged);
        assert_eq!(game.game_over.draw_status, 3);
        assert_eq!(
            game.player.data.get("source_k_up").copied().unwrap_or(0.0),
            1.0
        );
        assert_eq!(
            game.world.entity(chest).unwrap().sprite,
            "sChestOpen",
            "SOURCE containers precede the dead/stunned inventory-action gate"
        );
        assert_eq!(game.lifetime_stats.total_chests, 1);
        assert_eq!(game.data["totalChests"], 1.0);
        assert!(
            game.world.entities.iter().any(|actor| actor.id > before_id),
            "native trapped Bomb or treasure birth"
        );
        assert!(game.events.iter().any(|event| matches!(event,
            GameEvent::Sound { name, .. } if name == "sndChestOpen" || name == "sndTrap"
        )));
        assert_eq!(
            game.player.attack_timer, 0,
            "dead Actor does not begin a whip"
        );
        assert_eq!(game.player.held, None);
    }
}

#[test]
fn real_ghost_death_does_not_refresh_up_from_the_new_attack_frame() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, chest) = ready(rules, true);
        real_hidden_death(&mut game, chest, false);
        game.step(&InputFrame {
            up: true,
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert_eq!(game.state, RunState::Dead);
        assert!(!game.game_over.acknowledged);
        assert_eq!(game.game_over.draw_status, 3);
        assert_eq!(
            game.player.data.get("source_k_up").copied().unwrap_or(0.0),
            0.0
        );
        assert_eq!(game.world.entity(chest).unwrap().sprite, "sChest");
        assert_eq!(game.lifetime_stats.total_chests, 0);
        assert!(!game.events.iter().any(|event| matches!(event,
            GameEvent::Sound { name, .. } if name == "sndChestOpen" || name == "sndTrap"
        )));
        assert_eq!(game.player.attack_timer, 0);
    }
}

#[test]
fn ordinary_live_up_attack_still_opens_the_same_default_chest() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, chest) = ready(rules, false);
        game.step(&InputFrame {
            up: true,
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert_eq!(game.player.health, 4.0);
        assert_eq!(game.state, RunState::Playing);
        assert_eq!(game.world.entity(chest).unwrap().sprite, "sChestOpen");
        assert_eq!(game.lifetime_stats.total_chests, 1);
        assert_eq!(game.player.attack_timer, 0);
    }
}
