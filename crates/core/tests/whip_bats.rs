//! Ordinary authored-room and input regressions. No mid-run actor edits.
use spelunky_core::{Game, InputFrame, RulesPreset, level::Level};

fn authored_room(symbol: char, target_x: usize, rules: RulesPreset) -> (Game, u64) {
    let mut level = Level::default();
    level.set_tile(6, 7, '@').unwrap();
    level.set_tile(34, 7, 'X').unwrap();
    level.set_tile(target_x, 7, symbol).unwrap();
    for x in 0..spelunky_core::level::LEVEL_WIDTH {
        level.set_tile(x, 8, '1').unwrap();
    }
    let mut game = Game::shell(42, rules);
    game.load_custom(&level, "whip-source-contract", "");
    let object = match symbol {
        'b' => "oBat",
        '.' => "oBombBag",
        'R' => "oRopePile",
        _ => unreachable!(),
    };
    let target = game
        .world
        .entities
        .iter()
        .find(|e| e.alive && e.object == object)
        .unwrap()
        .id;
    for _ in 0..8 {
        game.step(&InputFrame::default());
    }
    assert!(game.player.grounded);
    assert_eq!(game.player.health, 4);
    assert_eq!(game.player.facing, 1);
    (game, target)
}

#[test]
fn authored_flying_bat_is_hit_by_normal_delayed_forward_whip() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, bat) = authored_room('b', 8, rules);
        let flying = game.world.entity(bat).unwrap();
        assert_eq!(flying.state, 1, "authored bat naturally leaves HANG");
        assert_eq!(flying.sprite, "sBatLeft");
        assert!(flying.vel.x < 0.0);
        assert_eq!(flying.health, 1);
        assert!(flying.engine_active);
        game.step(&InputFrame {
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert_eq!(game.player.sprite, "sAttackLeft");
        let mut saw_forward = false;
        let mut saw_hit = false;
        let mut saw_blood = false;
        for after_press in 1..=12 {
            game.step(&InputFrame::default());
            saw_forward |= game.world.entities.iter().any(|e| e.object == "oWhip");
            saw_blood |= game
                .world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oBlood");
            if let Some(target) = game.world.entity(bat) {
                saw_hit |= target.health <= 0;
                if after_press <= 4 {
                    assert_eq!(target.health, 1, "front bat must survive rear windup");
                }
            }
            if after_press <= 4 {
                assert!(
                    !saw_forward,
                    "source forward phase requires image_index > 4"
                );
            }
        }
        assert!(saw_forward, "normal input must create a forward whip");
        assert!(
            saw_hit,
            "real inherited Enemy/Whip collision must damage bat"
        );
        assert!(saw_blood);
        assert!(
            game.world.entity(bat).is_none(),
            "bat own next Step finishes death"
        );
        assert_eq!(game.player.health, 4, "stationary strike precedes contact");
        assert_eq!(game.player.stun, 0);
    }
}

#[test]
fn authored_distant_flying_bat_survives_same_normal_whip_input() {
    let (mut game, bat) = authored_room('b', 13, RulesPreset::Classic);
    assert_eq!(game.world.entity(bat).unwrap().sprite, "sBatLeft");
    game.step(&InputFrame {
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    let mut saw_forward = false;
    for _ in 0..12 {
        game.step(&InputFrame::default());
        saw_forward |= game.world.entities.iter().any(|e| e.object == "oWhip");
    }
    assert!(saw_forward);
    assert_eq!(game.world.entity(bat).unwrap().health, 1);
    assert_eq!(game.player.health, 4);
    assert!(!game.world.entities.iter().any(|e| e.object == "oBlood"));
}

#[test]
fn authored_bomb_bag_and_rope_pile_have_no_source_whip_collision() {
    for symbol in ['.', 'R'] {
        // At x7 the bag mask intersects the forward whip's precise pixels,
        // while remaining outside the stationary player's pickup/contact mask.
        let (mut game, bag) = authored_room(symbol, 7, RulesPreset::Classic);
        let inventory = (game.player.bombs, game.player.ropes);
        game.step(&InputFrame {
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        let mut saw_forward = false;
        let mut saw_mask_contact = false;
        for _ in 0..12 {
            game.step(&InputFrame::default());
            saw_forward |= game.world.entities.iter().any(|e| e.object == "oWhip");
            let target = game.world.entity(bag).expect("bag must survive every tick");
            saw_mask_contact |= game.world.entities.iter().any(|weapon| {
                weapon.alive
                    && weapon.engine_active
                    && weapon.object == "oWhip"
                    && target.engine_active
                    && weapon.event_intersects(target)
            });
        }
        assert!(saw_forward);
        assert!(
            saw_mask_contact,
            "live forward whip must reach the bag's actual mask"
        );
        assert!(
            game.world.entity(bag).is_some(),
            "Item-family bags do not break on whip"
        );
        assert_eq!((game.player.bombs, game.player.ropes), inventory);
        assert_eq!(game.player.health, 4);
    }
}
