//! Authored shop stock and default actors; all actions after setup are ordinary
//! pickup, release and throw inputs. No HP, velocity, timers or hit flags are
//! injected. Source: oItem Step380-410 and scrShopkeeperAnger reason 3.
use spelunky_core::{Area, Game, GameEvent, InputFrame, PlayerState, RulesPreset, Vec2, World};

fn throw_at_damsel(preset: RulesPreset, facing: i8, for_sale: bool) -> (Game, u64, u64) {
    let mut game = Game::shell(23, preset);
    game.world = World::new(24, 20, Area::Mines, 1);
    game.world.name = "rLevel".into();
    for x in 0..24 {
        game.world.set_tile(x, 10, "oHardBlock");
    }
    game.player.pos = Vec2::new(96.0 - f64::from(facing) * 32.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.player.facing = facing;
    game.world.entrance = game.player.pos;
    let keeper = game.spawn(
        "oShopkeeper",
        Vec2::new(96.0 + f64::from(facing) * 48.0, 144.0),
    );
    let damsel = game.spawn("oDamsel", Vec2::new(96.0, 152.0));
    {
        let stock = game.world.entity_mut(damsel).unwrap();
        // scrRoomGen stock overrides: SLAVE=5 and forSale=true. The control
        // changes only its initial sale/cost metadata, not its body or HP.
        stock.state = 5;
        stock.data.insert("status".into(), 5.0);
        stock
            .data
            .insert("forSale".into(), if for_sale { 1.0 } else { 0.0 });
        stock
            .data
            .insert("cost".into(), if for_sale { 15000.0 } else { 0.0 });
    }
    let rock = game.spawn("oRock", Vec2::new(game.player.pos.x, 156.0));
    game.step(&InputFrame {
        down: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert_eq!(
        game.player.held,
        Some(rock),
        "ordinary Rock pickup required"
    );
    assert_eq!(game.player.attack_timer, 0);
    game.step(&InputFrame {
        attack_released: true,
        ..Default::default()
    });
    assert_eq!(game.player.held, Some(rock));
    assert_eq!(game.player.state, PlayerState::Standing);
    assert_eq!(game.world.entity(damsel).unwrap().health, 4);
    assert!(!game.world.entity(damsel).unwrap().held);
    assert_eq!(
        game.world.entity(damsel).unwrap().value("forSale"),
        if for_sale { 1.0 } else { 0.0 }
    );
    assert_eq!(game.world.entity(keeper).unwrap().health, 20);
    assert_eq!(game.world.entity(keeper).unwrap().state, 0);
    assert_eq!(game.data.get("thiefLevel").copied().unwrap_or(0.0), 0.0);
    assert!(!game.flags.contains("wanted"));

    game.step(&InputFrame {
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert!(
        game.player.held.is_none(),
        "ordinary throw must release Rock"
    );
    assert!(!game.world.entity(rock).unwrap().held);
    for frame in 0..16 {
        let target = game.world.entity(damsel).expect("surviving Damsel");
        if target.health != 4 {
            assert_eq!(target.health, 3, "one nonfatal ordinary Rock hit");
            assert!(!target.held);
            assert_eq!(target.state, 2);
            assert_eq!(
                game.world.entity(keeper).unwrap().health,
                20,
                "keeper was not struck"
            );
            assert_eq!(game.player.health, 4.0, "no unrelated player hurt");
            assert!(!game.flags.contains("murderer"));
            assert!(
                !game.flags.contains("shopkeepers_angry"),
                "no fatal-Damsel backfill"
            );
            return (game, damsel, keeper);
        }
        game.step(&InputFrame {
            attack_released: frame == 0,
            ..Default::default()
        });
    }
    panic!("fixture prerequisite: ordinary Rock throw did not hit the unheld Damsel");
}

#[test]
fn ordinary_rock_surviving_hit_on_sale_damsel_angers_keeper_with_reason_three() {
    for preset in [RulesPreset::Classic, RulesPreset::Assisted] {
        for facing in [-1, 1] {
            let (game, _, keeper) = throw_at_damsel(preset, facing, true);
            assert_eq!(game.data.get("thiefLevel").copied().unwrap_or(0.0), 2.0);
            assert!(game.flags.contains("wanted"));
            assert_eq!(game.world.entity(keeper).unwrap().state, 2);
            assert!(
                game.events.iter().any(|event| matches!(
                    event,
                    GameEvent::Dialogue(message)
                        if message.first == "HEY, ONLY I CAN DO THAT!"
                            && message.second.is_empty()
                            && message.ticks == 80
                )),
                "successful surviving sale-Damsel hit must call reason 3"
            );
        }
    }
}

#[test]
fn ordinary_rock_surviving_hit_on_free_damsel_leaves_keeper_peaceful() {
    for preset in [RulesPreset::Classic, RulesPreset::Assisted] {
        for facing in [-1, 1] {
            let (game, _, keeper) = throw_at_damsel(preset, facing, false);
            assert_eq!(game.data.get("thiefLevel").copied().unwrap_or(0.0), 0.0);
            assert!(!game.flags.contains("wanted"));
            assert_eq!(game.world.entity(keeper).unwrap().state, 0);
            assert_eq!(game.world.entity(keeper).unwrap().value("angered"), 0.0);
            assert!(!game.events.iter().any(|event| matches!(
                event,
                GameEvent::Dialogue(message) if message.first == "HEY, ONLY I CAN DO THAT!"
            )));
        }
    }
}
