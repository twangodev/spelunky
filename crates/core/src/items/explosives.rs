//! Bombs and ropes translated from Classic 1.1's oItem, oBomb, oRopeThrow,
//! oExplosion, and oPlayer1 events. All durations are simulation ticks.
use crate::model::*;

pub const BOMB_FUSE: u32 = 120;
pub const BOMB_FAST_FUSE: u32 = 40;
const EXPLOSION_TICKS: u32 = 13; // Ten sprite frames at image_speed 0.8.

pub fn init_entity(entity: &mut Entity) {
    if entity.value("explosives_initialized") != 0.0 {
        return;
    }
    match entity.object.as_str() {
        "oBomb" => {
            entity.sprite = "sBomb".into();
            entity.data.entry("armed".into()).or_insert(0.0);
            entity.data.entry("attached_id".into()).or_insert(0.0);
            entity.data.entry("sticky_x".into()).or_insert(0.0);
            entity.data.entry("sticky_y".into()).or_insert(0.0);
        }
        "oFireFrogBomb" => {
            entity.data.insert("armed".into(), 0.0);
            entity.data.insert("swimming".into(), 0.0);
            entity.data.insert("heavy".into(), 1.0);
            entity.timer = BOMB_FUSE;
            entity.alarms[1] = BOMB_FUSE as i32;
        }
        "oRopeThrow" => {
            entity.sprite = "sRopeEnd".into();
        }
        "oExplosion" => {
            entity.sprite = "sExplosion".into();
            entity.timer = EXPLOSION_TICKS;
            entity.animation = 0.0;
            entity.set_image_speed(0.8);
        }
        "oRope" | "oRopeTop" => {}
        "oRopeBurn" => {
            entity.sprite = "sRopeBurn".into();
            entity.vel.y = -1.0;
            entity.set_image_speed(0.8);
        }
        _ => return,
    }
    entity.data.insert("explosives_initialized".into(), 1.0);
}

fn can_act(game: &Game) -> bool {
    game.state == RunState::Playing && !super::player_dead(game) && !game.player.is_stunned()
}

fn can_use(game: &Game) -> bool {
    can_act(game) && game.player.attack_timer == 0
}

fn solid_point(world: &World, pos: Vec2) -> bool {
    world.solid_point_except(pos, 0)
}

/// Literal precision-zero Solid rectangle/point query used by the two Rope
/// actions. Source endpoints are inclusive and may form a zero-area point.
fn rope_solid_rectangle(world: &World, first: Vec2, second: Vec2) -> bool {
    world.tiles.iter().enumerate().any(|(index, tile)| {
        crate::catalog::is_a(&tile.object, "oSolid")
            && world
                .tile_collision_mask((index % world.width) as i32, (index / world.width) as i32)
                .intersects_rectangle_bbox(first, second)
    }) || world.entities.iter().any(|entity| {
        entity.alive
            && entity.engine_active
            && crate::catalog::is_a(&entity.object, "oSolid")
            && entity
                .collision_mask()
                .intersects_rectangle_bbox(first, second)
    })
}

/// First use arms a carried bomb; subsequent use can throw it normally.
pub fn arm_bomb(bomb: &mut Entity) {
    init_entity(bomb);
    if bomb.value("armed") == 0.0 {
        bomb.data.insert("armed".into(), 1.0);
        bomb.sprite = "sBombArmed".into();
        bomb.timer = BOMB_FUSE;
        bomb.animation = 0.0;
        bomb.alarms[0] = 80;
        bomb.set_image_speed(0.2);
    }
}

/// Literal scrHoldItem whitelist, shared by inventory and room-entry carry.
const HOLD_TYPES: [(&str, &str); 16] = [
    ("Rock", "oRock"),
    ("Jar", "oJar"),
    ("Skull", "oSkull"),
    ("Fish Bone", "oFishBone"),
    ("Arrow", "oArrow"),
    ("Machete", "oMachete"),
    ("Mattock", "oMattock"),
    ("Mattock Head", "oMattockHead"),
    ("Pistol", "oPistol"),
    ("Web Cannon", "oWebCannon"),
    ("Teleporter", "oTeleporter"),
    ("Shotgun", "oShotgun"),
    ("Bow", "oBow"),
    ("Flare", "oFlare"),
    ("Sceptre", "oSceptre"),
    ("Key", "oKey"),
];

pub(crate) fn hold_object(item_type: &str) -> Option<&'static str> {
    HOLD_TYPES
        .iter()
        .find(|(name, _)| *name == item_type)
        .map(|(_, object)| *object)
}

pub(crate) fn object_type(object: &str) -> Option<&'static str> {
    HOLD_TYPES
        .iter()
        .find(|(_, kind)| *kind == object)
        .map(|(name, _)| *name)
}

/// Original Create type declarations for the two pickup parent families.
/// Resolve inheritance so corpses and aliases retain their original type;
/// unsupported scrHoldItem types are still remembered, then rejected there.
pub(crate) fn source_type(entity: &Entity) -> String {
    if let Some(item_type) = entity.text.get("type") {
        return item_type.clone();
    }
    let mut object = entity.object.as_str();
    while !object.is_empty() {
        let item_type = match object {
            "oAlien" => Some("Alien"),
            "oAlienBoss" => Some("Alien Boss"),
            "oAnkh" => Some("Ankh"),
            "oArrow" => Some("Arrow"),
            "oBall" => Some("Ball"),
            "oBall2" => Some("Ball"),
            "oBasketball" => Some("Basketball"),
            "oBomb" => Some("Bomb"),
            "oBombBag" => Some("Bomb Bag"),
            "oBombBox" => Some("Bomb Box"),
            "oBow" => Some("Bow"),
            "oCapePickup" => Some("Cape"),
            "oCaveman" => Some("Caveman"),
            "oChest" => Some("Chest"),
            "oCompass" => Some("Compass"),
            "oCrate" => Some("Crate"),
            "oCrown" => Some("Crown"),
            "oCrystalSkull" => Some("Gold Idol"),
            "oDamsel" => Some("Damsel"),
            "oDice" => Some("Dice"),
            "oEnemy" => Some("None"),
            "oFireFrog" => Some("Fire Frog"),
            "oFireFrogArmed" => Some("Fire Frog"),
            "oFireFrogBomb" => Some("Fire Frog Bomb"),
            "oFishBone" => Some("Fish Bone"),
            "oFlare" => Some("Flare"),
            "oFlareCrate" => Some("Flare Crate"),
            "oFrog" => Some("Frog"),
            "oGiantSpider" => Some("Giant Spider"),
            "oGloves" => Some("Gloves"),
            "oGoldIdol" => Some("Gold Idol"),
            "oHawkman" => Some("Yeti"),
            "oItem" => Some("None"),
            "oJar" => Some("Jar"),
            "oJaws" => Some("Mega Mouth"),
            "oJetpack" => Some("Jetpack"),
            "oJordans" => Some("Jordans"),
            "oKapala" => Some("Kapala"),
            "oKey" => Some("Key"),
            "oLampItem" => Some("Lamp"),
            "oLampRedItem" => Some("Lamp"),
            "oLockedChest" => Some("Locked Chest"),
            "oMachete" => Some("Machete"),
            "oMagmaMan" => Some("Magma Man"),
            "oManTrap" => Some("ManTrap"),
            "oMattock" => Some("Mattock"),
            "oMattockHead" => Some("Mattock Head"),
            "oMitt" => Some("Mitt"),
            "oMonkey" => Some("Monkey"),
            "oParaPickup" => Some("Parachute"),
            "oPaste" => Some("Paste"),
            "oPiranha" => Some("Piranha"),
            "oPistol" => Some("Pistol"),
            "oRock" => Some("Rock"),
            "oRopeBag" => Some("Bomb Bag"),
            "oRopePile" => Some("Rope Pile"),
            "oRopeThrow" => Some("Rope"),
            "oSceptre" => Some("Sceptre"),
            "oShopkeeper" => Some("Shopkeeper"),
            "oShotgun" => Some("Shotgun"),
            "oSkeleton" => Some("Skeleton"),
            "oSkull" => Some("Skull"),
            "oSnake" => Some("Snake"),
            "oSpectacles" => Some("Spectacles"),
            "oSpider" => Some("Spider"),
            "oSpikeShoes" => Some("Spike Shoes"),
            "oSpringShoes" => Some("Spring Shoes"),
            "oTeleporter" => Some("Teleporter"),
            "oTombLord" => Some("Tomb Lord"),
            "oUFO" => Some("UFO"),
            "oUdjatEye" => Some("Udjat Eye"),
            "oVampire" => Some("Vampire"),
            "oWebCannon" => Some("Web Cannon"),
            "oYeti" => Some("Yeti"),
            "oYetiKing" => Some("Yeti King"),
            _ => None,
        };
        if let Some(item_type) = item_type {
            return item_type.to_owned();
        }
        object = crate::catalog::object(object)
            .map(|object| object.parent.as_str())
            .unwrap_or("");
    }
    "None".into()
}

pub(crate) fn pickup_pose(game: &mut Game) {
    game.player.data.insert("whoaTimer".into(), 30.0);
}

/// scrHoldItem runs Create synchronously and discards all former instance state.
pub(crate) fn hold_item_type(game: &mut Game, item_type: &str) {
    game.player.held = None;
    game.player.pickup_item_type.clear();
    let Some(object) = hold_object(item_type) else {
        return;
    };
    let id = game.spawn(object, game.player.pos);
    let entity = game.world.entity_mut(id).expect("fresh source held item");
    entity.held = true;
    entity.data.insert("cost".into(), 0.0);
    entity.data.insert("new".into(), 0.0);
    if object == "oBow" {
        entity.data.insert("bow_picked_up".into(), 1.0);
    }
    game.player.held = Some(id);
    game.player.pickup_item_type = item_type.to_owned();
    pickup_pose(game);
}

pub fn restore_stashed_item(game: &mut Game) {
    let item_type = game.player.pickup_item_type.clone();
    hold_item_type(game, &item_type);
}

/// Import the previous retained-ID save shape before any actor alarms run.
/// Invalid references clear bookkeeping only; unrelated actors remain intact.
pub(crate) fn migrate_legacy_stash(game: &mut Game) {
    let Some(reference) = game.player.data.remove("stash_id") else {
        // Previous saves without a reservation have only the ordinary held
        // actor. Its validated source type supplies the missing serde field.
        // Reserved Bomb/Rope and unmarked references never become a weapon.
        if game.player.pickup_item_type.is_empty() {
            if let Some(entity) = game.player.held.and_then(|id| game.world.entity(id)) {
                if entity.held && entity.value("stashed") == 0.0 {
                    if let Some(item_type) = object_type(&entity.object) {
                        game.player.pickup_item_type = item_type.to_owned();
                    }
                }
            }
        }
        return;
    };
    let legacy = game
        .world
        .entities
        .iter()
        .find(|entity| {
            reference.is_finite()
                && reference > 0.0
                && reference == entity.id as f64
                && entity.alive
                && entity.held
                && entity.value("stashed") > 0.0
                && object_type(&entity.object).is_some_and(|kind| kind == source_type(entity))
        })
        .map(|entity| (entity.id, source_type(entity)));
    let Some((id, item_type)) = legacy else {
        return;
    };
    if game.player.pickup_item_type.is_empty() {
        game.player.pickup_item_type = item_type;
    }
    let entity = game
        .world
        .entities
        .iter_mut()
        .find(|entity| entity.id == id)
        .unwrap();
    entity.data.insert("breakPieces".into(), 0.0);
    entity.held = false;
    entity.alive = false;
    entity.alarms = crate::dispatcher::unset_alarms();
    entity.data.remove("stashed");
    if game.player.held == Some(id) {
        game.player.held = None;
    }
}

fn equip_inventory_item(game: &mut Game, kind: &str) {
    let id = game.spawn(kind, game.player.pos);
    let paste = game.player.has("Paste");
    let entity = game.world.entity_mut(id).unwrap();
    init_entity(entity);
    entity.held = true;
    if kind == "oBomb" {
        entity.data.insert("sticky".into(), f64::from(paste));
        game.player.bombs -= 1;
    } else {
        game.player.ropes -= 1;
    }
    game.player.held = Some(id);
    pickup_pose(game);
}

/// Cycle a free light item through reserved bomb, reserved rope, and the item.
pub fn cycle_inventory(game: &mut Game) {
    if !game.is_level()
        || !can_use(game)
        || game.player.data.get("active").copied().unwrap_or(1.0) == 0.0
    {
        return;
    }
    let held = game
        .player
        .held
        .and_then(|id| game.world.entity(id).cloned());
    if let Some(entity) = held {
        if entity.sprite == "sBombArmed" {
            return;
        }
        if entity.sprite == "sBomb" || entity.sprite == "sRopeEnd" {
            game.world.entity_mut(entity.id).unwrap().alive = false;
            game.player.held = None;
            if entity.sprite == "sBomb" {
                game.player.bombs += 1;
                if game.player.ropes > 0 {
                    equip_inventory_item(game, "oRopeThrow");
                } else {
                    restore_stashed_item(game);
                }
            } else {
                game.player.ropes += 1;
                restore_stashed_item(game);
            }
            return;
        }
        if entity.value("heavy") != 0.0
            || entity.value("cost") != 0.0
            || (game.player.bombs <= 0 && game.player.ropes <= 0)
        {
            return;
        }
        game.player.pickup_item_type = source_type(&entity);
        // Player Step1040: fire while the original Bow is still held, before
        // destruction and before this frame's later bow charging branch.
        if entity.object == "oBow" {
            super::weapons::fire_bow_on_drop(game);
        }
        game.world
            .entity_mut(entity.id)
            .unwrap()
            .data
            .insert("breakPieces".into(), 0.0);
        super::break_item(game, entity.id);
    }
    if game.player.bombs > 0 {
        equip_inventory_item(game, "oBomb");
    } else if game.player.ropes > 0 {
        equip_inventory_item(game, "oRopeThrow");
    }
}

/// A carried rope has already been removed from the rope inventory.
pub fn use_held_rope(game: &mut Game, id: u64, input: &InputFrame) -> bool {
    let Some(held) = game
        .world
        .entity(id)
        .filter(|entity| entity.object == "oRopeThrow" && entity.held)
        .cloned()
    else {
        return false;
    };
    if !can_act(game) || (!input.down && crate::player::cached_col_top(&game.player)) {
        return true;
    }
    let player_pos = game.player.pos;
    let facing = f64::from(game.player.facing);
    {
        let rope = game.world.entity_mut(id).unwrap();
        rope.held = false;
        rope.data.insert("armed".into(), 1.0);
        rope.data.insert("px".into(), player_pos.x);
        rope.data.insert("py".into(), player_pos.y);
    }
    if input.down {
        // scrUseItem creates the attempted anchor before any placement query.
        // The old held actor remains available for a failed attempt's origin.
        let attempt = game.spawn(
            "oRopeThrow",
            Vec2::new(player_pos.x + 16.0 * facing, player_pos.y),
        );
        let mut pos = game.world.entity(attempt).unwrap().pos;
        pos.x = (pos.x / 16.0).round_ties_even() * 16.0;
        pos.y = pos.y.round_ties_even();
        {
            let rope = game.world.entity_mut(attempt).unwrap();
            rope.pos = pos;
            rope.data.insert("t".into(), 1.0);
        }
        let probe_pos = pos;
        let point_clear = |x| {
            let point = Vec2::new(x, player_pos.y);
            !rope_solid_rectangle(&game.world, point, point)
        };
        let clear_side = |offset: f64| {
            let first = Vec2::new(probe_pos.x + offset, probe_pos.y);
            let second = Vec2::new(probe_pos.x + offset + 1.0, probe_pos.y + 16.0);
            !rope_solid_rectangle(&game.world, first, second)
        };
        let mut fits = true;
        if player_pos.x < pos.x && point_clear(player_pos.x + 2.0) {
            if clear_side(-8.0) {
                pos.x -= 8.0;
            } else if clear_side(7.0) {
                pos.x += 8.0;
            } else {
                fits = false;
            }
        } else if point_clear(player_pos.x - 2.0) {
            if clear_side(7.0) {
                pos.x += 8.0;
            } else if clear_side(-8.0) {
                pos.x -= 8.0;
            } else {
                fits = false;
            }
        }
        // The source deliberately has no final else that clears `fits`.
        {
            let rope = game.world.entity_mut(attempt).unwrap();
            rope.pos = pos;
            rope.data.insert("t".into(), f64::from(fits));
        }
        if fits {
            game.spawn("oRopeTop", pos);
            let rope = game.world.entity_mut(attempt).unwrap();
            rope.data.insert("armed".into(), 0.0);
            rope.data.insert("falling".into(), 1.0);
            rope.vel = Vec2::default();
        } else {
            let thrown = game.spawn("oRopeThrow", held.pos);
            game.world.entity_mut(thrown).unwrap().vel = Vec2::new(3.2 * facing, 0.5);
            game.world.entity_mut(attempt).unwrap().alive = false;
        }
        game.world.entity_mut(id).unwrap().alive = false;
        game.player.held = None;
    } else {
        // Upward use launches the SAME reserved actor and leaves its y intact.
        let rope = game.world.entity_mut(id).unwrap();
        rope.pos.x = player_pos.x;
        rope.vel = Vec2::new(0.0, -12.0);
    }
    restore_stashed_item(game);
    game.sound("sndThrow", player_pos);
    true
}

/// Dedicated bomb button. The normal inventory item starts unarmed instead.
pub fn use_bomb(game: &mut Game, input: &InputFrame) {
    if !input.bomb_pressed || game.player.bombs <= 0 || !can_use(game) {
        return;
    }
    let pos = game.player.pos;
    let mut velocity = Vec2::new(
        8.0 * f64::from(game.player.facing) + game.player.vel.x,
        -3.0,
    );
    if input.up {
        velocity.y = -9.0;
    }
    if input.down {
        velocity.y = 3.0;
        if game.player.grounded {
            velocity.x *= 0.1;
        }
    }
    let paste = game.player.has("Paste");
    let id = game.spawn("oBomb", pos);
    let bomb = game.world.entity_mut(id).unwrap();
    init_entity(bomb);
    bomb.vel = velocity;
    arm_bomb(bomb);
    bomb.data
        .insert("sticky".into(), if paste { 1.0 } else { 0.0 });
    bomb.data.insert("safe_ticks".into(), 10.0);
    bomb.alarms[2] = 10;
    bomb.data.insert("safe".into(), 1.0);
    game.player.bombs -= 1;
    game.sound("sndThrow", pos);
}

pub fn use_rope(game: &mut Game, input: &InputFrame) {
    if !input.rope_pressed || game.player.ropes <= 0 || !can_use(game) {
        return;
    }
    let pos = game.player.pos;
    if !input.down && crate::player::cached_col_top(&game.player) {
        return;
    }
    if input.down {
        // Source Create precedes every placement query. Even a failed attempt
        // consumes its instance ID before instance_destroy, without charging rope.
        let offset = if game.player.facing < 0 { -16.0 } else { 16.0 };
        let initial = Vec2::new(pos.x + offset, pos.y);
        let id = game.spawn("oRopeThrow", initial);
        let snapped = Vec2::new(
            (initial.x / 16.0).round_ties_even() * 16.0,
            initial.y.round_ties_even(),
        );
        {
            let rope = game.world.entity_mut(id).unwrap();
            rope.data.insert("t".into(), 1.0);
            rope.pos = snapped;
        }
        // These are collision_point/rectangle(..., oSolid, 0, 0), using raw
        // endpoints and cached bounding boxes, not platform movement probes.
        let player_left = pos.x < snapped.x;
        let point = Vec2::new(pos.x + if player_left { 8.0 } else { -8.0 }, pos.y);
        let point_blocked = rope_solid_rectangle(&game.world, point, point);
        let left_clear = || {
            !rope_solid_rectangle(
                &game.world,
                Vec2::new(snapped.x - 8.0, snapped.y),
                Vec2::new(snapped.x - 7.0, snapped.y + 16.0),
            )
        };
        let right_clear = || {
            !rope_solid_rectangle(
                &game.world,
                Vec2::new(snapped.x + 7.0, snapped.y),
                Vec2::new(snapped.x + 8.0, snapped.y + 16.0),
            )
        };
        let placed_x = if point_blocked {
            None
        } else if player_left {
            if left_clear() {
                Some(snapped.x - 8.0)
            } else if right_clear() {
                Some(snapped.x + 8.0)
            } else {
                None
            }
        } else if right_clear() {
            Some(snapped.x + 8.0)
        } else if left_clear() {
            Some(snapped.x - 8.0)
        } else {
            None
        };
        let Some(x) = placed_x else {
            let rope = game.world.entity_mut(id).unwrap();
            rope.data.insert("t".into(), 0.0);
            rope.alive = false;
            return;
        };
        let anchor = Vec2::new(x, snapped.y);
        game.world.entity_mut(id).unwrap().pos = anchor;
        // Top Create executes before the receiver's launch fields change.
        spawn_rope_part(game, "oRopeTop", anchor);
        let rope = game.world.entity_mut(id).unwrap();
        rope.data.insert("armed".into(), 0.0);
        rope.data.insert("falling".into(), 1.0);
        rope.vel = Vec2::default();
        game.player.ropes -= 1;
        game.sound("sndThrow", anchor);
    } else {
        let id = game.spawn("oRopeThrow", pos);
        let rope = game.world.entity_mut(id).unwrap();
        rope.data.insert("armed".into(), 1.0);
        rope.data.insert("px".into(), pos.x);
        rope.data.insert("py".into(), pos.y);
        rope.vel = Vec2::new(0.0, -12.0);
        game.player.ropes -= 1;
        game.sound("sndThrow", pos);
    }
}

pub(crate) fn handles(object: &str) -> bool {
    matches!(
        object,
        "oBomb"
            | "oFireFrogBomb"
            | "oRopeThrow"
            | "oExplosion"
            | "oRope"
            | "oRopeTop"
            | "oRopeBurn"
    )
}

pub(crate) fn create_instance(game: &mut Game, id: u64) {
    let Some(mut entity) = game.world.entity(id).cloned() else {
        return;
    };
    if !handles(&entity.object) {
        return;
    }
    init_entity(&mut entity);
    if entity.object == "oExplosion" {
        announce_explosion(game, &mut entity);
    }
    if let Some(stored) = game.world.entity_mut(id) {
        *stored = entity;
    }
}

/// Normal Step only. Alarms, automatic collisions and animation are dispatched separately.
pub(crate) fn step_instance(game: &mut Game, id: u64) {
    let Some(mut entity) = game.world.entity(id).cloned() else {
        return;
    };
    init_entity(&mut entity);
    entity
        .data
        .insert("simulation_tick".into(), game.engine_time() as f64);
    match entity.object.as_str() {
        "oBomb" | "oFireFrogBomb" => {
            step_bomb_body(game, &mut entity);
            warn_shopkeeper(game, &entity);
        }
        "oRopeThrow" => tick_rope(game, &mut entity),
        "oRope" | "oRopeTop" => tick_rope_burn(game, &mut entity),
        "oRopeBurn" => tick_burning_flame(game, &mut entity),
        // oExplosion has no Normal Step event.
        _ => {}
    }
    if let Some(stored) = game
        .world
        .entities
        .iter_mut()
        .find(|stored| stored.id == id)
    {
        *stored = entity;
    }
    destroy_bomb(game, id);
}

