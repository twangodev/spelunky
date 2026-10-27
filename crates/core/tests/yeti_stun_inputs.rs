//! Authored room, source-default Yeti/Pistol, ordinary pickup/fire, no mid-run writes.
use spelunky_core::{catalog, level::Level, *};

fn room(rules: RulesPreset) -> (Game, u64, u64) {
    let mut level = Level::default();
    level.set_tile(6, 8, '@').unwrap();
    level.set_tile(34, 8, 'X').unwrap();
    level.set_tile(9, 8, 'y').unwrap();
    // The supported ledge stops WALK and establishes a real positive IDLE counter.
    for x in 0..=10 {
        level.set_tile(x, 9, '1').unwrap();
    }
    // A lower platform catches ordinary shot knockback beyond the upper ledge.
    for x in 0..=20 {
        level.set_tile(x, 11, '1').unwrap();
    }
    // Keep the real Pistol Bullet knockback and first landing inside
    // the player's active viewport; the wall is beyond the IDLE ledge.
    for y in 7..=10 {
        level.set_tile(18, y, '1').unwrap();
    }
    // Select only initial source preconditions, never a hit/result or later route.
    for seed in 0..128 {
        let mut game = Game::shell(seed, rules);
        game.load_custom(&level, "yeti-stun-landing", "");
        if game.rng.clone().range(2) != 1 {
            continue;
        }
        let enemies: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|e| e.alive && catalog::is_a(&e.object, "oEnemy"))
            .collect();
        if enemies.len() != 1 || enemies[0].object != "oYeti" {
            continue;
        }
        let yeti = enemies[0].id;
        let source = game.world.entity(yeti).unwrap();
        assert_eq!(source.health, 5);
        assert_eq!(source.state, 0);
        assert_eq!(source.vel, Vec2::new(2.5, 0.0));
        assert_eq!(source.value("counter"), 0.0);
        assert_eq!(source.value("bounced"), 0.0);
        let pistol = game.spawn("oPistol", game.player.pos);
        return (game, yeti, pistol);
    }
    panic!("bounded initial-only quiet/right-facing source fixture must exist");
}

