//! NPC/ghost behavior from oGhost, oDamsel, oShopkeeper and oScarab events.
//! Shop purchase UI/dice/kissing-parlor economics belong to the economy system.
use super::{can_stomp, center, in_stars, move_body, steer_toward, touches_player};
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
            e.data.entry("canPickUp".into()).or_insert(1.0);
        }
        "oShopkeeper" | "oShopkeeper2" => {
            e.health = 20;
            e.sprite = "sShopLeft".into();
            e.facing = -1;
            e.data.insert("has_gun".into(), 1.0);
            e.data.insert("stun_time".into(), 5.0);
            e.data
                .entry("shop_room_x".into())
                .or_insert((((e.pos.x - 16.0).max(0.0) / 160.0).floor()).min(3.0));
            e.data
                .entry("shop_room_y".into())
                .or_insert((((e.pos.y - 16.0).max(0.0) / 128.0).floor()).min(4.0));
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

pub(super) fn animation_end(e: &mut Entity, sprite: &str) {
    if e.object == "oGhost" && sprite == "sGhostDisappear" {
        e.alive = false;
    } else if e.object == "oDamsel" && e.state == 3 {
        e.state = 0;
        e.timer = 200;
    } else if e.object == "oDamsel" && e.state == 6 {
        e.state = 5;
    } else if matches!(e.object.as_str(), "oShopkeeper" | "oShopkeeper2") && sprite == "sShopThrowL"
    {
        e.state = 2;
        e.sprite = "sShopLeft".into();
    }
}

pub(super) fn tick_independent_events(game: &mut Game, e: &mut Entity) {
    if !e.alive {
        return;
    }
    if e.object == "oScarab" {
        if game.player.sprite_bounds().intersects(e.sprite_bounds()) {
            // Source Create assigns 4000/8000/12000 in Mines/Jungle/Temple.
            // Retain caller overrides and the existing unsupported-area
            // fallback; Ice/Boss have no explicit value assignment in source.
            let value = e
                .data
                .get("value")
                .copied()
                .unwrap_or(match game.world.area {
                    Area::Mines => 4000.0,
                    Area::Jungle => 8000.0,
                    _ => 12000.0,
                })
                .max(0.0) as u32;
            *game.data.entry("collect".into()).or_default() += f64::from(value);
            let counter = game.data.entry("collectCounter".into()).or_default();
            *counter = (*counter + 20.0).min(100.0);
            *game.data.entry("scarabs".into()).or_default() += 1.0;
            game.events.push(GameEvent::Collected {
                object: "oScarab".into(),
                value,
            });
            game.sound("sndCoin", e.pos);
            e.alive = false;
            destroyed(game, e);
        }
    } else if e.object == "oBarrierEmitter" {
        if game.world.entities.iter().any(|item| {
            item.alive
                && item.sprite_bounds().intersects(e.sprite_bounds())
                && (item.object == "oBullet"
                    || crate::catalog::is_a(&item.object, "oItem")
                        && (item.vel.x.abs() > 2.0 || item.vel.y.abs() > 2.0))
        }) {
            destroy_emitter(game, e);
        }
    } else if matches!(e.object.as_str(), "oShopkeeper" | "oShopkeeper2")
        && e.health > 0
        && e.state == 2
        && e.value("has_gun") == 0.0
    {
        if let Some(gun) = game.world.entities.iter_mut().find(|gun| {
            gun.alive
                && crate::catalog::is_a(&gun.object, "oShotgun")
                && gun.sprite_bounds().intersects(e.sprite_bounds())
        }) {
            if gun.held {
                game.player.held = game.player.held.filter(|id| *id != gun.id);
                gun.held = false;
            }
            gun.alive = false;
            e.data.insert("has_gun".into(), 1.0);
        }
    }
}

fn destroy_emitter(game: &mut Game, e: &mut Entity) {
    e.alive = false;
    destroyed(game, e);
}

/// Source Destroy events also run after collection, outside-room removal, and
/// destruction by another system. Keep the callback idempotent across phases.
pub(super) fn destroyed(game: &mut Game, e: &mut Entity) {
    if e.alive || e.value("npc_destroyed") != 0.0 {
        return;
    }
    match e.object.as_str() {
        "oBarrierEmitter" => {
            e.data.insert("npc_destroyed".into(), 1.0);
            for _ in 0..6 {
                let pos = Vec2::new(
                    e.pos.x + 2.0 + f64::from(game.rng.range(15)),
                    e.pos.y + 2.0 + f64::from(game.rng.range(15)),
                );
                let id = game.world.spawn("oFlareSpark", pos);
                let vy = f64::from(1 + game.rng.range(3));
                game.world.entity_mut(id).unwrap().vel.y = vy;
            }
            for barrier in &mut game.world.entities {
                if barrier.object == "oBarrier" {
                    barrier.alive = false;
                }
            }
            let shake = game.data.entry("shake".into()).or_default();
            *shake = (*shake).max(10.0);
            game.sound("sndSmallExplode", e.pos);
        }
        "oScarab" => {
            e.data.insert("npc_destroyed".into(), 1.0);
            // Destroy.xml spells this origin `other.x/y`; its binding during
            // nested GM8 Collision destruction remains unverified. Preserve
            // the actor origin here rather than assume the collecting player.
            for _ in 0..3 {
                let pos = Vec2::new(
                    e.pos.x + 6.0 + f64::from(game.rng.range(5)),
                    e.pos.y + 6.0 + f64::from(game.rng.range(5)),
                );
                game.world.spawn("oFlareSpark", pos);
            }
        }
        _ => {}
    }
}

pub(super) fn finish_destroy_events(game: &mut Game) {
    let ids: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| {
            !e.alive
                && matches!(e.object.as_str(), "oBarrierEmitter" | "oScarab")
                && e.value("npc_destroyed") == 0.0
        })
        .map(|e| e.id)
        .collect();
    for id in ids {
        let index = game.world.entities.iter().position(|e| e.id == id).unwrap();
        let mut e = game.world.entities.remove(index);
        destroyed(game, &mut e);
        game.world.entities.insert(index, e);
    }
}

