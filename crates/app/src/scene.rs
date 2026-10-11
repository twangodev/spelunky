//! Builds a presentation snapshot without advancing simulation or animation.
use spelunky_core::{Area, Entity, Game, PlayerState, TICK_RATE, Vec2, catalog};
use std::collections::BTreeMap;

use crate::{
    atlas::Atlas,
    renderer::{DrawQuad, QuadBlend},
};

const WHITE: [f32; 4] = [1.0; 4];

pub fn rect(quads: &mut Vec<DrawQuad>, rect: [f32; 4], color: [f32; 4]) {
    quads.push(DrawQuad {
        rect,
        uv: [0.0; 4],
        color,
        ..DrawQuad::default()
    });
}

/// The source bitmap font starts at ASCII space and ends at uppercase Z.
pub fn text(
    quads: &mut Vec<DrawQuad>,
    atlas: &Atlas,
    value: &str,
    x: f32,
    y: f32,
    scale: f32,
    color: [f32; 4],
) {
    font(quads, atlas, value, x, y, scale, color, "sFontSmall");
}

/// Classic's two independent centered message lines. Their lifetime belongs
/// to the simulation; drawing at 240 Hz must not consume the source timer.
pub fn source_message(
    quads: &mut Vec<DrawQuad>,
    atlas: &Atlas,
    first: &str,
    second: &str,
    tutorial: bool,
) {
    for (value, y, color) in [
        (first, 216.0, WHITE),
        (
            second,
            224.0,
            if tutorial {
                WHITE
            } else {
                [1.0, 1.0, 0.0, 1.0]
            },
        ),
    ] {
        text(
            quads,
            atlas,
            value,
            ((320.0 - value.chars().count() as f32 * 8.0) / 2.0).ceil(),
            y,
            1.0,
            color,
        );
    }
}

fn intro_narrative(
    quads: &mut Vec<DrawQuad>,
    atlas: &Atlas,
    draw_status: i8,
    fade_level: f64,
    lines: &[String; 3],
) {
    // oIntro Draw places the story over the fade, so it remains legible while
    // the authored desert and travelling actor are still completely black.
    cinematic_fade(quads, fade_level);
    for (index, value) in lines.iter().enumerate() {
        if draw_status > index as i8 {
            text(
                quads,
                atlas,
                value,
                ((320.0 - value.chars().count() as f32 * 8.0) / 2.0).ceil(),
                100.0 + index as f32 * 16.0,
                1.0,
                WHITE,
            );
        }
    }
}

fn title_opening_overlay(quads: &mut Vec<DrawQuad>, atlas: &Atlas, stage: u8, darkness: f64) {
    // oScreen Begin Step retains this alpha for the presentation credit too.
    let alpha = darkness.clamp(0.0, 1.0);
    if alpha > 0.0 {
        cinematic_fade(quads, alpha);
    }
    if stage == 1 {
        text(
            quads,
            atlas,
            "DEREK YU PRESENTS",
            88.0,
            48.0,
            1.0,
            [1.0, 1.0, 1.0, alpha as f32],
        );
    }
}

/// oGame Draw reads the simulation's reveal and countup. The host supplies the
/// current keyboard/controller label for the source acknowledgment instruction.
pub fn game_over_overlay(quads: &mut Vec<DrawQuad>, atlas: &Atlas, game: &Game, hint: &str) {
    let game = game.presentation();
    if game.state != spelunky_core::RunState::Dead {
        return;
    }
    let challenge = ["hub_sun", "hub_moon", "hub_stars"]
        .iter()
        .any(|flag| game.flags.contains(*flag));
    if game.flags.contains("hub_moon") && game.data.get("timer").copied().unwrap_or(0.0) >= 0.0 {
        return;
    }
    if !challenge && !game.is_level() {
        return;
    }
    game_over_text(
        quads,
        atlas,
        game.game_over.draw_status,
        game.game_over.money_count,
        challenge,
        hub_value(game, "challenge_highscore") != 0.0,
        hint,
    );
}

fn game_over_text(
    quads: &mut Vec<DrawQuad>,
    atlas: &Atlas,
    status: u8,
    money_count: u32,
    challenge: bool,
    highscore: bool,
    hint: &str,
) {
    const YELLOW: [f32; 4] = [1.0, 1.0, 0.0, 1.0];
    if status > 0 {
        font(
            quads,
            atlas,
            if challenge { "FINISHED!" } else { "GAME OVER" },
            88.0,
            48.0,
            1.0,
            YELLOW,
            "sFont",
        );
    }
    if challenge {
        if status > 1 {
            let message = if highscore {
                "YOU SET A NEW RECORD!"
            } else {
                "BETTER LUCK NEXT TIME..."
            };
            text(
                quads,
                atlas,
                message,
                ((320.0 - message.len() as f32 * 8.0) / 2.0).ceil(),
                80.0,
                1.0,
                YELLOW,
            );
        }
    } else {
        if status > 1 {
            text(quads, atlas, "FINAL SCORE:", 88.0, 80.0, 1.0, YELLOW);
        }
        if status > 2 {
            font(
                quads,
                atlas,
                &format!("${money_count}"),
                88.0,
                88.0,
                1.0,
                WHITE,
                "sFont",
            );
            text(
                quads,
                atlas,
                hint,
                ((320.0 - hint.chars().count() as f32 * 8.0) / 2.0).ceil(),
                120.0,
                1.0,
                YELLOW,
            );
        }
    }
}

fn font(
    quads: &mut Vec<DrawQuad>,
    atlas: &Atlas,
    value: &str,
    x: f32,
    y: f32,
    scale: f32,
    color: [f32; 4],
    sprite: &str,
) {
    if !scale.is_finite() || scale <= 0.0 {
        return;
    }
    let Some(base) = atlas.sprite(sprite, 0.0) else {
        return;
    };
    let mut cursor = [x, y];
    for letter in value.chars() {
        if letter == '\n' {
            cursor[0] = x;
            cursor[1] += base.size[1] * scale;
            continue;
        }
        let letter = letter.to_ascii_uppercase();
        let index = if (' '..='Z').contains(&letter) {
            letter as u32 - 32
        } else {
            '?' as u32 - 32
        };
        if let Some(frame) = atlas.sprite(sprite, index as f64) {
            quads.push(DrawQuad {
                rect: [
                    cursor[0],
                    cursor[1],
                    frame.size[0] * scale,
                    frame.size[1] * scale,
                ],
                uv: frame.uv,
                color,
                ..DrawQuad::default()
            });
        }
        cursor[0] += base.size[0] * scale;
    }
}

fn sprite(
    quads: &mut Vec<DrawQuad>,
    atlas: &Atlas,
    name: &str,
    frame: f64,
    x: f32,
    y: f32,
    mirrored: bool,
    color: [f32; 4],
) {
    let Some(frame) = atlas.sprite(name, frame) else {
        return;
    };
    let [width, height] = frame.size;
    let mut uv = frame.uv;
    let left = if mirrored {
        uv[0] += uv[2];
        uv[2] = -uv[2];
        x + frame.origin[0] - width
    } else {
        x - frame.origin[0]
    };
    // Entity rotation is applied after this helper. Cull only when even the
    // furthest rotated corner cannot reach the viewport; an unrotated AABB
    // incorrectly drops projectiles that turn into view at a screen edge.
    let radius = frame.origin[0]
        .abs()
        .max((width - frame.origin[0]).abs())
        .hypot(frame.origin[1].abs().max((height - frame.origin[1]).abs()));
    if x + radius < 0.0 || x - radius > 320.0 || y + radius < 0.0 || y - radius > 240.0 {
        return;
    }
    quads.push(DrawQuad {
        rect: [left, y - frame.origin[1], width, height],
        uv,
        color,
        ..DrawQuad::default()
    });
}

fn interpolated(previous: Vec2, current: Vec2, alpha: f64) -> Vec2 {
    // Normal teleports reset previous positions in core. This also protects
    // imported snapshots and abrupt camera/level resets from sweeping the view.
    if previous.distance(current) > 64.0 {
        current
    } else {
        previous.lerp(current, alpha)
    }
}

fn screen(previous: Vec2, current: Vec2, camera: Vec2, alpha: f64, pixel_snap: bool) -> [f32; 2] {
    let point = interpolated(previous, current, alpha);
    let mut result = [(point.x - camera.x) as f32, (point.y - camera.y) as f32];
    if pixel_snap {
        result = result.map(f32::round);
    }
    result
}

fn entity_sprite<'a>(atlas: &Atlas, entity: &'a Entity) -> Option<&'a str> {
    if atlas.sprite(&entity.sprite, entity.animation).is_some() {
        return Some(&entity.sprite);
    }
    catalog::sprite(&entity.object)
}

fn entity_position(game: &Game, entity: &Entity, alpha: f64) -> Vec2 {
    let attached = (entity.held && game.player.held == Some(entity.id))
        || matches!(
            entity.object.as_str(),
            "oWhip" | "oWhipPre" | "oMachetePre" | "oMattockPre" | "oSlash" | "oMattockHit"
        );
    if attached {
        // Source Step events attach these objects to the character, including
        // their very first frame. Interpolating their independently initialized
        // previous positions detaches a new attack and sweeps held items through
        // the body when facing changes. Only the owner's motion is continuous.
        let owner = interpolated(game.player.previous_pos, game.player.pos, alpha);
        Vec2::new(
            owner.x + entity.pos.x - game.player.pos.x,
            owner.y + entity.pos.y - game.player.pos.y,
        )
    } else {
        interpolated(entity.previous_pos, entity.pos, alpha)
    }
}

fn entity_draw_x(object: &str, x: f32, mirrored: bool) -> f32 {
    // oEnemy's inherited Draw event compensates by exactly 16 original
    // pixels. A raw origin flip would move its top-left-origin sprite into
    // the neighboring tile. Other objects retain their own origin semantics.
    if mirrored && catalog::is_a(object, "oEnemy") {
        x + 16.0
    } else {
        x
    }
}

fn entity_mirrored(entity: &Entity, name: &str) -> bool {
    // These objects override oEnemy's Draw event and use their already
    // directional sprite without the inherited horizontal flip.
    if matches!(entity.object.as_str(), "oJaws" | "oYetiKing" | "oTombLord") {
        false
    } else if entity.object == "oPDummy6" {
        // Its own Draw event unconditionally assigns image_xscale = -1.
        true
    } else if matches!(entity.object.as_str(), "oPDummy" | "oPDummy2" | "oPDummy3") {
        // These Draw events assign the scale from facing on every draw.
        entity.facing > 0
    } else if entity.data.contains_key("image_xscale") {
        entity.value("image_xscale") < 0.0
    } else {
        (entity.object == "oAlien" || name.ends_with("Left") || name.ends_with('L'))
            && entity.facing > 0
    }
}

fn entity_draw_extras(
    layer: &mut Vec<DrawQuad>,
    game: &Game,
    entity: &Entity,
    atlas: &Atlas,
    x: f32,
    y: f32,
) {
    if matches!(entity.object.as_str(), "oPDummy" | "oPDummy2") {
        // These source actors have their own Draw event. Equipment follows the
        // cinematic actor rather than the frozen gameplay character.
        let exiting = matches!(
            entity.sprite.as_str(),
            "sPExit" | "sDamselExit" | "sTunnelExit"
        );
        if game.player.has("Jetpack") && exiting {
            sprite(layer, atlas, "sJetpackBack", 0.0, x, y, false, WHITE);
        }
        if !exiting {
            // The gameplay world has been replaced. Its old held ID must not
            // resolve to an unrelated authored cinematic instance.
            let held = entity.text.get("pickup_item").map(String::as_str);
            let name = match held.unwrap_or("") {
                "Rock" | "oRock" => "sRock",
                "Jar" | "oJar" => "sJar",
                "Skull" | "oSkull" => "sSkull",
                "Fish Bone" | "oFishBone" => "sFishBone",
                "Arrow" | "oArrow" => "sArrowRight",
                "Machete" | "oMachete" => "sMacheteRight",
                "Mattock" | "oMattock" => "sMattockRight",
                "Mattock Head" | "oMattockHead" => "sMattockHead",
                "Pistol" | "oPistol" => "sPistolRight",
                "Web Cannon" | "oWebCannon" => "sWebCannonR",
                "Teleporter" | "oTeleporter" => "sTeleporter",
                "Shotgun" | "oShotgun" => "sShotgunRight",
                "Bow" | "oBow" => "sBowRight",
                "Flare" | "oFlare" => "sFlare",
                "Sceptre" | "oSceptre" => "sSceptreRight",
                "Key" | "oKey" => "sKeyRight",
                _ => "",
            };
            if !name.is_empty() {
                sprite(
                    layer,
                    atlas,
                    name,
                    entity.animation,
                    x + 4.0,
                    y + 2.0,
                    false,
                    WHITE,
                );
            }
        }
    }
    if entity.object == "oJaws" {
        let damage = if entity.health < 10 {
            3
        } else if entity.health < 20 {
            2
        } else {
            1
        };
        let (side, offset) = match entity.sprite.as_str() {
            "sJawsLeft" => ("L", 16.0),
            "sJawsRight" => ("R", -48.0),
            _ => ("", 0.0), // The turn animation already contains the body.
        };
        if !side.is_empty() {
            sprite(
                layer,
                atlas,
                &format!("sJawsBody{damage}{side}"),
                0.0,
                x + offset,
                y,
                false,
                WHITE,
            );
        }
    }
    if matches!(entity.object.as_str(), "oShopkeeper" | "oShopkeeper2")
        && entity.value("has_gun") > 0.0
        && !matches!(entity.state, 0 | 5)
    {
        let right = entity.facing > 0;
        sprite(
            layer,
            atlas,
            if right {
                "sShotgunRight"
            } else {
                "sShotgunLeft"
            },
            0.0,
            x + if right { 10.0 } else { 6.0 },
            y + 10.0,
            false,
            WHITE,
        );
    }
    if catalog::is_a(&entity.object, "oItem") && entity.value("cost") > 0.0 {
        // Source increments cimg in Draw at its fixed 30 Hz. Keep this visual
        // animation on simulation time when rendering at higher refresh rates.
        sprite(
            layer,
            atlas,
            "sSmallCollect",
            entity.age.saturating_sub(1) as f64,
            x,
            y - 12.0,
            false,
            WHITE,
        );
    }
    if entity.object == "oDice"
        && entity.value("rolled") == 0.0
        && game.player.data.get("bet").copied().unwrap_or(0.0) > 0.0
    {
        sprite(
            layer,
            atlas,
            "sRedArrowDown",
            0.0,
            x,
            y - 12.0,
            false,
            WHITE,
        );
    }
}

