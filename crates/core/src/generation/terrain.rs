//! Original terrain Create-event treasure rolls and scrSetupWalls silhouettes.
//!
//! Tile instances are represented by invisible, centered metadata entities.
//! Custom destructible terrain Create children are live during loading;
//! generated legacy descriptors retain deferred adapters where the source does.
use crate::model::{Entity, Rng, Vec2, World};

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

pub(super) fn metadata(world: &mut World, x: usize, y: usize) -> u64 {
    let p = Vec2::new(x as f64 * 16.0 + 8.0, y as f64 * 16.0 + 8.0);
    if let Some(id) = world
        .entities
        .iter()
        .find(|e| {
            e.alive
                && e.object == "oTerrainMetadata"
                && (e.pos.x / 16.0).floor() as usize == x
                && (e.pos.y / 16.0).floor() as usize == y
        })
        .map(|entity| entity.id)
    {
        let entity = world.entity_mut(id).unwrap();
        entity.pos = p;
        entity.previous_pos = p;
        return id;
    }
    let id = world.spawn("oTerrainMetadata", p);
    world.entity_mut(id).unwrap().sprite.clear();
    id
}

/// Apply one of rOlmec's authored Temple Create outcomes at its source birth
/// phase. Its random result was captured before oGame.Create, so this applies
/// the recorded sprite/content without drawing again.
pub(super) fn create_authored_boss_temple(
    world: &mut World,
    rng: &mut Rng,
    x: usize,
    y: usize,
    create: super::special::BossTempleCreate,
    city_of_gold: bool,
    has_spectacles: bool,
) {
    world.set_tile(x, y, "oTemple");
    let id = metadata(world, x, y);
    let (sprite, gold) = if city_of_gold {
        ("sGTemple", 2)
    } else if create.roll < 20 {
        ("sTempleGold", 1)
    } else if create.roll < 30 {
        ("sTempleGoldBig", 2)
    } else {
        ("sTemple", 0)
    };
    let entity = world.entity_mut(id).unwrap();
    entity.data.insert("terrain_initialized".into(), 1.0);
    entity.data.insert("n".into(), f64::from(create.roll));
    entity.text.insert("terrain_sprite".into(), sprite.into());
    if gold > 0 {
        entity.data.insert("gold_chunks".into(), 3.0);
        entity
            .data
            .insert("gold_nuggets".into(), f64::from(gold == 2));
    }

    let Some(treasure) = create.treasure else {
        return;
    };
    let center = Vec2::new(x as f64 * 16.0 + 8.0, y as f64 * 16.0 + 8.0);
    let (object, dx, dy, underground_item) = match treasure {
        super::special::BossTempleTreasure::SapphireBig => ("oSapphireBig", 0.0, 0.0, false),
        super::special::BossTempleTreasure::EmeraldBig => ("oEmeraldBig", 0.0, 0.0, false),
        super::special::BossTempleTreasure::RubyBig => ("oRubyBig", 0.0, 0.0, false),
        super::special::BossTempleTreasure::Underground(index) => {
            let (object, dx, dy) = UNDERGROUND[usize::from(index)];
            (object, dx, dy, true)
        }
    };
    let child = crate::creation::spawn_generated_terrain_child(
        world,
        rng,
        object,
        Vec2::new(center.x + dx, center.y + dy),
        has_spectacles,
    );
    if underground_item {
        let child = world.entity_mut(child).unwrap();
        // scrGenerateItem(...,2) applies these producer overrides after Create.
        child.data.insert("cost".into(), 0.0);
        child.data.insert("forSale".into(), 0.0);
    }
}

/// Complete the closed scrInitLevel boundary constructor at its actual birth.
/// Tile storage remains the port's ordinary Solid representation; exterior
/// source instances remain concrete entities, without changing room bounds.
/// Interior terrain Create/stamp interleaving is a separate source-order debt.
pub(super) fn create_source_boundary(
    world: &mut World,
    rng: &mut Rng,
    x: usize,
    y: usize,
    object: &str,
    city: bool,
    invincible_override: bool,
) {
    let origin = Vec2::new(x as f64 * 16.0, y as f64 * 16.0);
    // The source condition is strict and uses room dimensions, not storage
    // existence. Every boundary and y=640 floor origin fails it. Refuse to
    // use this closed constructor for an interior parent that can birth loot.
    assert!(
        !(origin.x > 1.0
            && origin.x < (world.width - 1) as f64 * 16.0
            && origin.y > 1.0
            && origin.y < (world.height - 1) as f64 * 16.0)
    );
    let stored = x < world.width && y < world.height;
    let id = if stored {
        assert!(world.tile(x as i32, y as i32).unwrap().object.is_empty());
        world.set_tile(x, y, object);
        metadata(world, x, y)
    } else {
        world.spawn(object, origin)
    };
    let entity = world.entity_mut(id).unwrap();
    // oDrawnSprite Create, then oSolid Create, before own Create.
    entity.data.insert("blinkToggle".into(), 0.0);
    entity.data.insert("invincible".into(), 0.0);
    entity.data.insert("shopWall".into(), 0.0);
    entity.data.insert("cleanDeath".into(), 0.0);
    entity.text.insert("type".into(), "None".into());
    entity.text.insert("treasure".into(), String::new());
    #[cfg(test)]
    let rng_before = rng.state_bits();
    let mut sprite = format!("s{}", &object[1..]);
    if object == "oBrick" && rng.range(10) == 0 {
        sprite = "sBrick2".into();
    }
    // Even City of Gold Temple Create draws before selecting sGTemple.
    let roll = 1 + rng.range(100);
    if object == "oTemple" && city {
        sprite = "sGTemple".into();
    } else if roll < 20 {
        sprite = format!("s{}Gold", &object[1..]);
    } else if roll < 30 {
        sprite = format!("s{}GoldBig", &object[1..]);
    }
    #[cfg(test)]
    super::source_boundary_tests::observe_create(
        id,
        origin,
        object,
        &sprite,
        rng_before,
        rng.state_bits(),
    );
    // Caller assignments occur after own Create. The ordinary Ice floor has
    // neither assignment and retains its real constructor variant.
    if invincible_override && !(object == "oTemple" && city) {
        sprite = format!("s{}", &object[1..]);
    }
    let entity = world.entity_mut(id).unwrap();
    entity.data.insert("n".into(), f64::from(roll));
    entity.data.insert("terrain_initialized".into(), 1.0);
    entity.data.insert("source_boundary_birth".into(), 1.0);
    if invincible_override {
        entity.data.insert("invincible".into(), 1.0);
        entity.data.insert("source_boundary".into(), 1.0);
    }
    // Destroy reads the final sprite, not the overwritten Create variant.
    entity.data.remove("gold_chunks");
    entity.data.remove("gold_nuggets");
    if sprite.ends_with("Gold") || sprite.ends_with("GoldBig") || sprite == "sGTemple" {
        entity.data.insert("gold_chunks".into(), 3.0);
        entity.data.insert(
            "gold_nuggets".into(),
            if sprite.ends_with("GoldBig") || sprite == "sGTemple" {
                1.0
            } else {
                0.0
            },
        );
    }
    if stored {
        entity.sprite.clear();
        entity.text.insert("terrain_sprite".into(), sprite);
    } else {
        entity.sprite = sprite;
    }
}

/// oLava owns Create (no inherited action): its rand must finish before
/// the room-script caller proceeds to the next cell/condition/selector.
/// Fresh SourceClassic room births have no caller overrides at this point.
pub(super) fn create_source_room_lava(world: &mut World, rng: &mut Rng, id: u64) {
    let parent = world.entity_mut(id).expect("fresh Lava room birth");
    assert_eq!(parent.object, "oLava");
    assert_eq!(parent.value("lava_initialized"), 0.);
    parent.text.insert("type".into(), "Lava".into());
    parent.data.insert("spurt".into(), 0.);
    // rand.gml's current-receiver local write precedes random()/return.
    parent.data.insert("randNum".into(), 201.);
    let time = f64::from(100 + rng.range(201));
    parent.data.insert("spurt_time".into(), time);
    parent.data.insert("spurt_counter".into(), time);
    // Separate following DND variable action in oLava's own Create.
    parent.set_image_speed(0.4);
    parent.data.insert("lava_initialized".into(), 1.);
}

