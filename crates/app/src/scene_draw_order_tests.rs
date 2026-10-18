//! Draw layers retain the source active-instance order when depths tie.
use super::*;
use spelunky_core::{InputFrame, RulesPreset, World};

#[test]
fn reactivated_older_ruby_draws_over_newer_ruby_at_the_same_source_depth() {
    let atlas = Atlas::load().unwrap();
    let mut game = Game::shell(9, RulesPreset::Classic);
    game.world = World::new(80, 40, Area::Mines, 1);
    game.world.name = "rLevel".into();
    game.camera = Vec2::default();
    game.previous_camera = game.camera;
    game.player.pos = Vec2::new(24.0, 24.0);
    game.player.previous_pos = game.player.pos;

    let older_ruby = game.spawn("oRuby", Vec2::new(600.0, 160.0));
    let newer_big_ruby = game.spawn("oRubyBig", Vec2::new(160.0, 160.0));
    assert_eq!(catalog::object("oRuby").unwrap().depth, 101);
    assert_eq!(catalog::object("oRubyBig").unwrap().depth, 101);

    game.step(&InputFrame::default());
    assert!(!game.world.entity(older_ruby).unwrap().engine_active);
    assert!(game.world.entity(newer_big_ruby).unwrap().engine_active);

    // The Classic Level step deactivates off-view instances, then appends an
    // instance that reenters the active list before the next stable Draw sort.
    let return_position = game.world.entity(newer_big_ruby).unwrap().pos;
    let older = game.world.entity_mut(older_ruby).unwrap();
    older.pos = return_position;
    older.previous_pos = return_position;
    game.step(&InputFrame::default());

    let draw = game.presentation();
    let order: Vec<_> = draw
        .dispatcher
        .active_order
        .iter()
        .filter_map(|instance| match instance {
            spelunky_core::dispatcher::Instance::Entity(id)
                if *id == older_ruby || *id == newer_big_ruby =>
            {
                Some(*id)
            }
            _ => None,
        })
        .collect();
    assert_eq!(order, [newer_big_ruby, older_ruby]);

    let ruby_uv = atlas.sprite("sRuby", 0.0).unwrap().uv;
    let big_ruby_uv = atlas.sprite("sRubyBig", 0.0).unwrap().uv;
    assert_ne!(ruby_uv, big_ruby_uv);
    let quads = game_scene(draw, 1.0, false, &atlas);
    let source_draw_order: Vec<_> = quads
        .iter()
        .enumerate()
        .filter_map(|(index, quad)| {
            if quad.uv == ruby_uv {
                Some((index, older_ruby))
            } else if quad.uv == big_ruby_uv {
                Some((index, newer_big_ruby))
            } else {
                None
            }
        })
        .collect();
    assert_eq!(source_draw_order.len(), 2);
    let older_quad = &quads[source_draw_order[0].0.min(source_draw_order[1].0)];
    let newer_quad = &quads[source_draw_order[0].0.max(source_draw_order[1].0)];
    assert!(
        older_quad.rect[0] < newer_quad.rect[0] + newer_quad.rect[2]
            && newer_quad.rect[0] < older_quad.rect[0] + older_quad.rect[2]
            && older_quad.rect[1] < newer_quad.rect[1] + newer_quad.rect[3]
            && newer_quad.rect[1] < older_quad.rect[1] + older_quad.rect[3],
        "the two rendered Ruby sprites must overlap to expose the topmost instance"
    );
    assert_eq!(source_draw_order[0].1, newer_big_ruby);
    assert_eq!(source_draw_order[1].1, older_ruby);
    assert!(source_draw_order[1].0 > source_draw_order[0].0);
}