/// Alpha interpolates positions only. Sprite frames and all HUD values are
/// simulation state, so presentation never changes gameplay or animation speed.
pub fn game_scene(game: &Game, alpha: f64, pixel_snap: bool, atlas: &Atlas) -> Vec<DrawQuad> {
    let game = game.presentation();
    let alpha = if game.flags.contains("render_discontinuity") {
        1.0
    } else if alpha.is_finite() {
        alpha.clamp(0.0, 1.0)
    } else {
        1.0
    };
    let camera = interpolated(game.previous_camera, game.camera, alpha);
    // A cinematic uses the authored room's world, but its actors and Draw
    // controller replace the gameplay character, HUD and lighting pass.
    let cinematic = game
        .ending
        .as_ref()
        .filter(|ending| ending.stage != spelunky_core::ending::EndingStage::Exit);
    let mut quads = Vec::with_capacity(1000);
    background(&mut quads, game, camera, pixel_snap, atlas);
    let mut layers: Vec<(i32, u64, Vec<DrawQuad>)> = Vec::new();
    let terrain_sprites: BTreeMap<(usize, usize), (&str, f64)> = game
        .world
        .entities
        .iter()
        .filter(|entity| {
            entity.alive
                && entity.engine_active
                && entity.object == "oTerrainMetadata"
                && entity.pos.x >= 0.0
                && entity.pos.y >= 0.0
        })
        .filter_map(|entity| {
            entity.text.get("terrain_sprite").map(|name| {
                (
                    (
                        (entity.pos.x / 16.0).floor() as usize,
                        (entity.pos.y / 16.0).floor() as usize,
                    ),
                    (name.as_str(), entity.animation),
                )
            })
        })
        .collect();
    for (index, tile) in game.world.tiles.iter().enumerate() {
        if tile.object.is_empty() || !tile.engine_active {
            continue;
        }
        let tile_x = index % game.world.width.max(1);
        let tile_y = index / game.world.width.max(1);
        let override_sprite = terrain_sprites.get(&(tile_x, tile_y)).copied();
        let Some(name) = override_sprite
            .map(|(name, _)| name)
            .or_else(|| catalog::sprite(&tile.object))
            .or_else(|| {
                (tile.object == "oSolid").then_some(match game.world.area {
                    Area::Mines => "sBrick",
                    Area::Jungle => "sLush",
                    Area::Ice => "sDark",
                    Area::Temple | Area::Boss => "sTemple",
                })
            })
        else {
            continue;
        };
        let position = Vec2::new(tile_x as f64 * 16.0, tile_y as f64 * 16.0);
        let [x, y] = screen(position, position, camera, alpha, pixel_snap);
        if x < -64.0 || y < -64.0 || x > 320.0 || y > 240.0 {
            continue;
        }
        let definition = catalog::object(&tile.object);
        if definition.is_some_and(|def| !def.visible) {
            continue;
        }
        let mut layer = Vec::with_capacity(1);
        sprite(
            &mut layer,
            atlas,
            name,
            override_sprite.map_or(0.0, |(_, frame)| frame),
            x,
            y,
            false,
            WHITE,
        );
        layers.push((definition.map_or(100, |def| def.depth), index as u64, layer));
    }
    for metadata in game.world.entities.iter().filter(|entity| {
        entity.alive && entity.engine_active && entity.object == "oTerrainMetadata"
    }) {
        let frozen = metadata.value("frozen_caveman") != 0.0;
        let embedded = metadata.value("embedded_treasure") != 0.0;
        if !frozen && !embedded {
            continue;
        }
        let (name, position, depth) = if frozen {
            // oIce Create places oFrozenCaveman at the tile's top left. Its
            // depth 100 draws it above the translucent ice at depth 110.
            (
                "sFrozenCaveman",
                Vec2::new(metadata.pos.x - 8.0, metadata.pos.y - 8.0),
                100,
            )
        } else {
            let Some(object) = metadata.text.get("treasure") else {
                continue;
            };
            let object = match object.as_str() {
                "Big Ruby" => "oRubyBig",
                "Big Sapphire" => "oSapphireBig",
                "Big Emerald" => "oEmeraldBig",
                "Ruby" => "oRuby",
                "Sapphire" => "oSapphire",
                "Emerald" => "oEmerald",
                "Diamond" => "oDiamond",
                name => name,
            };
            let treasure = catalog::is_a(object, "oTreasure");
            let spectacles = game.player.has("Spectacles");
            if !spectacles && !(treasure && game.player.has("UdjatEye")) {
                continue;
            }
            let Some(name) = catalog::sprite(object) else {
                continue;
            };
            (
                name,
                Vec2::new(
                    metadata.pos.x + metadata.value("treasure_dx"),
                    metadata.pos.y + metadata.value("treasure_dy"),
                ),
                if treasure { 0 } else { 51 },
            )
        };
        let [x, y] = screen(position, position, camera, alpha, pixel_snap);
        let mut layer = Vec::with_capacity(1);
        sprite(&mut layer, atlas, name, 0.0, x, y, false, WHITE);
        layers.push((depth, 1_000_000 + metadata.id, layer));
    }
    for entity in game
        .world
        .entities
        .iter()
        .filter(|entity| entity.alive && entity.engine_active)
    {
        if entity.data.get("visible") == Some(&0.0)
            || entity.data.get("hidden") == Some(&1.0)
            || entity.object == "oTerrainMetadata"
        {
            continue;
        }
        let definition = catalog::object(&entity.object);
        if definition.is_some_and(|def| !def.visible) && !entity.data.contains_key("visible") {
            continue;
        }
        let Some(name) = entity_sprite(atlas, entity) else {
            continue;
        };
        let position = entity_position(game, entity, alpha);
        let [x, y] = screen(position, position, camera, alpha, pixel_snap);
        let mut layer = Vec::with_capacity(1);
        if entity.object == "oHintHand" && (game.player.pos.x - (entity.pos.x + 8.0)).abs() < 8.0 {
            // oHintHand Draw uses room coordinates, centered in the right
            // 320-pixel title view. Camera translation keeps the text attached
            // to that source view while walking between the title screens.
            let message = "PRESS UP TO ENTER!";
            let text_x = 320.0 + ((320.0 - message.len() as f64 * 8.0) / 2.0).ceil();
            let [text_x, text_y] = screen(
                Vec2::new(text_x, 216.0),
                Vec2::new(text_x, 216.0),
                camera,
                alpha,
                pixel_snap,
            );
            text(&mut layer, atlas, message, text_x, text_y, 1.0, WHITE);
        }
        if entity.object == "oGenerationDecoration" {
            let region =
                if entity.data.contains_key("source_x") || entity.data.contains_key("source_y") {
                    let crop = [
                        entity.value("source_x"),
                        entity.value("source_y"),
                        entity.data.get("source_w").copied().unwrap_or(16.0),
                        entity.data.get("source_h").copied().unwrap_or(16.0),
                    ];
                    if crop.iter().all(|value| value.is_finite() && *value >= 0.0) {
                        atlas.region(name, crop.map(|value| value as u32))
                    } else {
                        None
                    }
                } else if name == "bgAlienShip" {
                    atlas.region(
                        name,
                        [
                            if entity.value("variant") > 0.0 { 32 } else { 0 },
                            0,
                            32,
                            64,
                        ],
                    )
                } else {
                    atlas.sprite(name, 0.0)
                };
            if let Some(frame) = region {
                layer.push(DrawQuad {
                    rect: [x, y, frame.size[0], frame.size[1]],
                    uv: frame.uv,
                    color: WHITE,
                    ..DrawQuad::default()
                });
                let depth = entity
                    .data
                    .get("depth")
                    .copied()
                    .unwrap_or(if name == "bgKaliBody" {
                        10001.0
                    } else if name == "bgLadyXoc" {
                        99.0
                    } else if name == "bgDiceSign" {
                        9004.0
                    } else {
                        9005.0
                    }) as i32;
                layers.push((depth, 1_000_000 + entity.id, layer));
            }
            continue;
        }
        let mirror = entity_mirrored(entity, name);
        let alpha = entity
            .data
            .get("image_alpha")
            .copied()
            .unwrap_or(1.0)
            .clamp(0.0, 1.0) as f32;
        if matches!(entity.object.as_str(), "oPDummy" | "oPDummy2")
            && game.player.has("Jetpack")
            && !matches!(
                entity.sprite.as_str(),
                "sPExit" | "sDamselExit" | "sTunnelExit"
            )
        {
            sprite(
                &mut layer,
                atlas,
                "sJetpackRight",
                0.0,
                x - 4.0,
                y - 1.0,
                false,
                WHITE,
            );
        }
        sprite(
            &mut layer,
            atlas,
            name,
            entity.animation,
            entity_draw_x(&entity.object, x, mirror),
            y,
            mirror,
            [1.0, 1.0, 1.0, alpha],
        );
        if let Some(quad) = layer.last_mut() {
            let rotation = entity.value("rotation");
            if rotation.is_finite() {
                // Core velocity.atan2 uses screen-space y-down coordinates,
                // already the renderer's positive clockwise convention.
                quad.rotation = rotation as f32;
                quad.pivot = [
                    entity_draw_x(&entity.object, x, mirror) - quad.rect[0],
                    y - quad.rect[1],
                ];
            }
        }
        entity_draw_extras(&mut layer, game, entity, atlas, x, y);
        let depth = entity.data.get("depth").map_or_else(
            || definition.map_or(60, |def| def.depth),
            |depth| *depth as i32,
        );
        layers.push((depth, 1_000_000 + entity.id, layer));
    }
    let player = &game.player;
    if cinematic.is_none()
        && player.data.get("hidden") != Some(&1.0)
        && player.data.get("blink_toggle") != Some(&1.0)
    {
        let [x, y] = screen(player.previous_pos, player.pos, camera, alpha, pixel_snap);
        let mut layer = Vec::with_capacity(3);
        let jetpack = player.has("Jetpack") || player.has("jetpack");
        let climbing = player.state == PlayerState::Climbing
            || matches!(
                player.sprite.as_str(),
                "sPExit" | "sDamselExit" | "sTunnelExit"
            );
        if (player.has("Cape") || player.has("cape"))
            && !game
                .world
                .entities
                .iter()
                .any(|entity| entity.alive && entity.engine_active && entity.object == "oCape")
        {
            let right = player.facing > 0;
            let open = player.data.get("cape_open").copied().unwrap_or(0.0) > 0.0;
            let name = if climbing {
                "sCapeBack"
            } else if open {
                if right { "sCapeUR" } else { "sCapeUL" }
            } else if player.vel.x.abs() > 0.0 {
                if right { "sCapeRight" } else { "sCapeLeft" }
            } else if right {
                "sCapeDR"
            } else {
                "sCapeDL"
            };
            let mut cape = Vec::with_capacity(1);
            sprite(
                &mut cape,
                atlas,
                name,
                player.animation,
                if climbing {
                    x
                } else {
                    x - if right { 4.0 } else { -4.0 }
                },
                if climbing { y + 4.0 } else { y - 2.0 },
                false,
                WHITE,
            );
            layers.push((if climbing { 0 } else { 100 }, u64::MAX - 1, cape));
        }
        if player.data.get("parachute_open").copied().unwrap_or(0.0) > 0.0
            && !game
                .world
                .entities
                .iter()
                .any(|entity| entity.alive && entity.engine_active && entity.object == "oParachute")
        {
            let mut canopy = Vec::with_capacity(1);
            sprite(
                &mut canopy,
                atlas,
                "sParachute",
                0.0,
                x - 8.0,
                y - 16.0,
                false,
                WHITE,
            );
            layers.push((900, u64::MAX - 2, canopy));
        }
        if jetpack && !climbing {
            sprite(
                &mut layer,
                atlas,
                if player.facing > 0 {
                    "sJetpackRight"
                } else {
                    "sJetpackLeft"
                },
                0.0,
                x - if player.facing > 0 { 4.0 } else { -4.0 },
                y - 1.0,
                false,
                WHITE,
            );
        }
        let name = if atlas.sprite(&player.sprite, player.animation).is_some() {
            player.sprite.as_str()
        } else {
            "sStandLeft"
        };
        sprite(
            &mut layer,
            atlas,
            name,
            player.animation,
            x,
            y,
            player.facing > 0,
            WHITE,
        );
        if player.data.get("bow_armed").copied().unwrap_or(0.0) > 0.0 {
            let bomb = player.data.get("bow_bomb_armed").copied().unwrap_or(0.0) > 0.0;
            let right = player.facing > 0;
            let name = match (bomb, right) {
                (false, false) => "sArrowLeft",
                (false, true) => "sArrowRight",
                (true, false) => "sBombArrowLeft",
                (true, true) => "sBombArrowRight",
            };
            sprite(
                &mut layer,
                atlas,
                name,
                if bomb && player.data.get("bow_arrow_toggle").copied().unwrap_or(0.0) == 0.0 {
                    1.0
                } else {
                    0.0
                },
                x + if right { 4.0 } else { -4.0 },
                y + if bomb { 2.0 } else { 1.0 },
                false,
                WHITE,
            );
        }
        if jetpack && climbing {
            sprite(&mut layer, atlas, "sJetpackBack", 0.0, x, y, false, WHITE);
        }
        let depth = player.data.get("depth").copied().unwrap_or_else(|| {
            if player.data.get("burning").copied().unwrap_or(0.0) > 0.0 {
                999.0
            } else {
                50.0
            }
        }) as i32;
        layers.push((depth, u64::MAX, layer));
    }
    // GameMaker draws larger depths first. Stable IDs break ties deterministically.
    layers.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    for (_, _, layer) in layers {
        quads.extend(layer);
    }
    if let Some(ending) = cinematic {
        use spelunky_core::ending::EndingStage;
        let fade = interpolate_fade(ending.previous_fade_level, ending.fade_level, alpha);
        match ending.stage {
            EndingStage::Landing | EndingStage::Summary => ending_summary(
                &mut quads,
                atlas,
                ending.draw_status,
                ending.money_count,
                ending.source_time_ms,
                game.data.get("kills").copied().unwrap_or(0.0).max(0.0) as u32,
                game.data.get("damsels").copied().unwrap_or(0.0).max(0.0) as u32,
                ending.fade_out,
                fade,
            ),
            EndingStage::Credits => {
                ending_credits(&mut quads, atlas, ending.credits_page);
                if ending.fade_in || ending.fade_out {
                    cinematic_fade(&mut quads, fade);
                }
            }
            EndingStage::Transition => transition_text(
                &mut quads,
                atlas,
                game,
                ending.draw_loot,
                ending.is_loot,
                ending.is_kills,
                ending.money_count,
                ending.source_time_ms,
                ending.level_time_ms,
                ending.completed_level,
                &ending.completed_name,
            ),
            _ => {}
        }
    } else {
        if darkness_enabled(game) {
            darkness(&mut quads, game, camera, alpha);
        }
        hud(&mut quads, game, camera, atlas);
        if let Some(ending) = game.ending.as_ref().filter(|ending| {
            ending.kind == spelunky_core::ending::EndingKind::TitleCredits
                && ending.stage == spelunky_core::ending::EndingStage::Exit
                && ending.fade_out
        }) {
            cinematic_fade(
                &mut quads,
                interpolate_fade(ending.previous_fade_level, ending.fade_level, alpha),
            );
        }
    }
    if let Some(opening) = &game.opening {
        match opening.kind {
            spelunky_core::opening::OpeningKind::Intro => intro_narrative(
                &mut quads,
                atlas,
                opening.draw_status,
                interpolate_fade(opening.previous_fade_level, opening.fade_level, alpha),
                &opening.lines,
            ),
            spelunky_core::opening::OpeningKind::Title => {
                title_opening_overlay(
                    &mut quads,
                    atlas,
                    opening.stage,
                    interpolate_fade(opening.previous_darkness, opening.darkness, alpha),
                );
            }
        }
    }
    quads
}

/// Source oEnd3/oEndCustom Draw. Simulation owns every reveal and countup.
fn ending_summary(
    quads: &mut Vec<DrawQuad>,
    atlas: &Atlas,
    status: u8,
    money: u32,
    source_time_ms: u64,
    kills: u32,
    saves: u32,
    fade_out: bool,
    fade: f64,
) {
    const YELLOW: [f32; 4] = [1.0, 1.0, 0.0, 1.0];
    if status > 0 {
        font(
            quads,
            atlas,
            "YOU MADE IT!",
            64.0,
            32.0,
            1.0,
            YELLOW,
            "sFont",
        );
    }
    if status > 1 {
        text(quads, atlas, "FINAL SCORE:", 64.0, 64.0, 1.0, YELLOW);
    }
    if status > 2 {
        font(
            quads,
            atlas,
            &format!("${money}"),
            64.0,
            72.0,
            1.0,
            WHITE,
            "sFont",
        );
    }
    if status > 4 {
        let seconds = source_time_ms / 1000;
        text(quads, atlas, "TIME: ", 64.0, 96.0, 1.0, YELLOW);
        text(
            quads,
            atlas,
            &format!("{}:{:02}", seconds / 60, seconds % 60),
            120.0,
            96.0,
            1.0,
            WHITE,
        );
    }
    if status > 5 {
        text(quads, atlas, "KILLS: ", 64.0, 104.0, 1.0, YELLOW);
        text(quads, atlas, &kills.to_string(), 120.0, 104.0, 1.0, WHITE);
    }
    if status > 6 {
        text(quads, atlas, "SAVES: ", 64.0, 112.0, 1.0, YELLOW);
        text(quads, atlas, &saves.to_string(), 120.0, 112.0, 1.0, WHITE);
    }
    if fade_out {
        cinematic_fade(quads, fade);
    }
    // The hero inscription draws after the black fade in the original event.
    if status == 8 {
        let message = "YOU SHALL BE REMEMBERED AS A HERO.";
        text(
            quads,
            atlas,
            message,
            ((320.0 - message.len() as f32 * 8.0) / 2.0).ceil(),
            116.0,
            1.0,
            WHITE,
        );
    }
}

// Source opacity advances at 30 Hz. Only presentation reads the two actual
// snapshots; source alarms, stage changes and draw reveals stay authoritative.
fn interpolate_fade(previous: f64, current: f64, alpha: f64) -> f64 {
    previous + (current - previous) * alpha
}

fn cinematic_fade(quads: &mut Vec<DrawQuad>, fade: f64) {
    rect(
        quads,
        [0.0, 0.0, 320.0, 240.0],
        [0.0, 0.0, 0.0, fade.clamp(0.0, 1.0) as f32],
    );
}

/// Both source credits controllers use identical text, coordinates and fonts.
fn ending_credits(quads: &mut Vec<DrawQuad>, atlas: &Atlas, page: u8) {
    const YELLOW: [f32; 4] = [1.0, 1.0, 0.0, 1.0];
    let mut line = |value: &str, x: f32, y: f32, yellow: bool| {
        text(
            quads,
            atlas,
            value,
            x,
            y,
            1.0,
            if yellow { YELLOW } else { WHITE },
        );
    };
    match page {
        1 => font(quads, atlas, "SPELUNKY", 16.0, 16.0, 1.0, YELLOW, "sFont"),
        2 => {
            line("A GAME BY", 16.0, 16.0, true);
            line("DEREK YU", 64.0, 32.0, false);
        }
        3 => {
            line("PLATFORM ENGINE", 16.0, 16.0, true);
            line("MARTIN PIECYK", 16.0, 24.0, false);
            line("SOUND EFFECTS MADE USING", 16.0, 40.0, true);
            line("DR PETTER'S SFXR", 16.0, 48.0, false);
            line("SCREEN SCALING CODE", 16.0, 64.0, true);
            line("CHEVYRAY", 16.0, 72.0, false);
        }
        4 => {
            line("MUSIC BY", 16.0, 16.0, true);
            line("GEORGE BUZINKAI", 32.0, 32.0, false);
            line("JONATHAN PERRY", 32.0, 48.0, false);
        }
        5 => {
            line("BETA TESTING BY", 16.0, 16.0, true);
            for (i, value) in [
                "ANNABELLE K.",
                "BENZIDO",
                "CHUTUP",
                "CORPUS",
                "GENERALVALTER",
                "GUERT",
                "GRAHAM GORING",
                "HAOWAN",
                "HIDEOUS",
                "INANE",
            ]
            .iter()
            .enumerate()
            {
                line(value, 16.0, 24.0 + i as f32 * 8.0, false);
            }
            for (i, value) in [
                "INCREPARE",
                "KAO",
                "MARK JOHNS",
                "MELLY",
                "PAUL ERES",
                "SUPER JOE",
                "TANTAN",
                "TEAM QUIGGAN",
                "TERRY",
                "XION",
                "ZAPHOS",
            ]
            .iter()
            .enumerate()
            {
                line(value, 128.0, 24.0 + i as f32 * 8.0, false);
            }
        }
        6 => {
            line("THANKS FOR PLAYING!", 16.0, 16.0, true);
            line("SEE YOU NEXT ADVENTURE!", 32.0, 32.0, false);
        }
        _ => {}
    }
}

