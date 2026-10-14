//! Authored initial native actors, followed by three neutral public Steps.
//! No actor state, velocity, sprite, mask, terrain or RNG writes after play.
//! The fractional PushBlock and off-grid ceiling are initial spatial context;
//! this does not claim an untouched generated room or original game execution.
use spelunky_core::*;

fn close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-10, "{actual} != {expected}");
}

fn sound_count(game: &Game, sound: &str) -> usize {
    game.events
        .iter()
        .filter(|event| matches!(event, GameEvent::Sound { name, .. } if name == sound))
        .count()
}

fn healthy(game: &Game, monkey: u64) {
    let e = game.world.entity(monkey).expect("default Monkey survives");
    assert!(e.alive && e.engine_active && !e.held);
    assert_eq!(e.health, 1);
    assert_eq!(e.value("swimming"), 0.0);
    assert_eq!(e.value("burning"), 0.0);
    assert_eq!(game.player.health, 4);
    assert_eq!(game.player.stun, 0);
    assert!(game.player.held.is_none());
    assert_eq!(game.player.pos, Vec2::new(136.0, 152.0));
    assert!(game.player.grounded);
    let view = game.simulation_view_rect();
    assert!(e.pos.x > view.x - 20.0 && e.pos.x < view.x + view.w + 4.0);
    assert!(e.pos.y > view.y - 20.0 && e.pos.y < view.y + view.h + 4.0);
    assert!(
        !game
            .world
            .entities
            .iter()
            .any(|e| { matches!(e.object.as_str(), "oBlood" | "oSplash" | "oBurn") })
    );
}

