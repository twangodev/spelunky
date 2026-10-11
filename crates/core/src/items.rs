//! Classic item interactions. Reference: oItem, oPlayer1 Step and scrUseItem.
use crate::model::*;
#[path = "items/buried_create.rs"]
mod buried_create;
#[path = "items/collectibles.rs"]
pub mod collectibles;
#[path = "items/explosives.rs"]
pub mod explosives;
#[path = "items/utility.rs"]
pub mod utility;
#[path = "items/weapons.rs"]
pub mod weapons;
pub(crate) use utility::initialize_dice;

pub fn is_item(name: &str) -> bool {
    collectibles::is_collectible(name)
        || matches!(
            name,
            "oRock"
                | "oSkull"
                | "oJar"
                | "oFishBone"
                | "oGoldIdol"
                | "oCrystalSkull"
                | "oLampItem"
                | "oLampRedItem"
                | "oBasketball"
                | "oDice"
                | "oBomb"
                | "oFireFrogBomb"
                | "oFireFrogArmed"
                | "oBall"
                | "oBall2"
                | "oRope"
                | "oRopeThrow"
                | "oArrow"
                | "oBow"
                | "oPistol"
                | "oShotgun"
                | "oSceptre"
                | "oWebCannon"
                | "oMachete"
                | "oMattock"
                | "oMattockHead"
                | "oTeleporter"
                | "oFlare"
                | "oChest"
                | "oLockedChest"
                | "oKey"
                | "oCrate"
        )
}

pub fn init_entity(entity: &mut Entity) {
    if entity.object == "oBow" && !entity.data.contains_key("new") {
        let new = f64::from(entity.value("bow_picked_up") == 0.0);
        entity.data.insert("new".into(), new);
    }
    if entity.value("items_initialized") != 0.0 {
        return;
    }
    entity.data.insert("items_initialized".into(), 1.0);
    if is_item(&entity.object) && !matches!(entity.object.as_str(), "oArrow" | "oFlare") {
        entity.data.entry("gravity".into()).or_insert(0.6);
        entity.data.entry("bounceFactor".into()).or_insert(0.5);
        entity.data.entry("frictionFactor".into()).or_insert(0.3);
        entity.data.entry("canPickUp".into()).or_insert(1.0);
        entity.data.entry("damage".into()).or_insert(1.0);
        entity.data.entry("breakPieces".into()).or_insert(1.0);
        if entity.object == "oFishBone" {
            entity.data.insert("gravity".into(), 0.2);
        }
    }
    if matches!(
        entity.object.as_str(),
        "oGoldIdol" | "oCrystalSkull" | "oLampItem" | "oLampRedItem"
    ) {
        entity.data.insert("heavy".into(), 1.0);
    }
    if matches!(entity.object.as_str(), "oGoldIdol" | "oCrystalSkull") {
        entity.data.entry("trigger".into()).or_insert(1.0);
        let value = if entity.object == "oCrystalSkull" {
            15000.0
        } else {
            5000.0
        };
        entity.data.entry("value".into()).or_insert(value);
    }
    if entity.object == "oFireFrogArmed" {
        entity.data.insert("heavy".into(), 1.0);
        for (key, value) in [
            ("bboxLeft", -6.0),
            ("bboxTop", 0.0),
            ("bboxRight", 6.0),
            ("bboxBottom", 8.0),
        ] {
            entity.data.insert(key.into(), value);
        }
    }
    if entity.object == "oMattockHead" {
        for (key, value) in [
            ("bboxLeft", -6.0),
            ("bboxTop", -4.0),
            ("bboxRight", 6.0),
            ("bboxBottom", 4.0),
        ] {
            entity.data.insert(key.into(), value);
        }
    }
    if entity.object == "oWhipPre" {
        entity.alarms[0] = 3;
    }
    collectibles::init_entity(entity);
    explosives::init_entity(entity);
    weapons::init_entity(entity);
    utility::init_entity(entity);
}

/// Early oPlayer1 Step work runs before its light-distance sampling and
/// character movement. Item use and ordinary item updates remain later.
pub(crate) fn pre_player_tick(game: &mut Game) {
    weapons::pre_tick(game);
}

pub(crate) fn handles(object: &str) -> bool {
    (is_item(object) && !matches!(object, "oBall" | "oBall2"))
        || weapons::handles(object)
        || explosives::handles(object)
        || utility::handles(object)
        || collectibles::handles(object)
        || matches!(object, "oWhip" | "oWhipPre")
}

/// Context-dependent but RNG-free Create fields for the selected terrain
/// children. A marker protects subsequent producer overrides on reentry.
pub(crate) fn initialize_terrain_child(entity: &mut Entity, has_spectacles: bool) {
    buried_create::initialize(entity, has_spectacles);
}

pub(crate) fn initialize_source_jungle_child(entity: &mut Entity, has_spectacles: bool) {
    buried_create::initialize_source_jungle(entity, has_spectacles);
}

pub(crate) fn create_instance(game: &mut Game, id: u64) {
    let Some(mut entity) = game.world.entity(id).cloned() else {
        return;
    };
    init_entity(&mut entity);
    if game.generation_revision == crate::GenerationRevision::SourceClassicV2 {
        initialize_source_jungle_child(&mut entity, game.player.has("Spectacles"));
    } else {
        initialize_terrain_child(&mut entity, game.player.has("Spectacles"));
    }
    if entity.object == "oDice" {
        initialize_dice(&mut game.rng, &mut entity);
    } else if entity.object == "oWebBall" {
        weapons::initialize_web_ball(game, &mut entity);
    }
    if let Some(slot) = game.world.entities.iter_mut().find(|e| e.id == id) {
        *slot = entity;
    }
    explosives::create_instance(game, id);
}

/// oPlayer1's item actions are one event, independent from item instance Steps.
pub(crate) fn player_step(game: &mut Game, input: &InputFrame) {
    tick_whip(game);
    weapons::attack_step(game);
    clear_whip_when_not_whipping(game);
    player_actions(game, input);
    weapons::player_step_with_release(game, input.attack_released);
}

pub(crate) fn player_hurt(game: &mut Game) {
    weapons::player_hurt_queries(game);
}

pub(crate) fn player_after_hurt(game: &mut Game) {
    if player_dead(game) || game.player.is_stunned() {
        release_held_after_hit(game);
    }
}

pub(crate) fn player_collectibles(game: &mut Game) {
    collectibles::player_step(game);
}

pub(crate) fn outside_room(game: &mut Game, id: u64) {
    weapons::outside_room(game, id);
}

pub(crate) fn step_instance(game: &mut Game, id: u64) {
    let Some(mut entity) = game.world.entity(id).cloned() else {
        return;
    };
    if entity.value("stashed") != 0.0 {
        return;
    }
    init_entity(&mut entity);
    if entity.object == "oDice" {
        // Dice replaces oItem Step. Its camera-gated theft check precedes
        // attachment/movement; its final rolling block runs even off camera.
        let body_active = utility::dice_begin_step(game, &mut entity);
        if body_active {
            if entity.held {
                align_held(game, &mut entity);
                entity.data.insert("depth".into(), 1.0);
            } else {
                if advance_item_body(game, &mut entity) {
                    entity.alive = false;
                }
                entity.data.insert(
                    "depth".into(),
                    if game.player.has("Spectacles") {
                        0.0
                    } else {
                        101.0
                    },
                );
            }
        }
        if let Some(slot) = game.world.entities.iter_mut().find(|e| e.id == id) {
            *slot = entity;
        }
        if body_active {
            crate::enemies::item_step_impacts(game, id);
        }
        utility::step_instance(game, id);
        return;
    }
    let fragile = matches!(entity.object.as_str(), "oJar" | "oSkull");
    if fragile {
        for name in ["colTop", "colLeft", "colRight", "colBot"] {
            entity.data.insert(name.into(), 0.0);
        }
    }
    // Jar and Skull replace the parent Step without action_inherited.
    if !fragile && crate::catalog::is_a(&entity.object, "oItem") {
        crate::economy::item_step(game, &mut entity);
    }
    if matches!(entity.object.as_str(), "oWhip" | "oWhipPre") {
        let direction = if entity.object == "oWhipPre" {
            -1.0
        } else {
            1.0
        };
        entity.pos = Vec2::new(
            game.player.pos.x + direction * f64::from(entity.facing) * 16.0,
            game.player.pos.y,
        );
    }
    if entity.held && (fragile || item_body_active(game, &entity)) {
        align_held(game, &mut entity);
    }
    let mut broken = false;
    if ordinary_body(&entity.object) {
        broken = advance_item_body(game, &mut entity);
    } else if collectibles::treasure_value(&entity.object).is_some() {
        advance_treasure_body(game, &mut entity);
    }
    if fragile {
        entity
            .data
            .insert("depth".into(), if entity.held { 1.0 } else { 100.0 });
    }
    if let Some(slot) = game.world.entities.iter_mut().find(|e| e.id == id) {
        *slot = entity;
    }
    if fragile {
        // Lava's explicit instance_destroy precedes the remaining own Step;
        // wall/ceiling/floor damage only sets the final destruction latch.
        let lava_destroy = game.world.entity(id).is_some_and(|e| {
            !e.held && lava_at(game, Rect::new(e.pos.x, e.pos.y - 5.0, 0.001, 0.001))
        });
        if lava_destroy {
            break_item(game, id);
        }
        // Run BOTH creature queries with the post-response velocity. The
        // receiver can continue its event after the explicit lava Destroy.
        let destroy = crate::enemies::fragile_item_step_impacts(game, id) || broken;
        if destroy {
            if game.world.entity(id).is_some_and(|e| e.held) {
                game.player.held = None;
                game.player.pickup_item_type.clear();
            }
            break_item(game, id);
        }
        if let Some(mut entity) = game.world.entity(id).cloned() {
            update_depth(game, &mut entity);
            if let Some(slot) = game.world.entities.iter_mut().find(|e| e.id == id) {
                *slot = entity;
            }
        }
        return;
    }
    if broken {
        break_item(game, id);
    }
    weapons::step_instance(game, id);
    explosives::step_instance(game, id);
    utility::step_instance(game, id);
    collectibles::step_instance(game, id);
    if let Some(e) = game.world.entity_mut(id) {
        if e.object == "oFishBone" {
            e.data
                .insert("rotation".into(), weapons::item_rotation(e.vel, false, 0.0));
        }
    }
    if game.world.entity(id).is_some_and(|e| {
        crate::catalog::is_a(&e.object, "oItem")
            && !matches!(e.object.as_str(), "oArrow" | "oFlare")
            && (item_body_active(game, e)
                || matches!(e.object.as_str(), "oJar" | "oSkull" | "oRope"))
    }) {
        crate::enemies::item_step_impacts(game, id);
    }
    explosives::fire_frog_bomb_own_step(game, id);
    if game
        .world
        .entity(id)
        .is_some_and(|e| e.object == "oGoldIdol" || e.object == "oCrystalSkull")
    {
        crate::traps::altar::idol_step(game, id);
    }
    let Some(mut entity) = game.world.entity(id).cloned() else {
        return;
    };
    update_depth(game, &mut entity);
    if entity.object == "oBow" {
        weapons::bow_step(game, &mut entity);
    }
    if let Some(slot) = game.world.entities.iter_mut().find(|e| e.id == id) {
        *slot = entity;
    }
}

pub(crate) fn builtin_motion(game: &mut Game, id: u64) {
    weapons::builtin_motion(game, id);
    utility::builtin_motion(game, id);
    collectibles::builtin_motion(game, id);
}

pub(crate) fn collision(game: &mut Game, id: u64, other: crate::dispatcher::Instance) {
    if game.world.entity(id).is_some_and(|entity| {
        crate::catalog::is_a(&entity.object, "oSkull")
            && crate::catalog::is_a(other.object(game), "oBullet")
            && crate::dispatcher::collision_owner(&entity.object, other.object(game))
                == Some("oSkull")
    }) {
        // Skull's own Bullet event is only instance_destroy(). Its inherited
        // Item cleanup and own debris callback leave the bullet unchanged.
        break_item(game, id);
        return;
    }
    if jar_collision(game, id, other) {
        return;
    }
    weapons::collision(game, id, other);
    explosives::collision(game, id, other);
    utility::collision(game, id, other);
    collectibles::collision(game, id, other);
}

/// Literal Jar own collision events. WhipPre is a separate family and has no
/// Jar event. The Whip body finishes before instance_destroy's own callback;
/// default breakPieces therefore produces a second debris/loot burst.
fn jar_collision(game: &mut Game, id: u64, other: crate::dispatcher::Instance) -> bool {
    let Some(jar) = game.world.entity(id).cloned() else {
        return false;
    };
    let object = other.object(game);
    if !crate::catalog::is_a(&jar.object, "oJar")
        || jar.value("break_done") != 0.0
        || crate::dispatcher::collision_owner(&jar.object, object) != Some("oJar")
    {
        return false;
    }
    if crate::catalog::is_a(object, "oWhip") {
        game.sound("sndBreak", jar.pos);
        game.spawn("oSmokePuff", jar.pos);
        for _ in 0..3 {
            let piece = game.spawn("oRubbleSmall", Vec2::new(jar.pos.x - 2.0, jar.pos.y - 2.0));
            let velocity = f64::from(1 + game.rng.range(3)) - f64::from(1 + game.rng.range(3));
            if let Some(piece) = game.world.entity_mut(piece) {
                piece.vel.x = velocity;
            }
        }
        spawn_jar_loot(game, jar.pos, false, false);
        if jar.held {
            // Source Jar/Whip explicitly clears both held references.
            game.player.held = None;
            game.player.pickup_item_type.clear();
        }
        break_item(game, id);
        true
    } else if crate::catalog::is_a(object, "oBullet") {
        // Bullet's own Jar collision is only instance_destroy(). It leaves
        // bullet lifetime to the bullet's own Step/collision declarations.
        break_item(game, id);
        true
    } else {
        false
    }
}