fn transition_text(
    quads: &mut Vec<DrawQuad>,
    atlas: &Atlas,
    game: &Game,
    draw_loot: i8,
    is_loot: bool,
    is_kills: bool,
    money_count: u32,
    source_time_ms: u64,
    level_time_ms: u64,
    completed_level: u32,
    completed_name: &str,
) {
    const YELLOW: [f32; 4] = [1.0, 1.0, 0.0, 1.0];
    let mut centered = |value: &str, y: f32, color| {
        text(
            quads,
            atlas,
            value,
            ((320.0 - value.len() as f32 * 8.0) / 2.0).ceil(),
            y,
            1.0,
            color,
        );
    };
    if let Some(tunnel) = game
        .world
        .entities
        .iter()
        .find(|entity| entity.alive && entity.engine_active && entity.object == "oTunnelMan")
    {
        match tunnel.value("talk") as u8 {
            1 => {
                centered("HEY THERE! I'M THE TUNNEL MAN!", 208.0, WHITE);
                centered("I DIG SHORTCUTS.", 216.0, WHITE);
            }
            2 => {
                centered("CAN YOU LEND ME A LITTLE MONEY?", 208.0, WHITE);
                let key = if matches!(game.world.name.as_str(), "rTransition1x" | "rTransition3x") {
                    "tunnel1"
                } else {
                    "tunnel2"
                };
                let remaining = game.data.get(key).copied().unwrap_or(0.0).max(0.0) as u32;
                let request = format!("I NEED ${remaining} FOR A NEW SHORTCUT.");
                centered(&request, 216.0, WHITE);
                // Source aligns the donation amount with the request, rather
                // than centering the shorter amount independently.
                text(
                    quads,
                    atlas,
                    &format!("DONATE: {}", tunnel.value("donate") as u32),
                    ((320.0 - request.len() as f32 * 8.0) / 2.0).ceil(),
                    224.0,
                    1.0,
                    YELLOW,
                );
            }
            3 => centered("THANKS! YOU WON'T REGRET IT!", 216.0, WHITE),
            4 => centered("I'LL NEVER GET THIS SHORTCUT BUILT!", 216.0, WHITE),
            5 => centered("ONE SHORTCUT, COMING UP!", 216.0, WHITE),
            _ => {}
        }
    } else if game.world.entities.iter().any(|entity| {
        entity.alive
            && entity.engine_active
            && entity.object == "oDamselKiss"
            && entity.value("kissed") != 0.0
    }) {
        centered("MY HERO!", 216.0, WHITE);
    }
    let heading = if !completed_name.is_empty() {
        format!("{completed_name} COMPLETED!")
    } else if completed_level == 0 {
        "TUTORIAL CAVE COMPLETED!".into()
    } else {
        format!("LEVEL {completed_level} COMPLETED!")
    };
    text(quads, atlas, &heading, 32.0, 48.0, 1.0, YELLOW);
    for (label, y) in [
        ("TIME  = ", 64.0),
        ("LOOT  = ", 80.0),
        ("KILLS = ", 96.0),
        ("MONEY = ", 112.0),
    ] {
        text(quads, atlas, label, 32.0, y, 1.0, WHITE);
    }
    if draw_loot >= 1 && !is_loot {
        text(quads, atlas, "NONE", 96.0, 80.0, 1.0, WHITE);
    }
    if draw_loot > -2 {
        let level = level_time_ms / 1000;
        let total = source_time_ms / 1000;
        text(
            quads,
            atlas,
            &format!(
                "{}:{:02} / {}:{:02}",
                level / 60,
                level % 60,
                total / 60,
                total % 60
            ),
            96.0,
            64.0,
            1.0,
            WHITE,
        );
    }
    if draw_loot == 2 {
        if !is_kills {
            text(quads, atlas, "NONE", 96.0, 96.0, 1.0, WHITE);
        }
        text(
            quads,
            atlas,
            &format!("${money_count} / ${}", game.player.money),
            96.0,
            112.0,
            1.0,
            WHITE,
        );
    }
}

fn background(
    quads: &mut Vec<DrawQuad>,
    game: &Game,
    camera: Vec2,
    pixel_snap: bool,
    atlas: &Atlas,
) {
    let (name, tiled) = match game.world.name.as_str() {
        "rEnd" | "rEndCustom" | "rTransition1" | "rTransition1x" | "rTransition2"
        | "rTransition2x" | "rTransition3" | "rTransition3x" => ("bgCave", true),
        "rTransition4" => ("bgTemple", true),
        "rEnd3" | "rCredits2" => ("bgClouds", false),
        "rIntro" | "rCredits1" => ("backgroundNight", true),
        // The volcano room is covered by its authored oEnd2BG instance.
        "rEnd2" => return,
        _ => (
            match game.world.area {
                Area::Mines | Area::Jungle | Area::Ice => "bgCave",
                Area::Temple | Area::Boss => "bgTemple",
            },
            true,
        ),
    };
    let Some(frame) = atlas.sprite(name, 0.0) else {
        return;
    };
    let [width, height] = frame.size;
    if !tiled {
        sprite(
            quads,
            atlas,
            name,
            0.0,
            -camera.x as f32,
            -camera.y as f32,
            false,
            WHITE,
        );
        return;
    }
    let mut start_x = -(camera.x as f32).rem_euclid(width);
    let mut start_y = -(camera.y as f32).rem_euclid(height);
    if pixel_snap {
        start_x = start_x.round();
        start_y = start_y.round();
    }
    let mut y = start_y;
    while y < 240.0 {
        let mut x = start_x;
        while x < 320.0 {
            sprite(quads, atlas, name, 0.0, x, y, false, WHITE);
            x += width;
        }
        y += height;
    }
}

#[derive(Debug)]
struct Light {
    position: Vec2,
    radius: f64,
    tint: [f64; 3],
}

fn darkness_enabled(game: &Game) -> bool {
    game.world.dark
        && game.player.state != PlayerState::Dead
        && game.state != spelunky_core::RunState::Dead
}

fn scene_lights(game: &Game, alpha: f64) -> Vec<Light> {
    let game = game.presentation();
    let player = interpolated(game.player.previous_pos, game.player.pos, alpha);
    let darkness = game
        .data
        .get("darkness")
        .copied()
        .filter(|value| value.is_finite())
        .unwrap_or(1.0);
    // GM8 packs rounded components by addition before extracting RGB bytes.
    // Negative explosion darkness therefore carries overflow into other
    // channels instead of clamping to white.
    let channel = 255.0 - 255.0 * darkness;
    let mut tint = source_mask_rgb(channel, channel, 255.0);
    // Source selects the nearest static red lamp, then the nearest carried-lamp
    // class. The latter overrides the former; instance creation order does not.
    for distance in red_lamp_distances(game).into_iter().flatten() {
        if distance <= 96.0 {
            tint = source_mask_rgb(255.0 - distance, 24.0 + distance, 24.0 + distance);
        }
    }
    let mut lights = vec![Light {
        position: player,
        radius: 96.0 - 64.0 * darkness,
        tint,
    }];
    for entity in game
        .world
        .entities
        .iter()
        .filter(|entity| entity.alive && entity.engine_active)
    {
        let Some((radius, dx, dy)) = source_light(&entity.object) else {
            continue;
        };
        let radius = entity.data.get("light_radius").copied().unwrap_or(radius);
        if !radius.is_finite() || radius <= 0.0 {
            continue;
        }
        let mut position = entity_position(game, entity, alpha);
        position.x += entity.data.get("light_offset_x").copied().unwrap_or(dx);
        position.y += entity.data.get("light_offset_y").copied().unwrap_or(dy);
        lights.push(Light {
            position,
            radius,
            tint,
        });
    }
    for (index, tile) in game.world.tiles.iter().enumerate() {
        if !tile.engine_active {
            continue;
        }
        let Some((radius, dx, dy)) = source_light(&tile.object) else {
            continue;
        };
        let position = Vec2::new(
            (index % game.world.width.max(1)) as f64 * 16.0 + dx,
            (index / game.world.width.max(1)) as f64 * 16.0 + dy,
        );
        lights.push(Light {
            position,
            radius,
            tint,
        });
    }
    lights
}

fn red_lamp_distances(game: &Game) -> [Option<f64>; 2] {
    let game = game.presentation();
    let player_mask = game.player.sprite_bounds();
    ["oLampRed", "oLampRedItem"].map(|object| {
        game.world
            .entities
            .iter()
            .filter(|e| e.alive && e.engine_active && e.object == object)
            .map(|e| sprite_distance(player_mask, e.sprite_bounds()))
            .chain(
                game.world
                    .tiles
                    .iter()
                    .enumerate()
                    .filter(|(_, tile)| tile.engine_active && tile.object == object)
                    .map(|(index, _)| {
                        sprite_distance(
                            player_mask,
                            game.world.tile_bounds(
                                (index % game.world.width.max(1)) as i32,
                                (index / game.world.width.max(1)) as i32,
                            ),
                        )
                    }),
            )
            .min_by(f64::total_cmp)
    })
}

#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) fn lighting_diagnostics(game: &Game, alpha: f64) -> serde_json::Value {
    let game = game.presentation();
    let lights = scene_lights(game, alpha);
    serde_json::json!({
        "active": darkness_enabled(game),
        "final_tint_rgb": lights[0].tint.map(|channel| (channel * 255.0).round() as u8),
        "final_tint": lights[0].tint,
        "red_lamp_distances": red_lamp_distances(game),
        "circles": lights.iter().map(|light| serde_json::json!({
            "x": light.position.x, "y": light.position.y, "radius": light.radius,
        })).collect::<Vec<_>>(),
    })
}

fn source_mask_rgb(red: f64, green: f64, blue: f64) -> [f64; 3] {
    // Canonical Spelunky.exe initializes x87 rounding to nearest-even.
    // make_color_rgb at VA 0x55359c rounds to signed 64-bit integers, then
    // adds r + 256*g + 65536*b without per-channel masks. draw_set_color's
    // VA 0x49c813..0x49c833 extracts the three low bytes; alpha is separate.
    // Source mask inputs are finite and small; special/out-of-i64 inputs are
    // outside this narrowly reproduced runtime contract.
    let packed = (red.round_ties_even() as i64)
        .wrapping_add((green.round_ties_even() as i64).wrapping_mul(256))
        .wrapping_add((blue.round_ties_even() as i64).wrapping_mul(65536));
    [0, 8, 16].map(|shift| ((packed >> shift) & 255) as f64 / 255.0)
}

fn sprite_distance(a: spelunky_core::Rect, b: spelunky_core::Rect) -> f64 {
    if !a.is_valid() || !b.is_valid() {
        return f64::INFINITY;
    }
    let x = (a.x - (b.x + b.w - 1.0))
        .max(b.x - (a.x + a.w - 1.0))
        .max(0.0);
    let y = (a.y - (b.y + b.h - 1.0))
        .max(b.y - (a.y + a.h - 1.0))
        .max(0.0);
    x.hypot(y)
}

fn source_light(object: &str) -> Option<(f64, f64, f64)> {
    Some(match object {
        "oFlare" | "oFlareCrate" | "oExplosion" => (96.0, 0.0, 0.0),
        // oScreen's `with oLamp` includes its red-lamp child. The carried red
        // lamp has oItem as parent and therefore supplies no separate circle.
        "oLamp" | "oLampRed" => (96.0, 8.0, 8.0),
        "oLampItem" => (96.0, 0.0, -4.0),
        "oArrowTrapLeftLit" | "oArrowTrapRightLit" | "oTikiTorch" | "oFireFrog"
        | "oSpearTrapLit" | "oSmashTrapLit" | "oLava" => (32.0, 8.0, 8.0),
        "oScarab" => (16.0, 8.0, 8.0),
        "oGhost" => (64.0, 16.0, 16.0),
        _ => return None,
    })
}

#[cfg(test)]
fn illumination(lights: &[Light], point: Vec2) -> [f64; 3] {
    lights
        .iter()
        .find(|light| light.position.distance(point) <= light.radius)
        .map_or([0.0; 3], |light| light.tint)
}

fn darkness(quads: &mut Vec<DrawQuad>, game: &Game, camera: Vec2, alpha: f64) {
    let lights = scene_lights(game, alpha);
    let tint = lights[0].tint.map(|channel| channel as f32);
    let mut intervals: Vec<(i32, i32)> = Vec::with_capacity(lights.len());
    // oScreen builds one colored union of filled circles, then multiplies that
    // mask into the scene. Intersections must not multiply the scene twice.
    // Scanline runs retain logical-pixel edges without an offscreen framebuffer
    // or an arbitrary GPU light-count limit. GM8's polygon edge rasterization
    // has not been verified; these use analytic circles at pixel centers.
    for y in 0..240 {
        intervals.clear();
        for light in &lights {
            let dy = camera.y + f64::from(y) + 0.5 - light.position.y;
            if dy.abs() > light.radius {
                continue;
            }
            let half = (light.radius * light.radius - dy * dy).sqrt();
            let center = light.position.x - camera.x;
            let start = (center - half - 0.5).ceil().clamp(0.0, 320.0) as i32;
            let end = ((center + half - 0.5).floor() + 1.0).clamp(0.0, 320.0) as i32;
            if start < end {
                intervals.push((start, end));
            }
        }
        intervals.sort_unstable();
        let mut cursor = 0;
        let mut active: Option<(i32, i32)> = None;
        for (start, end) in intervals.iter().copied().chain(std::iter::once((321, 321))) {
            if let Some((left, right)) = active {
                if start <= right {
                    active = Some((left, right.max(end)));
                    continue;
                }
                mask_run(quads, cursor, left, y, [0.0; 3]);
                mask_run(quads, left, right, y, tint);
                cursor = right;
            }
            active = (start < 320).then_some((start, end));
        }
        mask_run(quads, cursor, 320, y, [0.0; 3]);
    }
}

fn mask_run(quads: &mut Vec<DrawQuad>, left: i32, right: i32, y: i32, color: [f32; 3]) {
    if right > left {
        quads.push(DrawQuad {
            rect: [left as f32, y as f32, (right - left) as f32, 1.0],
            color: [color[0], color[1], color[2], 1.0],
            blend: QuadBlend::Multiply,
            ..DrawQuad::default()
        });
    }
}

fn hub_value(game: &Game, key: &str) -> f64 {
    game.data
        .get(key)
        .copied()
        .filter(|value| value.is_finite())
        .unwrap_or(0.0)
        .max(0.0)
}

fn hub_hud(quads: &mut Vec<DrawQuad>, game: &Game, camera: Vec2, atlas: &Atlas) {
    let yellow = [1.0, 1.0, 0.0, 1.0];
    let (challenge, first_icon, first_value, second_icon, second_value) =
        if game.flags.contains("hub_sun") {
            (
                "DAMSEL",
                "sHeart",
                game.player.health.max(0) as f64,
                "sDamselIcon",
                hub_value(game, "points"),
            )
        } else if game.flags.contains("hub_moon") {
            (
                "ARCHERY",
                "sHoopsIcon",
                hub_value(game, "baskets"),
                "sTimerIcon",
                hub_value(game, "timer"),
            )
        } else if game.flags.contains("hub_stars") {
            (
                "SHOTGUN",
                "sHeart",
                game.player.health.max(0) as f64,
                "sShopkeeperIcon",
                hub_value(game, "stars_kills"),
            )
        } else {
            ("", "", 0.0, "", 0.0)
        };
    if !challenge.is_empty() {
        for (icon, value, x) in [
            (first_icon, first_value, 8.0),
            (second_icon, second_value, 64.0),
        ] {
            sprite(quads, atlas, icon, 0.0, x, 8.0, false, WHITE);
            font(
                quads,
                atlas,
                &format!("{}", value as u32),
                x + 16.0,
                8.0,
                1.0,
                WHITE,
                "sFont",
            );
        }
        let status = hub_value(game, "hub_draw_status") as u32;
        if status < 3 {
            let message = format!("{challenge} CHALLENGE BEGINS IN {}...", 3 - status);
            text(
                quads,
                atlas,
                &message,
                ((320.0 - message.len() as f32 * 8.0) / 2.0).ceil(),
                216.0,
                1.0,
                yellow,
            );
        }
        return;
    }
    if !game.flags.contains("hub_scores") {
        if game.flags.contains("hub_title")
            && game.player.data.get("hidden") != Some(&1.0)
            && game.player.pos.x < 256.0
            && game.camera.x <= 1.0
        {
            let message = "WELCOME TO THE SHORTCUT HOUSE!";
            text(
                quads,
                atlas,
                message,
                ((320.0 - message.len() as f32 * 8.0) / 2.0).ceil() - camera.x as f32,
                224.0 - camera.y as f32,
                1.0,
                WHITE,
            );
        }
        return;
    }
    if game.player.pos.y < 156.0 {
        text(quads, atlas, "SECRET CHALLENGES", 144.0, 32.0, 1.0, yellow);
        for (label, key, y) in [
            ("SUN", "best_mini1", 48.0),
            ("MOON", "best_mini2", 64.0),
            ("STARS", "best_mini3", 80.0),
        ] {
            text(
                quads,
                atlas,
                &format!("{:<7}{}", format!("{label}:"), hub_value(game, key) as u32),
                128.0,
                y,
                1.0,
                WHITE,
            );
        }
        let (title, flag, detail): (&str, &str, &[&str]) = if game.player.pos.y < 64.0 {
            (
                "SUN ROOM",
                "unlock_sun",
                &[
                    "KEEP YOURSELF AND",
                    "THE DAMSEL ALIVE",
                    "FOR AS LONG AS",
                    "POSSIBLE!",
                ],
            )
        } else if game.player.pos.y < 96.0 {
            (
                "MOON ROOM",
                "unlock_moon",
                &["SHOOT THE MOVING", "TARGETS WITH YOUR", "BOW AND ARROWS!"],
            )
        } else if game.player.pos.y < 128.0 {
            (
                "STARS ROOM",
                "unlock_stars",
                &[
                    "KILL AS MANY ANGRY",
                    "SHOPKEEPS AS YOU CAN",
                    "BEFORE THEY GET YOU!",
                ],
            )
        } else {
            ("CHANGING ROOM", "unlock_damsel", &["LADY IN RED..."])
        };
        text(
            quads,
            atlas,
            title,
            if flag == "unlock_damsel" {
                160.0
            } else {
                168.0
            },
            96.0,
            1.0,
            yellow,
        );
        if game.flags.contains(flag) && !game.flags.contains("hub_records_erased") {
            for (line, value) in detail.iter().enumerate() {
                text(
                    quads,
                    atlas,
                    value,
                    128.0,
                    112.0 + line as f32 * 16.0,
                    1.0,
                    WHITE,
                );
            }
        } else {
            text(quads, atlas, "LOCKED.", 128.0, 112.0, 1.0, WHITE);
        }
    } else {
        text(quads, atlas, "TOP DEFILERS", 160.0, 32.0, 1.0, yellow);
        for (label, key, y) in [
            ("MONEY", "best_money", 48.0),
            ("KILLS", "best_kills", 64.0),
            ("SAVES", "best_saves", 80.0),
        ] {
            text(
                quads,
                atlas,
                &format!("{:<8}{}", format!("{label}:"), hub_value(game, key) as u64),
                128.0,
                y,
                1.0,
                WHITE,
            );
        }
        let seconds = hub_value(game, "best_time") as u64;
        if hub_value(game, "total_wins") > 0.0 {
            text(
                quads,
                atlas,
                &format!("TIME:   {}:{:02}", seconds / 60, seconds % 60),
                128.0,
                96.0,
                1.0,
                WHITE,
            );
        }
        text(quads, atlas, "STATISTICS", 168.0, 112.0, 1.0, yellow);
        for (label, key, y) in [
            ("PLAYS", "total_plays", 128.0),
            ("DEATHS", "total_deaths", 144.0),
            ("WINS", "total_wins", 160.0),
        ] {
            text(
                quads,
                atlas,
                &format!("{:<8}{}", format!("{label}:"), hub_value(game, key) as u64),
                128.0,
                y,
                1.0,
                WHITE,
            );
        }
        let clear_button = game.world.entities.iter().find(|entity| {
            entity.alive && entity.engine_active && entity.object == "oButtonHighscore"
        });
        let block = game
            .world
            .entities
            .iter()
            .filter(|entity| {
                entity.alive && entity.engine_active && catalog::is_a(&entity.object, "oPushBlock")
            })
            .min_by(|left, right| {
                left.pos
                    .distance(Vec2::new(160.0, 240.0))
                    .total_cmp(&right.pos.distance(Vec2::new(160.0, 240.0)))
            });
        if clear_button.is_some_and(|button| button.value("pushed") == 0.0)
            && block.is_some_and(|block| block.pos.x > 160.0)
        {
            let message = "THIS WILL CLEAR EVERYTHING!";
            text(
                quads,
                atlas,
                message,
                ((320.0 - message.len() as f32 * 8.0) / 2.0).ceil(),
                216.0,
                1.0,
                yellow,
            );
        }
    }
}

