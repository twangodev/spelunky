//! Integer-only standalone x87 `FSIN`/`FCOS` profile used by the classic
//! collision sampler.
//!
//! This is a semantic port of the Bochs post-3.1 standalone transcendental
//! path represented by `bochs/cpu/fpu/fsincos.cc` and the generated
//! `fsincos_constants.h` snapshot through 2026-09-30. The profile uses the
//! legacy 66-bit Pi/2 reduction constant, the standalone FSIN/FCOS polynomial
//! paths, Bochs' 67-bit chopped intermediates, and its 113-bit-to-64-bit
//! nearest-even closing round. It is intentionally narrower than a complete
//! x87 emulator: finite normal inputs in the source sampler's +/-360-degree
//! range are supported; status flags and exception behavior are not.
//!
//! The arithmetic and coefficient provenance, including their license terms,
//! is isolated in this directory. This profile is deterministic across Rust
//! targets, but it does not promise identical last bits on every physical x87
//! implementation.

use crate::finite_pc64::{self as f, ArithmeticError, Ext80, F128};

pub(crate) const PROFILE_ID: &str = "BochsPost31StandaloneFSIN_FCOS_2026_09_30_v1";

// x87's legacy 66-bit Pi/2 approximation, represented as a 128-bit fixed-point
// fraction in [1, 2), as in Bochs fpu_constant.h (without BETTER_THAN_PENTIUM).
const PI2_FIXED: u128 = (0xc90f_daa2_2168_c234_u128 << 64) | 0xc000_0000_0000_0000_u128;

const ONE: F128 = F128::from_words(0x3fff_0000_0000_0000, 0);

const A: [F128; 6] = [
    F128::from_words(0xbffc_5555_5555_5555, 0x5555_4000_0000_0000),
    F128::from_words(0x3ff8_1111_1111_1111, 0x0f8d_4000_0000_0000),
    F128::from_words(0xbff2_a01a_01a0_19dc, 0xf1dd_0000_0000_0000),
    F128::from_words(0x3fec_71de_3a53_d425, 0x5b5c_0000_0000_0000),
    F128::from_words(0xbfe5_ae64_448b_be81, 0x4cbc_0000_0000_0000),
    F128::from_words(0x3fde_60c0_fc09_8535, 0x7936_0000_0000_0000),
];

const B: [F128; 6] = [
    F128::from_words(0xbffd_ffff_ffff_ffff, 0xffff_8000_0000_0000),
    F128::from_words(0x3ffa_5555_5555_5555, 0x509d_c000_0000_0000),
    F128::from_words(0xbff5_6c16_c16c_1686, 0x286e_8000_0000_0000),
    F128::from_words(0x3fef_a01a_019d_6d56, 0x7cb3_c000_0000_0000),
    F128::from_words(0xbfe9_27e4_ebd8_7d4d, 0x24c0_0000_0000_0000),
    F128::from_words(0x3fe2_1e93_c920_d453, 0x06be_0000_0000_0000),
];

const U: [F128; 4] = [
    F128::from_words(0xbffc_5555_5555_5555, 0x5511_4000_0000_0000),
    F128::from_words(0x3ff8_1111_1111_10e8, 0xff6d_8000_0000_0000),
    F128::from_words(0xbff2_a01a_0182_c811, 0x3a6b_8000_0000_0000),
    F128::from_words(0x3fec_71d5_c5b9_83cc, 0x8490_0000_0000_0000),
];

const V: [F128; 4] = [
    F128::from_words(0xbffd_ffff_ffff_ffff, 0xfe8a_0000_0000_0000),
    F128::from_words(0x3ffa_5555_5555_54e7, 0x3eb9_8000_0000_0000),
    F128::from_words(0xbff5_6c16_c143_cc6c, 0xb9c4_c000_0000_0000),
    F128::from_words(0x3fef_a00e_6235_5b8e, 0xffc4_0000_0000_0000),
];

