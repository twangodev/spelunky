//! Natural generated-campaign checks driven only by authoritative Game::step input.
use spelunky_core::{Game, InputFrame, PlayerState, RulesPreset, Vec2};

fn advance_to(game: &mut Game, level: u32) {
    while game.world.level < level {
        game.next_level();
    }
    assert_eq!(game.world.level, level);
}

#[test]
fn generated_key_chest_and_udjat_eye_route_uses_only_game_step_inputs() {
    // Seed 44, Mines 1-2: spawn (88,40), Key (184,76), Locked Chest (104,120).
    // No position or inventory fields are staged; the wall jump and pickups
    // happen through the same Player/Game dispatch as ordinary play.
    let mut game = Game::new(44, RulesPreset::Classic);
    advance_to(&mut game, 2);
    assert_eq!(game.player.pos, Vec2::new(88.0, 40.0));

    let key_id = game
        .world
        .entities
        .iter()
        .find(|entity| entity.alive && entity.object == "oKey")
        .expect("seed 44 generated Key")
        .id;
    let chest_id = game
        .world
        .entities
        .iter()
        .find(|entity| entity.alive && entity.object == "oLockedChest")
        .expect("seed 44 generated Locked Chest")
        .id;

    // Reach the generated wall with normal right/run input, then climb over it
    // with Classic's fresh jump press while continuing in the facing direction.
    let mut reached_hang = false;
    for _ in 0..32 {
        game.step(&InputFrame {
            right: true,
            run: true,
            ..Default::default()
        });
        if game.player.state == PlayerState::Hanging {
            reached_hang = true;
            break;
        }
    }
    assert!(
        reached_hang,
        "generated route must reach its natural wall hang"
    );
    assert_eq!(game.player.pos, Vec2::new(171.0, 88.0));

    // This is the source's HANGING jump path: a fresh jump starts the upward
    // release, then down+attack attempts the nearby generated Key. The first
    // pickup attempt is deliberately released and retried one frame later.
    let mut picked_key = false;
    for tick in 0..10 {
        let attack = tick == 1 || tick == 3;
        game.step(&InputFrame {
            right: true,
            run: true,
            jump: tick < 10,
            jump_pressed: tick == 0,
            down: attack,
            attack,
            attack_pressed: attack,
            ..Default::default()
        });
        if game.player.held == Some(key_id) {
            picked_key = true;
        }
    }
    assert!(picked_key, "Game::step pickup must hold the generated Key");
    assert!(
        game.world
            .entity(key_id)
            .is_some_and(|key| key.alive && key.held)
    );

    // oLockedChest Collision with oKey consumes the held Key and creates an
    // oUdjatEye at the chest. Keep steering left until that source interaction
    // occurs, then use the normal down+attack pickup for the spawned Eye.
    let mut saw_spawned_eye = false;
    let mut opened_chest = false;
    for _ in 0..80 {
        let eye = game
            .world
            .entities
            .iter()
            .find(|entity| entity.alive && entity.object == "oUdjatEye")
            .map(|entity| (entity.id, entity.pos));
        saw_spawned_eye |= eye.is_some();
        let collect_eye = eye.is_some_and(|(_, pos)| {
            (pos.x - game.player.pos.x).abs() < 16.0 && (pos.y - game.player.pos.y).abs() < 16.0
        });
        game.step(&InputFrame {
            left: true,
            run: true,
            down: collect_eye,
            attack: collect_eye,
            attack_pressed: collect_eye,
            ..Default::default()
        });
        opened_chest |= !game.world.entity(chest_id).is_some_and(|chest| chest.alive);
        saw_spawned_eye |= game
            .world
            .entities
            .iter()
            .any(|entity| entity.alive && entity.object == "oUdjatEye");
        if game.player.has("UdjatEye") {
            break;
        }
    }

    assert!(opened_chest, "the held generated Key must open its chest");
    assert!(
        saw_spawned_eye,
        "opening the chest must create its Udjat Eye"
    );
    assert!(
        !game.world.entity(key_id).is_some_and(|key| key.alive),
        "opening the chest must consume and remove the held Key"
    );
    assert!(game.player.held.is_none());
    assert!(
        game.player.has("UdjatEye"),
        "down+attack must collect the naturally spawned Udjat Eye"
    );
}

