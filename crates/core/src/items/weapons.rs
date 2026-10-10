//! Classic weapon timing and projectile terrain behavior.
//!
//! Reference: scrUseItem, scrFireBow, oBullet, oArrow, oMattockHit,
//! oPsychicWaveP and oFlare events in Classic 1.1. Enemy damage is dispatched
//! by the enemy subsystem, after projectile movement.
use crate::model::*;

fn attack_sprite(game: &Game) -> &'static str {
    if game.flags.contains("character_tunnelman") {
        "sTunnelAttackL"
    } else if game.flags.contains("character_damsel") {
        "sDamselAttackL"
    } else {
        "sAttackLeft"
    }
}

fn owned(e: &mut Entity, damage: f64) {
    e.data.insert("owner".into(), 1.0);
    e.data.insert("owner_player".into(), 1.0);
    e.data.insert("damage".into(), damage);
}

/// GameMaker uses the selected sprite's mask when no explicit mask is set.
/// Preserve its source rectangle and origin on both facing directions. This
/// remains a rectangle approximation of the source's per-frame alpha mask.
fn selected_sprite_bounds(e: &mut Entity) {
    let name = crate::catalog::object(&e.object)
        .filter(|object| !object.mask.is_empty())
        .map(|object| object.mask.as_str())
        .unwrap_or(&e.sprite);
    if let Some(sprite) = crate::catalog::sprite_def(name) {
        let [left, top, right, bottom] = sprite.bounds;
        for (key, value) in [
            ("collision_left", left - sprite.origin[0]),
            ("collision_top", top - sprite.origin[1]),
            ("collision_right", right + 1 - sprite.origin[0]),
            ("collision_bottom", bottom + 1 - sprite.origin[1]),
        ] {
            e.data.insert(key.into(), f64::from(value));
        }
    }
}

fn spawn_pre(game: &mut Game, mattock: bool) {
    let facing = game.player.facing;
    let id = game.world.spawn(
        if mattock {
            "oMattockPre"
        } else {
            "oMachetePre"
        },
        Vec2::new(
            game.player.pos.x - 16.0 * f64::from(facing),
            game.player.pos.y,
        ),
    );
    let e = game.world.entity_mut(id).unwrap();
    owned(e, 2.0);
    e.facing = facing;
    e.timer = 3;
    e.sprite = format!(
        "s{}Pre{}",
        if mattock { "Mattock" } else { "Machete" },
        if facing < 0 { "L" } else { "R" }
    );
    selected_sprite_bounds(e);
}

pub fn init_entity(e: &mut Entity) {
    if e.value("weapon_initialized") != 0.0 {
        return;
    }
    e.data.insert("weapon_initialized".into(), 1.0);
    if matches!(
        e.object.as_str(),
        "oBow"
            | "oPistol"
            | "oShotgun"
            | "oSceptre"
            | "oWebCannon"
            | "oMachete"
            | "oMattock"
            | "oArrow"
            | "oFlare"
    ) {
        // setCollisionBounds configures movement, not the sprite mask.
        let half_height = if e.object == "oMattock" { 6.0 } else { 4.0 };
        for (key, value) in [
            ("bboxLeft", -4.0),
            ("bboxTop", -half_height),
            ("bboxRight", 4.0),
            ("bboxBottom", half_height),
        ] {
            e.data.insert(key.into(), value);
        }
        e.data.entry("canPickUp".into()).or_insert(1.0);
        e.data.entry("bounceFactor".into()).or_insert(0.5);
        e.data.entry("frictionFactor".into()).or_insert(0.3);
    }
    match e.object.as_str() {
        "oArrow" => {
            e.data.insert("gravity".into(), 0.2);
        }
        "oFlare" => {
            e.data.insert("gravity".into(), 0.6);
            e.data.insert("invincible".into(), 1.0);
        }
        "oPsychicWaveP" => {
            e.timer = 5;
            e.data.insert("damage".into(), 3.0);
        }
        "oWebBall" => {
            e.data.entry("life".into()).or_insert(60.0);
        }
        _ => {}
    }
}

/// Returns true for recognized weapons, including when blocked by cooldown.
/// The host supplies continuous attack state in player.data["attack_held"].
pub fn use_weapon(game: &mut Game, held_id: u64, input: &InputFrame) -> bool {
    let Some(weapon) = game.world.entity(held_id) else {
        return false;
    };
    let object = weapon.object.clone();
    if !matches!(
        object.as_str(),
        "oBow" | "oShotgun" | "oPistol" | "oSceptre" | "oMachete" | "oMattock" | "oWebCannon"
    ) {
        return false;
    }
    if input.down
        || game.player.state == PlayerState::Ducking
        || game
            .player
            .data
            .get("weapon_cooldown")
            .copied()
            .unwrap_or(0.0)
            > 0.0
        || game.player.attack_timer > 0
    {
        return true;
    }
    let facing = f64::from(game.player.facing);
    let pos = game.player.pos;
    match object.as_str() {
        "oBow" => {
            if game.player.data.get("bow_armed").copied().unwrap_or(0.0) != 0.0 {
                return true;
            }
            if game.data.get("arrows").copied().unwrap_or(0.0) <= 0.0 {
                game.events
                    .push(GameEvent::Message("I'M OUT OF ARROWS!".into()));
            } else {
                game.player.data.insert("bow_armed".into(), 1.0);
                game.player.data.insert("bow_strength".into(), 0.0);
                game.sound("sndBowPull", pos);
            }
        }
        "oMachete" | "oMattock" => {
            let mattock = object == "oMattock";
            if mattock && !game.player.grounded {
                return true;
            }
            // characterStepEvent restores speed 1 every tick. Mattock's 0.2
            // applies only to the onset tick, extending the 11-frame attack
            // by one tick; its separate hit effect still advances at 0.5.
            game.player.attack_timer = if mattock { 12 } else { 11 };
            game.player.sprite = attack_sprite(game).into();
            game.player.animation = 0.0;
            if mattock {
                game.player.data.insert("cantJump".into(), 20.0);
            }
            if let Some(held) = game.world.entity_mut(held_id) {
                held.data.insert("hidden".into(), 1.0);
            }
            game.player.data.insert("weapon_melee_age".into(), 0.0);
            game.player.data.insert(
                "weapon_melee_mattock".into(),
                f64::from(object == "oMattock"),
            );
            game.player
                .data
                .insert("weapon_melee_id".into(), held_id as f64);
            spawn_pre(game, mattock);
        }
        "oSceptre" => {
            let id = game.world.spawn(
                "oPsychicWaveP",
                Vec2::new(pos.x + 12.0 * facing, pos.y - 2.0),
            );
            let e = game.world.entity_mut(id).unwrap();
            init_entity(e);
            owned(e, 3.0);
            e.vel.x = 6.0 * facing;
            game.player.data.insert("weapon_cooldown".into(), 20.0);
            game.sound("sndPsychic", pos);
        }
        _ => {
            let shotgun = object == "oShotgun";
            let web = object == "oWebCannon";
            let muzzle_x = if web {
                12.0 * facing
            } else if facing < 0.0 {
                -9.0
            } else {
                8.0
            };
            for _ in 0..if shotgun { 6 } else { 1 } {
                let speed = (6.0 + f64::from(game.rng.range(3))) * facing + game.player.vel.x;
                let y_speed = if shotgun {
                    game.rng.unit() - game.rng.unit()
                } else {
                    0.0
                };
                let id = game.world.spawn(
                    if web { "oWebBall" } else { "oBullet" },
                    Vec2::new(pos.x + muzzle_x, pos.y - 2.0),
                );
                let e = game.world.entity_mut(id).unwrap();
                init_entity(e);
                owned(e, if web { 0.0 } else { 4.0 });
                e.vel = Vec2::new(
                    if facing < 0.0 {
                        speed.min(-6.0)
                    } else {
                        speed.max(6.0)
                    },
                    y_speed,
                );
            }
            let flash = if facing < 0.0 {
                "oShotgunBlastLeft"
            } else {
                "oShotgunBlastRight"
            };
            let id = game
                .world
                .spawn(flash, Vec2::new(pos.x + muzzle_x, pos.y + 1.0));
            game.world.entity_mut(id).unwrap().timer = 3;
            if !matches!(
                game.player.state,
                PlayerState::Hanging | PlayerState::Climbing
            ) {
                game.player.vel.y -= 1.0;
                game.player.vel.x -= facing * if shotgun { 3.0 } else { 1.0 };
            }
            game.player
                .data
                .insert("weapon_cooldown".into(), if shotgun { 40.0 } else { 20.0 });
            game.sound("sndShotgun", pos);
        }
    }
    true
}

