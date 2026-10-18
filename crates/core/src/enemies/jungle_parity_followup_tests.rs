//! Follow-up source contracts. Direct state/velocity inputs below are explicit
//! statement controls; the Monkey fixture reaches CLIMB through ordinary steps.
use super::*;
use crate::{InputFrame, Rng, RulesPreset};

fn actor(object: &str, pos: Vec2) -> Entity {
    let mut actor = Entity::new(900, object, pos);
    super::super::init_entity(&mut actor);
    actor
}

fn water_room(seed: u32) -> Game {
    let mut game = Game::shell(seed, RulesPreset::Classic);
    game.world = World::new(24, 20, Area::Jungle, 5);
    game.player.pos = Vec2::new(200.0, 152.0);
    for y in 2..10 {
        for x in 2..12 {
            game.world.set_tile(x, y, "oWaterSwim");
        }
    }
    game
}

#[test]
fn jungle_followup_actual_monkey_climb_end_requires_vine_after_natural_hang() {
    for ladder in ["oRope", "oVine"] {
        let mut game = Game::shell(17, RulesPreset::Classic);
        game.world = World::new(24, 20, Area::Jungle, 5);
        game.player.pos = Vec2::new(200.0, 152.0);
        // Authored ladder fixture, not a claimed player rope-throw trajectory.
        game.world.set_tile(5, 5, ladder);
        let monkey = game.spawn("oMonkey", Vec2::new(80.0, 80.0));
        assert_eq!(game.world.entity(monkey).unwrap().state, 5);
        game.step(&InputFrame::default());
        assert_eq!(game.world.entity(monkey).unwrap().state, 6);
        game.step(&InputFrame::default());
        let monkey = game.world.entity(monkey).unwrap();
        if ladder == "oVine" {
            assert_eq!(monkey.state, 6);
            assert_eq!(monkey.value("dir"), 0.0);
            assert_eq!(monkey.vel.y, -1.0);
        } else {
            assert_eq!(monkey.state, 5);
            assert_eq!(monkey.value("dir"), 1.0);
            assert!((10..=40).contains(&monkey.timer));
        }
    }
}

#[test]
fn jungle_followup_walk_reverses_when_last_move_pixel_reaches_wall() {
    for object in ["oMonkey", "oManTrap"] {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.world = World::new(24, 20, Area::Jungle, 5);
        game.player.pos = Vec2::new(200.0, 152.0);
        for x in 0..24 {
            game.world.set_tile(x, 10, "oBlock");
        }
        game.world.set_tile(8, 9, "oBlock");
        // Explicit WALK entry and incoming speed isolate source postmove probes.
        // Both consume all requested pixels without attempting a blocked pixel.
        let (origin, speed, end_x) = if object == "oMonkey" {
            (114.0, 2.0, 116.0)
        } else {
            (113.0, 1.0, 114.0)
        };
        let mut walker = actor(object, Vec2::new(origin, 144.0));
        walker.state = 3;
        if object == "oManTrap" {
            walker.state = 1;
        }
        walker.facing = 1;
        walker.vel = Vec2::new(speed, 0.0);
        let mut source_rng = game.rng.clone();
        assert_ne!(source_rng.range(100), 0, "fixture excludes idle roll");
        if object == "oManTrap" {
            source_rng.range(8); // Source eager sleep/bone condition tail.
        }
        tick(&mut game, &mut walker);
        assert_eq!(walker.pos.x, end_x, "{object}");
        assert_eq!(walker.facing, -1, "{object}");
        assert_eq!(walker.vel.x, if object == "oMonkey" { -2.0 } else { -0.3 });
        assert_eq!(game.rng.state_bits(), source_rng.state_bits(), "{object}");
    }
}

#[test]
fn jungle_followup_piranha_pause_uses_source_roll_direction_and_one_draw() {
    let mut seen = [false; 2];
    for seed in [0, 1, 17, 31, 100, 12345, u32::MAX] {
        let mut game = water_room(seed);
        let mut fish = actor("oPiranha", Vec2::new(80.0, 80.0));
        // Explicit PAUSE/counter0 statement boundary, not natural attack loss.
        fish.state = 2;
        fish.timer = 0;
        fish.data.insert("bubble_timer".into(), 10.0);
        let mut source_rng = game.rng.clone();
        let roll = source_rng.range(2);
        seen[roll as usize] = true;
        tick(&mut game, &mut fish);
        assert_eq!(fish.state, 0);
        assert_eq!(fish.facing, if roll == 0 { 1 } else { -1 });
        assert_eq!(
            fish.sprite,
            if roll == 0 {
                "sPiranhaRight"
            } else {
                "sPiranhaLeft"
            }
        );
        assert_eq!(game.rng.state_bits(), source_rng.state_bits());
    }
    assert!(seen.into_iter().all(|branch| branch));
}

