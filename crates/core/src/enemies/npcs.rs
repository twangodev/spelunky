//! NPC/ghost behavior from oGhost, oDamsel, oShopkeeper and oScarab events.
//! Shop purchase UI/dice/kissing-parlor economics belong to the economy system.
use super::{bounds, can_stomp, center, in_stars, move_body, steer_toward, touches_player};
use crate::*;

pub fn init(e: &mut Entity) {
    e.data.insert("animation_speed".into(), 0.5);
    match e.object.as_str() {
        "oGhost" => {
            e.health = 1;
            e.state = 1;
            e.sprite = "sGhostRight".into();
            e.data.insert("invincible".into(), 1.0);
        }
        "oDamsel" => {
            e.health = 4;
            e.timer = 200;
            e.sprite = "sDamselLeft".into();
            e.facing = -1;
        }
        "oShopkeeper" | "oShopkeeper2" => {
            e.health = 20;
            e.sprite = "sShopLeft".into();
            e.facing = -1;
            e.data.insert("has_gun".into(), 1.0);
            e.data.insert("stun_time".into(), 5.0);
            if e.object == "oShopkeeper2" {
                e.state = 2;
                e.data.insert("has_gun".into(), 0.0);
                e.data.insert("death_timer".into(), 200.0);
            }
        }
        "oScarab" => {
            e.sprite = "sScarab".into();
            e.timer = 20;
        }
        "oDeadFish" => {
            e.sprite = "sDeadFishRight".into();
            e.facing = 1;
        }
        "oBarrier" | "oBarrierEmitter" => {
            e.data.insert("invincible".into(), 1.0);
        }
        _ => {}
    }
}

pub fn tick(game: &mut Game, e: &mut Entity) {
    match e.object.as_str() {
        "oGhost" => ghost(game, e),
        "oDamsel" => damsel(game, e),
        "oShopkeeper" => shopkeeper(game, e),
        "oShopkeeper2" => {
            shopkeeper(game, e);
            if e.state == 99 {
                let timer = e.value("death_timer");
                if timer > 0.0 {
                    e.data.insert("death_timer".into(), timer - 1.0);
                } else {
                    for _ in 0..3 {
                        game.world
                            .spawn("oBlood", Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0));
                    }
                    e.alive = false;
                }
            }
        }
        "oScarab" => scarab(game, e),
        "oDeadFish" => dead_fish(game, e),
        "oBarrier" | "oBarrierEmitter" => barrier(game, e),
        _ => {}
    }
}

fn ghost(game: &mut Game, e: &mut Entity) {
    if e.state == 0 {
        e.data
            .insert("disappear".into(), e.value("disappear") + 0.2);
        if e.value("disappear") >= 8.0 {
            e.alive = false;
        }
        return;
    }
    steer_toward(e, game.player.pos, 1.0);
    e.pos.x += e.vel.x;
    e.pos.y += e.vel.y;
    e.sprite = if e.facing < 0 {
        "sGhostLeft"
    } else {
        "sGhostRight"
    }
    .into();
    let gems: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|x| {
            x.alive
                && matches!(
                    x.object.as_str(),
                    "oEmeraldBig" | "oSapphireBig" | "oRubyBig"
                )
                && x.bounds().intersects(bounds(e))
        })
        .map(|x| x.id)
        .collect();
    for id in gems {
        if let Some(gem) = game.world.entity_mut(id) {
            gem.object = "oDiamond".into();
            gem.sprite = "sDiamond".into();
        }
    }
    if game.player.invincible == 0
        && game.player.health > 0
        && game.player.bounds().intersects(bounds(e))
    {
        game.player.health = -99;
        game.player.invincible = 9999;
        game.player
            .held
            .take()
            .and_then(|id| game.world.entity_mut(id))
            .map(|held| held.held = false);
        for _ in 0..3 {
            game.world.spawn("oBone", game.player.pos);
        }
        game.world.spawn("oSkull", game.player.pos);
        game.sound("sndDie", game.player.pos);
        game.sound("sndGhost", e.pos);
        e.state = 0;
        e.sprite = "sGhostDisappear".into();
    }
}