/// Attaches one inventory bomb to the drawn arrow. The source fuse is 80
/// simulation ticks and starts while the player is still drawing the bow.
/// Returns true when the bomb-button action belongs to the drawn bow.
pub fn arm_bomb_arrow(game: &mut Game) -> bool {
    let drawn = game.player.data.get("bow_armed").copied().unwrap_or(0.0) != 0.0;
    let holding_bow = game
        .player
        .held
        .and_then(|id| game.world.entity(id))
        .is_some_and(|item| item.object == "oBow");
    if !drawn || !holding_bow {
        return false;
    }
    if game
        .player
        .data
        .get("bow_bomb_armed")
        .copied()
        .unwrap_or(0.0)
        != 0.0
    {
        return true;
    }
    if game.player.bombs <= 0 || game.player.health <= 0 || game.player.stun > 0 {
        return true;
    }
    game.player.bombs -= 1;
    game.player.data.insert("bow_bomb_armed".into(), 1.0);
    game.player.data.insert("bow_bomb_fuse".into(), 80.0);
    true
}

/// oPlayer1 Step fires a drawn bow before releasing the item on stun/death.
pub fn fire_bow_on_drop(game: &mut Game) {
    if game.player.data.get("bow_armed").copied().unwrap_or(0.0) != 0.0
        && game
            .player
            .held
            .and_then(|id| game.world.entity(id))
            .is_some_and(|e| e.object == "oBow")
    {
        fire_bow(game);
    }
}

fn fire_bow(game: &mut Game) {
    let strength = game.player.data.get("bow_strength").copied().unwrap_or(0.0);
    let bomb_arrow = game
        .player
        .data
        .get("bow_bomb_armed")
        .copied()
        .unwrap_or(0.0)
        != 0.0;
    let bomb_fuse = game
        .player
        .data
        .get("bow_bomb_fuse")
        .copied()
        .unwrap_or(80.0);
    let facing = f64::from(game.player.facing);
    let mut pos = Vec2::new(game.player.pos.x + 14.0 * facing, game.player.pos.y);
    if game.world.solid_at(pos.x, pos.y) {
        pos = game.player.pos;
    }
    if game.player.state == PlayerState::Ducking {
        pos.y += 4.0;
    }
    let speed = game.player.vel.x + facing * (1.0 + strength);
    let id = game.world.spawn("oArrow", pos);
    let e = game.world.entity_mut(id).unwrap();
    init_entity(e);
    owned(e, 1.0);
    e.data.insert("safe".into(), 10.0);
    e.data.insert("thrown_by_player".into(), 1.0);
    e.vel.x = if facing < 0.0 {
        speed.min(-1.0)
    } else {
        speed.max(1.0)
    };
    e.sprite = if bomb_arrow {
        "sBombArrowRight"
    } else {
        "sArrowRight"
    }
    .into();
    if bomb_arrow {
        e.data.insert("bomb_arrow".into(), 1.0);
        e.data.insert("bomb_arrow_fuse".into(), bomb_fuse);
        game.player.data.insert("bow_bomb_armed".into(), 0.0);
        game.player.data.remove("bow_bomb_fuse");
    }
    if let Some(ammo) = game.data.get_mut("arrows") {
        *ammo = (*ammo - 1.0).max(0.0);
    }
    game.player.data.insert("weapon_cooldown".into(), 10.0);
    game.player.data.insert("bow_armed".into(), 0.0);
    game.player.data.insert("bow_strength".into(), 0.0);
    game.sound("sndArrowTrap", pos);
}

fn tile_object(game: &Game, pos: Vec2) -> &str {
    game.world
        .tile((pos.x / 16.0).floor() as i32, (pos.y / 16.0).floor() as i32)
        .map(|t| t.object.as_str())
        .unwrap_or("")
}

fn sprite_rectangle(sprite: &str, pos: Vec2) -> Option<Rect> {
    let definition = crate::catalog::sprite_def(sprite)?;
    let [left, top, right, bottom] = definition.bounds;
    Some(Rect::new(
        pos.x + f64::from(left - definition.origin[0]),
        pos.y + f64::from(top - definition.origin[1]),
        f64::from(right - left + 1),
        f64::from(bottom - top + 1),
    ))
}

