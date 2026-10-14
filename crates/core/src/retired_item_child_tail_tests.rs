//! Configured entered-event witnesses, not a generated campaign or throw route.
use super::*;

fn scene(rules: RulesPreset) -> Game {
    let mut game = Game::shell(42, rules);
    game.world = World::new(40, 24, Area::Mines, 1);
    game.world.name = "configured".into();
    for x in 0..40 {
        game.world.set_tile(x, 13, "oHardBlock");
    }
    game.player.pos = Vec2::new(24.0, 200.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.world.entrance = game.player.pos;
    game
}

#[test]
fn configured_sticky_lava_destroy_finishes_attach_without_a_second_cleanup() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (lava, speed, active, linked) in [
            (true, 3.0, true, true),
            (true, 2.0, true, false),
            (true, 3.0, false, false),
            (false, 3.0, true, true),
        ] {
            let mut game = scene(rules);
            let snake = game.spawn("oSnake", Vec2::new(154.0, 96.0));
            game.world.entity_mut(snake).unwrap().engine_active = active;
            if lava {
                game.spawn("oLava", Vec2::new(160.0, 80.0));
            }
            let bomb = game.spawn("oBomb", Vec2::new(160.0 - speed, 100.0));
            let receiver = game.world.entity_mut(bomb).unwrap();
            receiver.vel.x = speed;
            // Initial configured Paste property; no mid-event mutation.
            receiver.data.insert("sticky".into(), 1.0);
            assert!(item_body_active(&game, game.world.entity(bomb).unwrap()));
            assert!(
                game.world
                    .entity(snake)
                    .unwrap()
                    .mask_contains(Vec2::new(160.0, 100.0), CollisionPrecision::BoundingBox)
            );
            // Execute precisely the entered Item/child callback, before later
            // automatic Explosion collisions or endframe storage compaction.
            // Match the real entered Normal Step. Idle's retained legacy
            // adapter applies Explosion contacts immediately; native Create
            // does not, so that compatibility route is not this witness.
            game.dispatcher.phase = crate::dispatcher::Phase::NormalStep;
            step_instance(&mut game, bomb);
            let receiver = game.world.entities.iter().find(|e| e.id == bomb).unwrap();
            assert_eq!(receiver.alive, !lava);
            assert_eq!(receiver.pos, Vec2::new(160.0, 100.0));
            assert_eq!(receiver.vel.x, speed, "the Lava damping rectangle misses");
            assert_eq!(receiver.value("bomb_destroyed"), f64::from(lava));
            assert_eq!(
                receiver.value("attached_id"),
                if linked { snake as f64 } else { 0.0 }
            );
            assert_eq!(
                game.world.entity(snake).unwrap().value("bomb_id"),
                if linked { bomb as f64 } else { 0.0 }
            );
            assert_eq!(
                game.world.entity(snake).unwrap().health,
                1,
                "sticky branch replaces Item damage"
            );
            // Existing endframe fallback must not rerun the earlier callback
            // and clear the backlink written by continuation after Destroy.
            explosives::finish_sticky_destroy_events(&mut game);
            assert_eq!(
                game.world.entity(snake).unwrap().value("bomb_id"),
                if linked { bomb as f64 } else { 0.0 }
            );
            assert_eq!(game.player.health, 4);
        }
        let mut game = scene(rules);
        let old = game.spawn("oSnake", Vec2::new(400.0, 96.0));
        let nearby = game.spawn("oSnake", Vec2::new(154.0, 96.0));
        game.spawn("oLava", Vec2::new(160.0, 80.0));
        let bomb = game.spawn("oBomb", Vec2::new(157.0, 100.0));
        let receiver = game.world.entity_mut(bomb).unwrap();
        receiver.vel.x = 3.0;
        receiver.data.insert("sticky".into(), 1.0);
        receiver.data.insert("attached_id".into(), old as f64);
        game.world
            .entity_mut(old)
            .unwrap()
            .data
            .insert("bomb_id".into(), bomb as f64);
        game.dispatcher.phase = crate::dispatcher::Phase::NormalStep;
        step_instance(&mut game, bomb);
        let receiver = game.world.entities.iter().find(|e| e.id == bomb).unwrap();
        assert!(!receiver.alive);
        assert_eq!(receiver.value("bomb_destroyed"), 1.0);
        assert_eq!(
            receiver.value("attached_id"),
            old as f64,
            "Destroy does not reset source enemyID"
        );
        assert_eq!(game.world.entity(old).unwrap().value("bomb_id"), 0.0);
        assert_eq!(
            game.world.entity(nearby).unwrap().value("bomb_id"),
            0.0,
            "nonzero enemyID prevents reattachment"
        );
    }
}

