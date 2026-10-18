#[cfg(test)]
mod projectile_query_contract {
    use super::*;
    use crate::dispatcher::Instance;

    fn room(rules: RulesPreset) -> Game {
        let mut game = Game::shell(73, rules);
        game.world = World::new(80, 30, Area::Mines, 1);
        for x in 0..80 {
            game.world.set_tile(x, 10, "oHardBlock");
        }
        game.player.pos = Vec2::new(40.0, 152.0);
        game
    }

    fn projectile(game: &mut Game, object: &str, pos: Vec2, vx: f64) -> u64 {
        let id = game.spawn(object, pos);
        game.world.entity_mut(id).unwrap().vel.x = vx;
        id
    }

    fn hurt_sounds(game: &Game) -> usize {
        game.events
            .iter()
            .filter(|event| matches!(event, GameEvent::Sound { name, .. } if name == "sndHurt"))
            .count()
    }

    fn assert_hurt(game: &Game, expected: bool) {
        assert_eq!(game.player.health, if expected { 2.0 } else { 4.0 });
        assert_eq!(game.player.stun, if expected { 20 } else { 0 });
        assert_eq!(hurt_sounds(game), usize::from(expected));
    }

    // Real Level Step deactivates the distant actor; the subsequent callback
    // position change is controlled host-boundary setup, not ordinary input.
    fn inactive_actor(rules: RulesPreset, object: &str, vx: f64) -> (Game, u64) {
        let mut game = room(rules);
        let id = projectile(&mut game, object, Vec2::new(600.0, 152.0), vx);
        game.step(&InputFrame::default());
        let actor = game.world.entity(id).unwrap();
        assert!(!actor.engine_active);
        assert_eq!(actor.pos, Vec2::new(600.0, 152.0));
        assert_eq!(actor.vel.x, vx);
        assert_hurt(&game, false);
        game.player.pos = Vec2::new(600.0, 152.0);
        (game, id)
    }

