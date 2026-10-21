//! Classic hazards translated from object event code. Static hazard positions are
//! GameMaker instance origins (usually tile top-left), not actor centers.
use crate::dispatcher::Instance;
#[cfg(test)]
use crate::model::PlayerState;
use crate::model::{CollisionPrecision, Entity, Game, GameEvent, Rect, Vec2};

#[path = "traps/altar.rs"]
pub(crate) mod altar;
#[path = "traps/curses.rs"]
mod curses;
#[path = "traps/detectors.rs"]
mod detectors;
#[path = "traps/moving.rs"]
mod moving;

#[cfg(test)]
#[path = "traps/player_active_query_contract_tests.rs"]
mod player_active_query_contract_tests;
#[cfg(test)]
#[path = "traps/player_health_prelude_contract_tests.rs"]
mod player_health_prelude_contract_tests;

/// Global platform movement runs before player input, as in oGame Step.
pub fn pre_tick(game: &mut Game) {
    moving::pre_tick(game);
}

#[cfg(test)]
pub fn tick(game: &mut Game) {
    environment(game);
    detectors(game);
    idol_traps(game);
    moving::tick(game);
    altar::tick(game);
    curses::tick(game);
}

pub(crate) fn handles(object: &str) -> bool {
    detectors::handles(object)
        || moving::handles(object)
        || curses::handles(object)
        || matches!(
            object,
            "oWeb"
                | "oSpikes"
                | "oGiantTikiHead"
                | "oKaliHead"
                | "oTrapBlock"
                | "oSacAltarLeft"
                | "oSacAltarRight"
        )
}

pub(crate) fn init_entity(entity: &mut Entity) {
    if !handles(&entity.object) || entity.value("traps_fixed_initialized") != 0. {
        return;
    }
    detectors::init_entity(entity);
    moving::init_entity(entity);
    curses::init_entity(entity);
    if entity.object == "oWeb" {
        entity.data.entry("life".into()).or_insert(12.);
        entity.data.entry("dying".into()).or_insert(0.);
    }
    if entity.object == "oTrapBlock" {
        entity.data.entry("dying".into()).or_insert(0.);
        entity.data.entry("deathTimer".into()).or_insert(0.);
    }
    if entity.object == "oSpikes" {
        // oSpikes inherits oDrawnSprite Create, then explicitly sets
        // invincible=false in its own Create event.
        entity.text.insert("type".into(), String::new());
        entity.data.insert("blinkToggle".into(), 0.);
        entity.data.insert("invincible".into(), 0.);
    }
    entity.data.insert("traps_fixed_initialized".into(), 1.);
}

/// oKaliHead Create runs at birth, before subsequent generation RNG draws.
pub(crate) fn initialize_kali_head(world: &mut crate::World, rng: &mut crate::Rng, id: u64) {
    let Some(entity) = world.entity_mut(id) else {
        return;
    };
    if entity.object != "oKaliHead" || entity.value("kali_create_initialized") != 0. {
        return;
    }
    entity.set_sprite(&format!("sKaliHead{}", rng.range(3) + 1));
    entity.data.insert("kali_create_initialized".into(), 1.);
}

/// World/RNG-only trap Create bodies also run at generated instance birth.
pub(crate) fn create_local(
    context: &mut crate::creation::EntityCreateContext<'_>,
    entity: &mut Entity,
) {
    moving::create_local(context.rng, entity, context.world.name == "City of Gold");
}

pub(crate) fn initialize_source_jungle_spear(entity: &mut Entity) {
    detectors::initialize_source_jungle_spear(entity);
}

pub(crate) fn create(game: &mut Game, instance: Instance) {
    if !handles(instance.object(game)) {
        return;
    }
    match instance {
        Instance::Entity(id) => {
            let Some(entity) = game.world.entity_mut(id) else {
                return;
            };
            init_entity(entity);
            if game.generation_revision == crate::GenerationRevision::SourceClassicV2 {
                initialize_source_jungle_spear(entity);
            }
            if entity.value("traps_create_initialized") != 0. {
                return;
            }
            entity.data.insert("traps_create_initialized".into(), 1.);
        }
        Instance::Terrain(index) => {
            if game
                .data
                .insert(format!("traps_create_terrain:{index}"), 1.)
                .is_some()
            {
                return;
            }
        }
        Instance::Player => return,
    }
    detectors::create(game, instance);
    moving::create(game, instance);
    curses::create_instance(game, instance);
    if let Instance::Entity(id) = instance {
        initialize_kali_head(&mut game.world, &mut game.rng, id);
    }
}

pub(crate) fn step_instance(game: &mut Game, instance: Instance) {
    let object = instance.object(game).to_owned();
    if detectors::handles(&object) {
        detectors::step_instance(game, instance);
    } else if moving::handles(&object) {
        moving::step_instance(game, instance);
    } else if curses::handles(&object) {
        curses::step_instance(game, instance);
    } else if object == "oWeb" {
        web_step(game, instance);
    } else if object == "oTrapBlock" {
        trap_block_step(game, instance);
    } else if matches!(object.as_str(), "oSacAltarLeft" | "oSacAltarRight") {
        altar::step_instance(game, instance);
    }
}

pub(crate) fn builtin_motion(game: &mut Game, instance: Instance) {
    moving::builtin_motion(game, instance);
}

pub(crate) fn outside_room(game: &mut Game, instance: Instance) {
    moving::outside_room(game, instance);
}

pub(crate) fn collision(game: &mut Game, this: Instance, other: Instance) {
    let object = this.object(game).to_owned();
    if detectors::handles(&object) {
        detectors::collision(game, this, other);
    } else if moving::handles(&object) {
        moving::collision(game, this, other);
    } else if object == "oWeb" {
        web_collision(game, this, other);
    }
}

pub(crate) fn animation_end(game: &mut Game, instance: Instance) {
    let object = instance.object(game).to_owned();
    if detectors::handles(&object) {
        detectors::animation_end(game, instance);
    } else if moving::handles(&object) {
        moving::animation_end(game, instance);
    }
}

pub(crate) fn alarm(game: &mut Game, instance: Instance, index: u8) {
    let object = instance.object(game).to_owned();
    if matches!(object.as_str(), "oGiantTikiHead" | "oKaliHead") && index == 0 {
        let pos = instance.position(game);
        match instance {
            Instance::Entity(id) => {
                if let Some(entity) = game.world.entity_mut(id) {
                    entity.sprite = "sGTHHole".into();
                }
            }
            Instance::Terrain(_index) => {
                let center = Vec2::new(pos.x + 8., pos.y + 8.);
                let id = game
                    .world
                    .entities
                    .iter()
                    .find(|e| e.alive && e.object == "oTerrainMetadata" && e.pos == center)
                    .map(|e| e.id)
                    .unwrap_or_else(|| game.spawn("oTerrainMetadata", center));
                game.world
                    .entity_mut(id)
                    .unwrap()
                    .text
                    .insert("terrain_sprite".into(), "sGTHHole".into());
            }
            _ => {}
        }
        if object == "oGiantTikiHead" {
            game.spawn("oBoulder", pos);
        } else {
            for _ in 0..6 {
                let id = game.spawn("oSpider", pos);
                let velocity = Vec2::new(
                    f64::from(game.rng.range(4)) - f64::from(game.rng.range(4)),
                    -(1. + f64::from(game.rng.range(3))),
                );
                game.world.entity_mut(id).unwrap().vel = velocity;
            }
        }
        game.sound("sndThump", pos);
    } else {
        detectors::alarm(game, instance, index);
    }
}

