//! Temple enemies translated from Classic 1.1's individual object events.
//! Instances retain the original GameMaker origins, including top-left boss origins.
use super::{center, move_body};
use crate::model::{Entity, Game, GameEvent, Rect, Tile, Vec2, World};

pub fn matches(object: &str) -> bool {
    matches!(
        object,
        "oHawkman"
            | "oHawkmanWorship"
            | "oTombLord"
            | "oFly"
            | "oOlmec"
            | "oOlmecSlam"
            | "oOlmecDebris"
            | "oYellowBall"
            | "oMagmaMan"
            | "oMagma"
            | "oMagmaTrail"
    )
}

pub fn init(e: &mut Entity) {
    match e.object.as_str() {
        "oHawkman" | "oHawkmanWorship" => {
            e.health = 4;
            e.sprite = "sHawkLeft".into();
            e.data.insert("contact_damage".into(), 0.0);
        }
        "oTombLord" => {
            e.health = 20;
            e.sprite = "sTombLordRight".into();
            e.data.insert("contact_damage".into(), 2.0);
        }
        "oOlmec" => {
            e.health = 1;
            e.solid = true;
            e.state = -1;
            e.timer = 100;
            e.sprite = "sOlmecStart1".into();
            e.data.insert("intro_stage".into(), 0.0);
            e.data.insert("invincible".into(), 1.0);
            e.data.insert("no_stomp".into(), 1.0);
            e.data.insert("no_whip".into(), 1.0);
        }
        "oOlmecSlam" => {
            e.timer = 1;
            e.sprite = String::new();
        }
        "oMagmaMan" | "oMagma" => {
            e.health = 200;
            e.data.insert("invincible".into(), 1.0);
            e.data.insert("no_stomp".into(), 1.0);
            e.data.insert("no_whip".into(), 1.0);
            e.data.insert("contact_damage".into(), 2.0);
        }
        "oMagmaTrail" => {
            e.timer = 13;
        }
        "oFly" => {
            e.sprite = "sFlyRight".into();
        }
        "oYellowBall" => {
            e.data.insert("launch".into(), 1.0);
        }
        _ => {}
    }
}

pub fn tick(game: &mut Game, e: &mut Entity) {
    match e.object.as_str() {
        "oHawkman" => hawk(game, e),
        "oHawkmanWorship" => {}
        "oTombLord" => tomb_lord(game, e),
        "oFly" => fly(game, e),
        "oOlmec" => olmec(game, e),
        "oOlmecSlam" => slam(game, e),
        "oOlmecDebris" => debris(game, e),
        "oYellowBall" => yellow_ball(game, e),
        "oMagmaMan" => magma_man(game, e),
        "oMagma" => magma(game, e),
        "oMagmaTrail" => {
            e.animation += 0.4;
            if e.timer == 0 {
                e.alive = false;
            } else {
                e.timer -= 1;
            }
        }
        _ => {}
    }
}

fn ground(world: &World, e: &Entity) -> bool {
    let b = e.bounds();
    world.collides(Rect::new(b.x, b.y + b.h, b.w, 1.0))
}
fn countdown(e: &mut Entity, key: &str) {
    let v = e.value(key);
    if v > 0.0 {
        e.data.insert(key.into(), v - 1.0);
    }
}
fn side(e: &Entity) -> &'static str {
    if e.facing < 0 { "L" } else { "R" }
}
fn spawn(game: &mut Game, object: &str, pos: Vec2) -> u64 {
    let id = game.world.spawn(object, pos);
    if let Some(e) = game.world.entity_mut(id) {
        super::init_entity(e);
    }
    id
}
fn line_of_sight(world: &World, a: Vec2, b: Vec2) -> bool {
    let n = (a.distance(b) / 4.0).ceil().max(1.0) as usize;
    (1..n).all(|i| {
        !world.solid_at(
            a.lerp(b, i as f64 / n as f64).x,
            a.lerp(b, i as f64 / n as f64).y,
        )
    })
}

