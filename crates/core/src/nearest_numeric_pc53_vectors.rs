// Expected literals transcribed from independently authored Fraction oracle.
// vectors.json SHA256 2f56d88ba4209919729473c723fa48e67987809c64c017bda1f2e7b9801d625c.
use super::*;
#[test]
fn pc53_and_unchanged_pc64_match_independent_fraction_intermediate_vectors() {
    let rows: &[(&str, [u64; 4], [[f::Ext80; 5]; 2], [u64; 2], [bool; 2])] = &[
        (
            "origin",
            [
                0x0000000000000000,
                0x0000000000000000,
                0x0000000000000000,
                0x0000000000000000,
            ],
            [
                [
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                ],
                [
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                ],
            ],
            [0x0000000000000000, 0x0000000000000000],
            [true, true],
        ),
        (
            "negative_zero_dx",
            [
                0x8000000000000000,
                0x0000000000000000,
                0x0000000000000000,
                0x0000000000000000,
            ],
            [
                [
                    f::Ext80 {
                        sign_exp: 0x8000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                ],
                [
                    f::Ext80 {
                        sign_exp: 0x8000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                ],
            ],
            [0x0000000000000000, 0x0000000000000000],
            [true, true],
        ),
        (
            "negative_zero_query",
            [
                0x0000000000000000,
                0x0000000000000000,
                0x8000000000000000,
                0x0000000000000000,
            ],
            [
                [
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                ],
                [
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                ],
            ],
            [0x0000000000000000, 0x0000000000000000],
            [true, true],
        ),
        (
            "three_four",
            [
                0x4008000000000000,
                0x4010000000000000,
                0x0000000000000000,
                0x0000000000000000,
            ],
            [
                [
                    f::Ext80 {
                        sign_exp: 0x4000,
                        significand: 0xc000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x4002,
                        significand: 0x9000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x4001,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x4003,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x4003,
                        significand: 0xc800000000000000,
                    },
                ],
                [
                    f::Ext80 {
                        sign_exp: 0x4000,
                        significand: 0xc000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x4002,
                        significand: 0x9000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x4001,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x4003,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x4003,
                        significand: 0xc800000000000000,
                    },
                ],
            ],
            [0x4039000000000000, 0x4039000000000000],
            [true, true],
        ),
        (
            "negative_three_four",
            [
                0xc008000000000000,
                0xc010000000000000,
                0x0000000000000000,
                0x0000000000000000,
            ],
            [
                [
                    f::Ext80 {
                        sign_exp: 0xc000,
                        significand: 0xc000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x4002,
                        significand: 0x9000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0xc001,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x4003,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x4003,
                        significand: 0xc800000000000000,
                    },
                ],
                [
                    f::Ext80 {
                        sign_exp: 0xc000,
                        significand: 0xc000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x4002,
                        significand: 0x9000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0xc001,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x4003,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x4003,
                        significand: 0xc800000000000000,
                    },
                ],
            ],
            [0x4039000000000000, 0x4039000000000000],
            [true, true],
        ),
        (
            "translated_three_four",
            [
                0x4059c00000000000,
                0xc058000000000000,
                0x4059000000000000,
                0xc059000000000000,
            ],
            [
                [
                    f::Ext80 {
                        sign_exp: 0x4000,
                        significand: 0xc000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x4002,
                        significand: 0x9000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x4001,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x4003,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x4003,
                        significand: 0xc800000000000000,
                    },
                ],
                [
                    f::Ext80 {
                        sign_exp: 0x4000,
                        significand: 0xc000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x4002,
                        significand: 0x9000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x4001,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x4003,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x4003,
                        significand: 0xc800000000000000,
                    },
                ],
            ],
            [0x4039000000000000, 0x4039000000000000],
            [true, true],
        ),
        (
            "decimal_point_one",
            [
                0x3fb999999999999a,
                0x0000000000000000,
                0x0000000000000000,
                0x0000000000000000,
            ],
            [
                [
                    f::Ext80 {
                        sign_exp: 0x3ffb,
                        significand: 0xccccccccccccd000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3ff8,
                        significand: 0xa3d70a3d70a3e000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3ff8,
                        significand: 0xa3d70a3d70a3e000,
                    },
                ],
                [
                    f::Ext80 {
                        sign_exp: 0x3ffb,
                        significand: 0xccccccccccccd000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3ff8,
                        significand: 0xa3d70a3d70a3dc29,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3ff8,
                        significand: 0xa3d70a3d70a3dc29,
                    },
                ],
            ],
            [0x3f847ae147ae147c, 0x3f847ae147ae147c],
            [true, true],
        ),
        (
            "decimal_point_one_both_axes",
            [
                0x3fb999999999999a,
                0x3fb999999999999a,
                0x0000000000000000,
                0x0000000000000000,
            ],
            [
                [
                    f::Ext80 {
                        sign_exp: 0x3ffb,
                        significand: 0xccccccccccccd000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3ff8,
                        significand: 0xa3d70a3d70a3e000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3ffb,
                        significand: 0xccccccccccccd000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3ff8,
                        significand: 0xa3d70a3d70a3e000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3ff9,
                        significand: 0xa3d70a3d70a3e000,
                    },
                ],
                [
                    f::Ext80 {
                        sign_exp: 0x3ffb,
                        significand: 0xccccccccccccd000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3ff8,
                        significand: 0xa3d70a3d70a3dc29,
                    },
                    f::Ext80 {
                        sign_exp: 0x3ffb,
                        significand: 0xccccccccccccd000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3ff8,
                        significand: 0xa3d70a3d70a3dc29,
                    },
                    f::Ext80 {
                        sign_exp: 0x3ff9,
                        significand: 0xa3d70a3d70a3dc29,
                    },
                ],
            ],
            [0x3f947ae147ae147c, 0x3f947ae147ae147c],
            [true, true],
        ),
        (
            "minimum_subnormal",
            [
                0x0000000000000001,
                0x0000000000000000,
                0x0000000000000000,
                0x0000000000000000,
            ],
            [
                [
                    f::Ext80 {
                        sign_exp: 0x3bcd,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x379b,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x379b,
                        significand: 0x8000000000000000,
                    },
                ],
                [
                    f::Ext80 {
                        sign_exp: 0x3bcd,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x379b,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x379b,
                        significand: 0x8000000000000000,
                    },
                ],
            ],
            [0x0000000000000000, 0x0000000000000000],
            [true, true],
        ),
        (
            "negative_minimum_subnormal",
            [
                0x8000000000000001,
                0x0000000000000000,
                0x0000000000000000,
                0x0000000000000000,
            ],
            [
                [
                    f::Ext80 {
                        sign_exp: 0xbbcd,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x379b,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x379b,
                        significand: 0x8000000000000000,
                    },
                ],
                [
                    f::Ext80 {
                        sign_exp: 0xbbcd,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x379b,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x379b,
                        significand: 0x8000000000000000,
                    },
                ],
            ],
            [0x0000000000000000, 0x0000000000000000],
            [true, true],
        ),
        (
            "two_minimum_subnormal",
            [
                0x0000000000000002,
                0x0000000000000001,
                0x0000000000000000,
                0x0000000000000000,
            ],
            [
                [
                    f::Ext80 {
                        sign_exp: 0x3bce,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x379d,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3bcd,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x379b,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x379d,
                        significand: 0xa000000000000000,
                    },
                ],
                [
                    f::Ext80 {
                        sign_exp: 0x3bce,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x379d,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3bcd,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x379b,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x379d,
                        significand: 0xa000000000000000,
                    },
                ],
            ],
            [0x0000000000000000, 0x0000000000000000],
            [true, true],
        ),
        (
            "largest_subnormal",
            [
                0x000fffffffffffff,
                0x0000000000000000,
                0x0000000000000000,
                0x0000000000000000,
            ],
            [
                [
                    f::Ext80 {
                        sign_exp: 0x3c00,
                        significand: 0xfffffffffffff000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3802,
                        significand: 0xffffffffffffe000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3802,
                        significand: 0xffffffffffffe000,
                    },
                ],
                [
                    f::Ext80 {
                        sign_exp: 0x3c00,
                        significand: 0xfffffffffffff000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3802,
                        significand: 0xffffffffffffe000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3802,
                        significand: 0xffffffffffffe000,
                    },
                ],
            ],
            [0x0000000000000000, 0x0000000000000000],
            [true, true],
        ),
        (
            "minimum_normal",
            [
                0x0010000000000000,
                0x0000000000000000,
                0x0000000000000000,
                0x0000000000000000,
            ],
            [
                [
                    f::Ext80 {
                        sign_exp: 0x3c01,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3803,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3803,
                        significand: 0x8000000000000000,
                    },
                ],
                [
                    f::Ext80 {
                        sign_exp: 0x3c01,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3803,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3803,
                        significand: 0x8000000000000000,
                    },
                ],
            ],
            [0x0000000000000000, 0x0000000000000000],
            [true, true],
        ),
        (
            "minimum_normal_gap",
            [
                0x0010000000000000,
                0x0000000000000000,
                0x000fffffffffffff,
                0x0000000000000000,
            ],
            [
                [
                    f::Ext80 {
                        sign_exp: 0x3bcd,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x379b,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x379b,
                        significand: 0x8000000000000000,
                    },
                ],
                [
                    f::Ext80 {
                        sign_exp: 0x3bcd,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x379b,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x379b,
                        significand: 0x8000000000000000,
                    },
                ],
            ],
            [0x0000000000000000, 0x0000000000000000],
            [true, true],
        ),
        (
            "store_exact_minimum_subnormal",
            [
                0x1e60000000000000,
                0x0000000000000000,
                0x0000000000000000,
                0x0000000000000000,
            ],
            [
                [
                    f::Ext80 {
                        sign_exp: 0x3de6,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3bcd,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3bcd,
                        significand: 0x8000000000000000,
                    },
                ],
                [
                    f::Ext80 {
                        sign_exp: 0x3de6,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3bcd,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3bcd,
                        significand: 0x8000000000000000,
                    },
                ],
            ],
            [0x0000000000000001, 0x0000000000000001],
            [true, true],
        ),
        (
            "store_underflow_quarter_quantum",
            [
                0x1e50000000000000,
                0x0000000000000000,
                0x0000000000000000,
                0x0000000000000000,
            ],
            [
                [
                    f::Ext80 {
                        sign_exp: 0x3de5,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3bcb,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3bcb,
                        significand: 0x8000000000000000,
                    },
                ],
                [
                    f::Ext80 {
                        sign_exp: 0x3de5,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3bcb,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3bcb,
                        significand: 0x8000000000000000,
                    },
                ],
            ],
            [0x0000000000000000, 0x0000000000000000],
            [true, true],
        ),
        (
            "store_underflow_half_quantum_even",
            [
                0x1e50000000000000,
                0x1e50000000000000,
                0x0000000000000000,
                0x0000000000000000,
            ],
            [
                [
                    f::Ext80 {
                        sign_exp: 0x3de5,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3bcb,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3de5,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3bcb,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3bcc,
                        significand: 0x8000000000000000,
                    },
                ],
                [
                    f::Ext80 {
                        sign_exp: 0x3de5,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3bcb,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3de5,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3bcb,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3bcc,
                        significand: 0x8000000000000000,
                    },
                ],
            ],
            [0x0000000000000000, 0x0000000000000000],
            [true, true],
        ),
        (
            "store_one_and_quarter_quantum",
            [
                0x1e60000000000000,
                0x1e50000000000000,
                0x0000000000000000,
                0x0000000000000000,
            ],
            [
                [
                    f::Ext80 {
                        sign_exp: 0x3de6,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3bcd,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3de5,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3bcb,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3bcd,
                        significand: 0xa000000000000000,
                    },
                ],
                [
                    f::Ext80 {
                        sign_exp: 0x3de6,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3bcd,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3de5,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3bcb,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3bcd,
                        significand: 0xa000000000000000,
                    },
                ],
            ],
            [0x0000000000000001, 0x0000000000000001],
            [true, true],
        ),
        (
            "store_two_and_quarter_quantum",
            [
                0x1e68000000000000,
                0x0000000000000000,
                0x0000000000000000,
                0x0000000000000000,
            ],
            [
                [
                    f::Ext80 {
                        sign_exp: 0x3de6,
                        significand: 0xc000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3bce,
                        significand: 0x9000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3bce,
                        significand: 0x9000000000000000,
                    },
                ],
                [
                    f::Ext80 {
                        sign_exp: 0x3de6,
                        significand: 0xc000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3bce,
                        significand: 0x9000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3bce,
                        significand: 0x9000000000000000,
                    },
                ],
            ],
            [0x0000000000000002, 0x0000000000000002],
            [true, true],
        ),
        (
            "square_minimum_normal",
            [
                0x2000000000000000,
                0x0000000000000000,
                0x0000000000000000,
                0x0000000000000000,
            ],
            [
                [
                    f::Ext80 {
                        sign_exp: 0x3e00,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3c01,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3c01,
                        significand: 0x8000000000000000,
                    },
                ],
                [
                    f::Ext80 {
                        sign_exp: 0x3e00,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3c01,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3c01,
                        significand: 0x8000000000000000,
                    },
                ],
            ],
            [0x0010000000000000, 0x0010000000000000],
            [true, true],
        ),
        (
            "square_below_minimum_normal",
            [
                0x1ff0000000000000,
                0x0000000000000000,
                0x0000000000000000,
                0x0000000000000000,
            ],
            [
                [
                    f::Ext80 {
                        sign_exp: 0x3dff,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3bff,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3bff,
                        significand: 0x8000000000000000,
                    },
                ],
                [
                    f::Ext80 {
                        sign_exp: 0x3dff,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3bff,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3bff,
                        significand: 0x8000000000000000,
                    },
                ],
            ],
            [0x0004000000000000, 0x0004000000000000],
            [true, true],
        ),
        (
            "cancellation_adjacent_above_one",
            [
                0x3ff0000000000001,
                0x0000000000000000,
                0x3ff0000000000000,
                0x0000000000000000,
            ],
            [
                [
                    f::Ext80 {
                        sign_exp: 0x3fcb,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3f97,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3f97,
                        significand: 0x8000000000000000,
                    },
                ],
                [
                    f::Ext80 {
                        sign_exp: 0x3fcb,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3f97,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3f97,
                        significand: 0x8000000000000000,
                    },
                ],
            ],
            [0x3970000000000000, 0x3970000000000000],
            [true, true],
        ),
        (
            "cancellation_below_one",
            [
                0x3ff0000000000000,
                0x0000000000000000,
                0x3fefffffffffffff,
                0x0000000000000000,
            ],
            [
                [
                    f::Ext80 {
                        sign_exp: 0x3fca,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3f95,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3f95,
                        significand: 0x8000000000000000,
                    },
                ],
                [
                    f::Ext80 {
                        sign_exp: 0x3fca,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3f95,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3f95,
                        significand: 0x8000000000000000,
                    },
                ],
            ],
            [0x3950000000000000, 0x3950000000000000],
            [true, true],
        ),
        (
            "subtraction_midpoint_ties_even",
            [
                0x3ff0000000000000,
                0x0000000000000000,
                0xbca0000000000000,
                0x0000000000000000,
            ],
            [
                [
                    f::Ext80 {
                        sign_exp: 0x3fff,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3fff,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3fff,
                        significand: 0x8000000000000000,
                    },
                ],
                [
                    f::Ext80 {
                        sign_exp: 0x3fff,
                        significand: 0x8000000000000400,
                    },
                    f::Ext80 {
                        sign_exp: 0x3fff,
                        significand: 0x8000000000000800,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3fff,
                        significand: 0x8000000000000800,
                    },
                ],
            ],
            [0x3ff0000000000000, 0x3ff0000000000001],
            [true, true],
        ),
        (
            "subtraction_midpoint_ties_odd",
            [
                0x3ff0000000000001,
                0x0000000000000000,
                0xbca0000000000000,
                0x0000000000000000,
            ],
            [
                [
                    f::Ext80 {
                        sign_exp: 0x3fff,
                        significand: 0x8000000000001000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3fff,
                        significand: 0x8000000000002000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3fff,
                        significand: 0x8000000000002000,
                    },
                ],
                [
                    f::Ext80 {
                        sign_exp: 0x3fff,
                        significand: 0x8000000000000c00,
                    },
                    f::Ext80 {
                        sign_exp: 0x3fff,
                        significand: 0x8000000000001800,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3fff,
                        significand: 0x8000000000001800,
                    },
                ],
            ],
            [0x3ff0000000000004, 0x3ff0000000000003],
            [true, true],
        ),
        (
            "distance_sum_quarter_ulp",
            [
                0x3ff0000000000000,
                0x3e40000000000000,
                0x0000000000000000,
                0x0000000000000000,
            ],
            [
                [
                    f::Ext80 {
                        sign_exp: 0x3fff,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3fff,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3fe4,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3fc9,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3fff,
                        significand: 0x8000000000000000,
                    },
                ],
                [
                    f::Ext80 {
                        sign_exp: 0x3fff,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3fff,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3fe4,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3fc9,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3fff,
                        significand: 0x8000000000000200,
                    },
                ],
            ],
            [0x3ff0000000000000, 0x3ff0000000000000],
            [true, true],
        ),
        (
            "distance_sum_exact_ulp",
            [
                0x3ff0000000000000,
                0x3e50000000000000,
                0x0000000000000000,
                0x0000000000000000,
            ],
            [
                [
                    f::Ext80 {
                        sign_exp: 0x3fff,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3fff,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3fe5,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3fcb,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3fff,
                        significand: 0x8000000000000800,
                    },
                ],
                [
                    f::Ext80 {
                        sign_exp: 0x3fff,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3fff,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3fe5,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3fcb,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3fff,
                        significand: 0x8000000000000800,
                    },
                ],
            ],
            [0x3ff0000000000001, 0x3ff0000000000001],
            [true, true],
        ),
        (
            "distance_sum_nine_sixteenths_ulp",
            [
                0x3ff0000000000000,
                0x3e48000000000000,
                0x0000000000000000,
                0x0000000000000000,
            ],
            [
                [
                    f::Ext80 {
                        sign_exp: 0x3fff,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3fff,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3fe4,
                        significand: 0xc000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3fca,
                        significand: 0x9000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3fff,
                        significand: 0x8000000000000800,
                    },
                ],
                [
                    f::Ext80 {
                        sign_exp: 0x3fff,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3fff,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3fe4,
                        significand: 0xc000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3fca,
                        significand: 0x9000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3fff,
                        significand: 0x8000000000000480,
                    },
                ],
            ],
            [0x3ff0000000000001, 0x3ff0000000000001],
            [true, true],
        ),
        (
            "initial_cap_equal",
            [
                0x40f86a0000000000,
                0x0000000000000000,
                0x0000000000000000,
                0x0000000000000000,
            ],
            [
                [
                    f::Ext80 {
                        sign_exp: 0x400f,
                        significand: 0xc350000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x4020,
                        significand: 0x9502f90000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x4020,
                        significand: 0x9502f90000000000,
                    },
                ],
                [
                    f::Ext80 {
                        sign_exp: 0x400f,
                        significand: 0xc350000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x4020,
                        significand: 0x9502f90000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x4020,
                        significand: 0x9502f90000000000,
                    },
                ],
            ],
            [0x4202a05f20000000, 0x4202a05f20000000],
            [false, false],
        ),
        (
            "initial_cap_below",
            [
                0x40f869ffffffffff,
                0x0000000000000000,
                0x0000000000000000,
                0x0000000000000000,
            ],
            [
                [
                    f::Ext80 {
                        sign_exp: 0x400f,
                        significand: 0xc34ffffffffff800,
                    },
                    f::Ext80 {
                        sign_exp: 0x4020,
                        significand: 0x9502f8fffffff000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x4020,
                        significand: 0x9502f8fffffff000,
                    },
                ],
                [
                    f::Ext80 {
                        sign_exp: 0x400f,
                        significand: 0xc34ffffffffff800,
                    },
                    f::Ext80 {
                        sign_exp: 0x4020,
                        significand: 0x9502f8fffffff3cb,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x4020,
                        significand: 0x9502f8fffffff3cb,
                    },
                ],
            ],
            [0x4202a05f1ffffffe, 0x4202a05f1ffffffe],
            [true, true],
        ),
        (
            "initial_cap_above",
            [
                0x40f86a0000000001,
                0x0000000000000000,
                0x0000000000000000,
                0x0000000000000000,
            ],
            [
                [
                    f::Ext80 {
                        sign_exp: 0x400f,
                        significand: 0xc350000000000800,
                    },
                    f::Ext80 {
                        sign_exp: 0x4020,
                        significand: 0x9502f90000001000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x4020,
                        significand: 0x9502f90000001000,
                    },
                ],
                [
                    f::Ext80 {
                        sign_exp: 0x400f,
                        significand: 0xc350000000000800,
                    },
                    f::Ext80 {
                        sign_exp: 0x4020,
                        significand: 0x9502f90000000c35,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x4020,
                        significand: 0x9502f90000000c35,
                    },
                ],
            ],
            [0x4202a05f20000002, 0x4202a05f20000002],
            [false, false],
        ),
        (
            "maximum_finite",
            [
                0x7fefffffffffffff,
                0x0000000000000000,
                0x0000000000000000,
                0x0000000000000000,
            ],
            [
                [
                    f::Ext80 {
                        sign_exp: 0x43fe,
                        significand: 0xfffffffffffff800,
                    },
                    f::Ext80 {
                        sign_exp: 0x47fe,
                        significand: 0xfffffffffffff000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x47fe,
                        significand: 0xfffffffffffff000,
                    },
                ],
                [
                    f::Ext80 {
                        sign_exp: 0x43fe,
                        significand: 0xfffffffffffff800,
                    },
                    f::Ext80 {
                        sign_exp: 0x47fe,
                        significand: 0xfffffffffffff000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x47fe,
                        significand: 0xfffffffffffff000,
                    },
                ],
            ],
            [0x7ff0000000000000, 0x7ff0000000000000],
            [false, false],
        ),
        (
            "maximum_opposite_subtract",
            [
                0x7fefffffffffffff,
                0x0000000000000000,
                0xffefffffffffffff,
                0x0000000000000000,
            ],
            [
                [
                    f::Ext80 {
                        sign_exp: 0x43ff,
                        significand: 0xfffffffffffff800,
                    },
                    f::Ext80 {
                        sign_exp: 0x4800,
                        significand: 0xfffffffffffff000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x4800,
                        significand: 0xfffffffffffff000,
                    },
                ],
                [
                    f::Ext80 {
                        sign_exp: 0x43ff,
                        significand: 0xfffffffffffff800,
                    },
                    f::Ext80 {
                        sign_exp: 0x4800,
                        significand: 0xfffffffffffff000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x4800,
                        significand: 0xfffffffffffff000,
                    },
                ],
            ],
            [0x7ff0000000000000, 0x7ff0000000000000],
            [false, false],
        ),
        (
            "maximum_both_axes",
            [
                0x7fefffffffffffff,
                0x7fefffffffffffff,
                0xffefffffffffffff,
                0xffefffffffffffff,
            ],
            [
                [
                    f::Ext80 {
                        sign_exp: 0x43ff,
                        significand: 0xfffffffffffff800,
                    },
                    f::Ext80 {
                        sign_exp: 0x4800,
                        significand: 0xfffffffffffff000,
                    },
                    f::Ext80 {
                        sign_exp: 0x43ff,
                        significand: 0xfffffffffffff800,
                    },
                    f::Ext80 {
                        sign_exp: 0x4800,
                        significand: 0xfffffffffffff000,
                    },
                    f::Ext80 {
                        sign_exp: 0x4801,
                        significand: 0xfffffffffffff000,
                    },
                ],
                [
                    f::Ext80 {
                        sign_exp: 0x43ff,
                        significand: 0xfffffffffffff800,
                    },
                    f::Ext80 {
                        sign_exp: 0x4800,
                        significand: 0xfffffffffffff000,
                    },
                    f::Ext80 {
                        sign_exp: 0x43ff,
                        significand: 0xfffffffffffff800,
                    },
                    f::Ext80 {
                        sign_exp: 0x4800,
                        significand: 0xfffffffffffff000,
                    },
                    f::Ext80 {
                        sign_exp: 0x4801,
                        significand: 0xfffffffffffff000,
                    },
                ],
            ],
            [0x7ff0000000000000, 0x7ff0000000000000],
            [false, false],
        ),
        (
            "large_cancel_adjacent",
            [
                0x7fefffffffffffff,
                0x0000000000000000,
                0x7feffffffffffffe,
                0x0000000000000000,
            ],
            [
                [
                    f::Ext80 {
                        sign_exp: 0x43ca,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x4795,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x4795,
                        significand: 0x8000000000000000,
                    },
                ],
                [
                    f::Ext80 {
                        sign_exp: 0x43ca,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x4795,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x0000,
                        significand: 0x0000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x4795,
                        significand: 0x8000000000000000,
                    },
                ],
            ],
            [0x7ff0000000000000, 0x7ff0000000000000],
            [false, false],
        ),
        (
            "two_to_53_and_one",
            [
                0x4340000000000000,
                0x3ff0000000000000,
                0x0000000000000000,
                0x0000000000000000,
            ],
            [
                [
                    f::Ext80 {
                        sign_exp: 0x4034,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x4069,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3fff,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3fff,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x4069,
                        significand: 0x8000000000000000,
                    },
                ],
                [
                    f::Ext80 {
                        sign_exp: 0x4034,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x4069,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3fff,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x3fff,
                        significand: 0x8000000000000000,
                    },
                    f::Ext80 {
                        sign_exp: 0x4069,
                        significand: 0x8000000000000000,
                    },
                ],
            ],
            [0x4690000000000000, 0x4690000000000000],
            [false, false],
        ),
    ];
    for &(name, [x, y, qx, qy], expected, stores, strict_below) in rows {
        for (profile, admission) in [cap(), admit_control_word(0x1332).unwrap()]
            .into_iter()
            .enumerate()
        {
            let dx = admission.sub(wide(x), wide(qx)).unwrap();
            let xx = admission.mul(dx, dx).unwrap();
            let dy = admission.sub(wide(y), wide(qy)).unwrap();
            let yy = admission.mul(dy, dy).unwrap();
            let distance = admission.add(yy, xx).unwrap();
            for (stage, (actual, expected)) in [dx, xx, dy, yy, distance]
                .into_iter()
                .zip(expected[profile])
                .enumerate()
            {
                assert_eq!(
                    f::to_ext80_nearest(actual).unwrap(),
                    expected,
                    "{name} profile{profile} stage{stage}"
                );
            }
            if stores[profile] == 0x7ff0000000000000 {
                assert_eq!(
                    f::to_f64_nearest(distance),
                    Err(NumericError::Overflow),
                    "{name}"
                );
            } else {
                assert_eq!(
                    f::to_f64_nearest(distance).unwrap().to_bits(),
                    stores[profile],
                    "{name} profile{profile}"
                );
            }
            let out = evaluate(
                &admission,
                qx,
                qy,
                (-3f64).to_bits(),
                &mut Rows::at(&[(x, y)]),
            )
            .unwrap();
            let below = strict_below[profile];
            assert_eq!(
                out.selected_native_id,
                below.then_some(100001),
                "{name} profile{profile}"
            );
            assert_eq!(
                out.final_best_qword,
                if below {
                    stores[profile]
                } else {
                    INITIAL_BEST_QWORD
                },
                "{name} profile{profile}"
            );
        }
    }
}
