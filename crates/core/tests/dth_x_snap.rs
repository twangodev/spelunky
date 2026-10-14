//! Ordinary authored-room regression plus explicitly configured callback controls.
//! Initial authored placement is the only setup; later motion uses ordinary inputs.
use spelunky_core::characters::{self, Character};
use spelunky_core::{Area, Game, InputFrame, PlayerState, RulesPreset, Vec2, World};

#[test]
fn natural_head_carry_then_raised_ledge_flip_needs_both_axis_snap() {
    fractional_head_route(RulesPreset::Classic);
}

#[test]
fn assisted_head_carry_then_raised_ledge_flip_needs_both_axis_snap() {
    fractional_head_route(RulesPreset::Assisted);
}

fn fractional_head_route(rules: RulesPreset) {
    let mut game = Game::shell(42, rules);
    game.world = World::new(40, 20, Area::Temple, 14);
    game.world.name = "rLevel".into();
    for x in 0..40 {
        game.world.set_tile(x, 10, "oHardBlock");
    }
    // An adjacent raised original solid gives the player a stationary ledge
    // after leaving the moving head. It remains present and destructible by
    // genuine source callbacks; no floor or boss fields are rewritten later.
    // Original block actors can occupy off-grid authored coordinates. Moving
    // this wall eight pixels left makes the boss's x+62 right probe meet it
    // before the observed .25-through-1.75 carry sum becomes an integer.
    // This is a distinct initial geometry, with the prior input policy intact.
    for x in [328., 344., 360.] {
        for y in 2..10 {
            game.spawn("oHardBlock", Vec2::new(x, f64::from(y * 16)));
        }
    }
    game.player.pos = Vec2::new(321., 88.);
    game.player.previous_pos = game.player.pos;
    let boss_id = game.spawn("oOlmec", Vec2::new(256., 96.));
    let mut stage = 0;
    let mut jump_started = false;
    let mut right_started = false;
    let mut brake_frames = 0;
    let mut entered_flip = false;
    for frame in 0..600 {
        let before = game.player.clone();
        let boss = game
            .world
            .entity(boss_id)
            .expect("original live boss")
            .clone();
        let input = match stage {
            0 => InputFrame::default(),
            1 => {
                let pressed = !jump_started;
                jump_started = true;
                // First clear the raised ledge's top with the source foot
                // offset 8 plus one pixel of margin, then approach in the air.
                // A wall grab before clearance would itself snap X.
                if before.pos.y + 9. < 32. {
                    right_started = true;
                }
                InputFrame {
                    right: right_started,
                    jump: true,
                    jump_pressed: pressed,
                    ..Default::default()
                }
            }
            2 => InputFrame {
                down: true,
                ..Default::default()
            },
            3 => InputFrame {
                down: true,
                right: true,
                ..Default::default()
            },
            _ => InputFrame::default(),
        };
        game.step(&input);
        assert_eq!(
            game.player.health, 4,
            "healthy ordinary route at frame {frame}"
        );
        assert!(!game.player.is_stunned());
        match stage {
            0 if game.player.pos.x.fract() != 0. => {
                assert_eq!(before.pos.x.fract(), 0.);
                assert_ne!(boss.vel.x.fract(), 0.);
                assert_ne!(boss.value("carryPlayer"), 0.);
                assert_eq!(
                    game.player.pos.x.fract(),
                    (before.pos.x + boss.vel.x).fract()
                );
                stage = 1;
            }
            1 if game.player.grounded
                && game.player.pos.x >= 341.
                && (game.player.pos.y - 24.).abs() < 0.5 =>
            {
                stage = 2;
            }
            2 => {
                brake_frames += 1;
                if brake_frames == 2 {
                    assert_eq!(game.player.vel.x, 0., "source neutral crawl braking");
                    stage = 3;
                }
            }
            _ => {}
        }
        let flip = game.player.data.get("duck_to_hang") == Some(&1.);
        if flip && !entered_flip {
            assert_eq!(stage, 3);
            assert_ne!(
                game.player.pos.x.fract(),
                0.,
                "natural fractional flip entry"
            );
            entered_flip = true;
            stage = 4;
        }
        if entered_flip && before.data.get("duck_to_hang") == Some(&1.) && !flip {
            // The raised ledge is horizontally outside the raw carry box,
            // even using a conservative complete DuckToHang sprite margin.
            assert!(before.pos.x - 24. > boss.pos.x + 66.);
            assert_eq!(boss.value("carryPlayer"), 0.);
            assert_ne!(before.pos.x.fract(), 0.);
            assert_eq!(game.player.state, PlayerState::Hanging);
            assert_eq!(game.player.facing, -1);
            assert_eq!(
                game.player.pos.x,
                before.pos.x.round_ties_even() + 6.,
                "SOURCE_MOVE_SNAP_ROUNDS_X_BEFORE_RIGHT_LEDGE_OFFSET"
            );
            return;
        }
    }
    panic!(
        "finite route unqualified: stage={stage}, flip={entered_flip}, player={:?}, boss={:?}",
        game.player,
        game.world.entity(boss_id)
    );
}

// These initialize a pending callback before the first dispatch. They verify
// literal callback arithmetic, not a natural fractional left-facing encounter.
#[test]
fn configured_animation_end_snaps_x_before_both_hanging_offsets() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for character in [Character::Explorer, Character::Damsel, Character::TunnelMan] {
            for facing in [-1, 1] {
                for (x, snapped) in [
                    (375., 375.),
                    (375.25, 375.),
                    (375.5, 376.),
                    (374.5, 374.),
                    (-1.5, -2.),
                    (-2.5, -2.),
                ] {
                    let mut game = Game::shell(42, rules);
                    characters::apply(&mut game, character);
                    game.player.pos = Vec2::new(x, 24.25);
                    game.player.previous_pos = game.player.pos;
                    game.player.facing = facing;
                    game.player.state = PlayerState::Ducking;
                    game.player.data.insert("duck_to_hang".into(), 1.);
                    game.player.vel = Vec2::new(3., -4.);
                    game.player.set_sprite(match character {
                        Character::Explorer => "sDuckToHangL",
                        Character::Damsel => "sDamselDtHL",
                        Character::TunnelMan => "sTunnelDtHL",
                    });
                    let sprite = game.player.sprite.clone();
                    spelunky_core::player::animation_end(&mut game);
                    assert_eq!(
                        game.player.pos,
                        Vec2::new(snapped + if facing < 0 { -5. } else { 6. }, 40.),
                        "SOURCE_MOVE_SNAP_BOTH_AXES_BEFORE_OFFSET rules={rules:?} character={character:?} facing={facing} x={x}"
                    );
                    assert_eq!(game.player.state, PlayerState::Hanging);
                    assert_eq!(game.player.facing, -facing);
                    assert_eq!(game.player.vel, Vec2::default());
                    assert_eq!(game.player.previous_pos, game.player.pos);
                    assert_eq!(game.player.sprite, sprite);
                    assert_eq!(game.player.data.get("duck_to_hang"), Some(&0.));
                    assert_eq!(game.player.data.get("gravity"), Some(&0.));
                }
            }
        }
    }
}