fn hawk(game: &mut Game, e: &mut Entity) {
    e.animation += if e.state == 2 { 1.0 } else { 0.5 };
    let (wall, landed) = move_body(&game.world, e, 0.6);
    let grounded = landed || ground(&game.world, e);
    if e.state >= 98 {
        if grounded {
            e.vel.x *= 0.8;
            if e.timer > 0 {
                e.timer -= 1;
            } else if e.health > 0 {
                e.state = 0;
            }
        }
        e.sprite = if e.health <= 0 {
            "sHawkDeadL"
        } else {
            "sHawkStunL"
        }
        .into();
        return;
    }
    countdown(e, "sight");
    if e.value("sight") == 0.0 {
        let d = game.player.pos.x - center(e).x;
        if d * e.facing as f64 > 0.0
            && d.abs() < 160.0
            && (game.player.pos.y - center(e).y).abs() < 12.0
            && line_of_sight(&game.world, center(e), game.player.pos)
        {
            e.state = 2;
        }
        e.data.insert("sight".into(), 5.0);
    }
    match e.state {
        0 => {
            if grounded && wall {
                e.vel.y = -6.0;
                e.vel.x = e.facing as f64;
            }
            if grounded && e.timer > 0 {
                e.timer -= 1;
            }
            if e.timer == 0 {
                e.facing = if game.rng.chance(1, 2) { -1 } else { 1 };
                e.state = 1;
            }
        }
        1 => {
            if wall {
                e.facing = -e.facing;
            }
            e.vel.x = 1.5 * e.facing as f64;
            if grounded
                && (!game
                    .world
                    .solid_at(center(e).x + 9.0 * e.facing as f64, e.pos.y + 17.0)
                    || game.rng.chance(1, 100))
            {
                e.state = 0;
                e.timer = 20 + game.rng.range(31);
                e.vel.x = 0.0;
            }
        }
        2 => {
            if wall {
                e.facing = -e.facing;
            }
            e.vel.x = 3.0 * e.facing as f64;
            if grounded && !game.world.solid_at(e.pos.x, e.pos.y - 16.0) {
                e.vel.y = -6.0;
            }
        }
        3 => {
            e.vel.x = 0.0;
            if e.timer > 0 {
                e.timer -= 1;
            } else {
                e.state = 0;
            }
        }
        _ => {}
    }
    e.sprite = if e.vel.x.abs() > 0.0 {
        "sHawkRunLeft"
    } else {
        "sHawkLeft"
    }
    .into();
    // Hawkman's source grab/throw is an explicit attack, rather than contact damage.
    if e.state < 3
        && e.bounds().intersects(game.player.bounds())
        && game.player.invincible == 0
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
            e.state = 3;
            e.timer = 14;
            e.vel.x = 0.0;
            e.sprite = "sHawkThrowL".into();
            game.player.vel = Vec2::new(6.0 * e.facing as f64, -6.0);
            game.player.stun = 20;
            game.player.data.insert("wall_hurt".into(), 1.0);
        }
    }
}