fn damsel(game: &mut Game, e: &mut Entity) {
    if e.state == 4 {
        return;
    }
    let (_, grounded) = move_body(&game.world, e, 0.6);
    if e.value("invincible") == 0.0 && game.world.solid_at(e.pos.x, e.pos.y) {
        e.health = 0;
        e.alive = false;
        return;
    }
    if e.health > 0 && e.state != 2 && e.pos.distance(game.world.exit) < 12.0 {
        e.state = 4;
        e.vel = Vec2::default();
        e.pos = game.world.exit;
        e.sprite = "sDamselExit2".into();
        e.data.insert("inactive".into(), 1.0);
        game.flags.insert("damsel_rescued".into());
        *game.data.entry("damsels".into()).or_default() += 1.0;
        game.sound("sndSteps", e.pos);
        return;
    }
    if e.health <= 0 {
        e.state = 99;
        e.sprite = "sDamselDieL".into();
        return;
    }
    if e.vel.y > 2.0 && e.state != 2 {
        e.state = 2;
        e.timer = 120;
        game.sound("sndDamsel", e.pos);
    }
    match e.state {
        0 => {
            e.vel.x = 0.0;
            e.sprite = "sDamselLeft".into();
            if e.timer > 0 {
                e.timer -= 1;
            } else {
                e.state = 3;
                e.animation = 0.0;
                e.sprite = "sDamselYellL".into();
                game.sound("sndDamsel", e.pos);
            }
        }
        1 => {
            e.sprite = "sDamselRunL".into();
            if game
                .world
                .solid_at(e.pos.x + f64::from(e.facing) * 8.0, e.pos.y)
            {
                e.facing *= -1;
            }
            e.vel.x = f64::from(e.facing) * 1.5;
        }
        2 => {
            e.sprite = "sDamselStunL".into();
            if grounded {
                e.vel.x *= 0.7;
                if e.vel.x.abs() < 0.5 {
                    e.vel.x = 0.0;
                }
                if e.timer > 0 {
                    e.timer -= 1;
                } else {
                    e.state = 1;
                }
            }
        }
        3 => {
            if e.animation >= 8.0 {
                e.state = 0;
                e.timer = 200;
                e.animation = 0.0;
            }
        }
        _ => {}
    }
}

