//! End-to-end collision dispatch for the source WebBall Collision events.
use spelunky_core::{Area, Game, InputFrame, PlayerState, RulesPreset, Vec2, World};

fn room() -> Game {
    let mut game = Game::shell(17, RulesPreset::Classic);
    game.world = World::new(40, 30, Area::Mines, 1);
    game.world.name = "rLevel".into();
    game.player.pos = Vec2::new(500.0, 360.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game
}

fn ready_web_ball(game: &mut Game, pos: Vec2) -> u64 {
    let id = game.spawn("oWebBall", pos);
    let ball = game.world.entity_mut(id).unwrap();
    ball.vel = Vec2::default();
    ball.data.insert("life".into(), 50.0);
    // Model a projectile after its synchronous Create event, with launch
    // motion cleared so the next Game::step isolates collision dispatch.
    ball.data.insert("web_create_initialized".into(), 1.0);
    id
}

#[test]
fn game_step_routes_web_ball_enemy_exception_and_existing_web_collisions() {
    for (target_object, target_type, should_expand) in [
        ("oCaveman", Some("Caveman"), true),
        ("oGiantSpider", Some("Giant Spider"), false),
        ("oWeb", None, true),
    ] {
        let mut game = room();
        let pos = Vec2::new(160.0, 120.0);
        let ball = ready_web_ball(&mut game, pos);
        let target = game.spawn(target_object, pos);
        if let Some(source_type) = target_type {
            game.world
                .entity_mut(target)
                .unwrap()
                .text
                .insert("type".into(), source_type.into());
        }

        game.step(&InputFrame::default());

        let ball = game
            .world
            .entity(ball)
            .expect("the source WebBall collision leaves expansion visible");
        assert_eq!(
            ball.sprite == "sWebCreate",
            should_expand,
            "source target {target_object} ({target_type:?})"
        );
        if should_expand {
            assert_eq!(ball.vel, Vec2::default());
            assert_eq!(ball.state, 1);
            assert!(ball.value("life") > 0.0);
        } else {
            assert_ne!(ball.vel, Vec2::default());
            assert_eq!(ball.state, 0);
        }
    }
}

#[test]
fn game_step_barrier_source_event_destroys_web_ball_without_expansion() {
    let mut game = room();
    let pos = Vec2::new(160.0, 120.0);
    let barrier = game.spawn("oBarrier", pos);
    let ball = ready_web_ball(&mut game, pos);

    game.step(&InputFrame::default());

    assert!(game.world.entity(barrier).is_some());
    assert!(
        game.world.entity(ball).is_none(),
        "oBarrier Collision with oWebBall destroys the projectile"
    );
    assert!(
        !game
            .world
            .entities
            .iter()
            .any(|e| e.alive && e.object == "oWeb"),
        "a destroyed WebBall must not reach its Animation End expansion"
    );
}
