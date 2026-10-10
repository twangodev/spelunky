//! Builds a presentation snapshot without advancing simulation or animation.
use spelunky_core::{Area, Entity, Game, PlayerState, TICK_RATE, Vec2, catalog};
use std::collections::BTreeMap;

use crate::{atlas::Atlas, renderer::DrawQuad};

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
    if left + width < 0.0
        || left > 320.0
        || y - frame.origin[1] + height < 0.0
        || y - frame.origin[1] > 240.0
    {
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

/// Alpha interpolates positions only. Sprite frames and all HUD values are
/// simulation state, so presentation never changes gameplay or animation speed.
pub fn game_scene(game: &Game, alpha: f64, pixel_snap: bool, atlas: &Atlas) -> Vec<DrawQuad> {
    let alpha = if game.flags.contains("render_discontinuity") {
        1.0
    } else if alpha.is_finite() {
        alpha.clamp(0.0, 1.0)
    } else {
        1.0
    };
    let camera = interpolated(game.previous_camera, game.camera, alpha);
    let mut quads = Vec::with_capacity(1000);
    background(&mut quads, game, camera, pixel_snap, atlas);
    let mut layers: Vec<(i32, u64, Vec<DrawQuad>)> = Vec::new();
    let terrain_sprites: BTreeMap<(usize, usize), &str> = game
        .world
        .entities
        .iter()
        .filter(|entity| {
            entity.alive
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
                    name.as_str(),
                )
            })
        })
        .collect();
    for (index, tile) in game.world.tiles.iter().enumerate() {
        if tile.object.is_empty() {
            continue;
        }
        let tile_x = index % game.world.width.max(1);
        let tile_y = index / game.world.width.max(1);
        let Some(name) = terrain_sprites
            .get(&(tile_x, tile_y))
            .copied()
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
        sprite(&mut layer, atlas, name, 0.0, x, y, false, WHITE);
        layers.push((definition.map_or(100, |def| def.depth), index as u64, layer));
    }
    for entity in game.world.entities.iter().filter(|entity| entity.alive) {
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
        if entity.object == "oGenerationDecoration" {
            let region = if name == "bgAlienShip" {
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
                    } else if name == "bgDiceSign" {
                        9004.0
                    } else {
                        9005.0
                    }) as i32;
                layers.push((depth, 1_000_000 + entity.id, layer));
            }
            continue;
        }
        let scale = entity.value("image_xscale");
        let mirror = if entity.data.contains_key("image_xscale") {
            scale < 0.0
        } else {
            (name.ends_with("Left") || name.ends_with('L')) && entity.facing > 0
        };
        let alpha = entity
            .data
            .get("image_alpha")
            .copied()
            .unwrap_or(1.0)
            .clamp(0.0, 1.0) as f32;
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
        let depth = entity.data.get("depth").map_or_else(
            || definition.map_or(60, |def| def.depth),
            |depth| *depth as i32,
        );
        layers.push((depth, 1_000_000 + entity.id, layer));
    }
    let player = &game.player;
    if player.data.get("hidden") != Some(&1.0)
        && (player.invincible == 0 || (game.tick / 3).is_multiple_of(2))
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
                .any(|entity| entity.alive && entity.object == "oCape")
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
                .any(|entity| entity.alive && entity.object == "oParachute")
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
        if jetpack && climbing {
            sprite(&mut layer, atlas, "sJetpackBack", 0.0, x, y, false, WHITE);
        }
        layers.push((50, u64::MAX, layer));
    }
    // GameMaker draws larger depths first. Stable IDs break ties deterministically.
    layers.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    for (_, _, layer) in layers {
        quads.extend(layer);
    }
    if game.world.dark {
        darkness(&mut quads, game, camera, alpha);
    }
    hud(&mut quads, game, camera, atlas);
    quads
}

