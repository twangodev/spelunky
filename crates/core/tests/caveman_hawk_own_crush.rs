//! Bounded own-Step continuation checks. Falling-block cases use fresh actors
//! and only neutral public Game::step after setup. Initial-contact DEAD,
//! STUNNED and held fixtures are explicitly configured branch controls.
//! These tests do not execute the original GameMaker runtime.
use spelunky_core::*;

fn room(canonical: bool, rules: RulesPreset) -> Game {
    let mut game = if canonical {
        Game::shell(42, rules)
    } else {
        Game::legacy_shell(42, rules)
    };
    game.world = World::new(30, 20, Area::Mines, 1);
    game.world.name = "rLevel".into();
    for x in 0..30 {
        game.world.set_tile(x, 8, "oBlock");
    }
    game.player.pos = Vec2::new(240.0, 120.0);
    game.player.previous_pos = game.player.pos;
    game
}
fn blood_draws(rng: &mut Rng, count: usize) {
    for _ in 0..count {
        rng.unit();
        rng.unit();
        rng.unit();
        rng.range(6);
    }
}
fn count(game: &Game, object: &str) -> usize {
    game.world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == object)
        .count()
}
fn sounds(game: &Game) -> usize {
    game.events
        .iter()
        .filter(|event| matches!(event, GameEvent::Sound { name, .. } if name == "sndCavemanDie"))
        .count()
}
fn falling_block_fresh_actor_crush_reaches_walk_tail_without_midpoint_writes(
    canonical: bool,
    rules: RulesPreset,
    object: &str,
) {
    let mut witnesses = 0;
    // A deterministic bounded fixture search accommodates facing changes.
    // Every candidate keeps fresh Create state and immutable room geometry.
    for block_x in [80.0, 96.0, 112.0] {
        for block_y in [64.0, 72.0, 80.0, 88.0, 93.0] {
            let mut game = room(canonical, rules);
            let actor = game.spawn(object, Vec2::new(96.0, 112.0));
            let block = game.spawn("oPushBlock", Vec2::new(block_x, block_y));
            let initial = game.world.entity(actor).unwrap();
            assert_eq!(initial.state, 0);
            assert_eq!(initial.vel, Vec2::new(2.5, 0.0));
            assert!(!game.world.solid_at(104.0, 120.0), "NO_INITIAL_CRUSH");
            assert!(block_y + 15.0 < initial.pos.y, "BLOCK_STARTS_ABOVE_ACTOR");
            for _ in 0..24 {
                let Some(incoming) = game.world.entity(actor).cloned() else {
                    break;
                };
                assert!(incoming.health > 0, "NO_DAMAGE_SETUP_OR_UNRELATED_KILL");
                let mut expected = game.rng.clone();
                game.step(&InputFrame::default());
                if game.world.entity(actor).is_none() {
                    assert!(
                        game.world.entity(block).unwrap().pos.y > block_y,
                        "NORMAL_BLOCK_GRAVITY_PRODUCER"
                    );
                    assert_eq!(count(&game, "oBlood"), 3, "EXACT_CRUSH_PREFIX");
                    assert_eq!(sounds(&game), 1, "HEALTHY_CRUSH_HAS_ONLY_CRUSH_SOUND");
                    assert_eq!(
                        game.data.get("kills").copied().unwrap_or(0.0),
                        0.0,
                        "CRUSH_DOES_NOT_ZERO_HP_OR_CREDIT_DEAD"
                    );
                    assert!(
                        matches!(incoming.state, 0 | 1),
                        "ROUTE_MUST_REMAIN_IDLE_OR_WALK"
                    );
                    // A previous natural WALK random stop can leave a positive
                    // IDLE countdown. That route still verifies the death prefix
                    // but cannot witness this WALK random draw contract.
                    if incoming.state != 1 {
                        break;
                    }
                    blood_draws(&mut expected, 3);
                    if expected.chance(1, 100) {
                        expected.range(31);
                    }
                    // Differential assertion comes after route qualification.
                    assert_eq!(
                        game.rng.clone().next_u64(),
                        expected.next_u64(),
                        "POST_DESTROY_STATE_RANDOMNESS"
                    );
                    witnesses += 1;
                    break;
                }
            }
        }
    }
    assert!(
        witnesses > 0,
        "FALLING_BLOCK_ROUTE_MUST_QUALIFY: {canonical} {rules:?} {object}"
    );
}

