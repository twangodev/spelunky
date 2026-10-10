//! Ice enemies translated from Classic 1.1 Create/Step/Animation end events.
//! Positions retain the original GameMaker sprite origins and source offsets.
//! Limitations: rectangular masks, approximate shared movement resolution,
//! and omitted blood/fire trails and dynamic gold/tile-layer effects.
//! Source contains no oAlienUFO object; this module does not invent that alias.
use super::move_body;
use crate::{Entity, Game, GameEvent, PlayerState, Rect, Vec2};

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
        for (key, value) in [
            ("collision_left", -4.0),
            ("collision_top", -4.0),
            ("collision_right", 4.0),
            ("collision_bottom", 4.0),
        ] {
            e.data.insert(key.into(), value);
        }
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

fn distance_to_player(game: &Game, e: &Entity) -> f64 {
    let a = e.sprite_bounds();
    let b = game.player.sprite_bounds();
    let dx = (b.x - a.x - a.w).max(a.x - b.x - b.w).max(0.0);
    let dy = (b.y - a.y - a.h).max(a.y - b.y - b.h).max(0.0);
    dx.hypot(dy)
}

/// Image playback and Animation End are independent of the viewport guard
/// around Alien Boss Step. Item hits retain the current image index.
pub(super) fn tick_independent_animation(e: &mut Entity) {
    if e.object != "oAlienBoss" {
        return;
    }
    e.data.insert("boss_animation_advanced".into(), 0.0);
    if e.sprite != "sAlienBossHurt" {
        return;
    }
    e.data.insert("animation_speed".into(), 0.8);
    let frame = e.animation.rem_euclid(5.0) + 0.8;
    e.animation = frame.rem_euclid(5.0);
    e.data.insert("phase".into(), e.animation);
    e.data.insert("boss_animation_advanced".into(), 1.0);
    if frame >= 5.0 {
        e.sprite = "sAlienBoss".into();
    }
}

