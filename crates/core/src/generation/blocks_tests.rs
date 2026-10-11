use super::*;
use crate::{Area, Game, InputFrame, Rect, RulesPreset};
use std::collections::BTreeSet;

#[derive(Debug, PartialEq)]
struct Birth {
    object: &'static str,
    sprite: &'static str,
    pos: Vec2,
    vel: Vec2,
}

// Independent literal scrSetupWalls/Block Destroy statement ledger. Random
// values come from the selected port Rng mode, never fixed SplitMix constants;
// this checks statement/callback order, not original-runtime seed equivalence.
struct Expected {
    births: Vec<Birth>,
    promoted: BTreeSet<u64>,
}

fn source_phase(world: &World, rng: &mut Rng) -> Expected {
    let mut blocks: Vec<_> = world
        .tiles
        .iter()
        .enumerate()
        .filter_map(|(index, tile)| {
            if tile.object != "oBlock" {
                return None;
            }
            let origin = Vec2::new(
                (index % world.width) as f64 * 16.0,
                (index / world.width) as f64 * 16.0,
            );
            let clean = world
                .entities
                .iter()
                .find(|e| {
                    e.alive
                        && e.object == "oTerrainMetadata"
                        && e.pos == Vec2::new(origin.x + 8.0, origin.y + 8.0)
                })
                .map_or(false, |e| e.value("cleanDeath") != 0.0);
            Some((tile.creation_serial, origin, clean))
        })
        .chain(world.entities.iter().filter_map(|entity| {
            (entity.alive && entity.object == "oBlock").then_some((
                entity.creation_serial,
                entity.pos,
                entity.value("cleanDeath") != 0.0,
            ))
        }))
        .collect();
    blocks.sort_by_key(|(serial, _, _)| *serial);
    let query = |position: Vec2, family: &str| {
        // Source geometry primitives are shared; selection, operand evaluation,
        // random continuation and callback ordering are transcribed separately.
        world.tiles_point(position, family)
            || world.entities.iter().any(|entity| {
                entity.alive
                    && crate::catalog::is_a(&entity.object, family)
                    && entity.mask_contains(position, crate::CollisionPrecision::BoundingBox)
            })
    };
    let city = world.name == "City of Gold";
    let mut births = Vec::new();
    let mut promoted = BTreeSet::new();
    for (serial, origin, clean) in blocks {
        let brick = query(Vec2::new(origin.x, origin.y + 16.0), "oBrick");
        let temple = query(Vec2::new(origin.x, origin.y + 16.0), "oTemple");
        let hard = query(Vec2::new(origin.x, origin.y + 16.0), "oHardBlock");
        let left_lava = query(Vec2::new(origin.x - 16.0, origin.y), "oLava");
        let right_lava = query(Vec2::new(origin.x + 16.0, origin.y), "oLava");
        let n = rng.range(4) + 1;
        if !(brick || temple || hard) || left_lava || right_lava || n != 1 {
            continue;
        }
        promoted.insert(serial);
        births.push(Birth {
            object: "oPushBlock",
            sprite: if city { "sGoldBlock" } else { "sBlock" },
            pos: origin,
            vel: Vec2::default(),
        });
        if clean {
            continue;
        }
        if city {
            for object in ["oGoldChunk", "oGoldChunk", "oGoldChunk", "oGoldNugget"] {
                let x = origin.x + 8.0 + f64::from(rng.range(5)) - f64::from(rng.range(5));
                let y = origin.y + 8.0 + f64::from(rng.range(5)) - f64::from(rng.range(5));
                // Each collectible's fixed Create finishes before launch RNG.
                let xv = f64::from(rng.range(4)) - f64::from(rng.range(4));
                let yv = 2.0 + f64::from(rng.range(3));
                births.push(Birth {
                    object,
                    sprite: if object == "oGoldChunk" {
                        "sGoldChunk"
                    } else {
                        "sGoldNugget"
                    },
                    pos: Vec2::new(x, y),
                    vel: Vec2::new(xv, yv),
                });
            }
        } else {
            for (object, sprite) in [
                ("oRubble", "sRubbleLush"),
                ("oRubbleSmall", "sRubbleLushSmall"),
                ("oRubbleSmall", "sRubbleLushSmall"),
            ] {
                let x = origin.x + 8.0 + f64::from(rng.range(9)) - f64::from(rng.range(9));
                let y = origin.y + 8.0 + f64::from(rng.range(9)) - f64::from(rng.range(9));
                births.push(Birth {
                    object,
                    sprite,
                    pos: Vec2::new(x, y),
                    vel: Vec2::default(),
                });
            }
        }
    }
    Expected { births, promoted }
}