const SIN_C: [F128; 8] = [
    F128::from_words(0x3ffe_110d_0c4b_69c3, 0xb764_8000_0000_0000),
    F128::from_words(0x3ffe_44eb_381c_f386, 0xab04_c000_0000_0000),
    F128::from_words(0x3ffe_73b7_680d_ea57, 0x7dbb_8000_0000_0000),
    F128::from_words(0x3ffe_9cb6_a9bb_ce64, 0xad61_8000_0000_0000),
    F128::from_words(0x3ffd_1c37_d64c_6b87, 0x6518_0000_0000_0000),
    F128::from_words(0x3ffd_591b_c9fa_2f59, 0x72f9_0000_0000_0000),
    F128::from_words(0x3ffd_94a6_be9f_546c, 0x4a58_c000_0000_0000),
    F128::from_words(0x3ffd_ce9d_2e3d_4a51, 0xeda0_8000_0000_0000),
];

const COS_C: [F128; 8] = [
    F128::from_words(0x3ffe_b11d_0416_2a4c, 0x623b_c000_0000_0000),
    F128::from_words(0x3ffe_8bb1_05a5_dc90, 0x0619_0000_0000_0000),
    F128::from_words(0x3ffe_6018_526f_563d, 0xf51b_4000_0000_0000),
    F128::from_words(0x3ffe_2f01_1326_420e, 0x431c_8000_0000_0000),
    F128::from_words(0x3ffe_ebe2_14f7_6efa, 0x7bf4_0000_0000_0000),
    F128::from_words(0x3ffe_e20b_f49a_cd6c, 0x0f4d_0000_0000_0000),
    F128::from_words(0x3ffe_d653_f073_e403, 0xfa27_8000_0000_0000),
    F128::from_words(0x3ffe_c8c5_bf8c_e1a8, 0x4356_8000_0000_0000),
];

#[derive(Clone, Copy)]
struct Reduced {
    absolute: F128,
    negative: bool,
    quadrant: u8,
}

#[derive(Clone, Copy, Default)]
struct U192 {
    hi: u64,
    mid: u64,
    lo: u64,
}

impl U192 {
    fn shifted(value: u64, shift: u32) -> Self {
        let word = (shift / 64) as usize;
        let bits = shift % 64;
        let mut result = Self::default();
        if bits == 0 {
            match word {
                0 => result.lo = value,
                1 => result.mid = value,
                _ => result.hi = value,
            }
        } else {
            let low = value << bits;
            let high = value >> (64 - bits);
            match word {
                0 => {
                    result.lo = low;
                    result.mid = high;
                }
                1 => {
                    result.mid = low;
                    result.hi = high;
                }
                _ => unreachable!("supported angle range bounds the fixed shift"),
            }
        }
        result
    }

    fn ge_u128(self, other: u128) -> bool {
        self.hi != 0
            || self.mid > (other >> 64) as u64
            || (self.mid == (other >> 64) as u64 && self.lo >= other as u64)
    }

    fn sub_u128(self, other: u128) -> Self {
        let (lo, borrow_lo) = self.lo.overflowing_sub(other as u64);
        let (mid, borrow_mid_1) = self.mid.overflowing_sub((other >> 64) as u64);
        let (mid, borrow_mid_2) = mid.overflowing_sub(u64::from(borrow_lo));
        let (hi, borrow_hi) = self
            .hi
            .overflowing_sub(u64::from(borrow_mid_1 || borrow_mid_2));
        debug_assert!(!borrow_hi);
        Self { hi, mid, lo }
    }

    fn low_u128(self) -> u128 {
        debug_assert_eq!(self.hi, 0);
        (u128::from(self.mid) << 64) | u128::from(self.lo)
    }
}

