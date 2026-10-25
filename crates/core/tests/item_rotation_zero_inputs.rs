//! Initial tiny velocities are configured, not a claim about ordinary birth.
//! The separate pickup/throw control proves ordinary ground friction clamps
//! to zero long before the epsilon band; it is not the negative witness.
use spelunky_core::{
    Area, CollisionPrecision, Game, GenerationRevision, InputFrame, RulesPreset, Vec2, World,
};

fn scene(rules: RulesPreset) -> Game {
    let mut game = Game::shell(42, rules);
    game.world = World::new(30, 20, Area::Mines, 1);
    game.world.name = "rLevel".into();
    for col in 0..30 {
        game.spawn("oHardBlock", Vec2::new(f64::from(col) * 16.0, 127.0));
    }
    game.player.pos = Vec2::new(200.0, 119.0);
    game.player.previous_pos = game.player.pos;
    game
}
fn healthy(game: &Game) {
    assert_eq!(game.player.health, 4.0);
    assert_eq!(game.player.invincible, 0);
    assert_eq!(game.player.stun, 0);
    assert!(!game.player.is_stunned());
}
fn configured_tiny(object: &str, rules: RulesPreset, bits: u64, historical: bool) {
    let mut game = scene(rules);
    if historical {
        game.generation_revision = GenerationRevision::HistoricalV1;
    } else {
        assert_eq!(
            game.generation_revision,
            GenerationRevision::SourceClassicV2
        );
    }
    let actor = game.spawn(object, Vec2::new(96.0, 64.0));
    assert_eq!(game.world.entity(actor).unwrap().vel, Vec2::default());
    assert_eq!(game.world.entity(actor).unwrap().value("gravity"), 0.2);
    let tiny = f64::from_bits(bits);
    game.world.entity_mut(actor).unwrap().vel.x = tiny; // Initial configured operand only.
    let e = game.world.entity(actor).unwrap();
    assert!(e.engine_active && !e.held);
    assert_eq!(e.value("stuck"), 0.0);
    assert!(!game.world.rectangle_family_except(
        Vec2::new(90.0, 58.0),
        Vec2::new(102.0, 70.0),
        "oSolid",
        None,
        CollisionPrecision::BoundingBox
    ));
    assert!(!game.player.sprite_bounds().intersects(e.sprite_bounds()));
    let next_id = game.world.next_id;
    let rng = game.rng.state_bits();
    // All authored setup ends here. One ordinary frame, no state writes.
    game.step(&InputFrame::default());
    healthy(&game);
    assert_eq!(game.player.held, None);
    let e = game.world.entity(actor).expect("live airborne receiver");
    assert_eq!(e.pos, Vec2::new(96.0, 64.0));
    assert_eq!(e.vel.x.to_bits(), bits);
    assert_eq!(
        e.vel.y, 0.2,
        "zero incoming movement then default first gravity"
    );
    assert_eq!(
        e.value("gravity"),
        0.6,
        "parent ends with ordinary gravity reset"
    );
    assert_eq!(e.value("stuck"), 0.0);
    for name in ["colLeft", "colRight", "colBot", "colTop"] {
        assert_eq!(e.value(name), 0.0);
    }
    if historical {
        assert!(
            e.value("rotation").is_finite() && e.value("rotation") != 0.0,
            "preserved historical raw-sign angle branch, not native atan parity"
        );
    } else {
        assert_eq!(
            e.value("rotation").to_bits(),
            (-0.0_f64).to_bits(),
            "SOURCE_EPSILON_ZERO_BRANCH {object} tiny={bits:016x}"
        );
        assert_eq!(
            e.data.get("image_angle").copied().map(f64::to_bits),
            Some(0.0_f64.to_bits()),
            "source Arrow/FishBone Step writes its literal zero angle"
        );
    }
    assert_eq!(game.rng.state_bits(), rng);
    assert_eq!(game.world.next_id, next_id);
}

#[test]
fn configured_airborne_arrow_and_fishbone_take_source_near_zero_branch() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for object in ["oArrow", "oFishBone"] {
            for bits in [0x3d3c25c268497681, 0xbd3c25c268497681] {
                configured_tiny(object, rules, bits, false);
            }
        }
    }
}
#[test]
fn historical_airborne_arrow_and_fishbone_keep_raw_sign_branch() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for object in ["oArrow", "oFishBone"] {
            configured_tiny(object, rules, 0xbd3c25c268497681, true);
        }
    }
}
#[test]
fn ordinary_pickup_throw_ground_decay_clamps_before_epsilon_band() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = scene(rules);
        let arrow = game.spawn("oArrow", Vec2::new(200.0, 123.0));
        for _ in 0..4 {
            game.step(&InputFrame::default());
            healthy(&game);
        }
        assert_eq!(game.player.held, None);
        assert_eq!(
            game.world.entity(arrow).unwrap().pos,
            Vec2::new(200.0, 123.0)
        );
        game.step(&InputFrame {
            down: true,
            attack: true,
            attack_pressed: true,
            ..InputFrame::default()
        });
        assert_eq!(game.player.held, Some(arrow));
        game.step(&InputFrame {
            left: true,
            attack_released: true,
            ..InputFrame::default()
        });
        assert_eq!(game.player.facing, -1);
        assert_eq!(game.player.held, Some(arrow));
        game.step(&InputFrame {
            attack: true,
            attack_pressed: true,
            ..InputFrame::default()
        });
        assert_eq!(game.player.held, None);
        assert!(game.world.entity(arrow).unwrap().vel.x < -1.0);
        game.step(&InputFrame {
            attack_released: true,
            ..InputFrame::default()
        });
        let mut clamped = false;
        for _ in 0..96 {
            healthy(&game);
            let e = game
                .world
                .entity(arrow)
                .expect("ordinary thrown Arrow remains live");
            let vx = e.vel.x;
            let qualify = e.value("colBot") == 1.0 && e.vel.y == 0.0 && vx != 0.0 && vx.abs() < 0.1;
            if qualify {
                assert!(
                    vx.abs() > f64::from_bits(0x3d3c25c268497682),
                    "ground decay is still outside epsilon"
                );
                let at = e.pos;
                assert!(game.world.rectangle_family_except(
                    Vec2::new(at.x - 4.0, at.y + 4.0),
                    Vec2::new(at.x + 3.0, at.y + 4.0),
                    "oSolid",
                    None,
                    CollisionPrecision::Precise
                ));
                game.step(&InputFrame::default());
                healthy(&game);
                let e = game.world.entity(arrow).unwrap();
                assert_eq!(e.value("colBot"), 1.0);
                assert_eq!(e.vel, Vec2::default());
                assert_eq!(e.value("rotation").to_bits(), (-0.0_f64).to_bits());
                clamped = true;
                break;
            }
            game.step(&InputFrame::default());
        }
        assert!(
            clamped,
            "FIXTURE: unchanged ordinary throw must reach a real ground clamp"
        );
    }
}
