//! Source-based population rules from Classic's scrEntityGen/scrTreasureGen.
//!
//! Entity coordinates preserve GameMaker instance origins; entrance/exit
//! coordinates remain tile centers. Temple preserves eager GM8 Boolean
//! operands; statement-level else-if probability chains remain conditional.
use crate::model::{Area, Rect, Rng, Vec2, World};
use std::collections::BTreeSet;

fn flag(flags: &BTreeSet<String>, names: &[&str]) -> bool {
    names.iter().any(|name| flags.contains(*name))
}
fn solid(world: &World, x: i32, y: i32) -> bool {
    world.tile(x, y).is_none_or(|tile| tile.is_solid())
}
fn at(x: i32, y: i32) -> Vec2 {
    Vec2::new(x as f64 * 16.0 + 8.0, y as f64 * 16.0 + 8.0)
}
fn occupied(world: &World, x: i32, y: i32) -> bool {
    let pos = at(x, y);
    world
        .entities
        .iter()
        .any(|e| e.alive && e.bounds().contains(pos))
}
#[cfg(test)]
fn wet(world: &World, x: i32, y: i32) -> bool {
    let point = at(x, y);
    world.tiles_point(point, "oWater")
        || world.entities.iter().any(|entity| {
            entity.alive
                && crate::catalog::is_a(&entity.object, "oWater")
                && entity.sprite_bounds().contains(point)
        })
}
struct WaterMasks(Vec<Rect>);
impl WaterMasks {
    fn new(world: &World) -> Self {
        // Population only adds actors/items, never moves or creates water.
        // Resolve source masks once so repeated wet probes do not rescan all
        // unrelated terrain metadata and object ancestry for every cell.
        Self(
            world
                .tiles
                .iter()
                .enumerate()
                .filter(|(_, tile)| crate::catalog::is_a(&tile.object, "oWater"))
                .map(|(index, _)| {
                    world.tile_bounds((index % world.width) as i32, (index / world.width) as i32)
                })
                .chain(
                    world
                        .entities
                        .iter()
                        .filter(|entity| {
                            entity.alive && crate::catalog::is_a(&entity.object, "oWater")
                        })
                        .map(|entity| entity.sprite_bounds()),
                )
                .collect(),
        )
    }
    fn wet(&self, x: i32, y: i32) -> bool {
        let point = at(x, y);
        self.0.iter().any(|mask| mask.contains(point))
    }
}
fn campaign_shop_membership(world: &World, origin: Vec2) -> Option<bool> {
    let path = world.source_room_path.as_ref()?;
    let (x, y) = source_room(origin);
    Some(matches!(path[y as usize][x as usize], 4 | 5))
}
fn in_shop(world: &World, x: i32, y: i32) -> bool {
    if let Some(in_shop) = campaign_shop_membership(world, at(x, y)) {
        return in_shop;
    }
    let cell = room(at(x, y));
    // Historical/None contexts retain their original template-keeper adapter.
    world
        .entities
        .iter()
        .any(|e| e.object == "oShopkeeper" && room(e.pos) == cell)
}
fn room(pos: Vec2) -> (i32, i32) {
    (
        ((pos.x - 16.0).max(0.0) / 160.0) as i32,
        ((pos.y - 16.0).max(0.0) / 128.0) as i32,
    )
}
fn start_room(world: &World, x: i32, y: i32) -> bool {
    room(at(x, y)) == room(world.entrance)
}
fn spawn(world: &mut World, rng: &mut Rng, name: &str, x: i32, y: i32) {
    spawn_at(
        world,
        rng,
        name,
        Vec2::new(f64::from(x) * 16.0, f64::from(y) * 16.0),
    );
}
fn spawn_at(world: &mut World, rng: &mut Rng, name: &str, origin: Vec2) {
    // `x,y` denotes the tile, while these offsets are the literal
    // instance_create offsets used by the source population scripts.
    let offset = match name {
        "oGoldBar" | "oEmeraldBig" | "oSapphireBig" | "oRubyBig" | "oRock" => Vec2::new(8.0, 12.0),
        "oJar" => Vec2::new(8.0, 10.0),
        "oGoldBars" | "oLockedChest" | "oChest" | "oCrate" | "oDamsel" | "oKey" | "oShotgun"
        | "oGoldNugget" => Vec2::new(8.0, 8.0),
        "oPiranha" | "oDeadFish" => Vec2::new(4.0, 4.0),
        _ => Vec2::default(),
    };
    let id = crate::creation::spawn_generated_entity(
        world,
        rng,
        name,
        Vec2::new(origin.x + offset.x, origin.y + offset.y),
    );
    if name == "oDamsel" {
        // scrTreasureGen clears the price after the Damsel Create event.
        let damsel = world.entity_mut(id).unwrap();
        damsel.data.insert("cost".into(), 0.0);
        damsel.data.insert("for_sale".into(), 0.0);
    }
}
fn exists(world: &World, name: &str) -> bool {
    world.entities.iter().any(|e| e.alive && e.object == name)
}
fn bones_at(world: &mut World, rng: &mut Rng, origin: Vec2) {
    if rng.chance(1, 8) {
        spawn_at(world, rng, "oFakeBones", origin);
    } else {
        spawn_at(world, rng, "oBones", origin);
        world.spawn("oSkull", Vec2::new(origin.x + 12.0, origin.y + 12.0));
    }
}

struct TreasureState {
    udjat: bool,
    chest_chance: u32,
    damsel: bool,
    cemetery: bool,
}

fn source_point_family(world: &World, point: Vec2, family: &str) -> bool {
    world.tiles_point(point, family)
        || world.entities.iter().any(|entity| {
            entity.alive
                && crate::catalog::is_a(&entity.object, family)
                && entity.mask_contains(point, crate::CollisionPrecision::BoundingBox)
        })
}

fn source_rectangle_family(world: &World, rectangle: Rect, family: &str) -> bool {
    // Rect stores literal inclusive query endpoints as max-min+1. GM8 keeps
    // those endpoints fractional, but the target aggregate bbox is integer.
    // This is deliberately asymmetric: expanding a half-open query by one
    // would incorrectly hit a target just beyond a fractional query maximum.
    let hits = |target: Rect| {
        target.is_valid()
            && rectangle.is_valid()
            && target.x + target.w > rectangle.x
            && target.x <= rectangle.x + rectangle.w - 1.0
            && target.y + target.h > rectangle.y
            && target.y <= rectangle.y + rectangle.h - 1.0
    };
    world.tiles.iter().enumerate().any(|(index, tile)| {
        crate::catalog::is_a(&tile.object, family)
            && hits(
                world
                    .tile_collision_mask((index % world.width) as i32, (index / world.width) as i32)
                    .point_bounds(),
            )
    }) || world.entities.iter().any(|entity| {
        entity.alive
            && crate::catalog::is_a(&entity.object, family)
            && hits(entity.collision_mask().point_bounds())
    })
}

fn source_family_distance(world: &World, bounds: Rect, family: &str) -> f64 {
    world
        .tiles
        .iter()
        .enumerate()
        .filter(|(_, tile)| crate::catalog::is_a(&tile.object, family))
        .map(|(index, _)| {
            world
                .tile_collision_mask((index % world.width) as i32, (index / world.width) as i32)
                .point_bounds()
        })
        .chain(
            world
                .entities
                .iter()
                .filter(|entity| entity.alive && crate::catalog::is_a(&entity.object, family))
                .map(|entity| entity.collision_mask().point_bounds()),
        )
        .filter(|target| target.is_valid())
        .map(|target| crate::world_events::mask_distance(bounds, target))
        .fold(1_000_000.0, f64::min)
}

#[derive(Clone, Copy)]
enum FamilyInstance {
    Tile(usize),
    Entity(u64),
}

struct TempleReceiver {
    instance: FamilyInstance,
    origin: Vec2,
    bounds: Rect,
    kind: String,
    source: crate::model::Entity,
}

fn temple_receiver(
    world: &World,
    serial: u64,
    instance: FamilyInstance,
    origin: Vec2,
) -> Option<TempleReceiver> {
    let (mut source, bounds) = match instance {
        FamilyInstance::Tile(index) => {
            let tile = world.tiles.get(index)?;
            if tile.creation_serial != serial || !crate::catalog::is_a(&tile.object, "oSolid") {
                return None;
            }
            let center = Vec2::new(origin.x + 8.0, origin.y + 8.0);
            let metadata = world.entities.iter().find(|entity| {
                entity.alive && entity.object == "oTerrainMetadata" && entity.pos == center
            });
            let mut source = crate::model::Entity::new(0, &tile.object, origin);
            source.creation_serial = serial;
            if let Some(metadata) = metadata {
                source.data = metadata.data.clone();
                source.text = metadata.text.clone();
                source.animation = metadata.animation;
                if let Some(sprite) = metadata.text.get("terrain_sprite") {
                    source.sprite = sprite.clone();
                }
            }
            (
                source,
                world
                    .tile_collision_mask((index % world.width) as i32, (index / world.width) as i32)
                    .point_bounds(),
            )
        }
        FamilyInstance::Entity(id) => {
            let entity = world.entity(id)?;
            if entity.creation_serial != serial || !crate::catalog::is_a(&entity.object, "oSolid") {
                return None;
            }
            (entity.clone(), entity.collision_mask().point_bounds())
        }
    };
    let kind = source.text.get("type").cloned().unwrap_or_else(|| {
        let label = if matches!(source.object.as_str(), "oTree" | "oTreeTile") {
            "Tree"
        } else if matches!(
            source.object.as_str(),
            "oSacAltarLeft" | "oSacAltarRight" | "oAltarLeft" | "oAltarRight"
        ) {
            "Altar"
        } else {
            "None"
        };
        label.into()
    });
    source.data.insert("cleanDeath".into(), 1.0);
    Some(TempleReceiver {
        instance,
        origin,
        bounds,
        kind,
        source,
    })
}

fn retire_temple_receiver(world: &mut World, rng: &mut Rng, receiver: &TempleReceiver) {
    // set_tile marks any removal dirty; source overriding Destroy events do
    // not all inherit oSolid. Let the audited callback select checkWater.
    let was_dirty = world.fluid_dirty;
    match receiver.instance {
        FamilyInstance::Tile(index) => {
            world.set_tile(index % world.width, index / world.width, "");
            let center = Vec2::new(receiver.origin.x + 8.0, receiver.origin.y + 8.0);
            for metadata in world.entities.iter_mut().filter(|entity| {
                entity.alive && entity.object == "oTerrainMetadata" && entity.pos == center
            }) {
                metadata.alive = false;
            }
        }
        FamilyInstance::Entity(id) => {
            if let Some(entity) = world.entity_mut(id) {
                entity.data.insert("cleanDeath".into(), 1.0);
                entity.alive = false;
            }
        }
    }
    world.fluid_dirty = was_dirty;
    crate::items::explosives::destroy_temple_support_clean(world, rng, &receiver.source);
}

fn family_instances(world: &World, family: &str) -> Vec<(u64, FamilyInstance, Vec2)> {
    let mut instances: Vec<_> = world
        .tiles
        .iter()
        .enumerate()
        .filter(|(_, tile)| crate::catalog::is_a(&tile.object, family))
        .map(|(index, tile)| {
            (
                tile.creation_serial,
                FamilyInstance::Tile(index),
                Vec2::new(
                    (index % world.width) as f64 * 16.0,
                    (index / world.width) as f64 * 16.0,
                ),
            )
        })
        .chain(
            world
                .entities
                .iter()
                .filter(|entity| entity.alive && crate::catalog::is_a(&entity.object, family))
                .map(|entity| {
                    (
                        entity.creation_serial,
                        FamilyInstance::Entity(entity.id),
                        entity.pos,
                    )
                }),
        )
        .collect();
    // Source generation's with-family arrays append at instance creation.
    // Merge the port's two storage forms by that order; this does not establish
    // equivalence of earlier authored births or post-Draw rebuilt arrays.
    instances.sort_by_key(|(serial, _, _)| *serial);
    instances
}

fn family_origins(world: &World, family: &str) -> Vec<Vec2> {
    family_instances(world, family)
        .into_iter()
        .map(|(_, _, position)| position)
        .collect()
}

fn protect_gold_door_support(world: &mut World, instance: FamilyInstance) {
    match instance {
        FamilyInstance::Entity(id) => {
            if let Some(support) = world.entity_mut(id) {
                support.data.insert("invincible".into(), 1.0);
            }
        }
        FamilyInstance::Tile(index) => {
            super::tiles::terrain_property(
                world,
                index % world.width,
                index / world.width,
                "invincible",
                1.0,
            );
        }
    }
}

// scrEntityGen:174–208 is a complete Lush pass before the Solid pass.
// Its Grave/item births do not grow, destroy, or activate the Lush family,
// so captured identities with live checks preserve this bounded with-body.
fn cemetery_prepass(world: &mut World, rng: &mut Rng, context: super::GenerationContext) {
    for (serial, instance, origin) in family_instances(world, "oLush") {
        let live = match instance {
            FamilyInstance::Tile(index) => world.tiles.get(index).is_some_and(|tile| {
                tile.creation_serial == serial
                    && tile.engine_active
                    && crate::catalog::is_a(&tile.object, "oLush")
            }),
            FamilyInstance::Entity(id) => world.entity(id).is_some_and(|entity| {
                entity.alive
                    && entity.engine_active
                    && entity.creation_serial == serial
                    && crate::catalog::is_a(&entity.object, "oLush")
            }),
        };
        if !live {
            continue;
        }
        let above = Vec2::new(origin.x, origin.y - 16.0);
        let clear = !source_point_family(world, above, "oSolid")
            && !source_point_family(world, above, "oEntrance")
            && !source_point_family(world, above, "oExit");
        // GM8 evaluates this rand even when a collision operand is false.
        let grave_roll = rng.range(20) == 0;
        if !clear || !grave_roll || [160.0, 176.0, 320.0, 336.0, 480.0, 496.0].contains(&origin.x) {
            continue;
        }
        let grave = crate::creation::spawn_generated_entity(world, rng, "oGrave", above);
        cemetery_receiver_value(world, instance, "obj", grave as f64);
        // global.ashGrave was reset false, but :195 writes *receiver-local*
        // ashGrave. No later receiver is suppressed by an earlier Ash grave.
        if rng.range(40) == 0 {
            world.entity_mut(grave).unwrap().sprite = "sGraveAsh".into();
            let shotgun = crate::creation::spawn_generated_jungle_child(
                world,
                rng,
                "oShotgun",
                Vec2::new(origin.x + 8.0, origin.y + 8.0),
                context.has_spectacles,
            );
            let child = world.entity_mut(shotgun).unwrap();
            child.data.insert("cost".into(), 0.0);
            child.data.insert("forSale".into(), 0.0);
            cemetery_receiver_value(world, instance, "obj", shotgun as f64);
            cemetery_receiver_value(world, instance, "ashGrave", 1.0);
        } else {
            let center = Vec2::new(origin.x + 8.0, origin.y + 8.0);
            if !source_point_family(world, center, "oTreasure") {
                let object = if rng.range(2) == 0 {
                    Some("oGoldNugget")
                } else if rng.range(4) == 0 {
                    Some("oSapphireBig")
                } else if rng.range(6) == 0 {
                    Some("oEmeraldBig")
                } else if rng.range(8) == 0 {
                    Some("oRubyBig")
                } else {
                    None
                };
                if let Some(object) = object {
                    crate::creation::spawn_generated_jungle_child(
                        world,
                        rng,
                        object,
                        center,
                        context.has_spectacles,
                    );
                }
            }
        }
    }
}

fn cemetery_receiver_value(world: &mut World, instance: FamilyInstance, key: &str, value: f64) {
    match instance {
        FamilyInstance::Tile(index) => {
            super::tiles::terrain_property(
                world,
                index % world.width,
                index / world.width,
                key,
                value,
            );
        }
        FamilyInstance::Entity(id) => {
            if let Some(entity) = world.entity_mut(id) {
                entity.data.insert(key.into(), value);
            }
        }
    }
}

#[cfg(test)]
struct JunglePopulationTrace {
    world_before: World,
    rng_before: Rng,
    rng_after: Rng,
    cemetery: bool,
}

#[cfg(test)]
thread_local! {
    static JUNGLE_POPULATION_TRACE: std::cell::RefCell<Option<Vec<JunglePopulationTrace>>> =
        const { std::cell::RefCell::new(None) };
}

fn jungle_population(world: &mut World, rng: &mut Rng, cemetery: bool) {
    #[cfg(test)]
    let before = JUNGLE_POPULATION_TRACE.with(|trace| {
        trace
            .borrow()
            .as_ref()
            .map(|_| (world.clone(), rng.clone()))
    });
    // scrEntityGen:322–325 visits Vine instances, not covered grid cells.
    // VineTop is a sibling, so it does not participate. Each Monkey Create
    // consumes its facing draw before the next Vine's chance draw.
    for position in family_origins(world, "oVine") {
        if rng.chance(1, 15) {
            crate::creation::spawn_generated_entity(world, rng, "oMonkey", position);
        }
    }
    // The separate with oWater at :327–337 includes WaterSwim and Lava.
    // Probe the instance origin, then create exactly once per eligible
    // instance at +4,+4; overlapping instances still each get their roll.
    for position in family_origins(world, "oWater") {
        if !source_point_family(world, position, "oSolid") && rng.chance(1, 30) {
            crate::creation::spawn_generated_entity(
                world,
                rng,
                if cemetery { "oDeadFish" } else { "oPiranha" },
                Vec2::new(position.x + 4.0, position.y + 4.0),
            );
        }
    }
    #[cfg(test)]
    if let Some((world_before, rng_before)) = before {
        JUNGLE_POPULATION_TRACE.with(|trace| {
            if let Some(trace) = trace.borrow_mut().as_mut() {
                trace.push(JunglePopulationTrace {
                    world_before,
                    rng_before,
                    rng_after: rng.clone(),
                    cemetery,
                });
            }
        });
    }
}

fn treasure(
    world: &mut World,
    rng: &mut Rng,
    water: &WaterMasks,
    state: &mut TreasureState,
    x: i32,
    y: i32,
) {
    treasure_at(
        world,
        rng,
        water,
        state,
        Vec2::new(f64::from(x) * 16.0, f64::from(y) * 16.0),
        None,
    );
}

fn damsel_chance(rng: &mut Rng, already_exists: bool) -> bool {
    // The reached source condition draws even with an existing Damsel.
    let success = rng.chance(1, 8);
    !already_exists && success
}

