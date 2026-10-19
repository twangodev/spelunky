//! Controlled fractional character poses, not natural campaign witnesses.
//! Literal endpoints follow the original collision helpers before RTE rounding.
use super::move_solid;
use crate::{Entity, Game, InputFrame, PlayerState, RulesPreset, Vec2};

fn controlled_game(preset: RulesPreset, player: Vec2) -> Game {
    let mut game = Game::legacy_shell(7, preset);
    game.player.pos = player;
    game.player.previous_pos = player;
    game.player.vel = Vec2::default();
    game.player.state = PlayerState::Standing;
    game.player.grounded = false;
    game
}

fn controlled_solid(game: &mut Game, pos: Vec2, delta: Vec2, viscid: bool) -> Entity {
    // This deliberately controlled base receiver has the original full 16x16
    // sBlock mask. No arbitrary transform or altered collision API is needed.
    let id = game.spawn("oMovingSolid", pos);
    let solid = game.world.entity_mut(id).unwrap();
    solid.sprite = "sBlock".into();
    solid.vel = delta;
    solid
        .data
        .insert("viscidTop".into(), if viscid { 1.0 } else { 0.0 });
    solid.clone()
}

#[test]
fn controlled_side_push_includes_only_the_source_last_row_at_signed_half_ties() {
    // (player Y, solid first row, contact). Source last row is RTE(Y+8-1).
    // Integer controls include that last row and exclude its next neighbor.
    for (y, first_row, contact) in [
        (72.0, 79.0, true),
        (72.0, 80.0, false),
        (-72.0, -65.0, true),
        (-72.0, -64.0, false),
        (72.5, 80.0, true),
        (73.5, 81.0, false),
        (-73.5, -66.0, true),
        (-72.5, -65.0, false),
    ] {
        for direction in [-1.0, 1.0] {
            for preset in [RulesPreset::Classic, RulesPreset::Assisted] {
                let mut game = controlled_game(preset, Vec2::new(96.0, y));
                let x = if direction > 0.0 { 75.0 } else { 101.0 };
                let mut solid = controlled_solid(
                    &mut game,
                    Vec2::new(x, first_row),
                    Vec2::new(direction, 0.0),
                    false,
                );
                assert!(!move_solid(
                    &mut game,
                    &mut solid,
                    Vec2::new(direction, 0.0)
                ));
                assert_eq!(
                    game.player.pos.x,
                    96.0 + if contact { direction } else { 0.0 },
                    "Y={y}, first_row={first_row}, direction={direction}, preset={preset:?}"
                );
                assert_eq!(solid.pos, Vec2::new(x + direction, first_row));
            }
        }
    }
}

#[test]
fn controlled_side_push_rounds_horizontal_offsets_before_sampling() {
    // Source character-left X=RTE(X-5-1), character-right X=RTE(X+5).
    for (x, direction, solid_x, contact) in [
        (96.0, 1.0, 75.0, true),
        (96.0, -1.0, 101.0, true),
        (96.5, 1.0, 75.0, true),
        (97.5, 1.0, 76.0, false),
        (96.5, -1.0, 102.0, true),
        (97.5, -1.0, 103.0, false),
    ] {
        for preset in [RulesPreset::Classic, RulesPreset::Assisted] {
            let mut game = controlled_game(preset, Vec2::new(x, 72.0));
            let mut solid = controlled_solid(
                &mut game,
                Vec2::new(solid_x, 64.0),
                Vec2::new(direction, 0.0),
                false,
            );
            assert!(!move_solid(
                &mut game,
                &mut solid,
                Vec2::new(direction, 0.0)
            ));
            assert_eq!(
                game.player.pos.x,
                x + if contact { direction } else { 0.0 },
                "X={x}, direction={direction}, solid_x={solid_x}, preset={preset:?}"
            );
            assert_eq!(solid.pos.x, solid_x + direction);
        }
    }
}

