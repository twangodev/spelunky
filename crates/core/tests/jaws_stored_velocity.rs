//! Native default fish in an authored WaterSwim room with an initially
//! grounded Player, followed by one ordinary neutral Step.
//! No custom velocity/state/HP/timer/RNG injection. No arrow-trap extension.
use spelunky_core::*;

fn rng_wire(game: &Game) -> serde_json::Value {
    serde_json::to_value(&game.rng).unwrap()
}

fn room(rules: RulesPreset, canonical: bool) -> Game {
    let mut game = if canonical {
        Game::shell(42, rules)
    } else {
        Game::legacy_shell(42, rules)
    };
    game.world = World::new(24, 20, Area::Jungle, 1);
    game.world.name = "rLevel".into();
    // Native oWaterSwim supplies sWater; abstract oWater has no sprite/mask.
    // Walls and bottom contain the pond without a Solid replacement, so this
    // initial setup does not request the fluid-removal/drip RNG pass.
    for y in 4..13 {
        game.world.set_tile(3, y, "oHardBlock");
        game.world.set_tile(16, y, "oHardBlock");
    }
    for x in 4..16 {
        game.world.set_tile(x, 12, "oHardBlock");
        for y in 4..12 {
            game.world.set_tile(x, y, "oWaterSwim");
        }
    }
    for x in 0..24 {
        game.world.set_tile(x, 14, "oHardBlock");
    }
    // Authored initial Player pose; no claim of stock Player spawn coordinates.
    game.player.pos = Vec2::new(296.0, 216.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.player.facing = 1;
    game.world.entrance = game.player.pos;
    assert!(game.world.entities.is_empty());
    assert!(
        !game.world.fluid_dirty,
        "contained native pond prerequisite"
    );
    assert!(!water_point(&game, game.player.pos));
    game
}

fn water_point(game: &Game, point: Vec2) -> bool {
    game.world.tiles.iter().enumerate().any(|(index, tile)| {
        tile.engine_active
            && catalog::is_a(&tile.object, "oWater")
            && game
                .world
                .tile_collision_mask(
                    (index % game.world.width) as i32,
                    (index / game.world.width) as i32,
                )
                .contains(point, CollisionPrecision::BoundingBox)
    })
}

fn source_admitted(game: &Game, id: u64, jaws: bool) {
    let actor = game.world.entity(id).unwrap();
    assert!(actor.alive && actor.engine_active && !actor.held);
    assert_eq!(actor.value("inactive"), 0.0);
    if jaws {
        let view = game.simulation_view_rect();
        assert!(actor.pos.x > view.x - 48.0 && actor.pos.x < view.x + view.w + 48.0);
        assert!(actor.pos.y > view.y - 48.0 && actor.pos.y < view.y + view.h + 48.0);
    }
    assert_eq!(game.player.health, 4);
    assert_eq!(game.player.stun, 0);
    assert_eq!(game.player.state, PlayerState::Standing);
    assert_eq!(game.player.vel, Vec2::default());
    assert!(game.player.held.is_none());
    assert_eq!(
        game.player.data.get("swimming").copied().unwrap_or(0.0),
        0.0
    );
    assert!(!water_point(game, game.player.pos));
    assert_eq!(actor.value("burning"), 0.0);
    assert_eq!(actor.value("invincible"), 0.0);
    assert!(catalog::is_a(&actor.object, "oEnemy"));
}

fn single_bubble(game: &Game, actor: u64, velocity_y: f64, expected_pos: Vec2) {
    assert_eq!(
        game.world.entities.len(),
        2,
        "only native fish plus its Bubble"
    );
    let born: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.object == "oBubble")
        .collect();
    assert_eq!(born.len(), 1, "exact source first-Step Bubble birth");
    let bubble = born[0];
    assert!(bubble.alive && bubble.engine_active && bubble.id > actor);
    assert_eq!(
        bubble.vel,
        Vec2::new(0.0, velocity_y),
        "literal Bubble Create range3"
    );
    assert_eq!(
        bubble.pos, expected_pos,
        "source resource-order birth Step phase"
    );
    assert!(water_point(game, bubble.pos));
    assert!(!game.events.iter().any(|event| {
        matches!(event, GameEvent::Damage { .. })
            || matches!(event, GameEvent::Sound { name, .. }
                if name == "sndHit" || name == "sndArrowTrap" || name == "sndSplash")
    }));
}

