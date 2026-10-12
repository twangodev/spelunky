//! Ordinary pickup and generic throws from literal initial custom-level glyphs.
//! After loading the authored room, only InputFrame actions advance simulation.
use spelunky_core::{Game, GameEvent, InputFrame, PlayerState, RulesPreset, Vec2, level::Level};

fn picked_up(rules: RulesPreset, facing: i8, glyph: char, object: &str) -> (Game, u64) {
    let mut level = Level::default();
    level.set_tile(7, 8, '@').unwrap();
    level.set_tile(34, 8, 'X').unwrap();
    // The source glyph places a Damsel at y152 or an Idol at y156 over y160.
    level.set_tile(8, 8, glyph).unwrap();
    for x in 0..spelunky_core::level::LEVEL_WIDTH {
        level.set_tile(x, 9, '1').unwrap();
    }
    let mut game = Game::shell(23, rules);
    game.load_custom(&level, "damsel-throw-source", "");
    // Initial protagonist placement overlaps the glyph; no later field writes.
    game.player.pos = Vec2::new(152.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game.world.entrance = game.player.pos;
    game.player.facing = facing;
    let actors: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|actor| actor.alive && actor.object == object)
        .map(|actor| actor.id)
        .collect();
    assert_eq!(actors.len(), 1, "SETUP: one native glyph actor");
    let id = actors[0];
    assert_eq!(game.player.pos, Vec2::new(152.0, 152.0));
    for _ in 0..16 {
        game.step(&InputFrame::default());
    }
    assert!(game.player.grounded, "SETUP: ordinary floor settlement");
    assert_eq!(game.player.facing, facing);
    assert_eq!(game.player.health, 4);
    assert_eq!(
        game.world.entity(id).unwrap().pos,
        Vec2::new(152.0, if object == "oDamsel" { 152.0 } else { 156.0 })
    );
    assert_eq!(game.world.entity(id).unwrap().value("heavy"), 1.0);
    game.step(&InputFrame {
        down: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert_eq!(
        game.player.held,
        Some(id),
        "SETUP: ordinary Down+Attack pickup"
    );
    assert!(game.world.entity(id).unwrap().held);
    game.step(&InputFrame {
        attack_released: true,
        ..Default::default()
    });
    assert_eq!(game.player.held, Some(id));
    assert!(game.world.entity(id).unwrap().held);
    assert_eq!(game.player.state, PlayerState::Standing);
    assert_ne!(
        game.player.sprite, "sWhoaLeft",
        "SETUP: full floor excludes balance release"
    );
    assert_eq!(game.player.vel, Vec2::default());
    (game, id)
}

fn cues(game: &Game) -> Vec<(&str, Vec2)> {
    game.events
        .iter()
        .filter_map(|event| match event {
            GameEvent::Sound { name, pos } if name == "sndDamsel" || name == "sndThrow" => {
                Some((name.as_str(), *pos))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn ordinary_damsel_generic_throw_emits_damsel_before_throw_at_player_origin() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for facing in [-1, 1] {
            for up in [false, true] {
                let (mut game, id) = picked_up(rules, facing, 'D', "oDamsel");
                assert_eq!(game.player.pickup_item_type, "Damsel");
                assert_eq!(game.world.entity(id).unwrap().health, 4);
                let origin = game.player.pos;
                let held_y = game.world.entity(id).unwrap().pos.y;
                let resources = (game.player.health, game.player.bombs, game.player.ropes);
                let next_id = game.world.next_id;
                let rng = game.rng.state_bits();
                game.step(&InputFrame {
                    attack: true,
                    attack_pressed: true,
                    up,
                    up_pressed: up,
                    ..Default::default()
                });
                assert!(
                    game.player.held.is_none(),
                    "SETUP: actual generic throw releases hand"
                );
                assert!(game.player.pickup_item_type.is_empty());
                let actor = game
                    .world
                    .entity(id)
                    .expect("healthy thrown Damsel survives");
                assert!(!actor.held);
                assert_eq!((actor.health, actor.state), (4, 2));
                assert!(
                    actor.timer > 0 && actor.timer <= 120,
                    "source thrown stun counter is live"
                );
                assert_eq!(actor.vel.x, f64::from(facing) * 4.0);
                assert!(
                    actor.vel.y < 0.0 && actor.pos.y < held_y,
                    "ordinary heavy launch is airborne"
                );
                assert_eq!(actor.value("safe"), 1.0);
                assert_eq!(actor.alarms[2], 10);
                assert_eq!(actor.value("owner"), 1.0);
                assert_eq!(
                    cues(&game),
                    vec![("sndDamsel", origin), ("sndThrow", origin)],
                    "Damsel own throw cue precedes the shared cue ({rules:?}/{facing}/up={up})"
                );
                assert_eq!(
                    (game.player.health, game.player.bombs, game.player.ropes),
                    resources
                );
                assert_eq!(game.world.next_id, next_id);
                assert_eq!(game.rng.state_bits(), rng);
            }
        }
    }
}

#[test]
fn ordinary_other_heavy_glyph_throw_keeps_only_shared_throw_cue() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for facing in [-1, 1] {
            for up in [false, true] {
                let (mut game, id) = picked_up(rules, facing, '#', "oGoldIdol");
                let origin = game.player.pos;
                let resources = (game.player.health, game.player.bombs, game.player.ropes);
                let next_id = game.world.next_id;
                let rng = game.rng.state_bits();
                game.step(&InputFrame {
                    attack: true,
                    attack_pressed: true,
                    up,
                    up_pressed: up,
                    ..Default::default()
                });
                assert!(
                    game.player.held.is_none(),
                    "SETUP: other heavy actor is actually thrown"
                );
                assert!(game.player.pickup_item_type.is_empty());
                let actor = game.world.entity(id).expect("heavy control survives");
                assert!(!actor.held);
                assert_eq!(actor.vel.x, f64::from(facing) * 4.0);
                assert!(actor.vel.y < 0.0);
                assert_eq!(actor.value("safe"), 1.0);
                assert_eq!(actor.alarms[2], 10);
                assert_eq!(
                    cues(&game),
                    vec![("sndThrow", origin)],
                    "other heavy actor has no Damsel cue"
                );
                assert_eq!(
                    (game.player.health, game.player.bombs, game.player.ropes),
                    resources
                );
                assert_eq!(game.world.next_id, next_id);
                assert_eq!(game.rng.state_bits(), rng);
            }
        }
    }
}
