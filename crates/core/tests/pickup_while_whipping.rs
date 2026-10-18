//! Regressions driven by initial rooms, default actors, and ordinary player input.
use spelunky_core::{
    Area, Game, InputFrame, PlayerState, RulesPreset, RunState, Vec2, World,
    characters::{self, Character},
};

const PRESETS: [RulesPreset; 2] = [RulesPreset::Classic, RulesPreset::Assisted];
const CHARACTERS: [Character; 2] = [Character::Explorer, Character::Damsel];

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

fn start_then_release(game: &mut Game) {
    game.step(&attack());
    assert!(game.player.attack_timer > 0, "real attack onset required");
    assert!(game.player.held.is_none());
    assert!(!game.player.is_stunned());
    game.step(&release());
    assert!(
        game.player.attack_timer > 0,
        "release must not end whipping"
    );
    assert!(game.player.animation > 0.0 && game.player.animation < 4.0);
}

#[test]
fn down_attack_repress_picks_up_while_whipping_without_restarting_attack() {
    for preset in PRESETS {
        for character in CHARACTERS {
            for facing in [-1, 1] {
                let mut game = room(preset, character, facing);
                let rock = game.spawn("oRock", Vec2::new(88.0, 124.0));
                game.step(&InputFrame::default());
                start_then_release(&mut game);
                let timer = game.player.attack_timer;
                let phase = game.player.animation;
                let sprite = game.player.sprite.clone();
                let next_random = game.rng.clone().next_u64();

                game.step(&down_attack());

                assert_eq!(game.player.held, Some(rock));
                assert!(game.world.entity(rock).unwrap().held);
                assert_eq!(game.player.pickup_item_type, "Rock");
                assert_eq!(game.player.attack_timer, timer);
                assert_eq!(game.player.sprite, sprite);
                assert!(game.player.animation > phase);
                assert_eq!(game.player.state, PlayerState::Standing);
                assert_eq!(game.rng.clone().next_u64(), next_random);
                assert_eq!(game.player.health, 4.0);
                assert_eq!((game.player.bombs, game.player.ropes), (4.0, 4.0));

                for step in 0..16 {
                    if game.player.attack_timer == 0 {
                        break;
                    }
                    game.step(&if step == 0 {
                        release()
                    } else {
                        InputFrame::default()
                    });
                    assert_eq!(game.player.held, Some(rock));
                }
                assert_eq!(game.player.attack_timer, 0, "real Animation End required");
                game.step(&InputFrame {
                    down: true,
                    ..Default::default()
                });
                assert_eq!(game.player.state, PlayerState::Ducking);
                assert_eq!(game.player.held, Some(rock));
                assert_eq!(game.state, RunState::Playing);
            }
        }
    }
}

#[test]
fn down_without_a_new_attack_edge_does_not_pick_up_during_whipping() {
    for preset in PRESETS {
        for character in CHARACTERS {
            for facing in [-1, 1] {
                let mut game = room(preset, character, facing);
                let rock = game.spawn("oRock", Vec2::new(88.0, 124.0));
                game.step(&InputFrame::default());
                game.step(&attack());
                assert!(game.player.attack_timer > 0);
                game.step(&InputFrame {
                    attack: true,
                    ..Default::default()
                });
                assert!(game.player.attack_timer > 0);
                let timer = game.player.attack_timer;
                game.step(&InputFrame {
                    down: true,
                    attack: true,
                    ..Default::default()
                });
                assert!(game.player.held.is_none());
                assert!(!game.world.entity(rock).unwrap().held);
                assert_eq!(game.player.attack_timer, timer);
                assert_eq!(game.player.state, PlayerState::Standing);
            }
        }
    }
}

