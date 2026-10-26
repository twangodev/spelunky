//! Software model of the collision kernels' x87 PC=11/RC=00 intermediates.
//! Finite binary64 operands are exact inputs; arithmetic rounds each result to
//! a 64-bit significand. Transcendental coefficient production is separate.
use core::cmp::Ordering;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Real {
    negative: bool,
    significand: u64,
    exponent: i32,
}
impl Real {
    fn normalized(negative: bool, value: u128, mut exponent: i32) -> Self {
        if value == 0 {
            return Self {
                negative,
                significand: 0,
                exponent: 0,
            };
        }
        let bits = 128 - value.leading_zeros();
        let significand = if bits <= 64 {
            exponent -= (64 - bits) as i32;
            value << (64 - bits)
        } else {
            let shift = bits - 64;
            exponent += shift as i32;
            let quotient = value >> shift;
            let remainder = value & ((1u128 << shift) - 1);
            let half = 1u128 << (shift - 1);
            quotient + u128::from(remainder > half || (remainder == half && quotient & 1 != 0))
        };
        if significand == 1u128 << 64 {
            Self {
                negative,
                significand: 1u64 << 63,
                exponent: exponent + 1,
            }
        } else {
            Self {
                negative,
                significand: significand as u64,
                exponent,
            }
        }
    }
    pub(super) fn from_f64(value: f64) -> Option<Self> {
        if !value.is_finite() {
            return None;
        }
        let bits = value.to_bits();
        let negative = bits >> 63 != 0;
        let field = ((bits >> 52) & 0x7ff) as i32;
        let fraction = bits & ((1u64 << 52) - 1);
        Some(if field == 0 {
            Self::normalized(negative, u128::from(fraction), -1074)
        } else {
            Self::normalized(
                negative,
                u128::from(fraction | (1u64 << 52)),
                field - 1023 - 52,
            )
        })
    }
    pub(super) fn from_i32(value: i32) -> Self {
        Self::normalized(value < 0, u128::from(value.unsigned_abs()), 0)
    }
    pub(super) fn neg(self) -> Self {
        Self {
            negative: !self.negative,
            ..self
        }
    }
    pub(super) fn cmp(self, other: Self) -> Ordering {
        if self.significand == 0 && other.significand == 0 {
            return Ordering::Equal;
        }
        if self.negative != other.negative {
            return if self.negative {
                Ordering::Less
            } else {
                Ordering::Greater
            };
        }
        let magnitude = match (self.significand == 0, other.significand == 0) {
            (true, false) => Ordering::Less,
            (false, true) => Ordering::Greater,
            (true, true) => Ordering::Equal,
            (false, false) => {
                let self_top_bit = self.exponent + 63;
                let other_top_bit = other.exponent + 63;
                self_top_bit.cmp(&other_top_bit).then_with(|| {
                    debug_assert_eq!(self.exponent, other.exponent);
                    self.significand.cmp(&other.significand)
                })
            }
        };
        if self.negative {
            magnitude.reverse()
        } else {
            magnitude
        }
    }
    pub(super) fn add(self, other: Self) -> Self {
        if self.significand == 0 && other.significand == 0 {
            return Self {
                negative: self.negative && other.negative,
                significand: 0,
                exponent: 0,
            };
        }
        if self.significand == 0 {
            return other;
        }
        if other.significand == 0 {
            return self;
        }
        let (high, low) = if self.exponent >= other.exponent {
            (self, other)
        } else {
            (other, self)
        };
        let shift = (high.exponent - low.exponent) as u32;
        if shift > 64 {
            // Just below a binade boundary the lower neighbor has half the
            // usual spacing. Retain its one additional half-ULP case exactly.
            if shift == 65 && high.negative != low.negative && high.significand == 1u64 << 63 {
                return Self::normalized(
                    high.negative,
                    u128::MAX - u128::from(low.significand) + 1,
                    low.exponent,
                );
            }
            return high;
        }
        let a = u128::from(high.significand) << shift;
        let b = u128::from(low.significand);
        if high.negative == low.negative {
            Self::normalized(high.negative, a + b, low.exponent)
        } else if a >= b {
            Self::normalized(
                if a == b { false } else { high.negative },
                a - b,
                low.exponent,
            )
        } else {
            Self::normalized(low.negative, b - a, low.exponent)
        }
    }
    pub(super) fn sub(self, other: Self) -> Self {
        self.add(other.neg())
    }
    pub(super) fn mul(self, other: Self) -> Self {
        Self::normalized(
            self.negative != other.negative,
            u128::from(self.significand) * u128::from(other.significand),
            self.exponent + other.exponent,
        )
    }
    pub(super) fn div(self, other: Self) -> Option<Self> {
        if other.significand == 0 {
            return None;
        }
        if self.significand == 0 {
            return Some(Self {
                negative: self.negative != other.negative,
                ..self
            });
        }
        let shift = if self.significand >= other.significand {
            63
        } else {
            64
        };
        let numerator = u128::from(self.significand) << shift;
        let denominator = u128::from(other.significand);
        let quotient = numerator / denominator;
        let remainder = numerator % denominator;
        let rounded = quotient
            + u128::from(
                remainder * 2 > denominator || (remainder * 2 == denominator && quotient & 1 != 0),
            );
        Some(Self::normalized(
            self.negative != other.negative,
            rounded,
            self.exponent - other.exponent - shift as i32,
        ))
    }
    fn signed_integer(self, magnitude: u128) -> Option<i64> {
        if self.negative {
            if magnitude == 1u128 << 63 {
                Some(i64::MIN)
            } else {
                i64::try_from(magnitude).ok().map(|n| -n)
            }
        } else {
            i64::try_from(magnitude).ok()
        }
    }
    fn integer_parts(self) -> Option<(u128, u128, u32)> {
        if self.significand == 0 {
            return Some((0, 0, 0));
        }
        if self.exponent >= 0 {
            let shift = self.exponent as u32;
            if shift > 63 {
                return None;
            }
            Some((u128::from(self.significand) << shift, 0, 0))
        } else {
            let shift = (-self.exponent) as u32;
            if shift >= 64 {
                Some((0, u128::from(self.significand), shift))
            } else {
                Some((
                    u128::from(self.significand >> shift),
                    u128::from(self.significand & ((1u64 << shift) - 1)),
                    shift,
                ))
            }
        }
    }
    pub(super) fn floor_i32(self) -> Option<i32> {
        let (integer, remainder, _) = self.integer_parts()?;
        self.signed_integer(integer + u128::from(self.negative && remainder != 0))
            .map(|n| n as i32)
    }
    pub(super) fn round_i32(self) -> Option<i32> {
        let (integer, remainder, shift) = self.integer_parts()?;
        let increment = if shift == 0 || shift > 64 {
            false
        } else {
            let half = 1u128 << (shift - 1);
            remainder > half || (remainder == half && integer & 1 != 0)
        };
        self.signed_integer(integer + u128::from(increment))
            .map(|n| n as i32)
    }
}

