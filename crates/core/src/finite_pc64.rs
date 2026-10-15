//! Finite value arithmetic for a proposed, versioned Bochs-compatible profile.
//! This is a derivative semantic port of f128_67.cc and the finite rounding
//! paths of Bochs/SoftFloat, commit 1ae392db40302bcfc89a1dc331a05e8cef43eaa4.
//! Integer magnitude arithmetic below is independently implemented Rust.
//! Upstream attribution and unmodified notices accompany this file in
//! UPSTREAM-NOTICES.txt, COPYING-Bochs, and LICENSE-SoftFloat.txt.
//! No host floating arithmetic, C/C++ runtime, hardware FPU, or table of
//! measured native answers is used. Exception flags/masks and nonfinite values
//! are not emulated; unsupported value domains return an error.

/*============================================================================

This C source file is part of the SoftFloat IEEE Floating-Point Arithmetic
Package, Release 3e, by John R. Hauser.

Copyright 2011, 2012, 2013, 2014, 2015, 2017 The Regents of the University of
California.  All rights reserved.

Redistribution and use in source and binary forms, with or without
modification, are permitted provided that the following conditions are met:

 1. Redistributions of source code must retain the above copyright notice,
    this list of conditions, and the following disclaimer.

 2. Redistributions in binary form must reproduce the above copyright notice,
    this list of conditions, and the following disclaimer in the documentation
    and/or other materials provided with the distribution.

 3. Neither the name of the University nor the names of its contributors may
    be used to endorse or promote products derived from this software without
    specific prior written permission.

THIS SOFTWARE IS PROVIDED BY THE REGENTS AND CONTRIBUTORS "AS IS", AND ANY
EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE, ARE
DISCLAIMED.  IN NO EVENT SHALL THE REGENTS OR CONTRIBUTORS BE LIABLE FOR ANY
DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES
(INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES;
LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND
ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
(INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF THIS
SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

=============================================================================*/

