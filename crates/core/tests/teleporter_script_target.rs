//! Source caller scratch variables are exercised through ordinary input.
//! Authored initial actors/shop metadata are explicit; no runtime state writes.
use spelunky_core::{
    Area, Game, GameEvent, InputFrame, PlayerState, RulesPreset, Vec2, World,
    characters::{self, Character},
};

const PRESETS: [RulesPreset; 2] = [RulesPreset::Classic, RulesPreset::Assisted];
const CHARACTERS: [Character; 3] = [Character::Explorer, Character::Damsel, Character::TunnelMan];

fn room(preset: RulesPreset, character: Character) -> Game {
    let mut game = Game::shell(1, preset);
    characters::apply(&mut game, character);
    game.world = World::new(30, 16, Area::Mines, 1);
    game.world.name = "rLevel".into();
    for x in 0..30 {
        game.world.set_tile(x, 8, "oHardBlock");
    }
    game.player.pos = Vec2::new(88.0, 120.0);
    game.player.previous_pos = game.player.pos;
    game.player.grounded = true;
    game.player.facing = -1;
    game.world.entrance = game.player.pos;
    game
}
fn healthy(game: &Game) -> i32 {
    if game.flags.contains("character_tunnelman") {
        2
    } else {
        4
    }
}
fn attack() -> InputFrame {
    InputFrame {
        attack: true,
        attack_pressed: true,
        ..Default::default()
    }
}
fn release() -> InputFrame {
    InputFrame {
        attack_released: true,
        ..Default::default()
    }
}
fn down_attack() -> InputFrame {
    InputFrame {
        down: true,
        ..attack()
    }
}
fn births(game: &Game, object: &str, first: u64) -> usize {
    game.world
        .entities
        .iter()
        .filter(|e| e.id >= first && e.object == object)
        .count()
}
fn keeper(game: &mut Game) -> u64 {
    let id = game.spawn("oShopkeeper", Vec2::new(144.0, 112.0));
    game.world
        .entity_mut(id)
        .unwrap()
        .text
        .insert("style".into(), "Weapon".into());
    id
}
fn fresh_teleporter_during_gun_cooldown(
    preset: RulesPreset,
    character: Character,
    priced: bool,
) -> (Game, u64, Option<u64>) {
    let mut game = room(preset, character);
    let keeper_id = if priced {
        Some(keeper(&mut game))
    } else {
        None
    };
    let pistol = game.spawn("oPistol", Vec2::new(88.0, 124.0));
    let teleporter = game.spawn("oTeleporter", Vec2::new(96.0, 124.0));
    if priced {
        let stock = game.world.entity_mut(teleporter).unwrap();
        stock.data.insert("cost".into(), 10000.0);
        stock.data.insert("forSale".into(), 1.0);
    }
    game.step(&InputFrame::default());
    game.step(&down_attack());
    assert_eq!(
        game.player.held,
        Some(pistol),
        "FIXTURE_TIMING: natural Pistol pickup"
    );
    game.step(&release());
    let first = game.world.next_id;
    game.step(&attack());
    assert_eq!(
        births(&game, "oBullet", first),
        1,
        "natural shot establishes firing"
    );
    game.step(&InputFrame {
        down: true,
        ..release()
    });
    game.step(&down_attack());
    assert!(game.player.held.is_none(), "natural specialized gun drop");
    for step in 0..6 {
        game.step(&InputFrame {
            right: true,
            attack_released: step == 0,
            ..Default::default()
        });
    }
    game.step(&InputFrame {
        right_released: true,
        ..down_attack()
    });
    assert_eq!(
        game.player.held,
        Some(teleporter),
        "FIXTURE_TIMING: natural Teleporter pickup"
    );
    game.step(&release());
    assert!(
        game.player.data["weapon_cooldown"] > 4.0,
        "FIXTURE_TIMING: remaining real gun cooldown"
    );
    assert_eq!(game.player.health, f64::from(healthy(&game)));
    assert_eq!(
        game.player.data.get("bow_armed").copied().unwrap_or(0.0),
        0.0
    );
    assert!(
        !game
            .events
            .iter()
            .any(|e| matches!(e, GameEvent::Sound { name, .. } if name == "sndTeleport")),
        "no earlier Teleporter activation"
    );
    if let Some(id) = keeper_id {
        assert_eq!(game.world.entity(id).unwrap().health, 20);
        assert_eq!(game.data.get("thiefLevel").copied().unwrap_or(0.0), 0.0);
        assert_eq!(
            game.world.entity(teleporter).unwrap().value("cost"),
            10000.0
        );
    }
    (game, teleporter, keeper_id)
}
fn assert_blocked_assignment_tail(game: &mut Game, teleporter: u64, target: Option<Vec2>) {
    let first = game.world.next_id;
    let money = game.player.money;
    let resources = (game.player.bombs, game.player.ropes);
    let mut expected = game.rng.clone();
    for _ in 0..6 {
        expected.range(9);
    }
    game.step(&attack());
    assert!(
        game.player.data["weapon_cooldown"] > 0.0,
        "FIXTURE_TIMING: assignment remains blocked"
    );
    assert_eq!(
        births(game, "oFlareSpark", first),
        3,
        "CONTRACT: defined scratch target runs complete Teleporter tail"
    );
    if let Some(target) = target {
        assert_eq!(
            game.player.pos, target,
            "CONTRACT: tail uses the other script's target without a distance draw"
        );
    }
    assert_eq!(
        (game.player.data["tx"], game.player.data["ty"]),
        (game.player.pos.x, game.player.pos.y)
    );
    assert_eq!(
        game.rng.clone().next_u64(),
        expected.next_u64(),
        "six spark coordinate draws only"
    );
    assert_eq!(game.player.held, Some(teleporter));
    assert_eq!(game.player.pickup_item_type, "Teleporter");
    assert_eq!(game.player.health, f64::from(healthy(&game)));
    assert_eq!(game.player.money, money);
    assert_eq!((game.player.bombs, game.player.ropes), resources);
    assert_eq!(
        game.events
            .iter()
            .filter(|e| matches!(e, GameEvent::Sound { name, .. } if name == "sndTeleport"))
            .count(),
        1
    );
}

