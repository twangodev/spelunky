//! Ordinary pickup/cycle/charge inputs exercise previous-sprite balance release.
//! Actors, geometry, character and inventory are authored only before stepping.
use spelunky_core::{
    Area, Game, GameEvent, InputFrame, PlayerState, RulesPreset, Vec2, World,
    characters::{self, Character},
};
const PRESETS: [RulesPreset; 2] = [RulesPreset::Classic, RulesPreset::Assisted];
const CHARACTERS: [Character; 3] = [Character::Explorer, Character::Damsel, Character::TunnelMan];
fn scene(preset: RulesPreset, character: Character, facing: i8, edge: bool) -> Game {
    let mut game = Game::shell(11, preset);
    game.world = World::new(42, 20, Area::Mines, 1);
    game.world.name = "rLevel".into();
    for x in 20..42 {
        game.world.set_tile(x, 10, "oHardBlock");
    }
    characters::apply(&mut game, character);
    game.player.pos = Vec2::new(if edge { 321.0 } else { 344.0 }, 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.grounded = true;
    game.player.state = PlayerState::Standing;
    game.player.facing = facing;
    // Tunnel Man's authored resources are valid initial inventory, permitting
    // the same reserved-item controls as the other selected characters.
    game.player.bombs = 4.0;
    game.player.ropes = 4.0;
    game.world.entrance = game.player.pos;
    game
}
fn whoa(game: &Game) -> bool {
    matches!(
        game.player.sprite.as_str(),
        "sWhoaLeft" | "sDamselWhoaL" | "sTunnelWhoaL"
    )
}
fn pickup(game: &mut Game, object: &str) -> u64 {
    let pos = if object == "oDamsel" {
        Vec2::new(game.player.pos.x, 152.0)
    } else {
        Vec2::new(game.player.pos.x, 154.0)
    };
    let id = game.spawn(object, pos);
    game.step(&InputFrame {
        down: true,
        attack_pressed: true,
        attack: true,
        ..Default::default()
    });
    assert_eq!(
        game.player.held,
        Some(id),
        "ordinary Down+Attack chooses authored actor"
    );
    assert!(game.world.entity(id).unwrap().held);
    assert_eq!(game.player.data["whoaTimer"], 30.0);
    game.step(&InputFrame::default());
    assert_eq!(game.player.held, Some(id));
    assert_eq!(
        game.player.data["whoaTimer"], 30.0,
        "previous Duck sprite resets the maximum"
    );
    id
}
fn count_down(game: &mut Game, input: InputFrame, held_id: Option<u64>) -> Vec<f64> {
    assert!(
        whoa(game),
        "natural Standing at left floor edge selects WHOA sprite"
    );
    assert_eq!(game.player.data["whoaTimer"], 30.0);
    let mut trace = Vec::new();
    for _ in 0..30 {
        assert!(whoa(game));
        game.step(&input);
        trace.push(game.player.data["whoaTimer"]);
        assert_eq!(
            game.player.held, held_id,
            "a positive-entry timer decrements without releasing"
        );
    }
    trace
}
fn assert_timer_trace(trace: &[f64]) {
    let expected: Vec<f64> = (0..30).rev().map(f64::from).collect();
    assert_eq!(
        trace, expected,
        "all thirty source timer observations are retained after the primary consequence"
    );
}
fn sound(game: &Game, name: &str) -> usize {
    game.events
        .iter()
        .filter(|event| matches!(event, GameEvent::Sound { name: actual, .. } if actual == name))
        .count()
}
#[test]
fn natural_rock_pickup_then_31_previous_whoa_steps_release_same_actor() {
    for preset in PRESETS {
        for character in CHARACTERS {
            for facing in [-1, 1] {
                let mut game = scene(preset, character, facing, true);
                let id = pickup(&mut game, "oRock");
                assert_eq!(game.player.pickup_item_type, "Rock");
                let trace = count_down(&mut game, InputFrame::default(), Some(id));
                let next_id = game.world.next_id;
                game.step(&InputFrame::default());
                assert!(game.player.held.is_none());
                assert!(game.player.pickup_item_type.is_empty());
                assert!(
                    !game.world.entity(id).unwrap().held,
                    "original carried actor is released"
                );
                assert_eq!(
                    game.world.next_id, next_id,
                    "matching type does not restore a fresh item"
                );
                assert_eq!(
                    sound(&game, "sndThrow"),
                    0,
                    "balance release has no generic throw sound"
                );
                assert_timer_trace(&trace);
                assert_eq!(game.player.data["whoaTimer"], 0.0);
            }
        }
    }
}
#[test]
fn natural_charging_bow_fires_before_same_actor_balance_release() {
    for preset in PRESETS {
        for character in CHARACTERS {
            for facing in [-1, 1] {
                let mut game = scene(preset, character, facing, true);
                let bow = pickup(&mut game, "oBow");
                assert_eq!(game.data["arrows"], 6.0);
                game.step(&InputFrame {
                    attack_pressed: true,
                    attack: true,
                    ..Default::default()
                });
                assert_eq!(game.player.data["bow_armed"], 1.0);
                let mut trace = vec![game.player.data["whoaTimer"]];
                for _ in 0..29 {
                    game.step(&InputFrame {
                        attack: true,
                        ..Default::default()
                    });
                    assert!(whoa(&game));
                    trace.push(game.player.data["whoaTimer"]);
                    assert_eq!(game.player.held, Some(bow));
                    assert_eq!(game.data["arrows"], 6.0);
                }
                assert!(
                    game.player.data["bow_strength"] > 0.0,
                    "charging is produced by held Attack"
                );
                game.step(&InputFrame {
                    attack: true,
                    ..Default::default()
                });
                assert!(game.player.held.is_none());
                assert!(game.player.pickup_item_type.is_empty());
                assert!(!game.world.entity(bow).unwrap().held);
                assert_eq!(game.data["arrows"], 5.0);
                assert_timer_trace(&trace);
                assert_eq!(game.player.data["bow_armed"], 0.0);
                assert_eq!(game.player.data["bow_strength"], 0.0);
                assert_eq!(
                    game.player.data["weapon_cooldown"], 9.0,
                    "WHOA fire writes10 before early FIRING decrements to9"
                );
                assert_eq!(sound(&game, "sndArrowTrap"), 1);
                assert_eq!(sound(&game, "sndThrow"), 0);
                assert_eq!(
                    game.world
                        .entities
                        .iter()
                        .filter(|e| e.alive && e.object == "oArrow")
                        .count(),
                    1
                );
            }
        }
    }
}
#[test]
fn natural_reserved_bomb_and_rope_balance_release_restore_fresh_remembered_rock() {
    for preset in PRESETS {
        for character in CHARACTERS {
            for facing in [-1, 1] {
                for rope in [false, true] {
                    let mut game = scene(preset, character, facing, true);
                    let original = pickup(&mut game, "oRock");
                    game.step(&InputFrame {
                        cycle_pressed: true,
                        ..Default::default()
                    });
                    assert!(
                        game.world.entity(original).is_none(),
                        "cycle destroys the original Rock"
                    );
                    if rope {
                        game.step(&InputFrame::default());
                        game.step(&InputFrame {
                            cycle_pressed: true,
                            ..Default::default()
                        });
                    }
                    let reserved = game.player.held.unwrap();
                    assert_eq!(
                        game.world.entity(reserved).unwrap().object,
                        if rope { "oRopeThrow" } else { "oBomb" }
                    );
                    assert_eq!(game.player.pickup_item_type, "Rock");
                    let resources = (game.player.bombs, game.player.ropes);
                    let trace = count_down(&mut game, InputFrame::default(), Some(reserved));
                    let first_fresh = game.world.next_id;
                    game.step(&InputFrame::default());
                    assert!(!game.world.entity(reserved).unwrap().held);
                    let restored = game.player.held.unwrap();
                    assert_eq!(
                        restored, first_fresh,
                        "scrHoldItem runs synchronous fresh Rock Create"
                    );
                    assert_ne!(restored, original);
                    assert_ne!(restored, reserved);
                    assert_eq!(game.world.entity(restored).unwrap().object, "oRock");
                    assert!(game.world.entity(restored).unwrap().held);
                    assert_eq!(game.world.entity(restored).unwrap().value("cost"), 0.0);
                    assert_eq!(game.world.entity(restored).unwrap().value("new"), 0.0);
                    assert_eq!(game.player.pickup_item_type, "Rock");
                    assert_timer_trace(&trace);
                    assert_eq!(game.player.data["whoaTimer"], 30.0);
                    assert_eq!(
                        (game.player.bombs, game.player.ropes),
                        resources,
                        "release does not refund or charge inventory"
                    );
                    assert_eq!(sound(&game, "sndThrow"), 0);
                }
            }
        }
    }
}
#[test]
fn natural_damsel_balance_release_adds_its_source_sound() {
    for preset in PRESETS {
        for character in CHARACTERS {
            for facing in [-1, 1] {
                let mut game = scene(preset, character, facing, true);
                let damsel = pickup(&mut game, "oDamsel");
                assert_eq!(game.player.pickup_item_type, "Damsel");
                let trace = count_down(&mut game, InputFrame::default(), Some(damsel));
                game.step(&InputFrame::default());
                assert!(game.player.held.is_none());
                assert!(game.player.pickup_item_type.is_empty());
                assert!(!game.world.entity(damsel).unwrap().held);
                assert_eq!(sound(&game, "sndDamsel"), 1);
                assert_timer_trace(&trace);
                assert_eq!(sound(&game, "sndThrow"), 0);
            }
        }
    }
}
#[test]
fn empty_hand_and_uninterrupted_floor_are_independent_balance_controls() {
    for preset in PRESETS {
        for character in CHARACTERS {
            for facing in [-1, 1] {
                let mut empty = scene(preset, character, facing, true);
                empty.step(&InputFrame::default());
                let empty_trace = count_down(&mut empty, InputFrame::default(), None);
                let next = empty.world.next_id;
                empty.step(&InputFrame::default());
                assert!(empty.player.held.is_none());
                assert_timer_trace(&empty_trace);
                assert_eq!(empty.player.data["whoaTimer"], 0.0);
                assert_eq!(empty.world.next_id, next);
                let mut floor = scene(preset, character, facing, false);
                let id = pickup(&mut floor, "oRock");
                for _ in 0..45 {
                    assert!(!whoa(&floor));
                    floor.step(&InputFrame::default());
                    assert_eq!(floor.player.held, Some(id));
                    assert_eq!(floor.player.data["whoaTimer"], 30.0);
                }
            }
        }
    }
}

#[test]
fn native_player_create_exposes_source_balance_timer_defaults() {
    let game = Game::shell(11, RulesPreset::Classic);
    assert_eq!(game.player.data.get("whoaTimer"), Some(&0.0));
    assert_eq!(game.player.data.get("whoaTimerMax"), Some(&30.0));
}
