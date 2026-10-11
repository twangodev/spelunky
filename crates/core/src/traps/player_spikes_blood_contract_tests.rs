use crate::{Area, Game, GameEvent, InputFrame, RulesPreset, Vec2, World};

#[test]
fn actual_fall_onto_authored_spikes_creates_three_blood_without_hurt_sound() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut g = Game::shell(7, rules);
        g.world = World::new(30, 20, Area::Mines, 1);
        g.world.name = "rLevel".into();
        for x in 0..30 {
            g.world.set_tile(x, 7, "oHardBlock");
        }
        g.player.pos = Vec2::new(88.0, 40.0);
        g.player.previous_pos = g.player.pos;
        g.spawn("oSpikes", Vec2::new(80.0, 96.0));
        let mut reached = false;
        for _ in 0..40 {
            let before = g.clone();
            let first = g.world.next_id;
            g.step(&InputFrame::default());
            if !g.events.contains(&GameEvent::Damage { amount: 99 }) {
                continue;
            }
            reached = true;
            assert_eq!(
                g.world
                    .entities
                    .iter()
                    .filter(|e| e.alive && e.id >= first && e.object == "oBlood")
                    .count(),
                3
            );
            assert!(
                !g.events
                    .iter()
                    .any(|e| matches!(e, GameEvent::Sound { name, .. } if name == "sndHurt"))
            );
            assert_eq!(g.player.health, 0);
            let mut callback = before;
            crate::player::tick(&mut callback, &InputFrame::default());
            let first = callback.world.next_id;
            let mut oracle = callback.rng.clone();
            for _ in 0..12 {
                oracle.next_u64();
            }
            crate::traps::player_spikes(&mut callback);
            let blood: Vec<_> = callback
                .world
                .entities
                .iter()
                .filter(|e| e.alive && e.id >= first && e.object == "oBlood")
                .collect();
            assert_eq!(blood.len(), 3);
            assert!(
                blood
                    .iter()
                    .all(|e| e.value("world_effect_initialized") == 1.0)
            );
            assert_eq!(callback.rng.clone().next_u64(), oracle.next_u64());
            assert_eq!(callback.player.health, -95);
            break;
        }
        assert!(reached, "natural fall must reach spike activation");
    }
}