#[test]
fn natural_pay_outside_shop_establishes_first_blocked_teleporter_target() {
    for preset in PRESETS {
        for character in CHARACTERS {
            let (mut game, teleporter, _) =
                fresh_teleporter_during_gun_cooldown(preset, character, false);
            game.step(&InputFrame {
                pay_pressed: true,
                ..Default::default()
            });
            let sampled = game.player.pos;
            for _ in 0..3 {
                game.step(&InputFrame {
                    right: true,
                    ..Default::default()
                });
            }
            assert!(
                game.player.pos.x > sampled.x,
                "FIXTURE_TIMING: movement makes persistent sampled target distinguishable"
            );
            assert_blocked_assignment_tail(&mut game, teleporter, Some(sampled));
        }
    }
}

#[test]
fn natural_priced_held_room_query_establishes_first_blocked_teleporter_target() {
    for preset in PRESETS {
        for character in CHARACTERS {
            let (mut game, teleporter, keeper_id) =
                fresh_teleporter_during_gun_cooldown(preset, character, true);
            for _ in 0..3 {
                game.step(&InputFrame {
                    right: true,
                    ..Default::default()
                });
            }
            // Held-cost check runs after movement and immediately before scrUseItem;
            // no previous activation or Pay is needed, and its target is current x/y.
            let before = game.player.pos;
            assert!(
                game.player.vel.x > 0.1,
                "FIXTURE_TIMING: inertia distinguishes this frame's held query from preceding frames"
            );
            assert_blocked_assignment_tail(&mut game, teleporter, None);
            assert!(
                game.player.pos.x > before.x,
                "CONTRACT: priced held query samples after this frame's movement, not its previous stored point"
            );
            assert_eq!(game.player.pos.y, before.y);
            assert_eq!(
                game.world.entity(teleporter).unwrap().value("cost"),
                10000.0
            );
            assert_eq!(game.world.entity(keeper_id.unwrap()).unwrap().health, 20);
            assert_eq!(game.data.get("thiefLevel").copied().unwrap_or(0.0), 0.0);
        }
    }
}

