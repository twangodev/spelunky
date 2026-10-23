//! Natural generated-campaign checks driven only by authoritative Game::step input.
use spelunky_core::{Game, InputFrame, PlayerState, RulesPreset, Vec2};

fn advance_to(game: &mut Game, level: u32) {
    while game.world.level < level {
        game.next_level();
    }
    assert_eq!(game.world.level, level);
}

#[test]
fn generated_key_chest_and_udjat_eye_interactions_use_game_step_inputs() {
    // The previous seed's room layout moved with the Mines RNG correction.
    // Find a live generated Key + Locked Chest through the normal campaign
    // generator; keep this test focused on Classic's object triggers rather
    // than treating one room-specific movement heuristic as universal.
    let mut generated = None;
    for seed in [42, 43, 44, 0, 1, 2] {
        let mut candidate = Game::new(seed, RulesPreset::Classic);
        loop {
            let has_key = candidate
                .world
                .entities
                .iter()
                .any(|entity| entity.alive && entity.object == "oKey");
            let has_chest = candidate
                .world
                .entities
                .iter()
                .any(|entity| entity.alive && entity.object == "oLockedChest");
            if has_key && has_chest {
                generated = Some(candidate);
                break;
            }
            if candidate.world.level >= 4 {
                break;
            }
            candidate.next_level();
        }
        if generated.is_some() {
            break;
        }
    }
    let mut game = generated.expect("bounded generated route contains Key and Locked Chest");
    assert!(game.flags.contains("made_udjat_eye"));

    let key_id = game
        .world
        .entities
        .iter()
        .find(|entity| entity.alive && entity.object == "oKey")
        .unwrap()
        .id;
    let chest_id = game
        .world
        .entities
        .iter()
        .find(|entity| entity.alive && entity.object == "oLockedChest")
        .unwrap()
        .id;
    let key_pos = game.world.entity(key_id).unwrap().pos;
    let chest_pos = game.world.entity(chest_id).unwrap().pos;

    // Stage beside the generated item so this test verifies the source pickup
    // dispatch. Physical traversal is intentionally covered by the separate
    // fresh-hang and Market route regressions.
    stage_player(&mut game, key_pos);
    game.step(&InputFrame {
        down: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert_eq!(game.player.held, Some(key_id));
    assert!(
        game.world
            .entity(key_id)
            .is_some_and(|key| key.alive && key.held)
    );

    // A carried generated Key colliding with its generated chest consumes the
    // Key and spawns the Udjat Eye, matching oLockedChest's source collision.
    stage_player(&mut game, chest_pos);
    game.step(&InputFrame::default());
    assert!(game.world.entity(key_id).is_none());
    assert!(game.world.entity(chest_id).is_none());
    assert!(game.player.held.is_none());
    let eye_pos = game
        .world
        .entities
        .iter()
        .find(|entity| entity.alive && entity.object == "oUdjatEye")
        .expect("opening the chest creates its Udjat Eye")
        .pos;
    stage_player(&mut game, eye_pos);
    game.step(&InputFrame {
        down: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert!(game.player.has("UdjatEye"));
}

fn stage_player(game: &mut Game, pos: Vec2) {
    game.player.pos = pos;
    game.player.previous_pos = pos;
    game.player.vel = Vec2::default();
    game.player.grounded = true;
    if let Some(id) = game.player.held {
        let held = game.world.entity_mut(id).unwrap();
        held.pos = pos;
        held.previous_pos = pos;
        held.vel = Vec2::default();
    }
    game.update_camera();
    game.previous_camera = game.camera;
}

#[test]
fn generated_seed_three_wall_hang_jumps_up_on_the_source_fresh_press() {
    let mut game = Game::new(3, RulesPreset::Classic);
    advance_to(&mut game, 4);

    // This deterministic input policy keeps walking left and jumps after a
    // short no-progress interval. It reaches the generated wall hang without
    // editing the player's position or any world object.
    let mut previous_x = game.player.pos.x;
    let mut stuck_ticks = 0;
    let mut jump_ticks = 0;
    let mut reached_hang = false;
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
            reached_hang = true;
            break;
        }
    }

    assert!(reached_hang, "the generated route must reach a wall hang");
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

    // The source player uses bounds y-8..y+8. The generated wall's upper
    // surface is the tile boundary above this 8-pixel-snapped hang position.
    let upper_surface_y = (hanging_y / 16.0).floor() * 16.0;
    let upper_player_y = upper_surface_y - 8.0;
    let mut cleared_wall = false;
    let mut landed = false;
    for _ in 0..30 {
        game.step(&InputFrame {
            left: true,
            run: true,
            jump: true,
            ..Default::default()
        });
        cleared_wall |= game.player.pos.y + 8.0 < upper_surface_y;
        if game.player.grounded {
            assert!(cleared_wall, "jump must clear the wall's upper edge");
            assert_eq!(game.player.pos.y, upper_player_y);
            let column = (game.player.pos.x / 16.0).floor() as i32;
            let row = (upper_surface_y / 16.0).floor() as i32;
            assert!(
                game.world.tile_collision_mask(column, row).contains(
                    Vec2::new(game.player.pos.x, upper_surface_y),
                    spelunky_core::CollisionPrecision::Precise,
                ),
                "landing point must touch the generated wall's top collision mask"
            );
            landed = true;
            break;
        }
    }
    assert!(landed, "generated wall jump must land on its upper surface");

    // Keep the same natural directional input for one more source tick. This
    // catches a one-frame landing that immediately loses support.
    game.step(&InputFrame {
        left: true,
        run: true,
        jump: true,
        ..Default::default()
    });
    assert!(
        game.player.grounded,
        "upper surface must support the next tick"
    );
    assert_eq!(game.player.pos.y, upper_player_y);
}