fn treasure_at(
    world: &mut World,
    rng: &mut Rng,
    water: &WaterMasks,
    state: &mut TreasureState,
    origin: Vec2,
    receiver_bounds: Option<Rect>,
) {
    let pos = Vec2::new(origin.x + 8.0, origin.y + 8.0);
    let near_landmark = if let Some(bounds) = receiver_bounds {
        source_family_distance(world, bounds, "oEntrance") < 32.0
            || source_family_distance(world, bounds, "oExit") < 32.0
            || source_family_distance(world, bounds, "oGoldIdol") < 64.0
    } else {
        pos.distance(world.entrance) < 32.0
            || pos.distance(world.exit) < 32.0
            || world
                .entities
                .iter()
                .any(|e| e.object == "oGoldIdol" && e.pos.distance(pos) < 64.0)
    };
    if near_landmark {
        return;
    }
    // scrTreasureGen.gml:34–57 probes named source families at distinct
    // coordinates. Enemies do not block loot; oTreasure blocks the alcove /
    // tunnel branches but deliberately does not block the normal fallback.
    let above = Vec2::new(origin.x, origin.y - 16.0);
    let middle = Vec2::new(origin.x, origin.y - 8.0);
    let clear_normal = !source_point_family(world, above, "oSolid")
        && ["oChest", "oSpikes", "oEntrance", "oExit"]
            .iter()
            .all(|family| !source_point_family(world, middle, family));
    let col_stuff = !clear_normal
        || source_point_family(world, Vec2::new(origin.x, origin.y - 1.0), "oTreasure");
    if !col_stuff {
        if rng.chance(1, 100) {
            spawn_at(world, rng, "oRock", above);
            return;
        }
        if rng.chance(1, 40) {
            spawn_at(world, rng, "oJar", above);
            return;
        }
    }
    let left = Vec2::new(origin.x - 16.0, origin.y - 16.0);
    let right = Vec2::new(origin.x + 16.0, origin.y - 16.0);
    let bonus = if state.cemetery { 10 } else { 0 };
    let level = world.level;
    // scrTreasureGen:49–51 probes the roof before either side, and :96–97
    // only probes the tunnel when the alcove branch did not match.
    if !col_stuff
        && source_point_family(world, Vec2::new(origin.x, origin.y - 32.0), "oSolid")
        && (source_point_family(world, left, "oSolid")
            || source_point_family(world, right, "oSolid")
            || source_point_family(world, left, "oBlock")
            || source_point_family(world, right, "oBlock"))
    {
        // Preserve scrTreasureGen:54's literal typo, not intended proximity:
        // GiantSpider's compiled object index is 749, so (749 < 100) is 0.
        // distance_to_object(0) has no target and returns 1,000,000 (truthy).
        let web_roll = rng.chance(1, 5);
        if world.area != Area::Ice && web_roll {
            spawn_at(world, rng, "oWeb", above);
        } else if state.udjat && !exists(world, "oLockedChest") {
            if rng.chance(1, state.chest_chance) {
                spawn_at(world, rng, "oLockedChest", above);
            } else {
                state.chest_chance = state.chest_chance.saturating_sub(1).max(1);
            }
        } else if rng.chance(1, 10) {
            spawn_at(world, rng, "oCrate", above);
        } else if rng.chance(1, 15) {
            spawn_at(world, rng, "oChest", above);
        } else if damsel_chance(rng, state.damsel)
            && !if receiver_bounds.is_some() {
                source_point_family(world, Vec2::new(origin.x + 8.0, origin.y - 8.0), "oWater")
            } else {
                water
                    .0
                    .iter()
                    .any(|mask| mask.contains(Vec2::new(origin.x + 8.0, origin.y - 8.0)))
            }
        {
            spawn_at(world, rng, "oDamsel", above);
            state.damsel = true;
        } else if rng.chance(1 + bonus, 40_u32.saturating_sub(2 * level).max(1)) {
            bones_at(world, rng, above);
        } else if rng.chance(1, 3) {
            spawn_at(world, rng, "oGoldBar", above);
        } else if rng.chance(1, 6) {
            spawn_at(world, rng, "oGoldBars", above);
        } else if rng.chance(1, 6) {
            spawn_at(world, rng, "oEmeraldBig", above);
        } else if rng.chance(1, 8) {
            spawn_at(world, rng, "oSapphireBig", above);
        } else if rng.chance(1, 10) {
            spawn_at(world, rng, "oRubyBig", above);
        }
    } else if !col_stuff
        && source_point_family(world, left, "oSolid")
        && source_point_family(world, right, "oSolid")
    {
        // The same literal expression at scrTreasureGen:100 assigns n = 10.
        let web_roll = rng.chance(1, 10);
        if world.area != Area::Ice && web_roll {
            spawn_at(world, rng, "oWeb", above);
        } else if rng.chance(1, 4) {
            spawn_at(world, rng, "oGoldBar", above);
        } else if rng.chance(1, 8) {
            spawn_at(world, rng, "oGoldBars", above);
        } else if rng.chance(1 + bonus, 80_u32.saturating_sub(level).max(1)) {
            bones_at(world, rng, above);
        } else if rng.chance(1, 8) {
            spawn_at(world, rng, "oEmeraldBig", above);
        } else if rng.chance(1, 9) {
            spawn_at(world, rng, "oSapphireBig", above);
        } else if rng.chance(1, 10) {
            spawn_at(world, rng, "oRubyBig", above);
        }
    } else if clear_normal {
        if rng.chance(1, 40) {
            spawn_at(world, rng, "oGoldBar", above);
        } else if rng.chance(1, 50) {
            spawn_at(world, rng, "oGoldBars", above);
        } else if rng.chance(1 + bonus, 140_u32.saturating_sub(2 * level).max(1)) {
            bones_at(world, rng, above);
        }
    }
}

fn schedule(rng: &mut Rng, level: u32, first: u32, last: u32) -> bool {
    // scrInitLevel's two scheduling windows each have three levels. The
    // Boolean level guards are eager; the else-if statements are conditional.
    let first_roll = rng.chance(1, 3);
    if level == first && first_roll {
        true
    } else {
        let second_roll = rng.chance(1, 2);
        (level == first + 1 && second_roll) || level == last
    }
}

// Test-only observation of the existing Ice floor geometry adapter. This
// proves its random/event ordering on natural layouts, not geometry parity.
#[cfg(test)]
struct IceFloorRollTrace {
    origin: Vec2,
    dark_support: bool,
    spring_allowed: bool,
    before: Rng,
    after: Rng,
    births: Vec<(String, Vec2)>,
}
#[cfg(test)]
thread_local! {
    static ICE_FLOOR_ROLL_TRACE: std::cell::RefCell<Option<Vec<IceFloorRollTrace>>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PopulationPhase {
    Enemies,
    Treasure,
}

#[cfg(test)]
thread_local! {
    static POPULATION_PHASE_TRACE: std::cell::RefCell<Option<Vec<(u32, i32, i32, PopulationPhase)>>> =
        const { std::cell::RefCell::new(None) };
    static TEMPLE_RECEIVER_TRACE: std::cell::RefCell<Option<Vec<(u64, Vec2)>>> =
        const { std::cell::RefCell::new(None) };
    static TEMPLE_RNG_STARTS: std::cell::RefCell<Option<Vec<(u64, Rng)>>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
fn trace_population_phase(world: &World, x: i32, y: i32, phase: PopulationPhase) {
    POPULATION_PHASE_TRACE.with(|trace| {
        if let Some(trace) = trace.borrow_mut().as_mut() {
            trace.push((world.level, x, y, phase));
        }
    });
}

fn source_room(origin: Vec2) -> (i32, i32) {
    // Literal scrGetRoomX/Y ranges, including their outer-room clamps.
    let x = if origin.x < 176.0 {
        0
    } else if origin.x < 336.0 {
        1
    } else if origin.x < 496.0 {
        2
    } else {
        3
    };
    let y = if origin.y < 144.0 {
        0
    } else if origin.y < 272.0 {
        1
    } else if origin.y < 400.0 {
        2
    } else if origin.y < 528.0 {
        3
    } else {
        4
    };
    (x, y)
}

fn temple_in_shop(world: &World, origin: Vec2) -> bool {
    if let Some(in_shop) = campaign_shop_membership(world, origin) {
        return in_shop;
    }
    let room = source_room(origin);
    world.entities.iter().any(|entity| {
        entity.alive && entity.object == "oShopkeeper" && source_room(entity.pos) == room
    })
}

fn temple_statue(world: &mut World, origin: Vec2) {
    let id = world.spawn(
        "oGenerationDecoration",
        Vec2::new(origin.x, origin.y - 32.0),
    );
    let statue = world.entity_mut(id).unwrap();
    statue.sprite = "bgStatues".into();
    for (key, value) in [
        ("decoration", 1.0),
        ("source_x", 0.0),
        ("source_y", 0.0),
        ("source_w", 16.0),
        ("source_h", 48.0),
        ("depth", 9005.0),
    ] {
        statue.data.insert(key.into(), value);
    }
}

fn temple_population(
    world: &mut World,
    rng: &mut Rng,
    water: &WaterMasks,
    state: &mut TreasureState,
    gen_tomb_lord: bool,
    gold_chance: &mut u32,
) -> bool {
    // Canonical with(object) captures the initial descendant count; births
    // append beyond it and Destroy marks rather than compacts the list. This
    // generation call chain performs no activation/rebuild, so fixed identities
    // plus live identity checks preserve that scoped traversal contract.
    let mut made_gold = false;
    let mut made_tomb = false;
    let receivers = family_instances(world, "oSolid");
    let entrance = family_origins(world, "oEntrance").into_iter().next();
    let start = source_room(Vec2::new(world.entrance.x - 8.0, world.entrance.y - 8.0));
    for (serial, instance, origin) in receivers {
        let Some(receiver) = temple_receiver(world, serial, instance, origin) else {
            continue;
        };
        #[cfg(test)]
        TEMPLE_RECEIVER_TRACE.with(|trace| {
            if let Some(trace) = trace.borrow_mut().as_mut() {
                trace.push((serial, origin));
            }
        });
        #[cfg(test)]
        TEMPLE_RNG_STARTS.with(|trace| {
            if let Some(trace) = trace.borrow_mut().as_mut() {
                trace.push((serial, rng.clone()));
            }
        });
        let above = Vec2::new(origin.x, origin.y - 16.0);
        let high = Vec2::new(origin.x, origin.y - 32.0);
        // GM8 Boolean operands are eager. Always consume this draw before
        // the shop gate, even when the above-point is covered or graphics low.
        let statue_roll = rng.chance(1, 100);
        let statue_clear = !source_point_family(world, above, "oSolid");
        if statue_roll && statue_clear {
            temple_statue(world, origin);
        }
        if temple_in_shop(world, origin) {
            continue;
        }
        if origin.y > 32.0
            && !source_point_family(world, above, "oSolid")
            && world.level == 14
            && !made_gold
        {
            if rng.chance(1, *gold_chance) {
                spawn_at(world, rng, "oGoldDoor", above);
                protect_gold_door_support(world, receiver.instance);
                made_gold = true;
            } else {
                *gold_chance = gold_chance.saturating_sub(1).max(1);
            }
        } else if receiver.kind != "Tree"
            && receiver.kind != "Altar"
            && origin.y != 0.0
            && !source_point_family(world, above, "oSolid")
            && !source_point_family(world, above, "oLava")
            && !source_rectangle_family(world, Rect::new(origin.x, above.y, 16.0, 16.0), "oEnemy")
            && !source_point_family(world, high, "oSolid")
            && (!source_point_family(world, Vec2::new(origin.x - 16.0, origin.y), "oSolid")
                || !source_point_family(world, Vec2::new(origin.x + 16.0, origin.y), "oSolid"))
            && source_point_family(world, Vec2::new(origin.x, origin.y + 16.0), "oSolid")
            && ![160.0, 176.0, 320.0, 336.0, 480.0, 496.0].contains(&origin.x)
        {
            let spear_roll = rng.chance(1, 12);
            // Source-valid generated rooms have a real Entrance. Do not
            // substitute the presentation's center for its instance origin.
            let far = entrance.is_some_and(|entrance| origin.distance(entrance) > 64.0);
            if spear_roll && far {
                let plugs_lava =
                    source_point_family(world, Vec2::new(origin.x - 16.0, high.y), "oSolid")
                        && source_point_family(world, Vec2::new(origin.x + 16.0, high.y), "oSolid")
                        && !source_point_family(world, high, "oSolid");
                if !plugs_lava {
                    if source_point_family(world, above, "oSolid") {
                        let nearest = family_instances(world, "oSolid")
                            .into_iter()
                            .min_by(|a, b| a.2.distance(above).total_cmp(&b.2.distance(above)));
                        if let Some((serial, instance, position)) = nearest
                            && let Some(other) = temple_receiver(world, serial, instance, position)
                        {
                            retire_temple_receiver(world, rng, &other);
                        }
                    }
                    spawn_at(world, rng, "oSpearTrapBottom", origin);
                    spawn_at(
                        world,
                        rng,
                        if world.dark {
                            "oSpearTrapLit"
                        } else {
                            "oSpearTrapTop"
                        },
                        above,
                    );
                    retire_temple_receiver(world, rng, &receiver);
                }
            }
        }
        #[cfg(test)]
        trace_population_phase(
            world,
            (origin.x / 16.0) as i32,
            (origin.y / 16.0) as i32,
            PopulationPhase::Enemies,
        );
        if origin.y < world.height as f64 * 16.0 - 64.0
            && !source_point_family(world, Vec2::new(origin.x, origin.y + 16.0), "oSolid")
            && !source_point_family(world, Vec2::new(origin.x, origin.y + 32.0), "oSolid")
            && !source_point_family(world, Vec2::new(origin.x, origin.y + 16.0), "oWater")
            && !source_point_family(world, Vec2::new(origin.x, origin.y + 32.0), "oWater")
        {
            let scarab_roll = rng.chance(1, 40);
            if world.dark && scarab_roll {
                spawn_at(world, rng, "oScarab", Vec2::new(origin.x, origin.y + 16.0));
            }
        }
        let destination = source_room(above);
        // GM8 and/or have equal priority and associate left-to-right.
        if (destination.0 != start.0 || destination.1 != start.1)
            && !source_point_family(world, above, "oEnemy")
            && origin.y > 16.0
            && !source_point_family(world, above, "oSolid")
        {
            let clear =
                !source_rectangle_family(world, Rect::new(origin.x, high.y, 33.0, 32.0), "oSolid");
            let tomb_roll = rng.chance(1, 40);
            if gen_tomb_lord && !made_tomb && clear && tomb_roll {
                spawn_at(world, rng, "oTombLord", high);
                made_tomb = true;
            } else if rng.chance(1, 40) {
                spawn_at(world, rng, "oCaveman", above);
            } else if rng.chance(1, 40) {
                spawn_at(world, rng, "oHawkman", above);
            } else if rng.chance(1, 60) {
                spawn_at(
                    world,
                    rng,
                    if world.dark {
                        "oSmashTrapLit"
                    } else {
                        "oSmashTrap"
                    },
                    above,
                );
            }
        }
        if receiver.kind != "Altar" {
            #[cfg(test)]
            trace_population_phase(
                world,
                (origin.x / 16.0) as i32,
                (origin.y / 16.0) as i32,
                PopulationPhase::Treasure,
            );
            treasure_at(world, rng, water, state, origin, Some(receiver.bounds));
        }
    }
    made_gold
}

fn mines_ceiling(world: &mut World, rng: &mut Rng, giant_spider: bool, x: i32, y: i32) {
    // scrEntityGen:53-65: GM8 evaluates every Boolean operand, including
    // the roll when a flag or clearance check is false. Else-if statements
    // remain conditional: a successful spawn suppresses all later rolls.
    let giant_roll = rng.chance(1, 40);
    if giant_spider
        && !exists(world, "oGiantSpiderHang")
        && !solid(world, x + 1, y + 1)
        && !solid(world, x + 1, y + 2)
        && giant_roll
    {
        spawn(world, rng, "oGiantSpiderHang", x, y + 1);
    } else {
        let lamp_roll = rng.chance(1, 60);
        if world.dark && lamp_roll {
            spawn(world, rng, "oLamp", x, y + 1);
        } else {
            let scarab_roll = rng.chance(1, 40);
            if world.dark && scarab_roll {
                spawn(world, rng, "oScarab", x, y + 1);
            } else if rng.chance(1, 60) {
                spawn(world, rng, "oBat", x, y + 1);
            } else if rng.chance(1, 80) {
                spawn(world, rng, "oSpiderHang", x, y + 1);
            }
        }
    }
}

pub(super) fn populate(world: &mut World, rng: &mut Rng, flags: &BTreeSet<String>) {
    populate_internal(
        world,
        rng,
        flags,
        super::GenerationContext::default(),
        None,
        None,
    );
}

pub(super) fn populate_with_campaign(
    world: &mut World,
    rng: &mut Rng,
    flags: &BTreeSet<String>,
    context: super::GenerationContext,
    campaign: &mut super::GenerationState,
    room_path: &[[u8; 4]; 5],
) {
    if campaign.revision == crate::GenerationRevision::HistoricalV1 {
        populate(world, rng, flags);
    } else {
        populate_internal(world, rng, flags, context, Some(campaign), Some(room_path));
    }
}

fn populate_internal(
    world: &mut World,
    rng: &mut Rng,
    flags: &BTreeSet<String>,
    context: super::GenerationContext,
    campaign: Option<&mut super::GenerationState>,
    room_path: Option<&[[u8; 4]; 5]>,
) {
    if world.area == Area::Boss {
        return;
    }
    #[cfg(test)]
    let ice_revision = campaign.as_ref().map(|state| state.revision);
    let source_classic = campaign
        .as_ref()
        .is_some_and(|state| state.revision == crate::GenerationRevision::SourceClassicV2);
    let water = WaterMasks::new(world);
    let cemetery = flag(flags, &["cemetery", "cemetary"]);
    let market = flag(flags, &["black_market", "blackMarket"]);
    let yeti = flag(flags, &["yeti_lair", "yetiLair"]);
    // Preserve the existing InitLevel adapter location. Source performs these
    // decisions before EntityGen; full initialization timing is a separate gap.
    // Only each made flag suppresses its entire statement-level chain.
    let udjat = if flag(flags, &["made_udjat_eye", "madeUdjatEye"]) {
        false
    } else {
        let scheduled = schedule(rng, world.level, 2, 4);
        world.area == Area::Mines && (flag(flags, &["gen_udjat_eye", "genUdjatEye"]) || scheduled)
    };
    let need_market = if flag(flags, &["made_market_entrance", "madeMarketEntrance"]) {
        false
    } else {
        let scheduled = schedule(rng, world.level, 5, 7);
        world.area == Area::Jungle
            && (flag(flags, &["gen_market_entrance", "genMarketEntrance"]) || scheduled)
    };
    let mut state = TreasureState {
        udjat,
        chest_chance: 8,
        damsel: exists(world, "oDamsel"),
        cemetery,
    };
    let giant_spider = world.area == Area::Mines && rng.chance(1, 6);
    let tomb_lord = world.area == Area::Temple && (world.level == 13 || rng.chance(1, 4));
    let mut market_chance = 160_u32;
    let mut gold_chance = 160_u32;
    if world.area == Area::Jungle
        && let Some(campaign) = campaign
    {
        if cemetery {
            #[cfg(test)]
            let before = cemetery_v2_tests::trace_before(world, rng);
            cemetery_prepass(world, rng, context);
            #[cfg(test)]
            crate::creation::capture_generated_cemetery_prepass(world);
            #[cfg(test)]
            cemetery_v2_tests::trace_after(world, rng, before);
        }
        jungle_primary::populate(
            world,
            rng,
            &water,
            &mut state,
            campaign,
            room_path.expect("canonical generation carries actual roomPath"),
            need_market,
            context.has_spectacles,
        );
        jungle_population(world, rng, cemetery);
        return;
    }
    // Snapshot iteration mirrors `with oSolid`: spawned entities are not terrain.
    let supports: Vec<_> = (1..world.height as i32 - 1)
        .flat_map(|y| (1..world.width as i32 - 1).map(move |x| (x, y)))
        .filter(|&(x, y)| solid(world, x, y))
        .collect();
    let made_gold = world.area == Area::Temple
        && temple_population(world, rng, &water, &mut state, tomb_lord, &mut gold_chance);
    for &(x, y) in &supports {
        if world.area == Area::Temple {
            continue;
        }
        if in_shop(world, x, y) {
            continue;
        }
        let tile = world
            .tile(x, y)
            .map(|t| t.object.clone())
            .unwrap_or_default();
        let altar = matches!(
            tile.as_str(),
            "oAltarLeft" | "oAltarRight" | "oSacAltarLeft" | "oSacAltarRight"
        );
        let tree = tile == "oTree";
        let floor = !solid(world, x, y - 1);
        let ceiling = y < world.height as i32 - 4
            && !solid(world, x, y + 1)
            && !solid(world, x, y + 2)
            && !water.wet(x, y + 1)
            && !water.wet(x, y + 2);
        let far = at(x, y).distance(world.entrance) > 64.0;
        // Mines/Jungle call scrTreasureGen before their enemy statements;
        // Ice/Temple call it after them (scrEntityGen.gml:376,478).
        if matches!(world.area, Area::Mines | Area::Jungle) && !altar && !tree {
            #[cfg(test)]
            trace_population_phase(world, x, y, PopulationPhase::Treasure);
            treasure(world, rng, &water, &mut state, x, y);
        }
        #[cfg(test)]
        trace_population_phase(world, x, y, PopulationPhase::Enemies);
        match world.area {
            Area::Mines => {
                if start_room(world, x, y - 1) {
                    continue;
                }
                if ceiling && !occupied(world, x, y + 1) {
                    mines_ceiling(world, rng, giant_spider, x, y);
                }
                if floor && !occupied(world, x, y - 1) {
                    if rng.chance(1, 60) {
                        spawn(world, rng, "oSnake", x, y - 1);
                    } else if rng.chance(1, 800) {
                        spawn(world, rng, "oCaveman", x, y - 1);
                    }
                }
            }
            Area::Jungle => {
                if need_market
                    && !exists(world, "oXMarket")
                    && y > 2
                    && solid(world, x, y - 1)
                    && !tree
                    && !altar
                {
                    if rng.chance(1, market_chance) {
                        spawn(world, rng, "oXMarket", x, y - 1);
                    } else {
                        market_chance = market_chance.saturating_sub(1).max(1);
                    }
                }
                if cemetery
                    && floor
                    && !occupied(world, x, y - 1)
                    && x % 10 != 0
                    && x % 10 != 1
                    && rng.chance(1, 20)
                {
                    spawn(world, rng, "oGrave", x, y - 1);
                    if !exists(world, "oShotgun") && rng.chance(1, 40) {
                        if let Some(grave) = world.entities.last_mut() {
                            grave.sprite = "sGraveAsh".into();
                        }
                        spawn(world, rng, "oShotgun", x, y);
                    } else if rng.chance(1, 2) {
                        spawn(world, rng, "oGoldNugget", x, y);
                    } else if rng.chance(1, 4) {
                        world.spawn("oSapphireBig", at(x, y));
                    } else if rng.chance(1, 6) {
                        world.spawn("oEmeraldBig", at(x, y));
                    } else if rng.chance(1, 8) {
                        world.spawn("oRubyBig", at(x, y));
                    }
                }
                spear(world, rng, &water, x, y, altar || tree, far);
                if start_room(world, x, y - 1) {
                    continue;
                }
                if ceiling {
                    if world.dark && rng.chance(1, 40) {
                        spawn(world, rng, "oScarab", x, y + 1);
                    } else if rng.chance(1, if cemetery { 60 } else { 80 }) {
                        spawn(world, rng, "oBat", x, y + 1);
                    }
                }
                if y > 1 && floor && !occupied(world, x, y - 1) {
                    if cemetery {
                        if rng.chance(1, 25) {
                            spawn(world, rng, "oZombie", x, y - 1);
                        } else if rng.chance(1, 160) {
                            spawn(world, rng, "oVampire", x, y - 1);
                        }
                    } else if !water.wet(x, y - 1) && !(market && y % 8 == 0) && rng.chance(1, 60) {
                        spawn(world, rng, "oManTrap", x, y - 1);
                    } else if !water.wet(x, y - 1) && rng.chance(1, 60) {
                        spawn(world, rng, "oCaveman", x, y - 1);
                    } else if rng.chance(1, 120) {
                        spawn(world, rng, "oFireFrog", x, y - 1);
                    } else if rng.chance(1, 30) {
                        spawn(world, rng, "oFrog", x, y - 1);
                    }
                }
            }
            Area::Ice => {
                if !start_room(world, x, y - 1) {
                    if ceiling {
                        #[cfg(test)]
                        let ice_prefix = ice_ceiling_eager_rng_tests::observe_before(
                            world,
                            rng,
                            x,
                            y,
                            ice_revision,
                        );
                        // scrEntityGen:355 evaluates rand even on a bright Ice level.
                        // Preserve the prior stream for Historical/None callers.
                        let scarab = if source_classic {
                            let roll = rng.chance(1, 40);
                            world.dark && roll
                        } else {
                            world.dark && rng.chance(1, 40)
                        };
                        if scarab {
                            spawn(world, rng, "oScarab", x, y + 1);
                        }
                        #[cfg(test)]
                        ice_ceiling_eager_rng_tests::observe_after(world, rng, ice_prefix);
                    } else if floor
                        && y > 1
                        && y < 37
                        && !occupied(world, x, y - 1)
                        && rng.chance(1, if yeti { 90 } else { 30 })
                    {
                        spawn(world, rng, "oUFO", x, y - 1);
                    }
                }
                if floor && y > 1 && y < 37 && far && !occupied(world, x, y - 1) {
                    #[cfg(test)]
                    let observation = ICE_FLOOR_ROLL_TRACE.with(|trace| {
                        trace.borrow().is_some().then(|| {
                            (
                                rng.clone(),
                                world.entities.len(),
                                tile == "oDark"
                                    && !solid(world, x, y - 2)
                                    && !solid(world, x, y - 3)
                                    && !solid(world, x, y - 4)
                                    && at(x, y).distance(world.exit) > 64.0,
                            )
                        })
                    });
                    // Source :370 evaluates its roll before every pure guard.
                    let spring_roll = rng.chance(1, 10);
                    if spring_roll
                        && tile == "oDark"
                        && !solid(world, x, y - 2)
                        && !solid(world, x, y - 3)
                        && !solid(world, x, y - 4)
                        && at(x, y).distance(world.exit) > 64.0
                    {
                        spawn(world, rng, "oSpringTrap", x, y - 1);
                    } else if rng.chance(1, 20) {
                        spawn(world, rng, "oYeti", x, y - 1);
                    }
                    #[cfg(test)]
                    if let Some((before, first_birth, spring_allowed)) = observation {
                        ICE_FLOOR_ROLL_TRACE.with(|trace| {
                            trace
                                .borrow_mut()
                                .as_mut()
                                .unwrap()
                                .push(IceFloorRollTrace {
                                    origin: Vec2::new(f64::from(x) * 16.0, f64::from(y) * 16.0),
                                    dark_support: tile == "oDark",
                                    spring_allowed,
                                    before,
                                    after: rng.clone(),
                                    births: world.entities[first_birth..]
                                        .iter()
                                        .map(|e| (e.object.clone(), e.pos))
                                        .collect(),
                                });
                        });
                    }
                }
            }
            Area::Temple => unreachable!("Temple uses its source instance pass"),
            Area::Boss => {}
        }
        if world.area == Area::Ice && !altar {
            #[cfg(test)]
            trace_population_phase(world, x, y, PopulationPhase::Treasure);
            treasure(world, rng, &water, &mut state, x, y);
        }
    }
    if udjat && !exists(world, "oLockedChest") {
        world.spawn("oLockedChest", free_near(world, world.exit));
    }
    if exists(world, "oLockedChest") && !exists(world, "oKey") {
        let candidates: Vec<_> = world
            .entities
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                matches!(
                    e.object.as_str(),
                    "oGoldBar" | "oGoldBars" | "oEmeraldBig" | "oSapphireBig" | "oRubyBig"
                ) && !world.solid_at(e.pos.x, e.pos.y)
            })
            .map(|(i, _)| i)
            .collect();
        if !candidates.is_empty() {
            let index = candidates[rng.range(candidates.len() as u32) as usize];
            let mut pos = world.entities[index].pos;
            if world.entities[index].object == "oGoldBars" {
                pos.y += 4.0;
            }
            world.entities[index].alive = false;
            world.spawn("oKey", pos);
        } else {
            world.spawn("oKey", free_near(world, world.entrance));
        }
    }
    if need_market && !exists(world, "oXMarket") {
        if let Some(&(x, y)) = supports
            .iter()
            .find(|&&(x, y)| y > 2 && solid(world, x, y - 1) && !in_shop(world, x, y))
        {
            spawn(world, rng, "oXMarket", x, y - 1);
        }
    }
    if world.area == Area::Temple && world.level == 14 && !made_gold {
        // The second `with oSolid` is a fresh pass after enemy/treasure
        // generation. It includes moving blocks and has no shop exclusion.
        if let Some((_, support, origin)) =
            family_instances(world, "oSolid")
                .into_iter()
                .find(|(_, _, origin)| {
                    origin.y > 32.0
                        && !source_point_family(
                            world,
                            Vec2::new(origin.x, origin.y - 16.0),
                            "oSolid",
                        )
                })
        {
            crate::creation::spawn_generated_entity(
                world,
                rng,
                "oGoldDoor",
                Vec2::new(origin.x, origin.y - 16.0),
            );
            // Source :491 protects exactly the selected instance, even when
            // multiple solids occupy the same origin.
            protect_gold_door_support(world, support);
        }
    }
    if matches!(world.area, Area::Mines | Area::Temple) {
        arrows(world, rng, &supports);
    }
    if world.area == Area::Jungle {
        jungle_population(world, rng, cemetery);
    }
    if world.dark {
        world.spawn("oFlareCrate", free_near(world, world.entrance));
    }
}

