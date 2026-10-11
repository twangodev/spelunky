//! Regressions driven by initial rooms, default actors, and ordinary player input.
use spelunky_core::{
    Area, Game, InputFrame, RulesPreset, Vec2, World,
    characters::{self, Character},
};

const PRESETS: [RulesPreset; 2] = [RulesPreset::Classic, RulesPreset::Assisted];
const CHARACTERS: [Character; 3] = [Character::Explorer, Character::Damsel, Character::TunnelMan];

fn room(preset: RulesPreset, character: Character, facing: i8) -> Game {
    let mut game = Game::shell(1, preset);
    characters::apply(&mut game, character);
    game.world = World::new(24, 16, Area::Mines, 1);
    game.world.name = "rLevel".into();
    for x in 0..24 {
        game.world.set_tile(x, 8, "oHardBlock");
    }
    game.player.pos = Vec2::new(88.0, 120.0);
    game.player.previous_pos = game.player.pos;
    game.player.grounded = true;
    game.player.facing = facing;
    game
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

fn pickup_during_whip(game: &mut Game, object: &str) -> u64 {
    let id = game.spawn(object, Vec2::new(88.0, 124.0));
    game.step(&InputFrame::default());
    game.step(&attack());
    assert!(
        game.player.attack_timer > 0,
        "real empty attack onset required"
    );
    game.step(&release());
    game.step(&down_attack());
    assert_eq!(
        game.player.held,
        Some(id),
        "source pickup must remain eligible"
    );
    assert!(game.player.attack_timer > 0);
    game.step(&release());
    assert_eq!(game.player.held, Some(id));
    assert!(
        game.player.attack_timer > 0,
        "holding must not cancel whipping"
    );
    id
}

fn births(game: &Game, object: &str, first_id: u64) -> usize {
    game.world
        .entities
        .iter()
        .filter(|e| e.id >= first_id && e.object == object)
        .count()
}

#[test]
fn natural_whip_pickup_then_repress_throws_generic_item_without_restarting_whip() {
    for preset in PRESETS {
        for character in CHARACTERS {
            for facing in [-1, 1] {
                for object in ["oRock", "oLampItem"] {
                    let mut game = room(preset, character, facing);
                    let id = pickup_during_whip(&mut game, object);
                    let phase = game.player.animation;
                    let sprite = game.player.sprite.clone();
                    let resources = (game.player.bombs, game.player.ropes);
                    let health = game.player.health;
                    game.step(&attack());
                    assert!(game.player.held.is_none());
                    assert!(game.player.pickup_item_type.is_empty());
                    let thrown = game.world.entity(id).unwrap();
                    assert!(!thrown.held);
                    assert!(thrown.vel.x * f64::from(facing) > 0.0);
                    assert!(thrown.vel.y < 0.0);
                    assert_eq!(thrown.value("safe"), 1.0);
                    assert!(game.player.attack_timer > 0);
                    assert_eq!(game.player.sprite, sprite);
                    assert!(game.player.animation > phase);
                    assert_eq!((game.player.bombs, game.player.ropes), resources);
                    assert_eq!(game.player.health, health);
                }
            }
        }
    }
}

#[test]
fn newly_picked_melee_cannot_restart_or_drop_during_existing_whip() {
    for preset in PRESETS {
        for character in CHARACTERS {
            for object in ["oMachete", "oMattock"] {
                for down in [false, true] {
                    let mut game = room(preset, character, 1);
                    let id = pickup_during_whip(&mut game, object);
                    let phase = game.player.animation;
                    game.step(&InputFrame { down, ..attack() });
                    assert_eq!(game.player.held, Some(id));
                    assert!(game.world.entity(id).unwrap().held);
                    assert!(game.player.attack_timer > 0);
                    assert!(game.player.animation > phase, "no new swing/restart");
                }
            }
        }
    }
}

#[test]
fn newly_picked_ranged_fires_with_attack_art_and_down_is_not_a_drop() {
    for preset in PRESETS {
        for character in CHARACTERS {
            for (object, projectile, count) in [
                ("oPistol", "oBullet", 1),
                ("oShotgun", "oBullet", 6),
                ("oWebCannon", "oWebBall", 1),
                ("oSceptre", "oPsychicWaveP", 1),
            ] {
                for down in [false, true] {
                    let mut game = room(preset, character, 1);
                    let id = pickup_during_whip(&mut game, object);
                    let first = game.world.next_id;
                    let phase = game.player.animation;
                    game.step(&InputFrame { down, ..attack() });
                    assert_eq!(game.player.held, Some(id));
                    assert_eq!(births(&game, projectile, first), count);
                    assert!(game.player.attack_timer > 0);
                    assert!(game.player.animation > phase);
                    game.step(&release());
                    assert!(game.player.data["weapon_cooldown"] > 0.0);
                    let second = game.world.next_id;
                    game.step(&attack());
                    assert_eq!(
                        births(&game, projectile, second),
                        0,
                        "real firing cooldown still guards ranged use"
                    );
                    assert_eq!(game.player.held, Some(id));
                }
            }
        }
    }
}

#[test]
fn newly_picked_bow_can_draw_and_release_during_whip_but_down_is_noop() {
    for preset in PRESETS {
        for character in CHARACTERS {
            for down in [false, true] {
                let mut game = room(preset, character, 1);
                let id = pickup_during_whip(&mut game, "oBow");
                let arrows = game.data["arrows"];
                assert_eq!(arrows, 6.0, "natural first Bow pickup grants ammo");
                let first = game.world.next_id;
                game.step(&InputFrame { down, ..attack() });
                assert_eq!(game.player.held, Some(id));
                assert!(game.player.attack_timer > 0);
                assert_eq!(
                    game.player.data.get("bow_armed").copied().unwrap_or(0.0),
                    f64::from(!down)
                );
                game.step(&release());
                assert_eq!(births(&game, "oArrow", first), usize::from(!down));
                assert_eq!(game.data["arrows"], arrows - f64::from(!down));
                assert_eq!(game.player.held, Some(id));
                assert!(game.player.attack_timer > 0);
            }
        }
    }
}

#[test]
fn newly_picked_teleporter_can_use_during_whip_but_down_is_noop() {
    for preset in PRESETS {
        for character in CHARACTERS {
            for down in [false, true] {
                let mut game = room(preset, character, 1);
                let id = pickup_during_whip(&mut game, "oTeleporter");
                let pos = game.player.pos;
                let first = game.world.next_id;
                game.step(&InputFrame { down, ..attack() });
                assert_eq!(game.player.held, Some(id));
                assert_eq!(
                    births(&game, "oFlareSpark", first),
                    if down { 0 } else { 3 }
                );
                assert_eq!(game.player.pos == pos, down);
                assert!(game.player.attack_timer > 0);
                if !down {
                    assert_eq!(game.player.data["tx"], game.player.pos.x);
                    assert_eq!(game.player.data["ty"], game.player.pos.y);
                }
            }
        }
    }
}

#[test]
fn naturally_picked_bomb_arms_then_throws_same_actor_during_whip() {
    for preset in PRESETS {
        for character in CHARACTERS {
            let mut game = room(preset, character, 1);
            let resources = (game.player.bombs, game.player.ropes);
            let id = pickup_during_whip(&mut game, "oBomb");
            assert_eq!(game.world.entity(id).unwrap().sprite, "sBomb");
            game.step(&attack());
            assert_eq!(game.player.held, Some(id));
            assert_eq!(game.world.entity(id).unwrap().sprite, "sBombArmed");
            assert!(game.world.entity(id).unwrap().value("armed") != 0.0);
            assert!(game.player.attack_timer > 0);
            game.step(&release());
            game.step(&attack());
            assert!(game.player.held.is_none());
            assert!(game.player.pickup_item_type.is_empty());
            let bomb = game.world.entity(id).unwrap();
            assert!(!bomb.held);
            assert!(bomb.vel.x > 0.0 && bomb.vel.y < 0.0);
            assert!(game.player.attack_timer > 0);
            assert_eq!((game.player.bombs, game.player.ropes), resources);
        }
    }
}

#[test]
fn naturally_picked_rope_uses_same_upward_actor_or_fresh_downward_attempt_during_whip() {
    for preset in PRESETS {
        for character in CHARACTERS {
            for down in [false, true] {
                let mut game = room(preset, character, 1);
                let resources = (game.player.bombs, game.player.ropes);
                let id = pickup_during_whip(&mut game, "oRopeThrow");
                let first = game.world.next_id;
                game.step(&InputFrame { down, ..attack() });
                assert!(game.player.held.is_none());
                assert!(game.player.pickup_item_type.is_empty());
                assert!(game.player.attack_timer > 0);
                if down {
                    // Flat floor blocks both attempted side strips. Source
                    // retires the old held actor and attempt, then keeps fallback.
                    assert!(game.world.entity(id).is_none());
                    assert_eq!(game.world.next_id, first + 2);
                    let fallback = game
                        .world
                        .entities
                        .iter()
                        .find(|e| e.alive && e.id >= first && e.object == "oRopeThrow")
                        .unwrap();
                    assert!(!fallback.held);
                    assert!(fallback.vel.x > 0.0);
                } else {
                    let launched = game.world.entity(id).unwrap();
                    assert!(!launched.held);
                    assert!(launched.vel.y < 0.0);
                    assert_eq!(births(&game, "oRopeThrow", first), 0);
                }
                assert_eq!((game.player.bombs, game.player.ropes), resources);
            }
        }
    }
}

#[test]
fn ineligible_inventory_edges_do_not_consume_held_throw_while_whipping() {
    for preset in PRESETS {
        for character in CHARACTERS {
            let mut game = room(preset, character, 1);
            let id = pickup_during_whip(&mut game, "oRock");
            let resources = (game.player.bombs, game.player.ropes);
            let first = game.world.next_id;
            game.step(&InputFrame {
                cycle_pressed: true,
                rope_pressed: true,
                bomb_pressed: true,
                ..attack()
            });
            assert!(game.player.held.is_none());
            assert!(!game.world.entity(id).unwrap().held);
            assert!(game.player.attack_timer > 0);
            assert_eq!((game.player.bombs, game.player.ropes), resources);
            assert_eq!(births(&game, "oBomb", first), 0);
            assert_eq!(births(&game, "oRopeThrow", first), 0);
        }
    }
}

#[test]
fn empty_hand_airborne_down_attack_tests_attack_before_pickup() {
    for preset in PRESETS {
        for character in CHARACTERS {
            let mut game = room(preset, character, 1);
            game.spawn("oRock", Vec2::new(88.0, 112.0));
            game.step(&InputFrame::default());
            game.step(&InputFrame {
                jump: true,
                jump_pressed: true,
                ..Default::default()
            });
            assert!(
                !game.player.grounded,
                "real Jump must precede airborne action"
            );
            assert!(game.player.held.is_none());
            game.step(&InputFrame {
                jump_released: true,
                ..down_attack()
            });
            assert!(game.player.held.is_none());
            if character == Character::TunnelMan {
                // Source enters the empty-attack branch but its inner grounded
                // test fails; it does not fall through to the pickup else-if.
                assert_eq!(game.player.attack_timer, 0);
            } else {
                assert!(game.player.attack_timer > 0);
            }
        }
    }
}

#[test]
fn natural_specialized_down_drop_keeps_source_release_without_generic_throw_effects() {
    for preset in PRESETS {
        for character in CHARACTERS {
            for object in [
                "oMachete",
                "oMattock",
                "oPistol",
                "oShotgun",
                "oWebCannon",
                "oSceptre",
                "oBow",
                "oTeleporter",
            ] {
                let mut game = room(preset, character, 1);
                let id = game.spawn(object, Vec2::new(88.0, 124.0));
                game.step(&InputFrame::default());
                game.step(&down_attack());
                assert_eq!(game.player.held, Some(id));
                game.step(&InputFrame {
                    down: true,
                    ..release()
                });
                let armed = game.world.entity(id).unwrap().value("armed");
                game.step(&down_attack());
                assert!(game.player.held.is_none());
                assert!(game.player.pickup_item_type.is_empty());
                let item = game.world.entity(id).unwrap();
                assert!(!item.held);
                assert_eq!(item.value("safe"), 1.0);
                assert_eq!(item.value("armed"), armed, "specialized drop does not arm");
                assert!(!game.events.iter().any(|event| {
                    matches!(event, spelunky_core::GameEvent::Sound { name, .. } if name == "sndThrow")
                }), "specialized drop has no sndThrow");
                assert_eq!(game.player.attack_timer, 0);
            }
        }
    }
}

#[test]
fn natural_gun_firing_cooldown_does_not_block_later_melee_onset() {
    for preset in PRESETS {
        for character in CHARACTERS {
            for melee in ["oMachete", "oMattock"] {
                let mut game = room(preset, character, -1);
                let pistol = game.spawn("oPistol", Vec2::new(88.0, 124.0));
                let tool = game.spawn(melee, Vec2::new(96.0, 124.0));
                game.step(&InputFrame::default());
                game.step(&down_attack());
                assert_eq!(game.player.held, Some(pistol));
                game.step(&release());
                let first = game.world.next_id;
                game.step(&attack());
                assert_eq!(
                    births(&game, "oBullet", first),
                    1,
                    "actual gun fire required"
                );
                game.step(&InputFrame {
                    down: true,
                    ..release()
                });
                game.step(&down_attack());
                assert!(game.player.held.is_none());
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
                    Some(tool),
                    "FIXTURE_TIMING: real nearby melee pickup required"
                );
                game.step(&release());
                assert!(
                    game.player.data["weapon_cooldown"] > 0.0,
                    "real gun cooldown remains"
                );
                assert_eq!(game.player.attack_timer, 0);
                game.step(&attack());
                assert_eq!(game.player.held, Some(tool));
                assert!(
                    game.player.attack_timer > 0,
                    "melee has no firing cooldown guard"
                );
            }
        }
    }
}

