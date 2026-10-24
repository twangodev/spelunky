//! Persistent campaign membership witnesses. Actor statuses are initial
//! configured controls; none is claimed as an ordinary-input death route.
//! Baseline builds use existing APIs: old serde ignores the proposed field.
use super::*;
use crate::{Area, RulesPreset, World};

const MARKET_PATH: [[u8; 4]; 5] = [
    [2, 4, 4, 2],
    [2, 4, 4, 2],
    [2, 4, 5, 4],
    [3, 1, 1, 3],
    [0, 0, 0, 0],
];
fn marked(rules: RulesPreset, path: [[u8; 4]; 5]) -> Game {
    let mut g = Game::shell(123, rules);
    g.world = World::new(42, 34, Area::Jungle, 5);
    // Initial source-context fixture, not a mid-run synthesized shop.
    let mut wire = serde_json::to_value(&g.world).unwrap();
    wire.as_object_mut()
        .unwrap()
        .insert("source_room_path".into(), serde_json::json!(path));
    g.world = serde_json::from_value(wire).unwrap();
    g
}
fn literal_room(x: f64, y: f64) -> (usize, usize) {
    // Independent transcription of scrGetRoomX/Y's ordered comparisons,
    // not a call to economy::room or integer division on generation cells.
    let col = if x < 176.0 {
        0
    } else if x < 336.0 {
        1
    } else if x < 496.0 {
        2
    } else {
        3
    };
    let row = if y < 144.0 {
        0
    } else if y < 272.0 {
        1
    } else if y < 400.0 {
        2
    } else if y < 528.0 {
        3
    } else {
        4
    };
    (col, row)
}
#[test]
fn campaign_shop_markers_survive_missing_corpse_moving_and_inactive_keepers() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for status in [
            "none",
            "live",
            "corpse",
            "destroyed",
            "inactive",
            "moved",
            "outside_guard",
        ] {
            let mut g = marked(rules, MARKET_PATH);
            if status != "none" {
                let pos = if status == "outside_guard" {
                    Vec2::new(80.0, 80.0)
                } else {
                    Vec2::new(240.0, 208.0)
                };
                let id = g.spawn("oShopkeeper", pos);
                let k = g.world.entities.iter_mut().find(|k| k.id == id).unwrap();
                match status {
                    "corpse" => {
                        k.health = 0;
                        k.state = 99;
                        k.data.insert("dead".into(), 1.0);
                    }
                    "destroyed" => k.alive = false,
                    "inactive" => k.engine_active = false,
                    "moved" => {
                        k.pos = Vec2::new(640.0, 592.0);
                        k.vel = Vec2::new(2.5, 0.0);
                    }
                    _ => {}
                }
            }
            let rng = g.rng.state_bits();
            let actors = serde_json::to_value(&g.world.entities).unwrap();
            for x in [
                -100.0, 16.0, 175.999, 176.0, 335.999, 336.0, 495.999, 496.0, 10000.0,
            ] {
                for y in [
                    -100.0, 143.999, 144.0, 271.999, 272.0, 399.999, 400.0, 527.999, 528.0, 10000.0,
                ] {
                    let (col, row) = literal_room(x, y);
                    let expected = MARKET_PATH[row][col] == 4 || MARKET_PATH[row][col] == 5;
                    assert_eq!(
                        in_shop(&g, Vec2::new(x, y)),
                        expected,
                        "{status}: source room({col},{row}) from{x},{y}"
                    );
                }
            }
            assert_eq!(g.rng.state_bits(), rng);
            assert_eq!(serde_json::to_value(&g.world.entities).unwrap(), actors);
        }
        // A live keeper (including a generated exit guard) cannot invent a
        // shop in a zero/route-marker cell of the authoritative source grid.
        let mut g = marked(rules, [[0; 4]; 5]);
        g.spawn("oShopkeeper", Vec2::new(240.0, 208.0));
        assert!(!in_shop(&g, Vec2::new(240.0, 208.0)));
    }
}
#[test]
fn source_campaign_generation_carries_the_selected_shop_grid_without_keeper_reconstruction() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut g = Game::shell(123, rules);
        let flags = std::collections::BTreeSet::from(["black_market".to_owned()]);
        g.world = crate::generation::generate(&mut g.rng, 5, &flags);
        let wire = serde_json::to_value(&g.world).unwrap();
        // Literal source Black Market assignments scrLevelGen52..67. The
        // first four rows are authoritative; row4 is a nonshop extension in
        // the current producer, not a proof of prior-global row4 history.
        assert_eq!(wire["source_room_path"], serde_json::json!(MARKET_PATH));
        let rng = g.rng.state_bits();
        for row in 0..5 {
            for col in 0..4 {
                let point = Vec2::new(
                    16.0 + 160.0 * col as f64 + 80.0,
                    16.0 + 128.0 * row as f64 + 64.0,
                );
                assert_eq!(in_shop(&g, point), matches!(MARKET_PATH[row][col], 4 | 5));
            }
        }
        // Initial post-generation retirement control: discard only the
        // keeper instances before the first gameplay Step, keeping all room
        // metadata, terrain, loot and generated actors otherwise untouched.
        g.world
            .entities
            .retain(|k| !crate::catalog::is_a(&k.object, "oShopkeeper"));
        for row in 0..5 {
            for col in 0..4 {
                let point = Vec2::new(
                    16.0 + 160.0 * col as f64 + 80.0,
                    16.0 + 128.0 * row as f64 + 64.0,
                );
                assert_eq!(in_shop(&g, point), matches!(MARKET_PATH[row][col], 4 | 5));
            }
        }
        assert_eq!(g.rng.state_bits(), rng);
    }
}
#[test]
fn shop_grid_roundtrips_but_absent_historical_wire_shape_and_fallback_are_unchanged() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let g = marked(rules, MARKET_PATH);
        let wire = serde_json::to_value(&g.world).unwrap();
        assert_eq!(wire["source_room_path"], serde_json::json!(MARKET_PATH));
        let restored: World = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(serde_json::to_value(restored).unwrap(), wire);
        for legacy in [false, true] {
            let mut g = if legacy {
                Game::legacy_shell(123, rules)
            } else {
                Game::shell(123, rules)
            };
            g.world = World::new(42, 34, Area::Mines, 2);
            let original = serde_json::to_value(&g.world).unwrap();
            assert!(
                !original
                    .as_object()
                    .unwrap()
                    .contains_key("source_room_path")
            );
            g.world = serde_json::from_value(original.clone()).unwrap();
            assert_eq!(serde_json::to_value(&g.world).unwrap(), original);
            assert!(!in_shop(&g, Vec2::new(240.0, 208.0)));
            let id = g.spawn("oShopkeeper", Vec2::new(240.0, 208.0));
            // Existing unversioned/configured/historical adapter uses the
            // keeper's cached room, without an active-instance filter.
            g.world.entity_mut(id).unwrap().engine_active = false;
            assert!(in_shop(&g, Vec2::new(240.0, 208.0)));
            g.world.entity_mut(id).unwrap().pos = Vec2::new(640.0, 592.0);
            assert!(in_shop(&g, Vec2::new(240.0, 208.0)));
            assert!(!in_shop(&g, Vec2::new(640.0, 592.0)));
            g.world
                .entities
                .iter_mut()
                .find(|k| k.id == id)
                .unwrap()
                .alive = false;
            assert!(!in_shop(&g, Vec2::new(240.0, 208.0)));
        }
    }
    // Explicit HistoricalV1 generation must never emit the new field, even
    // if its independent RNG identity is canonical. Its old tape is untouched.
    for canonical in [false, true] {
        let mut rng = if canonical {
            crate::Rng::new(123)
        } else {
            crate::Rng::legacy(123)
        };
        let mut state = crate::generation::GenerationState {
            revision: crate::GenerationRevision::HistoricalV1,
            market_chance: 160,
            gold_chance: 160,
            made_market_entrance: false,
            probabilities: crate::generation::SourceProbabilities::default(),
        };
        let world = crate::generation::generate_with_campaign(
            &mut rng,
            2,
            &Default::default(),
            Default::default(),
            &mut state,
        );
        assert!(
            !serde_json::to_value(world)
                .unwrap()
                .as_object()
                .unwrap()
                .contains_key("source_room_path")
        );
    }
}
#[test]
fn source_shop_query_keeps_player_tx_ty_and_authored_contexts_unreconstructed() {
    let mut g = marked(RulesPreset::Classic, MARKET_PATH);
    let rng = g.rng.state_bits();
    assert!(player_in_shop(&mut g, Vec2::new(240.0, 208.0)));
    assert_eq!(g.player.data["tx"], 240.0);
    assert_eq!(g.player.data["ty"], 208.0);
    assert!(!player_in_shop(&mut g, Vec2::new(80.0, 80.0)));
    assert_eq!(g.player.data["tx"], 80.0);
    assert_eq!(g.player.data["ty"], 80.0);
    assert_eq!(g.rng.state_bits(), rng);
    for world in [
        World::new(42, 34, Area::Mines, 1),
        crate::level::Level::default().to_world(),
        crate::generation::tutorial(),
        crate::hub::world(crate::hub::HubKind::Title, &Default::default()),
    ] {
        let wire = serde_json::to_value(world).unwrap();
        assert!(!wire.as_object().unwrap().contains_key("source_room_path"));
    }
}

