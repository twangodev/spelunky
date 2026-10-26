//! Positioned-interaction acceptance for generated campaign content.
//! These checks teleport the player and grant survivability; they do not prove
//! that the generated routes are physically traversable or source-equivalent.
mod common;

use common::complete_source_exit;
use spelunky_core::{
    Area, Game, GenerationRevision, InputFrame, PlayerState, Rect, RulesPreset, RunState, Vec2,
    ending::{EndingKind, EndingStage},
};

fn finish_natural_level_transition(game: &mut Game) {
    for _ in 0..100 {
        if game
            .ending
            .as_ref()
            .is_none_or(|ending| ending.stage != EndingStage::Exit)
        {
            break;
        }
        game.step(&InputFrame::default());
    }
    assert_eq!(game.ending.as_ref().unwrap().stage, EndingStage::Transition);
    game.step(&InputFrame {
        start_pressed: true,
        ..InputFrame::default()
    });
    for tick in 0..600 {
        if game.state == RunState::Transition {
            break;
        }
        game.step(&InputFrame {
            attack_pressed: tick % 2 == 0,
            ..InputFrame::default()
        });
    }
    assert_eq!(game.state, RunState::Transition);
}

fn source_room_row(y: f64) -> usize {
    ((y - 16.0) / 128.0).floor().clamp(0.0, 3.0) as usize
}

