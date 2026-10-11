//! Canonical math uses frozen executable-derived vectors; generation tests
//! establish deterministic Rust integration, not full original-world parity.
use spelunky_core::{Game, InputFrame, Rng, RngAlgorithm, RulesPreset, replay};

#[test]
fn default_constructors_use_the_executable_state_transition_without_reseeding() {
    // Frozen static prediction from PE SHA56b913...: 40338C/4033A8 update
    // DWORD589008 by 0x08088405*x+1 modulo2^32. Not an original runtime capture.
    for (seed, first) in [
        (0, 1),
        (1, 134775814),
        (42, 1365616851),
        (0x80000000, 2147483649),
        (u32::MAX, 4160191484),
    ] {
        for mut game in [
            Game::shell(seed, RulesPreset::Classic),
            Game::tutorial(seed, RulesPreset::Classic),
        ] {
            assert_eq!(game.seed, u64::from(seed));
            assert_eq!(game.rng.algorithm(), RngAlgorithm::Gm8Lcg32V1);
            assert_eq!(game.rng.state_bits(), u64::from(seed));
            assert_eq!(game.rng.next_u64(), first);
        }
        assert_eq!(Rng::new(seed).next_u64(), first);
    }
}

#[test]
fn canonical_external_seeds_are_checked_before_construction() {
    for seed in [u64::from(u32::MAX) + 1, (1u64 << 53) + 1, u64::MAX] {
        assert_eq!(
            Game::try_new(seed, RulesPreset::Classic).unwrap_err().seed,
            seed
        );
        assert_eq!(
            Game::try_shell(seed, RulesPreset::Classic)
                .unwrap_err()
                .seed,
            seed
        );
        assert_eq!(
            Game::try_tutorial(seed, RulesPreset::Classic)
                .unwrap_err()
                .seed,
            seed
        );
        assert!(
            Game::new_with_algorithm(seed, RulesPreset::Classic, RngAlgorithm::Gm8Lcg32V1).is_err()
        );
    }
    assert_eq!(
        Game::try_shell(u64::from(u32::MAX), RulesPreset::Classic)
            .unwrap()
            .rng
            .state_bits(),
        u64::from(u32::MAX)
    );
}

#[test]
fn naturally_generated_run_and_json_continuation_keep_canonical_identity() {
    let mut original = Game::new(0, RulesPreset::Classic);
    let mut repeated = Game::new(0, RulesPreset::Classic);
    for room in 1..=9 {
        assert_eq!(original.world.level, room);
        assert_eq!(original.state_hash(), repeated.state_hash());
        assert_eq!(original.rng.algorithm(), RngAlgorithm::Gm8Lcg32V1);
        let json = serde_json::to_string(&original).unwrap();
        let wire: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(wire["rng"]["algorithm"], "gm8_lcg32_v1");
        let mut restored: Game = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.state_hash(), original.state_hash());
        let input = replay::scripted_input(room);
        for game in [&mut original, &mut repeated, &mut restored] {
            game.step(&input);
        }
        assert_eq!(original.state_hash(), repeated.state_hash());
        assert_eq!(original.state_hash(), restored.state_hash());
        assert_eq!(original.events, restored.events);
        assert_eq!(
            original.rng.clone().next_u64(),
            restored.rng.clone().next_u64()
        );
        if room < 9 {
            original.next_level();
            repeated.next_level();
        }
    }
}

#[test]
fn legacy_object_shape_roundtrip_keeps_full_state_and_current_continuation() {
    // This establishes current legacy continuation and old wire shape. Exact
    // pre-migration simulation equality is a separate baseline/candidate
    // differential proof harness, prepared but UNRUN under compiler HOLD.
    for seed in [0, (1u64 << 53) + 1, u64::MAX] {
        let mut original = Game::legacy(seed, RulesPreset::Classic);
        let json = serde_json::to_string(&original).unwrap();
        let wire: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(wire["seed"].as_u64(), Some(seed));
        assert_eq!(wire["rng"].as_object().unwrap().len(), 1);
        assert!(wire["rng"].get("algorithm").is_none());
        let mut restored: Game = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.rng.algorithm(), RngAlgorithm::LegacySplitMix64V1);
        assert_eq!(restored.state_hash(), original.state_hash());
        for t in 0..60 {
            let input = replay::scripted_input(t);
            original.step(&input);
            restored.step(&input);
            assert_eq!(restored.state_hash(), original.state_hash());
            assert_eq!(restored.events, original.events);
            assert_eq!(
                restored.rng.clone().next_u64(),
                original.rng.clone().next_u64()
            );
        }
    }
}

#[test]
fn explicit_legacy_and_new_canonical_replays_each_remain_deterministic() {
    let frames: Vec<InputFrame> = (0..120).map(replay::scripted_input).collect();
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (seed, algorithm) in [
            (42, RngAlgorithm::Gm8Lcg32V1),
            (u64::MAX, RngAlgorithm::LegacySplitMix64V1),
        ] {
            let a = replay::from_frames_with_algorithm(seed, rules, &frames, algorithm).unwrap();
            let b =
                replay::try_scripted_with_algorithm(seed, frames.len() as u32, rules, algorithm)
                    .unwrap();
            assert_eq!(a, b);
            assert_eq!(a.algorithm, algorithm);
            assert_eq!(a.seed, seed.to_string());
        }
    }
}

#[test]
fn canonical_integer_selector_rejects_modulo_mapping_without_extra_draws() {
    // Frozen original LCG prediction: seed0 -> raw1 -> raw134775814.
    // Source floor(random(5)) selects0; raw modulo5 would incorrectly select1.
    let mut rng = Rng::new(0);
    assert_eq!(rng.algorithm(), RngAlgorithm::Gm8Lcg32V1);
    assert_eq!(rng.state_bits(), 0);
    let raw = 1_u64;
    assert_eq!((raw * 5) >> 32, 0);
    assert_ne!(raw % 5, 0);
    assert_eq!(rng.range(5), 0);
    assert_eq!(
        rng.state_bits(),
        1,
        "selector consumes exactly one original draw"
    );
    assert_eq!(rng.next_u64(), 134_775_814);
    assert_eq!(rng.state_bits(), 134_775_814);
}

#[test]
fn canonical_unit_rejects_legacy_53bit_mapping_without_extra_draws() {
    // Original random uses all32 bits. The old53-bit adapter would discard
    // raw1 completely. Both expectations are literal independent arithmetic.
    let mut rng = Rng::new(0);
    assert_eq!(rng.algorithm(), RngAlgorithm::Gm8Lcg32V1);
    assert_eq!(rng.state_bits(), 0);
    assert_eq!(rng.unit(), 1.0 / 4_294_967_296.0);
    let raw = 1_u64;
    let legacy_unit = (raw >> 11) as f64 / 9_007_199_254_740_992.0;
    assert_ne!(legacy_unit, 1.0 / 4_294_967_296.0);
    assert_eq!(
        rng.state_bits(),
        1,
        "unit consumes exactly one original draw"
    );
    assert_eq!(rng.next_u64(), 134_775_814);
    assert_eq!(rng.state_bits(), 134_775_814);
}