pub(crate) fn initialize_level(game: &mut Game) {
    if game.level_tick == 0 {
        curses::initialize_level(game);
    }
}

pub(crate) fn player_crush(game: &mut Game) {
    moving::player_crush(game);
}
pub(crate) fn player_spears(game: &mut Game) {
    detectors::player_hurt(game);
}
pub(crate) fn player_moving_hurt(game: &mut Game) {
    moving::player_hurt(game);
}

fn web_step(game: &mut Game, instance: Instance) {
    let Instance::Entity(id) = instance else {
        return;
    };
    if let Some(entity) = game.world.entity_mut(id) {
        let life = entity.data.get("life").copied().unwrap_or(12.);
        entity.data.insert("image_alpha".into(), life / 12.);
        let next = life
            - if entity.value("dying") != 0. {
                0.02
            } else {
                0.
            };
        entity.data.insert("life".into(), next);
        if next <= 1. {
            entity.alive = false;
        }
    }
}

fn web_collision(game: &mut Game, this: Instance, other: Instance) {
    let Instance::Entity(web) = this else {
        return;
    };
    let object = other.object(game).to_owned();
    if crate::catalog::is_a(&object, "oWater")
        || matches!(object.as_str(), "oLava" | "oLaser" | "oSlash")
    {
        if let Some(entity) = game.world.entity_mut(web) {
            entity.alive = false;
        }
    }
    let Instance::Entity(id) = other else {
        return;
    };
    let Some(entity) = game.world.entity_mut(id) else {
        return;
    };
    let enemy = crate::catalog::is_a(&object, "oEnemy");
    let item = crate::catalog::is_a(&object, "oItem");
    let rubble = crate::catalog::is_a(&object, "oRubblePiece");
    let source_type = crate::items::explosives::source_type(entity);
    if enemy && !matches!(source_type.as_str(), "Spider" | "Giant Spider")
        || item && !entity.held && source_type != "Rope"
        || rubble
        || crate::catalog::is_a(&object, "oTreasure")
    {
        entity.vel = Vec2::default();
    }
    if enemy && source_type == "Shopkeeper" && entity.health > 0 {
        entity.state = 2;
    }
    if rubble {
        entity.data.insert("animation_speed".into(), 0.);
    }
    if enemy && source_type == "Magma Man" {
        if let Some(entity) = game.world.entity_mut(web) {
            entity.alive = false;
        }
    }
}

fn trap_block_step(game: &mut Game, instance: Instance) {
    let Instance::Entity(id) = instance else {
        return;
    };
    let Some(entity) = game.world.entity_mut(id) else {
        return;
    };
    if entity.value("dying") == 0. {
        return;
    }
    let timer = entity
        .data
        .get("death_timer")
        .copied()
        .unwrap_or_else(|| entity.value("deathTimer"));
    if timer > 0. {
        entity.data.insert("deathTimer".into(), timer - 1.);
        entity.data.remove("death_timer");
    } else {
        entity.alive = false;
        crate::items::explosives::destroy_solid_self(game, id);
    }
}

#[derive(Clone)]
struct Hazard {
    id: Option<u64>,
    object: String,
    pos: Vec2,
    data_key: String,
}
fn hazards(game: &Game) -> Vec<Hazard> {
    let mut result = Vec::new();
    for (i, tile) in game.world.tiles.iter().enumerate() {
        if is_hazard(&tile.object) {
            result.push(Hazard {
                id: None,
                object: tile.object.clone(),
                pos: Vec2::new(
                    (i % game.world.width) as f64 * 16.0,
                    (i / game.world.width) as f64 * 16.0,
                ),
                data_key: format!("hazard:{i}"),
            });
        }
    }
    for e in game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && is_hazard(&e.object))
    {
        result.push(Hazard {
            id: Some(e.id),
            object: e.object.clone(),
            pos: e.pos,
            data_key: format!("hazard_entity:{}", e.id),
        });
    }
    result
}
/// Source oPlayer1 collision_rectangle calls traverse active instance
/// membership, in registration order. The broad hazard inventory also serves
/// non-dispatch queries and therefore deliberately includes inactive storage.
fn active_hazards(game: &Game, object: &str) -> Vec<Hazard> {
    crate::dispatcher::descendant_instances(game, object)
        .into_iter()
        .filter(|instance| crate::dispatcher::is_active(game, *instance))
        .filter_map(|instance| {
            let object = instance.object(game).to_owned();
            let (id, data_key) = match instance {
                Instance::Entity(id) => (Some(id), format!("hazard_entity:{id}")),
                Instance::Terrain(index) => (None, format!("hazard:{index}")),
                Instance::Player => return None,
            };
            Some(Hazard {
                id,
                object,
                pos: instance.position(game),
                data_key,
            })
        })
        .collect()
}
fn is_hazard(object: &str) -> bool {
    matches!(
        object,
        "oSpikes"
            | "oLava"
            | "oWater"
            | "oWaterSwim"
            | "oWeb"
            | "oArrowTrapLeft"
            | "oArrowTrapRight"
            | "oArrowTrapLeftLit"
            | "oArrowTrapRightLit"
            | "oArrowRepeaterL"
            | "oArrowRepeaterR"
            | "oSpearTrapTop"
            | "oSpearTrapBottom"
            | "oSpearTrapLit"
            | "oSpearsLeft"
    )
}
fn rect(h: &Hazard) -> Rect {
    crate::catalog::bounds(&h.object, h.pos)
}
fn current_rect(game: &Game, h: &Hazard) -> Rect {
    h.id.and_then(|id| game.world.entity(id))
        .map(Entity::sprite_bounds)
        .unwrap_or_else(|| rect(h))
}
fn lethal(game: &mut Game) {
    // The independent source dead flag is latched after the entire player Step.
    // Hazard damage must not eagerly replace the player's movement state.
    game.player.lose_health(99);
    game.events.push(GameEvent::Damage { amount: 99 });
}

/// oPlayer1 Step1552-1558, before the single outer own-hurt guard.
/// Visibility uses the existing inverse adapter so first-fatal Ankh revival
/// removes this hide in its later source action.
pub(crate) fn player_health_prelude(game: &mut Game) {
    if game.player.health < -10000.0 {
        game.player.health = -10000.0;
    }
    let visible = game.player.data.get("hidden").copied().unwrap_or(0.0) == 0.0
        && game.player.data.get("visible").copied().unwrap_or(1.0) != 0.0;
    if game.player.health < -99.0 && visible {
        crate::player::script_player_blood(game, 3);
        game.player.data.insert("hidden".into(), 1.0);
    }
}

