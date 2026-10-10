//! Jungle behaviors translated from Classic 1.1 enemy events.
//! Instances retain original GameMaker sprite origins; probes use collision centers.
use super::{can_stomp, center, event_bounds, move_body, steer_toward, touches_player};
use crate::model::{Area, Entity, Game, PlayerState, Vec2, World};

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
    e.sprite = match e.object.as_str() {
        "oFrog" => "sFrogLeft",
        "oFireFrog" => "sFireFrogLeft",
        "oMonkey" => "sMonkeyHangL",
        "oPiranha" => "sPiranhaRight",
        "oJaws" => "sJawsLeft",
        _ => "sManTrapLeft",
    }
    .into();
    if e.object == "oJaws" {
        e.facing = -1;
    }
    if e.object == "oManTrap" {
        e.vel.x = 2.5;
    }
    if e.object == "oMonkey" {
        e.data.insert("throw_counter".into(), 60.0);
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
    world
        .tile((p.x / 16.0).floor() as i32, (p.y / 16.0).floor() as i32)
        .is_some_and(|t| t.is_ladder())
        || world.entities.iter().any(|e| {
            e.alive
                && matches!(e.object.as_str(), "oVine" | "oLadder" | "oRope")
                && e.bounds().contains(p)
        })
}

fn countdown(e: &mut Entity, key: &str) {
    let count = e.value(key);
    if count > 0.0 {
        e.data.insert(key.into(), count - 1.0);
    }
}

fn collision_offset(world: &World, e: &Entity, dx: f64, dy: f64) -> bool {
    let mut bounds = e.bounds();
    bounds.x += dx;
    bounds.y += dy;
    world.collides_except(bounds, e.id)
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
    let swimming = water(&game.world, Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0));
    if matches!(
        e.object.as_str(),
        "oFrog" | "oFireFrog" | "oMonkey" | "oManTrap"
    ) {
        e.data.insert("swimming".into(), f64::from(swimming));
    }
    // oEnemy Step runs before oFireFrog Step: water converts even a mortally
    // injured fire frog into a newly created ordinary frog, without a bomb.
    if e.object == "oFireFrog" && swimming {
        let id = game.world.spawn("oFrog", e.pos);
        if let Some(frog) = game.world.entity_mut(id) {
            super::init_entity(frog);
            frog.state = e.state;
        }
        e.alive = false;
        e.health = 1;
        return;
    }
    if e.health <= 0 && matches!(e.object.as_str(), "oPiranha" | "oJaws") {
        die(game, e);
        return;
    }
    match e.object.as_str() {
        "oFrog" | "oFireFrog" => frog(game, e),
        "oMonkey" => monkey(game, e),
        "oPiranha" => fish(game, e),
        "oJaws" => jaws(game, e),
        "oManTrap" => mantrap(game, e),
        _ => {}
    }
}

fn die(game: &mut Game, e: &mut Entity) {
    if e.object == "oFireFrog" {
        let id = game
            .world
            .spawn("oFireFrogBomb", Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0));
        if let Some(bomb) = game.world.entity_mut(id) {
            bomb.timer = 120;
            bomb.data.insert("armed".into(), 1.0);
            bomb.data.insert("heavy".into(), 1.0);
        }
    }
    if e.object == "oJaws" {
        for _ in 0..4 {
            let bone = Vec2::new(
                e.pos.x + 22.0 + game.rng.range(5) as f64,
                e.pos.y + 14.0 + game.rng.range(7) as f64,
            );
            game.world.spawn("oBone", bone);
            let id = game
                .world
                .spawn("oCrate", Vec2::new(e.pos.x + 16.0, e.pos.y + 16.0));
            let velocity = Vec2::new(
                game.rng.range(4) as f64 - game.rng.range(4) as f64,
                -(1.0 + game.rng.range(2) as f64),
            );
            if let Some(item) = game.world.entity_mut(id) {
                item.vel = velocity;
            }
        }
    }
    if e.object == "oManTrap" && e.value("ate_shopkeeper") != 0.0 {
        release_shopkeeper(game, e.pos);
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
        return;
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
    e.sprite = match (fire, grounded) {
        (true, true) => "sFireFrogLeft",
        (true, false) => "sFireFrogJumpL",
        (false, true) => "sFrogLeft",
        (false, false) => "sFrogJumpL",
    }
    .into();
}

