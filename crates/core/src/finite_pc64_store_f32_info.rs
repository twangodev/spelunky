// Direct binary32 store mathematics; no status, masks, stack or retirement engine.
// Reuses StoreRounded and the shared exact integer quantizer. For these APIs the
// precision_* fields describe UNBOUNDED 24-bit precision (rather than QWORD's
// 53-bit precision); the tiny thresholds are the binary32 normal boundary.
// The retained masked/unmasked underflow helpers remain numerical model facts.

fn store_f32_special(value: u32, class: StoreInputClass, invalid: bool) -> StoreRounded<u32> {
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

fn store_f32_finite_info(
    negative: bool,
    significand: u128,
    exponent: i32,
    mode: Ext80Rounding,
) -> Result<StoreRounded<u32>, ArithmeticError> {
    if significand == 0 {
        return Ok(store_f32_special(
            u32::from(negative) << 31,
            StoreInputClass::Zero,
            false,
        ));
    }
    let (top, _) = store_finite(negative, significand, exponent)?;
    let sign = u32::from(negative) << 31;
    // First round ONLY precision with unlimited exponent. The x87 conversion
    // range checks use this result, before the separate subnormal quantization.
    let unbounded_quantum = top - 23;
    let precision = store_integer(significand, exponent, unbounded_quantum, negative, mode)?;
    let precision_top = top + i32::from(precision.magnitude == 1u128 << 24);
    let overflow = precision_top > 127;
    let tiny_before_rounding = top < -126;
    let tiny_after_precision_rounding = precision_top < -126;
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
                    0x7f80_0000
                } else {
                    0x7f7f_ffff
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
    let mut quantum = unbounded_quantum.max(-149);
    let destination = store_integer(significand, exponent, quantum, negative, mode)?;
    let mut integer = destination.magnitude;
    if integer == 1u128 << 24 {
        integer >>= 1;
        quantum += 1;
    }
    let magnitude = if integer < 1u128 << 23 {
        if quantum != -149 {
            return Err(ArithmeticError::UnsupportedEncoding);
        }
        u32::try_from(integer).map_err(|_| ArithmeticError::ExponentDomain)?
    } else {
        let field = quantum + 23 + 127;
        if !(1..=254).contains(&field) || integer >= 1u128 << 24 {
            return Err(ArithmeticError::UnsupportedEncoding);
        }
        ((field as u32) << 23) | (integer as u32 - (1u32 << 23))
    };
    // This final-range fact is computed in addition to, never instead of, the
    // exact input/unbounded-precision facts above. Original zero was handled.
    let tiny_after_rounding = magnitude < 0x0080_0000;
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

/// Direct binary128-container -> binary32 store response at the requested RC.
/// The original exact input is rounded directly onto the binary32 destination
/// grid, never through QWORD. `precision_*` means unbounded24-bit precision.
/// Special payload truncation is not numerical inexact. No status is mutated.
pub fn store_f32_info(
    value: F128,
    mode: Ext80Rounding,
) -> Result<StoreRounded<u32>, ArithmeticError> {
    let field = (value.hi >> 48) & 0x7fff;
    if field == 0x7fff {
        let fraction = (u128::from(value.hi & 0x0000_ffff_ffff_ffff) << 64) | u128::from(value.lo);
        let sign = ((value.hi >> 63) as u32) << 31;
        if fraction == 0 {
            return Ok(store_f32_special(
                sign | 0x7f80_0000,
                StoreInputClass::Infinity,
                false,
            ));
        }
        let signaling = value.hi & (1u64 << 47) == 0;
        let payload = (fraction >> 89) as u32;
        return Ok(store_f32_special(
            sign | 0x7fc0_0000 | payload,
            if signaling {
                StoreInputClass::SignalingNaN
            } else {
                StoreInputClass::QuietNaN
            },
            signaling,
        ));
    }
    let n = value.number()?;
    store_f32_finite_info(n.negative, n.significand, n.exponent, mode)
}

/// Raw extended-register -> binary32 counterpart before any finite rejector.
/// True denormals and pseudo-denormals are numerical inputs; no FLD is modeled.
/// Unsupported nonzero-exponent/no-integer-bit encodings produce invalid and
/// the masked negative quiet indefinite; this response does not permit retire.
pub fn store_ext80_f32_info(
    value: Ext80,
    mode: Ext80Rounding,
) -> Result<StoreRounded<u32>, ArithmeticError> {
    let field = value.sign_exp & 0x7fff;
    if field != 0 && value.significand >> 63 == 0 {
        return Ok(store_f32_special(
            0xffc0_0000,
            StoreInputClass::UnsupportedEncoding,
            true,
        ));
    }
    if field == 0x7fff {
        let sign = u32::from(value.sign_exp >> 15) << 31;
        let fraction = value.significand & 0x7fff_ffff_ffff_ffff;
        if fraction == 0 {
            return Ok(store_f32_special(
                sign | 0x7f80_0000,
                StoreInputClass::Infinity,
                false,
            ));
        }
        let signaling = value.significand & (1u64 << 62) == 0;
        return Ok(store_f32_special(
            sign | 0x7fc0_0000 | (fraction >> 40) as u32,
            if signaling {
                StoreInputClass::SignalingNaN
            } else {
                StoreInputClass::QuietNaN
            },
            signaling,
        ));
    }
    store_f32_finite_info(
        value.sign_exp >> 15 != 0,
        u128::from(value.significand),
        i32::from(field.max(1)) - 16383 - 63,
        mode,
    )
}

#[cfg(test)]
#[path = "finite_pc64_store_f32_info_tests.rs"]
mod store_f32_info_tests;
