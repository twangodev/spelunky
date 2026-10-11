//! Existing public APIs only: the exact same bodies can be installed on the
//! pre-selector baseline. Source-only preparation does not establish a result.
use crate::{Area, Game, RngAlgorithm, RulesPreset, replay};

#[test]
fn selectors_public_actual_yeti_lair_never_authors_a_second_live_king() {
    // scrRoomGenYeti:104-117 tests instance_exists(oYetiKing) before choosing
    // the next main room. Once stamped, later main rooms draw only cases1..9.
    let mut king_births = 0;
    for seed in 0..32 {
        let mut game = Game::new(seed, RulesPreset::Classic);
        game.world.level = 8;
        game.flags.insert("yeti_lair".into());
        game.flags.insert("no_dark_level".into());
        game.next_level();
        assert_eq!(game.world.level, 9);
        assert_eq!(game.world.area, Area::Ice);
        assert_eq!(game.world.name, "Yeti Lair");
        let count = game
            .world
            .entities
            .iter()
            .filter(|e| e.alive && e.engine_active && e.object == "oYetiKing")
            .count();
        assert!(count <= 1, "seed {seed} authored {count} live Yeti Kings");
        king_births += count;
    }
    assert!(
        king_births > 0,
        "the entire fixed sweep must contain an authored King"
    );
}

#[test]
fn selectors_public_source_classic_wire_contract_preserves_historical_imports() {
    let canonical = Game::new(42, RulesPreset::Classic);
    let json = serde_json::to_value(&canonical).unwrap();
    assert_eq!(json["generation_revision"], "SourceClassicV2");
    assert_eq!(canonical.simulation_version(), 2);
    assert_eq!(
        replay::from_frames(42, RulesPreset::Classic, &[])
            .unwrap()
            .simulation_version,
        2
    );
    let restored: Game = serde_json::from_value(json.clone()).unwrap();
    assert_eq!(serde_json::to_value(restored).unwrap(), json);

    // Missing revision means historical, including a tagged canonical RNG.
    let mut missing = serde_json::to_value(Game::shell(42, RulesPreset::Classic)).unwrap();
    missing
        .as_object_mut()
        .unwrap()
        .remove("generation_revision");
    let historical: Game = serde_json::from_value(missing.clone()).unwrap();
    assert_eq!(historical.simulation_version(), 1);
    assert_eq!(historical.rng.algorithm(), RngAlgorithm::Gm8Lcg32V1);
    assert_eq!(serde_json::to_value(historical).unwrap(), missing);

    // Explicit legacy imports keep the full seed and the historical version.
    let legacy = Game::new_with_algorithm(
        u64::MAX,
        RulesPreset::Classic,
        RngAlgorithm::LegacySplitMix64V1,
    )
    .unwrap();
    assert_eq!(legacy.simulation_version(), 1);
    assert_eq!(legacy.rng.algorithm(), RngAlgorithm::LegacySplitMix64V1);
    assert_eq!(legacy.seed, u64::MAX);
    assert!(
        serde_json::to_value(legacy)
            .unwrap()
            .get("generation_revision")
            .is_none()
    );
    assert_eq!(
        replay::from_frames_with_algorithm(
            u64::MAX,
            RulesPreset::Classic,
            &[],
            RngAlgorithm::LegacySplitMix64V1
        )
        .unwrap()
        .simulation_version,
        1
    );
}
