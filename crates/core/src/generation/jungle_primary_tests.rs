//! Literal source-order controls; cloned public RNG checks statement ordering,
//! not independent canonical RNG mathematics or original gameplay execution.
use super::*;
use crate::{Game, GenerationRevision, RulesPreset};

fn rng_identity(rng: &Rng) -> (crate::RngAlgorithm, u64) {
    (rng.algorithm(), rng.state_bits())
}

fn fixture() -> World {
    let mut world = World::new(42, 34, Area::Jungle, 5);
    world.entrance = Vec2::new(8., 8.);
    world.spawn("oEntrance", Vec2::new(0., 0.));
    world
}
fn state(chance: i32) -> super::super::super::GenerationState {
    super::super::super::GenerationState {
        revision: GenerationRevision::SourceClassicV2,
        market_chance: chance,
        gold_chance: 160,
        made_market_entrance: false,
        probabilities: super::super::super::SourceProbabilities::default(),
    }
}
fn run(
    world: &mut World,
    rng: &mut Rng,
    campaign: &mut super::super::super::GenerationState,
    path: &[[u8; 4]; 5],
    market: bool,
) {
    let water = WaterMasks::new(world);
    let mut treasure = TreasureState {
        udjat: false,
        chest_chance: 8,
        damsel: false,
        cemetery: false,
    };
    populate(
        world,
        rng,
        &water,
        &mut treasure,
        campaign,
        path,
        market,
        true,
    );
}
fn births(world: &World, first: u64) -> Vec<(&str, Vec2)> {
    world
        .entities
        .iter()
        .filter(|e| e.alive && e.id >= first && e.object != "oGenerationDecoration")
        .map(|e| (e.object.as_str(), e.pos))
        .collect()
}
fn next_id(world: &World) -> u64 {
    world.entities.iter().map(|e| e.id).max().unwrap_or(0) + 1
}
fn source_spear_seed() -> u32 {
    // Select using only the literal source branch draws, never final actors.
    (0..4096)
        .find(|&seed| {
            let mut rng = Rng::new_canonical(seed);
            rng.range(100);
            rng.range(20);
            rng.range(12) == 0
        })
        .expect("bounded source branch selector")
}

#[test]
fn jungle_primary_source_market_eager_rejection_preserves_signed_draws() {
    for chance in [160, 1, 0, -1, -91088] {
        for seed in 0..16 {
            let mut world = fixture();
            world.entrance = Vec2::new(88., 136.);
            world.entities[0].pos = Vec2::new(80., 128.);
            let current = world.spawn("oSacAltarLeft", Vec2::new(80., 144.));
            let upper = world.spawn("oTemple", Vec2::new(80., 128.));
            world
                .entity_mut(current)
                .unwrap()
                .text
                .insert("type".into(), "Altar".into());
            world
                .entity_mut(upper)
                .unwrap()
                .text
                .insert("type".into(), "Tree".into());
            world
                .entity_mut(upper)
                .unwrap()
                .data
                .insert("invincible".into(), 1.);
            let mut path = [[0; 4]; 5];
            path[0][0] = 4;
            let mut expected = Rng::new_canonical(seed);
            // Current bg, all-eager signed market RHS, then shop upper bg.
            expected.range(100);
            expected.source_rand(1, chance).unwrap();
            expected.range(100);
            let mut rng = Rng::new_canonical(seed);
            let mut campaign = state(chance);
            run(&mut world, &mut rng, &mut campaign, &path, true);
            assert_eq!(
                rng_identity(&rng),
                rng_identity(&expected),
                "chance={chance}, seed={seed}"
            );
            assert_eq!(campaign.market_chance, chance - 1);
            assert!(!campaign.made_market_entrance);
            assert!(!world.entities.iter().any(|e| e.object == "oXMarket"));
        }
    }
}

