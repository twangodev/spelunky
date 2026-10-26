//! Configured source oTarget event scenes, not a natural Moon campaign claim.
//! Create/Step/Collision: Objects/Other/Title/oTarget.events in Classic 1.1.
//! All actor/state setup precedes the first Game::step; later writes are inputs.
use spelunky_core::{
    Area, Game, GameEvent, InputFrame, PlayerState, RulesPreset, Vec2, World, catalog,
};

fn room(rules: RulesPreset) -> Game {
    let mut game = Game::shell(123, rules);
    game.world = World::new(40, 24, Area::Mines, 1);
    game.world.name = "rLevel".into();
    for x in 0..40 {
        game.world.set_tile(x, 13, "oHardBlock");
    }
    game.player.pos = Vec2::new(24.0, 200.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.world.entrance = game.player.pos;
    // The source Moon controller fields are represented by these game keys.
    game.data.insert("timer".into(), 60.0);
    game.data.insert("baskets".into(), 0.0);
    game
}

fn hit_sounds(game: &Game) -> Vec<&str> {
    game.events
        .iter()
        .filter_map(|event| match event {
            GameEvent::Sound { name, .. } if name == "sndCoin" || name == "sndHit" => {
                Some(name.as_str())
            }
            _ => None,
        })
        .collect()
}

fn points(delta: f64) -> f64 {
    if (-2.0..=2.0).contains(&delta) {
        5.0
    } else if (-6.0..=-3.0).contains(&delta) || (3.0..=6.0).contains(&delta) {
        3.0
    } else if (-10.0..=-7.0).contains(&delta) || (7.0..=10.0).contains(&delta) {
        2.0
    } else {
        1.0
    }
}

fn moving_target(game: &mut Game, pos: Vec2, dir: f64, move_off: f64) -> u64 {
    let id = game.spawn("oTarget", pos);
    let target = game.world.entity_mut(id).unwrap();
    target.data.insert("dir".into(), dir);
    target.data.insert("moveOff".into(), move_off);
    id
}

#[test]
fn target_create_is_immediate_and_consumes_one_source_direction_draw_before_overrides() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for x in [208.0, 209.0] {
            let mut game = room(rules);
            let mut source_rng = game.rng.clone();
            let dir = source_rng.source_rand(0, 1).unwrap() as f64;
            let target = game.spawn("oTarget", Vec2::new(x, 100.0));
            let entity = game.world.entity(target).unwrap();
            for (field, value) in [
                ("life", 20.0),
                ("dying", 0.0),
                ("UP", 0.0),
                ("DOWN", 1.0),
                ("dir", dir),
                ("moveOff", 32.0),
                ("hard", f64::from(x > 208.0)),
                ("xDiff", 0.0),
            ] {
                assert_eq!(
                    entity.data.get(field),
                    Some(&value),
                    "immediate Create field {field}"
                );
            }
            assert_eq!(
                game.rng.state_bits(),
                source_rng.state_bits(),
                "exactly one Create draw"
            );
            let producer_draw = game.rng.source_rand(0, 99).unwrap();
            assert_eq!(producer_draw, source_rng.source_rand(0, 99).unwrap());
            // An initial producer override must survive the first actual Step.
            game.world
                .entity_mut(target)
                .unwrap()
                .data
                .insert("dir".into(), 1.0);
            game.step(&InputFrame::default());
            let target = game.world.entity(target).unwrap();
            assert_eq!(
                target.pos,
                Vec2::new(x, if x > 208.0 { 102.0 } else { 101.0 })
            );
            assert_eq!(target.value("dir"), 1.0);
            assert_eq!(target.value("moveOff"), 33.0);
            assert_eq!(
                game.rng.state_bits(),
                source_rng.state_bits(),
                "own Step draws no RNG"
            );
        }
    }
}