fn hud(quads: &mut Vec<DrawQuad>, game: &Game, camera: Vec2, atlas: &Atlas) {
    if game.opening.as_ref().is_some_and(|opening| {
        opening.kind == spelunky_core::opening::OpeningKind::Intro || !opening.player_present
    }) {
        return;
    }
    // oPlayer1's death handler clears global.drawHUD while the corpse and
    // surrounding actors continue to simulate behind the game-over text.
    if game.player.state == PlayerState::Dead {
        return;
    }
    if game.flags.contains("hub") {
        hub_hud(quads, game, camera, atlas);
        return;
    }
    let player = &game.player;
    // Original Classic HUD positions and 16-pixel bitmap font.
    for (icon, value, x) in [
        ("sHeart", player.health.max(0).to_string(), 8.0),
        (
            if player.has("Paste") {
                "sStickyBombIcon"
            } else {
                "sBombIcon"
            },
            player.bombs.to_string(),
            64.0,
        ),
        ("sRopeIcon", player.ropes.to_string(), 120.0),
        ("sDollarSign", player.money.to_string(), 176.0),
    ] {
        sprite(quads, atlas, icon, 0.0, x, 8.0, false, WHITE);
        font(quads, atlas, &value, x + 16.0, 8.0, 1.0, WHITE, "sFont");
    }
    sprite(quads, atlas, "sHoldItemIcon", 0.0, 8.0, 24.0, false, WHITE);
    if let Some(held) = player
        .held
        .and_then(|id| game.world.entity(id))
        .filter(|entity| entity.engine_active)
    {
        if let Some(name) = entity_sprite(atlas, held) {
            sprite(quads, atlas, name, 0.0, 16.0, 32.0, false, WHITE);
        }
    }
    let mut x = 28.0;
    for name in [
        "UdjatEye",
        "Ankh",
        "Crown",
        "Kapala",
        "Spectacles",
        "Gloves",
        "Mitt",
        "SpringShoes",
        "SpikeShoes",
        "Cape",
        "Jetpack",
        "Compass",
        "Parachute",
    ] {
        let spaced = match name {
            "SpringShoes" => "Spring Shoes",
            "SpikeShoes" => "Spike Shoes",
            "UdjatEye" => "Udjat Eye",
            "Gloves" => "Climbing Gloves",
            _ => name,
        };
        if player.has(name) || player.has(spaced) {
            sprite(
                quads,
                atlas,
                &if name == "UdjatEye" && game.flags.contains("udjat_blink") {
                    "sUdjatEyeIcon2".into()
                } else {
                    format!("s{name}Icon")
                },
                0.0,
                x,
                24.0,
                false,
                WHITE,
            );
            x += 20.0;
        }
    }
    if player.has("Compass") {
        let exit_x = (game.world.exit.x - camera.x) as f32;
        let exit_y = (game.world.exit.y - camera.y) as f32;
        let name = if exit_y > 240.0 {
            if exit_x < 0.0 {
                "sCompassLL"
            } else if exit_x > 304.0 {
                "sCompassLR"
            } else {
                "sCompassDown"
            }
        } else if exit_x < 0.0 {
            "sCompassLeft"
        } else if exit_x > 304.0 {
            "sCompassRight"
        } else {
            ""
        };
        sprite(
            quads,
            atlas,
            name,
            0.0,
            exit_x.clamp(0.0, 304.0),
            exit_y.clamp(40.0, 224.0),
            false,
            WHITE,
        );
    }
    let seconds = game.tick / u64::from(TICK_RATE);
    text(
        quads,
        atlas,
        &format!(
            "{}  {:02}:{:02}",
            game.world.level,
            seconds / 60,
            seconds % 60
        ),
        216.0,
        230.0,
        1.0,
        WHITE,
    );
}

#[cfg(test)]
mod tests {
    // Author a legal room, then use only actual tick inputs. No midrun player,
    // image-index, animation-speed or controller-counter assignments.
    fn natural_duck_to_hang_callback(right: bool) -> Game {
        let mut level = spelunky_core::level::Level::default();
        level
            .set_tile(if right { 12 } else { 14 }, 18, '@')
            .unwrap();
        let platform = if right { 3..=13 } else { 14..=24 };
        for x in platform {
            level.set_tile(x, 19, '1').unwrap();
        }
        let mut game = Game::shell(42, spelunky_core::RulesPreset::Classic);
        game.load_custom(&level, "natural-duck-to-hang", "");
        let mut trace = Vec::new();
        for _ in 0..4 {
            game.step(&spelunky_core::InputFrame::default());
            trace.push((
                game.tick,
                game.state_hash(),
                game.rng.clone().next_u64(),
                serde_json::to_string(&game.events).unwrap(),
            ));
        }
        assert!(game.player.grounded, "authored entrance did not settle");
        let mut entered = false;
        for _ in 0..80 {
            game.step(&spelunky_core::InputFrame {
                down: true,
                right,
                left: !right,
                ..Default::default()
            });
            trace.push((
                game.tick,
                game.state_hash(),
                game.rng.clone().next_u64(),
                serde_json::to_string(&game.events).unwrap(),
            ));
            entered |= game.player.data.get("duck_to_hang") == Some(&1.);
            if entered && game.presentation().player.pos != game.player.pos {
                assert_eq!(game.presentation().player.sprite, "sDuckToHangL");
                assert_eq!(game.player.state, spelunky_core::PlayerState::Hanging);
                eprintln!(
                    "DUCK_AUTHORITATIVE_TRACE right={right} {}",
                    serde_json::to_string(&trace).unwrap()
                );
                return game;
            }
        }
        panic!(
            "natural crawl failed to finish duck-to-hang: right={right}, state={:?}, pos={:?}, sprite={}, animation={}",
            game.player.state, game.player.pos, game.player.sprite, game.player.animation
        );
    }
    fn natural_duck_draw_anchor(game: &Game, alpha: f64, atlas: &Atlas) -> [f64; 2] {
        let player = &game.presentation().player;
        let frame = atlas.sprite(&player.sprite, player.animation).unwrap();
        let mirrored = player.facing > 0;
        let mut uv = frame.uv;
        if mirrored {
            uv[0] += uv[2];
            uv[2] = -uv[2];
        }
        let quads = game_scene(game, alpha, false, atlas);
        let quad = quads
            .iter()
            .find(|quad| quad.uv == uv)
            .expect("source last Draw frame absent");
        [
            f64::from(
                quad.rect[0]
                    + if mirrored {
                        frame.size[0] - frame.origin[0]
                    } else {
                        frame.origin[0]
                    },
            ),
            f64::from(quad.rect[1] + frame.origin[1]),
        ]
    }

    fn natural_duck_to_hang_animation_end_scene_has_no_camera_rebound(right: bool) {
        let atlas = Atlas::load().unwrap();
        {
            let mut game = natural_duck_to_hang_callback(right);
            let hash = game.state_hash();
            let frozen_frame = game.presentation().player.animation;
            let frozen_sprite = game.presentation().player.sprite.clone();
            let expected_x =
                game.presentation().player.pos.x - game.presentation().previous_camera.x;
            for alpha in [0., 0.125, 0.5, 0.875, 1.] {
                let anchor = natural_duck_draw_anchor(&game, alpha, &atlas);
                assert!(
                    (anchor[0] - expected_x).abs() < 1e-4,
                    "right={right} alpha={alpha}: {anchor:?}"
                );
                assert!(
                    (anchor[1] - 144.).abs() < 1e-4,
                    "right={right} alpha={alpha}: {anchor:?}"
                );
                assert_eq!(game.presentation().player.animation, frozen_frame);
                assert_eq!(game.presentation().player.sprite, frozen_sprite);
                assert_eq!(
                    game.state_hash(),
                    hash,
                    "render altered authoritative state"
                );
            }
            let previous = natural_duck_draw_anchor(&game, 1., &atlas);
            game.step(&spelunky_core::InputFrame::default());
            let next = natural_duck_draw_anchor(&game, 0., &atlas);
            assert!(
                (previous[1] - next[1]).abs() < 1e-4,
                "right={right}: {previous:?} -> {next:?}"
            );
            assert_eq!(game.presentation().player.sprite, "sHangLeft");
        }
    }
    #[test]
    fn natural_duck_to_hang_left_scene_has_no_camera_rebound() {
        natural_duck_to_hang_animation_end_scene_has_no_camera_rebound(false);
    }
    #[test]
    fn natural_duck_to_hang_right_scene_has_no_camera_rebound() {
        natural_duck_to_hang_animation_end_scene_has_no_camera_rebound(true);
    }

    use super::*;
    use spelunky_core::RulesPreset;

    fn source_draw_room() -> Game {
        let mut level = spelunky_core::level::Level::default();
        level.tiles[8 * 40 + 4] = '@';
        for x in 0..40 {
            level.tiles[10 * 40 + x] = '1';
        }
        for y in 5..10 {
            level.tiles[y * 40 + 9] = '1';
        }
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.load_custom(&level, "source-draw-order", "");
        game
    }

    #[test]
    fn actual_snake_whip_kill_draws_opaque_blood_at_fractional_refresh() {
        use spelunky_core::InputFrame;

        let atlas = Atlas::load().unwrap();
        let mut level = spelunky_core::level::Level::default();
        level.tiles[8 * 40 + 4] = '@';
        level.tiles[8 * 40 + 6] = 'n';
        for x in 0..40 {
            level.tiles[9 * 40 + x] = '1';
        }
        let mut game = Game::shell(42, RulesPreset::Classic);
        game.load_custom(&level, "blood-draw", "");
        let snake = game
            .world
            .entities
            .iter()
            .find(|entity| entity.object == "oSnake")
            .unwrap()
            .id;
        game.step(&InputFrame {
            right: true,
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });

        let mut blood_draw_ticks = 0;
        let mut frames = std::collections::BTreeSet::new();
        let mut saw_trail = false;
        for _ in 0..30 {
            let draw = game.presentation();
            let blood: Vec<_> = draw
                .world
                .entities
                .iter()
                .filter(|entity| entity.alive && entity.engine_active && entity.object == "oBlood")
                .collect();
            if !blood.is_empty() {
                blood_draw_ticks += 1;
            }
            saw_trail |= draw.world.entities.iter().any(|entity| {
                entity.alive && entity.engine_active && entity.object == "oBloodTrail"
            });
            let hash = game.state_hash();
            let draw_hash = draw.state_hash();
            for entity in blood {
                let frame = atlas.sprite("sBlood", entity.animation).unwrap();
                frames.insert(entity.animation.floor() as usize);
                let atlas_x = (frame.uv[0] * atlas.width as f32).round() as usize;
                let atlas_y = (frame.uv[1] * atlas.height as f32).round() as usize;
                let mut red_texels = 0;
                for y in atlas_y..atlas_y + frame.size[1] as usize {
                    for x in atlas_x..atlas_x + frame.size[0] as usize {
                        let offset = (y * atlas.width as usize + x) * 4;
                        let pixel = &atlas.rgba()[offset..offset + 4];
                        if pixel[3] != 0 {
                            assert_eq!(pixel[3], 255);
                            assert!(pixel[0] > pixel[1] && pixel[0] > pixel[2]);
                            red_texels += 1;
                        }
                    }
                }
                assert!(
                    red_texels >= 17,
                    "all original blood frames contain opaque red"
                );
                for alpha in [0.0, 0.25, 0.5, 0.75, 1.0] {
                    let quads = game_scene(&game, alpha, false, &atlas);
                    let position = entity_position(draw, entity, alpha);
                    let camera = interpolated(draw.previous_camera, draw.camera, alpha);
                    let expected = [
                        (position.x - camera.x) as f32 - frame.origin[0],
                        (position.y - camera.y) as f32 - frame.origin[1],
                        frame.size[0],
                        frame.size[1],
                    ];
                    let quad = quads
                        .iter()
                        .find(|quad| quad.uv == frame.uv && quad.rect == expected)
                        .expect("natural death blood has its own interpolated scene quad");
                    assert_eq!(quad.color, WHITE);
                    assert_eq!(quad.blend, QuadBlend::Alpha);
                    assert!(quad.rect[0] >= 0.0 && quad.rect[0] + quad.rect[2] <= 320.0);
                    assert!(quad.rect[1] >= 0.0 && quad.rect[1] + quad.rect[3] <= 240.0);
                    assert_eq!(game.state_hash(), hash);
                    assert_eq!(game.presentation().state_hash(), draw_hash);
                }
            }
            game.step(&InputFrame {
                right: true,
                ..Default::default()
            });
        }
        assert!(
            game.world.entity(snake).is_none(),
            "actual player whip kills the snake"
        );
        assert!(
            blood_draw_ticks >= 10,
            "blood remains visible across actual Draw phases"
        );
        assert_eq!(frames, [0, 1, 2].into_iter().collect());
        assert!(saw_trail, "actual blood alarms also produce visible trails");
    }

    #[test]
    fn actual_animation_end_destruction_keeps_the_last_source_draw_frame() {
        let atlas = Atlas::load().unwrap();
        let mut game = source_draw_room();
        let id = game.spawn("oPoof", Vec2::new(112.0, 80.0));
        for tick in 1..=15 {
            game.step(&spelunky_core::InputFrame::default());
            let draw = game.presentation().world.entity(id).unwrap();
            assert_eq!(draw.sprite, "sPoof");
            assert!((draw.animation - f64::from(tick - 1) * 0.4).abs() < 0.00001);
            let frame = atlas.sprite("sPoof", draw.animation).unwrap();
            let hash = game.state_hash();
            let draw_hash = game.presentation().state_hash();
            for alpha in [0.0, 0.5, 1.0] {
                let quads = game_scene(&game, alpha, false, &atlas);
                assert!(quads.iter().any(|quad| quad.uv == frame.uv));
                assert_eq!(game.state_hash(), hash);
                assert_eq!(game.presentation().state_hash(), draw_hash);
            }
        }
        assert!(game.world.entity(id).is_none(), "source AnimationEnd ran");
        assert!(game.presentation().world.entity(id).is_some());
        game.step(&spelunky_core::InputFrame::default());
        assert!(game.presentation().world.entity(id).is_none());
        let last_frame = atlas.sprite("sPoof", 5.0).unwrap();
        assert!(
            game_scene(&game, 1.0, false, &atlas)
                .iter()
                .all(|quad| quad.uv != last_frame.uv)
        );
    }

