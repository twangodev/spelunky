//! Fixed dyadic/rational controls for arithmetic metadata, not status emulation.
use super::*;
const MODES: [Ext80Rounding; 4] = [
    Ext80Rounding::NearestEven,
    Ext80Rounding::Down,
    Ext80Rounding::Up,
    Ext80Rounding::ToZero,
];
fn dyadic(negative: bool, sig: u128, exponent: i32) -> F128 {
    from_scaled_u128(negative, sig, exponent).unwrap()
}
fn precisions() -> [(Ext80Precision, u32); 2] {
    [(Ext80Precision::Pc53, 53), (Ext80Precision::Pc64, 64)]
}
fn check(got: Rounded<F128>, value: F128, inexact: bool, incremented: bool) {
    assert_eq!(
        got,
        Rounded {
            value,
            inexact,
            magnitude_rounded_up: incremented
        }
    );
}
fn directed_increment(mode: Ext80Rounding, negative: bool, nearest_increment: bool) -> bool {
    match mode {
        Ext80Rounding::NearestEven => nearest_increment,
        Ext80Rounding::Down => negative,
        Ext80Rounding::Up => !negative,
        Ext80Rounding::ToZero => false,
    }
}

#[test]
fn arithmetic_inexact_is_not_recoverable_from_exact_final_qword_store() {
    for (pc, p) in precisions() {
        for negative in [false, true] {
            let one = dyadic(negative, 1, 0);
            let tiny = dyadic(negative, 1, -(p as i32) - 2);
            for mode in [Ext80Rounding::NearestEven, Ext80Rounding::ToZero] {
                let lost = ext80_add_info(one, tiny, pc, mode).unwrap();
                check(lost, one, true, false);
                let exact = ext80_add_info(one, dyadic(false, 0, 0), pc, mode).unwrap();
                check(exact, one, false, false);
                // Identical final values and exact stores, opposite operation
                // inexact flags: deriving arithmetic PE from FSTP is invalid.
                let store = to_f64_nearest_info(lost.value).unwrap();
                assert!(!store.inexact && !store.magnitude_rounded_up);
                assert_eq!(lost.value, exact.value);
            }
        }
    }
}

#[test]
fn arithmetic_signed_carry_and_ties_report_magnitude_increment_for_all_modes() {
    for (pc, p) in precisions() {
        for negative in [false, true] {
            let below_two = dyadic(negative, (1u128 << p) - 1, 1 - p as i32);
            let three_quarters_ulp = dyadic(negative, 3, -(p as i32) - 1);
            for mode in MODES {
                let incremented = directed_increment(mode, negative, true);
                let expected = if incremented {
                    dyadic(negative, 1, 1)
                } else {
                    below_two
                };
                check(
                    ext80_add_info(below_two, three_quarters_ulp, pc, mode).unwrap(),
                    expected,
                    true,
                    incremented,
                );
            }
            let half_ulp = dyadic(negative, 1, -(p as i32));
            check(
                ext80_add_info(
                    dyadic(negative, 1, 0),
                    half_ulp,
                    pc,
                    Ext80Rounding::NearestEven,
                )
                .unwrap(),
                dyadic(negative, 1, 0),
                true,
                false,
            );
            let odd = dyadic(negative, (1u128 << (p - 1)) + 1, 1 - p as i32);
            let even = dyadic(negative, (1u128 << (p - 1)) + 2, 1 - p as i32);
            check(
                ext80_add_info(odd, half_ulp, pc, Ext80Rounding::NearestEven).unwrap(),
                even,
                true,
                true,
            );
        }
    }
}

#[test]
fn arithmetic_division_retains_actual_remainder_and_increment_for_each_precision() {
    for (pc, p) in precisions() {
        // 2^54/3 has remainder1 (<half); 2^65/3 has remainder2 (>half).
        // The old value-only div_at discarded whether this increment happened.
        let truncated = if p == 53 {
            0x15555555555555
        } else {
            0xaaaaaaaaaaaaaaaa
        };
        for negative in [false, true] {
            for mode in MODES {
                let incremented = directed_increment(mode, negative, p == 64);
                let expected = dyadic(
                    negative,
                    truncated + u128::from(incremented),
                    -(p as i32) - 1,
                );
                check(
                    ext80_div_info(dyadic(negative, 1, 0), dyadic(false, 3, 0), pc, mode).unwrap(),
                    expected,
                    true,
                    incremented,
                );
                check(
                    ext80_div_info(dyadic(negative, 6, 0), dyadic(false, 3, 0), pc, mode).unwrap(),
                    dyadic(negative, 2, 0),
                    false,
                    false,
                );
            }
        }
    }
}

