//! Source-based population rules from Classic's scrEntityGen/scrTreasureGen.
//!
//! Entity coordinates preserve GameMaker instance origins; entrance/exit
//! coordinates remain tile centers. The source's probability chains remain
//! conditional: a second roll is made only when the preceding roll failed.
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
fn in_shop(world: &World, x: i32, y: i32) -> bool {
    let cell = room(at(x, y));
    // Source isInShop uses the room-path shop marker. Shopkeeper instances
    // identify those same room cells after template expansion.
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
fn spawn(world: &mut World, name: &str, x: i32, y: i32) {
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
    world.spawn(
        name,
        Vec2::new(x as f64 * 16.0 + offset.x, y as f64 * 16.0 + offset.y),
    );
}
fn exists(world: &World, name: &str) -> bool {
    world.entities.iter().any(|e| e.alive && e.object == name)
}
fn bones(world: &mut World, rng: &mut Rng, x: i32, y: i32) {
    if rng.chance(1, 8) {
        spawn(world, "oFakeBones", x, y);
    } else {
        spawn(world, "oBones", x, y);
        world.spawn(
            "oSkull",
            Vec2::new(x as f64 * 16.0 + 12.0, y as f64 * 16.0 + 12.0),
        );
    }
}

struct TreasureState {
    udjat: bool,
    chest_chance: u32,
    damsel: bool,
    cemetery: bool,
}

fn treasure(
    world: &mut World,
    rng: &mut Rng,
    water: &WaterMasks,
    state: &mut TreasureState,
    x: i32,
    y: i32,
) {
    let pos = at(x, y);
    if pos.distance(world.entrance) < 32.0
        || pos.distance(world.exit) < 32.0
        || world
            .entities
            .iter()
            .any(|e| e.object == "oGoldIdol" && e.pos.distance(pos) < 64.0)
        || solid(world, x, y - 1)
        || occupied(world, x, y - 1)
    {
        return;
    }
    if rng.chance(1, 100) {
        spawn(world, "oRock", x, y - 1);
        return;
    }
    if rng.chance(1, 40) {
        spawn(world, "oJar", x, y - 1);
        return;
    }
    let side = solid(world, x - 1, y - 1) || solid(world, x + 1, y - 1);
    let alcove = solid(world, x, y - 2) && side;
    let tunnel = solid(world, x - 1, y - 1) && solid(world, x + 1, y - 1);
    let bonus = if state.cemetery { 10 } else { 0 };
    let level = world.level;
    if alcove {
        if world.area != Area::Ice && rng.chance(1, 60) {
            spawn(world, "oWeb", x, y - 1);
        } else if state.udjat && !exists(world, "oLockedChest") {
            if rng.chance(1, state.chest_chance) {
                spawn(world, "oLockedChest", x, y - 1);
            } else {
                state.chest_chance = state.chest_chance.saturating_sub(1).max(1);
            }
        } else if rng.chance(1, 10) {
            spawn(world, "oCrate", x, y - 1);
        } else if rng.chance(1, 15) {
            spawn(world, "oChest", x, y - 1);
        } else if !state.damsel && rng.chance(1, 8) && !water.wet(x, y - 1) {
            spawn(world, "oDamsel", x, y - 1);
            state.damsel = true;
        } else if rng.chance(1 + bonus, 40_u32.saturating_sub(2 * level).max(1)) {
            bones(world, rng, x, y - 1);
        } else if rng.chance(1, 3) {
            spawn(world, "oGoldBar", x, y - 1);
        } else if rng.chance(1, 6) {
            spawn(world, "oGoldBars", x, y - 1);
        } else if rng.chance(1, 6) {
            spawn(world, "oEmeraldBig", x, y - 1);
        } else if rng.chance(1, 8) {
            spawn(world, "oSapphireBig", x, y - 1);
        } else if rng.chance(1, 10) {
            spawn(world, "oRubyBig", x, y - 1);
        }
    } else if tunnel {
        if world.area != Area::Ice && rng.chance(1, 60) {
            spawn(world, "oWeb", x, y - 1);
        } else if rng.chance(1, 4) {
            spawn(world, "oGoldBar", x, y - 1);
        } else if rng.chance(1, 8) {
            spawn(world, "oGoldBars", x, y - 1);
        } else if rng.chance(1 + bonus, 80_u32.saturating_sub(level).max(1)) {
            bones(world, rng, x, y - 1);
        } else if rng.chance(1, 8) {
            spawn(world, "oEmeraldBig", x, y - 1);
        } else if rng.chance(1, 9) {
            spawn(world, "oSapphireBig", x, y - 1);
        } else if rng.chance(1, 10) {
            spawn(world, "oRubyBig", x, y - 1);
        }
    } else if rng.chance(1, 40) {
        spawn(world, "oGoldBar", x, y - 1);
    } else if rng.chance(1, 50) {
        spawn(world, "oGoldBars", x, y - 1);
    } else if rng.chance(1 + bonus, 140_u32.saturating_sub(2 * level).max(1)) {
        bones(world, rng, x, y - 1);
    }
}

fn schedule(rng: &mut Rng, level: u32, first: u32, last: u32) -> bool {
    level >= first && level <= last && (level == last || rng.chance(1, last - level + 1))
}

pub(super) fn populate(world: &mut World, rng: &mut Rng, flags: &BTreeSet<String>) {
    if world.area == Area::Boss {
        return;
    }
    let water = WaterMasks::new(world);
    let cemetery = flag(flags, &["cemetery", "cemetary"]);
    let market = flag(flags, &["black_market", "blackMarket"]);
    let yeti = flag(flags, &["yeti_lair", "yetiLair"]);
    let udjat = world.area == Area::Mines
        && !flag(flags, &["made_udjat_eye", "madeUdjatEye"])
        && (flag(flags, &["gen_udjat_eye", "genUdjatEye"]) || schedule(rng, world.level, 2, 4));
    let need_market = world.area == Area::Jungle
        && !market
        && !flag(flags, &["made_market_entrance", "madeMarketEntrance"])
        && (flag(flags, &["gen_market_entrance", "genMarketEntrance"])
            || schedule(rng, world.level, 5, 7));
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
    // Snapshot iteration mirrors `with oSolid`: spawned entities are not terrain.
    let supports: Vec<_> = (1..world.height as i32 - 1)
        .flat_map(|y| (1..world.width as i32 - 1).map(move |x| (x, y)))
        .filter(|&(x, y)| solid(world, x, y))
        .collect();
    for &(x, y) in &supports {
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
        if !altar && !tree {
            treasure(world, rng, &water, &mut state, x, y);
        }
        match world.area {
            Area::Mines => {
                if start_room(world, x, y - 1) {
                    continue;
                }
                if ceiling && !occupied(world, x, y + 1) {
                    if giant_spider
                        && !exists(world, "oGiantSpiderHang")
                        && !solid(world, x + 1, y + 1)
                        && !solid(world, x + 1, y + 2)
                        && rng.chance(1, 40)
                    {
                        spawn(world, "oGiantSpiderHang", x, y + 1);
                    } else if world.dark && rng.chance(1, 60) {
                        spawn(world, "oLamp", x, y + 1);
                    } else if world.dark && rng.chance(1, 40) {
                        spawn(world, "oScarab", x, y + 1);
                    } else if rng.chance(1, 60) {
                        spawn(world, "oBat", x, y + 1);
                    } else if rng.chance(1, 80) {
                        spawn(world, "oSpiderHang", x, y + 1);
                    }
                }
                if floor && !occupied(world, x, y - 1) {
                    if rng.chance(1, 60) {
                        spawn(world, "oSnake", x, y - 1);
                    } else if rng.chance(1, 800) {
                        spawn(world, "oCaveman", x, y - 1);
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
                        spawn(world, "oXMarket", x, y - 1);
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
                    spawn(world, "oGrave", x, y - 1);
                    if !exists(world, "oShotgun") && rng.chance(1, 40) {
                        if let Some(grave) = world.entities.last_mut() {
                            grave.sprite = "sGraveAsh".into();
                        }
                        spawn(world, "oShotgun", x, y);
                    } else if rng.chance(1, 2) {
                        spawn(world, "oGoldNugget", x, y);
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
                        spawn(world, "oScarab", x, y + 1);
                    } else if rng.chance(1, if cemetery { 60 } else { 80 }) {
                        spawn(world, "oBat", x, y + 1);
                    }
                }
                if y > 1 && floor && !occupied(world, x, y - 1) {
                    if cemetery {
                        if rng.chance(1, 25) {
                            spawn(world, "oZombie", x, y - 1);
                        } else if rng.chance(1, 160) {
                            spawn(world, "oVampire", x, y - 1);
                        }
                    } else if !water.wet(x, y - 1) && !(market && y % 8 == 0) && rng.chance(1, 60) {
                        spawn(world, "oManTrap", x, y - 1);
                    } else if !water.wet(x, y - 1) && rng.chance(1, 60) {
                        spawn(world, "oCaveman", x, y - 1);
                    } else if rng.chance(1, 120) {
                        spawn(world, "oFireFrog", x, y - 1);
                    } else if rng.chance(1, 30) {
                        spawn(world, "oFrog", x, y - 1);
                    }
                }
            }
            Area::Ice => {
                if !start_room(world, x, y - 1) {
                    if ceiling {
                        if world.dark && rng.chance(1, 40) {
                            spawn(world, "oScarab", x, y + 1);
                        }
                    } else if floor
                        && y > 1
                        && y < 37
                        && !occupied(world, x, y - 1)
                        && rng.chance(1, if yeti { 90 } else { 30 })
                    {
                        spawn(world, "oUFO", x, y - 1);
                    }
                }
                if floor && y > 1 && y < 37 && far && !occupied(world, x, y - 1) {
                    if tile == "oDark"
                        && !solid(world, x, y - 2)
                        && !solid(world, x, y - 3)
                        && !solid(world, x, y - 4)
                        && at(x, y).distance(world.exit) > 64.0
                        && rng.chance(1, 10)
                    {
                        spawn(world, "oSpringTrap", x, y - 1);
                    } else if rng.chance(1, 20) {
                        spawn(world, "oYeti", x, y - 1);
                    }
                }
            }
            Area::Temple => {
                if world.level == 14 && y > 2 && floor && !exists(world, "oGoldDoor") {
                    if rng.chance(1, gold_chance) {
                        spawn(world, "oGoldDoor", x, y - 1);
                    } else {
                        gold_chance = gold_chance.saturating_sub(1).max(1);
                    }
                } else {
                    spear(world, rng, &water, x, y, altar || tree, far);
                }
                if ceiling && world.dark && rng.chance(1, 40) {
                    spawn(world, "oScarab", x, y + 1);
                }
                if !start_room(world, x, y - 1) && floor && y > 1 && !occupied(world, x, y - 1) {
                    if tomb_lord
                        && !exists(world, "oTombLord")
                        && !solid(world, x, y - 2)
                        && !solid(world, x + 1, y - 2)
                        && !solid(world, x + 1, y - 1)
                        && rng.chance(1, 40)
                    {
                        spawn(world, "oTombLord", x, y - 2);
                    } else if rng.chance(1, 40) {
                        spawn(world, "oCaveman", x, y - 1);
                    } else if rng.chance(1, 40) {
                        spawn(world, "oHawkman", x, y - 1);
                    } else if rng.chance(1, 60) {
                        spawn(
                            world,
                            if world.dark {
                                "oSmashTrapLit"
                            } else {
                                "oSmashTrap"
                            },
                            x,
                            y - 1,
                        );
                    }
                }
            }
            Area::Boss => {}
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
            spawn(world, "oXMarket", x, y - 1);
        }
    }
    if world.area == Area::Temple && world.level == 14 && !exists(world, "oGoldDoor") {
        if let Some(&(x, y)) = supports
            .iter()
            .find(|&&(x, y)| y > 2 && !solid(world, x, y - 1) && !in_shop(world, x, y))
        {
            spawn(world, "oGoldDoor", x, y - 1);
        }
    }
    if matches!(world.area, Area::Mines | Area::Temple) {
        arrows(world, rng, &supports);
    }
    if world.area == Area::Jungle {
        for y in 1..world.height as i32 - 1 {
            for x in 1..world.width as i32 - 1 {
                let object = world.tile(x, y).map(|t| t.object.as_str()).unwrap_or("");
                if object == "oVine" && rng.chance(1, 15) {
                    spawn(world, "oMonkey", x, y);
                }
                if water.wet(x, y) && !solid(world, x, y) && rng.chance(1, 30) {
                    spawn(world, if cemetery { "oDeadFish" } else { "oPiranha" }, x, y);
                }
            }
        }
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
        || x % 10 == 0
        || x % 10 == 1
    {
        return;
    }
    if world.area == Area::Temple && solid(world, x - 1, y - 2) && solid(world, x + 1, y - 2) {
        return;
    }
    if world.dark && world.area == Area::Jungle && !water.wet(x, y - 2) && rng.chance(1, 20) {
        spawn(world, "oTikiTorch", x, y - 2);
    } else if rng.chance(1, 12) {
        spawn(world, "oSpearTrapBottom", x, y);
        if let Some(e) = world.entities.last_mut() {
            e.solid = true;
        }
        spawn(
            world,
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
fn arrows(world: &mut World, rng: &mut Rng, supports: &[(i32, i32)]) {
    for &(x, y) in supports {
        if world.tile(x, y).is_none_or(|t| t.object != "oBlock") || in_shop(world, x, y) {
            continue;
        }
        let pos = at(x, y);
        let distance = pos.distance(world.entrance);
        if distance <= 48.0
            || ((pos.y - world.entrance.y).abs() < 8.0 && distance < 144.0)
            || !rng.chance(1, if world.area == Area::Temple { 3 } else { 4 })
        {
            continue;
        }
        let left = solid(world, x + 1, y) && !solid(world, x - 1, y) && !solid(world, x - 2, y);
        let right = solid(world, x - 1, y)
            && !solid(world, x + 1, y)
            && !solid(world, x + 2, y)
            && !solid(world, x + 3, y);
        let name = match (left, right, world.dark) {
            (true, _, false) => "oArrowTrapLeft",
            (true, _, true) => "oArrowTrapLeftLit",
            (_, true, false) => "oArrowTrapRight",
            (_, true, true) => "oArrowTrapRightLit",
            _ => continue,
        };
        spawn(world, name, x, y);
        if let Some(e) = world.entities.last_mut() {
            e.solid = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn source_instance_offsets_are_preserved() {
        let mut world = room_fixture(Area::Mines, 1);
        spawn(&mut world, "oBat", 5, 6);
        spawn(&mut world, "oGoldBar", 5, 6);
        spawn(&mut world, "oLockedChest", 5, 6);
        assert_eq!(world.entities[0].pos, Vec2::new(80.0, 96.0));
        assert_eq!(world.entities[1].pos, Vec2::new(88.0, 108.0));
        assert_eq!(world.entities[2].pos, Vec2::new(88.0, 104.0));
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
