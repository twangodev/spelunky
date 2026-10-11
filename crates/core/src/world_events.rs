//! Source world objects and visual effects that have no item/enemy/trap owner.
//! All animation rates below are frames per original 30 Hz simulation tick.
use crate::{Entity, Game, GameEvent, InputFrame, Rect, VIEW_HEIGHT, VIEW_WIDTH, Vec2, catalog};

/// GameMaker's distance_to_object measures between inclusive bounding boxes.
pub(crate) fn mask_distance(a: Rect, b: Rect) -> f64 {
    if !a.is_valid() || !b.is_valid() {
        return f64::INFINITY;
    }
    let dx = (a.x - (b.x + b.w - 1.0))
        .max(b.x - (a.x + a.w - 1.0))
        .max(0.0);
    let dy = (a.y - (b.y + b.h - 1.0))
        .max(b.y - (a.y + a.h - 1.0))
        .max(0.0);
    dx.hypot(dy)
}

mod light;
mod temple;
pub(crate) use light::{lighting_tick, sample_light_distances};

/// oGame318 fluid pass runs before Player319 and Level553 activation.
/// Destroying a solid requests this pass; a quiet contained pool stays intact.
pub(crate) fn fluid_tick(game: &mut Game) {
    if !game.world.fluid_dirty {
        game.data.insert("water_loop_safety".into(), 0.0);
        return;
    }
    let custom = game.flags.contains("custom");
    if game.world.area == crate::Area::Boss && !custom {
        game.world.fluid_dirty = false;
        return;
    }
    // Canonical with(object) snapshots its count, but rereads the live
    // descendant array at each index. Region activation can rebuild that array.
    let count = crate::dispatcher::descendant_instances(game, "oWater").len();
    let mut drained = 0;
    for cursor in 0..count {
        let Some(instance) = crate::dispatcher::descendant_instances(game, "oWater")
            .get(cursor)
            .copied()
        else {
            continue;
        };
        if !crate::dispatcher::is_active(game, instance) {
            continue;
        }
        let object = instance.object(game).to_owned();
        if !catalog::is_a(&object, "oWater") {
            continue;
        }
        let origin = instance.position(game);
        if !custom && origin.y >= 512.0 {
            continue;
        }
        let id = match instance {
            crate::dispatcher::Instance::Entity(id) => Some(id),
            _ => None,
        };
        if id.is_some_and(|id| game.world.entity(id).is_none()) {
            continue;
        }
        crate::dispatcher::activate_region(
            game,
            Rect::new(origin.x - 16.0, origin.y - 16.0, 48.0, 48.0),
        );
        let fluids: Vec<_> = crate::dispatcher::descendant_instances(game, "oWater")
            .into_iter()
            .filter(|i| crate::dispatcher::is_active(game, *i))
            .map(|i| {
                let id = match i {
                    crate::dispatcher::Instance::Entity(id) => Some(id),
                    _ => None,
                };
                (id, i.position(game), i.object(game).to_owned())
            })
            .collect();
        let above = Vec2::new(origin.x, origin.y - 16.0);
        let exposed = !point_family(game, above, "oSolid") && !point_family(game, above, "oWater");
        if exposed {
            surface_fluid(game, id, origin, &object);
        }
        for dx in [-16.0, 16.0] {
            let mask = id
                .and_then(|id| game.world.entity(id).map(Entity::sprite_bounds))
                .unwrap_or_else(|| {
                    game.world
                        .tile_bounds((origin.x / 16.0) as i32, (origin.y / 16.0) as i32)
                });
            let top = {
                let probe = Rect::new(mask.x + dx, mask.y, mask.w, mask.h);
                // instance_place returns one instance, rather than accepting a
                // later surface behind an earlier overlapping non-surface cell.
                fluids
                    .iter()
                    .find_map(|(neighbor_id, neighbor_pos, _)| {
                        if let Some(id) = neighbor_id {
                            let neighbor = game.world.entity(*id)?;
                            return neighbor.sprite_bounds().intersects(probe).then(|| {
                                matches!(neighbor.sprite.as_str(), "sWaterTop" | "sLavaTop")
                            });
                        }
                        let x = (neighbor_pos.x / 16.0) as i32;
                        let y = (neighbor_pos.y / 16.0) as i32;
                        if !game
                            .world
                            .tile(x, y)
                            .is_some_and(|tile| catalog::is_a(&tile.object, "oWater"))
                            || !game.world.tile_bounds(x, y).intersects(probe)
                        {
                            return None;
                        }
                        let sprite = game
                            .world
                            .entities
                            .iter()
                            .find(|e| {
                                e.alive
                                    && e.object == "oTerrainMetadata"
                                    && (e.pos.x / 16.0).floor() == f64::from(x)
                                    && (e.pos.y / 16.0).floor() == f64::from(y)
                            })
                            .and_then(|e| e.text.get("terrain_sprite"));
                        Some(sprite.is_some_and(|sprite| {
                            matches!(sprite.as_str(), "sWaterTop" | "sLavaTop")
                        }))
                    })
                    .unwrap_or(false)
            };
            if top {
                surface_fluid(game, id, origin, &object);
            }
        }
        if [
            Vec2::new(origin.x - 16.0, origin.y),
            Vec2::new(origin.x + 16.0, origin.y),
            Vec2::new(origin.x, origin.y + 16.0),
        ]
        .into_iter()
        .any(|point| !point_family(game, point, "oSolid") && !point_family(game, point, "oWater"))
        {
            if let Some(id) = id {
                game.world.entity_mut(id).unwrap().alive = false;
            } else {
                game.world
                    .set_tile((origin.x / 16.0) as usize, (origin.y / 16.0) as usize, "");
            }
            for _ in 0..3 {
                let pos = Vec2::new(
                    origin.x + f64::from(game.rng.range(17)),
                    origin.y + f64::from(game.rng.range(17)),
                );
                game.world.spawn(
                    if object == "oLava" {
                        "oLavaDrip"
                    } else {
                        "oDrip"
                    },
                    pos,
                );
            }
            if object == "oLava" && game.rng.chance(1, 6) {
                let id = game
                    .world
                    .spawn("oFlame", Vec2::new(origin.x + 8.0, origin.y + 8.0));
                let mut flame = game.world.entity(id).unwrap().clone();
                initialize_detritus(game, &mut flame);
                flame.vel.y = 4.0;
                if !flame.alive {
                    game.world.spawn("oSmokePuff", flame.pos);
                }
                let index = game.world.entities.iter().position(|e| e.id == id).unwrap();
                game.world.entities[index] = flame;
            }
            drained += 1;
        }
        let safety = game.data.entry("water_loop_safety".into()).or_default();
        *safety += 1.0;
        if *safety > 100_000.0 {
            game.world.fluid_dirty = false;
        }
    }
    game.data.insert("water_counter".into(), f64::from(drained));
    if drained == 0 {
        game.world.fluid_dirty = false;
    }
}

fn surface_fluid(game: &mut Game, id: Option<u64>, origin: Vec2, object: &str) {
    let sprite = if object == "oLava" {
        "sLavaTop"
    } else {
        "sWaterTop"
    };
    if let Some(id) = id {
        game.world.entity_mut(id).unwrap().sprite = sprite.into();
    } else {
        let metadata = game
            .world
            .entities
            .iter()
            .find(|e| {
                e.alive
                    && e.object == "oTerrainMetadata"
                    && (e.pos.x / 16.0).floor() == origin.x / 16.0
                    && (e.pos.y / 16.0).floor() == origin.y / 16.0
            })
            .map(|e| e.id)
            .unwrap_or_else(|| {
                let id = game.world.spawn(
                    "oTerrainMetadata",
                    Vec2::new(origin.x + 8.0, origin.y + 8.0),
                );
                game.world.entity_mut(id).unwrap().sprite.clear();
                id
            });
        game.world
            .entity_mut(metadata)
            .unwrap()
            .text
            .insert("terrain_sprite".into(), sprite.into());
    }
}

/// oScreen Begin Step runs before normal collision refreshes a message.
pub(crate) fn begin_step(game: &mut Game) {
    if let Some(timer) = game.data.get_mut("message_timer") {
        *timer = (*timer - 1.0).max(0.0);
    }
}

pub(crate) fn prepare_tick(game: &mut Game) {
    // Generated/imported producers register these at Create time. A stable
    // grid-order fallback also supports raw World::set_tile fixtures/authors.
    let mut registered_trees = vec![false; game.world.tiles.len()];
    for e in &game.world.entities {
        if e.alive && e.object == "oTerrainMetadata" && e.value("tree_initialized") != 0.0 {
            let x = (e.pos.x / 16.0).floor() as usize;
            let y = (e.pos.y / 16.0).floor() as usize;
            if x < game.world.width && y < game.world.height {
                registered_trees[y * game.world.width + x] = true;
            }
        }
    }
    let raw_trees: Vec<_> = game
        .world
        .tiles
        .iter()
        .enumerate()
        .filter_map(|(i, tile)| {
            if tile.object != "oTree" {
                return None;
            }
            let x = i % game.world.width;
            let y = i / game.world.width;
            (!registered_trees[i]).then_some((x, y))
        })
        .collect();
    for (x, y) in raw_trees {
        initialize_tree(&mut game.world, x, y);
    }
    for e in &mut game.world.entities {
        if e.object == "oIceBottom" && e.value("drip_alarm_adapted") == 0.0 {
            if let Some(value) = e.data.get("drip_alarm") {
                e.alarms[0] = *value as i32;
                e.data.insert("drip_alarm_adapted".into(), 1.0);
            }
        }
        if e.object == "oTerrainMetadata" {
            let x = (e.pos.x / 16.0).floor() as i32;
            let y = (e.pos.y / 16.0).floor() as i32;
            let index = (x >= 0
                && y >= 0
                && (x as usize) < game.world.width
                && (y as usize) < game.world.height)
                .then_some(y.max(0) as usize * game.world.width + x.max(0) as usize);
            if let Some(tile) = index.and_then(|index| game.world.tiles.get(index)) {
                if tile.object == "oLava" {
                    e.data.entry("animation_speed".into()).or_insert(0.4);
                    if let Some(object) = catalog::object(&tile.object) {
                        e.text
                            .entry("terrain_sprite".into())
                            .or_insert_with(|| object.sprite.clone());
                    }
                }
            }
        }
    }
}

pub(crate) fn source_object(game: &Game, id: u64) -> Option<String> {
    let e = game.world.entity(id)?;
    if e.object != "oTerrainMetadata" {
        return Some(e.object.clone());
    }
    let x = (e.pos.x / 16.0).floor() as i32;
    let y = (e.pos.y / 16.0).floor() as i32;
    let object = &game.world.tile(x, y)?.object;
    matches!(object.as_str(), "oTree" | "oLava").then(|| object.clone())
}

pub(crate) fn handles(object: &str) -> bool {
    matches!(
        object,
        "oMsgSign"
            | "oGrave"
            | "oTempleFake"
            | "oTemple"
            | "oLamp"
            | "oLampRed"
            | "oTikiTorch"
            | "oHintHand"
            | "oYellHelp"
            | "oLeaves"
            | "oTreeBranch"
            | "oTree"
            | "oPsychicCreateP"
            | "oPsychicCreate2"
            | "oIceBottom"
            | "oFrozenCaveman"
            | "oBlood"
            | "oBone"
            | "oFlame"
            | "oLava"
            | "oPoof"
            | "oSmokePuff"
            | "oBloodSpark"
            | "oBloodTrail"
            | "oBubble"
            | "oSplash"
            | "oBurn"
            | "oFlareSpark"
            | "oLaserExplode"
            | "oFlameTrail"
            | "oYellowTrail"
            | "oLaserTrail"
            | "oBones"
            | "oFakeBones"
            | "oParaUsed"
            | "oParachute"
            | "oSmallCollect"
            | "oBigCollect"
            | "oHeart"
            | "oItemsGet"
            | "oRubbleDark"
    ) || catalog::is_a(object, "oRubblePiece")
}

/// Fixed Create defaults run synchronously before producer overrides.
pub(crate) fn init_entity(e: &mut Entity) {
    if e.value("world_create_initialized") != 0.0 {
        return;
    }
    e.data.insert("world_create_initialized".into(), 1.0);
    let speed = match e.object.as_str() {
        "oPoof" | "oSmokePuff" | "oBurn" | "oFlareSpark" | "oFlameTrail" | "oSmallCollect"
        | "oLavaDrip" => Some(0.4),
        "oBloodSpark" | "oLamp" | "oLampRed" | "oTikiTorch" | "oHintHand" => Some(0.5),
        "oBloodTrail" | "oLaserExplode" | "oLaserTrail" | "oItemsGet" => Some(0.8),
        "oBubble" | "oLeaf" | "oPsychicCreateP" => Some(0.2),
        "oSplash" => Some(0.6),
        "oBlood" | "oBone" | "oFlame" => Some(0.3),
        "oLava" | "oPsychicCreate2" => Some(0.4),
        _ => None,
    };
    if let Some(speed) = speed {
        e.set_image_speed(speed);
    }
    if catalog::is_a(&e.object, "oDetritus") {
        e.data.insert("bounce".into(), 1.0);
    }
    match e.object.as_str() {
        "oGrave" => {
            // Grave inherits DrawnSprite then Solid Create before its own roll.
            e.data.insert("blinkToggle".into(), 0.0);
            e.data.insert("invincible".into(), 0.0);
            e.data.insert("shopWall".into(), 0.0);
            e.data.insert("cleanDeath".into(), 0.0);
            e.text.insert("type".into(), "None".into());
            e.text.insert("treasure".into(), String::new());
        }
        "oSmallCollect" | "oBigCollect" | "oHeart" => e.alarms[0] = 30,
        "oItemsGet" => {
            e.alarms[0] = 40;
            e.vel.y = 0.1;
        }
        "oYellHelp" => e.alarms[0] = 40,
        "oBlood" => {
            e.alarms[0] = 1;
            e.alarms[1] = 1;
            e.alarms[2] = 5;
            e.data.insert("bounce".into(), 0.0);
            e.data.insert("invincible".into(), 1.0);
            e.data.insert("collectible".into(), 0.0);
        }
        "oFlame" => {
            e.alarms[0] = 2;
            e.alarms[1] = 50;
        }
        "oSmokePuff" => e.vel.y = 0.1,
        "oBurn" | "oFlareSpark" => e.vel.y = -0.1,
        "oLeaf" => {
            e.vel.y = 0.4;
            e.data.insert("gravity".into(), 0.01);
        }
        _ => {}
    }
    if catalog::is_a(&e.object, "oRubblePiece") {
        e.data.entry("gravity".into()).or_insert(0.6);
        e.data.insert("world_effect_initialized".into(), 1.0);
    }
    if matches!(e.object.as_str(), "oLamp" | "oLampRed" | "oTikiTorch") {
        light(e);
    }
}

/// World/RNG-only Create bodies shared by generated and runtime births.
pub(crate) fn create_local(
    context: &mut crate::creation::EntityCreateContext<'_>,
    entity: &mut Entity,
) {
    if entity.object == "oGrave" && entity.value("grave_create_initialized") == 0.0 {
        // oGrave Create: rand(1,5), then its literal sGrave2..sGrave6 switch.
        // Cemetery's possible sGraveAsh assignment follows this callback.
        entity.set_sprite(&format!("sGrave{}", context.rng.range(5) + 2));
        entity.data.insert("grave_create_initialized".into(), 1.0);
    }
}

