use spelunky_core::{Area, Game, InputFrame, RulesPreset, Vec2, World};

#[test]
fn diagonal_arrow_collides_with_webball_only_at_shared_opaque_pixels() {
    // The source Arrow Step derives this angle from its post-gravity velocity.
    // Canonical rotated-mask sampling (0x48A88A..0x48A987) gives no shared
    // opaque pixel at either upper corner, and 25 in the central control.
    for (web_pos, expected_state) in [
        (Vec2::new(94.0, 95.0), 0),
        (Vec2::new(105.0, 95.0), 0),
        (Vec2::new(100.0, 100.0), 1),
    ] {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.world = World::new(20, 15, Area::Mines, 1);
        game.player.pos = Vec2::new(24.0, 184.0);
        game.player.previous_pos = game.player.pos;
        for x in 0..20 {
            game.world.set_tile(x, 12, "oBlock");
        }
        let arrow_id = game.world.spawn("oArrow", Vec2::new(99.0, 100.0));
        game.world.entity_mut(arrow_id).unwrap().vel.x = 1.0;
        let web_id = game
            .world
            .spawn("oWebBall", Vec2::new(web_pos.x - 1.0, web_pos.y + 2.0));
        let web = game.world.entity_mut(web_id).unwrap();
        web.vel = Vec2::new(1.0, -2.0);
        web.data.insert("life".into(), 50.0);

        game.step(&InputFrame::default());

        let arrow = game.world.entity(arrow_id).unwrap();
        assert_eq!(arrow.pos, Vec2::new(100.0, 100.0));
        assert_eq!(arrow.vel, Vec2::new(1.0, 0.2));
        assert_eq!(arrow.value("rotation"), -0.2_f64.atan());
        assert_eq!(
            arrow.value("image_angle").to_bits(),
            0x4026_9eaf_781f_c537,
            "production Arrow Step must store Classic's FPATAN/radtodeg QWORD"
        );
        let web = game.world.entity(web_id).unwrap();
        assert_eq!(web.pos, web_pos);
        assert_eq!(web.state, expected_state, "WebBall at {web_pos:?}");
    }
}

#[test]
fn source_arrow_step_keeps_x87_rotated_bbox_half_tie_for_point_queries() {
    let mut game = Game::shell(1, RulesPreset::Classic);
    game.world = World::new(40, 20, Area::Mines, 1);
    game.player.pos = Vec2::new(20.0, 100.0);
    game.player.previous_pos = game.player.pos;

    // With the source arrow's 0.2 gravity, this step changes velocity from
    // (1, 0.8) to (1, 1), storing image_angle=45. The resulting X origin is
    // deliberately just beyond a source x87 bbox half tie: the port's f64
    // intermediate lands exactly on 1.5, while PC=11 rounds the source edge
    // to 1. This is the bbox path used by the character's oArrow ledge points.
    let arrow_x = 5.035_533_905_932_738;
    let arrow_id = game.world.spawn("oArrow", Vec2::new(arrow_x - 1.0, 99.0));
    let arrow = game.world.entity_mut(arrow_id).unwrap();
    arrow.vel = Vec2::new(1.0, 0.8);
    arrow.data.insert("stuck".into(), 1.0);

    game.step(&InputFrame::default());

    let arrow = game.world.entity(arrow_id).unwrap();
    assert_eq!(arrow.pos, Vec2::new(arrow_x, 100.0));
    assert_eq!(arrow.value("image_angle"), 45.0);
    assert!(arrow.engine_active);
    assert!(
        game.world
            .point_family_active(Vec2::new(1.5, 102.0), "oArrow")
    );
}