fn free_near(world: &World, origin: Vec2) -> Vec2 {
    for dx in [-16.0, 16.0, 0.0] {
        let p = Vec2::new(origin.x + dx, origin.y);
        if !world.solid_at(p.x, p.y) {
            return p;
        }
    }
    origin
}
fn spear(
    world: &mut World,
    rng: &mut Rng,
    water: &WaterMasks,
    x: i32,
    y: i32,
    excluded: bool,
    far: bool,
) {
    if excluded
        || !far
        || y == 0
        || solid(world, x, y - 1)
        || solid(world, x, y - 2)
        || water.wet(x, y - 1)
        || occupied(world, x, y - 1)
        || !solid(world, x, y + 1)
        || (solid(world, x - 1, y) && solid(world, x + 1, y))
    {
        return;
    }
    if world.area == Area::Temple && solid(world, x - 1, y - 2) && solid(world, x + 1, y - 2) {
        return;
    }
    // scrEntityGen:236 eagerly evaluates rand20 even when darkness or water
    // rejects Tiki; the separate spear else-if remains conditional.
    let tiki_roll = world.area == Area::Jungle && rng.chance(1, 20);
    if world.dark && !water.wet(x, y - 2) && tiki_roll {
        spawn(world, rng, "oTikiTorch", x, y - 2);
    } else if rng.chance(1, 12) && !matches!(x, 10 | 11 | 20 | 21 | 30 | 31) {
        // scrEntityGen:240-241 bans only these six raw pixel columns from
        // spear births, after their roll; Tiki and its roll remain eligible.
        spawn(world, rng, "oSpearTrapBottom", x, y);
        if let Some(e) = world.entities.last_mut() {
            e.solid = true;
        }
        spawn(
            world,
            rng,
            if world.dark {
                "oSpearTrapLit"
            } else {
                "oSpearTrapTop"
            },
            x,
            y - 1,
        );
        if let Some(e) = world.entities.last_mut() {
            e.solid = true;
        }
    }
}
struct ArrowReceiver {
    serial: u64,
    instance: FamilyInstance,
    metadata: Option<u64>,
    source: crate::Entity,
}

fn arrow_receiver(
    world: &World,
    serial: u64,
    instance: FamilyInstance,
    origin: Vec2,
) -> Option<ArrowReceiver> {
    let (source, metadata) = match instance {
        FamilyInstance::Tile(index) => {
            let tile = world.tiles.get(index)?;
            if tile.creation_serial != serial || tile.object != "oBlock" {
                return None;
            }
            let center = Vec2::new(origin.x + 8.0, origin.y + 8.0);
            let metadata = world.entities.iter().find(|entity| {
                entity.alive && entity.object == "oTerrainMetadata" && entity.pos == center
            });
            let mut source = crate::Entity::new(0, "oBlock", origin);
            source.creation_serial = serial;
            if let Some(metadata) = metadata {
                source.data = metadata.data.clone();
                source.text = metadata.text.clone();
            }
            (source, metadata.map(|entity| entity.id))
        }
        FamilyInstance::Entity(id) => {
            let source = world.entity(id)?;
            if source.creation_serial != serial || source.object != "oBlock" {
                return None;
            }
            (source.clone(), None)
        }
    };
    Some(ArrowReceiver {
        serial,
        instance,
        metadata,
        source,
    })
}

fn clean_arrow_receiver(world: &mut World, receiver: &mut ArrowReceiver) {
    receiver.source.data.insert("cleanDeath".into(), 1.0);
    match receiver.instance {
        FamilyInstance::Tile(_) => {
            let id = *receiver.metadata.get_or_insert_with(|| {
                world.spawn(
                    "oTerrainMetadata",
                    Vec2::new(receiver.source.pos.x + 8.0, receiver.source.pos.y + 8.0),
                )
            });
            if let Some(metadata) = world.entity_mut(id) {
                metadata.data.insert("cleanDeath".into(), 1.0);
            }
        }
        FamilyInstance::Entity(id) => {
            if let Some(entity) = world.entity_mut(id) {
                entity.data.insert("cleanDeath".into(), 1.0);
            }
        }
    }
}

fn retire_arrow_receiver(world: &mut World, rng: &mut Rng, receiver: &ArrowReceiver) -> bool {
    // Trap Create precedes this call. Recheck identity, never retire by origin:
    // a coincident Block or the freshly created Trap is a different receiver.
    let current = match receiver.instance {
        FamilyInstance::Tile(index) => world
            .tiles
            .get(index)
            .is_some_and(|tile| tile.creation_serial == receiver.serial && tile.object == "oBlock"),
        FamilyInstance::Entity(id) => world.entity(id).is_some_and(|entity| {
            entity.creation_serial == receiver.serial && entity.object == "oBlock"
        }),
    };
    if !current {
        return false;
    }
    // instance_destroy runs the own Destroy callback before marking the
    // original receiver destroyed. These stock births do not replace it.
    let city = world.name == "City of Gold";
    crate::items::explosives::destroy_generated_block_own(world, rng, &receiver.source, city);
    match receiver.instance {
        FamilyInstance::Tile(index) => {
            let dirty = world.fluid_dirty;
            world.set_tile(index % world.width, index / world.width, "");
            // oBlock owns a non-inheriting Destroy event: no checkWater write.
            world.fluid_dirty = dirty;
            if let Some(id) = receiver.metadata {
                if let Some(metadata) = world.entity_mut(id) {
                    metadata.alive = false;
                }
            }
        }
        FamilyInstance::Entity(id) => {
            if let Some(entity) = world.entity_mut(id) {
                entity.alive = false;
            }
        }
    }
    true
}

