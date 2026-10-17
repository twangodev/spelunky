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
