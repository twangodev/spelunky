//! Original oMagmaMan Step67 checks BOTH sides before its end-of-Step moveTo.
//! Prepared source-only fixture. All authored writes precede the first step.
use spelunky_core::*;

fn healthy(g: &Game) {
    assert_eq!(g.player.health, 4.0, "FIXTURE_HERO_HP4");
    assert_eq!(g.player.stun, 0, "FIXTURE_HERO_STUN0");
    assert!(!g.player.is_stunned(), "FIXTURE_HERO_NO_STUN_LATCH");
    assert!(
        !g.events
            .iter()
            .any(|event| matches!(event, GameEvent::Damage { .. })),
        "FIXTURE_NO_DAMAGE_EVENT"
    );
}

// Literal accepted canonical recurrence, independent from production range/chance.
// This is a source-order oracle for ONE fixed existing seed, never a seed search.
fn draw(state: &mut u32, width: u64) -> u32 {
    *state = state.wrapping_mul(0x0808_8405).wrapping_add(1);
    ((u64::from(*state) * width) >> 32) as u32
}

fn burn_prefix(state: &mut u32) -> bool {
    if draw(state, 20) == 0 {
        draw(state, 9); // x + rand(4,12)
        draw(state, 9); // y + rand(4,12)
        true
    } else {
        false
    }
}

fn source_line(g: &Game, e: &Entity, right: bool) -> bool {
    let b = e.bounds();
    let x = if right { b.x + b.w } else { b.x - 1.0 }.round_ties_even();
    g.world.solid_line_except(
        Vec2::new(x, b.y.round_ties_even()),
        Vec2::new(x, (b.y + b.h - 1.0).round_ties_even()),
        e.id,
    )
}

fn admit_wall_mask(g: &Game, wall: u64) {
    let wall = g.world.entity(wall).expect("FIXTURE_DEFAULT_WALL_ALIVE");
    assert_eq!(wall.object, "oHardBlock");
    assert_eq!(wall.sprite, "sBrick");
    assert!(catalog::is_a(&wall.object, "oSolid"));
    assert!(wall.engine_active && !wall.held);
    assert_eq!(wall.pos, Vec2::new(176.0, 144.0));
    assert_eq!(wall.vel, Vec2::default());
    assert_eq!(wall.sprite_bounds(), Rect::new(176.0, 144.0, 16.0, 16.0));
    // Its unrotated default RECTANGLE mask is opaque along the COMPLETE
    // source right line x176, y144..159, including the head above tb+5.
    for y in 144..160 {
        assert!(
            wall.mask_contains(Vec2::new(176.0, f64::from(y)), CollisionPrecision::Precise),
            "FIXTURE_FULL_HEIGHT_PRECISE_RIGHT_WALL"
        );
    }
    assert!(!wall.mask_contains(Vec2::new(163.0, 144.0), CollisionPrecision::Precise));
}