fn authored_healthy_idle_crush_still_casts_sight_after_blood_and_facing(
    canonical: bool,
    rules: RulesPreset,
    object: &str,
) {
    let mut game = room(canonical, rules);
    game.world.set_tile(6, 7, "oBlock");
    let actor = game.spawn(object, Vec2::new(88.0, 112.0));
    let mut expected = game.rng.clone();
    blood_draws(&mut expected, 3);
    expected.range(2);
    game.step(&InputFrame::default());
    assert!(game.world.entity(actor).is_none());
    assert_eq!(count(&game, "oBlood"), 3);
    assert_eq!(sounds(&game), 1);
    assert_eq!(game.data.get("kills"), None);
    let sight = game
        .world
        .entities
        .iter()
        .find(|e| e.alive && e.object == "oEnemySight")
        .expect("POST_DESTROY_SIGHT_CREATE");
    assert_eq!(
        sight.value("owner"),
        -4.0,
        "INSTANCE_PLACE_EXCLUDES_RETIRED_RECEIVER"
    );
    assert!(
        game.world
            .entities
            .iter()
            .filter(|e| e.object == "oBlood")
            .all(|blood| blood.creation_serial < sight.creation_serial)
    );
    assert_eq!(game.rng.clone().next_u64(), expected.next_u64());
}

fn configured_dead_crush_records_later_dead_credit_and_second_sound(
    canonical: bool,
    rules: RulesPreset,
    object: &str,
) {
    let mut game = room(canonical, rules);
    game.world.set_tile(6, 7, "oBlock");
    let actor = game.spawn(object, Vec2::new(96.0, 112.0));
    let e = game.world.entity_mut(actor).unwrap();
    e.health = 0;
    e.state = 99;
    e.vel = Vec2::default();
    let mut expected = game.rng.clone();
    blood_draws(&mut expected, 3);
    game.step(&InputFrame::default());
    assert!(game.world.entity(actor).is_none());
    assert_eq!(count(&game, "oBlood"), 3);
    assert_eq!(
        game.data.get("kills"),
        Some(&1.0),
        "POST_CRUSH_DEAD_BRANCH_CREDIT"
    );
    assert_eq!(sounds(&game), 2, "CRUSH_THEN_FIRST_DEAD_SOUND");
    assert_eq!(count(&game, "oEnemySight"), 0);
    assert_eq!(game.rng.clone().next_u64(), expected.next_u64());
    game.step(&InputFrame::default());
    assert_eq!(
        game.data.get("kills"),
        Some(&1.0),
        "RETIRED_ACTOR_NEVER_STEPS_AGAIN"
    );
    assert_eq!(sounds(&game), 0);
}

fn configured_stunned_held_crush_keeps_landing_blood_and_recovery_inventory_clear(
    canonical: bool,
    rules: RulesPreset,
    object: &str,
) {
    let mut game = room(canonical, rules);
    // Inherited Enemy keeps a held body at x236,y108. This is an explicitly
    // configured held branch; no claim of a natural pickup journey.
    // Non-grid resting solid covers the actor's x244 crush point while
    // leaving Player's x240 origin clear. Avoid unrelated Player crushing.
    game.spawn("oPushBlock", Vec2::new(244.0, 112.0));
    let actor = game.spawn(object, Vec2::new(236.0, 108.0));
    let e = game.world.entity_mut(actor).unwrap();
    e.state = 98;
    e.timer = 0;
    e.vel = Vec2::default();
    e.held = true;
    game.player.held = Some(actor);
    game.player.pickup_item_type = object.into();
    let mut expected = game.rng.clone();
    blood_draws(&mut expected, 4);
    game.step(&InputFrame::default());
    assert!(
        game.world.entity(actor).is_none(),
        "CONFIGURED_HELD_CRUSH_MUST_QUALIFY"
    );
    assert_eq!(
        count(&game, "oBlood"),
        4,
        "CRUSH_THEN_STUNNED_FIRST_LANDING_BLOOD"
    );
    assert_eq!(game.player.held, None);
    assert!(
        game.player.pickup_item_type.is_empty(),
        "POST_DESTROY_STUNNED_RECOVERY_CLEARS_TYPE"
    );
    assert_eq!(sounds(&game), 1);
    assert_eq!(game.data.get("kills"), None);
    assert_eq!(game.rng.clone().next_u64(), expected.next_u64());
}

