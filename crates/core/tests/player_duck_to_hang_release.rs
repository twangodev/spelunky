//! Source ledge-release effects reached by authored rooms and ordinary input.
use spelunky_core::*;

fn ledge_room(rules: RulesPreset, character: characters::Character, direction: i8) -> Game {
    let mut level = level::Level::default();
    level.set_tile(13, 18, '@').unwrap();
    let floor = if direction < 0 { 13..=23 } else { 3..=13 };
    for x in floor {
        level.set_tile(x, 19, '1').unwrap();
    }
    for x in 0..40 {
        level.set_tile(x, 22, '1').unwrap();
    }
    let mut game = Game::shell(42, rules);
    game.load_custom(&level, "ledge-release", "");
    characters::apply(&mut game, character);
    game
}

fn crawl(direction: i8) -> InputFrame {
    InputFrame {
        down: true,
        left: direction < 0,
        right: direction > 0,
        ..Default::default()
    }
}

fn entered_ledge(game: &Game) -> bool {
    game.player.data.get("duck_to_hang") == Some(&1.0)
}

fn assert_ledge(game: &Game) {
    assert!(entered_ledge(game));
    assert!(matches!(
        game.player.sprite.as_str(),
        "sDuckToHangL" | "sDamselDtHL" | "sTunnelDtHL"
    ));
    assert_eq!(game.player.vel, Vec2::default());
    assert!(!game.player.is_stunned());
    assert_eq!(game.state, RunState::Playing);
}

fn check_natural_item_release(object: &str) {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for direction in [-1, 1] {
            let mut game = ledge_room(rules, characters::Character::Explorer, direction);
            let item = game.spawn(
                object,
                Vec2::new(game.player.pos.x, game.player.pos.y + 2.0),
            );
            for _ in 0..4 {
                game.step(&InputFrame::default());
            }
            assert!(game.player.grounded);
            game.step(&InputFrame {
                down: true,
                attack_pressed: true,
                ..Default::default()
            });
            assert_eq!(game.player.held, Some(item), "{object}");
            game.step(&InputFrame {
                down: true,
                ..Default::default()
            });
            let mut released = false;
            for _ in 0..80 {
                let before = game.world.entity(item).unwrap().clone();
                let rng = game.rng.clone().next_u64();
                assert!(before.held);
                game.step(&crawl(direction));
                if entered_ledge(&game) {
                    assert_ledge(&game);
                    assert_eq!(game.player.held, None);
                    let after = game.world.entity(item).unwrap();
                    assert!(!after.held);
                    // characterStepEvent lowers the stored Gold Idol origin
                    // by eight, scrDropItem assigns velocity, then the later
                    // Item Step moves that origin by (direction,-4).
                    let lowering = if object == "oRock" { 0.0 } else { -8.0 };
                    assert_eq!(
                        after.pos,
                        Vec2::new(
                            before.pos.x + f64::from(direction),
                            before.pos.y + lowering - 4.0
                        ),
                        "{rules:?}/{direction}/{object}"
                    );
                    assert_eq!(after.vel.x, f64::from(direction));
                    assert!((after.vel.y - (-3.4)).abs() < 1e-12);
                    assert_eq!(game.rng.clone().next_u64(), rng);
                    released = true;
                    break;
                }
                assert_eq!(game.player.held, Some(item));
            }
            assert!(released, "natural crawl never released {object}");
        }
    }
}

#[test]
fn natural_ledge_release_lowers_gold_idol_and_crystal_skull_before_their_own_step() {
    for object in ["oGoldIdol", "oCrystalSkull"] {
        check_natural_item_release(object);
    }
}

#[test]
fn natural_ledge_release_keeps_ordinary_rock_origin_unchanged() {
    check_natural_item_release("oRock");
}

#[test]
fn natural_ledge_release_detaches_every_naturally_attached_monkey() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for direction in [-1, 1] {
            for character in [
                characters::Character::Explorer,
                characters::Character::Damsel,
                characters::Character::TunnelMan,
            ] {
                let mut game = ledge_room(rules, character, direction);
                let spawn = Vec2::new(game.player.pos.x - 8.0, game.player.pos.y - 8.0);
                let attached = [game.spawn("oMonkey", spawn), game.spawn("oMonkey", spawn)];
                // These initial actors remain HANG: the first is engine-active
                // outside its own AI viewport, the second is deactivated.
                // Neither may be changed merely because it is a Monkey.
                let outside_view = game.spawn("oMonkey", Vec2::new(432.0, 304.0));
                let inactive = game.spawn("oMonkey", Vec2::new(600.0, 304.0));
                for _ in 0..4 {
                    game.step(&InputFrame::default());
                }
                assert!(game.player.grounded);
                for id in attached {
                    assert_eq!(game.world.entity(id).unwrap().state, 7);
                }
                let health = game.player.health;
                let mut detached = false;
                for _ in 0..80 {
                    let before = attached.map(|id| game.world.entity(id).unwrap().clone());
                    let controls =
                        [outside_view, inactive].map(|id| game.world.entity(id).unwrap().clone());
                    let rng = game.rng.clone().next_u64();
                    for monkey in &before {
                        assert_eq!(monkey.state, 7);
                        assert!(monkey.timer > 0, "fixture reached the theft timeout");
                    }
                    game.step(&crawl(direction));
                    if entered_ledge(&game) {
                        assert_ledge(&game);
                        assert_eq!(game.player.health, health);
                        for (id, before) in attached.into_iter().zip(before) {
                            let after = game.world.entity(id).unwrap();
                            // Player's earlier Step assigns BOUNCE (1), velocity
                            // (direction,-4), counters20/60. Monkey's own later
                            // airborne Step moves, adds .6 gravity, decrements
                            // those counters and enters RECOVER (2), without RNG.
                            assert_eq!(after.state, 2);
                            assert_eq!(
                                after.pos,
                                Vec2::new(before.pos.x + f64::from(direction), before.pos.y - 4.0)
                            );
                            assert_eq!(after.vel.x, f64::from(direction));
                            assert!((after.vel.y - (-3.4)).abs() < 1e-12);
                            assert_eq!(after.value("vine_counter"), 19.0);
                            assert_eq!(after.value("grab_counter"), 59.0);
                            assert_eq!(after.timer, before.timer);
                        }
                        for (id, before) in [outside_view, inactive].into_iter().zip(controls) {
                            let after = game.world.entity(id).unwrap();
                            assert_eq!(after.state, before.state);
                            assert_eq!(after.pos, before.pos);
                            assert_eq!(after.vel, before.vel);
                            assert_eq!(after.timer, before.timer);
                            assert_eq!(after.value("grab_counter"), before.value("grab_counter"));
                            assert_eq!(after.value("vine_counter"), before.value("vine_counter"));
                        }
                        assert!(game.world.entity(outside_view).unwrap().engine_active);
                        assert!(!game.world.entity(inactive).unwrap().engine_active);
                        assert_eq!(game.rng.clone().next_u64(), rng);
                        detached = true;
                        break;
                    }
                }
                assert!(
                    detached,
                    "natural crawl never detached monkeys: {rules:?}/{direction}/{character:?}"
                );
            }
        }
    }
}
