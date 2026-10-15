// Exact Fraction oracle data. Tests not executed by author.
const CASES: &[(F128, Ext80Rounding, u32, u16)] = &[
    (
        F128 {
            hi: 0x0000000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0x00000000,
        0,
    ), // zero_positive
    (
        F128 {
            hi: 0x0000000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0x00000000,
        0,
    ), // zero_positive
    (
        F128 {
            hi: 0x0000000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0x00000000,
        0,
    ), // zero_positive
    (
        F128 {
            hi: 0x0000000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0x00000000,
        0,
    ), // zero_positive
    (
        F128 {
            hi: 0x8000000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0x80000000,
        0,
    ), // zero_negative
    (
        F128 {
            hi: 0x8000000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0x80000000,
        0,
    ), // zero_negative
    (
        F128 {
            hi: 0x8000000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0x80000000,
        0,
    ), // zero_negative
    (
        F128 {
            hi: 0x8000000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0x80000000,
        0,
    ), // zero_negative
    (
        F128 {
            hi: 0x3fff000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0x3f800000,
        0,
    ), // one_positive
    (
        F128 {
            hi: 0x3fff000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0x3f800000,
        0,
    ), // one_positive
    (
        F128 {
            hi: 0x3fff000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0x3f800000,
        0,
    ), // one_positive
    (
        F128 {
            hi: 0x3fff000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0x3f800000,
        0,
    ), // one_positive
    (
        F128 {
            hi: 0xbfff000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0xbf800000,
        0,
    ), // one_negative
    (
        F128 {
            hi: 0xbfff000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0xbf800000,
        0,
    ), // one_negative
    (
        F128 {
            hi: 0xbfff000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0xbf800000,
        0,
    ), // one_negative
    (
        F128 {
            hi: 0xbfff000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0xbf800000,
        0,
    ), // one_negative
    (
        F128 {
            hi: 0x3fff800000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0x3fc00000,
        0,
    ), // one_half_positive
    (
        F128 {
            hi: 0x3fff800000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0x3fc00000,
        0,
    ), // one_half_positive
    (
        F128 {
            hi: 0x3fff800000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0x3fc00000,
        0,
    ), // one_half_positive
    (
        F128 {
            hi: 0x3fff800000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0x3fc00000,
        0,
    ), // one_half_positive
    (
        F128 {
            hi: 0xbfff800000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0xbfc00000,
        0,
    ), // one_half_negative
    (
        F128 {
            hi: 0xbfff800000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0xbfc00000,
        0,
    ), // one_half_negative
    (
        F128 {
            hi: 0xbfff800000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0xbfc00000,
        0,
    ), // one_half_negative
    (
        F128 {
            hi: 0xbfff800000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0xbfc00000,
        0,
    ), // one_half_negative
    (
        F128 {
            hi: 0x3f6a000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0x00000001,
        4544,
    ), // min_subnormal32_positive
    (
        F128 {
            hi: 0x3f6a000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0x00000001,
        4544,
    ), // min_subnormal32_positive
    (
        F128 {
            hi: 0x3f6a000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0x00000001,
        4544,
    ), // min_subnormal32_positive
    (
        F128 {
            hi: 0x3f6a000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0x00000001,
        4544,
    ), // min_subnormal32_positive
    (
        F128 {
            hi: 0xbf6a000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0x80000001,
        4544,
    ), // min_subnormal32_negative
    (
        F128 {
            hi: 0xbf6a000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0x80000001,
        4544,
    ), // min_subnormal32_negative
    (
        F128 {
            hi: 0xbf6a000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0x80000001,
        4544,
    ), // min_subnormal32_negative
    (
        F128 {
            hi: 0xbf6a000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0x80000001,
        4544,
    ), // min_subnormal32_negative
    (
        F128 {
            hi: 0x3f80fffffc000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0x007fffff,
        4544,
    ), // max_subnormal32_positive
    (
        F128 {
            hi: 0x3f80fffffc000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0x007fffff,
        4544,
    ), // max_subnormal32_positive
    (
        F128 {
            hi: 0x3f80fffffc000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0x007fffff,
        4544,
    ), // max_subnormal32_positive
    (
        F128 {
            hi: 0x3f80fffffc000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0x007fffff,
        4544,
    ), // max_subnormal32_positive
    (
        F128 {
            hi: 0xbf80fffffc000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0x807fffff,
        4544,
    ), // max_subnormal32_negative
    (
        F128 {
            hi: 0xbf80fffffc000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0x807fffff,
        4544,
    ), // max_subnormal32_negative
    (
        F128 {
            hi: 0xbf80fffffc000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0x807fffff,
        4544,
    ), // max_subnormal32_negative
    (
        F128 {
            hi: 0xbf80fffffc000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0x807fffff,
        4544,
    ), // max_subnormal32_negative
    (
        F128 {
            hi: 0x3f81000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0x00800000,
        0,
    ), // min_normal32_positive
    (
        F128 {
            hi: 0x3f81000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0x00800000,
        0,
    ), // min_normal32_positive
    (
        F128 {
            hi: 0x3f81000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0x00800000,
        0,
    ), // min_normal32_positive
    (
        F128 {
            hi: 0x3f81000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0x00800000,
        0,
    ), // min_normal32_positive
    (
        F128 {
            hi: 0xbf81000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0x80800000,
        0,
    ), // min_normal32_negative
    (
        F128 {
            hi: 0xbf81000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0x80800000,
        0,
    ), // min_normal32_negative
    (
        F128 {
            hi: 0xbf81000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0x80800000,
        0,
    ), // min_normal32_negative
    (
        F128 {
            hi: 0xbf81000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0x80800000,
        0,
    ), // min_normal32_negative
    (
        F128 {
            hi: 0x407efffffe000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0x7f7fffff,
        0,
    ), // max_finite32_positive
    (
        F128 {
            hi: 0x407efffffe000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0x7f7fffff,
        0,
    ), // max_finite32_positive
    (
        F128 {
            hi: 0x407efffffe000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0x7f7fffff,
        0,
    ), // max_finite32_positive
    (
        F128 {
            hi: 0x407efffffe000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0x7f7fffff,
        0,
    ), // max_finite32_positive
    (
        F128 {
            hi: 0xc07efffffe000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0xff7fffff,
        0,
    ), // max_finite32_negative
    (
        F128 {
            hi: 0xc07efffffe000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0xff7fffff,
        0,
    ), // max_finite32_negative
    (
        F128 {
            hi: 0xc07efffffe000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0xff7fffff,
        0,
    ), // max_finite32_negative
    (
        F128 {
            hi: 0xc07efffffe000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0xff7fffff,
        0,
    ), // max_finite32_negative
    (
        F128 {
            hi: 0x407f000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0x7f800000,
        51,
    ), // two_to128_positive
    (
        F128 {
            hi: 0x407f000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0x7f7fffff,
        17,
    ), // two_to128_positive
    (
        F128 {
            hi: 0x407f000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0x7f800000,
        51,
    ), // two_to128_positive
    (
        F128 {
            hi: 0x407f000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0x7f7fffff,
        17,
    ), // two_to128_positive
    (
        F128 {
            hi: 0xc07f000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0xff800000,
        51,
    ), // two_to128_negative
    (
        F128 {
            hi: 0xc07f000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0xff800000,
        51,
    ), // two_to128_negative
    (
        F128 {
            hi: 0xc07f000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0xff7fffff,
        17,
    ), // two_to128_negative
    (
        F128 {
            hi: 0xc07f000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0xff7fffff,
        17,
    ), // two_to128_negative
    (
        F128 {
            hi: 0x4080000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0x7f800000,
        51,
    ), // two_to129_positive
    (
        F128 {
            hi: 0x4080000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0x7f7fffff,
        17,
    ), // two_to129_positive
    (
        F128 {
            hi: 0x4080000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0x7f800000,
        51,
    ), // two_to129_positive
    (
        F128 {
            hi: 0x4080000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0x7f7fffff,
        17,
    ), // two_to129_positive
    (
        F128 {
            hi: 0xc080000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0xff800000,
        51,
    ), // two_to129_negative
    (
        F128 {
            hi: 0xc080000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0xff800000,
        51,
    ), // two_to129_negative
    (
        F128 {
            hi: 0xc080000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0xff7fffff,
        17,
    ), // two_to129_negative
    (
        F128 {
            hi: 0xc080000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0xff7fffff,
        17,
    ), // two_to129_negative
    (
        F128 {
            hi: 0x0000000000000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::NearestEven,
        0x00000000,
        6593,
    ), // min_subnormal128_positive
    (
        F128 {
            hi: 0x0000000000000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::Down,
        0x00000000,
        6593,
    ), // min_subnormal128_positive
    (
        F128 {
            hi: 0x0000000000000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::Up,
        0x00000001,
        6595,
    ), // min_subnormal128_positive
    (
        F128 {
            hi: 0x0000000000000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::ToZero,
        0x00000000,
        6593,
    ), // min_subnormal128_positive
    (
        F128 {
            hi: 0x8000000000000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::NearestEven,
        0x80000000,
        6593,
    ), // min_subnormal128_negative
    (
        F128 {
            hi: 0x8000000000000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::Down,
        0x80000001,
        6595,
    ), // min_subnormal128_negative
    (
        F128 {
            hi: 0x8000000000000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::Up,
        0x80000000,
        6593,
    ), // min_subnormal128_negative
    (
        F128 {
            hi: 0x8000000000000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::ToZero,
        0x80000000,
        6593,
    ), // min_subnormal128_negative
    (
        F128 {
            hi: 0x0001000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0x00000000,
        6593,
    ), // min_normal128_positive
    (
        F128 {
            hi: 0x0001000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0x00000000,
        6593,
    ), // min_normal128_positive
    (
        F128 {
            hi: 0x0001000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0x00000001,
        6595,
    ), // min_normal128_positive
    (
        F128 {
            hi: 0x0001000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0x00000000,
        6593,
    ), // min_normal128_positive
    (
        F128 {
            hi: 0x8001000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0x80000000,
        6593,
    ), // min_normal128_negative
    (
        F128 {
            hi: 0x8001000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0x80000001,
        6595,
    ), // min_normal128_negative
    (
        F128 {
            hi: 0x8001000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0x80000000,
        6593,
    ), // min_normal128_negative
    (
        F128 {
            hi: 0x8001000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0x80000000,
        6593,
    ), // min_normal128_negative
    (
        F128 {
            hi: 0x0000ffffffffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::NearestEven,
        0x00000000,
        6605,
    ), // max_subnormal128_positive
    (
        F128 {
            hi: 0x0000ffffffffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::Down,
        0x00000000,
        6597,
    ), // max_subnormal128_positive
    (
        F128 {
            hi: 0x0000ffffffffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::Up,
        0x00000001,
        6607,
    ), // max_subnormal128_positive
    (
        F128 {
            hi: 0x0000ffffffffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::ToZero,
        0x00000000,
        6597,
    ), // max_subnormal128_positive
    (
        F128 {
            hi: 0x8000ffffffffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::NearestEven,
        0x80000000,
        6605,
    ), // max_subnormal128_negative
    (
        F128 {
            hi: 0x8000ffffffffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::Down,
        0x80000001,
        6607,
    ), // max_subnormal128_negative
    (
        F128 {
            hi: 0x8000ffffffffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::Up,
        0x80000000,
        6597,
    ), // max_subnormal128_negative
    (
        F128 {
            hi: 0x8000ffffffffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::ToZero,
        0x80000000,
        6597,
    ), // max_subnormal128_negative
    (
        F128 {
            hi: 0x7ffeffffffffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::NearestEven,
        0x7f800000,
        63,
    ), // max_finite128_positive
    (
        F128 {
            hi: 0x7ffeffffffffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::Down,
        0x7f7fffff,
        21,
    ), // max_finite128_positive
    (
        F128 {
            hi: 0x7ffeffffffffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::Up,
        0x7f800000,
        63,
    ), // max_finite128_positive
    (
        F128 {
            hi: 0x7ffeffffffffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::ToZero,
        0x7f7fffff,
        21,
    ), // max_finite128_positive
    (
        F128 {
            hi: 0xfffeffffffffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::NearestEven,
        0xff800000,
        63,
    ), // max_finite128_negative
    (
        F128 {
            hi: 0xfffeffffffffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::Down,
        0xff800000,
        63,
    ), // max_finite128_negative
    (
        F128 {
            hi: 0xfffeffffffffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::Up,
        0xff7fffff,
        21,
    ), // max_finite128_negative
    (
        F128 {
            hi: 0xfffeffffffffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::ToZero,
        0xff7fffff,
        21,
    ), // max_finite128_negative
    (
        F128 {
            hi: 0x3fff000000ffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::NearestEven,
        0x3f800000,
        5,
    ), // normal_even_tie_-1_positive
    (
        F128 {
            hi: 0x3fff000000ffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::Down,
        0x3f800000,
        5,
    ), // normal_even_tie_-1_positive
    (
        F128 {
            hi: 0x3fff000000ffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::Up,
        0x3f800001,
        15,
    ), // normal_even_tie_-1_positive
    (
        F128 {
            hi: 0x3fff000000ffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::ToZero,
        0x3f800000,
        5,
    ), // normal_even_tie_-1_positive
    (
        F128 {
            hi: 0xbfff000000ffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::NearestEven,
        0xbf800000,
        5,
    ), // normal_even_tie_-1_negative
    (
        F128 {
            hi: 0xbfff000000ffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::Down,
        0xbf800001,
        15,
    ), // normal_even_tie_-1_negative
    (
        F128 {
            hi: 0xbfff000000ffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::Up,
        0xbf800000,
        5,
    ), // normal_even_tie_-1_negative
    (
        F128 {
            hi: 0xbfff000000ffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::ToZero,
        0xbf800000,
        5,
    ), // normal_even_tie_-1_negative
    (
        F128 {
            hi: 0x3fff000001000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0x3f800000,
        5,
    ), // normal_even_tie_+0_positive
    (
        F128 {
            hi: 0x3fff000001000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0x3f800000,
        5,
    ), // normal_even_tie_+0_positive
    (
        F128 {
            hi: 0x3fff000001000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0x3f800001,
        15,
    ), // normal_even_tie_+0_positive
    (
        F128 {
            hi: 0x3fff000001000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0x3f800000,
        5,
    ), // normal_even_tie_+0_positive
    (
        F128 {
            hi: 0xbfff000001000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0xbf800000,
        5,
    ), // normal_even_tie_+0_negative
    (
        F128 {
            hi: 0xbfff000001000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0xbf800001,
        15,
    ), // normal_even_tie_+0_negative
    (
        F128 {
            hi: 0xbfff000001000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0xbf800000,
        5,
    ), // normal_even_tie_+0_negative
    (
        F128 {
            hi: 0xbfff000001000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0xbf800000,
        5,
    ), // normal_even_tie_+0_negative
    (
        F128 {
            hi: 0x3fff000001000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::NearestEven,
        0x3f800001,
        15,
    ), // normal_even_tie_+1_positive
    (
        F128 {
            hi: 0x3fff000001000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::Down,
        0x3f800000,
        5,
    ), // normal_even_tie_+1_positive
    (
        F128 {
            hi: 0x3fff000001000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::Up,
        0x3f800001,
        15,
    ), // normal_even_tie_+1_positive
    (
        F128 {
            hi: 0x3fff000001000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::ToZero,
        0x3f800000,
        5,
    ), // normal_even_tie_+1_positive
    (
        F128 {
            hi: 0xbfff000001000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::NearestEven,
        0xbf800001,
        15,
    ), // normal_even_tie_+1_negative
    (
        F128 {
            hi: 0xbfff000001000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::Down,
        0xbf800001,
        15,
    ), // normal_even_tie_+1_negative
    (
        F128 {
            hi: 0xbfff000001000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::Up,
        0xbf800000,
        5,
    ), // normal_even_tie_+1_negative
    (
        F128 {
            hi: 0xbfff000001000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::ToZero,
        0xbf800000,
        5,
    ), // normal_even_tie_+1_negative
    (
        F128 {
            hi: 0x3fff000002ffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::NearestEven,
        0x3f800001,
        5,
    ), // normal_odd_tie_-1_positive
    (
        F128 {
            hi: 0x3fff000002ffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::Down,
        0x3f800001,
        5,
    ), // normal_odd_tie_-1_positive
    (
        F128 {
            hi: 0x3fff000002ffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::Up,
        0x3f800002,
        15,
    ), // normal_odd_tie_-1_positive
    (
        F128 {
            hi: 0x3fff000002ffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::ToZero,
        0x3f800001,
        5,
    ), // normal_odd_tie_-1_positive
    (
        F128 {
            hi: 0xbfff000002ffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::NearestEven,
        0xbf800001,
        5,
    ), // normal_odd_tie_-1_negative
    (
        F128 {
            hi: 0xbfff000002ffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::Down,
        0xbf800002,
        15,
    ), // normal_odd_tie_-1_negative
    (
        F128 {
            hi: 0xbfff000002ffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::Up,
        0xbf800001,
        5,
    ), // normal_odd_tie_-1_negative
    (
        F128 {
            hi: 0xbfff000002ffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::ToZero,
        0xbf800001,
        5,
    ), // normal_odd_tie_-1_negative
    (
        F128 {
            hi: 0x3fff000003000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0x3f800002,
        15,
    ), // normal_odd_tie_+0_positive
    (
        F128 {
            hi: 0x3fff000003000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0x3f800001,
        5,
    ), // normal_odd_tie_+0_positive
    (
        F128 {
            hi: 0x3fff000003000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0x3f800002,
        15,
    ), // normal_odd_tie_+0_positive
    (
        F128 {
            hi: 0x3fff000003000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0x3f800001,
        5,
    ), // normal_odd_tie_+0_positive
    (
        F128 {
            hi: 0xbfff000003000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0xbf800002,
        15,
    ), // normal_odd_tie_+0_negative
    (
        F128 {
            hi: 0xbfff000003000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0xbf800002,
        15,
    ), // normal_odd_tie_+0_negative
    (
        F128 {
            hi: 0xbfff000003000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0xbf800001,
        5,
    ), // normal_odd_tie_+0_negative
    (
        F128 {
            hi: 0xbfff000003000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0xbf800001,
        5,
    ), // normal_odd_tie_+0_negative
    (
        F128 {
            hi: 0x3fff000003000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::NearestEven,
        0x3f800002,
        15,
    ), // normal_odd_tie_+1_positive
    (
        F128 {
            hi: 0x3fff000003000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::Down,
        0x3f800001,
        5,
    ), // normal_odd_tie_+1_positive
    (
        F128 {
            hi: 0x3fff000003000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::Up,
        0x3f800002,
        15,
    ), // normal_odd_tie_+1_positive
    (
        F128 {
            hi: 0x3fff000003000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::ToZero,
        0x3f800001,
        5,
    ), // normal_odd_tie_+1_positive
    (
        F128 {
            hi: 0xbfff000003000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::NearestEven,
        0xbf800002,
        15,
    ), // normal_odd_tie_+1_negative
    (
        F128 {
            hi: 0xbfff000003000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::Down,
        0xbf800002,
        15,
    ), // normal_odd_tie_+1_negative
    (
        F128 {
            hi: 0xbfff000003000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::Up,
        0xbf800001,
        5,
    ), // normal_odd_tie_+1_negative
    (
        F128 {
            hi: 0xbfff000003000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::ToZero,
        0xbf800001,
        5,
    ), // normal_odd_tie_+1_negative
    (
        F128 {
            hi: 0x3f68ffffffffffff,
            lo: 0xfffffffffffffffe,
        },
        Ext80Rounding::NearestEven,
        0x00000000,
        6605,
    ), // subzero_half_-1_positive
    (
        F128 {
            hi: 0x3f68ffffffffffff,
            lo: 0xfffffffffffffffe,
        },
        Ext80Rounding::Down,
        0x00000000,
        6597,
    ), // subzero_half_-1_positive
    (
        F128 {
            hi: 0x3f68ffffffffffff,
            lo: 0xfffffffffffffffe,
        },
        Ext80Rounding::Up,
        0x00000001,
        6607,
    ), // subzero_half_-1_positive
    (
        F128 {
            hi: 0x3f68ffffffffffff,
            lo: 0xfffffffffffffffe,
        },
        Ext80Rounding::ToZero,
        0x00000000,
        6597,
    ), // subzero_half_-1_positive
    (
        F128 {
            hi: 0xbf68ffffffffffff,
            lo: 0xfffffffffffffffe,
        },
        Ext80Rounding::NearestEven,
        0x80000000,
        6605,
    ), // subzero_half_-1_negative
    (
        F128 {
            hi: 0xbf68ffffffffffff,
            lo: 0xfffffffffffffffe,
        },
        Ext80Rounding::Down,
        0x80000001,
        6607,
    ), // subzero_half_-1_negative
    (
        F128 {
            hi: 0xbf68ffffffffffff,
            lo: 0xfffffffffffffffe,
        },
        Ext80Rounding::Up,
        0x80000000,
        6597,
    ), // subzero_half_-1_negative
    (
        F128 {
            hi: 0xbf68ffffffffffff,
            lo: 0xfffffffffffffffe,
        },
        Ext80Rounding::ToZero,
        0x80000000,
        6597,
    ), // subzero_half_-1_negative
    (
        F128 {
            hi: 0x3f69000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0x00000000,
        6593,
    ), // subzero_half_+0_positive
    (
        F128 {
            hi: 0x3f69000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0x00000000,
        6593,
    ), // subzero_half_+0_positive
    (
        F128 {
            hi: 0x3f69000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0x00000001,
        6595,
    ), // subzero_half_+0_positive
    (
        F128 {
            hi: 0x3f69000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0x00000000,
        6593,
    ), // subzero_half_+0_positive
    (
        F128 {
            hi: 0xbf69000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0x80000000,
        6593,
    ), // subzero_half_+0_negative
    (
        F128 {
            hi: 0xbf69000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0x80000001,
        6595,
    ), // subzero_half_+0_negative
    (
        F128 {
            hi: 0xbf69000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0x80000000,
        6593,
    ), // subzero_half_+0_negative
    (
        F128 {
            hi: 0xbf69000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0x80000000,
        6593,
    ), // subzero_half_+0_negative
    (
        F128 {
            hi: 0x3f69000000000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::NearestEven,
        0x00000001,
        6599,
    ), // subzero_half_+1_positive
    (
        F128 {
            hi: 0x3f69000000000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::Down,
        0x00000000,
        6597,
    ), // subzero_half_+1_positive
    (
        F128 {
            hi: 0x3f69000000000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::Up,
        0x00000001,
        6607,
    ), // subzero_half_+1_positive
    (
        F128 {
            hi: 0x3f69000000000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::ToZero,
        0x00000000,
        6597,
    ), // subzero_half_+1_positive
    (
        F128 {
            hi: 0xbf69000000000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::NearestEven,
        0x80000001,
        6599,
    ), // subzero_half_+1_negative
    (
        F128 {
            hi: 0xbf69000000000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::Down,
        0x80000001,
        6607,
    ), // subzero_half_+1_negative
    (
        F128 {
            hi: 0xbf69000000000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::Up,
        0x80000000,
        6597,
    ), // subzero_half_+1_negative
    (
        F128 {
            hi: 0xbf69000000000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::ToZero,
        0x80000000,
        6597,
    ), // subzero_half_+1_negative
    (
        F128 {
            hi: 0x3f6a7fffffffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::NearestEven,
        0x00000001,
        6605,
    ), // subodd_half_-1_positive
    (
        F128 {
            hi: 0x3f6a7fffffffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::Down,
        0x00000001,
        6597,
    ), // subodd_half_-1_positive
    (
        F128 {
            hi: 0x3f6a7fffffffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::Up,
        0x00000002,
        6607,
    ), // subodd_half_-1_positive
    (
        F128 {
            hi: 0x3f6a7fffffffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::ToZero,
        0x00000001,
        6597,
    ), // subodd_half_-1_positive
    (
        F128 {
            hi: 0xbf6a7fffffffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::NearestEven,
        0x80000001,
        6605,
    ), // subodd_half_-1_negative
    (
        F128 {
            hi: 0xbf6a7fffffffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::Down,
        0x80000002,
        6607,
    ), // subodd_half_-1_negative
    (
        F128 {
            hi: 0xbf6a7fffffffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::Up,
        0x80000001,
        6597,
    ), // subodd_half_-1_negative
    (
        F128 {
            hi: 0xbf6a7fffffffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::ToZero,
        0x80000001,
        6597,
    ), // subodd_half_-1_negative
    (
        F128 {
            hi: 0x3f6a800000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0x00000002,
        6595,
    ), // subodd_half_+0_positive
    (
        F128 {
            hi: 0x3f6a800000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0x00000001,
        6593,
    ), // subodd_half_+0_positive
    (
        F128 {
            hi: 0x3f6a800000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0x00000002,
        6595,
    ), // subodd_half_+0_positive
    (
        F128 {
            hi: 0x3f6a800000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0x00000001,
        6593,
    ), // subodd_half_+0_positive
    (
        F128 {
            hi: 0xbf6a800000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0x80000002,
        6595,
    ), // subodd_half_+0_negative
    (
        F128 {
            hi: 0xbf6a800000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0x80000002,
        6595,
    ), // subodd_half_+0_negative
    (
        F128 {
            hi: 0xbf6a800000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0x80000001,
        6593,
    ), // subodd_half_+0_negative
    (
        F128 {
            hi: 0xbf6a800000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0x80000001,
        6593,
    ), // subodd_half_+0_negative
    (
        F128 {
            hi: 0x3f6a800000000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::NearestEven,
        0x00000002,
        6599,
    ), // subodd_half_+1_positive
    (
        F128 {
            hi: 0x3f6a800000000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::Down,
        0x00000001,
        6597,
    ), // subodd_half_+1_positive
    (
        F128 {
            hi: 0x3f6a800000000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::Up,
        0x00000002,
        6607,
    ), // subodd_half_+1_positive
    (
        F128 {
            hi: 0x3f6a800000000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::ToZero,
        0x00000001,
        6597,
    ), // subodd_half_+1_positive
    (
        F128 {
            hi: 0xbf6a800000000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::NearestEven,
        0x80000002,
        6599,
    ), // subodd_half_+1_negative
    (
        F128 {
            hi: 0xbf6a800000000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::Down,
        0x80000002,
        6607,
    ), // subodd_half_+1_negative
    (
        F128 {
            hi: 0xbf6a800000000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::Up,
        0x80000001,
        6597,
    ), // subodd_half_+1_negative
    (
        F128 {
            hi: 0xbf6a800000000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::ToZero,
        0x80000001,
        6597,
    ), // subodd_half_+1_negative
    (
        F128 {
            hi: 0x3f6b3fffffffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::NearestEven,
        0x00000002,
        6605,
    ), // subeven_half_-1_positive
    (
        F128 {
            hi: 0x3f6b3fffffffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::Down,
        0x00000002,
        6597,
    ), // subeven_half_-1_positive
    (
        F128 {
            hi: 0x3f6b3fffffffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::Up,
        0x00000003,
        6607,
    ), // subeven_half_-1_positive
    (
        F128 {
            hi: 0x3f6b3fffffffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::ToZero,
        0x00000002,
        6597,
    ), // subeven_half_-1_positive
    (
        F128 {
            hi: 0xbf6b3fffffffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::NearestEven,
        0x80000002,
        6605,
    ), // subeven_half_-1_negative
    (
        F128 {
            hi: 0xbf6b3fffffffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::Down,
        0x80000003,
        6607,
    ), // subeven_half_-1_negative
    (
        F128 {
            hi: 0xbf6b3fffffffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::Up,
        0x80000002,
        6597,
    ), // subeven_half_-1_negative
    (
        F128 {
            hi: 0xbf6b3fffffffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::ToZero,
        0x80000002,
        6597,
    ), // subeven_half_-1_negative
    (
        F128 {
            hi: 0x3f6b400000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0x00000002,
        6593,
    ), // subeven_half_+0_positive
    (
        F128 {
            hi: 0x3f6b400000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0x00000002,
        6593,
    ), // subeven_half_+0_positive
    (
        F128 {
            hi: 0x3f6b400000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0x00000003,
        6595,
    ), // subeven_half_+0_positive
    (
        F128 {
            hi: 0x3f6b400000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0x00000002,
        6593,
    ), // subeven_half_+0_positive
    (
        F128 {
            hi: 0xbf6b400000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0x80000002,
        6593,
    ), // subeven_half_+0_negative
    (
        F128 {
            hi: 0xbf6b400000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0x80000003,
        6595,
    ), // subeven_half_+0_negative
    (
        F128 {
            hi: 0xbf6b400000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0x80000002,
        6593,
    ), // subeven_half_+0_negative
    (
        F128 {
            hi: 0xbf6b400000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0x80000002,
        6593,
    ), // subeven_half_+0_negative
    (
        F128 {
            hi: 0x3f6b400000000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::NearestEven,
        0x00000003,
        6599,
    ), // subeven_half_+1_positive
    (
        F128 {
            hi: 0x3f6b400000000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::Down,
        0x00000002,
        6597,
    ), // subeven_half_+1_positive
    (
        F128 {
            hi: 0x3f6b400000000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::Up,
        0x00000003,
        6607,
    ), // subeven_half_+1_positive
    (
        F128 {
            hi: 0x3f6b400000000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::ToZero,
        0x00000002,
        6597,
    ), // subeven_half_+1_positive
    (
        F128 {
            hi: 0xbf6b400000000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::NearestEven,
        0x80000003,
        6599,
    ), // subeven_half_+1_negative
    (
        F128 {
            hi: 0xbf6b400000000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::Down,
        0x80000003,
        6607,
    ), // subeven_half_+1_negative
    (
        F128 {
            hi: 0xbf6b400000000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::Up,
        0x80000002,
        6597,
    ), // subeven_half_+1_negative
    (
        F128 {
            hi: 0xbf6b400000000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::ToZero,
        0x80000002,
        6597,
    ), // subeven_half_+1_negative
    (
        F128 {
            hi: 0x3f80fffffdffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::NearestEven,
        0x007fffff,
        6605,
    ), // minnormal_half_-1_positive
    (
        F128 {
            hi: 0x3f80fffffdffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::Down,
        0x007fffff,
        6597,
    ), // minnormal_half_-1_positive
    (
        F128 {
            hi: 0x3f80fffffdffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::Up,
        0x00800000,
        6863,
    ), // minnormal_half_-1_positive
    (
        F128 {
            hi: 0x3f80fffffdffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::ToZero,
        0x007fffff,
        6597,
    ), // minnormal_half_-1_positive
    (
        F128 {
            hi: 0xbf80fffffdffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::NearestEven,
        0x807fffff,
        6605,
    ), // minnormal_half_-1_negative
    (
        F128 {
            hi: 0xbf80fffffdffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::Down,
        0x80800000,
        6863,
    ), // minnormal_half_-1_negative
    (
        F128 {
            hi: 0xbf80fffffdffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::Up,
        0x807fffff,
        6597,
    ), // minnormal_half_-1_negative
    (
        F128 {
            hi: 0xbf80fffffdffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::ToZero,
        0x807fffff,
        6597,
    ), // minnormal_half_-1_negative
    (
        F128 {
            hi: 0x3f80fffffe000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0x00800000,
        6851,
    ), // minnormal_half_+0_positive
    (
        F128 {
            hi: 0x3f80fffffe000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0x007fffff,
        6593,
    ), // minnormal_half_+0_positive
    (
        F128 {
            hi: 0x3f80fffffe000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0x00800000,
        6851,
    ), // minnormal_half_+0_positive
    (
        F128 {
            hi: 0x3f80fffffe000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0x007fffff,
        6593,
    ), // minnormal_half_+0_positive
    (
        F128 {
            hi: 0xbf80fffffe000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0x80800000,
        6851,
    ), // minnormal_half_+0_negative
    (
        F128 {
            hi: 0xbf80fffffe000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0x80800000,
        6851,
    ), // minnormal_half_+0_negative
    (
        F128 {
            hi: 0xbf80fffffe000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0x807fffff,
        6593,
    ), // minnormal_half_+0_negative
    (
        F128 {
            hi: 0xbf80fffffe000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0x807fffff,
        6593,
    ), // minnormal_half_+0_negative
    (
        F128 {
            hi: 0x3f80fffffe000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::NearestEven,
        0x00800000,
        6855,
    ), // minnormal_half_+1_positive
    (
        F128 {
            hi: 0x3f80fffffe000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::Down,
        0x007fffff,
        6597,
    ), // minnormal_half_+1_positive
    (
        F128 {
            hi: 0x3f80fffffe000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::Up,
        0x00800000,
        591,
    ), // minnormal_half_+1_positive
    (
        F128 {
            hi: 0x3f80fffffe000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::ToZero,
        0x007fffff,
        6597,
    ), // minnormal_half_+1_positive
    (
        F128 {
            hi: 0xbf80fffffe000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::NearestEven,
        0x80800000,
        6855,
    ), // minnormal_half_+1_negative
    (
        F128 {
            hi: 0xbf80fffffe000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::Down,
        0x80800000,
        591,
    ), // minnormal_half_+1_negative
    (
        F128 {
            hi: 0xbf80fffffe000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::Up,
        0x807fffff,
        6597,
    ), // minnormal_half_+1_negative
    (
        F128 {
            hi: 0xbf80fffffe000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::ToZero,
        0x807fffff,
        6597,
    ), // minnormal_half_+1_negative
    (
        F128 {
            hi: 0x3f80fffffeffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::NearestEven,
        0x00800000,
        6855,
    ), // minnormal_precision_half_-1_positive
    (
        F128 {
            hi: 0x3f80fffffeffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::Down,
        0x007fffff,
        6597,
    ), // minnormal_precision_half_-1_positive
    (
        F128 {
            hi: 0x3f80fffffeffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::Up,
        0x00800000,
        591,
    ), // minnormal_precision_half_-1_positive
    (
        F128 {
            hi: 0x3f80fffffeffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::ToZero,
        0x007fffff,
        6597,
    ), // minnormal_precision_half_-1_positive
    (
        F128 {
            hi: 0xbf80fffffeffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::NearestEven,
        0x80800000,
        6855,
    ), // minnormal_precision_half_-1_negative
    (
        F128 {
            hi: 0xbf80fffffeffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::Down,
        0x80800000,
        591,
    ), // minnormal_precision_half_-1_negative
    (
        F128 {
            hi: 0xbf80fffffeffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::Up,
        0x807fffff,
        6597,
    ), // minnormal_precision_half_-1_negative
    (
        F128 {
            hi: 0xbf80fffffeffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::ToZero,
        0x807fffff,
        6597,
    ), // minnormal_precision_half_-1_negative
    (
        F128 {
            hi: 0x3f80ffffff000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0x00800000,
        591,
    ), // minnormal_precision_half_+0_positive
    (
        F128 {
            hi: 0x3f80ffffff000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0x007fffff,
        6597,
    ), // minnormal_precision_half_+0_positive
    (
        F128 {
            hi: 0x3f80ffffff000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0x00800000,
        591,
    ), // minnormal_precision_half_+0_positive
    (
        F128 {
            hi: 0x3f80ffffff000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0x007fffff,
        6597,
    ), // minnormal_precision_half_+0_positive
    (
        F128 {
            hi: 0xbf80ffffff000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0x80800000,
        591,
    ), // minnormal_precision_half_+0_negative
    (
        F128 {
            hi: 0xbf80ffffff000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0x80800000,
        591,
    ), // minnormal_precision_half_+0_negative
    (
        F128 {
            hi: 0xbf80ffffff000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0x807fffff,
        6597,
    ), // minnormal_precision_half_+0_negative
    (
        F128 {
            hi: 0xbf80ffffff000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0x807fffff,
        6597,
    ), // minnormal_precision_half_+0_negative
    (
        F128 {
            hi: 0x3f80ffffff000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::NearestEven,
        0x00800000,
        591,
    ), // minnormal_precision_half_+1_positive
    (
        F128 {
            hi: 0x3f80ffffff000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::Down,
        0x007fffff,
        6597,
    ), // minnormal_precision_half_+1_positive
    (
        F128 {
            hi: 0x3f80ffffff000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::Up,
        0x00800000,
        591,
    ), // minnormal_precision_half_+1_positive
    (
        F128 {
            hi: 0x3f80ffffff000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::ToZero,
        0x007fffff,
        6597,
    ), // minnormal_precision_half_+1_positive
    (
        F128 {
            hi: 0xbf80ffffff000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::NearestEven,
        0x80800000,
        591,
    ), // minnormal_precision_half_+1_negative
    (
        F128 {
            hi: 0xbf80ffffff000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::Down,
        0x80800000,
        591,
    ), // minnormal_precision_half_+1_negative
    (
        F128 {
            hi: 0xbf80ffffff000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::Up,
        0x807fffff,
        6597,
    ), // minnormal_precision_half_+1_negative
    (
        F128 {
            hi: 0xbf80ffffff000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::ToZero,
        0x807fffff,
        6597,
    ), // minnormal_precision_half_+1_negative
    (
        F128 {
            hi: 0x407efffffeffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::NearestEven,
        0x7f7fffff,
        5,
    ), // overflow_half_-1_positive
    (
        F128 {
            hi: 0x407efffffeffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::Down,
        0x7f7fffff,
        5,
    ), // overflow_half_-1_positive
    (
        F128 {
            hi: 0x407efffffeffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::Up,
        0x7f800000,
        63,
    ), // overflow_half_-1_positive
    (
        F128 {
            hi: 0x407efffffeffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::ToZero,
        0x7f7fffff,
        5,
    ), // overflow_half_-1_positive
    (
        F128 {
            hi: 0xc07efffffeffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::NearestEven,
        0xff7fffff,
        5,
    ), // overflow_half_-1_negative
    (
        F128 {
            hi: 0xc07efffffeffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::Down,
        0xff800000,
        63,
    ), // overflow_half_-1_negative
    (
        F128 {
            hi: 0xc07efffffeffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::Up,
        0xff7fffff,
        5,
    ), // overflow_half_-1_negative
    (
        F128 {
            hi: 0xc07efffffeffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::ToZero,
        0xff7fffff,
        5,
    ), // overflow_half_-1_negative
    (
        F128 {
            hi: 0x407effffff000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0x7f800000,
        63,
    ), // overflow_half_+0_positive
    (
        F128 {
            hi: 0x407effffff000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0x7f7fffff,
        5,
    ), // overflow_half_+0_positive
    (
        F128 {
            hi: 0x407effffff000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0x7f800000,
        63,
    ), // overflow_half_+0_positive
    (
        F128 {
            hi: 0x407effffff000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0x7f7fffff,
        5,
    ), // overflow_half_+0_positive
    (
        F128 {
            hi: 0xc07effffff000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0xff800000,
        63,
    ), // overflow_half_+0_negative
    (
        F128 {
            hi: 0xc07effffff000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0xff800000,
        63,
    ), // overflow_half_+0_negative
    (
        F128 {
            hi: 0xc07effffff000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0xff7fffff,
        5,
    ), // overflow_half_+0_negative
    (
        F128 {
            hi: 0xc07effffff000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0xff7fffff,
        5,
    ), // overflow_half_+0_negative
    (
        F128 {
            hi: 0x407effffff000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::NearestEven,
        0x7f800000,
        63,
    ), // overflow_half_+1_positive
    (
        F128 {
            hi: 0x407effffff000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::Down,
        0x7f7fffff,
        5,
    ), // overflow_half_+1_positive
    (
        F128 {
            hi: 0x407effffff000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::Up,
        0x7f800000,
        63,
    ), // overflow_half_+1_positive
    (
        F128 {
            hi: 0x407effffff000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::ToZero,
        0x7f7fffff,
        5,
    ), // overflow_half_+1_positive
    (
        F128 {
            hi: 0xc07effffff000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::NearestEven,
        0xff800000,
        63,
    ), // overflow_half_+1_negative
    (
        F128 {
            hi: 0xc07effffff000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::Down,
        0xff800000,
        63,
    ), // overflow_half_+1_negative
    (
        F128 {
            hi: 0xc07effffff000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::Up,
        0xff7fffff,
        5,
    ), // overflow_half_+1_negative
    (
        F128 {
            hi: 0xc07effffff000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::ToZero,
        0xff7fffff,
        5,
    ), // overflow_half_+1_negative
    (
        F128 {
            hi: 0x3f80fffffd000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0x007fffff,
        6597,
    ), // minnormal_three_quarter_below_positive
    (
        F128 {
            hi: 0x3f80fffffd000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0x007fffff,
        6597,
    ), // minnormal_three_quarter_below_positive
    (
        F128 {
            hi: 0x3f80fffffd000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0x00800000,
        6863,
    ), // minnormal_three_quarter_below_positive
    (
        F128 {
            hi: 0x3f80fffffd000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0x007fffff,
        6597,
    ), // minnormal_three_quarter_below_positive
    (
        F128 {
            hi: 0xbf80fffffd000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0x807fffff,
        6597,
    ), // minnormal_three_quarter_below_negative
    (
        F128 {
            hi: 0xbf80fffffd000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0x80800000,
        6863,
    ), // minnormal_three_quarter_below_negative
    (
        F128 {
            hi: 0xbf80fffffd000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0x807fffff,
        6597,
    ), // minnormal_three_quarter_below_negative
    (
        F128 {
            hi: 0xbf80fffffd000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0x807fffff,
        6597,
    ), // minnormal_three_quarter_below_negative
    (
        F128 {
            hi: 0x407efffffe000000,
            lo: 0x0002000000000000,
        },
        Ext80Rounding::NearestEven,
        0x7f7fffff,
        5,
    ), // maxfinite_plus_ext80_ulp_positive
    (
        F128 {
            hi: 0x407efffffe000000,
            lo: 0x0002000000000000,
        },
        Ext80Rounding::Down,
        0x7f7fffff,
        5,
    ), // maxfinite_plus_ext80_ulp_positive
    (
        F128 {
            hi: 0x407efffffe000000,
            lo: 0x0002000000000000,
        },
        Ext80Rounding::Up,
        0x7f800000,
        63,
    ), // maxfinite_plus_ext80_ulp_positive
    (
        F128 {
            hi: 0x407efffffe000000,
            lo: 0x0002000000000000,
        },
        Ext80Rounding::ToZero,
        0x7f7fffff,
        5,
    ), // maxfinite_plus_ext80_ulp_positive
    (
        F128 {
            hi: 0xc07efffffe000000,
            lo: 0x0002000000000000,
        },
        Ext80Rounding::NearestEven,
        0xff7fffff,
        5,
    ), // maxfinite_plus_ext80_ulp_negative
    (
        F128 {
            hi: 0xc07efffffe000000,
            lo: 0x0002000000000000,
        },
        Ext80Rounding::Down,
        0xff800000,
        63,
    ), // maxfinite_plus_ext80_ulp_negative
    (
        F128 {
            hi: 0xc07efffffe000000,
            lo: 0x0002000000000000,
        },
        Ext80Rounding::Up,
        0xff7fffff,
        5,
    ), // maxfinite_plus_ext80_ulp_negative
    (
        F128 {
            hi: 0xc07efffffe000000,
            lo: 0x0002000000000000,
        },
        Ext80Rounding::ToZero,
        0xff7fffff,
        5,
    ), // maxfinite_plus_ext80_ulp_negative
    (
        F128 {
            hi: 0x407efffffe800000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0x7f7fffff,
        5,
    ), // maxfinite_plus_quarter_ulp_positive
    (
        F128 {
            hi: 0x407efffffe800000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0x7f7fffff,
        5,
    ), // maxfinite_plus_quarter_ulp_positive
    (
        F128 {
            hi: 0x407efffffe800000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0x7f800000,
        63,
    ), // maxfinite_plus_quarter_ulp_positive
    (
        F128 {
            hi: 0x407efffffe800000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0x7f7fffff,
        5,
    ), // maxfinite_plus_quarter_ulp_positive
    (
        F128 {
            hi: 0xc07efffffe800000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0xff7fffff,
        5,
    ), // maxfinite_plus_quarter_ulp_negative
    (
        F128 {
            hi: 0xc07efffffe800000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0xff800000,
        63,
    ), // maxfinite_plus_quarter_ulp_negative
    (
        F128 {
            hi: 0xc07efffffe800000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0xff7fffff,
        5,
    ), // maxfinite_plus_quarter_ulp_negative
    (
        F128 {
            hi: 0xc07efffffe800000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0xff7fffff,
        5,
    ), // maxfinite_plus_quarter_ulp_negative
    (
        F128 {
            hi: 0x407efffffdffffff,
            lo: 0xfffe000000000000,
        },
        Ext80Rounding::NearestEven,
        0x7f7fffff,
        15,
    ), // maxfinite_minus_ext80_ulp_positive
    (
        F128 {
            hi: 0x407efffffdffffff,
            lo: 0xfffe000000000000,
        },
        Ext80Rounding::Down,
        0x7f7ffffe,
        5,
    ), // maxfinite_minus_ext80_ulp_positive
    (
        F128 {
            hi: 0x407efffffdffffff,
            lo: 0xfffe000000000000,
        },
        Ext80Rounding::Up,
        0x7f7fffff,
        15,
    ), // maxfinite_minus_ext80_ulp_positive
    (
        F128 {
            hi: 0x407efffffdffffff,
            lo: 0xfffe000000000000,
        },
        Ext80Rounding::ToZero,
        0x7f7ffffe,
        5,
    ), // maxfinite_minus_ext80_ulp_positive
    (
        F128 {
            hi: 0xc07efffffdffffff,
            lo: 0xfffe000000000000,
        },
        Ext80Rounding::NearestEven,
        0xff7fffff,
        15,
    ), // maxfinite_minus_ext80_ulp_negative
    (
        F128 {
            hi: 0xc07efffffdffffff,
            lo: 0xfffe000000000000,
        },
        Ext80Rounding::Down,
        0xff7fffff,
        15,
    ), // maxfinite_minus_ext80_ulp_negative
    (
        F128 {
            hi: 0xc07efffffdffffff,
            lo: 0xfffe000000000000,
        },
        Ext80Rounding::Up,
        0xff7ffffe,
        5,
    ), // maxfinite_minus_ext80_ulp_negative
    (
        F128 {
            hi: 0xc07efffffdffffff,
            lo: 0xfffe000000000000,
        },
        Ext80Rounding::ToZero,
        0xff7ffffe,
        5,
    ), // maxfinite_minus_ext80_ulp_negative
    (
        F128 {
            hi: 0x3fff000001000000,
            lo: 0x0002000000000000,
        },
        Ext80Rounding::NearestEven,
        0x3f800001,
        15,
    ), // ext80_qword_even_double_round_positive
    (
        F128 {
            hi: 0x3fff000001000000,
            lo: 0x0002000000000000,
        },
        Ext80Rounding::Down,
        0x3f800000,
        5,
    ), // ext80_qword_even_double_round_positive
    (
        F128 {
            hi: 0x3fff000001000000,
            lo: 0x0002000000000000,
        },
        Ext80Rounding::Up,
        0x3f800001,
        15,
    ), // ext80_qword_even_double_round_positive
    (
        F128 {
            hi: 0x3fff000001000000,
            lo: 0x0002000000000000,
        },
        Ext80Rounding::ToZero,
        0x3f800000,
        5,
    ), // ext80_qword_even_double_round_positive
    (
        F128 {
            hi: 0xbfff000001000000,
            lo: 0x0002000000000000,
        },
        Ext80Rounding::NearestEven,
        0xbf800001,
        15,
    ), // ext80_qword_even_double_round_negative
    (
        F128 {
            hi: 0xbfff000001000000,
            lo: 0x0002000000000000,
        },
        Ext80Rounding::Down,
        0xbf800001,
        15,
    ), // ext80_qword_even_double_round_negative
    (
        F128 {
            hi: 0xbfff000001000000,
            lo: 0x0002000000000000,
        },
        Ext80Rounding::Up,
        0xbf800000,
        5,
    ), // ext80_qword_even_double_round_negative
    (
        F128 {
            hi: 0xbfff000001000000,
            lo: 0x0002000000000000,
        },
        Ext80Rounding::ToZero,
        0xbf800000,
        5,
    ), // ext80_qword_even_double_round_negative
    (
        F128 {
            hi: 0x3fff000002ffffff,
            lo: 0xfffe000000000000,
        },
        Ext80Rounding::NearestEven,
        0x3f800001,
        5,
    ), // ext80_qword_odd_double_round_positive
    (
        F128 {
            hi: 0x3fff000002ffffff,
            lo: 0xfffe000000000000,
        },
        Ext80Rounding::Down,
        0x3f800001,
        5,
    ), // ext80_qword_odd_double_round_positive
    (
        F128 {
            hi: 0x3fff000002ffffff,
            lo: 0xfffe000000000000,
        },
        Ext80Rounding::Up,
        0x3f800002,
        15,
    ), // ext80_qword_odd_double_round_positive
    (
        F128 {
            hi: 0x3fff000002ffffff,
            lo: 0xfffe000000000000,
        },
        Ext80Rounding::ToZero,
        0x3f800001,
        5,
    ), // ext80_qword_odd_double_round_positive
    (
        F128 {
            hi: 0xbfff000002ffffff,
            lo: 0xfffe000000000000,
        },
        Ext80Rounding::NearestEven,
        0xbf800001,
        5,
    ), // ext80_qword_odd_double_round_negative
    (
        F128 {
            hi: 0xbfff000002ffffff,
            lo: 0xfffe000000000000,
        },
        Ext80Rounding::Down,
        0xbf800002,
        15,
    ), // ext80_qword_odd_double_round_negative
    (
        F128 {
            hi: 0xbfff000002ffffff,
            lo: 0xfffe000000000000,
        },
        Ext80Rounding::Up,
        0xbf800001,
        5,
    ), // ext80_qword_odd_double_round_negative
    (
        F128 {
            hi: 0xbfff000002ffffff,
            lo: 0xfffe000000000000,
        },
        Ext80Rounding::ToZero,
        0xbf800001,
        5,
    ), // ext80_qword_odd_double_round_negative
    (
        F128 {
            hi: 0x3f6a7fffffc00000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0x00000001,
        6605,
    ), // precision_subodd_double_round_positive
    (
        F128 {
            hi: 0x3f6a7fffffc00000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0x00000001,
        6597,
    ), // precision_subodd_double_round_positive
    (
        F128 {
            hi: 0x3f6a7fffffc00000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0x00000002,
        6607,
    ), // precision_subodd_double_round_positive
    (
        F128 {
            hi: 0x3f6a7fffffc00000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0x00000001,
        6597,
    ), // precision_subodd_double_round_positive
    (
        F128 {
            hi: 0xbf6a7fffffc00000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0x80000001,
        6605,
    ), // precision_subodd_double_round_negative
    (
        F128 {
            hi: 0xbf6a7fffffc00000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0x80000002,
        6607,
    ), // precision_subodd_double_round_negative
    (
        F128 {
            hi: 0xbf6a7fffffc00000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0x80000001,
        6597,
    ), // precision_subodd_double_round_negative
    (
        F128 {
            hi: 0xbf6a7fffffc00000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0x80000001,
        6597,
    ), // precision_subodd_double_round_negative
    (
        F128 {
            hi: 0x3f6b400000200000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0x00000003,
        6599,
    ), // precision_subeven_double_round_positive
    (
        F128 {
            hi: 0x3f6b400000200000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0x00000002,
        6597,
    ), // precision_subeven_double_round_positive
    (
        F128 {
            hi: 0x3f6b400000200000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0x00000003,
        6607,
    ), // precision_subeven_double_round_positive
    (
        F128 {
            hi: 0x3f6b400000200000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0x00000002,
        6597,
    ), // precision_subeven_double_round_positive
    (
        F128 {
            hi: 0xbf6b400000200000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0x80000003,
        6599,
    ), // precision_subeven_double_round_negative
    (
        F128 {
            hi: 0xbf6b400000200000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0x80000003,
        6607,
    ), // precision_subeven_double_round_negative
    (
        F128 {
            hi: 0xbf6b400000200000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0x80000002,
        6597,
    ), // precision_subeven_double_round_negative
    (
        F128 {
            hi: 0xbf6b400000200000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0x80000002,
        6597,
    ), // precision_subeven_double_round_negative
    (
        F128 {
            hi: 0x0000850652c5b639,
            lo: 0x5a561e5d84c7e766,
        },
        Ext80Rounding::NearestEven,
        0x00000000,
        6605,
    ), // raw_field0_sample0_positive
    (
        F128 {
            hi: 0x0000850652c5b639,
            lo: 0x5a561e5d84c7e766,
        },
        Ext80Rounding::Down,
        0x00000000,
        6597,
    ), // raw_field0_sample0_positive
    (
        F128 {
            hi: 0x0000850652c5b639,
            lo: 0x5a561e5d84c7e766,
        },
        Ext80Rounding::Up,
        0x00000001,
        6607,
    ), // raw_field0_sample0_positive
    (
        F128 {
            hi: 0x0000850652c5b639,
            lo: 0x5a561e5d84c7e766,
        },
        Ext80Rounding::ToZero,
        0x00000000,
        6597,
    ), // raw_field0_sample0_positive
    (
        F128 {
            hi: 0x8000850652c5b639,
            lo: 0x5a561e5d84c7e766,
        },
        Ext80Rounding::NearestEven,
        0x80000000,
        6605,
    ), // raw_field0_sample0_negative
    (
        F128 {
            hi: 0x8000850652c5b639,
            lo: 0x5a561e5d84c7e766,
        },
        Ext80Rounding::Down,
        0x80000001,
        6607,
    ), // raw_field0_sample0_negative
    (
        F128 {
            hi: 0x8000850652c5b639,
            lo: 0x5a561e5d84c7e766,
        },
        Ext80Rounding::Up,
        0x80000000,
        6597,
    ), // raw_field0_sample0_negative
    (
        F128 {
            hi: 0x8000850652c5b639,
            lo: 0x5a561e5d84c7e766,
        },
        Ext80Rounding::ToZero,
        0x80000000,
        6597,
    ), // raw_field0_sample0_negative
    (
        F128 {
            hi: 0x000024610805a0d0,
            lo: 0xcbf35105a6392210,
        },
        Ext80Rounding::NearestEven,
        0x00000000,
        6597,
    ), // raw_field0_sample1_positive
    (
        F128 {
            hi: 0x000024610805a0d0,
            lo: 0xcbf35105a6392210,
        },
        Ext80Rounding::Down,
        0x00000000,
        6597,
    ), // raw_field0_sample1_positive
    (
        F128 {
            hi: 0x000024610805a0d0,
            lo: 0xcbf35105a6392210,
        },
        Ext80Rounding::Up,
        0x00000001,
        6607,
    ), // raw_field0_sample1_positive
    (
        F128 {
            hi: 0x000024610805a0d0,
            lo: 0xcbf35105a6392210,
        },
        Ext80Rounding::ToZero,
        0x00000000,
        6597,
    ), // raw_field0_sample1_positive
    (
        F128 {
            hi: 0x800024610805a0d0,
            lo: 0xcbf35105a6392210,
        },
        Ext80Rounding::NearestEven,
        0x80000000,
        6597,
    ), // raw_field0_sample1_negative
    (
        F128 {
            hi: 0x800024610805a0d0,
            lo: 0xcbf35105a6392210,
        },
        Ext80Rounding::Down,
        0x80000001,
        6607,
    ), // raw_field0_sample1_negative
    (
        F128 {
            hi: 0x800024610805a0d0,
            lo: 0xcbf35105a6392210,
        },
        Ext80Rounding::Up,
        0x80000000,
        6597,
    ), // raw_field0_sample1_negative
    (
        F128 {
            hi: 0x800024610805a0d0,
            lo: 0xcbf35105a6392210,
        },
        Ext80Rounding::ToZero,
        0x80000000,
        6597,
    ), // raw_field0_sample1_negative
    (
        F128 {
            hi: 0x000137d264f5ed00,
            lo: 0x7587d39b91b9acec,
        },
        Ext80Rounding::NearestEven,
        0x00000000,
        6597,
    ), // raw_field1_sample0_positive
    (
        F128 {
            hi: 0x000137d264f5ed00,
            lo: 0x7587d39b91b9acec,
        },
        Ext80Rounding::Down,
        0x00000000,
        6597,
    ), // raw_field1_sample0_positive
    (
        F128 {
            hi: 0x000137d264f5ed00,
            lo: 0x7587d39b91b9acec,
        },
        Ext80Rounding::Up,
        0x00000001,
        6607,
    ), // raw_field1_sample0_positive
    (
        F128 {
            hi: 0x000137d264f5ed00,
            lo: 0x7587d39b91b9acec,
        },
        Ext80Rounding::ToZero,
        0x00000000,
        6597,
    ), // raw_field1_sample0_positive
    (
        F128 {
            hi: 0x800137d264f5ed00,
            lo: 0x7587d39b91b9acec,
        },
        Ext80Rounding::NearestEven,
        0x80000000,
        6597,
    ), // raw_field1_sample0_negative
    (
        F128 {
            hi: 0x800137d264f5ed00,
            lo: 0x7587d39b91b9acec,
        },
        Ext80Rounding::Down,
        0x80000001,
        6607,
    ), // raw_field1_sample0_negative
    (
        F128 {
            hi: 0x800137d264f5ed00,
            lo: 0x7587d39b91b9acec,
        },
        Ext80Rounding::Up,
        0x80000000,
        6597,
    ), // raw_field1_sample0_negative
    (
        F128 {
            hi: 0x800137d264f5ed00,
            lo: 0x7587d39b91b9acec,
        },
        Ext80Rounding::ToZero,
        0x80000000,
        6597,
    ), // raw_field1_sample0_negative
    (
        F128 {
            hi: 0x0001da59b2979429,
            lo: 0x9ecc373f4e46108c,
        },
        Ext80Rounding::NearestEven,
        0x00000000,
        6597,
    ), // raw_field1_sample1_positive
    (
        F128 {
            hi: 0x0001da59b2979429,
            lo: 0x9ecc373f4e46108c,
        },
        Ext80Rounding::Down,
        0x00000000,
        6597,
    ), // raw_field1_sample1_positive
    (
        F128 {
            hi: 0x0001da59b2979429,
            lo: 0x9ecc373f4e46108c,
        },
        Ext80Rounding::Up,
        0x00000001,
        6607,
    ), // raw_field1_sample1_positive
    (
        F128 {
            hi: 0x0001da59b2979429,
            lo: 0x9ecc373f4e46108c,
        },
        Ext80Rounding::ToZero,
        0x00000000,
        6597,
    ), // raw_field1_sample1_positive
    (
        F128 {
            hi: 0x8001da59b2979429,
            lo: 0x9ecc373f4e46108c,
        },
        Ext80Rounding::NearestEven,
        0x80000000,
        6597,
    ), // raw_field1_sample1_negative
    (
        F128 {
            hi: 0x8001da59b2979429,
            lo: 0x9ecc373f4e46108c,
        },
        Ext80Rounding::Down,
        0x80000001,
        6607,
    ), // raw_field1_sample1_negative
    (
        F128 {
            hi: 0x8001da59b2979429,
            lo: 0x9ecc373f4e46108c,
        },
        Ext80Rounding::Up,
        0x80000000,
        6597,
    ), // raw_field1_sample1_negative
    (
        F128 {
            hi: 0x8001da59b2979429,
            lo: 0x9ecc373f4e46108c,
        },
        Ext80Rounding::ToZero,
        0x80000000,
        6597,
    ), // raw_field1_sample1_negative
    (
        F128 {
            hi: 0x00021be573cdf059,
            lo: 0xfdb203fe98a8a336,
        },
        Ext80Rounding::NearestEven,
        0x00000000,
        6605,
    ), // raw_field2_sample0_positive
    (
        F128 {
            hi: 0x00021be573cdf059,
            lo: 0xfdb203fe98a8a336,
        },
        Ext80Rounding::Down,
        0x00000000,
        6597,
    ), // raw_field2_sample0_positive
    (
        F128 {
            hi: 0x00021be573cdf059,
            lo: 0xfdb203fe98a8a336,
        },
        Ext80Rounding::Up,
        0x00000001,
        6607,
    ), // raw_field2_sample0_positive
    (
        F128 {
            hi: 0x00021be573cdf059,
            lo: 0xfdb203fe98a8a336,
        },
        Ext80Rounding::ToZero,
        0x00000000,
        6597,
    ), // raw_field2_sample0_positive
    (
        F128 {
            hi: 0x80021be573cdf059,
            lo: 0xfdb203fe98a8a336,
        },
        Ext80Rounding::NearestEven,
        0x80000000,
        6605,
    ), // raw_field2_sample0_negative
    (
        F128 {
            hi: 0x80021be573cdf059,
            lo: 0xfdb203fe98a8a336,
        },
        Ext80Rounding::Down,
        0x80000001,
        6607,
    ), // raw_field2_sample0_negative
    (
        F128 {
            hi: 0x80021be573cdf059,
            lo: 0xfdb203fe98a8a336,
        },
        Ext80Rounding::Up,
        0x80000000,
        6597,
    ), // raw_field2_sample0_negative
    (
        F128 {
            hi: 0x80021be573cdf059,
            lo: 0xfdb203fe98a8a336,
        },
        Ext80Rounding::ToZero,
        0x80000000,
        6597,
    ), // raw_field2_sample0_negative
    (
        F128 {
            hi: 0x0002754a0f963b67,
            lo: 0x4c54741e982928c8,
        },
        Ext80Rounding::NearestEven,
        0x00000000,
        6605,
    ), // raw_field2_sample1_positive
    (
        F128 {
            hi: 0x0002754a0f963b67,
            lo: 0x4c54741e982928c8,
        },
        Ext80Rounding::Down,
        0x00000000,
        6597,
    ), // raw_field2_sample1_positive
    (
        F128 {
            hi: 0x0002754a0f963b67,
            lo: 0x4c54741e982928c8,
        },
        Ext80Rounding::Up,
        0x00000001,
        6607,
    ), // raw_field2_sample1_positive
    (
        F128 {
            hi: 0x0002754a0f963b67,
            lo: 0x4c54741e982928c8,
        },
        Ext80Rounding::ToZero,
        0x00000000,
        6597,
    ), // raw_field2_sample1_positive
    (
        F128 {
            hi: 0x8002754a0f963b67,
            lo: 0x4c54741e982928c8,
        },
        Ext80Rounding::NearestEven,
        0x80000000,
        6605,
    ), // raw_field2_sample1_negative
    (
        F128 {
            hi: 0x8002754a0f963b67,
            lo: 0x4c54741e982928c8,
        },
        Ext80Rounding::Down,
        0x80000001,
        6607,
    ), // raw_field2_sample1_negative
    (
        F128 {
            hi: 0x8002754a0f963b67,
            lo: 0x4c54741e982928c8,
        },
        Ext80Rounding::Up,
        0x80000000,
        6597,
    ), // raw_field2_sample1_negative
    (
        F128 {
            hi: 0x8002754a0f963b67,
            lo: 0x4c54741e982928c8,
        },
        Ext80Rounding::ToZero,
        0x80000000,
        6597,
    ), // raw_field2_sample1_negative
    (
        F128 {
            hi: 0x3e80482c22fcd2b2,
            lo: 0xe23de046ccd73dbd,
        },
        Ext80Rounding::NearestEven,
        0x00000000,
        6597,
    ), // raw_field16000_sample0_positive
    (
        F128 {
            hi: 0x3e80482c22fcd2b2,
            lo: 0xe23de046ccd73dbd,
        },
        Ext80Rounding::Down,
        0x00000000,
        6597,
    ), // raw_field16000_sample0_positive
    (
        F128 {
            hi: 0x3e80482c22fcd2b2,
            lo: 0xe23de046ccd73dbd,
        },
        Ext80Rounding::Up,
        0x00000001,
        6607,
    ), // raw_field16000_sample0_positive
    (
        F128 {
            hi: 0x3e80482c22fcd2b2,
            lo: 0xe23de046ccd73dbd,
        },
        Ext80Rounding::ToZero,
        0x00000000,
        6597,
    ), // raw_field16000_sample0_positive
    (
        F128 {
            hi: 0xbe80482c22fcd2b2,
            lo: 0xe23de046ccd73dbd,
        },
        Ext80Rounding::NearestEven,
        0x80000000,
        6597,
    ), // raw_field16000_sample0_negative
    (
        F128 {
            hi: 0xbe80482c22fcd2b2,
            lo: 0xe23de046ccd73dbd,
        },
        Ext80Rounding::Down,
        0x80000001,
        6607,
    ), // raw_field16000_sample0_negative
    (
        F128 {
            hi: 0xbe80482c22fcd2b2,
            lo: 0xe23de046ccd73dbd,
        },
        Ext80Rounding::Up,
        0x80000000,
        6597,
    ), // raw_field16000_sample0_negative
    (
        F128 {
            hi: 0xbe80482c22fcd2b2,
            lo: 0xe23de046ccd73dbd,
        },
        Ext80Rounding::ToZero,
        0x80000000,
        6597,
    ), // raw_field16000_sample0_negative
    (
        F128 {
            hi: 0x3e80d80583968e80,
            lo: 0x40312903624e8cab,
        },
        Ext80Rounding::NearestEven,
        0x00000000,
        6605,
    ), // raw_field16000_sample1_positive
    (
        F128 {
            hi: 0x3e80d80583968e80,
            lo: 0x40312903624e8cab,
        },
        Ext80Rounding::Down,
        0x00000000,
        6597,
    ), // raw_field16000_sample1_positive
    (
        F128 {
            hi: 0x3e80d80583968e80,
            lo: 0x40312903624e8cab,
        },
        Ext80Rounding::Up,
        0x00000001,
        6607,
    ), // raw_field16000_sample1_positive
    (
        F128 {
            hi: 0x3e80d80583968e80,
            lo: 0x40312903624e8cab,
        },
        Ext80Rounding::ToZero,
        0x00000000,
        6597,
    ), // raw_field16000_sample1_positive
    (
        F128 {
            hi: 0xbe80d80583968e80,
            lo: 0x40312903624e8cab,
        },
        Ext80Rounding::NearestEven,
        0x80000000,
        6605,
    ), // raw_field16000_sample1_negative
    (
        F128 {
            hi: 0xbe80d80583968e80,
            lo: 0x40312903624e8cab,
        },
        Ext80Rounding::Down,
        0x80000001,
        6607,
    ), // raw_field16000_sample1_negative
    (
        F128 {
            hi: 0xbe80d80583968e80,
            lo: 0x40312903624e8cab,
        },
        Ext80Rounding::Up,
        0x80000000,
        6597,
    ), // raw_field16000_sample1_negative
    (
        F128 {
            hi: 0xbe80d80583968e80,
            lo: 0x40312903624e8cab,
        },
        Ext80Rounding::ToZero,
        0x80000000,
        6597,
    ), // raw_field16000_sample1_negative
    (
        F128 {
            hi: 0x3f6848c4a854dfc7,
            lo: 0x497b8f393bf83501,
        },
        Ext80Rounding::NearestEven,
        0x00000000,
        6597,
    ), // raw_field16232_sample0_positive
    (
        F128 {
            hi: 0x3f6848c4a854dfc7,
            lo: 0x497b8f393bf83501,
        },
        Ext80Rounding::Down,
        0x00000000,
        6597,
    ), // raw_field16232_sample0_positive
    (
        F128 {
            hi: 0x3f6848c4a854dfc7,
            lo: 0x497b8f393bf83501,
        },
        Ext80Rounding::Up,
        0x00000001,
        6607,
    ), // raw_field16232_sample0_positive
    (
        F128 {
            hi: 0x3f6848c4a854dfc7,
            lo: 0x497b8f393bf83501,
        },
        Ext80Rounding::ToZero,
        0x00000000,
        6597,
    ), // raw_field16232_sample0_positive
    (
        F128 {
            hi: 0xbf6848c4a854dfc7,
            lo: 0x497b8f393bf83501,
        },
        Ext80Rounding::NearestEven,
        0x80000000,
        6597,
    ), // raw_field16232_sample0_negative
    (
        F128 {
            hi: 0xbf6848c4a854dfc7,
            lo: 0x497b8f393bf83501,
        },
        Ext80Rounding::Down,
        0x80000001,
        6607,
    ), // raw_field16232_sample0_negative
    (
        F128 {
            hi: 0xbf6848c4a854dfc7,
            lo: 0x497b8f393bf83501,
        },
        Ext80Rounding::Up,
        0x80000000,
        6597,
    ), // raw_field16232_sample0_negative
    (
        F128 {
            hi: 0xbf6848c4a854dfc7,
            lo: 0x497b8f393bf83501,
        },
        Ext80Rounding::ToZero,
        0x80000000,
        6597,
    ), // raw_field16232_sample0_negative
    (
        F128 {
            hi: 0x3f686489f413cba2,
            lo: 0x1878b3cc47387678,
        },
        Ext80Rounding::NearestEven,
        0x00000000,
        6597,
    ), // raw_field16232_sample1_positive
    (
        F128 {
            hi: 0x3f686489f413cba2,
            lo: 0x1878b3cc47387678,
        },
        Ext80Rounding::Down,
        0x00000000,
        6597,
    ), // raw_field16232_sample1_positive
    (
        F128 {
            hi: 0x3f686489f413cba2,
            lo: 0x1878b3cc47387678,
        },
        Ext80Rounding::Up,
        0x00000001,
        6607,
    ), // raw_field16232_sample1_positive
    (
        F128 {
            hi: 0x3f686489f413cba2,
            lo: 0x1878b3cc47387678,
        },
        Ext80Rounding::ToZero,
        0x00000000,
        6597,
    ), // raw_field16232_sample1_positive
    (
        F128 {
            hi: 0xbf686489f413cba2,
            lo: 0x1878b3cc47387678,
        },
        Ext80Rounding::NearestEven,
        0x80000000,
        6597,
    ), // raw_field16232_sample1_negative
    (
        F128 {
            hi: 0xbf686489f413cba2,
            lo: 0x1878b3cc47387678,
        },
        Ext80Rounding::Down,
        0x80000001,
        6607,
    ), // raw_field16232_sample1_negative
    (
        F128 {
            hi: 0xbf686489f413cba2,
            lo: 0x1878b3cc47387678,
        },
        Ext80Rounding::Up,
        0x80000000,
        6597,
    ), // raw_field16232_sample1_negative
    (
        F128 {
            hi: 0xbf686489f413cba2,
            lo: 0x1878b3cc47387678,
        },
        Ext80Rounding::ToZero,
        0x80000000,
        6597,
    ), // raw_field16232_sample1_negative
    (
        F128 {
            hi: 0x3f695f41a19dc356,
            lo: 0x3293f6726ce0ffc8,
        },
        Ext80Rounding::NearestEven,
        0x00000001,
        6607,
    ), // raw_field16233_sample0_positive
    (
        F128 {
            hi: 0x3f695f41a19dc356,
            lo: 0x3293f6726ce0ffc8,
        },
        Ext80Rounding::Down,
        0x00000000,
        6597,
    ), // raw_field16233_sample0_positive
    (
        F128 {
            hi: 0x3f695f41a19dc356,
            lo: 0x3293f6726ce0ffc8,
        },
        Ext80Rounding::Up,
        0x00000001,
        6607,
    ), // raw_field16233_sample0_positive
    (
        F128 {
            hi: 0x3f695f41a19dc356,
            lo: 0x3293f6726ce0ffc8,
        },
        Ext80Rounding::ToZero,
        0x00000000,
        6597,
    ), // raw_field16233_sample0_positive
    (
        F128 {
            hi: 0xbf695f41a19dc356,
            lo: 0x3293f6726ce0ffc8,
        },
        Ext80Rounding::NearestEven,
        0x80000001,
        6607,
    ), // raw_field16233_sample0_negative
    (
        F128 {
            hi: 0xbf695f41a19dc356,
            lo: 0x3293f6726ce0ffc8,
        },
        Ext80Rounding::Down,
        0x80000001,
        6607,
    ), // raw_field16233_sample0_negative
    (
        F128 {
            hi: 0xbf695f41a19dc356,
            lo: 0x3293f6726ce0ffc8,
        },
        Ext80Rounding::Up,
        0x80000000,
        6597,
    ), // raw_field16233_sample0_negative
    (
        F128 {
            hi: 0xbf695f41a19dc356,
            lo: 0x3293f6726ce0ffc8,
        },
        Ext80Rounding::ToZero,
        0x80000000,
        6597,
    ), // raw_field16233_sample0_negative
    (
        F128 {
            hi: 0x3f69afde5d04361b,
            lo: 0x5987fee2050c8937,
        },
        Ext80Rounding::NearestEven,
        0x00000001,
        6607,
    ), // raw_field16233_sample1_positive
    (
        F128 {
            hi: 0x3f69afde5d04361b,
            lo: 0x5987fee2050c8937,
        },
        Ext80Rounding::Down,
        0x00000000,
        6597,
    ), // raw_field16233_sample1_positive
    (
        F128 {
            hi: 0x3f69afde5d04361b,
            lo: 0x5987fee2050c8937,
        },
        Ext80Rounding::Up,
        0x00000001,
        6607,
    ), // raw_field16233_sample1_positive
    (
        F128 {
            hi: 0x3f69afde5d04361b,
            lo: 0x5987fee2050c8937,
        },
        Ext80Rounding::ToZero,
        0x00000000,
        6597,
    ), // raw_field16233_sample1_positive
    (
        F128 {
            hi: 0xbf69afde5d04361b,
            lo: 0x5987fee2050c8937,
        },
        Ext80Rounding::NearestEven,
        0x80000001,
        6607,
    ), // raw_field16233_sample1_negative
    (
        F128 {
            hi: 0xbf69afde5d04361b,
            lo: 0x5987fee2050c8937,
        },
        Ext80Rounding::Down,
        0x80000001,
        6607,
    ), // raw_field16233_sample1_negative
    (
        F128 {
            hi: 0xbf69afde5d04361b,
            lo: 0x5987fee2050c8937,
        },
        Ext80Rounding::Up,
        0x80000000,
        6597,
    ), // raw_field16233_sample1_negative
    (
        F128 {
            hi: 0xbf69afde5d04361b,
            lo: 0x5987fee2050c8937,
        },
        Ext80Rounding::ToZero,
        0x80000000,
        6597,
    ), // raw_field16233_sample1_negative
    (
        F128 {
            hi: 0x3f80847aeafc7cf4,
            lo: 0xfefd4999fbb20c2d,
        },
        Ext80Rounding::NearestEven,
        0x00611ebb,
        6599,
    ), // raw_field16256_sample0_positive
    (
        F128 {
            hi: 0x3f80847aeafc7cf4,
            lo: 0xfefd4999fbb20c2d,
        },
        Ext80Rounding::Down,
        0x00611eba,
        6597,
    ), // raw_field16256_sample0_positive
    (
        F128 {
            hi: 0x3f80847aeafc7cf4,
            lo: 0xfefd4999fbb20c2d,
        },
        Ext80Rounding::Up,
        0x00611ebb,
        6607,
    ), // raw_field16256_sample0_positive
    (
        F128 {
            hi: 0x3f80847aeafc7cf4,
            lo: 0xfefd4999fbb20c2d,
        },
        Ext80Rounding::ToZero,
        0x00611eba,
        6597,
    ), // raw_field16256_sample0_positive
    (
        F128 {
            hi: 0xbf80847aeafc7cf4,
            lo: 0xfefd4999fbb20c2d,
        },
        Ext80Rounding::NearestEven,
        0x80611ebb,
        6599,
    ), // raw_field16256_sample0_negative
    (
        F128 {
            hi: 0xbf80847aeafc7cf4,
            lo: 0xfefd4999fbb20c2d,
        },
        Ext80Rounding::Down,
        0x80611ebb,
        6607,
    ), // raw_field16256_sample0_negative
    (
        F128 {
            hi: 0xbf80847aeafc7cf4,
            lo: 0xfefd4999fbb20c2d,
        },
        Ext80Rounding::Up,
        0x80611eba,
        6597,
    ), // raw_field16256_sample0_negative
    (
        F128 {
            hi: 0xbf80847aeafc7cf4,
            lo: 0xfefd4999fbb20c2d,
        },
        Ext80Rounding::ToZero,
        0x80611eba,
        6597,
    ), // raw_field16256_sample0_negative
    (
        F128 {
            hi: 0x3f807b16fa91c7d3,
            lo: 0x7e0a6be983e3a085,
        },
        Ext80Rounding::NearestEven,
        0x005ec5bf,
        6599,
    ), // raw_field16256_sample1_positive
    (
        F128 {
            hi: 0x3f807b16fa91c7d3,
            lo: 0x7e0a6be983e3a085,
        },
        Ext80Rounding::Down,
        0x005ec5be,
        6597,
    ), // raw_field16256_sample1_positive
    (
        F128 {
            hi: 0x3f807b16fa91c7d3,
            lo: 0x7e0a6be983e3a085,
        },
        Ext80Rounding::Up,
        0x005ec5bf,
        6607,
    ), // raw_field16256_sample1_positive
    (
        F128 {
            hi: 0x3f807b16fa91c7d3,
            lo: 0x7e0a6be983e3a085,
        },
        Ext80Rounding::ToZero,
        0x005ec5be,
        6597,
    ), // raw_field16256_sample1_positive
    (
        F128 {
            hi: 0xbf807b16fa91c7d3,
            lo: 0x7e0a6be983e3a085,
        },
        Ext80Rounding::NearestEven,
        0x805ec5bf,
        6599,
    ), // raw_field16256_sample1_negative
    (
        F128 {
            hi: 0xbf807b16fa91c7d3,
            lo: 0x7e0a6be983e3a085,
        },
        Ext80Rounding::Down,
        0x805ec5bf,
        6607,
    ), // raw_field16256_sample1_negative
    (
        F128 {
            hi: 0xbf807b16fa91c7d3,
            lo: 0x7e0a6be983e3a085,
        },
        Ext80Rounding::Up,
        0x805ec5be,
        6597,
    ), // raw_field16256_sample1_negative
    (
        F128 {
            hi: 0xbf807b16fa91c7d3,
            lo: 0x7e0a6be983e3a085,
        },
        Ext80Rounding::ToZero,
        0x805ec5be,
        6597,
    ), // raw_field16256_sample1_negative
    (
        F128 {
            hi: 0x3f81489de5bd63ec,
            lo: 0x14a38603da4376d2,
        },
        Ext80Rounding::NearestEven,
        0x00a44ef3,
        15,
    ), // raw_field16257_sample0_positive
    (
        F128 {
            hi: 0x3f81489de5bd63ec,
            lo: 0x14a38603da4376d2,
        },
        Ext80Rounding::Down,
        0x00a44ef2,
        5,
    ), // raw_field16257_sample0_positive
    (
        F128 {
            hi: 0x3f81489de5bd63ec,
            lo: 0x14a38603da4376d2,
        },
        Ext80Rounding::Up,
        0x00a44ef3,
        15,
    ), // raw_field16257_sample0_positive
    (
        F128 {
            hi: 0x3f81489de5bd63ec,
            lo: 0x14a38603da4376d2,
        },
        Ext80Rounding::ToZero,
        0x00a44ef2,
        5,
    ), // raw_field16257_sample0_positive
    (
        F128 {
            hi: 0xbf81489de5bd63ec,
            lo: 0x14a38603da4376d2,
        },
        Ext80Rounding::NearestEven,
        0x80a44ef3,
        15,
    ), // raw_field16257_sample0_negative
    (
        F128 {
            hi: 0xbf81489de5bd63ec,
            lo: 0x14a38603da4376d2,
        },
        Ext80Rounding::Down,
        0x80a44ef3,
        15,
    ), // raw_field16257_sample0_negative
    (
        F128 {
            hi: 0xbf81489de5bd63ec,
            lo: 0x14a38603da4376d2,
        },
        Ext80Rounding::Up,
        0x80a44ef2,
        5,
    ), // raw_field16257_sample0_negative
    (
        F128 {
            hi: 0xbf81489de5bd63ec,
            lo: 0x14a38603da4376d2,
        },
        Ext80Rounding::ToZero,
        0x80a44ef2,
        5,
    ), // raw_field16257_sample0_negative
    (
        F128 {
            hi: 0x3f81e08f6f8733a3,
            lo: 0x5436ce44175c2e13,
        },
        Ext80Rounding::NearestEven,
        0x00f047b8,
        15,
    ), // raw_field16257_sample1_positive
    (
        F128 {
            hi: 0x3f81e08f6f8733a3,
            lo: 0x5436ce44175c2e13,
        },
        Ext80Rounding::Down,
        0x00f047b7,
        5,
    ), // raw_field16257_sample1_positive
    (
        F128 {
            hi: 0x3f81e08f6f8733a3,
            lo: 0x5436ce44175c2e13,
        },
        Ext80Rounding::Up,
        0x00f047b8,
        15,
    ), // raw_field16257_sample1_positive
    (
        F128 {
            hi: 0x3f81e08f6f8733a3,
            lo: 0x5436ce44175c2e13,
        },
        Ext80Rounding::ToZero,
        0x00f047b7,
        5,
    ), // raw_field16257_sample1_positive
    (
        F128 {
            hi: 0xbf81e08f6f8733a3,
            lo: 0x5436ce44175c2e13,
        },
        Ext80Rounding::NearestEven,
        0x80f047b8,
        15,
    ), // raw_field16257_sample1_negative
    (
        F128 {
            hi: 0xbf81e08f6f8733a3,
            lo: 0x5436ce44175c2e13,
        },
        Ext80Rounding::Down,
        0x80f047b8,
        15,
    ), // raw_field16257_sample1_negative
    (
        F128 {
            hi: 0xbf81e08f6f8733a3,
            lo: 0x5436ce44175c2e13,
        },
        Ext80Rounding::Up,
        0x80f047b7,
        5,
    ), // raw_field16257_sample1_negative
    (
        F128 {
            hi: 0xbf81e08f6f8733a3,
            lo: 0x5436ce44175c2e13,
        },
        Ext80Rounding::ToZero,
        0x80f047b7,
        5,
    ), // raw_field16257_sample1_negative
    (
        F128 {
            hi: 0x3f82ba1e95be97cc,
            lo: 0x53f47753731f0c3f,
        },
        Ext80Rounding::NearestEven,
        0x015d0f4b,
        15,
    ), // raw_field16258_sample0_positive
    (
        F128 {
            hi: 0x3f82ba1e95be97cc,
            lo: 0x53f47753731f0c3f,
        },
        Ext80Rounding::Down,
        0x015d0f4a,
        5,
    ), // raw_field16258_sample0_positive
    (
        F128 {
            hi: 0x3f82ba1e95be97cc,
            lo: 0x53f47753731f0c3f,
        },
        Ext80Rounding::Up,
        0x015d0f4b,
        15,
    ), // raw_field16258_sample0_positive
    (
        F128 {
            hi: 0x3f82ba1e95be97cc,
            lo: 0x53f47753731f0c3f,
        },
        Ext80Rounding::ToZero,
        0x015d0f4a,
        5,
    ), // raw_field16258_sample0_positive
    (
        F128 {
            hi: 0xbf82ba1e95be97cc,
            lo: 0x53f47753731f0c3f,
        },
        Ext80Rounding::NearestEven,
        0x815d0f4b,
        15,
    ), // raw_field16258_sample0_negative
    (
        F128 {
            hi: 0xbf82ba1e95be97cc,
            lo: 0x53f47753731f0c3f,
        },
        Ext80Rounding::Down,
        0x815d0f4b,
        15,
    ), // raw_field16258_sample0_negative
    (
        F128 {
            hi: 0xbf82ba1e95be97cc,
            lo: 0x53f47753731f0c3f,
        },
        Ext80Rounding::Up,
        0x815d0f4a,
        5,
    ), // raw_field16258_sample0_negative
    (
        F128 {
            hi: 0xbf82ba1e95be97cc,
            lo: 0x53f47753731f0c3f,
        },
        Ext80Rounding::ToZero,
        0x815d0f4a,
        5,
    ), // raw_field16258_sample0_negative
    (
        F128 {
            hi: 0x3f82370fe86b989e,
            lo: 0x2cdb06c6ebb2c6cb,
        },
        Ext80Rounding::NearestEven,
        0x011b87f4,
        5,
    ), // raw_field16258_sample1_positive
    (
        F128 {
            hi: 0x3f82370fe86b989e,
            lo: 0x2cdb06c6ebb2c6cb,
        },
        Ext80Rounding::Down,
        0x011b87f4,
        5,
    ), // raw_field16258_sample1_positive
    (
        F128 {
            hi: 0x3f82370fe86b989e,
            lo: 0x2cdb06c6ebb2c6cb,
        },
        Ext80Rounding::Up,
        0x011b87f5,
        15,
    ), // raw_field16258_sample1_positive
    (
        F128 {
            hi: 0x3f82370fe86b989e,
            lo: 0x2cdb06c6ebb2c6cb,
        },
        Ext80Rounding::ToZero,
        0x011b87f4,
        5,
    ), // raw_field16258_sample1_positive
    (
        F128 {
            hi: 0xbf82370fe86b989e,
            lo: 0x2cdb06c6ebb2c6cb,
        },
        Ext80Rounding::NearestEven,
        0x811b87f4,
        5,
    ), // raw_field16258_sample1_negative
    (
        F128 {
            hi: 0xbf82370fe86b989e,
            lo: 0x2cdb06c6ebb2c6cb,
        },
        Ext80Rounding::Down,
        0x811b87f5,
        15,
    ), // raw_field16258_sample1_negative
    (
        F128 {
            hi: 0xbf82370fe86b989e,
            lo: 0x2cdb06c6ebb2c6cb,
        },
        Ext80Rounding::Up,
        0x811b87f4,
        5,
    ), // raw_field16258_sample1_negative
    (
        F128 {
            hi: 0xbf82370fe86b989e,
            lo: 0x2cdb06c6ebb2c6cb,
        },
        Ext80Rounding::ToZero,
        0x811b87f4,
        5,
    ), // raw_field16258_sample1_negative
    (
        F128 {
            hi: 0x3ffe4717bfad54a6,
            lo: 0xfcd1809bea295ffa,
        },
        Ext80Rounding::NearestEven,
        0x3f238be0,
        15,
    ), // raw_field16382_sample0_positive
    (
        F128 {
            hi: 0x3ffe4717bfad54a6,
            lo: 0xfcd1809bea295ffa,
        },
        Ext80Rounding::Down,
        0x3f238bdf,
        5,
    ), // raw_field16382_sample0_positive
    (
        F128 {
            hi: 0x3ffe4717bfad54a6,
            lo: 0xfcd1809bea295ffa,
        },
        Ext80Rounding::Up,
        0x3f238be0,
        15,
    ), // raw_field16382_sample0_positive
    (
        F128 {
            hi: 0x3ffe4717bfad54a6,
            lo: 0xfcd1809bea295ffa,
        },
        Ext80Rounding::ToZero,
        0x3f238bdf,
        5,
    ), // raw_field16382_sample0_positive
    (
        F128 {
            hi: 0xbffe4717bfad54a6,
            lo: 0xfcd1809bea295ffa,
        },
        Ext80Rounding::NearestEven,
        0xbf238be0,
        15,
    ), // raw_field16382_sample0_negative
    (
        F128 {
            hi: 0xbffe4717bfad54a6,
            lo: 0xfcd1809bea295ffa,
        },
        Ext80Rounding::Down,
        0xbf238be0,
        15,
    ), // raw_field16382_sample0_negative
    (
        F128 {
            hi: 0xbffe4717bfad54a6,
            lo: 0xfcd1809bea295ffa,
        },
        Ext80Rounding::Up,
        0xbf238bdf,
        5,
    ), // raw_field16382_sample0_negative
    (
        F128 {
            hi: 0xbffe4717bfad54a6,
            lo: 0xfcd1809bea295ffa,
        },
        Ext80Rounding::ToZero,
        0xbf238bdf,
        5,
    ), // raw_field16382_sample0_negative
    (
        F128 {
            hi: 0x3ffeb2739567299e,
            lo: 0xbebabfd32b8840c7,
        },
        Ext80Rounding::NearestEven,
        0x3f5939cb,
        15,
    ), // raw_field16382_sample1_positive
    (
        F128 {
            hi: 0x3ffeb2739567299e,
            lo: 0xbebabfd32b8840c7,
        },
        Ext80Rounding::Down,
        0x3f5939ca,
        5,
    ), // raw_field16382_sample1_positive
    (
        F128 {
            hi: 0x3ffeb2739567299e,
            lo: 0xbebabfd32b8840c7,
        },
        Ext80Rounding::Up,
        0x3f5939cb,
        15,
    ), // raw_field16382_sample1_positive
    (
        F128 {
            hi: 0x3ffeb2739567299e,
            lo: 0xbebabfd32b8840c7,
        },
        Ext80Rounding::ToZero,
        0x3f5939ca,
        5,
    ), // raw_field16382_sample1_positive
    (
        F128 {
            hi: 0xbffeb2739567299e,
            lo: 0xbebabfd32b8840c7,
        },
        Ext80Rounding::NearestEven,
        0xbf5939cb,
        15,
    ), // raw_field16382_sample1_negative
    (
        F128 {
            hi: 0xbffeb2739567299e,
            lo: 0xbebabfd32b8840c7,
        },
        Ext80Rounding::Down,
        0xbf5939cb,
        15,
    ), // raw_field16382_sample1_negative
    (
        F128 {
            hi: 0xbffeb2739567299e,
            lo: 0xbebabfd32b8840c7,
        },
        Ext80Rounding::Up,
        0xbf5939ca,
        5,
    ), // raw_field16382_sample1_negative
    (
        F128 {
            hi: 0xbffeb2739567299e,
            lo: 0xbebabfd32b8840c7,
        },
        Ext80Rounding::ToZero,
        0xbf5939ca,
        5,
    ), // raw_field16382_sample1_negative
    (
        F128 {
            hi: 0x3fff50166607ccd8,
            lo: 0x238ccf4096e8db94,
        },
        Ext80Rounding::NearestEven,
        0x3fa80b33,
        5,
    ), // raw_field16383_sample0_positive
    (
        F128 {
            hi: 0x3fff50166607ccd8,
            lo: 0x238ccf4096e8db94,
        },
        Ext80Rounding::Down,
        0x3fa80b33,
        5,
    ), // raw_field16383_sample0_positive
    (
        F128 {
            hi: 0x3fff50166607ccd8,
            lo: 0x238ccf4096e8db94,
        },
        Ext80Rounding::Up,
        0x3fa80b34,
        15,
    ), // raw_field16383_sample0_positive
    (
        F128 {
            hi: 0x3fff50166607ccd8,
            lo: 0x238ccf4096e8db94,
        },
        Ext80Rounding::ToZero,
        0x3fa80b33,
        5,
    ), // raw_field16383_sample0_positive
    (
        F128 {
            hi: 0xbfff50166607ccd8,
            lo: 0x238ccf4096e8db94,
        },
        Ext80Rounding::NearestEven,
        0xbfa80b33,
        5,
    ), // raw_field16383_sample0_negative
    (
        F128 {
            hi: 0xbfff50166607ccd8,
            lo: 0x238ccf4096e8db94,
        },
        Ext80Rounding::Down,
        0xbfa80b34,
        15,
    ), // raw_field16383_sample0_negative
    (
        F128 {
            hi: 0xbfff50166607ccd8,
            lo: 0x238ccf4096e8db94,
        },
        Ext80Rounding::Up,
        0xbfa80b33,
        5,
    ), // raw_field16383_sample0_negative
    (
        F128 {
            hi: 0xbfff50166607ccd8,
            lo: 0x238ccf4096e8db94,
        },
        Ext80Rounding::ToZero,
        0xbfa80b33,
        5,
    ), // raw_field16383_sample0_negative
    (
        F128 {
            hi: 0x3fffe4599571ed79,
            lo: 0x92f90bc78efba752,
        },
        Ext80Rounding::NearestEven,
        0x3ff22ccb,
        15,
    ), // raw_field16383_sample1_positive
    (
        F128 {
            hi: 0x3fffe4599571ed79,
            lo: 0x92f90bc78efba752,
        },
        Ext80Rounding::Down,
        0x3ff22cca,
        5,
    ), // raw_field16383_sample1_positive
    (
        F128 {
            hi: 0x3fffe4599571ed79,
            lo: 0x92f90bc78efba752,
        },
        Ext80Rounding::Up,
        0x3ff22ccb,
        15,
    ), // raw_field16383_sample1_positive
    (
        F128 {
            hi: 0x3fffe4599571ed79,
            lo: 0x92f90bc78efba752,
        },
        Ext80Rounding::ToZero,
        0x3ff22cca,
        5,
    ), // raw_field16383_sample1_positive
    (
        F128 {
            hi: 0xbfffe4599571ed79,
            lo: 0x92f90bc78efba752,
        },
        Ext80Rounding::NearestEven,
        0xbff22ccb,
        15,
    ), // raw_field16383_sample1_negative
    (
        F128 {
            hi: 0xbfffe4599571ed79,
            lo: 0x92f90bc78efba752,
        },
        Ext80Rounding::Down,
        0xbff22ccb,
        15,
    ), // raw_field16383_sample1_negative
    (
        F128 {
            hi: 0xbfffe4599571ed79,
            lo: 0x92f90bc78efba752,
        },
        Ext80Rounding::Up,
        0xbff22cca,
        5,
    ), // raw_field16383_sample1_negative
    (
        F128 {
            hi: 0xbfffe4599571ed79,
            lo: 0x92f90bc78efba752,
        },
        Ext80Rounding::ToZero,
        0xbff22cca,
        5,
    ), // raw_field16383_sample1_negative
    (
        F128 {
            hi: 0x4000425f2a2f363f,
            lo: 0x7fba2ce647ea27c3,
        },
        Ext80Rounding::NearestEven,
        0x40212f95,
        5,
    ), // raw_field16384_sample0_positive
    (
        F128 {
            hi: 0x4000425f2a2f363f,
            lo: 0x7fba2ce647ea27c3,
        },
        Ext80Rounding::Down,
        0x40212f95,
        5,
    ), // raw_field16384_sample0_positive
    (
        F128 {
            hi: 0x4000425f2a2f363f,
            lo: 0x7fba2ce647ea27c3,
        },
        Ext80Rounding::Up,
        0x40212f96,
        15,
    ), // raw_field16384_sample0_positive
    (
        F128 {
            hi: 0x4000425f2a2f363f,
            lo: 0x7fba2ce647ea27c3,
        },
        Ext80Rounding::ToZero,
        0x40212f95,
        5,
    ), // raw_field16384_sample0_positive
    (
        F128 {
            hi: 0xc000425f2a2f363f,
            lo: 0x7fba2ce647ea27c3,
        },
        Ext80Rounding::NearestEven,
        0xc0212f95,
        5,
    ), // raw_field16384_sample0_negative
    (
        F128 {
            hi: 0xc000425f2a2f363f,
            lo: 0x7fba2ce647ea27c3,
        },
        Ext80Rounding::Down,
        0xc0212f96,
        15,
    ), // raw_field16384_sample0_negative
    (
        F128 {
            hi: 0xc000425f2a2f363f,
            lo: 0x7fba2ce647ea27c3,
        },
        Ext80Rounding::Up,
        0xc0212f95,
        5,
    ), // raw_field16384_sample0_negative
    (
        F128 {
            hi: 0xc000425f2a2f363f,
            lo: 0x7fba2ce647ea27c3,
        },
        Ext80Rounding::ToZero,
        0xc0212f95,
        5,
    ), // raw_field16384_sample0_negative
    (
        F128 {
            hi: 0x40000f43113791af,
            lo: 0x7979209e7c743740,
        },
        Ext80Rounding::NearestEven,
        0x4007a189,
        15,
    ), // raw_field16384_sample1_positive
    (
        F128 {
            hi: 0x40000f43113791af,
            lo: 0x7979209e7c743740,
        },
        Ext80Rounding::Down,
        0x4007a188,
        5,
    ), // raw_field16384_sample1_positive
    (
        F128 {
            hi: 0x40000f43113791af,
            lo: 0x7979209e7c743740,
        },
        Ext80Rounding::Up,
        0x4007a189,
        15,
    ), // raw_field16384_sample1_positive
    (
        F128 {
            hi: 0x40000f43113791af,
            lo: 0x7979209e7c743740,
        },
        Ext80Rounding::ToZero,
        0x4007a188,
        5,
    ), // raw_field16384_sample1_positive
    (
        F128 {
            hi: 0xc0000f43113791af,
            lo: 0x7979209e7c743740,
        },
        Ext80Rounding::NearestEven,
        0xc007a189,
        15,
    ), // raw_field16384_sample1_negative
    (
        F128 {
            hi: 0xc0000f43113791af,
            lo: 0x7979209e7c743740,
        },
        Ext80Rounding::Down,
        0xc007a189,
        15,
    ), // raw_field16384_sample1_negative
    (
        F128 {
            hi: 0xc0000f43113791af,
            lo: 0x7979209e7c743740,
        },
        Ext80Rounding::Up,
        0xc007a188,
        5,
    ), // raw_field16384_sample1_negative
    (
        F128 {
            hi: 0xc0000f43113791af,
            lo: 0x7979209e7c743740,
        },
        Ext80Rounding::ToZero,
        0xc007a188,
        5,
    ), // raw_field16384_sample1_negative
    (
        F128 {
            hi: 0x407dc9b1b375d245,
            lo: 0xd6d120a3c213e6c1,
        },
        Ext80Rounding::NearestEven,
        0x7ee4d8da,
        15,
    ), // raw_field16509_sample0_positive
    (
        F128 {
            hi: 0x407dc9b1b375d245,
            lo: 0xd6d120a3c213e6c1,
        },
        Ext80Rounding::Down,
        0x7ee4d8d9,
        5,
    ), // raw_field16509_sample0_positive
    (
        F128 {
            hi: 0x407dc9b1b375d245,
            lo: 0xd6d120a3c213e6c1,
        },
        Ext80Rounding::Up,
        0x7ee4d8da,
        15,
    ), // raw_field16509_sample0_positive
    (
        F128 {
            hi: 0x407dc9b1b375d245,
            lo: 0xd6d120a3c213e6c1,
        },
        Ext80Rounding::ToZero,
        0x7ee4d8d9,
        5,
    ), // raw_field16509_sample0_positive
    (
        F128 {
            hi: 0xc07dc9b1b375d245,
            lo: 0xd6d120a3c213e6c1,
        },
        Ext80Rounding::NearestEven,
        0xfee4d8da,
        15,
    ), // raw_field16509_sample0_negative
    (
        F128 {
            hi: 0xc07dc9b1b375d245,
            lo: 0xd6d120a3c213e6c1,
        },
        Ext80Rounding::Down,
        0xfee4d8da,
        15,
    ), // raw_field16509_sample0_negative
    (
        F128 {
            hi: 0xc07dc9b1b375d245,
            lo: 0xd6d120a3c213e6c1,
        },
        Ext80Rounding::Up,
        0xfee4d8d9,
        5,
    ), // raw_field16509_sample0_negative
    (
        F128 {
            hi: 0xc07dc9b1b375d245,
            lo: 0xd6d120a3c213e6c1,
        },
        Ext80Rounding::ToZero,
        0xfee4d8d9,
        5,
    ), // raw_field16509_sample0_negative
    (
        F128 {
            hi: 0x407df4ca786a7143,
            lo: 0xb0625af42f6fc7ea,
        },
        Ext80Rounding::NearestEven,
        0x7efa653c,
        5,
    ), // raw_field16509_sample1_positive
    (
        F128 {
            hi: 0x407df4ca786a7143,
            lo: 0xb0625af42f6fc7ea,
        },
        Ext80Rounding::Down,
        0x7efa653c,
        5,
    ), // raw_field16509_sample1_positive
    (
        F128 {
            hi: 0x407df4ca786a7143,
            lo: 0xb0625af42f6fc7ea,
        },
        Ext80Rounding::Up,
        0x7efa653d,
        15,
    ), // raw_field16509_sample1_positive
    (
        F128 {
            hi: 0x407df4ca786a7143,
            lo: 0xb0625af42f6fc7ea,
        },
        Ext80Rounding::ToZero,
        0x7efa653c,
        5,
    ), // raw_field16509_sample1_positive
    (
        F128 {
            hi: 0xc07df4ca786a7143,
            lo: 0xb0625af42f6fc7ea,
        },
        Ext80Rounding::NearestEven,
        0xfefa653c,
        5,
    ), // raw_field16509_sample1_negative
    (
        F128 {
            hi: 0xc07df4ca786a7143,
            lo: 0xb0625af42f6fc7ea,
        },
        Ext80Rounding::Down,
        0xfefa653d,
        15,
    ), // raw_field16509_sample1_negative
    (
        F128 {
            hi: 0xc07df4ca786a7143,
            lo: 0xb0625af42f6fc7ea,
        },
        Ext80Rounding::Up,
        0xfefa653c,
        5,
    ), // raw_field16509_sample1_negative
    (
        F128 {
            hi: 0xc07df4ca786a7143,
            lo: 0xb0625af42f6fc7ea,
        },
        Ext80Rounding::ToZero,
        0xfefa653c,
        5,
    ), // raw_field16509_sample1_negative
    (
        F128 {
            hi: 0x407e80f044ada743,
            lo: 0x8b5e82973113e84c,
        },
        Ext80Rounding::NearestEven,
        0x7f407822,
        5,
    ), // raw_field16510_sample0_positive
    (
        F128 {
            hi: 0x407e80f044ada743,
            lo: 0x8b5e82973113e84c,
        },
        Ext80Rounding::Down,
        0x7f407822,
        5,
    ), // raw_field16510_sample0_positive
    (
        F128 {
            hi: 0x407e80f044ada743,
            lo: 0x8b5e82973113e84c,
        },
        Ext80Rounding::Up,
        0x7f407823,
        15,
    ), // raw_field16510_sample0_positive
    (
        F128 {
            hi: 0x407e80f044ada743,
            lo: 0x8b5e82973113e84c,
        },
        Ext80Rounding::ToZero,
        0x7f407822,
        5,
    ), // raw_field16510_sample0_positive
    (
        F128 {
            hi: 0xc07e80f044ada743,
            lo: 0x8b5e82973113e84c,
        },
        Ext80Rounding::NearestEven,
        0xff407822,
        5,
    ), // raw_field16510_sample0_negative
    (
        F128 {
            hi: 0xc07e80f044ada743,
            lo: 0x8b5e82973113e84c,
        },
        Ext80Rounding::Down,
        0xff407823,
        15,
    ), // raw_field16510_sample0_negative
    (
        F128 {
            hi: 0xc07e80f044ada743,
            lo: 0x8b5e82973113e84c,
        },
        Ext80Rounding::Up,
        0xff407822,
        5,
    ), // raw_field16510_sample0_negative
    (
        F128 {
            hi: 0xc07e80f044ada743,
            lo: 0x8b5e82973113e84c,
        },
        Ext80Rounding::ToZero,
        0xff407822,
        5,
    ), // raw_field16510_sample0_negative
    (
        F128 {
            hi: 0x407ee862e8d34511,
            lo: 0x514680e00f737606,
        },
        Ext80Rounding::NearestEven,
        0x7f743174,
        5,
    ), // raw_field16510_sample1_positive
    (
        F128 {
            hi: 0x407ee862e8d34511,
            lo: 0x514680e00f737606,
        },
        Ext80Rounding::Down,
        0x7f743174,
        5,
    ), // raw_field16510_sample1_positive
    (
        F128 {
            hi: 0x407ee862e8d34511,
            lo: 0x514680e00f737606,
        },
        Ext80Rounding::Up,
        0x7f743175,
        15,
    ), // raw_field16510_sample1_positive
    (
        F128 {
            hi: 0x407ee862e8d34511,
            lo: 0x514680e00f737606,
        },
        Ext80Rounding::ToZero,
        0x7f743174,
        5,
    ), // raw_field16510_sample1_positive
    (
        F128 {
            hi: 0xc07ee862e8d34511,
            lo: 0x514680e00f737606,
        },
        Ext80Rounding::NearestEven,
        0xff743174,
        5,
    ), // raw_field16510_sample1_negative
    (
        F128 {
            hi: 0xc07ee862e8d34511,
            lo: 0x514680e00f737606,
        },
        Ext80Rounding::Down,
        0xff743175,
        15,
    ), // raw_field16510_sample1_negative
    (
        F128 {
            hi: 0xc07ee862e8d34511,
            lo: 0x514680e00f737606,
        },
        Ext80Rounding::Up,
        0xff743174,
        5,
    ), // raw_field16510_sample1_negative
    (
        F128 {
            hi: 0xc07ee862e8d34511,
            lo: 0x514680e00f737606,
        },
        Ext80Rounding::ToZero,
        0xff743174,
        5,
    ), // raw_field16510_sample1_negative
    (
        F128 {
            hi: 0x407fb9ed2df056a2,
            lo: 0x1ef1ddf526e44a3a,
        },
        Ext80Rounding::NearestEven,
        0x7f800000,
        63,
    ), // raw_field16511_sample0_positive
    (
        F128 {
            hi: 0x407fb9ed2df056a2,
            lo: 0x1ef1ddf526e44a3a,
        },
        Ext80Rounding::Down,
        0x7f7fffff,
        21,
    ), // raw_field16511_sample0_positive
    (
        F128 {
            hi: 0x407fb9ed2df056a2,
            lo: 0x1ef1ddf526e44a3a,
        },
        Ext80Rounding::Up,
        0x7f800000,
        63,
    ), // raw_field16511_sample0_positive
    (
        F128 {
            hi: 0x407fb9ed2df056a2,
            lo: 0x1ef1ddf526e44a3a,
        },
        Ext80Rounding::ToZero,
        0x7f7fffff,
        21,
    ), // raw_field16511_sample0_positive
    (
        F128 {
            hi: 0xc07fb9ed2df056a2,
            lo: 0x1ef1ddf526e44a3a,
        },
        Ext80Rounding::NearestEven,
        0xff800000,
        63,
    ), // raw_field16511_sample0_negative
    (
        F128 {
            hi: 0xc07fb9ed2df056a2,
            lo: 0x1ef1ddf526e44a3a,
        },
        Ext80Rounding::Down,
        0xff800000,
        63,
    ), // raw_field16511_sample0_negative
    (
        F128 {
            hi: 0xc07fb9ed2df056a2,
            lo: 0x1ef1ddf526e44a3a,
        },
        Ext80Rounding::Up,
        0xff7fffff,
        21,
    ), // raw_field16511_sample0_negative
    (
        F128 {
            hi: 0xc07fb9ed2df056a2,
            lo: 0x1ef1ddf526e44a3a,
        },
        Ext80Rounding::ToZero,
        0xff7fffff,
        21,
    ), // raw_field16511_sample0_negative
    (
        F128 {
            hi: 0x407f0e3e8502c382,
            lo: 0x49df0f3e00a4f626,
        },
        Ext80Rounding::NearestEven,
        0x7f800000,
        63,
    ), // raw_field16511_sample1_positive
    (
        F128 {
            hi: 0x407f0e3e8502c382,
            lo: 0x49df0f3e00a4f626,
        },
        Ext80Rounding::Down,
        0x7f7fffff,
        21,
    ), // raw_field16511_sample1_positive
    (
        F128 {
            hi: 0x407f0e3e8502c382,
            lo: 0x49df0f3e00a4f626,
        },
        Ext80Rounding::Up,
        0x7f800000,
        63,
    ), // raw_field16511_sample1_positive
    (
        F128 {
            hi: 0x407f0e3e8502c382,
            lo: 0x49df0f3e00a4f626,
        },
        Ext80Rounding::ToZero,
        0x7f7fffff,
        21,
    ), // raw_field16511_sample1_positive
    (
        F128 {
            hi: 0xc07f0e3e8502c382,
            lo: 0x49df0f3e00a4f626,
        },
        Ext80Rounding::NearestEven,
        0xff800000,
        63,
    ), // raw_field16511_sample1_negative
    (
        F128 {
            hi: 0xc07f0e3e8502c382,
            lo: 0x49df0f3e00a4f626,
        },
        Ext80Rounding::Down,
        0xff800000,
        63,
    ), // raw_field16511_sample1_negative
    (
        F128 {
            hi: 0xc07f0e3e8502c382,
            lo: 0x49df0f3e00a4f626,
        },
        Ext80Rounding::Up,
        0xff7fffff,
        21,
    ), // raw_field16511_sample1_negative
    (
        F128 {
            hi: 0xc07f0e3e8502c382,
            lo: 0x49df0f3e00a4f626,
        },
        Ext80Rounding::ToZero,
        0xff7fffff,
        21,
    ), // raw_field16511_sample1_negative
    (
        F128 {
            hi: 0x7ffe04c017e4da4c,
            lo: 0xed4aeb090f30a002,
        },
        Ext80Rounding::NearestEven,
        0x7f800000,
        63,
    ), // raw_field32766_sample0_positive
    (
        F128 {
            hi: 0x7ffe04c017e4da4c,
            lo: 0xed4aeb090f30a002,
        },
        Ext80Rounding::Down,
        0x7f7fffff,
        21,
    ), // raw_field32766_sample0_positive
    (
        F128 {
            hi: 0x7ffe04c017e4da4c,
            lo: 0xed4aeb090f30a002,
        },
        Ext80Rounding::Up,
        0x7f800000,
        63,
    ), // raw_field32766_sample0_positive
    (
        F128 {
            hi: 0x7ffe04c017e4da4c,
            lo: 0xed4aeb090f30a002,
        },
        Ext80Rounding::ToZero,
        0x7f7fffff,
        21,
    ), // raw_field32766_sample0_positive
    (
        F128 {
            hi: 0xfffe04c017e4da4c,
            lo: 0xed4aeb090f30a002,
        },
        Ext80Rounding::NearestEven,
        0xff800000,
        63,
    ), // raw_field32766_sample0_negative
    (
        F128 {
            hi: 0xfffe04c017e4da4c,
            lo: 0xed4aeb090f30a002,
        },
        Ext80Rounding::Down,
        0xff800000,
        63,
    ), // raw_field32766_sample0_negative
    (
        F128 {
            hi: 0xfffe04c017e4da4c,
            lo: 0xed4aeb090f30a002,
        },
        Ext80Rounding::Up,
        0xff7fffff,
        21,
    ), // raw_field32766_sample0_negative
    (
        F128 {
            hi: 0xfffe04c017e4da4c,
            lo: 0xed4aeb090f30a002,
        },
        Ext80Rounding::ToZero,
        0xff7fffff,
        21,
    ), // raw_field32766_sample0_negative
    (
        F128 {
            hi: 0x7ffe66aa55b88662,
            lo: 0xe38d1a80f41f8dcd,
        },
        Ext80Rounding::NearestEven,
        0x7f800000,
        63,
    ), // raw_field32766_sample1_positive
    (
        F128 {
            hi: 0x7ffe66aa55b88662,
            lo: 0xe38d1a80f41f8dcd,
        },
        Ext80Rounding::Down,
        0x7f7fffff,
        21,
    ), // raw_field32766_sample1_positive
    (
        F128 {
            hi: 0x7ffe66aa55b88662,
            lo: 0xe38d1a80f41f8dcd,
        },
        Ext80Rounding::Up,
        0x7f800000,
        63,
    ), // raw_field32766_sample1_positive
    (
        F128 {
            hi: 0x7ffe66aa55b88662,
            lo: 0xe38d1a80f41f8dcd,
        },
        Ext80Rounding::ToZero,
        0x7f7fffff,
        21,
    ), // raw_field32766_sample1_positive
    (
        F128 {
            hi: 0xfffe66aa55b88662,
            lo: 0xe38d1a80f41f8dcd,
        },
        Ext80Rounding::NearestEven,
        0xff800000,
        63,
    ), // raw_field32766_sample1_negative
    (
        F128 {
            hi: 0xfffe66aa55b88662,
            lo: 0xe38d1a80f41f8dcd,
        },
        Ext80Rounding::Down,
        0xff800000,
        63,
    ), // raw_field32766_sample1_negative
    (
        F128 {
            hi: 0xfffe66aa55b88662,
            lo: 0xe38d1a80f41f8dcd,
        },
        Ext80Rounding::Up,
        0xff7fffff,
        21,
    ), // raw_field32766_sample1_negative
    (
        F128 {
            hi: 0xfffe66aa55b88662,
            lo: 0xe38d1a80f41f8dcd,
        },
        Ext80Rounding::ToZero,
        0xff7fffff,
        21,
    ), // raw_field32766_sample1_negative
];