fn monkey(game: &mut Game, e: &mut Entity) {
    for key in ["grab_counter", "vine_counter", "throw_counter"] {
        countdown(e, key);
    }
    let (wall, _) = move_body(
        &game.world,
        e,
        if matches!(e.state, 5..=7) { 0.0 } else { 0.2 },
    );
    if game.world.solid_at(e.pos.x + 8.0, e.pos.y + 8.0) {
        e.health = -999;
    }
    if e.health <= 0 {
        die(game, e);
        return;
    }
    if collision_offset(&game.world, e, 1.0, 0.0) {
        e.vel.x = -1.0;
    }
    if collision_offset(&game.world, e, -1.0, 0.0) {
        e.vel.x = 1.0;
    }
    let grounded = ground(&game.world, e);
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
            e.vel = Vec2::default();
            e.pos = Vec2::new(
                game.player.pos.x + e.value("grab_x"),
                game.player.pos.y + e.value("grab_y"),
            );
            e.facing = game.player.facing;
            if game.player.health <= 0 {
                release_monkey(game, e);
            } else if e.timer > 0 {
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
    // Collision with oCharacter overrides ordinary contact damage. Defer
    // source-eligible stomps to the parent; all other centered contacts grab.
    if e.state != 7
        && e.value("grab_counter") == 0.0
        && game.player.invincible == 0
        && game.player.health > 0
        && (game.player.pos.x - (e.pos.x + 8.0)).abs() <= 4.0
        && touches_player(game, e)
        && !can_stomp(game, e)
    {
        e.state = 7;
        e.vel = Vec2::default();
        e.data.insert("grab_x".into(), e.pos.x - game.player.pos.x);
        e.data.insert(
            "grab_y".into(),
            (e.pos.y + 8.0 - game.player.pos.y).clamp(-2.0, 2.0) - 8.0,
        );
        e.pos.y = game.player.pos.y + e.value("grab_y");
        e.timer = 40 + game.rng.range(41);
    }
    // Collision with oItem applies to the source parent family, including jars
    // and every weapon. A rope already falling into placement is exempt.
    if e.state != 7 && e.value("throw_counter") == 0.0 {
        let item = game
            .world
            .entities
            .iter()
            .find(|item| {
                item.alive
                    && !item.held
                    && crate::catalog::is_a(&item.object, "oItem")
                    && item.value("inactive") == 0.0
                    && !(item.object == "oRopeThrow" && item.value("falling") != 0.0)
                    && event_bounds(item).intersects(event_bounds(e))
            })
            .map(|item| (item.id, item.object == "oRopeThrow", item.pos));
        if let Some((id, rope, pos)) = item {
            let lift = !game
                .world
                .solid_at(pos.x, pos.y - if rope { 0.0 } else { 2.0 });
            if let Some(item) = game.world.entity_mut(id) {
                item.vel = Vec2::new(5.0 * e.facing as f64, -4.0);
                if lift {
                    item.pos.y = if rope {
                        e.pos.y - 2.0
                    } else {
                        item.pos.y - 2.0
                    };
                }
            }
            e.data.insert("throw_counter".into(), 60.0);
            e.state = 0;
            e.timer = 20 + game.rng.range(41);
        }
    }
    if e.state != 7 && collision_offset(&game.world, e, 0.0, -1.0) {
        e.vel.y = 1.0;
    }
    e.sprite = match e.state {
        5 => "sMonkeyHangL",
        6 | 7 => "sMonkeyClimbL",
        3 if grounded => "sMonkeyWalkL",
        _ if !grounded => "sMonkeyJumpL",
        _ => "sMonkeyLeft",
    }
    .into();
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
    if game.rng.chance(1, 4) {
        game.player.vel = Vec2::new(3.0 * game.player.facing as f64, -3.0);
        game.player.stun = 40;
        game.player.state = PlayerState::Stunned;
        if let Some(id) = game.player.held.take() {
            if let Some(item) = game.world.entity_mut(id) {
                item.held = false;
            }
        }
        game.sound("sndHit", e.pos);
        return;
    } else if game.player.money >= money && game.rng.chance(8, 10) {
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
    let armed = object == "oBomb" && game.rng.chance(1, 10);
    let velocity = Vec2::new(
        game.rng.range(3) as f64 - game.rng.range(3) as f64,
        -(3.0 + game.rng.range(2) as f64),
    );
    let id = game.world.spawn(object, e.pos);
    if let Some(item) = game.world.entity_mut(id) {
        crate::items::init_entity(item);
        item.vel = velocity;
        item.data.insert("collect_delay".into(), 20.0);
        item.data.insert("source_monkey".into(), 1.0);
        if armed {
            item.data.insert("armed".into(), 1.0);
            item.timer = 40;
            item.sprite = "sBombArmed".into();
        }
    }
    game.sound("sndThrow", e.pos);
}

fn fish(game: &mut Game, e: &mut Entity) {
    let target_swimming = game.player.health > 0 && water(&game.world, game.player.pos);
    let near = center(e).distance(game.player.pos) < 90.0;
    countdown(e, "bite_counter");
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
            e.data.insert("bite_counter".into(), 0.0);
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
                if center(e).distance(pos) < 4.0 && e.value("bite_counter") == 0.0 {
                    if let Some(target) = game.world.entity_mut(id) {
                        target.health -= 1;
                        target.state = 2;
                    }
                    e.data.insert("bite_counter".into(), 10.0);
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
    e.sprite = if e.facing < 0 {
        "sPiranhaLeft"
    } else {
        "sPiranhaRight"
    }
    .into();
    bubbles(game, e, true);
    if !water(&game.world, Vec2::new(e.pos.x + 4.0, e.pos.y + 4.0)) {
        game.world.spawn("oFishBone", e.pos);
        e.alive = false;
    }
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
                return water(world, Vec2::new(t.pos.x + 8.0, t.pos.y + 8.0)).then_some(t.id);
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
        if e.health <= 0 {
            die(game, e);
            return;
        }
    }
    let swimming = game.player.health > 0 && water(&game.world, game.player.pos);
    let turn_before = e.value("jaws_turn_ticks") > 0.0;
    let turn_serial = e.value("jaws_turn_serial");
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
    if turn_before && turn_serial == e.value("jaws_turn_serial") {
        let remaining = (e.value("jaws_turn_ticks") - 1.0).max(0.0);
        e.data.insert("jaws_turn_ticks".into(), remaining);
        e.timer = remaining as u32;
        if remaining == 0.0 {
            if e.facing > 0 {
                e.pos.x += 48.0;
                e.previous_pos = e.pos;
            }
            e.state = 2;
            e.timer = 40;
        }
    }
    let turning = e.value("jaws_turn_ticks") > 0.0;
    e.sprite = match (turning, e.facing < 0) {
        (true, true) => "sJawsTurnL",
        (true, false) => "sJawsTurnR",
        (false, true) => "sJawsLeft",
        (false, false) => "sJawsRight",
    }
    .into();
    // Turning sprites retain the previous collision rectangle in GameMaker.
    if !turning {
        jaws_bounds(e);
    }
    bubbles(game, e, false);
}

fn begin_jaws_turn(e: &mut Entity) {
    e.state = 3;
    e.facing *= -1;
    e.timer = 12;
    e.vel = Vec2::default();
    e.animation = 0.0;
    e.data.insert("jaws_turn_ticks".into(), 12.0);
    e.data
        .insert("jaws_turn_serial".into(), e.value("jaws_turn_serial") + 1.0);
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
        game.world.spawn("oBubble", e.pos);
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

fn release_shopkeeper(game: &mut Game, pos: Vec2) {
    let id = game.world.spawn("oShopkeeper", pos);
    if let Some(shopkeeper) = game.world.entity_mut(id) {
        super::init_entity(shopkeeper);
        shopkeeper.state = 2;
        shopkeeper.data.insert("has_gun".into(), 0.0);
    }
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
        die(game, e);
        return;
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
            } else {
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
            e.timer = e.timer.saturating_sub(1);
            if e.timer == 0 {
                e.state = 96;
                e.timer = 32;
                e.sprite = "sManTrapSleepL".into();
                e.animation = 0.0;
            }
        }
        96 => {
            e.vel.x = 0.0;
            e.timer = e.timer.saturating_sub(1);
            if e.timer == 0 {
                e.state = 98;
                e.timer = 400;
            }
        }
        98 => {
            if grounded || e.held {
                if e.timer > 0 {
                    e.timer -= 1;
                } else if e.value("ate_shopkeeper") != 0.0 {
                    release_shopkeeper(game, e.pos);
                    e.alive = false;
                    return;
                } else {
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
        e.vel.x *= 0.3;
        if e.vel.x.abs() < 0.1 {
            e.vel.x = 0.0;
        }
    }
    if game.world.collides_except(e.bounds(), e.id) {
        e.pos.y -= 2.0;
    }
    // An awake trap eats rather than dealing ordinary contact damage. Spike
    // shoes permit the parent combat handler to resolve a stomp first.
    if !matches!(e.state, 97 | 98)
        && game.player.health > 0
        && game.player.invincible == 0
        && (game.player.pos.x - (e.pos.x + 8.0)).abs() <= 8.0
        && touches_player(game, e)
        && !can_stomp(game, e)
    {
        eat(e, "sManTrapEatL", game.player.pos.x);
        game.player.health = -99;
        game.player.invincible = 9999;
        game.player.data.insert("hidden".into(), 1.0);
        if let Some(id) = game.player.held.take() {
            if let Some(item) = game.world.entity_mut(id) {
                item.held = false;
                item.vel = Vec2::new(e.facing as f64 * 2.0, -4.0);
            }
        }
        game.sound("sndDie", game.player.pos);
    }
    if !matches!(e.state, 97 | 98) {
        let victim = game
            .world
            .entities
            .iter()
            .find(|v| {
                v.alive
                    && v.id != e.id
                    && matches!(v.object.as_str(), "oCaveman" | "oDamsel" | "oShopkeeper")
                    && event_bounds(v).intersects(event_bounds(e))
            })
            .map(|v| {
                (
                    v.id,
                    v.object.clone(),
                    v.pos.x,
                    v.health,
                    v.data.get("has_gun").copied().unwrap_or(1.0) > 0.0,
                )
            });
        if let Some((id, kind, x, health, has_gun)) = victim {
            eat(
                e,
                match kind.as_str() {
                    "oDamsel" => "sManTrapEatDamselL",
                    "oShopkeeper" => "sManTrapEatShopkeeperL",
                    _ => "sManTrapEatCavemanL",
                },
                x,
            );
            if kind == "oShopkeeper" && health > 0 {
                e.data.insert("ate_shopkeeper".into(), 1.0);
            }
            if let Some(v) = game.world.entity_mut(id) {
                v.alive = false;
            }
            if game.player.held == Some(id) {
                game.player.held = None;
            }
            if kind == "oShopkeeper" && has_gun {
                game.world.spawn("oShotgun", e.pos);
            }
        }
    }
    if e.state == 98 {
        e.sprite = "sManTrapStunL".into();
    } else if e.state < 96 {
        e.sprite = "sManTrapLeft".into();
    }
}

fn eat(e: &mut Entity, sprite: &str, x: f64) {
    e.state = 97;
    e.timer = 28;
    e.vel.x = 0.0;
    e.animation = 0.0;
    e.facing = if x > center(e).x { 1 } else { -1 };
    e.sprite = sprite.into();
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
        for y in 1..10 {
            for x in 1..10 {
                game.world.set_tile(x, y, "oWater");
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
        assert_eq!(e.state, 7);
        assert!((40..=80).contains(&e.timer));
        assert_eq!(game.player.health, 4);
    }
    #[test]
    fn mantrap_eating_is_lethal_and_then_sleeps() {
        let mut game = Game::shell(6, RulesPreset::Classic);
        game.player.pos = Vec2::new(50.0, 50.0);
        let mut e = entity("oManTrap", game.player.pos);
        tick(&mut game, &mut e);
        assert_eq!(e.state, 97);
        assert_eq!(game.player.health, -99);
        for _ in 0..29 {
            tick(&mut game, &mut e);
        }
        assert_eq!(e.state, 96);
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
        for _ in 0..12 {
            tick(&mut game, &mut e);
        }
        assert_eq!(e.state, 2);
        assert_eq!(e.timer, 40);
    }
    #[test]
    fn inherited_water_extinguishes_fire_frog_before_death_bomb() {
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
        assert!(
            !game
                .world
                .entities
                .iter()
                .any(|e| e.object == "oFireFrogBomb")
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
        game.world.entities.push(e);
        super::super::tick(&mut game);
        assert_ne!(game.world.entity(999).unwrap().state, 7);
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
        game.world.entities.push(e);
        super::super::tick(&mut game);
        let monkey = game.world.entities.iter().find(|e| e.id == 999).unwrap();
        assert_ne!(monkey.state, 7);
        assert_eq!(monkey.health, 0);
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
            game.world.entities.push(e);
            super::super::tick(&mut game);
            assert_eq!(
                game.player.health, expected_health,
                "spikes={spikes} stun={stun}"
            );
            if spikes && stun == 0 {
                assert_eq!(
                    game.world
                        .entities
                        .iter()
                        .find(|e| e.id == 999)
                        .unwrap()
                        .health,
                    0
                );
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
        for _ in 0..12 {
            tick(&mut game, &mut e);
        }
        assert_eq!(e.state, 2);
        assert_eq!(e.pos.x, 96.0);
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
        for _ in 0..11 {
            tick(&mut game, &mut e);
        }
        assert_eq!(e.state, 2);
        assert_eq!(e.sprite, "sJawsRight");
        assert_eq!(e.timer, 40);
        assert_eq!(e.pos.x, 100.0);
    }
}
