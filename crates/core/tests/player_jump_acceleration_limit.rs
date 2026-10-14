//! UNRUN source-grounded ordinary-input proposal; no passing runtime claim.
//! The only writes configure the initial authored room; later changes are inputs.
use spelunky_core::{Area, Game, InputFrame, PlayerState, RulesPreset, Vec2, World};

fn healthy_step(game: &mut Game, input: InputFrame) {
    game.step(&input);
    assert_eq!(game.player.health, 4, "healthy ordinary ladder route");
    assert!(!game.player.is_stunned());
}

fn ladder_water_escape(rules: RulesPreset, jordans: bool) {
    let mut game = Game::shell(42, rules);
    game.world = World::new(20, 15, Area::Mines, 1);
    game.world.name = "rLevel".into();
    for x in 0..20 {
        game.world.set_tile(x, 10, "oHardBlock");
    }
    // Concrete descendants have the original masks. The empty family-root
    // objects oWater/oLadder are intentionally not used as physical actors.
    game.world.set_tile(5, 7, "oWaterSwim");
    for y in [112., 128., 144.] {
        game.spawn("oLadderOrange", Vec2::new(80., y));
    }
    if jordans {
        game.spawn("oJordans", Vec2::new(88., 152.));
    }
    game.player.pos = Vec2::new(88., 152.);
    game.player.previous_pos = game.player.pos;

    healthy_step(&mut game, InputFrame::default());
    healthy_step(
        &mut game,
        InputFrame {
            down: true,
            attack: true,
            attack_pressed: true,
            ..Default::default()
        },
    );
    healthy_step(&mut game, InputFrame::default());
    assert_eq!(game.player.has("Jordans"), jordans, "real source pickup");
    assert!(game.player.held.is_none());
    assert!(game.player.grounded);
    assert_eq!(game.player.state, PlayerState::Standing);
    assert_eq!(game.player.pos, Vec2::new(88., 152.));
    assert!(!game.world.tiles_point(Vec2::new(88., 143.), "oWater"));

    // This is a dry, ordinary ground jump: Jordans reaches the source -10
    // velocity limit and assigns yAccLimit=12, default shoes retain six.
    healthy_step(
        &mut game,
        InputFrame {
            jump: true,
            jump_pressed: true,
            ..Default::default()
        },
    );
    assert_eq!(game.player.vel.y, if jordans { -10. } else { -4. });
    assert_eq!(game.player.pos.y, if jordans { 142. } else { 148. });
    assert!(game.events.iter().any(|event| matches!(event,
        spelunky_core::GameEvent::Sound { name, .. } if name == "sndJump")));

    // A fresh Up attaches to the real ladder after the ground jump. No state,
    // coordinate, equipment or acceleration limit is edited after Step begins.
    healthy_step(
        &mut game,
        InputFrame {
            up: true,
            ..Default::default()
        },
    );
    assert_eq!(game.player.state, PlayerState::Climbing);
    for _ in 0..40 {
        if game.player.pos.y <= 134. {
            break;
        }
        healthy_step(
            &mut game,
            InputFrame {
                up: true,
                ..Default::default()
            },
        );
        assert_eq!(game.player.state, PlayerState::Climbing);
    }
    assert!(game.player.pos.y <= 134., "finite real upward climb");
    // Source climb friction .6 and approximatelyZero(<.1) stop the residual
    // velocity within six quiet frames; these frames also release Jump.
    for _ in 0..6 {
        healthy_step(&mut game, InputFrame::default());
    }
    assert_eq!(game.player.state, PlayerState::Climbing);
    assert_eq!(game.player.vel.y, 0.);
    assert!(game.player.pos.y > 127., "origin remains outside water");
    assert!(
        game.world
            .tiles_point(Vec2::new(88., game.player.pos.y - 9.), "oWater")
    );
    assert!(!game.world.tiles_point(game.player.pos, "oWater"));
    assert_eq!(game.player.data.get("gravity_intensity"), Some(&1.));

    // Source order: ladder depart -4, IN_AIR prior gravity +1, waterTop -4;
    // water escape also changes incoming yVel from zero to -3. The retained
    // twelve allows -7 acceleration; six clamps it to -6. Origin is dry, so
    // no water friction changes the resulting -10/-9 vertical velocities.
    healthy_step(
        &mut game,
        InputFrame {
            jump: true,
            jump_pressed: true,
            ..Default::default()
        },
    );
    assert_eq!(game.player.state, PlayerState::Jumping);
    assert_eq!(
        game.player.vel.y,
        if jordans { -10. } else { -9. },
        "SOURCE_INSTANCE_YACC_LIMIT_SURVIVES_CLIMBING rules={rules:?} jordans={jordans}"
    );
}

#[test]
fn natural_jordans_jump_retains_acceleration_limit_for_ladder_water_departure() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        ladder_water_escape(rules, true);
    }
}

#[test]
fn natural_default_jump_keeps_six_acceleration_limit_for_ladder_water_departure() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        ladder_water_escape(rules, false);
    }
}