pub(crate) fn player_hurt_enabled(game: &Game) -> bool {
    !game.player.cheat_invulnerable
        && game.player.health >= -99.0
        && game.player.data.get("hidden").copied().unwrap_or(0.) == 0.
        && game.player.data.get("visible").copied().unwrap_or(1.) != 0.
        && !game.player.is_exiting()
}

/// oPlayer1 Step 210: before active/controller and outside the exit hurt guard.
pub(crate) fn player_lava(game: &mut Game) {
    if game.player.cheat_invulnerable {
        return;
    }
    let pos = game.player.pos;
    let foot = Vec2::new(pos.x, pos.y + 6.0);
    let in_lava = crate::dispatcher::descendant_instances(game, "oLava")
        .into_iter()
        .filter(|instance| crate::dispatcher::is_active(game, *instance))
        .any(|instance| match instance {
            Instance::Entity(id) => game.world.entity(id).is_some_and(|entity| {
                entity.mask_contains(foot, crate::CollisionPrecision::BoundingBox)
            }),
            Instance::Terrain(index) => game
                .world
                .tile_collision_mask(
                    (index % game.world.width) as i32,
                    (index / game.world.width) as i32,
                )
                .contains(foot, crate::CollisionPrecision::BoundingBox),
            Instance::Player => false,
        });
    if in_lava {
        if game.state != crate::RunState::Dead {
            if crate::statistics::is_real_level(game) {
                game.lifetime_stats
                    .misc_death(crate::statistics::MiscDeath::Lava);
            }
            game.sound("sndFlame", pos);
        }
        lethal(game);
        game.player.vel = Vec2::new(0., 0.1);
        game.player.data.insert("burning".into(), 100.);
        game.player.data.insert("gravity".into(), 0.);
        game.player.data.insert("my_gravity".into(), 0.);
        game.player.data.insert("bounced".into(), 1.);
        game.player.data.insert("depth".into(), 999.);
    }
}

/// oPlayer1 Step 1752-1781: the last late-hurt family, after ceiling traps.
pub(crate) fn player_spikes(game: &mut Game) {
    // The dispatcher evaluates the source outer hurt guard once, before crush.
    // Its later visibility/health changes do not skip the rest of this block.
    crate::dispatcher::register_instances(game);
    let pos = game.player.pos;
    let spikes = active_hazards(game, "oSpikes")
        .into_iter()
        .find(|h| Rect::new(pos.x - 4., pos.y - 4., 9., 13.).intersects(current_rect(game, h)));
    if spikes.is_some() && game.state == crate::RunState::Dead {
        let point = Vec2::new(pos.x, pos.y + 9.0);
        let supported = game.world.point_family_active(point, "oSolid");
        if !supported {
            game.player.pos.y += 0.05;
        } else {
            game.player.data.insert("my_gravity".into(), 0.6);
        }
    } else {
        game.player.data.insert("my_gravity".into(), 0.6);
    }
    let Some(_) = spikes else {
        return;
    };
    let fall = game.player.data.get("fall_ticks").copied().unwrap_or(0.);
    if game.player.vel.y > 0. && (fall > 4. || game.player.is_stunned()) {
        if game.state != crate::RunState::Dead {
            if crate::statistics::is_real_level(game) {
                game.lifetime_stats
                    .misc_death(crate::statistics::MiscDeath::Spikes);
            }
            crate::player::script_player_blood(game, 3);
            lethal(game);
            game.player.vel = Vec2::default();
            game.player.data.insert("my_gravity".into(), 0.0);
        }
        crate::dispatcher::register_instances(game);
        let player_mask = game.player.collision_mask();
        let selected = crate::dispatcher::descendant_instances(game, "oSpikes")
            .into_iter()
            .find(|instance| {
                if !crate::dispatcher::is_active(game, *instance) {
                    return false;
                }
                let mask = match *instance {
                    Instance::Entity(id) => {
                        let Some(entity) = game.world.entity(id) else {
                            return false;
                        };
                        entity.collision_mask()
                    }
                    Instance::Terrain(index) => game.world.tile_collision_mask(
                        (index % game.world.width) as i32,
                        (index / game.world.width) as i32,
                    ),
                    Instance::Player => return false,
                };
                player_mask.intersects_mask(&mask)
            });
        match selected {
            Some(Instance::Entity(id)) => {
                game.world
                    .entity_mut(id)
                    .unwrap()
                    .set_sprite("sSpikesBlood");
            }
            Some(Instance::Terrain(index)) => {
                let origin = Vec2::new(
                    (index % game.world.width) as f64 * 16.0,
                    (index / game.world.width) as f64 * 16.0,
                );
                let metadata = game
                    .world
                    .entities
                    .iter()
                    .find(|entity| {
                        entity.alive
                            && entity.object == "oTerrainMetadata"
                            && (entity.pos.x / 16.0).floor() == (origin.x / 16.0).floor()
                            && (entity.pos.y / 16.0).floor() == (origin.y / 16.0).floor()
                    })
                    .map(|entity| entity.id)
                    .unwrap_or_else(|| {
                        game.spawn(
                            "oTerrainMetadata",
                            Vec2::new(origin.x + 8.0, origin.y + 8.0),
                        )
                    });
                game.world
                    .entity_mut(metadata)
                    .unwrap()
                    .text
                    .insert("terrain_sprite".into(), "sSpikesBlood".into());
            }
            _ => {}
        }
    }
}

#[cfg(test)]
fn environment(game: &mut Game) {
    player_lava(game);
    player_spikes(game);
    let all = hazards(game);
    // Web collision handlers freeze movable objects. Spider-family immunity and
    // magma melting are explicit exceptions in the Classic event source.
    let webs: Vec<_> = all.iter().filter(|h| h.object == "oWeb").cloned().collect();
    for web in &webs {
        let r = current_rect(game, web);
        let wet = all.iter().any(|h| {
            matches!(h.object.as_str(), "oWater" | "oWaterSwim" | "oLava")
                && current_rect(game, h).intersects(r)
        });
        let magma = game
            .world
            .entities
            .iter()
            .any(|e| e.alive && e.object == "oMagmaMan" && e.sprite_bounds().intersects(r));
        if wet || magma {
            remove_hazard(game, web);
            continue;
        }
        for e in &mut game.world.entities {
            let enemy = crate::catalog::is_a(&e.object, "oEnemy");
            let item = crate::catalog::is_a(&e.object, "oItem");
            let rubble = crate::catalog::is_a(&e.object, "oRubblePiece");
            if e.alive
                && (!item || !e.held)
                && e.sprite_bounds().intersects(r)
                && (enemy || item || rubble || crate::catalog::is_a(&e.object, "oTreasure"))
                && !matches!(
                    e.object.as_str(),
                    "oSpider" | "oGiantSpider" | "oRope" | "oRopeTop" | "oRopeSegment"
                )
            {
                e.vel = Vec2::default();
                if enemy && crate::catalog::is_a(&e.object, "oShopkeeper") && e.health > 0 {
                    e.state = 2;
                }
                if rubble {
                    e.data.insert("animation_speed".into(), 0.0);
                }
            }
        }
        if let Some(entity) = web.id.and_then(|id| game.world.entity_mut(id)) {
            // Both jumping and natural decay consume the same source life.
            let life = entity.data.get("life").copied().unwrap_or(12.0);
            entity.data.insert("image_alpha".into(), life / 12.0);
            let life = life
                - if entity.value("dying") != 0.0 {
                    0.02
                } else {
                    0.0
                };
            entity.data.insert("life".into(), life);
            if life <= 1.0 {
                entity.alive = false;
            }
        }
    }
}
fn remove_hazard(game: &mut Game, h: &Hazard) {
    if crate::catalog::is_a(&h.object, "oSpearTrapTop")
        || crate::catalog::is_a(&h.object, "oSpearTrapBottom")
    {
        // The SpearTrap Step's instance_destroy invokes its own non-inheriting
        // Destroy for either storage representation, bypassing attack immunity.
        if let Some(id) = h.id {
            crate::items::explosives::destroy_solid_self(game, id);
        } else {
            crate::items::explosives::destroy_terrain_self(
                game,
                (h.pos.x / 16.0) as usize,
                (h.pos.y / 16.0) as usize,
            );
        }
        return;
    }
    if let Some(id) = h.id {
        if let Some(e) = game.world.entity_mut(id) {
            e.alive = false;
        }
        crate::items::explosives::destroy_solid_instance(game, id);
    } else {
        game.world
            .set_tile((h.pos.x / 16.0) as usize, (h.pos.y / 16.0) as usize, "");
    }
}
#[cfg(test)]
fn movable(e: &Entity) -> bool {
    [
        "oCharacter",
        "oEnemy",
        "oItem",
        "oTreasure",
        "oMoveableSolid",
    ]
    .iter()
    .any(|parent| crate::catalog::is_a(&e.object, parent))
}

