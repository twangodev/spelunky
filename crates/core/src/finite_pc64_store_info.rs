// Exact binary64 store conversion and rounding facts. This is numerical data,
// not an x87 status/mask/stack/retirement implementation. `value` is the masked
// destination response; consumers decide whether that response may retire.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StoreInputClass {
    Zero,
    Finite,
    Infinity,
    QuietNaN,
    SignalingNaN,
    UnsupportedEncoding,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StoreRounded<T> {
    /// Masked destination response. For invalid encodings this is x87's
    /// negative quiet indefinite; signaling NaNs are quieted with payload.
    pub value: T,
    pub input_class: StoreInputClass,
    /// Precision loss in the masked destination response, including overflow.
    /// NaN payload truncation is not a numerical precision exception.
    pub inexact: bool,
    /// Masked destination magnitude is larger than the exact input magnitude.
    /// This differs from the internal increment when overflow saturates.
    pub magnitude_rounded_up: bool,
    /// Discarded tail when rounding to53 bits with an UNBOUNDED exponent.
    /// Needed separately for an unmasked overflow/underflow response.
    pub precision_inexact: bool,
    pub precision_magnitude_rounded_up: bool,
    /// The53-bit rounded result exceeds the maximum binary64 exponent.
    /// An already infinite operand is not a numerical overflow.
    pub overflow: bool,
    pub overflow_to_infinity: bool,
    /// Nonzero exact finite input is below the minimum normal binary64.
    pub tiny_before_rounding: bool,
    /// Nonzero53-bit result is tiny before exponent-range/subnormal rounding.
    /// This can be TRUE even when the final destination becomes min-normal.
    pub tiny_after_precision_rounding: bool,
    /// Nonzero finite input rounds below min-normal in the destination; this
    /// includes a rounded zero. An originally exact zero is not tiny.
    pub tiny_after_rounding: bool,
    pub rounded_to_normal: bool,
    /// Signaling NaN or unsupported raw extended encoding. The response above
    /// is not permission to retire when the invalid exception is unmasked.
    pub invalid: bool,
}

impl<T> StoreRounded<T> {
    /// x87 conversion's masked-underflow classification. Tininess is tested
    /// before denormalization, separately from the final destination exponent.
    pub fn masked_underflow(&self) -> bool {
        self.tiny_after_precision_rounding && self.inexact
    }
    /// Underflow detection before choosing the masked or unmasked response.
    /// An exact tiny value still raises underflow when that exception is
    /// unmasked; precision then depends on `precision_inexact` instead.
    pub fn unmasked_underflow(&self) -> bool {
        self.tiny_after_precision_rounding
    }
}

fn store_special(value: u64, class: StoreInputClass, invalid: bool) -> StoreRounded<u64> {
    StoreRounded {
        value,
        input_class: class,
        inexact: false,
        magnitude_rounded_up: false,
        precision_inexact: false,
        precision_magnitude_rounded_up: false,
        overflow: false,
        overflow_to_infinity: false,
        tiny_before_rounding: false,
        tiny_after_precision_rounding: false,
        tiny_after_rounding: false,
        rounded_to_normal: false,
        invalid,
    }
}

#[derive(Clone, Copy)]
struct StoreInteger {
    magnitude: u128,
    inexact: bool,
    incremented: bool,
}