#[test]
fn arithmetic_subtraction_and_zero_signs_follow_explicit_rounding_without_false_inexact() {
    for (pc, p) in precisions() {
        for mode in MODES {
            let one = dyadic(false, 1, 0);
            check(
                ext80_sub_info(one, one, pc, mode).unwrap(),
                F128::zero(mode == Ext80Rounding::Down),
                false,
                false,
            );
            check(
                ext80_add_info(F128::zero(false), F128::zero(true), pc, mode).unwrap(),
                F128::zero(mode == Ext80Rounding::Down),
                false,
                false,
            );
            check(
                ext80_add_info(F128::zero(true), F128::zero(true), pc, mode).unwrap(),
                F128::zero(true),
                false,
                false,
            );
            check(
                ext80_mul_info(F128::zero(true), one, pc, mode).unwrap(),
                F128::zero(true),
                false,
                false,
            );
            check(
                ext80_div_info(F128::zero(true), one.neg(), pc, mode).unwrap(),
                F128::zero(false),
                false,
                false,
            );
            for negative in [false, true] {
                let a = dyadic(negative, 1, 0);
                let b = dyadic(negative, 1, -(p as i32) - 2);
                // Just below |1| has a different quantum from just above1.
                let previous_one = dyadic(negative, (1u128 << p) - 1, -(p as i32));
                let incremented = directed_increment(mode, negative, true);
                check(
                    ext80_sub_info(a, b, pc, mode).unwrap(),
                    if incremented { a } else { previous_one },
                    true,
                    incremented,
                );
            }
        }
    }
}

#[test]
fn arithmetic_multiply_reports_its_own_low_product_tail_and_directed_sign() {
    for (pc, p) in precisions() {
        let significand = (1u128 << (p - 1)) + 1;
        for negative in [false, true] {
            for mode in MODES {
                // (1+u)^2 =1+2u+u²; 0<u²<half an ulp at both precisions.
                let incremented = directed_increment(mode, negative, false);
                let expected = dyadic(
                    negative,
                    (1u128 << (p - 1)) + 2 + u128::from(incremented),
                    1 - p as i32,
                );
                check(
                    ext80_mul_info(
                        dyadic(negative, significand, 1 - p as i32),
                        dyadic(false, significand, 1 - p as i32),
                        pc,
                        mode,
                    )
                    .unwrap(),
                    expected,
                    true,
                    incremented,
                );
                check(
                    ext80_mul_info(dyadic(negative, 3, -1), dyadic(false, 2, 0), pc, mode).unwrap(),
                    dyadic(negative, 3, 0),
                    false,
                    false,
                );
            }
        }
    }
}

