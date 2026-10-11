//! Jungle behaviors translated from Classic 1.1 enemy events.
//! Instances retain original GameMaker sprite origins; probes use collision centers.
#[cfg(test)]
use super::event_bounds;
use super::{center, move_body, steer_toward};
use crate::model::{Area, Entity, Game, PlayerState, Rect, Vec2, World};

pub fn matches(object: &str) -> bool {
    matches!(
        object,
        "oFrog" | "oFireFrog" | "oMonkey" | "oPiranha" | "oJaws" | "oManTrap"
    )
}

pub fn init(e: &mut Entity) {
    e.health = match e.object.as_str() {
        "oJaws" => 40,
        "oManTrap" => 3,
        _ => 1,
    };
    e.state = if e.object == "oMonkey" { 5 } else { 0 };
    e.timer = 0;
    e.data.insert(
        "animation_speed".into(),
        if matches!(e.object.as_str(), "oFrog" | "oFireFrog" | "oMonkey") {
            0.4
        } else {
            0.5
        },
    );
    let sprite = match e.object.as_str() {
        "oFrog" => "sFrogLeft",
        "oFireFrog" => "sFireFrogLeft",
        "oMonkey" => "sMonkeyHangL",
        "oPiranha" => "sPiranhaRight",
        "oJaws" => "sJawsLeft",
        _ => "sManTrapLeft",
    };
    e.set_sprite(sprite);
    if e.object == "oJaws" {
        e.facing = -1;
    }
    if e.object == "oManTrap" {
        e.vel.x = 2.5;
    }
    if e.object == "oMonkey" {
        e.data.insert("throw_counter".into(), 60.0);
    }
    if e.object == "oPiranha" {
        e.data.insert("can_bite".into(), 1.0);
    }
}

pub(super) fn create(game: &mut Game, e: &mut Entity) {
    create_local(
        &mut crate::creation::EntityCreateContext {
            world: &game.world,
            rng: &mut game.rng,
        },
        e,
    );
}

pub(super) fn create_local(ctx: &mut crate::creation::EntityCreateContext<'_>, e: &mut Entity) {
    if !matches(&e.object) || e.value("jungle_context_created") != 0.0 {
        return;
    }
    e.data.insert("jungle_context_created".into(), 1.0);
    match e.object.as_str() {
        "oFrog" | "oFireFrog" | "oMonkey" => {
            e.facing = if ctx.rng.range(2) == 0 { -1 } else { 1 };
            if water(ctx.world, e.pos) {
                e.data.insert("swimming".into(), 1.0);
            }
        }
        "oPiranha" => e.facing = if ctx.rng.range(2) == 0 { -1 } else { 1 },
        _ => {}
    }
}

fn water(world: &World, p: Vec2) -> bool {
    world
        .tile((p.x / 16.0).floor() as i32, (p.y / 16.0).floor() as i32)
        .is_some_and(|t| crate::catalog::is_a(&t.object, "oWater"))
        || world
            .entities
            .iter()
            .any(|e| e.alive && crate::catalog::is_a(&e.object, "oWater") && e.bounds().contains(p))
}

fn vine(world: &World, p: Vec2) -> bool {
    world.tiles_point(p, "oLadder")
        || world.entities.iter().any(|e| {
            e.alive && crate::catalog::is_a(&e.object, "oLadder") && e.sprite_bounds().contains(p)
        })
}

fn countdown(e: &mut Entity, key: &str) {
    let count = e.value(key);
    if count > 0.0 {
        e.data.insert(key.into(), count - 1.0);
    }
}

fn collision_offset(world: &World, e: &Entity, dx: f64, dy: f64) -> bool {
    let b = e.bounds();
    // isCollisionLeft/Right/Top/Bottom check inclusive boundary lines.
    let edge = if dx < 0.0 {
        Rect::new(b.x + dx, b.y, 1.0, b.h)
    } else if dx > 0.0 {
        Rect::new(b.x + b.w + dx - 1.0, b.y, 1.0, b.h)
    } else if dy < 0.0 {
        Rect::new(b.x, b.y + dy, b.w, 1.0)
    } else {
        Rect::new(b.x, b.y + b.h + dy - 1.0, b.w, 1.0)
    };
    super::solid_mask(world, edge, e.id)
}

fn distance_to_player(game: &Game, e: &Entity) -> f64 {
    let a = e.sprite_bounds();
    let b = game.player.sprite_bounds();
    let dx = (a.x - (b.x + b.w - 1.0))
        .max(b.x - (a.x + a.w - 1.0))
        .max(0.0);
    let dy = (a.y - (b.y + b.h - 1.0))
        .max(b.y - (a.y + a.h - 1.0))
        .max(0.0);
    dx.hypot(dy)
}

fn ground(world: &World, e: &Entity) -> bool {
    collision_offset(world, e, 0.0, 1.0)
}

pub fn tick(game: &mut Game, e: &mut Entity) {
    match e.object.as_str() {
        "oFrog" | "oFireFrog" => frog(game, e),
        "oMonkey" => monkey(game, e),
        "oPiranha" => fish(game, e),
        "oJaws" => jaws(game, e),
        "oManTrap" => mantrap(game, e),
        _ => {}
    }
}

pub(super) fn animation_end(_game: &mut Game, e: &mut Entity, sprite: &str) {
    if e.object == "oJaws" {
        match sprite {
            "sJawsTurnL" => e.set_sprite("sJawsLeft"),
            "sJawsTurnR" => {
                e.set_sprite("sJawsRight");
                e.pos.x += 48.0;
                e.previous_pos = e.pos;
            }
            _ => return,
        }
        e.state = 2;
        e.timer = 40;
        return;
    }
    if e.object != "oManTrap" {
        return;
    }
    if e.state == 97 {
        e.set_sprite("sManTrapSleepL");
        e.state = 96;
    } else if sprite == "sManTrapSleepL" {
        e.set_sprite("sManTrapStunL");
        e.state = 98;
        e.timer = (e.value("stun_time") * 2.0) as u32;
    }
}

fn die(game: &mut Game, e: &mut Entity) {
    match e.object.as_str() {
        "oFrog" | "oMonkey" => source_blood(game, e, Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0), 3),
        "oPiranha" => source_blood(game, e, Vec2::new(e.pos.x + 4.0, e.pos.y + 4.0), 3),
        _ => {}
    }
    super::record_death(game, e);
    if e.object == "oFireFrog" {
        let id = game.spawn("oFireFrogBomb", Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0));
        if let Some(bomb) = game.world.entity_mut(id) {
            bomb.timer = 120;
            bomb.data.insert("heavy".into(), 1.0);
        }
    }
    if e.object == "oJaws" {
        let blood = Vec2::new(
            e.pos.x + 22.0 + f64::from(game.rng.range(5)),
            e.pos.y + 14.0 + f64::from(game.rng.range(5)),
        );
        if e.value("bloodless") == 0.0 {
            for _ in 0..4 {
                if game
                    .world
                    .entities
                    .iter()
                    .filter(|other| {
                        other.alive
                            && other.engine_active
                            && crate::catalog::is_a(&other.object, "oDetritus")
                    })
                    .count()
                    < 16
                {
                    crate::world_events::spawn_detritus(game, "oBlood", blood);
                }
            }
        }
        // Original Step executes all four Bone Create events before drawing
        // any crate launch velocity. Their random draws are synchronous.
        for _ in 0..4 {
            let bone = Vec2::new(
                e.pos.x + 22.0 + game.rng.range(5) as f64,
                e.pos.y + 14.0 + game.rng.range(7) as f64,
            );
            crate::world_events::spawn_detritus(game, "oBone", bone);
        }
        for _ in 0..4 {
            let id = game.spawn("oCrate", Vec2::new(e.pos.x + 16.0, e.pos.y + 16.0));
            let velocity = Vec2::new(
                game.rng.range(4) as f64 - game.rng.range(4) as f64,
                -(1.0 + game.rng.range(2) as f64),
            );
            if let Some(item) = game.world.entity_mut(id) {
                item.vel = velocity;
            }
        }
    }
    if e.object == "oManTrap" {
        mantrap_leaf_burst(game, e);
        source_blood(game, e, Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0), 1);
        if e.value("ate_shopkeeper") != 0.0 {
            release_shopkeeper(game, e);
        }
    }
    // Source instance_destroy is immediate. Mark removal so a death caused
    // earlier in this same tick cannot execute its spawn hook twice.
    e.alive = false;
}

fn frog(game: &mut Game, e: &mut Entity) {
    move_body(&game.world, e, 0.2);
    if game.world.solid_at(e.pos.x + 8.0, e.pos.y + 8.0) {
        e.health = -999;
    }
    if e.health <= 0 {
        die(game, e);
        // Both Frog events continue their collision/state/sprite tail after
        // immediate Enemy Destroy. FireFrog's Bomb birth remains inside die.
        super::destroyed(game, e);
    }
    if collision_offset(&game.world, e, 1.0, 0.0) {
        e.vel.x = -1.0;
    }
    if collision_offset(&game.world, e, -1.0, 0.0) {
        e.vel.x = 1.0;
    }
    let grounded = ground(&game.world, e);
    match e.state {
        0 => {
            e.vel.x = 0.0;
            if e.timer > 0 {
                e.timer -= 1;
            } else if distance_to_player(game, e) < 64.0 {
                e.state = 1;
                game.sound("sndFrog", e.pos);
            }
        }
        1 => {
            if grounded {
                e.vel.y = -(2.0 + game.rng.range(3) as f64);
                e.facing = if game.player.pos.x < e.pos.x { -1 } else { 1 };
                e.vel.x = 3.0 * e.facing as f64;
            } else {
                e.state = 2;
            }
        }
        2 if grounded => {
            e.state = 0;
            e.vel = Vec2::default();
            e.timer = 10 + game.rng.range(31);
        }
        2 | 4 => {}
        _ => {
            e.state = 0;
            e.vel.x = 0.0;
        }
    }
    if collision_offset(&game.world, e, 0.0, -1.0) {
        e.vel.y = 1.0;
    }
    let fire = e.object == "oFireFrog";
    let sprite = match (fire, grounded) {
        (true, true) => "sFireFrogLeft",
        (true, false) => "sFireFrogJumpL",
        (false, true) => "sFrogLeft",
        (false, false) => "sFrogJumpL",
    };
    e.set_sprite(sprite);
}

fn monkey(game: &mut Game, e: &mut Entity) {
    e.data.insert("depth".into(), 40.0);
    let (wall, _) = move_body(
        &game.world,
        e,
        if matches!(e.state, 5..=7) { 0.0 } else { 0.2 },
    );
    if game.world.solid_at(e.pos.x + 8.0, e.pos.y + 8.0) {
        e.health = -999;
    }
    let swimming = water(&game.world, Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0));
    if swimming && e.value("swimming") == 0.0 {
        game.spawn("oSplash", Vec2::new(e.pos.x + 8.0, e.pos.y));
        game.sound("sndSplash", e.pos);
    }
    e.data.insert("swimming".into(), f64::from(swimming));
    if e.health <= 0 {
        die(game, e);
        // instance_destroy runs inherited Enemy Destroy immediately, then the
        // current Monkey Step continues through its independent state tail.
        super::destroyed(game, e);
    }
    if collision_offset(&game.world, e, 1.0, 0.0) {
        e.vel.x = -1.0;
    }
    if collision_offset(&game.world, e, -1.0, 0.0) {
        e.vel.x = 1.0;
    }
    let grounded = ground(&game.world, e);
    for key in ["grab_counter", "vine_counter", "throw_counter"] {
        countdown(e, key);
    }
    let dist = distance_to_player(game, e);
    match e.state {
        0 => {
            e.vel.x = 0.0;
            if e.timer > 0 {
                e.timer -= 1;
            } else {
                e.state = 3;
            }
            if dist < 64.0 {
                e.state = 1;
            }
        }
        1 => {
            if grounded {
                e.vel.y = -(4.0 + game.rng.range(2) as f64);
                e.facing = if game.player.pos.x < center(e).x {
                    -1
                } else {
                    1
                };
                e.vel.x = 2.0 * e.facing as f64;
            } else {
                e.state = 2;
                game.sound("sndMonkey", e.pos);
            }
        }
        2 => {
            if grounded {
                e.state = 0;
                e.vel = Vec2::default();
                e.timer = 10 + game.rng.range(31);
            } else if vine(&game.world, center(e)) && e.value("vine_counter") == 0.0 {
                e.state = 5;
                e.vel = Vec2::default();
                e.timer = 10 + game.rng.range(31);
            }
        }
        3 => {
            if wall {
                e.facing *= -1;
            }
            e.vel.x = 2.0 * e.facing as f64;
            if game.rng.chance(1, 100) {
                e.state = 0;
                e.timer = 20 + game.rng.range(31);
                e.vel.x = 0.0;
            }
        }
        5 => {
            e.vel = Vec2::default();
            if e.timer > 0 {
                e.timer -= 1;
            } else {
                e.state = 6;
            }
        }
        6 => {
            e.vel.x = 0.0;
            e.vel.y = if e.value("dir") == 0.0 { -1.0 } else { 1.0 };
            if !vine(
                &game.world,
                Vec2::new(
                    e.pos.x + 8.0,
                    e.pos.y + if e.vel.y < 0.0 { 0.0 } else { 22.0 },
                ),
            ) {
                e.data
                    .insert("dir".into(), if e.vel.y < 0.0 { 1.0 } else { 0.0 });
                e.state = 5;
                e.timer = 10 + game.rng.range(31);
            }
            if dist < 64.0 && game.player.pos.y > e.pos.y {
                e.state = 1;
                e.data.insert("vine_counter".into(), 30.0);
                e.vel.y = -(2.0 + game.rng.range(3) as f64);
                e.facing = if game.player.pos.x < e.pos.x { -1 } else { 1 };
                e.vel.x = 3.0 * e.facing as f64;
            }
        }
        7 => {
            e.data.insert("depth".into(), 120.0);
            e.vel = Vec2::default();
            e.pos = Vec2::new(
                game.player.pos.x + e.value("grab_x"),
                game.player.pos.y + e.value("grab_y"),
            );
            e.facing = game.player.facing;
            if e.timer > 0 {
                e.timer -= 1;
            } else {
                monkey_steal(game, e);
                release_monkey(game, e);
            }
        }
        4 => {}
        _ => {
            e.state = 0;
            e.vel.x = 0.0;
        }
    }
    if e.state != 7 && collision_offset(&game.world, e, 0.0, -1.0) {
        e.vel.y = 1.0;
    }
    let sprite = match e.state {
        5 => "sMonkeyHangL",
        6 | 7 => "sMonkeyClimbL",
        3 if grounded => "sMonkeyWalkL",
        _ if !grounded => "sMonkeyJumpL",
        _ => "sMonkeyLeft",
    };
    e.set_sprite(sprite);
}

