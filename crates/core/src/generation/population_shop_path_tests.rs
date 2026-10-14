//! Initial configured room-path controls. Missing/moved keepers are explicit
//! source-context discriminators, not claimed stock generation frequencies.
use super::*;

fn source_range(state: &mut u32, upper: u32) -> u32 {
    // Independent canonical integer RNG, not Rng::range/chance or membership.
    *state = state.wrapping_mul(0x0808_8405).wrapping_add(1);
    ((u64::from(*state) * u64::from(upper)) >> 32) as u32
}
fn flags() -> BTreeSet<String> {
    BTreeSet::from(["made_udjat_eye".into(), "made_market_entrance".into()])
}
fn marked_world(area: Area, marker: u8, keeper: &str) -> World {
    let mut world = World::new(42, 34, area, if area == Area::Temple { 13 } else { 1 });
    world.entrance = at(1, 1);
    world.spawn("oEntrance", Vec2::new(16.0, 16.0));
    // Two supports inside source room(1,0), the right support born first.
    // x=336 begins room(2,0); keep HardBlock at320 and Block at304.
    world.set_tile(20, 5, "oHardBlock");
    world.set_tile(19, 5, "oBlock");
    let mut path = [[0; 4]; 5];
    path[0][1] = marker;
    world.source_room_path = Some(path);
    if keeper != "none" {
        let id = world.spawn("oShopkeeper", Vec2::new(320.0, 80.0));
        let actor = world.entities.iter_mut().find(|e| e.id == id).unwrap();
        match keeper {
            "destroyed" => actor.alive = false,
            "inactive" => actor.engine_active = false,
            "moved" => actor.pos = Vec2::new(640.0, 592.0),
            "corpse" => {
                actor.health = 0;
                actor.data.insert("dead".into(), 1.0);
            }
            "live" => {}
            _ => panic!("unsupported initial keeper control"),
        }
    }
    world
}
fn without_grid(world: &World) -> serde_json::Value {
    let mut wire = serde_json::to_value(world).unwrap();
    wire.as_object_mut().unwrap().remove("source_room_path");
    wire
}

#[test]
fn population_shop_helpers_read_four_five_and_literal_outer_clamps_without_keepers() {
    for marker in [0, 1, 2, 3, 4, 5, 6, 7, 8, 9] {
        for keeper in ["none", "live", "corpse", "destroyed", "inactive", "moved"] {
            let world = marked_world(Area::Mines, marker, keeper);
            let expected = marker == 4 || marker == 5;
            // Source raw320,80 and the Block grid-center312,88 both belong
            // to room(1,0). The authoritative marker, not the actor, decides.
            assert_eq!(in_shop(&world, 20, 5), expected, "{marker}/{keeper}");
            assert_eq!(temple_in_shop(&world, Vec2::new(320.0, 80.0)), expected);
            assert!(!temple_in_shop(&world, Vec2::new(80.0, 80.0)));
        }
    }
    let mut world = marked_world(Area::Mines, 0, "none");
    let mut path = [[0; 4]; 5];
    path[4][3] = 5;
    path[0][0] = 4;
    world.source_room_path = Some(path);
    // scrGetRoomX/Y are ordered comparisons, clamped beyond both ends:
    // x<176 ->0; x>=496 ->3; y<144 ->0; y>=528 ->4.
    assert!(temple_in_shop(&world, Vec2::new(-100.0, -100.0)));
    assert!(temple_in_shop(&world, Vec2::new(10000.0, 10000.0)));
    assert!(in_shop(&world, -20, -20));
    assert!(in_shop(&world, 1000, 1000));
    assert!(!temple_in_shop(&world, Vec2::new(495.999, 528.0)));
    assert!(temple_in_shop(&world, Vec2::new(496.0, 528.0)));
    assert!(!temple_in_shop(&world, Vec2::new(496.0, 527.999)));
}

#[test]
fn population_none_context_retains_both_distinct_historical_keeper_adapters() {
    for keeper in ["none", "live", "corpse", "destroyed", "inactive", "moved"] {
        let mut world = marked_world(Area::Mines, 4, keeper);
        world.source_room_path = None;
        // Existing generic helper includes any stored exact keeper, even
        // retired/inactive. Temple/Arrow's existing helper excludes retired
        // keepers but not inactive ones. Neither fallback is silently repaired.
        let generic = matches!(keeper, "live" | "corpse" | "destroyed" | "inactive");
        let temple = matches!(keeper, "live" | "corpse" | "inactive");
        assert_eq!(in_shop(&world, 20, 5), generic);
        assert_eq!(temple_in_shop(&world, Vec2::new(320.0, 80.0)), temple);
    }
}

