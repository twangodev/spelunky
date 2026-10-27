//! Source Player Step803/908 and2199/2229 use separate collision_point and
//! instance_place queries. A door covering the origin can enable selection of
//! an earlier door touched only by the Player's full collision mask.
use super::*;
use crate::RulesPreset;

fn adjacent_exits(rules: RulesPreset, reverse_storage: bool) -> (Game, u64, u64) {
    let mut game = Game::shell(37, rules);
    for x in 0..game.world.width {
        game.world.set_tile(x, 4, "oTemple");
    }
    game.player.pos = Vec2::new(98., 56.);
    game.player.previous_pos = game.player.pos;
    let first = game.spawn("oExit", Vec2::new(80., 48.));
    let second = game.spawn("oExit", Vec2::new(96., 48.));
    game.world
        .entity_mut(first)
        .unwrap()
        .text
        .insert("leadsTo".into(), "first.lvl".into());
    game.world
        .entity_mut(second)
        .unwrap()
        .text
        .insert("leadsTo".into(), "second.lvl".into());
    if reverse_storage {
        game.world.entities.reverse();
    }
    let first_mask = game.world.entity(first).unwrap().collision_mask();
    let second_mask = game.world.entity(second).unwrap().collision_mask();
    assert!(!first_mask.contains(game.player.pos, crate::CollisionPrecision::BoundingBox));
    assert!(second_mask.contains(game.player.pos, crate::CollisionPrecision::BoundingBox));
    assert!(game.player.collision_mask().intersects_mask(&first_mask));
    (game, first, second)
}

#[test]
fn actual_step_exit_gate_and_receiver_are_independent_in_source_family_order() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for reverse_storage in [false, true] {
            let (mut game, _, _) = adjacent_exits(rules, reverse_storage);
            game.step(&InputFrame {
                up: true,
                ..Default::default()
            });
            assert_eq!(game.state, RunState::Ending);
            assert_eq!(game.player.pos, Vec2::new(88., 56.));
            assert!(game.flags.contains("custom_level:first.lvl"));
            assert!(!game.flags.contains("custom_level:second.lvl"));
        }
    }
}

#[test]
fn legacy_independent_exit_frame_does_not_open_door_but_up_does() {
    let (mut game, _, _) = adjacent_exits(RulesPreset::Classic, false);
    let mut legacy_exit = serde_json::to_value(InputFrame::default()).unwrap();
    legacy_exit
        .as_object_mut()
        .unwrap()
        .insert("exit".into(), serde_json::Value::Bool(true));
    legacy_exit
        .as_object_mut()
        .unwrap()
        .insert("exit_pressed".into(), serde_json::Value::Bool(true));
    let legacy_exit = serde_json::from_value::<InputFrame>(legacy_exit).unwrap();

    game.step(&legacy_exit);
    assert_eq!(game.state, RunState::Playing);

    game.step(&InputFrame {
        up: true,
        up_pressed: true,
        ..InputFrame::default()
    });
    assert_eq!(game.state, RunState::Ending);
}

#[test]
fn actual_step_held_up_binding_survives_airborne_press_until_landing() {
    let mut game = Game::shell(41, RulesPreset::Classic);
    for x in 0..game.world.width {
        game.world.set_tile(x, 4, "oTemple");
    }
    game.world.spawn("oExit", Vec2::new(80., 48.));
    game.player.pos = Vec2::new(88., 32.);
    game.player.previous_pos = game.player.pos;
    game.player.vel = Vec2::default();
    game.player.state = crate::PlayerState::Falling;
    game.player.grounded = false;
    game.player.attack_timer = 0;

    // The Up edge arrives while the Player is above the door and is consumed
    // on this first simulation tick. Classic keeps sampling its held Up value,
    // so it exits once the Player lands with the key held.
    game.step(&InputFrame {
        up: true,
        up_pressed: true,
        ..InputFrame::default()
    });
    assert_eq!(game.state, RunState::Playing);
    assert!(!crate::player::source_platform_grounded(&game));

    for _ in 0..40 {
        if game.state != RunState::Playing {
            break;
        }
        game.step(&InputFrame {
            up: true,
            ..InputFrame::default()
        });
    }
    assert_eq!(game.state, RunState::Ending);
}

