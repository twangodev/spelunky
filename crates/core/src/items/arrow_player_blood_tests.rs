#[cfg(test)]
mod arrow_player_blood_source_tests {
    use super::*;

    // Literal oBlood Create after inherited Detritus defaults/cap: three
    // velocity random() calls followed by its one rand(1,6).
    // This is an ownhurt-prefix oracle, not a whole-frame random stream claim.
    fn literal_create(rng: &mut Rng) -> (Vec2, f64) {
        (
            Vec2::new(rng.unit() * 4.0 - rng.unit() * 4.0, -1.0 - rng.unit() * 2.0),
            f64::from(1 + rng.range(6)) * 0.1,
        )
    }

    #[test]
    fn arrow_ownhurt_literal_blood_caps_dead_stun_held_destroy_and_rng() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            for (active, inactive) in [(0, 0), (15, 0), (16, 0), (0, 16), (15, 16), (16, 16)] {
                for bloodless in [false, true] {
                    for prior_dead in [false, true] {
                        for held in [false, true] {
                            for health in [8.0, 0.0] {
                                let mut game = Game::shell(7, rules);
                                game.world.name = "rLevel".into();
                                game.player.pos = Vec2::new(104.0, 120.0);
                                game.player.health = health;
                                game.player.stun = 9;
                                game.player.invincible = 99;
                                game.player
                                    .data
                                    .insert("bloodless".into(), f64::from(bloodless));
                                if prior_dead {
                                    game.player.state = PlayerState::Dead;
                                    game.state = RunState::Dead;
                                }
                                let mut bone_ids = Vec::new();
                                for _ in 0..active + inactive {
                                    bone_ids.push(game.spawn("oBone", Vec2::new(200.0, 80.0)));
                                }
                                for &id in bone_ids.iter().skip(active) {
                                    game.world.entity_mut(id).unwrap().engine_active = false;
                                }
                                assert_eq!(
                                    game.world
                                        .entities
                                        .iter()
                                        .filter(|e| e.alive
                                            && e.engine_active
                                            && crate::catalog::is_a(&e.object, "oDetritus"))
                                        .count(),
                                    active
                                );
                                let id = game.spawn("oArrow", game.player.pos);
                                let mut arrow = game.world.entity(id).unwrap().clone();
                                arrow.vel.x = 12.0;
                                arrow.held = held;
                                game.player.held = Some(id);
                                game.player.pickup_item_type = "Arrow".into();
                                game.data.insert("stored_item".into(), 17.0);
                                let saved_data = game.data.clone();
                                let mut literal = game.rng.clone();
                                let expected_count = if bloodless {
                                    0
                                } else {
                                    (16_usize.saturating_sub(active)).min(3)
                                };
                                let expected: Vec<_> = (0..expected_count)
                                    .map(|_| literal_create(&mut literal))
                                    .collect();

                                hit_player(&mut game, &mut arrow, 2);

                                assert_eq!(
                                    game.player.health,
                                    if health > 0.0 { health - 2.0 } else { health }
                                );
                                assert_eq!(game.player.vel, Vec2::new(12.0, -4.0));
                                assert_eq!(game.player.stun, 20);
                                assert_eq!(game.player.invincible, 99);
                                assert!(!arrow.alive);
                                assert_eq!(
                                    arrow.held, held,
                                    "Item Destroy clears only the player backlink"
                                );
                                assert_eq!(game.player.held, if held { None } else { Some(id) });
                                assert_eq!(game.player.pickup_item_type, "Arrow");
                                assert_eq!(
                                    game.data, saved_data,
                                    "Arrow Destroy must not restore or mutate a stash"
                                );
                                let blood: Vec<_> = game
                                    .world
                                    .entities
                                    .iter()
                                    .filter(|e| e.object == "oBlood")
                                    .collect();
                                assert_eq!(blood.len(), expected_count);
                                for (actual, (velocity, gravity)) in blood.iter().zip(expected) {
                                    assert!(actual.alive && actual.engine_active);
                                    assert_eq!(actual.pos, game.player.pos);
                                    assert_eq!(actual.vel, velocity);
                                    assert_eq!(actual.value("gravity"), gravity);
                                    assert_eq!(actual.alarms[0], 1);
                                    assert_eq!(actual.alarms[1], 1);
                                    assert_eq!(actual.alarms[2], 5);
                                }
                                assert_eq!(game.events.iter().filter(|e| matches!(e, GameEvent::Sound { name, .. } if name == "sndHurt")).count(), 1);
                                assert_eq!(
                                    game.events
                                        .iter()
                                        .filter(|e| matches!(e, GameEvent::Damage { amount: 2 }))
                                        .count(),
                                    usize::from(health > 0.0)
                                );
                                assert_eq!(game.rng.clone().next_u64(), literal.next_u64());
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn arrow_direct_exit_guard_and_dispatcher_outer_health_visibility_controls() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            for sprite in ["sPExit", "sDamselExit", "sTunnelExit"] {
                let mut game = Game::shell(7, rules);
                game.player.set_sprite(sprite);
                let mut arrow = Entity::new(100, "oArrow", game.player.pos);
                arrow.vel.x = 12.0;
                let rng = game.rng.clone().next_u64();
                hit_player(&mut game, &mut arrow, 2);
                assert!(arrow.alive);
                assert_eq!(game.player.health, 4.0);
                assert_eq!(game.player.stun, 0);
                assert!(game.events.is_empty());
                assert!(!game.world.entities.iter().any(|e| e.object == "oBlood"));
                assert_eq!(game.rng.next_u64(), rng);
            }
            for health in [-100, -99, 0, 4] {
                for hidden in [false, true] {
                    let mut game = Game::shell(7, rules);
                    game.player.health = f64::from(health);
                    game.player.state = PlayerState::Dead;
                    game.state = RunState::Dead;
                    game.player.stun = 9;
                    game.player.data.insert("hidden".into(), f64::from(hidden));
                    assert_eq!(
                        crate::traps::player_hurt_enabled(&game),
                        health >= -99 && !hidden
                    );
                }
            }
        }
    }
}
