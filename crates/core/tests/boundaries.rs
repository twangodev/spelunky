//! Controller acceptance against the actual generated perimeter. Interior
//! hazards are removed to isolate movement; the perimeter remains untouched.
use spelunky_core::{Game, InputFrame, PlayerState, RulesPreset, RunState, Vec2, generation};

fn perimeter_fixture(level: u32, right: bool) -> Game {
    let mut game = Game::new(42, RulesPreset::Classic);
    if level != 1 {
        game.world = generation::generate(&mut game.rng, level, &game.flags);
    }
    for y in 1..game.world.height - 1 {
        for x in 1..game.world.width - 1 {
            game.world.set_tile(x, y, "");
        }
    }
    game.world.entities.retain(|entity| {
        entity.object == "oTerrainMetadata"
            || (entity.solid && (entity.pos.x == 0.0 || entity.pos.x == 656.0))
    });
    for x in 1..game.world.width - 1 {
        game.world.set_tile(x, 8, "oHardBlock");
    }
    game.player.pos = Vec2::new(if right { 624.0 } else { 48.0 }, 120.0);
    game.player.previous_pos = game.player.pos;
    game.player.vel = Vec2::default();
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.player.health = 99;
    game.player.invincible = 999;
    game
}

#[test]
fn generated_top_corners_stop_jumps_without_opening_the_side_wall() {
    for level in [1, 5, 9, 13, 16] {
        for right in [false, true] {
            let mut game = perimeter_fixture(level, right);
            for x in 1..game.world.width - 1 {
                game.world.set_tile(x, 3, "oHardBlock");
            }
            game.player.pos = Vec2::new(if right { 640.0 } else { 32.0 }, 40.0);
            for tick in 0..40 {
                game.step(&InputFrame {
                    left: !right,
                    right,
                    run: true,
                    jump: true,
                    jump_pressed: tick == 0,
                    ..Default::default()
                });
                assert!(
                    game.player.pos.y >= 24.0,
                    "level {level}, tick {tick}: {:?}",
                    game.player.pos
                );
                assert!((21.0..=651.0).contains(&game.player.pos.x));
            }
        }
    }
}

#[test]
fn ice_side_walls_continue_to_source_last_row_while_bottom_remains_an_abyss() {
    for right in [false, true] {
        let mut game = perimeter_fixture(9, right);
        game.player.pos = Vec2::new(if right { 648.0 } else { 24.0 }, 616.0);
        game.player.vel = Vec2::new(if right { 10.0 } else { -10.0 }, 0.0);
        game.player.stun = 20;
        game.step(&InputFrame::default());
        assert!((21.0..=651.0).contains(&game.player.pos.x));
        assert_eq!(game.state, RunState::Playing);
    }
    let mut game = perimeter_fixture(9, false);
    game.player.pos = Vec2::new(336.0, 600.0);
    game.player.vel = Vec2::new(0.0, 10.0);
    game.player.state = PlayerState::Falling;
    game.player.grounded = false;
    // rLevel2 is 608px high. oPlayer1 Step checks y > room_height + 16
    // before movement, so crossing that line remains alive until the next Step.
    let death_line = game.world.height as f64 * 16.0 + 16.0;
    for _ in 0..8 {
        game.step(&InputFrame::default());
        assert_eq!(game.state, RunState::Playing);
        if game.player.pos.y > death_line {
            break;
        }
    }
    assert!(
        game.player.pos.y > death_line,
        "the open bottom must be crossed"
    );
    game.step(&InputFrame::default());
    assert_eq!(game.state, RunState::Dead);
    assert_eq!(game.lifetime_stats.misc_deaths[10], 1);
}

#[test]
fn ordinary_lower_corners_stop_stunned_knockback_against_both_surfaces() {
    for level in [1, 5, 13] {
        for right in [false, true] {
            let mut game = perimeter_fixture(level, right);
            let floor = game.world.height as f64 * 16.0 - 16.0;
            game.player.pos = Vec2::new(if right { 648.0 } else { 24.0 }, floor - 8.0);
            game.player.vel = Vec2::new(if right { 10.0 } else { -10.0 }, 10.0);
            game.player.stun = 30;
            for tick in 0..40 {
                game.step(&InputFrame::default());
                assert!((21.0..=651.0).contains(&game.player.pos.x));
                assert!(
                    game.player.pos.y <= floor - 8.0,
                    "level {level}, tick {tick}: {:?}",
                    game.player.pos
                );
                assert_eq!(game.state, RunState::Playing);
            }
        }
    }
}

