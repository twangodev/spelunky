//! Isolated own-Step oracle; controlled initial states are not natural inputs.
use super::*;
use crate::{Area, RulesPreset, World};

fn room() -> Game {
    let mut game = Game::shell(42, RulesPreset::Classic);
    game.world = World::new(30, 20, Area::Ice, 9);
    game.player.pos = Vec2::new(240.0, 152.0);
    game.player.previous_pos = game.player.pos;
    for x in 0..30 {
        game.world.set_tile(x, 10, "oBlock");
    }
    game
}

fn literal_blood(rng: &mut crate::Rng) -> (Vec2, f64) {
    let x = rng.unit() * 4.0 - rng.unit() * 4.0;
    let y = -1.0 - rng.unit() * 2.0;
    let gravity = f64::from(1 + rng.range(6)) * 0.1;
    (Vec2::new(x, y), gravity)
}

#[test]
fn yeti_first_supported_stun_bleeds_before_recovery_with_literal_caps_and_rng() {
    for bloodless in [0.0, 1.0, -1.0] {
        for count in [0, 15, 16] {
            for bounced in [false, true] {
                let mut game = room();
                for _ in 0..count {
                    game.spawn("oBone", Vec2::new(240.0, 80.0));
                }
                let id = game.spawn("oYeti", Vec2::new(96.0, 144.0));
                let mut actor = game.world.entity(id).unwrap().clone();
                actor.state = 98;
                actor.health = 1;
                actor.timer = 0;
                actor.vel = Vec2::default();
                actor.data.insert("counter".into(), 37.0);
                actor.data.insert("bloodless".into(), bloodless);
                actor.data.insert("bounced".into(), f64::from(bounced));
                let admitted = !bounced && bloodless == 0.0 && count < 16;
                let next_id = game.world.next_id;
                let mut expected = game.rng.clone();
                let expected_blood = admitted.then(|| literal_blood(&mut expected));
                yeti(&mut game, &mut actor);
                let drops: Vec<_> = game
                    .world
                    .entities
                    .iter()
                    .filter(|e| e.id >= next_id && e.object == "oBlood")
                    .collect();
                assert_eq!(drops.len(), usize::from(admitted));
                if let Some((velocity, gravity)) = expected_blood {
                    assert_eq!(drops[0].pos, Vec2::new(104.0, 152.0));
                    assert_eq!(drops[0].vel, velocity);
                    assert_eq!(drops[0].value("gravity"), gravity);
                    assert_eq!(&drops[0].alarms[..3], &[1, 1, 5]);
                    assert_eq!(drops[0].value("bounce"), 0.0);
                    assert!(drops[0].alive);
                }
                assert_eq!(actor.value("bounced"), 1.0, "latch precedes script gates");
                assert_eq!(
                    actor.state, 0,
                    "same own Step recovers after the landing producer"
                );
                assert_eq!(actor.timer, 0);
                assert_eq!(
                    actor.value("counter"),
                    0.0,
                    "source shared counter is exhausted"
                );
                assert_eq!(game.rng.clone().next_u64(), expected.next_u64());
                assert!(
                    !game
                        .events
                        .iter()
                        .any(|e| matches!(e, GameEvent::Sound { .. })),
                    "the source landing producer itself has no sound"
                );
            }
        }
    }
}

#[test]
fn yeti_zero_counter_recovers_on_following_supported_tick_then_walks_without_stale_pause() {
    let mut game = room();
    let id = game.spawn("oYeti", Vec2::new(96.0, 144.0));
    let mut actor = game.world.entity(id).unwrap().clone();
    actor.state = 98;
    actor.health = 1;
    actor.timer = 1;
    actor.vel = Vec2::default();
    actor.data.insert("counter".into(), 37.0);
    actor.data.insert("bounced".into(), 1.0);
    let mut expected = game.rng.clone();
    yeti(&mut game, &mut actor);
    assert_eq!(
        (actor.state, actor.timer),
        (98, 0),
        "decrement does not recover early"
    );
    assert_eq!(game.rng.clone().next_u64(), expected.clone().next_u64());
    yeti(&mut game, &mut actor);
    assert_eq!((actor.state, actor.timer), (0, 0));
    assert_eq!(actor.value("counter"), 0.0);
    assert_eq!(
        actor.value("bounced"),
        1.0,
        "IDLE body has not run on recovery tick"
    );
    assert_eq!(game.rng.clone().next_u64(), expected.clone().next_u64());
    let facing = if expected.range(2) == 0 { -1 } else { 1 };
    yeti(&mut game, &mut actor);
    assert_eq!(
        actor.state, 1,
        "the next IDLE body transitions with no stale idle pause"
    );
    assert_eq!(actor.facing, facing);
    assert_eq!(actor.value("counter"), 0.0);
    assert_eq!(
        actor.value("bounced"),
        0.0,
        "source IDLE resets the next-stun latch"
    );
    assert_eq!(game.rng.clone().next_u64(), expected.next_u64());
    assert!(!game.world.entities.iter().any(|e| e.object == "oBlood"));
}

#[test]
fn yeti_carried_airborne_stun_counts_down_without_landing_blood_and_clears_hand() {
    let mut game = room();
    let id = game.spawn("oYeti", Vec2::new(96.0, 96.0));
    let mut actor = game.world.entity(id).unwrap().clone();
    actor.state = 98;
    actor.health = 1;
    actor.timer = 1;
    actor.held = true;
    actor.vel = Vec2::default();
    actor.data.insert("counter".into(), 37.0);
    game.player.held = Some(id);
    game.player.pickup_item_type = "Yeti".into();
    let mut expected = game.rng.clone();
    yeti(&mut game, &mut actor);
    assert_eq!((actor.state, actor.timer), (98, 0));
    assert_eq!(actor.value("bounced"), 0.0);
    assert_eq!(game.player.held, Some(id));
    yeti(&mut game, &mut actor);
    assert_eq!(actor.state, 0);
    assert_eq!(actor.value("counter"), 0.0);
    assert!(!actor.held);
    assert_eq!(game.player.held, None);
    assert!(game.player.pickup_item_type.is_empty());
    assert!(!game.world.entities.iter().any(|e| e.object == "oBlood"));
    assert_eq!(game.rng.clone().next_u64(), expected.next_u64());
}