#[test]
fn custom_loads_rebuild_classic_room_path_without_stale_shop_membership() {
    use crate::{Game, InputFrame};

    let level = crate::level::Level::default();

    // scrInitLevel clears the prior global grid, then scrLevelGen rebuilds it
    // for the custom room. Mines' custom route has no shop rooms.
    let mut fresh = Game::shell(123, RulesPreset::Classic);
    fresh.load_custom(&level, "fresh", "");
    let fresh_path = fresh.world.source_room_path.unwrap();
    assert_ne!(fresh_path, [[0; 4]; 5]);
    assert!(!crate::economy::in_shop(&fresh, Vec2::new(240.0, 80.0)));
    let keeper = fresh.world.spawn("oShopkeeper", Vec2::new(240.0, 208.0));
    let bomb = fresh.world.spawn("oBomb", Vec2::new(232.0, 208.0));
    crate::items::explosives::arm_bomb(fresh.world.entity_mut(bomb).unwrap());
    fresh.step(&crate::InputFrame::default());
    assert_eq!(fresh.world.entity(keeper).unwrap().value("angered"), 0.0);
    assert!(!fresh.flags.contains("wanted"));

    // Loading a custom room after a Market discards its shop grid. A linked
    // custom room runs the same path initialization again.
    let mut linked = Game::shell(123, RulesPreset::Classic);
    let flags = std::collections::BTreeSet::from(["black_market".to_owned()]);
    linked.world = crate::generation::generate(&mut linked.rng, 5, &flags);
    let campaign_market_path = linked.world.source_room_path.unwrap();
    assert_eq!(campaign_market_path, MARKET_PATH);
    linked.load_custom(&level, "first", "");
    let first_custom_path = linked.world.source_room_path.unwrap();
    assert_ne!(first_custom_path, campaign_market_path);
    assert!(!crate::economy::in_shop(&linked, Vec2::new(240.0, 80.0)));
    linked.continue_custom(&level, "second", "first");
    let second_custom_path = linked.world.source_room_path.unwrap();
    assert_ne!(second_custom_path, campaign_market_path);
    assert!(!crate::economy::in_shop(&linked, Vec2::new(240.0, 80.0)));
    let keeper = linked.world.spawn("oShopkeeper", Vec2::new(80.0, 80.0));
    let bomb = linked.world.spawn("oBomb", Vec2::new(72.0, 80.0));
    crate::items::explosives::arm_bomb(linked.world.entity_mut(bomb).unwrap());
    linked.step(&InputFrame::default());
    assert_eq!(linked.world.entity(keeper).unwrap().value("angered"), 0.0);
    assert!(!linked.flags.contains("wanted"));
}