/// Source Create for callers with the game RNG available.
pub(crate) fn create_instance(game: &mut Game, id: u64) {
    let Some(index) = game.world.entities.iter().position(|e| e.id == id) else {
        return;
    };
    let mut e = game.world.entities[index].clone();
    create_local(
        &mut crate::creation::EntityCreateContext {
            world: &game.world,
            rng: &mut game.rng,
        },
        &mut e,
    );
    if e.object == "oTemple" {
        temple::create(game, id);
        return;
    }
    if e.object == "oLeaves" {
        let cemetery = game.flags.contains("cemetery") || game.flags.contains("cemetary");
        initialize_leaves(&mut game.world, id, cemetery);
        return;
    }
    if matches!(e.object.as_str(), "oPsychicCreateP" | "oPsychicCreate2") {
        initialize_psychic_create(game, &mut e);
    } else if catalog::is_a(&e.object, "oDetritus") {
        initialize_detritus(game, &mut e);
    } else if e.object == "oLava" {
        initialize_lava(game, &mut e);
    } else if handles(&e.object) {
        initialize_effect(game, &mut e);
    }
    game.world.entities[index] = e;
}

/// Create callbacks requiring the game's RNG or event queue.
pub(crate) fn spawn_effect(game: &mut Game, object: &str, pos: Vec2) -> u64 {
    let id = game.world.spawn(object, pos);
    let mut e = game.world.entity(id).unwrap().clone();
    initialize_effect(game, &mut e);
    let index = game.world.entities.iter().position(|e| e.id == id).unwrap();
    game.world.entities[index] = e;
    id
}

fn initialize_effect(game: &mut Game, e: &mut Entity) {
    if e.value("world_effect_initialized") != 0.0 {
        return;
    }
    match e.object.as_str() {
        "oBloodSpark" => e.vel.y = -f64::from(1 + game.rng.range(3)) * 0.2,
        "oBubble" => e.vel.y = -f64::from(1 + game.rng.range(3)) * 0.1,
        "oLaserExplode" => game.sound("sndSmallExplode", e.pos),
        "oIceBottom" => {
            e.alarms[0] = (20 + game.rng.range(381)) as i32;
        }
        _ => {}
    }
    e.data.insert("world_effect_initialized".into(), 1.0);
}

pub(crate) fn step_instance(game: &mut Game, id: u64) {
    let Some(index) = game
        .world
        .entities
        .iter()
        .position(|e| e.id == id && e.alive)
    else {
        return;
    };
    let mut e = game.world.entities[index].clone();
    let Some(object) = source_object(game, id) else {
        return;
    };
    match object.as_str() {
        "oTempleFake" => temple::fake_step(game, &mut e),
        "oGrave" => {
            // oGrave owns Step: no inherited Solid Step or clean-death gate.
            // Its source query uses the origin, not the sprite center.
            if e.pos.x > game.camera.x - 20.0
                && e.pos.x < game.camera.x + VIEW_WIDTH + 4.0
                && e.pos.y > game.camera.y - 20.0
                && e.pos.y < game.camera.y + VIEW_HEIGHT + 4.0
                && !point_family(game, Vec2::new(e.pos.x, e.pos.y + 16.0), "oSolid")
            {
                crate::items::explosives::destroy_solid_self(game, id);
            }
            // Destroy can append rubble and retire this receiver. Do not write
            // the pre-callback clone back over the marked/dead instance.
            return;
        }
        "oTree" => {
            let terrain = e.object == "oTerrainMetadata";
            tree(game, &mut e, terrain);
        }
        "oLeaves" | "oTreeBranch" => {
            if object == "oLeaves" && e.value("leaves_initialized") == 0.0 {
                let cemetery = game.flags.contains("cemetery") || game.flags.contains("cemetary");
                initialize_leaves(&mut game.world, id, cemetery);
                e = game.world.entities[index].clone();
            }
            canopy(game, &mut e);
        }
        "oPsychicCreateP" | "oPsychicCreate2" => psychic_create(game, &mut e),
        "oIceBottom" => ice_bottom(game, &mut e),
        "oFrozenCaveman" => frozen_caveman(game, &mut e),
        "oBlood" | "oBone" => detritus(game, &mut e),
        "oFlame" => flame(game, &mut e),
        "oLava" => {
            let terrain = e.object == "oTerrainMetadata";
            lava(game, &mut e, terrain);
        }
        "oPoof" | "oSmokePuff" | "oBloodSpark" | "oBloodTrail" | "oBubble" | "oSplash"
        | "oBurn" | "oFlareSpark" | "oLaserExplode" | "oFlameTrail" | "oYellowTrail"
        | "oLaserTrail" => effect(game, &mut e),
        "oBones" | "oFakeBones" | "oParaUsed" => fallen_decoration(game, &mut e),
        "oParachute" => e.pos = Vec2::new(game.player.pos.x - 8.0, game.player.pos.y - 16.0),
        "oYellHelp" => e.pos.y -= 0.1,
        "oSmallCollect" | "oBigCollect" | "oHeart" | "oItemsGet" => collect_effect(&mut e),
        "oRubbleDark" => {
            e.pos.y += e.vel.y;
            e.vel.y += 0.6;
            if point_family(game, e.pos, "oBrick")
                || point_family(game, e.pos, "oBlock")
                || outside_room(game, &e)
            {
                e.alive = false;
            }
        }
        _ if catalog::is_a(&object, "oRubblePiece") => rubble(game, &mut e),
        _ => {}
    }
    game.world.entities[index] = e;
}

pub(crate) fn builtin_motion(_game: &mut Game, _id: u64) {}

pub(crate) fn alarm(game: &mut Game, id: u64, index: u8) {
    let Some(mut e) = game.world.entity(id).cloned() else {
        return;
    };
    match (e.object.as_str(), index) {
        ("oSmallCollect" | "oBigCollect" | "oHeart" | "oItemsGet" | "oYellHelp", 0) => {
            e.alive = false
        }
        ("oBlood", 1) => {
            e.data.insert("invincible".into(), 0.0);
            e.data.insert("bounce".into(), 1.0);
        }
        ("oBlood", 2) => {
            e.data.insert("collectible".into(), 1.0);
        }
        ("oBlood" | "oFlame", 0) => {
            let blood = e.object == "oBlood";
            if !blood || !game.flags.contains("low_graphics") {
                let trail = if blood { "oBloodTrail" } else { "oFlameTrail" };
                if game
                    .world
                    .entities
                    .iter()
                    .filter(|e| e.alive && e.object == trail)
                    .count()
                    < 12
                {
                    spawn_effect(game, trail, e.pos);
                }
                e.alarms[0] = if blood { 4 } else { 2 };
            }
        }
        ("oIceBottom", 0) => {
            game.world
                .spawn("oDrip", Vec2::new(e.pos.x + 8.0, e.pos.y + 4.0));
            e.alarms[0] = (20 + game.rng.range(381)) as i32;
        }
        _ => {}
    }
    if let Some(index) = game.world.entities.iter().position(|other| other.id == id) {
        game.world.entities[index] = e;
    }
}

pub(crate) fn animation_end(game: &mut Game, id: u64) {
    let Some(mut e) = game.world.entity(id).cloned() else {
        return;
    };
    match e.object.as_str() {
        "oPoof" | "oSmokePuff" | "oBloodSpark" | "oBloodTrail" | "oBubble" | "oSplash"
        | "oBurn" | "oFlareSpark" | "oLaserExplode" | "oFlameTrail" | "oYellowTrail"
        | "oLaserTrail" | "oLavaDrip" | "oPsychicCreateP" | "oPsychicCreate2" => e.alive = false,
        "oBone" if e.value("dying") != 0.0 => e.alive = false,
        "oParachute" if e.sprite == "sParaOpen" => e.set_sprite("sParachute"),
        "oFakeBones" if e.sprite == "sSkeletonCreateL" => {
            game.world.spawn("oSkeleton", e.pos);
            e.alive = false;
        }
        _ => {}
    }
    if let Some(index) = game.world.entities.iter().position(|other| other.id == id) {
        game.world.entities[index] = e;
    }
}

pub(crate) fn collision(game: &mut Game, id: u64, other: crate::dispatcher::Instance) {
    let Some(mut e) = game.world.entity(id).cloned() else {
        return;
    };
    let object = other.object(game).to_owned();
    match e.object.as_str() {
        "oPsychicCreateP" if catalog::is_a(&object, "oSolid") => e.alive = false,
        "oFlame" if catalog::is_a(&object, "oWater") => {
            spawn_effect(game, "oSmokePuff", e.pos);
            spawn_effect(game, "oSmokePuff", e.pos);
            e.alive = false;
        }
        "oMsgSign" if catalog::is_a(&object, "oCharacter") => sign(game, &mut e),
        "oParachute" if catalog::is_a(&object, "oItem") => {
            if let crate::dispatcher::Instance::Entity(other_id) = other {
                if game
                    .world
                    .entity(other_id)
                    .is_some_and(|other| other.vel.x > 0.0 || other.vel.y > 0.0)
                {
                    game.world.spawn("oParaUsed", e.pos);
                    e.alive = false;
                    game.player.data.insert("parachute_open".into(), 0.0);
                }
            }
        }
        _ => {}
    }
    if let Some(index) = game.world.entities.iter().position(|other| other.id == id) {
        game.world.entities[index] = e;
    }
}

/// Legacy focused harness; production uses the source phase dispatcher.
pub fn tick(game: &mut Game, _input: &InputFrame) {
    prepare_tick(game);
    sync_parachute(game);
    let count = game.world.entities.len();
    canopy_tick(game, count);
    let ids: Vec<_> = game
        .world
        .entities
        .iter()
        .take(count)
        .filter(|e| {
            !matches!(
                source_object(game, e.id).as_deref(),
                Some("oTree" | "oLeaves" | "oTreeBranch")
            )
        })
        .map(|e| e.id)
        .collect();
    for id in ids {
        step_instance(game, id);
    }
}

fn canopy_tick(game: &mut Game, count: usize) {
    // Classic's Step dispatcher iterates object resources first, then each
    // object's instances in creation order: Tree 585, Leaves 586, Branch 587.
    // Stored Tree metadata occupies that instance's producer creation slot.
    for object in ["oTree", "oLeaves", "oTreeBranch"] {
        for index in 0..count {
            let e = &game.world.entities[index];
            let stored_tree = e.object == "oTerrainMetadata" && e.value("tree_initialized") != 0.0;
            if !e.alive || (e.object != object && !(object == "oTree" && stored_tree)) {
                continue;
            }
            if object == "oLeaves" && e.value("leaves_initialized") == 0.0 {
                let id = e.id;
                let cemetery = game.flags.contains("cemetery") || game.flags.contains("cemetary");
                initialize_leaves(&mut game.world, id, cemetery);
            }
            let mut e = game.world.entities[index].clone();
            if object == "oTree" {
                tree(game, &mut e, stored_tree);
            } else {
                canopy(game, &mut e);
            }
            game.world.entities[index] = e;
        }
    }
}

/// Keep a stored Tree's Step at its source creation position among the room's
/// instances. Reusing metadata preserves its stable id and cell-local fields.
pub(crate) fn initialize_tree(world: &mut crate::World, x: usize, y: usize) -> u64 {
    let pos = Vec2::new(x as f64 * 16.0 + 8.0, y as f64 * 16.0 + 8.0);
    if let Some(index) = world.entities.iter().position(|e| {
        e.alive
            && e.object == "oTerrainMetadata"
            && (e.pos.x / 16.0).floor() as usize == x
            && (e.pos.y / 16.0).floor() as usize == y
    }) {
        if world.entities[index].value("tree_initialized") != 0.0 {
            return world.entities[index].id;
        }
        let mut metadata = world.entities.remove(index);
        metadata.pos = pos;
        metadata.previous_pos = pos;
        metadata.data.insert("tree_initialized".into(), 1.0);
        let id = metadata.id;
        world.entities.push(metadata);
        return id;
    }
    let id = world.spawn("oTerrainMetadata", pos);
    let metadata = world.entity_mut(id).unwrap();
    metadata.sprite.clear();
    metadata.data.insert("tree_initialized".into(), 1.0);
    id
}

fn tree(game: &mut Game, e: &mut Entity, metadata: bool) {
    let pos = if metadata {
        Vec2::new(e.pos.x - 8.0, e.pos.y - 8.0)
    } else {
        e.pos
    };
    if pos.x <= game.camera.x - 16.0
        || pos.x >= game.camera.x + VIEW_WIDTH + 16.0
        || pos.y <= game.camera.y - 16.0
        || pos.y >= game.camera.y + VIEW_HEIGHT + 16.0
        || point_family(game, Vec2::new(pos.x, pos.y + 16.0), "oSolid")
    {
        return;
    }
    let destroyed = if metadata {
        crate::items::explosives::destroy_unsupported_tree_tile(
            game,
            (pos.x / 16.0).floor() as usize,
            (pos.y / 16.0).floor() as usize,
        )
    } else {
        crate::items::explosives::destroy_unsupported_tree_instance(game, e.id)
    };
    if destroyed {
        // Preserve the canonical callback's bookkeeping when the dispatcher
        // writes its local clone back to storage.
        if let Some(updated) = game.world.entities.iter().find(|other| other.id == e.id) {
            *e = updated.clone();
        }
        e.alive = false;
    }
}

/// Source Leaves Create runs synchronously in the tree generator. The runtime
/// fallback also supports instances authored outside that producer.
pub(crate) fn initialize_leaves(world: &mut crate::World, id: u64, cemetery: bool) {
    let Some(e) = world.entity(id) else { return };
    if e.value("leaves_initialized") != 0.0 {
        return;
    }
    let left = Vec2::new(e.pos.x - 16.0, e.pos.y);
    let right_sprite =
        world_point_family(world, left, "oTree") || world_point_family(world, left, "oLeaves");
    let sprite = match (cemetery, right_sprite) {
        (false, false) => "sLeaves",
        (false, true) => "sLeavesRight",
        (true, false) => "sLeavesDead",
        (true, true) => "sLeavesDeadR",
    };
    let e = world.entity_mut(id).unwrap();
    e.sprite = sprite.into();
    e.data.insert("sprite_set".into(), 0.0);
    e.data.insert("leaves_initialized".into(), 1.0);
}