/// Complete the remaining room-script terrain constructors at actual birth.
/// The grid/proxy representation is retained; duplicate overlapping instances
/// and SetupWalls interleaving remain separately stated adapter debt.
pub(super) fn create_source_room_terrain(
    world: &mut World,
    rng: &mut Rng,
    x: usize,
    y: usize,
    context: super::GenerationContext,
) {
    let object = world.tiles[y * world.width + x].object.clone();
    if object == "oLush" {
        create_source_ground_lush(world, rng, x, y, context);
        return;
    }
    assert!(matches!(
        object.as_str(),
        "oBrick" | "oDark" | "oTemple" | "oIce" | "oBlock"
    ));
    for old in &mut world.entities {
        if old.alive
            && old.object == "oTerrainMetadata"
            && (old.pos.x / 16.).floor() as usize == x
            && (old.pos.y / 16.).floor() as usize == y
        {
            old.alive = false;
        }
    }
    let id = metadata(world, x, y);
    let parent = world.entity_mut(id).unwrap();
    for name in ["blinkToggle", "invincible", "shopWall", "cleanDeath"] {
        parent.data.insert(name.into(), 0.);
    }
    parent.text.insert("type".into(), "None".into());
    parent.text.insert("treasure".into(), String::new());
    #[cfg(test)]
    let trace = synchronous_room_terrain_tests::observe_before(world, rng, id);
    #[cfg(test)]
    let contents_trace = create_contents_tests::observe_before(world, rng, x, y, id, false);
    let city = world.name == "City of Gold";
    let in_level = if world.name.starts_with('r') {
        matches!(
            world.name.as_str(),
            "rTutorial" | "rLoadLevel" | "rLevel" | "rLevel2" | "rLevel3" | "rOlmec"
        )
    } else {
        (1..=16).contains(&world.level) || world.name == "Tutorial Cave"
    };
    let interior = in_level && x > 0 && y > 0 && x + 1 < world.width && y + 1 < world.height;
    let mut sprite = format!("s{}", &object[1..]);
    let mut gold = 0;
    let mut selected = None;
    let mut frozen = false;
    if object == "oBlock" {
        if city {
            sprite = "sGoldBlock".into();
        }
    } else if object == "oIce" {
        frozen = rng.range(80) == 0;
    } else {
        if object == "oBrick" && rng.range(10) == 0 {
            sprite = "sBrick2".into();
        }
        let n = 1 + rng.range(100);
        world
            .entity_mut(id)
            .unwrap()
            .data
            .insert("n".into(), f64::from(n));
        if object == "oTemple" && city {
            sprite = "sGTemple".into();
            gold = 2;
        } else if object != "oDark" || world.name != "rIntro" {
            if n < 20 {
                sprite = format!("s{}Gold", &object[1..]);
                gold = 1;
            } else if n < 30 {
                sprite = format!("s{}GoldBig", &object[1..]);
                gold = 2;
            } else if interior {
                let chances = match object.as_str() {
                    "oBrick" => [100, 120, 140],
                    "oDark" => [40, 60, 80],
                    "oTemple" => [60, 80, 100],
                    _ => unreachable!(),
                };
                for (name, width) in ["oSapphireBig", "oEmeraldBig", "oRubyBig"]
                    .into_iter()
                    .zip(chances)
                {
                    if rng.range(width) == 0 {
                        selected = Some((name, 0., 0.));
                        break;
                    }
                }
                if selected.is_none() && rng.range(1200) == 0 {
                    selected = Some(UNDERGROUND[rng.range(19) as usize]);
                }
            }
        }
    }
    let parent = world.entity_mut(id).unwrap();
    parent.data.insert("terrain_initialized".into(), 1.);
    parent.data.insert("source_room_terrain_created".into(), 1.);
    parent.text.insert("terrain_sprite".into(), sprite);
    if gold != 0 {
        parent.data.insert("gold_chunks".into(), 3.);
        parent
            .data
            .insert("gold_nuggets".into(), if gold == 2 { 1. } else { 0. });
    }
    if let Some((object, dx, dy)) = selected {
        let child = crate::creation::spawn_generated_terrain_child(
            world,
            rng,
            object,
            Vec2::new(x as f64 * 16. + 8. + dx, y as f64 * 16. + 8. + dy),
            context.has_spectacles,
        );
        if crate::catalog::is_a(object, "oItem") {
            let child = world.entity_mut(child).unwrap();
            child.data.insert("cost".into(), 0.);
            child.data.insert("forSale".into(), 0.);
        }
    }
    if frozen {
        // This is the source's real independent child. Do not record the old
        // virtual frozen_caveman/embedded_treasure debt on parent metadata.
        crate::creation::spawn_generated_entity(
            world,
            rng,
            "oFrozenCaveman",
            Vec2::new(x as f64 * 16., y as f64 * 16.),
        );
    }
    #[cfg(test)]
    create_contents_tests::observe_after(world, rng, contents_trace);
    #[cfg(test)]
    synchronous_room_terrain_tests::observe_after(world, rng, trace);
}

/// Complete every SourceClassic room-script Lush birth, including its real
/// child constructor, before later predicates, births, or caller overrides.
/// Shared with the complete represented room-terrain constructor above.
pub(super) fn create_source_ground_lush(
    world: &mut World,
    rng: &mut Rng,
    x: usize,
    y: usize,
    context: super::GenerationContext,
) {
    assert_eq!(world.tiles[y * world.width + x].object, "oLush");
    // set_tile has just created/replaced the represented parent. A stale
    // metadata proxy must not carry old initialization/sprite/loot fields into
    // its new Create. This changes no lifetime of its already-live children.
    for old in &mut world.entities {
        if old.alive
            && old.object == "oTerrainMetadata"
            && (old.pos.x / 16.).floor() as usize == x
            && (old.pos.y / 16.).floor() as usize == y
        {
            old.alive = false;
        }
    }
    let id = metadata(world, x, y);
    let parent = world.entity_mut(id).unwrap();
    parent.data.insert("blinkToggle".into(), 0.);
    parent.data.insert("invincible".into(), 0.);
    parent.data.insert("shopWall".into(), 0.);
    parent.data.insert("cleanDeath".into(), 0.);
    parent.text.insert("type".into(), "None".into());
    parent.text.insert("treasure".into(), String::new());
    #[cfg(test)]
    let trace = create_contents_tests::observe_before(world, rng, x, y, id, false);
    let n = 1 + rng.range(100);
    let mut sprite = "sLush";
    let mut gold = 0;
    let mut selected = None;
    if n < 20 {
        sprite = "sLushGold";
        gold = 1;
    } else if n < 30 {
        sprite = "sLushGoldBig";
        gold = 2;
    } else if x > 0 && x + 1 < world.width && y > 0 && y + 1 < world.height {
        if rng.range(80) == 0 {
            selected = Some(("oSapphireBig", 0., 0.));
        } else if rng.range(100) == 0 {
            selected = Some(("oEmeraldBig", 0., 0.));
        } else if rng.range(120) == 0 {
            selected = Some(("oRubyBig", 0., 0.));
        } else if rng.range(1200) == 0 {
            selected = Some(UNDERGROUND[rng.range(19) as usize]);
        }
    }
    let parent = world.entity_mut(id).unwrap();
    parent.data.insert("n".into(), f64::from(n));
    parent.data.insert("terrain_initialized".into(), 1.);
    parent.data.insert("source_ground_lush_created".into(), 1.);
    parent.text.insert("terrain_sprite".into(), sprite.into());
    if gold != 0 {
        parent.data.insert("gold_chunks".into(), 3.);
        parent
            .data
            .insert("gold_nuggets".into(), if gold == 2 { 1. } else { 0. });
    }
    if let Some((object, dx, dy)) = selected {
        let child = crate::creation::spawn_generated_terrain_child(
            world,
            rng,
            object,
            Vec2::new(x as f64 * 16. + 8. + dx, y as f64 * 16. + 8. + dy),
            context.has_spectacles,
        );
        if crate::catalog::is_a(object, "oItem") {
            // scrGenerateItem Set2 assigns these only after own Create.
            let child = world.entity_mut(child).unwrap();
            child.data.insert("cost".into(), 0.);
            child.data.insert("forSale".into(), 0.);
        }
    }
    #[cfg(test)]
    create_contents_tests::observe_after(world, rng, trace);
}

