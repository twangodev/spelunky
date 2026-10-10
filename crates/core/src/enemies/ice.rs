//! Ice enemies translated from Classic 1.1 Create/Step/Animation end events.
//! Positions retain the original GameMaker sprite origins and source offsets.
//! Limitations: rectangular masks, ray-based Yeti sight, simplified corpse
//! settling, and omitted blood/fire trails and dynamic gold/tile-layer effects.
//! Source contains no oAlienUFO object; this module does not invent that alias.
use super::move_body;
use crate::{Entity, Game, PlayerState, Vec2};

pub fn matches(object: &str) -> bool {
    matches!(
        object,
        "oYeti"
            | "oYetiKing"
            | "oAlien"
            | "oUFO"
            | "oAlienBoss"
            | "oAlienEject"
            | "oUFOCrash"
            | "oLaser"
            | "oPsychicWave"
            | "oPsychicCreate"
    )
}

pub fn init(e: &mut Entity) {
    e.health = match e.object.as_str() {
        "oYeti" => 5,
        "oYetiKing" => 30,
        "oAlienBoss" => 10,
        _ => 1,
    };
    e.state = 0;
    e.data.insert("phase".into(), 0.0);
    e.data.insert("animation_speed".into(), 0.5);
    if matches!(e.object.as_str(), "oYetiKing" | "oAlienBoss") {
        e.data.insert(
            "size_width".into(),
            if e.object == "oYetiKing" { 20.0 } else { 32.0 },
        );
        e.data.insert("size_height".into(), 32.0);
        e.data.insert("animation_speed".into(), 0.25);
    }
    if e.object == "oYetiKing" {
        e.data.insert("contact_damage".into(), 2.0);
    }
    match e.object.as_str() {
        "oAlienBoss" => {
            e.facing = -1;
            e.data.insert("psychic_recover".into(), 100.0);
        }
        "oYeti" | "oYetiKing" | "oAlien" => e.vel.x = 2.5,
        "oAlienEject" => e.vel.y = -6.0,
        "oUFO" => {
            e.data.insert("shift_toggle".into(), -1.0);
        }
        _ => {}
    }
    if e.object == "oAlienEject" {
        e.data.insert("drift_unset".into(), 1.0);
    }
    e.sprite = match e.object.as_str() {
        "oYeti" => "sYetiLeft",
        "oYetiKing" => "sYetiKingRight",
        "oAlienBoss" => "sAlienBoss",
        "oAlienEject" => "sAlienEject",
        "oUFOCrash" => "sUFOCrash",
        "oPsychicWave" => "sPsychicWave",
        "oPsychicCreate" => "sPsychicCreate",
        "oLaser" => "sLaser",
        "oUFO" => "sUFO",
        _ => "sAlien",
    }
    .into();
}

fn counter(e: &mut Entity, key: &str) -> f64 {
    let value = (e.value(key) - 1.0).max(0.0);
    e.data.insert(key.into(), value);
    value
}

fn phase(e: &mut Entity, speed: f64) -> f64 {
    let value = e.value("phase") + speed;
    e.data.insert("phase".into(), value);
    e.animation = value;
    value
}

fn enter(e: &mut Entity, state: i32) {
    e.state = state;
    e.animation = 0.0;
    e.data.insert("phase".into(), 0.0);
}

pub fn tick(game: &mut Game, e: &mut Entity) {
    match e.object.as_str() {
        "oYeti" => yeti(game, e),
        "oYetiKing" => king(game, e),
        "oAlien" => alien(game, e),
        "oUFO" => ufo(game, e),
        "oAlienBoss" => boss(game, e),
        "oAlienEject" => eject(game, e),
        "oUFOCrash" => crash(game, e),
        "oLaser" => laser(game, e),
        "oPsychicWave" => psychic(game, e),
        "oPsychicCreate" => {
            let delta = Vec2::new(e.value("target_x") - e.pos.x, e.value("target_y") - e.pos.y);
            let distance = delta.x.hypot(delta.y);
            if distance > 0.0 {
                e.pos.x += delta.x / distance * 2.0;
                e.pos.y += delta.y / distance * 2.0;
            }
            if phase(e, 0.4) >= 6.0 {
                e.alive = false;
            }
        }
        _ => {}
    }
}

