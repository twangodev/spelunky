//! Source Jar own Step samples its origin AFTER moveTo, not before it.
//! Configured launch geometry at Olmec's real precise top-left corner; the
//! campaign route producing this launch remains unproved. No private helpers.
use spelunky_core::{
    Area, CollisionPrecision, Game, GenerationRevision, InputFrame, RulesPreset, Vec2, World,
};

fn prepared(rules: RulesPreset, relative_x: f64, vx: f64) -> (Game, u64, u64) {
    let mut game = Game::shell(123, rules);
    game.world = World::new(42, 56, Area::Boss, 16);
    for x in 1..41 {
        game.world.set_tile(x, 35, "oTemple");
    }
    game.player.pos = Vec2::new(24.0, 552.0);
    game.player.previous_pos = game.player.pos;
    let boss = game.spawn("oOlmec", Vec2::new(304.0, 496.0));
    let jar = game.spawn("oJar", Vec2::new(304.0 + relative_x, 496.0));
    // Frozen initial launched-item velocity, not a midrun state/phase write.
    game.world.entity_mut(jar).unwrap().vel = Vec2::new(vx, 0.0);
    let actor = game.world.entity(boss).unwrap();
    assert_eq!(actor.pos, Vec2::new(304.0, 496.0));
    assert_eq!(actor.sprite, "sOlmecStart1");
    assert!(actor.solid);
    let item = game.world.entity(jar).unwrap();
    assert_eq!(item.sprite, "sJar");
    assert!(!item.held);
    assert_eq!(item.value("gravity"), 0.6);
    assert_eq!(game.player.health, 4);
    assert!(game.player.engine_active);
    assert!(game.player.held.is_none());
    (game, boss, jar)
}

fn run_case(rules: RulesPreset, initial_x: f64, vx: f64, survives: bool) {
    run_case_revision(
        rules,
        initial_x,
        vx,
        survives,
        GenerationRevision::SourceClassicV2,
    );
}

fn run_case_revision(
    rules: RulesPreset,
    initial_x: f64,
    vx: f64,
    survives: bool,
    revision: GenerationRevision,
) {
    let (mut game, boss, jar) = prepared(rules, initial_x, vx);
    game.generation_revision = revision;
    let origin = Vec2::new(304.0 + initial_x, 496.0);
    let destination = Vec2::new(origin.x + vx, origin.y);
    let actor = game.world.entity(boss).unwrap();
    assert_eq!(
        actor.mask_contains(origin, CollisionPrecision::BoundingBox),
        initial_x >= 0.0
    );
    assert_eq!(
        actor.mask_contains(destination, CollisionPrecision::BoundingBox),
        destination.x >= 304.0 && destination.x <= 367.0
    );
    // Independent source-geometry prerequisite: every pixel of the Jar's
    // swept body, full post-move side probes, top/bottom probes and origin
    // lies in the real transparent corner. No precise collision may explain
    // a break. Original collision_point uses the wider bbox instead.
    let clear = |x: i32, y: i32| {
        assert!(
            !actor.mask_contains(
                Vec2::new(f64::from(x), f64::from(y)),
                CollisionPrecision::Precise
            ),
            "SOURCE_OLMEC_PRECISE_CORNER prerequisite {x},{y}"
        );
    };
    for center_x in origin.x.min(destination.x) as i32..=origin.x.max(destination.x) as i32 {
        for y in 490..=501 {
            for x in center_x - 4..=center_x + 3 {
                clear(x, y);
            }
        }
        if vx != 0.0 {
            let probe_x = center_x + if vx < 0.0 { -5 } else { 4 };
            for y in 495..=501 {
                clear(probe_x, y);
            }
        }
    }
    let final_x = destination.x as i32;
    for y in 490..=501 {
        clear(final_x - 5, y);
        clear(final_x + 4, y);
    }
    for x in final_x - 4..=final_x + 3 {
        for y in [489, 501, 502] {
            clear(x, y);
        }
    }
    let first_birth = game.world.next_id;
    let inventory = (game.player.bombs, game.player.ropes, game.player.money);
    let before_rng = game.rng.state_bits();
    // Everything after this boundary is one complete public neutral frame.
    game.step(&InputFrame::default());
    assert_eq!(
        game.world.entity(boss).unwrap().pos,
        Vec2::new(304.0, 496.0)
    );
    assert_eq!(game.world.entity(boss).unwrap().sprite, "sOlmecStart1");
    assert_eq!(game.player.health, 4);
    assert_eq!(
        (game.player.bombs, game.player.ropes, game.player.money),
        inventory
    );
    assert!(game.player.held.is_none());
    assert!(game.player.pickup_item_type.is_empty());
    if survives {
        let item = game
            .world
            .entity(jar)
            .expect("JAR_REVISION_ORIGIN_CHECK_SURVIVES");
        assert_eq!(item.pos, destination);
        assert_eq!(item.vel, Vec2::new(vx, 0.6));
        assert_eq!(game.rng.state_bits(), before_rng);
        assert!(
            game.world
                .entities
                .iter()
                .all(|entity| entity.id < first_birth)
        );
    } else {
        assert!(
            game.world.entity(jar).is_none(),
            "JAR_REVISION_ORIGIN_CHECK_BREAKS"
        );
        let effects = game
            .world
            .entities
            .iter()
            .filter(|entity| entity.id >= first_birth)
            .collect::<Vec<_>>();
        let smoke = effects
            .iter()
            .find(|entity| entity.object == "oSmokePuff")
            .unwrap();
        assert_eq!(
            smoke.previous_pos, destination,
            "Destroy must follow movement"
        );
        assert_eq!(
            effects
                .iter()
                .filter(|entity| entity.object == "oRubbleSmall")
                .count(),
            3
        );
        assert_ne!(game.rng.state_bits(), before_rng);
    }
    assert!(
        !game
            .world
            .entities
            .iter()
            .any(|entity| entity.id >= first_birth && entity.object == "oBlood")
    );
}

#[test]
fn jar_escapes_olmec_bbox_corner_before_source_origin_check() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        run_case(rules, 1.0, -2.0, true);
    }
}

#[test]
fn jar_enters_olmec_bbox_corner_before_source_origin_check() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        run_case(rules, -1.0, 2.0, false);
    }
}

#[test]
fn stationary_corner_controls_distinguish_bbox_from_precise_collision() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        run_case(rules, -1.0, 0.0, true);
        run_case(rules, 1.0, 0.0, false);
    }
}

#[test]
fn historical_jar_origin_check_retains_pre_move_latch() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        run_case_revision(rules, 1.0, -2.0, false, GenerationRevision::HistoricalV1);
        run_case_revision(rules, -1.0, 2.0, true, GenerationRevision::HistoricalV1);
        run_case_revision(rules, -1.0, 0.0, true, GenerationRevision::HistoricalV1);
        run_case_revision(rules, 1.0, 0.0, false, GenerationRevision::HistoricalV1);
    }
}