fn naturally_traverse_seed_zero_level_two(game: &mut Game) {
    let path = game.world.source_room_path.expect("level-two room path");
    let scan_offsets = [
        0.0, -16.0, 16.0, -32.0, 32.0, -48.0, 48.0, -64.0, 64.0, -72.0, 72.0,
    ];
    let mut previous = game.player.pos;
    let mut previous_row = source_room_row(game.player.pos.y);
    let mut stuck = 0_u8;
    let mut jump_ticks = 0_u8;
    let mut previous_jump = false;
    let mut row_stall = 0_u8;
    let mut scan_index = 0_usize;
    let mut previous_attack = false;
    let mut spider_whip_tick = None;

    // Keep this a bounded natural-input route. It reaches row two but does not
    // claim that the generated level-two exit has been reached.
    for route_tick in 0_usize..300 {
        if game.state != RunState::Playing {
            break;
        }
        let row = source_room_row(game.player.pos.y);
        if row != previous_row {
            previous_row = row;
            row_stall = 0;
            scan_index = 0;
        }
        let anchor_x = if row < 3 {
            let col = path[row]
                .iter()
                .position(|room| *room == 2)
                .expect("each level-two descent row has a marked opening");
            96.0 + col as f64 * 160.0
        } else {
            game.world.exit.x
        };
        let target_x = if row < 3 {
            anchor_x + scan_offsets[scan_index]
        } else {
            anchor_x
        };
        let dx = target_x - game.player.pos.x;
        let mut horizontal = if dx.abs() < 5.0 {
            0
        } else if dx > 0.0 {
            1
        } else {
            -1
        };
        if row < 3 && game.player.grounded && dx.abs() < 8.0 {
            row_stall = row_stall.saturating_add(1);
            if row_stall >= 12 {
                scan_index = (scan_index + 1).min(scan_offsets.len() - 1);
                row_stall = 0;
            }
        } else {
            row_stall = 0;
        }

        let no_progress = (game.player.pos.x - previous.x).abs() < 0.5
            && (game.player.pos.y - previous.y).abs() < 0.5;
        if jump_ticks == 0
            && !previous_jump
            && horizontal != 0
            && no_progress
            && (game.player.grounded || game.player.state == PlayerState::Hanging)
        {
            stuck = stuck.saturating_add(1);
        } else {
            stuck = 0;
        }
        if stuck >= 3 && jump_ticks == 0 && dx.abs() >= 8.0 {
            jump_ticks = if game.player.state == PlayerState::Hanging {
                1
            } else {
                8
            };
            stuck = 0;
        }
        let jump = jump_ticks > 0;
        let jump_pressed = jump && !previous_jump;
        let jump_released = previous_jump && !jump;
        let drop_from_hang = jump_pressed && game.player.state == PlayerState::Hanging;
        jump_ticks = jump_ticks.saturating_sub(1);

        let hanging_spider = game.world.entities.iter().find(|entity| {
            entity.alive && entity.object == "oSpiderHang" && (entity.pos.x - 576.0).abs() < 2.0
        });
        let mut hold_for_hanging_spider = false;
        let mut early_spider_attack = false;
        if let Some(spider) = hanging_spider {
            let dx = spider.pos.x - game.player.pos.x;
            let dy = game.player.pos.y - spider.pos.y;
            if spider_whip_tick.is_none()
                && row == 1
                && (512.0..=520.0).contains(&game.player.pos.x)
                && (210.0..=222.0).contains(&game.player.pos.y)
                && dx > 0.0
            {
                // Try the ordinary attack before entering the source's
                // hanging-spider drop alignment. Keep the player left of its
                // trigger through the windup, then continue below it.
                horizontal = 1;
                early_spider_attack = game.player.attack_timer == 0;
                hold_for_hanging_spider = true;
            } else if let Some(started) = spider_whip_tick
                && route_tick.saturating_sub(started) <= 16
            {
                // Let the attack animation play, then fall past the hanging
                // spider without entering its x alignment.
                horizontal = if game.player.pos.x < 544.0 { 1 } else { 0 };
                hold_for_hanging_spider = true;
            } else if row == 1 && dx < 80.0 && dy > -10.0 && dy < 36.0 {
                // Do not start a late whip while already below the hanging
                // actor; it would miss vertically and provoke its drop.
                hold_for_hanging_spider = true;
            }
        }

        let enemy_ahead = !hold_for_hanging_spider
            && horizontal != 0
            && game.world.entities.iter().any(|entity| {
                entity.alive
                    && entity.engine_active
                    && spelunky_core::catalog::is_a(&entity.object, "oEnemy")
                    && (entity.pos.x - game.player.pos.x).abs() < 64.0
                    && (entity.pos.y - game.player.pos.y).abs() < 36.0
                    && (entity.pos.x - game.player.pos.x).signum() == f64::from(horizontal)
            });
        let attack_pressed = early_spider_attack || (enemy_ahead && game.player.attack_timer == 0);
        if early_spider_attack {
            spider_whip_tick = Some(route_tick);
        }
        let near_exit = (game.player.pos.x - game.world.exit.x).abs() < 34.0
            && (game.player.pos.y - game.world.exit.y).abs() < 44.0;
        game.step(&InputFrame {
            left: horizontal < 0,
            right: horizontal > 0,
            down: drop_from_hang || (row < 3 && dx.abs() < 20.0),
            up: near_exit,
            run: true,
            jump,
            jump_pressed,
            jump_released,
            attack: attack_pressed,
            attack_pressed,
            attack_released: previous_attack && !attack_pressed,
            ..InputFrame::default()
        });
        previous_jump = jump;
        previous_attack = attack_pressed;
        previous = game.player.pos;
    }
}

