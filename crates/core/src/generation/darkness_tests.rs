use super::*;

#[test]
fn eager_darkness_selection_matches_independent_literal_guard_and_rng_tape() {
    for seed in 0_u32..32 {
        for level in [0, 1, 2, 5, 6, 9, 13, 16] {
            for bits in 0..16 {
                let mut flags = BTreeSet::new();
                for (bit, flag) in ["had_dark_level", "no_dark_level", "black_market", "dark"]
                    .into_iter()
                    .enumerate()
                {
                    if bits & (1 << bit) != 0 {
                        flags.insert(flag.into());
                    }
                }
                let mut actual = Rng::new(seed);
                let state = seed.wrapping_mul(0x0808_8405).wrapping_add(1);
                // Independent canonical LCG + unsigned multiply-high tape.
                // This is a one-call oracle, not full original generation.
                let roll = 1 + ((u64::from(state) * 12) >> 32);
                let mut eligible = !flags.contains("had_dark_level");
                eligible &= !flags.contains("no_dark_level");
                eligible &= level != 0;
                eligible &= level != 1;
                eligible &= Area::for_level(level) != Area::Ice;
                eligible &= level != 16;
                eligible &= roll == 1;
                let visible =
                    !flags.contains("black_market") && (flags.contains("dark") || eligible);
                assert_eq!(
                    select_dark_level(&mut actual, level, Area::for_level(level), &flags),
                    (visible, eligible),
                    "seed={seed} level={level} bits={bits}"
                );
                assert_eq!(
                    actual.next_u64(),
                    u64::from(state.wrapping_mul(0x0808_8405).wrapping_add(1)),
                    "exactly one chance draw, even when ineligible: seed={seed} level={level} bits={bits}"
                );
            }
        }
    }
}

#[test]
fn generated_market_outcome_can_record_a_dark_roll_without_dark_world() {
    let flags = BTreeSet::from(["black_market".into()]);
    let mut witness = None;
    for seed in 0..64 {
        let mut rng = Rng::new(seed);
        let mut state = GenerationState::fresh(&rng);
        let generated = generate_campaign_result(
            &mut rng,
            6,
            &flags,
            GenerationContext::default(),
            &mut state,
        );
        assert_eq!(generated.world.name, "Black Market");
        assert!(!generated.world.dark);
        if generated.dark_roll_selected {
            witness = Some(seed);
            break;
        }
    }
    assert!(
        witness.is_some(),
        "bounded stock-probability generated Market must exercise a selected-but-invisible dark roll"
    );
}
