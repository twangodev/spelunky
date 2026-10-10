//! Authored Classic room placement, transcribed from the distributed 1.1 XML.
//!
//! rTutorial.xml is 672x304; rOlmec.xml is 672x880. The Olmec room
//! combines generated upper/underground rooms with this authored arena, floor,
//! worshippers, and lava basin. No room-generation randomness is consumed here.
//! Copyright (c) 2008, 2009 Derek Yu and Mossmouth, LLC.
//! Source-derived layout is covered by the Spelunky User License; see COPYING.txt.

use crate::model::{Area, Rng, Vec2, World};
use std::collections::BTreeSet;

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
pub(super) fn apply(world: &mut World, _rng: &mut Rng, flags: &BTreeSet<String>) {
    if world.area == Area::Boss {
        boss_arena(world);
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

fn boss_arena(world: &mut World) {
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
    for y in [35, 36] {
        for x in 1..41 {
            world.set_tile(x, y, "oTemple");
        }
    }
    for y in 48..54 {
        for x in 1..41 {
            world.set_tile(x, y, "");
        }
    }
    // rOlmec instances 127721 onward are authored column by column, bottom
    // first. Preserve their order: SetupWalls mutates surface sprite masks,
    // and its exact-origin probes must see untouched lava above each layer.
    for x in 1..41 {
        for y in (48..54).rev() {
            world.spawn("oLava", Vec2::new(x as f64 * 16.0, y as f64 * 16.0));
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
        let id = world.spawn(object, Vec2::new(x, y));
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
        boss_arena(&mut world);
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
}
