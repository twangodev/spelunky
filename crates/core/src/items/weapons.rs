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
/// Preserve movement bounds and origin on both facing directions. Automatic
/// Collision events independently use the current frame's precise alpha mask.
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
    let id = game.spawn(
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
    init_entity(e);
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
    // These Creates inherit DrawnSprite first. Specialized front/rear
    // hitboxes then overwrite Whip's type/damage; only front Whip declares
    // puncture, so rear hitboxes intentionally leave that property absent.
    let hitbox_create = match e.object.as_str() {
        "oWhip" => Some(("Whip", 1.0, Some(0.0))),
        "oWhipPre" => Some(("Whip", 1.0, None)),
        "oSlash" => Some(("Machete", 2.0, Some(1.0))),
        "oMachetePre" => Some(("Machete", 2.0, None)),
        "oMattockHit" => Some(("Mattock", 2.0, Some(1.0))),
        "oMattockPre" => Some(("Mattock", 2.0, None)),
        _ => None,
    };
    if let Some((kind, damage, puncture)) = hitbox_create {
        e.data.insert("blinkToggle".into(), 0.0);
        e.text.insert("type".into(), kind.into());
        e.data.insert("damage".into(), damage);
        if let Some(puncture) = puncture {
            e.data.insert("puncture".into(), puncture);
        }
    }
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
        "oBow" => e.set_image_speed(0.0),
        "oArrow" => {
            e.data.insert("gravity".into(), 0.2);
        }
        "oFlare" => {
            e.set_image_speed(0.3);
            e.alarms[0] = 1;
            e.data.entry("dist_to_player".into()).or_insert(0.0);
            e.data.insert("gravity".into(), 0.6);
            e.data.insert("invincible".into(), 1.0);
        }
        "oPsychicWaveP" => {
            e.set_image_speed(0.25);
            e.timer = 5;
            e.data.insert("damage".into(), 3.0);
        }
        "oWhipPre" | "oMachetePre" | "oMattockPre" => {
            e.alarms[0] = 3;
        }
        "oMattockHit" => e.set_image_speed(0.5),
        "oSlash" => e.set_image_speed(1.0),
        "oShotgunBlastLeft" | "oShotgunBlastRight" => e.set_image_speed(0.8),
        "oWebBall" => {
            e.data.insert("invincible".into(), 1.0);
            // Entity initialization has no RNG access. The first weapon
            // update supplies the source Create event's inclusive rand(20,100).
            if !e.data.contains_key("life") {
                e.data.insert("web_life_pending".into(), 1.0);
            }
        }
        _ => {}
    }
}

