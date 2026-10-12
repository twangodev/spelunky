//! Source: Player Step613–623; Item/Bomb Create; Bomb Step and Alarm1.
//! Fixed initial seed/room/default actors, then public inputs only. Shop cases
//! exercise the current keeper-Create room adapter, not literal roomPath4/5.
use spelunky_core::*;

const SEED: u32 = 0;
const ORIGIN: Vec2 = Vec2::new(136.0, 152.0);

fn scene(rules: RulesPreset, keeper: bool) -> (Game, Option<u64>) {
    let mut game = Game::shell(SEED, rules);
    game.world.name = "rLevel".into();
    game.player.pos = ORIGIN;
    game.player.previous_pos = ORIGIN;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.world.entrance = ORIGIN;
    for x in 0..game.world.width {
        game.world.set_tile(x, 10, "oHardBlock");
    }
    // All setup ends before the first Step; no artificial arming/shop flags.
    let keeper = keeper.then(|| game.spawn("oShopkeeper", Vec2::new(152.0, 144.0)));
    if let Some(id) = keeper {
        let actor = game.world.entity(id).unwrap();
        assert_eq!(actor.state, 0);
        assert_eq!(actor.health, 20);
        assert_eq!(actor.value("armed"), 0.0);
        assert_eq!(actor.value("dead"), 0.0);
        assert_eq!(actor.value("angered"), 0.0);
        assert_eq!(actor.value("shop_room_x"), 0.0);
        assert_eq!(actor.value("shop_room_y"), 1.0);
    }
    (game, keeper)
}

fn bombs(game: &Game) -> Vec<&Entity> {
    game.world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oBomb")
        .collect()
}

fn assert_completed_step(game: &Game) {
    assert_eq!(game.dispatcher.phase, dispatcher::Phase::Idle);
    assert_eq!(game.presentation().tick, game.tick);
    assert_eq!(
        game.presentation().dispatcher.phase,
        dispatcher::Phase::Draw
    );
}

fn assert_no_warning(game: &Game, keeper: Option<u64>) {
    assert!(!game.flags.contains("wanted"));
    assert!(!game.flags.contains("shopkeepers_angry"));
    assert_eq!(game.data.get("thiefLevel").copied().unwrap_or(0.0), 0.0);
    assert!(!game.events.iter().any(|e| matches!(e,
        GameEvent::Dialogue(message) if message.first == "TERRORIST!")));
    if let Some(id) = keeper {
        let actor = game.world.entity(id).unwrap();
        assert!(
            actor.engine_active,
            "a frozen keeper cannot establish the negative warning case"
        );
        assert_eq!(actor.state, 0);
        assert_eq!(actor.health, 20);
        assert_eq!(actor.value("angered"), 0.0);
        assert_eq!(actor.value("dead"), 0.0);
        assert_eq!(actor.value("shop_room_x"), 0.0);
        assert_eq!(actor.value("shop_room_y"), 1.0);
    }
}