pub(crate) fn animation_end(game: &mut Game, id: u64) {
    weapons::animation_end(game, id);
    explosives::animation_end(game, id);
    utility::animation_end(game, id);
    collectibles::animation_end(game, id);
}

pub(crate) fn alarm(game: &mut Game, id: u64, alarm: u8) {
    if alarm == 0
        && game
            .world
            .entity(id)
            .is_some_and(|e| e.object == "oWhipPre")
    {
        game.world.entity_mut(id).unwrap().alive = false;
        return;
    }
    if alarm == 2 {
        if let Some(entity) = game.world.entity_mut(id) {
            if crate::catalog::is_a(&entity.object, "oItem") {
                entity.data.insert("safe".into(), 0.0);
                entity.data.insert("safe_ticks".into(), 0.0);
            }
        }
    }
    weapons::alarm(game, id, alarm);
    explosives::alarm(game, id, alarm);
    utility::alarm(game, id, alarm);
    collectibles::alarm(game, id, alarm);
}

pub(crate) fn end_instance(game: &mut Game, id: u64) {
    explosives::end_instance(game, id);
}

/// Compatibility entry point for subsystem fixtures; the live game dispatches
/// each source event separately in resource order.
pub fn tick(game: &mut Game, input: &InputFrame) {
    player_step(game, input);
    let ids: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && handles(&e.object))
        .map(|e| e.id)
        .collect();
    for id in ids {
        step_instance(game, id);
    }
}

fn player_actions(game: &mut Game, input: &InputFrame) {
    game.player
        .data
        .insert("attack_held".into(), f64::from(input.attack));
    if game
        .player
        .held
        .is_some_and(|id| game.world.entity(id).is_none())
    {
        game.player.held = None;
    }
    if player_dead(game)
        || game.player.is_stunned()
        || game.player.data.get("active").copied().unwrap_or(1.0) == 0.0
    {
        // Source action input is skipped; held release already preceded tumble
        // movement, and whip cancellation belongs to late Player Step1816.
    } else {
        let interacted = collectibles::interact(game, input);
        let inventory_selected = if game.is_level() && game.player.attack_timer == 0 {
            if input.cycle_pressed {
                explosives::cycle_inventory(game);
                true
            } else if input.rope_pressed && game.player.ropes > 0 {
                explosives::use_rope(game, input);
                true
            } else if input.bomb_pressed && game.player.bombs > 0 {
                if !weapons::arm_bomb_arrow(game) {
                    explosives::use_bomb(game, input);
                }
                true
            } else {
                false
            }
        } else {
            false
        };
        // Player Step1219 guards only the empty-hand attack onset. Its
        // following pickup and held scrUseItem branches have their own guards.
        let empty_attack =
            game.player.held.is_none() && input.attack_pressed && empty_attack_allowed(game);
        let character_input = InputFrame {
            attack_pressed: empty_attack && !interacted && !inventory_selected,
            ..*input
        };
        crate::characters::tick(game, &character_input);
        if !interacted && !inventory_selected && input.attack_pressed {
            if let Some(id) = game.player.held {
                use_held_item(game, id, input);
            } else if empty_attack {
                if !game.flags.contains("character_tunnelman") {
                    start_whip(game);
                }
            } else if input.down {
                pick_up(game);
            }
        }
    }
}

/// Literal scrPlayerIsDucking predicate. Whipping preserves attack art even
/// with Down held, so platform state does not replace this sprite test.
pub(crate) fn player_is_ducking(player: &Player) -> bool {
    matches!(
        player.sprite.as_str(),
        "sDuckLeft"
            | "sCrawlLeft"
            | "sDamselDuckL"
            | "sDamselCrawlL"
            | "sTunnelDuckL"
            | "sTunnelCrawlL"
    )
}

fn empty_attack_allowed(game: &Game) -> bool {
    game.player.attack_timer == 0
        && game.player.state != PlayerState::Ducking
        && game.player.data.get("duck_to_hang").copied().unwrap_or(0.0) == 0.0
        && !matches!(game.player.sprite.as_str(), "sPExit" | "sDamselExit")
}

fn use_held_item(game: &mut Game, id: u64, input: &InputFrame) {
    let Some(item) = game.world.entity(id) else {
        return;
    };
    let object = item.object.clone();
    // scrUseItem's sprite-specific reserved objects precede its type branches.
    if item.sprite == "sBomb" {
        explosives::arm_bomb(game.world.entity_mut(id).unwrap());
        return;
    }
    if item.sprite == "sRopeEnd" {
        explosives::use_held_rope(game, id, input);
        return;
    }
    if matches!(object.as_str(), "oMachete" | "oMattock") && game.player.attack_timer > 0 {
        return;
    }
    if input.down && is_usable_weapon(&object) {
        if player_is_ducking(&game.player) {
            drop_held_weapon(game, id);
            return;
        }
        // Bow/Teleporter use else, while guns separately test non-ducking.
        if matches!(object.as_str(), "oBow" | "oTeleporter") {
            return;
        }
    }
    if !weapons::use_weapon(game, id, input) && !utility::use_utility(game, id, input) {
        throw_item(game, id, input);
    }
}

/// Specialized scrUseItem Down drops omit generic throw offsets, wall/Mitt
/// adjustments and sndThrow. Their own safe/alarm/velocity writes are literal.
fn drop_held_weapon(game: &mut Game, id: u64) {
    let facing = f64::from(game.player.facing);
    let player_vx = game.player.vel.x;
    if let Some(item) = game.world.entity_mut(id) {
        item.held = false;
        item.data.insert("safe".into(), 1.0);
        item.data.insert("safe_ticks".into(), 10.0);
        item.alarms[2] = 10;
        item.vel = Vec2::new(
            (facing * if item.value("heavy") != 0.0 { 4.0 } else { 8.0 } + player_vx) * 0.4,
            0.5,
        );
    }
    game.player.held = None;
    game.player.pickup_item_type.clear();
}

fn update_depth(game: &Game, entity: &mut Entity) {
    let spectacles = game.player.has("Spectacles");
    let treasure_visible = spectacles || game.player.has("UdjatEye");
    if !entity.alive {
        return;
    }
    let fragile = matches!(entity.object.as_str(), "oJar" | "oSkull");
    let mut depth = None;
    if collectibles::treasure_value(&entity.object).is_some() {
        if item_body_active(game, entity) {
            depth = Some(if treasure_visible { 0.0 } else { 101.0 });
        }
    } else if crate::catalog::is_a(&entity.object, "oItem")
        && (fragile || item_body_active(game, entity))
    {
        depth = Some(if entity.held {
            1.0
        } else if fragile {
            100.0
        } else if entity.object == "oDice" && spectacles {
            0.0
        } else if spectacles {
            51.0
        } else {
            101.0
        });
    }
    // oBomb's own Step follows the inherited parent and overrides it.
    if entity.object == "oBomb" {
        if entity.sprite == "sBombArmed" {
            depth = Some(49.0);
        }
        if entity.value("sticky") != 0.0 {
            depth = Some(1.0);
        }
    }
    if let Some(depth) = depth {
        entity.data.insert("depth".into(), depth);
    }
}

pub(super) fn player_dead(game: &Game) -> bool {
    game.player.state == PlayerState::Dead || game.state == RunState::Dead
}

pub(crate) fn cancel_player_attack_after_hurt(game: &mut Game) {
    if game.player.attack_timer == 0 {
        return;
    }
    // oPlayer1's dead/stunned branch clears whipping and destroys oWhip
    // (including oSlash/oMattockHit descendants) before item updates.
    game.player.attack_timer = 0;
    game.player.data.remove("whip_elapsed");
    game.player.data.remove("weapon_melee_age");
    game.player.data.remove("character_attack_active");
    game.player.data.remove("character_attack_elapsed");
    for entity in &mut game.world.entities {
        if entity.alive && entity.engine_active && crate::catalog::is_a(&entity.object, "oWhip") {
            entity.alive = false;
        }
    }
}

/// Source Step343-360 releases an already-stunned player's held item before
/// tumble movement, without the late hit branch's velocity or armed writes.
pub(crate) fn release_held_before_stunned_motion(game: &mut Game) {
    weapons::fire_bow_on_drop(game);
    if let Some(id) = game.player.held.take() {
        let released_type = game
            .world
            .entity(id)
            .map(explosives::source_type)
            .unwrap_or_default();
        if let Some(entity) = game.world.entity_mut(id) {
            entity.held = false;
            entity.data.insert("visible".into(), 1.0);
            entity.data.insert("hidden".into(), 0.0);
        }
        if released_type != game.player.pickup_item_type {
            explosives::restore_stashed_item(game);
        } else {
            game.player.pickup_item_type.clear();
        }
    }
}

/// The late oPlayer1 Step release after a projectile hit differs from its
/// early already-stunned release: the newly dropped item inherits knockback.
pub(super) fn release_held_after_hit(game: &mut Game) {
    let velocity = Vec2::new(game.player.vel.x, -6.0);
    let Some(id) = drop_held_with_velocity(game, velocity) else {
        return;
    };
    if let Some(entity) = game.world.entity_mut(id) {
        entity.data.insert("armed".into(), 1.0);
        if entity.object == "oDamsel" {
            entity.state = 2;
        }
    }
}

/// scrDropItem's explicit velocity release, also used by enemies that throw
/// the player. Unlike the player's late damage branch this does not arm items.
pub fn drop_held_with_velocity(game: &mut Game, velocity: Vec2) -> Option<u64> {
    weapons::fire_bow_on_drop(game);
    let id = game.player.held.take()?;
    let dropped_type = game
        .world
        .entity(id)
        .map(explosives::source_type)
        .unwrap_or_default();
    if let Some(entity) = game.world.entity_mut(id) {
        entity.held = false;
        entity.vel = velocity;
    }
    if dropped_type != game.player.pickup_item_type {
        explosives::restore_stashed_item(game);
    } else {
        game.player.pickup_item_type.clear();
    }
    Some(id)
}

fn is_usable_weapon(name: &str) -> bool {
    matches!(
        name,
        "oMachete"
            | "oMattock"
            | "oShotgun"
            | "oBow"
            | "oPistol"
            | "oSceptre"
            | "oWebCannon"
            | "oTeleporter"
    )
}

fn pick_up(game: &mut Game) {
    let area = Rect::new(game.player.pos.x - 8.0, game.player.pos.y, 17.0, 9.0);
    crate::dispatcher::register_instances(game);
    let items: Vec<_> = crate::dispatcher::descendant_instances(game, "oItem")
        .into_iter()
        .filter_map(|instance| match instance {
            crate::dispatcher::Instance::Entity(id) => game.world.entity(id),
            _ => None,
        })
        .filter(|entity| entity.engine_active)
        .collect();
    // The rectangle gates the attempt; instance_nearest then searches the
    // complete active parent array before eligibility checks its one result.
    if !items
        .iter()
        .any(|entity| entity.mask_intersects(area, CollisionPrecision::BoundingBox))
    {
        // Source's Enemy query is an else-if: any overlapping Item prevents
        // this fallback, even when that branch's nearest Item is ineligible.
        let enemies: Vec<_> = crate::dispatcher::descendant_instances(game, "oEnemy")
            .into_iter()
            .filter_map(|instance| match instance {
                crate::dispatcher::Instance::Entity(id) => game.world.entity(id),
                _ => None,
            })
            .filter(|entity| entity.engine_active)
            .collect();
        if enemies
            .iter()
            .any(|entity| entity.mask_intersects(area, CollisionPrecision::BoundingBox))
        {
            let nearest = enemies
                .into_iter()
                .fold(None, |nearest: Option<&Entity>, entity| {
                    if nearest.is_none_or(|current| {
                        entity.pos.distance(game.player.pos) < current.pos.distance(game.player.pos)
                    }) {
                        Some(entity)
                    } else {
                        nearest
                    }
                })
                .filter(|entity| entity.state >= 98 && entity.value("canPickUp") != 0.0)
                .map(|entity| entity.id);
            if let Some(id) = nearest {
                game.world.entity_mut(id).unwrap().held = true;
                game.player.held = Some(id);
                game.player.pickup_item_type =
                    explosives::source_type(game.world.entity(id).unwrap());
                explosives::pickup_pose(game);
            }
        }
        return;
    }
    let nearest = items
        .into_iter()
        .fold(None, |nearest: Option<&Entity>, entity| {
            if nearest.is_none_or(|current| {
                entity.pos.distance(game.player.pos) < current.pos.distance(game.player.pos)
            }) {
                Some(entity)
            } else {
                nearest
            }
        });
    let nearest = nearest
        .filter(|entity| {
            entity.value("canPickUp") != 0.0
                && !game
                    .world
                    .solid_at_except(entity.pos.x, entity.pos.y, entity.id)
        })
        .map(|entity| entity.id);
    if let Some(id) = nearest {
        let entity = game.world.entity_mut(id).unwrap();
        entity.held = true;
        let new_bow = entity
            .data
            .get("new")
            .copied()
            .unwrap_or_else(|| f64::from(entity.value("bow_picked_up") == 0.0));
        if entity.object == "oBow" && new_bow != 0.0 {
            entity.data.insert("new".into(), 0.0);
            entity.data.insert("bow_picked_up".into(), 1.0);
            *game.data.entry("arrows".into()).or_default() += 6.0;
        }
        game.player.pickup_item_type = explosives::source_type(entity);
        game.player.held = Some(id);
        explosives::pickup_pose(game);
        utility::on_pickup(game, id);
        crate::traps::altar::idol_pickup(game, id);
        // Source scrStealItem consumes equipment only after this single
        // parent-item selection has made it the held item.
        if game
            .world
            .entity(id)
            .is_some_and(|item| item.value("cost") == 0.0)
        {
            collectibles::grant(game, id);
        }
    }
}

