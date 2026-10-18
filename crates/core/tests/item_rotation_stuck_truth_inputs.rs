//! Fractional `stuck` is configured/imported state: stock writers use only 0/1.
//! One ordinary held-Arrow Step isolates `not stuck` from parent wall resets.
use spelunky_core::{Area, Game, GenerationRevision, InputFrame, RulesPreset, Vec2, World};

fn held_case(rules: RulesPreset, revision: GenerationRevision, bits: u64) {
    let mut game = Game::shell(42, rules);
    game.generation_revision = revision;
    game.world = World::new(30, 20, Area::Mines, 1);
    game.world.name = "rLevel".into();
    for col in 0..30 {
        game.spawn("oHardBlock", Vec2::new(f64::from(col) * 16.0, 127.0));
    }
    game.player.pos = Vec2::new(200.0, 119.0);
    game.player.previous_pos = game.player.pos;
    let arrow = game.spawn("oArrow", Vec2::new(204.0, 121.0));
    assert_eq!(game.world.entity(arrow).unwrap().vel, Vec2::default());
    assert_eq!(game.world.entity(arrow).unwrap().value("stuck"), 0.0);
    let value = f64::from_bits(bits);
    assert!(value.is_finite());
    {
        let e = game.world.entity_mut(arrow).unwrap();
        e.held = true;
        e.data.insert("stuck".into(), value);
        e.data.insert("rotation".into(), 0.7);
    }
    game.player.held = Some(arrow);
    game.player.pickup_item_type = "Arrow".into();
    let rng = game.rng.state_bits();
    let next_id = game.world.next_id;
    // All authored setup ends here. No mutations after the first public input.
    game.step(&InputFrame::default());
    assert_eq!(game.player.health, 4.0);
    assert_eq!(game.player.invincible, 0);
    assert_eq!(game.player.stun, 0);
    assert!(!game.player.is_stunned());
    assert_eq!(game.player.held, Some(arrow));
    assert_eq!(game.player.pickup_item_type, "Arrow");
    assert_eq!(game.player.pos, Vec2::new(200.0, 119.0));
    let e = game.world.entity(arrow).expect("held Arrow remains live");
    assert!(e.engine_active && e.held);
    assert_eq!(e.pos, Vec2::new(204.0, 121.0));
    assert_eq!(
        e.vel,
        Vec2::default(),
        "inherited held branch zeros velocity"
    );
    assert_eq!(
        e.value("stuck").to_bits(),
        bits,
        "held branch retains operand"
    );
    let retains_previous = match revision {
        GenerationRevision::SourceClassicV2 => value >= 0.5,
        GenerationRevision::HistoricalV1 => value != 0.0,
    };
    let expected = if retains_previous { 0.7_f64 } else { -0.0_f64 };
    assert_eq!(
        e.value("rotation").to_bits(),
        expected.to_bits(),
        "SOURCE_NOT_STUCK revision={revision:?} stuck={bits:016x}"
    );
    assert_eq!(game.rng.state_bits(), rng);
    assert_eq!(game.world.next_id, next_id);
}

#[test]
fn configured_finite_stuck_uses_exact_half_threshold_in_source_arrow_step() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for bits in [
            0xbff0000000000000, // -1: false, unlike host nonzero
            0x0000000000000000,
            0x8000000000000000,
            0x3fd0000000000000, // 0.25: false
            0x3fdfffffffffffff, // immediate predecessor of 0.5: false
            0x3fe0000000000000, // exactly 0.5: true
            0x3fe0000000000001, // immediate successor: true
            0x3ff0000000000000, // 1: ordinary embedded-state control
        ] {
            held_case(rules, GenerationRevision::SourceClassicV2, bits);
        }
    }
}

#[test]
fn historical_finite_stuck_keeps_nonzero_adapter_in_arrow_step() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for bits in [
            0xbff0000000000000,
            0,
            0x8000000000000000,
            0x3fd0000000000000,
            0x3fdfffffffffffff,
            0x3fe0000000000000,
            0x3fe0000000000001,
            0x3ff0000000000000,
        ] {
            held_case(rules, GenerationRevision::HistoricalV1, bits);
        }
    }
}