fn background(
    quads: &mut Vec<DrawQuad>,
    game: &Game,
    camera: Vec2,
    pixel_snap: bool,
    atlas: &Atlas,
) {
    let name = match game.world.area {
        // All three source level rooms start with the same cave background;
        // the temple changes it in scrInitLevel.
        Area::Mines | Area::Jungle | Area::Ice => "bgCave",
        Area::Temple | Area::Boss => "bgTemple",
    };
    let Some(frame) = atlas.sprite(name, 0.0) else {
        return;
    };
    let [width, height] = frame.size;
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

fn scene_lights(game: &Game, alpha: f64) -> Vec<Light> {
    let player = interpolated(game.player.previous_pos, game.player.pos, alpha);
    let darkness = game
        .data
        .get("darkness")
        .copied()
        .unwrap_or(1.0)
        .clamp(0.0, 1.0);
    let mut tint = [1.0; 3];
    for entity in game.world.entities.iter().filter(|entity| entity.alive) {
        let range = entity.data.get("red_light_range").copied().unwrap_or(
            if matches!(entity.object.as_str(), "oLampRed" | "oLampRedItem") {
                96.0
            } else {
                0.0
            },
        );
        let distance = player.distance(entity_position(game, entity, alpha));
        if range > 0.0 && distance <= range {
            // oScreen's red lamp changes the nearby player's light color;
            // it does not create a separate white light circle.
            let source_distance = distance / range * 96.0;
            tint = [
                (255.0 - source_distance) / 255.0,
                (24.0 + source_distance) / 255.0,
                (24.0 + source_distance) / 255.0,
            ];
        }
    }
    let mut lights = vec![Light {
        position: player,
        radius: 96.0 - 64.0 * darkness,
        tint,
    }];
    for entity in game.world.entities.iter().filter(|entity| entity.alive) {
        if matches!(entity.object.as_str(), "oLampRed" | "oLampRedItem") {
            continue;
        }
        let default_radius = match entity.object.as_str() {
            "oLamp" | "oLampItem" | "oFlare" | "oFlareCrate" => 96.0,
            "oArrowTrapLeftLit" | "oArrowTrapRightLit" => 32.0,
            "oTorch" | "oExplosion" => 64.0,
            _ => 0.0,
        };
        let radius = entity
            .data
            .get("light_radius")
            .copied()
            .unwrap_or(default_radius);
        if !radius.is_finite() || radius <= 0.0 {
            continue;
        }
        let mut position = entity_position(game, entity, alpha);
        let static_lamp = matches!(
            entity.object.as_str(),
            "oLamp" | "oArrowTrapLeftLit" | "oArrowTrapRightLit"
        );
        position.x += entity
            .data
            .get("light_offset_x")
            .copied()
            .unwrap_or(if static_lamp { 8.0 } else { 0.0 });
        position.y += entity
            .data
            .get("light_offset_y")
            .copied()
            .unwrap_or(if static_lamp {
                8.0
            } else if entity.object == "oLampItem" {
                -4.0
            } else {
                0.0
            });
        lights.push(Light {
            position,
            radius,
            tint: [1.0; 3],
        });
    }
    lights
}

fn illumination(lights: &[Light], point: Vec2) -> [f64; 3] {
    let mut result = [0.0_f64; 3];
    for light in lights {
        let strength = (1.0 - point.distance(light.position) / light.radius).clamp(0.0, 1.0);
        for (channel, tint) in result.iter_mut().zip(light.tint) {
            *channel = channel.max(strength * tint);
        }
    }
    result
}

fn darkness(quads: &mut Vec<DrawQuad>, game: &Game, camera: Vec2, alpha: f64) {
    let lights = scene_lights(game, alpha);
    // Source light radii/colors with a cheap soft falloff. Exact GameMaker
    // surface blending remains a separate visual parity task.
    for y in (0..240).step_by(8) {
        for x in (0..320).step_by(8) {
            let point = Vec2::new(camera.x + x as f64 + 4.0, camera.y + y as f64 + 4.0);
            let [red, green, blue] = illumination(&lights, point);
            let light = red.max(green).max(blue);
            let bounds = [x as f32, y as f32, 8.0, 8.0];
            rect(
                quads,
                bounds,
                [0.0, 0.0, 0.0, (0.96 * (1.0 - light)) as f32],
            );
            if red > green {
                rect(
                    quads,
                    bounds,
                    [0.65, 0.0, 0.0, ((red - green) * 0.65) as f32],
                );
            }
        }
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

fn hub_hud(quads: &mut Vec<DrawQuad>, game: &Game, atlas: &Atlas) {
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
        if game.flags.contains(flag) {
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
        if seconds > 0 {
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
    }
}

fn hud(quads: &mut Vec<DrawQuad>, game: &Game, camera: Vec2, atlas: &Atlas) {
    if game.flags.contains("hub") {
        hub_hud(quads, game, atlas);
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
    if let Some(held) = player.held.and_then(|id| game.world.entity(id)) {
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
    use super::*;
    use spelunky_core::RulesPreset;

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
        hub_hud(&mut quads, &game, &atlas);
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
        hub_hud(&mut quads, &game, &atlas);
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
        assert_eq!(illumination(&lights, Vec2::new(153.0, 146.0)), [1.0; 3]);
    }

    #[test]
    fn red_lamp_range_tints_player_light_without_emitting_white_circle() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.player.pos = Vec2::new(100.0, 100.0);
        game.player.previous_pos = game.player.pos;
        let id = game.world.spawn("oLampRedItem", Vec2::new(110.0, 100.0));
        game.world
            .entity_mut(id)
            .unwrap()
            .data
            .insert("red_light_range".into(), 8.0);
        assert_eq!(scene_lights(&game, 1.0)[0].tint, [1.0; 3]);
        game.world
            .entity_mut(id)
            .unwrap()
            .data
            .insert("red_light_range".into(), 16.0);
        let lights = scene_lights(&game, 1.0);
        assert_eq!(lights.len(), 1);
        assert!(lights[0].tint[0] > lights[0].tint[1]);
        let lamp = game.world.entity_mut(id).unwrap();
        lamp.object = "oLampRed".into();
        lamp.data.insert("light_radius".into(), 96.0);
        assert_eq!(scene_lights(&game, 1.0).len(), 1);
        let mut quads = Vec::new();
        darkness(&mut quads, &game, Vec2::default(), 1.0);
        assert!(
            quads
                .iter()
                .any(|quad| quad.color[0] > 0.0 && quad.color[1] == 0.0 && quad.color[3] > 0.0)
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
