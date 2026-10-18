//! Authored Classic room placement, transcribed from the distributed 1.1 XML.
//!
//! rTutorial.xml is 672x304; rOlmec.xml is 672x880. The Olmec room
//! combines generated upper/underground rooms with this authored arena, floor,
//! worshippers, and lava basin. No room-generation randomness is consumed here.
//! Copyright (c) 2008, 2009 Derek Yu and Mossmouth, LLC.
//! Source-derived layout is covered by the Spelunky User License; see LICENSE.

use crate::model::{Area, Rng, Vec2, World};
use std::collections::BTreeSet;

/// rOlmec's static Create calls run before oGame.Create starts route
/// generation. The first draw is oOlmec.facing, followed by each authored
/// oLava.spurtTime in the resource's instance order. Positions encode
/// `row * 40 + column`, with row 0 at y=768 and column 0 at x=16.
pub(super) const BOSS_LAVA_CREATE_ORDER: [u8; 240] = [
    200, 160, 120, 80, 40, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19,
    20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 79, 78, 77, 76,
    112, 110, 107, 106, 104, 102, 101, 98, 97, 96, 95, 94, 93, 92, 91, 90, 89, 88, 87, 86, 85, 41,
    42, 43, 52, 55, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 75, 74, 111,
    109, 108, 105, 103, 100, 99, 57, 56, 54, 53, 51, 50, 49, 48, 47, 46, 45, 44, 84, 83, 82, 81,
    121, 122, 123, 124, 125, 126, 127, 128, 129, 130, 131, 132, 133, 134, 179, 180, 181, 182, 183,
    184, 185, 186, 187, 188, 189, 190, 191, 192, 193, 194, 195, 156, 155, 140, 135, 136, 137, 138,
    139, 141, 145, 149, 154, 153, 152, 150, 148, 142, 143, 144, 146, 147, 151, 113, 114, 115, 116,
    117, 118, 119, 159, 158, 157, 196, 197, 198, 199, 239, 238, 237, 236, 235, 234, 233, 232, 231,
    230, 229, 228, 227, 226, 225, 224, 223, 222, 221, 220, 219, 218, 178, 177, 176, 175, 174, 173,
    172, 171, 170, 169, 168, 167, 166, 165, 164, 161, 162, 163, 201, 202, 203, 204, 205, 206, 207,
    208, 209, 210, 211, 212, 213, 214, 215, 216, 217,
];

