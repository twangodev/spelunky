//! Portable QWORD results for the source's x87 `FPATAN` and `radtodeg` path.
//!
//! GameMaker passes the QWORD result of `FPATAN` into its `radtodeg` script.
//! The original x87 profile uses 64-bit significand precision and
//! round-to-nearest-even. This module evaluates atan with integer-backed
//! extended arithmetic, rounds it through Ext80, and models the script's
//! binary80 180/PI multiply before the QWORD store.

use crate::finite_pc64::{self as f, ArithmeticError, Ext80, F128};

const ONE: F128 = F128::from_words(0x3fff_0000_0000_0000, 0);
const HALF: F128 = F128::from_words(0x3ffe_0000_0000_0000, 0);
const SMALL_ATAN_LIMIT: f64 = f64::from_bits(0x3e40_0000_0000_0000); // 2^-27
const SMALL_ATAN_LIMIT_F128: F128 = F128::from_words(0x3e40_0000_0000_0000, 0);

const PI_EXT80: Ext80 = Ext80 {
    sign_exp: 0x4000,
    significand: 0xc90f_daa2_2168_c235,
};

const RAD_TO_DEG_EXT80: Ext80 = Ext80 {
    sign_exp: 0x4004,
    significand: 0xe52e_e0d3_1e0f_bdc3,
};

/// Return the source FPATAN result after its QWORD store.
pub(crate) fn atan_qword(value: f64) -> Result<f64, ArithmeticError> {
    if value.is_nan() {
        return Err(ArithmeticError::NonFinite);
    }
    if value.is_infinite() {
        return Ok(if value.is_sign_negative() {
            -std::f64::consts::FRAC_PI_2
        } else {
            std::f64::consts::FRAC_PI_2
        });
    }
    if value == 0.0 || value.abs() <= SMALL_ATAN_LIMIT {
        // For this interval atan(x) rounds to x at QWORD precision.
        return Ok(value);
    }

    let negative = value.is_sign_negative();
    let magnitude = f::from_f64(value.abs())?;
    let pi = f::from_ext80(PI_EXT80)?;
    let result = if value.abs() > 1.0 {
        let reciprocal = f::div113_nearest(ONE, magnitude)?;
        f::sub113_nearest(
            f::div113_nearest(pi, f::from_f64(2.0)?)?,
            atan_unit(reciprocal)?,
        )?
    } else {
        atan_unit(magnitude)?
    };
    let result = if negative { result.neg() } else { result };

    // FPATAN returns an extended result; the GML builtin then stores a QWORD.
    let ext80 = f::to_ext80_nearest(result)?;
    f::to_f64_nearest(f::from_ext80(ext80)?)
}

/// Match `radtodeg`: load the QWORD radians, multiply by the source's exact
/// binary80 constant at PC=64/RN, then store the QWORD result.
pub(crate) fn radtodeg_qword(radians: f64) -> Result<f64, ArithmeticError> {
    let radians = f::from_f64(radians)?;
    let factor = f::from_ext80(RAD_TO_DEG_EXT80)?;
    f::to_f64_nearest(f::mul64_ne(radians, factor)?)
}

fn atan_unit(value: F128) -> Result<F128, ArithmeticError> {
    if f::nearest_compare_finite(value, SMALL_ATAN_LIMIT_F128)? != core::cmp::Ordering::Greater {
        // Avoid an unnecessary series for reciprocals near the full QWORD
        // exponent range; the correction is far below a stored QWORD ulp.
        return Ok(value);
    }
    if f::nearest_compare_finite(value, HALF)? == core::cmp::Ordering::Greater {
        // atan(x) = PI/4 + atan((x - 1)/(x + 1)); the transformed argument
        // has magnitude at most 1/3, so the alternating series converges fast.
        let numerator = f::sub113_nearest(value, ONE)?;
        let denominator = f::add113_nearest(value, ONE)?;
        let reduced = f::div113_nearest(numerator, denominator)?;
        let pi = f::from_ext80(PI_EXT80)?;
        return f::add113_nearest(
            f::div113_nearest(pi, f::from_f64(4.0)?)?,
            atan_series(reduced)?,
        );
    }
    atan_series(value)
}

fn atan_series(value: F128) -> Result<F128, ArithmeticError> {
    if value.is_zero() {
        return Ok(value);
    }
    let square = f::mul113_nearest(value, value)?;
    let mut power = value;
    let mut sum = value;
    // The input magnitude is at most 1/2. After 64 added terms the analytic
    // remainder is below 2^-128; the 113-bit arithmetic error is also far
    // below a QWORD rounding boundary for the source's materialized inputs.
    for index in 1..=64 {
        power = f::mul113_nearest(power, square)?;
        let divisor = f::from_f64((2 * index + 1) as f64)?;
        let term = f::div113_nearest(power, divisor)?;
        sum = if index % 2 == 0 {
            f::add113_nearest(sum, term)?
        } else {
            f::sub113_nearest(sum, term)?
        };
    }
    Ok(sum)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qword_atan_and_radtodeg_match_the_original_x87_profile_vectors() {
        for (ratio, atan_bits, degree_bits) in [
            (0.2, 0x3fc9_4441_f8f7_260c, 0x4026_9eaf_781f_c537),
            (0.75, 0x3fe4_978f_a326_9ee1, 0x4042_6f58_ce59_e23c),
            (1.0, 0x3fe9_21fb_5444_2d18, 0x4046_8000_0000_0000),
            (0.025, 0x3f99_983c_34a7_74d2, 0x3ff6_e9dd_b03b_6ab3),
            (0.04, 0x3fa4_7816_2376_8d56, 0x4002_532b_5ba8_2c57),
            (0.08, 0x3fb4_6fbc_e9df_cc00, 0x4012_4bb2_03c7_0365),
            (0.5, 0x3fdd_ac67_0561_bb4f, 0x403a_90a7_31a6_1dc3),
            (1.5, 0x3fef_730b_d281_f69b, 0x404c_27ab_de07_f14e),
            (3.0, 0x3ff3_fc17_6b7a_8560, 0x4051_e429_cc69_8771),
        ] {
            let radians = atan_qword(ratio).unwrap();
            assert_eq!(radians.to_bits(), atan_bits, "atan({ratio})");
            assert_eq!(
                radtodeg_qword(radians).unwrap().to_bits(),
                degree_bits,
                "radtodeg(atan({ratio}))"
            );
        }
    }

    #[test]
    fn qword_atan_preserves_signed_zero_and_rounds_tiny_values_unchanged() {
        for value in [0.0, -0.0, SMALL_ATAN_LIMIT, -SMALL_ATAN_LIMIT] {
            assert_eq!(atan_qword(value).unwrap().to_bits(), value.to_bits());
        }
    }
}