fn shopkeeper(game: &mut Game, e: &mut Entity) {
    let (wall, grounded) = move_body(&game.world, e, 0.6);
    if game.world.solid_at(
        e.pos.x + 8.0,
        e.pos.y + if e.state >= 98 { 12.0 } else { 8.0 },
    ) {
        e.health = 0;
        e.alive = false;
        return;
    }
    if e.health <= 0 {
        e.state = 99;
        if !in_stars(game) && e.value("death_gold") == 0.0 {
            e.data.insert("death_gold".into(), 1.0);
            for _ in 0..1 + game.rng.range(4) {
                let id = game
                    .world
                    .spawn("oGoldNugget", Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0));
                let vx = f64::from(1 + game.rng.range(3)) - f64::from(1 + game.rng.range(3));
                if let Some(gold) = game.world.entity_mut(id) {
                    gold.vel = Vec2::new(vx, -1.0);
                }
            }
            game.sound("sndCavemanDie", e.pos);
        }
    }
    let angry = e.value("angry") > 0.0
        || game.flags.contains("wanted")
        || game.flags.contains("shopkeepers_angry")
        || game.flags.contains("murderer")
        || game.flags.contains("thief")
        || game.data.get("thiefLevel").copied().unwrap_or(0.0) > 0.0
        || game.data.get("thief_level").copied().unwrap_or(0.0) > 0.0;
    if angry && e.state == 0 {
        e.state = 4;
        e.timer = 20;
    }
    if e.state >= 98 {
        if e.value("has_gun") > 0.0 {
            let id = game.world.spawn("oShotgun", e.pos);
            if let Some(gun) = game.world.entity_mut(id) {
                gun.vel = Vec2::new(f64::from(e.facing) * 5.0, -5.0);
            }
            e.data.insert("has_gun".into(), 0.0);
        }
        e.sprite = if e.health > 0 {
            "sShopStunL"
        } else {
            "sShopDieL"
        }
        .into();
        e.vel.x -= e.vel.x.signum() * 0.1;
        if e.vel.x.abs() < 0.5 {
            e.vel.x = 0.0;
        }
        if grounded {
            if e.timer > 0 {
                e.timer -= 1;
            } else if e.health > 0 {
                e.state = 2;
            }
        }
        if e.health <= 0 {
            e.state = if e.vel.x == 0.0 && e.vel.y == 0.0 {
                99
            } else {
                98
            };
        }
        return;
    }
    if e.state == 0 {
        e.vel.x = 0.0;
        e.facing = if game.player.pos.x < center(e).x {
            -1
        } else {
            1
        };
        e.sprite = "sShopLeft".into();
        if e.value("welcomed") == 0.0 && e.pos.distance(game.player.pos) < 80.0 {
            game.events
                .push(GameEvent::Message("WELCOME TO MY SHOP!".into()));
            e.data.insert("welcomed".into(), 1.0);
        }
        if let Some(held) = game.player.held.and_then(|id| game.world.entity(id)) {
            if held.value("cost") > 0.0 {
                e.state = 5;
            }
        }
    } else if e.state == 5 {
        let d = center(e).distance(game.player.pos);
        e.facing = if game.player.pos.x < center(e).x {
            -1
        } else {
            1
        };
        e.vel.x = if d < 12.0 {
            0.0
        } else {
            f64::from(e.facing) * (d / 16.0 * 1.5).min(3.0)
        };
        if !game
            .player
            .held
            .and_then(|id| game.world.entity(id))
            .is_some_and(|x| x.value("cost") > 0.0)
        {
            e.state = 0;
        }
    } else if e.state == 4 {
        e.vel.x = 0.0;
        if grounded && e.timer > 0 {
            e.timer -= 1;
        }
        if e.timer == 0 {
            e.state = 1;
            e.facing = if game.rng.range(2) == 0 { -1 } else { 1 };
        }
    } else if e.state == 1 {
        if wall {
            e.facing *= -1;
        }
        e.vel.x = f64::from(e.facing) * 1.5;
        if center(e).distance(game.player.pos) < 64.0 {
            e.state = 2;
        }
    } else if e.state == 3 {
        if e.animation >= 8.0 {
            e.state = 2;
            e.animation = 0.0;
        }
    } else if e.state == 2 {
        if e.value("turn_timer") > 0.0 {
            e.data
                .insert("turn_timer".into(), e.value("turn_timer") - 1.0);
        } else if grounded
            && (game.player.pos.y - center(e).y).abs() < 8.0
            && center(e).distance(game.player.pos) > 16.0
        {
            e.facing = if game.player.pos.x < center(e).x {
                -1
            } else {
                1
            };
            e.data.insert("turn_timer".into(), 20.0);
        }
        if wall {
            e.facing *= -1;
        }
        e.vel.x = f64::from(e.facing) * 3.0;
        let cooldown = e.value("firing");
        if cooldown > 0.0 {
            e.data.insert("firing".into(), cooldown - 1.0);
        } else if e.value("has_gun") > 0.0
            && (game.player.pos.y - center(e).y).abs() < 32.0
            && center(e).distance(game.player.pos) < 96.0
            && (game.player.pos.x - center(e).x) * f64::from(e.facing) > 0.0
        {
            for _ in 0..6 {
                let speed = f64::from(6 + game.rng.range(3));
                let spread = game.rng.unit() - game.rng.unit();
                let id = game.world.spawn(
                    "oBullet",
                    Vec2::new(
                        e.pos.x + if e.facing < 0 { 4.0 } else { 12.0 },
                        e.pos.y + 8.0,
                    ),
                );
                if let Some(b) = game.world.entity_mut(id) {
                    b.vel = Vec2::new(f64::from(e.facing) * (speed + 3.0), spread);
                    b.data.insert("safe".into(), 1.0);
                    b.data.insert("owner_player".into(), 0.0);
                    b.data.insert("damage".into(), 4.0);
                }
            }
            e.vel.y -= 1.0;
            e.vel.x -= f64::from(e.facing) * 3.0;
            e.data.insert("firing".into(), 30.0);
            game.sound("sndShotgun", e.pos);
        }
        if grounded
            && (wall
                || !game
                    .world
                    .solid_at(e.pos.x + f64::from(e.facing) * 24.0, e.pos.y + 16.0))
            && game.player.pos.y <= e.pos.y + 8.0
        {
            e.vel.y = -f64::from(7 + game.rng.range(2));
        }
        if touches_player(game, e)
            && (game.player.pos.x - (e.pos.x + 8.0)).abs() <= 8.0
            && !can_stomp(game, e)
            && game.player.invincible == 0
            && game.player.stun == 0
        {
            if !game.world.solid_at(e.pos.x + 8.0, e.pos.y - 4.0) {
                e.state = 3;
                e.animation = 0.0;
                e.facing = if game.player.pos.x > e.pos.x + 8.0 {
                    1
                } else {
                    -1
                };
                e.vel.x = 0.0;
                game.player.pos =
                    Vec2::new(e.pos.x + if e.facing > 0 { 0.0 } else { 16.0 }, e.pos.y);
                game.player.previous_pos = game.player.pos;
                game.player.vel = Vec2::new(f64::from(e.facing) * 6.0, -6.0);
                game.player.stun = 30;
                game.player.state = PlayerState::Stunned;
                game.player.data.insert("bounced".into(), 0.0);
                game.player.data.insert("wall_hurt".into(), 1.0);
                if let Some(id) = game.player.held.take() {
                    if let Some(held) = game.world.entity_mut(id) {
                        held.held = false;
                        held.vel = game.player.vel;
                    }
                }
            } else {
                game.player.health -= 1;
                game.player.invincible = 30;
                game.player.vel.x = if game.player.pos.x < e.pos.x {
                    -6.0
                } else {
                    6.0
                };
                game.events.push(GameEvent::Damage { amount: 1 });
                game.sound("sndHurt", game.player.pos);
            }
        }
        e.sprite = if e.state == 3 {
            "sShopThrowL"
        } else {
            "sShopRunLeft"
        }
        .into();
    }
}