fn neighbors(world: &World, x: usize, y: usize, object: &str) -> (bool, bool, bool, bool) {
    let connected = |dx: i32, dy: i32| {
        let point = Vec2::new(
            (x as f64 + f64::from(dx)) * 16.0,
            (y as f64 + f64::from(dy)) * 16.0,
        );
        world.tile(x as i32 + dx, y as i32 + dy).is_some_and(|t| {
            t.object == object
                || (object == "oBrick" && t.object == "oHardBlock")
                || (object == "oTemple" && t.object == "oTempleFake")
        }) || (object == "oTemple"
            && world.entities.iter().any(|entity| {
                entity.alive
                    && entity.object == "oTempleFake"
                    && entity.sprite_bounds().contains(point)
            }))
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
                .is_some_and(|t| t.object == "oTempleFake")
            || world.entities.iter().any(|entity| {
                entity.alive
                    && entity.object == "oTempleFake"
                    && entity
                        .sprite_bounds()
                        .contains(Vec2::new(x as f64 * 16.0, (y + 1) as f64 * 16.0))
            });
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
#[cfg(test)]
pub(super) fn initialize(world: &mut World, rng: &mut Rng) {
    initialize_with_graphics(world, rng, true);
}

pub(super) fn initialize_custom(world: &mut World, rng: &mut Rng) {
    initialize_custom_with_context(world, rng, super::GenerationContext::default());
}

pub(super) fn initialize_custom_with_context(
    world: &mut World,
    rng: &mut Rng,
    context: super::GenerationContext,
) {
    // Classic runs each tile object's Create immediately from
    // scrCreateTileObj. Its RNG work therefore interleaves with neighboring
    // enemy and trap Create events in row-major room order.
    let mut births: Vec<_> = world
        .tiles
        .iter()
        .enumerate()
        .filter(|(_, tile)| {
            matches!(
                tile.object.as_str(),
                "oBrick" | "oLush" | "oDark" | "oTemple" | "oIce" | "oLava"
            )
        })
        .map(|(index, tile)| (tile.creation_serial, CustomCreateBirth::Tile(index)))
        .chain(
            world
                .entities
                .iter()
                .filter(|entity| entity.alive)
                .map(|entity| (entity.creation_serial, CustomCreateBirth::Entity(entity.id))),
        )
        .collect();
    births.sort_by_key(|(serial, _)| *serial);
    let city = world.name == "City of Gold";
    for (_, birth) in births {
        match birth {
            CustomCreateBirth::Tile(index) => {
                let x = index % world.width;
                let y = index / world.width;
                let creation_serial = world.tiles[index].creation_serial;
                let object = world.tiles[index].object.as_str();
                if object == "oLava" {
                    let id = metadata(world, x, y);
                    create_lava(world.entity_mut(id).unwrap(), rng);
                } else {
                    create_terrain_tile(
                        world,
                        rng,
                        x,
                        y,
                        true,
                        context,
                        city,
                        Some(creation_serial),
                    );
                }
            }
            CustomCreateBirth::Entity(id) => {
                let Some(object) = world.entity(id).map(|entity| entity.object.clone()) else {
                    continue;
                };
                if object == "oLava" {
                    if let Some(entity) = world.entity_mut(id) {
                        create_lava(entity, rng);
                    }
                } else if matches!(
                    object.as_str(),
                    "oBrick" | "oLush" | "oDark" | "oTemple" | "oIce"
                ) {
                    initialize_custom_instance(world, rng, id, context);
                } else {
                    crate::creation::initialize_entity_local(world, rng, id);
                    if crate::catalog::is_a(&object, "oItem") {
                        crate::items::initialize_inherited_depth(
                            world.entity_mut(id).unwrap(),
                            context.has_spectacles,
                        );
                    }
                }
            }
        }
    }
}

#[derive(Clone, Copy)]
enum CustomCreateBirth {
    Tile(usize),
    Entity(u64),
}

#[cfg(test)]
pub(super) fn initialize_with_graphics(world: &mut World, rng: &mut Rng, graphics_high: bool) {
    create_generated(world, rng);
    setup_generated(world, rng, graphics_high);
    finalize_generated(world, crate::GenerationRevision::HistoricalV1);
}

/// Generated terrain Create events precede scrEntityGen. This phase establishes
/// the original base/treasure sprites and buried contents without SetupWalls
/// masks, fringes, drips or water surfaces.
#[cfg(test)]
pub(super) fn create_generated(world: &mut World, rng: &mut Rng) {
    create_generated_with_context(world, rng, super::GenerationContext::default());
}

pub(super) fn create_generated_with_context(
    world: &mut World,
    rng: &mut Rng,
    context: super::GenerationContext,
) {
    create_terrain(world, rng, false, context);
}

fn create_terrain(
    world: &mut World,
    rng: &mut Rng,
    custom: bool,
    context: super::GenerationContext,
) {
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
            create_terrain_tile(world, rng, x, y, custom, context, city, None);
        }
    }
    // oLava Create selects its spurt timer when the instance is created, before
    // population. Surface selection and its extra random roll belong later.
    if !custom {
        for y in 0..world.height {
            for x in 0..world.width {
                if world.tile(x as i32, y as i32).unwrap().object == "oLava" {
                    let id = metadata(world, x, y);
                    create_lava(world.entity_mut(id).unwrap(), rng);
                }
            }
        }
        let lava: Vec<_> = world
            .entities
            .iter()
            .filter(|entity| entity.alive && entity.object == "oLava")
            .map(|entity| entity.id)
            .collect();
        for id in lava {
            create_lava(world.entity_mut(id).unwrap(), rng);
        }
    }
}