fn yeti(game: &mut Game, e: &mut Entity) {
    let (wall, floor) = move_body(&game.world, e, 0.6);
    if e.health < 1 {
        e.state = 99;
        e.sprite = "sYetiDeadL".into();
        if e.value("death_sound") == 0.0 {
            game.sound("sndCavemanDie", e.pos);
            e.data.insert("death_sound".into(), 1.0);
        }
        return;
    }
    if e.held {
        e.sprite = "sYetiHeldL".into();
        return;
    }
    if e.state == 98 {
        e.sprite = "sYetiStunL".into();
        if floor {
            e.timer = e.timer.saturating_sub(1);
            if e.timer == 0 {
                enter(e, 0);
            }
        }
        return;
    }
    if e.state == 3 {
        e.vel.x = 0.0;
        e.sprite = "sYetiThrowL".into();
        if phase(e, 0.5) >= 7.0 {
            enter(e, 0);
        }
        return;
    }
    // Yeti grabs and throws; contact itself causes no immediate damage except
    // when there is insufficient headroom, matching its collision event.
    if e.bounds().intersects(game.player.bounds())
        && game.player.invincible == 0
        && game.player.stun == 0
        && !(game.player.vel.y > 0.0 && game.player.pos.y < e.pos.y + 5.0)
    {
        if game.world.solid_at(e.pos.x + 8.0, e.pos.y - 4.0) {
            game.damage_player(1);
        } else {
            e.facing = if game.player.pos.x > e.pos.x + 8.0 {
                1
            } else {
                -1
            };
            game.player.pos = Vec2::new(e.pos.x + if e.facing > 0 { 0.0 } else { 16.0 }, e.pos.y);
            game.player.previous_pos = game.player.pos;
            enter(e, 3);
            game.player.vel = Vec2::new(f64::from(e.facing) * 6.0, -6.0);
            game.player.stun = 30;
            game.player.state = PlayerState::Stunned;
            game.player.data.insert("wall_hurt".into(), 1.0);
            if let Some(id) = game.player.held.take() {
                if let Some(item) = game.world.entity_mut(id) {
                    item.held = false;
                    item.vel = game.player.vel;
                }
            }
            e.sprite = "sYetiThrowL".into();
            return;
        }
    }
    match e.state {
        0 => {
            if floor && wall {
                e.vel.y = -6.0;
                e.vel.x = f64::from(e.facing);
            }
            if floor && counter(e, "counter") == 0.0 {
                e.facing = if game.rng.range(2) == 0 { -1 } else { 1 };
                enter(e, 1);
            }
        }
        1 => {
            if wall {
                e.facing = -e.facing;
            }
            if floor {
                if game.rng.range(100) == 0
                    || !game.world.solid_at(
                        e.pos.x + if e.facing < 0 { -1.0 } else { 16.0 },
                        e.pos.y + 16.0,
                    )
                {
                    enter(e, 0);
                    e.vel.x = 0.0;
                    e.data
                        .insert("counter".into(), f64::from(20 + game.rng.range(31)));
                } else {
                    e.vel.x = f64::from(e.facing) * 1.5;
                }
            }
        }
        2 => {
            if wall {
                e.facing = -e.facing;
            }
            e.vel.x = f64::from(e.facing) * 3.0;
        }
        _ => {}
    }
    // EnemySight is represented by an unobstructed horizontal ray every six ticks.
    if counter(e, "sight") == 0.0 {
        e.data.insert("sight".into(), 5.0);
        let dx = game.player.pos.x - (e.pos.x + 8.0);
        if dx * f64::from(e.facing) > 0.0
            && dx.abs() < 160.0
            && (game.player.pos.y - (e.pos.y + 8.0)).abs() < 8.0
        {
            let clear = (1..=(dx.abs() / 8.0) as i32).all(|i| {
                !game.world.solid_at(
                    e.pos.x + 8.0 + f64::from(e.facing) * f64::from(i) * 8.0,
                    e.pos.y + 8.0,
                )
            });
            if clear {
                enter(e, 2);
            }
        }
    }
    if e.vel.x > 0.0 {
        e.vel.x -= 0.1;
    } else if e.vel.x < 0.0 {
        e.vel.x += 0.1;
    }
    if e.vel.x.abs() < 0.5 {
        e.vel.x = 0.0;
    }
    if e.state != 3 {
        e.sprite = if e.vel.x == 0.0 {
            "sYetiLeft"
        } else {
            "sYetiRunLeft"
        }
        .into();
        phase(e, if e.state == 2 { 1.0 } else { 0.5 });
    }
}

