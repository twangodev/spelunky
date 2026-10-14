//! Source Player Collect2097–2113 recovers only after hurt and held release.
//! Initial public actor contexts are authored before first Step; natural case
//! throws a picked-up Rock while running and observes its unsafe wall return.
use spelunky_core::{
    Area, CollisionPrecision, Entity, Game, GameEvent, InputFrame, PlayerState, RulesPreset, Vec2,
    World,
};
const PRESETS: [RulesPreset; 2] = [RulesPreset::Classic, RulesPreset::Assisted];
fn attack() -> InputFrame {
    InputFrame {
        attack: true,
        attack_pressed: true,
        ..Default::default()
    }
}
fn release() -> InputFrame {
    InputFrame {
        attack_released: true,
        ..Default::default()
    }
}
fn room(rules: RulesPreset, x: f64) -> Game {
    let mut game = Game::shell(37, rules);
    game.world = World::new(42, 18, Area::Mines, 1);
    game.world.name = "rLevel".into();
    game.player.pos = Vec2::new(x, 120.0);
    game.player.previous_pos = game.player.pos;
    game.player.grounded = true;
    game.player.state = PlayerState::Standing;
    game.player.facing = 1;
    for x in 0..42 {
        game.world.set_tile(x, 8, "oHardBlock");
    }
    game
}
fn free_bow(game: &mut Game, pos: Vec2) -> u64 {
    let id = game.spawn("oBow", pos);
    let e = game.world.entity_mut(id).unwrap();
    e.data.insert("cost".into(), 0.0);
    e.data.insert("forSale".into(), 0.0);
    id
}
fn square_hit(target: &Entity, origin: Vec2) -> bool {
    // These integer-origin, unrotated source fixtures use exact inclusive
    // prec0 bbox queries. Public point oracle avoids private mask APIs.
    assert_eq!(origin.x.fract(), 0.0, "FIXTURE_INTEGER_PLAYER_X");
    assert_eq!(origin.y.fract(), 0.0, "FIXTURE_INTEGER_PLAYER_Y");
    (-8..=8).any(|dx| {
        (-8..=8).any(|dy| {
            target.mask_contains(
                Vec2::new(origin.x + f64::from(dx), origin.y + f64::from(dy)),
                CollisionPrecision::BoundingBox,
            )
        })
    })
}
fn controlled(rules: RulesPreset, rock_safe: Option<bool>, stuck: bool) -> (Game, u64, u64) {
    let mut game = room(rules, 72.0);
    let bow = free_bow(&mut game, Vec2::new(72.0, 124.0));
    let arrow = game.spawn("oArrow", Vec2::new(79.0, 124.0));
    game.world
        .entity_mut(arrow)
        .unwrap()
        .data
        .insert("stuck".into(), f64::from(stuck));
    if let Some(safe) = rock_safe {
        let rock = game.spawn("oRock", Vec2::new(80.0, 120.0));
        let e = game.world.entity_mut(rock).unwrap();
        // Controlled source-call launch override, not a claimed input throw.
        e.vel = Vec2::new(-8.0, 0.0);
        e.data.insert("safe".into(), f64::from(safe));
    }
    assert_eq!(game.player.held, None);
    assert_eq!(game.player.health, 4);
    assert_eq!(game.data.get("arrows").copied().unwrap_or(0.0), 0.0);
    (game, bow, arrow)
}
#[test]
fn controlled_incoming_rock_stun_keeps_original_arrow_until_late_collection() {
    for rules in PRESETS {
        let (mut game, bow, arrow) = controlled(rules, Some(false), false);
        game.step(&InputFrame {
            down: true,
            ..attack()
        });
        assert_eq!(
            game.world.entity(bow).unwrap().value("new"),
            0.0,
            "FIXTURE_REAL_BOW_PICKUP"
        );
        assert_eq!(game.player.health, 2, "FIXTURE_ROCK_HURT_REQUIRED");
        assert!(game.player.is_stunned(), "FIXTURE_ROCK_STUN_REQUIRED");
        assert_eq!(game.player.held, None, "FIXTURE_LATE_BOW_RELEASE");
        assert!(
            game.world.entity(arrow).is_some(),
            "ARROW_RECOVERY_MUST_WAIT_UNTIL_AFTER_HURT"
        );
        // Source late hurt scrFireBow is unconditional even when not armed.
        assert_eq!(
            game.data["arrows"], 5.0,
            "LATE_HURT_BOW_SPEND_WITHOUT_PREMATURE_RECOVERY"
        );
        assert!(
            game.world
                .entities
                .iter()
                .any(|e| e.object == "oArrow" && e.id != arrow)
        );
    }
}
#[test]
fn safe_incoming_rock_does_not_block_normal_late_arrow_recovery() {
    for rules in PRESETS {
        let (mut game, bow, arrow) = controlled(rules, Some(true), false);
        game.step(&InputFrame {
            down: true,
            ..attack()
        });
        assert_eq!(game.player.health, 4);
        assert!(!game.player.is_stunned());
        assert_eq!(game.player.held, Some(bow));
        assert!(game.world.entity(arrow).is_none());
        assert_eq!(game.data["arrows"], 7.0);
    }
}
#[test]
fn first_bow_pickup_recovers_free_arrow_in_the_same_native_step() {
    for rules in PRESETS {
        let (mut game, bow, arrow) = controlled(rules, None, false);
        game.step(&InputFrame {
            down: true,
            ..attack()
        });
        assert_eq!(game.player.held, Some(bow));
        assert!(game.world.entity(arrow).is_none());
        assert_eq!(game.data["arrows"], 7.0);
        assert_eq!(game.player.health, 4);
    }
}
#[test]
fn stuck_arrow_is_not_recovered_after_real_bow_pickup() {
    for rules in PRESETS {
        let (mut game, bow, arrow) = controlled(rules, None, true);
        game.step(&InputFrame {
            down: true,
            ..attack()
        });
        assert_eq!(game.player.held, Some(bow));
        assert!(game.world.entity(arrow).is_some());
        assert_eq!(game.data["arrows"], 6.0);
    }
}
#[test]
fn ordinary_running_throw_wall_return_skips_recovery_on_the_rock_hurt_tick() {
    for rules in PRESETS {
        let mut game = room(rules, 96.0);
        for y in 0..8 {
            game.world.set_tile(11, y, "oHardBlock");
        }
        let rock = game.spawn("oRock", Vec2::new(97.0, 124.0));
        let bow = free_bow(&mut game, Vec2::new(120.0, 124.0));
        // Controlled authored source Arrow birth; default velocity/gravity.
        // It falls and settles under ordinary Steps; no midrun actor edits.
        // Fixed source bbox boundary:154-4=150 equals first hurt142+8;
        // prior Player139+8=147 stays clear. Root trace pins these origins.
        let arrow = game.spawn("oArrow", Vec2::new(154.0, 16.0));
        // Falling speed caps at8; inherited bounce0.5 yields shrinking arcs.
        // Fixed120 neutral ticks cover the conservative shrinking-arc bound and preserve
        // the source displacement periods2/3/5. Rest is still a mandatory
        // observed precondition, not inferred from the fixed wait alone.
        for _ in 0..120 {
            game.step(&InputFrame::default());
        }
        assert_eq!(
            game.player.pos,
            Vec2::new(96.0, 120.0),
            "FIXTURE_PREFIX_PLAYER_UNMOVED"
        );
        assert_eq!(game.player.health, 4, "FIXTURE_PREFIX_NO_DAMAGE");
        assert!(!game.player.is_stunned(), "FIXTURE_PREFIX_NOT_STUNNED");
        assert_eq!(game.player.held, None, "FIXTURE_PREFIX_EMPTY_HAND");
        assert_eq!(
            game.data.get("arrows").copied().unwrap_or(0.0),
            0.0,
            "FIXTURE_PREFIX_NO_ARROW_GRANT"
        );
        let settled_arrow = game.world.entity(arrow).unwrap();
        assert_eq!(
            settled_arrow.vel,
            Vec2::default(),
            "FIXTURE_PREFIX_ARROW_REST_REQUIRED"
        );
        assert_eq!(
            settled_arrow.pos.x, 154.0,
            "FIXTURE_PREFIX_ARROW_X_UNCHANGED"
        );
        assert_eq!(
            settled_arrow.value("stuck"),
            0.0,
            "FIXTURE_PREFIX_ARROW_FREE"
        );
        assert_eq!(
            settled_arrow.value("rotation"),
            0.0,
            "FIXTURE_PREFIX_ARROW_ROTATION_ZERO"
        );
        assert!(!settled_arrow.held);
        assert!(
            settled_arrow.pos.y >= 122.0 && settled_arrow.pos.y <= 124.0,
            "FIXTURE_PREFIX_ARROW_ON_SAME_FLOOR"
        );
        assert!(
            game.world
                .solid_at(settled_arrow.pos.x, settled_arrow.pos.y + 5.0)
        );
        assert!(
            settled_arrow.mask_contains(
                Vec2::new(150.0, settled_arrow.pos.y - 1.0),
                CollisionPrecision::BoundingBox
            ),
            "FIXTURE_PREFIX_ARROW_LEFT150_INCLUDED"
        );
        assert!(
            !settled_arrow.mask_contains(
                Vec2::new(149.0, settled_arrow.pos.y - 1.0),
                CollisionPrecision::BoundingBox
            ),
            "FIXTURE_PREFIX_ARROW_LEFT149_EXCLUDED"
        );
        for (id, x) in [(rock, 97.0), (bow, 120.0)] {
            let e = game.world.entity(id).unwrap();
            assert_eq!(e.pos.x, x, "FIXTURE_PREFIX_STOCK_X_UNCHANGED");
            assert_eq!(e.vel, Vec2::default(), "FIXTURE_PREFIX_STOCK_REST");
            assert!(!e.held);
            assert_eq!(e.value("safe"), 0.0, "FIXTURE_PREFIX_STOCK_SAFE_DEFAULT");
            assert!(
                e.pos.y >= 122.0 && e.pos.y <= 124.0,
                "FIXTURE_PREFIX_STOCK_SAME_FLOOR"
            );
            assert!(game.world.solid_at(e.pos.x, e.pos.y + 5.0));
        }
        assert_eq!(game.world.entity(bow).unwrap().value("cost"), 0.0);
        assert_eq!(
            game.world.entity(bow).unwrap().value("new"),
            1.0,
            "FIXTURE_PREFIX_BOW_UNPICKED"
        );
        game.step(&InputFrame {
            down: true,
            ..attack()
        });
        assert_eq!(game.player.held, Some(rock), "FIXTURE_REAL_ROCK_PICKUP");
        game.step(&InputFrame {
            right: true,
            ..release()
        });
        for _ in 0..6 {
            game.step(&InputFrame {
                right: true,
                ..Default::default()
            });
        }
        assert!(game.player.vel.x > 0.0, "FIXTURE_RUNNING_THROW_REQUIRED");
        game.step(&InputFrame {
            right: true,
            ..attack()
        });
        assert_eq!(game.player.held, None);
        assert!(
            game.world.entity(rock).unwrap().vel.x > 8.0,
            "FIXTURE_THROW_SPEED_EXCEEDS_STANDING"
        );
        game.step(&InputFrame {
            right: true,
            ..release()
        });
        game.step(&InputFrame {
            right: true,
            down: true,
            ..attack()
        });
        assert_eq!(
            game.player.held,
            Some(bow),
            "FIXTURE_REAL_BOW_PICKUP_BEFORE_RETURN"
        );
        let mut bounced = false;
        for _ in 0..48 {
            let rock_before = game.world.entity(rock).unwrap().clone();
            bounced |= rock_before.vel.x < -4.0;
            let arrow_before = game.world.entity(arrow).cloned();
            let ammo = game.data["arrows"];
            let health = game.player.health;
            let held = game.player.held;
            let origin = game.player.pos;
            game.step(&InputFrame {
                right: true,
                ..Default::default()
            });
            if game.player.health < health {
                assert_eq!(health - game.player.health, 2, "FIXTURE_ROCK_DAMAGE_TWO");
                assert!(
                    bounced && rock_before.vel.x < -4.0,
                    "FIXTURE_FAST_RETURNING_ROCK"
                );
                // Snapshots are before Alarm. Item Alarm2 clears safe first;
                // Rock450 own movement follows Player319 in this isolated room.
                let safe_at_player = if rock_before.alarms[2] == 1 {
                    0.0
                } else {
                    rock_before.value("safe")
                };
                assert_eq!(
                    safe_at_player, 0.0,
                    "FIXTURE_SOURCE_ALARM_UNSAFE_AT_PLAYER_HURT"
                );
                assert_eq!(
                    game.world.entity(rock).unwrap().value("safe"),
                    0.0,
                    "FIXTURE_ACTUAL_ALARM_CLEAR_CONFIRMED"
                );
                assert!(
                    rock_before.value("safe") == 0.0 || rock_before.alarms[2] == 1,
                    "FIXTURE_UNSAFE_FAST_RETURNING_ROCK"
                );
                assert_eq!(held, Some(bow), "FIXTURE_HELD_BOW_ON_CONTACT_TICK");
                let a = arrow_before.expect("FIXTURE_ARROW_NOT_EARLIER_RECOVERED");
                assert!(
                    a.vel.x.abs() < 1.0 && a.vel.y.abs() < 1.0 && a.value("stuck") == 0.0,
                    "FIXTURE_SLOW_FREE_ARROW_ON_CONTACT_TICK"
                );
                assert_eq!(a.pos.x, 154.0, "FIXTURE_FIXED_ARROW_X");
                assert_eq!(
                    a.value("rotation"),
                    0.0,
                    "FIXTURE_STOCK_ARROW_ZERO_X_ROTATION"
                );
                assert!(
                    a.mask_contains(
                        Vec2::new(a.pos.x - 4.0, a.pos.y - 1.0),
                        CollisionPrecision::BoundingBox
                    ),
                    "FIXTURE_ARROW_BBOX_LEFT_MINUS_FOUR_INCLUDED"
                );
                assert!(
                    !a.mask_contains(
                        Vec2::new(a.pos.x - 5.0, a.pos.y - 1.0),
                        CollisionPrecision::BoundingBox
                    ),
                    "FIXTURE_ARROW_BBOX_ONE_PIXEL_LEFT_EXCLUDED"
                );
                assert_eq!(game.player.pos.x, 142.0, "FIXTURE_FIXED_PLAYER_HURT_ORIGIN");
                assert_eq!(origin.x, 139.0, "FIXTURE_FIXED_PRIOR_PLAYER_ORIGIN");
                assert!(
                    !square_hit(&a, origin),
                    "FIXTURE_PRIOR_PLAYER_ARROW_GATE_CLEAR"
                );
                // Player movement happens within its own Step before hurt.
                // Later Rock/Arrow own Steps do not move Player in this room.
                assert!(
                    square_hit(&a, game.player.pos),
                    "FIXTURE_ARROW_CONTACT_GATE"
                );
                assert!(game.player.is_stunned());
                assert_eq!(game.player.held, None);
                assert!(
                    game.world.entity(arrow).is_some(),
                    "NATURAL_RETURN_RECOVERY_MUST_WAIT_UNTIL_AFTER_HURT"
                );
                assert_eq!(game.data["arrows"], ammo - 1.0);
                assert!(
                    game.events
                        .iter()
                        .any(|e| matches!(e, GameEvent::Sound { name, .. } if name == "sndHurt"))
                );
                break;
            }
            assert_eq!(
                game.player.held,
                Some(bow),
                "FIXTURE_BOW_RETAINED_UNTIL_RETURN"
            );
            if game.world.entity(arrow).is_none() {
                panic!("FIXTURE_ARROW_EARLY_RECOVERY_BEFORE_RETURN");
            }
        }
        assert_eq!(game.player.health, 2, "FIXTURE_RETURN_MUST_REACH_ROCK_HURT");
    }
}
