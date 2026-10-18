//! Stock Moon alarm births plus initially configured BatTarget event controls.
//! All actor/property setup precedes the first input; no mid-run state writes.
use spelunky_core::{
    Area, Game, GameEvent, InputFrame, PlayerState, RulesPreset, Vec2, World, hub::HubKind,
};

fn configured_room(rules: RulesPreset, timer: f64) -> Game {
    let mut game = Game::shell(42, rules);
    game.world = World::new(40, 24, Area::Mines, 1);
    game.world.name = "rMoon".into();
    game.flags.insert("hub".into());
    game.flags.insert("hub_moon".into());
    for x in 0..40 {
        game.world.set_tile(x, 13, "oHardBlock");
    }
    game.player.pos = Vec2::new(24.0, 200.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.world.entrance = game.player.pos;
    game.data.insert("timer".into(), timer);
    game.data.insert("baskets".into(), 0.0);
    game
}

#[test]
fn untouched_moon_alarm_chain_creates_three_bats_before_their_first_own_steps() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = Game::shell(42, rules);
        game.load_hub(HubKind::Moon);
        assert_eq!(game.player.health, 8.0, "source Moon Create health");
        for tick in 1..=67 {
            game.step(&InputFrame::default());
            assert!(
                !game.world.entities.iter().any(|e| e.object == "oBatTarget"),
                "no birth before Alarm3 tick68: {tick}"
            );
            if tick == 30 {
                assert_eq!(game.data.get("hub_draw_status"), Some(&1.0));
            }
            if tick == 59 {
                assert_eq!(game.data.get("hub_draw_status"), Some(&2.0));
            }
        }
        game.step(&InputFrame::default());
        assert_eq!(game.level_tick, 68);
        assert_eq!(game.data.get("hub_draw_status"), Some(&3.0));
        let bats: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|e| e.alive && e.object == "oBatTarget")
            .collect();
        assert_eq!(bats.len(), 3, "SOURCE_ALARM3_BIRTH_BEFORE_STEP {rules:?}");
        for (bat, (x, initial_y, difficulty)) in bats.iter().zip([
            (160.0, -16.0, 1.0),
            (208.0, -64.0, 2.0),
            (256.0, -128.0, 4.0),
        ]) {
            assert!(bat.engine_active);
            assert_eq!(bat.value("difficulty"), difficulty);
            assert_eq!(bat.value("dir"), 1.0);
            assert_eq!(bat.value("xMid"), x);
            assert_eq!(
                bat.pos,
                Vec2::new(x, initial_y + difficulty),
                "newborn940 owns one same-frame Step after Alarm3 at715"
            );
            assert_eq!(bat.sprite, "sBatLeft");
        }
        assert_eq!(game.player.health, 8.0);
        assert_eq!(game.data.get("baskets"), Some(&0.0));
    }
}

#[test]
fn bat_target_create_and_own_step_use_source_timer_and_difficulty_without_rng() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for timer in [60.0, 20.0, 19.0] {
            for (x, base) in [(160.0, 1.0), (208.0, 2.0), (256.0, 4.0)] {
                let mut game = configured_room(rules, timer);
                let rng = game.rng.state_bits();
                let id = game.spawn("oBatTarget", Vec2::new(x, 100.0));
                let bat = game.world.entity(id).unwrap();
                let difficulty = base + if timer <= 20.0 { 2.0 } else { 0.0 };
                for (key, value) in [
                    ("bloodless", 0.0),
                    ("UP", 0.0),
                    ("DOWN", 1.0),
                    ("dir", 1.0),
                    ("difficulty", difficulty),
                    ("xMid", x),
                    ("xDiff", 0.0),
                ] {
                    assert_eq!(
                        bat.data.get(key),
                        Some(&value),
                        "SOURCE_IMMEDIATE_BAT_CREATE {key}"
                    );
                }
                assert_eq!(
                    bat.data.get("image_xscale"),
                    Some(&1.0),
                    "parentless source default must not inherit Scene facing flip"
                );
                assert_eq!(bat.sprite, if timer <= 20.0 { "sUFO" } else { "sBatLeft" });
                assert_eq!(game.rng.state_bits(), rng);
                game.step(&InputFrame::default());
                let bat = game.world.entity(id).unwrap();
                assert_eq!(bat.pos, Vec2::new(x, 100.0 + difficulty));
                assert_eq!(bat.value("xDiff"), if timer <= 20.0 { 0.01 } else { 0.0 });
                assert_eq!(game.data.get("baskets"), Some(&0.0));
                assert_eq!(game.player.health, 4.0);
            }
        }
    }
}

