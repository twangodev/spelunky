// Independently prepared Fraction references. Rust/compiler UNRUN.
use super::*;
#[test]
fn reference_add113_nearest() {
    assert_eq!(
        add113_nearest(
            F128::from_words(0x0000000000000000, 0x0000000000000000),
            F128::from_words(0x8000000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x0000000000000000, 0x0000000000000000)),
        "zero-sign"
    );
    assert_eq!(
        add113_nearest(
            F128::from_words(0x8000000000000000, 0x0000000000000000),
            F128::from_words(0x8000000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x8000000000000000, 0x0000000000000000)),
        "negative-zero"
    );
    assert_eq!(
        add113_nearest(
            F128::from_words(0x3fff000000000000, 0x0000000000000000),
            F128::from_words(0x4000800000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x4001000000000000, 0x0000000000000000)),
        "one-third"
    );
    assert_eq!(
        add113_nearest(
            F128::from_words(0x3fff000000000000, 0x0000000000000000),
            F128::from_words(0x3f8e000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x3fff000000000000, 0x0000000000000000)),
        "quad-tie-even"
    );
    assert_eq!(
        add113_nearest(
            F128::from_words(0x3fff000000000000, 0x0000000000000001),
            F128::from_words(0x3f8e000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x3fff000000000000, 0x0000000000000002)),
        "quad-tie-odd"
    );
    assert_eq!(
        add113_nearest(
            F128::from_words(0x3fff000000000000, 0x0000000000000000),
            F128::from_words(0x3fbf000000000000, 0x4000000000000000)
        ),
        Ok(F128::from_words(0x3fff000000000000, 0x0001000000000000)),
        "double-round"
    );
    assert_eq!(
        add113_nearest(
            F128::from_words(0x3fff000000000000, 0x0000400000000000),
            F128::from_words(0x3fff000000000000, 0x0000800000000000)
        ),
        Ok(F128::from_words(0x4000000000000000, 0x0000600000000000)),
        "asymmetric-right"
    );
    assert_eq!(
        add113_nearest(
            F128::from_words(0x3fff000000000000, 0x0000800000000000),
            F128::from_words(0x3fff000000000000, 0x0000400000000000)
        ),
        Ok(F128::from_words(0x4000000000000000, 0x0000600000000000)),
        "asymmetric-left"
    );
    assert_eq!(
        add113_nearest(
            F128::from_words(0x3fff000000000000, 0x0000000000000000),
            F128::from_words(0xbc17000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x3fff000000000000, 0x0000000000000000)),
        "tiny-opposite"
    );
    assert_eq!(
        add113_nearest(
            F128::from_words(0xbfff000000000000, 0x0000000000000000),
            F128::from_words(0x3c17000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0xbfff000000000000, 0x0000000000000000)),
        "negative-tiny-opposite"
    );
    assert_eq!(
        add113_nearest(
            F128::from_words(0x4001000000000000, 0x0000000000000000),
            F128::from_words(0xbfc0800000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x4000ffffffffffff, 0xfffe800000000000)),
        "below-binade"
    );
    assert_eq!(
        add113_nearest(
            F128::from_words(0x4001000000000000, 0x0000000000000000),
            F128::from_words(0x3fc0800000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x4001000000000000, 0x0000c00000000000)),
        "above-binade"
    );
    assert_eq!(
        add113_nearest(
            F128::from_words(0x0000000000000000, 0x0000000000000003),
            F128::from_words(0x0000000000000000, 0x0000000000000004)
        ),
        Ok(F128::from_words(0x0000000000000000, 0x0000000000000007)),
        "subnormal-add-shortcut"
    );
    assert_eq!(
        add113_nearest(
            F128::from_words(0x0000ffffffffffff, 0xffffffffffffffff),
            F128::from_words(0x0000000000000000, 0x0000000000000002)
        ),
        Ok(F128::from_words(0x0001000000000000, 0x0000000000000001)),
        "subnormal-normal-cross"
    );
    assert_eq!(
        add113_nearest(
            F128::from_words(0x0001000000000000, 0x0000000000000000),
            F128::from_words(0x3ff5000000000000, 0x0000400000000000)
        ),
        Ok(F128::from_words(0x3ff5000000000000, 0x0000400000000000)),
        "subnormal-internal67"
    );
    assert_eq!(
        add113_nearest(
            F128::from_words(0x0000000000000000, 0x0000000000000001),
            F128::from_words(0x3ffe000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x3ffe000000000000, 0x0000000000000000)),
        "underflow-product"
    );
    assert_eq!(
        add113_nearest(
            F128::from_words(0x0000000000000000, 0x0000000000000001),
            F128::from_words(0x4000000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x4000000000000000, 0x0000000000000000)),
        "half-minimum"
    );
    assert_eq!(
        add113_nearest(
            F128::from_words(0x7e7f000000000000, 0x0000000000000000),
            F128::from_words(0x817f000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x7e7f000000000000, 0x0000000000000000)),
        "huge-opposite"
    );
    assert_eq!(
        add113_nearest(
            F128::from_words(0x3fffffffffffffff, 0xffffffffffffffff),
            F128::from_words(0x3fffffffffffffff, 0xffffffffffffffff)
        ),
        Ok(F128::from_words(0x4000ffffffffffff, 0xffffffffffffffff)),
        "wide-product"
    );
    assert_eq!(
        add113_nearest(
            F128::from_words(0x3f976e0364caa050, 0x8b88599668a6328e),
            F128::from_words(0x407ec4406dfade38, 0x9adc2542fa7422b1)
        ),
        Ok(F128::from_words(0x407ec4406dfade38, 0x9adc2542fa7422b1)),
        "rational-0"
    );
    assert_eq!(
        add113_nearest(
            F128::from_words(0x401dd161ab6e8e1d, 0xd23ab5033ba5120a),
            F128::from_words(0xc0a29f9cba4fbdeb, 0x5582b8a652fe4854)
        ),
        Ok(F128::from_words(0xc0a29f9cba4fbdeb, 0x5582b8a652fe4854)),
        "rational-1"
    );
    assert_eq!(
        add113_nearest(
            F128::from_words(0x3f7526f6860ee0ad, 0x8eb452c36cff7606),
            F128::from_words(0x403e55319397bf4c, 0x338950d18897ce69)
        ),
        Ok(F128::from_words(0x403e55319397bf4c, 0x338950d18897ce69)),
        "rational-2"
    );
    assert_eq!(
        add113_nearest(
            F128::from_words(0x407ed0e5d4fcde8a, 0x5a07b21766575899),
            F128::from_words(0x402f31d543d75467, 0x8f2d1c302929ce5c)
        ),
        Ok(F128::from_words(0x407ed0e5d4fcde8a, 0x5a07b219ca01e048)),
        "rational-3"
    );
    assert_eq!(
        add113_nearest(
            F128::from_words(0xbf6fa93875ccd29c, 0xf4a4b1d2823a55be),
            F128::from_words(0x4046687a91deaa23, 0x80e6b1de6e41b28b)
        ),
        Ok(F128::from_words(0x4046687a91deaa23, 0x80e6b1de6e41b28b)),
        "rational-4"
    );
    assert_eq!(
        add113_nearest(
            F128::from_words(0x40563949f3796c30, 0xc257681164d2a50c),
            F128::from_words(0x4019b1ee9579f94c, 0x4eaed9ca369d4c37)
        ),
        Ok(F128::from_words(0x40563949f3796c30, 0xc264f78610a26f6e)),
        "rational-5"
    );
    assert_eq!(
        add113_nearest(
            F128::from_words(0x40a0d40f4dedffc8, 0x4872f8cf5ea16b82),
            F128::from_words(0xbf8cbf37785626ca, 0x59ec4e6748db3775)
        ),
        Ok(F128::from_words(0x40a0d40f4dedffc8, 0x4872f8cf5ea16b82)),
        "rational-6"
    );
    assert_eq!(
        add113_nearest(
            F128::from_words(0x40218c303fcc2682, 0x10e9a7dfa0c84c0c),
            F128::from_words(0x4096305c412a05f3, 0xe55ce2d5da562d0f)
        ),
        Ok(F128::from_words(0x4096305c412a05f3, 0xe55ce2d5da562d0f)),
        "rational-7"
    );
    assert_eq!(
        add113_nearest(
            F128::from_words(0xc018b179d909cb04, 0xba30a83aeb3d10d0),
            F128::from_words(0x40b2ede9d1d55254, 0x740d7f67c593d6da)
        ),
        Ok(F128::from_words(0x40b2ede9d1d55254, 0x740d7f67c593d6da)),
        "rational-8"
    );
    assert_eq!(
        add113_nearest(
            F128::from_words(0xbff78794a804f48f, 0xc2b2277ae5448dac),
            F128::from_words(0xc01cafdf5234216b, 0xc3b248e12c219f6f)
        ),
        Ok(F128::from_words(0xc01cafdf52342da8, 0x68f27085aa3730ab)),
        "rational-9"
    );
    assert_eq!(
        add113_nearest(
            F128::from_words(0x403417b28ac30de6, 0xdaaa694cb8c61dc8),
            F128::from_words(0xbfc646abe76fb68a, 0x3786dcd6370c5dfe)
        ),
        Ok(F128::from_words(0x403417b28ac30de6, 0xdaaa694cb8c61dc3)),
        "rational-10"
    );
    assert_eq!(
        add113_nearest(
            F128::from_words(0x3f81aed55a1aa8f2, 0xb9929c8651647899),
            F128::from_words(0xc07ac9f1bc8a8794, 0x944e553eba43ebba)
        ),
        Ok(F128::from_words(0xc07ac9f1bc8a8794, 0x944e553eba43ebba)),
        "rational-11"
    );
    assert_eq!(
        add113_nearest(
            F128::from_words(0x4052293b2522b36c, 0x51ae68d346a92b28),
            F128::from_words(0x3f78be450f941f1a, 0xfafaba94d160477f)
        ),
        Ok(F128::from_words(0x4052293b2522b36c, 0x51ae68d346a92b28)),
        "rational-12"
    );
    assert_eq!(
        add113_nearest(
            F128::from_words(0x4033ed930a3f5f8e, 0x9d73e514430664b1),
            F128::from_words(0xc0afa4c59f00ce24, 0xe4945d544d2d7ac9)
        ),
        Ok(F128::from_words(0xc0afa4c59f00ce24, 0xe4945d544d2d7ac9)),
        "rational-13"
    );
    assert_eq!(
        add113_nearest(
            F128::from_words(0xc01aa6388b0cd844, 0x9c7146f0c0206cec),
            F128::from_words(0xc0a7bb7ec3bc4b2b, 0x238d013d590b3b44)
        ),
        Ok(F128::from_words(0xc0a7bb7ec3bc4b2b, 0x238d013d590b3b44)),
        "rational-14"
    );
    assert_eq!(
        add113_nearest(
            F128::from_words(0x3f6b3feefc29d05b, 0x383484e56b2f8cee),
            F128::from_words(0x3ff9735d47b31306, 0xb8406e5ef8b1615a)
        ),
        Ok(F128::from_words(0x3ff9735d47b31306, 0xb8406e5ef8b1615a)),
        "rational-15"
    );
    assert_eq!(
        add113_nearest(
            F128::from_words(0xc06765bba6d242d5, 0x1b878e0f4a5c1679),
            F128::from_words(0xc07fa6a87c0ced04, 0xc758747c98b9636b)
        ),
        Ok(F128::from_words(0xc07fa6a87d72a8ab, 0x999b4998204772b5)),
        "rational-16"
    );
    assert_eq!(
        add113_nearest(
            F128::from_words(0x3f87ada2f38bd6ac, 0x0979bc14034f4082),
            F128::from_words(0x3fa22cf56289f4df, 0x76496d0797838687)
        ),
        Ok(F128::from_words(0x3fa22cf562bfa93d, 0xe7c44288c6bb0907)),
        "rational-17"
    );
    assert_eq!(
        add113_nearest(
            F128::from_words(0x406b1c1e71f8f9d3, 0x9f83c3db94f26656),
            F128::from_words(0xc017365f17e9f056, 0x6fb38542338b72c8)
        ),
        Ok(F128::from_words(0x406b1c1e71f8f9d3, 0x9f83c3db818c74d7)),
        "rational-18"
    );
    assert_eq!(
        add113_nearest(
            F128::from_words(0x40b316d1d4f2f338, 0xd1997d9fc706580d),
            F128::from_words(0xbf5144bbffc68296, 0xe2294cf3edb7321f)
        ),
        Ok(F128::from_words(0x40b316d1d4f2f338, 0xd1997d9fc706580d)),
        "rational-19"
    );
    assert_eq!(
        add113_nearest(
            F128::from_words(0x4085061e0e40fd35, 0x1a9606d6b79e1731),
            F128::from_words(0xc01f6a34288afab6, 0x3361343b1a74bdcc)
        ),
        Ok(F128::from_words(0x4085061e0e40fd35, 0x1a9606d6b79e1188)),
        "rational-20"
    );
    assert_eq!(
        add113_nearest(
            F128::from_words(0x4038e6299b406762, 0x39f74335d8426357),
            F128::from_words(0x408878acf8159ae3, 0x30c7403b17700db5)
        ),
        Ok(F128::from_words(0x408878acf8159ae3, 0x30c7403cfd99a8f5)),
        "rational-21"
    );
    assert_eq!(
        add113_nearest(
            F128::from_words(0xbf9c4603f1d0122b, 0xfa6fbedc7fb712b1),
            F128::from_words(0xc05f2c8cb710998d, 0xca54bca8286b1afa)
        ),
        Ok(F128::from_words(0xc05f2c8cb710998d, 0xca54bca8286b1afa)),
        "rational-22"
    );
    assert_eq!(
        add113_nearest(
            F128::from_words(0x3ffe55f46c23593e, 0x8b222784e8912b03),
            F128::from_words(0x3f855edb1d05a930, 0xad5056dd1cea2000)
        ),
        Ok(F128::from_words(0x3ffe55f46c23593e, 0x8b222784e8912b03)),
        "rational-23"
    );
    assert_eq!(
        add113_nearest(
            F128::from_words(0xc00dc6f9731d7188, 0x88ab3f369dc88f0a),
            F128::from_words(0x3fcd0983fc817668, 0x79e30be509ff2540)
        ),
        Ok(F128::from_words(0xc00dc6f9731d7188, 0x88aa35b2a14718a2)),
        "rational-24"
    );
    assert_eq!(
        add113_nearest(
            F128::from_words(0xbf5319de0ca1331b, 0xfd0f1690cd8d31e0),
            F128::from_words(0xbf8cc8f264973a9a, 0xd918d134201cf6b0)
        ),
        Ok(F128::from_words(0xbf8cc8f264973a9a, 0xd9a5c03a70b684af)),
        "rational-25"
    );
    assert_eq!(
        add113_nearest(
            F128::from_words(0xbf8b3dc86e05c0b8, 0x6f71ce41c31671c1),
            F128::from_words(0xbf6f78e83b17518a, 0x1af08066346e5bd8)
        ),
        Ok(F128::from_words(0xbf8b3dc86e1d4f3c, 0x20e6e6e3721e7824)),
        "rational-26"
    );
    assert_eq!(
        add113_nearest(
            F128::from_words(0xbfcb6e000ec540c5, 0xd6ecf142257585a3),
            F128::from_words(0xbffefb6d0c5d8b55, 0xc0c1e7628d3f1dca)
        ),
        Ok(F128::from_words(0xbffefb6d0c5d8b55, 0xee81e93b3557d8a8)),
        "rational-27"
    );
    assert_eq!(
        add113_nearest(
            F128::from_words(0xc049a21188fe809d, 0x9ab48d201b616253),
            F128::from_words(0xbf9acc0df912452f, 0x05bd48e6edc45a9e)
        ),
        Ok(F128::from_words(0xc049a21188fe809d, 0x9ab48d201b616253)),
        "rational-28"
    );
    assert_eq!(
        add113_nearest(
            F128::from_words(0x3fe52ef74185e00b, 0x37bd91ed0535bee7),
            F128::from_words(0xc08700a071d58a43, 0x6e540b594e598872)
        ),
        Ok(F128::from_words(0xc08700a071d58a43, 0x6e540b594e598872)),
        "rational-29"
    );
}
#[test]
fn reference_mul113_nearest() {
    assert_eq!(
        mul113_nearest(
            F128::from_words(0x0000000000000000, 0x0000000000000000),
            F128::from_words(0x8000000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x8000000000000000, 0x0000000000000000)),
        "zero-sign"
    );
    assert_eq!(
        mul113_nearest(
            F128::from_words(0x8000000000000000, 0x0000000000000000),
            F128::from_words(0x8000000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x0000000000000000, 0x0000000000000000)),
        "negative-zero"
    );
    assert_eq!(
        mul113_nearest(
            F128::from_words(0x3fff000000000000, 0x0000000000000000),
            F128::from_words(0x4000800000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x4000800000000000, 0x0000000000000000)),
        "one-third"
    );
    assert_eq!(
        mul113_nearest(
            F128::from_words(0x3fff000000000000, 0x0000000000000000),
            F128::from_words(0x3f8e000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x3f8e000000000000, 0x0000000000000000)),
        "quad-tie-even"
    );
    assert_eq!(
        mul113_nearest(
            F128::from_words(0x3fff000000000000, 0x0000000000000001),
            F128::from_words(0x3f8e000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x3f8e000000000000, 0x0000000000000001)),
        "quad-tie-odd"
    );
    assert_eq!(
        mul113_nearest(
            F128::from_words(0x3fff000000000000, 0x0000000000000000),
            F128::from_words(0x3fbf000000000000, 0x4000000000000000)
        ),
        Ok(F128::from_words(0x3fbf000000000000, 0x4000000000000000)),
        "double-round"
    );
    assert_eq!(
        mul113_nearest(
            F128::from_words(0x3fff000000000000, 0x0000400000000000),
            F128::from_words(0x3fff000000000000, 0x0000800000000000)
        ),
        Ok(F128::from_words(0x3fff000000000000, 0x0000c00000000000)),
        "asymmetric-right"
    );
    assert_eq!(
        mul113_nearest(
            F128::from_words(0x3fff000000000000, 0x0000800000000000),
            F128::from_words(0x3fff000000000000, 0x0000400000000000)
        ),
        Ok(F128::from_words(0x3fff000000000000, 0x0000c00000000000)),
        "asymmetric-left"
    );
    assert_eq!(
        mul113_nearest(
            F128::from_words(0x3fff000000000000, 0x0000000000000000),
            F128::from_words(0xbc17000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0xbc17000000000000, 0x0000000000000000)),
        "tiny-opposite"
    );
    assert_eq!(
        mul113_nearest(
            F128::from_words(0xbfff000000000000, 0x0000000000000000),
            F128::from_words(0x3c17000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0xbc17000000000000, 0x0000000000000000)),
        "negative-tiny-opposite"
    );
    assert_eq!(
        mul113_nearest(
            F128::from_words(0x4001000000000000, 0x0000000000000000),
            F128::from_words(0xbfc0800000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0xbfc2800000000000, 0x0000000000000000)),
        "below-binade"
    );
    assert_eq!(
        mul113_nearest(
            F128::from_words(0x4001000000000000, 0x0000000000000000),
            F128::from_words(0x3fc0800000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x3fc2800000000000, 0x0000000000000000)),
        "above-binade"
    );
    assert_eq!(
        mul113_nearest(
            F128::from_words(0x0000000000000000, 0x0000000000000003),
            F128::from_words(0x0000000000000000, 0x0000000000000004)
        ),
        Ok(F128::from_words(0x0000000000000000, 0x0000000000000000)),
        "subnormal-add-shortcut"
    );
    assert_eq!(
        mul113_nearest(
            F128::from_words(0x0000ffffffffffff, 0xffffffffffffffff),
            F128::from_words(0x0000000000000000, 0x0000000000000002)
        ),
        Ok(F128::from_words(0x0000000000000000, 0x0000000000000000)),
        "subnormal-normal-cross"
    );
    assert_eq!(
        mul113_nearest(
            F128::from_words(0x0001000000000000, 0x0000000000000000),
            F128::from_words(0x3ff5000000000000, 0x0000400000000000)
        ),
        Ok(F128::from_words(0x0000004000000000, 0x0000001000000000)),
        "subnormal-internal67"
    );
    assert_eq!(
        mul113_nearest(
            F128::from_words(0x0000000000000000, 0x0000000000000001),
            F128::from_words(0x3ffe000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x0000000000000000, 0x0000000000000000)),
        "underflow-product"
    );
    assert_eq!(
        mul113_nearest(
            F128::from_words(0x0000000000000000, 0x0000000000000001),
            F128::from_words(0x4000000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x0000000000000000, 0x0000000000000002)),
        "half-minimum"
    );
    assert_eq!(
        mul113_nearest(
            F128::from_words(0x7e7f000000000000, 0x0000000000000000),
            F128::from_words(0x817f000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0xbfff000000000000, 0x0000000000000000)),
        "huge-opposite"
    );
    assert_eq!(
        mul113_nearest(
            F128::from_words(0x3fffffffffffffff, 0xffffffffffffffff),
            F128::from_words(0x3fffffffffffffff, 0xffffffffffffffff)
        ),
        Ok(F128::from_words(0x4000ffffffffffff, 0xfffffffffffffffe)),
        "wide-product"
    );
    assert_eq!(
        mul113_nearest(
            F128::from_words(0x3f976e0364caa050, 0x8b88599668a6328e),
            F128::from_words(0x407ec4406dfade38, 0x9adc2542fa7422b1)
        ),
        Ok(F128::from_words(0x4017434d0e068a54, 0x6f1884407ab1dff3)),
        "rational-0"
    );
    assert_eq!(
        mul113_nearest(
            F128::from_words(0x401dd161ab6e8e1d, 0xd23ab5033ba5120a),
            F128::from_words(0xc0a29f9cba4fbdeb, 0x5582b8a652fe4854)
        ),
        Ok(F128::from_words(0xc0c179c51f8f4e5a, 0x0c026de16fd05b64)),
        "rational-1"
    );
    assert_eq!(
        mul113_nearest(
            F128::from_words(0x3f7526f6860ee0ad, 0x8eb452c36cff7606),
            F128::from_words(0x403e55319397bf4c, 0x338950d18897ce69)
        ),
        Ok(F128::from_words(0x3fb4891f7fcfe0ed, 0x88aeebe03bbcde5f)),
        "rational-2"
    );
    assert_eq!(
        mul113_nearest(
            F128::from_words(0x407ed0e5d4fcde8a, 0x5a07b21766575899),
            F128::from_words(0x402f31d543d75467, 0x8f2d1c302929ce5c)
        ),
        Ok(F128::from_words(0x40af15b28e975649, 0xc3ad93d4f3ca309e)),
        "rational-3"
    );
    assert_eq!(
        mul113_nearest(
            F128::from_words(0xbf6fa93875ccd29c, 0xf4a4b1d2823a55be),
            F128::from_words(0x4046687a91deaa23, 0x80e6b1de6e41b28b)
        ),
        Ok(F128::from_words(0xbfb72b617e6d8009, 0x5c65a2947ce49a31)),
        "rational-4"
    );
    assert_eq!(
        mul113_nearest(
            F128::from_words(0x40563949f3796c30, 0xc257681164d2a50c),
            F128::from_words(0x4019b1ee9579f94c, 0x4eaed9ca369d4c37)
        ),
        Ok(F128::from_words(0x40710985073f0550, 0x8f3b76d90a13c976)),
        "rational-5"
    );
    assert_eq!(
        mul113_nearest(
            F128::from_words(0x40a0d40f4dedffc8, 0x4872f8cf5ea16b82),
            F128::from_words(0xbf8cbf37785626ca, 0x59ec4e6748db3775)
        ),
        Ok(F128::from_words(0xc02e98d61230802c, 0x533828da35a24eff)),
        "rational-6"
    );
    assert_eq!(
        mul113_nearest(
            F128::from_words(0x40218c303fcc2682, 0x10e9a7dfa0c84c0c),
            F128::from_words(0x4096305c412a05f3, 0xe55ce2d5da562d0f)
        ),
        Ok(F128::from_words(0x40b8d70811f2a46b, 0xfe1b57f32f3985dd)),
        "rational-7"
    );
    assert_eq!(
        mul113_nearest(
            F128::from_words(0xc018b179d909cb04, 0xba30a83aeb3d10d0),
            F128::from_words(0x40b2ede9d1d55254, 0x740d7f67c593d6da)
        ),
        Ok(F128::from_words(0xc0cca229c9160aee, 0x3ee22e5a7c3317f7)),
        "rational-8"
    );
    assert_eq!(
        mul113_nearest(
            F128::from_words(0xbff78794a804f48f, 0xc2b2277ae5448dac),
            F128::from_words(0xc01cafdf5234216b, 0xc3b248e12c219f6f)
        ),
        Ok(F128::from_words(0x40154a4c6f8e0798, 0xe4bf08d23f9b3a40)),
        "rational-9"
    );
    assert_eq!(
        mul113_nearest(
            F128::from_words(0x403417b28ac30de6, 0xdaaa694cb8c61dc8),
            F128::from_words(0xbfc646abe76fb68a, 0x3786dcd6370c5dfe)
        ),
        Ok(F128::from_words(0xbffb64e92dd33d0e, 0x615795e84114fdac)),
        "rational-10"
    );
    assert_eq!(
        mul113_nearest(
            F128::from_words(0x3f81aed55a1aa8f2, 0xb9929c8651647899),
            F128::from_words(0xc07ac9f1bc8a8794, 0x944e553eba43ebba)
        ),
        Ok(F128::from_words(0xbffd8158d9009e06, 0x87d60950e3e2b215)),
        "rational-11"
    );
    assert_eq!(
        mul113_nearest(
            F128::from_words(0x4052293b2522b36c, 0x51ae68d346a92b28),
            F128::from_words(0x3f78be450f941f1a, 0xfafaba94d160477f)
        ),
        Ok(F128::from_words(0x3fcc03129cdcf436, 0x87fd64f983a5757a)),
        "rational-12"
    );
    assert_eq!(
        mul113_nearest(
            F128::from_words(0x4033ed930a3f5f8e, 0x9d73e514430664b1),
            F128::from_words(0xc0afa4c59f00ce24, 0xe4945d544d2d7ac9)
        ),
        Ok(F128::from_words(0xc0e495a120c35e2d, 0xa98629d8830d4e1e)),
        "rational-13"
    );
    assert_eq!(
        mul113_nearest(
            F128::from_words(0xc01aa6388b0cd844, 0x9c7146f0c0206cec),
            F128::from_words(0xc0a7bb7ec3bc4b2b, 0x238d013d590b3b44)
        ),
        Ok(F128::from_words(0x40c36dba75a3a4fd, 0xe4c341e3ef883d81)),
        "rational-14"
    );
    assert_eq!(
        mul113_nearest(
            F128::from_words(0x3f6b3feefc29d05b, 0x383484e56b2f8cee),
            F128::from_words(0x3ff9735d47b31306, 0xb8406e5ef8b1615a)
        ),
        Ok(F128::from_words(0x3f65d01beadd47f5, 0xf032d269bfaf0604)),
        "rational-15"
    );
    assert_eq!(
        mul113_nearest(
            F128::from_words(0xc06765bba6d242d5, 0x1b878e0f4a5c1679),
            F128::from_words(0xc07fa6a87c0ced04, 0xc758747c98b9636b)
        ),
        Ok(F128::from_words(0x40e8274f62be847b, 0x1c32cec9d2dc639b)),
        "rational-16"
    );
    assert_eq!(
        mul113_nearest(
            F128::from_words(0x3f87ada2f38bd6ac, 0x0979bc14034f4082),
            F128::from_words(0x3fa22cf56289f4df, 0x76496d0797838687)
        ),
        Ok(F128::from_words(0x3f2af916c7badccc, 0x435b3dbb1f543330)),
        "rational-17"
    );
    assert_eq!(
        mul113_nearest(
            F128::from_words(0x406b1c1e71f8f9d3, 0x9f83c3db94f26656),
            F128::from_words(0xc017365f17e9f056, 0x6fb38542338b72c8)
        ),
        Ok(F128::from_words(0xc083587667da289a, 0x8bcb4a76dcb0eabc)),
        "rational-18"
    );
    assert_eq!(
        mul113_nearest(
            F128::from_words(0x40b316d1d4f2f338, 0xd1997d9fc706580d),
            F128::from_words(0xbf5144bbffc68296, 0xe2294cf3edb7321f)
        ),
        Ok(F128::from_words(0xc00561ae535d40ef, 0x2b5d741de71ca450)),
        "rational-19"
    );
    assert_eq!(
        mul113_nearest(
            F128::from_words(0x4085061e0e40fd35, 0x1a9606d6b79e1731),
            F128::from_words(0xc01f6a34288afab6, 0x3361343b1a74bdcc)
        ),
        Ok(F128::from_words(0xc0a572dbe7c5ca62, 0x44df741ba0e615e8)),
        "rational-20"
    );
    assert_eq!(
        mul113_nearest(
            F128::from_words(0x4038e6299b406762, 0x39f74335d8426357),
            F128::from_words(0x408878acf8159ae3, 0x30c7403b17700db5)
        ),
        Ok(F128::from_words(0x40c265aacb8e1e89, 0x2b3eace4d643e99a)),
        "rational-21"
    );
    assert_eq!(
        mul113_nearest(
            F128::from_words(0xbf9c4603f1d0122b, 0xfa6fbedc7fb712b1),
            F128::from_words(0xc05f2c8cb710998d, 0xca54bca8286b1afa)
        ),
        Ok(F128::from_words(0x3ffc7ebfd2aa08bd, 0x40e30c5d2bd2d36f)),
        "rational-22"
    );
    assert_eq!(
        mul113_nearest(
            F128::from_words(0x3ffe55f46c23593e, 0x8b222784e8912b03),
            F128::from_words(0x3f855edb1d05a930, 0xad5056dd1cea2000)
        ),
        Ok(F128::from_words(0x3f84d4a8dab51646, 0x593fa64345b1bd7b)),
        "rational-23"
    );
    assert_eq!(
        mul113_nearest(
            F128::from_words(0xc00dc6f9731d7188, 0x88ab3f369dc88f0a),
            F128::from_words(0x3fcd0983fc817668, 0x79e30be509ff2540)
        ),
        Ok(F128::from_words(0xbfdbd7e2ca93060c, 0x14f5926f25e217d9)),
        "rational-24"
    );
    assert_eq!(
        mul113_nearest(
            F128::from_words(0xbf5319de0ca1331b, 0xfd0f1690cd8d31e0),
            F128::from_words(0xbf8cc8f264973a9a, 0xd918d134201cf6b0)
        ),
        Ok(F128::from_words(0x3ee0f71e692852db, 0x63bcd73c021dab49)),
        "rational-25"
    );
    assert_eq!(
        mul113_nearest(
            F128::from_words(0xbf8b3dc86e05c0b8, 0x6f71ce41c31671c1),
            F128::from_words(0xbf6f78e83b17518a, 0x1af08066346e5bd8)
        ),
        Ok(F128::from_words(0x3efbd3dea8964bda, 0x9b0effdfcf7f5c41)),
        "rational-26"
    );
    assert_eq!(
        mul113_nearest(
            F128::from_words(0xbfcb6e000ec540c5, 0xd6ecf142257585a3),
            F128::from_words(0xbffefb6d0c5d8b55, 0xc0c1e7628d3f1dca)
        ),
        Ok(F128::from_words(0x3fcb6abb027a5797, 0xd19a6c1d6e59ca99)),
        "rational-27"
    );
    assert_eq!(
        mul113_nearest(
            F128::from_words(0xc049a21188fe809d, 0x9ab48d201b616253),
            F128::from_words(0xbf9acc0df912452f, 0x05bd48e6edc45a9e)
        ),
        Ok(F128::from_words(0x3fe577a729e7143f, 0x35bfea304e341792)),
        "rational-28"
    );
    assert_eq!(
        mul113_nearest(
            F128::from_words(0x3fe52ef74185e00b, 0x37bd91ed0535bee7),
            F128::from_words(0xc08700a071d58a43, 0x6e540b594e598872)
        ),
        Ok(F128::from_words(0xc06d2fb522c6aefd, 0xf1a2cadf4d2afa7b)),
        "rational-29"
    );
}
#[test]
fn reference_div113_nearest() {
    assert_eq!(
        div113_nearest(
            F128::from_words(0x0000000000000000, 0x0000000000000000),
            F128::from_words(0x8000000000000000, 0x0000000000000000)
        ),
        Err(ArithmeticError::DivisionByZero),
        "zero-sign"
    );
    assert_eq!(
        div113_nearest(
            F128::from_words(0x8000000000000000, 0x0000000000000000),
            F128::from_words(0x8000000000000000, 0x0000000000000000)
        ),
        Err(ArithmeticError::DivisionByZero),
        "negative-zero"
    );
    assert_eq!(
        div113_nearest(
            F128::from_words(0x3fff000000000000, 0x0000000000000000),
            F128::from_words(0x4000800000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x3ffd555555555555, 0x5555555555555555)),
        "one-third"
    );
    assert_eq!(
        div113_nearest(
            F128::from_words(0x3fff000000000000, 0x0000000000000000),
            F128::from_words(0x3f8e000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x4070000000000000, 0x0000000000000000)),
        "quad-tie-even"
    );
    assert_eq!(
        div113_nearest(
            F128::from_words(0x3fff000000000000, 0x0000000000000001),
            F128::from_words(0x3f8e000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x4070000000000000, 0x0000000000000001)),
        "quad-tie-odd"
    );
    assert_eq!(
        div113_nearest(
            F128::from_words(0x3fff000000000000, 0x0000000000000000),
            F128::from_words(0x3fbf000000000000, 0x4000000000000000)
        ),
        Ok(F128::from_words(0x403effffffffffff, 0x8000000000002000)),
        "double-round"
    );
    assert_eq!(
        div113_nearest(
            F128::from_words(0x3fff000000000000, 0x0000400000000000),
            F128::from_words(0x3fff000000000000, 0x0000800000000000)
        ),
        Ok(F128::from_words(0x3ffeffffffffffff, 0xffff800000000000)),
        "asymmetric-right"
    );
    assert_eq!(
        div113_nearest(
            F128::from_words(0x3fff000000000000, 0x0000800000000000),
            F128::from_words(0x3fff000000000000, 0x0000400000000000)
        ),
        Ok(F128::from_words(0x3fff000000000000, 0x0000400000000000)),
        "asymmetric-left"
    );
    assert_eq!(
        div113_nearest(
            F128::from_words(0x3fff000000000000, 0x0000000000000000),
            F128::from_words(0xbc17000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0xc3e7000000000000, 0x0000000000000000)),
        "tiny-opposite"
    );
    assert_eq!(
        div113_nearest(
            F128::from_words(0xbfff000000000000, 0x0000000000000000),
            F128::from_words(0x3c17000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0xc3e7000000000000, 0x0000000000000000)),
        "negative-tiny-opposite"
    );
    assert_eq!(
        div113_nearest(
            F128::from_words(0x4001000000000000, 0x0000000000000000),
            F128::from_words(0xbfc0800000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0xc03f555555555555, 0x5555555555555555)),
        "below-binade"
    );
    assert_eq!(
        div113_nearest(
            F128::from_words(0x4001000000000000, 0x0000000000000000),
            F128::from_words(0x3fc0800000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x403f555555555555, 0x5555555555555555)),
        "above-binade"
    );
    assert_eq!(
        div113_nearest(
            F128::from_words(0x0000000000000000, 0x0000000000000003),
            F128::from_words(0x0000000000000000, 0x0000000000000004)
        ),
        Ok(F128::from_words(0x3ffe800000000000, 0x0000000000000000)),
        "subnormal-add-shortcut"
    );
    assert_eq!(
        div113_nearest(
            F128::from_words(0x0000ffffffffffff, 0xffffffffffffffff),
            F128::from_words(0x0000000000000000, 0x0000000000000002)
        ),
        Ok(F128::from_words(0x406dffffffffffff, 0xfffffffffffffffe)),
        "subnormal-normal-cross"
    );
    assert_eq!(
        div113_nearest(
            F128::from_words(0x0001000000000000, 0x0000000000000000),
            F128::from_words(0x3ff5000000000000, 0x0000400000000000)
        ),
        Ok(F128::from_words(0x000affffffffffff, 0xffff800000000000)),
        "subnormal-internal67"
    );
    assert_eq!(
        div113_nearest(
            F128::from_words(0x0000000000000000, 0x0000000000000001),
            F128::from_words(0x3ffe000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x0000000000000000, 0x0000000000000002)),
        "underflow-product"
    );
    assert_eq!(
        div113_nearest(
            F128::from_words(0x0000000000000000, 0x0000000000000001),
            F128::from_words(0x4000000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x0000000000000000, 0x0000000000000000)),
        "half-minimum"
    );
    assert_eq!(
        div113_nearest(
            F128::from_words(0x7e7f000000000000, 0x0000000000000000),
            F128::from_words(0x817f000000000000, 0x0000000000000000)
        ),
        Err(ArithmeticError::Overflow),
        "huge-opposite"
    );
    assert_eq!(
        div113_nearest(
            F128::from_words(0x3fffffffffffffff, 0xffffffffffffffff),
            F128::from_words(0x3fffffffffffffff, 0xffffffffffffffff)
        ),
        Ok(F128::from_words(0x3fff000000000000, 0x0000000000000000)),
        "wide-product"
    );
    assert_eq!(
        div113_nearest(
            F128::from_words(0x3f976e0364caa050, 0x8b88599668a6328e),
            F128::from_words(0x407ec4406dfade38, 0x9adc2542fa7422b1)
        ),
        Ok(F128::from_words(0x3f179e5e4d058a92, 0x643b9f788dab465f)),
        "rational-0"
    );
    assert_eq!(
        div113_nearest(
            F128::from_words(0x401dd161ab6e8e1d, 0xd23ab5033ba5120a),
            F128::from_words(0xc0a29f9cba4fbdeb, 0x5582b8a652fe4854)
        ),
        Ok(F128::from_words(0xbf7a1ea7e52a55f4, 0x95a81e2d74f18c1e)),
        "rational-1"
    );
    assert_eq!(
        div113_nearest(
            F128::from_words(0x3f7526f6860ee0ad, 0x8eb452c36cff7606),
            F128::from_words(0x403e55319397bf4c, 0x338950d18897ce69)
        ),
        Ok(F128::from_words(0x3f35baa0273e3e13, 0x98acc6533691e091)),
        "rational-2"
    );
    assert_eq!(
        div113_nearest(
            F128::from_words(0x407ed0e5d4fcde8a, 0x5a07b21766575899),
            F128::from_words(0x402f31d543d75467, 0x8f2d1c302929ce5c)
        ),
        Ok(F128::from_words(0x404e8525797037e9, 0x2215f8b871bcd611)),
        "rational-3"
    );
    assert_eq!(
        div113_nearest(
            F128::from_words(0xbf6fa93875ccd29c, 0xf4a4b1d2823a55be),
            F128::from_words(0x4046687a91deaa23, 0x80e6b1de6e41b28b)
        ),
        Ok(F128::from_words(0xbf282dfa393bd997, 0x3df998ac999f0201)),
        "rational-4"
    );
    assert_eq!(
        div113_nearest(
            F128::from_words(0x40563949f3796c30, 0xc257681164d2a50c),
            F128::from_words(0x4019b1ee9579f94c, 0x4eaed9ca369d4c37)
        ),
        Ok(F128::from_words(0x403b71a6f5e26894, 0xd1f1d274e72c1c6a)),
        "rational-5"
    );
    assert_eq!(
        div113_nearest(
            F128::from_words(0x40a0d40f4dedffc8, 0x4872f8cf5ea16b82),
            F128::from_words(0xbf8cbf37785626ca, 0x59ec4e6748db3775)
        ),
        Ok(F128::from_words(0xc1130bee6384b339, 0x419d27b39a65add1)),
        "rational-6"
    );
    assert_eq!(
        div113_nearest(
            F128::from_words(0x40218c303fcc2682, 0x10e9a7dfa0c84c0c),
            F128::from_words(0x4096305c412a05f3, 0xe55ce2d5da562d0f)
        ),
        Ok(F128::from_words(0x3f8a4d3cc452eb11, 0x654ca7589467750d)),
        "rational-7"
    );
    assert_eq!(
        div113_nearest(
            F128::from_words(0xc018b179d909cb04, 0xba30a83aeb3d10d0),
            F128::from_words(0x40b2ede9d1d55254, 0x740d7f67c593d6da)
        ),
        Ok(F128::from_words(0xbf64c15975881f48, 0xc8c42616f827ab8c)),
        "rational-8"
    );
    assert_eq!(
        div113_nearest(
            F128::from_words(0xbff78794a804f48f, 0xc2b2277ae5448dac),
            F128::from_words(0xc01cafdf5234216b, 0xc3b248e12c219f6f)
        ),
        Ok(F128::from_words(0x3fd9d03b99468b80, 0x658f7428d8379426)),
        "rational-9"
    );
    assert_eq!(
        div113_nearest(
            F128::from_words(0x403417b28ac30de6, 0xdaaa694cb8c61dc8),
            F128::from_words(0xbfc646abe76fb68a, 0x3786dcd6370c5dfe)
        ),
        Ok(F128::from_words(0xc06cb6605d52dc91, 0x3c7422d7bb5dc1a7)),
        "rational-10"
    );
    assert_eq!(
        div113_nearest(
            F128::from_words(0x3f81aed55a1aa8f2, 0xb9929c8651647899),
            F128::from_words(0xc07ac9f1bc8a8794, 0x944e553eba43ebba)
        ),
        Ok(F128::from_words(0xbf05e1b05f75ca11, 0x30937bf5cc4c0ae4)),
        "rational-11"
    );
    assert_eq!(
        div113_nearest(
            F128::from_words(0x4052293b2522b36c, 0x51ae68d346a92b28),
            F128::from_words(0x3f78be450f941f1a, 0xfafaba94d160477f)
        ),
        Ok(F128::from_words(0x40d85502774e847e, 0xeabffe72c738e248)),
        "rational-12"
    );
    assert_eq!(
        div113_nearest(
            F128::from_words(0x4033ed930a3f5f8e, 0x9d73e514430664b1),
            F128::from_words(0xc0afa4c59f00ce24, 0xe4945d544d2d7ac9)
        ),
        Ok(F128::from_words(0xbf832c4b1bff885c, 0x31bba7cb248ebed7)),
        "rational-13"
    );
    assert_eq!(
        div113_nearest(
            F128::from_words(0xc01aa6388b0cd844, 0x9c7146f0c0206cec),
            F128::from_words(0xc0a7bb7ec3bc4b2b, 0x238d013d590b3b44)
        ),
        Ok(F128::from_words(0x3f71e77086151212, 0x527ea648f0aca5fc)),
        "rational-14"
    );
    assert_eq!(
        div113_nearest(
            F128::from_words(0x3f6b3feefc29d05b, 0x383484e56b2f8cee),
            F128::from_words(0x3ff9735d47b31306, 0xb8406e5ef8b1615a)
        ),
        Ok(F128::from_words(0x3f70b917a19b2147, 0xc2542944e2127d67)),
        "rational-15"
    );
    assert_eq!(
        div113_nearest(
            F128::from_words(0xc06765bba6d242d5, 0x1b878e0f4a5c1679),
            F128::from_words(0xc07fa6a87c0ced04, 0xc758747c98b9636b)
        ),
        Ok(F128::from_words(0x3fe6b159d72ba0fd, 0x04d0ffd44e9bea98)),
        "rational-16"
    );
    assert_eq!(
        div113_nearest(
            F128::from_words(0x3f87ada2f38bd6ac, 0x0979bc14034f4082),
            F128::from_words(0x3fa22cf56289f4df, 0x76496d0797838687)
        ),
        Ok(F128::from_words(0x3fe46d749b8b6eb0, 0x13ddf88a6759413a)),
        "rational-17"
    );
    assert_eq!(
        div113_nearest(
            F128::from_words(0x406b1c1e71f8f9d3, 0x9f83c3db94f26656),
            F128::from_words(0xc017365f17e9f056, 0x6fb38542338b72c8)
        ),
        Ok(F128::from_words(0xc052d4b160130b25, 0x815550c2fd34de07)),
        "rational-18"
    );
    assert_eq!(
        div113_nearest(
            F128::from_words(0x40b316d1d4f2f338, 0xd1997d9fc706580d),
            F128::from_words(0xbf5144bbffc68296, 0xe2294cf3edb7321f)
        ),
        Ok(F128::from_words(0xc160b79b84ccb82a, 0xfc1dafbd7f07918b)),
        "rational-19"
    );
    assert_eq!(
        div113_nearest(
            F128::from_words(0x4085061e0e40fd35, 0x1a9606d6b79e1731),
            F128::from_words(0xc01f6a34288afab6, 0x3361343b1a74bdcc)
        ),
        Ok(F128::from_words(0xc064728563863381, 0x866caca057f7da2d)),
        "rational-20"
    );
    assert_eq!(
        div113_nearest(
            F128::from_words(0x4038e6299b406762, 0x39f74335d8426357),
            F128::from_words(0x408878acf8159ae3, 0x30c7403b17700db5)
        ),
        Ok(F128::from_words(0x3faf4a6918d05d27, 0xb19fbb95305db8cd)),
        "rational-21"
    );
    assert_eq!(
        div113_nearest(
            F128::from_words(0xbf9c4603f1d0122b, 0xfa6fbedc7fb712b1),
            F128::from_words(0xc05f2c8cb710998d, 0xca54bca8286b1afa)
        ),
        Ok(F128::from_words(0x3f3c15b0e6d23ded, 0xf6a37fb0b901ea4c)),
        "rational-22"
    );
    assert_eq!(
        div113_nearest(
            F128::from_words(0x3ffe55f46c23593e, 0x8b222784e8912b03),
            F128::from_words(0x3f855edb1d05a930, 0xad5056dd1cea2000)
        ),
        Ok(F128::from_words(0x4077f302bb839d1d, 0x906fca00caf444d5)),
        "rational-23"
    );
    assert_eq!(
        div113_nearest(
            F128::from_words(0xc00dc6f9731d7188, 0x88ab3f369dc88f0a),
            F128::from_words(0x3fcd0983fc817668, 0x79e30be509ff2540)
        ),
        Ok(F128::from_words(0xc03fb6ab436cb11d, 0xe984e35dc218020e)),
        "rational-24"
    );
    assert_eq!(
        div113_nearest(
            F128::from_words(0xbf5319de0ca1331b, 0xfd0f1690cd8d31e0),
            F128::from_words(0xbf8cc8f264973a9a, 0xd918d134201cf6b0)
        ),
        Ok(F128::from_words(0x3fc53bd3aea098b8, 0x1285b9ad50f9db4a)),
        "rational-25"
    );
    assert_eq!(
        div113_nearest(
            F128::from_words(0xbf8b3dc86e05c0b8, 0x6f71ce41c31671c1),
            F128::from_words(0xbf6f78e83b17518a, 0x1af08066346e5bd8)
        ),
        Ok(F128::from_words(0x401aafaf2743904a, 0xb16ecf75200527b8)),
        "rational-26"
    );
    assert_eq!(
        div113_nearest(
            F128::from_words(0xbfcb6e000ec540c5, 0xd6ecf142257585a3),
            F128::from_words(0xbffefb6d0c5d8b55, 0xc0c1e7628d3f1dca)
        ),
        Ok(F128::from_words(0x3fcb714ca6a927d2, 0x03964b96cd82c45e)),
        "rational-27"
    );
    assert_eq!(
        div113_nearest(
            F128::from_words(0xc049a21188fe809d, 0x9ab48d201b616253),
            F128::from_words(0xbf9acc0df912452f, 0x05bd48e6edc45a9e)
        ),
        Ok(F128::from_words(0x40add145f0d68fd1, 0xc9cb46120c00ae50)),
        "rational-28"
    );
    assert_eq!(
        div113_nearest(
            F128::from_words(0x3fe52ef74185e00b, 0x37bd91ed0535bee7),
            F128::from_words(0xc08700a071d58a43, 0x6e540b594e598872)
        ),
        Ok(F128::from_words(0xbf5d2e39d6fbe15e, 0x8555e1968e231d9d)),
        "rational-29"
    );
}
#[test]
fn reference_add67_chop() {
    assert_eq!(
        add67_chop(
            F128::from_words(0x0000000000000000, 0x0000000000000000),
            F128::from_words(0x8000000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x0000000000000000, 0x0000000000000000)),
        "zero-sign"
    );
    assert_eq!(
        add67_chop(
            F128::from_words(0x8000000000000000, 0x0000000000000000),
            F128::from_words(0x8000000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x8000000000000000, 0x0000000000000000)),
        "negative-zero"
    );
    assert_eq!(
        add67_chop(
            F128::from_words(0x3fff000000000000, 0x0000000000000000),
            F128::from_words(0x4000800000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x4001000000000000, 0x0000000000000000)),
        "one-third"
    );
    assert_eq!(
        add67_chop(
            F128::from_words(0x3fff000000000000, 0x0000000000000000),
            F128::from_words(0x3f8e000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x3fff000000000000, 0x0000000000000000)),
        "quad-tie-even"
    );
    assert_eq!(
        add67_chop(
            F128::from_words(0x3fff000000000000, 0x0000000000000001),
            F128::from_words(0x3f8e000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x3fff000000000000, 0x0000000000000000)),
        "quad-tie-odd"
    );
    assert_eq!(
        add67_chop(
            F128::from_words(0x3fff000000000000, 0x0000000000000000),
            F128::from_words(0x3fbf000000000000, 0x4000000000000000)
        ),
        Ok(F128::from_words(0x3fff000000000000, 0x0001000000000000)),
        "double-round"
    );
    assert_eq!(
        add67_chop(
            F128::from_words(0x3fff000000000000, 0x0000400000000000),
            F128::from_words(0x3fff000000000000, 0x0000800000000000)
        ),
        Ok(F128::from_words(0x4000000000000000, 0x0000400000000000)),
        "asymmetric-right"
    );
    assert_eq!(
        add67_chop(
            F128::from_words(0x3fff000000000000, 0x0000800000000000),
            F128::from_words(0x3fff000000000000, 0x0000400000000000)
        ),
        Ok(F128::from_words(0x4000000000000000, 0x0000400000000000)),
        "asymmetric-left"
    );
    assert_eq!(
        add67_chop(
            F128::from_words(0x3fff000000000000, 0x0000000000000000),
            F128::from_words(0xbc17000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x3ffeffffffffffff, 0xffffc00000000000)),
        "tiny-opposite"
    );
    assert_eq!(
        add67_chop(
            F128::from_words(0xbfff000000000000, 0x0000000000000000),
            F128::from_words(0x3c17000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0xbffeffffffffffff, 0xffffc00000000000)),
        "negative-tiny-opposite"
    );
    assert_eq!(
        add67_chop(
            F128::from_words(0x4001000000000000, 0x0000000000000000),
            F128::from_words(0xbfc0800000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x4000ffffffffffff, 0xfffe800000000000)),
        "below-binade"
    );
    assert_eq!(
        add67_chop(
            F128::from_words(0x4001000000000000, 0x0000000000000000),
            F128::from_words(0x3fc0800000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x4001000000000000, 0x0000c00000000000)),
        "above-binade"
    );
    assert_eq!(
        add67_chop(
            F128::from_words(0x0000000000000000, 0x0000000000000003),
            F128::from_words(0x0000000000000000, 0x0000000000000004)
        ),
        Ok(F128::from_words(0x0000000000000000, 0x0000000000000007)),
        "subnormal-add-shortcut"
    );
    assert_eq!(
        add67_chop(
            F128::from_words(0x0000ffffffffffff, 0xffffffffffffffff),
            F128::from_words(0x0000000000000000, 0x0000000000000002)
        ),
        Ok(F128::from_words(0x0001000000000000, 0x0000000000000001)),
        "subnormal-normal-cross"
    );
    assert_eq!(
        add67_chop(
            F128::from_words(0x0001000000000000, 0x0000000000000000),
            F128::from_words(0x3ff5000000000000, 0x0000400000000000)
        ),
        Ok(F128::from_words(0x3ff5000000000000, 0x0000400000000000)),
        "subnormal-internal67"
    );
    assert_eq!(
        add67_chop(
            F128::from_words(0x0000000000000000, 0x0000000000000001),
            F128::from_words(0x3ffe000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x3ffe000000000000, 0x0000000000000000)),
        "underflow-product"
    );
    assert_eq!(
        add67_chop(
            F128::from_words(0x0000000000000000, 0x0000000000000001),
            F128::from_words(0x4000000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x4000000000000000, 0x0000000000000000)),
        "half-minimum"
    );
    assert_eq!(
        add67_chop(
            F128::from_words(0x7e7f000000000000, 0x0000000000000000),
            F128::from_words(0x817f000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x7e7effffffffffff, 0xffffc00000000000)),
        "huge-opposite"
    );
    assert_eq!(
        add67_chop(
            F128::from_words(0x3fffffffffffffff, 0xffffffffffffffff),
            F128::from_words(0x3fffffffffffffff, 0xffffffffffffffff)
        ),
        Ok(F128::from_words(0x4000ffffffffffff, 0xffffc00000000000)),
        "wide-product"
    );
    assert_eq!(
        add67_chop(
            F128::from_words(0x3f976e0364caa050, 0x8b88599668a6328e),
            F128::from_words(0x407ec4406dfade38, 0x9adc2542fa7422b1)
        ),
        Ok(F128::from_words(0x407ec4406dfade38, 0x9adc000000000000)),
        "rational-0"
    );
    assert_eq!(
        add67_chop(
            F128::from_words(0x401dd161ab6e8e1d, 0xd23ab5033ba5120a),
            F128::from_words(0xc0a29f9cba4fbdeb, 0x5582b8a652fe4854)
        ),
        Ok(F128::from_words(0xc0a29f9cba4fbdeb, 0x5582800000000000)),
        "rational-1"
    );
    assert_eq!(
        add67_chop(
            F128::from_words(0x3f7526f6860ee0ad, 0x8eb452c36cff7606),
            F128::from_words(0x403e55319397bf4c, 0x338950d18897ce69)
        ),
        Ok(F128::from_words(0x403e55319397bf4c, 0x3389400000000000)),
        "rational-2"
    );
    assert_eq!(
        add67_chop(
            F128::from_words(0x407ed0e5d4fcde8a, 0x5a07b21766575899),
            F128::from_words(0x402f31d543d75467, 0x8f2d1c302929ce5c)
        ),
        Ok(F128::from_words(0x407ed0e5d4fcde8a, 0x5a07800000000000)),
        "rational-3"
    );
    assert_eq!(
        add67_chop(
            F128::from_words(0xbf6fa93875ccd29c, 0xf4a4b1d2823a55be),
            F128::from_words(0x4046687a91deaa23, 0x80e6b1de6e41b28b)
        ),
        Ok(F128::from_words(0x4046687a91deaa23, 0x80e6800000000000)),
        "rational-4"
    );
    assert_eq!(
        add67_chop(
            F128::from_words(0x40563949f3796c30, 0xc257681164d2a50c),
            F128::from_words(0x4019b1ee9579f94c, 0x4eaed9ca369d4c37)
        ),
        Ok(F128::from_words(0x40563949f3796c30, 0xc264c00000000000)),
        "rational-5"
    );
    assert_eq!(
        add67_chop(
            F128::from_words(0x40a0d40f4dedffc8, 0x4872f8cf5ea16b82),
            F128::from_words(0xbf8cbf37785626ca, 0x59ec4e6748db3775)
        ),
        Ok(F128::from_words(0x40a0d40f4dedffc8, 0x4872c00000000000)),
        "rational-6"
    );
    assert_eq!(
        add67_chop(
            F128::from_words(0x40218c303fcc2682, 0x10e9a7dfa0c84c0c),
            F128::from_words(0x4096305c412a05f3, 0xe55ce2d5da562d0f)
        ),
        Ok(F128::from_words(0x4096305c412a05f3, 0xe55cc00000000000)),
        "rational-7"
    );
    assert_eq!(
        add67_chop(
            F128::from_words(0xc018b179d909cb04, 0xba30a83aeb3d10d0),
            F128::from_words(0x40b2ede9d1d55254, 0x740d7f67c593d6da)
        ),
        Ok(F128::from_words(0x40b2ede9d1d55254, 0x740d400000000000)),
        "rational-8"
    );
    assert_eq!(
        add67_chop(
            F128::from_words(0xbff78794a804f48f, 0xc2b2277ae5448dac),
            F128::from_words(0xc01cafdf5234216b, 0xc3b248e12c219f6f)
        ),
        Ok(F128::from_words(0xc01cafdf52342da8, 0x68f2400000000000)),
        "rational-9"
    );
    assert_eq!(
        add67_chop(
            F128::from_words(0x403417b28ac30de6, 0xdaaa694cb8c61dc8),
            F128::from_words(0xbfc646abe76fb68a, 0x3786dcd6370c5dfe)
        ),
        Ok(F128::from_words(0x403417b28ac30de6, 0xdaaa400000000000)),
        "rational-10"
    );
    assert_eq!(
        add67_chop(
            F128::from_words(0x3f81aed55a1aa8f2, 0xb9929c8651647899),
            F128::from_words(0xc07ac9f1bc8a8794, 0x944e553eba43ebba)
        ),
        Ok(F128::from_words(0xc07ac9f1bc8a8794, 0x944e400000000000)),
        "rational-11"
    );
    assert_eq!(
        add67_chop(
            F128::from_words(0x4052293b2522b36c, 0x51ae68d346a92b28),
            F128::from_words(0x3f78be450f941f1a, 0xfafaba94d160477f)
        ),
        Ok(F128::from_words(0x4052293b2522b36c, 0x51ae400000000000)),
        "rational-12"
    );
    assert_eq!(
        add67_chop(
            F128::from_words(0x4033ed930a3f5f8e, 0x9d73e514430664b1),
            F128::from_words(0xc0afa4c59f00ce24, 0xe4945d544d2d7ac9)
        ),
        Ok(F128::from_words(0xc0afa4c59f00ce24, 0xe494400000000000)),
        "rational-13"
    );
    assert_eq!(
        add67_chop(
            F128::from_words(0xc01aa6388b0cd844, 0x9c7146f0c0206cec),
            F128::from_words(0xc0a7bb7ec3bc4b2b, 0x238d013d590b3b44)
        ),
        Ok(F128::from_words(0xc0a7bb7ec3bc4b2b, 0x238d000000000000)),
        "rational-14"
    );
    assert_eq!(
        add67_chop(
            F128::from_words(0x3f6b3feefc29d05b, 0x383484e56b2f8cee),
            F128::from_words(0x3ff9735d47b31306, 0xb8406e5ef8b1615a)
        ),
        Ok(F128::from_words(0x3ff9735d47b31306, 0xb840400000000000)),
        "rational-15"
    );
    assert_eq!(
        add67_chop(
            F128::from_words(0xc06765bba6d242d5, 0x1b878e0f4a5c1679),
            F128::from_words(0xc07fa6a87c0ced04, 0xc758747c98b9636b)
        ),
        Ok(F128::from_words(0xc07fa6a87d72a8ab, 0x999b400000000000)),
        "rational-16"
    );
    assert_eq!(
        add67_chop(
            F128::from_words(0x3f87ada2f38bd6ac, 0x0979bc14034f4082),
            F128::from_words(0x3fa22cf56289f4df, 0x76496d0797838687)
        ),
        Ok(F128::from_words(0x3fa22cf562bfa93d, 0xe7c4400000000000)),
        "rational-17"
    );
    assert_eq!(
        add67_chop(
            F128::from_words(0x406b1c1e71f8f9d3, 0x9f83c3db94f26656),
            F128::from_words(0xc017365f17e9f056, 0x6fb38542338b72c8)
        ),
        Ok(F128::from_words(0x406b1c1e71f8f9d3, 0x9f83c00000000000)),
        "rational-18"
    );
    assert_eq!(
        add67_chop(
            F128::from_words(0x40b316d1d4f2f338, 0xd1997d9fc706580d),
            F128::from_words(0xbf5144bbffc68296, 0xe2294cf3edb7321f)
        ),
        Ok(F128::from_words(0x40b316d1d4f2f338, 0xd199400000000000)),
        "rational-19"
    );
    assert_eq!(
        add67_chop(
            F128::from_words(0x4085061e0e40fd35, 0x1a9606d6b79e1731),
            F128::from_words(0xc01f6a34288afab6, 0x3361343b1a74bdcc)
        ),
        Ok(F128::from_words(0x4085061e0e40fd35, 0x1a96000000000000)),
        "rational-20"
    );
    assert_eq!(
        add67_chop(
            F128::from_words(0x4038e6299b406762, 0x39f74335d8426357),
            F128::from_words(0x408878acf8159ae3, 0x30c7403b17700db5)
        ),
        Ok(F128::from_words(0x408878acf8159ae3, 0x30c7400000000000)),
        "rational-21"
    );
    assert_eq!(
        add67_chop(
            F128::from_words(0xbf9c4603f1d0122b, 0xfa6fbedc7fb712b1),
            F128::from_words(0xc05f2c8cb710998d, 0xca54bca8286b1afa)
        ),
        Ok(F128::from_words(0xc05f2c8cb710998d, 0xca54800000000000)),
        "rational-22"
    );
    assert_eq!(
        add67_chop(
            F128::from_words(0x3ffe55f46c23593e, 0x8b222784e8912b03),
            F128::from_words(0x3f855edb1d05a930, 0xad5056dd1cea2000)
        ),
        Ok(F128::from_words(0x3ffe55f46c23593e, 0x8b22000000000000)),
        "rational-23"
    );
    assert_eq!(
        add67_chop(
            F128::from_words(0xc00dc6f9731d7188, 0x88ab3f369dc88f0a),
            F128::from_words(0x3fcd0983fc817668, 0x79e30be509ff2540)
        ),
        Ok(F128::from_words(0xc00dc6f9731d7188, 0x88aa000000000000)),
        "rational-24"
    );
    assert_eq!(
        add67_chop(
            F128::from_words(0xbf5319de0ca1331b, 0xfd0f1690cd8d31e0),
            F128::from_words(0xbf8cc8f264973a9a, 0xd918d134201cf6b0)
        ),
        Ok(F128::from_words(0xbf8cc8f264973a9a, 0xd9a5c00000000000)),
        "rational-25"
    );
    assert_eq!(
        add67_chop(
            F128::from_words(0xbf8b3dc86e05c0b8, 0x6f71ce41c31671c1),
            F128::from_words(0xbf6f78e83b17518a, 0x1af08066346e5bd8)
        ),
        Ok(F128::from_words(0xbf8b3dc86e1d4f3c, 0x20e6c00000000000)),
        "rational-26"
    );
    assert_eq!(
        add67_chop(
            F128::from_words(0xbfcb6e000ec540c5, 0xd6ecf142257585a3),
            F128::from_words(0xbffefb6d0c5d8b55, 0xc0c1e7628d3f1dca)
        ),
        Ok(F128::from_words(0xbffefb6d0c5d8b55, 0xee81c00000000000)),
        "rational-27"
    );
    assert_eq!(
        add67_chop(
            F128::from_words(0xc049a21188fe809d, 0x9ab48d201b616253),
            F128::from_words(0xbf9acc0df912452f, 0x05bd48e6edc45a9e)
        ),
        Ok(F128::from_words(0xc049a21188fe809d, 0x9ab4800000000000)),
        "rational-28"
    );
    assert_eq!(
        add67_chop(
            F128::from_words(0x3fe52ef74185e00b, 0x37bd91ed0535bee7),
            F128::from_words(0xc08700a071d58a43, 0x6e540b594e598872)
        ),
        Ok(F128::from_words(0xc08700a071d58a43, 0x6e54000000000000)),
        "rational-29"
    );
}
#[test]
fn reference_mul67_chop() {
    assert_eq!(
        mul67_chop(
            F128::from_words(0x0000000000000000, 0x0000000000000000),
            F128::from_words(0x8000000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x8000000000000000, 0x0000000000000000)),
        "zero-sign"
    );
    assert_eq!(
        mul67_chop(
            F128::from_words(0x8000000000000000, 0x0000000000000000),
            F128::from_words(0x8000000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x0000000000000000, 0x0000000000000000)),
        "negative-zero"
    );
    assert_eq!(
        mul67_chop(
            F128::from_words(0x3fff000000000000, 0x0000000000000000),
            F128::from_words(0x4000800000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x4000800000000000, 0x0000000000000000)),
        "one-third"
    );
    assert_eq!(
        mul67_chop(
            F128::from_words(0x3fff000000000000, 0x0000000000000000),
            F128::from_words(0x3f8e000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x3f8e000000000000, 0x0000000000000000)),
        "quad-tie-even"
    );
    assert_eq!(
        mul67_chop(
            F128::from_words(0x3fff000000000000, 0x0000000000000001),
            F128::from_words(0x3f8e000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x3f8e000000000000, 0x0000000000000000)),
        "quad-tie-odd"
    );
    assert_eq!(
        mul67_chop(
            F128::from_words(0x3fff000000000000, 0x0000000000000000),
            F128::from_words(0x3fbf000000000000, 0x4000000000000000)
        ),
        Ok(F128::from_words(0x3fbf000000000000, 0x4000000000000000)),
        "double-round"
    );
    assert_eq!(
        mul67_chop(
            F128::from_words(0x3fff000000000000, 0x0000400000000000),
            F128::from_words(0x3fff000000000000, 0x0000800000000000)
        ),
        Ok(F128::from_words(0x3fff000000000000, 0x0000c00000000000)),
        "asymmetric-right"
    );
    assert_eq!(
        mul67_chop(
            F128::from_words(0x3fff000000000000, 0x0000800000000000),
            F128::from_words(0x3fff000000000000, 0x0000400000000000)
        ),
        Ok(F128::from_words(0x3fff000000000000, 0x0000c00000000000)),
        "asymmetric-left"
    );
    assert_eq!(
        mul67_chop(
            F128::from_words(0x3fff000000000000, 0x0000000000000000),
            F128::from_words(0xbc17000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0xbc17000000000000, 0x0000000000000000)),
        "tiny-opposite"
    );
    assert_eq!(
        mul67_chop(
            F128::from_words(0xbfff000000000000, 0x0000000000000000),
            F128::from_words(0x3c17000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0xbc17000000000000, 0x0000000000000000)),
        "negative-tiny-opposite"
    );
    assert_eq!(
        mul67_chop(
            F128::from_words(0x4001000000000000, 0x0000000000000000),
            F128::from_words(0xbfc0800000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0xbfc2800000000000, 0x0000000000000000)),
        "below-binade"
    );
    assert_eq!(
        mul67_chop(
            F128::from_words(0x4001000000000000, 0x0000000000000000),
            F128::from_words(0x3fc0800000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x3fc2800000000000, 0x0000000000000000)),
        "above-binade"
    );
    assert_eq!(
        mul67_chop(
            F128::from_words(0x0000000000000000, 0x0000000000000003),
            F128::from_words(0x0000000000000000, 0x0000000000000004)
        ),
        Ok(F128::from_words(0x0000000000000000, 0x0000000000000000)),
        "subnormal-add-shortcut"
    );
    assert_eq!(
        mul67_chop(
            F128::from_words(0x0000ffffffffffff, 0xffffffffffffffff),
            F128::from_words(0x0000000000000000, 0x0000000000000002)
        ),
        Ok(F128::from_words(0x0000000000000000, 0x0000000000000000)),
        "subnormal-normal-cross"
    );
    assert_eq!(
        mul67_chop(
            F128::from_words(0x0001000000000000, 0x0000000000000000),
            F128::from_words(0x3ff5000000000000, 0x0000400000000000)
        ),
        Ok(F128::from_words(0x0000004000000000, 0x0000001000000000)),
        "subnormal-internal67"
    );
    assert_eq!(
        mul67_chop(
            F128::from_words(0x0000000000000000, 0x0000000000000001),
            F128::from_words(0x3ffe000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x0000000000000000, 0x0000000000000000)),
        "underflow-product"
    );
    assert_eq!(
        mul67_chop(
            F128::from_words(0x0000000000000000, 0x0000000000000001),
            F128::from_words(0x4000000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x0000000000000000, 0x0000000000000002)),
        "half-minimum"
    );
    assert_eq!(
        mul67_chop(
            F128::from_words(0x7e7f000000000000, 0x0000000000000000),
            F128::from_words(0x817f000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0xbfff000000000000, 0x0000000000000000)),
        "huge-opposite"
    );
    assert_eq!(
        mul67_chop(
            F128::from_words(0x3fffffffffffffff, 0xffffffffffffffff),
            F128::from_words(0x3fffffffffffffff, 0xffffffffffffffff)
        ),
        Ok(F128::from_words(0x4000ffffffffffff, 0xffffc00000000000)),
        "wide-product"
    );
    assert_eq!(
        mul67_chop(
            F128::from_words(0x3f976e0364caa050, 0x8b88599668a6328e),
            F128::from_words(0x407ec4406dfade38, 0x9adc2542fa7422b1)
        ),
        Ok(F128::from_words(0x4017434d0e068a54, 0x6f18800000000000)),
        "rational-0"
    );
    assert_eq!(
        mul67_chop(
            F128::from_words(0x401dd161ab6e8e1d, 0xd23ab5033ba5120a),
            F128::from_words(0xc0a29f9cba4fbdeb, 0x5582b8a652fe4854)
        ),
        Ok(F128::from_words(0xc0c179c51f8f4e5a, 0x0c02400000000000)),
        "rational-1"
    );
    assert_eq!(
        mul67_chop(
            F128::from_words(0x3f7526f6860ee0ad, 0x8eb452c36cff7606),
            F128::from_words(0x403e55319397bf4c, 0x338950d18897ce69)
        ),
        Ok(F128::from_words(0x3fb4891f7fcfe0ed, 0x88aec00000000000)),
        "rational-2"
    );
    assert_eq!(
        mul67_chop(
            F128::from_words(0x407ed0e5d4fcde8a, 0x5a07b21766575899),
            F128::from_words(0x402f31d543d75467, 0x8f2d1c302929ce5c)
        ),
        Ok(F128::from_words(0x40af15b28e975649, 0xc3ad800000000000)),
        "rational-3"
    );
    assert_eq!(
        mul67_chop(
            F128::from_words(0xbf6fa93875ccd29c, 0xf4a4b1d2823a55be),
            F128::from_words(0x4046687a91deaa23, 0x80e6b1de6e41b28b)
        ),
        Ok(F128::from_words(0xbfb72b617e6d8009, 0x5c65800000000000)),
        "rational-4"
    );
    assert_eq!(
        mul67_chop(
            F128::from_words(0x40563949f3796c30, 0xc257681164d2a50c),
            F128::from_words(0x4019b1ee9579f94c, 0x4eaed9ca369d4c37)
        ),
        Ok(F128::from_words(0x40710985073f0550, 0x8f3b400000000000)),
        "rational-5"
    );
    assert_eq!(
        mul67_chop(
            F128::from_words(0x40a0d40f4dedffc8, 0x4872f8cf5ea16b82),
            F128::from_words(0xbf8cbf37785626ca, 0x59ec4e6748db3775)
        ),
        Ok(F128::from_words(0xc02e98d61230802c, 0x5338000000000000)),
        "rational-6"
    );
    assert_eq!(
        mul67_chop(
            F128::from_words(0x40218c303fcc2682, 0x10e9a7dfa0c84c0c),
            F128::from_words(0x4096305c412a05f3, 0xe55ce2d5da562d0f)
        ),
        Ok(F128::from_words(0x40b8d70811f2a46b, 0xfe1b400000000000)),
        "rational-7"
    );
    assert_eq!(
        mul67_chop(
            F128::from_words(0xc018b179d909cb04, 0xba30a83aeb3d10d0),
            F128::from_words(0x40b2ede9d1d55254, 0x740d7f67c593d6da)
        ),
        Ok(F128::from_words(0xc0cca229c9160aee, 0x3ee2000000000000)),
        "rational-8"
    );
    assert_eq!(
        mul67_chop(
            F128::from_words(0xbff78794a804f48f, 0xc2b2277ae5448dac),
            F128::from_words(0xc01cafdf5234216b, 0xc3b248e12c219f6f)
        ),
        Ok(F128::from_words(0x40154a4c6f8e0798, 0xe4bf000000000000)),
        "rational-9"
    );
    assert_eq!(
        mul67_chop(
            F128::from_words(0x403417b28ac30de6, 0xdaaa694cb8c61dc8),
            F128::from_words(0xbfc646abe76fb68a, 0x3786dcd6370c5dfe)
        ),
        Ok(F128::from_words(0xbffb64e92dd33d0e, 0x6157800000000000)),
        "rational-10"
    );
    assert_eq!(
        mul67_chop(
            F128::from_words(0x3f81aed55a1aa8f2, 0xb9929c8651647899),
            F128::from_words(0xc07ac9f1bc8a8794, 0x944e553eba43ebba)
        ),
        Ok(F128::from_words(0xbffd8158d9009e06, 0x87d6000000000000)),
        "rational-11"
    );
    assert_eq!(
        mul67_chop(
            F128::from_words(0x4052293b2522b36c, 0x51ae68d346a92b28),
            F128::from_words(0x3f78be450f941f1a, 0xfafaba94d160477f)
        ),
        Ok(F128::from_words(0x3fcc03129cdcf436, 0x87fd400000000000)),
        "rational-12"
    );
    assert_eq!(
        mul67_chop(
            F128::from_words(0x4033ed930a3f5f8e, 0x9d73e514430664b1),
            F128::from_words(0xc0afa4c59f00ce24, 0xe4945d544d2d7ac9)
        ),
        Ok(F128::from_words(0xc0e495a120c35e2d, 0xa986000000000000)),
        "rational-13"
    );
    assert_eq!(
        mul67_chop(
            F128::from_words(0xc01aa6388b0cd844, 0x9c7146f0c0206cec),
            F128::from_words(0xc0a7bb7ec3bc4b2b, 0x238d013d590b3b44)
        ),
        Ok(F128::from_words(0x40c36dba75a3a4fd, 0xe4c3400000000000)),
        "rational-14"
    );
    assert_eq!(
        mul67_chop(
            F128::from_words(0x3f6b3feefc29d05b, 0x383484e56b2f8cee),
            F128::from_words(0x3ff9735d47b31306, 0xb8406e5ef8b1615a)
        ),
        Ok(F128::from_words(0x3f65d01beadd47f5, 0xf032c00000000000)),
        "rational-15"
    );
    assert_eq!(
        mul67_chop(
            F128::from_words(0xc06765bba6d242d5, 0x1b878e0f4a5c1679),
            F128::from_words(0xc07fa6a87c0ced04, 0xc758747c98b9636b)
        ),
        Ok(F128::from_words(0x40e8274f62be847b, 0x1c32c00000000000)),
        "rational-16"
    );
    assert_eq!(
        mul67_chop(
            F128::from_words(0x3f87ada2f38bd6ac, 0x0979bc14034f4082),
            F128::from_words(0x3fa22cf56289f4df, 0x76496d0797838687)
        ),
        Ok(F128::from_words(0x3f2af916c7badccc, 0x435b000000000000)),
        "rational-17"
    );
    assert_eq!(
        mul67_chop(
            F128::from_words(0x406b1c1e71f8f9d3, 0x9f83c3db94f26656),
            F128::from_words(0xc017365f17e9f056, 0x6fb38542338b72c8)
        ),
        Ok(F128::from_words(0xc083587667da289a, 0x8bcb400000000000)),
        "rational-18"
    );
    assert_eq!(
        mul67_chop(
            F128::from_words(0x40b316d1d4f2f338, 0xd1997d9fc706580d),
            F128::from_words(0xbf5144bbffc68296, 0xe2294cf3edb7321f)
        ),
        Ok(F128::from_words(0xc00561ae535d40ef, 0x2b5d400000000000)),
        "rational-19"
    );
    assert_eq!(
        mul67_chop(
            F128::from_words(0x4085061e0e40fd35, 0x1a9606d6b79e1731),
            F128::from_words(0xc01f6a34288afab6, 0x3361343b1a74bdcc)
        ),
        Ok(F128::from_words(0xc0a572dbe7c5ca62, 0x44df400000000000)),
        "rational-20"
    );
    assert_eq!(
        mul67_chop(
            F128::from_words(0x4038e6299b406762, 0x39f74335d8426357),
            F128::from_words(0x408878acf8159ae3, 0x30c7403b17700db5)
        ),
        Ok(F128::from_words(0x40c265aacb8e1e89, 0x2b3e800000000000)),
        "rational-21"
    );
    assert_eq!(
        mul67_chop(
            F128::from_words(0xbf9c4603f1d0122b, 0xfa6fbedc7fb712b1),
            F128::from_words(0xc05f2c8cb710998d, 0xca54bca8286b1afa)
        ),
        Ok(F128::from_words(0x3ffc7ebfd2aa08bd, 0x40e3000000000000)),
        "rational-22"
    );
    assert_eq!(
        mul67_chop(
            F128::from_words(0x3ffe55f46c23593e, 0x8b222784e8912b03),
            F128::from_words(0x3f855edb1d05a930, 0xad5056dd1cea2000)
        ),
        Ok(F128::from_words(0x3f84d4a8dab51646, 0x593f800000000000)),
        "rational-23"
    );
    assert_eq!(
        mul67_chop(
            F128::from_words(0xc00dc6f9731d7188, 0x88ab3f369dc88f0a),
            F128::from_words(0x3fcd0983fc817668, 0x79e30be509ff2540)
        ),
        Ok(F128::from_words(0xbfdbd7e2ca93060c, 0x14f5800000000000)),
        "rational-24"
    );
    assert_eq!(
        mul67_chop(
            F128::from_words(0xbf5319de0ca1331b, 0xfd0f1690cd8d31e0),
            F128::from_words(0xbf8cc8f264973a9a, 0xd918d134201cf6b0)
        ),
        Ok(F128::from_words(0x3ee0f71e692852db, 0x63bcc00000000000)),
        "rational-25"
    );
    assert_eq!(
        mul67_chop(
            F128::from_words(0xbf8b3dc86e05c0b8, 0x6f71ce41c31671c1),
            F128::from_words(0xbf6f78e83b17518a, 0x1af08066346e5bd8)
        ),
        Ok(F128::from_words(0x3efbd3dea8964bda, 0x9b0ec00000000000)),
        "rational-26"
    );
    assert_eq!(
        mul67_chop(
            F128::from_words(0xbfcb6e000ec540c5, 0xd6ecf142257585a3),
            F128::from_words(0xbffefb6d0c5d8b55, 0xc0c1e7628d3f1dca)
        ),
        Ok(F128::from_words(0x3fcb6abb027a5797, 0xd19a400000000000)),
        "rational-27"
    );
    assert_eq!(
        mul67_chop(
            F128::from_words(0xc049a21188fe809d, 0x9ab48d201b616253),
            F128::from_words(0xbf9acc0df912452f, 0x05bd48e6edc45a9e)
        ),
        Ok(F128::from_words(0x3fe577a729e7143f, 0x35bfc00000000000)),
        "rational-28"
    );
    assert_eq!(
        mul67_chop(
            F128::from_words(0x3fe52ef74185e00b, 0x37bd91ed0535bee7),
            F128::from_words(0xc08700a071d58a43, 0x6e540b594e598872)
        ),
        Ok(F128::from_words(0xc06d2fb522c6aefd, 0xf1a2c00000000000)),
        "rational-29"
    );
}
#[test]
fn reference_mul_e67() {
    assert_eq!(
        mul_e67(
            F128::from_words(0x0000000000000000, 0x0000000000000000),
            F128::from_words(0x8000000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x8000000000000000, 0x0000000000000000)),
        "zero-sign"
    );
    assert_eq!(
        mul_e67(
            F128::from_words(0x8000000000000000, 0x0000000000000000),
            F128::from_words(0x8000000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x0000000000000000, 0x0000000000000000)),
        "negative-zero"
    );
    assert_eq!(
        mul_e67(
            F128::from_words(0x3fff000000000000, 0x0000000000000000),
            F128::from_words(0x4000800000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x4000800000000000, 0x0000000000000000)),
        "one-third"
    );
    assert_eq!(
        mul_e67(
            F128::from_words(0x3fff000000000000, 0x0000000000000000),
            F128::from_words(0x3f8e000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x3f8e000000000000, 0x0000000000000000)),
        "quad-tie-even"
    );
    assert_eq!(
        mul_e67(
            F128::from_words(0x3fff000000000000, 0x0000000000000001),
            F128::from_words(0x3f8e000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x3f8e000000000000, 0x0000000000000000)),
        "quad-tie-odd"
    );
    assert_eq!(
        mul_e67(
            F128::from_words(0x3fff000000000000, 0x0000000000000000),
            F128::from_words(0x3fbf000000000000, 0x4000000000000000)
        ),
        Ok(F128::from_words(0x3fbf000000000000, 0x4000000000000000)),
        "double-round"
    );
    assert_eq!(
        mul_e67(
            F128::from_words(0x3fff000000000000, 0x0000400000000000),
            F128::from_words(0x3fff000000000000, 0x0000800000000000)
        ),
        Ok(F128::from_words(0x3fff000000000000, 0x0000400000000000)),
        "asymmetric-right"
    );
    assert_eq!(
        mul_e67(
            F128::from_words(0x3fff000000000000, 0x0000800000000000),
            F128::from_words(0x3fff000000000000, 0x0000400000000000)
        ),
        Ok(F128::from_words(0x3fff000000000000, 0x0000800000000000)),
        "asymmetric-left"
    );
    assert_eq!(
        mul_e67(
            F128::from_words(0x3fff000000000000, 0x0000000000000000),
            F128::from_words(0xbc17000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0xbc17000000000000, 0x0000000000000000)),
        "tiny-opposite"
    );
    assert_eq!(
        mul_e67(
            F128::from_words(0xbfff000000000000, 0x0000000000000000),
            F128::from_words(0x3c17000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0xbc17000000000000, 0x0000000000000000)),
        "negative-tiny-opposite"
    );
    assert_eq!(
        mul_e67(
            F128::from_words(0x4001000000000000, 0x0000000000000000),
            F128::from_words(0xbfc0800000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0xbfc2800000000000, 0x0000000000000000)),
        "below-binade"
    );
    assert_eq!(
        mul_e67(
            F128::from_words(0x4001000000000000, 0x0000000000000000),
            F128::from_words(0x3fc0800000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x3fc2800000000000, 0x0000000000000000)),
        "above-binade"
    );
    assert_eq!(
        mul_e67(
            F128::from_words(0x0000000000000000, 0x0000000000000003),
            F128::from_words(0x0000000000000000, 0x0000000000000004)
        ),
        Ok(F128::from_words(0x0000000000000000, 0x0000000000000000)),
        "subnormal-add-shortcut"
    );
    assert_eq!(
        mul_e67(
            F128::from_words(0x0000ffffffffffff, 0xffffffffffffffff),
            F128::from_words(0x0000000000000000, 0x0000000000000002)
        ),
        Ok(F128::from_words(0x0000000000000000, 0x0000000000000000)),
        "subnormal-normal-cross"
    );
    assert_eq!(
        mul_e67(
            F128::from_words(0x0001000000000000, 0x0000000000000000),
            F128::from_words(0x3ff5000000000000, 0x0000400000000000)
        ),
        Ok(F128::from_words(0x0000004000000000, 0x0000000000000000)),
        "subnormal-internal67"
    );
    assert_eq!(
        mul_e67(
            F128::from_words(0x0000000000000000, 0x0000000000000001),
            F128::from_words(0x3ffe000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x0000000000000000, 0x0000000000000000)),
        "underflow-product"
    );
    assert_eq!(
        mul_e67(
            F128::from_words(0x0000000000000000, 0x0000000000000001),
            F128::from_words(0x4000000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x0000000000000000, 0x0000000000000002)),
        "half-minimum"
    );
    assert_eq!(
        mul_e67(
            F128::from_words(0x7e7f000000000000, 0x0000000000000000),
            F128::from_words(0x817f000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0xbfff000000000000, 0x0000000000000000)),
        "huge-opposite"
    );
    assert_eq!(
        mul_e67(
            F128::from_words(0x3fffffffffffffff, 0xffffffffffffffff),
            F128::from_words(0x3fffffffffffffff, 0xffffffffffffffff)
        ),
        Ok(F128::from_words(0x4000ffffffffffff, 0xfffdc00000000000)),
        "wide-product"
    );
    assert_eq!(
        mul_e67(
            F128::from_words(0x3f976e0364caa050, 0x8b88599668a6328e),
            F128::from_words(0x407ec4406dfade38, 0x9adc2542fa7422b1)
        ),
        Ok(F128::from_words(0x4017434d0e068a54, 0x6f18400000000000)),
        "rational-0"
    );
    assert_eq!(
        mul_e67(
            F128::from_words(0x401dd161ab6e8e1d, 0xd23ab5033ba5120a),
            F128::from_words(0xc0a29f9cba4fbdeb, 0x5582b8a652fe4854)
        ),
        Ok(F128::from_words(0xc0c179c51f8f4e5a, 0x0c01c00000000000)),
        "rational-1"
    );
    assert_eq!(
        mul_e67(
            F128::from_words(0x3f7526f6860ee0ad, 0x8eb452c36cff7606),
            F128::from_words(0x403e55319397bf4c, 0x338950d18897ce69)
        ),
        Ok(F128::from_words(0x3fb4891f7fcfe0ed, 0x88ad400000000000)),
        "rational-2"
    );
    assert_eq!(
        mul_e67(
            F128::from_words(0x407ed0e5d4fcde8a, 0x5a07b21766575899),
            F128::from_words(0x402f31d543d75467, 0x8f2d1c302929ce5c)
        ),
        Ok(F128::from_words(0x40af15b28e975649, 0xc3ac800000000000)),
        "rational-3"
    );
    assert_eq!(
        mul_e67(
            F128::from_words(0xbf6fa93875ccd29c, 0xf4a4b1d2823a55be),
            F128::from_words(0x4046687a91deaa23, 0x80e6b1de6e41b28b)
        ),
        Ok(F128::from_words(0xbfb72b617e6d8009, 0x5c65000000000000)),
        "rational-4"
    );
    assert_eq!(
        mul_e67(
            F128::from_words(0x40563949f3796c30, 0xc257681164d2a50c),
            F128::from_words(0x4019b1ee9579f94c, 0x4eaed9ca369d4c37)
        ),
        Ok(F128::from_words(0x40710985073f0550, 0x8f3ac00000000000)),
        "rational-5"
    );
    assert_eq!(
        mul_e67(
            F128::from_words(0x40a0d40f4dedffc8, 0x4872f8cf5ea16b82),
            F128::from_words(0xbf8cbf37785626ca, 0x59ec4e6748db3775)
        ),
        Ok(F128::from_words(0xc02e98d61230802c, 0x5337c00000000000)),
        "rational-6"
    );
    assert_eq!(
        mul_e67(
            F128::from_words(0x40218c303fcc2682, 0x10e9a7dfa0c84c0c),
            F128::from_words(0x4096305c412a05f3, 0xe55ce2d5da562d0f)
        ),
        Ok(F128::from_words(0x40b8d70811f2a46b, 0xfe19c00000000000)),
        "rational-7"
    );
    assert_eq!(
        mul_e67(
            F128::from_words(0xc018b179d909cb04, 0xba30a83aeb3d10d0),
            F128::from_words(0x40b2ede9d1d55254, 0x740d7f67c593d6da)
        ),
        Ok(F128::from_words(0xc0cca229c9160aee, 0x3ee0c00000000000)),
        "rational-8"
    );
    assert_eq!(
        mul_e67(
            F128::from_words(0xbff78794a804f48f, 0xc2b2277ae5448dac),
            F128::from_words(0xc01cafdf5234216b, 0xc3b248e12c219f6f)
        ),
        Ok(F128::from_words(0x40154a4c6f8e0798, 0xe4bec00000000000)),
        "rational-9"
    );
    assert_eq!(
        mul_e67(
            F128::from_words(0x403417b28ac30de6, 0xdaaa694cb8c61dc8),
            F128::from_words(0xbfc646abe76fb68a, 0x3786dcd6370c5dfe)
        ),
        Ok(F128::from_words(0xbffb64e92dd33d0e, 0x6156800000000000)),
        "rational-10"
    );
    assert_eq!(
        mul_e67(
            F128::from_words(0x3f81aed55a1aa8f2, 0xb9929c8651647899),
            F128::from_words(0xc07ac9f1bc8a8794, 0x944e553eba43ebba)
        ),
        Ok(F128::from_words(0xbffd8158d9009e06, 0x87d5c00000000000)),
        "rational-11"
    );
    assert_eq!(
        mul_e67(
            F128::from_words(0x4052293b2522b36c, 0x51ae68d346a92b28),
            F128::from_words(0x3f78be450f941f1a, 0xfafaba94d160477f)
        ),
        Ok(F128::from_words(0x3fcc03129cdcf436, 0x87fcc00000000000)),
        "rational-12"
    );
    assert_eq!(
        mul_e67(
            F128::from_words(0x4033ed930a3f5f8e, 0x9d73e514430664b1),
            F128::from_words(0xc0afa4c59f00ce24, 0xe4945d544d2d7ac9)
        ),
        Ok(F128::from_words(0xc0e495a120c35e2d, 0xa985c00000000000)),
        "rational-13"
    );
    assert_eq!(
        mul_e67(
            F128::from_words(0xc01aa6388b0cd844, 0x9c7146f0c0206cec),
            F128::from_words(0xc0a7bb7ec3bc4b2b, 0x238d013d590b3b44)
        ),
        Ok(F128::from_words(0x40c36dba75a3a4fd, 0xe4c2400000000000)),
        "rational-14"
    );
    assert_eq!(
        mul_e67(
            F128::from_words(0x3f6b3feefc29d05b, 0x383484e56b2f8cee),
            F128::from_words(0x3ff9735d47b31306, 0xb8406e5ef8b1615a)
        ),
        Ok(F128::from_words(0x3f65d01beadd47f5, 0xf032400000000000)),
        "rational-15"
    );
    assert_eq!(
        mul_e67(
            F128::from_words(0xc06765bba6d242d5, 0x1b878e0f4a5c1679),
            F128::from_words(0xc07fa6a87c0ced04, 0xc758747c98b9636b)
        ),
        Ok(F128::from_words(0x40e8274f62be847b, 0x1c32400000000000)),
        "rational-16"
    );
    assert_eq!(
        mul_e67(
            F128::from_words(0x3f87ada2f38bd6ac, 0x0979bc14034f4082),
            F128::from_words(0x3fa22cf56289f4df, 0x76496d0797838687)
        ),
        Ok(F128::from_words(0x3f2af916c7badccc, 0x4358c00000000000)),
        "rational-17"
    );
    assert_eq!(
        mul_e67(
            F128::from_words(0x406b1c1e71f8f9d3, 0x9f83c3db94f26656),
            F128::from_words(0xc017365f17e9f056, 0x6fb38542338b72c8)
        ),
        Ok(F128::from_words(0xc083587667da289a, 0x8bc9800000000000)),
        "rational-18"
    );
    assert_eq!(
        mul_e67(
            F128::from_words(0x40b316d1d4f2f338, 0xd1997d9fc706580d),
            F128::from_words(0xbf5144bbffc68296, 0xe2294cf3edb7321f)
        ),
        Ok(F128::from_words(0xc00561ae535d40ef, 0x2b5c000000000000)),
        "rational-19"
    );
    assert_eq!(
        mul_e67(
            F128::from_words(0x4085061e0e40fd35, 0x1a9606d6b79e1731),
            F128::from_words(0xc01f6a34288afab6, 0x3361343b1a74bdcc)
        ),
        Ok(F128::from_words(0xc0a572dbe7c5ca62, 0x44de000000000000)),
        "rational-20"
    );
    assert_eq!(
        mul_e67(
            F128::from_words(0x4038e6299b406762, 0x39f74335d8426357),
            F128::from_words(0x408878acf8159ae3, 0x30c7403b17700db5)
        ),
        Ok(F128::from_words(0x40c265aacb8e1e89, 0x2b3d400000000000)),
        "rational-21"
    );
    assert_eq!(
        mul_e67(
            F128::from_words(0xbf9c4603f1d0122b, 0xfa6fbedc7fb712b1),
            F128::from_words(0xc05f2c8cb710998d, 0xca54bca8286b1afa)
        ),
        Ok(F128::from_words(0x3ffc7ebfd2aa08bd, 0x40e2000000000000)),
        "rational-22"
    );
    assert_eq!(
        mul_e67(
            F128::from_words(0x3ffe55f46c23593e, 0x8b222784e8912b03),
            F128::from_words(0x3f855edb1d05a930, 0xad5056dd1cea2000)
        ),
        Ok(F128::from_words(0x3f84d4a8dab51646, 0x593f000000000000)),
        "rational-23"
    );
    assert_eq!(
        mul_e67(
            F128::from_words(0xc00dc6f9731d7188, 0x88ab3f369dc88f0a),
            F128::from_words(0x3fcd0983fc817668, 0x79e30be509ff2540)
        ),
        Ok(F128::from_words(0xbfdbd7e2ca93060c, 0x14f3800000000000)),
        "rational-24"
    );
    assert_eq!(
        mul_e67(
            F128::from_words(0xbf5319de0ca1331b, 0xfd0f1690cd8d31e0),
            F128::from_words(0xbf8cc8f264973a9a, 0xd918d134201cf6b0)
        ),
        Ok(F128::from_words(0x3ee0f71e692852db, 0x63bbc00000000000)),
        "rational-25"
    );
    assert_eq!(
        mul_e67(
            F128::from_words(0xbf8b3dc86e05c0b8, 0x6f71ce41c31671c1),
            F128::from_words(0xbf6f78e83b17518a, 0x1af08066346e5bd8)
        ),
        Ok(F128::from_words(0x3efbd3dea8964bda, 0x9b0e400000000000)),
        "rational-26"
    );
    assert_eq!(
        mul_e67(
            F128::from_words(0xbfcb6e000ec540c5, 0xd6ecf142257585a3),
            F128::from_words(0xbffefb6d0c5d8b55, 0xc0c1e7628d3f1dca)
        ),
        Ok(F128::from_words(0x3fcb6abb027a5797, 0xd199000000000000)),
        "rational-27"
    );
    assert_eq!(
        mul_e67(
            F128::from_words(0xc049a21188fe809d, 0x9ab48d201b616253),
            F128::from_words(0xbf9acc0df912452f, 0x05bd48e6edc45a9e)
        ),
        Ok(F128::from_words(0x3fe577a729e7143f, 0x35bec00000000000)),
        "rational-28"
    );
    assert_eq!(
        mul_e67(
            F128::from_words(0x3fe52ef74185e00b, 0x37bd91ed0535bee7),
            F128::from_words(0xc08700a071d58a43, 0x6e540b594e598872)
        ),
        Ok(F128::from_words(0xc06d2fb522c6aefd, 0xf1a2800000000000)),
        "rational-29"
    );
}
#[test]
fn reference_div67_chop() {
    assert_eq!(
        div67_chop(
            F128::from_words(0x0000000000000000, 0x0000000000000000),
            F128::from_words(0x8000000000000000, 0x0000000000000000)
        ),
        Err(ArithmeticError::DivisionByZero),
        "zero-sign"
    );
    assert_eq!(
        div67_chop(
            F128::from_words(0x8000000000000000, 0x0000000000000000),
            F128::from_words(0x8000000000000000, 0x0000000000000000)
        ),
        Err(ArithmeticError::DivisionByZero),
        "negative-zero"
    );
    assert_eq!(
        div67_chop(
            F128::from_words(0x3fff000000000000, 0x0000000000000000),
            F128::from_words(0x4000800000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x3ffd555555555555, 0x5555400000000000)),
        "one-third"
    );
    assert_eq!(
        div67_chop(
            F128::from_words(0x3fff000000000000, 0x0000000000000000),
            F128::from_words(0x3f8e000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x4070000000000000, 0x0000000000000000)),
        "quad-tie-even"
    );
    assert_eq!(
        div67_chop(
            F128::from_words(0x3fff000000000000, 0x0000000000000001),
            F128::from_words(0x3f8e000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x4070000000000000, 0x0000000000000000)),
        "quad-tie-odd"
    );
    assert_eq!(
        div67_chop(
            F128::from_words(0x3fff000000000000, 0x0000000000000000),
            F128::from_words(0x3fbf000000000000, 0x4000000000000000)
        ),
        Ok(F128::from_words(0x403effffffffffff, 0x8000000000000000)),
        "double-round"
    );
    assert_eq!(
        div67_chop(
            F128::from_words(0x3fff000000000000, 0x0000400000000000),
            F128::from_words(0x3fff000000000000, 0x0000800000000000)
        ),
        Ok(F128::from_words(0x3ffeffffffffffff, 0xffff800000000000)),
        "asymmetric-right"
    );
    assert_eq!(
        div67_chop(
            F128::from_words(0x3fff000000000000, 0x0000800000000000),
            F128::from_words(0x3fff000000000000, 0x0000400000000000)
        ),
        Ok(F128::from_words(0x3fff000000000000, 0x0000000000000000)),
        "asymmetric-left"
    );
    assert_eq!(
        div67_chop(
            F128::from_words(0x3fff000000000000, 0x0000000000000000),
            F128::from_words(0xbc17000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0xc3e7000000000000, 0x0000000000000000)),
        "tiny-opposite"
    );
    assert_eq!(
        div67_chop(
            F128::from_words(0xbfff000000000000, 0x0000000000000000),
            F128::from_words(0x3c17000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0xc3e7000000000000, 0x0000000000000000)),
        "negative-tiny-opposite"
    );
    assert_eq!(
        div67_chop(
            F128::from_words(0x4001000000000000, 0x0000000000000000),
            F128::from_words(0xbfc0800000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0xc03f555555555555, 0x5555400000000000)),
        "below-binade"
    );
    assert_eq!(
        div67_chop(
            F128::from_words(0x4001000000000000, 0x0000000000000000),
            F128::from_words(0x3fc0800000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x403f555555555555, 0x5555400000000000)),
        "above-binade"
    );
    assert_eq!(
        div67_chop(
            F128::from_words(0x0000000000000000, 0x0000000000000003),
            F128::from_words(0x0000000000000000, 0x0000000000000004)
        ),
        Ok(F128::from_words(0x3ffe800000000000, 0x0000000000000000)),
        "subnormal-add-shortcut"
    );
    assert_eq!(
        div67_chop(
            F128::from_words(0x0000ffffffffffff, 0xffffffffffffffff),
            F128::from_words(0x0000000000000000, 0x0000000000000002)
        ),
        Ok(F128::from_words(0x406dffffffffffff, 0xffffc00000000000)),
        "subnormal-normal-cross"
    );
    assert_eq!(
        div67_chop(
            F128::from_words(0x0001000000000000, 0x0000000000000000),
            F128::from_words(0x3ff5000000000000, 0x0000400000000000)
        ),
        Ok(F128::from_words(0x000affffffffffff, 0xffff800000000000)),
        "subnormal-internal67"
    );
    assert_eq!(
        div67_chop(
            F128::from_words(0x0000000000000000, 0x0000000000000001),
            F128::from_words(0x3ffe000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x0000000000000000, 0x0000000000000002)),
        "underflow-product"
    );
    assert_eq!(
        div67_chop(
            F128::from_words(0x0000000000000000, 0x0000000000000001),
            F128::from_words(0x4000000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x0000000000000000, 0x0000000000000000)),
        "half-minimum"
    );
    assert_eq!(
        div67_chop(
            F128::from_words(0x7e7f000000000000, 0x0000000000000000),
            F128::from_words(0x817f000000000000, 0x0000000000000000)
        ),
        Err(ArithmeticError::Overflow),
        "huge-opposite"
    );
    assert_eq!(
        div67_chop(
            F128::from_words(0x3fffffffffffffff, 0xffffffffffffffff),
            F128::from_words(0x3fffffffffffffff, 0xffffffffffffffff)
        ),
        Ok(F128::from_words(0x3fff000000000000, 0x0000000000000000)),
        "wide-product"
    );
    assert_eq!(
        div67_chop(
            F128::from_words(0x3f976e0364caa050, 0x8b88599668a6328e),
            F128::from_words(0x407ec4406dfade38, 0x9adc2542fa7422b1)
        ),
        Ok(F128::from_words(0x3f179e5e4d058a92, 0x643b800000000000)),
        "rational-0"
    );
    assert_eq!(
        div67_chop(
            F128::from_words(0x401dd161ab6e8e1d, 0xd23ab5033ba5120a),
            F128::from_words(0xc0a29f9cba4fbdeb, 0x5582b8a652fe4854)
        ),
        Ok(F128::from_words(0xbf7a1ea7e52a55f4, 0x95a8000000000000)),
        "rational-1"
    );
    assert_eq!(
        div67_chop(
            F128::from_words(0x3f7526f6860ee0ad, 0x8eb452c36cff7606),
            F128::from_words(0x403e55319397bf4c, 0x338950d18897ce69)
        ),
        Ok(F128::from_words(0x3f35baa0273e3e13, 0x98acc00000000000)),
        "rational-2"
    );
    assert_eq!(
        div67_chop(
            F128::from_words(0x407ed0e5d4fcde8a, 0x5a07b21766575899),
            F128::from_words(0x402f31d543d75467, 0x8f2d1c302929ce5c)
        ),
        Ok(F128::from_words(0x404e8525797037e9, 0x2215c00000000000)),
        "rational-3"
    );
    assert_eq!(
        div67_chop(
            F128::from_words(0xbf6fa93875ccd29c, 0xf4a4b1d2823a55be),
            F128::from_words(0x4046687a91deaa23, 0x80e6b1de6e41b28b)
        ),
        Ok(F128::from_words(0xbf282dfa393bd997, 0x3df9800000000000)),
        "rational-4"
    );
    assert_eq!(
        div67_chop(
            F128::from_words(0x40563949f3796c30, 0xc257681164d2a50c),
            F128::from_words(0x4019b1ee9579f94c, 0x4eaed9ca369d4c37)
        ),
        Ok(F128::from_words(0x403b71a6f5e26894, 0xd1f1c00000000000)),
        "rational-5"
    );
    assert_eq!(
        div67_chop(
            F128::from_words(0x40a0d40f4dedffc8, 0x4872f8cf5ea16b82),
            F128::from_words(0xbf8cbf37785626ca, 0x59ec4e6748db3775)
        ),
        Ok(F128::from_words(0xc1130bee6384b339, 0x419d000000000000)),
        "rational-6"
    );
    assert_eq!(
        div67_chop(
            F128::from_words(0x40218c303fcc2682, 0x10e9a7dfa0c84c0c),
            F128::from_words(0x4096305c412a05f3, 0xe55ce2d5da562d0f)
        ),
        Ok(F128::from_words(0x3f8a4d3cc452eb11, 0x654c800000000000)),
        "rational-7"
    );
    assert_eq!(
        div67_chop(
            F128::from_words(0xc018b179d909cb04, 0xba30a83aeb3d10d0),
            F128::from_words(0x40b2ede9d1d55254, 0x740d7f67c593d6da)
        ),
        Ok(F128::from_words(0xbf64c15975881f48, 0xc8c4000000000000)),
        "rational-8"
    );
    assert_eq!(
        div67_chop(
            F128::from_words(0xbff78794a804f48f, 0xc2b2277ae5448dac),
            F128::from_words(0xc01cafdf5234216b, 0xc3b248e12c219f6f)
        ),
        Ok(F128::from_words(0x3fd9d03b99468b80, 0x658f400000000000)),
        "rational-9"
    );
    assert_eq!(
        div67_chop(
            F128::from_words(0x403417b28ac30de6, 0xdaaa694cb8c61dc8),
            F128::from_words(0xbfc646abe76fb68a, 0x3786dcd6370c5dfe)
        ),
        Ok(F128::from_words(0xc06cb6605d52dc91, 0x3c74000000000000)),
        "rational-10"
    );
    assert_eq!(
        div67_chop(
            F128::from_words(0x3f81aed55a1aa8f2, 0xb9929c8651647899),
            F128::from_words(0xc07ac9f1bc8a8794, 0x944e553eba43ebba)
        ),
        Ok(F128::from_words(0xbf05e1b05f75ca11, 0x3093400000000000)),
        "rational-11"
    );
    assert_eq!(
        div67_chop(
            F128::from_words(0x4052293b2522b36c, 0x51ae68d346a92b28),
            F128::from_words(0x3f78be450f941f1a, 0xfafaba94d160477f)
        ),
        Ok(F128::from_words(0x40d85502774e847e, 0xeabfc00000000000)),
        "rational-12"
    );
    assert_eq!(
        div67_chop(
            F128::from_words(0x4033ed930a3f5f8e, 0x9d73e514430664b1),
            F128::from_words(0xc0afa4c59f00ce24, 0xe4945d544d2d7ac9)
        ),
        Ok(F128::from_words(0xbf832c4b1bff885c, 0x31bb800000000000)),
        "rational-13"
    );
    assert_eq!(
        div67_chop(
            F128::from_words(0xc01aa6388b0cd844, 0x9c7146f0c0206cec),
            F128::from_words(0xc0a7bb7ec3bc4b2b, 0x238d013d590b3b44)
        ),
        Ok(F128::from_words(0x3f71e77086151212, 0x527e800000000000)),
        "rational-14"
    );
    assert_eq!(
        div67_chop(
            F128::from_words(0x3f6b3feefc29d05b, 0x383484e56b2f8cee),
            F128::from_words(0x3ff9735d47b31306, 0xb8406e5ef8b1615a)
        ),
        Ok(F128::from_words(0x3f70b917a19b2147, 0xc254000000000000)),
        "rational-15"
    );
    assert_eq!(
        div67_chop(
            F128::from_words(0xc06765bba6d242d5, 0x1b878e0f4a5c1679),
            F128::from_words(0xc07fa6a87c0ced04, 0xc758747c98b9636b)
        ),
        Ok(F128::from_words(0x3fe6b159d72ba0fd, 0x04d0c00000000000)),
        "rational-16"
    );
    assert_eq!(
        div67_chop(
            F128::from_words(0x3f87ada2f38bd6ac, 0x0979bc14034f4082),
            F128::from_words(0x3fa22cf56289f4df, 0x76496d0797838687)
        ),
        Ok(F128::from_words(0x3fe46d749b8b6eb0, 0x13ddc00000000000)),
        "rational-17"
    );
    assert_eq!(
        div67_chop(
            F128::from_words(0x406b1c1e71f8f9d3, 0x9f83c3db94f26656),
            F128::from_words(0xc017365f17e9f056, 0x6fb38542338b72c8)
        ),
        Ok(F128::from_words(0xc052d4b160130b25, 0x8155400000000000)),
        "rational-18"
    );
    assert_eq!(
        div67_chop(
            F128::from_words(0x40b316d1d4f2f338, 0xd1997d9fc706580d),
            F128::from_words(0xbf5144bbffc68296, 0xe2294cf3edb7321f)
        ),
        Ok(F128::from_words(0xc160b79b84ccb82a, 0xfc1d800000000000)),
        "rational-19"
    );
    assert_eq!(
        div67_chop(
            F128::from_words(0x4085061e0e40fd35, 0x1a9606d6b79e1731),
            F128::from_words(0xc01f6a34288afab6, 0x3361343b1a74bdcc)
        ),
        Ok(F128::from_words(0xc064728563863381, 0x866c800000000000)),
        "rational-20"
    );
    assert_eq!(
        div67_chop(
            F128::from_words(0x4038e6299b406762, 0x39f74335d8426357),
            F128::from_words(0x408878acf8159ae3, 0x30c7403b17700db5)
        ),
        Ok(F128::from_words(0x3faf4a6918d05d27, 0xb19f800000000000)),
        "rational-21"
    );
    assert_eq!(
        div67_chop(
            F128::from_words(0xbf9c4603f1d0122b, 0xfa6fbedc7fb712b1),
            F128::from_words(0xc05f2c8cb710998d, 0xca54bca8286b1afa)
        ),
        Ok(F128::from_words(0x3f3c15b0e6d23ded, 0xf6a3400000000000)),
        "rational-22"
    );
    assert_eq!(
        div67_chop(
            F128::from_words(0x3ffe55f46c23593e, 0x8b222784e8912b03),
            F128::from_words(0x3f855edb1d05a930, 0xad5056dd1cea2000)
        ),
        Ok(F128::from_words(0x4077f302bb839d1d, 0x906fc00000000000)),
        "rational-23"
    );
    assert_eq!(
        div67_chop(
            F128::from_words(0xc00dc6f9731d7188, 0x88ab3f369dc88f0a),
            F128::from_words(0x3fcd0983fc817668, 0x79e30be509ff2540)
        ),
        Ok(F128::from_words(0xc03fb6ab436cb11d, 0xe984c00000000000)),
        "rational-24"
    );
    assert_eq!(
        div67_chop(
            F128::from_words(0xbf5319de0ca1331b, 0xfd0f1690cd8d31e0),
            F128::from_words(0xbf8cc8f264973a9a, 0xd918d134201cf6b0)
        ),
        Ok(F128::from_words(0x3fc53bd3aea098b8, 0x1285800000000000)),
        "rational-25"
    );
    assert_eq!(
        div67_chop(
            F128::from_words(0xbf8b3dc86e05c0b8, 0x6f71ce41c31671c1),
            F128::from_words(0xbf6f78e83b17518a, 0x1af08066346e5bd8)
        ),
        Ok(F128::from_words(0x401aafaf2743904a, 0xb16ec00000000000)),
        "rational-26"
    );
    assert_eq!(
        div67_chop(
            F128::from_words(0xbfcb6e000ec540c5, 0xd6ecf142257585a3),
            F128::from_words(0xbffefb6d0c5d8b55, 0xc0c1e7628d3f1dca)
        ),
        Ok(F128::from_words(0x3fcb714ca6a927d2, 0x0396400000000000)),
        "rational-27"
    );
    assert_eq!(
        div67_chop(
            F128::from_words(0xc049a21188fe809d, 0x9ab48d201b616253),
            F128::from_words(0xbf9acc0df912452f, 0x05bd48e6edc45a9e)
        ),
        Ok(F128::from_words(0x40add145f0d68fd1, 0xc9cb400000000000)),
        "rational-28"
    );
    assert_eq!(
        div67_chop(
            F128::from_words(0x3fe52ef74185e00b, 0x37bd91ed0535bee7),
            F128::from_words(0xc08700a071d58a43, 0x6e540b594e598872)
        ),
        Ok(F128::from_words(0xbf5d2e39d6fbe15e, 0x8555c00000000000)),
        "rational-29"
    );
}
#[test]
fn reference_add64_ne() {
    assert_eq!(
        add64_ne(
            F128::from_words(0x0000000000000000, 0x0000000000000000),
            F128::from_words(0x8000000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x0000000000000000, 0x0000000000000000)),
        "zero-sign"
    );
    assert_eq!(
        add64_ne(
            F128::from_words(0x8000000000000000, 0x0000000000000000),
            F128::from_words(0x8000000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x8000000000000000, 0x0000000000000000)),
        "negative-zero"
    );
    assert_eq!(
        add64_ne(
            F128::from_words(0x3fff000000000000, 0x0000000000000000),
            F128::from_words(0x4000800000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x4001000000000000, 0x0000000000000000)),
        "one-third"
    );
    assert_eq!(
        add64_ne(
            F128::from_words(0x3fff000000000000, 0x0000000000000000),
            F128::from_words(0x3f8e000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x3fff000000000000, 0x0000000000000000)),
        "quad-tie-even"
    );
    assert_eq!(
        add64_ne(
            F128::from_words(0x3fff000000000000, 0x0000000000000001),
            F128::from_words(0x3f8e000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x3fff000000000000, 0x0000000000000000)),
        "quad-tie-odd"
    );
    assert_eq!(
        add64_ne(
            F128::from_words(0x3fff000000000000, 0x0000000000000000),
            F128::from_words(0x3fbf000000000000, 0x4000000000000000)
        ),
        Ok(F128::from_words(0x3fff000000000000, 0x0000000000000000)),
        "double-round"
    );
    assert_eq!(
        add64_ne(
            F128::from_words(0x3fff000000000000, 0x0000400000000000),
            F128::from_words(0x3fff000000000000, 0x0000800000000000)
        ),
        Ok(F128::from_words(0x4000000000000000, 0x0000000000000000)),
        "asymmetric-right"
    );
    assert_eq!(
        add64_ne(
            F128::from_words(0x3fff000000000000, 0x0000800000000000),
            F128::from_words(0x3fff000000000000, 0x0000400000000000)
        ),
        Ok(F128::from_words(0x4000000000000000, 0x0000000000000000)),
        "asymmetric-left"
    );
    assert_eq!(
        add64_ne(
            F128::from_words(0x3fff000000000000, 0x0000000000000000),
            F128::from_words(0xbc17000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x3fff000000000000, 0x0000000000000000)),
        "tiny-opposite"
    );
    assert_eq!(
        add64_ne(
            F128::from_words(0xbfff000000000000, 0x0000000000000000),
            F128::from_words(0x3c17000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0xbfff000000000000, 0x0000000000000000)),
        "negative-tiny-opposite"
    );
    assert_eq!(
        add64_ne(
            F128::from_words(0x4001000000000000, 0x0000000000000000),
            F128::from_words(0xbfc0800000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x4000ffffffffffff, 0xfffe000000000000)),
        "below-binade"
    );
    assert_eq!(
        add64_ne(
            F128::from_words(0x4001000000000000, 0x0000000000000000),
            F128::from_words(0x3fc0800000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x4001000000000000, 0x0000000000000000)),
        "above-binade"
    );
    assert_eq!(
        add64_ne(
            F128::from_words(0x0000000000000000, 0x0000000000000003),
            F128::from_words(0x0000000000000000, 0x0000000000000004)
        ),
        Ok(F128::from_words(0x0000000000000000, 0x0000000000000000)),
        "subnormal-add-shortcut"
    );
    assert_eq!(
        add64_ne(
            F128::from_words(0x0000ffffffffffff, 0xffffffffffffffff),
            F128::from_words(0x0000000000000000, 0x0000000000000002)
        ),
        Ok(F128::from_words(0x0001000000000000, 0x0000000000000000)),
        "subnormal-normal-cross"
    );
    assert_eq!(
        add64_ne(
            F128::from_words(0x0001000000000000, 0x0000000000000000),
            F128::from_words(0x3ff5000000000000, 0x0000400000000000)
        ),
        Ok(F128::from_words(0x3ff5000000000000, 0x0000000000000000)),
        "subnormal-internal67"
    );
    assert_eq!(
        add64_ne(
            F128::from_words(0x0000000000000000, 0x0000000000000001),
            F128::from_words(0x3ffe000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x3ffe000000000000, 0x0000000000000000)),
        "underflow-product"
    );
    assert_eq!(
        add64_ne(
            F128::from_words(0x0000000000000000, 0x0000000000000001),
            F128::from_words(0x4000000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x4000000000000000, 0x0000000000000000)),
        "half-minimum"
    );
    assert_eq!(
        add64_ne(
            F128::from_words(0x7e7f000000000000, 0x0000000000000000),
            F128::from_words(0x817f000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x7e7f000000000000, 0x0000000000000000)),
        "huge-opposite"
    );
    assert_eq!(
        add64_ne(
            F128::from_words(0x3fffffffffffffff, 0xffffffffffffffff),
            F128::from_words(0x3fffffffffffffff, 0xffffffffffffffff)
        ),
        Ok(F128::from_words(0x4001000000000000, 0x0000000000000000)),
        "wide-product"
    );
    assert_eq!(
        add64_ne(
            F128::from_words(0x3f976e0364caa050, 0x8b88599668a6328e),
            F128::from_words(0x407ec4406dfade38, 0x9adc2542fa7422b1)
        ),
        Ok(F128::from_words(0x407ec4406dfade38, 0x9adc000000000000)),
        "rational-0"
    );
    assert_eq!(
        add64_ne(
            F128::from_words(0x401dd161ab6e8e1d, 0xd23ab5033ba5120a),
            F128::from_words(0xc0a29f9cba4fbdeb, 0x5582b8a652fe4854)
        ),
        Ok(F128::from_words(0xc0a29f9cba4fbdeb, 0x5582000000000000)),
        "rational-1"
    );
    assert_eq!(
        add64_ne(
            F128::from_words(0x3f7526f6860ee0ad, 0x8eb452c36cff7606),
            F128::from_words(0x403e55319397bf4c, 0x338950d18897ce69)
        ),
        Ok(F128::from_words(0x403e55319397bf4c, 0x338a000000000000)),
        "rational-2"
    );
    assert_eq!(
        add64_ne(
            F128::from_words(0x407ed0e5d4fcde8a, 0x5a07b21766575899),
            F128::from_words(0x402f31d543d75467, 0x8f2d1c302929ce5c)
        ),
        Ok(F128::from_words(0x407ed0e5d4fcde8a, 0x5a08000000000000)),
        "rational-3"
    );
    assert_eq!(
        add64_ne(
            F128::from_words(0xbf6fa93875ccd29c, 0xf4a4b1d2823a55be),
            F128::from_words(0x4046687a91deaa23, 0x80e6b1de6e41b28b)
        ),
        Ok(F128::from_words(0x4046687a91deaa23, 0x80e6000000000000)),
        "rational-4"
    );
    assert_eq!(
        add64_ne(
            F128::from_words(0x40563949f3796c30, 0xc257681164d2a50c),
            F128::from_words(0x4019b1ee9579f94c, 0x4eaed9ca369d4c37)
        ),
        Ok(F128::from_words(0x40563949f3796c30, 0xc264000000000000)),
        "rational-5"
    );
    assert_eq!(
        add64_ne(
            F128::from_words(0x40a0d40f4dedffc8, 0x4872f8cf5ea16b82),
            F128::from_words(0xbf8cbf37785626ca, 0x59ec4e6748db3775)
        ),
        Ok(F128::from_words(0x40a0d40f4dedffc8, 0x4872000000000000)),
        "rational-6"
    );
    assert_eq!(
        add64_ne(
            F128::from_words(0x40218c303fcc2682, 0x10e9a7dfa0c84c0c),
            F128::from_words(0x4096305c412a05f3, 0xe55ce2d5da562d0f)
        ),
        Ok(F128::from_words(0x4096305c412a05f3, 0xe55c000000000000)),
        "rational-7"
    );
    assert_eq!(
        add64_ne(
            F128::from_words(0xc018b179d909cb04, 0xba30a83aeb3d10d0),
            F128::from_words(0x40b2ede9d1d55254, 0x740d7f67c593d6da)
        ),
        Ok(F128::from_words(0x40b2ede9d1d55254, 0x740e000000000000)),
        "rational-8"
    );
    assert_eq!(
        add64_ne(
            F128::from_words(0xbff78794a804f48f, 0xc2b2277ae5448dac),
            F128::from_words(0xc01cafdf5234216b, 0xc3b248e12c219f6f)
        ),
        Ok(F128::from_words(0xc01cafdf52342da8, 0x68f2000000000000)),
        "rational-9"
    );
    assert_eq!(
        add64_ne(
            F128::from_words(0x403417b28ac30de6, 0xdaaa694cb8c61dc8),
            F128::from_words(0xbfc646abe76fb68a, 0x3786dcd6370c5dfe)
        ),
        Ok(F128::from_words(0x403417b28ac30de6, 0xdaaa000000000000)),
        "rational-10"
    );
    assert_eq!(
        add64_ne(
            F128::from_words(0x3f81aed55a1aa8f2, 0xb9929c8651647899),
            F128::from_words(0xc07ac9f1bc8a8794, 0x944e553eba43ebba)
        ),
        Ok(F128::from_words(0xc07ac9f1bc8a8794, 0x944e000000000000)),
        "rational-11"
    );
    assert_eq!(
        add64_ne(
            F128::from_words(0x4052293b2522b36c, 0x51ae68d346a92b28),
            F128::from_words(0x3f78be450f941f1a, 0xfafaba94d160477f)
        ),
        Ok(F128::from_words(0x4052293b2522b36c, 0x51ae000000000000)),
        "rational-12"
    );
    assert_eq!(
        add64_ne(
            F128::from_words(0x4033ed930a3f5f8e, 0x9d73e514430664b1),
            F128::from_words(0xc0afa4c59f00ce24, 0xe4945d544d2d7ac9)
        ),
        Ok(F128::from_words(0xc0afa4c59f00ce24, 0xe494000000000000)),
        "rational-13"
    );
    assert_eq!(
        add64_ne(
            F128::from_words(0xc01aa6388b0cd844, 0x9c7146f0c0206cec),
            F128::from_words(0xc0a7bb7ec3bc4b2b, 0x238d013d590b3b44)
        ),
        Ok(F128::from_words(0xc0a7bb7ec3bc4b2b, 0x238e000000000000)),
        "rational-14"
    );
    assert_eq!(
        add64_ne(
            F128::from_words(0x3f6b3feefc29d05b, 0x383484e56b2f8cee),
            F128::from_words(0x3ff9735d47b31306, 0xb8406e5ef8b1615a)
        ),
        Ok(F128::from_words(0x3ff9735d47b31306, 0xb840000000000000)),
        "rational-15"
    );
    assert_eq!(
        add64_ne(
            F128::from_words(0xc06765bba6d242d5, 0x1b878e0f4a5c1679),
            F128::from_words(0xc07fa6a87c0ced04, 0xc758747c98b9636b)
        ),
        Ok(F128::from_words(0xc07fa6a87d72a8ab, 0x999c000000000000)),
        "rational-16"
    );
    assert_eq!(
        add64_ne(
            F128::from_words(0x3f87ada2f38bd6ac, 0x0979bc14034f4082),
            F128::from_words(0x3fa22cf56289f4df, 0x76496d0797838687)
        ),
        Ok(F128::from_words(0x3fa22cf562bfa93d, 0xe7c4000000000000)),
        "rational-17"
    );
    assert_eq!(
        add64_ne(
            F128::from_words(0x406b1c1e71f8f9d3, 0x9f83c3db94f26656),
            F128::from_words(0xc017365f17e9f056, 0x6fb38542338b72c8)
        ),
        Ok(F128::from_words(0x406b1c1e71f8f9d3, 0x9f84000000000000)),
        "rational-18"
    );
    assert_eq!(
        add64_ne(
            F128::from_words(0x40b316d1d4f2f338, 0xd1997d9fc706580d),
            F128::from_words(0xbf5144bbffc68296, 0xe2294cf3edb7321f)
        ),
        Ok(F128::from_words(0x40b316d1d4f2f338, 0xd19a000000000000)),
        "rational-19"
    );
    assert_eq!(
        add64_ne(
            F128::from_words(0x4085061e0e40fd35, 0x1a9606d6b79e1731),
            F128::from_words(0xc01f6a34288afab6, 0x3361343b1a74bdcc)
        ),
        Ok(F128::from_words(0x4085061e0e40fd35, 0x1a96000000000000)),
        "rational-20"
    );
    assert_eq!(
        add64_ne(
            F128::from_words(0x4038e6299b406762, 0x39f74335d8426357),
            F128::from_words(0x408878acf8159ae3, 0x30c7403b17700db5)
        ),
        Ok(F128::from_words(0x408878acf8159ae3, 0x30c8000000000000)),
        "rational-21"
    );
    assert_eq!(
        add64_ne(
            F128::from_words(0xbf9c4603f1d0122b, 0xfa6fbedc7fb712b1),
            F128::from_words(0xc05f2c8cb710998d, 0xca54bca8286b1afa)
        ),
        Ok(F128::from_words(0xc05f2c8cb710998d, 0xca54000000000000)),
        "rational-22"
    );
    assert_eq!(
        add64_ne(
            F128::from_words(0x3ffe55f46c23593e, 0x8b222784e8912b03),
            F128::from_words(0x3f855edb1d05a930, 0xad5056dd1cea2000)
        ),
        Ok(F128::from_words(0x3ffe55f46c23593e, 0x8b22000000000000)),
        "rational-23"
    );
    assert_eq!(
        add64_ne(
            F128::from_words(0xc00dc6f9731d7188, 0x88ab3f369dc88f0a),
            F128::from_words(0x3fcd0983fc817668, 0x79e30be509ff2540)
        ),
        Ok(F128::from_words(0xc00dc6f9731d7188, 0x88aa000000000000)),
        "rational-24"
    );
    assert_eq!(
        add64_ne(
            F128::from_words(0xbf5319de0ca1331b, 0xfd0f1690cd8d31e0),
            F128::from_words(0xbf8cc8f264973a9a, 0xd918d134201cf6b0)
        ),
        Ok(F128::from_words(0xbf8cc8f264973a9a, 0xd9a6000000000000)),
        "rational-25"
    );
    assert_eq!(
        add64_ne(
            F128::from_words(0xbf8b3dc86e05c0b8, 0x6f71ce41c31671c1),
            F128::from_words(0xbf6f78e83b17518a, 0x1af08066346e5bd8)
        ),
        Ok(F128::from_words(0xbf8b3dc86e1d4f3c, 0x20e6000000000000)),
        "rational-26"
    );
    assert_eq!(
        add64_ne(
            F128::from_words(0xbfcb6e000ec540c5, 0xd6ecf142257585a3),
            F128::from_words(0xbffefb6d0c5d8b55, 0xc0c1e7628d3f1dca)
        ),
        Ok(F128::from_words(0xbffefb6d0c5d8b55, 0xee82000000000000)),
        "rational-27"
    );
    assert_eq!(
        add64_ne(
            F128::from_words(0xc049a21188fe809d, 0x9ab48d201b616253),
            F128::from_words(0xbf9acc0df912452f, 0x05bd48e6edc45a9e)
        ),
        Ok(F128::from_words(0xc049a21188fe809d, 0x9ab4000000000000)),
        "rational-28"
    );
    assert_eq!(
        add64_ne(
            F128::from_words(0x3fe52ef74185e00b, 0x37bd91ed0535bee7),
            F128::from_words(0xc08700a071d58a43, 0x6e540b594e598872)
        ),
        Ok(F128::from_words(0xc08700a071d58a43, 0x6e54000000000000)),
        "rational-29"
    );
}
#[test]
fn reference_mul64_ne() {
    assert_eq!(
        mul64_ne(
            F128::from_words(0x0000000000000000, 0x0000000000000000),
            F128::from_words(0x8000000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x8000000000000000, 0x0000000000000000)),
        "zero-sign"
    );
    assert_eq!(
        mul64_ne(
            F128::from_words(0x8000000000000000, 0x0000000000000000),
            F128::from_words(0x8000000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x0000000000000000, 0x0000000000000000)),
        "negative-zero"
    );
    assert_eq!(
        mul64_ne(
            F128::from_words(0x3fff000000000000, 0x0000000000000000),
            F128::from_words(0x4000800000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x4000800000000000, 0x0000000000000000)),
        "one-third"
    );
    assert_eq!(
        mul64_ne(
            F128::from_words(0x3fff000000000000, 0x0000000000000000),
            F128::from_words(0x3f8e000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x3f8e000000000000, 0x0000000000000000)),
        "quad-tie-even"
    );
    assert_eq!(
        mul64_ne(
            F128::from_words(0x3fff000000000000, 0x0000000000000001),
            F128::from_words(0x3f8e000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x3f8e000000000000, 0x0000000000000000)),
        "quad-tie-odd"
    );
    assert_eq!(
        mul64_ne(
            F128::from_words(0x3fff000000000000, 0x0000000000000000),
            F128::from_words(0x3fbf000000000000, 0x4000000000000000)
        ),
        Ok(F128::from_words(0x3fbf000000000000, 0x4000000000000000)),
        "double-round"
    );
    assert_eq!(
        mul64_ne(
            F128::from_words(0x3fff000000000000, 0x0000400000000000),
            F128::from_words(0x3fff000000000000, 0x0000800000000000)
        ),
        Ok(F128::from_words(0x3fff000000000000, 0x0000000000000000)),
        "asymmetric-right"
    );
    assert_eq!(
        mul64_ne(
            F128::from_words(0x3fff000000000000, 0x0000800000000000),
            F128::from_words(0x3fff000000000000, 0x0000400000000000)
        ),
        Ok(F128::from_words(0x3fff000000000000, 0x0000000000000000)),
        "asymmetric-left"
    );
    assert_eq!(
        mul64_ne(
            F128::from_words(0x3fff000000000000, 0x0000000000000000),
            F128::from_words(0xbc17000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0xbc17000000000000, 0x0000000000000000)),
        "tiny-opposite"
    );
    assert_eq!(
        mul64_ne(
            F128::from_words(0xbfff000000000000, 0x0000000000000000),
            F128::from_words(0x3c17000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0xbc17000000000000, 0x0000000000000000)),
        "negative-tiny-opposite"
    );
    assert_eq!(
        mul64_ne(
            F128::from_words(0x4001000000000000, 0x0000000000000000),
            F128::from_words(0xbfc0800000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0xbfc2800000000000, 0x0000000000000000)),
        "below-binade"
    );
    assert_eq!(
        mul64_ne(
            F128::from_words(0x4001000000000000, 0x0000000000000000),
            F128::from_words(0x3fc0800000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x3fc2800000000000, 0x0000000000000000)),
        "above-binade"
    );
    assert_eq!(
        mul64_ne(
            F128::from_words(0x0000000000000000, 0x0000000000000003),
            F128::from_words(0x0000000000000000, 0x0000000000000004)
        ),
        Ok(F128::from_words(0x0000000000000000, 0x0000000000000000)),
        "subnormal-add-shortcut"
    );
    assert_eq!(
        mul64_ne(
            F128::from_words(0x0000ffffffffffff, 0xffffffffffffffff),
            F128::from_words(0x0000000000000000, 0x0000000000000002)
        ),
        Ok(F128::from_words(0x0000000000000000, 0x0000000000000000)),
        "subnormal-normal-cross"
    );
    assert_eq!(
        mul64_ne(
            F128::from_words(0x0001000000000000, 0x0000000000000000),
            F128::from_words(0x3ff5000000000000, 0x0000400000000000)
        ),
        Ok(F128::from_words(0x0000004000000000, 0x0000000000000000)),
        "subnormal-internal67"
    );
    assert_eq!(
        mul64_ne(
            F128::from_words(0x0000000000000000, 0x0000000000000001),
            F128::from_words(0x3ffe000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x0000000000000000, 0x0000000000000000)),
        "underflow-product"
    );
    assert_eq!(
        mul64_ne(
            F128::from_words(0x0000000000000000, 0x0000000000000001),
            F128::from_words(0x4000000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x0000000000000000, 0x0000000000000000)),
        "half-minimum"
    );
    assert_eq!(
        mul64_ne(
            F128::from_words(0x7e7f000000000000, 0x0000000000000000),
            F128::from_words(0x817f000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0xbfff000000000000, 0x0000000000000000)),
        "huge-opposite"
    );
    assert_eq!(
        mul64_ne(
            F128::from_words(0x3fffffffffffffff, 0xffffffffffffffff),
            F128::from_words(0x3fffffffffffffff, 0xffffffffffffffff)
        ),
        Ok(F128::from_words(0x4000ffffffffffff, 0xfffe000000000000)),
        "wide-product"
    );
    assert_eq!(
        mul64_ne(
            F128::from_words(0x3f976e0364caa050, 0x8b88599668a6328e),
            F128::from_words(0x407ec4406dfade38, 0x9adc2542fa7422b1)
        ),
        Ok(F128::from_words(0x4017434d0e068a54, 0x6f18000000000000)),
        "rational-0"
    );
    assert_eq!(
        mul64_ne(
            F128::from_words(0x401dd161ab6e8e1d, 0xd23ab5033ba5120a),
            F128::from_words(0xc0a29f9cba4fbdeb, 0x5582b8a652fe4854)
        ),
        Ok(F128::from_words(0xc0c179c51f8f4e5a, 0x0c02000000000000)),
        "rational-1"
    );
    assert_eq!(
        mul64_ne(
            F128::from_words(0x3f7526f6860ee0ad, 0x8eb452c36cff7606),
            F128::from_words(0x403e55319397bf4c, 0x338950d18897ce69)
        ),
        Ok(F128::from_words(0x3fb4891f7fcfe0ed, 0x88ae000000000000)),
        "rational-2"
    );
    assert_eq!(
        mul64_ne(
            F128::from_words(0x407ed0e5d4fcde8a, 0x5a07b21766575899),
            F128::from_words(0x402f31d543d75467, 0x8f2d1c302929ce5c)
        ),
        Ok(F128::from_words(0x40af15b28e975649, 0xc3ac000000000000)),
        "rational-3"
    );
    assert_eq!(
        mul64_ne(
            F128::from_words(0xbf6fa93875ccd29c, 0xf4a4b1d2823a55be),
            F128::from_words(0x4046687a91deaa23, 0x80e6b1de6e41b28b)
        ),
        Ok(F128::from_words(0xbfb72b617e6d8009, 0x5c66000000000000)),
        "rational-4"
    );
    assert_eq!(
        mul64_ne(
            F128::from_words(0x40563949f3796c30, 0xc257681164d2a50c),
            F128::from_words(0x4019b1ee9579f94c, 0x4eaed9ca369d4c37)
        ),
        Ok(F128::from_words(0x40710985073f0550, 0x8f3a000000000000)),
        "rational-5"
    );
    assert_eq!(
        mul64_ne(
            F128::from_words(0x40a0d40f4dedffc8, 0x4872f8cf5ea16b82),
            F128::from_words(0xbf8cbf37785626ca, 0x59ec4e6748db3775)
        ),
        Ok(F128::from_words(0xc02e98d61230802c, 0x5338000000000000)),
        "rational-6"
    );
    assert_eq!(
        mul64_ne(
            F128::from_words(0x40218c303fcc2682, 0x10e9a7dfa0c84c0c),
            F128::from_words(0x4096305c412a05f3, 0xe55ce2d5da562d0f)
        ),
        Ok(F128::from_words(0x40b8d70811f2a46b, 0xfe1a000000000000)),
        "rational-7"
    );
    assert_eq!(
        mul64_ne(
            F128::from_words(0xc018b179d909cb04, 0xba30a83aeb3d10d0),
            F128::from_words(0x40b2ede9d1d55254, 0x740d7f67c593d6da)
        ),
        Ok(F128::from_words(0xc0cca229c9160aee, 0x3ee0000000000000)),
        "rational-8"
    );
    assert_eq!(
        mul64_ne(
            F128::from_words(0xbff78794a804f48f, 0xc2b2277ae5448dac),
            F128::from_words(0xc01cafdf5234216b, 0xc3b248e12c219f6f)
        ),
        Ok(F128::from_words(0x40154a4c6f8e0798, 0xe4be000000000000)),
        "rational-9"
    );
    assert_eq!(
        mul64_ne(
            F128::from_words(0x403417b28ac30de6, 0xdaaa694cb8c61dc8),
            F128::from_words(0xbfc646abe76fb68a, 0x3786dcd6370c5dfe)
        ),
        Ok(F128::from_words(0xbffb64e92dd33d0e, 0x6156000000000000)),
        "rational-10"
    );
    assert_eq!(
        mul64_ne(
            F128::from_words(0x3f81aed55a1aa8f2, 0xb9929c8651647899),
            F128::from_words(0xc07ac9f1bc8a8794, 0x944e553eba43ebba)
        ),
        Ok(F128::from_words(0xbffd8158d9009e06, 0x87d6000000000000)),
        "rational-11"
    );
    assert_eq!(
        mul64_ne(
            F128::from_words(0x4052293b2522b36c, 0x51ae68d346a92b28),
            F128::from_words(0x3f78be450f941f1a, 0xfafaba94d160477f)
        ),
        Ok(F128::from_words(0x3fcc03129cdcf436, 0x87fc000000000000)),
        "rational-12"
    );
    assert_eq!(
        mul64_ne(
            F128::from_words(0x4033ed930a3f5f8e, 0x9d73e514430664b1),
            F128::from_words(0xc0afa4c59f00ce24, 0xe4945d544d2d7ac9)
        ),
        Ok(F128::from_words(0xc0e495a120c35e2d, 0xa986000000000000)),
        "rational-13"
    );
    assert_eq!(
        mul64_ne(
            F128::from_words(0xc01aa6388b0cd844, 0x9c7146f0c0206cec),
            F128::from_words(0xc0a7bb7ec3bc4b2b, 0x238d013d590b3b44)
        ),
        Ok(F128::from_words(0x40c36dba75a3a4fd, 0xe4c2000000000000)),
        "rational-14"
    );
    assert_eq!(
        mul64_ne(
            F128::from_words(0x3f6b3feefc29d05b, 0x383484e56b2f8cee),
            F128::from_words(0x3ff9735d47b31306, 0xb8406e5ef8b1615a)
        ),
        Ok(F128::from_words(0x3f65d01beadd47f5, 0xf032000000000000)),
        "rational-15"
    );
    assert_eq!(
        mul64_ne(
            F128::from_words(0xc06765bba6d242d5, 0x1b878e0f4a5c1679),
            F128::from_words(0xc07fa6a87c0ced04, 0xc758747c98b9636b)
        ),
        Ok(F128::from_words(0x40e8274f62be847b, 0x1c32000000000000)),
        "rational-16"
    );
    assert_eq!(
        mul64_ne(
            F128::from_words(0x3f87ada2f38bd6ac, 0x0979bc14034f4082),
            F128::from_words(0x3fa22cf56289f4df, 0x76496d0797838687)
        ),
        Ok(F128::from_words(0x3f2af916c7badccc, 0x4358000000000000)),
        "rational-17"
    );
    assert_eq!(
        mul64_ne(
            F128::from_words(0x406b1c1e71f8f9d3, 0x9f83c3db94f26656),
            F128::from_words(0xc017365f17e9f056, 0x6fb38542338b72c8)
        ),
        Ok(F128::from_words(0xc083587667da289a, 0x8bca000000000000)),
        "rational-18"
    );
    assert_eq!(
        mul64_ne(
            F128::from_words(0x40b316d1d4f2f338, 0xd1997d9fc706580d),
            F128::from_words(0xbf5144bbffc68296, 0xe2294cf3edb7321f)
        ),
        Ok(F128::from_words(0xc00561ae535d40ef, 0x2b5c000000000000)),
        "rational-19"
    );
    assert_eq!(
        mul64_ne(
            F128::from_words(0x4085061e0e40fd35, 0x1a9606d6b79e1731),
            F128::from_words(0xc01f6a34288afab6, 0x3361343b1a74bdcc)
        ),
        Ok(F128::from_words(0xc0a572dbe7c5ca62, 0x44de000000000000)),
        "rational-20"
    );
    assert_eq!(
        mul64_ne(
            F128::from_words(0x4038e6299b406762, 0x39f74335d8426357),
            F128::from_words(0x408878acf8159ae3, 0x30c7403b17700db5)
        ),
        Ok(F128::from_words(0x40c265aacb8e1e89, 0x2b3e000000000000)),
        "rational-21"
    );
    assert_eq!(
        mul64_ne(
            F128::from_words(0xbf9c4603f1d0122b, 0xfa6fbedc7fb712b1),
            F128::from_words(0xc05f2c8cb710998d, 0xca54bca8286b1afa)
        ),
        Ok(F128::from_words(0x3ffc7ebfd2aa08bd, 0x40e2000000000000)),
        "rational-22"
    );
    assert_eq!(
        mul64_ne(
            F128::from_words(0x3ffe55f46c23593e, 0x8b222784e8912b03),
            F128::from_words(0x3f855edb1d05a930, 0xad5056dd1cea2000)
        ),
        Ok(F128::from_words(0x3f84d4a8dab51646, 0x5940000000000000)),
        "rational-23"
    );
    assert_eq!(
        mul64_ne(
            F128::from_words(0xc00dc6f9731d7188, 0x88ab3f369dc88f0a),
            F128::from_words(0x3fcd0983fc817668, 0x79e30be509ff2540)
        ),
        Ok(F128::from_words(0xbfdbd7e2ca93060c, 0x14f4000000000000)),
        "rational-24"
    );
    assert_eq!(
        mul64_ne(
            F128::from_words(0xbf5319de0ca1331b, 0xfd0f1690cd8d31e0),
            F128::from_words(0xbf8cc8f264973a9a, 0xd918d134201cf6b0)
        ),
        Ok(F128::from_words(0x3ee0f71e692852db, 0x63bc000000000000)),
        "rational-25"
    );
    assert_eq!(
        mul64_ne(
            F128::from_words(0xbf8b3dc86e05c0b8, 0x6f71ce41c31671c1),
            F128::from_words(0xbf6f78e83b17518a, 0x1af08066346e5bd8)
        ),
        Ok(F128::from_words(0x3efbd3dea8964bda, 0x9b0e000000000000)),
        "rational-26"
    );
    assert_eq!(
        mul64_ne(
            F128::from_words(0xbfcb6e000ec540c5, 0xd6ecf142257585a3),
            F128::from_words(0xbffefb6d0c5d8b55, 0xc0c1e7628d3f1dca)
        ),
        Ok(F128::from_words(0x3fcb6abb027a5797, 0xd19a000000000000)),
        "rational-27"
    );
    assert_eq!(
        mul64_ne(
            F128::from_words(0xc049a21188fe809d, 0x9ab48d201b616253),
            F128::from_words(0xbf9acc0df912452f, 0x05bd48e6edc45a9e)
        ),
        Ok(F128::from_words(0x3fe577a729e7143f, 0x35be000000000000)),
        "rational-28"
    );
    assert_eq!(
        mul64_ne(
            F128::from_words(0x3fe52ef74185e00b, 0x37bd91ed0535bee7),
            F128::from_words(0xc08700a071d58a43, 0x6e540b594e598872)
        ),
        Ok(F128::from_words(0xc06d2fb522c6aefd, 0xf1a2000000000000)),
        "rational-29"
    );
}
#[test]
fn reference_div64_ne() {
    assert_eq!(
        div64_ne(
            F128::from_words(0x0000000000000000, 0x0000000000000000),
            F128::from_words(0x8000000000000000, 0x0000000000000000)
        ),
        Err(ArithmeticError::DivisionByZero),
        "zero-sign"
    );
    assert_eq!(
        div64_ne(
            F128::from_words(0x8000000000000000, 0x0000000000000000),
            F128::from_words(0x8000000000000000, 0x0000000000000000)
        ),
        Err(ArithmeticError::DivisionByZero),
        "negative-zero"
    );
    assert_eq!(
        div64_ne(
            F128::from_words(0x3fff000000000000, 0x0000000000000000),
            F128::from_words(0x4000800000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x3ffd555555555555, 0x5556000000000000)),
        "one-third"
    );
    assert_eq!(
        div64_ne(
            F128::from_words(0x3fff000000000000, 0x0000000000000000),
            F128::from_words(0x3f8e000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x4070000000000000, 0x0000000000000000)),
        "quad-tie-even"
    );
    assert_eq!(
        div64_ne(
            F128::from_words(0x3fff000000000000, 0x0000000000000001),
            F128::from_words(0x3f8e000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x4070000000000000, 0x0000000000000000)),
        "quad-tie-odd"
    );
    assert_eq!(
        div64_ne(
            F128::from_words(0x3fff000000000000, 0x0000000000000000),
            F128::from_words(0x3fbf000000000000, 0x4000000000000000)
        ),
        Ok(F128::from_words(0x403effffffffffff, 0x8000000000000000)),
        "double-round"
    );
    assert_eq!(
        div64_ne(
            F128::from_words(0x3fff000000000000, 0x0000400000000000),
            F128::from_words(0x3fff000000000000, 0x0000800000000000)
        ),
        Ok(F128::from_words(0x3fff000000000000, 0x0000000000000000)),
        "asymmetric-right"
    );
    assert_eq!(
        div64_ne(
            F128::from_words(0x3fff000000000000, 0x0000800000000000),
            F128::from_words(0x3fff000000000000, 0x0000400000000000)
        ),
        Ok(F128::from_words(0x3fff000000000000, 0x0000000000000000)),
        "asymmetric-left"
    );
    assert_eq!(
        div64_ne(
            F128::from_words(0x3fff000000000000, 0x0000000000000000),
            F128::from_words(0xbc17000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0xc3e7000000000000, 0x0000000000000000)),
        "tiny-opposite"
    );
    assert_eq!(
        div64_ne(
            F128::from_words(0xbfff000000000000, 0x0000000000000000),
            F128::from_words(0x3c17000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0xc3e7000000000000, 0x0000000000000000)),
        "negative-tiny-opposite"
    );
    assert_eq!(
        div64_ne(
            F128::from_words(0x4001000000000000, 0x0000000000000000),
            F128::from_words(0xbfc0800000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0xc03f555555555555, 0x5556000000000000)),
        "below-binade"
    );
    assert_eq!(
        div64_ne(
            F128::from_words(0x4001000000000000, 0x0000000000000000),
            F128::from_words(0x3fc0800000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x403f555555555555, 0x5556000000000000)),
        "above-binade"
    );
    assert_eq!(
        div64_ne(
            F128::from_words(0x0000000000000000, 0x0000000000000003),
            F128::from_words(0x0000000000000000, 0x0000000000000004)
        ),
        Ok(F128::from_words(0x3ffe800000000000, 0x0000000000000000)),
        "subnormal-add-shortcut"
    );
    assert_eq!(
        div64_ne(
            F128::from_words(0x0000ffffffffffff, 0xffffffffffffffff),
            F128::from_words(0x0000000000000000, 0x0000000000000002)
        ),
        Ok(F128::from_words(0x406e000000000000, 0x0000000000000000)),
        "subnormal-normal-cross"
    );
    assert_eq!(
        div64_ne(
            F128::from_words(0x0001000000000000, 0x0000000000000000),
            F128::from_words(0x3ff5000000000000, 0x0000400000000000)
        ),
        Ok(F128::from_words(0x000b000000000000, 0x0000000000000000)),
        "subnormal-internal67"
    );
    assert_eq!(
        div64_ne(
            F128::from_words(0x0000000000000000, 0x0000000000000001),
            F128::from_words(0x3ffe000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x0000000000000000, 0x0000000000000000)),
        "underflow-product"
    );
    assert_eq!(
        div64_ne(
            F128::from_words(0x0000000000000000, 0x0000000000000001),
            F128::from_words(0x4000000000000000, 0x0000000000000000)
        ),
        Ok(F128::from_words(0x0000000000000000, 0x0000000000000000)),
        "half-minimum"
    );
    assert_eq!(
        div64_ne(
            F128::from_words(0x7e7f000000000000, 0x0000000000000000),
            F128::from_words(0x817f000000000000, 0x0000000000000000)
        ),
        Err(ArithmeticError::Overflow),
        "huge-opposite"
    );
    assert_eq!(
        div64_ne(
            F128::from_words(0x3fffffffffffffff, 0xffffffffffffffff),
            F128::from_words(0x3fffffffffffffff, 0xffffffffffffffff)
        ),
        Ok(F128::from_words(0x3fff000000000000, 0x0000000000000000)),
        "wide-product"
    );
    assert_eq!(
        div64_ne(
            F128::from_words(0x3f976e0364caa050, 0x8b88599668a6328e),
            F128::from_words(0x407ec4406dfade38, 0x9adc2542fa7422b1)
        ),
        Ok(F128::from_words(0x3f179e5e4d058a92, 0x643c000000000000)),
        "rational-0"
    );
    assert_eq!(
        div64_ne(
            F128::from_words(0x401dd161ab6e8e1d, 0xd23ab5033ba5120a),
            F128::from_words(0xc0a29f9cba4fbdeb, 0x5582b8a652fe4854)
        ),
        Ok(F128::from_words(0xbf7a1ea7e52a55f4, 0x95a8000000000000)),
        "rational-1"
    );
    assert_eq!(
        div64_ne(
            F128::from_words(0x3f7526f6860ee0ad, 0x8eb452c36cff7606),
            F128::from_words(0x403e55319397bf4c, 0x338950d18897ce69)
        ),
        Ok(F128::from_words(0x3f35baa0273e3e13, 0x98ac000000000000)),
        "rational-2"
    );
    assert_eq!(
        div64_ne(
            F128::from_words(0x407ed0e5d4fcde8a, 0x5a07b21766575899),
            F128::from_words(0x402f31d543d75467, 0x8f2d1c302929ce5c)
        ),
        Ok(F128::from_words(0x404e8525797037e9, 0x2216000000000000)),
        "rational-3"
    );
    assert_eq!(
        div64_ne(
            F128::from_words(0xbf6fa93875ccd29c, 0xf4a4b1d2823a55be),
            F128::from_words(0x4046687a91deaa23, 0x80e6b1de6e41b28b)
        ),
        Ok(F128::from_words(0xbf282dfa393bd997, 0x3dfa000000000000)),
        "rational-4"
    );
    assert_eq!(
        div64_ne(
            F128::from_words(0x40563949f3796c30, 0xc257681164d2a50c),
            F128::from_words(0x4019b1ee9579f94c, 0x4eaed9ca369d4c37)
        ),
        Ok(F128::from_words(0x403b71a6f5e26894, 0xd1f2000000000000)),
        "rational-5"
    );
    assert_eq!(
        div64_ne(
            F128::from_words(0x40a0d40f4dedffc8, 0x4872f8cf5ea16b82),
            F128::from_words(0xbf8cbf37785626ca, 0x59ec4e6748db3775)
        ),
        Ok(F128::from_words(0xc1130bee6384b339, 0x419e000000000000)),
        "rational-6"
    );
    assert_eq!(
        div64_ne(
            F128::from_words(0x40218c303fcc2682, 0x10e9a7dfa0c84c0c),
            F128::from_words(0x4096305c412a05f3, 0xe55ce2d5da562d0f)
        ),
        Ok(F128::from_words(0x3f8a4d3cc452eb11, 0x654c000000000000)),
        "rational-7"
    );
    assert_eq!(
        div64_ne(
            F128::from_words(0xc018b179d909cb04, 0xba30a83aeb3d10d0),
            F128::from_words(0x40b2ede9d1d55254, 0x740d7f67c593d6da)
        ),
        Ok(F128::from_words(0xbf64c15975881f48, 0xc8c4000000000000)),
        "rational-8"
    );
    assert_eq!(
        div64_ne(
            F128::from_words(0xbff78794a804f48f, 0xc2b2277ae5448dac),
            F128::from_words(0xc01cafdf5234216b, 0xc3b248e12c219f6f)
        ),
        Ok(F128::from_words(0x3fd9d03b99468b80, 0x6590000000000000)),
        "rational-9"
    );
    assert_eq!(
        div64_ne(
            F128::from_words(0x403417b28ac30de6, 0xdaaa694cb8c61dc8),
            F128::from_words(0xbfc646abe76fb68a, 0x3786dcd6370c5dfe)
        ),
        Ok(F128::from_words(0xc06cb6605d52dc91, 0x3c74000000000000)),
        "rational-10"
    );
    assert_eq!(
        div64_ne(
            F128::from_words(0x3f81aed55a1aa8f2, 0xb9929c8651647899),
            F128::from_words(0xc07ac9f1bc8a8794, 0x944e553eba43ebba)
        ),
        Ok(F128::from_words(0xbf05e1b05f75ca11, 0x3094000000000000)),
        "rational-11"
    );
    assert_eq!(
        div64_ne(
            F128::from_words(0x4052293b2522b36c, 0x51ae68d346a92b28),
            F128::from_words(0x3f78be450f941f1a, 0xfafaba94d160477f)
        ),
        Ok(F128::from_words(0x40d85502774e847e, 0xeac0000000000000)),
        "rational-12"
    );
    assert_eq!(
        div64_ne(
            F128::from_words(0x4033ed930a3f5f8e, 0x9d73e514430664b1),
            F128::from_words(0xc0afa4c59f00ce24, 0xe4945d544d2d7ac9)
        ),
        Ok(F128::from_words(0xbf832c4b1bff885c, 0x31bc000000000000)),
        "rational-13"
    );
    assert_eq!(
        div64_ne(
            F128::from_words(0xc01aa6388b0cd844, 0x9c7146f0c0206cec),
            F128::from_words(0xc0a7bb7ec3bc4b2b, 0x238d013d590b3b44)
        ),
        Ok(F128::from_words(0x3f71e77086151212, 0x527e000000000000)),
        "rational-14"
    );
    assert_eq!(
        div64_ne(
            F128::from_words(0x3f6b3feefc29d05b, 0x383484e56b2f8cee),
            F128::from_words(0x3ff9735d47b31306, 0xb8406e5ef8b1615a)
        ),
        Ok(F128::from_words(0x3f70b917a19b2147, 0xc254000000000000)),
        "rational-15"
    );
    assert_eq!(
        div64_ne(
            F128::from_words(0xc06765bba6d242d5, 0x1b878e0f4a5c1679),
            F128::from_words(0xc07fa6a87c0ced04, 0xc758747c98b9636b)
        ),
        Ok(F128::from_words(0x3fe6b159d72ba0fd, 0x04d0000000000000)),
        "rational-16"
    );
    assert_eq!(
        div64_ne(
            F128::from_words(0x3f87ada2f38bd6ac, 0x0979bc14034f4082),
            F128::from_words(0x3fa22cf56289f4df, 0x76496d0797838687)
        ),
        Ok(F128::from_words(0x3fe46d749b8b6eb0, 0x13de000000000000)),
        "rational-17"
    );
    assert_eq!(
        div64_ne(
            F128::from_words(0x406b1c1e71f8f9d3, 0x9f83c3db94f26656),
            F128::from_words(0xc017365f17e9f056, 0x6fb38542338b72c8)
        ),
        Ok(F128::from_words(0xc052d4b160130b25, 0x8156000000000000)),
        "rational-18"
    );
    assert_eq!(
        div64_ne(
            F128::from_words(0x40b316d1d4f2f338, 0xd1997d9fc706580d),
            F128::from_words(0xbf5144bbffc68296, 0xe2294cf3edb7321f)
        ),
        Ok(F128::from_words(0xc160b79b84ccb82a, 0xfc1e000000000000)),
        "rational-19"
    );
    assert_eq!(
        div64_ne(
            F128::from_words(0x4085061e0e40fd35, 0x1a9606d6b79e1731),
            F128::from_words(0xc01f6a34288afab6, 0x3361343b1a74bdcc)
        ),
        Ok(F128::from_words(0xc064728563863381, 0x866c000000000000)),
        "rational-20"
    );
    assert_eq!(
        div64_ne(
            F128::from_words(0x4038e6299b406762, 0x39f74335d8426357),
            F128::from_words(0x408878acf8159ae3, 0x30c7403b17700db5)
        ),
        Ok(F128::from_words(0x3faf4a6918d05d27, 0xb1a0000000000000)),
        "rational-21"
    );
    assert_eq!(
        div64_ne(
            F128::from_words(0xbf9c4603f1d0122b, 0xfa6fbedc7fb712b1),
            F128::from_words(0xc05f2c8cb710998d, 0xca54bca8286b1afa)
        ),
        Ok(F128::from_words(0x3f3c15b0e6d23ded, 0xf6a4000000000000)),
        "rational-22"
    );
    assert_eq!(
        div64_ne(
            F128::from_words(0x3ffe55f46c23593e, 0x8b222784e8912b03),
            F128::from_words(0x3f855edb1d05a930, 0xad5056dd1cea2000)
        ),
        Ok(F128::from_words(0x4077f302bb839d1d, 0x9070000000000000)),
        "rational-23"
    );
    assert_eq!(
        div64_ne(
            F128::from_words(0xc00dc6f9731d7188, 0x88ab3f369dc88f0a),
            F128::from_words(0x3fcd0983fc817668, 0x79e30be509ff2540)
        ),
        Ok(F128::from_words(0xc03fb6ab436cb11d, 0xe984000000000000)),
        "rational-24"
    );
    assert_eq!(
        div64_ne(
            F128::from_words(0xbf5319de0ca1331b, 0xfd0f1690cd8d31e0),
            F128::from_words(0xbf8cc8f264973a9a, 0xd918d134201cf6b0)
        ),
        Ok(F128::from_words(0x3fc53bd3aea098b8, 0x1286000000000000)),
        "rational-25"
    );
    assert_eq!(
        div64_ne(
            F128::from_words(0xbf8b3dc86e05c0b8, 0x6f71ce41c31671c1),
            F128::from_words(0xbf6f78e83b17518a, 0x1af08066346e5bd8)
        ),
        Ok(F128::from_words(0x401aafaf2743904a, 0xb16e000000000000)),
        "rational-26"
    );
    assert_eq!(
        div64_ne(
            F128::from_words(0xbfcb6e000ec540c5, 0xd6ecf142257585a3),
            F128::from_words(0xbffefb6d0c5d8b55, 0xc0c1e7628d3f1dca)
        ),
        Ok(F128::from_words(0x3fcb714ca6a927d2, 0x0396000000000000)),
        "rational-27"
    );
    assert_eq!(
        div64_ne(
            F128::from_words(0xc049a21188fe809d, 0x9ab48d201b616253),
            F128::from_words(0xbf9acc0df912452f, 0x05bd48e6edc45a9e)
        ),
        Ok(F128::from_words(0x40add145f0d68fd1, 0xc9cc000000000000)),
        "rational-28"
    );
    assert_eq!(
        div64_ne(
            F128::from_words(0x3fe52ef74185e00b, 0x37bd91ed0535bee7),
            F128::from_words(0xc08700a071d58a43, 0x6e540b594e598872)
        ),
        Ok(F128::from_words(0xbf5d2e39d6fbe15e, 0x8556000000000000)),
        "rational-29"
    );
}