/// rOlmec's 80 authored oTemple instances, in room-resource order. Each value
/// stores `(row * 40 + column)` where row 0/1 maps to y=560/576 and column 0
/// maps to x=16. Their Create events run before Olmec, Lava, and oGame.Create.
pub(super) const BOSS_TEMPLE_CREATE_ORDER: [u8; 80] = [
    0, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32,
    33, 34, 35, 36, 37, 38, 39, 79, 78, 77, 76, 75, 74, 73, 72, 71, 70, 69, 68, 67, 66, 65, 64, 63,
    62, 61, 60, 59, 58, 57, 56, 55, 54, 53, 52, 51, 50, 5, 6, 7, 9, 8, 49, 48, 47, 46, 45, 4, 3, 2,
    1, 40, 41, 42, 43, 44,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum BossTempleTreasure {
    SapphireBig,
    EmeraldBig,
    RubyBig,
    Underground(u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct BossTempleCreate {
    pub roll: u8,
    pub treasure: Option<BossTempleTreasure>,
}

/// The source Create results captured before any route or room draws. Temple
/// outcomes are attached at their source phase, before oGame.Create starts the
/// generated level.
pub(super) struct BossCreateTape {
    city_of_gold: bool,
    has_spectacles: bool,
    temple_creates: [BossTempleCreate; 80],
    olmec_facing: i8,
    lava_spurt_times: [u16; 240],
}

pub(super) fn create_source_boss_tape(
    rng: &mut Rng,
    city_of_gold: bool,
    has_spectacles: bool,
) -> BossCreateTape {
    let temple_creates = std::array::from_fn(|_| {
        // oTemple.Create always consumes rand(1,100). Its treasure predicates
        // then short-circuit in source order for each interior rOlmec tile.
        let roll = (rng.range(100) + 1) as u8;
        let treasure = if city_of_gold || roll < 30 {
            None
        } else if rng.range(60) == 0 {
            Some(BossTempleTreasure::SapphireBig)
        } else if rng.range(80) == 0 {
            Some(BossTempleTreasure::EmeraldBig)
        } else if rng.range(100) == 0 {
            Some(BossTempleTreasure::RubyBig)
        } else if rng.range(1200) == 0 {
            Some(BossTempleTreasure::Underground(rng.range(19) as u8))
        } else {
            None
        };
        BossTempleCreate { roll, treasure }
    });
    let olmec_facing = if rng.range(2) == 0 { -1 } else { 1 };
    let lava_spurt_times = std::array::from_fn(|_| (100 + rng.range(201)) as u16);
    BossCreateTape {
        city_of_gold,
        has_spectacles,
        temple_creates,
        olmec_facing,
        lava_spurt_times,
    }
}

/// Birth rOlmec's authored Temple instances at the source room-entry boundary.
/// rOlmec.xml lists these before oGame, so their parent serials precede
/// scrInitLevel's boundaries and generated rooms.
pub(super) fn create_source_boss_temples(world: &mut World, rng: &mut Rng, tape: &BossCreateTape) {
    assert_eq!(world.area, Area::Boss);
    for (&position, create) in BOSS_TEMPLE_CREATE_ORDER.iter().zip(tape.temple_creates) {
        let position = usize::from(position);
        let x = 1 + position % 40;
        let y = 35 + position / 40;
        super::terrain::create_authored_boss_temple(
            world,
            rng,
            x,
            y,
            create,
            tape.city_of_gold,
            tape.has_spectacles,
        );
    }
}

// b = destructible brick, h = invincible hard block, l/L = ladder/top.
const TUTORIAL_TERRAIN: [&str; 19] = [
    "hhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhh",
    "h0000000000000000000000000000000000000000h",
    "h000000000000000000000000000000000000000lh",
    "h000000000000000000000000000000bbbbbbbbbLh",
    "h000000000000000000000000000000000000000lh",
    "hbb00000000000bbbbbbbbb00000000000000000lh",
    "bbb00000b00000bbbbbbbbb000000000000000000h",
    "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb0000bbbh",
    "hbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb0000bh",
    "hbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb00bh",
    "hbbb00000000b000000000000bbbbbbb000000000h",
    "hbb000000000b0000000000000000000000000000h",
    "hb0000000000b0000000000000bbbb00000000000h",
    "h00000000000b0000000000000bbbb00000000000h",
    "h00000000000bbbbb0000bbbb0bbbb00000000000h",
    "h0000000000bbbbbb000000000bbbb00000000000h",
    "h0bhb00000bbbbbbb00000000bbbbb00000000000h",
    "hbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbh",
    "hhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhhh",
];

// Object positions retain GameMaker sprite origins, not assumed tile centers.
const TUTORIAL_ENTITIES: &[(&str, f64, f64, &str)] = &[
    ("oXTitle", 16.0, 64.0, "sExit"),
    ("oExit", 48.0, 240.0, "sExit"),
    ("oMsgSign", 80.0, 96.0, "sMsgSign"),
    ("oMsgSign", 176.0, 96.0, "sMsgSign"),
    ("oMsgSign", 576.0, 128.0, "sMsgSign"),
    ("oMsgSign", 640.0, 96.0, "sMsgSign"),
    ("oMsgSign", 608.0, 256.0, "sMsgSign"),
    ("oMsgSign", 496.0, 256.0, "sMsgSign"),
    ("oMsgSign", 384.0, 208.0, "sMsgSign"),
    ("oMsgSign", 256.0, 208.0, "sMsgSign"),
    ("oMsgSign", 96.0, 256.0, "sMsgSign"),
    ("oMsgSign", 384.0, 96.0, "sMsgSign"),
    ("oMsgSign", 240.0, 64.0, "sMsgSign"),
    ("oMsgSign", 32.0, 64.0, "sMsgSign"),
    ("oMsgSign", 512.0, 32.0, "sMsgSign"),
    ("oMsgSign", 544.0, 112.0, "sMsgSign"),
    ("oMsgSign", 576.0, 256.0, "sMsgSign"),
    ("oMsgSign", 432.0, 176.0, "sMsgSign"),
    ("oMsgSign", 224.0, 208.0, "sMsgSign"),
    ("oCrate", 216.0, 216.0, "sCrate"),
    ("oChest", 504.0, 40.0, "sChest"),
    ("oEmeraldBig", 568.0, 44.0, "sEmeraldBig"),
    ("oSapphireBig", 552.0, 44.0, "sSapphireBig"),
    ("oRubyBig", 536.0, 44.0, "sRubyBig"),
    ("oRock", 648.0, 264.0, "sRock"),
    ("oGoldBar", 416.0, 108.0, "sGoldBar"),
    ("oGoldBar", 432.0, 108.0, "sGoldBar"),
    ("oGoldBar", 448.0, 108.0, "sGoldBar"),
    ("oGoldBar", 464.0, 108.0, "sGoldBar"),
    ("oGoldBar", 480.0, 108.0, "sGoldBar"),
    ("oGoldBar", 496.0, 108.0, "sGoldBar"),
    ("oGoldBars", 512.0, 104.0, "sGoldBars"),
    ("oSnake", 352.0, 64.0, "sSnakeLeft"),
    ("oBat", 480.0, 176.0, "sBatHang"),
];

/// Replace the normal layout only for the authored tutorial cave.
pub(super) fn generate_special(
    world: &mut World,
    _rng: &mut Rng,
    flags: &BTreeSet<String>,
) -> bool {
    if !flags.contains("tutorial") {
        return false;
    }
    *world = tutorial();
    true
}

fn tutorial() -> World {
    let mut world = World::new(42, 19, Area::Mines, 0);
    world.name = "Tutorial Cave".into();
    world.entrance = Vec2::new(24.0, 72.0);
    world.exit = Vec2::new(56.0, 248.0);
    for (y, row) in TUTORIAL_TERRAIN.iter().enumerate() {
        for (x, tile) in row.bytes().enumerate() {
            match tile {
                b'b' => world.set_tile(x, y, "oBrick"),
                b'h' => world.set_tile(x, y, "oHardBlock"),
                b'l' => world.set_tile(x, y, "oLadderOrange"),
                b'L' => world.set_tile(x, y, "oLadderTop"),
                _ => {}
            }
        }
    }
    for &(object, x, y, sprite) in TUTORIAL_ENTITIES {
        let id = world.spawn(object, Vec2::new(x, y));
        let entity = world.entity_mut(id).expect("new tutorial entity");
        entity.sprite = sprite.into();
        if object == "oMsgSign" {
            let (index, message, message2) = tutorial_sign(x as i32, y as i32);
            entity.data.insert("tutorial_sign".into(), index as f64);
            entity.text.insert("message".into(), message.into());
            entity.text.insert("message2".into(), message2.into());
        }
        if object == "oCrate" {
            // The tutorial explicitly teaches bombs at its single crate.
            entity.text.insert("contents".into(), "oBombBag".into());
        }
    }
    world
}

// The original sign wording is retained where independent of rebinding. Dynamic
// key names become semantic control names, so hints remain true for controllers.
fn tutorial_sign(x: i32, y: i32) -> (u32, &'static str, &'static str) {
    match (x, y) {
        (32, 64) => (
            0,
            "WELCOME TO THE TUTORIAL CAVE!",
            "THIS MIGHT BE YOUR FIRST TIME PLAYING.",
        ),
        (80, 96) => (1, "PRESS JUMP TO JUMP.", ""),
        (176, 96) => (2, "YOU CAN HANG ON LEDGES, TOO!", ""),
        (240, 64) => (3, "PRESS ATTACK TO USE YOUR WHIP.", ""),
        (384, 96) => (4, "COLLECT THE TREASURE!", ""),
        (512, 32) => (5, "HOLD UP AND PRESS ATTACK TO OPEN CHESTS.", ""),
        (544, 112) => (6, "HOLD DOWN TO DUCK AND CRAWL.", ""),
        (576, 128) => (
            7,
            "CRAWL OVER THE EDGE TO DO A FLIP HANG.",
            "FALLING TOO FAR CAN REALLY HURT!",
        ),
        (640, 96) => (8, "HOLD UP TO CLIMB THE LADDER.", ""),
        (608, 256) => (9, "DUCK AND PRESS ATTACK TO PICK UP ITEMS.", ""),
        (576, 256) => (
            10,
            "PRESS ATTACK TO THROW OR USE ITEMS.",
            "HOLD UP TO THROW HIGH AND DOWN TO DROP.",
        ),
        (496, 256) => (11, "SELECT YOUR ROPE AND PRESS ATTACK TO USE IT.", ""),
        (432, 176) => (
            12,
            "CROUCH AND PRESS ATTACK",
            "TO THROW A ROPE DOWN A LEDGE.",
        ),
        (384, 208) => (13, "TO RUN, HOLD DOWN ATTACK OR RUN.", ""),
        (256, 208) => (14, "OPEN THIS CRATE FOR A BAG OF BOMBS.", ""),
        (224, 208) => (15, "SELECT BOMBS AND PRESS ATTACK TO USE THEM.", ""),
        (96, 256) => (
            16,
            "THERE'S LOTS MORE AHEAD!",
            "USE YOUR WITS AND BEWARE OF TRAPS!",
        ),
        _ => (u32::MAX, "", ""),
    }
}

/// Overlay source-authored placements after the procedural room templates.
pub(super) fn apply(
    world: &mut World,
    rng: &mut Rng,
    flags: &BTreeSet<String>,
    revision: crate::GenerationRevision,
    boss_create_tape: Option<&BossCreateTape>,
) {
    if world.area == Area::Boss {
        boss_arena(world, rng, revision, boss_create_tape);
    }
    if flags.contains("city_of_gold") {
        // Tile identities remain oTemple; rendering chooses the golden temple
        // tileset from this level identity. Replacing them with oGold would
        // incorrectly change collision and destruction behavior.
        world.name = "City of Gold".into();
    }
    if flags.contains("cemetery") {
        for entity in &mut world.entities {
            entity.sprite = match entity.sprite.as_str() {
                "sTreeTop" => "sTreeTopDead".into(),
                "sLeaves" => "sLeavesDead".into(),
                "sLeavesRight" => "sLeavesDeadR".into(),
                "sTreeBranchLeft" => "sTreeBranchDeadL".into(),
                "sTreeBranchRight" => "sTreeBranchDeadR".into(),
                _ => continue,
            };
        }
    }
}

fn boss_arena(
    world: &mut World,
    rng: &mut Rng,
    revision: crate::GenerationRevision,
    boss_create_tape: Option<&BossCreateTape>,
) {
    world.name = "Olmec's Lair".into();
    world.entrance = Vec2::new(24.0, 552.0);
    world.exit = Vec2::new(648.0, 552.0);
    // The four underground rooms start at y=592, leaving a 176px arena
    // between the last upper rooms and the authored two-tile-thick floor.
    for y in 25..35 {
        for x in 1..41 {
            world.set_tile(x, y, "");
        }
    }
    world.entities.retain(|entity| {
        !matches!(
            entity.object.as_str(),
            "oEntrance"
                | "oExit"
                | "oOlmec"
                | "oFinalBoss"
                | "oCavemanWorship"
                | "oHawkmanWorship"
                | "oLava"
                | "oLavaSolid"
        )
    });
    if boss_create_tape.is_none() {
        for y in [35, 36] {
            for x in 1..41 {
                world.set_tile(x, y, "oTemple");
            }
        }
    }
    for y in 48..54 {
        for x in 1..41 {
            world.set_tile(x, y, "");
        }
    }
    // SourceClassic follows rOlmec.xml's authored instance order so its timer
    // tape remains attached to the right positions. SetupWalls later mutates
    // surface sprite masks, so its exact-origin probes also need this order.
    if let Some(tape) = boss_create_tape {
        assert_eq!(revision, crate::GenerationRevision::SourceClassicV2);
        for (birth, &position) in BOSS_LAVA_CREATE_ORDER.iter().enumerate() {
            let position = usize::from(position);
            let x = 1 + position % 40;
            let y = 48 + position / 40;
            let id = world.spawn("oLava", Vec2::new(x as f64 * 16.0, y as f64 * 16.0));
            // create_generated later completes the normal lava Create. The
            // preset timer prevents it from drawing again and keeps each draw
            // attached to the matching source instance.
            world
                .entity_mut(id)
                .expect("new authored lava")
                .data
                .insert("spurt_time".into(), f64::from(tape.lava_spurt_times[birth]));
        }
    } else {
        // HistoricalV1 retains its original producer order. This also keeps
        // direct legacy special-room fixtures independent of the source tape.
        for x in 1..41 {
            for y in (48..54).rev() {
                world.spawn("oLava", Vec2::new(x as f64 * 16.0, y as f64 * 16.0));
            }
        }
    }
    for x in 1..41 {
        world.set_tile(x, 54, "oLavaSolid");
    }
    for (object, x, y, sprite) in [
        ("oEntrance", 16.0, 544.0, "sEntrance"),
        ("oEntrance", 640.0, 544.0, "sEntrance"),
        ("oOlmec", 304.0, 496.0, "sOlmecStart1"),
        ("oCavemanWorship", 264.0, 544.0, "sCavemanWorshipR"),
        ("oCavemanWorship", 248.0, 544.0, "sCavemanWorshipR"),
        ("oCavemanWorship", 232.0, 544.0, "sCavemanWorshipR"),
        ("oHawkmanWorship", 280.0, 544.0, "sHawkLeft"),
    ] {
        let id = if object == "oOlmec" && revision == crate::GenerationRevision::SourceClassicV2 {
            if let Some(tape) = boss_create_tape {
                // Reuse the authored Create draw made before route generation.
                let id = world.spawn(object, Vec2::new(x, y));
                let entity = world.entity_mut(id).expect("new authored Olmec");
                entity.facing = tape.olmec_facing;
                entity.data.insert("olmec_facing_created".into(), 1.0);
                id
            } else {
                // Preserve standalone arena fixtures that do not run the
                // level-generation preamble.
                crate::creation::spawn_generated_entity(world, rng, object, Vec2::new(x, y))
            }
        } else {
            world.spawn(object, Vec2::new(x, y))
        };
        let entity = world.entity_mut(id).expect("new authored boss entity");
        entity.sprite = sprite.into();
        if object == "oEntrance" && x == 640.0 {
            entity.data.insert("boss_exit".into(), 1.0);
        }
    }
    // rOlmec instance128448: its Step opens the end portal after Olmec dies.
    // It remains an actual resource580 controller, after Olmec559.
    world.spawn("oFinalBoss", Vec2::new(48., 0.));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tutorial_preserves_authored_terrain_and_interactive_placements() {
        let world = tutorial();
        assert_eq!((world.width, world.height), (42, 19));
        assert_eq!(world.entrance, Vec2::new(24.0, 72.0));
        assert_eq!(world.exit, Vec2::new(56.0, 248.0));
        assert_eq!(
            world
                .tiles
                .iter()
                .filter(|tile| tile.object == "oBrick")
                .count(),
            246
        );
        assert_eq!(
            world
                .tiles
                .iter()
                .filter(|tile| tile.object == "oHardBlock")
                .count(),
            117
        );
        assert_eq!(
            world
                .entities
                .iter()
                .filter(|entity| entity.object == "oMsgSign")
                .count(),
            17
        );
        assert!(
            world
                .entities
                .iter()
                .filter(|entity| entity.object == "oMsgSign")
                .all(|entity| !entity.text["message"].is_empty())
        );
        assert_eq!(
            world
                .entities
                .iter()
                .filter(|entity| entity.object == "oGoldBar")
                .count(),
            6
        );
        assert!(
            world
                .entities
                .iter()
                .any(|entity| entity.object == "oSnake" && entity.pos == Vec2::new(352.0, 64.0))
        );
    }

    #[test]
    fn boss_uses_deep_authored_arena_and_a_sealed_victory_door() {
        let mut world = World::new(42, 55, Area::Boss, 16);
        boss_arena(
            &mut world,
            &mut Rng::new(0),
            crate::GenerationRevision::SourceClassicV2,
            None,
        );
        assert_eq!(world.entrance, Vec2::new(24.0, 552.0));
        assert_eq!(
            world
                .entities
                .iter()
                .filter(|entity| entity.object == "oLava")
                .count(),
            240
        );
        assert_eq!(
            world
                .tiles
                .iter()
                .filter(|tile| tile.object == "oTemple")
                .count(),
            80
        );
        assert_eq!(
            world
                .tiles
                .iter()
                .filter(|tile| tile.object == "oLavaSolid")
                .count(),
            40
        );
        assert!(
            world
                .entities
                .iter()
                .any(|entity| entity.object == "oOlmec" && entity.pos == Vec2::new(304.0, 496.0))
        );
        assert!(!world.entities.iter().any(|entity| entity.object == "oExit"));
        assert_eq!(
            world
                .entities
                .iter()
                .filter(|e| e.object == "oFinalBoss" && e.pos == Vec2::new(48., 0.))
                .count(),
            1
        );
        assert!(
            world
                .entities
                .iter()
                .any(|entity| entity.value("boss_exit") == 1.0)
        );
    }

    #[test]
    fn authored_temple_underground_item_reuses_its_recorded_choice_without_later_draws() {
        let mut rng = Rng::new(32);
        let tape = create_source_boss_tape(&mut rng, false, false);
        assert_eq!(
            tape.temple_creates[78].treasure,
            Some(BossTempleTreasure::Underground(6))
        );
        let mut world = World::new(42, 55, Area::Boss, 16);
        super::create_source_boss_temples(&mut world, &mut rng, &tape);
        let state_after_source_creates = rng.state_bits();
        boss_arena(
            &mut world,
            &mut rng,
            crate::GenerationRevision::SourceClassicV2,
            Some(&tape),
        );
        assert_eq!(rng.state_bits(), state_after_source_creates);
        let entities_before_terrain = world.entities.len();
        super::super::terrain::create_generated_with_context(
            &mut world,
            &mut rng,
            super::super::GenerationContext::default(),
        );
        assert_eq!(
            rng.state_bits(),
            state_after_source_creates,
            "the later terrain pass must not rerun authored Temple or Lava Create draws"
        );
        assert_eq!(
            world.entities.len(),
            entities_before_terrain,
            "the later terrain pass must not duplicate authored Temple treasure"
        );
        let spectacles = world
            .entities
            .iter()
            .find(|entity| entity.object == "oSpectacles")
            .expect("recorded Underground-set Temple item is born");
        assert_eq!(spectacles.pos, Vec2::new(72.0, 584.0));
        assert_eq!(spectacles.value("cost"), 0.0);
        assert_eq!(spectacles.value("forSale"), 0.0);
    }

    #[test]
    fn authored_olmec_completes_one_facing_draw_before_following_births() {
        for seed in 0..16 {
            for revision in [
                crate::GenerationRevision::SourceClassicV2,
                crate::GenerationRevision::HistoricalV1,
            ] {
                let mut world = World::new(42, 55, Area::Boss, 16);
                let mut actual = Rng::new(seed);
                let mut expected = actual.clone();
                let facing = if revision == crate::GenerationRevision::SourceClassicV2 {
                    if expected.range(2) == 0 { -1 } else { 1 }
                } else {
                    1
                };
                boss_arena(&mut world, &mut actual, revision, None);
                let boss = world
                    .entities
                    .iter()
                    .find(|e| e.object == "oOlmec")
                    .unwrap();
                assert_eq!(boss.pos, Vec2::new(304., 496.));
                assert_eq!(boss.sprite, "sOlmecStart1");
                assert_eq!(boss.facing, facing);
                assert_eq!(
                    boss.value("olmec_facing_created"),
                    if revision == crate::GenerationRevision::SourceClassicV2 {
                        1.
                    } else {
                        0.
                    }
                );
                let worship = world
                    .entities
                    .iter()
                    .find(|e| e.object == "oCavemanWorship")
                    .unwrap();
                assert!(boss.creation_serial < worship.creation_serial);
                assert_eq!(actual.state_bits(), expected.state_bits());
                assert_eq!(actual.next_u64(), expected.next_u64());
            }
        }
    }
}