fn scene(rules: RulesPreset, revision: GenerationRevision) {
    // Seed0 is the existing, source-reviewed natural MagmaMan lifetime fixture.
    // Revision is an INITIAL authored snapshot configuration, never changed later.
    // HistoricalV1 with canonical RNG is supported for old canonical snapshots.
    let mut g = Game::shell(0, rules);
    g.generation_revision = revision;
    g.world = World::new(40, 24, Area::Mines, 1);
    g.world.name = "rLevel".into();
    g.flags.insert("custom".into());
    for x in 0..40 {
        g.world.set_tile(x, 10, "oHardBlock");
    }
    g.player.pos = Vec2::new(32.0, 152.0);
    g.player.previous_pos = g.player.pos;
    g.world.entrance = g.player.pos;
    let wall = g.spawn("oHardBlock", Vec2::new(176.0, 144.0));
    let id = g.spawn("oMagmaMan", Vec2::new(160.0, 144.0));
    let e = g.world.entity(id).unwrap();
    assert_eq!(e.health, 200, "FIXTURE_DEFAULT_BIRTH_HP");
    assert_eq!((e.state, e.timer, e.facing), (0, 0, 1));
    assert_eq!(e.vel, Vec2::new(2.5, 0.0), "FIXTURE_DEFAULT_BIRTH_VELOCITY");
    assert_eq!(e.bounds(), Rect::new(162.0, 144.0, 12.0, 16.0));
    assert!(!e.held && e.engine_active);
    assert_eq!(g.rng.algorithm(), RngAlgorithm::Gm8Lcg32V1);
    assert_eq!(
        g.rng.state_bits(),
        0,
        "FIXTURE_BIRTHS_CONSUME_NO_RANDOM_DRAWS"
    );
    admit_wall_mask(&g, wall);
    healthy(&g);

    // Own Create has no random draw. First IDLE burns (three draws), chooses
    // LEFT (one draw), and does not transform (one draw). WALK is an ELSE-IF,
    // so it cannot overwrite the default +2.5 before this callback's end moveTo.
    let mut expected_rng = 0;
    assert!(burn_prefix(&mut expected_rng));
    assert_eq!(
        draw(&mut expected_rng, 2),
        0,
        "FIXTURE_LITERAL_FIRST_FACING_LEFT"
    );
    assert_ne!(
        draw(&mut expected_rng, 6),
        0,
        "FIXTURE_LITERAL_NO_TRANSFORM"
    );
    assert_eq!(expected_rng, 1_172_187_917);
    g.step(&InputFrame::default());
    healthy(&g);
    assert_eq!(g.rng.state_bits(), u64::from(expected_rng));
    admit_wall_mask(&g, wall);
    let e = g
        .world
        .entity(id)
        .expect("FIXTURE_DEFAULT_BIRTH_SURVIVES_IDLE");
    assert_eq!(
        (e.state, e.timer, e.facing),
        (1, 0, -1),
        "FIXTURE_NATURAL_IDLE_TO_WALK"
    );
    assert_eq!(e.health, 199);
    assert_eq!(e.age, 1);
    assert_eq!(e.vel, Vec2::new(2.5, 0.0));
    // Engine time2 requests three pixels; the third is blocked by x176.
    assert_eq!(g.engine_time(), 2);
    assert_eq!(
        e.pos,
        Vec2::new(162.0, 144.0),
        "FIXTURE_INITIAL_MOVE_TO_AT_END"
    );
    assert_eq!(e.bounds(), Rect::new(164.0, 144.0, 12.0, 16.0));
    assert!(
        !source_line(&g, e, false),
        "FIXTURE_LEADING_LEFT_LINE_CLEAR"
    );
    assert!(source_line(&g, e, true), "FIXTURE_REAR_RIGHT_LINE_HIT");
    // Vertical side line is x163/x176, y144..159. Floor begins on row160,
    // so it cannot manufacture the side contact; both possible ledges support.
    assert!(g.world.tiles_point(Vec2::new(161.0, 160.0), "oSolid"));
    assert!(g.world.tiles_point(Vec2::new(178.0, 160.0), "oSolid"));
    assert!(!g.world.tiles_point(Vec2::new(163.0, 144.0), "oSolid"));
    let b = e.bounds();
    assert!(g.world.solid_line_except(
        Vec2::new(b.x.round_ties_even(), (b.y + b.h).round_ties_even()),
        Vec2::new(
            (b.x + b.w - 1.0).round_ties_even(),
            (b.y + b.h).round_ties_even()
        ),
        id,
    ));
    assert!(
        !g.world.rectangle_family_except(
            Vec2::new(b.x, b.y),
            Vec2::new(b.x + b.w - 1.0, b.y + b.h - 1.0),
            "oSolid",
            Some(id),
            CollisionPrecision::Precise,
        ),
        "FIXTURE_BODY_NOT_INSIDE_SOLID"
    );
    assert_eq!(
        g.world
            .entities
            .iter()
            .filter(|e| e.alive && e.object == "oBurn")
            .count(),
        1
    );
    let births_before = g.world.next_id;
    let hero_before = g.player.pos;

    // The next WALK callback must skip burn, skip the independent random-idle
    // branch, and consume exactly two draws. Both possible ledges are supported.
    assert!(!burn_prefix(&mut expected_rng));
    assert_ne!(
        draw(&mut expected_rng, 100),
        0,
        "FIXTURE_NO_RANDOM_IDLE_OVERRIDE"
    );
    assert_eq!(expected_rng, 1_368_768_587);
    g.step(&InputFrame::default());
    healthy(&g);
    assert_eq!(
        g.rng.state_bits(),
        u64::from(expected_rng),
        "MAGMA_WALK_SOURCE_RNG_ORDER"
    );
    assert_eq!(
        g.world.next_id, births_before,
        "FIXTURE_NO_NEW_BURN_OR_TRANSFORM"
    );
    assert_eq!(
        g.player.pos, hero_before,
        "FIXTURE_NEUTRAL_PLAYER_STAYS_SEPARATE"
    );
    admit_wall_mask(&g, wall);
    let e = g.world.entity(id).expect("FIXTURE_TARGET_SURVIVES");
    assert_eq!((e.health, e.state, e.timer, e.age), (198, 1, 0, 2));
    assert!(!e.held && e.engine_active);
    assert_eq!(e.vel.y, 0.0);
    assert_eq!(e.pos.y, 144.0);
    assert_eq!(g.engine_time(), 3);
    if revision == GenerationRevision::SourceClassicV2 {
        assert_eq!(e.facing, 1, "MAGMA_WALK_EITHER_SIDE_REAR_WALL_FACING");
        assert_eq!(e.vel.x, 1.5, "MAGMA_WALK_EITHER_SIDE_REAR_WALL_SPEED");
        assert_eq!(
            e.pos.x, 162.0,
            "MAGMA_WALK_TURNS_TOWARD_WALL_BEFORE_MOVE_TO"
        );
    } else {
        assert_eq!(e.facing, -1, "HISTORICAL_RETAINS_FACING_SIDE_ONLY");
        assert_eq!(e.vel.x, -1.5, "HISTORICAL_REAR_WALL_IGNORED");
        assert_eq!(e.pos.x, 161.0, "HISTORICAL_MOVES_AWAY_ONE_PIXEL_AT_TIME3");
    }
    assert_eq!(
        g.presentation().world.entity(id).unwrap().sprite,
        "sMagmaManWalkL"
    );
}

#[test]
fn source_magma_man_rear_wall_classic() {
    scene(RulesPreset::Classic, GenerationRevision::SourceClassicV2);
}

#[test]
fn source_magma_man_rear_wall_assisted() {
    scene(RulesPreset::Assisted, GenerationRevision::SourceClassicV2);
}

#[test]
fn historical_magma_man_rear_wall_classic() {
    scene(RulesPreset::Classic, GenerationRevision::HistoricalV1);
}

#[test]
fn historical_magma_man_rear_wall_assisted() {
    scene(RulesPreset::Assisted, GenerationRevision::HistoricalV1);
}