fn assert_retirement(world: &World, before: &World, expected: &Expected) {
    for (index, tile) in before
        .tiles
        .iter()
        .enumerate()
        .filter(|(_, tile)| tile.object == "oBlock")
    {
        let converted = expected.promoted.contains(&tile.creation_serial);
        let after = &world.tiles[index];
        if converted {
            assert!(after.object.is_empty(), "retire original tile {index}");
        } else {
            assert_eq!(after, tile, "preserve unselected tile {index}");
        }
        let center = Vec2::new(
            (index % before.width) as f64 * 16.0 + 8.0,
            (index / before.width) as f64 * 16.0 + 8.0,
        );
        for metadata in before
            .entities
            .iter()
            .filter(|e| e.alive && e.object == "oTerrainMetadata" && e.pos == center)
        {
            assert_eq!(
                world.entity(metadata.id).is_none(),
                converted,
                "original metadata {} for tile {index}",
                metadata.id
            );
        }
    }
    for entity in before
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oBlock")
    {
        if expected.promoted.contains(&entity.creation_serial) {
            assert!(
                world.entity(entity.id).is_none(),
                "retire original entity {}",
                entity.id
            );
        } else {
            let after = world
                .entity(entity.id)
                .expect("preserve unselected source entity");
            assert_eq!(after.creation_serial, entity.creation_serial);
            assert_eq!(after.pos, entity.pos);
            assert_eq!(after.object, "oBlock");
        }
    }
}

fn assert_births(world: &World, after_serial: u64, expected: &[Birth]) {
    let mut actual: Vec<_> = world
        .entities
        .iter()
        .filter(|entity| {
            entity.creation_serial > after_serial
                && matches!(
                    entity.object.as_str(),
                    "oPushBlock" | "oRubble" | "oRubbleSmall" | "oGoldChunk" | "oGoldNugget"
                )
        })
        .collect();
    actual.sort_by_key(|entity| entity.creation_serial);
    assert_eq!(actual.len(), expected.len());
    for (entity, expected) in actual.into_iter().zip(expected) {
        assert_eq!(entity.object, expected.object);
        assert_eq!(entity.sprite, expected.sprite);
        assert_eq!(entity.pos, expected.pos);
        assert_eq!(entity.vel, expected.vel);
    }
}

// Used by the real high/low Mines generation control as well as the focused
// phase fixtures; each graphics arm retains final source callback/RNG checks.
pub(in crate::generation) fn assert_source_outcome(world: &World, trace: &SetupTrace) {
    let mut source = trace.rng_before.clone();
    let expected = source_phase(&trace.before, &mut source);
    assert_births(world, trace.before.creation_serial, &expected.births);
    assert_retirement(world, &trace.before, &expected);
    assert_eq!(trace.rng_after.clone().next_u64(), source.next_u64());
}

fn fixture(city: bool) -> World {
    let mut world = World::new(20, 15, Area::Mines, 1);
    if city {
        world.name = "City of Gold".into();
    }
    // Birth order deliberately disagrees with coordinate/storage order.
    world.spawn("oBlock", Vec2::new(144.0, 64.0));
    world.set_tile(4, 4, "oBlock");
    world.spawn("oBlock", Vec2::new(64.0, 64.0)); // distinct coincident receiver
    let clean_tile = world.spawn("oTerrainMetadata", Vec2::new(72.0, 72.0));
    world
        .entity_mut(clean_tile)
        .unwrap()
        .data
        .insert("cleanDeath".into(), 1.0);
    world.set_tile(6, 2, "oBlock"); // unsupported, but still consumes rand1..4
    world.set_tile(8, 4, "oBlock");
    let metadata = world.spawn("oTerrainMetadata", Vec2::new(136.0, 72.0));
    world
        .entity_mut(metadata)
        .unwrap()
        .data
        .insert("cleanDeath".into(), 1.0);
    world
        .entity_mut(metadata)
        .unwrap()
        .data
        .insert("invincible".into(), 1.0);
    for x in [4, 8, 9] {
        world.set_tile(x, 5, "oHardBlock");
    }
    // These sibling resources must neither promote nor consume Block rolls.
    world.set_tile(11, 4, "oXocBlock");
    world.spawn("oPushBlock", Vec2::new(192.0, 64.0));
    world
}

