//! Supplemental direct source-Step controls; target activation is initial setup.
use super::*;
use crate::{Game, InputFrame, RulesPreset, Vec2};

#[test]
fn game_step_spear_support_only_checks_source_sprites() {
    for (revision, sprite, supported, survives) in [
        (
            crate::GenerationRevision::SourceClassicV2,
            "sBlock",
            false,
            true,
        ),
        (
            crate::GenerationRevision::SourceClassicV2,
            "sSpearsLeft",
            false,
            false,
        ),
        (
            crate::GenerationRevision::SourceClassicV2,
            "sSpearsRight",
            false,
            false,
        ),
        (
            crate::GenerationRevision::SourceClassicV2,
            "sSpearsRight",
            true,
            true,
        ),
        (
            crate::GenerationRevision::HistoricalV1,
            "sBlock",
            false,
            false,
        ),
    ] {
        let mut game = Game::shell(42, RulesPreset::Classic);
        game.generation_revision = revision;
        game.player.pos = Vec2::new(200.0, 100.0);
        if supported {
            game.world.set_tile(4, 6, "oHardBlock");
            game.spawn("oSpearTrapTop", Vec2::new(64.0, 80.0));
        }
        let spear = game.spawn("oSpearsLeft", Vec2::new(80.0, 80.0));
        let entity = game.world.entity_mut(spear).unwrap();
        entity.set_sprite(sprite);
        entity.set_image_speed(0.0);

        game.step(&InputFrame::default());

        let entity = game.world.entity(spear);
        assert_eq!(
            entity.is_some(),
            survives,
            "{revision:?} sprite={sprite} supported={supported}"
        );
        if let Some(entity) = entity {
            assert_eq!(entity.value("depth"), 995.0);
        }
    }
}

#[test]
fn spear_nearest_ignores_inactive_members_and_includes_terrain() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for terrain in [false, true] {
            let mut game = Game::shell(42, rules);
            game.player.pos = Vec2::new(320.0, 104.0);
            let trap = game.spawn("oSpearTrapTop", Vec2::new(160.0, 96.0));
            game.world.set_tile(10, 7, "oHardBlock");
            let inactive = game.spawn("oPushBlock", Vec2::new(144.0, 112.0));
            game.world.entity_mut(inactive).unwrap().engine_active = false;
            if terrain {
                game.world.set_tile(8, 6, "oPushBlock");
            } else {
                game.spawn("oPushBlock", Vec2::new(128.0, 96.0));
            }
            crate::dispatcher::register_instances(&mut game);
            game.world.entity_mut(inactive).unwrap().engine_active = false;
            step_instance(&mut game, Instance::Entity(trap));
            assert_eq!(
                game.world
                    .entities
                    .iter()
                    .filter(|e| e.alive && e.object == "oSpearsLeft")
                    .count(),
                1
            );
            assert_eq!(game.world.entity(trap).unwrap().value("fired"), 50.0);
        }
    }
}

#[test]
fn spear_support_point_ignores_inactive_backing_traps() {
    for (trap_active, spear_survives) in [(false, false), (true, true)] {
        let mut game = Game::shell(42, RulesPreset::Classic);
        let trap = game.spawn("oSpearTrapTop", Vec2::new(80.0, 96.0));
        let spear = game.spawn("oSpearsLeft", Vec2::new(72.0, 96.0));
        game.world.entity_mut(trap).unwrap().engine_active = trap_active;

        let anchor = Vec2::new(88.0, 96.0);
        assert_eq!(
            game.world.point_family_active(anchor, "oSpearTrapTop"),
            trap_active
        );
        step_instance(&mut game, Instance::Entity(spear));
        assert_eq!(
            game.world.entity(spear).is_some(),
            spear_survives,
            "spear support query must honor the backing trap's activation state"
        );
    }
}