    #[test]
    fn public_hurt_callback_ignores_real_step_deactivated_overlap() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            for object in ["oArrow", "oRock"] {
                let (mut game, id) = inactive_actor(rules, object, 6.0);
                let rng = game.rng.clone().next_u64();
                let events = game.events.len();
                player_hurt_queries(&mut game);
                assert_hurt(&game, false);
                assert!(game.world.entity(id).is_some());
                assert_eq!(game.events.len(), events);
                assert_eq!(game.rng.clone().next_u64(), rng);
                assert!(game.player.held.is_none());
            }
        }
    }

    #[test]
    fn public_hurt_callback_ignores_inactive_nearest_and_keeps_global_selection() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            for object in ["oArrow", "oRock"] {
                for active_fast in [false, true] {
                    let (mut game, inactive) =
                        inactive_actor(rules, object, if active_fast { 0.0 } else { 6.0 });
                    // One active actor overlaps the literal rectangle, while
                    // the closer active actor is outside it. Source nearest is
                    // global to the active family, not restricted to overlaps.
                    let provider_pos = if object == "oArrow" {
                        Vec2::new(612.0, 161.0)
                    } else {
                        Vec2::new(611.0, 163.0)
                    };
                    let provider = projectile(&mut game, object, provider_pos, 6.0);
                    let nearest = projectile(
                        &mut game,
                        object,
                        Vec2::new(600.0, 165.0),
                        if active_fast { 6.0 } else { 0.0 },
                    );
                    let first = Vec2::new(592.0, 144.0);
                    let second = Vec2::new(608.0, 160.0);
                    assert!(
                        game.world
                            .entity(provider)
                            .unwrap()
                            .collision_mask()
                            .intersects_rectangle_bbox(first, second)
                    );
                    assert!(
                        !game
                            .world
                            .entity(nearest)
                            .unwrap()
                            .collision_mask()
                            .intersects_rectangle_bbox(first, second)
                    );
                    assert!(game.world.entity(provider).unwrap().engine_active);
                    assert!(game.world.entity(nearest).unwrap().engine_active);
                    assert!(!game.world.entity(inactive).unwrap().engine_active);
                    player_hurt_queries(&mut game);
                    assert_hurt(&game, active_fast);
                    assert!(game.world.entity(inactive).is_some());
                    assert!(game.world.entity(provider).is_some());
                    assert_eq!(
                        game.world.entity(nearest).is_none(),
                        active_fast && object == "oArrow"
                    );
                    if active_fast {
                        assert_eq!(game.player.vel, Vec2::new(6.0, -4.0));
                    }
                }
            }
        }
    }

    #[test]
    fn game_step_rock_rectangle_uses_literal_rounded_mask_edges() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            for (xscale, x, hit) in [
                (1.0, 115.25, true),
                (1.0, 115.75, false),
                (1.0, 92.75, true),
                (1.0, 92.25, false),
                (-1.0, 117.0, true),
                (-1.0, 118.0, false),
                (-1.0, 93.0, true),
                (-1.0, 92.0, false),
            ] {
                let mut game = room(rules);
                game.player.pos = Vec2::new(104.0, 152.0);
                let id = projectile(&mut game, "oRock", Vec2::new(x, 152.0), 6.0);
                game.world
                    .entity_mut(id)
                    .unwrap()
                    .data
                    .insert("image_xscale".into(), xscale);
                // Positive sRock extrema are -3..3. Quarter-pixel positions
                // distinguish integer cache from the old player+9 float box.
                // Integer mirroring has source padded extrema -5..3: x117
                // genuinely hits while the old tight mirrored mask misses.
                // Fractional mirrors are deliberately outside this contract.
                game.step(&InputFrame::default());
                assert_hurt(&game, hit);
                assert!(game.world.entity(id).is_some());
            }
        }
    }

    #[test]
    fn game_step_arrow_rectangle_keeps_selected_mirrored_mask_edges() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            // Unmirrored fractional edges exercise the verified integer cache.
            // Fractional mirrors retain the existing unsupported-transform
            // fallback, so mirror controls below use integral positions only.
            // Original updater's inclusive negative-scale envelope is -5..4
            // for sArrowRight, rather than an intuitive reflected -3..4.
            for (xscale, x, hit) in [
                (1.0, 116.25, true),
                (1.0, 116.75, false),
                (1.0, 92.75, true),
                (1.0, 92.25, false),
                (-1.0, 117.0, true),
                (-1.0, 118.0, false),
                (-1.0, 92.0, true),
                (-1.0, 91.0, false),
            ] {
                let mut game = room(rules);
                game.player.pos = Vec2::new(104.0, 152.0);
                let id = projectile(&mut game, "oArrow", Vec2::new(x, 152.0), 6.0);
                game.world
                    .entity_mut(id)
                    .unwrap()
                    .data
                    .insert("image_xscale".into(), xscale);
                game.step(&InputFrame::default());
                assert_hurt(&game, hit);
                assert_eq!(game.world.entity(id).is_none(), hit);
            }
        }
    }

    #[test]
    fn callback_integer_nearest_tie_uses_actual_step_reactivation_order() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            for object in ["oArrow", "oRock"] {
                let mut game = room(rules);
                let older = projectile(&mut game, object, Vec2::new(600.0, 40.0), 6.0);
                let newer = projectile(&mut game, object, Vec2::new(620.0, 40.0), -6.0);
                game.step(&InputFrame::default());
                assert!(!game.world.entity(older).unwrap().engine_active);
                assert!(!game.world.entity(newer).unwrap().engine_active);
                // Controlled camera/player setup makes real Level callbacks
                // reactivate newer alone (left edge608), then older (584).
                // Both remain outside the item's tighter16-pixel body view.
                game.player.pos = Vec2::new(864.0, 152.0);
                game.camera = Vec2::new(704.0, 0.0);
                game.step(&InputFrame::default());
                assert!(!game.world.entity(older).unwrap().engine_active);
                assert!(game.world.entity(newer).unwrap().engine_active);
                game.player.pos = Vec2::new(840.0, 152.0);
                game.camera = Vec2::new(680.0, 0.0);
                game.step(&InputFrame::default());
                assert!(game.world.entity(older).unwrap().engine_active);
                assert!(game.world.entity(newer).unwrap().engine_active);
                assert_eq!(
                    game.world.entity(older).unwrap().pos,
                    Vec2::new(600.0, 40.0)
                );
                assert_eq!(
                    game.world.entity(newer).unwrap().pos,
                    Vec2::new(620.0, 40.0)
                );
                let family: Vec<_> = crate::dispatcher::descendant_instances(&game, object)
                    .into_iter()
                    .filter_map(|instance| match instance {
                        Instance::Entity(id) => Some(id),
                        _ => None,
                    })
                    .collect();
                let storage: Vec<_> = game
                    .world
                    .entities
                    .iter()
                    .filter(|e| e.object == object)
                    .map(|e| e.id)
                    .collect();
                assert_eq!(family, vec![newer, older]);
                assert_eq!(storage, vec![older, newer]);
                game.player.pos = Vec2::new(610.0, 40.0);
                player_hurt_queries(&mut game);
                assert_hurt(&game, true);
                assert_eq!(game.player.vel, Vec2::new(-6.0, -4.0));
                assert!(game.world.entity(older).is_some());
                assert_eq!(game.world.entity(newer).is_none(), object == "oArrow");
            }
        }
    }
}