#[test]
fn jungle_followup_piranha_vertical_aim_uses_final_source_angle_and_ordered_draws() {
    let mut seen = [false; 3];
    for seed in 0..32 {
        for up in [true, false] {
            let mut game = water_room(seed);
            let mut fish = actor("oPiranha", Vec2::new(80.0, 80.0));
            let origin = center(&fish);
            let target = Vec2::new(origin.x, origin.y + if up { -32.0 } else { 32.0 });
            let mut source_rng = Rng::new(seed);
            let first = source_rng.range(2) as i32;
            let second = source_rng.range(2) as i32;
            let jitter = first - second;
            seen[(jitter + 1) as usize] = true;
            aim_fish(&mut game, &mut fish, target);
            // Independent vertical source values: point_direction is90/270,
            // x=cos(dir), screen y=-sin(dir), left iff90<dir<270.
            let offset = f64::from(jitter).to_radians();
            let expected_x = if up { -offset.sin() } else { offset.sin() };
            let expected_y = if up { -offset.cos() } else { offset.cos() };
            assert!((fish.vel.x - expected_x).abs() < 1e-12);
            assert!((fish.vel.y - expected_y).abs() < 1e-12);
            assert_eq!(
                fish.facing,
                if (up && jitter > 0) || (!up && jitter < 0) {
                    -1
                } else {
                    1
                }
            );
            assert_eq!(game.rng.state_bits(), source_rng.state_bits());
        }
    }
    assert!(seen.into_iter().all(|branch| branch));
}

#[test]
fn jungle_followup_piranha_npc_attack_water_probe_does_not_invent_solid_pause() {
    let mut game = water_room(17);
    let victim = game.world.spawn("oCaveman", Vec2::new(104.0, 80.0));
    let victim = game.world.entity_mut(victim).unwrap();
    victim.health = 3;
    victim.data.insert("swimming".into(), 1.0);
    // Authored overlap control: source permits Water+Solid at the probe;
    // moveTo must still resolve body collision, without choosing PAUSE.
    game.world.spawn("oBlock", Vec2::new(80.0, 80.0));
    let mut fish = actor("oPiranha", Vec2::new(80.0, 80.0));
    fish.state = 3;
    fish.data.insert("bubble_timer".into(), 10.0);
    let mut source_rng = game.rng.clone();
    source_rng.range(2);
    source_rng.range(2);
    tick(&mut game, &mut fish);
    assert_eq!(fish.state, 3);
    assert_eq!(fish.timer, 0);
    assert_eq!(fish.pos, Vec2::new(80.0, 80.0));
    assert_eq!(game.rng.state_bits(), source_rng.state_bits());
}

#[test]
fn jungle_followup_piranha_nearest_tie_uses_source_family_order_after_storage_reorder() {
    let mut game = Game::shell(23, RulesPreset::Classic);
    game.world = World::new(24, 20, Area::Jungle, 5);
    game.player.pos = Vec2::new(304.0, 128.0);
    for x in 4..=8 {
        game.world.set_tile(x, 5, "oWaterSwim");
        game.world.set_tile(x, 6, "oBlock");
    }

    // Equal-distance Cavemen expose which active-family member
    // instance_nearest returns. Register them in Classic creation order.
    let first = game.world.spawn("oCaveman", Vec2::new(96.0, 80.0));
    let second = game.world.spawn("oCaveman", Vec2::new(96.0, 80.0));
    let fish = game.world.spawn("oPiranha", Vec2::new(96.0, 80.0));
    game.step(&InputFrame::default());

    for id in [first, second] {
        let caveman = game.world.entity_mut(id).unwrap();
        caveman.state = 0;
        caveman.vel = Vec2::default();
        caveman.data.insert("counter".into(), 100.0);
        caveman.data.insert("sight_counter".into(), 100.0);
    }
    let fish_actor = game.world.entity_mut(fish).unwrap();
    fish_actor.state = 0;
    fish_actor.data.insert("bubble_timer".into(), 100.0);
    assert_eq!(
        crate::dispatcher::descendant_instances(&game, "oCaveman"),
        [
            crate::dispatcher::Instance::Entity(first),
            crate::dispatcher::Instance::Entity(second)
        ]
    );

    // Engine instance arrays retain their registration order while physical
    // storage can be compacted/reordered independently.
    game.world.entities.reverse();
    game.step(&InputFrame::default());

    let fish_actor = game.world.entity(fish).unwrap();
    assert_eq!(fish_actor.state, 3);
    assert_eq!(fish_actor.value("fish_target") as u64, first);
}