/// Return the standalone x87 `FSIN`/`FCOS` pair for an exact binary80 angle,
/// then model the source's separate stores to IEEE binary64 QWORDs.
pub(crate) fn qword_sin_cos(radians: F128) -> Result<(f64, f64), ArithmeticError> {
    let cosine_input = f::to_ext80_nearest(radians)?;
    let mut sine_input = cosine_input;
    sine_input.sign_exp ^= 0x8000;

    // The source invokes FSIN with the negated degree-to-radian result. FCOS
    // is evaluated independently and is even, so its sign is immaterial to
    // this profile (the native oracle uses the positive argument).
    let sine_ext = standalone(sine_input, false)?;
    let cosine_ext = standalone(cosine_input, true)?;
    let sine = f::to_f64_nearest(f::from_ext80(sine_ext)?)?;
    let cosine = f::to_f64_nearest(f::from_ext80(cosine_ext)?)?;
    Ok((sine, cosine))
}

fn standalone(mut input: Ext80, cosine: bool) -> Result<Ext80, ArithmeticError> {
    let negative = input.sign_exp >> 15 != 0;
    input.sign_exp &= 0x7fff;
    let exponent = i32::from(input.sign_exp & 0x7fff);
    let mut significand = input.significand;
    if exponent == 0x7fff || significand == 0 && exponent != 0 {
        return Err(ArithmeticError::UnsupportedEncoding);
    }
    if significand == 0 {
        return Ok(if cosine {
            Ext80 {
                sign_exp: 0x3fff,
                significand: 1_u64 << 63,
            }
        } else {
            Ext80 {
                sign_exp: u16::from(negative) << 15,
                significand: 0,
            }
        });
    }
    // The sampler's finite angle range cannot produce x87 subnormal inputs,
    // but normalizing here keeps the helper's finite domain explicit.
    let exponent = if exponent == 0 {
        if significand >> 63 != 0 {
            1
        } else {
            let shift = significand.leading_zeros();
            significand <<= shift;
            1 - shift as i32
        }
    } else {
        exponent
    };
    let exponent_delta = exponent - 16_383;
    if exponent_delta >= 63 {
        return Err(ArithmeticError::ExponentDomain);
    }
    if exponent_delta < -68 {
        return Ok(if cosine {
            Ext80 {
                sign_exp: 0x3fff,
                significand: 1_u64 << 63,
            }
        } else {
            Ext80 {
                sign_exp: (u16::from(negative) << 15) | exponent as u16,
                significand,
            }
        });
    }

    let reduced = reduce(significand, exponent)?;
    let result = standalone_core(
        reduced.absolute,
        reduced.negative,
        reduced.quadrant,
        u8::from(cosine),
        u8::from(negative && !cosine),
    )?;
    Ok(result)
}

fn reduce(significand: u64, exponent: i32) -> Result<Reduced, ArithmeticError> {
    let exponent_delta = exponent - 16_383;
    if exponent_delta < -1 {
        let angle = f::from_ext80(Ext80 {
            sign_exp: exponent as u16,
            significand,
        })?;
        return Ok(Reduced {
            absolute: angle,
            negative: false,
            quadrant: 0,
        });
    }

    // The supported source range ends at 2*pi, so the largest exponent delta
    // is two. The quotient is at most four; repeated exact fixed-point
    // subtraction is equivalent to Bochs' bit-serial remainder here.
    if exponent_delta > 2 {
        return Err(ArithmeticError::ExponentDomain);
    }
    let shift = (exponent_delta + 64) as u32;
    let mut remainder = U192::shifted(significand, shift);
    let mut quadrant = 0_u8;
    while remainder.ge_u128(PI2_FIXED) {
        remainder = remainder.sub_u128(PI2_FIXED);
        quadrant = quadrant.wrapping_add(1);
        if quadrant > 8 {
            return Err(ArithmeticError::ExponentDomain);
        }
    }
    let mut remainder = remainder.low_u128();
    let pi4_fixed = PI2_FIXED >> 1;
    let mut negative_remainder = false;
    if remainder > pi4_fixed {
        remainder = PI2_FIXED - remainder;
        negative_remainder = true;
        quadrant = quadrant.wrapping_add(1);
    }
    let absolute = f::from_scaled_u128(false, remainder, -127)?;
    Ok(Reduced {
        absolute,
        negative: negative_remainder,
        quadrant: quadrant & 3,
    })
}

