//! Configured callback witnesses. Direct callbacks stop before automatic
//! collisions/compaction; they are not recorded ordinary throw routes.
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
    game.dispatcher.phase = crate::dispatcher::Phase::NormalStep;
    game
}
fn raw(game: &Game, id: u64) -> &Entity {
    game.world
        .entities
        .iter()
        .find(|entity| entity.id == id)
        .unwrap()
}
fn sound_count(game: &Game, name: &str) -> usize {
    game.events
        .iter()
        .filter(|event| {
            matches!(event,
        GameEvent::Sound { name: observed, .. } if observed == name)
        })
        .count()
}

#[test]
fn configured_retired_dice_finishes_hit_blood_before_rolling_rng() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (lava, speed, invincible, hit) in [
            (true, 4.0, false, true),
            (true, 3.0, false, false),
            (true, 4.0, true, false),
            (false, 4.0, false, true),
        ] {
            let mut game = scene(rules);
            let snake = game.spawn("oSnake", Vec2::new(154.0, 96.0));
            game.world
                .entity_mut(snake)
                .unwrap()
                .data
                .insert("invincible".into(), f64::from(invincible));
            if lava {
                game.spawn("oLava", Vec2::new(160.0, 80.0));
            }
            let dice = game.spawn("oDice", Vec2::new(160.0 - speed, 100.0));
            game.world.entity_mut(dice).unwrap().vel.x = speed;
            let before = game.rng.clone();
            assert!(item_body_active(&game, raw(&game, dice)));
            step_instance(&mut game, dice);
            assert_eq!(raw(&game, dice).alive, !lava);
            assert_eq!(raw(&game, dice).pos, Vec2::new(160.0, 100.0));
            assert_eq!(raw(&game, dice).vel, Vec2::new(speed, 0.6));
            assert_eq!(
                game.world.entity(snake).unwrap().health,
                if hit { 0 } else { 1 }
            );
            let blood: Vec<_> = game
                .world
                .entities
                .iter()
                .filter(|e| e.object == "oBlood")
                .collect();
            assert_eq!(blood.len(), usize::from(hit));
            if hit {
                assert_eq!(blood[0].pos, Vec2::new(162.0, 104.0));
                assert!(blood[0].alive);
                assert_eq!(game.world.entity(snake).unwrap().vel.x, speed * 0.3);
                assert_eq!(game.world.entity(snake).unwrap().value("assaulted"), 0.0);
            }
            let mut expected = before;
            if hit {
                for _ in 0..4 {
                    expected.unit();
                }
            }
            let value = f64::from(1 + expected.range(6));
            assert_eq!(
                game.rng.state_bits(),
                expected.state_bits(),
                "direct Blood Create before rolling draw"
            );
            assert_eq!(raw(&game, dice).value("value"), value);
            assert_eq!(raw(&game, dice).sprite, "sDiceRoll");
            assert_eq!(sound_count(&game, "sndHit"), usize::from(hit));
            assert_eq!(game.player.health, 4);
        }
    }
}