fn tomb_lord(game: &mut Game, e: &mut Entity) {
    countdown(e, "attack_cooldown");
    let (wall, _) = move_body(&game.world, e, 0.6);
    if e.health <= 0 {
        if game.world.level == 13 {
            spawn(game, "oSceptre", e.pos);
        }
        for _ in 0..4 {
            spawn(game, "oBone", e.pos);
        }
        e.alive = false;
        return;
    }
    if e.state >= 98 {
        e.state = 1;
    }
    match e.state {
        0 => {
            e.vel.x = 0.0;
            if e.timer > 0 {
                e.timer -= 1;
            } else {
                e.state = 1;
            }
        }
        1 => {
            let d = game.player.pos.x - center(e).x;
            if wall
                || (d * (e.facing as f64) < 0.0
                    && (game.player.pos.y - (e.pos.y + 32.0)).abs() < 16.0
                    && e.timer == 0)
            {
                e.state = 2;
                e.timer = 28;
                e.vel.x = 0.0;
                e.animation = 0.0;
                e.sprite = if e.facing < 0 {
                    "sTombLordTurnR"
                } else {
                    "sTombLordTurnL"
                }
                .into();
            } else if d * e.facing as f64 > 0.0
                && (game.player.pos.y - center(e).y).abs() < 32.0
                && e.value("attack_cooldown") == 0.0
            {
                e.state = 3;
                e.animation = 0.0;
                e.vel.x = 0.0;
                e.sprite = format!("sTombLordAttack{}", side(e));
                e.data.insert("attack_cooldown".into(), 100.0);
            } else {
                e.vel.x = e.facing as f64;
                e.sprite = format!("sTombLordWalk{}", side(e));
            }
        }
        2 => {
            e.vel.x = 0.0;
            if e.timer > 0 {
                e.timer -= 1;
            } else {
                e.facing = -e.facing;
                e.state = 1;
                e.timer = 30;
            }
        }
        3 => {
            e.vel.x = 0.0;
            if (7.0..=12.0).contains(&e.animation) {
                let pos = Vec2::new(
                    e.pos.x + if e.facing < 0 { 8.0 } else { 24.0 },
                    e.pos.y + 12.0 + game.rng.range(5) as f64,
                );
                let vx = (3 + game.rng.range(3)) as f64 * e.facing as f64;
                let vy = 0.5 - 3.0 * game.rng.unit();
                let id = spawn(game, "oFly", pos);
                if let Some(f) = game.world.entity_mut(id) {
                    f.vel = Vec2::new(vx, vy);
                }
            }
            if e.animation >= 14.0 {
                e.state = 0;
                e.timer = 30;
                e.sprite = format!("sTombLord{}", if e.facing < 0 { "Left" } else { "Right" });
            }
        }
        _ => {}
    }
    if e.state == 1 && e.timer > 0 {
        e.timer -= 1;
    }
    e.animation += if e.state == 3 { 0.5 } else { 0.25 };
}

fn fly(game: &mut Game, e: &mut Entity) {
    e.pos.x += e.vel.x;
    e.pos.y += e.vel.y;
    e.animation += 0.5;
    e.sprite = if e.vel.x < 0.0 {
        "sFlyLeft"
    } else {
        "sFlyRight"
    }
    .into();
    if game.world.collides(e.bounds()) {
        e.alive = false;
        game.sound("sndHit", e.pos);
        return;
    }
    if e.bounds().intersects(game.player.bounds()) {
        game.damage_player(2);
        game.player.vel = Vec2::new(e.vel.x, -4.0);
        e.alive = false;
        return;
    }
    if let Some(other) = game.world.entities.iter_mut().find(|o| {
        o.alive
            && o.id != e.id
            && o.object != "oTombLord"
            && o.object != "oFly"
            && super::is_enemy(&o.object)
            && o.bounds().intersects(e.bounds())
    }) {
        other.health -= 2;
        other.state = 98;
        other.timer = 20;
        other.vel = Vec2::new(e.vel.x, -4.0);
        e.alive = false;
    }
}

