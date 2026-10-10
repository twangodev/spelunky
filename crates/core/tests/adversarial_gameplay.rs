//! Bounded, deterministic full-scheduler audit. These scenarios retain hazards,
//! ordinary life and collision; room loading is an explicit initial condition.
use spelunky_core::{
    Game, InputFrame, PlayerState, RulesPreset, RunState, Vec2, hub::HubKind, level::Level,
};

fn assert_finite(game: &Game, context: &str) {
    for (label, point) in [
        ("player", game.player.pos),
        ("previous player", game.player.previous_pos),
        ("velocity", game.player.vel),
        ("camera", game.camera),
    ] {
        assert!(
            point.x.is_finite() && point.y.is_finite(),
            "{context}: {label}: {point:?}"
        );
    }
    for entity in &game.world.entities {
        assert!(
            entity.pos.x.is_finite() && entity.pos.y.is_finite(),
            "{context}: {} at {:?}",
            entity.object,
            entity.pos
        );
    }
}

fn sequence(tick: u64, seed: u64) -> InputFrame {
    // Held runs, reversals, rapid release/repress, simultaneous directions and
    // pickup/attack while changing posture exercise more than fresh presses.
    let phase = (tick + seed * 13) % 180;
    let jump = tick % 17 < 9;
    let previous_jump = tick > 0 && (tick - 1) % 17 < 9;
    InputFrame {
        left: phase < 60 || (120..125).contains(&phase),
        right: phase >= 60 && phase < 125,
        up: tick % 89 < 12,
        down: tick % 43 < 7,
        run: tick % 120 < 100,
        jump,
        jump_pressed: jump && !previous_jump,
        jump_released: !jump && previous_jump,
        attack: tick % 31 < 5,
        attack_pressed: tick % 31 == 0,
        bomb_pressed: tick % 151 == 50,
        rope_pressed: tick % 173 == 100,
        cycle_pressed: tick % 127 == 70,
        ..Default::default()
    }
}

#[test]
fn bounded_mixed_inputs_keep_generated_and_tutorial_geometry_finite() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for seed in 0..4 {
            for level in [1, 5, 9, 13] {
                let mut game = Game::new(seed, rules);
                // Deliberate room-entry fixture, not a claim of traversing the
                // preceding floors. Every simulated tick retains live hazards.
                while game.world.level < level {
                    game.next_level();
                }
                for tick in 0..240 {
                    game.step(&sequence(tick, seed));
                    assert_finite(
                        &game,
                        &format!("seed={seed} level={level} tick={tick} rules={rules:?}"),
                    );
                    assert!(
                        game.player.pos.x >= 0.0
                            && game.player.pos.x < game.world.width as f64 * 16.0,
                        "horizontal room escape: seed={seed} level={level} tick={tick} rules={rules:?} pos={:?} state={:?}",
                        game.player.pos,
                        game.player.state
                    );
                    if game.state != RunState::Playing {
                        break;
                    }
                }
            }
            let mut game = Game::tutorial(seed, rules);
            for tick in 0..240 {
                game.step(&sequence(tick, seed));
                assert_finite(&game, &format!("tutorial seed={seed} tick={tick}"));
                if game.state != RunState::Playing {
                    break;
                }
            }
        }
    }
}

#[test]
fn authored_hub_inputs_do_not_leave_nonfinite_state() {
    for kind in [
        HubKind::Title,
        HubKind::Scores,
        HubKind::Sun,
        HubKind::Moon,
        HubKind::Stars,
    ] {
        let mut game = Game::shell(17, RulesPreset::Classic);
        game.load_hub(kind);
        for tick in 0..600 {
            game.step(&sequence(tick, 17));
            assert_finite(&game, &format!("hub={kind:?} tick={tick}"));
            if game.state != RunState::Playing {
                break;
            }
        }
    }
}

fn flat_custom(floor: char) -> Game {
    let mut level = Level::default();
    level.set_tile(6, 7, '@').unwrap();
    level.set_tile(34, 7, 'X').unwrap();
    for x in 0..40 {
        level.set_tile(x, 8, floor).unwrap();
    }
    let mut game = Game::shell(11, RulesPreset::Classic);
    game.load_custom(&level, "audit", "");
    for _ in 0..8 {
        game.step(&InputFrame::default());
    }
    assert!(game.player.grounded);
    game
}

