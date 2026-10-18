//! Fresh Player Create and an ordinary neutral Step after configured water birth.
use spelunky_core::*;

fn room() -> level::Level {
    let mut room = level::Level::default();
    room.set_tile(8, 6, '@').unwrap();
    for x in 0..40 {
        room.set_tile(x, 7, '1').unwrap();
    }
    room
}

fn incoming_left(rules: RulesPreset, character: characters::Character) -> Game {
    let mut game = Game::shell(42, rules);
    game.load_custom(&room(), "incoming-left", "");
    characters::apply(&mut game, character);
    for _ in 0..4 {
        game.step(&InputFrame::default());
    }
    let pose = game.player.pos;
    assert!(game.player.grounded);
    assert_eq!(game.player.state, PlayerState::Standing);
    game.step(&InputFrame {
        left: true,
        ..Default::default()
    });
    assert_eq!(
        game.player.facing, -1,
        "ordinary Left input establishes incoming facing"
    );
    assert_eq!(game.player.pos, pose);
    assert!(!game.player.is_stunned());
    assert_eq!(game.state, RunState::Playing);
    game
}

fn assert_fresh_right(game: &Game) {
    assert_eq!(game.player.facing, 1, "SOURCE_FRESH_PLAYER_CREATE_RIGHT");
    assert_eq!(game.player.pos, game.world.entrance);
    assert_eq!(game.player.previous_pos, game.player.pos);
    assert_eq!(game.player.vel, Vec2::default());
    assert_eq!(game.player.state, PlayerState::Falling);
    assert!(!game.player.is_stunned());
    assert_eq!(game.state, RunState::Playing);
}

#[test]
fn incoming_left_custom_and_linked_rooms_create_a_right_facing_player() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for character in [
            characters::Character::Explorer,
            characters::Character::Damsel,
            characters::Character::TunnelMan,
        ] {
            for linked in [false, true] {
                let mut game = incoming_left(rules, character);
                let health = game.player.health;
                let topology: Vec<_> = game
                    .world
                    .tiles
                    .iter()
                    .map(|tile| tile.object.clone())
                    .collect();
                if linked {
                    game.continue_custom(&room(), "linked-right", "incoming-left");
                    assert_eq!(game.player.health, health);
                } else {
                    game.load_custom(&room(), "fresh-right", "");
                    assert_eq!(game.player.health, f64::from(room().starting_life));
                }
                assert_eq!(
                    game.world
                        .tiles
                        .iter()
                        .map(|tile| tile.object.clone())
                        .collect::<Vec<_>>(),
                    topology
                );
                assert_fresh_right(&game);
                let health = game.player.health;
                game.step(&InputFrame::default());
                assert_eq!(game.player.facing, 1);
                assert_eq!(game.player.data.get("image_xscale"), Some(&-1.0));
                assert_eq!(game.player.health, health);
                assert!(game.player.grounded);
                assert!(!game.player.is_stunned());
            }
        }
    }
}

#[test]
fn incoming_left_campaign_and_tutorial_room_constructors_reset_facing() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for character in [
            characters::Character::Explorer,
            characters::Character::Damsel,
            characters::Character::TunnelMan,
        ] {
            for destination in 0..3 {
                let mut game = incoming_left(rules, character);
                match destination {
                    0 => game.next_level(),
                    1 => game.start_from_hub(true, 1),
                    _ => game.start_from_hub(false, 5),
                }
                assert_fresh_right(&game);
                assert_eq!(characters::selected(&game), character);
            }
        }
    }
}

#[test]
fn hub_creation_retains_its_explicit_title_facing_overrides() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for kind in [
            hub::HubKind::Scores,
            hub::HubKind::Sun,
            hub::HubKind::Moon,
            hub::HubKind::Stars,
        ] {
            let mut game = incoming_left(rules, characters::Character::Explorer);
            game.load_hub(kind);
            assert_fresh_right(&game);
        }
        for (start, facing) in [(1, 1), (2, -1), (3, 1)] {
            let mut game = Game::shell(42, rules);
            // Initial source room-return context; the later ordinary Left
            // input still establishes the constructor's incoming facing.
            game.flags.insert(format!("title_start:{start}"));
            game.load_custom(&room(), "title-facing-context", "");
            for _ in 0..4 {
                game.step(&InputFrame::default());
            }
            game.step(&InputFrame {
                left: true,
                ..Default::default()
            });
            assert_eq!(game.player.facing, -1);
            game.load_hub(hub::HubKind::Title);
            assert_eq!(game.player.facing, facing);
        }
    }
}

#[test]
fn configured_fractional_water_birth_is_seen_by_the_first_neutral_player_step() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for water_x in [144.0, 152.4] {
            let mut game = Game::shell(42, rules);
            game.load_custom(&room(), "raw-water-entry", "");
            let water = game.spawn("oWaterSwim", Vec2::new(water_x, 112.0));
            let actor = game.world.entity(water).unwrap();
            assert!(actor.alive && actor.engine_active);
            assert_eq!(actor.pos, Vec2::new(water_x, 112.0));
            assert!(actor.mask_contains(game.player.pos, CollisionPrecision::BoundingBox));
            if water_x > game.player.pos.x {
                assert!(
                    !actor.sprite_bounds().contains(game.player.pos),
                    "smooth geometry excludes the source-rounded left edge"
                );
            }
            assert_eq!(game.player.pos, Vec2::new(152.0, 120.0));
            assert_eq!(game.player.data.get("swimming"), None);
            let health = game.player.health;
            game.step(&InputFrame::default());
            assert_eq!(
                game.player.data.get("swimming"),
                Some(&1.0),
                "SOURCE_RAW_WATER_ENTRY"
            );
            assert_eq!(game.player.data.get("bubble_timer"), Some(&20.0));
            assert_eq!(game.player.data.get("fall_timer"), Some(&0.0));
            assert_eq!(
                game.world
                    .entities
                    .iter()
                    .filter(|e| e.object == "oSplash")
                    .count(),
                1
            );
            assert_eq!(
                game.world
                    .entities
                    .iter()
                    .filter(|e| e.object == "oBubble")
                    .count(),
                1
            );
            assert!(game.events.iter().any(
                |event| matches!(event, GameEvent::Sound { name, .. } if name == "sndSplash")
            ));
            for expected in [19.0, 18.0] {
                game.step(&InputFrame::default());
                assert_eq!(game.player.data.get("bubble_timer"), Some(&expected));
                assert_eq!(game.player.data.get("swimming"), Some(&1.0));
                assert!(!game.events.iter().any(
                    |event| matches!(event, GameEvent::Sound { name, .. } if name == "sndSplash")
                ));
            }
            assert_eq!(game.player.pos, Vec2::new(152.0, 120.0));
            assert_eq!(
                game.world.entity(water).unwrap().pos,
                Vec2::new(water_x, 112.0)
            );
            assert_eq!(game.player.health, health);
            assert_eq!(game.state, RunState::Playing);
        }
    }
}