    #[test]
    fn input_skipped_intro_dummy_uses_source_draw_phase_at_fractional_refresh() {
        let atlas = Atlas::load().unwrap();
        let mut game = Game::shell(7, RulesPreset::Classic);
        spelunky_core::opening::begin_intro(&mut game);
        game.step(&spelunky_core::InputFrame {
            attack_pressed: true,
            ..Default::default()
        });
        for _ in 0..60 {
            game.step(&spelunky_core::InputFrame::default());
            let Some(draw) = game
                .presentation()
                .world
                .entities
                .iter()
                .find(|entity| entity.object == "oPDummy3" && entity.pos.x > 16.0)
            else {
                continue;
            };
            assert_eq!(draw.sprite, "sRunLeft");
            let live = game.world.entity(draw.id).unwrap();
            let mut expected = draw.animation;
            spelunky_core::animation::advance(
                &mut expected,
                draw.image_speed(),
                catalog::sprite_frame_count(&draw.sprite),
            );
            assert!((live.animation - expected).abs() < 0.00001);
            let frame = atlas.sprite(&draw.sprite, draw.animation).unwrap();
            let mut uv = frame.uv;
            uv[0] += uv[2];
            uv[2] = -uv[2];
            let hash = game.state_hash();
            let draw_hash = game.presentation().state_hash();
            for alpha in [0.0, 0.5, 1.0] {
                let position = entity_position(game.presentation(), draw, alpha);
                let quads = game_scene(&game, alpha, false, &atlas);
                let quad = quads.iter().find(|quad| quad.uv == uv).unwrap();
                assert_eq!(quad.rect[0], position.x as f32 - 8.0);
                assert_eq!(game.state_hash(), hash);
                assert_eq!(game.presentation().state_hash(), draw_hash);
            }
            return;
        }
        panic!("input skip must naturally start the travelling intro dummy");
    }