#[test]
fn real_arrow_hit_recovers_then_accepts_movement_and_jump_on_stone_and_ice() {
    for floor in ['1', 'i'] {
        for character in ["explorer", "character_damsel", "character_tunnelman"] {
            let mut game = flat_custom(floor);
            if character != "explorer" {
                game.flags.insert(character.into());
            }
            // A live projectile is the only stun trigger. No direct player
            // state/counter edits, invincibility grant or post-hit reposition.
            let arrow = game.world.spawn(
                "oArrow",
                Vec2::new(game.player.pos.x - 18.0, game.player.pos.y),
            );
            game.world.entity_mut(arrow).unwrap().vel.x = 8.0;
            let mut hit = false;
            let mut recovered = false;
            for tick in 0..240 {
                game.step(&InputFrame::default());
                hit |= game.player.stun > 0;
                assert_finite(
                    &game,
                    &format!("arrow floor={floor} character={character} tick={tick}"),
                );
                assert_eq!(
                    game.state,
                    RunState::Playing,
                    "floor={floor} character={character} tick={tick}"
                );
                if hit && game.player.stun == 0 && game.player.grounded {
                    recovered = true;
                    break;
                }
            }
            assert!(
                hit,
                "arrow must actually hit floor={floor} character={character}"
            );
            assert!(
                recovered,
                "floor={floor} character={character}: {:?} sprite={} stun={} pos={:?} vel={:?}",
                game.player.state,
                game.player.sprite,
                game.player.stun,
                game.player.pos,
                game.player.vel
            );
            let recovered_x = game.player.pos.x;
            for _ in 0..12 {
                game.step(&InputFrame {
                    left: true,
                    ..Default::default()
                });
            }
            assert!(
                game.player.pos.x < recovered_x - 4.0,
                "movement remains disabled after recovery"
            );
            let ground_y = game.player.pos.y;
            game.step(&InputFrame {
                jump: true,
                jump_pressed: true,
                ..Default::default()
            });
            assert!(
                game.player.pos.y < ground_y,
                "jump remains disabled after recovery: {:?}",
                game.player.state
            );
            assert_ne!(game.player.state, PlayerState::Stunned);
        }
    }
}

#[test]
fn room_reload_discards_real_hit_stun_and_resumes_live_controller() {
    let mut game = flat_custom('1');
    let arrow = game.world.spawn(
        "oArrow",
        Vec2::new(game.player.pos.x - 18.0, game.player.pos.y),
    );
    game.world.entity_mut(arrow).unwrap().vel.x = 8.0;
    for _ in 0..4 {
        game.step(&InputFrame::default());
        if game.player.stun > 0 {
            break;
        }
    }
    assert!(game.player.stun > 0);
    game.load_hub(HubKind::Title);
    assert_eq!(game.player.stun, 0);
    for tick in 0..30 {
        game.step(&sequence(tick, 4));
    }
    assert_eq!(game.state, RunState::Playing);
    assert_ne!(game.player.state, PlayerState::Stunned);
    assert_finite(&game, "post-hit title reload");
}

#[test]
fn natural_custom_run_jump_sequences_cannot_cross_authored_side_walls() {
    for direction in [-1, 1] {
        let mut game = flat_custom('1');
        for tick in 0..320 {
            let jump = tick % 24 < 12;
            game.step(&InputFrame {
                left: direction < 0,
                right: direction > 0,
                run: true,
                jump,
                jump_pressed: tick % 24 == 0,
                jump_released: tick % 24 == 12,
                ..Default::default()
            });
            assert_eq!(game.state, RunState::Playing);
            assert!(
                game.player.pos.x >= 16.0
                    && game.player.pos.x < (game.world.width - 1) as f64 * 16.0,
                "custom direction={direction} tick={tick}: {:?}",
                game.player.pos
            );
            assert_finite(&game, "custom side-wall run/jump");
        }
    }
}

