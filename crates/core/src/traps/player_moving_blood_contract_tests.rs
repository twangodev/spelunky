use crate::{Area, Game, GameEvent, InputFrame, Rng, RulesPreset, Vec2, World};

fn room(rules: RulesPreset) -> Game {
    let mut g = Game::shell(7, rules);
    g.world = World::new(30, 20, Area::Mines, 1);
    g.world.name = "rLevel".into();
    for x in 0..30 {
        g.world.set_tile(x, 7, "oHardBlock");
    }
    g.player.pos = Vec2::new(88.0, 88.0);
    g.player.previous_pos = g.player.pos;
    g
}

fn literal_blood(rng: &mut Rng) -> (Vec2, f64) {
    fn random(rng: &mut Rng, n: f64) -> f64 {
        ((rng.next_u64() >> 11) as f64 / (1_u64 << 53) as f64) * n
    }
    let velocity = Vec2::new(random(rng, 4.0) - random(rng, 4.0), -1.0 - random(rng, 2.0));
    let gravity = (1 + rng.next_u64() % 6) as f64 * 0.1;
    (velocity, gravity)
}

#[test]
fn player_crush_callback_creates_blood_before_return_and_honors_nonzero_bloodless() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (existing, bloodless) in [(0, 0.0), (15, 0.0), (16, 0.0), (0, 2.0), (0, -1.0)] {
            let mut g = room(rules);
            g.world.set_tile(5, 5, "oHardBlock");
            g.player.data.insert("bloodless".into(), bloodless);
            for _ in 0..existing {
                g.spawn("oBlood", Vec2::new(240.0, 80.0));
            }
            let first = g.world.next_id;
            let admitted = if bloodless != 0.0 {
                0
            } else {
                (16_usize - existing).min(3)
            };
            let mut oracle = g.rng.clone();
            let expected: Vec<_> = (0..admitted).map(|_| literal_blood(&mut oracle)).collect();
            crate::traps::player_crush(&mut g);
            let blood: Vec<_> = g
                .world
                .entities
                .iter()
                .filter(|e| e.alive && e.id >= first && e.object == "oBlood")
                .collect();
            assert_eq!(blood.len(), admitted);
            for (particle, (velocity, gravity)) in blood.iter().zip(expected) {
                assert_eq!(particle.pos, g.player.pos);
                assert_eq!(
                    particle.vel, velocity,
                    "Create must finish before Crush returns"
                );
                assert_eq!(particle.value("gravity"), gravity);
                assert_eq!(particle.value("world_effect_initialized"), 1.0);
            }
            assert_eq!(g.rng.clone().next_u64(), oracle.next_u64());
            assert_eq!(g.player.health, -95);
            assert_eq!(g.player.data.get("hidden"), Some(&1.0));
            assert!(
                g.events
                    .iter()
                    .any(|e| matches!(e, GameEvent::Sound { name, .. } if name == "sndDie"))
            );
        }
    }
}

#[test]
fn actual_default_smash_overlap_runs_crush_blood_before_smash_velocity_draw() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut g = room(rules);
        let trap = g.spawn("oSmashTrap", Vec2::new(80.0, 80.0));
        assert_eq!(g.world.entity(trap).unwrap().state, 0);
        let first = g.world.next_id;
        let mut oracle = g.rng.clone();
        let mut expected: Vec<_> = (0..3).map(|_| literal_blood(&mut oracle)).collect();
        let speed = (4 + oracle.next_u64() % 3) as f64;
        expected.push(literal_blood(&mut oracle));
        g.step(&InputFrame::default());
        assert!(g.events.contains(&GameEvent::Damage { amount: 99 }));
        assert!(g.events.contains(&GameEvent::Damage { amount: 10 }));
        let blood: Vec<_> = g
            .world
            .entities
            .iter()
            .filter(|e| e.alive && e.id >= first && e.object == "oBlood")
            .collect();
        assert_eq!(blood.len(), 4);
        for (particle, (_, gravity)) in blood.iter().zip(expected) {
            assert_eq!(
                particle.value("gravity"),
                gravity,
                "Crush child RNG precedes Smash velocity RNG"
            );
        }
        assert_eq!(g.player.vel.x, speed);
        assert_eq!(g.rng.clone().next_u64(), oracle.next_u64());
        assert_eq!(g.player.health, 0);
    }
}
