use super::*;

#[test]
fn source_zero_band_uses_exact_epsilon80_binary64_neighbors_and_signed_zero() {
    let source = GenerationRevision::SourceClassicV2;
    let historical = GenerationRevision::HistoricalV1;
    // Exact Fraction proof brackets epsilon80 strictly between681 and682.
    for bits in [
        0,
        1,
        0x8000000000000000,
        0x8000000000000001,
        0x3d3c25c268497680,
        0x3d3c25c268497681,
        0xbd3c25c268497680,
        0xbd3c25c268497681,
    ] {
        let x = f64::from_bits(bits);
        assert_eq!(
            item_rotation(Vec2::new(x, -1.0), false, 0.7, source).to_bits(),
            (-0.0_f64).to_bits()
        );
        assert_eq!(
            item_rotation(Vec2::new(x, -1.0), true, 0.7, source).to_bits(),
            0.7_f64.to_bits()
        );
        assert_eq!(
            item_rotation(Vec2::new(-1.0, x), false, 0.7, source).to_bits(),
            (-std::f64::consts::PI).to_bits()
        );
    }
    for bits in [
        0x3d3c25c268497682,
        0x3d3c25c268497683,
        0xbd3c25c268497682,
        0xbd3c25c268497683,
    ] {
        let velocity = Vec2::new(f64::from_bits(bits), -1.0);
        let legacy = item_rotation(velocity, false, 0.7, historical);
        assert!(legacy.is_finite() && legacy != 0.0);
        assert_eq!(
            item_rotation(velocity, false, 0.7, source).to_bits(),
            legacy.to_bits(),
            "outside band selects the unchanged existing angle expression; no native atan claim"
        );
    }
    let known = Vec2::new(f64::from_bits(0xbd3c25c268497681), 0.0);
    assert_eq!(
        item_rotation(known, false, 0.7, source).to_bits(),
        (-0.0_f64).to_bits()
    );
    assert_eq!(
        item_rotation(known, false, 0.7, historical).to_bits(),
        (-std::f64::consts::PI).to_bits()
    );
    assert_eq!(
        item_rotation(
            Vec2::new(-f64::from_bits(0x3d3c25c268497682), 0.0),
            false,
            0.7,
            source
        )
        .to_bits(),
        (-std::f64::consts::PI).to_bits()
    );
}
