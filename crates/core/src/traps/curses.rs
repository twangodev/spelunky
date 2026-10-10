//! Kali's iron ball and presentation-room chain variants. Source: oBall/2,
//! oChain/2 and the inherited oItem Step. Player tether limits run in player.rs.
use crate::model::{Entity, Game, PlayerState, Rect, Vec2};

pub(super) fn tick(game: &mut Game) {
    // oLevel Create restores the punishment on each new campaign room, including
    // after the prior ball was destroyed by lava. Never regenerate it mid-room.
    if game.level_tick == 1
        && !game.flags.contains("hub")
        && game.data.get("kali_punish").copied().unwrap_or(0.0) >= 2.0
        && !game
            .world
            .entities
            .iter()
            .any(|e| e.alive && e.object == "oBall")
    {
        spawn_chain(game, game.player.pos, false);
    }
    let ids: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && matches!(e.object.as_str(), "oBall" | "oBall2"))
        .map(|e| e.id)
        .collect();
    for id in ids {
        let Some(index) = game
            .world
            .entities
            .iter()
            .position(|e| e.id == id && e.alive)
        else {
            continue;
        };
        let mut ball = game.world.entities[index].clone();
        init(&mut ball);
        item_physics(game, &mut ball);
        if ball.alive {
            // Inherited item impacts occur before the child ball-follow event.
            impact(game, &ball);
            if ball.object == "oBall" {
                follow_player(game, &mut ball);
            } else if let Some(dummy) = game
                .world
                .entities
                .iter()
                .find(|e| e.alive && e.object == "oPDummy")
            {
                if bounds_distance(
                    sprite_edges(&ball.sprite, ball.pos, ball.bounds()),
                    sprite_edges(&dummy.sprite, dummy.pos, dummy.bounds()),
                ) >= 24.0
                {
                    ball.pos.x = dummy.pos.x - 24.0;
                }
            }
        }
        game.world.entities[index] = ball;
    }
    chains(game);
}

fn init(ball: &mut Entity) {
    if ball.value("curse_initialized") != 0.0 {
        return;
    }
    ball.data.insert("curse_initialized".into(), 1.0);
    ball.data.insert("heavy".into(), 1.0);
    ball.data.insert("canPickUp".into(), 1.0);
    ball.data.insert("gravity".into(), 1.0);
    ball.data.insert("bounceFactor".into(), 0.5);
    ball.data.insert("frictionFactor".into(), 0.3);
    for (key, value) in [
        ("collision_left", -5.0),
        ("collision_top", -5.0),
        ("collision_right", 5.0),
        ("collision_bottom", 5.0),
    ] {
        ball.data.insert(key.into(), value);
    }
}

fn spawn_chain(game: &mut Game, pos: Vec2, presentation: bool) {
    game.world
        .spawn(if presentation { "oBall2" } else { "oBall" }, pos);
    for link in 1..=4 {
        let id = game
            .world
            .spawn(if presentation { "oChain2" } else { "oChain" }, pos);
        game.world
            .entity_mut(id)
            .unwrap()
            .data
            .insert("link_val".into(), f64::from(link));
    }
}

