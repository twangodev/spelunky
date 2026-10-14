//! Initially configured IDLE/counter/velocity, then one real neutral Game.step.
//! Native collision_point flags -1,-1 select bbox with no self-exclusion.
//! These authored encounters do not establish a stock generated-room route.
use spelunky_core::*;

fn modes() -> impl Iterator<Item = (RulesPreset, RngAlgorithm)> {
    [RulesPreset::Classic, RulesPreset::Assisted]
        .into_iter()
        .flat_map(|rules| {
            [RngAlgorithm::Gm8Lcg32V1, RngAlgorithm::LegacySplitMix64V1]
                .into_iter()
                .map(move |algorithm| (rules, algorithm))
        })
}

fn scene(rules: RulesPreset, algorithm: RngAlgorithm, obstacle_x: f64) -> (Game, u64, u64) {
    let mut game = Game::shell_with_algorithm(42, rules, algorithm).unwrap();
    for x in 0..game.world.width {
        game.world.set_tile(x, 10, "oBlock");
    }
    game.player.pos = Vec2::new(256.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game.set_simulation_view_extent(
        game.world.width as f64 * 16.0,
        game.world.height as f64 * 16.0,
    )
    .unwrap();
    let obstacle = game.spawn("oDarkFall", Vec2::new(obstacle_x, 144.0));
    let hawk = game.spawn("oHawkman", Vec2::new(80.0, 144.0));
    let actor = game.world.entity_mut(hawk).unwrap();
    assert_eq!(actor.state, 0);
    assert_eq!(actor.health, 4);
    actor.vel = Vec2::default();
    actor.timer = 37;
    (game, hawk, obstacle)
}

fn assert_step(game: &mut Game, hawk: u64, obstacle: u64, jump: bool) {
    let obstacle_before = game.world.entity(obstacle).unwrap().clone();
    game.step(&InputFrame::default());
    let obstacle_after = game.world.entity(obstacle).unwrap();
    assert!(obstacle_after.alive && obstacle_after.engine_active);
    assert_eq!(
        obstacle_after.pos, obstacle_before.pos,
        "DarkFall did not move before the Hawk query"
    );
    assert_eq!(
        obstacle_after.value("timeFall"),
        20.0,
        "distant player does not start DarkFall"
    );
    let actor = game.world.entity(hawk).unwrap();
    assert!(actor.alive && actor.engine_active);
    assert_eq!(
        actor.pos,
        Vec2::new(80.0, 144.0),
        "supported stationary IDLE reaches the exact corner"
    );
    assert_eq!(actor.health, 4);
    assert_eq!(actor.state, 0);
    assert_eq!(
        actor.vel.y,
        if jump { -6.0 } else { 0.0 },
        "HAWK_MINUS_ONE_POINT_FLAGS_REQUIRE_BBOX_HOP"
    );
    assert_eq!(
        actor.timer,
        if jump { 26 } else { 36 },
        "source subtracts ten before the grounded decrement"
    );
    assert_eq!(actor.vel.x, if jump { 0.9 } else { 0.0 });
    assert_eq!(game.player.health, 4);
    assert!(
        !game
            .events
            .iter()
            .any(|event| matches!(event, GameEvent::Damage { .. }))
    );
}

#[test]
fn configured_hawk_idle_hops_at_transparent_bbox_corner_after_opaque_control() {
    for (rules, algorithm) in modes() {
        // Source sDarkFall PRECISE alpha(1,0) is opaque; alpha(0,0) is
        // transparent but both are inside its cached bbox. Source -1 flags
        // admit both. The positive compatibility case precedes the old failure.
        for (obstacle_x, opaque) in [(95.0, true), (96.0, false)] {
            let (mut game, hawk, obstacle) = scene(rules, algorithm, obstacle_x);
            let point = Vec2::new(96.0, 144.0); // Hawk x+16, y
            let obstacle_entity = game.world.entity(obstacle).unwrap();
            assert!(obstacle_entity.mask_contains(point, CollisionPrecision::BoundingBox));
            assert_eq!(
                obstacle_entity.mask_contains(point, CollisionPrecision::Precise),
                opaque
            );
            assert_step(&mut game, hawk, obstacle, true);
        }
    }
}

#[test]
fn configured_hawk_idle_outside_bbox_does_not_hop() {
    for (rules, algorithm) in modes() {
        let (mut game, hawk, obstacle) = scene(rules, algorithm, 97.0);
        let obstacle_entity = game.world.entity(obstacle).unwrap();
        assert!(
            !obstacle_entity.mask_contains(Vec2::new(96.0, 144.0), CollisionPrecision::BoundingBox)
        );
        assert_step(&mut game, hawk, obstacle, false);
    }
}
