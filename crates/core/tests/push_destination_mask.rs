//! Source-authored initial geometry; all subsequent movement is ordinary input.
//! Lava's own Step supplies the fractional block origin. No mid-run actor writes.
use spelunky_core::{
    Area, CollisionPrecision, Game, GameEvent, InputFrame, PlayerState, Rect, RulesPreset,
    RunState, Vec2, World, catalog,
    dispatcher::{self, Instance},
};

fn quiet(game: &Game) {
    assert_eq!(game.state, RunState::Playing);
    assert_eq!(game.player.health, 4);
    assert_eq!(game.player.invincible, 0);
    assert_eq!(game.player.stun, 0);
    assert!(!game.player.is_stunned());
    assert!(game.player.engine_active && game.player.grounded);
    assert!(
        !game.world.fluid_dirty,
        "no ordinary terrain change may request fluid redistribution"
    );
    assert_eq!(game.player.pos.y, 24.0);
    assert_eq!(game.player.vel.y, 0.0);
    assert_eq!(game.player.attack_timer, 0);
    assert!(game.player.equipment.is_empty() && game.player.held.is_none());
}

fn scene(rules: RulesPreset, lava: bool, obstruction_y: f64) -> (Game, u64, u64) {
    let mut game = Game::shell(42, rules);
    game.world = World::new(40, 24, Area::Mines, 1);
    for x in 0..4 {
        game.world.set_tile(x, 2, "oHardBlock");
    }
    if !lava {
        game.world.set_tile(4, 2, "oHardBlock");
    }
    // Initial authored entrance geometry only. Default HP/equipment/velocity
    // and Falling state are retained; the neutral Steps establish grounding.
    game.player.pos = Vec2::new(55.0, 24.0);
    game.player.previous_pos = game.player.pos;
    game.world.entrance = game.player.pos;
    let block = game.spawn("oPushBlock", Vec2::new(64.0, 16.0));
    let obstruction = game.spawn("oHardBlock", Vec2::new(80.0, obstruction_y));
    if lava {
        game.spawn("oLava", Vec2::new(64.0, 16.0));
    }
    assert_eq!(game.player.health, 4);
    assert_eq!(game.player.vel, Vec2::default());
    assert_eq!(game.player.state, PlayerState::Falling);
    assert!(!game.world.fluid_dirty);
    (game, block, obstruction)
}

fn mask(entity: &spelunky_core::Entity) -> catalog::CollisionMask {
    assert_eq!(entity.data.get("image_xscale").copied().unwrap_or(1.0), 1.0);
    assert_eq!(entity.data.get("image_yscale").copied().unwrap_or(1.0), 1.0);
    assert_eq!(entity.data.get("image_angle").copied().unwrap_or(0.0), 0.0);
    catalog::instance_mask(
        &entity.object,
        &entity.sprite,
        entity.pos,
        entity.animation,
        Vec2::new(1.0, 1.0),
        0.0,
    )
}

