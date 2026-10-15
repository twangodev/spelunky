// Exact Fraction oracle literals. Source-only; tests UNRUN.
const CASES: &[(F128, Ext80Rounding, u64, u16)] = &[
    (
        F128 {
            hi: 0x0000000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0x0000000000000000,
        0,
    ), // zero_positive
    (
        F128 {
            hi: 0x0000000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0x0000000000000000,
        0,
    ), // zero_positive
    (
        F128 {
            hi: 0x0000000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0x0000000000000000,
        0,
    ), // zero_positive
    (
        F128 {
            hi: 0x0000000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0x0000000000000000,
        0,
    ), // zero_positive
    (
        F128 {
            hi: 0x8000000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0x8000000000000000,
        0,
    ), // zero_negative
    (
        F128 {
            hi: 0x8000000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0x8000000000000000,
        0,
    ), // zero_negative
    (
        F128 {
            hi: 0x8000000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0x8000000000000000,
        0,
    ), // zero_negative
    (
        F128 {
            hi: 0x8000000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0x8000000000000000,
        0,
    ), // zero_negative
    (
        F128 {
            hi: 0x3fff000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0x3ff0000000000000,
        0,
    ), // one_positive
    (
        F128 {
            hi: 0x3fff000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0x3ff0000000000000,
        0,
    ), // one_positive
    (
        F128 {
            hi: 0x3fff000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0x3ff0000000000000,
        0,
    ), // one_positive
    (
        F128 {
            hi: 0x3fff000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0x3ff0000000000000,
        0,
    ), // one_positive
    (
        F128 {
            hi: 0xbfff000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0xbff0000000000000,
        0,
    ), // one_negative
    (
        F128 {
            hi: 0xbfff000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0xbff0000000000000,
        0,
    ), // one_negative
    (
        F128 {
            hi: 0xbfff000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0xbff0000000000000,
        0,
    ), // one_negative
    (
        F128 {
            hi: 0xbfff000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0xbff0000000000000,
        0,
    ), // one_negative
    (
        F128 {
            hi: 0x3fff800000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0x3ff8000000000000,
        0,
    ), // one_half_positive
    (
        F128 {
            hi: 0x3fff800000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0x3ff8000000000000,
        0,
    ), // one_half_positive
    (
        F128 {
            hi: 0x3fff800000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0x3ff8000000000000,
        0,
    ), // one_half_positive
    (
        F128 {
            hi: 0x3fff800000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0x3ff8000000000000,
        0,
    ), // one_half_positive
    (
        F128 {
            hi: 0xbfff800000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0xbff8000000000000,
        0,
    ), // one_half_negative
    (
        F128 {
            hi: 0xbfff800000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0xbff8000000000000,
        0,
    ), // one_half_negative
    (
        F128 {
            hi: 0xbfff800000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0xbff8000000000000,
        0,
    ), // one_half_negative
    (
        F128 {
            hi: 0xbfff800000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0xbff8000000000000,
        0,
    ), // one_half_negative
    (
        F128 {
            hi: 0x3bcd000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0x0000000000000001,
        4544,
    ), // min_subnormal64_positive
    (
        F128 {
            hi: 0x3bcd000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0x0000000000000001,
        4544,
    ), // min_subnormal64_positive
    (
        F128 {
            hi: 0x3bcd000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0x0000000000000001,
        4544,
    ), // min_subnormal64_positive
    (
        F128 {
            hi: 0x3bcd000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0x0000000000000001,
        4544,
    ), // min_subnormal64_positive
    (
        F128 {
            hi: 0xbbcd000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0x8000000000000001,
        4544,
    ), // min_subnormal64_negative
    (
        F128 {
            hi: 0xbbcd000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0x8000000000000001,
        4544,
    ), // min_subnormal64_negative
    (
        F128 {
            hi: 0xbbcd000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0x8000000000000001,
        4544,
    ), // min_subnormal64_negative
    (
        F128 {
            hi: 0xbbcd000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0x8000000000000001,
        4544,
    ), // min_subnormal64_negative
    (
        F128 {
            hi: 0x3c00ffffffffffff,
            lo: 0xe000000000000000,
        },
        Ext80Rounding::NearestEven,
        0x000fffffffffffff,
        4544,
    ), // max_subnormal64_positive
    (
        F128 {
            hi: 0x3c00ffffffffffff,
            lo: 0xe000000000000000,
        },
        Ext80Rounding::Down,
        0x000fffffffffffff,
        4544,
    ), // max_subnormal64_positive
    (
        F128 {
            hi: 0x3c00ffffffffffff,
            lo: 0xe000000000000000,
        },
        Ext80Rounding::Up,
        0x000fffffffffffff,
        4544,
    ), // max_subnormal64_positive
    (
        F128 {
            hi: 0x3c00ffffffffffff,
            lo: 0xe000000000000000,
        },
        Ext80Rounding::ToZero,
        0x000fffffffffffff,
        4544,
    ), // max_subnormal64_positive
    (
        F128 {
            hi: 0xbc00ffffffffffff,
            lo: 0xe000000000000000,
        },
        Ext80Rounding::NearestEven,
        0x800fffffffffffff,
        4544,
    ), // max_subnormal64_negative
    (
        F128 {
            hi: 0xbc00ffffffffffff,
            lo: 0xe000000000000000,
        },
        Ext80Rounding::Down,
        0x800fffffffffffff,
        4544,
    ), // max_subnormal64_negative
    (
        F128 {
            hi: 0xbc00ffffffffffff,
            lo: 0xe000000000000000,
        },
        Ext80Rounding::Up,
        0x800fffffffffffff,
        4544,
    ), // max_subnormal64_negative
    (
        F128 {
            hi: 0xbc00ffffffffffff,
            lo: 0xe000000000000000,
        },
        Ext80Rounding::ToZero,
        0x800fffffffffffff,
        4544,
    ), // max_subnormal64_negative
    (
        F128 {
            hi: 0x3c01000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0x0010000000000000,
        0,
    ), // min_normal64_positive
    (
        F128 {
            hi: 0x3c01000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0x0010000000000000,
        0,
    ), // min_normal64_positive
    (
        F128 {
            hi: 0x3c01000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0x0010000000000000,
        0,
    ), // min_normal64_positive
    (
        F128 {
            hi: 0x3c01000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0x0010000000000000,
        0,
    ), // min_normal64_positive
    (
        F128 {
            hi: 0xbc01000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0x8010000000000000,
        0,
    ), // min_normal64_negative
    (
        F128 {
            hi: 0xbc01000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0x8010000000000000,
        0,
    ), // min_normal64_negative
    (
        F128 {
            hi: 0xbc01000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0x8010000000000000,
        0,
    ), // min_normal64_negative
    (
        F128 {
            hi: 0xbc01000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0x8010000000000000,
        0,
    ), // min_normal64_negative
    (
        F128 {
            hi: 0x43feffffffffffff,
            lo: 0xf000000000000000,
        },
        Ext80Rounding::NearestEven,
        0x7fefffffffffffff,
        0,
    ), // max_finite64_positive
    (
        F128 {
            hi: 0x43feffffffffffff,
            lo: 0xf000000000000000,
        },
        Ext80Rounding::Down,
        0x7fefffffffffffff,
        0,
    ), // max_finite64_positive
    (
        F128 {
            hi: 0x43feffffffffffff,
            lo: 0xf000000000000000,
        },
        Ext80Rounding::Up,
        0x7fefffffffffffff,
        0,
    ), // max_finite64_positive
    (
        F128 {
            hi: 0x43feffffffffffff,
            lo: 0xf000000000000000,
        },
        Ext80Rounding::ToZero,
        0x7fefffffffffffff,
        0,
    ), // max_finite64_positive
    (
        F128 {
            hi: 0xc3feffffffffffff,
            lo: 0xf000000000000000,
        },
        Ext80Rounding::NearestEven,
        0xffefffffffffffff,
        0,
    ), // max_finite64_negative
    (
        F128 {
            hi: 0xc3feffffffffffff,
            lo: 0xf000000000000000,
        },
        Ext80Rounding::Down,
        0xffefffffffffffff,
        0,
    ), // max_finite64_negative
    (
        F128 {
            hi: 0xc3feffffffffffff,
            lo: 0xf000000000000000,
        },
        Ext80Rounding::Up,
        0xffefffffffffffff,
        0,
    ), // max_finite64_negative
    (
        F128 {
            hi: 0xc3feffffffffffff,
            lo: 0xf000000000000000,
        },
        Ext80Rounding::ToZero,
        0xffefffffffffffff,
        0,
    ), // max_finite64_negative
    (
        F128 {
            hi: 0x43ff000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0x7ff0000000000000,
        51,
    ), // two_to1024_positive
    (
        F128 {
            hi: 0x43ff000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0x7fefffffffffffff,
        17,
    ), // two_to1024_positive
    (
        F128 {
            hi: 0x43ff000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0x7ff0000000000000,
        51,
    ), // two_to1024_positive
    (
        F128 {
            hi: 0x43ff000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0x7fefffffffffffff,
        17,
    ), // two_to1024_positive
    (
        F128 {
            hi: 0xc3ff000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0xfff0000000000000,
        51,
    ), // two_to1024_negative
    (
        F128 {
            hi: 0xc3ff000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0xfff0000000000000,
        51,
    ), // two_to1024_negative
    (
        F128 {
            hi: 0xc3ff000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0xffefffffffffffff,
        17,
    ), // two_to1024_negative
    (
        F128 {
            hi: 0xc3ff000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0xffefffffffffffff,
        17,
    ), // two_to1024_negative
    (
        F128 {
            hi: 0x4400000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0x7ff0000000000000,
        51,
    ), // two_to1025_positive
    (
        F128 {
            hi: 0x4400000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0x7fefffffffffffff,
        17,
    ), // two_to1025_positive
    (
        F128 {
            hi: 0x4400000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0x7ff0000000000000,
        51,
    ), // two_to1025_positive
    (
        F128 {
            hi: 0x4400000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0x7fefffffffffffff,
        17,
    ), // two_to1025_positive
    (
        F128 {
            hi: 0xc400000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0xfff0000000000000,
        51,
    ), // two_to1025_negative
    (
        F128 {
            hi: 0xc400000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0xfff0000000000000,
        51,
    ), // two_to1025_negative
    (
        F128 {
            hi: 0xc400000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0xffefffffffffffff,
        17,
    ), // two_to1025_negative
    (
        F128 {
            hi: 0xc400000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0xffefffffffffffff,
        17,
    ), // two_to1025_negative
    (
        F128 {
            hi: 0x0000000000000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::NearestEven,
        0x0000000000000000,
        6593,
    ), // min_subnormal128_positive
    (
        F128 {
            hi: 0x0000000000000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::Down,
        0x0000000000000000,
        6593,
    ), // min_subnormal128_positive
    (
        F128 {
            hi: 0x0000000000000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::Up,
        0x0000000000000001,
        6595,
    ), // min_subnormal128_positive
    (
        F128 {
            hi: 0x0000000000000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::ToZero,
        0x0000000000000000,
        6593,
    ), // min_subnormal128_positive
    (
        F128 {
            hi: 0x8000000000000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::NearestEven,
        0x8000000000000000,
        6593,
    ), // min_subnormal128_negative
    (
        F128 {
            hi: 0x8000000000000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::Down,
        0x8000000000000001,
        6595,
    ), // min_subnormal128_negative
    (
        F128 {
            hi: 0x8000000000000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::Up,
        0x8000000000000000,
        6593,
    ), // min_subnormal128_negative
    (
        F128 {
            hi: 0x8000000000000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::ToZero,
        0x8000000000000000,
        6593,
    ), // min_subnormal128_negative
    (
        F128 {
            hi: 0x0001000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0x0000000000000000,
        6593,
    ), // min_normal128_positive
    (
        F128 {
            hi: 0x0001000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0x0000000000000000,
        6593,
    ), // min_normal128_positive
    (
        F128 {
            hi: 0x0001000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0x0000000000000001,
        6595,
    ), // min_normal128_positive
    (
        F128 {
            hi: 0x0001000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0x0000000000000000,
        6593,
    ), // min_normal128_positive
    (
        F128 {
            hi: 0x8001000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0x8000000000000000,
        6593,
    ), // min_normal128_negative
    (
        F128 {
            hi: 0x8001000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0x8000000000000001,
        6595,
    ), // min_normal128_negative
    (
        F128 {
            hi: 0x8001000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0x8000000000000000,
        6593,
    ), // min_normal128_negative
    (
        F128 {
            hi: 0x8001000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0x8000000000000000,
        6593,
    ), // min_normal128_negative
    (
        F128 {
            hi: 0x0000ffffffffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::NearestEven,
        0x0000000000000000,
        6605,
    ), // max_subnormal128_positive
    (
        F128 {
            hi: 0x0000ffffffffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::Down,
        0x0000000000000000,
        6597,
    ), // max_subnormal128_positive
    (
        F128 {
            hi: 0x0000ffffffffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::Up,
        0x0000000000000001,
        6607,
    ), // max_subnormal128_positive
    (
        F128 {
            hi: 0x0000ffffffffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::ToZero,
        0x0000000000000000,
        6597,
    ), // max_subnormal128_positive
    (
        F128 {
            hi: 0x8000ffffffffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::NearestEven,
        0x8000000000000000,
        6605,
    ), // max_subnormal128_negative
    (
        F128 {
            hi: 0x8000ffffffffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::Down,
        0x8000000000000001,
        6607,
    ), // max_subnormal128_negative
    (
        F128 {
            hi: 0x8000ffffffffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::Up,
        0x8000000000000000,
        6597,
    ), // max_subnormal128_negative
    (
        F128 {
            hi: 0x8000ffffffffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::ToZero,
        0x8000000000000000,
        6597,
    ), // max_subnormal128_negative
    (
        F128 {
            hi: 0x7ffeffffffffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::NearestEven,
        0x7ff0000000000000,
        63,
    ), // max_finite128_positive
    (
        F128 {
            hi: 0x7ffeffffffffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::Down,
        0x7fefffffffffffff,
        21,
    ), // max_finite128_positive
    (
        F128 {
            hi: 0x7ffeffffffffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::Up,
        0x7ff0000000000000,
        63,
    ), // max_finite128_positive
    (
        F128 {
            hi: 0x7ffeffffffffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::ToZero,
        0x7fefffffffffffff,
        21,
    ), // max_finite128_positive
    (
        F128 {
            hi: 0xfffeffffffffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::NearestEven,
        0xfff0000000000000,
        63,
    ), // max_finite128_negative
    (
        F128 {
            hi: 0xfffeffffffffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::Down,
        0xfff0000000000000,
        63,
    ), // max_finite128_negative
    (
        F128 {
            hi: 0xfffeffffffffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::Up,
        0xffefffffffffffff,
        21,
    ), // max_finite128_negative
    (
        F128 {
            hi: 0xfffeffffffffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::ToZero,
        0xffefffffffffffff,
        21,
    ), // max_finite128_negative
    (
        F128 {
            hi: 0x3fff000000000000,
            lo: 0x07ffffffffffffff,
        },
        Ext80Rounding::NearestEven,
        0x3ff0000000000000,
        5,
    ), // normal_even_tie_-1_positive
    (
        F128 {
            hi: 0x3fff000000000000,
            lo: 0x07ffffffffffffff,
        },
        Ext80Rounding::Down,
        0x3ff0000000000000,
        5,
    ), // normal_even_tie_-1_positive
    (
        F128 {
            hi: 0x3fff000000000000,
            lo: 0x07ffffffffffffff,
        },
        Ext80Rounding::Up,
        0x3ff0000000000001,
        15,
    ), // normal_even_tie_-1_positive
    (
        F128 {
            hi: 0x3fff000000000000,
            lo: 0x07ffffffffffffff,
        },
        Ext80Rounding::ToZero,
        0x3ff0000000000000,
        5,
    ), // normal_even_tie_-1_positive
    (
        F128 {
            hi: 0xbfff000000000000,
            lo: 0x07ffffffffffffff,
        },
        Ext80Rounding::NearestEven,
        0xbff0000000000000,
        5,
    ), // normal_even_tie_-1_negative
    (
        F128 {
            hi: 0xbfff000000000000,
            lo: 0x07ffffffffffffff,
        },
        Ext80Rounding::Down,
        0xbff0000000000001,
        15,
    ), // normal_even_tie_-1_negative
    (
        F128 {
            hi: 0xbfff000000000000,
            lo: 0x07ffffffffffffff,
        },
        Ext80Rounding::Up,
        0xbff0000000000000,
        5,
    ), // normal_even_tie_-1_negative
    (
        F128 {
            hi: 0xbfff000000000000,
            lo: 0x07ffffffffffffff,
        },
        Ext80Rounding::ToZero,
        0xbff0000000000000,
        5,
    ), // normal_even_tie_-1_negative
    (
        F128 {
            hi: 0x3fff000000000000,
            lo: 0x0800000000000000,
        },
        Ext80Rounding::NearestEven,
        0x3ff0000000000000,
        5,
    ), // normal_even_tie_+0_positive
    (
        F128 {
            hi: 0x3fff000000000000,
            lo: 0x0800000000000000,
        },
        Ext80Rounding::Down,
        0x3ff0000000000000,
        5,
    ), // normal_even_tie_+0_positive
    (
        F128 {
            hi: 0x3fff000000000000,
            lo: 0x0800000000000000,
        },
        Ext80Rounding::Up,
        0x3ff0000000000001,
        15,
    ), // normal_even_tie_+0_positive
    (
        F128 {
            hi: 0x3fff000000000000,
            lo: 0x0800000000000000,
        },
        Ext80Rounding::ToZero,
        0x3ff0000000000000,
        5,
    ), // normal_even_tie_+0_positive
    (
        F128 {
            hi: 0xbfff000000000000,
            lo: 0x0800000000000000,
        },
        Ext80Rounding::NearestEven,
        0xbff0000000000000,
        5,
    ), // normal_even_tie_+0_negative
    (
        F128 {
            hi: 0xbfff000000000000,
            lo: 0x0800000000000000,
        },
        Ext80Rounding::Down,
        0xbff0000000000001,
        15,
    ), // normal_even_tie_+0_negative
    (
        F128 {
            hi: 0xbfff000000000000,
            lo: 0x0800000000000000,
        },
        Ext80Rounding::Up,
        0xbff0000000000000,
        5,
    ), // normal_even_tie_+0_negative
    (
        F128 {
            hi: 0xbfff000000000000,
            lo: 0x0800000000000000,
        },
        Ext80Rounding::ToZero,
        0xbff0000000000000,
        5,
    ), // normal_even_tie_+0_negative
    (
        F128 {
            hi: 0x3fff000000000000,
            lo: 0x0800000000000001,
        },
        Ext80Rounding::NearestEven,
        0x3ff0000000000001,
        15,
    ), // normal_even_tie_+1_positive
    (
        F128 {
            hi: 0x3fff000000000000,
            lo: 0x0800000000000001,
        },
        Ext80Rounding::Down,
        0x3ff0000000000000,
        5,
    ), // normal_even_tie_+1_positive
    (
        F128 {
            hi: 0x3fff000000000000,
            lo: 0x0800000000000001,
        },
        Ext80Rounding::Up,
        0x3ff0000000000001,
        15,
    ), // normal_even_tie_+1_positive
    (
        F128 {
            hi: 0x3fff000000000000,
            lo: 0x0800000000000001,
        },
        Ext80Rounding::ToZero,
        0x3ff0000000000000,
        5,
    ), // normal_even_tie_+1_positive
    (
        F128 {
            hi: 0xbfff000000000000,
            lo: 0x0800000000000001,
        },
        Ext80Rounding::NearestEven,
        0xbff0000000000001,
        15,
    ), // normal_even_tie_+1_negative
    (
        F128 {
            hi: 0xbfff000000000000,
            lo: 0x0800000000000001,
        },
        Ext80Rounding::Down,
        0xbff0000000000001,
        15,
    ), // normal_even_tie_+1_negative
    (
        F128 {
            hi: 0xbfff000000000000,
            lo: 0x0800000000000001,
        },
        Ext80Rounding::Up,
        0xbff0000000000000,
        5,
    ), // normal_even_tie_+1_negative
    (
        F128 {
            hi: 0xbfff000000000000,
            lo: 0x0800000000000001,
        },
        Ext80Rounding::ToZero,
        0xbff0000000000000,
        5,
    ), // normal_even_tie_+1_negative
    (
        F128 {
            hi: 0x3fff000000000000,
            lo: 0x17ffffffffffffff,
        },
        Ext80Rounding::NearestEven,
        0x3ff0000000000001,
        5,
    ), // normal_odd_tie_-1_positive
    (
        F128 {
            hi: 0x3fff000000000000,
            lo: 0x17ffffffffffffff,
        },
        Ext80Rounding::Down,
        0x3ff0000000000001,
        5,
    ), // normal_odd_tie_-1_positive
    (
        F128 {
            hi: 0x3fff000000000000,
            lo: 0x17ffffffffffffff,
        },
        Ext80Rounding::Up,
        0x3ff0000000000002,
        15,
    ), // normal_odd_tie_-1_positive
    (
        F128 {
            hi: 0x3fff000000000000,
            lo: 0x17ffffffffffffff,
        },
        Ext80Rounding::ToZero,
        0x3ff0000000000001,
        5,
    ), // normal_odd_tie_-1_positive
    (
        F128 {
            hi: 0xbfff000000000000,
            lo: 0x17ffffffffffffff,
        },
        Ext80Rounding::NearestEven,
        0xbff0000000000001,
        5,
    ), // normal_odd_tie_-1_negative
    (
        F128 {
            hi: 0xbfff000000000000,
            lo: 0x17ffffffffffffff,
        },
        Ext80Rounding::Down,
        0xbff0000000000002,
        15,
    ), // normal_odd_tie_-1_negative
    (
        F128 {
            hi: 0xbfff000000000000,
            lo: 0x17ffffffffffffff,
        },
        Ext80Rounding::Up,
        0xbff0000000000001,
        5,
    ), // normal_odd_tie_-1_negative
    (
        F128 {
            hi: 0xbfff000000000000,
            lo: 0x17ffffffffffffff,
        },
        Ext80Rounding::ToZero,
        0xbff0000000000001,
        5,
    ), // normal_odd_tie_-1_negative
    (
        F128 {
            hi: 0x3fff000000000000,
            lo: 0x1800000000000000,
        },
        Ext80Rounding::NearestEven,
        0x3ff0000000000002,
        15,
    ), // normal_odd_tie_+0_positive
    (
        F128 {
            hi: 0x3fff000000000000,
            lo: 0x1800000000000000,
        },
        Ext80Rounding::Down,
        0x3ff0000000000001,
        5,
    ), // normal_odd_tie_+0_positive
    (
        F128 {
            hi: 0x3fff000000000000,
            lo: 0x1800000000000000,
        },
        Ext80Rounding::Up,
        0x3ff0000000000002,
        15,
    ), // normal_odd_tie_+0_positive
    (
        F128 {
            hi: 0x3fff000000000000,
            lo: 0x1800000000000000,
        },
        Ext80Rounding::ToZero,
        0x3ff0000000000001,
        5,
    ), // normal_odd_tie_+0_positive
    (
        F128 {
            hi: 0xbfff000000000000,
            lo: 0x1800000000000000,
        },
        Ext80Rounding::NearestEven,
        0xbff0000000000002,
        15,
    ), // normal_odd_tie_+0_negative
    (
        F128 {
            hi: 0xbfff000000000000,
            lo: 0x1800000000000000,
        },
        Ext80Rounding::Down,
        0xbff0000000000002,
        15,
    ), // normal_odd_tie_+0_negative
    (
        F128 {
            hi: 0xbfff000000000000,
            lo: 0x1800000000000000,
        },
        Ext80Rounding::Up,
        0xbff0000000000001,
        5,
    ), // normal_odd_tie_+0_negative
    (
        F128 {
            hi: 0xbfff000000000000,
            lo: 0x1800000000000000,
        },
        Ext80Rounding::ToZero,
        0xbff0000000000001,
        5,
    ), // normal_odd_tie_+0_negative
    (
        F128 {
            hi: 0x3fff000000000000,
            lo: 0x1800000000000001,
        },
        Ext80Rounding::NearestEven,
        0x3ff0000000000002,
        15,
    ), // normal_odd_tie_+1_positive
    (
        F128 {
            hi: 0x3fff000000000000,
            lo: 0x1800000000000001,
        },
        Ext80Rounding::Down,
        0x3ff0000000000001,
        5,
    ), // normal_odd_tie_+1_positive
    (
        F128 {
            hi: 0x3fff000000000000,
            lo: 0x1800000000000001,
        },
        Ext80Rounding::Up,
        0x3ff0000000000002,
        15,
    ), // normal_odd_tie_+1_positive
    (
        F128 {
            hi: 0x3fff000000000000,
            lo: 0x1800000000000001,
        },
        Ext80Rounding::ToZero,
        0x3ff0000000000001,
        5,
    ), // normal_odd_tie_+1_positive
    (
        F128 {
            hi: 0xbfff000000000000,
            lo: 0x1800000000000001,
        },
        Ext80Rounding::NearestEven,
        0xbff0000000000002,
        15,
    ), // normal_odd_tie_+1_negative
    (
        F128 {
            hi: 0xbfff000000000000,
            lo: 0x1800000000000001,
        },
        Ext80Rounding::Down,
        0xbff0000000000002,
        15,
    ), // normal_odd_tie_+1_negative
    (
        F128 {
            hi: 0xbfff000000000000,
            lo: 0x1800000000000001,
        },
        Ext80Rounding::Up,
        0xbff0000000000001,
        5,
    ), // normal_odd_tie_+1_negative
    (
        F128 {
            hi: 0xbfff000000000000,
            lo: 0x1800000000000001,
        },
        Ext80Rounding::ToZero,
        0xbff0000000000001,
        5,
    ), // normal_odd_tie_+1_negative
    (
        F128 {
            hi: 0x3bcbffffffffffff,
            lo: 0xfffffffffffffffe,
        },
        Ext80Rounding::NearestEven,
        0x0000000000000000,
        6605,
    ), // subzero_half_-1_positive
    (
        F128 {
            hi: 0x3bcbffffffffffff,
            lo: 0xfffffffffffffffe,
        },
        Ext80Rounding::Down,
        0x0000000000000000,
        6597,
    ), // subzero_half_-1_positive
    (
        F128 {
            hi: 0x3bcbffffffffffff,
            lo: 0xfffffffffffffffe,
        },
        Ext80Rounding::Up,
        0x0000000000000001,
        6607,
    ), // subzero_half_-1_positive
    (
        F128 {
            hi: 0x3bcbffffffffffff,
            lo: 0xfffffffffffffffe,
        },
        Ext80Rounding::ToZero,
        0x0000000000000000,
        6597,
    ), // subzero_half_-1_positive
    (
        F128 {
            hi: 0xbbcbffffffffffff,
            lo: 0xfffffffffffffffe,
        },
        Ext80Rounding::NearestEven,
        0x8000000000000000,
        6605,
    ), // subzero_half_-1_negative
    (
        F128 {
            hi: 0xbbcbffffffffffff,
            lo: 0xfffffffffffffffe,
        },
        Ext80Rounding::Down,
        0x8000000000000001,
        6607,
    ), // subzero_half_-1_negative
    (
        F128 {
            hi: 0xbbcbffffffffffff,
            lo: 0xfffffffffffffffe,
        },
        Ext80Rounding::Up,
        0x8000000000000000,
        6597,
    ), // subzero_half_-1_negative
    (
        F128 {
            hi: 0xbbcbffffffffffff,
            lo: 0xfffffffffffffffe,
        },
        Ext80Rounding::ToZero,
        0x8000000000000000,
        6597,
    ), // subzero_half_-1_negative
    (
        F128 {
            hi: 0x3bcc000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0x0000000000000000,
        6593,
    ), // subzero_half_+0_positive
    (
        F128 {
            hi: 0x3bcc000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0x0000000000000000,
        6593,
    ), // subzero_half_+0_positive
    (
        F128 {
            hi: 0x3bcc000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0x0000000000000001,
        6595,
    ), // subzero_half_+0_positive
    (
        F128 {
            hi: 0x3bcc000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0x0000000000000000,
        6593,
    ), // subzero_half_+0_positive
    (
        F128 {
            hi: 0xbbcc000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0x8000000000000000,
        6593,
    ), // subzero_half_+0_negative
    (
        F128 {
            hi: 0xbbcc000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0x8000000000000001,
        6595,
    ), // subzero_half_+0_negative
    (
        F128 {
            hi: 0xbbcc000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0x8000000000000000,
        6593,
    ), // subzero_half_+0_negative
    (
        F128 {
            hi: 0xbbcc000000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0x8000000000000000,
        6593,
    ), // subzero_half_+0_negative
    (
        F128 {
            hi: 0x3bcc000000000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::NearestEven,
        0x0000000000000001,
        6599,
    ), // subzero_half_+1_positive
    (
        F128 {
            hi: 0x3bcc000000000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::Down,
        0x0000000000000000,
        6597,
    ), // subzero_half_+1_positive
    (
        F128 {
            hi: 0x3bcc000000000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::Up,
        0x0000000000000001,
        6607,
    ), // subzero_half_+1_positive
    (
        F128 {
            hi: 0x3bcc000000000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::ToZero,
        0x0000000000000000,
        6597,
    ), // subzero_half_+1_positive
    (
        F128 {
            hi: 0xbbcc000000000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::NearestEven,
        0x8000000000000001,
        6599,
    ), // subzero_half_+1_negative
    (
        F128 {
            hi: 0xbbcc000000000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::Down,
        0x8000000000000001,
        6607,
    ), // subzero_half_+1_negative
    (
        F128 {
            hi: 0xbbcc000000000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::Up,
        0x8000000000000000,
        6597,
    ), // subzero_half_+1_negative
    (
        F128 {
            hi: 0xbbcc000000000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::ToZero,
        0x8000000000000000,
        6597,
    ), // subzero_half_+1_negative
    (
        F128 {
            hi: 0x3bcd7fffffffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::NearestEven,
        0x0000000000000001,
        6605,
    ), // subodd_half_-1_positive
    (
        F128 {
            hi: 0x3bcd7fffffffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::Down,
        0x0000000000000001,
        6597,
    ), // subodd_half_-1_positive
    (
        F128 {
            hi: 0x3bcd7fffffffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::Up,
        0x0000000000000002,
        6607,
    ), // subodd_half_-1_positive
    (
        F128 {
            hi: 0x3bcd7fffffffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::ToZero,
        0x0000000000000001,
        6597,
    ), // subodd_half_-1_positive
    (
        F128 {
            hi: 0xbbcd7fffffffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::NearestEven,
        0x8000000000000001,
        6605,
    ), // subodd_half_-1_negative
    (
        F128 {
            hi: 0xbbcd7fffffffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::Down,
        0x8000000000000002,
        6607,
    ), // subodd_half_-1_negative
    (
        F128 {
            hi: 0xbbcd7fffffffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::Up,
        0x8000000000000001,
        6597,
    ), // subodd_half_-1_negative
    (
        F128 {
            hi: 0xbbcd7fffffffffff,
            lo: 0xffffffffffffffff,
        },
        Ext80Rounding::ToZero,
        0x8000000000000001,
        6597,
    ), // subodd_half_-1_negative
    (
        F128 {
            hi: 0x3bcd800000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0x0000000000000002,
        6595,
    ), // subodd_half_+0_positive
    (
        F128 {
            hi: 0x3bcd800000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0x0000000000000001,
        6593,
    ), // subodd_half_+0_positive
    (
        F128 {
            hi: 0x3bcd800000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0x0000000000000002,
        6595,
    ), // subodd_half_+0_positive
    (
        F128 {
            hi: 0x3bcd800000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0x0000000000000001,
        6593,
    ), // subodd_half_+0_positive
    (
        F128 {
            hi: 0xbbcd800000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::NearestEven,
        0x8000000000000002,
        6595,
    ), // subodd_half_+0_negative
    (
        F128 {
            hi: 0xbbcd800000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Down,
        0x8000000000000002,
        6595,
    ), // subodd_half_+0_negative
    (
        F128 {
            hi: 0xbbcd800000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::Up,
        0x8000000000000001,
        6593,
    ), // subodd_half_+0_negative
    (
        F128 {
            hi: 0xbbcd800000000000,
            lo: 0x0000000000000000,
        },
        Ext80Rounding::ToZero,
        0x8000000000000001,
        6593,
    ), // subodd_half_+0_negative
    (
        F128 {
            hi: 0x3bcd800000000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::NearestEven,
        0x0000000000000002,
        6599,
    ), // subodd_half_+1_positive
    (
        F128 {
            hi: 0x3bcd800000000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::Down,
        0x0000000000000001,
        6597,
    ), // subodd_half_+1_positive
    (
        F128 {
            hi: 0x3bcd800000000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::Up,
        0x0000000000000002,
        6607,
    ), // subodd_half_+1_positive
    (
        F128 {
            hi: 0x3bcd800000000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::ToZero,
        0x0000000000000001,
        6597,
    ), // subodd_half_+1_positive
    (
        F128 {
            hi: 0xbbcd800000000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::NearestEven,
        0x8000000000000002,
        6599,
    ), // subodd_half_+1_negative
    (
        F128 {
            hi: 0xbbcd800000000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::Down,
        0x8000000000000002,
        6607,
    ), // subodd_half_+1_negative
    (
        F128 {
            hi: 0xbbcd800000000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::Up,
        0x8000000000000001,
        6597,
    ), // subodd_half_+1_negative
    (
        F128 {
            hi: 0xbbcd800000000000,
            lo: 0x0000000000000001,
        },
        Ext80Rounding::ToZero,
        0x8000000000000001,
        6597,
    ), // subodd_half_+1_negative
    (
        F128 {
            hi: 0x3c00ffffffffffff,
            lo: 0xefffffffffffffff,
        },
        Ext80Rounding::NearestEven,
        0x000fffffffffffff,
        6605,
    ), // minnormal_half_-1_positive
    (
        F128 {
            hi: 0x3c00ffffffffffff,
            lo: 0xefffffffffffffff,
        },
        Ext80Rounding::Down,
        0x000fffffffffffff,
        6597,
    ), // minnormal_half_-1_positive
    (
        F128 {
            hi: 0x3c00ffffffffffff,
            lo: 0xefffffffffffffff,
        },
        Ext80Rounding::Up,
        0x0010000000000000,
        6863,
    ), // minnormal_half_-1_positive
    (
        F128 {
            hi: 0x3c00ffffffffffff,
            lo: 0xefffffffffffffff,
        },
        Ext80Rounding::ToZero,
        0x000fffffffffffff,
        6597,
    ), // minnormal_half_-1_positive
    (
        F128 {
            hi: 0xbc00ffffffffffff,
            lo: 0xefffffffffffffff,
        },
        Ext80Rounding::NearestEven,
        0x800fffffffffffff,
        6605,
    ), // minnormal_half_-1_negative
    (
        F128 {
            hi: 0xbc00ffffffffffff,
            lo: 0xefffffffffffffff,
        },
        Ext80Rounding::Down,
        0x8010000000000000,
        6863,
    ), // minnormal_half_-1_negative
    (
        F128 {
            hi: 0xbc00ffffffffffff,
            lo: 0xefffffffffffffff,
        },
        Ext80Rounding::Up,
        0x800fffffffffffff,
        6597,
    ), // minnormal_half_-1_negative
    (
        F128 {
            hi: 0xbc00ffffffffffff,
            lo: 0xefffffffffffffff,
        },
        Ext80Rounding::ToZero,
        0x800fffffffffffff,
        6597,
    ), // minnormal_half_-1_negative
    (
        F128 {
            hi: 0x3c00ffffffffffff,
            lo: 0xf000000000000000,
        },
        Ext80Rounding::NearestEven,
        0x0010000000000000,
        6851,
    ), // minnormal_half_+0_positive
    (
        F128 {
            hi: 0x3c00ffffffffffff,
            lo: 0xf000000000000000,
        },
        Ext80Rounding::Down,
        0x000fffffffffffff,
        6593,
    ), // minnormal_half_+0_positive
    (
        F128 {
            hi: 0x3c00ffffffffffff,
            lo: 0xf000000000000000,
        },
        Ext80Rounding::Up,
        0x0010000000000000,
        6851,
    ), // minnormal_half_+0_positive
    (
        F128 {
            hi: 0x3c00ffffffffffff,
            lo: 0xf000000000000000,
        },
        Ext80Rounding::ToZero,
        0x000fffffffffffff,
        6593,
    ), // minnormal_half_+0_positive
    (
        F128 {
            hi: 0xbc00ffffffffffff,
            lo: 0xf000000000000000,
        },
        Ext80Rounding::NearestEven,
        0x8010000000000000,
        6851,
    ), // minnormal_half_+0_negative
    (
        F128 {
            hi: 0xbc00ffffffffffff,
            lo: 0xf000000000000000,
        },
        Ext80Rounding::Down,
        0x8010000000000000,
        6851,
    ), // minnormal_half_+0_negative
    (
        F128 {
            hi: 0xbc00ffffffffffff,
            lo: 0xf000000000000000,
        },
        Ext80Rounding::Up,
        0x800fffffffffffff,
        6593,
    ), // minnormal_half_+0_negative
    (
        F128 {
            hi: 0xbc00ffffffffffff,
            lo: 0xf000000000000000,
        },
        Ext80Rounding::ToZero,
        0x800fffffffffffff,
        6593,
    ), // minnormal_half_+0_negative
    (
        F128 {
            hi: 0x3c00ffffffffffff,
            lo: 0xf000000000000001,
        },
        Ext80Rounding::NearestEven,
        0x0010000000000000,
        6855,
    ), // minnormal_half_+1_positive
    (
        F128 {
            hi: 0x3c00ffffffffffff,
            lo: 0xf000000000000001,
        },
        Ext80Rounding::Down,
        0x000fffffffffffff,
        6597,
    ), // minnormal_half_+1_positive
    (
        F128 {
            hi: 0x3c00ffffffffffff,
            lo: 0xf000000000000001,
        },
        Ext80Rounding::Up,
        0x0010000000000000,
        591,
    ), // minnormal_half_+1_positive
    (
        F128 {
            hi: 0x3c00ffffffffffff,
            lo: 0xf000000000000001,
        },
        Ext80Rounding::ToZero,
        0x000fffffffffffff,
        6597,
    ), // minnormal_half_+1_positive
    (
        F128 {
            hi: 0xbc00ffffffffffff,
            lo: 0xf000000000000001,
        },
        Ext80Rounding::NearestEven,
        0x8010000000000000,
        6855,
    ), // minnormal_half_+1_negative
    (
        F128 {
            hi: 0xbc00ffffffffffff,
            lo: 0xf000000000000001,
        },
        Ext80Rounding::Down,
        0x8010000000000000,
        591,
    ), // minnormal_half_+1_negative
    (
        F128 {
            hi: 0xbc00ffffffffffff,
            lo: 0xf000000000000001,
        },
        Ext80Rounding::Up,
        0x800fffffffffffff,
        6597,
    ), // minnormal_half_+1_negative
    (
        F128 {
            hi: 0xbc00ffffffffffff,
            lo: 0xf000000000000001,
        },
        Ext80Rounding::ToZero,
        0x800fffffffffffff,
        6597,
    ), // minnormal_half_+1_negative
    (
        F128 {
            hi: 0x3c00ffffffffffff,
            lo: 0xf7ffffffffffffff,
        },
        Ext80Rounding::NearestEven,
        0x0010000000000000,
        6855,
    ), // minnormal_precision_half_-1_positive
    (
        F128 {
            hi: 0x3c00ffffffffffff,
            lo: 0xf7ffffffffffffff,
        },
        Ext80Rounding::Down,
        0x000fffffffffffff,
        6597,
    ), // minnormal_precision_half_-1_positive
    (
        F128 {
            hi: 0x3c00ffffffffffff,
            lo: 0xf7ffffffffffffff,
        },
        Ext80Rounding::Up,
        0x0010000000000000,
        591,
    ), // minnormal_precision_half_-1_positive
    (
        F128 {
            hi: 0x3c00ffffffffffff,
            lo: 0xf7ffffffffffffff,
        },
        Ext80Rounding::ToZero,
        0x000fffffffffffff,
        6597,
    ), // minnormal_precision_half_-1_positive
    (
        F128 {
            hi: 0xbc00ffffffffffff,
            lo: 0xf7ffffffffffffff,
        },
        Ext80Rounding::NearestEven,
        0x8010000000000000,
        6855,
    ), // minnormal_precision_half_-1_negative
    (
        F128 {
            hi: 0xbc00ffffffffffff,
            lo: 0xf7ffffffffffffff,
        },
        Ext80Rounding::Down,
        0x8010000000000000,
        591,
    ), // minnormal_precision_half_-1_negative
    (
        F128 {
            hi: 0xbc00ffffffffffff,
            lo: 0xf7ffffffffffffff,
        },
        Ext80Rounding::Up,
        0x800fffffffffffff,
        6597,
    ), // minnormal_precision_half_-1_negative
    (
        F128 {
            hi: 0xbc00ffffffffffff,
            lo: 0xf7ffffffffffffff,
        },
        Ext80Rounding::ToZero,
        0x800fffffffffffff,
        6597,
    ), // minnormal_precision_half_-1_negative
    (
        F128 {
            hi: 0x3c00ffffffffffff,
            lo: 0xf800000000000000,
        },
        Ext80Rounding::NearestEven,
        0x0010000000000000,
        591,
    ), // minnormal_precision_half_+0_positive
    (
        F128 {
            hi: 0x3c00ffffffffffff,
            lo: 0xf800000000000000,
        },
        Ext80Rounding::Down,
        0x000fffffffffffff,
        6597,
    ), // minnormal_precision_half_+0_positive
    (
        F128 {
            hi: 0x3c00ffffffffffff,
            lo: 0xf800000000000000,
        },
        Ext80Rounding::Up,
        0x0010000000000000,
        591,
    ), // minnormal_precision_half_+0_positive
    (
        F128 {
            hi: 0x3c00ffffffffffff,
            lo: 0xf800000000000000,
        },
        Ext80Rounding::ToZero,
        0x000fffffffffffff,
        6597,
    ), // minnormal_precision_half_+0_positive
    (
        F128 {
            hi: 0xbc00ffffffffffff,
            lo: 0xf800000000000000,
        },
        Ext80Rounding::NearestEven,
        0x8010000000000000,
        591,
    ), // minnormal_precision_half_+0_negative
    (
        F128 {
            hi: 0xbc00ffffffffffff,
            lo: 0xf800000000000000,
        },
        Ext80Rounding::Down,
        0x8010000000000000,
        591,
    ), // minnormal_precision_half_+0_negative
    (
        F128 {
            hi: 0xbc00ffffffffffff,
            lo: 0xf800000000000000,
        },
        Ext80Rounding::Up,
        0x800fffffffffffff,
        6597,
    ), // minnormal_precision_half_+0_negative
    (
        F128 {
            hi: 0xbc00ffffffffffff,
            lo: 0xf800000000000000,
        },
        Ext80Rounding::ToZero,
        0x800fffffffffffff,
        6597,
    ), // minnormal_precision_half_+0_negative
    (
        F128 {
            hi: 0x3c00ffffffffffff,
            lo: 0xf800000000000001,
        },
        Ext80Rounding::NearestEven,
        0x0010000000000000,
        591,
    ), // minnormal_precision_half_+1_positive
    (
        F128 {
            hi: 0x3c00ffffffffffff,
            lo: 0xf800000000000001,
        },
        Ext80Rounding::Down,
        0x000fffffffffffff,
        6597,
    ), // minnormal_precision_half_+1_positive
    (
        F128 {
            hi: 0x3c00ffffffffffff,
            lo: 0xf800000000000001,
        },
        Ext80Rounding::Up,
        0x0010000000000000,
        591,
    ), // minnormal_precision_half_+1_positive
    (
        F128 {
            hi: 0x3c00ffffffffffff,
            lo: 0xf800000000000001,
        },
        Ext80Rounding::ToZero,
        0x000fffffffffffff,
        6597,
    ), // minnormal_precision_half_+1_positive
    (
        F128 {
            hi: 0xbc00ffffffffffff,
            lo: 0xf800000000000001,
        },
        Ext80Rounding::NearestEven,
        0x8010000000000000,
        591,
    ), // minnormal_precision_half_+1_negative
    (
        F128 {
            hi: 0xbc00ffffffffffff,
            lo: 0xf800000000000001,
        },
        Ext80Rounding::Down,
        0x8010000000000000,
        591,
    ), // minnormal_precision_half_+1_negative
    (
        F128 {
            hi: 0xbc00ffffffffffff,
            lo: 0xf800000000000001,
        },
        Ext80Rounding::Up,
        0x800fffffffffffff,
        6597,
    ), // minnormal_precision_half_+1_negative
    (
        F128 {
            hi: 0xbc00ffffffffffff,
            lo: 0xf800000000000001,
        },
        Ext80Rounding::ToZero,
        0x800fffffffffffff,
        6597,
    ), // minnormal_precision_half_+1_negative
    (
        F128 {
            hi: 0x43feffffffffffff,
            lo: 0xf7ffffffffffffff,
        },
        Ext80Rounding::NearestEven,
        0x7fefffffffffffff,
        5,
    ), // overflow_half_-1_positive
    (
        F128 {
            hi: 0x43feffffffffffff,
            lo: 0xf7ffffffffffffff,
        },
        Ext80Rounding::Down,
        0x7fefffffffffffff,
        5,
    ), // overflow_half_-1_positive
    (
        F128 {
            hi: 0x43feffffffffffff,
            lo: 0xf7ffffffffffffff,
        },
        Ext80Rounding::Up,
        0x7ff0000000000000,
        63,
    ), // overflow_half_-1_positive
    (
        F128 {
            hi: 0x43feffffffffffff,
            lo: 0xf7ffffffffffffff,
        },
        Ext80Rounding::ToZero,
        0x7fefffffffffffff,
        5,
    ), // overflow_half_-1_positive
    (
        F128 {
            hi: 0xc3feffffffffffff,
            lo: 0xf7ffffffffffffff,
        },
        Ext80Rounding::NearestEven,
        0xffefffffffffffff,
        5,
    ), // overflow_half_-1_negative
    (
        F128 {
            hi: 0xc3feffffffffffff,
            lo: 0xf7ffffffffffffff,
        },
        Ext80Rounding::Down,
        0xfff0000000000000,
        63,
    ), // overflow_half_-1_negative
    (
        F128 {
            hi: 0xc3feffffffffffff,
            lo: 0xf7ffffffffffffff,
        },
        Ext80Rounding::Up,
        0xffefffffffffffff,
        5,
    ), // overflow_half_-1_negative
    (
        F128 {
            hi: 0xc3feffffffffffff,
            lo: 0xf7ffffffffffffff,
        },
        Ext80Rounding::ToZero,
        0xffefffffffffffff,
        5,
    ), // overflow_half_-1_negative
    (
        F128 {
            hi: 0x43feffffffffffff,
            lo: 0xf800000000000000,
        },
        Ext80Rounding::NearestEven,
        0x7ff0000000000000,
        63,
    ), // overflow_half_+0_positive
    (
        F128 {
            hi: 0x43feffffffffffff,
            lo: 0xf800000000000000,
        },
        Ext80Rounding::Down,
        0x7fefffffffffffff,
        5,
    ), // overflow_half_+0_positive
    (
        F128 {
            hi: 0x43feffffffffffff,
            lo: 0xf800000000000000,
        },
        Ext80Rounding::Up,
        0x7ff0000000000000,
        63,
    ), // overflow_half_+0_positive
    (
        F128 {
            hi: 0x43feffffffffffff,
            lo: 0xf800000000000000,
        },
        Ext80Rounding::ToZero,
        0x7fefffffffffffff,
        5,
    ), // overflow_half_+0_positive
    (
        F128 {
            hi: 0xc3feffffffffffff,
            lo: 0xf800000000000000,
        },
        Ext80Rounding::NearestEven,
        0xfff0000000000000,
        63,
    ), // overflow_half_+0_negative
    (
        F128 {
            hi: 0xc3feffffffffffff,
            lo: 0xf800000000000000,
        },
        Ext80Rounding::Down,
        0xfff0000000000000,
        63,
    ), // overflow_half_+0_negative
    (
        F128 {
            hi: 0xc3feffffffffffff,
            lo: 0xf800000000000000,
        },
        Ext80Rounding::Up,
        0xffefffffffffffff,
        5,
    ), // overflow_half_+0_negative
    (
        F128 {
            hi: 0xc3feffffffffffff,
            lo: 0xf800000000000000,
        },
        Ext80Rounding::ToZero,
        0xffefffffffffffff,
        5,
    ), // overflow_half_+0_negative
    (
        F128 {
            hi: 0x43feffffffffffff,
            lo: 0xf800000000000001,
        },
        Ext80Rounding::NearestEven,
        0x7ff0000000000000,
        63,
    ), // overflow_half_+1_positive
    (
        F128 {
            hi: 0x43feffffffffffff,
            lo: 0xf800000000000001,
        },
        Ext80Rounding::Down,
        0x7fefffffffffffff,
        5,
    ), // overflow_half_+1_positive
    (
        F128 {
            hi: 0x43feffffffffffff,
            lo: 0xf800000000000001,
        },
        Ext80Rounding::Up,
        0x7ff0000000000000,
        63,
    ), // overflow_half_+1_positive
    (
        F128 {
            hi: 0x43feffffffffffff,
            lo: 0xf800000000000001,
        },
        Ext80Rounding::ToZero,
        0x7fefffffffffffff,
        5,
    ), // overflow_half_+1_positive
    (
        F128 {
            hi: 0xc3feffffffffffff,
            lo: 0xf800000000000001,
        },
        Ext80Rounding::NearestEven,
        0xfff0000000000000,
        63,
    ), // overflow_half_+1_negative
    (
        F128 {
            hi: 0xc3feffffffffffff,
            lo: 0xf800000000000001,
        },
        Ext80Rounding::Down,
        0xfff0000000000000,
        63,
    ), // overflow_half_+1_negative
    (
        F128 {
            hi: 0xc3feffffffffffff,
            lo: 0xf800000000000001,
        },
        Ext80Rounding::Up,
        0xffefffffffffffff,
        5,
    ), // overflow_half_+1_negative
    (
        F128 {
            hi: 0xc3feffffffffffff,
            lo: 0xf800000000000001,
        },
        Ext80Rounding::ToZero,
        0xffefffffffffffff,
        5,
    ), // overflow_half_+1_negative
    (
        F128 {
            hi: 0x3c00ffffffffffff,
            lo: 0xe800000000000000,
        },
        Ext80Rounding::NearestEven,
        0x000fffffffffffff,
        6597,
    ), // minnormal_three_quarter_below_positive
    (
        F128 {
            hi: 0x3c00ffffffffffff,
            lo: 0xe800000000000000,
        },
        Ext80Rounding::Down,
        0x000fffffffffffff,
        6597,
    ), // minnormal_three_quarter_below_positive
    (
        F128 {
            hi: 0x3c00ffffffffffff,
            lo: 0xe800000000000000,
        },
        Ext80Rounding::Up,
        0x0010000000000000,
        6863,
    ), // minnormal_three_quarter_below_positive
    (
        F128 {
            hi: 0x3c00ffffffffffff,
            lo: 0xe800000000000000,
        },
        Ext80Rounding::ToZero,
        0x000fffffffffffff,
        6597,
    ), // minnormal_three_quarter_below_positive
    (
        F128 {
            hi: 0xbc00ffffffffffff,
            lo: 0xe800000000000000,
        },
        Ext80Rounding::NearestEven,
        0x800fffffffffffff,
        6597,
    ), // minnormal_three_quarter_below_negative
    (
        F128 {
            hi: 0xbc00ffffffffffff,
            lo: 0xe800000000000000,
        },
        Ext80Rounding::Down,
        0x8010000000000000,
        6863,
    ), // minnormal_three_quarter_below_negative
    (
        F128 {
            hi: 0xbc00ffffffffffff,
            lo: 0xe800000000000000,
        },
        Ext80Rounding::Up,
        0x800fffffffffffff,
        6597,
    ), // minnormal_three_quarter_below_negative
    (
        F128 {
            hi: 0xbc00ffffffffffff,
            lo: 0xe800000000000000,
        },
        Ext80Rounding::ToZero,
        0x800fffffffffffff,
        6597,
    ), // minnormal_three_quarter_below_negative
    (
        F128 {
            hi: 0x43feffffffffffff,
            lo: 0xf002000000000000,
        },
        Ext80Rounding::NearestEven,
        0x7fefffffffffffff,
        5,
    ), // maxfinite_plus_ext80_ulp_positive
    (
        F128 {
            hi: 0x43feffffffffffff,
            lo: 0xf002000000000000,
        },
        Ext80Rounding::Down,
        0x7fefffffffffffff,
        5,
    ), // maxfinite_plus_ext80_ulp_positive
    (
        F128 {
            hi: 0x43feffffffffffff,
            lo: 0xf002000000000000,
        },
        Ext80Rounding::Up,
        0x7ff0000000000000,
        63,
    ), // maxfinite_plus_ext80_ulp_positive
    (
        F128 {
            hi: 0x43feffffffffffff,
            lo: 0xf002000000000000,
        },
        Ext80Rounding::ToZero,
        0x7fefffffffffffff,
        5,
    ), // maxfinite_plus_ext80_ulp_positive
    (
        F128 {
            hi: 0xc3feffffffffffff,
            lo: 0xf002000000000000,
        },
        Ext80Rounding::NearestEven,
        0xffefffffffffffff,
        5,
    ), // maxfinite_plus_ext80_ulp_negative
    (
        F128 {
            hi: 0xc3feffffffffffff,
            lo: 0xf002000000000000,
        },
        Ext80Rounding::Down,
        0xfff0000000000000,
        63,
    ), // maxfinite_plus_ext80_ulp_negative
    (
        F128 {
            hi: 0xc3feffffffffffff,
            lo: 0xf002000000000000,
        },
        Ext80Rounding::Up,
        0xffefffffffffffff,
        5,
    ), // maxfinite_plus_ext80_ulp_negative
    (
        F128 {
            hi: 0xc3feffffffffffff,
            lo: 0xf002000000000000,
        },
        Ext80Rounding::ToZero,
        0xffefffffffffffff,
        5,
    ), // maxfinite_plus_ext80_ulp_negative
    (
        F128 {
            hi: 0x43feffffffffffff,
            lo: 0xf400000000000000,
        },
        Ext80Rounding::NearestEven,
        0x7fefffffffffffff,
        5,
    ), // maxfinite_plus_quarter_ulp_positive
    (
        F128 {
            hi: 0x43feffffffffffff,
            lo: 0xf400000000000000,
        },
        Ext80Rounding::Down,
        0x7fefffffffffffff,
        5,
    ), // maxfinite_plus_quarter_ulp_positive
    (
        F128 {
            hi: 0x43feffffffffffff,
            lo: 0xf400000000000000,
        },
        Ext80Rounding::Up,
        0x7ff0000000000000,
        63,
    ), // maxfinite_plus_quarter_ulp_positive
    (
        F128 {
            hi: 0x43feffffffffffff,
            lo: 0xf400000000000000,
        },
        Ext80Rounding::ToZero,
        0x7fefffffffffffff,
        5,
    ), // maxfinite_plus_quarter_ulp_positive
    (
        F128 {
            hi: 0xc3feffffffffffff,
            lo: 0xf400000000000000,
        },
        Ext80Rounding::NearestEven,
        0xffefffffffffffff,
        5,
    ), // maxfinite_plus_quarter_ulp_negative
    (
        F128 {
            hi: 0xc3feffffffffffff,
            lo: 0xf400000000000000,
        },
        Ext80Rounding::Down,
        0xfff0000000000000,
        63,
    ), // maxfinite_plus_quarter_ulp_negative
    (
        F128 {
            hi: 0xc3feffffffffffff,
            lo: 0xf400000000000000,
        },
        Ext80Rounding::Up,
        0xffefffffffffffff,
        5,
    ), // maxfinite_plus_quarter_ulp_negative
    (
        F128 {
            hi: 0xc3feffffffffffff,
            lo: 0xf400000000000000,
        },
        Ext80Rounding::ToZero,
        0xffefffffffffffff,
        5,
    ), // maxfinite_plus_quarter_ulp_negative
    (
        F128 {
            hi: 0x43feffffffffffff,
            lo: 0xeffe000000000000,
        },
        Ext80Rounding::NearestEven,
        0x7fefffffffffffff,
        15,
    ), // maxfinite_minus_ext80_ulp_positive
    (
        F128 {
            hi: 0x43feffffffffffff,
            lo: 0xeffe000000000000,
        },
        Ext80Rounding::Down,
        0x7feffffffffffffe,
        5,
    ), // maxfinite_minus_ext80_ulp_positive
    (
        F128 {
            hi: 0x43feffffffffffff,
            lo: 0xeffe000000000000,
        },
        Ext80Rounding::Up,
        0x7fefffffffffffff,
        15,
    ), // maxfinite_minus_ext80_ulp_positive
    (
        F128 {
            hi: 0x43feffffffffffff,
            lo: 0xeffe000000000000,
        },
        Ext80Rounding::ToZero,
        0x7feffffffffffffe,
        5,
    ), // maxfinite_minus_ext80_ulp_positive
    (
        F128 {
            hi: 0xc3feffffffffffff,
            lo: 0xeffe000000000000,
        },
        Ext80Rounding::NearestEven,
        0xffefffffffffffff,
        15,
    ), // maxfinite_minus_ext80_ulp_negative
    (
        F128 {
            hi: 0xc3feffffffffffff,
            lo: 0xeffe000000000000,
        },
        Ext80Rounding::Down,
        0xffefffffffffffff,
        15,
    ), // maxfinite_minus_ext80_ulp_negative
    (
        F128 {
            hi: 0xc3feffffffffffff,
            lo: 0xeffe000000000000,
        },
        Ext80Rounding::Up,
        0xffeffffffffffffe,
        5,
    ), // maxfinite_minus_ext80_ulp_negative
    (
        F128 {
            hi: 0xc3feffffffffffff,
            lo: 0xeffe000000000000,
        },
        Ext80Rounding::ToZero,
        0xffeffffffffffffe,
        5,
    ), // maxfinite_minus_ext80_ulp_negative
    (
        F128 {
            hi: 0x0000357bbe2e809b,
            lo: 0x190b9a0f02eb207c,
        },
        Ext80Rounding::NearestEven,
        0x0000000000000000,
        6605,
    ), // raw_field0_sample0_positive
    (
        F128 {
            hi: 0x0000357bbe2e809b,
            lo: 0x190b9a0f02eb207c,
        },
        Ext80Rounding::Down,
        0x0000000000000000,
        6597,
    ), // raw_field0_sample0_positive
    (
        F128 {
            hi: 0x0000357bbe2e809b,
            lo: 0x190b9a0f02eb207c,
        },
        Ext80Rounding::Up,
        0x0000000000000001,
        6607,
    ), // raw_field0_sample0_positive
    (
        F128 {
            hi: 0x0000357bbe2e809b,
            lo: 0x190b9a0f02eb207c,
        },
        Ext80Rounding::ToZero,
        0x0000000000000000,
        6597,
    ), // raw_field0_sample0_positive
    (
        F128 {
            hi: 0x8000357bbe2e809b,
            lo: 0x190b9a0f02eb207c,
        },
        Ext80Rounding::NearestEven,
        0x8000000000000000,
        6605,
    ), // raw_field0_sample0_negative
    (
        F128 {
            hi: 0x8000357bbe2e809b,
            lo: 0x190b9a0f02eb207c,
        },
        Ext80Rounding::Down,
        0x8000000000000001,
        6607,
    ), // raw_field0_sample0_negative
    (
        F128 {
            hi: 0x8000357bbe2e809b,
            lo: 0x190b9a0f02eb207c,
        },
        Ext80Rounding::Up,
        0x8000000000000000,
        6597,
    ), // raw_field0_sample0_negative
    (
        F128 {
            hi: 0x8000357bbe2e809b,
            lo: 0x190b9a0f02eb207c,
        },
        Ext80Rounding::ToZero,
        0x8000000000000000,
        6597,
    ), // raw_field0_sample0_negative
    (
        F128 {
            hi: 0x000014b1a5650ec2,
            lo: 0x3066f6cf6ba8fe4d,
        },
        Ext80Rounding::NearestEven,
        0x0000000000000000,
        6597,
    ), // raw_field0_sample1_positive
    (
        F128 {
            hi: 0x000014b1a5650ec2,
            lo: 0x3066f6cf6ba8fe4d,
        },
        Ext80Rounding::Down,
        0x0000000000000000,
        6597,
    ), // raw_field0_sample1_positive
    (
        F128 {
            hi: 0x000014b1a5650ec2,
            lo: 0x3066f6cf6ba8fe4d,
        },
        Ext80Rounding::Up,
        0x0000000000000001,
        6607,
    ), // raw_field0_sample1_positive
    (
        F128 {
            hi: 0x000014b1a5650ec2,
            lo: 0x3066f6cf6ba8fe4d,
        },
        Ext80Rounding::ToZero,
        0x0000000000000000,
        6597,
    ), // raw_field0_sample1_positive
    (
        F128 {
            hi: 0x800014b1a5650ec2,
            lo: 0x3066f6cf6ba8fe4d,
        },
        Ext80Rounding::NearestEven,
        0x8000000000000000,
        6597,
    ), // raw_field0_sample1_negative
    (
        F128 {
            hi: 0x800014b1a5650ec2,
            lo: 0x3066f6cf6ba8fe4d,
        },
        Ext80Rounding::Down,
        0x8000000000000001,
        6607,
    ), // raw_field0_sample1_negative
    (
        F128 {
            hi: 0x800014b1a5650ec2,
            lo: 0x3066f6cf6ba8fe4d,
        },
        Ext80Rounding::Up,
        0x8000000000000000,
        6597,
    ), // raw_field0_sample1_negative
    (
        F128 {
            hi: 0x800014b1a5650ec2,
            lo: 0x3066f6cf6ba8fe4d,
        },
        Ext80Rounding::ToZero,
        0x8000000000000000,
        6597,
    ), // raw_field0_sample1_negative
    (
        F128 {
            hi: 0x0001475c775db3d8,
            lo: 0xf3372af181b161d1,
        },
        Ext80Rounding::NearestEven,
        0x0000000000000000,
        6597,
    ), // raw_field1_sample0_positive
    (
        F128 {
            hi: 0x0001475c775db3d8,
            lo: 0xf3372af181b161d1,
        },
        Ext80Rounding::Down,
        0x0000000000000000,
        6597,
    ), // raw_field1_sample0_positive
    (
        F128 {
            hi: 0x0001475c775db3d8,
            lo: 0xf3372af181b161d1,
        },
        Ext80Rounding::Up,
        0x0000000000000001,
        6607,
    ), // raw_field1_sample0_positive
    (
        F128 {
            hi: 0x0001475c775db3d8,
            lo: 0xf3372af181b161d1,
        },
        Ext80Rounding::ToZero,
        0x0000000000000000,
        6597,
    ), // raw_field1_sample0_positive
    (
        F128 {
            hi: 0x8001475c775db3d8,
            lo: 0xf3372af181b161d1,
        },
        Ext80Rounding::NearestEven,
        0x8000000000000000,
        6597,
    ), // raw_field1_sample0_negative
    (
        F128 {
            hi: 0x8001475c775db3d8,
            lo: 0xf3372af181b161d1,
        },
        Ext80Rounding::Down,
        0x8000000000000001,
        6607,
    ), // raw_field1_sample0_negative
    (
        F128 {
            hi: 0x8001475c775db3d8,
            lo: 0xf3372af181b161d1,
        },
        Ext80Rounding::Up,
        0x8000000000000000,
        6597,
    ), // raw_field1_sample0_negative
    (
        F128 {
            hi: 0x8001475c775db3d8,
            lo: 0xf3372af181b161d1,
        },
        Ext80Rounding::ToZero,
        0x8000000000000000,
        6597,
    ), // raw_field1_sample0_negative
    (
        F128 {
            hi: 0x00015dbeeb79e012,
            lo: 0xe669bb01790ceb3d,
        },
        Ext80Rounding::NearestEven,
        0x0000000000000000,
        6597,
    ), // raw_field1_sample1_positive
    (
        F128 {
            hi: 0x00015dbeeb79e012,
            lo: 0xe669bb01790ceb3d,
        },
        Ext80Rounding::Down,
        0x0000000000000000,
        6597,
    ), // raw_field1_sample1_positive
    (
        F128 {
            hi: 0x00015dbeeb79e012,
            lo: 0xe669bb01790ceb3d,
        },
        Ext80Rounding::Up,
        0x0000000000000001,
        6607,
    ), // raw_field1_sample1_positive
    (
        F128 {
            hi: 0x00015dbeeb79e012,
            lo: 0xe669bb01790ceb3d,
        },
        Ext80Rounding::ToZero,
        0x0000000000000000,
        6597,
    ), // raw_field1_sample1_positive
    (
        F128 {
            hi: 0x80015dbeeb79e012,
            lo: 0xe669bb01790ceb3d,
        },
        Ext80Rounding::NearestEven,
        0x8000000000000000,
        6597,
    ), // raw_field1_sample1_negative
    (
        F128 {
            hi: 0x80015dbeeb79e012,
            lo: 0xe669bb01790ceb3d,
        },
        Ext80Rounding::Down,
        0x8000000000000001,
        6607,
    ), // raw_field1_sample1_negative
    (
        F128 {
            hi: 0x80015dbeeb79e012,
            lo: 0xe669bb01790ceb3d,
        },
        Ext80Rounding::Up,
        0x8000000000000000,
        6597,
    ), // raw_field1_sample1_negative
    (
        F128 {
            hi: 0x80015dbeeb79e012,
            lo: 0xe669bb01790ceb3d,
        },
        Ext80Rounding::ToZero,
        0x8000000000000000,
        6597,
    ), // raw_field1_sample1_negative
    (
        F128 {
            hi: 0x00020c7bf5605c1b,
            lo: 0x78b702511bdf8f84,
        },
        Ext80Rounding::NearestEven,
        0x0000000000000000,
        6605,
    ), // raw_field2_sample0_positive
    (
        F128 {
            hi: 0x00020c7bf5605c1b,
            lo: 0x78b702511bdf8f84,
        },
        Ext80Rounding::Down,
        0x0000000000000000,
        6597,
    ), // raw_field2_sample0_positive
    (
        F128 {
            hi: 0x00020c7bf5605c1b,
            lo: 0x78b702511bdf8f84,
        },
        Ext80Rounding::Up,
        0x0000000000000001,
        6607,
    ), // raw_field2_sample0_positive
    (
        F128 {
            hi: 0x00020c7bf5605c1b,
            lo: 0x78b702511bdf8f84,
        },
        Ext80Rounding::ToZero,
        0x0000000000000000,
        6597,
    ), // raw_field2_sample0_positive
    (
        F128 {
            hi: 0x80020c7bf5605c1b,
            lo: 0x78b702511bdf8f84,
        },
        Ext80Rounding::NearestEven,
        0x8000000000000000,
        6605,
    ), // raw_field2_sample0_negative
    (
        F128 {
            hi: 0x80020c7bf5605c1b,
            lo: 0x78b702511bdf8f84,
        },
        Ext80Rounding::Down,
        0x8000000000000001,
        6607,
    ), // raw_field2_sample0_negative
    (
        F128 {
            hi: 0x80020c7bf5605c1b,
            lo: 0x78b702511bdf8f84,
        },
        Ext80Rounding::Up,
        0x8000000000000000,
        6597,
    ), // raw_field2_sample0_negative
    (
        F128 {
            hi: 0x80020c7bf5605c1b,
            lo: 0x78b702511bdf8f84,
        },
        Ext80Rounding::ToZero,
        0x8000000000000000,
        6597,
    ), // raw_field2_sample0_negative
    (
        F128 {
            hi: 0x0002a732e6564f5b,
            lo: 0xe4f1bef0f9e8c955,
        },
        Ext80Rounding::NearestEven,
        0x0000000000000000,
        6597,
    ), // raw_field2_sample1_positive
    (
        F128 {
            hi: 0x0002a732e6564f5b,
            lo: 0xe4f1bef0f9e8c955,
        },
        Ext80Rounding::Down,
        0x0000000000000000,
        6597,
    ), // raw_field2_sample1_positive
    (
        F128 {
            hi: 0x0002a732e6564f5b,
            lo: 0xe4f1bef0f9e8c955,
        },
        Ext80Rounding::Up,
        0x0000000000000001,
        6607,
    ), // raw_field2_sample1_positive
    (
        F128 {
            hi: 0x0002a732e6564f5b,
            lo: 0xe4f1bef0f9e8c955,
        },
        Ext80Rounding::ToZero,
        0x0000000000000000,
        6597,
    ), // raw_field2_sample1_positive
    (
        F128 {
            hi: 0x8002a732e6564f5b,
            lo: 0xe4f1bef0f9e8c955,
        },
        Ext80Rounding::NearestEven,
        0x8000000000000000,
        6597,
    ), // raw_field2_sample1_negative
    (
        F128 {
            hi: 0x8002a732e6564f5b,
            lo: 0xe4f1bef0f9e8c955,
        },
        Ext80Rounding::Down,
        0x8000000000000001,
        6607,
    ), // raw_field2_sample1_negative
    (
        F128 {
            hi: 0x8002a732e6564f5b,
            lo: 0xe4f1bef0f9e8c955,
        },
        Ext80Rounding::Up,
        0x8000000000000000,
        6597,
    ), // raw_field2_sample1_negative
    (
        F128 {
            hi: 0x8002a732e6564f5b,
            lo: 0xe4f1bef0f9e8c955,
        },
        Ext80Rounding::ToZero,
        0x8000000000000000,
        6597,
    ), // raw_field2_sample1_negative
    (
        F128 {
            hi: 0x3a9854cf0ffb0193,
            lo: 0xcc2ec272237b3c4d,
        },
        Ext80Rounding::NearestEven,
        0x0000000000000000,
        6605,
    ), // raw_field15000_sample0_positive
    (
        F128 {
            hi: 0x3a9854cf0ffb0193,
            lo: 0xcc2ec272237b3c4d,
        },
        Ext80Rounding::Down,
        0x0000000000000000,
        6597,
    ), // raw_field15000_sample0_positive
    (
        F128 {
            hi: 0x3a9854cf0ffb0193,
            lo: 0xcc2ec272237b3c4d,
        },
        Ext80Rounding::Up,
        0x0000000000000001,
        6607,
    ), // raw_field15000_sample0_positive
    (
        F128 {
            hi: 0x3a9854cf0ffb0193,
            lo: 0xcc2ec272237b3c4d,
        },
        Ext80Rounding::ToZero,
        0x0000000000000000,
        6597,
    ), // raw_field15000_sample0_positive
    (
        F128 {
            hi: 0xba9854cf0ffb0193,
            lo: 0xcc2ec272237b3c4d,
        },
        Ext80Rounding::NearestEven,
        0x8000000000000000,
        6605,
    ), // raw_field15000_sample0_negative
    (
        F128 {
            hi: 0xba9854cf0ffb0193,
            lo: 0xcc2ec272237b3c4d,
        },
        Ext80Rounding::Down,
        0x8000000000000001,
        6607,
    ), // raw_field15000_sample0_negative
    (
        F128 {
            hi: 0xba9854cf0ffb0193,
            lo: 0xcc2ec272237b3c4d,
        },
        Ext80Rounding::Up,
        0x8000000000000000,
        6597,
    ), // raw_field15000_sample0_negative
    (
        F128 {
            hi: 0xba9854cf0ffb0193,
            lo: 0xcc2ec272237b3c4d,
        },
        Ext80Rounding::ToZero,
        0x8000000000000000,
        6597,
    ), // raw_field15000_sample0_negative
    (
        F128 {
            hi: 0x3a98f0acfc607957,
            lo: 0xd7587c8108efcbfe,
        },
        Ext80Rounding::NearestEven,
        0x0000000000000000,
        6597,
    ), // raw_field15000_sample1_positive
    (
        F128 {
            hi: 0x3a98f0acfc607957,
            lo: 0xd7587c8108efcbfe,
        },
        Ext80Rounding::Down,
        0x0000000000000000,
        6597,
    ), // raw_field15000_sample1_positive
    (
        F128 {
            hi: 0x3a98f0acfc607957,
            lo: 0xd7587c8108efcbfe,
        },
        Ext80Rounding::Up,
        0x0000000000000001,
        6607,
    ), // raw_field15000_sample1_positive
    (
        F128 {
            hi: 0x3a98f0acfc607957,
            lo: 0xd7587c8108efcbfe,
        },
        Ext80Rounding::ToZero,
        0x0000000000000000,
        6597,
    ), // raw_field15000_sample1_positive
    (
        F128 {
            hi: 0xba98f0acfc607957,
            lo: 0xd7587c8108efcbfe,
        },
        Ext80Rounding::NearestEven,
        0x8000000000000000,
        6597,
    ), // raw_field15000_sample1_negative
    (
        F128 {
            hi: 0xba98f0acfc607957,
            lo: 0xd7587c8108efcbfe,
        },
        Ext80Rounding::Down,
        0x8000000000000001,
        6607,
    ), // raw_field15000_sample1_negative
    (
        F128 {
            hi: 0xba98f0acfc607957,
            lo: 0xd7587c8108efcbfe,
        },
        Ext80Rounding::Up,
        0x8000000000000000,
        6597,
    ), // raw_field15000_sample1_negative
    (
        F128 {
            hi: 0xba98f0acfc607957,
            lo: 0xd7587c8108efcbfe,
        },
        Ext80Rounding::ToZero,
        0x8000000000000000,
        6597,
    ), // raw_field15000_sample1_negative
    (
        F128 {
            hi: 0x3bb00d4451fd3a17,
            lo: 0xb5f7ac7838eecfbf,
        },
        Ext80Rounding::NearestEven,
        0x0000000000000000,
        6597,
    ), // raw_field15280_sample0_positive
    (
        F128 {
            hi: 0x3bb00d4451fd3a17,
            lo: 0xb5f7ac7838eecfbf,
        },
        Ext80Rounding::Down,
        0x0000000000000000,
        6597,
    ), // raw_field15280_sample0_positive
    (
        F128 {
            hi: 0x3bb00d4451fd3a17,
            lo: 0xb5f7ac7838eecfbf,
        },
        Ext80Rounding::Up,
        0x0000000000000001,
        6607,
    ), // raw_field15280_sample0_positive
    (
        F128 {
            hi: 0x3bb00d4451fd3a17,
            lo: 0xb5f7ac7838eecfbf,
        },
        Ext80Rounding::ToZero,
        0x0000000000000000,
        6597,
    ), // raw_field15280_sample0_positive
    (
        F128 {
            hi: 0xbbb00d4451fd3a17,
            lo: 0xb5f7ac7838eecfbf,
        },
        Ext80Rounding::NearestEven,
        0x8000000000000000,
        6597,
    ), // raw_field15280_sample0_negative
    (
        F128 {
            hi: 0xbbb00d4451fd3a17,
            lo: 0xb5f7ac7838eecfbf,
        },
        Ext80Rounding::Down,
        0x8000000000000001,
        6607,
    ), // raw_field15280_sample0_negative
    (
        F128 {
            hi: 0xbbb00d4451fd3a17,
            lo: 0xb5f7ac7838eecfbf,
        },
        Ext80Rounding::Up,
        0x8000000000000000,
        6597,
    ), // raw_field15280_sample0_negative
    (
        F128 {
            hi: 0xbbb00d4451fd3a17,
            lo: 0xb5f7ac7838eecfbf,
        },
        Ext80Rounding::ToZero,
        0x8000000000000000,
        6597,
    ), // raw_field15280_sample0_negative
    (
        F128 {
            hi: 0x3bb03111d981ca30,
            lo: 0x285be5bdd7dd806a,
        },
        Ext80Rounding::NearestEven,
        0x0000000000000000,
        6605,
    ), // raw_field15280_sample1_positive
    (
        F128 {
            hi: 0x3bb03111d981ca30,
            lo: 0x285be5bdd7dd806a,
        },
        Ext80Rounding::Down,
        0x0000000000000000,
        6597,
    ), // raw_field15280_sample1_positive
    (
        F128 {
            hi: 0x3bb03111d981ca30,
            lo: 0x285be5bdd7dd806a,
        },
        Ext80Rounding::Up,
        0x0000000000000001,
        6607,
    ), // raw_field15280_sample1_positive
    (
        F128 {
            hi: 0x3bb03111d981ca30,
            lo: 0x285be5bdd7dd806a,
        },
        Ext80Rounding::ToZero,
        0x0000000000000000,
        6597,
    ), // raw_field15280_sample1_positive
    (
        F128 {
            hi: 0xbbb03111d981ca30,
            lo: 0x285be5bdd7dd806a,
        },
        Ext80Rounding::NearestEven,
        0x8000000000000000,
        6605,
    ), // raw_field15280_sample1_negative
    (
        F128 {
            hi: 0xbbb03111d981ca30,
            lo: 0x285be5bdd7dd806a,
        },
        Ext80Rounding::Down,
        0x8000000000000001,
        6607,
    ), // raw_field15280_sample1_negative
    (
        F128 {
            hi: 0xbbb03111d981ca30,
            lo: 0x285be5bdd7dd806a,
        },
        Ext80Rounding::Up,
        0x8000000000000000,
        6597,
    ), // raw_field15280_sample1_negative
    (
        F128 {
            hi: 0xbbb03111d981ca30,
            lo: 0x285be5bdd7dd806a,
        },
        Ext80Rounding::ToZero,
        0x8000000000000000,
        6597,
    ), // raw_field15280_sample1_negative
    (
        F128 {
            hi: 0x3bcc8cf0a77abea2,
            lo: 0x28e6c77f4141cda4,
        },
        Ext80Rounding::NearestEven,
        0x0000000000000001,
        6607,
    ), // raw_field15308_sample0_positive
    (
        F128 {
            hi: 0x3bcc8cf0a77abea2,
            lo: 0x28e6c77f4141cda4,
        },
        Ext80Rounding::Down,
        0x0000000000000000,
        6597,
    ), // raw_field15308_sample0_positive
    (
        F128 {
            hi: 0x3bcc8cf0a77abea2,
            lo: 0x28e6c77f4141cda4,
        },
        Ext80Rounding::Up,
        0x0000000000000001,
        6607,
    ), // raw_field15308_sample0_positive
    (
        F128 {
            hi: 0x3bcc8cf0a77abea2,
            lo: 0x28e6c77f4141cda4,
        },
        Ext80Rounding::ToZero,
        0x0000000000000000,
        6597,
    ), // raw_field15308_sample0_positive
    (
        F128 {
            hi: 0xbbcc8cf0a77abea2,
            lo: 0x28e6c77f4141cda4,
        },
        Ext80Rounding::NearestEven,
        0x8000000000000001,
        6607,
    ), // raw_field15308_sample0_negative
    (
        F128 {
            hi: 0xbbcc8cf0a77abea2,
            lo: 0x28e6c77f4141cda4,
        },
        Ext80Rounding::Down,
        0x8000000000000001,
        6607,
    ), // raw_field15308_sample0_negative
    (
        F128 {
            hi: 0xbbcc8cf0a77abea2,
            lo: 0x28e6c77f4141cda4,
        },
        Ext80Rounding::Up,
        0x8000000000000000,
        6597,
    ), // raw_field15308_sample0_negative
    (
        F128 {
            hi: 0xbbcc8cf0a77abea2,
            lo: 0x28e6c77f4141cda4,
        },
        Ext80Rounding::ToZero,
        0x8000000000000000,
        6597,
    ), // raw_field15308_sample0_negative
    (
        F128 {
            hi: 0x3bcce185f2937564,
            lo: 0xbe13d45874e12216,
        },
        Ext80Rounding::NearestEven,
        0x0000000000000001,
        6607,
    ), // raw_field15308_sample1_positive
    (
        F128 {
            hi: 0x3bcce185f2937564,
            lo: 0xbe13d45874e12216,
        },
        Ext80Rounding::Down,
        0x0000000000000000,
        6597,
    ), // raw_field15308_sample1_positive
    (
        F128 {
            hi: 0x3bcce185f2937564,
            lo: 0xbe13d45874e12216,
        },
        Ext80Rounding::Up,
        0x0000000000000001,
        6607,
    ), // raw_field15308_sample1_positive
    (
        F128 {
            hi: 0x3bcce185f2937564,
            lo: 0xbe13d45874e12216,
        },
        Ext80Rounding::ToZero,
        0x0000000000000000,
        6597,
    ), // raw_field15308_sample1_positive
    (
        F128 {
            hi: 0xbbcce185f2937564,
            lo: 0xbe13d45874e12216,
        },
        Ext80Rounding::NearestEven,
        0x8000000000000001,
        6607,
    ), // raw_field15308_sample1_negative
    (
        F128 {
            hi: 0xbbcce185f2937564,
            lo: 0xbe13d45874e12216,
        },
        Ext80Rounding::Down,
        0x8000000000000001,
        6607,
    ), // raw_field15308_sample1_negative
    (
        F128 {
            hi: 0xbbcce185f2937564,
            lo: 0xbe13d45874e12216,
        },
        Ext80Rounding::Up,
        0x8000000000000000,
        6597,
    ), // raw_field15308_sample1_negative
    (
        F128 {
            hi: 0xbbcce185f2937564,
            lo: 0xbe13d45874e12216,
        },
        Ext80Rounding::ToZero,
        0x8000000000000000,
        6597,
    ), // raw_field15308_sample1_negative
    (
        F128 {
            hi: 0x3bcd174502a91084,
            lo: 0x5ea5adf0437d3a74,
        },
        Ext80Rounding::NearestEven,
        0x0000000000000001,
        6605,
    ), // raw_field15309_sample0_positive
    (
        F128 {
            hi: 0x3bcd174502a91084,
            lo: 0x5ea5adf0437d3a74,
        },
        Ext80Rounding::Down,
        0x0000000000000001,
        6597,
    ), // raw_field15309_sample0_positive
    (
        F128 {
            hi: 0x3bcd174502a91084,
            lo: 0x5ea5adf0437d3a74,
        },
        Ext80Rounding::Up,
        0x0000000000000002,
        6607,
    ), // raw_field15309_sample0_positive
    (
        F128 {
            hi: 0x3bcd174502a91084,
            lo: 0x5ea5adf0437d3a74,
        },
        Ext80Rounding::ToZero,
        0x0000000000000001,
        6597,
    ), // raw_field15309_sample0_positive
    (
        F128 {
            hi: 0xbbcd174502a91084,
            lo: 0x5ea5adf0437d3a74,
        },
        Ext80Rounding::NearestEven,
        0x8000000000000001,
        6605,
    ), // raw_field15309_sample0_negative
    (
        F128 {
            hi: 0xbbcd174502a91084,
            lo: 0x5ea5adf0437d3a74,
        },
        Ext80Rounding::Down,
        0x8000000000000002,
        6607,
    ), // raw_field15309_sample0_negative
    (
        F128 {
            hi: 0xbbcd174502a91084,
            lo: 0x5ea5adf0437d3a74,
        },
        Ext80Rounding::Up,
        0x8000000000000001,
        6597,
    ), // raw_field15309_sample0_negative
    (
        F128 {
            hi: 0xbbcd174502a91084,
            lo: 0x5ea5adf0437d3a74,
        },
        Ext80Rounding::ToZero,
        0x8000000000000001,
        6597,
    ), // raw_field15309_sample0_negative
    (
        F128 {
            hi: 0x3bcd02cca64121ef,
            lo: 0x24f87b7d556c5053,
        },
        Ext80Rounding::NearestEven,
        0x0000000000000001,
        6597,
    ), // raw_field15309_sample1_positive
    (
        F128 {
            hi: 0x3bcd02cca64121ef,
            lo: 0x24f87b7d556c5053,
        },
        Ext80Rounding::Down,
        0x0000000000000001,
        6597,
    ), // raw_field15309_sample1_positive
    (
        F128 {
            hi: 0x3bcd02cca64121ef,
            lo: 0x24f87b7d556c5053,
        },
        Ext80Rounding::Up,
        0x0000000000000002,
        6607,
    ), // raw_field15309_sample1_positive
    (
        F128 {
            hi: 0x3bcd02cca64121ef,
            lo: 0x24f87b7d556c5053,
        },
        Ext80Rounding::ToZero,
        0x0000000000000001,
        6597,
    ), // raw_field15309_sample1_positive
    (
        F128 {
            hi: 0xbbcd02cca64121ef,
            lo: 0x24f87b7d556c5053,
        },
        Ext80Rounding::NearestEven,
        0x8000000000000001,
        6597,
    ), // raw_field15309_sample1_negative
    (
        F128 {
            hi: 0xbbcd02cca64121ef,
            lo: 0x24f87b7d556c5053,
        },
        Ext80Rounding::Down,
        0x8000000000000002,
        6607,
    ), // raw_field15309_sample1_negative
    (
        F128 {
            hi: 0xbbcd02cca64121ef,
            lo: 0x24f87b7d556c5053,
        },
        Ext80Rounding::Up,
        0x8000000000000001,
        6597,
    ), // raw_field15309_sample1_negative
    (
        F128 {
            hi: 0xbbcd02cca64121ef,
            lo: 0x24f87b7d556c5053,
        },
        Ext80Rounding::ToZero,
        0x8000000000000001,
        6597,
    ), // raw_field15309_sample1_negative
    (
        F128 {
            hi: 0x3c007276982b0dca,
            lo: 0x3bfeb4257c383b12,
        },
        Ext80Rounding::NearestEven,
        0x000b93b4c1586e52,
        6607,
    ), // raw_field15360_sample0_positive
    (
        F128 {
            hi: 0x3c007276982b0dca,
            lo: 0x3bfeb4257c383b12,
        },
        Ext80Rounding::Down,
        0x000b93b4c1586e51,
        6597,
    ), // raw_field15360_sample0_positive
    (
        F128 {
            hi: 0x3c007276982b0dca,
            lo: 0x3bfeb4257c383b12,
        },
        Ext80Rounding::Up,
        0x000b93b4c1586e52,
        6607,
    ), // raw_field15360_sample0_positive
    (
        F128 {
            hi: 0x3c007276982b0dca,
            lo: 0x3bfeb4257c383b12,
        },
        Ext80Rounding::ToZero,
        0x000b93b4c1586e51,
        6597,
    ), // raw_field15360_sample0_positive
    (
        F128 {
            hi: 0xbc007276982b0dca,
            lo: 0x3bfeb4257c383b12,
        },
        Ext80Rounding::NearestEven,
        0x800b93b4c1586e52,
        6607,
    ), // raw_field15360_sample0_negative
    (
        F128 {
            hi: 0xbc007276982b0dca,
            lo: 0x3bfeb4257c383b12,
        },
        Ext80Rounding::Down,
        0x800b93b4c1586e52,
        6607,
    ), // raw_field15360_sample0_negative
    (
        F128 {
            hi: 0xbc007276982b0dca,
            lo: 0x3bfeb4257c383b12,
        },
        Ext80Rounding::Up,
        0x800b93b4c1586e51,
        6597,
    ), // raw_field15360_sample0_negative
    (
        F128 {
            hi: 0xbc007276982b0dca,
            lo: 0x3bfeb4257c383b12,
        },
        Ext80Rounding::ToZero,
        0x800b93b4c1586e51,
        6597,
    ), // raw_field15360_sample0_negative
    (
        F128 {
            hi: 0x3c007f0776cf0476,
            lo: 0x04325157afed853b,
        },
        Ext80Rounding::NearestEven,
        0x000bf83bb67823b0,
        6597,
    ), // raw_field15360_sample1_positive
    (
        F128 {
            hi: 0x3c007f0776cf0476,
            lo: 0x04325157afed853b,
        },
        Ext80Rounding::Down,
        0x000bf83bb67823b0,
        6597,
    ), // raw_field15360_sample1_positive
    (
        F128 {
            hi: 0x3c007f0776cf0476,
            lo: 0x04325157afed853b,
        },
        Ext80Rounding::Up,
        0x000bf83bb67823b1,
        6607,
    ), // raw_field15360_sample1_positive
    (
        F128 {
            hi: 0x3c007f0776cf0476,
            lo: 0x04325157afed853b,
        },
        Ext80Rounding::ToZero,
        0x000bf83bb67823b0,
        6597,
    ), // raw_field15360_sample1_positive
    (
        F128 {
            hi: 0xbc007f0776cf0476,
            lo: 0x04325157afed853b,
        },
        Ext80Rounding::NearestEven,
        0x800bf83bb67823b0,
        6597,
    ), // raw_field15360_sample1_negative
    (
        F128 {
            hi: 0xbc007f0776cf0476,
            lo: 0x04325157afed853b,
        },
        Ext80Rounding::Down,
        0x800bf83bb67823b1,
        6607,
    ), // raw_field15360_sample1_negative
    (
        F128 {
            hi: 0xbc007f0776cf0476,
            lo: 0x04325157afed853b,
        },
        Ext80Rounding::Up,
        0x800bf83bb67823b0,
        6597,
    ), // raw_field15360_sample1_negative
    (
        F128 {
            hi: 0xbc007f0776cf0476,
            lo: 0x04325157afed853b,
        },
        Ext80Rounding::ToZero,
        0x800bf83bb67823b0,
        6597,
    ), // raw_field15360_sample1_negative
    (
        F128 {
            hi: 0x3c0151256b2d6aa5,
            lo: 0x8b7b6431de21280b,
        },
        Ext80Rounding::NearestEven,
        0x00151256b2d6aa59,
        15,
    ), // raw_field15361_sample0_positive
    (
        F128 {
            hi: 0x3c0151256b2d6aa5,
            lo: 0x8b7b6431de21280b,
        },
        Ext80Rounding::Down,
        0x00151256b2d6aa58,
        5,
    ), // raw_field15361_sample0_positive
    (
        F128 {
            hi: 0x3c0151256b2d6aa5,
            lo: 0x8b7b6431de21280b,
        },
        Ext80Rounding::Up,
        0x00151256b2d6aa59,
        15,
    ), // raw_field15361_sample0_positive
    (
        F128 {
            hi: 0x3c0151256b2d6aa5,
            lo: 0x8b7b6431de21280b,
        },
        Ext80Rounding::ToZero,
        0x00151256b2d6aa58,
        5,
    ), // raw_field15361_sample0_positive
    (
        F128 {
            hi: 0xbc0151256b2d6aa5,
            lo: 0x8b7b6431de21280b,
        },
        Ext80Rounding::NearestEven,
        0x80151256b2d6aa59,
        15,
    ), // raw_field15361_sample0_negative
    (
        F128 {
            hi: 0xbc0151256b2d6aa5,
            lo: 0x8b7b6431de21280b,
        },
        Ext80Rounding::Down,
        0x80151256b2d6aa59,
        15,
    ), // raw_field15361_sample0_negative
    (
        F128 {
            hi: 0xbc0151256b2d6aa5,
            lo: 0x8b7b6431de21280b,
        },
        Ext80Rounding::Up,
        0x80151256b2d6aa58,
        5,
    ), // raw_field15361_sample0_negative
    (
        F128 {
            hi: 0xbc0151256b2d6aa5,
            lo: 0x8b7b6431de21280b,
        },
        Ext80Rounding::ToZero,
        0x80151256b2d6aa58,
        5,
    ), // raw_field15361_sample0_negative
    (
        F128 {
            hi: 0x3c01a30f728c30ac,
            lo: 0x3c488de12e8f73a4,
        },
        Ext80Rounding::NearestEven,
        0x001a30f728c30ac4,
        15,
    ), // raw_field15361_sample1_positive
    (
        F128 {
            hi: 0x3c01a30f728c30ac,
            lo: 0x3c488de12e8f73a4,
        },
        Ext80Rounding::Down,
        0x001a30f728c30ac3,
        5,
    ), // raw_field15361_sample1_positive
    (
        F128 {
            hi: 0x3c01a30f728c30ac,
            lo: 0x3c488de12e8f73a4,
        },
        Ext80Rounding::Up,
        0x001a30f728c30ac4,
        15,
    ), // raw_field15361_sample1_positive
    (
        F128 {
            hi: 0x3c01a30f728c30ac,
            lo: 0x3c488de12e8f73a4,
        },
        Ext80Rounding::ToZero,
        0x001a30f728c30ac3,
        5,
    ), // raw_field15361_sample1_positive
    (
        F128 {
            hi: 0xbc01a30f728c30ac,
            lo: 0x3c488de12e8f73a4,
        },
        Ext80Rounding::NearestEven,
        0x801a30f728c30ac4,
        15,
    ), // raw_field15361_sample1_negative
    (
        F128 {
            hi: 0xbc01a30f728c30ac,
            lo: 0x3c488de12e8f73a4,
        },
        Ext80Rounding::Down,
        0x801a30f728c30ac4,
        15,
    ), // raw_field15361_sample1_negative
    (
        F128 {
            hi: 0xbc01a30f728c30ac,
            lo: 0x3c488de12e8f73a4,
        },
        Ext80Rounding::Up,
        0x801a30f728c30ac3,
        5,
    ), // raw_field15361_sample1_negative
    (
        F128 {
            hi: 0xbc01a30f728c30ac,
            lo: 0x3c488de12e8f73a4,
        },
        Ext80Rounding::ToZero,
        0x801a30f728c30ac3,
        5,
    ), // raw_field15361_sample1_negative
    (
        F128 {
            hi: 0x3c02d1b51655bc35,
            lo: 0xe8964438498a67e8,
        },
        Ext80Rounding::NearestEven,
        0x002d1b51655bc35f,
        15,
    ), // raw_field15362_sample0_positive
    (
        F128 {
            hi: 0x3c02d1b51655bc35,
            lo: 0xe8964438498a67e8,
        },
        Ext80Rounding::Down,
        0x002d1b51655bc35e,
        5,
    ), // raw_field15362_sample0_positive
    (
        F128 {
            hi: 0x3c02d1b51655bc35,
            lo: 0xe8964438498a67e8,
        },
        Ext80Rounding::Up,
        0x002d1b51655bc35f,
        15,
    ), // raw_field15362_sample0_positive
    (
        F128 {
            hi: 0x3c02d1b51655bc35,
            lo: 0xe8964438498a67e8,
        },
        Ext80Rounding::ToZero,
        0x002d1b51655bc35e,
        5,
    ), // raw_field15362_sample0_positive
    (
        F128 {
            hi: 0xbc02d1b51655bc35,
            lo: 0xe8964438498a67e8,
        },
        Ext80Rounding::NearestEven,
        0x802d1b51655bc35f,
        15,
    ), // raw_field15362_sample0_negative
    (
        F128 {
            hi: 0xbc02d1b51655bc35,
            lo: 0xe8964438498a67e8,
        },
        Ext80Rounding::Down,
        0x802d1b51655bc35f,
        15,
    ), // raw_field15362_sample0_negative
    (
        F128 {
            hi: 0xbc02d1b51655bc35,
            lo: 0xe8964438498a67e8,
        },
        Ext80Rounding::Up,
        0x802d1b51655bc35e,
        5,
    ), // raw_field15362_sample0_negative
    (
        F128 {
            hi: 0xbc02d1b51655bc35,
            lo: 0xe8964438498a67e8,
        },
        Ext80Rounding::ToZero,
        0x802d1b51655bc35e,
        5,
    ), // raw_field15362_sample0_negative
    (
        F128 {
            hi: 0x3c0262fbb2d1cc20,
            lo: 0x739c763a3ffa4159,
        },
        Ext80Rounding::NearestEven,
        0x00262fbb2d1cc207,
        5,
    ), // raw_field15362_sample1_positive
    (
        F128 {
            hi: 0x3c0262fbb2d1cc20,
            lo: 0x739c763a3ffa4159,
        },
        Ext80Rounding::Down,
        0x00262fbb2d1cc207,
        5,
    ), // raw_field15362_sample1_positive
    (
        F128 {
            hi: 0x3c0262fbb2d1cc20,
            lo: 0x739c763a3ffa4159,
        },
        Ext80Rounding::Up,
        0x00262fbb2d1cc208,
        15,
    ), // raw_field15362_sample1_positive
    (
        F128 {
            hi: 0x3c0262fbb2d1cc20,
            lo: 0x739c763a3ffa4159,
        },
        Ext80Rounding::ToZero,
        0x00262fbb2d1cc207,
        5,
    ), // raw_field15362_sample1_positive
    (
        F128 {
            hi: 0xbc0262fbb2d1cc20,
            lo: 0x739c763a3ffa4159,
        },
        Ext80Rounding::NearestEven,
        0x80262fbb2d1cc207,
        5,
    ), // raw_field15362_sample1_negative
    (
        F128 {
            hi: 0xbc0262fbb2d1cc20,
            lo: 0x739c763a3ffa4159,
        },
        Ext80Rounding::Down,
        0x80262fbb2d1cc208,
        15,
    ), // raw_field15362_sample1_negative
    (
        F128 {
            hi: 0xbc0262fbb2d1cc20,
            lo: 0x739c763a3ffa4159,
        },
        Ext80Rounding::Up,
        0x80262fbb2d1cc207,
        5,
    ), // raw_field15362_sample1_negative
    (
        F128 {
            hi: 0xbc0262fbb2d1cc20,
            lo: 0x739c763a3ffa4159,
        },
        Ext80Rounding::ToZero,
        0x80262fbb2d1cc207,
        5,
    ), // raw_field15362_sample1_negative
    (
        F128 {
            hi: 0x3ffedb891129ec60,
            lo: 0xa9109af8fb131ae3,
        },
        Ext80Rounding::NearestEven,
        0x3fedb891129ec60b,
        15,
    ), // raw_field16382_sample0_positive
    (
        F128 {
            hi: 0x3ffedb891129ec60,
            lo: 0xa9109af8fb131ae3,
        },
        Ext80Rounding::Down,
        0x3fedb891129ec60a,
        5,
    ), // raw_field16382_sample0_positive
    (
        F128 {
            hi: 0x3ffedb891129ec60,
            lo: 0xa9109af8fb131ae3,
        },
        Ext80Rounding::Up,
        0x3fedb891129ec60b,
        15,
    ), // raw_field16382_sample0_positive
    (
        F128 {
            hi: 0x3ffedb891129ec60,
            lo: 0xa9109af8fb131ae3,
        },
        Ext80Rounding::ToZero,
        0x3fedb891129ec60a,
        5,
    ), // raw_field16382_sample0_positive
    (
        F128 {
            hi: 0xbffedb891129ec60,
            lo: 0xa9109af8fb131ae3,
        },
        Ext80Rounding::NearestEven,
        0xbfedb891129ec60b,
        15,
    ), // raw_field16382_sample0_negative
    (
        F128 {
            hi: 0xbffedb891129ec60,
            lo: 0xa9109af8fb131ae3,
        },
        Ext80Rounding::Down,
        0xbfedb891129ec60b,
        15,
    ), // raw_field16382_sample0_negative
    (
        F128 {
            hi: 0xbffedb891129ec60,
            lo: 0xa9109af8fb131ae3,
        },
        Ext80Rounding::Up,
        0xbfedb891129ec60a,
        5,
    ), // raw_field16382_sample0_negative
    (
        F128 {
            hi: 0xbffedb891129ec60,
            lo: 0xa9109af8fb131ae3,
        },
        Ext80Rounding::ToZero,
        0xbfedb891129ec60a,
        5,
    ), // raw_field16382_sample0_negative
    (
        F128 {
            hi: 0x3ffe0bfc3839c599,
            lo: 0x36b3f3116b364e10,
        },
        Ext80Rounding::NearestEven,
        0x3fe0bfc3839c5993,
        5,
    ), // raw_field16382_sample1_positive
    (
        F128 {
            hi: 0x3ffe0bfc3839c599,
            lo: 0x36b3f3116b364e10,
        },
        Ext80Rounding::Down,
        0x3fe0bfc3839c5993,
        5,
    ), // raw_field16382_sample1_positive
    (
        F128 {
            hi: 0x3ffe0bfc3839c599,
            lo: 0x36b3f3116b364e10,
        },
        Ext80Rounding::Up,
        0x3fe0bfc3839c5994,
        15,
    ), // raw_field16382_sample1_positive
    (
        F128 {
            hi: 0x3ffe0bfc3839c599,
            lo: 0x36b3f3116b364e10,
        },
        Ext80Rounding::ToZero,
        0x3fe0bfc3839c5993,
        5,
    ), // raw_field16382_sample1_positive
    (
        F128 {
            hi: 0xbffe0bfc3839c599,
            lo: 0x36b3f3116b364e10,
        },
        Ext80Rounding::NearestEven,
        0xbfe0bfc3839c5993,
        5,
    ), // raw_field16382_sample1_negative
    (
        F128 {
            hi: 0xbffe0bfc3839c599,
            lo: 0x36b3f3116b364e10,
        },
        Ext80Rounding::Down,
        0xbfe0bfc3839c5994,
        15,
    ), // raw_field16382_sample1_negative
    (
        F128 {
            hi: 0xbffe0bfc3839c599,
            lo: 0x36b3f3116b364e10,
        },
        Ext80Rounding::Up,
        0xbfe0bfc3839c5993,
        5,
    ), // raw_field16382_sample1_negative
    (
        F128 {
            hi: 0xbffe0bfc3839c599,
            lo: 0x36b3f3116b364e10,
        },
        Ext80Rounding::ToZero,
        0xbfe0bfc3839c5993,
        5,
    ), // raw_field16382_sample1_negative
    (
        F128 {
            hi: 0x3fff71097b5c3491,
            lo: 0xbf186524fd7fd54c,
        },
        Ext80Rounding::NearestEven,
        0x3ff71097b5c3491c,
        15,
    ), // raw_field16383_sample0_positive
    (
        F128 {
            hi: 0x3fff71097b5c3491,
            lo: 0xbf186524fd7fd54c,
        },
        Ext80Rounding::Down,
        0x3ff71097b5c3491b,
        5,
    ), // raw_field16383_sample0_positive
    (
        F128 {
            hi: 0x3fff71097b5c3491,
            lo: 0xbf186524fd7fd54c,
        },
        Ext80Rounding::Up,
        0x3ff71097b5c3491c,
        15,
    ), // raw_field16383_sample0_positive
    (
        F128 {
            hi: 0x3fff71097b5c3491,
            lo: 0xbf186524fd7fd54c,
        },
        Ext80Rounding::ToZero,
        0x3ff71097b5c3491b,
        5,
    ), // raw_field16383_sample0_positive
    (
        F128 {
            hi: 0xbfff71097b5c3491,
            lo: 0xbf186524fd7fd54c,
        },
        Ext80Rounding::NearestEven,
        0xbff71097b5c3491c,
        15,
    ), // raw_field16383_sample0_negative
    (
        F128 {
            hi: 0xbfff71097b5c3491,
            lo: 0xbf186524fd7fd54c,
        },
        Ext80Rounding::Down,
        0xbff71097b5c3491c,
        15,
    ), // raw_field16383_sample0_negative
    (
        F128 {
            hi: 0xbfff71097b5c3491,
            lo: 0xbf186524fd7fd54c,
        },
        Ext80Rounding::Up,
        0xbff71097b5c3491b,
        5,
    ), // raw_field16383_sample0_negative
    (
        F128 {
            hi: 0xbfff71097b5c3491,
            lo: 0xbf186524fd7fd54c,
        },
        Ext80Rounding::ToZero,
        0xbff71097b5c3491b,
        5,
    ), // raw_field16383_sample0_negative
    (
        F128 {
            hi: 0x3fff4b5b46dc2fec,
            lo: 0x66437a63fd2c11d8,
        },
        Ext80Rounding::NearestEven,
        0x3ff4b5b46dc2fec6,
        5,
    ), // raw_field16383_sample1_positive
    (
        F128 {
            hi: 0x3fff4b5b46dc2fec,
            lo: 0x66437a63fd2c11d8,
        },
        Ext80Rounding::Down,
        0x3ff4b5b46dc2fec6,
        5,
    ), // raw_field16383_sample1_positive
    (
        F128 {
            hi: 0x3fff4b5b46dc2fec,
            lo: 0x66437a63fd2c11d8,
        },
        Ext80Rounding::Up,
        0x3ff4b5b46dc2fec7,
        15,
    ), // raw_field16383_sample1_positive
    (
        F128 {
            hi: 0x3fff4b5b46dc2fec,
            lo: 0x66437a63fd2c11d8,
        },
        Ext80Rounding::ToZero,
        0x3ff4b5b46dc2fec6,
        5,
    ), // raw_field16383_sample1_positive
    (
        F128 {
            hi: 0xbfff4b5b46dc2fec,
            lo: 0x66437a63fd2c11d8,
        },
        Ext80Rounding::NearestEven,
        0xbff4b5b46dc2fec6,
        5,
    ), // raw_field16383_sample1_negative
    (
        F128 {
            hi: 0xbfff4b5b46dc2fec,
            lo: 0x66437a63fd2c11d8,
        },
        Ext80Rounding::Down,
        0xbff4b5b46dc2fec7,
        15,
    ), // raw_field16383_sample1_negative
    (
        F128 {
            hi: 0xbfff4b5b46dc2fec,
            lo: 0x66437a63fd2c11d8,
        },
        Ext80Rounding::Up,
        0xbff4b5b46dc2fec6,
        5,
    ), // raw_field16383_sample1_negative
    (
        F128 {
            hi: 0xbfff4b5b46dc2fec,
            lo: 0x66437a63fd2c11d8,
        },
        Ext80Rounding::ToZero,
        0xbff4b5b46dc2fec6,
        5,
    ), // raw_field16383_sample1_negative
    (
        F128 {
            hi: 0x4000f3d251aaf5e2,
            lo: 0xa3c1ccc0eb70c8c9,
        },
        Ext80Rounding::NearestEven,
        0x400f3d251aaf5e2a,
        5,
    ), // raw_field16384_sample0_positive
    (
        F128 {
            hi: 0x4000f3d251aaf5e2,
            lo: 0xa3c1ccc0eb70c8c9,
        },
        Ext80Rounding::Down,
        0x400f3d251aaf5e2a,
        5,
    ), // raw_field16384_sample0_positive
    (
        F128 {
            hi: 0x4000f3d251aaf5e2,
            lo: 0xa3c1ccc0eb70c8c9,
        },
        Ext80Rounding::Up,
        0x400f3d251aaf5e2b,
        15,
    ), // raw_field16384_sample0_positive
    (
        F128 {
            hi: 0x4000f3d251aaf5e2,
            lo: 0xa3c1ccc0eb70c8c9,
        },
        Ext80Rounding::ToZero,
        0x400f3d251aaf5e2a,
        5,
    ), // raw_field16384_sample0_positive
    (
        F128 {
            hi: 0xc000f3d251aaf5e2,
            lo: 0xa3c1ccc0eb70c8c9,
        },
        Ext80Rounding::NearestEven,
        0xc00f3d251aaf5e2a,
        5,
    ), // raw_field16384_sample0_negative
    (
        F128 {
            hi: 0xc000f3d251aaf5e2,
            lo: 0xa3c1ccc0eb70c8c9,
        },
        Ext80Rounding::Down,
        0xc00f3d251aaf5e2b,
        15,
    ), // raw_field16384_sample0_negative
    (
        F128 {
            hi: 0xc000f3d251aaf5e2,
            lo: 0xa3c1ccc0eb70c8c9,
        },
        Ext80Rounding::Up,
        0xc00f3d251aaf5e2a,
        5,
    ), // raw_field16384_sample0_negative
    (
        F128 {
            hi: 0xc000f3d251aaf5e2,
            lo: 0xa3c1ccc0eb70c8c9,
        },
        Ext80Rounding::ToZero,
        0xc00f3d251aaf5e2a,
        5,
    ), // raw_field16384_sample0_negative
    (
        F128 {
            hi: 0x40006c99b14e3a8f,
            lo: 0xf4ad5641bdaba0c3,
        },
        Ext80Rounding::NearestEven,
        0x4006c99b14e3a8ff,
        5,
    ), // raw_field16384_sample1_positive
    (
        F128 {
            hi: 0x40006c99b14e3a8f,
            lo: 0xf4ad5641bdaba0c3,
        },
        Ext80Rounding::Down,
        0x4006c99b14e3a8ff,
        5,
    ), // raw_field16384_sample1_positive
    (
        F128 {
            hi: 0x40006c99b14e3a8f,
            lo: 0xf4ad5641bdaba0c3,
        },
        Ext80Rounding::Up,
        0x4006c99b14e3a900,
        15,
    ), // raw_field16384_sample1_positive
    (
        F128 {
            hi: 0x40006c99b14e3a8f,
            lo: 0xf4ad5641bdaba0c3,
        },
        Ext80Rounding::ToZero,
        0x4006c99b14e3a8ff,
        5,
    ), // raw_field16384_sample1_positive
    (
        F128 {
            hi: 0xc0006c99b14e3a8f,
            lo: 0xf4ad5641bdaba0c3,
        },
        Ext80Rounding::NearestEven,
        0xc006c99b14e3a8ff,
        5,
    ), // raw_field16384_sample1_negative
    (
        F128 {
            hi: 0xc0006c99b14e3a8f,
            lo: 0xf4ad5641bdaba0c3,
        },
        Ext80Rounding::Down,
        0xc006c99b14e3a900,
        15,
    ), // raw_field16384_sample1_negative
    (
        F128 {
            hi: 0xc0006c99b14e3a8f,
            lo: 0xf4ad5641bdaba0c3,
        },
        Ext80Rounding::Up,
        0xc006c99b14e3a8ff,
        5,
    ), // raw_field16384_sample1_negative
    (
        F128 {
            hi: 0xc0006c99b14e3a8f,
            lo: 0xf4ad5641bdaba0c3,
        },
        Ext80Rounding::ToZero,
        0xc006c99b14e3a8ff,
        5,
    ), // raw_field16384_sample1_negative
    (
        F128 {
            hi: 0x43fe5cd93a969d5f,
            lo: 0xc57c50c2260f402f,
        },
        Ext80Rounding::NearestEven,
        0x7fe5cd93a969d5fc,
        5,
    ), // raw_field17406_sample0_positive
    (
        F128 {
            hi: 0x43fe5cd93a969d5f,
            lo: 0xc57c50c2260f402f,
        },
        Ext80Rounding::Down,
        0x7fe5cd93a969d5fc,
        5,
    ), // raw_field17406_sample0_positive
    (
        F128 {
            hi: 0x43fe5cd93a969d5f,
            lo: 0xc57c50c2260f402f,
        },
        Ext80Rounding::Up,
        0x7fe5cd93a969d5fd,
        15,
    ), // raw_field17406_sample0_positive
    (
        F128 {
            hi: 0x43fe5cd93a969d5f,
            lo: 0xc57c50c2260f402f,
        },
        Ext80Rounding::ToZero,
        0x7fe5cd93a969d5fc,
        5,
    ), // raw_field17406_sample0_positive
    (
        F128 {
            hi: 0xc3fe5cd93a969d5f,
            lo: 0xc57c50c2260f402f,
        },
        Ext80Rounding::NearestEven,
        0xffe5cd93a969d5fc,
        5,
    ), // raw_field17406_sample0_negative
    (
        F128 {
            hi: 0xc3fe5cd93a969d5f,
            lo: 0xc57c50c2260f402f,
        },
        Ext80Rounding::Down,
        0xffe5cd93a969d5fd,
        15,
    ), // raw_field17406_sample0_negative
    (
        F128 {
            hi: 0xc3fe5cd93a969d5f,
            lo: 0xc57c50c2260f402f,
        },
        Ext80Rounding::Up,
        0xffe5cd93a969d5fc,
        5,
    ), // raw_field17406_sample0_negative
    (
        F128 {
            hi: 0xc3fe5cd93a969d5f,
            lo: 0xc57c50c2260f402f,
        },
        Ext80Rounding::ToZero,
        0xffe5cd93a969d5fc,
        5,
    ), // raw_field17406_sample0_negative
    (
        F128 {
            hi: 0x43fe5cedcfb02f78,
            lo: 0xa11761b680864b9d,
        },
        Ext80Rounding::NearestEven,
        0x7fe5cedcfb02f78a,
        5,
    ), // raw_field17406_sample1_positive
    (
        F128 {
            hi: 0x43fe5cedcfb02f78,
            lo: 0xa11761b680864b9d,
        },
        Ext80Rounding::Down,
        0x7fe5cedcfb02f78a,
        5,
    ), // raw_field17406_sample1_positive
    (
        F128 {
            hi: 0x43fe5cedcfb02f78,
            lo: 0xa11761b680864b9d,
        },
        Ext80Rounding::Up,
        0x7fe5cedcfb02f78b,
        15,
    ), // raw_field17406_sample1_positive
    (
        F128 {
            hi: 0x43fe5cedcfb02f78,
            lo: 0xa11761b680864b9d,
        },
        Ext80Rounding::ToZero,
        0x7fe5cedcfb02f78a,
        5,
    ), // raw_field17406_sample1_positive
    (
        F128 {
            hi: 0xc3fe5cedcfb02f78,
            lo: 0xa11761b680864b9d,
        },
        Ext80Rounding::NearestEven,
        0xffe5cedcfb02f78a,
        5,
    ), // raw_field17406_sample1_negative
    (
        F128 {
            hi: 0xc3fe5cedcfb02f78,
            lo: 0xa11761b680864b9d,
        },
        Ext80Rounding::Down,
        0xffe5cedcfb02f78b,
        15,
    ), // raw_field17406_sample1_negative
    (
        F128 {
            hi: 0xc3fe5cedcfb02f78,
            lo: 0xa11761b680864b9d,
        },
        Ext80Rounding::Up,
        0xffe5cedcfb02f78a,
        5,
    ), // raw_field17406_sample1_negative
    (
        F128 {
            hi: 0xc3fe5cedcfb02f78,
            lo: 0xa11761b680864b9d,
        },
        Ext80Rounding::ToZero,
        0xffe5cedcfb02f78a,
        5,
    ), // raw_field17406_sample1_negative
    (
        F128 {
            hi: 0x43ffaf8881357212,
            lo: 0xda0429e8038279c6,
        },
        Ext80Rounding::NearestEven,
        0x7ff0000000000000,
        63,
    ), // raw_field17407_sample0_positive
    (
        F128 {
            hi: 0x43ffaf8881357212,
            lo: 0xda0429e8038279c6,
        },
        Ext80Rounding::Down,
        0x7fefffffffffffff,
        21,
    ), // raw_field17407_sample0_positive
    (
        F128 {
            hi: 0x43ffaf8881357212,
            lo: 0xda0429e8038279c6,
        },
        Ext80Rounding::Up,
        0x7ff0000000000000,
        63,
    ), // raw_field17407_sample0_positive
    (
        F128 {
            hi: 0x43ffaf8881357212,
            lo: 0xda0429e8038279c6,
        },
        Ext80Rounding::ToZero,
        0x7fefffffffffffff,
        21,
    ), // raw_field17407_sample0_positive
    (
        F128 {
            hi: 0xc3ffaf8881357212,
            lo: 0xda0429e8038279c6,
        },
        Ext80Rounding::NearestEven,
        0xfff0000000000000,
        63,
    ), // raw_field17407_sample0_negative
    (
        F128 {
            hi: 0xc3ffaf8881357212,
            lo: 0xda0429e8038279c6,
        },
        Ext80Rounding::Down,
        0xfff0000000000000,
        63,
    ), // raw_field17407_sample0_negative
    (
        F128 {
            hi: 0xc3ffaf8881357212,
            lo: 0xda0429e8038279c6,
        },
        Ext80Rounding::Up,
        0xffefffffffffffff,
        21,
    ), // raw_field17407_sample0_negative
    (
        F128 {
            hi: 0xc3ffaf8881357212,
            lo: 0xda0429e8038279c6,
        },
        Ext80Rounding::ToZero,
        0xffefffffffffffff,
        21,
    ), // raw_field17407_sample0_negative
    (
        F128 {
            hi: 0x43ff8659e7622400,
            lo: 0x4384ed09949a4753,
        },
        Ext80Rounding::NearestEven,
        0x7ff0000000000000,
        55,
    ), // raw_field17407_sample1_positive
    (
        F128 {
            hi: 0x43ff8659e7622400,
            lo: 0x4384ed09949a4753,
        },
        Ext80Rounding::Down,
        0x7fefffffffffffff,
        21,
    ), // raw_field17407_sample1_positive
    (
        F128 {
            hi: 0x43ff8659e7622400,
            lo: 0x4384ed09949a4753,
        },
        Ext80Rounding::Up,
        0x7ff0000000000000,
        63,
    ), // raw_field17407_sample1_positive
    (
        F128 {
            hi: 0x43ff8659e7622400,
            lo: 0x4384ed09949a4753,
        },
        Ext80Rounding::ToZero,
        0x7fefffffffffffff,
        21,
    ), // raw_field17407_sample1_positive
    (
        F128 {
            hi: 0xc3ff8659e7622400,
            lo: 0x4384ed09949a4753,
        },
        Ext80Rounding::NearestEven,
        0xfff0000000000000,
        55,
    ), // raw_field17407_sample1_negative
    (
        F128 {
            hi: 0xc3ff8659e7622400,
            lo: 0x4384ed09949a4753,
        },
        Ext80Rounding::Down,
        0xfff0000000000000,
        63,
    ), // raw_field17407_sample1_negative
    (
        F128 {
            hi: 0xc3ff8659e7622400,
            lo: 0x4384ed09949a4753,
        },
        Ext80Rounding::Up,
        0xffefffffffffffff,
        21,
    ), // raw_field17407_sample1_negative
    (
        F128 {
            hi: 0xc3ff8659e7622400,
            lo: 0x4384ed09949a4753,
        },
        Ext80Rounding::ToZero,
        0xffefffffffffffff,
        21,
    ), // raw_field17407_sample1_negative
    (
        F128 {
            hi: 0x44005c0e5a309fa8,
            lo: 0x97ea60ba9f6004fc,
        },
        Ext80Rounding::NearestEven,
        0x7ff0000000000000,
        55,
    ), // raw_field17408_sample0_positive
    (
        F128 {
            hi: 0x44005c0e5a309fa8,
            lo: 0x97ea60ba9f6004fc,
        },
        Ext80Rounding::Down,
        0x7fefffffffffffff,
        21,
    ), // raw_field17408_sample0_positive
    (
        F128 {
            hi: 0x44005c0e5a309fa8,
            lo: 0x97ea60ba9f6004fc,
        },
        Ext80Rounding::Up,
        0x7ff0000000000000,
        63,
    ), // raw_field17408_sample0_positive
    (
        F128 {
            hi: 0x44005c0e5a309fa8,
            lo: 0x97ea60ba9f6004fc,
        },
        Ext80Rounding::ToZero,
        0x7fefffffffffffff,
        21,
    ), // raw_field17408_sample0_positive
    (
        F128 {
            hi: 0xc4005c0e5a309fa8,
            lo: 0x97ea60ba9f6004fc,
        },
        Ext80Rounding::NearestEven,
        0xfff0000000000000,
        55,
    ), // raw_field17408_sample0_negative
    (
        F128 {
            hi: 0xc4005c0e5a309fa8,
            lo: 0x97ea60ba9f6004fc,
        },
        Ext80Rounding::Down,
        0xfff0000000000000,
        63,
    ), // raw_field17408_sample0_negative
    (
        F128 {
            hi: 0xc4005c0e5a309fa8,
            lo: 0x97ea60ba9f6004fc,
        },
        Ext80Rounding::Up,
        0xffefffffffffffff,
        21,
    ), // raw_field17408_sample0_negative
    (
        F128 {
            hi: 0xc4005c0e5a309fa8,
            lo: 0x97ea60ba9f6004fc,
        },
        Ext80Rounding::ToZero,
        0xffefffffffffffff,
        21,
    ), // raw_field17408_sample0_negative
    (
        F128 {
            hi: 0x440086258652016d,
            lo: 0x7973ae1bc54ebc87,
        },
        Ext80Rounding::NearestEven,
        0x7ff0000000000000,
        63,
    ), // raw_field17408_sample1_positive
    (
        F128 {
            hi: 0x440086258652016d,
            lo: 0x7973ae1bc54ebc87,
        },
        Ext80Rounding::Down,
        0x7fefffffffffffff,
        21,
    ), // raw_field17408_sample1_positive
    (
        F128 {
            hi: 0x440086258652016d,
            lo: 0x7973ae1bc54ebc87,
        },
        Ext80Rounding::Up,
        0x7ff0000000000000,
        63,
    ), // raw_field17408_sample1_positive
    (
        F128 {
            hi: 0x440086258652016d,
            lo: 0x7973ae1bc54ebc87,
        },
        Ext80Rounding::ToZero,
        0x7fefffffffffffff,
        21,
    ), // raw_field17408_sample1_positive
    (
        F128 {
            hi: 0xc40086258652016d,
            lo: 0x7973ae1bc54ebc87,
        },
        Ext80Rounding::NearestEven,
        0xfff0000000000000,
        63,
    ), // raw_field17408_sample1_negative
    (
        F128 {
            hi: 0xc40086258652016d,
            lo: 0x7973ae1bc54ebc87,
        },
        Ext80Rounding::Down,
        0xfff0000000000000,
        63,
    ), // raw_field17408_sample1_negative
    (
        F128 {
            hi: 0xc40086258652016d,
            lo: 0x7973ae1bc54ebc87,
        },
        Ext80Rounding::Up,
        0xffefffffffffffff,
        21,
    ), // raw_field17408_sample1_negative
    (
        F128 {
            hi: 0xc40086258652016d,
            lo: 0x7973ae1bc54ebc87,
        },
        Ext80Rounding::ToZero,
        0xffefffffffffffff,
        21,
    ), // raw_field17408_sample1_negative
    (
        F128 {
            hi: 0x7ffe308d70a4eedf,
            lo: 0x2978b316a0c9d46f,
        },
        Ext80Rounding::NearestEven,
        0x7ff0000000000000,
        63,
    ), // raw_field32766_sample0_positive
    (
        F128 {
            hi: 0x7ffe308d70a4eedf,
            lo: 0x2978b316a0c9d46f,
        },
        Ext80Rounding::Down,
        0x7fefffffffffffff,
        21,
    ), // raw_field32766_sample0_positive
    (
        F128 {
            hi: 0x7ffe308d70a4eedf,
            lo: 0x2978b316a0c9d46f,
        },
        Ext80Rounding::Up,
        0x7ff0000000000000,
        63,
    ), // raw_field32766_sample0_positive
    (
        F128 {
            hi: 0x7ffe308d70a4eedf,
            lo: 0x2978b316a0c9d46f,
        },
        Ext80Rounding::ToZero,
        0x7fefffffffffffff,
        21,
    ), // raw_field32766_sample0_positive
    (
        F128 {
            hi: 0xfffe308d70a4eedf,
            lo: 0x2978b316a0c9d46f,
        },
        Ext80Rounding::NearestEven,
        0xfff0000000000000,
        63,
    ), // raw_field32766_sample0_negative
    (
        F128 {
            hi: 0xfffe308d70a4eedf,
            lo: 0x2978b316a0c9d46f,
        },
        Ext80Rounding::Down,
        0xfff0000000000000,
        63,
    ), // raw_field32766_sample0_negative
    (
        F128 {
            hi: 0xfffe308d70a4eedf,
            lo: 0x2978b316a0c9d46f,
        },
        Ext80Rounding::Up,
        0xffefffffffffffff,
        21,
    ), // raw_field32766_sample0_negative
    (
        F128 {
            hi: 0xfffe308d70a4eedf,
            lo: 0x2978b316a0c9d46f,
        },
        Ext80Rounding::ToZero,
        0xffefffffffffffff,
        21,
    ), // raw_field32766_sample0_negative
    (
        F128 {
            hi: 0x7ffebf3c6222a3b8,
            lo: 0xb7c3fd3c896ca5ad,
        },
        Ext80Rounding::NearestEven,
        0x7ff0000000000000,
        55,
    ), // raw_field32766_sample1_positive
    (
        F128 {
            hi: 0x7ffebf3c6222a3b8,
            lo: 0xb7c3fd3c896ca5ad,
        },
        Ext80Rounding::Down,
        0x7fefffffffffffff,
        21,
    ), // raw_field32766_sample1_positive
    (
        F128 {
            hi: 0x7ffebf3c6222a3b8,
            lo: 0xb7c3fd3c896ca5ad,
        },
        Ext80Rounding::Up,
        0x7ff0000000000000,
        63,
    ), // raw_field32766_sample1_positive
    (
        F128 {
            hi: 0x7ffebf3c6222a3b8,
            lo: 0xb7c3fd3c896ca5ad,
        },
        Ext80Rounding::ToZero,
        0x7fefffffffffffff,
        21,
    ), // raw_field32766_sample1_positive
    (
        F128 {
            hi: 0xfffebf3c6222a3b8,
            lo: 0xb7c3fd3c896ca5ad,
        },
        Ext80Rounding::NearestEven,
        0xfff0000000000000,
        55,
    ), // raw_field32766_sample1_negative
    (
        F128 {
            hi: 0xfffebf3c6222a3b8,
            lo: 0xb7c3fd3c896ca5ad,
        },
        Ext80Rounding::Down,
        0xfff0000000000000,
        63,
    ), // raw_field32766_sample1_negative
    (
        F128 {
            hi: 0xfffebf3c6222a3b8,
            lo: 0xb7c3fd3c896ca5ad,
        },
        Ext80Rounding::Up,
        0xffefffffffffffff,
        21,
    ), // raw_field32766_sample1_negative
    (
        F128 {
            hi: 0xfffebf3c6222a3b8,
            lo: 0xb7c3fd3c896ca5ad,
        },
        Ext80Rounding::ToZero,
        0xffefffffffffffff,
        21,
    ), // raw_field32766_sample1_negative
];