fn create_terrain_tile(
    world: &mut World,
    rng: &mut Rng,
    x: usize,
    y: usize,
    custom: bool,
    context: super::GenerationContext,
    city: bool,
    custom_creation_serial: Option<u64>,
) {
    let object = world.tiles[y * world.width + x].object.clone();
    if !matches!(
        object.as_str(),
        "oBrick" | "oLush" | "oDark" | "oTemple" | "oIce"
    ) {
        return;
    }
    let id = metadata(world, x, y);
    if world.entity(id).unwrap().value("terrain_initialized") != 0.0 {
        return;
    }
    #[cfg(test)]
    let create_trace = create_contents_tests::observe_before(world, rng, x, y, id, custom);
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
        // variants with the area's base sprite. Boundary Create RNG
        // draws are not reproduced here yet; this phase split does
        // not establish exact GameMaker generation/RNG parity.
        if object == "oTemple" && city {
            sprite = "sGTemple".into();
        }
    } else if object == "oIce" {
        if rng.chance(1, 80) {
            // FrozenCaveman Destroy releases this live caveman.
            if custom {
                release_custom_frozen(
                    world,
                    Vec2::new(x as f64 * 16.0, y as f64 * 16.0),
                    custom_creation_serial,
                );
            } else {
                treasure = Some(("oCaveman", -8.0, -8.0));
                frozen = true;
            }
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
    if let Some(explicit) = explicit_sprite {
        sprite = explicit;
        if !(object == "oTemple" && city) {
            gold = 0;
        }
    }
    if custom {
        sprite = format!("s{}", &object[1..]);
        gold = 0;
    }
    let has_existing_destroy_treasure = world.entity(id).unwrap().text.contains_key("treasure");
    {
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
    }
    if let Some((name, dx, dy)) = treasure {
        let custom_tile = custom
            && custom_creation_serial.is_some()
            && matches!(object.as_str(), "oBrick" | "oLush" | "oDark" | "oTemple");
        let generated_live_child = !custom && matches!(object.as_str(), "oLush" | "oTemple");
        if custom_tile || generated_live_child {
            // Classic instance_create runs the child's Create immediately.
            // Insert custom children after their parent because the loader's
            // initial birth list was collected before nested Creates ran.
            let pos = Vec2::new(x as f64 * 16.0 + 8.0 + dx, y as f64 * 16.0 + 8.0 + dy);
            spawn_live_terrain_child(
                world,
                rng,
                name,
                pos,
                context.has_spectacles,
                custom_creation_serial,
            );
        } else if !has_existing_destroy_treasure {
            // Legacy terrain metadata remains the adapter for generated
            // families whose source birth ordering is still being modeled.
            let e = world.entity_mut(id).unwrap();
            e.data.insert("embedded_treasure".into(), 1.0);
            e.data.insert("treasure_dx".into(), dx);
            e.data.insert("treasure_dy".into(), dy);
            if crate::catalog::is_a(name, "oItem") {
                e.data.insert(
                    "create_has_spectacles".into(),
                    f64::from(context.has_spectacles),
                );
            }
            e.text.insert("treasure".into(), name.into());
        }
    }
    #[cfg(test)]
    create_contents_tests::observe_after(world, rng, create_trace);
}

/// scrSetupWalls runs after population, so its probes see the solids/traps
/// population added or removed. Keep its initialization guard independent of
/// terrain Create to avoid rerolling buried contents while setting up edges.
#[cfg(test)]
pub(super) fn setup_generated(world: &mut World, rng: &mut Rng, graphics_high: bool) {
    setup_generated_with_revision(
        world,
        rng,
        graphics_high,
        crate::GenerationRevision::HistoricalV1,
        false,
    );
}

#[derive(Clone, Copy)]
enum SetupParent {
    Tile(usize),
    Entity(u64),
}

fn setup_source_point(world: &World, point: Vec2, family: &str) -> bool {
    if family == "oSolid" {
        return world.solid_point_active(point, crate::CollisionPrecision::BoundingBox, None);
    }
    world.tiles_point_active(point, family)
        || world.entities.iter().any(|entity| {
            entity.alive
                && entity.engine_active
                && crate::catalog::is_a(&entity.object, family)
                && entity.mask_contains(point, crate::CollisionPrecision::BoundingBox)
        })
}

fn setup_source_sides(
    world: &World,
    origin: Vec2,
    family: &str,
    bottom: f64,
) -> (bool, bool, bool, bool) {
    let point = |dx: f64, dy: f64, group: &str| {
        setup_source_point(world, Vec2::new(origin.x + dx, origin.y + dy), group)
    };
    match family {
        "oBrick" => (
            origin.y == 0. || point(0., -16., family) || point(0., -16., "oHardBlock"),
            origin.y >= bottom || point(0., 16., family) || point(0., 16., "oHardBlock"),
            point(-16., 0., family) || point(-16., 0., "oHardBlock"),
            point(16., 0., family) || point(16., 0., "oHardBlock"),
        ),
        "oTemple" => (
            // Literal source asymmetry: Temple probes Fake BELOW for up.
            origin.y == 0. || point(0., -16., family) || point(0., 16., "oTempleFake"),
            origin.y >= bottom || point(0., 16., family) || point(0., 16., "oTempleFake"),
            point(-16., 0., family) || point(-16., 0., "oTempleFake"),
            point(16., 0., family) || point(16., 0., "oTempleFake"),
        ),
        "oIce" => (
            // Ice has neither y==0 nor argument0 hard-boundary override.
            point(0., -16., family),
            point(0., 16., family),
            point(-16., 0., family),
            point(16., 0., family),
        ),
        _ => (
            origin.y == 0. || point(0., -16., family),
            origin.y >= bottom || point(0., 16., family),
            point(-16., 0., family),
            point(16., 0., family),
        ),
    }
}

fn setup_source_fringe(
    world: &mut World,
    rng: &mut Rng,
    family: &str,
    origin: Vec2,
    sides: (bool, bool, bool, bool),
) {
    // Keep the original upper-fringe selector and its conditional second
    // draw, but use the active source Solid query for Lush's lower fringe.
    let upper_only = if family == "oLush" {
        (sides.0, true, sides.2, sides.3)
    } else {
        sides
    };
    setup_fringe(world, rng, family, origin, upper_only);
    if family == "oLush"
        && !sides.1
        && !setup_source_point(world, Vec2::new(origin.x, origin.y + 16.), "oSolid")
    {
        let offset = if rng.chance(1, 12) {
            Some(48.)
        } else if rng.chance(1, 12) {
            Some(64.)
        } else {
            None
        };
        if let Some(offset) = offset {
            decoration(
                world,
                "bgCaveTop2",
                Vec2::new(origin.x, origin.y + 16.),
                offset,
                16.,
                3.,
            );
        }
    }
}

fn setup_source_parents(
    world: &mut World,
    rng: &mut Rng,
    city: bool,
    graphics_high: bool,
    bottom: f64,
) {
    // The five canonical families have no descendants. No body creates a
    // new parent in its own family, changes activation, or deletes another
    // parent: each captured identity list is valid for this generation pass.
    // Tile storage cannot recover parents already discarded by a producer.
    for family in ["oBrick", "oLush", "oDark", "oIce", "oTemple"] {
        let mut receivers: Vec<_> = world
            .tiles
            .iter()
            .enumerate()
            .filter(|(_, tile)| tile.engine_active && tile.object == family)
            .map(|(index, tile)| (tile.creation_serial, SetupParent::Tile(index)))
            .chain(
                world
                    .entities
                    .iter()
                    .filter(|entity| {
                        entity.alive && entity.engine_active && entity.object == family
                    })
                    .map(|entity| (entity.creation_serial, SetupParent::Entity(entity.id))),
            )
            .collect();
        receivers.sort_by_key(|(serial, _)| *serial);
        for (serial, receiver) in receivers {
            let (origin, id) = match receiver {
                SetupParent::Tile(index) => {
                    let tile = &world.tiles[index];
                    if !tile.engine_active
                        || tile.creation_serial != serial
                        || tile.object != family
                    {
                        continue;
                    }
                    let (x, y) = (index % world.width, index / world.width);
                    let origin = Vec2::new(x as f64 * 16., y as f64 * 16.);
                    (origin, metadata(world, x, y))
                }
                SetupParent::Entity(id) => {
                    let Some(entity) = world.entity(id) else {
                        continue;
                    };
                    if !entity.engine_active
                        || entity.creation_serial != serial
                        || entity.object != family
                    {
                        continue;
                    }
                    (entity.pos, id)
                }
            };
            if world.entity(id).unwrap().value("terrain_setup_initialized") != 0. {
                continue;
            }
            let sides = setup_source_sides(world, origin, family, bottom);
            let edge = edge_sprite(family, city, sides);
            let entity = world.entity_mut(id).unwrap();
            for (name, value) in ["up", "down", "left", "right"]
                .into_iter()
                .zip([sides.0, sides.1, sides.2, sides.3])
            {
                entity.data.insert(name.into(), f64::from(value));
            }
            if let Some(edge) = edge {
                match receiver {
                    SetupParent::Tile(_) => {
                        entity.text.insert("terrain_sprite".into(), edge);
                    }
                    // Updating the real sprite also updates the live bbox;
                    // no proxy metadata or replacement parent is created.
                    SetupParent::Entity(_) => {
                        entity.sprite = edge;
                    }
                }
                if !(family == "oTemple" && city) {
                    entity.data.remove("gold_chunks");
                    entity.data.remove("gold_nuggets");
                }
            }
            entity.data.insert("terrain_setup_initialized".into(), 1.);
            if graphics_high {
                setup_source_fringe(world, rng, family, origin, sides);
            }
            if family == "oIce" && !sides.1 && rng.chance(1, 20) {
                let below = Vec2::new(origin.x, origin.y + 16.);
                if !setup_source_point(world, below, "oSolid") {
                    let child = world.spawn("oIceBottom", below);
                    world
                        .entity_mut(child)
                        .unwrap()
                        .data
                        .insert("drip_alarm".into(), f64::from(20 + rng.range(381)));
                }
            }
        }
    }
}

pub(super) fn setup_generated_with_revision(
    world: &mut World,
    rng: &mut Rng,
    graphics_high: bool,
    revision: crate::GenerationRevision,
    cemetery: bool,
) {
    let city = world.name == "City of Gold";
    let bottom = if world.area == crate::model::Area::Boss {
        864.
    } else if world.name == "Lake" {
        656.
    } else {
        528.
    };
    if revision == crate::GenerationRevision::SourceClassicV2 {
        setup_source_parents(world, rng, city, graphics_high, bottom);
    } else {
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
                if world.entity(id).unwrap().value("terrain_setup_initialized") != 0.0 {
                    continue;
                }
                let sides = neighbors(world, x, y, &object);
                let edge = edge_sprite(&object, city, sides);
                let entity = world.entity_mut(id).unwrap();
                if let Some(edge) = edge {
                    entity.text.insert("terrain_sprite".into(), edge);
                    // Gold is determined by the current sprite at destruction.
                    // SetupWalls replaces Create's gold sprite on exposed edges.
                    if !(object == "oTemple" && city) {
                        entity.data.remove("gold_chunks");
                        entity.data.remove("gold_nuggets");
                    }
                }
                entity.data.insert("terrain_setup_initialized".into(), 1.0);
                if graphics_high {
                    setup_fringe(
                        world,
                        rng,
                        &object,
                        Vec2::new(x as f64 * 16.0, y as f64 * 16.0),
                        sides,
                    );
                }
                if object == "oIce" && !sides.1 && rng.chance(1, 20) {
                    let below = Vec2::new(x as f64 * 16.0, (y + 1) as f64 * 16.0);
                    if !world.solid_point_except(below, 0) {
                        let id = world.spawn("oIceBottom", below);
                        world
                            .entity_mut(id)
                            .unwrap()
                            .data
                            .insert("drip_alarm".into(), f64::from(20 + rng.range(381)));
                    }
                }
            }
        }
    }
    setup_wall_remainder(world, rng, graphics_high, revision, city, cemetery);
}