fn release_monkey(game: &mut Game, e: &mut Entity) {
    e.state = 1;
    e.data.insert("vine_counter".into(), 20.0);
    e.data.insert("grab_counter".into(), 60.0);
    e.facing = if game.player.pos.x > center(e).x {
        -1
    } else {
        1
    };
    e.vel = Vec2::new(3.0 * e.facing as f64, -(2.0 + game.rng.range(3) as f64));
}

fn monkey_steal(game: &mut Game, e: &Entity) {
    let area = match game.world.area {
        Area::Mines => 0,
        Area::Jungle => 1,
        Area::Ice => 2,
        Area::Temple => 3,
        Area::Boss => 4,
    };
    let money = 500 + 125 * area;
    let object;
    // GML evaluates both operands of the source money-and-rand condition,
    // including its roll when the player's money is below the threshold.
    if game.rng.chance(1, 4) {
        game.player.vel = Vec2::new(3.0 * game.player.facing as f64, -3.0);
        game.player.stun = 40;
        game.player.state = PlayerState::Stunned;
        crate::items::drop_held_with_velocity(game, Vec2::default());
        game.sound("sndHit", e.pos);
        return;
    } else if game.rng.chance(8, 10) && game.player.money >= money {
        game.player.money -= money;
        object = "oGoldNugget";
    } else if game.rng.chance(1, 2) && game.player.ropes > 0 {
        game.player.ropes -= 1;
        object = "oRopeThrow";
    } else if game.player.bombs > 0 {
        game.player.bombs -= 1;
        object = "oBomb";
    } else {
        return;
    }
    let id = game.spawn(object, e.pos);
    let armed = object == "oBomb" && game.rng.chance(1, 10);
    let velocity = Vec2::new(
        game.rng.range(3) as f64 - game.rng.range(3) as f64,
        -(3.0 + game.rng.range(2) as f64),
    );
    if let Some(item) = game.world.entity_mut(id) {
        crate::items::init_entity(item);
        item.vel = velocity;
        if object == "oGoldNugget" {
            item.data.insert("canCollect".into(), 0.0);
            item.alarms[0] = 20;
        }
        if armed {
            item.data.insert("armed".into(), 1.0);
            item.timer = 40;
            item.alarms[1] = 40;
            item.set_sprite("sBombArmed");
            item.set_image_speed(1.0);
        }
    }
    game.sound("sndThrow", e.pos);
}

fn fish(game: &mut Game, e: &mut Entity) {
    if e.health <= 0 {
        die(game, e);
        // The source retires this receiver, then executes the independent
        // swimming, bubble, and dry FishBone tail of its current Step.
        super::destroyed(game, e);
    }
    // All Piranha states pass displacement directly to moveTo; they never
    // assign xVel/yVel. Preserve velocity supplied by external collisions.
    let stored_velocity = e.vel;
    let target_swimming = super::player_alive(game)
        && game.player.data.get("swimming").copied().unwrap_or(0.0) != 0.0;
    let near = center(e).distance(game.player.pos) < 90.0;
    match e.state {
        0 => {
            e.vel = Vec2::new(e.facing as f64, 0.0);
            let probe = Vec2::new(e.pos.x + if e.facing > 0 { 10.0 } else { -2.0 }, e.pos.y);
            if water(&game.world, probe) && !game.world.solid_at(probe.x, probe.y) {
                move_body(&game.world, e, 0.0);
            } else {
                e.facing *= -1;
                e.vel = Vec2::default();
            }
            if target_swimming && near {
                e.state = 1;
            }
            // Source independently checks an NPC after selecting the player;
            // a swimming NPC therefore takes priority over a nearby player.
            if let Some(target) = fish_target(&game.world, e.pos) {
                e.state = 3;
                e.data.insert("fish_target".into(), target as f64);
            }
        }
        1 => {
            if target_swimming && near {
                let target = game.player.pos;
                aim_fish(game, e, target);
                swim_move(&game.world, e);
            } else {
                e.state = 2;
                e.timer = 20 + game.rng.range(21);
            }
        }
        2 => {
            e.data.insert("can_bite".into(), 1.0);
            e.vel = Vec2::default();
            if e.timer > 0 {
                e.timer -= 1;
            } else {
                e.state = 0;
                e.facing = if game.rng.chance(1, 2) { -1 } else { 1 };
            }
        }
        3 => {
            // instance_nearest is evaluated afresh every tick in the source.
            let target = fish_target(&game.world, e.pos)
                .and_then(|id| game.world.entity(id))
                .map(|t| (t.id, Vec2::new(t.pos.x + 8.0, t.pos.y + 8.0)));
            if let Some((id, pos)) = target {
                e.data.insert("fish_target".into(), id as f64);
                aim_fish(game, e, pos);
                if !swim_move(&game.world, e) {
                    e.state = 2;
                    e.timer = 20 + game.rng.range(21);
                }
                if center(e).distance(pos) < 4.0 {
                    let bite = e.value("can_bite") != 0.0;
                    if let Some(target) = game.world.entity_mut(id) {
                        target.state = 2;
                        if bite {
                            target.health -= 1;
                        }
                    }
                    if bite {
                        e.data.insert("can_bite".into(), 0.0);
                        e.alarms[0] = 10;
                        source_blood(game, e, Vec2::new(e.pos.x + 4.0, e.pos.y + 4.0), 1);
                    }
                }
            } else {
                e.state = 2;
                e.timer = 20 + game.rng.range(21);
            }
        }
        _ => {
            e.state = 0;
        }
    }
    let sprite = if e.facing < 0 {
        "sPiranhaLeft"
    } else {
        "sPiranhaRight"
    };
    e.set_sprite(sprite);
    bubbles(game, e, true);
    if !water(&game.world, Vec2::new(e.pos.x + 4.0, e.pos.y + 4.0)) {
        game.spawn("oFishBone", e.pos);
        e.alive = false;
        super::destroyed(game, e);
    }
    e.vel = stored_velocity;
}

fn aim_fish(game: &mut Game, e: &mut Entity, target: Vec2) {
    steer_toward(e, target, 1.0);
    let jitter = (game.rng.range(2) as f64 - game.rng.range(2) as f64).to_radians();
    let (sin, cos) = jitter.sin_cos();
    e.vel = Vec2::new(e.vel.x * cos - e.vel.y * sin, e.vel.x * sin + e.vel.y * cos);
}

fn fish_target(world: &World, pos: Vec2) -> Option<u64> {
    // Classic checks each family in this order, rather than choosing globally.
    for family in ["oCaveman", "oShopkeeper", "oHawkman", "oYeti"] {
        let target = world
            .entities
            .iter()
            .filter(|t| t.alive && crate::catalog::is_a(&t.object, family))
            .min_by(|a, b| a.pos.distance(pos).total_cmp(&b.pos.distance(pos)));
        if let Some(t) = target {
            if t.health > 0 {
                return (t.value("swimming") != 0.0).then_some(t.id);
            }
        }
    }
    None
}

fn swim_move(world: &World, e: &mut Entity) -> bool {
    let next = Vec2::new(e.pos.x + e.vel.x, e.pos.y + e.vel.y);
    if !water(world, next) || world.solid_at(next.x, next.y) {
        return false;
    }
    move_body(world, e, 0.0);
    true
}

fn jaws(game: &mut Game, e: &mut Entity) {
    // Source tests the mouth, not the middle of the 64-pixel body.
    if !water(&game.world, Vec2::new(e.pos.x + 8.0, e.pos.y + 16.0)) {
        e.health -= 1;
    }
    if e.health <= 0 {
        die(game, e);
        // Mouth drying precedes the sole death block. Loot and immediate
        // Enemy Destroy finish before the remaining state/bubble statements.
        super::destroyed(game, e);
    }
    let swimming = super::player_alive(game)
        && game.player.data.get("swimming").copied().unwrap_or(0.0) != 0.0;
    let turn_before = matches!(e.sprite.as_str(), "sJawsTurnL" | "sJawsTurnR");
    match e.state {
        0 => {
            let probe = Vec2::new(
                e.pos.x + if e.facing > 0 { 18.0 } else { -2.0 },
                e.pos.y + 16.0,
            );
            if water(&game.world, probe) && !game.world.solid_at(probe.x, probe.y) {
                e.vel = Vec2::new(2.0 * e.facing as f64, 0.0);
                move_body(&game.world, e, 0.0);
            } else {
                let free = if e.facing > 0 {
                    crate::Rect::new(e.pos.x - 32.0, e.pos.y, 32.0, 32.0)
                } else {
                    crate::Rect::new(e.pos.x + 16.0, e.pos.y, 32.0, 32.0)
                };
                if !game.world.collides_except(free, e.id) {
                    begin_jaws_turn(e);
                }
            }
            if !collision_offset(&game.world, e, 0.0, 2.0) {
                e.pos.y += 1.0;
            }
            // The source deliberately overwrites TURN here. Animation end is
            // independent of the state and still finishes the turn later.
            if swimming {
                e.state = 1;
            }
        }
        1 => {
            if !swimming {
                e.state = 0;
            } else if turn_before {
                // Turn sprites suppress re-aiming; source dir is the horizontal
                // direction established when the turn began.
                let probe = Vec2::new(e.pos.x + e.facing as f64, e.pos.y);
                if water(&game.world, probe) && !game.world.solid_at(probe.x, probe.y) {
                    e.vel = Vec2::new(3.0 * e.facing as f64, 0.0);
                    move_body(&game.world, e, 0.0);
                }
            } else {
                let wants_left = game.player.pos.x < e.pos.x + 8.0;
                let can_turn = if wants_left {
                    !game.world.collides_except(
                        crate::Rect::new(e.pos.x - 32.0, e.pos.y, 32.0, 32.0),
                        e.id,
                    )
                } else {
                    !game.world.solid_at(e.pos.x - 2.0, e.pos.y + 16.0)
                };
                if wants_left != (e.facing < 0) && can_turn {
                    begin_jaws_turn(e);
                } else {
                    let origin = Vec2::new(e.pos.x + 8.0, e.pos.y + 16.0);
                    let target = Vec2::new(game.player.pos.x, game.player.pos.y - 8.0);
                    let dist = origin.distance(target);
                    if dist > 0.0 {
                        let dir =
                            Vec2::new((target.x - origin.x) / dist, (target.y - origin.y) / dist);
                        let probe = Vec2::new(e.pos.x + dir.x, e.pos.y + dir.y);
                        if water(&game.world, probe) && !game.world.solid_at(probe.x, probe.y) {
                            e.vel = Vec2::new(dir.x * 3.0, dir.y * 3.0);
                            move_body(&game.world, e, 0.0);
                        }
                    }
                }
            }
        }
        2 => {
            if e.timer > 0 {
                e.timer -= 1;
            } else {
                e.state = 0;
            }
        }
        3 => {}
        _ => {
            e.state = 0;
        }
    }
    let turning = matches!(e.sprite.as_str(), "sJawsTurnL" | "sJawsTurnR");
    let sprite = match (turning, e.facing < 0) {
        (true, true) => "sJawsTurnL",
        (true, false) => "sJawsTurnR",
        (false, true) => "sJawsLeft",
        (false, false) => "sJawsRight",
    };
    e.set_sprite(sprite);
    // Turning sprites retain the previous collision rectangle in GameMaker.
    if !turning {
        jaws_bounds(e);
    }
    bubbles(game, e, false);
}

fn begin_jaws_turn(e: &mut Entity) {
    e.state = 3;
    e.facing *= -1;
    e.vel = Vec2::default();
    e.animation = 0.0;
    let sprite = if e.facing < 0 {
        "sJawsTurnL"
    } else {
        "sJawsTurnR"
    };
    e.set_sprite(sprite);
    if e.facing < 0 {
        e.pos.x -= 48.0;
        e.previous_pos = e.pos;
    }
}

