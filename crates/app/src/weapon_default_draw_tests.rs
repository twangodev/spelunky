//! Ordinary Pistol use births a parentless muzzle flash with default Draw.
//! All fixture writes precede input; rendering must retain the authored art.
use super::*;
use spelunky_core::{Area, GameEvent, InputFrame, RulesPreset, World};

fn ordinary_pistol_flash(rules: RulesPreset, left: bool) -> (Game, u64) {
    let mut game = Game::shell(9, rules);
    game.world = World::new(20, 16, Area::Mines, 1);
    for x in 0..20 {
        game.world.set_tile(x, 10, "oHardBlock");
    }
    game.player.pos = Vec2::new(88.0, 152.0);
    game.player.previous_pos = game.player.pos;
    let gun = game.spawn("oPistol", Vec2::new(88.0, 156.0));
    for _ in 0..2 {
        game.step(&InputFrame::default());
    }
    assert!(game.player.grounded);
    game.step(&InputFrame {
        down: true,
        ..Default::default()
    });
    game.step(&InputFrame {
        down: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert_eq!(game.player.held, Some(gun), "SOURCE_ORDINARY_PISTOL_PICKUP");
    assert_eq!(game.world.entity(gun).unwrap().value("cost"), 0.0);
    game.step(&InputFrame {
        attack_released: true,
        ..Default::default()
    });
    game.step(&InputFrame {
        left,
        right: !left,
        ..Default::default()
    });
    game.step(&InputFrame::default());
    assert_eq!(game.player.facing, if left { -1 } else { 1 });
    game.step(&InputFrame {
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert_eq!(game.player.health, 4.0);
    assert_eq!(game.player.held, Some(gun));
    assert!(game.events.iter().any(|event| matches!(event,
        GameEvent::Sound { name, .. } if name == "sndShotgun")));
    let object = if left {
        "oShotgunBlastLeft"
    } else {
        "oShotgunBlastRight"
    };
    let flash = game
        .presentation()
        .world
        .entities
        .iter()
        .find(|entity| entity.alive && entity.engine_active && entity.object == object)
        .expect("SOURCE_ORDINARY_PISTOL_MUZZLE_FLASH");
    let definition = catalog::object(object).unwrap();
    assert!(definition.parent.is_empty());
    assert!(definition.visible);
    assert_eq!(definition.depth, 0);
    assert_eq!(flash.image_speed(), 0.8);
    assert_eq!(flash.data.get("image_xscale"), None);
    assert_eq!(
        flash.facing, 1,
        "port default facing is not source image_xscale"
    );
    assert_eq!(flash.previous_pos, flash.pos);
    let source_player = &game.presentation().player;
    assert_eq!(
        flash.pos,
        Vec2::new(
            source_player.pos.x + if left { -9.0 } else { 8.0 },
            source_player.pos.y + 1.0
        )
    );
    let id = flash.id;
    (game, id)
}

fn assert_default_flash_draw(left: bool) {
    let atlas = Atlas::load().unwrap();
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (game, id) = ordinary_pistol_flash(rules, left);
        let draw = game.presentation();
        let flash = draw.world.entity(id).unwrap();
        let frame = atlas.sprite(&flash.sprite, flash.animation).unwrap();
        let hash = game.state_hash();
        let draw_hash = draw.state_hash();
        for alpha in [0.0, 0.25, 0.5, 0.75, 1.0] {
            let camera = draw.previous_camera.lerp(draw.camera, alpha);
            let expected = [
                (flash.pos.x - camera.x) as f32 - frame.origin[0],
                (flash.pos.y - camera.y) as f32 - frame.origin[1],
                frame.size[0],
                frame.size[1],
            ];
            let quads = game_scene(&game, alpha, false, &atlas);
            let quad = quads
                .iter()
                .find(|quad| quad.uv == frame.uv && quad.rect == expected)
                .expect("SOURCE_PARENTLESS_MUZZLE_DEFAULT_DRAW_XSCALE");
            assert_eq!(quad.color, WHITE);
            assert_eq!(quad.rotation, 0.0);
            assert_eq!(quad.blend, QuadBlend::Alpha);
            assert_eq!(game.state_hash(), hash);
            assert_eq!(game.presentation().state_hash(), draw_hash);
        }
    }
}

#[test]
fn ordinary_left_pistol_flash_keeps_its_authored_default_draw_orientation() {
    assert_default_flash_draw(true);
}

#[test]
fn ordinary_right_pistol_flash_default_draw_remains_unchanged() {
    assert_default_flash_draw(false);
}