pub(crate) fn alarm(game: &mut Game, id: u64, alarm: u8) {
    let Some(entity) = game.world.entity(id) else {
        return;
    };
    let object = entity.object.clone();
    if !matches!(object.as_str(), "oBomb" | "oFireFrogBomb") {
        return;
    }
    match alarm {
        0 if object == "oBomb" => {
            let bomb = game.world.entity_mut(id).unwrap();
            bomb.set_image_speed(1.0);
            bomb.alarms[1] = 40;
        }
        1 => {
            let pos = game.world.entity(id).unwrap().pos;
            // Create executes immediately; contact effects await the collision phase.
            game.spawn("oExplosion", pos);
            if object == "oFireFrogBomb" {
                // This child Alarm emits blood before the guarded flame script.
                // Its Create explicitly sets bloodless=false.
                let bomb = game.world.entity(id).unwrap().clone();
                crate::enemies::create_blood(game, &bomb, pos, 3);
            }
            if !game.flags.contains("low_graphics") {
                crate::world_events::create_flames(game, pos, 3);
            }
            if game.player.held == Some(id) {
                game.player.held = None;
            }
            game.world.entity_mut(id).unwrap().alive = false;
            destroy_bomb(game, id);
        }
        2 if object == "oBomb" => {
            game.world
                .entity_mut(id)
                .unwrap()
                .data
                .insert("safe_ticks".into(), 0.0);
        }
        _ => {}
    }
}

/// The dispatcher has already tested the pair's automatic collision masks.
pub(crate) fn collision(game: &mut Game, self_id: u64, other: crate::dispatcher::Instance) {
    use crate::dispatcher::Instance;
    let Some(explosion) = game
        .world
        .entity(self_id)
        .filter(|e| e.object == "oExplosion")
        .cloned()
    else {
        return;
    };
    let solids_active =
        game.flags.contains("tutorial") || super::item_body_active(game, &explosion);
    match other {
        Instance::Player => {} // Player hurt is a query in Player Step.
        Instance::Terrain(index) => {
            if solids_active && game.world.tiles.get(index).is_some_and(Tile::is_solid) {
                if game.world.tiles[index].is_destructible() {
                    destroy_terrain(game, index % game.world.width, index / game.world.width);
                }
                explosion_solid_cleanup(game);
            }
        }
        Instance::Entity(id) => {
            let Some(mut entity) = game.world.entity(id).cloned() else {
                return;
            };
            // The exact Boulder collision overrides the inherited Solid event:
            // it is unconditional despite the Boulder Create invincibility.
            if entity.object == "oBoulder" {
                explosion_boulder(game, &entity);
                return;
            }
            if entity.object == "oBarrierEmitter" {
                crate::enemies::destroy_barrier_emitter(game, id);
                return;
            }
            if crate::catalog::is_a(&entity.object, "oSolid") {
                if solids_active {
                    if entity.value("invincible") == 0.0 && entity.object != "oHardBlock" {
                        destroy_solid_instance(game, id);
                    }
                    explosion_solid_cleanup(game);
                }
                return;
            }
            if entity.object == "oMagmaMan" {
                for _ in 0..2 {
                    let magma = crate::world_events::spawn_detritus(
                        game,
                        "oMagma",
                        Vec2::new(entity.pos.x + 8.0, entity.pos.y - 4.0),
                    );
                    let y_vel = -(1.0 + f64::from(game.rng.range(3)));
                    if let Some(magma) = game.world.entities.iter_mut().find(|e| e.id == magma) {
                        magma.vel.y = y_vel;
                    }
                }
                entity.alive = false;
            } else if (crate::enemies::is_enemy(&entity.object) || entity.object == "oDamsel")
                && entity.value("invincible") == 0.0
            {
                entity.health -= if entity.object == "oDamsel" { 100 } else { 30 };
                let speed = 4.0 + f64::from(game.rng.range(3));
                entity.vel = Vec2::new(
                    if entity.pos.x > explosion.pos.x {
                        speed
                    } else {
                        -speed
                    },
                    -6.0,
                );
                entity.data.insert("burning".into(), 50.0);
                if entity.object == "oDamsel" {
                    entity.state = 2;
                }
            } else if entity.object == "oWeb" {
                entity.alive = false;
            } else if crate::catalog::is_a(&entity.object, "oItem") {
                match entity.object.as_str() {
                    "oArrow" | "oFishBone" | "oJar" | "oSkull" => entity.alive = false,
                    "oBomb" => {
                        entity.sprite = "sBombArmed".into();
                        entity.set_image_speed(1.0);
                        // Do not cancel Alarm 0: its pending callback may reset Alarm 1.
                        entity.alarms[1] = 4 + game.rng.range(5) as i32;
                        entity.timer = entity.alarms[1] as u32;
                        entity.data.insert("attached_id".into(), 0.0);
                        entity.data.insert("stuck".into(), 0.0);
                        if entity.pos.y < explosion.pos.y {
                            entity.vel.y = -(2.0 + f64::from(game.rng.range(3)));
                        }
                        let speed = 2.0 + f64::from(game.rng.range(3));
                        entity.vel.x = if entity.pos.x < explosion.pos.x {
                            -speed
                        } else {
                            speed
                        };
                    }
                    _ if entity.object != "oRopeThrow" || entity.value("falling") == 0.0 => {
                        entity.vel.y += if entity.pos.y < explosion.pos.y {
                            -6.0
                        } else {
                            6.0
                        };
                        let speed = 4.0 + f64::from(game.rng.range(3));
                        entity.vel.x += if entity.pos.x < explosion.pos.x {
                            -speed
                        } else {
                            speed
                        };
                    }
                    _ => {}
                }
                if entity.held {
                    entity.held = false;
                    game.player.held = None;
                    game.player.pickup_item_type.clear();
                }
            }
            let fragile = !entity.alive && matches!(entity.object.as_str(), "oJar" | "oSkull");
            if let Some(stored) = game
                .world
                .entities
                .iter_mut()
                .find(|stored| stored.id == id)
            {
                *stored = entity;
            }
            // Explosion's MagmaMan receiver calls instance_destroy before
            // later automatic pairs and Bomb End Step can observe its link.
            crate::enemies::finish_actor_destroy(game, id);
            if fragile {
                super::break_item(game, id);
            }
        }
    }
}

/// oExplosion Collision with oBoulder runs before inherited oSolid routing.
/// RubblePiece and Rock Creates are fixed, uncapped and consume no RNG.
fn explosion_boulder(game: &mut Game, boulder: &Entity) {
    for _ in 0..3 {
        let pos = Vec2::new(
            boulder.pos.x + f64::from(game.rng.range(16)) - f64::from(game.rng.range(16)),
            boulder.pos.y + f64::from(game.rng.range(16)) - f64::from(game.rng.range(16)),
        );
        let id = game.spawn("oRubble", pos);
        game.world.entity_mut(id).unwrap().set_sprite("sRubbleTan");
        if game.rng.range(3) == 0 {
            let pos = Vec2::new(
                boulder.pos.x + f64::from(game.rng.range(16)) - f64::from(game.rng.range(16)),
                boulder.pos.y + f64::from(game.rng.range(16)) - f64::from(game.rng.range(16)),
            );
            game.spawn("oRock", pos);
        }
    }
    for _ in 0..6 {
        let pos = Vec2::new(
            boulder.pos.x + f64::from(game.rng.range(16)) - f64::from(game.rng.range(16)),
            boulder.pos.y + f64::from(game.rng.range(16)) - f64::from(game.rng.range(16)),
        );
        let id = game.spawn("oRubbleSmall", pos);
        game.world
            .entity_mut(id)
            .unwrap()
            .set_sprite("sRubbleTanSmall");
    }
    // Literal with other instance_destroy bypasses attack immunity and still
    // invokes the inherited oSolid Destroy continuation after all fragments.
    destroy_solid_self(game, boulder.id);
}

fn explosion_solid_cleanup(game: &mut Game) {
    for entity in &mut game.world.entities {
        if entity.alive && crate::catalog::is_a(&entity.object, "oTreasure") {
            entity.state = 1;
        }
    }
    let unsupported: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|entity| {
            entity.alive
                && entity.object == "oSpikes"
                && !game.world.solid_at(entity.pos.x, entity.pos.y + 16.0)
        })
        .map(|entity| entity.id)
        .collect();
    for id in unsupported {
        if let Some(spikes) = game.world.entity_mut(id) {
            spikes.alive = false;
        }
    }
}

pub(crate) fn animation_end(game: &mut Game, id: u64) {
    if let Some(entity) = game.world.entity_mut(id) {
        if entity.object == "oExplosion" {
            entity.alive = false;
        }
    }
}

pub(crate) fn end_instance(game: &mut Game, id: u64) {
    let Some(bomb) = game.world.entity(id).filter(|e| e.object == "oBomb") else {
        return;
    };
    let target = bomb.value("attached_id") as u64;
    if target == 0 {
        return;
    }
    let offset = Vec2::new(bomb.value("sticky_x"), bomb.value("sticky_y"));
    let target_pos = game.world.entity(target).map(|entity| entity.pos);
    let bomb = game.world.entity_mut(id).unwrap();
    if let Some(pos) = target_pos {
        bomb.pos = Vec2::new(pos.x - offset.x, pos.y - offset.y);
    } else {
        bomb.data.insert("attached_id".into(), 0.0);
    }
}

/// Uses stable IDs so entities spawned during a tick do not update early.
pub fn tick(game: &mut Game) {
    player_explosion_hurt(game);
    tick_entities(game);
}

/// Legacy family loop without the separately dispatched Player Step callback.
pub(crate) fn tick_entities(game: &mut Game) {
    let ids: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|entity| {
            entity.alive
                && matches!(
                    entity.object.as_str(),
                    "oBomb"
                        | "oFireFrogBomb"
                        | "oRopeThrow"
                        | "oExplosion"
                        | "oRope"
                        | "oRopeTop"
                        | "oRopeBurn"
                )
        })
        .map(|entity| entity.id)
        .collect();
    for id in ids {
        let Some(mut entity) = game.world.entity(id).cloned() else {
            continue;
        };
        init_entity(&mut entity);
        entity
            .data
            .insert("simulation_tick".into(), game.engine_time() as f64);
        match entity.object.as_str() {
            "oBomb" | "oFireFrogBomb" => {
                let normal_step = tick_bomb(game, &mut entity);
                warn_shopkeeper(game, &entity);
                if normal_step && entity.object == "oFireFrogBomb" {
                    fire_frog_bomb_tail(game, &mut entity);
                }
            }
            "oRopeThrow" => tick_rope(game, &mut entity),
            "oExplosion" => {
                announce_explosion(game, &mut entity);
                apply_explosion(game, &entity);
                entity.timer = entity.timer.saturating_sub(1);
                entity.animation += 0.8;
                if entity.timer == 0 {
                    entity.alive = false;
                }
            }
            "oRope" | "oRopeTop" => tick_rope_burn(game, &mut entity),
            "oRopeBurn" => {
                tick_burning_flame(game, &mut entity);
                entity.animation += 0.8;
            }
            _ => unreachable!(),
        }
        if let Some(stored) = game
            .world
            .entities
            .iter_mut()
            .find(|stored| stored.id == id)
        {
            *stored = entity;
        }
        destroy_bomb(game, id);
    }
}

/// oBomb End Step follows its enemy after ordinary enemy movement has finished.
pub fn end_tick(game: &mut Game) {
    let bombs: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|entity| {
            entity.alive && entity.object == "oBomb" && entity.value("attached_id") != 0.0
        })
        .map(|entity| {
            (
                entity.id,
                entity.value("attached_id") as u64,
                entity.value("sticky_x"),
                entity.value("sticky_y"),
            )
        })
        .collect();
    for (id, target, dx, dy) in bombs {
        let target_pos = game.world.entity(target).map(|entity| entity.pos);
        if let Some(bomb) = game.world.entity_mut(id) {
            if let Some(pos) = target_pos {
                bomb.pos = Vec2::new(pos.x - dx, pos.y - dy);
            } else {
                bomb.data.insert("attached_id".into(), 0.0);
            }
        }
    }
}

fn tick_bomb(game: &mut Game, bomb: &mut Entity) -> bool {
    if bomb.object == "oFireFrogBomb" || bomb.value("armed") != 0.0 {
        bomb.timer = bomb.timer.saturating_sub(1);
        bomb.animation += if bomb.timer <= BOMB_FAST_FUSE {
            1.0
        } else {
            0.2
        };
        if bomb.timer == 0 {
            bomb.alive = false;
            if let Some(stored) = game.world.entity_mut(bomb.id) {
                stored.alive = false;
            }
            if game.player.held == Some(bomb.id) {
                game.player.held = None;
            }
            explode(game, bomb.pos);
            return false;
        }
    }
    step_bomb_body(game, bomb);
    true
}

fn step_bomb_body(game: &mut Game, bomb: &mut Entity) {
    if !super::item_body_active(game, bomb) {
        return;
    }
    if bomb.held {
        if bomb.object == "oFireFrogBomb" {
            super::align_held(game, bomb);
        } else {
            bomb.pos = Vec2::new(
                game.player.pos.x + 4.0 * f64::from(game.player.facing),
                game.player.pos.y + 2.0,
            );
        }
        bomb.vel = Vec2::default();
        return;
    }
    // Attachment is an oBomb End Step position override, not an oItem Step
    // guard. Retain ordinary motion, gravity and environment checks here.
    if !projectile_motion(
        &game.world,
        bomb,
        bomb.value("sticky") != 0.0 && bomb.value("armed") != 0.0,
    ) {
        return;
    }
    // oItem Step sinks in lava before testing the destruction point. Authored
    // lava is an instance, not necessarily a tile in the generated terrain.
    if super::lava_at(
        game,
        Rect::new(bomb.pos.x - 3.0, bomb.pos.y - 3.0, 7.0, 7.0),
    ) {
        bomb.data.insert("gravity".into(), 0.0);
        bomb.vel = Vec2::default();
        bomb.pos.y += 0.05;
    } else {
        bomb.data.insert("gravity".into(), 0.6);
    }
    if super::lava_at(game, Rect::new(bomb.pos.x, bomb.pos.y - 5.0, 1.0, 1.0)) {
        bomb.alive = false;
        if let Some(stored) = game.world.entity_mut(bomb.id) {
            stored.alive = false;
        }
        if bomb.object == "oBomb" {
            explode(game, bomb.pos);
        }
        return;
    }
    if bomb.object == "oBomb"
        && bomb.value("sticky") != 0.0
        && bomb.value("attached_id") == 0.0
        && (bomb.vel.x.abs() > 2.0 || bomb.vel.y.abs() > 2.0)
    {
        if let Some((id, pos)) = sticky_target(game, bomb.pos) {
            bomb.data.insert("attached_id".into(), id as f64);
            bomb.data.insert("sticky_x".into(), pos.x - bomb.pos.x);
            bomb.data.insert("sticky_y".into(), pos.y - bomb.pos.y);
            // oItem Step248/266 overwrites the backlink. The earlier bomb is
            // deliberately left attached: the one-bomb restriction is commented out.
            game.world
                .entity_mut(id)
                .unwrap()
                .data
                .insert("bomb_id".into(), bomb.id as f64);
        }
    }
}

// oItem Step234-268: contact only gates the separate Enemy nearest query.
// Source rectangle endpoints are inclusive; Damsel is a later fallback.
fn sticky_target(game: &mut Game, pos: Vec2) -> Option<(u64, Vec2)> {
    use crate::dispatcher::Instance;
    crate::dispatcher::register_instances(game);
    let contact = |e: &Entity| {
        e.collision_mask().intersects_rectangle_bbox(
            Vec2::new(pos.x - 2.0, pos.y - 2.0),
            Vec2::new(pos.x + 2.0, pos.y + 2.0),
        )
    };
    let enemies: Vec<_> = crate::dispatcher::descendant_instances(game, "oEnemy")
        .into_iter()
        .filter_map(|i| match i {
            Instance::Entity(id) => game.world.entity(id),
            _ => None,
        })
        .filter(|e| e.engine_active)
        .collect();
    if enemies.iter().any(|e| contact(e)) {
        let mut nearest = None;
        let mut distance = f64::INFINITY;
        for e in enemies {
            let candidate = e.pos.distance(pos);
            if candidate < distance {
                nearest = Some((e.id, e.pos));
                distance = candidate;
            }
        }
        return nearest;
    }
    crate::dispatcher::descendant_instances(game, "oDamsel")
        .into_iter()
        .filter_map(|i| match i {
            Instance::Entity(id) => game.world.entity(id),
            _ => None,
        })
        .find(|e| e.engine_active && contact(e))
        .map(|e| (e.id, e.pos))
}

/// Literal oBomb Destroy. It clears the receiver unconditionally, even when
/// another live bomb has since overwritten that receiver's bombID.
pub(crate) fn destroy_bomb(game: &mut Game, id: u64) {
    let Some(bomb) = game
        .world
        .entities
        .iter_mut()
        .find(|e| e.id == id && e.object == "oBomb")
    else {
        return;
    };
    if bomb.alive || bomb.value("bomb_destroyed") != 0.0 {
        return;
    }
    bomb.data.insert("bomb_destroyed".into(), 1.0);
    let target = bomb.value("attached_id") as u64;
    if let Some(receiver) = game.world.entity_mut(target) {
        receiver.data.insert("bomb_id".into(), 0.0);
    }
}

/// oEnemy Destroy and Damsel's own inherited Item/own Destroy chain.
/// Scarab replaces Enemy Destroy without action_inherited.
pub(crate) fn sticky_receiver_destroyed(game: &mut Game, receiver: &mut Entity) {
    if receiver.alive
        || receiver.value("sticky_destroyed") != 0.0
        || !(crate::enemies::is_enemy(&receiver.object) && receiver.object != "oScarab"
            || receiver.object == "oDamsel")
    {
        return;
    }
    receiver.data.insert("sticky_destroyed".into(), 1.0);
    if let Some(bomb) = game.world.entity_mut(receiver.value("bomb_id") as u64) {
        bomb.data.insert("attached_id".into(), 0.0);
    }
}

/// Fallback for cross-system retirements; normal Bomb Alarm runs synchronously.
pub(crate) fn finish_sticky_destroy_events(game: &mut Game) {
    let ids: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| !e.alive && e.object == "oBomb" && e.value("bomb_destroyed") == 0.0)
        .map(|e| e.id)
        .collect();
    for id in ids {
        destroy_bomb(game, id);
    }
}

/// oFireFrogBomb's own event follows the complete inherited Item event, even
/// when the inherited event has already retired this receiver.
pub(super) fn fire_frog_bomb_own_step(game: &mut Game, id: u64) {
    let Some(mut bomb) = game
        .world
        .entities
        .iter()
        .find(|entity| entity.id == id && entity.object == "oFireFrogBomb")
        .cloned()
    else {
        return;
    };
    fire_frog_bomb_tail(game, &mut bomb);
    if let Some(stored) = game
        .world
        .entities
        .iter_mut()
        .find(|entity| entity.id == id)
    {
        *stored = bomb;
    }
}

fn fire_frog_bomb_tail(game: &mut Game, bomb: &mut Entity) {
    if bomb.value("armed") != 0.0 {
        crate::dispatcher::register_instances(game);
        let keepers: Vec<_> = crate::dispatcher::descendant_instances(game, "oShopkeeper")
            .into_iter()
            .filter_map(|instance| match instance {
                crate::dispatcher::Instance::Entity(id) => game.world.entity(id),
                _ => None,
            })
            .filter(|keeper| keeper.engine_active)
            .map(|keeper| (keeper.pos, keeper.collision_mask().point_bounds()))
            .collect();
        let bounds = bomb.collision_mask().point_bounds();
        if !keepers.is_empty()
            && crate::economy::in_shop(game, bomb.pos)
            && keepers
                .iter()
                .any(|(_, keeper)| crate::world_events::mask_distance(bounds, *keeper) < 96.0)
        {
            // `with oShopkeeper` calls the nearest-keeper script from each
            // active receiver, retaining coincident-origin ties and repetitions.
            for (origin, _) in keepers {
                crate::economy::anger(game, origin, "TERRORIST!");
            }
        }
    }
    let water = game.world.tiles.iter().enumerate().any(|(index, water)| {
        water.engine_active
            && crate::catalog::is_a(&water.object, "oWaterSwim")
            && game
                .world
                .tile_collision_mask(
                    (index % game.world.width) as i32,
                    (index / game.world.width) as i32,
                )
                .contains(bomb.pos, CollisionPrecision::Precise)
    }) || game.world.entities.iter().any(|water| {
        water.id != bomb.id
            && water.alive
            && water.engine_active
            && crate::catalog::is_a(&water.object, "oWaterSwim")
            && water.mask_contains(bomb.pos, CollisionPrecision::Precise)
    });
    if water {
        if bomb.value("swimming") == 0.0 {
            game.spawn("oSplash", bomb.pos);
            bomb.data.insert("swimming".into(), 1.0);
            game.sound("sndSplash", bomb.pos);
        }
    } else {
        bomb.data.insert("swimming".into(), 0.0);
    }
}

fn warn_shopkeeper(game: &mut Game, bomb: &Entity) {
    if !bomb.alive || bomb.object != "oBomb" || bomb.value("armed") == 0.0 {
        return;
    }
    let room = |pos: Vec2| {
        (
            (((pos.x - 16.0).max(0.0) / 160.0).floor() as i32).min(3),
            (((pos.y - 16.0).max(0.0) / 128.0).floor() as i32).min(4),
        )
    };
    let distance = |keeper: &Entity| {
        let a = bomb.sprite_bounds();
        let b = keeper.sprite_bounds();
        let dx = (a.x - b.x - b.w + 1.0).max(b.x - a.x - a.w + 1.0).max(0.0);
        let dy = (a.y - b.y - b.h + 1.0).max(b.y - a.y - a.h + 1.0).max(0.0);
        dx.hypot(dy)
    };
    if game.world.entities.iter().any(|keeper| {
        let keeper_room = room(keeper.pos);
        keeper.alive
            && keeper.object == "oShopkeeper"
            && (
                keeper
                    .data
                    .get("shop_room_x")
                    .copied()
                    .unwrap_or(f64::from(keeper_room.0)) as i32,
                keeper
                    .data
                    .get("shop_room_y")
                    .copied()
                    .unwrap_or(f64::from(keeper_room.1)) as i32,
            ) == room(bomb.pos)
            && distance(keeper) < 96.0
    }) {
        crate::economy::anger(game, bomb.pos, "TERRORIST!");
    }
}