#[test]
fn bat_target_arrow_collision_uses_current_precise_membership_and_source_rebirth_order() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (active, arrow_start, speed, arrow_y, hits) in [
            (true, 140.0, 20.0, 108.0, true),
            (true, 140.0, 48.0, 108.0, false),
            (false, 140.0, 20.0, 108.0, false),
            (true, 140.0, 17.0, 102.0, false),
        ] {
            let mut game = configured_room(rules, 60.0);
            let id = game.spawn("oBatTarget", Vec2::new(160.0, 100.0));
            // Explicit legal source producer overrides make both old and new
            // arms share the same incoming scene; immediate Create has its
            // independent discriminator above.
            let bat = game.world.entity_mut(id).unwrap();
            bat.data.insert("dir".into(), 1.0);
            bat.data.insert("difficulty".into(), 1.0);
            bat.data.insert("xMid".into(), 160.0);
            bat.data.insert("xDiff".into(), 0.0);
            bat.data.insert("bloodless".into(), 0.0);
            bat.engine_active = active;
            let arrow = game.spawn("oArrow", Vec2::new(arrow_start, arrow_y));
            let projectile = game.world.entity_mut(arrow).unwrap();
            projectile.vel = Vec2::new(speed, 0.0);
            projectile.data.insert("gravity".into(), 0.0);
            projectile.data.insert("safe".into(), 1.0);
            // Qualify actual selected-frame mask contact before the input,
            // using detached endpoint copies, never a live actor rewrite.
            let mut endpoint_bat = game.world.entity(id).unwrap().clone();
            endpoint_bat.pos.y = if active { 101.0 } else { 100.0 };
            let mut endpoint_arrow = game.world.entity(arrow).unwrap().clone();
            endpoint_arrow.pos.x = arrow_start + speed;
            assert_eq!(
                endpoint_bat.event_intersects(&endpoint_arrow) && active,
                hits,
                "FIXTURE: current precise overlap/membership"
            );
            if arrow_y == 102.0 {
                // Bat RECTANGLE frame0 fills x160..175,y105..114 after its Step.
                // Arrow endpoint157,102 overlaps only world160,105: source
                // Arrow pixel(7,7) is clear (row7 .##..##.), despite bbox overlap.
                assert!(
                    endpoint_bat
                        .sprite_bounds()
                        .intersects(endpoint_arrow.sprite_bounds()),
                    "FIXTURE: prior smooth bbox proxy sees the transparent corner"
                );
            }
            game.step(&InputFrame::default());
            assert_eq!(
                game.world.entity(id).is_none(),
                hits,
                "SOURCE_PRECISE_BAT_COLLISION {rules:?} active={active} speed={speed}"
            );
            assert_eq!(game.world.entity(arrow).is_none(), hits);
            assert_eq!(
                game.data.get("baskets"),
                Some(&(if hits { 1.0 } else { 0.0 }))
            );
            if hits {
                let sounds: Vec<_> = game
                    .events
                    .iter()
                    .filter_map(|e| match e {
                        GameEvent::Sound { name, .. } if name == "sndHit" || name == "sndCoin" => {
                            Some(name.as_str())
                        }
                        _ => None,
                    })
                    .collect();
                assert_eq!(sounds, ["sndHit", "sndCoin"]);
                let replacement = game
                    .world
                    .entities
                    .iter()
                    .find(|e| e.alive && e.object == "oBatTarget")
                    .unwrap();
                assert_eq!(
                    replacement.pos,
                    Vec2::new(160.0, -64.0),
                    "automatic Collision newborn cannot reenter finished Normal Step"
                );
                assert_eq!(replacement.value("difficulty"), 1.0);
                assert_eq!(
                    game.world
                        .entities
                        .iter()
                        .filter(|e| e.object == "oBlood")
                        .count(),
                    3
                );
                assert!(
                    game.world
                        .entities
                        .iter()
                        .filter(|e| e.object == "oBlood")
                        .all(|e| e.creation_serial > replacement.creation_serial)
                );
            } else {
                assert!(!game.world.entities.iter().any(|e| e.object == "oBlood"));
                assert_eq!(
                    game.world.entity(id).unwrap().pos,
                    Vec2::new(160.0, if active { 101.0 } else { 100.0 }),
                    "FIXTURE: own Step/membership completed before collision control"
                );
                assert_eq!(
                    game.world.entity(arrow).unwrap().pos,
                    Vec2::new(arrow_start + speed, arrow_y),
                    "FIXTURE: Arrow actually traversed unchanged input-free source path"
                );
            }
            assert_eq!(game.player.health, 4.0);
        }
    }
}

