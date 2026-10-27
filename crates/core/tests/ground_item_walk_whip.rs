//! Ordinary walking and attack against settled ground objects in Classic.
use spelunky_core::{
    Area, Game, GameEvent, InputFrame, PlayerState, RulesPreset, Vec2, World, catalog,
};

fn fixture(object: &str) -> (Game, u64) {
    let mut game = Game::shell(23, RulesPreset::Classic);
    game.world = World::new(30, 20, Area::Mines, 1);
    game.world.name = "rLevel".into();
    for x in 0..30 {
        game.world.set_tile(x, 10, "oBrick");
    }
    game.player.pos = Vec2::new(48., 152.);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.player.facing = 1;
    let target = game.spawn(object, Vec2::new(112., 152.));
    (game, target)
}

#[test]
fn walking_to_ground_items_then_whipping_and_collecting() {
    for object in ["oJar", "oGoldBars", "oBombBag"] {
        let (mut game, target) = fixture(object);
        if object == "oBombBag" {
            game.world
                .entity_mut(target)
                .unwrap()
                .data
                .insert("cost".into(), 0.);
        }
        for _ in 0..30 {
            game.step(&InputFrame::default());
        }
        let start_x = game.player.pos.x;
        let initial_bombs = game.player.bombs;
        let initial_collect = game.data.get("collect").copied().unwrap_or(0.);
        for _ in 0..80 {
            if game.world.entity(target).unwrap().pos.x - game.player.pos.x <= 25. {
                break;
            }
            game.step(&InputFrame {
                right: true,
                ..Default::default()
            });
        }
        assert!(
            game.world.entity(target).unwrap().pos.x - game.player.pos.x <= 25.,
            "approach {object}"
        );
        assert!(game.player.pos.x > start_x, "player walked toward {object}");
        assert!(game.player.grounded, "player is grounded before attack");
        let settled = game.world.entity(target).expect("target survives approach");
        assert_eq!(settled.vel.y, 0., "target settled before attack: {object}");
        game.step(&InputFrame {
            attack_pressed: true,
            ..Default::default()
        });
        let mut whip_overlap = false;
        let mut break_sound = false;
        for _ in 0..6 {
            game.step(&InputFrame::default());
            break_sound |= game
                .events
                .iter()
                .any(|event| matches!(event, GameEvent::Sound { name, .. } if name == "sndBreak"));
            if let Some(target_entity) = game.world.entity(target) {
                whip_overlap |= game.world.entities.iter().any(|whip| {
                    whip.alive
                        && whip.object == "oWhip"
                        && catalog::instance_mask(
                            &whip.object,
                            &whip.sprite,
                            whip.pos,
                            whip.animation,
                            Vec2::new(1., 1.),
                            0.,
                        )
                        .intersects_mask(&catalog::instance_mask(
                            &target_entity.object,
                            &target_entity.sprite,
                            target_entity.pos,
                            target_entity.animation,
                            Vec2::new(1., 1.),
                            0.,
                        ))
                });
            }
        }
        if object == "oJar" {
            assert!(
                game.world.entity(target).is_none(),
                "walking whip must break jar"
            );
            assert!(break_sound, "jar break emits source sound");
            assert!(
                game.world
                    .entities
                    .iter()
                    .any(|entity| entity.alive && entity.object == "oRubbleSmall"),
                "jar break creates debris"
            );
        } else {
            assert!(
                game.world.entity(target).is_some(),
                "whip must not break {object}"
            );
            assert!(whip_overlap, "forward whip must reach {object}");
            for _ in 0..40 {
                if game.world.entity(target).is_none() {
                    break;
                }
                game.step(&InputFrame {
                    right: true,
                    ..Default::default()
                });
            }
            assert!(
                game.world.entity(target).is_none(),
                "walking must collect {object}"
            );
            if object == "oGoldBars" {
                assert_eq!(
                    game.data.get("collect").copied().unwrap_or(0.) - initial_collect,
                    1000.
                );
            } else {
                assert_eq!(game.player.bombs - initial_bombs, 3.);
            }
        }
    }
}