#[test]
fn natural_fall_damage_recovers_without_repositioning_or_counter_edits() {
    let mut level = Level::default();
    level.set_tile(8, 1, '@').unwrap();
    for x in 0..40 {
        level.set_tile(x, 13, '1').unwrap();
    }
    let mut game = Game::shell(23, RulesPreset::Classic);
    game.load_custom(&level, "fall-recovery", "");
    let mut hit = false;
    let mut recovered = false;
    for tick in 0..240 {
        game.step(&InputFrame::default());
        assert_eq!(
            game.state,
            RunState::Playing,
            "fall fixture died tick={tick}: health={} pos={:?}",
            game.player.health,
            game.player.pos
        );
        assert_finite(&game, "natural fall injury");
        hit |= game.player.stun > 0;
        if hit && game.player.stun == 0 && game.player.grounded {
            recovered = true;
            break;
        }
    }
    assert!(hit, "drop must actually trigger source fall injury");
    assert!(
        recovered,
        "fall stun remains: sprite={} stun={} pos={:?} vel={:?}",
        game.player.sprite, game.player.stun, game.player.pos, game.player.vel
    );
    let ground_y = game.player.pos.y;
    game.step(&InputFrame {
        jump: true,
        jump_pressed: true,
        ..Default::default()
    });
    assert!(game.player.pos.y < ground_y);
}

#[test]
fn live_arrow_recovers_in_a_one_character_high_tunnel() {
    let mut game = flat_custom('1');
    // Preserve the loaded fixture's source-aligned floor; add a real ceiling
    // exactly one standing character above it, not an overlap at spawn.
    for x in 1..game.world.width - 1 {
        game.world.set_tile(x, 7, "oBrick");
    }
    assert!(!game.world.collides(game.player.bounds()));
    let arrow = game.world.spawn(
        "oArrow",
        Vec2::new(game.player.pos.x - 18.0, game.player.pos.y),
    );
    game.world.entity_mut(arrow).unwrap().vel.x = 8.0;
    let mut hit = false;
    let mut recovered = false;
    for tick in 0..240 {
        game.step(&InputFrame::default());
        hit |= game.player.stun > 0;
        assert_eq!(game.state, RunState::Playing, "tunnel died tick={tick}");
        if hit && game.player.stun == 0 && game.player.grounded {
            recovered = true;
            break;
        }
    }
    assert!(hit, "tunnel projectile must hit");
    assert!(
        recovered,
        "tunnel stun remains: sprite={} stun={} pos={:?} vel={:?}",
        game.player.sprite, game.player.stun, game.player.pos, game.player.vel
    );
}

#[test]
fn real_hit_releases_a_naturally_picked_up_rock_with_source_knockback() {
    let mut game = flat_custom('1');
    let rock = game.world.spawn(
        "oRock",
        Vec2::new(game.player.pos.x, game.player.pos.y + 4.0),
    );
    game.step(&InputFrame {
        down: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert_eq!(
        game.player.held,
        Some(rock),
        "fixture must actually pick up the rock"
    );
    let arrow = game.world.spawn(
        "oArrow",
        Vec2::new(game.player.pos.x - 18.0, game.player.pos.y),
    );
    game.world.entity_mut(arrow).unwrap().vel.x = 8.0;
    let mut hit = false;
    for _ in 0..8 {
        game.step(&InputFrame::default());
        hit |= game.player.stun > 0;
        if hit {
            break;
        }
    }
    assert!(hit, "fixture must actually hit the player");
    assert_eq!(
        game.player.held, None,
        "new projectile stun releases the held item during the same source Step event"
    );
    let released = game
        .world
        .entity(rock)
        .expect("released rock remains alive");
    assert!(!released.held);
    // oPlayer1 Step:1784-1805 assigns xVel=player.xVel, yVel=-6 and
    // armed=true. Ordinary item gravity may already advance during this step.
    assert!(
        released.vel.y < -4.0,
        "released rock should be thrown upward, not frozen: {:?}",
        released.vel
    );
    assert!(
        released.vel.x > 0.0,
        "released rock should inherit the hit's horizontal knockback"
    );
    assert!(released.value("armed") > 0.0);
}
