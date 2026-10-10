//! Original terrain Create-event treasure rolls and scrSetupWalls silhouettes.
//!
//! Tile instances are represented by invisible, centered metadata entities.
//! Their contents become available when the containing terrain is destroyed.
use crate::model::{Rng, Vec2, World};

const UNDERGROUND: [(&str, f64, f64); 19] = [
    ("oJetpack", 0.0, -2.0),
    ("oCapePickup", 0.0, 0.0),
    ("oShotgun", 0.0, 0.0),
    ("oMattock", 0.0, 0.0),
    ("oTeleporter", 0.0, 3.0),
    ("oGloves", 0.0, -1.0),
    ("oSpectacles", 0.0, 0.0),
    ("oWebCannon", -2.0, 0.0),
    ("oPistol", 0.0, 0.0),
    ("oMitt", 0.0, -1.0),
    ("oPaste", 0.0, 0.0),
    ("oSpringShoes", 0.0, 0.0),
    ("oSpikeShoes", 0.0, 0.0),
    ("oMachete", 0.0, 0.0),
    ("oBombBox", 0.0, -2.0),
    ("oBow", 0.0, 0.0),
    ("oCompass", 0.0, 0.0),
    ("oParaPickup", 0.0, 0.0),
    ("oRopePile", 0.0, 0.0),
];

fn metadata(world: &mut World, x: usize, y: usize) -> u64 {
    let p = Vec2::new(x as f64 * 16.0 + 8.0, y as f64 * 16.0 + 8.0);
    if let Some(e) = world
        .entities
        .iter()
        .find(|e| e.alive && e.object == "oTerrainMetadata" && e.pos == p)
    {
        return e.id;
    }
    let id = world.spawn("oTerrainMetadata", p);
    world.entity_mut(id).unwrap().sprite.clear();
    id
}

fn neighbors(world: &World, x: usize, y: usize, object: &str) -> (bool, bool, bool, bool) {
    let connected = |dx: i32, dy: i32| {
        world.tile(x as i32 + dx, y as i32 + dy).is_some_and(|t| {
            t.object == object
                || (object == "oBrick" && t.object == "oHardBlock")
                || (object == "oTemple" && t.object == "oTempleFake")
        })
    };
    let boundary = object != "oIce";
    let bottom = if world.area == crate::model::Area::Boss {
        864
    } else if world.name == "Lake" {
        656
    } else {
        528
    };
    let mut up = (boundary && y == 0) || connected(0, -1);
    if object == "oTemple" {
        // The original oTemple up check deliberately queries Fake *below*.
        up = (boundary && y == 0)
            || world
                .tile(x as i32, y as i32 - 1)
                .is_some_and(|t| t.object == "oTemple")
            || world
                .tile(x as i32, y as i32 + 1)
                .is_some_and(|t| t.object == "oTempleFake");
    }
    (
        up,
        (boundary && y * 16 >= bottom) || connected(0, 1),
        connected(-1, 0),
        connected(1, 0),
    )
}

