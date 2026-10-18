//! Literal late Player hurt release from actual glyph Bow/trap constructors.
//! The supported initial protagonist pose is controlled before any ticks.
//! After that: pickup, charge/release, cycle and Jump inputs only; no state writes.
use spelunky_core::{
    Game, GameEvent, InputFrame, PlayerState, RulesPreset, RunState, Vec2, level::Level,
};

const PRESETS: [RulesPreset; 2] = [RulesPreset::Classic, RulesPreset::Assisted];

fn diagnostic(game: &Game) -> String {
    format!(
        "tick={} pos={:?} vel={:?} hp={} stun={} held={:?} type={} armed={:?} arrows={:?}",
        game.tick,
        game.player.pos,
        game.player.vel,
        game.player.health,
        game.player.stun,
        game.player.held,
        game.player.pickup_item_type,
        game.player.data.get("bow_armed"),
        game.data.get("arrows")
    )
}

fn sound(game: &Game, name: &str) -> usize {
    game.events
        .iter()
        .filter(|event| matches!(event, GameEvent::Sound { name: actual, .. } if actual == name))
        .count()
}

fn room(rules: RulesPreset, facing: i8, trap: bool) -> Game {
    let mut level = Level::default();
    for x in 0..40 {
        level.set_tile(x, 9, '1').unwrap();
    }
    level.set_tile(14, 8, '@').unwrap();
    level.set_tile(15, 8, '}').unwrap();
    if trap {
        // Trap behind the facing direction keeps the source ±14 shot muzzle
        // clear even when the hurt occurs above the floor during Jump.
        level
            .set_tile(
                if facing < 0 { 16 } else { 14 },
                7,
                if facing < 0 { '<' } else { '>' },
            )
            .unwrap();
    }
    let mut game = Game::shell(19, rules);
    game.load_custom(&level, "late-hurt-bow-source", "");
    // Explicit authored initial pose, not entrance-traversal coverage. Actor
    // fields, source glyph price and inventory remain constructor-produced.
    game.player.pos = Vec2::new(264.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.player.facing = facing;
    game.world.entrance = game.player.pos;
    game
}

fn pickup(game: &mut Game) -> u64 {
    let bow = game
        .world
        .entities
        .iter()
        .find(|item| item.alive && item.object == "oBow")
        .expect("actual } custom-loader Bow")
        .id;
    let target = game.world.entity(bow).unwrap().pos.x - 6.0;
    for _ in 0..40 {
        if game.player.pos.x >= target {
            break;
        }
        game.step(&InputFrame {
            right: true,
            ..Default::default()
        });
        assert_eq!(
            game.player.health,
            4.0,
            "pickup precedes trap range: {}",
            diagnostic(game)
        );
    }
    assert!(
        game.player.pos.x >= target,
        "FIXTURE_SUPPORTED_PICKUP_POSITION: {}",
        diagnostic(game)
    );
    assert_eq!(game.world.entity(bow).unwrap().value("cost"), 0.0);
    assert_eq!(game.world.entity(bow).unwrap().pos, Vec2::new(264.0, 156.0));
    assert_eq!(game.player.pos, Vec2::new(264.0, 152.0));
    assert_eq!(game.player.state, PlayerState::Standing);
    assert!(game.player.grounded);
    assert_eq!(game.player.vel, Vec2::default());
    game.step(&InputFrame {
        down: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert_eq!(
        game.player.held,
        Some(bow),
        "FIXTURE_REAL_PICKUP: {}",
        diagnostic(game)
    );
    assert_eq!(game.player.pickup_item_type, "Bow");
    assert_eq!(game.data["arrows"], 6.0, "new source Bow grants six arrows");
    assert_eq!(
        game.player.data.get("bow_armed").copied().unwrap_or(0.0),
        0.0
    );
    game.step(&InputFrame::default());
    assert_eq!(game.player.held, Some(bow));
    assert_eq!(game.player.health, 4.0);
    bow
}

fn face(game: &mut Game, facing: i8, attack: bool) {
    // Initial facing is authored. Quiet steps keep the same fully supported
    // pose while the caller either holds its charge or leaves the Bow unarmed.
    for _ in 0..3 {
        game.step(&InputFrame {
            attack,
            ..Default::default()
        });
        assert_eq!(
            game.player.health,
            4.0,
            "FIXTURE_FACE_BEFORE_HIT: {}",
            diagnostic(game)
        );
    }
    assert_eq!(game.player.facing, facing);
    assert_eq!(game.player.pos, Vec2::new(264.0, 152.0));
    assert_eq!(game.player.vel, Vec2::default());
}

fn unsafe_arrow(game: &Game) -> Option<u64> {
    game.world
        .entities
        .iter()
        .find(|item| {
            item.alive
                && item.engine_active
                && item.object == "oArrow"
                && item.value("thrown_by_player") == 0.0
                && item.value("safe") == 0.0
                && item.vel.x.abs() > 3.0
        })
        .map(|item| item.id)
}

fn enter_detector(game: &mut Game, attack: bool) -> u64 {
    assert!(
        unsafe_arrow(game).is_none(),
        "raised detector has not fired on stationary pickup/shots"
    );
    assert_eq!(game.player.pos, Vec2::new(264.0, 152.0));
    for tick in 0..8 {
        assert_eq!(
            game.player.health,
            4.0,
            "FIXTURE_NO_PRIOR_HIT: {}",
            diagnostic(game)
        );
        // Jump enters the raised row7 detector. Its shot is born six pixels
        // from the player's x, at y132, after this frame's Player hurt query.
        game.step(&InputFrame {
            jump: true,
            jump_pressed: tick == 0,
            attack,
            ..Default::default()
        });
        if tick == 0 {
            assert!(
                game.player.pos.y < 152.0 && game.player.vel.y < 0.0,
                "FIXTURE_REAL_GROUNDED_JUMP: {}",
                diagnostic(game)
            );
            assert!(!game.player.grounded);
            assert_eq!(sound(game, "sndJump"), 1);
        }
        if let Some(id) = unsafe_arrow(game) {
            assert_eq!(game.player.health, 4.0, "trap birth follows Player Hurt");
            assert_eq!(
                sound(game, "sndArrowTrap"),
                1,
                "real detector emits one trap shot"
            );
            let arrow = game.world.entity(id).unwrap();
            let right = game.player.facing > 0;
            assert_eq!(
                arrow.pos,
                Vec2::new(if right { 258.0 } else { 270.0 }, 132.0)
            );
            assert_eq!(arrow.vel, Vec2::new(if right { 8.0 } else { -8.0 }, 0.0));
            assert_eq!(arrow.value("safe"), 0.0);
            assert_eq!(
                arrow.age, 0,
                "automatic-collision birth has no earlier Arrow Step"
            );
            return id;
        }
    }
    panic!("FIXTURE_REAL_TRAP_BIRTH_TIMEOUT: {}", diagnostic(game));
}

fn wait_hit(game: &mut Game, trap_arrow: u64, attack: bool) {
    for _ in 0..100 {
        game.step(&InputFrame {
            jump: true,
            attack,
            ..Default::default()
        });
        if game.player.health < 4.0 {
            assert_eq!(
                game.player.health,
                2.0,
                "real first Arrow hit: {}",
                diagnostic(game)
            );
            assert_eq!(game.player.stun, 20);
            assert!(
                game.world.entity(trap_arrow).is_none(),
                "Player consumes unsafe trap Arrow"
            );
            assert_eq!(sound(game, "sndHurt"), 1);
            return;
        }
        assert_eq!(game.state, RunState::Playing);
    }
    panic!("FIXTURE_REAL_TRAP_HIT_TIMEOUT: {}", diagnostic(game));
}

fn assert_same_bow_release(game: &Game, bow: u64, expected_ammo: f64) {
    // Primary semantic consequence precedes timer/type/held metadata oracles.
    assert_eq!(
        game.data["arrows"],
        expected_ammo,
        "CONTRACT_LATE_BOW_FIRES_WITHOUT_ARMED_OR_AMMO_GUARD: {}",
        diagnostic(game)
    );
    let shots: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|item| {
            item.alive
                && item.object == "oArrow"
                && item.value("thrown_by_player") == 1.0
                && item.age == 0
        })
        .collect();
    assert_eq!(shots.len(), 1, "one newborn Bow shot follows trap hurt");
    assert_eq!(
        sound(game, "sndArrowTrap"),
        1,
        "Bow emits its own shot sound on hurt frame"
    );
    assert_eq!(game.player.held, None);
    assert!(game.player.pickup_item_type.is_empty());
    let released = game.world.entity(bow).expect("same released Bow survives");
    assert!(!released.held);
    assert_eq!(
        released.value("armed"),
        1.0,
        "late branch arms original before firing"
    );
    assert_eq!(game.player.data["bow_armed"], 0.0);
    assert_eq!(game.player.data["bow_strength"], 0.0);
    assert_eq!(
        game.player.data["weapon_cooldown"], 10.0,
        "late fire follows this frame's early FIRING decrement"
    );
    assert_eq!(shots[0].value("safe"), 1.0);
    assert_eq!(shots[0].alarms[2], 10, "late birth follows alarms");
    assert_eq!(
        shots[0].previous_pos.y, game.player.pos.y,
        "standing shot has no source duck-y adjustment"
    );
    assert_eq!(
        shots[0].previous_pos.x,
        game.player.pos.x + 14.0 * f64::from(game.player.facing)
    );
    assert_eq!(
        sound(game, "sndThrow"),
        0,
        "hurt release has no generic throw sound"
    );
}

#[test]
fn natural_uncharged_bow_fires_once_on_real_trap_hurt_before_release() {
    for rules in PRESETS {
        for facing in [-1, 1] {
            let mut game = room(rules, facing, true);
            let bow = pickup(&mut game);
            face(&mut game, facing, false);
            let trap_arrow = enter_detector(&mut game, false);
            assert_eq!(
                game.player.data.get("bow_armed").copied().unwrap_or(0.0),
                0.0
            );
            assert_eq!(game.data["arrows"], 6.0);
            wait_hit(&mut game, trap_arrow, false);
            assert_same_bow_release(&game, bow, 5.0);
        }
    }
}

#[test]
fn natural_charged_bow_real_trap_hurt_keeps_one_shot_not_two() {
    for rules in PRESETS {
        for facing in [-1, 1] {
            let mut game = room(rules, facing, true);
            let bow = pickup(&mut game);
            game.step(&InputFrame {
                attack: true,
                attack_pressed: true,
                ..Default::default()
            });
            assert_eq!(game.player.data["bow_armed"], 1.0);
            face(&mut game, facing, true);
            let trap_arrow = enter_detector(&mut game, true);
            assert_eq!(game.player.data["bow_armed"], 1.0);
            assert!(game.player.data["bow_strength"] > 0.0);
            assert_eq!(game.data["arrows"], 6.0);
            wait_hit(&mut game, trap_arrow, true);
            assert_same_bow_release(&game, bow, 5.0);
        }
    }
}

#[test]
fn natural_exhausted_six_shots_then_trap_hurt_spends_zero_to_negative_one() {
    for rules in PRESETS {
        for facing in [-1, 1] {
            let mut game = room(rules, facing, true);
            let bow = pickup(&mut game);
            face(&mut game, facing, false);
            // Six actual standing shots stay below the raised detector. They
            // start beyond the collection rectangle and travel away before settling.
            for remaining in (0..6).rev() {
                game.step(&InputFrame {
                    attack: true,
                    attack_pressed: true,
                    ..Default::default()
                });
                assert_eq!(game.player.data["bow_armed"], 1.0);
                for _ in 0..49 {
                    game.step(&InputFrame {
                        attack: true,
                        ..Default::default()
                    });
                    assert_eq!(game.player.health, 4.0, "trap not yet triggered");
                }
                assert!(game.player.data["bow_strength"] >= 9.0);
                game.step(&InputFrame {
                    attack_released: true,
                    ..Default::default()
                });
                assert_eq!(
                    game.data["arrows"],
                    f64::from(remaining),
                    "ordinary release spends one"
                );
                assert_eq!(sound(&game, "sndArrowTrap"), 1);
                for _ in 0..20 {
                    game.step(&InputFrame::default());
                    assert_eq!(game.player.health, 4.0);
                    assert_eq!(
                        game.data["arrows"],
                        f64::from(remaining),
                        "no automatic ammo recollection"
                    );
                }
                assert_eq!(game.player.held, Some(bow));
            }
            assert_eq!(game.data["arrows"], 0.0);
            assert_eq!(game.player.data["bow_armed"], 0.0);
            face(&mut game, facing, false);
            let trap_arrow = enter_detector(&mut game, false);
            wait_hit(&mut game, trap_arrow, false);
            assert_same_bow_release(&game, bow, -1.0);
        }
    }
}

#[test]
fn natural_reserved_bomb_hurt_arms_original_then_restores_fresh_bow_without_firing() {
    for rules in PRESETS {
        for facing in [-1, 1] {
            let mut game = room(rules, facing, true);
            let bow = pickup(&mut game);
            game.step(&InputFrame {
                cycle_pressed: true,
                ..Default::default()
            });
            let bomb = game.player.held.expect("ordinary cycle reserves Bomb");
            assert_eq!(game.world.entity(bomb).unwrap().object, "oBomb");
            assert!(game.world.entity(bow).is_none());
            assert_eq!(game.player.pickup_item_type, "Bow");
            game.step(&InputFrame::default());
            face(&mut game, facing, false);
            let trap_arrow = enter_detector(&mut game, false);
            wait_hit(&mut game, trap_arrow, false);
            // Source's Bomb branch restores its remembered type; it does not
            // execute the Bow branch again on the newly restored receiver.
            assert_eq!(game.data["arrows"], 6.0);
            assert_eq!(sound(&game, "sndArrowTrap"), 0);
            let restored = game.player.held.expect("fresh scrHoldItem Bow");
            assert_ne!(restored, bow);
            assert_ne!(restored, bomb);
            assert_eq!(game.world.entity(restored).unwrap().object, "oBow");
            assert_eq!(game.world.entity(restored).unwrap().value("new"), 0.0);
            assert_eq!(game.world.entity(restored).unwrap().value("cost"), 0.0);
            assert_eq!(game.player.pickup_item_type, "Bow");
            let released = game
                .world
                .entity(bomb)
                .expect("original Bomb released, no explosion here");
            assert!(!released.held);
            assert_eq!(released.value("armed"), 1.0);
        }
    }
}

#[test]
fn natural_uncharged_supported_bow_has_no_hurt_and_no_spontaneous_shot() {
    for rules in PRESETS {
        for facing in [-1, 1] {
            let mut game = room(rules, facing, false);
            let bow = pickup(&mut game);
            face(&mut game, facing, false);
            for _ in 0..40 {
                game.step(&InputFrame::default());
                assert_eq!(game.player.health, 4.0);
                assert_eq!(game.data["arrows"], 6.0);
                assert_eq!(game.player.held, Some(bow));
                assert_eq!(sound(&game, "sndArrowTrap"), 0);
                assert_eq!(
                    game.player.data.get("bow_armed").copied().unwrap_or(0.0),
                    0.0
                );
            }
        }
    }
}