#[test]
fn default_seed_zero_mines_exit_handoff_and_level_two_descent_use_game_steps() {
    let mut game = Game::new(0, RulesPreset::Classic);
    assert_eq!(
        game.generation_revision,
        GenerationRevision::SourceClassicV2
    );
    assert_eq!(game.world.area, Area::Mines);
    assert_eq!(game.world.entrance, Vec2::new(424.0, 40.0));
    assert_eq!(game.world.exit, Vec2::new(584.0, 504.0));
    assert_eq!(game.player.pos, game.world.entrance);
    assert_eq!(game.player.health, 4.0);
    assert_eq!(game.player.bombs, 4.0);
    assert_eq!(game.player.ropes, 4.0);
    assert!(game.world.entities.iter().any(|entity| {
        entity.alive && matches!(entity.object.as_str(), "oCaveman" | "oArrowTrapLeft")
    }));

    // These tile-center waypoints follow the generated open corridor from the
    // natural entrance. Every movement, jump, recovery, hazard and exit action
    // is advanced by Game::step; no world or player state is staged.
    let mut waypoint = 0;
    let mut previous_x = game.player.pos.x;
    let mut stuck_ticks = 0_u8;
    let mut jump_ticks = 0_u8;
    for _ in 0..320 {
        if waypoint == 0 && game.player.pos.y >= 150.0 {
            waypoint = 1;
        }
        if waypoint == 1 && game.player.pos.y >= 180.0 {
            waypoint = 2;
        }
        if waypoint == 2 && game.player.pos.y >= 220.0 {
            waypoint = 3;
        }
        if waypoint == 3 && game.player.pos.y >= 280.0 {
            waypoint = 4;
        }
        let target_x = match waypoint {
            0 => 264.0,
            1 => 248.0,
            2 => 168.0,
            3 => 120.0,
            _ => game.world.exit.x,
        };
        let dx = target_x - game.player.pos.x;
        let direction = if dx.abs() < 2.0 {
            0
        } else if dx > 0.0 {
            1
        } else {
            -1
        };
        let made_no_progress = direction != 0 && (game.player.pos.x - previous_x).abs() < 0.5;
        if made_no_progress
            && (game.player.grounded || game.player.state == spelunky_core::PlayerState::Hanging)
        {
            stuck_ticks = stuck_ticks.saturating_add(1);
        } else {
            stuck_ticks = 0;
        }
        if stuck_ticks >= 3 && jump_ticks == 0 {
            jump_ticks = 8;
            stuck_ticks = 0;
        }
        let jump = jump_ticks > 0;
        let jump_pressed = jump_ticks == 8;
        jump_ticks = jump_ticks.saturating_sub(1);
        let near_exit = (game.player.pos.x - game.world.exit.x).abs() < 34.0
            && (game.player.pos.y - game.world.exit.y).abs() < 44.0;
        game.step(&InputFrame {
            left: direction < 0,
            right: direction > 0,
            run: true,
            jump,
            jump_pressed,
            up: near_exit,
            ..InputFrame::default()
        });
        previous_x = game.player.pos.x;
        if game.state != RunState::Playing {
            break;
        }
    }

    assert_eq!(game.state, RunState::Ending);
    let ending = game.ending.as_ref().expect("source exit animation starts");
    assert_eq!(ending.kind, EndingKind::Transition);
    assert_eq!(ending.stage, EndingStage::Exit);
    assert!(game.player.pos.distance(game.world.exit) < 1.0);
    assert!(game.tick <= 300, "route took {} ticks", game.tick);
    assert_eq!(
        game.player.health, 3.0,
        "one natural hazard hit is survived"
    );
    assert_eq!(game.player.bombs, 4.0);
    assert_eq!(game.player.ropes, 4.0);

    // Game::step owns the exit, tally, and transition-room interactions.
    // The host's actual next-level handoff is then applied before continuing
    // from the authored level-two entrance without staging the player.
    finish_natural_level_transition(&mut game);
    assert_eq!(game.world.level, 1);
    assert_eq!(game.world.name, "rTransition1");
    game.next_level();
    assert_eq!(game.state, RunState::Playing);
    assert_eq!(game.world.level, 2);
    assert_eq!(game.world.area, Area::Mines);
    assert_eq!(game.player.pos, game.world.entrance);
    assert_eq!(game.player.health, 3.0);

    naturally_traverse_seed_zero_level_two(&mut game);
    assert!(matches!(game.state, RunState::Playing | RunState::Ending));
    assert_eq!(
        source_room_row(game.player.pos.y),
        2,
        "bounded 300-step natural route ended at {:?}",
        game.player.pos
    );
    assert_eq!(
        game.player.health, 2.0,
        "the long fall costs one life; the hanging spider does not hit"
    );
}

