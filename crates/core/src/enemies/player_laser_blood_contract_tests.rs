use crate::{Area, Game, GameEvent, InputFrame, RulesPreset, Vec2, World};

#[test]
fn actual_default_ufo_laser_creates_player_blood_before_impact_then_hurt_sound() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut g = Game::shell(7, rules);
        g.world = World::new(30, 20, Area::Ice, 9);
        g.world.name = "rLevel".into();
        for x in 0..30 {
            g.world.set_tile(x, 10, "oHardBlock");
        }
        g.player.pos = Vec2::new(88.0, 152.0);
        g.player.previous_pos = g.player.pos;
        g.player.grounded = true;
        g.spawn("oUFO", Vec2::new(80.0, 96.0));
        let mut reached = false;
        for _ in 0..160 {
            let before = g.clone();
            let first = g.world.next_id;
            g.step(&InputFrame::default());
            if !g.events.contains(&GameEvent::Damage { amount: 3 }) {
                continue;
            }
            reached = true;
            assert_eq!(
                before
                    .world
                    .entities
                    .iter()
                    .filter(|e| e.alive
                        && e.engine_active
                        && crate::catalog::is_a(&e.object, "oDetritus"))
                    .count(),
                0
            );
            let blood: Vec<_> = g
                .world
                .entities
                .iter()
                .filter(|e| e.alive && e.id >= first && e.object == "oBlood")
                .collect();
            assert_eq!(
                blood.len(),
                6,
                "Laser Blood3 precedes the later grounded stun-bounce Blood3"
            );
            assert_eq!(g.player.data.get("bounced"), Some(&1.0));
            assert!(
                blood
                    .iter()
                    .all(|e| e.value("world_effect_initialized") == 1.0)
            );
            let impact = g
                .events
                .iter()
                .position(
                    |e| matches!(e, GameEvent::Sound { name, .. } if name == "sndSmallExplode"),
                )
                .unwrap();
            let hurt = g
                .events
                .iter()
                .position(|e| matches!(e, GameEvent::Sound { name, .. } if name == "sndHurt"))
                .unwrap();
            assert!(impact < hurt);
            assert_eq!(g.player.health, 1.0);
            assert_eq!(g.player.stun, 20);
            // Supplemental source callback-context witness from naturally
            // reached state: stop before later object/Blood Normal Steps.
            let mut callback = before;
            crate::player::tick(&mut callback, &InputFrame::default());
            let first = callback.world.next_id;
            let mut oracle = callback.rng.clone();
            let expected: Vec<_> = (0..3)
                .map(|_| {
                    let random = |rng: &mut crate::Rng, n: f64| {
                        (rng.next_u64() as f64 / 4_294_967_296.0) * n
                    };
                    let velocity = Vec2::new(
                        random(&mut oracle, 4.0) - random(&mut oracle, 4.0),
                        -1.0 - random(&mut oracle, 2.0),
                    );
                    let gravity = (1 + ((oracle.next_u64() * 6) >> 32)) as f64 * 0.1;
                    (velocity, gravity)
                })
                .collect();
            crate::enemies::player_hurt(&mut callback);
            let blood: Vec<_> = callback
                .world
                .entities
                .iter()
                .filter(|e| e.alive && e.id >= first && e.object == "oBlood")
                .collect();
            assert_eq!(blood.len(), 3);
            for (particle, (velocity, gravity)) in blood.iter().zip(expected) {
                assert_eq!(particle.pos, callback.player.pos);
                assert_eq!(particle.vel, velocity);
                assert_eq!(particle.value("gravity"), gravity);
            }
            assert_eq!(callback.rng.clone().next_u64(), oracle.next_u64());
            break;
        }
        assert!(reached, "default UFO must produce a natural laser hit");
    }
}
