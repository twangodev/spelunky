//! Ordinary pickup and throw across inherited Flare destruction.
use spelunky_core::{
    Area, CollisionPrecision, Game, GameEvent, InputFrame, PlayerState, RulesPreset, Vec2, World,
};

fn pickup_flare(rules: RulesPreset, facing: i8, fluid: Option<&str>) -> (Game, u64) {
    let mut game = Game::shell(42, rules);
    game.world = World::new(40, 30, Area::Mines, 1);
    game.world.name = "rLevel".into();
    for x in 0..game.world.width {
        game.world.set_tile(x, 10, "oHardBlock");
    }
    if let Some(object) = fluid {
        game.world.set_tile(7, 9, object);
    }
    game.player.pos = Vec2::new(if facing > 0 { 104.0 } else { 136.0 }, 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.player.facing = facing;
    game.world.entrance = game.player.pos;
    let flare = game.spawn("oFlare", game.player.pos);
    game.step(&InputFrame {
        down: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert_eq!(
        game.player.held,
        Some(flare),
        "ordinary input picks up Flare"
    );
    game.step(&InputFrame {
        attack_released: true,
        ..Default::default()
    });
    assert_eq!(game.player.state, PlayerState::Standing);
    assert_eq!(game.player.vel, Vec2::default());
    assert_eq!(game.player.health, 4.0);
    assert!(
        !game.world.fluid_dirty,
        "authored quiet pool has no drain request"
    );
    let held = game
        .world
        .entity(flare)
        .expect("held Flare stays alive outside fluid");
    assert!(held.held);
    assert_eq!(
        held.pos,
        Vec2::new(game.player.pos.x + 4.0 * f64::from(facing), 154.0)
    );
    assert!(
        !game
            .world
            .tiles_point_precision(held.pos, "oWater", CollisionPrecision::Precise)
    );
    assert!(game.world.entities.iter().all(|e| e.object != "oSplash"));
    (game, flare)
}

fn splash_events(game: &Game) -> usize {
    game.events
        .iter()
        .filter(|event| matches!(event, GameEvent::Sound { name, .. } if name == "sndSplash"))
        .count()
}

fn throw_flare(game: &mut Game) {
    game.step(&InputFrame {
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert_eq!(game.player.held, None);
    assert!(game.player.pickup_item_type.is_empty());
    assert_eq!(game.player.health, 4.0);
}

fn check_fluid(rules: RulesPreset, facing: i8, fluid: &str, expected_y: f64) {
    let (mut game, flare) = pickup_flare(rules, facing, Some(fluid));
    let expected = Vec2::new(if facing > 0 { 116.0 } else { 124.0 }, expected_y);
    assert!(
        game.world
            .tiles_point_precision(expected, "oWater", CollisionPrecision::Precise)
    );
    let death_probe = Vec2::new(expected.x, expected.y - 5.0);
    assert_eq!(
        game.world
            .tiles_point_precision(death_probe, "oLava", CollisionPrecision::Precise),
        fluid == "oLava",
        "only lava retires the receiver inside inherited Step"
    );
    throw_flare(&mut game);
    assert!(game.world.entity(flare).is_none(), "Flare remains retired");
    let splashes: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oSplash")
        .collect();
    assert_eq!(
        splashes.len(),
        1,
        "Flare own Step creates Splash after inherited Step"
    );
    let splash = splashes[0];
    assert!((splash.pos.x - expected.x).abs() < 1e-9);
    assert!((splash.pos.y - expected.y).abs() < 1e-9);
    assert_eq!(splash.age, 0);
    assert_eq!(splash.image_speed(), 0.6);
    assert_eq!(splash_events(&game), 1);
    assert!(game.events.iter().any(|event| {
        matches!(event, GameEvent::Sound { name, pos } if name == "sndSplash"
            && (pos.x - expected.x).abs() < 1e-9 && (pos.y - expected.y).abs() < 1e-9)
    }));
    for _ in 0..2 {
        game.step(&InputFrame::default());
        assert!(
            game.world.entity(flare).is_none(),
            "continuation never revives Flare"
        );
        assert_eq!(splash_events(&game), 0, "retired Flare cannot splash again");
    }
}

#[test]
fn naturally_thrown_flare_splashes_after_inherited_lava_retirement() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for facing in [-1, 1] {
            // Held154 -> throw151; inherited lava sinking adds0.05 before child query.
            check_fluid(rules, facing, "oLava", 151.05);
        }
    }
}

#[test]
fn naturally_thrown_flare_water_only_splash_is_preserved() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for facing in [-1, 1] {
            check_fluid(rules, facing, "oWaterSwim", 151.0);
        }
    }
}

#[test]
fn naturally_thrown_flare_dry_control_keeps_moving_without_splash() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for facing in [-1, 1] {
            let (mut game, flare) = pickup_flare(rules, facing, None);
            throw_flare(&mut game);
            let flying = game
                .world
                .entity(flare)
                .expect("dry throw keeps Flare alive");
            assert!(!flying.held);
            assert_eq!(
                flying.pos,
                Vec2::new(if facing > 0 { 116.0 } else { 124.0 }, 151.0)
            );
            assert_eq!(flying.vel.x, 8.0 * f64::from(facing));
            assert_eq!(flying.vel.y, -2.4);
            assert_eq!(splash_events(&game), 0);
            assert!(game.world.entities.iter().all(|e| e.object != "oSplash"));
        }
    }
}