fn canopy(game: &mut Game, e: &mut Entity) {
    let branch = e.object == "oTreeBranch";
    let support = |x| {
        let point = Vec2::new(x, e.pos.y);
        point_family(game, point, "oTree") || (!branch && point_family(game, point, "oLeaves"))
    };
    let left = support(e.pos.x - 16.0);
    let right = support(e.pos.x + 16.0);
    // The source never changes spriteSet after Create.
    if !branch && e.value("sprite_set") == 0.0 && left && right {
        e.set_sprite("sLeavesTop");
    }
    let inside_view = e.pos.x > game.camera.x - 16.0
        && e.pos.x < game.camera.x + VIEW_WIDTH + 16.0
        && e.pos.y > game.camera.y - 16.0
        && e.pos.y < game.camera.y + VIEW_HEIGHT + 16.0;
    if !inside_view {
        return;
    }
    let unsupported = if branch {
        !left && !right
    } else {
        match e.sprite.as_str() {
            "sLeavesTop" => !left || !right,
            "sLeaves" | "sLeavesDead" => !right,
            "sLeavesRight" | "sLeavesDeadR" => !left,
            _ => false,
        }
    };
    if !unsupported {
        return;
    }
    e.alive = false;
    if matches!(
        e.sprite.as_str(),
        "sLeavesDead" | "sLeavesDeadR" | "sTreeBranchDeadL" | "sTreeBranchDeadR"
    ) {
        return;
    }
    for _ in 0..if branch { 1 } else { 2 } {
        let x = e.pos.x + 8.0 + f64::from(game.rng.range(9)) - f64::from(game.rng.range(9));
        let y = e.pos.y + 8.0 + f64::from(game.rng.range(9)) - f64::from(game.rng.range(9));
        game.world.spawn("oLeaf", Vec2::new(x, y));
    }
}

pub(crate) fn spawn_psychic_create_p(game: &mut Game, pos: Vec2, velocity: Vec2) -> u64 {
    let id = game.world.spawn("oPsychicCreateP", pos);
    let e = game.world.entity_mut(id).unwrap();
    e.vel = velocity;
    e.data.insert("psychic_initialized".into(), 1.0);
    id
}

pub(crate) fn spawn_psychic_create_2(game: &mut Game, pos: Vec2, target: Vec2) -> u64 {
    let id = game.world.spawn("oPsychicCreate2", pos);
    let e = game.world.entity_mut(id).unwrap();
    let angle = (target.y - pos.y).atan2(target.x - pos.x);
    e.vel = Vec2::new(2.0 * angle.cos(), 2.0 * angle.sin());
    e.data.insert("psychic_initialized".into(), 1.0);
    id
}

fn initialize_psychic_create(game: &Game, e: &mut Entity) {
    if e.value("psychic_initialized") != 0.0 {
        return;
    }
    e.vel = Vec2::default();
    if e.object == "oPsychicCreate2" {
        if let Some(olmec) = game
            .world
            .entities
            .iter()
            .find(|o| o.alive && o.object == "oOlmec")
        {
            let dx = olmec.pos.x + 32.0 - e.pos.x;
            let dy = olmec.pos.y + 16.0 - e.pos.y;
            let angle = dy.atan2(dx);
            e.vel = Vec2::new(2.0 * angle.cos(), 2.0 * angle.sin());
        }
    }
    e.data.insert("psychic_initialized".into(), 1.0);
}

fn psychic_create(game: &Game, e: &mut Entity) {
    initialize_psychic_create(game, e);
    e.pos.x += e.vel.x;
    e.pos.y += e.vel.y;
    let player_effect = e.object == "oPsychicCreateP";
    if player_effect && e.vel.y < 6.0 {
        e.vel.y += 0.6;
    }
}

fn sign(game: &mut Game, e: &mut Entity) {
    let remaining = (e.value("message_cooldown_until") - game.tick as f64).max(0.0);
    let mask = e.sprite_bounds();
    if !mask.intersects(game.player.sprite_bounds())
        && !game.world.entities.iter().any(|character| {
            character.alive
                && catalog::is_a(&character.object, "oCharacter")
                && mask.intersects(character.sprite_bounds())
        })
    {
        return;
    }
    // The source collision refreshes the global display lifetime every tick.
    game.data.insert("message_timer".into(), 200.0);
    let changed = game.data.get("message_sign_id").copied() != Some(e.id as f64);
    game.data.insert("message_sign_id".into(), e.id as f64);
    if remaining != 0.0 && !changed {
        return;
    }
    let first = e.text.get("message").map(String::as_str).unwrap_or("");
    let second = e.text.get("message2").map(String::as_str).unwrap_or("");
    let message = [first, second]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    if !message.is_empty() || changed {
        game.events.push(GameEvent::Message(message));
        // Events notify the presentation once, rather than duplicating the same
        // notification on every overlapping tick. Display lifetime still refreshes.
        e.data
            .insert("message_cooldown_until".into(), game.tick as f64 + 200.0);
    }
}

fn light(e: &mut Entity) {
    let torch = e.object == "oTikiTorch";
    e.data
        .insert("light_radius".into(), if torch { 32.0 } else { 96.0 });
    e.data.insert("light_offset_x".into(), 8.0);
    e.data.insert("light_offset_y".into(), 8.0);
    if e.object == "oLampRed" {
        e.data.insert("red_light_range".into(), 96.0);
    }
}

fn initialize_lava(game: &mut Game, e: &mut Entity) {
    if e.value("lava_initialized") == 0.0 {
        let time = e
            .data
            .get("spurt_time")
            .copied()
            .unwrap_or_else(|| f64::from(100 + game.rng.range(201)));
        e.data.entry("spurt_time".into()).or_insert(time);
        e.data.entry("spurt_counter".into()).or_insert(time);
        e.data.entry("spurt".into()).or_insert(0.0);
        e.data.insert("lava_initialized".into(), 1.0);
    }
}

fn lava(game: &mut Game, e: &mut Entity, terrain: bool) {
    initialize_lava(game, e);
    let origin = if terrain {
        Vec2::new(e.pos.x - 8.0, e.pos.y - 8.0)
    } else {
        e.pos
    };
    let dist = if game.flags.contains("hub") {
        0.0
    } else {
        (origin.x - game.player.pos.x).hypot(origin.y - game.player.pos.y)
    };
    if e.value("spurt") == 0.0 || dist >= 240.0 {
        return;
    }
    if e.value("spurt_counter") > 0.0 {
        let counter = e.value("spurt_counter") - 1.0;
        e.data.insert("spurt_counter".into(), counter);
        return;
    }
    e.data.insert("spurt_counter".into(), e.value("spurt_time"));
    let object = if game.rng.chance(1, 8) {
        "oMagma"
    } else {
        "oFlame"
    };
    let id = game
        .world
        .spawn(object, Vec2::new(origin.x + 8.0, origin.y - 4.0));
    let mut particle = game.world.entity(id).unwrap().clone();
    // Source Create runs before the caller overrides yVel. Initialize here so
    // the first particle tick cannot replace the lava's deliberate launch.
    initialize_detritus(game, &mut particle);
    if object == "oMagma" {
        crate::enemies::init_entity(&mut particle);
        particle.data.insert("started".into(), 1.0);
        for (key, value) in [
            ("collision_left", -8.0),
            ("collision_top", -8.0),
            ("collision_right", 8.0),
            ("collision_bottom", 8.0),
        ] {
            particle.data.insert(key.into(), value);
        }
    }
    particle.vel.y = -f64::from(1 + game.rng.range(4));
    if !particle.alive && object == "oFlame" {
        game.world.spawn("oSmokePuff", particle.pos);
    }
    let index = game
        .world
        .entities
        .iter()
        .position(|entity| entity.id == id)
        .unwrap();
    game.world.entities[index] = particle;
}

fn point_family(game: &Game, point: Vec2, family: &str) -> bool {
    world_point_family(&game.world, point, family)
}

fn world_point_family(world: &crate::World, point: Vec2, family: &str) -> bool {
    world.tiles_point(point, family)
        || world.entities.iter().any(|e| {
            e.alive && catalog::is_a(&e.object, family) && e.sprite_bounds().contains(point)
        })
}

fn ice_bottom(game: &mut Game, e: &mut Entity) {
    if !point_family(game, Vec2::new(e.pos.x + 8.0, e.pos.y - 1.0), "oSolid") {
        e.alive = false;
        return;
    }
}

fn frozen_caveman(game: &mut Game, e: &mut Entity) {
    if point_family(game, e.pos, "oIce") {
        return;
    }
    e.alive = false;
    // Generated ice stores the frozen occupant on terrain metadata, whose
    // destruction adapter already owns its release. Explicit instances (e.g.
    // imported rooms) need the source Destroy event here instead.
    let adapted = game.world.entities.iter().any(|other| {
        other.value("frozen_caveman") != 0.0
            && (other.pos.x / 16.0).floor() == (e.pos.x / 16.0).floor()
            && (other.pos.y / 16.0).floor() == (e.pos.y / 16.0).floor()
    });
    if adapted
        || game.flags.contains("cleanSolids")
        || game.data.get("cleanSolids").copied().unwrap_or(0.0) != 0.0
    {
        return;
    }
    let id = game.world.spawn("oCaveman", e.pos);
    let caveman = game.world.entity_mut(id).unwrap();
    caveman.state = 98;
    caveman.timer = caveman.value("stun_time") as u32;
    caveman.data.insert("invincible".into(), 20.0);
}

fn effect(game: &mut Game, e: &mut Entity) {
    initialize_effect(game, e);
    match e.object.as_str() {
        "oPoof" => {
            e.pos.x += e.vel.x;
            e.pos.y += e.vel.y;
        }
        "oSmokePuff" => e.pos.y -= e.vel.y,
        "oBloodSpark" | "oBubble" | "oBurn" | "oFlareSpark" => e.pos.y += e.vel.y,
        _ => {}
    }
    if (e.object == "oBubble" && !point_family(game, e.pos, "oWater"))
        || (matches!(e.object.as_str(), "oBurn" | "oFlareSpark")
            && point_family(game, e.pos, "oSolid"))
    {
        e.alive = false;
    }
}

fn collect_effect(e: &mut Entity) {
    let items = e.object == "oItemsGet";
    if items {
        e.pos.y = (e.pos.y - 0.1).ceil();
        e.pos.x = e.pos.x.ceil();
    } else {
        e.pos.y -= 1.0;
        if e.object == "oSmallCollect" {
            e.pos.x = e.pos.x.ceil();
            e.pos.y = e.pos.y.ceil();
        }
    }
}

fn flame(game: &mut Game, e: &mut Entity) {
    detritus(game, e);
    if e.alive {
        if e.vel.y > 6.0 {
            e.alive = false;
        }
        if game
            .world
            .collides_except(Rect::new(e.pos.x - 4.0, e.pos.y + 4.0, 8.0, 1.0), e.id)
            && e.value("life") > 20.0
        {
            e.data.insert("life".into(), 20.0);
        }
    }
    if !e.alive {
        spawn_effect(game, "oSmokePuff", e.pos);
    }
}

fn outside(game: &Game, p: Vec2, margin: f64) -> bool {
    p.x < game.camera.x - margin
        || p.x > game.camera.x + VIEW_WIDTH + margin
        || p.y < game.camera.y - margin
        || p.y > game.camera.y + VIEW_HEIGHT + margin
}

fn outside_room(game: &Game, e: &Entity) -> bool {
    let mask = e.sprite_bounds();
    mask.is_valid()
        && (mask.x + mask.w <= 0.0
            || mask.y + mask.h <= 0.0
            || mask.x >= game.world.width as f64 * 16.0
            || mask.y >= game.world.height as f64 * 16.0)
}

fn rubble(game: &Game, e: &mut Entity) {
    if e.value("world_effect_initialized") == 0.0 {
        if e.object == "oLeaf" {
            e.vel.y = 0.4;
        }
        e.data.insert("world_effect_initialized".into(), 1.0);
    }
    e.pos.x += e.vel.x;
    e.pos.y += e.vel.y;
    e.vel.y += e.data.get("gravity").copied().unwrap_or(0.6);
    if point_family(game, e.pos, "oWaterSwim") {
        if e.object == "oDrip" {
            e.alive = false;
        } else if e.object == "oLeaf" {
            e.vel.y = 0.0;
            e.set_sprite("sLeafStill");
        } else {
            e.vel.y = 0.2;
        }
    } else if point_family(game, e.pos, "oLava") {
        e.alive = false;
    }
    if point_family(game, e.pos, "oSolid")
        || outside(game, e.pos, 32.0)
        || (e.object == "oRubbleDarkSmall" && outside_room(game, e))
    {
        e.alive = false;
    }
}

// moveTo uses reciprocal/modulo displacement, rather than accumulating fractions.
fn displacement(v: f64, tick: u64) -> i32 {
    let fraction = v.abs().fract();
    let extra =
        fraction != 0.0 && tick.is_multiple_of((1.0 / fraction).round_ties_even().max(1.0) as u64);
    (v.abs().floor() as i32 + i32::from(extra)) * if v < 0.0 { -1 } else { 1 }
}

/// Execute inherited/child Create before the producer's following RNG draws
/// and velocity overrides. Capped-instance continuation remains unchanged.
pub(crate) fn spawn_detritus(game: &mut Game, object: &str, pos: Vec2) -> u64 {
    let id = game.world.spawn(object, pos);
    let mut e = game.world.entity(id).unwrap().clone();
    initialize_detritus(game, &mut e);
    let index = game.world.entities.iter().position(|e| e.id == id).unwrap();
    game.world.entities[index] = e;
    id
}

/// scrCreateFlame checks the shared detritus cap before each synchronous Create.
/// The graphics setting belongs to each caller, rather than this script.
pub(crate) fn create_flames(game: &mut Game, pos: Vec2, count: usize) {
    for _ in 0..count {
        if game
            .world
            .entities
            .iter()
            .filter(|e| e.alive && e.engine_active && catalog::is_a(&e.object, "oDetritus"))
            .count()
            < 16
        {
            game.spawn("oFlame", pos);
        }
    }
}

pub(crate) fn initialize_detritus(game: &mut Game, e: &mut Entity) {
    if e.value("world_effect_initialized") == 0.0 {
        e.vel = Vec2::new(
            game.rng.unit() * 4.0 - game.rng.unit() * 4.0,
            -1.0 - game.rng.unit() * 2.0,
        );
        e.data
            .entry("gravity".into())
            .or_insert_with(|| f64::from(1 + game.rng.range(6)) * 0.1);
        e.data.insert("life".into(), 60.0);
        e.data.insert("world_effect_initialized".into(), 1.0);
        let extent = if e.object == "oMagma" { 8.0 } else { 4.0 };
        for (key, value) in [
            ("collision_left", -extent),
            ("collision_top", -extent),
            ("collision_right", extent),
            ("collision_bottom", extent),
        ] {
            e.data.insert(key.into(), value);
        }
        if game
            .world
            .entities
            .iter()
            .take_while(|other| other.id != e.id)
            .filter(|other| {
                other.alive && other.engine_active && catalog::is_a(&other.object, "oDetritus")
            })
            .count()
            >= 32
        {
            e.alive = false;
            return;
        }
    }
}