// The exact finite input is a nonnegative integer times a power of two. Its
// significand has at most113 bits, while both target grids require at most53
// significant integer bits. Long tiny shifts retain sticky information rather
// than shift a machine word by its width or allocate an enormous zero prefix.
fn store_integer(
    significand: u128,
    exponent: i32,
    quantum: i32,
    negative: bool,
    mode: Ext80Rounding,
) -> Result<StoreInteger, ArithmeticError> {
    let shift = quantum - exponent;
    if shift <= 0 {
        let count = u32::try_from(-shift).map_err(|_| ArithmeticError::ExponentDomain)?;
        let factor = 1u128
            .checked_shl(count)
            .ok_or(ArithmeticError::ExponentDomain)?;
        let magnitude = significand
            .checked_mul(factor)
            .ok_or(ArithmeticError::ExponentDomain)?;
        return Ok(StoreInteger {
            magnitude,
            inexact: false,
            incremented: false,
        });
    }
    let shift = u32::try_from(shift).map_err(|_| ArithmeticError::ExponentDomain)?;
    let magnitude = if shift >= 128 {
        0
    } else {
        significand >> shift
    };
    let inexact = if shift >= 128 {
        significand != 0
    } else {
        significand & ((1u128 << shift) - 1) != 0
    };
    let half_order = if shift > 128 {
        Ordering::Less
    } else {
        let half = 1u128 << (shift - 1);
        let remainder = if shift == 128 {
            significand
        } else {
            significand & ((1u128 << shift) - 1)
        };
        remainder.cmp(&half)
    };
    let incremented =
        arithmetic_info_increment(mode, negative, inexact, half_order, magnitude & 1 != 0);
    Ok(StoreInteger {
        magnitude: magnitude + u128::from(incremented),
        inexact,
        incremented,
    })
}

fn store_finite(
    negative: bool,
    significand: u128,
    exponent: i32,
) -> Result<(i32, u64), ArithmeticError> {
    let width = i32::try_from(128 - significand.leading_zeros())
        .map_err(|_| ArithmeticError::ExponentDomain)?;
    if width == 0 {
        return Err(ArithmeticError::UnsupportedEncoding);
    }
    Ok((exponent + width - 1, u64::from(negative) << 63))
}

fn store_finite_info(
    negative: bool,
    significand: u128,
    exponent: i32,
    mode: Ext80Rounding,
) -> Result<StoreRounded<u64>, ArithmeticError> {
    if significand == 0 {
        return Ok(store_special(
            u64::from(negative) << 63,
            StoreInputClass::Zero,
            false,
        ));
    }
    let (top, sign) = store_finite(negative, significand, exponent)?;
    // First round ONLY precision with unlimited exponent. The x87 conversion
    // range checks use this result, before the separate subnormal quantization.
    let unbounded_quantum = top - 52;
    let precision = store_integer(significand, exponent, unbounded_quantum, negative, mode)?;
    let precision_top = top + i32::from(precision.magnitude == 1u128 << 53);
    let overflow = precision_top > 1023;
    let tiny_before_rounding = top < -1022;
    let tiny_after_precision_rounding = precision_top < -1022;
    if overflow {
        let to_infinity = match mode {
            Ext80Rounding::NearestEven => true,
            Ext80Rounding::Down => negative,
            Ext80Rounding::Up => !negative,
            Ext80Rounding::ToZero => false,
        };
        return Ok(StoreRounded {
            value: sign
                | if to_infinity {
                    0x7ff0_0000_0000_0000
                } else {
                    0x7fef_ffff_ffff_ffff
                },
            input_class: StoreInputClass::Finite,
            inexact: true,
            magnitude_rounded_up: to_infinity,
            precision_inexact: precision.inexact,
            precision_magnitude_rounded_up: precision.incremented,
            overflow: true,
            overflow_to_infinity: to_infinity,
            tiny_before_rounding: false,
            tiny_after_precision_rounding: false,
            tiny_after_rounding: false,
            rounded_to_normal: false,
            invalid: false,
        });
    }
    let mut quantum = unbounded_quantum.max(-1074);
    let destination = store_integer(significand, exponent, quantum, negative, mode)?;
    let mut integer = destination.magnitude;
    if integer == 1u128 << 53 {
        integer >>= 1;
        quantum += 1;
    }
    let magnitude = if integer < 1u128 << 52 {
        if quantum != -1074 {
            return Err(ArithmeticError::UnsupportedEncoding);
        }
        u64::try_from(integer).map_err(|_| ArithmeticError::ExponentDomain)?
    } else {
        let field = quantum + 52 + 1023;
        if !(1..=2046).contains(&field) || integer >= 1u128 << 53 {
            return Err(ArithmeticError::UnsupportedEncoding);
        }
        ((field as u64) << 52) | (integer as u64 - (1u64 << 52))
    };
    // This final-range fact is computed in addition to, never instead of, the
    // exact input/unbounded-precision facts above. Original zero was handled.
    let tiny_after_rounding = magnitude < 0x0010_0000_0000_0000;
    Ok(StoreRounded {
        value: sign | magnitude,
        input_class: StoreInputClass::Finite,
        inexact: destination.inexact,
        magnitude_rounded_up: destination.incremented,
        precision_inexact: precision.inexact,
        precision_magnitude_rounded_up: precision.incremented,
        overflow: false,
        overflow_to_infinity: false,
        tiny_before_rounding,
        tiny_after_precision_rounding,
        tiny_after_rounding,
        rounded_to_normal: tiny_before_rounding && !tiny_after_rounding,
        invalid: false,
    })
}