fn scarab(game: &mut Game, e: &mut Entity) {
    let old = e.vel;
    let (wall, ground) = move_body(&game.world, e, 0.0);
    if game.world.solid_at(e.pos.x + 8.0, e.pos.y + 8.0) {
        e.health = 0;
        e.alive = false;
        return;
    }
    if wall {
        e.vel.x = -old.x;
    }
    if ground && old.y > 0.0 {
        e.vel.y = -old.y;
    }
    e.vel.x = (e.vel.x.abs() - 0.5).max(0.0) * e.vel.x.signum();
    e.vel.y = (e.vel.y.abs() - 0.5).max(0.0) * e.vel.y.signum();
    if e.vel.x.abs() < 1.0 {
        e.vel.x = 0.0;
    }
    if e.vel.y.abs() < 1.0 {
        e.vel.y = 0.0;
    }
    if e.vel == Vec2::default() {
        if e.timer > 0 {
            e.timer -= 1;
        } else {
            if e.pos.distance(game.player.pos) < 64.0 {
                steer_toward(e, game.player.pos, -4.0);
            } else {
                let angle = game.rng.unit() * std::f64::consts::TAU;
                e.vel = Vec2::new(angle.cos() * 4.0, angle.sin() * 4.0);
            }
            e.timer = 10 + game.rng.range(21);
        }
    }
    if game.player.bounds().intersects(e.bounds()) {
        let value = match game.world.area {
            Area::Mines => 4000,
            Area::Jungle => 8000,
            _ => 12000,
        };
        game.player.money += value;
        game.events.push(GameEvent::Collected {
            object: "oScarab".into(),
            value,
        });
        game.sound("sndCoin", e.pos);
        e.alive = false;
    }
}