#[test]
fn generated_seed_three_wall_hang_jumps_up_on_the_source_fresh_press() {
    let mut game = Game::new(3, RulesPreset::Classic);
    advance_to(&mut game, 4);

    // This deterministic input policy keeps walking left and jumps after a
    // short no-progress interval. It reaches the actual generated hang at
    // (341,88) without editing the player's position or any world object.
    let mut previous_x = game.player.pos.x;
    let mut stuck_ticks = 0;
    let mut jump_ticks = 0;
    let mut reached_target_hang = false;
    for _ in 0..120 {
        let blocked = game.player.grounded && (game.player.pos.x - previous_x).abs() < 0.5;
        stuck_ticks = if blocked { stuck_ticks + 1 } else { 0 };
        previous_x = game.player.pos.x;
        if stuck_ticks >= 2 && jump_ticks == 0 {
            jump_ticks = 8;
            stuck_ticks = 0;
        }
        let jump = jump_ticks > 0;
        let jump_pressed = jump && jump_ticks == 8;
        if jump {
            jump_ticks -= 1;
        }
        game.step(&InputFrame {
            left: true,
            run: true,
            jump,
            jump_pressed,
            ..Default::default()
        });
        if game.player.state == PlayerState::Hanging {
            assert_eq!(game.player.pos, Vec2::new(341.0, 88.0));
            reached_target_hang = true;
            break;
        }
    }

    assert!(
        reached_target_hang,
        "the generated route must reach its target hang"
    );
    assert_eq!(game.player.facing, -1);
    let hanging_y = game.player.pos.y;

    // characterStepEvent.gml:495-532 consumes a fresh jump press while
    // HANGING. With facing left, left+jump launches upward and shifts away
    // from the edge before normal movement continues.
    game.step(&InputFrame {
        left: true,
        run: true,
        jump: true,
        jump_pressed: true,
        ..Default::default()
    });
    assert!(game.player.pos.y < hanging_y, "fresh jump must move upward");
    let first_y = game.player.pos.y;
    game.step(&InputFrame {
        left: true,
        run: true,
        jump: true,
        ..Default::default()
    });
    assert!(
        game.player.pos.y < first_y,
        "held jump continues the release arc"
    );
    assert_eq!(game.player.state, PlayerState::Jumping);
}

#[test]
fn generated_seed_three_market_distance_rearms_udjat_blink_by_real_movement() {
    let mut game = Game::new(3, RulesPreset::Classic);
    advance_to(&mut game, 6);

    let market = game
        .world
        .entities
        .iter()
        .find(|entity| entity.alive && entity.object == "oXMarket")
        .expect("seed 3 level 6 generated a market entrance");
    assert_eq!(market.pos, Vec2::new(64.0, 176.0));
    assert!(
        game.world
            .tile(4, 11)
            .is_some_and(|tile| tile.object.is_empty())
    );
    assert!(game.world.entities.iter().any(|entity| {
        entity.alive && entity.object == "oSpearTrapBottom" && entity.pos == market.pos
    }));

    // The player follows a short real route from the generated level spawn.
    for _ in 0..15 {
        game.step(&InputFrame {
            left: true,
            run: true,
            ..Default::default()
        });
    }
    for _ in 0..11 {
        game.step(&InputFrame {
            right: true,
            run: true,
            ..Default::default()
        });
    }
    assert_eq!(game.player.pos, Vec2::new(59.0, 120.0));
    assert_eq!(game.alarms[2], 48);
    assert!(!game.flags.contains("udjat_blink"));

    // oGame Step keeps the closest market alarm live; Alarm 2 toggles after it
    // counts down. This crosses the blink edge from actual Game::step calls.
    for _ in 0..48 {
        game.step(&InputFrame::default());
    }
    assert!(game.flags.contains("udjat_blink"));
}