/// Sweep the source sprite rectangle, including its asymmetric origin.
fn move_projectile(game: &Game, e: &mut Entity) -> bool {
    let steps = e.vel.x.abs().max(e.vel.y.abs()).ceil().max(1.0) as u32;
    let dx = e.vel.x / f64::from(steps);
    let dy = e.vel.y / f64::from(steps);
    for _ in 0..steps {
        let next = Vec2::new(e.pos.x + dx, e.pos.y + dy);
        let bounds = sprite_rectangle(&e.sprite, next)
            .unwrap_or_else(|| Rect::new(next.x - 1.0, next.y - 1.0, 2.0, 2.0));
        if game.world.collides_except(bounds, e.id) {
            return true;
        }
        e.pos = next;
    }
    false
}

fn move_item_projectile(game: &Game, e: &mut Entity, radius: f64) -> (bool, bool) {
    let steps = e.vel.x.abs().max(e.vel.y.abs()).ceil().max(1.0) as u32;
    let dx = e.vel.x / f64::from(steps);
    let dy = e.vel.y / f64::from(steps);
    let (mut wall, mut floor) = (false, false);
    for _ in 0..steps {
        if !wall {
            let next = Vec2::new(e.pos.x + dx, e.pos.y);
            if game.world.collides_except(
                Rect::new(next.x - radius, next.y - radius, radius * 2.0, radius * 2.0),
                e.id,
            ) {
                wall = true;
            } else {
                e.pos = next;
            }
        }
        if !floor {
            let next = Vec2::new(e.pos.x, e.pos.y + dy);
            if game.world.collides_except(
                Rect::new(next.x - radius, next.y - radius, radius * 2.0, radius * 2.0),
                e.id,
            ) {
                floor = true;
            } else {
                e.pos = next;
            }
        }
    }
    (wall, floor)
}

fn hit_player(game: &mut Game, projectile: &mut Entity, damage: i32) {
    if game.player.health <= 0 {
        return;
    }
    // Classic projectile damage bypasses contact-damage invulnerability.
    game.player.health -= damage;
    game.player.vel = Vec2::new(projectile.vel.x, -4.0);
    game.player.stun = 20;
    game.events.push(GameEvent::Damage { amount: damage });
    game.sound("sndHurt", game.player.pos);
    projectile.alive = false;
}

fn mattock_dig(game: &mut Game, e: &Entity) {
    enum DigTarget {
        Tile(i32, i32),
        Dynamic(u64),
    }
    let mut target = None;
    for point_y in [e.pos.y, e.pos.y + 9.0] {
        let x = (e.pos.x / 16.0).floor() as i32;
        let y = (point_y / 16.0).floor() as i32;
        if game.world.tile(x, y).is_some_and(Tile::is_solid) {
            target = Some(DigTarget::Tile(x, y));
            break;
        }
        if let Some(body) = game.world.entities.iter().find(|body| {
            body.id != e.id
                && body.alive
                && body.solid
                && !body.held
                && body.bounds().contains(Vec2::new(e.pos.x, point_y))
        }) {
            target = Some(DigTarget::Dynamic(body.id));
            break;
        }
    }
    if let Some(target) = target {
        match target {
            DigTarget::Tile(x, y) => {
                let center = Vec2::new(f64::from(x) * 16.0 + 8.0, f64::from(y) * 16.0 + 8.0);
                let protected = game.world.entities.iter().any(|metadata| {
                    metadata.alive
                        && metadata.object == "oTerrainMetadata"
                        && metadata.pos == center
                        && metadata.value("invincible") != 0.0
                });
                if protected || !game.world.tile(x, y).is_some_and(Tile::is_destructible) {
                    return;
                }
                game.world.set_tile(x as usize, y as usize, "");
            }
            DigTarget::Dynamic(id) => {
                let Some(body) = game.world.entity_mut(id) else {
                    return;
                };
                if body.value("invincible") != 0.0 {
                    return;
                }
                body.alive = false;
            }
        }
        // Source evaluates rand before checking the character flag.
        let breaks = game.rng.chance(1, 20) && !game.flags.contains("character_tunnelman");
        if breaks {
            let held = e.value("weapon_id") as u64;
            if let Some(item) = game.world.entity_mut(held) {
                item.alive = false;
            }
            if game.player.held == Some(held) {
                game.player.held = None;
            }
            let id = game.world.spawn("oMattockHead", e.pos);
            game.world.entity_mut(id).unwrap().vel.y = -2.0;
            game.sound("sndMattockBreak", e.pos);
        } else {
            game.sound("sndCrunch", e.pos);
        }
    }
}