use core::cmp::Ordering;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct F128 {
    pub hi: u64,
    pub lo: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ext80 {
    pub sign_exp: u16,
    pub significand: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArithmeticError {
    NonFinite,
    UnsupportedEncoding,
    DivisionByZero,
    Overflow,
    ExponentDomain,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rounding {
    NearestEven,
    ToZero,
}

/// Numerical store information, not a full x87 status-word implementation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rounded<T> {
    pub value: T,
    pub inexact: bool,
    pub magnitude_rounded_up: bool,
}

#[derive(Clone, Copy)]
struct Number {
    negative: bool,
    significand: u128,
    exponent: i32,
}

// A finite binary128 addition spans at most 32,879 significant bit positions.
// A fixed maximum also bounds the public scaled-integer constructor.
const MAX_SHIFT: usize = 66_000;
const MIN_QUAD_QUANTUM: i32 = -16_494;
const MIN_EXT_QUANTUM: i32 = -16_445;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Nat(Vec<u64>);

impl Nat {
    fn from_u128(value: u128) -> Self {
        let mut n = Self(vec![value as u64, (value >> 64) as u64]);
        n.trim();
        n
    }
    fn trim(&mut self) {
        while self.0.last() == Some(&0) {
            self.0.pop();
        }
    }
    fn is_zero(&self) -> bool {
        self.0.is_empty()
    }
    fn bits(&self) -> usize {
        self.0.last().map_or(0, |last| {
            (self.0.len() - 1) * 64 + 64 - last.leading_zeros() as usize
        })
    }
    fn bit(&self, index: usize) -> bool {
        self.0
            .get(index / 64)
            .is_some_and(|word| word & (1u64 << (index % 64)) != 0)
    }
    fn any_below(&self, count: usize) -> bool {
        let whole = (count / 64).min(self.0.len());
        if self.0[..whole].iter().any(|&word| word != 0) {
            return true;
        }
        count % 64 != 0
            && self
                .0
                .get(count / 64)
                .is_some_and(|word| word & ((1u64 << (count % 64)) - 1) != 0)
    }
    fn cmp(&self, other: &Self) -> Ordering {
        self.0
            .len()
            .cmp(&other.0.len())
            .then_with(|| self.0.iter().rev().cmp(other.0.iter().rev()))
    }
    fn shl(&self, count: usize) -> Result<Self, ArithmeticError> {
        if count > MAX_SHIFT {
            return Err(ArithmeticError::ExponentDomain);
        }
        if self.is_zero() {
            return Ok(Self(vec![]));
        }
        let words = count / 64;
        let bits = count % 64;
        let mut out = vec![0u64; words + self.0.len() + usize::from(bits != 0)];
        for (i, &word) in self.0.iter().enumerate() {
            out[words + i] |= word << bits;
            if bits != 0 {
                out[words + i + 1] |= word >> (64 - bits);
            }
        }
        let mut result = Self(out);
        result.trim();
        Ok(result)
    }
    fn shr(&self, count: usize) -> Self {
        let words = count / 64;
        if words >= self.0.len() {
            return Self(vec![]);
        }
        let bits = count % 64;
        let mut out = Vec::with_capacity(self.0.len() - words);
        for i in words..self.0.len() {
            let mut word = self.0[i] >> bits;
            if bits != 0 && i + 1 < self.0.len() {
                word |= self.0[i + 1] << (64 - bits);
            }
            out.push(word);
        }
        let mut result = Self(out);
        result.trim();
        result
    }
    fn add(&self, other: &Self) -> Self {
        let mut out = Vec::with_capacity(self.0.len().max(other.0.len()) + 1);
        let mut carry = 0u128;
        for i in 0..self.0.len().max(other.0.len()) {
            let sum = u128::from(*self.0.get(i).unwrap_or(&0))
                + u128::from(*other.0.get(i).unwrap_or(&0))
                + carry;
            out.push(sum as u64);
            carry = sum >> 64;
        }
        if carry != 0 {
            out.push(carry as u64);
        }
        Self(out)
    }
    fn sub(&self, other: &Self) -> Self {
        debug_assert!(self.cmp(other) != Ordering::Less);
        let mut out = Vec::with_capacity(self.0.len());
        let mut borrow = false;
        for (i, &word) in self.0.iter().enumerate() {
            let (first, b1) = word.overflowing_sub(*other.0.get(i).unwrap_or(&0));
            let (second, b2) = first.overflowing_sub(u64::from(borrow));
            out.push(second);
            borrow = b1 || b2;
        }
        debug_assert!(!borrow);
        let mut result = Self(out);
        result.trim();
        result
    }
    fn mul(&self, other: &Self) -> Self {
        if self.is_zero() || other.is_zero() {
            return Self(vec![]);
        }
        let mut out = vec![0u64; self.0.len() + other.0.len()];
        for (i, &a) in self.0.iter().enumerate() {
            let mut carry = 0u128;
            for (j, &b) in other.0.iter().enumerate() {
                let product = u128::from(a) * u128::from(b) + u128::from(out[i + j]) + carry;
                out[i + j] = product as u64;
                carry = product >> 64;
            }
            out[i + other.0.len()] = carry as u64;
        }
        let mut result = Self(out);
        result.trim();
        result
    }
    fn div_rem(&self, divisor: &Self) -> Result<(Self, Self), ArithmeticError> {
        if divisor.is_zero() {
            return Err(ArithmeticError::DivisionByZero);
        }
        if self.cmp(divisor) == Ordering::Less {
            return Ok((Self(vec![]), self.clone()));
        }
        let mut quotient = Self(vec![0; self.0.len()]);
        let mut remainder = Self(vec![]);
        for bit in (0..self.bits()).rev() {
            remainder = remainder.shl(1)?;
            if self.bit(bit) {
                remainder = remainder.add(&Self::from_u128(1));
            }
            if remainder.cmp(divisor) != Ordering::Less {
                remainder = remainder.sub(divisor);
                quotient.0[bit / 64] |= 1u64 << (bit % 64);
            }
        }
        quotient.trim();
        Ok((quotient, remainder))
    }
    fn to_u128(&self) -> Result<u128, ArithmeticError> {
        if self.0.len() > 2 {
            return Err(ArithmeticError::ExponentDomain);
        }
        Ok(u128::from(*self.0.first().unwrap_or(&0))
            | (u128::from(*self.0.get(1).unwrap_or(&0)) << 64))
    }
}

impl F128 {
    pub const fn from_words(hi: u64, lo: u64) -> Self {
        Self { hi, lo }
    }
    pub const fn zero(negative: bool) -> Self {
        Self {
            hi: (negative as u64) << 63,
            lo: 0,
        }
    }
    pub fn neg(self) -> Self {
        Self {
            hi: self.hi ^ (1u64 << 63),
            lo: self.lo,
        }
    }
    pub fn is_zero(self) -> bool {
        self.hi & 0x7fff_ffff_ffff_ffff == 0 && self.lo == 0
    }
    fn number(self) -> Result<Number, ArithmeticError> {
        let field = ((self.hi >> 48) & 0x7fff) as i32;
        if field == 0x7fff {
            return Err(ArithmeticError::NonFinite);
        }
        let fraction = (u128::from(self.hi & 0x0000_ffff_ffff_ffff) << 64) | u128::from(self.lo);
        Ok(Number {
            negative: self.hi >> 63 != 0,
            significand: fraction | if field != 0 { 1u128 << 112 } else { 0 },
            exponent: if field == 0 {
                MIN_QUAD_QUANTUM
            } else {
                field - 16383 - 112
            },
        })
    }
}

struct Quantized {
    magnitude: Nat,
    exponent: i32,
    inexact: bool,
    incremented: bool,
}

fn quantize(
    magnitude: Nat,
    exponent: i32,
    precision: u32,
    min_quantum: i32,
    mode: Rounding,
) -> Result<Quantized, ArithmeticError> {
    if magnitude.is_zero() {
        return Ok(Quantized {
            magnitude,
            exponent: min_quantum,
            inexact: false,
            incremented: false,
        });
    }
    let quantum = (exponent + magnitude.bits() as i32 - precision as i32).max(min_quantum);
    let shift = quantum - exponent;
    if shift <= 0 {
        return Ok(Quantized {
            magnitude: magnitude.shl((-shift) as usize)?,
            exponent: quantum,
            inexact: false,
            incremented: false,
        });
    }
    let shift = shift as usize;
    let mut rounded = magnitude.shr(shift);
    let inexact = magnitude.any_below(shift);
    let incremented = mode == Rounding::NearestEven
        && magnitude.bit(shift - 1)
        && (magnitude.any_below(shift - 1) || rounded.bit(0));
    if incremented {
        rounded = rounded.add(&Nat::from_u128(1));
    }
    Ok(Quantized {
        magnitude: rounded,
        exponent: quantum,
        inexact,
        incremented,
    })
}

fn pack_quad(negative: bool, q: Quantized) -> Result<Rounded<F128>, ArithmeticError> {
    let bits = q.magnitude.bits();
    let value = if bits == 0 {
        F128::zero(negative)
    } else {
        let top = q.exponent + bits as i32 - 1;
        if top > 16383 {
            return Err(ArithmeticError::Overflow);
        }
        if top >= -16382 {
            // A rounded carry can produce 114 bits, with a known zero tail.
            let sig = if bits > 113 {
                q.magnitude.shr(bits - 113).to_u128()?
            } else {
                q.magnitude.shl(113 - bits)?.to_u128()?
            };
            F128 {
                hi: (u64::from(negative) << 63)
                    | (((top + 16383) as u64) << 48)
                    | (((sig >> 64) as u64) & 0x0000_ffff_ffff_ffff),
                lo: sig as u64,
            }
        } else {
            let shift = q.exponent - MIN_QUAD_QUANTUM;
            if shift < 0 {
                return Err(ArithmeticError::ExponentDomain);
            }
            let sig = q.magnitude.shl(shift as usize)?.to_u128()?;
            F128 {
                hi: (u64::from(negative) << 63) | ((sig >> 64) as u64),
                lo: sig as u64,
            }
        }
    };
    Ok(Rounded {
        value,
        inexact: q.inexact,
        magnitude_rounded_up: q.incremented,
    })
}

fn rounded_quad(
    negative: bool,
    magnitude: Nat,
    exponent: i32,
    precision: u32,
    mode: Rounding,
) -> Result<F128, ArithmeticError> {
    // Internal67 clamping is BEFORE the physical binary128 subnormal shift.
    // Its minimum quantum is therefore binary128's, not a 67-bit float's.
    Ok(pack_quad(
        negative,
        quantize(magnitude, exponent, precision, MIN_QUAD_QUANTUM, mode)?,
    )?
    .value)
}

pub fn from_scaled_u128(
    negative: bool,
    significand: u128,
    exponent: i32,
) -> Result<F128, ArithmeticError> {
    if !(-33_000..=33_000).contains(&exponent) {
        return Err(ArithmeticError::ExponentDomain);
    }
    rounded_quad(
        negative,
        Nat::from_u128(significand),
        exponent,
        113,
        Rounding::NearestEven,
    )
}

pub fn from_ext80(value: Ext80) -> Result<F128, ArithmeticError> {
    let field = i32::from(value.sign_exp & 0x7fff);
    if field == 0x7fff {
        return Err(ArithmeticError::NonFinite);
    }
    if field != 0 && value.significand >> 63 == 0 {
        return Err(ArithmeticError::UnsupportedEncoding);
    }
    // A pseudo-denormal (field0/integerbit1) has the field1 numerical value.
    from_scaled_u128(
        value.sign_exp >> 15 != 0,
        u128::from(value.significand),
        field.max(1) - 16383 - 63,
    )
}

pub fn from_f64(value: f64) -> Result<F128, ArithmeticError> {
    let bits = value.to_bits();
    let field = ((bits >> 52) & 0x7ff) as i32;
    if field == 0x7ff {
        return Err(ArithmeticError::NonFinite);
    }
    let fraction = bits & 0x000f_ffff_ffff_ffff;
    from_scaled_u128(
        bits >> 63 != 0,
        u128::from(fraction | if field != 0 { 1u64 << 52 } else { 0 }),
        field.max(1) - 1023 - 52,
    )
}

fn add_at(a: F128, b: F128, precision: u32, mode: Rounding) -> Result<F128, ArithmeticError> {
    // s_addMagsF128 returns the exact raw fraction sum before roundPack
    // when BOTH exponent fields are zero and the signs agree. In particular
    // this upstream shortcut bypasses the optional internal67 clamp.
    let both_subnormal_fields = (a.hi >> 48) & 0x7fff == 0 && (b.hi >> 48) & 0x7fff == 0;
    if both_subnormal_fields && (a.hi ^ b.hi) >> 63 == 0 {
        let x = (u128::from(a.hi & 0x0000_ffff_ffff_ffff) << 64) | u128::from(a.lo);
        let y = (u128::from(b.hi & 0x0000_ffff_ffff_ffff) << 64) | u128::from(b.lo);
        let sum = x + y;
        return Ok(F128 {
            hi: (a.hi & (1u64 << 63)) | (sum >> 64) as u64,
            lo: sum as u64,
        });
    }
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
            Ordering::Equal => (false, Nat(vec![])),
        }
    };
    rounded_quad(negative, magnitude, exponent, precision, mode)
}

