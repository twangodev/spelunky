//! UNRUN ordinary-input authored ladder geometry, not a generated stock seed.
use spelunky_core::{
    Area, CollisionPrecision, Game, InputFrame, PlayerState, RulesPreset, Vec2, World,
};

fn healthy_step(game: &mut Game, input: InputFrame) {
    game.step(&input);
    assert_eq!(game.player.health, 4);
    assert!(!game.player.is_stunned());
}

fn source_up_gate(rules: RulesPreset, aligned: bool) {
    let mut game = Game::shell(42, rules);
    game.world = World::new(20, 15, Area::Mines, 1);
    game.world.name = "rLevel".into();
    for x in 0..20 {
        game.world.set_tile(x, 12, "oHardBlock");
    }
    let center = if aligned { 88. } else { 80. };
    let lower = game.spawn("oLadderOrange", Vec2::new(center - 8., 160.));
    let foot = game.spawn("oLadderOrange", Vec2::new(center - 8., 176.));
    game.world.set_tile(5, 9, "oLadderOrange");
    let upper = 9 * game.world.width + 5;
    game.player.pos = Vec2::new(center, 184.);
    game.player.previous_pos = game.player.pos;

    healthy_step(&mut game, InputFrame::default());
    assert_eq!(game.player.state, PlayerState::Standing);
    assert!(game.player.grounded);
    assert_eq!(game.player.pos, Vec2::new(center, 184.));
    let up = InputFrame {
        up: true,
        ..Default::default()
    };
    healthy_step(&mut game, up);
    assert_eq!(
        game.player.state,
        PlayerState::Climbing,
        "actual ordinary Up attachment"
    );
    assert_eq!(game.player.pos, Vec2::new(center, 184.));
    assert_eq!(game.player.vel, Vec2::default());

    let mut observed = false;
    for _ in 0..48 {
        let probe = Vec2::new(game.player.pos.x, game.player.pos.y - 8.);
        if (144.0..160.0).contains(&probe.y) {
            assert_eq!(game.player.state, PlayerState::Climbing);
            assert_eq!(game.player.pos.x, center);
            assert!(
                game.player.pos.y < 180.,
                "real upward motion preceded the gate"
            );
            assert!(
                game.player.vel.y < -0.5,
                "incoming ordinary climbing momentum"
            );
            assert_eq!(game.world.tile(5, 9).unwrap().object, "oLadderOrange");
            assert!(game.world.tiles[upper].engine_active);
            assert_eq!(
                game.world
                    .tiles_point_precision(probe, "oLadder", CollisionPrecision::BoundingBox),
                aligned,
                "actual upper source cached bbox point, independent of its occupied grid cell"
            );
            for id in [lower, foot] {
                let ladder = game
                    .world
                    .entity(id)
                    .expect("original lower ladder still alive");
                assert!(ladder.engine_active);
                assert!(!ladder.mask_contains(probe, CollisionPrecision::BoundingBox));
            }
            assert!(!game.world.tiles_point_precision(
                probe,
                "oLadderTop",
                CollisionPrecision::BoundingBox
            ));
            let incoming_y = game.player.vel.y;
            healthy_step(&mut game, up);
            let expected_y = (incoming_y - if aligned { 0.6 } else { 0.0 }) * 0.6;
            assert_eq!(game.player.state, PlayerState::Climbing);
            assert_eq!(
                game.player.vel.y, expected_y,
                "SOURCE_UP_CLIMB_REQUIRES_NATIVE_LADDER_POINT rules={rules:?} aligned={aligned}"
            );
            observed = true;
            break;
        }
        healthy_step(&mut game, up);
    }
    assert!(
        observed,
        "finite ordinary route reached the upper tile border"
    );
    for _ in 0..8 {
        healthy_step(&mut game, up);
        assert_eq!(game.player.state, PlayerState::Climbing);
        assert_eq!(game.player.pos.x, center);
    }
    if aligned {
        assert!(game.player.vel.y < -0.5);
        assert!(game.player.pos.y < 164.);
    } else {
        assert_eq!(game.player.vel.y, 0.);
        assert!(
            game.player.pos.y >= 160.,
            "momentum settles on the real lower ladder"
        );
    }
}

#[test]
fn natural_climb_up_stops_at_an_upper_ladders_transparent_tile_border() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        source_up_gate(rules, false);
    }
}

#[test]
fn natural_climb_up_continues_when_the_upper_source_mask_contains_its_probe() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        source_up_gate(rules, true);
    }
}
