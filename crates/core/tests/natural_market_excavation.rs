//! The generated Black Market entrance can be exposed through ordinary play.
//! Level generation is source-driven; all approach, bomb placement, movement,
//! and fuse ticks use public `Game::step` input.
use spelunky_core::{Game, InputFrame, PlayerState, RulesPreset, Vec2};

#[test]
fn seed_zero_market_entrance_cover_is_excavated_by_a_real_bomb() {
    let mut game = Game::new(0, RulesPreset::Classic);
    while game.world.level < 5 {
        game.next_level();
    }

    let market = game
        .world
        .entities
        .iter()
        .find(|entity| entity.alive && entity.object == "oXMarket")
        .expect("seed 0 generates a Market entrance on Level 5");
    let market_id = market.id;
    let market_pos = market.pos;
    assert_eq!(market_pos, Vec2::new(128.0, 64.0));

    let wall_x = (market_pos.x / 16.0) as i32;
    let wall_y = (market_pos.y / 16.0) as i32;
    assert_eq!(game.world.tile(wall_x, wall_y).unwrap().object, "oLush");

    // Reach the generated wall from the natural spawn. Jump whenever ordinary
    // movement meets terrain; no player or terrain coordinates are staged.
    let mut stationary = 0;
    let mut last_x = game.player.pos.x;
    let mut jump = 0;
    for tick in 0..140 {
        if tick >= 50
            && game.player.pos.x < market_pos.x + 32.0
            && game.player.pos.y < market_pos.y + 16.0
        {
            break;
        }
        if stationary >= 2 && jump == 0 {
            jump = 8;
            stationary = 0;
        }
        let jumping = jump > 0;
        if jump > 0 {
            jump -= 1;
        }
        game.step(&InputFrame {
            left: true,
            run: true,
            jump: jumping,
            jump_pressed: jumping && jump == 7,
            ..Default::default()
        });
        if (game.player.pos.x - last_x).abs() < 0.5 {
            stationary += 1;
        } else {
            stationary = 0;
        }
        last_x = game.player.pos.x;
    }
    assert!(game.player.pos.x < market_pos.x + 32.0);
    assert!(game.player.pos.y < market_pos.y + 16.0);
    assert!(game.player.pos.x > market_pos.x + 8.0);
    assert_eq!(game.player.state, PlayerState::Running);

    let bombs = game.player.bombs;
    game.step(&InputFrame {
        down: true,
        bomb_pressed: true,
        ..Default::default()
    });
    assert_eq!(game.player.bombs, bombs - 1.0);
    let bomb_id = game
        .world
        .entities
        .iter()
        .find(|entity| entity.alive && entity.object == "oBomb")
        .expect("Down+B creates the live bomb instance")
        .id;

    for _ in 0..12 {
        game.step(&InputFrame {
            right: true,
            run: true,
            ..Default::default()
        });
    }
    assert!(game.player.pos.x > 190.0, "escape the bomb blast");

    for _ in 0..150 {
        if game.world.entity(bomb_id).is_none() {
            break;
        }
        game.step(&InputFrame::default());
    }

    assert!(
        game.world.entity(bomb_id).is_none(),
        "the real fuse completes"
    );
    assert!(game.world.tile(wall_x, wall_y).unwrap().object.is_empty());
    assert!(
        game.world.entity(market_id).is_some(),
        "excavation must preserve the generated Market exit"
    );
    assert!(game.player.health > 0.0);
}
