//! Parentless worshipper uses default Draw, before Olmec's external Alarm.
use super::*;
use spelunky_core::{InputFrame, RulesPreset, World};

fn assert_worshipper_quad(game: &Game, id: u64, scale: f64, atlas: &Atlas) {
    let view = game.presentation();
    let actor = view.world.entity(id).unwrap();
    assert_eq!(actor.object, "oHawkmanWorship");
    assert_eq!(actor.sprite, "sHawkLeft");
    let definition = catalog::object(&actor.object).unwrap();
    assert!(definition.parent.is_empty());
    assert!(definition.visible);
    assert_eq!(definition.depth, 0);
    let image = atlas.sprite("sHawkLeft", actor.animation).unwrap();
    let mut uv = image.uv;
    if scale < 0.0 {
        uv[0] += uv[2];
        uv[2] = -uv[2];
    }
    let mut opposite = image.uv;
    if scale > 0.0 {
        opposite[0] += opposite[2];
        opposite[2] = -opposite[2];
    }
    let before = game.state_hash();
    for alpha in [0.0, 0.5, 1.0] {
        let quads = game_scene(game, alpha, false, atlas);
        let body = quads
            .iter()
            .find(|quad| quad.uv == uv)
            .expect("SOURCE_PARENTLESS_WORSHIPPER_DEFAULT_DRAW_UV");
        assert!(!quads.iter().any(|quad| quad.uv == opposite));
        let camera = view.previous_camera.lerp(view.camera, alpha);
        let position = actor.previous_pos.lerp(actor.pos, alpha);
        let x = (position.x - camera.x) as f32;
        let y = (position.y - camera.y) as f32;
        let left = if scale < 0.0 {
            x + image.origin[0] - image.size[0]
        } else {
            x - image.origin[0]
        };
        assert_eq!(
            body.rect,
            [left, y - image.origin[1], image.size[0], image.size[1]]
        );
        assert_eq!(body.color, WHITE);
        assert_eq!(body.rotation, 0.0);
    }
    assert_eq!(game.state_hash(), before);
}

#[test]
fn generated_boss_worshipper_keeps_original_unmirrored_art_before_first_alarm() {
    let atlas = Atlas::load().unwrap();
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = Game::new(42, rules);
        while game.world.level < 16 {
            game.next_level();
        }
        let id = game
            .world
            .entities
            .iter()
            .find(|actor| actor.object == "oHawkmanWorship")
            .unwrap()
            .id;
        let actor = game.world.entity(id).unwrap();
        assert_eq!(actor.pos, Vec2::new(280.0, 544.0));
        assert_eq!(actor.facing, 1, "port-facing is not source image_xscale");
        assert_worshipper_quad(&game, id, 1.0, &atlas);
        assert_eq!(game.world.entity(id).unwrap().value("image_xscale"), 1.0);
        // Run the ordinary intro rather than forcing a boss timer or phase.
        for _ in 0..20 {
            game.step(&InputFrame::default());
        }
        assert!(game.world.entity(id).is_some());
        assert!(
            !game
                .world
                .entities
                .iter()
                .any(|actor| actor.object == "oHawkman")
        );
        assert_worshipper_quad(&game, id, 1.0, &atlas);
    }
}

#[test]
fn world_and_game_births_keep_default_draw_and_explicit_scale_overrides() {
    let atlas = Atlas::load().unwrap();
    for through_game in [false, true] {
        for facing in [-1, 1] {
            for override_scale in [None, Some(1.0), Some(-1.0)] {
                let mut game = Game::shell(123, RulesPreset::Classic);
                game.world = World::new(30, 20, Area::Mines, 1);
                game.player.pos = Vec2::new(104.0, 136.0);
                game.player.previous_pos = game.player.pos;
                let id = if through_game {
                    game.spawn("oHawkmanWorship", Vec2::new(160.0, 96.0))
                } else {
                    game.world.spawn("oHawkmanWorship", Vec2::new(160.0, 96.0))
                };
                assert_eq!(game.world.entity(id).unwrap().value("image_xscale"), 1.0);
                let actor = game.world.entity_mut(id).unwrap();
                actor.facing = facing;
                if let Some(scale) = override_scale {
                    actor.data.insert("image_xscale".into(), scale);
                }
                assert_worshipper_quad(&game, id, override_scale.unwrap_or(1.0), &atlas);
            }
        }
    }
    // The constructor adds no new property to unrelated source families.
    let mut game = Game::shell(123, RulesPreset::Classic);
    for object in ["oWhip", "oWhipPre", "oHawkman", "oRock"] {
        let id = game.world.spawn(object, Vec2::new(160.0, 96.0));
        assert!(
            !game
                .world
                .entity(id)
                .unwrap()
                .data
                .contains_key("image_xscale")
        );
    }
}
