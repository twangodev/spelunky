use super::*;
include!("finite_pc64_store_f32_info_fraction_cases.rs");
fn flags(r: StoreRounded<u32>) -> u16 {
    [
        r.inexact,
        r.magnitude_rounded_up,
        r.precision_inexact,
        r.precision_magnitude_rounded_up,
        r.overflow,
        r.overflow_to_infinity,
        r.tiny_before_rounding,
        r.tiny_after_precision_rounding,
        r.tiny_after_rounding,
        r.rounded_to_normal,
        r.invalid,
        r.masked_underflow(),
        r.unmasked_underflow(),
    ]
    .into_iter()
    .enumerate()
    .fold(0, |b, (i, v)| b | u16::from(v) << i)
}
fn modes() -> [Ext80Rounding; 4] {
    [
        Ext80Rounding::NearestEven,
        Ext80Rounding::Down,
        Ext80Rounding::Up,
        Ext80Rounding::ToZero,
    ]
}
#[test]
fn direct_fraction_grid_covers_all_rc_and_entire_finite_binary128_range() {
    assert_eq!(CASES.len(), 616);
    for &(input, mode, expected, facts) in CASES {
        let result = store_f32_info(input, mode).unwrap();
        assert_eq!(
            (result.value, flags(result)),
            (expected, facts),
            "{input:?} {mode:?}"
        );
        assert_eq!(
            result.input_class,
            if input.hi & 0x7fff_ffff_ffff_ffff == 0 && input.lo == 0 {
                StoreInputClass::Zero
            } else {
                StoreInputClass::Finite
            }
        );
    }
}
#[test]
fn extended_to_qword_intermediate_changes_both_even_and_odd_binary32_ties() {
    for (significand, qword, staged) in [
        (0x8000_0080_0000_0001, 0x3ff0_0000_1000_0000, 0x3f80_0000),
        (0x8000_017f_ffff_ffff, 0x3ff0_0000_3000_0000, 0x3f80_0002),
    ] {
        for negative in [false, true] {
            let raw = Ext80 {
                sign_exp: 0x3fff | (u16::from(negative) << 15),
                significand,
            };
            let direct = store_ext80_f32_info(raw, Ext80Rounding::NearestEven).unwrap();
            assert_eq!(direct.value, 0x3f80_0001 | (u32::from(negative) << 31));
            let through_qword = store_ext80_qword_info(raw, Ext80Rounding::NearestEven).unwrap();
            assert_eq!(through_qword.value, qword | (u64::from(negative) << 63));
            let staged_input = from_f64(f64::from_bits(through_qword.value)).unwrap();
            let staged_result = store_f32_info(staged_input, Ext80Rounding::NearestEven).unwrap();
            assert_eq!(staged_result.value, staged | (u32::from(negative) << 31));
            assert_ne!(direct.value, staged_result.value);
        }
    }
}
#[test]
fn destination_subnormal_rounding_does_not_stage_through_precision24() {
    for (input, rounded_precision, direct, staged) in [
        (
            F128 {
                hi: 0x3f6a_7fff_ffc0_0000,
                lo: 0,
            },
            F128 {
                hi: 0x3f6a_8000_0000_0000,
                lo: 0,
            },
            1,
            2,
        ),
        (
            F128 {
                hi: 0x3f6b_4000_0020_0000,
                lo: 0,
            },
            F128 {
                hi: 0x3f6b_4000_0000_0000,
                lo: 0,
            },
            3,
            2,
        ),
    ] {
        let result = store_f32_info(input, Ext80Rounding::NearestEven).unwrap();
        assert_eq!(result.value, direct);
        assert!(result.tiny_after_precision_rounding && result.masked_underflow());
        assert_eq!(
            store_f32_info(rounded_precision, Ext80Rounding::NearestEven)
                .unwrap()
                .value,
            staged
        );
        assert_ne!(direct, staged);
    }
}
#[test]
fn precision_tininess_survives_rounding_to_minimum_normal() {
    let half = F128 {
        hi: 0x3f80_ffff_fe00_0000,
        lo: 0,
    };
    let quarter = F128 {
        hi: 0x3f80_ffff_ff00_0000,
        lo: 0,
    };
    let a = store_f32_info(half, Ext80Rounding::NearestEven).unwrap();
    let b = store_f32_info(quarter, Ext80Rounding::NearestEven).unwrap();
    assert_eq!((a.value, b.value), (0x0080_0000, 0x0080_0000));
    assert!(a.tiny_before_rounding && a.tiny_after_precision_rounding);
    assert!(a.rounded_to_normal && a.masked_underflow() && !a.tiny_after_rounding);
    assert!(b.tiny_before_rounding && b.rounded_to_normal && b.inexact);
    assert!(!b.tiny_after_precision_rounding && !b.masked_underflow());
    let exact_subnormal = Ext80 {
        sign_exp: 0x3f6a,
        significand: 1u64 << 63,
    };
    let exact = store_ext80_f32_info(exact_subnormal, Ext80Rounding::NearestEven).unwrap();
    assert_eq!(exact.value, 1);
    assert!(exact.unmasked_underflow() && !exact.masked_underflow() && !exact.inexact);
}
#[test]
fn overflow_depends_on_unbounded24_round_and_preserves_directed_response() {
    for negative in [false, true] {
        let sign = u32::from(negative) << 31;
        let raw = Ext80 {
            sign_exp: 0x407e | (u16::from(negative) << 15),
            significand: 0xffff_ff00_0000_0001,
        };
        let chopped = store_ext80_f32_info(raw, Ext80Rounding::ToZero).unwrap();
        assert_eq!(chopped.value, sign | 0x7f7f_ffff);
        assert!(chopped.inexact && !chopped.overflow && !chopped.magnitude_rounded_up);
        let away = if negative {
            Ext80Rounding::Down
        } else {
            Ext80Rounding::Up
        };
        let raised = store_ext80_f32_info(raw, away).unwrap();
        assert_eq!(raised.value, sign | 0x7f80_0000);
        assert!(raised.overflow && raised.inexact && raised.magnitude_rounded_up);
        let power = Ext80 {
            sign_exp: 0x407f | (u16::from(negative) << 15),
            significand: 1u64 << 63,
        };
        for mode in modes() {
            let result = store_ext80_f32_info(power, mode).unwrap();
            assert!(result.overflow && result.inexact && !result.precision_inexact);
            let inf = mode == Ext80Rounding::NearestEven || mode == away;
            assert_eq!(
                result.value,
                sign | if inf { 0x7f80_0000 } else { 0x7f7f_ffff }
            );
            assert_eq!(result.magnitude_rounded_up, inf);
            assert!(!result.precision_magnitude_rounded_up);
        }
    }
}
#[test]
fn zero_and_infinity_preserve_sign_without_false_range_or_precision_flags() {
    for negative in [false, true] {
        let sign = u32::from(negative) << 31;
        for mode in modes() {
            let zero = store_f32_info(
                F128 {
                    hi: u64::from(negative) << 63,
                    lo: 0,
                },
                mode,
            )
            .unwrap();
            assert_eq!(
                (zero.value, flags(zero), zero.input_class),
                (sign, 0, StoreInputClass::Zero)
            );
            let raw = Ext80 {
                sign_exp: 0x7fff | (u16::from(negative) << 15),
                significand: 1u64 << 63,
            };
            let inf = store_ext80_f32_info(raw, mode).unwrap();
            assert_eq!(
                (inf.value, flags(inf), inf.input_class),
                (sign | 0x7f80_0000, 0, StoreInputClass::Infinity)
            );
            let same = store_f32_info(
                F128 {
                    hi: 0x7fff_0000_0000_0000 | (u64::from(negative) << 63),
                    lo: 0,
                },
                mode,
            )
            .unwrap();
            assert_eq!(same, inf);
        }
    }
}
#[test]
fn nan_payload_quieting_and_raw_unsupported_are_not_precision_loss() {
    for mode in modes() {
        for (hi, significand, class, invalid) in [
            (
                0xffff_8024_6800_0000,
                0xc012_3400_0000_0001,
                StoreInputClass::QuietNaN,
                false,
            ),
            (
                0xffff_0024_6800_0000,
                0x8012_3400_0000_0001,
                StoreInputClass::SignalingNaN,
                true,
            ),
        ] {
            let r = store_f32_info(F128 { hi, lo: 1 }, mode).unwrap();
            let raw = store_ext80_f32_info(
                Ext80 {
                    sign_exp: 0xffff,
                    significand,
                },
                mode,
            )
            .unwrap();
            assert_eq!(r, raw);
            assert_eq!(
                (r.value, r.input_class, r.invalid),
                (0xffc0_1234, class, invalid)
            );
            assert_eq!(flags(r), if invalid { 1 << 10 } else { 0 });
        }
        for field in [1, 0x7ffe, 0x7fff, 0x8001, 0xfffe, 0xffff] {
            for significand in [0, 1, 0x4000_0000_0000_0000] {
                let r = store_ext80_f32_info(
                    Ext80 {
                        sign_exp: field,
                        significand,
                    },
                    mode,
                )
                .unwrap();
                assert_eq!(
                    (r.value, r.input_class, flags(r)),
                    (0xffc0_0000, StoreInputClass::UnsupportedEncoding, 1 << 10)
                );
            }
        }
    }
}
#[test]
fn raw_extended_finite_forms_match_exact_container_without_qword_spill() {
    for sign_exp in [0, 1, 0x3f69, 0x3f6a, 0x3f81, 0x3fff, 0x407e, 0x407f, 0x7ffe] {
        for significand in [
            1,
            0x7fff_ffff_ffff_ffff,
            0x8000_0000_0000_0000,
            0xffff_ffff_ffff_ffff,
        ] {
            if sign_exp != 0 && significand >> 63 == 0 {
                continue;
            }
            for negative in [false, true] {
                let raw = Ext80 {
                    sign_exp: sign_exp | (u16::from(negative) << 15),
                    significand,
                };
                for mode in modes() {
                    assert_eq!(
                        store_ext80_f32_info(raw, mode).unwrap(),
                        store_f32_info(from_ext80(raw).unwrap(), mode).unwrap()
                    );
                }
            }
        }
    }
}