fn pickup_and_wait_for_idle(game: &mut Game, yeti: u64, pistol: u64) {
    game.step(&InputFrame {
        down: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert_eq!(game.player.held, Some(pistol), "actual pickup input");
    let mut saw_walk = false;
    for _ in 0..100 {
        let actor = game.world.entity(yeti).unwrap();
        saw_walk |= actor.state == 1;
        assert_eq!(actor.health, 5);
        assert_eq!(game.player.health, 4.0);
        assert!(!game.world.entities.iter().any(|e| e.object == "oBlood"));
        if actor.state == 0 && actor.value("counter") >= 20.0 {
            assert!(saw_walk);
            assert_eq!(actor.vel.x, 0.0);
            return;
        }
        game.step(&InputFrame::default());
    }
    panic!("ordinary default Yeti must naturally enter a positive-counter IDLE pause");
}

fn shoot_paused_yeti(game: &mut Game, yeti: u64, pistol: u64) {
    pickup_and_wait_for_idle(game, yeti, pistol);
    game.step(&InputFrame {
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    let mut saw_bullet = game
        .world
        .entities
        .iter()
        .any(|e| e.object == "oBullet" && e.vel.x > 0.0);
    let mut hit = false;
    for _ in 0..24 {
        let actor = game.world.entity(yeti).unwrap();
        if actor.health < 5 {
            assert_eq!(
                actor.health, 1,
                "one actual pistol Bullet deals source four damage"
            );
            assert_eq!(actor.state, 98);
            assert_eq!(actor.timer, 20);
            assert!(
                actor.value("counter") > 0.0,
                "real pre-hit idle pause remains in port alias"
            );
            assert_eq!(actor.value("bounced"), 0.0);
            assert!(saw_bullet);
            hit = true;
            break;
        }
        game.step(&InputFrame::default());
        saw_bullet |= game
            .world
            .entities
            .iter()
            .any(|e| e.object == "oBullet" && e.vel.x > 0.0);
    }
    assert!(
        hit,
        "ordinary pickup/fire must genuinely hit the naturally paused Yeti"
    );
}

#[test]
fn ordinary_pistol_fired_away_from_yeti_does_not_stun_or_create_landing_blood() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, yeti, pistol) = room(rules);
        pickup_and_wait_for_idle(&mut game, yeti, pistol);
        game.step(&InputFrame {
            left: true,
            ..Default::default()
        });
        assert_eq!(game.player.facing, -1);
        game.step(&InputFrame {
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert!(
            game.world
                .entities
                .iter()
                .any(|e| e.object == "oBullet" && e.vel.x < 0.0)
        );
        for _ in 0..24 {
            game.step(&InputFrame::default());
            let actor = game.world.entity(yeti).unwrap();
            assert_eq!(actor.health, 5);
            assert_ne!(actor.state, 98);
            assert!(!game.world.entities.iter().any(|e| e.object == "oBlood"));
            assert_eq!(game.player.health, 4.0);
            assert_eq!(game.player.held, Some(pistol));
        }
    }
}

// Only authored terrain differs from the source-default pickup/fire fixture.
fn settlement_room(rules: RulesPreset, ceiling: bool, wall: bool) -> (Game, u64, u64) {
    let mut level = Level::default();
    level.set_tile(6, 8, '@').unwrap();
    level.set_tile(34, 8, 'X').unwrap();
    level.set_tile(9, 8, 'y').unwrap();
    // The supported ledge stops WALK and establishes a real positive IDLE counter.
    for x in 0..=10 {
        level.set_tile(x, 9, '1').unwrap();
    }
    // A lower platform catches ordinary shot knockback beyond the upper ledge.
    for x in 0..=20 {
        level.set_tile(x, 11, '1').unwrap();
    }
    // Keep the real Pistol Bullet knockback and first landing inside
    // the player's active viewport; the wall is beyond the IDLE ledge.
    for y in 7..=10 {
        level.set_tile(if wall { 13 } else { 18 }, y, '1').unwrap();
    }
    if ceiling {
        // Start above the idle ledge's edge so the stunned Yeti's head reaches
        // the roof before its horizontal side probe can stop it.
        for x in 9..=13 {
            level.set_tile(x, 7, '1').unwrap();
        }
    }
    // Select only initial source preconditions, never a hit/result or later route.
    for seed in 0..128 {
        let mut game = Game::shell(seed, rules);
        game.load_custom(&level, "yeti-stun-landing", "");
        if game.rng.clone().range(2) != 1 {
            continue;
        }
        let enemies: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|e| e.alive && catalog::is_a(&e.object, "oEnemy"))
            .collect();
        if enemies.len() != 1 || enemies[0].object != "oYeti" {
            continue;
        }
        let yeti = enemies[0].id;
        let source = game.world.entity(yeti).unwrap();
        assert_eq!(source.health, 5);
        assert_eq!(source.state, 0);
        assert_eq!(source.vel, Vec2::new(2.5, 0.0));
        assert_eq!(source.value("counter"), 0.0);
        assert_eq!(source.value("bounced"), 0.0);
        let pistol = game.spawn("oPistol", game.player.pos);
        return (game, yeti, pistol);
    }
    panic!("bounded initial-only quiet/right-facing source fixture must exist");
}

fn literal_motion_and_contacts(
    game: &Game,
    body: &Entity,
    ceiling: bool,
    wall: bool,
) -> (Vec2, [bool; 4]) {
    let mut blocks = Vec::new();
    for x in 0..=10 {
        blocks.push((16 + x * 16, 160));
    }
    for x in 0..=20 {
        blocks.push((16 + x * 16, 192));
    }
    for y in 7..=10 {
        blocks.push((16 + if wall { 13 * 16 } else { 18 * 16 }, 16 + y * 16));
    }
    if ceiling {
        for x in 9..=13 {
            blocks.push((16 + x * 16, 128));
        }
    }
    let hit = |x0: f64, y0: f64, x1: f64, y1: f64| {
        let (x0, y0, x1, y1) = (
            x0.round_ties_even(),
            y0.round_ties_even(),
            x1.round_ties_even(),
            y1.round_ties_even(),
        );
        blocks.iter().any(|&(x, y)| {
            x0 <= f64::from(x + 15)
                && x1 >= f64::from(x)
                && y0 <= f64::from(y + 15)
                && y1 >= f64::from(y)
        })
    };
    let displacement = |v: f64| {
        let f = v.abs().fract();
        let extra = f > 0.0
            && (game.engine_time() + 1).is_multiple_of((1.0 / f).round_ties_even().max(1.0) as u64);
        (v.abs().floor() + f64::from(extra)) * v.signum()
    };
    let mut p = body.pos;
    let l = body.value("collision_left");
    let t = body.value("collision_top");
    let r = body.value("collision_right");
    let b = body.value("collision_bottom");
    let dx = displacement(body.vel.x);
    let dy = displacement(body.vel.y);
    for _ in 0..dx.abs() as usize {
        let x = if dx < 0.0 { p.x + l - 1.0 } else { p.x + r };
        if hit(x, p.y + t + 5.0, x, p.y + b - 1.0) {
            break;
        }
        p.x += dx.signum();
    }
    for _ in 0..dy.abs() as usize {
        let y = if dy < 0.0 { p.y + t - 1.0 } else { p.y + b };
        if hit(p.x + l, y, p.x + r - 1.0, y) {
            break;
        }
        p.y += dy.signum();
    }
    (
        p,
        [
            hit(p.x + l - 1.0, p.y + t, p.x + l - 1.0, p.y + b - 1.0),
            hit(p.x + r, p.y + t, p.x + r, p.y + b - 1.0),
            hit(p.x + l, p.y + t - 1.0, p.x + r - 1.0, p.y + t - 1.0),
            hit(p.x + l, p.y + b, p.x + r - 1.0, p.y + b),
        ],
    )
}

#[test]
fn ordinary_pistol_yeti_wall_impact_uses_full_cached_side_and_one_pixel_nudge() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, yeti, pistol) = settlement_room(rules, false, true);
        shoot_paused_yeti(&mut game, yeti, pistol);
        let mut wall_contact = false;
        for _ in 0..80 {
            let before = game.world.entity(yeti).unwrap().clone();
            if before.state != 98 {
                break;
            }
            let v = before.vel.x;
            let fraction = v.abs().fract();
            let next_tick = game.engine_time() + 1;
            let extra = fraction > 0.0
                && next_tick.is_multiple_of((1.0 / fraction).round_ties_even().max(1.0) as u64);
            let dx = (v.abs().floor() + f64::from(extra)) * v.signum();
            game.step(&InputFrame::default());
            let actor = game.world.entity(yeti).unwrap();
            // Source collision bounds right14; wall at16+13*16=224. moveTo
            // stops at210, colRight is captured there, then scrCheckCollisions
            // nudges x-1 independently of reflected speed and ordinary friction.
            if v > 0.0 && before.pos.x + dx >= 210.0 {
                assert_eq!(
                    actor.pos.x, 209.0,
                    "source wall settlement after movement: before={before:?}"
                );
                assert!(actor.vel.x <= 0.0);
                assert_eq!(actor.value("collision_left"), 2.0);
                assert_eq!(actor.value("collision_top"), 6.0);
                assert_eq!(actor.health, 1);
                wall_contact = true;
                break;
            }
            assert_eq!(game.player.health, 4.0);
        }
        assert!(
            wall_contact,
            "ordinary fired Bullet must drive the actual stunned Yeti to the authored wall"
        );
    }
}

