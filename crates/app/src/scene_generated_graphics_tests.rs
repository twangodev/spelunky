use super::*;
use spelunky_core::{GeneratedGraphicTileSpec, RulesPreset, World};

fn graphic(x: f64, color: u32, depth: f64) -> GeneratedGraphicTileSpec {
    GeneratedGraphicTileSpec {
        background: "bgCaveTop".into(),
        source: [16.0, 0.0, 16.0, 16.0],
        position: Vec2::new(x, 100.0),
        depth,
        scale: Vec2::new(1.0, 1.0),
        color,
        alpha: 1.0,
        visible: true,
    }
}

#[test]
fn actual_atlas_crop_projection_retains_scale_tint_alpha_and_fractional_position() {
    let atlas = Atlas::load().unwrap();
    let mut spec = graphic(40.5, 0x00332211, 3.25);
    spec.position.y = 20.25;
    spec.scale = Vec2::new(2.0, 3.0);
    spec.alpha = 0.5;
    let quad = generated_graphic_quad(&spec, &atlas, Vec2::new(10.0, 4.0), false).unwrap();
    assert_eq!(quad.rect, [30.5, 16.25, 32.0, 48.0]);
    assert_eq!(
        quad.uv,
        atlas.region("bgCaveTop", [16, 0, 16, 16]).unwrap().uv
    );
    assert_eq!(quad.color, [17.0 / 255.0, 34.0 / 255.0, 51.0 / 255.0, 0.5]);
}

#[test]
fn scene_reads_real_graphics_storage_and_keeps_legacy_actor_first_at_equal_zero_depth() {
    let atlas = Atlas::load().unwrap();
    let mut game = Game::shell(1, RulesPreset::Classic);
    game.world = World::new(40, 32, Area::Mines, 1);
    game.camera = Vec2::new(0.0, 0.0);
    game.previous_camera = game.camera;
    game.player.pos = Vec2::new(500.0, 400.0);
    game.player.previous_pos = game.player.pos;
    let legacy = game
        .world
        .spawn("oGenerationDecoration", Vec2::new(100.0, 100.0));
    let e = game.world.entity_mut(legacy).unwrap();
    e.sprite = "bgCaveTop".into();
    for (name, value) in [
        ("source_x", 16.0),
        ("source_y", 0.0),
        ("source_w", 16.0),
        ("source_h", 16.0),
        ("depth", 0.0),
    ] {
        e.data.insert(name.into(), value);
    }
    game.world
        .add_generated_graphic_tile(graphic(100.0, 0x000000ff, -0.0))
        .unwrap();
    game.world
        .add_generated_graphic_tile(graphic(100.0, 0x0000ff00, 0.0))
        .unwrap();
    let before = serde_json::to_value(&game.world).unwrap();
    let quads = game_scene(&game, 1.0, false, &atlas);
    let uv = atlas.region("bgCaveTop", [16, 0, 16, 16]).unwrap().uv;
    let colors: Vec<_> = quads
        .iter()
        .filter(|q| q.rect == [100.0, 100.0, 16.0, 16.0] && q.uv == uv)
        .map(|q| q.color)
        .collect();
    assert_eq!(colors, [WHITE, [1.0, 0.0, 0.0, 1.0], [0.0, 1.0, 0.0, 1.0]]);
    assert_eq!(serde_json::to_value(&game.world).unwrap(), before);
    assert!(game.world.entity(legacy).is_some());
}

#[test]
fn scene_preserves_fractional_depth_and_current_graphics_order_after_removal() {
    let atlas = Atlas::load().unwrap();
    let mut game = Game::shell(1, RulesPreset::Classic);
    game.world = World::new(40, 32, Area::Mines, 1);
    game.camera = Vec2::new(0.0, 0.0);
    game.previous_camera = game.camera;
    game.player.pos = Vec2::new(500.0, 400.0);
    game.player.previous_pos = game.player.pos;
    let a = game
        .world
        .add_generated_graphic_tile(graphic(101.0, 0xffffff, 3.25))
        .unwrap();
    game.world
        .add_generated_graphic_tile(graphic(102.0, 0xffffff, 3.75))
        .unwrap();
    game.world
        .add_generated_graphic_tile(graphic(103.0, 0xffffff, 3.25))
        .unwrap();
    assert!(game.world.remove_generated_graphic_tile(a));
    game.world
        .add_generated_graphic_tile(graphic(104.0, 0xffffff, 3.25))
        .unwrap();
    let uv = atlas.region("bgCaveTop", [16, 0, 16, 16]).unwrap().uv;
    let order: Vec<_> = game_scene(&game, 1.0, false, &atlas)
        .iter()
        .filter(|q| {
            q.uv == uv
                && q.rect[1..] == [100.0, 16.0, 16.0]
                && [102.0, 103.0, 104.0].contains(&q.rect[0])
        })
        .map(|q| q.rect[0])
        .collect();
    assert_eq!(order, [102.0, 103.0, 104.0]);
}

#[test]
fn hidden_unknown_or_unsupported_crop_does_not_invent_atlas_content() {
    let atlas = Atlas::load().unwrap();
    let mut spec = graphic(0.0, 0xffffff, 0.0);
    spec.visible = false;
    assert!(generated_graphic_quad(&spec, &atlas, Vec2::new(0.0, 0.0), false).is_none());
    spec.visible = true;
    spec.background = "missing".into();
    assert!(generated_graphic_quad(&spec, &atlas, Vec2::new(0.0, 0.0), false).is_none());
    spec.background = "bgCaveTop".into();
    spec.source[0] = 0.5;
    assert!(generated_graphic_quad(&spec, &atlas, Vec2::new(0.0, 0.0), false).is_none());
    spec.source[0] = -1.0;
    assert!(generated_graphic_quad(&spec, &atlas, Vec2::new(0.0, 0.0), false).is_none());
}