#[test]
fn configured_dice_enemy_switch_uses_direct_creates_and_literal_default_branches() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (object, position, offset, state) in [
            ("oCaveman", Vec2::new(154.0, 96.0), Vec2::default(), 98),
            ("oYeti", Vec2::new(154.0, 96.0), Vec2::default(), 98),
            ("oHawkman", Vec2::new(154.0, 96.0), Vec2::default(), 98),
            ("oShopkeeper", Vec2::new(154.0, 96.0), Vec2::default(), 2),
            (
                "oGiantSpider",
                Vec2::new(144.0, 80.0),
                Vec2::new(16.0, 24.0),
                0,
            ),
            (
                "oAlienBoss",
                Vec2::new(160.0, 100.0),
                Vec2::new(8.0, 8.0),
                98,
            ),
            // These are default Dice branches, NOT Item's Vampire/Tomb/UFO cases.
            ("oVampire", Vec2::new(154.0, 96.0), Vec2::new(8.0, 8.0), 0),
            ("oTombLord", Vec2::new(144.0, 84.0), Vec2::new(8.0, 8.0), 0),
            ("oUFO", Vec2::new(154.0, 96.0), Vec2::new(8.0, 8.0), 0),
            (
                "oGiantSpiderHang",
                Vec2::new(144.0, 96.0),
                Vec2::new(8.0, 8.0),
                0,
            ),
        ] {
            let mut game = scene(rules);
            let target = game.spawn(object, position);
            // Explicit source branch state, with Create HP/sprite/bloodless retained.
            game.world.entity_mut(target).unwrap().state = state;
            if object == "oGiantSpider" {
                // Original GiantSpider Create sets whipped=10. This configured
                // hit case enters Dice's successful whipped==0 branch, after
                // that initial recovery interval; no intervening Steps are claimed.
                assert_eq!(raw(&game, target).value("hit_cooldown"), 10.0);
                game.world
                    .entity_mut(target)
                    .unwrap()
                    .data
                    .insert("hit_cooldown".into(), 0.0);
                assert_eq!(raw(&game, target).value("hit_cooldown"), 0.0);
            }
            // Source collision_rectangle(x-2,y-2,x+2,y+2,oEnemy,0,0):
            // active Enemy membership and the existing raw BBox gate below
            // qualify this callback; no precise-alpha or notme=true substitution.
            assert!(crate::catalog::is_a(&raw(&game, target).object, "oEnemy"));
            assert!(raw(&game, target).engine_active);
            if matches!(object, "oCaveman" | "oYeti" | "oHawkman") {
                game.world.entity_mut(target).unwrap().state = 0;
            }
            let before_state = raw(&game, target).state;
            let hp = raw(&game, target).health;
            assert!(
                raw(&game, target)
                    .collision_mask()
                    .intersects_rectangle_bbox(Vec2::new(158.0, 98.0), Vec2::new(162.0, 102.0))
            );
            let dice = game.spawn("oDice", Vec2::new(160.0, 100.0));
            game.world.entity_mut(dice).unwrap().vel.x = 4.0;
            let before = game.rng.clone();
            crate::enemies::item_step_impacts(&mut game, dice);
            let target = raw(&game, target);
            assert_eq!(target.health, hp - 1, "{object}");
            let expected_state = if matches!(
                object,
                "oVampire"
                    | "oTombLord"
                    | "oUFO"
                    | "oGiantSpider"
                    | "oGiantSpiderHang"
                    | "oAlienBoss"
            ) {
                before_state
            } else {
                state
            };
            assert_eq!(target.state, expected_state, "{object}");
            assert_eq!(target.vel.x, 1.2);
            assert_eq!(target.value("assaulted"), 0.0);
            if object == "oAlienBoss" {
                assert_eq!(target.sprite, "sAlienBossHurt");
                assert_eq!(target.value("animation_speed"), 0.8);
            }
            if object == "oGiantSpider" {
                assert_eq!(target.value("hit_cooldown"), 10.0);
            }
            if matches!(
                object,
                "oVampire" | "oTombLord" | "oUFO" | "oGiantSpiderHang"
            ) {
                assert_eq!(target.value("origX"), position.x);
                assert_eq!(target.value("origY"), position.y);
            }
            let blood: Vec<_> = game
                .world
                .entities
                .iter()
                .filter(|e| e.object == "oBlood")
                .collect();
            assert_eq!(
                blood.len(),
                1,
                "{object} direct Create even bloodless Vampire"
            );
            assert_eq!(
                blood[0].pos,
                Vec2::new(position.x + offset.x, position.y + offset.y)
            );
            assert_eq!(sound_count(&game, "sndHit"), 1);
            assert!(!game.world.entities.iter().any(|e| e.object == "oExplosion"));
            let mut expected = before;
            for _ in 0..4 {
                expected.unit();
            }
            assert_eq!(game.rng.state_bits(), expected.state_bits());
        }
        for count in [16, 32] {
            let mut game = scene(rules);
            for _ in 0..count {
                game.spawn("oDetritus", Vec2::new(400.0, 200.0));
            }
            let target = game.spawn("oSnake", Vec2::new(154.0, 96.0));
            let dice = game.spawn("oDice", Vec2::new(160.0, 100.0));
            game.world.entity_mut(dice).unwrap().vel.x = 4.0;
            crate::enemies::item_step_impacts(&mut game, dice);
            assert_eq!(raw(&game, target).health, 0);
            let blood: Vec<_> = game
                .world
                .entities
                .iter()
                .filter(|e| e.object == "oBlood")
                .collect();
            assert_eq!(blood.len(), 1);
            assert_eq!(
                blood[0].alive,
                count == 16,
                "direct Create uses parent cap32"
            );
        }
    }
}

