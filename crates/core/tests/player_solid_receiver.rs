//! moveTo selects one source Solid receiver before deciding whether to push.
//! All actors are authored before the first Step; the trajectory uses held keys.
use spelunky_core::{
    CollisionPrecision, Game, InputFrame, PlayerState, RulesPreset, RunState, Vec2, characters,
    dispatcher::{self, Instance},
    level::Level,
};

fn matrix(mut run: impl FnMut(RulesPreset, characters::Character, f64)) {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for character in [
            characters::Character::Explorer,
            characters::Character::Damsel,
            characters::Character::TunnelMan,
        ] {
            for direction in [-1.0, 1.0] {
                run(rules, character, direction);
            }
        }
    }
}

fn quiet(game: &Game) {
    assert_eq!(game.state, RunState::Playing);
    assert_eq!(game.player.health, 4.0);
    assert!(!game.player.is_stunned());
}

fn key(direction: f64) -> InputFrame {
    InputFrame {
        left: direction < 0.0,
        right: direction > 0.0,
        ..Default::default()
    }
}

fn seam(
    rules: RulesPreset,
    character: characters::Character,
    direction: f64,
    brick_first: bool,
) -> (Game, u64, u64) {
    let mut level = Level::default();
    let start_x = if direction > 0.0 { 4 } else { 8 };
    level.set_tile(start_x - 1, 3, '@').unwrap();
    level.set_tile(start_x - 1, 4, '1').unwrap();
    level.set_tile(5, 6, '1').unwrap(); // PushBlock support at (96,112).
    for x in 0..40 {
        level.set_tile(x, 9, '1').unwrap();
    }
    let mut game = Game::shell(42, rules);
    characters::apply(&mut game, character);
    game.load_custom(&level, "source-solid-receiver", "");
    // Explicit initial Create order supplies the source family ledger; no
    // spatial ordering assumption is used to choose the collision recipient.
    let (brick, block) = if brick_first {
        let brick = game.spawn("oBrick", Vec2::new(96.0, 80.0));
        let block = game.spawn("oPushBlock", Vec2::new(96.0, 96.0));
        (brick, block)
    } else {
        let block = game.spawn("oPushBlock", Vec2::new(96.0, 96.0));
        let brick = game.spawn("oBrick", Vec2::new(96.0, 80.0));
        (brick, block)
    };
    for _ in 0..4 {
        game.step(&InputFrame::default());
    }
    quiet(&game);
    assert_eq!(game.player.pos.y, 72.0);
    assert!(game.player.grounded);
    assert_eq!(game.world.entity(block).unwrap().pos, Vec2::new(96.0, 96.0));
    (game, brick, block)
}

fn source_hits(game: &Game, direction: f64) -> Vec<Instance> {
    // characterStepEvent600 shrinks the incoming falling controller mask to
    // top=-6. getIdCollisionRight/Left samples tb+5 through bb-1 inclusive.
    assert_eq!(game.player.state, PlayerState::Falling);
    assert!(game.player.vel.y > 0.0);
    let x = if direction > 0.0 {
        game.player.pos.x + 5.0
    } else {
        game.player.pos.x - 6.0
    };
    let first = Vec2::new(
        x.round_ties_even(),
        (game.player.pos.y - 1.0).round_ties_even(),
    );
    let second = Vec2::new(first.x, (game.player.pos.y + 7.0).round_ties_even());
    dispatcher::descendant_instances(game, "oSolid")
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
                        .intersects_axis_line(first, second, CollisionPrecision::Precise)
            }
            Instance::Entity(id) => game.world.entity(id).is_some_and(|entity| {
                entity.alive
                    && entity.engine_active
                    && spelunky_core::catalog::instance_mask(
                        &entity.object,
                        &entity.sprite,
                        entity.pos,
                        entity.animation,
                        Vec2::new(
                            entity.data.get("image_xscale").copied().unwrap_or(1.0),
                            entity.data.get("image_yscale").copied().unwrap_or(1.0),
                        ),
                        -entity
                            .data
                            .get("image_angle")
                            .copied()
                            .unwrap_or(0.0)
                            .to_radians(),
                    )
                    .intersects_axis_line(
                        first,
                        second,
                        CollisionPrecision::Precise,
                    )
            }),
        })
        .collect()
}

