//! Literal oEnemy/Shopkeeper Draw geometry; source-configured and full-Step cases.
use super::*;
use spelunky_core::{InputFrame, RulesPreset, World};

fn room(seed: u32, rules: RulesPreset) -> Game {
    let mut game = Game::shell(seed, rules);
    game.world = World::new(40, 30, Area::Mines, 1);
    game.flags.insert("custom".into());
    game.player.pos = Vec2::new(32.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    for x in 0..40 {
        game.world.set_tile(x, 10, "oBlock");
    }
    game
}

fn assert_source_body(game: &Game, id: u64, right: bool, atlas: &Atlas) {
    let draw = game.presentation();
    let actor = draw.world.entity(id).expect("visible live Draw receiver");
    let image = atlas.sprite(&actor.sprite, actor.animation).unwrap();
    assert_eq!(image.origin, [0.0, 0.0]);
    let mut expected_uv = image.uv;
    if right {
        expected_uv[0] += expected_uv[2];
        expected_uv[2] = -expected_uv[2];
    }
    let before = game.state_hash();
    let random = game.rng.clone().next_u64();
    for alpha in [0.0, 0.5, 1.0] {
        for snap in [false, true] {
            let quads = game_scene(game, alpha, snap, atlas);
            let body = quads
                .iter()
                .find(|q| q.uv == expected_uv)
                .expect("SOURCE_INHERITED_DRAW_FACING_UV");
            let alpha = if draw.flags.contains("render_discontinuity") {
                1.0
            } else {
                alpha
            };
            let position = actor.previous_pos.lerp(actor.pos, alpha);
            let camera = draw.previous_camera.lerp(draw.camera, alpha);
            let x = position.x - camera.x;
            let y = position.y - camera.y;
            let x = if snap { x.round() } else { x } as f32;
            let y = if snap { y.round() } else { y } as f32;
            // Source x+16 then xscale=-1: a 16px body retains the same left edge.
            let left = if right { x + 16.0 - image.size[0] } else { x };
            assert_eq!(body.rect, [left, y, image.size[0], image.size[1]]);
            assert_eq!(body.color, WHITE);
        }
    }
    assert_eq!(game.state_hash(), before);
    assert_eq!(game.rng.clone().next_u64(), random);
}

#[test]
fn inherited_enemy_draw_uses_source_facing_on_every_death_frame() {
    let atlas = Atlas::load().unwrap();
    for (object, sprite, frames) in [
        ("oCaveman", "sCavemanDieLR", 1),
        ("oYeti", "sYetiDieLR", 1),
        ("oHawkman", "sHawkDieLR", 1),
        ("oShopkeeper", "sShopDieLR", 1),
        ("oShopkeeper2", "sShopDieLR", 1),
        ("oMagmaMan", "sMagmaManDie", 13),
    ] {
        for right in [false, true] {
            let mut game = room(0, RulesPreset::Classic);
            let id = game.spawn(object, Vec2::new(160.0, 144.0));
            let actor = game.world.entity_mut(id).unwrap();
            actor.set_sprite(sprite);
            actor.facing = if right { 1 } else { -1 };
            // Inherited Draw supplies literal xscale from facing, ignoring the builtin.
            actor
                .data
                .insert("image_xscale".into(), if right { 1.0 } else { -1.0 });
            for frame in 0..frames {
                game.world.entity_mut(id).unwrap().animation = f64::from(frame);
                assert_source_body(&game, id, right, &atlas);
            }
        }
    }
}

#[test]
fn default_magma_man_full_step_enters_mirrored_die_draw_in_authored_solid() {
    let atlas = Atlas::load().unwrap();
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = room(0, rules);
        // Initial authored crushing block; no actor state/health edits or midrun writes.
        game.world.set_tile(10, 9, "oHardBlock");
        let id = game.spawn("oMagmaMan", Vec2::new(160.0, 144.0));
        assert_eq!(game.world.entity(id).unwrap().health, 200);
        assert_eq!(game.world.entity(id).unwrap().facing, 1);
        game.step(&InputFrame::default());
        let actor = game.presentation().world.entity(id).unwrap();
        assert_eq!((actor.health, actor.state), (199, 99));
        assert_eq!(actor.sprite, "sMagmaManDie");
        assert_eq!(actor.facing, 1);
        assert_eq!(game.player.health, 4);
        assert_source_body(&game, id, true, &atlas);
    }
}