fn throw_item(game: &mut Game, id: u64, input: &InputFrame) {
    let facing = f64::from(game.player.facing);
    let pos = game.player.pos;
    let velocity = game.player.vel;
    let grounded = game.player.grounded;
    let mitt = game.player.has("Mitt");
    let ducking = player_is_ducking(&game.player);
    let throwing_bomb = game.world.entity(id).is_some_and(|e| e.object == "oBomb");
    let overhead = game
        .world
        .solid_at_except(pos.x + facing * 8.0, pos.y - 10.0, id);
    let wall = game.world.solid_at_except(pos.x + facing * 8.0, pos.y, id);
    if let Some(entity) = game.world.entity_mut(id) {
        let heavy = entity.value("heavy") != 0.0 || entity.object == "oDamsel";
        if entity.object == "oDamsel" {
            entity.state = 2;
            entity.timer = 120;
            entity.pos.y -= 4.0;
        }
        entity.held = false;
        entity.vel.x = facing * if heavy { 4.0 } else { 8.0 } + velocity.x;
        entity.vel.y = if input.up {
            if heavy { -4.0 } else { -9.0 }
        } else if heavy {
            -2.0
        } else {
            -3.0
        };
        entity.data.insert("safe".into(), 1.0);
        entity.data.insert("safe_ticks".into(), 10.0);
        entity.alarms[2] = 10;
        if entity.object != "oFireFrogBomb" {
            entity.data.insert("armed".into(), 1.0);
        }
        entity.data.insert("owner".into(), 1.0);
        if wall {
            entity.pos.x -= facing * 8.0;
        }
        if input.down {
            if grounded {
                entity.pos.y -= 2.0;
                entity.vel.x *= if matches!(
                    entity.object.as_str(),
                    "oMachete"
                        | "oMattock"
                        | "oShotgun"
                        | "oBow"
                        | "oPistol"
                        | "oSceptre"
                        | "oWebCannon"
                        | "oTeleporter"
                ) {
                    0.4
                } else {
                    0.6
                };
                entity.vel.y = 0.5;
            } else {
                entity.vel.y = 3.0;
            }
        }
        if !mitt && !input.down && overhead {
            entity.vel.y = 0.0;
            entity.vel.x += facing;
        }
        if mitt && !ducking {
            entity.vel.x += if entity.vel.x < 0.0 { -6.0 } else { 6.0 };
            if !input.up && !input.down {
                entity.vel.y = -0.4;
            } else if input.down {
                entity.vel.y = 6.0;
            }
            entity.data.insert("gravity".into(), 0.1);
        }
    }
    game.player.held = None;
    if throwing_bomb {
        explosives::restore_stashed_item(game);
    } else {
        game.player.pickup_item_type.clear();
    }
    game.sound("sndThrow", pos);
}

fn align_held(game: &Game, entity: &mut Entity) {
    let pos = game.player.pos;
    let facing = game.player.facing;
    let ducking = game.player.state == PlayerState::Ducking && game.player.vel.x.abs() < 2.0;
    entity.held = true;
    // Their replacement held branch changes position/depth only. Retained
    // custom velocity still participates in the following creature queries.
    if !matches!(entity.object.as_str(), "oJar" | "oSkull") {
        entity.vel = Vec2::default();
    }
    let heavy = entity.value("heavy") != 0.0;
    let large = matches!(
        entity.object.as_str(),
        "oGoldIdol" | "oCrystalSkull" | "oLampItem" | "oLampRedItem" | "oDamsel"
    );
    let y_offset = if entity.object == "oDice" {
        if ducking { 0.0 } else { -2.0 }
    } else if large {
        if ducking { 2.0 } else { 0.0 }
    } else if heavy {
        if ducking { -2.0 } else { -4.0 }
    } else if ducking {
        4.0
    } else if matches!(entity.object.as_str(), "oJar" | "oSkull") {
        0.0
    } else {
        2.0
    };
    entity.pos = Vec2::new(pos.x + f64::from(facing) * 4.0, pos.y + y_offset);
    entity.facing = facing;
}

fn start_whip(game: &mut Game) {
    game.player.attack_timer = 12;
    game.player
        .set_sprite(if game.flags.contains("character_damsel") {
            "sDamselAttackL"
        } else {
            "sAttackLeft"
        });
    game.player.animation = 0.0;
    game.player.set_image_speed(0.6);
}

fn spawn_whip_part(game: &mut Game, pre: bool, tunnelman: bool) {
    let direction = if pre { -1.0 } else { 1.0 };
    let facing = game.player.facing;
    let object = match (tunnelman, pre) {
        (true, true) => "oMattockPre",
        (true, false) => "oMattockHit",
        (false, true) => "oWhipPre",
        (false, false) => "oWhip",
    };
    let id = game.spawn(
        object,
        Vec2::new(
            game.player.pos.x + direction * f64::from(facing) * 16.0,
            game.player.pos.y,
        ),
    );
    let entity = game.world.entity_mut(id).unwrap();
    init_entity(entity);
    entity.facing = facing;
    entity
        .data
        .insert("damage".into(), if tunnelman { 2.0 } else { 1.0 });
    entity.data.insert("owner".into(), 1.0);
    if tunnelman {
        entity.data.insert("owner_player".into(), 1.0);
        entity.data.insert("character_mattock".into(), 1.0);
        entity.timer = if pre { 3 } else { 5 };
        if !pre {
            entity.data.insert("puncture".into(), 1.0);
        }
    }
    if tunnelman {
        entity.set_sprite(&format!(
            "sMattock{}{}",
            if pre { "Pre" } else { "Hit" },
            if facing < 0 { "L" } else { "R" }
        ));
    } else {
        entity.set_sprite(if pre {
            if facing < 0 { "sWhipPreL" } else { "sWhipPreR" }
        } else if facing < 0 {
            "sWhipLeft"
        } else {
            "sWhipRight"
        });
    }
    if !pre {
        game.sound("sndWhip", game.player.pos);
    }
}

fn tick_whip(game: &mut Game) {
    if game.player.held.is_some()
        || !matches!(
            game.player.sprite.as_str(),
            "sAttackLeft" | "sDamselAttackL" | "sTunnelAttackL"
        )
    {
        return;
    }
    if game.player.animation > 4.0
        && !game.world.entities.iter().any(|entity| {
            entity.alive && entity.engine_active && crate::catalog::is_a(&entity.object, "oWhip")
        })
    {
        spawn_whip_part(game, false, game.flags.contains("character_tunnelman"));
    }
    // Literal source Step560-574 chain: the Tunnel sprite's own pre test
    // precedes the generic attack-sprite fallback, including the Tunnel art.
    if game.player.sprite == "sTunnelAttackL"
        && game.player.animation < 2.0
        && !game.world.entities.iter().any(|entity| {
            entity.alive
                && entity.engine_active
                && crate::catalog::is_a(&entity.object, "oMattockPre")
        })
    {
        spawn_whip_part(game, true, true);
    } else if game.player.animation < 2.0
        && !game.world.entities.iter().any(|entity| {
            entity.alive && entity.engine_active && crate::catalog::is_a(&entity.object, "oWhipPre")
        })
    {
        spawn_whip_part(game, true, false);
    }
}

fn clear_whip_when_not_whipping(game: &mut Game) {
    // Source Step577 follows all pre/forward birth queries. They use sprite,
    // not whipping, so even a subsequently destroyed birth is observable.
    if game.player.attack_timer == 0 {
        for entity in &mut game.world.entities {
            if entity.alive
                && entity.engine_active
                && (crate::catalog::is_a(&entity.object, "oWhip")
                    || crate::catalog::is_a(&entity.object, "oWhipPre"))
            {
                entity.alive = false;
            }
        }
    }
}

fn ordinary_body(name: &str) -> bool {
    is_item(name)
        && collectibles::treasure_value(name).is_none()
        && !matches!(
            name,
            "oBomb"
                | "oFireFrogBomb"
                | "oRope"
                | "oRopeThrow"
                | "oArrow"
                | "oFlare"
                | "oBall"
                | "oBall2"
        )
}

pub(crate) fn item_body_active(game: &Game, entity: &Entity) -> bool {
    entity.pos.x > game.camera.x - 16.0
        && entity.pos.x < game.camera.x + VIEW_WIDTH + 16.0
        && entity.pos.y > game.camera.y - 16.0
        && entity.pos.y < game.camera.y + VIEW_HEIGHT + 16.0
}

fn body_bounds(entity: &Entity, pos: Vec2) -> Rect {
    if entity.data.contains_key("bboxLeft") {
        return Rect::new(
            pos.x + entity.value("bboxLeft"),
            pos.y + entity.value("bboxTop"),
            entity.value("bboxRight") - entity.value("bboxLeft"),
            entity.value("bboxBottom") - entity.value("bboxTop"),
        );
    }
    if matches!(
        entity.object.as_str(),
        "oRock" | "oSkull" | "oFishBone" | "oGoldIdol" | "oCrystalSkull"
    ) {
        return Rect::new(pos.x - 4.0, pos.y - 4.0, 8.0, 8.0);
    }
    if entity.object == "oJar" {
        return Rect::new(pos.x - 4.0, pos.y - 6.0, 8.0, 12.0);
    }
    let bounds = entity.bounds();
    Rect::new(
        bounds.x + pos.x - entity.pos.x,
        bounds.y + pos.y - entity.pos.y,
        bounds.w,
        bounds.h,
    )
}

/// Classic moveTo reciprocal/modulo quantization, shared by item bodies.
pub(super) fn displacement(velocity: f64, tick: u64) -> i32 {
    let magnitude = velocity.abs();
    let fraction = magnitude.fract();
    let period = if fraction > 0.0 {
        (1.0 / fraction).round_ties_even() as u64
    } else {
        0
    };
    (magnitude.floor() as i32 + i32::from(period > 0 && tick.is_multiple_of(period)))
        * if velocity < 0.0 { -1 } else { 1 }
}

fn lava_at(game: &Game, rect: Rect) -> bool {
    let x0 = (rect.x / 16.0).floor() as i32;
    let y0 = (rect.y / 16.0).floor() as i32;
    let x1 = ((rect.x + rect.w - 0.0001) / 16.0).floor() as i32;
    let y1 = ((rect.y + rect.h - 0.0001) / 16.0).floor() as i32;
    (y0..=y1).any(|y| (x0..=x1).any(|x| game.world.tile(x, y).is_some_and(|t| t.object == "oLava")))
        || game
            .world
            .entities
            .iter()
            .any(|e| e.alive && e.object == "oLava" && rect.intersects(e.bounds()))
}

/// Collision edge scripts pass precise=true to collision_line. Keep their
/// target-mask query separate from explicit source bounding-box rectangles.
fn solid_edge(world: &World, probe: Rect, excluded: u64) -> bool {
    world.tiles_collide_precision(probe, "oSolid", CollisionPrecision::Precise)
        || world.entities.iter().any(|target| {
            target.id != excluded
                && target.alive
                && target.solid
                && !target.held
                && target.mask_intersects(probe, CollisionPrecision::Precise)
        })
}

fn body_contact(world: &World, entity: &Entity, dx: i8, dy: i8) -> bool {
    let bounds = body_bounds(entity, entity.pos);
    let probe = if dx < 0 {
        Rect::new(
            (bounds.x - 1.0).round_ties_even(),
            bounds.y.round_ties_even(),
            1.0,
            bounds.h,
        )
    } else if dx > 0 {
        Rect::new(
            (bounds.x + bounds.w).round_ties_even(),
            bounds.y.round_ties_even(),
            1.0,
            bounds.h,
        )
    } else if dy < 0 {
        Rect::new(
            bounds.x.round_ties_even(),
            (bounds.y - 1.0).round_ties_even(),
            bounds.w,
            1.0,
        )
    } else {
        Rect::new(
            bounds.x.round_ties_even(),
            (bounds.y + bounds.h).round_ties_even(),
            bounds.w,
            1.0,
        )
    };
    solid_edge(world, probe, entity.id)
}

fn body_horizontal_move_contact(world: &World, entity: &Entity, dx: i8) -> bool {
    // moveTo calls getIdCollisionLeft/Right, whose line starts at tb+5.
    // The post-movement isCollisionLeft/Right checks use the full height.
    let bounds = body_bounds(entity, entity.pos);
    let x = if dx < 0 {
        bounds.x - 1.0
    } else {
        bounds.x + bounds.w
    };
    let top = (bounds.y + 5.0).round_ties_even();
    let bottom = (bounds.y + bounds.h - 1.0).round_ties_even();
    solid_edge(
        world,
        Rect::new(
            x.round_ties_even(),
            top.min(bottom),
            1.0,
            (bottom - top).abs() + 1.0,
        ),
        entity.id,
    )
}

/// Run one inherited item body without advancing other world instances.
/// Used by scripted actors whose source Step explicitly calls oItem Step.
pub(crate) fn tick_single_item_body(game: &mut Game, entity: &mut Entity) {
    init_entity(entity);
    if advance_item_body(game, entity) {
        entity.alive = false;
    }
}

/// Full inherited oItem Step for curse actors owned by the trap dispatcher.
pub(crate) fn tick_inherited_ball_body(game: &mut Game, entity: &mut Entity) {
    init_entity(entity);
    if !item_body_active(game, entity) {
        return;
    }
    crate::economy::item_step(game, entity);
    if entity.held {
        align_held(game, entity);
    }
    if advance_item_body(game, entity) {
        entity.alive = false;
    }
    let id = entity.id;
    let existing = game.world.entities.iter().position(|e| e.id == id);
    if let Some(index) = existing {
        game.world.entities[index] = entity.clone();
    } else {
        game.world.entities.push(entity.clone());
    }
    crate::enemies::item_step_impacts(game, id);
    if let Some(index) = game.world.entities.iter().position(|e| e.id == id) {
        *entity = game.world.entities[index].clone();
        if existing.is_none() {
            game.world.entities.remove(index);
        }
    }
}