#[test]
fn jungle_primary_source_zero_negative_success_and_market_native_create() {
    for chance in [1, 0, -1, -91088] {
        let mut world = fixture();
        world.entrance = Vec2::new(88., 136.);
        world.entities[0].pos = Vec2::new(80., 128.);
        let current = world.spawn("oLush", Vec2::new(80., 144.));
        world.spawn("oTemple", Vec2::new(80., 128.));
        let first = next_id(&world);
        let mut path = [[0; 4]; 5];
        path[0][0] = 4;
        let mut expected = Rng::new_canonical(17);
        expected.range(100);
        assert!(expected.source_rand(1, chance).unwrap() <= 1);
        expected.range(100);
        let mut rng = Rng::new_canonical(17);
        let mut campaign = state(chance);
        run(&mut world, &mut rng, &mut campaign, &path, true);
        assert_eq!(rng_identity(&rng), rng_identity(&expected));
        assert_eq!(campaign.market_chance, chance);
        assert!(campaign.made_market_entrance);
        assert_eq!(
            births(&world, first),
            vec![("oXMarket", Vec2::new(80., 128.))]
        );
        let market = world
            .entities
            .iter()
            .find(|e| e.object == "oXMarket")
            .unwrap();
        assert_eq!(
            market.text.get("type").map(String::as_str),
            Some("Market Exit")
        );
        assert_eq!(market.text.get("leadsTo").map(String::as_str), Some(""));
        assert_eq!(world.entity(current).unwrap().value("invincible"), 1.);
    }
}

#[test]
fn jungle_primary_source_forced_fresh_pass_visits_every_shop_receiver() {
    let mut world = fixture();
    let mut path = [[0; 4]; 5];
    path[0][0] = 4;
    path[1][0] = 5;
    for x in [80., 112.] {
        world.spawn("oLush", Vec2::new(x, 144.));
        world.spawn("oTemple", Vec2::new(x, 128.));
    }
    let first = next_id(&world);
    let mut expected = Rng::new_canonical(91);
    for _ in 0..4 {
        expected.range(100);
    }
    let mut rng = Rng::new_canonical(91);
    let mut campaign = state(160);
    run(&mut world, &mut rng, &mut campaign, &path, true);
    assert_eq!(rng_identity(&rng), rng_identity(&expected));
    assert_eq!(campaign.market_chance, 160);
    assert!(campaign.made_market_entrance);
    assert_eq!(
        births(&world, first),
        vec![
            ("oXMarket", Vec2::new(80., 128.)),
            ("oXMarket", Vec2::new(112., 128.))
        ]
    );
}

#[test]
fn jungle_primary_source_spear_eager_bright_wet_and_live_child_survival() {
    for (dark, wet) in [(false, false), (true, true)] {
        let mut world = fixture();
        world.dark = dark;
        let current = world.spawn("oLush", Vec2::new(80., 80.));
        world.spawn("oLush", Vec2::new(80., 96.));
        let content = crate::creation::spawn_generated_terrain_child(
            &mut world,
            &mut Rng::new_canonical(77),
            "oBombBag",
            Vec2::new(88., 88.),
            true,
        );
        if wet {
            world.spawn("oWater", Vec2::new(80., 48.));
        }
        let first = next_id(&world);
        let seed = source_spear_seed();
        let mut expected = Rng::new_canonical(seed);
        expected.range(100);
        expected.range(20);
        assert_eq!(expected.range(12), 0);
        expected.range(100);
        let mut rng = Rng::new_canonical(seed);
        let mut campaign = state(160);
        run(&mut world, &mut rng, &mut campaign, &[[0; 4]; 5], false);
        assert_eq!(rng_identity(&rng), rng_identity(&expected));
        assert!(world.entity(current).is_none());
        assert!(world.entity(content).is_some());
        let top = if dark {
            "oSpearTrapLit"
        } else {
            "oSpearTrapTop"
        };
        assert_eq!(
            births(&world, first),
            vec![
                ("oSpearTrapBottom", Vec2::new(80., 80.)),
                (top, Vec2::new(80., 64.))
            ]
        );
        for spear in world.entities.iter().filter(|e| {
            e.alive
                && matches!(
                    e.object.as_str(),
                    "oSpearTrapBottom" | "oSpearTrapTop" | "oSpearTrapLit"
                )
        }) {
            assert_eq!(spear.value("fired"), 0.);
            assert_eq!(spear.value("firedMax"), 50.);
            assert_eq!(spear.value("prox"), 4.);
            assert_eq!(spear.value("cleanDeath"), 0.);
        }
    }
}