#[test]
fn configured_held_dice_keeps_velocity_and_own_heavy_attachment_for_damage() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for ducking in [false, true] {
            let mut game = scene(rules);
            if ducking {
                game.player.state = PlayerState::Ducking;
            }
            let y = if ducking { 200.0 } else { 198.0 };
            let target = game.spawn("oSnake", Vec2::new(22.0, y - 4.0));
            let dice = game.spawn("oDice", Vec2::new(80.0, 100.0));
            game.player.held = Some(dice);
            let receiver = game.world.entity_mut(dice).unwrap();
            receiver.held = true;
            receiver.vel.x = 4.0;
            step_instance(&mut game, dice);
            assert_eq!(raw(&game, dice).pos, Vec2::new(28.0, y));
            assert_eq!(raw(&game, dice).vel.x, 4.0);
            assert!(raw(&game, dice).held);
            assert_eq!(raw(&game, target).health, 0);
            assert_eq!(raw(&game, target).vel.x, 1.2);
        }
    }
}

#[test]
fn configured_bow_own_frame_reset_continues_after_lava_destroy() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for lava in [false, true] {
            let mut game = scene(rules);
            if lava {
                game.spawn("oLava", Vec2::new(160.0, 80.0));
            }
            let bow = game.spawn("oBow", Vec2::new(157.0, 100.0));
            let receiver = game.world.entity_mut(bow).unwrap();
            receiver.vel.x = 3.0;
            receiver.animation = 3.0; // initial frame retained from a prior held bow
            step_instance(&mut game, bow);
            assert_eq!(raw(&game, bow).alive, !lava);
            assert_eq!(raw(&game, bow).pos, Vec2::new(160.0, 100.0));
            assert_eq!(raw(&game, bow).animation, 0.0);
        }
    }
}

#[test]
fn configured_rope_parent_damage_precedes_deployment_and_survives_lava_retirement() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (lava, armed, speed, hit) in [
            (true, true, 4.0, true),
            (false, true, 4.0, true),
            (false, false, 4.0, true),
            (false, true, 2.0, false),
        ] {
            let mut game = scene(rules);
            let target = game.spawn("oSnake", Vec2::new(154.0, 96.0));
            if lava {
                game.spawn("oLava", Vec2::new(160.0, 80.0));
            }
            let rope = game.spawn("oRopeThrow", Vec2::new(160.0 - speed, 100.0));
            let receiver = game.world.entity_mut(rope).unwrap();
            receiver.vel.x = speed;
            receiver.data.insert("armed".into(), f64::from(armed));
            assert!(
                raw(&game, target)
                    .mask_contains(Vec2::new(160.0, 100.0), CollisionPrecision::BoundingBox)
            );
            step_instance(&mut game, rope);
            assert_eq!(raw(&game, rope).alive, !lava);
            assert_eq!(raw(&game, target).health, if hit { 0 } else { 1 });
            assert_eq!(
                sound_count(&game, "sndHit"),
                usize::from(hit),
                "not a second generic query"
            );
            let blood: Vec<_> = game
                .world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == "oBlood")
                .collect();
            assert_eq!(blood.len(), usize::from(hit));
            let tops: Vec<_> = game
                .world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == "oRopeTop")
                .collect();
            assert_eq!(tops.len(), usize::from(armed));
            if armed {
                assert_eq!(raw(&game, rope).vel, Vec2::default());
                assert_eq!(raw(&game, rope).value("falling"), 1.0);
                assert_eq!(raw(&game, rope).value("fall_count"), 1.0);
                if hit {
                    assert!(blood[0].creation_serial < tops[0].creation_serial);
                }
            } else {
                assert_eq!(raw(&game, rope).vel.x, speed);
            }
        }
    }
}

