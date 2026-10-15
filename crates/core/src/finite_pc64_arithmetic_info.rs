// Successful finite x87 arithmetic value/rounding information, not VM status.
// Each exact integer/rational operation is rounded ONCE at explicit PC/RC.
// A later register/QWORD store cannot recover this operation's inexact tail.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ext80Precision {
    Pc53,
    Pc64,
}
impl Ext80Precision {
    fn bits(self) -> u32 {
        match self {
            Self::Pc53 => 53,
            Self::Pc64 => 64,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ext80Rounding {
    NearestEven,
    Down,
    Up,
    ToZero,
}

/// Conservative normal-Ext80 arithmetic domain. Both input magnitudes have
/// top exponents in[-8190,8190] and at most64 significant bits. Products and
/// ratios are normal Ext80; even exact cancellation has quantum >=-8253.
/// Consequently this capability needs no Ext80 underflow/overflow result or
/// status emulation. Wider/denormal-Ext80/non-register inputs fail explicitly.
fn arithmetic_info_operand(value: F128) -> Result<(), ArithmeticError> {
    let n = value.number()?;
    if n.significand == 0 {
        return Ok(());
    }
    let top = n.exponent + (128 - n.significand.leading_zeros()) as i32 - 1;
    if !(-8190..=8190).contains(&top) {
        return Err(ArithmeticError::ExponentDomain);
    }
    if to_ext80_nearest_info(value)?.inexact {
        return Err(ArithmeticError::UnsupportedEncoding);
    }
    Ok(())
}
fn arithmetic_info_result(value: F128) -> Result<(), ArithmeticError> {
    let n = value.number()?;
    if n.significand == 0 {
        return Ok(());
    }
    let top = n.exponent + (128 - n.significand.leading_zeros()) as i32 - 1;
    if !(-16382..=16383).contains(&top) {
        return Err(ArithmeticError::ExponentDomain);
    }
    if to_ext80_nearest_info(value)?.inexact {
        return Err(ArithmeticError::UnsupportedEncoding);
    }
    Ok(())
}
fn arithmetic_info_increment(
    mode: Ext80Rounding,
    negative: bool,
    inexact: bool,
    half_order: Ordering,
    odd: bool,
) -> bool {
    inexact
        && match mode {
            Ext80Rounding::NearestEven => {
                half_order == Ordering::Greater || half_order == Ordering::Equal && odd
            }
            Ext80Rounding::Down => negative,
            Ext80Rounding::Up => !negative,
            Ext80Rounding::ToZero => false,
        }
}
fn arithmetic_info_quantize(
    negative: bool,
    magnitude: Nat,
    exponent: i32,
    precision: Ext80Precision,
    mode: Ext80Rounding,
) -> Result<Rounded<F128>, ArithmeticError> {
    let quantum = if magnitude.is_zero() {
        MIN_QUAD_QUANTUM
    } else {
        exponent + magnitude.bits() as i32 - precision.bits() as i32
    };
    if quantum < MIN_QUAD_QUANTUM {
        return Err(ArithmeticError::ExponentDomain);
    }
    let shift = quantum - exponent;
    let q = if magnitude.is_zero() {
        Quantized {
            magnitude,
            exponent: quantum,
            inexact: false,
            incremented: false,
        }
    } else if shift <= 0 {
        Quantized {
            magnitude: magnitude.shl((-shift) as usize)?,
            exponent: quantum,
            inexact: false,
            incremented: false,
        }
    } else {
        let shift = shift as usize;
        let mut integer = magnitude.shr(shift);
        let inexact = magnitude.any_below(shift);
        let half_order = if !magnitude.bit(shift - 1) {
            Ordering::Less
        } else if magnitude.any_below(shift - 1) {
            Ordering::Greater
        } else {
            Ordering::Equal
        };
        let incremented =
            arithmetic_info_increment(mode, negative, inexact, half_order, integer.bit(0));
        if incremented {
            integer = integer.add(&Nat::from_u128(1));
        }
        Quantized {
            magnitude: integer,
            exponent: quantum,
            inexact,
            incremented,
        }
    };
    let rounded = pack_quad(negative, q)?;
    arithmetic_info_result(rounded.value)?;
    Ok(rounded)
}

/// `inexact` reports this addition's discarded nonzero exact tail.
/// `magnitude_rounded_up` reports incrementing the truncated magnitude, including
/// negative results. Neither field is a sticky flag, full C1 status or FSTP flag.
pub fn ext80_add_info(
    a: F128,
    b: F128,
    precision: Ext80Precision,
    mode: Ext80Rounding,
) -> Result<Rounded<F128>, ArithmeticError> {
    arithmetic_info_operand(a)?;
    arithmetic_info_operand(b)?;
    let a = a.number()?;
    let b = b.number()?;
    let exponent = a.exponent.min(b.exponent);
    let x = Nat::from_u128(a.significand).shl((a.exponent - exponent) as usize)?;
    let y = Nat::from_u128(b.significand).shl((b.exponent - exponent) as usize)?;
    let (negative, magnitude) = if a.negative == b.negative {
        (a.negative, x.add(&y))
    } else {
        match x.cmp(&y) {
            Ordering::Greater => (a.negative, x.sub(&y)),
            Ordering::Less => (b.negative, y.sub(&x)),
            // Exact cancellation has negative zero only for round-down.
            Ordering::Equal => (mode == Ext80Rounding::Down, Nat::from_u128(0)),
        }
    };
    arithmetic_info_quantize(negative, magnitude, exponent, precision, mode)
}
pub fn ext80_sub_info(
    a: F128,
    b: F128,
    precision: Ext80Precision,
    mode: Ext80Rounding,
) -> Result<Rounded<F128>, ArithmeticError> {
    ext80_add_info(a, b.neg(), precision, mode)
}
pub fn ext80_mul_info(
    a: F128,
    b: F128,
    precision: Ext80Precision,
    mode: Ext80Rounding,
) -> Result<Rounded<F128>, ArithmeticError> {
    arithmetic_info_operand(a)?;
    arithmetic_info_operand(b)?;
    let a = a.number()?;
    let b = b.number()?;
    arithmetic_info_quantize(
        a.negative ^ b.negative,
        Nat::from_u128(a.significand).mul(&Nat::from_u128(b.significand)),
        a.exponent + b.exponent,
        precision,
        mode,
    )
}
pub fn ext80_div_info(
    a: F128,
    b: F128,
    precision: Ext80Precision,
    mode: Ext80Rounding,
) -> Result<Rounded<F128>, ArithmeticError> {
    arithmetic_info_operand(a)?;
    arithmetic_info_operand(b)?;
    let a = a.number()?;
    let b = b.number()?;
    if b.significand == 0 {
        return Err(ArithmeticError::DivisionByZero);
    }
    let negative = a.negative ^ b.negative;
    if a.significand == 0 {
        return Ok(Rounded {
            value: F128::zero(negative),
            inexact: false,
            magnitude_rounded_up: false,
        });
    }
    let x = Nat::from_u128(a.significand);
    let y = Nat::from_u128(b.significand);
    let mut log_ratio = x.bits() as i32 - y.bits() as i32;
    let comparison = if log_ratio >= 0 {
        x.cmp(&y.shl(log_ratio as usize)?)
    } else {
        x.shl((-log_ratio) as usize)?.cmp(&y)
    };
    if comparison == Ordering::Less {
        log_ratio -= 1;
    }
    let exponent = a.exponent - b.exponent;
    let quantum = exponent + log_ratio - precision.bits() as i32 + 1;
    if quantum < MIN_QUAD_QUANTUM {
        return Err(ArithmeticError::ExponentDomain);
    }
    let shift = exponent - quantum;
    let numerator = if shift >= 0 {
        x.shl(shift as usize)?
    } else {
        x
    };
    let denominator = if shift < 0 {
        y.shl((-shift) as usize)?
    } else {
        y
    };
    let (mut quotient, remainder) = numerator.div_rem(&denominator)?;
    let inexact = !remainder.is_zero();
    let half_order = remainder.shl(1)?.cmp(&denominator);
    let incremented =
        arithmetic_info_increment(mode, negative, inexact, half_order, quotient.bit(0));
    if incremented {
        quotient = quotient.add(&Nat::from_u128(1));
    }
    let rounded = pack_quad(
        negative,
        Quantized {
            magnitude: quotient,
            exponent: quantum,
            inexact,
            incremented,
        },
    )?;
    arithmetic_info_result(rounded.value)?;
    Ok(rounded)
}

#[cfg(test)]
#[path = "finite_pc64_arithmetic_info_tests.rs"]
mod arithmetic_info_tests;