fn position_player(game: &mut Game, pos: Vec2) {
    game.player.pos = pos;
    game.player.previous_pos = pos;
    game.player.vel = Vec2::default();
    game.player.health = 99.0;
    game.player.invincible = 999;
    game.player.grounded = true;
    // This fixture teleports the owner between distant generated interactions.
    // Move its actual held instance too: in normal play oItem Step has already
    // kept it attached before the next room activation partition runs.
    if let Some(id) = game.player.held {
        let held = game.world.entity_mut(id).expect("held fixture instance");
        held.pos = pos;
        held.previous_pos = pos;
        held.vel = Vec2::default();
    }
    game.update_camera();
    game.previous_camera = game.camera;
}

fn positioned_exit(game: &mut Game) {
    let door = game
        .world
        .entities
        .iter()
        .find(|e| e.alive && e.object == "oExit" && e.bounds().contains(game.world.exit))
        .expect("generated normal exit at the recorded destination");
    let origin = door.pos;
    let door_id = door.id;
    let bounds = door.bounds();
    let center = Vec2::new(bounds.x + bounds.w / 2.0, bounds.y + bounds.h / 2.0);
    assert!(
        !game.world.tiles_point(origin, "oSolid") && !game.world.tiles_point(center, "oSolid"),
        "door terrain must be clear: seed {}, level {}",
        game.seed,
        game.world.level
    );
    if game.world.solid_at(origin.x, origin.y) {
        // scrEntityGen:456–471 excludes only the entrance's start room.
        // A Temple floor may therefore spawn a live Smash on its exit. This
        // positioned fixture stages a nearby approach, then uses actual input
        // to bait the trap away and enter; it never deletes the obstruction.
        let blockers: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|entity| {
                entity.alive
                    && entity.solid
                    && !entity.held
                    && spelunky_core::catalog::is_a(&entity.object, "oSolid")
                    && entity.mask_contains(origin, spelunky_core::CollisionPrecision::BoundingBox)
            })
            .collect();
        assert!(
            !blockers.is_empty()
                && blockers.iter().all(|entity| {
                    matches!(entity.object.as_str(), "oSmashTrap" | "oSmashTrapLit")
                }),
            "unexpected exit obstruction: seed {}, level {}: {:?}",
            game.seed,
            game.world.level,
            blockers
                .iter()
                .map(|entity| (&entity.object, entity.pos))
                .collect::<Vec<_>>()
        );
        let traps: Vec<_> = blockers.iter().map(|entity| entity.id).collect();
        let approach = [-32.0, 32.0, -16.0, 16.0, -48.0, 48.0]
            .into_iter()
            .map(|offset| Vec2::new(center.x + offset, center.y))
            .find(|pos| {
                !game
                    .world
                    .collides(Rect::new(pos.x - 6.0, pos.y - 8.0, 12.0, 16.0))
                    && game.world.solid_at(pos.x, pos.y + 9.0)
            })
            .expect("exit Smash fixture needs a clear supported approach");
        let from_left = approach.x < center.x;
        position_player(game, approach);
        let mut gate_cleared = false;
        for tick in 0..90 {
            let toward_door = if from_left {
                game.player.pos.x < center.x - 2.0
            } else {
                game.player.pos.x > center.x + 2.0
            };
            game.step(&InputFrame {
                left: tick >= 2 && toward_door && !from_left,
                right: tick >= 2 && toward_door && from_left,
                jump_pressed: tick == 2,
                jump: (2..10).contains(&tick),
                up: true,
                ..Default::default()
            });
            gate_cleared |= !game.world.solid_at(origin.x, origin.y);
            if game.state != RunState::Playing {
                break;
            }
        }
        // Momentum can carry the Player's exit-query point just past the
        // door's source mask after the Smash clears. Re-aim using real input
        // until the live exit action succeeds.
        for _ in 0..16 {
            if game.state != RunState::Playing {
                break;
            }
            let left = game.player.pos.x > center.x;
            let right = game.player.pos.x < center.x;
            game.step(&InputFrame {
                left,
                right,
                up: true,
                ..Default::default()
            });
            gate_cleared |= !game.world.solid_at(origin.x, origin.y);
        }
        assert!(
            gate_cleared,
            "Smash must actually vacate the source exit gate"
        );
        assert!(
            traps.iter().all(|id| game.world.entity(*id).is_some()),
            "the positioned interaction must preserve its generated Smash"
        );
        assert_eq!(
            game.state,
            RunState::Ending,
            "bait and enter: seed {}, level {}, player {:?}, vel {:?}, grounded {}, stun {}, attack_timer {}, door {} at {:?}, center {:?}, approach {:?}, gate_cleared {}, door_live {}",
            game.seed,
            game.world.level,
            game.player.pos,
            game.player.vel,
            game.player.grounded,
            game.player.stun,
            game.player.attack_timer,
            door_id,
            origin,
            center,
            approach,
            gate_cleared,
            game.world.entity(door_id).is_some()
        );
    } else {
        position_player(game, center);
        // Teleporting does not establish the source ON_GROUND controller state.
        // Let the live controller settle before requesting the actual exit.
        game.step(&InputFrame::default());
        assert_eq!(game.state, RunState::Playing);
        game.step(&InputFrame {
            up: true,
            ..Default::default()
        });
    }
    complete_source_exit(game);
}

