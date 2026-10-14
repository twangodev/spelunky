//! Own Create's source collision_point(x,y,oWater,0,0), before any Step.
//! Transforms/activation/surface sprite are disclosed initial query contexts;
//! these constructor controls do not claim a stock generated traversal route.
use spelunky_core::{CollisionPrecision, Game, RngAlgorithm, RulesPreset, Vec2, level::Level};

const ACTORS: [&str; 4] = ["oFrog", "oFireFrog", "oMonkey", "oZombie"];
fn configurations(mut run: impl FnMut(RulesPreset, RngAlgorithm)) {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for algorithm in [RngAlgorithm::LegacySplitMix64V1, RngAlgorithm::Gm8Lcg32V1] {
            run(rules, algorithm);
        }
    }
}

// Independent literal generator and source rand(0,1) mapping. Setup may have
// its own Create draws; capture its completed state before the actor's birth.
fn source_facing(state: u64, algorithm: RngAlgorithm) -> (u64, i8) {
    let (next, direction) = match algorithm {
        RngAlgorithm::Gm8Lcg32V1 => {
            let next = u64::from((state as u32).wrapping_mul(0x08088405).wrapping_add(1));
            (next, (next * 2) >> 32)
        }
        RngAlgorithm::LegacySplitMix64V1 => {
            let next = state.wrapping_add(0x9e3779b97f4a7c15);
            let mut z = next;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
            z ^= z >> 31;
            (next, z % 2)
        }
    };
    (next, if direction == 0 { -1 } else { 1 })
}
fn check_birth(g: &mut Game, actor: &str, pos: Vec2, wet: bool, algorithm: RngAlgorithm) {
    let (next, facing) = source_facing(g.rng.state_bits(), algorithm);
    let count = g.world.entities.len();
    let id = g.spawn(actor, pos);
    let e = g.world.entity(id).unwrap();
    assert_eq!(e.pos, pos);
    assert_eq!(e.previous_pos, pos);
    assert_eq!(e.age, 0);
    assert_eq!(e.vel, Vec2::default());
    assert_eq!(
        e.facing, facing,
        "{actor} literal own facing before water query"
    );
    assert_eq!(e.value("swimming"), f64::from(wet), "{actor}/{pos:?}");
    assert_eq!(e.value("enemy_context_created"), 1.0);
    if actor == "oZombie" {
        assert_eq!(e.value("source_facing_pending"), 0.0);
    } else {
        assert_eq!(e.value("jungle_context_created"), 1.0);
    }
    assert_eq!(e.state, if actor == "oMonkey" { 5 } else { 0 });
    assert_eq!(e.image_speed(), 0.4);
    assert_eq!(
        g.world.entities.len(),
        count + 1,
        "Create has no Splash child"
    );
    assert_eq!(
        g.rng.state_bits(),
        next,
        "{actor} exactly one facing draw, wet or dry"
    );
}

#[test]
fn own_creates_ignore_inactive_or_retired_dynamic_water_without_skipping_facing_draw() {
    configurations(|rules, algorithm| {
        for actor in ACTORS {
            for (alive, active, wet) in [
                (true, true, true),
                (true, false, false),
                (false, true, false),
            ] {
                let mut g = Game::shell_with_algorithm(42, rules, algorithm).unwrap();
                let water = g.world.spawn("oWaterSwim", Vec2::new(80.0, 80.0));
                let e = g.world.entity_mut(water).unwrap();
                assert!(e.mask_contains(Vec2::new(88.0, 88.0), CollisionPrecision::BoundingBox));
                e.engine_active = active;
                e.alive = alive;
                check_birth(&mut g, actor, Vec2::new(88.0, 88.0), wet, algorithm);
                let unchanged = g.world.entities.iter().find(|e| e.id == water).unwrap();
                assert_eq!(
                    unchanged.engine_active, active,
                    "query does not activate water"
                );
                assert_eq!(
                    unchanged.alive, alive,
                    "query does not revive retired water"
                );
            }
        }
    });
}

#[test]
fn own_creates_use_mirrored_instance_bbox_instead_of_movement_default_bounds() {
    configurations(|rules, algorithm| {
        for actor in ACTORS {
            for (x, wet) in [(72.0, true), (88.0, false)] {
                let mut g = Game::shell_with_algorithm(42, rules, algorithm).unwrap();
                let water = g.world.spawn("oWaterSwim", Vec2::new(80.0, 80.0));
                let e = g.world.entity_mut(water).unwrap();
                e.data.insert("image_xscale".into(), -1.0);
                let point = Vec2::new(x, 88.0);
                assert_eq!(e.mask_contains(point, CollisionPrecision::BoundingBox), wet);
                assert_eq!(
                    e.bounds().contains(point),
                    !wet,
                    "old default-bounds premise differs"
                );
                check_birth(&mut g, actor, point, wet, algorithm);
            }
        }
    });
}