#[test]
fn target_own_step_preserves_literal_direction_bounds_and_strict_move_offset_limit() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (x, y, dir, off, expected_y, expected_dir, expected_off) in [
            (208.0, 100.0, 0.0, 32.0, 99.0, 0.0, 33.0),
            (209.0, 100.0, 1.0, 32.0, 102.0, 1.0, 33.0),
            (208.0, 65.0, 0.0, 32.0, 64.0, 1.0, 0.0),
            (209.0, 65.0, 0.0, 32.0, 63.0, 1.0, 0.0),
            (208.0, 159.0, 1.0, 32.0, 160.0, 0.0, 0.0),
            (209.0, 159.0, 1.0, 32.0, 161.0, 0.0, 0.0),
            (192.0, 100.0, 0.0, 63.0, 99.0, 0.0, 64.0),
            (192.0, 100.0, 0.0, 64.0, 99.0, 1.0, 0.0),
            // The source has two explicit branches; another dir moves neither.
            (192.0, 100.0, 2.0, 32.0, 100.0, 2.0, 32.0),
        ] {
            let mut game = room(rules);
            let id = moving_target(&mut game, Vec2::new(x, y), dir, off);
            game.step(&InputFrame::default());
            let target = game.world.entity(id).unwrap();
            assert_eq!(target.pos, Vec2::new(x, expected_y));
            assert_eq!(target.value("dir"), expected_dir);
            assert_eq!(target.value("moveOff"), expected_off);
            assert_eq!(target.value("life"), 20.0);
        }
    }
}

#[test]
fn target_sine_motion_uses_old_phase_and_dying_reset_has_a_stationary_frame() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for timer in [31.0, 30.0] {
            let mut game = room(rules);
            game.data.insert("timer".into(), timer);
            let id = moving_target(&mut game, Vec2::new(192.0, 100.0), 0.0, 32.0);
            game.world
                .entity_mut(id)
                .unwrap()
                .data
                .insert("xDiff".into(), 0.7);
            game.step(&InputFrame::default());
            let target = game.world.entity(id).unwrap();
            assert_eq!(target.pos.y, 99.0);
            if timer == 30.0 {
                let expected_x = 240.0 - (0.7_f64.to_radians().sin() * 64.0).abs();
                assert!((target.pos.x - expected_x).abs() < 1e-10);
                assert_eq!(target.value("xDiff"), 0.7 + 0.01);
            } else {
                assert_eq!(target.pos.x, 192.0);
                assert_eq!(target.value("xDiff"), 0.7);
            }
        }
        let mut game = room(rules);
        game.data.insert("timer".into(), 30.0);
        let id = moving_target(&mut game, Vec2::new(192.0, 100.0), 0.0, 32.0);
        let target = game.world.entity_mut(id).unwrap();
        target.data.insert("dying".into(), 1.0);
        target.data.insert("life".into(), 1.0);
        target.data.insert("xDiff".into(), 0.7);
        for (life, dying) in [(0.0, 1.0), (20.0, 0.0)] {
            game.step(&InputFrame::default());
            let target = game.world.entity(id).unwrap();
            assert_eq!(target.pos, Vec2::new(192.0, 100.0));
            assert_eq!(target.value("life"), life);
            assert_eq!(target.value("dying"), dying);
            assert_eq!(target.value("xDiff"), 0.7);
            assert_eq!(target.value("moveOff"), 32.0);
        }
        game.step(&InputFrame::default());
        let target = game.world.entity(id).unwrap();
        let expected_x = 240.0 - (0.7_f64.to_radians().sin() * 64.0).abs();
        assert_eq!(
            target.pos.y, 99.0,
            "dying reset leaves the target stationary before degree-based motion resumes"
        );
        assert!((target.pos.x - expected_x).abs() < 1e-10);
        assert_eq!(target.value("xDiff"), 0.7 + 0.01);
    }
}