fn projectile_motion(world: &World, entity: &mut Entity, sticky: bool) -> bool {
    let frog = entity.object == "oFireFrogBomb";
    let body_at = |pos: Vec2| {
        if frog {
            Rect::new(pos.x - 6.0, pos.y - 4.0, 12.0, 12.0)
        } else {
            Rect::new(pos.x - 4.0, pos.y - 4.0, 8.0, 8.0)
        }
    };
    let contact = |pos, dx: i8, dy: i8| {
        let b = body_at(pos);
        let rect = if dx != 0 {
            Rect::new(
                (if dx < 0 { b.x - 1.0 } else { b.x + b.w }).round_ties_even(),
                b.y.round_ties_even(),
                1.0,
                b.h,
            )
        } else {
            Rect::new(
                b.x.round_ties_even(),
                (if dy < 0 { b.y - 1.0 } else { b.y + b.h }).round_ties_even(),
                b.w,
                1.0,
            )
        };
        super::solid_edge(world, rect, entity.id)
    };
    if world.solid_at_except(entity.pos.x, entity.pos.y, entity.id) {
        let (left, right, bottom, top) = (
            contact(entity.pos, -1, 0),
            contact(entity.pos, 1, 0),
            contact(entity.pos, 0, 1),
            contact(entity.pos, 0, -1),
        );
        // The inherited embedded branch retains its pre-escape probes.
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
        return false;
    }
    // Platform Engine/moveTo.gml converts fractional speeds using global time,
    // then moves every horizontal pixel before moving any vertical pixels.
    let time = entity.value("simulation_tick") as u64;
    let dx = super::displacement(entity.vel.x, time);
    let dy = super::displacement(entity.vel.y, time);
    for _ in 0..dx.unsigned_abs() {
        let next = Vec2::new(entity.pos.x + f64::from(dx.signum()), entity.pos.y);
        let bounds = body_at(entity.pos);
        let probe = Rect::new(
            (if dx > 0 {
                bounds.x + bounds.w
            } else {
                bounds.x - 1.0
            })
            .round_ties_even(),
            (bounds.y + 5.0).round_ties_even(),
            1.0,
            (bounds.y + bounds.h - 1.0).round_ties_even() - (bounds.y + 5.0).round_ties_even()
                + 1.0,
        );
        if super::solid_edge(world, probe, entity.id) {
            break;
        }
        entity.pos = next;
    }
    for _ in 0..dy.unsigned_abs() {
        let next = Vec2::new(entity.pos.x, entity.pos.y + f64::from(dy.signum()));
        let bounds = body_at(entity.pos);
        let probe = Rect::new(
            bounds.x,
            if dy > 0 {
                bounds.y + bounds.h
            } else {
                bounds.y - 1.0
            },
            bounds.w,
            1.0,
        );
        if super::solid_edge(world, probe, entity.id) {
            break;
        }
        entity.pos = next;
    }
    let (left, right, floor) = (
        contact(entity.pos, -1, 0),
        contact(entity.pos, 1, 0),
        contact(entity.pos, 0, 1),
    );
    let side = left || right;
    let ceiling = contact(entity.pos, 0, -1);
    // oItem saves this snapshot immediately after moveTo, before responses.
    for (name, value) in [
        ("colLeft", left),
        ("colRight", right),
        ("colBot", floor),
        ("colTop", ceiling),
    ] {
        entity.data.insert(name.into(), f64::from(value));
    }
    if !side {
        entity.data.insert("stuck".into(), 0.0);
    }
    if !floor {
        let gravity = entity.data.get("gravity").copied().unwrap_or(0.6);
        entity.vel.y = (entity.vel.y + gravity).min(8.0);
    }
    if side || floor || ceiling {
        entity.data.insert("gravity".into(), 0.6);
    }
    if side {
        entity.vel.x *= -0.5;
    }
    if floor {
        entity.vel.y = if entity.vel.y > 1.0 {
            -entity.vel.y * 0.5
        } else {
            0.0
        };
        if entity.vel.x.abs() < 0.1 {
            entity.vel.x = 0.0;
        } else {
            entity.vel.x *= 0.3;
        }
        if entity.vel.y.abs() < 1.0 {
            entity.pos.y -= 1.0;
            if !contact(entity.pos, 0, 1) {
                entity.pos.y += 1.0;
            }
            entity.vel.y = 0.0;
        }
    }
    if sticky {
        if side || floor || ceiling {
            entity.vel = Vec2::default();
            entity.data.insert("stuck".into(), 1.0);
            if floor {
                entity.pos.y += 1.0;
            }
        }
    } else {
        if left && entity.value("stuck") == 0.0 {
            if !right {
                entity.pos.x += 1.0;
            }
        } else if right && entity.value("stuck") == 0.0 {
            entity.pos.x -= 1.0;
        }
        if contact(entity.pos, 0, -1) {
            if entity.vel.y < 0.0 {
                entity.vel.y *= -0.8;
            } else {
                entity.pos.y += 1.0;
            }
            entity.data.insert("gravity".into(), 0.6);
        }
    }
    true
}

fn tick_rope(game: &mut Game, rope: &mut Entity) {
    if rope.held {
        return;
    }
    // oItem's inherited Step always runs for ropes, even outside the view and
    // while the custom rope extension step is suppressing their velocity.
    if projectile_motion(&game.world, rope, false) {
        if super::lava_at(
            game,
            Rect::new(rope.pos.x - 3.0, rope.pos.y - 3.0, 7.0, 7.0),
        ) {
            rope.data.insert("gravity".into(), 0.0);
            rope.vel = Vec2::default();
            rope.pos.y += 0.05;
        } else {
            rope.data.insert("gravity".into(), 0.6);
        }
        if super::lava_at(game, Rect::new(rope.pos.x, rope.pos.y - 5.0, 0.001, 0.001)) {
            rope.alive = false;
            return;
        }
    }
    if rope.value("falling") == 0.0 {
        if rope.value("armed") != 0.0 && rope.vel.y >= 0.0 {
            let snapped = (rope.pos.x / 16.0).round_ties_even() * 16.0;
            let near = if rope.value("px") < snapped {
                snapped - 8.0
            } else {
                snapped + 8.0
            };
            rope.pos.x = if solid_point(&game.world, Vec2::new(near, rope.pos.y)) {
                snapped * 2.0 - near
            } else {
                near
            };
            rope.pos.y = rope.pos.y.round_ties_even();
            rope.vel = Vec2::default();
            rope.data.insert("armed".into(), 0.0);
            rope.data.insert("falling".into(), 1.0);
            spawn_rope_part(game, "oRopeTop", rope.pos);
        }
    }
    if rope.value("falling") != 0.0 {
        rope.vel = Vec2::default();
        rope.pos.y += 8.0;
        let count = rope.value("fall_count") + 1.0;
        rope.data.insert("fall_count".into(), count);
        if count > 16.0
            || super::solid_edge(
                &game.world,
                Rect::new(rope.pos.x - 4.0, rope.pos.y + 4.0, 8.0, 1.0),
                rope.id,
            )
        {
            rope.alive = false;
        } else {
            spawn_rope_part(game, "oRope", Vec2::new(rope.pos.x - 8.0, rope.pos.y));
        }
    }
}

fn spawn_rope_part(game: &mut Game, kind: &str, pos: Vec2) {
    let id = game.spawn(kind, pos);
    init_entity(game.world.entity_mut(id).unwrap());
    // oRope inherits oLadder and has a narrow, original per-instance climb mask.
    // Synthetic full-tile ladders would permit climbing beside the actual rope.
}

fn tick_rope_burn(game: &mut Game, rope: &mut Entity) {
    let burn = rope.value("burn_timer");
    let in_lava = super::lava_at(game, Rect::new(rope.pos.x + 12.0, rope.pos.y, 1.0, 1.0));
    if burn == 0.0 && in_lava && rope.object == "oRope" {
        let id = game.spawn("oRopeBurn", Vec2::new(rope.pos.x + 8.0, rope.pos.y));
        init_entity(game.world.entity_mut(id).unwrap());
    } else if burn > 1.0 {
        rope.data.insert("burn_timer".into(), burn - 1.0);
    } else if burn == 1.0 {
        // oRope Step releases a climber touching the segment being removed.
        // Waiting for a later ladder query leaves the player climbing a rope
        // that has already disappeared during this authoritative frame.
        if game.player.state == PlayerState::Climbing
            && game
                .player
                .sprite_bounds()
                .contains(Vec2::new(rope.pos.x + 12.0, rope.pos.y + 4.0))
        {
            game.player.state = PlayerState::Falling;
        }
        rope.alive = false;
    }
}

fn tick_burning_flame(game: &mut Game, flame: &mut Entity) {
    flame.pos.y -= 1.0;

    let near = Rect::new(flame.pos.x - 1.0, flame.pos.y - 8.0, 2.0, 16.0);
    if solid_point(&game.world, flame.pos)
        || !game.world.entities.iter().any(|rope| {
            rope.alive && rope.object == "oRope" && near.intersects(rope.sprite_bounds())
        })
    {
        flame.alive = false;
        return;
    }
    if let Some(rope) = game.world.entities.iter_mut().find(|rope| {
        rope.alive && rope.object == "oRope" && rope.sprite_bounds().contains(flame.pos)
    }) {
        rope.data.insert("burn_timer".into(), 1.0);
    }
}

fn explode(game: &mut Game, pos: Vec2) {
    let id = game.spawn("oExplosion", pos);
    init_entity(game.world.entity_mut(id).unwrap());
    // Legacy direct fixtures retain immediate contact scanning below. Runtime
    // Create only announces the blast; the dispatcher owns its contact phase.
    let mut explosion = game.world.entity(id).unwrap().clone();
    announce_explosion(game, &mut explosion);
    *game.world.entity_mut(id).unwrap() = explosion.clone();
    if game.dispatcher.phase == crate::dispatcher::Phase::Idle {
        apply_explosion(game, &explosion);
    }
}

fn announce_explosion(game: &mut Game, explosion: &mut Entity) {
    if explosion.value("announced") != 0.0 {
        return;
    }
    explosion.data.insert("announced".into(), 1.0);
    game.events.push(GameEvent::Explosion {
        pos: explosion.pos,
        radius: 24.0,
    });
    game.sound("sndExplosion", explosion.pos);
    crate::camera_shake::request(game, 5.0);
}

/// oPlayer1 Step queries overlap once, then uses the nearest explosion's origin.
/// The latter query intentionally includes explosions that did not overlap.
pub(crate) fn player_explosion_hurt(game: &mut Game) {
    if game.player.is_exiting() {
        return;
    }
    let pos = game.player.pos;
    let probe = Rect::new(pos.x - 8.0, pos.y - 8.0, 17.0, 17.0);
    let explosions = || {
        game.world.entities.iter().filter(|entity| {
            entity.alive
                && entity.engine_active
                && crate::catalog::is_a(&entity.object, "oExplosion")
        })
    };
    if !explosions().any(|entity| entity.mask_intersects(probe, CollisionPrecision::BoundingBox)) {
        return;
    }
    let nearest = explosions()
        .min_by(|a, b| a.pos.distance(pos).total_cmp(&b.pos.distance(pos)))
        .unwrap()
        .pos;
    // This source block bypasses ordinary contact invincibility.
    game.player.health -= 10;
    // Literal source survivor quirk: this cause counter increments after subtraction.
    if game.player.health > 0 && crate::statistics::is_real_level(game) {
        game.lifetime_stats
            .misc_death(crate::statistics::MiscDeath::Explosion);
    }
    game.events.push(GameEvent::Damage { amount: 10 });
    let speed = 4.0 + f64::from(game.rng.range(3));
    game.player.vel = Vec2::new(if pos.x > nearest.x { speed } else { -speed }, -6.0);
    game.player.stun = 100;
    game.player.data.insert("burning".into(), 50.0);
    crate::player::script_player_blood(game, 1);
}

fn apply_explosion(game: &mut Game, explosion: &Entity) {
    let frame = (explosion.animation.floor() as usize).min(9);
    let hits = |bounds| mask_intersects(explosion.pos, frame, bounds);
    let solids_active = game.flags.contains("tutorial") || super::item_body_active(game, explosion);
    let mut solid_collision = false;
    let mut boulders = Vec::new();
    let mut emitters = Vec::new();
    for y in 0..game.world.height {
        for x in 0..game.world.width {
            let bounds = Rect::new(x as f64 * 16.0, y as f64 * 16.0, 16.0, 16.0);
            if solids_active
                && game.world.tiles[y * game.world.width + x].is_solid()
                && hits(bounds)
            {
                solid_collision = true;
                if game.world.tiles[y * game.world.width + x].is_destructible() {
                    destroy_terrain(game, x, y);
                }
            }
        }
    }
    let mut magma_spawns = Vec::new();
    let mut solid_destroys = Vec::new();
    let mut fragile_destroys = Vec::new();
    for entity in &mut game.world.entities {
        if !entity.alive || entity.id == explosion.id || !hits(entity.sprite_bounds()) {
            continue;
        }
        if entity.object == "oBoulder" {
            boulders.push(entity.clone());
            entity.alive = false;
        } else if entity.object == "oBarrierEmitter" {
            emitters.push(entity.id);
            entity.alive = false;
        } else if crate::catalog::is_a(&entity.object, "oSolid") {
            solid_collision |= solids_active;
            // The original Collision with oSolid event applies equally to
            // tiles and instances such as push blocks and thin ice.
            if solids_active && entity.value("invincible") == 0.0 && entity.object != "oHardBlock" {
                solid_destroys.push(entity.id);
            }
        } else if entity.object == "oBomb" {
            // Collision callbacks run on every overlapping frame. In
            // particular an overlapping blast resets this short fuse again.
            entity.sprite = "sBombArmed".into();
            entity.data.insert("armed".into(), 1.0);
            entity.data.insert("attached_id".into(), 0.0);
            entity.data.insert("stuck".into(), 0.0);
            entity.timer = 4 + game.rng.range(5);
            if entity.pos.y < explosion.pos.y {
                entity.vel.y = -(2.0 + f64::from(game.rng.range(3)));
            }
            let horizontal_speed = 2.0 + f64::from(game.rng.range(3));
            entity.vel.x = if entity.pos.x < explosion.pos.x {
                -horizontal_speed
            } else {
                horizontal_speed
            };
        } else if entity.object == "oMagmaMan" {
            entity.alive = false;
            for _ in 0..2 {
                magma_spawns.push(Vec2::new(entity.pos.x + 8.0, entity.pos.y - 4.0));
            }
        } else if (crate::enemies::is_enemy(&entity.object) || entity.object == "oDamsel")
            && entity.value("invincible") == 0.0
        {
            entity.health -= if entity.object == "oDamsel" { 100 } else { 30 };
            let speed = 4.0 + f64::from(game.rng.range(3));
            entity.vel = Vec2::new(
                if entity.pos.x > explosion.pos.x {
                    speed
                } else {
                    -speed
                },
                -6.0,
            );
            entity.data.insert("burning".into(), 50.0);
            if entity.object == "oDamsel" {
                entity.state = 2;
            }
        } else if matches!(
            entity.object.as_str(),
            "oWeb" | "oArrow" | "oFishBone" | "oJar" | "oSkull"
        ) {
            entity.alive = false;
            if matches!(entity.object.as_str(), "oJar" | "oSkull") {
                fragile_destroys.push(entity.id);
            }
        } else if crate::catalog::is_a(&entity.object, "oItem") {
            if entity.object != "oRopeThrow" || entity.value("falling") == 0.0 {
                entity.vel.y += if entity.pos.y < explosion.pos.y {
                    -6.0
                } else {
                    6.0
                };
                let speed = 4.0 + f64::from(game.rng.range(3));
                entity.vel.x += if entity.pos.x < explosion.pos.x {
                    -speed
                } else {
                    speed
                };
            }
        }
        if entity.held && entity.object != "oExplosion" {
            entity.held = false;
            if game.player.held == Some(entity.id) {
                game.player.held = None;
                game.player.pickup_item_type.clear();
            }
        }
    }
    for boulder in boulders {
        explosion_boulder(game, &boulder);
    }
    for id in emitters {
        crate::enemies::destroy_barrier_emitter(game, id);
    }
    for id in fragile_destroys {
        super::break_item(game, id);
    }
    for id in solid_destroys {
        destroy_solid_instance(game, id);
    }
    if solid_collision {
        // This cleanup belongs to the explosion event itself, independently
        // of whether a solid's Destroy event inherits oSolid's event.
        let unsupported: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|entity| {
                entity.alive
                    && entity.object == "oSpikes"
                    && !game.world.solid_at(entity.pos.x, entity.pos.y + 16.0)
            })
            .map(|entity| entity.id)
            .collect();
        for id in unsupported {
            if let Some(spikes) = game.world.entity_mut(id) {
                spikes.alive = false;
            }
        }
    }
    for pos in magma_spawns {
        // instance_create completes the four random oMagma Create draws
        // before this caller's override, then repeats for the next fragment.
        let id = crate::world_events::spawn_detritus(game, "oMagma", pos);
        let velocity_y = -(1.0 + f64::from(game.rng.range(3)));
        // A parent detritus cap may mark the fresh instance dead. Preserve
        // existing cap continuation while still applying the caller's write.
        if let Some(magma) = game
            .world
            .entities
            .iter_mut()
            .find(|entity| entity.id == id)
        {
            magma.vel.y = velocity_y;
        }
    }
}

pub(crate) fn destroy_terrain(game: &mut Game, x: usize, y: usize) {
    destroy_terrain_impl(game, x, y, true);
}

/// A solid's source instance_destroy call bypasses its attack immunity.
/// Empty or non-solid cells have no solid instance Destroy event to dispatch.
pub(crate) fn destroy_terrain_self(game: &mut Game, x: usize, y: usize) -> bool {
    destroy_terrain_impl(game, x, y, false)
}

/// oTree Step destroys itself on lost support, regardless of invincibility.
/// The caller owns its source viewport, support point, and instance order.
pub(crate) fn destroy_unsupported_tree_tile(game: &mut Game, x: usize, y: usize) -> bool {
    if !game
        .world
        .tile(x as i32, y as i32)
        .is_some_and(|tile| crate::catalog::is_a(&tile.object, "oTree"))
    {
        return false;
    }
    destroy_terrain_impl(game, x, y, false)
}

fn destroy_terrain_impl(game: &mut Game, x: usize, y: usize, respect_invincibility: bool) -> bool {
    if x >= game.world.width || y >= game.world.height {
        return false;
    }
    let Some(tile) = game.world.tile(x as i32, y as i32) else {
        return false;
    };
    if !crate::catalog::is_a(&tile.object, "oSolid")
        || (respect_invincibility && tile.object == "oHardBlock")
    {
        return false;
    }
    let object = tile.object.clone();
    let center = Vec2::new(x as f64 * 16.0 + 8.0, y as f64 * 16.0 + 8.0);
    let metadata = game
        .world
        .entities
        .iter()
        .find(|entity| entity.alive && entity.object == "oTerrainMetadata" && entity.pos == center)
        .cloned();
    if respect_invincibility
        && metadata
            .as_ref()
            .is_some_and(|metadata| metadata.value("invincible") != 0.0)
    {
        return false;
    }
    // World::set_tile also serves non-event editing callers. Runtime Destroy
    // requests water only when the actual source callback reaches checkWater.
    let water_was_dirty = game.world.fluid_dirty;
    game.world.set_tile(x, y, "");
    game.world.fluid_dirty = water_was_dirty;
    let mut source = Entity::new(0, &object, Vec2::new(center.x - 8.0, center.y - 8.0));
    if let Some(metadata) = &metadata {
        source.data = metadata.data.clone();
        source.text = metadata.text.clone();
        if let Some(sprite) = metadata.text.get("terrain_sprite") {
            source.sprite = sprite.clone();
        }
    }
    if inherits_solid_destroy(&object) {
        solid_parent_destroy(game, source.pos, false);
    }
    // Generated metadata already records gold released from source sprites.
    // Block/Xoc families additionally have source global-city loot branches.
    let stored_gold =
        source.data.contains_key("gold_chunks") || source.data.contains_key("gold_nuggets");
    solid_destroy_body(game, &source, !stored_gold);
    let outputs_enabled = solid_outputs_enabled(game, &source);
    if let Some(metadata) = metadata {
        game.world.entity_mut(metadata.id).unwrap().alive = false;
        if metadata.value("shopWall") != 0.0
            && !matches!(
                solid_destroy_family(&object),
                "oSpearTrapTop" | "oSpearTrapBottom"
            )
        {
            // SpearTrap own Destroy never inherits Solid's shop-wall anger.
            // Other generated metadata retains its existing crime adapter.
            // Existing generated metadata records shop wall crime explicitly.
            crate::economy::anger(game, center, "DIE, YOU VANDAL!");
            game.flags.insert("shopkeepers_angry".into());
        }
        for (key, object) in [
            ("gold_chunks", "oGoldChunk"),
            ("gold_nuggets", "oGoldNugget"),
        ] {
            for _ in 0..if outputs_enabled {
                metadata.value(key).max(0.0) as usize
            } else {
                0
            } {
                let pos = Vec2::new(
                    center.x + f64::from(game.rng.range(5)) - f64::from(game.rng.range(5)),
                    center.y + f64::from(game.rng.range(5)) - f64::from(game.rng.range(5)),
                );
                let velocity = Vec2::new(
                    f64::from(game.rng.range(4)) - f64::from(game.rng.range(4)),
                    2.0 + f64::from(game.rng.range(3)),
                );
                let id = game.spawn(object, pos);
                game.world.entity_mut(id).unwrap().vel = velocity;
            }
        }
        if metadata.value("embedded_treasure") != 0.0
            && metadata.value("frozen_caveman") == 0.0
            && object != "oXocBlock"
        {
            // Concrete objects represent deferred terrain Create contents.
            // Source treasure labels name own Destroy loot, whose clean guard
            // also applies here (notably Temple's R-template Big Ruby).
            if let Some(treasure) = metadata
                .text
                .get("treasure")
                .filter(|name| outputs_enabled || name.starts_with('o'))
            {
                let object = match treasure.as_str() {
                    "Big Ruby" => "oRubyBig",
                    "Big Sapphire" => "oSapphireBig",
                    "Big Emerald" => "oEmeraldBig",
                    "Ruby" => "oRuby",
                    "Sapphire" => "oSapphire",
                    "Emerald" => "oEmerald",
                    "Diamond" => "oDiamond",
                    name => name,
                };
                let dx = metadata.data.get("treasure_dx").copied().unwrap_or(0.0);
                let dy = metadata.data.get("treasure_dy").copied().unwrap_or(0.0);
                let id = game.spawn(object, Vec2::new(center.x + dx, center.y + dy));
                let child = game.world.entity_mut(id).unwrap();
                if treasure.starts_with('o') && crate::catalog::is_a(object, "oItem") {
                    // Concrete descriptors retain deferred Set2 Create debt;
                    // its producer clears price AFTER the child's constructor.
                    child.data.insert("cost".into(), 0.0);
                    child.data.insert("forSale".into(), 0.0);
                }
                child.state = 1;
            }
        }
        if metadata.value("frozen_caveman") != 0.0 && outputs_enabled {
            let id = game.spawn("oCaveman", Vec2::new(center.x - 8.0, center.y - 8.0));
            crate::enemies::init_entity(game.world.entity_mut(id).unwrap());
            let caveman = game.world.entity_mut(id).unwrap();
            caveman.state = 98;
            caveman.timer = 200;
            caveman.data.insert("invincible".into(), 20.0);
        }
    }
    true
}