#[test]
fn bat_target_character_collision_executes_stomp_hurt_and_source_guards() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for mode in ["stomp", "hurt", "invincible", "swimming", "dead"] {
            let mut game = configured_room(rules, 60.0);
            let id = game.spawn("oBatTarget", Vec2::new(160.0, 100.0));
            game.player.pos = Vec2::new(
                168.0,
                if matches!(mode, "hurt" | "invincible") {
                    110.0
                } else {
                    104.0
                },
            );
            game.player.previous_pos = game.player.pos;
            game.player.state = PlayerState::Falling;
            game.player.grounded = false;
            game.player.vel = Vec2::new(0.0, 1.0);
            if mode == "invincible" {
                game.player.invincible = 2;
            }
            if mode == "swimming" {
                game.world.set_tile(10, 6, "oWaterSwim");
            }
            if mode == "dead" {
                game.player.health = 0.0;
                game.player.state = PlayerState::Dead;
                game.state = spelunky_core::RunState::Dead;
                game.player.vel = Vec2::default();
            }
            game.step(&InputFrame::default());
            if mode == "stomp" {
                assert_eq!(
                    game.player.pos,
                    Vec2::new(168.0, 106.0),
                    "FIXTURE: actual downward controller movement precedes contact"
                );
                assert!(game.world.entity(id).is_none());
                assert_eq!(game.data.get("baskets"), Some(&1.0));
                assert!(
                    (game.player.vel.y - (-6.0 - 0.2 * 2.0)).abs() < 1e-12,
                    "literal bounce uses post-controller downward velocity2"
                );
                assert_eq!(game.player.health, 4.0);
                let replacement = game
                    .world
                    .entities
                    .iter()
                    .find(|e| e.alive && e.object == "oBatTarget")
                    .unwrap();
                assert_eq!(replacement.pos, Vec2::new(160.0, -64.0));
                assert_eq!(
                    game.world
                        .entities
                        .iter()
                        .filter(|e| e.object == "oBlood")
                        .count(),
                    3
                );
            } else {
                let bat = game
                    .world
                    .entity(id)
                    .expect("guard/hurt branch preserves target");
                assert_eq!(game.data.get("baskets"), Some(&0.0));
                if mode == "swimming" {
                    assert_eq!(
                        game.player.data.get("swimming"),
                        Some(&1.0),
                        "FIXTURE: actual WaterSwim query preserves source swimming guard"
                    );
                    assert!(game.player.pos.y < bat.pos.y + 8.0);
                    assert!(matches!(
                        game.player.state,
                        PlayerState::Jumping | PlayerState::Falling
                    ));
                }
                if mode == "invincible" {
                    assert_eq!(game.player.health, 4.0);
                    assert_eq!(game.player.invincible, 1);
                    assert!(!game.world.entities.iter().any(|e| e.object == "oBlood"));
                } else {
                    assert_eq!(game.player.health, if mode == "dead" { 0.0 } else { 3.0 });
                    assert_eq!(game.player.invincible, 30);
                    assert_eq!(game.player.data.get("blink"), Some(&30.0));
                    assert_eq!(game.player.vel.x, 6.0);
                    assert_eq!(
                        game.world
                            .entities
                            .iter()
                            .filter(|e| e.object == "oBlood")
                            .count(),
                        1
                    );
                    assert!(
                        game.events.iter().any(
                            |e| matches!(e, GameEvent::Sound { name, .. } if name == "sndHurt")
                        )
                    );
                }
            }
            assert_eq!(
                game.player.stun, 0,
                "source BatTarget contact does not assign stunned"
            );
        }
    }
}