fn advance_item_body(game: &mut Game, entity: &mut Entity) -> bool {
    if !entity.alive || entity.held {
        return false;
    }
    let fragile = matches!(entity.object.as_str(), "oJar" | "oSkull");
    let dice = entity.object == "oDice";
    let overrides_parent = fragile || dice;
    if !fragile && !item_body_active(game, entity) {
        return false;
    }
    let velocity = entity.vel;
    let mut destroy = entity.value("broken") > 0.0;
    // moveTo resolves the complete quantized horizontal displacement first.
    let dx = displacement(velocity.x, game.engine_time());
    let dy = displacement(velocity.y, game.engine_time());
    let mut hit_x = false;
    let mut hit_y = false;
    let embedded = game
        .world
        .solid_at_except(entity.pos.x, entity.pos.y, entity.id);
    if embedded && !overrides_parent {
        let left = body_contact(&game.world, entity, -1, 0);
        let right = body_contact(&game.world, entity, 1, 0);
        let bottom = body_contact(&game.world, entity, 0, 1);
        let top = body_contact(&game.world, entity, 0, -1);
        // oItem Step refreshes every contact flag before its embedded escape.
        // Child Steps consume this snapshot even when the escape moves us.
        for (name, value) in [
            ("colLeft", left),
            ("colRight", right),
            ("colBot", bottom),
            ("colTop", top),
        ] {
            entity.data.insert(name.into(), f64::from(value));
        }
        if top && !bottom {
            entity.pos.y += 1.0;
        } else if left && !right {
            entity.pos.x += 1.0;
        } else if right && !left {
            entity.pos.x -= 1.0;
        } else {
            entity.vel = Vec2::default();
        }
        // The source's embedded escape is the final `else` of oItem Step;
        // ordinary movement, gravity, bounces and lava run only outside it.
        return false;
    } else {
        for _ in 0..dx.unsigned_abs() {
            if body_horizontal_move_contact(&game.world, entity, dx.signum() as i8) {
                hit_x = true;
                break;
            }
            entity.pos.x += f64::from(dx.signum());
        }
        for _ in 0..dy.unsigned_abs() {
            if body_contact(&game.world, entity, 0, dy.signum() as i8) {
                hit_y = true;
                break;
            }
            entity.pos.y += f64::from(dy.signum());
        }
    }
    if entity.object == "oJar" && embedded {
        destroy = true;
    }
    let left = body_contact(&game.world, entity, -1, 0);
    let right = body_contact(&game.world, entity, 1, 0);
    let bottom = body_contact(&game.world, entity, 0, 1);
    let top = body_contact(&game.world, entity, 0, -1);
    {
        for (name, value) in [
            ("colLeft", left),
            ("colRight", right),
            ("colTop", top),
            ("colBot", bottom),
        ] {
            entity.data.insert(name.into(), f64::from(value));
        }
    }
    hit_x |= left || right;
    if fragile {
        // Jar and skull override oItem: gravity precedes collision response.
        if entity.vel.y < 6.0 {
            entity.vel.y += entity.value("gravity");
        }
    } else if dice {
        if !bottom && entity.vel.y < 6.0 {
            entity.vel.y += entity.value("gravity");
        }
    } else {
        if !bottom {
            entity.vel.y += entity.value("gravity");
        }
        entity.vel.y = entity.vel.y.min(8.0);
    }
    if fragile && top && entity.vel.y < 0.0 {
        if entity.object == "oSkull" || entity.vel.y < -3.0 {
            destroy = true;
        }
        entity.vel.y = -entity.vel.y * 0.8;
    }
    if hit_x {
        if fragile && velocity.x.abs() > if entity.object == "oSkull" { 2.0 } else { 3.0 } {
            destroy = true;
        }
        entity.vel.x = -velocity.x * 0.5;
        if !overrides_parent {
            entity.data.insert("gravity".into(), 0.6);
        }
    }
    if bottom || (hit_y && velocity.y > 0.0) {
        if fragile && entity.vel.y > 3.0 {
            destroy = true;
        }
        entity.vel.y = if entity.vel.y > 1.0 {
            -entity.vel.y * entity.value("bounceFactor")
        } else {
            0.0
        };
        entity.vel.x = if entity.vel.x.abs() < 0.1 {
            0.0
        } else {
            entity.vel.x * entity.value("frictionFactor")
        };
        if !overrides_parent {
            entity.data.insert("gravity".into(), 0.6);
        }
        if !fragile && entity.vel.y.abs() < 1.0 {
            entity.pos.y -= 1.0;
            if !body_contact(&game.world, entity, 0, 1) {
                entity.pos.y += 1.0;
            }
            entity.vel.y = 0.0;
        }
    }
    if left && !right {
        entity.pos.x += 1.0;
        if fragile {
            entity.vel.y = 0.0;
        }
    } else if right {
        entity.pos.x -= 1.0;
        if fragile {
            entity.vel.y = 0.0;
        }
    }
    if !fragile && body_contact(&game.world, entity, 0, -1) {
        if entity.vel.y < 0.0 {
            entity.vel.y = -entity.vel.y * 0.8;
        } else {
            entity.pos.y += 1.0;
        }
        if !dice {
            entity.data.insert("gravity".into(), 0.6);
        }
    }
    if fragile && entity.vel.y.abs() < 1.0 {
        let bounds = body_bounds(entity, entity.pos);
        if solid_edge(
            &game.world,
            Rect::new(
                bounds.x.round_ties_even(),
                (bounds.y + bounds.h - 1.0).round_ties_even(),
                bounds.w,
                1.0,
            ),
            entity.id,
        ) {
            entity.pos.y -= 1.0;
            entity.vel.y = 0.0;
        }
    }
    if lava_at(
        game,
        Rect::new(entity.pos.x - 3.0, entity.pos.y - 3.0, 7.0, 7.0),
    ) {
        entity.data.insert("gravity".into(), 0.0);
        entity.vel = Vec2::default();
        entity.pos.y += 0.05;
    } else if !overrides_parent {
        // oItem restores normal gravity after the movement response. A
        // mitt changes this tick's acceleration; it is not permanent.
        entity.data.insert("gravity".into(), 0.6);
    }
    if lava_at(
        game,
        Rect::new(entity.pos.x, entity.pos.y - 5.0, 0.001, 0.001),
    ) && entity.object != "oSceptre"
    {
        destroy = true;
    }
    destroy
}

#[cfg(test)]
fn tick_bodies(game: &mut Game) {
    tick_treasures(game);
    let mut broken = Vec::new();
    for index in 0..game.world.entities.len() {
        let mut entity = game.world.entities[index].clone();
        if !entity.alive || entity.held || !ordinary_body(&entity.object) {
            continue;
        }
        if advance_item_body(game, &mut entity) {
            broken.push(entity.id);
        }
        game.world.entities[index] = entity;
    }
    for id in broken {
        break_item(game, id);
    }
    for entity in &mut game.world.entities {
        if entity.alive && entity.object == "oFishBone" {
            // oFishBone Step calls inherited before updating image_angle;
            // its own orientation runs even when parent body motion is gated.
            entity.data.insert(
                "rotation".into(),
                weapons::item_rotation(entity.vel, false, 0.0),
            );
        }
    }
}

fn advance_treasure_body(game: &mut Game, entity: &mut Entity) {
    if !entity.alive
        || collectibles::treasure_value(&entity.object).is_none()
        || entity.state != 1
        || entity.pos.x <= game.camera.x - 16.0
        || entity.pos.x >= game.camera.x + VIEW_WIDTH + 16.0
        || entity.pos.y <= game.camera.y - 16.0
        || entity.pos.y >= game.camera.y + VIEW_HEIGHT + 16.0
    {
        return;
    }
    let left = body_contact(&game.world, entity, -1, 0);
    let right = body_contact(&game.world, entity, 1, 0);
    let bottom = body_contact(&game.world, entity, 0, 1);
    let dx = displacement(entity.vel.x, game.engine_time());
    let dy = displacement(entity.vel.y, game.engine_time());
    for _ in 0..dx.unsigned_abs() {
        if body_horizontal_move_contact(&game.world, entity, dx.signum() as i8) {
            break;
        }
        entity.pos.x += f64::from(dx.signum());
    }
    for _ in 0..dy.unsigned_abs() {
        if body_contact(&game.world, entity, 0, dy.signum() as i8) {
            break;
        }
        entity.pos.y += f64::from(dy.signum());
    }
    if !bottom {
        entity.vel.y += entity.value("gravity");
    }
    entity.vel.y = entity.vel.y.min(8.0);
    if body_contact(&game.world, entity, 0, -1) {
        if entity.vel.y < 0.0 {
            entity.vel.y = -entity.vel.y * 0.8;
        } else {
            entity.pos.y += 1.0;
        }
    }
    if left || right {
        entity.vel.x *= -0.5;
    }
    if bottom {
        entity.vel.x = if entity.vel.x.abs() < 0.1 {
            0.0
        } else {
            entity.vel.x * 0.3
        };
        entity.pos.y -= 1.0;
        if !body_contact(&game.world, entity, 0, 1) {
            entity.pos.y += 1.0;
        }
        entity.vel.y = 0.0;
    }
    if left && !right {
        entity.pos.x += 1.0;
    } else if right {
        entity.pos.x -= 1.0;
    }
    if lava_at(
        game,
        Rect::new(entity.pos.x - 3.0, entity.pos.y - 3.0, 7.0, 7.0),
    ) {
        entity.data.insert("gravity".into(), 0.0);
        entity.vel = Vec2::default();
        entity.pos.y += 0.05;
    }
    if lava_at(
        game,
        Rect::new(entity.pos.x, entity.pos.y - 5.0, 0.001, 0.001),
    ) {
        entity.alive = false;
    }
}

#[cfg(test)]
fn tick_treasures(game: &mut Game) {
    for index in 0..game.world.entities.len() {
        let mut entity = game.world.entities[index].clone();
        advance_treasure_body(game, &mut entity);
        game.world.entities[index] = entity;
    }
}

/// Called by combat when a fragile thrown body breaks on a creature.
pub fn break_item(game: &mut Game, id: u64) {
    let Some(entity) = game.world.entities.iter_mut().find(|e| e.id == id) else {
        return;
    };
    if entity.value("break_done") != 0.0 {
        return;
    }
    entity.data.insert("break_done".into(), 1.0);
    entity.alive = false;
    let pos = entity.pos;
    let jar = crate::catalog::is_a(&entity.object, "oJar");
    let pieces = entity.data.get("breakPieces").copied().unwrap_or(1.0) != 0.0;
    let left = entity.value("colLeft") != 0.0;
    let right = entity.value("colRight") != 0.0;
    let top = entity.value("colTop") != 0.0;
    let skull = entity.object == "oSkull";
    if game.player.held == Some(id) {
        game.player.held = None;
    }
    if !pieces {
        return;
    }
    if jar || skull {
        game.sound("sndBreak", pos);
        game.spawn("oSmokePuff", pos);
        let mut index = 0;
        // Skull's source condition evaluates rand(1,2) on every loop test,
        // including the final failed comparison. Jar has a fixed three pieces.
        while index < if jar { 3 } else { 1 + game.rng.range(2) } {
            let object = if jar { "oRubbleSmall" } else { "oBone" };
            let id = game.spawn(object, Vec2::new(pos.x - 2.0, pos.y - 2.0));
            // Create may mark a capped detritus instance dead before returning;
            // the source caller still assigns its explicit launch fields.
            let mut piece = game
                .world
                .entities
                .iter()
                .find(|e| e.id == id)
                .unwrap()
                .clone();
            piece.vel.x = if left {
                f64::from(1 + game.rng.range(3))
            } else if right {
                -f64::from(1 + game.rng.range(3))
            } else {
                f64::from(1 + game.rng.range(3)) - f64::from(1 + game.rng.range(3))
            };
            piece.vel.y = f64::from(game.rng.range(4)) * if top { 1.0 } else { -1.0 };
            let slot = game.world.entities.iter_mut().find(|e| e.id == id).unwrap();
            *slot = piece;
            index += 1;
        }
    }
    if jar {
        spawn_jar_loot(game, pos, left, right);
    }
}

