//! Explicit callback-context controls, not whole-Step or stock Boss routes.
use super::*;
use crate::{Area, RulesPreset, World};

#[test]
fn actual_level_step_reactivates_the_only_character_before_wave_qualified_reads() {
    for preset in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = Game::shell(42, preset);
        game.world = World::new(20, 16, Area::Ice, 9);
        game.world.name = "rLevel".into();
        for x in 0..20 {
            game.world.set_tile(x, 7, "oHardBlock");
        }
        game.player.pos = Vec2::new(200.0, 104.0);
        game.player.previous_pos = game.player.pos;
        game.player.engine_active = false;
        let wave = game.spawn("oPsychicWave", Vec2::new(104.0, 104.0));
        let active_characters = crate::dispatcher::descendant_instances(&game, "oCharacter")
            .into_iter()
            .filter(|instance| crate::dispatcher::is_active(&game, *instance))
            .count();
        assert_eq!(active_characters, 0, "explicit initial empty active family");
        let rng = game.rng.state_bits();
        game.step(&crate::InputFrame::default());
        assert!(game.player.engine_active);
        assert_eq!(game.player.pos, Vec2::new(200.0, 104.0));
        assert_eq!(game.player.health, 4);
        let wave = game.world.entity(wave).unwrap();
        assert_eq!(wave.pos, Vec2::new(106.0, 104.0));
        assert_eq!(wave.value("dir"), 0.0);
        assert_eq!(game.rng.state_bits(), rng);
        // oLevel Step activates Character before PsychicWave's resource Step.
        // This is a normal-dispatch availability invariant; it does not define
        // native getter/error semantics for an isolated callback with no target.
    }
}

#[test]
fn configured_wave_lookup_skips_inactive_and_retired_characters_before_live_member() {
    for preset in [RulesPreset::Classic, RulesPreset::Assisted] {
        // Compatibility controls precede the intended active-family difference.
        for state in [0, 1, 2] {
            let mut game = Game::shell(42, preset);
            game.world = World::new(20, 16, Area::Ice, 9);
            game.player.pos = Vec2::new(200.0, 104.0);
            let id = game.spawn("oCharacter", Vec2::new(104.0, 120.0));
            let target = game.world.entities.iter_mut().find(|e| e.id == id).unwrap();
            target.engine_active = state != 0;
            target.alive = state != 1;
            // Controlled native family order, authored before the callback.
            game.dispatcher.registration_order = vec![
                crate::dispatcher::Instance::Entity(id),
                crate::dispatcher::Instance::Player,
            ];
            let mut wave = Entity::new(999, "oPsychicWave", Vec2::new(104.0, 104.0));
            let rng = game.rng.clone();
            psychic(&mut game, &mut wave);
            let expected = if state == 2 {
                Vec2::new(104.0, 106.0)
            } else {
                Vec2::new(106.0, 104.0)
            };
            assert_eq!(
                wave.pos, expected,
                "only the first active unretired Character supplies the heading"
            );
            assert_eq!(game.rng.clone().next_u64(), rng.clone().next_u64());
            if state == 2 {
                assert_eq!(wave.data.get("dir").copied(), Some(270.0));
            }
        }
    }
}