#[test]
fn existing_ufo_crash_create_step_and_alarm_follow_literal_three_draw_trails() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = configured_room(rules, 60.0);
        let rng_before = game.rng.state_bits();
        let id = game.spawn("oUFOCrash", Vec2::new(160.0, 80.0));
        let crash = game.world.entity(id).unwrap();
        assert_eq!(crash.vel, Vec2::default());
        assert_eq!(crash.alarms[0], 3);
        assert_eq!(crash.sprite, "sUFOCrash");
        assert!(
            !spelunky_core::enemies::is_enemy(&crash.object),
            "root object has no Enemy inheritance"
        );
        assert_eq!(game.rng.state_bits(), rng_before);
        game.step(&InputFrame::default());
        assert_eq!(game.world.entity(id).unwrap().pos, Vec2::new(160.0, 80.0));
        assert_eq!(game.world.entity(id).unwrap().alarms[0], 2);
        game.step(&InputFrame::default());
        let origin = game.world.entity(id).unwrap().pos;
        assert!((origin.y - 80.6).abs() < 1e-12);
        let mut literal_rng = game.rng.clone();
        let child = if literal_rng.source_rand(1, 2).unwrap() == 1 {
            "oFlameTrail"
        } else {
            "oBurn"
        };
        let dx = literal_rng.source_rand(0, 16).unwrap() as f64;
        let dy = literal_rng.source_rand(0, 16).unwrap() as f64;
        game.step(&InputFrame::default());
        assert_eq!(
            game.rng.state_bits(),
            literal_rng.state_bits(),
            "one selector and two offsets; DrawnSprite child Create is RNG-free"
        );
        let crash = game.world.entity(id).unwrap();
        assert_eq!(crash.alarms[0], 3);
        assert!((crash.pos.y - 81.8).abs() < 1e-12);
        assert!((crash.vel.y - 1.8).abs() < 1e-12);
        let trails: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|e| matches!(e.object.as_str(), "oFlameTrail" | "oBurn"))
            .collect();
        assert_eq!(trails.len(), 1);
        let trail = trails[0];
        assert_eq!(trail.object, child);
        assert_eq!(trail.pos.x, origin.x + dx);
        assert!(
            (trail.pos.y - (origin.y + dy - if child == "oBurn" { 0.1 } else { 0.0 })).abs()
                < 1e-12,
            "Alarm birth precedes child own Normal Step"
        );
        assert_eq!(trail.image_speed(), 0.4);
        assert_eq!(game.player.health, 4.0);
    }
}

#[test]
fn existing_ufo_crash_gravity_threshold_adds_without_clamping_and_collision_creates_effects() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = configured_room(rules, 60.0);
        let id = game.spawn("oUFOCrash", Vec2::new(160.0, 80.0));
        game.world.entity_mut(id).unwrap().vel = Vec2::new(2.0, 5.9);
        game.step(&InputFrame::default());
        let crash = game.world.entity(id).unwrap();
        assert_eq!(crash.pos.x, 162.0);
        assert!((crash.pos.y - 85.9).abs() < 1e-12);
        assert!(
            (crash.vel.y - 6.5).abs() < 1e-12,
            "source compares before adding and permits overshoot"
        );
        game.step(&InputFrame::default());
        let crash = game.world.entity(id).unwrap();
        assert_eq!(crash.pos.x, 164.0);
        assert!((crash.pos.y - 92.4).abs() < 1e-12);
        assert!((crash.vel.y - 6.5).abs() < 1e-12);
        for solid in [true, false] {
            let mut game = configured_room(rules, 60.0);
            if solid {
                game.world.set_tile(10, 6, "oHardBlock");
            } else {
                game.spawn("oCaveman", Vec2::new(160.0, 96.0));
            }
            let id = game.spawn("oUFOCrash", Vec2::new(160.0, 96.0));
            game.step(&InputFrame::default());
            assert!(
                game.world.entity(id).is_none(),
                "source Crash {solid} precise contact retires receiver"
            );
            let explosion = game
                .world
                .entities
                .iter()
                .find(|e| e.object == "oExplosion")
                .expect("source Collision Create");
            assert_eq!(explosion.pos, Vec2::new(168.0, 104.0));
            assert_eq!(
                game.events
                    .iter()
                    .filter(
                        |e| matches!(e, GameEvent::Sound { name, .. } if name == "sndExplosion")
                    )
                    .count(),
                2,
                "Explosion Create sound then explicit Crash caller sound"
            );
            assert_eq!(
                game.lifetime_stats.enemy_kills[spelunky_core::statistics::EnemyKill::Ufo as usize],
                0,
                "Crash is not Enemy death credit"
            );
            assert_eq!(game.player.health, 4.0);
        }
    }
}