#[test]
fn native_default_jaws_neutral_step_translates_without_writing_custom_velocity() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for canonical in [false, true] {
            let mut game = room(rules, canonical);
            let before_create_rng = rng_wire(&game);
            let id = game.spawn("oJaws", Vec2::new(128.0, 96.0));
            assert_eq!(
                rng_wire(&game),
                before_create_rng,
                "native Jaws Create has no RNG"
            );
            source_admitted(&game, id, true);
            let actor = game.world.entity(id).unwrap();
            assert_eq!((&*actor.object, &*actor.sprite), ("oJaws", "sJawsLeft"));
            assert_eq!(
                (actor.health, actor.state, actor.timer, actor.facing),
                (40, 0, 0, -1)
            );
            assert_eq!(actor.pos, Vec2::new(128.0, 96.0));
            assert_eq!(actor.vel, Vec2::default());
            assert_eq!(actor.image_speed(), 0.5);
            assert_eq!(actor.bounds(), Rect::new(128.0, 96.0, 48.0, 32.0));
            assert_eq!(actor.value("bubble_timer"), 0.0);
            assert_eq!(actor.value("canPickUp"), 0.0);
            assert!(water_point(&game, Vec2::new(136.0, 112.0)));
            assert!(water_point(&game, Vec2::new(126.0, 112.0)));
            assert!(!game.world.solid_point_except(Vec2::new(126.0, 112.0), id));
            // Source bottom(2) after the left2 movement is clear in this pond.
            assert!(!game.world.solid_line_except(
                Vec2::new(126.0, 129.0),
                Vec2::new(173.0, 129.0),
                id
            ));
            let mut expected_rng = game.rng.clone();
            let bubble_y = -f64::from(1 + expected_rng.range(3)) * 0.1;
            game.step(&InputFrame::default());
            source_admitted(&game, id, true);
            let actor = game.world.entity(id).unwrap();
            assert_eq!((actor.health, actor.state, actor.facing), (40, 0, -1));
            assert_eq!(
                actor.pos,
                Vec2::new(126.0, 97.0),
                "source moveTo(-2,0), then y+=1"
            );
            assert_eq!(actor.sprite, "sJawsLeft");
            assert_eq!(actor.bounds(), Rect::new(126.0, 97.0, 64.0, 32.0));
            assert_eq!(actor.value("bubble_timer"), 40.0);
            assert_eq!(
                rng_wire(&game),
                serde_json::to_value(&expected_rng).unwrap(),
                "only one synchronous Bubble Create draw; clone checks order/count"
            );
            // Bubble504 precedes Jaws885: newborn misses its own Step this tick.
            single_bubble(&game, id, bubble_y, Vec2::new(126.0, 113.0));
            assert_eq!(
                actor.vel,
                Vec2::default(),
                "JAWS_SOURCE_RETAINS_CUSTOM_ZERO_VELOCITY"
            );
        }
    }
}

#[test]
fn native_default_piranha_neutral_step_preserves_custom_velocity_agreement_control() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for canonical in [false, true] {
            let mut game = room(rules, canonical);
            let mut create_rng = game.rng.clone();
            let facing = if create_rng.range(2) == 0 { -1 } else { 1 };
            let id = game.spawn("oPiranha", Vec2::new(128.0, 96.0));
            assert_eq!(
                rng_wire(&game),
                serde_json::to_value(&create_rng).unwrap(),
                "one native Piranha direction Create draw"
            );
            source_admitted(&game, id, false);
            let actor = game.world.entity(id).unwrap();
            assert_eq!(actor.object, "oPiranha");
            assert_eq!(
                (actor.health, actor.state, actor.timer, actor.facing),
                (1, 0, 0, facing)
            );
            assert_eq!(actor.pos, Vec2::new(128.0, 96.0));
            assert_eq!(actor.vel, Vec2::default());
            assert_eq!(actor.image_speed(), 0.5);
            assert_eq!(actor.bounds(), Rect::new(128.0, 96.0, 8.0, 8.0));
            assert_eq!(actor.value("bubble_timer"), 0.0);
            assert_eq!(actor.value("can_bite"), 1.0);
            let probe = Vec2::new(128.0 + if facing < 0 { -2.0 } else { 10.0 }, 96.0);
            assert!(water_point(&game, probe));
            assert!(!game.world.solid_point_except(probe, id));
            let mut expected_rng = game.rng.clone();
            let bubble_y = -f64::from(1 + expected_rng.range(3)) * 0.1;
            let bubble_timer = f64::from(30 + expected_rng.range(21));
            game.step(&InputFrame::default());
            source_admitted(&game, id, false);
            let actor = game.world.entity(id).unwrap();
            assert_eq!((actor.health, actor.state, actor.facing), (1, 0, facing));
            let end_pos = Vec2::new(128.0 + f64::from(facing), 96.0);
            assert_eq!(actor.pos, end_pos, "source moveTo direction ±1,0");
            assert_eq!(actor.value("bubble_timer"), bubble_timer);
            assert_eq!(
                rng_wire(&game),
                serde_json::to_value(&expected_rng).unwrap(),
                "Bubble Create range3 before own timer range21"
            );
            // Piranha481 precedes Bubble504: newborn runs its own y+=yVel Step.
            single_bubble(
                &game,
                id,
                bubble_y,
                Vec2::new(end_pos.x, end_pos.y + bubble_y),
            );
            assert_eq!(
                actor.vel,
                Vec2::default(),
                "SOURCE_PIRANHA_CUSTOM_ZERO_AGREEMENT"
            );
        }
    }
}