    #[test]
    fn input_fired_web_cannon_draws_all_source_frames_before_animation_end_birth() {
        let atlas = Atlas::load().unwrap();
        let mut game = source_draw_room();
        let weapon = game.spawn(
            "oWebCannon",
            Vec2::new(game.player.pos.x, game.player.pos.y + 2.0),
        );
        game.step(&spelunky_core::InputFrame {
            down: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert_eq!(game.player.held, Some(weapon));
        game.step(&spelunky_core::InputFrame::default());
        game.step(&spelunky_core::InputFrame {
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        let ball = game
            .world
            .entities
            .iter()
            .find(|entity| entity.object == "oWebBall")
            .expect("actual player fire creates source WebBall")
            .id;
        let mut visible_frames = Vec::new();
        for _ in 0..60 {
            if let Some(draw) = game
                .presentation()
                .world
                .entity(ball)
                .filter(|entity| entity.sprite == "sWebCreate")
            {
                let frame = draw.animation.floor() as u32;
                visible_frames.push(frame);
                let uv = atlas.sprite("sWebCreate", draw.animation).unwrap().uv;
                let hash = game.state_hash();
                let draw_hash = game.presentation().state_hash();
                for alpha in [0.0, 0.5, 1.0] {
                    assert!(
                        game_scene(&game, alpha, false, &atlas)
                            .iter()
                            .any(|quad| quad.uv == uv)
                    );
                    assert_eq!(game.state_hash(), hash);
                    assert_eq!(game.presentation().state_hash(), draw_hash);
                }
            }
            if game.world.entity(ball).is_none() {
                break;
            }
            game.step(&spelunky_core::InputFrame::default());
        }
        assert_eq!(visible_frames, vec![0, 1, 2, 3, 4]);
        assert!(game.world.entity(ball).is_none());
        let web = game
            .world
            .entities
            .iter()
            .find(|entity| entity.object == "oWeb")
            .expect("actual AnimationEnd creates source Web")
            .id;
        assert!(game.presentation().world.entity(ball).is_some());
        assert!(game.presentation().world.entity(web).is_none());
        game.step(&spelunky_core::InputFrame::default());
        assert!(game.presentation().world.entity(ball).is_none());
        let draw = game.presentation().world.entity(web).unwrap();
        let uv = atlas.sprite(&draw.sprite, draw.animation).unwrap().uv;
        assert!(
            game_scene(&game, 1.0, false, &atlas)
                .iter()
                .any(|quad| quad.uv == uv)
        );
    }

    #[test]
    fn restored_inactive_actors_and_terrain_never_draw_or_supply_light() {
        let atlas = Atlas::load().unwrap();
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.world.dark = true;
        game.world.set_tile(5, 5, "oBrick");
        game.world.tiles[5 * game.world.width + 5].engine_active = false;
        for object in ["oLampRed", "oLampItem", "oScarab"] {
            let id = game.world.spawn(object, Vec2::new(80.0, 80.0));
            game.world.entity_mut(id).unwrap().engine_active = false;
        }
        let metadata = game.world.spawn("oTerrainMetadata", Vec2::new(88.0, 88.0));
        let metadata = game.world.entity_mut(metadata).unwrap();
        metadata.engine_active = false;
        metadata.data.insert("frozen_caveman".into(), 1.0);
        let game: Game = serde_json::from_str(&serde_json::to_string(&game).unwrap()).unwrap();
        let hash = game.state_hash();
        let quads = game_scene(&game, 1.0, false, &atlas);
        for name in [
            "sLampRed",
            "sLampItem",
            "sScarab",
            "sBrick",
            "sFrozenCaveman",
        ] {
            let uv = atlas.sprite(name, 0.0).unwrap().uv;
            assert!(quads.iter().all(|quad| quad.uv != uv), "{name}");
        }
        assert_eq!(scene_lights(&game, 1.0).len(), 1);
        assert_eq!(red_lamp_distances(&game), [None, None]);
        assert_eq!(game.state_hash(), hash);
    }

    #[test]
    fn source_game_over_reveals_score_and_bound_key_hint_at_original_positions() {
        let atlas = Atlas::load().unwrap();
        let mut quads = Vec::new();
        game_over_text(
            &mut quads,
            &atlas,
            0,
            0,
            false,
            false,
            "PRESS J FOR HIGH SCORES.",
        );
        assert!(quads.is_empty());
        game_over_text(
            &mut quads,
            &atlas,
            1,
            0,
            false,
            false,
            "PRESS J FOR HIGH SCORES.",
        );
        assert_eq!(quads[0].rect, [88.0, 48.0, 16.0, 16.0]);
        assert!(
            !quads
                .iter()
                .any(|quad| matches!(quad.rect[1], 80.0 | 88.0 | 120.0))
        );
        quads.clear();
        game_over_text(
            &mut quads,
            &atlas,
            3,
            12300,
            false,
            false,
            "PRESS J FOR HIGH SCORES.",
        );
        assert!(quads.iter().any(|quad| quad.rect == [88.0, 80.0, 8.0, 8.0]));
        assert!(
            quads
                .iter()
                .any(|quad| quad.rect == [88.0, 88.0, 16.0, 16.0] && quad.color == WHITE)
        );
        let hint: Vec<_> = quads.iter().filter(|quad| quad.rect[1] == 120.0).collect();
        assert_eq!(hint.len(), "PRESS J FOR HIGH SCORES.".len());
        assert_eq!(hint[0].rect[0], 64.0);
    }

    #[test]
    fn source_challenge_finish_uses_record_message_without_campaign_score_or_hint() {
        let atlas = Atlas::load().unwrap();
        let mut quads = Vec::new();
        game_over_text(
            &mut quads,
            &atlas,
            3,
            12300,
            true,
            true,
            "PRESS J FOR HIGH SCORES.",
        );
        assert_eq!(quads[0].rect, [88.0, 48.0, 16.0, 16.0]);
        assert!(
            !quads
                .iter()
                .any(|quad| matches!(quad.rect[1], 88.0 | 120.0))
        );
        let record: Vec<_> = quads.iter().filter(|quad| quad.rect[1] == 80.0).collect();
        assert_eq!(record.len(), "YOU SET A NEW RECORD!".len());
        assert_eq!(record[0].rect[0], 76.0);
    }

    #[test]
    fn moon_challenge_finish_waits_for_negative_source_timer() {
        let atlas = Atlas::load().unwrap();
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.flags.insert("hub_moon".into());
        game.state = spelunky_core::RunState::Dead;
        game.game_over.draw_status = 2;
        for (timer, visible) in [(1.0, false), (0.0, false), (-1.0, true)] {
            game.data.insert("timer".into(), timer);
            let mut quads = Vec::new();
            game_over_overlay(&mut quads, &atlas, &game, "");
            assert_eq!(!quads.is_empty(), visible);
        }
    }

    #[test]
    fn authored_intro_snapshot_keeps_source_story_and_simulation_stable_at_high_refresh() {
        let atlas = Atlas::load().unwrap();
        let mut game = Game::shell(7, RulesPreset::Classic);
        spelunky_core::opening::begin_intro(&mut game);
        game.opening.as_mut().unwrap().draw_status = 2;
        let lines = game.opening.as_ref().unwrap().lines.clone();
        let hash = game.state_hash();
        for alpha in [0.0, 0.25, 0.5, 0.75, 1.0] {
            let quads = game_scene(&game, alpha, false, &atlas);
            assert_eq!(
                quads[0].uv,
                atlas.sprite("backgroundNight", 0.0).unwrap().uv
            );
            let story: Vec<_> = quads
                .iter()
                .filter(|quad| {
                    matches!(quad.rect[1], 100.0 | 116.0)
                        && quad.rect[2] == 8.0
                        && quad.rect[3] == 8.0
                })
                .collect();
            assert_eq!(story.len(), lines[0].len() + lines[1].len());
            assert!(
                !quads
                    .iter()
                    .any(|quad| quad.uv == atlas.sprite("sHeart", 0.0).unwrap().uv)
            );
            assert_eq!(game.state_hash(), hash);
        }
    }

    #[test]
    fn reading_game_over_countup_never_advances_its_counter_or_source_alarms() {
        let atlas = Atlas::load().unwrap();
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.state = spelunky_core::RunState::Dead;
        game.game_over.draw_status = 3;
        game.game_over.money_count = 12300;
        game.player.money = 23000;
        game.alarms[0] = 10;
        let hash = game.state_hash();
        for _ in 0..8 {
            let mut quads = Vec::new();
            game_over_overlay(&mut quads, &atlas, &game, "PRESS J FOR HIGH SCORES.");
            assert!(!quads.is_empty());
            assert_eq!(game.state_hash(), hash);
        }
    }

    #[test]
    fn source_dialogue_preserves_two_independent_lines_colors_and_full_size_font() {
        let atlas = Atlas::load().unwrap();
        let mut quads = Vec::new();
        source_message(
            &mut quads,
            &atlas,
            "WELCOME!",
            "PRESS P TO PURCHASE.",
            false,
        );
        assert_eq!(quads[0].rect, [128.0, 216.0, 8.0, 8.0]);
        assert_eq!(quads[0].color, WHITE);
        assert_eq!(quads[8].rect, [80.0, 224.0, 8.0, 8.0]);
        assert_eq!(quads[8].color, [1.0, 1.0, 0.0, 1.0]);
        assert_eq!(quads.len(), "WELCOME!".len() + "PRESS P TO PURCHASE.".len());
        quads.clear();
        source_message(&mut quads, &atlas, "", "TRY THE ROPE!", true);
        assert!(quads.iter().all(|quad| quad.color == WHITE));
        assert!(quads.iter().all(|quad| quad.rect[1] == 224.0));
    }

    #[test]
    fn intro_story_reveals_source_lines_over_the_black_fade() {
        let atlas = Atlas::load().unwrap();
        let lines = [
            "AS THE MOON BURNED BRIGHT ABOVE,".into(),
            "I SQUINTED INTO THE DARKNESS,".into(),
            "AND THAT'S WHEN IT ALL STARTED.".into(),
        ];
        let mut quads = Vec::new();
        intro_narrative(&mut quads, &atlas, 2, 1.0, &lines);
        assert_eq!(quads[0].rect, [0.0, 0.0, 320.0, 240.0]);
        assert_eq!(quads[0].color, [0.0, 0.0, 0.0, 1.0]);
        assert_eq!(quads[1].rect[1], 100.0);
        assert!(quads.iter().any(|quad| quad.rect[1] == 116.0));
        assert!(!quads.iter().any(|quad| quad.rect[1] == 132.0));
        assert!(quads.iter().skip(1).all(|quad| quad.color == WHITE));
        quads.clear();
        intro_narrative(&mut quads, &atlas, -1, 0.5, &lines);
        assert_eq!(quads.len(), 1, "travelling actor stage hides narrative");
    }

    #[test]
    fn title_presentation_credit_retains_source_overlay_alpha() {
        let atlas = Atlas::load().unwrap();
        let mut quads = Vec::new();
        title_opening_overlay(&mut quads, &atlas, 1, 0.5);
        assert_eq!(quads[0].color, [0.0, 0.0, 0.0, 0.5]);
        assert_eq!(quads[1].rect, [88.0, 48.0, 8.0, 8.0]);
        assert!(
            quads
                .iter()
                .skip(1)
                .all(|quad| quad.color == [1.0, 1.0, 1.0, 0.5])
        );
        quads.clear();
        title_opening_overlay(&mut quads, &atlas, 2, 0.0);
        assert!(quads.is_empty());
    }

    #[test]
    fn source_opening_fades_interpolate_real_snapshots_without_advancing_simulation() {
        let atlas = Atlas::load().unwrap();
        let mut game = Game::shell(7, RulesPreset::Classic);
        spelunky_core::opening::begin_intro(&mut game);
        let opening = game.opening.as_mut().unwrap();
        opening.draw_status = 0;
        opening.previous_fade_level = 0.8;
        opening.fade_level = 0.4;
        let hash = game.state_hash();
        for (alpha, opacity) in [(0.0, 0.8), (0.5, 0.6), (1.0, 0.4)] {
            let quads = game_scene(&game, alpha, false, &atlas);
            let overlay = quads.last().unwrap();
            assert_eq!(overlay.rect, [0.0, 0.0, 320.0, 240.0]);
            assert!((overlay.color[3] - opacity).abs() < 0.00001);
            assert_eq!(game.state_hash(), hash);
        }

        let opening = game.opening.as_mut().unwrap();
        opening.kind = spelunky_core::opening::OpeningKind::Title;
        opening.stage = 1;
        opening.previous_darkness = 0.9;
        opening.darkness = 0.3;
        let hash = game.state_hash();
        for (alpha, opacity) in [(0.0, 0.9), (0.5, 0.6), (1.0, 0.3)] {
            let quads = game_scene(&game, alpha, false, &atlas);
            let credit = quads.last().unwrap();
            assert_eq!(credit.rect[1], 48.0);
            assert!((credit.color[3] - opacity).abs() < 0.00001);
            let overlay = &quads[quads.len() - "DEREK YU PRESENTS".len() - 1];
            assert_eq!(overlay.rect, [0.0, 0.0, 320.0, 240.0]);
            assert!((overlay.color[3] - opacity).abs() < 0.00001);
            assert_eq!(game.state_hash(), hash);
        }
        // A room discontinuity selects the new authoritative frame even when
        // the host's accumulator still contains a fractional render interval.
        game.flags.insert("render_discontinuity".into());
        let quads = game_scene(&game, 0.0, false, &atlas);
        assert!((quads.last().unwrap().color[3] - 0.3).abs() < 0.00001);
    }

    #[test]
    fn ending_fades_interpolate_real_snapshots_and_snap_at_room_discontinuities() {
        use spelunky_core::ending::{EndingKind, EndingStage, EndingState};
        let atlas = Atlas::load().unwrap();
        let mut game = Game::shell(1, RulesPreset::Classic);
        for (kind, stage) in [
            (EndingKind::Custom, EndingStage::Summary),
            (EndingKind::Campaign, EndingStage::Credits),
            (EndingKind::TitleCredits, EndingStage::Exit),
        ] {
            let mut ending = EndingState::new(&game, kind, stage);
            ending.fade_out = true;
            ending.previous_fade_level = 0.2;
            ending.fade_level = 0.8;
            game.ending = Some(ending);
            game.state = spelunky_core::RunState::Ending;
            let hash = game.state_hash();
            for (alpha, opacity) in [(0.0, 0.2), (0.5, 0.5), (1.0, 0.8)] {
                let quads = game_scene(&game, alpha, false, &atlas);
                let overlay = quads.last().unwrap();
                assert_eq!(overlay.rect, [0.0, 0.0, 320.0, 240.0]);
                assert!((overlay.color[3] - opacity).abs() < 0.00001);
                assert_eq!(game.state_hash(), hash);
            }
            // Core room entry resets the previous snapshot to the new room's
            // initial opacity, so none of the old room's fade leaks through.
            game.ending.as_mut().unwrap().previous_fade_level = 0.8;
            for alpha in [0.0, 0.5, 1.0] {
                let quads = game_scene(&game, alpha, false, &atlas);
                assert!((quads.last().unwrap().color[3] - 0.8).abs() < 0.00001);
            }
        }
        game.ending.as_mut().unwrap().previous_fade_level = 0.2;
        game.flags.insert("render_discontinuity".into());
        let quads = game_scene(&game, 0.0, false, &atlas);
        assert!((quads.last().unwrap().color[3] - 0.8).abs() < 0.00001);
    }

    #[test]
    fn title_shortcut_house_welcome_uses_source_room_and_camera_gate() {
        let atlas = Atlas::load().unwrap();
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.flags.extend(["hub".into(), "hub_title".into()]);
        game.player.pos.x = 255.0;
        game.camera.x = 1.0;
        let mut quads = Vec::new();
        hub_hud(&mut quads, &game, game.camera, &atlas);
        assert_eq!(quads[0].rect, [39.0, 224.0, 8.0, 8.0]);
        assert_eq!(quads.len(), "WELCOME TO THE SHORTCUT HOUSE!".len());
        for (x, camera) in [(256.0, 1.0), (255.0, 1.01)] {
            game.player.pos.x = x;
            game.camera.x = camera;
            quads.clear();
            hub_hud(&mut quads, &game, game.camera, &atlas);
            assert!(quads.is_empty());
        }
    }

    #[test]
    fn scores_show_zero_time_after_a_win_and_the_source_clear_warning() {
        let atlas = Atlas::load().unwrap();
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.flags.extend(["hub".into(), "hub_scores".into()]);
        game.player.pos.y = 180.0;
        game.data.insert("total_wins".into(), 1.0);
        let button = game
            .world
            .spawn("oButtonHighscore", Vec2::new(224.0, 192.0));
        let block = game.world.spawn("oPushBlock", Vec2::new(176.0, 192.0));
        let mut quads = Vec::new();
        hub_hud(&mut quads, &game, game.camera, &atlas);
        let mut expected = Vec::new();
        text(
            &mut expected,
            &atlas,
            "TIME:   0:00",
            128.0,
            96.0,
            1.0,
            WHITE,
        );
        let time: Vec<_> = quads.iter().filter(|quad| quad.rect[1] == 96.0).collect();
        assert_eq!(time.len(), expected.len());
        for (actual, expected) in time.iter().zip(expected) {
            assert_eq!(actual.uv, expected.uv);
        }
        assert!(quads.iter().any(|quad| quad.rect[1] == 216.0));
        game.world.entity_mut(block).unwrap().pos.x = 160.0;
        quads.clear();
        hub_hud(&mut quads, &game, game.camera, &atlas);
        assert!(!quads.iter().any(|quad| quad.rect[1] == 216.0));
        game.world.entity_mut(block).unwrap().pos.x = 176.0;
        game.world
            .entity_mut(button)
            .unwrap()
            .data
            .insert("pushed".into(), 1.0);
        quads.clear();
        hub_hud(&mut quads, &game, game.camera, &atlas);
        assert!(!quads.iter().any(|quad| quad.rect[1] == 216.0));
    }

    #[test]
    fn scores_button_erases_local_unlock_text_before_persisted_records_reset() {
        let atlas = Atlas::load().unwrap();
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.flags
            .extend(["hub".into(), "hub_scores".into(), "unlock_sun".into()]);
        game.player.pos.y = 48.0;
        game.data.insert("best_money".into(), 500000.0);
        let mut quads = Vec::new();
        hub_hud(&mut quads, &game, game.camera, &atlas);
        assert_eq!(
            quads.iter().filter(|quad| quad.rect[1] == 112.0).count(),
            "KEEP YOURSELF AND".len()
        );
        // oButtonHighscore zeroes the oHighscores t* copies on its first
        // pressed Step. Persistent highscore values clear later at counter1.
        game.flags.insert("hub_records_erased".into());
        quads.clear();
        hub_hud(&mut quads, &game, game.camera, &atlas);
        let locked: Vec<_> = quads.iter().filter(|quad| quad.rect[1] == 112.0).collect();
        assert_eq!(locked.len(), "LOCKED.".len());
        assert_eq!(game.data.get("best_money"), Some(&500000.0));
    }

    #[test]
    fn cinematic_room_renders_interpolated_dummy_without_gameplay_player_hud_or_lighting() {
        use spelunky_core::ending::{EndingKind, EndingStage, EndingState};
        let atlas = Atlas::load().unwrap();
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.world.name = "rEnd".into();
        game.world.dark = true;
        game.player.pos = Vec2::new(250.0, 100.0);
        game.player.previous_pos = game.player.pos;
        game.player.sprite = "sStandLeft".into();
        game.previous_camera = Vec2::new(0.0, 16.0);
        game.camera = Vec2::new(2.0, 16.0);
        let id = game.world.spawn("oPDummy", Vec2::new(84.0, 80.0));
        let actor = game.world.entity_mut(id).unwrap();
        actor.previous_pos = Vec2::new(80.0, 80.0);
        actor.sprite = "sRunLeft".into();
        actor.facing = 1;
        game.ending = Some(EndingState::new(
            &game,
            EndingKind::Campaign,
            EndingStage::Chamber,
        ));
        game.state = spelunky_core::RunState::Ending;
        let quads = game_scene(&game, 0.5, false, &atlas);
        let run = atlas.sprite("sRunLeft", 0.0).unwrap();
        let actor = quads
            .iter()
            .find(|quad| quad.uv[1] == run.uv[1] && quad.uv[2] == -run.uv[2])
            .unwrap();
        assert_eq!(actor.rect, [73.0, 56.0, 16.0, 16.0]);
        assert!(quads.iter().all(|quad| quad.blend != QuadBlend::Multiply));
        assert!(
            quads
                .iter()
                .all(|quad| quad.uv != atlas.sprite("sStandLeft", 0.0).unwrap().uv)
        );
        assert_eq!(game.world.entity(id).unwrap().pos, Vec2::new(84.0, 80.0));
    }

    #[test]
    fn custom_ending_dummy_draw_always_faces_right() {
        let mut actor = Entity::new(1, "oPDummy6", Vec2::new(144.0, 184.0));
        actor.facing = -1;
        actor.data.insert("image_xscale".into(), 1.0);
        assert!(entity_mirrored(&actor, "sStandLeft"));
    }

    #[test]
    fn title_rope_exit_fade_covers_the_authored_title_scene() {
        use spelunky_core::ending::{EndingKind, EndingStage, EndingState};
        let atlas = Atlas::load().unwrap();
        let mut game = Game::shell(1, RulesPreset::Classic);
        let mut ending = EndingState::new(&game, EndingKind::TitleCredits, EndingStage::Exit);
        ending.fade_out = true;
        ending.fade_level = 0.6;
        game.ending = Some(ending);
        game.state = spelunky_core::RunState::Ending;
        let quads = game_scene(&game, 1.0, false, &atlas);
        let overlay = quads.last().unwrap();
        assert_eq!(overlay.rect, [0.0, 0.0, 320.0, 240.0]);
        assert_eq!(overlay.color, [0.0, 0.0, 0.0, 0.6]);
    }

    #[test]
    fn source_ending_summary_reveals_original_fonts_coordinates_and_clock() {
        let atlas = Atlas::load().unwrap();
        let mut hidden = Vec::new();
        ending_summary(&mut hidden, &atlas, 0, 200000, 549000, 12, 3, false, 0.0);
        assert!(hidden.is_empty());
        let mut quads = Vec::new();
        ending_summary(&mut quads, &atlas, 7, 200000, 549000, 12, 3, false, 0.0);
        assert_eq!(quads[0].rect, [64.0, 32.0, 16.0, 16.0]);
        assert_eq!(quads[0].color, [1.0, 1.0, 0.0, 1.0]);
        let mut expected = Vec::new();
        text(&mut expected, &atlas, "9:09", 120.0, 96.0, 1.0, WHITE);
        let time: Vec<_> = quads
            .iter()
            .filter(|quad| quad.rect[1] == 96.0 && quad.rect[0] >= 120.0)
            .collect();
        assert_eq!(time.len(), 4);
        for (actual, expected) in time.into_iter().zip(expected) {
            assert_eq!(actual.rect, expected.rect);
            assert_eq!(actual.uv, expected.uv);
        }
    }

    #[test]
    fn source_hero_inscription_draws_after_the_full_black_fade() {
        let atlas = Atlas::load().unwrap();
        let mut quads = Vec::new();
        ending_summary(&mut quads, &atlas, 8, 200000, 549000, 12, 3, true, 1.0);
        let fade = quads
            .iter()
            .position(|quad| quad.rect == [0.0, 0.0, 320.0, 240.0])
            .unwrap();
        assert_eq!(quads[fade].color, [0.0, 0.0, 0.0, 1.0]);
        assert_eq!(quads[fade + 1].rect, [24.0, 116.0, 8.0, 8.0]);
        assert_eq!(
            quads.len() - fade - 1,
            "YOU SHALL BE REMEMBERED AS A HERO.".len()
        );
    }

    #[test]
    fn source_credits_render_each_page_without_advancing_the_world() {
        let atlas = Atlas::load().unwrap();
        let mut quads = Vec::new();
        ending_credits(&mut quads, &atlas, 1);
        assert_eq!(quads.len(), 8);
        assert_eq!(quads[0].rect, [16.0, 16.0, 16.0, 16.0]);
        quads.clear();
        ending_credits(&mut quads, &atlas, 5);
        let final_row: Vec<_> = quads.iter().filter(|quad| quad.rect[1] == 104.0).collect();
        assert_eq!(final_row.len(), 6);
        assert_eq!(final_row[0].rect, [128.0, 104.0, 8.0, 8.0]);
        let mut expected = Vec::new();
        text(&mut expected, &atlas, "ZAPHOS", 128.0, 104.0, 1.0, WHITE);
        for (actual, expected) in final_row.into_iter().zip(expected) {
            assert_eq!(actual.uv, expected.uv);
        }
    }

    #[test]
    fn ending_rooms_use_original_backgrounds_and_authored_volcano() {
        let atlas = Atlas::load().unwrap();
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.world.name = "rEnd3".into();
        let mut quads = Vec::new();
        background(&mut quads, &game, Vec2::default(), false, &atlas);
        assert_eq!(quads.len(), 1);
        assert_eq!(quads[0].uv, atlas.sprite("bgClouds", 0.0).unwrap().uv);
        game.world.name = "rEnd2".into();
        quads.clear();
        background(&mut quads, &game, Vec2::default(), false, &atlas);
        assert!(quads.is_empty(), "the source oEnd2BG supplies this room");
    }

    #[test]
    fn darkness_mask_covers_every_pixel_once_with_a_flat_union_and_exact_black_outside() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.player.pos = Vec2::new(80.0, 120.0);
        game.player.previous_pos = game.player.pos;
        game.data.insert("darkness".into(), 0.5);
        game.world.spawn("oLamp", Vec2::new(96.0, 112.0));
        let mut quads = Vec::new();
        darkness(&mut quads, &game, Vec2::default(), 1.0);
        let mut coverage = vec![0; 320 * 240];
        let mut colors = vec![[0.0; 4]; 320 * 240];
        for quad in &quads {
            assert_eq!(quad.blend, QuadBlend::Multiply);
            assert_eq!(quad.rect[3], 1.0);
            for x in quad.rect[0] as usize..(quad.rect[0] + quad.rect[2]) as usize {
                let index = quad.rect[1] as usize * 320 + x;
                coverage[index] += 1;
                colors[index] = quad.color;
            }
        }
        assert!(
            coverage.iter().all(|count| *count == 1),
            "overlap must multiply only once"
        );
        let tint = [128.0 / 255.0, 128.0 / 255.0, 1.0, 1.0];
        assert_eq!(colors[120 * 320 + 80], tint);
        assert_eq!(colors[120 * 320 + 180], tint);
        assert_eq!(colors[220 * 320 + 300], [0.0, 0.0, 0.0, 1.0]);
        assert!(
            quads.len() < 1200,
            "single lamp needs fewer quads than the old 8px overlay"
        );
    }

    #[test]
    fn source_mask_rgb_rounds_to_even_and_adds_overflow_before_extracting_bytes() {
        assert_eq!(
            source_mask_rgb(127.5, 178.5, 255.0),
            [128.0 / 255.0, 178.0 / 255.0, 1.0]
        );
        assert_eq!(
            source_mask_rgb(331.5, 331.5, 255.0),
            [76.0 / 255.0, 77.0 / 255.0, 0.0]
        );
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    #[ignore = "requires a native or software GPU; run explicitly for renderer acceptance"]
    fn gpu_darkness_multiplies_rgb_bytes_once_and_preserves_hud_on_linear_and_srgb_targets() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.player.pos = Vec2::new(80.0, 120.0);
        game.player.previous_pos = game.player.pos;
        game.world.spawn("oLamp", Vec2::new(96.0, 112.0));
        for (level_darkness, lit_pixel) in [(0.5, [102, 51, 51, 255]), (-0.3, [61, 31, 0, 255])] {
            game.data.insert("darkness".into(), level_darkness);
            let mut quads = vec![DrawQuad {
                rect: [0.0, 0.0, 320.0, 240.0],
                color: [0.8, 0.4, 0.2, 1.0],
                ..Default::default()
            }];
            darkness(&mut quads, &game, Vec2::default(), 1.0);
            rect(&mut quads, [280.0, 8.0, 16.0, 16.0], [1.0, 0.5, 0.0, 1.0]);
            for srgb in [false, true] {
                let pixels = crate::renderer::tests::gpu_pixels(&quads, srgb);
                for (x, y, expected) in [
                    (80, 120, lit_pixel),
                    (180, 120, lit_pixel),
                    (300, 220, [0, 0, 0, 255]),
                    (288, 16, [255, 128, 0, 255]),
                ] {
                    let actual = &pixels[(y * 320 + x) * 4..(y * 320 + x) * 4 + 4];
                    assert!(
                        actual
                            .iter()
                            .zip(expected)
                            .all(|(a, b)| (i32::from(*a) - b).abs() <= 1),
                        "darkness={level_darkness}, target sRGB={srgb}, pixel({x},{y}): {actual:?} expected{expected:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn source_emitter_families_and_authored_lava_tiles_supply_their_original_circles() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        for (index, object) in [
            "oGhost",
            "oScarab",
            "oFireFrog",
            "oSpearTrapLit",
            "oSmashTrapLit",
            "oExplosion",
            "oFlare",
            "oTikiTorch",
        ]
        .into_iter()
        .enumerate()
        {
            let position = Vec2::new(40.0 + index as f64 * 20.0, 100.0);
            game.world.spawn(object, position);
        }
        game.world.set_tile(10, 8, "oLava");
        let lights = scene_lights(&game, 1.0);
        for (position, radius) in [
            (Vec2::new(56.0, 116.0), 64.0),
            (Vec2::new(68.0, 108.0), 16.0),
            (Vec2::new(88.0, 108.0), 32.0),
            (Vec2::new(108.0, 108.0), 32.0),
            (Vec2::new(128.0, 108.0), 32.0),
            (Vec2::new(140.0, 100.0), 96.0),
            (Vec2::new(160.0, 100.0), 96.0),
            (Vec2::new(188.0, 108.0), 32.0),
            (Vec2::new(168.0, 136.0), 32.0),
        ] {
            assert!(
                lights
                    .iter()
                    .any(|light| light.position == position && light.radius == radius),
                "missing source emitter at{position:?}"
            );
        }
        assert!(lights.iter().all(|light| light.tint == lights[0].tint));
    }

    #[test]
    fn game_step_updates_player_circle_for_crown_near_lamps_and_throwable_flares() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.world.dark = true;
        game.player.pos = Vec2::new(80.0, 120.0);
        game.player.previous_pos = game.player.pos;
        game.step(&spelunky_core::InputFrame::default());
        assert!((scene_lights(&game, 1.0)[0].radius - 38.4).abs() < 1e-9);
        game.player.equipment.insert("Crown".into());
        game.step(&spelunky_core::InputFrame::default());
        assert_eq!(scene_lights(&game, 1.0)[0].radius, 96.0);
        assert_eq!(scene_lights(&game, 1.0)[0].tint, [1.0; 3]);
        game.player.equipment.remove("Crown");
        let lamp = game.world.spawn("oLamp", game.player.pos);
        game.step(&spelunky_core::InputFrame::default());
        assert_eq!(
            scene_lights(&game, 1.0)[0].radius,
            96.0,
            "nearby lamp expands player light"
        );
        game.world.entity_mut(lamp).unwrap().alive = false;
        let flare = game.world.spawn("oFlare", Vec2::new(200.0, 120.0));
        game.step(&spelunky_core::InputFrame::default());
        let position = game.world.entity(flare).unwrap().pos;
        assert!(
            scene_lights(&game, 1.0)
                .iter()
                .any(|light| light.position == position && light.radius == 96.0),
            "actual flare uses its source light radius after item update"
        );
    }

    #[test]
    fn explosion_flash_retains_negative_source_darkness_and_expanded_player_radius() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.world.dark = true;
        game.player.health = 20;
        game.player.pos = Vec2::new(80.0, 120.0);
        game.player.previous_pos = game.player.pos;
        let id = game.world.spawn("oExplosion", game.player.pos);
        spelunky_core::items::init_entity(game.world.entity_mut(id).unwrap());
        game.world.entity_mut(id).unwrap().animation = 3.0;
        game.step(&spelunky_core::InputFrame::default());
        assert_eq!(game.data.get("darkness"), Some(&-0.3));
        assert!((scene_lights(&game, 1.0)[0].radius - 115.2).abs() < 1e-9);
        assert_eq!(
            scene_lights(&game, 1.0)[0].tint,
            [76.0 / 255.0, 77.0 / 255.0, 0.0]
        );
    }

    #[test]
    fn latched_dead_players_are_not_obscured_by_darkness_even_after_kapala_healing() {
        let atlas = Atlas::load().unwrap();
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.world.dark = true;
        assert!(
            game_scene(&game, 1.0, false, &atlas)
                .iter()
                .any(|q| q.blend == QuadBlend::Multiply)
        );
        game.state = spelunky_core::RunState::Dead;
        game.player.state = PlayerState::Dead;
        game.player.health = 1;
        assert!(
            !game_scene(&game, 1.0, false, &atlas)
                .iter()
                .any(|q| q.blend == QuadBlend::Multiply)
        );
    }

    #[test]
    fn generated_xoc_mural_draws_thirty_cropped_fragments_above_matching_blocks() {
        let atlas = Atlas::load().unwrap();
        let mut game = Game::shell(1, RulesPreset::Classic);
        let flags = std::collections::BTreeSet::from([
            "city_of_gold".into(),
            "murderer".into(),
            "character_damsel".into(),
        ]);
        game.world =
            spelunky_core::generation::generate(&mut spelunky_core::Rng::new(0), 13, &flags);
        let fragments: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|entity| entity.sprite == "bgLadyXoc")
            .cloned()
            .collect();
        assert_eq!(fragments.len(), 30, "source 5-by-6 mural");
        game.camera = Vec2::new(fragments[0].pos.x - 32.0, fragments[0].pos.y - 32.0);
        game.previous_camera = game.camera;
        let quads = game_scene(&game, 1.0, false, &atlas);
        for fragment in fragments {
            let frame = atlas
                .region(
                    "bgLadyXoc",
                    [
                        fragment.value("source_x") as u32,
                        fragment.value("source_y") as u32,
                        16,
                        16,
                    ],
                )
                .unwrap();
            let rect = [
                (fragment.pos.x - game.camera.x) as f32,
                (fragment.pos.y - game.camera.y) as f32,
                16.0,
                16.0,
            ];
            let mural = quads
                .iter()
                .position(|q| q.uv == frame.uv && q.rect == rect)
                .expect("one original mural segment");
            let block_uv = atlas
                .sprite(catalog::sprite("oXocBlock").unwrap(), 0.0)
                .unwrap()
                .uv;
            let block = quads
                .iter()
                .position(|q| q.uv == block_uv && q.rect[..2] == rect[..2])
                .unwrap();
            assert!(
                mural > block,
                "source mural depth99 must cover depth100 block"
            );
        }
    }

    #[test]
    fn authored_title_hint_uses_source_animation_and_strict_world_proximity_prompt() {
        let atlas = Atlas::load().unwrap();
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.flags.insert("first_time".into());
        game.flags.insert("title_start:1".into());
        game.load_hub(spelunky_core::hub::HubKind::Title);
        let hand = game
            .world
            .entities
            .iter()
            .find(|e| e.object == "oHintHand")
            .expect("first-time authored title creates original hint")
            .id;
        assert_eq!(
            game.world.entity(hand).unwrap().pos,
            Vec2::new(384.0, 144.0)
        );
        for step in 1..=10 {
            game.step(&spelunky_core::InputFrame::default());
            let hand = game.world.entity(hand).unwrap();
            let draw = game.presentation().world.entity(hand.id).unwrap();
            assert!((draw.animation - f64::from(step - 1) * 0.5).abs() < 0.00001);
            let frame = atlas.sprite("sHintPoint", draw.animation).unwrap();
            let quad = game_scene(&game, 1.0, false, &atlas)
                .into_iter()
                .find(|q| q.uv == frame.uv)
                .expect("source half-frame animation");
            assert_eq!(quad.rect, [64.0, 144.0, 16.0, 16.0]);
            assert_eq!(hand.sprite, "sHintPoint");
        }
        // Check the strict Draw-coordinate gate on a standalone copy of the
        // captured source frame, without modifying a running frame's snapshot.
        let mut game = game.presentation().clone();
        let prompt_uv = atlas
            .sprite("sFontSmall", f64::from('P' as u32 - 32))
            .unwrap()
            .uv;
        for (player_x, visible) in [
            (384.0, false),
            (384.01, true),
            (392.0, true),
            (399.99, true),
            (400.0, false),
        ] {
            game.player.pos.x = player_x;
            game.player.previous_pos = game.player.pos;
            for (camera_x, expected_x) in [(320.0, 88.0), (312.0, 96.0)] {
                game.camera.x = camera_x;
                game.previous_camera = game.camera;
                let quads = game_scene(&game, 0.5, false, &atlas);
                assert_eq!(
                    quads
                        .iter()
                        .any(|q| q.uv == prompt_uv && q.rect[..2] == [expected_x, 216.0]),
                    visible,
                    "strict source x test at {player_x}, camera {camera_x}"
                );
            }
        }
    }

    #[test]
    fn jaws_keeps_complete_body_at_source_damage_stages_and_directions() {
        let atlas = Atlas::load().unwrap();
        for (health, damage) in [(40, 1), (20, 1), (19, 2), (10, 2), (9, 3)] {
            for (head, side, offset) in [("sJawsLeft", "L", 16.0), ("sJawsRight", "R", -48.0)] {
                let mut game = Game::shell(1, RulesPreset::Classic);
                let id = game.world.spawn("oJaws", Vec2::new(120.0, 80.0));
                let jaws = game.world.entity_mut(id).unwrap();
                jaws.sprite = head.into();
                jaws.health = health;
                jaws.facing = 1;
                let quads = game_scene(&game, 1.0, false, &atlas);
                let body = atlas
                    .sprite(&format!("sJawsBody{damage}{side}"), 0.0)
                    .unwrap();
                let quad = quads
                    .iter()
                    .find(|q| q.uv == body.uv)
                    .expect("complete Mega Mouth body");
                assert_eq!(
                    quad.rect,
                    [
                        120.0 + offset - body.origin[0],
                        80.0 - body.origin[1],
                        48.0,
                        32.0
                    ]
                );
            }
        }
    }

    #[test]
    fn boss_custom_draw_does_not_inherit_enemy_flip_but_alien_does() {
        let atlas = Atlas::load().unwrap();
        for (object, name) in [
            ("oYetiKing", "sYetiKingTurnL"),
            ("oTombLord", "sTombLordTurnL"),
            ("oAlien", "sAlien"),
        ] {
            let mut game = Game::shell(1, RulesPreset::Classic);
            let id = game.world.spawn(object, Vec2::new(120.0, 80.0));
            let entity = game.world.entity_mut(id).unwrap();
            entity.sprite = name.into();
            entity.facing = 1;
            let frame = atlas.sprite(name, 0.0).unwrap();
            let quads = game_scene(&game, 1.0, false, &atlas);
            let quad = quads
                .iter()
                .find(|q| {
                    q.uv[1] == frame.uv[1]
                        && q.uv[3] == frame.uv[3]
                        && q.rect[1] == 80.0 - frame.origin[1]
                })
                .unwrap();
            assert_eq!(quad.uv[2].is_sign_negative(), object == "oAlien");
            assert_eq!(quad.rect[0], 120.0 - frame.origin[0]);
        }
    }

    #[test]
    fn armed_shopkeeper_draws_weapon_only_in_source_active_states() {
        let atlas = Atlas::load().unwrap();
        for (state, armed, visible) in [
            (0, true, false),
            (5, true, false),
            (2, true, true),
            (2, false, false),
        ] {
            let mut game = Game::shell(1, RulesPreset::Classic);
            let id = game.world.spawn("oShopkeeper", Vec2::new(120.0, 80.0));
            let keeper = game.world.entity_mut(id).unwrap();
            keeper.state = state;
            keeper.facing = 1;
            keeper
                .data
                .insert("has_gun".into(), if armed { 1.0 } else { 0.0 });
            let gun = atlas.sprite("sShotgunRight", 0.0).unwrap();
            let quads = game_scene(&game, 1.0, false, &atlas);
            assert_eq!(
                quads.iter().any(|q| q.uv == gun.uv
                    && q.rect[..2] == [130.0 - gun.origin[0], 90.0 - gun.origin[1]]),
                visible
            );
        }
    }

    #[test]
    fn spectacles_reveal_items_udjat_reveals_gems_and_frozen_cavemen_are_visible() {
        let atlas = Atlas::load().unwrap();
        let mut game = Game::shell(1, RulesPreset::Classic);
        for (x, object) in [(56.0, "oRubyBig"), (88.0, "oJetpack")] {
            game.world.set_tile((x / 16.0) as usize, 4, "oBrick");
            let id = game.world.spawn("oTerrainMetadata", Vec2::new(x, 72.0));
            let metadata = game.world.entity_mut(id).unwrap();
            metadata.data.insert("embedded_treasure".into(), 1.0);
            metadata.text.insert("treasure".into(), object.into());
        }
        game.world.set_tile(7, 4, "oIce");
        let id = game.world.spawn("oTerrainMetadata", Vec2::new(120.0, 72.0));
        game.world
            .entity_mut(id)
            .unwrap()
            .data
            .insert("frozen_caveman".into(), 1.0);
        let gem = atlas.sprite("sRubyBig", 0.0).unwrap().uv;
        let item = atlas.sprite("sJetpackRight", 0.0).unwrap().uv;
        let frozen = atlas.sprite("sFrozenCaveman", 0.0).unwrap().uv;
        for (equipment, gems, items) in [
            (None, false, false),
            (Some("UdjatEye"), true, false),
            (Some("Spectacles"), true, true),
        ] {
            game.player.equipment.clear();
            if let Some(name) = equipment {
                game.player.equipment.insert(name.into());
            }
            let quads = game_scene(&game, 1.0, false, &atlas);
            assert_eq!(quads.iter().any(|q| q.uv == gem && q.rect[1] > 40.0), gems);
            assert_eq!(
                quads.iter().any(|q| q.uv == item && q.rect[1] > 40.0),
                items
            );
            assert!(
                quads
                    .iter()
                    .any(|q| q.uv == frozen && q.rect[..2] == [112.0, 64.0])
            );
        }
    }

    #[test]
    fn priced_merchandise_and_unrolled_wagered_dice_show_source_markers() {
        let atlas = Atlas::load().unwrap();
        let mut game = Game::shell(1, RulesPreset::Classic);
        let id = game.world.spawn("oDamsel", Vec2::new(120.0, 80.0));
        game.world
            .entity_mut(id)
            .unwrap()
            .data
            .insert("cost".into(), 5000.0);
        let id = game.world.spawn("oDice", Vec2::new(160.0, 80.0));
        game.player.data.insert("bet".into(), 1500.0);
        let quads = game_scene(&game, 1.0, false, &atlas);
        assert!(
            quads
                .iter()
                .any(|q| q.uv == atlas.sprite("sSmallCollect", 0.0).unwrap().uv)
        );
        assert!(
            quads
                .iter()
                .any(|q| q.uv == atlas.sprite("sRedArrowDown", 0.0).unwrap().uv)
        );
        game.world
            .entity_mut(id)
            .unwrap()
            .data
            .insert("rolled".into(), 1.0);
        assert!(
            !game_scene(&game, 1.0, false, &atlas)
                .iter()
                .any(|q| q.uv == atlas.sprite("sRedArrowDown", 0.0).unwrap().uv)
        );
    }

    #[test]
    fn dead_player_hides_gameplay_hud_while_corpse_remains_visible() {
        let atlas = Atlas::load().unwrap();
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.player.state = PlayerState::Dead;
        game.player.facing = -1;
        game.player.sprite = "sDieL".into();
        game.player.pos = Vec2::new(120.0, 80.0);
        game.player.previous_pos = game.player.pos;
        let quads = game_scene(&game, 1.0, false, &atlas);
        assert!(
            !quads
                .iter()
                .any(|q| q.uv == atlas.sprite("sHeart", 0.0).unwrap().uv)
        );
        assert!(
            quads
                .iter()
                .any(|q| q.uv == atlas.sprite("sDieL", 0.0).unwrap().uv)
        );
    }

    #[test]
    fn game_step_charged_bow_displays_arrow_at_interpolated_owner_until_release() {
        let atlas = Atlas::load().unwrap();
        for right in [false, true] {
            let mut game = Game::shell(1, RulesPreset::Classic);
            game.player.pos = Vec2::new(120.0, 80.0);
            game.player.previous_pos = game.player.pos;
            game.player.facing = if right { 1 } else { -1 };
            let id = game.world.spawn("oBow", game.player.pos);
            game.player.held = Some(id);
            game.world.entity_mut(id).unwrap().held = true;
            game.data.insert("arrows".into(), 6.0);
            game.step(&spelunky_core::InputFrame {
                attack: true,
                attack_pressed: true,
                ..Default::default()
            });
            assert_eq!(game.player.data.get("bow_armed"), Some(&1.0));
            for bomb in [false, true] {
                if bomb {
                    game.step(&spelunky_core::InputFrame {
                        attack: true,
                        bomb_pressed: true,
                        ..Default::default()
                    });
                    assert_eq!(game.player.data.get("bow_bomb_armed"), Some(&1.0));
                }
                let name = match (bomb, right) {
                    (false, false) => "sArrowLeft",
                    (false, true) => "sArrowRight",
                    (true, false) => "sBombArrowLeft",
                    (true, true) => "sBombArrowRight",
                };
                let frame = atlas
                    .sprite(
                        name,
                        if bomb
                            && game
                                .player
                                .data
                                .get("bow_arrow_toggle")
                                .copied()
                                .unwrap_or(0.0)
                                == 0.0
                        {
                            1.0
                        } else {
                            0.0
                        },
                    )
                    .unwrap();
                for alpha in [0.0, 0.5, 1.0] {
                    let owner = game.player.previous_pos.lerp(game.player.pos, alpha);
                    let camera = game.previous_camera.lerp(game.camera, alpha);
                    let quads = game_scene(&game, alpha, false, &atlas);
                    let quad = quads
                        .iter()
                        .find(|q| q.uv == frame.uv && q.rect[1] > 40.0)
                        .unwrap();
                    assert!(
                        (quad.rect[0]
                            - (owner.x - camera.x) as f32
                            - if right { 4.0 } else { -4.0 }
                            + frame.origin[0])
                            .abs()
                            < 0.001
                    );
                    assert!(
                        (quad.rect[1] - (owner.y - camera.y) as f32 - if bomb { 2.0 } else { 1.0 }
                            + frame.origin[1])
                            .abs()
                            < 0.001
                    );
                }
            }
            game.step(&spelunky_core::InputFrame::default());
            assert_eq!(game.player.data.get("bow_armed"), Some(&0.0));
        }
    }

    #[test]
    fn actual_enemy_contact_blinks_every_step_without_hiding_invulnerability_alone() {
        let atlas = Atlas::load().unwrap();
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.world = spelunky_core::World::new(20, 20, Area::Mines, 1);
        for x in 0..20 {
            game.world.set_tile(x, 10, "oBlock");
        }
        game.player.pos = Vec2::new(88.0, 152.0);
        game.player.previous_pos = game.player.pos;
        game.player.facing = -1;
        game.player.state = PlayerState::Standing;
        game.player.grounded = true;
        game.player.invincible = 60;
        game.step(&spelunky_core::InputFrame::default());
        let visible = |game: &Game| {
            let mut uv = atlas
                .sprite(&game.player.sprite, game.player.animation)
                .unwrap()
                .uv;
            if game.player.facing > 0 {
                uv[0] += uv[2];
                uv[2] = -uv[2];
            }
            game_scene(game, 1.0, false, &atlas)
                .iter()
                .any(|q| q.uv == uv)
        };
        assert!(visible(&game), "invulnerability alone must remain visible");
        game.player.invincible = 0;
        let snake = game.world.spawn("oSnake", Vec2::new(80.0, 144.0));
        game.step(&spelunky_core::InputFrame::default());
        assert_eq!(game.player.health, 3);
        assert!(visible(&game));
        game.world.entity_mut(snake).unwrap().alive = false;
        for step in 1..=30 {
            game.step(&spelunky_core::InputFrame::default());
            assert_eq!(visible(&game), step % 2 == 0, "blink step {step}");
        }
        game.step(&spelunky_core::InputFrame::default());
        assert!(visible(&game));
    }

    #[test]
    fn rotated_projectile_corner_at_viewport_edge_is_not_culled_before_rotation() {
        let atlas = Atlas::load().unwrap();
        let mut game = Game::shell(1, RulesPreset::Classic);
        let id = game.world.spawn("oArrow", Vec2::new(80.0, -5.0));
        let arrow = game.world.entity_mut(id).unwrap();
        arrow.sprite = "sArrowRight".into();
        arrow
            .data
            .insert("rotation".into(), std::f64::consts::FRAC_PI_4);
        let quads = game_scene(&game, 1.0, false, &atlas);
        let frame = atlas.sprite("sArrowRight", 0.0).unwrap();
        let quad = quads
            .iter()
            .find(|q| q.uv == frame.uv)
            .expect("rotated corner reaches into view");
        assert_eq!(quad.rect, [76.0, -9.0, 8.0, 8.0]);
        assert_eq!(quad.rotation, std::f32::consts::FRAC_PI_4);
    }

    #[test]
    fn game_step_lava_surface_animation_is_authoritative_at_every_render_alpha() {
        let atlas = Atlas::load().unwrap();
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.player.pos = Vec2::new(40.0, 40.0);
        game.player.previous_pos = game.player.pos;
        game.world.set_tile(5, 5, "oLava");
        let id = game.world.spawn("oTerrainMetadata", Vec2::new(88.0, 88.0));
        game.world
            .entity_mut(id)
            .unwrap()
            .text
            .insert("terrain_sprite".into(), "sLavaTop".into());
        for step in 1..=7 {
            game.step(&spelunky_core::InputFrame::default());
            let animation = game.world.entity(id).unwrap().animation;
            assert!((animation - f64::from(step) * 0.4).abs() < 0.0001);
            let draw_animation = game.presentation().world.entity(id).unwrap().animation;
            assert!((draw_animation - f64::from(step - 1) * 0.4).abs() < 0.0001);
            let uv = atlas.sprite("sLavaTop", draw_animation).unwrap().uv;
            let hash = game.state_hash();
            for alpha in [0.0, 0.25, 0.5, 1.0] {
                assert!(
                    game_scene(&game, alpha, false, &atlas)
                        .iter()
                        .any(|q| q.uv == uv)
                );
                assert_eq!(game.state_hash(), hash);
            }
            assert_eq!(game.world.entity(id).unwrap().animation, animation);
        }
    }

    #[test]
    fn actual_lava_death_draws_corpse_behind_surface() {
        let atlas = Atlas::load().unwrap();
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.player.pos = Vec2::new(120.0, 75.0);
        game.player.previous_pos = game.player.pos;
        game.player.facing = -1;
        game.world.set_tile(7, 5, "oLava");
        game.step(&spelunky_core::InputFrame::default());
        assert_eq!(game.player.state, PlayerState::Dead);
        game.step(&spelunky_core::InputFrame::default());
        let quads = game_scene(&game, 1.0, false, &atlas);
        let player_uv = atlas
            .sprite(&game.player.sprite, game.player.animation)
            .unwrap()
            .uv;
        let corpse = quads.iter().position(|q| q.uv == player_uv).unwrap();
        let lava = quads
            .iter()
            .position(|q| q.uv == atlas.sprite("sLava", 0.0).unwrap().uv)
            .unwrap();
        assert!(
            corpse < lava,
            "source depth999 burns beneath the lava surface"
        );
    }

    #[test]
    fn integrated_rising_platform_and_rider_preserve_offset_at_fractional_alpha() {
        let atlas = Atlas::load().unwrap();
        let mut game = Game::shell(1, RulesPreset::Classic);
        let id = game.world.spawn("oThwompTrap", Vec2::new(80.0, 80.0));
        let platform = game.world.entity_mut(id).unwrap();
        platform.state = 3;
        platform.vel.y = -1.0;
        game.player.pos = Vec2::new(88.0, 73.0);
        game.player.state = PlayerState::Standing;
        game.player.grounded = true;
        game.step(&spelunky_core::InputFrame::default());

        // Game's snapshot must precede the source global platform pass. The
        // character tick must not discard that pass's one-pixel carry delta.
        let platform = game.world.entity(id).unwrap();
        assert_eq!(platform.previous_pos, Vec2::new(80.0, 80.0));
        assert_eq!(platform.pos, Vec2::new(80.0, 79.0));
        assert_eq!(game.player.previous_pos, Vec2::new(88.0, 73.0));
        assert_eq!(game.player.pos, Vec2::new(88.0, 72.0));
        let platform_uv = atlas
            .sprite(&platform.sprite, platform.animation)
            .unwrap()
            .uv;
        let mut player_uv = atlas
            .sprite(&game.player.sprite, game.player.animation)
            .unwrap()
            .uv;
        player_uv[0] += player_uv[2];
        player_uv[2] = -player_uv[2];
        let hash = game.state_hash();
        for alpha in [0.0, 0.125, 0.25, 0.5, 0.875, 1.0] {
            let quads = game_scene(&game, alpha, false, &atlas);
            let platform = quads.iter().find(|quad| quad.uv == platform_uv).unwrap();
            let player = quads.iter().find(|quad| quad.uv == player_uv).unwrap();
            assert_eq!(platform.rect[1], 80.0 - alpha as f32);
            assert_eq!(player.rect[1], 65.0 - alpha as f32);
            assert_eq!(platform.rect[1] - player.rect[1], 15.0);
            assert_eq!(game.state_hash(), hash);
        }
    }

    #[test]
    fn newly_created_whip_stays_at_source_offset_during_intermediate_frames() {
        let atlas = Atlas::load().unwrap();
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.player.sprite = "sAttackLeft".into();
        game.player.previous_pos = Vec2::new(40.0, 40.0);
        game.player.pos = Vec2::new(48.0, 40.0);
        game.previous_camera = Vec2::default();
        game.camera = Vec2::new(8.0, 0.0);
        let id = game.world.spawn("oWhip", Vec2::new(64.0, 40.0));
        game.world.entity_mut(id).unwrap().sprite = "sWhipRight".into();
        let whip_uv = atlas.sprite("sWhipRight", 0.0).unwrap().uv;
        let mut player_uv = atlas.sprite("sAttackLeft", 0.0).unwrap().uv;
        player_uv[0] += player_uv[2];
        player_uv[2] = -player_uv[2];
        for alpha in [0.0, 0.125, 0.25, 0.5, 0.875, 1.0] {
            let quads = game_scene(&game, alpha, false, &atlas);
            let player = quads.iter().find(|quad| quad.uv == player_uv).unwrap();
            let whip = quads.iter().find(|quad| quad.uv == whip_uv).unwrap();
            assert_eq!(whip.rect[0] - player.rect[0], 16.0);
        }
        assert_eq!(game.player.pos, Vec2::new(48.0, 40.0));
        assert_eq!(game.world.entity(id).unwrap().pos, Vec2::new(64.0, 40.0));
    }

    #[test]
    fn held_item_turns_with_owner_without_sweeping_through_the_body() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.player.previous_pos = Vec2::new(40.0, 40.0);
        game.player.pos = Vec2::new(48.0, 40.0);
        let id = game.world.spawn("oRock", Vec2::new(52.0, 42.0));
        game.player.held = Some(id);
        let rock = game.world.entity_mut(id).unwrap();
        rock.held = true;
        rock.previous_pos = Vec2::new(36.0, 42.0);
        assert_eq!(
            entity_position(&game, game.world.entity(id).unwrap(), 0.125),
            Vec2::new(45.0, 42.0)
        );
        game.world.entity_mut(id).unwrap().held = false;
        assert_eq!(
            entity_position(&game, game.world.entity(id).unwrap(), 0.125),
            Vec2::new(38.0, 42.0)
        );
    }

    #[test]
    fn all_playable_characters_keep_original_center_and_flip_animation_origins() {
        let atlas = Atlas::load().unwrap();
        for (standing, flip) in [
            ("sStandLeft", "sDuckToHangL"),
            ("sDamselLeft", "sDamselDtHL"),
            ("sTunnelLeft", "sTunnelDtHL"),
        ] {
            let mut quads = Vec::new();
            sprite(&mut quads, &atlas, standing, 0.0, 40.0, 40.0, false, WHITE);
            sprite(&mut quads, &atlas, standing, 0.0, 40.0, 40.0, true, WHITE);
            sprite(&mut quads, &atlas, flip, 0.0, 40.0, 40.0, false, WHITE);
            sprite(&mut quads, &atlas, flip, 0.0, 40.0, 40.0, true, WHITE);
            assert_eq!(quads[0].rect, [32.0, 32.0, 16.0, 16.0]);
            assert_eq!(quads[1].rect, [32.0, 32.0, 16.0, 16.0]);
            assert_eq!(quads[2].rect, [16.0, 32.0, 32.0, 32.0]);
            assert_eq!(quads[3].rect, [32.0, 32.0, 32.0, 32.0]);
        }
    }

    #[test]
    fn challenge_hud_uses_source_icons_and_countdown_instead_of_campaign_hud() {
        let atlas = Atlas::load().unwrap();
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.flags.extend(["hub".into(), "hub_moon".into()]);
        game.data.insert("baskets".into(), 42.0);
        game.data.insert("timer".into(), -1.0);
        game.data.insert("hub_draw_status".into(), 1.0);
        let mut quads = Vec::new();
        hud(&mut quads, &game, Vec2::default(), &atlas);
        assert!(
            quads
                .iter()
                .any(|quad| quad.uv == atlas.sprite("sHoopsIcon", 0.0).unwrap().uv)
        );
        assert!(
            quads
                .iter()
                .any(|quad| quad.uv == atlas.sprite("sTimerIcon", 0.0).unwrap().uv)
        );
        assert!(
            !quads
                .iter()
                .any(|quad| quad.uv == atlas.sprite("sBombIcon", 0.0).unwrap().uv)
        );
        let mut expected = Vec::new();
        text(
            &mut expected,
            &atlas,
            "ARCHERY CHALLENGE BEGINS IN 2...",
            40.0,
            216.0,
            1.0,
            [1.0, 1.0, 0.0, 1.0],
        );
        let countdown: Vec<_> = quads
            .iter()
            .filter(|quad| quad.rect[1] == 216.0)
            .map(|quad| quad.uv)
            .collect();
        assert_eq!(
            countdown,
            expected.iter().map(|quad| quad.uv).collect::<Vec<_>>()
        );
        game.data.insert("hub_draw_status".into(), 3.0);
        quads.clear();
        hud(&mut quads, &game, Vec2::default(), &atlas);
        assert!(!quads.iter().any(|quad| quad.rect[1] == 216.0));
    }

    #[test]
    fn scores_hud_renders_persisted_challenge_and_campaign_records() {
        let atlas = Atlas::load().unwrap();
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.flags.extend(["hub".into(), "hub_scores".into()]);
        game.data.insert("best_mini1".into(), 42.0);
        game.data.insert("best_money".into(), 200000.0);
        let mut quads = Vec::new();
        hub_hud(&mut quads, &game, game.camera, &atlas);
        let mut expected = Vec::new();
        text(&mut expected, &atlas, "SUN:   42", 128.0, 48.0, 1.0, WHITE);
        assert_eq!(
            quads
                .iter()
                .filter(|quad| quad.rect[1] == 48.0)
                .map(|quad| quad.uv)
                .collect::<Vec<_>>(),
            expected.iter().map(|quad| quad.uv).collect::<Vec<_>>()
        );
        game.player.pos.y = 180.0;
        quads.clear();
        expected.clear();
        hub_hud(&mut quads, &game, game.camera, &atlas);
        text(
            &mut expected,
            &atlas,
            "MONEY:  200000",
            128.0,
            48.0,
            1.0,
            WHITE,
        );
        assert_eq!(
            quads
                .iter()
                .filter(|quad| quad.rect[1] == 48.0)
                .map(|quad| quad.uv)
                .collect::<Vec<_>>(),
            expected.iter().map(|quad| quad.uv).collect::<Vec<_>>()
        );
    }

    #[test]
    fn projectile_rotation_uses_velocity_radians_and_original_origin_pivot() {
        let atlas = Atlas::load().unwrap();
        let mut game = Game::shell(1, RulesPreset::Classic);
        let id = game.world.spawn("oArrow", Vec2::new(120.0, 100.0));
        let arrow = game.world.entity_mut(id).unwrap();
        arrow.sprite = "sArrowRight".into();
        arrow
            .data
            .insert("rotation".into(), std::f64::consts::FRAC_PI_2);
        let uv = atlas.sprite("sArrowRight", 0.0).unwrap().uv;
        let quads = game_scene(&game, 1.0, false, &atlas);
        let quad = quads.iter().find(|quad| quad.uv == uv).unwrap();
        assert_eq!(quad.rotation, std::f32::consts::FRAC_PI_2);
        assert_eq!(
            [quad.rect[0] + quad.pivot[0], quad.rect[1] + quad.pivot[1]],
            [120.0, 100.0]
        );
    }

    #[test]
    fn static_and_carried_lamps_preserve_source_radius_and_center_offsets() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.world.spawn("oLamp", Vec2::new(20.0, 30.0));
        let id = game.world.spawn("oLampItem", Vec2::new(150.0, 150.0));
        let lamp = game.world.entity_mut(id).unwrap();
        lamp.data.insert("light_radius".into(), 96.0);
        lamp.data.insert("light_offset_y".into(), -4.0);
        lamp.data.insert("light_offset_x".into(), 3.0);
        let lights = scene_lights(&game, 1.0);
        assert!(
            lights
                .iter()
                .any(|light| light.position == Vec2::new(28.0, 38.0) && light.radius == 96.0)
        );
        assert!(
            lights
                .iter()
                .any(|light| light.position == Vec2::new(153.0, 146.0) && light.radius == 96.0)
        );
        assert_eq!(
            illumination(&lights, Vec2::new(153.0, 146.0)),
            lights[0].tint
        );
    }

    #[test]
    fn red_lamp_range_uses_nearest_class_and_tints_every_circle() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.data.insert("darkness".into(), 0.9);
        game.player.pos = Vec2::new(100.0, 100.0);
        game.player.previous_pos = game.player.pos;
        let item = game.world.spawn("oLampRedItem", Vec2::new(400.0, 400.0));
        let initial = scene_lights(&game, 1.0)[0].tint;
        assert_eq!(initial[0], 26.0 / 255.0);
        assert_eq!(initial[2], 1.0);
        game.world.spawn("oLampRed", Vec2::new(104.0, 100.0));
        // A far lamp created later must not suppress the nearer static lamp.
        game.world.spawn("oLampRed", Vec2::new(400.0, 400.0));
        game.world.spawn("oLamp", Vec2::new(120.0, 120.0));
        let static_tint = scene_lights(&game, 1.0)[0].tint;
        assert!(static_tint[0] > static_tint[1]);
        game.world.entity_mut(item).unwrap().pos = Vec2::new(150.0, 100.0);
        let distance = sprite_distance(
            game.player.sprite_bounds(),
            game.world.entity(item).unwrap().sprite_bounds(),
        );
        let lights = scene_lights(&game, 1.0);
        let expected = source_mask_rgb(255.0 - distance, 24.0 + distance, 24.0 + distance);
        assert_eq!(
            lights.len(),
            4,
            "static red lamps inherit Lamp circles; carried red lamps only tint"
        );
        assert!(lights.iter().all(|light| light.tint == expected));
        assert_ne!(
            expected, static_tint,
            "carried-lamp class overrides the static class"
        );
    }

