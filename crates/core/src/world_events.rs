//! Source world objects and visual effects that have no item/enemy/trap owner.
//! All animation rates below are frames per original 30 Hz simulation tick.
use crate::{Entity, Game, GameEvent, InputFrame, Rect, VIEW_HEIGHT, VIEW_WIDTH, Vec2, catalog};

/// oGame's global fluid pass runs before instance activation and object Steps.
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
    // Tiles have stable storage order; entity instances retain creation order.
    let mut fluids: Vec<(Option<u64>, Vec2, String)> = game
        .world
        .tiles
        .iter()
        .enumerate()
        .filter(|(_, tile)| catalog::is_a(&tile.object, "oWater"))
        .map(|(index, tile)| {
            (
                None,
                Vec2::new(
                    (index % game.world.width) as f64 * 16.0,
                    (index / game.world.width) as f64 * 16.0,
                ),
                tile.object.clone(),
            )
        })
        .collect();
    fluids.extend(
        game.world
            .entities
            .iter()
            .filter(|e| e.alive && catalog::is_a(&e.object, "oWater"))
            .map(|e| (Some(e.id), e.pos, e.object.clone())),
    );
    let mut drained = 0;
    let regions = crate::activation::Regions::for_game(game);
    let mut activated: Vec<Rect> = Vec::new();
    for (id, origin, object) in fluids.clone() {
        if !custom && origin.y >= 512.0 {
            continue;
        }
        if id.is_some_and(|id| game.world.entity(id).is_none()) {
            continue;
        }
        let mask = id
            .and_then(|id| game.world.entity(id).map(Entity::sprite_bounds))
            .unwrap_or_else(|| {
                game.world
                    .tile_bounds((origin.x / 16.0) as i32, (origin.y / 16.0) as i32)
            });
        if !regions.bounds_active(mask) && !activated.iter().any(|region| region.intersects(mask)) {
            continue;
        }
        activated.push(Rect::new(origin.x - 16.0, origin.y - 16.0, 48.0, 48.0));
        let above = Vec2::new(origin.x, origin.y - 16.0);
        let exposed = !point_family(game, above, "oSolid") && !point_family(game, above, "oWater");
        let neighbor_top =
            [-16.0, 16.0].into_iter().any(|dx| {
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
            });
        if exposed || neighbor_top {
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

pub fn tick(game: &mut Game, _input: &InputFrame) {
    if let Some(timer) = game.data.get_mut("message_timer") {
        *timer = (*timer - 1.0).max(0.0);
    }
    parachute(game);
    let count = game.world.entities.len();
    for index in 0..count {
        let mut e = game.world.entities[index].clone();
        if !e.alive {
            continue;
        }
        match e.object.as_str() {
            "oMsgSign" => sign(game, &mut e),
            "oLamp" | "oLampRed" | "oTikiTorch" => light(&mut e),
            "oIceBottom" => ice_bottom(game, &mut e),
            "oFrozenCaveman" => frozen_caveman(game, &mut e),
            "oBlood" | "oBone" => detritus(game, &mut e),
            "oFlame" => flame(game, &mut e),
            "oLava" => lava(game, &mut e, false),
            "oTerrainMetadata"
                if game
                    .world
                    .tile(
                        (e.pos.x / 16.0).floor() as i32,
                        (e.pos.y / 16.0).floor() as i32,
                    )
                    .is_some_and(|tile| tile.object == "oLava") =>
            {
                lava(game, &mut e, true)
            }
            "oPoof" | "oSmokePuff" | "oBloodSpark" | "oBloodTrail" | "oLavaDrip" | "oBubble"
            | "oSplash" | "oBurn" | "oLaserExplode" | "oFlameTrail" | "oYellowTrail"
            | "oLaserTrail" => effect(game, &mut e),
            "oBones" | "oFakeBones" | "oParaUsed" => fallen_decoration(game, &mut e),
            "oParachute" => {
                e.pos = Vec2::new(game.player.pos.x - 8.0, game.player.pos.y - 16.0);
                if game.world.entities.iter().any(|item| {
                    item.alive
                        && catalog::is_a(&item.object, "oItem")
                        && (item.vel.x > 0.0 || item.vel.y > 0.0)
                        && e.sprite_bounds().intersects(item.sprite_bounds())
                }) {
                    e.alive = false;
                    game.player.data.insert("parachute_open".into(), 0.0);
                    game.world.spawn("oParaUsed", e.pos);
                }
                if e.sprite == "sParaOpen" {
                    e.animation += 1.0;
                    if e.animation >= 5.0 {
                        e.sprite = "sParachute".into();
                        e.animation = 0.0;
                    }
                }
            }
            "oSmallCollect" | "oBigCollect" | "oHeart" | "oItemsGet" => collect_effect(&mut e),
            "oRubbleDark" => {
                e.pos.y += e.vel.y;
                e.vel.y += 0.6;
                if point_family(game, e.pos, "oBrick")
                    || point_family(game, e.pos, "oBlock")
                    || e.pos.x < 0.0
                    || e.pos.y < 0.0
                    || e.pos.x >= game.world.width as f64 * 16.0
                    || e.pos.y >= game.world.height as f64 * 16.0
                {
                    e.alive = false;
                }
            }
            _ if catalog::is_a(&e.object, "oRubblePiece") => rubble(game, &mut e),
            _ => {}
        }
        game.world.entities[index] = e;
    }
}

fn sign(game: &mut Game, e: &mut Entity) {
    let remaining = (e.value("message_cooldown") - 1.0).max(0.0);
    e.data.insert("message_cooldown".into(), remaining);
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
        e.data.insert("message_cooldown".into(), 200.0);
    }
}

fn light(e: &mut Entity) {
    let torch = e.object == "oTikiTorch";
    e.animation = (e.animation + 0.5) % if torch { 5.0 } else { 3.0 };
    e.data.insert(
        "light_radius".into(),
        if e.object == "oLampRed" {
            0.0
        } else if torch {
            32.0
        } else {
            96.0
        },
    );
    e.data.insert("light_offset_x".into(), 8.0);
    e.data.insert("light_offset_y".into(), 8.0);
    if e.object == "oLampRed" {
        e.data.insert("red_light_range".into(), 96.0);
    }
}

fn lava(game: &mut Game, e: &mut Entity, terrain: bool) {
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
    e.animation += 0.4;
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
    game.world.tiles_point(point, family)
        || game.world.entities.iter().any(|e| {
            e.alive && catalog::is_a(&e.object, family) && e.sprite_bounds().contains(point)
        })
}

fn ice_bottom(game: &mut Game, e: &mut Entity) {
    if !point_family(game, Vec2::new(e.pos.x + 8.0, e.pos.y - 1.0), "oSolid") {
        e.alive = false;
        return;
    }
    if !e.data.contains_key("drip_alarm") {
        e.data
            .insert("drip_alarm".into(), f64::from(20 + game.rng.range(381)));
    }
    let alarm = e.value("drip_alarm") - 1.0;
    if alarm <= 0.0 {
        game.world
            .spawn("oDrip", Vec2::new(e.pos.x + 8.0, e.pos.y + 4.0));
        e.data
            .insert("drip_alarm".into(), f64::from(20 + game.rng.range(381)));
    } else {
        e.data.insert("drip_alarm".into(), alarm);
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
    if e.value("world_effect_initialized") == 0.0 {
        match e.object.as_str() {
            "oSmokePuff" => e.vel.y = 0.1,
            "oBloodSpark" => e.vel.y = -f64::from(1 + game.rng.range(3)) * 0.2,
            "oBubble" => e.vel.y = -f64::from(1 + game.rng.range(3)) * 0.1,
            "oBurn" => e.vel.y = -0.1,
            "oLaserExplode" => game.sound("sndSmallExplode", e.pos),
            _ => {}
        }
        e.data.insert("world_effect_initialized".into(), 1.0);
    }
    let (speed, frames) = match e.object.as_str() {
        "oBloodSpark" => (0.5, 9.0),
        "oBloodTrail" => (0.8, 7.0),
        "oBubble" => (0.2, 9.0),
        "oPoof" => (0.4, 6.0),
        "oLavaDrip" => (0.4, 5.0),
        "oSplash" => (0.6, 6.0),
        "oBurn" => (0.4, 9.0),
        "oLaserExplode" => (0.8, 8.0),
        "oFlameTrail" => (0.4, 5.0),
        "oYellowTrail" => (1.0, 12.0),
        "oLaserTrail" => (0.8, 5.0),
        _ => (0.4, 8.0),
    };
    match e.object.as_str() {
        "oPoof" => {
            e.pos.x += e.vel.x;
            e.pos.y += e.vel.y;
        }
        "oSmokePuff" => e.pos.y -= e.vel.y,
        "oBloodSpark" | "oBubble" | "oBurn" => e.pos.y += e.vel.y,
        _ => {}
    }
    e.animation += speed;
    if e.animation + 1e-9 >= frames
        || (e.object == "oBubble" && !point_family(game, e.pos, "oWater"))
        || (e.object == "oBurn" && point_family(game, e.pos, "oSolid"))
    {
        e.alive = false;
    }
}

fn collect_effect(e: &mut Entity) {
    let items = e.object == "oItemsGet";
    if items {
        e.pos.y = (e.pos.y - 0.1).ceil();
        e.pos.x = e.pos.x.ceil();
        e.animation += 0.8;
    } else {
        e.pos.y -= 1.0;
        if e.object == "oSmallCollect" {
            e.pos.x = e.pos.x.ceil();
            e.pos.y = e.pos.y.ceil();
            e.animation += 0.4;
        }
    }
    if e.age >= if items { 40 } else { 30 } {
        e.alive = false;
    }
}

fn flame(game: &mut Game, e: &mut Entity) {
    let alarm_pos = e.pos;
    // Flame inherits all oDetritus movement, bounce, life and instance-cap
    // behavior, but not oBone's switch to a dying smoke sprite.
    detritus(game, e);
    if e.alive {
        if e.age > 0
            && e.age.is_multiple_of(2)
            && game
                .world
                .entities
                .iter()
                .filter(|other| other.alive && other.object == "oFlameTrail")
                .count()
                < 12
        {
            game.world.spawn("oFlameTrail", alarm_pos);
        }
        e.animation += 0.3;
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
        let mask = e.sprite_bounds();
        if game.world.tiles_collide(mask, "oWater")
            || game.world.entities.iter().any(|water| {
                water.alive
                    && catalog::is_a(&water.object, "oWater")
                    && water.sprite_bounds().intersects(mask)
            })
        {
            // The source Collision event creates one puff, then its Destroy
            // event creates another. Both are intentional.
            game.world.spawn("oSmokePuff", e.pos);
            e.alive = false;
        }
    }
    if !e.alive {
        game.world.spawn("oSmokePuff", e.pos);
    }
}

fn outside(game: &Game, p: Vec2, margin: f64) -> bool {
    p.x < game.camera.x - margin
        || p.x > game.camera.x + VIEW_WIDTH + margin
        || p.y < game.camera.y - margin
        || p.y > game.camera.y + VIEW_HEIGHT + margin
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
    e.vel.y += if e.object == "oLeaf" { 0.01 } else { 0.6 };
    if e.object == "oLeaf" {
        e.animation = (e.animation + 0.2) % 10.0;
    }
    if point_family(game, e.pos, "oWaterSwim") {
        if e.object == "oDrip" {
            e.alive = false;
        } else if e.object == "oLeaf" {
            e.vel.y = 0.0;
            e.sprite = "sLeafStill".into();
        } else {
            e.vel.y = 0.2;
        }
    } else if point_family(game, e.pos, "oLava") {
        e.alive = false;
    }
    if point_family(game, e.pos, "oSolid") || outside(game, e.pos, 32.0) {
        e.alive = false;
    }
}

// moveTo uses reciprocal/modulo displacement, rather than accumulating fractions.
fn displacement(v: f64, tick: u64) -> i32 {
    let fraction = v.abs().fract();
    let extra = fraction != 0.0 && tick.is_multiple_of((1.0 / fraction).round().max(1.0) as u64);
    (v.abs().floor() as i32 + i32::from(extra)) * if v < 0.0 { -1 } else { 1 }
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
            .filter(|other| other.alive && catalog::is_a(&other.object, "oDetritus"))
            .count()
            >= 32
        {
            e.alive = false;
            return;
        }
    }
}

pub(crate) fn detritus(game: &mut Game, e: &mut Entity) {
    let alarm_pos = e.pos;
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
    if e.object == "oBlood" {
        e.data.insert("collectible".into(), f64::from(e.age >= 5));
        if e.vel.y > 6.0 {
            e.alive = false;
        }
        if bottom && life > 20.0 {
            e.data.insert("life".into(), 20.0);
        }
        if e.age % 4 == 1
            && game
                .world
                .entities
                .iter()
                .filter(|other| other.alive && other.object == "oBloodTrail")
                .count()
                < 12
        {
            game.world.spawn("oBloodTrail", alarm_pos);
        }
        if e.age >= 5
            && game.player.has("Kapala")
            && e.sprite_bounds().intersects(game.player.sprite_bounds())
        {
            let blood = game.data.get("bloodLevel").copied().unwrap_or(0.0) + 1.0;
            game.data
                .insert("bloodLevel".into(), if blood > 8.0 { 0.0 } else { blood });
            game.world.spawn("oBloodSpark", e.pos);
            if blood > 8.0 {
                game.player.health += 1;
                game.world.spawn(
                    "oHeart",
                    Vec2::new(game.player.pos.x, game.player.pos.y - 8.0),
                );
                game.sound("sndKiss", game.player.pos);
            }
            e.alive = false;
        }
        e.animation = (e.animation + 0.3) % 3.0;
    } else if e.object == "oBone" {
        if bottom && e.value("dying") == 0.0 {
            e.sprite = "sSmokePuff".into();
            e.animation = 0.0;
            e.data.insert("dying".into(), 1.0);
        }
        e.animation += 0.3;
        if e.value("dying") != 0.0 && e.animation + 1e-9 >= 8.0 {
            e.alive = false;
        } else if e.value("dying") == 0.0 {
            e.animation %= 8.0;
        }
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
            e.sprite = "sSkeletonCreateL".into();
        }
        if e.sprite == "sSkeletonCreateL" {
            e.animation += 1.0;
            if e.animation >= 6.0 {
                game.world.spawn("oSkeleton", e.pos);
                e.alive = false;
            }
        }
    }
}