#[test]
fn default_magma_man_natural_lifetime_keeps_source_draw_through_die_animation() {
    let atlas = Atlas::load().unwrap();
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = room(0, rules);
        for y in 0..10 {
            game.world.set_tile(4, y, "oBlock");
            game.world.set_tile(16, y, "oBlock");
        }
        let id = game.spawn("oMagmaMan", Vec2::new(160.0, 144.0));
        for tick in 1..=200 {
            game.step(&InputFrame::default());
            let actor = game
                .world
                .entity(id)
                .expect("source seed0 patrol avoids transformation");
            assert_eq!(actor.health, 200 - tick);
            assert_eq!(game.player.health, 4);
        }
        let actor = game.presentation().world.entity(id).unwrap();
        assert_eq!((actor.health, actor.state), (0, 99));
        assert_eq!(actor.sprite, "sMagmaManDie");
        assert_source_body(&game, id, actor.facing > 0, &atlas);
    }
}

#[test]
fn bat_and_fish_directional_art_remains_independent_of_source_left() {
    let atlas = Atlas::load().unwrap();
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for right in [false, true] {
            let mut game = room(16, rules);
            game.player.pos = Vec2::new(if right { 240.0 } else { 40.0 }, 152.0);
            game.player.previous_pos = game.player.pos;
            let id = game.spawn("oBat", Vec2::new(112.0, 96.0));
            for _ in 0..3 {
                game.step(&InputFrame::default());
            }
            let actor = game.presentation().world.entity(id).unwrap();
            assert_eq!(actor.sprite, if right { "sBatRight" } else { "sBatLeft" });
            assert_eq!(actor.facing, if right { 1 } else { -1 });
            assert_eq!(actor.value("source_facing"), 0.0);
            assert_source_body(&game, id, false, &atlas);
        }
        for (seed, right) in [(0, false), (16, true)] {
            let mut game = room(seed, rules);
            for y in 4..=8 {
                for x in 4..=10 {
                    game.world.set_tile(x, y, "oWaterSwim");
                }
            }
            let id = game.spawn("oPiranha", Vec2::new(96.0, 96.0));
            for _ in 0..3 {
                game.step(&InputFrame::default());
            }
            let actor = game.presentation().world.entity(id).unwrap();
            assert_eq!(
                actor.sprite,
                if right {
                    "sPiranhaRight"
                } else {
                    "sPiranhaLeft"
                }
            );
            assert_eq!(actor.facing, if right { 1 } else { -1 });
            assert_source_body(&game, id, false, &atlas);
        }
    }
}

#[test]
fn default_ufo_keeps_inherited_left_during_ordinary_steps() {
    let atlas = Atlas::load().unwrap();
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = room(42, rules);
        let id = game.spawn("oUFO", Vec2::new(160.0, 80.0));
        assert_eq!(game.world.entity(id).unwrap().value("source_facing"), 0.0);
        for _ in 0..4 {
            game.step(&InputFrame::default());
            assert_source_body(&game, id, false, &atlas);
        }
    }
}

#[test]
fn enemy_own_draw_overrides_and_non_enemy_scale_remain_distinct() {
    for (object, name) in [
        ("oJaws", "sJawsRight"),
        ("oYetiKing", "sYetiKingRight"),
        ("oTombLord", "sTombLordLeft"),
    ] {
        let mut actor = Entity::new(1, object, Vec2::new(80.0, 80.0));
        actor.facing = 1;
        actor.data.insert("image_xscale".into(), -1.0);
        assert!(!entity_mirrored(&actor, name));
    }
    let mut item = Entity::new(1, "oDamsel", Vec2::new(80.0, 80.0));
    item.data.insert("image_xscale".into(), -1.0);
    assert!(entity_mirrored(&item, "sDamselLeft"));
    assert_eq!(entity_draw_x("oDamsel", 80.0, true), 80.0);
}