pub(crate) fn detritus(game: &mut Game, e: &mut Entity) {
    initialize_detritus(game, e);
    if !e.alive {
        return;
    }
    let extent = if e.object == "oMagma" { 8.0 } else { 4.0 };
    let life = e.value("life");
    e.data.insert("life".into(), life - 1.0);
    if life <= 0.0 || outside(game, e.pos, 4.0) {
        e.alive = false;
        return;
    }
    let dx = displacement(e.vel.x, game.engine_time());
    let dy = displacement(e.vel.y, game.engine_time());
    for _ in 0..dx.unsigned_abs() {
        let next = Vec2::new(e.pos.x + f64::from(dx.signum()), e.pos.y);
        if game.world.collides_except(
            Rect::new(
                e.pos.x + if dx > 0 { extent } else { -extent - 1.0 },
                e.pos.y - extent + 5.0,
                1.0,
                extent * 2.0 - 5.0,
            ),
            e.id,
        ) {
            break;
        }
        e.pos = next;
    }
    for _ in 0..dy.unsigned_abs() {
        let next = Vec2::new(e.pos.x, e.pos.y + f64::from(dy.signum()));
        if game.world.collides_except(
            Rect::new(
                e.pos.x - extent,
                e.pos.y + if dy > 0 { extent } else { -extent - 1.0 },
                extent * 2.0,
                1.0,
            ),
            e.id,
        ) {
            break;
        }
        e.pos = next;
    }
    if point_family(game, Vec2::new(e.pos.x, e.pos.y - 4.0), "oLava") {
        e.alive = false;
        return;
    }
    let bottom = game.world.collides_except(
        Rect::new(e.pos.x - extent, e.pos.y + extent, extent * 2.0, 1.0),
        e.id,
    );
    if e.value("bounce") != 0.0 {
        if e.vel.y < 6.0 {
            e.vel.y += e.value("gravity");
        }
        if e.vel.y < 0.0
            && game.world.collides_except(
                Rect::new(e.pos.x - extent, e.pos.y - extent - 1.0, extent * 2.0, 1.0),
                e.id,
            )
        {
            e.vel.y *= -0.8;
        }
        if game.world.collides_except(
            Rect::new(e.pos.x - extent - 1.0, e.pos.y - extent, 1.0, extent * 2.0),
            e.id,
        ) || game.world.collides_except(
            Rect::new(e.pos.x + extent, e.pos.y - extent, 1.0, extent * 2.0),
            e.id,
        ) {
            e.vel.x *= -0.5;
        }
        if bottom {
            e.vel.y = if e.vel.y > 1.0 { e.vel.y * -0.5 } else { 0.0 };
        }
    }
    if e.object == "oBlood" {
        if e.vel.y > 6.0 {
            e.alive = false;
        }
        if bottom && life > 20.0 {
            e.data.insert("life".into(), 20.0);
        }
    } else if e.object == "oBone" && bottom {
        e.set_sprite("sSmokePuff");
        e.data.insert("dying".into(), 1.0);
    }
}

fn fallen_decoration(game: &mut Game, e: &mut Entity) {
    if e.object == "oFakeBones"
        && !(e.pos.x > game.camera.x - 16.0
            && e.pos.x < game.camera.x + VIEW_WIDTH
            && e.pos.y > game.camera.y - 16.0
            && e.pos.y < game.camera.y + VIEW_HEIGHT)
    {
        return;
    }
    if !point_family(game, Vec2::new(e.pos.x + 8.0, e.pos.y + 16.0), "oSolid") {
        e.pos.y += e.vel.y;
        e.vel.y += 0.2;
    }
    if point_family(game, Vec2::new(e.pos.x + 8.0, e.pos.y + 15.0), "oSolid") {
        e.pos.y -= 1.0;
    }
    if e.object == "oFakeBones" {
        if (game.player.pos.y - (e.pos.y + 8.0)).abs() < 8.0
            && (game.player.pos.x - (e.pos.x + 8.0)).abs() < 64.0
        {
            e.set_sprite("sSkeletonCreateL");
        }
    }
}