fn alien(game: &mut Game, e: &mut Entity) {
    let (wall, floor) = move_body(&game.world, e, 0.6);
    if e.health < 1 {
        e.alive = false;
        return;
    }
    if e.state == 98 {
        if floor {
            e.timer = e.timer.saturating_sub(1);
            if e.timer == 0 {
                enter(e, 0);
            }
        }
        return;
    }
    if e.state == 0 {
        if counter(e, "counter") == 0.0 {
            e.facing = if game.rng.range(2) == 0 { -1 } else { 1 };
            enter(e, 1);
        }
    } else if e.state == 1 {
        if wall {
            e.facing = -e.facing;
        }
        let left = game.world.solid_at(e.pos.x - 1.0, e.pos.y + 16.0)
            && !game.world.solid_at(e.pos.x - 1.0, e.pos.y);
        let right = game.world.solid_at(e.pos.x + 16.0, e.pos.y + 16.0)
            && !game.world.solid_at(e.pos.x + 16.0, e.pos.y);
        if e.facing < 0 && !left {
            e.facing = 1;
        } else if e.facing > 0 && !right {
            e.facing = -1;
        }
        e.vel.x = if !left && !right {
            0.0
        } else {
            f64::from(e.facing)
        };
        if game.rng.range(100) == 0 {
            enter(e, 0);
            e.vel.x = 0.0;
            e.data
                .insert("counter".into(), f64::from(20 + game.rng.range(31)));
        }
    }
    phase(e, 0.5);
}