#[test]
fn natural_teleporter_up_ignores_gun_cooldown_and_horizontal_reuses_established_target() {
    for preset in PRESETS {
        for character in CHARACTERS {
            let mut game = room(preset, character, -1);
            let pistol = game.spawn("oPistol", Vec2::new(88.0, 124.0));
            let teleporter = game.spawn("oTeleporter", Vec2::new(96.0, 124.0));
            game.step(&InputFrame::default());
            game.step(&down_attack());
            assert_eq!(game.player.held, Some(pistol));
            game.step(&release());
            let first = game.world.next_id;
            game.step(&attack());
            assert_eq!(births(&game, "oBullet", first), 1);
            game.step(&InputFrame {
                down: true,
                ..release()
            });
            game.step(&down_attack());
            assert!(game.player.held.is_none());
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
                "FIXTURE_TIMING: natural Teleporter pickup required"
            );
            game.step(&release());
            assert!(game.player.data["weapon_cooldown"] > 0.0);
            let origin = game.player.pos;
            game.step(&InputFrame {
                up: true,
                up_pressed: true,
                ..attack()
            });
            assert!(
                game.player.pos.y < origin.y,
                "Up target assignment ignores firing"
            );
            let target = game.player.pos;
            game.step(&InputFrame {
                up: true,
                ..release()
            });
            assert!(game.player.data["weapon_cooldown"] > 0.0);
            let first = game.world.next_id;
            let mut expected = game.rng.clone();
            for _ in 0..6 {
                expected.range(9);
            }
            game.step(&attack());
            assert_eq!(game.player.pos, target, "source tail reuses assigned tx/ty");
            assert_eq!(births(&game, "oFlareSpark", first), 3);
            assert_eq!(
                game.rng.clone().next_u64(),
                expected.next_u64(),
                "no target-distance draw when assignment is skipped"
            );
            assert_eq!(game.player.held, Some(teleporter));
        }
    }
}