#[test]
fn ordinary_up_attack_chest_trap_retains_create_armed_flag_and_alarm_fuse() {
    // Source resource order makes the newborn's own Step observable this frame:
    // Player319 < Bomb454 < Chest472 < Level553 < Shopkeeper666. The complete
    // Alarm pass preceded Player Step, so the newborn fuse must still be40.
    assert_eq!(catalog::object("oPlayer1").unwrap().id, 319);
    assert_eq!(catalog::object("oBomb").unwrap().id, 454);
    assert_eq!(catalog::object("oChest").unwrap().id, 472);
    assert_eq!(catalog::object("oShopkeeper").unwrap().id, 666);
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for shop in [false, true] {
            let (mut game, keeper) = scene(rules, shop);
            let chest = game.spawn("oChest", ORIGIN);
            assert!(
                game.world
                    .entity(chest)
                    .unwrap()
                    .mask_contains(game.player.pos, CollisionPrecision::BoundingBox)
            );
            let inventory = (game.player.bombs, game.player.ropes, game.player.money);
            // The source rand(1,12)==1 is range(12)==0. Canonical seed0's
            // first LCG state is1; this is a natural first-draw branch, with
            // no seed search, stream replacement, or post-start writes.
            let mut expected = Rng::new(SEED);
            assert_eq!(expected.range(12), 0);
            let vx = f64::from(expected.range(4)) - f64::from(expected.range(4));
            assert!(
                vx < 0.0,
                "trap leaves the default keeper on its right untouched"
            );
            if shop {
                expected.range(32); // peaceful keeper's later first welcome
            }
            game.step(&InputFrame {
                up: true,
                attack: true,
                attack_pressed: true,
                ..Default::default()
            });
            assert_completed_step(&game);
            assert_eq!(game.tick, 1);
            assert_eq!(game.world.entity(chest).unwrap().sprite, "sChestOpen");
            assert_eq!(game.lifetime_stats.total_chests, 1);
            assert_eq!(game.data["totalChests"], 1.0);
            assert_eq!(
                (game.player.bombs, game.player.ropes, game.player.money),
                inventory
            );
            assert_eq!(game.player.held, None);
            assert_eq!(
                game.player.attack_timer, 0,
                "opening consumes the attack edge"
            );
            assert!(!game.world.entities.iter().any(|e| e.alive
                && (catalog::is_a(&e.object, "oWhip") || catalog::is_a(&e.object, "oWhipPre"))));
            let sounds: Vec<_> = game
                .events
                .iter()
                .filter_map(|e| match e {
                    GameEvent::Sound { name, .. } | GameEvent::SoundIfIdle { name, .. } => {
                        Some(name.as_str())
                    }
                    _ => None,
                })
                .collect();
            assert_eq!(sounds, ["sndTrap"]);
            let live = bombs(&game);
            assert_eq!(live.len(), 1);
            let bomb = live[0];
            let id = bomb.id;
            assert_eq!(
                bomb.value("armed"),
                0.0,
                "SOURCE_CHEST_TRAP_RETAINS_ITEM_CREATE_ARMED_FALSE"
            );
            assert_eq!(
                game.rng.state_bits(),
                expected.state_bits(),
                "source trap/welcome draw order"
            );
            assert_eq!(
                bomb.pos,
                Vec2::new(ORIGIN.x + vx, ORIGIN.y - 2.0),
                "source producer velocity feeds same-frame inherited Item motion"
            );
            assert_eq!(bomb.vel, Vec2::new(vx, -2.0 + 0.6));
            assert_eq!(bomb.sprite, "sBombArmed");
            assert_eq!(bomb.image_speed(), 1.0);
            assert_eq!(bomb.value("depth"), 49.0);
            assert_eq!(bomb.alarms[0], -1);
            assert_eq!(bomb.alarms[1], 40);
            assert_eq!(bomb.value("cost"), 0.0);
            assert_eq!(bomb.value("forSale"), 0.0);
            assert_eq!(bomb.age, 0, "birth followed the frame's age pass");
            assert!(bomb.engine_active);
            assert_eq!(
                bomb.value("simulation_tick"),
                game.engine_time() as f64,
                "newborn Bomb own Step ran, including its armed warning guard"
            );
            assert_eq!(game.presentation().world.entity(id).unwrap().animation, 0.0);
            assert_eq!(bomb.animation, 1.0, "Animation follows Draw");
            assert_no_warning(&game, keeper);
            // No mutation of actors or RNG from this point: neutral Steps
            // must count down Alarm1 even though the native armed flag isfalse.
            for elapsed in 1..=39 {
                game.step(&InputFrame::default());
                assert_completed_step(&game);
                let bomb = game.world.entity(id).expect("trap survives before Alarm1");
                assert!(bomb.engine_active, "fuse tick {elapsed}");
                assert_eq!(bomb.alarms[0], -1);
                assert_eq!(bomb.alarms[1], 40 - elapsed, "fuse tick {elapsed}");
                assert_eq!(bomb.value("armed"), 0.0);
                assert_eq!(bomb.sprite, "sBombArmed");
                assert_no_warning(&game, keeper);
                assert!(
                    !game
                        .events
                        .iter()
                        .any(|e| matches!(e, GameEvent::Explosion { .. }))
                );
            }
            game.step(&InputFrame::default());
            assert_completed_step(&game);
            assert!(game.world.entity(id).is_none());
            assert_eq!(
                game.events
                    .iter()
                    .filter(|e| matches!(e, GameEvent::Explosion { .. }))
                    .count(),
                1
            );
            assert_eq!(
                (game.player.bombs, game.player.ropes),
                (inventory.0, inventory.1)
            );
        }
    }
}

#[test]
fn ordinary_bomb_button_still_arms_and_warns_only_with_nearby_shop_keeper() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for shop in [false, true] {
            let (mut game, keeper) = scene(rules, shop);
            let inventory = game.player.bombs;
            game.step(&InputFrame {
                down: true,
                bomb_pressed: true,
                ..Default::default()
            });
            assert_completed_step(&game);
            let live = bombs(&game);
            assert_eq!(live.len(), 1);
            let bomb = live[0];
            assert!(bomb.engine_active);
            assert_eq!(bomb.value("simulation_tick"), game.engine_time() as f64);
            assert_eq!(bomb.value("armed"), 1.0);
            assert_eq!(bomb.sprite, "sBombArmed");
            assert_eq!(bomb.image_speed(), 0.2);
            assert_eq!(bomb.alarms[0], 80);
            assert_eq!(bomb.alarms[1], -1);
            assert_eq!(game.player.bombs, inventory - 1);
            assert!(
                !game
                    .events
                    .iter()
                    .any(|e| matches!(e, GameEvent::Explosion { .. }))
            );
            if let Some(id) = keeper {
                let actor = game.world.entity(id).unwrap();
                assert!(actor.engine_active);
                assert_eq!(actor.value("shop_room_x"), 0.0);
                assert_eq!(actor.value("shop_room_y"), 1.0);
                assert_eq!(actor.state, 2);
                assert_eq!(actor.value("angered"), 1.0);
                assert!(game.flags.contains("wanted"));
                assert_eq!(game.data["thiefLevel"], 2.0);
                assert_eq!(
                    game.events
                        .iter()
                        .filter(|e| matches!(e,
                    GameEvent::Dialogue(message) if message.first == "TERRORIST!"))
                        .count(),
                    1
                );
            } else {
                assert_no_warning(&game, None);
            }
        }
    }
}
