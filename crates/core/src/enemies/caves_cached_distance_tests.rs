//! Configured source-branch witnesses, not stock campaign reachability claims.
//! Every callback used here already exists in the old production source.
use super::*;
use crate::{Area, RulesPreset};

fn fixture(rules: RulesPreset, legacy: bool) -> Game {
    let mut g = if legacy {
        Game::legacy_shell(123, rules)
    } else {
        Game::shell(123, rules)
    };
    g.world = World::new(40, 20, Area::Mines, 1);
    g.player.sprite = "sStandLeft".into();
    g.player.data.insert("image_xscale".into(), -1.0);
    g
}
fn sight(g: &mut Game) {
    let mut e = Entity::new(900_000, "oEnemySight", Vec2::new(40.0, 64.0));
    crate::enemies::init_entity(&mut e);
    assert!(collision(g, &mut e, crate::dispatcher::Instance::Player));
}

#[test]
fn sight_alert_uses_cached_owner_player_envelopes_at_strict_hundred() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for legacy in [false, true] {
            for (object, right) in [("oCaveman", 78.0), ("oHawkman", 79.0)] {
                for gap in [99.0, 100.0, 101.0] {
                    let mut g = fixture(rules, legacy);
                    let id = g.spawn(object, Vec2::new(64.0, 64.0));
                    g.player.pos = Vec2::new(right + gap + 7.0, 72.0);
                    // Source Caveman x[1,14], Hawk x[0,15], origins0.
                    // Mirrored Stand x[2,13],origin8 caches [P-7,P+6],
                    // whereas the old smooth adapter starts at P-5.
                    let literal_gap = (g.player.pos.x - 7.0 - right).max(0.0);
                    assert_eq!(literal_gap, gap);
                    assert_eq!(g.player.collision_mask().point_bounds().x, right + gap);
                    assert_eq!(g.world.entity(id).unwrap().state, 0);
                    let rng = g.rng.state_bits();
                    sight(&mut g);
                    let owner = g.world.entity(id).unwrap();
                    assert_eq!(owner.state, if gap < 100.0 { 2 } else { 0 });
                    assert_eq!(owner.health, if object == "oCaveman" { 3 } else { 4 });
                    assert_eq!(g.rng.state_bits(), rng);
                    assert_eq!(
                        g.events
                            .iter()
                            .filter(|event| matches!(event,
                                crate::GameEvent::Sound { name, .. } if name == "sndAlert"
                            ))
                            .count(),
                        usize::from(gap < 100.0)
                    );
                }
            }
        }
    }
}

#[test]
fn hanging_spider_drop_uses_rounded_envelopes_at_strict_ninety() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for legacy in [false, true] {
            for giant in [false, true] {
                for gap in [89.0, 90.0, 91.0] {
                    let mut g = fixture(rules, legacy);
                    g.world.set_tile(4, 3, "oHardBlock");
                    let mut e = Entity::new(
                        900_000,
                        if giant {
                            "oGiantSpiderHang"
                        } else {
                            "oSpiderHang"
                        },
                        Vec2::new(64.0, 64.49),
                    );
                    crate::enemies::init_entity(&mut e);
                    let bottom = if giant { 79.0 } else { 73.0 };
                    g.player.pos = Vec2::new(if giant { 80.0 } else { 72.0 }, bottom + gap + 8.0);
                    // Hang aggregate bottom9/15 at64.49 rounds to73/79.
                    // Source Player top=P.y-8; both horizontal boxes overlap.
                    let literal_gap = (g.player.pos.y - 8.0 - bottom).max(0.0);
                    assert_eq!(literal_gap, gap);
                    assert_eq!(e.collision_mask().point_bounds().y, 64.0);
                    assert!(g.world.solid_at(e.pos.x, e.pos.y - 16.0));
                    assert!(!g.world.solid_at(
                        e.pos.x + if giant { 16.0 } else { 8.0 },
                        e.pos.y + if giant { 8.0 } else { 4.0 }
                    ));
                    let mut rng = g.rng.clone();
                    let drops = literal_gap < 90.0;
                    if giant && drops {
                        // GiantSpider own Create has exactly rand(100,1000).
                        rng.range(901);
                    }
                    tick(&mut g, &mut e);
                    assert_eq!(e.alive, !drops);
                    let born: Vec<_> = g
                        .world
                        .entities
                        .iter()
                        .filter(|target| {
                            target.alive
                                && target.object == if giant { "oGiantSpider" } else { "oSpider" }
                        })
                        .collect();
                    assert_eq!(born.len(), usize::from(drops));
                    if drops {
                        assert_eq!(born[0].pos, Vec2::new(64.0, 64.49));
                        assert_eq!(born[0].health, if giant { 10 } else { 1 });
                    }
                    assert_eq!(g.rng.state_bits(), rng.state_bits());
                }
            }
        }
    }
}