#[test]
fn empty_bow_message_remains_eligible_during_real_firing_cooldown() {
    for preset in PRESETS {
        for character in CHARACTERS {
            let mut game = room(preset, character, 1);
            let bow = game.spawn("oBow", Vec2::new(88.0, 124.0));
            game.step(&InputFrame::default());
            game.step(&down_attack());
            assert_eq!(game.player.held, Some(bow));
            game.step(&release());
            assert_eq!(game.data["arrows"], 6.0);
            for shot in 0..6 {
                for _ in 0..12 {
                    if game
                        .player
                        .data
                        .get("weapon_cooldown")
                        .copied()
                        .unwrap_or(0.0)
                        == 0.0
                    {
                        break;
                    }
                    game.step(&InputFrame::default());
                }
                assert_eq!(
                    game.player
                        .data
                        .get("weapon_cooldown")
                        .copied()
                        .unwrap_or(0.0),
                    0.0
                );
                game.step(&attack());
                assert_eq!(game.player.data["bow_armed"], 1.0);
                game.step(&release());
                assert_eq!(game.data["arrows"], 5.0 - f64::from(shot));
            }
            assert!(game.player.data["weapon_cooldown"] > 0.0);
            game.step(&attack());
            assert_eq!(game.player.held, Some(bow));
            assert!(game.events.iter().any(|event| {
                matches!(event, spelunky_core::GameEvent::Message(text) if text == "I'M OUT OF ARROWS!")
            }));
            game.step(&release());
            game.step(&down_attack());
            assert!(game.player.held.is_none());
            assert!(!game.events.iter().any(|event| {
                matches!(event, spelunky_core::GameEvent::Message(text) if text == "I'M OUT OF ARROWS!")
            }), "Down drop is the preceding branch, not the empty-ammo message");
        }
    }
}
