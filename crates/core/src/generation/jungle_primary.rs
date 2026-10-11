//! Versioned scrEntityGen208–319 Jungle primary and forced-market passes.
//! Earlier room/boundary construction and shared Treasure helper limitations
//! remain separate from this phase's live receivers and statement ordering.
use super::*;

#[cfg(test)]
struct MarketTrace {
    before_bg: Rng,
    before_roll: Rng,
    after_roll: Rng,
    current_kind: String,
    upper_kind: String,
    upper_invincible: bool,
    origin: Vec2,
    chance_before: i32,
    chance_after: i32,
    made_after: bool,
    births: Vec<(String, Vec2)>,
}
#[cfg(test)]
thread_local! {
    static MARKET_TRACE: std::cell::RefCell<Option<Vec<MarketTrace>>> = const { std::cell::RefCell::new(None) };
}

fn receiver(
    world: &World,
    serial: u64,
    instance: FamilyInstance,
    origin: Vec2,
) -> Option<TempleReceiver> {
    let active = match instance {
        FamilyInstance::Tile(index) => world.tiles.get(index)?.engine_active,
        FamilyInstance::Entity(id) => world.entity(id)?.engine_active,
    };
    if !active {
        return None;
    }
    let mut receiver = temple_receiver(world, serial, instance, origin)?;
    // Temple's helper prepares a clean Destroy; this live frame has not run it.
    let clean = match instance {
        FamilyInstance::Tile(_) => world
            .entities
            .iter()
            .find(|e| {
                e.alive
                    && e.object == "oTerrainMetadata"
                    && e.pos == Vec2::new(origin.x + 8.0, origin.y + 8.0)
            })
            .map_or(0.0, |e| e.value("cleanDeath")),
        FamilyInstance::Entity(id) => world.entity(id)?.value("cleanDeath"),
    };
    receiver.source.data.insert("cleanDeath".into(), clean);
    Some(receiver)
}
fn same(a: FamilyInstance, b: FamilyInstance) -> bool {
    match (a, b) {
        (FamilyInstance::Tile(a), FamilyInstance::Tile(b)) => a == b,
        (FamilyInstance::Entity(a), FamilyInstance::Entity(b)) => a == b,
        _ => false,
    }
}
fn live_solid(world: &World) -> impl Iterator<Item = TempleReceiver> + '_ {
    family_instances(world, "oSolid")
        .into_iter()
        .filter_map(|(serial, instance, origin)| receiver(world, serial, instance, origin))
}
fn point(world: &World, position: Vec2, family: &str) -> bool {
    world.tiles.iter().enumerate().any(|(index, tile)| {
        tile.engine_active
            && crate::catalog::is_a(&tile.object, family)
            && world
                .tile_collision_mask((index % world.width) as i32, (index / world.width) as i32)
                .contains(position, crate::CollisionPrecision::BoundingBox)
    }) || world.entities.iter().any(|e| {
        e.alive
            && e.engine_active
            && crate::catalog::is_a(&e.object, family)
            && e.mask_contains(position, crate::CollisionPrecision::BoundingBox)
    })
}
fn rectangle(world: &World, query: Rect, family: &str, ignore: Option<FamilyInstance>) -> bool {
    // Inclusive raw query endpoints versus the target's integer point bbox,
    // as independently reviewed in the rectangle caller increment.
    let intersects = |target: Rect| {
        target.is_valid()
            && query.is_valid()
            && target.x + target.w > query.x
            && target.x <= query.x + query.w - 1.0
            && target.y + target.h > query.y
            && target.y <= query.y + query.h - 1.0
    };
    world.tiles.iter().enumerate().any(|(index, tile)| {
        tile.engine_active
            && !ignore.is_some_and(|ignored| same(ignored, FamilyInstance::Tile(index)))
            && crate::catalog::is_a(&tile.object, family)
            && intersects(
                world
                    .tile_collision_mask((index % world.width) as i32, (index / world.width) as i32)
                    .point_bounds(),
            )
    }) || world.entities.iter().any(|e| {
        e.alive
            && e.engine_active
            && !ignore.is_some_and(|ignored| same(ignored, FamilyInstance::Entity(e.id)))
            && crate::catalog::is_a(&e.object, family)
            && intersects(e.collision_mask().point_bounds())
    })
}
fn place_solid(world: &World, source: &TempleReceiver, position: Vec2) -> Option<TempleReceiver> {
    let mut translated = source.source.clone();
    translated.pos = position;
    let mask = translated.collision_mask();
    live_solid(world).find(|target| {
        !same(source.instance, target.instance)
            && mask.intersects_mask(&target.source.collision_mask())
    })
}
fn in_shop(path: &[[u8; 4]; 5], position: Vec2) -> bool {
    let (x, y) = source_room(position);
    matches!(path[y as usize][x as usize], 4 | 5)
}
fn write(world: &mut World, instance: FamilyInstance, key: &str, value: f64) {
    cemetery_receiver_value(world, instance, key, value);
}
fn retire(world: &mut World, rng: &mut Rng, receiver: &TempleReceiver, spectacles: bool) {
    write(world, receiver.instance, "cleanDeath", 1.0);
    let mut source = receiver.source.clone();
    source.data.insert("cleanDeath".into(), 1.0);
    // Source Destroy executes synchronously while its receiver is still live.
    crate::items::explosives::destroy_source_jungle_support_clean(world, rng, &source, spectacles);
    let callback_dirty = world.fluid_dirty;
    match receiver.instance {
        FamilyInstance::Tile(index) => {
            world.set_tile(index % world.width, index / world.width, "");
            let center = Vec2::new(receiver.origin.x + 8.0, receiver.origin.y + 8.0);
            for metadata in world
                .entities
                .iter_mut()
                .filter(|e| e.alive && e.object == "oTerrainMetadata" && e.pos == center)
            {
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
    world.fluid_dirty = callback_dirty;
}
fn spawn(world: &mut World, rng: &mut Rng, object: &str, position: Vec2, spectacles: bool) -> u64 {
    crate::creation::spawn_generated_jungle_child(world, rng, object, position, spectacles)
}
fn background(world: &mut World, position: Vec2) {
    let id = world.spawn(
        "oGenerationDecoration",
        Vec2::new(position.x, position.y - 32.0),
    );
    let tile = world.entity_mut(id).unwrap();
    tile.sprite = "bgTrees".into();
    for (key, value) in [
        ("decoration", 1.0),
        ("source_x", 0.0),
        ("source_y", 0.0),
        ("source_w", 16.0),
        ("source_h", 48.0),
        ("depth", 9005.0),
    ] {
        tile.data.insert(key.into(), value);
    }
}

pub(super) fn populate(
    world: &mut World,
    rng: &mut Rng,
    water: &WaterMasks,
    treasure_state: &mut TreasureState,
    campaign: &mut super::super::GenerationState,
    room_path: &[[u8; 4]; 5],
    gen_market: bool,
    spectacles: bool,
) {
    let entrance = family_origins(world, "oEntrance")
        .into_iter()
        .next()
        .expect("source-authored Jungle room has an Entrance instance");
    let start = source_room(Vec2::new(world.entrance.x - 8.0, world.entrance.y - 8.0));
    let dark = world.dark;
    let cemetery = treasure_state.cemetery;
    let black_market = world.name == "Black Market";
    // This body creates Spear Solid descendants, but source with() captures
    // its count before those births. Later original receivers are revalidated;
    // a current receiver destroyed in its body still finishes that body.
    let receivers = family_instances(world, "oSolid");
    for (serial, instance, origin) in receivers {
        let Some(receiver) = receiver(world, serial, instance, origin) else {
            continue;
        };
        let above = Vec2::new(origin.x, origin.y - 16.0);
        let high = Vec2::new(origin.x, origin.y - 32.0);
        #[cfg(test)]
        let before_bg = MARKET_TRACE.with(|trace| trace.borrow().as_ref().map(|_| rng.clone()));
        let bg_roll = rng.range(100) == 0;
        if bg_roll && !point(world, above, "oSolid") {
            background(world, origin);
        }
        if in_shop(room_path, origin) {
            continue;
        }
        if origin.y > 32.0
            && point(world, above, "oSolid")
            && gen_market
            && !campaign.made_market_entrance
        {
            let upper = place_solid(world, &receiver, above)
                .expect("source instance_place requires a translated Solid receiver");
            write(world, instance, "obj", upper.source.id as f64);
            #[cfg(test)]
            let trace_before = before_bg.map(|before_bg| {
                (
                    before_bg,
                    rng.clone(),
                    campaign.market_chance,
                    world.entities.len(),
                )
            });
            let roll = rng
                .source_rand(1, campaign.market_chance)
                .expect("prevalidated normal-campaign signed marketChance domain");
            if upper.kind != "Tree"
                && receiver.kind != "Altar"
                && upper.source.value("invincible") == 0.0
                && roll <= 1
            {
                spawn(world, rng, "oXMarket", above, spectacles);
                write(world, instance, "invincible", 1.0);
                campaign.made_market_entrance = true;
            } else {
                campaign.market_chance -= 1;
            }
            #[cfg(test)]
            if let Some((before_bg, before_roll, chance_before, old_len)) = trace_before {
                MARKET_TRACE.with(|trace| {
                    if let Some(trace) = trace.borrow_mut().as_mut() {
                        trace.push(MarketTrace {
                            before_bg,
                            before_roll,
                            after_roll: rng.clone(),
                            current_kind: receiver.kind.clone(),
                            upper_kind: upper.kind.clone(),
                            upper_invincible: upper.source.value("invincible") != 0.0,
                            origin,
                            chance_before,
                            chance_after: campaign.market_chance,
                            made_after: campaign.made_market_entrance,
                            births: world.entities[old_len..]
                                .iter()
                                .map(|e| (e.object.clone(), e.pos))
                                .collect(),
                        });
                    }
                });
            }
        } else if receiver.kind != "Tree"
            && receiver.kind != "Altar"
            && origin.y != 0.0
            && !rectangle(
                world,
                Rect::new(origin.x, origin.y - 32.0, 16.0, 32.0),
                "oSolid",
                Some(instance),
            )
            && !rectangle(
                world,
                Rect::new(origin.x, origin.y - 16.0, 16.0, 16.0),
                "oEnemy",
                None,
            )
            && (!point(world, Vec2::new(origin.x - 16.0, origin.y), "oSolid")
                || !point(world, Vec2::new(origin.x + 16.0, origin.y), "oSolid"))
            && point(world, Vec2::new(origin.x, origin.y + 16.0), "oSolid")
            && !point(world, origin, "oXMarket")
            && !in_shop(room_path, origin)
            && origin.distance(entrance) > 64.0
        {
            let torch_roll = rng.range(20) == 0;
            if dark && !point(world, high, "oWater") && torch_roll {
                spawn(world, rng, "oTikiTorch", high, spectacles);
            } else {
                let spear_roll = rng.range(12) == 0;
                if spear_roll && ![160.0, 176.0, 320.0, 336.0, 480.0, 496.0].contains(&origin.x) {
                    if point(world, above, "oSolid") {
                        let nearest = live_solid(world).min_by(|a, b| {
                            a.origin
                                .distance(above)
                                .total_cmp(&b.origin.distance(above))
                        });
                        if let Some(upper) = nearest {
                            retire(world, rng, &upper, spectacles);
                        }
                    }
                    spawn(world, rng, "oSpearTrapBottom", origin, spectacles);
                    spawn(
                        world,
                        rng,
                        if dark {
                            "oSpearTrapLit"
                        } else {
                            "oSpearTrapTop"
                        },
                        above,
                        spectacles,
                    );
                    retire(world, rng, &receiver, spectacles);
                }
            }
        }
        // Deliberately use the captured frame after self Destroy. New Spears
        // and callback side effects already participate in these live queries.
        if receiver.kind != "Altar" {
            #[cfg(test)]
            trace_population_phase(
                world,
                (origin.x / 16.0) as i32,
                (origin.y / 16.0) as i32,
                PopulationPhase::Treasure,
            );
            treasure_at(
                world,
                rng,
                water,
                treasure_state,
                origin,
                Some(receiver.bounds),
            );
        }
        let destination = source_room(above);
        if destination.0 != start.0 || destination.1 != start.1 {
            #[cfg(test)]
            trace_population_phase(
                world,
                (origin.x / 16.0) as i32,
                (origin.y / 16.0) as i32,
                PopulationPhase::Enemies,
            );
            if origin.y < world.height as f64 * 16.0 - 64.0
                && !point(world, Vec2::new(origin.x, origin.y + 16.0), "oSolid")
                && !point(world, Vec2::new(origin.x, origin.y + 32.0), "oSolid")
                && !point(world, Vec2::new(origin.x, origin.y + 16.0), "oWater")
                && !point(world, Vec2::new(origin.x, origin.y + 32.0), "oWater")
            {
                let scarab_roll = rng.range(40) == 0;
                if dark && scarab_roll {
                    spawn(
                        world,
                        rng,
                        "oScarab",
                        Vec2::new(origin.x, origin.y + 16.0),
                        spectacles,
                    );
                } else if rng.range(if cemetery { 60 } else { 80 }) == 0 {
                    spawn(
                        world,
                        rng,
                        "oBat",
                        Vec2::new(origin.x, origin.y + 16.0),
                        spectacles,
                    );
                }
            }
            if origin.y > 16.0
                && !point(world, above, "oSolid")
                && !point(world, origin, "oEnemy")
                && !point(world, origin, "oSpikes")
            {
                if cemetery {
                    if rng.range(25) == 0 {
                        spawn(world, rng, "oZombie", above, spectacles);
                    } else if rng.range(160) == 0 {
                        spawn(world, rng, "oVampire", above, spectacles);
                    }
                } else if !point(world, above, "oWater") {
                    let n = if black_market && origin.y % 128.0 == 0.0 {
                        0
                    } else {
                        1
                    };
                    if 1 + rng.range(60) == n {
                        spawn(world, rng, "oManTrap", above, spectacles);
                    } else if rng.range(60) == 0 {
                        spawn(world, rng, "oCaveman", above, spectacles);
                    } else if rng.range(120) == 0 {
                        spawn(world, rng, "oFireFrog", above, spectacles);
                    } else if rng.range(30) == 0 {
                        spawn(world, rng, "oFrog", above, spectacles);
                    }
                } else if rng.range(120) == 0 {
                    spawn(world, rng, "oFireFrog", above, spectacles);
                } else if rng.range(30) == 0 {
                    spawn(world, rng, "oFrog", above, spectacles);
                }
            }
        }
    }
    // Outer guard only: original source deliberately keeps visiting every
    // eligible receiver after its first forced birth, including shop rooms.
    if gen_market && !campaign.made_market_entrance {
        for (serial, instance, origin) in family_instances(world, "oSolid") {
            let Some(receiver) = receiver(world, serial, instance, origin) else {
                continue;
            };
            let above = Vec2::new(origin.x, origin.y - 16.0);
            if origin.y > 32.0 && point(world, above, "oSolid") {
                let upper = place_solid(world, &receiver, above)
                    .expect("source forced instance_place translated Solid target");
                write(world, instance, "obj", upper.source.id as f64);
                if upper.kind != "Tree"
                    && receiver.kind != "Altar"
                    && upper.source.value("invincible") == 0.0
                {
                    spawn(world, rng, "oXMarket", above, spectacles);
                    write(world, instance, "invincible", 1.0);
                    campaign.made_market_entrance = true;
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "jungle_primary_tests.rs"]
mod tests;