fn king(game: &mut Game, e: &mut Entity) {
    if e.health < 1 {
        for _ in 0..4 {
            let item = if game.rng.range(12) == 0 {
                "oSpikeShoes"
            } else if game.rng.range(2) == 0 {
                "oSapphireBig"
            } else {
                "oRopePile"
            };
            let id = game
                .world
                .spawn(item, Vec2::new(e.pos.x + 16.0, e.pos.y + 16.0));
            let vx = f64::from(game.rng.range(4)) - f64::from(game.rng.range(4));
            let vy = -f64::from(1 + game.rng.range(2));
            if let Some(loot) = game.world.entity_mut(id) {
                loot.vel = Vec2::new(vx, vy);
            }
        }
        e.alive = false;
        return;
    }
    let (wall, _) = move_body(&game.world, e, 0.6);
    counter(e, "attack_timer");
    counter(e, "counter");
    match e.state {
        0 => {
            e.vel.x = 0.0;
            if e.value("counter") == 0.0 {
                enter(e, 1);
            }
        }
        1 => {
            let dx = game.player.pos.x - (e.pos.x + 16.0);
            let dy = (game.player.pos.y - (e.pos.y + 16.0)).abs();
            if wall || (dx * f64::from(e.facing) < 0.0 && dy < 16.0 && e.value("counter") == 0.0) {
                enter(e, 2);
                e.vel.x = 0.0;
                e.data.insert("counter".into(), 30.0);
            } else if dx * f64::from(e.facing) > 0.0 && dy < 32.0 && e.value("attack_timer") == 0.0
            {
                enter(e, 3);
                e.vel.x = 0.0;
            } else {
                e.vel.x = f64::from(e.facing);
            }
        }
        2 => {
            e.vel.x = 0.0;
            if phase(e, 0.25) >= 5.0 {
                e.facing = -e.facing;
                enter(e, 1);
            }
        }
        3 => {
            e.vel.x = 0.0;
            e.data.insert("attack_timer".into(), 100.0);
            let frame = phase(e, 0.5);
            if (7.0..13.0).contains(&frame) {
                if frame == 7.0 {
                    game.sound("sndYetiYell", e.pos);
                }
                let mut falling = Vec::new();
                for y in 0..game.world.height {
                    for x in 0..game.world.width {
                        let pos = Vec2::new(x as f64 * 16.0 + 8.0, y as f64 * 16.0 + 8.0);
                        if game
                            .world
                            .tile(x as i32, y as i32)
                            .is_some_and(|t| t.object == "oIce")
                            && (pos.x - (e.pos.x + 16.0)).abs() > 16.0
                            && pos.distance(e.pos) < 96.0
                            && game.rng.range(60) == 0
                        {
                            falling.push((x, y, pos));
                        }
                    }
                }
                for (x, y, pos) in falling {
                    game.world.set_tile(x, y, "");
                    game.world
                        .spawn("oIceBlock", Vec2::new(pos.x - 8.0, pos.y - 8.0));
                }
                for ice in &mut game.world.entities {
                    if ice.object == "oThinIce" {
                        let thickness = ice.data.entry("thickness".into()).or_insert(20.0);
                        *thickness -= 2.0;
                    }
                }
            }
            if frame >= 13.0 {
                enter(e, 0);
                e.data.insert("counter".into(), 30.0);
            }
        }
        _ => {}
    }
    e.sprite = match (e.state, e.facing < 0) {
        (2, true) => "sYetiKingTurnR",
        (2, false) => "sYetiKingTurnL",
        (3, true) => "sYetiKingYellL",
        (3, false) => "sYetiKingYellR",
        (1, true) => "sYetiKingWalkL",
        (1, false) => "sYetiKingWalkR",
        (_, true) => "sYetiKingLeft",
        _ => "sYetiKingRight",
    }
    .into();
    if e.state < 2 {
        phase(e, 0.25);
    }
}

fn ufo(game: &mut Game, e: &mut Entity) {
    if e.value("shift_toggle") < 0.0 {
        e.data
            .insert("shift_toggle".into(), f64::from(game.rng.range(2)));
    }
    if e.health < 1 {
        game.world.spawn("oUFOCrash", e.pos);
        e.alive = false;
        return;
    }
    let (wall, _) = move_body(&game.world, e, 0.0);
    if e.pos.distance(game.player.pos) < 160.0 && e.value("alerted") == 0.0 {
        e.data.insert("alerted".into(), 1.0);
        game.sound("sndAlien", e.pos);
    }
    if e.state == 2 {
        e.vel = Vec2::default();
        e.sprite = "sUFOBlast".into();
        let frame = phase(e, 0.5);
        if frame == 16.0 {
            game.world
                .spawn("oLaser", Vec2::new(e.pos.x + 8.0, e.pos.y + 16.0));
            game.sound("sndLaser", e.pos);
        }
        if frame >= 20.0 {
            enter(e, 1);
            e.sprite = "sUFO".into();
        }
        return;
    }
    let limit = if e.state == 1 { 64.0 } else { 32.0 };
    let mut shift = e.value("shift");
    if e.value("shift_toggle") == 0.0 {
        e.vel.x = 1.0;
        shift += 1.0;
        if shift >= limit {
            e.data.insert("shift_toggle".into(), 1.0);
        }
    } else {
        e.vel.x = -1.0;
        shift -= 1.0;
        if shift <= -limit {
            e.data.insert("shift_toggle".into(), 0.0);
        }
    }
    if wall {
        e.data
            .insert("shift_toggle".into(), if e.vel.x > 0.0 { 1.0 } else { 0.0 });
    }
    e.data.insert("shift".into(), shift);
    if e.state == 1 {
        e.vel.y = 0.0;
        if (game.player.pos.x - (e.pos.x + 8.0)).abs() < 8.0 {
            enter(e, 2);
            game.sound("sndLaserCharge", e.pos);
        }
        if game.player.pos.y < e.pos.y || e.pos.y < game.player.pos.y - 96.0 {
            enter(e, 0);
        }
    } else {
        let next = if e.pos.y > game.player.pos.y - 48.0 {
            -2.0
        } else {
            2.0
        };
        if e.vel.y == -next {
            enter(e, 1);
        }
        e.vel.y = next;
    }
    phase(e, 0.5);
}