fn check(rules: RulesPreset, algorithm: RngAlgorithm, sinking: bool) {
    let mut game = Game::shell_with_algorithm(42, rules, algorithm).unwrap();
    game.world = World::new(24, 20, Area::Jungle, 1);
    game.world.name = "rLevel".into();
    // Separate static support for the Player leaves the Monkey/block pit open.
    for x in 8..game.world.width {
        game.world.set_tile(x, 10, "oHardBlock");
    }
    if !sinking {
        // Integer control: block bottom176 rests on this native tile.
        game.world.set_tile(6, 11, "oHardBlock");
    }
    game.player.pos = Vec2::new(136.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.world.entrance = game.player.pos;
    let roof = game.spawn("oHardBlock", Vec2::new(108.0, 134.0));
    let block = game.spawn(
        "oPushBlock",
        Vec2::new(96.0, if sinking { 159.45 } else { 159.0 }),
    );
    if sinking {
        game.spawn("oLava", Vec2::new(96.0, 160.0));
    }
    let monkey = game.spawn("oMonkey", Vec2::new(96.0, 144.0));
    let born = game.world.entity(monkey).unwrap();
    assert_eq!((born.health, born.state, born.timer), (1, 5, 0));
    assert_eq!(born.vel, Vec2::default());
    assert_eq!(born.sprite, "sMonkeyLeft");
    assert_eq!(born.bounds(), Rect::new(100.0, 150.0, 8.0, 10.0));
    let born_block = game.world.entity(block).unwrap();
    assert_eq!(born_block.vel, Vec2::default());
    assert_eq!(born_block.value("push_gravity"), 0.6);
    assert_eq!(born_block.sprite, "sBlock");
    // Source top line100..107 misses the authored roof until X-first movement.
    assert!(!game.world.entity(roof).unwrap().mask_intersects(
        Rect::new(100.0, 149.0, 8.0, 1.0),
        CollisionPrecision::Precise
    ));
    let born_rng = game.rng.state_bits();

    game.step(&InputFrame::default());
    healthy(&game, monkey);
    let climb = game.world.entity(monkey).unwrap();
    assert_eq!((climb.state, climb.timer), (6, 0));
    assert_eq!(climb.pos, Vec2::new(96.0, 144.0));
    assert_eq!(climb.vel, Vec2::default());
    assert_eq!(game.rng.state_bits(), born_rng, "SOURCE_HANG_HAS_NO_DRAW");
    close(
        game.world.entity(block).unwrap().pos.y,
        if sinking { 160.50 } else { 160.0 },
    );
    assert_eq!(sound_count(&game, "sndMonkey"), 0);

    // Own CLIMB: missing upper Vine first draws10..40, then the near lower
    // Player independently selects BOUNCE and draws its2..4 launch.
    let mut oracle = game.rng.clone();
    let hang_counter = 10 + oracle.range(31);
    let launch = -f64::from(2 + oracle.range(3));
    assert!(!game.world.tiles_point(Vec2::new(104.0, 144.0), "oVine"));
    assert!(
        !game
            .world
            .entities
            .iter()
            .any(|e| catalog::is_a(&e.object, "oVine"))
    );
    game.step(&InputFrame::default());
    healthy(&game, monkey);
    let bounce = game.world.entity(monkey).unwrap();
    assert_eq!((bounce.state, bounce.timer), (1, hang_counter));
    assert_eq!(bounce.pos, Vec2::new(96.0, 144.0));
    assert_eq!(bounce.vel, Vec2::new(3.0, launch));
    assert_eq!(bounce.value("vine_counter"), 30.0);
    assert_eq!(game.rng.state_bits(), oracle.state_bits());
    close(
        game.world.entity(block).unwrap().pos.y,
        if sinking { 160.55 } else { 160.0 },
    );
    assert_eq!(sound_count(&game, "sndMonkey"), 0);

    let before = bounce.clone();
    // Every negative entry displacement is stopped by the genuine roof mask
    // only after X-first movement96->99 brings top103..110 under it.
    assert!(
        game.world
            .solid_line_except(Vec2::new(103.0, 149.0), Vec2::new(110.0, 149.0), monkey)
    );
    assert!(!game.world.solid_line_except(
        Vec2::new(111.0, 155.0),
        Vec2::new(111.0, 159.0),
        monkey
    ));
    assert!(!game.world.solid_at(107.0, 152.0));
    if !sinking {
        oracle.range(2);
    }
    game.step(&InputFrame::default());
    healthy(&game, monkey);
    let after = game.world.entity(monkey).unwrap();
    assert_eq!(game.engine_time(), 4);
    assert_eq!(after.previous_pos, before.pos);
    assert_eq!(after.pos, Vec2::new(99.0, 144.0));
    assert_eq!(after.bounds(), Rect::new(103.0, 150.0, 8.0, 10.0));
    assert_eq!(after.timer, hang_counter);
    assert_eq!(after.value("vine_counter"), 29.0);
    close(
        game.world.entity(block).unwrap().pos.y,
        if sinking { 160.60 } else { 160.0 },
    );
    assert_eq!(
        game.world.entity(block).unwrap().value("push_gravity"),
        if sinking { 0.0 } else { 0.6 }
    );
    assert_eq!(
        game.world.entity(roof).unwrap().pos,
        Vec2::new(108.0, 134.0)
    );
    // Native isCollisionBottom(1) is the raw inclusive line below. Contrast
    // the older float rectangle using the exact same unmodified native mask.
    assert_eq!(
        game.world
            .solid_line_except(Vec2::new(103.0, 160.0), Vec2::new(110.0, 160.0), monkey),
        !sinking,
        "SOURCE_NATIVE_BOTTOM_CONTACT_PREREQUISITE"
    );
    assert!(
        game.world.entity(block).unwrap().mask_intersects(
            Rect::new(103.0, 160.0, 8.0, 1.0),
            CollisionPrecision::Precise
        ),
        "OLDER_FLOAT_BOTTOM_RECTANGLE_PREREQUISITE"
    );
    assert_eq!(after.vel.y, 1.0, "SOURCE_FINAL_CEILING_CLAMP");
    // All admission/motion/health/shape/launch premises precede the effect.
    assert_eq!(
        after.state,
        if sinking { 2 } else { 1 },
        "SOURCE_MONKEY_SINKING_SUPPORT_RECOVER"
    );
    assert_eq!(
        game.rng.state_bits(),
        oracle.state_bits(),
        "SOURCE_BOUNCE_SUPPORT_DRAW_CHRONOLOGY"
    );
    assert_eq!(sound_count(&game, "sndMonkey"), usize::from(sinking));
    assert_eq!(after.vel.x, if sinking { 3.0 } else { 2.0 });
    assert_eq!(
        after.sprite,
        if sinking {
            "sMonkeyJumpL"
        } else {
            "sMonkeyLeft"
        }
    );
}

#[test]
fn default_monkey_natural_sinking_support_selects_recover_without_jump_draw() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for algorithm in [RngAlgorithm::Gm8Lcg32V1, RngAlgorithm::LegacySplitMix64V1] {
            check(rules, algorithm, true);
        }
    }
}

#[test]
fn default_monkey_integer_block_support_keeps_bounce_and_its_jump_draw() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for algorithm in [RngAlgorithm::Gm8Lcg32V1, RngAlgorithm::LegacySplitMix64V1] {
            check(rules, algorithm, false);
        }
    }
}