/// Direct IEEE binary128-container -> binary64 store response at the requested
/// RC. No arithmetic add-zero, host float conversion, CW mutation or metadata
/// reconstruction from the final QWORD. Finite binary128 subnormals are valid.
/// Quiet/signaling NaNs preserve the leading payload and sign while quieting;
/// infinity is exact and never mislabeled overflow.
pub fn store_qword_info(
    value: F128,
    mode: Ext80Rounding,
) -> Result<StoreRounded<u64>, ArithmeticError> {
    let field = (value.hi >> 48) & 0x7fff;
    if field == 0x7fff {
        let fraction = (u128::from(value.hi & 0x0000_ffff_ffff_ffff) << 64) | u128::from(value.lo);
        let sign = value.hi & (1u64 << 63);
        if fraction == 0 {
            return Ok(store_special(
                sign | 0x7ff0_0000_0000_0000,
                StoreInputClass::Infinity,
                false,
            ));
        }
        let signaling = value.hi & (1u64 << 47) == 0;
        let payload = ((value.hi & 0x0000_ffff_ffff_ffff) << 4) | (value.lo >> 60);
        return Ok(store_special(
            sign | 0x7ff8_0000_0000_0000 | payload,
            if signaling {
                StoreInputClass::SignalingNaN
            } else {
                StoreInputClass::QuietNaN
            },
            signaling,
        ));
    }
    let n = value.number()?;
    store_finite_info(n.negative, n.significand, n.exponent, mode)
}

/// Raw extended-register counterpart, including exceptional encodings before
/// `from_ext80` could reject them. Pseudo-denormals retain the field1 numerical
/// value; true denormals are finite. A nonzero exponent without the explicit
/// integer bit is unsupported and produces invalid + masked x87 indefinite.
/// This describes STORE math only, not FLD denormal/invalid operand retirement.
pub fn store_ext80_qword_info(
    value: Ext80,
    mode: Ext80Rounding,
) -> Result<StoreRounded<u64>, ArithmeticError> {
    let field = value.sign_exp & 0x7fff;
    if field != 0 && value.significand >> 63 == 0 {
        return Ok(store_special(
            0xfff8_0000_0000_0000,
            StoreInputClass::UnsupportedEncoding,
            true,
        ));
    }
    if field == 0x7fff {
        let sign = u64::from(value.sign_exp >> 15) << 63;
        let fraction = value.significand & 0x7fff_ffff_ffff_ffff;
        if fraction == 0 {
            return Ok(store_special(
                sign | 0x7ff0_0000_0000_0000,
                StoreInputClass::Infinity,
                false,
            ));
        }
        let signaling = value.significand & (1u64 << 62) == 0;
        return Ok(store_special(
            sign | 0x7ff8_0000_0000_0000 | (fraction >> 11),
            if signaling {
                StoreInputClass::SignalingNaN
            } else {
                StoreInputClass::QuietNaN
            },
            signaling,
        ));
    }
    store_finite_info(
        value.sign_exp >> 15 != 0,
        u128::from(value.significand),
        i32::from(field.max(1)) - 16383 - 63,
        mode,
    )
}

#[cfg(test)]
#[path = "finite_pc64_store_info_tests.rs"]
mod store_info_tests;