fn spawn_jar_loot(game: &mut Game, pos: Vec2, left: bool, right: bool) {
    let loot = if game.rng.chance(1, 3) {
        Some("oGoldChunk")
    } else if game.rng.chance(1, 6) {
        Some("oGoldNugget")
    } else if game.rng.chance(1, 12) {
        Some("oEmeraldBig")
    } else if game.rng.chance(1, 12) {
        Some("oSapphireBig")
    } else if game.rng.chance(1, 12) {
        Some("oRubyBig")
    } else if game.rng.chance(1, 6) {
        Some("oSpider")
    } else if game.rng.chance(1, 12) {
        Some("oSnake")
    } else {
        None
    };
    if let Some(name) = loot {
        let spawn = if name == "oSpider" {
            Vec2::new(pos.x - 8.0, pos.y - 8.0)
        } else if name == "oSnake" {
            Vec2::new(
                pos.x
                    - if left {
                        0.0
                    } else if right {
                        16.0
                    } else {
                        8.0
                    },
                pos.y - 8.0,
            )
        } else {
            pos
        };
        game.spawn(name, spawn);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn game() -> Game {
        Game::shell(7, RulesPreset::Classic)
    }
    fn pickup_query_fixture() -> Game {
        let mut game = game();
        game.world = World::new(80, 40, Area::Mines, 1);
        game.world.name = "fixture".into();
        for x in 0..game.world.width {
            game.world.set_tile(x, 5, "oBrick");
        }
        game.player.pos = Vec2::new(72.0, 72.0);
        game.player.previous_pos = game.player.pos;
        game.player.state = PlayerState::Standing;
        game.player.grounded = true;
        game
    }

    fn pickup_query_input() -> InputFrame {
        InputFrame {
            down: true,
            attack: true,
            attack_pressed: true,
            ..InputFrame::default()
        }
    }

    #[test]
    fn game_step_equipment_grant_follows_the_one_parent_pickup_selection() {
        for (rock_position, expected_cape) in [
            (Vec2::new(72.0, 64.0), false),
            (Vec2::new(96.0, 64.0), true),
        ] {
            let mut game = pickup_query_fixture();
            let rock = game.spawn("oRock", rock_position);
            let cape = game.spawn("oCapePickup", Vec2::new(80.0, 76.0));
            let pickup = game.world.entity_mut(cape).unwrap();
            pickup.data.insert("cost".into(), 0.0);
            pickup.data.insert("forSale".into(), 0.0);
            game.step(&pickup_query_input());
            assert_eq!(game.player.has("Cape"), expected_cape);
            assert_eq!(
                game.player.held,
                if expected_cape { None } else { Some(rock) }
            );
            assert_eq!(game.world.entity(cape).is_none(), expected_cape);
            assert_eq!(game.events.iter().filter(|event|matches!(event,GameEvent::Collected{object,..}if object=="oCapePickup")).count(),usize::from(expected_cape));
        }
    }

    #[test]
    fn game_step_nearer_unpickable_parent_item_blocks_equipment_grant() {
        let mut game = pickup_query_fixture();
        let rock = game.spawn("oRock", Vec2::new(72.0, 64.0));
        game.world
            .entity_mut(rock)
            .unwrap()
            .data
            .insert("canPickUp".into(), 0.0);
        let cape = game.spawn("oCapePickup", Vec2::new(80.0, 76.0));
        let pickup = game.world.entity_mut(cape).unwrap();
        pickup.data.insert("cost".into(), 0.0);
        pickup.data.insert("forSale".into(), 0.0);
        game.step(&pickup_query_input());
        assert_eq!(game.player.held, None);
        assert!(!game.player.has("Cape"));
        assert!(game.world.entity(cape).is_some());
    }

    #[test]
    fn game_step_pickup_rectangle_gates_nearest_all_active_parent_items() {
        let mut game = pickup_query_fixture();
        let nearest = game.spawn("oRock", Vec2::new(72.0, 64.0));
        let gate = game.spawn("oRock", Vec2::new(80.0, 76.0));
        let area = Rect::new(64.0, 72.0, 17.0, 9.0);
        assert!(
            !game
                .world
                .entity(nearest)
                .unwrap()
                .mask_intersects(area, CollisionPrecision::BoundingBox)
        );
        assert!(
            game.world
                .entity(gate)
                .unwrap()
                .mask_intersects(area, CollisionPrecision::BoundingBox)
        );
        game.step(&pickup_query_input());
        assert_eq!(game.player.held, Some(nearest));
        assert!(!game.world.entity(gate).unwrap().held);
    }

    #[test]
    fn game_step_pickup_checks_only_selected_nearest_eligibility() {
        for blocked_by_solid in [false, true] {
            let mut game = pickup_query_fixture();
            let nearest = game.spawn("oRock", Vec2::new(72.0, 64.0));
            game.spawn("oRock", Vec2::new(80.0, 77.0));
            if blocked_by_solid {
                // This solid contains the item point and lies above the
                // player's point-crush probe.
                game.world.set_tile(4, 3, "oBrick");
                game.world.entity_mut(nearest).unwrap().pos.y = 63.0;
            } else {
                game.world
                    .entity_mut(nearest)
                    .unwrap()
                    .data
                    .insert("canPickUp".into(), 0.0);
            }
            game.step(&pickup_query_input());
            assert_eq!(game.player.held, None, "solid={blocked_by_solid}");
        }
    }

    #[test]
    fn game_step_pickup_requires_contact_gate_and_uses_parent_array_ties() {
        let mut game = pickup_query_fixture();
        game.spawn("oRock", Vec2::new(72.0, 64.0));
        game.step(&pickup_query_input());
        assert_eq!(
            game.player.held, None,
            "nearest item alone does not satisfy the contact gate"
        );

        for first_object in ["oJar", "oRock"] {
            let mut game = pickup_query_fixture();
            let first = game.spawn(first_object, Vec2::new(76.0, 76.0));
            let second = game.spawn(
                if first_object == "oJar" {
                    "oRock"
                } else {
                    "oJar"
                },
                Vec2::new(76.0, 76.0),
            );
            game.world.entities.reverse();
            game.step(&pickup_query_input());
            assert_eq!(game.player.held, Some(first), "first={first_object}");
            assert!(!game.world.entity(second).unwrap().held);
        }
    }

    #[test]
    fn pick_up_requires_down_and_attack_and_throw_uses_mitt() {
        let mut game = game();
        game.player.pos = Vec2::new(40.0, 40.0);
        let id = game.world.spawn("oRock", Vec2::new(40.0, 44.0));
        init_entity(game.world.entity_mut(id).unwrap());
        pick_up(&mut game);
        assert_eq!(game.player.held, Some(id));
        game.player.equipment.insert("Mitt".into());
        throw_item(&mut game, id, &InputFrame::default());
        assert_eq!(game.player.held, None);
        let rock = game.world.entity(id).unwrap();
        assert_eq!(rock.vel, Vec2::new(14.0, -0.4));
        assert_eq!(rock.value("gravity"), 0.1);
    }
    #[test]
    fn game_step_mattock_head_inherits_motion_pickup_and_throw() {
        let mut game = game();
        game.player.pos = Vec2::new(200.0, 80.0);
        // Keep the original head trajectory clear, then support Player at
        // y78 for the later controlled reposition and Down pickup.
        game.spawn("oHardBlock", Vec2::new(96.0, 86.0));
        let id = game.world.spawn("oMattockHead", Vec2::new(100.0, 80.0));
        game.world.entity_mut(id).unwrap().vel = Vec2::new(2.0, -2.0);
        game.step(&InputFrame::default());
        let head = game.world.entity(id).unwrap();
        assert_eq!(head.pos, Vec2::new(102.0, 78.0));
        assert!((head.vel.y + 1.4).abs() < 1e-9);
        assert_eq!(head.value("canPickUp"), 1.0);
        assert_eq!(head.value("bboxLeft"), -6.0);
        assert_eq!(head.value("bboxRight"), 6.0);
        assert_eq!(head.value("bboxTop"), -4.0);
        assert_eq!(head.value("bboxBottom"), 4.0);

        game.player.pos = head.pos;
        game.player.previous_pos = game.player.pos;
        game.player.vel = Vec2::default();
        game.player.state = PlayerState::Standing;
        game.player.grounded = true;
        game.step(&InputFrame {
            down: true,
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert_eq!(game.player.state, PlayerState::Ducking);
        assert_eq!(game.player.held, Some(id));
        assert!(game.world.entity(id).unwrap().held);
        game.step(&InputFrame {
            attack_released: true,
            ..Default::default()
        });
        game.step(&InputFrame {
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert_eq!(game.player.held, None);
        let head = game.world.entity(id).unwrap();
        assert!(!head.held);
        assert!(head.vel.x > 0.0 && head.vel.y < 0.0);
        assert_eq!(head.value("armed"), 1.0);
    }
    #[test]
    fn game_step_mitt_throw_returns_to_source_item_gravity_after_first_tick() {
        let mut game = game();
        game.player.pos = Vec2::new(100.0, 100.0);
        game.player.equipment.insert("Mitt".into());
        let rock = game.world.spawn("oRock", game.player.pos);
        game.world.entity_mut(rock).unwrap().held = true;
        game.player.held = Some(rock);
        game.step(&InputFrame {
            attack_pressed: true,
            ..Default::default()
        });
        assert!((game.world.entity(rock).unwrap().vel.y + 0.3).abs() < 1e-9);
        assert_eq!(game.world.entity(rock).unwrap().value("gravity"), 0.6);
        game.step(&InputFrame::default());
        assert!((game.world.entity(rock).unwrap().vel.y - 0.3).abs() < 1e-9);
    }

    #[test]
    fn game_step_embedded_item_only_uses_source_escape_branch() {
        let mut game = game();
        game.player.pos = Vec2::new(200.0, 200.0);
        game.world.set_tile(6, 6, "oBlock");
        let rock = game.world.spawn("oRock", Vec2::new(104.0, 104.0));
        game.world.entity_mut(rock).unwrap().vel = Vec2::new(3.0, 3.0);
        game.step(&InputFrame::default());
        let rock = game.world.entity(rock).unwrap();
        assert_eq!(rock.pos, Vec2::new(104.0, 104.0));
        assert_eq!(rock.vel, Vec2::default());
    }

    #[test]
    fn source_edge_queries_use_darkfall_alpha_and_ignore_leaves_platforms() {
        for dynamic in [false, true] {
            let mut game = game();
            if dynamic {
                game.world.spawn("oDarkFall", Vec2::new(80.0, 80.0));
            } else {
                game.world.set_tile(5, 5, "oDarkFall");
            }
            // DarkFall's source-image pixel (0,0) is transparent inside its
            // AUTO mask rectangle, while adjacent pixel (1,0) is opaque.
            let clear = Rect::new(80.0, 80.0, 1.0, 1.0);
            assert!(game.world.collides_except(clear, 0), "{dynamic}");
            assert!(!solid_edge(&game.world, clear, 0), "{dynamic}");
            assert!(solid_edge(&game.world, Rect::new(81.0, 80.0, 1.0, 1.0), 0));

            let mut game = Game::shell(7, RulesPreset::Classic);
            if dynamic {
                game.world.spawn("oLeaves", Vec2::new(80.0, 80.0));
            } else {
                game.world.set_tile(5, 5, "oLeaves");
            }
            // Leaves inherits oPlatform, not oSolid: an item's oSolid edge
            // query ignores both its transparent and opaque source pixels.
            for x in [80.0, 85.0] {
                assert!(!solid_edge(&game.world, Rect::new(x, 95.0, 1.0, 1.0), 0));
            }
        }
    }

    #[test]
    fn game_step_item_passes_transparent_source_terrain_corners() {
        for (object, y, velocity, expected_y) in [
            ("oDarkFall", 76.0, 1.0, 77.0),
            ("oLeaves", 100.0, -1.0, 99.0),
        ] {
            for dynamic in [false, true] {
                let mut game = game();
                game.player.pos = Vec2::new(200.0, 200.0);
                if dynamic {
                    game.world.spawn(object, Vec2::new(80.0, 80.0));
                } else {
                    game.world.set_tile(5, 5, object);
                }
                let rock = game.world.spawn("oRock", Vec2::new(77.0, y));
                game.world.entity_mut(rock).unwrap().vel.y = velocity;
                game.step(&InputFrame::default());
                // Only x=80 of the item's top/bottom line overlaps the
                // terrain. DarkFall alpha permits one pixel; Leaves must
                // remain excluded from the oSolid family altogether.
                assert_eq!(
                    game.world.entity(rock).unwrap().pos.y,
                    expected_y,
                    "{object}/{dynamic}"
                );
            }
        }
    }

    #[test]
    fn game_step_item_moves_past_upper_corner_using_source_horizontal_strip() {
        let mut game = game();
        game.player.pos = Vec2::new(200.0, 200.0);
        game.world.set_tile(6, 5, "oBlock");
        let rock = game.world.spawn("oRock", Vec2::new(91.0, 98.0));
        game.world.entity_mut(rock).unwrap().vel = Vec2::new(2.0, 1.0);
        game.step(&InputFrame::default());
        // moveTo advances x by two; the subsequent full side probe backs off
        // one. Its fresh ceiling probe is then clear, so y remains 99.
        assert_eq!(game.world.entity(rock).unwrap().pos, Vec2::new(92.0, 99.0));
    }

    #[test]
    fn game_step_item_settles_out_of_one_pixel_floor_overlap() {
        for object in ["oRock", "oSkull", "oJar"] {
            let mut game = game();
            game.player.pos = Vec2::new(200.0, 200.0);
            for x in 0..game.world.width {
                game.world.set_tile(x, 4, "oBlock");
            }
            let y = if object == "oJar" { 59.0 } else { 61.0 };
            let item = game.world.spawn(object, Vec2::new(100.0, y));
            game.step(&InputFrame::default());
            assert_eq!(game.world.entity(item).unwrap().pos.y, y - 1.0, "{object}");
            assert_eq!(game.world.entity(item).unwrap().vel.y, 0.0, "{object}");
        }
    }

    #[test]
    fn game_step_treasure_uses_pre_move_floor_contact_without_item_bounce() {
        let mut game = game();
        for x in 0..game.world.width {
            game.world.set_tile(x, 4, "oBlock");
        }
        game.player.pos = Vec2::new(40.0, 56.0);
        let gold = game.world.spawn("oGoldNugget", Vec2::new(100.0, 54.0));
        game.world.entity_mut(gold).unwrap().vel.y = 6.0;
        game.step(&InputFrame::default());
        assert_eq!(game.world.entity(gold).unwrap().pos.y, 60.0);
        assert_eq!(game.world.entity(gold).unwrap().vel.y, 6.6);
        game.step(&InputFrame::default());
        assert_eq!(game.world.entity(gold).unwrap().pos.y, 60.0);
        assert_eq!(game.world.entity(gold).unwrap().vel.y, 0.0);
    }

    #[test]
    fn game_step_treasure_outside_view_keeps_source_position_and_velocity() {
        let mut game = game();
        let gold = game.world.spawn("oGoldNugget", Vec2::new(360.0, 100.0));
        game.world.entity_mut(gold).unwrap().vel = Vec2::new(1.0, 2.0);
        for _ in 0..3 {
            game.step(&InputFrame::default());
        }
        assert_eq!(
            game.world.entity(gold).unwrap().pos,
            Vec2::new(360.0, 100.0)
        );
        assert_eq!(game.world.entity(gold).unwrap().vel, Vec2::new(1.0, 2.0));
    }

    #[test]
    fn game_step_parent_item_freezes_outside_view_but_jar_override_keeps_moving() {
        let mut game = game();
        // Beyond oItem's view+16, within oLevel's active view+96 region.
        let rock = game.world.spawn("oRock", Vec2::new(360.0, 100.0));
        let jar = game.world.spawn("oJar", Vec2::new(380.0, 100.0));
        game.world.entity_mut(rock).unwrap().vel = Vec2::new(1.0, 2.0);
        game.world.entity_mut(jar).unwrap().vel = Vec2::new(1.0, 2.0);
        game.step(&InputFrame::default());
        assert_eq!(
            game.world.entity(rock).unwrap().pos,
            Vec2::new(360.0, 100.0)
        );
        assert_eq!(game.world.entity(rock).unwrap().vel, Vec2::new(1.0, 2.0));
        assert_eq!(game.world.entity(jar).unwrap().pos, Vec2::new(381.0, 102.0));
    }

    #[test]
    fn game_step_naturally_picks_up_living_damsel_but_not_exiting_or_living_enemy() {
        for (object, state, expected) in [
            ("oDamsel", 0, true),
            ("oDamsel", 4, false),
            ("oCaveman", 0, false),
        ] {
            let mut game = game();
            for x in 0..game.world.width {
                game.world.set_tile(x, 8, "oHardBlock");
            }
            game.player.pos = Vec2::new(100.0, 120.0);
            game.player.grounded = true;
            game.player.state = PlayerState::Standing;
            let id = game.world.spawn(object, Vec2::new(104.0, 120.0));
            if object == "oDamsel" && state == 4 {
                // Reach EXIT through its source event, including canPickUp=false.
                game.spawn("oExit", Vec2::new(96.0, 112.0));
                game.step(&InputFrame::default());
                assert_eq!(game.world.entity(id).unwrap().state, 4);
                assert_eq!(game.world.entity(id).unwrap().value("canPickUp"), 0.0);
            }
            game.step(&InputFrame {
                down: true,
                attack_pressed: true,
                ..Default::default()
            });
            assert_eq!(game.player.held, expected.then_some(id), "{object}/{state}");
        }
    }

    #[test]
    fn jar_fragments_consume_source_random_draws_before_loot() {
        let mut game = Game::shell(2, RulesPreset::Classic);
        let mut expected_rng = game.rng.clone();
        // Three unobstructed fragments consume three draws each. Seed 2 then
        // fails all seven loot branches: this was a Sapphire before debris.
        for _ in 0..16 {
            expected_rng.next_u64();
        }
        let jar = game.world.spawn("oJar", Vec2::new(100.0, 100.0));
        break_item(&mut game, jar);
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|e| e.object == "oSmokePuff")
                .count(),
            1
        );
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|e| e.object == "oRubbleSmall")
                .count(),
            3
        );
        assert!(
            game.world
                .entities
                .iter()
                .all(|e| matches!(e.object.as_str(), "oJar" | "oSmokePuff" | "oRubbleSmall"))
        );
        assert_eq!(game.rng.next_u64(), expected_rng.next_u64());
        let entity_count = game.world.entities.len();
        break_item(&mut game, jar);
        assert_eq!(game.world.entities.len(), entity_count);
    }

    #[test]
    fn skull_creation_precedes_launch_overrides_and_loop_condition_is_resampled() {
        let mut game = Game::shell(194, RulesPreset::Classic);
        game.player.pos = Vec2::new(200.0, 200.0);
        let mut expected_rng = game.rng.clone();
        // Canonical seed 194 gives loop bounds 1, 2, 1 at draws 1, 9, 17:
        // resampling runs two bodies even though the first bound was one.
        // Each body consumes four Bone Create draws before three launch draws.
        for _ in 0..17 {
            expected_rng.next_u64();
        }
        let skull = game.world.spawn("oSkull", Vec2::new(100.0, 100.0));
        break_item(&mut game, skull);
        let bones: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|e| e.object == "oBone")
            .collect();
        assert_eq!(bones.len(), 2);
        assert_eq!(bones[0].vel, Vec2::new(-2.0, -3.0));
        assert_eq!(bones[1].vel, Vec2::new(-1.0, -1.0));
        assert!(
            bones
                .iter()
                .all(|e| e.value("world_effect_initialized") == 1.0)
        );
        let ids = [bones[0].id, bones[1].id];
        assert_eq!(game.rng.state_bits(), 1_451_304_187);
        assert_eq!(game.rng.next_u64(), expected_rng.next_u64());
        game.step(&InputFrame::default());
        assert_eq!(
            game.world.entity(ids[0]).unwrap().pos,
            Vec2::new(96.0, 95.0)
        );
        assert_eq!(
            game.world.entity(ids[1]).unwrap().pos,
            Vec2::new(97.0, 97.0)
        );
    }

    #[test]
    fn game_step_floor_breaks_emit_source_jar_and_skull_fragments() {
        for object in ["oJar", "oSkull"] {
            let mut game = game();
            game.player.pos = Vec2::new(200.0, 200.0);
            for x in 0..game.world.width {
                game.world.set_tile(x, 6, "oHardBlock");
            }
            let id = game.world.spawn(object, Vec2::new(80.0, 80.0));
            game.world.entity_mut(id).unwrap().vel.y = 6.0;
            for _ in 0..3 {
                game.step(&InputFrame::default());
            }
            assert!(game.world.entity(id).is_none());
            assert_eq!(
                game.world
                    .entities
                    .iter()
                    .filter(|e| e.object == "oSmokePuff")
                    .count(),
                1
            );
            let fragment = if object == "oJar" {
                "oRubbleSmall"
            } else {
                "oBone"
            };
            let count = game
                .world
                .entities
                .iter()
                .filter(|e| e.object == fragment)
                .count();
            if object == "oJar" {
                assert_eq!(count, 3);
            } else {
                assert!((1..=2).contains(&count));
            }
        }
    }

    #[test]
    fn game_step_throwing_damsel_uses_source_thrown_state_and_origin_shift() {
        let mut game = game();
        for x in 0..game.world.width {
            game.world.set_tile(x, 8, "oBlock");
        }
        game.player.pos = Vec2::new(100.0, 120.0);
        game.player.grounded = true;
        game.player.state = PlayerState::Standing;
        let damsel = game.world.spawn("oDamsel", Vec2::new(104.0, 120.0));
        crate::enemies::init_entity(game.world.entity_mut(damsel).unwrap());
        game.world.entity_mut(damsel).unwrap().held = true;
        game.player.held = Some(damsel);
        game.step(&InputFrame {
            attack_pressed: true,
            ..Default::default()
        });
        let damsel = game.world.entity(damsel).unwrap();
        assert_eq!(game.player.held, None);
        assert_eq!(damsel.state, 2);
        assert_eq!(damsel.timer, 120);
        assert_eq!(damsel.pos.y, 114.0);
        assert_eq!(damsel.vel.x, 4.0);
    }

    #[test]
    fn game_step_dice_uses_own_soft_fall_cap_and_preserves_mitt_gravity() {
        let mut game = game();
        let die = game.world.spawn("oDice", Vec2::new(100.0, 60.0));
        game.world.entity_mut(die).unwrap().vel.y = 5.8;
        game.step(&InputFrame::default());
        assert!((game.world.entity(die).unwrap().vel.y - 6.4).abs() < 1e-9);
        game.step(&InputFrame::default());
        assert!((game.world.entity(die).unwrap().vel.y - 6.4).abs() < 1e-9);
        game.world.entity_mut(die).unwrap().vel.y = -2.0;
        game.world
            .entity_mut(die)
            .unwrap()
            .data
            .insert("gravity".into(), 0.1);
        game.step(&InputFrame::default());
        assert_eq!(game.world.entity(die).unwrap().value("gravity"), 0.1);
    }

    #[test]
    fn game_step_held_die_uses_own_standing_and_ducking_offsets() {
        let mut game = game();
        for x in 0..game.world.width {
            game.world.set_tile(x, 8, "oBlock");
        }
        game.player.pos = Vec2::new(100.0, 120.0);
        game.player.grounded = true;
        game.player.state = PlayerState::Standing;
        let die = game.world.spawn("oDice", game.player.pos);
        game.world.entity_mut(die).unwrap().held = true;
        game.player.held = Some(die);
        game.step(&InputFrame::default());
        assert_eq!(
            game.world.entity(die).unwrap().pos.y,
            game.player.pos.y - 2.0
        );
        game.step(&InputFrame {
            down: true,
            ..Default::default()
        });
        assert_eq!(game.world.entity(die).unwrap().pos.y, game.player.pos.y);
    }

    #[test]
    fn game_step_item_depths_follow_selected_family_and_equipment() {
        for spectacles in [false, true] {
            let mut game = game();
            if spectacles {
                game.player.equipment.insert("Spectacles".into());
            }
            for (index, (object, expected)) in [
                ("oRock", if spectacles { 51.0 } else { 101.0 }),
                ("oJar", 100.0),
                ("oSkull", 100.0),
                ("oGoldNugget", if spectacles { 0.0 } else { 101.0 }),
                ("oDice", if spectacles { 0.0 } else { 101.0 }),
            ]
            .into_iter()
            .enumerate()
            {
                let id = game
                    .world
                    .spawn(object, Vec2::new(100.0 + index as f64 * 24.0, 100.0));
                game.world
                    .entity_mut(id)
                    .unwrap()
                    .data
                    .insert("expected_depth".into(), expected);
            }
            let bomb = game.world.spawn("oBomb", Vec2::new(240.0, 100.0));
            explosives::arm_bomb(game.world.entity_mut(bomb).unwrap());
            let sticky = game.world.spawn("oBomb", Vec2::new(260.0, 100.0));
            explosives::arm_bomb(game.world.entity_mut(sticky).unwrap());
            game.world
                .entity_mut(sticky)
                .unwrap()
                .data
                .insert("sticky".into(), 1.0);
            let held = game.world.spawn("oRock", game.player.pos);
            game.world.entity_mut(held).unwrap().held = true;
            game.player.held = Some(held);
            game.step(&InputFrame::default());
            for entity in &game.world.entities {
                if let Some(expected) = entity.data.get("expected_depth") {
                    assert_eq!(entity.value("depth"), *expected, "{}", entity.object);
                }
            }
            assert_eq!(game.world.entity(bomb).unwrap().value("depth"), 49.0);
            assert_eq!(game.world.entity(sticky).unwrap().value("depth"), 1.0);
            assert_eq!(game.world.entity(held).unwrap().value("depth"), 0.0);
        }
    }

    #[test]
    fn game_step_item_safety_is_boolean_with_an_independent_alarm_while_held() {
        let mut game = game();
        let permanent = game.world.spawn("oRock", Vec2::new(100.0, 100.0));
        game.world
            .entity_mut(permanent)
            .unwrap()
            .data
            .insert("safe".into(), 1.0);
        let held = game.world.spawn("oRock", game.player.pos);
        let item = game.world.entity_mut(held).unwrap();
        item.held = true;
        item.data.insert("safe".into(), 1.0);
        item.alarms[2] = 2;
        game.player.held = Some(held);
        game.step(&InputFrame::default());
        assert_eq!(game.world.entity(held).unwrap().value("safe"), 1.0);
        assert_eq!(game.world.entity(held).unwrap().alarms[2], 1);
        game.step(&InputFrame::default());
        assert_eq!(game.world.entity(held).unwrap().value("safe"), 0.0);
        assert_eq!(game.world.entity(held).unwrap().value("safe_ticks"), 0.0);
        assert_eq!(game.world.entity(permanent).unwrap().value("safe"), 1.0);
    }

    #[test]
    fn game_step_dead_latch_blocks_item_actions_even_with_positive_life() {
        let mut game = game();
        for x in 0..game.world.width {
            game.world.set_tile(x, 4, "oBlock");
        }
        game.player.pos = Vec2::new(40.0, 56.0);
        game.player.state = PlayerState::Dead;
        game.state = RunState::Dead;
        game.player.health = 1;
        game.world.spawn("oRock", Vec2::new(40.0, 60.0));
        game.world.spawn("oGoldNugget", Vec2::new(40.0, 56.0));
        let before = (game.player.bombs, game.player.ropes);
        game.step(&InputFrame {
            down: true,
            attack: true,
            attack_pressed: true,
            bomb_pressed: true,
            rope_pressed: true,
            cycle_pressed: true,
            ..Default::default()
        });
        assert!(game.player.held.is_none());
        assert_eq!((game.player.bombs, game.player.ropes), before);
        assert_eq!(game.player.attack_timer, 0);
        assert_eq!(game.data.get("collect").copied().unwrap_or(0.0), 0.0);
        assert!(!game.world.entities.iter().any(|e| matches!(
            e.object.as_str(),
            "oWhip" | "oWhipPre" | "oBomb" | "oRopeThrow"
        )));
    }

    #[test]
    fn game_step_broken_jar_enemy_loot_uses_source_origins() {
        let mut spider = false;
        let mut snake = false;
        for seed in 0..256 {
            let mut game = Game::shell(seed, RulesPreset::Classic);
            game.player.pos = Vec2::new(200.0, 200.0);
            game.world.set_tile(4, 2, "oBlock");
            let jar = game.world.spawn("oJar", Vec2::new(55.0, 40.0));
            game.world.entity_mut(jar).unwrap().vel.x = 10.0;
            game.step(&InputFrame::default());
            for entity in &game.world.entities {
                if entity.object == "oSpider" {
                    spider = true;
                    assert_eq!(entity.previous_pos, Vec2::new(51.0, 32.0));
                }
                if entity.object == "oSnake" {
                    snake = true;
                    assert_eq!(entity.previous_pos, Vec2::new(43.0, 32.0));
                }
            }
            if spider && snake {
                break;
            }
        }
        assert!(spider && snake);
    }

    #[test]
    fn clean_jar_destroy_releases_hand_without_break_sound_or_loot_rng() {
        let mut game = game();
        let jar = game.world.spawn("oJar", game.player.pos);
        game.player.held = Some(jar);
        game.world
            .entity_mut(jar)
            .unwrap()
            .data
            .insert("breakPieces".into(), 0.0);
        let mut rng = game.rng.clone();
        break_item(&mut game, jar);
        assert!(game.player.held.is_none());
        assert!(game.events.is_empty());
        assert_eq!(game.world.entities.len(), 1);
        assert_eq!(game.rng.next_u64(), rng.next_u64());
    }

    #[test]
    fn game_step_stun_cancels_whip_before_forward_hit_is_created() {
        let mut game = game();
        for x in 0..game.world.width {
            game.world.set_tile(x, 4, "oBlock");
        }
        game.player.pos = Vec2::new(40.0, 56.0);
        game.player.grounded = true;
        game.player.state = PlayerState::Standing;
        game.step(&InputFrame {
            attack_pressed: true,
            ..Default::default()
        });
        for _ in 0..4 {
            game.step(&InputFrame::default());
        }
        game.player.stun = 20;
        game.step(&InputFrame::default());
        assert_eq!(game.player.attack_timer, 0);
        assert!(
            !game
                .world
                .entities
                .iter()
                .any(|e| e.alive && crate::catalog::is_a(&e.object, "oWhip"))
        );
        for _ in 0..12 {
            game.step(&InputFrame::default());
        }
        assert!(!game.world.entities.iter().any(|e| e.object == "oWhip"));
    }
    #[test]
    fn whip_has_rear_windup_and_delayed_forward_hit() {
        let mut game = game();
        for x in 0..game.world.width {
            game.world.set_tile(x, 4, "oBlock");
        }
        game.player.pos = Vec2::new(40.0, 56.0);
        game.step(&InputFrame {
            attack_pressed: true,
            ..InputFrame::default()
        });
        assert_eq!(game.player.sprite, "sAttackLeft");
        assert_eq!(game.player.animation, 0.6);
        assert!(
            !game
                .world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oWhipPre")
        );
        game.step(&InputFrame::default());
        let pre = game
            .world
            .entities
            .iter()
            .find(|e| e.alive && e.object == "oWhipPre")
            .unwrap();
        assert_eq!(pre.pos.x, 24.0);
        assert_eq!(pre.alarms[0], 3);
        for _ in 0..3 {
            game.step(&InputFrame::default());
        }
        assert!(
            !game
                .world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oWhip")
        );
        game.step(&InputFrame::default());
        assert!(
            game.world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oWhip" && e.pos.x == 56.0)
        );
    }
    #[test]
    fn tunnel_man_innate_attack_digs_and_damages_through_game_step() {
        let mut game = game();
        crate::characters::apply(&mut game, crate::characters::Character::TunnelMan);
        for x in 0..game.world.width {
            game.world.set_tile(x, 4, "oBlock");
        }
        game.world.set_tile(3, 3, "oBlock");
        game.player.pos = Vec2::new(40.0, 56.0);
        game.player.previous_pos = game.player.pos;
        game.player.grounded = true;
        game.player.state = PlayerState::Standing;
        let mut damage_game = game.clone();
        damage_game.world.set_tile(3, 3, "");
        let enemy = damage_game.world.spawn("oCaveman", Vec2::new(50.0, 48.0));
        crate::enemies::init_entity(damage_game.world.entity_mut(enemy).unwrap());
        damage_game.player.invincible = 100;
        damage_game.world.entity_mut(enemy).unwrap().state = 0;
        damage_game.world.entity_mut(enemy).unwrap().timer = 200;
        damage_game.world.entity_mut(enemy).unwrap().vel = Vec2::default();
        damage_game.step(&InputFrame {
            attack_pressed: true,
            ..InputFrame::default()
        });
        for _ in 0..11 {
            damage_game.step(&InputFrame::default());
        }
        game.step(&InputFrame {
            attack_pressed: true,
            ..InputFrame::default()
        });
        assert_eq!(game.player.sprite, "sTunnelAttackL");
        assert!(
            !game
                .world
                .entities
                .iter()
                .any(|e| matches!(e.object.as_str(), "oWhip" | "oWhipPre"))
        );
        for _ in 0..11 {
            game.step(&InputFrame::default());
        }
        assert_eq!(game.world.tile(3, 3).unwrap().object, "");
        assert!(
            damage_game
                .world
                .entity(enemy)
                .is_none_or(|e| e.health <= 1),
            "enemy={:?}",
            damage_game.world.entity(enemy)
        );
        assert_eq!(game.player.attack_timer, 0);
        assert!(!game.player.data.contains_key("character_attack_active"));
        assert!(game.player.held.is_none());
        assert_eq!((game.player.bombs, game.player.ropes), (0, 0));
    }

    #[test]
    fn ordinary_item_can_fall_away_from_partial_ceiling_overlap() {
        let mut game = game();
        game.player.pos = Vec2::new(200.0, 200.0);
        game.world.set_tile(6, 5, "oBlock");
        let rock = game.world.spawn("oRock", Vec2::new(104.0, 98.0));
        game.world.entity_mut(rock).unwrap().vel.y = 2.0;
        game.step(&InputFrame::default());
        assert_eq!(game.world.entity(rock).unwrap().pos.y, 101.0);
    }

    #[test]
    fn throw_under_an_overhang_uses_source_forward_flat_trajectory() {
        let mut game = game();
        game.player.pos = Vec2::new(104.0, 104.0);
        game.world.set_tile(7, 5, "oBlock");
        let rock = game.world.spawn("oRock", game.player.pos);
        game.world.entity_mut(rock).unwrap().held = true;
        game.player.held = Some(rock);
        game.step(&InputFrame {
            attack_pressed: true,
            ..InputFrame::default()
        });
        let rock = game.world.entity(rock).unwrap();
        assert_eq!(rock.vel.x, 9.0);
        assert_eq!(rock.vel.y, 0.6);
    }

    #[test]
    fn resting_items_apply_floor_friction_without_a_vertical_move() {
        let mut game = game();
        game.player.pos = Vec2::new(200.0, 200.0);
        for x in 0..game.world.width {
            game.world.set_tile(x, 4, "oBlock");
        }
        let rock = game.world.spawn("oRock", Vec2::new(100.0, 60.0));
        game.world.entity_mut(rock).unwrap().vel.x = 1.0;
        for _ in 0..4 {
            game.step(&InputFrame::default());
        }
        let rock = game.world.entity(rock).unwrap();
        assert_eq!(rock.pos, Vec2::new(102.0, 60.0));
        assert_eq!(rock.vel, Vec2::default());
    }

    #[test]
    fn slow_item_motion_uses_source_modulo_in_real_game_steps() {
        let mut game = game();
        let rock = game.world.spawn("oRock", Vec2::new(100.0, 100.0));
        game.world.entity_mut(rock).unwrap().vel.x = 0.3;
        for expected in [100.0, 101.0, 101.0, 101.0, 102.0, 102.0] {
            game.step(&InputFrame::default());
            assert_eq!(game.world.entity(rock).unwrap().pos.x, expected);
        }
    }

    #[test]
    fn item_motion_resolves_full_horizontal_axis_before_vertical_corner() {
        let mut game = game();
        game.player.pos = Vec2::new(160.0, 160.0);
        game.world.set_tile(4, 3, "oBlock");
        let rock = game.world.spawn("oRock", Vec2::new(55.0, 40.0));
        game.world.entity_mut(rock).unwrap().vel = Vec2::new(10.0, 10.0);
        game.step(&InputFrame::default());
        assert_eq!(game.world.entity(rock).unwrap().pos, Vec2::new(65.0, 44.0));
    }

    #[test]
    fn down_attack_in_air_preserves_weapon_and_held_jar_uses_source_origin() {
        let mut game = game();
        game.player.pos = Vec2::new(100.0, 100.0);
        let weapon = game.world.spawn("oShotgun", game.player.pos);
        game.world.entity_mut(weapon).unwrap().held = true;
        game.player.held = Some(weapon);
        game.step(&InputFrame {
            down: true,
            attack_pressed: true,
            ..InputFrame::default()
        });
        assert_eq!(game.player.held, Some(weapon));
        assert_eq!(game.world.entity(weapon).unwrap().vel, Vec2::default());
        game.world.entity_mut(weapon).unwrap().alive = false;
        let jar = game.world.spawn("oJar", game.player.pos);
        game.world.entity_mut(jar).unwrap().held = true;
        game.player.held = Some(jar);
        game.step(&InputFrame::default());
        assert_eq!(game.world.entity(jar).unwrap().pos.y, game.player.pos.y);
    }

    #[test]
    fn ordinary_item_drowns_in_authored_lava_entity_without_break_sound() {
        let mut game = game();
        game.player.pos = Vec2::new(200.0, 200.0);
        game.world.spawn("oLava", Vec2::new(80.0, 80.0));
        let rock = game.world.spawn("oRock", Vec2::new(88.0, 88.0));
        game.step(&InputFrame::default());
        assert!(game.world.entity(rock).is_none());
        assert!(
            !game
                .events
                .iter()
                .any(|event| matches!(event,GameEvent::Sound{name,..} if name=="sndBreak"))
        );
    }

    #[test]
    fn container_interaction_consumes_tunnel_attack_edge() {
        let mut game = game();
        crate::characters::apply(&mut game, crate::characters::Character::TunnelMan);
        game.player.grounded = true;
        let pos = game.player.pos;
        // This subsystem fixture bypasses Game.step; synchronous Create must
        // register the crate before the source parent-array container query.
        let crate_id = game.spawn("oCrate", pos);
        tick(
            &mut game,
            &InputFrame {
                up: true,
                attack: true,
                attack_pressed: true,
                ..InputFrame::default()
            },
        );
        assert!(game.world.entity(crate_id).is_none());
        assert!(!game.player.data.contains_key("character_attack_active"));
        assert!(
            !game
                .world
                .entities
                .iter()
                .any(|e| matches!(e.object.as_str(), "oMattockPre" | "oWhipPre"))
        );
    }

    #[test]
    fn fire_frog_armed_is_heavy_body_and_ball_physics_is_delegated() {
        let mut game = game();
        let id = game.world.spawn("oFireFrogArmed", Vec2::new(40.0, 40.0));
        init_entity(game.world.entity_mut(id).unwrap());
        let frog = game.world.entity(id).unwrap();
        assert!(is_item(&frog.object));
        assert_eq!(frog.value("heavy"), 1.0);
        assert_eq!(
            body_bounds(frog, frog.pos),
            Rect::new(34.0, 40.0, 12.0, 8.0)
        );
        assert!(is_item("oBall") && is_item("oBall2"));
        assert!(!ordinary_body("oBall") && !ordinary_body("oBall2"));
    }

    #[test]
    fn throwing_reserved_bomb_restores_carried_item() {
        let mut game = game();
        let rock = game.world.spawn("oRock", game.player.pos);
        init_entity(game.world.entity_mut(rock).unwrap());
        game.player.held = Some(rock);
        game.world.entity_mut(rock).unwrap().held = true;
        explosives::cycle_inventory(&mut game);
        let bomb = game.player.held.unwrap();
        explosives::arm_bomb(game.world.entity_mut(bomb).unwrap());
        throw_item(&mut game, bomb, &InputFrame::default());
        let restored = game.player.held.unwrap();
        assert_ne!(restored, rock, "scrHoldItem creates a fresh instance");
        assert!(game.world.entity(rock).is_none());
        let fresh = game.world.entity(restored).unwrap();
        assert_eq!(fresh.object, "oRock");
        assert_eq!(fresh.value("cost"), 0.0);
        assert_eq!(fresh.value("new"), 0.0);
        assert_eq!(game.player.pickup_item_type, "Rock");
        assert!(!game.world.entity(bomb).unwrap().held);
        assert_eq!(game.player.bombs, 3);
        assert_eq!(fresh.value("stashed"), 0.0);
    }

    #[test]
    fn thrown_rock_bounces_from_dynamic_push_block() {
        let mut game = game();
        let block = game.world.spawn("oPushBlock", Vec2::new(64.0, 32.0));
        game.world.entity_mut(block).unwrap().solid = true;
        let id = game.world.spawn("oRock", Vec2::new(55.0, 40.0));
        let entity = game.world.entity_mut(id).unwrap();
        init_entity(entity);
        entity.vel.x = 10.0;
        tick_bodies(&mut game);
        let rock = game.world.entity(id).unwrap();
        assert!(rock.pos.x < 64.0);
        assert!(rock.vel.x < 0.0);
    }

    #[test]
    fn fast_jar_breaks_at_wall_without_tunneling() {
        let mut game = game();
        game.world.set_tile(4, 2, "oBlock");
        let id = game.world.spawn("oJar", Vec2::new(55.0, 40.0));
        let entity = game.world.entity_mut(id).unwrap();
        init_entity(entity);
        entity.vel.x = 10.0;
        tick_bodies(&mut game);
        assert!(game.world.entity(id).is_none());
        assert!(
            game.events
                .iter()
                .any(|e| matches!(e,GameEvent::Sound{name,..} if name=="sndBreak"))
        );
    }
}