fn olmec_rect(e: &Entity) -> Rect {
    Rect::new(e.pos.x + 2.0, e.pos.y, 60.0, 64.0)
}
fn olmec_ground(world: &World, e: &Entity) -> bool {
    world.collides(Rect::new(e.pos.x + 2.0, e.pos.y + 64.0, 60.0, 1.0))
}
fn olmec_move(world: &World, e: &mut Entity) {
    for horizontal in [true, false] {
        let v = if horizontal { e.vel.x } else { e.vel.y };
        let n = v.abs().ceil().max(1.0) as usize;
        for _ in 0..n {
            let old = e.pos;
            if horizontal {
                e.pos.x += v / n as f64;
            } else {
                e.pos.y += v / n as f64;
            }
            if world.collides(olmec_rect(e)) {
                e.pos = old;
                if horizontal {
                    e.vel.x = 0.0;
                } else {
                    e.vel.y = 0.0;
                }
                break;
            }
        }
    }
}
fn olmec(game: &mut Game, e: &mut Entity) {
    let old = e.pos;
    olmec_move(&game.world, e);
    let grounded = olmec_ground(&game.world, e);
    let player_on_top = (game.player.pos.y + 8.0 - old.y).abs() < 3.0
        && (game.player.pos.x - (old.x + 32.0)).abs() < 34.0;
    if player_on_top && e.state != 6 {
        game.player.pos.x += e.pos.x - old.x;
        game.player.pos.y += e.pos.y - old.y;
    }
    let foot_y = ((e.pos.y + 64.0) / 16.0).floor() as i32;
    let head_y = ((e.pos.y - 2.0) / 16.0).floor() as i32;
    let tx = ((e.pos.x + 32.0) / 16.0).floor() as i32;
    let lava = |world: &World, y| world.tile(tx, y).is_some_and(|t| t.object == "oLava");
    if lava(&game.world, head_y) {
        e.alive = false;
        game.flags.insert("olmec_dead".into());
        game.events
            .push(GameEvent::Message("Olmec has fallen!".into()));
        return;
    }
    if lava(&game.world, foot_y) {
        e.state = 4;
    }
    if e.state != 5 && e.state != 4 {
        e.vel.y = (e.vel.y + 0.2).min(6.0);
    }
    match e.state {
        -1 => {
            if e.timer > 0 {
                e.timer -= 1;
            } else {
                let stage = e.value("intro_stage") as u32;
                e.data.insert("intro_stage".into(), (stage + 1) as f64);
                e.timer = 50;
                e.sprite = match stage {
                    0 => "sOlmecStart2",
                    1 => "sOlmecStart3",
                    _ => "sOlmec",
                }
                .into();
                for _ in 0..if stage < 2 { 6 } else { 12 } {
                    let p = Vec2::new(
                        e.pos.x + game.rng.unit() * 64.0,
                        e.pos.y + game.rng.unit() * 32.0,
                    );
                    spawn(game, "oOlmecDebris", p);
                }
                game.sound("sndThump", e.pos);
                if stage == 0 {
                    for hawk in game
                        .world
                        .entities
                        .iter_mut()
                        .filter(|h| h.object == "oHawkmanWorship")
                    {
                        hawk.object = "oHawkman".into();
                        hawk.health = 1;
                        hawk.state = 98;
                        hawk.timer = 300;
                        hawk.vel = Vec2::new(-3.0, -5.0);
                    }
                }
                if stage >= 3 {
                    e.state = 0;
                    e.timer = 100;
                }
            }
        }
        0 => {
            if grounded {
                e.vel.y = 0.0;
            }
            if e.timer > 0 {
                e.timer -= 1;
            } else {
                e.state = 1;
            }
        }
        1 => {
            if grounded {
                e.vel.y = -4.0;
            } else {
                e.state = 2;
                e.timer = 10;
                game.sound("sndBigJump", e.pos);
            }
        }
        2 => {
            if grounded {
                e.state = 0;
                e.timer = 40 + game.rng.range(61);
                e.vel = Vec2::default();
                game.sound("sndThump", e.pos);
            } else {
                if e.timer > 1 {
                    e.timer -= 1;
                } else if e.timer == 1 {
                    e.vel.x = if game.player.pos.x < e.pos.x {
                        -0.25
                    } else if game.player.pos.x > e.pos.x + 64.0 {
                        0.25
                    } else {
                        0.0
                    };
                    e.timer = 0;
                } else {
                    let a = if e.value("toggle") == 0.0 {
                        0.25
                    } else {
                        -0.25
                    };
                    e.vel.x += a * e.vel.x.signum();
                    if e.vel.x.abs() >= 2.0 {
                        e.data
                            .insert("toggle".into(), if a > 0.0 { 1.0 } else { 0.0 });
                    }
                }
                if game.player.pos.y > e.pos.y
                    && (game.player.pos.x - (e.pos.x + 32.0)).abs() < 32.0
                    && e.vel.x > -1.0
                {
                    e.state = 5;
                    e.timer = 20;
                    e.vel = Vec2::default();
                }
            }
        }
        4 => {
            e.vel = Vec2::new(0.0, 0.1);
            e.pos.y += 0.1;
            game.data.insert("shake".into(), 10.0);
        }
        5 => {
            if e.timer > 0 {
                e.timer -= 1;
            } else {
                e.state = 6;
                e.vel.y = 5.0;
                e.data.insert("slammed".into(), 0.0);
            }
        }
        6 => {
            if olmec_rect(e).intersects(game.player.bounds()) && !player_on_top {
                game.player.invincible = 0;
                game.damage_player(game.player.health.max(1));
            }
            if grounded {
                if e.value("slammed") == 0.0 {
                    spawn(game, "oOlmecSlam", Vec2::new(e.pos.x, e.pos.y + 64.0));
                    e.data.insert("slammed".into(), 1.0);
                    game.data.insert("shake".into(), 5.0);
                } else {
                    e.state = if game.rng.chance(1, 2) { 0 } else { 7 };
                    e.vel = Vec2::default();
                    e.timer = 60;
                }
            }
        }
        7 => {
            for _ in 0..3 {
                spawn(
                    game,
                    "oYellowBall",
                    Vec2::new(e.pos.x + 32.0, e.pos.y + 16.0),
                );
            }
            game.sound("sndPsychic", e.pos);
            e.state = 0;
        }
        _ => {}
    }
    e.animation += 0.4;
}