pub(crate) fn sync_parachute(game: &mut Game) {
    let open = game
        .player
        .data
        .get("parachute_open")
        .copied()
        .unwrap_or(0.0)
        != 0.0
        && game.player.health > 0
        && game.player.state != crate::PlayerState::Dead
        && !game.player.is_stunned();
    let existing: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oParachute")
        .map(|e| e.id)
        .collect();
    if open && existing.is_empty() {
        game.world.spawn(
            "oParachute",
            Vec2::new(game.player.pos.x - 8.0, game.player.pos.y - 16.0),
        );
    }
    if !open {
        game.player.data.insert("parachute_open".into(), 0.0);
        for id in existing {
            if let Some(e) = game.world.entity_mut(id) {
                e.alive = false;
            }
            game.world.spawn(
                "oParaUsed",
                Vec2::new(game.player.pos.x - 8.0, game.player.pos.y - 24.0),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Area, RulesPreset, World};
    fn room() -> Game {
        let mut g = Game::shell(1, RulesPreset::Classic);
        g.world = World::new(20, 15, Area::Mines, 1);
        g.player.pos = Vec2::new(40.0, 40.0);
        g
    }
    fn advance(g: &mut Game) {
        g.tick += 1;
        g.level_tick += 1;
        for e in &mut g.world.entities {
            e.age += 1;
        }
        g.events.clear();
        begin_step(g);
        prepare_tick(g);
        let ids: Vec<_> = g
            .world
            .entities
            .iter()
            .filter(|e| e.alive)
            .map(|e| e.id)
            .collect();
        for number in 0..12 {
            for &id in &ids {
                let fire = if let Some(e) = g.world.entity_mut(id) {
                    if e.alarms[number] > 0 {
                        e.alarms[number] -= 1;
                        e.alarms[number] == 0
                    } else {
                        false
                    }
                } else {
                    false
                };
                if fire {
                    alarm(g, id, number as u8);
                }
            }
        }
        tick(g, &InputFrame::default());
        let ids: Vec<_> = g
            .world
            .entities
            .iter()
            .filter(|e| e.alive)
            .map(|e| e.id)
            .collect();
        for id in ids {
            let Some(e) = g.world.entity(id).cloned() else {
                continue;
            };
            if e.sprite_bounds().intersects(g.player.sprite_bounds()) {
                collision(g, id, crate::dispatcher::Instance::Player);
            }
            let others: Vec<_> = g
                .world
                .entities
                .iter()
                .filter(|o| o.alive && o.id != id && e.event_intersects(o))
                .map(|o| o.id)
                .collect();
            for other in others {
                collision(g, id, crate::dispatcher::Instance::Entity(other));
            }
            let tiles: Vec<_> =
                g.world
                    .tiles
                    .iter()
                    .enumerate()
                    .filter(|(i, t)| {
                        !t.object.is_empty()
                            && e.sprite_bounds().intersects(g.world.tile_bounds(
                                (*i % g.world.width) as i32,
                                (*i / g.world.width) as i32,
                            ))
                    })
                    .map(|(i, _)| i)
                    .collect();
            for tile in tiles {
                collision(g, id, crate::dispatcher::Instance::Terrain(tile));
            }
        }
        let ids: Vec<_> = g
            .world
            .entities
            .iter()
            .filter(|e| e.alive)
            .map(|e| e.id)
            .collect();
        for id in ids {
            let e = g.world.entity_mut(id).unwrap();
            let speed = e.image_speed();
            let frames = catalog::sprite_frame_count(&e.sprite);
            if crate::animation::advance(&mut e.animation, speed, frames) {
                animation_end(g, id);
            }
        }
    }
    #[test]
    fn sign_preserves_both_metadata_lines_and_bounds_and_cooldown() {
        let mut g = room();
        let id = g.world.spawn("oMsgSign", Vec2::new(32.0, 32.0));
        let sign = g.world.entity_mut(id).unwrap();
        sign.text.insert("message".into(), "FIRST".into());
        sign.text.insert("message2".into(), "SECOND".into());
        advance(&mut g);
        assert_eq!(g.events, vec![GameEvent::Message("FIRST\nSECOND".into())]);
        for _ in 0..199 {
            advance(&mut g);
            assert!(g.events.is_empty());
        }
        assert_eq!(g.data["message_timer"], 200.0);
        advance(&mut g);
        assert_eq!(g.events.len(), 1);
        g.player.pos.x = 80.0;
        for _ in 0..200 {
            advance(&mut g);
        }
        assert!(g.events.is_empty());
        assert_eq!(g.data["message_timer"], 0.0);
    }
    #[test]
    fn source_lights_have_correct_ranges_origins_and_animation() {
        let mut g = room();
        for name in ["oLamp", "oLampRed", "oTikiTorch"] {
            g.world.spawn(name, Vec2::new(80.0, 80.0));
        }
        advance(&mut g);
        for e in &g.world.entities {
            assert_eq!(e.animation, 0.5);
            assert_eq!(e.value("light_offset_x"), 8.0);
            assert_eq!(e.value("light_offset_y"), 8.0);
            assert_eq!(
                e.value("light_radius"),
                match e.object.as_str() {
                    "oTikiTorch" => 32.0,
                    _ => 96.0,
                }
            );
        }
        assert_eq!(g.world.entities[1].value("red_light_range"), 96.0);
    }
    #[test]
    fn actual_damsel_help_rises_and_expires_after_its_source_alarm() {
        let mut g = room();
        for x in 0..g.world.width {
            g.world.set_tile(x, 9, "oBlock");
        }
        g.world.spawn("oDamsel", Vec2::new(120.0, 136.0));
        let (id, initial_y) = (0..240)
            .find_map(|_| {
                g.step(&InputFrame::default());
                g.world
                    .entities
                    .iter()
                    .find_map(|e| (e.alive && e.object == "oYellHelp").then_some((e.id, e.pos.y)))
            })
            .expect("the ordinary Damsel idle/yell sequence emits help");
        assert!((initial_y - 119.9).abs() < 1e-9);
        for _ in 0..39 {
            g.step(&InputFrame::default());
        }
        assert!((g.world.entity(id).unwrap().pos.y - (initial_y - 3.9)).abs() < 1e-9);
        g.step(&InputFrame::default());
        assert!(g.world.entity(id).is_none());
    }
    #[test]
    fn actual_tree_loss_collapses_both_directional_canopies_and_removes_support() {
        let mut g = room();
        g.world.set_tile(5, 5, "oTree");
        g.world.set_tile(5, 6, "oBlock");
        let left = g.world.spawn("oLeaves", Vec2::new(64.0, 80.0));
        initialize_leaves(&mut g.world, left, false);
        let right = g.world.spawn("oLeaves", Vec2::new(96.0, 80.0));
        initialize_leaves(&mut g.world, right, false);
        assert_eq!(g.world.entity(left).unwrap().sprite, "sLeaves");
        assert_eq!(g.world.entity(right).unwrap().sprite, "sLeavesRight");
        g.step(&InputFrame::default());
        assert!(g.world.entity(left).is_some() && g.world.entity(right).is_some());
        g.world.set_tile(5, 5, "");
        g.step(&InputFrame::default());
        assert!(g.world.entity(left).is_none() && g.world.entity(right).is_none());
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| e.object == "oLeaf")
                .count(),
            4
        );
        g.player.pos = Vec2::new(72.0, 64.0);
        g.player.vel = Vec2::new(0.0, 2.0);
        for _ in 0..8 {
            g.step(&InputFrame::default());
        }
        assert!(
            g.player.pos.y > 80.0,
            "unsupported canopy no longer catches the player"
        );
    }
    #[test]
    fn actual_top_canopy_requires_both_sides_and_dead_leaves_emit_no_debris() {
        let mut g = room();
        for x in [4, 6] {
            g.world.set_tile(x, 5, "oTree");
            g.world.set_tile(x, 6, "oBlock");
        }
        let top = g.world.spawn("oLeaves", Vec2::new(80.0, 80.0));
        initialize_leaves(&mut g.world, top, false);
        g.step(&InputFrame::default());
        assert_eq!(g.world.entity(top).unwrap().sprite, "sLeavesTop");
        g.world.set_tile(4, 5, "");
        g.step(&InputFrame::default());
        assert!(g.world.entity(top).is_none());
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| e.object == "oLeaf")
                .count(),
            2
        );

        let mut g = room();
        g.world.set_tile(5, 5, "oTree");
        let dead = g.world.spawn("oLeaves", Vec2::new(96.0, 80.0));
        initialize_leaves(&mut g.world, dead, true);
        assert_eq!(g.world.entity(dead).unwrap().sprite, "sLeavesDeadR");
        g.world.set_tile(5, 5, "");
        g.step(&InputFrame::default());
        assert!(g.world.entity(dead).is_none());
        assert!(!g.world.entities.iter().any(|e| e.object == "oLeaf"));
    }
    #[test]
    fn actual_blast_removes_unsupported_branch_but_preserves_tile_variants() {
        let mut g = room();
        g.player.pos = Vec2::new(240.0, 40.0);
        g.world.set_tile(5, 5, "oTree");
        let branch = g.world.spawn("oTreeBranch", Vec2::new(96.0, 80.0));
        let branch_tile = g.world.spawn("oTreeBranchTile", Vec2::new(144.0, 80.0));
        let leaves_tile = g.world.spawn("oLeavesTile", Vec2::new(176.0, 80.0));
        g.world.spawn("oExplosion", Vec2::new(88.0, 88.0));
        g.step(&InputFrame::default());
        assert!(g.world.tile(5, 5).unwrap().object.is_empty());
        assert!(g.world.entity(branch).is_none());
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| e.object == "oLeaf")
                .count(),
            1
        );
        assert!(g.world.entity(branch_tile).is_some());
        assert!(g.world.entity(leaves_tile).is_some());
    }
    #[test]
    fn actual_canopy_collapse_preserves_strict_view_boundary_and_dead_branches() {
        let mut g = room();
        g.world = World::new(50, 15, Area::Jungle, 5);
        let edge = g
            .world
            .spawn("oTreeBranch", Vec2::new(VIEW_WIDTH + 16.0, 80.0));
        let dead = g.world.spawn("oTreeBranch", Vec2::new(80.0, 80.0));
        g.world.entity_mut(dead).unwrap().sprite = "sTreeBranchDeadL".into();
        g.step(&InputFrame::default());
        assert!(g.world.entity(edge).is_some());
        assert!(g.world.entity(dead).is_none());
        assert!(!g.world.entities.iter().any(|e| e.object == "oLeaf"));
        g.world.entity_mut(edge).unwrap().pos.x -= 1.0;
        g.step(&InputFrame::default());
        assert!(g.world.entity(edge).is_none());
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| e.object == "oLeaf")
                .count(),
            1
        );
    }
    #[test]
    fn actual_psychic_player_effect_moves_collides_and_expires() {
        let mut g = room();
        let id = spawn_psychic_create_p(&mut g, Vec2::new(100.0, 80.0), Vec2::new(0.75, -2.0));
        g.step(&InputFrame::default());
        let e = g.world.entity(id).unwrap();
        assert_eq!(e.pos, Vec2::new(100.75, 78.0));
        assert!((e.vel.y + 1.4).abs() < 1e-9);
        assert_eq!(e.animation, 0.2);
        g.world.entity_mut(id).unwrap().vel.y = 5.8;
        g.step(&InputFrame::default());
        assert!((g.world.entity(id).unwrap().vel.y - 6.4).abs() < 1e-9);

        let mut g = room();
        let id = spawn_psychic_create_p(&mut g, Vec2::new(100.0, 40.0), Vec2::default());
        for _ in 0..29 {
            g.step(&InputFrame::default());
        }
        assert!(g.world.entity(id).is_some());
        g.step(&InputFrame::default());
        assert!(g.world.entity(id).is_none());

        let mut g = room();
        g.world.set_tile(5, 5, "oBlock");
        let id = spawn_psychic_create_p(&mut g, Vec2::new(79.0, 88.0), Vec2::default());
        assert!(!g.world.tiles_point(Vec2::new(79.0, 88.0), "oSolid"));
        g.step(&InputFrame::default());
        assert!(
            g.world.entity(id).is_none(),
            "sprite event collision extends beyond the origin"
        );
    }
    #[test]
    fn actual_olmec_create_effect_keeps_its_creation_heading_and_expires() {
        let mut g = room();
        let olmec = g.world.spawn("oOlmec", Vec2::new(200.0, 80.0));
        let id = spawn_psychic_create_2(&mut g, Vec2::new(100.0, 96.0), Vec2::new(232.0, 96.0));
        assert_eq!(g.world.entity(id).unwrap().vel, Vec2::new(2.0, 0.0));
        g.step(&InputFrame::default());
        assert_eq!(g.world.entity(id).unwrap().pos, Vec2::new(102.0, 96.0));
        g.world.entity_mut(olmec).unwrap().pos = Vec2::new(100.0, 200.0);
        g.step(&InputFrame::default());
        assert_eq!(g.world.entity(id).unwrap().pos, Vec2::new(104.0, 96.0));
        for _ in 2..14 {
            g.step(&InputFrame::default());
        }
        assert!(g.world.entity(id).is_some());
        g.step(&InputFrame::default());
        assert!(g.world.entity(id).is_none());
    }
    #[test]
    fn synchronous_bone_create_precedes_following_producer_draws_and_keeps_overrides() {
        let mut g = room();
        let mut source_draws = crate::Rng::new(1);
        let mut bones = Vec::new();
        for _ in 0..3 {
            source_draws.unit();
            source_draws.unit();
            source_draws.unit();
            source_draws.range(6);
            bones.push(spawn_detritus(&mut g, "oBone", Vec2::new(80.0, 80.0)));
        }
        assert_eq!(g.rng.range(9), source_draws.range(9));
        let bone = g.world.entity_mut(bones[0]).unwrap();
        bone.vel = Vec2::new(7.0, -3.0);
        g.step(&InputFrame::default());
        assert_eq!(g.world.entity(bones[0]).unwrap().vel.x, 7.0);
    }
    #[test]
    fn actual_generated_tree_order_collapses_in_step_after_explosion_collision() {
        let mut g = room();
        g.player.pos = Vec2::new(240.0, 40.0);
        g.world.set_tile(5, 9, "oBlock");
        let mut branch = 0;
        // Source tree() creates the trunk bottom-to-top, interleaving births.
        // Step still dispatches all Trees before Leaves and TreeBranches.
        for y in (4..=8).rev() {
            g.world.set_tile(5, y, "oTree");
            initialize_tree(&mut g.world, 5, y);
            if y == 6 {
                branch = g.world.spawn("oTreeBranch", Vec2::new(96.0, 96.0));
            }
        }
        let leaves: Vec<_> = [64.0, 96.0]
            .into_iter()
            .map(|x| {
                let id = g.world.spawn("oLeaves", Vec2::new(x, 64.0));
                initialize_leaves(&mut g.world, id, false);
                id
            })
            .collect();
        g.world.spawn("oExplosion", Vec2::new(88.0, 136.0));
        g.step(&InputFrame::default());
        g.step(&InputFrame::default());
        assert!((4..=8).all(|y| g.world.tile(5, y).unwrap().object.is_empty()));
        assert!(g.world.entity(branch).is_none());
        assert!(leaves.iter().all(|&id| g.world.entity(id).is_none()));
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| e.object == "oLeaf")
                .count(),
            5
        );
        let output: Vec<_> = g
            .world
            .entities
            .iter()
            .filter(|e| matches!(e.object.as_str(), "oRubble" | "oRubbleSmall" | "oLeaf"))
            .map(|e| e.object.as_str())
            .collect();
        let first_leaf = output.iter().position(|&object| object == "oLeaf").unwrap();
        assert!(
            output[first_leaf..].iter().all(|&object| object == "oLeaf"),
            "Tree Destroy outputs precede the later Leaves/Branch Step phases"
        );
    }
    #[test]
    fn actual_canopy_step_uses_object_order_even_when_instance_births_are_reversed() {
        let mut g = room();
        g.world.spawn("oTreeBranch", Vec2::new(96.0, 80.0));
        let leaves = g.world.spawn("oLeaves", Vec2::new(64.0, 80.0));
        g.world.spawn("oTree", Vec2::new(80.0, 80.0));
        initialize_leaves(&mut g.world, leaves, false);
        g.step(&InputFrame::default());
        let output: Vec<_> = g
            .world
            .entities
            .iter()
            .filter(|e| matches!(e.object.as_str(), "oRubble" | "oRubbleSmall" | "oLeaf"))
            .collect();
        assert_eq!(
            output.iter().map(|e| e.object.as_str()).collect::<Vec<_>>(),
            [
                "oRubble",
                "oRubbleSmall",
                "oRubbleSmall",
                "oLeaf",
                "oLeaf",
                "oLeaf"
            ]
        );
        assert!(
            output[3..5].iter().all(|e| e.pos.x <= 80.0),
            "Leaves Step precedes Branch Step"
        );
        assert!(
            output[5].pos.x >= 96.0,
            "Branch emits the final Leaf despite being born first"
        );
    }
    #[test]
    fn actual_authored_top_to_bottom_tree_order_cascades_once_per_source_step() {
        let mut g = room();
        g.player.pos = Vec2::new(240.0, 40.0);
        g.world.set_tile(5, 9, "oBlock");
        for y in 4..=8 {
            g.world.set_tile(5, y, "oTree");
            initialize_tree(&mut g.world, 5, y);
        }
        g.world.spawn("oExplosion", Vec2::new(88.0, 136.0));
        for remaining in [3, 2, 1, 0] {
            g.step(&InputFrame::default());
            assert_eq!(
                (4..=8)
                    .filter(|&y| g.world.tile(5, y).unwrap().object == "oTree")
                    .count(),
                remaining
            );
        }
    }
    #[test]
    fn actual_dynamic_tree_order_uses_full_destroy_and_preserves_tree_tile() {
        let mut g = room();
        g.player.pos = Vec2::new(240.0, 40.0);
        g.world.set_tile(5, 9, "oBlock");
        let trees: Vec<_> = (4..=8)
            .rev()
            .map(|y| g.world.spawn("oTree", Vec2::new(80.0, f64::from(y) * 16.0)))
            .collect();
        let tile = g.world.spawn("oTreeTile", Vec2::new(160.0, 80.0));
        g.world.spawn("oExplosion", Vec2::new(88.0, 136.0));
        g.step(&InputFrame::default());
        g.step(&InputFrame::default());
        assert!(trees.iter().all(|&id| g.world.entity(id).is_none()));
        assert!(g.world.entity(tile).is_some());
        assert!(g.world.entities.iter().any(|e| e.object == "oRubbleSmall"));
    }
    #[test]
    fn tree_create_reuses_metadata_and_keeps_the_creation_slot_idempotently() {
        let mut g = room();
        let metadata = g.world.spawn("oTerrainMetadata", Vec2::new(88.0, 88.0));
        g.world
            .entity_mut(metadata)
            .unwrap()
            .data
            .insert("cleanDeath".into(), 1.0);
        let earlier = g.world.spawn("oTreeBranch", Vec2::new(96.0, 80.0));
        g.world.set_tile(5, 5, "oTree");
        assert_eq!(initialize_tree(&mut g.world, 5, 5), metadata);
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| e.object == "oTerrainMetadata")
                .count(),
            1
        );
        assert_eq!(g.world.entity(metadata).unwrap().value("cleanDeath"), 1.0);
        let order: Vec<_> = g.world.entities.iter().map(|e| e.id).collect();
        assert_eq!(order, vec![earlier, metadata]);
        assert_eq!(initialize_tree(&mut g.world, 5, 5), metadata);
        assert_eq!(
            g.world.entities.iter().map(|e| e.id).collect::<Vec<_>>(),
            order
        );
    }
    #[test]
    fn source_one_shot_animation_lifetimes() {
        for (object, lifetime) in [
            ("oPoof", 15),
            ("oSmokePuff", 20),
            ("oBloodTrail", 9),
            ("oBloodSpark", 18),
            ("oLavaDrip", 13),
        ] {
            let mut g = room();
            let id = g.world.spawn(object, Vec2::new(80.0, 80.0));
            for _ in 1..lifetime {
                advance(&mut g);
                assert!(g.world.entity(id).is_some(), "{object}");
            }
            advance(&mut g);
            assert!(g.world.entity(id).is_none(), "{object}");
        }
    }
    #[test]
    fn lava_drips_inherit_motion_gravity_and_solid_collision() {
        let mut g = room();
        let id = g.world.spawn("oLavaDrip", Vec2::new(80.0, 80.0));
        g.step(&InputFrame::default());
        assert_eq!(g.world.entity(id).unwrap().pos.y, 80.0);
        assert_eq!(g.world.entity(id).unwrap().vel.y, 0.6);
        g.step(&InputFrame::default());
        assert_eq!(g.world.entity(id).unwrap().pos.y, 80.6);
        assert_eq!(g.world.entity(id).unwrap().vel.y, 1.2);
        assert_eq!(g.world.entity(id).unwrap().animation, 0.8);
        g.world.set_tile(6, 5, "oBrick");
        let blocked = g.world.spawn("oLavaDrip", Vec2::new(100.0, 84.0));
        g.step(&InputFrame::default());
        assert!(g.world.entity(blocked).is_none());
    }
    #[test]
    fn dark_rubble_outside_room_uses_complete_sprite_bounds() {
        let mut g = room();
        let outside = g.world.spawn("oRubbleDarkSmall", Vec2::new(-10.0, 80.0));
        let partial = g.world.spawn("oRubbleDark", Vec2::new(-1.0, 80.0));
        g.step(&InputFrame::default());
        assert!(g.world.entity(outside).is_none());
        assert!(g.world.entity(partial).is_some());
    }
    #[test]
    fn webbed_leaf_and_lava_drip_stop_animation_and_remain_caught() {
        let mut g = room();
        for x in 0..20 {
            g.world.set_tile(x, 3, "oBrick");
        }
        g.world.spawn("oWeb", Vec2::new(76.0, 76.0));
        let leaf = g.world.spawn("oLeaf", Vec2::new(80.0, 80.0));
        let drip = g.world.spawn("oLavaDrip", Vec2::new(80.0, 80.0));
        for _ in 0..20 {
            g.step(&InputFrame::default());
        }
        assert_eq!(g.world.entity(leaf).unwrap().animation, 0.0);
        assert_eq!(g.world.entity(drip).unwrap().animation, 0.0);
    }
    fn dark_room() -> Game {
        let mut g = room();
        for x in 0..20 {
            g.world.set_tile(x, 3, "oBrick");
        }
        g.world.dark = true;
        g.player.facing = -1;
        g
    }
    #[test]
    fn canonical_darkness_tracks_lamp_approach_and_source_family_inheritance() {
        let mut g = dark_room();
        g.world.spawn("oLampRed", Vec2::new(80.0, 32.0));
        g.step(&InputFrame::default());
        // StandLeft ends at x45; lamp frame zero starts at x82.
        assert_eq!(g.data["darkness"], 37.0 / 160.0);
        g.player.pos.x = 60.0;
        g.step(&InputFrame::default());
        assert_eq!(g.data["darkness"], 17.0 / 160.0);
    }
    #[test]
    fn canonical_darkness_clamps_unlit_rooms_and_honors_crown_during_dead_simulation() {
        let mut g = dark_room();
        g.step(&InputFrame::default());
        assert_eq!(g.data["darkness"], 0.9);
        g.player.equipment.insert("Crown".into());
        g.step(&InputFrame::default());
        assert_eq!(g.data["darkness"], 0.0);
        g.player.health = 0;
        g.state = crate::RunState::Dead;
        g.step(&InputFrame::default());
        assert_eq!(g.data["darkness"], 0.0);
    }
    #[test]
    fn canonical_darkness_keeps_source_explosion_flash_below_zero() {
        let mut g = dark_room();
        g.player.health = 100;
        let id = g.world.spawn("oExplosion", g.player.pos);
        let explosion = g.world.entity_mut(id).unwrap();
        crate::items::init_entity(explosion);
        explosion.animation = 3.0;
        g.step(&InputFrame::default());
        assert_eq!(g.data["darkness"], -48.0 / 160.0);
    }
    #[test]
    fn canonical_darkness_consumes_flare_step_cache_after_player_repositioning() {
        let mut g = dark_room();
        let flare = g.world.spawn("oFlare", Vec2::new(80.0, 32.0));
        g.step(&InputFrame::default());
        assert_eq!(
            g.data["darkness"], 0.0,
            "Flare Create initializes its cache to zero"
        );
        let cached = g.world.entity(flare).unwrap().value("dist_to_player");
        assert!(cached > 0.0);
        g.player.pos.x = 200.0;
        g.step(&InputFrame::default());
        assert_eq!(g.data["darkness"], (cached / 160.0).min(0.9));
        assert_eq!(g.data.get("darkness_lerp").copied().unwrap_or(0.0), 0.0);
    }
    #[test]
    fn canonical_darkness_lerp_consumption_and_room_create_reset_follow_source() {
        let mut g = dark_room();
        g.player.equipment.insert("Crown".into());
        g.data.insert("darkness_lerp".into(), 1.0);
        for expected in [0.9, 0.9, 0.8] {
            g.step(&InputFrame::default());
            assert!((g.data["darkness"] - expected).abs() < 1e-9);
        }
        g.next_level();
        assert_eq!(g.data["darkness_lerp"], 0.0);
        assert_eq!(g.data["darkness"], if g.world.dark { 1.0 } else { 0.0 });
    }
    #[test]
    fn title_hint_hand_loops_source_frames_without_moving_or_expiring() {
        let mut g = room();
        g.flags.insert("hub".into());
        for x in 0..20 {
            g.world.set_tile(x, 3, "oBrick");
        }
        let pos = Vec2::new(88.0, 80.0);
        let id = g.world.spawn("oHintHand", pos);
        for step in 1..=20 {
            g.step(&InputFrame::default());
            let hand = g.world.entity(id).unwrap();
            assert_eq!(hand.pos, pos);
            assert_eq!(hand.animation, (f64::from(step) * 0.5) % 5.0);
            assert_eq!(hand.sprite, "sHintPoint");
        }
    }
    fn contained_pool(object: &str, tiles: bool) -> Game {
        let mut g = room();
        g.flags.insert("custom".into());
        for x in 0..20 {
            g.world.set_tile(x, 3, "oBrick");
        }
        for x in 5..9 {
            g.world.set_tile(x, 6, "oBrick");
            if tiles {
                g.world.set_tile(x, 5, object);
            } else {
                g.world.spawn(object, Vec2::new(x as f64 * 16.0, 80.0));
            }
        }
        g.world.set_tile(4, 5, "oBrick");
        g.world.set_tile(9, 5, "oBrick");
        g.world.fluid_dirty = false;
        g
    }
    #[test]
    fn mattock_breach_triggers_source_fluid_drainage_on_next_global_pass() {
        for object in ["oWaterSwim", "oLava"] {
            for tiles in [false, true] {
                let mut g = contained_pool(object, tiles);
                g.player.pos = Vec2::new(64.0, 88.0);
                g.player.facing = 1;
                g.player.attack_timer = 3;
                g.player.set_sprite("sWhipRight");
                let hit = g.world.spawn("oMattockHit", Vec2::new(80.0, 96.0));
                let hit = g.world.entity_mut(hit).unwrap();
                hit.animation = catalog::sprite_frame_count(&hit.sprite) as f64 - 0.5;
                g.step(&InputFrame::default());
                assert!(g.world.tile(5, 6).unwrap().object.is_empty());
                assert!(
                    g.world.fluid_dirty,
                    "solid Destroy must request the global pass"
                );
                for _ in 0..8 {
                    g.step(&InputFrame::default());
                }
                assert!(
                    !g.world
                        .entities
                        .iter()
                        .any(|e| e.alive && catalog::is_a(&e.object, "oWater")),
                    "{object} dynamic={}",
                    !tiles
                );
                assert!(
                    !g.world
                        .tiles
                        .iter()
                        .any(|tile| catalog::is_a(&tile.object, "oWater"))
                );
                assert!(!g.world.fluid_dirty);
            }
        }
    }
    #[test]
    fn unrequested_pool_stays_intact_and_fluid_pass_keeps_source_exemptions() {
        let mut quiet = contained_pool("oWaterSwim", false);
        quiet.step(&InputFrame::default());
        assert_eq!(
            quiet
                .world
                .entities
                .iter()
                .filter(|e| e.object == "oWaterSwim")
                .count(),
            4
        );
        for (boss, custom, y, expected) in [
            (true, false, 80.0, 1),
            (false, false, 512.0, 1),
            (false, true, 512.0, 0),
        ] {
            let mut g = room();
            g.world = World::new(20, 40, if boss { Area::Boss } else { Area::Jungle }, 5);
            for x in 0..20 {
                g.world.set_tile(x, 3, "oBrick");
            }
            if custom {
                g.flags.insert("custom".into());
            }
            if y >= 512.0 {
                g.camera.y = y - 160.0;
            }
            g.world.spawn("oWaterSwim", Vec2::new(80.0, y));
            g.world.fluid_dirty = true;
            g.step(&InputFrame::default());
            assert_eq!(
                g.world
                    .entities
                    .iter()
                    .filter(|e| e.alive && e.object == "oWaterSwim")
                    .count(),
                expected
            );
        }
    }
    #[test]
    fn fluid_pass_uses_prior_activation_and_fixed_descendant_count() {
        let mut g = room();
        g.world = World::new(80, 20, Area::Jungle, 5);
        g.flags.insert("custom".into());
        for x in 0..80 {
            g.world.set_tile(x, 3, "oBrick");
        }
        let chain: Vec<_> = [392.0, 408.0, 424.0, 440.0]
            .into_iter()
            .map(|x| g.world.spawn("oWaterSwim", Vec2::new(x, 80.0)))
            .collect();
        let isolated = g.world.spawn("oWaterSwim", Vec2::new(600.0, 80.0));
        // Fresh source instances start active. The first Level553 Step performs
        // the initial region deactivation before a later Game318 fluid request.
        g.step(&InputFrame::default());
        g.world.fluid_dirty = true;
        g.step(&InputFrame::default());
        assert!(chain[..2].iter().all(|id| g.world.entity(*id).is_none()));
        assert!(chain[2..].iter().all(|id| g.world.entity(*id).is_some()));
        assert_eq!(g.data["water_counter"], 2.0);
        assert!(g.world.entity(isolated).is_some());
        g.step(&InputFrame::default());
        assert!(g.world.entity(isolated).is_some());
        assert!(!g.world.fluid_dirty);
    }
    #[test]
    fn magma_inherits_detritus_lifetime_and_instance_cap_without_replacing_launch() {
        let mut g = room();
        for x in 0..20 {
            g.world.set_tile(x, 3, "oBrick");
        }
        let id = g.world.spawn("oMagma", Vec2::new(80.0, 80.0));
        let magma = g.world.entity_mut(id).unwrap();
        magma.data.insert("started".into(), 1.0);
        magma.data.insert("world_effect_initialized".into(), 1.0);
        magma.data.insert("life".into(), 1.0);
        magma.data.insert("gravity".into(), 0.0);
        magma.vel = Vec2::new(1.0, 0.0);
        g.step(&InputFrame::default());
        assert_eq!(g.world.entity(id).unwrap().pos, Vec2::new(81.0, 80.0));
        g.step(&InputFrame::default());
        assert!(g.world.entity(id).is_none());
        let mut g = room();
        for _ in 0..32 {
            g.world.spawn("oBone", Vec2::new(80.0, 80.0));
        }
        let rejected = g.world.spawn("oMagma", Vec2::new(80.0, 80.0));
        g.step(&InputFrame::default());
        assert!(g.world.entity(rejected).is_none());
    }
    fn configured_lava(g: &mut Game, pos: Vec2, terrain: bool) -> u64 {
        for x in 0..20 {
            g.world.set_tile(x, 3, "oBrick");
        }
        let id = if terrain {
            g.world
                .set_tile((pos.x / 16.0) as usize, (pos.y / 16.0) as usize, "oLava");
            g.world
                .spawn("oTerrainMetadata", Vec2::new(pos.x + 8.0, pos.y + 8.0))
        } else {
            g.world.spawn("oLava", pos)
        };
        let lava = g.world.entity_mut(id).unwrap();
        if terrain {
            lava.text.insert("terrain_sprite".into(), "sLavaTop".into());
        } else {
            lava.set_sprite("sLavaTop");
        }
        lava.data.insert("lava_initialized".into(), 1.0);
        lava.data.insert("spurt".into(), 1.0);
        lava.data.insert("spurt_time".into(), 100.0);
        lava.data.insert("spurt_counter".into(), 1.0);
        id
    }
    #[test]
    fn lava_and_tile_metadata_animate_and_spurt_on_source_counter_boundary() {
        for terrain in [false, true] {
            let mut g = room();
            // Canonical seed 10 selects Flame, whose resource 456 Step has
            // already passed when Lava 573 creates it. This preserves the birth
            // position witness; Magma 649 would move during this same Step.
            g.rng = crate::Rng::new(10);
            let id = configured_lava(&mut g, Vec2::new(80.0, 96.0), terrain);
            g.step(&InputFrame::default());
            let lava = g.world.entity(id).unwrap();
            assert_eq!(lava.animation, 0.4);
            assert_eq!(lava.value("spurt_counter"), 0.0);
            assert!(
                !g.world
                    .entities
                    .iter()
                    .any(|e| matches!(e.object.as_str(), "oFlame" | "oMagma"))
            );
            g.step(&InputFrame::default());
            let lava = g.world.entity(id).unwrap();
            assert_eq!(lava.animation, 0.8);
            assert_eq!(lava.value("spurt_counter"), 100.0);
            let particle = g
                .world
                .entities
                .iter()
                .find(|e| matches!(e.object.as_str(), "oFlame" | "oMagma"))
                .unwrap();
            assert_eq!(particle.pos, Vec2::new(88.0, 92.0));
            assert!((-4.0..=-1.0).contains(&particle.vel.y));
            let particle_id = particle.id;
            g.step(&InputFrame::default());
            assert!(
                g.world.entity(particle_id).unwrap().pos.y < 92.0,
                "Create must preserve launch override"
            );
        }
    }
    #[test]
    fn lava_spurt_clock_pauses_at_exact_240_pixel_distance_and_authored_defaults_stay_off() {
        let mut g = room();
        let id = configured_lava(&mut g, Vec2::new(280.0, 40.0), false);
        g.step(&InputFrame::default());
        assert_eq!(g.world.entity(id).unwrap().value("spurt_counter"), 1.0);
        g.player.pos.x = 41.0;
        g.step(&InputFrame::default());
        assert_eq!(g.world.entity(id).unwrap().value("spurt_counter"), 0.0);
        let authored = g.world.spawn("oLava", Vec2::new(96.0, 96.0));
        g.step(&InputFrame::default());
        let authored = g.world.entity(authored).unwrap();
        assert_eq!(authored.value("spurt"), 0.0);
        assert!((100.0..=300.0).contains(&authored.value("spurt_time")));
        assert_eq!(
            authored.value("spurt_counter"),
            authored.value("spurt_time")
        );
        assert_eq!(authored.animation, 0.4);
    }
    #[test]
    fn lava_spurts_initialize_both_flame_and_magma_before_overriding_launch() {
        let mut seen = std::collections::BTreeSet::new();
        for seed in 0..128 {
            let mut g = room();
            g.rng = crate::Rng::new(seed);
            let id = configured_lava(&mut g, Vec2::new(80.0, 96.0), false);
            g.world
                .entity_mut(id)
                .unwrap()
                .data
                .insert("spurt_counter".into(), 0.0);
            let mut source = g.rng.clone();
            let magma = source.chance(1, 8);
            source.unit();
            source.unit();
            source.unit();
            let gravity = f64::from(1 + source.range(6)) * 0.1;
            let launch = -f64::from(1 + source.range(4));
            g.step(&InputFrame::default());
            let particle = g
                .world
                .entities
                .iter()
                .find(|e| matches!(e.object.as_str(), "oFlame" | "oMagma"))
                .unwrap();
            seen.insert(particle.object.clone());
            assert_eq!(particle.object == "oMagma", magma);
            // Magma649 receives Step after Lava573 creates it. Flame456's
            // resource phase has already passed, so its first Step is tomorrow.
            assert!((particle.vel.y - (launch + if magma { gravity } else { 0.0 })).abs() < 1e-9);
            assert_eq!(particle.value("life"), if magma { 59.0 } else { 60.0 });
            assert_eq!(particle.value("world_effect_initialized"), 1.0);
            if particle.object == "oMagma" {
                assert_eq!(particle.health, 200);
                assert_eq!(particle.value("started"), 1.0);
                assert_eq!(particle.value("collision_left"), -8.0);
            }
        }
        assert_eq!(
            seen,
            ["oFlame".to_owned(), "oMagma".to_owned()]
                .into_iter()
                .collect()
        );
    }
    #[test]
    fn game_steps_animate_and_remove_splash_burn_and_laser_explosion() {
        // These are real gameplay-spawned effects, not persistent decorations.
        // Previously each survived indefinitely at frame zero in Game::step.
        for (object, speed, lifetime) in [
            ("oSplash", 0.6, 11),
            ("oBurn", 0.4, 23),
            ("oLaserExplode", 0.8, 11),
            ("oFlameTrail", 0.4, 13),
            ("oYellowTrail", 1.0, 12),
            ("oLaserTrail", 0.8, 7),
        ] {
            let mut g = room();
            for x in 0..20 {
                g.world.set_tile(x, 3, "oBrick");
            }
            let id = spawn_effect(&mut g, object, Vec2::new(80.0, 80.0));
            assert_eq!(
                g.events
                    .iter()
                    .filter(
                        |event| matches!(event,GameEvent::Sound{name,..} if name=="sndSmallExplode")
                    )
                    .count(),
                usize::from(object == "oLaserExplode")
            );
            for step in 1..lifetime {
                g.step(&InputFrame::default());
                let effect = g.world.entity(id).expect(object);
                assert!((effect.animation - speed * f64::from(step)).abs() < 1e-9);
                if object == "oBurn" {
                    assert!((effect.pos.y - (80.0 - 0.1 * f64::from(step))).abs() < 1e-9);
                }
                let sounds = g.events.iter().filter(|event| {
                    matches!(event, GameEvent::Sound { name, .. } if name == "sndSmallExplode")
                }).count();
                assert_eq!(sounds, 0);
            }
            g.step(&InputFrame::default());
            assert!(g.world.entity(id).is_none(), "{object} must finish once");
            assert!(g.player.health > 0);
        }
    }
    #[test]
    fn burn_uses_source_origin_point_collision_against_moving_solids() {
        let mut g = room();
        for x in 0..20 {
            g.world.set_tile(x, 3, "oBrick");
            g.world.set_tile(x, 7, "oBrick");
        }
        g.world.spawn("oPushBlock", Vec2::new(80.0, 96.0));
        let touching = g.world.spawn("oBurn", Vec2::new(80.0, 104.0));
        let outside = g.world.spawn("oBurn", Vec2::new(79.9, 104.0));
        g.step(&InputFrame::default());
        assert!(g.world.entity(touching).is_none());
        assert!(g.world.entity(outside).is_some());
    }
    #[test]
    fn burn_point_queries_see_tall_tile_masks_below_their_storage_cell() {
        let mut g = room();
        for x in 0..20 {
            g.world.set_tile(x, 3, "oBrick");
        }
        g.world.set_tile(5, 4, "oMoai");
        let id = g.world.spawn("oBurn", Vec2::new(88.0, 104.0));
        assert!(g.world.tile(5, 6).unwrap().object.is_empty());
        g.step(&InputFrame::default());
        assert!(g.world.entity(id).is_none());
    }
    #[test]
    fn laser_wall_collision_produces_a_finite_animated_explosion() {
        let mut g = room();
        for x in 0..20 {
            g.world.set_tile(x, 3, "oBrick");
        }
        g.world.set_tile(6, 6, "oBlock");
        let laser = g.world.spawn("oLaser", Vec2::new(104.0, 104.0));
        g.step(&InputFrame::default());
        assert!(g.world.entity(laser).is_none());
        let explosion = g
            .world
            .entities
            .iter()
            .find(|e| e.object == "oLaserExplode")
            .unwrap();
        assert_eq!(explosion.animation, 0.8);
        let id = explosion.id;
        assert!(g.events.iter().any(|event| {
            matches!(event, GameEvent::Sound { name, .. } if name == "sndSmallExplode")
        }));
        for _ in 1..11 {
            g.step(&InputFrame::default());
        }
        assert!(g.world.entity(id).is_none());
    }
    #[test]
    fn altar_flames_bounce_emit_bounded_trails_and_finish_as_smoke() {
        let mut g = room();
        for x in 0..20 {
            g.world.set_tile(x, 3, "oBrick");
            g.world.set_tile(x, 7, "oBrick");
        }
        let id = g.world.spawn("oFlame", Vec2::new(80.0, 104.0));
        // Fixed initial launch isolates the source life/grounding behavior from
        // random launch velocity; all updates still run through Game::step.
        let flame = g.world.entity_mut(id).unwrap();
        flame.data.insert("world_effect_initialized".into(), 1.0);
        flame.data.insert("life".into(), 60.0);
        flame.data.insert("gravity".into(), 0.6);
        flame.vel = Vec2::new(0.0, 1.0);
        g.step(&InputFrame::default());
        assert_eq!(g.world.entity(id).unwrap().sprite, "sFlame");
        g.step(&InputFrame::default());
        assert!(g.world.entities.iter().any(|e| e.object == "oFlameTrail"));
        for _ in 0..60 {
            g.step(&InputFrame::default());
            assert!(
                g.world
                    .entities
                    .iter()
                    .filter(|e| e.object == "oFlameTrail")
                    .count()
                    <= 12
            );
        }
        assert!(g.world.entity(id).is_none());
        assert!(
            g.world
                .entities
                .iter()
                .all(|e| !matches!(e.object.as_str(), "oFlame" | "oFlameTrail" | "oSmokePuff"))
        );
    }
    #[test]
    fn water_extinguishes_flame_with_collision_and_destroy_smoke_events() {
        let mut g = room();
        for x in 0..20 {
            g.world.set_tile(x, 3, "oBrick");
        }
        g.world.set_tile(5, 5, "oWaterSwim");
        let id = g.world.spawn("oFlame", Vec2::new(88.0, 88.0));
        let flame = g.world.entity_mut(id).unwrap();
        flame.data.insert("world_effect_initialized".into(), 1.0);
        flame.data.insert("life".into(), 60.0);
        flame.data.insert("gravity".into(), 0.0);
        g.step(&InputFrame::default());
        assert!(g.world.entity(id).is_none());
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| e.object == "oSmokePuff")
                .count(),
            2
        );
    }
    #[test]
    fn detritus_move_to_ignores_head_overlap_in_horizontal_probe() {
        let mut g = room();
        g.world.set_tile(5, 4, "oBlock");
        let id = g.world.spawn("oBone", Vec2::new(76.0, 80.0));
        let bone = g.world.entity_mut(id).unwrap();
        bone.data.insert("world_effect_initialized".into(), 1.0);
        bone.data.insert("life".into(), 60.0);
        bone.data.insert("gravity".into(), 0.0);
        bone.vel = Vec2::new(1.0, 0.0);
        g.step(&InputFrame::default());
        assert_eq!(g.world.entity(id).unwrap().pos.x, 77.0);
    }
    #[test]
    fn particle_alarm_trails_remain_at_pre_movement_position() {
        for (object, trail, first_emission) in
            [("oBlood", "oBloodTrail", 1), ("oFlame", "oFlameTrail", 2)]
        {
            let mut g = room();
            let id = g.world.spawn(object, Vec2::new(80.0, 80.0));
            let particle = g.world.entity_mut(id).unwrap();
            particle.data.insert("world_effect_initialized".into(), 1.0);
            particle.data.insert("life".into(), 60.0);
            particle.data.insert("gravity".into(), 0.0);
            particle.vel = Vec2::new(2.0, 0.0);
            for _ in 0..first_emission {
                g.step(&InputFrame::default());
            }
            let particle = g.world.entity(id).unwrap();
            let trail = g.world.entities.iter().find(|e| e.object == trail).unwrap();
            assert_eq!(trail.pos, Vec2::new(particle.pos.x - 2.0, particle.pos.y));
        }
    }
    #[test]
    fn ice_trim_drips_and_requires_ceiling_support() {
        let mut g = room();
        g.world.set_tile(5, 3, "oIce");
        let id = g.world.spawn("oIceBottom", Vec2::new(80.0, 64.0));
        g.world
            .entity_mut(id)
            .unwrap()
            .data
            .insert("drip_alarm".into(), 1.0);
        advance(&mut g);
        assert!(g.world.entities.iter().any(|e| e.object == "oDrip"));
        g.world.set_tile(5, 3, "");
        advance(&mut g);
        assert!(g.world.entity(id).is_none());
    }
    #[test]
    fn explicit_frozen_caveman_releases_once_with_source_stun() {
        let mut g = room();
        g.world.set_tile(5, 4, "oIce");
        let frozen = g.world.spawn("oFrozenCaveman", Vec2::new(80.0, 64.0));
        advance(&mut g);
        assert!(g.world.entity(frozen).is_some());
        g.world.set_tile(5, 4, "");
        advance(&mut g);
        advance(&mut g);
        let released: Vec<_> = g
            .world
            .entities
            .iter()
            .filter(|e| e.alive && e.object == "oCaveman")
            .collect();
        assert_eq!(released.len(), 1);
        assert_eq!(released[0].state, 98);
        assert_eq!(released[0].timer, 200);
        assert_eq!(released[0].value("invincible"), 20.0);
    }
    #[test]
    fn fake_bones_rise_once_and_make_a_real_skeleton() {
        let mut g = room();
        g.player.pos = Vec2::new(40.0, 40.0);
        let id = g.world.spawn("oFakeBones", Vec2::new(32.0, 32.0));
        for _ in 0..6 {
            advance(&mut g);
        }
        assert!(g.world.entity(id).is_none());
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| e.object == "oSkeleton")
                .count(),
            1
        );
    }
    #[test]
    fn parachute_visual_follows_opens_and_becomes_used() {
        let mut g = room();
        g.player.data.insert("parachute_open".into(), 1.0);
        for _ in 0..5 {
            advance(&mut g);
        }
        let e = g
            .world
            .entities
            .iter()
            .find(|e| e.alive && e.object == "oParachute")
            .unwrap();
        assert_eq!(e.sprite, "sParachute");
        assert_eq!(e.pos, Vec2::new(32.0, 24.0));
        g.player.data.insert("parachute_open".into(), 0.0);
        advance(&mut g);
        assert!(
            !g.world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oParachute")
        );
        assert!(
            g.world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oParaUsed")
        );
    }
    #[test]
    fn naturally_deployed_parachute_is_interrupted_by_yeti_zero_timer_and_timed_stun() {
        let mut level = crate::level::Level::default();
        level.set_tile(20, 2, '@').unwrap();
        for x in 0..40 {
            level.set_tile(x, 28, '1').unwrap();
        }
        let mut g = Game::shell(11, RulesPreset::Classic);
        g.load_custom(&level, "parachute-throw", "");
        let pickup = g.world.spawn(
            "oParaPickup",
            Vec2::new(g.player.pos.x, g.player.pos.y + 4.0),
        );
        g.world
            .entity_mut(pickup)
            .unwrap()
            .data
            .insert("cost".into(), 0.0);
        // Source Player Step1219 starts a whip before airborne Down+Attack
        // pickup. Use a real first attack so the next attack reaches the
        // pickup else-if while whipping, without changing the actor state.
        g.step(&InputFrame {
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert!(g.player.attack_timer > 0);
        // Release before the next ordinary press; keep the whip in progress.
        g.step(&InputFrame {
            attack_released: true,
            ..Default::default()
        });
        assert!(g.player.attack_timer > 0);
        g.step(&InputFrame {
            down: true,
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert!(g.player.has("Parachute"));
        for _ in 0..100 {
            g.step(&InputFrame::default());
            if g.world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oParachute")
            {
                break;
            }
        }
        assert!(!g.player.has("Parachute"));
        assert_eq!(g.player.data["parachute_open"], 1.0);
        assert!(
            g.world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oParachute")
        );

        let mut timed = g.clone();
        timed.player.stun = 20;
        timed.step(&InputFrame::default());
        assert!(timed.player.is_stunned());
        assert_eq!(timed.player.data["parachute_open"], 0.0);
        assert!(
            !timed
                .world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oParachute")
        );

        let p = g.player.pos;
        g.world.spawn("oYeti", Vec2::new(p.x - 8.0, p.y - 14.0));
        g.step(&InputFrame::default());
        assert!(g.player.is_stunned());
        assert_eq!(g.player.stun, 0, "source Yeti throw supplies no timer");
        g.step(&InputFrame::default());
        assert_eq!(g.player.data["parachute_open"], 0.0);
        assert!(
            !g.world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oParachute")
        );
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == "oParaUsed")
                .count(),
            1
        );
        let mut long_fall = false;
        for _ in 0..60 {
            g.step(&InputFrame::default());
            if g.player.data["fall_timer"] > 16.0 {
                long_fall = true;
                break;
            }
        }
        assert!(
            long_fall,
            "interrupted parachute no longer resets descending ticks"
        );
        assert_eq!(g.player.data["wall_hurt"], 0.0);
    }
    #[test]
    fn moving_item_cuts_parachute_with_source_direction_condition() {
        let mut g = room();
        g.player.data.insert("parachute_open".into(), 1.0);
        let rock = g.world.spawn("oRock", Vec2::new(40.0, 28.0));
        g.world.entity_mut(rock).unwrap().vel.x = 1.0;
        // The rock touches the canopy's engine rectangle but is above the
        // opening frame's actual sprite mask, so it must not cut it.
        advance(&mut g);
        assert!(
            g.world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oParachute")
        );
        let item = g.world.entity_mut(rock).unwrap();
        item.pos.y = 36.0;
        item.vel.x = -1.0;
        advance(&mut g);
        assert!(
            g.world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oParachute")
        );
        g.world.entity_mut(rock).unwrap().vel.x = 1.0;
        advance(&mut g);
        assert!(
            !g.world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oParachute")
        );
        assert_eq!(g.player.data["parachute_open"], 0.0);
    }
    #[test]
    fn integrated_sign_event_uses_player_sprite_mask_at_movement_edge() {
        let mut g = room();
        for x in 0..20 {
            g.world.set_tile(x, 3, "oBrick");
        }
        g.player.grounded = true;
        g.player.state = crate::PlayerState::Standing;
        let sign = g.world.spawn("oMsgSign", Vec2::new(46.0, 32.0));
        g.world
            .entity_mut(sign)
            .unwrap()
            .text
            .insert("message".into(), "SPRITE EDGE".into());
        g.step(&InputFrame::default());
        assert!(
            !g.events.contains(&GameEvent::Message("SPRITE EDGE".into())),
            "first Collision uses Create scale +1 before the first Draw"
        );
        assert!(
            !g.player
                .bounds()
                .intersects(g.world.entity(sign).unwrap().sprite_bounds())
        );
        assert!(
            g.player
                .sprite_bounds()
                .intersects(g.world.entity(sign).unwrap().sprite_bounds())
        );
        // The first Draw persisted the right-facing scale. The following
        // Collision can now reach the same sign without moving either actor.
        g.step(&InputFrame::default());
        assert!(g.events.contains(&GameEvent::Message("SPRITE EDGE".into())));
    }
    #[test]
    fn switching_back_to_a_sign_replaces_current_message_before_cooldown() {
        let mut g = room();
        for (x, text) in [(32.0, "FIRST"), (80.0, "SECOND")] {
            let id = g.world.spawn("oMsgSign", Vec2::new(x, 32.0));
            g.world
                .entity_mut(id)
                .unwrap()
                .text
                .insert("message".into(), text.into());
        }
        advance(&mut g);
        assert_eq!(g.events, vec![GameEvent::Message("FIRST".into())]);
        g.player.pos.x = 88.0;
        advance(&mut g);
        assert_eq!(g.events, vec![GameEvent::Message("SECOND".into())]);
        g.player.pos.x = 40.0;
        advance(&mut g);
        assert_eq!(g.events, vec![GameEvent::Message("FIRST".into())]);
    }
    #[test]
    fn room_clock_quantizes_detritus_without_using_lifetime_tick() {
        let mut g = room();
        g.tick = 101;
        let id = g.world.spawn("oBone", Vec2::new(80.0, 80.0));
        let e = g.world.entity_mut(id).unwrap();
        e.data.insert("world_effect_initialized".into(), 1.0);
        e.data.insert("life".into(), 60.0);
        e.data.insert("gravity".into(), 0.0);
        // Non-tie reciprocal 1/.3 rounds to period 3, isolating room-clock phase.
        // The separate ties-even regressions cover .4, whose period is 2.
        e.vel = Vec2::new(0.3, 0.0);
        advance(&mut g); // source time 2, lifetime tick 102: no displacement
        assert_eq!(g.world.entity(id).unwrap().pos.x, 80.0);
        advance(&mut g); // source time 3, lifetime tick 103: one pixel
        assert_eq!(g.world.entity(id).unwrap().pos.x, 81.0);
        g.level_tick = 0;
        advance(&mut g); // another room restarts the phase at source time 2
        assert_eq!(g.world.entity(id).unwrap().pos.x, 81.0);
        advance(&mut g); // restarted source time 3, not unreset time 5
        assert_eq!(g.world.entity(id).unwrap().pos.x, 82.0);
    }
    #[test]
    fn detritus_cap_preserves_oldest_instances_and_rejects_newest() {
        let mut g = room();
        let ids: Vec<_> = (0..33)
            .map(|_| g.world.spawn("oBone", Vec2::new(80.0, 80.0)))
            .collect();
        advance(&mut g);
        assert!(ids[..32].iter().all(|id| g.world.entity(*id).is_some()));
        assert!(g.world.entity(ids[32]).is_none());
    }
    #[test]
    fn source_point_collision_sees_moving_solid_support_but_not_room_boundary() {
        let mut g = room();
        let support = g.world.spawn("oPushBlock", Vec2::new(80.0, 48.0));
        let ice = g.world.spawn("oIceBottom", Vec2::new(80.0, 64.0));
        advance(&mut g);
        assert!(g.world.entity(ice).is_some());
        g.world.entity_mut(support).unwrap().alive = false;
        advance(&mut g);
        assert!(g.world.entity(ice).is_none());
        let edge = g.world.spawn("oIceBottom", Vec2::new(80.0, 0.0));
        advance(&mut g);
        assert!(g.world.entity(edge).is_none());
    }
    #[test]
    fn adjacent_frozen_metadata_does_not_suppress_explicit_occupant_release() {
        let mut g = room();
        let metadata = g.world.spawn("oTerrainMetadata", Vec2::new(72.0, 72.0));
        g.world
            .entity_mut(metadata)
            .unwrap()
            .data
            .insert("frozen_caveman".into(), 1.0);
        g.world.spawn("oFrozenCaveman", Vec2::new(80.0, 64.0));
        advance(&mut g);
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == "oCaveman")
                .count(),
            1
        );
    }
    #[test]
    fn phase_effect_step_does_not_advance_or_expire_animation() {
        let mut g = room();
        let id = spawn_effect(&mut g, "oBurn", Vec2::new(80.0, 80.0));
        g.world.entity_mut(id).unwrap().animation = 8.8;
        step_instance(&mut g, id);
        let e = g.world.entity(id).unwrap();
        assert_eq!(e.animation, 8.8);
        assert!((e.pos.y - 79.9).abs() < 1e-9);
        animation_end(&mut g, id);
        assert!(g.world.entity(id).is_none());
    }

    #[test]
    fn collect_expiry_belongs_to_alarm_not_age_or_animation_end() {
        let mut g = room();
        let id = g.world.spawn("oItemsGet", Vec2::new(80.0, 80.0));
        let e = g.world.entity_mut(id).unwrap();
        e.age = 500;
        e.animation = 1.9;
        step_instance(&mut g, id);
        animation_end(&mut g, id);
        assert!(g.world.entity(id).is_some());
        alarm(&mut g, id, 0);
        assert!(g.world.entity(id).is_none());
    }

    #[test]
    fn random_effect_create_precedes_overrides_and_does_not_repeat_in_step() {
        let mut g = room();
        let mut expected = g.rng.clone();
        expected.range(3);
        let id = spawn_effect(&mut g, "oBloodSpark", Vec2::new(80.0, 80.0));
        g.world.entity_mut(id).unwrap().vel.y = 2.0;
        step_instance(&mut g, id);
        assert_eq!(g.world.entity(id).unwrap().pos.y, 82.0);
        assert_eq!(g.rng.next_u64(), expected.next_u64());
    }

    #[test]
    fn fixed_create_is_idempotent_after_caller_overrides() {
        let mut g = room();
        let id = g.world.spawn("oFlareSpark", Vec2::new(80.0, 80.0));
        let e = g.world.entity_mut(id).unwrap();
        e.vel.y = 3.0;
        e.set_image_speed(0.0);
        e.alarms[0] = 7;
        init_entity(e);
        assert_eq!(e.vel.y, 3.0);
        assert_eq!(e.image_speed(), 0.0);
        assert_eq!(e.alarms[0], 7);
    }

    #[test]
    fn blood_alarm_flags_replace_age_approximations() {
        let mut g = room();
        let id = spawn_detritus(&mut g, "oBlood", Vec2::new(80.0, 80.0));
        assert_eq!(g.world.entity(id).unwrap().value("bounce"), 0.0);
        assert_eq!(g.world.entity(id).unwrap().value("collectible"), 0.0);
        alarm(&mut g, id, 1);
        assert_eq!(g.world.entity(id).unwrap().value("bounce"), 1.0);
        assert_eq!(g.world.entity(id).unwrap().value("invincible"), 0.0);
        alarm(&mut g, id, 2);
        assert_eq!(g.world.entity(id).unwrap().value("collectible"), 1.0);
        g.flags.insert("low_graphics".into());
        g.world.entity_mut(id).unwrap().alarms[0] = 0;
        alarm(&mut g, id, 0);
        assert_eq!(g.world.entity(id).unwrap().alarms[0], 0);
        assert!(!g.world.entities.iter().any(|e| e.object == "oBloodTrail"));
    }
    #[test]
    fn fluid_with_reloads_live_parent_array_after_activation_rebuild() {
        use crate::dispatcher::Instance;
        for activate_neighbor in [false, true] {
            let mut g = room();
            g.flags.insert("custom".into());
            for x in [5, 10] {
                g.world.set_tile(x - 1, 5, "oBrick");
                g.world.set_tile(x + 1, 5, "oBrick");
                g.world.set_tile(x, 6, "oBrick");
            }
            let lava = g.world.spawn("oLava", Vec2::new(80.0, 80.0));
            let water = g.world.spawn("oWaterSwim", Vec2::new(160.0, 80.0));
            let actor = g.world.spawn("oHintHand", Vec2::new(88.0, 88.0));
            g.world.entity_mut(actor).unwrap().engine_active = !activate_neighbor;
            // Creation registered Lava then Water. A prior Draw sorted the
            // global list by depth, leaving the descendant array unchanged.
            g.dispatcher.active_order = vec![
                Instance::Entity(water),
                Instance::Entity(lava),
                Instance::Player,
            ];
            g.dispatcher.active_order.extend(
                g.world
                    .tiles
                    .iter()
                    .enumerate()
                    .filter(|(_, tile)| !tile.object.is_empty())
                    .map(|(i, _)| Instance::Terrain(i)),
            );
            g.dispatcher.registration_order = vec![Instance::Entity(lava), Instance::Entity(water)];
            if activate_neighbor {
                g.dispatcher.inactive_order.push(Instance::Entity(actor));
            } else {
                g.dispatcher.active_order.push(Instance::Entity(actor));
            }
            g.world.fluid_dirty = true;
            fluid_tick(&mut g);
            assert_eq!(g.world.entity(lava).unwrap().sprite, "sLavaTop");
            assert_eq!(
                g.world.entity(water).unwrap().sprite,
                if activate_neighbor {
                    "sWater"
                } else {
                    "sWaterTop"
                }
            );
            assert_eq!(g.data["water_loop_safety"], 2.0);
            assert_eq!(g.data["water_counter"], 0.0);
        }
    }

    #[test]
    fn detritus_move_reciprocal_rounding_uses_gml_nearest_even_vectors() {
        // Expected values come from independent binary64 calculations, not a
        // call to another production movement helper.
        for (velocity, tick, expected) in [
            (0.4_f64, 2_u64, 1),
            (0.4, 3, 0),
            (-0.4, 2, -1),
            (-0.4, 3, 0),
            (2.0 / 9.0, 4, 1),
            (2.0 / 9.0, 5, 0),
            (-2.0 / 9.0, 4, -1),
            (2.0 / 7.0, 4, 1),
            (2.0 / 11.0, 6, 1),
            (1.4, 2, 1),
            (1.4, 3, 2),
            (2.4, 2, 2),
            (2.4, 3, 3),
            (1.5, 2, 2),
            (0.0, 2, 0),
        ] {
            assert_eq!(
                displacement(velocity, tick),
                expected,
                "velocity={velocity:?}, tick={tick}",
            );
        }
    }

    #[test]
    fn source_born_lava_flame_reaches_even_period_without_motion_overrides() {
        // scrSetupWalls enables top-lava spurts. Start at a legitimate elapsed
        // spurt timer, then let lava Step/Create and every particle Step run.
        // Canonical seed 10's six source-ordered raw draws are 1347758131,
        // 1409029376, 1187014913, 3891171590, 1117777183, 609983900.
        // rand(1,8) chooses Flame, then Create consumes three random calls;
        // grav=rand(1,6)*.1=.2 and the producer launch=-rand(1,4)=-1.
        // No particle velocity, gravity, timer or state is written by the test.
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            let mut game = room();
            game.rules = rules;
            game.rng = crate::Rng::new(10);
            let lava = configured_lava(&mut game, Vec2::new(80.0, 96.0), false);
            game.world
                .entity_mut(lava)
                .unwrap()
                .data
                .insert("spurt_counter".into(), 0.0);
            game.step(&InputFrame::default());
            let flame = game
                .world
                .entities
                .iter()
                .find(|e| e.object == "oFlame")
                .unwrap();
            let id = flame.id;
            assert_eq!(flame.pos, Vec2::new(88.0, 92.0));
            assert_eq!(flame.vel.y, -1.0);
            assert_eq!(flame.value("gravity"), 0.2);
            assert_eq!(flame.value("life"), 60.0);
            for y in [91.0, 90.0, 90.0, 89.0, 89.0, 89.0, 89.0] {
                game.step(&InputFrame::default());
                assert_eq!(game.world.entity(id).unwrap().pos.y, y);
            }
            let flame = game.world.entity(id).unwrap();
            assert_eq!(flame.vel.y.to_bits(), 0x3fd9_9999_9999_9999);
            assert_eq!(1.0 / flame.vel.y, 2.5);
            game.step(&InputFrame::default());
            assert_eq!(game.engine_time(), 10);
            assert_eq!(game.world.entity(id).unwrap().pos.y, 90.0);
            assert_eq!(game.world.entity(id).unwrap().value("life"), 52.0);
            assert_eq!(game.player.health, 4);
        }
    }
}
#[cfg(test)]
mod particle_cap_membership_source_tests {
    use super::*;
    use crate::RulesPreset;