#[test]
fn actual_step_passive_damsel_deposit_uses_the_independent_mask_receiver() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for reverse_storage in [false, true] {
            let (mut game, _, _) = adjacent_exits(rules, reverse_storage);
            let damsel = game.spawn("oDamsel", Vec2::new(102., 54.));
            game.world.entity_mut(damsel).unwrap().held = true;
            game.player.held = Some(damsel);
            game.step(&InputFrame::default());
            assert_eq!(game.state, RunState::Playing);
            assert_eq!(game.player.pos, Vec2::new(98., 56.));
            assert_eq!(game.player.held, None);
            let rescued = game.world.entity(damsel).unwrap();
            assert_eq!(rescued.state, 4);
            assert_eq!(rescued.pos, Vec2::new(88., 56.));
            assert_eq!(game.lifetime_stats.damsels_saved_total, 1);
            assert_eq!(game.data["xdamsels"], 1.);
        }
    }
}

#[test]
fn actual_step_player_mask_overlap_alone_does_not_enable_exit_or_deposit() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for up in [false, true] {
            let (mut game, _, second) = adjacent_exits(rules, false);
            game.world.entity_mut(second).unwrap().alive = false;
            let idol = game.spawn("oGoldIdol", game.player.pos);
            game.world.entity_mut(idol).unwrap().held = true;
            game.player.held = Some(idol);
            game.step(&InputFrame {
                up,
                ..Default::default()
            });
            assert_eq!(game.state, RunState::Playing);
            assert_eq!(game.player.held, Some(idol));
            assert_eq!(game.lifetime_stats.idols_converted, 0);
        }
    }
}

fn walkoff_exit_game() -> Game {
    let mut game = Game::shell(23, RulesPreset::Classic);
    for x in 0..5 {
        game.world.set_tile(x, 4, "oTemple");
    }
    game.world.spawn("oExit", Vec2::new(80., 48.));
    game.player.pos = Vec2::new(72., 56.);
    game.player.previous_pos = game.player.pos;
    game.player.state = crate::PlayerState::Standing;
    game.player.grounded = true;
    game
}

#[test]
fn actual_step_exit_accepts_classic_ground_state_after_walking_off_support() {
    let mut game = walkoff_exit_game();
    let mut exited_at_walkoff = false;
    for _ in 0..40 {
        let mut probe = game.clone();
        probe.step(&InputFrame {
            right: true,
            ..Default::default()
        });
        if !probe.player.grounded && crate::player::source_platform_grounded(&probe) {
            // Replaying this source-grounded edge tick with Up must still be
            // eligible even though the post-move support query is now false.
            game.step(&InputFrame {
                right: true,
                up: true,
                ..Default::default()
            });
            assert!(!game.player.grounded);
            assert!(crate::player::source_platform_grounded(&game));
            assert!((80. ..96.).contains(&game.player.pos.x));
            assert_eq!(game.state, RunState::Ending);
            exited_at_walkoff = true;
            break;
        }
        game = probe;
    }
    assert!(exited_at_walkoff, "fixture must walk off inside the exit");
}

#[test]
fn actual_step_exit_waits_for_classic_landing_state_even_if_post_move_support_exists() {
    let mut game = Game::shell(29, RulesPreset::Classic);
    for x in 0..game.world.width {
        game.world.set_tile(x, 4, "oTemple");
    }
    game.world.spawn("oExit", Vec2::new(80., 48.));
    game.player.pos = Vec2::new(88., 55.);
    game.player.previous_pos = game.player.pos;
    game.player.vel = Vec2::new(0., 1.);
    game.player.state = crate::PlayerState::Falling;

    game.step(&InputFrame {
        up: true,
        ..Default::default()
    });

    assert!(
        game.player.grounded,
        "the post-move support probe sees the floor"
    );
    assert!(!crate::player::source_platform_grounded(&game));
    assert_eq!(game.player.state, crate::PlayerState::Falling);
    assert_eq!(game.state, RunState::Playing);
}