// Resolve the nearest source Destroy event, rather than applying a parent's
// debris to descendants whose own event replaces it.
fn solid_destroy_family(object: &str) -> &str {
    let mut family = object;
    while let Some(definition) = crate::catalog::object(family) {
        if matches!(
            family,
            "oSolid"
                | "oBrick"
                | "oBrickSmooth"
                | "oLush"
                | "oTemple"
                | "oDark"
                | "oBlock"
                | "oPushBlock"
                | "oXocBlock"
                | "oIce"
                | "oIceBlock"
                | "oGrave"
                | "oSign"
                | "oTree"
                | "oAltarLeft"
                | "oAltarRight"
                | "oSacAltarLeft"
                | "oAlienShip"
                | "oAlienShipFloor"
        ) || matches!(
            family,
            "oArrowRepeaterR"
                | "oArrowTrapLeft"
                | "oSmashTrapLit"
                | "oArrowRepeaterL"
                | "oDoor"
                | "oSmashTrap"
                | "oTrapBlock"
                | "oCeilingTrap"
                | "oArrowTrapRight"
                | "oSpearTrapTop"
                | "oSpearTrapBottom"
        ) {
            return family;
        }
        family = &definition.parent;
    }
    object
}

fn solid_outputs_enabled(game: &Game, entity: &Entity) -> bool {
    let family = solid_destroy_family(&entity.object);
    if matches!(family, "oGrave" | "oAlienShip" | "oAlienShipFloor") {
        return true; // These own events have no clean-death guard.
    }
    entity.value("cleanDeath") == 0.0
        && (matches!(family, "oBlock" | "oXocBlock")
            || (!game.flags.contains("cleanSolids")
                && game.data.get("cleanSolids").copied().unwrap_or(0.0) == 0.0))
}

fn solid_destroy_body(game: &mut Game, entity: &Entity, release_gold: bool) {
    let family = solid_destroy_family(&entity.object);
    // This proxy field is source instance state. Synchronous rubble Creates
    // may retire a cleared terrain registration and its bookkeeping map, so
    // preserve fired before the first nested callback.
    let arrow_fired = matches!(family, "oArrowTrapLeft" | "oArrowTrapRight").then(|| {
        let key = if entity.id == 0 {
            let x = (entity.pos.x / 16.0).floor() as usize;
            let y = (entity.pos.y / 16.0).floor() as usize;
            format!("hazard:{}:fired", y * game.world.width + x)
        } else {
            format!("hazard_entity:{}:fired", entity.id)
        };
        game.data
            .get(&key)
            .copied()
            .unwrap_or(entity.value("fired"))
            != 0.0
    });
    if family == "oXocBlock" {
        // tile_layer_find(99,x,y) removes the individual foreground mural
        // fragment, independently from cleanDeath and the loot branch.
        remove_xoc_fragment(&mut game.world, entity.pos);
    }
    if family == "oDoor" {
        // Door owns an unguarded two-small-piece event, with a narrower
        // horizontal range than the guarded tan rubble families below.
        for _ in 0..2 {
            let pos = Vec2::new(
                entity.pos.x + 8.0 + f64::from(game.rng.range(5)) - f64::from(game.rng.range(5)),
                entity.pos.y + 8.0 + f64::from(game.rng.range(9)) - f64::from(game.rng.range(9)),
            );
            let id = game.spawn("oRubbleSmall", pos);
            game.world
                .entity_mut(id)
                .unwrap()
                .set_sprite("sRubbleTanSmall");
        }
        return;
    }
    if !solid_outputs_enabled(game, entity) {
        if family == "oSacAltarLeft" {
            crate::traps::altar::destroyed(game, entity);
        }
        if matches!(family, "oSpearTrapTop" | "oSpearTrapBottom") {
            // Their own non-inheriting Destroy writes this outside its clean
            // output guard, unlike other trap/block Destroy overrides.
            game.world.fluid_dirty = true;
        }
        return;
    }
    let city = game.flags.contains("city_of_gold") || game.flags.contains("cityOfGold");
    let center = Vec2::new(entity.pos.x + 8.0, entity.pos.y + 8.0);
    let rubble_style = match family {
        "oBrick" | "oTree" => Some(("sRubble", "sRubbleSmall")),
        "oBrickSmooth" | "oGrave" | "oSign" | "oAltarLeft" | "oAltarRight" | "oSacAltarLeft"
        | "oArrowTrapLeft" | "oArrowTrapRight" | "oTrapBlock" | "oSpearTrapTop"
        | "oSpearTrapBottom" | "oSmashTrap" | "oSmashTrapLit" | "oCeilingTrap"
        | "oArrowRepeaterL" | "oArrowRepeaterR" => Some(("sRubbleTan", "sRubbleTanSmall")),
        "oLush" | "oTemple" => Some(("sRubbleLush", "sRubbleLushSmall")),
        "oBlock" | "oPushBlock" | "oXocBlock" if !city => Some(("sRubbleLush", "sRubbleLushSmall")),
        _ => None,
    };
    if let Some((large, small)) = rubble_style {
        for (object, sprite) in [
            ("oRubble", large),
            ("oRubbleSmall", small),
            ("oRubbleSmall", small),
        ] {
            let pos = Vec2::new(
                center.x + f64::from(game.rng.range(9)) - f64::from(game.rng.range(9)),
                center.y + f64::from(game.rng.range(9)) - f64::from(game.rng.range(9)),
            );
            let id = game.spawn(object, pos);
            game.world.entity_mut(id).unwrap().sprite = sprite.into();
        }
    }
    if family == "oTrapBlock" && entity.value("dying") != 0.0 {
        game.sound("sndThump", entity.pos);
        crate::camera_shake::request(game, 10.0);
    }
    if family == "oSacAltarLeft" {
        // Defilement follows rubble, outside the clean-death output guard.
        crate::traps::altar::destroyed(game, entity);
        return;
    }
    if matches!(family, "oIce" | "oIceBlock") {
        for _ in 0..3 {
            let pos = Vec2::new(
                entity.pos.x + f64::from(game.rng.range(17)),
                entity.pos.y + f64::from(game.rng.range(17)),
            );
            game.spawn("oDrip", pos);
        }
    }
    if matches!(family, "oDark" | "oAlienShip" | "oAlienShipFloor") {
        for object in ["oRubbleDark", "oRubbleDarkSmall", "oRubbleDarkSmall"] {
            // Preserve the source's literal rand(0-8), not rand(0,8).
            // Its omitted argument1 is zero, so rand(-8) evaluates rand(-8,0).
            let pos = Vec2::new(
                center.x + f64::from(game.rng.range(9)) - (f64::from(game.rng.range(9)) - 8.0),
                center.y + f64::from(game.rng.range(9)) - (f64::from(game.rng.range(9)) - 8.0),
            );
            game.spawn(object, pos);
        }
    }
    if arrow_fired == Some(false) {
        game.spawn("oArrow", center);
    }
    if release_gold {
        let gold_sprite = matches!(
            entity.sprite.as_str(),
            "sBrickGold" | "sLushGold" | "sDarkGold" | "sTempleGold"
        );
        let big_gold_sprite = matches!(
            entity.sprite.as_str(),
            "sBrickGoldBig" | "sLushGoldBig" | "sDarkGoldBig" | "sTempleGoldBig"
        );
        let (chunks, nuggets) = if matches!(family, "oBlock" | "oPushBlock" | "oXocBlock") && city {
            (3, 1)
        } else if matches!(family, "oBrick" | "oLush" | "oDark" | "oTemple") && gold_sprite {
            (3, 0)
        } else if matches!(family, "oBrick" | "oLush" | "oDark" | "oTemple")
            && (big_gold_sprite || (family == "oTemple" && city))
        {
            (3, 1)
        } else {
            (0, 0)
        };
        for (object, count) in [("oGoldChunk", chunks), ("oGoldNugget", nuggets)] {
            for _ in 0..count {
                let pos = Vec2::new(
                    center.x + f64::from(game.rng.range(5)) - f64::from(game.rng.range(5)),
                    center.y + f64::from(game.rng.range(5)) - f64::from(game.rng.range(5)),
                );
                let vel = Vec2::new(
                    f64::from(game.rng.range(4)) - f64::from(game.rng.range(4)),
                    2.0 + f64::from(game.rng.range(3)),
                );
                let id = game.spawn(object, pos);
                game.world.entity_mut(id).unwrap().vel = vel;
            }
        }
    }
    if family == "oXocBlock" && city {
        let treasure = entity
            .text
            .get("treasure")
            .map(String::as_str)
            .unwrap_or("");
        let gem = match treasure {
            "Diamond" | "oDiamond" => Some("oDiamond"),
            "Ruby" | "oRubyBig" => Some("oRubyBig"),
            "Sapphire" | "oSapphireBig" => Some("oSapphireBig"),
            "Emerald" | "oEmeraldBig" => Some("oEmeraldBig"),
            _ => None,
        };
        if let Some(gem) = gem {
            for _ in 0..2 {
                game.spawn(gem, center);
            }
        }
    }
    if family == "oTemple"
        && entity.id != 0
        && entity
            .text
            .get("treasure")
            .is_some_and(|treasure| treasure == "Big Ruby")
    {
        game.spawn("oRubyBig", center);
    }
    if matches!(family, "oSpearTrapTop" | "oSpearTrapBottom") {
        game.world.fluid_dirty = true;
    }
}

// Objects with their own non-inheriting Destroy events stop the event chain.
// Derived from the official Destroy.xml action_inherited entries.
fn inherits_solid_destroy(object: &str) -> bool {
    let mut name = object;
    while let Some(def) = crate::catalog::object(name) {
        if name == "oSolid" {
            return true;
        }
        if matches!(
            name,
            "oArrowRepeaterR"
                | "oArrowTrapLeft"
                | "oSmashTrapLit"
                | "oArrowRepeaterL"
                | "oDoor"
                | "oSmashTrap"
                | "oTrapBlock"
                | "oCeilingTrap"
                | "oArrowTrapRight"
                | "oSpearTrapTop"
                | "oSpearTrapBottom"
                | "oGrave"
                | "oIceBlock"
                | "oSign"
                | "oBlock"
                | "oAltarRight"
                | "oIce"
                | "oXocBlock"
                | "oPushBlock"
                | "oAltarLeft"
                | "oSacAltarLeft"
        ) {
            return false;
        }
        name = &def.parent;
    }
    false
}

fn remove_xoc_fragment(world: &mut World, origin: Vec2) {
    if let Some(tile) = world.entities.iter_mut().find(|tile| {
        tile.alive
            && tile.object == "oGenerationDecoration"
            && tile.sprite == "bgLadyXoc"
            && tile.value("depth") == 99.0
            && tile.pos == origin
    }) {
        tile.alive = false;
    }
}

// oSolid Destroy first gates on a literal collision_point, then selects ONE
// instance_place target using the destroying receiver's translated full mask.
// These Temple fixtures are independently stored entities, not terrain cells.
fn temple_parent_attachment(
    world: &World,
    source: &Entity,
    family: &str,
    gate: Vec2,
    translated_origin: Vec2,
) -> Option<u64> {
    let targets = || {
        world
            .entities
            .iter()
            .filter(|entity| entity.alive && crate::catalog::is_a(&entity.object, family))
    };
    if !targets().any(|entity| entity.mask_contains(gate, CollisionPrecision::BoundingBox)) {
        return None;
    }
    let mut translated = source.clone();
    translated.pos = translated_origin;
    let mask = translated.collision_mask();
    targets()
        .find(|entity| mask.intersects_mask(&entity.collision_mask()))
        .map(|entity| entity.id)
}

/// Literal non-inheriting oBlock Destroy body for generation replacements.
/// This does not widen the supported Temple-primary callback families. City
/// state is the generator's normalized world name, selected from either flag
/// alias before population. global.cleanSolids is absent from this own event.
pub(crate) fn destroy_generated_block_literal(
    world: &mut World,
    rng: &mut Rng,
    source: &Entity,
    city: bool,
) {
    assert_eq!(source.object, "oBlock");
    let center = Vec2::new(source.pos.x + 8.0, source.pos.y + 8.0);
    if source.value("cleanDeath") == 0.0 {
        if !city {
            for (object, sprite) in [
                ("oRubble", "sRubbleLush"),
                ("oRubbleSmall", "sRubbleLushSmall"),
                ("oRubbleSmall", "sRubbleLushSmall"),
            ] {
                let pos = Vec2::new(
                    center.x + f64::from(rng.range(9)) - f64::from(rng.range(9)),
                    center.y + f64::from(rng.range(9)) - f64::from(rng.range(9)),
                );
                // RubblePiece/Create has fixed defaults and no RNG. Its local
                // Create and the caller's sprite assignment finish before the
                // next fragment's coordinate draws.
                let id = crate::creation::spawn_generated_entity(world, rng, object, pos);
                if let Some(rubble) = world.entity_mut(id) {
                    rubble.set_sprite(sprite);
                }
            }
        } else {
            for object in ["oGoldChunk", "oGoldChunk", "oGoldChunk", "oGoldNugget"] {
                let pos = Vec2::new(
                    center.x + f64::from(rng.range(5)) - f64::from(rng.range(5)),
                    center.y + f64::from(rng.range(5)) - f64::from(rng.range(5)),
                );
                // Source creates each collectible before launch RNG/overrides.
                let id = crate::creation::spawn_generated_entity(world, rng, object, pos);
                let velocity = Vec2::new(
                    f64::from(rng.range(4)) - f64::from(rng.range(4)),
                    2.0 + f64::from(rng.range(3)),
                );
                if let Some(gold) = world.entity_mut(id) {
                    gold.vel = velocity;
                }
            }
        }
    }
}

/// Arrow replacement retains the existing deferred-content compatibility
/// adapter after the literal Block event. SetupWalls promotion uses only the
/// literal event: Block Create never produces buried item contents.
pub(crate) fn destroy_generated_block_own(
    world: &mut World,
    rng: &mut Rng,
    source: &Entity,
    city: bool,
) {
    destroy_generated_block_literal(world, rng, source, city);
    let center = Vec2::new(source.pos.x + 8.0, source.pos.y + 8.0);
    // Block owns no treasure-label loot. These concrete contents are the
    // port's existing deferred Create representation; retaining them does not
    // repair their earlier birth timing. Frozen occupants have a separate
    // adapter and must not be materialized a second time here.
    if source.value("embedded_treasure") != 0.0 && source.value("frozen_caveman") == 0.0 {
        if let Some(object) = source
            .text
            .get("treasure")
            .filter(|name| name.starts_with('o'))
        {
            let pos = Vec2::new(
                center.x + source.value("treasure_dx"),
                center.y + source.value("treasure_dy"),
            );
            // New descriptors preserve the real generation-time snapshot;
            // older serialized records use the explicit fresh-run fallback.
            let id = crate::creation::spawn_generated_terrain_child(
                world,
                rng,
                object,
                pos,
                source.value("create_has_spectacles") != 0.0,
            );
            if let Some(item) = world.entity_mut(id) {
                if crate::catalog::is_a(object, "oItem") {
                    item.data.insert("cost".into(), 0.0);
                    item.data.insert("forSale".into(), 0.0);
                }
                item.state = 1;
            }
        }
    }
}

/// The clean Destroy callbacks reachable from scrEntityGen's Temple support
/// replacement. The caller captures the complete receiver before retirement,
/// restores the pre-removal fluid flag, then invokes this callback body.
///
/// This intentionally does not replace general Game destruction. Stock Temple
/// templates place shop walls/lamps only in excluded shop rooms, never create
/// Graves/TikiTorches, and type-exclude Trees/Altars. The sole Door template has
/// guaranteed support at y-32, failing the replacement predicate. Those Game-
/// dependent or unguarded callbacks require the runtime dispatcher instead.
pub(crate) fn destroy_temple_support_clean(world: &mut World, rng: &mut Rng, source: &Entity) {
    let family = solid_destroy_family(&source.object);
    assert!(
        matches!(
            family,
            "oSolid"
                | "oTemple"
                | "oLush"
                | "oBlock"
                | "oPushBlock"
                | "oXocBlock"
                | "oTrapBlock"
                | "oCeilingTrap"
        ),
        "unsupported Temple clean Destroy family: {}",
        source.object
    );
    assert_eq!(
        source.value("shopWall"),
        0.0,
        "Temple replacement excludes shop rooms"
    );
    assert!(
        !source
            .text
            .get("type")
            .is_some_and(|kind| kind == "Tree" || kind == "Altar")
    );
    assert_ne!(source.value("cleanDeath"), 0.0);
    let inherited = inherits_solid_destroy(&source.object);
    let upper = Vec2::new(source.pos.x + 8.0, source.pos.y - 1.0);
    let lower = Vec2::new(source.pos.x + 8.0, source.pos.y + 18.0);
    // Preflight unsupported callbacks before any local effect. These cannot
    // occur in source-authored Temple generation; arbitrary worlds must use
    // the full Game dispatcher, including grave debris and Lamp Item Create.
    if inherited {
        for (family, point) in [("oGrave", upper), ("oLampRed", lower), ("oLamp", lower)] {
            assert!(
                !world.tiles_point(point, family)
                    && !world.entities.iter().any(|entity| entity.alive
                        && crate::catalog::is_a(&entity.object, family)
                        && entity.mask_contains(point, CollisionPrecision::BoundingBox)),
                "unsupported Temple clean Destroy attachment: {family}"
            );
        }
        for family in ["oSpikes", "oTikiTorch"] {
            if let Some(id) = temple_parent_attachment(world, source, family, upper, upper) {
                world.entity_mut(id).unwrap().alive = false;
            }
        }
        // Own non-inheriting clean events do not request a water check.
        world.fluid_dirty = true;
    }
    if family == "oXocBlock" {
        remove_xoc_fragment(world, source.pos);
    }
    // These metadata contents represent objects already born in terrain
    // Create, rather than this clean Destroy's suppressed gold/treasure loot.
    // Materializing their deferred representation preserves contents, but
    // does not repair the port's existing original-birth-order timing gap.
    // Xoc mural metadata and R-template Big Ruby use the same flag for OWN
    // Destroy loot, which cleanDeath suppresses. Actual deferred terrain Create
    // contents use concrete object names (oRubyBig, equipment, etc.), whereas
    // those template assignments retain source treasure labels.
    if family != "oXocBlock"
        && source.value("embedded_treasure") != 0.0
        && source.value("frozen_caveman") == 0.0
    {
        if let Some(object) = source
            .text
            .get("treasure")
            .filter(|name| name.starts_with('o'))
        {
            let pos = Vec2::new(
                source.pos.x + 8.0 + source.value("treasure_dx"),
                source.pos.y + 8.0 + source.value("treasure_dy"),
            );
            let id = crate::creation::spawn_generated_terrain_child(
                world,
                rng,
                object,
                pos,
                source.value("create_has_spectacles") != 0.0,
            );
            let child = world.entity_mut(id).unwrap();
            if crate::catalog::is_a(object, "oItem") {
                child.data.insert("cost".into(), 0.0);
                child.data.insert("forSale".into(), 0.0);
            }
            child.state = 1;
        }
    }
}

fn solid_parent_destroy(game: &mut Game, pos: Vec2, shop_wall: bool) {
    if shop_wall {
        crate::economy::anger(game, pos, "DIE, YOU VANDAL!");
        game.flags.insert("shopkeepers_angry".into());
    }
    let upper = Vec2::new(pos.x + 8.0, pos.y - 1.0);
    let lower = Vec2::new(pos.x + 8.0, pos.y + 18.0);
    let mut lamps = Vec::new();
    let mut graves = Vec::new();
    for entity in &mut game.world.entities {
        if !entity.alive {
            continue;
        }
        if matches!(entity.object.as_str(), "oSpikes" | "oTikiTorch" | "oGrave")
            && entity.sprite_bounds().contains(upper)
        {
            if entity.object == "oGrave" {
                graves.push(entity.clone());
                entity.data.insert("destroy_done".into(), 1.0);
            }
            entity.alive = false;
        } else if matches!(entity.object.as_str(), "oLamp" | "oLampRed")
            && entity.sprite_bounds().contains(lower)
        {
            lamps.push((
                if entity.object == "oLampRed" {
                    "oLampRedItem"
                } else {
                    "oLampItem"
                },
                Vec2::new(entity.pos.x + 8.0, entity.pos.y + 12.0),
            ));
            entity.alive = false;
        }
    }
    for grave in graves {
        solid_destroy_body(game, &grave, true);
    }
    for (object, pos) in lamps {
        game.spawn(object, pos);
    }
    // oSolid Destroy.xml:50 follows attachment callbacks and runs regardless
    // of the child event's cleanDeath/global.cleanSolids loot guards.
    game.world.fluid_dirty = true;
}

#[cfg(test)]
#[path = "explosives_fluid_tests.rs"]
mod fluid_invalidation_tests;

/// Execute gameplay Destroy effects once for an independently stored solid.
/// Used by explosions and mattocks; target invincibility is respected here.
pub(crate) fn destroy_solid_instance(game: &mut Game, id: u64) -> bool {
    destroy_solid_instance_impl(game, id, true)
}

/// Dispatch an instance's own source destruction without attack guards.
/// destroy_done still prevents repeating the same instance's callbacks.
pub(crate) fn destroy_solid_self(game: &mut Game, id: u64) -> bool {
    destroy_solid_instance_impl(game, id, false)
}

/// oTree's own lost-support destruction bypasses the attack immunity guard.
pub(crate) fn destroy_unsupported_tree_instance(game: &mut Game, id: u64) -> bool {
    if !game
        .world
        .entity(id)
        .is_some_and(|entity| crate::catalog::is_a(&entity.object, "oTree"))
    {
        return false;
    }
    destroy_solid_instance_impl(game, id, false)
}

fn destroy_solid_instance_impl(game: &mut Game, id: u64, respect_invincibility: bool) -> bool {
    let Some(entity) = game
        .world
        .entities
        .iter()
        .find(|entity| entity.id == id)
        .cloned()
    else {
        return false;
    };
    if !crate::catalog::is_a(&entity.object, "oSolid")
        || entity.value("destroy_done") != 0.0
        || (respect_invincibility
            && (entity.value("invincible") != 0.0 || entity.object == "oHardBlock"))
    {
        return false;
    }
    let stored = game
        .world
        .entities
        .iter_mut()
        .find(|entity| entity.id == id)
        .unwrap();
    stored.alive = false;
    stored.data.insert("destroy_done".into(), 1.0);
    if inherits_solid_destroy(&entity.object) {
        solid_parent_destroy(game, entity.pos, entity.value("shopWall") != 0.0);
    }
    solid_destroy_body(game, &entity, true);
    true
}