#[test]
fn arithmetic_extended_exponents_are_retained_and_unsupported_domains_return_no_result_record() {
    for (pc, p) in precisions() {
        for mode in MODES {
            let low = dyadic(false, 1, -8190);
            let high = dyadic(false, 1, 8190);
            check(
                ext80_mul_info(low, low, pc, mode).unwrap(),
                dyadic(false, 1, -16380),
                false,
                false,
            );
            check(
                ext80_mul_info(high, high, pc, mode).unwrap(),
                dyadic(false, 1, 16380),
                false,
                false,
            );
            check(
                ext80_div_info(low, high, pc, mode).unwrap(),
                dyadic(false, 1, -16380),
                false,
                false,
            );
            // The largest admitted operands retain all64 loaded bits. Rounding
            // their product at PC53 can carry into top16382, still normal80.
            let maximal = dyadic(false, (1u128 << 64) - 1, 8127);
            let (expected, incremented) = if p == 53 {
                let inc = matches!(mode, Ext80Rounding::NearestEven | Ext80Rounding::Up);
                (
                    if inc {
                        dyadic(false, 1, 16382)
                    } else {
                        dyadic(false, (1u128 << 53) - 1, 16329)
                    },
                    inc,
                )
            } else {
                let inc = mode == Ext80Rounding::Up;
                (
                    dyadic(false, (1u128 << 64) - 2 + u128::from(inc), 16318),
                    inc,
                )
            };
            check(
                ext80_mul_info(maximal, maximal, pc, mode).unwrap(),
                expected,
                true,
                incremented,
            );
            assert_eq!(
                ext80_add_info(dyadic(false, 1, 8191), low, pc, mode),
                Err(ArithmeticError::ExponentDomain)
            );
            assert_eq!(
                ext80_div_info(low, F128::zero(false), pc, mode),
                Err(ArithmeticError::DivisionByZero)
            );
            assert_eq!(
                ext80_mul_info(
                    F128 {
                        hi: 0x7fff000000000000,
                        lo: 0
                    },
                    low,
                    pc,
                    mode
                ),
                Err(ArithmeticError::NonFinite)
            );
            // 1+2^-100 is a binary128 value, not an exactly loaded Ext80 value.
            assert_eq!(
                ext80_add_info(dyadic(false, (1u128 << 100) + 1, -100), low, pc, mode),
                Err(ArithmeticError::UnsupportedEncoding)
            );
        }
    }
}

// Root alone generates these fixed tables with the independently authored
// Fraction oracle, then freezes/pins them before compiling this test module.
include!("finite_pc64_arithmetic_info_fraction_cases.rs");
include!("finite_pc64_arithmetic_info_fraction_errors.rs");

fn oracle_call(
    op: &str,
    pc: u32,
    rc: &str,
    a: F128,
    b: F128,
) -> Result<Rounded<F128>, ArithmeticError> {
    let precision = match pc {
        53 => Ext80Precision::Pc53,
        64 => Ext80Precision::Pc64,
        _ => panic!("unadmitted independent precision"),
    };
    let mode = match rc {
        "nearest" => Ext80Rounding::NearestEven,
        "down" => Ext80Rounding::Down,
        "up" => Ext80Rounding::Up,
        "zero" => Ext80Rounding::ToZero,
        _ => panic!("unadmitted independent rounding"),
    };
    match op {
        "add" => ext80_add_info(a, b, precision, mode),
        "sub" => ext80_sub_info(a, b, precision, mode),
        "mul" => ext80_mul_info(a, b, precision, mode),
        "div" => ext80_div_info(a, b, precision, mode),
        _ => panic!("unadmitted independent operation"),
    }
}
#[test]
fn independent_fraction_matrix_pins_all_arithmetic_values_and_operation_metadata() {
    assert_eq!(ARITHMETIC_INFO_CASES.len(), 2368);
    assert_eq!(ARITHMETIC_INFO_ERROR_CASES.len(), 192);
    for (index, &(op, pc, rc, ahi, alo, bhi, blo, rhi, rlo, inexact, incremented)) in
        ARITHMETIC_INFO_CASES.iter().enumerate()
    {
        assert_eq!(
            oracle_call(
                op,
                pc,
                rc,
                F128 { hi: ahi, lo: alo },
                F128 { hi: bhi, lo: blo }
            ),
            Ok(Rounded {
                value: F128 { hi: rhi, lo: rlo },
                inexact,
                magnitude_rounded_up: incremented
            }),
            "independentFraction success row{index} {op} PC{pc} RC{rc}"
        );
    }
    for (index, &(op, pc, rc, ahi, alo, bhi, blo, error)) in
        ARITHMETIC_INFO_ERROR_CASES.iter().enumerate()
    {
        let expected = match error {
            "NonFinite" => ArithmeticError::NonFinite,
            "UnsupportedEncoding" => ArithmeticError::UnsupportedEncoding,
            "DivisionByZero" => ArithmeticError::DivisionByZero,
            "Overflow" => ArithmeticError::Overflow,
            "ExponentDomain" => ArithmeticError::ExponentDomain,
            _ => panic!("unadmitted independent error"),
        };
        assert_eq!(
            oracle_call(
                op,
                pc,
                rc,
                F128 { hi: ahi, lo: alo },
                F128 { hi: bhi, lo: blo }
            ),
            Err(expected),
            "independentFraction error row{index} {op} PC{pc} RC{rc}"
        );
    }
}
