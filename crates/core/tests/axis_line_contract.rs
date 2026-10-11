//! Frozen original-byte query results, rather than a second Rust geometry oracle.
use spelunky_core::{Area, CollisionPrecision, Vec2, catalog};

#[test]
fn actual_push_block_axis_lines_match_forty_two_original_engine_controls() {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/axis_line_native_cases.json")).unwrap();
    let cases = cases.as_array().unwrap();
    assert_eq!(cases.len(), 42);
    for case in cases {
        let number = |key: &str, index: usize| case[key][index].as_f64().unwrap();
        let mask = catalog::instance_mask(
            "oPushBlock",
            "sBlock",
            Vec2::new(number("pos", 0), number("pos", 1)),
            case["animation"].as_f64().unwrap(),
            Vec2::new(1.0, 1.0),
            0.0,
        );
        let precision = if case["precision"].as_u64().unwrap() == 0 {
            CollisionPrecision::BoundingBox
        } else {
            CollisionPrecision::Precise
        };
        assert_eq!(
            mask.intersects_axis_line(
                Vec2::new(number("query", 0), number("query", 1)),
                Vec2::new(number("query", 2), number("query", 3)),
                precision,
            ),
            case["native_hit"].as_bool().unwrap(),
            "original engine case {}",
            case["name"]
        );
    }
}

#[test]
fn rectangle_sprite_line_samples_selected_frame_inside_aggregate_bbox() {
    // Independent original Bat PNG extents: frame0[0,4,15,13],
    // frame1[0,4,15,11], sprite aggregate[0,3,15,13], origin0,0.
    for (animation, precise_hit) in [(0.0, true), (1.0, false)] {
        let mask = catalog::instance_mask(
            "oBat",
            "sBatLeft",
            Vec2::new(80.0, 80.0),
            animation,
            Vec2::new(1.0, 1.0),
            0.0,
        );
        let point = Vec2::new(88.0, 93.0);
        assert_eq!(
            mask.intersects_axis_line(point, point, CollisionPrecision::Precise),
            precise_hit,
        );
        assert!(mask.intersects_axis_line(point, point, CollisionPrecision::BoundingBox));
    }
}

#[test]
fn tile_line_query_distinguishes_source_darkfall_transparent_corner() {
    // Original sDarkFall frame0 pixel(0,0) is transparent; (1,0) is opaque.
    // The previously retained actual Game.step DarkFall regression establishes
    // this source alpha distinction independently of the line API.
    let mut world = spelunky_core::World::new(10, 10, Area::Mines, 1);
    world.set_tile(5, 5, "oDarkFall");
    for (x, precise_hit) in [(80.0, false), (81.0, true)] {
        let point = Vec2::new(x, 80.0);
        assert_eq!(
            world.tiles_line_precision(point, point, "oSolid", CollisionPrecision::Precise),
            precise_hit,
        );
        assert!(world.tiles_line_precision(
            point,
            point,
            "oSolid",
            CollisionPrecision::BoundingBox
        ));
    }
}