fn jaws_bounds(e: &mut Entity) {
    let [left, right] = if e.facing < 0 {
        [0.0, 64.0]
    } else {
        [-48.0, 16.0]
    };
    for (key, value) in [
        ("collision_left", left),
        ("collision_right", right),
        ("collision_top", 0.0),
        ("collision_bottom", 32.0),
    ] {
        e.data.insert(key.into(), value);
    }
}

fn bubbles(game: &mut Game, e: &mut Entity, jitter: bool) {
    if e.value("bubble_timer") > 0.0 {
        countdown(e, "bubble_timer");
    } else {
        let pos = if e.object == "oJaws" {
            Vec2::new(e.pos.x, e.pos.y + 16.0)
        } else {
            e.pos
        };
        crate::world_events::spawn_effect(game, "oBubble", pos);
        e.data.insert(
            "bubble_timer".into(),
            if jitter {
                30.0 + game.rng.range(21) as f64
            } else {
                40.0
            },
        );
    }
}

fn source_blood(game: &mut Game, e: &Entity, pos: Vec2, count: usize) {
    if e.value("bloodless") != 0.0 {
        return;
    }
    for _ in 0..count {
        if game
            .world
            .entities
            .iter()
            .filter(|other| {
                other.alive
                    && other.engine_active
                    && crate::catalog::is_a(&other.object, "oDetritus")
            })
            .count()
            < 16
        {
            crate::world_events::spawn_detritus(game, "oBlood", pos);
        }
    }
}

fn mantrap_leaf_burst(game: &mut Game, e: &Entity) {
    for _ in 0..3 {
        let pos = Vec2::new(
            e.pos.x + f64::from(game.rng.range(17)),
            e.pos.y - 8.0 + f64::from(game.rng.range(17)),
        );
        crate::world_events::spawn_effect(game, "oLeaf", pos);
    }
}

fn release_shopkeeper(game: &mut Game, e: &Entity) {
    let pos = e.pos;
    let id = game.spawn("oShopkeeper", pos);
    if let Some(shopkeeper) = game.world.entity_mut(id) {
        super::init_entity(shopkeeper);
        shopkeeper.state = 2;
        shopkeeper.data.insert("has_gun".into(), 0.0);
    }
    mantrap_leaf_burst(game, e);
    source_blood(game, e, Vec2::new(pos.x + 8.0, pos.y + 8.0), 1);
    game.sound("sndSmallExplode", pos);
}

fn mantrap(game: &mut Game, e: &mut Entity) {
    let gravity = if e.value("swimming") > 0.0 { 0.2 } else { 0.6 };
    let incoming_y = if e.held { 0.0 } else { e.vel.y };
    let (wall, _) = if e.held {
        e.vel = Vec2::default();
        (false, false)
    } else {
        move_body(&game.world, e, gravity)
    };
    // moveTo does not clear source yVel at a collision. Preserve the incoming
    // speed for scrCheckCollisions' stunned-body bounce.
    e.vel.y = if e.held {
        0.0
    } else {
        (incoming_y + gravity).min(10.0)
    };
    if e.health <= 0 {
        // instance_destroy marks this receiver; the current GML event continues.
        die(game, e);
    }
    let left = collision_offset(&game.world, e, -1.0, 0.0);
    let right = collision_offset(&game.world, e, 1.0, 0.0);
    let top = collision_offset(&game.world, e, 0.0, -1.0);
    let grounded = ground(&game.world, e);
    if grounded && e.state != 98 {
        e.vel.y = 0.0;
    }
    match e.state {
        0 => {
            if e.timer > 0 {
                e.timer -= 1;
            }
            if e.timer == 0 {
                e.state = 1;
                e.facing = if game.rng.chance(1, 2) { -1 } else { 1 };
            }
        }
        1 => {
            if wall {
                e.facing *= -1;
            }
            let left_top = game.world.solid_at(e.pos.x - 1.0, e.pos.y);
            let left_floor = game.world.solid_at(e.pos.x - 1.0, e.pos.y + 16.0);
            let right_top = game.world.solid_at(e.pos.x + 16.0, e.pos.y);
            let right_floor = game.world.solid_at(e.pos.x + 16.0, e.pos.y + 16.0);
            if e.facing < 0 && !left_top && !left_floor {
                e.facing = 1;
            } else if e.facing > 0 && !right_top && !right_floor {
                e.facing = -1;
            }
            if (!left_floor || left_top) && (!right_floor || right_top) {
                e.facing = if left_top { 1 } else { -1 };
                e.vel.x = 0.0;
            } else {
                e.vel.x = e.facing as f64;
            }
            if game.rng.chance(1, 100) {
                e.state = 0;
                e.timer = 20 + game.rng.range(31);
                e.vel.x = 0.0;
            }
        }
        97 => {
            e.vel.x = 0.0;
        }
        96 => {
            e.vel.x = 0.0;
        }
        98 => {
            e.set_sprite("sManTrapStunL");
            if grounded && e.value("bounced") == 0.0 {
                e.data.insert("bounced".into(), 1.0);
                source_blood(game, e, Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0), 1);
            }
            if grounded || e.held {
                if e.timer > 0 {
                    e.timer -= 1;
                } else if e.value("ate_shopkeeper") != 0.0 {
                    release_shopkeeper(game, e);
                    e.alive = false;
                } else if e.health > 0 {
                    e.state = 0;
                    e.timer = 20 + game.rng.range(31);
                    if e.held {
                        if game.world.solid_at(e.pos.x + 16.0, e.pos.y + 8.0) {
                            e.pos.x = game.player.pos.x - 12.0;
                        } else if game.world.solid_at(e.pos.x, e.pos.y + 8.0) {
                            e.pos.x = game.player.pos.x - 4.0;
                        }
                        e.pos.y = game.player.pos.y - 8.0;
                    }
                    e.held = false;
                    if game.player.held == Some(e.id) {
                        game.player.held = None;
                        game.player.pickup_item_type.clear();
                    }
                }
            }
        }
        _ => {}
    }
    if e.state >= 98 {
        for (key, value) in [
            ("collision_left", 2.0),
            ("collision_top", 6.0),
            ("collision_right", 14.0),
            ("collision_bottom", 16.0),
        ] {
            e.data.insert(key.into(), value);
        }
        if left && !right {
            e.pos.x += 1.0;
        } else if right {
            e.pos.x -= 1.0;
        }
        if left || right {
            e.vel.x *= -0.5;
        }
        if top && !grounded {
            e.pos.y += 1.0;
        } else if grounded {
            if e.vel.y > 1.0 {
                e.vel.y *= -0.5;
            } else if e.vel.y.abs() < 1.0 {
                e.vel.y = 0.0;
            }
            if e.vel.x.abs() < 0.1 {
                e.vel.x = 0.0;
            } else {
                e.vel.x *= 0.3;
            }
        }
    }
    if grounded {
        if e.vel.x.abs() < 0.1 {
            e.vel.x = 0.0;
        } else {
            e.vel.x *= 0.3;
        }
    }
    if super::solid_mask(&game.world, e.bounds(), e.id) {
        e.pos.y -= 2.0;
    }
    if e.state == 97 && e.animation == 8.0 {
        source_blood(game, e, Vec2::new(e.pos.x + 8.0, e.pos.y), 1);
    }
    // GML evaluates the tail roll even when status or the real image phase
    // excludes a Bone. Blood Create above must consume its RNG first.
    let spit_bone = game.rng.range(8) == 0;
    if e.state == 96 && e.animation == 6.0 && spit_bone {
        let pos = Vec2::new(
            e.pos.x + if e.facing < 0 { 2.0 } else { 14.0 },
            e.pos.y + 4.0,
        );
        let bone = crate::world_events::spawn_detritus(game, "oBone", pos);
        if let Some(bone) = game.world.entity_mut(bone) {
            bone.vel.x = if e.facing < 0 { -2.0 } else { 2.0 };
        }
    }
    if e.state < 96 {
        e.set_sprite("sManTrapLeft");
    }
}

/// Original Alarm 0 is an engine event, independently of Piranha's Step.
pub(super) fn alarm(_game: &mut Game, e: &mut Entity, index: u8) {
    if e.object == "oPiranha" && index == 0 {
        e.data.insert("can_bite".into(), 1.0);
    }
}

/// Automatic collision dispatch already selected this exact overlapping pair.
/// Return true for an overridden event, including source branches that ignore it.
pub(super) fn collision(
    game: &mut Game,
    e: &mut Entity,
    other: crate::dispatcher::Instance,
) -> bool {
    match e.object.as_str() {
        "oManTrap" => mantrap_collision(game, e, other),
        "oMonkey" => monkey_collision(game, e, other),
        _ => false,
    }
}

fn mantrap_collision(game: &mut Game, e: &mut Entity, other: crate::dispatcher::Instance) -> bool {
    use crate::dispatcher::Instance;
    if other == Instance::Player {
        if !e.alive || (game.player.pos.x - (e.pos.x + 8.0)).abs() > 8.0 {
            return true;
        }
        let stomp = (game.player.has("SpikeShoes") || e.state == 97)
            && super::player_alive(game)
            && game.player.stun == 0
            && matches!(
                game.player.state,
                PlayerState::Jumping | PlayerState::Falling
            )
            && game.player.pos.y < e.pos.y + 5.0
            && game.player.data.get("swimming").copied().unwrap_or(0.0) == 0.0;
        if stomp {
            let shoes = game.player.has("SpikeShoes");
            let falls = game.player.data.get("fall_timer").copied().unwrap_or(0.0);
            let damage = (falls / 16.0).floor() as i32 + 1;
            game.player.vel.y = -6.0 - 0.2 * game.player.vel.y;
            game.player.data.insert("fall_timer".into(), 0.0);
            e.health -= damage * if shoes { 3 } else { 1 };
            if shoes && e.value("bloodless") == 0.0 {
                crate::world_events::spawn_detritus(
                    game,
                    "oBlood",
                    Vec2::new(game.player.pos.x, game.player.pos.y + 8.0),
                );
            }
            e.data.insert("counts_as_kill".into(), 1.0);
            e.data.insert("environmental_death".into(), 0.0);
            e.state = 98;
            e.timer = e.value("stun_time") as u32;
            e.vel.y = -6.0;
            e.vel.x += if game.player.pos.x < e.pos.x + 8.0 {
                1.0
            } else {
                -1.0
            };
            e.data.insert("animation_speed".into(), 0.5);
            game.sound("sndHit", e.pos);
        } else if game.player.data.get("hidden").copied().unwrap_or(0.0) == 0.0
            && game.player.invincible == 0
            && !matches!(e.state, 97 | 98)
        {
            let sprite = if game.flags.contains("character_damsel") {
                "sManTrapEatDamselL"
            } else if game.flags.contains("character_tunnelman") {
                "sManTrapEatTunnelL"
            } else {
                "sManTrapEatL"
            };
            eat(e, sprite, game.player.pos.x);
            game.player.health = -99;
            game.player.invincible = 9999;
            game.player.data.insert("hidden".into(), 1.0);
            game.player.data.insert("bounced".into(), 1.0);
            game.data.insert("draw_hud".into(), 0.0);
            if crate::statistics::is_real_level(game) {
                game.lifetime_stats
                    .enemy_death(crate::statistics::EnemyDeath::ManTrap);
            }
            if let Some(id) = game.player.held.take() {
                game.player.pickup_item_type.clear();
                if e.held {
                    e.held = false;
                } else if let Some(item) = game.world.entity_mut(id) {
                    item.held = false;
                    item.vel = Vec2::new(e.facing as f64 * 2.0, -4.0);
                }
            }
            game.sound("sndDie", game.player.pos);
        }
        return true;
    }
    let Instance::Entity(id) = other else {
        return false;
    };
    let Some(victim) = game.world.entity(id) else {
        return false;
    };
    let (family, sprite) = if crate::catalog::is_a(&victim.object, "oCaveman") {
        ("oCaveman", "sManTrapEatCavemanL")
    } else if crate::catalog::is_a(&victim.object, "oDamsel") {
        ("oDamsel", "sManTrapEatDamselL")
    } else if crate::catalog::is_a(&victim.object, "oShopkeeper") {
        ("oShopkeeper", "sManTrapEatShopkeeperL")
    } else {
        return false;
    };
    if !e.alive || matches!(e.state, 97 | 98) {
        return true;
    }
    let (pos, velocity_x, health, held, has_gun) = (
        victim.pos,
        victim.vel.x,
        victim.health,
        victim.held,
        victim.data.get("has_gun").copied().unwrap_or(1.0) != 0.0,
    );
    eat(e, sprite, pos.x);
    e.facing = if pos.x > e.pos.x { 1 } else { -1 };
    if family == "oShopkeeper" {
        if health > 0 {
            e.data.insert("ate_shopkeeper".into(), 1.0);
        }
        if has_gun {
            let gun = game.spawn("oShotgun", Vec2::new(pos.x + 8.0, pos.y + 8.0));
            let vertical = f64::from(4 + game.rng.range(3));
            let horizontal =
                f64::from(4 + game.rng.range(3)) * if velocity_x < 0.0 { -1.0 } else { 1.0 };
            if let Some(gun) = game.world.entity_mut(gun) {
                gun.vel = Vec2::new(horizontal, vertical);
                gun.data.insert("cost".into(), 0.0);
                gun.data.insert("forSale".into(), 0.0);
                gun.data.insert("for_sale".into(), 0.0);
                gun.data
                    .insert("released_by_enemy_tick".into(), game.tick as f64);
            }
        }
    }
    if let Some(victim) = game.world.entity_mut(id) {
        victim.alive = false;
        if family == "oShopkeeper" {
            victim.data.insert("has_gun".into(), 0.0);
        }
    }
    if family == "oDamsel" && held {
        game.player.held = None;
        game.player.pickup_item_type.clear();
    }
    true
}

