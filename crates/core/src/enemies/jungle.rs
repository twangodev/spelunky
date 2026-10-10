//! Jungle behaviors translated from Classic 1.1 enemy events.
//! Instances retain original GameMaker sprite origins; probes use collision centers.
use super::{center, move_body, steer_toward};
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
    if matches!(e.object.as_str(), "oJaws" | "oPiranha") {
        e.data.insert("no_stomp".into(), 1.0);
    }
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

fn ground(world: &World, e: &Entity) -> bool {
    let mut b = e.bounds();
    b.y += 1.0;
    world.collides(b)
}

pub fn tick(game: &mut Game, e: &mut Entity) {
    if e.health <= 0 {
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
                game.world.spawn("oBone", e.pos);
                let id = game.world.spawn("oCrate", e.pos);
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
        // Let the parent's common death path record the kill and remove the body.
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

fn frog(game: &mut Game, e: &mut Entity) {
    let (wall, _) = move_body(&game.world, e, 0.2);
    if wall {
        e.vel.x = -(e.facing as f64);
    }
    let grounded = ground(&game.world, e);
    match e.state {
        0 => {
            e.vel.x = 0.0;
            if e.timer > 0 {
                e.timer -= 1;
            } else if center(e).distance(game.player.pos) < 64.0 {
                e.state = 1;
                game.sound("sndFrog", e.pos);
            }
        }
        1 => {
            if grounded {
                e.vel.y = -(2.0 + game.rng.range(3) as f64);
                e.facing = if game.player.pos.x < center(e).x {
                    -1
                } else {
                    1
                };
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
    let grounded = ground(&game.world, e);
    let dist = center(e).distance(game.player.pos);
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
            // Generated monkeys begin hanging; unsupported hanging transitions to a fall.
            if !vine(&game.world, center(e)) {
                e.state = 2;
            } else if e.timer > 0 {
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
                Vec2::new(center(e).x, center(e).y + e.vel.y * 12.0),
            ) {
                e.data
                    .insert("dir".into(), if e.vel.y < 0.0 { 1.0 } else { 0.0 });
                e.state = 5;
                e.timer = 10 + game.rng.range(31);
            }
            if dist < 64.0 && game.player.pos.y > center(e).y {
                e.state = 1;
                e.data.insert("vine_counter".into(), 30.0);
                e.vel.y = -(2.0 + game.rng.range(3) as f64);
                e.facing = if game.player.pos.x < center(e).x {
                    -1
                } else {
                    1
                };
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
    // Monkey collision is a grab rather than normal contact damage. Stomps are
    // handled in the parent before this special mechanic is eligible.
    if e.state != 7
        && e.value("grab_counter") == 0.0
        && game.player.invincible == 0
        && game.player.health > 0
        && e.bounds().intersects(game.player.bounds())
        && !(game.player.vel.y > 0.0 && game.player.pos.y < center(e).y - 5.0)
    {
        e.state = 7;
        e.vel = Vec2::default();
        e.data.insert("grab_x".into(), e.pos.x - game.player.pos.x);
        e.data.insert(
            "grab_y".into(),
            (center(e).y - game.player.pos.y).clamp(-2.0, 2.0) - (center(e).y - e.pos.y),
        );
        e.timer = 40 + game.rng.range(41);
    }
    // Monkeys throw loose carried-sized items away, once per 60 ticks.
    if e.state != 7 && e.value("throw_counter") == 0.0 {
        if let Some(item) = game.world.entities.iter_mut().find(|item| {
            item.alive
                && !item.held
                && matches!(
                    item.object.as_str(),
                    "oRock" | "oSkull" | "oBomb" | "oRopeThrow" | "oMachete" | "oShotgun"
                )
                && item.bounds().intersects(e.bounds())
        }) {
            item.vel = Vec2::new(5.0 * e.facing as f64, -4.0);
            e.data.insert("throw_counter".into(), 60.0);
            e.state = 0;
            e.timer = 20 + game.rng.range(41);
        }
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
    if !water(&game.world, center(e)) {
        game.world.spawn("oFishBone", e.pos);
        e.alive = false;
        return;
    }
    let target_swimming = game.player.health > 0 && water(&game.world, game.player.pos);
    let near = center(e).distance(game.player.pos) < 90.0;
    countdown(e, "bite_counter");
    match e.state {
        0 => {
            e.vel = Vec2::new(e.facing as f64, 0.0);
            if !swim_move(&game.world, e) {
                e.facing *= -1;
            }
            if target_swimming && near {
                e.state = 1;
            } else if let Some(target) = fish_target(&game.world, center(e)) {
                e.state = 3;
                e.data.insert("fish_target".into(), target as f64);
            }
        }
        1 => {
            if target_swimming && near {
                aim_fish(game, e, game.player.pos);
                swim_move(&game.world, e);
            } else {
                e.state = 2;
                e.timer = 20 + game.rng.range(21);
            }
        }
        2 => {
            e.vel = Vec2::default();
            if e.timer > 0 {
                e.timer -= 1;
            } else {
                e.state = 0;
                e.facing = if game.rng.chance(1, 2) { -1 } else { 1 };
            }
        }
        3 => {
            let target = game
                .world
                .entity(e.value("fish_target") as u64)
                .filter(|t| t.health > 0 && water(&game.world, center(t)))
                .map(|t| (t.id, center(t)));
            if let Some((id, pos)) = target {
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
    if e.vel.x != 0.0 {
        e.facing = if e.vel.x < 0.0 { -1 } else { 1 };
    }
    e.sprite = if e.facing < 0 {
        "sPiranhaLeft"
    } else {
        "sPiranhaRight"
    }
    .into();
    bubbles(game, e, true);
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
            .filter(|t| t.alive && t.object == family)
            .min_by(|a, b| center(a).distance(pos).total_cmp(&center(b).distance(pos)));
        if let Some(t) = target {
            if t.health > 0 {
                return water(world, center(t)).then_some(t.id);
            }
        }
    }
    None
}

fn swim_move(world: &World, e: &mut Entity) -> bool {
    let next = Vec2::new(center(e).x + e.vel.x, center(e).y + e.vel.y);
    if !water(world, next) || world.solid_at(next.x, next.y) {
        return false;
    }
    move_body(world, e, 0.0);
    true
}

fn jaws(game: &mut Game, e: &mut Entity) {
    jaws_bounds(e);
    if !water(&game.world, center(e)) {
        e.health -= 1;
    }
    let swimming = game.player.health > 0 && water(&game.world, game.player.pos);
    match e.state {
        0 => {
            e.vel = Vec2::new(2.0 * e.facing as f64, 0.0);
            if !swim_move(&game.world, e) {
                begin_jaws_turn(e);
            }
            if !ground(&game.world, e)
                && water(&game.world, Vec2::new(center(e).x, center(e).y + 1.0))
            {
                e.pos.y += 1.0;
            }
            if swimming && e.state != 3 {
                e.state = 1;
            }
        }
        1 => {
            if !swimming {
                e.state = 0;
            } else if (game.player.pos.x < center(e).x) != (e.facing < 0) {
                begin_jaws_turn(e);
            } else {
                steer_toward(
                    e,
                    Vec2::new(game.player.pos.x, game.player.pos.y - 8.0),
                    3.0,
                );
                swim_move(&game.world, e);
            }
        }
        2 => {
            if e.timer > 0 {
                e.timer -= 1;
            } else {
                e.state = 0;
            }
        }
        3 => {
            // Six turn frames at image_speed=.5.
            if e.timer > 0 {
                e.timer -= 1;
            } else {
                e.state = 2;
                e.timer = 40;
            }
        }
        _ => {
            e.state = 0;
        }
    }
    e.sprite = match (e.state == 3, e.facing < 0) {
        (true, true) => "sJawsTurnL",
        (true, false) => "sJawsTurnR",
        (false, true) => "sJawsLeft",
        (false, false) => "sJawsRight",
    }
    .into();
    bubbles(game, e, false);
}

fn begin_jaws_turn(e: &mut Entity) {
    e.state = 3;
    e.facing *= -1;
    e.timer = 12;
    e.vel = Vec2::default();
    e.animation = 0.0;
    // A directional sprite origin moves 48 pixels while the body stays put.
    e.pos.x += if e.facing > 0 { 48.0 } else { -48.0 };
    jaws_bounds(e);
    e.previous_pos = e.pos;
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
    let (wall, _) = if e.held {
        (false, false)
    } else {
        move_body(&game.world, e, 0.6)
    };
    let grounded = ground(&game.world, e);
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
            let ahead = center(e).x + e.facing as f64 * 9.0;
            if !game
                .world
                .solid_at(ahead, e.bounds().y + e.bounds().h + 1.0)
            {
                e.facing *= -1;
            }
            let ahead = center(e).x + e.facing as f64 * 9.0;
            e.vel.x = if game
                .world
                .solid_at(ahead, e.bounds().y + e.bounds().h + 1.0)
            {
                e.facing as f64
            } else {
                0.0
            };
            if game.rng.chance(1, 100) {
                e.state = 0;
                e.timer = 20 + game.rng.range(31);
                e.vel.x = 0.0;
            }
        }
        97 => {
            e.vel.x = 0.0;
            if e.timer > 0 {
                e.timer -= 1;
            } else {
                e.state = 96;
                e.timer = 32;
                e.sprite = "sManTrapSleepL".into();
                e.animation = 0.0;
            }
        }
        96 => {
            e.vel.x = 0.0;
            if e.timer > 0 {
                e.timer -= 1;
            } else {
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
                    e.held = false;
                    if game.player.held == Some(e.id) {
                        game.player.held = None;
                    }
                }
            }
        }
        _ => {}
    }
    if grounded && e.state == 98 {
        e.vel.x *= 0.3;
        if e.vel.x.abs() < 0.1 {
            e.vel.x = 0.0;
        }
    }
    // An awake trap eats rather than dealing ordinary contact damage. Spike
    // shoes permit the parent combat handler to resolve a stomp first.
    let spike_stomp = game.player.has("Spike Shoes")
        || game.player.has("SpikeShoes")
        || game.player.has("oSpikeShoes");
    if !matches!(e.state, 97 | 98)
        && game.player.health > 0
        && game.player.invincible == 0
        && e.bounds().intersects(game.player.bounds())
        && !(spike_stomp && game.player.vel.y > 0.0 && game.player.pos.y < center(e).y)
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
                    && v.bounds().intersects(e.bounds())
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
        for _ in 0..13 {
            tick(&mut game, &mut e);
        }
        assert_eq!(e.state, 2);
        assert_eq!(e.timer, 40);
    }
}
