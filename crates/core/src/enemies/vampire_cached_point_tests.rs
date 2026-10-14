//! Native point-kernel and configured own-Step gates, with old-existing APIs.
//! These are not claims of stock frequency or ordinary weapon reachability.
use super::*;
use crate::{Area, RulesPreset};

#[test]
fn native_point_distance_keeps_raw_point_cached_envelope_and_no_cap_or_alive_gate() {
    let mut e = Entity::new(900_000, "oVampire", Vec2::new(300.25, 64.25));
    crate::enemies::init_entity(&mut e);
    e.set_sprite("sVampireRunL");
    e.animation = 3.0;
    e.data.insert("image_xscale".into(), -1.0);
    // Source Run aggregate[0,0,15,15],origin0. Mirroring independently
    // rounds x0=300.25 and x1=300.25+(15+1)*-1-1=283.25.
    // Cached envelope[283,64,300,79]; selected frame3's smooth L=285.25.
    assert_eq!(
        e.collision_mask().point_bounds(),
        Rect::new(283.0, 64.0, 18.0, 16.0)
    );
    for point in [
        Vec2::new(187.25, 64.125),
        Vec2::new(280.0, 60.0),
        Vec2::new(300.125, 70.0),
        Vec2::new(-2_000_000.0, 70.0),
    ] {
        let dx = (283.0 - point.x).max(point.x - 300.0).max(0.0);
        let dy = (64.0 - point.y).max(point.y - 79.0).max(0.0);
        let literal = (dx * dx + dy * dy).sqrt();
        assert_eq!(distance_to_point(&e, point), literal);
    }
    assert_eq!(distance_to_point(&e, Vec2::new(187.25, 64.125)), 95.75);
    assert_eq!(
        distance_to_point(&e, Vec2::new(-2_000_000.0, 70.0)),
        2_000_283.0
    );
    e.alive = false;
    e.engine_active = false;
    assert_eq!(
        distance_to_point(&e, Vec2::new(187.25, 64.125)),
        95.75,
        "kernel has no receiver retirement/activation guard"
    );
}

#[test]
fn vampire_idle_hang_and_fly_strict_gates_query_cache_after_own_movement() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for legacy in [false, true] {
            for (status, threshold) in [(0, 96.0), (6, 90.0), (7, 160.0)] {
                for gap in [threshold - 1.0, threshold, threshold + 1.0] {
                    let mut g = if legacy {
                        Game::legacy_shell(123, rules)
                    } else {
                        Game::shell(123, rules)
                    };
                    g.world = World::new(40, 20, Area::Mines, 1);
                    let mut e = Entity::new(900_000, "oVampire", Vec2::new(300.0, 64.0));
                    crate::enemies::init_entity(&mut e);
                    e.state = status;
                    e.timer = 5;
                    e.vel = Vec2::new(1.0, 0.0);
                    e.data.insert("image_xscale".into(), -1.0);
                    e.set_sprite(match status {
                        6 => "sVampireBatHang",
                        7 => "sVampireBatLeft",
                        _ => "sVampireRunL",
                    });
                    e.animation = if status == 0 {
                        3.0
                    } else if status == 7 {
                        2.0
                    } else {
                        0.0
                    };
                    if status == 0 {
                        for x in 0..40 {
                            g.world.set_tile(x, 5, "oHardBlock");
                        }
                    } else {
                        // Preserve HANG's support gate and make FLY's far
                        // branch enter HANG, instead of the unrelated up-flight.
                        g.world.set_tile(19, 3, "oHardBlock");
                    }
                    // Source moveTo writes300->301 before the query. Run/Bat
                    // aggregate right15 mirrors to cache left284; Hang right14
                    // mirrors to285. Point is RAW Player.x-8,Player.y-8.
                    let left = if status == 6 { 285.0 } else { 284.0 };
                    g.player.pos =
                        Vec2::new(left - gap + 8.0, if status == 6 { 81.0 } else { 72.0 });
                    g.player.previous_pos = g.player.pos;
                    let literal_gap = (left - (g.player.pos.x - 8.0)).max(0.0);
                    assert_eq!(literal_gap, gap);
                    assert!(!solid_at(&g.world, &e, e.pos.x + 8.0, e.pos.y + 8.0));
                    assert_eq!(bottom(&g.world, &e), status == 0);
                    let rng = g.rng.state_bits();
                    vampire(&mut g, &mut e);
                    assert_eq!(
                        e.pos,
                        Vec2::new(301.0, 64.0),
                        "own X setter refresh precedes distance"
                    );
                    let near = literal_gap < threshold;
                    match status {
                        0 => {
                            assert_eq!(e.state, if near { 2 } else { 0 });
                            assert_eq!(e.timer, 4);
                            assert!((e.vel.x - 0.9).abs() < 1e-12);
                            assert_eq!(e.vel.y, 0.0);
                            assert_eq!(e.sprite, "sVampireRunL");
                        }
                        6 => {
                            assert_eq!(e.state, if near { 7 } else { 6 });
                            assert_eq!(e.vel, Vec2::default());
                            assert_eq!(
                                e.sprite,
                                if near {
                                    "sVampireBatLeft"
                                } else {
                                    "sVampireBatHang"
                                }
                            );
                            assert_eq!(g.events.iter().filter(|event|matches!(event,crate::GameEvent::Sound{name,..} if name=="sndBat")).count(),usize::from(near));
                        }
                        7 => {
                            assert_eq!(e.state, if near { 7 } else { 6 });
                            assert_eq!(
                                e.sprite,
                                if near {
                                    "sVampireBatLeft"
                                } else {
                                    "sVampireBatHang"
                                }
                            );
                            assert!((e.vel.x - if near { -1.9 } else { 0.0 }).abs() < 1e-12);
                            assert_eq!(e.vel.y, 0.0);
                        }
                        _ => unreachable!(),
                    }
                    assert_eq!(e.health, 6);
                    assert!(e.alive);
                    assert_eq!(
                        g.rng.state_bits(),
                        rng,
                        "this quiet source branch has no timer expiry/burn/pause draws"
                    );
                }
            }
        }
    }
}
