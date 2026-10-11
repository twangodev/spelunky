use spelunky_core::{Vec2, catalog};

#[test]
fn rotating_arrow_rectangle_uses_native_cached_bounds_and_raw_corners() {
    let mask = catalog::instance_mask(
        "oArrow",
        "sArrowRight",
        Vec2::new(390.0, 165.0),
        0.0,
        Vec2::new(1.0, 1.0),
        -(std::f64::consts::PI + (1.4_f64 / 8.0).atan()),
    );
    // Native updater4AA958/query4AC834, not alpha-overlap, give the integer
    // inclusive envelope [387,161,394,167] for this naturally emitted arrow.
    for (first, second, expected) in [
        (Vec2::new(370.0, 160.0), Vec2::new(386.0, 176.0), false),
        (Vec2::new(371.0, 160.0), Vec2::new(387.0, 176.0), true),
        (Vec2::new(370.0, 160.0), Vec2::new(386.75, 176.0), false),
        (Vec2::new(394.75, 161.0), Vec2::new(394.75, 161.0), true),
        (Vec2::new(395.0, 161.0), Vec2::new(395.0, 161.0), false),
        (Vec2::new(387.0, 160.75), Vec2::new(387.0, 160.75), false),
        (Vec2::new(387.0, 161.0), Vec2::new(387.0, 161.0), true),
        (Vec2::new(387.0, 167.75), Vec2::new(387.0, 167.75), true),
        (Vec2::new(387.0, 168.0), Vec2::new(387.0, 168.0), false),
    ] {
        assert_eq!(mask.intersects_rectangle_bbox(first, second), expected);
        assert_eq!(mask.intersects_rectangle_bbox(second, first), expected);
    }
    assert!(!mask.intersects_rectangle_bbox(Vec2::new(f64::NAN, 0.0), Vec2::default()));
}

#[test]
fn imprecise_rectangle_retains_aggregate_frames_and_fractional_edge_rounding() {
    for animation in [0.0, 1.0, 2.0] {
        let bat = catalog::instance_mask(
            "oBat",
            "sBatLeft",
            Vec2::new(80.0, 80.0),
            animation,
            Vec2::new(1.0, 1.0),
            0.0,
        );
        // Aggregate bottom93 survives selected frame1's shorter bottom91.
        assert!(bat.intersects_rectangle_bbox(Vec2::new(80.0, 93.0), Vec2::new(84.0, 97.0)));
    }
    let arrow = catalog::instance_mask(
        "oArrow",
        "sArrowRight",
        Vec2::new(100.5, 100.5),
        0.0,
        Vec2::new(1.0, 1.0),
        0.0,
    );
    // Translated-edge nearest-even ties produce [96,100,104,104].
    assert!(arrow.intersects_rectangle_bbox(Vec2::new(96.0, 100.0), Vec2::new(96.0, 100.0)));
    assert!(!arrow.intersects_rectangle_bbox(Vec2::new(96.0, 99.75), Vec2::new(96.0, 99.75)));
}