    #[test]
    fn script_blood16_uses_active_family_before_each_birth_and_preserves_rng() {
        for (active, inactive) in [
            (0, 0),
            (15, 0),
            (16, 0),
            (0, 16),
            (15, 16),
            (16, 16),
            (0, 32),
        ] {
            for bloodless in [false, true] {
                let mut game = Game::shell(7, RulesPreset::Classic);
                let ids: Vec<_> = (0..active + inactive)
                    .map(|_| game.spawn("oBone", Vec2::new(560.0, 400.0)))
                    .collect();
                for &id in ids.iter().skip(active) {
                    game.world.entity_mut(id).unwrap().engine_active = false;
                }
                let mut receiver = Entity::new(0, "oSnake", Vec2::new(80.0, 80.0));
                receiver
                    .data
                    .insert("bloodless".into(), f64::from(bloodless));
                let count = if bloodless {
                    0
                } else {
                    3.min(16usize.saturating_sub(active))
                };
                let mut oracle = game.rng.clone();
                let expected: Vec<_> = (0..count)
                    .map(|_| {
                        let velocity = Vec2::new(
                            oracle.unit() * 4.0 - oracle.unit() * 4.0,
                            -1.0 - oracle.unit() * 2.0,
                        );
                        let gravity = f64::from(1 + oracle.range(6)) * 0.1;
                        (velocity, gravity)
                    })
                    .collect();
                crate::enemies::create_blood(&mut game, &receiver, Vec2::new(88.0, 88.0), 3);
                let actual: Vec<_> = game
                    .world
                    .entities
                    .iter()
                    .filter(|e| e.object == "oBlood")
                    .collect();
                assert_eq!(
                    actual.len(),
                    expected.len(),
                    "active={active} inactive={inactive} bloodless={bloodless}"
                );
                for (blood, (velocity, gravity)) in actual.iter().zip(expected) {
                    assert!(blood.alive && blood.engine_active);
                    assert_eq!(blood.pos, Vec2::new(88.0, 88.0));
                    assert_eq!(blood.vel, velocity);
                    assert_eq!(blood.value("gravity"), gravity);
                }
                assert_eq!(game.rng.next_u64(), oracle.next_u64());
            }
        }
    }
}