fn edge_sprite(object: &str, city: bool, n: (bool, bool, bool, bool)) -> Option<String> {
    let (up, down, left, right) = n;
    let name = match object {
        "oBrick" => match (up, down) {
            (false, false) => "sCaveUp2",
            (false, true) => "sCaveUp",
            (true, false) => "sBrickDown",
            _ => return None,
        },
        "oLush" => match (up, down) {
            (false, false) => "sLushUp2",
            (false, true) => "sLushUp",
            (true, false) => "sLushDown",
            _ => return None,
        },
        "oDark" => match (up, down) {
            (false, false) => "sDarkUp2",
            (false, true) => "sDarkUp",
            (true, false) => "sDarkDown",
            _ => return None,
        },
        "oTemple" => {
            let suffix = if !up {
                match (left, right, down) {
                    (false, false, false) => "Up6",
                    (false, false, true) => "Up5",
                    (false, true, false) => "Up7",
                    (false, true, true) => "Up3",
                    (true, false, false) => "Up8",
                    (true, false, true) => "Up4",
                    (true, true, false) => "Up2",
                    (true, true, true) => "Up",
                }
            } else if !down {
                "Down"
            } else {
                return None;
            };
            return Some(format!(
                "{}{}",
                if city { "sGTemple" } else { "sTemple" },
                suffix
            ));
        }
        "oIce" => match (up, down, left, right) {
            (false, false, false, false) => "sIceBlock",
            (false, true, false, false) => "sIceULR",
            (true, false, false, false) => "sIceDLR",
            (true, true, false, false) => "sIceLR",
            (false, false, _, false) => "sIceUDR",
            (false, true, _, false) => "sIceUR",
            (true, false, _, false) => "sIceDR",
            (true, true, _, false) => "sIceRight",
            (false, false, false, true) => "sIceUDL",
            (false, true, false, true) => "sIceUL",
            (true, false, false, true) => "sIceDL",
            (true, true, false, true) => "sIceLeft",
            (false, false, true, true) => "sIceUp2",
            (false, true, true, true) => "sIceUp",
            (true, false, true, true) => "sIceDown",
            _ => return None,
        },
        _ => return None,
    };
    Some(name.into())
}