#[test]
fn configured_dice_own_switch_keeps_guarded_recoil_leaf_and_explicit_type() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (object, position, state, cooldown) in [
            ("oCaveman", Vec2::new(154.0, 96.0), 98, 0.0),
            ("oShopkeeper", Vec2::new(154.0, 96.0), 98, 0.0),
            ("oGiantSpider", Vec2::new(144.0, 80.0), 0, 10.0),
            ("oAlienBoss", Vec2::new(160.0, 100.0), 99, 0.0),
        ] {
            let mut game = scene(rules);
            let target = game.spawn(object, position);
            let target_body = game.world.entity_mut(target).unwrap();
            target_body.state = state;
            target_body.data.insert("hit_cooldown".into(), cooldown);
            let hp = target_body.health;
            let dice = game.spawn("oDice", Vec2::new(160.0, 100.0));
            game.world.entity_mut(dice).unwrap().vel.x = 4.0;
            let before = game.rng.state_bits();
            crate::enemies::item_step_impacts(&mut game, dice);
            assert_eq!(raw(&game, target).health, hp);
            assert_eq!(raw(&game, target).state, state);
            assert_eq!(
                raw(&game, target).vel.x,
                1.2,
                "post-with recoil runs even when own HP branch skips"
            );
            assert_eq!(game.rng.state_bits(), before);
            assert_eq!(sound_count(&game, "sndHit"), 0);
        }
        let mut game = scene(rules);
        let trap = game.spawn("oManTrap", Vec2::new(154.0, 96.0));
        let hp = raw(&game, trap).health;
        let dice = game.spawn("oDice", Vec2::new(160.0, 100.0));
        game.world.entity_mut(dice).unwrap().vel.x = 4.0;
        crate::enemies::item_step_impacts(&mut game, dice);
        assert_eq!(raw(&game, trap).health, hp - 1);
        assert_eq!(raw(&game, trap).state, 98);
        assert_eq!(
            raw(&game, trap).timer,
            raw(&game, trap).value("stun_time") as u32
        );
        let leaves: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|e| e.object == "oLeaf")
            .collect();
        assert_eq!(leaves.len(), 1);
        assert!((154.0..=170.0).contains(&leaves[0].pos.x));
        assert!((88.0..=104.0).contains(&leaves[0].pos.y));
        assert!(!game.world.entities.iter().any(|e| e.object == "oBlood"));
        assert_eq!(sound_count(&game, "sndHit"), 1);
        let mut game = scene(rules);
        let caveman = game.spawn("oCaveman", Vec2::new(154.0, 96.0));
        game.world
            .entity_mut(caveman)
            .unwrap()
            .text
            .insert("type".into(), "None".into());
        let state = raw(&game, caveman).state;
        let hp = raw(&game, caveman).health;
        let dice = game.spawn("oDice", Vec2::new(160.0, 100.0));
        game.world.entity_mut(dice).unwrap().vel.x = 4.0;
        crate::enemies::item_step_impacts(&mut game, dice);
        assert_eq!(raw(&game, caveman).health, hp - 1);
        assert_eq!(
            raw(&game, caveman).state,
            state,
            "explicit source type selects default, not object stun"
        );
        assert_eq!(raw(&game, caveman).value("origX"), 154.0);
        assert_eq!(raw(&game, caveman).value("origY"), 96.0);
    }
}