#[test]
fn controlled_side_obstruction_uses_the_same_source_bottom_endpoint() {
    for (y, blocker_row, blocked) in [
        (72.0, 79.0, true),
        (72.0, 80.0, false),
        (72.5, 80.0, true),
        (73.5, 81.0, false),
    ] {
        for direction in [-1.0, 1.0] {
            for preset in [RulesPreset::Classic, RulesPreset::Assisted] {
                let mut game = controlled_game(preset, Vec2::new(96.0, y));
                let x = if direction > 0.0 { 75.0 } else { 101.0 };
                let mut solid = controlled_solid(
                    &mut game,
                    Vec2::new(x, 64.0),
                    Vec2::new(direction, 0.0),
                    false,
                );
                let blocker_x = if direction > 0.0 { 101.0 } else { 75.0 };
                game.spawn("oBlock", Vec2::new(blocker_x, blocker_row));
                assert_eq!(
                    move_solid(&mut game, &mut solid, Vec2::new(direction, 0.0)),
                    blocked
                );
                let movement = if blocked { 0.0 } else { direction };
                assert_eq!(solid.pos, Vec2::new(x + movement, 64.0));
                assert_eq!(game.player.pos, Vec2::new(96.0 + movement, y));
            }
        }
    }
}

#[test]
fn controlled_rising_underside_rounds_the_complete_signed_endpoint() {
    // Source character-bottom(1) samples RTE(Y-8-1).
    for (y, solid_y, contact) in [
        (72.5, 64.0, true),
        (73.5, 65.0, false),
        (-72.5, -97.0, true),
        (-73.5, -98.0, false),
    ] {
        for preset in [RulesPreset::Classic, RulesPreset::Assisted] {
            let mut game = controlled_game(preset, Vec2::new(96.0, y));
            game.player.jump_time = 2;
            game.player.vel.y = -4.0;
            let mut solid = controlled_solid(
                &mut game,
                Vec2::new(91.0, solid_y),
                Vec2::new(0.0, -1.0),
                false,
            );
            assert!(!move_solid(&mut game, &mut solid, Vec2::new(0.0, -1.0)));
            assert_eq!(game.player.jump_time, if contact { 10 } else { 2 });
            assert_eq!(game.player.vel.y, if contact { -2.0 } else { -4.0 });
            assert_eq!(game.player.pos, Vec2::new(96.0, y));
            assert_eq!(solid.pos.y, solid_y - 1.0);
        }
    }
}

#[test]
fn controlled_downward_carry_rounds_character_top_two_before_sampling() {
    // Source character-top(2) samples RTE(Y+8+2-1).
    for (y, solid_y, contact) in [
        (72.5, 82.0, true),
        (73.5, 83.0, false),
        (-73.5, -64.0, true),
        (-72.5, -63.0, false),
    ] {
        for preset in [RulesPreset::Classic, RulesPreset::Assisted] {
            let mut game = controlled_game(preset, Vec2::new(96.0, y));
            let mut solid = controlled_solid(
                &mut game,
                Vec2::new(91.0, solid_y),
                Vec2::new(0.0, 1.0),
                true,
            );
            assert!(!move_solid(&mut game, &mut solid, Vec2::new(0.0, 1.0)));
            assert_eq!(
                game.player.pos,
                Vec2::new(96.0, y + if contact { 1.0 } else { 0.0 })
            );
            assert_eq!(game.player.grounded, contact);
            assert_eq!(solid.pos.y, solid_y + 1.0);
        }
    }
}

#[test]
fn full_game_step_controlled_fractional_side_contact_keeps_source_bottom_row() {
    // Public entry point, controlled initial fractional pose and solid velocity;
    // this does not claim an ordinary-input campaign route reaches that pose.
    for (y, first_row, contact) in [
        (72.0, 79.0, true),
        (72.0, 80.0, false),
        (72.5, 80.0, true),
        (73.5, 81.0, false),
    ] {
        for preset in [RulesPreset::Classic, RulesPreset::Assisted] {
            let mut game = controlled_game(preset, Vec2::new(96.0, y));
            let solid = controlled_solid(
                &mut game,
                Vec2::new(75.0, first_row),
                Vec2::new(1.0, 0.0),
                false,
            );
            game.step(&InputFrame::default());
            assert_eq!(game.world.entity(solid.id).unwrap().pos.x, 76.0);
            assert_eq!(
                game.player.pos.x,
                if contact { 97.0 } else { 96.0 },
                "Y={y}, first_row={first_row}, preset={preset:?}"
            );
        }
    }
}
