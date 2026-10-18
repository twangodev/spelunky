//! Source Create defaults and automatic playback through the public frame.
//! These expectations derive from preserved events, not original gameplay captures.
use spelunky_core::*;

fn supported_emitter(preset: RulesPreset) -> (Game, u64, u64) {
    let mut game = Game::shell(42, preset);
    game.world = World::new(40, 30, Area::Ice, 9);
    for x in 0..40 {
        game.world.set_tile(x, 10, "oBlock");
    }
    game.world.set_tile(10, 4, "oBlock");
    game.player.pos = Vec2::new(40.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.grounded = true;
    game.player.state = PlayerState::Standing;
    let emitter = game.spawn("oBarrierEmitter", Vec2::new(160.0, 80.0));
    let barriers: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|entity| entity.alive && entity.object == "oBarrier")
        .map(|entity| entity.id)
        .collect();
    assert_eq!(barriers.len(), 1);
    (game, emitter, barriers[0])
}

#[test]
fn emitter_child_keeps_default_speed_and_source_draw_frames() {
    for preset in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, emitter, barrier) = supported_emitter(preset);
        // Emitter Create alone writes 0.5. Barrier has no parent or Create;
        // the runner's 1.0 default therefore survives its synchronous birth.
        assert_eq!(game.world.entity(emitter).unwrap().image_speed(), 0.5);
        assert_eq!(catalog::sprite_frame_count("sBarrier"), 3);
        for (tick, frame) in [0.0, 1.0, 2.0, 0.0, 1.0, 2.0].into_iter().enumerate() {
            game.step(&InputFrame::default());
            let draw = game.presentation();
            assert_eq!(draw.world.entity(barrier).unwrap().animation, frame);
            assert_eq!(
                draw.world.entity(emitter).unwrap().animation,
                tick as f64 * 0.5
            );
            let child = game.world.entity(barrier).unwrap();
            assert_eq!(child.animation, (frame + 1.0) % 3.0);
            assert_eq!(child.pos, Vec2::new(160.0, 96.0));
            assert_eq!(child.image_speed(), 1.0);
            assert_eq!(game.world.entity(emitter).unwrap().image_speed(), 0.5);
            assert_eq!(game.state, RunState::Playing);
            assert_eq!(game.player.health, 4.0);
        }
    }
}

#[test]
fn normal_bullet_collision_still_destroys_emitter_and_child_once() {
    for preset in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, emitter, barrier) = supported_emitter(preset);
        for _ in 0..4 {
            game.step(&InputFrame::default());
        }
        let bullet = game.spawn("oBullet", Vec2::new(164.0, 84.0));
        game.world.entity_mut(bullet).unwrap().vel = Vec2::default();
        game.step(&InputFrame::default());
        assert!(game.world.entity(emitter).is_none());
        assert!(game.world.entity(barrier).is_none());
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == "oFlareSpark")
                .count(),
            6
        );
        assert_eq!(
            game.events
                .iter()
                .filter(|event| {
                    matches!(event, GameEvent::Sound { name, .. } if name == "sndSmallExplode")
                })
                .count(),
            1
        );
        assert_eq!(game.player.health, 4.0);
    }
}
