use super::*;
use crate::{Area, InputFrame, PlayerState, Rng, RulesPreset, World};

fn room(seed: u32) -> Game {
    let mut game = Game::shell(seed, RulesPreset::Classic);
    game.world = World::new(20, 16, Area::Temple, 13);
    game.flags.insert("custom".into());
    game.player.pos = Vec2::new(40.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    for x in 0..20 {
        game.world.set_tile(x, 10, "oBrick");
    }
    game.world.fluid_dirty = false;
    game
}

#[test]
fn actual_step_replaces_uncovered_fake_with_new_dynamic_temple() {
    let mut game = room(7);
    let origin = Vec2::new(80.0, 80.0);
    let fake = game.spawn("oTempleFake", origin);
    let fake_serial = game.world.entity(fake).unwrap().creation_serial;
    game.step(&InputFrame::default());
    assert!(game.world.entity(fake).is_none());
    let temple = game
        .world
        .entities
        .iter()
        .find(|e| e.alive && e.object == "oTemple" && e.pos == origin)
        .unwrap();
    assert_ne!(
        temple.id, fake,
        "source creates a new identity, not instance_change"
    );
    assert!(temple.creation_serial > fake_serial);
    assert_eq!(
        temple.value("runtime_temple_created"),
        1.0,
        "Create completed synchronously"
    );
    assert!(game.world.solid_at(88.0, 88.0));
    assert!(
        !game.world.fluid_dirty,
        "Fake has no parent/Destroy; Temple Create requests no water"
    );
    let id = temple.id;
    game.step(&InputFrame::default());
    assert!(game.world.entity(id).is_some());
    assert_eq!(
        game.world
            .entities
            .iter()
            .filter(|e| e.alive && e.object == "oTemple" && e.pos == origin)
            .count(),
        1
    );
}

#[test]
fn actual_falling_door_exposes_both_source_fake_cells() {
    let mut game = room(7);
    let door = game.spawn("oDoor", Vec2::new(80.0, 64.0));
    let top = game.spawn("oTempleFake", Vec2::new(80.0, 64.0));
    let bottom = game.spawn("oTempleFake", Vec2::new(80.0, 80.0));
    game.step(&InputFrame::default());
    assert!(
        game.world.entity(top).is_some() && game.world.entity(bottom).is_some(),
        "idle source door masks cover both fake centers"
    );
    game.world.entity_mut(door).unwrap().state = 1; // source DROP state
    for _ in 0..30 {
        game.step(&InputFrame::default());
    }
    assert!(game.world.entity(top).is_none() && game.world.entity(bottom).is_none());
    for y in [64.0, 80.0] {
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == "oTemple" && e.pos == Vec2::new(80.0, y))
                .count(),
            1
        );
    }
}

#[test]
fn fake_step_door_probe_excludes_inactive_masks_before_storage_partition() {
    let source = include_str!(
        "../../../../references/classic-1.1/Objects/Blocks/oTempleFake.events/Step.xml"
    );
    assert!(source.contains("collision_point(x+8, y+8, oDoor, 0, 0)"));
    for scheduled in [false, true] {
        for terrain in [false, true] {
            for active in [false, true] {
                let mut game = room(7);
                let origin = Vec2::new(80., 80.);
                if terrain {
                    game.world.set_tile(5, 5, "oDoor");
                    let width = game.world.width;
                    game.world.tiles[5 * width + 5].engine_active = active;
                    assert!(
                        game.world
                            .tile_collision_mask(5, 5)
                            .contains(Vec2::new(88., 88.), crate::CollisionPrecision::BoundingBox)
                    );
                } else {
                    let door = game.spawn("oDoor", origin);
                    game.world.entity_mut(door).unwrap().engine_active = active;
                }
                let fake = game.spawn("oTempleFake", origin);
                if scheduled {
                    game.step(&InputFrame::default());
                } else {
                    // Enter the real Step with storage still restored. The
                    // collision predicate must enforce activity itself.
                    crate::world_events::step_instance(&mut game, fake);
                }
                assert_eq!(
                    game.world.entity(fake).is_some(),
                    active,
                    "scheduled={scheduled}, terrain={terrain}, active={active}"
                );
                assert_eq!(
                    game.world
                        .entities
                        .iter()
                        .any(|e| { e.alive && e.object == "oTemple" && e.pos == origin }),
                    !active,
                    "replacement: scheduled={scheduled}, terrain={terrain}, active={active}"
                );
            }
        }
    }
}