#[test]
fn configured_rope_hand_clear_keeps_entered_held_branch_but_runs_own_deployment() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = scene(rules);
        game.player.pos = Vec2::new(24.0, 80.0);
        game.player.previous_pos = game.player.pos;
        game.spawn("oLava", Vec2::new(16.0, 64.0));
        let rope = game.spawn("oRopeThrow", Vec2::new(80.0, 100.0));
        let receiver = game.world.entity_mut(rope).unwrap();
        receiver.held = true;
        receiver.vel.x = 4.0;
        receiver.data.insert("armed".into(), 1.0);
        assert!(
            game.player.held.is_none(),
            "initial empty source hold pointer"
        );
        step_instance(&mut game, rope);
        let receiver = raw(&game, rope);
        assert!(
            receiver.alive,
            "source held branch skips this tick's unheld Lava work"
        );
        assert!(!receiver.held);
        assert_eq!(receiver.vel, Vec2::default());
        assert_eq!(receiver.pos, Vec2::new(24.0, 90.0));
        assert_eq!(receiver.value("armed"), 0.0);
        assert_eq!(receiver.value("falling"), 1.0);
        assert_eq!(receiver.value("fall_count"), 1.0);
        let tops: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|e| e.alive && e.object == "oRopeTop")
            .collect();
        assert_eq!(tops.len(), 1);
        assert_eq!(tops[0].pos, Vec2::new(24.0, 82.0));
    }
}

// Literal finite nearest boundary: origin gap100000 squares to1e10. The
// configured wide mirrored rectangle intersects the moving receiver even
// when its target origin lies beyond the nearest sentinel. No stock claim.
fn wide_target(game: &mut Game, object: &str, distance: f64, active: bool) -> u64 {
    let id = game.spawn(object, Vec2::new(160.0 + distance, 100.0));
    let target = game.world.entity_mut(id).unwrap();
    target.set_sprite("sBlock");
    target.data.insert("image_xscale".into(), -10_000.0);
    target.engine_active = active;
    assert!(
        target
            .collision_mask()
            .intersects_rectangle_bbox(Vec2::new(158.0, 98.0), Vec2::new(162.0, 102.0)),
        "source raw BBox gate precedes nearest"
    );
    id
}
fn shop_context(game: &mut Game) {
    let mut path = [[0; 4]; 5];
    path[0][0] = 4;
    game.world.source_room_path = Some(path);
}
fn error_count(game: &Game) -> f64 {
    game.data
        .get("source_runtime_errors")
        .copied()
        .unwrap_or(0.0)
}

#[test]
fn configured_dice_damsel_nearest_sentinel_aborts_same_action_without_registering() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (distance, active, failed, hit) in [
            (99_999.0, true, false, true),
            (100_000.0, true, true, false),
            (100_001.0, true, true, false),
            (100_000.0, false, false, false),
        ] {
            let mut game = scene(rules);
            shop_context(&mut game); // exact initial shop avoids unrelated theft script
            let target = wide_target(&mut game, "oDamsel", distance, active);
            let hp = raw(&game, target).health;
            let dice = game.spawn("oDice", Vec2::new(156.0, 100.0));
            game.world.entity_mut(dice).unwrap().vel.x = 4.0;
            let old_value = raw(&game, dice).value("value");
            let old_sprite = raw(&game, dice).sprite.clone();
            // Bare-World pending birth is explicitly a compatibility control,
            // not native Create chronology. A read must not bootstrap it.
            let pending = if failed {
                Some(game.world.spawn("oGrave", Vec2::new(400.0, 200.0)))
            } else {
                None
            };
            let before = game.rng.clone();
            let ledger = game.dispatcher.registration_order.clone();
            step_instance(&mut game, dice);
            assert_eq!(raw(&game, dice).pos, Vec2::new(160.0, 100.0));
            assert_eq!(raw(&game, dice).vel, Vec2::new(4.0, 0.6));
            assert_eq!(raw(&game, target).health, hp - i32::from(hit));
            assert_eq!(error_count(&game), f64::from(failed));
            let blood: Vec<_> = game
                .world
                .entities
                .iter()
                .filter(|e| e.object == "oBlood")
                .collect();
            assert_eq!(blood.len(), usize::from(hit));
            assert_eq!(sound_count(&game, "sndHit"), usize::from(hit));
            if failed {
                assert_eq!(
                    game.rng.state_bits(),
                    before.state_bits(),
                    "no Blood or same-action rolling draw after failed read"
                );
                assert_eq!(raw(&game, dice).value("value"), old_value);
                assert_eq!(raw(&game, dice).sprite, old_sprite);
                assert_eq!(game.dispatcher.registration_order, ledger);
                assert_eq!(
                    raw(&game, pending.unwrap()).value("grave_create_initialized"),
                    0.0
                );
                assert!(game.events.iter().any(|event| matches!(event,
                    GameEvent::Message(text) if text.contains("oDice Step: obj.invincible"))));
            } else {
                let mut expected = before;
                if hit {
                    for _ in 0..4 {
                        expected.unit();
                    }
                }
                let value = f64::from(1 + expected.range(6));
                assert_eq!(game.rng.state_bits(), expected.state_bits());
                assert_eq!(raw(&game, dice).value("value"), value);
                assert_eq!(raw(&game, dice).sprite, "sDiceRoll");
                if hit {
                    assert_eq!(blood[0].pos, Vec2::new(160.0, 100.0));
                    assert_eq!(raw(&game, target).state, 2);
                    assert_eq!(raw(&game, target).timer, 120);
                }
            }
        }
    }
}

