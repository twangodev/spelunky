//! Bow's own Step observes this tick's Player charge/release after inherited
//! oItem Step. Public Game::step inputs only; no charge/frame/held injection.
use spelunky_core::{
    Area, Game, GameEvent, InputFrame, PlayerState, RulesPreset, Vec2, World, catalog,
};

fn room(rules: RulesPreset) -> Game {
    let mut game = Game::shell(42, rules);
    game.world = World::new(80, 40, Area::Mines, 1);
    for x in 0..game.world.width {
        game.world.set_tile(x, 10, "oBrickSmooth");
    }
    game.world.entrance = Vec2::new(256.0, 152.0);
    game.player.pos = game.world.entrance;
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.camera = Vec2::new(64.0, 0.0);
    game.previous_camera = game.camera;
    game
}

fn pickup(game: &mut Game) -> u64 {
    let bow = game.spawn("oBow", game.player.pos);
    game.step(&InputFrame {
        down: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert_eq!(game.player.held, Some(bow));
    game.step(&InputFrame::default());
    assert_eq!(game.data["arrows"], 6.0);
    bow
}

fn held_attack(game: &mut Game, pressed: bool) {
    game.step(&InputFrame {
        attack: true,
        attack_pressed: pressed,
        ..Default::default()
    });
}

fn charge(game: &mut Game, ticks: usize) {
    held_attack(game, true);
    for _ in 1..ticks {
        held_attack(game, false);
    }
    assert_eq!(game.player.data["bow_armed"], 1.0);
}

fn expected_frame(strength: f64) -> f64 {
    if strength >= 10.0 {
        3.0
    } else if strength > 6.0 {
        2.0
    } else if strength > 2.0 {
        1.0
    } else {
        0.0
    }
}

fn assert_frame(game: &Game, bow: u64, frame: f64) {
    let live = game
        .world
        .entity(bow)
        .expect("Bow remains in the live room");
    let draw = game
        .presentation()
        .world
        .entity(bow)
        .expect("Bow remains in frozen Draw");
    assert_eq!(
        live.animation, frame,
        "own Step must use post-Player bowStrength"
    );
    assert_eq!(
        draw.animation, frame,
        "Draw must not retain previous tick's charge frame"
    );
    assert_eq!(draw.sprite, live.sprite);
    assert_eq!(catalog::sprite_dimensions(&draw.sprite), Some((16, 16)));
    assert_eq!(catalog::sprite_frame_count(&draw.sprite), 4);
    assert_eq!(
        live.image_speed(),
        0.0,
        "own Step selects frames; Animation must not advance Bow"
    );
}

fn arrow_births(game: &Game) -> usize {
    game.world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oArrow")
        .count()
}

fn shot_sounds(game: &Game) -> usize {
    game.events.iter().filter(|e| matches!(e,
        GameEvent::Sound { name, .. } | GameEvent::SoundIfIdle { name, .. } if name == "sndArrowTrap"
    )).count()
}

#[test]
fn natural_bow_charge_thresholds_refresh_in_own_step_before_draw() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for right in [false, true] {
            let mut game = room(rules);
            let bow = pickup(&mut game);
            assert_frame(&game, bow, 0.0);
            let mut strength = 0.0_f64;
            let mut frames = [false; 4];
            for tick in 0..55 {
                game.step(&InputFrame {
                    attack: true,
                    attack_pressed: tick == 0,
                    right,
                    left: !right,
                    ..Default::default()
                });
                strength += 0.2;
                assert!((game.player.data["bow_strength"] - strength).abs() < 1e-9);
                let frame = expected_frame(strength);
                frames[frame as usize] = true;
                assert_frame(&game, bow, frame);
                assert_eq!(
                    game.world.entity(bow).unwrap().sprite,
                    if right { "sBowRight" } else { "sBowLeft" }
                );
                assert_eq!(game.data["arrows"], 6.0);
                assert_eq!(arrow_births(&game), 0);
            }
            assert_eq!(
                frames, [true; 4],
                "all authored charge thresholds reached by held input"
            );
        }
    }
}

#[test]
fn natural_bow_release_resets_visible_frame_in_the_same_tick() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = room(rules);
        let bow = pickup(&mut game);
        charge(&mut game, 40);
        assert!(game.player.data["bow_strength"] > 6.0);
        game.step(&InputFrame {
            attack_released: true,
            ..Default::default()
        });
        assert_eq!(game.player.held, Some(bow));
        assert_eq!(game.player.data["bow_strength"], 0.0);
        assert_eq!(game.player.data["bow_armed"], 0.0);
        assert_frame(&game, bow, 0.0);
        assert_eq!(game.data["arrows"], 5.0);
        assert_eq!(arrow_births(&game), 1);
        assert_eq!(shot_sounds(&game), 1);
    }
}

#[test]
fn natural_bow_duck_drop_clears_unheld_frame_without_firing_or_spending_ammo() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = room(rules);
        let bow = pickup(&mut game);
        charge(&mut game, 40);
        game.step(&InputFrame {
            down: true,
            attack: true,
            ..Default::default()
        });
        assert_eq!(game.player.state, PlayerState::Ducking);
        assert!(game.player.data["bow_strength"] > 6.0);
        game.step(&InputFrame {
            down: true,
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert!(game.player.held.is_none());
        assert!(!game.world.entity(bow).unwrap().held);
        assert_eq!(game.player.data["bow_strength"], 0.0);
        assert_frame(&game, bow, 0.0);
        // scrUseItem's Bow duck-drop clears holdItem without scrFireBow.
        // Later held attack clears the now-unheld sub-cap draw, spending no ammo.
        assert_eq!(game.player.data["bow_armed"], 0.0);
        assert_eq!(game.data["arrows"], 6.0);
        assert_eq!(arrow_births(&game), 0);
        assert_eq!(shot_sounds(&game), 0);
        for _ in 0..4 {
            game.step(&InputFrame::default());
            assert_frame(&game, bow, 0.0);
        }
    }
}