#[test]
fn spider_bounce_uses_cached_gap_and_preserves_literal_launch_rng() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for legacy in [false, true] {
            for giant in [false, true] {
                let threshold = if giant { 120.0 } else { 90.0 };
                for gap in [threshold - 1.0, threshold, threshold + 1.0] {
                    let mut g = fixture(rules, legacy);
                    for x in 0..40 {
                        g.world.set_tile(x, if giant { 6 } else { 5 }, "oHardBlock");
                    }
                    let mut e = Entity::new(
                        900_000,
                        if giant { "oGiantSpider" } else { "oSpider" },
                        Vec2::new(64.0, 64.0),
                    );
                    crate::enemies::init_entity(&mut e);
                    e.set_sprite(if giant { "sGiantSpider" } else { "sSpider" });
                    e.state = 1;
                    e.vel.x = 1.0;
                    e.data.insert("squirt_timer".into(), 50.0);
                    // The real own moveTo writes64->65 before the query.
                    // Source Spider right15/Giant right30 at65 =>80/95.
                    let post_right = if giant { 95.0 } else { 80.0 };
                    g.player.pos =
                        Vec2::new(post_right + gap + 7.0, if giant { 88.0 } else { 72.0 });
                    assert!(bottom(&g.world, &e));
                    assert!(!crushed(
                        &g,
                        &e,
                        if giant {
                            Vec2::new(16.0, 24.0)
                        } else {
                            Vec2::new(8.0, 8.0)
                        }
                    ));
                    let literal_gap = (g.player.pos.x - 7.0 - post_right).max(0.0);
                    assert_eq!(literal_gap, gap);
                    let admitted = literal_gap < threshold;
                    let mut rng = g.rng.clone();
                    let launch = if admitted {
                        Some((rng.range(4), rng.range(4)))
                    } else {
                        None
                    };
                    tick(&mut g, &mut e);
                    assert_eq!(
                        e.pos,
                        Vec2::new(65.0, 64.0),
                        "query follows real coordinate refresh"
                    );
                    if let Some((vertical, idle)) = launch {
                        assert_eq!(e.state, if idle == 0 { 0 } else { 1 });
                        assert_eq!(e.vel.x, if idle == 0 { 0.0 } else { 2.5 });
                        assert_eq!(
                            e.vel.y,
                            if idle == 0 {
                                0.0
                            } else {
                                -f64::from((if giant { 3 } else { 2 }) + vertical)
                            }
                        );
                    } else {
                        assert_eq!(e.state, 0);
                        assert_eq!(e.vel, Vec2::new(1.0, if giant { 0.3 } else { 0.2 }));
                    }
                    assert_eq!(e.health, 1);
                    assert_eq!(g.rng.state_bits(), rng.state_bits());
                }
            }
        }
    }
}

#[test]
fn source_family_queries_exclude_inactive_targets_and_inactive_sight_owners() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for legacy in [false, true] {
            // The source with oCaveman excludes deactivated receivers.
            let mut g = fixture(rules, legacy);
            g.player.pos = Vec2::new(184.0, 72.0);
            let id = g.spawn("oCaveman", Vec2::new(64.0, 64.0));
            g.world.entity_mut(id).unwrap().engine_active = false;
            sight(&mut g);
            assert_eq!(g.world.entity(id).unwrap().state, 0);
            // EnemySight explicitly asks for Player1, not Character.
            let mut g = fixture(rules, legacy);
            g.player.engine_active = false;
            let id = g.spawn("oCaveman", Vec2::new(64.0, 64.0));
            let character = g.world.spawn("oCharacter", Vec2::new(80.0, 72.0));
            g.world
                .entity_mut(character)
                .unwrap()
                .set_sprite("sStandLeft");
            sight(&mut g);
            assert_eq!(g.world.entity(id).unwrap().state, 0);
            for (object, active, alive, admitted) in [
                ("oCharacter", true, true, true),
                ("oCharacter", false, true, false),
                ("oCharacter", true, false, false),
                ("oPDummy", true, true, false),
            ] {
                let mut g = fixture(rules, legacy);
                for x in 0..40 {
                    g.world.set_tile(x, 5, "oHardBlock");
                }
                g.player.engine_active = false;
                g.player.pos = Vec2::new(500.0, 72.0);
                let target = g.world.spawn(object, Vec2::new(176.0, 72.0));
                let t = g
                    .world
                    .entities
                    .iter_mut()
                    .find(|t| t.id == target)
                    .unwrap();
                t.set_sprite("sStandLeft");
                t.data.insert("image_xscale".into(), -1.0);
                t.engine_active = active;
                t.alive = alive;
                let mut e = Entity::new(900_000, "oSpider", Vec2::new(64.0, 64.0));
                crate::enemies::init_entity(&mut e);
                e.set_sprite("sSpider");
                e.state = 1;
                e.vel.x = 1.0;
                let mut rng = g.rng.clone();
                let rolls = if admitted {
                    Some((rng.range(4), rng.range(4)))
                } else {
                    None
                };
                tick(&mut g, &mut e);
                assert_eq!(e.pos, Vec2::new(65.0, 64.0));
                assert_eq!(
                    e.state,
                    if rolls.is_some_and(|(_, r)| r != 0) {
                        1
                    } else {
                        0
                    }
                );
                assert_eq!(
                    e.vel.y,
                    if let Some((r, idle)) = rolls {
                        if idle == 0 { 0.0 } else { -f64::from(2 + r) }
                    } else {
                        0.2
                    }
                );
                assert_eq!(g.rng.state_bits(), rng.state_bits());
            }
        }
    }
}