fn ghost(game: &mut Game, e: &mut Entity) {
    if e.state == 0 {
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
                && x.sprite_bounds().intersects(e.sprite_bounds())
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
        && game.player.sprite_bounds().intersects(e.sprite_bounds())
    {
        game.player.health = -99;
        game.player.invincible = 9999;
        game.player
            .held
            .take()
            .and_then(|id| game.world.entity_mut(id))
            .map(|held| held.held = false);
        for _ in 0..3 {
            crate::world_events::spawn_detritus(game, "oBone", game.player.pos);
        }
        let skull = game.world.spawn(
            "oSkull",
            Vec2::new(game.player.pos.x, game.player.pos.y - 2.0),
        );
        let vy = -f64::from(1 + game.rng.range(3));
        let vx = f64::from(game.rng.range(4)) - f64::from(game.rng.range(4));
        game.world.entity_mut(skull).unwrap().vel = Vec2::new(vx, vy);
        game.sound("sndDie", game.player.pos);
        game.sound("sndGhost", e.pos);
        e.state = 0;
        e.sprite = "sGhostDisappear".into();
        e.animation = 0.0;
        e.data.insert("animation_speed".into(), 0.2);
    }
}

fn damsel(game: &mut Game, e: &mut Entity) {
    e.data
        .insert("depth".into(), if e.held { 1.0 } else { 99.0 });
    if e.state == 4 {
        e.data.insert("depth".into(), 1000.0);
        e.data.insert("invincible".into(), 1.0);
        return;
    }
    let swimming = super::inherited::point(&game.world, e.pos, "oWaterSwim");
    e.data.insert("swimming".into(), f64::from(swimming));
    // Only the inherited oItem movement has a viewport guard. Damsel's own
    // hazards, counters and state events continue for every engine-active NPC.
    let inherited_active = e.pos.x > game.camera.x - 16.0
        && e.pos.x < game.camera.x + VIEW_WIDTH + 16.0
        && e.pos.y > game.camera.y - 16.0
        && e.pos.y < game.camera.y + VIEW_HEIGHT + 16.0;
    let (_, grounded) = if e.held || !inherited_active {
        (false, false)
    } else {
        move_body(&game.world, e, if swimming { 0.2 } else { 0.6 })
    };
    if e.value("invincible") == 0.0 && !e.held && game.world.solid_at(e.pos.x, e.pos.y) {
        e.health = 0;
        e.alive = false;
        return;
    }
    if e.value("invincible") == 0.0 {
        if super::inherited::point(&game.world, Vec2::new(e.pos.x, e.pos.y + 6.0), "oLava") {
            e.health = 0;
            e.state = 99;
            e.vel = Vec2::new(0.0, 0.1);
            e.data.insert("burning".into(), 1.0);
            e.data.insert("depth".into(), 999.0);
        }
        if e.vel.y > 2.0
            && super::inherited::point(&game.world, Vec2::new(e.pos.x, e.pos.y + 6.0), "oSpikes")
        {
            e.health = 0;
            e.state = 99;
            e.vel = Vec2::new(0.0, 0.2);
        }
        let spear_area = Rect::new(e.pos.x - 3.0, e.pos.y - 3.0, 7.0, 7.0);
        let spear = game
            .world
            .entities
            .iter()
            .filter(|s| {
                s.alive
                    && crate::catalog::is_a(&s.object, "oSpearsLeft")
                    && s.sprite_bounds().intersects(spear_area)
            })
            .min_by(|a, b| a.pos.distance(e.pos).total_cmp(&b.pos.distance(e.pos)))
            .map(|s| (s.pos, s.animation));
        if let Some((pos, _)) =
            spear.filter(|(_, frame)| *frame >= 19.0 && *frame < 28.0 && (e.state != 2 || grounded))
        {
            e.health -= 2;
            e.state = 2;
            e.timer = 120;
            e.vel = Vec2::new(if pos.x < e.pos.x { 4.0 } else { -4.0 }, -6.0);
            if e.held {
                e.held = false;
                game.player.held = game.player.held.filter(|id| *id != e.id);
            }
            game.sound("sndHit", e.pos);
        }
    }
    let exit = game
        .world
        .entities
        .iter()
        .find(|door| {
            door.alive
                && crate::catalog::is_a(&door.object, "oExit")
                && door.sprite_bounds().contains(e.pos)
        })
        .map(|door| door.pos);
    if let Some(exit) = exit.filter(|_| e.health > 0 && e.state != 2 && !e.held) {
        e.state = 4;
        e.vel = Vec2::default();
        e.pos = Vec2::new(exit.x + 8.0, exit.y + 8.0);
        e.previous_pos = e.pos;
        e.sprite = if game.flags.contains("character_damsel") {
            "sPExit"
        } else {
            "sDamselExit2"
        }
        .into();
        e.animation = 0.0;
        e.data.insert("inactive".into(), 1.0);
        e.data.insert("depth".into(), 1000.0);
        e.data.insert("invincible".into(), 1.0);
        game.flags.insert("damsel_rescued".into());
        *game.data.entry("pending_damsel_kisses".into()).or_default() += 1.0;
        *game.data.entry("damsels".into()).or_default() += 1.0;
        *game.data.entry("xdamsels".into()).or_default() += 1.0;
        game.sound("sndSteps", e.pos);
        return;
    }
    if e.health <= 0 {
        if e.value("cost") > 0.0 || e.value("for_sale") > 0.0 {
            game.flags.insert("shopkeepers_angry".into());
        }
        e.state = 99;
        e.sprite = "sDamselDieL".into();
        return;
    }
    if e.vel.y > 2.0 && e.state != 2 {
        e.state = 2;
        e.timer = 120;
        game.sound("sndDamsel", e.pos);
    }
    if e.held {
        e.facing = game.player.facing;
        return;
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
            let b = e.bounds();
            let supported =
                super::solid_mask(&game.world, Rect::new(b.x, b.y + b.h + 1.0, b.w, 1.0), e.id)
                    || game.world.entities.iter().any(|web| {
                        web.alive
                            && web.object == "oWeb"
                            && web.sprite_bounds().intersects(e.sprite_bounds())
                    });
            if grounded || supported {
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
            if e.animation == 4.0 {
                game.world
                    .spawn("oYellHelp", Vec2::new(e.pos.x, e.pos.y - 16.0));
            }
        }
        5 => {
            e.facing = if game.player.pos.x < e.pos.x { -1 } else { 1 };
            e.vel.x = 0.0;
            e.sprite = if game.flags.contains("character_damsel") {
                "sStandLeft"
            } else {
                "sDamselLeft"
            }
            .into();
        }
        6 => {
            if e.animation == 7.0 {
                game.world.spawn(
                    "oHeart",
                    Vec2::new(
                        e.pos.x + if e.facing < 0 { -8.0 } else { 8.0 },
                        e.pos.y - 8.0,
                    ),
                );
                game.sound("sndKiss", e.pos);
            }
        }
        _ => {}
    }
}

fn shopkeeper(game: &mut Game, e: &mut Entity) {
    let impact_y = (e.vel.y + super::inherited::gravity(e, 0.6)).min(10.0);
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
    let player_bounds = game.player.sprite_bounds();
    let own_bounds = e.sprite_bounds();
    let distance = ((player_bounds.x - own_bounds.x - own_bounds.w + 1.0)
        .max(own_bounds.x - player_bounds.x - player_bounds.w + 1.0)
        .max(0.0))
    .hypot(
        (player_bounds.y - own_bounds.y - own_bounds.h + 1.0)
            .max(own_bounds.y - player_bounds.y - player_bounds.h + 1.0)
            .max(0.0),
    );
    if matches!(e.state, 1 | 4)
        && ((super::player_alive(game)
            && distance < 64.0
            && game.player.pos.y - (e.pos.y + 8.0) < 16.0)
            || (game.player.pos.x - (e.pos.x + 8.0)).abs() < 4.0)
    {
        e.state = 2;
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
        super::settle_body(&game.world, e, impact_y);
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
        if angry {
            e.state = 4;
            return;
        }
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
        e.data.insert("animation_speed".into(), 0.5);
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
        if grounded && e.timer > 0 {
            e.timer -= 1;
        }
        if e.timer == 0 {
            e.state = 1;
            e.facing = if game.rng.range(2) == 0 { -1 } else { 1 };
        }
    } else if e.state == 1 {
        e.data.insert("animation_speed".into(), 0.5);
        if wall {
            e.facing *= -1;
        }
        e.vel.x = f64::from(e.facing) * 1.5;
    } else if e.state == 3 {
    } else if e.state == 2 {
        e.data.insert("animation_speed".into(), 1.0);
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
            && {
                let b = e.bounds();
                !super::solid_mask(&game.world, Rect::new(b.x, b.y - 4.0, b.w, 1.0), e.id)
            }
        {
            e.vel.y = -f64::from(7 + game.rng.range(2));
        }
        if touches_player(game, e)
            && (game.player.pos.x - (e.pos.x + 8.0)).abs() <= 8.0
            && !can_stomp(game, e)
            && game.player.invincible == 0
            && game.player.stun == 0
            && game.player.state != PlayerState::Stunned
            && game.player.state != PlayerState::Dead
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
                game.player.state = PlayerState::Stunned;
                game.player.data.insert("bounced".into(), 0.0);
                game.player.data.insert("wall_hurt".into(), 1.0);
                super::drop_from_throw(game);
            } else {
                game.player.health -= 1;
                game.player.invincible = 30;
                game.player.data.insert("blink".into(), 30.0);
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
        if !super::player_alive(game) {
            e.state = 1;
        }
    }
}

fn scarab(game: &mut Game, e: &mut Entity) {
    let old = e.vel;
    let (wall, ground) = move_body(&game.world, e, 0.0);
    if game.world.solid_at(e.pos.x + 8.0, e.pos.y + 8.0) {
        e.health = 0;
    }
    if e.health < 1 {
        // The Step death burst is separate from its three-spark Destroy event.
        for _ in 0..3 {
            let pos = Vec2::new(
                e.pos.x + 2.0 + f64::from(game.rng.range(15)),
                e.pos.y + 2.0 + f64::from(game.rng.range(15)),
            );
            let id = game.world.spawn("oFlareSpark", pos);
            let vy = f64::from(1 + game.rng.range(3));
            game.world.entity_mut(id).unwrap().vel.y = vy;
        }
        e.alive = false;
        destroyed(game, e);
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
            crate::world_events::spawn_detritus(
                game,
                "oBone",
                Vec2::new(e.pos.x + 4.0, e.pos.y + 4.0),
            );
        }
        e.alive = false;
        return;
    }
    if !water(game, Vec2::new(e.pos.x + 4.0, e.pos.y + 4.0)) {
        game.world.spawn("oFishBone", e.pos);
        e.alive = false;
        return;
    }
    let swimming = water(game, game.player.pos) && super::player_alive(game);
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
        if !game.world.solid_at(e.pos.x, e.pos.y - 16.0) {
            destroy_emitter(game, e);
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
    if e.sprite_bounds().intersects(game.player.sprite_bounds()) && game.player.invincible == 0 {
        game.player.health -= 1;
        game.events.push(GameEvent::Damage { amount: 1 });
        game.sound("sndHurt", game.player.pos);
        game.player.invincible = 30;
        game.player.data.insert("blink".into(), 30.0);
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
        if !item.alive || !item.sprite_bounds().intersects(e.sprite_bounds()) {
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
    fn actual_room() -> Game {
        let mut g = Game::shell(123, RulesPreset::Classic);
        g.world = World::new(30, 20, Area::Mines, 1);
        g.player.pos = Vec2::new(240.0, 128.0);
        g.player.previous_pos = g.player.pos;
        g
    }
    #[test]
    fn actual_step_barrier_hit_does_not_stun() {
        let mut g = actual_room();
        g.player.pos = Vec2::new(104.0, 104.0);
        g.world.set_tile(6, 4, "oBlock");
        let emitter = g.world.spawn("oBarrierEmitter", Vec2::new(96.0, 80.0));
        g.world
            .entity_mut(emitter)
            .unwrap()
            .data
            .insert("barrier_created".into(), 1.0);
        g.world.spawn("oBarrier", Vec2::new(96.0, 96.0));
        g.step(&InputFrame::default());
        assert_eq!(g.player.health, 3);
        assert_eq!(g.player.stun, 0);
        assert_eq!(g.player.invincible, 30);
        assert_ne!(g.player.state, PlayerState::Stunned);
    }
    #[test]
    fn actual_step_ghost_visible_edge_hits_player() {
        let mut g = actual_room();
        g.player.pos = Vec2::new(101.0, 96.0);
        g.world.spawn("oGhost", Vec2::new(80.0, 80.0));
        g.step(&InputFrame::default());
        assert_eq!(g.state, RunState::Dead);
    }
    fn source_bone_create(rng: &mut Rng) -> (Vec2, f64) {
        // oBone/Create.xml: xVel=random(4)-random(4),
        // yVel=-1-random(2), grav=rand(1,6)*.1.
        let velocity = Vec2::new(rng.unit() * 4.0 - rng.unit() * 4.0, -1.0 - rng.unit() * 2.0);
        (velocity, f64::from(1 + rng.range(6)) * 0.1)
    }
    #[test]
    fn actual_step_ghost_bone_creates_precede_skull_launch_draws() {
        let mut g = actual_room();
        for x in 0..30 {
            g.world.set_tile(x, 9, "oBlock");
        }
        g.player.pos = Vec2::new(104.0, 136.0);
        g.world.spawn("oGhost", Vec2::new(96.0, 120.0));
        let mut source_rng = g.rng.clone();
        let bones = (0..3)
            .map(|_| source_bone_create(&mut source_rng))
            .collect::<Vec<_>>();
        let skull_vy = -f64::from(1 + source_rng.range(3));
        let skull_vx = f64::from(source_rng.range(4)) - f64::from(source_rng.range(4));
        g.step(&InputFrame::default());
        assert_eq!(g.state, RunState::Dead);
        let actual_bones = g
            .world
            .entities
            .iter()
            .filter(|e| e.object == "oBone")
            .collect::<Vec<_>>();
        assert_eq!(actual_bones.len(), 3);
        for (bone, (velocity, gravity)) in actual_bones.into_iter().zip(bones) {
            assert_eq!(bone.value("world_effect_initialized"), 1.0);
            assert_eq!(bone.vel.x, velocity.x);
            assert!((bone.vel.y - (velocity.y + gravity)).abs() < 1e-9);
            assert_eq!(bone.value("gravity"), gravity);
        }
        let skull = g
            .world
            .entities
            .iter()
            .find(|e| e.object == "oSkull")
            .unwrap();
        assert_eq!(skull.pos, Vec2::new(104.0, 134.0));
        assert_eq!(skull.vel, Vec2::new(skull_vx, skull_vy));
        assert_eq!(g.rng.next_u64(), source_rng.next_u64());
    }
    #[test]
    fn actual_step_dead_fish_bone_create_matches_source_rng_order() {
        let mut g = actual_room();
        let id = g.world.spawn("oDeadFish", Vec2::new(96.0, 96.0));
        g.world.entity_mut(id).unwrap().health = 0;
        let mut source_rng = g.rng.clone();
        let bones = (0..3)
            .map(|_| source_bone_create(&mut source_rng))
            .collect::<Vec<_>>();
        g.step(&InputFrame::default());
        assert!(g.world.entity(id).is_none());
        let actual_bones = g
            .world
            .entities
            .iter()
            .filter(|e| e.object == "oBone")
            .collect::<Vec<_>>();
        assert_eq!(actual_bones.len(), 3);
        for (bone, (velocity, gravity)) in actual_bones.into_iter().zip(bones) {
            assert_eq!(bone.value("world_effect_initialized"), 1.0);
            assert_eq!(bone.vel.x, velocity.x);
            assert!((bone.vel.y - (velocity.y + gravity)).abs() < 1e-9);
            assert_eq!(bone.value("gravity"), gravity);
        }
        assert_eq!(g.rng.next_u64(), source_rng.next_u64());
    }
    #[test]
    fn actual_step_ghost_disappears_after_full_fourteen_frame_animation() {
        let mut g = actual_room();
        for x in 0..30 {
            g.world.set_tile(x, 9, "oBlock");
        }
        g.player.pos = Vec2::new(104.0, 136.0);
        let id = g.world.spawn("oGhost", Vec2::new(96.0, 120.0));
        g.step(&InputFrame::default());
        let ghost = g.world.entity(id).unwrap();
        assert_eq!(ghost.sprite, "sGhostDisappear");
        assert_eq!(ghost.value("animation_speed"), 0.2);
        assert_eq!(crate::catalog::sprite_frame_count(&ghost.sprite), 14);
        for _ in 0..69 {
            g.step(&InputFrame::default());
            assert!(g.world.entity(id).is_some());
        }
        // Allow the final floating-point image_index addition to cross 14.
        for _ in 0..2 {
            g.step(&InputFrame::default());
        }
        assert!(g.world.entity(id).is_none());
    }
    #[test]
    fn actual_step_rescued_damsel_is_invincible_during_exit_animation() {
        let mut g = actual_room();
        g.world.spawn("oExit", Vec2::new(96.0, 96.0));
        let id = g.world.spawn("oDamsel", Vec2::new(104.0, 104.0));
        g.step(&InputFrame::default());
        assert_eq!(g.world.entity(id).unwrap().state, 4);
        assert_eq!(g.world.entity(id).unwrap().value("invincible"), 1.0);
        g.world.spawn("oExplosion", Vec2::new(104.0, 104.0));
        g.step(&InputFrame::default());
        let damsel = g.world.entity(id).unwrap();
        assert_eq!(damsel.state, 4);
        assert_eq!(damsel.health, 4);
    }
    #[test]
    fn actual_step_damsel_requires_real_exit_and_finishes_exit_animation_once() {
        let mut g = actual_room();
        g.world.exit = Vec2::new(104.0, 104.0);
        let id = g.world.spawn("oDamsel", g.world.exit);
        g.step(&InputFrame::default());
        assert!(!g.flags.contains("damsel_rescued"));
        assert!(!g.data.contains_key("xdamsels"));
        g.world.spawn("oExit", Vec2::new(96.0, 96.0));
        g.step(&InputFrame::default());
        assert_eq!(g.world.entity(id).unwrap().state, 4);
        assert_eq!(g.data["damsels"], 1.0);
        assert_eq!(g.data["xdamsels"], 1.0);
        assert_eq!(g.data["pending_damsel_kisses"], 1.0);
        for _ in 0..40 {
            g.step(&InputFrame::default());
        }
        assert!(g.world.entity(id).is_none());
        assert_eq!(g.data["damsels"], 1.0);
        assert_eq!(g.data["xdamsels"], 1.0);
    }
    #[test]
    fn actual_step_damsel_water_and_lava_hazards() {
        for (liquid, health, vy) in [("oWaterSwim", 4, 0.2), ("oLava", 0, 0.1)] {
            let mut g = actual_room();
            g.world.spawn(liquid, Vec2::new(96.0, 96.0));
            let id = g.world.spawn("oDamsel", Vec2::new(104.0, 104.0));
            g.step(&InputFrame::default());
            let e = g.world.entity(id).unwrap();
            assert_eq!(e.health, health);
            assert!((e.vel.y - vy).abs() < 1e-9, "{liquid} vy={}", e.vel.y);
        }
    }
    #[test]
    fn actual_step_kiss_frame_emits_once_and_returns_to_slave() {
        let mut g = actual_room();
        g.world.set_tile(6, 7, "oBlock");
        let id = g.world.spawn("oDamsel", Vec2::new(104.0, 104.0));
        let e = g.world.entity_mut(id).unwrap();
        e.state = 6;
        e.sprite = "sDamselKissL".into();
        e.animation = 6.5;
        g.step(&InputFrame::default());
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| e.object == "oHeart")
                .count(),
            1
        );
        for _ in 0..6 {
            g.step(&InputFrame::default());
        }
        assert_eq!(g.world.entity(id).unwrap().state, 5);
    }
    #[test]
    fn ghost_moves_one_pixel_through_walls() {
        let mut g = Game::shell(1, RulesPreset::Classic);
        g.player.pos = Vec2::new(100.0, 48.0);
        let mut e = Entity::new(1, "oGhost", Vec2::new(40.0, 40.0));
        init(&mut e);
        g.world.set_tile(2, 2, "oBlock");
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
        g.world.spawn("oExit", Vec2::new(32.0, 32.0));
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
        g.world.set_tile(5, 4, "oBlock");
        let id = g.world.spawn("oBarrierEmitter", Vec2::new(80.0, 80.0));
        assert!(
            g.world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oBarrier")
        );
        g.world.spawn("oBullet", Vec2::new(88.0, 88.0));
        g.step(&InputFrame::default());
        assert!(g.world.entity(id).is_none());
        assert!(
            !g.world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oBarrier")
        );
    }
    #[test]
    fn actual_step_new_murderer_keeper_attacks_without_invented_patrol_wait() {
        let mut g = actual_room();
        for x in 0..30 {
            g.world.set_tile(x, 9, "oBlock");
        }
        g.player.pos = Vec2::new(184.0, 136.0);
        g.flags.insert("murderer".into());
        let id = g.world.spawn("oShopkeeper", Vec2::new(128.0, 128.0));
        g.world
            .entity_mut(id)
            .unwrap()
            .data
            .insert("has_gun".into(), 0.0);
        g.step(&InputFrame::default());
        assert_eq!(g.world.entity(id).unwrap().state, 4);
        assert_eq!(g.world.entity(id).unwrap().timer, 0);
        g.step(&InputFrame::default());
        assert_eq!(g.world.entity(id).unwrap().state, 2);
    }
    #[test]
    fn actual_step_stars_keeper_stops_attacking_dead_player() {
        let mut g = actual_room();
        g.player.health = 0;
        g.player.state = PlayerState::Dead;
        g.state = RunState::Dead;
        let id = g.world.spawn("oShopkeeper2", Vec2::new(128.0, 128.0));
        g.step(&InputFrame::default());
        assert_eq!(g.world.entity(id).unwrap().state, 1);
    }
    #[test]
    fn actual_step_damsel_yell_uses_ten_frames_and_frame_four_help() {
        let mut g = actual_room();
        for x in 0..30 {
            g.world.set_tile(x, 9, "oBlock");
        }
        let id = g.world.spawn("oDamsel", Vec2::new(120.0, 136.0));
        for _ in 0..201 {
            g.step(&InputFrame::default());
        }
        assert_eq!(g.world.entity(id).unwrap().state, 3);
        for frame_tick in 1..=20 {
            g.step(&InputFrame::default());
            if frame_tick == 8 {
                assert!(g.world.entities.iter().any(|e| e.object == "oYellHelp"));
            }
            assert_eq!(
                g.world.entity(id).unwrap().state,
                if frame_tick == 20 { 0 } else { 3 }
            );
        }
        assert_eq!(g.world.entity(id).unwrap().timer, 200);
    }
    #[test]
    fn actual_step_keeper_throw_ends_after_seven_source_frames() {
        let mut g = actual_room();
        for x in 0..30 {
            g.world.set_tile(x, 9, "oBlock");
        }
        g.player.pos = Vec2::new(136.0, 136.0);
        let id = g.world.spawn("oShopkeeper2", Vec2::new(128.0, 128.0));
        g.step(&InputFrame::default());
        assert_eq!(g.world.entity(id).unwrap().state, 3);
        assert_eq!(g.world.entity(id).unwrap().value("animation_speed"), 1.0);
        for _ in 0..6 {
            g.step(&InputFrame::default());
            assert_eq!(g.world.entity(id).unwrap().state, 3);
        }
        g.step(&InputFrame::default());
        assert_eq!(g.world.entity(id).unwrap().state, 2);
        assert_eq!(g.world.entity(id).unwrap().sprite, "sShopLeft");
    }
    #[test]
    fn actual_step_damsel_child_events_run_outside_inherited_item_viewport() {
        for lava in [false, true] {
            let mut g = actual_room();
            g.player.pos = Vec2::new(40.0, 136.0);
            let id = g.world.spawn("oDamsel", Vec2::new(370.0, 136.0));
            g.world.entity_mut(id).unwrap().vel.x = 1.0;
            if lava {
                g.world.spawn("oLava", Vec2::new(368.0, 128.0));
            }
            g.step(&InputFrame::default());
            let e = g.world.entity(id).unwrap();
            assert_eq!(e.pos, Vec2::new(370.0, 136.0));
            assert_eq!(e.health, if lava { 0 } else { 4 });
            if !lava {
                assert_eq!(e.timer, 199);
            }
        }
    }
    #[test]
    fn actual_step_offscreen_emitter_creates_once_and_keeps_collision_events() {
        let mut g = actual_room();
        g.player.pos = Vec2::new(40.0, 128.0);
        let id = g.world.spawn("oBarrierEmitter", Vec2::new(352.0, 80.0));
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| e.object == "oBarrier")
                .count(),
            1
        );
        g.step(&InputFrame::default());
        assert!(g.world.entity(id).is_some()); // narrow Step support check skipped
        let bullet = g.world.spawn("oBullet", Vec2::new(356.0, 84.0));
        g.world.entity_mut(bullet).unwrap().vel = Vec2::default();
        g.step(&InputFrame::default());
        assert!(g.world.entity(id).is_none());
        assert!(!g.world.entities.iter().any(|e| e.object == "oBarrier"));
    }
    #[test]
    fn actual_step_emitter_destruction_callbacks_run_once_when_causes_overlap() {
        for (supported, bullet) in [(false, false), (true, true), (false, true)] {
            let mut g = actual_room();
            if supported {
                g.world.set_tile(10, 4, "oBlock");
            }
            let emitter = g.world.spawn("oBarrierEmitter", Vec2::new(160.0, 80.0));
            if bullet {
                g.world.spawn("oBullet", Vec2::new(168.0, 88.0));
            }
            g.step(&InputFrame::default());
            assert!(g.world.entity(emitter).is_none());
            assert_eq!(
                g.world
                    .entities
                    .iter()
                    .filter(|e| e.object == "oFlareSpark")
                    .count(),
                6
            );
            assert_eq!(g.data["shake"], 10.0);
            assert_eq!(
                g.events
                    .iter()
                    .filter(|event| matches!(event,
                        GameEvent::Sound { name, .. } if name == "sndSmallExplode"
                    ))
                    .count(),
                1,
                "supported={supported}, bullet={bullet}"
            );
        }
    }
    #[test]
    fn actual_step_late_explosion_runs_emitter_destroy_event_once() {
        let mut g = actual_room();
        g.world.set_tile(10, 4, "oBlock");
        let emitter = g.world.spawn("oBarrierEmitter", Vec2::new(160.0, 80.0));
        g.world.spawn("oExplosion", Vec2::new(160.0, 80.0));
        g.step(&InputFrame::default());
        assert!(g.world.entity(emitter).is_none());
        assert!(!g.world.entities.iter().any(|e| e.object == "oBarrier"));
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| e.object == "oFlareSpark")
                .count(),
            6
        );
        assert_eq!(g.data["shake"], 10.0);
        g.step(&InputFrame::default());
        assert!(!g.events.iter().any(|event| matches!(event,
            GameEvent::Sound { name, .. } if name == "sndSmallExplode"
        )));
    }
    #[test]
    fn actual_step_scarab_destroy_bursts_distinguish_collection_and_death() {
        for (collected, dead, expected_sparks) in
            [(true, false, 3), (false, true, 6), (false, false, 3)]
        {
            let mut g = actual_room();
            let pos = if collected {
                Vec2::new(96.0, 120.0)
            } else if dead {
                Vec2::new(160.0, 80.0)
            } else {
                Vec2::new(-32.0, 80.0)
            };
            if collected {
                g.player.pos = Vec2::new(104.0, 128.0);
            }
            let id = g.world.spawn("oScarab", pos);
            if dead {
                g.world.entity_mut(id).unwrap().health = 0;
            }
            g.step(&InputFrame::default());
            assert!(
                g.world.entity(id).is_none(),
                "collected={collected}, dead={dead}"
            );
            assert_eq!(
                g.world
                    .entities
                    .iter()
                    .filter(|e| e.object == "oFlareSpark")
                    .count(),
                expected_sparks,
                "collected={collected}, dead={dead}"
            );
            assert_eq!(g.player.money, 0);
            assert_eq!(
                g.data.get("collect").copied().unwrap_or(0.0),
                if collected { 4000.0 } else { 0.0 }
            );
            assert_eq!(
                g.data.get("scarabs").copied().unwrap_or(0.0),
                if collected { 1.0 } else { 0.0 }
            );
        }
    }
    #[test]
    fn actual_step_scarab_collection_runs_outside_ai_step_viewport() {
        let mut g = actual_room();
        g.world.area = Area::Temple;
        g.player.pos = Vec2::new(338.0, 88.0);
        let id = g.world.spawn("oScarab", Vec2::new(330.0, 80.0));
        assert_eq!(g.camera, Vec2::default());
        g.step(&InputFrame::default());
        assert!(g.world.entity(id).is_none());
        assert_eq!(g.player.money, 0);
        assert_eq!(g.data["collect"], 12000.0);
        assert_eq!(g.data["scarabs"], 1.0);
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| e.object == "oFlareSpark")
                .count(),
            3
        );
        g.step(&InputFrame::default());
        assert_eq!(g.player.money, 0);
        assert_eq!(g.data["collect"], 12000.0);
        assert_eq!(g.data["scarabs"], 1.0);
        assert!(!g.events.iter().any(|event| matches!(event,
            GameEvent::Collected { object, .. } if object == "oScarab"
        )));
    }
    #[test]
    fn actual_step_scarab_queues_literal_area_value_then_counts_money_and_earnings() {
        for (area, value) in [
            (Area::Mines, 4000),
            (Area::Jungle, 8000),
            (Area::Temple, 12000),
        ] {
            let mut g = actual_room();
            g.world.area = area;
            for x in 0..30 {
                g.world.set_tile(x, 9, "oBlock");
            }
            g.player.pos = Vec2::new(104.0, 128.0);
            g.player.money = 500;
            g.data.insert("xmoney".into(), 25.0);
            let id = g.world.spawn("oScarab", Vec2::new(96.0, 120.0));
            g.step(&InputFrame::default());
            assert!(g.world.entity(id).is_none());
            assert_eq!(g.player.money, 500);
            assert_eq!(g.data["xmoney"], 25.0);
            assert_eq!(g.data["collect"], f64::from(value));
            assert_eq!(g.data["collectCounter"], 20.0);
            assert_eq!(g.data["scarabs"], 1.0);
            for _ in 0..20 {
                g.step(&InputFrame::default());
            }
            assert_eq!(g.data["collectCounter"], 0.0);
            assert_eq!(g.player.money, 500);
            g.step(&InputFrame::default());
            assert_eq!(g.player.money, 600);
            assert_eq!(g.data["xmoney"], 125.0);
            assert_eq!(g.data["collect"], f64::from(value - 100));
            for _ in 1..value / 100 {
                g.step(&InputFrame::default());
            }
            assert_eq!(g.player.money, 500 + value);
            assert_eq!(g.data["xmoney"], 25.0 + f64::from(value));
            assert_eq!(g.data["collect"], 0.0);
            assert_eq!(g.data["scarabs"], 1.0);
        }
    }
    #[test]
    fn actual_step_scarab_queue_adds_to_existing_collection_and_caps_delay() {
        let mut g = actual_room();
        g.player.pos = Vec2::new(104.0, 128.0);
        g.data.insert("collect".into(), 250.0);
        g.data.insert("collectCounter".into(), 95.0);
        g.world.spawn("oScarab", Vec2::new(96.0, 120.0));
        g.step(&InputFrame::default());
        assert_eq!(g.player.money, 0);
        assert_eq!(g.data["collect"], 4250.0);
        assert_eq!(g.data["collectCounter"], 100.0);
        assert_eq!(g.data["scarabs"], 1.0);
    }
    #[test]
    fn actual_step_attacking_unarmed_keeper_consumes_shotgun_and_clears_holder() {
        for held in [false, true] {
            let mut g = actual_room();
            for x in 0..30 {
                g.world.set_tile(x, 6, "oBlock");
            }
            g.player.pos = if held {
                Vec2::new(84.0, 88.0)
            } else {
                Vec2::new(200.0, 88.0)
            };
            g.player.facing = -1;
            g.player.invincible = 30;
            let keeper = g.world.spawn("oShopkeeper2", Vec2::new(72.0, 80.0));
            g.world
                .entity_mut(keeper)
                .unwrap()
                .data
                .insert("firing".into(), 30.0);
            let gun = g.world.spawn("oShotgun", Vec2::new(80.0, 88.0));
            g.world
                .entity_mut(gun)
                .unwrap()
                .data
                .insert("gravity".into(), 0.0);
            g.world.entity_mut(gun).unwrap().held = held;
            if held {
                g.player.held = Some(gun);
            }
            g.step(&InputFrame::default());
            assert_eq!(g.world.entity(keeper).unwrap().value("has_gun"), 1.0);
            assert!(g.world.entity(gun).is_none());
            assert_eq!(g.player.held, None);
        }
    }
    #[test]
    fn stars_corpse_expires_after_source_death_timer() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.flags.insert("hub_stars".into());
        game.player.pos = Vec2::new(200.0, 200.0);
        game.world.set_tile(4, 5, "oBlock");
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
