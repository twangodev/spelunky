//! Classic 1.1 Skeleton, Zombie and Vampire Create/Step event translations.
//! Source instance origins are retained. Tile/rect collisions stand in for
//! GameMaker masks; blood, bone and skull instances need their own item systems.
//! Activation, combat, kill accounting and Vampire cape drops belong to dispatcher.
use super::{bounds, move_body};
use crate::{Entity, Game, PlayerState, Rect, Vec2, World};

pub fn matches(object: &str) -> bool {
    matches!(object, "oSkeleton" | "oZombie" | "oVampire")
}

pub fn init(e: &mut Entity) {
    e.state = 0;
    e.timer = if e.object == "oSkeleton" { 20 } else { 0 };
    e.health = if e.object == "oVampire" { 6 } else { 1 };
    e.facing = 1;
    e.vel = Vec2::new(if e.object == "oVampire" { 2.5 } else { 0.0 }, 0.0);
    e.data.insert(
        "animation_speed".into(),
        if e.object == "oZombie" { 0.4 } else { 0.5 },
    );
    e.data.insert("stun_time".into(), 60.0);
    e.data.insert(
        "bloodless".into(),
        if e.object == "oZombie" { 0.0 } else { 1.0 },
    );
    e.data.insert("source_facing_pending".into(), 1.0);
    e.sprite = match e.object.as_str() {
        "oSkeleton" => "sSkeletonLeft",
        "oZombie" => "sZombieLeft",
        _ => "sVampireLeft",
    }
    .into();
}

pub fn tick(game: &mut Game, e: &mut Entity) {
    if e.value("source_facing_pending") > 0.0 {
        e.facing = match e.object.as_str() {
            "oSkeleton" => {
                if game.player.pos.x < e.pos.x + 8.0 {
                    -1
                } else {
                    1
                }
            }
            "oZombie" => random_facing(game),
            _ => 1,
        };
        e.data.insert("source_facing_pending".into(), 0.0);
    }
    match e.object.as_str() {
        "oSkeleton" => skeleton(game, e),
        "oZombie" => zombie(game, e),
        "oVampire" => vampire(game, e),
        _ => {}
    }
}