fn collision_scene(rules: RulesPreset, hard: bool, delta: f64) -> (Game, u64, u64) {
    let mut game = room(rules);
    let x = if hard { 224.0 } else { 168.0 };
    let target = moving_target(&mut game, Vec2::new(x, 100.0), 1.0, 32.0);
    let target_y = if hard { 102.0 } else { 101.0 };
    // Arrow465 moves before Target771. Gravity is authored before the first
    // Step to keep this controlled first-frame scoring ordinate literal.
    let arrow = game.spawn("oArrow", Vec2::new(x + 4.0, target_y + delta));
    let entity = game.world.entity_mut(arrow).unwrap();
    entity.vel.x = 4.0;
    entity.data.insert("gravity".into(), 0.0);
    entity.data.insert("safe".into(), 1.0);
    (game, target, arrow)
}

#[test]
fn actual_arrow_collision_scores_literal_intervals_without_hard_multiplier_and_orders_sounds() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for hard in [false, true] {
            for delta in [
                -11.0, -10.0, -7.0, -6.0, -3.0, -2.5, -2.0, 0.0, 2.0, 2.5, 3.0, 6.0, 7.0, 10.0,
                11.0,
            ] {
                let (mut game, target, arrow) = collision_scene(rules, hard, delta);
                game.step(&InputFrame::default());
                let target = game.world.entity(target).unwrap();
                let arrow = game.world.entity(arrow).expect("life20 leaves Arrow alive");
                assert_eq!(
                    arrow.vel,
                    Vec2::default(),
                    "source collision zeros both speeds"
                );
                assert_eq!(target.value("dying"), 1.0);
                assert_eq!(target.value("life"), 20.0);
                assert_eq!(game.data["baskets"], points(delta));
                assert_eq!(
                    hit_sounds(&game),
                    if points(delta) == 5.0 {
                        vec!["sndCoin", "sndHit"]
                    } else {
                        vec!["sndHit"]
                    }
                );
            }
        }
    }
}

#[test]
fn repeated_real_collision_stops_arrow_until_source_life_two_then_resets_without_rescoring() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, target, arrow) = collision_scene(rules, false, 0.0);
        game.step(&InputFrame::default());
        assert_eq!(game.data["baskets"], 5.0);
        assert_eq!(hit_sounds(&game), ["sndCoin", "sndHit"]);
        for life in (2..20).rev() {
            game.step(&InputFrame::default());
            assert_eq!(
                game.world.entity(target).unwrap().value("life"),
                f64::from(life)
            );
            assert_eq!(game.data["baskets"], 5.0);
            assert!(hit_sounds(&game).is_empty(), "dying target cannot rescore");
            if life > 2 {
                assert_eq!(game.world.entity(arrow).unwrap().vel, Vec2::default());
            } else {
                assert!(
                    game.world.entity(arrow).is_none(),
                    "Collision retires Arrow at life2"
                );
            }
        }
        for (life, dying) in [(1.0, 1.0), (0.0, 1.0), (20.0, 0.0)] {
            game.step(&InputFrame::default());
            let target = game.world.entity(target).unwrap();
            assert_eq!(target.pos, Vec2::new(168.0, 101.0));
            assert_eq!(target.value("life"), life);
            assert_eq!(target.value("dying"), dying);
            assert_eq!(game.data["baskets"], 5.0);
        }
        game.step(&InputFrame::default());
        assert_eq!(game.world.entity(target).unwrap().pos.y, 102.0);
    }
}

