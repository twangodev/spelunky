//! Source-qualified initial callback controls, not natural campaign routes.
//! Declare as an enemies child cfg(test) module; no public API changes.
use super::*;

#[test]
fn ordinary_item_ufo_birth_prefix_and_literal_credit_ignore_counts_as_kill() {
    let mut saw_eject = false;
    let mut saw_no_eject = false;
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for seed in 0..16 {
            for real in [false, true] {
                for policy in 0..3 {
                    let mut g = Game::shell(seed, rules);
                    g.world = World::new(30, 20, Area::Mines, 1);
                    g.world.name = "rLevel".into();
                    if !real {
                        g.flags.insert("custom".into());
                    }
                    assert_eq!(crate::statistics::is_real_level(&g), real);
                    let ufo = g.spawn("oUFO", Vec2::new(80.0, 80.0));
                    match policy {
                        0 => {}
                        1 => {
                            g.world
                                .entity_mut(ufo)
                                .unwrap()
                                .data
                                .insert("counts_as_kill".into(), 0.0);
                        }
                        2 => {
                            g.world
                                .entity_mut(ufo)
                                .unwrap()
                                .data
                                .insert("environmental_death".into(), 1.0);
                        }
                        _ => unreachable!(),
                    }
                    let rock = g.spawn("oRock", Vec2::new(88.0, 88.0));
                    g.world.entity_mut(rock).unwrap().vel = Vec2::new(4.0, 0.0);
                    assert_eq!(g.world.entity(ufo).unwrap().health, 1);
                    assert_eq!(g.world.entity(ufo).unwrap().value("invincible"), 0.0);
                    assert_eq!(g.player.health, 4);
                    assert_eq!(g.world.entity(rock).unwrap().value("safe"), 0.0);
                    assert_eq!(item_impact_family(&g, "oEnemy", rock), vec![ufo]);
                    assert!(g.world.entity(ufo).unwrap().mask_intersects(
                        Rect::new(86.0, 86.0, 5.0, 5.0),
                        crate::CollisionPrecision::BoundingBox,
                    ));
                    // Literal source width3 roll; child AlienEject Create
                    // consumes its own width2 dir roll before caller credits.
                    let mut expected = g.rng.clone();
                    let eject = expected.range(3) == 0;
                    let facing = eject.then(|| if expected.range(2) == 0 { -1 } else { 1 });
                    saw_eject |= eject;
                    saw_no_eject |= !eject;
                    let first_birth = g.world.next_id;
                    g.events.clear(); // Initial callback-scene preparation only.
                    item_step_impacts(&mut g, rock);
                    // No actor/field/RNG writes after entering the real item query.
                    let births = g
                        .world
                        .entities
                        .iter()
                        .filter(|e| e.id >= first_birth)
                        .collect::<Vec<_>>();
                    assert_eq!(births.len(), 1 + usize::from(eject));
                    assert_eq!(births[0].object, "oExplosion");
                    assert_eq!(births[0].pos, Vec2::new(88.0, 88.0));
                    assert_eq!(births[0].value("announced"), 1.0);
                    if eject {
                        let child = births[1];
                        assert_eq!(child.object, "oAlienEject");
                        assert_eq!(child.pos, Vec2::new(88.0, 88.0));
                        assert_eq!(child.health, 1);
                        assert_eq!(child.state, 0);
                        assert_eq!(child.vel, Vec2::new(0.0, -6.0));
                        assert_eq!(child.facing, facing.unwrap());
                        assert_eq!(child.value("ice_context_created"), 1.0);
                        assert_eq!(child.value("enemy_context_created"), 1.0);
                    }
                    assert_eq!(g.rng.state_bits(), expected.state_bits());
                    let sounds = g
                        .events
                        .iter()
                        .filter_map(|e| match e {
                            GameEvent::Sound { name, .. } => Some(name.as_str()),
                            _ => None,
                        })
                        .collect::<Vec<_>>();
                    assert_eq!(sounds, vec!["sndExplosion", "sndExplosion"]);
                    assert!(g.world.entity(ufo).is_none());
                    assert_eq!(
                        g.data.get("ufos").copied().unwrap_or(0.0),
                        1.0,
                        "SOURCE_ITEM_UFO_UNCONDITIONAL_CREDIT policy={policy}"
                    );
                    assert_eq!(g.data.get("kills").copied().unwrap_or(0.0), 1.0);
                    assert_eq!(g.lifetime_stats.enemy_kills[16], u64::from(real));
                    let retired = g.world.entities.iter().find(|e| e.id == ufo).unwrap();
                    assert_eq!(retired.health, 1, "source branch never writes hp");
                    assert_eq!(g.player.health, 4);
                    assert!(g.world.entity(rock).is_some());
                }
            }
        }
    }
    assert!(
        saw_eject && saw_no_eject,
        "both literal producer branches covered"
    );
}

#[test]
fn ordinary_item_ufo_query_keeps_invincible_and_inactive_family_guards() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for inactive in [false, true] {
            let mut g = Game::shell(16, rules);
            g.world = World::new(30, 20, Area::Ice, 9);
            g.world.name = "rLevel".into();
            let ufo = g.spawn("oUFO", Vec2::new(80.0, 80.0));
            let e = g.world.entity_mut(ufo).unwrap();
            if inactive {
                e.engine_active = false;
            } else {
                e.data.insert("invincible".into(), 1.0);
            }
            let rock = g.spawn("oRock", Vec2::new(88.0, 88.0));
            g.world.entity_mut(rock).unwrap().vel = Vec2::new(4.0, 0.0);
            let first_birth = g.world.next_id;
            let rng = g.rng.state_bits();
            g.events.clear();
            item_step_impacts(&mut g, rock);
            assert_eq!(g.world.next_id, first_birth);
            assert_eq!(g.rng.state_bits(), rng);
            assert!(g.events.is_empty());
            assert_eq!(g.world.entity(ufo).unwrap().health, 1);
            assert_eq!(g.data.get("ufos").copied().unwrap_or(0.0), 0.0);
            assert_eq!(g.data.get("kills").copied().unwrap_or(0.0), 0.0);
            assert_eq!(g.lifetime_stats.enemy_kills[16], 0);
            assert_eq!(g.player.health, 4);
        }
    }
}
