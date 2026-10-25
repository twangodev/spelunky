//! Source Player Step1217-1350 selects the empty-hand branch before pickup.
//! A Down+Attack pickup during Tunnel Man's innate swing must finish pickup
//! on that tick; the next Attack edge then acts on the now-held item even
//! while the original swing animation is still running.
use spelunky_core::{
    Game, GameEvent, InputFrame, RulesPreset,
    characters::{self, Character},
    level::Level,
};

fn grounded_tunnelman() -> Game {
    let mut level = Level::default();
    level.set_tile(4, 3, '@').unwrap();
    for x in 0..40 {
        level.set_tile(x, 4, '1').unwrap();
    }

    let mut game = Game::shell(1, RulesPreset::Classic);
    game.load_custom(&level, "tunnelman-swing-pickup", "");
    characters::apply(&mut game, Character::TunnelMan);
    for _ in 0..8 {
        game.step(&InputFrame::default());
    }
    assert!(game.player.grounded, "natural settle on the authored floor");
    assert_eq!(game.player.health, 2.0);
    game
}

fn start_swing_and_pick_up(game: &mut Game, object: &str) -> u64 {
    game.step(&InputFrame {
        attack: true,
        attack_pressed: true,
        ..InputFrame::default()
    });
    assert_eq!(game.player.sprite, "sTunnelAttackL");
    assert_eq!(game.player.data.get("character_attack_active"), Some(&1.0));

    // Release Attack for one tick so the next Down+Attack is a new edge.
    game.step(&InputFrame::default());
    let item = game.spawn(object, game.player.pos);
    game.step(&InputFrame {
        down: true,
        attack: true,
        attack_pressed: true,
        ..InputFrame::default()
    });

    // In Classic, holdItem==0 selects the pickup branch before the pickup
    // mutates holdItem. The same tick must not re-enter scrUseItem.
    assert_eq!(
        game.player.held,
        Some(item),
        "Down+Attack picks up the item"
    );
    assert!(game.world.entity(item).unwrap().held);
    assert_eq!(game.player.sprite, "sTunnelAttackL");
    assert_eq!(game.player.data.get("character_attack_active"), Some(&1.0));
    assert!(game.player.attack_timer > 0);
    assert!(
        !game
            .world
            .entities
            .iter()
            .any(|e| e.alive && e.object == "oBullet"),
        "pickup must not also use a Shotgun on the same action branch"
    );
    item
}

#[test]
fn tunnelman_can_pick_up_then_use_items_during_innate_swing() {
    for object in ["oRock", "oShotgun"] {
        let mut game = grounded_tunnelman();
        let item = start_swing_and_pick_up(&mut game, object);

        // A fresh Attack edge enters the source held-item branch. The source
        // keeps `whipping` true until Animation End; it does not block these
        // item actions while the Tunnel Man attack sprite is still active.
        game.step(&InputFrame::default());
        game.step(&InputFrame {
            attack: true,
            attack_pressed: true,
            ..InputFrame::default()
        });
        assert_eq!(game.player.sprite, "sTunnelAttackL");
        assert_eq!(game.player.data.get("character_attack_active"), Some(&1.0));

        let used = game.world.entity(item).unwrap();
        match object {
            "oRock" => {
                assert_eq!(game.player.held, None);
                assert!(!used.held);
                assert!(used.vel.x > 0.0, "Rock was thrown facing right");
                assert!(game.events.iter().any(
                    |event| matches!(event, GameEvent::Sound { name, .. } if name == "sndThrow")
                ));
            }
            "oShotgun" => {
                assert_eq!(game.player.held, Some(item));
                assert!(used.held);
                assert_eq!(
                    game.world
                        .entities
                        .iter()
                        .filter(|e| e.alive && e.object == "oBullet")
                        .count(),
                    6,
                    "Shotgun fires during the innate swing"
                );
                assert!(game.events.iter().any(
                    |event| matches!(event, GameEvent::Sound { name, .. } if name == "sndShotgun")
                ));
            }
            _ => unreachable!(),
        }
    }
}