fn random_facing(game: &mut Game) -> i8 {
    if game.rng.range(2) == 0 { -1 } else { 1 }
}
fn side(world: &World, e: &Entity, direction: i8, distance: f64) -> bool {
    let r = bounds(e);
    world.collides(Rect::new(
        r.x + f64::from(direction) * distance,
        r.y,
        r.w,
        r.h,
    ))
}
fn bottom(world: &World, e: &Entity) -> bool {
    let r = bounds(e);
    world.collides(Rect::new(r.x, r.y + 1.0, r.w, r.h))
}
fn top(world: &World, e: &Entity) -> bool {
    let r = bounds(e);
    world.collides(Rect::new(r.x, r.y - 1.0, r.w, r.h))
}
fn water(world: &World, p: Vec2) -> bool {
    world
        .tile((p.x / 16.0).floor() as i32, (p.y / 16.0).floor() as i32)
        .is_some_and(|t| matches!(t.object.as_str(), "oWater" | "oWaterSwim" | "oWaterTop"))
        || world.entities.iter().any(|e| {
            e.alive
                && matches!(e.object.as_str(), "oWater" | "oWaterSwim" | "oWaterTop")
                && e.bounds().contains(p)
        })
}
fn player_available(game: &Game) -> bool {
    game.player.health > 0
        && game.player.state != PlayerState::Dead
        && game.player.data.get("swimming").copied().unwrap_or(0.0) == 0.0
}
fn debris(game: &mut Game, e: &mut Entity) {
    if e.value("undead_debris") != 0.0 {
        return;
    }
    e.data.insert("undead_debris".into(), 1.0);
    let p = Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0);
    for _ in 0..3 {
        game.world.spawn("oBone", p);
    }
    let id = game.world.spawn("oSkull", p);
    let y_velocity = -f64::from(1 + game.rng.range(3));
    let velocity = Vec2::new(
        f64::from(game.rng.range(4)) - f64::from(game.rng.range(4)),
        y_velocity,
    );
    if let Some(skull) = game.world.entity_mut(id) {
        skull.vel = velocity;
    }
}
fn create_blood(game: &mut Game, e: &Entity) {
    // scrCreateBlood suppresses bloodless enemies and caps oDetritus at 16.
    if e.value("bloodless") == 0.0
        && game
            .world
            .entities
            .iter()
            .filter(|p| p.alive && crate::catalog::is_a(&p.object, "oDetritus"))
            .count()
            < 16
    {
        game.world
            .spawn("oBlood", Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0));
    }
}
fn skeleton(game: &mut Game, e: &mut Entity) {
    move_body(&game.world, e, 0.6);
    if game.world.solid_at(e.pos.x + 8.0, e.pos.y + 8.0) {
        e.health = 0;
    }
    if e.health < 1 {
        debris(game, e);
        e.alive = false;
        return;
    }
    if bottom(&game.world, e) && e.state != 98 {
        e.vel.y = 0.0;
    }
    if e.state == 0 {
        e.timer = e.timer.saturating_sub(1);
        if e.timer == 0 {
            e.state = 1;
        }
    } else if e.state == 1 {
        let left = side(&game.world, e, -1, 1.0);
        let right = side(&game.world, e, 1, 1.0);
        if !(side(&game.world, e, -1, 4.0) && side(&game.world, e, 1, 4.0)) && (left || right) {
            e.facing *= -1;
        }
        e.vel.x = f64::from(e.facing);
    }
    if game.world.collides(bounds(e)) {
        e.pos.y -= 2.0;
    }
    if e.state != 98 {
        e.sprite = if e.state == 1 {
            "sSkeletonWalkLeft"
        } else {
            "sSkeletonLeft"
        }
        .into();
    }
}
fn zombie(game: &mut Game, e: &mut Entity) {
    move_body(&game.world, e, 0.2);
    if game.world.solid_at(e.pos.x + 8.0, e.pos.y + 8.0) {
        e.health = -999;
    }
    if e.health < 1 {
        create_blood(game, e);
        debris(game, e);
        e.alive = false;
        return;
    }
    if side(&game.world, e, 1, 1.0) {
        e.vel.x = -1.0;
    }
    if side(&game.world, e, -1, 1.0) {
        e.vel.x = 1.0;
    }
    let grounded = bottom(&game.world, e);
    let distance = e
        .pos
        .distance(Vec2::new(game.player.pos.x - 8.0, game.player.pos.y - 8.0));
    match e.state {
        0 => {
            e.vel.x = 0.0;
            if e.timer > 0 {
                e.timer -= 1;
            } else if distance < 64.0 {
                e.state = 1;
            }
            if distance < 48.0 {
                e.state = 1;
            }
            if game.player.data.get("swimming").copied().unwrap_or(0.0) != 0.0 {
                e.state = 0;
            }
            if e.state == 1 {
                game.sound("sndZombie", e.pos);
            }
        }
        2 if grounded => {
            e.state = 0;
            e.vel = Vec2::default();
            e.timer = 40 + game.rng.range(61);
        }
        1 => {
            if grounded {
                let big = game.rng.chance(1, 4);
                e.vel.y = -f64::from(if big {
                    2 + game.rng.range(3)
                } else {
                    1 + game.rng.range(2)
                });
                e.facing = if game.player.pos.x < e.pos.x { -1 } else { 1 };
                e.vel.x = f64::from(e.facing) * if big { 3.0 } else { 1.0 };
            } else {
                e.state = 2;
            }
        }
        2 | 4 => {}
        _ => {
            e.state = 0;
            e.vel.x = 0.0;
        }
    }
    if top(&game.world, e) {
        e.vel.y = 1.0;
    }
    e.sprite = if grounded {
        "sZombieLeft"
    } else {
        "sZombieJumpL"
    }
    .into();
}
fn vampire(game: &mut Game, e: &mut Entity) {
    let gravity = if e.held || matches!(e.state, 6 | 7) {
        0.0
    } else if e.state == 98 {
        0.6
    } else {
        0.2
    };
    move_body(&game.world, e, gravity);
    let grounded = bottom(&game.world, e);
    let ceiling = top(&game.world, e);
    let left = side(&game.world, e, -1, 1.0);
    let right = side(&game.world, e, 1, 1.0);
    let crushed = game.world.solid_at(
        e.pos.x + 8.0,
        e.pos.y + if e.state >= 98 { 12.0 } else { 8.0 },
    );
    if crushed && (e.state >= 98 || !e.held) {
        game.world
            .spawn("oSmokePuff", Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0));
        game.sound("sndCavemanDie", e.pos);
        e.alive = false;
        return;
    }
    if grounded && e.state != 98 {
        e.vel.y = 0.0;
    }
    if e.state < 98 && e.health < 1 {
        e.state = 99;
    }
    let burning = e.value("burning");
    if burning > 0.0 {
        if game.rng.chance(1, 5) {
            let p = Vec2::new(
                e.pos.x + f64::from(4 + game.rng.range(9)),
                e.pos.y + f64::from(4 + game.rng.range(9)),
            );
            game.world.spawn("oBurn", p);
        }
        e.data.insert("burning".into(), burning - 1.0);
    }
    let target = game.player.pos;
    let distance = e.pos.distance(Vec2::new(target.x - 8.0, target.y - 8.0));
    match e.state {
        0 => {
            e.data.insert("bounced".into(), 0.0);
            if grounded
                && (game.world.solid_at(e.pos.x - 1.0, e.pos.y)
                    || game.world.solid_at(e.pos.x + 16.0, e.pos.y))
            {
                e.vel = Vec2::new(f64::from(e.facing), -6.0);
                e.timer = e.timer.saturating_sub(10);
            }
            if e.vel.y < 0.0 && ceiling {
                e.vel.y = 0.0;
            }
            if grounded {
                e.timer = e.timer.saturating_sub(1);
            }
            if e.timer == 0 {
                e.facing = random_facing(game);
                e.state = 1;
            }
            if distance < 96.0 {
                e.facing = if target.x < e.pos.x + 8.0 { -1 } else { 1 };
                e.state = 2;
            }
        }
        1 => {
            if left || right {
                e.facing *= -1;
            }
            if grounded {
                let stop = game.rng.chance(1, 100);
                let ahead_x = e.pos.x + if e.facing < 0 { -1.0 } else { 16.0 };
                if stop || !game.world.solid_at(ahead_x, e.pos.y + 16.0) {
                    e.state = 0;
                    e.timer = 20 + game.rng.range(31);
                    e.vel.x = 0.0;
                    if !stop {
                        e.vel.y = 0.0;
                    }
                } else {
                    e.vel.x = f64::from(e.facing) * 1.5;
                }
            }
            if distance < 96.0 {
                e.state = 2;
            }
        }
        2 => {
            e.data.insert("animation_speed".into(), 1.0);
            if side(&game.world, e, e.facing, 4.0) {
                if ceiling {
                    e.facing *= -1;
                } else {
                    e.state = 5;
                }
            } else if !game
                .world
                .solid_at(e.pos.x + f64::from(e.facing) * 8.0, e.pos.y + 16.0)
            {
                e.state = 5;
            }
            if (target.x - e.pos.x).abs() < 32.0
                && target.y < e.pos.y + 8.0
                && !water(&game.world, Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0))
            {
                e.state = 7;
            }
            e.vel.x = f64::from(e.facing) * 4.0;
        }
        4 => {
            if (e.facing < 0 && left) || (e.facing > 0 && right) {
                e.facing *= -1;
                e.vel.x = -e.vel.x;
            } else if grounded {
                e.state = 0;
                e.vel = Vec2::default();
                e.timer = 40 + game.rng.range(61);
            } else if !water(&game.world, Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0))
                && (game.rng.chance(1, 100)
                    || water(&game.world, Vec2::new(e.pos.x + 8.0, e.pos.y + 24.0)))
            {
                e.state = 7;
            }
        }
        5 => {
            if grounded {
                e.vel.y = -f64::from(3 + game.rng.range(4));
            } else {
                e.state = 4;
            }
        }
        6 => {
            e.vel = Vec2::default();
            if player_available(game)
                && ((distance < 90.0 && target.y > e.pos.y + 16.0)
                    || !game.world.solid_at(e.pos.x + 8.0, e.pos.y - 1.0))
            {
                e.state = 7;
                game.sound("sndBat", e.pos);
            }
        }
        7 => {
            e.vel = Vec2::default();
            let at_ceiling = game.world.solid_at(e.pos.x + 8.0, e.pos.y - 1.0);
            if player_available(game) {
                if distance < 160.0 {
                    let delta = Vec2::new(target.x - (e.pos.x + 8.0), target.y - (e.pos.y + 8.0));
                    let length = delta.x.hypot(delta.y);
                    if length > 0.0 {
                        e.vel = Vec2::new(delta.x / length * 2.0, delta.y / length * 2.0);
                    }
                    if (right && target.x > e.pos.x + 8.0) || (left && target.x < e.pos.x + 8.0) {
                        e.vel = Vec2::new(0.0, if target.y < e.pos.y + 8.0 { -2.0 } else { 2.0 });
                    }
                    if ((ceiling && target.y < e.pos.y + 8.0)
                        || (grounded && target.y > e.pos.y + 8.0))
                        && (target.x - e.pos.x).abs() > 8.0
                    {
                        e.vel = Vec2::new(if target.x < e.pos.x + 8.0 { -2.0 } else { 2.0 }, 0.0);
                    }
                    if water(&game.world, Vec2::new(e.pos.x + 8.0, e.pos.y + 16.0)) && e.vel.y > 0.0
                    {
                        e.vel = Vec2::new(0.0, -2.0);
                    }
                    if water(&game.world, Vec2::new(e.pos.x, e.pos.y + 12.0)) && target.y >= e.pos.y
                    {
                        e.vel = Vec2::default();
                    }
                } else if at_ceiling {
                    e.state = 6;
                } else {
                    e.vel.y = -2.0;
                }
                e.facing = if target.x < e.pos.x + 8.0 { -1 } else { 1 };
                if grounded || water(&game.world, e.pos) {
                    e.state = 0;
                }
            } else if at_ceiling {
                e.state = 6;
            } else {
                e.vel.y = -1.0;
            }
        }
        98 => {
            if !grounded {
                e.sprite = if e.vel.x == 0.0 && e.health > 0 {
                    "sVampireStunL"
                } else if e.value("bounced") > 0.0 {
                    if e.vel.y < 0.0 {
                        "sVampireBounceL"
                    } else {
                        "sVampireFallL"
                    }
                } else if e.vel.x.abs() > 0.0 {
                    "sVampireDieLL"
                } else {
                    "sVampireDieLR"
                }
                .into();
            }
            if grounded && e.value("bounced") == 0.0 {
                e.data.insert("bounced".into(), 1.0);
                create_blood(game, e);
            }
            if grounded || e.held {
                if e.timer > 0 {
                    e.timer -= 1;
                } else if e.health > 0 {
                    e.state = 0;
                    e.held = false;
                    if game.player.held == Some(e.id) {
                        game.player.held = None;
                    }
                }
            }
        }
        99 => {
            debris(game, e);
            game.sound("sndCavemanDie", e.pos);
            e.alive = false;
            return;
        }
        // THROW is declared in the source but no Step branch assigns behavior.
        _ => {}
    }
    if e.state >= 98 && e.vel.x == 0.0 && e.vel.y == 0.0 && e.health < 1 {
        e.state = 99;
    }
    // Source applies friction even in bat form.
    if e.vel.x > 0.0 {
        e.vel.x -= 0.1;
    }
    if e.vel.x < 0.0 {
        e.vel.x += 0.1;
    }
    if e.vel.x.abs() < 0.5 {
        e.vel.x = 0.0;
    }
    if e.state == 6 {
        e.sprite = "sVampireBatHang".into();
    } else if e.state < 98 && e.state != 3 {
        e.sprite = match e.state {
            7 => "sVampireBatLeft",
            4 => "sVampireJumpL",
            _ if e.vel.x.abs() > 0.0 => "sVampireRunL",
            _ => "sVampireLeft",
        }
        .into();
    }
    if e.held {
        e.sprite = if e.health > 0 {
            "sVampireHeldL"
        } else {
            "sVampireDHeldL"
        }
        .into();
    }
    // Collision with oBlood: uncapped healing, including while stunned.
    let body = bounds(e);
    for blood in &mut game.world.entities {
        if blood.alive && blood.object == "oBlood" && body.intersects(blood.bounds()) {
            e.health += 1;
            blood.alive = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RulesPreset;
    #[test]
    fn source_health_and_skeleton_delay() {
        let mut game = Game::shell(42, RulesPreset::Classic);
        let mut e = Entity::new(99, "oSkeleton", Vec2::new(40.0, 40.0));
        init(&mut e);
        for _ in 0..19 {
            tick(&mut game, &mut e);
            assert_eq!(e.state, 0);
        }
        tick(&mut game, &mut e);
        assert_eq!(e.state, 1);
        let mut vampire = Entity::new(100, "oVampire", Vec2::new(40.0, 40.0));
        init(&mut vampire);
        assert_eq!(vampire.health, 6);
    }
    #[test]
    fn zombie_does_not_chase_swimming_player() {
        let mut game = Game::shell(42, RulesPreset::Classic);
        game.player.pos = Vec2::new(48.0, 48.0);
        game.player.data.insert("swimming".into(), 1.0);
        let mut e = Entity::new(99, "oZombie", Vec2::new(40.0, 40.0));
        init(&mut e);
        tick(&mut game, &mut e);
        assert_eq!(e.state, 0);
        game.player.data.insert("swimming".into(), 0.0);
        tick(&mut game, &mut e);
        assert_eq!(e.state, 1);
    }
    #[test]
    fn vampire_hangs_and_reactivates() {
        let mut game = Game::shell(42, RulesPreset::Classic);
        game.player.pos = Vec2::new(48.0, 80.0);
        let mut e = Entity::new(99, "oVampire", Vec2::new(40.0, 40.0));
        init(&mut e);
        e.state = 6;
        tick(&mut game, &mut e);
        assert_eq!(e.state, 7);
        assert_eq!(e.vel, Vec2::default());
        tick(&mut game, &mut e);
        assert!(e.vel.y > 0.0);
    }
    #[test]
    fn skeleton_death_spawns_source_debris_once() {
        let mut game = Game::shell(42, RulesPreset::Classic);
        let mut e = Entity::new(99, "oSkeleton", Vec2::new(40.0, 40.0));
        init(&mut e);
        e.health = 0;
        tick(&mut game, &mut e);
        assert!(!e.alive);
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|e| e.object == "oBone")
                .count(),
            3
        );
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|e| e.object == "oSkull")
                .count(),
            1
        );
    }
    #[test]
    fn vampire_absorbs_external_blood_without_health_cap() {
        let mut game = Game::shell(42, RulesPreset::Classic);
        let mut e = Entity::new(99, "oVampire", Vec2::new(40.0, 40.0));
        init(&mut e);
        e.health = 12;
        e.state = 6;
        let id = game.world.spawn("oBlood", Vec2::new(48.0, 48.0));
        tick(&mut game, &mut e);
        assert_eq!(e.health, 13);
        assert!(
            !game
                .world
                .entities
                .iter()
                .find(|e| e.id == id)
                .unwrap()
                .alive
        );
    }
    #[test]
    fn bloodless_vampire_does_not_create_own_healing_blood() {
        let mut game = Game::shell(42, RulesPreset::Classic);
        let mut e = Entity::new(99, "oVampire", Vec2::new(40.0, 40.0));
        init(&mut e);
        create_blood(&mut game, &e);
        assert!(game.world.entities.is_empty());
    }
}