#[test]
fn setup_block_promotion_mixed_receivers_interleave_create_destroy_and_next_roll() {
    for city in [false, true] {
        let mut promoted = 0;
        for seed in 0..32 {
            let mut world = fixture(city);
            let before = world.clone();
            let mut actual = Rng::new(seed);
            let mut source = actual.clone();
            let expected = source_phase(&before, &mut source);
            promoted += expected
                .births
                .iter()
                .filter(|birth| birth.object == "oPushBlock")
                .count();
            super::super::terrain::setup_generated(&mut world, &mut actual, false);
            assert_births(&world, before.creation_serial, &expected.births);
            assert_retirement(&world, &before, &expected);
            assert_eq!(actual.next_u64(), source.next_u64());
            let count_blocks = |world: &World| {
                world
                    .tiles
                    .iter()
                    .filter(|tile| tile.object == "oBlock")
                    .count()
                    + world
                        .entities
                        .iter()
                        .filter(|e| e.alive && e.object == "oBlock")
                        .count()
            };
            let converted = expected
                .births
                .iter()
                .filter(|birth| birth.object == "oPushBlock")
                .count();
            assert_eq!(count_blocks(&world), count_blocks(&before) - converted);
            assert_eq!(world.tile(11, 4).unwrap().object, "oXocBlock");
            assert_eq!(world.fluid_dirty, before.fluid_dirty);
        }
        assert!(
            promoted > 0,
            "source preconditions exercise successful promotions"
        );
    }
}

#[test]
fn setup_block_promotion_named_support_families_and_lava_guards() {
    let mut observed_promotion = 0;
    for (support, dynamic, lava_side, allowed) in [
        ("oBrick", false, 0, true),
        ("oTemple", true, 0, true),
        ("oHardBlock", false, 0, true),
        ("oLush", false, 0, false),
        ("oDark", false, 0, false),
        ("oIce", false, 0, false),
        ("oBlock", false, 0, false),
        ("oPushBlock", true, 0, false),
        ("oHardBlock", false, -1, false),
        ("oHardBlock", false, 1, false),
    ] {
        let mut case_promotions = 0;
        for seed in 0..16 {
            let mut world = World::new(20, 15, Area::Mines, 1);
            world.set_tile(5, 4, "oBlock");
            if dynamic {
                world.spawn(support, Vec2::new(80.0, 80.0));
            } else {
                world.set_tile(5, 5, support);
            }
            if lava_side != 0 {
                world.spawn("oLava", Vec2::new(80.0 + f64::from(lava_side) * 16.0, 64.0));
            }
            let before_serial = world.creation_serial;
            let mut rng = Rng::new(seed);
            super::super::terrain::setup_generated(&mut world, &mut rng, false);
            let promoted = world.entities.iter().any(|e| {
                e.creation_serial > before_serial
                    && e.object == "oPushBlock"
                    && e.pos == Vec2::new(80.0, 64.0)
            });
            if !allowed {
                assert!(!promoted, "{support}, lava {lava_side}");
            }
            observed_promotion += usize::from(promoted);
            case_promotions += usize::from(promoted);
        }
        if allowed {
            assert!(case_promotions > 0, "exercise positive support {support}");
        }
    }
    assert!(observed_promotion > 0);
}

#[test]
fn setup_block_promotion_eager_roll_on_unsupported_live_block_only() {
    for seed in 0..32 {
        let mut world = World::new(20, 15, Area::Mines, 1);
        world.set_tile(5, 4, "oBlock");
        let dead = world.spawn("oBlock", Vec2::new(112.0, 64.0));
        world.entity_mut(dead).unwrap().alive = false;
        world.set_tile(9, 4, "oXocBlock");
        let mut actual = Rng::new(seed);
        let mut source = actual.clone();
        source.range(4); // eager condition, despite down=false
        super::super::terrain::setup_generated(&mut world, &mut actual, false);
        assert_eq!(actual.next_u64(), source.next_u64());
        assert_eq!(world.tile(5, 4).unwrap().object, "oBlock");
        assert_eq!(world.tile(9, 4).unwrap().object, "oXocBlock");
    }
}