pub(super) fn animation_end(e: &mut Entity, ended_sprite: &str) {
    if e.object != "oYetiKing" {
        return;
    }
    match ended_sprite {
        "sYetiKingTurnR" => {
            e.facing = 1;
            e.state = 1;
        }
        "sYetiKingTurnL" => {
            e.facing = -1;
            e.state = 1;
        }
        "sYetiKingYellL" | "sYetiKingYellR" => {
            e.state = 0;
            e.data.insert("counter".into(), 30.0);
            e.data.insert("animation_speed".into(), 0.25);
        }
        _ => {}
    }
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
    let impact_y = (e.vel.y + super::inherited::gravity(e, 0.6)).min(10.0);
    let (wall, floor) = move_body(&game.world, e, 0.6);
    let crush_y = if e.state >= 98 || e.health <= 0 {
        12.0
    } else {
        8.0
    };
    if game
        .world
        .solid_point_except(Vec2::new(e.pos.x + 8.0, e.pos.y + crush_y), e.id)
    {
        e.health = 0;
        e.alive = false;
        game.sound("sndCavemanDie", e.pos);
        return;
    }
    if e.health < 1 {
        settle_yeti(e, wall, floor, impact_y);
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
        settle_yeti(e, wall, floor, impact_y);
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
    if e.sprite_bounds().intersects(game.player.sprite_bounds())
        && (game.player.pos.x - (e.pos.x + 8.0)).abs() <= 8.0
        && game.player.invincible == 0
        && !super::can_stomp(game, e)
    {
        if game.world.solid_at(e.pos.x + 8.0, e.pos.y - 4.0) {
            hurt_player(game, 1);
            game.player.invincible = 30;
            game.player.data.insert("blink".into(), 30.0);
            game.player.vel.x = if game.player.pos.x < e.pos.x {
                -6.0
            } else {
                6.0
            };
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
            game.player.state = PlayerState::Stunned;
            game.player.data.insert("wall_hurt".into(), 1.0);
            game.player.data.insert("bounced".into(), 0.0);
            super::drop_from_throw(game);
            e.sprite = "sYetiThrowL".into();
            return;
        }
    }
    match e.state {
        0 => {
            if floor
                && (game
                    .world
                    .solid_point_except(Vec2::new(e.pos.x - 1.0, e.pos.y), e.id)
                    || game
                        .world
                        .solid_point_except(Vec2::new(e.pos.x + 16.0, e.pos.y), e.id))
            {
                e.vel.y = -6.0;
                e.vel.x = f64::from(e.facing);
                e.data.insert("counter".into(), e.value("counter") - 10.0);
            }
            if floor && e.value("counter") > 0.0 {
                counter(e, "counter");
            }
            if e.value("counter") < 1.0 {
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
    // Source Yeti sight rays alert cavemen and hawkmen via oEnemySight. The
    // source sight collision does not place the Yeti itself in ATTACK.
    if e.value("sight") == 0.0 && matches!(e.state, 0 | 1) {
        let id = game.world.spawn("oEnemySight", e.pos);
        if let Some(sight) = game.world.entity_mut(id) {
            sight.vel.x = f64::from(e.facing) * 10.0;
            sight.data.insert("owner".into(), e.id as f64);
        }
        e.data.insert("sight".into(), 5.0);
    } else if matches!(e.state, 0 | 1) {
        counter(e, "sight");
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

fn settle_yeti(e: &mut Entity, wall: bool, floor: bool, impact_y: f64) {
    // scrCheckCollisions narrows the body after a knockdown and applies ground
    // friction and a half-height bounce. Capture impact speed before move_body
    // zeroes its resolved vertical collision velocity.
    for (key, value) in [
        ("collision_left", 2.0),
        ("collision_top", 6.0),
        ("collision_right", 14.0),
        ("collision_bottom", 16.0),
    ] {
        e.data.insert(key.into(), value);
    }
    if wall {
        e.vel.x *= -0.5;
    }
    if floor {
        e.vel.y = if impact_y > 1.0 {
            -impact_y * 0.5
        } else if impact_y.abs() < 1.0 {
            0.0
        } else {
            impact_y
        };
        e.vel.x = if e.vel.x.abs() < 0.1 {
            0.0
        } else {
            e.vel.x * 0.3
        };
        e.data.insert("bounced".into(), 1.0);
    }
    e.vel.x = (e.vel.x.abs() - 0.1).max(0.0) * e.vel.x.signum();
    if e.vel.x.abs() < 0.5 {
        e.vel.x = 0.0;
    }
}

fn alien(game: &mut Game, e: &mut Entity) {
    let next_y = e.vel.y + 0.6;
    let (wall, floor) = move_body(&game.world, e, 0.6);
    // Alien Step does not apply the inherited yVelLimit and moveTo itself
    // does not clear velocity when it resolves a ceiling collision.
    e.vel.y = if floor && e.state != 98 { 0.0 } else { next_y };
    if game
        .world
        .solid_point_except(Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0), e.id)
    {
        e.health = 0;
    }
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
    let next_y = (e.vel.y + super::inherited::gravity(e, 0.6)).min(10.0);
    let (wall, floor) = move_body(&game.world, e, 0.6);
    e.vel.y = if floor && e.state != 98 { 0.0 } else { next_y };
    if game
        .world
        .solid_point_except(Vec2::new(e.pos.x + 16.0, e.pos.y + 16.0), e.id)
    {
        e.health = 0;
    }
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
    counter(e, "attack_timer");
    if matches!(e.state, 0 | 1) {
        counter(e, "counter");
    }
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
            let turn_dy = (game.player.pos.y - (e.pos.y + 32.0)).abs();
            if wall
                || (dx * f64::from(e.facing) < 0.0 && turn_dy < 16.0 && e.value("counter") == 0.0)
            {
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
        }
        3 => {
            e.vel.x = 0.0;
            e.data.insert("attack_timer".into(), 100.0);
            e.data.insert("animation_speed".into(), 0.5);
            let frame = e.animation;
            if (7.0..=12.0).contains(&frame) {
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
                            && Vec2::new(pos.x - 8.0, pos.y - 8.0).distance(e.pos) < 96.0
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
    if super::solid_mask(&game.world, e.bounds(), e.id) {
        e.pos.y -= 2.0;
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
    let previous_y = e.vel.y;
    let (wall, _) = move_body(&game.world, e, 0.0);
    e.vel.y = previous_y;
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
    // Only SEARCH checks the character's swimming/dead flags. Its moveTo
    // above still uses the previous velocity while this branch is skipped.
    if e.state != 1
        && (!super::player_alive(game)
            || game.player.data.get("swimming").copied().unwrap_or(0.0) != 0.0)
    {
        phase(e, 0.5);
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
    // Boss Step checks crushing before its moveTo, unlike the walkers.
    if game
        .world
        .solid_point_except(Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0), e.id)
    {
        e.health = 0;
    }
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
    e.vel.x = (e.vel.x.abs() - 0.1).max(0.0) * e.vel.x.signum();
    if e.vel.x.abs() < 0.5 || e.state == 0 {
        e.vel.x = 0.0;
    }
    // Boss Step applies gravity before moveTo (the walkers do it after).
    e.vel.y = (e.vel.y + 0.6).min(10.0);
    let b = e.bounds();
    let feet = Rect::new(b.x, b.y + b.h, b.w, 1.0);
    if super::solid_mask(&game.world, feet, e.id) {
        e.vel.y = 0.0;
    }
    move_body(&game.world, e, 0.0);
    if super::solid_mask(&game.world, e.bounds(), e.id) {
        e.pos.y -= 2.0;
    }
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
    let recovered = e.value("psychic_recover") == 0.0;
    counter(e, "psychic_recover");
    if recovered
        && distance_to_player(game, e) < 96.0
        && super::player_alive(game)
        && game.player.stun == 0
        && game.player.state != PlayerState::Stunned
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
    if e.sprite != "sAlienBossHurt" {
        e.data.insert("animation_speed".into(), 0.25);
        if e.value("boss_animation_advanced") == 0.0 {
            // sAlienBoss has four frames. Keeping its current image bounded
            // preserves the image index when oItem changes the hurt sprite.
            e.animation = phase(e, 0.25).rem_euclid(4.0);
            e.data.insert("phase".into(), e.animation);
        }
    }
}

fn eject(game: &mut Game, e: &mut Entity) {
    if e.value("drift_unset") > 0.0 {
        e.facing = if game.rng.range(2) == 0 { -1 } else { 1 };
        e.data.insert("drift_unset".into(), 0.0);
    }
    // Eject Step uses raw x/y motion rather than the platform moveTo solver.
    e.pos.x += e.vel.x;
    e.pos.y += e.vel.y;
    if e.state == 0 {
        e.vel.x = 0.0;
        e.vel.y += 0.5;
        let top = Rect::new(e.pos.x - 4.0, e.pos.y - 5.0, 8.0, 1.0);
        if e.vel.y >= 0.0 || super::solid_mask(&game.world, top, e.id) {
            enter(e, 2);
            e.vel.y = 0.0;
            e.sprite = "sAlienDeploy".into();
        }
    } else {
        e.vel.x = 0.0;
        e.vel.y = 2.0;
        if game
            .world
            .solid_point_except(Vec2::new(e.pos.x, e.pos.y + 6.0), e.id)
        {
            game.world
                .spawn("oAlien", Vec2::new(e.pos.x - 8.0, e.pos.y - 12.0));
            e.alive = false;
            return;
        }
        e.vel.x = f64::from(e.facing);
        if game
            .world
            .solid_point_except(Vec2::new(e.pos.x + e.vel.x * 8.0, e.pos.y), e.id)
        {
            e.facing = 0;
        }
        if phase(e, 0.6) >= 6.0 {
            e.sprite = "sAlienParachute".into();
        }
    }
    if game.world.entities.iter().any(|web| {
        web.alive && web.object == "oWeb" && web.sprite_bounds().intersects(e.sprite_bounds())
    }) {
        game.world
            .spawn("oAlien", Vec2::new(e.pos.x - 8.0, e.pos.y - 12.0));
        e.alive = false;
        return;
    }
    if e.sprite_bounds().intersects(game.player.sprite_bounds()) {
        if game.player.health > 0
            && matches!(
                game.player.state,
                PlayerState::Jumping | PlayerState::Falling
            )
            && game.player.pos.y < e.pos.y - 3.0
            && game.player.data.get("swimming").copied().unwrap_or(0.0) == 0.0
        {
            game.player.vel.y = -6.0 - 0.2 * game.player.vel.y;
            game.player.data.insert("fall_timer".into(), 0.0);
            e.health = 0;
            e.alive = false;
            game.sound("sndHit", e.pos);
            return;
        } else if game.player.invincible == 0 {
            hurt_player(game, 1);
            game.player.invincible = 30;
            game.player.data.insert("blink".into(), 30.0);
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
    if outside_room(game, e) {
        e.alive = false;
    }
}

fn outside_room(game: &Game, e: &Entity) -> bool {
    let b = e.sprite_bounds();
    b.x + b.w < 0.0
        || b.y + b.h < 0.0
        || b.x > game.world.width as f64 * 16.0
        || b.y > game.world.height as f64 * 16.0
}

fn crash(game: &mut Game, e: &mut Entity) {
    e.pos.x += e.vel.x;
    e.pos.y += e.vel.y;
    if e.vel.y < 6.0 {
        e.vel.y += 0.6;
    }
    if game.world.collides_except(e.sprite_bounds(), e.id)
        || game.world.entities.iter().any(|other| {
            other.alive
                && other.id != e.id
                && crate::catalog::is_a(&other.object, "oEnemy")
                && other.sprite_bounds().intersects(e.sprite_bounds())
        })
    {
        game.world
            .spawn("oExplosion", Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0));
        game.sound("sndExplosion", e.pos);
        e.alive = false;
    }
}

/// These two hazards explicitly subtract life in oPlayer1 Step. They bypass
/// contact invincibility and do not grant a new invincibility interval.
fn hurt_player(game: &mut Game, amount: i32) {
    if game.player.health > 0 {
        game.player.health -= amount;
        game.events.push(GameEvent::Damage { amount });
    }
    game.sound("sndHurt", game.player.pos);
}

fn hazard_hits_player(game: &Game, e: &Entity) -> bool {
    e.sprite_bounds().intersects(Rect::new(
        game.player.pos.x - 8.0,
        game.player.pos.y - 8.0,
        16.0,
        16.0,
    ))
}

fn laser(game: &mut Game, e: &mut Entity) {
    e.pos.y += e.vel.y;
    e.vel.y += 0.6;
    if e.vel.y > 4.0 {
        e.vel.y = 0.0;
    }
    // The Collision with oSolid event uses the laser's full mask, including
    // tile edges and moving solid instances, rather than just its origin.
    let body = e.sprite_bounds();
    let x0 = (body.x / 16.0).floor() as i32;
    let y0 = (body.y / 16.0).floor() as i32;
    let x1 = ((body.x + body.w - 0.001) / 16.0).floor() as i32;
    let y1 = ((body.y + body.h - 0.001) / 16.0).floor() as i32;
    for y in y0..=y1 {
        for x in x0..=x1 {
            if let Some(tile) = game.world.tile(x, y) {
                if tile.is_solid() {
                    if tile.is_destructible() {
                        game.world.set_tile(x as usize, y as usize, "");
                    }
                    game.world.spawn("oLaserExplode", e.pos);
                    e.alive = false;
                    return;
                }
            }
        }
    }
    let hit = game.world.entities.iter().position(|solid| {
        solid.id != e.id && solid.alive && solid.solid && solid.sprite_bounds().intersects(body)
    });
    if let Some(index) = hit {
        let solid = &mut game.world.entities[index];
        if solid.value("invincible") == 0.0 && crate::Tile::new(&solid.object).is_destructible() {
            solid.alive = false;
        }
        game.world.spawn("oLaserExplode", e.pos);
        e.alive = false;
        return;
    }
    if body.x + body.w < 0.0
        || body.y + body.h < 0.0
        || body.x > game.world.width as f64 * 16.0
        || body.y > game.world.height as f64 * 16.0
    {
        e.alive = false;
        return;
    }
    if hazard_hits_player(game, e) {
        hurt_player(game, 3);
        game.player.stun = 20;
        game.player.state = PlayerState::Stunned;
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
        return;
    }
    for enemy in &mut game.world.entities {
        if enemy.alive
            && enemy.object != "oUFO"
            && enemy.object != "oLaser"
            && (crate::catalog::is_a(&enemy.object, "oEnemy") || enemy.object == "oDamsel")
            && enemy.sprite_bounds().intersects(e.sprite_bounds())
            && enemy.value("invincible") == 0.0
        {
            enemy.health -= 3;
            enemy.vel = Vec2::new(-1.0, -6.0);
            if enemy.object == "oDamsel" {
                // The source's unqualified status assignment belongs to the
                // laser, not to the NPC receiving its velocity and damage.
                e.state = 2;
            }
            e.alive = false;
            break;
        }
    }
    if !e.alive {
        game.world.spawn("oLaserExplode", e.pos);
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
    if phase(e, 0.25) >= 17.0 || outside_room(game, e) {
        e.alive = false;
        return;
    }
    if hazard_hits_player(game, e)
        && game.player.stun == 0
        && game.player.state != PlayerState::Stunned
        && super::player_alive(game)
    {
        hurt_player(game, 1);
        game.player.stun = 40;
        game.player.state = PlayerState::Stunned;
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
            && enemy.sprite_bounds().intersects(e.sprite_bounds())
        {
            if enemy.value("invincible") == 0.0 {
                enemy.health -= 3;
                enemy.vel = Vec2::new(-1.0, -6.0);
                if enemy.object == "oDamsel" {
                    e.state = 2;
                }
            }
            if enemy.object == "oDamsel" {
                // Unlike the laser event, this destruction is outside the
                // source's invincibility guard.
                e.alive = false;
                break;
            }
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

    fn boss_item_room() -> Game {
        let mut g = Game::shell(5, RulesPreset::Classic);
        g.world = World::new(60, 30, Area::Ice, 9);
        g.world.name = "boss item animation fixture".into();
        g.player.pos = Vec2::new(40.0, 152.0);
        g.player.state = PlayerState::Standing;
        g.player.grounded = true;
        for x in 0..60 {
            g.world.set_tile(x, 10, "oBlock");
        }
        g
    }

    fn throw_boss_rock(g: &mut Game, x: f64) {
        let id = g.world.spawn("oRock", Vec2::new(x - 1.0, 140.0));
        g.world.entity_mut(id).unwrap().vel.x = 5.0;
        g.step(&crate::InputFrame::default());
        // The first rock need not remain overlapping during recovery.
        if let Some(rock) = g.world.entity_mut(id) {
            rock.alive = false;
        }
    }

    #[test]
    fn actual_step_laser_and_psychic_hit_damsel_without_changing_her_state() {
        for object in ["oLaser", "oPsychicWave"] {
            let mut g = boss_item_room();
            let damsel = g.world.spawn("oDamsel", Vec2::new(136.0, 136.0));
            let projectile = g.world.spawn(object, Vec2::new(138.0, 136.0));
            g.step(&crate::InputFrame::default());
            let target = g.world.entity(damsel).unwrap();
            assert_eq!(target.health, 1, "{object}");
            assert_eq!(target.state, 0, "{object}");
            assert_eq!(target.vel, Vec2::new(-1.0, -6.0), "{object}");
            assert!(g.world.entity(projectile).is_none(), "{object}");
        }
    }

    #[test]
    fn actual_step_rescued_damsel_blocks_damage_and_consumes_only_psychic_wave() {
        for (object, consumed) in [("oLaser", false), ("oPsychicWave", true)] {
            let mut g = boss_item_room();
            let damsel = g.world.spawn("oDamsel", Vec2::new(136.0, 136.0));
            g.world.spawn("oExit", Vec2::new(128.0, 128.0));
            // Let the normal rescue enter its visible exit frames before the
            // incoming projectile arrives. No NPC flags or state are injected.
            for _ in 0..7 {
                g.step(&crate::InputFrame::default());
            }
            let projectile = g.world.spawn(object, Vec2::new(138.0, 136.0));
            assert!(
                g.world
                    .entity(projectile)
                    .unwrap()
                    .sprite_bounds()
                    .intersects(g.world.entity(damsel).unwrap().sprite_bounds()),
                "{object} must overlap the exit sprite"
            );
            g.step(&crate::InputFrame::default());
            let target = g.world.entity(damsel).unwrap();
            assert_eq!(target.state, 4, "{object}");
            assert_eq!(target.health, 4, "{object}");
            assert_eq!(g.world.entity(projectile).is_none(), consumed, "{object}");
            for _ in 0..40 {
                g.step(&crate::InputFrame::default());
            }
            assert!(g.world.entity(damsel).is_none(), "{object}");
            assert_eq!(g.data["pending_damsel_kisses"], 1.0);
        }
    }

    #[test]
    fn actual_step_ufo_search_does_not_start_chasing_a_swimming_player() {
        let mut g = boss_item_room();
        for x in 1..5 {
            for y in 7..10 {
                g.world
                    .spawn("oWaterSwim", Vec2::new((x * 16) as f64, (y * 16) as f64));
            }
        }
        let id = g.world.spawn("oUFO", Vec2::new(200.0, 70.0));
        for _ in 0..3 {
            g.step(&crate::InputFrame::default());
            assert_eq!(g.player.data["swimming"], 1.0);
            let ufo = g.world.entity(id).unwrap();
            assert_eq!(ufo.state, 0);
            assert_eq!(ufo.vel, Vec2::default());
            assert_eq!(ufo.pos, Vec2::new(200.0, 70.0));
        }
    }

    #[test]
    fn actual_step_ufo_search_does_not_start_chasing_after_natural_player_death() {
        let mut g = boss_item_room();
        g.world.spawn("oGhost", Vec2::new(32.0, 144.0));
        g.step(&crate::InputFrame::default());
        assert_eq!(g.player.state, PlayerState::Dead);
        let id = g.world.spawn("oUFO", Vec2::new(200.0, 70.0));
        for _ in 0..3 {
            g.step(&crate::InputFrame::default());
            let ufo = g.world.entity(id).unwrap();
            assert_eq!(ufo.state, 0);
            assert_eq!(ufo.vel, Vec2::default());
            assert_eq!(ufo.pos, Vec2::new(200.0, 70.0));
        }
    }

    #[test]
    fn actual_step_boss_item_hurt_finishes_and_accepts_another_hit() {
        let mut g = boss_item_room();
        let id = g.world.spawn("oAlienBoss", Vec2::new(128.0, 128.0));
        throw_boss_rock(&mut g, 128.0);
        assert_eq!(g.world.entity(id).unwrap().health, 9);
        assert_eq!(g.world.entity(id).unwrap().sprite, "sAlienBossHurt");
        for _ in 0..5 {
            g.step(&crate::InputFrame::default());
        }
        let boss = g.world.entity(id).unwrap();
        assert_eq!(boss.sprite, "sAlienBossHurt");
        assert!((boss.animation - 4.25).abs() < 1e-9);
        assert_eq!(boss.value("animation_speed"), 0.8);
        g.step(&crate::InputFrame::default());
        let boss = g.world.entity(id).unwrap();
        assert_eq!(boss.sprite, "sAlienBoss");
        assert!((boss.animation - 0.05).abs() < 1e-9);
        assert_eq!(boss.value("animation_speed"), 0.25);
        throw_boss_rock(&mut g, 128.0);
        assert_eq!(g.world.entity(id).unwrap().health, 8);
    }

    #[test]
    fn actual_step_boss_late_item_hurt_retains_current_source_frame() {
        let mut g = boss_item_room();
        let id = g.world.spawn("oAlienBoss", Vec2::new(128.0, 128.0));
        for _ in 0..40 {
            g.step(&crate::InputFrame::default());
        }
        throw_boss_rock(&mut g, 128.0);
        assert!((g.world.entity(id).unwrap().animation - 2.25).abs() < 1e-9);
        for _ in 0..3 {
            g.step(&crate::InputFrame::default());
        }
        assert_eq!(g.world.entity(id).unwrap().sprite, "sAlienBossHurt");
        g.step(&crate::InputFrame::default());
        assert_eq!(g.world.entity(id).unwrap().sprite, "sAlienBoss");
    }

    #[test]
    fn actual_step_boss_hurt_animation_end_runs_outside_ai_viewport() {
        let mut g = boss_item_room();
        g.player.pos.x = 208.0;
        g.update_camera();
        assert_eq!(g.camera.x, 16.0);
        let left = crate::InputFrame {
            left: true,
            run: true,
            ..Default::default()
        };
        while g.player.pos.x > 152.0 {
            g.step(&left);
        }
        assert_eq!(g.camera.x, 16.0);
        // The rock's source Step gate is still active on the hit frame. Running
        // farther left then scrolls the boss beyond its narrower AI guard.
        let id = g.world.spawn("oAlienBoss", Vec2::new(335.0, 128.0));
        throw_boss_rock(&mut g, 335.0);
        assert_eq!(g.world.entity(id).unwrap().sprite, "sAlienBossHurt");
        for _ in 0..7 {
            g.step(&left);
        }
        let boss = g.world.entity(id).unwrap();
        assert_eq!(boss.sprite, "sAlienBoss");
        assert_eq!(boss.pos, Vec2::new(335.0, 128.0));
        assert!(g.camera.x < 11.0);
        // Animation End changes the sprite; the skipped Step has not restored
        // its normal image speed yet.
        assert_eq!(boss.value("animation_speed"), 0.8);
        let frozen_counter = boss.value("psychic_recover");
        g.step(&left);
        assert_eq!(
            g.world.entity(id).unwrap().value("psychic_recover"),
            frozen_counter
        );
    }

    #[test]
    fn actual_step_king_turn_finishes_while_falling_player_scrolls_away() {
        let mut g = boss_item_room();
        for x in 0..8 {
            g.world.set_tile(x, 10, "");
            g.world.set_tile(x, 21, "oBlock");
        }
        g.player.state = PlayerState::Falling;
        g.player.grounded = false;
        g.player.vel.y = 10.0;
        let id = g.world.spawn("oYetiKing", Vec2::new(128.0, 128.0));
        g.step(&crate::InputFrame::default());
        g.step(&crate::InputFrame::default());
        assert_eq!(g.world.entity(id).unwrap().state, 2);
        assert_eq!(g.world.entity(id).unwrap().sprite, "sYetiKingTurnL");
        for _ in 0..48 {
            g.step(&crate::InputFrame::default());
        }
        let king = g.world.entity(id).unwrap();
        assert_eq!(g.camera.y, 184.0);
        assert_eq!(king.pos, Vec2::new(131.0, 128.0));
        assert_eq!(king.state, 1);
        assert_eq!(king.facing, -1);
        // Animation End changes facing/state even when Step is skipped. Its
        // counter and attack timer remain frozen by the source viewport guard.
        assert_eq!(king.value("counter"), 30.0);
        assert_eq!(king.value("attack_timer"), 0.0);
    }

    #[test]
    fn actual_step_king_yell_finishes_offscreen_without_running_ai_timers() {
        let mut g = boss_item_room();
        for x in 0..8 {
            g.world.set_tile(x, 10, "");
            g.world.set_tile(x, 21, "oBlock");
        }
        // The king remains supported while the player's narrower body falls
        // alongside this tile and scrolls the camera down.
        g.world.set_tile(1, 10, "oBlock");
        g.player.state = PlayerState::Falling;
        g.player.grounded = false;
        g.player.vel.y = 10.0;
        let id = g.world.spawn("oYetiKing", Vec2::new(16.0, 128.0));
        g.step(&crate::InputFrame::default());
        g.step(&crate::InputFrame::default());
        assert_eq!(g.world.entity(id).unwrap().state, 3);
        for _ in 0..48 {
            g.step(&crate::InputFrame::default());
        }
        let king = g.world.entity(id).unwrap();
        assert_eq!(g.camera.y, 184.0);
        assert_eq!(king.pos, Vec2::new(19.0, 128.0));
        assert_eq!(king.state, 0);
        assert_eq!(king.value("counter"), 30.0);
        assert_eq!(king.value("attack_timer"), 100.0);
        assert_eq!(king.value("animation_speed"), 0.25);
    }

    #[test]
    fn integrated_yeti_stomps_before_grabbing_even_while_rising() {
        let mut g = game();
        let id = g.world.spawn("oYeti", Vec2::new(64.0, 64.0));
        g.world.entity_mut(id).unwrap().vel = Vec2::default();
        g.player.pos = Vec2::new(72.0, 65.0);
        g.player.state = PlayerState::Jumping;
        g.player.vel.y = -1.0;
        g.player.data.insert("fall_timer".into(), 16.0);
        crate::enemies::tick(&mut g);
        let yeti = g.world.entity(id).unwrap();
        assert_eq!(yeti.health, 4);
        assert_eq!(yeti.state, 98);
        assert_eq!(g.player.stun, 0);
        assert_eq!(g.player.pos, Vec2::new(72.0, 65.0));
    }

    #[test]
    fn integrated_yeti_ignores_outer_collision_without_center_overlap() {
        let mut g = game();
        let id = g.world.spawn("oYeti", Vec2::new(64.0, 64.0));
        g.world.entity_mut(id).unwrap().vel = Vec2::default();
        g.player.pos = Vec2::new(85.0, 72.0);
        g.player.state = PlayerState::Standing;
        crate::enemies::tick(&mut g);
        assert_eq!(g.player.health, 4);
        assert_eq!(g.player.stun, 0);
        assert_ne!(g.world.entity(id).unwrap().state, 3);
    }

    #[test]
    fn integrated_laser_ignores_contact_invincibility_and_uses_hazard_rectangle() {
        let mut g = game();
        g.player.invincible = 20;
        let id = g.world.spawn("oLaser", Vec2::new(110.0, 100.0));
        crate::enemies::tick(&mut g);
        assert_eq!(g.player.health, 1);
        assert_eq!(g.player.invincible, 20);
        assert_eq!(g.player.stun, 20);
        assert_eq!(g.player.vel, Vec2::new(-2.0, -4.0));
        assert!(g.world.entity(id).is_none());
    }

    #[test]
    fn integrated_psychic_wave_ignores_invincibility_but_respects_stun() {
        let mut g = game();
        g.player.invincible = 20;
        g.world.spawn("oPsychicWave", g.player.pos);
        crate::enemies::tick(&mut g);
        assert_eq!(g.player.health, 3);
        assert_eq!(g.player.stun, 40);
        crate::enemies::tick(&mut g);
        assert_eq!(g.player.health, 3);
    }

    #[test]
    fn integrated_laser_collides_at_tile_mask_edge() {
        let mut g = game();
        g.player.pos = Vec2::new(200.0, 200.0);
        g.world.set_tile(4, 4, "oIce");
        let id = g.world.spawn("oLaser", Vec2::new(62.0, 72.0));
        crate::enemies::tick(&mut g);
        assert!(g.world.entity(id).is_none());
        assert!(!g.world.tile(4, 4).unwrap().is_solid());
    }

    #[test]
    fn integrated_laser_hits_moving_source_solids() {
        let mut g = game();
        g.player.pos = Vec2::new(200.0, 200.0);
        let block = g.world.spawn("oPushBlock", Vec2::new(64.0, 64.0));
        let laser = g.world.spawn("oLaser", Vec2::new(62.0, 72.0));
        crate::enemies::tick(&mut g);
        assert!(g.world.entity(block).is_none());
        assert!(g.world.entity(laser).is_none());
    }

    #[test]
    fn integrated_yeti_low_ceiling_hit_grants_thirty_ticks_without_stun() {
        let mut g = game();
        g.world.set_tile(4, 3, "oBlock");
        let id = g.world.spawn("oYeti", Vec2::new(64.0, 64.0));
        g.world.entity_mut(id).unwrap().vel = Vec2::default();
        g.player.pos = Vec2::new(72.0, 72.0);
        g.player.state = PlayerState::Standing;
        crate::enemies::tick(&mut g);
        assert_eq!(g.player.health, 3);
        assert_eq!(g.player.invincible, 30);
        assert_eq!(g.player.stun, 0);
        assert_eq!(g.player.vel.x, 6.0);
    }

    #[test]
    fn integrated_yeti_sight_emits_source_rays_without_synthetic_charge() {
        let mut g = game();
        g.player.pos = Vec2::new(140.0, 72.0);
        let id = g.world.spawn("oYeti", Vec2::new(64.0, 64.0));
        let yeti = g.world.entity_mut(id).unwrap();
        yeti.state = 1;
        yeti.vel = Vec2::default();
        crate::enemies::tick(&mut g);
        assert_ne!(g.world.entity(id).unwrap().state, 2);
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == "oEnemySight")
                .count(),
            1
        );
    }

    #[test]
    fn integrated_yeti_corpse_bounces_and_loses_horizontal_speed() {
        let mut g = game();
        g.player.pos = Vec2::new(200.0, 200.0);
        for x in 0..20 {
            g.world.set_tile(x, 5, "oBlock");
        }
        let id = g.world.spawn("oYeti", Vec2::new(64.0, 64.0));
        let yeti = g.world.entity_mut(id).unwrap();
        yeti.health = 0;
        yeti.vel = Vec2::new(4.0, 3.0);
        crate::enemies::tick(&mut g);
        let yeti = g.world.entity(id).unwrap();
        assert!((yeti.vel.x - 1.1).abs() < 1e-9);
        assert!((yeti.vel.y + 1.8).abs() < 1e-9);
        assert_eq!(yeti.bounds(), Rect::new(70.0, 70.0, 12.0, 10.0));
    }

    #[test]
    fn integrated_ejected_alien_contact_uses_source_invincibility_without_stun() {
        let mut g = game();
        let id = g.world.spawn("oAlienEject", g.player.pos);
        let alien = g.world.entity_mut(id).unwrap();
        alien.state = 2;
        alien.vel = Vec2::default();
        crate::enemies::tick(&mut g);
        assert_eq!(g.player.health, 3);
        assert_eq!(g.player.invincible, 30);
        assert_eq!(g.player.stun, 0);
    }

    #[test]
    fn integrated_ejected_alien_converts_on_source_web_collision() {
        let mut g = game();
        g.player.pos = Vec2::new(200.0, 200.0);
        let id = g.world.spawn("oAlienEject", Vec2::new(100.0, 100.0));
        let alien = g.world.entity_mut(id).unwrap();
        alien.state = 2;
        alien.vel = Vec2::default();
        g.world.spawn("oWeb", Vec2::new(96.0, 96.0));
        crate::enemies::tick(&mut g);
        assert!(g.world.entity(id).is_none());
        let alien = g
            .world
            .entities
            .iter()
            .find(|e| e.alive && e.object == "oAlien")
            .unwrap();
        assert_eq!(alien.pos, Vec2::new(92.0, 88.0));
        assert_eq!(g.data.get("kills"), None);
    }

    #[test]
    fn integrated_boss_crush_uses_source_point_instead_of_body_center() {
        let mut g = game();
        g.player.pos = Vec2::new(200.0, 200.0);
        g.world.set_tile(4, 4, "oBlock");
        let id = g.world.spawn("oAlienBoss", Vec2::new(52.0, 52.0));
        crate::enemies::tick(&mut g);
        assert_eq!(g.world.entity(id).unwrap().health, 10);
    }

    #[test]
    fn integrated_alien_fall_velocity_has_no_source_terminal_clamp() {
        let mut g = game();
        g.player.pos = Vec2::new(200.0, 200.0);
        let id = g.world.spawn("oAlien", Vec2::new(50.0, 50.0));
        g.world.entity_mut(id).unwrap().vel.y = 12.0;
        crate::enemies::tick(&mut g);
        assert!((g.world.entity(id).unwrap().vel.y - 12.6).abs() < 1e-9);
    }

    #[test]
    fn integrated_boss_applies_gravity_before_source_move_to() {
        let mut g = game();
        g.level_tick = 1; // source room clock 2 emits a fractional 0.6 step
        g.player.pos = Vec2::new(200.0, 200.0);
        let id = g.world.spawn("oAlienBoss", Vec2::new(50.0, 50.0));
        crate::enemies::tick(&mut g);
        assert_eq!(g.world.entity(id).unwrap().pos.y, 51.0);
    }

    #[test]
    fn integrated_ufo_preserves_search_velocity_when_blocked_by_ceiling() {
        let mut g = game();
        g.world.set_tile(3, 2, "oBlock");
        let id = g.world.spawn("oUFO", Vec2::new(50.0, 46.0));
        g.world.entity_mut(id).unwrap().vel.y = -2.0;
        crate::enemies::tick(&mut g);
        let ufo = g.world.entity(id).unwrap();
        assert_eq!(ufo.health, 1);
        assert_eq!(ufo.pos.y, 46.0);
        assert_eq!(ufo.state, 1);
    }
}
