//! Default-born Jar/Skull, ordinary pickup/flat throw, with an explicitly
//! authored subpixel initial Player x. No generated frequency claim.
use spelunky_core::{
    Area, CollisionPrecision, Game, GameEvent, InputFrame, RulesPreset, Vec2, World,
};

fn healthy(game: &Game, origin: Vec2) {
    assert_eq!(game.player.pos, origin);
    assert_eq!(game.player.vel, Vec2::default());
    assert_eq!(game.player.health, 4.0);
    assert_eq!(game.player.invincible, 0);
    assert_eq!(game.player.stun, 0);
    assert!(!game.player.is_stunned());
    assert!(
        !game.world.fluid_dirty,
        "the authored pool must not redistribute"
    );
}

fn pickup_and_flat_throw(object: &str, x: f64, rules: RulesPreset, expect_destroy: bool) {
    let mut game = Game::shell(42, rules);
    game.world = World::new(30, 20, Area::Mines, 1);
    game.world.name = "rLevel".into();
    // Floor is a normal default-born Solid row at literal integer127. This
    // qualifies supported Player y119 without floor/timer/state injection.
    for col in 0..30 {
        game.spawn("oHardBlock", Vec2::new(f64::from(col) * 16.0, 127.0));
    }
    game.world.set_tile(6, 6, "oLava");
    let origin = Vec2::new(x, 119.0);
    game.player.pos = origin;
    game.player.previous_pos = origin;
    let item = game.spawn(object, Vec2::new(x, 123.0));
    assert_eq!(game.world.entity(item).unwrap().vel, Vec2::default());
    assert!(!game.world.entity(item).unwrap().held);
    assert_eq!(game.world.entity(item).unwrap().value("breakPieces"), 1.0);
    // Everything from the first Step onward is ordinary input/read-only
    // observation; actor hp/position/velocity/held/physics are never edited.
    for _ in 0..4 {
        game.step(&InputFrame::default());
        healthy(&game, origin);
        assert!(game.world.entity(item).is_some());
    }
    assert!(game.player.grounded);
    assert_eq!(game.player.facing, 1);
    game.step(&InputFrame {
        down: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    healthy(&game, origin);
    assert_eq!(game.player.held, Some(item));
    assert!(game.world.entity(item).unwrap().held);
    game.step(&InputFrame {
        attack_released: true,
        ..Default::default()
    });
    healthy(&game, origin);
    assert_eq!(game.player.held, Some(item));
    assert_eq!(
        game.world.entity(item).unwrap().pos,
        Vec2::new(x + 4.0, 119.0)
    );
    // Source generic light throw is vx8/vy-3. Jar/Skull hand y isPlayer.y,
    // so first unobstructed own Step reaches(x+12,116), point y-5=111.
    let expected = Vec2::new(x + 12.0, 116.0);
    let point = Vec2::new(expected.x, 111.0);
    let lava = game.world.tile(6, 6).unwrap();
    assert_eq!(lava.object, "oLava");
    assert!(lava.engine_active);
    let mask = game.world.tile_collision_mask(6, 6);
    assert_eq!(
        mask.contains(point, CollisionPrecision::BoundingBox),
        expect_destroy,
        "literal Source collision_point raw bbox prerequisite"
    );
    assert!(
        !mask.intersects_rectangle(
            Vec2::new(expected.x - 3.0, 113.0),
            Vec2::new(expected.x + 3.0, 119.0),
            CollisionPrecision::BoundingBox
        ),
        "preceding Lava rectangle cannot stop motion or add sink offset"
    );
    assert!(!game.world.solid_at(expected.x, expected.y));
    assert!(
        !game.world.rectangle_family_except(
            Vec2::new(x, 110.0),
            Vec2::new(expected.x + 4.0, 121.0),
            "oSolid",
            None,
            CollisionPrecision::BoundingBox
        ),
        "full prethrow-to-endpoint tube is clear of wall/floor break causes"
    );
    let rng = game.rng.state_bits();
    let first_birth = game.world.next_id;
    game.step(&InputFrame {
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    healthy(&game, origin);
    assert_eq!(
        game.player.held, None,
        "actual source throw action completed"
    );
    assert_eq!(
        game.world.entity(item).is_none(),
        expect_destroy,
        "SOURCE_FRAGILE_LAVA_RAW_POINT {object} raw={point:?}"
    );
    if expect_destroy {
        assert!(game.events.iter().any(|event| matches!(event,
            GameEvent::Sound { name, .. } if name == "sndBreak")));
        assert!(
            game.world
                .entities
                .iter()
                .any(|e| e.id >= first_birth && e.object == "oSmokePuff")
        );
    } else {
        let body = game.world.entity(item).unwrap();
        assert_eq!(body.pos, expected);
        assert_eq!(body.vel.x, 8.0);
        assert!((body.vel.y - (-3.0 + 0.6)).abs() < 1e-12);
        assert_eq!(body.value("gravity"), 0.6);
        assert_eq!(body.value("colLeft"), 0.0);
        assert_eq!(body.value("colRight"), 0.0);
        assert_eq!(body.value("colTop"), 0.0);
        assert_eq!(body.value("colBot"), 0.0);
        assert_eq!(game.world.next_id, first_birth);
        assert_eq!(game.rng.state_bits(), rng);
        assert!(!game.events.iter().any(|event| matches!(event,
            GameEvent::Sound { name, .. } if name == "sndBreak")));
        assert!(!game.world.entities.iter().any(|e| e.id >= first_birth
            && matches!(e.object.as_str(), "oSmokePuff" | "oRubbleSmall" | "oBone")));
    }
    assert_eq!(game.player.bombs, 4.0);
    assert_eq!(game.player.ropes, 4.0);
}

#[test]
fn ordinary_default_jar_and_skull_toss_preserves_raw_point_just_left_of_lava() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for object in ["oJar", "oSkull"] {
            pickup_and_flat_throw(object, 83.9995, rules, false);
        }
    }
}

#[test]
fn ordinary_default_skull_toss_at_exact_lava_point_keeps_destroy_control() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        // Skull has only Bone debris, so this control never depends on Jar's
        // random enemy loot creating a second healthy-player interaction.
        pickup_and_flat_throw("oSkull", 84.0, rules, true);
    }
}