fn boss(game: &mut Game, e: &mut Entity) {
    if e.health < 1 && e.state != 99 {
        enter(e, 99);
        e.sprite = "sAlienBossDie".into();
        for _ in 0..4 {
            let name = match game.rng.range(3) {
                0 => "oEmeraldBig",
                1 => "oSapphireBig",
                _ => "oRubyBig",
            };
            let id = game
                .world
                .spawn(name, Vec2::new(e.pos.x + 16.0, e.pos.y + 16.0));
            let vx = f64::from(game.rng.range(4)) - f64::from(game.rng.range(4));
            if let Some(gem) = game.world.entity_mut(id) {
                gem.vel = Vec2::new(vx, -2.0);
            }
        }
    }
    e.vel.x = 0.0;
    move_body(&game.world, e, 0.6);
    if e.state == 99 {
        if phase(e, 0.25) >= 9.0 {
            e.sprite = "sAlienBossDead".into();
            for emitter in &mut game.world.entities {
                if emitter.object == "oBarrierEmitter" {
                    emitter.alive = false;
                }
            }
        }
        return;
    }
    if counter(e, "psychic_recover") == 0.0
        && e.pos.distance(game.player.pos) < 96.0
        && game.player.health > 0
        && game.player.stun == 0
        && game.player.invincible == 0
    {
        for _ in 0..6 {
            let pos = Vec2::new(
                e.pos.x + 16.0 + f64::from(game.rng.range(33)) - f64::from(game.rng.range(33)),
                e.pos.y + 16.0 + f64::from(game.rng.range(33)) - f64::from(game.rng.range(33)),
            );
            let id = game.world.spawn("oPsychicCreate", pos);
            if let Some(effect) = game.world.entity_mut(id) {
                effect.data.insert("target_x".into(), e.pos.x + 16.0);
                effect.data.insert("target_y".into(), e.pos.y + 16.0);
            }
        }
        game.world
            .spawn("oPsychicWave", Vec2::new(e.pos.x + 16.0, e.pos.y + 16.0));
        e.data.insert("psychic_recover".into(), 100.0);
        game.sound("sndPsychic", e.pos);
    }
    phase(e, 0.25);
}