#[test]
fn ordinary_pistol_yeti_ceiling_impact_uses_full_cached_top_and_one_pixel_nudge() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, yeti, pistol) = settlement_room(rules, true, false);
        shoot_paused_yeti(&mut game, yeti, pistol);
        let mut ceiling_contact = false;
        for _ in 0..40 {
            let before = game.world.entity(yeti).unwrap().clone();
            if before.state != 98 {
                break;
            }
            let (moved, contacts) = literal_motion_and_contacts(&game, &before, true, false);
            game.step(&InputFrame::default());
            let actor = game.world.entity(yeti).unwrap();
            if contacts[2] && !contacts[3] {
                assert_eq!(
                    actor.pos.y,
                    moved.y + 1.0,
                    "source top contact nudges down after full moveTo"
                );
                assert_eq!(
                    actor.vel.y,
                    (before.vel.y + 0.6).min(10.0),
                    "source moveTo/top settlement retains upward stored velocity"
                );
                assert_eq!(actor.health, 1);
                ceiling_contact = true;
                break;
            }
            assert_eq!(game.player.health, 4.0);
        }
        assert!(
            ceiling_contact,
            "ordinary fired Bullet must reach the authored ceiling while airborne; final yeti={:?}",
            game.world.entity(yeti)
        );
    }
}