#[test]
fn encoding_stores_and_errors() {
    for bits in [
        0u64,
        1,
        0x8000000000000000,
        0x0010000000000000,
        0x3ff0000000000000,
        0x400921fb54442d18,
        0x7fefffffffffffff,
    ] {
        let wide = from_f64(f64::from_bits(bits)).unwrap();
        assert_eq!(to_f64_nearest(wide).unwrap().to_bits(), bits);
        assert_eq!(from_ext80(to_ext80_nearest(wide).unwrap()).unwrap(), wide);
    }
    assert_eq!(from_f64(f64::INFINITY), Err(ArithmeticError::NonFinite));
    assert_eq!(
        from_ext80(Ext80 {
            sign_exp: 1,
            significand: 1
        }),
        Err(ArithmeticError::UnsupportedEncoding)
    );
    let maximum = F128::from_words(0x7ffeffffffffffff, u64::MAX);
    assert_eq!(
        add113_nearest(maximum, maximum),
        Err(ArithmeticError::Overflow)
    );
    assert_eq!(
        div113_nearest(F128::zero(false), F128::zero(false)),
        Err(ArithmeticError::DivisionByZero)
    );
    let half = from_scaled_u128(false, 1, -1075).unwrap();
    let three_halves = from_scaled_u128(false, 3, -1075).unwrap();
    assert_eq!(to_f64_nearest(half).unwrap().to_bits(), 0);
    assert_eq!(to_f64_nearest(three_halves).unwrap().to_bits(), 2);
    let half_ext = from_scaled_u128(false, 1, -16446).unwrap();
    let three_halves_ext = from_scaled_u128(false, 3, -16446).unwrap();
    assert_eq!(
        to_ext80_nearest(half_ext).unwrap(),
        Ext80 {
            sign_exp: 0,
            significand: 0
        }
    );
    assert_eq!(
        to_ext80_nearest(three_halves_ext).unwrap(),
        Ext80 {
            sign_exp: 0,
            significand: 2
        }
    );
}

#[test]
fn asymmetric_and_double_rounding_controls_are_distinct() {
    let one = from_f64(1.0).unwrap();
    let a = add113_nearest(one, from_scaled_u128(false, 1, -66).unwrap()).unwrap();
    let b = add113_nearest(one, from_scaled_u128(false, 1, -65).unwrap()).unwrap();
    assert_ne!(mul_e67(a, b).unwrap(), mul_e67(b, a).unwrap());
    let small = add113_nearest(
        from_scaled_u128(false, 1, -64).unwrap(),
        from_scaled_u128(false, 1, -114).unwrap(),
    )
    .unwrap();
    assert_eq!(add64_ne(one, small).unwrap(), one);
    // Direct exact rounding would increment; the intermediate RN113 erases
    // the below-half remainder, making the final RN64 an even tie at one.
    assert_eq!(
        to_ext80_nearest(add64_ne(one, small).unwrap())
            .unwrap()
            .significand,
        1u64 << 63
    );
}
