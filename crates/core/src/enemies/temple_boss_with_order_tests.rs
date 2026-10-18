//! Authored multiple-Worship arena, followed only by neutral Game.step.
//! Olmec's normal intro pan produces deactivation/reactivation and his natural
//! alarms reach each with statement; no arrays, timers or storage are rewritten.
use super::*;
use crate::{Area, InputFrame, PlayerState, RulesPreset};

#[test]
fn actual_olmec_intro_reactivation_orders_worship_replacement_births() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (family, replacement, alarm) in [
            ("oHawkmanWorship", "oHawkman", 1),
            ("oCavemanWorship", "oCaveman", 6),
        ] {
            let mut game = Game::shell(17, rules);
            game.world = World::new(50, 55, Area::Boss, 16);
            game.world.name = "rLevel".into();
            for x in 0..50 {
                game.world.set_tile(x, 35, "oHardBlock");
            }
            game.player.pos = Vec2::new(24.0, 552.0);
            game.player.previous_pos = game.player.pos;
            game.player.grounded = true;
            game.player.state = PlayerState::Standing;
            game.world.entrance = game.player.pos;
            let far = game.spawn(family, Vec2::new(420.0, 544.0));
            let near = game.spawn(family, Vec2::new(192.0, 544.0));
            let inactive = game.spawn(family, Vec2::new(700.0, 544.0));
            let boss = game.spawn("oOlmec", Vec2::new(288.0, 496.0));
            game.step(&InputFrame::default());
            assert!(!game.world.entity(far).unwrap().engine_active);
            for _ in 0..400 {
                if game.world.entity(boss).unwrap().alarms[alarm] == 1 {
                    break;
                }
                game.step(&InputFrame::default());
            }
            assert_eq!(game.world.entity(boss).unwrap().alarms[alarm], 1);
            assert!(game.world.entity(far).unwrap().engine_active);
            assert!(!game.world.entity(inactive).unwrap().engine_active);
            assert_eq!(
                crate::dispatcher::descendant_instances(&game, family),
                vec![Instance::Entity(near), Instance::Entity(far)]
            );
            assert_eq!(
                game.world
                    .entities
                    .iter()
                    .filter(|e| e.object == family && e.engine_active)
                    .map(|e| e.id)
                    .collect::<Vec<_>>(),
                vec![far, near]
            );
            game.step(&InputFrame::default());
            let mut births: Vec<_> = game
                .world
                .entities
                .iter()
                .filter(|e| e.object == replacement)
                .collect();
            births.sort_by_key(|e| e.creation_serial);
            assert_eq!(births.len(), 2);
            assert_eq!(births[0].previous_pos, Vec2::new(192.0, 544.0), "{family}");
            assert_eq!(births[1].previous_pos, Vec2::new(420.0, 544.0), "{family}");
            assert!(game.world.entity(far).is_none());
            assert!(game.world.entity(near).is_none());
            assert!(!game.world.entity(inactive).unwrap().engine_active);
        }
    }
}