fn bounds_at(pos: Vec2) -> Rect {
    Rect::new(pos.x - 5.0, pos.y - 5.0, 10.0, 10.0)
}
fn blocked(game: &Game, ball: &Entity, pos: Vec2) -> bool {
    game.world.collides_except(bounds_at(pos), ball.id)
}
fn displacement(v: f64, tick: u64) -> i32 {
    let f = v.abs().fract();
    let period = if f > 0.0 {
        (1.0 / f).round_ties_even() as u64
    } else {
        0
    };
    (v.abs().floor() as i32 + i32::from(period > 0 && tick.is_multiple_of(period)))
        * if v < 0.0 { -1 } else { 1 }
}
fn liquid(game: &Game, pos: Vec2, name: &str) -> bool {
    game.world
        .tile((pos.x / 16.0).floor() as i32, (pos.y / 16.0).floor() as i32)
        .is_some_and(|t| t.object == name)
        || game
            .world
            .entities
            .iter()
            .any(|e| e.alive && e.object == name && e.sprite_bounds().contains(pos))
}
fn item_physics(game: &Game, ball: &mut Entity) {
    let active = ball.pos.x > game.camera.x - 16.0
        && ball.pos.x < game.camera.x + 336.0
        && ball.pos.y > game.camera.y - 16.0
        && ball.pos.y < game.camera.y + 256.0;
    if !active {
        return;
    }
    if ball.held {
        ball.vel = Vec2::default();
        ball.pos = Vec2::new(
            game.player.pos.x + f64::from(game.player.facing) * 4.0,
            game.player.pos.y
                - if game.player.state == PlayerState::Ducking && game.player.vel.x.abs() < 2.0 {
                    2.0
                } else {
                    4.0
                },
        );
        if game.player.held != Some(ball.id) {
            ball.held = false;
        }
        return;
    }
    if game.world.solid_at(ball.pos.x, ball.pos.y) {
        let left = blocked(game, ball, Vec2::new(ball.pos.x - 1.0, ball.pos.y));
        let right = blocked(game, ball, Vec2::new(ball.pos.x + 1.0, ball.pos.y));
        let top = blocked(game, ball, Vec2::new(ball.pos.x, ball.pos.y - 1.0));
        let bottom = blocked(game, ball, Vec2::new(ball.pos.x, ball.pos.y + 1.0));
        if top && !bottom {
            ball.pos.y += 1.0;
        } else if left && !right {
            ball.pos.x += 1.0;
        } else if right && !left {
            ball.pos.x -= 1.0;
        } else {
            ball.vel = Vec2::default();
        }
        return;
    }
    for (horizontal, v) in [(true, ball.vel.x), (false, ball.vel.y)] {
        let d = displacement(v, game.engine_time());
        for _ in 0..d.unsigned_abs() {
            let p = Vec2::new(
                ball.pos.x
                    + if horizontal {
                        f64::from(d.signum())
                    } else {
                        0.0
                    },
                ball.pos.y
                    + if horizontal {
                        0.0
                    } else {
                        f64::from(d.signum())
                    },
            );
            if blocked(game, ball, p) {
                break;
            }
            ball.pos = p;
        }
    }
    let left = blocked(game, ball, Vec2::new(ball.pos.x - 1.0, ball.pos.y));
    let right = blocked(game, ball, Vec2::new(ball.pos.x + 1.0, ball.pos.y));
    let bottom = blocked(game, ball, Vec2::new(ball.pos.x, ball.pos.y + 1.0));
    let top = blocked(game, ball, Vec2::new(ball.pos.x, ball.pos.y - 1.0));
    ball.data.insert("colBot".into(), f64::from(bottom));
    if !left && !right {
        ball.data.insert("stuck".into(), 0.0);
    }
    if !bottom && ball.value("stuck") == 0.0 {
        ball.vel.y = (ball.vel.y + ball.value("gravity")).min(8.0);
    }
    if left || right {
        ball.vel.x *= -0.5;
    }
    if bottom {
        ball.vel.y = if ball.vel.y > 1.0 {
            -ball.vel.y * 0.5
        } else {
            0.0
        };
        ball.vel.x = if ball.vel.x.abs() < 0.1 {
            0.0
        } else {
            ball.vel.x * 0.3
        };
        if ball.vel.y.abs() < 1.0 {
            ball.pos.y -= 1.0;
            if !blocked(game, ball, Vec2::new(ball.pos.x, ball.pos.y + 1.0)) {
                ball.pos.y += 1.0;
            }
            ball.vel.y = 0.0;
        }
    }
    if left && !right && ball.value("stuck") == 0.0 {
        ball.pos.x += 1.0;
    } else if right && ball.value("stuck") == 0.0 {
        ball.pos.x -= 1.0;
    }
    if top {
        if ball.vel.y < 0.0 {
            ball.vel.y *= -0.8;
        } else {
            ball.pos.y += 1.0;
        }
    }
    // oItem inherits no water drag: only lava arrests motion and destroys it
    // when submerged beyond its upper point. This matters for curse escape.
    let lava = Rect::new(ball.pos.x - 3.0, ball.pos.y - 3.0, 6.0, 6.0);
    let in_lava = game.world.tiles.iter().enumerate().any(|(i, t)| {
        t.object == "oLava"
            && Rect::new(
                (i % game.world.width) as f64 * 16.0,
                (i / game.world.width) as f64 * 16.0,
                16.0,
                16.0,
            )
            .intersects(lava)
    }) || game
        .world
        .entities
        .iter()
        .any(|e| e.alive && e.object == "oLava" && e.sprite_bounds().intersects(lava));
    ball.data
        .insert("gravity".into(), if in_lava { 0.0 } else { 0.6 });
    if in_lava {
        ball.vel = Vec2::default();
        ball.pos.y += 0.05;
    }
    if liquid(game, Vec2::new(ball.pos.x, ball.pos.y - 5.0), "oLava") {
        ball.alive = false;
    }
}