fn monkey_collision(game: &mut Game, e: &mut Entity, other: crate::dispatcher::Instance) -> bool {
    use crate::dispatcher::Instance;
    if other == Instance::Player {
        if !e.alive || e.state == 7 || (game.player.pos.x - (e.pos.x + 8.0)).abs() > 4.0 {
            return true;
        }
        if super::player_alive(game)
            && matches!(
                game.player.state,
                PlayerState::Jumping | PlayerState::Falling
            )
            && game.player.pos.y < e.pos.y + 2.0
            && game.player.data.get("swimming").copied().unwrap_or(0.0) == 0.0
        {
            game.player.vel.y = -6.0 - 0.2 * game.player.vel.y;
            game.player.data.insert("fall_timer".into(), 0.0);
            e.health -= 1;
            game.sound("sndHit", e.pos);
        } else if game.player.invincible == 0 && e.value("grab_counter") == 0.0 {
            e.pos.y = (e.pos.y + 8.0).clamp(game.player.pos.y - 2.0, game.player.pos.y + 2.0) - 8.0;
            e.state = 7;
            e.vel = Vec2::default();
            e.data.insert("grab_x".into(), e.pos.x - game.player.pos.x);
            e.data.insert("grab_y".into(), e.pos.y - game.player.pos.y);
            e.timer = 40 + game.rng.range(41);
        }
        return true;
    }
    let Instance::Entity(id) = other else {
        return false;
    };
    let Some(item) = game.world.entity(id) else {
        return false;
    };
    if !crate::catalog::is_a(&item.object, "oItem") {
        return false;
    }
    if !e.alive
        || e.state == 7
        || e.value("throw_counter") != 0.0
        || item.held
        || item.data.get("active").copied().unwrap_or(1.0) == 0.0
    {
        return true;
    }
    let rope = item.object == "oRopeThrow";
    if rope && item.value("falling") != 0.0 {
        return true;
    }
    let pos = item.pos;
    let lift = !game
        .world
        .solid_at(pos.x, pos.y - if rope { 0.0 } else { 2.0 });
    if let Some(item) = game.world.entity_mut(id) {
        item.vel = Vec2::new(5.0 * e.facing as f64, -4.0);
        if lift {
            item.pos.y = if rope { e.pos.y - 2.0 } else { pos.y - 2.0 };
        }
    }
    e.data.insert("throw_counter".into(), 60.0);
    e.state = 0;
    e.timer = 20 + game.rng.range(41);
    true
}