/// The source uses per-frame precise alpha collision masks with origin (24,24).
/// Rows below are derived from the official sExplosion images, alpha > 0.
fn mask_intersects(pos: Vec2, frame: usize, rect: Rect) -> bool {
    let left = pos.x.round() - 24.0;
    let top = pos.y.round() - 24.0;
    let x0 = ((rect.x - left).floor() as i32).clamp(0, 48);
    let y0 = ((rect.y - top).floor() as i32).clamp(0, 48);
    let x1 = ((rect.x + rect.w - left).ceil() as i32).clamp(0, 48);
    let y1 = ((rect.y + rect.h - top).ceil() as i32).clamp(0, 48);
    if x1 <= x0 || y1 <= y0 {
        return false;
    }
    let columns = ((1u64 << x1) - 1) & !((1u64 << x0) - 1);
    EXPLOSION_MASKS[frame][y0 as usize..y1 as usize]
        .iter()
        .any(|row| row & columns != 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn game() -> Game {
        let mut g = Game::shell(7, RulesPreset::Classic);
        g.player.pos = Vec2::new(160.0, 160.0);
        g
    }

    fn temple_clean_source(object: &str) -> Entity {
        let mut source = Entity::new(0, object, Vec2::new(80.0, 80.0));
        source.data.insert("cleanDeath".into(), 1.0);
        source
    }

    #[test]
    fn temple_clean_destroy_parent_water_check_follows_source_inheritance() {
        for object in [
            "oSolid",
            "oTemple",
            "oLush",
            "oBlock",
            "oPushBlock",
            "oXocBlock",
            "oTrapBlock",
            "oCeilingTrap",
        ] {
            for already_dirty in [false, true] {
                let mut world = World::new(16, 16, Area::Temple, 13);
                world.fluid_dirty = already_dirty;
                let mut rng = Rng::new(71);
                let mut expected = rng.clone();
                destroy_temple_support_clean(&mut world, &mut rng, &temple_clean_source(object));
                assert_eq!(
                    world.fluid_dirty,
                    already_dirty || matches!(object, "oSolid" | "oTemple" | "oLush"),
                    "{object}"
                );
                assert_eq!(rng.range(1_000_000), expected.range(1_000_000), "{object}");
                assert!(world.entities.is_empty());
            }
        }
    }

    #[test]
    fn temple_clean_destroy_preserves_buried_content_without_own_destroy_loot() {
        let mut world = World::new(16, 16, Area::Temple, 13);
        let mut source = temple_clean_source("oTemple");
        source.data.insert("embedded_treasure".into(), 1.0);
        source.data.insert("gold_chunks".into(), 3.0);
        source.data.insert("gold_nuggets".into(), 1.0);
        source.data.insert("treasure_dx".into(), -2.0);
        source.data.insert("treasure_dy".into(), 3.0);
        source.text.insert("treasure".into(), "oSapphireBig".into());
        let mut rng = Rng::new(71);
        destroy_temple_support_clean(&mut world, &mut rng, &source);
        assert_eq!(world.entities.len(), 1);
        assert_eq!(world.entities[0].object, "oSapphireBig");
        assert_eq!(world.entities[0].pos, Vec2::new(86.0, 91.0));
        // The actual R producer also sets the embedded flag, despite this
        // label referring to OWN Destroy loot rather than a Create-born item.
        source.text.insert("treasure".into(), "Big Ruby".into());
        destroy_temple_support_clean(&mut world, &mut rng, &source);
        assert_eq!(
            world.entities.len(),
            1,
            "R-template own Destroy treasure is clean-suppressed"
        );
    }

    #[test]
    fn temple_clean_destroy_removes_only_matching_xoc_fragment() {
        let mut world = World::new(16, 16, Area::Temple, 13);
        let mut ids = Vec::new();
        for (pos, depth) in [
            (Vec2::new(80.0, 80.0), 99.0),
            (Vec2::new(80.0, 80.0), 100.0),
            (Vec2::new(96.0, 80.0), 99.0),
        ] {
            let id = world.spawn("oGenerationDecoration", pos);
            let fragment = world.entity_mut(id).unwrap();
            fragment.sprite = "bgLadyXoc".into();
            fragment.data.insert("depth".into(), depth);
            ids.push(id);
        }
        let mut source = temple_clean_source("oXocBlock");
        // tiles::terrain_treasure tags the mural's own Destroy loot this way.
        source.data.insert("embedded_treasure".into(), 1.0);
        source.text.insert("treasure".into(), "Diamond".into());
        destroy_temple_support_clean(&mut world, &mut Rng::new(71), &source);
        assert_eq!(
            world.entities.len(),
            3,
            "clean Xoc must not release its tagged own loot"
        );
        assert!(world.entity(ids[0]).is_none());
        assert!(world.entity(ids[1]).is_some());
        assert!(world.entity(ids[2]).is_some());
        assert!(!world.fluid_dirty);
    }

    #[test]
    fn temple_parent_attachment_uses_translated_receiver_after_independent_point_gate() {
        let mut world = World::new(16, 16, Area::Temple, 13);
        // First source instance intersects the translated receiver, but does
        // not contain the independent gate point. A later spike opens the gate.
        let selected = world.spawn("oSpikes", Vec2::new(96.0, 80.0));
        let gate = world.spawn("oSpikes", Vec2::new(80.0, 64.0));
        let source = temple_clean_source("oTemple");
        let point = Vec2::new(88.0, 79.0);
        assert_eq!(
            temple_parent_attachment(&world, &source, "oSpikes", point, point),
            Some(selected)
        );
        destroy_temple_support_clean(&mut world, &mut Rng::new(71), &source);
        assert!(world.entity(selected).is_none());
        assert!(
            world.entity(gate).is_some(),
            "Destroy selects one translated-mask target, not all gate hits"
        );
        world.entity_mut(gate).unwrap().alive = false;
        world.spawn("oSpikes", Vec2::new(96.0, 80.0));
        assert_eq!(
            temple_parent_attachment(&world, &source, "oSpikes", point, point),
            None
        );
    }

    #[test]
    #[should_panic(expected = "unsupported Temple clean Destroy family: oDoor")]
    fn temple_clean_destroy_rejects_unguarded_door_callback() {
        destroy_temple_support_clean(
            &mut World::new(16, 16, Area::Temple, 13),
            &mut Rng::new(71),
            &temple_clean_source("oDoor"),
        );
    }

    #[test]
    #[should_panic(expected = "unsupported Temple clean Destroy attachment: oGrave")]
    fn temple_clean_destroy_rejects_game_dependent_grave_callback() {
        let mut world = World::new(16, 16, Area::Temple, 13);
        world.spawn("oGrave", Vec2::new(80.0, 64.0));
        destroy_temple_support_clean(
            &mut world,
            &mut Rng::new(71),
            &temple_clean_source("oTemple"),
        );
    }

    #[test]
    fn magma_explosion_finishes_each_create_before_its_launch_override() {
        for seed in [1, 2, 7, 11] {
            let mut g = Game::shell(seed, RulesPreset::Classic);
            g.player.pos = Vec2::new(200.0, 200.0);
            let man = g.world.spawn("oMagmaMan", Vec2::new(80.0, 80.0));
            g.world.spawn("oExplosion", Vec2::new(88.0, 88.0));
            let mut source_rng = g.rng.clone();
            let mut launches = Vec::new();
            for _ in 0..2 {
                let vx = source_rng.unit() * 4.0 - source_rng.unit() * 4.0;
                let _create_vy = -1.0 - source_rng.unit() * 2.0;
                let gravity = f64::from(1 + source_rng.range(6)) * 0.1;
                let vy = -f64::from(1 + source_rng.range(3));
                launches.push((Vec2::new(vx, vy), gravity));
            }
            tick(&mut g);
            assert!(g.world.entity(man).is_none());
            let magma: Vec<_> = g
                .world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == "oMagma")
                .map(|e| {
                    (
                        e.id,
                        e.vel,
                        e.value("gravity"),
                        e.value("world_effect_initialized"),
                    )
                })
                .collect();
            assert_eq!(magma.len(), 2);
            for ((_, vel, gravity, initialized), (expected_vel, expected_gravity)) in
                magma.iter().zip(&launches)
            {
                assert_eq!(vel, expected_vel, "seed {seed}");
                assert_eq!(gravity, expected_gravity, "seed {seed}");
                assert_eq!(*initialized, 1.0);
            }
            assert_eq!(
                g.rng.clone().range(100000),
                source_rng.range(100000),
                "seed {seed}"
            );
            g.step(&InputFrame::default());
            for ((id, _, _, _), (velocity, gravity)) in magma.iter().zip(&launches) {
                let e = g.world.entity(*id).unwrap();
                assert_eq!(e.pos.y, 76.0 + velocity.y, "seed {seed}");
                assert!(
                    (e.vel.y - (velocity.y + gravity)).abs() < 1e-9,
                    "seed {seed}"
                );
                assert_eq!(e.vel.x, velocity.x, "seed {seed}");
            }
        }
    }

    #[test]
    fn unsupported_tree_callbacks_bypass_only_the_tree_attack_immunity() {
        let mut g = game();
        g.world.set_tile(5, 5, "oTree");
        let metadata = g.world.spawn("oTerrainMetadata", Vec2::new(88.0, 88.0));
        g.world
            .entity_mut(metadata)
            .unwrap()
            .data
            .insert("invincible".into(), 1.0);
        destroy_terrain(&mut g, 5, 5);
        assert_eq!(g.world.tile(5, 5).unwrap().object, "oTree");
        assert!(destroy_unsupported_tree_tile(&mut g, 5, 5));
        assert!(g.world.tile(5, 5).unwrap().object.is_empty());
        assert!(g.world.entity(metadata).is_none());
        assert!(!destroy_unsupported_tree_tile(&mut g, 5, 5));
        let tree = g.world.spawn("oTree", Vec2::new(112.0, 80.0));
        g.world
            .entity_mut(tree)
            .unwrap()
            .data
            .insert("invincible".into(), 1.0);
        assert!(!destroy_solid_instance(&mut g, tree));
        assert!(destroy_unsupported_tree_instance(&mut g, tree));
        assert!(!destroy_unsupported_tree_instance(&mut g, tree));
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| e.alive && matches!(e.object.as_str(), "oRubble" | "oRubbleSmall"))
                .count(),
            6
        );
        g.world.set_tile(8, 5, "oBrick");
        assert!(!destroy_unsupported_tree_tile(&mut g, 8, 5));
        let brick = g.world.spawn("oBrick", Vec2::new(144.0, 80.0));
        assert!(!destroy_unsupported_tree_instance(&mut g, brick));
        assert!(g.world.entity(brick).is_some());
    }

    #[test]
    fn bomb_throw_source_velocity_and_inventory() {
        let mut g = game();
        g.player.vel.x = 2.0;
        use_bomb(
            &mut g,
            &InputFrame {
                bomb_pressed: true,
                up: true,
                ..Default::default()
            },
        );
        assert_eq!(g.player.bombs, 3);
        assert_eq!(g.world.entities[0].vel, Vec2::new(10.0, -9.0));
        assert_eq!(g.world.entities[0].timer, 120);
        g.player.bombs = 0;
        use_bomb(
            &mut g,
            &InputFrame {
                bomb_pressed: true,
                ..Default::default()
            },
        );
        assert_eq!(g.world.entities.len(), 1);
    }

    #[test]
    fn fuse_takes_120_ticks_and_unarmed_bombs_do_not_explode() {
        let mut g = game();
        use_bomb(
            &mut g,
            &InputFrame {
                bomb_pressed: true,
                down: true,
                ..Default::default()
            },
        );
        for _ in 0..119 {
            tick(&mut g);
        }
        assert!(
            !g.events
                .iter()
                .any(|event| matches!(event, GameEvent::Explosion { .. }))
        );
        tick(&mut g);
        assert_eq!(
            g.events
                .iter()
                .filter(|event| matches!(event, GameEvent::Explosion { .. }))
                .count(),
            1
        );
        let mut g = game();
        g.world.spawn("oBomb", Vec2::new(80.0, 80.0));
        for _ in 0..150 {
            tick(&mut g);
        }
        assert!(g.world.entities[0].alive);
    }

    #[test]
    fn explosion_destroys_terrain_but_preserves_boundary() {
        let mut g = game();
        g.world.set_tile(5, 5, "oBlock");
        g.world.set_tile(6, 5, "oSolid");
        explode(&mut g, Vec2::new(88.0, 88.0));
        for _ in 0..13 {
            tick(&mut g);
        }
        assert_eq!(g.world.tile(5, 5).unwrap().object, "");
        assert_eq!(g.world.tile(6, 5).unwrap().object, "oSolid");
    }

    #[test]
    fn chain_reaction_arms_an_unarmed_bomb() {
        let mut g = game();
        let bomb = g.world.spawn("oBomb", Vec2::new(88.0, 88.0));
        init_entity(g.world.entity_mut(bomb).unwrap());
        explode(&mut g, Vec2::new(88.0, 88.0));
        let bomb = g.world.entity(bomb).unwrap();
        assert_eq!(bomb.value("armed"), 1.0);
        assert!((4..=8).contains(&bomb.timer));
    }

    #[test]
    fn downward_rope_extends_128_pixels_and_stops_at_terrain() {
        let mut g = game();
        use_rope(
            &mut g,
            &InputFrame {
                rope_pressed: true,
                down: true,
                ..Default::default()
            },
        );
        for _ in 0..17 {
            tick(&mut g);
        }
        assert_eq!(g.player.ropes, 3);
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|entity| entity.object == "oRope" && entity.alive)
                .count(),
            16
        );
        assert!(
            g.world
                .entities
                .iter()
                .any(|entity| entity.object == "oRopeTop")
        );
        assert!(
            g.world
                .entities
                .iter()
                .any(|entity| entity.object == "oRope"
                    && crate::catalog::is_a(&entity.object, "oLadder"))
        );
    }

    #[test]
    fn blocked_rope_placement_does_not_spend_inventory() {
        let mut g = game();
        g.world.set_tile(10, 10, "oBlock");
        let attempt_id = g.world.next_id;
        let creation_serial = g.world.creation_serial;
        let entity_count = g.world.entities.len();
        let events = g.events.clone();
        let mut expected_rng = g.rng.clone();
        use_rope(
            &mut g,
            &InputFrame {
                rope_pressed: true,
                down: true,
                ..Default::default()
            },
        );
        assert_eq!(g.player.ropes, 4);
        // Source creates before probing, then destroys the failed attempt.
        // Direct helper calls precede the dispatcher's dead-instance cleanup.
        assert_eq!(g.world.entities.len(), entity_count + 1);
        assert_eq!(g.world.next_id, attempt_id + 1);
        assert_eq!(g.world.creation_serial, creation_serial + 1);
        let attempt = &g.world.entities[entity_count];
        assert_eq!(attempt.id, attempt_id);
        assert_eq!(attempt.creation_serial, creation_serial + 1);
        assert_eq!(attempt.object, "oRopeThrow");
        assert_eq!(attempt.sprite, "sRopeEnd");
        assert_eq!(attempt.pos, Vec2::new(176.0, 160.0));
        assert_eq!(attempt.data.get("t"), Some(&0.0));
        assert!(!attempt.alive);
        assert!(!attempt.held);
        assert_eq!(attempt.value("armed"), 0.0);
        assert_eq!(attempt.value("falling"), 0.0);
        assert_eq!(attempt.vel, Vec2::default());
        assert!(g.world.entity(attempt_id).is_none());
        assert!(!g.world.entities.iter().any(|entity| entity.alive));
        assert_eq!(g.player.held, None);
        assert_eq!(g.events, events);
        assert!(
            !g.events
                .iter()
                .any(|event| matches!(event, GameEvent::Sound { name, .. } if name == "sndThrow"))
        );
        assert_eq!(g.rng.clone().next_u64(), expected_rng.next_u64());
    }

    #[test]
    fn paste_sticks_to_terrain_and_releases_when_support_is_destroyed() {
        let mut g = game();
        g.player.pos = Vec2::new(88.0, 88.0);
        g.player.equipment.insert("Paste".into());
        g.world.set_tile(6, 5, "oBlock");
        use_bomb(
            &mut g,
            &InputFrame {
                bomb_pressed: true,
                ..Default::default()
            },
        );
        tick(&mut g);
        assert_eq!(g.world.entities[0].value("stuck"), 1.0);
        assert_eq!(g.world.entities[0].vel, Vec2::default());
        g.world.set_tile(6, 5, "");
        tick(&mut g);
        assert_eq!(g.world.entities[0].value("stuck"), 0.0);
        assert!(g.world.entities[0].vel.y > 0.0);
    }

    #[test]
    fn precise_explosion_mask_changes_with_animation_frame() {
        let pos = Vec2::new(24.0, 24.0);
        assert!(!mask_intersects(pos, 0, Rect::new(0.0, 0.0, 8.0, 48.0)));
        assert!(mask_intersects(pos, 3, Rect::new(0.0, 0.0, 8.0, 48.0)));
        assert!(!mask_intersects(pos, 3, Rect::new(48.0, 0.0, 8.0, 48.0)));
    }

    #[test]
    fn arming_carried_bomb_preserves_running_fuse() {
        let mut bomb = Entity::new(1, "oBomb", Vec2::default());
        arm_bomb(&mut bomb);
        assert_eq!(bomb.timer, BOMB_FUSE);
        bomb.timer = 27;
        arm_bomb(&mut bomb);
        assert_eq!(bomb.timer, 27);
    }

    #[test]
    fn inventory_cycle_refunds_reservations_and_recreates_original_type() {
        let mut g = game();
        let original = g.world.spawn("oRock", g.player.pos);
        g.world.entity_mut(original).unwrap().held = true;
        g.player.held = Some(original);
        cycle_inventory(&mut g);
        assert_eq!(g.player.bombs, 3);
        assert_eq!(
            g.world.entity(g.player.held.unwrap()).unwrap().object,
            "oBomb"
        );
        assert!(g.world.entity(original).is_none());
        assert_eq!(g.player.pickup_item_type, "Rock");
        cycle_inventory(&mut g);
        assert_eq!((g.player.bombs, g.player.ropes), (4, 3));
        assert_eq!(
            g.world.entity(g.player.held.unwrap()).unwrap().object,
            "oRopeThrow"
        );
        cycle_inventory(&mut g);
        assert_eq!((g.player.bombs, g.player.ropes), (4, 4));
        let fresh = g.player.held.unwrap();
        assert_ne!(fresh, original);
        assert_eq!(g.world.entity(fresh).unwrap().object, "oRock");
        assert_eq!(g.world.entity(fresh).unwrap().pos, g.player.pos);
    }

    #[test]
    fn using_reserved_rope_does_not_spend_a_second_rope() {
        let mut g = game();
        g.player.bombs = 0;
        cycle_inventory(&mut g);
        let held = g.player.held.unwrap();
        assert_eq!(g.player.ropes, 3);
        assert!(use_held_rope(&mut g, held, &InputFrame::default()));
        assert_eq!(g.player.ropes, 3);
        assert!(g.player.held.is_none());
        assert!(
            g.world
                .entities
                .iter()
                .any(|rope| rope.alive && rope.object == "oRopeThrow" && !rope.held)
        );
    }

    #[test]
    fn terrain_metadata_preserves_invincibility_and_releases_treasure_once() {
        let mut g = game();
        g.world.set_tile(5, 5, "oBlock");
        let id = g.world.spawn("oTerrainMetadata", Vec2::new(88.0, 88.0));
        let metadata = g.world.entity_mut(id).unwrap();
        metadata.data.insert("invincible".into(), 1.0);
        metadata.data.insert("gold_chunks".into(), 3.0);
        metadata.data.insert("embedded_treasure".into(), 1.0);
        metadata.data.insert("shopWall".into(), 1.0);
        metadata.text.insert("treasure".into(), "Big Ruby".into());
        destroy_terrain(&mut g, 5, 5);
        assert_eq!(g.world.tile(5, 5).unwrap().object, "oBlock");
        g.world
            .entity_mut(id)
            .unwrap()
            .data
            .insert("invincible".into(), 0.0);
        destroy_terrain(&mut g, 5, 5);
        destroy_terrain(&mut g, 5, 5);
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| e.object == "oGoldChunk")
                .count(),
            3
        );
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| e.object == "oRubyBig")
                .count(),
            1
        );
        assert!(g.flags.contains("shopkeepers_angry"));
    }

    #[test]
    fn bomb_bounces_off_dynamic_push_block_without_tunneling() {
        let mut g = game();
        let block = g.world.spawn("oPushBlock", Vec2::new(96.0, 80.0));
        g.world.entity_mut(block).unwrap().solid = true;
        let mut bomb = Entity::new(999, "oBomb", Vec2::new(88.0, 88.0));
        bomb.vel = Vec2::new(12.0, 0.0);
        projectile_motion(&g.world, &mut bomb, false);
        assert!(bomb.pos.x <= 92.0);
        assert!(bomb.vel.x < 0.0);
    }

    #[test]
    fn embedded_treasure_offsets_are_relative_to_tile_center() {
        let mut g = game();
        g.world.set_tile(5, 5, "oBlock");
        let id = g.world.spawn("oTerrainMetadata", Vec2::new(88.0, 88.0));
        let metadata = g.world.entity_mut(id).unwrap();
        metadata.data.insert("embedded_treasure".into(), 1.0);
        metadata.data.insert("treasure_dy".into(), -2.0);
        metadata.text.insert("treasure".into(), "oJetpack".into());
        destroy_terrain(&mut g, 5, 5);
        assert!(
            g.world
                .entities
                .iter()
                .any(|entity| entity.object == "oJetpack" && entity.pos == Vec2::new(88.0, 86.0))
        );
    }

    #[test]
    fn game_step_quantizes_bomb_motion_by_level_engine_time_and_hits_dynamic_walls() {
        let mut g = game();
        g.tick = 100; // Entering a new room resets engine phase, not total run ticks.
        let id = g.world.spawn("oBomb", Vec2::new(80.0, 80.0));
        g.world.entity_mut(id).unwrap().vel.x = 1.5;
        g.step(&InputFrame::default());
        assert_eq!(g.world.entity(id).unwrap().pos.x, 82.0);
        g.step(&InputFrame::default());
        assert_eq!(g.world.entity(id).unwrap().pos.x, 83.0);
        let block = g.world.spawn("oPushBlock", Vec2::new(96.0, 80.0));
        g.world.entity_mut(block).unwrap().solid = true;
        g.world.entity_mut(id).unwrap().vel = Vec2::new(12.0, 0.0);
        g.step(&InputFrame::default());
        let bomb = g.world.entity(id).unwrap();
        // Source post-movement side correction backs one pixel away.
        assert_eq!(bomb.pos.x, 91.0);
        assert!(bomb.vel.x < 0.0);
    }

    #[test]
    fn game_step_bomb_can_fall_away_from_an_overlapping_ceiling() {
        let mut g = game();
        g.world.set_tile(5, 5, "oBlock");
        let id = g.world.spawn("oBomb", Vec2::new(88.0, 99.0));
        g.world.entity_mut(id).unwrap().vel.y = 2.0;
        g.step(&InputFrame::default());
        assert_eq!(g.world.entity(id).unwrap().pos.y, 101.0);
    }

    #[test]
    fn game_step_explosion_bypasses_contact_invincibility_and_knocks_player_back() {
        let mut g = game();
        g.player.health = 30;
        g.player.invincible = 40;
        g.world.spawn("oExplosion", g.player.pos);
        g.step(&InputFrame::default());
        assert_eq!(g.player.health, 20);
        assert_eq!(g.player.stun, 100);
        assert_eq!(g.player.vel.y, -6.0);
        assert!((4.0..=6.0).contains(&g.player.vel.x.abs()));
        assert!(g.player.invincible < 40);
    }

    #[test]
    fn game_step_downward_rope_places_over_ledge_without_creating_broad_ladder_tiles() {
        let mut g = game();
        g.player.pos = Vec2::new(152.0, 152.0);
        g.player.previous_pos = g.player.pos;
        g.player.state = PlayerState::Standing;
        g.player.grounded = true;
        g.world.set_tile(9, 10, "oBlock");
        g.step(&InputFrame {
            down: true,
            rope_pressed: true,
            ..Default::default()
        });
        let top = g
            .world
            .entities
            .iter()
            .find(|entity| entity.object == "oRopeTop")
            .unwrap();
        assert_eq!(top.pos, Vec2::new(168.0, 152.0));
        for _ in 0..17 {
            g.step(&InputFrame::default());
        }
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|entity| entity.object == "oRope")
                .count(),
            16
        );
        assert!(!g.world.tiles.iter().any(Tile::is_ladder));
    }

    #[test]
    fn game_step_blocked_carried_rope_is_thrown_without_spending_another_rope() {
        let mut g = game();
        g.player.pos = Vec2::new(152.0, 152.0);
        g.player.previous_pos = g.player.pos;
        g.player.state = PlayerState::Standing;
        g.player.grounded = true;
        for x in 0..g.world.width {
            g.world.set_tile(x, 10, "oBlock");
        }
        g.player.bombs = 0;
        cycle_inventory(&mut g);
        let reserved = g.player.held.unwrap();
        g.step(&InputFrame {
            down: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert!(g.player.held.is_none());
        assert_eq!(g.player.ropes, 3);
        assert!(g.world.entity(reserved).is_none());
        let rope = g
            .world
            .entities
            .iter()
            .find(|rope| rope.alive && rope.object == "oRopeThrow" && !rope.held)
            .unwrap();
        assert_ne!(rope.id, reserved);
        assert_eq!(rope.value("falling"), 0.0);
        assert_eq!(rope.vel.x, 3.2);
    }

    #[test]
    fn game_step_explosion_destroys_dynamic_solids_and_preserves_invincible_ones() {
        for object in ["oPushBlock", "oThinIce", "oDarkFall"] {
            for invincible in [false, true] {
                let mut g = game();
                let target = g.world.spawn(object, Vec2::new(80.0, 80.0));
                g.world
                    .entity_mut(target)
                    .unwrap()
                    .data
                    .insert("invincible".into(), f64::from(invincible));
                // Keep moving solids supported so this checks destruction,
                // rather than incidental movement out of the blast mask.
                g.world.set_tile(5, 6, "oHardBlock");
                g.world.spawn("oExplosion", Vec2::new(88.0, 84.0));
                g.step(&InputFrame::default());
                assert_eq!(g.world.entity(target).is_some(), invincible, "{object}");
                assert_eq!(g.world.tile(5, 6).unwrap().object, "oHardBlock");
            }
        }
    }

    #[test]
    fn game_step_overlapping_explosion_refreshes_a_bombs_chain_fuse() {
        let mut g = game();
        let bomb = g.world.spawn("oBomb", Vec2::new(88.0, 88.0));
        g.world.spawn("oExplosion", Vec2::new(88.0, 88.0));
        g.step(&InputFrame::default());
        assert!((4..=8).contains(&g.world.entity(bomb).unwrap().alarms[1]));
        // A short alarm can be reached while a bomb remains in a persistent
        // blast. The next collision callback must refresh it, not ignore the
        // bomb because the same explosion touched it on an earlier frame.
        g.world.entity_mut(bomb).unwrap().alarms[1] = 2;
        g.step(&InputFrame::default());
        assert!((4..=8).contains(&g.world.entity(bomb).unwrap().alarms[1]));
    }

    #[test]
    fn game_step_authored_lava_destroys_bombs_and_burns_rope_segments() {
        let mut g = game();
        g.world.spawn("oLava", Vec2::new(80.0, 80.0));
        let bomb = g.world.spawn("oBomb", Vec2::new(88.0, 88.0));
        g.step(&InputFrame::default());
        assert!(g.world.entity(bomb).is_none());
        assert!(
            g.events
                .iter()
                .any(|event| matches!(event, GameEvent::Explosion { .. }))
        );

        let mut g = game();
        g.world.spawn("oLava", Vec2::new(80.0, 80.0));
        g.world.spawn("oRope", Vec2::new(72.0, 84.0));
        g.step(&InputFrame::default());
        assert!(
            g.world
                .entities
                .iter()
                .any(|entity| entity.object == "oRopeBurn")
        );
        for _ in 0..3 {
            g.step(&InputFrame::default());
        }
        assert!(
            !g.world
                .entities
                .iter()
                .any(|entity| entity.object == "oRope")
        );
    }

    #[test]
    fn game_step_bomb_clears_corner_above_its_horizontal_movement_strip() {
        let mut g = game();
        g.world.set_tile(6, 5, "oBlock");
        // Body top y=93 overlaps the bottom three pixels of the wall at
        // y=80..95. moveTo probes y=98..100, allowing passage underneath.
        let bomb = g.world.spawn("oBomb", Vec2::new(91.0, 97.0));
        g.world.entity_mut(bomb).unwrap().vel = Vec2::new(2.0, 0.0);
        g.step(&InputFrame::default());
        // moveTo reaches x=93, then source isCollisionRight backs off one
        // pixel because its full-height strip still touches the corner.
        assert_eq!(g.world.entity(bomb).unwrap().pos.x, 92.0);
    }

    #[test]
    fn game_step_offscreen_bomb_keeps_its_body_still_but_its_fuse_runs() {
        let mut g = game();
        // Outside oItem's 16px view margin, inside oLevel's 96px active margin.
        let bomb = g.world.spawn("oBomb", Vec2::new(400.0, 80.0));
        arm_bomb(g.world.entity_mut(bomb).unwrap());
        let entity = g.world.entity_mut(bomb).unwrap();
        entity.alarms[0] = -1;
        entity.alarms[1] = 2;
        entity.vel = Vec2::new(3.0, 2.0);
        g.step(&InputFrame::default());
        let entity = g.world.entity(bomb).unwrap();
        assert_eq!(entity.pos, Vec2::new(400.0, 80.0));
        assert_eq!(entity.vel, Vec2::new(3.0, 2.0));
        assert_eq!(entity.alarms[1], 1);
        g.step(&InputFrame::default());
        assert!(g.world.entity(bomb).is_none());
        assert!(g.events.iter().any(|event| matches!(event,
            GameEvent::Explosion { pos, .. } if *pos == Vec2::new(400.0, 80.0))));
    }

    #[test]
    fn game_step_embedded_bomb_escapes_without_applying_velocity_or_gravity() {
        let mut g = game();
        g.world.set_tile(5, 5, "oBlock");
        let bomb = g.world.spawn("oBomb", Vec2::new(88.0, 95.0));
        g.world.entity_mut(bomb).unwrap().vel = Vec2::new(3.0, -2.0);
        g.step(&InputFrame::default());
        let entity = g.world.entity(bomb).unwrap();
        assert_eq!(entity.pos, Vec2::new(88.0, 96.0));
        assert_eq!(entity.vel, Vec2::new(3.0, -2.0));
    }

    #[test]
    fn game_step_bomb_settles_out_of_floor_and_pushes_away_from_ceiling() {
        let mut g = game();
        g.world.set_tile(5, 6, "oBlock");
        let bomb = g.world.spawn("oBomb", Vec2::new(88.0, 93.0));
        g.step(&InputFrame::default());
        assert_eq!(g.world.entity(bomb).unwrap().pos.y, 92.0);
        assert_eq!(g.world.entity(bomb).unwrap().vel.y, 0.0);

        let mut g = game();
        g.world.set_tile(5, 5, "oBlock");
        let bomb = g.world.spawn("oBomb", Vec2::new(88.0, 100.0));
        g.step(&InputFrame::default());
        assert_eq!(g.world.entity(bomb).unwrap().pos.y, 101.0);
        assert_eq!(g.world.entity(bomb).unwrap().vel.y, 0.6);
    }

    #[test]
    fn game_step_armed_bomb_warns_shopkeeper_before_exploding() {
        let mut g = game();
        let keeper = g.world.spawn("oShopkeeper", Vec2::new(96.0, 88.0));
        let bomb = g.world.spawn("oBomb", Vec2::new(88.0, 88.0));
        arm_bomb(g.world.entity_mut(bomb).unwrap());
        g.step(&InputFrame::default());
        assert_eq!(g.world.entity(keeper).unwrap().value("angered"), 1.0);
        assert!(g.flags.contains("wanted"));
        assert!(
            !g.events
                .iter()
                .any(|event| matches!(event, GameEvent::Explosion { .. }))
        );
    }
}

#[cfg(test)]
mod destruction_end_step_regression {
    use super::*;

    fn game() -> Game {
        let mut game = Game::shell(7, RulesPreset::Classic);
        game.player.pos = Vec2::new(160.0, 160.0);
        game
    }

    #[test]
    fn game_step_blast_releases_city_of_gold_push_block_treasure_once() {
        let mut g = game();
        g.flags.insert("city_of_gold".into());
        let block = g.world.spawn("oPushBlock", Vec2::new(80.0, 80.0));
        g.world.set_tile(5, 6, "oHardBlock");
        g.world.spawn("oExplosion", Vec2::new(88.0, 88.0));
        g.step(&InputFrame::default());
        assert!(g.world.entity(block).is_none());
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|entity| entity.object == "oGoldChunk")
                .count(),
            3
        );
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|entity| entity.object == "oGoldNugget")
                .count(),
            1
        );
        g.step(&InputFrame::default());
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|entity| entity.object == "oGoldChunk")
                .count(),
            3
        );
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|entity| entity.object == "oGoldNugget")
                .count(),
            1
        );
    }

    #[test]
    fn game_step_inherited_solid_destroy_removes_fixtures_and_releases_lamps() {
        for (lamp, item) in [("oLamp", "oLampItem"), ("oLampRed", "oLampRedItem")] {
            let mut g = game();
            let block = g.world.spawn("oThinIce", Vec2::new(80.0, 80.0));
            let torch = g.world.spawn("oTikiTorch", Vec2::new(80.0, 48.0));
            let grave = g.world.spawn("oGrave", Vec2::new(80.0, 64.0));
            let hanging = g.world.spawn(lamp, Vec2::new(80.0, 96.0));
            g.world.spawn("oExplosion", Vec2::new(88.0, 84.0));
            g.step(&InputFrame::default());
            for removed in [block, torch, grave, hanging] {
                assert!(g.world.entity(removed).is_none(), "{lamp}: {removed}");
            }
            let dropped: Vec<_> = g
                .world
                .entities
                .iter()
                .filter(|entity| entity.object == item)
                .collect();
            assert_eq!(dropped.len(), 1);
            assert_eq!(dropped[0].pos, Vec2::new(88.0, 108.0));
        }
    }

    #[test]
    fn non_inheriting_solid_destroy_does_not_drop_parent_lamps() {
        for object in ["oPushBlock", "oBlock", "oIce"] {
            let mut g = game();
            let block = g.world.spawn(object, Vec2::new(80.0, 80.0));
            let lamp = g.world.spawn("oLamp", Vec2::new(80.0, 96.0));
            assert!(destroy_solid_instance(&mut g, block));
            assert!(g.world.entity(lamp).is_some(), "{object}");
            assert!(
                !g.world
                    .entities
                    .iter()
                    .any(|entity| entity.object == "oLampItem")
            );
            assert!(!destroy_solid_instance(&mut g, block));
        }
    }

    #[test]
    fn game_step_sticky_bomb_follows_enemys_current_end_step_position() {
        let mut g = game();
        for x in 0..12 {
            g.world.set_tile(x, 6, "oHardBlock");
        }
        let enemy = g.world.spawn("oSnake", Vec2::new(80.0, 80.0));
        crate::enemies::init_entity(g.world.entity_mut(enemy).unwrap());
        let bomb = g.world.spawn("oBomb", Vec2::new(88.0, 84.0));
        arm_bomb(g.world.entity_mut(bomb).unwrap());
        let attached = g.world.entity_mut(bomb).unwrap();
        attached.data.insert("attached_id".into(), enemy as f64);
        attached.data.insert("sticky_x".into(), -8.0);
        attached.data.insert("sticky_y".into(), -4.0);
        g.step(&InputFrame::default());
        let target = g.world.entity(enemy).unwrap();
        assert!(target.pos.x > 80.0);
        let bomb = g.world.entity(bomb).unwrap();
        assert_eq!(bomb.pos, Vec2::new(target.pos.x + 8.0, target.pos.y + 4.0));
        assert_eq!(bomb.previous_pos, Vec2::new(88.0, 84.0));
    }
}