fn natural_seam(
    rules: RulesPreset,
    character: characters::Character,
    direction: f64,
    brick_first: bool,
) -> (Game, u64, u64, InputFrame) {
    let (mut game, brick, block) = seam(rules, character, direction, brick_first);
    let wall_x = if direction > 0.0 { 91.0 } else { 117.0 };
    let mut reached_wall = false;
    for _ in 0..80 {
        game.step(&key(direction));
        quiet(&game);
        assert_ne!(
            game.player.state,
            PlayerState::Hanging,
            "walk prefix grabbed before reversal"
        );
        if game.player.pos.x == wall_x {
            assert_eq!(game.player.state, PlayerState::Falling);
            assert!(game.player.vel.x * direction > 0.0);
            assert!(game.player.pos.y < 86.0);
            reached_wall = true;
            break;
        }
    }
    assert!(reached_wall, "ordinary walk did not reach the seam wall");
    // Newly pressing away changes facing before the source wall-velocity
    // guard; its first two held ticks do not accelerate against momentum.
    for tick in 0..2 {
        let mut input = key(-direction);
        if tick == 0 {
            input.left_released = direction < 0.0;
            input.right_released = direction > 0.0;
        }
        game.step(&input);
        quiet(&game);
        assert_eq!(game.player.state, PlayerState::Falling);
        assert!(game.player.vel.x * direction > 0.0);
    }
    for tick in 0..8 {
        let hits = source_hits(&game, direction);
        if hits.contains(&Instance::Entity(brick)) && hits.contains(&Instance::Entity(block)) {
            let receiver = if brick_first { brick } else { block };
            assert_eq!(hits.first(), Some(&Instance::Entity(receiver)));
            assert_ne!(
                spelunky_core::catalog::object("oBrick").unwrap().parent,
                "oMoveableSolid"
            );
            assert_eq!(
                spelunky_core::catalog::object("oPushBlock").unwrap().parent,
                "oMoveableSolid"
            );
            assert!(game.player.vel.x * direction > 0.1);
            println!(
                "SOURCE_RECEIVER_WITNESS rules={rules:?} character={character:?} direction={direction} brick_first={brick_first} pos={:?} vel={:?} hits={hits:?}",
                game.player.pos, game.player.vel
            );
            let mut input = InputFrame::default();
            if tick == 0 {
                input.left_released = direction > 0.0;
                input.right_released = direction < 0.0;
            }
            return (game, brick, block, input);
        }
        let mut input = InputFrame::default();
        if tick == 0 {
            input.left_released = direction > 0.0;
            input.right_released = direction < 0.0;
        }
        game.step(&input);
        quiet(&game);
    }
    panic!("natural trajectory never reached both precise masks");
}

#[test]
fn natural_falling_momentum_uses_the_first_solid_receiver_before_pushing() {
    matrix(|rules, character, direction| {
        let (mut game, _, block, input) = natural_seam(rules, character, direction, true);
        let before_player = game.player.pos;
        let before_block = game.world.entity(block).unwrap().pos;
        game.step(&input);
        quiet(&game);
        assert_eq!(
            game.world.entity(block).unwrap().pos,
            before_block,
            "first Solid is Brick: no other intersecting block may be pushed: {rules:?} {character:?} direction{direction}"
        );
        assert_eq!(game.player.pos.x, before_player.x);
    });
}

#[test]
fn natural_falling_momentum_can_push_when_the_first_receiver_is_moveable() {
    matrix(|rules, character, direction| {
        let (mut game, brick, block, input) = natural_seam(rules, character, direction, false);
        let before_player = game.player.pos;
        let before_block = game.world.entity(block).unwrap().pos;
        let before_brick = game.world.entity(brick).unwrap().pos;
        game.step(&input);
        quiet(&game);
        assert_eq!(game.world.entity(brick).unwrap().pos, before_brick);
        assert!(direction * (game.world.entity(block).unwrap().pos.x - before_block.x) >= 1.0);
        assert!(direction * (game.player.pos.x - before_player.x) >= 1.0);
    });
}
