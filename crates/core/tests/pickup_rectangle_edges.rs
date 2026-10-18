//! Ordinary pickup inputs with a controlled pre-first-tick half-pixel origin.
//! This verifies source rectangle edges, not generated-route frequency.
use spelunky_core::{Game, InputFrame, RulesPreset, Vec2, level::Level};

fn room(rules: RulesPreset) -> (Game, u64) {
    let mut level = Level::default();
    level.set_tile(6, 7, '@').unwrap();
    level.set_tile(34, 7, 'X').unwrap();
    for x in 0..spelunky_core::level::LEVEL_WIDTH {
        level.set_tile(x, 8, '1').unwrap();
    }
    let mut game = Game::shell(42, rules);
    game.load_custom(&level, "pickup-source-edge", "");
    // A legal fractional source origin is controlled only before input starts.
    game.player.pos.x += 0.5;
    game.player.previous_pos = game.player.pos;
    let rock = game.spawn("oRock", Vec2::new(132.0, 140.0));
    for _ in 0..8 {
        game.step(&InputFrame::default());
    }
    assert_eq!(game.player.pos, Vec2::new(120.5, 136.0));
    assert_eq!(game.player.health, 4.0);
    assert!(game.player.grounded);
    assert_eq!(game.player.held, None);
    let target = game.world.entity(rock).unwrap();
    assert_eq!(target.pos, Vec2::new(132.0, 140.0));
    assert_eq!(target.vel, Vec2::default());
    assert_eq!(target.sprite_bounds().x, 129.0);
    assert!(target.value("canPickUp") != 0.0);
    assert!(!game.world.solid_at(target.pos.x, target.pos.y));
    (game, rock)
}

fn pickup() -> InputFrame {
    InputFrame {
        down: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    }
}

#[test]
fn down_attack_does_not_pick_up_beyond_source_inclusive_rectangle() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, rock) = room(rules);
        // Source max128.5 is before the Rock's cached left129. The former
        // width17 adapter reaches129.5 and falsely gates instance_nearest.
        assert!(game.player.pos.x + 8.0 < 129.0);
        assert!(game.player.pos.x + 9.0 > 129.0);
        game.step(&pickup());
        assert_eq!(game.player.pos.x, 120.5);
        assert_eq!(
            game.player.held, None,
            "SOURCE_RAW_PICKUP_RECTANGLE {rules:?}"
        );
        assert!(!game.world.entity(rock).unwrap().held);
        assert_eq!(game.player.health, 4.0);
    }
}

#[test]
fn walking_into_source_rectangle_still_picks_up_the_same_rock() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, rock) = room(rules);
        for _ in 0..8 {
            if game.player.pos.x + 8.0 >= 129.0 {
                break;
            }
            game.step(&InputFrame {
                right: true,
                ..Default::default()
            });
        }
        assert!(
            game.player.pos.x + 8.0 >= 129.0,
            "ordinary approach required"
        );
        assert!(game.player.held.is_none());
        game.step(&pickup());
        assert_eq!(game.player.held, Some(rock));
        assert!(game.world.entity(rock).unwrap().held);
        assert_eq!(game.player.pickup_item_type, "Rock");
        assert_eq!(game.player.health, 4.0);
    }
}
