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
fn spawn(world: &mut World, rng: &mut Rng, name: &str, x: i32, y: i32) {
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
        Vec2::new(x as f64 * 16.0 + offset.x, y as f64 * 16.0 + offset.y),
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
fn bones(world: &mut World, rng: &mut Rng, x: i32, y: i32) {
    if rng.chance(1, 8) {
        spawn(world, rng, "oFakeBones", x, y);
    } else {
        spawn(world, rng, "oBones", x, y);
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

fn source_point_family(world: &World, point: Vec2, family: &str) -> bool {
    world.tiles_point(point, family)
        || world.entities.iter().any(|entity| {
            entity.alive
                && crate::catalog::is_a(&entity.object, family)
                && entity.sprite_bounds().contains(point)
        })
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
    {
        return;
    }
    // scrTreasureGen.gml:34–57 probes named source families at distinct
    // coordinates. Enemies do not block loot; oTreasure blocks the alcove /
    // tunnel branches but deliberately does not block the normal fallback.
    let origin = Vec2::new(f64::from(x) * 16.0, f64::from(y) * 16.0);
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
            spawn(world, rng, "oRock", x, y - 1);
            return;
        }
        if rng.chance(1, 40) {
            spawn(world, rng, "oJar", x, y - 1);
            return;
        }
    }
    let left = Vec2::new(origin.x - 16.0, origin.y - 16.0);
    let right = Vec2::new(origin.x + 16.0, origin.y - 16.0);
    let side = [left, right].iter().any(|point| {
        source_point_family(world, *point, "oSolid") || source_point_family(world, *point, "oBlock")
    });
    let alcove = !col_stuff
        && source_point_family(world, Vec2::new(origin.x, origin.y - 32.0), "oSolid")
        && side;
    let tunnel = !col_stuff
        && source_point_family(world, left, "oSolid")
        && source_point_family(world, right, "oSolid");
    let bonus = if state.cemetery { 10 } else { 0 };
    let level = world.level;
    if alcove {
        // Preserve scrTreasureGen:54's literal typo, not intended proximity:
        // GiantSpider's compiled object index is 749, so (749 < 100) is 0.
        // distance_to_object(0) has no target and returns 1,000,000 (truthy).
        if world.area != Area::Ice && rng.chance(1, 5) {
            spawn(world, rng, "oWeb", x, y - 1);
        } else if state.udjat && !exists(world, "oLockedChest") {
            if rng.chance(1, state.chest_chance) {
                spawn(world, rng, "oLockedChest", x, y - 1);
            } else {
                state.chest_chance = state.chest_chance.saturating_sub(1).max(1);
            }
        } else if rng.chance(1, 10) {
            spawn(world, rng, "oCrate", x, y - 1);
        } else if rng.chance(1, 15) {
            spawn(world, rng, "oChest", x, y - 1);
        } else if !state.damsel && rng.chance(1, 8) && !water.wet(x, y - 1) {
            spawn(world, rng, "oDamsel", x, y - 1);
            state.damsel = true;
        } else if rng.chance(1 + bonus, 40_u32.saturating_sub(2 * level).max(1)) {
            bones(world, rng, x, y - 1);
        } else if rng.chance(1, 3) {
            spawn(world, rng, "oGoldBar", x, y - 1);
        } else if rng.chance(1, 6) {
            spawn(world, rng, "oGoldBars", x, y - 1);
        } else if rng.chance(1, 6) {
            spawn(world, rng, "oEmeraldBig", x, y - 1);
        } else if rng.chance(1, 8) {
            spawn(world, rng, "oSapphireBig", x, y - 1);
        } else if rng.chance(1, 10) {
            spawn(world, rng, "oRubyBig", x, y - 1);
        }
    } else if tunnel {
        // The same literal expression at scrTreasureGen:100 assigns n = 10.
        if world.area != Area::Ice && rng.chance(1, 10) {
            spawn(world, rng, "oWeb", x, y - 1);
        } else if rng.chance(1, 4) {
            spawn(world, rng, "oGoldBar", x, y - 1);
        } else if rng.chance(1, 8) {
            spawn(world, rng, "oGoldBars", x, y - 1);
        } else if rng.chance(1 + bonus, 80_u32.saturating_sub(level).max(1)) {
            bones(world, rng, x, y - 1);
        } else if rng.chance(1, 8) {
            spawn(world, rng, "oEmeraldBig", x, y - 1);
        } else if rng.chance(1, 9) {
            spawn(world, rng, "oSapphireBig", x, y - 1);
        } else if rng.chance(1, 10) {
            spawn(world, rng, "oRubyBig", x, y - 1);
        }
    } else if clear_normal {
        if rng.chance(1, 40) {
            spawn(world, rng, "oGoldBar", x, y - 1);
        } else if rng.chance(1, 50) {
            spawn(world, rng, "oGoldBars", x, y - 1);
        } else if rng.chance(1 + bonus, 140_u32.saturating_sub(2 * level).max(1)) {
            bones(world, rng, x, y - 1);
        }
    }
}

fn schedule(rng: &mut Rng, level: u32, first: u32, last: u32) -> bool {
    level >= first && level <= last && (level == last || rng.chance(1, last - level + 1))
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
}

#[cfg(test)]
fn trace_population_phase(world: &World, x: i32, y: i32, phase: PopulationPhase) {
    POPULATION_PHASE_TRACE.with(|trace| {
        if let Some(trace) = trace.borrow_mut().as_mut() {
            trace.push((world.level, x, y, phase));
        }
    });
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
                    if giant_spider
                        && !exists(world, "oGiantSpiderHang")
                        && !solid(world, x + 1, y + 1)
                        && !solid(world, x + 1, y + 2)
                        && rng.chance(1, 40)
                    {
                        spawn(world, rng, "oGiantSpiderHang", x, y + 1);
                    } else if world.dark && rng.chance(1, 60) {
                        spawn(world, rng, "oLamp", x, y + 1);
                    } else if world.dark && rng.chance(1, 40) {
                        spawn(world, rng, "oScarab", x, y + 1);
                    } else if rng.chance(1, 60) {
                        spawn(world, rng, "oBat", x, y + 1);
                    } else if rng.chance(1, 80) {
                        spawn(world, rng, "oSpiderHang", x, y + 1);
                    }
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
                        if world.dark && rng.chance(1, 40) {
                            spawn(world, rng, "oScarab", x, y + 1);
                        }
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
                    if tile == "oDark"
                        && !solid(world, x, y - 2)
                        && !solid(world, x, y - 3)
                        && !solid(world, x, y - 4)
                        && at(x, y).distance(world.exit) > 64.0
                        && rng.chance(1, 10)
                    {
                        spawn(world, rng, "oSpringTrap", x, y - 1);
                    } else if rng.chance(1, 20) {
                        spawn(world, rng, "oYeti", x, y - 1);
                    }
                }
            }
            Area::Temple => {
                if world.level == 14 && y > 2 && floor && !exists(world, "oGoldDoor") {
                    if rng.chance(1, gold_chance) {
                        spawn(world, rng, "oGoldDoor", x, y - 1);
                    } else {
                        gold_chance = gold_chance.saturating_sub(1).max(1);
                    }
                } else {
                    spear(world, rng, &water, x, y, altar || tree, far);
                }
                if ceiling && world.dark && rng.chance(1, 40) {
                    spawn(world, rng, "oScarab", x, y + 1);
                }
                if !start_room(world, x, y - 1) && floor && y > 1 && !occupied(world, x, y - 1) {
                    if tomb_lord
                        && !exists(world, "oTombLord")
                        && !solid(world, x, y - 2)
                        && !solid(world, x + 1, y - 2)
                        && !solid(world, x + 1, y - 1)
                        && rng.chance(1, 40)
                    {
                        spawn(world, rng, "oTombLord", x, y - 2);
                    } else if rng.chance(1, 40) {
                        spawn(world, rng, "oCaveman", x, y - 1);
                    } else if rng.chance(1, 40) {
                        spawn(world, rng, "oHawkman", x, y - 1);
                    } else if rng.chance(1, 60) {
                        spawn(
                            world,
                            rng,
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
        if matches!(world.area, Area::Ice | Area::Temple) && !altar {
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
    if world.area == Area::Temple && world.level == 14 && !exists(world, "oGoldDoor") {
        if let Some(&(x, y)) = supports
            .iter()
            .find(|&&(x, y)| y > 2 && !solid(world, x, y - 1) && !in_shop(world, x, y))
        {
            spawn(world, rng, "oGoldDoor", x, y - 1);
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
                    spawn(world, rng, "oMonkey", x, y);
                }
                if water.wet(x, y) && !solid(world, x, y) && rng.chance(1, 30) {
                    spawn(
                        world,
                        rng,
                        if cemetery { "oDeadFish" } else { "oPiranha" },
                        x,
                        y,
                    );
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
        spawn(world, rng, "oTikiTorch", x, y - 2);
    } else if rng.chance(1, 12) {
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
        spawn(world, rng, name, x, y);
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

    fn treasure_state() -> TreasureState {
        TreasureState {
            udjat: false,
            chest_chance: 8,
            damsel: false,
            cemetery: false,
        }
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
        if area != Area::Ice && 1 + rng.range(n) == 1 {
            return vec!["oWeb"];
        }
        let level = if area == Area::Ice { 9 } else { 1 };
        let rolls: &[(u32, &str)] = if alcove {
            &[
                (10, "oCrate"),
                (15, "oChest"),
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
            if 1 + rng.range(limit) == 1 {
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
                    assert_eq!(webs, 0, "Ice skips the web random statement");
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
        let mut actual = Rng::new(1297);
        let mut source = actual.clone();
        assert_eq!(source.range(30), 0); // UFO chance succeeds.
        let shift = source.range(2); // Synchronous UFO Create.
        assert_eq!(source.range(100), 0); // Following treasure Rock succeeds.
        populate(&mut world, &mut actual, &BTreeSet::new());
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
        // cannot make a spear or ceiling enemy: TombLord rand(1,40), then
        // scrTreasureGen's Rock rand(1,100) are the complete source sequence.
        let mut world = World::new(42, 34, Area::Temple, 13);
        world.entrance = at(1, 1);
        world.exit = at(37, 30);
        world.set_tile(10, 32, "oTemple");
        let mut actual = Rng::new(6491);
        let mut source = actual.clone();
        assert_eq!(source.range(40), 0);
        assert_eq!(source.range(100), 0);
        populate(&mut world, &mut actual, &BTreeSet::new());
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
        let mut actual = Rng::new(6491);
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
        for seed in [0, 1, 23, u64::MAX] {
            let mut world = World::new(12, 12, Area::Jungle, 5);
            world.set_tile(4, 4, "oWater");
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