fn parachute(game: &mut Game) {
    let open = game
        .player
        .data
        .get("parachute_open")
        .copied()
        .unwrap_or(0.0)
        != 0.0
        && game.player.health > 0
        && game.player.stun == 0;
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
        tick(g, &InputFrame::default());
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
                    "oLampRed" => 0.0,
                    _ => 96.0,
                }
            );
        }
        assert_eq!(g.world.entities[1].value("red_light_range"), 96.0);
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
                g.world.spawn("oMattockHit", Vec2::new(80.0, 96.0));
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
    fn fluid_pass_activates_neighbor_chain_but_preserves_isolated_offscreen_cell() {
        let mut g = room();
        g.world = World::new(80, 20, Area::Jungle, 5);
        g.flags.insert("custom".into());
        for x in 0..80 {
            g.world.set_tile(x, 3, "oBrick");
        }
        let chain: Vec<_> = [400.0, 416.0, 432.0, 448.0]
            .into_iter()
            .map(|x| g.world.spawn("oWaterSwim", Vec2::new(x, 80.0)))
            .collect();
        let isolated = g.world.spawn("oWaterSwim", Vec2::new(600.0, 80.0));
        g.world.fluid_dirty = true;
        g.step(&InputFrame::default());
        assert!(chain.iter().all(|id| g.world.entity(*id).is_none()));
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
            g.step(&InputFrame::default());
            let particle = g
                .world
                .entities
                .iter()
                .find(|e| matches!(e.object.as_str(), "oFlame" | "oMagma"))
                .unwrap();
            seen.insert(particle.object.clone());
            assert!((-4.0..=-1.0).contains(&particle.vel.y));
            assert_eq!(particle.value("life"), 60.0);
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
            ("oSplash", 0.6, 10),
            ("oBurn", 0.4, 23),
            ("oLaserExplode", 0.8, 10),
            ("oFlameTrail", 0.4, 13),
            ("oYellowTrail", 1.0, 12),
            ("oLaserTrail", 0.8, 7),
        ] {
            let mut g = room();
            for x in 0..20 {
                g.world.set_tile(x, 3, "oBrick");
            }
            let id = g.world.spawn(object, Vec2::new(80.0, 80.0));
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
                assert_eq!(sounds, usize::from(object == "oLaserExplode" && step == 1));
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
        for _ in 1..10 {
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
            !g.player
                .bounds()
                .intersects(g.world.entity(sign).unwrap().sprite_bounds())
        );
        assert!(
            g.player
                .sprite_bounds()
                .intersects(g.world.entity(sign).unwrap().sprite_bounds())
        );
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
        g.tick = 100;
        let id = g.world.spawn("oBone", Vec2::new(80.0, 80.0));
        let e = g.world.entity_mut(id).unwrap();
        e.data.insert("world_effect_initialized".into(), 1.0);
        e.data.insert("life".into(), 60.0);
        e.data.insert("gravity".into(), 0.0);
        e.vel = Vec2::new(0.4, 0.0);
        advance(&mut g); // source time 2, lifetime tick 101: no displacement
        assert_eq!(g.world.entity(id).unwrap().pos.x, 80.0);
        advance(&mut g); // source time 3, lifetime tick 102: one pixel
        assert_eq!(g.world.entity(id).unwrap().pos.x, 81.0);
        g.level_tick = 0;
        advance(&mut g); // another room restarts the phase at source time 2
        assert_eq!(g.world.entity(id).unwrap().pos.x, 81.0);
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
}