#[cfg(test)]
mod tests {
    use super::Real;
    use core::cmp::Ordering;

    fn binary80(value: Real) -> [u8; 10] {
        let mut bytes = [0u8; 10];
        bytes[..8].copy_from_slice(&value.significand.to_le_bytes());
        let exponent = if value.significand == 0 {
            0
        } else {
            (value.exponent + 63 + 16383) as u16
        };
        bytes[8..]
            .copy_from_slice(&(exponent | if value.negative { 0x8000 } else { 0 }).to_le_bytes());
        bytes
    }
    #[test]
    fn comparison_orders_signed_zero_signs_fractions_and_binade_edges() {
        let real = |value| Real::from_f64(value).unwrap();
        let positive_zero = real(0.0);
        let negative_zero = real(-0.0);

        assert_eq!(negative_zero.cmp(positive_zero), Ordering::Equal);
        assert_eq!(negative_zero.cmp(real(1.0)), Ordering::Less);
        assert_eq!(positive_zero.cmp(real(-1.0)), Ordering::Greater);
        assert_eq!(real(-2.0).cmp(real(-1.0)), Ordering::Less);
        assert_eq!(real(-1.0).cmp(positive_zero), Ordering::Less);
        assert_eq!(real(1.0).cmp(positive_zero), Ordering::Greater);

        let before_two = f64::from_bits(2.0f64.to_bits() - 1);
        let after_two = f64::from_bits(2.0f64.to_bits() + 1);
        assert_eq!(real(before_two).cmp(real(2.0)), Ordering::Less);
        assert_eq!(real(2.0).cmp(real(after_two)), Ordering::Less);
        assert_eq!(real(-after_two).cmp(real(-2.0)), Ordering::Less);
        assert_eq!(real(-2.0).cmp(real(-before_two)), Ordering::Less);

        let three_halves = Real::from_i32(3).div(Real::from_i32(2)).unwrap();
        let negative_five_quarters = Real::from_i32(-5).div(Real::from_i32(4)).unwrap();
        assert_eq!(real(1.5).cmp(three_halves), Ordering::Equal);
        assert_eq!(real(-1.25).cmp(negative_five_quarters), Ordering::Equal);
        assert_eq!(real(1.25).cmp(real(1.5)), Ordering::Less);
        assert_eq!(real(-1.5).cmp(real(-1.25)), Ordering::Less);
    }