/// Creation order is stable within this Rust port; it does not reproduce the
/// GameMaker interleaving of room creation and individual Create events.
pub(super) fn initialize(world: &mut World, rng: &mut Rng) {
    let city = world.name == "City of Gold";
    for y in 0..world.height {
        for x in 0..world.width {
            let object = world.tiles[y * world.width + x].object.clone();
            if !matches!(
                object.as_str(),
                "oBrick" | "oLush" | "oDark" | "oTemple" | "oIce"
            ) {
                continue;
            }
            let id = metadata(world, x, y);
            if world.entity(id).unwrap().value("terrain_initialized") != 0.0 {
                continue;
            }
            let explicit_sprite = world
                .entity(id)
                .unwrap()
                .text
                .get("terrain_sprite")
                .cloned();
            let source_boundary = world.entity(id).unwrap().value("source_boundary") != 0.0;
            let mut sprite = if object == "oBrick" && !source_boundary && rng.chance(1, 10) {
                "sBrick2".into()
            } else {
                format!("s{}", &object[1..])
            };
            let mut gold = 0;
            let mut treasure = None;
            let mut frozen = false;
            if source_boundary {
                // scrInitLevel explicitly replaces boundary Create-event
                // variants with the area's base sprite. Preserve the port's
                // established interior RNG stream while applying SetupWalls
                // silhouettes below; exact GameMaker RNG remains unaudited.
                if object == "oTemple" && city {
                    sprite = "sGTemple".into();
                }
            } else if object == "oIce" {
                if rng.chance(1, 80) {
                    // FrozenCaveman Destroy releases this live caveman.
                    treasure = Some(("oCaveman", -8.0, -8.0));
                    frozen = true;
                }
            } else {
                let roll = rng.range(100) + 1;
                if object == "oTemple" && city {
                    sprite = "sGTemple".into();
                    gold = 2;
                } else if roll < 20 {
                    sprite = format!("s{}Gold", &object[1..]);
                    gold = 1;
                } else if roll < 30 {
                    sprite = format!("s{}GoldBig", &object[1..]);
                    gold = 2;
                } else if x > 0 && y > 0 && x + 1 < world.width && y + 1 < world.height {
                    let chances = match object.as_str() {
                        "oBrick" => [100, 120, 140],
                        "oLush" => [80, 100, 120],
                        "oDark" => [40, 60, 80],
                        _ => [60, 80, 100],
                    };
                    for (name, chance) in ["oSapphireBig", "oEmeraldBig", "oRubyBig"]
                        .into_iter()
                        .zip(chances)
                    {
                        if rng.chance(1, chance) {
                            treasure = Some((name, 0.0, 0.0));
                            break;
                        }
                    }
                    if treasure.is_none() && rng.chance(1, 1200) {
                        treasure = Some(UNDERGROUND[rng.range(19) as usize]);
                    }
                }
            }
            let sides = neighbors(world, x, y, &object);
            // SetupWalls overwrites gold sprites; their Destroy event then
            // releases no gold. City of Gold uses the global flag instead.
            if let Some(edge) = edge_sprite(&object, city, sides) {
                sprite = edge;
                if !(object == "oTemple" && city) {
                    gold = 0;
                }
            }
            if let Some(explicit) = explicit_sprite {
                sprite = explicit;
                if !(object == "oTemple" && city) {
                    gold = 0;
                }
            }
            let e = world.entity_mut(id).unwrap();
            e.sprite.clear();
            e.data.insert("terrain_initialized".into(), 1.0);
            e.text.insert("terrain_sprite".into(), sprite);
            if gold > 0 {
                e.data.insert("gold_chunks".into(), 3.0);
                e.data
                    .insert("gold_nuggets".into(), if gold == 2 { 1.0 } else { 0.0 });
            }
            if frozen {
                e.data.insert("frozen_caveman".into(), 1.0);
            }
            // Explicit temple Big Ruby assignments from room templates win.
            if !e.text.contains_key("treasure") {
                if let Some((name, dx, dy)) = treasure {
                    e.data.insert("embedded_treasure".into(), 1.0);
                    e.data.insert("treasure_dx".into(), dx);
                    e.data.insert("treasure_dy".into(), dy);
                    e.text.insert("treasure".into(), name.into());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Area;

    #[test]
    fn ice_and_temple_silhouettes_match_source_precedence() {
        assert_eq!(
            edge_sprite("oIce", false, (false, false, false, false)).as_deref(),
            Some("sIceBlock")
        );
        assert_eq!(
            edge_sprite("oIce", false, (false, false, true, false)).as_deref(),
            Some("sIceUDR")
        );
        assert_eq!(
            edge_sprite("oTemple", true, (false, false, false, true)).as_deref(),
            Some("sGTempleUp7")
        );
        assert_eq!(
            edge_sprite("oBrick", false, (true, true, false, false)),
            None
        );
    }

    #[test]
    fn city_edges_keep_gold_and_existing_treasure_metadata() {
        let mut w = World::new(3, 3, Area::Temple, 15);
        w.name = "City of Gold".into();
        w.set_tile(1, 1, "oTemple");
        let id = metadata(&mut w, 1, 1);
        w.entity_mut(id)
            .unwrap()
            .text
            .insert("treasure".into(), "oRubyBig".into());
        initialize(&mut w, &mut Rng::new(0));
        let e = w.entity(id).unwrap();
        assert_eq!(e.value("gold_chunks"), 3.0);
        assert_eq!(e.value("gold_nuggets"), 1.0);
        assert_eq!(e.text["treasure"], "oRubyBig");
        assert_eq!(e.text["terrain_sprite"], "sGTempleUp6");
        assert_eq!(w.entities.len(), 1);
        let previous = serde_json::to_string(&w).unwrap();
        initialize(&mut w, &mut Rng::new(100));
        assert_eq!(serde_json::to_string(&w).unwrap(), previous);
    }

    #[test]
    fn covered_bricks_roll_both_gold_classes_but_edges_do_not() {
        let mut classes = std::collections::BTreeSet::new();
        for seed in 0..128 {
            let mut w = World::new(3, 3, Area::Mines, 1);
            for tile in &mut w.tiles {
                tile.object = "oBrick".into();
            }
            initialize(&mut w, &mut Rng::new(seed));
            let middle = w
                .entities
                .iter()
                .find(|e| e.pos == Vec2::new(24.0, 24.0))
                .unwrap();
            if middle.value("gold_chunks") == 3.0 {
                classes.insert(middle.value("gold_nuggets") as u8);
            }
        }
        assert_eq!(classes, [0, 1].into_iter().collect());
        let mut w = World::new(3, 3, Area::Mines, 1);
        w.set_tile(1, 1, "oBrick");
        initialize(&mut w, &mut Rng::new(4));
        assert_eq!(w.entities[0].value("gold_chunks"), 0.0);
    }
}
