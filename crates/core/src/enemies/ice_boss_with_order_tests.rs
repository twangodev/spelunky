//! Authored multiple-emitter room; ordinary walking produces the membership
//! reorder. Only the later Boss death entry is a configured callback control.
//! Source AlienBoss Animation end uses the current with(oBarrierEmitter) array.
use super::*;
use crate::{Area, InputFrame, RulesPreset, World};

#[test]
fn actual_scroll_reactivation_orders_alien_boss_emitter_destroy_callbacks() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = Game::shell(17, rules);
        game.world = World::new(50, 20, Area::Ice, 9);
        game.world.name = "rLevel".into();
        for x in 0..50 {
            game.world.set_tile(x, 10, "oHardBlock");
        }
        game.world.set_tile(26, 7, "oHardBlock");
        game.world.set_tile(12, 7, "oHardBlock");
        game.player.pos = Vec2::new(160.0, 152.0);
        game.player.previous_pos = game.player.pos;
        game.player.grounded = true;
        game.player.state = PlayerState::Standing;
        game.world.entrance = game.player.pos;
        // Birth/storage order starts far then near. No source arrays or
        // physical entity storage are rewritten to manufacture the reorder.
        let far = game.spawn("oBarrierEmitter", Vec2::new(420.0, 128.0));
        let near = game.spawn("oBarrierEmitter", Vec2::new(192.0, 128.0));
        let boss = game.spawn("oAlienBoss", Vec2::new(96.0, 128.0));
        game.step(&InputFrame::default());
        assert!(!game.world.entity(far).unwrap().engine_active);
        for _ in 0..24 {
            game.step(&InputFrame {
                right: true,
                ..Default::default()
            });
            if game.world.entity(far).unwrap().engine_active {
                break;
            }
        }
        assert!(game.world.entity(far).unwrap().engine_active);
        assert_eq!(
            crate::dispatcher::descendant_instances(&game, "oBarrierEmitter"),
            vec![
                crate::dispatcher::Instance::Entity(near),
                crate::dispatcher::Instance::Entity(far)
            ]
        );
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|e| e.object == "oBarrierEmitter")
                .map(|e| e.id)
                .collect::<Vec<_>>(),
            vec![far, near]
        );
        game.world.entity_mut(boss).unwrap().health = 0;
        let mut destruction_sounds = Vec::new();
        for _ in 0..80 {
            game.step(&InputFrame::default());
            destruction_sounds.extend(game.events.iter().filter_map(|event| match event {
                GameEvent::Sound { name, pos } if name == "sndSmallExplode" => Some(*pos),
                _ => None,
            }));
            if game.world.entity(boss).unwrap().sprite == "sAlienBossDead" {
                break;
            }
        }
        assert_eq!(game.world.entity(boss).unwrap().sprite, "sAlienBossDead");
        assert_eq!(
            destruction_sounds,
            vec![Vec2::new(192.0, 128.0), Vec2::new(420.0, 128.0)]
        );
        assert!(game.world.entity(near).is_none());
        assert!(game.world.entity(far).is_none());
    }
}