fn eject(game: &mut Game, e: &mut Entity) {
    if e.value("drift_unset") > 0.0 {
        e.facing = if game.rng.range(2) == 0 { -1 } else { 1 };
        e.data.insert("drift_unset".into(), 0.0);
    }
    if e.bounds().intersects(game.player.bounds()) {
        if game.player.health > 0 && game.player.vel.y > 0.0 && game.player.pos.y < e.pos.y - 3.0 {
            game.player.vel.y = -6.0 - 0.2 * game.player.vel.y;
            game.player.data.insert("fall_timer".into(), 0.0);
            e.health = 0;
            e.alive = false;
            game.sound("sndHit", e.pos);
            return;
        } else if game.player.invincible == 0 {
            game.damage_player(1);
            game.player.invincible = 30;
            game.player.vel.x = if game.player.pos.x < e.pos.x {
                -6.0
            } else {
                6.0
            };
            if game.player.pos.y < e.pos.y {
                game.player.vel.y = -6.0;
            }
        }
    }
    let (_, floor) = move_body(&game.world, e, if e.state == 0 { 0.5 } else { 0.0 });
    if e.state == 0 {
        e.vel.x = 0.0;
        if e.vel.y >= 0.0 || game.world.solid_at(e.pos.x, e.pos.y - 5.0) {
            enter(e, 2);
            e.vel.y = 0.0;
            e.sprite = "sAlienDeploy".into();
        }
    } else {
        e.vel.y = 2.0;
        if floor {
            game.world
                .spawn("oAlien", Vec2::new(e.pos.x - 8.0, e.pos.y - 12.0));
            e.alive = false;
            return;
        }
        e.vel.x = f64::from(e.facing);
        if game.world.solid_at(e.pos.x + e.vel.x * 8.0, e.pos.y) {
            e.vel.x = 0.0;
            e.facing = 0;
        }
        if phase(e, 0.6) >= 6.0 {
            e.sprite = "sAlienParachute".into();
        }
    }
}

fn crash(game: &mut Game, e: &mut Entity) {
    e.pos.x += e.vel.x;
    e.pos.y += e.vel.y;
    e.vel.y = (e.vel.y + 0.6).min(6.0);
    if game.world.collides(e.bounds())
        || game.world.entities.iter().any(|other| {
            other.alive
                && other.id != e.id
                && other.object != "oUFO"
                && crate::catalog::is_a(&other.object, "oEnemy")
                && other.bounds().intersects(e.bounds())
        })
    {
        game.world
            .spawn("oExplosion", Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0));
        game.sound("sndExplosion", e.pos);
        e.alive = false;
    }
}

fn laser(game: &mut Game, e: &mut Entity) {
    e.pos.y += e.vel.y;
    e.vel.y += 0.6;
    if e.vel.y > 4.0 {
        e.vel.y = 0.0;
    }
    let tx = (e.pos.x / 16.0).floor() as i32;
    let ty = (e.pos.y / 16.0).floor() as i32;
    if let Some(tile) = game.world.tile(tx, ty) {
        if tile.is_solid() {
            if tile.is_destructible() {
                game.world.set_tile(tx as usize, ty as usize, "");
            }
            game.world.spawn("oLaserExplode", e.pos);
            e.alive = false;
            return;
        }
    } else {
        e.alive = false;
        return;
    }
    if e.bounds().intersects(game.player.bounds()) {
        game.damage_player(3);
        game.player.vel = Vec2::new(
            if e.pos.x < game.player.pos.x {
                2.0
            } else {
                -2.0
            },
            -4.0,
        );
        game.world.spawn("oLaserExplode", e.pos);
        e.alive = false;
    }
    for enemy in &mut game.world.entities {
        if enemy.alive
            && enemy.object != "oUFO"
            && enemy.object != "oLaser"
            && (crate::catalog::is_a(&enemy.object, "oEnemy") || enemy.object == "oDamsel")
            && enemy.bounds().intersects(e.bounds())
            && enemy.value("invincible") == 0.0
        {
            enemy.health -= 3;
            enemy.vel = Vec2::new(-1.0, -6.0);
            e.alive = false;
            break;
        }
    }
}