#[cfg(test)]
mod grave_step_tests {
    use super::*;
    use crate::{Area, Rng, RulesPreset, World};

    fn room(seed: u32) -> Game {
        let mut game = Game::shell(seed, RulesPreset::Classic);
        game.world = World::new(42, 34, Area::Jungle, 5);
        game.player.pos = Vec2::new(40.0, 40.0);
        game
    }

    // Literal Grave Destroy statements. Rubble and RubbleSmall inherit only
    // fixed DrawnSprite/RubblePiece Create fields, so each birth adds no rolls.
    // This verifies the port RNG's statement order, not GM8 seed equivalence.
    fn source_destroy(rng: &mut Rng, origin: Vec2) -> Vec<(&'static str, &'static str, Vec2)> {
        let mut expected = Vec::new();
        for (object, sprite) in [
            ("oRubble", "sRubbleTan"),
            ("oRubbleSmall", "sRubbleTanSmall"),
            ("oRubbleSmall", "sRubbleTanSmall"),
        ] {
            let x = origin.x + 8.0 + f64::from(rng.range(9)) - f64::from(rng.range(9));
            let y = origin.y + 8.0 + f64::from(rng.range(9)) - f64::from(rng.range(9));
            expected.push((object, sprite, Vec2::new(x, y)));
        }
        expected
    }