#[test]
fn natural_bow_cycle_fires_before_replacement_and_restores_a_zero_frame_fresh_bow() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = room(rules);
        let old = pickup(&mut game);
        charge(&mut game, 40);
        let strength = game.player.data["bow_strength"];
        game.step(&InputFrame {
            cycle_pressed: true,
            attack: true,
            ..Default::default()
        });
        assert!(game.world.entity(old).is_none());
        assert_eq!(game.data["arrows"], 5.0);
        assert_eq!(game.player.data["bow_strength"], 0.0);
        assert_eq!(arrow_births(&game), 1);
        let arrow = game
            .world
            .entities
            .iter()
            .find(|e| e.alive && e.object == "oArrow")
            .unwrap();
        assert!(
            (arrow.vel.x - (1.0 + strength)).abs() < 1e-9,
            "cycle fires before this tick's later +0.2 charge"
        );
        assert_eq!(shot_sounds(&game), 1);
        for expected in ["oRopeThrow", "oBow"] {
            game.step(&InputFrame {
                cycle_pressed: true,
                ..Default::default()
            });
            assert_eq!(
                game.world.entity(game.player.held.unwrap()).unwrap().object,
                expected
            );
        }
        let fresh = game.player.held.unwrap();
        assert_ne!(fresh, old);
        assert_eq!(game.world.entity(fresh).unwrap().value("new"), 0.0);
        assert_frame(&game, fresh, 0.0);
        assert_eq!(
            game.data["arrows"], 5.0,
            "restoring Bow must not grant six more arrows"
        );
    }
}

#[test]
fn natural_bow_no_reserves_cycle_still_refreshes_the_new_charge_frame() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = room(rules);
        game.player.bombs = 0;
        game.player.ropes = 0;
        let bow = pickup(&mut game);
        charge(&mut game, 10);
        let before = game.player.data["bow_strength"];
        assert!(before <= 2.0);
        game.step(&InputFrame {
            cycle_pressed: true,
            attack: true,
            ..Default::default()
        });
        assert_eq!(game.player.held, Some(bow));
        let strength = game.player.data["bow_strength"];
        assert!((strength - before - 0.2).abs() < 1e-9);
        assert!(strength > 2.0);
        assert_frame(&game, bow, 1.0);
        assert_eq!(game.data["arrows"], 6.0);
        assert_eq!(arrow_births(&game), 0);
        assert_eq!(shot_sounds(&game), 0);
    }
}

#[test]
fn natural_bow_unarmed_and_genuinely_exhausted_ammo_keep_frame_zero() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = room(rules);
        let bow = pickup(&mut game);
        for _ in 0..4 {
            game.step(&InputFrame::default());
            assert_frame(&game, bow, 0.0);
        }
        for expected_ammo in (0..6).rev() {
            // Full strength moves spent arrows away from the stationary Bow
            // holder, avoiding the source's automatic nearby-arrow recovery.
            charge(&mut game, 55);
            game.step(&InputFrame {
                attack_released: true,
                ..Default::default()
            });
            assert_eq!(game.data["arrows"], f64::from(expected_ammo));
            assert_eq!(shot_sounds(&game), 1);
            assert_frame(&game, bow, 0.0);
            for _ in 0..11 {
                game.step(&InputFrame::default());
            }
        }
        held_attack(&mut game, true);
        assert!(
            game.events
                .iter()
                .any(|e| matches!(e, GameEvent::Dialogue(message)
                    if message.first == "I'M OUT OF ARROWS!"
                        && message.second.is_empty()
                        && message.ticks == 80))
        );
        assert_eq!(game.data["message_timer"], 80.0);
        assert_eq!(game.data["arrows"], 0.0);
        assert_eq!(game.player.data["bow_armed"], 0.0);
        assert_frame(&game, bow, 0.0);
        assert_eq!(shot_sounds(&game), 0);
    }
}

#[test]
fn uncharged_bow_own_step_preserves_the_strict_sixteen_pixel_parent_movement_gate() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = room(rules);
        // Authored initial room/camera, not a mid-run teleport. All three Bows
        // remain inside the source's larger +/-96 activation margin.
        game.player.pos.x = 392.0;
        game.player.previous_pos = game.player.pos;
        game.world.entrance = game.player.pos;
        game.camera.x = 200.0;
        game.previous_camera = game.camera;
        let outside = game.spawn("oBow", Vec2::new(183.75, 80.0));
        let boundary = game.spawn("oBow", Vec2::new(184.0, 80.0));
        let inside = game.spawn("oBow", Vec2::new(184.25, 80.0));
        for _ in 0..6 {
            game.step(&InputFrame::default());
        }
        for id in [outside, boundary, inside] {
            assert!(game.world.entity(id).unwrap().engine_active);
            assert!(!game.world.entity(id).unwrap().held);
            assert_frame(&game, id, 0.0);
        }
        assert_eq!(game.world.entity(outside).unwrap().pos.y, 80.0);
        assert_eq!(game.world.entity(boundary).unwrap().pos.y, 80.0);
        assert!(game.world.entity(inside).unwrap().pos.y > 80.0);
        assert_eq!(game.player.held, None);
        assert_eq!(
            game.player.data.get("bow_strength").copied().unwrap_or(0.0),
            0.0
        );
    }
}
