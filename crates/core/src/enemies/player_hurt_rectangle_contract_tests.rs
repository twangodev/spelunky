use crate::{Area, Game, GameEvent, InputFrame, RulesPreset, Vec2, World};

#[test]
fn actual_default_projectile_first_step_hits_inclusive_query_edge_only() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (object, edge_x, damage) in [("oLaser", 92.0, 3), ("oPsychicWave", 95.0, 1)] {
            for outside in [false, true] {
                let mut g = Game::shell(7, rules);
                g.world = World::new(30, 20, Area::Ice, 9);
                g.world.name = "rLevel".into();
                for x in 0..30 {
                    g.world.set_tile(x, 7, "oHardBlock");
                }
                g.player.pos = Vec2::new(80.0, 104.0);
                g.player.previous_pos = g.player.pos;
                g.player.grounded = true;
                let actor = g.spawn(object, Vec2::new(edge_x + f64::from(outside), 104.0));
                let e = g.world.entity(actor).unwrap();
                assert_eq!(e.pos.y, 104.0);
                // Initial default sprites have cached bbox minimum88 at the
                // inclusive query's right endpoint; outside control minimum89.
                let source_gate = e
                    .collision_mask()
                    .intersects_rectangle_bbox(Vec2::new(72.0, 96.0), Vec2::new(88.0, 112.0));
                assert_eq!(source_gate, !outside);
                g.step(&InputFrame::default());
                assert_eq!(g.player.health, if outside { 4 } else { 4 - damage });
                assert_eq!(
                    g.events.contains(&GameEvent::Damage { amount: damage }),
                    !outside
                );
                assert_eq!(g.player.pos, Vec2::new(80.0, 104.0));
                if !outside {
                    assert_eq!(g.player.vel, Vec2::new(-2.0, -4.0));
                    assert_eq!(g.player.stun, if object == "oLaser" { 20 } else { 40 });
                }
                if object == "oPsychicWave" {
                    assert!(
                        g.world.entity(actor).is_some(),
                        "source wave survives its own Player hurt query"
                    );
                }
            }
        }
    }
}