fn arrows(world: &mut World, rng: &mut Rng, _supports: &[(i32, i32)]) {
    // Source with(oBlock) is a fresh family pass, including stored instances.
    // No Block descendants, family births, activation or array rebuild occur
    // in these callbacks. Existing generation creation-order limits remain.
    let receivers = family_instances(world, "oBlock");
    let entrance = family_origins(world, "oEntrance").into_iter().next();
    for (serial, instance, origin) in receivers {
        let Some(mut receiver) = arrow_receiver(world, serial, instance, origin) else {
            continue;
        };
        if world.area == Area::Temple {
            // Source :500 precedes even the outer shop condition.
            clean_arrow_receiver(world, &mut receiver);
        }
        // Same literal isInShop as the Temple primary pass. Preserve its
        // None-context fallback; the campaign path is keeper-independent.
        if temple_in_shop(world, origin) {
            continue;
        }
        // Stock generated rooms always have an Entrance. Legacy isolated
        // worlds without one cannot reproduce the qualified source query;
        // retain Temple's unconditional assignment but omit probability work.
        let Some(entrance) = entrance else {
            continue;
        };
        let distance = origin.distance(entrance);
        let roll = rng.chance(1, if world.area == Area::Temple { 3 } else { 4 });
        if distance <= 48.0 || (origin.y == entrance.y && distance < 144.0) || !roll {
            continue;
        }
        let left = source_point_family(world, Vec2::new(origin.x + 16.0, origin.y), "oSolid")
            && !source_rectangle_family(
                world,
                Rect::new(origin.x - 32.0, origin.y, 32.0, 16.0),
                "oSolid",
            );
        let right = !left
            && source_point_family(world, Vec2::new(origin.x - 16.0, origin.y), "oSolid")
            && !source_rectangle_family(
                world,
                Rect::new(origin.x + 16.0, origin.y, 33.0, 16.0),
                "oSolid",
            );
        let name = match (left, right, world.dark) {
            (true, _, false) => "oArrowTrapLeft",
            (true, _, true) => "oArrowTrapLeftLit",
            (_, true, false) => "oArrowTrapRight",
            (_, true, true) => "oArrowTrapRightLit",
            _ => continue,
        };
        let trap = crate::creation::spawn_generated_entity(world, rng, name, origin);
        if let Some(trap) = world.entity_mut(trap) {
            trap.solid = true;
        }
        retire_arrow_receiver(world, rng, &receiver);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn literal_jungle_column_statement(
        source: &mut Rng,
        raw_x: i32,
        raw_y: i32,
        dark: bool,
        upper_water: bool,
    ) -> Vec<(&'static str, Vec2)> {
        // Independent transcription of scrEntityGen:236-250. Explicit raw
        // pixel exclusions do not reuse the grid/modulo implementation.
        let tiki_roll = source.range(20);
        if dark && !upper_water && tiki_roll == 0 {
            return vec![("oTikiTorch", Vec2::new(raw_x.into(), (raw_y - 32).into()))];
        }
        let spear_roll = source.range(12);
        if spear_roll == 0 && ![160, 176, 320, 336, 480, 496].contains(&raw_x) {
            vec![
                ("oSpearTrapBottom", Vec2::new(raw_x.into(), raw_y.into())),
                (
                    if dark {
                        "oSpearTrapLit"
                    } else {
                        "oSpearTrapTop"
                    },
                    Vec2::new(raw_x.into(), (raw_y - 16).into()),
                ),
            ]
        } else {
            Vec::new()
        }
    }

    #[test]
    fn jungle_column_all_grid_positions_match_raw_source_statement_tapes() {
        let mut tiki_births = 0;
        let mut spear_births = 0;
        let mut forbidden_spear_wins = 0;
        let mut extra_edge_spears = 0;
        for x in 0..42 {
            for dark in [false, true] {
                for upper_water in [false, true] {
                    //29 wins Tiki;0 wins fallback Spear;19 loses both.
                    // These are guard-entry port RNG states, not level seeds.
                    for seed in [29, 0, 19] {
                        let mut world = World::new(42, 34, Area::Jungle, 5);
                        world.dark = dark;
                        world.set_tile(x as usize, 5, "oLush");
                        world.set_tile(x as usize, 6, "oLush");
                        if upper_water {
                            world.set_tile(x as usize, 3, "oWaterSwim");
                        }
                        let water = WaterMasks::new(&world);
                        assert_eq!(water.wet(x, 3), upper_water);
                        assert!(!water.wet(x, 4));
                        let mut actual = Rng::new(seed);
                        let mut source = actual.clone();
                        let expected = literal_jungle_column_statement(
                            &mut source,
                            x * 16,
                            80,
                            dark,
                            upper_water,
                        );
                        spear(&mut world, &mut actual, &water, x, 5, false, true);
                        let births: Vec<_> = world
                            .entities
                            .iter()
                            .map(|e| (e.object.as_str(), e.pos))
                            .collect();
                        assert_eq!(
                            births, expected,
                            "x={x} dark={dark} upper_water={upper_water} entry_seed={seed}"
                        );
                        assert_eq!(
                            actual.next_u64(),
                            source.next_u64(),
                            "statement RNG x={x} dark={dark} upper_water={upper_water} entry_seed={seed}"
                        );
                        if expected.first().is_some_and(|e| e.0 == "oTikiTorch") {
                            tiki_births += 1;
                        } else if !expected.is_empty() {
                            spear_births += 1;
                            if matches!(x, 1 | 40) {
                                extra_edge_spears += 1;
                            }
                        } else if [160, 176, 320, 336, 480, 496].contains(&(x * 16)) && seed == 0 {
                            forbidden_spear_wins += 1;
                        }
                    }
                }
            }
        }
        assert!(tiki_births > 0 && spear_births > 0);
        assert!(forbidden_spear_wins > 0 && extra_edge_spears > 0);
    }

    #[test]
    fn jungle_column_outer_rejection_still_skips_all_inner_rolls() {
        for x in [1, 10, 11, 20, 21, 30, 31, 40] {
            for (excluded, far) in [(true, true), (false, false)] {
                let mut world = World::new(42, 34, Area::Jungle, 5);
                world.dark = true;
                world.set_tile(x as usize, 5, "oLush");
                world.set_tile(x as usize, 6, "oLush");
                let water = WaterMasks::new(&world);
                let mut actual = Rng::new(29);
                let mut unchanged = actual.clone();
                spear(&mut world, &mut actual, &water, x, 5, excluded, far);
                assert!(world.entities.is_empty());
                assert_eq!(actual.next_u64(), unchanged.next_u64());
            }
        }
    }

    #[test]
    fn historical_jungle_column_dark_seed86_keeps_tiki_at_spear_banned_column() {
        // Historical SplitMix layout counterexample, fixed from a bounded
        // real generate(0..512, Jungle5, default flags) discovery. Seed27 was
        // the historical counterexample before the
        // corrected InitLevel Udjat/Market draws shifted later population RNG.
        // This is an unmodified natural layout/darkness selection,
        // not a canonical seed-layout claim or a carved room fixture.
        // Canonical authored-column controls separately preserve branch coverage.
        // scrEntityGen:236 permits Tiki at rawx496; only the separate :240
        // spear is banned.
        // The generated support satisfies the source side-clearance OR.
        let world = super::super::generate(&mut Rng::legacy(86), 5, &BTreeSet::new());
        assert!(world.dark, "darkness is selected naturally for this seed");
        assert_eq!(world.tile(31, 31).unwrap().object, "oLush");
        assert_eq!(world.tile(31, 32).unwrap().object, "oLush");
        for y in [29, 30] {
            assert!(!world.tiles_point(Vec2::new(496.0, f64::from(y * 16)), "oSolid"));
        }
        assert!(
            !world.tile(30, 31).unwrap().is_solid() || !world.tile(32, 31).unwrap().is_solid(),
            "source requires at least one open side"
        );
        assert!(
            world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oTikiTorch" && e.pos == Vec2::new(496.0, 464.0)),
            "source Tiki is not restricted by the spear-only rawx496 ban"
        );
        assert!(
            !world.entities.iter().any(|e| e.alive
                && e.object == "oSpearTrapBottom"
                && e.pos == Vec2::new(496.0, 496.0)),
            "successful Tiki stops the separate spear statement"
        );
    }

    fn arrow_source_room(area: Area, dynamic: bool, right: bool) -> (World, Option<u64>) {
        let mut world = World::new(42, 34, area, if area == Area::Mines { 1 } else { 13 });
        world.entrance = at(1, 1);
        world.spawn("oEntrance", Vec2::new(16.0, 16.0));
        world.set_tile(if right { 19 } else { 21 }, 5, "oHardBlock");
        let id = if dynamic {
            Some(world.spawn("oBlock", Vec2::new(320.0, 80.0)))
        } else {
            world.set_tile(20, 5, "oBlock");
            None
        };
        (world, id)
    }

    fn arrow_source_metadata(world: &mut World, dynamic: Option<u64>) -> u64 {
        dynamic.unwrap_or_else(|| world.spawn("oTerrainMetadata", at(20, 5)))
    }

    fn arrow_source_winning_seed(area: Area) -> u32 {
        // Independent canonical guard-entry controls from the unsigned LCG:
        // Mines0 wins, consumes twelve debris draws, then loses on a second
        // receiver. Temple1 wins then loses, with no clean-death debris draws.
        if area == Area::Mines { 0 } else { 1 }
    }

    #[test]
    fn source_arrow_near_then_far_canonical_control_consumes_protected_guard() {
        let mut world = World::new(42, 34, Area::Mines, 1);
        world.entrance = at(1, 1);
        world.spawn("oEntrance", Vec2::new(16.0, 16.0));
        world.set_tile(3, 1, "oBlock");
        world.set_tile(11, 1, "oBlock");
        world.set_tile(12, 1, "oHardBlock");
        // Independent canonical seed4 yields range4=[0,1]: the near
        // protected receiver consumes its winning roll before the far loss.
        let mut actual = Rng::new(4);
        let mut expected = actual.clone();
        assert_eq!(1 + expected.range(4), 1);
        assert_eq!(1 + expected.range(4), 2);
        arrows(&mut world, &mut actual, &[(3, 1), (11, 1), (12, 1)]);
        assert!(
            !world
                .entities
                .iter()
                .any(|e| e.object.starts_with("oArrowTrap"))
        );
        assert_eq!(world.tile(3, 1).unwrap().object, "oBlock");
        assert_eq!(world.tile(11, 1).unwrap().object, "oBlock");
        assert_eq!(actual.next_u64(), expected.next_u64());
    }

    #[test]
    fn source_arrow_success_creates_trap_then_exact_block_own_destroy() {
        for area in [Area::Mines, Area::Temple] {
            for dynamic in [false, true] {
                for right in [false, true] {
                    for dark in [false, true] {
                        for prior_clean in [false, true] {
                            for dirty in [false, true] {
                                let (mut world, receiver) = arrow_source_room(area, dynamic, right);
                                world.dark = dark;
                                let metadata = arrow_source_metadata(&mut world, receiver);
                                let source = world.entity_mut(metadata).unwrap();
                                source
                                    .data
                                    .insert("cleanDeath".into(), f64::from(prior_clean));
                                source.data.insert("invincible".into(), 1.0);
                                source.data.insert("shopWall".into(), 1.0);
                                // Block's own event does not inherit support cleanup.
                                let spikes = world.spawn("oSpikes", Vec2::new(320.0, 64.0));
                                world.fluid_dirty = dirty;
                                let mut actual = Rng::new(arrow_source_winning_seed(area));
                                let mut expected = actual.clone();
                                assert_eq!(
                                    expected.range(if area == Area::Mines { 4 } else { 3 }),
                                    0
                                );
                                let debris = area == Area::Mines && !prior_clean;
                                let mut positions = Vec::new();
                                let mut birth_rng = Vec::new();
                                if debris {
                                    for _ in 0..3 {
                                        positions.push(Vec2::new(
                                            328.0 + f64::from(expected.range(9))
                                                - f64::from(expected.range(9)),
                                            88.0 + f64::from(expected.range(9))
                                                - f64::from(expected.range(9)),
                                        ));
                                        birth_rng.push(expected.clone().next_u64());
                                    }
                                }
                                crate::creation::begin_generated_create_trace();
                                arrows(&mut world, &mut actual, &[(20, 5)]);
                                let trace = crate::creation::take_generated_create_traces();
                                assert_eq!(trace.len(), 1 + if debris { 3 } else { 0 });
                                let expected_trap = match (right, dark) {
                                    (false, false) => "oArrowTrapLeft",
                                    (false, true) => "oArrowTrapLeftLit",
                                    (true, false) => "oArrowTrapRight",
                                    (true, true) => "oArrowTrapRightLit",
                                };
                                let trap = &trace[0].entity_after;
                                assert_eq!(trap.object, expected_trap);
                                assert_eq!(trap.pos, Vec2::new(320.0, 80.0));
                                assert_eq!(trap.alarms[1], 1);
                                assert_eq!(trap.value("invincible"), 0.0);
                                assert_eq!(trap.value("cleanDeath"), 0.0);
                                assert_eq!(trap.value("shopWall"), 0.0);
                                assert!(world.entity(metadata).is_none());
                                assert!(world.tile(20, 5).unwrap().object.is_empty());
                                assert!(world.entity(spikes).is_some());
                                assert_eq!(world.fluid_dirty, dirty);
                                for (index, ((birth, pos), marker)) in trace
                                    .iter()
                                    .skip(1)
                                    .zip(positions)
                                    .zip(birth_rng)
                                    .enumerate()
                                {
                                    assert_eq!(
                                        birth.entity_after.object,
                                        if index == 0 {
                                            "oRubble"
                                        } else {
                                            "oRubbleSmall"
                                        }
                                    );
                                    assert_eq!(birth.entity_after.pos, pos);
                                    assert_eq!(birth.entity_after.vel, Vec2::default());
                                    assert_eq!(birth.entity_after.value("gravity"), 0.6);
                                    assert_eq!(birth.rng_before.clone().next_u64(), marker);
                                    let final_entity = world.entity(birth.entity_after.id).unwrap();
                                    assert_eq!(
                                        final_entity.sprite,
                                        if index == 0 {
                                            "sRubbleLush"
                                        } else {
                                            "sRubbleLushSmall"
                                        }
                                    );
                                }
                                assert_eq!(actual.next_u64(), expected.next_u64());
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn source_arrow_temple_clean_assignment_survives_every_failed_guard() {
        for dynamic in [false, true] {
            for reason in ["shop", "near", "losing_roll", "geometry"] {
                let (mut world, receiver) = arrow_source_room(Area::Temple, dynamic, false);
                let metadata = arrow_source_metadata(&mut world, receiver);
                if reason == "shop" {
                    world.spawn("oShopkeeper", Vec2::new(320.0, 80.0));
                } else if reason == "near" {
                    world
                        .entities
                        .iter_mut()
                        .find(|e| e.object == "oEntrance")
                        .unwrap()
                        .pos = Vec2::new(304.0, 80.0);
                } else if reason == "geometry" {
                    world.set_tile(21, 5, "");
                }
                let mut actual = Rng::new(if reason == "losing_roll" { 29 } else { 11 });
                let mut expected = actual.clone();
                if reason != "shop" {
                    expected.range(3);
                }
                arrows(&mut world, &mut actual, &[(20, 5)]);
                assert_eq!(
                    world.entity(metadata).unwrap().value("cleanDeath"),
                    1.0,
                    "{reason}"
                );
                assert!(
                    !world
                        .entities
                        .iter()
                        .any(|e| e.object.starts_with("oArrowTrap"))
                );
                assert_eq!(actual.next_u64(), expected.next_u64());
            }
        }
    }

    #[test]
    fn source_arrow_creation_order_retires_only_selected_coincident_receiver() {
        for area in [Area::Mines, Area::Temple] {
            let (mut world, _) = arrow_source_room(area, true, false);
            let selected = world
                .entities
                .iter()
                .find(|e| e.object == "oBlock")
                .unwrap()
                .id;
            // Dynamic Block was born before this coincident tile. This is the
            // opposite of storage-form iteration order, and must stay so.
            world.set_tile(20, 5, "oBlock");
            let metadata = world.spawn("oTerrainMetadata", at(20, 5));
            let untouched_metadata = world.spawn("oTerrainMetadata", at(20, 5));
            let ruby = world.spawn("oRubyBig", at(20, 5));
            let mut actual = Rng::new(arrow_source_winning_seed(area));
            let mut expected = actual.clone();
            assert_eq!(expected.range(if area == Area::Mines { 4 } else { 3 }), 0);
            if area == Area::Mines {
                for _ in 0..12 {
                    expected.range(9);
                }
            }
            assert_ne!(expected.range(if area == Area::Mines { 4 } else { 3 }), 0);
            arrows(&mut world, &mut actual, &[(20, 5)]);
            assert!(world.entity(selected).is_none());
            assert_eq!(world.tile(20, 5).unwrap().object, "oBlock");
            assert!(world.entity(metadata).is_some());
            assert!(world.entity(untouched_metadata).is_some());
            assert!(world.entity(ruby).is_some());
            if area == Area::Temple {
                assert_eq!(world.entity(metadata).unwrap().value("cleanDeath"), 1.0);
                assert_eq!(
                    world
                        .entity(untouched_metadata)
                        .unwrap()
                        .value("cleanDeath"),
                    0.0
                );
            }
            assert_eq!(
                world
                    .entities
                    .iter()
                    .filter(|e| e.alive && e.object.starts_with("oArrowTrap"))
                    .count(),
                1
            );
            assert_eq!(actual.next_u64(), expected.next_u64());
        }
    }

    #[test]
    fn source_arrow_preserves_concrete_deferred_contents_and_ignores_block_labels() {
        for area in [Area::Mines, Area::Temple] {
            for treasure in ["Big Ruby", "oRubyBig", "oJetpack"] {
                let (mut world, _) = arrow_source_room(area, false, false);
                let metadata = world.spawn("oTerrainMetadata", at(20, 5));
                let source = world.entity_mut(metadata).unwrap();
                source.data.insert("embedded_treasure".into(), 1.0);
                source.data.insert("treasure_dx".into(), -2.0);
                source.data.insert("treasure_dy".into(), 3.0);
                source.text.insert("treasure".into(), treasure.into());
                let existing = world.spawn("oRubyBig", Vec2::new(328.0, 88.0));
                let mut actual = Rng::new(arrow_source_winning_seed(area));
                let mut expected = actual.clone();
                expected.range(if area == Area::Mines { 4 } else { 3 });
                if area == Area::Mines {
                    for _ in 0..12 {
                        expected.range(9);
                    }
                }
                arrows(&mut world, &mut actual, &[(20, 5)]);
                assert!(world.entity(existing).is_some());
                let new_items: Vec<_> = world
                    .entities
                    .iter()
                    .filter(|e| {
                        e.alive
                            && e.id != existing
                            && matches!(e.object.as_str(), "oRubyBig" | "oJetpack")
                    })
                    .collect();
                assert_eq!(new_items.len(), usize::from(treasure.starts_with('o')));
                if treasure.starts_with('o') {
                    assert_eq!(new_items[0].object, treasure);
                    assert_eq!(new_items[0].pos, Vec2::new(326.0, 91.0));
                }
                assert_eq!(actual.next_u64(), expected.next_u64());
            }
        }
    }

    #[test]
    fn source_arrow_city_gold_create_precedes_each_launch_rng() {
        let (mut world, _) = arrow_source_room(Area::Mines, false, false);
        world.name = "City of Gold".into(); // Controlled global.cityOfGold=true source branch.
        let mut actual = Rng::new(6);
        let mut expected = actual.clone();
        assert_eq!(expected.range(4), 0);
        let mut expected_gold = Vec::new();
        for _ in 0..4 {
            let pos = Vec2::new(
                328.0 + f64::from(expected.range(5)) - f64::from(expected.range(5)),
                88.0 + f64::from(expected.range(5)) - f64::from(expected.range(5)),
            );
            let birth = expected.clone().next_u64();
            let velocity = Vec2::new(
                f64::from(expected.range(4)) - f64::from(expected.range(4)),
                2.0 + f64::from(expected.range(3)),
            );
            expected_gold.push((pos, birth, velocity));
        }
        crate::creation::begin_generated_create_trace();
        arrows(&mut world, &mut actual, &[(20, 5)]);
        let trace = crate::creation::take_generated_create_traces();
        assert_eq!(trace.len(), 5);
        for (index, (birth, (pos, marker, velocity))) in
            trace.iter().skip(1).zip(expected_gold).enumerate()
        {
            assert_eq!(
                birth.entity_after.object,
                if index == 3 {
                    "oGoldNugget"
                } else {
                    "oGoldChunk"
                }
            );
            assert_eq!(birth.entity_after.pos, pos);
            assert_eq!(birth.rng_before.clone().next_u64(), marker);
            assert_eq!(birth.entity_after.vel, Vec2::default());
            assert_eq!(world.entity(birth.entity_after.id).unwrap().vel, velocity);
        }
        assert_eq!(actual.next_u64(), expected.next_u64());
    }

    #[test]
    fn source_arrow_generated_world_has_replacement_not_overlaid_block_and_source_birth_tape() {
        let flags = BTreeSet::from(["no_dark_level".into()]);
        let mut rng = Rng::new(7);
        crate::creation::begin_generated_create_trace();
        let world = super::super::generate(&mut rng, 1, &flags);
        let trace = crate::creation::take_generated_create_traces();
        let mut traps = 0;
        for (index, birth) in trace.iter().enumerate() {
            if !birth.entity_after.object.starts_with("oArrowTrap") {
                continue;
            }
            traps += 1;
            let origin = birth.entity_after.pos;
            assert_ne!(
                world
                    .tile((origin.x / 16.0) as i32, (origin.y / 16.0) as i32)
                    .unwrap()
                    .object,
                "oBlock"
            );
            let mut oracle = birth.rng_before.clone();
            for fragment in 0..3 {
                let pos = Vec2::new(
                    origin.x + 8.0 + f64::from(oracle.range(9)) - f64::from(oracle.range(9)),
                    origin.y + 8.0 + f64::from(oracle.range(9)) - f64::from(oracle.range(9)),
                );
                let rubble = &trace[index + 1 + fragment];
                assert_eq!(
                    rubble.entity_after.object,
                    if fragment == 0 {
                        "oRubble"
                    } else {
                        "oRubbleSmall"
                    }
                );
                assert_eq!(rubble.entity_after.pos, pos);
                assert_eq!(
                    rubble.rng_before.clone().next_u64(),
                    oracle.clone().next_u64()
                );
            }
        }
        assert!(
            traps > 0,
            "natural seed7 must exercise the stock Arrow pass"
        );
        let mut other_rng = Rng::new(7);
        let other_flags = BTreeSet::from(["no_dark_level".into(), "cleanSolids".into()]);
        let other = super::super::generate(&mut other_rng, 1, &other_flags);
        assert_eq!(
            serde_json::to_value(&world).unwrap(),
            serde_json::to_value(&other).unwrap(),
            "global.cleanSolids is absent from Block's own Destroy guard"
        );
        assert_eq!(rng.next_u64(), other_rng.next_u64());
    }

    #[test]
    fn source_arrow_dead_keeper_does_not_gate_and_frozen_contents_do_not_duplicate() {
        for area in [Area::Mines, Area::Temple] {
            let (mut world, _) = arrow_source_room(area, false, false);
            let keeper = world.spawn("oShopkeeper", Vec2::new(320.0, 80.0));
            world.entity_mut(keeper).unwrap().alive = false;
            let metadata = world.spawn("oTerrainMetadata", at(20, 5));
            let source = world.entity_mut(metadata).unwrap();
            source.data.insert("embedded_treasure".into(), 1.0);
            source.data.insert("frozen_caveman".into(), 1.0);
            source.text.insert("treasure".into(), "oCaveman".into());
            let frozen = world.spawn("oFrozenCaveman", at(20, 5));
            let mut actual = Rng::new(arrow_source_winning_seed(area));
            let mut oracle = actual.clone();
            oracle.range(if area == Area::Mines { 4 } else { 3 });
            if area == Area::Mines {
                for _ in 0..12 {
                    oracle.range(9);
                }
            }
            arrows(&mut world, &mut actual, &[(20, 5)]);
            assert_eq!(world.tile(20, 5).unwrap().object, "");
            assert!(
                world
                    .entities
                    .iter()
                    .any(|e| e.alive && e.object == "oArrowTrapLeft")
            );
            assert!(world.entity(frozen).is_some());
            assert!(!world.entities.iter().any(|e| e.object == "oCaveman"));
            assert_eq!(actual.next_u64(), oracle.next_u64());
        }
    }

    #[test]
    fn source_arrow_uses_first_raw_entrance_and_exact_fractional_row() {
        for same_row in [false, true] {
            let (mut world, receiver) = arrow_source_room(Area::Mines, true, false);
            let entrance = world
                .entities
                .iter_mut()
                .find(|e| e.object == "oEntrance")
                .unwrap();
            entrance.pos = Vec2::new(200.0, 80.0);
            // Later Entrance and the convenience center must not change the
            // qualified first-instance source distance/row query.
            world.spawn("oEntrance", Vec2::new(320.0, 80.0));
            world.entrance = Vec2::new(328.0, 88.0);
            if !same_row {
                world.entity_mut(receiver.unwrap()).unwrap().pos.y += 0.5;
            }
            let mut actual = Rng::new(6);
            let mut oracle = actual.clone();
            assert_eq!(oracle.range(4), 0);
            if !same_row {
                for _ in 0..12 {
                    oracle.range(9);
                }
            }
            arrows(&mut world, &mut actual, &[(20, 5)]);
            assert_eq!(world.entity(receiver.unwrap()).is_some(), same_row);
            let trap = world
                .entities
                .iter()
                .find(|e| e.alive && e.object == "oArrowTrapLeft");
            assert_eq!(trap.is_some(), !same_row);
            if let Some(trap) = trap {
                assert_eq!(trap.pos, Vec2::new(320.0, 80.5));
            }
            assert_eq!(actual.next_u64(), oracle.next_u64());
        }
    }

    #[test]
    fn source_arrow_temple_shop_survivor_gains_missing_clean_metadata() {
        let (mut world, _) = arrow_source_room(Area::Temple, false, false);
        world.spawn("oShopkeeper", Vec2::new(320.0, 80.0));
        let mut actual = Rng::new(11);
        let mut oracle = actual.clone();
        arrows(&mut world, &mut actual, &[(20, 5)]);
        assert_eq!(world.tile(20, 5).unwrap().object, "oBlock");
        let metadata = world
            .entities
            .iter()
            .find(|e| e.alive && e.object == "oTerrainMetadata" && e.pos == at(20, 5))
            .unwrap();
        assert_eq!(metadata.value("cleanDeath"), 1.0);
        assert_eq!(actual.next_u64(), oracle.next_u64());
    }

    fn jungle_spear_guard_fixture(dark: bool, upper_water: bool) -> World {
        let mut world = World::new(42, 34, Area::Jungle, 5);
        world.dark = dark;
        world.set_tile(5, 5, "oLush");
        world.set_tile(5, 6, "oLush");
        if upper_water {
            world.set_tile(5, 3, "oWaterSwim");
        }
        world
    }

    #[test]
    fn jungle_spear_guard_bright_room_consumes_tiki_roll_before_spear() {
        // scrEntityGen:236-241 after the outer candidate statement matched.
        // GM8 evaluates rand20 even when darkLevel is false, then enters the
        // separate else-if rand12 statement. Canonical seed2 is guard-entry
        // RNG state, not a generated-world seed or complete GM stream claim.
        let mut world = jungle_spear_guard_fixture(false, false);
        let water = WaterMasks::new(&world);
        let mut actual = Rng::new(2);
        let mut source = actual.clone();
        assert_eq!(source.range(20), 1);
        assert_eq!(source.range(12), 8);
        spear(&mut world, &mut actual, &water, 5, 5, false, true);
        assert!(
            world.entities.is_empty(),
            "neither source roll creates a trap"
        );
        assert_eq!(actual.next_u64(), source.next_u64());
    }

    #[test]
    fn jungle_spear_guard_upper_water_still_consumes_tiki_roll() {
        // Water at y-32 rejects only Tiki; y-16 stays dry so the outer
        // spear-candidate statement is eligible. The false water operand
        // must not suppress the eager rand20 or its subsequent rand12.
        let mut world = jungle_spear_guard_fixture(true, true);
        let water = WaterMasks::new(&world);
        assert!(water.wet(5, 3), "upper water must have a source mask");
        assert!(!water.wet(5, 4), "outer spear cell stays dry");
        let mut actual = Rng::new(2);
        let mut source = actual.clone();
        assert_eq!(source.range(20), 1);
        assert_eq!(source.range(12), 8);
        spear(&mut world, &mut actual, &water, 5, 5, false, true);
        assert!(world.entities.is_empty());
        assert_eq!(actual.next_u64(), source.next_u64());
    }

    #[test]
    fn jungle_spear_guard_tiki_success_skips_separate_spear_statement() {
        let mut world = jungle_spear_guard_fixture(true, false);
        let water = WaterMasks::new(&world);
        // Canonical seed0 wins the first Tiki guard; no Spear draw follows.
        let mut actual = Rng::new(0);
        let mut source = actual.clone();
        assert_eq!(source.range(20), 0);
        spear(&mut world, &mut actual, &water, 5, 5, false, true);
        assert_eq!(world.entities.len(), 1);
        assert_eq!(world.entities[0].object, "oTikiTorch");
        assert_eq!(world.entities[0].pos, Vec2::new(80.0, 48.0));
        // Tiki Create only sets image_speed; it adds no RNG draw.
        assert_eq!(actual.next_u64(), source.next_u64());
    }

    #[test]
    fn jungle_spear_guard_fallback_creates_ordered_bright_and_lit_segments() {
        for dark in [false, true] {
            let mut world = jungle_spear_guard_fixture(dark, false);
            let water = WaterMasks::new(&world);
            // Independently frozen canonical range20/12=[3,0].
            let mut actual = Rng::new(6);
            let mut source = actual.clone();
            assert_eq!(source.range(20), 3);
            assert_eq!(source.range(12), 0);
            spear(&mut world, &mut actual, &water, 5, 5, false, true);
            let births: Vec<_> = world
                .entities
                .iter()
                .map(|e| (e.object.as_str(), e.pos))
                .collect();
            assert_eq!(
                births,
                vec![
                    ("oSpearTrapBottom", Vec2::new(80.0, 80.0)),
                    (
                        if dark {
                            "oSpearTrapLit"
                        } else {
                            "oSpearTrapTop"
                        },
                        Vec2::new(80.0, 64.0)
                    ),
                ]
            );
            assert!(world.entities[0].creation_serial < world.entities[1].creation_serial);
            // These source segment Create bodies introduce no random draws.
            assert_eq!(actual.next_u64(), source.next_u64());
        }
    }

    #[test]
    fn jungle_spear_guard_rejected_outer_statement_consumes_no_inner_rolls() {
        for (excluded, far) in [(true, true), (false, false)] {
            let mut world = jungle_spear_guard_fixture(false, false);
            let water = WaterMasks::new(&world);
            let mut actual = Rng::new(19);
            let mut source = actual.clone();
            spear(&mut world, &mut actual, &water, 5, 5, excluded, far);
            assert!(world.entities.is_empty());
            assert_eq!(actual.next_u64(), source.next_u64());
        }
    }

    #[test]
    fn mines_ceiling_eager_seed39_literal_tape_preserves_bright_and_dark_guards() {
        // scrEntityGen:53-65: within an eligible ceiling statement, each
        // reached condition evaluates its roll even if an earlier flag is false.
        // This is a guard-entry RNG state, not a generated-level seed.
        for giant_enabled in [false, true] {
            for dark in [false, true] {
                let mut world = World::new(42, 34, Area::Mines, 1);
                world.dark = dark;
                // Independently frozen canonical seed39: five failed guards,
                // exact literal rolls and tail; original no-birth checks remain.
                let mut actual = Rng::new(39);
                let mut oracle = actual.clone();
                for (denominator, literal_value) in [(40, 9), (60, 24), (40, 8), (60, 57), (80, 66)]
                {
                    assert_eq!(1 + oracle.range(denominator), literal_value);
                }
                mines_ceiling(&mut world, &mut actual, giant_enabled, 20, 5);
                assert!(
                    world.entities.is_empty(),
                    "giant={giant_enabled} dark={dark}"
                );
                assert_eq!(
                    actual.next_u64(),
                    oracle.next_u64(),
                    "giant={giant_enabled} dark={dark}"
                );
            }
        }
    }

    #[test]
    fn mines_ceiling_eager_lamp_success_stops_later_statement_rolls() {
        let mut world = World::new(42, 34, Area::Mines, 1);
        world.dark = true;
        // Canonical seed6 independently yields the two-guard tape [8,1].
        let mut actual = Rng::new(6);
        let mut oracle = actual.clone();
        assert_eq!(1 + oracle.range(40), 8); // giant flag false, still evaluated
        assert_eq!(1 + oracle.range(60), 1); // Lamp succeeds: later else-if bodies wait
        mines_ceiling(&mut world, &mut actual, false, 20, 5);
        assert_eq!(world.entities.len(), 1);
        assert_eq!(world.entities[0].object, "oLamp");
        assert_eq!(world.entities[0].pos, Vec2::new(320., 96.));
        assert_eq!(actual.next_u64(), oracle.next_u64());
    }

    #[test]
    fn mines_ceiling_eager_giant_success_stops_all_later_rolls() {
        let mut world = World::new(42, 34, Area::Mines, 1);
        world.dark = true;
        // Canonical seed0 wins the first guard and suppresses later draws.
        let mut actual = Rng::new(0);
        let mut oracle = actual.clone();
        assert_eq!(1 + oracle.range(40), 1);
        mines_ceiling(&mut world, &mut actual, true, 20, 5);
        assert!(exists(&world, "oGiantSpiderHang"));
        assert_eq!(actual.next_u64(), oracle.next_u64());
    }

    #[test]
    fn mines_ceiling_eager_existing_or_blocked_giant_still_consumes_first_roll() {
        for already_exists in [false, true] {
            let mut world = World::new(42, 34, Area::Mines, 1);
            if already_exists {
                world.spawn("oGiantSpiderHang", Vec2::new(64., 64.));
            } else {
                world.set_tile(21, 6, "oBlock");
            }
            let before = world.entities.len();
            // The canonical seed39 tape reaches all five losing conditions.
            let mut actual = Rng::new(39);
            let mut oracle = actual.clone();
            for (denominator, literal_value) in [(40, 9), (60, 24), (40, 8), (60, 57), (80, 66)] {
                assert_eq!(1 + oracle.range(denominator), literal_value);
            }
            mines_ceiling(&mut world, &mut actual, true, 20, 5);
            assert_eq!(world.entities.len(), before);
            assert_eq!(actual.next_u64(), oracle.next_u64());
        }
    }

    #[test]
    fn generated_temple_primary_receiver_callbacks_accept_stock_templates() {
        for level in [13, 14, 15] {
            for seed in [0, 1, 2, 3, 7, 29, 42, 6491] {
                TEMPLE_RECEIVER_TRACE.with(|trace| *trace.borrow_mut() = Some(Vec::new()));
                let world = super::super::generate(&mut Rng::new(seed), level, &BTreeSet::new());
                let visits = TEMPLE_RECEIVER_TRACE.with(|trace| trace.borrow_mut().take().unwrap());
                assert!(visits.len() >= 100, "level={level} seed={seed}");
                assert!(
                    visits.windows(2).all(|w| w[0].0 < w[1].0),
                    "level={level} seed={seed}"
                );
                assert_eq!(world.area, Area::Temple);
                if level == 14 {
                    assert!(exists(&world, "oGoldDoor"));
                }
            }
        }
    }

    #[test]
    fn temple_primary_statue_roll_precedes_shop_gate_even_in_low_graphics() {
        for low_graphics in [false, true] {
            let mut world = World::new(42, 34, Area::Temple, 13);
            world.set_tile(20, 20, "oTemple");
            world.spawn("oShopkeeper", Vec2::new(320.0, 320.0));
            // Canonical seed0 independently wins the first range100 guard.
            let mut actual = Rng::new(0);
            let mut oracle = actual.clone();
            // Literal :395 precedes :397. The shop suppresses every later
            // receiver statement, so this source-statement oracle is complete.
            assert_eq!(oracle.range(100), 0);
            let mut flags = entity_gen_only_flags();
            if low_graphics {
                flags.insert("low_graphics".into());
            }
            populate(&mut world, &mut actual, &flags);
            let statues: Vec<_> = world
                .entities
                .iter()
                .filter(|e| e.sprite == "bgStatues")
                .collect();
            assert_eq!(statues.len(), 1);
            assert_eq!(statues[0].pos, Vec2::new(320.0, 288.0));
            assert_eq!(statues[0].value("source_w"), 16.0);
            assert_eq!(statues[0].value("source_h"), 48.0);
            assert_eq!(statues[0].value("depth"), 9005.0);
            assert_eq!(actual.next_u64(), oracle.next_u64());
        }
    }

    #[test]
    fn temple_primary_visits_dynamic_coincident_perimeter_and_reverse_births() {
        let mut world = World::new(42, 34, Area::Temple, 13);
        let positions = [
            Vec2::new(320.0, 512.0),
            Vec2::new(160.5, 512.25),
            Vec2::new(160.5, 512.25),
            Vec2::new(0.0, 0.0),
            Vec2::new(160.0, 512.0),
        ];
        world.set_tile(20, 32, "oTemple");
        world.spawn("oPushBlock", positions[1]);
        world.spawn("oPushBlock", positions[2]);
        world.set_tile(0, 0, "oTemple");
        world.set_tile(10, 32, "oTemple");
        for position in [positions[0], positions[1], positions[3]] {
            world.spawn("oShopkeeper", position);
        }
        let mut actual = Rng::new(29);
        let mut oracle = actual.clone();
        for _ in 0..5 {
            oracle.range(100);
        }
        TEMPLE_RECEIVER_TRACE.with(|trace| *trace.borrow_mut() = Some(Vec::new()));
        populate(&mut world, &mut actual, &entity_gen_only_flags());
        let visits = TEMPLE_RECEIVER_TRACE.with(|trace| trace.borrow_mut().take().unwrap());
        assert_eq!(
            visits,
            positions
                .into_iter()
                .enumerate()
                .map(|(i, p)| (i as u64 + 1, p))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn temple_primary_spear_replaces_exact_receiver_and_excludes_newborns() {
        // Search the literal local prefix: statue chance, spear chance. No
        // earlier receiver exists and the source spear guards all pass here.
        let seed = (0..100_000)
            .find(|&seed| {
                let mut rng = Rng::new(seed);
                rng.range(100);
                rng.range(12) == 0
            })
            .unwrap();
        let mut world = World::new(42, 34, Area::Temple, 13);
        world.entrance = Vec2::new(24.0, 24.0);
        world.exit = Vec2::new(616.0, 504.0);
        world.set_tile(12, 10, "oTemple");
        world.set_tile(12, 11, "oTemple");
        world.spawn("oEntrance", Vec2::new(16.0, 16.0));
        TEMPLE_RECEIVER_TRACE.with(|trace| *trace.borrow_mut() = Some(Vec::new()));
        POPULATION_PHASE_TRACE.with(|trace| *trace.borrow_mut() = Some(Vec::new()));
        populate(&mut world, &mut Rng::new(seed), &entity_gen_only_flags());
        let visits = TEMPLE_RECEIVER_TRACE.with(|trace| trace.borrow_mut().take().unwrap());
        let phases = POPULATION_PHASE_TRACE.with(|trace| trace.borrow_mut().take().unwrap());
        assert_eq!(world.tile(12, 10).unwrap().object, "");
        assert!(world.entities.iter().any(|e| e.alive
            && e.object == "oSpearTrapBottom"
            && e.pos == Vec2::new(192.0, 160.0)));
        assert!(
            world.entities.iter().any(|e| e.alive
                && e.object == "oSpearTrapTop"
                && e.pos == Vec2::new(192.0, 144.0))
        );
        assert_eq!(
            visits,
            vec![(1, Vec2::new(192.0, 160.0)), (2, Vec2::new(192.0, 176.0))]
        );
        assert_eq!(
            &phases[..2],
            &[
                (13, 12, 10, PopulationPhase::Enemies),
                (13, 12, 10, PopulationPhase::Treasure)
            ]
        );
    }

    #[test]
    fn temple_spear_clean_destroy_preserves_embedded_content_and_parent_water_request() {
        let seed = (0..100_000)
            .find(|&seed| {
                let mut r = Rng::new(seed);
                r.range(100);
                r.range(12) == 0
            })
            .unwrap();
        for (object, embedded, expected_dirty) in [
            ("oTemple", true, true),
            ("oTemple", false, true),
            ("oPushBlock", false, false),
        ] {
            let mut world = World::new(42, 34, Area::Temple, 13);
            world.entrance = Vec2::new(24.0, 24.0);
            let support = if object == "oPushBlock" {
                Some(world.spawn(object, Vec2::new(192.0, 160.0)))
            } else {
                if embedded {
                    world.set_tile(12, 10, object);
                } else {
                    super::super::tiles::stamp_at(
                        &mut world,
                        &mut Rng::new(1),
                        "R",
                        12,
                        10,
                        false,
                        false,
                        "",
                    );
                }
                None
            };
            world.set_tile(12, 11, "oTemple");
            if object == "oTemple" {
                super::super::tiles::terrain_property(&mut world, 12, 10, "gold_chunks", 3.0);
                let metadata = world
                    .entities
                    .iter_mut()
                    .find(|e| e.object == "oTerrainMetadata" && e.pos == Vec2::new(200.0, 168.0))
                    .unwrap();
                metadata
                    .text
                    .insert("terrain_sprite".into(), "sTempleGoldBig".into());
                if embedded {
                    metadata.text.insert("treasure".into(), "oRubyBig".into());
                    metadata.data.insert("embedded_treasure".into(), 1.0);
                } else {
                    assert_eq!(metadata.value("embedded_treasure"), 1.0);
                    assert_eq!(metadata.text["treasure"], "Big Ruby");
                }
            }
            world.spawn("oEntrance", Vec2::new(16.0, 16.0));
            world.fluid_dirty = false;
            populate(&mut world, &mut Rng::new(seed), &entity_gen_only_flags());
            if let Some(id) = support {
                assert!(world.entity(id).is_none());
            } else {
                assert_eq!(world.tile(12, 10).unwrap().object, "");
            }
            assert!(world.entities.iter().any(|e| e.alive
                && e.object == "oSpearTrapBottom"
                && e.pos == Vec2::new(192.0, 160.0)));
            let ruby: Vec<_> = world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == "oRubyBig")
                .collect();
            assert_eq!(ruby.len(), usize::from(embedded));
            if embedded {
                assert_eq!(ruby[0].pos, Vec2::new(200.0, 168.0));
                assert_eq!(ruby[0].state, 1);
            }
            assert!(
                !world
                    .entities
                    .iter()
                    .any(|e| e.alive && matches!(e.object.as_str(), "oGoldChunk" | "oGoldNugget"))
            );
            assert_eq!(
                world.fluid_dirty, expected_dirty,
                "{object} embedded={embedded}"
            );
            assert!(!world.entities.iter().any(|e| e.alive
                && e.object == "oTerrainMetadata"
                && e.pos == Vec2::new(200.0, 168.0)));
        }
    }

    #[test]
    fn temple_clean_retirement_uses_actual_r_and_xoc_producer_metadata() {
        for (symbol, x, y, label, inherited, murals) in [
            ("R", 12, 10, "Big Ruby", true, 0),
            ("X", 14, 11, "Diamond", false, 30),
        ] {
            let mut world = World::new(42, 34, Area::Temple, 13);
            super::super::tiles::stamp_at(
                &mut world,
                &mut Rng::new(1),
                symbol,
                12,
                10,
                false,
                false,
                "",
            );
            let index = y * world.width + x;
            let serial = world.tiles[index].creation_serial;
            let origin = Vec2::new(x as f64 * 16.0, y as f64 * 16.0);
            let receiver =
                temple_receiver(&world, serial, FamilyInstance::Tile(index), origin).unwrap();
            assert_eq!(receiver.source.value("embedded_treasure"), 1.0);
            assert_eq!(receiver.source.text["treasure"], label);
            assert_eq!(
                world
                    .entities
                    .iter()
                    .filter(|e| e.alive && e.sprite == "bgLadyXoc")
                    .count(),
                murals
            );
            world.fluid_dirty = false;
            let mut actual = Rng::new(17);
            let mut oracle = actual.clone();
            retire_temple_receiver(&mut world, &mut actual, &receiver);
            assert_eq!(world.tiles[index].object, "");
            assert_eq!(world.fluid_dirty, inherited);
            assert!(
                !world
                    .entities
                    .iter()
                    .any(|e| e.alive && matches!(e.object.as_str(), "oRubyBig" | "oDiamond"))
            );
            assert!(!world.entities.iter().any(|e| e.alive
                && e.object == "oTerrainMetadata"
                && e.pos == Vec2::new(origin.x + 8.0, origin.y + 8.0)));
            assert_eq!(
                world
                    .entities
                    .iter()
                    .filter(|e| e.alive && e.sprite == "bgLadyXoc")
                    .count(),
                murals.saturating_sub(1)
            );
            assert_eq!(actual.next_u64(), oracle.next_u64());
        }
    }

    #[test]
    fn temple_rectangle_false_keeps_fractional_query_endpoints_and_integer_targets() {
        let mut world = World::new(42, 34, Area::Temple, 13);
        let id = world.spawn("oPushBlock", Vec2::new(16.0, 80.25));
        world.entity_mut(id).unwrap().solid = false;
        world.entity_mut(id).unwrap().held = true;
        assert_eq!(
            world.entity(id).unwrap().collision_mask().point_bounds(),
            Rect::new(16.0, 80.0, 16.0, 16.0)
        );
        // Canonical rectangle targetT=80 > raw queryMaxY=79.25: no self hit.
        assert!(!source_rectangle_family(
            &world,
            Rect::new(16.0, 48.25, 33.0, 32.0),
            "oSolid"
        ));
        assert!(source_rectangle_family(
            &world,
            Rect::new(16.0, 80.0, 16.0, 1.0),
            "oSolid"
        ));
        assert!(!source_rectangle_family(
            &world,
            Rect::new(32.0, 80.0, 1.0, 1.0),
            "oSolid"
        ));
        assert!(source_rectangle_family(
            &world,
            Rect::new(31.75, 80.0, 1.0, 1.0),
            "oSolid"
        ));
        world.entity_mut(id).unwrap().alive = false;
        assert!(!source_rectangle_family(
            &world,
            Rect::new(16.0, 80.0, 16.0, 16.0),
            "oSolid"
        ));
        world.set_tile(1, 5, "oTemple");
        assert!(!source_rectangle_family(
            &world,
            Rect::new(16.0, 48.25, 33.0, 32.0),
            "oSolid"
        ));
        assert!(source_rectangle_family(
            &world,
            Rect::new(31.75, 80.0, 1.0, 1.0),
            "oSolid"
        ));
    }

    #[test]
    fn temple_treasure_uses_saved_inclusive_bbox_distance_and_raw_fractional_origin() {
        for (exit_x, expected_distance, blocked) in [(364.0, 29.0, true), (367.0, 32.0, false)] {
            let mut world = World::new(42, 34, Area::Temple, 13);
            let origin = Vec2::new(320.25, 320.25);
            let support = world.spawn("oPushBlock", origin);
            let saved = world
                .entity(support)
                .unwrap()
                .collision_mask()
                .point_bounds();
            world.entity_mut(support).unwrap().alive = false;
            world.spawn("oExit", Vec2::new(exit_x, 320.0));
            world.exit = Vec2::new(exit_x + 8.0, 328.0);
            assert!(Vec2::new(origin.x + 8.0, origin.y + 8.0).distance(world.exit) > 32.0);
            assert_eq!(
                source_family_distance(&world, saved, "oExit"),
                expected_distance
            );
            assert_eq!(
                source_family_distance(&world, saved, "oEntrance"),
                1_000_000.0
            );
            // Canonical seed0 independently wins the first range100 guard.
            let mut actual = Rng::new(0);
            let mut oracle = actual.clone();
            if !blocked {
                assert_eq!(oracle.range(100), 0);
            }
            let water = WaterMasks::new(&world);
            treasure_at(
                &mut world,
                &mut actual,
                &water,
                &mut treasure_state(),
                origin,
                Some(saved),
            );
            assert_eq!(actual.next_u64(), oracle.next_u64());
            let rocks: Vec<_> = world
                .entities
                .iter()
                .filter(|e| e.object == "oRock")
                .collect();
            assert_eq!(rocks.len(), usize::from(!blocked));
            if !blocked {
                assert_eq!(rocks[0].pos, Vec2::new(328.25, 316.25));
            }
        }
    }

    #[test]
    fn temple_light_ceiling_still_consumes_nested_scarab_roll() {
        let mut world = World::new(42, 34, Area::Temple, 13);
        world.entrance = Vec2::new(328.0, 88.0);
        let id = world.spawn("oPushBlock", Vec2::new(320.0, 80.0));
        world
            .entity_mut(id)
            .unwrap()
            .text
            .insert("type".into(), "Altar".into());
        let mut actual = Rng::new(29);
        let mut oracle = actual.clone();
        oracle.range(100);
        oracle.range(40);
        populate(&mut world, &mut actual, &entity_gen_only_flags());
        assert!(!exists(&world, "oScarab"));
        assert_eq!(actual.next_u64(), oracle.next_u64());
    }

    #[test]
    fn temple_existing_damsel_still_consumes_alcove_damsel_roll() {
        let seed = (0..100_000)
            .find(|&seed| {
                let mut r = Rng::new(seed);
                r.range(100) != 0
                    && r.range(40) != 0
                    && r.range(5) != 0
                    && r.range(10) != 0
                    && r.range(15) != 0
                    && r.range(8) == 0
                    && r.range(14) != 0
                    && r.range(3) == 0
            })
            .unwrap();
        let mut world = World::new(42, 34, Area::Temple, 13);
        for (x, y) in [(20, 20), (20, 18), (19, 19)] {
            world.set_tile(x, y, "oTemple");
        }
        let bounds = world.tile_collision_mask(20, 20).point_bounds();
        let water = WaterMasks::new(&world);
        let mut state = treasure_state();
        state.damsel = true;
        let mut actual = Rng::new(seed);
        let mut oracle = actual.clone();
        for n in [100, 40, 5, 10, 15, 8, 14, 3] {
            oracle.range(n);
        }
        treasure_at(
            &mut world,
            &mut actual,
            &water,
            &mut state,
            Vec2::new(320.0, 320.0),
            Some(bounds),
        );
        assert!(!exists(&world, "oDamsel"));
        assert!(exists(&world, "oGoldBar"));
        assert_eq!(actual.next_u64(), oracle.next_u64());
    }

    #[test]
    fn temple_room_difference_still_requires_named_enemy_point_to_be_clear() {
        let mut world = World::new(42, 34, Area::Temple, 13);
        world.entrance = Vec2::new(24.0, 504.0); // different column, same source room row
        world.set_tile(20, 32, "oTemple");
        world.spawn("oCaveman", Vec2::new(318.0, 496.0));
        world.spawn("oExit", Vec2::new(320.0, 512.0));
        assert!(source_point_family(
            &world,
            Vec2::new(320.0, 496.0),
            "oEnemy"
        ));
        let mut actual = Rng::new(29);
        let mut oracle = actual.clone();
        oracle.range(100); // only the unconditional statue statement
        populate(&mut world, &mut actual, &entity_gen_only_flags());
        assert!(!exists(&world, "oTombLord"));
        assert_eq!(
            world
                .entities
                .iter()
                .filter(|e| e.object == "oCaveman")
                .count(),
            1
        );
        assert_eq!(actual.next_u64(), oracle.next_u64());
    }

    #[test]
    fn temple_fresh_forced_pass_sees_smash_birth_after_initial_pass() {
        let seed = (0..2_000_000)
            .find(|&seed| {
                let mut r = Rng::new(seed);
                r.range(4) != 0
                    && r.range(100) != 0
                    && r.range(160) != 0
                    && r.range(40) != 0
                    && r.range(40) != 0
                    && r.range(40) != 0
                    && r.range(60) == 0
            })
            .unwrap();
        let mut world = World::new(42, 34, Area::Temple, 14);
        world.entrance = Vec2::new(24.0, 24.0);
        world.set_tile(10, 32, "oTemple");
        let mut actual = Rng::new(seed);
        let mut oracle = actual.clone();
        for n in [4, 100, 160, 40, 40, 40, 60] {
            oracle.range(n);
        }
        let direction = oracle.range(4);
        TEMPLE_RECEIVER_TRACE.with(|trace| *trace.borrow_mut() = Some(Vec::new()));
        populate(&mut world, &mut actual, &entity_gen_only_flags());
        let visits = TEMPLE_RECEIVER_TRACE.with(|trace| trace.borrow_mut().take().unwrap());
        assert_eq!(visits, vec![(1, Vec2::new(160.0, 512.0))]);
        let smash = world
            .entities
            .iter()
            .find(|e| e.object == "oSmashTrap")
            .unwrap();
        assert_eq!(smash.pos, Vec2::new(160.0, 496.0));
        assert_eq!(smash.value("direction"), f64::from(direction));
        assert_eq!(smash.value("invincible"), 1.0);
        let door = world
            .entities
            .iter()
            .find(|e| e.object == "oGoldDoor")
            .unwrap();
        assert_eq!(door.pos, Vec2::new(160.0, 480.0));
        assert!(smash.creation_serial < door.creation_serial);
        assert_eq!(actual.next_u64(), oracle.next_u64());
    }

    #[test]
    fn temple_reset_gold_latch_does_not_treat_preexisting_door_as_generated() {
        let mut world = World::new(42, 34, Area::Temple, 14);
        let old = world.spawn("oGoldDoor", Vec2::new(16.0, 16.0));
        world.set_tile(20, 20, "oTemple");
        world.spawn("oShopkeeper", Vec2::new(320.0, 320.0));
        let mut actual = Rng::new(42);
        let mut oracle = actual.clone();
        oracle.range(4);
        oracle.range(100);
        populate(&mut world, &mut actual, &entity_gen_only_flags());
        assert!(world.entity(old).is_some());
        assert_eq!(
            world
                .entities
                .iter()
                .filter(|e| e.object == "oGoldDoor")
                .count(),
            2
        );
        assert!(
            world
                .entities
                .iter()
                .any(|e| e.object == "oGoldDoor" && e.pos == Vec2::new(320.0, 304.0))
        );
        assert_eq!(actual.next_u64(), oracle.next_u64());
    }

    #[test]
    fn temple_reset_tomb_latch_does_not_treat_preexisting_tomb_as_generated() {
        let seed = (0..1_000_000)
            .find(|&seed| {
                let mut r = Rng::new(seed);
                r.range(100) != 0 && r.range(40) == 0 && r.range(100) == 0
            })
            .unwrap();
        let mut world = World::new(42, 34, Area::Temple, 13);
        world.entrance = Vec2::new(24.0, 24.0);
        world.set_tile(10, 32, "oTemple");
        let old = world.spawn("oTombLord", Vec2::new(16.0, 16.0));
        let mut actual = Rng::new(seed);
        let mut oracle = actual.clone();
        for n in [100, 40, 100] {
            oracle.range(n);
        }
        populate(&mut world, &mut actual, &entity_gen_only_flags());
        assert!(world.entity(old).is_some());
        assert_eq!(
            world
                .entities
                .iter()
                .filter(|e| e.object == "oTombLord")
                .count(),
            2
        );
        assert!(
            world
                .entities
                .iter()
                .any(|e| e.object == "oTombLord" && e.pos == Vec2::new(160.0, 480.0))
        );
        assert_eq!(actual.next_u64(), oracle.next_u64());
    }

    fn isolated_temple_support(level: u32) -> World {
        let mut world = World::new(42, 34, Area::Temple, level);
        world.entrance = at(1, 1);
        // Suppress unrelated treasure draws at the sole terrain support.
        world.exit = at(10, 32);
        world.set_tile(10, 32, "oTemple");
        world
    }

    #[test]
    fn tomb_lord_blocked_rectangle_still_consumes_eager_chance_before_caveman() {
        let seed = (0..100_000)
            .find(|&seed| {
                let mut rng = Rng::new(seed);
                rng.range(100) != 0 && rng.range(40) == 0 && rng.range(40) == 0
            })
            .unwrap();
        for offset in [16.0, 32.0] {
            let mut world = isolated_temple_support(13);
            let block = world.spawn("oPushBlock", Vec2::new(160.0 + offset, 496.0));
            world.entity_mut(block).unwrap().solid = false;
            world.entity_mut(block).unwrap().held = true;
            world.spawn("oExit", Vec2::new(160.0, 512.0)); // exact bbox suppresses treasure
            let mut actual = Rng::new(seed);
            let mut source = actual.clone();
            assert_ne!(source.range(100), 0); // unconditional statue :395
            assert_eq!(source.range(40), 0); // eager TombLord :461, blocked
            assert_eq!(source.range(40), 0); // conditional Caveman :466
            TEMPLE_RNG_STARTS.with(|trace| *trace.borrow_mut() = Some(Vec::new()));
            populate(&mut world, &mut actual, &entity_gen_only_flags());
            let starts = TEMPLE_RNG_STARTS.with(|trace| trace.borrow_mut().take().unwrap());
            let mut after_first_receiver = starts[1].1.clone();
            assert_eq!(after_first_receiver.next_u64(), source.next_u64());
            assert!(
                !world
                    .entities
                    .iter()
                    .any(|e| e.object == "oTombLord" && e.pos == Vec2::new(160.0, 480.0))
            );
            assert!(
                world
                    .entities
                    .iter()
                    .any(|e| e.object == "oCaveman" && e.pos == Vec2::new(160.0, 496.0))
            );
        }
    }

    #[test]
    fn ordinary_gold_door_rejects_dynamic_solid_before_chance_draw() {
        let seed = (0..100_000)
            .find(|&seed| {
                let mut rng = Rng::new(seed);
                rng.range(4); // genTombLord on level fourteen.
                rng.range(100); // unconditional primary statue roll
                rng.range(160) == 0 // A tile-only ordinary guard would create the door.
            })
            .unwrap();
        let mut world = isolated_temple_support(14);
        world.spawn("oPushBlock", Vec2::new(160.0, 496.0));
        world.spawn("oExit", Vec2::new(160.0, 512.0));
        let mut actual = Rng::new(seed);
        let mut source = actual.clone();
        source.range(4);
        source.range(100); // blocked above suppresses every later first-receiver branch
        TEMPLE_RNG_STARTS.with(|trace| *trace.borrow_mut() = Some(Vec::new()));
        populate(&mut world, &mut actual, &entity_gen_only_flags());
        assert!(!world.entities.iter().any(|entity| {
            entity.object == "oGoldDoor" && entity.pos == Vec2::new(160.0, 496.0)
        }));
        let starts = TEMPLE_RNG_STARTS.with(|trace| trace.borrow_mut().take().unwrap());
        let mut after_first_receiver = starts[1].1.clone();
        assert_eq!(
            after_first_receiver.next_u64(),
            source.next_u64(),
            "blocked ordinary placement consumes only the unconditional statue draw"
        );
    }

    #[test]
    fn ordinary_gold_door_protects_its_terrain_receiver() {
        let seed = (0..100_000)
            .find(|&seed| {
                let mut rng = Rng::new(seed);
                rng.range(4);
                rng.range(100);
                rng.range(160) == 0
            })
            .unwrap();
        let mut world = isolated_temple_support(14);
        populate(&mut world, &mut Rng::new(seed), &entity_gen_only_flags());
        let door = world
            .entities
            .iter()
            .find(|e| e.object == "oGoldDoor")
            .unwrap();
        assert_eq!(door.pos, Vec2::new(160.0, 496.0));
        assert_eq!(
            door.value("invincible"),
            0.0,
            "source write targets the support, not the new door"
        );
        let metadata = world
            .entities
            .iter()
            .find(|e| e.object == "oTerrainMetadata" && e.pos == at(10, 32))
            .unwrap();
        assert_eq!(metadata.value("invincible"), 1.0);
        let mut game = crate::Game::shell(42, crate::RulesPreset::Classic);
        game.world = world;
        game.world.set_tile(11, 32, "oTemple");
        crate::items::explosives::destroy_terrain(&mut game, 10, 32);
        crate::items::explosives::destroy_terrain(&mut game, 11, 32);
        assert_eq!(game.world.tile(10, 32).unwrap().object, "oTemple");
        assert_eq!(game.world.tile(11, 32).unwrap().object, "");
    }

    #[test]
    fn forced_gold_door_protects_exact_mixed_storage_receiver() {
        for dynamic_first in [false, true] {
            let mut world = World::new(42, 34, Area::Temple, 14);
            let pos = Vec2::new(320.0, 320.0);
            if !dynamic_first {
                world.set_tile(20, 20, "oTemple");
            }
            let block = world.spawn("oPushBlock", pos);
            if dynamic_first {
                world.set_tile(20, 20, "oTemple");
            }
            // Exclude the ordinary terrain pass; forced source :488 has no
            // shop exclusion and chooses between coincident real receivers.
            world.spawn("oShopkeeper", pos);
            let mut actual = Rng::new(42);
            let mut source = actual.clone();
            source.range(4);
            source.range(100); // initial terrain receiver, before shop gate
            source.range(100); // coincident dynamic receiver, before shop gate
            populate(&mut world, &mut actual, &entity_gen_only_flags());
            assert!(
                world
                    .entities
                    .iter()
                    .any(|e| e.object == "oGoldDoor" && e.pos == Vec2::new(320.0, 304.0))
            );
            assert_eq!(
                world.entity(block).unwrap().value("invincible"),
                f64::from(dynamic_first)
            );
            let protected_tile = world.entities.iter().any(|e| {
                e.object == "oTerrainMetadata"
                    && e.pos == at(20, 20)
                    && e.value("invincible") == 1.0
            });
            assert_eq!(protected_tile, !dynamic_first);
            assert_eq!(actual.next_u64(), source.next_u64());
        }
    }

    #[test]
    fn forced_gold_door_visits_dynamic_solids_in_birth_order_even_in_shops() {
        let mut world = World::new(42, 34, Area::Temple, 14);
        world.entrance = at(1, 1);
        world.exit = at(37, 30);
        // All three captured primary receivers are shop-gated after their statue roll.
        let first = world.spawn("oPushBlock", Vec2::new(320.0, 320.0));
        let duplicate = world.spawn("oPushBlock", Vec2::new(320.0, 320.0));
        world.spawn("oPushBlock", Vec2::new(160.0, 320.0));
        world.spawn("oShopkeeper", Vec2::new(320.0, 320.0));
        world.spawn("oShopkeeper", Vec2::new(160.0, 320.0));
        let mut actual = Rng::new(42);
        let mut source = actual.clone();
        // Literal local source statements: initial generation flag, one
        // statue roll per captured support, then the nonrandom forced pass.
        source.range(4);
        for _ in 0..3 {
            source.range(100);
        }
        populate(&mut world, &mut actual, &entity_gen_only_flags());
        let doors: Vec<_> = world
            .entities
            .iter()
            .filter(|e| e.object == "oGoldDoor")
            .collect();
        assert_eq!(doors.len(), 1);
        assert_eq!(doors[0].pos, Vec2::new(320.0, 304.0));
        assert_eq!(actual.next_u64(), source.next_u64());
        assert_eq!(world.entity(first).unwrap().value("invincible"), 1.0);
        assert_eq!(world.entity(duplicate).unwrap().value("invincible"), 0.0);
        let mut game = crate::Game::shell(42, crate::RulesPreset::Classic);
        game.world = world;
        // Use the real attack destruction service on coincident instances:
        // protection applies to the selected receiver, not its neighbors.
        assert!(!crate::items::explosives::destroy_solid_instance(
            &mut game, first
        ));
        assert!(crate::items::explosives::destroy_solid_instance(
            &mut game, duplicate
        ));
        assert!(game.world.entity(first).is_some());
        assert!(game.world.entity(duplicate).is_none());
    }

    fn assert_campaign_temple_placement_clears_push_blocks(selected_level: u32) {
        // Captured immediately before Game::next_level in the seed-two
        // Market -> Ankh -> Moai -> Crown campaign interaction prefix.
        // These are the natural runtime RNG states, not independent level
        // seeds. The carried Sceptre flag is removed before level14 generation.
        let flags = BTreeSet::from(
            [
                "made_black_market",
                "made_market_entrance",
                "made_moai",
                "made_udjat_eye",
                "moai_open",
                "no_dark_level",
                "render_discontinuity",
                "transition_cinematic_complete",
            ]
            .map(str::to_owned),
        );
        for (level, rng_state, invalid_origin, blocker_origin) in [
            (
                13,
                10778720839894840496_u64,
                Vec2::new(224.0, 352.0),
                Vec2::new(240.0, 368.0),
            ),
            (
                14,
                11972203220667455624_u64,
                Vec2::new(448.0, 112.0),
                Vec2::new(448.0, 112.0),
            ),
        ] {
            if level != selected_level {
                continue;
            }
            let mut rng: Rng =
                serde_json::from_value(serde_json::json!({"state": rng_state})).unwrap();
            let world = crate::generation::generate(&mut rng, level, &flags);
            assert!(
                world
                    .entities
                    .iter()
                    .any(|e| e.alive && e.object == "oPushBlock" && e.pos == blocker_origin),
                "level {level} must retain the natural blocking instance"
            );
            assert!(
                !world.entities.iter().any(|e| e.alive
                    && e.object
                        == if level == 13 {
                            "oTombLord"
                        } else {
                            "oGoldDoor"
                        }
                    && e.pos == invalid_origin),
                "level {level} invalid original placement must be rejected"
            );
            if level == 14 {
                assert!(exists(&world, "oGoldDoor"));
            }
            for entity in &world.entities {
                let probe = match entity.object.as_str() {
                    "oTombLord" => {
                        // Literal source rectangle, including x+32.
                        Rect::new(entity.pos.x, entity.pos.y, 33.0, 32.0)
                    }
                    "oGoldDoor" => Rect::new(entity.pos.x, entity.pos.y, 1.0, 1.0),
                    _ => continue,
                };
                assert!(
                    !world.entities.iter().any(|block| {
                        block.alive
                            && block.object == "oPushBlock"
                            && block.sprite_bounds().intersects(probe)
                    }),
                    "campaign seed2, level {level}, {} at {:?}",
                    entity.object,
                    entity.pos
                );
            }
        }
    }

    #[test]
    fn generated_temple_special_tomb_lord_clears_preexisting_push_blocks() {
        assert_campaign_temple_placement_clears_push_blocks(13);
    }

    #[test]
    fn generated_temple_special_gold_door_clears_preexisting_push_blocks() {
        assert_campaign_temple_placement_clears_push_blocks(14);
    }

    #[test]
    fn population_detects_water_descendants_and_source_masks() {
        let mut world = World::new(5, 5, Area::Jungle, 5);
        world.set_tile(1, 1, "oWaterSwim");
        assert!(wet(&world, 1, 1));
        let water = world.spawn("oWaterSwim", Vec2::new(32.0, 32.0));
        assert!(wet(&world, 2, 2));
        assert!(!wet(&world, 3, 2));
        world.entity_mut(water).unwrap().alive = false;
        assert!(!wet(&world, 2, 2));
    }

    #[test]
    fn population_water_snapshot_matches_live_point_queries() {
        let mut world = World::new(8, 8, Area::Jungle, 5);
        world.set_tile(1, 1, "oWaterSwim");
        let metadata = world.spawn("oTerrainMetadata", at(1, 1));
        world
            .entity_mut(metadata)
            .unwrap()
            .text
            .insert("terrain_sprite".into(), "sWaterBottomTall2".into());
        world.spawn("oWaterSwim", Vec2::new(48.0, 48.0));
        world.spawn("oLava", Vec2::new(80.0, 80.0));
        let snapshot = WaterMasks::new(&world);
        for y in -1..=8 {
            for x in -1..=8 {
                assert_eq!(snapshot.wet(x, y), wet(&world, x, y), "point {x},{y}");
            }
        }
    }

    #[test]
    fn source_jungle_water_population_produces_fish_and_cemetery_dead_fish() {
        for cemetery in [false, true] {
            let mut world = World::new(42, 34, Area::Jungle, 5);
            world.entrance = Vec2::new(24.0, 24.0);
            for y in 3..30 {
                for x in 1..41 {
                    world.spawn("oWaterSwim", Vec2::new(x as f64 * 16.0, y as f64 * 16.0));
                }
            }
            let flags = if cemetery {
                BTreeSet::from(["cemetery".into()])
            } else {
                BTreeSet::new()
            };
            populate(&mut world, &mut Rng::new(42), &flags);
            let fish: Vec<_> = world
                .entities
                .iter()
                .filter(|entity| matches!(entity.object.as_str(), "oPiranha" | "oDeadFish"))
                .collect();
            assert!(!fish.is_empty());
            assert!(
                fish.iter()
                    .all(|entity| entity.object == if cemetery { "oDeadFish" } else { "oPiranha" })
            );
            assert!(fish.iter().all(|entity| wet(
                &world,
                (entity.pos.x / 16.0) as i32,
                (entity.pos.y / 16.0) as i32
            )));
            let fish_id = fish[0].id;
            let mut game = crate::Game::new(42, crate::RulesPreset::Classic);
            game.world = world;
            game.player.pos = game.world.entrance;
            game.player.previous_pos = game.player.pos;
            game.step(&crate::InputFrame::default());
            let fish = game.world.entity(fish_id).unwrap();
            assert_eq!(fish.value("simulation_tick"), game.engine_time() as f64);
            if !cemetery {
                assert!(fish.animation > 0.0);
            }
        }
    }

    fn room_fixture(area: Area, level: u32) -> World {
        let mut world = World::new(42, 34, area, level);
        world.entrance = at(3, 3);
        world.exit = at(37, 30);
        for y in 0..34 {
            for x in 0..42 {
                if x == 0 || x == 41 || y == 0 || y == 33 || y % 4 <= 1 {
                    world.set_tile(x, y, "oBlock");
                }
            }
        }
        world
    }

    fn treasure_state() -> TreasureState {
        TreasureState {
            udjat: false,
            chest_chance: 8,
            damsel: false,
            cemetery: false,
        }
    }

    fn literal_jungle_source(
        world: &World,
        rng: &mut Rng,
        cemetery: bool,
    ) -> Vec<(&'static str, Vec2, Option<i8>)> {
        // Explicit source XML membership: VineTop is not a Vine child;
        // WaterSwim and Lava are the two Water descendants. Establish the
        // port's input instance order independently of family_origins.
        let mut instances = Vec::new();
        for (index, tile) in world.tiles.iter().enumerate() {
            instances.push((
                tile.creation_serial,
                tile.object.as_str(),
                Vec2::new(
                    (index % world.width) as f64 * 16.0,
                    (index / world.width) as f64 * 16.0,
                ),
            ));
        }
        for entity in world.entities.iter().filter(|entity| entity.alive) {
            instances.push((entity.creation_serial, entity.object.as_str(), entity.pos));
        }
        instances.sort_by_key(|(serial, _, _)| *serial);
        let mut births = Vec::new();
        // scrEntityGen:322–325; Monkey Create facing=rand(0,1).
        for &(_, object, position) in &instances {
            if object == "oVine" && 1 + rng.range(15) == 1 {
                let facing = if rng.range(2) == 0 { -1 } else { 1 };
                births.push(("oMonkey", position, Some(facing)));
            }
        }
        // scrEntityGen:327–337; Piranha Create rand(1,2), whereas DeadFish
        // Create has no random statement. Do not rasterize Water masks.
        for &(_, object, position) in &instances {
            if !matches!(object, "oWater" | "oWaterSwim" | "oLava") {
                continue;
            }
            let blocked = world.tiles_point(position, "oSolid")
                || world.entities.iter().any(|entity| {
                    entity.alive
                        && crate::catalog::is_a(&entity.object, "oSolid")
                        && entity.sprite_bounds().contains(position)
                });
            if !blocked && 1 + rng.range(30) == 1 {
                let facing = if cemetery {
                    None
                } else {
                    Some(if 1 + rng.range(2) == 1 { -1 } else { 1 })
                };
                births.push((
                    if cemetery { "oDeadFish" } else { "oPiranha" },
                    Vec2::new(position.x + 4.0, position.y + 4.0),
                    facing,
                ));
            }
        }
        births
    }

    fn assert_jungle_births(
        world: &World,
        after_serial: u64,
        expected: &[(&str, Vec2, Option<i8>)],
    ) {
        let actual: Vec<_> = world
            .entities
            .iter()
            .filter(|entity| {
                entity.creation_serial > after_serial
                    && matches!(entity.object.as_str(), "oMonkey" | "oPiranha" | "oDeadFish")
            })
            .collect();
        assert_eq!(actual.len(), expected.len());
        for (entity, &(object, position, facing)) in actual.iter().zip(expected) {
            assert_eq!(entity.object, object);
            assert_eq!(entity.pos, position);
            if let Some(facing) = facing {
                assert_eq!(entity.facing, facing);
                assert_eq!(entity.value("jungle_context_created"), 1.0);
            }
        }
        assert!(
            actual
                .windows(2)
                .all(|pair| pair[0].creation_serial < pair[1].creation_serial)
        );
    }

    #[test]
    fn jungle_instance_passes_match_literal_source_with_immediate_create_draws() {
        let mut fixture = World::new(12, 12, Area::Jungle, 5);
        // Deliberately interleave storage types, source families and spatial
        // order. Duplicate/overhanging Water instances must each get one roll.
        fixture.spawn("oWaterSwim", Vec2::new(21.5, 24.5));
        fixture.set_tile(9, 8, "oVine");
        fixture.spawn("oVine", Vec2::new(18.5, 41.0));
        fixture.set_tile(1, 1, "oWaterSwim");
        fixture.spawn("oVineTop", Vec2::new(36.0, 36.0));
        fixture.spawn("oWaterSwim", Vec2::new(21.5, 24.5));
        fixture.spawn("oLava", Vec2::new(72.0, 40.0));
        fixture.spawn("oWater", Vec2::new(0.0, 0.0));
        fixture.set_tile(6, 6, "oBlock");
        fixture.spawn("oWaterSwim", Vec2::new(111.5, 111.5));
        fixture.spawn("oPushBlock", Vec2::new(48.0, 80.0));
        fixture.spawn("oWaterSwim", Vec2::new(48.0, 80.0));
        let dead = fixture.spawn("oVine", Vec2::new(70.0, 70.0));
        fixture.entity_mut(dead).unwrap().alive = false;
        assert!(source_point_family(
            &fixture,
            Vec2::new(111.5, 111.5),
            "oSolid"
        ));
        assert!(!source_point_family(
            &fixture,
            Vec2::new(119.5, 119.5),
            "oSolid"
        ));
        for cemetery in [false, true] {
            let mut monkeys = 0;
            let mut fish = 0;
            for seed in 0..128 {
                let mut world = fixture.clone();
                let mut actual = Rng::new(seed);
                let mut source = actual.clone();
                let expected = literal_jungle_source(&fixture, &mut source, cemetery);
                jungle_population(&mut world, &mut actual, cemetery);
                assert_jungle_births(&world, fixture.creation_serial, &expected);
                assert_eq!(
                    actual.next_u64(),
                    source.next_u64(),
                    "seed={seed} cemetery={cemetery}"
                );
                monkeys += expected
                    .iter()
                    .filter(|(object, _, _)| *object == "oMonkey")
                    .count();
                fish += expected
                    .iter()
                    .filter(|(object, _, _)| *object != "oMonkey")
                    .count();
            }
            assert!(monkeys > 0 && fish > 0, "exercise both Create sequences");
        }
    }

    #[test]
    fn generated_jungle_uses_separate_source_instance_passes() {
        let flags = BTreeSet::from(["no_dark_level".into(), "made_market_entrance".into()]);
        let mut vine_instances = 0;
        let mut water_instances = 0;
        for seed in 0..8 {
            JUNGLE_POPULATION_TRACE.with(|trace| *trace.borrow_mut() = Some(Vec::new()));
            let world = super::super::generate(&mut Rng::new(seed), 5, &flags);
            let traces = JUNGLE_POPULATION_TRACE.with(|trace| trace.borrow_mut().take().unwrap());
            assert_eq!(traces.len(), 1);
            let trace = &traces[0];
            let mut source = trace.rng_before.clone();
            let expected = literal_jungle_source(&trace.world_before, &mut source, trace.cemetery);
            assert_jungle_births(&world, trace.world_before.creation_serial, &expected);
            let mut actual = trace.rng_after.clone();
            assert_eq!(
                actual.next_u64(),
                source.next_u64(),
                "generated seed={seed}"
            );
            // Coverage is aggregated across real layouts; no fixture injects
            // a pool/vine or forces a successful actor chance into generation.
            vine_instances += family_origins(&trace.world_before, "oVine").len();
            water_instances += family_origins(&trace.world_before, "oWater").len();
        }
        assert!(vine_instances > 0 && water_instances > 0);
    }

    fn literal_source_web_branch(rng: &mut Rng, area: Area, alcove: bool) -> Vec<&'static str> {
        // Independent scrTreasureGen:43–125 oracle for genUdjatEye=false,
        // oGame.damsel=true, argument0=0. Use literal rand(1,n) results and
        // source statement order, without calling the production helper.
        for (limit, object) in [(100, "oRock"), (40, "oJar")] {
            if 1 + rng.range(limit) == 1 {
                return vec![object];
            }
        }
        // Canonical EXE object table: oGiantSpider=749, object slot0 absent.
        // Its numeric '<' yields0, distance_to_object(0) returns1,000,000,
        // and the compiled if tests >=0.5. Spider proximity is never queried.
        let argument = u32::from(749_u32 < 100);
        assert_eq!(argument, 0);
        let distance_to_object_zero = 1_000_000.0;
        let mut n = 60;
        if distance_to_object_zero >= 0.5 {
            n = if alcove { 5 } else { 10 };
        }
        let web_roll = 1 + rng.range(n);
        if area != Area::Ice && web_roll == 1 {
            return vec!["oWeb"];
        }
        let level = if area == Area::Ice { 9 } else { 1 };
        let rolls: &[(u32, &str)] = if alcove {
            &[
                (10, "oCrate"),
                (15, "oChest"),
                (8, "existing_damsel_guard"),
                (40 - 2 * level, "bones"),
                (3, "oGoldBar"),
                (6, "oGoldBars"),
                (6, "oEmeraldBig"),
                (8, "oSapphireBig"),
                (10, "oRubyBig"),
            ]
        } else {
            &[
                (4, "oGoldBar"),
                (8, "oGoldBars"),
                (80 - level, "bones"),
                (8, "oEmeraldBig"),
                (9, "oSapphireBig"),
                (10, "oRubyBig"),
            ]
        };
        for &(limit, object) in rolls {
            let roll = 1 + rng.range(limit);
            // Source :74 draws even though oGame.damsel is already true.
            if object == "existing_damsel_guard" {
                continue;
            }
            if roll == 1 {
                return if object == "bones" {
                    if 1 + rng.range(8) == 1 {
                        vec!["oFakeBones"]
                    } else {
                        vec!["oBones", "oSkull"]
                    }
                } else {
                    vec![object]
                };
            }
        }
        Vec::new()
    }

    #[test]
    fn eager_ice_web_exclusion_still_changes_downstream_treasure() {
        // Independent canonical controls preserve the complete source branch
        // vectors: all twelve alcove rolls lose; tunnel GoldBar wins only
        // after Rock/Jar and the eager Ice-excluded web roll have been consumed.
        for (alcove, seed, limits, expected_object) in [
            (
                true,
                3,
                vec![100, 40, 5, 10, 15, 8, 22, 3, 6, 6, 8, 10],
                None,
            ),
            (false, 12, vec![100, 40, 10, 4], Some("oGoldBar")),
        ] {
            let mut world = World::new(16, 16, Area::Ice, 9);
            world.entrance = at(1, 1);
            world.exit = at(14, 14);
            world.set_tile(6, 8, "oDark");
            world.set_tile(5, 7, "oDark");
            world.set_tile(
                if alcove { 6 } else { 7 },
                if alcove { 6 } else { 7 },
                "oDark",
            );
            let water = WaterMasks::new(&world);
            let mut state = treasure_state();
            state.damsel = false;
            let mut actual = Rng::new(seed);
            let mut source = actual.clone();
            for limit in limits {
                source.range(limit);
            }
            treasure(&mut world, &mut actual, &water, &mut state, 6, 8);
            assert!(!exists(&world, "oWeb"));
            assert_eq!(
                world.entities.first().map(|e| e.object.as_str()),
                expected_object
            );
            assert_eq!(world.entities.len(), usize::from(expected_object.is_some()));
            assert_eq!(actual.next_u64(), source.next_u64(), "alcove={alcove}");
        }
    }

    #[test]
    fn eager_web_roll_waits_for_rock_jar_returns_and_udjat_branch_stops() {
        // Canonical guard tapes were derived independently from raw LCG
        // states, preserving the Rock/Jar early returns and failed Udjat body.
        for (seed, limits, object, udjat, chest_chance) in [
            (0, vec![100], Some("oRock"), false, 8),
            (6, vec![100, 40], Some("oJar"), false, 8),
            (1, vec![100, 40, 5, 8], None, true, 7),
        ] {
            let mut world = World::new(16, 16, Area::Mines, 2);
            world.entrance = at(1, 1);
            world.exit = at(14, 14);
            for (x, y) in [(6, 8), (5, 7), (6, 6)] {
                world.set_tile(x, y, "oBrick");
            }
            let water = WaterMasks::new(&world);
            let mut state = treasure_state();
            state.udjat = udjat;
            let mut actual = Rng::new(seed);
            let mut source = actual.clone();
            for limit in limits {
                source.range(limit);
            }
            treasure(&mut world, &mut actual, &water, &mut state, 6, 8);
            assert_eq!(world.entities.first().map(|e| e.object.as_str()), object);
            assert_eq!(world.entities.len(), usize::from(object.is_some()));
            assert_eq!(state.chest_chance, chest_chance);
            assert_eq!(actual.next_u64(), source.next_u64());
        }
    }

    #[test]
    fn eager_non_temple_damsel_guard_draws_even_when_an_existing_damsel_blocks_spawn() {
        for existing in [false, true] {
            let mut world = World::new(16, 16, Area::Mines, 1);
            world.entrance = at(1, 1);
            world.exit = at(14, 14);
            for (x, y) in [(6, 8), (5, 7), (6, 6)] {
                world.set_tile(x, y, "oBrick");
            }
            if existing {
                world.spawn("oDamsel", Vec2::new(200.0, 200.0));
            }
            let before = world.entities.len();
            let water = WaterMasks::new(&world);
            let mut state = treasure_state();
            state.damsel = existing;
            // Independent canonical seed67 wins Damsel after five losses;
            // existing Damsel suppresses its body, then bones loses and gold wins.
            let mut actual = Rng::new(67);
            let mut source = actual.clone();
            // Source :45/46/56/66/70/74. The Damsel roll succeeds here.
            for (limit, value) in [(100, 10), (40, 24), (5, 3), (10, 9), (15, 14), (8, 0)] {
                assert_eq!(source.range(limit), value);
            }
            let expected_object = if existing {
                assert_eq!(source.range(38), 30);
                assert_eq!(source.range(3), 0);
                "oGoldBar"
            } else {
                "oDamsel"
            };
            treasure(&mut world, &mut actual, &water, &mut state, 6, 8);
            assert_eq!(
                world.entities[before..]
                    .iter()
                    .map(|e| e.object.as_str())
                    .collect::<Vec<_>>(),
                [expected_object]
            );
            assert_eq!(
                world
                    .entities
                    .iter()
                    .filter(|e| e.object == "oDamsel")
                    .count(),
                1
            );
            assert!(state.damsel);
            assert_eq!(actual.next_u64(), source.next_u64());
        }
    }

    #[test]
    fn treasure_web_typo_matches_literal_source_rng_and_ignores_spider_proximity() {
        for area in [Area::Mines, Area::Ice] {
            for alcove in [false, true] {
                let mut webs = 0;
                let mut fallthrough = 0;
                for seed in 0..64 {
                    for spider in [
                        None,
                        Some(Vec2::new(96.0, 112.0)),
                        Some(Vec2::new(512.0, 512.0)),
                    ] {
                        let level = if area == Area::Ice { 9 } else { 1 };
                        let mut world = World::new(16, 16, area, level);
                        world.entrance = at(1, 1);
                        world.exit = at(14, 14);
                        world.set_tile(6, 8, "oBlock");
                        world.set_tile(5, 7, "oBlock");
                        if alcove {
                            world.set_tile(6, 6, "oBlock");
                        } else {
                            world.set_tile(7, 7, "oBlock");
                        }
                        if let Some(position) = spider {
                            world.spawn("oGiantSpider", position);
                        }
                        let first_new = world.entities.len();
                        let water = WaterMasks::new(&world);
                        let mut state = treasure_state();
                        state.damsel = true;
                        let mut actual = Rng::new(seed);
                        let mut source = actual.clone();
                        let expected = literal_source_web_branch(&mut source, area, alcove);
                        treasure(&mut world, &mut actual, &water, &mut state, 6, 8);
                        let spawned = &world.entities[first_new..];
                        assert_eq!(
                            spawned
                                .iter()
                                .map(|entity| entity.object.as_str())
                                .collect::<Vec<_>>(),
                            expected,
                            "{area:?} alcove={alcove} seed={seed} spider={spider:?}"
                        );
                        assert_eq!(actual.next_u64(), source.next_u64(), "source RNG order");
                        for entity in spawned {
                            let expected_position = match entity.object.as_str() {
                                "oWeb" | "oFakeBones" | "oBones" => Vec2::new(96.0, 112.0),
                                "oSkull" => Vec2::new(108.0, 124.0),
                                "oJar" => Vec2::new(104.0, 122.0),
                                "oCrate" | "oChest" | "oGoldBars" => Vec2::new(104.0, 120.0),
                                _ => Vec2::new(104.0, 124.0),
                            };
                            assert_eq!(entity.pos, expected_position);
                        }
                        if expected == ["oWeb"] {
                            webs += 1;
                        } else if !matches!(expected.first(), Some(&"oRock" | &"oJar")) {
                            fallthrough += 1;
                        }
                    }
                }
                assert!(fallthrough > 0, "exercise downstream loot chain");
                if area == Area::Ice {
                    assert_eq!(webs, 0, "Ice consumes the web roll but cannot create a web");
                } else {
                    assert!(webs > 0, "exercise source web successes");
                }
            }
        }
    }

    #[test]
    fn ice_ufo_create_draw_precedes_same_support_treasure_draws() {
        // scrEntityGen:359 calls UFO Create (shiftToggle rand(0,1)) before
        // scrTreasureGen:376. Seed is selected from those literal statements,
        // not a generated layout. One support isolates their complete stream.
        let mut world = World::new(42, 38, Area::Ice, 9);
        world.entrance = at(1, 1);
        world.exit = at(37, 30);
        world.set_tile(10, 36, "oDark");
        // Independent canonical seed2741: range30, range2, range100=[0,0,0].
        let mut actual = Rng::new(2741);
        let mut source = actual.clone();
        assert_eq!(source.range(30), 0); // UFO chance succeeds.
        let shift = source.range(2); // Synchronous UFO Create.
        assert_eq!(source.range(100), 0); // Following treasure Rock succeeds.
        populate(&mut world, &mut actual, &entity_gen_only_flags());
        let ufo = world
            .entities
            .iter()
            .find(|entity| entity.object == "oUFO")
            .unwrap();
        let rock = world
            .entities
            .iter()
            .find(|entity| entity.object == "oRock")
            .unwrap();
        assert_eq!(ufo.value("shift_toggle"), f64::from(shift));
        assert_eq!(ufo.pos, Vec2::new(160.0, 560.0));
        assert_eq!(rock.pos, Vec2::new(168.0, 572.0));
        assert!(ufo.creation_serial < rock.creation_serial);
        assert_eq!(actual.next_u64(), source.next_u64());
    }

    #[test]
    fn temple_enemy_precedes_same_support_treasure_draws() {
        // At level13 genTombLord is unconditionally true. The only support
        // cannot make a spear or ceiling enemy: statue rand100 precedes
        // TombLord rand40, then scrTreasureGen Rock rand100 follows Create.
        let mut world = World::new(42, 34, Area::Temple, 13);
        world.entrance = at(1, 1);
        world.exit = at(37, 30);
        world.set_tile(10, 32, "oTemple");
        let seed = (0..1_000_000)
            .find(|&seed| {
                let mut rng = Rng::new(seed);
                rng.range(100) != 0 && rng.range(40) == 0 && rng.range(100) == 0
            })
            .unwrap();
        let mut actual = Rng::new(seed);
        let mut source = actual.clone();
        assert_ne!(source.range(100), 0);
        assert_eq!(source.range(40), 0);
        assert_eq!(source.range(100), 0);
        populate(&mut world, &mut actual, &entity_gen_only_flags());
        let enemy = world
            .entities
            .iter()
            .find(|entity| entity.object == "oTombLord")
            .unwrap();
        let rock = world
            .entities
            .iter()
            .find(|entity| entity.object == "oRock")
            .unwrap();
        assert_eq!(enemy.pos, Vec2::new(160.0, 480.0));
        assert_eq!(rock.pos, Vec2::new(168.0, 508.0));
        assert!(enemy.creation_serial < rock.creation_serial);
        assert_eq!(actual.next_u64(), source.next_u64());
    }

    #[test]
    fn treasure_source_family_guards_block_named_masks_but_allow_enemy_overlap() {
        for blocker in ["oPushBlock", "oChest", "oSpikes", "oEntrance", "oExit"] {
            let mut world = World::new(42, 34, Area::Ice, 9);
            world.entrance = at(1, 1);
            world.exit = at(37, 30);
            world.set_tile(10, 10, "oDark");
            let pos = if blocker == "oChest" {
                Vec2::new(160.0, 152.0)
            } else {
                Vec2::new(160.0, 144.0)
            };
            world.spawn(blocker, pos);
            let water = WaterMasks::new(&world);
            let mut actual = Rng::new(1297);
            let mut source = actual.clone();
            treasure(
                &mut world,
                &mut actual,
                &water,
                &mut treasure_state(),
                10,
                10,
            );
            assert_eq!(world.entities.len(), 1, "{blocker}");
            assert_eq!(
                actual.next_u64(),
                source.next_u64(),
                "{blocker} consumes no loot draw"
            );
        }
        let mut world = World::new(42, 34, Area::Ice, 9);
        world.entrance = at(1, 1);
        world.exit = at(37, 30);
        world.set_tile(10, 10, "oDark");
        world.spawn("oUFO", Vec2::new(160.0, 144.0));
        let water = WaterMasks::new(&world);
        let mut actual = Rng::new(6491);
        let mut source = actual.clone();
        // Enemy presence must not skip any of the source's normal loot rolls.
        if source.range(100) != 0
            && source.range(40) != 0
            && source.range(40) != 0
            && source.range(50) != 0
            && source.range(122) == 0
        {
            source.range(8);
        }
        treasure(
            &mut world,
            &mut actual,
            &water,
            &mut treasure_state(),
            10,
            10,
        );
        assert_eq!(actual.next_u64(), source.next_u64());
    }

    #[test]
    fn existing_treasure_skips_col_stuff_branches_but_keeps_source_normal_fallback() {
        let mut world = World::new(42, 34, Area::Ice, 9);
        world.entrance = at(1, 1);
        world.exit = at(37, 30);
        world.set_tile(10, 10, "oDark");
        world.spawn("oGoldBars", Vec2::new(160.0, 152.0));
        let water = WaterMasks::new(&world);
        // Canonical seed0 wins the first normal-fallback range40 guard.
        let mut actual = Rng::new(0);
        let mut source = actual.clone();
        assert_eq!(source.range(40), 0); // No Rock/ Jar rolls when colStuff=true.
        treasure(
            &mut world,
            &mut actual,
            &water,
            &mut treasure_state(),
            10,
            10,
        );
        assert_eq!(world.entities.len(), 2);
        assert_eq!(world.entities[1].object, "oGoldBar");
        assert_eq!(actual.next_u64(), source.next_u64());
    }

    #[test]
    fn natural_game_api_runs_area_specific_population_phase_order() {
        use crate::{Game, RulesPreset};
        let mut game = Game::new(0, RulesPreset::Classic);
        for level in [1, 5, 9, 13] {
            POPULATION_PHASE_TRACE.with(|trace| *trace.borrow_mut() = Some(Vec::new()));
            game.start_from_hub(false, level);
            let trace = POPULATION_PHASE_TRACE.with(|trace| trace.borrow_mut().take().unwrap());
            let mut supports =
                std::collections::BTreeMap::<(i32, i32), Vec<PopulationPhase>>::new();
            for (destination, x, y, phase) in trace {
                assert_eq!(destination, level);
                supports.entry((x, y)).or_default().push(phase);
            }
            let expected = if level < 9 {
                [PopulationPhase::Treasure, PopulationPhase::Enemies]
            } else {
                [PopulationPhase::Enemies, PopulationPhase::Treasure]
            };
            let mut compared = 0;
            for phases in supports.values().filter(|phases| phases.len() == 2) {
                assert_eq!(phases.as_slice(), expected);
                compared += 1;
            }
            assert!(
                compared > 0,
                "actual level {level} must exercise ordinary supports"
            );
            assert_eq!(game.level_tick, 0);
        }
    }

    fn assert_local_create_completed(entity: &crate::Entity) {
        let marker = match entity.object.as_str() {
            "oFrog" | "oFireFrog" | "oMonkey" | "oPiranha" => "jungle_context_created",
            "oUFO" => "ice_ufo_local_created",
            "oDamsel" | "oScarab" => "npc_local_created",
            "oZombie" => {
                assert_eq!(entity.value("source_facing_pending"), 0.0);
                return;
            }
            _ => panic!("not a scoped generated actor: {}", entity.object),
        };
        assert_eq!(
            entity.value(marker),
            1.0,
            "{} at {:?}",
            entity.object,
            entity.pos
        );
    }

    #[test]
    fn generated_birth_runs_source_random_statement_before_the_next_producer_roll() {
        // Independent source statements: Frog/FireFrog/Monkey/Zombie facing
        // rand(0,1), Piranha dir rand(1,2), UFO shiftToggle rand(0,1),
        // Scarab counter rand(10,30). Damsel Create has no random statement;
        // scrTreasureGen clears its initial price after instance_create returns.
        // New canonical seeds use the full valid u32 boundary; legacy u64
        // import and raw vectors have separate explicit compatibility tests.
        for seed in [0, 1, 23, u32::MAX] {
            let mut world = World::new(12, 12, Area::Jungle, 5);
            // Original oWater is an abstract, sprite-less parent. Use its
            // concrete oWaterSwim child (sWater) for this wet birth premise.
            world.set_tile(4, 4, "oWaterSwim");
            let water = world.tile(4, 4).unwrap();
            assert!(water.engine_active);
            assert!(crate::catalog::is_a(&water.object, "oWater"));
            assert_eq!(crate::catalog::sprite(&water.object), Some("sWater"));
            assert!(world.tile_collision_mask(4, 4).contains(
                Vec2::new(64.0, 64.0),
                crate::CollisionPrecision::BoundingBox
            ));
            let mut actual = Rng::new(seed);
            let mut source = actual.clone();
            for object in [
                "oFrog",
                "oFireFrog",
                "oMonkey",
                "oPiranha",
                "oZombie",
                "oUFO",
                "oScarab",
                "oDamsel",
            ] {
                let expected = match object {
                    "oDamsel" => 0,
                    "oScarab" => 10 + source.range(21),
                    _ => source.range(2),
                };
                spawn(&mut world, &mut actual, object, 4, 4);
                let entity = world.entities.last().unwrap();
                match object {
                    "oUFO" => assert_eq!(entity.value("shift_toggle"), f64::from(expected)),
                    "oScarab" => {
                        assert_eq!(entity.timer, expected);
                        assert_eq!(entity.value("value"), 8000.0);
                    }
                    "oDamsel" => assert_eq!(entity.value("cost"), 0.0),
                    _ => assert_eq!(entity.facing, if expected == 0 { -1 } else { 1 }),
                }
                if matches!(object, "oFrog" | "oFireFrog" | "oMonkey") {
                    assert_eq!(entity.value("swimming"), 1.0);
                }
                assert_local_create_completed(entity);
                // This producer's later decision must use the next source draw,
                // rather than accidentally supplying the pending Create value.
                assert_eq!(actual.range(97), source.range(97), "{object} seed {seed}");
            }
            assert_eq!(actual.next_u64(), source.next_u64());
        }
    }

    #[test]
    fn natural_game_generation_completes_local_create_before_any_room_step() {
        use crate::{Game, RulesPreset};
        crate::creation::begin_generated_create_trace();
        let mut game = Game::new(0, RulesPreset::Classic);
        let mut checked = std::collections::BTreeSet::new();
        let mut birth_counts = std::collections::BTreeMap::<String, usize>::new();
        for level in 1..=9 {
            assert_eq!(game.world.level, level);
            assert_eq!(game.level_tick, 0);
            for trace in crate::creation::take_generated_create_traces() {
                let entity = trace.entity_after;
                let mut source_rng = trace.rng_before;
                match entity.object.as_str() {
                    "oFrog" | "oFireFrog" | "oMonkey" | "oPiranha" | "oZombie" => {
                        assert_eq!(entity.facing, if source_rng.range(2) == 0 { -1 } else { 1 });
                    }
                    "oUFO" => {
                        assert_eq!(entity.value("shift_toggle"), f64::from(source_rng.range(2)))
                    }
                    "oScarab" => assert_eq!(entity.timer, 10 + source_rng.range(21)),
                    "oDamsel" => assert_eq!(entity.value("cost"), 15000.0 * f64::from(level)),
                    _ => continue,
                }
                assert_eq!(trace.level, level);
                *birth_counts.entry(entity.object.clone()).or_default() += 1;
                assert_local_create_completed(&entity);
            }
            let ids: Vec<_> = game
                .world
                .entities
                .iter()
                .filter(|entity| {
                    matches!(
                        entity.object.as_str(),
                        "oFrog"
                            | "oFireFrog"
                            | "oMonkey"
                            | "oPiranha"
                            | "oZombie"
                            | "oUFO"
                            | "oScarab"
                            | "oDamsel"
                    )
                })
                .map(|entity| {
                    assert_local_create_completed(entity);
                    checked.insert(entity.object.clone());
                    entity.id
                })
                .collect();
            let mut source_rng = game.rng.clone();
            for id in ids {
                crate::enemies::create_instance(&mut game, id);
            }
            assert_eq!(
                game.rng.clone().next_u64(),
                source_rng.next_u64(),
                "first-frame bootstrap must not repeat generated Create RNG"
            );
            if level < 9 {
                crate::creation::begin_generated_create_trace();
                game.next_level();
            }
        }
        eprintln!("natural generated local Create births checked: {birth_counts:?}");
        // The seed selects content; no fixed entity IDs, positions or room
        // layouts encode the earlier, incorrectly deferred random stream.
        for object in ["oFrog", "oUFO", "oDamsel"] {
            assert!(checked.contains(object), "fixture must exercise {object}");
        }
    }

    #[test]
    fn source_instance_offsets_are_preserved() {
        let mut world = room_fixture(Area::Mines, 1);
        spawn(&mut world, &mut Rng::new(0), "oBat", 5, 6);
        spawn(&mut world, &mut Rng::new(0), "oGoldBar", 5, 6);
        spawn(&mut world, &mut Rng::new(0), "oLockedChest", 5, 6);
        assert_eq!(world.entities[0].pos, Vec2::new(80.0, 96.0));
        assert_eq!(world.entities[1].pos, Vec2::new(88.0, 108.0));
        assert_eq!(world.entities[2].pos, Vec2::new(88.0, 104.0));
    }

    // These unit fixtures enter the EntityGen phase with InitLevel's two
    // secret chains already completed. Its scheduling has separate tests below.
    fn entity_gen_only_flags() -> BTreeSet<String> {
        BTreeSet::from(["made_udjat_eye".into(), "made_market_entrance".into()])
    }

    fn literal_init_schedule(rng: &mut Rng, level: u32, first: u32) -> bool {
        let first_roll = 1 + rng.range(3);
        if level == first && first_roll == 1 {
            true
        } else {
            let second_roll = 1 + rng.range(2);
            if level == first + 1 && second_roll == 1 {
                true
            } else {
                level == first + 2
            }
        }
    }

    #[test]
    fn eager_init_schedule_retains_repeated_level_guards_and_conditional_else_if() {
        for first in [2, 5] {
            for level in 1..=15 {
                for seed in 0..64 {
                    let mut actual = Rng::new(seed);
                    let mut source = actual.clone();
                    let expected = literal_init_schedule(&mut source, level, first);
                    assert_eq!(
                        schedule(&mut actual, level, first, first + 2),
                        expected,
                        "first={first} level={level} seed={seed}"
                    );
                    assert_eq!(
                        actual.next_u64(),
                        source.next_u64(),
                        "first={first} level={level} seed={seed}"
                    );
                }
            }
        }
    }

    #[test]
    fn eager_init_schedules_obey_made_flags_and_preserve_explicit_fixture_overrides() {
        for level in [1, 2, 3, 4, 5, 6, 7, 9, 13, 14] {
            for made_udjat in [false, true] {
                for made_market in [false, true] {
                    for forced in [false, true] {
                        let area = Area::for_level(level);
                        let mut world = World::new(42, 38, area, level);
                        world.entrance = at(1, 1);
                        world.exit = at(38, 30);
                        let mut flags = BTreeSet::new();
                        if made_udjat {
                            flags.insert("made_udjat_eye".into());
                        }
                        if made_market {
                            flags.insert("made_market_entrance".into());
                        }
                        if forced {
                            // These are API fixture overrides, not source globals
                            // surviving scrInitLevel's reset statements.
                            flags.insert("gen_udjat_eye".into());
                            flags.insert("gen_market_entrance".into());
                            flags.insert("black_market".into());
                        }
                        let mut actual = Rng::new(0);
                        let mut source = actual.clone();
                        let udjat = if made_udjat {
                            false
                        } else {
                            let scheduled = literal_init_schedule(&mut source, level, 2);
                            area == Area::Mines && (scheduled || forced)
                        };
                        if !made_market {
                            literal_init_schedule(&mut source, level, 5);
                        }
                        if area == Area::Mines {
                            source.range(6);
                        }
                        if area == Area::Temple && level != 13 {
                            source.range(4);
                        }
                        populate(&mut world, &mut actual, &flags);
                        assert_eq!(
                            exists(&world, "oLockedChest"),
                            udjat,
                            "level={level} made={made_udjat}/{made_market} forced={forced}"
                        );
                        assert_eq!(
                            actual.next_u64(),
                            source.next_u64(),
                            "level={level} made={made_udjat}/{made_market} forced={forced}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn eager_market_schedule_reaches_actual_entrance_placement() {
        // InitLevel :187-193 has only the made flag as its outer guard.
        // Udjat is already completed here, isolating the Market chain's
        // guard-entry RNG. Historical SplitMix tapes are independent of placement:
        // seed3 => range3=0; seed0 => range3=1, range2=0;
        // seed1 => range3=2, range2=1. Explicit Legacy preserves these frozen
        // outcomes; canonical scheduling is covered by the literal-source tests.
        for (level, seed, made, forced, black_market, expected) in [
            (5, 3, false, false, false, true),
            (5, 0, false, false, false, false),
            (6, 0, false, false, false, true),
            (6, 1, false, false, false, false),
            (7, 1, false, false, false, true),
            (8, 3, false, false, false, false),
            (5, 3, false, false, true, true),
            (5, 0, false, true, false, true),
            (5, 3, true, true, false, false),
        ] {
            let mut source = Rng::legacy(seed);
            let scheduled = if made {
                false
            } else {
                literal_init_schedule(&mut source, level, 5)
            };
            assert_eq!(!made && (scheduled || forced), expected);

            let mut world = World::new(12, 12, Area::Jungle, level);
            world.entrance = at(6, 5);
            world.exit = at(10, 10);
            // One eligible stacked support makes the existing entrance
            // placement adapter observable without changing its geometry.
            // Both treasure calls are naturally near the authored entrance.
            world.set_tile(6, 5, "oLush");
            world.set_tile(6, 6, "oLush");
            assert!(solid(&world, 6, 5) && solid(&world, 6, 6));
            let mut flags = BTreeSet::from(["made_udjat_eye".into()]);
            if made {
                flags.insert("made_market_entrance".into());
            }
            if forced {
                flags.insert("gen_market_entrance".into());
            }
            if black_market {
                flags.insert("black_market".into());
            }
            populate(&mut world, &mut Rng::legacy(seed), &flags);
            let entrances: Vec<_> = world
                .entities
                .iter()
                .filter(|entity| entity.alive && entity.object == "oXMarket")
                .collect();
            assert_eq!(
                entrances.len(),
                usize::from(expected),
                "level={level} seed={seed} made={made} forced={forced} market={black_market}"
            );
            if expected {
                assert_eq!(entrances[0].pos, Vec2::new(96.0, 80.0));
            }
        }
    }

    #[test]
    fn eager_natural_ice_generation_uses_literal_spring_then_yeti_stream() {
        let mut non_dark = 0;
        let mut allowed = 0;
        let mut disallowed = 0;
        let mut springs = 0;
        let mut yetis = 0;
        for seed in [0, 1, 42] {
            ICE_FLOOR_ROLL_TRACE.with(|trace| *trace.borrow_mut() = Some(Vec::new()));
            let mut game = crate::Game::shell(seed, crate::RulesPreset::Classic);
            game.start_from_hub(false, 9);
            let traces = ICE_FLOOR_ROLL_TRACE.with(|trace| trace.borrow_mut().take().unwrap());
            assert!(!traces.is_empty(), "generated seed={seed}");
            for trace in traces {
                non_dark += usize::from(!trace.dark_support);
                allowed += usize::from(trace.spring_allowed);
                disallowed += usize::from(!trace.spring_allowed);
                let mut source = trace.before;
                let spring_roll = 1 + source.range(10);
                let object = if spring_roll == 1 && trace.spring_allowed {
                    springs += 1;
                    Some("oSpringTrap")
                } else if 1 + source.range(20) == 1 {
                    yetis += 1;
                    Some("oYeti")
                } else {
                    None
                };
                let expected: Vec<_> = object
                    .into_iter()
                    .map(|object| {
                        (
                            object.to_owned(),
                            Vec2::new(trace.origin.x, trace.origin.y - 16.0),
                        )
                    })
                    .collect();
                assert_eq!(
                    trace.births, expected,
                    "seed={seed} origin={:?}",
                    trace.origin
                );
                let mut actual = trace.after;
                assert_eq!(
                    actual.next_u64(),
                    source.next_u64(),
                    "seed={seed} origin={:?}",
                    trace.origin
                );
            }
            assert_eq!(game.world.area, Area::Ice);
            assert_eq!(game.player.health, 4);
        }
        assert!(non_dark > 0 && allowed > 0 && disallowed > 0);
        assert!(
            springs > 0 && yetis > 0,
            "actual generation must cover both branch births"
        );
    }

    #[test]
    fn eager_ice_spring_floor_keeps_roll_before_sprite_and_exit_guards() {
        for (object, near_exit, seed) in [
            ("oIce", false, 0),
            ("oIce", false, 2),
            ("oDark", false, 2),
            ("oDark", true, 2),
        ] {
            let mut world = World::new(42, 38, Area::Ice, 9);
            world.entrance = at(1, 1);
            world.exit = if near_exit { at(8, 6) } else { at(38, 30) };
            world.set_tile(6, 6, object);
            // Same start room suppresses the earlier UFO/ceiling branch.
            assert!(start_room(&world, 6, 5));
            assert!(at(6, 6).distance(world.entrance) > 64.0);
            // An authored nearby idol suppresses only later treasure generation.
            world.spawn("oGoldIdol", Vec2::new(136.0, 116.0));
            let flags = BTreeSet::from(["made_udjat_eye".into(), "made_market_entrance".into()]);
            let mut actual = Rng::new(seed);
            let mut source = actual.clone();
            let spring_roll = source.range(10) == 0;
            let spring = spring_roll && object == "oDark" && !near_exit;
            let yeti = !spring && source.range(20) == 0;
            populate(&mut world, &mut actual, &flags);
            assert_eq!(
                exists(&world, "oSpringTrap"),
                spring,
                "{object} near_exit={near_exit} seed={seed}"
            );
            assert_eq!(exists(&world, "oYeti"), yeti);
            assert_eq!(actual.next_u64(), source.next_u64());
        }
    }

    #[test]
    fn eager_ice_spring_outer_floor_exclusion_suppresses_all_branch_draws() {
        let mut world = World::new(42, 38, Area::Ice, 9);
        world.entrance = at(6, 6);
        world.exit = at(38, 30);
        world.set_tile(6, 6, "oDark");
        let flags = BTreeSet::from(["made_udjat_eye".into(), "made_market_entrance".into()]);
        let mut actual = Rng::new(2);
        let mut source = actual.clone();
        populate(&mut world, &mut actual, &flags);
        assert!(!exists(&world, "oSpringTrap") && !exists(&world, "oYeti"));
        assert_eq!(actual.next_u64(), source.next_u64());
    }

    #[test]
    fn final_mines_level_guarantees_chest_and_key() {
        for seed in 0..32 {
            let mut world = room_fixture(Area::Mines, 4);
            populate(&mut world, &mut Rng::new(seed), &BTreeSet::new());
            assert!(exists(&world, "oLockedChest"));
            assert!(exists(&world, "oKey"));
            assert_eq!(
                world
                    .entities
                    .iter()
                    .filter(|e| e.alive && e.object == "oLockedChest")
                    .count(),
                1
            );
        }
    }

    #[test]
    fn completed_udjat_chain_does_not_generate_another_chest() {
        let mut world = room_fixture(Area::Mines, 4);
        let flags = BTreeSet::from(["made_udjat_eye".to_owned()]);
        populate(&mut world, &mut Rng::new(17), &flags);
        assert!(!exists(&world, "oLockedChest"));
        assert!(!exists(&world, "oKey"));
    }

    #[test]
    fn population_replays_exactly() {
        let mut a = room_fixture(Area::Jungle, 7);
        let mut b = a.clone();
        populate(&mut a, &mut Rng::new(42), &BTreeSet::new());
        populate(&mut b, &mut Rng::new(42), &BTreeSet::new());
        assert_eq!(
            serde_json::to_string(&a).unwrap(),
            serde_json::to_string(&b).unwrap()
        );
        assert!(exists(&a, "oXMarket"));
    }

    #[test]
    fn dark_levels_supply_flares_and_temple_fourteen_has_gold_door() {
        let mut world = room_fixture(Area::Temple, 14);
        world.dark = true;
        populate(&mut world, &mut Rng::new(9), &BTreeSet::new());
        assert!(exists(&world, "oGoldDoor"));
        assert!(exists(&world, "oFlareCrate"));
        let crate_pos = world
            .entities
            .iter()
            .find(|e| e.object == "oFlareCrate")
            .unwrap()
            .pos;
        assert!(!world.solid_at(crate_pos.x, crate_pos.y));
    }
}

#[path = "jungle_primary.rs"]
mod jungle_primary;

#[cfg(test)]
#[path = "cemetery_v2_tests.rs"]
mod cemetery_v2_tests;

#[cfg(test)]
#[path = "population_shop_path_tests.rs"]
mod population_shop_path_tests;

#[cfg(test)]
#[path = "ice_ceiling_eager_rng_tests.rs"]
mod ice_ceiling_eager_rng_tests;