    #[test]
    fn pc64_matrix_matches_all_original_byte_probe_prefloor_values() {
        // Native measured coefficients are literal operands, not a proposed
        // trig implementation. Expected binary80 fields come from the frozen
        // original-byte instruction adapter, independently decoded as rationals.
        for (dx, dy, sx, sy, sine_bits, cosine_bits, expected_x, expected_y) in [
            (
                -8,
                -1,
                2.0,
                1.0,
                0x8000000000000000,
                0x3ff0000000000000,
                [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
                [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xc0, 0x00, 0x40],
            ),
            (
                -4,
                -1,
                2.0,
                1.0,
                0x8000000000000000,
                0x3ff0000000000000,
                [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x00, 0x40],
                [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xc0, 0x00, 0x40],
            ),
            (
                -2,
                -1,
                0.5,
                1.0,
                0x8000000000000000,
                0x3ff0000000000000,
                [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
                [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xc0, 0x00, 0x40],
            ),
            (
                -1,
                -1,
                0.5,
                1.0,
                0x8000000000000000,
                0x3ff0000000000000,
                [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x00, 0x40],
                [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xc0, 0x00, 0x40],
            ),
            (
                8,
                -1,
                -2.0,
                1.0,
                0x8000000000000000,
                0x3ff0000000000000,
                [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
                [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xc0, 0x00, 0x40],
            ),
            (
                4,
                -1,
                -2.0,
                1.0,
                0x8000000000000000,
                0x3ff0000000000000,
                [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x00, 0x40],
                [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xc0, 0x00, 0x40],
            ),
            (
                -4,
                -2,
                1.0,
                2.0,
                0x8000000000000000,
                0x3ff0000000000000,
                [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
                [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xc0, 0x00, 0x40],
            ),
            (
                -2,
                -2,
                1.0,
                2.0,
                0x8000000000000000,
                0x3ff0000000000000,
                [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x00, 0x40],
                [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xc0, 0x00, 0x40],
            ),
            (
                -4,
                0,
                1.0,
                -0.5,
                0x8000000000000000,
                0x3ff0000000000000,
                [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
                [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x01, 0x40],
            ),
            (
                -2,
                0,
                1.0,
                -0.5,
                0x8000000000000000,
                0x3ff0000000000000,
                [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x00, 0x40],
                [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x01, 0x40],
            ),
            (
                -8,
                0,
                2.0,
                0.5,
                0x8000000000000000,
                0x3ff0000000000000,
                [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
                [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x01, 0x40],
            ),
            (
                -4,
                0,
                2.0,
                0.5,
                0x8000000000000000,
                0x3ff0000000000000,
                [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x00, 0x40],
                [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x01, 0x40],
            ),
            (
                0,
                -2,
                1.0,
                1.0,
                0xbfe491b7523c161d,
                0x3fe8836fa2cf5039,
                [0x00, 0x3a, 0x2c, 0x78, 0xa4, 0x6e, 0x23, 0xa9, 0x01, 0x40],
                [0x00, 0x1c, 0xbf, 0xc2, 0x74, 0x41, 0xf2, 0x9d, 0x00, 0x40],
            ),
            (
                0,
                0,
                1.0,
                1.0,
                0xbfe491b7523c161d,
                0x3fe8836fa2cf5039,
                [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x01, 0x40],
                [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x01, 0x40],
            ),
            (
                0,
                -2,
                1.0,
                1.0,
                0xbfe6a09e667f3bcd,
                0x3fe6a09e667f3bcd,
                [0x00, 0x9a, 0x77, 0xfe, 0xcc, 0x3c, 0x41, 0xad, 0x01, 0x40],
                [0x00, 0xcc, 0x10, 0x03, 0x66, 0x86, 0x7d, 0xa5, 0x00, 0x40],
            ),
            (
                0,
                0,
                1.0,
                1.0,
                0xbfe6a09e667f3bcd,
                0x3fe6a09e667f3bcd,
                [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x01, 0x40],
                [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x01, 0x40],
            ),
            (
                0,
                -2,
                1.0,
                1.0,
                0xbfe8836fa2cf5039,
                0x3fe491b7523c161d,
                [0x00, 0x72, 0xa0, 0x9e, 0x45, 0xdf, 0x06, 0xb1, 0x01, 0x40],
                [0x00, 0x8c, 0xa7, 0x0f, 0xb7, 0x22, 0xb9, 0xad, 0x00, 0x40],
            ),
            (
                0,
                0,
                1.0,
                1.0,
                0xbfe8836fa2cf5039,
                0x3fe491b7523c161d,
                [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x01, 0x40],
                [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x01, 0x40],
            ),
            (
                0,
                -3,
                1.0,
                1.0,
                0xbff0000000000000,
                0xbbe0000000000000,
                [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xe0, 0x01, 0x40],
                [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x01, 0x40],
            ),
            (
                -3,
                0,
                1.0,
                1.0,
                0x3bf0000000000000,
                0xbff0000000000000,
                [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xe0, 0x01, 0x40],
                [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x01, 0x40],
            ),
            (
                0,
                3,
                1.0,
                1.0,
                0x3ff0000000000000,
                0x3c0c000000000000,
                [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xe0, 0x01, 0x40],
                [0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0x01, 0x40],
            ),
            (
                0,
                -2,
                -1.0,
                1.0,
                0xbfe6a09e667f3bcd,
                0x3fe6a09e667f3bcd,
                [0x00, 0xcc, 0x10, 0x03, 0x66, 0x86, 0x7d, 0xa5, 0x00, 0x40],
                [0x00, 0xcc, 0x10, 0x03, 0x66, 0x86, 0x7d, 0xa5, 0x00, 0x40],
            ),
            (
                0,
                -2,
                1.0,
                -1.0,
                0xbfe6a09e667f3bcd,
                0x3fe6a09e667f3bcd,
                [0x00, 0x9a, 0x77, 0xfe, 0xcc, 0x3c, 0x41, 0xad, 0x01, 0x40],
                [0x00, 0x9a, 0x77, 0xfe, 0xcc, 0x3c, 0x41, 0xad, 0x01, 0x40],
            ),
            (
                -3,
                0,
                -1.0,
                -1.0,
                0x3bf0000000000000,
                0xbff0000000000000,
                [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80, 0xff, 0x3f],
                [0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x00, 0x40],
            ),
            (
                0,
                -2,
                2.0,
                0.5,
                0xbfe6a09e667f3bcd,
                0x3fe6a09e667f3bcd,
                [0x00, 0xcd, 0x3b, 0x7f, 0x66, 0x9e, 0xa0, 0x96, 0x01, 0x40],
                [0x00, 0x30, 0x43, 0x0c, 0x98, 0x19, 0xf6, 0x95, 0xff, 0x3f],
            ),
        ] {
            let sine = Real::from_f64(f64::from_bits(sine_bits)).unwrap();
            let cosine = Real::from_f64(f64::from_bits(cosine_bits)).unwrap();
            let x = Real::from_i32(dx)
                .mul(cosine)
                .add(Real::from_i32(dy).mul(sine))
                .div(Real::from_f64(sx).unwrap())
                .unwrap()
                .add(Real::from_i32(4));
            let y = Real::from_i32(dy)
                .mul(cosine)
                .sub(Real::from_i32(dx).mul(sine))
                .div(Real::from_f64(sy).unwrap())
                .unwrap()
                .add(Real::from_i32(4));
            assert_eq!(binary80(x), expected_x, "NATIVE_PC64_X {dx}/{dy}/{sx}/{sy}");
            assert_eq!(binary80(y), expected_y, "NATIVE_PC64_Y {dx}/{dy}/{sx}/{sy}");
        }
    }
    #[test]
    fn pc64_binade_halfway_subtraction_preserves_native_floor() {
        // Below 1, PC64 spacing is2^-64. Exactly half that gap ties to1;
        // the next binary64 value above2^-65 instead rounds to1−2^-64.
        // f64 arithmetic rounds both to1, so this is a source-derived
        // precision boundary, not an implementation-mirroring test.
        let plus_zero = Real::from_f64(0.0).unwrap();
        let minus_zero = Real::from_f64(-0.0).unwrap();
        assert!(!plus_zero.add(minus_zero).negative);
        assert!(!plus_zero.sub(plus_zero).negative);
        assert!(minus_zero.add(minus_zero).negative);
        let half_gap = f64::from_bits(0x3be0000000000000);
        let one = Real::from_i32(1);
        assert_eq!(
            one.sub(Real::from_f64(half_gap).unwrap()).floor_i32(),
            Some(1)
        );
        let above_half = f64::from_bits(half_gap.to_bits() + 1);
        assert_eq!(
            one.sub(Real::from_f64(above_half).unwrap()).floor_i32(),
            Some(0)
        );
        for (value, expected) in [(0.5, 0), (1.5, 2), (-0.5, 0), (-1.5, -2)] {
            assert_eq!(Real::from_f64(value).unwrap().round_i32(), Some(expected));
        }
    }
}