    fn assert_debris(game: &Game, expected: &[(&str, &str, Vec2)]) {
        let mut actual: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|e| e.alive && matches!(e.object.as_str(), "oRubble" | "oRubbleSmall"))
            .collect();
        actual.sort_by_key(|e| e.creation_serial);
        assert_eq!(actual.len(), expected.len());
        for (entity, &(object, sprite, pos)) in actual.iter().zip(expected) {
            assert_eq!(entity.object, object);
            assert_eq!(entity.sprite, sprite);
            assert_eq!(entity.pos, pos);
            // Rubble525/526 already passed Normal Step before Grave743 births.
            assert_eq!(entity.vel, Vec2::default());
        }
    }

    #[test]
    fn actual_grave_step_unsupported_birth_runs_unguarded_destroy_rng_once() {
        for seed in 0..16 {
            for clean in [false, true] {
                let mut game = room(seed);
                let origin = Vec2::new(80.0, 64.0);
                let id = game.spawn("oGrave", origin);
                if clean {
                    game.flags.insert("cleanSolids".into());
                    game.data.insert("cleanSolids".into(), 1.0);
                    let grave = game.world.entity_mut(id).unwrap();
                    grave.data.insert("cleanDeath".into(), 1.0);
                    grave.data.insert("invincible".into(), 1.0);
                }
                let mut source = game.rng.clone();
                let expected = source_destroy(&mut source, origin);
                game.step(&InputFrame::default());
                assert!(
                    game.world.entity(id).is_none(),
                    "seed {seed}, clean {clean}"
                );
                assert_debris(&game, &expected);
                assert_eq!(game.rng.clone().next_u64(), source.clone().next_u64());
                game.step(&InputFrame::default());
                assert_eq!(game.rng.next_u64(), source.next_u64());
                assert_eq!(
                    game.world
                        .entities
                        .iter()
                        .filter(|e| matches!(e.object.as_str(), "oRubble" | "oRubbleSmall"))
                        .count(),
                    3
                );
            }
        }
    }

    #[test]
    fn actual_grave_step_falling_support_exposes_origin_during_normal_game_tick() {
        let mut game = room(7);
        let support = game.spawn("oPushBlock", Vec2::new(80.0, 80.0));
        let origin = Vec2::new(80.0, 64.0);
        let grave = game.spawn("oGrave", origin);
        assert!(point_family(&game, Vec2::new(80.0, 80.0), "oSolid"));
        let mut source = game.rng.clone();
        let expected = source_destroy(&mut source, origin);
        game.step(&InputFrame::default());
        assert!(game.world.entity(support).unwrap().pos.y > 80.0);
        assert!(game.world.entity(grave).is_none());
        assert_debris(&game, &expected);
        assert_eq!(game.rng.next_u64(), source.next_u64());
    }

    #[test]
    fn actual_grave_step_viewport_guard_has_strict_source_origin_boundaries() {
        for origin in [
            Vec2::new(-20.0, 80.0),
            Vec2::new(VIEW_WIDTH + 4.0, 80.0),
            Vec2::new(80.0, -20.0),
            Vec2::new(80.0, VIEW_HEIGHT + 4.0),
        ] {
            let mut game = room(11);
            let id = game.spawn("oGrave", origin);
            let mut source = game.rng.clone();
            game.step(&InputFrame::default());
            assert_eq!(game.camera, Vec2::default());
            assert!(crate::dispatcher::is_active(
                &game,
                crate::dispatcher::Instance::Entity(id)
            ));
            assert!(game.world.entity(id).is_some(), "boundary {origin:?}");
            assert_debris(&game, &[]);
            assert_eq!(game.rng.next_u64(), source.next_u64());
        }
        for origin in [
            Vec2::new(-19.0, 80.0),
            Vec2::new(VIEW_WIDTH + 3.0, 80.0),
            Vec2::new(80.0, -19.0),
            Vec2::new(80.0, VIEW_HEIGHT + 3.0),
        ] {
            let mut game = room(11);
            let id = game.spawn("oGrave", origin);
            let mut source = game.rng.clone();
            let expected = source_destroy(&mut source, origin);
            game.step(&InputFrame::default());
            assert!(game.world.entity(id).is_none(), "inside {origin:?}");
            assert_debris(&game, &expected);
            assert_eq!(game.rng.next_u64(), source.next_u64());
        }
    }

    #[test]
    fn actual_grave_step_support_uses_solid_family_at_origin_not_overlap_or_platform() {
        for (dynamic, support_object, offset, survives) in [
            (false, "oBlock", 0.0, true),
            (true, "oHardBlock", 0.0, true),
            (true, "oHardBlock", 1.0, false),
            (false, "oPlatform", 0.0, false),
        ] {
            let mut game = room(19);
            if dynamic {
                game.spawn(support_object, Vec2::new(80.0 + offset, 80.0));
            } else {
                game.world.set_tile(5, 5, support_object);
            }
            let origin = Vec2::new(80.0, 64.0);
            let id = game.spawn("oGrave", origin);
            let mut source = game.rng.clone();
            let expected = if survives {
                Vec::new()
            } else {
                source_destroy(&mut source, origin)
            };
            game.step(&InputFrame::default());
            assert_eq!(
                game.world.entity(id).is_some(),
                survives,
                "{support_object}, {offset}"
            );
            assert_debris(&game, &expected);
            assert_eq!(game.rng.next_u64(), source.next_u64());
        }
    }
}