pub(super) fn bounds_distance(a: Rect, b: Rect) -> f64 {
    let dx = (a.x - (b.x + b.w)).max(b.x - (a.x + a.w)).max(0.0);
    let dy = (a.y - (b.y + b.h)).max(b.y - (a.y + a.h)).max(0.0);
    dx.hypot(dy)
}
fn sprite_edges(sprite: &str, pos: Vec2, fallback: Rect) -> Rect {
    let Some(s) = crate::catalog::sprite_def(sprite) else {
        return fallback;
    };
    Rect::new(
        pos.x + f64::from(s.bounds[0] - s.origin[0]),
        pos.y + f64::from(s.bounds[1] - s.origin[1]),
        f64::from(s.bounds[2] - s.bounds[0]),
        f64::from(s.bounds[3] - s.bounds[1]),
    )
}
fn follow_player(game: &Game, ball: &mut Entity) {
    if bounds_distance(
        sprite_edges(&ball.sprite, ball.pos, ball.bounds()),
        sprite_edges(&game.player.sprite, game.player.pos, game.player.bounds()),
    ) >= 24.0
    {
        let dx = game.player.pos.x - ball.pos.x;
        if dx.abs() >= 24.0 || ball.value("colBot") == 0.0 {
            if dx.abs() < 1.0 {
                ball.pos.x = game.player.pos.x;
                ball.vel.x = 0.0;
            }
            if game.player.pos.x > ball.pos.x {
                if game.player.vel.x > 0.0 && ball.pos.y >= game.player.pos.y {
                    ball.vel.x = game.player.vel.x;
                } else if ball.vel.x < 0.0 {
                    ball.vel.x *= -0.5;
                } else if ball.vel.x == 0.0 {
                    ball.vel.x = 2.0;
                }
            } else if game.player.pos.x < ball.pos.x {
                if game.player.vel.x < 0.0 && ball.pos.y >= game.player.pos.y {
                    ball.vel.x = game.player.vel.x;
                } else if ball.vel.x > 0.0 {
                    ball.vel.x *= -0.5;
                } else if ball.vel.x == 0.0 {
                    ball.vel.x = -2.0;
                }
            }
        } else {
            ball.vel.x *= 0.5;
            if ball.vel.x.abs() < 0.5 {
                ball.vel.x = 0.0;
            }
        }
        if (game.player.pos.y - ball.pos.y).abs() >= 24.0 && game.player.pos.y < ball.pos.y {
            ball.vel.y = 0.0;
        }
    } else if ball.value("colBot") != 0.0 {
        ball.vel.x = 0.0;
    }
}
fn chains(game: &mut Game) {
    let ball = game
        .world
        .entities
        .iter()
        .find(|e| e.alive && e.object == "oBall")
        .map(|e| e.pos);
    let ball2 = game
        .world
        .entities
        .iter()
        .find(|e| e.alive && e.object == "oBall2")
        .map(|e| e.pos);
    let dummy = game
        .world
        .entities
        .iter()
        .find(|e| e.alive && e.object == "oPDummy")
        .map(|e| e.pos)
        .unwrap_or(Vec2::new(280.0, 184.0));
    for e in &mut game.world.entities {
        if !e.alive || !matches!(e.object.as_str(), "oChain" | "oChain2") {
            continue;
        }
        let (origin, end) = if e.object == "oChain" {
            (ball, game.player.pos)
        } else {
            (ball2, dummy)
        };
        if let Some(origin) = origin {
            let link = e
                .data
                .get("link_val")
                .or_else(|| e.data.get("linkVal"))
                .copied()
                .unwrap_or(2.0);
            e.pos = origin.lerp(end, link / 4.0);
        } else {
            e.alive = false;
        }
    }
}
fn impact(game: &mut Game, ball: &Entity) {
    if ball.held || ball.vel.x.abs() <= 2.0 && ball.vel.y.abs() <= 2.0 {
        return;
    }
    let strike = Rect::new(ball.pos.x - 2.0, ball.pos.y - 2.0, 5.0, 5.0);
    let mut sound = Vec::new();
    let mut explosions = Vec::new();
    for target in &mut game.world.entities {
        if !target.alive
            || target.id == ball.id
            || !strike.intersects(target.sprite_bounds())
            || target.value("invincible") != 0.0
            || target.object == "oMagmaMan"
        {
            continue;
        }
        if !crate::enemies::is_enemy(&target.object) && target.object != "oDamsel" {
            continue;
        }
        match target.object.as_str() {
            "oGhost" | "oScarab" | "oOlmec" | "oMagma" => continue,
            "oCaveman" | "oManTrap" | "oVampire" | "oYeti" | "oHawkman" => {
                if target.state == 98 {
                    continue;
                }
                target.health -= 1;
                target.state = 98;
                target.timer = target.value("stun_time") as u32;
                target.vel.y = -6.0;
            }
            "oShopkeeper" => {
                if target.state >= 98 {
                    continue;
                }
                target.health -= 1;
                target.state = 2;
                target.vel.y = -6.0;
                game.flags.insert("shopkeepers_angry".into());
            }
            "oDamsel" => {
                if target.state == 99 || target.value("hit") > 0.0 {
                    continue;
                }
                target.held = false;
                target.health -= 1;
                target.state = 2;
                target.timer = 120;
                target.data.insert("hit".into(), 10.0);
                target.vel.y = -6.0;
                if game.player.held == Some(target.id) {
                    game.player.held = None;
                }
                if target.value("forSale") != 0.0 {
                    game.flags.insert("shopkeepers_angry".into());
                }
            }
            "oGiantSpider" | "oTombLord" => {
                if target.value("hit_cooldown") > 0.0 {
                    continue;
                }
                target.health -= 1;
                target.data.insert(
                    "hit_cooldown".into(),
                    if target.object == "oGiantSpider" {
                        10.0
                    } else {
                        20.0
                    },
                );
            }
            "oAlienBoss" => {
                if target.state == 99 || target.sprite == "sAlienBossHurt" {
                    continue;
                }
                target.health -= 1;
                target.sprite = "sAlienBossHurt".into();
                target.data.insert("animation_speed".into(), 0.8);
            }
            "oUFO" => {
                target.alive = false;
                target.health = 0;
                explosions.push(Vec2::new(target.pos.x + 8.0, target.pos.y + 8.0));
            }
            _ => {
                target.health -= 1;
            }
        }
        target.vel.x = ball.vel.x * 0.3;
        target.data.insert("assaulted".into(), 1.0);
        if target.health <= 0 {
            target.data.insert("killed_by_player".into(), 1.0);
        }
        sound.push(target.pos);
    }
    for pos in sound {
        game.sound("sndHit", pos);
    }
    for pos in explosions {
        game.world.spawn("oExplosion", pos);
        game.sound("sndExplosion", pos);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RulesPreset;
    fn room() -> Game {
        let mut g = Game::shell(1, RulesPreset::Classic);
        g.player.pos = Vec2::new(120.0, 80.0);
        g
    }
    #[test]
    fn moving_curse_ball_hits_damsel_in_game_step() {
        let mut game = room();
        let damsel = game.world.spawn("oDamsel", Vec2::new(72.0, 80.0));
        game.world
            .entity_mut(damsel)
            .unwrap()
            .data
            .insert("forSale".into(), 1.0);
        let ball = game.world.spawn("oBall", Vec2::new(72.0, 72.0));
        game.world.entity_mut(ball).unwrap().vel.y = 4.0;
        game.step(&crate::InputFrame::default());
        let damsel = game.world.entity(damsel).unwrap();
        assert_eq!(damsel.health, 3);
        assert_eq!(damsel.state, 2);
        assert_eq!(damsel.timer, 120);
        assert_eq!(damsel.value("hit"), 10.0);
        assert_eq!(damsel.vel.y, -6.0);
        assert!(game.flags.contains("shopkeepers_angry"));
    }

    #[test]
    fn hub_room_does_not_restore_campaign_punishment_ball() {
        let mut game = room();
        game.load_hub(crate::hub::HubKind::Title);
        game.data.insert("kali_punish".into(), 2.0);
        game.step(&crate::InputFrame::default());
        assert!(!game.world.entities.iter().any(|e| e.object == "oBall"));
    }
    #[test]
    fn links_interpolate_all_four_quarters_and_vanish_with_ball() {
        let mut g = room();
        spawn_chain(&mut g, Vec2::new(40.0, 80.0), false);
        chains(&mut g);
        let links: Vec<_> = g
            .world
            .entities
            .iter()
            .filter(|e| e.object == "oChain")
            .map(|e| e.pos.x)
            .collect();
        assert_eq!(links, vec![60.0, 80.0, 100.0, 120.0]);
        g.world
            .entities
            .iter_mut()
            .find(|e| e.object == "oBall")
            .unwrap()
            .alive = false;
        chains(&mut g);
        assert!(
            g.world
                .entities
                .iter()
                .filter(|e| e.object == "oChain")
                .all(|e| !e.alive)
        );
    }
    #[test]
    fn ball_uses_bounds_gap_and_follows_player_velocity() {
        let g = room();
        let mut e = Entity::new(1, "oBall", Vec2::new(70.0, 80.0));
        init(&mut e);
        let mut g = g;
        g.player.vel.x = 3.0;
        follow_player(&g, &mut e);
        assert_eq!(e.vel.x, 3.0);
        e.pos.x = 95.0;
        e.vel.x = 2.0;
        e.data.insert("colBot".into(), 1.0);
        follow_player(&g, &mut e);
        assert_eq!(e.vel.x, 0.0);
    }
    #[test]
    fn submerged_ball_is_destroyed_and_links_release() {
        let mut g = room();
        g.world.set_tile(4, 4, "oLava");
        spawn_chain(&mut g, Vec2::new(72.0, 73.0), false);
        tick(&mut g);
        assert!(
            g.world
                .entities
                .iter()
                .filter(|e| matches!(e.object.as_str(), "oBall" | "oChain"))
                .all(|e| !e.alive)
        );
    }
    #[test]
    fn water_does_not_apply_character_drag_to_iron_ball() {
        let mut g = room();
        g.world.set_tile(4, 4, "oWaterSwim");
        let mut e = Entity::new(1, "oBall", Vec2::new(72.0, 72.0));
        init(&mut e);
        e.vel.y = 2.0;
        item_physics(&g, &mut e);
        assert_eq!(e.pos.y, 74.0);
        assert_eq!(e.vel.y, 3.0);
        assert_eq!(e.value("gravity"), 0.6);
    }
    #[test]
    fn falling_ball_stuns_body_enemy_once_without_hurting_magma() {
        let mut g = room();
        let id = g.world.spawn("oCaveman", Vec2::new(64.0, 64.0));
        let mut b = Entity::new(100, "oBall", Vec2::new(72.0, 72.0));
        init(&mut b);
        b.vel.y = 4.0;
        impact(&mut g, &b);
        assert_eq!(g.world.entity(id).unwrap().health, 2);
        assert_eq!(g.world.entity(id).unwrap().state, 98);
        impact(&mut g, &b);
        assert_eq!(g.world.entity(id).unwrap().health, 2);
        let magma = g.world.spawn("oMagmaMan", Vec2::new(64.0, 64.0));
        let hp = g.world.entity(magma).unwrap().health;
        impact(&mut g, &b);
        assert_eq!(g.world.entity(magma).unwrap().health, hp);
    }
    #[test]
    fn transition_variant_tracks_dummy_and_fallback_endpoint() {
        let mut g = room();
        spawn_chain(&mut g, Vec2::new(200.0, 184.0), true);
        chains(&mut g);
        assert_eq!(
            g.world
                .entities
                .iter()
                .find(|e| e.object == "oChain2" && e.value("link_val") == 4.0)
                .unwrap()
                .pos,
            Vec2::new(280.0, 184.0)
        );
        let dummy = g.world.spawn("oPDummy", Vec2::new(300.0, 184.0));
        tick(&mut g);
        assert_eq!(
            g.world
                .entities
                .iter()
                .find(|e| e.object == "oBall2")
                .unwrap()
                .pos
                .x,
            g.world.entity(dummy).unwrap().pos.x - 24.0
        );
        assert_eq!(
            g.world
                .entities
                .iter()
                .find(|e| e.object == "oChain2" && e.value("link_val") == 4.0)
                .unwrap()
                .pos,
            g.world.entity(dummy).unwrap().pos
        );
    }
    #[test]
    fn room_entry_restores_curse_once() {
        let mut g = room();
        g.data.insert("kali_punish".into(), 2.0);
        g.level_tick = 1;
        tick(&mut g);
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| e.object == "oBall")
                .count(),
            1
        );
        g.level_tick = 2;
        g.world
            .entities
            .iter_mut()
            .find(|e| e.object == "oBall")
            .unwrap()
            .alive = false;
        tick(&mut g);
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| e.object == "oBall")
                .count(),
            1
        );
    }
}