/// Execute the authored controller's scrSetupWalls(224) after the room has
/// been assembled. The source-room argument is distinct from generated-level
/// bounds (528/656/864), especially for transition-room terrain at the floor.
pub(super) fn setup_authored_walls(
    world: &mut World,
    rng: &mut Rng,
    graphics_high: bool,
    city: bool,
    cemetery: bool,
) {
    initialize_authored_lava_creates(world, rng);
    setup_source_parents(world, rng, city, graphics_high, 224.);
    setup_wall_remainder(
        world,
        rng,
        graphics_high,
        crate::GenerationRevision::SourceClassicV2,
        city,
        cemetery,
    );
}

fn initialize_authored_lava_creates(world: &mut World, rng: &mut Rng) {
    let mut receivers: Vec<_> = world
        .tiles
        .iter()
        .enumerate()
        .filter(|(_, tile)| tile.object == "oLava")
        .map(|(index, tile)| (tile.creation_serial, SetupParent::Tile(index)))
        .chain(
            world
                .entities
                .iter()
                .filter(|entity| entity.alive && entity.object == "oLava")
                .map(|entity| (entity.creation_serial, SetupParent::Entity(entity.id))),
        )
        .collect();
    receivers.sort_by_key(|(serial, _)| *serial);
    for (_, receiver) in receivers {
        let id = match receiver {
            SetupParent::Tile(index) => metadata(world, index % world.width, index / world.width),
            SetupParent::Entity(id) => id,
        };
        create_lava(
            world.entity_mut(id).expect("authored lava Create receiver"),
            rng,
        );
    }
}

fn setup_wall_remainder(
    world: &mut World,
    rng: &mut Rng,
    graphics_high: bool,
    revision: crate::GenerationRevision,
    city: bool,
    cemetery: bool,
) {
    setup_fake_temple(world, rng, city, graphics_high, revision);
    super::water::setup_with_revision(world, rng, revision);
    for y in 0..world.height {
        for x in 0..world.width {
            if world.tile(x as i32, y as i32).unwrap().object != "oLava" {
                continue;
            }
            let origin = Vec2::new(x as f64 * 16.0, y as f64 * 16.0);
            let exposed = lava_exposed(world, origin, revision);
            let id = metadata(world, x, y);
            setup_lava(world.entity_mut(id).unwrap(), rng, exposed, true);
        }
    }
    let lava: Vec<_> = world
        .entities
        .iter()
        .filter(|entity| entity.alive && entity.object == "oLava")
        .map(|entity| entity.id)
        .collect();
    for id in lava {
        let origin = world.entity(id).unwrap().pos;
        let exposed = lava_exposed(world, origin, revision);
        setup_lava(world.entity_mut(id).unwrap(), rng, exposed, false);
    }
    // Preserve the historical layout/masks and RNG continuation, including
    // imported canonical RNG state whose generation revision is HistoricalV1.
    let source_canopy = revision == crate::GenerationRevision::SourceClassicV2;
    if source_canopy {
        super::canopy::vines(world);
    }
    super::blocks::setup(world, rng, revision);
    if source_canopy {
        super::canopy::trees_and_branches(world, rng, cemetery);
    }
}

/// scrInitLevel propagates surface sprites after background details and
/// post-generation cleanup, rather than as part of scrSetupWalls itself.
pub(super) fn finalize_generated(world: &mut World, revision: crate::GenerationRevision) {
    super::water::propagate_surface(
        world,
        revision == crate::GenerationRevision::SourceClassicV2,
    );
}

/// Source tile_add decorations use background pixel rectangles, not sprites.
pub(super) fn decoration(
    world: &mut World,
    background: &str,
    origin: Vec2,
    source_x: f64,
    size: f64,
    depth: f64,
) {
    let id = world.spawn("oGenerationDecoration", origin);
    let entity = world.entity_mut(id).unwrap();
    entity.sprite = background.into();
    for (key, value) in [
        ("decoration", 1.0),
        ("source_x", source_x),
        ("source_y", 0.0),
        ("source_w", size),
        ("source_h", size),
        ("depth", depth),
    ] {
        entity.data.insert(key.into(), value);
    }
}

fn setup_fringe(
    world: &mut World,
    rng: &mut Rng,
    object: &str,
    origin: Vec2,
    sides: (bool, bool, bool, bool),
) {
    let (up, down, _, _) = sides;
    if !up {
        let choice = match object {
            "oBrick" => Some(("bgCaveTop", if rng.range(3) < 2 { 0.0 } else { 16.0 })),
            "oLush" => Some((
                "bgCaveTop2",
                if rng.chance(1, 8) {
                    32.0
                } else if rng.range(3) < 2 {
                    0.0
                } else {
                    16.0
                },
            )),
            "oDark" => Some(("bgCaveTop3", if rng.range(3) < 2 { 0.0 } else { 16.0 })),
            "oTemple" => {
                if rng.chance(1, 4) {
                    Some(("bgCaveTop4", 0.0))
                } else if rng.chance(1, 4) {
                    Some(("bgCaveTop4", 16.0))
                } else {
                    None
                }
            }
            _ => None,
        };
        if let Some((background, source_x)) = choice {
            decoration(
                world,
                background,
                Vec2::new(origin.x, origin.y - 16.0),
                source_x,
                16.0,
                3.0,
            );
        }
    }
    if object == "oLush"
        && !down
        && !world.solid_point_except(Vec2::new(origin.x, origin.y + 16.0), 0)
    {
        let source_x = if rng.chance(1, 12) {
            Some(48.0)
        } else if rng.chance(1, 12) {
            Some(64.0)
        } else {
            None
        };
        if let Some(source_x) = source_x {
            decoration(
                world,
                "bgCaveTop2",
                Vec2::new(origin.x, origin.y + 16.0),
                source_x,
                16.0,
                3.0,
            );
        }
    }
}

fn setup_fake_temple(
    world: &mut World,
    rng: &mut Rng,
    city: bool,
    graphics_high: bool,
    revision: crate::GenerationRevision,
) {
    let source = revision == crate::GenerationRevision::SourceClassicV2;
    let ids: Vec<_> = world
        .entities
        .iter()
        .filter(|entity| {
            entity.alive
                && entity.object == "oTempleFake"
                && (!source || entity.engine_active)
                && entity.value("fake_setup_initialized") == 0.0
        })
        .map(|entity| entity.id)
        .collect();
    for id in ids {
        let origin = world.entity(id).unwrap().pos;
        let connected = |point: Vec2| {
            if source {
                world.point_family_active(point, "oTemple")
                    || world.point_family_active(point, "oTempleFake")
            } else {
                world.tiles_point(point, "oTemple")
                    || world.tiles_point(point, "oTempleFake")
                    || world.entities.iter().any(|entity| {
                        entity.alive
                            && matches!(entity.object.as_str(), "oTemple" | "oTempleFake")
                            && entity.sprite_bounds().contains(point)
                    })
            }
        };
        let up = origin.y == 0.0 || connected(Vec2::new(origin.x, origin.y - 16.0));
        let down = connected(Vec2::new(origin.x, origin.y + 16.0));
        let left = connected(Vec2::new(origin.x - 16.0, origin.y));
        let right = connected(Vec2::new(origin.x + 16.0, origin.y));
        if city && graphics_high {
            setup_fringe(world, rng, "oTemple", origin, (up, down, left, right));
        }
        let sprite = edge_sprite("oTemple", city, (up, down, left, right));
        let entity = world.entity_mut(id).unwrap();
        if let Some(sprite) = sprite {
            entity.sprite = sprite;
        }
        entity.data.insert("fake_setup_initialized".into(), 1.0);
    }
}