#[test]
fn lava_surface_bbox_at_constructor_origin_is_invariant_across_declared_frames() {
    configurations(|rules, algorithm| {
        for actor in ACTORS {
            for (frame, y, wet) in [(0.0, 80.0, true), (0.0, 81.0, true), (1.0, 80.0, true)] {
                let mut g = Game::shell_with_algorithm(42, rules, algorithm).unwrap();
                let water = g.spawn("oLava", Vec2::new(80.0, 80.0));
                let e = g.world.entity_mut(water).unwrap();
                // Completed source SetupWalls surface assignment, not a claim
                // that raw Lava Create itself changes sLava to sLavaTop.
                e.set_sprite("sLavaTop");
                e.animation = frame;
                let point = Vec2::new(88.0, y);
                // Source 4AC73F uses cached aggregate bbox [0,0,15,15],
                // not frame zero's smooth [0,1,15,15]. Both frames hit y80.
                assert_eq!(e.sprite_bounds().y, if frame == 0.0 { 81.0 } else { 80.0 });
                assert_eq!(e.mask_contains(point, CollisionPrecision::BoundingBox), wet);
                assert!(e.bounds().contains(point));
                check_birth(&mut g, actor, point, wet, algorithm);
            }
        }
    });
}

#[test]
fn tile_lava_surface_uses_aggregate_bbox_across_frames_and_respects_active_flag() {
    configurations(|rules, algorithm| {
        for actor in ACTORS {
            for (frame, active, wet) in [
                (0.0, true, true),
                (1.0, true, true),
                (0.0, false, false),
                (1.0, false, false),
            ] {
                let mut g = Game::shell_with_algorithm(42, rules, algorithm).unwrap();
                g.world.set_tile(5, 5, "oLava");
                let width = g.world.width;
                g.world.tiles[5 * width + 5].engine_active = active;
                let metadata = g.world.spawn("oTerrainMetadata", Vec2::new(80.0, 80.0));
                let e = g.world.entity_mut(metadata).unwrap();
                e.text.insert("terrain_sprite".into(), "sLavaTop".into());
                e.animation = frame;
                let point = Vec2::new(88.0, 80.0);
                assert_eq!(
                    g.world
                        .tile_collision_mask(5, 5)
                        .contains(point, CollisionPrecision::BoundingBox),
                    true,
                    "aggregate source bbox includes row80 in both frames"
                );
                check_birth(&mut g, actor, point, wet, algorithm);
                assert_eq!(
                    g.world.tiles[5 * width + 5].engine_active,
                    active,
                    "query does not activate terrain"
                );
            }
        }
    });
}

#[test]
fn fractional_water_origin_uses_cached_integer_bbox_without_rounding_query_point() {
    configurations(|rules, algorithm| {
        for actor in ACTORS {
            for (y, wet) in [(80.0, true), (96.0, false)] {
                let mut g = Game::shell_with_algorithm(42, rules, algorithm).unwrap();
                let water = g.world.spawn("oWaterSwim", Vec2::new(80.0, 80.25));
                let e = g.world.entity(water).unwrap();
                let point = Vec2::new(88.0, y);
                // Literal source bbox update RTE(80.25)=80/RTE(95.25)=95;
                // raw point compare is T <= y < B+1, hence 80 <= y < 96.
                // Smooth/movement bbox still spans [80.25,96.25).
                assert_eq!(e.mask_contains(point, CollisionPrecision::BoundingBox), wet);
                assert_eq!(e.sprite_bounds().contains(point), !wet);
                assert_eq!(e.bounds().contains(point), !wet);
                check_birth(&mut g, actor, point, wet, algorithm);
                assert_eq!(g.world.entity(water).unwrap().pos, Vec2::new(80.0, 80.25));
            }
        }
    });
}

#[test]
fn ordinary_editor_lava_tile_supports_wet_and_adjacent_dry_constructor_controls() {
    configurations(|rules, algorithm| {
        for actor in ACTORS {
            for (x, wet) in [(80.0, true), (64.0, false)] {
                let mut level = Level::default();
                level.set_tile(4, 4, 'l').unwrap();
                let mut g = Game::shell_with_algorithm(42, rules, algorithm).unwrap();
                g.load_custom(&level, "WATER_CREATE_CONTROL", "NONE");
                assert!(g.flags.contains("custom_level"));
                assert_eq!(g.world.tile(5, 5).unwrap().object, "oLava");
                let point = Vec2::new(x, 80.0);
                assert_eq!(g.world.tiles_point(point, "oWater"), wet);
                // A real synchronous Game producer over ordinary editor-loaded
                // terrain; this does not rewrite authored glyph positions or
                // claim editor bootstrapping has already dispatched an actor.
                check_birth(&mut g, actor, point, wet, algorithm);
            }
        }
    });
}
