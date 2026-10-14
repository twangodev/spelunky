//! Configured directional Whip art uses the builtin scale, not port-facing.
use super::*;
use spelunky_core::{RulesPreset, World};

#[test]
fn chosen_whip_art_draws_unchanged_for_either_facing_and_obeys_explicit_scale() {
    let atlas = Atlas::load().unwrap();
    for (object, sprites) in [
        ("oWhip", ["sWhipLeft", "sWhipRight"]),
        ("oWhipPre", ["sWhipPreL", "sWhipPreR"]),
    ] {
        for name in sprites {
            let image = atlas.sprite(name, 0.0).unwrap();
            for facing in [-1, 1] {
                for scale in [None, Some(1.0), Some(-1.0)] {
                    let mut game = Game::shell(123, RulesPreset::Classic);
                    game.world = World::new(30, 20, Area::Mines, 1);
                    game.player.pos = Vec2::new(104.0, 136.0);
                    game.player.previous_pos = game.player.pos;
                    let id = game.world.spawn(object, Vec2::new(160.0, 96.0));
                    let actor = game.world.entity_mut(id).unwrap();
                    actor.set_sprite(name);
                    actor.facing = facing;
                    if let Some(scale) = scale {
                        actor.data.insert("image_xscale".into(), scale);
                    }
                    let mirrored = scale == Some(-1.0);
                    let mut expected_uv = image.uv;
                    if mirrored {
                        expected_uv[0] += expected_uv[2];
                        expected_uv[2] = -expected_uv[2];
                    }
                    let mut opposite_uv = image.uv;
                    if !mirrored {
                        opposite_uv[0] += opposite_uv[2];
                        opposite_uv[2] = -opposite_uv[2];
                    }
                    let before = game.state_hash();
                    for alpha in [0.0, 0.5, 1.0] {
                        let quads = game_scene(&game, alpha, false, &atlas);
                        let body = quads
                            .iter()
                            .find(|quad| quad.uv == expected_uv)
                            .expect("SOURCE_DIRECTIONAL_WHIP_ART_UV");
                        assert!(!quads.iter().any(|quad| quad.uv == opposite_uv));
                        let x = 160.0 - game.camera.x as f32;
                        let y = 96.0 - game.camera.y as f32;
                        let left = if mirrored {
                            x + image.origin[0] - image.size[0]
                        } else {
                            x - image.origin[0]
                        };
                        assert_eq!(
                            body.rect,
                            [left, y - image.origin[1], image.size[0], image.size[1]]
                        );
                        assert_eq!(body.color, WHITE);
                    }
                    assert_eq!(game.state_hash(), before);
                }
            }
        }
    }
}

#[test]
fn non_enemy_builtin_draw_uses_source_scale_without_directional_inference() {
    let mut game = Game::shell(123, RulesPreset::Classic);
    // Parentless flash has no Draw override: builtin Draw uses image_xscale.
    // Rock's own inherited Item Draw passes literal +1, so it is not a valid
    // control for the builtin scale behavior or directional-name inference.
    let id = game
        .world
        .spawn("oShotgunBlastLeft", Vec2::new(160.0, 96.0));
    assert!(
        catalog::object("oShotgunBlastLeft")
            .unwrap()
            .parent
            .is_empty()
    );
    let actor = game.world.entity_mut(id).unwrap();
    actor.set_sprite("sWhipLeft");
    for facing in [-1, 1] {
        actor.facing = facing;
        actor.data.remove("image_xscale");
        assert!(!entity_mirrored(actor, "sWhipLeft"), "default +1 scale");
        actor.data.insert("image_xscale".into(), 1.0);
        assert!(!entity_mirrored(actor, "sWhipLeft"));
        actor.data.insert("image_xscale".into(), -1.0);
        assert!(entity_mirrored(actor, "sWhipLeft"));
    }
}