/// Synchronous oWebBall Create. Callers must run this before applying their
/// launch overrides: its four random draws also affect later gameplay RNG.
pub(crate) fn initialize_web_ball(game: &mut Game, e: &mut Entity) {
    if e.value("web_create_initialized") != 0.0 {
        return;
    }
    init_entity(e);
    e.vel.y = -(game.rng.unit() * 3.0 + 1.0);
    e.vel.x = f64::from(1 + game.rng.range(3));
    if game.rng.chance(1, 2) {
        e.vel.x *= -1.0;
    }
    e.data
        .insert("life".into(), f64::from(20 + game.rng.range(81)));
    e.data.remove("web_life_pending");
    e.data.insert("web_create_initialized".into(), 1.0);
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
    let ducking = super::player_is_ducking(&game.player);
    let firing = game
        .player
        .data
        .get("weapon_cooldown")
        .copied()
        .unwrap_or(0.0);
    let melee = matches!(object.as_str(), "oMachete" | "oMattock");
    if (melee && (ducking || game.player.attack_timer > 0))
        || (!melee && object != "oBow" && (ducking || firing != 0.0))
    {
        return true;
    }
    let facing = f64::from(game.player.facing);
    let pos = game.player.pos;
    match object.as_str() {
        "oBow" => {
            if input.down {
                return true;
            }
            let armed = game.player.data.get("bow_armed").copied().unwrap_or(0.0) != 0.0;
            let arrows = game.data.get("arrows").copied().unwrap_or(0.0);
            if !ducking && firing == 0.0 && !armed && arrows > 0.0 {
                game.player.data.insert("bow_armed".into(), 1.0);
                game.player.data.insert("bow_strength".into(), 0.0);
                game.sound("sndBowPull", pos);
            } else if arrows <= 0.0 {
                crate::enemies::shop_dialogue::say(game, "I'M OUT OF ARROWS!", "", 80);
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
            game.player.set_sprite(attack_sprite(game));
            game.player.animation = 0.0;
            game.player.set_image_speed(if mattock { 0.2 } else { 1.0 });
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
        }
        "oSceptre" => {
            for _ in 0..3 {
                let velocity = Vec2::new(
                    facing * f64::from(1 + game.rng.range(3)),
                    -game.rng.unit() * 2.0,
                );
                crate::world_events::spawn_psychic_create_p(
                    game,
                    Vec2::new(pos.x + 12.0 * facing, pos.y + 4.0),
                    velocity,
                );
            }
            let id = game.spawn(
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
                let id = game.spawn(
                    if web { "oWebBall" } else { "oBullet" },
                    Vec2::new(pos.x + muzzle_x, pos.y - 2.0),
                );
                if web {
                    let mut projectile = game.world.entity(id).unwrap().clone();
                    initialize_web_ball(game, &mut projectile);
                    *game.world.entity_mut(id).unwrap() = projectile;
                }
                let speed = (6.0 + f64::from(game.rng.range(3))) * facing + game.player.vel.x;
                let y_speed = if shotgun {
                    game.rng.unit() - game.rng.unit()
                } else {
                    0.0
                };
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
            let id = game.spawn(flash, Vec2::new(pos.x + muzzle_x, pos.y + 1.0));
            init_entity(game.world.entity_mut(id).unwrap());
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
    if game.player.bombs <= 0 || super::player_dead(game) || game.player.is_stunned() {
        return true;
    }
    // Classic's bomb-arrow branch requires a bomb, but does not consume one.
    // The decrement belongs to the separate dedicated bomb-throw branch.
    game.player.data.insert("bow_bomb_armed".into(), 1.0);
    game.player.data.insert("bow_bomb_fuse".into(), 80.0);
    game.player.alarms[11] = 1;
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

/// Player Step1795–1797 calls scrFireBow without a bowArmed guard after hurt.
/// Its live held reference and source type remain available until inventory release.
pub(crate) fn fire_bow_on_late_hurt(game: &mut Game) {
    if game
        .player
        .held
        .and_then(|id| game.world.entity(id))
        .is_some_and(|item| super::explosives::source_type(item) == "Bow")
    {
        fire_bow(game);
    }
}

/// WHOA uses the held receiver's source type and GameMaker's boolean threshold.
/// Other drop paths retain their existing guards independently.
pub(crate) fn fire_bow_on_balance(game: &mut Game) {
    if game.player.data.get("bow_armed").copied().unwrap_or(0.0) >= 0.5
        && game
            .player
            .held
            .and_then(|id| game.world.entity(id))
            .is_some_and(|item| super::explosives::source_type(item) == "Bow")
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
    if matches!(game.player.sprite.as_str(), "sDuckLeft" | "sDamselDuckL") {
        pos.y += 4.0;
    }
    let speed = game.player.vel.x + facing * (1.0 + strength);
    let id = game.spawn("oArrow", pos);
    let e = game.world.entity_mut(id).unwrap();
    init_entity(e);
    owned(e, 1.0);
    e.data.insert("safe".into(), 1.0);
    e.alarms[2] = 10;
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
        e.alarms[1] = bomb_fuse as i32;
        game.player.data.insert("bow_bomb_armed".into(), 0.0);
        game.player.data.remove("bow_bomb_fuse");
    }
    if let Some(ammo) = game.data.get_mut("arrows") {
        // scrFireBow decrements even a zero resource balance on late hurt.
        *ammo -= 1.0;
    }
    game.player.data.insert("weapon_cooldown".into(), 10.0);
    game.player.data.insert("bow_armed".into(), 0.0);
    game.player.data.insert("bow_strength".into(), 0.0);
    game.sound("sndArrowTrap", pos);
    game.stop_sound("sndBowPull");
}

/// oArrow and oFlare explicitly inherit oItem Step before their own code.
/// Keep movement on the room's integer moveTo clock and preserve the original
/// order of wall bounce, ground friction, arrow embedding and ceiling bounce.
fn tick_item_body(game: &Game, e: &mut Entity) {
    if e.held || !super::item_body_active(game, e) {
        return;
    }
    let contact = |e: &Entity, x, y| super::body_contact(&game.world, e, x, y);
    if game.world.solid_at_except(e.pos.x, e.pos.y, e.id) {
        let (left, right, bottom, top) = (
            contact(e, -1, 0),
            contact(e, 1, 0),
            contact(e, 0, 1),
            contact(e, 0, -1),
        );
        // The inherited embedded branch retains its pre-escape probes.
        for (name, value) in [
            ("colLeft", left),
            ("colRight", right),
            ("colBot", bottom),
            ("colTop", top),
        ] {
            e.data.insert(name.into(), f64::from(value));
        }
        if top && !bottom {
            e.pos.y += 1.0;
        } else if left && !right {
            e.pos.x += 1.0;
        } else if right && !left {
            e.pos.x -= 1.0;
        } else {
            e.vel = Vec2::default();
        }
        return;
    }
    let dx = super::displacement(e.vel.x, game.engine_time());
    let dy = super::displacement(e.vel.y, game.engine_time());
    for _ in 0..dx.unsigned_abs() {
        if super::body_horizontal_move_contact(&game.world, e, dx.signum() as i8) {
            break;
        }
        e.pos.x += f64::from(dx.signum());
    }
    for _ in 0..dy.unsigned_abs() {
        if contact(e, 0, dy.signum() as i8) {
            break;
        }
        e.pos.y += f64::from(dy.signum());
    }
    let (left, right, bottom, top) = (
        contact(e, -1, 0),
        contact(e, 1, 0),
        contact(e, 0, 1),
        contact(e, 0, -1),
    );
    // oItem saves this snapshot immediately after moveTo, before responses.
    for (name, value) in [
        ("colLeft", left),
        ("colRight", right),
        ("colBot", bottom),
        ("colTop", top),
    ] {
        e.data.insert(name.into(), f64::from(value));
    }
    if !left && !right {
        e.data.insert("stuck".into(), 0.0);
    }
    if !bottom && e.value("stuck") == 0.0 {
        e.vel.y += e.value("gravity");
    }
    e.vel.y = e.vel.y.min(8.0);
    if left || right {
        e.vel.x *= -0.5;
        e.data.insert("gravity".into(), 0.6);
    }
    if bottom {
        e.data.insert("gravity".into(), 0.6);
        e.vel.y = if e.vel.y > 1.0 {
            -e.vel.y * e.value("bounceFactor")
        } else {
            0.0
        };
        e.vel.x = if e.vel.x.abs() < 0.1 {
            0.0
        } else {
            e.vel.x * e.value("frictionFactor")
        };
        if e.vel.y.abs() < 1.0 {
            e.pos.y -= 1.0;
            if !contact(e, 0, 1) {
                e.pos.y += 1.0;
            }
            e.vel.y = 0.0;
        }
    }
    if e.object == "oArrow" && e.vel.x.abs() > 6.0 {
        if left {
            e.pos.x -= 2.0;
            e.vel = Vec2::default();
        } else if right {
            e.pos.x += 2.0;
            e.vel = Vec2::default();
        }
        e.data.insert("stuck".into(), 1.0);
    } else if left && e.value("stuck") == 0.0 {
        if !right {
            e.pos.x += 1.0;
        }
    } else if right && e.value("stuck") == 0.0 {
        e.pos.x -= 1.0;
    }
    if contact(e, 0, -1) {
        if e.vel.y < 0.0 {
            e.vel.y *= -0.8;
        } else {
            e.pos.y += 1.0;
        }
        e.data.insert("gravity".into(), 0.6);
    }
    if super::lava_at(game, Rect::new(e.pos.x - 3.0, e.pos.y - 3.0, 7.0, 7.0)) {
        e.data.insert("gravity".into(), 0.0);
        e.vel = Vec2::default();
        e.pos.y += 0.05;
    } else {
        e.data.insert("gravity".into(), 0.6);
    }
    if super::lava_at(game, Rect::new(e.pos.x, e.pos.y - 5.0, 0.001, 0.001)) {
        e.alive = false;
    }
}

/// oArrow/oFishBone Step's literal image_angle quadrants. GML angles
/// rotate counterclockwise; renderer/collision rotations use screen radians.
pub(super) fn item_rotation(velocity: Vec2, stuck: bool, previous: f64) -> f64 {
    let Vec2 { x, y } = velocity;
    let angle = if x > 0.0 && y < 0.0 {
        (-y / x).atan()
    } else if x < 0.0 && y < 0.0 {
        std::f64::consts::PI - (-y / -x).atan()
    } else if x > 0.0 && y > 0.0 {
        (y / x).atan()
    } else if x < 0.0 && y > 0.0 {
        std::f64::consts::PI + (y / -x).atan()
    } else if x < 0.0 {
        std::f64::consts::PI
    } else if !stuck {
        0.0
    } else {
        return previous;
    };
    -angle
}

fn hit_player(game: &mut Game, projectile: &mut Entity, damage: i32) {
    if game.player.is_exiting()
        || !matches!(projectile.object.as_str(), "oBullet" | "oArrow") && super::player_dead(game)
    {
        return;
    }
    // Classic projectile damage bypasses contact-damage invulnerability.
    let damaged = game.player.health > 0;
    if damaged {
        game.player.health -= damage;
    }
    if damaged
        && projectile.object == "oArrow"
        && game.player.health <= 0
        && crate::statistics::is_real_level(game)
    {
        game.lifetime_stats
            .misc_death(crate::statistics::MiscDeath::ArrowTrap);
    }
    if damaged
        && projectile.object == "oBullet"
        && game.player.health <= 0
        && crate::statistics::is_real_level(game)
    {
        game.lifetime_stats
            .enemy_death(crate::statistics::EnemyDeath::Shopkeeper);
    }
    game.player.vel = Vec2::new(projectile.vel.x, -4.0);
    if projectile.object == "oArrow" {
        // Player Step: script Blood, synchronous inherited Item Destroy, then
        // hurt sound and stun. Arrow has no additional dead/stunned guard.
        crate::player::script_player_blood(game, 3);
        if projectile.held {
            game.player.held = None;
        }
        projectile.alive = false;
        if damaged {
            game.events.push(GameEvent::Damage { amount: damage });
        }
        game.sound("sndHurt", game.player.pos);
        game.player.stun = 20;
        return;
    }
    if projectile.object == "oBullet" {
        // Character collision creates Blood directly at the receiver's origin,
        // even on a corpse. It does not use scrCreateBlood's bloodless/16 cap.
        game.spawn("oBlood", game.player.pos);
    }
    game.player.stun = 20;
    if damaged {
        game.events.push(GameEvent::Damage { amount: damage });
    }
    game.sound("sndHurt", game.player.pos);
    projectile.alive = false;
    // Bullet's automatic Collision has no held-item release in source.
}

/// Source instance_nearest walks the current active object-family array.
/// World storage retains lifetime order even after deactivation/reactivation.
fn nearest_active_projectile(game: &mut Game, object: &str) -> Option<Entity> {
    crate::dispatcher::register_instances(game);
    let origin = game.player.pos;
    crate::dispatcher::descendant_instances(game, object)
        .into_iter()
        .filter_map(|instance| match instance {
            crate::dispatcher::Instance::Entity(id) => game.world.entity(id),
            _ => None,
        })
        .filter(|entity| entity.engine_active)
        .min_by(|a, b| {
            let distance_squared = |entity: &Entity| {
                let dx = entity.pos.x - origin.x;
                let dy = entity.pos.y - origin.y;
                dx * dx + dy * dy
            };
            distance_squared(a).total_cmp(&distance_squared(b))
        })
        .cloned()
}

/// oPlayer1 Step checks rocks independently from Arrow collisions. Rocks
/// survive impact, and the Mitt catches an unsafe fast rock only with empty
/// hands. Contact-damage invulnerability does not guard this source branch.
fn rock_player_contact(game: &mut Game) {
    if game.player.is_exiting() || super::player_dead(game) || game.player.is_stunned() {
        return;
    }
    let first = Vec2::new(game.player.pos.x - 8.0, game.player.pos.y - 8.0);
    let second = Vec2::new(game.player.pos.x + 8.0, game.player.pos.y + 8.0);
    if !game.world.entities.iter().any(|e| {
        e.alive
            && e.engine_active
            && e.object == "oRock"
            && e.collision_mask().intersects_rectangle_bbox(first, second)
    }) {
        return;
    }
    // The original first tests collision_rectangle, then instance_nearest.
    let Some(rock) = nearest_active_projectile(game, "oRock") else {
        return;
    };
    if rock.vel.x.abs() <= 4.0 || rock.value("safe") != 0.0 {
        return;
    }
    let (id, vx) = (rock.id, rock.vel.x);
    if game.player.has("Mitt") && game.player.held.is_none() {
        game.player.held = Some(id);
        game.player.pickup_item_type = "Rock".into();
        game.world.entity_mut(id).unwrap().held = true;
    } else {
        let damaged = game.player.health > 0;
        if damaged {
            game.player.health -= 2;
        }
        if damaged && game.player.health <= 0 && crate::statistics::is_real_level(game) {
            game.lifetime_stats
                .misc_death(crate::statistics::MiscDeath::Rock);
        }
        game.player.vel = Vec2::new(vx, -4.0);
        if damaged {
            game.events.push(GameEvent::Damage { amount: 2 });
        }
        // oPlayer1 Step1630: synchronous script Blood precedes sound/stun.
        crate::player::script_player_blood(game, 3);
        game.sound("sndHurt", game.player.pos);
        game.player.stun = 20;
    }
}

fn mattock_decorations(game: &mut Game, origin: Vec2) {
    for y in [origin.y - 16.0, origin.y + 16.0] {
        game.dispatcher
            .remove_decoration_at(&mut game.world.entities, 3.0, Vec2::new(origin.x, y));
    }
}

fn mattock_sweeps(game: &mut Game) {
    use crate::dispatcher::Instance;
    crate::dispatcher::register_instances(game);
    for instance in crate::dispatcher::descendant_instances(game, "oTreasure") {
        if let Instance::Entity(id) = instance {
            if let Some(treasure) = game.world.entity_mut(id) {
                treasure.state = 1;
            }
        }
    }
    for instance in crate::dispatcher::descendant_instances(game, "oSpikes") {
        let point = instance.position(game);
        let point = Vec2::new(point.x, point.y + 16.0);
        let supported = crate::dispatcher::descendant_instances(game, "oSolid")
            .into_iter()
            .any(|solid| match solid {
                Instance::Entity(id) => game
                    .world
                    .entity(id)
                    .is_some_and(|body| body.mask_contains(point, CollisionPrecision::BoundingBox)),
                Instance::Terrain(index) => game.world.tiles.get(index).is_some_and(|tile| {
                    !tile.object.is_empty()
                        && game
                            .world
                            .tile_collision_mask(
                                (index % game.world.width) as i32,
                                (index / game.world.width) as i32,
                            )
                            .contains(point, CollisionPrecision::BoundingBox)
                }),
                Instance::Player => false,
            });
        if !supported {
            match instance {
                Instance::Entity(id) => {
                    if let Some(spikes) = game.world.entity_mut(id) {
                        spikes.alive = false;
                    }
                }
                Instance::Terrain(index) => {
                    game.world
                        .set_tile(index % game.world.width, index / game.world.width, "");
                }
                Instance::Player => {}
            }
        }
    }
}

fn mattock_break(game: &mut Game, origin: Vec2) {
    game.player.held = None;
    game.player.pickup_item_type.clear();
    game.flags.retain(|flag| !flag.starts_with("carry:"));
    let id = game.spawn("oMattockHead", origin);
    game.world.entity_mut(id).unwrap().vel.y = -2.0;
    game.sound("sndMattockBreak", origin);
    for instance in crate::dispatcher::descendant_instances(game, "oMattock") {
        let crate::dispatcher::Instance::Entity(id) = instance else {
            continue;
        };
        if let Some(mattock) = game.world.entity_mut(id) {
            let visible = mattock.data.get("visible").map_or_else(
                || crate::catalog::object(&mattock.object).is_some_and(|object| object.visible),
                |value| *value != 0.0,
            );
            if !visible || mattock.value("hidden") != 0.0 {
                mattock.alive = false;
            }
        }
    }
}

fn mattock_dig(game: &mut Game, e: &Entity) {
    if matches!(game.world.name.as_str(), "rTitle" | "rHighscores") {
        return;
    }
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
                && body.sprite_bounds().contains(Vec2::new(e.pos.x, point_y))
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
                mattock_decorations(game, Vec2::new(f64::from(x) * 16.0, f64::from(y) * 16.0));
                super::explosives::destroy_terrain(game, x as usize, y as usize);
            }
            DigTarget::Dynamic(id) => {
                let Some(body) = game.world.entity(id) else {
                    return;
                };
                if body.value("invincible") != 0.0 {
                    return;
                }
                let origin = body.pos;
                mattock_decorations(game, origin);
                super::explosives::destroy_solid_instance(game, id);
            }
        }
        mattock_sweeps(game);
        // Source evaluates rand before checking the character flag.
        let breaks = game.rng.chance(1, 20) && !game.flags.contains("character_tunnelman");
        if breaks {
            mattock_break(game, e.pos);
        } else {
            game.sound("sndCrunch", e.pos);
        }
    }
}

/// Player Step's held bomb-arrow counter precedes item activation/use.
pub fn pre_tick(game: &mut Game) {
    if game
        .player
        .data
        .get("bow_bomb_armed")
        .copied()
        .unwrap_or(0.0)
        == 0.0
    {
        return;
    }
    let fuse = game
        .player
        .data
        .entry("bow_bomb_fuse".into())
        .or_insert(80.0);
    if *fuse > 0.0 {
        *fuse -= 1.0;
    } else {
        game.spawn("oExplosion", game.player.pos);
        if !game.flags.contains("low_graphics") {
            crate::world_events::create_flames(game, game.player.pos, 3);
        }
        game.player.data.insert("bow_bomb_armed".into(), 0.0);
        game.player.data.remove("bow_bomb_fuse");
    }
    // oPlayer1 Step57–60 queries even on the detonation tick. The script
    // assigns Player tx/ty before the nearest-keeper anger callback.
    if crate::economy::player_in_shop(game, game.player.pos) {
        crate::economy::anger(game, game.player.pos, "TERRORIST!");
    }
}

/// Source oPlayer1 Step decrements FIRING before controller and item actions.
pub(crate) fn firing_tick(game: &mut Game) {
    if let Some(firing) = game.player.data.get_mut("weapon_cooldown") {
        if *firing > 0.0 {
            *firing -= 1.0;
        }
    }
}

/// Isolated legacy driver owns its one early decrement; live dispatch calls
/// firing_tick separately and only calls the late release helper below.
pub fn player_step(game: &mut Game) {
    firing_tick(game);
    player_step_with_release(game, false);
}

/// Source release is independent from aggregate held state in mixed-device play.
/// The compatibility driver above retains held-only subsystem fixtures.
pub(crate) fn player_step_with_release(game: &mut Game, attack_released: bool) {
    for item in game.world.entities.iter_mut().filter(|e| e.alive && e.held) {
        let base = match item.object.as_str() {
            "oPistol" => "Pistol",
            "oShotgun" => "Shotgun",
            "oSceptre" => "Sceptre",
            "oMachete" => "Machete",
            "oMattock" => "Mattock",
            "oWebCannon" => "WebCannon",
            _ => continue,
        };
        let sprite = if item.object == "oWebCannon" {
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
        item.set_sprite(&sprite);
    }
    if game.player.data.get("bow_armed").copied().unwrap_or(0.0) != 0.0 {
        let holding_bow = game
            .player
            .held
            .and_then(|id| game.world.entity(id))
            .is_some_and(|e| e.object == "oBow");
        let bow_strength = game.player.data.get("bow_strength").copied().unwrap_or(0.0);
        if !holding_bow {
            // oPlayer1 Step1505: cancellation sound belongs to the charging gate.
            if game.player.held.is_none()
                && game.player.data.get("attack_held").copied().unwrap_or(0.0) != 0.0
                && bow_strength < 12.0
            {
                game.stop_sound("sndBowPull");
            }
            game.player.data.insert("bow_armed".into(), 0.0);
            game.player.data.insert("bow_strength".into(), 0.0);
        } else {
            let held = game.player.data.get("attack_held").copied().unwrap_or(0.0) != 0.0;
            if held {
                let strength = game.player.data.entry("bow_strength".into()).or_default();
                *strength = (*strength + 0.2).min(12.0);
            }
            // oPlayer1 charges first, then evaluates checkAttackReleased().
            // Held-only input producers retain their existing falling-level path.
            if attack_released || !held {
                fire_bow(game);
            }
        }
    }
}

/// oBow's own Step follows inherited oItem Step and Player charge/release.
/// The child frame refresh is unconditional, even outside oItem's 16px gate.
pub(super) fn bow_step(game: &Game, bow: &mut Entity) {
    bow.animation = if bow.held {
        bow.set_sprite(if game.player.facing < 0 {
            "sBowLeft"
        } else {
            "sBowRight"
        });
        let strength = game.player.data.get("bow_strength").copied().unwrap_or(0.0);
        if strength >= 10.0 {
            3.0
        } else if strength > 6.0 {
            2.0
        } else if strength > 2.0 {
            1.0
        } else {
            0.0
        }
    } else {
        0.0
    };
}

/// oPlayer pre/forward weapon queries precede action input in the source.
pub fn attack_step(game: &mut Game) {
    // Source queries the live hand after the early stunned release/restoration,
    // not the derived phase owner of a previously held weapon.
    let current = game.player.held.and_then(|id| {
        game.world
            .entity(id)
            .filter(|item| item.alive && item.engine_active)
            .and_then(|item| match super::explosives::source_type(item).as_str() {
                "Machete" => Some((id, false)),
                "Mattock" => Some((id, true)),
                _ => None,
            })
    });
    let attack_art = matches!(
        game.player.sprite.as_str(),
        "sAttackLeft" | "sDamselAttackL" | "sTunnelAttackL"
    );
    if !attack_art || current.is_none() {
        if let Some(id) = game.player.data.get("weapon_melee_id").copied() {
            if let Some(held) = game.world.entity_mut(id as u64) {
                held.data.insert("hidden".into(), 0.0);
            }
        }
        // Keep whipping: an empty restored hand can still take the source's
        // ordinary/TunnelMan query before its later hurt cancellation.
        game.player.data.remove("weapon_melee_age");
        return;
    }
    let (weapon_id, mattock) = current.unwrap();
    let before = game.player.animation;
    // Source tests the image index before central animation advance.
    if before > 4.0
        && !game
            .world
            .entities
            .iter()
            .any(|e| e.alive && e.engine_active && crate::catalog::is_a(&e.object, "oWhip"))
    {
        let object = if mattock { "oMattockHit" } else { "oSlash" };
        let facing = game.player.facing;
        let id = game.spawn(
            object,
            Vec2::new(
                game.player.pos.x + 16.0 * f64::from(facing),
                game.player.pos.y,
            ),
        );
        let e = game.world.entity_mut(id).unwrap();
        owned(e, 2.0);
        e.facing = facing;
        init_entity(e);
        e.data.insert("puncture".into(), 1.0);
        e.data.insert("weapon_id".into(), weapon_id as f64);
        e.sprite = if mattock {
            format!("sMattockHit{}", if facing < 0 { "L" } else { "R" })
        } else {
            format!("sSlash{}", if facing < 0 { "Left" } else { "Right" })
        };
        game.sound("sndWhip", game.player.pos);
    }
    // The source explicitly names oMachetePre for both held weapons.
    // oMattockPre directly inherits oWhipPre and does not satisfy that guard.
    if before < 2.0
        && !game
            .world
            .entities
            .iter()
            .any(|e| e.alive && e.engine_active && crate::catalog::is_a(&e.object, "oMachetePre"))
    {
        spawn_pre(game, mattock);
    }
    if let Some(age) = game.player.data.get("weapon_melee_age").copied() {
        game.player
            .data
            .insert("weapon_melee_age".into(), age + 1.0);
        game.player
            .data
            .insert("weapon_melee_id".into(), weapon_id as f64);
        game.player.data.insert(
            "weapon_melee_mattock".into(),
            if mattock { 1.0 } else { 0.0 },
        );
    }
}

/// Source oPlayer hurt queries run before later resource-ordered projectile Step.
pub fn player_hurt_queries(game: &mut Game) {
    let first = Vec2::new(game.player.pos.x - 8.0, game.player.pos.y - 8.0);
    let second = Vec2::new(game.player.pos.x + 8.0, game.player.pos.y + 8.0);
    if game.world.entities.iter().any(|e| {
        e.alive
            && e.engine_active
            && e.object == "oArrow"
            && e.collision_mask().intersects_rectangle_bbox(first, second)
    }) {
        let nearest = nearest_active_projectile(game, "oArrow");
        if let Some(mut arrow) = nearest {
            if arrow.vel.x.abs() > 3.0 && arrow.value("safe") == 0.0 {
                hit_player(game, &mut arrow, 2);
            } else if game
                .player
                .held
                .and_then(|id| game.world.entity(id))
                .is_some_and(|e| e.object == "oBow")
                && !super::player_dead(game)
                && !game.player.is_stunned()
                && arrow.value("stuck") == 0.0
                && arrow.vel.x.abs() < 1.0
                && arrow.vel.y.abs() < 1.0
            {
                arrow.alive = false;
                *game.data.entry("arrows".into()).or_default() += 1.0;
                game.sound("sndPickup", arrow.pos);
            }
            if let Some(original) = game.world.entity_mut(arrow.id) {
                *original = arrow;
            }
        }
    }
    rock_player_contact(game);
}

pub fn handles(object: &str) -> bool {
    matches!(
        object,
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
}

pub fn step_instance(game: &mut Game, id: u64) {
    let Some(mut e) = game.world.entity(id).cloned() else {
        return;
    };
    if !handles(&e.object) {
        return;
    }
    init_entity(&mut e);
    match e.object.as_str() {
        "oBullet" => {
            e.pos.x += e.vel.x;
            e.pos.y += e.vel.y;
        }
        "oArrow" => {
            if !inherited_item_step(game, &mut e) {
                return;
            }
            let angle = item_rotation(e.vel, e.value("stuck") != 0.0, e.value("rotation"));
            e.data.insert("rotation".into(), angle);
        }
        "oFlare" => {
            // The child event continues after inherited lava destruction.
            inherited_item_step(game, &mut e);
            e.data.insert(
                "dist_to_player".into(),
                crate::world_events::mask_distance(
                    e.collision_mask().point_bounds(),
                    game.player.collision_mask().point_bounds(),
                ),
            );
            if game
                .world
                .tiles_point_precision(e.pos, "oWater", CollisionPrecision::Precise)
                || game.world.entities.iter().any(|water| {
                    water.alive
                        && crate::catalog::is_a(&water.object, "oWater")
                        && water.mask_contains(e.pos, CollisionPrecision::Precise)
                })
            {
                e.alive = false;
                if game.player.held == Some(id) {
                    game.player.held = None;
                    game.player.pickup_item_type.clear();
                }
                game.spawn("oSplash", e.pos);
                game.sound("sndSplash", e.pos);
            }
            e.data.insert("light_radius".into(), 96.0);
        }
        "oPsychicWaveP" => {
            if e.timer > 0 {
                e.timer -= 1;
                e.pos.x += e.vel.x;
                e.data.insert(
                    "psychic_direction".into(),
                    if e.vel.x > 0.0 {
                        0.0
                    } else {
                        std::f64::consts::PI
                    },
                );
            } else {
                if let Some(target) = game
                    .world
                    .entities
                    .iter()
                    .filter(|target| target.alive && crate::catalog::is_a(&target.object, "oEnemy"))
                    .min_by(|a, b| e.pos.distance(a.pos).total_cmp(&e.pos.distance(b.pos)))
                {
                    e.data.insert(
                        "psychic_direction".into(),
                        (e.pos.y - target.pos.y - 8.0).atan2(target.pos.x + 8.0 - e.pos.x),
                    );
                }
                let direction = e.value("psychic_direction");
                e.pos.x += 2.0 * direction.cos();
                e.pos.y -= 2.0 * direction.sin();
            }
        }
        "oWebBall" => {
            if e.value("web_create_initialized") == 0.0 {
                let life = e.data.get("life").copied();
                let velocity = (e.vel != Vec2::default() || life.is_some()).then_some(e.vel);
                initialize_web_ball(game, &mut e);
                if let Some(life) = life {
                    e.data.insert("life".into(), life);
                }
                if let Some(velocity) = velocity {
                    e.vel = velocity;
                }
            }
            e.pos.x += e.vel.x;
            e.pos.y += e.vel.y;
            if e.vel.y < 6.0 {
                e.vel.y += 0.2;
            }
            if e.value("life") > 0.0 {
                let life = e.value("life") - 1.0;
                e.data.insert("life".into(), life);
            } else {
                e.set_sprite("sWebCreate");
                e.state = 1;
            }
        }
        "oMachetePre" | "oMattockPre" | "oSlash" | "oMattockHit" => {
            let pre = e.object.ends_with("Pre");
            e.pos = Vec2::new(
                game.player.pos.x + 16.0 * f64::from(e.facing) * if pre { -1.0 } else { 1.0 },
                game.player.pos.y,
            );
            selected_sprite_bounds(&mut e);
        }
        _ => {}
    }
    let continue_retired = e.object == "oFlare";
    if let Some(original) = game
        .world
        .entities
        .iter_mut()
        .find(|original| original.id == id && (original.alive || continue_retired))
    {
        *original = e;
    }
}

/// Publish inherited movement before its oItem rectangle damage queries. The
/// child Arrow/Flare event continues only after these source callbacks finish.
fn inherited_item_step(game: &mut Game, e: &mut Entity) -> bool {
    let impacts = super::item_body_active(game, e);
    tick_item_body(game, e);
    if let Some(slot) = game.world.entities.iter_mut().find(|slot| slot.id == e.id) {
        *slot = e.clone();
    }
    if impacts && e.alive {
        crate::enemies::item_step_impacts(game, e.id);
    }
    if let Some(updated) = game.world.entities.iter().find(|slot| slot.id == e.id) {
        *e = updated.clone();
    }
    e.alive
}

/// These objects move through explicit Step code, not GM's speed variable.
pub fn builtin_motion(_game: &mut Game, _id: u64) {}

pub fn collision(game: &mut Game, id: u64, other: crate::dispatcher::Instance) {
    let Some(mut e) = game.world.entity(id).cloned() else {
        return;
    };
    if !e.alive {
        return;
    }
    let object = other.object(game).to_owned();
    match e.object.as_str() {
        "oBullet" | "oPsychicWaveP"
            if crate::catalog::is_a(&object, "oEnemy") || object == "oDamsel" =>
        {
            if let crate::dispatcher::Instance::Entity(target) = other {
                crate::enemies::projectile_collision(game, id, target);
            }
            return;
        }
        "oBullet" if other == crate::dispatcher::Instance::Player => hit_player(game, &mut e, 4),
        "oBullet" if crate::catalog::is_a(&object, "oSolid") => {
            game.spawn("oSmokePuff", e.pos);
            game.sound("sndHit", e.pos);
            e.alive = false;
        }
        "oWebBall"
            if crate::catalog::is_a(&object, "oSolid")
                || crate::catalog::is_a(&object, "oItem")
                || crate::catalog::is_a(&object, "oEnemy") && object != "oGiantSpider"
                || crate::catalog::is_a(&object, "oWater")
                || object == "oWeb" =>
        {
            e.set_sprite("sWebCreate");
            e.vel = Vec2::default();
            e.state = 1;
        }
        _ => {}
    }
    if let Some(original) = game.world.entity_mut(id) {
        *original = e;
    }
}

/// Only these weapon objects declare source Outside Room destruction events.
pub fn outside_room(game: &mut Game, id: u64) {
    if let Some(e) = game.world.entity_mut(id) {
        if matches!(e.object.as_str(), "oWebBall" | "oPsychicWaveP") {
            e.alive = false;
        }
    }
}

pub fn animation_end(game: &mut Game, id: u64) {
    let Some(e) = game.world.entity(id).cloned() else {
        return;
    };
    match e.object.as_str() {
        "oWebBall" if e.sprite == "sWebCreate" => {
            let web = game.spawn("oWeb", Vec2::new(e.pos.x - 8.0, e.pos.y - 8.0));
            game.world
                .entity_mut(web)
                .unwrap()
                .data
                .insert("dying".into(), 1.0);
        }
        "oMattockHit" => mattock_dig(game, &e),
        "oSlash" | "oPsychicWaveP" | "oShotgunBlastLeft" | "oShotgunBlastRight" => {}
        _ => return,
    }
    if let Some(e) = game.world.entity_mut(id) {
        e.alive = false;
    }
}

pub fn alarm(game: &mut Game, id: u64, number: u8) {
    let Some(e) = game.world.entity(id).cloned() else {
        return;
    };
    match (e.object.as_str(), number) {
        ("oMachetePre" | "oMattockPre", 0) => game.world.entity_mut(id).unwrap().alive = false,
        ("oBullet", 0) | ("oArrow", 2) => {
            game.world
                .entity_mut(id)
                .unwrap()
                .data
                .insert("safe".into(), 0.0);
        }
        ("oArrow", 1) => {
            game.spawn("oExplosion", e.pos);
            if !game.flags.contains("low_graphics") {
                crate::world_events::create_flames(game, e.pos, 3);
            }
            if game.player.held == Some(id) {
                game.player.held = None;
            }
            game.world.entity_mut(id).unwrap().alive = false;
        }
        ("oFlare", 0) => {
            let pos = Vec2::new(
                e.pos.x + f64::from(game.rng.range(4)) - f64::from(game.rng.range(4)),
                e.pos.y + f64::from(game.rng.range(4)) - f64::from(game.rng.range(4)),
            );
            game.spawn("oFlareSpark", pos);
            game.world.entity_mut(id).unwrap().alarms[0] = 2;
        }
        _ => {}
    }
}

/// Isolated subsystem fixture driver; real gameplay uses the central phases.
#[cfg(test)]
pub fn tick(game: &mut Game) {
    for e in game
        .world
        .entities
        .iter_mut()
        .filter(|e| e.alive && handles(&e.object))
    {
        init_entity(e);
    }
    for number in 0..12 {
        let mut fired = Vec::new();
        for e in game
            .world
            .entities
            .iter_mut()
            .filter(|e| e.alive && handles(&e.object))
        {
            if e.alarms[number] > 0 {
                e.alarms[number] -= 1;
                if e.alarms[number] == 0 {
                    fired.push(e.id);
                }
            }
        }
        for id in fired {
            alarm(game, id, number as u8);
        }
    }
    if game.player.attack_timer == 0 && game.player.data.contains_key("weapon_melee_age") {
        game.player.set_sprite("sStandLeft");
    }
    attack_step(game);
    player_step(game);
    let ids: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && handles(&e.object))
        .map(|e| e.id)
        .collect();
    for &id in &ids {
        step_instance(game, id);
    }
    player_hurt_queries(game);
    for &id in &ids {
        let Some(e) = game.world.entity(id).cloned() else {
            continue;
        };
        if !e.alive {
            continue;
        }
        if game.player.sprite_bounds().intersects(e.sprite_bounds()) {
            collision(game, id, crate::dispatcher::Instance::Player);
        }
        for target in game.world.entities.clone() {
            if target.alive && target.id != id && target.event_intersects(&e) {
                collision(game, id, crate::dispatcher::Instance::Entity(target.id));
            }
        }
        for index in 0..game.world.tiles.len() {
            let x = index % game.world.width;
            let y = index / game.world.width;
            if game.world.tiles[index].is_solid()
                && game
                    .world
                    .tile_bounds(x as i32, y as i32)
                    .intersects(e.sprite_bounds())
            {
                collision(game, id, crate::dispatcher::Instance::Terrain(index));
            }
        }
    }
    for id in ids {
        let Some(e) = game.world.entity_mut(id) else {
            continue;
        };
        if !e.alive {
            continue;
        }
        let end = crate::animation::advance(
            &mut e.animation,
            e.data.get("animation_speed").copied().unwrap_or(1.0),
            crate::catalog::sprite_frame_count(&e.sprite),
        );
        if end {
            animation_end(game, id);
        }
    }
    let speed = game.player.image_speed();
    if crate::animation::advance(
        &mut game.player.animation,
        speed,
        crate::catalog::sprite_frame_count(&game.player.sprite),
    ) {
        crate::player::animation_end(game);
    }
    if game.player.attack_timer > 0 {
        game.player.set_image_speed(1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn spawn_web_ball(game: &mut Game, pos: Vec2) -> u64 {
        let id = game.world.spawn("oWebBall", pos);
        let mut ball = game.world.entity(id).unwrap().clone();
        initialize_web_ball(game, &mut ball);
        *game.world.entity_mut(id).unwrap() = ball;
        id
    }
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
    fn fast_bullet_uses_source_endpoint_collision_instead_of_sweeping() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.world.set_tile(5, 5, "oBlock");
        let id = game.world.spawn("oBullet", Vec2::new(70.0, 88.0));
        game.world.entity_mut(id).unwrap().vel.x = 30.0;
        tick(&mut game);
        let bullet = game.world.entity(id).unwrap();
        assert_eq!(bullet.pos.x, 100.0);
        assert!(
            bullet.alive,
            "the source has no sweep between explicit Step endpoints"
        );
    }
    #[test]
    fn flare_is_extinguished_in_water_and_releases_hand() {
        let (mut game, id) = armed("oFlare");
        // oWater is sprite-less; real source pools instantiate its child.
        game.world.set_tile(5, 5, "oWaterSwim");
        tick(&mut game);
        assert!(game.world.entity(id).is_none());
        assert_eq!(game.player.held, None);
    }
    fn natural_flare_crate_beside_pool(dynamic_water: bool, initial_y: f64) -> (Game, Vec<u64>) {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.world = World::new(20, 15, Area::Mines, 1);
        game.player.pos = Vec2::new(88.0, initial_y);
        for x in 0..20 {
            game.world.set_tile(x, 10, "oBrick");
            if x < 6 || x > 10 {
                game.world.set_tile(x, 6, "oBrick");
            }
        }
        for y in 7..10 {
            game.world.set_tile(5, y, "oBrick");
            game.world.set_tile(11, y, "oBrick");
            for x in 6..11 {
                if dynamic_water {
                    game.world
                        .spawn("oWaterSwim", Vec2::new(x as f64 * 16.0, y as f64 * 16.0));
                } else {
                    game.world.set_tile(x, y, "oWaterSwim");
                }
            }
        }
        let crate_id = game.spawn("oFlareCrate", game.player.pos);
        game.step(&InputFrame {
            up: true,
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert!(game.world.entity(crate_id).is_none());
        let flares: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|e| e.object == "oFlare")
            .collect();
        assert_eq!(
            flares.len(),
            3,
            "real Up+Attack opens the source flare crate"
        );
        for flare in &flares {
            assert!(!flare.held);
            assert_eq!(flare.sprite, "sFlare");
            assert_eq!(flare.value("invincible"), 1.0);
            assert_eq!(flare.alarms[0], 1, "created after this frame's alarm pass");
        }
        let ids = flares.iter().map(|e| e.id).collect();
        (game, ids)
    }

    #[test]
    fn actual_flare_crate_launch_into_water_descendant_extinguishes() {
        for dynamic_water in [false, true] {
            // The source allows opening a floating crate while falling.
            // Start above the dry deck so its first positive launch clears
            // the lip before landing friction can stop it.
            let (mut game, ids) = natural_flare_crate_beside_pool(dynamic_water, 64.0);
            let id = ids
                .into_iter()
                .find(|id| game.world.entity(*id).unwrap().vel.x > 0.0)
                .expect("seed1 source crate launches one flare toward the pool");
            let mut extinguished = false;
            let mut cleared_dry_lip = false;
            let mut trajectory = Vec::new();
            for _ in 0..60 {
                game.step(&InputFrame::default());
                if let Some(flare) = game.world.entity(id) {
                    trajectory.push((flare.pos, flare.vel));
                    cleared_dry_lip |= flare.pos.x >= 96.0 && flare.pos.y < 112.0;
                    let immersed = game.world.tiles_point_precision(
                        flare.pos,
                        "oWater",
                        CollisionPrecision::Precise,
                    ) || game.world.entities.iter().any(|water| {
                        water.object == "oWaterSwim"
                            && water.mask_contains(flare.pos, CollisionPrecision::Precise)
                    });
                    assert!(
                        !immersed,
                        "flare survived actual WaterSwim immersion (dynamic={dynamic_water}), trajectory={trajectory:?}"
                    );
                } else {
                    assert!(
                        cleared_dry_lip,
                        "fixture must prove natural clearance before immersion, trajectory={trajectory:?}"
                    );
                    let splashes: Vec<_> = game
                        .world
                        .entities
                        .iter()
                        .filter(|e| e.object == "oSplash" && e.age == 0)
                        .collect();
                    assert_eq!(
                        splashes.len(),
                        1,
                        "one source Flare Step splash, no Flame Destroy smoke"
                    );
                    assert!(game.events.iter().any(|event| matches!(event,
                        GameEvent::Sound { name, pos } if name == "sndSplash" && *pos == splashes[0].pos)));
                    assert!(
                        game.world.tiles_point_precision(
                            splashes[0].pos,
                            "oWater",
                            CollisionPrecision::Precise
                        ) || game
                            .world
                            .entities
                            .iter()
                            .any(|water| water.object == "oWaterSwim"
                                && water
                                    .mask_contains(splashes[0].pos, CollisionPrecision::Precise))
                    );
                    assert_eq!(game.player.held, None);
                    extinguished = true;
                    break;
                }
            }
            assert!(
                extinguished,
                "natural launched flare must extinguish in WaterSwim (dynamic={dynamic_water}), trajectory={trajectory:?}"
            );
        }
    }

    #[test]
    fn actual_pickup_and_carry_flare_into_water_releases_hand() {
        for dynamic_water in [false, true] {
            let (mut game, _) = natural_flare_crate_beside_pool(dynamic_water, 88.0);
            for _ in 0..20 {
                game.step(&InputFrame::default());
            }
            game.step(&InputFrame {
                down: true,
                attack: true,
                attack_pressed: true,
                ..Default::default()
            });
            let id = game
                .player
                .held
                .expect("Down+Attack naturally picks up a settled crate flare");
            let flare = game.world.entity(id).unwrap();
            assert_eq!(flare.object, "oFlare");
            assert!(flare.held);
            let mut extinguished = false;
            for _ in 0..60 {
                game.step(&InputFrame {
                    right: true,
                    ..Default::default()
                });
                if game.world.entity(id).is_none() {
                    assert_eq!(
                        game.player.held, None,
                        "source water branch releases the held flare"
                    );
                    assert!(game.events.iter().any(|event| matches!(event,
                        GameEvent::Sound { name, .. } if name == "sndSplash")));
                    extinguished = true;
                    break;
                }
                assert_eq!(
                    game.player.held,
                    Some(id),
                    "do not substitute a throw/drop for immersion"
                );
            }
            assert!(
                extinguished,
                "naturally carried flare must extinguish (dynamic={dynamic_water})"
            );
        }
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
            hit.animation = (crate::catalog::sprite_frame_count(&hit.sprite) - 1) as f64 + 0.5;
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
        arrow.data.insert("safe".into(), 1.0);
        tick(&mut game);
        assert_eq!(game.player.health, 4);
        assert_eq!(game.world.entity(id).unwrap().value("safe"), 1.0);
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
            0
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
    fn bomb_arrow_requires_but_does_not_consume_bomb_and_preserves_cooked_fuse() {
        let (mut game, id) = armed("oBow");
        game.data.insert("arrows".into(), 6.0);
        game.player.data.insert("attack_held".into(), 1.0);
        use_weapon(&mut game, id, &InputFrame::default());
        assert!(arm_bomb_arrow(&mut game));
        assert!(arm_bomb_arrow(&mut game));
        assert_eq!(game.player.bombs, 4);
        for _ in 0..10 {
            pre_tick(&mut game);
            tick(&mut game);
        }
        assert_eq!(game.player.data["bow_bomb_fuse"], 70.0);
        game.player.data.insert("attack_held".into(), 0.0);
        pre_tick(&mut game);
        tick(&mut game);
        let arrow = game
            .world
            .entities
            .iter()
            .find(|e| e.object == "oArrow")
            .unwrap();
        assert_eq!(arrow.sprite, "sBombArrowRight");
        assert_eq!(arrow.value("bomb_arrow"), 1.0);
        assert_eq!(arrow.alarms[1], 69);
        assert_eq!(game.data["arrows"], 5.0);
        assert_eq!(game.player.data["bow_bomb_armed"], 0.0);
    }
    #[test]
    fn bomb_arrow_explodes_on_fuse_even_when_stuck_in_wall() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        let id = game.world.spawn("oArrow", Vec2::new(88.0, 88.0));
        let arrow = game.world.entity_mut(id).unwrap();
        arrow.data.insert("bomb_arrow".into(), 1.0);
        arrow.alarms[1] = 2;
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
            pre_tick(&mut game);
            tick(&mut game);
        }
        assert!(!game.world.entities.iter().any(|e| e.object == "oExplosion"));
        pre_tick(&mut game);
        tick(&mut game);
        assert!(
            game.world
                .entities
                .iter()
                .any(|e| e.object == "oExplosion" && e.pos == game.player.pos)
        );
        assert_eq!(game.player.data["bow_bomb_armed"], 0.0);
    }

    #[test]
    fn game_step_bomb_arrow_arming_preserves_initial_counter_and_blink_phase() {
        let (mut game, _) = flat_room("oBow");
        game.data.insert("arrows".into(), 6.0);
        game.step(&InputFrame {
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        game.step(&InputFrame {
            attack: true,
            bomb_pressed: true,
            ..Default::default()
        });
        assert_eq!(game.player.bombs, 4);
        assert_eq!(game.player.data["bow_bomb_fuse"], 80.0);
        assert_eq!(
            game.player
                .data
                .get("bow_arrow_toggle")
                .copied()
                .unwrap_or(0.0),
            0.0
        );
        game.step(&InputFrame {
            attack: true,
            ..Default::default()
        });
        assert_eq!(game.player.data["bow_bomb_fuse"], 79.0);
        assert_eq!(game.player.data["bow_arrow_toggle"], 1.0);
        game.step(&InputFrame::default());
        let arrow = game
            .world
            .entities
            .iter()
            .find(|e| e.object == "oArrow")
            .unwrap();
        assert_eq!(arrow.alarms[1], 78);
        assert_eq!(game.player.data["bow_arrow_toggle"], 0.0);
        assert_eq!(game.player.bombs, 4);
    }
    #[test]
    fn bullets_collide_with_dynamic_solids_and_mattock_digs_them() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        let block = game.world.spawn("oPushBlock", Vec2::new(80.0, 80.0));
        game.world.entity_mut(block).unwrap().solid = true;
        let bullet = game.world.spawn("oBullet", Vec2::new(70.0, 88.0));
        game.world.entity_mut(bullet).unwrap().vel.x = 18.0;
        tick(&mut game);
        assert!(game.world.entity(bullet).is_none());
        let hit = Entity::new(999, "oMattockHit", Vec2::new(88.0, 88.0));
        mattock_dig(&mut game, &hit);
        assert!(game.world.entity(block).is_none());
    }
    #[test]
    fn mattock_hit_digs_once_on_its_source_animation_end() {
        let (mut game, _) = flat_room("oMattock");
        game.world.set_tile(6, 9, "oBlock");
        game.step(&InputFrame {
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        for _ in 0..9 {
            game.step(&InputFrame::default());
        }
        assert_eq!(game.world.tile(6, 9).unwrap().object, "oBlock");
        let hit = game
            .world
            .entities
            .iter()
            .find(|e| e.object == "oMattockHit")
            .unwrap();
        assert_eq!(hit.animation, 2.5);
        game.step(&InputFrame::default());
        assert_eq!(game.world.tile(6, 9).unwrap().object, "");
        assert!(
            !game
                .world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oMattockHit")
        );
        game.world.set_tile(6, 9, "oBlock");
        for _ in 0..4 {
            game.step(&InputFrame::default());
        }
        assert_eq!(game.world.tile(6, 9).unwrap().object, "oBlock");
    }

    #[test]
    fn tunnelman_mattock_consumes_durability_rng_without_breaking() {
        let (mut game, id) = armed("oMattock");
        game.flags.insert("character_tunnelman".into());
        game.world.set_tile(6, 5, "oBlock");
        let mut expected = game.rng.clone();
        // oBlock Destroy creates three debris pieces, with four rand(0,8)
        // position draws each, before Mattock Animation End rolls durability.
        for _ in 0..12 {
            expected.range(9);
        }
        expected.chance(1, 20);
        let mut hit = Entity::new(999, "oMattockHit", Vec2::new(104.0, 88.0));
        hit.data.insert("weapon_id".into(), id as f64);
        mattock_dig(&mut game, &hit);
        assert_eq!(game.world.tile(6, 5).unwrap().object, "");
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|e| matches!(e.object.as_str(), "oRubble" | "oRubbleSmall"))
                .count(),
            3
        );
        assert!(game.world.entity(id).is_some());
        assert_eq!(game.rng.next_u64(), expected.next_u64());
    }
    #[test]
    fn melee_uses_selected_character_attack_sprite() {
        for (flag, sprite) in [
            ("character_damsel", "sDamselAttackL"),
            ("character_tunnelman", "sTunnelAttackL"),
        ] {
            let (mut game, id) = armed("oMachete");
            game.flags.insert(flag.into());
            use_weapon(&mut game, id, &InputFrame::default());
            assert_eq!(game.player.sprite, sprite);
            tick(&mut game);
            assert_eq!(game.player.sprite, sprite);
        }
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
        assert!(
            !game
                .world
                .entities
                .iter()
                .any(|e| e.object == "oMachetePre")
        );
        game.step(&InputFrame::default());
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
        for _ in 0..4 {
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
        let id = spawn_web_ball(&mut game, Vec2::new(70.0, 78.0));
        game.world.entity_mut(id).unwrap().vel = Vec2::new(12.0, 0.0);
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
        let wave = game.world.entity_mut(id).unwrap();
        init_entity(wave);
        wave.timer = 0;
        wave.vel.x = 6.0;
        tick(&mut game);
        let wave = game.world.entity(id).unwrap();
        assert!(wave.pos.x > 100.0);
        assert!(wave.pos.y > 100.0);
        assert!(((wave.pos.x - 100.0).hypot(wave.pos.y - 100.0) - 2.0).abs() < 1e-12);
        assert_eq!(
            wave.vel.x, 6.0,
            "source homing updates direction rather than xVel"
        );
    }

    #[test]
    fn game_step_arrow_and_flare_follow_room_move_to_fraction_phase() {
        for name in ["oArrow", "oFlare"] {
            let (mut game, _) = flat_room("oMachete");
            let id = game.world.spawn(name, Vec2::new(200.0, 80.0));
            game.world.entity_mut(id).unwrap().vel.x = 0.3;
            game.step(&InputFrame::default());
            assert_eq!(game.engine_time(), 2);
            assert_eq!(game.world.entity(id).unwrap().pos.x, 200.0, "{name}");
            game.step(&InputFrame::default());
            assert_eq!(game.engine_time(), 3);
            assert_eq!(game.world.entity(id).unwrap().pos.x, 201.0, "{name}");
        }
    }

    #[test]
    fn game_step_arrow_bounces_before_testing_embedding_speed() {
        let (mut game, _) = flat_room("oMachete");
        game.world.set_tile(10, 5, "oBlock");
        let id = game.world.spawn("oArrow", Vec2::new(148.0, 88.0));
        game.world.entity_mut(id).unwrap().vel.x = 8.0;
        game.step(&InputFrame::default());
        let arrow = game.world.entity(id).unwrap();
        assert_eq!(arrow.pos.x, 155.0);
        assert_eq!(arrow.vel.x, -4.0);
        assert_eq!(arrow.value("stuck"), 0.0);
    }

    #[test]
    fn game_step_flare_rebounds_from_ceiling() {
        let (mut game, _) = flat_room("oMachete");
        game.world.set_tile(10, 5, "oBlock");
        let id = game.world.spawn("oFlare", Vec2::new(168.0, 100.0));
        game.world.entity_mut(id).unwrap().vel.y = -4.0;
        game.step(&InputFrame::default());
        assert!(game.world.entity(id).unwrap().vel.y > 0.0);
        assert_eq!(game.world.entity(id).unwrap().pos.y, 100.0);
    }

    #[test]
    fn game_step_webball_collides_with_pickup_items() {
        let (mut game, _) = flat_room("oMachete");
        game.world.spawn("oRock", Vec2::new(200.0, 100.0));
        let id = spawn_web_ball(&mut game, Vec2::new(200.0, 100.0));
        game.world.entity_mut(id).unwrap().vel = Vec2::default();
        game.step(&InputFrame::default());
        assert_eq!(game.world.entity(id).unwrap().sprite, "sWebCreate");
        assert_eq!(game.world.entity(id).unwrap().state, 1);
    }

    #[test]
    fn game_step_web_create_preserves_source_corner_origin() {
        let (mut game, _) = flat_room("oMachete");
        let id = spawn_web_ball(&mut game, Vec2::new(200.0, 100.0));
        let ball = game.world.entity_mut(id).unwrap();
        ball.set_sprite("sWebCreate");
        ball.vel = Vec2::default();
        ball.animation = 4.0;
        ball.data.insert("life".into(), 50.0);
        game.step(&InputFrame::default());
        let web = game
            .world
            .entities
            .iter()
            .find(|e| e.object == "oWeb")
            .unwrap();
        assert_eq!(web.pos, Vec2::new(192.0, 92.0));
    }

    #[test]
    fn game_step_down_does_not_suppress_airborne_gunfire() {
        for name in ["oPistol", "oShotgun", "oSceptre", "oWebCannon", "oMachete"] {
            let (mut game, _) = flat_room(name);
            game.player.pos.y = 100.0;
            game.player.grounded = false;
            game.player.state = PlayerState::Falling;
            game.step(&InputFrame {
                down: true,
                attack: true,
                attack_pressed: true,
                ..Default::default()
            });
            if name == "oMachete" {
                game.step(&InputFrame::default());
            }
            assert!(
                game.world.entities.iter().any(|e| matches!(
                    e.object.as_str(),
                    "oBullet" | "oPsychicWaveP" | "oWebBall" | "oMachetePre"
                )),
                "{name}"
            );
        }
    }

    #[test]
    fn game_step_mattock_keeps_title_and_score_terrain_intact() {
        for room in ["rTitle", "rHighscores"] {
            let (mut game, _) = flat_room("oMattock");
            game.world.name = room.into();
            game.world.set_tile(6, 9, "oBlock");
            game.player.attack_timer = 12;
            game.player.set_sprite("sAttackLeft");
            let id = game.world.spawn("oMattockHit", Vec2::new(96.0, 152.0));
            let hit = game.world.entity_mut(id).unwrap();
            hit.animation = (crate::catalog::sprite_frame_count(&hit.sprite) - 1) as f64 + 0.5;
            game.step(&InputFrame::default());
            assert_eq!(game.world.tile(6, 9).unwrap().object, "oBlock", "{room}");
            assert!(
                game.world.entity(id).is_none(),
                "Animation End consumes the active hit even in protected rooms"
            );
        }
    }

    #[test]
    fn game_step_offscreen_arrow_keeps_body_frozen_but_bomb_fuse_runs() {
        let (mut game, _) = flat_room("oMachete");
        game.world = World::new(100, 20, Area::Mines, 1);
        game.player.held = None;
        let id = game.world.spawn("oArrow", Vec2::new(380.0, 100.0));
        let arrow = game.world.entity_mut(id).unwrap();
        arrow.vel.x = 8.0;
        arrow.data.insert("bomb_arrow".into(), 1.0);
        arrow.alarms[1] = 2;
        game.step(&InputFrame::default());
        let arrow = game.world.entity(id).unwrap();
        assert_eq!(arrow.pos, Vec2::new(380.0, 100.0));
        assert_eq!(arrow.alarms[1], 1);
        game.step(&InputFrame::default());
        assert!(game.world.entity(id).is_none());
    }

    #[test]
    fn game_step_webball_lifetime_uses_source_random_range_and_expiry_order() {
        let mut distinct = std::collections::BTreeSet::new();
        for seed in 1..=32 {
            let mut game = Game::shell(seed, RulesPreset::Classic);
            game.player.pos = Vec2::new(40.0, 40.0);
            let id = spawn_web_ball(&mut game, Vec2::new(200.0, 60.0));
            game.step(&InputFrame::default());
            let life = game.world.entity(id).unwrap().value("life");
            assert!((19.0..=99.0).contains(&life));
            distinct.insert(life as i32);
            game.world
                .entity_mut(id)
                .unwrap()
                .data
                .insert("life".into(), 2.0);
            game.step(&InputFrame::default());
            assert_eq!(game.world.entity(id).unwrap().value("life"), 1.0);
            game.step(&InputFrame::default());
            assert_eq!(game.world.entity(id).unwrap().state, 0);
            game.step(&InputFrame::default());
            assert_eq!(game.world.entity(id).unwrap().state, 1);
        }
        assert!(
            distinct.len() > 1,
            "Create lifetime must vary rather than remain fixed at 60"
        );
    }

    #[test]
    fn arrow_rotation_preserves_source_quadrants_and_stuck_angle() {
        for (velocity, degrees) in [
            (Vec2::new(1.0, -1.0), -45.0),
            (Vec2::new(-1.0, -1.0), -135.0),
            (Vec2::new(1.0, 1.0), -45.0),
            (Vec2::new(-1.0, 1.0), -225.0),
            (Vec2::new(0.0, 1.0), 0.0),
        ] {
            assert!((item_rotation(velocity, false, 1.0).to_degrees() - degrees).abs() < 1e-10);
        }
        assert_eq!(item_rotation(Vec2::default(), true, 0.7), 0.7);
    }

    fn incoming_rock(mitt: bool) -> (Game, u64) {
        let (mut game, weapon) = flat_room("oMachete");
        game.world.entity_mut(weapon).unwrap().alive = false;
        game.player.held = None;
        if mitt {
            game.player.equipment.insert("Mitt".into());
        }
        let id = game.world.spawn("oRock", Vec2::new(128.0, 148.0));
        game.world.entity_mut(id).unwrap().vel.x = -8.0;
        (game, id)
    }

    #[test]
    fn game_step_unsafe_rock_damages_player_and_survives_impact() {
        let (mut game, id) = incoming_rock(false);
        game.player.invincible = 60;
        for _ in 0..8 {
            game.step(&InputFrame::default());
            if game.player.health < 4 {
                break;
            }
        }
        assert_eq!(game.player.health, 2);
        assert_eq!(game.player.stun, 20);
        assert_eq!(game.player.vel.x, -8.0);
        assert_eq!(game.player.vel.y, -4.0);
        assert!(
            game.world.entity(id).is_some(),
            "Rock is not consumed like an Arrow"
        );
    }

    #[test]
    fn game_step_mitt_catches_incoming_rock_only_with_empty_hands() {
        let (mut game, id) = incoming_rock(true);
        for _ in 0..8 {
            game.step(&InputFrame::default());
            if game.player.held.is_some() {
                break;
            }
        }
        assert_eq!(game.player.health, 4);
        assert_eq!(game.player.stun, 0);
        assert_eq!(game.player.held, Some(id));
        assert!(game.world.entity(id).unwrap().held);
        let (mut occupied, _) = incoming_rock(true);
        let held = occupied.world.spawn("oPistol", occupied.player.pos);
        occupied.world.entity_mut(held).unwrap().held = true;
        occupied.player.held = Some(held);
        for _ in 0..8 {
            occupied.step(&InputFrame::default());
            if occupied.player.health < 4 {
                break;
            }
        }
        assert_eq!(occupied.player.health, 2);
        assert!(occupied.player.held.is_none());
    }

    #[test]
    fn game_step_safe_and_slow_rocks_do_not_hurt_or_get_caught() {
        for mitt in [false, true] {
            for safe in [false, true] {
                let (mut game, id) = incoming_rock(mitt);
                if safe {
                    game.world
                        .entity_mut(id)
                        .unwrap()
                        .data
                        .insert("safe".into(), 20.0);
                } else {
                    game.world.entity_mut(id).unwrap().vel.x = -4.0;
                }
                for _ in 0..8 {
                    game.step(&InputFrame::default());
                }
                assert_eq!(game.player.health, 4);
                assert!(game.player.held.is_none());
            }
        }
    }

    #[test]
    fn game_step_flare_uses_source_ninety_six_pixel_light_radius() {
        let (mut game, _) = flat_room("oMachete");
        let id = game.world.spawn("oFlare", Vec2::new(200.0, 80.0));
        game.step(&InputFrame::default());
        assert_eq!(game.world.entity(id).unwrap().value("light_radius"), 96.0);
    }

    #[test]
    fn game_step_webball_collision_requires_shared_precise_pixels() {
        for (rock_pos, expected_state) in [(Vec2::new(72.0, 72.0), 0), (Vec2::new(74.0, 74.0), 1)] {
            let mut game = Game::shell(1, RulesPreset::Classic);
            game.player.pos = Vec2::new(24.0, 24.0);
            game.world.spawn("oRock", rock_pos);
            let id = spawn_web_ball(&mut game, Vec2::new(79.0, 82.0));
            let ball = game.world.entity_mut(id).unwrap();
            ball.vel = Vec2::new(1.0, -2.0);
            ball.data.insert("life".into(), 50.0);
            game.step(&InputFrame::default());
            let ball = game.world.entity(id).unwrap();
            assert_eq!(ball.pos, Vec2::new(80.0, 80.0));
            assert_eq!(ball.state, expected_state, "rock at {rock_pos:?}");
        }
    }

    #[test]
    fn game_step_flare_light_distance_samples_after_inherited_motion() {
        let (mut game, _) = flat_room("oMachete");
        let id = game.world.spawn("oFlare", Vec2::new(200.0, 140.0));
        game.world.entity_mut(id).unwrap().vel.x = 6.0;
        // Flare Step samples after inherited motion, before the first Player
        // Draw changes its persisted image_xscale from Create's +1 to -1.
        let player_before_draw = game.player.clone();
        let before = crate::world_events::mask_distance(
            game.world
                .entity(id)
                .unwrap()
                .collision_mask()
                .point_bounds(),
            player_before_draw.collision_mask().point_bounds(),
        );
        game.step(&InputFrame::default());
        assert_eq!(game.player.pos, player_before_draw.pos);
        assert_eq!(game.player.sprite, player_before_draw.sprite);
        let flare = game.world.entity(id).unwrap();
        assert_eq!(flare.pos.x, 206.0);
        let sampled = flare.value("dist_to_player");
        assert!(
            sampled > before,
            "Flare's own Step samples its moved position"
        );
        assert_eq!(
            sampled,
            crate::world_events::mask_distance(
                flare.collision_mask().point_bounds(),
                player_before_draw.collision_mask().point_bounds()
            )
        );
        assert_ne!(
            sampled,
            crate::world_events::mask_distance(
                flare.collision_mask().point_bounds(),
                game.player.collision_mask().point_bounds()
            ),
            "the later Player Draw changes the mask after Flare Step sampled it"
        );
    }

    #[test]
    fn game_step_cooked_bomb_arrow_illuminates_on_its_expiry_tick() {
        let (mut game, _) = flat_room("oBow");
        game.world.dark = true;
        game.player.data.insert("bow_bomb_armed".into(), 1.0);
        game.player.data.insert("bow_bomb_fuse".into(), 0.0);
        game.step(&InputFrame::default());
        assert!(game.world.entities.iter().any(|e| e.object == "oExplosion"));
        assert_eq!(game.player.data["dist_to_nearest_light_source"], 0.0);
        assert_eq!(game.data["darkness"], 0.0);
        assert_eq!(game.player.data["bow_bomb_armed"], 0.0);
    }

    #[test]
    fn game_step_web_cannon_executes_create_randomness_before_launch_override() {
        let (mut game, _) = flat_room("oWebCannon");
        let mut expected = game.rng.clone();
        expected.unit(); // WebBall Create yVel
        expected.range(3); // WebBall Create xVel
        expected.range(2); // WebBall Create sign
        let life = 20 + expected.range(81);
        let speed = f64::from(6 + expected.range(3)); // scrUseItem override
        game.step(&InputFrame {
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        let web = game
            .world
            .entities
            .iter()
            .find(|e| e.object == "oWebBall")
            .unwrap();
        assert_eq!(web.vel.x, speed);
        assert_eq!(web.vel.y, 0.2);
        assert_eq!(web.value("life"), f64::from(life - 1));
        assert_eq!(web.value("web_create_initialized"), 1.0);
        assert_eq!(game.rng.next_u64(), expected.next_u64());
    }

    #[test]
    fn web_ball_create_defaults_follow_source_y_x_sign_life_order_once() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        let mut expected = game.rng.clone();
        let y = -(expected.unit() * 3.0 + 1.0);
        let mut x = f64::from(1 + expected.range(3));
        if expected.chance(1, 2) {
            x *= -1.0;
        }
        let life = f64::from(20 + expected.range(81));
        let mut web = Entity::new(1, "oWebBall", Vec2::new(80.0, 80.0));
        initialize_web_ball(&mut game, &mut web);
        assert_eq!(web.vel, Vec2::new(x, y));
        assert_eq!(web.value("life"), life);
        initialize_web_ball(&mut game, &mut web);
        assert_eq!(game.rng.next_u64(), expected.next_u64());
    }

    #[test]
    fn game_step_natural_mattock_break_creates_an_active_pickup_head() {
        let (mut game, _) = flat_room("oMattock");
        // oBlock Destroy consumes twelve rand(0,8) position draws. Canonical
        // seed 9 then gives raw word 13_465_890: rand(1,20) == 1 at draw 13.
        // Rubble and MattockHead Create callbacks consume no random draws.
        game.rng = Rng::new(9);
        game.world.set_tile(6, 9, "oBlock");
        game.step(&InputFrame {
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        for _ in 0..20 {
            if game
                .world
                .entities
                .iter()
                .any(|e| e.object == "oMattockHead")
            {
                break;
            }
            game.step(&InputFrame::default());
        }
        let head = game
            .world
            .entities
            .iter()
            .find(|e| e.object == "oMattockHead")
            .unwrap();
        let (head_id, y) = (head.id, head.pos.y);
        assert_eq!(
            head.value("canPickUp"),
            1.0,
            "Create completes before caller's upward launch"
        );
        assert_eq!(head.value("bboxLeft"), -6.0);
        assert_eq!(head.vel.y, -2.0);
        assert!(game.player.held.is_none());
        assert_eq!(game.rng.state_bits(), 13_465_890);
        game.step(&InputFrame::default());
        assert_eq!(game.world.entity(head_id).unwrap().pos.y, y - 2.0);
    }

    #[test]
    fn game_step_sceptre_creates_three_source_particles_before_the_wave() {
        for facing in [-1, 1] {
            let (mut game, _) = flat_room("oSceptre");
            game.player.facing = facing;
            let origin = Vec2::new(
                game.player.pos.x + 12.0 * f64::from(facing),
                game.player.pos.y + 4.0,
            );
            let mut expected = game.rng.clone();
            let velocities: Vec<_> = (0..3)
                .map(|_| {
                    Vec2::new(
                        f64::from(facing) * f64::from(1 + expected.range(3)),
                        -expected.unit() * 2.0,
                    )
                })
                .collect();
            game.step(&InputFrame {
                attack: true,
                attack_pressed: true,
                ..Default::default()
            });
            let particles: Vec<_> = game
                .world
                .entities
                .iter()
                .filter(|e| e.object == "oPsychicCreateP")
                .collect();
            assert_eq!(particles.len(), 3);
            for (particle, velocity) in particles.iter().zip(velocities) {
                assert_eq!(
                    particle.pos,
                    Vec2::new(origin.x + velocity.x, origin.y + velocity.y)
                );
                assert_eq!(particle.vel, Vec2::new(velocity.x, velocity.y + 0.6));
                assert_eq!(particle.animation, 0.2);
            }
            let wave = game
                .world
                .entities
                .iter()
                .find(|e| e.object == "oPsychicWaveP")
                .unwrap();
            assert!(particles.iter().all(|particle| particle.id < wave.id));
            assert_eq!(game.rng.next_u64(), expected.next_u64());
        }
    }

    #[test]
    fn game_step_real_exit_protects_all_characters_from_arrows_rocks_and_bullets() {
        for character in [
            crate::characters::Character::Explorer,
            crate::characters::Character::Damsel,
            crate::characters::Character::TunnelMan,
        ] {
            for object in ["oArrow", "oRock", "oBullet"] {
                let (mut game, _) = flat_room("oMachete");
                crate::characters::apply(&mut game, character);
                game.world.spawn("oExit", Vec2::new(72.0, 144.0));
                game.step(&InputFrame {
                    up: true,
                    ..Default::default()
                });
                assert!(
                    game.player.is_exiting(),
                    "fixture must enter a real exit: {character:?}"
                );
                let health = game.player.health;
                let pos = game.player.pos;
                let id = game.world.spawn(object, Vec2::new(pos.x - 18.0, pos.y));
                game.world.entity_mut(id).unwrap().vel.x =
                    if object == "oBullet" { 12.0 } else { 8.0 };
                let mut rng = game.rng.clone();
                game.step(&InputFrame::default());
                assert_eq!(game.player.health, health, "{character:?} {object}");
                assert_eq!(game.player.stun, 0, "{character:?} {object}");
                assert_eq!(game.player.vel, Vec2::default(), "{character:?} {object}");
                assert!(
                    game.world.entity(id).is_some(),
                    "exit collision must not consume {object}"
                );
                assert!(!game.events.iter().any(|event| match event {
                    GameEvent::Damage { .. } => true,
                    GameEvent::Sound { name, .. } => matches!(name.as_str(), "sndHurt" | "sndHit"),
                    _ => false,
                }));
                assert_eq!(game.rng.next_u64(), rng.next_u64());
            }
        }
    }

    #[test]
    fn game_step_non_exit_projectiles_still_hurt_all_characters() {
        for character in [
            crate::characters::Character::Explorer,
            crate::characters::Character::Damsel,
            crate::characters::Character::TunnelMan,
        ] {
            for object in ["oArrow", "oRock", "oBullet"] {
                let (mut game, _) = flat_room("oMachete");
                crate::characters::apply(&mut game, character);
                let health = game.player.health;
                let pos = game.player.pos;
                let id = game.world.spawn(object, Vec2::new(pos.x - 18.0, pos.y));
                game.world.entity_mut(id).unwrap().vel.x =
                    if object == "oBullet" { 12.0 } else { 8.0 };
                for _ in 0..4 {
                    game.step(&InputFrame::default());
                    if game.player.health < health {
                        break;
                    }
                }
                let amount = if object == "oBullet" { 4 } else { 2 };
                assert_eq!(
                    game.player.health,
                    health - amount,
                    "{character:?} {object}"
                );
                assert_eq!(game.player.stun, 20, "{character:?} {object}");
                assert!(game.events.contains(&GameEvent::Damage { amount }));
            }
        }
    }

    #[test]
    fn game_step_real_exit_skips_mitt_catch_as_well_as_rock_damage() {
        let (mut game, weapon) = flat_room("oMachete");
        game.world.entity_mut(weapon).unwrap().alive = false;
        game.player.held = None;
        let mitt = game.spawn("oMitt", Vec2::new(80.0, 156.0));
        // Its inherited source Step clears the default price before pickup.
        game.step(&InputFrame::default());
        assert_eq!(game.world.entity(mitt).unwrap().value("cost"), 0.0);
        game.step(&InputFrame {
            down: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert!(game.player.has("Mitt"));
        game.world.spawn("oExit", Vec2::new(72.0, 144.0));
        game.step(&InputFrame {
            up: true,
            ..Default::default()
        });
        assert!(game.player.is_exiting());
        let rock = game.world.spawn(
            "oRock",
            Vec2::new(game.player.pos.x - 18.0, game.player.pos.y),
        );
        game.world.entity_mut(rock).unwrap().vel.x = 8.0;
        game.step(&InputFrame::default());
        assert_eq!(game.player.health, 4);
        assert!(game.player.held.is_none());
        assert!(!game.world.entity(rock).unwrap().held);
    }

    #[test]
    fn game_step_fired_web_cannon_visibly_expands_then_creates_web() {
        let (mut game, _) = flat_room("oWebCannon");
        for y in 5..10 {
            game.world.set_tile(8, y, "oBlock");
        }
        game.step(&InputFrame {
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        let id = game
            .world
            .entities
            .iter()
            .find(|e| e.object == "oWebBall")
            .unwrap()
            .id;
        for _ in 0..20 {
            if game.world.entity(id).unwrap().sprite == "sWebCreate" {
                break;
            }
            game.step(&InputFrame::default());
        }
        let ball = game
            .world
            .entity(id)
            .expect("flight must reach an expanding WebBall");
        assert_eq!(ball.sprite, "sWebCreate");
        assert_eq!(
            ball.animation, 1.0,
            "collision changes sprite before one central animation advance"
        );
        let corner = Vec2::new(ball.pos.x - 8.0, ball.pos.y - 8.0);
        assert_eq!(crate::catalog::sprite_frame_count("sWebCreate"), 5);
        for frame in 2..=4 {
            game.step(&InputFrame::default());
            let ball = game
                .world
                .entity(id)
                .expect("all five source expansion frames are visible");
            assert_eq!(ball.sprite, "sWebCreate");
            assert_eq!(ball.animation, f64::from(frame));
            assert!(!game.world.entities.iter().any(|e| e.object == "oWeb"));
        }
        game.step(&InputFrame::default());
        assert!(game.world.entity(id).is_none());
        let web = game
            .world
            .entities
            .iter()
            .find(|e| e.object == "oWeb")
            .unwrap();
        assert_eq!(web.pos, corner);
        assert_eq!(web.value("dying"), 1.0);
    }

    #[test]
    fn game_step_arrow_and_rock_statistics_only_record_lethal_real_level_damage() {
        for (object, index) in [("oArrow", 6), ("oRock", 0)] {
            for (room, health, exit, expected) in [
                ("rLevel", 1, false, 1),
                ("rLevel", 4, false, 0),
                ("rTitle", 1, false, 0),
                ("rLevel", 1, true, 0),
                ("rLevel", 0, false, 0),
            ] {
                let mut game = Game::shell(17, RulesPreset::Classic);
                game.world.name = room.into();
                game.player.pos = Vec2::new(80.0, 80.0);
                game.player.health = health;
                if exit {
                    game.player.set_sprite("sPExit");
                }
                let id = game.world.spawn(object, game.player.pos);
                game.world.entity_mut(id).unwrap().vel.x = 6.0;
                game.step(&InputFrame::default());
                assert_eq!(
                    game.lifetime_stats.misc_deaths[index], expected,
                    "{object} {room} health={health} exit={exit}"
                );
                assert_eq!(
                    game.lifetime_stats.misc_deaths.iter().sum::<u64>(),
                    expected
                );
            }
        }
    }

    #[test]
    fn webball_life_expiry_preserves_motion_and_fractional_animation_index() {
        let mut game = Game::shell(18, RulesPreset::Classic);
        let id = spawn_web_ball(&mut game, Vec2::new(150.0, 80.0));
        let ball = game.world.entity_mut(id).unwrap();
        ball.vel = Vec2::new(2.0, 5.9);
        ball.animation = 0.75;
        ball.data.insert("life".into(), 0.0);
        step_instance(&mut game, id);
        let ball = game.world.entity(id).unwrap();
        assert_eq!(ball.pos, Vec2::new(152.0, 85.9));
        assert!((ball.vel.y - 6.1).abs() < 1e-12);
        assert_eq!(ball.sprite, "sWebCreate");
        assert_eq!(ball.animation, 0.75);
        assert_eq!(ball.vel.x, 2.0);
        step_instance(&mut game, id);
        assert_eq!(game.world.entity(id).unwrap().pos.x, 154.0);
        assert_eq!(
            game.world.entity(id).unwrap().animation,
            0.75,
            "Step owns no animation clock"
        );
    }

    #[test]
    fn game_step_lethal_bullet_records_source_shopkeeper_death_without_misc_death() {
        for (room, health, expected) in [("rLevel", 4, 1), ("rLevel", 8, 0), ("rTitle", 4, 0)] {
            let mut game = Game::shell(19, RulesPreset::Classic);
            game.world.name = room.into();
            game.player.pos = Vec2::new(80.0, 80.0);
            game.player.health = health;
            let id = game.world.spawn("oBullet", Vec2::new(68.0, 80.0));
            game.world.entity_mut(id).unwrap().vel.x = 12.0;
            game.step(&InputFrame::default());
            assert_eq!(
                game.lifetime_stats.enemy_deaths[19], expected,
                "{room} hp={health}"
            );
            assert_eq!(game.lifetime_stats.misc_deaths, [0; 12]);
            assert!(game.world.entity(id).is_none());
        }
    }

    #[test]
    fn game_step_arrow_hurt_query_precedes_the_arrow_normal_step() {
        let mut game = Game::shell(20, RulesPreset::Classic);
        game.player.pos = Vec2::new(80.0, 80.0);
        let id = game.spawn("oArrow", Vec2::new(60.0, 80.0));
        game.world.entity_mut(id).unwrap().vel.x = 8.0;
        game.step(&InputFrame::default());
        assert_eq!(game.world.entity(id).unwrap().pos.x, 68.0);
        assert_eq!(
            game.player.health, 4,
            "entering the hurt query after Player Step waits until next Step"
        );
        game.step(&InputFrame::default());
        assert_eq!(game.world.entity(id).unwrap().pos.x, 76.0);
        assert_eq!(
            game.player.health, 4,
            "source cached bbox ends at71; right+1 equals query min72, so it still misses"
        );
        game.step(&InputFrame::default());
        assert_eq!(game.player.health, 2);
        assert!(game.world.entity(id).is_none());
    }

    #[test]
    fn game_step_bullet_moves_once_before_automatic_solid_collision() {
        let mut game = Game::shell(21, RulesPreset::Classic);
        game.player.pos = Vec2::new(24.0, 24.0);
        game.world.set_tile(5, 5, "oBlock");
        let id = game.spawn("oBullet", Vec2::new(70.0, 88.0));
        game.world.entity_mut(id).unwrap().vel.x = 24.0;
        game.step(&InputFrame::default());
        assert!(game.world.entity(id).is_none());
        let smoke = game
            .world
            .entities
            .iter()
            .find(|e| e.object == "oSmokePuff")
            .unwrap();
        assert_eq!(
            smoke.pos.x, 94.0,
            "source Bullet Step adds velocity before Collision; no sweep or duplicate built-in motion"
        );
    }

    #[test]
    fn embedded_arrow_inherited_step_still_queries_enemy_after_source_nudge() {
        let mut game = Game::shell(22, RulesPreset::Classic);
        game.player.pos = Vec2::new(24.0, 24.0);
        game.world.set_tile(5, 5, "oBlock");
        let snake = game.spawn("oSnake", Vec2::new(80.0, 80.0));
        let arrow = game.spawn("oArrow", Vec2::new(80.0, 88.0));
        game.world.entity_mut(arrow).unwrap().vel.x = 8.0;
        let hp = game.world.entity(snake).unwrap().health;
        step_instance(&mut game, arrow);
        assert!(
            game.world.entity(arrow).is_none(),
            "inherited impact runs outside the embedded movement branch"
        );
        assert!(
            game.world
                .entities
                .iter()
                .find(|e| e.id == snake)
                .unwrap()
                .health
                < hp
        );
    }
}

#[cfg(test)]
mod bullet_character_source_tests {
    use super::*;
    use crate::dispatcher::Instance;

    // Literal oBlood inherited/own Create: its caller is direct instance_create,
    // not scrCreateBlood. The four draws also run after inherited cap removal.
    fn literal_blood_create(rng: &mut Rng) -> (Vec2, f64) {
        let velocity = Vec2::new(rng.unit() * 4.0 - rng.unit() * 4.0, -1.0 - rng.unit() * 2.0);
        let gravity = f64::from(1 + rng.range(6)) * 0.1;
        (velocity, gravity)
    }

    #[test]
    fn bullet_character_literal_health_exit_dead_flags_caps_rng_and_retirement() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            for (health, dead, room) in [
                (8, false, "rLevel"),
                (4, false, "rLevel"),
                (0, false, "rLevel"),
                (-2, false, "rLevel"),
                (8, true, "rLevel"),
                (0, true, "rLevel"),
                (4, false, "rTitle"),
            ] {
                for sprite in ["sStandLeft", "sPExit", "sDamselExit", "sTunnelExit"] {
                    for count in [0, 15, 16, 31, 32] {
                        for (safe, owner, protected, bloodless) in [
                            (false, false, false, false),
                            (true, false, false, false),
                            (true, true, true, false),
                            (true, true, true, true),
                        ] {
                            let mut game = Game::shell(7, rules);
                            game.world.name = room.into();
                            game.player.pos = Vec2::new(104.0, 120.0);
                            game.player.health = health;
                            game.player.vel = Vec2::new(1.0, 2.0);
                            game.player.invincible = if protected { 100 } else { 0 };
                            game.player.stun = if protected { 9 } else { 0 };
                            if dead {
                                game.player.state = PlayerState::Dead;
                                game.state = RunState::Dead;
                            }
                            game.player.set_sprite(sprite);
                            game.player
                                .data
                                .insert("bloodless".into(), f64::from(bloodless));
                            if bloodless {
                                game.flags.insert("bloodless".into());
                            }
                            for _ in 0..count {
                                game.spawn("oBone", Vec2::new(200.0, 120.0));
                            }
                            let held = game.spawn("oRock", game.player.pos);
                            game.world.entity_mut(held).unwrap().held = true;
                            game.player.held = Some(held);
                            let bullet = game.spawn("oBullet", Vec2::new(104.0, 118.0));
                            let projectile = game.world.entity_mut(bullet).unwrap();
                            projectile.vel = Vec2::new(12.0, 0.5);
                            projectile.data.insert("safe".into(), f64::from(safe));
                            projectile
                                .data
                                .insert("owner_player".into(), f64::from(owner));
                            let mut oracle = game.rng.clone();
                            let exiting = sprite != "sStandLeft";
                            let expected_birth = if exiting {
                                None
                            } else {
                                Some(literal_blood_create(&mut oracle))
                            };
                            collision(&mut game, bullet, Instance::Player);
                            let births: Vec<_> = game
                                .world
                                .entities
                                .iter()
                                .filter(|e| e.id > bullet)
                                .collect();
                            assert_eq!(
                                births.len(),
                                usize::from(!exiting),
                                "health={health} dead={dead} sprite={sprite} count={count}"
                            );
                            if let Some((velocity, gravity)) = expected_birth {
                                let blood = births[0];
                                assert_eq!(blood.object, "oBlood");
                                assert_eq!(blood.pos, Vec2::new(104.0, 120.0));
                                assert_eq!(blood.vel, velocity);
                                assert_eq!(blood.value("gravity"), gravity);
                                assert_eq!(blood.alive, count < 32);
                                assert_eq!(blood.value("life"), 60.0);
                                assert_eq!(blood.value("bounce"), 0.0);
                                assert_eq!(blood.value("invincible"), 1.0);
                                assert_eq!(blood.value("collectible"), 0.0);
                                assert_eq!(&blood.alarms[..3], &[1, 1, 5]);
                                assert_eq!(blood.image_speed(), 0.3);
                            }
                            let damaged = !exiting && health > 0;
                            assert_eq!(
                                game.player.health,
                                if damaged { health - 4 } else { health }
                            );
                            assert_eq!(
                                game.player.vel,
                                if exiting {
                                    Vec2::new(1.0, 2.0)
                                } else {
                                    Vec2::new(12.0, -4.0)
                                }
                            );
                            assert_eq!(
                                game.player.stun,
                                if exiting {
                                    if protected { 9 } else { 0 }
                                } else {
                                    20
                                }
                            );
                            assert_eq!(game.world.entity(bullet).is_some(), exiting);
                            assert_eq!(
                                game.player.held,
                                Some(held),
                                "Bullet does not release held item in this callback"
                            );
                            assert_eq!(
                                game.lifetime_stats.enemy_deaths[19],
                                u64::from(damaged && health <= 4 && room == "rLevel")
                            );
                            assert_eq!(game.lifetime_stats.misc_deaths, [0; 12]);
                            assert_eq!(
                                game.events
                                    .iter()
                                    .filter(|e| matches!(e, GameEvent::Damage { amount: 4 }))
                                    .count(),
                                usize::from(damaged)
                            );
                            assert_eq!(game.events.iter().filter(|e| matches!(e, GameEvent::Sound{name,..} if name=="sndHurt")).count(), usize::from(!exiting));
                            assert_eq!(game.rng.clone().next_u64(), oracle.next_u64());
                            if !exiting {
                                let before = serde_json::to_value(&game.world).unwrap();
                                let events = game.events.len();
                                let rng = game.rng.clone().next_u64();
                                collision(&mut game, bullet, Instance::Player);
                                assert_eq!(serde_json::to_value(&game.world).unwrap(), before);
                                assert_eq!(game.events.len(), events);
                                assert_eq!(game.rng.next_u64(), rng);
                            }
                        }
                    }
                }
            }
        }
    }
}

include!("arrow_player_blood_tests.rs");

#[cfg(test)]
#[path = "rock_player_blood_tests.rs"]
mod rock_player_blood_source_tests;

#[cfg(test)]
include!("projectile_query_contract.rs");