#[test]
fn explosions_preserve_concrete_generated_perimeter_tiles() {
    for level in [1, 5, 9, 13, 16] {
        for right in [false, true] {
            let mut game = perimeter_fixture(level, right);
            let x = if right { 41 } else { 0 };
            let original = game.world.tile(x, 6).unwrap().object.clone();
            assert_ne!(original, "oSolid");
            let wall_bounds = game.world.tile_bounds(x, 6);
            // A neighboring destructible block proves the actual blast reaches
            // this height; perimeter survival alone could pass with a missed blast.
            let inner_x = if right { 40 } else { 1 };
            let inner_center = Vec2::new(inner_x as f64 * 16.0 + 8.0, 104.0);
            game.world
                .entities
                .retain(|entity| entity.object != "oTerrainMetadata" || entity.pos != inner_center);
            game.world.set_tile(inner_x, 6, "oBrick");
            game.player.pos.x = if right { 648.0 } else { 24.0 };
            game.player.facing = if right { 1 } else { -1 };
            let bombs_before = game.player.bombs;
            game.step(&InputFrame {
                down: true,
                bomb_pressed: true,
                ..Default::default()
            });
            assert_eq!(game.player.bombs, bombs_before - 1);
            let bomb = game
                .world
                .entities
                .iter()
                .find(|e| e.object == "oBomb")
                .unwrap();
            assert_eq!(bomb.alarms[0], 80);
            let bomb_id = bomb.id;
            // Dedicated Bomb arms source Alarm 0 (80), which schedules Alarm 1
            // (40). The generic Entity timer is not the runtime fuse.
            let mut blast_reached_wall = false;
            for _ in 0..134 {
                game.step(&InputFrame::default());
                blast_reached_wall |= game.world.entities.iter().any(|entity| {
                    entity.object == "oExplosion" && entity.sprite_bounds().intersects(wall_bounds)
                });
                assert_eq!(game.world.tile(x, 6).unwrap().object, original);
            }
            assert!(
                blast_reached_wall,
                "level {level}, right {right}: the blast must reach the wall"
            );
            assert!(game.world.entity(bomb_id).is_none());
            assert!(
                game.world
                    .tile(inner_x as i32, 6)
                    .unwrap()
                    .object
                    .is_empty()
            );
        }
    }
}

#[test]
fn generated_side_walls_stop_running_jumps_in_every_campaign_area() {
    for level in [1, 5, 9, 13, 16] {
        for right in [false, true] {
            let mut game = perimeter_fixture(level, right);
            for tick in 0..90 {
                game.step(&InputFrame {
                    left: !right,
                    right,
                    run: true,
                    jump: tick % 30 < 12,
                    jump_pressed: tick % 30 == 0,
                    jump_released: tick % 30 == 12,
                    ..Default::default()
                });
                assert!(
                    (21.0..=651.0).contains(&game.player.pos.x),
                    "level {level}, right {right}, tick {tick}: {:?}",
                    game.player.pos
                );
            }
            assert!(
                if right {
                    game.player.pos.x > 640.0
                } else {
                    game.player.pos.x < 32.0
                },
                "the player must actually reach the wall"
            );
        }
    }
}

#[test]
fn generated_side_walls_stop_stunned_knockback_in_every_campaign_area() {
    for level in [1, 5, 9, 13, 16] {
        for right in [false, true] {
            let mut game = perimeter_fixture(level, right);
            game.player.pos.x = if right { 648.0 } else { 24.0 };
            game.player.vel = Vec2::new(if right { 10.0 } else { -10.0 }, -5.0);
            game.player.stun = 30;
            for tick in 0..40 {
                game.step(&InputFrame::default());
                assert!(
                    (21.0..=651.0).contains(&game.player.pos.x),
                    "level {level}, right {right}, tick {tick}: {:?}",
                    game.player.pos
                );
            }
        }
    }
}
