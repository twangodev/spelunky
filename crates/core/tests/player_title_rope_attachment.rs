//! Original rTitle rope, through ordinary public input only. The canonical
//! attachment must satisfy characterStepEvent608–630 before climbing to credits.
use spelunky_core::{catalog, dispatcher, ending, hub, *};

fn source_point(game: &Game, point: Vec2, family: &str) -> bool {
    game.world.tiles.iter().enumerate().any(|(index, tile)| {
        tile.engine_active
            && catalog::is_a(&tile.object, family)
            && game
                .world
                .tile_collision_mask(
                    (index % game.world.width) as i32,
                    (index / game.world.width) as i32,
                )
                .contains(point, CollisionPrecision::BoundingBox)
    }) || game.world.entities.iter().any(|entity| {
        entity.alive
            && entity.engine_active
            && catalog::is_a(&entity.object, family)
            && entity.mask_contains(point, CollisionPrecision::BoundingBox)
    })
}

fn run(rules: RulesPreset, historical: bool) {
    let mut game = if historical {
        Game::legacy_shell(7, rules)
    } else {
        Game::shell(7, rules)
    };
    game.load_hub(hub::HubKind::Title);
    assert_eq!(game.player.pos, Vec2::new(600.0, 184.0));
    assert_eq!(game.player.health, 4);
    let mut first_attachment = false;
    for frame in 0..221 {
        let before = game.player.clone();
        let origin_hit = source_point(&game, before.pos, "oLadder")
            || source_point(&game, before.pos, "oLadderTop");
        let above_hit = source_point(
            &game,
            Vec2::new(before.pos.x, before.pos.y - 8.0),
            "oLadder",
        );
        let order = dispatcher::descendant_instances(&game, "oLadder");
        game.step(&InputFrame {
            up: true,
            jump: frame == 0,
            jump_pressed: frame == 0,
            ..Default::default()
        });
        assert_eq!(game.player.health, 4, "{rules:?} frame {frame}");
        if !first_attachment && game.player.state == PlayerState::Climbing {
            first_attachment = true;
            if !historical {
                assert!(
                    origin_hit
                        || (matches!(before.state, PlayerState::Jumping | PlayerState::Falling)
                            && above_hit),
                    "SOURCE_ATTACHMENT_REQUIRES_NATIVE_POINT {rules:?} frame {frame}: {:?}",
                    before.pos
                );
                assert_eq!(before.sprite, "sJumpLeft");
                assert_eq!(before.animation, 0.0);
                // Original title registration order alternates stored tiles and
                // entities, but retains all 21 authored rope origins in order.
                let positions: Vec<_> = order
                    .iter()
                    .map(|instance| instance.position(&game))
                    .collect();
                assert_eq!(
                    positions,
                    (0..21)
                        .map(|i| Vec2::new(592.0, f64::from(i * 8)))
                        .collect::<Vec<_>>()
                );
                assert!(
                    order
                        .iter()
                        .all(|instance| dispatcher::is_active(&game, *instance))
                );
                let target_index = order
                    .iter()
                    .position(|instance| instance.position(&game) == Vec2::new(592.0, 152.0))
                    .unwrap();
                let mut translated = before.clone();
                translated.pos.y -= 8.0;
                assert!(
                    translated.mask_contains(Vec2::new(600.0, 155.0), CollisionPrecision::Precise)
                );
                for instance in &order[..target_index] {
                    let overlaps = match *instance {
                        dispatcher::Instance::Terrain(index) => game
                            .world
                            .tile_collision_mask(
                                (index % game.world.width) as i32,
                                (index / game.world.width) as i32,
                            )
                            .intersects(
                                translated.sprite_bounds(),
                                CollisionPrecision::BoundingBox,
                            ),
                        dispatcher::Instance::Entity(id) => {
                            game.world.entity(id).unwrap().mask_intersects(
                                translated.sprite_bounds(),
                                CollisionPrecision::BoundingBox,
                            )
                        }
                        dispatcher::Instance::Player => panic!("Player is not a ladder"),
                    };
                    assert!(!overlaps, "earlier source recipient must be disjoint");
                }
                let dispatcher::Instance::Entity(id) = order[target_index] else {
                    panic!("off-grid rope is an entity")
                };
                assert!(
                    game.world
                        .entity(id)
                        .unwrap()
                        .mask_contains(Vec2::new(600.0, 155.0), CollisionPrecision::Precise)
                );
                assert!(
                    !origin_hit,
                    "fresh origin point requires reanchoring on this route"
                );
                assert_eq!(
                    game.player.pos,
                    Vec2::new(600.0, 166.0),
                    "SOURCE_POST_SNAP_POINT_REANCHORS_TO_FIRST_PRECISE_ROPE"
                );
                assert_eq!(game.player.vel, Vec2::default());
            }
        }
        if game.state == RunState::Ending {
            break;
        }
    }
    assert!(first_attachment, "ordinary jump must attach");
    let ending = game.ending.as_ref().expect("authored title rope can exit");
    assert_eq!(ending.kind, ending::EndingKind::TitleCredits);
    assert_eq!(ending.stage, ending::EndingStage::Exit);
    assert_eq!(game.world.name, "rTitle");
    let dummy = game
        .world
        .entities
        .iter()
        .find(|entity| entity.object == "oPDummy5")
        .unwrap();
    assert!(dummy.pos.y < 32.0);
    assert_eq!(dummy.vel.y, -1.0);
    assert_eq!(game.player.data["active"], 0.0);
}

#[test]
fn authored_title_jump_attaches_only_on_native_rope_points_then_reaches_credits() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        run(rules, false);
    }
}

#[test]
fn historical_authored_title_jump_still_reaches_credits() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        run(rules, true);
    }
}