#[cfg(test)]
fn detector(game: &Game, h: &Hazard, right: bool) -> Rect {
    let y = h.pos.y + 8.0;
    if right {
        let mut x = h.pos.x + 16.0;
        while x - h.pos.x <= 96.0 && !game.world.solid_at(x, y) {
            x += 1.0;
        }
        let length = (x - (h.pos.x + 8.0)).max(32.0);
        Rect::new(
            h.pos.x + 16.0,
            h.pos.y,
            ((length - 16.0) / 16.0).ceil() * 16.0,
            16.0,
        )
    } else {
        let mut x = h.pos.x - 1.0;
        while h.pos.x - x <= 96.0 && !game.world.solid_at(x, y) {
            x -= 1.0;
        }
        x = x.min(h.pos.x - 16.0);
        Rect::new(x, h.pos.y, ((h.pos.x - 1.0 - x) / 16.0).ceil() * 16.0, 16.0)
    }
}

#[cfg(test)]
fn detector_occupied_bounds(area: Rect) -> Rect {
    detectors::detector_occupied_bounds(area)
}

#[cfg(test)]
fn detectors(game: &mut Game) {
    for h in hazards(game) {
        match h.object.as_str() {
            "oArrowTrapLeft" | "oArrowTrapRight" | "oArrowTrapLeftLit" | "oArrowTrapRightLit"
            | "oArrowRepeaterL" | "oArrowRepeaterR" => {
                let right = matches!(
                    h.object.as_str(),
                    "oArrowTrapRight" | "oArrowTrapRightLit" | "oArrowRepeaterR"
                );
                let repeat = h.object.starts_with("oArrowRepeater");
                let key = format!("{}:fired", h.data_key);
                let fired = game.data.get(&key).copied().unwrap_or(0.0);
                if repeat && !game.data.contains_key(&key) {
                    let delay = 100 + game.rng.range(100);
                    game.data.insert(key, f64::from(delay));
                    continue;
                }
                if fired > 0.0 {
                    if repeat {
                        game.data.insert(key, fired - 1.0);
                    }
                    continue;
                }
                let range_key = format!("{}:range", h.data_key);
                let area = if let Some(&x) = game.data.get(&range_key) {
                    Rect::new(
                        x,
                        h.pos.y,
                        game.data
                            .get(&format!("{}:range_width", h.data_key))
                            .copied()
                            .unwrap_or(16.0),
                        16.0,
                    )
                } else {
                    let area = detector(game, &h, right);
                    game.data.insert(range_key, area.x);
                    game.data
                        .insert(format!("{}:range_width", h.data_key), area.w);
                    area
                };
                let occupied = detector_occupied_bounds(area);
                let duck_to_hang = matches!(
                    game.player.sprite.as_str(),
                    "sDuckToHangL" | "sDamselDtHL" | "sTunnelDtHL"
                ) && game.player.animation > 6.0;
                let activated = repeat
                    || (game.player.vel.x != 0.0 || game.player.vel.y != 0.0 || duck_to_hang)
                        && game
                            .player
                            .mask_intersects(occupied, CollisionPrecision::Precise)
                    || game.world.entities.iter().any(|e| {
                        e.alive
                            && e.id != h.id.unwrap_or(0)
                            && (movable(e)
                                || matches!(e.object.as_str(), "oBoulder" | "oPushBlock"))
                            && (e.vel.x != 0.0 || e.vel.y != 0.0)
                            && e.mask_intersects(occupied, CollisionPrecision::Precise)
                    });
                if activated {
                    let pos = Vec2::new(h.pos.x + if right { 18.0 } else { -2.0 }, h.pos.y + 4.0);
                    let id = game.spawn("oArrow", pos);
                    if let Some(e) = game.world.entity_mut(id) {
                        e.vel.x = if right { 8.0 } else { -8.0 };
                        e.facing = if right { 1 } else { -1 };
                        e.data.insert("armed".into(), 1.0);
                        e.data.insert("owner".into(), -1.0);
                    }
                    let delay = if repeat {
                        100.0 + f64::from(game.rng.range(101))
                    } else {
                        1.0
                    };
                    game.data.insert(key, delay);
                    game.sound("sndArrowTrap", h.pos);
                }
            }
            "oSpearTrapTop" | "oSpearTrapBottom" | "oSpearTrapLit" => {
                let key = format!("{}:cooldown", h.data_key);
                let cooldown = game.data.get(&key).copied().unwrap_or(0.0);
                if cooldown > 1.0 {
                    game.data.insert(key.clone(), cooldown - 1.0);
                }
                if cooldown <= 1.0
                    && let Some(right) = spear_target(game, &h)
                {
                    let id = game.spawn(
                        "oSpearsLeft",
                        Vec2::new(h.pos.x + if right { 16.0 } else { -16.0 }, h.pos.y),
                    );
                    if let Some(e) = game.world.entity_mut(id) {
                        e.sprite = if right { "sSpearsRight" } else { "sSpearsLeft" }.into();
                        e.facing = if right { 1 } else { -1 };
                        e.animation = 0.0;
                    }
                    game.data.insert(key, 50.0);
                } else if cooldown <= 1.0 {
                    game.data.insert(key, 0.0);
                }
                let active = h.pos.x > game.camera.x - 8.0
                    && h.pos.x < game.camera.x + 328.0
                    && h.pos.y > game.camera.y - 8.0
                    && h.pos.y < game.camera.y + 248.0;
                if active
                    && !game.world.solid_point_active(
                        Vec2::new(h.pos.x, h.pos.y + 16.0),
                        CollisionPrecision::BoundingBox,
                        None,
                    )
                {
                    remove_hazard(game, &h);
                }
            }
            "oSpearsLeft" => {
                if let Some(id) = h.id {
                    let right = game
                        .world
                        .entity(id)
                        .is_some_and(|e| e.sprite == "sSpearsRight");
                    let anchor = Vec2::new(h.pos.x + if right { -16.0 } else { 16.0 }, h.pos.y);
                    let supported = hazards(game).iter().any(|trap| {
                        matches!(
                            trap.object.as_str(),
                            "oSpearTrapTop" | "oSpearTrapBottom" | "oSpearTrapLit"
                        ) && current_rect(game, trap).contains(anchor)
                    });
                    if !supported {
                        remove_hazard(game, &h);
                        continue;
                    }
                    let overlaps = current_rect(game, &h).intersects(Rect::new(
                        game.player.pos.x - 6.0,
                        game.player.pos.y - 6.0,
                        13.0,
                        14.0,
                    ));
                    let damaging = game
                        .world
                        .entity(id)
                        .is_some_and(|e| e.animation >= 20.0 && e.animation < 24.0 && overlaps);
                    if damaging && game.player.health > 0.0 && !game.player.is_exiting() {
                        // oPlayer1 Step intentionally bypasses both ordinary
                        // damage immunity and stun. Every active frame can hit.
                        game.player.lose_health(4);
                        game.events.push(GameEvent::Damage { amount: 4 });
                        let speed = 4.0 + f64::from(game.rng.range(3));
                        game.player.vel = Vec2::new(if right { speed } else { -speed }, -6.0);
                        game.player.pos.y -= 1.0;
                    }
                    if let Some(e) = game.world.entity_mut(id) {
                        e.animation += 1.0;
                        if e.animation >= 32.0 {
                            e.alive = false;
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

/// The source tests the nearest member of each family, first on the left,
/// then on the right. Actor and item origins use different vertical probes.
#[cfg(test)]
fn spear_target(game: &Game, trap: &Hazard) -> Option<bool> {
    let center = Vec2::new(trap.pos.x + 8.0, trap.pos.y + 8.0);
    let nearest = |family: &str| {
        game.world
            .entities
            .iter()
            .filter(|e| e.alive && crate::catalog::is_a(&e.object, family))
            .min_by(|a, b| {
                a.pos
                    .distance(trap.pos)
                    .total_cmp(&b.pos.distance(trap.pos))
            })
    };
    for right in [false, true] {
        let on_side = |x: f64, boundary: f64| if right { x > boundary } else { x < boundary };
        if (game.player.pos.y - center.y).abs() < 4.0
            && game.player.pos.distance(center) < 64.0
            && on_side(game.player.pos.x, if right { center.x } else { trap.pos.x })
        {
            return Some(right);
        }
        for family in ["oEnemy", "oMoveableSolid", "oItem"] {
            if let Some(entity) = nearest(family) {
                let origin = if family == "oItem" { center } else { trap.pos };
                let boundary = if family == "oItem" {
                    center.x
                } else {
                    trap.pos.x
                };
                if (entity.pos.y - origin.y).abs() < 4.0
                    && entity.pos.distance(origin) < 64.0
                    && on_side(entity.pos.x, boundary)
                {
                    return Some(right);
                }
            }
        }
    }
    None
}
#[cfg(test)]
fn idol_traps(game: &mut Game) {
    let ids: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && matches!(e.object.as_str(), "oGiantTikiHead" | "oTrapBlock"))
        .map(|e| e.id)
        .collect();
    for id in ids {
        let mut spawn = None;
        let mut destroyed = None;
        if let Some(e) = game.world.entity_mut(id) {
            if e.object == "oGiantTikiHead" {
                let alarm = e.value("idol_alarm");
                if alarm > 0.0 {
                    e.data.insert("idol_alarm".into(), alarm - 1.0);
                    if alarm <= 1.0 {
                        e.sprite = "sGTHHole".into();
                        spawn = Some(e.pos);
                    }
                }
            } else if e.value("dying") != 0.0 {
                let delay = e.value("death_timer");
                if delay > 0.0 {
                    e.data.insert("death_timer".into(), delay - 1.0);
                } else {
                    e.alive = false;
                    destroyed = Some(e.pos);
                }
            }
        }
        if let Some(pos) = spawn {
            game.spawn("oBoulder", pos);
            game.sound("sndThump", pos);
        }
        if let Some(pos) = destroyed {
            game.sound("sndThump", pos);
            game.data.insert("screen_shake".into(), 10.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::RulesPreset;
    fn room() -> Game {
        let mut g = Game::shell(1, RulesPreset::Classic);
        g.player.pos = Vec2::new(40.0, 40.0);
        g
    }
    #[test]
    fn arrow_detector_requires_motion_and_fires_once() {
        let mut g = room();
        g.world.spawn("oArrowTrapLeft", Vec2::new(80.0, 32.0));
        tick(&mut g);
        assert!(!g.world.entities.iter().any(|e| e.object == "oArrow"));
        g.player.vel.x = 1.0;
        tick(&mut g);
        let arrows: Vec<_> = g
            .world
            .entities
            .iter()
            .filter(|e| e.object == "oArrow")
            .collect();
        assert_eq!(arrows.len(), 1);
        assert_eq!(arrows[0].vel.x, -8.0);
        tick(&mut g);
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| e.object == "oArrow")
                .count(),
            1
        );
    }
    #[test]
    fn wall_blocks_arrow_motion_detector() {
        let mut g = room();
        g.world.set_tile(3, 2, "oBlock");
        g.world.spawn("oArrowTrapLeft", Vec2::new(80.0, 32.0));
        g.player.vel.x = 1.0;
        tick(&mut g);
        assert!(!g.world.entities.iter().any(|e| e.object == "oArrow"));
    }
    #[test]
    fn scaled_arrow_detector_uses_source_opaque_rows_and_actor_pixels() {
        let mask = crate::catalog::instance_mask(
            "oArrowTrapTest",
            "sRed",
            Vec2::default(),
            0.0,
            Vec2::new(1.0, 1.0),
            0.0,
        );
        // The scaled detector is exactly a filled horizontal band; this is
        // source mask data, not a guess from its visible color or image size.
        for y in 0..16 {
            for x in 0..16 {
                assert_eq!(
                    mask.contains(
                        Vec2::new(f64::from(x), f64::from(y)),
                        CollisionPrecision::Precise
                    ),
                    (1..=14).contains(&y),
                    "source detector pixel ({x},{y})",
                );
            }
        }
        for scale in [1.0, 2.0, 5.0] {
            assert_eq!(
                detector_occupied_bounds(Rect::new(96.0, 80.0, 16.0 * scale, 16.0)),
                Rect::new(96.0, 81.0, 16.0 * scale, 14.0),
            );
        }
        for (trap, rock_x) in [("oArrowTrapLeft", 50.0), ("oArrowTrapRight", 104.0)] {
            for (rock_y, should_fire) in [(77.0, false), (78.0, true), (97.0, true), (98.0, false)]
            {
                let mut game = room();
                game.world = crate::World::new(20, 15, crate::Area::Mines, 1);
                game.player.pos = Vec2::new(24.0, 184.0);
                game.player.previous_pos = game.player.pos;
                for x in 0..20 {
                    game.world.set_tile(x, 12, "oBlock");
                }
                game.spawn(trap, Vec2::new(80.0, 80.0));
                let rock = game.spawn("oRock", Vec2::new(rock_x, rock_y));
                game.world.entity_mut(rock).unwrap().vel.x = 1.0;

                game.step(&crate::InputFrame::default());

                let rock = game.world.entity(rock).unwrap();
                assert_eq!(rock.pos, Vec2::new(rock_x + 1.0, rock_y));
                assert_eq!(
                    game.world.entities.iter().any(|e| e.object == "oArrow"),
                    should_fire,
                    "{trap}, rock at y={rock_y}",
                );
            }
        }
    }
    #[test]
    fn arrow_detector_ignores_moving_debris_but_accepts_items_and_moveable_solids() {
        // oArrowTrapTest declares collisions with Character, Enemy, Item,
        // Treasure, MoveableSolid, and Boulder. RubblePiece inherits only
        // DrawnSprite: its visible motion must not activate those callbacks.
        for (object, should_fire) in [
            ("oRubble", false),
            ("oRubbleSmall", false),
            ("oRock", true),
            ("oPushBlock", true),
        ] {
            let mut game = room();
            game.world = crate::World::new(20, 15, crate::Area::Mines, 1);
            game.player.pos = Vec2::new(40.0, 184.0);
            game.player.previous_pos = game.player.pos;
            game.player.state = PlayerState::Standing;
            game.player.grounded = true;
            for x in 0..20 {
                game.world.set_tile(x, 12, "oBlock");
            }
            game.spawn("oArrowTrapLeft", Vec2::new(80.0, 80.0));
            let moving = game.spawn(object, Vec2::new(50.0, 82.0));
            game.world.entity_mut(moving).unwrap().vel.x = 1.0;

            game.step(&crate::InputFrame::default());

            let entity = game.world.entity(moving).unwrap();
            assert_ne!(
                entity.pos,
                Vec2::new(50.0, 82.0),
                "{object} must actually move"
            );
            assert_eq!(
                game.world.entities.iter().any(|e| e.object == "oArrow"),
                should_fire,
                "source collision family: {object}",
            );
        }
    }
    #[test]
    fn spikes_require_long_descent_or_stun() {
        let mut g = room();
        g.world.spawn("oSpikes", Vec2::new(32.0, 32.0));
        g.player.vel.y = 1.0;
        g.player.data.insert("fall_ticks".into(), 4.0);
        tick(&mut g);
        assert_eq!(g.player.health, 4.0);
        g.player.data.insert("fall_ticks".into(), 5.0);
        tick(&mut g);
        assert!(g.player.health <= 0.0);
    }

    #[test]
    fn runtime_spikes_create_applies_drawn_sprite_inherited_state() {
        let mut game = room();
        let id = game.spawn("oSpikes", Vec2::new(256.0, 160.0));

        let spikes = game.world.entity(id).unwrap();
        assert_eq!(spikes.text.get("type").map(String::as_str), Some(""));
        assert_eq!(spikes.value("blinkToggle"), 0.0);
        assert_eq!(spikes.value("invincible"), 0.0);
        assert_eq!(spikes.value("traps_create_initialized"), 1.0);

        game.step(&crate::InputFrame::default());

        let spikes = game.world.entity(id).unwrap();
        assert_eq!(spikes.text.get("type").map(String::as_str), Some(""));
        assert_eq!(spikes.value("blinkToggle"), 0.0);
        assert_eq!(spikes.value("invincible"), 0.0);
    }

    #[test]
    fn actual_yeti_zero_timer_throw_is_impaled_on_its_first_descending_overlap() {
        // oPlayer1 Step: colSpikes && yVel > 0 && (fallTimer > 4 || stunned).
        // The Yeti sets the independent stunned flag without adding a timer.
        let mut level = crate::level::Level::default();
        level.set_tile(20, 7, '@').unwrap();
        for x in 0..40 {
            level.set_tile(x, 8, '1').unwrap();
        }
        let mut game = Game::shell(11, RulesPreset::Classic);
        game.load_custom(&level, "zero-timer-spikes", "");
        for _ in 0..8 {
            game.step(&crate::InputFrame::default());
        }
        assert!(game.player.grounded);
        assert_eq!(game.player.health, 4.0);
        let spikes = game.spawn("oSpikes", Vec2::new(288.0, 96.0));
        let pos = game.player.pos;
        let yeti = game.spawn("oYeti", Vec2::new(pos.x - 8.0, pos.y - 8.0));
        game.step(&crate::InputFrame::default());
        assert_eq!(game.player.stun, 0);
        assert!(
            game.player.is_stunned(),
            "the live Yeti must actually throw"
        );
        assert_eq!(game.player.health, 4.0);
        for _ in 0..12 {
            game.step(&crate::InputFrame::default());
            if game.state == crate::RunState::Dead {
                break;
            }
        }
        assert_eq!(game.state, crate::RunState::Dead);
        assert_eq!(game.player.stun, 0);
        assert!(game.player.data["fall_ticks"] <= 4.0);
        assert_eq!(game.world.entity(spikes).unwrap().sprite, "sSpikesBlood");
        assert!(
            game.world.entity(yeti).is_some(),
            "the hazard remains alive"
        );
    }
    #[test]
    fn actual_nonstunned_descent_survives_spikes_until_fall_exceeds_four_ticks() {
        let mut level = crate::level::Level::default();
        level.set_tile(20, 2, '@').unwrap();
        for x in 0..40 {
            level.set_tile(x, 28, '1').unwrap();
        }
        let mut game = Game::shell(11, RulesPreset::Classic);
        game.load_custom(&level, "ordinary-fall-spikes", "");
        let pos = game.player.pos;
        game.spawn("oSpikes", Vec2::new(pos.x - 8.0, pos.y + 8.0));
        let mut survived_four = false;
        for _ in 0..12 {
            game.step(&crate::InputFrame::default());
            let fall = game.player.data["fall_ticks"];
            if fall <= 4.0 {
                assert_eq!(game.state, crate::RunState::Playing);
                assert_eq!(game.player.health, 4.0);
                assert!(!game.player.is_stunned());
                survived_four |= fall == 4.0;
            } else {
                assert_eq!(fall, 5.0);
                assert_eq!(game.state, crate::RunState::Dead);
                assert!(survived_four);
                return;
            }
        }
        panic!("ordinary descent must reach the live spikes after four ticks");
    }
    #[test]
    fn lava_bypasses_damage_immunity() {
        let mut g = room();
        g.world.spawn("oLava", Vec2::new(32.0, 32.0));
        crate::dispatcher::register_instances(&mut g);
        g.player.invincible = 60;
        tick(&mut g);
        assert!(g.player.health <= 0.0);
    }
    #[test]
    fn lava_applies_the_source_ninety_nine_damage_once_per_character_step() {
        let mut game = room();
        game.spawn("oLava", Vec2::new(32.0, 32.0));
        game.spawn("oLava", Vec2::new(32.0, 32.0));
        // Source Player Step caps transient overhealing to 99 before hurt.
        // One point query then subtracts 99 despite overlapping lava and immunity.
        game.player.health = 200.0;
        game.player.invincible = 60;
        game.step(&crate::InputFrame::default());
        assert_eq!(game.player.health, 0.0);
        assert_eq!(game.state, crate::RunState::Dead);
        assert_eq!(game.player.invincible, 60);
        assert_eq!(
            game.events
                .iter()
                .filter(|event| matches!(event, GameEvent::Damage { amount: 99 }))
                .count(),
            1,
            "overlapping lava must produce exactly one damage action"
        );
    }
    #[test]
    fn web_freezes_rock_but_spares_spiders() {
        let mut g = room();
        g.world.spawn("oWeb", Vec2::new(32.0, 32.0));
        let rock = g.world.spawn("oRock", Vec2::new(40.0, 40.0));
        let spider = g.world.spawn("oSpider", Vec2::new(40.0, 40.0));
        g.world.entity_mut(rock).unwrap().vel.x = 2.0;
        g.world.entity_mut(spider).unwrap().vel.x = 2.0;
        tick(&mut g);
        assert_eq!(g.world.entity(rock).unwrap().vel.x, 0.0);
        assert_eq!(g.world.entity(spider).unwrap().vel.x, 2.0);
    }

    #[test]
    fn dying_web_shares_escape_life_and_fades_before_destroying() {
        let mut game = room();
        game.player.pos = Vec2::new(120.0, 120.0);
        let web = game.spawn("oWeb", Vec2::new(32.0, 32.0));
        let entity = game.world.entity_mut(web).unwrap();
        entity.data.insert("life".into(), 1.03);
        entity.data.insert("dying".into(), 1.0);
        game.step(&crate::InputFrame::default());
        let entity = game.world.entity(web).unwrap();
        assert!((entity.value("life") - 1.01).abs() < 0.00001);
        assert!((entity.value("image_alpha") - 1.03 / 12.0).abs() < 0.00001);
        game.step(&crate::InputFrame::default());
        assert!(game.world.entity(web).is_none());
    }

    #[test]
    fn web_does_not_freeze_inherited_stun_timers_or_weapon_collision_events() {
        let mut game = room();
        game.player.pos = Vec2::new(120.0, 40.0);
        game.world.set_tile(5, 6, "oHardBlock");
        game.spawn("oWeb", Vec2::new(80.0, 80.0));
        let id = game.spawn("oCaveman", Vec2::new(80.0, 80.0));
        let caveman = game.world.entity_mut(id).unwrap();
        crate::enemies::init_entity(caveman);
        caveman.state = 98;
        caveman.timer = 5;
        caveman.vel = Vec2::default();
        game.step(&crate::InputFrame::default());
        game.step(&crate::InputFrame::default());
        assert_eq!(game.world.entity(id).unwrap().timer, 3);
        assert_eq!(game.world.entity(id).unwrap().value("webbed"), 0.0);
        let bullet = game.spawn("oBullet", Vec2::new(87.0, 87.0));
        let bullet = game.world.entity_mut(bullet).unwrap();
        bullet.data.insert("owner".into(), 1.0);
        bullet.data.insert("damage".into(), 4.0);
        game.step(&crate::InputFrame::default());
        assert!(game.world.entity(id).is_none_or(|e| e.health <= 0));
    }

    #[test]
    fn living_shopkeeper_in_web_becomes_angry_without_global_theft() {
        let mut game = room();
        game.player.pos = Vec2::new(120.0, 40.0);
        game.world.set_tile(5, 6, "oHardBlock");
        game.spawn("oWeb", Vec2::new(80.0, 80.0));
        let id = game.spawn("oShopkeeper", Vec2::new(80.0, 80.0));
        game.step(&crate::InputFrame::default());
        let keeper = game.world.entity(id).unwrap();
        assert_eq!(keeper.state, 2);
        assert_eq!(keeper.vel, Vec2::default());
    }

    fn spear_room(right: bool, player_x: f64) -> (Game, u64) {
        let mut game = room();
        for x in 0..game.world.width {
            game.world.set_tile(x, 10, "oHardBlock");
        }
        let trap_x = if right { 64.0 } else { 96.0 };
        let trap = game.spawn("oSpearTrapTop", Vec2::new(trap_x, 144.0));
        game.data
            .insert(format!("hazard_entity:{trap}:cooldown"), 50.0);
        let spear = game.spawn("oSpearsLeft", Vec2::new(80.0, 144.0));
        let entity = game.world.entity_mut(spear).unwrap();
        entity.sprite = if right { "sSpearsRight" } else { "sSpearsLeft" }.into();
        entity.animation = 20.0;
        game.player.pos = Vec2::new(player_x, 152.0);
        game.player.state = PlayerState::Standing;
        game.player.grounded = true;
        game.player.health = 16.0;
        (game, spear)
    }

    #[test]
    fn active_spear_damage_bypasses_immunity_without_creating_stun() {
        let (mut game, spear) = spear_room(false, 90.0);
        game.player.invincible = 60;
        game.step(&crate::InputFrame::default());
        assert_eq!(game.player.health, 12.0);
        assert_eq!(game.player.stun, 0);
        assert_eq!(game.player.invincible, 59);
        assert_eq!(game.player.pos.y, 151.0);
        assert_eq!(game.player.vel.y, -6.0);
        assert!((-6.0..=-4.0).contains(&game.player.vel.x));
        assert_eq!(game.world.entity(spear).unwrap().animation, 21.0);
    }

    fn authored_exit_room(character: crate::characters::Character, exit: bool) -> Game {
        let mut level = crate::level::Level::default();
        level.set_tile(6, 7, '@').unwrap();
        level.set_tile(7, 7, 'X').unwrap();
        for x in 0..40 {
            level.set_tile(x, 8, '1').unwrap();
        }
        let mut game = Game::shell(11, RulesPreset::Classic);
        crate::characters::apply(&mut game, character);
        game.load_custom(&level, "exit-hazards", "");
        for _ in 0..8 {
            game.step(&crate::InputFrame::default());
        }
        if exit {
            for _ in 0..16 {
                game.step(&crate::InputFrame {
                    right: true,
                    up: true,
                    ..Default::default()
                });
                if game.state == crate::RunState::Ending {
                    break;
                }
            }
            assert_eq!(game.state, crate::RunState::Ending);
            assert!(game.player.is_exiting());
        }
        assert_eq!(game.player.health, 4.0);
        game
    }

    #[test]
    fn real_exit_sprites_ignore_active_spears_while_the_hazard_keeps_animating() {
        use crate::characters::Character;
        for character in [Character::Explorer, Character::Damsel, Character::TunnelMan] {
            for exit in [false, true] {
                let mut game = authored_exit_room(character, exit);
                let player_pos = game.player.pos;
                let spear_pos = Vec2::new(player_pos.x - 8.0, player_pos.y - 8.0);
                let trap = game.spawn("oSpearTrapTop", Vec2::new(spear_pos.x + 16.0, spear_pos.y));
                game.world
                    .entity_mut(trap)
                    .unwrap()
                    .data
                    .insert("fired".into(), 50.0);
                let spear = game.spawn("oSpearsLeft", spear_pos);
                game.world.entity_mut(spear).unwrap().animation = 20.0;
                game.step(&crate::InputFrame::default());
                assert_eq!(game.world.entity(spear).unwrap().animation, 21.0);
                assert_eq!(
                    game.data[&format!("hazard_entity:{trap}:cooldown")],
                    49.0,
                    "exit protection must leave the trap controller running"
                );
                if exit {
                    assert_eq!(game.player.health, 4.0, "{character:?}");
                    assert_eq!(game.player.pos, player_pos);
                    assert!(game.player.is_exiting());
                    assert_eq!(game.state, crate::RunState::Ending);
                    assert!(!game.events.contains(&GameEvent::Damage { amount: 4 }));
                } else {
                    assert_eq!(game.player.health, 0.0, "ordinary player {character:?}");
                    assert!(game.events.contains(&GameEvent::Damage { amount: 4 }));
                    assert_eq!(game.state, crate::RunState::Dead);
                }
            }
        }
    }

    #[test]
    fn actual_lava_records_source_cause_for_revived_and_fatal_campaign_hits_only() {
        for (custom, ankh) in [(false, false), (false, true), (true, true)] {
            let mut game = if custom {
                authored_exit_room(crate::characters::Character::Explorer, false)
            } else {
                Game::new(42, RulesPreset::Classic)
            };
            if ankh {
                let p = game.player.pos;
                let id = game.spawn("oAnkh", Vec2::new(p.x, p.y + 4.));
                let item = game.world.entity_mut(id).unwrap();
                item.data.insert("cost".into(), 0.);
                item.data.insert("forSale".into(), 0.);
                game.step(&crate::InputFrame {
                    down: true,
                    attack: true,
                    attack_pressed: true,
                    ..Default::default()
                });
                assert!(game.player.has("Ankh"));
            }
            let p = game.player.pos;
            // Two overlapping lava instances still satisfy one source point query.
            game.spawn("oLava", Vec2::new(p.x - 8., p.y - 8.));
            game.spawn("oLava", Vec2::new(p.x - 8., p.y - 8.));
            game.step(&crate::InputFrame::default());
            assert_eq!(game.lifetime_stats.misc_deaths[11], u64::from(!custom));
            assert_eq!(
                game.events
                    .iter()
                    .filter(|e| **e == GameEvent::Damage { amount: 99 })
                    .count(),
                1
            );
            if ankh {
                assert_eq!(game.player.health, 4.0);
                assert!(!game.player.has("Ankh"));
                assert_ne!(game.state, crate::RunState::Dead);
            } else {
                assert_eq!(game.state, crate::RunState::Dead);
                game.step(&crate::InputFrame::default());
                assert_eq!(
                    game.lifetime_stats.misc_deaths[11], 1,
                    "the independent dead flag suppresses another lava cause"
                );
            }
        }
    }

    #[test]
    fn real_exit_animation_does_not_protect_from_the_outer_lava_check() {
        let mut game = authored_exit_room(crate::characters::Character::Explorer, true);
        let pos = game.player.pos;
        game.spawn("oLava", Vec2::new(pos.x - 8.0, pos.y - 8.0));
        game.step(&crate::InputFrame::default());
        assert_eq!(game.player.health, 0.0);
        assert_eq!(game.state, crate::RunState::Dead);
        assert!(game.events.contains(&GameEvent::Damage { amount: 99 }));
    }

    #[test]
    fn spear_damage_uses_current_directional_mask_and_active_frame() {
        // Frame 20's right mask occupies x80..88; the left mask occupies
        // x87..95. A player probe x93..105 therefore hits only the left one.
        let (mut right, _) = spear_room(true, 99.0);
        right.step(&crate::InputFrame::default());
        assert_eq!(right.player.health, 16.0);
        let (mut left, spear) = spear_room(false, 90.0);
        left.world.entity_mut(spear).unwrap().animation = 19.0;
        left.step(&crate::InputFrame::default());
        assert_eq!(left.player.health, 16.0);
        left.step(&crate::InputFrame::default());
        assert_eq!(left.player.health, 12.0);
    }

    #[test]
    fn spear_animation_finishes_during_the_death_reveal() {
        let (mut game, spear) = spear_room(false, 40.0);
        game.player.health = 0.0;
        game.player.state = PlayerState::Dead;
        game.state = crate::RunState::Dead;
        for _ in 0..35 {
            game.step(&crate::InputFrame::default());
        }
        assert_eq!(game.state, crate::RunState::Dead);
        assert!(game.world.entity(spear).is_none());
        assert!(game.data.get("death_ticks").copied().unwrap_or(0.0) >= 35.0);
    }

    #[test]
    fn unsupported_spear_trap_and_detached_spears_are_removed_in_game_step() {
        let (mut game, spear) = spear_room(false, 90.0);
        game.world
            .entities
            .iter_mut()
            .find(|e| e.object == "oSpearTrapTop")
            .unwrap()
            .alive = false;
        game.step(&crate::InputFrame::default());
        assert!(game.world.entity(spear).is_none());
        // Player319 observes the active spear before Spear528 destroys it.
        assert_eq!(game.player.health, 12.0);
        assert!(game.events.contains(&GameEvent::Damage { amount: 4 }));
        let mut game = room();
        let trap = game.spawn("oSpearTrapTop", Vec2::new(80.0, 80.0));
        game.data
            .insert(format!("hazard_entity:{trap}:cooldown"), 25.0);
        game.step(&crate::InputFrame::default());
        assert!(game.world.entity(trap).is_none());
    }

    #[test]
    fn spear_detector_uses_enemy_origin_and_excludes_treasure() {
        let mut game = room();
        for x in 0..game.world.width {
            game.world.set_tile(x, 6, "oHardBlock");
        }
        game.spawn("oSpearTrapTop", Vec2::new(80.0, 80.0));
        game.spawn("oCaveman", Vec2::new(110.0, 80.0));
        game.step(&crate::InputFrame::default());
        assert!(
            game.world
                .entities
                .iter()
                .any(|e| e.object == "oSpearsLeft" && e.sprite == "sSpearsRight")
        );

        let mut game = room();
        for x in 0..game.world.width {
            game.world.set_tile(x, 6, "oHardBlock");
        }
        game.spawn("oSpearTrapTop", Vec2::new(80.0, 80.0));
        game.spawn("oGoldNugget", Vec2::new(110.0, 88.0));
        game.step(&crate::InputFrame::default());
        assert!(
            !game
                .world
                .entities
                .iter()
                .any(|e| e.object == "oSpearsLeft")
        );
    }
}

#[cfg(test)]
#[path = "traps/player_spikes_blood_contract_tests.rs"]
mod player_spikes_blood_contract_tests;