#[test]
fn active_terrain_door_requires_a_source_collision_mask() {
    for scheduled in [false, true] {
        let mut game = room(7);
        let origin = Vec2::new(80., 80.);
        game.world.set_tile(5, 5, "oDoor");
        let metadata = game.world.spawn("oTerrainMetadata", origin);
        game.world
            .entity_mut(metadata)
            .unwrap()
            .text
            .insert("terrain_sprite".into(), String::new());
        assert!(
            !game
                .world
                .tile_collision_mask(5, 5)
                .contains(Vec2::new(88., 88.), crate::CollisionPrecision::BoundingBox)
        );
        let fake = game.spawn("oTempleFake", origin);
        if scheduled {
            game.step(&InputFrame::default());
        } else {
            crate::world_events::step_instance(&mut game, fake);
        }
        assert!(game.world.entity(fake).is_none(), "scheduled={scheduled}");
        assert!(
            game.world
                .entities
                .iter()
                .any(|e| { e.alive && e.object == "oTemple" && e.pos == origin })
        );
    }
}

#[test]
fn runtime_temple_create_keeps_parent_defaults_and_city_draw_before_any_branch() {
    for seed in 1..=128 {
        for city in [false, true] {
            let mut game = room(seed);
            if city {
                game.flags.insert("cityOfGold".into());
            }
            let mut expected = game.rng.clone();
            let roll = expected.range(100) + 1;
            // Boundary origin excludes treasure, but still consumes n before
            // both the city branch and gold/non-gold branch selection.
            let id = game.spawn("oTemple", Vec2::new(0.0, 80.0));
            let e = game.world.entity(id).unwrap();
            let sprite = if city {
                "sGTemple"
            } else if roll < 20 {
                "sTempleGold"
            } else if roll < 30 {
                "sTempleGoldBig"
            } else {
                "sTemple"
            };
            assert_eq!(e.sprite, sprite);
            assert_eq!(e.value("n"), f64::from(roll));
            assert_eq!(e.value("cleanDeath"), 0.0);
            assert_eq!(e.value("invincible"), 0.0);
            assert_eq!(e.value("shopWall"), 0.0);
            assert_eq!(e.text.get("type").map(String::as_str), Some("None"));
            assert_eq!(e.text.get("treasure").map(String::as_str), Some(""));
            assert_eq!(game.rng.clone().next_u64(), expected.next_u64());
            let before = game.rng.clone().next_u64();
            super::create(&mut game, id);
            assert_eq!(
                game.rng.clone().next_u64(),
                before,
                "same birth must not repeat Create"
            );
        }
    }
}

#[test]
fn runtime_temple_each_gem_is_born_during_create_at_source_center() {
    for (branch, object, value) in [
        (0, "oSapphireBig", 1200.0),
        (1, "oEmeraldBig", 800.0),
        (2, "oRubyBig", 1600.0),
    ] {
        let seed = (1..100_000)
            .find(|&seed| {
                let mut r = Rng::new(seed);
                if r.range(100) + 1 < 30 {
                    return false;
                }
                for (index, denominator) in [60, 80, 100].into_iter().enumerate() {
                    if r.chance(1, denominator) {
                        return index == branch;
                    }
                }
                false
            })
            .expect("bounded fixture must reach each literal gem branch");
        let mut game = room(seed);
        let mut expected = game.rng.clone();
        expected.range(100);
        for (index, denominator) in [60, 80, 100].into_iter().enumerate() {
            let chosen = expected.chance(1, denominator);
            assert_eq!(chosen, index == branch);
            if chosen {
                break;
            }
        }
        let temple = game.spawn("oTemple", Vec2::new(80.0, 80.0));
        let child = game
            .world
            .entities
            .iter()
            .find(|e| e.alive && e.object == object)
            .unwrap();
        assert_eq!(child.pos, Vec2::new(88.0, 88.0));
        assert!(child.creation_serial > game.world.entity(temple).unwrap().creation_serial);
        assert_eq!(child.value("value"), value);
        assert_eq!(child.value("canCollect"), 0.0);
        assert_eq!(
            child.alarms[0], 20,
            "nested gem Create completed before return"
        );
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == object)
                .count(),
            1
        );
        assert!(
            game.world.solid_at(child.pos.x, child.pos.y),
            "treasure exists inside live Temple, not deferred to Destroy"
        );
        assert_eq!(game.rng.clone().next_u64(), expected.next_u64());
    }
}