fn slam(game: &mut Game, e: &mut Entity) {
    // Source's 64-pixel-wide collision helper deletes only intersecting destructible terrain.
    let rect = Rect::new(e.pos.x, e.pos.y, 64.0, 16.0);
    let x0 = (rect.x / 16.0).floor() as i32;
    let y0 = (rect.y / 16.0).floor() as i32;
    for y in y0..=y0 {
        for x in x0..x0 + 4 {
            if x >= 0 && y >= 0 && game.world.tile(x, y).is_some_and(Tile::is_destructible) {
                game.world.set_tile(x as usize, y as usize, "");
            }
        }
    }
    game.sound("sndSlam", e.pos);
    e.alive = false;
}
fn debris(game: &mut Game, e: &mut Entity) {
    if e.value("started") == 0.0 {
        e.vel = Vec2::new(game.rng.unit() * 8.0 - 4.0, -1.0 - game.rng.unit() * 2.0);
        e.data.insert("started".into(), 1.0);
        e.sprite = format!("sOlmecDebris{}", 2 + game.rng.range(2));
    }
    let vy = e.vel.y;
    let (_, landed) = move_body(&game.world, e, 0.6);
    if landed {
        if vy > 1.0 {
            e.vel.y = -vy * 0.4;
            e.vel.x *= 0.3;
        } else {
            e.alive = false;
        }
    }
    e.animation += 0.3;
}
fn yellow_ball(game: &mut Game, e: &mut Entity) {
    if e.value("launch") != 0.0 {
        let direction = if game.rng.chance(1, 2) { -1.0 } else { 1.0 };
        e.vel = Vec2::new(
            (2 + game.rng.range(4)) as f64 * direction,
            -4.0 - game.rng.unit() * 3.0,
        );
        e.data.insert("launch".into(), 0.0);
    }
    let old = e.pos;
    e.pos.x += e.vel.x;
    e.pos.y += e.vel.y;
    e.vel.y = (e.vel.y + 0.15).min(6.0);
    let boss_overlaps = game
        .world
        .entities
        .iter()
        .any(|o| o.object == "oOlmec" && olmec_rect(o).intersects(e.bounds()));
    if game.world.collides(e.bounds()) && !boss_overlaps {
        let n = if game.rng.chance(1, 2) {
            game.rng.range(4)
        } else {
            game.rng.range(5)
        };
        spawn(
            game,
            ["oBat", "oSpider", "oSnake", "oFrog", "oFireFrog"][n as usize],
            old,
        );
        e.alive = false;
    }
    if e.pos.x < 0.0
        || e.pos.x > game.world.width as f64 * 16.0
        || e.pos.y > game.world.height as f64 * 16.0
    {
        e.alive = false;
    }
    e.animation += 0.5;
}