#[test]
fn moon_strict_near_and_precise_nonoverlap_controls_do_not_invent_target_hits() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        // Current Arrow origins relative to the Target after both Steps.
        // dx19 leaves distance exactly4: the Moon Arrow guard requires <4.
        for (dx, dy, speed, bbox_only) in [
            (1.0, -17.0, 4.0, true),
            (19.0, 0.0, 4.0, false),
            (24.0, 0.0, 32.0, false),
        ] {
            let mut game = room(rules);
            game.world.name = "rMoon".into();
            let target = moving_target(&mut game, Vec2::new(168.0, 100.0), 1.0, 32.0);
            let final_pos = Vec2::new(168.0 + dx, 101.0 + dy);
            let arrow = game.spawn("oArrow", Vec2::new(final_pos.x - speed, final_pos.y));
            let entity = game.world.entity_mut(arrow).unwrap();
            entity.vel.x = speed;
            entity.data.insert("gravity".into(), 0.0);
            entity.data.insert("safe".into(), 1.0);
            let unit = Vec2::new(1.0, 1.0);
            let am = catalog::instance_mask("oArrow", "sArrowRight", final_pos, 0.0, unit, 0.0);
            let tm = catalog::instance_mask(
                "oTarget",
                "sTarget",
                Vec2::new(168.0, 101.0),
                0.0,
                unit,
                0.0,
            );
            assert!(
                !am.intersects_mask(&tm),
                "FIXTURE: current precise masks are separated"
            );
            assert_eq!(am.bounds().intersects(tm.bounds()), bbox_only);
            game.step(&InputFrame::default());
            let arrow = game.world.entity(arrow).unwrap();
            assert_eq!(
                arrow.pos, final_pos,
                "strict near guard must leave this origin alone"
            );
            assert_eq!(arrow.vel, Vec2::new(speed, 0.0));
            assert_eq!(game.world.entity(target).unwrap().value("dying"), 0.0);
            assert_eq!(game.data["baskets"], 0.0);
            assert!(hit_sounds(&game).is_empty());
        }
    }
}

#[test]
fn ordinary_bow_pickup_draw_and_release_hits_a_configured_target_through_real_collision() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = room(rules);
        game.world = World::new(40, 24, Area::Mines, 1);
        game.world.name = "rMoon".into();
        for x in 0..40 {
            game.world.set_tile(x, 10, "oHardBlock");
        }
        game.player.pos = Vec2::new(64.0, 152.0);
        game.player.previous_pos = game.player.pos;
        game.world.entrance = game.player.pos;
        let bow = game.spawn("oBow", game.player.pos);
        // Initial configured dir2 exercises the source's stationary branch;
        // all subsequent pickup/charge/release/flight work uses ordinary input.
        let target = moving_target(&mut game, Vec2::new(120.0, 152.0), 2.0, 32.0);
        game.step(&InputFrame {
            down: true,
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert_eq!(
            game.player.held,
            Some(bow),
            "ordinary Bow pickup prerequisite"
        );
        assert_eq!(game.data["arrows"], 6.0);
        game.step(&InputFrame {
            attack_released: true,
            ..Default::default()
        });
        game.step(&InputFrame {
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        for _ in 0..60 {
            game.step(&InputFrame {
                attack: true,
                ..Default::default()
            });
        }
        assert_eq!(game.player.data["bow_armed"], 1.0);
        // Original Player Step1505 permits a final addition across 12.
        let mut source_strength = 0.0;
        for _ in 0..61 {
            if source_strength < 12.0 {
                source_strength += 0.2;
            }
        }
        assert_eq!(game.player.data["bow_strength"], source_strength);
        game.step(&InputFrame {
            attack_released: true,
            ..Default::default()
        });
        assert_eq!(game.data["arrows"], 5.0);
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|e| e.object == "oArrow")
                .count(),
            1
        );
        let mut hit = false;
        for _ in 0..12 {
            if game.data["baskets"] > 0.0 {
                let arrow = game
                    .world
                    .entities
                    .iter()
                    .find(|e| e.alive && e.object == "oArrow")
                    .unwrap();
                let target = game.world.entity(target).unwrap();
                assert_eq!(target.value("dying"), 1.0);
                assert_eq!(target.value("life"), 20.0);
                assert_eq!(arrow.vel, Vec2::default());
                assert_eq!(game.data["baskets"], points(arrow.pos.y - target.pos.y));
                assert_eq!(
                    hit_sounds(&game),
                    if game.data["baskets"] == 5.0 {
                        vec!["sndCoin", "sndHit"]
                    } else {
                        vec!["sndHit"]
                    }
                );
                hit = true;
                break;
            }
            game.step(&InputFrame::default());
        }
        assert!(hit, "ordinary released Arrow must hit configured target");
        assert_eq!(game.player.held, Some(bow));
        assert_eq!(game.player.health, 4.0);
    }
}