#[test]
fn configured_fish_bone_own_angle_runs_after_retired_parent() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (lava, speed) in [(true, -3.0), (true, 3.0), (false, -3.0)] {
            let mut game = scene(rules);
            if lava {
                game.spawn("oLava", Vec2::new(160.0, 80.0));
            }
            let bone = game.spawn("oFishBone", Vec2::new(160.0 - speed, 100.0));
            game.world.entity_mut(bone).unwrap().vel.x = speed;
            assert!(item_body_active(&game, game.world.entity(bone).unwrap()));
            step_instance(&mut game, bone);
            let receiver = game.world.entities.iter().find(|e| e.id == bone).unwrap();
            assert_eq!(receiver.alive, !lava);
            assert_eq!(receiver.pos, Vec2::new(160.0, 100.0));
            assert_eq!(receiver.vel, Vec2::new(speed, 0.2));
            let acute = (0.2_f64 / 3.0).atan();
            let expected = if speed < 0.0 {
                std::f64::consts::PI + acute
            } else {
                acute
            };
            assert_eq!(
                receiver.value("rotation"),
                -expected,
                "own quadrant tail follows inherited Destroy"
            );
        }
    }
}

#[test]
fn configured_default_idol_lava_retirement_keeps_unsupported_trigger_credit() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (lava, trigger) in [(true, true), (false, true), (true, false)] {
            let mut game = scene(rules);
            if lava {
                game.spawn("oLava", Vec2::new(160.0, 80.0));
            }
            let idol = game.spawn("oGoldIdol", Vec2::new(157.0, 100.0));
            assert_eq!(
                game.world.entity(idol).unwrap().value("trigger"),
                1.0,
                "literal own Create"
            );
            game.world.entity_mut(idol).unwrap().vel.x = 3.0;
            if !trigger {
                game.world
                    .entity_mut(idol)
                    .unwrap()
                    .data
                    .insert("trigger".into(), 0.0);
            }
            assert!(item_body_active(&game, game.world.entity(idol).unwrap()));
            if lava {
                let lava = game
                    .world
                    .entities
                    .iter()
                    .find(|e| e.alive && e.object == "oLava")
                    .unwrap();
                assert!(
                    lava.mask_contains(Vec2::new(160.0, 95.0), CollisionPrecision::BoundingBox)
                );
                assert!(!lava.mask_intersects(
                    Rect::new(157.0, 97.0, 7.0, 7.0),
                    CollisionPrecision::BoundingBox
                ));
            }
            // This witness uses the whole ordinary tick after initial setup.
            game.step(&InputFrame::default());
            assert_eq!(game.world.entity(idol).is_none(), lava);
            assert_eq!(
                game.data.get("idols_grabbed").copied().unwrap_or(0.0),
                f64::from(trigger)
            );
            assert_eq!(game.lifetime_stats.idols_grabbed, u64::from(trigger));
            if !lava {
                assert_eq!(game.world.entity(idol).unwrap().value("trigger"), 0.0);
            }
            assert_eq!(game.player.health, 4);
            game.step(&InputFrame::default());
            assert_eq!(
                game.data.get("idols_grabbed").copied().unwrap_or(0.0),
                f64::from(trigger)
            );
            assert_eq!(game.lifetime_stats.idols_grabbed, u64::from(trigger));
        }
    }
}