fn exercise(rules: RulesPreset, lava: bool, obstruction_y: f64, blocked: bool) {
    let (mut game, block_id, obstruction_id) = scene(rules, lava, obstruction_y);
    for _ in 0..4 {
        game.step(&InputFrame::default());
        quiet(&game);
    }
    let right = InputFrame {
        right: true,
        ..Default::default()
    };
    let mut reached = false;
    for _ in 0..8 {
        quiet(&game);
        let block = game.world.entity(block_id).unwrap();
        assert_eq!(
            block.pos.x, 64.0,
            "no earlier push before qualified contact"
        );
        if block.pos.x - (game.player.pos.x + 5.0) == 2.0 {
            reached = true;
            break;
        }
        game.step(&right);
    }
    assert!(
        reached,
        "SETUP: fixed ordinary Right route must reach the two-pixel gap"
    );
    quiet(&game);
    assert_eq!(game.player.state, PlayerState::Running);
    assert_eq!(game.player.facing, 1);
    assert!(game.player.vel.x > 1.5 && game.player.vel.x < 3.0);
    assert!(game.player.data.get("right_steps").copied().unwrap_or(0.0) >= 3.0);
    let block = game.world.entity(block_id).unwrap();
    assert!(block.engine_active && !block.held);
    assert_eq!(block.vel, Vec2::default());
    assert_eq!(block.sprite, "sBlock");
    if lava {
        assert_eq!(block.value("push_gravity"), 0.0);
        let lava = game
            .world
            .entities
            .iter()
            .find(|e| e.alive && e.object == "oLava")
            .unwrap();
        assert!(lava.engine_active);
        assert_eq!(lava.pos, Vec2::new(64.0, 16.0));
        assert!(lava.mask_contains(
            Vec2::new(block.pos.x + 8.0, block.pos.y + 14.0),
            CollisionPrecision::BoundingBox
        ));
        assert!(!lava.mask_contains(
            Vec2::new(game.player.pos.x, game.player.pos.y + 6.0),
            CollisionPrecision::BoundingBox
        ));
        assert!(
            block.pos.y > 17.0 && block.pos.y < 17.5,
            "SETUP: literal own-Step sinking must precede the native rounding boundary"
        );
    } else {
        assert_eq!(block.pos.y, 16.0);
        assert_eq!(block.value("push_gravity"), 0.6);
    }
    // Grounded characterStepEvent947–966 first lifts the controller by5
    // (the inclusive maxSlope4 loop), then moveTo settles it back onto floor.
    // On horizontal pixel3, the actual raised probe is y16..26, not21..31.
    assert_eq!(game.player.data.get("colTop").copied().unwrap_or(0.0), 0.0);
    assert_eq!(game.player.data.get("collision_top").copied(), Some(-8.0));
    assert_eq!(
        game.player.data.get("collision_half_width").copied(),
        Some(5.0)
    );
    for (rise, phase) in [(0.0, "incoming geometry"), (5.0, "actual raised movement")] {
        let first = Vec2::new(game.player.pos.x + 7.0, game.player.pos.y - 3.0 - rise);
        let last = Vec2::new(first.x, game.player.pos.y + 7.0 - rise);
        let hits = dispatcher::descendant_instances(&game, "oSolid")
            .into_iter()
            .filter(|instance| match *instance {
                Instance::Player => false,
                Instance::Terrain(index) => {
                    game.world.tiles[index].engine_active
                        && game
                            .world
                            .tile_collision_mask(
                                (index % game.world.width) as i32,
                                (index / game.world.width) as i32,
                            )
                            .intersects_axis_line(first, last, CollisionPrecision::Precise)
                }
                Instance::Entity(id) => game.world.entity(id).is_some_and(|e| {
                    e.engine_active
                        && mask(e).intersects_axis_line(first, last, CollisionPrecision::Precise)
                }),
            })
            .collect::<Vec<_>>();
        assert_eq!(hits, vec![Instance::Entity(block_id)], "{phase}");
    }
    let mut destination = block.clone();
    destination.pos.x += 1.0; // Oracle copy only; the live actor is never edited.
    let destination_mask = mask(&destination);
    let obstruction = game.world.entity(obstruction_id).unwrap();
    assert!(obstruction.engine_active);
    assert_eq!(
        destination.event_intersects(obstruction),
        blocked,
        "source place_meeting uses the translated receiver's precise mask"
    );
    assert!(
        !game
            .world
            .tiles
            .iter()
            .enumerate()
            .any(|(index, tile)| tile.engine_active
                && catalog::is_a(&tile.object, "oSolid")
                && destination_mask.intersects_mask(&game.world.tile_collision_mask(
                    (index % game.world.width) as i32,
                    (index / game.world.width) as i32
                )))
    );
    let old = block.sprite_bounds();
    let smooth = Rect::new(old.x + 1.0, old.y, old.w, old.h);
    assert_eq!(
        smooth.intersects(obstruction.sprite_bounds()),
        lava,
        "fractional source-clear case must actually discriminate the old smooth query"
    );
    assert_eq!(game.world.collides_except(smooth, block_id), lava);
    let incoming = game.player.pos;
    let block_x = block.pos.x;
    let expected_move = if blocked { 2.0 } else { 3.0 };
    eprintln!(
        "QUALIFIED {rules:?} lava={lava} blocker_y={obstruction_y} tick={} player={:?} block={:?} native_blocked={blocked}",
        game.tick, incoming, block.pos
    );
    game.step(&right);
    quiet(&game);
    assert_eq!(
        game.world.entity(block_id).unwrap().pos.x,
        block_x + if blocked { 0.0 } else { 1.0 },
        "SOURCE_PUSH_DESTINATION: native clear mask must permit exactly one push"
    );
    assert_eq!(game.player.pos.x, incoming.x + expected_move);
    let pushes = game
        .events
        .iter()
        .filter(|event| {
            matches!(event,
        GameEvent::SoundIfIdle { name, .. } if name == "sndPush")
        })
        .count();
    assert_eq!(pushes, usize::from(!blocked));
}

#[test]
fn ordinary_walk_pushes_lava_sinking_block_past_native_clear_lower_corner() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        exercise(rules, true, 33.0, false);
    }
}
#[test]
fn ordinary_walk_respects_native_blocked_lower_corner() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        exercise(rules, true, 32.0, true);
    }
}
#[test]
fn integer_supported_block_preserves_existing_clear_destination_push() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        exercise(rules, false, 33.0, false);
    }
}