#[cfg(test)]
mod frozen_caveman_regression {
    use super::*;

    #[test]
    fn frozen_metadata_releases_exactly_one_actor_with_200_tick_stun() {
        let mut game = Game::shell(7, RulesPreset::Classic);
        game.world.set_tile(5, 5, "oIce");
        let id = game.world.spawn("oTerrainMetadata", Vec2::new(88.0, 88.0));
        let metadata = game.world.entity_mut(id).unwrap();
        metadata.data.insert("embedded_treasure".into(), 1.0);
        metadata.data.insert("frozen_caveman".into(), 1.0);
        metadata.text.insert("treasure".into(), "oCaveman".into());
        destroy_terrain(&mut game, 5, 5);
        let actors: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|entity| entity.alive && entity.object == "oCaveman")
            .collect();
        assert_eq!(actors.len(), 1);
        assert_eq!(actors[0].timer, 200);
        assert_eq!(actors[0].state, 98);
    }
}

#[cfg(test)]
mod destruction_callbacks {
    use super::*;

    fn count(game: &Game, object: &str) -> usize {
        game.world
            .entities
            .iter()
            .filter(|e| e.object == object)
            .count()
    }

    #[test]
    fn source_rubble_consumes_twelve_draws_and_preserves_family_sprites() {
        for (object, large, small) in [
            ("oBrick", "sRubble", "sRubbleSmall"),
            ("oBrickSmooth", "sRubbleTan", "sRubbleTanSmall"),
            ("oLush", "sRubbleLush", "sRubbleLushSmall"),
            ("oSacAltarRight", "sRubbleTan", "sRubbleTanSmall"),
        ] {
            let mut game = Game::shell(7, RulesPreset::Classic);
            let mut expected = game.rng.clone();
            for _ in 0..12 {
                expected.next_u64();
            }
            let target = game.world.spawn(object, Vec2::new(80.0, 80.0));
            assert!(destroy_solid_instance(&mut game, target));
            assert_eq!(count(&game, "oRubble"), 1, "{object}");
            assert_eq!(count(&game, "oRubbleSmall"), 2, "{object}");
            assert!(
                game.world
                    .entities
                    .iter()
                    .filter(|e| e.object == "oRubble")
                    .all(|e| e.sprite == large && e.vel == Vec2::default())
            );
            assert!(
                game.world
                    .entities
                    .iter()
                    .filter(|e| e.object == "oRubbleSmall")
                    .all(|e| e.sprite == small && e.vel == Vec2::default())
            );
            assert_eq!(game.rng.next_u64(), expected.next_u64(), "{object}");
        }
    }

    #[test]
    fn game_step_blast_gold_brick_runs_rubble_before_gold_for_tiles_and_instances() {
        for dynamic in [false, true] {
            let mut game = Game::shell(7, RulesPreset::Classic);
            game.player.pos = Vec2::new(200.0, 200.0);
            if dynamic {
                let id = game.world.spawn("oBrick", Vec2::new(80.0, 80.0));
                game.world.entity_mut(id).unwrap().sprite = "sBrickGoldBig".into();
            } else {
                game.world.set_tile(5, 5, "oBrick");
                let id = game.world.spawn("oTerrainMetadata", Vec2::new(88.0, 88.0));
                game.world
                    .entity_mut(id)
                    .unwrap()
                    .text
                    .insert("terrain_sprite".into(), "sBrickGoldBig".into());
            }
            game.world.spawn("oExplosion", Vec2::new(88.0, 88.0));
            game.step(&InputFrame::default());
            assert_eq!(count(&game, "oRubble"), 1, "{dynamic}");
            assert_eq!(count(&game, "oRubbleSmall"), 2, "{dynamic}");
            assert_eq!(count(&game, "oGoldChunk"), 3, "{dynamic}");
            assert_eq!(count(&game, "oGoldNugget"), 1, "{dynamic}");
            let rubble = game
                .world
                .entities
                .iter()
                .find(|e| e.object == "oRubble")
                .unwrap();
            let gold = game
                .world
                .entities
                .iter()
                .find(|e| e.object == "oGoldChunk")
                .unwrap();
            assert!(rubble.id < gold.id);
        }
    }

    #[test]
    fn game_step_blast_xoc_releases_four_gold_two_gems_and_only_its_mural_fragment() {
        for dynamic in [false, true] {
            for (treasure, gem) in [
                ("Diamond", "oDiamond"),
                ("Ruby", "oRubyBig"),
                ("Sapphire", "oSapphireBig"),
                ("Emerald", "oEmeraldBig"),
            ] {
                let mut game = Game::shell(7, RulesPreset::Classic);
                game.flags.insert("city_of_gold".into());
                game.player.pos = Vec2::new(200.0, 200.0);
                let source = if dynamic {
                    game.world.spawn("oXocBlock", Vec2::new(80.0, 80.0))
                } else {
                    game.world.set_tile(5, 5, "oXocBlock");
                    game.world.spawn("oTerrainMetadata", Vec2::new(88.0, 88.0))
                };
                game.world
                    .entity_mut(source)
                    .unwrap()
                    .text
                    .insert("treasure".into(), treasure.into());
                game.world
                    .entity_mut(source)
                    .unwrap()
                    .data
                    .insert("embedded_treasure".into(), 1.0);
                let mut decorations = Vec::new();
                for (pos, depth) in [
                    (Vec2::new(80.0, 80.0), 99.0),
                    (Vec2::new(96.0, 80.0), 99.0),
                    (Vec2::new(80.0, 80.0), 98.0),
                ] {
                    let id = game.world.spawn("oGenerationDecoration", pos);
                    game.world.entity_mut(id).unwrap().sprite = "bgLadyXoc".into();
                    game.world
                        .entity_mut(id)
                        .unwrap()
                        .data
                        .insert("depth".into(), depth);
                    decorations.push(id);
                }
                game.world.spawn("oExplosion", Vec2::new(88.0, 88.0));
                game.step(&InputFrame::default());
                assert_eq!(count(&game, "oGoldChunk"), 3, "{dynamic}/{treasure}");
                assert_eq!(count(&game, "oGoldNugget"), 1);
                assert_eq!(count(&game, gem), 2, "{dynamic}/{treasure}");
                assert!(game.world.entity(decorations[0]).is_none());
                assert!(game.world.entity(decorations[1]).is_some());
                assert!(game.world.entity(decorations[2]).is_some());
            }
        }
    }

    #[test]
    fn source_clean_guards_suppress_outputs_but_xoc_mural_cleanup_is_unconditional() {
        for object in ["oBrick", "oIceBlock", "oPushBlock"] {
            let mut game = Game::shell(7, RulesPreset::Classic);
            game.flags.insert("cleanSolids".into());
            let id = game.world.spawn(object, Vec2::new(80.0, 80.0));
            let mut expected = game.rng.clone();
            assert!(destroy_solid_instance(&mut game, id));
            assert_eq!(game.world.entities.len(), 1, "{object}");
            assert_eq!(game.rng.next_u64(), expected.next_u64());
        }
        // oBlock omits the global cleanSolids condition in its own event.
        let mut game = Game::shell(7, RulesPreset::Classic);
        game.flags.insert("cleanSolids".into());
        let block = game.world.spawn("oBlock", Vec2::new(80.0, 80.0));
        assert!(destroy_solid_instance(&mut game, block));
        assert_eq!(count(&game, "oRubbleSmall"), 2);
        let xoc = game.world.spawn("oXocBlock", Vec2::new(112.0, 80.0));
        game.world
            .entity_mut(xoc)
            .unwrap()
            .data
            .insert("cleanDeath".into(), 1.0);
        let mural = game
            .world
            .spawn("oGenerationDecoration", Vec2::new(112.0, 80.0));
        game.world.entity_mut(mural).unwrap().sprite = "bgLadyXoc".into();
        game.world
            .entity_mut(mural)
            .unwrap()
            .data
            .insert("depth".into(), 99.0);
        assert!(destroy_solid_instance(&mut game, xoc));
        assert!(game.world.entity(mural).is_none());
        assert_eq!(count(&game, "oRubbleSmall"), 2);
    }

    #[test]
    fn game_step_blast_ice_families_emit_three_inherited_drips() {
        for object in ["oIce", "oIceBlock"] {
            for dynamic in [false, true] {
                let mut game = Game::shell(7, RulesPreset::Classic);
                game.player.pos = Vec2::new(200.0, 200.0);
                if dynamic {
                    game.world.spawn(object, Vec2::new(80.0, 80.0));
                } else {
                    game.world.set_tile(5, 5, object);
                }
                game.world.spawn("oExplosion", Vec2::new(88.0, 88.0));
                game.step(&InputFrame::default());
                assert_eq!(count(&game, "oDrip"), 3, "{object}/{dynamic}");
                assert!(
                    game.world
                        .entities
                        .iter()
                        .filter(|e| e.object == "oDrip")
                        .all(|e| e.vel.y == 0.0)
                );
                // These births occur during automatic collisions, after their
                // Normal Step slot. Gravity starts on the following frame.
                game.step(&InputFrame::default());
                assert!(
                    game.world
                        .entities
                        .iter()
                        .filter(|e| e.object == "oDrip")
                        .all(|e| e.vel.y == 0.6)
                );
            }
        }
    }

    #[test]
    fn inherited_support_destroy_invokes_graves_own_rubble_callback() {
        let mut game = Game::shell(7, RulesPreset::Classic);
        let support = game.world.spawn("oThinIce", Vec2::new(80.0, 80.0));
        let grave = game.world.spawn("oGrave", Vec2::new(80.0, 64.0));
        assert!(destroy_solid_instance(&mut game, support));
        assert!(game.world.entity(grave).is_none());
        assert_eq!(count(&game, "oRubble"), 1);
        assert_eq!(count(&game, "oRubbleSmall"), 2);
        assert!(
            game.world
                .entities
                .iter()
                .filter(|e| e.object == "oRubble")
                .all(|e| e.sprite == "sRubbleTan")
        );
    }

    #[test]
    fn game_step_explosion_executes_jar_and_skull_destroy_callbacks() {
        for object in ["oJar", "oSkull"] {
            let mut game = Game::shell(7, RulesPreset::Classic);
            game.player.pos = Vec2::new(200.0, 200.0);
            let item = game.world.spawn(object, Vec2::new(88.0, 88.0));
            game.world.spawn("oExplosion", Vec2::new(88.0, 88.0));
            game.step(&InputFrame::default());
            assert!(game.world.entity(item).is_none());
            assert_eq!(count(&game, "oSmokePuff"), 1);
            let fragments = count(
                &game,
                if object == "oJar" {
                    "oRubbleSmall"
                } else {
                    "oBone"
                },
            );
            if object == "oJar" {
                assert_eq!(fragments, 3);
            } else {
                assert!((1..=2).contains(&fragments));
            }
        }
    }
}

#[cfg(test)]
mod literal_dark_and_arrow_destroy_regressions {
    use super::*;