pub fn tick(game: &mut Game) {
    let bow_strength = game.player.data.get("bow_strength").copied().unwrap_or(0.0);
    for item in game.world.entities.iter_mut().filter(|e| e.alive && e.held) {
        let base = match item.object.as_str() {
            "oBow" => "Bow",
            "oPistol" => "Pistol",
            "oShotgun" => "Shotgun",
            "oSceptre" => "Sceptre",
            "oMachete" => "Machete",
            "oMattock" => "Mattock",
            "oWebCannon" => "WebCannon",
            _ => continue,
        };
        item.sprite = if item.object == "oWebCannon" {
            format!(
                "sWebCannon{}",
                if game.player.facing < 0 { "L" } else { "R" }
            )
        } else {
            format!(
                "s{}{}",
                base,
                if game.player.facing < 0 {
                    "Left"
                } else {
                    "Right"
                }
            )
        };
        if item.object == "oBow" {
            item.animation = if bow_strength >= 10.0 {
                3.0
            } else if bow_strength > 6.0 {
                2.0
            } else if bow_strength > 2.0 {
                1.0
            } else {
                0.0
            };
        }
    }
    if let Some(cooldown) = game.player.data.get_mut("weapon_cooldown") {
        *cooldown = (*cooldown - 1.0).max(0.0);
    }
    if game
        .player
        .data
        .get("bow_bomb_armed")
        .copied()
        .unwrap_or(0.0)
        != 0.0
    {
        // A release transfers the remaining fuse to the projectile below;
        // otherwise cooking continues even after the bow was dropped.
        let releasing = game.player.data.get("bow_armed").copied().unwrap_or(0.0) != 0.0
            && game.player.data.get("attack_held").copied().unwrap_or(0.0) == 0.0
            && game
                .player
                .held
                .and_then(|id| game.world.entity(id))
                .is_some_and(|item| item.object == "oBow");
        if !releasing {
            let fuse = game
                .player
                .data
                .entry("bow_bomb_fuse".into())
                .or_insert(80.0);
            *fuse -= 1.0;
            if *fuse <= 0.0 {
                game.world.spawn("oExplosion", game.player.pos);
                game.player.data.insert("bow_bomb_armed".into(), 0.0);
                game.player.data.remove("bow_bomb_fuse");
            }
        }
    }
    if game.player.data.get("bow_armed").copied().unwrap_or(0.0) != 0.0 {
        let holding_bow = game
            .player
            .held
            .and_then(|id| game.world.entity(id))
            .is_some_and(|e| e.object == "oBow");
        if !holding_bow {
            game.player.data.insert("bow_armed".into(), 0.0);
            game.player.data.insert("bow_strength".into(), 0.0);
        } else if game.player.data.get("attack_held").copied().unwrap_or(0.0) != 0.0 {
            let strength = game.player.data.entry("bow_strength".into()).or_default();
            *strength = (*strength + 0.2).min(12.0);
        } else {
            fire_bow(game);
        }
    }
    if (game.player.health <= 0 || game.player.stun > 0)
        && game.player.data.contains_key("weapon_melee_age")
    {
        if let Some(id) = game.player.data.get("weapon_melee_id").copied() {
            if let Some(held) = game.world.entity_mut(id as u64) {
                held.data.insert("hidden".into(), 0.0);
            }
        }
        game.player.data.remove("weapon_melee_age");
        game.player.attack_timer = 0;
    }
    if let Some(age) = game.player.data.get("weapon_melee_age").copied() {
        let age = age as u32 + 1;
        let mattock = game
            .player
            .data
            .get("weapon_melee_mattock")
            .copied()
            .unwrap_or(0.0)
            != 0.0;
        game.player.sprite = attack_sprite(game).into();
        game.player.animation = if mattock { 0.2 } else { 1.0 } + f64::from(age - 1);
        let before = if mattock { 0.2 } else { 1.0 } + f64::from(age.saturating_sub(2));
        // The original guard explicitly names oMachetePre for both weapons.
        // oMattockPre directly inherits oWhipPre, so it does not satisfy it.
        if age > 1
            && before < 2.0
            && !game
                .world
                .entities
                .iter()
                .any(|e| e.alive && crate::catalog::is_a(&e.object, "oMachetePre"))
        {
            spawn_pre(game, mattock);
        }
        // At age 6 the pre-advance index first exceeds 4 for both weapons.
        if age == 6 {
            let object = if mattock { "oMattockHit" } else { "oSlash" };
            let facing = game.player.facing;
            let id = game.world.spawn(
                object,
                Vec2::new(
                    game.player.pos.x + 16.0 * f64::from(facing),
                    game.player.pos.y,
                ),
            );
            let e = game.world.entity_mut(id).unwrap();
            owned(e, 2.0);
            e.facing = facing;
            e.timer = if mattock { 6 } else { 3 };
            e.data.insert("puncture".into(), 1.0);
            e.data.insert(
                "weapon_id".into(),
                game.player
                    .data
                    .get("weapon_melee_id")
                    .copied()
                    .unwrap_or(0.0),
            );
            e.sprite = if mattock {
                format!("sMattockHit{}", if facing < 0 { "L" } else { "R" })
            } else {
                format!("sSlash{}", if facing < 0 { "Left" } else { "Right" })
            };
            game.sound("sndWhip", game.player.pos);
        }
        if age >= if mattock { 12 } else { 11 } {
            if let Some(id) = game.player.data.get("weapon_melee_id").copied() {
                if let Some(held) = game.world.entity_mut(id as u64) {
                    held.data.insert("hidden".into(), 0.0);
                }
            }
            game.player.data.remove("weapon_melee_age");
        } else {
            game.player
                .data
                .insert("weapon_melee_age".into(), f64::from(age));
        }
    }
    let ids: Vec<u64> = game
        .world
        .entities
        .iter()
        .filter(|e| {
            e.alive
                && matches!(
                    e.object.as_str(),
                    "oBullet"
                        | "oArrow"
                        | "oFlare"
                        | "oPsychicWaveP"
                        | "oWebBall"
                        | "oMachetePre"
                        | "oMattockPre"
                        | "oSlash"
                        | "oMattockHit"
                        | "oShotgunBlastLeft"
                        | "oShotgunBlastRight"
                )
        })
        .map(|e| e.id)
        .collect();
    for id in ids {
        let Some(mut e) = game.world.entity(id).cloned() else {
            continue;
        };
        init_entity(&mut e);
        match e.object.as_str() {
            "oBullet" => {
                if move_projectile(game, &mut e) {
                    e.alive = false;
                    game.sound("sndHit", e.pos);
                }
                let player_mask = sprite_rectangle(&game.player.sprite, game.player.pos)
                    .unwrap_or_else(|| game.player.bounds());
                let exiting = matches!(
                    game.player.sprite.as_str(),
                    "sPExit" | "sDamselExit" | "sTunnelExit"
                );
                if e.alive && !exiting && e.bounds().intersects(player_mask) {
                    hit_player(game, &mut e, 4);
                }
            }
            "oArrow" => {
                if e.value("safe") > 0.0 {
                    e.data.insert("safe".into(), e.value("safe") - 1.0);
                }
                if !e.held && e.value("stuck") == 0.0 {
                    let speed = e.vel.x;
                    let (wall, floor) = move_item_projectile(game, &mut e, 4.0);
                    if wall && speed.abs() > 6.0 {
                        e.data.insert("stuck".into(), 1.0);
                        e.vel = Vec2::default();
                    } else {
                        if wall {
                            e.vel.x *= -0.5;
                            e.data.insert("gravity".into(), 0.6);
                        }
                        if floor {
                            e.vel.y = if e.vel.y.abs() > 1.0 {
                                -e.vel.y * 0.5
                            } else {
                                0.0
                            };
                            e.vel.x *= 0.3;
                            e.data.insert("gravity".into(), 0.6);
                        } else {
                            e.vel.y = (e.vel.y + e.value("gravity")).min(8.0);
                        }
                    }
                }
                e.data.insert("rotation".into(), e.vel.y.atan2(e.vel.x));
                if e.alive
                    && !e.held
                    && e.vel.x.abs() > 3.0
                    && e.value("safe") == 0.0
                    && e.bounds().intersects(Rect::new(
                        game.player.pos.x - 8.0,
                        game.player.pos.y - 8.0,
                        16.0,
                        16.0,
                    ))
                {
                    hit_player(game, &mut e, 2);
                }

                let bow = game
                    .player
                    .held
                    .and_then(|held| game.world.entity(held))
                    .is_some_and(|held| held.object == "oBow");
                if e.alive
                    && !e.held
                    && bow
                    && game.player.health > 0
                    && game.player.stun == 0
                    && e.value("stuck") == 0.0
                    && e.vel.x.abs() < 1.0
                    && e.vel.y.abs() < 1.0
                    && e.bounds().intersects(Rect::new(
                        game.player.pos.x - 8.0,
                        game.player.pos.y - 8.0,
                        16.0,
                        16.0,
                    ))
                {
                    e.alive = false;
                    *game.data.entry("arrows".into()).or_default() += 1.0;
                    game.sound("sndPickup", e.pos);
                }
            }
            "oFlare" => {
                if tile_object(game, e.pos) == "oWater"
                    || game.world.entities.iter().any(|water| {
                        water.alive && water.object == "oWater" && water.bounds().contains(e.pos)
                    })
                {
                    e.alive = false;
                    if game.player.held == Some(id) {
                        game.player.held = None;
                    }
                    game.sound("sndSplash", e.pos);
                } else if !e.held {
                    let (wall, floor) = move_item_projectile(game, &mut e, 4.0);
                    if wall {
                        e.vel.x *= -0.5;
                    }
                    if floor {
                        e.vel.x *= 0.3;
                        e.vel.y = if e.vel.y.abs() > 1.0 {
                            -e.vel.y * 0.5
                        } else {
                            0.0
                        };
                    } else {
                        e.vel.y = (e.vel.y + 0.6).min(8.0);
                    }
                }
                e.animation += 0.3;
                e.data.insert("light_radius".into(), 64.0);
            }
            "oPsychicWaveP" => {
                if e.timer > 0 {
                    e.timer -= 1;
                    e.pos.x += e.vel.x;
                } else {
                    let target = game
                        .world
                        .entities
                        .iter()
                        .filter(|target| {
                            target.alive && crate::catalog::is_a(&target.object, "oEnemy")
                        })
                        .min_by(|a, b| e.pos.distance(a.pos).total_cmp(&e.pos.distance(b.pos)));
                    if let Some(target) = target {
                        let body = target.bounds();
                        let delta = Vec2::new(
                            body.x + body.w / 2.0 - e.pos.x,
                            body.y + body.h / 2.0 - e.pos.y,
                        );
                        let len = delta.x.hypot(delta.y);
                        if len > 0.0 {
                            e.vel = Vec2::new(2.0 * delta.x / len, 2.0 * delta.y / len);
                        }
                    } else {
                        let len = e.vel.x.hypot(e.vel.y);
                        if len > 0.0 {
                            e.vel.x *= 2.0 / len;
                            e.vel.y *= 2.0 / len;
                        }
                    }
                    e.pos.x += e.vel.x;
                    e.pos.y += e.vel.y;
                }
                e.animation += 0.25;
            }
            "oWebBall" => {
                if e.state == 0 {
                    let collided = move_projectile(game, &mut e);
                    e.vel.y = (e.vel.y + 0.2).min(6.0);
                    let life = e.value("life") - 1.0;
                    e.data.insert("life".into(), life);
                    let target = game.world.entities.iter().any(|target| {
                        target.alive
                            && target.id != id
                            && (crate::catalog::is_a(&target.object, "oEnemy")
                                && target.object != "oGiantSpider"
                                || matches!(target.object.as_str(), "oWater" | "oWeb"))
                            && target.bounds().intersects(e.bounds())
                    });
                    if collided || target || life < 0.0 {
                        e.state = 1;
                        e.timer = 5;
                        e.vel = Vec2::default();
                        e.sprite = "sWebCreate".into();
                    }
                } else if e.timer > 0 {
                    e.timer -= 1;
                } else {
                    e.alive = false;
                    let web = game.world.spawn("oWeb", e.pos);
                    game.world
                        .entity_mut(web)
                        .unwrap()
                        .data
                        .insert("dying".into(), 1.0);
                }
            }
            "oMachetePre" | "oMattockPre" | "oSlash" | "oMattockHit" => {
                let pre = e.object.ends_with("Pre");
                e.pos = Vec2::new(
                    game.player.pos.x + 16.0 * f64::from(e.facing) * if pre { -1.0 } else { 1.0 },
                    game.player.pos.y,
                );
                if e.timer > 0 {
                    e.timer -= 1;
                } else {
                    if e.object == "oMattockHit" {
                        mattock_dig(game, &e);
                    }
                    e.alive = false;
                }
                e.animation += if e.object == "oMattockHit" { 0.5 } else { 1.0 };
                selected_sprite_bounds(&mut e);
            }
            _ => {
                if e.timer > 0 {
                    e.timer -= 1;
                } else {
                    e.alive = false;
                }
            }
        }
        if e.alive && e.object == "oArrow" && e.value("bomb_arrow") != 0.0 {
            let fuse = e.value("bomb_arrow_fuse") - 1.0;
            e.data.insert("bomb_arrow_fuse".into(), fuse);
            if fuse <= 0.0 {
                game.world.spawn("oExplosion", e.pos);
                if game.player.held == Some(id) {
                    game.player.held = None;
                }
                e.alive = false;
            }
        }
        if e.pos.x < -32.0
            || e.pos.y < -64.0
            || e.pos.x > game.world.width as f64 * 16.0 + 32.0
            || e.pos.y > game.world.height as f64 * 16.0 + 64.0
        {
            e.alive = false;
        }
        if let Some(original) = game
            .world
            .entities
            .iter_mut()
            .find(|original| original.id == id)
        {
            *original = e;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn armed(object: &str) -> (Game, u64) {
        let mut game = Game::shell(7, RulesPreset::Classic);
        game.player.pos = Vec2::new(80.0, 80.0);
        let id = game.world.spawn(object, game.player.pos);
        game.player.held = Some(id);
        game.world.entity_mut(id).unwrap().held = true;
        (game, id)
    }
    #[test]
    fn shotgun_emits_six_and_applies_recoil_once() {
        let (mut game, id) = armed("oShotgun");
        let input = InputFrame {
            attack: true,
            attack_pressed: true,
            ..Default::default()
        };
        assert!(use_weapon(&mut game, id, &input));
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|e| e.object == "oBullet")
                .count(),
            6
        );
        assert_eq!(game.player.vel, Vec2::new(-3.0, -1.0));
        use_weapon(&mut game, id, &input);
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|e| e.object == "oBullet")
                .count(),
            6
        );
    }
    #[test]
    fn bow_charges_until_release_and_consumes_one_arrow() {
        let (mut game, id) = armed("oBow");
        game.data.insert("arrows".into(), 6.0);
        game.player.data.insert("attack_held".into(), 1.0);
        use_weapon(&mut game, id, &InputFrame::default());
        for _ in 0..60 {
            tick(&mut game);
        }
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|e| e.object == "oArrow")
                .count(),
            0
        );
        game.player.data.insert("attack_held".into(), 0.0);
        tick(&mut game);
        let arrow = game
            .world
            .entities
            .iter()
            .find(|e| e.object == "oArrow")
            .unwrap();
        assert!((arrow.vel.x - 13.0).abs() < 1e-10);
        assert_eq!(game.data["arrows"], 5.0);
        tick(&mut game);
        assert_eq!(game.data["arrows"], 5.0);
    }
    #[test]
    fn fast_bullet_does_not_cross_wall() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.world.set_tile(5, 5, "oBlock");
        let id = game.world.spawn("oBullet", Vec2::new(70.0, 88.0));
        game.world.entity_mut(id).unwrap().vel.x = 30.0;
        tick(&mut game);
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
    fn flare_is_extinguished_in_water_and_releases_hand() {
        let (mut game, id) = armed("oFlare");
        game.world.set_tile(5, 5, "oWater");
        tick(&mut game);
        assert!(game.world.entity(id).is_none());
        assert_eq!(game.player.held, None);
    }
    #[test]
    fn mattock_cannot_dig_invincible_terrain() {
        let mut game = Game::shell(3, RulesPreset::Classic);
        game.world.set_tile(5, 5, "oSolid");
        let e = Entity::new(1, "oMattockHit", Vec2::new(88.0, 88.0));
        mattock_dig(&mut game, &e);
        assert_eq!(game.world.tile(5, 5).unwrap().object, "oSolid");
        game.world.set_tile(5, 5, "oBlock");
        mattock_dig(&mut game, &e);
        assert_eq!(game.world.tile(5, 5).unwrap().object, "");
    }
    #[test]
    fn held_and_innate_mattocks_preserve_invincible_boss_floor_metadata() {
        for tunnel in [false, true] {
            let mut game = Game::shell(3, RulesPreset::Classic);
            if tunnel {
                game.flags.insert("character_tunnelman".into());
            }
            game.world.set_tile(5, 5, "oTemple");
            let metadata = game.world.spawn("oTerrainMetadata", Vec2::new(88.0, 88.0));
            game.world
                .entity_mut(metadata)
                .unwrap()
                .data
                .insert("invincible".into(), 1.0);
            let id = game.world.spawn("oMattockHit", Vec2::new(88.0, 88.0));
            game.player.pos = Vec2::new(72.0, 88.0);
            let hit = game.world.entity_mut(id).unwrap();
            hit.timer = 0;
            hit.data
                .insert("character_mattock".into(), f64::from(tunnel));
            tick(&mut game);
            assert_eq!(game.world.tile(5, 5).unwrap().object, "oTemple");
            assert!(game.world.entity(metadata).is_some());
            // The same tile is normally diggable when its override is removed.
            game.world
                .entity_mut(metadata)
                .unwrap()
                .data
                .insert("invincible".into(), 0.0);
            let hit = Entity::new(999, "oMattockHit", Vec2::new(88.0, 88.0));
            mattock_dig(&mut game, &hit);
            assert_eq!(game.world.tile(5, 5).unwrap().object, "");
        }
    }

    #[test]
    fn projectile_damage_bypasses_contact_invulnerability() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.player.pos = Vec2::new(88.0, 88.0);
        game.player.invincible = 50;
        game.player.health = 8;
        let id = game.world.spawn("oBullet", Vec2::new(80.0, 88.0));
        game.world.entity_mut(id).unwrap().vel.x = 8.0;
        tick(&mut game);
        assert_eq!(game.player.health, 4);
        assert_eq!(game.player.stun, 20);
        assert!(game.world.entity(id).is_none());
    }
    #[test]
    fn safe_arrow_cannot_hurt_shooter() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.player.pos = Vec2::new(88.0, 88.0);
        let id = game.world.spawn("oArrow", Vec2::new(80.0, 88.0));
        let arrow = game.world.entity_mut(id).unwrap();
        arrow.vel.x = 8.0;
        arrow.data.insert("safe".into(), 10.0);
        tick(&mut game);
        assert_eq!(game.player.health, 4);
        assert_eq!(game.world.entity(id).unwrap().value("safe"), 9.0);
        assert_eq!(game.world.entity(id).unwrap().vel.y, 0.2);
    }
    #[test]
    fn fast_arrow_sticks_in_wall_instead_of_bouncing() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.world.set_tile(5, 5, "oBlock");
        let id = game.world.spawn("oArrow", Vec2::new(70.0, 88.0));
        game.world.entity_mut(id).unwrap().vel.x = 13.0;
        tick(&mut game);
        let arrow = game.world.entity(id).unwrap();
        assert_eq!(arrow.value("stuck"), 1.0);
        assert_eq!(arrow.vel, Vec2::default());
    }
    #[test]
    fn machete_has_windup_before_forward_slash() {
        let (mut game, id) = armed("oMachete");
        use_weapon(&mut game, id, &InputFrame::default());
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|e| e.object == "oMachetePre")
                .count(),
            1
        );
        for _ in 0..5 {
            tick(&mut game);
        }
        assert!(!game.world.entities.iter().any(|e| e.object == "oSlash"));
        tick(&mut game);
        assert!(
            game.world
                .entities
                .iter()
                .any(|e| e.object == "oSlash" && e.alive && e.value("damage") == 2.0)
        );
    }
    #[test]
    fn mattock_requires_ground_and_only_slows_the_onset_tick() {
        let (mut game, id) = armed("oMattock");
        use_weapon(&mut game, id, &InputFrame::default());
        assert_eq!(game.player.attack_timer, 0);
        game.player.grounded = true;
        use_weapon(&mut game, id, &InputFrame::default());
        assert_eq!(game.player.attack_timer, 12);
        tick(&mut game);
        assert_eq!(game.player.animation, 0.2);
        tick(&mut game);
        assert_eq!(game.player.animation, 1.2);
        for _ in 0..3 {
            tick(&mut game);
        }
        assert!(
            !game
                .world
                .entities
                .iter()
                .any(|e| e.object == "oMattockHit")
        );
        tick(&mut game);
        assert!(
            game.world
                .entities
                .iter()
                .any(|e| e.object == "oMattockHit" && e.alive)
        );
    }
    #[test]
    fn bomb_arrow_consumes_one_bomb_and_preserves_cooked_fuse() {
        let (mut game, id) = armed("oBow");
        game.data.insert("arrows".into(), 6.0);
        game.player.data.insert("attack_held".into(), 1.0);
        use_weapon(&mut game, id, &InputFrame::default());
        assert!(arm_bomb_arrow(&mut game));
        assert!(arm_bomb_arrow(&mut game));
        assert_eq!(game.player.bombs, 3);
        for _ in 0..10 {
            tick(&mut game);
        }
        assert_eq!(game.player.data["bow_bomb_fuse"], 70.0);
        game.player.data.insert("attack_held".into(), 0.0);
        tick(&mut game);
        let arrow = game
            .world
            .entities
            .iter()
            .find(|e| e.object == "oArrow")
            .unwrap();
        assert_eq!(arrow.sprite, "sBombArrowRight");
        assert_eq!(arrow.value("bomb_arrow"), 1.0);
        assert_eq!(arrow.value("bomb_arrow_fuse"), 69.0);
        assert_eq!(game.data["arrows"], 5.0);
        assert_eq!(game.player.data["bow_bomb_armed"], 0.0);
    }
    #[test]
    fn bomb_arrow_explodes_on_fuse_even_when_stuck_in_wall() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        let id = game.world.spawn("oArrow", Vec2::new(88.0, 88.0));
        let arrow = game.world.entity_mut(id).unwrap();
        arrow.data.insert("bomb_arrow".into(), 1.0);
        arrow.data.insert("bomb_arrow_fuse".into(), 2.0);
        arrow.data.insert("stuck".into(), 1.0);
        tick(&mut game);
        assert!(game.world.entity(id).is_some());
        tick(&mut game);
        assert!(game.world.entity(id).is_none());
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|e| e.object == "oExplosion")
                .count(),
            1
        );
    }
    #[test]
    fn cooking_bomb_arrow_too_long_explodes_in_hand() {
        let (mut game, id) = armed("oBow");
        game.data.insert("arrows".into(), 6.0);
        game.player.data.insert("attack_held".into(), 1.0);
        use_weapon(&mut game, id, &InputFrame::default());
        arm_bomb_arrow(&mut game);
        for _ in 0..80 {
            tick(&mut game);
        }
        assert!(
            game.world
                .entities
                .iter()
                .any(|e| e.object == "oExplosion" && e.pos == game.player.pos)
        );
        assert_eq!(game.player.data["bow_bomb_armed"], 0.0);
    }
    #[test]
    fn bullets_collide_with_dynamic_solids_and_mattock_digs_them() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        let block = game.world.spawn("oPushBlock", Vec2::new(80.0, 80.0));
        game.world.entity_mut(block).unwrap().solid = true;
        let bullet = game.world.spawn("oBullet", Vec2::new(70.0, 88.0));
        game.world.entity_mut(bullet).unwrap().vel.x = 30.0;
        tick(&mut game);
        assert!(game.world.entity(bullet).is_none());
        let hit = Entity::new(999, "oMattockHit", Vec2::new(88.0, 88.0));
        mattock_dig(&mut game, &hit);
        assert!(game.world.entity(block).is_none());
    }
    #[test]
    fn mattock_hit_finishes_once_after_attack_lifecycle_ends() {
        let (mut game, id) = armed("oMattock");
        game.player.grounded = true;
        game.world.set_tile(6, 5, "oBlock");
        use_weapon(&mut game, id, &InputFrame::default());
        for _ in 0..11 {
            tick(&mut game);
        }
        assert_eq!(game.world.tile(6, 5).unwrap().object, "oBlock");
        tick(&mut game);
        assert_eq!(game.world.tile(6, 5).unwrap().object, "");
        assert!(!game.player.data.contains_key("weapon_melee_age"));
        game.world.set_tile(6, 5, "oBlock");
        tick(&mut game);
        assert_eq!(game.world.tile(6, 5).unwrap().object, "oBlock");
    }
    #[test]
    fn tunnelman_mattock_consumes_durability_rng_without_breaking() {
        let (mut game, id) = armed("oMattock");
        game.flags.insert("character_tunnelman".into());
        game.world.set_tile(6, 5, "oBlock");
        let mut expected = game.rng.clone();
        expected.chance(1, 20);
        let mut hit = Entity::new(999, "oMattockHit", Vec2::new(104.0, 88.0));
        hit.data.insert("weapon_id".into(), id as f64);
        mattock_dig(&mut game, &hit);
        assert!(game.world.entity(id).is_some());
        assert_eq!(game.rng.next_u64(), expected.next_u64());
    }
    #[test]
    fn melee_uses_selected_character_attack_sprite() {
        let (mut game, id) = armed("oMachete");
        game.flags.insert("character_damsel".into());
        use_weapon(&mut game, id, &InputFrame::default());
        assert_eq!(game.player.sprite, "sDamselAttackL");
        game.flags.insert("character_tunnelman".into());
        tick(&mut game);
        assert_eq!(game.player.sprite, "sTunnelAttackL");
    }
    fn flat_room(object: &str) -> (Game, u64) {
        let (mut game, _) = armed(object);
        game.world = World::new(20, 20, Area::Mines, 1);
        for x in 0..20 {
            game.world.set_tile(x, 10, "oBlock");
        }
        game.player.pos = Vec2::new(80.0, 152.0);
        game.player.state = PlayerState::Standing;
        game.player.grounded = true;
        let id = game.world.spawn(object, game.player.pos);
        game.world.entity_mut(id).unwrap().held = true;
        game.player.held = Some(id);
        (game, id)
    }
    #[test]
    fn game_step_melee_collision_uses_selected_left_sprite_origin() {
        let (mut game, _) = flat_room("oMachete");
        game.player.facing = -1;
        game.step(&InputFrame {
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        let rear = game
            .world
            .entities
            .iter()
            .find(|e| e.object == "oMachetePre")
            .unwrap();
        assert_eq!(rear.sprite, "sMachetePreL");
        assert_eq!(
            rear.bounds(),
            Rect::new(rear.pos.x - 8.0, rear.pos.y - 8.0, 7.0, 9.0)
        );
        for _ in 0..5 {
            game.step(&InputFrame::default());
        }
        let slash = game
            .world
            .entities
            .iter()
            .find(|e| e.object == "oSlash")
            .unwrap();
        assert_eq!(slash.sprite, "sSlashLeft");
        assert_eq!(
            slash.bounds(),
            Rect::new(slash.pos.x - 3.0, slash.pos.y - 20.0, 29.0, 24.0)
        );
    }
    #[test]
    fn game_step_mattock_inhibits_twenty_jumps_but_machete_allows_jump() {
        let (mut game, _) = flat_room("oMattock");
        game.step(&InputFrame {
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        let grounded_y = game.player.pos.y;
        for _ in 0..20 {
            game.step(&InputFrame {
                jump: true,
                jump_pressed: true,
                ..Default::default()
            });
            assert_eq!(game.player.pos.y, grounded_y);
        }
        assert_eq!(game.player.data["cantJump"], 0.0);
        game.step(&InputFrame {
            jump: true,
            jump_pressed: true,
            ..Default::default()
        });
        assert!(game.player.vel.y < 0.0);
        let (mut machete, _) = flat_room("oMachete");
        machete.step(&InputFrame {
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        machete.step(&InputFrame {
            jump: true,
            jump_pressed: true,
            ..Default::default()
        });
        assert!(machete.player.vel.y < 0.0);
    }
    #[test]
    fn game_step_stun_cancels_pending_forward_slash() {
        let (mut game, _) = flat_room("oMachete");
        game.step(&InputFrame {
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        game.player.stun = 10;
        for _ in 0..6 {
            game.step(&InputFrame::default());
        }
        assert!(!game.player.data.contains_key("weapon_melee_age"));
        assert!(!game.world.entities.iter().any(|e| e.object == "oSlash"));
    }
    #[test]
    fn game_step_stun_releases_drawn_bow_as_arrow() {
        let (mut game, _) = flat_room("oBow");
        game.data.insert("arrows".into(), 6.0);
        game.step(&InputFrame {
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        game.player.stun = 10;
        game.step(&InputFrame::default());
        assert!(game.player.held.is_none());
        assert_eq!(game.data["arrows"], 5.0);
        assert!(game.world.entities.iter().any(|e| e.object == "oArrow"));
    }
    #[test]
    fn source_weapon_motion_bounds_are_independent_of_sprite_masks() {
        let mut mattock = Entity::new(1, "oMattock", Vec2::default());
        init_entity(&mut mattock);
        assert_eq!(mattock.value("bboxTop"), -6.0);
        assert_eq!(mattock.value("bboxBottom"), 6.0);
        for name in ["oArrow", "oFlare"] {
            let mut item = Entity::new(2, name, Vec2::default());
            init_entity(&mut item);
            assert_eq!(item.value("canPickUp"), 1.0);
            assert_eq!(item.value("bboxLeft"), -4.0);
            assert_eq!(item.value("bboxRight"), 4.0);
        }
    }
    #[test]
    fn bullet_terrain_collision_preserves_source_vertical_sprite_offset() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.world.set_tile(5, 5, "oBlock");
        let id = game.world.spawn("oBullet", Vec2::new(70.0, 79.0));
        game.world.entity_mut(id).unwrap().vel.x = 12.0;
        tick(&mut game);
        assert!(game.world.entity(id).is_none());
    }
    #[test]
    fn webball_terrain_collision_uses_full_nine_pixel_mask_rectangle() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.world.set_tile(5, 5, "oBlock");
        let id = game.world.spawn("oWebBall", Vec2::new(70.0, 78.0));
        game.world.entity_mut(id).unwrap().vel.x = 12.0;
        tick(&mut game);
        assert_eq!(game.world.entity(id).unwrap().state, 1);
    }
    #[test]
    fn arrow_hurt_query_uses_source_sixteen_pixel_player_rectangle() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.player.pos = Vec2::new(88.0, 88.0);
        let id = game.world.spawn("oArrow", Vec2::new(70.0, 88.0));
        game.world.entity_mut(id).unwrap().vel.x = 8.0;
        tick(&mut game);
        assert_eq!(game.player.health, 2);
        assert!(game.world.entity(id).is_none());
    }
    #[test]
    fn mattock_rear_effect_preserves_original_machete_guard_hierarchy() {
        let (mut game, id) = armed("oMattock");
        game.player.grounded = true;
        assert!(!crate::catalog::is_a("oMattockPre", "oMachetePre"));
        use_weapon(&mut game, id, &InputFrame::default());
        for _ in 0..3 {
            tick(&mut game);
        }
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == "oMattockPre")
                .count(),
            3
        );
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|e| e.object == "oMattockPre")
                .count(),
            3
        );
    }
    #[test]
    fn held_webcannon_uses_existing_source_direction_sprites() {
        let (mut game, id) = armed("oWebCannon");
        tick(&mut game);
        assert_eq!(game.world.entity(id).unwrap().sprite, "sWebCannonR");
        game.player.facing = -1;
        tick(&mut game);
        assert_eq!(game.world.entity(id).unwrap().sprite, "sWebCannonL");
    }
    #[test]
    fn sceptre_homes_to_enemy_family_instead_of_damsel_item() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.world.spawn("oDamsel", Vec2::new(100.0, 60.0));
        game.world.spawn("oSnake", Vec2::new(160.0, 100.0));
        let id = game.world.spawn("oPsychicWaveP", Vec2::new(100.0, 100.0));
        game.world.entity_mut(id).unwrap().timer = 0;
        tick(&mut game);
        let wave = game.world.entity(id).unwrap();
        assert!(wave.vel.x > 0.0);
        assert!(wave.vel.y >= 0.0);
        assert!((wave.vel.x.hypot(wave.vel.y) - 2.0).abs() < 1e-12);
    }
}