#[test]
fn jungle_primary_source_banned_x_still_draws_torch_and_spear() {
    let mut world = fixture();
    let current = world.spawn("oLush", Vec2::new(160., 80.));
    world.spawn("oLush", Vec2::new(160., 96.));
    world.spawn("oSpikes", Vec2::new(160., 64.));
    let first = next_id(&world);
    let seed = source_spear_seed();
    let mut expected = Rng::new_canonical(seed);
    expected.range(100);
    expected.range(20);
    assert_eq!(expected.range(12), 0);
    expected.range(100);
    let mut rng = Rng::new_canonical(seed);
    let mut campaign = state(160);
    run(&mut world, &mut rng, &mut campaign, &[[0; 4]; 5], false);
    assert_eq!(rng_identity(&rng), rng_identity(&expected));
    assert!(world.entity(current).is_some());
    assert!(births(&world, first).is_empty());
}

#[test]
fn jungle_primary_source_grave_self_destroy_continues_frame_with_twelve_rolls() {
    let mut world = fixture();
    let current = world.spawn("oGrave", Vec2::new(80., 80.));
    world.entity_mut(current).unwrap().set_sprite("sGrave2");
    world.spawn("oLush", Vec2::new(80., 96.));
    let first = next_id(&world);
    let seed = source_spear_seed();
    let mut expected = Rng::new_canonical(seed);
    expected.range(100);
    expected.range(20);
    assert_eq!(expected.range(12), 0);
    let mut expected_births = vec![
        ("oSpearTrapBottom", Vec2::new(80., 80.)),
        ("oSpearTrapTop", Vec2::new(80., 64.)),
    ];
    for name in ["oRubble", "oRubbleSmall", "oRubbleSmall"] {
        let x = 88. + f64::from(expected.range(9)) - f64::from(expected.range(9));
        let y = 88. + f64::from(expected.range(9)) - f64::from(expected.range(9));
        expected_births.push((name, Vec2::new(x, y)));
    }
    expected.range(100);
    let mut rng = Rng::new_canonical(seed);
    let mut campaign = state(160);
    POPULATION_PHASE_TRACE.with(|trace| *trace.borrow_mut() = Some(Vec::new()));
    run(&mut world, &mut rng, &mut campaign, &[[0; 4]; 5], false);
    let continuation = POPULATION_PHASE_TRACE.with(|trace| trace.borrow_mut().take().unwrap());
    assert_eq!(
        continuation,
        vec![
            (5, 5, 5, PopulationPhase::Treasure),
            (5, 5, 6, PopulationPhase::Treasure)
        ],
        "the captured Grave frame enters Treasure after its own Destroy"
    );
    assert_eq!(rng_identity(&rng), rng_identity(&expected));
    assert!(world.entity(current).is_none());
    assert_eq!(births(&world, first), expected_births);
    assert!(
        !world.fluid_dirty,
        "Grave own Destroy does not inherit checkWater"
    );
}

