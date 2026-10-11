//! Regular Yeti normal-Step spike Blood. Natural fall uses source-default
//! Create and only ordinary empty input; explicit controlled gates are authored
//! before the first Step and never called through private tick/handler APIs.
use spelunky_core::*;
fn room(legacy: bool, seed: u32, rules: RulesPreset) -> Game {
    let mut g = if legacy {
        Game::legacy_shell(u64::from(seed), rules)
    } else {
        Game::shell(seed, rules)
    };
    g.world = World::new(30, 24, Area::Ice, 9);
    g.world.name = "rLevel".into();
    for x in 14..18 {
        g.world.set_tile(x, 10, "oBlock");
    }
    g.player.pos = Vec2::new(248.0, 152.0);
    g.player.previous_pos = g.player.pos;
    g
}
fn blood(g: &Game) -> Vec<&Entity> {
    g.world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oBlood")
        .collect()
}
fn natural_fall(legacy: bool) {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut g = room(legacy, if legacy { 4 } else { 1 }, rules);
        for x in 0..30 {
            g.world.set_tile(x, 9, "oSpikes");
            g.world.set_tile(x, 18, "oBlock");
        }
        let id = g.spawn("oYeti", Vec2::new(96.0, 64.0));
        let created = g.world.entity(id).unwrap();
        assert_eq!(
            (
                created.health,
                created.state,
                created.vel,
                created.value("dead")
            ),
            (5, 0, Vec2::new(2.5, 0.0), 0.0)
        );
        let mut natural_death = false;
        for _ in 0..26 {
            g.step(&InputFrame::default());
            let e = g
                .world
                .entity(id)
                .expect("DEFAULT_YETI_SURVIVES_SPIKE_DEATH_AS_BODY");
            if e.health == 0 {
                natural_death = true;
            }
        }
        let before = g.world.entity(id).unwrap().clone();
        assert!(
            natural_death,
            "ORDINARY_FALL_PRODUCED_DEATH_WITH_NO_BODY_EDITS"
        );
        assert_eq!(
            (before.state, before.health, before.value("dead")),
            (98, 0, 1.0)
        );
        assert_eq!(before.pos, Vec2::new(126.0, 144.0));
        assert_eq!(before.vel, Vec2::new(0.0, 2.6));
        assert!(!g.world.tiles_point(before.pos, "oSpikes"));
        assert_eq!(
            blood(&g).len(),
            3,
            "INHERITED_SPIKE_DEATH_ALREADY_EMITTED_THREE"
        );
        let serial = g.world.next_id;
        let mut expected = g.rng.clone();
        assert_eq!(
            expected.range(8),
            0,
            "FIRST_ELIGIBLE_NATURAL_SPIKE_ROLL_SUCCEEDS"
        );
        for _ in 0..4 {
            expected.next_u64();
        }
        g.step(&InputFrame::default());
        let actor = g.world.entity(id).unwrap();
        assert_eq!(actor.pos, Vec2::new(126.0, 147.0));
        assert_eq!((actor.state, actor.value("dead")), (98, 1.0));
        assert!(g.world.tiles_point(actor.pos, "oSpikes"));
        assert_eq!(g.player.health, 4);
        let newborn: Vec<_> = blood(&g).into_iter().filter(|e| e.id >= serial).collect();
        assert_eq!(newborn.len(), 1, "NATURAL_DEAD_BODY_SPIKE_DIRECT_BLOOD");
        assert_eq!(
            newborn[0].pos, actor.pos,
            "NORMAL_STEP_OTHER_IS_CURRENT_YETI"
        );
        assert_eq!(
            newborn[0].value("life"),
            60.0,
            "OWN_STEP_BIRTH_HAS_NOT_STEPPED"
        );
        assert_eq!(
            g.rng.state_bits(),
            expected.state_bits(),
            "ONE_ROLL_THEN_FOUR_SYNCHRONOUS_BLOOD_DRAWS"
        );
    }
}
#[derive(Clone, Copy, Debug)]
enum Gate {
    Eligible,
    DeadFalse,
    StoredZero,
    DeadBranch,
    PointOutsideBbox,
    InactiveEntity,
    InactiveTile,
    ReactivatedEntity,
    ReactivatedTile,
}
fn controlled_room(
    legacy: bool,
    seed: u32,
    rules: RulesPreset,
    gate: Gate,
    tile: bool,
    cap: usize,
    bloodless: bool,
    bounce: bool,
) -> (Game, u64, Option<u64>) {
    let mut g = room(legacy, seed, rules);
    for _ in 0..cap {
        g.spawn("oBone", Vec2::new(240.0, 64.0));
    }
    let spike = if tile {
        g.world.set_tile(6, 6, "oSpikes");
        if matches!(gate, Gate::InactiveTile | Gate::ReactivatedTile) {
            let index = 6 * g.world.width + 6;
            g.world.tiles[index].engine_active = false;
        }
        None
    } else {
        let id = g.spawn("oSpikes", Vec2::new(96.0, 96.0));
        if matches!(gate, Gate::InactiveEntity | Gate::ReactivatedEntity) {
            g.world.entity_mut(id).unwrap().engine_active = false;
        }
        Some(id)
    };
    if bounce {
        g.spawn("oHardBlock", Vec2::new(96.0, 116.0));
    }
    if matches!(gate, Gate::InactiveEntity | Gate::InactiveTile) {
        // Explicit isolated Normal-Step fixture: omit Level553 reactivation.
        g.world.name = "rIsolatedNoLevelController".into();
        assert!(!g.is_level());
    }
    let pos = Vec2::new(
        96.0,
        if matches!(gate, Gate::StoredZero) {
            100.0
        } else if matches!(gate, Gate::PointOutsideBbox) {
            97.0
        } else {
            96.0
        },
    );
    let id = g.spawn("oYeti", pos);
    let actor = g.world.entity_mut(id).unwrap();
    // Explicit source-reachable corpse/stun state, authored before first Step.
    actor.health = 0;
    actor.state = if matches!(gate, Gate::DeadBranch) {
        99
    } else {
        98
    };
    actor.timer = 200;
    actor.vel = Vec2::new(
        0.0,
        if matches!(gate, Gate::StoredZero) {
            -0.6
        } else if matches!(gate, Gate::PointOutsideBbox) {
            1.0
        } else {
            4.0
        },
    );
    actor
        .data
        .insert("dead".into(), f64::from(!matches!(gate, Gate::DeadFalse)));
    actor.data.insert("bounced".into(), f64::from(!bounce));
    actor.data.insert("bloodless".into(), f64::from(bloodless));
    (g, id, spike)
}
fn controlled(
    legacy: bool,
    gate: Gate,
    tile: bool,
    cap: usize,
    bloodless: bool,
    bounce: bool,
    success: bool,
) {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        // Select a run seed before the first Step, never overwrite live RNG.
        let seed = (0..1000)
            .find(|seed| {
                let (g, _, _) =
                    controlled_room(legacy, *seed, rules, gate, tile, cap, bloodless, bounce);
                let mut rng = g.rng.clone();
                (rng.range(8) == 0) == success
            })
            .unwrap();
        let (mut g, id, spike) =
            controlled_room(legacy, seed, rules, gate, tile, cap, bloodless, bounce);
        let start = g.world.entity(id).unwrap().clone();
        let direct = matches!(
            gate,
            Gate::Eligible | Gate::ReactivatedEntity | Gate::ReactivatedTile
        );
        let mut expected = g.rng.clone();
        if direct {
            assert_eq!(expected.range(8) == 0, success);
            if success {
                for _ in 0..4 {
                    expected.next_u64();
                }
            }
        }
        let births = usize::from(direct && success && cap < 32);
        let bounce_birth = usize::from(bounce && cap + births < 16 && !bloodless);
        for _ in 0..4 * bounce_birth {
            expected.next_u64();
        }
        let next_id = g.world.next_id;
        g.step(&InputFrame::default());
        let actor = g
            .world
            .entity(id)
            .expect("CONTROLLED_BODY_SURVIVES_OWN_STEP");
        assert_eq!(g.player.health, 4);
        assert_eq!(actor.health, 0);
        assert_eq!(actor.pos.x, start.pos.x);
        assert_eq!(
            actor.pos.y,
            if matches!(gate, Gate::StoredZero) {
                99.0
            } else if matches!(gate, Gate::PointOutsideBbox) {
                98.0
            } else {
                start.pos.y + 4.0
            }
        );
        let point = if let Some(spike) = spike {
            let spikes = g.world.entity(spike).unwrap();
            if matches!(gate, Gate::InactiveEntity) {
                assert!(!spikes.engine_active);
            }
            if matches!(gate, Gate::ReactivatedEntity) {
                assert!(
                    spikes.engine_active,
                    "LEVEL553_REACTIVATES_BEFORE_YETI_OWN_STEP"
                );
            }
            spikes.mask_contains(actor.pos, CollisionPrecision::BoundingBox)
        } else {
            if matches!(gate, Gate::InactiveTile) {
                assert!(!g.world.tile(6, 6).unwrap().engine_active);
            }
            if matches!(gate, Gate::ReactivatedTile) {
                assert!(
                    g.world.tile(6, 6).unwrap().engine_active,
                    "LEVEL553_REACTIVATES_TERRAIN_BEFORE_YETI"
                );
            }
            g.world.tiles_point(actor.pos, "oSpikes")
        };
        if matches!(gate, Gate::PointOutsideBbox) {
            assert!(!point, "POINT_IS_INSIDE_CANVAS_BUT_OUTSIDE_SPIKE_BBOX");
        } else if !matches!(gate, Gate::InactiveTile) {
            assert!(point);
        }
        if matches!(gate, Gate::DeadFalse) {
            assert_eq!(actor.value("dead"), 0.0);
        }
        if matches!(gate, Gate::StoredZero) {
            assert_eq!(actor.vel.y, 0.0, "TESTS_STORED_POST_GRAVITY_ZERO");
        }
        if matches!(gate, Gate::DeadBranch) {
            assert_eq!(
                actor.state, 98,
                "DEAD_TO_STUNNED_TRANSITION_STAYS_IN_DEAD_BRANCH_THIS_STEP"
            );
        }
        if bounce {
            assert_eq!(actor.value("bounced"), 1.0);
            assert_eq!(actor.vel.y, -2.3);
        }
        let actual = blood(&g);
        assert_eq!(
            actual.len(),
            births + bounce_birth,
            "DIRECT_BLOOD_BYPASSES_SCRIPT16_AND_BLOODLESS"
        );
        if births == 1 {
            assert_eq!(actual[0].pos, actor.pos, "NORMAL_STEP_OTHER_POSITION");
            assert_eq!(actual[0].value("life"), 60.0);
            assert_eq!(actual[0].alarms[0], 1);
            assert_eq!(actual[0].alarms[1], 1);
            assert_eq!(actual[0].alarms[2], 5);
        }
        if bounce_birth == 1 {
            assert_eq!(
                actual[births].pos,
                Vec2::new(actor.pos.x + 8.0, actor.pos.y + 8.0),
                "DIRECT_SPIKE_BIRTH_PRECEDES_GUARDED_BOUNCE_BLOOD"
            );
            if births == 1 {
                assert!(actual[0].id < actual[1].id);
            }
        }
        if direct && success && cap == 32 {
            assert_eq!(
                g.world.next_id,
                next_id + 1,
                "INHERITED_CAP_RETIRES_NEWEST_DIRECT_BLOOD_AFTER_ATTEMPT"
            );
        }
        assert_eq!(
            g.rng.state_bits(),
            expected.state_bits(),
            "ONLY_ELIGIBLE_GUARDS_ROLL_AND_DIRECT_CREATE_CONTINUES_THROUGH_CAP32"
        );
    }
}
#[test]
fn canonical_natural_fall() {
    natural_fall(false);
}
#[test]
fn legacy_natural_fall() {
    natural_fall(true);
}
macro_rules! pair {
    ($a:ident,$b:ident,$gate:expr,$tile:expr,$cap:expr,$bloodless:expr,$bounce:expr,$success:expr) => {
        #[test]
        fn $a() {
            controlled(false, $gate, $tile, $cap, $bloodless, $bounce, $success);
        }
        #[test]
        fn $b() {
            controlled(true, $gate, $tile, $cap, $bloodless, $bounce, $success);
        }
    };
}
pair!(
    canonical_entity_success,
    legacy_entity_success,
    Gate::Eligible,
    false,
    0,
    false,
    false,
    true
);
pair!(
    canonical_tile_success,
    legacy_tile_success,
    Gate::Eligible,
    true,
    0,
    false,
    false,
    true
);
pair!(
    canonical_direct_cap16_bloodless,
    legacy_direct_cap16_bloodless,
    Gate::Eligible,
    false,
    16,
    true,
    false,
    true
);
pair!(
    canonical_direct_cap32_continues_create,
    legacy_direct_cap32_continues_create,
    Gate::Eligible,
    false,
    32,
    true,
    false,
    true
);
pair!(
    canonical_failed_roll_draws_once,
    legacy_failed_roll_draws_once,
    Gate::Eligible,
    false,
    0,
    false,
    false,
    false
);
pair!(
    canonical_before_bounce_order,
    legacy_before_bounce_order,
    Gate::Eligible,
    false,
    0,
    false,
    true,
    true
);
pair!(
    canonical_dead_false_control,
    legacy_dead_false_control,
    Gate::DeadFalse,
    false,
    0,
    false,
    false,
    true
);
pair!(
    canonical_post_gravity_zero_control,
    legacy_post_gravity_zero_control,
    Gate::StoredZero,
    false,
    0,
    false,
    false,
    true
);
pair!(
    canonical_dead_branch_control,
    legacy_dead_branch_control,
    Gate::DeadBranch,
    false,
    0,
    false,
    false,
    true
);
pair!(
    canonical_bbox_edge_control,
    legacy_bbox_edge_control,
    Gate::PointOutsideBbox,
    false,
    0,
    false,
    false,
    true
);
pair!(
    canonical_inactive_entity_control,
    legacy_inactive_entity_control,
    Gate::InactiveEntity,
    false,
    0,
    false,
    false,
    true
);
pair!(
    canonical_inactive_tile_control,
    legacy_inactive_tile_control,
    Gate::InactiveTile,
    true,
    0,
    false,
    false,
    true
);

pair!(
    canonical_level553_reactivates_entity,
    legacy_level553_reactivates_entity,
    Gate::ReactivatedEntity,
    false,
    0,
    false,
    false,
    true
);
pair!(
    canonical_level553_reactivates_tile,
    legacy_level553_reactivates_tile,
    Gate::ReactivatedTile,
    true,
    0,
    false,
    false,
    true
);