fn standalone_core(
    rabs: F128,
    r_negative: bool,
    quadrant: u8,
    function_offset: u8,
    input_negative: u8,
) -> Result<Ext80, ArithmeticError> {
    let n = (quadrant + function_offset) & 3;
    let sine_lane = n & 1 == 0;
    let quadrant_sign = (n >> 1) & 1;
    let sign = input_negative ^ quadrant_sign;
    let sign_with_fold = sign ^ u8::from(r_negative);

    let exponent = ((rabs.hi >> 48) & 0x7fff) as i32;
    if exponent >= 16_383 - 2 {
        let (c, index) = get_c(rabs);
        let mut sigma = SIN_C[index];
        let mut gamma = COS_C[index];
        let d = f::add67_chop(rabs, c.neg())?;
        let (mut p, q) = sincos_d(d)?;
        if sine_lane {
            sigma = set_sign(sigma, sign_with_fold != 0);
            gamma = set_sign(gamma, sign_with_fold != 0);
            let f = f::mul67_chop(sigma, q)?;
            let g = f::mul67_chop(gamma, p)?;
            finish(sigma, f, g)
        } else {
            sigma = set_sign(sigma, sign != 0);
            gamma = set_sign(gamma, sign != 0);
            p = p.neg();
            let f = f::mul67_chop(sigma, p)?;
            let g = f::mul67_chop(gamma, q)?;
            finish(gamma, f, g)
        }
    } else {
        let s = f::mul_e67(rabs, rabs)?;
        let t = f::mul_e67(s, s)?;
        if sine_lane {
            let rr = set_sign(rabs, sign_with_fold != 0);
            let mut p = f::mul67_chop(t, A[4])?;
            p = f::add64_ne(p, A[2])?;
            p = f::mul67_chop(t, p)?;
            p = f::add64_ne(p, A[0])?;
            p = f::mul67_chop(s, p)?;
            let mut q = f::mul67_chop(t, A[5])?;
            q = f::add64_ne(q, A[3])?;
            q = f::mul67_chop(t, q)?;
            q = f::add64_ne(q, A[1])?;
            q = f::mul67_chop(t, q)?;
            let correction = f::add64_ne(p, q)?;
            let correction = f::mul67_chop(rr, correction)?;
            f::to_ext80_nearest(f::add113_nearest(rr, correction)?)
        } else {
            let mut p = f::mul67_chop(t, B[4])?;
            p = f::add64_ne(p, B[2])?;
            p = f::mul67_chop(t, p)?;
            p = f::add64_ne(p, B[0])?;
            p = f::mul67_chop(s, p)?;
            let mut q = f::mul67_chop(t, B[5])?;
            q = f::add64_ne(q, B[3])?;
            q = f::mul67_chop(t, q)?;
            q = f::add64_ne(q, B[1])?;
            q = f::mul67_chop(t, q)?;
            let correction = f::add67_chop(p, q)?;
            let one = set_sign(ONE, sign != 0);
            let correction = if sign != 0 {
                correction.neg()
            } else {
                correction
            };
            f::to_ext80_nearest(f::add113_nearest(one, correction)?)
        }
    }
}

fn get_c(rabs: F128) -> (F128, usize) {
    let exponent = (rabs.hi >> 48) & 0x7fff;
    let b1 = (rabs.hi >> 47) & 1;
    let b2 = (rabs.hi >> 46) & 1;
    let index = (((exponent & 1) << 2) | (b1 << 1) | b2) as usize;
    let c = F128::from_words((rabs.hi & !((1_u64 << 45) - 1)) | (1_u64 << 45), 0);
    (c, index)
}