#[cfg(test)]
mod jar_collision_source_tests {
    use super::*;
    use crate::{GameEvent, Rng, dispatcher::Instance};
    use std::collections::{BTreeMap, BTreeSet};
    type Birth = (&'static str, Vec2, Vec2);

    // Literal source oracle, deliberately independent of spawn_jar_loot and
    // break_item. Each elseif consumes a roll only when actually reached.
    fn literal_loot(
        rng: &mut Rng,
        pos: Vec2,
        left: bool,
        right: bool,
        out: &mut Vec<Birth>,
    ) -> &'static str {
        for (object, denominator) in [
            ("oGoldChunk", 3),
            ("oGoldNugget", 6),
            ("oEmeraldBig", 12),
            ("oSapphireBig", 12),
            ("oRubyBig", 12),
            ("oSpider", 6),
            ("oSnake", 12),
        ] {
            if rng.range(denominator) == 0 {
                let spawn = if object == "oSpider" {
                    Vec2::new(pos.x - 8.0, pos.y - 8.0)
                } else if object == "oSnake" {
                    Vec2::new(
                        pos.x
                            - if left {
                                0.0
                            } else if right {
                                16.0
                            } else {
                                8.0
                            },
                        pos.y - 8.0,
                    )
                } else {
                    pos
                };
                out.push((
                    object,
                    spawn,
                    if object == "oSnake" {
                        Vec2::new(2.5, 0.0)
                    } else {
                        Vec2::default()
                    },
                ));
                return object;
            }
        }
        "none"
    }
    fn literal_whip(rng: &mut Rng, pos: Vec2, out: &mut Vec<Birth>) -> &'static str {
        out.push(("oSmokePuff", pos, Vec2::new(0.0, 0.1)));
        for _ in 0..3 {
            let vx = f64::from(1 + rng.range(3)) - f64::from(1 + rng.range(3));
            out.push((
                "oRubbleSmall",
                Vec2::new(pos.x - 2.0, pos.y - 2.0),
                Vec2::new(vx, 0.0),
            ));
        }
        literal_loot(rng, pos, false, false, out)
    }
    fn literal_destroy(
        rng: &mut Rng,
        pos: Vec2,
        left: bool,
        right: bool,
        top: bool,
        out: &mut Vec<Birth>,
    ) -> &'static str {
        out.push(("oSmokePuff", pos, Vec2::new(0.0, 0.1)));
        for _ in 0..3 {
            let vx = if left {
                f64::from(1 + rng.range(3))
            } else if right {
                -f64::from(1 + rng.range(3))
            } else {
                f64::from(1 + rng.range(3)) - f64::from(1 + rng.range(3))
            };
            let vy = f64::from(rng.range(4)) * if top { 1.0 } else { -1.0 };
            out.push((
                "oRubbleSmall",
                Vec2::new(pos.x - 2.0, pos.y - 2.0),
                Vec2::new(vx, vy),
            ));
        }
        literal_loot(rng, pos, left, right, out)
    }
    fn branch_seeds() -> BTreeSet<u32> {
        let mut first = BTreeMap::new();
        let mut second = BTreeMap::new();
        let mut bullet = BTreeMap::new();
        for seed in 0..2048 {
            let mut rng = Rng::new(seed);
            let mut ignored = Vec::new();
            let pos = Vec2::new(100.0, 100.0);
            first
                .entry(literal_whip(&mut rng, pos, &mut ignored))
                .or_insert(seed);
            second
                .entry(literal_destroy(
                    &mut rng,
                    pos,
                    false,
                    false,
                    false,
                    &mut ignored,
                ))
                .or_insert(seed);
            let mut rng = Rng::new(seed);
            bullet
                .entry(literal_destroy(
                    &mut rng,
                    pos,
                    false,
                    false,
                    false,
                    &mut ignored,
                ))
                .or_insert(seed);
            if first.len() == 8 && second.len() == 8 && bullet.len() == 8 {
                break;
            }
        }
        assert_eq!((first.len(), second.len(), bullet.len()), (8, 8, 8));
        first
            .into_values()
            .chain(second.into_values())
            .chain(bullet.into_values())
            .collect()
    }
    #[test]
    fn literal_jar_whip_bullet_pre_effect_order_rng_held_clean_flags_and_idempotency() {
        let mut first_branches = BTreeSet::new();
        let mut destroy_branches = BTreeSet::new();
        let mut bullet_branches = BTreeSet::new();
        for seed in branch_seeds() {
            for projectile in ["oWhip", "oBullet", "oWhipPre"] {
                for pieces in [false, true] {
                    for held in [false, true] {
                        for (left, right, top) in [
                            (false, false, false),
                            (true, false, false),
                            (false, true, true),
                            (true, true, true),
                        ] {
                            let mut game = Game::shell(seed, RulesPreset::Classic);
                            let pos = Vec2::new(100.0, 100.0);
                            let jar = game.spawn("oJar", pos);
                            let other = game.spawn(projectile, pos);
                            let source = game.world.entity_mut(jar).unwrap();
                            source.held = held;
                            for (key, value) in [
                                ("breakPieces", pieces),
                                ("colLeft", left),
                                ("colRight", right),
                                ("colTop", top),
                            ] {
                                source.data.insert(key.into(), f64::from(value));
                            }
                            if held {
                                game.player.held = Some(jar);
                            }
                            let mut expected = game.rng.clone();
                            let mut births = Vec::new();
                            let mut sounds = 0;
                            if projectile == "oWhip" {
                                first_branches.insert(literal_whip(
                                    &mut expected,
                                    pos,
                                    &mut births,
                                ));
                                sounds += 1;
                            }
                            if projectile != "oWhipPre" && pieces {
                                let branch = literal_destroy(
                                    &mut expected,
                                    pos,
                                    left,
                                    right,
                                    top,
                                    &mut births,
                                );
                                if projectile == "oWhip" {
                                    destroy_branches.insert(branch);
                                } else {
                                    bullet_branches.insert(branch);
                                }
                                sounds += 1;
                            }
                            collision(&mut game, jar, Instance::Entity(other));
                            let actual: Vec<_> = game
                                .world
                                .entities
                                .iter()
                                .filter(|e| e.id > other)
                                .collect();
                            assert_eq!(
                                actual.len(),
                                births.len(),
                                "seed={seed} {projectile} pieces={pieces}"
                            );
                            for (entity, (object, pos, velocity)) in actual.iter().zip(&births) {
                                assert_eq!(entity.object, *object);
                                assert_eq!(entity.pos, *pos);
                                assert_eq!(entity.vel, *velocity);
                                assert!(entity.alive && entity.engine_active);
                                if *object == "oRubbleSmall" {
                                    assert_eq!(entity.value("gravity"), 0.6);
                                }
                                if *object == "oSpider" || *object == "oSnake" {
                                    assert_eq!(entity.health, 1);
                                }
                            }
                            assert_eq!(game.events.iter().filter(|e| matches!(e,GameEvent::Sound{name,..} if name=="sndBreak")).count(),sounds);
                            assert_eq!(game.world.entity(jar).is_some(), projectile == "oWhipPre");
                            assert_eq!(
                                game.player.held,
                                if projectile == "oWhipPre" && held {
                                    Some(jar)
                                } else {
                                    None
                                }
                            );
                            assert!(
                                game.world.entity(other).is_some(),
                                "Jar callback does not retire projectile"
                            );
                            assert_eq!(game.rng.clone().next_u64(), expected.next_u64());
                            let before = serde_json::to_value(&game.world).unwrap();
                            let rng_before = game.rng.clone().next_u64();
                            let events = game.events.len();
                            collision(&mut game, jar, Instance::Entity(other));
                            if projectile != "oWhipPre" {
                                break_item(&mut game, jar);
                            }
                            assert_eq!(serde_json::to_value(&game.world).unwrap(), before);
                            assert_eq!(game.rng.next_u64(), rng_before);
                            assert_eq!(game.events.len(), events);
                        }
                    }
                }
            }
        }
        assert_eq!(
            (
                first_branches.len(),
                destroy_branches.len(),
                bullet_branches.len()
            ),
            (8, 8, 8)
        );
    }
}