    #[test]
    fn alien_ship_variant_crops_source_background_and_solid_boundaries_are_visible() {
        let atlas = Atlas::load().unwrap();
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.world.set_tile(0, 0, "oSolid");
        let id = game
            .world
            .spawn("oGenerationDecoration", Vec2::new(100.0, 100.0));
        let decoration = game.world.entity_mut(id).unwrap();
        decoration.sprite = "bgAlienShip".into();
        decoration.data.insert("variant".into(), 1.0);
        let quads = game_scene(&game, 1.0, false, &atlas);
        assert!(
            quads
                .iter()
                .any(|quad| quad.rect == [100.0, 100.0, 32.0, 64.0]
                    && quad.uv == atlas.region("bgAlienShip", [32, 0, 32, 64]).unwrap().uv)
        );
        assert!(quads.iter().any(|quad| quad.rect[..2] == [0.0, 0.0]
            && quad.uv == atlas.sprite("sBrick", 0.0).unwrap().uv));
    }

    #[test]
    fn inherited_enemy_flip_preserves_top_left_sprite_footprint() {
        let atlas = Atlas::load().unwrap();
        let mut normal = Vec::new();
        let mut mirrored = Vec::new();
        sprite(
            &mut normal,
            &atlas,
            "sSnakeLeft",
            0.0,
            entity_draw_x("oSnake", 40.0, false),
            50.0,
            false,
            WHITE,
        );
        sprite(
            &mut mirrored,
            &atlas,
            "sSnakeLeft",
            0.0,
            entity_draw_x("oSnake", 40.0, true),
            50.0,
            true,
            WHITE,
        );
        assert_eq!(normal[0].rect, [40.0, 50.0, 16.0, 16.0]);
        assert_eq!(normal[0].rect, mirrored[0].rect);
        assert_eq!(normal[0].uv[2], -mirrored[0].uv[2]);
        // The enemy source applies a constant +16 even for substituted sprites
        // with a different origin. Other objects flip around their own origin.
        let mut centered = Vec::new();
        sprite(
            &mut centered,
            &atlas,
            "sStandLeft",
            0.0,
            entity_draw_x("oSnake", 40.0, true),
            50.0,
            true,
            WHITE,
        );
        assert_eq!(centered[0].rect[0], 48.0);
        assert_eq!(entity_draw_x("oPlayer1", 40.0, true), 40.0);
    }