#[test]
fn configured_dice_missing_enemy_stops_damsel_and_roll_but_keeps_prior_enemy_effects() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for enemy_failure in [true, false] {
            let mut game = scene(rules);
            shop_context(&mut game);
            let enemy = if enemy_failure {
                wide_target(&mut game, "oSnake", 100_000.0, true)
            } else {
                game.spawn("oSnake", Vec2::new(154.0, 96.0))
            };
            let damsel = if enemy_failure {
                game.spawn("oDamsel", Vec2::new(160.0, 100.0))
            } else {
                wide_target(&mut game, "oDamsel", 100_000.0, true)
            };
            let enemy_hp = raw(&game, enemy).health;
            let damsel_hp = raw(&game, damsel).health;
            let dice = game.spawn("oDice", Vec2::new(156.0, 100.0));
            game.world.entity_mut(dice).unwrap().vel.x = 4.0;
            let value = raw(&game, dice).value("value");
            let sprite = raw(&game, dice).sprite.clone();
            let mut expected = game.rng.clone();
            if !enemy_failure {
                for _ in 0..4 {
                    expected.unit();
                }
            }
            step_instance(&mut game, dice);
            assert_eq!(error_count(&game), 1.0);
            assert_eq!(
                raw(&game, enemy).health,
                enemy_hp - i32::from(!enemy_failure)
            );
            assert_eq!(raw(&game, damsel).health, damsel_hp);
            assert_eq!(
                game.world
                    .entities
                    .iter()
                    .filter(|e| e.object == "oBlood")
                    .count(),
                usize::from(!enemy_failure)
            );
            assert_eq!(sound_count(&game, "sndHit"), usize::from(!enemy_failure));
            assert_eq!(
                game.rng.state_bits(),
                expected.state_bits(),
                "only completed prefix is retained; no rolling suffix"
            );
            assert_eq!(raw(&game, dice).value("value"), value);
            assert_eq!(raw(&game, dice).sprite, sprite);
        }
    }
}

#[test]
fn configured_parent_nearest_error_still_allows_separate_bow_and_rope_actions() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for object in ["oBow", "oRopeThrow"] {
            let mut game = scene(rules);
            wide_target(&mut game, "oSnake", 100_000.0, true);
            let item = game.spawn(object, Vec2::new(156.0, 100.0));
            let receiver = game.world.entity_mut(item).unwrap();
            receiver.vel.x = 4.0;
            if object == "oBow" {
                receiver.animation = 3.0;
            } else {
                receiver.data.insert("armed".into(), 1.0);
            }
            let before = game.rng.state_bits();
            step_instance(&mut game, item);
            assert_eq!(error_count(&game), 1.0);
            assert_eq!(game.rng.state_bits(), before);
            assert!(!game.world.entities.iter().any(|e| e.object == "oBlood"));
            assert!(raw(&game, item).alive);
            if object == "oBow" {
                assert_eq!(
                    raw(&game, item).animation,
                    0.0,
                    "separate own Code frame reset is reached"
                );
            } else {
                assert_eq!(raw(&game, item).value("armed"), 0.0);
                assert_eq!(raw(&game, item).value("falling"), 1.0);
                assert!(
                    game.world
                        .entities
                        .iter()
                        .any(|e| e.alive && e.object == "oRopeTop")
                );
            }
        }
    }
}