#[test]
fn jungle_primary_source_clean_support_callbacks_keep_native_lamps_and_contents() {
    let mut world = fixture();
    let current = world.spawn("oLush", Vec2::new(80., 144.));
    let grave = world.spawn("oGrave", Vec2::new(80., 128.));
    world.entity_mut(grave).unwrap().set_sprite("sGrave2");
    let red = world.spawn("oLampRed", Vec2::new(80., 160.));
    let lamp = world.spawn("oLamp", Vec2::new(80., 160.));
    let content = crate::creation::spawn_generated_terrain_child(
        &mut world,
        &mut Rng::new_canonical(17),
        "oShotgun",
        Vec2::new(88., 152.),
        true,
    );
    let first = next_id(&world);
    let mut expected = Rng::new_canonical(31);
    let mut expected_births = Vec::new();
    for name in ["oRubble", "oRubbleSmall", "oRubbleSmall"] {
        let x = 88. + f64::from(expected.range(9)) - f64::from(expected.range(9));
        let y = 136. + f64::from(expected.range(9)) - f64::from(expected.range(9));
        expected_births.push((name, Vec2::new(x, y)));
    }
    expected_births.extend([
        ("oLampRedItem", Vec2::new(88., 172.)),
        ("oLampItem", Vec2::new(88., 172.)),
    ]);
    let receiver = receiver(
        &world,
        world.entity(current).unwrap().creation_serial,
        FamilyInstance::Entity(current),
        Vec2::new(80., 144.),
    )
    .unwrap();
    let mut rng = Rng::new_canonical(31);
    retire(&mut world, &mut rng, &receiver, true);
    assert_eq!(rng_identity(&rng), rng_identity(&expected));
    assert_eq!(births(&world, first), expected_births);
    assert!(world.entity(content).is_some());
    for id in [current, grave, red, lamp] {
        assert!(world.entity(id).is_none());
    }
    for item in world
        .entities
        .iter()
        .filter(|e| e.alive && matches!(e.object.as_str(), "oLampItem" | "oLampRedItem"))
    {
        assert_eq!(item.text.get("type").map(String::as_str), Some("Lamp"));
        assert_eq!(item.value("value"), 1000.);
        assert_eq!(item.value("heavy"), 1.);
        assert_eq!(item.value("trigger"), 1.);
    }
    assert!(world.fluid_dirty);
}

#[test]
fn jungle_primary_source_market_runtime_reentry_keeps_producer_overrides() {
    let mut game = Game::shell(7, RulesPreset::Classic);
    let id = game.spawn("oXMarket", Vec2::new(80., 64.));
    assert_eq!(
        game.world
            .entity(id)
            .unwrap()
            .text
            .get("type")
            .map(String::as_str),
        Some("Market Exit")
    );
    game.world
        .entity_mut(id)
        .unwrap()
        .text
        .insert("leadsTo".into(), "CUSTOM".into());
    game.world
        .entity_mut(id)
        .unwrap()
        .text
        .insert("type".into(), "Exit".into());
    let before = game.rng.clone();
    crate::world_events::create_instance(&mut game, id);
    assert_eq!(rng_identity(&game.rng), rng_identity(&before));
    assert_eq!(
        game.world
            .entity(id)
            .unwrap()
            .text
            .get("leadsTo")
            .map(String::as_str),
        Some("CUSTOM")
    );
    assert_eq!(
        game.world
            .entity(id)
            .unwrap()
            .text
            .get("type")
            .map(String::as_str),
        Some("Exit")
    );
}

