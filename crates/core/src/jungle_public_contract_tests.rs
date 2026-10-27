//! Existing-API controls also compile against the unchanged canonical V3
//! baseline. No proposed production helper or revised type is used here.
use crate::{Game, RngAlgorithm, RulesPreset, Vec2};

fn rng_identity(rng: &crate::Rng) -> (RngAlgorithm, u64) {
    (rng.algorithm(), rng.state_bits())
}

#[test]
fn jungle_public_source_actual_generated_markets_have_native_create_before_first_tick() {
    for seed in [0, 1, 42, 8765] {
        let mut game = Game::shell(seed, RulesPreset::Classic);
        game.world.level = 4;
        game.flags.insert("genMarketEntrance".into());
        game.flags.insert("no_dark_level".into());
        game.data.insert("marketChance".into(), 0.);
        game.next_level();
        assert_eq!(game.world.level, 5);
        assert_eq!(game.level_tick, 0);
        let markets: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|e| e.alive && e.object == "oXMarket")
            .collect();
        assert!(
            !markets.is_empty(),
            "source primary or forced entrance at seed{seed}"
        );
        for market in markets {
            assert_eq!(
                market.text.get("type").map(String::as_str),
                Some("Market Exit")
            );
            assert_eq!(market.text.get("leadsTo").map(String::as_str), Some(""));
        }
        let carried = game.data["marketChance"];
        assert!(
            carried <= 0.,
            "signed failed conditions decrement without saturation"
        );
        game.next_level();
        assert_eq!(game.world.level, 6);
        assert_eq!(game.data["marketChance"], carried);
    }
}

#[test]
fn jungle_public_source_actual_native_children_are_complete_and_reentry_rng_free() {
    let mut game = Game::shell(31, RulesPreset::Classic);
    game.player.equipment.insert("Spectacles".into());
    for object in [
        "oGoldNugget",
        "oLampItem",
        "oLampRedItem",
        "oSpearTrapBottom",
        "oSpearTrapTop",
        "oSpearTrapLit",
        "oXMarket",
    ] {
        let before = game.rng.clone();
        let id = game.spawn(object, Vec2::new(80., 64.));
        assert_eq!(rng_identity(&game.rng), rng_identity(&before));
        let child = game.world.entity(id).unwrap();
        match object {
            "oGoldNugget" => {
                assert_eq!(child.value("canCollect"), 1.);
                assert_eq!(child.value("value"), 500.);
                assert_eq!(child.value("yOff"), 4.);
            }
            "oLampItem" | "oLampRedItem" => {
                assert_eq!(child.value("depth"), 51.);
                assert_eq!(child.value("value"), 1000.);
                assert_eq!(child.value("heavy"), 1.);
            }
            "oXMarket" => {
                assert_eq!(
                    child.text.get("type").map(String::as_str),
                    Some("Market Exit")
                );
                assert_eq!(child.text.get("leadsTo").map(String::as_str), Some(""));
            }
            _ => {
                assert_eq!(child.value("prox"), 4.);
                assert_eq!(child.value("firedMax"), 50.);
            }
        }
        game.world
            .entity_mut(id)
            .unwrap()
            .data
            .insert("fired".into(), 17.);
        game.world
            .entity_mut(id)
            .unwrap()
            .data
            .insert("value".into(), 123.);
        game.world
            .entity_mut(id)
            .unwrap()
            .text
            .insert("leadsTo".into(), "PRODUCER".into());
        crate::dispatcher::create(&mut game, id);
        let child = game.world.entity(id).unwrap();
        assert_eq!(rng_identity(&game.rng), rng_identity(&before));
        assert_eq!(child.value("fired"), 17.);
        assert_eq!(child.value("value"), 123.);
        assert_eq!(
            child.text.get("leadsTo").map(String::as_str),
            Some("PRODUCER")
        );
    }
}

#[test]
fn jungle_public_source_missing_revision_keeps_old_canonical_counter_opaque() {
    let mut json = serde_json::to_value(Game::shell(42, RulesPreset::Classic)).unwrap();
    json.as_object_mut().unwrap().remove("generation_revision");
    let mut historical: Game = serde_json::from_value(json.clone()).unwrap();
    assert_eq!(serde_json::to_value(&historical).unwrap(), json);
    historical.data.insert("marketChance".into(), -9_999_999.);
    historical.next_level();
    assert_eq!(historical.world.level, 2);
    assert_eq!(historical.data["marketChance"], -9_999_999.);
    assert!(
        serde_json::to_value(&historical)
            .unwrap()
            .get("generation_revision")
            .is_none()
    );
}
