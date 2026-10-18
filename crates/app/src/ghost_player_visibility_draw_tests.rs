//! Ghost's ordinary collision hides the character before drawing its remains.
use super::*;
use spelunky_core::{InputFrame, RulesPreset, RunState, World};

fn player_uv(game: &Game, atlas: &Atlas) -> [f32; 4] {
    let player = &game.presentation().player;
    let mut uv = atlas.sprite(&player.sprite, player.animation).unwrap().uv;
    if player.facing > 0 {
        uv[0] += uv[2];
        uv[2] = -uv[2];
    }
    uv
}

#[test]
fn ordinary_ghost_collision_hides_player_draw_and_keeps_bones_and_skull() {
    let atlas = Atlas::load().unwrap();
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = Game::shell(123, rules);
        game.world = World::new(30, 20, Area::Mines, 1);
        for x in 0..30 {
            game.world.set_tile(x, 9, "oBlock");
        }
        game.player.pos = Vec2::new(104.0, 136.0);
        game.player.previous_pos = game.player.pos;
        game.spawn("oGhost", Vec2::new(96.0, 120.0));
        assert!(!game.player.data.contains_key("visible"));
        assert!(
            game_scene(&game, 1.0, false, &atlas)
                .iter()
                .any(|quad| quad.uv == player_uv(&game, &atlas))
        );

        // No forced health, invulnerability, visibility or death state: the
        // source Collision receiver performs all of these writes itself.
        game.step(&InputFrame::default());
        assert_eq!(game.player.health, -99.0);
        assert_eq!(game.player.data.get("visible"), Some(&0.0));
        assert_eq!(game.state, RunState::Playing);
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|entity| entity.alive && entity.object == "oBone")
                .count(),
            3
        );
        assert!(
            game.world
                .entities
                .iter()
                .any(|entity| entity.alive && entity.object == "oSkull")
        );
        for step in 0..2 {
            if step == 1 {
                game.step(&InputFrame::default());
                assert_eq!(game.state, RunState::Dead);
            }
            let before = game.state_hash();
            for alpha in [0.0, 0.5, 1.0] {
                let quads = game_scene(&game, alpha, false, &atlas);
                assert!(!quads.iter().any(|quad| quad.uv == player_uv(&game, &atlas)));
                for object in ["oBone", "oSkull"] {
                    let actor = game
                        .presentation()
                        .world
                        .entities
                        .iter()
                        .find(|entity| entity.alive && entity.object == object)
                        .unwrap();
                    let uv = atlas.sprite(&actor.sprite, actor.animation).unwrap().uv;
                    assert!(quads.iter().any(|quad| quad.uv == uv));
                }
            }
            assert_eq!(game.state_hash(), before);
        }
    }
}

#[test]
fn player_draw_defaults_visible_and_honors_explicit_visibility_without_hiding_corpses() {
    let atlas = Atlas::load().unwrap();
    let mut game = Game::shell(123, RulesPreset::Classic);
    game.player.pos = Vec2::new(104.0, 136.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Dead;
    game.player.sprite = "sDieL".into();
    game.player.facing = -1;
    for visible in [None, Some(1.0), Some(0.0)] {
        if let Some(visible) = visible {
            game.player.data.insert("visible".into(), visible);
        } else {
            game.player.data.remove("visible");
        }
        let drawn = game_scene(&game, 1.0, false, &atlas)
            .iter()
            .any(|quad| quad.uv == player_uv(&game, &atlas));
        assert_eq!(drawn, visible != Some(0.0));
    }
}
