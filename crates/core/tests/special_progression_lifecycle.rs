//! Integrated state-boundary coverage for the Ice Caves' Alien Ship route.
//! This selects the source route through a controlled prior `madeMoai` state
//! and a deterministic source-valid RNG draw; it does not test natural
//! traversal or the random route-selection odds.
use spelunky_core::{
    Game, GameEvent, InputFrame, PlayerState, RulesPreset, RunState, Vec2,
    ending::{EndingKind, EndingStage},
};

#[test]
fn alien_craft_room_identity_survives_route_flag_consumption_and_clears_on_exit() {
    let mut game = Game::shell(5, RulesPreset::Classic);
    // scrLevelGen reaches its alienCraft branch only after the Moai event has
    // already been made. Stage only that persistent source state, then select
    // the branch from seed 5's real next-level boundary draws. This prevents
    // stale per-room `alienCraft` state from artificially forcing the route.
    game.world.level = 8;
    game.flags.insert("made_moai".into());
    game.next_level();

    assert_eq!(game.world.level, 9);
    assert_eq!(game.world.name, "Alien Ship");
    assert_eq!(game.world.area, spelunky_core::Area::Ice);
    assert!(
        game.world
            .entities
            .iter()
            .any(|entity| { entity.alive && entity.object == "oAlienBoss" })
    );
    assert!(
        !game.flags.contains("alienCraft"),
        "per-room source flag is consumed after generation"
    );
    assert_eq!(game.player.alarms[0], 10);

    for _ in 0..10 {
        game.step(&InputFrame::default());
    }
    assert!(game.events.iter().any(|event| matches!(
        event,
        GameEvent::Dialogue(message)
            if message.first == "THERE'S A PSYCHIC PRESENCE HERE!"
    )));

    // Position only at the generated portal to test its live Player Step and
    // Animation End sequence. Natural traversal of the Mothership layout is a
    // separate question from this route-state regression.
    let exit = game
        .world
        .entities
        .iter()
        .find(|entity| entity.alive && entity.object == "oExit")
        .expect("Alien Ship has its generated campaign exit");
    let bounds = exit.sprite_bounds();
    let center = Vec2::new(bounds.x + bounds.w / 2.0, bounds.y + bounds.h / 2.0);
    game.player.pos = center;
    game.player.previous_pos = center;
    game.player.vel = Vec2::default();
    game.player.grounded = true;
    game.player.state = PlayerState::Standing;
    game.player.invincible = 999;
    game.update_camera();
    game.previous_camera = game.camera;
    game.step(&InputFrame::default());
    game.step(&InputFrame {
        up: true,
        ..Default::default()
    });

    assert_eq!(game.state, RunState::Ending);
    assert_eq!(game.ending.as_ref().unwrap().kind, EndingKind::Transition);
    assert_eq!(game.ending.as_ref().unwrap().stage, EndingStage::Exit);
    assert_eq!(game.data["next_level"], 10.0);

    for _ in 0..100 {
        if game.ending.as_ref().unwrap().stage != EndingStage::Exit {
            break;
        }
        game.step(&InputFrame::default());
    }
    assert_eq!(game.ending.as_ref().unwrap().stage, EndingStage::Transition);
    for tick in 0..600 {
        if game.state == RunState::Transition {
            break;
        }
        game.step(&InputFrame {
            attack: tick % 2 == 0,
            attack_pressed: tick % 2 == 0,
            ..Default::default()
        });
    }
    assert_eq!(game.state, RunState::Transition);
    game.next_level();
    assert_eq!(game.world.level, 10);
    assert_ne!(game.world.name, "Alien Ship");
    assert!(!game.flags.contains("alienCraft"));
}