#[test]
fn temple_treasure_rolls_require_level_and_strict_interior_edges() {
    let seed = (1..1024)
        .find(|&seed| Rng::new(seed).range(100) + 1 >= 30)
        .unwrap();
    for (origin, hub) in [
        (Vec2::new(1.0, 80.0), false),
        (Vec2::new(80.0, 1.0), false),
        (Vec2::new(304.0, 80.0), false),
        (Vec2::new(80.0, 240.0), false),
        (Vec2::new(80.0, 80.0), true),
    ] {
        let mut game = room(seed);
        if hub {
            game.flags.insert("hub".into());
        }
        let mut expected = game.rng.clone();
        expected.range(100);
        let id = game.spawn("oTemple", origin);
        assert_eq!(game.world.entity(id).unwrap().sprite, "sTemple");
        assert_eq!(
            game.rng.clone().next_u64(),
            expected.next_u64(),
            "no gem/item random draw outside the literal gate"
        );
        assert_eq!(game.world.entities.len(), 1);
    }
}

// Independent source fixture: parse the preserved GML switch rather than
// treating the production lookup table as its own expected answer.
fn source_underground_cases() -> Vec<(&'static str, f64, f64)> {
    let source = include_str!(
        "../../../../references/classic-1.1/Scripts/Level Generation/scrGenerateItem.gml"
    );
    let underground = source.split("else if (argument2 == 2)").nth(1).unwrap();
    let mut cases = Vec::new();
    for line in underground
        .lines()
        .filter(|line| line.trim().starts_with("case "))
    {
        let args = line
            .split("instance_create(")
            .nth(1)
            .unwrap()
            .split(')')
            .next()
            .unwrap();
        let mut args = args.split(',').map(str::trim);
        let x = args.next().unwrap().strip_prefix("argument0").unwrap();
        let y = args.next().unwrap().strip_prefix("argument1").unwrap();
        let object = args.next().unwrap();
        let offset = |suffix: &str| {
            if suffix.is_empty() {
                0.0
            } else {
                suffix.parse().unwrap()
            }
        };
        cases.push((object, offset(x), offset(y)));
    }
    assert_eq!(cases.len(), 19);
    cases
}

#[test]
fn underground_switch_preserves_all_source_offsets_and_post_create_sale_overrides() {
    let source_cases = source_underground_cases();
    let mut seen = [false; 19];
    for seed in 1..=512 {
        let mut game = room(seed);
        let mut expected = game.rng.clone();
        let index = expected.range(19) as usize;
        if seen[index] {
            continue;
        }
        seen[index] = true;
        let (object, dx, dy) = source_cases[index];
        let position = Vec2::new(88.0 + dx, 88.0 + dy);
        let mut source_control = room(seed);
        source_control.rng = expected;
        let expected_id = source_control.spawn(object, position);
        super::underground(&mut game, Vec2::new(88.0, 88.0));
        let actual = game
            .world
            .entities
            .iter()
            .find(|e| e.alive && e.object == object)
            .unwrap();
        assert_eq!(actual.pos, position, "switch case {index}");
        assert_eq!(actual.value("cost"), 0.0);
        assert_eq!(actual.value("forSale"), 0.0);
        assert_eq!(
            actual.sprite,
            source_control.world.entity(expected_id).unwrap().sprite,
            "normal synchronous item Create must run before producer overrides"
        );
        assert_eq!(
            game.rng.clone().next_u64(),
            source_control.rng.clone().next_u64()
        );
    }
    assert!(
        seen.into_iter().all(|case| case),
        "all19 source switch branches must be exercised"
    );
}