#[test]
fn ordinary_nonwhipping_down_attack_still_picks_up_and_ducks() {
    for preset in PRESETS {
        for character in CHARACTERS {
            for facing in [-1, 1] {
                let mut game = room(preset, character, facing);
                let rock = game.spawn("oRock", Vec2::new(88.0, 124.0));
                game.step(&InputFrame::default());
                game.step(&down_attack());
                assert_eq!(game.player.held, Some(rock));
                assert!(game.world.entity(rock).unwrap().held);
                assert_eq!(game.player.pickup_item_type, "Rock");
                assert_eq!(game.player.attack_timer, 0);
                assert_eq!(game.player.state, PlayerState::Ducking);
            }
        }
    }
}

#[test]
fn already_held_machete_is_not_dropped_or_replaced_during_its_swing() {
    for preset in PRESETS {
        for character in CHARACTERS {
            let mut game = room(preset, character, 1);
            let machete = game.spawn("oMachete", Vec2::new(88.0, 124.0));
            let rock = game.spawn("oRock", Vec2::new(112.0, 124.0));
            game.step(&InputFrame::default());
            game.step(&down_attack());
            assert_eq!(game.player.held, Some(machete));
            game.step(&release());
            game.step(&attack());
            assert!(game.player.attack_timer > 0, "real held swing required");
            game.step(&release());
            let timer = game.player.attack_timer;
            let phase = game.player.animation;

            game.step(&down_attack());

            assert_eq!(game.player.held, Some(machete));
            assert!(game.world.entity(machete).unwrap().held);
            assert!(!game.world.entity(rock).unwrap().held);
            assert_eq!(game.player.pickup_item_type, "Machete");
            assert_eq!(game.player.attack_timer, timer);
            assert!(game.player.animation > phase);
            assert_eq!(game.player.state, PlayerState::Standing);
        }
    }
}

#[test]
fn natural_arrow_trap_hit_cancels_whipping_and_keeps_stunned_pickup_blocked() {
    for preset in PRESETS {
        for character in CHARACTERS {
            let mut game = room(preset, character, 1);
            game.spawn("oRock", Vec2::new(88.0, 124.0));
            game.spawn("oArrowTrapLeft", Vec2::new(144.0, 112.0));
            // A second default Rock falls through the detector. Its motion,
            // detector firing and unsafe Arrow velocity arise from source events.
            game.spawn("oRock", Vec2::new(128.0, 104.0));
            let mut approaching = false;
            for _ in 0..48 {
                approaching = game.world.entities.iter().any(|arrow| {
                    arrow.alive
                        && arrow.object == "oArrow"
                        && arrow.vel.x < -3.0
                        && arrow.value("safe") == 0.0
                        && (32.0..=64.0).contains(&(arrow.pos.x - game.player.pos.x))
                });
                if approaching {
                    break;
                }
                game.step(&InputFrame::default());
            }
            assert!(
                approaching,
                "FIXTURE_TIMING: natural incoming Arrow required"
            );
            assert_eq!(game.player.health, 4.0);
            start_then_release(&mut game);
            let mut hit = false;
            for step in 0..16 {
                assert!(
                    game.player.attack_timer > 0,
                    "FIXTURE_TIMING: Arrow must arrive before Animation End"
                );
                game.step(&InputFrame {
                    down: true,
                    attack: true,
                    attack_pressed: step == 0,
                    ..Default::default()
                });
                if game.player.is_stunned() {
                    hit = true;
                    break;
                }
            }
            assert!(hit, "FIXTURE_TIMING: real Arrow hurt required");
            assert_eq!(game.player.health, 2.0);
            assert_eq!(game.player.attack_timer, 0);
            assert!(game.player.held.is_none());
            game.step(&release());
            assert!(game.player.is_stunned());
            game.step(&down_attack());
            assert!(game.player.is_stunned());
            assert!(game.player.held.is_none());
            assert_eq!(game.player.attack_timer, 0);
            assert!(!game.world.entities.iter().any(|entity| {
                entity.alive
                    && (spelunky_core::catalog::is_a(&entity.object, "oWhip")
                        || spelunky_core::catalog::is_a(&entity.object, "oWhipPre"))
            }));
            assert_eq!(game.state, RunState::Playing);
        }
    }
}