    #[test]
    fn terrain_metadata_selects_original_override_sprite_and_hidden_items_are_omitted() {
        let atlas = Atlas::load().unwrap();
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.world.set_tile(3, 4, "oBrick");
        let id = game.world.spawn("oTerrainMetadata", Vec2::new(56.0, 72.0));
        game.world
            .entity_mut(id)
            .unwrap()
            .text
            .insert("terrain_sprite".into(), "sDarkSmooth".into());
        let id = game.world.spawn("oRock", Vec2::new(120.0, 80.0));
        game.world
            .entity_mut(id)
            .unwrap()
            .data
            .insert("hidden".into(), 1.0);
        let quads = game_scene(&game, 0.0, false, &atlas);
        let terrain_uv = atlas.sprite("sDarkSmooth", 0.0).unwrap().uv;
        assert!(
            quads
                .iter()
                .any(|quad| quad.uv == terrain_uv && quad.rect[..2] == [48.0, 64.0])
        );
        assert!(
            !quads
                .iter()
                .any(|quad| quad.uv == atlas.sprite("sRock", 0.0).unwrap().uv)
        );
    }

    #[test]
    fn explicit_discontinuity_snaps_player_and_camera_even_for_short_teleports() {
        let atlas = Atlas::load().unwrap();
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.player.sprite = "sStandLeft".into();
        game.player.previous_pos = Vec2::new(40.0, 40.0);
        game.player.pos = Vec2::new(56.0, 40.0);
        game.previous_camera = Vec2::default();
        game.camera = Vec2::new(8.0, 0.0);
        game.flags.insert("render_discontinuity".into());
        let mut uv = atlas.sprite("sStandLeft", 0.0).unwrap().uv;
        uv[0] += uv[2];
        uv[2] = -uv[2];
        let quads = game_scene(&game, 0.0, false, &atlas);
        let player = quads.iter().find(|quad| quad.uv == uv).unwrap();
        assert_eq!(player.rect[..2], [40.0, 32.0]);
    }

    #[test]
    fn high_refresh_positions_are_fractional_and_pixel_snap_is_explicit() {
        let previous = Vec2::new(40.0, 40.0);
        let current = Vec2::new(42.0, 40.0);
        assert_eq!(
            screen(previous, current, Vec2::default(), 0.125, false),
            [40.25, 40.0]
        );
        assert_eq!(
            screen(previous, current, Vec2::default(), 0.125, true),
            [40.0, 40.0]
        );
    }

    #[test]
    fn camera_and_entity_interpolate_together_and_teleports_do_not_sweep() {
        let camera = interpolated(Vec2::new(10.0, 0.0), Vec2::new(12.0, 0.0), 0.25);
        assert_eq!(
            screen(
                Vec2::new(40.0, 40.0),
                Vec2::new(42.0, 40.0),
                camera,
                0.25,
                false
            ),
            [30.0, 40.0]
        );
        assert_eq!(
            interpolated(Vec2::default(), Vec2::new(500.0, 200.0), 0.5),
            Vec2::new(500.0, 200.0)
        );
    }

    #[test]
    fn player_sprite_moves_at_intermediate_frames_without_mutating_simulation() {
        let atlas = Atlas::load().unwrap();
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.player.sprite = "sStandLeft".into();
        game.player.previous_pos = Vec2::new(40.0, 40.0);
        game.player.pos = Vec2::new(42.0, 40.0);
        let mut uv = atlas.sprite("sStandLeft", 0.0).unwrap().uv;
        uv[0] += uv[2];
        uv[2] = -uv[2];
        let find_player = |alpha| {
            game_scene(&game, alpha, false, &atlas)
                .into_iter()
                .find(|quad| quad.uv == uv)
                .unwrap()
                .rect
        };
        let first = find_player(0.0);
        let second = find_player(0.125);
        assert_eq!(second[0] - first[0], 0.25);
        assert_eq!(game.player.pos, Vec2::new(42.0, 40.0));
        assert_eq!(game.tick, 0);
    }
}