#[test]
fn marked_missing_keeper_skips_primary_rng_but_keeps_mandatory_temple_statue_rolls() {
    for area in [Area::Mines, Area::Temple] {
        for marker in [4, 5] {
            for keeper in ["none", "destroyed", "moved"] {
                for seed in [0_u32, 6, 42] {
                    let mut world = marked_world(area, marker, keeper);
                    let before_serial = world.creation_serial;
                    let mut actual = Rng::new(seed);
                    let mut literal = seed;
                    let mut statues = Vec::new();
                    if area == Area::Mines {
                        // Original scrEntityGen35 rand1..6 precedes withSolid.
                        source_range(&mut literal, 6);
                    } else {
                        // Original395 background roll precedes397 shop gate
                        // for each live Solid in birth order, even in shops.
                        for origin in [Vec2::new(320.0, 80.0), Vec2::new(304.0, 80.0)] {
                            if source_range(&mut literal, 100) == 0 {
                                statues.push(Vec2::new(origin.x, origin.y - 32.0));
                            }
                        }
                    }
                    populate(&mut world, &mut actual, &flags());
                    assert_eq!(
                        actual.state_bits(),
                        u64::from(literal),
                        "{area:?}/{marker}/{keeper}/{seed}"
                    );
                    assert_eq!(world.tile(19, 5).unwrap().object, "oBlock");
                    assert_eq!(world.tile(20, 5).unwrap().object, "oHardBlock");
                    let observed: Vec<_> = world
                        .entities
                        .iter()
                        .filter(|e| e.alive && e.sprite == "bgStatues")
                        .map(|e| e.pos)
                        .collect();
                    assert_eq!(observed, statues);
                    for born in world
                        .entities
                        .iter()
                        .filter(|e| e.creation_serial > before_serial)
                    {
                        assert!(
                            born.object == "oTerrainMetadata"
                                || born.object == "oGenerationDecoration",
                            "shop admitted {}",
                            born.object
                        );
                    }
                    if area == Area::Temple {
                        // Block's unconditional cleanDeath still runs before
                        // its Arrow shop gate, with no probability draw.
                        let metadata = world
                            .entities
                            .iter()
                            .find(|e| {
                                e.alive && e.object == "oTerrainMetadata" && e.pos == at(19, 5)
                            })
                            .unwrap();
                        assert_eq!(metadata.value("cleanDeath"), 1.0);
                    }
                    assert!(!world.fluid_dirty);
                }
            }
        }
    }
}

#[test]
fn intact_matching_keeper_phase_controls_keep_identical_outputs_and_rng() {
    for area in [Area::Mines, Area::Temple, Area::Ice] {
        for marker in [4, 5] {
            for legacy in [false, true] {
                for seed in [0_u32, 6, 42] {
                    // Explicit intact-template control. This is not a claim
                    // that every ordinary stock layout has been replayed.
                    let mut authoritative = marked_world(area, marker, "live");
                    let mut compatibility = authoritative.clone();
                    compatibility.source_room_path = None;
                    let mut a = if legacy {
                        Rng::legacy(seed.into())
                    } else {
                        Rng::new(seed)
                    };
                    let mut b = a.clone();
                    populate(&mut authoritative, &mut a, &flags());
                    populate(&mut compatibility, &mut b, &flags());
                    assert_eq!(a.state_bits(), b.state_bits());
                    assert_eq!(without_grid(&authoritative), without_grid(&compatibility));
                }
            }
        }
    }
}

#[test]
fn arrow_shop_gate_uses_persistent_marker_before_roll_and_preserves_clean_and_destroy_draws() {
    for area in [Area::Mines, Area::Temple] {
        for marker in [0, 4, 5] {
            for keeper in ["none", "live", "destroyed", "moved"] {
                let mut world = marked_world(area, marker, keeper);
                let before_serial = world.creation_serial;
                let mut actual = Rng::new(6);
                let mut literal = 6_u32;
                let shop = marker == 4 || marker == 5;
                if !shop {
                    assert_eq!(
                        source_range(&mut literal, if area == Area::Mines { 4 } else { 3 }),
                        0
                    );
                    if area == Area::Mines {
                        // Source Block own Destroy emits three fragments,
                        // each four coordinate draws; trap Create draws none.
                        for _ in 0..12 {
                            source_range(&mut literal, 9);
                        }
                    }
                }
                arrows(&mut world, &mut actual, &[(19, 5), (20, 5)]);
                assert_eq!(
                    actual.state_bits(),
                    u64::from(literal),
                    "{area:?}/{marker}/{keeper}"
                );
                assert_eq!(
                    world.tile(19, 5).unwrap().object,
                    if shop { "oBlock" } else { "" }
                );
                let traps: Vec<_> = world
                    .entities
                    .iter()
                    .filter(|e| e.alive && e.object == "oArrowTrapLeft")
                    .collect();
                assert_eq!(traps.len(), usize::from(!shop));
                if !shop {
                    assert_eq!(traps[0].pos, Vec2::new(304.0, 80.0));
                }
                let debris = world
                    .entities
                    .iter()
                    .filter(|e| {
                        e.alive
                            && e.creation_serial > before_serial
                            && matches!(e.object.as_str(), "oRubble" | "oRubbleSmall")
                    })
                    .count();
                assert_eq!(debris, if !shop && area == Area::Mines { 3 } else { 0 });
                if shop && area == Area::Temple {
                    let metadata = world
                        .entities
                        .iter()
                        .find(|e| e.alive && e.object == "oTerrainMetadata" && e.pos == at(19, 5))
                        .unwrap();
                    assert_eq!(metadata.value("cleanDeath"), 1.0);
                }
                assert!(!world.fluid_dirty);
            }
        }
    }
}