fn water(game: &Game, pos: Vec2) -> bool {
    game.world
        .tile((pos.x / 16.0).floor() as i32, (pos.y / 16.0).floor() as i32)
        .is_some_and(|t| matches!(t.object.as_str(), "oWater" | "oWaterSwim"))
        || game.world.entities.iter().any(|e| {
            e.alive
                && matches!(e.object.as_str(), "oWater" | "oWaterSwim")
                && e.bounds().contains(pos)
        })
}

fn dead_fish(game: &mut Game, e: &mut Entity) {
    if e.health <= 0 {
        for _ in 0..3 {
            game.world
                .spawn("oBone", Vec2::new(e.pos.x + 4.0, e.pos.y + 4.0));
        }
        e.alive = false;
        return;
    }
    if !water(game, Vec2::new(e.pos.x + 4.0, e.pos.y + 4.0)) {
        game.world.spawn("oFishBone", e.pos);
        e.alive = false;
        return;
    }
    let swimming = water(game, game.player.pos) && game.player.health > 0;
    let dist = e.pos.distance(game.player.pos);
    if e.state == 0 {
        e.vel = Vec2::new(f64::from(e.facing), 0.0);
        let probe = Vec2::new(e.pos.x + if e.facing > 0 { 10.0 } else { -2.0 }, e.pos.y);
        if water(game, probe) && !game.world.solid_at(probe.x, probe.y) {
            move_body(&game.world, e, 0.0);
        } else {
            e.facing *= -1;
        }
        if dist < 90.0 && swimming {
            e.state = 1;
        }
    } else if e.state == 2 {
        if e.timer > 0 {
            e.timer -= 1;
        } else {
            e.state = 0;
            e.facing = if game.rng.range(2) == 0 { -1 } else { 1 };
        }
    } else if dist < 90.0 && swimming {
        steer_toward(e, game.player.pos, 1.0);
        let next = Vec2::new(e.pos.x + e.vel.x, e.pos.y + e.vel.y);
        if water(game, next) {
            move_body(&game.world, e, 0.0);
        }
    } else {
        e.state = 2;
        e.timer = 20 + game.rng.range(21);
    }
    e.sprite = if e.facing < 0 {
        "sDeadFishLeft"
    } else {
        "sDeadFishRight"
    }
    .into();
}