fn noncrushed_fresh_actor_control_preserves_ordinary_sight_owner(
    canonical: bool,
    rules: RulesPreset,
    object: &str,
) {
    let mut game = room(canonical, rules);
    let actor = game.spawn(object, Vec2::new(96.0, 112.0));
    let mut expected = game.rng.clone();
    expected.range(2);
    game.step(&InputFrame::default());
    assert!(game.world.entity(actor).is_some());
    assert_eq!(count(&game, "oBlood"), 0);
    assert_eq!(sounds(&game), 0);
    assert_eq!(game.data.get("kills"), None);
    let sight = game
        .world
        .entities
        .iter()
        .find(|e| e.alive && e.object == "oEnemySight")
        .unwrap();
    assert_eq!(sight.value("owner"), actor as f64);
    assert_eq!(game.rng.clone().next_u64(), expected.next_u64());
}

// Explicit authored overlapping-family query control, not a natural room route.
fn configured_crushed_emitter_selects_other_live_family_member(
    canonical: bool,
    rules: RulesPreset,
    object: &str,
) {
    let mut game = room(canonical, rules);
    game.world.set_tile(6, 7, "oBlock");
    let retired = game.spawn(object, Vec2::new(88.0, 112.0));
    let survivor = game.spawn(object, Vec2::new(80.0, 112.0));
    assert!(
        game.world
            .entity(retired)
            .unwrap()
            .event_intersects(game.world.entity(survivor).unwrap())
    );
    assert!(!game.world.solid_at(88.0, 120.0), "OTHER_CENTER_CLEAR");
    game.step(&InputFrame::default());
    assert!(game.world.entity(retired).is_none());
    assert!(
        game.world.entity(survivor).is_some(),
        "QUERY_TARGET_SURVIVES"
    );
    let mut sights: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oEnemySight")
        .collect();
    sights.sort_by_key(|e| e.creation_serial);
    assert!(
        !sights.is_empty(),
        "RETIRED_SIGHT_SURVIVES_ON_CLEAR_LEFT_SIDE"
    );
    assert_eq!(
        sights[0].previous_pos,
        Vec2::new(88.0, 112.0),
        "SIGHT_BELONGS_TO_RETIRED_EMITTER_NOT_LATER_SURVIVOR"
    );
    assert_eq!(
        sights[0].value("owner"),
        survivor as f64,
        "RETIRED_RECEIVER_LOOKUP_FINDS_REMAINING_LIVE_FAMILY"
    );
}

macro_rules! cases {
    ($module:ident, $case:ident) => {
        mod $module {
            use super::*;
            #[test]
            fn legacy_classic_caveman() {
                $case(false, RulesPreset::Classic, "oCaveman");
            }
            #[test]
            fn legacy_assisted_caveman() {
                $case(false, RulesPreset::Assisted, "oCaveman");
            }
            #[test]
            fn canonical_classic_caveman() {
                $case(true, RulesPreset::Classic, "oCaveman");
            }
            #[test]
            fn canonical_assisted_caveman() {
                $case(true, RulesPreset::Assisted, "oCaveman");
            }
            #[test]
            fn legacy_classic_hawk() {
                $case(false, RulesPreset::Classic, "oHawkman");
            }
            #[test]
            fn legacy_assisted_hawk() {
                $case(false, RulesPreset::Assisted, "oHawkman");
            }
            #[test]
            fn canonical_classic_hawk() {
                $case(true, RulesPreset::Classic, "oHawkman");
            }
            #[test]
            fn canonical_assisted_hawk() {
                $case(true, RulesPreset::Assisted, "oHawkman");
            }
        }
    };
}
cases!(
    natural_falling_block,
    falling_block_fresh_actor_crush_reaches_walk_tail_without_midpoint_writes
);
cases!(
    authored_healthy_sight,
    authored_healthy_idle_crush_still_casts_sight_after_blood_and_facing
);
cases!(
    configured_dead_credit,
    configured_dead_crush_records_later_dead_credit_and_second_sound
);
cases!(
    configured_stunned_held,
    configured_stunned_held_crush_keeps_landing_blood_and_recovery_inventory_clear
);
cases!(
    positive_noncrush,
    noncrushed_fresh_actor_control_preserves_ordinary_sight_owner
);
cases!(
    configured_live_query_target,
    configured_crushed_emitter_selects_other_live_family_member
);
