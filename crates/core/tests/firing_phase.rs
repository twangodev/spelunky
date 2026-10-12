//! Source firing decay precedes new item actions; release shots assign their
//! cooldown later in the same Player Step. All actors are initial-only.
use spelunky_core::{Area, Game, GameEvent, InputFrame, RulesPreset, Vec2, World};

fn pickup_weapon(rules: RulesPreset, facing: i8, object: &str) -> (Game, u64) {
    let mut game = Game::shell(42, rules);
    game.world = World::new(80, 30, Area::Mines, 1);
    game.world.name = "rLevel".into();
    for x in 0..game.world.width {
        game.world.set_tile(x, 10, "oHardBlock");
    }
    game.player.pos = Vec2::new(256.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.facing = facing;
    game.world.entrance = game.player.pos;
    game.camera = Vec2::new(96.0, 0.0);
    game.previous_camera = game.camera;
    let weapon = game.spawn(object, game.player.pos);
    for _ in 0..3 {
        game.step(&InputFrame::default());
    }
    assert!(game.player.grounded, "SETUP: natural floor settlement");
    game.step(&InputFrame {
        down: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert_eq!(
        game.player.held,
        Some(weapon),
        "SETUP: real Down+Attack pickup"
    );
    game.step(&InputFrame {
        attack_released: true,
        ..Default::default()
    });
    assert_eq!(game.player.held, Some(weapon));
    assert_eq!(game.player.facing, facing);
    assert_eq!(game.player.health, 4);
    assert_eq!(game.player.attack_timer, 0);
    assert_eq!(
        game.player
            .data
            .get("weapon_cooldown")
            .copied()
            .unwrap_or(0.0),
        0.0
    );
    (game, weapon)
}

fn attack_edge(game: &mut Game) {
    game.step(&InputFrame {
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
}

fn sound_count(game: &Game, expected: &str) -> usize {
    game.events
        .iter()
        .filter(|event| matches!(event, GameEvent::Sound { name, .. } if name == expected))
        .count()
}

#[test]
fn ordinary_bow_redraw_observes_early_cooldown_boundary_and_controls() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for facing in [-1, 1] {
            for neutral_ticks in [8, 9, 10] {
                let (mut game, bow) = pickup_weapon(rules, facing, "oBow");
                assert_eq!(game.data["arrows"], 6.0);
                attack_edge(&mut game);
                assert_eq!(game.player.data["bow_armed"], 1.0, "SETUP: first draw");
                game.step(&InputFrame {
                    attack_released: true,
                    ..Default::default()
                });
                assert_eq!(game.player.data["bow_armed"], 0.0);
                assert_eq!(game.data["arrows"], 5.0);
                assert_eq!(
                    game.player.data["weapon_cooldown"], 10.0,
                    "release assigns firing10 after early decay"
                );
                assert_eq!(sound_count(&game, "sndArrowTrap"), 1);
                assert_eq!(
                    game.world
                        .entities
                        .iter()
                        .filter(|e| e.object == "oArrow")
                        .count(),
                    1
                );
                for _ in 0..neutral_ticks {
                    game.step(&InputFrame::default());
                }
                assert_eq!(
                    game.player.data["weapon_cooldown"],
                    (10.0 - f64::from(neutral_ticks)).max(0.0)
                );
                attack_edge(&mut game);
                let eligible = neutral_ticks >= 9;
                assert_eq!(
                    game.player.data["bow_armed"],
                    f64::from(eligible),
                    "source early firing decay: neutral={neutral_ticks} facing={facing} rules={rules:?}"
                );
                assert_eq!(sound_count(&game, "sndBowPull"), usize::from(eligible));
                assert_eq!(
                    sound_count(&game, "sndArrowTrap"),
                    0,
                    "redraw does not itself shoot"
                );
                assert_eq!(game.data["arrows"], 5.0);
                assert_eq!(game.player.held, Some(bow));
                assert_eq!(game.player.health, 4);
            }
        }
    }
}

#[test]
fn ordinary_gun_shots_keep_new_cooldown_until_next_early_player_step() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for facing in [-1, 1] {
            for (object, projectile, count, cooldown, sound) in [
                ("oPistol", "oBullet", 1, 20.0, "sndShotgun"),
                ("oShotgun", "oBullet", 6, 40.0, "sndShotgun"),
                ("oSceptre", "oPsychicWaveP", 1, 20.0, "sndPsychic"),
                ("oWebCannon", "oWebBall", 1, 20.0, "sndShotgun"),
            ] {
                let (mut game, weapon) = pickup_weapon(rules, facing, object);
                let resources = (game.player.health, game.player.bombs, game.player.ropes);
                attack_edge(&mut game);
                assert_eq!(
                    game.world
                        .entities
                        .iter()
                        .filter(|e| e.alive && e.object == projectile)
                        .count(),
                    count,
                    "SETUP: actual {object} shot"
                );
                assert_eq!(sound_count(&game, sound), 1);
                assert_eq!(game.player.held, Some(weapon));
                assert_eq!(
                    game.player.data["weapon_cooldown"], cooldown,
                    "newly assigned firing is not decremented in the late tail ({object})"
                );
                game.step(&InputFrame::default());
                assert_eq!(
                    game.player.data["weapon_cooldown"],
                    cooldown - 1.0,
                    "the next Player Step decrements exactly once ({object})"
                );
                assert_eq!(
                    (game.player.health, game.player.bombs, game.player.ropes),
                    resources
                );
                assert_eq!(sound_count(&game, sound), 0);
                assert_eq!(game.player.held, Some(weapon));
            }
        }
    }
}
