//! Existing public APIs only: the exact same bodies can be installed on the
//! pre-selector baseline. Source-only preparation does not establish a result.
use crate::{Area, Game, RngAlgorithm, RulesPreset, replay};

#[test]
fn selectors_public_actual_yeti_lair_never_authors_a_second_live_king() {
    // Source Yeti selection only runs once the Moai flag is already set. Seed
    // 11 naturally generates the level-9 Moai, then the ordinary transition
    // selects the Yeti Lair at level 10.
    let mut game = Game::shell(11, RulesPreset::Classic);
    game.start_from_hub(false, 9);
    assert_eq!(game.world.level, 9);
    assert_eq!(game.world.area, Area::Ice);
    assert!(game.flags.contains("made_moai"));
    assert!(
        game.world
            .entities
            .iter()
            .any(|e| e.alive && e.object == "oMoai")
    );

    game.next_level();

    assert_eq!(game.world.level, 10);
    assert_eq!(game.world.area, Area::Ice);
    assert_eq!(game.world.name, "Yeti Lair");
    let count = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && e.engine_active && e.object == "oYetiKing")
        .count();
    assert!(count > 0, "the generated Yeti Lair must contain its King");
    assert!(count <= 1, "generated {count} live Yeti Kings");
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