fn eat(e: &mut Entity, sprite: &str, x: f64) {
    e.state = 97;
    e.vel.x = 0.0;
    e.facing = if x > center(e).x { 1 } else { -1 };
    e.set_sprite(sprite);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::RulesPreset;

    fn entity(object: &str, pos: Vec2) -> Entity {
        let mut entity = Entity::new(999, object, Vec2::default());
        super::super::init_entity(&mut entity);
        let offset = center(&entity);
        entity.pos = Vec2::new(pos.x - offset.x, pos.y - offset.y);
        entity.previous_pos = entity.pos;
        entity
    }
    fn pond(game: &mut Game) {
        game.player.data.insert("swimming".into(), 1.0);
        for y in 1..10 {
            for x in 1..10 {
                game.world.set_tile(x, y, "oWaterSwim");
            }
        }
    }

    #[test]
    fn frog_uses_source_jump_speed_and_recovery() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        for x in 0..10 {
            game.world.set_tile(x, 4, "oBlock");
        }
        game.player.pos = Vec2::new(75.0, 58.0);
        let mut e = entity("oFrog", Vec2::new(56.0, 58.0));
        tick(&mut game, &mut e);
        assert_eq!(e.state, 1);
        tick(&mut game, &mut e);
        assert_eq!(e.vel.x, 3.0);
        assert!((-4.0..=-2.0).contains(&e.vel.y));
        tick(&mut game, &mut e);
        assert_eq!(e.state, 2);
    }
    #[test]
    fn piranha_pursues_only_swimming_player() {
        let mut game = Game::shell(2, RulesPreset::Classic);
        pond(&mut game);
        game.player.pos = Vec2::new(70.0, 50.0);
        let mut e = entity("oPiranha", Vec2::new(50.0, 50.0));
        tick(&mut game, &mut e);
        assert_eq!(e.state, 1);
        game.player.pos = Vec2::new(300.0, 300.0);
        tick(&mut game, &mut e);
        assert_eq!(e.state, 2);
        assert!((20..=40).contains(&e.timer));
    }
    #[test]
    fn piranha_outside_water_becomes_fishbone() {
        let mut game = Game::shell(3, RulesPreset::Classic);
        let mut e = entity("oPiranha", Vec2::new(50.0, 50.0));
        tick(&mut game, &mut e);
        assert!(!e.alive);
        assert!(game.world.entities.iter().any(|e| e.object == "oFishBone"));
    }
    #[test]
    fn fire_frog_death_spawns_its_delayed_bomb() {
        let mut game = Game::shell(4, RulesPreset::Classic);
        let mut e = entity("oFireFrog", Vec2::new(50.0, 50.0));
        e.health = 0;
        tick(&mut game, &mut e);
        assert_eq!(game.world.entities[0].object, "oFireFrogBomb");
    }
    #[test]
    fn monkey_grabs_without_dealing_contact_damage() {
        let mut game = Game::shell(5, RulesPreset::Classic);
        game.player.pos = Vec2::new(50.0, 50.0);
        let mut e = entity("oMonkey", game.player.pos);
        tick(&mut game, &mut e);
        collision(&mut game, &mut e, crate::dispatcher::Instance::Player);
        assert_eq!(e.state, 7);
        assert!((40..=80).contains(&e.timer));
        assert_eq!(game.player.health, 4);
    }

    #[test]
    fn actual_monkey_grab_preserves_eager_money_roll_and_branch_stops() {
        use crate::{InputFrame, Rng, RunState, level::Level};

        #[derive(Clone, Copy, Debug)]
        enum Case {
            InsufficientMoney,
            MoneyTaken,
            MoneyRollFails,
            TripStopsTheft,
        }

        fn grabbed(seed: u32, money: u32) -> (Game, u64) {
            let mut level = Level::default();
            level.starting_bombs = 0;
            level.starting_ropes = 0;
            level.set_tile(6, 7, '@').unwrap();
            for x in 0..crate::level::LEVEL_WIDTH {
                level.set_tile(x, 8, '1').unwrap();
            }
            let mut game = Game::shell(seed, RulesPreset::Classic);
            game.load_custom(&level, "monkey-theft", "");
            game.player.money = money;
            for _ in 0..8 {
                game.step(&InputFrame::default());
            }
            assert!(game.player.grounded);
            let monkey = game.spawn(
                "oMonkey",
                Vec2::new(game.player.pos.x - 8.0, game.player.pos.y - 8.0),
            );
            game.step(&InputFrame::default());
            let actor = game.world.entity(monkey).unwrap();
            assert_eq!(actor.state, 7, "real collision must acquire GRAB");
            assert!((40..=80).contains(&actor.timer));
            assert_eq!(game.player.health, 4);
            (game, monkey)
        }

        // Literal source Step.xml:226..308 branch order, using only the RNG
        // primitive. This oracle evaluates the money roll before combining
        // its result with affordability. Empty bomb/rope inventory keeps the
        // failed-theft path free of item Create draws. Gold Nugget's inherited
        // Create is deterministic; its three launch draws precede release.
        fn source_theft(rng: &mut Rng, money: u32) -> (bool, bool, Option<Vec2>, f64) {
            let trip = 1 + rng.range(4) == 1;
            let mut took_money = false;
            let mut launch = None;
            if !trip {
                let money_roll = 1 + rng.range(10);
                if money >= 500 && money_roll <= 8 {
                    took_money = true;
                    launch = Some(Vec2::new(
                        f64::from(1 + rng.range(3)) - f64::from(1 + rng.range(3)),
                        -f64::from(3 + rng.range(2)),
                    ));
                } else {
                    let _rope_roll = 1 + rng.range(2);
                }
            }
            let bounce_y = -f64::from(2 + rng.range(3));
            (trip, took_money, launch, bounce_y)
        }

        for case in [
            Case::InsufficientMoney,
            Case::MoneyTaken,
            Case::MoneyRollFails,
            Case::TripStopsTheft,
        ] {
            let money = if matches!(case, Case::InsufficientMoney) {
                0
            } else {
                2_000
            };
            // Select an initial seed with the required source branch outcome,
            // after actual Create and GRAB acquisition have consumed their RNG.
            let (mut game, monkey) = (0..64)
                .find_map(|seed| {
                    let (game, monkey) = grabbed(seed, money);
                    let mut preview = game.rng.clone();
                    let trip = 1 + preview.range(4) == 1;
                    let money_roll = if trip { 0 } else { 1 + preview.range(10) };
                    let selected = match case {
                        Case::InsufficientMoney => !trip,
                        Case::MoneyTaken => !trip && money_roll <= 8,
                        Case::MoneyRollFails => !trip && money_roll > 8,
                        Case::TripStopsTheft => trip,
                    };
                    selected.then_some((game, monkey))
                })
                .expect("bounded seed set supplies each source theft branch");
            assert_eq!(game.world.area, Area::Mines);
            let before_wait = game.rng.clone().next_u64();
            for _ in 0..80 {
                if game.world.entity(monkey).unwrap().timer == 0 {
                    break;
                }
                game.step(&InputFrame::default());
                assert_eq!(game.state, RunState::Playing, "{case:?}");
                assert_eq!(game.world.entity(monkey).unwrap().state, 7);
            }
            assert_eq!(game.world.entity(monkey).unwrap().timer, 0);
            assert_eq!(game.rng.clone().next_u64(), before_wait);
            let mut expected_rng = game.rng.clone();
            let (trip, took_money, launch, bounce_y) = source_theft(&mut expected_rng, money);
            game.step(&InputFrame::default());
            assert_eq!(game.state, RunState::Playing, "{case:?}");
            let actor = game.world.entity(monkey).unwrap();
            assert_eq!(actor.state, 1, "{case:?}: expired GRAB must release");
            assert_eq!(actor.vel.y, bounce_y, "{case:?}: release RNG");
            assert_eq!(game.player.health, 4, "{case:?}");
            assert_eq!(game.player.money, money - if took_money { 500 } else { 0 });
            assert_eq!(game.player.is_stunned(), trip, "{case:?}");
            let nuggets: Vec<_> = game
                .world
                .entities
                .iter()
                .filter(|entity| entity.alive && entity.object == "oGoldNugget")
                .collect();
            assert_eq!(nuggets.len(), usize::from(took_money), "{case:?}");
            if let Some(launch) = launch {
                assert_eq!(nuggets[0].vel, launch, "{case:?}: source launch RNG");
                assert_eq!(nuggets[0].value("canCollect"), 0.0);
            }
            assert_eq!(
                game.rng.next_u64(),
                expected_rng.next_u64(),
                "{case:?}: next RNG after source theft and common release"
            );
        }
    }

    #[test]
    fn mantrap_eating_is_lethal_and_then_sleeps() {
        let mut game = Game::shell(6, RulesPreset::Classic);
        for x in 0..30 {
            game.world.set_tile(x, 4, "oBlock");
        }
        game.player.pos = Vec2::new(50.0, 56.0);
        let id = game.world.spawn("oManTrap", Vec2::new(42.0, 48.0));
        game.world.entity_mut(id).unwrap().vel = Vec2::default();
        game.step(&crate::InputFrame::default());
        assert_eq!(game.world.entity(id).unwrap().state, 97);
        assert_eq!(game.player.health, -99);
        // Automatic collision follows the player's Normal Step. Its dead
        // latch is observed on the following frame; animation still advances.
        let eat_ticks = crate::catalog::sprite_frame_count("sManTrapEatL") * 2;
        for _ in 1..eat_ticks {
            game.step(&crate::InputFrame::default());
        }
        assert_eq!(game.state, crate::RunState::Dead);
        assert_eq!(game.world.entity(id).unwrap().state, 96);
        for _ in 0..crate::catalog::sprite_frame_count("sManTrapSleepL") * 2 {
            game.step(&crate::InputFrame::default());
        }
        assert_eq!(game.world.entity(id).unwrap().state, 98);
    }
    #[test]
    fn jaws_has_forty_health_and_turn_pause() {
        let mut game = Game::shell(7, RulesPreset::Classic);
        pond(&mut game);
        game.player.pos = Vec2::new(80.0, 50.0);
        let mut e = entity("oJaws", Vec2::new(50.0, 50.0));
        assert_eq!(e.health, 40);
        tick(&mut game, &mut e);
        tick(&mut game, &mut e);
        assert_eq!(e.state, 3);
        let turn_sprite = e.sprite.clone();
        animation_end(&mut game, &mut e, &turn_sprite);
        assert_eq!(e.state, 2);
        assert_eq!(e.timer, 40);
    }
    #[test]
    fn inherited_water_conversion_retains_hp_zero_old_child_bomb_branch() {
        let mut game = Game::shell(8, RulesPreset::Classic);
        pond(&mut game);
        let mut e = entity("oFireFrog", Vec2::new(50.0, 50.0));
        e.health = 0;
        e.state = 2;
        game.world.entities.push(e);
        super::super::tick(&mut game);
        let frog = game
            .world
            .entities
            .iter()
            .find(|e| e.alive && e.object == "oFrog")
            .unwrap();
        assert_eq!(frog.state, 2);
        assert_eq!(frog.health, 1);
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|e| e.object == "oFireFrogBomb")
                .count(),
            1,
            "Water destroys the old receiver but does not abort its hp<1 child action"
        );
    }

    #[test]
    fn monkey_event_contact_uses_four_pixel_horizontal_gate() {
        let mut game = Game::shell(9, RulesPreset::Classic);
        let mut e = Entity::new(999, "oMonkey", Vec2::new(42.0, 42.0));
        super::super::init_entity(&mut e);
        game.player.pos = Vec2::new(55.0, 50.0);
        game.player.state = PlayerState::Standing;
        assert!(event_bounds(&e).intersects(game.player.bounds()));
        collision(&mut game, &mut e, crate::dispatcher::Instance::Player);
        assert_ne!(e.state, 7);
        assert_eq!(game.player.health, 4);
    }

    #[test]
    fn monkey_source_stomp_precedes_grab_even_on_upward_jump() {
        let mut game = Game::shell(10, RulesPreset::Classic);
        let mut e = Entity::new(999, "oMonkey", Vec2::new(42.0, 40.0));
        super::super::init_entity(&mut e);
        game.player.pos = Vec2::new(50.0, 41.0);
        game.player.state = PlayerState::Jumping;
        game.player.vel.y = -1.0;
        collision(&mut game, &mut e, crate::dispatcher::Instance::Player);
        assert_ne!(e.state, 7);
        assert_eq!(e.health, 0);
        assert_eq!(game.player.health, 4);
        assert_eq!(game.player.vel.y, -5.8);
    }

    #[test]
    fn monkey_grab_anchors_source_y_plus_eight() {
        let mut game = Game::shell(11, RulesPreset::Classic);
        game.player.pos = Vec2::new(50.0, 50.0);
        game.player.state = PlayerState::Standing;
        let mut e = Entity::new(999, "oMonkey", Vec2::new(42.0, 39.0));
        super::super::init_entity(&mut e);
        tick(&mut game, &mut e);
        collision(&mut game, &mut e, crate::dispatcher::Instance::Player);
        assert_eq!(e.state, 7);
        assert_eq!(e.pos.y, 40.0);
        assert_eq!(e.value("grab_y"), -10.0);
        game.player.pos.y += 5.0;
        tick(&mut game, &mut e);
        assert_eq!(e.pos.y, 45.0);
    }

    #[test]
    fn mantrap_source_stomp_runs_before_eating_and_stun_prevents_it() {
        for (spikes, stun, expected_health) in [(false, 0, -99), (true, 0, 4), (true, 1, -99)] {
            let mut game = Game::shell(12, RulesPreset::Classic);
            for x in 0..10 {
                game.world.set_tile(x, 4, "oBlock");
            }
            let mut e = Entity::new(999, "oManTrap", Vec2::new(48.0, 48.0));
            super::super::init_entity(&mut e);
            e.vel.x = 0.0;
            game.player.pos = Vec2::new(56.0, 52.0);
            game.player.state = PlayerState::Falling;
            game.player.vel.y = -1.0;
            game.player.stun = stun;
            if spikes {
                game.player.equipment.insert("SpikeShoes".into());
            }
            collision(&mut game, &mut e, crate::dispatcher::Instance::Player);
            assert_eq!(
                game.player.health, expected_health,
                "spikes={spikes} stun={stun}"
            );
            if spikes && stun == 0 {
                assert_eq!(e.health, 0);
            }
        }
    }

    #[test]
    fn swimming_npc_overrides_fish_player_target_and_retargets_each_tick() {
        let mut game = Game::shell(13, RulesPreset::Classic);
        pond(&mut game);
        game.player.pos = Vec2::new(60.0, 50.0);
        let first = game.world.spawn("oCaveman", Vec2::new(70.0, 42.0));
        let second = game.world.spawn("oCaveman", Vec2::new(100.0, 42.0));
        for id in [first, second] {
            game.world
                .entity_mut(id)
                .unwrap()
                .data
                .insert("swimming".into(), 1.0);
        }
        let mut e = entity("oPiranha", Vec2::new(50.0, 50.0));
        tick(&mut game, &mut e);
        assert_eq!(e.state, 3);
        assert_eq!(e.value("fish_target") as u64, first);
        game.world.entity_mut(first).unwrap().pos.x = 140.0;
        tick(&mut game, &mut e);
        assert_eq!(e.value("fish_target") as u64, second);
    }

    #[test]
    fn mantrap_ground_friction_applies_to_walking_as_in_source() {
        let mut game = Game::shell(14, RulesPreset::Classic);
        game.player.pos = Vec2::new(200.0, 200.0);
        for x in 0..10 {
            game.world.set_tile(x, 4, "oBlock");
        }
        let mut e = Entity::new(999, "oManTrap", Vec2::new(48.0, 48.0));
        super::super::init_entity(&mut e);
        e.state = 1;
        e.vel = Vec2::default();
        e.facing = 1;
        tick(&mut game, &mut e);
        assert_eq!(e.vel.x, 0.3);
    }

    #[test]
    fn jaws_turn_uses_source_right_origin_change_at_animation_end() {
        let mut game = Game::shell(15, RulesPreset::Classic);
        pond(&mut game);
        let mut e = Entity::new(999, "oJaws", Vec2::new(48.0, 48.0));
        super::super::init_entity(&mut e);
        e.state = 1;
        game.player.pos = Vec2::new(80.0, 64.0);
        tick(&mut game, &mut e);
        assert_eq!(e.state, 3);
        assert_eq!(e.pos.x, 48.0);
        let turn_sprite = e.sprite.clone();
        animation_end(&mut game, &mut e, &turn_sprite);
        assert_eq!(e.state, 2);
        assert_eq!(e.pos.x, 96.0);
        // Animation End changes the sprite; the next Step updates the
        // movement rectangle, exactly as the source event ordering does.
        assert_eq!(e.bounds().x, 96.0);
        tick(&mut game, &mut e);
        assert_eq!(e.bounds().x, 48.0);
    }
    #[test]
    fn source_postmove_crush_executes_fire_frog_bomb_only_once() {
        let mut game = Game::shell(16, RulesPreset::Classic);
        game.player.pos = Vec2::new(200.0, 200.0);
        game.world.set_tile(3, 3, "oBlock");
        game.world.spawn("oFireFrog", Vec2::new(48.0, 48.0));
        super::super::tick(&mut game);
        super::super::tick(&mut game);
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|e| e.object == "oFireFrogBomb")
                .count(),
            1
        );
    }

    #[test]
    fn jaws_dehydration_drops_four_crates_at_source_offset_once() {
        let mut game = Game::shell(17, RulesPreset::Classic);
        game.player.pos = Vec2::new(200.0, 200.0);
        let id = game.world.spawn("oJaws", Vec2::new(48.0, 48.0));
        game.world.entity_mut(id).unwrap().health = 1;
        super::super::tick(&mut game);
        super::super::tick(&mut game);
        let crates: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|e| e.object == "oCrate")
            .collect();
        assert_eq!(crates.len(), 4);
        assert!(crates.iter().all(|e| e.pos == Vec2::new(64.0, 64.0)));
    }

    #[test]
    fn held_mantrap_regurgitates_live_unarmed_shopkeeper_after_stun() {
        let mut game = Game::shell(18, RulesPreset::Classic);
        game.player.pos = Vec2::new(200.0, 200.0);
        let id = game.world.spawn("oManTrap", Vec2::new(48.0, 48.0));
        let trap = game.world.entity_mut(id).unwrap();
        trap.held = true;
        trap.state = 98;
        trap.timer = 0;
        trap.data.insert("ate_shopkeeper".into(), 1.0);
        game.player.held = Some(id);
        super::super::tick(&mut game);
        assert!(game.world.entity(id).is_none());
        let keeper = game
            .world
            .entities
            .iter()
            .find(|e| e.alive && e.object == "oShopkeeper")
            .unwrap();
        assert_eq!(keeper.state, 2);
        assert_eq!(keeper.value("has_gun"), 0.0);
        assert!(keeper.health > 0);
    }

    #[test]
    fn stunned_mantrap_uses_source_bounce_and_shorter_movement_rectangle() {
        let mut game = Game::shell(19, RulesPreset::Classic);
        game.player.pos = Vec2::new(200.0, 200.0);
        for x in 0..10 {
            game.world.set_tile(x, 4, "oBlock");
        }
        let mut e = Entity::new(999, "oManTrap", Vec2::new(48.0, 48.0));
        super::super::init_entity(&mut e);
        e.state = 98;
        e.timer = 10;
        e.vel = Vec2::new(2.0, 4.0);
        tick(&mut game, &mut e);
        assert_eq!(e.value("collision_top"), 6.0);
        assert_eq!(e.vel.y, -2.3);
        assert!((e.vel.x - 0.18).abs() < 1e-10);
    }
    #[test]
    fn jaws_idle_water_edge_turn_is_overridden_by_swimming_attack() {
        let mut game = Game::shell(20, RulesPreset::Classic);
        pond(&mut game);
        game.player.pos = Vec2::new(80.0, 64.0);
        let mut e = Entity::new(999, "oJaws", Vec2::new(16.0, 48.0));
        super::super::init_entity(&mut e);
        tick(&mut game, &mut e);
        assert_eq!(e.state, 1);
        assert_eq!(e.sprite, "sJawsTurnR");
        assert_eq!(e.pos.x, 16.0);
        tick(&mut game, &mut e);
        assert_eq!(e.pos.x, 19.0);
        assert_eq!(e.state, 1);
        let turn_sprite = e.sprite.clone();
        animation_end(&mut game, &mut e, &turn_sprite);
        assert_eq!(e.state, 2);
        assert_eq!(e.sprite, "sJawsRight");
        assert_eq!(e.timer, 40);
        assert_eq!(e.pos.x, 67.0);
    }
    fn collision_room() -> Game {
        let mut game = Game::shell(123, RulesPreset::Classic);
        game.world = World::new(40, 30, Area::Jungle, 5);
        game.player.pos = Vec2::new(40.0, 152.0);
        for x in 0..40 {
            game.world.set_tile(x, 10, "oBlock");
        }
        game
    }

    #[test]
    fn game_step_mantrap_eats_npc_families_outside_ai_viewport() {
        for (family, sprite) in [
            ("oCaveman", "sManTrapEatCavemanL"),
            ("oDamsel", "sManTrapEatDamselL"),
            ("oShopkeeper", "sManTrapEatShopkeeperL"),
            ("oShopkeeper2", "sManTrapEatShopkeeperL"),
        ] {
            let mut game = collision_room();
            let trap = game.world.spawn("oManTrap", Vec2::new(370.0, 144.0));
            let victim = game.world.spawn(family, Vec2::new(374.0, 144.0));
            assert!(370.0 >= game.camera.x + crate::VIEW_WIDTH + 4.0);
            game.step(&crate::InputFrame::default());
            assert!(game.world.entity(victim).is_none(), "{family}");
            let trap = game.world.entity(trap).unwrap();
            assert_eq!(trap.pos, Vec2::new(370.0, 144.0), "AI remains frozen");
            assert_eq!(trap.state, 97, "{family}");
            assert_eq!(trap.sprite, sprite, "{family}");
            assert_eq!(trap.facing, 1, "NPC compares origins, not center");
        }
    }

    #[test]
    fn game_step_mantrap_drops_inherited_keeper_gun_at_victim_origin_with_source_launch() {
        let mut game = collision_room();
        let trap = game.world.spawn("oManTrap", Vec2::new(370.0, 144.0));
        let victim = game.world.spawn("oShopkeeper2", Vec2::new(374.0, 144.0));
        game.world.entity_mut(victim).unwrap().vel.x = -2.0;
        game.world
            .entity_mut(victim)
            .unwrap()
            .data
            .insert("has_gun".into(), 1.0);
        let mut rng = game.rng.clone();
        let expected_y = f64::from(4 + rng.range(3));
        let expected_x = -f64::from(4 + rng.range(3));
        game.step(&crate::InputFrame::default());
        assert!(game.world.entity(victim).is_none());
        assert_eq!(
            game.world.entity(trap).unwrap().value("ate_shopkeeper"),
            1.0
        );
        let guns: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|e| e.object == "oShotgun")
            .collect();
        assert_eq!(guns.len(), 1);
        assert_eq!(guns[0].pos, Vec2::new(382.0, 152.0));
        assert_eq!(guns[0].vel, Vec2::new(expected_x, expected_y));
        assert_eq!(guns[0].value("cost"), 0.0);
        assert_eq!(guns[0].value("for_sale"), 0.0);
    }

    #[test]
    fn game_step_mantrap_player_collision_runs_outside_ai_viewport() {
        let mut game = collision_room();
        game.player.pos = Vec2::new(378.0, 152.0);
        let trap = game.world.spawn("oManTrap", Vec2::new(370.0, 144.0));
        assert!(370.0 >= game.camera.x + crate::VIEW_WIDTH + 4.0);
        game.step(&crate::InputFrame::default());
        assert_eq!(game.player.health, -99);
        let trap = game.world.entity(trap).unwrap();
        assert_eq!(trap.state, 97);
        assert_eq!(trap.pos, Vec2::new(370.0, 144.0));
        game.step(&crate::InputFrame::default());
        assert_eq!(game.state, crate::RunState::Dead);
    }

    #[test]
    fn game_step_newly_dropped_keeper_gun_does_not_interrupt_eating_in_same_collision_phase() {
        let mut game = collision_room();
        let trap = game.world.spawn("oManTrap", Vec2::new(144.0, 144.0));
        let victim = game.world.spawn("oShopkeeper", Vec2::new(144.0, 144.0));
        game.step(&crate::InputFrame::default());
        assert!(game.world.entity(victim).is_none());
        let trap = game.world.entity(trap).unwrap();
        assert_eq!(trap.state, 97);
        assert_eq!(trap.health, 3);
        assert_eq!(trap.sprite, "sManTrapEatShopkeeperL");
        assert!(game.world.entities.iter().any(|e| e.object == "oShotgun"));
    }

    #[test]
    fn game_step_jaws_turn_animation_finishes_after_natural_camera_scroll_freezes_ai() {
        let mut game = Game::shell(123, RulesPreset::Classic);
        game.world = World::new(50, 30, Area::Jungle, 5);
        game.player.pos = Vec2::new(228.0, 152.0);
        game.camera = Vec2::new(100.0, 0.0);
        for x in 0..50 {
            game.world.set_tile(x, 10, "oBlock");
        }
        for x in 29..42 {
            for y in 8..10 {
                game.world.set_tile(x, y, "oWaterSwim");
            }
        }
        let jaws = game.world.spawn("oJaws", Vec2::new(464.0, 128.0));
        game.step(&crate::InputFrame {
            left: true,
            run: true,
            ..Default::default()
        });
        assert_eq!(game.world.entity(jaws).unwrap().sprite, "sJawsTurnR");
        for tick in 2..=13 {
            game.step(&crate::InputFrame {
                left: tick <= 5,
                run: tick <= 5,
                ..Default::default()
            });
        }
        let jaws = game.world.entity(jaws).unwrap();
        assert!(464.0 >= game.camera.x + crate::VIEW_WIDTH + 48.0);
        assert_eq!(jaws.sprite, "sJawsRight");
        assert_eq!(jaws.state, 2);
        assert_eq!(jaws.timer, 40);
        assert_eq!(jaws.pos, Vec2::new(512.0, 128.0));
    }
    #[test]
    fn game_step_jaws_death_runs_blood_then_bone_creates_before_crate_launch_rng() {
        let mut game = Game::shell(17, RulesPreset::Classic);
        game.world = World::new(20, 20, Area::Jungle, 5);
        game.player.pos = Vec2::new(200.0, 200.0);
        let jaws = game.world.spawn("oJaws", Vec2::new(48.0, 48.0));
        game.world.entity_mut(jaws).unwrap().health = 0;
        let mut rng = game.rng.clone();
        let blood_pos = Vec2::new(
            70.0 + f64::from(rng.range(5)),
            62.0 + f64::from(rng.range(5)),
        );
        // scrCreateBlood receives one shared coordinate pair. Each immediate
        // oBlood Create consumes its three random() and one rand() draw.
        for _ in 0..4 {
            rng.unit();
            rng.unit();
            rng.unit();
            rng.range(6);
        }
        let mut bones = Vec::new();
        for _ in 0..4 {
            let pos = Vec2::new(
                70.0 + f64::from(rng.range(5)),
                62.0 + f64::from(rng.range(7)),
            );
            let velocity = Vec2::new(rng.unit() * 4.0 - rng.unit() * 4.0, -1.0 - rng.unit() * 2.0);
            let gravity = f64::from(1 + rng.range(6)) * 0.1;
            bones.push((pos, velocity, gravity));
        }
        let crates: Vec<_> = (0..4)
            .map(|_| {
                Vec2::new(
                    f64::from(rng.range(4)) - f64::from(rng.range(4)),
                    -f64::from(1 + rng.range(2)),
                )
            })
            .collect();
        // Jaws Step continues after Destroy: dry IDLE turns right and drops
        // one pixel, then its due Bubble Create draws rand(1,3) after all loot.
        let bubble_velocity = -f64::from(1 + rng.range(3)) * 0.1;
        game.step(&crate::InputFrame::default());
        assert!(game.world.entity(jaws).is_none());
        let effects: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|e| matches!(e.object.as_str(), "oBlood" | "oBone" | "oCrate"))
            .collect();
        assert_eq!(
            effects
                .iter()
                .map(|e| e.object.as_str())
                .collect::<Vec<_>>(),
            [
                "oBlood", "oBlood", "oBlood", "oBlood", "oBone", "oBone", "oBone", "oBone",
                "oCrate", "oCrate", "oCrate", "oCrate",
            ]
        );
        for blood in &effects[..4] {
            assert_eq!(blood.previous_pos, blood_pos);
            assert_eq!(blood.value("world_effect_initialized"), 1.0);
        }
        let bone_ids: Vec<_> = effects[4..8].iter().map(|e| e.id).collect();
        // These source resources precede Jaws in the Normal Step pass, so
        // newborns retain their synchronous Create state until the next frame.
        for (bone, (pos, velocity, gravity)) in effects[4..8].iter().zip(&bones) {
            assert_eq!(bone.previous_pos, *pos);
            assert_eq!(bone.vel, *velocity);
            assert_eq!(bone.value("gravity"), *gravity);
            assert_eq!(bone.value("life"), 60.0);
        }
        for (item, velocity) in effects[8..].iter().zip(crates) {
            assert_eq!(item.pos, Vec2::new(64.0, 64.0));
            assert_eq!(item.vel, velocity);
        }
        let bubble = game
            .world
            .entities
            .iter()
            .find(|e| e.alive && e.object == "oBubble")
            .expect("post-Destroy Jaws Step must create its due Bubble");
        let bubble_id = bubble.id;
        assert!(effects.last().unwrap().id < bubble_id);
        // Bubble504 precedes Jaws885, so it retains Create state this frame.
        assert_eq!(bubble.pos, Vec2::new(48.0, 65.0));
        assert_eq!(bubble.vel, Vec2::new(0.0, bubble_velocity));
        assert_eq!(game.rng.next_u64(), rng.next_u64());
        game.step(&crate::InputFrame::default());
        // Its next own Step rises once, then destroys it outside water.
        assert!(game.world.entity(bubble_id).is_none());
        for (id, (_, velocity, gravity)) in bone_ids.into_iter().zip(bones) {
            let bone = game.world.entity(id).unwrap();
            assert_eq!(bone.vel, Vec2::new(velocity.x, velocity.y + gravity));
            assert_eq!(bone.value("life"), 59.0);
        }
    }

    #[test]
    fn mantrap_collision_acts_only_on_the_supplied_npc_pair() {
        let mut game = Game::shell(31, RulesPreset::Classic);
        let mut trap = entity("oManTrap", Vec2::new(56.0, 56.0));
        let first = game.world.spawn("oCaveman", trap.pos);
        let second = game.world.spawn("oDamsel", trap.pos);
        assert!(collision(
            &mut game,
            &mut trap,
            crate::dispatcher::Instance::Entity(second)
        ));
        assert!(game.world.entity(first).unwrap().alive);
        assert!(game.world.entity(second).is_none());
        assert_eq!(trap.sprite, "sManTrapEatDamselL");
    }

    #[test]
    fn monkey_collision_throws_only_the_supplied_item_pair() {
        let mut game = Game::shell(32, RulesPreset::Classic);
        let mut monkey = entity("oMonkey", Vec2::new(56.0, 56.0));
        monkey.data.insert("throw_counter".into(), 0.0);
        let first = game.world.spawn("oRock", monkey.pos);
        let second = game.world.spawn("oRock", monkey.pos);
        assert!(collision(
            &mut game,
            &mut monkey,
            crate::dispatcher::Instance::Entity(second)
        ));
        assert_eq!(game.world.entity(first).unwrap().vel, Vec2::default());
        assert_eq!(game.world.entity(second).unwrap().vel, Vec2::new(5.0, -4.0));
        assert_eq!(monkey.value("throw_counter"), 60.0);
    }

    #[test]
    fn monkey_item_collision_respects_source_active_property() {
        let mut game = Game::shell(32, RulesPreset::Classic);
        let mut monkey = entity("oMonkey", Vec2::new(56.0, 56.0));
        monkey.data.insert("throw_counter".into(), 0.0);
        let rock = game.world.spawn("oRock", monkey.pos);
        game.world
            .entity_mut(rock)
            .unwrap()
            .data
            .insert("active".into(), 0.0);
        assert!(collision(
            &mut game,
            &mut monkey,
            crate::dispatcher::Instance::Entity(rock)
        ));
        assert_eq!(game.world.entity(rock).unwrap().vel, Vec2::default());
        assert_eq!(monkey.value("throw_counter"), 0.0);
    }

    #[test]
    fn mantrap_spike_stomp_remains_eligible_while_already_stunned() {
        let mut game = Game::shell(33, RulesPreset::Classic);
        let mut trap = entity("oManTrap", Vec2::new(56.0, 56.0));
        trap.state = 98;
        game.player.pos = Vec2::new(56.0, trap.pos.y + 4.0);
        game.player.state = PlayerState::Falling;
        game.player.equipment.insert("SpikeShoes".into());
        collision(&mut game, &mut trap, crate::dispatcher::Instance::Player);
        assert_eq!(trap.health, 0);
        assert_eq!(game.player.health, 4);
        assert!(trap.alive, "death belongs to the next child Step");
        assert_eq!(game.lifetime_stats.enemy_kills[10], 0);
    }

    #[test]
    fn mantrap_eating_preserves_image_index_and_sleep_uses_instance_stun_time() {
        let mut game = Game::shell(34, RulesPreset::Classic);
        let mut trap = entity("oManTrap", Vec2::new(56.0, 56.0));
        trap.animation = 1.25;
        game.player.pos = Vec2::new(56.0, 56.0);
        collision(&mut game, &mut trap, crate::dispatcher::Instance::Player);
        assert_eq!(trap.animation, 1.25);
        trap.data.insert("stun_time".into(), 17.0);
        animation_end(&mut game, &mut trap, "sManTrapEatL");
        assert_eq!(trap.state, 96);
        animation_end(&mut game, &mut trap, "sManTrapSleepL");
        assert_eq!(trap.state, 98);
        assert_eq!(trap.timer, 34);
    }

    #[test]
    fn mantrap_lifetime_player_death_obeys_real_level_eligibility() {
        for custom in [false, true] {
            let mut game = Game::shell(35, RulesPreset::Classic);
            if custom {
                game.flags.insert("custom".into());
            }
            let mut trap = entity("oManTrap", Vec2::new(56.0, 56.0));
            game.player.pos = Vec2::new(56.0, 56.0);
            collision(&mut game, &mut trap, crate::dispatcher::Instance::Player);
            assert_eq!(game.player.health, -99);
            assert_eq!(game.lifetime_stats.enemy_deaths[10], u64::from(!custom));
        }
    }

    #[test]
    fn fish_targeting_uses_the_other_instances_stored_swimming_flag() {
        for object in ["oPiranha", "oJaws"] {
            let mut game = Game::shell(36, RulesPreset::Classic);
            pond(&mut game);
            game.player.pos = Vec2::new(70.0, 64.0);
            game.player.data.insert("swimming".into(), 0.0);
            let mut fish = Entity::new(999, object, Vec2::new(48.0, 48.0));
            super::super::init_entity(&mut fish);
            tick(&mut game, &mut fish);
            assert_eq!(fish.state, 0, "{object} must use the stored flag");
            game.player.data.insert("swimming".into(), 1.0);
            tick(&mut game, &mut fish);
            assert_eq!(fish.state, 1, "{object}");
        }
    }

    #[test]
    fn fish_bubble_create_consumes_rng_before_piranha_timer_and_uses_jaws_mouth() {
        for object in ["oPiranha", "oJaws"] {
            let mut game = Game::shell(37, RulesPreset::Classic);
            let mut fish = Entity::new(999, object, Vec2::new(48.0, 48.0));
            super::super::init_entity(&mut fish);
            let mut expected = game.rng.clone();
            let velocity = -f64::from(1 + expected.range(3)) * 0.1;
            let timer = if object == "oPiranha" {
                30.0 + f64::from(expected.range(21))
            } else {
                40.0
            };
            bubbles(&mut game, &mut fish, object == "oPiranha");
            let bubble = game
                .world
                .entities
                .iter()
                .find(|e| e.object == "oBubble")
                .unwrap();
            assert_eq!(bubble.vel.y, velocity);
            assert_eq!(
                bubble.pos,
                Vec2::new(48.0, if object == "oJaws" { 64.0 } else { 48.0 })
            );
            assert_eq!(fish.value("bubble_timer"), timer);
            assert_eq!(game.rng.next_u64(), expected.next_u64());
        }
    }

    #[test]
    fn monkey_stolen_gold_uses_treasure_alarm_instead_of_age_delay() {
        let mut found = false;
        for seed in 0..32 {
            let mut game = Game::shell(seed, RulesPreset::Classic);
            game.player.money = 1000;
            let monkey = entity("oMonkey", Vec2::new(56.0, 56.0));
            monkey_steal(&mut game, &monkey);
            if let Some(gold) = game
                .world
                .entities
                .iter()
                .find(|e| e.object == "oGoldNugget")
            {
                assert_eq!(gold.value("canCollect"), 0.0);
                assert_eq!(gold.alarms[0], 20);
                assert!(!gold.data.contains_key("collect_delay"));
                found = true;
                break;
            }
        }
        assert!(found, "fixture must reach the source gold theft branch");
    }

    #[test]
    fn game_step_monkey_trip_drops_reserved_item_and_restores_the_stash() {
        let mut game = collision_room();
        game.player.pos = Vec2::new(56.0, 152.0);
        let rock = game.spawn("oRock", game.player.pos);
        let item = game.world.entity_mut(rock).unwrap();
        item.held = true;
        item.data.insert("stashed".into(), 1.0);
        game.player.data.insert("stash_id".into(), rock as f64);
        let bomb = game.spawn("oBomb", game.player.pos);
        game.world.entity_mut(bomb).unwrap().held = true;
        game.player.held = Some(bomb);
        let monkey = game.spawn("oMonkey", Vec2::new(48.0, 144.0));
        // Establish GRAB through the actual automatic Character collision pair.
        game.step(&crate::InputFrame::default());
        assert_eq!(game.world.entity(monkey).unwrap().state, 7);
        game.world.entity_mut(monkey).unwrap().timer = 0;
        let trip_seed = (0..32)
            .find(|seed| crate::Rng::new(*seed).chance(1, 4))
            .unwrap();
        game.rng = crate::Rng::new(trip_seed);
        game.step(&crate::InputFrame::default());
        assert_eq!(game.player.stun, 40);
        let fresh = game.player.held.unwrap();
        assert_ne!(fresh, rock);
        assert!(game.world.entity(rock).is_none());
        assert_eq!(game.world.entity(fresh).unwrap().object, "oRock");
        assert!(!game.player.data.contains_key("stash_id"));
        let item = game.world.entity(bomb).unwrap();
        assert!(!item.held);
        assert_eq!(item.vel, Vec2::default());
        let item = game.world.entity(fresh).unwrap();
        assert!(item.held);
        assert!(!item.data.contains_key("stashed"));
    }

    #[test]
    fn game_step_frog_death_and_blood_wait_for_its_source_viewport_step() {
        let mut game = collision_room();
        let id = game.world.spawn("oFrog", Vec2::new(370.0, 144.0));
        game.world.entity_mut(id).unwrap().health = 0;
        game.step(&crate::InputFrame::default());
        assert!(game.world.entity(id).unwrap().alive);
        assert_eq!(game.lifetime_stats.enemy_kills[8], 0);
        assert!(!game.world.entities.iter().any(|e| e.object == "oBlood"));
        game.world.entity_mut(id).unwrap().pos.x = 100.0;
        game.step(&crate::InputFrame::default());
        assert!(game.world.entity(id).is_none());
        assert_eq!(game.lifetime_stats.enemy_kills[8], 1);
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|e| e.object == "oBlood")
                .count(),
            3
        );
        game.step(&crate::InputFrame::default());
        assert_eq!(game.lifetime_stats.enemy_kills[8], 1);
    }

    #[test]
    fn game_step_mantrap_stomp_credits_death_on_next_normal_step() {
        let mut game = collision_room();
        game.player.pos = Vec2::new(56.0, 139.0);
        game.player.state = PlayerState::Falling;
        game.player.vel.y = 1.0;
        game.player.equipment.insert("SpikeShoes".into());
        let id = game.spawn("oManTrap", Vec2::new(48.0, 144.0));
        game.world.entity_mut(id).unwrap().vel = Vec2::default();
        game.step(&crate::InputFrame::default());
        assert_eq!(game.world.entity(id).unwrap().health, 0);
        assert_eq!(game.lifetime_stats.enemy_kills[10], 0);
        game.step(&crate::InputFrame::default());
        assert!(game.world.entity(id).is_none());
        assert_eq!(game.lifetime_stats.enemy_kills[10], 1);
        game.step(&crate::InputFrame::default());
        assert_eq!(game.lifetime_stats.enemy_kills[10], 1);
    }

    #[test]
    fn jungle_create_randomizes_source_facing_once_and_latches_origin_water() {
        for object in ["oFrog", "oFireFrog", "oMonkey", "oPiranha"] {
            let mut game = Game::shell(39, RulesPreset::Classic);
            game.world.set_tile(3, 3, "oWaterSwim");
            let mut actor = Entity::new(999, object, Vec2::new(48.0, 48.0));
            super::super::init_entity(&mut actor);
            let mut expected = game.rng.clone();
            let facing = if expected.range(2) == 0 { -1 } else { 1 };
            create(&mut game, &mut actor);
            assert_eq!(actor.facing, facing, "{object}");
            if object != "oPiranha" {
                assert_eq!(actor.value("swimming"), 1.0, "{object}");
            }
            create(&mut game, &mut actor);
            assert_eq!(game.rng.next_u64(), expected.next_u64(), "{object}");
        }
    }

    #[test]
    fn local_jungle_create_preserves_producer_overrides_at_runtime_bootstrap() {
        for object in ["oFrog", "oFireFrog", "oMonkey", "oPiranha"] {
            let mut game = Game::shell(43, RulesPreset::Classic);
            game.world.set_tile(3, 3, "oWaterSwim");
            let mut actor = Entity::new(999, object, Vec2::new(48.0, 48.0));
            super::super::init_entity(&mut actor);
            let mut expected = game.rng.clone();
            let facing = if expected.range(2) == 0 { -1 } else { 1 };
            create_local(
                &mut crate::creation::EntityCreateContext {
                    world: &game.world,
                    rng: &mut game.rng,
                },
                &mut actor,
            );
            assert_eq!(actor.facing, facing, "{object}");
            assert_eq!(actor.value("swimming"), f64::from(object != "oPiranha"));
            actor.facing = -facing;
            actor.state = 3;
            actor.timer = 88;
            actor.health = 17;
            actor.data.insert("swimming".into(), 0.0);
            create_local(
                &mut crate::creation::EntityCreateContext {
                    world: &game.world,
                    rng: &mut game.rng,
                },
                &mut actor,
            );
            create(&mut game, &mut actor);
            assert_eq!(actor.facing, -facing, "{object}");
            assert_eq!((actor.state, actor.timer, actor.health), (3, 88, 17));
            assert_eq!(actor.value("swimming"), 0.0);
            assert_eq!(game.rng.next_u64(), expected.next_u64(), "{object}");
        }
    }

    #[test]
    fn generated_jungle_spawn_runs_local_create_before_producer_overrides() {
        for object in ["oFrog", "oFireFrog", "oMonkey", "oPiranha"] {
            let mut game = Game::shell(44, RulesPreset::Classic);
            game.world.set_tile(3, 3, "oWaterSwim");
            let mut expected = game.rng.clone();
            let facing = if expected.range(2) == 0 { -1 } else { 1 };
            let id = crate::creation::spawn_generated_entity(
                &mut game.world,
                &mut game.rng,
                object,
                Vec2::new(48.0, 48.0),
            );
            let actor = game.world.entity_mut(id).unwrap();
            assert_eq!(actor.value("jungle_context_created"), 1.0);
            assert_eq!(actor.value("enemy_context_created"), 0.0);
            assert_eq!(actor.facing, facing);
            assert_eq!(actor.value("swimming"), f64::from(object != "oPiranha"));
            actor.facing = -facing;
            actor.state = 2;
            actor.health = 5;
            super::super::create_instance(&mut game, id);
            super::super::create_instance(&mut game, id);
            let actor = game.world.entity(id).unwrap();
            assert_eq!((actor.facing, actor.state, actor.health), (-facing, 2, 5));
            assert_eq!(game.rng.next_u64(), expected.next_u64(), "{object}");
        }
    }

    #[test]
    fn shortcut_game_api_generates_jungle_creates_before_runtime_bootstrap() {
        let mut game = Game::shell(45, RulesPreset::Classic);
        game.start_from_hub(false, 5);
        let actors: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|e| matches(&e.object))
            .map(|e| (e.id, e.facing, e.state))
            .collect();
        assert!(!actors.is_empty(), "fixture must generate a jungle family");
        for (id, _, _) in &actors {
            assert_eq!(
                game.world
                    .entity(*id)
                    .unwrap()
                    .value("jungle_context_created"),
                1.0
            );
        }
        let mut expected = game.rng.clone();
        for (id, facing, state) in actors {
            super::super::create_instance(&mut game, id);
            let actor = game.world.entity(id).unwrap();
            assert_eq!((actor.facing, actor.state), (facing, state));
        }
        assert_eq!(game.rng.next_u64(), expected.next_u64());
    }

    #[test]
    fn game_step_piranha_alarm_reopens_bite_latch_before_normal_step() {
        let mut game = collision_room();
        game.world.set_tile(23, 9, "oWaterSwim");
        let id = game.spawn("oPiranha", Vec2::new(370.0, 144.0));
        let fish = game.world.entity_mut(id).unwrap();
        fish.state = 3;
        fish.data.insert("can_bite".into(), 0.0);
        fish.alarms[0] = 1;
        game.step(&crate::InputFrame::default());
        let fish = game.world.entity(id).unwrap();
        assert_eq!(fish.state, 2, "no eligible NPC target");
        assert_eq!(
            fish.value("can_bite"),
            1.0,
            "Alarm0 runs independently before Step"
        );
    }

    // Literal Create oracle: random(4), random(4), random(2), rand(1,6).
    // Deliberately does not call the port's detritus or ManTrap helpers.
    fn mantrap_detritus_oracle(rng: &mut crate::Rng) -> (Vec2, f64) {
        let velocity = Vec2::new(rng.unit() * 4.0 - rng.unit() * 4.0, -1.0 - rng.unit() * 2.0);
        (velocity, f64::from(1 + rng.range(6)) * 0.1)
    }

    #[test]
    fn mantrap_own_step_exact_real_phases_and_eager_tail_rng() {
        for state in [97, 96] {
            for phase in [6.0, 6.5, 8.0, 8.5] {
                for facing in [-1, 1] {
                    for succeeds in [false, true] {
                        let seed = (0..64)
                            .find(|seed| (crate::Rng::new(*seed).range(8) == 0) == succeeds)
                            .unwrap();
                        let mut game = collision_room();
                        let mut trap = Entity::new(999, "oManTrap", Vec2::new(112.0, 144.0));
                        super::super::init_entity(&mut trap);
                        trap.state = state;
                        trap.set_sprite(if state == 97 {
                            "sManTrapEatL"
                        } else {
                            "sManTrapSleepL"
                        });
                        trap.animation = phase;
                        trap.facing = facing;
                        trap.vel = Vec2::default();
                        game.rng = crate::Rng::new(seed);
                        let mut expected = game.rng.clone();
                        let blood = state == 97 && phase == 8.0;
                        let blood_create = blood.then(|| mantrap_detritus_oracle(&mut expected));
                        let roll = expected.range(8);
                        let bone = state == 96 && phase == 6.0 && roll == 0;
                        let bone_create = bone.then(|| mantrap_detritus_oracle(&mut expected));
                        tick(&mut game, &mut trap);
                        let output: Vec<_> = game
                            .world
                            .entities
                            .iter()
                            .filter(|e| matches!(e.object.as_str(), "oBlood" | "oBone"))
                            .collect();
                        assert_eq!(
                            output.len(),
                            usize::from(blood || bone),
                            "state={state} phase={phase} facing={facing}"
                        );
                        if let Some((velocity, gravity)) = blood_create {
                            assert_eq!(output[0].object, "oBlood");
                            assert_eq!(output[0].pos, Vec2::new(trap.pos.x + 8.0, trap.pos.y));
                            assert_eq!(output[0].vel, velocity);
                            assert_eq!(output[0].value("gravity"), gravity);
                        }
                        if let Some((velocity, gravity)) = bone_create {
                            assert_eq!(output[0].object, "oBone");
                            assert_eq!(
                                output[0].pos,
                                Vec2::new(
                                    trap.pos.x + if facing < 0 { 2.0 } else { 14.0 },
                                    trap.pos.y + 4.0
                                )
                            );
                            assert_eq!(
                                output[0].vel,
                                Vec2::new(f64::from(facing) * 2.0, velocity.y)
                            );
                            assert_eq!(output[0].value("gravity"), gravity);
                            assert_eq!(output[0].value("world_effect_initialized"), 1.0);
                        }
                        assert_eq!(
                            game.rng.next_u64(),
                            expected.next_u64(),
                            "state={state} phase={phase}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn mantrap_own_step_bone_cap_keeps_create_draws_and_skips_destroyed_with_body() {
        for count in [31, 32] {
            for facing in [-1, 1] {
                let mut game = collision_room();
                for _ in 0..count {
                    crate::world_events::spawn_detritus(
                        &mut game,
                        "oBone",
                        Vec2::new(240.0, 144.0),
                    );
                }
                let mut trap = Entity::new(999, "oManTrap", Vec2::new(112.0, 144.0));
                super::super::init_entity(&mut trap);
                trap.state = 96;
                trap.set_sprite("sManTrapSleepL");
                trap.animation = 6.0;
                trap.facing = facing;
                trap.vel = Vec2::default();
                let seed = (0..64)
                    .find(|seed| crate::Rng::new(*seed).range(8) == 0)
                    .unwrap();
                game.rng = crate::Rng::new(seed);
                let mut expected = game.rng.clone();
                assert_eq!(expected.range(8), 0);
                // oDetritus Create destroys the 33rd instance, but does not
                // abort oBone's own Create. Its four draws still happen.
                let (velocity, gravity) = mantrap_detritus_oracle(&mut expected);
                tick(&mut game, &mut trap);
                let bone = game.world.entities.last().unwrap();
                assert_eq!(bone.object, "oBone");
                assert_eq!(game.world.entities.len(), count + 1);
                assert_eq!(bone.alive, count == 31);
                assert_eq!(
                    bone.pos,
                    Vec2::new(
                        trap.pos.x + if facing < 0 { 2.0 } else { 14.0 },
                        trap.pos.y + 4.0,
                    ),
                );
                // The following with(explicit ID) skips a destroyed receiver;
                // only a live Bone receives the producer's xVel override.
                assert_eq!(
                    bone.vel,
                    Vec2::new(
                        if count == 31 {
                            f64::from(facing) * 2.0
                        } else {
                            velocity.x
                        },
                        velocity.y,
                    ),
                );
                assert_eq!(bone.value("gravity"), gravity);
                assert_eq!(bone.value("world_effect_initialized"), 1.0);
                assert_eq!(game.rng.next_u64(), expected.next_u64());
            }
        }
    }

    #[test]
    fn mantrap_own_step_bloodless_and_detritus_cap_precede_tail_roll() {
        for (bloodless, count) in [(true, 0), (false, 15), (false, 16)] {
            let mut game = collision_room();
            for _ in 0..count {
                crate::world_events::spawn_detritus(&mut game, "oBone", Vec2::new(240.0, 144.0));
            }
            let mut trap = Entity::new(999, "oManTrap", Vec2::new(112.0, 144.0));
            super::super::init_entity(&mut trap);
            trap.state = 97;
            trap.set_sprite("sManTrapEatL");
            trap.animation = 8.0;
            trap.vel = Vec2::default();
            trap.data.insert("bloodless".into(), f64::from(bloodless));
            let mut expected = game.rng.clone();
            if !bloodless && count < 16 {
                mantrap_detritus_oracle(&mut expected);
            }
            expected.range(8);
            tick(&mut game, &mut trap);
            assert_eq!(
                game.world
                    .entities
                    .iter()
                    .filter(|e| e.object == "oBlood")
                    .count(),
                usize::from(!bloodless && count < 16)
            );
            assert_eq!(game.rng.next_u64(), expected.next_u64());
        }
    }

    #[test]
    fn mantrap_own_step_idle_countdown_and_pre_multiply_friction() {
        for counter in [1, 2] {
            let mut game = collision_room();
            let mut trap = Entity::new(999, "oManTrap", Vec2::new(112.0, 144.0));
            super::super::init_entity(&mut trap);
            trap.timer = counter;
            trap.vel = Vec2::new(0.2, 0.0);
            let mut expected = game.rng.clone();
            let facing = if counter == 1 {
                Some(if expected.range(2) == 0 { -1 } else { 1 })
            } else {
                None
            };
            expected.range(8);
            tick(&mut game, &mut trap);
            assert_eq!(trap.timer, counter - 1);
            assert_eq!(trap.state, i32::from(counter == 1));
            if let Some(facing) = facing {
                assert_eq!(trap.facing, facing);
            }
            assert_eq!(
                trap.vel.x, 0.06,
                "0.2 passes the source threshold before multiplication"
            );
            assert_eq!(game.rng.next_u64(), expected.next_u64());
        }
    }

    #[test]
    fn mantrap_own_step_first_stunned_landing_blood_latches_before_recovery() {
        let mut game = collision_room();
        let mut trap = Entity::new(999, "oManTrap", Vec2::new(112.0, 144.0));
        super::super::init_entity(&mut trap);
        trap.state = 98;
        trap.timer = 2;
        trap.vel = Vec2::default();
        let mut expected = game.rng.clone();
        mantrap_detritus_oracle(&mut expected);
        expected.range(8);
        tick(&mut game, &mut trap);
        assert_eq!((trap.state, trap.timer), (98, 1));
        assert_eq!(trap.value("bounced"), 1.0);
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|e| e.object == "oBlood")
                .count(),
            1
        );
        assert_eq!(game.rng.clone().next_u64(), expected.clone().next_u64());
        expected.range(8);
        tick(&mut game, &mut trap);
        assert_eq!((trap.state, trap.timer), (98, 0));
        let recovery = 20 + expected.range(31);
        expected.range(8);
        tick(&mut game, &mut trap);
        assert_eq!((trap.state, trap.timer), (0, recovery));
        assert_eq!(trap.sprite, "sManTrapLeft");
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|e| e.object == "oBlood")
                .count(),
            1
        );
        assert_eq!(game.rng.next_u64(), expected.next_u64());
    }

    #[test]
    fn mantrap_own_step_destroy_continues_and_dead_stunned_does_not_recover() {
        for (state, keeper) in [(0, false), (98, false), (98, true)] {
            let mut game = collision_room();
            let mut trap = Entity::new(999, "oManTrap", Vec2::new(112.0, 144.0));
            super::super::init_entity(&mut trap);
            trap.health = 0;
            trap.state = state;
            trap.vel = Vec2::default();
            trap.data.insert("bounced".into(), 1.0);
            trap.data.insert("ate_shopkeeper".into(), f64::from(keeper));
            let mut expected = game.rng.clone();
            // Death's leaf/blood burst, then both literal keeper-release sites.
            for _ in 0..if keeper { 3 } else { 1 } {
                for _ in 0..3 {
                    expected.range(17);
                    expected.range(17);
                }
                mantrap_detritus_oracle(&mut expected);
            }
            let facing = if state == 0 {
                Some(if expected.range(2) == 0 { -1 } else { 1 })
            } else {
                None
            };
            expected.range(8);
            tick(&mut game, &mut trap);
            assert!(!trap.alive);
            assert_eq!(trap.state, if state == 0 { 1 } else { 98 });
            if let Some(facing) = facing {
                assert_eq!(trap.facing, facing);
            }
            assert_eq!(
                game.world
                    .entities
                    .iter()
                    .filter(|e| e.object == "oShopkeeper")
                    .count(),
                if keeper { 2 } else { 0 }
            );
            assert_eq!(
                game.world
                    .entities
                    .iter()
                    .filter(|e| e.object == "oLeaf")
                    .count(),
                if keeper { 9 } else { 3 }
            );
            assert_eq!(
                game.world
                    .entities
                    .iter()
                    .filter(|e| e.object == "oBlood")
                    .count(),
                if keeper { 3 } else { 1 }
            );
            assert_eq!(game.rng.next_u64(), expected.next_u64());
        }
    }

    #[test]
    fn game_step_mantrap_own_destroy_keeps_later_eating_blood_and_rng() {
        let mut game = collision_room();
        let id = game.spawn("oManTrap", Vec2::new(112.0, 144.0));
        let trap = game.world.entity_mut(id).unwrap();
        // Controlled lethal onset during an existing meal; the inherited
        // event sees no lava/other Destroy trigger in this authored room.
        trap.health = 0;
        trap.state = 97;
        trap.set_sprite("sManTrapEatL");
        trap.animation = 8.0;
        trap.vel = Vec2::default();
        let mut expected = game.rng.clone();
        for _ in 0..3 {
            expected.range(17);
            expected.range(17);
        }
        mantrap_detritus_oracle(&mut expected); // hp death Blood
        mantrap_detritus_oracle(&mut expected); // surviving meal tail Blood
        expected.range(8);
        game.step(&crate::InputFrame::default());
        assert!(game.world.entity(id).is_none());
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|e| e.object == "oBlood")
                .count(),
            2
        );
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|e| e.object == "oLeaf")
                .count(),
            3
        );
        assert_eq!(game.lifetime_stats.enemy_kills[10], 1);
        assert_eq!(game.player.health, 4);
        assert_eq!(game.rng.next_u64(), expected.next_u64());
    }

    #[test]
    fn game_step_mantrap_own_tail_is_view_gated_at_real_animation_phase() {
        for (state, phase) in [(97, 8.0), (97, 8.5), (96, 6.0), (96, 6.5)] {
            for offview in [false, true] {
                let mut game = collision_room();
                let id = game.spawn(
                    "oManTrap",
                    Vec2::new(if offview { 370.0 } else { 112.0 }, 144.0),
                );
                let trap = game.world.entity_mut(id).unwrap();
                trap.state = state;
                trap.set_sprite(if state == 97 {
                    "sManTrapEatL"
                } else {
                    "sManTrapSleepL"
                });
                trap.animation = phase;
                trap.vel = Vec2::default();
                let seed = (0..64)
                    .find(|seed| crate::Rng::new(*seed).range(8) == 0)
                    .unwrap();
                game.rng = crate::Rng::new(seed);
                let mut expected = game.rng.clone();
                let blood = !offview && state == 97 && phase == 8.0;
                if blood {
                    mantrap_detritus_oracle(&mut expected);
                }
                let bone = if offview {
                    false
                } else {
                    expected.range(8) == 0 && state == 96 && phase == 6.0
                };
                if bone {
                    mantrap_detritus_oracle(&mut expected);
                }
                game.step(&crate::InputFrame::default());
                assert_eq!(game.player.health, 4);
                assert_eq!(
                    game.world
                        .entities
                        .iter()
                        .filter(|e| e.object == "oBlood")
                        .count(),
                    usize::from(blood)
                );
                assert_eq!(
                    game.world
                        .entities
                        .iter()
                        .filter(|e| e.object == "oBone")
                        .count(),
                    usize::from(bone)
                );
                assert_eq!(
                    game.rng.next_u64(),
                    expected.next_u64(),
                    "state={state} phase={phase} offview={offview}"
                );
            }
        }
    }

    #[test]
    fn game_step_mantrap_actual_npc_meal_emits_blood_then_sleeps_and_recovers() {
        let mut game = collision_room();
        let id = game.spawn("oManTrap", Vec2::new(112.0, 144.0));
        game.world.entity_mut(id).unwrap().vel = Vec2::default();
        let victim = game.spawn("oDamsel", Vec2::new(116.0, 144.0));
        game.step(&crate::InputFrame::default());
        assert!(game.world.entity(victim).is_none());
        assert_eq!(game.world.entity(id).unwrap().state, 97);
        let mut observed_blood_phase = false;
        let mut observed_sleep = false;
        let mut observed_stun = false;
        let mut recovered = false;
        for _ in 0..600 {
            let trap = game.world.entity(id).unwrap();
            let (state, phase, bounced) = (trap.state, trap.animation, trap.value("bounced"));
            let blood_before: Vec<_> = game
                .world
                .entities
                .iter()
                .filter(|e| e.object == "oBlood")
                .map(|e| e.id)
                .collect();
            game.step(&crate::InputFrame::default());
            if state == 97 && phase == 8.0 {
                assert!(
                    game.world
                        .entities
                        .iter()
                        .any(|e| e.object == "oBlood" && !blood_before.contains(&e.id))
                );
                observed_blood_phase = true;
            }
            let trap = game.world.entity(id).unwrap();
            observed_sleep |= trap.state == 96;
            observed_stun |= trap.state == 98;
            if state == 98 && bounced == 0.0 && trap.value("bounced") != 0.0 {
                assert!(
                    game.world
                        .entities
                        .iter()
                        .any(|e| e.object == "oBlood" && !blood_before.contains(&e.id))
                );
            }
            if observed_stun && trap.state == 0 {
                assert_eq!(trap.value("bounced"), 1.0);
                recovered = true;
                break;
            }
        }
        assert!(observed_blood_phase && observed_sleep && observed_stun && recovered);
        assert_eq!(game.player.health, 4);
        assert_eq!(game.state, crate::RunState::Playing);
    }

    #[test]
    fn mantrap_death_creates_leaves_blood_keeper_then_second_leaf_blood_burst() {
        let mut game = Game::shell(38, RulesPreset::Classic);
        let mut trap = Entity::new(999, "oManTrap", Vec2::new(48.0, 48.0));
        super::super::init_entity(&mut trap);
        trap.health = 0;
        trap.vel = Vec2::default();
        trap.data.insert("ate_shopkeeper".into(), 1.0);
        let mut expected = game.rng.clone();
        let mut leaf_positions = Vec::new();
        for _ in 0..2 {
            for _ in 0..3 {
                leaf_positions.push(Vec2::new(
                    48.0 + f64::from(expected.range(17)),
                    40.0 + f64::from(expected.range(17)),
                ));
            }
            expected.unit();
            expected.unit();
            expected.unit();
            expected.range(6);
        }
        expected.range(2);
        expected.range(8);
        tick(&mut game, &mut trap);
        assert!(!trap.alive);
        assert_eq!(
            game.world
                .entities
                .iter()
                .map(|e| e.object.as_str())
                .collect::<Vec<_>>(),
            [
                "oLeaf",
                "oLeaf",
                "oLeaf",
                "oBlood",
                "oShopkeeper",
                "oLeaf",
                "oLeaf",
                "oLeaf",
                "oBlood"
            ]
        );
        for (leaf, pos) in game
            .world
            .entities
            .iter()
            .filter(|e| e.object == "oLeaf")
            .zip(leaf_positions)
        {
            assert_eq!(leaf.pos, pos);
            assert_eq!(leaf.vel.y, 0.4);
            assert_eq!(leaf.value("gravity"), 0.01);
        }
        assert_eq!(game.lifetime_stats.enemy_kills[10], 1);
        assert_eq!(game.rng.next_u64(), expected.next_u64());
    }
}
