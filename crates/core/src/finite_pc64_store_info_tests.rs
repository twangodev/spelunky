use super::*;
include!("finite_pc64_store_info_fraction_cases.rs");
fn flags(r: StoreRounded<u64>) -> u16 {
    let fields = [
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
    ];
    fields
        .into_iter()
        .enumerate()
        .fold(0, |bits, (i, value)| bits | u16::from(value) << i)
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
fn exact_fraction_grid_covers_all_four_modes_and_full_binary128_exponent_domain() {
    assert_eq!(CASES.len(), 576);
    for &(input, mode, value, expected) in CASES {
        let result = store_qword_info(input, mode).unwrap();
        assert_eq!(
            (result.value, flags(result)),
            (value, expected),
            "input={input:?} mode={mode:?}"
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
fn signed_zeros_are_exact_and_infinity_is_never_reported_as_overflow() {
    for mode in modes() {
        for negative in [false, true] {
            let sign = u64::from(negative) << 63;
            let zero = store_qword_info(F128::zero(negative), mode).unwrap();
            assert_eq!(zero.value, sign);
            assert_eq!(zero.input_class, StoreInputClass::Zero);
            assert_eq!(flags(zero), 0);
            let ext_zero = store_ext80_qword_info(
                Ext80 {
                    sign_exp: u16::from(negative) << 15,
                    significand: 0,
                },
                mode,
            )
            .unwrap();
            assert_eq!(ext_zero, zero);
            let infinity = store_qword_info(
                F128 {
                    hi: sign | 0x7fff_0000_0000_0000,
                    lo: 0,
                },
                mode,
            )
            .unwrap();
            assert_eq!(infinity.value, sign | 0x7ff0_0000_0000_0000);
            assert_eq!(infinity.input_class, StoreInputClass::Infinity);
            assert_eq!(flags(infinity), 0);
            let ext_inf = store_ext80_qword_info(
                Ext80 {
                    sign_exp: (u16::from(negative) << 15) | 0x7fff,
                    significand: 1u64 << 63,
                },
                mode,
            )
            .unwrap();
            assert_eq!(ext_inf, infinity);
        }
    }
}
#[test]
fn tininess_after_precision_is_distinct_from_final_rounded_normal() {
    // min-normal - half a subnormal ULP, versus min-normal - quarter ULP.
    let half = F128 {
        hi: 0x3c00_ffff_ffff_ffff,
        lo: 0xf000_0000_0000_0000,
    };
    let quarter = F128 {
        hi: 0x3c00_ffff_ffff_ffff,
        lo: 0xf800_0000_0000_0000,
    };
    let a = store_qword_info(half, Ext80Rounding::NearestEven).unwrap();
    let b = store_qword_info(quarter, Ext80Rounding::NearestEven).unwrap();
    assert_eq!(a.value, 0x0010_0000_0000_0000);
    assert_eq!(b.value, a.value);
    assert!(a.tiny_before_rounding && b.tiny_before_rounding);
    assert!(a.tiny_after_precision_rounding);
    assert!(!b.tiny_after_precision_rounding);
    assert!(!a.tiny_after_rounding && !b.tiny_after_rounding);
    assert!(a.rounded_to_normal && b.rounded_to_normal);
    assert!(a.masked_underflow());
    assert!(!b.masked_underflow());
    assert!(a.inexact && b.inexact);
    let exact = store_ext80_qword_info(
        Ext80 {
            sign_exp: 0x3bcd,
            significand: 1u64 << 63,
        },
        Ext80Rounding::NearestEven,
    )
    .unwrap();
    assert_eq!(exact.value, 1);
    assert!(!exact.inexact);
    assert!(!exact.masked_underflow());
    assert!(exact.unmasked_underflow());
}
#[test]
fn overflow_detection_and_directed_response_do_not_follow_final_bits_alone() {
    // max-finite + one Ext80 ULP at that exponent. Toward-zero stays finite
    // with PE only; positive Up crosses the unbounded precision boundary.
    let above = Ext80 {
        sign_exp: 0x43fe,
        significand: 0xffff_ffff_ffff_f801,
    };
    let zero = store_ext80_qword_info(above, Ext80Rounding::ToZero).unwrap();
    let up = store_ext80_qword_info(above, Ext80Rounding::Up).unwrap();
    assert_eq!(zero.value, 0x7fef_ffff_ffff_ffff);
    assert!(zero.inexact && !zero.overflow);
    assert_eq!(up.value, 0x7ff0_0000_0000_0000);
    assert!(up.overflow && up.overflow_to_infinity);
    for negative in [false, true] {
        for mode in modes() {
            let exact = store_ext80_qword_info(
                Ext80 {
                    sign_exp: 0x43ff | (u16::from(negative) << 15),
                    significand: 1u64 << 63,
                },
                mode,
            )
            .unwrap();
            assert!(exact.overflow && exact.inexact);
            assert!(!exact.precision_inexact);
            assert!(!exact.precision_magnitude_rounded_up);
            let infinity = mode == Ext80Rounding::NearestEven
                || mode == Ext80Rounding::Down && negative
                || mode == Ext80Rounding::Up && !negative;
            assert_eq!(exact.overflow_to_infinity, infinity);
            assert_eq!(exact.magnitude_rounded_up, infinity);
            assert_eq!(
                exact.value,
                (u64::from(negative) << 63)
                    | if infinity {
                        0x7ff0_0000_0000_0000
                    } else {
                        0x7fef_ffff_ffff_ffff
                    }
            );
        }
    }
}
#[test]
fn nan_payload_quieting_and_unsupported_extended_invalid_are_separate_from_precision() {
    for mode in modes() {
        let quiet = store_qword_info(
            F128 {
                hi: 0xffff_8000_0000_0123,
                lo: 0x4000_0000_0000_0000,
            },
            mode,
        )
        .unwrap();
        assert_eq!(quiet.value, 0xfff8_0000_0000_1234);
        assert_eq!(quiet.input_class, StoreInputClass::QuietNaN);
        assert_eq!(flags(quiet), 0);
        let signal = store_qword_info(
            F128 {
                hi: 0x7fff_0000_0000_0123,
                lo: 0x4000_0000_0000_0000,
            },
            mode,
        )
        .unwrap();
        assert_eq!(signal.value, 0x7ff8_0000_0000_1234);
        assert_eq!(signal.input_class, StoreInputClass::SignalingNaN);
        assert_eq!(flags(signal), 1 << 10);
        let low_payload = store_qword_info(
            F128 {
                hi: 0x7fff_0000_0000_0000,
                lo: 1,
            },
            mode,
        )
        .unwrap();
        assert_eq!(low_payload.value, 0x7ff8_0000_0000_0000);
        assert!(low_payload.invalid && !low_payload.inexact);
        let ext_quiet = store_ext80_qword_info(
            Ext80 {
                sign_exp: 0xffff,
                significand: 0xc000_0000_0091_a000,
            },
            mode,
        )
        .unwrap();
        assert_eq!(ext_quiet, quiet);
        let ext_signal = store_ext80_qword_info(
            Ext80 {
                sign_exp: 0x7fff,
                significand: 0x8000_0000_0091_a000,
            },
            mode,
        )
        .unwrap();
        assert_eq!(ext_signal, signal);
        for raw in [
            Ext80 {
                sign_exp: 0x3fff,
                significand: 0,
            },
            Ext80 {
                sign_exp: 0xbfff,
                significand: 1,
            },
            Ext80 {
                sign_exp: 0x7fff,
                significand: 0,
            },
            Ext80 {
                sign_exp: 0x7fff,
                significand: 1,
            },
        ] {
            let invalid = store_ext80_qword_info(raw, mode).unwrap();
            assert_eq!(invalid.value, 0xfff8_0000_0000_0000);
            assert_eq!(invalid.input_class, StoreInputClass::UnsupportedEncoding);
            assert_eq!(flags(invalid), 1 << 10);
        }
    }
}
#[test]
fn raw_extended_denormal_pseudo_and_canonical_frontends_agree_numerically() {
    for raw in [
        Ext80 {
            sign_exp: 0,
            significand: 1,
        },
        Ext80 {
            sign_exp: 0,
            significand: 1u64 << 63,
        },
        Ext80 {
            sign_exp: 1,
            significand: 1u64 << 63,
        },
        Ext80 {
            sign_exp: 0x3fff,
            significand: u64::MAX,
        },
        Ext80 {
            sign_exp: 0x8000,
            significand: 1,
        },
        Ext80 {
            sign_exp: 0xbc00,
            significand: u64::MAX,
        },
    ] {
        for mode in modes() {
            assert_eq!(
                store_ext80_qword_info(raw, mode).unwrap(),
                store_qword_info(from_ext80(raw).unwrap(), mode).unwrap()
            );
        }
    }
    let pseudo = store_ext80_qword_info(
        Ext80 {
            sign_exp: 0,
            significand: 1u64 << 63,
        },
        Ext80Rounding::Up,
    )
    .unwrap();
    assert_eq!(pseudo.value, 1);
    assert!(!pseudo.invalid);
    assert!(pseudo.inexact && pseudo.masked_underflow());
}
#[test]
fn historical_nearest_api_keeps_its_finite_domain_and_matches_new_nearest_values() {
    for &(input, mode, _, _) in CASES {
        if mode != Ext80Rounding::NearestEven {
            continue;
        }
        let current = store_qword_info(input, mode).unwrap();
        if current.overflow {
            assert_eq!(
                to_f64_nearest_info(input).unwrap_err(),
                ArithmeticError::Overflow
            );
        } else {
            let old = to_f64_nearest_info(input).unwrap();
            assert_eq!(
                (old.value.to_bits(), old.inexact, old.magnitude_rounded_up),
                (current.value, current.inexact, current.magnitude_rounded_up)
            );
        }
    }
    assert_eq!(
        to_f64_nearest_info(F128 {
            hi: 0x7fff_0000_0000_0000,
            lo: 0
        })
        .unwrap_err(),
        ArithmeticError::NonFinite
    );
}