fn release_custom_frozen(world: &mut World, origin: Vec2, after_serial: Option<u64>) {
    // scrCreateTileObj destroys its freshly created FrozenCaveman. In
    // rLoadLevel cleanSolids is false, so that Destroy event releases a live,
    // stunned caveman; there is no frozen overlay left in the custom room.
    let id = world.spawn("oCaveman", origin);
    if let Some(after_serial) = after_serial {
        insert_birth_after(world, id, after_serial);
    }
    let caveman = world.entity_mut(id).unwrap();
    caveman.data.insert("invincible".into(), 20.0);
    caveman.state = 98;
    caveman.timer = caveman.value("stun_time") as u32;
}

/// GameMaker runs the Ice Create and its Destroy-created CaveMan before it
/// creates the next row-major map cell. The custom World already contains all
/// authored cells here, so insert this nested birth into the parent's slot.
fn insert_birth_after(world: &mut World, id: u64, after_serial: u64) {
    let Some(child_serial) = world.entity(id).map(|entity| entity.creation_serial) else {
        return;
    };
    for tile in &mut world.tiles {
        if tile.creation_serial > after_serial && tile.creation_serial < child_serial {
            tile.creation_serial = tile.creation_serial.saturating_add(1);
        }
    }
    for entity in &mut world.entities {
        if entity.id != id
            && entity.creation_serial > after_serial
            && entity.creation_serial < child_serial
        {
            entity.creation_serial = entity.creation_serial.saturating_add(1);
        }
    }
    if let Some(child) = world.entity_mut(id) {
        child.creation_serial = after_serial.saturating_add(1);
    }
}

fn spawn_live_terrain_child(
    world: &mut World,
    rng: &mut Rng,
    object: &str,
    pos: Vec2,
    has_spectacles: bool,
    parent_serial: Option<u64>,
) -> u64 {
    let id =
        crate::creation::spawn_generated_terrain_child(world, rng, object, pos, has_spectacles);
    if let Some(parent_serial) = parent_serial {
        insert_birth_after(world, id, parent_serial);
    }
    if crate::catalog::is_a(object, "oItem") {
        // scrGenerateItem(...,2) applies these producer overrides after Create.
        let child = world.entity_mut(id).unwrap();
        child.data.insert("cost".into(), 0.0);
        child.data.insert("forSale".into(), 0.0);
    }
    id
}