#[test]
fn setup_block_promotion_own_destroy_ignores_treasure_labels_and_stale_buried_metadata() {
    let mut promoted = 0;
    for seed in 0..32 {
        let mut world = World::new(20, 15, Area::Mines, 1);
        world.set_tile(5, 4, "oBlock");
        world.set_tile(5, 5, "oHardBlock");
        let id = world.spawn("oTerrainMetadata", Vec2::new(88.0, 72.0));
        let metadata = world.entity_mut(id).unwrap();
        metadata.data.insert("embedded_treasure".into(), 1.0);
        metadata.text.insert("treasure".into(), "oRubyBig".into());
        let mut rng = Rng::new(seed);
        super::super::terrain::setup_generated(&mut world, &mut rng, false);
        if world.tile(5, 4).unwrap().object.is_empty() {
            promoted += 1;
            assert!(world.entity(id).is_none());
        }
        assert!(!world.entities.iter().any(|e| e.object == "oRubyBig"));
    }
    assert!(promoted > 0);
}

#[test]
fn setup_block_promotion_actual_game_generation_has_literal_local_phase_continuation() {
    let mut promotions = 0;
    for level in [1, 5, 9, 13, 16] {
        for seed in 0..4 {
            TRACE.with(|trace| *trace.borrow_mut() = Some(Vec::new()));
            let mut game = Game::shell(seed, RulesPreset::Classic);
            game.flags.extend([
                "no_dark_level".into(),
                "low_graphics".into(),
                "made_udjat_eye".into(),
                "made_market_entrance".into(),
            ]);
            game.data.insert("next_level".into(), f64::from(level));
            game.next_level();
            let traces = TRACE.with(|trace| trace.borrow_mut().take().unwrap());
            assert_eq!(traces.len(), 1, "level {level}, seed {seed}");
            let trace = &traces[0];
            let mut source = trace.rng_before.clone();
            let expected = source_phase(&trace.before, &mut source);
            promotions += expected
                .births
                .iter()
                .filter(|birth| birth.object == "oPushBlock")
                .count();
            assert_births(&game.world, trace.before.creation_serial, &expected.births);
            assert_retirement(&game.world, &trace.before, &expected);
            assert_eq!(trace.rng_after.clone().next_u64(), source.next_u64());
        }
    }
    assert!(
        promotions > 0,
        "natural generated fixtures exercise supported Blocks"
    );
}

#[test]
fn setup_block_promotion_push_block_shared_create_city_defaults_and_caller_overrides() {
    let mut game = Game::shell(4, RulesPreset::Classic);
    game.world.name = "City of Gold".into();
    game.player.pos = Vec2::new(40.0, 40.0);
    game.world.set_tile(5, 5, "oHardBlock");
    let mut source = game.rng.clone();
    let id = game.spawn("oPushBlock", Vec2::new(80.0, 64.0));
    let block = game.world.entity(id).unwrap();
    assert_eq!(block.sprite, "sGoldBlock");
    assert_eq!(block.bounds(), Rect::new(80.0, 64.0, 16.0, 16.0));
    assert_eq!(block.value("push_gravity"), 0.6);
    assert_eq!(block.value("invincible"), 0.0);
    assert_eq!(block.value("cleanDeath"), 0.0);
    assert_eq!(block.text.get("type").map(String::as_str), Some("None"));
    assert_eq!(game.rng.clone().next_u64(), source.clone().next_u64());
    let block = game.world.entity_mut(id).unwrap();
    block.set_sprite("sBlock");
    block.data.insert("cleanDeath".into(), 1.0);
    crate::creation::initialize_entity_local(&mut game.world, &mut game.rng, id);
    game.step(&InputFrame::default());
    assert_eq!(game.world.entity(id).unwrap().sprite, "sBlock");
    assert_eq!(game.world.entity(id).unwrap().value("cleanDeath"), 1.0);
    assert_eq!(game.rng.next_u64(), source.next_u64());
}