    #[test]
    fn real_door_exit_suppresses_explosion_hurt_and_rng_for_every_character() {
        use crate::characters::Character;
        for character in [Character::Explorer, Character::Damsel, Character::TunnelMan] {
            for exit in [false, true] {
                let mut game = Game::shell(7, RulesPreset::Classic);
                crate::characters::apply(&mut game, character);
                game.player.health = 30;
                game.player.pos = Vec2::new(88.0, 88.0);
                game.player.grounded = true;
                game.player.state = PlayerState::Standing;
                game.world.set_tile(5, 6, "oHardBlock");
                game.world.spawn("oExit", Vec2::new(80.0, 80.0));
                game.step(&InputFrame {
                    up: exit,
                    ..Default::default()
                });
                assert_eq!(game.player.is_exiting(), exit, "{character:?}");
                assert_eq!(game.state == RunState::Ending, exit);
                // Exercise the hurt handler after real door entry. The host's
                // ending loop is separate from Game::step; checking only that
                // host dispatch would leave this source sprite guard untested.
                game.player.invincible = 999;
                let before_velocity = game.player.vel;
                assert!(!game.world.entities.iter().any(|e| e.object == "oBlood"));
                let mut expected_rng = game.rng.clone();
                let expected_blood = if exit {
                    None
                } else {
                    let _speed_roll = (expected_rng.next_u64() * 3) >> 32;
                    // Player Step1683-1696 emits script Blood1 after speed.
                    // Literal own oBlood Create: three velocity random calls,
                    // then rand(1,6) gravity. Literal original raw32 scaling and
                    // multiply-high selectors do not call production range/unit.
                    Some((
                        Vec2::new(
                            (expected_rng.next_u64() as f64 / 4_294_967_296.0) * 4.0
                                - (expected_rng.next_u64() as f64 / 4_294_967_296.0) * 4.0,
                            -1.0 - (expected_rng.next_u64() as f64 / 4_294_967_296.0) * 2.0,
                        ),
                        (1.0 + ((expected_rng.next_u64() * 6) >> 32) as f64) * 0.1,
                    ))
                };
                game.events.clear();
                game.world.spawn("oExplosion", game.player.pos);
                tick(&mut game);
                assert_eq!(
                    game.player.health,
                    if exit { 30 } else { 20 },
                    "{character:?}/{exit}"
                );
                assert_eq!(
                    game.events
                        .iter()
                        .filter(|event| matches!(event, GameEvent::Damage { amount: 10 }))
                        .count(),
                    usize::from(!exit)
                );
                assert_eq!(
                    game.lifetime_stats.misc_deaths
                        [crate::statistics::MiscDeath::Explosion as usize],
                    u64::from(!exit)
                );
                if exit {
                    assert_eq!(game.player.vel, before_velocity);
                    assert_eq!(game.player.stun, 0);
                    assert_eq!(game.player.data.get("burning").copied().unwrap_or(0.0), 0.0);
                } else {
                    assert_eq!(game.player.vel.y, -6.0);
                    assert_eq!(game.player.stun, 100);
                }
                let blood: Vec<_> = game
                    .world
                    .entities
                    .iter()
                    .filter(|e| e.alive && e.object == "oBlood")
                    .collect();
                assert_eq!(blood.len(), usize::from(!exit), "{character:?}/{exit}");
                if let Some((velocity, gravity)) = expected_blood {
                    assert_eq!(blood[0].pos, game.player.pos);
                    assert_eq!(blood[0].vel, velocity);
                    assert_eq!(blood[0].value("gravity"), gravity);
                }
                assert_eq!(
                    game.rng.range(100_000),
                    expected_rng.range(100_000),
                    "{character:?}/{exit}"
                );
            }
        }
    }

    #[test]
    fn game_step_dark_debris_preserves_literal_negative_rand_offsets_and_rng_order() {
        for object in ["oDark", "oAlienShip", "oAlienShipFloor"] {
            let mut game = Game::shell(7, RulesPreset::Classic);
            game.player.pos = Vec2::new(160.0, 160.0);
            let target = game.world.spawn(object, Vec2::new(80.0, 80.0));
            game.world.spawn("oExplosion", Vec2::new(88.0, 88.0));
            let mut expected_rng = game.rng.clone();
            let expected: Vec<_> = (0..3)
                .map(|_| {
                    Vec2::new(
                        88.0 + f64::from(expected_rng.range(9))
                            - (f64::from(expected_rng.range(9)) - 8.0),
                        88.0 + f64::from(expected_rng.range(9))
                            - (f64::from(expected_rng.range(9)) - 8.0),
                    )
                })
                .collect();
            game.step(&InputFrame::default());
            assert!(game.world.entity(target).is_none());
            let rubble: Vec<_> = game
                .world
                .entities
                .iter()
                .filter(|entity| {
                    matches!(entity.object.as_str(), "oRubbleDark" | "oRubbleDarkSmall")
                })
                .collect();
            assert_eq!(rubble.len(), 3, "{object}");
            assert_eq!(rubble[0].object, "oRubbleDark");
            for (rubble, expected) in rubble.iter().zip(expected) {
                assert_eq!(rubble.pos, expected, "{object}");
                assert!((88.0..=104.0).contains(&rubble.pos.x));
                assert!((88.0..=104.0).contains(&rubble.pos.y));
            }
            assert_eq!(
                game.rng.range(100_000),
                expected_rng.range(100_000),
                "{object}"
            );
        }
    }

    #[test]
    fn game_step_dark_and_alien_destroy_keep_their_distinct_clean_guards() {
        for object in ["oDark", "oAlienShip", "oAlienShipFloor"] {
            for clean_global in [false, true] {
                let mut game = Game::shell(7, RulesPreset::Classic);
                game.player.pos = Vec2::new(160.0, 160.0);
                let target = game.world.spawn(object, Vec2::new(80.0, 80.0));
                if clean_global {
                    game.flags.insert("cleanSolids".into());
                } else {
                    game.world
                        .entity_mut(target)
                        .unwrap()
                        .data
                        .insert("cleanDeath".into(), 1.0);
                }
                game.world.spawn("oExplosion", Vec2::new(88.0, 88.0));
                game.step(&InputFrame::default());
                let count = game
                    .world
                    .entities
                    .iter()
                    .filter(|entity| {
                        matches!(entity.object.as_str(), "oRubbleDark" | "oRubbleDarkSmall")
                    })
                    .count();
                assert_eq!(
                    count,
                    if object == "oDark" { 0 } else { 3 },
                    "{object}/{clean_global}"
                );
            }
        }
    }

    #[test]
    fn game_step_mattock_recovers_only_unfired_arrow_traps_for_tiles_and_instances() {
        for dynamic in [false, true] {
            for fired in [false, true] {
                let mut game = Game::shell(7, RulesPreset::Classic);
                game.player.pos = Vec2::new(72.0, 88.0);
                game.player.state = PlayerState::Standing;
                game.player.grounded = true;
                game.player.facing = 1;
                game.world.set_tile(4, 6, "oHardBlock");
                let fired_key = if dynamic {
                    let target = game.world.spawn("oArrowTrapRight", Vec2::new(80.0, 80.0));
                    format!("hazard_entity:{target}:fired")
                } else {
                    game.world.set_tile(5, 5, "oArrowTrapRight");
                    format!("hazard:{}:fired", 5 * game.world.width + 5)
                };
                let mattock = game.world.spawn("oMattock", game.player.pos);
                game.world.entity_mut(mattock).unwrap().held = true;
                game.player.held = Some(mattock);
                // Let source Create establish the trap state before overriding
                // the fixture's initial fired/unfired condition.
                game.step(&InputFrame::default());
                game.data.insert(fired_key, f64::from(fired));
                game.step(&InputFrame {
                    attack: true,
                    attack_pressed: true,
                    ..Default::default()
                });
                for _ in 0..10 {
                    let target_exists = if dynamic {
                        game.world
                            .entities
                            .iter()
                            .any(|e| e.alive && e.object == "oArrowTrapRight")
                    } else {
                        game.world.tile(5, 5).is_some_and(Tile::is_solid)
                    };
                    if !target_exists {
                        break;
                    }
                    game.step(&InputFrame::default());
                }
                let arrows: Vec<_> = game
                    .world
                    .entities
                    .iter()
                    .filter(|entity| entity.object == "oArrow")
                    .collect();
                assert_eq!(arrows.len(), usize::from(!fired), "{dynamic}/{fired}");
                if !fired {
                    assert_eq!(arrows[0].pos, Vec2::new(88.0, 88.0));
                }
                assert_eq!(
                    game.world
                        .entities
                        .iter()
                        .filter(|entity| entity.object == "oRubble")
                        .count(),
                    1
                );
                assert_eq!(
                    game.world
                        .entities
                        .iter()
                        .filter(|entity| entity.object == "oRubbleSmall")
                        .count(),
                    2
                );
            }
        }
    }
}

const EXPLOSION_MASKS: [[u64; 48]; 10] = [
    [
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000007f00000,
        0x00001ffc0000,
        0x00007fff0000,
        0x0000ffff8000,
        0x0001ffffc000,
        0x0003ffffe000,
        0x0007fffff000,
        0x0007fffff000,
        0x0007fffff000,
        0x000ffffff800,
        0x000ffffff800,
        0x000ffffff800,
        0x000ffffff800,
        0x000ffffff800,
        0x000ffffff800,
        0x0007fffff000,
        0x0007fffff000,
        0x0007fffff000,
        0x0003ffffe000,
        0x0001ffffc000,
        0x0000ffff8000,
        0x00007fff0000,
        0x00001ffc0000,
        0x000007f00000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
    ],
    [
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000007e00000,
        0x00007ffe0000,
        0x0001ffff8000,
        0x0003ffffc000,
        0x000ffffff000,
        0x001ffffff800,
        0x003ffffffc00,
        0x007ffffffe00,
        0x007ffffffe00,
        0x00ffffffff00,
        0x01ffffffff80,
        0x01ffffffff80,
        0x03ffffffffc0,
        0x03ffffffffc0,
        0x03ffffffffc0,
        0x03ffffffffc0,
        0x07ffffffffe0,
        0x07ffffffffe0,
        0x07ffffffffe0,
        0x07ffffffffe0,
        0x07ffffffffe0,
        0x07ffffffffe0,
        0x03ffffffffc0,
        0x03ffffffffc0,
        0x03ffffffffc0,
        0x03ffffffffc0,
        0x01ffffffff80,
        0x01ffffffff80,
        0x00ffffffff00,
        0x007ffffffe00,
        0x007ffffffe00,
        0x003ffffffc00,
        0x001ffffff800,
        0x000ffffff000,
        0x0003ffffc000,
        0x0001ffff8000,
        0x00007ffe0000,
        0x000007e00000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
    ],
    [
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000001000000,
        0x000001000000,
        0x000001100000,
        0x000001100000,
        0x000043100000,
        0x000043900000,
        0x000143b00000,
        0x000143b80000,
        0x001043f82400,
        0x001ce7f86400,
        0x000fe7f8f000,
        0x000fe7fdf040,
        0x010ffffff0c0,
        0x018ffffff180,
        0x01cffffff380,
        0x0cfffffff700,
        0x0fffffffff00,
        0x07ffffffff20,
        0x07ffffffff60,
        0x03ffffffff40,
        0x03fffffffe00,
        0x01fffffffe00,
        0x01fffffffe00,
        0x01fffffffe00,
        0x00fffffffe00,
        0x00fffffffe00,
        0x03ffffffff00,
        0x07ffffffff80,
        0x07ffffffff80,
        0x07ffffffff80,
        0x07ffffffff80,
        0x03ffffffff00,
        0x00fffffffc00,
        0x003ffffff000,
        0x003ffffff000,
        0x001ffffff000,
        0x001fffffe000,
        0x000fffffe000,
        0x0007ffffc000,
        0x0003ffff8000,
        0x0000fffe0000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
    ],
    [
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x00003fe07000,
        0x0000fff8f800,
        0x0063fffc1c00,
        0x0067ffffc400,
        0x000ffffff000,
        0x003ffffffc00,
        0x00fffffffe00,
        0x01ffffffff00,
        0x01ffffffff00,
        0x03ffffffff80,
        0x07ffffffff80,
        0x0fffffffffc0,
        0x0fffffffffe0,
        0x1ffffffffff8,
        0x3ffffffffffc,
        0x7ffffffffffe,
        0xffffffffffff,
        0xffffffffffff,
        0xffffffffffff,
        0xffffffffffff,
        0xffffffffffff,
        0xffffffffffff,
        0xffffffffffff,
        0xffffffffffff,
        0x7fffffffffff,
        0x7ffffffffffe,
        0x3ffffffffffc,
        0x1ffffffffff8,
        0x1ffffffffff8,
        0x0ffffffffff0,
        0x07fffffffff0,
        0x03ffffffffe0,
        0x03ffffffffe0,
        0x03ffffffffc0,
        0x03ffffffff80,
        0x01ffffffff00,
        0x00fffffffce0,
        0x007ffffff1e0,
        0x033fffffc1e0,
        0x039fffff80c0,
        0x010efffe0000,
        0x00003ffc0000,
        0x00000fe00000,
        0x000000000000,
        0x000000000000,
    ],
    [
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000033e00000,
        0x0000fff80000,
        0x0001fffe0000,
        0x0003ffff0000,
        0x0007ffff8000,
        0x000fffffc000,
        0x000fffffd800,
        0x003ffffffc00,
        0x003ffffffc00,
        0x007ffffffc00,
        0x007ffffffc00,
        0x00fffffffc00,
        0x00ffffffff00,
        0x00ffffffff80,
        0x007fffffff80,
        0x00ffffffff80,
        0x01ffffffff00,
        0x01fffffff800,
        0x01fffffffc00,
        0x01fffffffc00,
        0x007ffffff800,
        0x001ffffff000,
        0x001fffffc000,
        0x000fffffc000,
        0x0003ffff8000,
        0x0000fffe0000,
        0x00007ffc0000,
        0x00003ff80000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
    ],
    [
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x00003ff80000,
        0x00007ffe0000,
        0x0001ffff8000,
        0x0003ffffc000,
        0x0007ffffe000,
        0x000fffffe000,
        0x001ffffff800,
        0x003ffffffc00,
        0x00fffffffe00,
        0x01ffffffff00,
        0x03ffffffff80,
        0x03ffffffffc0,
        0x03ffffffffc0,
        0x07ffffffffc0,
        0x07ffffffffe0,
        0x07ffffffffe0,
        0x07ffffffffe0,
        0x07ffffffffe0,
        0x07ffffffffe0,
        0x03ffffffffe0,
        0x03ffffffffe0,
        0x01ffffffffc0,
        0x03ffffffffc0,
        0x03ffffffffc0,
        0x03ffffffffc0,
        0x01ffffffffc0,
        0x00ffffffff80,
        0x007ffffffc00,
        0x027ffffffe00,
        0x003ffffffe00,
        0x001ffffffe00,
        0x00cffffffe00,
        0x00c7fffffc00,
        0x0003ffff8000,
        0x0001ffff6000,
        0x0000fffef000,
        0x00006ff8f000,
        0x000007f06000,
        0x000003e00000,
        0x000001c00000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
    ],
    [
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x00003ff00000,
        0x0000fffe0000,
        0x0003ffff8000,
        0x0007ffffc000,
        0x000fffffe000,
        0x001fffffe000,
        0x001ffffff0c0,
        0x007ffffff0c0,
        0x01fffffff800,
        0x03fffffffe00,
        0x03ffffffff00,
        0x07ffffffff80,
        0x07ffffffff98,
        0x07fffffffffc,
        0x07fffffffffc,
        0x0ffffffffffc,
        0x0ffffffffffc,
        0x0ffffffffffc,
        0x0ffffffffffc,
        0x0ffffffffff8,
        0x0ffffffffff0,
        0x0ffffffffff0,
        0x0ffffffffff0,
        0x07ffffffffe0,
        0x07ffffffffe0,
        0x03ffffffffc0,
        0x03ffffffffc0,
        0x01ffffffff80,
        0x00ffffffff00,
        0x00fffffffc00,
        0x021ffffffe00,
        0x000ffffffe00,
        0x000ffffffe00,
        0x00c6fffffe00,
        0x00c67fff9c00,
        0x00031ffe0000,
        0x00010ff86000,
        0x00000fe0e000,
        0x000007c0c000,
        0x000002000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
    ],
    [
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x0183e7f80000,
        0x01cffffe0000,
        0x01dffffff0c0,
        0x019ffffff8c0,
        0x001ffffff800,
        0x03fffffffb00,
        0x07ffffffffc0,
        0x0fffffffffe0,
        0x1ffffffffff8,
        0x1ffffffffffc,
        0x1ffffffffffc,
        0x1ffffffffffc,
        0x1ffffffffffc,
        0x0ffffffffffc,
        0x1ffffffffff8,
        0x1ffffffffff0,
        0x1ffffffffff0,
        0x1ffffffffff0,
        0x0ffffffffff0,
        0x07ffffffffe0,
        0x07ffffffffe0,
        0x03ffffffffc0,
        0x03efffffffc0,
        0x01efffffff80,
        0x00e7ffffff00,
        0x00f7fffffc00,
        0x0003fffffe00,
        0x020dfffffe00,
        0x000c7ffffe00,
        0x00040ffffc00,
        0x000007ff8000,
        0x000003f84000,
        0x000000c00000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
    ],
    [
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000080000,
        0x0000441c0000,
        0x0000060e0000,
        0x0003e00e0000,
        0x000fcff40000,
        0x001ffff98000,
        0x001fffffc000,
        0x003fffffe400,
        0x003fffffe000,
        0x007ffffff000,
        0x007ffffff000,
        0x00fffffff800,
        0x00fffffffc00,
        0x00fffffffe00,
        0x00ffffffff00,
        0x00ffffffff00,
        0x00ffffffff00,
        0x00fffffffe00,
        0x007ffffffe00,
        0x007ffffffe00,
        0x007ffffffc00,
        0x007ffffff000,
        0x003ffffff000,
        0x001fffffe000,
        0x001fffffe000,
        0x0007ffffc000,
        0x0003ffff8000,
        0x0001ffff0000,
        0x00007ffe0000,
        0x00003ffc0000,
        0x00001ff80000,
        0x00004e000000,
        0x000000900000,
        0x000001800000,
        0x000000800000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
    ],
    [
        0x000000000000,
        0x000000000000,
        0x000000c00000,
        0x000001c00000,
        0x000021f00000,
        0x000031f80000,
        0x000033fc0000,
        0x00004bfd0000,
        0x00005ffe0000,
        0x00001ffe0000,
        0x0003fffe0000,
        0x000fdffc0000,
        0x001fffff8000,
        0x001fffff8000,
        0x003fffffc000,
        0x003fffffe000,
        0x003fffffc000,
        0x001fffffc000,
        0x001fffffc000,
        0x001fffffe000,
        0x001ffffff000,
        0x000ffffff000,
        0x000ffffff000,
        0x0007fffff000,
        0x0003fffff000,
        0x0001fffff000,
        0x0001ffffe000,
        0x0000ffff8000,
        0x0000fffc0000,
        0x00007ff80000,
        0x00000ff00000,
        0x00000fe00000,
        0x000007800000,
        0x000003000000,
        0x000003000000,
        0x000002400000,
        0x000008400000,
        0x000001400000,
        0x000001000000,
        0x000000000000,
        0x000002000000,
        0x000001000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
    ],
];

#[cfg(test)]
mod source_player_explosion_step {
    use super::*;

    #[test]
    fn game_step_queries_overlapping_explosions_once_with_literal_survivor_counter() {
        for count in [1, 2] {
            for (hp, real_room, expected_count) in [(20, true, 1), (4, true, 0), (20, false, 0)] {
                let mut game = Game::shell(7, RulesPreset::Classic);
                game.player.pos = Vec2::new(160.0, 160.0);
                game.player.previous_pos = game.player.pos;
                game.player.health = hp;
                if !real_room {
                    game.flags.insert("tutorial".into());
                }
                for n in 0..count {
                    game.world
                        .spawn("oExplosion", Vec2::new(159.0 + 2.0 * f64::from(n), 160.0));
                }
                let mut expected_rng = game.rng.clone();
                let speed = 4.0 + ((expected_rng.next_u64() * 3) >> 32) as f64;
                // Source Player Step emits one Blood, independently of the
                // number of overlapping blasts or whether damage is fatal.
                // Original raw32/2^32 and multiply-high selectors form this
                // independent oracle; each occurrence consumes one raw word.
                let blood_velocity = Vec2::new(
                    (expected_rng.next_u64() as f64 / 4_294_967_296.0) * 4.0
                        - (expected_rng.next_u64() as f64 / 4_294_967_296.0) * 4.0,
                    -1.0 - (expected_rng.next_u64() as f64 / 4_294_967_296.0) * 2.0,
                );
                let blood_gravity = (1.0 + ((expected_rng.next_u64() * 6) >> 32) as f64) * 0.1;
                game.step(&InputFrame::default());
                assert_eq!(
                    game.player.health,
                    (hp - 10).max(0),
                    "blasts={count} hp={hp} real={real_room}"
                );
                assert_eq!(
                    game.events
                        .iter()
                        .filter(|event| matches!(event, GameEvent::Damage { amount: 10 }))
                        .count(),
                    1
                );
                assert_eq!(game.player.vel, Vec2::new(speed, -6.0));
                assert_eq!(
                    game.player.data.get("bounced").copied().unwrap_or(0.0),
                    0.0,
                    "airborne fixture excludes the independent late-bottom script Blood3"
                );
                let blood: Vec<_> = game
                    .world
                    .entities
                    .iter()
                    .filter(|e| e.alive && e.object == "oBlood")
                    .collect();
                assert_eq!(blood.len(), 1, "blasts={count} hp={hp} real={real_room}");
                assert_eq!(blood[0].vel, blood_velocity);
                assert_eq!(blood[0].value("gravity"), blood_gravity);
                assert_eq!(game.rng.next_u64(), expected_rng.next_u64());
                assert_eq!(
                    game.lifetime_stats.misc_deaths
                        [crate::statistics::MiscDeath::Explosion as usize],
                    expected_count
                );
            }
        }
    }

    #[test]
    fn game_step_bomb_alarm_zero_overwrites_a_pending_short_alarm_one() {
        let mut game = Game::shell(7, RulesPreset::Classic);
        game.player.pos = Vec2::new(160.0, 160.0);
        game.world.set_tile(10, 11, "oHardBlock");
        game.world.set_tile(5, 6, "oHardBlock");
        let id = game.world.spawn("oBomb", Vec2::new(88.0, 88.0));
        let bomb = game.world.entity_mut(id).unwrap();
        arm_bomb(bomb);
        bomb.alarms[0] = 1;
        bomb.alarms[1] = 2;
        // Alarm-number order is observable: Alarm 0 writes 40 before Alarm 1's
        // same-frame decrement, extending a previously shortened fuse.
        game.step(&InputFrame::default());
        let bomb = game.world.entity(id).unwrap();
        assert_eq!(bomb.alarms[0], 0);
        assert_eq!(bomb.alarms[1], 39);
        assert_eq!(bomb.image_speed(), 1.0);
        for _ in 0..38 {
            game.step(&InputFrame::default());
        }
        assert_eq!(game.world.entity(id).unwrap().alarms[1], 1);
        game.step(&InputFrame::default());
        assert!(game.world.entity(id).is_none());
        assert_eq!(
            game.events
                .iter()
                .filter(|e| matches!(e, GameEvent::Explosion { .. }))
                .count(),
            1
        );
    }