#[cfg(test)]
mod skull_bullet_source_tests {
    use super::*;
    use crate::{GameEvent, Rng, dispatcher::Instance};
    use std::collections::BTreeSet;

    // Literal Skull Destroy loop and full Bone Create, independent of break_item.
    // The condition consumes rand(1,2) again on the final failed comparison.
    fn literal_bones(rng: &mut Rng, left: bool, right: bool, top: bool) -> Vec<(Vec2, f64)> {
        let mut result = Vec::new();
        while result.len() < (1 + rng.range(2)) as usize {
            let _created_x = rng.unit() * 4.0 - rng.unit() * 4.0;
            let _created_y = -1.0 - rng.unit() * 2.0;
            let gravity = f64::from(1 + rng.range(6)) * 0.1;
            let vx = if left {
                f64::from(1 + rng.range(3))
            } else if right {
                -f64::from(1 + rng.range(3))
            } else {
                f64::from(1 + rng.range(3)) - f64::from(1 + rng.range(3))
            };
            let vy = f64::from(rng.range(4)) * if top { 1.0 } else { -1.0 };
            result.push((Vec2::new(vx, vy), gravity));
        }
        result
    }

    #[test]
    fn skull_bullet_runs_literal_destroy_rng_caps_held_and_once_only() {
        let mut counts = BTreeSet::new();
        for seed in 0..16 {
            for existing in [0, 31, 32] {
                for pieces in [false, true] {
                    for held in [false, true] {
                        for (left, right, top) in [
                            (false, false, false),
                            (true, false, false),
                            (false, true, false),
                            (false, false, true),
                            (true, true, true),
                        ] {
                            let mut game = Game::shell(seed, RulesPreset::Classic);
                            let pos = Vec2::new(100.0, 100.0);
                            for _ in 0..existing {
                                game.spawn("oBone", Vec2::new(200.0, 200.0));
                            }
                            let skull = game.spawn("oSkull", pos);
                            let entity = game.world.entity_mut(skull).unwrap();
                            entity.held = held;
                            for (key, value) in [
                                ("breakPieces", pieces),
                                ("colLeft", left),
                                ("colRight", right),
                                ("colTop", top),
                            ] {
                                entity.data.insert(key.into(), f64::from(value));
                            }
                            if held {
                                game.player.held = Some(skull);
                            }
                            let bullet = game.spawn("oBullet", pos);
                            let mut oracle = game.rng.clone();
                            let expected = if pieces {
                                literal_bones(&mut oracle, left, right, top)
                            } else {
                                Vec::new()
                            };
                            if pieces {
                                counts.insert(expected.len());
                            }
                            collision(&mut game, skull, Instance::Entity(bullet));
                            assert!(
                                game.world.entity(skull).is_none(),
                                "Skull Bullet callback missing, seed={seed} pieces={pieces}"
                            );
                            assert!(
                                game.world.entity(bullet).is_some(),
                                "Skull event leaves projectile unchanged"
                            );
                            assert_eq!(game.player.held, None);
                            let births: Vec<_> = game
                                .world
                                .entities
                                .iter()
                                .filter(|e| e.id > bullet)
                                .collect();
                            assert_eq!(births.len(), expected.len() + usize::from(pieces));
                            if pieces {
                                assert_eq!(births[0].object, "oSmokePuff");
                                assert_eq!(births[0].pos, pos);
                                assert_eq!(births[0].vel, Vec2::new(0.0, 0.1));
                                for (index, (bone, (velocity, gravity))) in
                                    births[1..].iter().zip(&expected).enumerate()
                                {
                                    assert_eq!(bone.object, "oBone");
                                    assert_eq!(bone.pos, Vec2::new(98.0, 98.0));
                                    // Direct piece.field writes also override a capped newborn.
                                    assert_eq!(bone.vel, *velocity);
                                    assert_eq!(bone.value("gravity"), *gravity);
                                    assert_eq!(bone.alive, existing + index < 32);
                                    assert_eq!(bone.value("world_effect_initialized"), 1.0);
                                }
                            }
                            assert_eq!(game.events.iter().filter(|e| matches!(e, GameEvent::Sound{name,..} if name=="sndBreak")).count(), usize::from(pieces));
                            assert_eq!(game.rng.clone().next_u64(), oracle.next_u64());
                            let world = serde_json::to_value(&game.world).unwrap();
                            let rng = game.rng.clone().next_u64();
                            let events = game.events.len();
                            collision(&mut game, skull, Instance::Entity(bullet));
                            break_item(&mut game, skull);
                            assert_eq!(serde_json::to_value(&game.world).unwrap(), world);
                            assert_eq!(game.rng.next_u64(), rng);
                            assert_eq!(game.events.len(), events);
                        }
                    }
                }
            }
        }
        assert_eq!(counts, BTreeSet::from([1, 2]));
    }

    #[test]
    fn skull_has_no_whip_or_whip_pre_break_callback() {
        for object in ["oWhip", "oWhipPre"] {
            let mut game = Game::shell(2, RulesPreset::Classic);
            let pos = Vec2::new(100.0, 100.0);
            let skull = game.spawn("oSkull", pos);
            let other = game.spawn(object, pos);
            let expected_rng = game.rng.clone().next_u64();
            let before = serde_json::to_value(&game.world).unwrap();
            collision(&mut game, skull, Instance::Entity(other));
            assert_eq!(serde_json::to_value(&game.world).unwrap(), before);
            assert_eq!(game.rng.next_u64(), expected_rng);
            assert!(game.events.is_empty());
        }
    }
}

#[cfg(test)]
mod embedded_contact_tests;