#[test]
fn jungle_primary_source_actual_generated_room_has_eager_market_order_and_carry() {
    // Fixed canonical inputs; no selection on final actor placement or success.
    // Observe the reached call boundaries of actual next_level generation.
    for seed in [0, 1, 42, 8765] {
        let mut game = Game::shell(seed, RulesPreset::Classic);
        game.world.level = 4;
        game.flags.insert("genMarketEntrance".into());
        game.flags.insert("no_dark_level".into());
        game.data.insert("marketChance".into(), -37.);
        MARKET_TRACE.with(|trace| *trace.borrow_mut() = Some(Vec::new()));
        game.next_level();
        let traces = MARKET_TRACE.with(|trace| trace.borrow_mut().take().unwrap());
        assert_eq!(game.world.level, 5);
        assert!(
            !traces.is_empty(),
            "seed {seed}: literal reached upper-support condition"
        );
        let mut chance = -37;
        for trace in traces {
            assert_eq!(trace.chance_before, chance);
            let mut expected = trace.before_bg.clone();
            expected.range(100);
            assert_eq!(
                rng_identity(&expected),
                rng_identity(&trace.before_roll),
                "bg must precede signed RHS with no intervening draws"
            );
            let roll = expected.source_rand(1, chance).unwrap();
            let wins = trace.upper_kind != "Tree"
                && trace.current_kind != "Altar"
                && !trace.upper_invincible
                && roll <= 1;
            if wins {
                assert!(trace.made_after);
                assert_eq!(
                    trace.births,
                    vec![(
                        "oXMarket".to_owned(),
                        Vec2::new(trace.origin.x, trace.origin.y - 16.)
                    )]
                );
            } else {
                chance -= 1;
                assert!(trace.births.is_empty());
                assert!(!trace.made_after);
            }
            assert_eq!(trace.chance_after, chance);
            assert_eq!(
                rng_identity(&trace.after_roll),
                rng_identity(&expected),
                "native XMarket Create consumes no RNG"
            );
        }
        assert_eq!(game.data["marketChance"], f64::from(chance));
        assert!(game.flags.contains("made_market_entrance"));
        let before = game.data["marketChance"];
        MARKET_TRACE.with(|trace| *trace.borrow_mut() = Some(Vec::new()));
        game.next_level();
        let traces = MARKET_TRACE.with(|trace| trace.borrow_mut().take().unwrap());
        assert_eq!(game.world.level, 6);
        assert!(traces.is_empty());
        assert_eq!(game.data["marketChance"], before);
    }
}

#[test]
fn jungle_primary_source_translated_place_uses_first_mask_target_not_point_target() {
    let mut world = fixture();
    world.entrance = Vec2::new(88., 136.);
    world.entities[0].pos = Vec2::new(80., 128.);
    world.spawn("oLush", Vec2::new(80., 144.));
    let first_upper = world.spawn("oTemple", Vec2::new(88., 128.));
    world
        .entity_mut(first_upper)
        .unwrap()
        .text
        .insert("type".into(), "Tree".into());
    world
        .entity_mut(first_upper)
        .unwrap()
        .data
        .insert("invincible".into(), 1.);
    world.set_tile(5, 8, "oTemple");
    assert!(
        !world
            .entity(first_upper)
            .unwrap()
            .mask_contains(Vec2::new(80., 128.), crate::CollisionPrecision::BoundingBox)
    );
    assert!(
        world
            .tile_collision_mask(5, 8)
            .contains(Vec2::new(80., 128.), crate::CollisionPrecision::BoundingBox)
    );
    let mut path = [[0; 4]; 5];
    path[0][0] = 4;
    let mut expected = Rng::new_canonical(19);
    expected.range(100);
    expected.source_rand(1, 0).unwrap();
    expected.range(100);
    expected.range(100);
    let mut rng = Rng::new_canonical(19);
    let mut campaign = state(0);
    run(&mut world, &mut rng, &mut campaign, &path, true);
    assert_eq!(rng_identity(&rng), rng_identity(&expected));
    assert_eq!(campaign.market_chance, -1);
    assert!(!campaign.made_market_entrance);
    assert!(!world.entities.iter().any(|e| e.object == "oXMarket"));
}

#[test]
fn jungle_primary_source_inactive_solid_neither_receives_nor_blocks_queries() {
    let mut world = fixture();
    world.entrance = Vec2::new(88., 136.);
    world.entities[0].pos = Vec2::new(80., 128.);
    world.spawn("oLush", Vec2::new(80., 144.));
    let upper = world.spawn("oTemple", Vec2::new(80., 128.));
    world.entity_mut(upper).unwrap().engine_active = false;
    let mut path = [[0; 4]; 5];
    path[0][0] = 4;
    let mut expected = Rng::new_canonical(19);
    expected.range(100);
    let mut rng = Rng::new_canonical(19);
    let mut campaign = state(-1);
    run(&mut world, &mut rng, &mut campaign, &path, true);
    assert_eq!(rng_identity(&rng), rng_identity(&expected));
    assert_eq!(campaign.market_chance, -1);
    assert!(!campaign.made_market_entrance);
}

