//! Authored source-glyph actors and an explicitly controlled one-pixel initial
//! Player offset. Every subsequent action is an ordinary public InputFrame;
//! no generated-campaign reachability claim.
use spelunky_core::{Game, GameEvent, InputFrame, RulesPreset, level::Level};

fn room(start: usize, target: char, object: &str, rules: RulesPreset) -> (Game, u64) {
    let mut level = Level::default();
    level.set_tile(start, 7, '@').unwrap();
    level.set_tile(7, 7, target).unwrap();
    level.set_tile(34, 7, 'X').unwrap();
    for x in 0..spelunky_core::level::LEVEL_WIDTH {
        level.set_tile(x, 8, '1').unwrap();
    }
    let mut game = Game::shell(42, rules);
    game.load_custom(&level, "source-bag-whip-control", "");
    // Pre-first-input only: source bordered entrancecenter120 would auto-collect wide
    // GoldBars at inclusive left128. Atx119 pickup right127 misses it, while
    // the forward Whip origin135 still reaches its rectangle.
    game.player.pos.x -= 1.0;
    game.player.previous_pos = game.player.pos;
    assert_eq!(game.player.pos.x, 119.0);
    let id = game
        .world
        .entities
        .iter()
        .find(|e| e.alive && e.object == object)
        .unwrap()
        .id;
    let expected_y = match object {
        "oBombBag" => 138.0,
        "oBombBox" | "oGoldBars" => 136.0,
        "oRopePile" => 139.0,
        "oGoldBar" | "oEmeraldBig" | "oDiamond" => 140.0,
        _ => panic!("source glyph table must declare the actor"),
    };
    assert_eq!(
        (
            game.world.entity(id).unwrap().pos.x,
            game.world.entity(id).unwrap().pos.y
        ),
        (136.0, expected_y)
    );
    for _ in 0..8 {
        game.step(&InputFrame::default());
    }
    assert!(game.player.grounded);
    assert_eq!((game.player.pos.x, game.player.pos.y), (119.0, 136.0));
    assert!(game.world.entity(id).is_some());
    let entity = game.world.entity(id).unwrap();
    assert_eq!((entity.pos.x, entity.pos.y), (136.0, expected_y));
    assert_eq!((entity.vel.x, entity.vel.y), (0.0, 0.0));
    (game, id)
}

fn whip_without_walking(game: &mut Game, target: u64, expected_contact: bool) {
    game.step(&InputFrame {
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    let mut saw_forward = false;
    let mut saw_post_step_mask_overlap = false;
    for _ in 0..12 {
        game.step(&InputFrame::default());
        let target = game
            .world
            .entity(target)
            .expect("source has no whip-break callback for this pickup");
        for e in game
            .world
            .entities
            .iter()
            .filter(|e| e.alive && e.object == "oWhip")
        {
            saw_forward = true;
            // This is a post-EndFrame sample, not an instrumentation of the
            // earlier Collision phase. Source ownership separately proves
            // that this pickup has no Jar-like break event.
            saw_post_step_mask_overlap |= e.event_intersects(target);
        }
    }
    assert!(saw_forward);
    assert_eq!(
        saw_post_step_mask_overlap, expected_contact,
        "literal source sprite geometry, not rear-only windup"
    );
    assert!(
        !game
            .world
            .entities
            .iter()
            .any(|e| matches!(e.object.as_str(), "oRubbleSmall" | "oSmokePuff"))
    );
    assert_eq!(game.player.health, 4);
}

#[test]
fn source_free_supply_bags_survive_front_whip_then_collect_by_walking() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (glyph, object, bombs, ropes) in [
            ('.', "oBombBag", 3, 0),
            (':', "oBombBox", 12, 0),
            ('R', "oRopePile", 0, 3),
        ] {
            let (mut game, id) = room(6, glyph, object, rules);
            let entity = game.world.entity(id).unwrap();
            assert_eq!(entity.object, object);
            assert_eq!(entity.value("cost"), 0.0);
            assert_eq!(entity.value("forSale"), 0.0);
            let before = (game.player.bombs, game.player.ropes);
            whip_without_walking(&mut game, id, true);
            assert_eq!((game.player.bombs, game.player.ropes), before);
            for _ in 0..16 {
                if game.world.entity(id).is_none() {
                    break;
                }
                game.step(&InputFrame {
                    right: true,
                    ..Default::default()
                });
            }
            assert!(game.world.entity(id).is_none());
            assert_eq!(game.player.bombs, before.0 + bombs);
            assert_eq!(game.player.ropes, before.1 + ropes);
            assert_eq!(game.player.held, None);
        }
    }
}

#[test]
fn source_loose_money_and_gems_survive_whip_then_collect_without_container_loot() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        // Grounded Whip y134..138 passes ABOVE GoldBar y139..143.
        // Larger treasure has genuine sampled overlap; no pickup has a break event.
        for (glyph, object, value, contact) in [
            ('$', "oGoldBar", 500, false),
            ('*', "oGoldBars", 1000, true),
            ('5', "oEmeraldBig", 800, true),
            ('8', "oDiamond", 5000, true),
        ] {
            let (mut game, id) = room(6, glyph, object, rules);
            assert_eq!(game.world.entity(id).unwrap().object, object);
            whip_without_walking(&mut game, id, contact);
            let mut collected = Vec::new();
            for _ in 0..16 {
                if game.world.entity(id).is_none() {
                    break;
                }
                game.step(&InputFrame {
                    right: true,
                    ..Default::default()
                });
                for event in &game.events {
                    if let GameEvent::Collected { object, value } = event {
                        collected.push((object.clone(), *value));
                    }
                }
            }
            assert!(game.world.entity(id).is_none());
            assert_eq!(collected, vec![(object.to_string(), value)]);
            assert_eq!(game.player.held, None);
            assert!(
                !game
                    .world
                    .entities
                    .iter()
                    .any(|e| e.object == "oRubbleSmall")
            );
        }
    }
}