    #[test]
    fn game_step_clean_altar_destroy_still_defiles_in_its_synchronous_callback() {
        for object in ["oSacAltarLeft", "oSacAltarRight"] {
            for dynamic in [false, true] {
                let mut game = Game::shell(7, RulesPreset::Classic);
                game.player.pos = Vec2::new(160.0, 160.0);
                game.flags.insert("cleanSolids".into());
                if dynamic {
                    game.world.spawn(object, Vec2::new(80.0, 80.0));
                } else {
                    game.world.set_tile(5, 5, object);
                }
                game.world.spawn("oExplosion", Vec2::new(88.0, 88.0));
                let mut expected_rng = game.rng.clone();
                game.step(&InputFrame::default());
                assert_eq!(
                    game.data.get("favor").copied(),
                    Some(-16.0),
                    "{object} dynamic={dynamic}"
                );
                assert_eq!(game.data.get("kali_punish").copied(), Some(1.0));
                assert!(
                    !game
                        .world
                        .entities
                        .iter()
                        .any(|e| matches!(e.object.as_str(), "oRubble" | "oRubbleSmall"))
                );
                assert_eq!(game.rng.next_u64(), expected_rng.next_u64());
            }
        }
    }

    #[test]
    fn player_step_uses_bounding_box_query_and_nearest_across_all_explosions() {
        let mut game = Game::shell(7, RulesPreset::Classic);
        game.player.health = 30;
        game.player.pos = Vec2::new(160.0, 160.0);
        // Late frame has a narrower bbox: this nearer instance does not overlap.
        let near = game.world.spawn("oExplosion", Vec2::new(181.0, 160.0));
        game.world.entity_mut(near).unwrap().animation = 9.0;
        // Earlier frame is broad enough to overlap from a more distant origin.
        let far = game.world.spawn("oExplosion", Vec2::new(136.0, 160.0));
        game.world.entity_mut(far).unwrap().animation = 3.0;
        let probe = Rect::new(152.0, 152.0, 17.0, 17.0);
        assert!(
            !game
                .world
                .entity(near)
                .unwrap()
                .mask_intersects(probe, CollisionPrecision::BoundingBox)
        );
        assert!(
            game.world
                .entity(far)
                .unwrap()
                .mask_intersects(probe, CollisionPrecision::BoundingBox)
        );
        player_explosion_hurt(&mut game);
        assert_eq!(game.player.health, 20);
        assert!(
            game.player.vel.x < 0.0,
            "nearest query includes the nonoverlapping explosion to the right"
        );
    }
}

#[cfg(test)]
mod source_solid_self_destroy {
    use super::*;

    #[test]
    fn self_destroy_bypasses_attack_immunity_for_cells_and_instances_exactly_once() {
        for object in ["oTrapBlock", "oHardBlock"] {
            for dynamic in [false, true] {
                let mut game = Game::shell(7, RulesPreset::Classic);
                let id = if dynamic {
                    game.world.spawn(object, Vec2::new(80.0, 80.0))
                } else {
                    game.world.set_tile(5, 5, object);
                    game.world.spawn("oTerrainMetadata", Vec2::new(88.0, 88.0))
                };
                game.world
                    .entity_mut(id)
                    .unwrap()
                    .data
                    .insert("invincible".into(), 1.0);
                let mut expected_rng = game.rng.clone();
                if dynamic {
                    assert!(!destroy_solid_instance(&mut game, id));
                } else {
                    destroy_terrain(&mut game, 5, 5);
                    assert_eq!(game.world.tile(5, 5).unwrap().object, object);
                }
                assert!(game.world.entity(id).is_some());
                assert!(game.events.is_empty());
                if object == "oTrapBlock" {
                    for _ in 0..12 {
                        expected_rng.next_u64();
                    }
                }
                if dynamic {
                    // Some source own Steps have already marked the instance
                    // dead when the stored callback is dispatched.
                    game.world.entity_mut(id).unwrap().alive = false;
                    assert!(destroy_solid_self(&mut game, id));
                    assert!(!destroy_solid_self(&mut game, id));
                } else {
                    assert!(destroy_terrain_self(&mut game, 5, 5));
                    assert!(!destroy_terrain_self(&mut game, 5, 5));
                    assert!(!destroy_terrain_self(&mut game, usize::MAX, 5));
                }
                assert_eq!(
                    game.world
                        .entities
                        .iter()
                        .filter(|e| matches!(e.sprite.as_str(), "sRubbleTan" | "sRubbleTanSmall"))
                        .count(),
                    if object == "oTrapBlock" { 3 } else { 0 }
                );
                assert_eq!(game.rng.next_u64(), expected_rng.next_u64());
            }
        }
    }

    #[test]
    fn explosion_and_trap_destroy_preserve_the_longer_source_camera_shake() {
        for existing in [0.0_f64, 3.0, 20.0] {
            for (object, duration) in [("oExplosion", 5.0), ("oTrapBlock", 10.0)] {
                let mut game = Game::shell(7, RulesPreset::Classic);
                game.data.insert("shake".into(), existing);
                let id = game.spawn(object, Vec2::new(80.0, 80.0));
                if object == "oTrapBlock" {
                    game.world
                        .entity_mut(id)
                        .unwrap()
                        .data
                        .insert("dying".into(), 1.0);
                    assert!(destroy_solid_self(&mut game, id));
                }
                assert_eq!(game.data["shake"], existing.max(duration));
            }
        }
    }

    #[test]
    fn trap_block_thump_and_shake_share_the_source_debris_clean_guard() {
        for clean in ["none", "cleanDeath", "cleanSolids"] {
            for dying in [false, true] {
                let mut game = Game::shell(7, RulesPreset::Classic);
                let id = game.world.spawn("oTrapBlock", Vec2::new(80.0, 80.0));
                game.world
                    .entity_mut(id)
                    .unwrap()
                    .data
                    .insert("dying".into(), f64::from(dying));
                if clean == "cleanDeath" {
                    game.world
                        .entity_mut(id)
                        .unwrap()
                        .data
                        .insert("cleanDeath".into(), 1.0);
                }
                if clean == "cleanSolids" {
                    game.flags.insert("cleanSolids".into());
                }
                let mut expected_rng = game.rng.clone();
                if clean == "none" {
                    for _ in 0..12 {
                        expected_rng.next_u64();
                    }
                }
                assert!(destroy_solid_self(&mut game, id));
                assert_eq!(
                    game.events
                        .iter()
                        .filter(
                            |e| matches!(e, GameEvent::Sound { name, .. } if name == "sndThump")
                        )
                        .count(),
                    usize::from(clean == "none" && dying)
                );
                assert_eq!(
                    game.data.get("shake").copied(),
                    (clean == "none" && dying).then_some(10.0)
                );
                assert_eq!(
                    game.world
                        .entities
                        .iter()
                        .filter(|e| matches!(e.sprite.as_str(), "sRubbleTan" | "sRubbleTanSmall"))
                        .count(),
                    if clean == "none" { 3 } else { 0 }
                );
                assert_eq!(game.rng.next_u64(), expected_rng.next_u64());
            }
        }
    }
}

#[cfg(test)]
mod runtime_clean_terrain_treasure {
    use super::*;

    #[test]
    fn runtime_clean_guards_suppress_destroy_tags_but_preserve_create_contents() {
        for clean in ["none", "cleanDeath", "global_flag", "global_data"] {
            for treasure in ["Big Ruby", "oRubyBig", "oJetpack"] {
                let mut game = Game::shell(7, RulesPreset::Classic);
                game.world.set_tile(5, 5, "oTemple");
                let id = game.world.spawn("oTerrainMetadata", Vec2::new(88.0, 88.0));
                let metadata = game.world.entity_mut(id).unwrap();
                metadata.data.insert("embedded_treasure".into(), 1.0);
                metadata.text.insert("treasure".into(), treasure.into());
                if clean == "cleanDeath" {
                    metadata.data.insert("cleanDeath".into(), 1.0);
                }
                if treasure.starts_with('o') {
                    metadata.data.insert("treasure_dx".into(), -2.0);
                    metadata.data.insert("treasure_dy".into(), 3.0);
                }
                if clean == "global_flag" {
                    game.flags.insert("cleanSolids".into());
                } else if clean == "global_data" {
                    game.data.insert("cleanSolids".into(), 1.0);
                }
                let spikes = game.world.spawn("oSpikes", Vec2::new(80.0, 64.0));
                let mut expected = game.rng.clone();
                if clean == "none" {
                    // Three rubble origins, each with four source rand(0,8).
                    for _ in 0..12 {
                        expected.range(9);
                    }
                }
                assert!(destroy_terrain_self(&mut game, 5, 5));
                assert!(!destroy_terrain_self(&mut game, 5, 5));
                assert!(game.world.tile(5, 5).unwrap().object.is_empty());
                assert!(game.world.entity(id).is_none());
                assert!(
                    game.world.entity(spikes).is_none(),
                    "inherited Destroy still runs"
                );
                assert!(game.world.fluid_dirty);
                let output_object = if treasure == "Big Ruby" {
                    "oRubyBig"
                } else {
                    treasure
                };
                let outputs: Vec<_> = game
                    .world
                    .entities
                    .iter()
                    .filter(|e| e.alive && e.object == output_object)
                    .collect();
                let should_release = clean == "none" || treasure.starts_with('o');
                assert_eq!(
                    outputs.len(),
                    usize::from(should_release),
                    "{clean}/{treasure}"
                );
                if treasure.starts_with('o') {
                    assert_eq!(outputs[0].pos, Vec2::new(86.0, 91.0));
                    assert_eq!(outputs[0].state, 1);
                }
                assert_eq!(
                    game.world
                        .entities
                        .iter()
                        .filter(
                            |e| e.alive && matches!(e.object.as_str(), "oRubble" | "oRubbleSmall")
                        )
                        .count(),
                    if clean == "none" { 3 } else { 0 }
                );
                assert_eq!(
                    game.rng.next_u64(),
                    expected.next_u64(),
                    "{clean}/{treasure}"
                );
            }
        }
    }

    #[test]
    fn game_step_explosion_respects_clean_temple_destroy_treasure_guard() {
        for clean in [false, true] {
            let mut game = Game::shell(7, RulesPreset::Classic);
            game.player.pos = Vec2::new(200.0, 200.0);
            game.world.set_tile(5, 5, "oTemple");
            let id = game.world.spawn("oTerrainMetadata", Vec2::new(88.0, 88.0));
            let metadata = game.world.entity_mut(id).unwrap();
            metadata.data.insert("embedded_treasure".into(), 1.0);
            metadata.data.insert("cleanDeath".into(), f64::from(clean));
            metadata.text.insert("treasure".into(), "Big Ruby".into());
            game.spawn("oExplosion", Vec2::new(88.0, 88.0));
            game.step(&InputFrame::default());
            assert!(game.world.tile(5, 5).unwrap().object.is_empty());
            assert!(game.world.entity(id).is_none());
            assert_eq!(
                game.world
                    .entities
                    .iter()
                    .filter(|e| e.alive && e.object == "oRubyBig")
                    .count(),
                usize::from(!clean)
            );
            assert_eq!(game.player.health, 4);
        }
    }
}

#[cfg(test)]
#[path = "explosion_laser_contract_tests.rs"]
mod explosion_laser_contract_tests;

#[cfg(test)]
mod spear_destroy_source_oracle {
    use super::*;

    #[test]
    fn own_spear_destroy_preserves_literal_fragments_rng_guards_and_no_parent_effects() {
        for object in ["oSpearTrapTop", "oSpearTrapBottom", "oSpearTrapLit"] {
            for clean_death in [false, true] {
                for global_clean in ["none", "flag", "data", "both"] {
                    for detritus_count in [0, 32] {
                        let mut game = Game::shell(59, RulesPreset::Classic);
                        for _ in 0..detritus_count {
                            game.spawn("oBone", Vec2::new(400.0, 400.0));
                        }
                        let receiver = game.spawn(object, Vec2::new(160.0, 80.0));
                        {
                            let entity = game.world.entity_mut(receiver).unwrap();
                            entity
                                .data
                                .insert("cleanDeath".into(), f64::from(clean_death));
                            entity.data.insert("shopWall".into(), 1.0);
                        }
                        // These parent-only support effects must not happen:
                        // neither own Destroy includes action_inherited.
                        let torch = game.spawn("oTikiTorch", Vec2::new(160.0, 48.0));
                        let lamp = game.spawn("oLamp", Vec2::new(160.0, 96.0));
                        assert!(
                            game.world
                                .entity(torch)
                                .unwrap()
                                .sprite_bounds()
                                .contains(Vec2::new(168.0, 79.0))
                        );
                        assert!(
                            game.world
                                .entity(lamp)
                                .unwrap()
                                .sprite_bounds()
                                .contains(Vec2::new(168.0, 98.0))
                        );
                        if matches!(global_clean, "flag" | "both") {
                            game.flags.insert("cleanSolids".into());
                        }
                        if matches!(global_clean, "data" | "both") {
                            game.data.insert("cleanSolids".into(), 1.0);
                        }
                        let enabled = !clean_death && global_clean == "none";
                        let mut oracle = game.rng.clone();
                        let expected: Vec<_> = (0..if enabled { 3 } else { 0 })
                            .map(|_| {
                                Vec2::new(
                                    168.0 + f64::from(oracle.range(9)) - f64::from(oracle.range(9)),
                                    88.0 + f64::from(oracle.range(9)) - f64::from(oracle.range(9)),
                                )
                            })
                            .collect();
                        assert!(destroy_solid_self(&mut game, receiver));
                        let actual: Vec<_> = game
                            .world
                            .entities
                            .iter()
                            .filter(|e| crate::catalog::is_a(&e.object, "oRubblePiece"))
                            .collect();
                        assert_eq!(
                            actual.len(),
                            expected.len(),
                            "{object} clean={clean_death}/{global_clean} priorDetritus={detritus_count}"
                        );
                        for ((piece, position), (kind, sprite)) in
                            actual.iter().zip(expected).zip([
                                ("oRubble", "sRubbleTan"),
                                ("oRubbleSmall", "sRubbleTanSmall"),
                                ("oRubbleSmall", "sRubbleTanSmall"),
                            ])
                        {
                            assert!(piece.alive && piece.engine_active);
                            assert_eq!(piece.object, kind);
                            assert_eq!(piece.sprite, sprite);
                            assert_eq!(piece.pos, position);
                            assert_eq!(piece.vel, Vec2::default());
                            assert_eq!(piece.value("gravity"), 0.6);
                            assert!(
                                !crate::catalog::is_a(&piece.object, "oDetritus"),
                                "Rubble has no Detritus cap"
                            );
                        }
                        assert!(game.world.entity(torch).is_some());
                        assert!(game.world.entity(lamp).is_some());
                        assert!(!game.flags.contains("shopkeepers_angry"));
                        assert!(!game.world.entities.iter().any(|e| e.object == "oLampItem"));
                        assert_eq!(game.rng.clone().next_u64(), oracle.next_u64());
                        let count = game.world.entities.len();
                        let mut repeat_oracle = game.rng.clone();
                        assert!(!destroy_solid_self(&mut game, receiver));
                        assert_eq!(game.world.entities.len(), count);
                        assert_eq!(game.rng.next_u64(), repeat_oracle.next_u64());
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod rope_action_time_source_companions {
    use super::*;

    fn naturally_reserved() -> (Game, u64) {
        let mut game = Game::shell(7, RulesPreset::Classic);
        game.world = World::new(30, 20, Area::Mines, 1);
        game.world.name = "rLevel".into();
        for x in 0..game.world.width {
            game.world.set_tile(x, 10, "oHardBlock");
        }
        game.player.pos = Vec2::new(88.0, 152.0);
        game.player.previous_pos = game.player.pos;
        game.player.state = PlayerState::Standing;
        game.player.grounded = true;
        game.player.facing = 1;
        let rock = game.spawn("oRock", game.player.pos);
        game.step(&InputFrame {
            down: true,
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert_eq!(game.player.held, Some(rock));
        game.step(&InputFrame::default());
        game.step(&InputFrame {
            cycle_pressed: true,
            ..Default::default()
        });
        game.step(&InputFrame::default());
        game.step(&InputFrame {
            cycle_pressed: true,
            ..Default::default()
        });
        game.step(&InputFrame::default());
        assert!(game.world.entity(rock).is_none());
        assert_eq!(game.player.pickup_item_type, "Rock");
        assert_eq!(game.player.ropes, 3);
        let reserved = game.player.held.unwrap();
        let rope = game.world.entity(reserved).unwrap();
        assert_eq!(rope.object, "oRopeThrow");
        assert!(rope.held);
        assert_eq!(rope.pos, Vec2::new(92.0, 154.0));
        (game, reserved)
    }

    #[test]
    fn held_rope_up_action_keeps_old_y_and_same_actor_before_469_step() {
        let (mut game, reserved) = naturally_reserved();
        let old = game.world.entity(reserved).unwrap().clone();
        let player_pos = game.player.pos;
        let next_id = game.world.next_id;
        let mut expected_rng = game.rng.clone();
        assert!(use_held_rope(&mut game, reserved, &InputFrame::default()));
        let launched = game.world.entity(reserved).unwrap();
        assert!(!launched.held);
        assert_eq!(launched.pos, Vec2::new(player_pos.x, old.pos.y));
        assert_ne!(launched.pos.y, player_pos.y);
        assert_eq!(launched.vel, Vec2::new(0.0, -12.0));
        assert_eq!(launched.value("armed"), 1.0);
        assert_eq!(launched.value("px"), player_pos.x);
        assert_eq!(launched.value("py"), player_pos.y);
        assert_eq!(launched.value("falling"), 0.0);
        assert_eq!(game.player.ropes, 3);
        let restored = game.world.entity(game.player.held.unwrap()).unwrap();
        assert_eq!(restored.id, next_id);
        assert_eq!(restored.object, "oRock");
        assert!(restored.held);
        assert_eq!(restored.value("cost"), 0.0);
        assert_eq!(game.rng.next_u64(), expected_rng.next_u64());
    }

    #[test]
    fn held_rope_failed_anchor_creates_fresh_fallback_at_old_actor_position() {
        let (mut game, reserved) = naturally_reserved();
        let old_pos = game.world.entity(reserved).unwrap().pos;
        let attempt = game.world.next_id;
        let mut expected_rng = game.rng.clone();
        // The full floor intersects both inclusive y..y+16 side rectangles.
        assert!(use_held_rope(
            &mut game,
            reserved,
            &InputFrame {
                down: true,
                ..Default::default()
            }
        ));
        assert!(game.world.entity(reserved).is_none());
        assert!(game.world.entity(attempt).is_none());
        let fallback = game.world.entity(attempt + 1).unwrap();
        assert_eq!(fallback.object, "oRopeThrow");
        assert_eq!(fallback.pos, old_pos);
        assert_ne!(fallback.pos, game.player.pos);
        assert_eq!(fallback.vel, Vec2::new(3.2, 0.5));
        assert!(!fallback.held);
        for key in ["armed", "falling", "fall_count", "px", "py"] {
            assert_eq!(fallback.value(key), 0.0, "fresh fallback {key}");
        }
        assert_eq!(fallback.age, 0);
        assert_eq!(fallback.alarms, [-1; 12]);
        assert!(
            !game
                .world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oRopeTop")
        );
        assert_eq!(game.player.ropes, 3);
        let restored = game.world.entity(game.player.held.unwrap()).unwrap();
        assert_eq!(restored.id, attempt + 2);
        assert_eq!(restored.object, "oRock");
        assert_eq!(game.world.next_id, attempt + 3);
        assert_eq!(game.rng.next_u64(), expected_rng.next_u64());
    }

    #[test]
    fn dedicated_failed_anchor_runs_create_without_spending_or_replacing_reservation() {
        let (mut game, reserved) = naturally_reserved();
        let before = game.world.entity(reserved).unwrap().clone();
        let attempt = game.world.next_id;
        let mut expected_rng = game.rng.clone();
        use_rope(
            &mut game,
            &InputFrame {
                down: true,
                rope_pressed: true,
                ..Default::default()
            },
        );
        assert_eq!(
            game.world.next_id,
            attempt + 1,
            "failed Create consumes identity"
        );
        assert!(game.world.entity(attempt).is_none());
        assert_eq!(game.player.held, Some(reserved));
        let still_held = game.world.entity(reserved).unwrap();
        assert!(still_held.held);
        assert_eq!(still_held.pos, before.pos);
        assert_eq!(still_held.vel, before.vel);
        assert_eq!(still_held.value("armed"), 0.0);
        assert_eq!(game.player.ropes, 3);
        assert_eq!(game.player.pickup_item_type, "Rock");
        assert!(
            !game
                .world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oRopeTop")
        );
        assert_eq!(game.rng.next_u64(), expected_rng.next_u64());
    }
}

#[cfg(test)]
mod held_rope_literal_clearance_companion {
    use super::*;
    #[test]
    fn neither_two_pixel_point_clear_does_not_clear_source_attempt_t() {
        let mut game = Game::shell(7, RulesPreset::Classic);
        game.world = World::new(30, 20, Area::Mines, 1);
        for x in 0..game.world.width {
            game.world.set_tile(x, 10, "oHardBlock");
        }
        game.player.pos = Vec2::new(152.0, 152.0);
        game.player.previous_pos = game.player.pos;
        game.player.state = PlayerState::Standing;
        game.player.grounded = true;
        game.step(&InputFrame {
            cycle_pressed: true,
            ..Default::default()
        });
        game.step(&InputFrame::default());
        game.step(&InputFrame {
            cycle_pressed: true,
            ..Default::default()
        });
        let reserved = game.player.held.unwrap();
        // Controlled adjacent Solid instances leave the Player center open.
        // This companion captures the source action, before later physics/hurt.
        game.spawn("oBrickSmooth", Vec2::new(135.0, 144.0));
        game.spawn("oBrickSmooth", Vec2::new(154.0, 144.0));
        let attempt = game.world.next_id;
        let mut expected_rng = game.rng.clone();
        assert!(use_held_rope(
            &mut game,
            reserved,
            &InputFrame {
                down: true,
                ..Default::default()
            }
        ));
        assert!(game.world.entity(reserved).is_none());
        let rope = game.world.entity(attempt).unwrap();
        assert_eq!(rope.pos, Vec2::new(160.0, 152.0));
        assert_eq!(rope.value("t"), 1.0);
        assert_eq!(rope.value("falling"), 1.0);
        assert_eq!(game.world.entity(attempt + 1).unwrap().object, "oRopeTop");
        assert_eq!(game.world.next_id, attempt + 2);
        assert_eq!(game.rng.next_u64(), expected_rng.next_u64());
    }
}

#[cfg(test)]
#[path = "explosives_trap_destroy_tests.rs"]
mod trap_destroy_tests;

#[cfg(test)]
#[path = "sticky_bomb_contract_tests.rs"]
mod sticky_bomb_contract_tests;

#[cfg(test)]
#[path = "player_explosion_blood_contract_tests.rs"]
mod player_explosion_blood_contract_tests;

#[cfg(test)]
#[path = "firefrog_bomb_own_step_tests.rs"]
mod firefrog_bomb_own_step_tests;