fn sincos_d(d: F128) -> Result<(F128, F128), ArithmeticError> {
    let e = f::mul67_chop(d, d)?;
    let mut p1 = U[3];
    p1 = f::add64_ne(f::mul67_chop(e, p1)?, U[2])?;
    p1 = f::add64_ne(f::mul67_chop(e, p1)?, U[1])?;
    p1 = f::add64_ne(f::mul67_chop(e, p1)?, U[0])?;
    p1 = f::mul67_chop(e, p1)?;
    let p = f::add64_ne(d, f::mul67_chop(p1, d)?)?;

    let mut q1 = V[3];
    q1 = f::add64_ne(f::mul67_chop(e, q1)?, V[2])?;
    q1 = f::add64_ne(f::mul67_chop(e, q1)?, V[1])?;
    q1 = f::add64_ne(f::mul67_chop(e, q1)?, V[0])?;
    let q = f::mul64_ne(e, q1)?;
    Ok((p, q))
}

fn finish(head: F128, f_term: F128, g_term: F128) -> Result<Ext80, ArithmeticError> {
    let tail = f::add67_chop(f_term, g_term)?;
    f::to_ext80_nearest(f::add113_nearest(head, tail)?)
}

fn set_sign(value: F128, negative: bool) -> F128 {
    F128::from_words(
        (value.hi & !(1_u64 << 63)) | (u64::from(negative) << 63),
        value.lo,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn radians_from_degrees(degrees: f64) -> F128 {
        let degrees = f::from_f64(degrees).unwrap();
        let pi = f::from_ext80(Ext80 {
            sign_exp: 0x4000,
            significand: 0xc90f_daa2_2168_c235,
        })
        .unwrap();
        let product = f::mul64_ne(degrees, pi).unwrap();
        f::div64_ne(product, f::from_f64(180.0).unwrap()).unwrap()
    }

    #[test]
    fn standalone_profile_matches_the_measured_rotated_drip_sine() {
        assert_eq!(PROFILE_ID, "BochsPost31StandaloneFSIN_FCOS_2026_09_30_v1");
        let (sine, cosine) = qword_sin_cos(radians_from_degrees(35.9437)).unwrap();
        assert_eq!(sine.to_bits(), 0xbfe2_c89f_471e_63fc);
        assert_eq!(cosine.to_bits(), 0x3fe9_e832_0a43_2f5e);
    }

    #[test]
    fn standalone_profile_matches_native_quadrant_and_near_quadrant_vectors() {
        let vectors = [
            (90.0, 0xbff0_0000_0000_0000, 0xbbe0_0000_0000_0000),
            (
                f64::from_bits(90.0_f64.to_bits() + 1),
                0xbff0_0000_0000_0000,
                0xbcb1_e080_0000_0000,
            ),
            (
                f64::from_bits(90.0_f64.to_bits() - 1),
                0xbff0_0000_0000_0000,
                0x3cb1_dd80_0000_0000,
            ),
            (180.0, 0x3bf0_0000_0000_0000, 0xbff0_0000_0000_0000),
            (-180.0, 0xbbf0_0000_0000_0000, 0xbff0_0000_0000_0000),
            (
                216.86989764584402,
                0x3fe3_3333_3333_3333,
                0xbfe9_9999_9999_999a,
            ),
            (-35.9437, 0x3fe2_c89f_471e_63fc, 0x3fe9_e832_0a43_2f5e),
            (
                359.99999999999994,
                0x3cd1_dd80_0000_0000,
                0x3ff0_0000_0000_0000,
            ),
        ];
        for (degrees, expected_sine, expected_cosine) in vectors {
            let (sine, cosine) = qword_sin_cos(radians_from_degrees(degrees)).unwrap();
            assert_eq!(sine.to_bits(), expected_sine, "{degrees} degrees FSIN");
            assert_eq!(cosine.to_bits(), expected_cosine, "{degrees} degrees FCOS");
        }
    }

    #[test]
    fn standalone_profile_preserves_fsin_negative_zero() {
        let (sine, cosine) = qword_sin_cos(radians_from_degrees(0.0)).unwrap();
        assert_eq!(sine.to_bits(), 0x8000_0000_0000_0000);
        assert_eq!(cosine.to_bits(), 0x3ff0_0000_0000_0000);
    }
}