fn barrier(game: &mut Game, e: &mut Entity) {
    if e.object == "oBarrierEmitter" {
        if e.value("barrier_created") == 0.0 {
            if !game
                .world
                .entities
                .iter()
                .any(|x| x.alive && x.object == "oBarrier")
            {
                game.world
                    .spawn("oBarrier", Vec2::new(e.pos.x, e.pos.y + 16.0));
            }
            e.data.insert("barrier_created".into(), 1.0);
        }
        let struck = game.world.entities.iter().any(|x| {
            x.alive
                && x.bounds().intersects(e.bounds())
                && (x.object == "oBullet"
                    || crate::catalog::is_a(&x.object, "oItem")
                        && (x.vel.x.abs() > 2.0 || x.vel.y.abs() > 2.0))
        });
        if struck || !game.world.solid_at(e.pos.x, e.pos.y - 16.0) {
            e.alive = false;
            for b in &mut game.world.entities {
                if b.object == "oBarrier" {
                    b.alive = false;
                }
            }
            game.sound("sndSmallExplode", e.pos);
        }
        return;
    }
    if !game
        .world
        .entities
        .iter()
        .any(|x| x.alive && x.object == "oBarrierEmitter")
    {
        e.alive = false;
        return;
    }
    if e.bounds().intersects(game.player.bounds()) && game.player.invincible == 0 {
        game.damage_player(1);
        game.player.invincible = 30;
        game.player.vel = Vec2::new(
            if game.player.pos.x < e.pos.x {
                -6.0
            } else {
                6.0
            },
            -2.0,
        );
    }
    let mut release = false;
    for item in &mut game.world.entities {
        if !item.alive || !item.bounds().intersects(e.bounds()) {
            continue;
        }
        if matches!(item.object.as_str(), "oBullet" | "oWebBall") {
            item.alive = false;
        } else if crate::catalog::is_a(&item.object, "oItem") {
            item.vel = Vec2::new(-f64::from(4 + game.rng.range(3)), -2.0);
            if item.held {
                release = true;
                item.held = false;
            }
            if matches!(item.object.as_str(), "oBomb" | "oFireFrogBomb") {
                item.timer = 4 + game.rng.range(5);
                item.data.insert("armed".into(), 1.0);
            }
        }
    }
    if release {
        game.player.held = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ghost_moves_one_pixel_through_walls() {
        let mut g = Game::shell(1, RulesPreset::Classic);
        g.player.pos = Vec2::new(100.0, 48.0);
        let mut e = Entity::new(1, "oGhost", Vec2::new(40.0, 40.0));
        init(&mut e);
        g.world.set_tile(2, 2, "oSolid");
        let before = e.pos;
        ghost(&mut g, &mut e);
        assert!((e.pos.distance(before) - 1.0).abs() < 1e-9);
        assert!(e.pos.x > before.x);
    }
    #[test]
    fn ghost_contact_is_lethal() {
        let mut g = Game::shell(1, RulesPreset::Classic);
        let mut e = Entity::new(1, "oGhost", g.player.pos);
        init(&mut e);
        ghost(&mut g, &mut e);
        assert_eq!(g.player.health, -99);
        assert_eq!(e.state, 0);
    }
    #[test]
    fn damsel_exit_sets_rescue_without_immediate_heal() {
        let mut g = Game::shell(1, RulesPreset::Classic);
        g.world.exit = Vec2::new(40.0, 40.0);
        let mut e = Entity::new(1, "oDamsel", g.world.exit);
        init(&mut e);
        damsel(&mut g, &mut e);
        assert!(g.flags.contains("damsel_rescued"));
        assert_eq!(g.player.health, 4);
        assert_eq!(e.state, 4);
    }
    #[test]
    fn dead_fish_outside_water_becomes_portable_bone() {
        let mut g = Game::shell(1, RulesPreset::Classic);
        let mut e = Entity::new(99, "oDeadFish", Vec2::new(80.0, 80.0));
        init(&mut e);
        dead_fish(&mut g, &mut e);
        assert!(!e.alive);
        assert!(g.world.entities.iter().any(|e| e.object == "oFishBone"));
    }
    #[test]
    fn destroying_emitter_releases_barrier() {
        let mut g = Game::shell(1, RulesPreset::Classic);
        g.world.set_tile(5, 4, "oSolid");
        let mut e = Entity::new(99, "oBarrierEmitter", Vec2::new(80.0, 80.0));
        init(&mut e);
        barrier(&mut g, &mut e);
        assert!(
            g.world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oBarrier")
        );
        g.world.spawn("oBullet", Vec2::new(88.0, 88.0));
        barrier(&mut g, &mut e);
        assert!(!e.alive);
        assert!(
            !g.world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oBarrier")
        );
    }
    #[test]
    fn stars_corpse_expires_after_source_death_timer() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.flags.insert("hub_stars".into());
        game.player.pos = Vec2::new(200.0, 200.0);
        game.world.set_tile(4, 5, "oSolid");
        let mut e = Entity::new(99, "oShopkeeper2", Vec2::new(64.0, 64.0));
        super::super::init_entity(&mut e);
        e.health = 0;
        for _ in 0..200 {
            tick(&mut game, &mut e);
            assert!(e.alive);
        }
        assert_eq!(e.value("death_timer"), 0.0);
        tick(&mut game, &mut e);
        assert!(!e.alive);
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|e| e.object == "oBlood")
                .count(),
            3
        );
    }
}