#[test]
fn natural_paid_teleporter_crossing_shop_boundary_uses_current_room_coordinates() {
    for preset in PRESETS {
        for character in CHARACTERS {
            let (mut game, teleporter, keeper_id) =
                fresh_teleporter_during_gun_cooldown(preset, character, true);
            let mut room_path = [[0; 4]; 5];
            room_path[0][0] = 4;
            game.world.source_room_path = Some(room_path);

            // Room zero ends at x=176. Start inside the shop with a prior
            // teleporter target there, then cross the boundary while using it.
            game.player.pos = Vec2::new(175.0, 120.0);
            game.player.previous_pos = game.player.pos;
            game.player.vel = Vec2::new(2.0, 0.0);
            game.player.state = PlayerState::Standing;
            game.player.grounded = true;
            game.player.facing = 1;
            game.player.data.insert("tx".into(), 160.0);
            game.player.data.insert("ty".into(), 120.0);
            assert!(game.player.data["weapon_cooldown"] > 1.0);
            assert_eq!(
                game.world.entity(teleporter).unwrap().value("cost"),
                10000.0
            );

            game.step(&InputFrame {
                right: true,
                ..attack()
            });

            assert!(
                game.player.pos.x >= 176.0,
                "the full step crosses out of the shop: {:?} {:?}",
                game.player.pos,
                game.player.state
            );
            assert!(game.player.data["weapon_cooldown"] > 0.0);
            let item = game.world.entity(teleporter).unwrap();
            assert_eq!(
                item.value("cost"),
                0.0,
                "the held-item source check steals it"
            );
            assert_eq!(item.value("forSale"), 0.0);
            assert_eq!(game.player.held, Some(teleporter));
            assert_eq!(game.player.pos.x, game.player.data["tx"]);
            assert_eq!(game.player.pos.y, game.player.data["ty"]);
            assert!(
                game.events.iter().any(
                    |event| matches!(event, GameEvent::Sound { name, .. } if name == "sndTeleport")
                ),
                "the cooldown fallback still uses the just-sampled outside position"
            );
            assert_eq!(game.data.get("thiefLevel"), Some(&2.0));
            assert_eq!(game.world.entity(keeper_id.unwrap()).unwrap().health, 20);
        }
    }
}

#[test]
fn natural_unblocked_teleporter_replaces_pay_scratch_with_distance_target() {
    for preset in PRESETS {
        for character in CHARACTERS {
            let mut game = room(preset, character);
            let teleporter = game.spawn("oTeleporter", Vec2::new(88.0, 124.0));
            game.step(&InputFrame::default());
            game.step(&down_attack());
            assert_eq!(game.player.held, Some(teleporter));
            game.step(&release());
            game.step(&InputFrame {
                pay_pressed: true,
                ..Default::default()
            });
            let origin = game.player.pos;
            let mut expected = game.rng.clone();
            let distance = 16.0 * f64::from(4 + expected.range(5));
            for _ in 0..6 {
                expected.range(9);
            }
            let first = game.world.next_id;
            game.step(&attack());
            let target = Vec2::new((origin.x - distance).max(8.0), origin.y);
            assert_eq!(game.player.pos, target);
            assert_eq!(births(&game, "oFlareSpark", first), 3);
            assert_eq!(game.rng.clone().next_u64(), expected.next_u64());
            assert_eq!(game.player.held, Some(teleporter));
            assert_eq!(game.player.health, f64::from(healthy(&game)));
        }
    }
}