fn magma_man(game: &mut Game, e: &mut Entity) {
    e.health -= 1;
    if e.health <= 0 || game.world.solid_at(e.pos.x + 8.0, e.pos.y + 8.0) {
        if e.state != 99 {
            e.animation = 0.0;
        }
        e.state = 99;
        e.vel = Vec2::default();
        e.sprite = "sMagmaManDie".into();
        e.animation += 0.5;
        if e.animation >= 13.0 {
            e.alive = false;
        }
        return;
    }
    if !ground(&game.world, e) {
        let id = spawn(game, "oMagma", Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0));
        if let Some(m) = game.world.entity_mut(id) {
            m.health = e.health;
        }
        e.alive = false;
        return;
    }
    let (wall, _) = move_body(&game.world, e, 0.6);
    if e.state == 0 {
        if e.timer > 0 {
            e.timer -= 1;
        } else {
            e.facing = if game.rng.chance(1, 2) { -1 } else { 1 };
            e.state = 1;
            if game.rng.chance(1, 6) {
                let id = spawn(game, "oMagma", Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0));
                if let Some(m) = game.world.entity_mut(id) {
                    m.health = e.health;
                }
                e.alive = false;
                return;
            }
        }
    } else if e.state == 1 {
        if wall {
            e.facing = -e.facing;
        }
        e.vel.x = 1.5 * e.facing as f64;
        if !game
            .world
            .solid_at(center(e).x + 9.0 * e.facing as f64, e.pos.y + 17.0)
            || game.rng.chance(1, 100)
        {
            e.state = 0;
            e.timer = 20 + game.rng.range(31);
            e.vel.x = 0.0;
        }
    }
    e.sprite = if e.vel.x == 0.0 {
        "sMagmaManLeft"
    } else {
        "sMagmaManWalkL"
    }
    .into();
    e.animation += 0.5;
    ignite(game, e);
}
fn magma(game: &mut Game, e: &mut Entity) {
    if e.value("started") == 0.0 {
        e.vel = Vec2::new(
            game.rng.unit() * 4.0 - game.rng.unit() * 4.0,
            -1.0 - game.rng.unit() * 2.0,
        );
        e.data
            .insert("gravity".into(), (1 + game.rng.range(6)) as f64 * 0.1);
        e.data.insert("started".into(), 1.0);
    }
    let (_, landed) = move_body(&game.world, e, e.value("gravity"));
    if landed || ground(&game.world, e) {
        e.vel = Vec2::default();
        e.sprite = "sMagmaManCreate".into();
        let t = e.value("create_timer") + 1.0;
        e.data.insert("create_timer".into(), t);
        if t >= 24.0 {
            let id = spawn(game, "oMagmaMan", Vec2::new(e.pos.x - 8.0, e.pos.y - 8.0));
            if let Some(m) = game.world.entity_mut(id) {
                m.health = e.health;
            }
            e.alive = false;
        }
    } else if e.age % 2 == 0 {
        spawn(game, "oMagmaTrail", e.pos);
    }
    let x = (e.pos.x / 16.0).floor() as i32;
    let y = (e.pos.y / 16.0).floor() as i32;
    if game.world.tile(x, y).is_some_and(|t| t.object == "oWater") {
        e.alive = false;
        spawn(game, "oSmokePuff", e.pos);
    }
    e.animation += 0.3;
    ignite(game, e);
}
fn ignite(game: &mut Game, e: &Entity) {
    // Parent handles ordinary contact; burning and bombs are specific magma mechanics.
    if e.bounds().intersects(game.player.bounds()) && game.player.invincible == 0 {
        game.player.data.insert("burning".into(), 100.0);
        game.player.vel = Vec2::new(
            if game.player.pos.x < center(e).x {
                -6.0
            } else {
                6.0
            },
            -4.0,
        );
    }
    for bomb in game
        .world
        .entities
        .iter_mut()
        .filter(|b| b.object == "oBomb" && b.bounds().intersects(e.bounds()))
    {
        if bomb.value("armed") == 0.0 {
            bomb.data.insert("armed".into(), 1.0);
            bomb.timer = 8 + game.rng.range(5);
            bomb.sprite = "sBombArmed".into();
        }
        bomb.vel.x = if bomb.pos.x < e.pos.x { -3.0 } else { 3.0 };
        if bomb.pos.y < e.pos.y {
            bomb.vel.y = -3.0;
        }
        bomb.held = false;
        if game.player.held == Some(bomb.id) {
            game.player.held = None;
        }
    }
    for other in game.world.entities.iter_mut().filter(|o| {
        o.alive
            && super::is_enemy(&o.object)
            && !matches!(
                o.object.as_str(),
                "oMagmaMan"
                    | "oMagma"
                    | "oMagmaTrail"
                    | "oOlmec"
                    | "oOlmecSlam"
                    | "oOlmecDebris"
                    | "oYellowBall"
            )
            && o.bounds().intersects(e.bounds())
    }) {
        other.health -= 2;
        other.vel = Vec2::new(if e.pos.x < other.pos.x { -3.0 } else { 3.0 }, -4.0);
        other.data.insert("burning".into(), 100.0);
        if !matches!(other.object.as_str(), "oTombLord" | "oYetiKing") {
            other.state = 98;
            other.timer = 50;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Area, RulesPreset};
    fn room() -> Game {
        let mut g = Game::shell(9, RulesPreset::Classic);
        g.world = World::new(24, 20, Area::Temple, 13);
        for x in 0..24 {
            g.world.set_tile(x, 10, "oTemple");
        }
        g.player.pos = Vec2::new(230.0, 120.0);
        g
    }
    #[test]
    fn source_stats_match() {
        for (name, hp) in [("oHawkman", 4), ("oTombLord", 20), ("oMagmaMan", 200)] {
            let mut e = Entity::new(1, name, Vec2::new(50.0, 50.0));
            init(&mut e);
            assert_eq!(e.health, hp);
        }
    }
    #[test]
    fn tomb_attack_spawns_fly_with_source_velocity() {
        let mut g = room();
        let mut e = Entity::new(1, "oTombLord", Vec2::new(80.0, 152.0));
        init(&mut e);
        e.state = 3;
        e.animation = 7.0;
        tick(&mut g, &mut e);
        let f = g
            .world
            .entities
            .iter()
            .find(|e| e.object == "oFly")
            .unwrap();
        assert!((3.0..=5.0).contains(&f.vel.x));
        assert!((-2.5..=0.5).contains(&f.vel.y));
    }
    #[test]
    fn slam_preserves_indestructible_tiles() {
        let mut g = room();
        g.world.set_tile(7, 10, "oSolid");
        let mut e = Entity::new(1, "oOlmecSlam", Vec2::new(96.0, 160.0));
        init(&mut e);
        tick(&mut g, &mut e);
        assert_eq!(g.world.tile(7, 10).unwrap().object, "oSolid");
        assert!(g.world.tile(6, 10).unwrap().object.is_empty());
        assert!(g.world.tile(8, 10).unwrap().object.is_empty());
        assert!(!e.alive);
    }
    #[test]
    fn magma_man_falls_into_magma() {
        let mut g = room();
        let mut e = Entity::new(1, "oMagmaMan", Vec2::new(80.0, 80.0));
        init(&mut e);
        tick(&mut g, &mut e);
        assert!(!e.alive);
        assert_eq!(g.world.entities[0].object, "oMagma");
        assert_eq!(g.world.entities[0].health, 199);
    }
    #[test]
    fn magma_arms_bomb_once_without_extending_fuse() {
        let mut g = room();
        let mut magma = Entity::new(100, "oMagmaMan", Vec2::new(80.0, 144.0));
        init(&mut magma);
        let id = g.world.spawn("oBomb", Vec2::new(88.0, 152.0));
        ignite(&mut g, &magma);
        let bomb = g.world.entity_mut(id).unwrap();
        assert_eq!(bomb.value("armed"), 1.0);
        assert!((8..=12).contains(&bomb.timer));
        bomb.timer = 3;
        ignite(&mut g, &magma);
        assert_eq!(g.world.entity(id).unwrap().timer, 3);
    }
    #[test]
    fn olmec_prepares_for_twenty_ticks() {
        let mut g = room();
        let mut e = Entity::new(1, "oOlmec", Vec2::new(80.0, 100.0));
        init(&mut e);
        e.state = 5;
        e.timer = 20;
        for _ in 0..20 {
            tick(&mut g, &mut e);
            assert_eq!(e.state, 5);
        }
        tick(&mut g, &mut e);
        assert_eq!(e.state, 6);
        assert_eq!(e.vel.y, 5.0);
    }
}