#[test]
fn jungle_primary_source_new_native_constructors_are_rng_free_and_revision_gated() {
    for spectacles in [false, true] {
        let mut game = Game::shell(31, RulesPreset::Classic);
        if spectacles {
            game.player.equipment.insert("Spectacles".into());
        }
        for object in [
            "oGoldNugget",
            "oLampItem",
            "oLampRedItem",
            "oSpearTrapBottom",
            "oSpearTrapTop",
            "oSpearTrapLit",
        ] {
            let before = game.rng.clone();
            let id = game.spawn(object, Vec2::new(80., 64.));
            assert_eq!(rng_identity(&game.rng), rng_identity(&before));
            let source = game.world.entity(id).unwrap();
            if object == "oGoldNugget" {
                assert_eq!(source.value("canCollect"), 1.);
                assert_eq!(source.value("value"), 500.);
            } else if object.contains("Lamp") {
                assert_eq!(source.value("depth"), if spectacles { 51. } else { 101. });
                assert_eq!(source.value("value"), 1000.);
            } else {
                assert_eq!(source.value("prox"), 4.);
                assert_eq!(source.value("firedMax"), 50.);
            }
            game.world
                .entity_mut(id)
                .unwrap()
                .data
                .insert("value".into(), 123.);
            game.world
                .entity_mut(id)
                .unwrap()
                .data
                .insert("fired".into(), 17.);
            crate::items::create_instance(&mut game, id);
            crate::traps::create(&mut game, crate::dispatcher::Instance::Entity(id));
            assert_eq!(rng_identity(&game.rng), rng_identity(&before));
            assert_eq!(game.world.entity(id).unwrap().value("value"), 123.);
            assert_eq!(game.world.entity(id).unwrap().value("fired"), 17.);
        }
    }
    let mut historical = Game::legacy_shell(u64::MAX, RulesPreset::Classic);
    for object in ["oGoldNugget", "oLampItem", "oLampRedItem"] {
        let before = historical.rng.clone();
        let id = historical.spawn(object, Vec2::new(80., 64.));
        assert_eq!(rng_identity(&historical.rng), rng_identity(&before));
        assert_eq!(
            historical
                .world
                .entity(id)
                .unwrap()
                .value("source_pickup_create_initialized"),
            0.
        );
    }
}

#[test]
fn jungle_primary_source_forced_pass_includes_new_spears_without_retesting_made() {
    let seed = (0..4096)
        .find(|&seed| {
            let mut source = Rng::new_canonical(seed);
            source.range(100);
            source.range(20);
            let spear = source.range(12) == 0;
            source.range(100);
            let failed_market = source.source_rand(1, 160).unwrap() > 1;
            spear && failed_market
        })
        .expect("literal primary branch guards");
    let mut world = fixture();
    world.spawn("oLush", Vec2::new(80., 80.));
    world.spawn("oLush", Vec2::new(80., 96.));
    let first = next_id(&world);
    let mut expected = Rng::new_canonical(seed);
    expected.range(100);
    expected.range(20);
    assert_eq!(expected.range(12), 0);
    expected.range(100);
    assert!(expected.source_rand(1, 160).unwrap() > 1);
    let mut rng = Rng::new_canonical(seed);
    let mut campaign = state(160);
    run(&mut world, &mut rng, &mut campaign, &[[0; 4]; 5], true);
    assert_eq!(rng_identity(&rng), rng_identity(&expected));
    assert_eq!(campaign.market_chance, 159);
    assert!(campaign.made_market_entrance);
    assert_eq!(
        births(&world, first),
        vec![
            ("oSpearTrapBottom", Vec2::new(80., 80.)),
            ("oSpearTrapTop", Vec2::new(80., 64.)),
            ("oXMarket", Vec2::new(80., 80.)),
            ("oXMarket", Vec2::new(80., 64.))
        ]
    );
}