#[test]
fn natural_bomb_arrow_shop_prefix_updates_scratch_and_angers_only_local_keeper() {
    for preset in PRESETS {
        for shop in [false, true] {
            let mut game = room(preset, Character::Explorer);
            // Keep the prefix-query fixture protected during full charging:
            // FOLLOW otherwise brings the owner into lethal shotgun range.
            // This initial source-solid wall blocks both following and shots.
            for y in 6..=7 {
                game.world.set_tile(7, y, "oHardBlock");
            }
            let keeper_id = if shop { Some(keeper(&mut game)) } else { None };
            let bow = game.spawn("oBow", Vec2::new(88.0, 124.0));
            game.step(&InputFrame::default());
            game.step(&down_attack());
            assert_eq!(game.player.held, Some(bow));
            game.step(&release());
            game.step(&attack());
            assert_eq!(game.player.data["bow_armed"], 1.0);
            // Source's late charge gate replaces Bomb with Normal below 12.
            // Reach full charge through ordinary frames before arming Bomb.
            for _ in 0..64 {
                if game.player.data["bow_strength"] >= 12.0 {
                    break;
                }
                game.step(&InputFrame {
                    attack: true,
                    ..Default::default()
                });
            }
            assert!(game.player.data["bow_strength"] >= 12.0);
            let resources = (game.player.bombs, game.player.ropes);
            game.step(&InputFrame {
                attack: true,
                bomb_pressed: true,
                ..Default::default()
            });
            assert_eq!(game.player.data["bow_bomb_armed"], 1.0);
            assert_eq!(game.player.data["bow_bomb_fuse"], 80.0);
            assert_eq!(game.data.get("thiefLevel").copied().unwrap_or(0.0), 0.0);
            let early_origin = game.player.pos;
            game.step(&InputFrame {
                attack: true,
                ..Default::default()
            });
            assert_eq!(game.player.data["bow_bomb_fuse"], 79.0);
            assert_eq!(
                (
                    game.player.data.get("tx").copied(),
                    game.player.data.get("ty").copied()
                ),
                (Some(early_origin.x), Some(early_origin.y)),
                "CONTRACT: early bomb-arrow script query writes caller scratch"
            );
            assert_eq!((game.player.bombs, game.player.ropes), resources);
            assert_eq!(game.player.held, Some(bow));
            assert_eq!(game.player.health, f64::from(healthy(&game)));
            if let Some(id) = keeper_id {
                assert_eq!(game.data["thiefLevel"], 2.0);
                assert_eq!(game.world.entity(id).unwrap().health, 20);
                assert!(game.events.iter().any(|event| matches!(event, GameEvent::Dialogue(message) if message.first == "TERRORIST!" && message.second.is_empty() && message.ticks == 80)));
            } else {
                assert_eq!(game.data.get("thiefLevel").copied().unwrap_or(0.0), 0.0);
                assert!(!game.events.iter().any(|event| matches!(event, GameEvent::Dialogue(message) if message.first == "TERRORIST!")));
            }
        }
    }
}

#[test]
fn natural_pay_overwrites_an_earlier_teleporter_target_before_cooldown_reuse() {
    for preset in PRESETS {
        for character in CHARACTERS {
            let (mut game, teleporter, _) =
                fresh_teleporter_during_gun_cooldown(preset, character, false);
            game.step(&InputFrame {
                up: true,
                up_pressed: true,
                ..attack()
            });
            let earlier = game.player.pos;
            assert_eq!(
                (game.player.data["tx"], game.player.data["ty"]),
                (earlier.x, earlier.y)
            );
            game.step(&release());
            for _ in 0..3 {
                game.step(&InputFrame {
                    right: true,
                    ..Default::default()
                });
            }
            game.step(&InputFrame {
                pay_pressed: true,
                right_released: true,
                ..Default::default()
            });
            let sampled = game.player.pos;
            assert_ne!(
                sampled, earlier,
                "FIXTURE_TIMING: a real intervening move distinguishes old target"
            );
            assert_blocked_assignment_tail(&mut game, teleporter, Some(sampled));
        }
    }
}