fn initialize_custom_instance(
    world: &mut World,
    rng: &mut Rng,
    id: u64,
    context: super::GenerationContext,
) {
    // Custom compound symbols can overlap static terrain with pools or room
    // borders. Those additional terrain instances live outside tile storage
    // but still execute the same source Create events.
    let Some(entity) = world.entity(id) else {
        return;
    };
    if entity.value("terrain_initialized") != 0.0 {
        return;
    }
    let object = entity.object.clone();
    let origin = entity.pos;
    let creation_serial = entity.creation_serial;
    let mut treasure = None;
    if object == "oIce" {
        if rng.chance(1, 80) {
            release_custom_frozen(world, origin, Some(creation_serial));
        }
    } else {
        if object == "oBrick" {
            rng.range(10);
        }
        let roll = rng.range(100) + 1;
        if roll >= 30
            && origin.x > 1.0
            && origin.x < (world.width - 1) as f64 * 16.0
            && origin.y > 1.0
            && origin.y < (world.height - 1) as f64 * 16.0
        {
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
    if let Some((object, dx, dy)) = treasure {
        spawn_live_terrain_child(
            world,
            rng,
            object,
            Vec2::new(origin.x + 8.0 + dx, origin.y + 8.0 + dy),
            context.has_spectacles,
            Some(creation_serial),
        );
    }
    let entity = world.entity_mut(id).unwrap();
    entity.data.insert("terrain_initialized".into(), 1.0);
    entity.sprite = format!("s{}", &object[1..]);
}

fn lava_exposed(world: &World, origin: Vec2, revision: crate::GenerationRevision) -> bool {
    // scrSetupWalls probes the original instance origin, not its tile center.
    // oLava itself inherits oWater, so this also suppresses buried lava layers.
    let above = Vec2::new(origin.x, origin.y - 16.0);
    let solid_above = if revision == crate::GenerationRevision::SourceClassicV2 {
        world.solid_point_active(above, crate::CollisionPrecision::BoundingBox, None)
    } else {
        world.solid_point_except(above, 0)
    };
    let water_above = if revision == crate::GenerationRevision::SourceClassicV2 {
        world.point_family_active(above, "oWater")
    } else {
        world.tiles_point(above, "oWater")
            || world.entities.iter().any(|entity| {
                entity.alive
                    && crate::catalog::is_a(&entity.object, "oWater")
                    && entity.sprite_bounds().contains(above)
            })
    };
    !solid_above && !water_above
}

fn setup_lava(entity: &mut Entity, rng: &mut Rng, exposed: bool, metadata: bool) {
    if entity.value("lava_setup_initialized") != 0.0 {
        return;
    }
    create_lava(entity, rng);
    let explicit_spurt = entity.value("lava_explicit_spurt") != 0.0;
    if exposed && !explicit_spurt && rng.chance(1, 4) {
        entity.data.insert("spurt".into(), 1.0);
    }
    let sprite = if exposed { "sLavaTop" } else { "sLava" };
    if metadata {
        entity.text.insert("terrain_sprite".into(), sprite.into());
    } else {
        entity.sprite = sprite.into();
    }
    entity.data.insert("lava_setup_initialized".into(), 1.0);
}

fn create_lava(entity: &mut Entity, rng: &mut Rng) {
    if entity.value("lava_initialized") == 0.0 {
        // Preserve authored overrides across the separated Create/setup calls;
        // Create's own default `spurt = false` is not an authored override.
        if entity.data.contains_key("spurt") {
            entity.data.insert("lava_explicit_spurt".into(), 1.0);
        }
        let time = entity
            .data
            .get("spurt_time")
            .copied()
            .unwrap_or_else(|| 100.0 + f64::from(rng.range(201)));
        entity.data.entry("spurt_time".into()).or_insert(time);
        entity.data.entry("spurt_counter".into()).or_insert(time);
        entity.data.entry("spurt".into()).or_insert(0.0);
        entity.data.insert("lava_initialized".into(), 1.0);
        entity.animation = 0.0;
    }
}

#[cfg(test)]
#[path = "terrain_setup_order_tests.rs"]
mod setup_order_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Area;

    #[test]
    fn lava_solid_probe_uses_source_family_but_preserves_historical_adapter() {
        let mut world = World::new(8, 8, Area::Mines, 1);
        let crate_id = world.spawn("oPushBlock", Vec2::new(32.0, 32.0));
        world.entity_mut(crate_id).unwrap().held = true;
        let lava_origin = Vec2::new(32.0, 48.0);

        assert!(world.solid_point_active(
            Vec2::new(32.0, 32.0),
            crate::CollisionPrecision::BoundingBox,
            None,
        ));
        assert!(!lava_exposed(
            &world,
            lava_origin,
            crate::GenerationRevision::SourceClassicV2,
        ));
        assert!(lava_exposed(
            &world,
            lava_origin,
            crate::GenerationRevision::HistoricalV1,
        ));
    }

    #[test]
    fn mines_population_before_block_promotion_is_independent_of_graphics_rng() {
        let source = include_str!(
            "../../../../references/classic-1.1/Scripts/Level Generation/scrInitLevel.gml"
        );
        let population = source.find("scrEntityGen();").unwrap();
        let walls = source.find("scrSetupWalls(864)").unwrap();
        let details = source.find("if (global.graphicsHigh)").unwrap();
        let surface = source.find("scrCheckWaterTop();").unwrap();
        assert!(population < walls && walls < details && details < surface);

        let walls_source = include_str!(
            "../../../../references/classic-1.1/Scripts/Level Generation/scrSetupWalls.gml"
        );
        let graphics = walls_source.find("if (global.graphicsHigh)").unwrap();
        let fringe_roll = walls_source[graphics..].find("rand(1,3)").unwrap() + graphics;
        let blocks = walls_source.find("with oBlock").unwrap();
        assert!(graphics < fringe_roll && fringe_roll < blocks);

        let generate_arm = |flags: &std::collections::BTreeSet<String>| {
            use super::super::blocks::TRACE;
            TRACE.with(|trace| *trace.borrow_mut() = Some(Vec::new()));
            let world = crate::generation::generate(&mut Rng::new(42), 1, flags);
            let mut traces = TRACE.with(|trace| trace.borrow_mut().take().unwrap());
            assert_eq!(traces.len(), 1);
            let trace = traces.pop().unwrap();
            super::super::blocks::tests::assert_source_outcome(&world, &trace);
            (world, trace.before)
        };
        let (high, high_before_promotion) = generate_arm(&Default::default());
        let (low, low_before_promotion) =
            generate_arm(&["low_graphics".into()].into_iter().collect());
        let actors = |world: &World| {
            world
                .entities
                .iter()
                .filter(|entity| {
                    !matches!(
                        entity.object.as_str(),
                        "oTerrainMetadata" | "oGenerationDecoration" | "oIceBottom"
                    )
                })
                .map(|entity| {
                    (
                        entity.object.clone(),
                        entity.pos,
                        entity.sprite.clone(),
                        entity.state,
                        entity.timer,
                    )
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(
            actors(&high_before_promotion),
            actors(&low_before_promotion),
            "Mines enemy/treasure population precedes graphics and Block promotion"
        );
        let decorations = |world: &World| {
            world
                .entities
                .iter()
                .filter(|entity| entity.object == "oGenerationDecoration")
                .count()
        };
        assert!(decorations(&high) > decorations(&low));
    }

    #[test]
    fn create_phase_keeps_source_gold_until_setup_without_fringe_draws() {
        let source = include_str!(
            "../../../../references/classic-1.1/Objects/Blocks/oBrick.events/Create.xml"
        );
        assert!(source.contains("n = rand(1,10);"));
        assert!(source.contains("n = rand(1,100);"));
        assert!(source.contains("if (n &lt; 20) sprite_index = sBrickGold;"));
        let seed = (0..64)
            .find(|&seed| {
                let mut source_rng = Rng::new(seed);
                source_rng.range(10);
                1 + source_rng.range(100) < 20
            })
            .unwrap();
        let mut source_rng = Rng::new(seed);
        source_rng.range(10);
        source_rng.range(100);
        let mut world = World::new(3, 3, Area::Mines, 1);
        world.set_tile(1, 1, "oBrick");
        let mut actual = Rng::new(seed);
        create_generated(&mut world, &mut actual);
        let id = metadata(&mut world, 1, 1);
        let brick = world.entity(id).unwrap();
        assert_eq!(brick.text["terrain_sprite"], "sBrickGold");
        assert_eq!(brick.value("gold_chunks"), 3.0);
        assert_eq!(brick.value("terrain_setup_initialized"), 0.0);
        assert!(
            !world
                .entities
                .iter()
                .any(|e| e.object == "oGenerationDecoration")
        );
        assert_eq!(actual.next_u64(), source_rng.next_u64());
        setup_generated(&mut world, &mut actual, false);
        let brick = world.entity(id).unwrap();
        assert_eq!(brick.text["terrain_sprite"], "sCaveUp2");
        assert_eq!(brick.value("gold_chunks"), 0.0);
        assert_eq!(brick.value("terrain_initialized"), 1.0);
        assert_eq!(brick.value("terrain_setup_initialized"), 1.0);
    }

    #[test]
    fn setup_observes_terrain_changes_since_create_and_preserves_metadata() {
        let mut world = World::new(5, 5, Area::Mines, 1);
        for y in 1..=3 {
            world.set_tile(2, y, "oBrick");
        }
        let id = metadata(&mut world, 2, 2);
        let brick = world.entity_mut(id).unwrap();
        brick.data.insert("invincible".into(), 1.0);
        brick.text.insert("treasure".into(), "oRubyBig".into());
        let mut rng = Rng::new(4);
        create_generated(&mut world, &mut rng);
        assert!(!world.entity(id).unwrap().text["terrain_sprite"].starts_with("sCave"));
        // A population operation removes the neighbor before SetupWalls.
        world.set_tile(2, 1, "");
        setup_generated(&mut world, &mut rng, false);
        let brick = world.entity(id).unwrap();
        assert_eq!(brick.text["terrain_sprite"], "sCaveUp");
        assert_eq!(brick.text["treasure"], "oRubyBig");
        assert_eq!(brick.value("invincible"), 1.0);
        assert_eq!(
            world.entities.iter().filter(|e| e.id == id).count(),
            1,
            "phase changes must retain metadata identity"
        );
    }

    #[test]
    fn lava_create_timer_precedes_population_and_surface_setup() {
        let mut world = World::new(5, 5, Area::Temple, 13);
        let id = world.spawn("oLava", Vec2::new(16.0, 32.0));
        let mut actual = Rng::new(42);
        let mut source_rng = Rng::new(42);
        let time = 100.0 + f64::from(source_rng.range(201));
        create_generated(&mut world, &mut actual);
        let lava = world.entity(id).unwrap();
        assert_eq!(lava.sprite, "sLava");
        assert_eq!(lava.value("spurt_time"), time);
        assert_eq!(lava.value("lava_setup_initialized"), 0.0);
        assert_eq!(actual.next_u64(), source_rng.next_u64());
        // A solid created by population covers the lava before SetupWalls.
        world.spawn("oPushBlock", Vec2::new(16.0, 16.0));
        setup_generated(&mut world, &mut actual, false);
        let lava = world.entity(id).unwrap();
        assert_eq!(lava.sprite, "sLava");
        assert_eq!(lava.value("spurt_time"), time);
        assert_eq!(lava.value("spurt"), 0.0);
        assert_eq!(lava.value("lava_setup_initialized"), 1.0);
    }

    #[test]
    fn horizontal_surface_propagation_waits_for_final_phase() {
        let mut world = World::new(12, 12, Area::Jungle, 5);
        let open = world.spawn("oWaterSwim", Vec2::new(64.0, 80.0));
        let covered = world.spawn("oWaterSwim", Vec2::new(80.0, 80.0));
        world.set_tile(5, 4, "oLush");
        let mut rng = Rng::new(1);
        create_generated(&mut world, &mut rng);
        assert_eq!(world.entity(open).unwrap().sprite, "sWater");
        setup_generated(&mut world, &mut rng, false);
        assert_eq!(world.entity(open).unwrap().sprite, "sWaterTop");
        assert_eq!(world.entity(covered).unwrap().sprite, "sWater");
        finalize_generated(&mut world, crate::GenerationRevision::HistoricalV1);
        assert_eq!(world.entity(covered).unwrap().sprite, "sWaterTop");
    }

    #[test]
    fn high_graphics_fringe_choices_follow_source_short_circuit_draw_order() {
        for seed in 0..64 {
            for object in ["oBrick", "oLush", "oDark", "oTemple"] {
                let mut expected = Rng::new(seed);
                let mut choices = Vec::new();
                let upper = match object {
                    "oBrick" | "oDark" => Some(if expected.range(3) < 2 { 0.0 } else { 16.0 }),
                    "oLush" => Some(if expected.chance(1, 8) {
                        32.0
                    } else if expected.range(3) < 2 {
                        0.0
                    } else {
                        16.0
                    }),
                    _ => {
                        if expected.chance(1, 4) {
                            Some(0.0)
                        } else if expected.chance(1, 4) {
                            Some(16.0)
                        } else {
                            None
                        }
                    }
                };
                if let Some(x) = upper {
                    choices.push((x, 16.0));
                }
                if object == "oLush" {
                    let lower = if expected.chance(1, 12) {
                        Some(48.0)
                    } else if expected.chance(1, 12) {
                        Some(64.0)
                    } else {
                        None
                    };
                    if let Some(x) = lower {
                        choices.push((x, 48.0));
                    }
                }
                let mut world = World::new(4, 4, Area::Mines, 1);
                let mut actual = Rng::new(seed);
                setup_fringe(
                    &mut world,
                    &mut actual,
                    object,
                    Vec2::new(32.0, 32.0),
                    (false, false, false, false),
                );
                let emitted: Vec<_> = world
                    .entities
                    .iter()
                    .map(|e| (e.value("source_x"), e.pos.y))
                    .collect();
                assert_eq!(emitted, choices, "{object} seed {seed}");
                assert_eq!(
                    actual.range(100000),
                    expected.range(100000),
                    "{object} seed {seed}"
                );
            }
        }
    }

    #[test]
    fn fake_temple_keeps_source_gold_edge_and_covered_base_sprite_rules() {
        for city in [false, true] {
            for covered in [false, true] {
                let mut world = World::new(5, 5, Area::Temple, 13);
                let id = world.spawn("oTempleFake", Vec2::new(32.0, 32.0));
                if covered {
                    world.set_tile(2, 1, "oTemple");
                    world.set_tile(2, 3, "oTemple");
                }
                let mut actual = Rng::new(42);
                let mut expected = actual.clone();
                setup_fake_temple(
                    &mut world,
                    &mut actual,
                    city,
                    false,
                    crate::GenerationRevision::SourceClassicV2,
                );
                assert_eq!(
                    actual.range(100000),
                    expected.range(100000),
                    "low graphics must consume no fringe RNG"
                );
                let entity = world.entity(id).unwrap();
                assert_eq!(
                    entity.sprite,
                    if covered {
                        "sTemple"
                    } else if city {
                        "sGTempleUp6"
                    } else {
                        "sTempleUp6"
                    }
                );
                assert!(!entity.solid);
            }
        }
        // Normal fake temple never adds fringes, even at high graphics.
        let mut world = World::new(5, 5, Area::Temple, 13);
        world.spawn("oTempleFake", Vec2::new(32.0, 32.0));
        let mut actual = Rng::new(42);
        let mut expected = actual.clone();
        setup_fake_temple(
            &mut world,
            &mut actual,
            false,
            true,
            crate::GenerationRevision::SourceClassicV2,
        );
        assert_eq!(actual.range(100000), expected.range(100000));
        assert!(!world.entities.iter().any(|e| e.sprite == "bgCaveTop4"));
    }

    #[test]
    fn fake_temple_source_setup_skips_inactive_receivers() {
        let mut world = World::new(5, 5, Area::Temple, 13);
        let id = world.spawn("oTempleFake", Vec2::new(32.0, 32.0));
        world.entity_mut(id).unwrap().engine_active = false;
        let sprite = world.entity(id).unwrap().sprite.clone();

        setup_fake_temple(
            &mut world,
            &mut Rng::new(42),
            false,
            true,
            crate::GenerationRevision::SourceClassicV2,
        );

        let fake = world.entity(id).unwrap();
        assert_eq!(fake.sprite, sprite);
        assert_eq!(fake.value("fake_setup_initialized"), 0.0);
    }

    #[test]
    fn lava_setup_uses_source_origins_water_ancestry_and_dynamic_solid_masks() {
        for blocker in ["oBrick", "oWaterSwim", "oPushBlock", "oLava"] {
            let mut world = World::new(4, 4, Area::Temple, 13);
            let lava = world.spawn("oLava", Vec2::new(16.0, 32.0));
            if matches!(blocker, "oBrick" | "oWaterSwim") {
                world.set_tile(1, 1, blocker);
            } else {
                world.spawn(blocker, Vec2::new(16.0, 16.0));
            }
            initialize(&mut world, &mut Rng::new(42));
            let lava = world.entity(lava).unwrap();
            assert_eq!(lava.sprite, "sLava", "blocker {blocker}");
            assert_eq!(lava.value("spurt"), 0.0);
            assert!((100.0..=300.0).contains(&lava.value("spurt_time")));
            assert_eq!(lava.value("spurt_counter"), lava.value("spurt_time"));
        }
    }

    #[test]
    fn exposed_lava_uses_source_spurt_roll_and_preserves_explicit_state() {
        let mut spurts = 0;
        for seed in 0..64 {
            let mut world = World::new(4, 4, Area::Temple, 13);
            let id = world.spawn("oLava", Vec2::new(16.0, 32.0));
            let mut expected = Rng::new(seed);
            let time = 100.0 + f64::from(expected.range(201));
            let spurt = expected.chance(1, 4);
            initialize(&mut world, &mut Rng::new(seed));
            let lava = world.entity(id).unwrap();
            assert_eq!(lava.sprite, "sLavaTop");
            assert_eq!(lava.value("spurt_time"), time);
            assert_eq!(lava.value("spurt"), f64::from(spurt));
            spurts += usize::from(spurt);
            let previous = serde_json::to_string(&world).unwrap();
            initialize(&mut world, &mut Rng::new(999));
            assert_eq!(serde_json::to_string(&world).unwrap(), previous);
        }
        assert!(spurts > 0 && spurts < 64);
        let mut world = World::new(4, 4, Area::Temple, 13);
        world.set_tile(1, 2, "oLava");
        let id = metadata(&mut world, 1, 2);
        let lava = world.entity_mut(id).unwrap();
        lava.data.insert("spurt".into(), 1.0);
        lava.data.insert("spurt_time".into(), 123.0);
        lava.data.insert("spurt_counter".into(), 7.0);
        initialize(&mut world, &mut Rng::new(42));
        let lava = world.entity(id).unwrap();
        assert_eq!(lava.text["terrain_sprite"], "sLavaTop");
        assert_eq!(lava.value("spurt"), 1.0);
        assert_eq!(lava.value("spurt_time"), 123.0);
        assert_eq!(lava.value("spurt_counter"), 7.0);
    }

    #[test]
    fn generated_olmec_lava_preserves_source_ordered_surfaces_and_spurts() {
        let world = crate::generation::generate(&mut Rng::new(42), 16, &Default::default());
        let lava: Vec<_> = world
            .entities
            .iter()
            .filter(|entity| entity.object == "oLava")
            .collect();
        assert_eq!(lava.len(), 240);
        assert!(
            lava.iter()
                .filter(|entity| entity.pos.y == 768.0)
                .any(|entity| entity.value("spurt") == 1.0)
        );
        // rOlmec.xml creates (640,768) before (640,784). scrSetupWalls changes
        // the first instance to sLavaTop, whose cached aggregate bbox still
        // contains the collision_point at y=768. The lower lava therefore sees
        // that water-family instance above it and remains covered.
        let at = |x, y| {
            lava.iter()
                .find(|entity| entity.pos == Vec2::new(x, y))
                .unwrap()
        };
        assert_eq!(at(640.0, 784.0).sprite, "sLava");
        assert_eq!(at(640.0, 784.0).value("spurt"), 0.0);
        assert_eq!(at(624.0, 784.0).sprite, "sLava");

        // scrCheckWaterTop later propagates the surface horizontally. It can
        // spread across a row, but does not propagate into this covered row.
        assert_eq!(at(16.0, 784.0).sprite, "sLava");
        for entity in lava {
            assert_eq!(entity.value("lava_initialized"), 1.0);
            if entity.value("spurt") == 1.0 {
                assert_eq!(entity.sprite, "sLavaTop");
            }
        }
    }

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
        assert_eq!(
            w.entities
                .iter()
                .filter(|e| e.object == "oTerrainMetadata")
                .count(),
            1
        );
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

#[cfg(test)]
#[path = "terrain_create_contents_tests.rs"]
mod create_contents_tests;

#[cfg(test)]
pub(super) fn observe_init_cleanup(world: &World, after: bool) {
    create_contents_tests::observe_init_cleanup(world, after);
}

#[cfg(test)]
#[path = "synchronous_room_terrain_tests.rs"]
mod synchronous_room_terrain_tests;

#[cfg(test)]
#[path = "lava_source_birth_tests.rs"]
pub(super) mod lava_source_birth_tests;
