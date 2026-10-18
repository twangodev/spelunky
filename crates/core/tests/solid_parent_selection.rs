//! Original oSolid Destroy point gates and translated first-family queries.
//! Initial rooms are authored fixtures, not a generated-campaign parity claim.
//! ThinIce uses source-default Create and thirty neutral Game.step frames.
use spelunky_core::{Area, Game, InputFrame, PlayerState, RulesPreset, Vec2, World};
const PRESETS: [RulesPreset; 2] = [RulesPreset::Classic, RulesPreset::Assisted];
fn scene(rules: RulesPreset) -> (Game, u64) {
    let mut g = Game::shell(1, rules);
    g.world = World::new(20, 20, Area::Ice, 9);
    g.flags.insert("custom".into());
    g.player.pos = Vec2::new(88., 72.);
    g.player.previous_pos = g.player.pos;
    g.player.state = PlayerState::Standing;
    g.player.grounded = true;
    let ice = g.spawn("oThinIce", Vec2::new(80., 80.));
    assert_eq!(g.world.entity(ice).unwrap().value("thickness"), 60.);
    (g, ice)
}
fn collapse(g: &mut Game, ice: u64) {
    for frame in 1..=29 {
        g.step(&InputFrame::default());
        assert_eq!(g.player.pos, Vec2::new(88., 72.));
        assert_eq!(g.player.health, 4.0);
        assert_eq!(
            g.world.entity(ice).unwrap().value("thickness"),
            60. - 2. * f64::from(frame)
        );
    }
    assert_eq!(g.world.entity(ice).unwrap().sprite, "sThinIce6");
    assert!(
        !g.world
            .entities
            .iter()
            .any(|e| e.object.ends_with("LampItem") || e.object.ends_with("LampRedItem"))
    );
    g.step(&InputFrame::default());
    assert!(g.world.entity(ice).is_none());
    assert_eq!(g.player.pos, Vec2::new(88., 72.));
    assert_eq!(g.player.health, 4.0);
    assert!(
        g.world.fluid_dirty,
        "parent water invalidation follows attachments"
    );
}
fn drops(g: &Game) -> Vec<(&str, Vec2)> {
    g.world
        .entities
        .iter()
        .filter(|e| e.alive && matches!(e.object.as_str(), "oLampItem" | "oLampRedItem"))
        .map(|e| (e.object.as_str(), e.pos))
        .collect()
}
#[test]
fn neutral_thin_ice_selects_first_translated_target_not_point_gate_owner() {
    for rules in PRESETS {
        let (mut g, ice) = scene(rules);
        let b = g.spawn("oLamp", Vec2::new(96., 96.));
        let a = g.spawn("oLamp", Vec2::new(80., 96.));
        assert!(!g.world.entity(b).unwrap().mask_contains(
            Vec2::new(88., 98.),
            spelunky_core::CollisionPrecision::BoundingBox
        ));
        assert!(g.world.entity(a).unwrap().mask_contains(
            Vec2::new(88., 98.),
            spelunky_core::CollisionPrecision::BoundingBox
        ));
        collapse(&mut g, ice);
        assert!(
            g.world.entity(b).is_none(),
            "first translated overlap must be selected"
        );
        assert!(
            g.world.entity(a).is_some(),
            "point gate owner is not necessarily selected"
        );
        assert_eq!(drops(&g), vec![("oLampItem", Vec2::new(104., 108.))]);
    }
}
#[test]
fn neutral_thin_ice_removes_only_one_normal_lamp() {
    for rules in PRESETS {
        let (mut g, ice) = scene(rules);
        let a = g.spawn("oLamp", Vec2::new(80., 96.));
        let b = g.spawn("oLamp", Vec2::new(80., 96.));
        collapse(&mut g, ice);
        assert!(g.world.entity(a).is_none());
        assert!(g.world.entity(b).is_some());
        assert_eq!(drops(&g), vec![("oLampItem", Vec2::new(88., 108.))]);
    }
}
#[test]
fn neutral_thin_ice_second_lamp_pass_includes_surviving_red_descendant() {
    for rules in PRESETS {
        let (mut g, ice) = scene(rules);
        let a = g.spawn("oLampRed", Vec2::new(80., 96.));
        let b = g.spawn("oLampRed", Vec2::new(80., 96.));
        collapse(&mut g, ice);
        assert!(g.world.entity(a).is_none());
        assert!(g.world.entity(b).is_none());
        assert_eq!(
            drops(&g),
            vec![
                ("oLampRedItem", Vec2::new(88., 108.)),
                ("oLampItem", Vec2::new(88., 108.))
            ]
        );
    }
}
#[test]
fn neutral_thin_ice_red_replacement_precedes_normal_despite_birth_order() {
    for rules in PRESETS {
        let (mut g, ice) = scene(rules);
        g.spawn("oLamp", Vec2::new(80., 96.));
        g.spawn("oLampRed", Vec2::new(80., 96.));
        collapse(&mut g, ice);
        assert_eq!(
            drops(&g),
            vec![
                ("oLampRedItem", Vec2::new(88., 108.)),
                ("oLampItem", Vec2::new(88., 108.))
            ]
        );
    }
}
#[test]
fn neutral_thin_ice_does_not_query_without_point_gate() {
    for rules in PRESETS {
        let (mut g, ice) = scene(rules);
        let lamp = g.spawn("oLamp", Vec2::new(96., 96.));
        collapse(&mut g, ice);
        assert!(g.world.entity(lamp).is_some());
        assert!(drops(&g).is_empty());
    }
}
#[test]
fn neutral_thin_ice_selects_terrain_and_entities_in_one_family_order() {
    for rules in PRESETS {
        for terrain_first in [false, true] {
            let (mut g, ice) = scene(rules);
            let dynamic = if terrain_first {
                g.world.set_tile(6, 6, "oLamp");
                g.spawn("oLamp", Vec2::new(80., 96.))
            } else {
                let id = g.spawn("oLamp", Vec2::new(96., 96.));
                g.world.set_tile(5, 6, "oLamp");
                id
            };
            collapse(&mut g, ice);
            if terrain_first {
                assert!(g.world.tile(6, 6).unwrap().object.is_empty());
                assert!(g.world.entity(dynamic).is_some());
            } else {
                assert!(g.world.entity(dynamic).is_none());
                assert_eq!(g.world.tile(5, 6).unwrap().object, "oLamp");
            }
            assert_eq!(drops(&g), vec![("oLampItem", Vec2::new(104., 108.))]);
        }
    }
}
#[test]
fn ordinary_mattock_grave_query_selects_first_overlap_and_runs_unguarded_own_body() {
    for rules in PRESETS {
        let mut g = Game::shell(23, rules);
        g.world = World::new(40, 20, Area::Mines, 1);
        g.flags.insert("custom".into());
        for x in 0..40 {
            g.world.set_tile(x, 10, "oHardBlock");
        }
        g.player.pos = Vec2::new(80., 152.);
        g.player.previous_pos = g.player.pos;
        g.player.state = PlayerState::Standing;
        g.player.grounded = true;
        g.player.facing = 1;
        let block = g.spawn("oBrick", Vec2::new(96., 144.));
        g.world
            .entity_mut(block)
            .unwrap()
            .data
            .insert("cleanDeath".into(), 1.);
        let b = g.spawn("oGrave", Vec2::new(112., 144.));
        let a = g.spawn("oGrave", Vec2::new(96., 128.));
        let mattock = g.spawn("oMattock", g.player.pos);
        {
            let m = g.world.entity_mut(mattock).unwrap();
            m.data.insert("cost".into(), 0.);
            m.data.insert("forSale".into(), 0.);
        }
        g.step(&InputFrame {
            down: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert_eq!(g.player.held, Some(mattock));
        g.step(&InputFrame::default());
        g.step(&InputFrame {
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        let mut terminal = false;
        for _ in 0..40 {
            let hit = g
                .world
                .entities
                .iter()
                .find(|e| e.alive && e.object == "oMattockHit")
                .cloned();
            let mut expected = g.rng.clone();
            let mut rubble = Vec::new();
            for _ in 0..3 {
                rubble.push(Vec2::new(
                    120. + f64::from(expected.range(9)) - f64::from(expected.range(9)),
                    152. + f64::from(expected.range(9)) - f64::from(expected.range(9)),
                ));
            }
            expected.range(20);
            g.step(&InputFrame::default());
            if let Some(hit) = hit {
                if g.world.entity(hit.id).is_none() {
                    assert!(
                        hit.animation + hit.image_speed()
                            >= spelunky_core::catalog::sprite_frame_count(&hit.sprite) as f64
                    );
                    assert!(g.world.entity(block).is_none());
                    assert!(g.world.entity(b).is_none());
                    assert!(g.world.entity(a).is_some());
                    let actual: Vec<_> = g
                        .world
                        .entities
                        .iter()
                        .filter(|e| matches!(e.object.as_str(), "oRubble" | "oRubbleSmall"))
                        .map(|e| e.pos)
                        .collect();
                    assert_eq!(
                        actual, rubble,
                        "Grave own debris ignores receiver cleanDeath"
                    );
                    assert_eq!(g.rng.clone().next_u64(), expected.next_u64());
                    terminal = true;
                    break;
                }
            }
        }
        assert!(terminal, "actual Mattock AnimationEnd must execute");
    }
}

#[test]
fn configured_maskless_receiver_does_not_destroy_point_gated_lamp() {
    for rules in PRESETS {
        let (mut g, ice) = scene(rules);
        {
            let source = g.world.entity_mut(ice).unwrap();
            source.set_sprite("");
            source.data.insert("thickness".into(), 2.);
        }
        let lamp = g.spawn("oLamp", Vec2::new(80., 96.));
        g.step(&InputFrame::default());
        assert!(
            g.world.entity(ice).is_none(),
            "raw source thinIce contact rectangle reaches configured final thickness"
        );
        assert!(
            g.world.entity(lamp).is_some(),
            "instance_place has no receiver mask although point gate is true"
        );
        assert!(drops(&g).is_empty());
    }
}
#[test]
fn configured_mirrored_receiver_rejects_adjacent_first_target_outside_translated_mask() {
    for rules in PRESETS {
        let (mut g, ice) = scene(rules);
        {
            let source = g.world.entity_mut(ice).unwrap();
            source.data.insert("image_xscale".into(), -1.);
            source.data.insert("thickness".into(), 2.);
        }
        let b = g.spawn("oLamp", Vec2::new(96., 96.));
        let a = g.spawn("oLamp", Vec2::new(80., 96.));
        g.step(&InputFrame::default());
        assert!(g.world.entity(ice).is_none());
        assert!(g.world.entity(b).is_some());
        assert!(g.world.entity(a).is_none());
        assert_eq!(drops(&g), vec![("oLampItem", Vec2::new(88., 108.))]);
    }
}