#[test]
fn positioned_generated_doors_transition_through_all_four_campaign_areas() {
    for seed in [1, 42, 1337, 9999] {
        let mut game = Game::new(seed, RulesPreset::Classic);
        for level in 1..=15 {
            assert_eq!(game.world.level, level);
            assert_eq!(game.world.area, Area::for_level(level));
            positioned_exit(&mut game);
            game.next_level();
            assert_eq!(game.state, RunState::Playing);
            assert_eq!(game.world.level, level + 1);
            assert_eq!(game.world.area, Area::for_level(level + 1));
        }
    }
}

#[test]
fn generated_key_opens_generated_locked_chest_through_game_step() {
    let mut game = Game::new(42, RulesPreset::Classic);
    while !game
        .world
        .entities
        .iter()
        .any(|e| e.object == "oLockedChest")
    {
        assert!(
            game.world.level < 4,
            "scheduled Udjat chest must appear by level four"
        );
        positioned_exit(&mut game);
        game.next_level();
    }
    assert!(
        game.flags.contains("made_udjat_eye"),
        "source progression flag is set when the generated key exists"
    );
    assert!(
        game.world
            .entities
            .iter()
            .any(|e| e.alive && e.object == "oKey")
    );
    let key = game
        .world
        .entities
        .iter()
        .find(|e| e.alive && e.object == "oKey")
        .unwrap()
        .id;
    let chest = game
        .world
        .entities
        .iter()
        .find(|e| e.object == "oLockedChest")
        .unwrap();
    let chest_id = chest.id;
    let chest_pos = chest.pos;
    // Use the actual generated key; isolate the chest interaction from pickup.
    game.player.held = Some(key);
    game.world.entity_mut(key).unwrap().held = true;
    position_player(&mut game, chest_pos);
    game.step(&InputFrame::default());
    assert!(game.world.entity(key).is_none());
    assert!(game.world.entity(chest_id).is_none());
    let eye_pos = game
        .world
        .entities
        .iter()
        .find(|e| e.alive && e.object == "oUdjatEye")
        .unwrap()
        .pos;
    position_player(&mut game, eye_pos);
    game.step(&InputFrame {
        down: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert!(game.player.has("UdjatEye"));
}

#[test]
fn actual_pickups_and_walking_open_gold_door_without_entering_its_solid() {
    for crown in [false, true] {
        let mut level = spelunky_core::level::Level::default();
        level.set_tile(6, 7, '@').unwrap();
        for x in 0..40 {
            level.set_tile(x, 8, '1').unwrap();
        }
        let mut game = Game::shell(42, RulesPreset::Classic);
        game.load_custom(&level, "gold-door-approach", "");
        for _ in 0..8 {
            game.step(&InputFrame::default());
        }
        for object in if crown {
            vec!["oCrown", "oSceptre"]
        } else {
            vec!["oSceptre"]
        } {
            let id = game.spawn(object, Vec2::new(game.player.pos.x, game.player.pos.y + 4.));
            let item = game.world.entity_mut(id).unwrap();
            item.data.insert("cost".into(), 0.);
            item.data.insert("forSale".into(), 0.);
            game.step(&InputFrame {
                down: true,
                attack: true,
                attack_pressed: true,
                ..Default::default()
            });
            game.step(&InputFrame::default());
        }
        assert_eq!(game.player.has("Crown"), crown);
        let sceptre = game.player.held.expect("actual pickup holds sceptre");
        assert_eq!(game.world.entity(sceptre).unwrap().object, "oSceptre");
        let p = game.player.pos;
        let door = game.spawn("oGoldDoor", Vec2::new(p.x + 32., p.y - 8.));
        for _ in 0..20 {
            game.step(&InputFrame {
                right: true,
                ..Default::default()
            });
            assert_eq!(game.state, RunState::Playing);
            assert_eq!(game.player.health, 4.0);
            if game.world.entity(door).is_none() {
                break;
            }
        }
        assert_eq!(game.world.entity(door).is_none(), crown);
        assert_eq!(
            game.world.entities.iter().any(|e| e.object == "oXGold"),
            crown
        );
        assert_eq!(game.player.held.is_none(), crown);
        assert_eq!(game.world.entity(sceptre).is_none(), crown);
    }
}

fn generated_market_ankh_moai_chain_to_temple(seed: u32) -> Game {
    let mut game = Game::new(seed, RulesPreset::Classic);
    while !game.world.entities.iter().any(|e| e.object == "oXMarket") {
        assert!(
            game.world.level < 7,
            "market entrance scheduled by level seven"
        );
        positioned_exit(&mut game);
        game.next_level();
    }
    let market = game
        .world
        .entities
        .iter()
        .find(|e| e.object == "oXMarket")
        .unwrap();
    let market_pos = market.pos;
    // Isolate entrance use after discovery: remove its single covering tile,
    // preserving the generated supporting floor. This does not test excavation.
    game.world.set_tile(
        (market_pos.x / 16.0) as usize,
        (market_pos.y / 16.0) as usize,
        "",
    );
    position_player(&mut game, Vec2::new(market_pos.x + 8.0, market_pos.y + 8.0));
    game.step(&InputFrame {
        up: true,
        ..Default::default()
    });
    complete_source_exit(&mut game);
    game.next_level();
    assert_eq!(game.world.name, "Black Market");

    let ankh = game
        .world
        .entities
        .iter()
        .find(|e| e.object == "oAnkh")
        .unwrap();
    let ankh_id = ankh.id;
    let ankh_pos = ankh.pos;
    assert_eq!(ankh.value("cost"), 50000.0);
    // Classic tile A only creates oAnkh; its inherited Create leaves
    // forSale=false. Positive cost still makes the Player pay branch purchase it.
    assert_eq!(ankh.value("forSale"), 0.0);

    // Drop it into an adjacent non-shop room. A source-faithful Ankh is not
    // stock for oItem's outside-shop theft check, so its price remains intact.
    let outside_shop = Vec2::new(552.0, 200.0); // Clear air in Black Market path[1][3] == 2.
    assert_eq!(game.world.source_room_path.unwrap()[1][3], 2);
    position_player(&mut game, outside_shop);
    let ankh = game.world.entity_mut(ankh_id).unwrap();
    ankh.pos = outside_shop;
    ankh.previous_pos = outside_shop;
    game.step(&InputFrame::default());
    assert_eq!(game.world.entity(ankh_id).unwrap().value("cost"), 50000.0);
    assert_eq!(game.world.entity(ankh_id).unwrap().value("forSale"), 0.0);
    assert_eq!(game.lifetime_stats.items_stolen, 0);

    position_player(&mut game, ankh_pos);
    let ankh = game.world.entity_mut(ankh_id).unwrap();
    ankh.pos = ankh_pos;
    ankh.previous_pos = ankh_pos;
    ankh.held = true;
    game.player.held = Some(ankh_id);
    game.player.money = 1_000_000; // Purchase affordability is not this scenario's target.
    game.step(&InputFrame {
        pay_pressed: true,
        ..Default::default()
    });
    assert!(game.player.has("Ankh"));
    assert!(game.world.entity(ankh_id).is_none());
    assert_eq!(game.player.money, 950_000);

    while !game
        .world
        .entities
        .iter()
        .any(|e| e.object == "oMoaiInside")
    {
        assert!(game.world.level < 12, "Moai scheduled by level twelve");
        positioned_exit(&mut game);
        game.next_level();
    }
    let moai = game
        .world
        .entities
        .iter()
        .find(|e| e.object == "oMoaiInside")
        .unwrap();
    let moai_id = moai.id;
    let revival_pos = Vec2::new(moai.pos.x + 8.0, moai.pos.y + 8.0);
    // Source revival checks instance_exists(oMoai), which only sees active
    // instances. Stage the death beside the discovered Moai, rather than back
    // at a distant entrance whose activation region excludes the monument.
    position_player(&mut game, Vec2::new(revival_pos.x, revival_pos.y - 32.0));
    game.player.health = 0.0;
    game.step(&InputFrame::default());
    assert_eq!(game.state, RunState::Playing);
    assert_eq!(game.player.pos, revival_pos);
    assert!(!game.player.has("Ankh"));
    assert!(game.world.entity(moai_id).is_none());
    let crown_pos = game
        .world
        .entities
        .iter()
        .find(|e| e.object == "oCrown")
        .unwrap()
        .pos;
    position_player(&mut game, crown_pos);
    game.step(&InputFrame {
        down: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert!(game.player.has("Crown"));

    while game.world.level < 13 {
        positioned_exit(&mut game);
        game.next_level();
    }
    game
}

#[test]
fn exposed_generated_market_ankh_moai_and_gold_door_form_a_reachable_state_chain() {
    // scrEntityGen:385 enables Tomb Lord placement on thirteen, but :458–464
    // still requires eligible support and rand(1,40). Try a known natural
    // fixture first, then a bounded seed fallback if generation rolls change.
    // Select only this prerequisite after the complete asserted prefix; never
    // catch an interaction failure or filter on the downstream chain result.
    let mut game = [2, 0, 1, 3, 4, 5, 6, 7]
        .into_iter()
        .map(generated_market_ankh_moai_chain_to_temple)
        .find(|game| {
            game.world
                .entities
                .iter()
                .any(|entity| entity.alive && entity.object == "oTombLord")
        })
        .expect("bounded natural chain fixtures must supply a level-thirteen Tomb Lord");
    let tomb = game
        .world
        .entities
        .iter()
        .find(|e| e.object == "oTombLord")
        .expect("selected chain seed must generate a Tomb Lord on level thirteen")
        .id;
    // Isolate the generated boss's death/drop transition from combat execution.
    let tomb_pos = game.world.entity(tomb).unwrap().pos;
    // Choose a nearby clear player pose instead of assuming the boss's center
    // is clear. Source crushing ignores the fixture's invincibility timer.
    let player_bounds = game.player.bounds();
    let clear_pose = |position: Vec2| {
        let bounds = Rect::new(
            position.x + player_bounds.x - game.player.pos.x,
            position.y + player_bounds.y - game.player.pos.y,
            player_bounds.w,
            player_bounds.h,
        );
        !game.world.tiles_collide(bounds, "oSolid")
            && !game.world.entities.iter().any(|entity| {
                entity.alive
                    && spelunky_core::catalog::is_a(&entity.object, "oSolid")
                    && entity.sprite_bounds().intersects(bounds)
            })
    };
    let staging = [
        Vec2::new(tomb_pos.x + 16.0, tomb_pos.y + 16.0),
        Vec2::new(tomb_pos.x - 8.0, tomb_pos.y + 16.0),
        Vec2::new(tomb_pos.x + 16.0, tomb_pos.y - 8.0),
        Vec2::new(tomb_pos.x + 40.0, tomb_pos.y + 16.0),
    ]
    .into_iter()
    .find(|position| clear_pose(*position))
    .expect("generated boss needs a nearby clear interaction pose");
    assert!(clear_pose(staging));
    position_player(&mut game, staging);
    assert!(!game.world.solid_at(game.player.pos.x, game.player.pos.y));
    game.world.entity_mut(tomb).unwrap().health = 0;
    game.step(&InputFrame::default());
    assert_eq!(game.state, RunState::Playing);
    assert!(game.player.health > 0.0);
    let sceptre_id = game
        .world
        .entities
        .iter()
        .find(|e| e.alive && e.object == "oSceptre")
        .unwrap()
        .id;
    game.player.held = Some(sceptre_id);
    game.world.entity_mut(sceptre_id).unwrap().held = true;
    positioned_exit(&mut game);
    game.next_level();
    assert_eq!(game.world.level, 14);
    let door = game
        .world
        .entities
        .iter()
        .find(|e| e.object == "oGoldDoor")
        .unwrap();
    let door_id = door.id;
    // GoldDoor is nonsolid; generation guarantees clearance above its support,
    // not on its left. Center the controller on the actual door mask and align
    // its bottom with the mask's bottom, then qualify the full interaction pose.
    let door_bounds = door.sprite_bounds();
    let player_bounds = game.player.bounds();
    let staging = Vec2::new(
        door_bounds.x + door_bounds.w / 2.0
            - (player_bounds.x + player_bounds.w / 2.0 - game.player.pos.x),
        door_bounds.y + door_bounds.h - (player_bounds.y + player_bounds.h - game.player.pos.y),
    );
    let staging_bounds = Rect::new(
        staging.x + player_bounds.x - game.player.pos.x,
        staging.y + player_bounds.y - game.player.pos.y,
        player_bounds.w,
        player_bounds.h,
    );
    assert!(door_bounds.intersects(staging_bounds));
    assert!(!game.world.tiles_collide(staging_bounds, "oSolid"));
    assert!(!game.world.entities.iter().any(|entity| {
        entity.alive
            && spelunky_core::catalog::is_a(&entity.object, "oSolid")
            && entity.sprite_bounds().intersects(staging_bounds)
    }));
    position_player(&mut game, staging);
    assert!(!game.world.solid_at(game.player.pos.x, game.player.pos.y));
    for _ in 0..8 {
        game.step(&InputFrame {
            right: true,
            ..Default::default()
        });
        assert_eq!(game.state, RunState::Playing);
        assert!(game.player.health > 0.0);
        if game.world.entity(door_id).is_none() {
            break;
        }
    }
    assert!(game.world.entity(door_id).is_none());
    assert!(game.player.held.is_none());
    let gold_door = game
        .world
        .entities
        .iter()
        .find(|e| e.alive && e.object == "oXGold")
        .unwrap();
    let gold_pos = gold_door.pos;
    position_player(&mut game, Vec2::new(gold_pos.x + 8.0, gold_pos.y + 8.0));
    game.step(&InputFrame::default());
    assert_eq!(game.state, RunState::Playing);
    game.step(&InputFrame {
        up: true,
        ..Default::default()
    });
    complete_source_exit(&mut game);
    game.next_level();
    assert_eq!(game.world.name, "City of Gold");
    assert_eq!(game.world.level, 15);
}