fn psychic(game: &mut Game, e: &mut Entity) {
    let dx = game.player.pos.x - e.pos.x;
    let dy = game.player.pos.y - e.pos.y;
    let distance = dx.hypot(dy);
    if distance > 0.0 {
        e.pos.x += dx / distance * 2.0;
        e.pos.y += dy / distance * 2.0;
    }
    if phase(e, 0.25) >= 17.0 {
        e.alive = false;
        return;
    }
    if e.bounds().intersects(game.player.bounds())
        && game.player.stun == 0
        && game.player.health > 0
        && game.player.invincible == 0
    {
        game.damage_player(1);
        game.player.stun = 40;
        game.player.vel = Vec2::new(
            if e.pos.x < game.player.pos.x {
                2.0
            } else {
                -2.0
            },
            -4.0,
        );
    }
    for enemy in &mut game.world.entities {
        if enemy.alive
            && enemy.object != "oAlienBoss"
            && enemy.object != "oPsychicWave"
            && (crate::catalog::is_a(&enemy.object, "oEnemy") || enemy.object == "oDamsel")
            && enemy.bounds().intersects(e.bounds())
            && enemy.value("invincible") == 0.0
        {
            enemy.health -= 3;
            enemy.vel = Vec2::new(-1.0, -6.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Area, RulesPreset, World};
    fn game() -> Game {
        let mut g = Game::shell(7, RulesPreset::Classic);
        g.world = World::new(20, 20, Area::Ice, 9);
        g.player.pos = Vec2::new(100.0, 100.0);
        g
    }
    #[test]
    fn source_health_values() {
        for (name, hp) in [
            ("oYeti", 5),
            ("oYetiKing", 30),
            ("oAlienBoss", 10),
            ("oUFO", 1),
        ] {
            let mut e = Entity::new(1, name, Vec2::new(50.0, 50.0));
            init(&mut e);
            assert_eq!(e.health, hp);
        }
    }
    #[test]
    fn ufo_fires_once_at_frame_sixteen() {
        let mut g = game();
        let mut e = Entity::new(1, "oUFO", Vec2::new(100.0, 40.0));
        init(&mut e);
        e.state = 2;
        for _ in 0..40 {
            tick(&mut g, &mut e);
        }
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| e.object == "oLaser")
                .count(),
            1
        );
        assert_eq!(e.state, 1);
    }
    #[test]
    fn psychic_attack_requires_recovery_and_range() {
        let mut g = game();
        let mut e = Entity::new(1, "oAlienBoss", Vec2::new(100.0, 100.0));
        init(&mut e);
        e.data.insert("psychic_recover".into(), 2.0);
        tick(&mut g, &mut e);
        assert!(g.world.entities.is_empty());
        tick(&mut g, &mut e);
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| e.object == "oPsychicWave")
                .count(),
            1
        );
    }
    #[test]
    fn dead_king_drops_four_items() {
        let mut g = game();
        let mut e = Entity::new(1, "oYetiKing", Vec2::new(100.0, 100.0));
        init(&mut e);
        e.health = 0;
        tick(&mut g, &mut e);
        assert!(!e.alive);
        assert_eq!(g.world.entities.len(), 4);
    }
    #[test]
    fn laser_removes_destructible_tile_only() {
        let mut g = game();
        g.world.set_tile(4, 4, "oIce");
        let mut e = Entity::new(1, "oLaser", Vec2::new(72.0, 72.0));
        init(&mut e);
        tick(&mut g, &mut e);
        assert!(!e.alive);
        assert!(!g.world.tile(4, 4).unwrap().is_solid());
    }

    #[test]
    fn psychic_wave_has_source_damage_and_lifetime() {
        let mut g = game();
        let mut e = Entity::new(1, "oPsychicWave", g.player.pos);
        init(&mut e);
        tick(&mut g, &mut e);
        assert_eq!(g.player.health, 3);
        assert_eq!(g.player.stun, 40);
        assert!(e.alive);
        for _ in 1..68 {
            tick(&mut g, &mut e);
        }
        assert!(!e.alive);
    }

    #[test]
    fn boss_death_releases_barrier_after_animation() {
        let mut g = game();
        let id = g.world.spawn("oBarrierEmitter", Vec2::new(200.0, 100.0));
        let mut e = Entity::new(9, "oAlienBoss", Vec2::new(50.0, 50.0));
        init(&mut e);
        e.health = 0;
        for _ in 0..36 {
            tick(&mut g, &mut e);
        }
        assert_eq!(e.sprite, "sAlienBossDead");
        assert!(g.world.entity(id).is_none());
    }
}