fn mul_at(a: F128, b: F128, precision: u32, mode: Rounding) -> Result<F128, ArithmeticError> {
    let a = a.number()?;
    let b = b.number()?;
    rounded_quad(
        a.negative ^ b.negative,
        Nat::from_u128(a.significand).mul(&Nat::from_u128(b.significand)),
        a.exponent + b.exponent,
        precision,
        mode,
    )
}

fn div_at(a: F128, b: F128, precision: u32, mode: Rounding) -> Result<F128, ArithmeticError> {
    let a = a.number()?;
    let b = b.number()?;
    if b.significand == 0 {
        return Err(ArithmeticError::DivisionByZero);
    }
    if a.significand == 0 {
        return Ok(F128::zero(a.negative ^ b.negative));
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
    let quantum = (exponent + log_ratio - precision as i32 + 1).max(MIN_QUAD_QUANTUM);
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
    let comparison = remainder.shl(1)?.cmp(&denominator);
    if mode == Rounding::NearestEven
        && (comparison == Ordering::Greater || comparison == Ordering::Equal && quotient.bit(0))
    {
        quotient = quotient.add(&Nat::from_u128(1));
    }
    Ok(pack_quad(
        a.negative ^ b.negative,
        Quantized {
            magnitude: quotient,
            exponent: quantum,
            inexact: !remainder.is_zero(),
            incremented: false,
        },
    )?
    .value)
}

pub fn add113_nearest(a: F128, b: F128) -> Result<F128, ArithmeticError> {
    add_at(a, b, 113, Rounding::NearestEven)
}
pub fn sub113_nearest(a: F128, b: F128) -> Result<F128, ArithmeticError> {
    add113_nearest(a, b.neg())
}
pub fn mul113_nearest(a: F128, b: F128) -> Result<F128, ArithmeticError> {
    mul_at(a, b, 113, Rounding::NearestEven)
}
pub fn div113_nearest(a: F128, b: F128) -> Result<F128, ArithmeticError> {
    div_at(a, b, 113, Rounding::NearestEven)
}
pub fn add67_chop(a: F128, b: F128) -> Result<F128, ArithmeticError> {
    add_at(a, b, 67, Rounding::ToZero)
}
pub fn mul67_chop(a: F128, b: F128) -> Result<F128, ArithmeticError> {
    mul_at(a, b, 67, Rounding::ToZero)
}
pub fn div67_chop(a: F128, b: F128) -> Result<F128, ArithmeticError> {
    div_at(a, b, 67, Rounding::ToZero)
}

/// Bochs f128_round(...,80,...): raw significand narrowing, not a store.
/// Subnormals drop the same 49 low fraction bits as normals.
pub fn narrow64(a: F128, mode: Rounding) -> Result<F128, ArithmeticError> {
    let n = a.number()?;
    Ok(pack_quad(
        n.negative,
        quantize(
            Nat::from_u128(n.significand),
            n.exponent,
            64,
            MIN_EXT_QUANTUM,
            mode,
        )?,
    )?
    .value)
}
pub fn mul_e67(a: F128, b: F128) -> Result<F128, ArithmeticError> {
    mul67_chop(a, narrow64(b, Rounding::ToZero)?)
}
pub fn add64_ne(a: F128, b: F128) -> Result<F128, ArithmeticError> {
    narrow64(add113_nearest(a, b)?, Rounding::NearestEven)
}
pub fn mul64_ne(a: F128, b: F128) -> Result<F128, ArithmeticError> {
    narrow64(
        mul113_nearest(a, narrow64(b, Rounding::ToZero)?)?,
        Rounding::NearestEven,
    )
}
pub fn div64_ne(a: F128, b: F128) -> Result<F128, ArithmeticError> {
    narrow64(div113_nearest(a, b)?, Rounding::NearestEven)
}

pub fn to_ext80_nearest_info(a: F128) -> Result<Rounded<Ext80>, ArithmeticError> {
    let n = a.number()?;
    let q = quantize(
        Nat::from_u128(n.significand),
        n.exponent,
        64,
        MIN_EXT_QUANTUM,
        Rounding::NearestEven,
    )?;
    let bits = q.magnitude.bits();
    let top = q.exponent + bits as i32 - 1;
    let (field, significand) = if bits == 0 {
        (0, 0)
    } else if top > 16383 {
        return Err(ArithmeticError::Overflow);
    } else if top >= -16382 {
        let sig = if bits > 64 {
            q.magnitude.shr(bits - 64)
        } else {
            q.magnitude.shl(64 - bits)?
        };
        ((top + 16383) as u16, sig.to_u128()? as u64)
    } else {
        (
            0,
            q.magnitude
                .shl((q.exponent - MIN_EXT_QUANTUM) as usize)?
                .to_u128()? as u64,
        )
    };
    Ok(Rounded {
        value: Ext80 {
            sign_exp: (u16::from(n.negative) << 15) | field,
            significand,
        },
        inexact: q.inexact,
        magnitude_rounded_up: q.incremented,
    })
}
pub fn to_ext80_nearest(a: F128) -> Result<Ext80, ArithmeticError> {
    Ok(to_ext80_nearest_info(a)?.value)
}

pub fn to_f64_nearest_info(a: F128) -> Result<Rounded<f64>, ArithmeticError> {
    let n = a.number()?;
    let q = quantize(
        Nat::from_u128(n.significand),
        n.exponent,
        53,
        -1074,
        Rounding::NearestEven,
    )?;
    let bits = q.magnitude.bits();
    let top = q.exponent + bits as i32 - 1;
    let raw = if bits == 0 {
        u64::from(n.negative) << 63
    } else if top > 1023 {
        return Err(ArithmeticError::Overflow);
    } else if top >= -1022 {
        let sig = if bits > 53 {
            q.magnitude.shr(bits - 53)
        } else {
            q.magnitude.shl(53 - bits)?
        };
        (u64::from(n.negative) << 63)
            | (((top + 1023) as u64) << 52)
            | (sig.to_u128()? as u64 & 0x000f_ffff_ffff_ffff)
    } else {
        (u64::from(n.negative) << 63)
            | q.magnitude.shl((q.exponent + 1074) as usize)?.to_u128()? as u64
    };
    Ok(Rounded {
        value: f64::from_bits(raw),
        inexact: q.inexact,
        magnitude_rounded_up: q.incremented,
    })
}
pub fn to_f64_nearest(a: F128) -> Result<f64, ArithmeticError> {
    Ok(to_f64_nearest_info(a)?.value)
}

#[cfg(test)]
#[path = "finite_reference_tests.rs"]
mod tests;

// Additive nearest-only finite PC64/RN primitives. This is direct rounding
// to 64 significant bits, NOT the Bochs RN113-then-RN64 transcendental helpers.
// The nearest caller's materialized binary64 X/Y provenance bounds every
// nonzero intermediate inside normal Ext80 range: conservative top±2200.
fn nearest_domain(a: F128) -> Result<(), ArithmeticError> {
    let n = a.number()?;
    if n.significand != 0 {
        let top = n.exponent + (128 - n.significand.leading_zeros()) as i32 - 1;
        if !(-2200..=2200).contains(&top) {
            return Err(ArithmeticError::ExponentDomain);
        }
    }
    Ok(())
}
pub(crate) fn nearest_add_pc64_rn(a: F128, b: F128) -> Result<F128, ArithmeticError> {
    nearest_domain(a)?;
    nearest_domain(b)?;
    let value = add_at(a, b, 64, Rounding::NearestEven)?;
    nearest_domain(value)?;
    Ok(value)
}
pub(crate) fn nearest_sub_pc64_rn(a: F128, b: F128) -> Result<F128, ArithmeticError> {
    nearest_add_pc64_rn(a, b.neg())
}
pub(crate) fn nearest_mul_pc64_rn(a: F128, b: F128) -> Result<F128, ArithmeticError> {
    nearest_domain(a)?;
    nearest_domain(b)?;
    let value = mul_at(a, b, 64, Rounding::NearestEven)?;
    nearest_domain(value)?;
    Ok(value)
}
pub(crate) fn nearest_compare_finite(a: F128, b: F128) -> Result<Ordering, ArithmeticError> {
    a.number()?;
    b.number()?;
    if a.is_zero() && b.is_zero() {
        return Ok(Ordering::Equal);
    }
    let sa = a.hi >> 63 != 0;
    let sb = b.hi >> 63 != 0;
    if sa != sb {
        return Ok(if sa {
            Ordering::Less
        } else {
            Ordering::Greater
        });
    }
    let order = (a.hi & 0x7fff_ffff_ffff_ffff, a.lo).cmp(&(b.hi & 0x7fff_ffff_ffff_ffff, b.lo));
    Ok(if sa { order.reverse() } else { order })
}

// PC53/RN variant of the native nearest finite-QWORD operation graph.
// This rounds each exact integer operation directly to 53 significant bits,
// retaining the extended exponent. In particular 2^-1074 squared is 2^-2148,
// not a binary64 zero until the separate QWORD best-distance store.
pub(crate) fn nearest_add_pc53_rn(a: F128, b: F128) -> Result<F128, ArithmeticError> {
    nearest_domain(a)?;
    nearest_domain(b)?;
    let value = add_at(a, b, 53, Rounding::NearestEven)?;
    nearest_domain(value)?;
    Ok(value)
}
pub(crate) fn nearest_sub_pc53_rn(a: F128, b: F128) -> Result<F128, ArithmeticError> {
    nearest_add_pc53_rn(a, b.neg())
}
pub(crate) fn nearest_mul_pc53_rn(a: F128, b: F128) -> Result<F128, ArithmeticError> {
    nearest_domain(a)?;
    nearest_domain(b)?;
    let value = mul_at(a, b, 53, Rounding::NearestEven)?;
    nearest_domain(value)?;
    Ok(value)
}

// Additive direct PC53/PC64 operation-rounding metadata; historical APIs unchanged.
include!("finite_pc64_arithmetic_info.rs");
