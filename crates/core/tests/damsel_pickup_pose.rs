//! Native authored glyphs and public run-start character selection; after
//! loading, all simulation changes come from ordinary public InputFrame calls.
use spelunky_core::{
    CollisionPrecision, Game, GenerationRevision, InputFrame, PlayerState, RulesPreset, RunState,
    Vec2,
    characters::{self, Character},
    level::Level,
};

fn prepared(rules: RulesPreset, avatar: Character, facing: i8, historical: bool) -> (Game, u64) {
    let mut level = Level::default();
    level
        .set_tile(if facing > 0 { 8 } else { 10 }, 8, '@')
        .unwrap();
    level.set_tile(9, 8, 'D').unwrap();
    level.set_tile(34, 8, 'X').unwrap();
    for x in 0..spelunky_core::level::LEVEL_WIDTH {
        level.set_tile(x, 9, '1').unwrap();
    }
    let mut game = if historical {
        Game::legacy_shell(23, rules)
    } else {
        Game::shell(23, rules)
    };
    // Public supported run-start avatar choice, not a manual global flag.
    characters::apply(&mut game, avatar);
    game.load_custom(&level, "damsel-pickup-source-pose", "");
    assert_eq!(characters::selected(&game), avatar);
    assert_eq!(
        game.generation_revision,
        if historical {
            GenerationRevision::HistoricalV1
        } else {
            GenerationRevision::SourceClassicV2
        }
    );
    let actors: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oDamsel")
        .map(|e| e.id)
        .collect();
    assert_eq!(actors.len(), 1, "FIXTURE one default D glyph actor");
    let id = actors[0];
    assert_eq!(game.world.entity(id).unwrap().pos, Vec2::new(168.0, 152.0));
    for _ in 0..2 {
        game.step(&InputFrame::default());
    }
    for _ in 0..4 {
        game.step(&InputFrame {
            left: facing < 0,
            right: facing > 0,
            ..Default::default()
        });
    }
    for _ in 0..12 {
        game.step(&InputFrame::default());
    }
    assert_eq!(game.state, RunState::Playing);
    assert_eq!(game.player.health, 4);
    assert_eq!(game.player.state, PlayerState::Standing);
    assert!(game.player.grounded && !game.player.is_stunned());
    assert_eq!(game.player.vel, Vec2::default());
    assert_eq!(game.player.facing, facing);
    assert!(game.player.held.is_none());
    assert_eq!(game.player.attack_timer, 0);
    let actor = game.world.entity(id).unwrap();
    assert!(actor.engine_active && !actor.held);
    assert_eq!((actor.health, actor.state), (4, 0));
    assert_eq!(actor.value("canPickUp"), 1.0);
    assert_eq!(actor.image_speed(), 0.5);
    assert_eq!(
        actor.sprite,
        if avatar == Character::Damsel {
            "sStandLeft"
        } else {
            "sDamselLeft"
        }
    );
    assert!(
        actor.mask_intersects(
            spelunky_core::Rect::new(game.player.pos.x - 8.0, game.player.pos.y, 17.0, 9.0),
            CollisionPrecision::BoundingBox,
        ),
        "FIXTURE ordinary walk reaches the literal pickup rectangle"
    );
    (game, id)
}

fn actual_pickup(game: &mut Game, id: u64) {
    game.step(&InputFrame {
        down: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert_eq!(
        game.player.held,
        Some(id),
        "FIXTURE actual Down+Attack selects the same live Damsel"
    );
    let actor = game.world.entity(id).unwrap();
    assert!(actor.held && actor.engine_active);
    assert_eq!(actor.health, 4);
    assert_eq!(actor.state, 0);
    assert_eq!(actor.vel, Vec2::default());
    assert_eq!(game.player.pickup_item_type, "Damsel");
    assert_eq!(game.player.health, 4);
    assert_eq!(game.player.attack_timer, 0, "pickup does not start a whip");
    assert_eq!(game.state, RunState::Playing);
}

#[test]
fn ordinary_damsel_pickup_assigns_source_carried_pose_and_mask() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for avatar in [Character::Explorer, Character::Damsel] {
            for facing in [-1, 1] {
                let (mut game, id) = prepared(rules, avatar, facing, false);
                let resources = (game.player.bombs, game.player.ropes, game.player.money);
                actual_pickup(&mut game, id);
                let wanted = if avatar == Character::Damsel {
                    "sDieLBounce"
                } else {
                    "sDamselHoldL"
                };
                // Primary visible Source contract, only after true pickup admission.
                assert_eq!(
                    game.world.entity(id).unwrap().sprite,
                    wanted,
                    "CONTRACT_SOURCE_PICKUP_SELECTS_DAMSEL_CARRIED_ART: avatar={avatar:?} facing={facing}"
                );
                for elapsed in 0..=3 {
                    if elapsed > 0 {
                        game.step(&InputFrame::default());
                    }
                    let actor = game.world.entity(id).unwrap();
                    assert_eq!(actor.sprite, wanted);
                    assert_eq!(actor.image_speed(), 0.5, "pickup does not reset speed");
                    assert_eq!(actor.facing, facing);
                    assert_eq!(
                        actor.value("image_xscale"),
                        if facing > 0 { -1.0 } else { 1.0 }
                    );
                    assert_eq!(
                        game.presentation().world.entity(id).unwrap().sprite,
                        wanted,
                        "Draw capture preserves current entity art"
                    );
                    let top_outside = if avatar == Character::Damsel {
                        -6.0
                    } else {
                        -8.0
                    };
                    assert!(
                        !actor.mask_contains(
                            Vec2::new(actor.pos.x, actor.pos.y + top_outside),
                            CollisionPrecision::BoundingBox
                        ),
                        "CONTRACT held sprite's aggregate top excludes the old standing-only row"
                    );
                    assert!(actor.held && actor.health == 4);
                    assert_eq!(game.player.held, Some(id));
                    assert_eq!(
                        (game.player.bombs, game.player.ropes, game.player.money),
                        resources
                    );
                    assert_eq!(game.player.health, 4);
                }
            }
        }
    }
}

#[test]
fn historical_pickup_retains_existing_held_sprite_behavior() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for avatar in [Character::Explorer, Character::Damsel] {
            for facing in [-1, 1] {
                let (mut game, id) = prepared(rules, avatar, facing, true);
                let before = game.world.entity(id).unwrap().sprite.clone();
                actual_pickup(&mut game, id);
                for _ in 0..4 {
                    assert_eq!(game.world.entity(id).unwrap().sprite, before);
                    assert_eq!(game.player.held, Some(id));
                    game.step(&InputFrame::default());
                }
            }
        }
    }
}
