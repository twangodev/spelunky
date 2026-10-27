//! Builds a presentation snapshot without advancing simulation or animation.
use spelunky_core::{Area, Entity, Game, Player, PlayerState, Vec2, catalog};
use std::collections::{HashMap, HashSet};

use crate::{
    atlas::Atlas,
    renderer::{DrawQuad, QuadBlend},
    viewport::Viewport,
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
    let living_moon_finish =
        game.flags.contains("hub_moon") && game.data.get("timer").copied().unwrap_or(0.0) < 0.0;
    if game.state != spelunky_core::RunState::Dead && !living_moon_finish {
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
    sprite_in_view(
        quads,
        atlas,
        name,
        frame,
        x,
        y,
        mirrored,
        color,
        Viewport::canonical(),
    );
}

fn sprite_in_view(
    quads: &mut Vec<DrawQuad>,
    atlas: &Atlas,
    name: &str,
    frame: f64,
    x: f32,
    y: f32,
    mirrored: bool,
    color: [f32; 4],
    view: Viewport,
) {
    let Some((frame, next_frame, frame_mix)) = atlas.sprite_transition(name, frame) else {
        return;
    };
    let [width, height] = frame.size;
    let mut uv = frame.uv;
    let mut next_uv = next_frame.uv;
    let left = if mirrored {
        uv[0] += uv[2];
        uv[2] = -uv[2];
        next_uv[0] += next_uv[2];
        next_uv[2] = -next_uv[2];
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
    if x + radius < 0.0 || x - radius > view.width || y + radius < 0.0 || y - radius > view.height {
        return;
    }
    quads.push(DrawQuad {
        rect: [left, y - frame.origin[1], width, height],
        uv,
        next_uv,
        frame_mix,
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

fn interpolated_angle(previous: f64, current: f64, alpha: f64) -> f64 {
    if !previous.is_finite() || !current.is_finite() || !alpha.is_finite() {
        return current;
    }
    let delta = (current - previous + std::f64::consts::PI).rem_euclid(std::f64::consts::TAU)
        - std::f64::consts::PI;
    previous + delta * alpha.clamp(0.0, 1.0)
}

pub(crate) fn interpolated_move_to(previous: Vec2, current: Vec2, alpha: f64) -> Vec2 {
    // Classic moveTo resolves horizontal motion before vertical motion.
    // Follow that same L-shaped route between simulation ticks so high-refresh
    // presentation cannot cut a collision-safe corner.
    if previous.distance(current) > 64.0 || !alpha.is_finite() {
        return current;
    }
    let alpha = alpha.clamp(0.0, 1.0);
    let dx = current.x - previous.x;
    let dy = current.y - previous.y;
    let horizontal = dx.abs();
    let vertical = dy.abs();
    let distance = (horizontal + vertical) * alpha;
    if distance <= horizontal && horizontal > 0.0 {
        Vec2::new(previous.x + dx.signum() * distance, previous.y)
    } else if vertical > 0.0 {
        Vec2::new(
            current.x,
            previous.y + dy.signum() * (distance - horizontal).clamp(0.0, vertical),
        )
    } else {
        current
    }
}

fn screen_player(
    previous: Vec2,
    current: Vec2,
    camera: Vec2,
    alpha: f64,
    pixel_snap: bool,
) -> [f32; 2] {
    let point = interpolated_move_to(previous, current, alpha);
    let mut result = [(point.x - camera.x) as f32, (point.y - camera.y) as f32];
    if pixel_snap {
        result = result.map(f32::round);
    }
    result
}

fn screen(previous: Vec2, current: Vec2, camera: Vec2, alpha: f64, pixel_snap: bool) -> [f32; 2] {
    let point = interpolated(previous, current, alpha);
    let mut result = [(point.x - camera.x) as f32, (point.y - camera.y) as f32];
    if pixel_snap {
        result = result.map(f32::round);
    }
    result
}

pub(crate) fn presentation_entity_animation(
    entity: &Entity,
    current_sprite: Option<&str>,
    _alpha: f64,
) -> f64 {
    if current_sprite == Some(entity.sprite.as_str()) {
        entity.animation
    } else if entity.animation.is_finite() {
        // Animation End runs on the authoritative tick. If it destroys this
        // instance or changes its sprite, retain the last source subimage.
        entity.animation.floor()
    } else {
        entity.animation
    }
}

pub(crate) fn presentation_player_animation(
    player: &Player,
    current_sprite: &str,
    _alpha: f64,
) -> f64 {
    if current_sprite == player.sprite {
        player.animation
    } else if player.animation.is_finite() {
        player.animation.floor()
    } else {
        player.animation
    }
}

fn entity_sprite<'a>(atlas: &Atlas, entity: &'a Entity) -> Option<&'a str> {
    if atlas.sprite(&entity.sprite, entity.animation).is_some() {
        return Some(&entity.sprite);
    }
    catalog::sprite(&entity.object)
}

fn entity_position(game: &Game, entity: &Entity, alpha: f64) -> Vec2 {
    let grabbed_monkey = entity.object == "oMonkey" && entity.state == 7;
    let attached = (entity.held && game.player.held == Some(entity.id))
        || matches!(
            entity.object.as_str(),
            "oWhip" | "oWhipPre" | "oMachetePre" | "oMattockPre" | "oSlash" | "oMattockHit"
        )
        || grabbed_monkey;
    if attached {
        // Source Step events attach these objects to the character, including
        // their very first frame. GRAB monkeys also set their position directly
        // from the Player each Step. Interpolating their independent previous
        // positions detaches attacks and makes monkeys cut across the owner's
        // collision-resolved path; only the owner's motion is continuous.
        let owner = interpolated_move_to(game.player.previous_pos, game.player.pos, alpha);
        Vec2::new(
            owner.x + entity.pos.x - game.player.pos.x,
            owner.y + entity.pos.y - game.player.pos.y,
        )
    } else if entity.object == "oDamsel"
        && matches!(entity.sprite.as_str(), "sDamselExit2" | "sPExit")
    {
        // oDamsel snaps to the exit door after inherited oItem movement. That
        // coordinate change is a source teleport, not a path through the room.
        entity.pos
    } else if source_uses_axis_resolved_motion(&entity.object) {
        // These source Step families resolve horizontal motion before vertical
        // motion through moveTo. Keep high-refresh interpolation on that same
        // collision-safe path rather than cutting a diagonal through corners.
        interpolated_move_to(entity.previous_pos, entity.pos, alpha)
    } else {
        interpolated(entity.previous_pos, entity.pos, alpha)
    }
}

fn source_uses_axis_resolved_motion(object: &str) -> bool {
    // The source oItem, oEnemy, oTreasure, and oDetritus Step bodies all call
    // moveTo; their descendants inherit or invoke that path. oBoulder has its
    // own Step that calls moveTo while inheriting from oMovingSolid.
    catalog::is_a(object, "oItem")
        || catalog::is_a(object, "oEnemy")
        || catalog::is_a(object, "oTreasure")
        || catalog::is_a(object, "oDetritus")
        || object == "oBoulder"
}

fn presented_entity_position(game: &Game, entity: &Entity, alpha: f64, frozen: bool) -> Vec2 {
    if frozen {
        entity.pos
    } else {
        entity_position(game, entity, alpha)
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

fn entity_mirrored(entity: &Entity, _name: &str) -> bool {
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
    } else if catalog::is_a(&entity.object, "oEnemy") {
        spelunky_core::enemies::source_facing(entity) > 0
    } else if entity.data.contains_key("image_xscale") {
        entity.value("image_xscale") < 0.0
    } else if matches!(entity.object.as_str(), "oWhip" | "oWhipPre") {
        // oDrawnSprite has no Draw override. These directional sprites keep
        // the engine's default +1 scale, independent of port-facing metadata.
        false
    } else {
        // Objects without a custom Draw override use the engine's default
        // +1 image_xscale. Directional sprite names and port AI facing do
        // not assign that source property (notably left muzzle flashes).
        false
    }
}

fn entity_draw_depth(game: &Game, entity: &Entity, fallback: i32) -> i32 {
    if let Some(depth) = entity.data.get("depth") {
        return *depth as i32;
    }
    if game.generation_revision == spelunky_core::GenerationRevision::SourceClassicV2
        && catalog::is_a(&entity.object, "oItem")
    {
        return if game.player.has("Spectacles") {
            51
        } else {
            101
        };
    }
    fallback
}

fn entity_draw_extras(
    layer: &mut Vec<DrawQuad>,
    game: &Game,
    entity: &Entity,
    current_sprite: Option<&str>,
    price_marker_frame: Option<f64>,
    atlas: &Atlas,
    x: f32,
    y: f32,
    animation_alpha: f64,
    view: Viewport,
) {
    let sprite =
        |quads: &mut Vec<DrawQuad>, atlas: &Atlas, name: &str, frame, x, y, mirrored, color| {
            sprite_in_view(quads, atlas, name, frame, x, y, mirrored, color, view);
        };
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
                    presentation_entity_animation(entity, current_sprite, animation_alpha),
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
    if let Some(frame) = price_marker_frame {
        // Classic samples cimg, draws this icon, then advances cimg in its
        // source Draw phase. The scene reuses that sampled frame at any refresh.
        sprite(
            layer,
            atlas,
            "sSmallCollect",
            frame,
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

/// Default Scene's atlas-backed generated graphic projection. This is a
/// presentation adapter, not a native tile raster/culling or conversion proof.
/// Unsupported nonfinite/fractional/negative crop data stays in model storage
/// and does not produce a guessed atlas rectangle.
fn generated_graphic_quad(
    graphic: &spelunky_core::GeneratedGraphicTileSpec,
    atlas: &Atlas,
    camera: Vec2,
    pixel_snap: bool,
) -> Option<DrawQuad> {
    if !graphic.visible
        || !graphic.depth.is_finite()
        || !graphic.position.x.is_finite()
        || !graphic.position.y.is_finite()
        || !graphic.scale.x.is_finite()
        || !graphic.scale.y.is_finite()
        || !graphic.alpha.is_finite()
        || graphic
            .source
            .iter()
            .any(|v| !v.is_finite() || *v < 0.0 || *v > f64::from(u32::MAX) || v.fract() != 0.0)
    {
        return None;
    }
    let frame = atlas.region(&graphic.background, graphic.source.map(|v| v as u32))?;
    let [x, y] = screen(graphic.position, graphic.position, camera, 1.0, pixel_snap);
    let width = (f64::from(frame.size[0]) * graphic.scale.x) as f32;
    let height = (f64::from(frame.size[1]) * graphic.scale.y) as f32;
    if ![x, y, width, height].iter().all(|v| v.is_finite()) {
        return None;
    }
    Some(DrawQuad {
        rect: [x, y, width, height],
        uv: frame.uv,
        color: [
            (graphic.color & 255) as f32 / 255.0,
            ((graphic.color >> 8) & 255) as f32 / 255.0,
            ((graphic.color >> 16) & 255) as f32 / 255.0,
            graphic.alpha.clamp(0.0, 1.0) as f32,
        ],
        ..DrawQuad::default()
    })
}

#[cfg(test)]
pub fn game_scene(game: &Game, alpha: f64, pixel_snap: bool, atlas: &Atlas) -> Vec<DrawQuad> {
    game_scene_in_view(game, alpha, pixel_snap, atlas, Viewport::canonical())
}

/// Draws the requested logical view; the host supplies the same extent to
/// gameplay activation at each fixed tick. Scratch storage belongs to the host,
/// not the simulation. Ranges index a
/// flat sprite arena, preserving each actor's internal Draw order after sorting.
#[derive(Default)]
pub struct SceneBuffers {
    pub quads: Vec<DrawQuad>,
    sprites: Vec<DrawQuad>,
    layers: Vec<(f64, u8, u64, std::ops::Range<usize>)>,
    source_entity_order: HashMap<u64, u64>,
    source_terrain_order: HashMap<usize, u64>,
    current_entity_index: HashMap<u64, (bool, usize)>,
    source_player_order: Option<u64>,
    visible_ids: HashSet<u64>,
    hidden_ids: HashSet<u64>,
    hidden_indices: Vec<usize>,
    terrain_sprites: HashMap<(usize, usize), (bool, usize)>,
}

#[cfg(test)]
pub fn game_scene_in_view(
    game: &Game,
    alpha: f64,
    pixel_snap: bool,
    atlas: &Atlas,
    view: Viewport,
) -> Vec<DrawQuad> {
    let mut buffers = SceneBuffers::default();
    game_scene_in_view_into(game, alpha, pixel_snap, atlas, view, &mut buffers);
    buffers.quads
}

pub fn game_scene_in_view_into(
    game: &Game,
    alpha: f64,
    pixel_snap: bool,
    atlas: &Atlas,
    view: Viewport,
    buffers: &mut SceneBuffers,
) {
    let SceneBuffers {
        quads,
        sprites,
        layers,
        source_entity_order,
        source_terrain_order,
        current_entity_index,
        source_player_order,
        visible_ids,
        hidden_ids,
        hidden_indices,
        terrain_sprites,
    } = buffers;
    quads.clear();
    sprites.clear();
    layers.clear();
    source_entity_order.clear();
    source_terrain_order.clear();
    current_entity_index.clear();
    *source_player_order = None;
    visible_ids.clear();
    hidden_ids.clear();
    hidden_indices.clear();
    terrain_sprites.clear();
    let current_game = game;
    let game = current_game.presentation();
    let has_previous_arrow_draw =
        game.level_tick == game.render_history.arrow_level_tick.saturating_add(1);
    for (index, entity) in current_game.world.entities.iter().enumerate() {
        if entity.alive {
            current_entity_index.insert(entity.id, (false, index));
        }
    }
    for (index, entity) in current_game.dispatcher.hidden_entities().iter().enumerate() {
        if entity.alive {
            current_entity_index.insert(entity.id, (true, index));
        }
    }
    // Classic draws the global active list after sorting it by depth. The
    // stable sort keeps the list's instance order for depth ties, including
    // instances appended by region reactivation. Reuse that per-frame rank as
    // the renderer's tie key so scene collection order and birth IDs cannot
    // reverse overlapping instances.
    let mut source_rank = 0_u64;
    for instance in game
        .dispatcher
        .active_order
        .iter()
        .chain(&game.dispatcher.inactive_order)
    {
        match instance {
            spelunky_core::dispatcher::Instance::Player => {
                *source_player_order = Some(source_rank);
            }
            spelunky_core::dispatcher::Instance::Entity(id) => {
                source_entity_order.entry(*id).or_insert(source_rank);
            }
            spelunky_core::dispatcher::Instance::Terrain(index) => {
                source_terrain_order.entry(*index).or_insert(source_rank);
            }
        }
        source_rank = source_rank.saturating_add(1);
    }
    let fallback_source_rank = source_rank.saturating_add(1);
    let alpha = if game.flags.contains("render_discontinuity") {
        1.0
    } else if alpha.is_finite() {
        alpha.clamp(0.0, 1.0)
    } else {
        1.0
    };
    let animation_alpha = if game.flags.contains("render_discontinuity") {
        0.0
    } else {
        alpha
    };
    // Keep camera follow phase-aligned with actors between fixed ticks. The
    // source moves actors on X and then Y; using the same path prevents a
    // tracked player from visibly drifting against a diagonally interpolated
    // viewport at high refresh rates.
    let canonical_camera = interpolated_move_to(game.previous_camera, game.camera, alpha);
    let expanded = view.width != 320.0 || view.height != 240.0;
    // Canonical helper preserves authored room framing, including small rooms.
    let camera = if expanded {
        view.camera(
            canonical_camera,
            game.world.width as f64 * 16.0,
            game.world.height as f64 * 16.0,
        )
    } else {
        canonical_camera
    };
    let sprite =
        |quads: &mut Vec<DrawQuad>, atlas: &Atlas, name: &str, frame, x, y, mirrored, color| {
            sprite_in_view(quads, atlas, name, frame, x, y, mirrored, color, view);
        };
    if expanded {
        visible_ids.extend(game.world.entities.iter().map(|entity| entity.id));
        for (index, entity) in game.dispatcher.hidden_entities().iter().enumerate() {
            if !visible_ids.contains(&entity.id) {
                hidden_indices.push(index);
                hidden_ids.insert(entity.id);
            }
        }
    }
    let hidden = hidden_indices
        .iter()
        .map(|&i| &game.dispatcher.hidden_entities()[i]);
    let entities = game
        .world
        .entities
        .iter()
        .filter(|entity| entity.engine_active)
        .chain(hidden.clone());
    // Keep visible grid traversal first; hidden storage is borrowed, never activated.
    let hidden_tile_storage = if expanded {
        game.dispatcher.hidden_tiles()
    } else {
        &[]
    };
    let hidden_tiles = hidden_tile_storage
        .iter()
        .filter(|(index, _)| {
            expanded
                && *index < game.world.tiles.len()
                && game.world.tiles[*index].object.is_empty()
        })
        .map(|(index, tile)| (*index, tile));
    let tiles = game
        .world
        .tiles
        .iter()
        .enumerate()
        .filter(|(_, tile)| !tile.object.is_empty() && tile.engine_active)
        .chain(hidden_tiles);
    // A cinematic uses the authored room's world, but its actors and Draw
    // controller replace the gameplay character, HUD and lighting pass.
    let cinematic = game
        .ending
        .as_ref()
        .filter(|ending| ending.stage != spelunky_core::ending::EndingStage::Exit);
    background_in_view(quads, game, camera, pixel_snap, atlas, view);
    // Match the old collect's last-writer wins behavior: hidden first, then
    // active physical metadata. Store locations, never stale borrowed sprites.
    for (hidden, index, entity) in hidden_indices
        .iter()
        .map(|&i| (true, i, &game.dispatcher.hidden_entities()[i]))
        .chain(
            game.world
                .entities
                .iter()
                .enumerate()
                .filter(|(_, e)| e.engine_active)
                .map(|(i, e)| (false, i, e)),
        )
    {
        if entity.alive
            && entity.object == "oTerrainMetadata"
            && entity.pos.x >= 0.0
            && entity.pos.y >= 0.0
            && entity.text.contains_key("terrain_sprite")
        {
            terrain_sprites.insert(
                (
                    (entity.pos.x / 16.0).floor() as usize,
                    (entity.pos.y / 16.0).floor() as usize,
                ),
                (hidden, index),
            );
        }
    }
    for (index, tile) in tiles {
        if tile.object.is_empty() {
            continue;
        }
        let tile_x = index % game.world.width.max(1);
        let tile_y = index / game.world.width.max(1);
        let override_sprite = terrain_sprites
            .get(&(tile_x, tile_y))
            .map(|&(hidden, index)| {
                let entity = if hidden {
                    &game.dispatcher.hidden_entities()[index]
                } else {
                    &game.world.entities[index]
                };
                (
                    entity.text["terrain_sprite"].as_str(),
                    entity.animation,
                    entity.data.get("depth").map(|depth| *depth as i32),
                )
            });
        let Some(name) = override_sprite
            .map(|(name, _, _)| name)
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
        let definition = catalog::object(&tile.object);
        if definition.is_some_and(|def| !def.visible) {
            continue;
        }
        let layer_start = sprites.len();
        let layer = &mut *sprites;
        sprite(
            layer,
            atlas,
            name,
            override_sprite.map_or(0.0, |(_, animation, _)| animation),
            x,
            y,
            false,
            WHITE,
        );
        let depth = override_sprite
            .and_then(|(_, _, depth)| depth)
            .unwrap_or_else(|| definition.map_or(100, |def| def.depth));
        let order = source_terrain_order
            .get(&index)
            .copied()
            .unwrap_or_else(|| fallback_source_rank.saturating_add(tile.creation_serial));
        layers.push((f64::from(depth), 0, order, layer_start..layer.len()));
    }
    for metadata in entities
        .clone()
        .filter(|entity| entity.alive && entity.object == "oTerrainMetadata")
    {
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
        let layer_start = sprites.len();
        let layer = &mut *sprites;
        sprite(layer, atlas, name, 0.0, x, y, false, WHITE);
        layers.push((
            f64::from(depth),
            0,
            source_entity_order
                .get(&metadata.id)
                .copied()
                .unwrap_or_else(|| fallback_source_rank.saturating_add(metadata.creation_serial)),
            layer_start..layer.len(),
        ));
    }
    for entity in entities.clone().filter(|entity| entity.alive) {
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
        // Hidden actors retain stale previous_pos while source simulation is frozen.
        let position =
            presented_entity_position(game, entity, alpha, hidden_ids.contains(&entity.id));
        let [x, y] = screen(position, position, camera, alpha, pixel_snap);
        let layer_start = sprites.len();
        let layer = &mut *sprites;
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
            text(layer, atlas, message, text_x, text_y, 1.0, WHITE);
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
                layers.push((
                    f64::from(depth),
                    0,
                    source_entity_order
                        .get(&entity.id)
                        .copied()
                        .unwrap_or_else(|| {
                            fallback_source_rank.saturating_add(entity.creation_serial)
                        }),
                    layer_start..layer.len(),
                ));
            }
            continue;
        }
        let mirror = entity_mirrored(entity, name);
        let source_alpha = entity
            .data
            .get("image_alpha")
            .copied()
            .unwrap_or(1.0)
            .clamp(0.0, 1.0);
        let alpha = if entity.object == "oWeb" {
            let previous_alpha = game
                .render_history
                .web_alpha
                .get(&entity.id)
                .copied()
                .unwrap_or(source_alpha)
                .clamp(0.0, 1.0);
            interpolate_effect(previous_alpha, source_alpha, alpha).clamp(0.0, 1.0) as f32
        } else {
            source_alpha as f32
        };
        let current_sprite = current_entity_index
            .get(&entity.id)
            .map(|&(hidden, index)| {
                if hidden {
                    current_game.dispatcher.hidden_entities()[index]
                        .sprite
                        .as_str()
                } else {
                    current_game.world.entities[index].sprite.as_str()
                }
            });
        let entity_animation =
            presentation_entity_animation(entity, current_sprite, animation_alpha);
        if matches!(entity.object.as_str(), "oPDummy" | "oPDummy2")
            && game.player.has("Jetpack")
            && !matches!(
                entity.sprite.as_str(),
                "sPExit" | "sDamselExit" | "sTunnelExit"
            )
        {
            sprite(
                layer,
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
            layer,
            atlas,
            name,
            entity_animation,
            entity_draw_x(&entity.object, x, mirror),
            y,
            mirror,
            [1.0, 1.0, 1.0, alpha],
        );
        if let Some(quad) = layer[layer_start..].last_mut() {
            // Fish Bone inherits oItem Draw's literal zero angle while its
            // collision angle stays in Game. Arrow owns Draw; smooth its
            // previous/current image_angle samples only in presentation.
            let current_rotation = entity.value("rotation");
            let rotation = match entity.object.as_str() {
                "oFishBone" => 0.0,
                "oArrow" => {
                    let previous_rotation = if has_previous_arrow_draw {
                        game.render_history
                            .arrow_rotation
                            .get(&(entity.id, entity.creation_serial))
                            .copied()
                            .unwrap_or(current_rotation)
                    } else {
                        current_rotation
                    };
                    interpolated_angle(previous_rotation, current_rotation, animation_alpha)
                }
                _ => current_rotation,
            };
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
        if game.world.name == "rLevelEditor"
            && matches!(entity.object.as_str(), "oEntrance" | "oExit")
        {
            if let Some(target) = entity
                .text
                .get("leadsTo")
                .filter(|target| !target.is_empty())
            {
                text(layer, atlas, target, x, y, 1.0, WHITE);
            }
        }
        let price_marker_frame = if spelunky_core::items::draws_price_marker(&entity.object)
            && entity.value("cost") > 0.0
        {
            let key = (entity.id, entity.creation_serial);
            Some(
                if game.render_history.price_marker_tick == Some(game.tick) {
                    game.render_history
                        .price_marker_frames
                        .get(&key)
                        .copied()
                        .unwrap_or_else(|| entity.value("cimg"))
                } else {
                    entity.value("cimg")
                },
            )
        } else {
            None
        };
        entity_draw_extras(
            layer,
            game,
            entity,
            current_sprite,
            price_marker_frame,
            atlas,
            x,
            y,
            animation_alpha,
            view,
        );
        let depth = entity_draw_depth(game, entity, definition.map_or(60, |def| def.depth));
        layers.push((
            f64::from(depth),
            0,
            source_entity_order
                .get(&entity.id)
                .copied()
                .unwrap_or_else(|| fallback_source_rank.saturating_add(entity.creation_serial)),
            layer_start..layer.len(),
        ));
    }
    let player = &game.player;
    if cinematic.is_none()
        && player.data.get("visible") != Some(&0.0)
        && player.data.get("hidden") != Some(&1.0)
        && player.data.get("blink_toggle") != Some(&1.0)
    {
        let [x, y] = screen_player(player.previous_pos, player.pos, camera, alpha, pixel_snap);
        let jetpack = player.has("Jetpack") || player.has("jetpack");
        let climbing = spelunky_core::player::source_climbing(game)
            || matches!(
                player.sprite.as_str(),
                "sPExit" | "sDamselExit" | "sTunnelExit"
            );
        // characterDrawEvent's early back-jetpack branch bypasses redColor.
        // Whipping uses the ordinary tinted character and side jetpack instead.
        let back_jetpack = jetpack && climbing && player.attack_timer == 0;
        if player.data.get("parachute_open").copied().unwrap_or(0.0) > 0.0
            && !game
                .world
                .entities
                .iter()
                .any(|entity| entity.alive && entity.engine_active && entity.object == "oParachute")
        {
            let canopy_start = sprites.len();
            let canopy = &mut *sprites;
            sprite(
                canopy,
                atlas,
                "sParachute",
                0.0,
                x - 8.0,
                y - 16.0,
                false,
                WHITE,
            );
            layers.push((900.0, 0, u64::MAX - 2, canopy_start..canopy.len()));
        }
        let layer_start = sprites.len();
        let layer = &mut *sprites;
        if jetpack && !back_jetpack {
            sprite(
                layer,
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
        let red = player.data.get("red_color").copied().unwrap_or(0.0);
        let color = if back_jetpack {
            WHITE
        } else {
            let previous = player_tint(game.render_history.player_red_color);
            let current = player_tint(red);
            interpolate_color(previous, current, alpha)
        };
        sprite(
            layer,
            atlas,
            name,
            presentation_player_animation(player, &current_game.player.sprite, animation_alpha),
            x,
            y,
            player.facing > 0,
            color,
        );
        if back_jetpack {
            // characterDrawEvent draws the back-facing jetpack immediately
            // after the character, before its held-arrow decoration.
            sprite(layer, atlas, "sJetpackBack", 0.0, x, y, false, WHITE);
        }
        let held_arrow = player.data.get("hold_arrow").copied().unwrap_or(0.0);
        if held_arrow > 0.0 {
            let bomb = held_arrow == 2.0;
            let right = player.facing > 0;
            let name = match (bomb, right) {
                (false, false) => "sArrowLeft",
                (false, true) => "sArrowRight",
                (true, false) => "sBombArrowLeft",
                (true, true) => "sBombArrowRight",
            };
            sprite(
                layer,
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
        let depth = player.data.get("depth").copied().unwrap_or_else(|| {
            if player.data.get("burning").copied().unwrap_or(0.0) > 0.0 {
                999.0
            } else {
                50.0
            }
        }) as i32;
        layers.push((
            f64::from(depth),
            0,
            // The source player is created before level terrain and actors.
            // This fallback is used only before its dispatcher entry exists.
            source_player_order.unwrap_or(0),
            layer_start..layer.len(),
        ));
    }
    for (index, tile) in game.world.generated_graphic_tiles.iter().enumerate() {
        if let Some(quad) = generated_graphic_quad(&tile.graphic, atlas, camera, pixel_snap) {
            // Graphics order is current record order, never a fabricated actor
            // identity. Finite equal-depth actors are emitted before graphics.
            let start = sprites.len();
            sprites.push(quad);
            layers.push((tile.graphic.depth, 2, index as u64, start..sprites.len()));
        }
    }
    // Equal-depth player, terrain and entity layers follow Classic's stable
    // global instance order. Generated graphics follow those instances at a
    // depth tie. partial_cmp preserves finite equal signed zero; total_cmp
    // would reorder it.
    layers.sort_by(|a, b| {
        b.0.partial_cmp(&a.0)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.1.cmp(&b.1))
            .then(a.2.cmp(&b.2))
    });
    for (_, _, _, range) in layers.iter() {
        quads.extend_from_slice(&sprites[range.clone()]);
    }
    let mut panel_start = quads.len();
    if let Some(ending) = cinematic {
        use spelunky_core::ending::EndingStage;
        let fade = interpolate_fade(ending.previous_fade_level, ending.fade_level, alpha);
        match ending.stage {
            EndingStage::Landing | EndingStage::Summary => ending_summary(
                quads,
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
                ending_credits(quads, atlas, ending.credits_page);
                if ending.fade_in || ending.fade_out {
                    cinematic_fade(quads, fade);
                }
            }
            EndingStage::Transition => transition_text(
                quads,
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
            darkness_in_view(quads, game, camera, alpha, view);
            // Darkness already occupies world coordinates, outside the fixed UI panel.
        }
        panel_start = quads.len();
        let [ox, oy] = view.ui_origin();
        // World-attached source HUD text retains world coordinates after panel translation.
        hud_in_view(
            quads,
            game,
            Vec2::new(camera.x + f64::from(ox), camera.y + f64::from(oy)),
            atlas,
            view,
        );
        if let Some(ending) = game.ending.as_ref().filter(|ending| {
            ending.kind == spelunky_core::ending::EndingKind::TitleCredits
                && ending.stage == spelunky_core::ending::EndingStage::Exit
                && ending.fade_out
        }) {
            cinematic_fade(
                quads,
                interpolate_fade(ending.previous_fade_level, ending.fade_level, alpha),
            );
        }
    }
    if let Some(opening) = &game.opening {
        match opening.kind {
            spelunky_core::opening::OpeningKind::Intro => intro_narrative(
                quads,
                atlas,
                opening.draw_status,
                interpolate_fade(opening.previous_fade_level, opening.fade_level, alpha),
                &opening.lines,
            ),
            spelunky_core::opening::OpeningKind::Title => {
                title_opening_overlay(
                    quads,
                    atlas,
                    opening.stage,
                    interpolate_fade(opening.previous_darkness, opening.darkness, alpha),
                );
            }
        }
    }
    place_authored_panel(&mut quads[panel_start..], view);
}
/// Center authored UI and expand the full-screen fades emitted by cinematic_fade.
fn place_authored_panel(quads: &mut [DrawQuad], view: Viewport) {
    let [x, y] = view.ui_origin();
    for quad in quads {
        if quad.rect == [0.0, 0.0, 320.0, 240.0]
            && quad.uv == [0.0; 4]
            && quad.color[..3] == [0.0; 3]
            && quad.blend == QuadBlend::Alpha
        {
            quad.rect = [0.0, 0.0, view.width, view.height];
        } else {
            quad.rect[0] += x;
            quad.rect[1] += y;
        }
    }
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

#[cfg(test)]
fn background(
    quads: &mut Vec<DrawQuad>,
    game: &Game,
    camera: Vec2,
    pixel_snap: bool,
    atlas: &Atlas,
) {
    background_in_view(
        quads,
        game,
        camera,
        pixel_snap,
        atlas,
        Viewport::canonical(),
    );
}

fn background_in_view(
    quads: &mut Vec<DrawQuad>,
    game: &Game,
    camera: Vec2,
    pixel_snap: bool,
    atlas: &Atlas,
    view: Viewport,
) {
    let sprite =
        |quads: &mut Vec<DrawQuad>, atlas: &Atlas, name: &str, frame, x, y, mirrored, color| {
            sprite_in_view(quads, atlas, name, frame, x, y, mirrored, color, view);
        };
    let (name, tiled) = match game.world.name.as_str() {
        "rEnd" | "rEndCustom" | "rTransition1" | "rTransition1x" | "rTransition2"
        | "rTransition2x" | "rTransition3" | "rTransition3x" => ("bgCave", true),
        "rTransition4" => ("bgTemple", true),
        "rEnd3" | "rCredits2" => ("bgClouds", false),
        "rIntro" | "rCredits1" => ("backgroundNight", true),
        // Preserve its authored center panel; repeat the existing background
        // beyond that panel only when the display reveals additional world.
        "rEnd2" if view.width != 320.0 || view.height != 240.0 => ("sEnd2BG", true),
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
    if !tiled && view.width == 320.0 && view.height == 240.0 {
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
    while y < view.height {
        let mut x = start_x;
        while x < view.width {
            sprite(quads, atlas, name, 0.0, x, y, false, WHITE);
            x += width;
        }
        y += height;
    }

    // rLevel2.xml authors these two full-width scenery rows at the bottom of
    // the Ice room. They sit behind the open abyss and fade the tiled cave
    // background to black; the source objects are static oMenu descendants.
    // Custom editor rooms use rLoadLevel and do not inherit these room objects.
    if game.world.area == Area::Ice && !game.flags.contains("custom") {
        // Greater GameMaker depths draw first, so preserve the oBlackBG then
        // oBlackFadeUp ordering where their one-pixel overlap is visible.
        for (sprite_name, world_y) in [("sBlack", 592.0_f32), ("sBlackFadeUp", 576.0_f32)] {
            for x in 0..42 {
                sprite(
                    quads,
                    atlas,
                    sprite_name,
                    0.0,
                    x as f32 * 16.0 - camera.x as f32,
                    world_y - camera.y as f32,
                    false,
                    WHITE,
                );
            }
        }
    }
}

#[derive(Debug)]
struct Light {
    position: Vec2,
    radius: f64,
    tint: [f64; 3],
}

// Classic 1.1 does not set circle precision; GameMaker 8's default is a
// 24-sided polygon. Keep the source draw-circle geometry instead of treating
// its edge as an analytic circle.
const SOURCE_CIRCLE_SEGMENTS: usize = 24;
type SourceCircleVertices = [[f64; 2]; SOURCE_CIRCLE_SEGMENTS];

fn source_circle_vertices(light: &Light) -> SourceCircleVertices {
    std::array::from_fn(|index| {
        let angle = std::f64::consts::TAU * index as f64 / SOURCE_CIRCLE_SEGMENTS as f64;
        [
            light.position.x + light.radius * angle.cos(),
            light.position.y + light.radius * angle.sin(),
        ]
    })
}

fn source_circle_scanline_span(vertices: &SourceCircleVertices, y: f64) -> Option<(f64, f64)> {
    let mut left = f64::INFINITY;
    let mut right = f64::NEG_INFINITY;
    let mut intersections = 0;
    for index in 0..SOURCE_CIRCLE_SEGMENTS {
        let first = vertices[index];
        let second = vertices[(index + 1) % SOURCE_CIRCLE_SEGMENTS];
        // Half-open vertical edges count a shared vertex once, matching the
        // non-antialiased scan conversion used for the source polygon.
        if (first[1] <= y && y < second[1]) || (second[1] <= y && y < first[1]) {
            let fraction = (y - first[1]) / (second[1] - first[1]);
            let x = first[0] + (second[0] - first[0]) * fraction;
            left = left.min(x);
            right = right.max(x);
            intersections += 1;
        }
    }
    (intersections >= 2).then_some((left, right))
}

fn darkness_enabled(game: &Game) -> bool {
    game.world.dark
        && game.player.state != PlayerState::Dead
        && game.state != spelunky_core::RunState::Dead
}

fn scene_lights(game: &Game, alpha: f64) -> Vec<Light> {
    scene_lights_in_view(game, alpha, None)
}

fn scene_lights_in_view(
    game: &Game,
    alpha: f64,
    presentation_view: Option<(Vec2, Viewport)>,
) -> Vec<Light> {
    let game = game.presentation();
    let presentation_view =
        presentation_view.filter(|(_, view)| view.width != 320.0 || view.height != 240.0);
    let player = interpolated_move_to(game.player.previous_pos, game.player.pos, alpha);
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
    if let Some((camera, view)) = presentation_view {
        let visible_ids: HashSet<_> = game.world.entities.iter().map(|entity| entity.id).collect();
        for entity in game.dispatcher.hidden_entities().iter().filter(|entity| {
            entity.alive && !entity.engine_active && !visible_ids.contains(&entity.id)
        }) {
            let Some((radius, dx, dy)) = source_light(&entity.object) else {
                continue;
            };
            let radius = entity.data.get("light_radius").copied().unwrap_or(radius);
            if !radius.is_finite() || radius <= 0.0 {
                continue;
            }
            let position = Vec2::new(
                entity.pos.x + entity.data.get("light_offset_x").copied().unwrap_or(dx),
                entity.pos.y + entity.data.get("light_offset_y").copied().unwrap_or(dy),
            );
            if circle_intersects_view(position, radius, camera, view) {
                lights.push(Light {
                    position,
                    radius,
                    tint,
                });
            }
        }
        for (index, tile) in game
            .dispatcher
            .hidden_tiles()
            .iter()
            .filter(|(index, tile)| {
                !tile.engine_active
                    && *index < game.world.tiles.len()
                    && game.world.tiles[*index].object.is_empty()
            })
        {
            let Some((radius, dx, dy)) = source_light(&tile.object) else {
                continue;
            };
            let position = Vec2::new(
                (*index % game.world.width.max(1)) as f64 * 16.0 + dx,
                (*index / game.world.width.max(1)) as f64 * 16.0 + dy,
            );
            if circle_intersects_view(position, radius, camera, view) {
                lights.push(Light {
                    position,
                    radius,
                    tint,
                });
            }
        }
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

fn circle_intersects_view(position: Vec2, radius: f64, camera: Vec2, view: Viewport) -> bool {
    let closest_x = position.x.clamp(camera.x, camera.x + f64::from(view.width));
    let closest_y = position
        .y
        .clamp(camera.y, camera.y + f64::from(view.height));
    let dx = position.x - closest_x;
    let dy = position.y - closest_y;
    dx * dx + dy * dy <= radius * radius
}

fn red_lamp_distances(game: &Game) -> [Option<f64>; 2] {
    spelunky_core::world_events::screen_red_lamp_distances(game.presentation())
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

fn player_tint(red: f64) -> [f32; 4] {
    if red > 0.0 {
        let [r, g, b] = source_mask_rgb(200.0 + red, 0.0, 0.0);
        [r as f32, g as f32, b as f32, 1.0]
    } else {
        WHITE
    }
}

fn interpolate_color(previous: [f32; 4], current: [f32; 4], alpha: f64) -> [f32; 4] {
    let alpha = if alpha.is_finite() {
        alpha.clamp(0.0, 1.0) as f32
    } else {
        1.0
    };
    if alpha == 0.0 {
        return previous;
    }
    if alpha == 1.0 {
        return current;
    }
    std::array::from_fn(|channel| {
        previous[channel] + (current[channel] - previous[channel]) * alpha
    })
}

fn interpolate_effect(previous: f64, current: f64, alpha: f64) -> f64 {
    if !previous.is_finite() || !current.is_finite() || !alpha.is_finite() {
        return current;
    }
    match alpha.clamp(0.0, 1.0) {
        0.0 => previous,
        1.0 => current,
        alpha => previous + (current - previous) * alpha,
    }
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

#[cfg(test)]
fn darkness(quads: &mut Vec<DrawQuad>, game: &Game, camera: Vec2, alpha: f64) {
    darkness_in_view(quads, game, camera, alpha, Viewport::canonical());
}

fn darkness_in_view(
    quads: &mut Vec<DrawQuad>,
    game: &Game,
    camera: Vec2,
    alpha: f64,
    view: Viewport,
) {
    let columns = view.width.ceil() as i32;
    let rows = view.height.ceil() as i32;
    let lights = scene_lights_in_view(game, alpha, Some((camera, view)));
    let tint = lights[0].tint.map(|channel| channel as f32);
    let mut intervals: Vec<(i32, i32)> = Vec::with_capacity(lights.len());
    let circles: Vec<_> = lights.iter().map(source_circle_vertices).collect();
    // oScreen builds one colored union of filled circles, then multiplies that
    // mask into the scene. Intersections must not multiply the scene twice.
    // Scanline runs retain logical-pixel edges without an offscreen framebuffer
    // or an arbitrary GPU light-count limit. Pixel centers are tested against
    // the default 24-sided GameMaker circle polygon.
    for y in 0..rows {
        intervals.clear();
        let sample_y = camera.y + f64::from(y) + 0.5;
        for vertices in &circles {
            let Some((left, right)) = source_circle_scanline_span(vertices, sample_y) else {
                continue;
            };
            let left = left - camera.x;
            let right = right - camera.x;
            let start = (left - 0.5).ceil().clamp(0.0, f64::from(columns)) as i32;
            let end = (right - 0.5).ceil().clamp(0.0, f64::from(columns)) as i32;
            if start < end {
                intervals.push((start, end));
            }
        }
        intervals.sort_unstable();
        let mut cursor = 0;
        let mut active: Option<(i32, i32)> = None;
        for (start, end) in intervals
            .iter()
            .copied()
            .chain(std::iter::once((columns + 1, columns + 1)))
        {
            if let Some((left, right)) = active {
                if start <= right {
                    active = Some((left, right.max(end)));
                    continue;
                }
                mask_run(quads, cursor, left, y, [0.0; 3]);
                mask_run(quads, left, right, y, tint);
                cursor = right;
            }
            active = (start < columns).then_some((start, end));
        }
        mask_run(quads, cursor, columns, y, [0.0; 3]);
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

fn hub_hud(quads: &mut Vec<DrawQuad>, game: &Game, camera: Vec2, atlas: &Atlas, view: Viewport) {
    let yellow = [1.0, 1.0, 0.0, 1.0];
    let (challenge, first_icon, first_value, second_icon, second_value) =
        if game.flags.contains("hub_sun") {
            (
                "DAMSEL",
                "sHeart",
                game.player.health.max(0.0) as f64,
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
                game.player.health.max(0.0) as f64,
                "sShopkeeperIcon",
                hub_value(game, "stars_kills"),
            )
        } else {
            ("", "", 0.0, "", 0.0)
        };
    if !challenge.is_empty() {
        let hud_start = quads.len();
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
        // Challenge Draw anchors these counters to view_xview/view_yview.
        // The countdown text below stays centered in the authored panel.
        offset_panel_translation(&mut quads[hud_start..], view.ui_origin(), -1.0);
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
    let text_in_scores_room =
        |quads: &mut Vec<DrawQuad>, label: &str, x: f32, y: f32, scale: f32, color| {
            text(
                quads,
                atlas,
                label,
                x - camera.x as f32,
                y - camera.y as f32,
                scale,
                color,
            );
        };
    if game.player.pos.y < 156.0 {
        text_in_scores_room(quads, "SECRET CHALLENGES", 144.0, 32.0, 1.0, yellow);
        for (label, key, y) in [
            ("SUN", "best_mini1", 48.0),
            ("MOON", "best_mini2", 64.0),
            ("STARS", "best_mini3", 80.0),
        ] {
            text_in_scores_room(
                quads,
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
        text_in_scores_room(
            quads,
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
                text_in_scores_room(quads, value, 128.0, 112.0 + line as f32 * 16.0, 1.0, WHITE);
            }
        } else {
            text_in_scores_room(quads, "LOCKED.", 128.0, 112.0, 1.0, WHITE);
        }
    } else {
        text_in_scores_room(quads, "TOP DEFILERS", 160.0, 32.0, 1.0, yellow);
        for (label, key, y) in [
            ("MONEY", "best_money", 48.0),
            ("KILLS", "best_kills", 64.0),
            ("SAVES", "best_saves", 80.0),
        ] {
            text_in_scores_room(
                quads,
                &format!("{:<8}{}", format!("{label}:"), hub_value(game, key) as u64),
                128.0,
                y,
                1.0,
                WHITE,
            );
        }
        let seconds = hub_value(game, "best_time") as u64;
        if hub_value(game, "total_wins") > 0.0 {
            text_in_scores_room(
                quads,
                &format!("TIME:   {}:{:02}", seconds / 60, seconds % 60),
                128.0,
                96.0,
                1.0,
                WHITE,
            );
        }
        text_in_scores_room(quads, "STATISTICS", 168.0, 112.0, 1.0, yellow);
        for (label, key, y) in [
            ("PLAYS", "total_plays", 128.0),
            ("DEATHS", "total_deaths", 144.0),
            ("WINS", "total_wins", 160.0),
        ] {
            text_in_scores_room(
                quads,
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
            text_in_scores_room(
                quads,
                message,
                ((320.0 - message.len() as f32 * 8.0) / 2.0).ceil(),
                216.0,
                1.0,
                yellow,
            );
        }
    }
}

#[cfg(test)]
fn hud(quads: &mut Vec<DrawQuad>, game: &Game, camera: Vec2, atlas: &Atlas) {
    hud_in_view(quads, game, camera, atlas, Viewport::canonical());
}

// Old saves can be drawn before their first migration Step. This is the same
// read-only, whitelist-only inference as the pre-dispatch migration; reserved
// Bomb/Rope and unsupported actor types never imply a remembered weapon.
fn hud_inventory_type(game: &Game) -> &str {
    if !game.player.pickup_item_type.is_empty() {
        return &game.player.pickup_item_type;
    }
    let Some(held) = game.player.held.and_then(|id| game.world.entity(id)) else {
        return "";
    };
    if !held.held || held.value("stashed") != 0.0 {
        return "";
    }
    match held.object.as_str() {
        "oRock" => "Rock",
        "oJar" => "Jar",
        "oSkull" => "Skull",
        "oFishBone" => "Fish Bone",
        "oArrow" => "Arrow",
        "oMachete" => "Machete",
        "oMattock" => "Mattock",
        "oMattockHead" => "Mattock Head",
        "oPistol" => "Pistol",
        "oWebCannon" => "Web Cannon",
        "oTeleporter" => "Teleporter",
        "oShotgun" => "Shotgun",
        "oBow" => "Bow",
        "oSceptre" => "Sceptre",
        "oFlare" => "Flare",
        "oKey" => "Key",
        _ => "",
    }
}

fn hud_in_view(
    quads: &mut Vec<DrawQuad>,
    game: &Game,
    camera: Vec2,
    atlas: &Atlas,
    view: Viewport,
) {
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
        hub_hud(quads, game, camera, atlas, view);
        return;
    }
    // scrDrawHUD is gated by global.drawHUD independently of the player's
    // death state. Ghost/ManTrap collisions clear this flag before the same
    // room Draw runs, one simulation tick before the host marks the run dead.
    if game
        .data
        .get("draw_hud")
        .or_else(|| game.data.get("drawHUD"))
        .is_some_and(|draw_hud| *draw_hud == 0.0)
    {
        return;
    }
    let player = &game.player;
    // Original Classic HUD positions and 16-pixel bitmap font.
    let status_start = quads.len();
    for (icon, value, x) in [
        ("sHeart", player.health.max(0.0).to_string(), 8.0),
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
    // scrDrawHUD's status row is expressed at view_xview + x,
    // view_yview + y. Keep it at the expanded view's real upper-left;
    // place_authored_panel centers only the original fixed-size panel.
    offset_panel_translation(&mut quads[status_start..], view.ui_origin(), -1.0);
    if player.has("Compass") {
        let [ox, oy] = view.ui_origin();
        // World exits are stored at the 16x16 door center. Classic's global
        // exitX/exitY and compass Draw use the door's top-left tile origin.
        let exit_x = (game.world.exit.x - 8.0 - camera.x) as f32 + ox;
        let exit_y = (game.world.exit.y - 8.0 - camera.y) as f32 + oy;
        let below_view = exit_y > view.height;
        let direction = if below_view {
            if exit_x < 0.0 {
                "LL"
            } else if exit_x > view.width - 16.0 {
                "LR"
            } else {
                "Down"
            }
        } else if exit_x < 0.0 {
            "Left"
        } else if exit_x > view.width - 16.0 {
            "Right"
        } else {
            ""
        };
        // scrDrawHUD uses smaller occupied artwork during source messages.
        // Both variants retain their original 16-pixel sprite geometry.
        let name = if direction.is_empty() {
            String::new()
        } else {
            format!(
                "sCompass{}{direction}",
                if game
                    .data
                    .get("source_message_begin_timer")
                    .or_else(|| game.data.get("message_timer"))
                    .copied()
                    .unwrap_or(0.0)
                    > 0.0
                {
                    "Small"
                } else {
                    ""
                }
            )
        };
        let start = quads.len();
        sprite_in_view(
            quads,
            atlas,
            &name,
            0.0,
            exit_x.clamp(0.0, view.width - 16.0),
            if below_view {
                view.height - 16.0
            } else {
                exit_y
            },
            false,
            WHITE,
            view,
        );
        // The final panel placement moves fixed HUD only; compass is already in full-view coordinates.
        for quad in &mut quads[start..] {
            quad.rect[0] -= ox;
            quad.rect[1] -= oy;
        }
    }
    let inventory_start = quads.len();
    sprite(quads, atlas, "sHoldItemIcon", 0.0, 8.0, 24.0, false, WHITE);
    // Canonical scrDrawHUD reads the remembered type, including while a
    // Bomb/Rope reservation occupies the hand. Actor frames/facing do not apply.
    let inventory_type = hud_inventory_type(game);
    let icon = match inventory_type {
        "Rock" => Some("sRock"),
        "Jar" => Some("sJar"),
        "Skull" => Some("sSkull"),
        "Fish Bone" => Some("sFishBone"),
        "Arrow" => Some("sArrowRight"),
        "Machete" => Some("sMacheteRight"),
        "Mattock" => Some("sMattockRight"),
        "Mattock Head" => Some("sMattockHead"),
        "Pistol" => Some("sPistolRight"),
        "Web Cannon" => Some("sWebCannonR"),
        "Teleporter" => Some("sTeleporter"),
        "Shotgun" => Some("sShotgunRight"),
        "Bow" => Some("sBowDisp"),
        "Sceptre" => Some("sSceptreRight"),
        "Flare" => Some("sFlare"),
        "Key" => Some("sKeyRight"),
        _ => None,
    };
    if let Some(name) = icon {
        sprite(quads, atlas, name, 0.0, 16.0, 32.0, false, WHITE);
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
            let frame = if name == "Kapala" {
                let blood = game.data.get("bloodLevel").copied().unwrap_or(0.0);
                if blood == 0.0 {
                    0.0
                } else if blood <= 2.0 {
                    1.0
                } else if blood <= 4.0 {
                    2.0
                } else if blood <= 6.0 {
                    3.0
                } else if blood <= 8.0 {
                    4.0
                } else {
                    // The source still advances the equipment slot if no
                    // Kapala frame matches (natural collection resets at 9).
                    x += 20.0;
                    continue;
                }
            } else {
                0.0
            };
            sprite(
                quads,
                atlas,
                &if name == "UdjatEye" && game.flags.contains("udjat_blink") {
                    "sUdjatEyeIcon2".into()
                } else {
                    format!("s{name}Icon")
                },
                frame,
                x,
                24.0,
                false,
                WHITE,
            );
            x += 20.0;
        }
    }
    // Source bow HUD displays ammunition, not the animated world bow's charge.
    if inventory_type == "Bow" {
        let mut arrows = game.data.get("arrows").copied().unwrap_or(0.0);
        while arrows > 0.0 {
            sprite(quads, atlas, "sArrowIcon", 0.0, x, 24.0, false, WHITE);
            x += 4.0;
            arrows -= 1.0;
        }
    }
    let collect = game.data.get("collect").copied().unwrap_or(0.0);
    if collect > 0.0 {
        text(
            quads,
            atlas,
            &format!("+{collect}"),
            176.0,
            24.0,
            1.0,
            [1.0, 1.0, 0.0, 1.0],
        );
    }
    offset_panel_translation(&mut quads[inventory_start..], view.ui_origin(), -1.0);
}

fn offset_panel_translation(quads: &mut [DrawQuad], origin: [f32; 2], direction: f32) {
    for quad in quads {
        quad.rect[0] += origin[0] * direction;
        quad.rect[1] += origin[1] * direction;
    }
}

#[cfg(test)]
mod tests {
    use crate::atlas::quad_matches_sprite_transition;

    fn opaque_sprite_quad_overlaps_rect(
        quad: &DrawQuad,
        atlas: &Atlas,
        obstacle: [f32; 4],
    ) -> bool {
        let width = quad.rect[2].round() as usize;
        let height = quad.rect[3].round() as usize;
        for y in 0..height {
            for x in 0..width {
                let u = quad.uv[0] + (x as f32 + 0.5) / width as f32 * quad.uv[2];
                let v = quad.uv[1] + (y as f32 + 0.5) / height as f32 * quad.uv[3];
                let atlas_x = (u * atlas.width as f32).floor() as usize;
                let atlas_y = (v * atlas.height as f32).floor() as usize;
                let offset = (atlas_y * atlas.width as usize + atlas_x) * 4 + 3;
                if atlas.rgba().get(offset).copied().unwrap_or(0) == 0 {
                    continue;
                }
                let left = quad.rect[0] + x as f32 * quad.rect[2] / width as f32;
                let top = quad.rect[1] + y as f32 * quad.rect[3] / height as f32;
                let right = quad.rect[0] + (x + 1) as f32 * quad.rect[2] / width as f32;
                let bottom = quad.rect[1] + (y + 1) as f32 * quad.rect[3] / height as f32;
                if left < obstacle[0] + obstacle[2]
                    && right > obstacle[0]
                    && top < obstacle[1] + obstacle[3]
                    && bottom > obstacle[1]
                {
                    return true;
                }
            }
        }
        false
    }

    fn presentation_animation_alpha(game: &Game, alpha: f64) -> f64 {
        if game.presentation().flags.contains("render_discontinuity") {
            0.0
        } else if alpha.is_finite() {
            alpha.clamp(0.0, 1.0)
        } else {
            1.0
        }
    }

    fn presentation_player_frame_matches(
        game: &Game,
        quad: &DrawQuad,
        atlas: &Atlas,
        alpha: f64,
    ) -> bool {
        let player = &game.presentation().player;
        quad_matches_sprite_transition(
            atlas,
            quad,
            &player.sprite,
            presentation_player_animation(
                player,
                &game.player.sprite,
                presentation_animation_alpha(game, alpha),
            ),
            player.facing > 0,
        )
    }

    fn presentation_entity_frame_matches(
        game: &Game,
        entity: &Entity,
        quad: &DrawQuad,
        atlas: &Atlas,
        alpha: f64,
        mirrored: bool,
    ) -> bool {
        quad_matches_sprite_transition(
            atlas,
            quad,
            &entity.sprite,
            presentation_entity_animation(
                entity,
                game.world
                    .entity(entity.id)
                    .or_else(|| {
                        game.dispatcher
                            .hidden_entities()
                            .iter()
                            .find(|current| current.id == entity.id && current.alive)
                    })
                    .map(|current| current.sprite.as_str()),
                presentation_animation_alpha(game, alpha),
            ),
            mirrored,
        )
    }

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
        let quads = game_scene(game, alpha, false, atlas);
        let quad = quads
            .iter()
            .find(|quad| presentation_player_frame_matches(game, quad, atlas, alpha))
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

    #[test]
    fn classic_gameplay_hud_has_no_unmapped_level_clock_overlay() {
        let source = include_str!("../../../references/classic-1.1/Scripts/scrDrawHUD.gml");
        assert!(source.contains("draw_text(moneyX, 8+16, \"+\" + string(global.collect))"));
        assert!(!source.contains("global.currLevel"));
        assert!(!source.contains("global.time"));

        let atlas = Atlas::load().unwrap();
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.tick = u64::from(spelunky_core::TICK_RATE) * 61;
        let mut quads = Vec::new();
        hud(&mut quads, &game, Vec2::default(), &atlas);
        assert!(
            quads.iter().all(|quad| quad.rect[1] != 230.0),
            "scrDrawHUD does not add a level/clock line at y=230"
        );
    }

    #[test]
    fn source_hud_kapala_uses_literal_fill_frames_and_preserves_equipment_slots() {
        let atlas = Atlas::load().unwrap();
        // scrDrawHUD119-123: all natural blood values plus the unmatched
        // value9 control. Source still advances n when no frame is drawn.
        for (blood, frame) in [
            (0.0, Some(0.0)),
            (1.0, Some(1.0)),
            (2.0, Some(1.0)),
            (3.0, Some(2.0)),
            (4.0, Some(2.0)),
            (5.0, Some(3.0)),
            (6.0, Some(3.0)),
            (7.0, Some(4.0)),
            (8.0, Some(4.0)),
            (9.0, None),
        ] {
            let mut game = Game::shell(1, RulesPreset::Classic);
            for equipment in ["UdjatEye", "Ankh", "Crown", "Kapala", "Spectacles"] {
                game.player.equipment.insert(equipment.into());
            }
            game.data.insert("bloodLevel".into(), blood);
            let before = game.state_hash();
            let random = game.rng.clone().next_u64();
            for view in [
                Viewport::canonical(),
                Viewport::from_surface(1280, 720),
                Viewport::from_surface(720, 960),
            ] {
                let mut quads = Vec::new();
                hud_in_view(&mut quads, &game, Vec2::new(0.0, 0.0), &atlas, view);
                place_authored_panel(&mut quads, view);
                let kapala_frames: Vec<_> = (0..5)
                    .map(|frame| atlas.sprite("sKapalaIcon", f64::from(frame)).unwrap().uv)
                    .collect();
                let actual: Vec<_> = quads
                    .iter()
                    .filter(|quad| kapala_frames.contains(&quad.uv))
                    .collect();
                if let Some(frame) = frame {
                    assert_eq!(actual.len(), 1, "blood={blood}");
                    assert_eq!(actual[0].uv, atlas.sprite("sKapalaIcon", frame).unwrap().uv);
                    assert_eq!(actual[0].rect, [88.0, 24.0, 16.0, 16.0]);
                    assert_eq!(actual[0].color, WHITE);
                    assert_eq!(actual[0].blend, QuadBlend::Alpha);
                } else {
                    assert!(actual.is_empty(), "source has no frame for blood9");
                }
                let spectacles = atlas.sprite("sSpectaclesIcon", 0.0).unwrap();
                let quad = quads.iter().find(|quad| quad.uv == spectacles.uv).unwrap();
                assert_eq!(quad.rect, [108.0, 24.0, 16.0, 16.0]);
                assert_eq!(game.state_hash(), before);
                assert_eq!(game.rng.clone().next_u64(), random);
            }
        }
    }

    #[test]
    fn expanded_challenge_counters_stay_at_the_view_origin_while_countdown_stays_centered() {
        let source =
            include_str!("../../../references/classic-1.1/Objects/oSunRoom.events/Draw.xml");
        assert!(source.contains("view_xview[0]+8, view_yview[0]+8"));
        assert!(source.contains("view_xview[0]+64, view_yview[0]+8"));

        let atlas = Atlas::load().unwrap();
        for view in [
            Viewport::from_surface(1920, 1080),
            Viewport::from_surface(600, 1200),
        ] {
            let mut game = Game::shell(1, RulesPreset::Classic);
            game.flags.extend(["hub".into(), "hub_sun".into()]);
            game.data.insert("hub_draw_status".into(), 0.0);
            game.data.insert("points".into(), 27.0);
            let [ox, oy] = view.ui_origin();
            let mut quads = Vec::new();
            hud_in_view(
                &mut quads,
                &game,
                Vec2::new(f64::from(ox), f64::from(oy)),
                &atlas,
                view,
            );
            place_authored_panel(&mut quads, view);

            let heart = atlas.sprite("sHeart", 0.0).unwrap();
            let heart = quads.iter().find(|quad| quad.uv == heart.uv).unwrap();
            assert_eq!(heart.rect[0..2], [8.0, 8.0]);
            let points_icon = atlas.sprite("sDamselIcon", 0.0).unwrap();
            let points_icon = quads.iter().find(|quad| quad.uv == points_icon.uv).unwrap();
            assert_eq!(points_icon.rect[0..2], [64.0, 8.0]);

            let countdown = "DAMSEL CHALLENGE BEGINS IN 3...";
            let countdown_x = ((320.0 - countdown.len() as f32 * 8.0) / 2.0).ceil();
            assert!(quads.iter().any(|quad| {
                quad.rect[0] == countdown_x + ox
                    && quad.rect[1] == 216.0 + oy
                    && quad.color == [1.0, 1.0, 0.0, 1.0]
            }));
        }
    }

    #[test]
    fn source_hud_saved_inventory_uses_literal_icons_independent_of_actor() {
        let atlas = Atlas::load().unwrap();
        let mut game = Game::shell(1, RulesPreset::Classic);
        let unrelated = game.spawn("oDamsel", Vec2::new(64.0, 64.0));
        game.player.held = Some(unrelated);
        let actor = game.world.entity_mut(unrelated).unwrap();
        actor.held = true;
        actor.facing = -1;
        actor.animation = 7.0;
        let cases = [
            ("Rock", "sRock"),
            ("Jar", "sJar"),
            ("Skull", "sSkull"),
            ("Fish Bone", "sFishBone"),
            ("Arrow", "sArrowRight"),
            ("Machete", "sMacheteRight"),
            ("Mattock", "sMattockRight"),
            ("Mattock Head", "sMattockHead"),
            ("Pistol", "sPistolRight"),
            ("Web Cannon", "sWebCannonR"),
            ("Teleporter", "sTeleporter"),
            ("Shotgun", "sShotgunRight"),
            ("Bow", "sBowDisp"),
            ("Sceptre", "sSceptreRight"),
            ("Flare", "sFlare"),
            ("Key", "sKeyRight"),
        ];
        for (saved_type, sprite_name) in cases {
            game.player.pickup_item_type = saved_type.into();
            for actor_present in [true, false] {
                game.player.held = actor_present.then_some(unrelated);
                let before = game.state_hash();
                let random = game.rng.clone().next_u64();
                let mut quads = Vec::new();
                hud(&mut quads, &game, Vec2::new(0.0, 0.0), &atlas);
                let background = atlas.sprite("sHoldItemIcon", 0.0).unwrap();
                let index = quads
                    .iter()
                    .position(|quad| quad.uv == background.uv)
                    .unwrap();
                let mut expected = Vec::new();
                sprite(
                    &mut expected,
                    &atlas,
                    sprite_name,
                    0.0,
                    16.0,
                    32.0,
                    false,
                    WHITE,
                );
                assert_eq!(
                    quad_bytes(&quads[index + 1..index + 2]),
                    quad_bytes(&expected),
                    "{saved_type}, actor={actor_present}"
                );
                assert_eq!(game.state_hash(), before);
                assert_eq!(game.rng.clone().next_u64(), random);
            }
        }
        game.player.held = Some(unrelated);
        game.player.pickup_item_type.clear();
        let mut empty = Vec::new();
        hud(&mut empty, &game, Vec2::new(0.0, 0.0), &atlas);
        for unsupported in ["Damsel", "Bomb", "Rope", "Dice", "oBow", "None"] {
            game.player.pickup_item_type = unsupported.into();
            let mut actual = Vec::new();
            hud(&mut actual, &game, Vec2::new(0.0, 0.0), &atlas);
            assert_eq!(
                quad_bytes(&actual),
                quad_bytes(&empty),
                "unsupported {unsupported}"
            );
        }
    }

    #[test]
    fn source_hud_natural_bow_cycle_keeps_saved_icon_and_ammunition() {
        let atlas = Atlas::load().unwrap();
        let mut game = adaptive_room();
        let old = game.spawn("oBow", game.player.pos);
        game.step(&spelunky_core::InputFrame {
            down: true,
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert_eq!(game.player.held, Some(old));
        game.step(&spelunky_core::InputFrame::default());
        game.step(&spelunky_core::InputFrame {
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        for _ in 0..4 {
            game.step(&spelunky_core::InputFrame {
                attack: true,
                ..Default::default()
            });
        }
        for object in ["oBomb", "oRopeThrow", "oBow"] {
            game.step(&spelunky_core::InputFrame {
                cycle_pressed: true,
                attack: true,
                ..Default::default()
            });
            assert_eq!(
                game.world.entity(game.player.held.unwrap()).unwrap().object,
                object
            );
            assert_eq!(game.player.pickup_item_type, "Bow");
            assert_eq!(game.data.get("arrows"), Some(&5.0));
            let before = game.state_hash();
            let random = game.rng.clone().next_u64();
            let mut quads = Vec::new();
            hud(&mut quads, &game, Vec2::new(0.0, 0.0), &atlas);
            let icon = atlas.sprite("sBowDisp", 0.0).unwrap();
            assert_eq!(quads.iter().filter(|quad| quad.uv == icon.uv).count(), 1);
            let ammo = atlas.sprite("sArrowIcon", 0.0).unwrap();
            assert_eq!(quads.iter().filter(|quad| quad.uv == ammo.uv).count(), 5);
            assert_eq!(game.state_hash(), before);
            assert_eq!(game.rng.clone().next_u64(), random);
        }
    }

    #[test]
    fn source_hud_legacy_held_bow_before_first_step_is_read_only() {
        let atlas = Atlas::load().unwrap();
        let mut original = adaptive_room();
        let id = original.spawn("oBow", original.player.pos);
        original.player.held = Some(id);
        original.world.entity_mut(id).unwrap().held = true;
        original.data.insert("arrows".into(), 3.0);
        let mut saved = serde_json::to_value(&original).unwrap();
        saved["player"]
            .as_object_mut()
            .unwrap()
            .remove("pickup_item_type");
        let game: Game = serde_json::from_value(saved).unwrap();
        assert!(game.player.pickup_item_type.is_empty());
        let before = game.state_hash();
        let random = game.rng.clone().next_u64();
        let mut quads = Vec::new();
        hud(&mut quads, &game, Vec2::new(0.0, 0.0), &atlas);
        let icon = atlas.sprite("sBowDisp", 0.0).unwrap();
        assert_eq!(quads.iter().filter(|quad| quad.uv == icon.uv).count(), 1);
        let ammo = atlas.sprite("sArrowIcon", 0.0).unwrap();
        assert_eq!(quads.iter().filter(|quad| quad.uv == ammo.uv).count(), 3);
        assert_eq!(game.state_hash(), before);
        assert_eq!(game.rng.clone().next_u64(), random);
    }

    #[test]
    fn source_hud_real_bow_charge_and_treasure_show_ammo_and_pending_money() {
        let atlas = Atlas::load().unwrap();
        let mut game = adaptive_room();
        for equipment in ["Crown", "Kapala"] {
            game.player.equipment.insert(equipment.into());
        }
        let bow = game.spawn("oBow", game.player.pos);
        game.player.held = Some(bow);
        game.world.entity_mut(bow).unwrap().held = true;
        game.data.insert("arrows".into(), 6.0);
        let gold = game.spawn("oGoldBar", game.player.pos);
        game.step(&spelunky_core::InputFrame {
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert!(
            !game
                .world
                .entities
                .iter()
                .any(|entity| entity.id == gold && entity.alive)
        );
        assert_eq!(game.data.get("collect"), Some(&500.0));
        assert_eq!(game.player.data.get("bow_armed"), Some(&1.0));
        for _ in 0..20 {
            game.step(&spelunky_core::InputFrame {
                attack: true,
                ..Default::default()
            });
        }
        assert!(game.world.entity(bow).unwrap().animation > 0.0);
        assert_eq!(game.data.get("collect"), Some(&500.0));
        let before = game.state_hash();
        let random = game.rng.clone().next_u64();
        for alpha in [0.0, 0.125, 0.5, 0.875, 1.0] {
            let quads = game_scene(&game, alpha, false, &atlas);
            let bow_display = atlas.sprite("sBowDisp", 0.0).unwrap();
            let slot = quads.iter().find(|quad| quad.uv == bow_display.uv).unwrap();
            assert_eq!(slot.rect, [8.0, 24.0, 16.0, 16.0]);
            let ammo = atlas.sprite("sArrowIcon", 0.0).unwrap();
            let arrows: Vec<_> = quads.iter().filter(|quad| quad.uv == ammo.uv).collect();
            assert_eq!(arrows.len(), 6);
            for (index, quad) in arrows.iter().enumerate() {
                assert_eq!(quad.rect, [68.0 + index as f32 * 4.0, 24.0, 16.0, 16.0]);
                assert_eq!(quad.color, WHITE);
                assert_eq!(quad.blend, QuadBlend::Alpha);
            }
            let mut expected_popup = Vec::new();
            text(
                &mut expected_popup,
                &atlas,
                "+500",
                176.0,
                24.0,
                1.0,
                [1.0, 1.0, 0.0, 1.0],
            );
            let popup: Vec<_> = quads
                .iter()
                .filter(|quad| quad.color == [1.0, 1.0, 0.0, 1.0] && quad.rect[1] == 24.0)
                .cloned()
                .collect();
            assert_eq!(quad_bytes(&popup), quad_bytes(&expected_popup));
            let final_arrow = quads.iter().rposition(|quad| quad.uv == ammo.uv).unwrap();
            let first_popup = quads
                .iter()
                .position(|quad| {
                    quad.uv == expected_popup[0].uv && quad.rect == expected_popup[0].rect
                })
                .unwrap();
            assert!(
                final_arrow < first_popup,
                "source draws pending money after ammunition"
            );
            assert_eq!(game.state_hash(), before);
            assert_eq!(game.rng.clone().next_u64(), random);
        }
        game.step(&spelunky_core::InputFrame::default());
        assert_eq!(game.data.get("arrows"), Some(&5.0));
        assert_eq!(game.data.get("collect"), Some(&400.0));
        let quads = game_scene(&game, 0.5, false, &atlas);
        let ammo = atlas.sprite("sArrowIcon", 0.0).unwrap();
        assert_eq!(quads.iter().filter(|quad| quad.uv == ammo.uv).count(), 5);
        let mut expected_popup = Vec::new();
        text(
            &mut expected_popup,
            &atlas,
            "+400",
            176.0,
            24.0,
            1.0,
            [1.0, 1.0, 0.0, 1.0],
        );
        let popup: Vec<_> = quads
            .iter()
            .filter(|quad| quad.color == [1.0, 1.0, 0.0, 1.0] && quad.rect[1] == 24.0)
            .cloned()
            .collect();
        assert_eq!(quad_bytes(&popup), quad_bytes(&expected_popup));
        for _ in 0..5 {
            game.step(&spelunky_core::InputFrame::default());
        }
        assert_eq!(game.data.get("collect"), Some(&0.0));
        assert_eq!(game.player.money, 500);
        let quads = game_scene(&game, 0.5, false, &atlas);
        assert!(
            !quads
                .iter()
                .any(|quad| quad.color == [1.0, 1.0, 0.0, 1.0] && quad.rect[1] == 24.0)
        );
        // Empty source remembered type suppresses ammunition even with stored arrows.
        game.player.held = None;
        game.player.pickup_item_type.clear();
        game.draw_view = None;
        let mut quads = Vec::new();
        hud(&mut quads, &game, Vec2::new(0.0, 0.0), &atlas);
        assert!(!quads.iter().any(|quad| quad.uv == ammo.uv));
    }

    #[test]
    fn source_hud_compass_uses_message_art_raw_side_y_and_source_draw_order() {
        let atlas = Atlas::load().unwrap();
        let all_arrows: Vec<_> = ["", "Small"]
            .into_iter()
            .flat_map(|size| {
                ["LL", "LR", "Down", "Left", "Right"].map(|direction| {
                    atlas
                        .sprite(&format!("sCompass{size}{direction}"), 0.0)
                        .unwrap()
                        .uv
                })
            })
            .collect();
        for view in [
            Viewport::canonical(),
            Viewport::from_surface(1280, 720),
            Viewport::from_surface(720, 960),
        ] {
            let right = view.width - 16.0;
            for (exit, expected) in [
                (
                    Vec2::new(-1.0, f64::from(view.height + 1.0)),
                    Some(("LL", 0.0, view.height - 16.0)),
                ),
                (
                    Vec2::new(f64::from(right + 1.0), f64::from(view.height + 1.0)),
                    Some(("LR", right, view.height - 16.0)),
                ),
                (
                    Vec2::new(100.0, f64::from(view.height + 1.0)),
                    Some(("Down", 100.0, view.height - 16.0)),
                ),
                (
                    Vec2::new(f64::from(right), f64::from(view.height + 1.0)),
                    Some(("Down", right, view.height - 16.0)),
                ),
                (Vec2::new(-1.0, 24.0), Some(("Left", 0.0, 24.0))),
                (Vec2::new(-1.0, -8.0), Some(("Left", 0.0, -8.0))),
                (
                    Vec2::new(f64::from(right + 1.0), f64::from(view.height - 8.0)),
                    Some(("Right", right, view.height - 8.0)),
                ),
                (Vec2::new(100.0, f64::from(view.height)), None),
                (Vec2::new(0.0, 100.0), None),
                (Vec2::new(f64::from(right), 100.0), None),
            ] {
                for message in [0.0, 1.0] {
                    let mut game = Game::shell(1, RulesPreset::Classic);
                    game.player.equipment.insert("Compass".into());
                    game.data.insert("message_timer".into(), message);
                    // Fixtures above are expressed in Classic's source tile
                    // origin; the world model stores the door center.
                    game.world.exit = Vec2::new(exit.x + 8.0, exit.y + 8.0);
                    let [ox, oy] = view.ui_origin();
                    // hud_in_view receives expanded camera plus uiOrigin;
                    // fixture rendered camera is independently fixed at0,0.
                    let mut quads = Vec::new();
                    hud_in_view(
                        &mut quads,
                        &game,
                        Vec2::new(f64::from(ox), f64::from(oy)),
                        &atlas,
                        view,
                    );
                    place_authored_panel(&mut quads, view);
                    let actual: Vec<_> = quads
                        .iter()
                        .filter(|quad| all_arrows.contains(&quad.uv))
                        .collect();
                    if let Some((direction, x, y)) = expected {
                        assert_eq!(actual.len(), 1, "exit={exit:?} message={message}");
                        let name = format!(
                            "sCompass{}{direction}",
                            if message > 0.0 { "Small" } else { "" }
                        );
                        assert_eq!(actual[0].uv, atlas.sprite(&name, 0.0).unwrap().uv);
                        assert!((actual[0].rect[0] - x).abs() < 0.0001);
                        assert!((actual[0].rect[1] - y).abs() < 0.0001);
                        assert_eq!(actual[0].rect[2..], [16.0, 16.0]);
                        assert_eq!(actual[0].color, WHITE);
                        assert_eq!(actual[0].blend, QuadBlend::Alpha);
                        let compass_index = quads
                            .iter()
                            .position(|quad| quad.uv == actual[0].uv)
                            .unwrap();
                        let held = atlas.sprite("sHoldItemIcon", 0.0).unwrap();
                        let held_index = quads.iter().position(|quad| quad.uv == held.uv).unwrap();
                        assert!(
                            compass_index < held_index,
                            "source compass precedes held/equipment HUD"
                        );
                    } else {
                        assert!(
                            actual.is_empty(),
                            "strict source visibility boundary exit={exit:?}"
                        );
                    }
                    assert_eq!(game.data.get("message_timer"), Some(&message));
                }
            }
        }
    }

    // Use real synchronous Blood Create, its five-tick collectible alarm,
    // movement and Player collision. Never assign red_color, red_toggle,
    // collectible, particle velocity/age, or invoke the collision directly.
    fn natural_kapala_flash(preset: RulesPreset) -> Game {
        let mut game = adaptive_room();
        game.rules = preset;
        game.player.equipment.insert("Kapala".into());
        for _ in 0..16 {
            game.spawn("oBlood", game.player.pos);
        }
        for _ in 0..120 {
            game.step(&spelunky_core::InputFrame::default());
            if game
                .presentation()
                .player
                .data
                .get("red_color")
                .copied()
                .unwrap_or(0.0)
                > 0.0
            {
                assert!(
                    game.data.get("bloodLevel").copied().unwrap_or(0.0) > 0.0
                        || game.player.health > 4.0
                );
                assert!(
                    game.world
                        .entities
                        .iter()
                        .any(|e| e.object == "oBloodSpark")
                );
                return game;
            }
        }
        panic!("natural collectible Blood did not reach the Kapala player: {preset:?}");
    }

    fn assert_source_player_color(game: &Game, atlas: &Atlas, color: [f32; 4], bypass_tint: bool) {
        let live = game.state_hash();
        let draw = game.presentation().state_hash();
        let random = game.rng.clone().next_u64();
        let previous_red = game.presentation().render_history.player_red_color;
        let previous = if bypass_tint || previous_red <= 0.0 {
            WHITE
        } else {
            [(200.0 + previous_red) as f32 / 255.0, 0.0, 0.0, 1.0]
        };
        for view in [
            Viewport::canonical(),
            Viewport::from_surface(1920, 1080),
            Viewport::from_surface(720, 960),
        ] {
            for alpha in [0.0, 0.125, 0.5, 0.875, 1.0] {
                let quads = game_scene_in_view(game, alpha, false, atlas, view);
                let player: Vec<_> = quads
                    .iter()
                    .filter(|quad| presentation_player_frame_matches(game, quad, atlas, alpha))
                    .collect();
                assert_eq!(
                    player.len(),
                    1,
                    "frozen character frame missing or duplicated"
                );
                let expected = if bypass_tint {
                    WHITE
                } else {
                    std::array::from_fn(|channel| {
                        previous[channel] + (color[channel] - previous[channel]) * alpha as f32
                    })
                };
                assert_eq!(
                    player[0].color, expected,
                    "interpolated source redColor at alpha={alpha}"
                );
            }
        }
        assert_eq!(game.state_hash(), live);
        assert_eq!(game.presentation().state_hash(), draw);
        assert_eq!(game.rng.clone().next_u64(), random);
    }

    #[test]
    fn source_kapala_tint_interpolates_between_draw_samples_without_advancing_state() {
        let atlas = Atlas::load().unwrap();
        let game = natural_kapala_flash(RulesPreset::Classic);
        let draw = game.presentation();
        let previous_red = draw.render_history.player_red_color;
        let current_red = draw.player.data["red_color"];
        assert_eq!(previous_red, 0.0);
        assert!(current_red > previous_red);
        let before = game.state_hash();
        let draw_before = draw.state_hash();
        let random = game.rng.clone().next_u64();
        let mut colors = Vec::new();

        for alpha in [0.0, 0.25, 0.5, 0.75, 1.0] {
            let quads = game_scene(&game, alpha, false, &atlas);
            let player = quads
                .iter()
                .find(|quad| presentation_player_frame_matches(&game, quad, &atlas, alpha))
                .expect("player tint frame missing");
            colors.push(player.color);
        }

        assert_eq!(colors[0], WHITE, "previous Draw sample is untinted");
        assert_eq!(
            colors[4],
            [(200.0 + current_red) as f32 / 255.0, 0.0, 0.0, 1.0],
            "current Draw sample retains the source color"
        );
        assert!(colors.windows(2).all(|pair| pair[0][1] > pair[1][1]));
        assert_ne!(colors[2], colors[0]);
        assert_ne!(colors[2], colors[4]);
        assert_eq!(game.state_hash(), before);
        assert_eq!(game.presentation().state_hash(), draw_before);
        assert_eq!(game.rng.clone().next_u64(), random);
    }

    #[test]
    fn dying_web_alpha_interpolates_between_draw_samples_without_advancing_state() {
        let atlas = Atlas::load().unwrap();
        let mut game = adaptive_room();
        let web_id = game.spawn("oWeb", Vec2::new(200.0, 160.0));
        {
            let web = game.world.entity_mut(web_id).unwrap();
            web.data.insert("life".into(), 6.0);
            web.data.insert("dying".into(), 1.0);
        }
        game.step(&spelunky_core::InputFrame::default());
        game.step(&spelunky_core::InputFrame::default());

        let draw = game.presentation();
        let web = draw.world.entity(web_id).expect("web must remain alive");
        let previous = draw.render_history.web_alpha[&web_id];
        let current = web.data["image_alpha"];
        assert!((previous - 0.5).abs() < 1.0e-9);
        assert!(current < previous, "source web alpha must be fading");
        let state = game.state_hash();
        let draw_state = draw.state_hash();
        let random = game.rng.clone().next_u64();
        let mut opacities = Vec::new();

        for alpha in [0.0, 0.25, 0.5, 0.75, 1.0] {
            let quads = game_scene(&game, alpha, false, &atlas);
            let web_quad = quads
                .iter()
                .find(|quad| {
                    presentation_entity_frame_matches(&game, web, quad, &atlas, alpha, false)
                })
                .expect("web sprite missing from scene");
            let expected = (previous + (current - previous) * alpha) as f32;
            assert!((web_quad.color[3] - expected).abs() < 1.0e-6);
            opacities.push(web_quad.color[3]);
        }

        assert!(opacities.windows(2).all(|pair| pair[0] > pair[1]));
        assert_ne!(opacities[2], opacities[0]);
        assert_ne!(opacities[2], opacities[4]);
        assert_eq!(game.state_hash(), state);
        assert_eq!(game.presentation().state_hash(), draw_state);
        assert_eq!(game.rng.clone().next_u64(), random);
    }

    #[test]
    fn source_kapala_real_blood_collection_tints_frozen_character_and_naturally_fades() {
        let atlas = Atlas::load().unwrap();
        for preset in [RulesPreset::Classic, RulesPreset::Assisted] {
            let neutral = adaptive_room();
            assert_source_player_color(&neutral, &atlas, WHITE, false);
            let mut game = natural_kapala_flash(preset);
            let red = game.presentation().player.data["red_color"];
            assert!(red > 0.0 && red <= 55.0 && red % 5.0 == 0.0);
            // Independent integer-source oracle; no production color helper.
            assert_source_player_color(
                &game,
                &atlas,
                [(200.0 + red) as f32 / 255.0, 0.0, 0.0, 1.0],
                false,
            );
            let mut faded = false;
            for _ in 0..240 {
                game.step(&spelunky_core::InputFrame::default());
                if game.presentation().player.data.get("red_color") == Some(&0.0) {
                    faded = true;
                    break;
                }
            }
            assert!(faded, "ordinary source pulse never finished: {preset:?}");
            assert_source_player_color(&game, &atlas, WHITE, false);

            let mut without_kapala = adaptive_room();
            without_kapala.rules = preset;
            for _ in 0..16 {
                without_kapala.spawn("oBlood", without_kapala.player.pos);
            }
            for _ in 0..40 {
                without_kapala.step(&spelunky_core::InputFrame::default());
            }
            assert_eq!(
                without_kapala
                    .data
                    .get("bloodLevel")
                    .copied()
                    .unwrap_or(0.0),
                0.0
            );
            assert_eq!(
                without_kapala
                    .presentation()
                    .player
                    .data
                    .get("red_color")
                    .copied()
                    .unwrap_or(0.0),
                0.0
            );
            assert_source_player_color(&without_kapala, &atlas, WHITE, false);
        }
    }

    #[test]
    fn source_kapala_flash_jetpack_exception_whipping_decorations_and_blink_draw_controls() {
        let atlas = Atlas::load().unwrap();
        // Synthetic Draw-branch controls derive their red value from an actual
        // acquired snapshot. They make no claim to reach Climbing/Exit via input.
        let acquired = natural_kapala_flash(RulesPreset::Classic);
        for (state, sprite_name) in [
            (PlayerState::Standing, "sStandLeft"),
            (PlayerState::Climbing, "sClimbUp"),
            (PlayerState::Standing, "sPExit"),
            (PlayerState::Standing, "sDamselExit"),
            (PlayerState::Standing, "sTunnelExit"),
        ] {
            for jetpack in [false, true] {
                for whipping in [false, true] {
                    for facing in [-1, 1] {
                        let mut draw = acquired.presentation().clone();
                        draw.player.state = state;
                        draw.player.set_sprite(sprite_name);
                        draw.player.facing = facing;
                        draw.player.attack_timer = if whipping { 12 } else { 0 };
                        if jetpack {
                            draw.player.equipment.insert("Jetpack".into());
                        }
                        draw.player.data.insert("bow_armed".into(), 1.0);
                        let exception = jetpack
                            && !whipping
                            && (state == PlayerState::Climbing || sprite_name.ends_with("Exit"));
                        let red = draw.player.data["red_color"];
                        assert_source_player_color(
                            &draw,
                            &atlas,
                            if exception {
                                WHITE
                            } else {
                                [(200.0 + red) as f32 / 255.0, 0.0, 0.0, 1.0]
                            },
                            exception,
                        );
                        let quads = game_scene(&draw, 0.5, false, &atlas);
                        let back_uv = atlas.sprite("sJetpackBack", 0.0).unwrap().uv;
                        assert_eq!(quads.iter().any(|q| q.uv == back_uv), exception);
                        let side = if facing > 0 {
                            "sJetpackRight"
                        } else {
                            "sJetpackLeft"
                        };
                        let side_uv = atlas.sprite(side, 0.0).unwrap().uv;
                        assert_eq!(quads.iter().any(|q| q.uv == side_uv), jetpack && !exception);
                        let arrow = if facing > 0 {
                            "sArrowRight"
                        } else {
                            "sArrowLeft"
                        };
                        for name in [side, "sJetpackBack", arrow] {
                            let uv = atlas.sprite(name, 0.0).unwrap().uv;
                            assert!(
                                quads
                                    .iter()
                                    .filter(|q| q.uv == uv)
                                    .all(|q| q.color == WHITE),
                                "decoration tinted: {name}"
                            );
                        }
                        draw.player.data.insert("blink_toggle".into(), 1.0);
                        let hidden = game_scene(&draw, 0.5, false, &atlas);
                        assert!(
                            !hidden.iter().any(|quad| {
                                presentation_player_frame_matches(&draw, quad, &atlas, 0.5)
                            }),
                            "blink must hide source player layer"
                        );
                        for uv in [back_uv, side_uv, atlas.sprite(arrow, 0.0).unwrap().uv] {
                            assert!(
                                !hidden.iter().any(|q| q.uv == uv),
                                "blink must hide source player layer"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn source_back_jetpack_draw_precedes_held_arrow_while_climbing() {
        let atlas = Atlas::load().unwrap();
        let mut draw = Game::shell(1, RulesPreset::Classic);
        draw.player.state = PlayerState::Climbing;
        draw.player.set_sprite("sClimbUp");
        draw.player.facing = -1;
        draw.player.equipment.insert("Jetpack".into());
        draw.player.data.insert("hold_arrow".into(), 1.0);

        let quads = game_scene(&draw, 1.0, false, &atlas);
        let back = atlas.sprite("sJetpackBack", 0.0).unwrap().uv;
        let arrow = atlas.sprite("sArrowLeft", 0.0).unwrap().uv;
        let back_index = quads.iter().position(|quad| quad.uv == back).unwrap();
        let arrow_index = quads.iter().position(|quad| quad.uv == arrow).unwrap();

        assert!(
            back_index < arrow_index,
            "characterDrawEvent draws the back jetpack before its held arrow"
        );
    }

    fn adaptive_room() -> Game {
        let mut game = Game::shell(5, RulesPreset::Classic);
        game.world = spelunky_core::World::new(80, 40, Area::Mines, 1);
        game.player.pos = Vec2::new(40.0, 152.0);
        game.player.previous_pos = game.player.pos;
        game.player.state = PlayerState::Standing;
        game.player.grounded = true;
        for x in 0..80 {
            game.world.set_tile(x, 10, "oBrick");
        }
        game.camera = Vec2::new(0.0, 0.0);
        game.previous_camera = game.camera;
        game
    }

    #[test]
    fn high_refresh_keeps_source_sprite_frames_discrete_and_interpolates_motion() {
        let atlas = Atlas::load().unwrap();
        let mut game = adaptive_room();
        game.player.previous_pos = Vec2::new(40.0, 152.0);
        game.player.pos = Vec2::new(48.0, 152.0);
        game.player.set_sprite("sRunLeft");
        game.player.facing = -1;
        game.player.animation = 0.25;
        game.player.set_image_speed(1.0);
        let state_hash = game.state_hash();
        let draw_hash = game.presentation().state_hash();
        let frame_zero = atlas.sprite("sRunLeft", 0.0).unwrap().uv;
        let animation = game.player.animation;
        let mut x_positions = Vec::new();

        // 240 Hz samples each 30 Hz interval eight times. The image index is
        // source-discrete while world motion is interpolated for each sample.
        let tick_boundary = game_scene(&game, 0.0, false, &atlas);
        let tick_actor = tick_boundary
            .iter()
            .find(|quad| quad.uv == frame_zero)
            .expect("source player subimage missing");
        assert_eq!(tick_actor.next_uv, frame_zero);
        assert_eq!(tick_actor.frame_mix, 0.0);
        for sample in 0..8 {
            let alpha = f64::from(sample) / 8.0;
            let quads = game_scene(&game, alpha, false, &atlas);
            let actor = quads
                .iter()
                .find(|quad| quad.uv == frame_zero)
                .expect("interpolated player sprite missing");
            assert_eq!(actor.next_uv, frame_zero);
            assert_eq!(actor.frame_mix, 0.0);
            assert_eq!(
                presentation_player_animation(&game.player, "sRunLeft", alpha),
                animation
            );
            x_positions.push(actor.rect[0]);
        }
        assert_eq!(x_positions.len(), 8);
        assert!(x_positions.windows(2).all(|pair| pair[1] > pair[0]));
        assert_eq!(game.state_hash(), state_hash);
        assert_eq!(game.presentation().state_hash(), draw_hash);
    }

    fn quad_bytes(
        quads: &[DrawQuad],
    ) -> Vec<(
        [u32; 4],
        [u32; 4],
        [u32; 4],
        u32,
        [u32; 4],
        u32,
        [u32; 2],
        QuadBlend,
    )> {
        quads
            .iter()
            .map(|q| {
                (
                    q.rect.map(f32::to_bits),
                    q.uv.map(f32::to_bits),
                    q.next_uv.map(f32::to_bits),
                    q.frame_mix.to_bits(),
                    q.color.map(f32::to_bits),
                    q.rotation.to_bits(),
                    q.pivot.map(f32::to_bits),
                    q.blend,
                )
            })
            .collect()
    }

    #[test]
    fn culled_actor_does_not_rotate_a_previous_layer_and_buffers_survive_room_changes() {
        let atlas = Atlas::load().unwrap();
        let mut game = adaptive_room();
        let rock = game.world.spawn("oRock", Vec2::new(80.0, 80.0));
        game.world
            .entity_mut(rock)
            .unwrap()
            .data
            .insert("rotation".into(), 0.25);
        let mut buffers = SceneBuffers::default();
        game_scene_in_view_into(
            &game,
            0.5,
            false,
            &atlas,
            Viewport::canonical(),
            &mut buffers,
        );
        let before = quad_bytes(&buffers.quads);
        let arrow = game.world.spawn("oArrow", Vec2::new(10000.0, 10000.0));
        game.world
            .entity_mut(arrow)
            .unwrap()
            .data
            .insert("rotation".into(), 1.5);
        game_scene_in_view_into(
            &game,
            0.5,
            false,
            &atlas,
            Viewport::canonical(),
            &mut buffers,
        );
        assert_eq!(
            quad_bytes(&buffers.quads),
            before,
            "culled actor changed an earlier sprite"
        );
        for (w, h) in [(960, 540), (540, 960), (320, 240)] {
            let view = Viewport::from_surface(w, h);
            // Same tick: metadata, world and viewport can change without stepping.
            let metadata = game.world.spawn("oTerrainMetadata", Vec2::new(8.0, 168.0));
            game.world
                .entity_mut(metadata)
                .unwrap()
                .text
                .insert("terrain_sprite".into(), "sIce".into());
            game_scene_in_view_into(&game, 0.375, true, &atlas, view, &mut buffers);
            let fresh = game_scene_in_view(&game, 0.375, true, &atlas, view);
            assert_eq!(quad_bytes(&buffers.quads), quad_bytes(&fresh));
            game = adaptive_room();
        }
        // Reuse scratch storage for a wholly different, empty room.
        game.world = spelunky_core::World::new(8, 8, Area::Ice, 2);
        game_scene_in_view_into(
            &game,
            1.0,
            false,
            &atlas,
            Viewport::canonical(),
            &mut buffers,
        );
        assert_eq!(
            quad_bytes(&buffers.quads),
            quad_bytes(&game_scene(&game, 1.0, false, &atlas))
        );
    }

    #[test]
    fn adaptive_view_reveals_horizontal_and_vertical_world_markers_without_stretching() {
        let atlas = Atlas::load().unwrap();
        let mut game = adaptive_room();
        game.world.spawn("oRuby", Vec2::new(368.0, 64.0));
        game.world.spawn("oDiamond", Vec2::new(520.0, 64.0));
        game.world.spawn("oEmerald", Vec2::new(80.0, 304.0));
        let ruby = atlas.sprite("sRuby", 0.0).unwrap();
        let diamond = atlas.sprite("sDiamond", 0.0).unwrap();
        let emerald = atlas.sprite("sEmerald", 0.0).unwrap();
        let state = game.state_hash();
        let rng = game.rng.clone().next_u64();
        let canonical = game_scene(&game, 0.5, false, &atlas);
        assert!(
            !canonical
                .iter()
                .any(|q| q.uv == ruby.uv || q.uv == diamond.uv || q.uv == emerald.uv)
        );
        let wide = game_scene_in_view(
            &game,
            0.5,
            false,
            &atlas,
            Viewport::from_surface(1920, 1080),
        );
        let ultrawide = game_scene_in_view(
            &game,
            0.5,
            false,
            &atlas,
            Viewport::from_surface(3440, 1440),
        );
        let tall = game_scene_in_view(&game, 0.5, false, &atlas, Viewport::from_surface(800, 800));
        let q = wide
            .iter()
            .find(|q| q.uv == ruby.uv)
            .expect("horizontal strip missing");
        assert_eq!(&q.rect[2..], &ruby.size);
        assert!(!wide.iter().any(|q| q.uv == diamond.uv));
        assert!(ultrawide.iter().any(|q| q.uv == ruby.uv));
        let q = ultrawide
            .iter()
            .find(|q| q.uv == diamond.uv)
            .expect("ultrawide horizontal strip missing");
        assert!(q.rect[0] > 426.0 && q.rect[0] < 573.0);
        assert_eq!(&q.rect[2..], &diamond.size);
        assert!(tall.iter().any(|q| q.uv == emerald.uv));
        assert_eq!(game.state_hash(), state);
        assert_eq!(game.rng.clone().next_u64(), rng);
        assert_eq!(
            quad_bytes(&canonical),
            quad_bytes(&game_scene_in_view(
                &game,
                0.5,
                false,
                &atlas,
                Viewport::from_surface(960, 720)
            ))
        );
    }

    #[test]
    fn adaptive_view_borrows_frozen_draw_actors_and_tiles_and_visible_storage_wins_duplicates() {
        let atlas = Atlas::load().unwrap();
        let mut game = adaptive_room();
        let id = game.world.spawn("oBomb", Vec2::new(600.0, 152.0));
        for _ in 0..8 {
            game.step(&spelunky_core::InputFrame::default());
        }
        let view = Viewport::from_surface(2400, 720);
        let draw = game.presentation();
        let hidden = draw
            .dispatcher
            .hidden_entities()
            .iter()
            .find(|e| e.id == id)
            .expect("fixture not frozen");
        let frame = atlas.sprite(&hidden.sprite, hidden.animation).unwrap();
        let index = 10 * game.world.width + 37;
        let camera = view.camera(
            interpolated(draw.previous_camera, draw.camera, 0.5),
            draw.world.width as f64 * 16.0,
            draw.world.height as f64 * 16.0,
        );
        let floor_x = 592.0 - camera.x as f32;
        let floor_y = 160.0 - camera.y as f32;
        assert!(draw.world.entity(id).is_none());
        assert!(draw.world.tiles[index].object.is_empty());
        let hash = game.state_hash();
        let draw_hash = draw.state_hash();
        let rng = game.rng.clone().next_u64();
        for alpha in [0., 0.5, 1.] {
            let quads = game_scene_in_view(&game, alpha, false, &atlas, view);
            let bomb = quads
                .iter()
                .find(|q| q.uv == frame.uv)
                .expect("frozen outer actor absent");
            assert_eq!(bomb.rect[0], (hidden.pos.x as f32) - frame.origin[0]);
            assert!(
                quads
                    .iter()
                    .any(|q| q.rect[0] == floor_x && q.rect[1] == floor_y),
                "frozen outer floor absent"
            );
        }
        let mut stale = hidden.clone();
        stale.previous_pos.x -= 20.0;
        for alpha in [0., 0.5, 1.] {
            assert_eq!(
                presented_entity_position(draw, &stale, alpha, true),
                stale.pos
            );
        }
        assert_eq!(game.state_hash(), hash);
        assert_eq!(game.presentation().state_hash(), draw_hash);
        assert_eq!(game.rng.clone().next_u64(), rng);
        // Deliberately malformed duplicate controls: visible storage is authoritative.
        let mut replacement = hidden.clone();
        let draw = game.draw_view.as_mut().unwrap();
        replacement.pos.x = 560.0;
        replacement.previous_pos = replacement.pos;
        replacement.engine_active = true;
        replacement.sprite = "sRuby".into();
        replacement.animation = 0.0;
        draw.world.entities.push(replacement);
        draw.world.tiles[index] = spelunky_core::Tile::new("oLush");
        let quads = game_scene_in_view(&game, 0.5, false, &atlas, view);
        assert!(
            !quads.iter().any(|q| q.uv == frame.uv),
            "hidden duplicate resurrected"
        );
        let at_index: Vec<_> = quads
            .iter()
            .filter(|q| q.rect[0] == floor_x && q.rect[1] == floor_y)
            .collect();
        assert_eq!(at_index.len(), 1, "duplicate terrain drawn twice");
        assert_eq!(
            at_index[0].uv,
            atlas
                .sprite(catalog::sprite("oLush").unwrap(), 0.0)
                .unwrap()
                .uv
        );
    }

    #[test]
    fn expanded_paused_scene_keeps_a_borrowed_flare_light_without_advancing_game() {
        let atlas = Atlas::load().unwrap();
        let mut game = adaptive_room();
        game.world.dark = true;
        let id = game.world.spawn("oFlare", Vec2::new(440.0, 80.0));
        let lava_index = 8 * game.world.width + 33;
        game.world.set_tile(33, 8, "oLava");
        game.step(&spelunky_core::InputFrame::default());

        let draw = game.presentation();
        let frozen = draw
            .dispatcher
            .hidden_entities()
            .iter()
            .find(|entity| entity.id == id)
            .expect("offscreen flare should be frozen before pause");
        assert!(!frozen.engine_active);
        let hidden_lava = draw
            .dispatcher
            .hidden_tiles()
            .iter()
            .find(|(index, _)| *index == lava_index)
            .expect("offscreen lava should be frozen before pause");
        assert!(!hidden_lava.1.engine_active);
        let frame = atlas.sprite(&frozen.sprite, frozen.animation).unwrap();
        let view = Viewport::from_surface(2560, 1080);
        let camera = view.camera(
            interpolated(draw.previous_camera, draw.camera, 1.0),
            draw.world.width as f64 * 16.0,
            draw.world.height as f64 * 16.0,
        );
        let game_hash = game.state_hash();
        let draw_hash = draw.state_hash();
        let quads = game_scene_in_view(&game, 1.0, false, &atlas, view);
        assert!(
            quads.iter().any(|quad| quad.uv == frame.uv),
            "expanded paused view should borrow the flare sprite"
        );
        let lights = scene_lights_in_view(&game, 1.0, Some((camera, view)));
        assert!(
            lights
                .iter()
                .any(|light| light.position == Vec2::new(536.0, 136.0) && light.radius == 32.0),
            "expanded paused view should borrow the frozen lava light"
        );

        let mut darkness = Vec::new();
        darkness_in_view(&mut darkness, &game, camera, 1.0, view);
        let lit_point = Vec2::new(552.0, 136.0);
        let local_x = (lit_point.x - camera.x).floor() as f32;
        let local_y = (lit_point.y - camera.y).floor() as f32;
        assert!(
            darkness.iter().any(|quad| {
                quad.blend == QuadBlend::Multiply
                    && quad.rect[0] <= local_x
                    && local_x < quad.rect[0] + quad.rect[2]
                    && quad.rect[1] == local_y
                    && quad.color[..3] != [0.0, 0.0, 0.0]
            }),
            "borrowed sources should illuminate the widened viewport"
        );
        assert_eq!(
            game.state_hash(),
            game_hash,
            "pause rendering cannot step simulation"
        );
        assert_eq!(
            draw.state_hash(),
            draw_hash,
            "pause rendering cannot mutate Draw state"
        );
        assert!(
            !frozen.engine_active,
            "expanded rendering cannot reactivate the flare"
        );
    }

    #[test]
    fn adaptive_darkness_fades_background_and_hud_cover_the_expanded_view() {
        let atlas = Atlas::load().unwrap();
        let mut game = adaptive_room();
        game.world.dark = true;
        for view in [
            Viewport::from_surface(1920, 1080),
            Viewport::from_surface(800, 800),
            Viewport::from_surface(600, 1200),
        ] {
            let quads = game_scene_in_view(&game, 0.5, false, &atlas, view);
            let mask: Vec<_> = quads
                .iter()
                .filter(|q| q.blend == QuadBlend::Multiply)
                .collect();
            assert_eq!(mask.first().unwrap().rect[0], 0.0);
            assert!(mask.iter().any(
                |q| q.rect[0] + q.rect[2] >= view.width && q.rect[1] + q.rect[3] >= view.height
            ));
            for y in 0..view.height.ceil() as u32 {
                let width: f32 = mask
                    .iter()
                    .filter(|q| q.rect[1] == y as f32)
                    .map(|q| q.rect[2])
                    .sum();
                assert_eq!(width, view.width.ceil(), "uncovered darkness row{y}");
            }
            let heart = atlas.sprite("sHeart", 0.0).unwrap();
            let q = quads.iter().find(|q| q.uv == heart.uv).unwrap();
            assert_eq!(q.rect[0], 8.0 - heart.origin[0]);
            assert_eq!(q.rect[1], 8.0 - heart.origin[1]);
            let mut panel = Vec::new();
            cinematic_fade(&mut panel, 0.5);
            place_authored_panel(&mut panel, view);
            assert_eq!(panel[0].rect, [0.0, 0.0, view.width, view.height]);
        }
        game.world.name = "rEnd3".into();
        assert_eq!(
            quad_bytes(&game_scene(&game, 0.5, false, &atlas)),
            quad_bytes(&game_scene_in_view(
                &game,
                0.5,
                false,
                &atlas,
                Viewport::from_surface(960, 720)
            ))
        );
    }

    #[test]
    fn adaptive_volcano_background_fills_outer_strips_without_changing_authored_panel() {
        let atlas = Atlas::load().unwrap();
        let mut game = adaptive_room();
        game.world.name = "rEnd2".into();
        let view = Viewport::from_surface(1920, 1080);
        let mut quads = Vec::new();
        background_in_view(
            &mut quads,
            &game,
            Vec2::new(-53.333333, 0.0),
            false,
            &atlas,
            view,
        );
        assert!(
            quads
                .iter()
                .any(|q| q.rect[0] <= 0.0 && q.rect[0] + q.rect[2] > 0.0)
        );
        assert!(
            quads
                .iter()
                .any(|q| q.rect[0] < view.width && q.rect[0] + q.rect[2] >= view.width)
        );
        quads.clear();
        background_in_view(
            &mut quads,
            &game,
            Vec2::default(),
            false,
            &atlas,
            Viewport::from_surface(960, 720),
        );
        assert!(quads.is_empty(), "canonical room background changed");
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    #[ignore = "requires a native or software GPU; run explicitly for renderer acceptance"]
    fn adaptive_gpu_darkness_covers_full_wide_and_tall_surface_and_preserves_hud() {
        let mut game = adaptive_room();
        game.data.insert("darkness".into(), 0.5);
        for view in [
            Viewport::from_surface(1920, 1080),
            Viewport::from_surface(800, 800),
            Viewport::from_surface(600, 1200),
        ] {
            let mut quads = Vec::new();
            rect(
                &mut quads,
                [0., 0., view.width, view.height],
                [0.8, 0.4, 0.2, 1.],
            );
            darkness_in_view(&mut quads, &game, Vec2::default(), 1.0, view);
            rect(
                &mut quads,
                [view.width - 24., view.height - 24., 16., 16.],
                [1., 0.5, 0., 1.],
            );
            for srgb in [false, true] {
                let pixels = crate::renderer::tests::gpu_pixels_in_view(&quads, srgb, view);
                for (x, y, expected) in [
                    (
                        view.surface_width - 1,
                        view.surface_height - 1,
                        [0, 0, 0, 255],
                    ),
                    (
                        ((f64::from(view.width) - 16.0) * view.scale).floor() as u32,
                        ((f64::from(view.height) - 16.0) * view.scale).floor() as u32,
                        [255, 128, 0, 255],
                    ),
                    (
                        (40.0 * view.scale).floor() as u32,
                        (152.0 * view.scale).floor() as u32,
                        [102, 51, 51, 255],
                    ),
                ] {
                    let offset = ((y * view.surface_width + x) * 4) as usize;
                    let actual = &pixels[offset..offset + 4];
                    assert!(
                        actual
                            .iter()
                            .zip(expected)
                            .all(|(a, b)| (i32::from(*a) - b).abs() <= 1),
                        "view={view:?} srgb={srgb} pixel({x},{y})={actual:?} expected={expected:?}"
                    );
                }
                // The HUD square uses the real projection, not an independent viewport formula.
                let orange: Vec<_> = pixels
                    .chunks_exact(4)
                    .enumerate()
                    .filter_map(|(i, p)| {
                        (p[0] >= 254 && (127..=129).contains(&p[1]) && p[2] <= 1).then_some((
                            i as u32 % view.surface_width,
                            i as u32 / view.surface_width,
                        ))
                    })
                    .collect();
                let min_x = orange.iter().map(|p| p.0).min().unwrap();
                let max_x = orange.iter().map(|p| p.0).max().unwrap();
                let min_y = orange.iter().map(|p| p.1).min().unwrap();
                let max_y = orange.iter().map(|p| p.1).max().unwrap();
                let width = max_x - min_x + 1;
                let height = max_y - min_y + 1;
                assert!((f64::from(width) - 16.0 * view.scale).abs() <= 1.0);
                assert!((f64::from(height) - 16.0 * view.scale).abs() <= 1.0);
                assert!(
                    width.abs_diff(height) <= 1,
                    "square pixels stretched on GPU"
                );
            }
        }
    }

    #[test]
    fn adaptive_compass_uses_expanded_world_visibility_and_window_edges() {
        let atlas = Atlas::load().unwrap();
        let mut game = adaptive_room();
        game.player.equipment.insert("Compass".into());
        let view = Viewport::from_surface(1920, 1080);
        let arrows: Vec<_> = [
            "sCompassLL",
            "sCompassLR",
            "sCompassDown",
            "sCompassLeft",
            "sCompassRight",
        ]
        .iter()
        .map(|name| atlas.sprite(name, 0.0).unwrap().uv)
        .collect();
        game.world.exit = Vec2::new(368.0, 160.0);
        let quads = game_scene_in_view(&game, 0.5, false, &atlas, view);
        assert!(
            !quads.iter().any(|q| arrows.contains(&q.uv)),
            "visible expanded exit incorrectly got arrow"
        );
        game.world.exit = Vec2::new(808.0, 408.0);
        let quads = game_scene_in_view(&game, 0.5, false, &atlas, view);
        let frame = atlas.sprite("sCompassLR", 0.0).unwrap();
        let q = quads.iter().find(|q| q.uv == frame.uv).unwrap();
        assert!((q.rect[0] - (view.width - 16.0 - frame.origin[0])).abs() < 0.0001);
        assert_eq!(q.rect[1], view.height - 16.0 - frame.origin[1]);
    }

    #[test]
    fn adaptive_views_do_not_change_actual_tick_trace_or_random_stream() {
        let atlas = Atlas::load().unwrap();
        let mut game = adaptive_room();
        let mut reference = game.clone();
        for tick in 0..12 {
            let input = spelunky_core::InputFrame {
                right: tick < 6,
                ..Default::default()
            };
            game.step(&input);
            reference.step(&input);
            for view in [
                Viewport::from_surface(3440, 1440),
                Viewport::from_surface(600, 1200),
                Viewport::canonical(),
            ] {
                let _ = game_scene_in_view(&game, 0.375, false, &atlas, view);
            }
            assert_eq!(game.state_hash(), reference.state_hash());
            assert_eq!(
                game.rng.clone().next_u64(),
                reference.rng.clone().next_u64()
            );
            assert_eq!(
                serde_json::to_string(&game.events).unwrap(),
                serde_json::to_string(&reference.events).unwrap()
            );
            assert_eq!(
                game.presentation().state_hash(),
                reference.presentation().state_hash()
            );
        }
    }

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
                        .find(|quad| {
                            quad.rect == expected
                                && presentation_entity_frame_matches(
                                    &game, entity, quad, &atlas, alpha, false,
                                )
                        })
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
    fn actual_frog_whip_death_burst_is_created_and_drawn() {
        use spelunky_core::InputFrame;

        let atlas = Atlas::load().unwrap();
        let mut game = Game::shell(42, RulesPreset::Classic);
        game.world = spelunky_core::World::new(20, 16, Area::Jungle, 5);
        game.world.name = "rLevel".into();
        for x in 0..20 {
            game.world.set_tile(x, 10, "oBlock");
        }
        game.player.pos = Vec2::new(80.0, 152.0);
        game.player.previous_pos = game.player.pos;
        game.player.facing = 1;
        let frog = game.spawn("oFrog", Vec2::new(104.0, 144.0));
        game.world.entity_mut(frog).unwrap().timer = 1000;

        game.step(&InputFrame {
            right: true,
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        for _ in 0..30 {
            if game.world.entity(frog).is_none() {
                break;
            }
            game.step(&InputFrame {
                right: true,
                ..Default::default()
            });
        }
        assert!(
            game.world.entity(frog).is_none(),
            "real input kills the frog"
        );

        // The Frog Step's death hook creates three drops after the ordinary
        // whip impact drop. Advance once so all four have a Draw snapshot.
        game.step(&InputFrame::default());
        let draw = game.presentation();
        let blood: Vec<_> = draw
            .world
            .entities
            .iter()
            .filter(|entity| entity.alive && entity.engine_active && entity.object == "oBlood")
            .collect();
        assert_eq!(blood.len(), 4, "one whip impact plus three death drops");

        let quads = game_scene(&game, 1.0, false, &atlas);
        for entity in blood {
            let frame = atlas.sprite("sBlood", entity.animation).unwrap();
            let position = entity_position(draw, entity, 1.0);
            let camera = draw.camera;
            let expected = [
                (position.x - camera.x) as f32 - frame.origin[0],
                (position.y - camera.y) as f32 - frame.origin[1],
                frame.size[0],
                frame.size[1],
            ];
            let quad = quads
                .iter()
                .find(|quad| {
                    quad.rect == expected
                        && presentation_entity_frame_matches(
                            &game, entity, quad, &atlas, 1.0, false,
                        )
                })
                .expect("each real frog death drop has a scene quad");
            assert_eq!(quad.color, WHITE);
            assert_eq!(quad.blend, QuadBlend::Alpha);
        }
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
            let hash = game.state_hash();
            let draw_hash = game.presentation().state_hash();
            for alpha in [0.0, 0.5, 1.0] {
                let quads = game_scene(&game, alpha, false, &atlas);
                assert!(quads.iter().any(|quad| {
                    presentation_entity_frame_matches(&game, draw, quad, &atlas, alpha, false)
                }));
                if tick == 15 {
                    let last_frame = atlas.sprite("sPoof", 5.0).unwrap();
                    let poof = quads
                        .iter()
                        .find(|quad| quad.uv == last_frame.uv)
                        .expect("the Animation End Draw keeps the source's last poof frame");
                    assert_eq!(poof.next_uv, last_frame.uv);
                    assert_eq!(poof.frame_mix, 0.0);
                }
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
                .all(|quad| quad.uv != last_frame.uv && quad.next_uv != last_frame.uv)
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
            let hash = game.state_hash();
            let draw_hash = game.presentation().state_hash();
            for alpha in [0.0, 0.5, 1.0] {
                let position = entity_position(game.presentation(), draw, alpha);
                let quads = game_scene(&game, alpha, false, &atlas);
                let quad = quads
                    .iter()
                    .find(|quad| {
                        presentation_entity_frame_matches(&game, draw, quad, &atlas, alpha, true)
                    })
                    .unwrap();
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
        // The room starts airborne. Source Player Step1219 gives an empty-hand
        // attack priority over pickup until the player can duck on the floor.
        for _ in 0..20 {
            if game.player.grounded {
                break;
            }
            game.step(&spelunky_core::InputFrame::default());
        }
        assert!(game.player.grounded);
        game.step(&spelunky_core::InputFrame {
            down: true,
            ..Default::default()
        });
        assert_eq!(game.player.state, PlayerState::Ducking);
        let weapon = game.spawn(
            "oWebCannon",
            Vec2::new(game.player.pos.x, game.player.pos.y + 2.0),
        );
        game.step(&spelunky_core::InputFrame {
            down: true,
            attack: true,
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
                let hash = game.state_hash();
                let draw_hash = game.presentation().state_hash();
                for alpha in [0.0, 0.5, 1.0] {
                    assert!(game_scene(&game, alpha, false, &atlas).iter().any(|quad| {
                        presentation_entity_frame_matches(&game, draw, quad, &atlas, alpha, false)
                    }));
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
        assert!(game_scene(&game, 1.0, false, &atlas).iter().any(|quad| {
            presentation_entity_frame_matches(&game, draw, quad, &atlas, 1.0, false)
        }));
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
        hub_hud(
            &mut quads,
            &game,
            game.camera,
            &atlas,
            Viewport::canonical(),
        );
        assert_eq!(quads[0].rect, [39.0, 224.0, 8.0, 8.0]);
        assert_eq!(quads.len(), "WELCOME TO THE SHORTCUT HOUSE!".len());
        for (x, camera) in [(256.0, 1.0), (255.0, 1.01)] {
            game.player.pos.x = x;
            game.camera.x = camera;
            quads.clear();
            hub_hud(
                &mut quads,
                &game,
                game.camera,
                &atlas,
                Viewport::canonical(),
            );
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
        hub_hud(
            &mut quads,
            &game,
            game.camera,
            &atlas,
            Viewport::canonical(),
        );
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
        hub_hud(
            &mut quads,
            &game,
            game.camera,
            &atlas,
            Viewport::canonical(),
        );
        assert!(!quads.iter().any(|quad| quad.rect[1] == 216.0));
        game.world.entity_mut(block).unwrap().pos.x = 176.0;
        game.world
            .entity_mut(button)
            .unwrap()
            .data
            .insert("pushed".into(), 1.0);
        quads.clear();
        hub_hud(
            &mut quads,
            &game,
            game.camera,
            &atlas,
            Viewport::canonical(),
        );
        assert!(!quads.iter().any(|quad| quad.rect[1] == 216.0));
    }

    #[test]
    fn scores_room_text_moves_with_camera_shake() {
        let atlas = Atlas::load().unwrap();
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.flags
            .extend(["hub".into(), "hub_scores".into(), "unlock_sun".into()]);
        game.player.pos.y = 180.0;
        game.data.insert("total_wins".into(), 1.0);
        let button = game
            .world
            .spawn("oButtonHighscore", Vec2::new(224.0, 192.0));
        game.world.spawn("oPushBlock", Vec2::new(176.0, 192.0));
        game.camera = Vec2::new(2.0, 4.0);

        let mut quads = Vec::new();
        hub_hud(
            &mut quads,
            &game,
            game.camera,
            &atlas,
            Viewport::canonical(),
        );
        assert!(
            quads
                .iter()
                .any(|quad| quad.rect == [158.0, 28.0, 8.0, 8.0])
        );
        assert!(
            quads
                .iter()
                .any(|quad| quad.rect == [126.0, 44.0, 8.0, 8.0])
        );
        let warning = "THIS WILL CLEAR EVERYTHING!";
        assert_eq!(
            quads.iter().filter(|quad| quad.rect[1] == 212.0).count(),
            warning.len(),
            "the clear warning follows the room camera too"
        );

        game.player.pos.y = 48.0;
        game.camera.y = 3.0;
        quads.clear();
        hub_hud(
            &mut quads,
            &game,
            game.camera,
            &atlas,
            Viewport::canonical(),
        );
        assert!(quads.iter().any(|quad| quad.rect[1] == 29.0));
        assert!(quads.iter().any(|quad| quad.rect[1] == 93.0));
        assert!(quads.iter().any(|quad| quad.rect[1] == 109.0));

        // Retain the live clear-button reference so the test exercises the
        // warning's world-space condition, not an unrelated missing button.
        assert!(game.world.entity(button).is_some());
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
        hub_hud(
            &mut quads,
            &game,
            game.camera,
            &atlas,
            Viewport::canonical(),
        );
        assert_eq!(
            quads.iter().filter(|quad| quad.rect[1] == 112.0).count(),
            "KEEP YOURSELF AND".len()
        );
        // oButtonHighscore zeroes the oHighscores t* copies on its first
        // pressed Step. Persistent highscore values clear later at counter1.
        game.flags.insert("hub_records_erased".into());
        quads.clear();
        hub_hud(
            &mut quads,
            &game,
            game.camera,
            &atlas,
            Viewport::canonical(),
        );
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
    fn cinematic_dummy_draw_preserves_jetpack_body_and_held_item_order() {
        use spelunky_core::ending::{EndingKind, EndingStage, EndingState};

        fn draw_game(sprite: &str, facing: i8, jetpack: bool, held_item: &str) -> Game {
            let mut game = Game::shell(1, RulesPreset::Classic);
            game.world.name = "rEnd".into();
            game.world.dark = true;
            game.camera = Vec2::default();
            game.previous_camera = Vec2::default();
            game.player.pos = Vec2::new(250.0, 100.0);
            game.player.previous_pos = game.player.pos;
            game.player.sprite = "sStandLeft".into();
            if jetpack {
                game.player.equipment.insert("Jetpack".into());
            }
            let id = game.world.spawn("oPDummy", Vec2::new(80.0, 80.0));
            let actor = game.world.entity_mut(id).unwrap();
            actor.previous_pos = actor.pos;
            actor.sprite = sprite.into();
            actor.animation = 0.0;
            actor.facing = facing;
            actor.text.insert("pickup_item".into(), held_item.into());
            game.ending = Some(EndingState::new(
                &game,
                EndingKind::Campaign,
                EndingStage::Chamber,
            ));
            game.state = spelunky_core::RunState::Ending;
            game
        }

        let atlas = Atlas::load().unwrap();
        let run = atlas.sprite("sRunLeft", 0.0).unwrap();
        let jetpack_side = atlas.sprite("sJetpackRight", 0.0).unwrap();
        let shotgun = atlas.sprite("sShotgunRight", 0.0).unwrap();
        let gameplay = draw_game("sRunLeft", 1, true, "Shotgun");
        let quads = game_scene(&gameplay, 0.0, false, &atlas);
        let mirrored_run_uv = [run.uv[0] + run.uv[2], run.uv[1], -run.uv[2], run.uv[3]];
        let side_index = quads
            .iter()
            .position(|quad| quad.uv == jetpack_side.uv)
            .expect("nonexit dummy draws its side jetpack");
        let body_index = quads
            .iter()
            .position(|quad| quad.uv == mirrored_run_uv)
            .expect("RIGHT-facing dummy mirrors its authored left-facing run sprite");
        let shotgun_index = quads
            .iter()
            .position(|quad| quad.uv == shotgun.uv)
            .expect("source Shotgun held-item branch draws the right-facing shotgun");
        assert!(
            side_index < body_index,
            "nonexit jetpack draws behind the body"
        );
        assert!(body_index < shotgun_index, "held item draws over the body");
        let side = &quads[side_index];
        assert_eq!(
            side.rect,
            [
                80.0 - 4.0 - jetpack_side.origin[0],
                80.0 - 1.0 - jetpack_side.origin[1],
                jetpack_side.size[0],
                jetpack_side.size[1],
            ],
            "side jetpack keeps the source x-4,y-1 placement"
        );
        let held = &quads[shotgun_index];
        assert_eq!(
            held.rect,
            [
                80.0 + 4.0 - shotgun.origin[0],
                80.0 + 2.0 - shotgun.origin[1],
                shotgun.size[0],
                shotgun.size[1],
            ],
            "held Shotgun keeps the source x+4,y+2 placement"
        );

        let exit = draw_game("sPExit", -1, true, "Shotgun");
        let quads = game_scene(&exit, 0.0, false, &atlas);
        let exit_body = atlas.sprite("sPExit", 0.0).unwrap();
        let jetpack_back = atlas.sprite("sJetpackBack", 0.0).unwrap();
        let body_index = quads
            .iter()
            .position(|quad| quad.uv == exit_body.uv)
            .expect("exit dummy draws its source exit sprite");
        let back_index = quads
            .iter()
            .position(|quad| quad.uv == jetpack_back.uv)
            .expect("exit dummy draws the back jetpack");
        assert!(
            body_index < back_index,
            "exit back jetpack draws over the body"
        );
        assert!(
            !quads.iter().any(|quad| quad.uv == shotgun.uv),
            "source exit branch omits the held item"
        );
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
    fn natural_whip_while_climbing_uses_the_cape_facing_pose() {
        let atlas = Atlas::load().unwrap();
        let mut game = Game::shell(7, RulesPreset::Classic);
        game.world = spelunky_core::World::new(30, 20, Area::Mines, 1);
        game.world.name = "rLevel".into();
        for column in 0..30 {
            game.world.set_tile(column, 10, "oHardBlock");
        }
        for row in 7..10 {
            game.world.set_tile(6, row, "oLadderOrange");
        }
        game.player.pos = Vec2::new(104.0, 152.0);
        game.player.previous_pos = game.player.pos;
        game.world.entrance = game.player.pos;
        game.player.equipment.insert("Cape".into());

        for _ in 0..4 {
            game.step(&spelunky_core::InputFrame::default());
        }
        assert!(game.player.grounded, "authored floor must settle Player");
        game.step(&spelunky_core::InputFrame {
            up: true,
            ..Default::default()
        });
        assert!(spelunky_core::player::source_climbing(&game));
        game.step(&spelunky_core::InputFrame {
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert!(spelunky_core::player::source_climbing(&game));
        assert!(game.player.attack_timer > 0, "ordinary attack starts whip");
        assert!(game.player.has("Cape"));
        let cape = game
            .world
            .entities
            .iter()
            .find(|entity| entity.alive && entity.object == "oCape")
            .expect("Player Step creates a separate Cape actor");
        assert_eq!(
            cape.sprite, "sCapeDR",
            "whipping makes the source Cape Step use the facing pose"
        );
        assert!(
            !game
                .world
                .entities
                .iter()
                .any(|entity| entity.alive && entity.object == "oCape" && entity.id != cape.id)
        );

        let quads = game_scene(&game, 1.0, false, &atlas);
        let back = atlas.sprite("sCapeBack", game.player.animation).unwrap().uv;
        let facing_pose = atlas.sprite("sCapeDR", game.player.animation).unwrap().uv;
        assert!(!quads.iter().any(|quad| quad.uv == back));
        assert!(quads.iter().any(|quad| quad.uv == facing_pose));
    }

    #[test]
    fn natural_cape_pickup_does_not_draw_the_cape_before_player_creates_its_actor() {
        let atlas = Atlas::load().unwrap();
        let mut level = spelunky_core::level::Level::default();
        level.set_tile(7, 4, '@').unwrap();
        for x in 7..=8 {
            level.set_tile(x, 5, '1').unwrap();
        }
        for x in 0..40 {
            level.set_tile(x, 24, '1').unwrap();
        }
        let mut game = Game::shell(42, RulesPreset::Classic);
        game.load_custom(&level, "natural-cape-draw-phase", "");
        let pickup = game.spawn("oCapePickup", Vec2::new(136.0, 90.0));
        for _ in 0..4 {
            game.step(&spelunky_core::InputFrame::default());
        }
        assert!(game.player.grounded);
        assert_eq!(game.world.entity(pickup).unwrap().value("cost"), 0.0);

        game.step(&spelunky_core::InputFrame {
            down: true,
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert!(game.player.has("Cape"));
        assert!(game.world.entity(pickup).is_none());
        assert!(
            !game
                .world
                .entities
                .iter()
                .any(|entity| entity.alive && entity.object == "oCape"),
            "oPlayer1 checks for oCape before its later same-Step scrStealItem pickup"
        );

        let cape_poses = [
            "sCapeBack",
            "sCapeUR",
            "sCapeUL",
            "sCapeRight",
            "sCapeLeft",
            "sCapeDR",
            "sCapeDL",
        ];
        let drawn = game_scene(&game, 1.0, false, &atlas);
        for pose in cape_poses {
            let uv = atlas.sprite(pose, 0.0).unwrap().uv;
            assert!(
                !drawn.iter().any(|quad| quad.uv == uv),
                "source has no Cape actor on the pickup frame, but renderer drew {pose}"
            );
        }

        game.step(&spelunky_core::InputFrame::default());
        assert!(
            game.world
                .entities
                .iter()
                .any(|entity| entity.alive && entity.object == "oCape")
        );
        let drawn = game_scene(&game, 1.0, false, &atlas);
        let actor_pose = game
            .world
            .entities
            .iter()
            .find(|entity| entity.alive && entity.object == "oCape")
            .unwrap()
            .sprite
            .clone();
        let uv = atlas.sprite(&actor_pose, 0.0).unwrap().uv;
        assert!(drawn.iter().any(|quad| quad.uv == uv));
    }

    #[test]
    fn ice_room_background_keeps_the_authored_abyss_fade_rows() {
        let atlas = Atlas::load().unwrap();
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.world.area = spelunky_core::Area::Ice;
        game.world.name = "Ice Caves".into();
        let view = Viewport {
            width: 672.0,
            height: 608.0,
            scale: 1.0,
            surface_width: 672,
            surface_height: 608,
        };
        let mut quads = Vec::new();
        background_in_view(&mut quads, &game, Vec2::default(), false, &atlas, view);

        for (sprite_name, world_y) in [("sBlack", 592.0), ("sBlackFadeUp", 576.0)] {
            let frame = atlas.sprite(sprite_name, 0.0).unwrap();
            let row: Vec<_> = quads
                .iter()
                .filter(|quad| quad.uv == frame.uv)
                .map(|quad| quad.rect)
                .collect();
            assert_eq!(row.len(), 42, "source {sprite_name} row width");
            for (x, rect) in row.iter().enumerate() {
                assert_eq!(
                    *rect,
                    [x as f32 * 16.0, world_y, frame.size[0], frame.size[1]],
                    "source {sprite_name} instance {x}"
                );
            }
        }
        let black_first = quads
            .iter()
            .position(|quad| quad.uv == atlas.sprite("sBlack", 0.0).unwrap().uv)
            .unwrap();
        let fade_first = quads
            .iter()
            .position(|quad| quad.uv == atlas.sprite("sBlackFadeUp", 0.0).unwrap().uv)
            .unwrap();
        assert!(
            black_first < fade_first,
            "source depth draws oBlackBG first"
        );

        game.flags.insert("custom".into());
        quads.clear();
        background_in_view(&mut quads, &game, Vec2::default(), false, &atlas, view);
        let black = atlas.sprite("sBlack", 0.0).unwrap().uv;
        let fade = atlas.sprite("sBlackFadeUp", 0.0).unwrap().uv;
        assert!(!quads.iter().any(|quad| quad.uv == black || quad.uv == fade));
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
    fn reachable_transition_and_custom_ending_keep_authored_black_panel_composition() {
        use spelunky_core::ending::{EndingKind, EndingStage, EndingState};

        let atlas = Atlas::load().unwrap();
        let black = atlas.sprite("sBlack", 0.0).unwrap();
        let transition_rooms = [
            "rTransition1",
            "rTransition1x",
            "rTransition2",
            "rTransition2x",
            "rTransition3",
            "rTransition3x",
            "rTransition4",
        ];

        // Each authored transition room places the 16x5 backing grid at
        // x=32..272, y=48..112. rTransition4 alone changes the background.
        for room in transition_rooms {
            let mut game = Game::shell(1, RulesPreset::Classic);
            let mut ending =
                EndingState::new(&game, EndingKind::Transition, EndingStage::Transition);
            ending.completed_level = 1;
            game.ending = Some(ending);
            spelunky_core::ending::load_room(&mut game, room);

            let quads = game_scene(&game, 1.0, false, &atlas);
            let black_indices: Vec<_> = quads
                .iter()
                .enumerate()
                .filter_map(|(index, quad)| (quad.uv == black.uv).then_some(index))
                .collect();
            let expected_positions: Vec<_> = (48..=112)
                .step_by(16)
                .flat_map(|y| (32..=272).step_by(16).map(move |x| (x as f32, y as f32)))
                .collect();
            let mut actual_positions: Vec<_> = black_indices
                .iter()
                .map(|&index| {
                    let rect = quads[index].rect;
                    assert_eq!([rect[2], rect[3]], black.size, "{room} black tile size");
                    (rect[0], rect[1])
                })
                .collect();
            actual_positions.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
            let mut expected_positions = expected_positions;
            expected_positions.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
            assert_eq!(actual_positions, expected_positions, "{room} oBlack layout");

            let background_name = if room == "rTransition4" {
                "bgTemple"
            } else {
                "bgCave"
            };
            let background_uv = atlas.sprite(background_name, 0.0).unwrap().uv;
            let background_index = quads
                .iter()
                .position(|quad| quad.uv == background_uv)
                .unwrap_or_else(|| panic!("{room} must draw its authored {background_name}"));
            assert!(
                background_index < black_indices[0],
                "{room} background under oBlack"
            );

            let heading = atlas
                .sprite("sFontSmall", f64::from('L' as u32 - 32))
                .unwrap();
            let heading_index = quads
                .iter()
                .position(|quad| quad.uv == heading.uv && quad.rect[..2] == [32.0, 48.0])
                .unwrap_or_else(|| panic!("{room} transition heading must draw over oBlack"));
            assert!(
                black_indices.last().unwrap() < &heading_index,
                "{room} text layering"
            );
        }

        // Classic's custom completion route enters rEndCustom directly. Its
        // 12x6 oBlack grid is covered by the controller's first summary line.
        let mut game = Game::shell(1, RulesPreset::Classic);
        spelunky_core::ending::begin_custom(&mut game);
        assert_eq!(game.world.name, "rEndCustom");
        assert_eq!(game.ending.as_ref().unwrap().stage, EndingStage::Summary);
        // oEndCustom Create hides all oMenu descendants; Alarm 0 reveals them
        // alongside the first summary line after fifty source ticks. Advancing
        // that callback keeps the fixture's draw state and panel visibility in
        // sync, which direct draw_status assignment would skip.
        assert_eq!(game.ending.as_ref().unwrap().draw_status, 0);
        let hidden_quads = game_scene(&game, 1.0, false, &atlas);
        assert!(!hidden_quads.iter().any(|quad| quad.uv == black.uv));
        for _ in 0..50 {
            game.step(&spelunky_core::InputFrame::default());
        }
        assert_eq!(game.ending.as_ref().unwrap().draw_status, 1);

        let quads = game_scene(&game, 1.0, false, &atlas);
        let mut actual_positions: Vec<_> = quads
            .iter()
            .filter(|quad| quad.uv == black.uv)
            .map(|quad| {
                assert_eq!(
                    [quad.rect[2], quad.rect[3]],
                    black.size,
                    "rEndCustom black tile size"
                );
                (quad.rect[0], quad.rect[1])
            })
            .collect();
        let mut expected_positions: Vec<_> = (32..=112)
            .step_by(16)
            .flat_map(|y| (64..=240).step_by(16).map(move |x| (x as f32, y as f32)))
            .collect();
        actual_positions.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
        expected_positions.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
        assert_eq!(
            actual_positions, expected_positions,
            "rEndCustom oBlack layout"
        );

        let summary_heading = atlas.sprite("sFont", f64::from('Y' as u32 - 32)).unwrap();
        let heading_index = quads
            .iter()
            .position(|quad| quad.uv == summary_heading.uv && quad.rect[..2] == [64.0, 32.0])
            .expect("rEndCustom draws YOU MADE IT over the authored black grid");
        let black_last = quads.iter().rposition(|quad| quad.uv == black.uv).unwrap();
        assert!(
            black_last < heading_index,
            "rEndCustom summary text layering"
        );
    }

    #[test]
    fn reachable_authored_intro_and_ending_props_reach_scene_quads() {
        let atlas = Atlas::load().unwrap();
        let mut game = Game::shell(1, RulesPreset::Classic);
        spelunky_core::opening::begin_intro(&mut game);

        // App::new starts in the authored rIntro. Its source background
        // instances span a 960-pixel room, so check both camera regions.
        for (camera_x, sprites) in [
            (0.0, ["sPalmTreeDark", "sShrubDark"]),
            (640.0, ["sCaveBG", "sCaveBGEntrance"]),
        ] {
            game.camera = Vec2::new(camera_x, 0.0);
            game.previous_camera = game.camera;
            let quads = game_scene(&game, 1.0, false, &atlas);
            for name in sprites {
                let uv = atlas.sprite(name, 0.0).unwrap().uv;
                assert!(
                    quads.iter().any(|quad| quad.uv == uv),
                    "rIntro's {name} source instance must reach the scene renderer at camera x={camera_x}"
                );
            }
        }

        // These are static visible objects in the authored cinematic room
        // tables. Their object defaults supply the same sprites as Classic.
        game.opening = None;
        for (room, sprites) in [
            ("rEnd", &["sEndWall", "sBigChest"][..]),
            ("rEnd2", &["sEnd2BG"][..]),
            ("rEnd3", &["sPalmTree", "sShrub", "sBlack"][..]),
        ] {
            spelunky_core::ending::load_room(&mut game, room);
            // rEnd is a 640-pixel room; both its end wall and chest are in
            // the second canonical viewport. The following authored rooms
            // are 320 pixels wide and restart at their origin.
            game.camera = Vec2::new(if room == "rEnd" { 320.0 } else { 0.0 }, 0.0);
            game.previous_camera = game.camera;
            let quads = game_scene(&game, 1.0, false, &atlas);
            for name in sprites {
                let uv = atlas.sprite(name, 0.0).unwrap().uv;
                assert!(
                    quads.iter().any(|quad| quad.uv == uv),
                    "{room}'s authored {name} instance must reach the scene renderer"
                );
            }
            if room == "rEnd2" {
                let uv = atlas.sprite("sEnd2BG", 0.0).unwrap().uv;
                assert!(
                    quads
                        .iter()
                        .any(|quad| quad.uv == uv && quad.rect == [0.0, 0.0, 320.0, 240.0]),
                    "canonical rEnd2 uses the authored object for its full-room background"
                );
            }
        }
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
    fn darkness_circle_edge_matches_default_source_polygon() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.player.pos = Vec2::new(100.0, 100.0);
        game.player.previous_pos = game.player.pos;
        game.data.insert("darkness".into(), 0.0);

        let mut quads = Vec::new();
        darkness(&mut quads, &game, Vec2::default(), 1.0);

        let color_at = |x: usize, y: usize| {
            quads
                .iter()
                .find(|quad| {
                    quad.rect[1] == y as f32
                        && quad.rect[0] <= x as f32
                        && (quad.rect[0] + quad.rect[2]) > x as f32
                })
                .expect("darkness mask must cover every pixel")
                .color
        };

        // The classic oScreen Begin Step draws this filled radius-96 circle
        // without setting precision. The default 24-gon excludes (194, 112),
        // although that pixel center still lies inside an analytic circle.
        assert_eq!(color_at(194, 110), [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(color_at(194, 112), [0.0, 0.0, 0.0, 1.0]);
        assert!(Vec2::new(194.5, 112.5).distance(game.player.pos) < 96.0);
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
        game.player.health = 20.0;
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
        game.player.health = 1.0;
        assert!(
            !game_scene(&game, 1.0, false, &atlas)
                .iter()
                .any(|q| q.blend == QuadBlend::Multiply)
        );
    }

    #[test]
    fn stored_tree_top_depth_override_matches_a_dynamic_tree_top() {
        let atlas = Atlas::load().unwrap();
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.world = spelunky_core::World::new(20, 20, Area::Jungle, 5);
        game.world.set_tile(4, 4, "oTree");
        let metadata = game.world.spawn("oTerrainMetadata", Vec2::new(72.0, 72.0));
        let e = game.world.entity_mut(metadata).unwrap();
        e.sprite.clear();
        e.text.insert("terrain_sprite".into(), "sTreeTop".into());
        e.data.insert("depth".into(), 1.0);
        let dynamic = game.world.spawn("oTree", Vec2::new(112.0, 64.0));
        let e = game.world.entity_mut(dynamic).unwrap();
        e.set_sprite("sTreeTop");
        e.data.insert("depth".into(), 1.0);
        // Depth 1 must draw after a depth-2 canopy neighbour for both storage forms.
        let branch = game.world.spawn("oTreeBranch", Vec2::new(80.0, 64.0));
        game.world
            .entity_mut(branch)
            .unwrap()
            .set_sprite("sTreeBranchRight");
        game.camera = Vec2::default();
        game.previous_camera = game.camera;
        let quads = game_scene(&game, 1.0, false, &atlas);
        let top_uv = atlas.sprite("sTreeTop", 0.0).unwrap().uv;
        let branch_uv = atlas.sprite("sTreeBranchRight", 0.0).unwrap().uv;
        let branch_index = quads.iter().position(|q| q.uv == branch_uv).unwrap();
        for x in [64.0, 112.0] {
            let index = quads
                .iter()
                .position(|q| q.uv == top_uv && q.rect[0] == x)
                .unwrap();
            assert!(index > branch_index, "tree tops use source depth 1");
        }
        game.world
            .entity_mut(metadata)
            .unwrap()
            .data
            .remove("depth");
        let ordinary = game_scene(&game, 1.0, false, &atlas);
        let tile_index = ordinary
            .iter()
            .position(|q| q.uv == top_uv && q.rect[0] == 64.0)
            .unwrap();
        let branch_index = ordinary.iter().position(|q| q.uv == branch_uv).unwrap();
        assert!(
            tile_index < branch_index,
            "fallback equal-depth order follows source creation order"
        );
    }

    #[test]
    fn equal_depth_terrain_entities_and_player_follow_source_instance_order() {
        use spelunky_core::dispatcher::Instance;

        let atlas = Atlas::load().unwrap();
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.world = spelunky_core::World::new(20, 20, Area::Mines, 5);
        game.world.set_tile(4, 4, "oBrick");
        game.player.pos = Vec2::new(64.0, 64.0);
        game.player.previous_pos = game.player.pos;
        game.player.data.insert("depth".into(), 100.0);
        let rock = game.world.spawn("oRock", Vec2::new(64.0, 64.0));
        game.world
            .entity_mut(rock)
            .unwrap()
            .data
            .insert("depth".into(), 100.0);

        let terrain = Instance::Terrain(4 + 4 * game.world.width);
        let entity = Instance::Entity(rock);
        let player = Instance::Player;
        let terrain_uv = atlas.sprite("sBrick", 0.0).unwrap().uv;
        let entity_uv = atlas.sprite("sRock", 0.0).unwrap().uv;

        // This order can result when Classic rebuilds its active list after
        // region activation. It must win even though the tile was born first.
        game.dispatcher.active_order = vec![terrain, entity, player];
        let terrain_first = game_scene(&game, 1.0, false, &atlas);
        let terrain_index = terrain_first
            .iter()
            .position(|q| q.uv == terrain_uv)
            .unwrap();
        let entity_index = terrain_first
            .iter()
            .position(|q| q.uv == entity_uv)
            .unwrap();
        let player_index = terrain_first
            .iter()
            .position(|q| presentation_player_frame_matches(&game, q, &atlas, 1.0))
            .unwrap();
        assert!(terrain_index < entity_index && entity_index < player_index);

        // Reversing the source list must reverse these equal-depth layers;
        // the renderer cannot impose actor-before-terrain ordering.
        game.dispatcher.active_order = vec![player, entity, terrain];
        let player_first = game_scene(&game, 1.0, false, &atlas);
        let terrain_index = player_first
            .iter()
            .position(|q| q.uv == terrain_uv)
            .unwrap();
        let entity_index = player_first.iter().position(|q| q.uv == entity_uv).unwrap();
        let player_index = player_first
            .iter()
            .position(|q| presentation_player_frame_matches(&game, q, &atlas, 1.0))
            .unwrap();
        assert!(player_index < entity_index && entity_index < terrain_index);

        // Before dispatcher registration, creation serials provide the source
        // order fallback, with the source player first.
        game.dispatcher.active_order.clear();
        let fallback = game_scene(&game, 1.0, false, &atlas);
        let terrain_index = fallback.iter().position(|q| q.uv == terrain_uv).unwrap();
        let entity_index = fallback.iter().position(|q| q.uv == entity_uv).unwrap();
        let player_index = fallback
            .iter()
            .position(|q| presentation_player_frame_matches(&game, q, &atlas, 1.0))
            .unwrap();
        assert!(player_index < terrain_index && terrain_index < entity_index);
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
            let source_frame = atlas.sprite("sHintPoint", draw.animation).unwrap();
            for alpha in [0.0, 0.5, 1.0] {
                let quad = game_scene(&game, alpha, false, &atlas)
                    .into_iter()
                    .find(|quad| {
                        quad.rect == [64.0, 144.0, 16.0, 16.0] && quad.uv == source_frame.uv
                    })
                    .expect("title hint must display floor(image_index) at every render alpha");
                assert_eq!(quad.next_uv, source_frame.uv);
                assert_eq!(quad.frame_mix, 0.0);
            }
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
    fn paid_markers_follow_source_draw_ticks_at_60_and_240_hz() {
        let atlas = Atlas::load().unwrap();
        assert!(
            atlas.sprite("sSmallCollect", 10.0).unwrap().uv
                != atlas.sprite("sSmallCollect", 0.0).unwrap().uv
        );
        for object in ["oBombBag", "oDamsel"] {
            for refresh_hz in [60_u64, 240] {
                let mut game = Game::shell(1, RulesPreset::Classic);
                game.world = spelunky_core::World::new(40, 20, Area::Mines, 1);
                game.world.name = "price-marker-rate-test".into();
                for x in 0..game.world.width {
                    game.world.set_tile(x, 10, "oBrick");
                }
                game.player.pos = Vec2::new(32.0, 152.0);
                game.player.previous_pos = game.player.pos;
                // oItem Step clears positive sale prices when no active
                // shopkeeper exists, so keep the stock in a valid shop flow.
                game.spawn("oShopkeeper", Vec2::new(280.0, 144.0));
                let id = game.spawn(object, Vec2::new(120.0, 144.0));
                game.world
                    .entity_mut(id)
                    .unwrap()
                    .data
                    .insert("cost".into(), 5000.0);
                let mut buffers = SceneBuffers::default();
                for render_frame in 0..refresh_hz {
                    let target_tick = 1 + render_frame * 30 / refresh_hz;
                    while game.tick < target_tick {
                        game.step(&spelunky_core::InputFrame::default());
                    }
                    assert!(
                        game.world.entity(id).is_some(),
                        "{object} at {refresh_hz} Hz"
                    );
                    assert_eq!(
                        game.world.entity(id).unwrap().value("cost"),
                        5000.0,
                        "{object} at {refresh_hz} Hz tick {}",
                        game.tick
                    );
                    let before = game.state_hash();
                    let alpha = (render_frame * 30 % refresh_hz) as f64 / refresh_hz as f64;
                    game_scene_in_view_into(
                        &game,
                        alpha,
                        false,
                        &atlas,
                        Viewport::canonical(),
                        &mut buffers,
                    );
                    let expected = ((target_tick - 1) % 10) as f64;
                    assert!(
                        buffers.quads.iter().any(|q| {
                            q.uv == atlas.sprite("sSmallCollect", expected).unwrap().uv
                                && q.frame_mix == 0.0
                        }),
                        "{object} at {refresh_hz} Hz render {render_frame}, tick {}, alpha {alpha}",
                        game.tick
                    );
                    assert_eq!(game.state_hash(), before, "render mutated Game");
                    assert_eq!(
                        game.world.entity(id).unwrap().value("cimg"),
                        (target_tick % 10) as f64,
                        "source Draw increments authoritative cimg once per fixed tick"
                    );
                }
                assert_eq!(game.tick, 30, "fixed simulation remains 30 Hz");
                assert_eq!(
                    game.presentation().render_history.price_marker_frames[&(
                        game.world.entity(id).unwrap().id,
                        game.world.entity(id).unwrap().creation_serial
                    )],
                    9.0,
                    "30 source Draws show frames 0 through 9 three times"
                );
            }
        }
    }

    #[test]
    fn newly_spawned_paid_marker_uses_its_first_source_draw_frame() {
        let atlas = Atlas::load().unwrap();
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.world = spelunky_core::World::new(40, 20, Area::Mines, 1);
        game.world.name = "price-marker-spawn-phase".into();
        for x in 0..game.world.width {
            game.world.set_tile(x, 10, "oBrick");
        }
        game.player.pos = Vec2::new(32.0, 152.0);
        game.player.previous_pos = game.player.pos;
        game.spawn("oShopkeeper", Vec2::new(280.0, 144.0));
        game.step(&spelunky_core::InputFrame::default());
        let id = game.spawn("oBombBag", Vec2::new(120.0, 144.0));
        game.world
            .entity_mut(id)
            .unwrap()
            .data
            .insert("cost".into(), 5000.0);
        let mut buffers = SceneBuffers::default();
        game.step(&spelunky_core::InputFrame::default());
        assert_eq!(game.tick, 2);
        assert_eq!(game.world.entity(id).unwrap().value("cimg"), 1.0);
        for _ in 0..8 {
            game_scene_in_view_into(
                &game,
                0.0,
                false,
                &atlas,
                Viewport::canonical(),
                &mut buffers,
            );
            assert!(
                buffers
                    .quads
                    .iter()
                    .any(|quad| { quad.uv == atlas.sprite("sSmallCollect", 0.0).unwrap().uv })
            );
        }
        let before = game.state_hash();
        game_scene_in_view_into(
            &game,
            0.0,
            false,
            &atlas,
            Viewport::canonical(),
            &mut buffers,
        );
        assert_eq!(game.state_hash(), before, "scene sampling is read-only");
        game.step(&spelunky_core::InputFrame::default());
        assert_eq!(game.world.entity(id).unwrap().value("cimg"), 2.0);
        game_scene_in_view_into(
            &game,
            0.0,
            false,
            &atlas,
            Viewport::canonical(),
            &mut buffers,
        );
        assert!(
            buffers
                .quads
                .iter()
                .any(|quad| { quad.uv == atlas.sprite("sSmallCollect", 1.0).unwrap().uv })
        );
    }

    #[test]
    fn offscreen_active_paid_marker_advances_on_source_draw_and_reuses_that_sample_when_visible() {
        let atlas = Atlas::load().unwrap();
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.world = spelunky_core::World::new(50, 20, Area::Mines, 1);
        game.world.name = "offscreen-price-marker-rate".into();
        for x in 0..game.world.width {
            game.world.set_tile(x, 10, "oBrick");
        }
        game.player.pos = Vec2::new(32.0, 152.0);
        game.player.previous_pos = game.player.pos;
        game.spawn("oShopkeeper", Vec2::new(280.0, 144.0));
        let id = game.spawn("oBombBag", Vec2::new(400.0, 144.0));
        game.world
            .entity_mut(id)
            .unwrap()
            .data
            .insert("cost".into(), 5000.0);
        for _ in 0..4 {
            game.step(&spelunky_core::InputFrame::default());
        }
        let bag = game.world.entity(id).unwrap();
        assert!(
            bag.engine_active,
            "the source activation margin includes x=400"
        );
        assert_eq!(bag.value("cimg"), 4.0);
        let key = (bag.id, bag.creation_serial);
        assert_eq!(
            game.presentation().render_history.price_marker_frames[&key],
            3.0
        );
        let before = game.state_hash();
        let mut buffers = SceneBuffers::default();
        game_scene_in_view_into(
            &game,
            0.0,
            false,
            &atlas,
            Viewport::canonical(),
            &mut buffers,
        );
        for frame in 0..10 {
            assert!(
                !buffers
                    .quads
                    .iter()
                    .any(|quad| quad.uv == atlas.sprite("sSmallCollect", frame as f64).unwrap().uv),
                "the paid marker is outside the canonical viewport"
            );
        }
        game.draw_view.as_mut().unwrap().camera = Vec2::new(320.0, 0.0);
        game.draw_view.as_mut().unwrap().previous_camera = Vec2::new(320.0, 0.0);
        game_scene_in_view_into(
            &game,
            0.0,
            false,
            &atlas,
            Viewport::canonical(),
            &mut buffers,
        );
        assert!(
            buffers
                .quads
                .iter()
                .any(|quad| { quad.uv == atlas.sprite("sSmallCollect", 3.0).unwrap().uv })
        );
        assert_eq!(game.world.entity(id).unwrap().value("cimg"), 4.0);
        assert_eq!(
            game.state_hash(),
            before,
            "presentation never advances cimg"
        );
    }

    #[test]
    fn overriding_item_draws_omit_price_markers_and_invisible_items_do_not_tick_cimg() {
        let atlas = Atlas::load().unwrap();
        let mut game = Game::shell(1, RulesPreset::Classic);
        for x in 0..game.world.width {
            game.world.set_tile(x, 10, "oBrick");
        }
        game.player.pos = Vec2::new(32.0, 152.0);
        game.player.previous_pos = game.player.pos;
        let keeper = game.spawn("oShopkeeper", Vec2::new(144.0, 144.0));
        let mut ids = Vec::new();
        for object in ["oArrow", "oDice", "oBombBag"] {
            let id = game.spawn(object, Vec2::new(120.0, 144.0));
            ids.push(id);
            let entity = game.world.entity_mut(id).unwrap();
            entity.data.insert("cost".into(), 5000.0);
            if object == "oBombBag" {
                entity.data.insert("visible".into(), 0.0);
            }
        }
        for _ in 0..12 {
            game.step(&spelunky_core::InputFrame::default());
        }
        assert!(game.world.entity(keeper).is_some());
        for &id in &ids {
            let entity = game.world.entity(id).expect("marker control must survive");
            assert_eq!(
                entity.value("cost"),
                5000.0,
                "Create must precede the price override"
            );
            assert_eq!(entity.value("cimg"), 0.0);
        }
        let mut buffers = SceneBuffers::default();
        game_scene_in_view_into(
            &game,
            0.5,
            false,
            &atlas,
            Viewport::canonical(),
            &mut buffers,
        );
        for frame in 0..20 {
            assert!(
                !buffers
                    .quads
                    .iter()
                    .any(|q| q.uv == atlas.sprite("sSmallCollect", frame as f64).unwrap().uv)
            );
        }
        for id in ids {
            let entity = game.world.entity(id).unwrap();
            assert!(
                !game
                    .presentation()
                    .render_history
                    .price_marker_frames
                    .contains_key(&(entity.id, entity.creation_serial)),
                "overridden or invisible Draw must not consume a marker frame"
            );
        }
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
            // Keep the configured render scene supported during full charging.
            for x in 0..game.world.width {
                game.world.set_tile(x, 6, "oBlock");
            }
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
                    // Below full charge the source's later charge gate would
                    // replace Bomb with Normal in this same Player Step.
                    for _ in 0..64 {
                        if game.player.data["bow_strength"] >= 12.0 {
                            break;
                        }
                        game.step(&spelunky_core::InputFrame {
                            attack: true,
                            ..Default::default()
                        });
                    }
                    assert!(game.player.data["bow_strength"] >= 12.0);
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
                    let owner =
                        interpolated_move_to(game.player.previous_pos, game.player.pos, alpha);
                    let camera = interpolated(game.previous_camera, game.camera, alpha);
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
    fn expired_bomb_arrow_is_not_rendered_while_fully_charged_bow_stays_armed() {
        let atlas = Atlas::load().unwrap();
        let mut game = Game::shell(42, spelunky_core::RulesPreset::Classic);
        game.world = spelunky_core::World::new(20, 20, Area::Mines, 1);
        for x in 0..20 {
            game.world.set_tile(x, 10, "oBlock");
        }
        game.player.pos = Vec2::new(80.0, 152.0);
        game.player.previous_pos = game.player.pos;
        game.player.state = PlayerState::Standing;
        game.player.grounded = true;
        // Keep the expiry blast from interrupting the armed-bow state this
        // renderer regression is measuring.
        game.player.cheat_invulnerable = true;
        let bow = game.world.spawn("oBow", game.player.pos);
        game.world.entity_mut(bow).unwrap().held = true;
        game.player.held = Some(bow);
        game.player.data.insert("bow_armed".into(), 1.0);
        game.player.data.insert("bow_strength".into(), 12.0);
        game.player.data.insert("bow_bomb_armed".into(), 1.0);
        game.player.data.insert("bow_bomb_fuse".into(), 0.0);
        game.player.data.insert("hold_arrow".into(), 2.0);
        game.player.data.insert("bow_arrow_toggle".into(), 0.0);
        game.player.alarms[11] = 1;

        game.step(&spelunky_core::InputFrame::default());

        assert_eq!(game.player.data.get("bow_armed"), Some(&1.0));
        assert_eq!(game.player.data.get("hold_arrow"), Some(&0.0));
        let arrow_uvs = [
            atlas.sprite("sArrowLeft", 0.0).unwrap().uv,
            atlas.sprite("sArrowRight", 0.0).unwrap().uv,
            atlas.sprite("sBombArrowLeft", 0.0).unwrap().uv,
            atlas.sprite("sBombArrowRight", 0.0).unwrap().uv,
            atlas.sprite("sBombArrowLeft", 1.0).unwrap().uv,
            atlas.sprite("sBombArrowRight", 1.0).unwrap().uv,
        ];
        let quads = game_scene(&game, 1.0, false, &atlas);
        assert!(
            quads.iter().all(|quad| !arrow_uvs.contains(&quad.uv)),
            "Classic clears holdArrow when the bomb-arrow fuse expires"
        );
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
        assert_eq!(game.player.health, 3.0);
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
        for alpha in [0.0, 0.125, 0.25, 0.5, 0.875, 1.0] {
            let quads = game_scene(&game, alpha, false, &atlas);
            let player = quads
                .iter()
                .find(|quad| presentation_player_frame_matches(&game, quad, &atlas, alpha))
                .unwrap();
            let whip_entity = game.world.entity(id).unwrap();
            let whip = quads
                .iter()
                .find(|quad| {
                    presentation_entity_frame_matches(
                        &game,
                        whip_entity,
                        quad,
                        &atlas,
                        alpha,
                        false,
                    )
                })
                .unwrap();
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
        hub_hud(
            &mut quads,
            &game,
            game.camera,
            &atlas,
            Viewport::canonical(),
        );
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
        hub_hud(
            &mut quads,
            &game,
            game.camera,
            &atlas,
            Viewport::canonical(),
        );
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
    fn arrow_rotation_uses_shortest_wrap_and_newborns_start_at_current_angle() {
        let atlas = Atlas::load().unwrap();
        let previous = std::f64::consts::PI - 0.1;
        let current = -std::f64::consts::PI + 0.1;
        let mut game = Game::shell(1, RulesPreset::Classic);
        let id = game.world.spawn("oArrow", Vec2::new(120.0, 100.0));
        let arrow = game.world.entity_mut(id).unwrap();
        arrow.sprite = "sArrowRight".into();
        arrow.data.insert("rotation".into(), current);
        let key = (arrow.id, arrow.creation_serial);
        game.render_history.arrow_rotation.insert(key, previous);
        game.level_tick = 1;
        let uv = atlas.sprite("sArrowRight", 0.0).unwrap().uv;
        let hash_before_draws = game.state_hash();

        for alpha in [0.0, 0.5, 1.0] {
            let quad = game_scene(&game, alpha, false, &atlas)
                .into_iter()
                .find(|quad| quad.uv == uv)
                .expect("rotated arrow");
            if alpha == 0.5 {
                assert!((f64::from(quad.rotation).abs() - std::f64::consts::PI).abs() < 1e-6);
            } else {
                let expected = if alpha == 0.0 { previous } else { current };
                let difference = (f64::from(quad.rotation) - expected + std::f64::consts::PI)
                    .rem_euclid(std::f64::consts::TAU)
                    - std::f64::consts::PI;
                assert!(
                    difference.abs() < 1e-6,
                    "angle {} vs {expected}",
                    quad.rotation
                );
            }
            assert_eq!(game.state_hash(), hash_before_draws);
        }

        let mut newborn = Game::shell(2, RulesPreset::Classic);
        let id = newborn.world.spawn("oArrow", Vec2::new(120.0, 100.0));
        let current = 1.25;
        let arrow = newborn.world.entity_mut(id).unwrap();
        arrow.sprite = "sArrowRight".into();
        arrow.data.insert("rotation".into(), current);
        assert!(newborn.render_history.arrow_rotation.is_empty());
        let hash_before_draws = newborn.state_hash();
        for alpha in [0.0, 0.5, 1.0] {
            let quad = game_scene(&newborn, alpha, false, &atlas)
                .into_iter()
                .find(|quad| quad.uv == uv)
                .expect("newborn arrow");
            assert_eq!(quad.rotation, current as f32);
            assert_eq!(newborn.state_hash(), hash_before_draws);
        }
    }

    #[test]
    fn flying_fish_bone_keeps_rotated_collision_mask_but_inherits_zero_angle_draw() {
        let atlas = Atlas::load().unwrap();
        let mut level = spelunky_core::level::Level::default();
        level.set_tile(4, 8, '@').unwrap();
        for x in 0..40 {
            level.set_tile(x, 9, '1').unwrap();
        }
        let mut game = Game::shell(42, RulesPreset::Classic);
        game.load_custom(&level, "fish-bone-draw", "");
        // Public creation performs the real callbacks. Only initial launch
        // velocities are authored; every angle below comes from Game::step.
        let bone = game.spawn("oFishBone", Vec2::new(144.0, 64.0));
        let arrow = game.spawn("oArrow", Vec2::new(192.0, 64.0));
        for id in [bone, arrow] {
            game.world.entity_mut(id).unwrap().vel = Vec2::new(3.0, -2.5);
        }
        for _ in 0..5 {
            game.step(&spelunky_core::InputFrame::default());
            let draw = game.presentation();
            let bone = draw.world.entity(bone).expect("flying Fish Bone survives");
            let arrow = draw.world.entity(arrow).expect("flying Arrow survives");
            assert!(bone.engine_active && arrow.engine_active);
            assert!(bone.pos.x > 144.0 && arrow.pos.x > 192.0);
            assert!(bone.value("rotation").abs() > 0.01);
            assert!(arrow.value("rotation").abs() > 0.01);
            let axis_aligned = spelunky_core::catalog::instance_sprite_bounds(
                &bone.object,
                &bone.sprite,
                bone.pos,
                bone.animation,
                Vec2::new(1.0, 1.0),
                0.0,
            );
            assert_ne!(
                bone.sprite_bounds(),
                axis_aligned,
                "source collision still uses the actual flight angle"
            );
            let bone_uv = atlas.sprite(&bone.sprite, bone.animation).unwrap().uv;
            let arrow_uv = atlas.sprite(&arrow.sprite, arrow.animation).unwrap().uv;
            let state = game.state_hash();
            let draw_state = draw.state_hash();
            for alpha in [0.0, 0.125, 0.5, 0.875, 1.0] {
                let quads = game_scene(&game, alpha, false, &atlas);
                let bone_quad = quads.iter().find(|q| q.uv == bone_uv).unwrap();
                let arrow_quad = quads.iter().find(|q| q.uv == arrow_uv).unwrap();
                assert_eq!(
                    bone_quad.rotation, 0.0,
                    "oFishBone inherits the literal-zero-angle oItem Draw"
                );
                let previous_rotation = draw
                    .render_history
                    .arrow_rotation
                    .get(&(arrow.id, arrow.creation_serial))
                    .copied()
                    .unwrap_or_else(|| arrow.value("rotation"));
                assert_eq!(
                    arrow_quad.rotation,
                    interpolated_angle(previous_rotation, arrow.value("rotation"), alpha) as f32,
                    "oArrow's own Draw interpolates image_angle"
                );
                assert_eq!(game.state_hash(), state);
                assert_eq!(game.presentation().state_hash(), draw_state);
            }
        }
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
        // Source cached aggregate StandLeft at(100,100) ends x105;
        // positive-unit LampRedItem at(150,100) starts x143. Y overlaps.
        let distance = 38.0;
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
    fn editor_entrance_and_exit_draw_their_white_destination_labels_only_in_the_editor_room() {
        let atlas = Atlas::load().unwrap();
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.world.name = "rLevelEditor".into();
        let exit = game.world.spawn("oExit", Vec2::new(100.0, 80.0));
        game.world
            .entity_mut(exit)
            .unwrap()
            .text
            .insert("leadsTo".into(), "bonus-level".into());
        let entrance = game.world.spawn("oEntrance", Vec2::new(140.0, 80.0));
        game.world
            .entity_mut(entrance)
            .unwrap()
            .text
            .insert("leadsTo".into(), "next-level".into());

        let mut expected = Vec::new();
        text(
            &mut expected,
            &atlas,
            "bonus-level",
            100.0,
            80.0,
            1.0,
            WHITE,
        );
        let mut expected_entrance = Vec::new();
        text(
            &mut expected_entrance,
            &atlas,
            "next-level",
            140.0,
            80.0,
            1.0,
            WHITE,
        );
        let editor_quads = game_scene(&game, 1.0, false, &atlas);
        for glyph in expected.iter().chain(&expected_entrance) {
            assert!(editor_quads.iter().any(|quad| {
                quad.uv == glyph.uv && quad.rect == glyph.rect && quad.color == WHITE
            }));
        }

        game.world.name = "rLevel".into();
        let gameplay_quads = game_scene(&game, 1.0, false, &atlas);
        assert!(
            !expected
                .iter()
                .all(|glyph| gameplay_quads.iter().any(|quad| {
                    quad.uv == glyph.uv && quad.rect == glyph.rect && quad.color == WHITE
                }))
        );
    }

    #[test]
    fn adaptive_view_keeps_overhanging_source_terrain_sprite_at_right_edge() {
        let atlas = Atlas::load().unwrap();
        let mut game = Game::shell(1, RulesPreset::Classic);
        // oKaliHead Alarm 0 changes the terrain instance's sprite to sGTHHole
        // and stores the visual override at its cell center. Its 32x32 sprite
        // has origin (16,16), so a 16px tile at x=432 reaches back into the
        // 426.67px-wide logical view used by a 1920x1080 surface.
        game.world.set_tile(27, 5, "oKaliHead");
        let metadata = game.world.spawn("oTerrainMetadata", Vec2::new(440.0, 88.0));
        game.world
            .entity_mut(metadata)
            .unwrap()
            .text
            .insert("terrain_sprite".into(), "sGTHHole".into());

        let view = Viewport::from_surface(1920, 1080);
        let quads = game_scene_in_view(&game, 1.0, false, &atlas, view);
        let hole = atlas.sprite("sGTHHole", 0.0).unwrap();
        let drawn: Vec<_> = quads.iter().filter(|quad| quad.uv == hole.uv).collect();
        assert_eq!(drawn.len(), 1, "the source terrain image overlaps the view");
        assert_eq!(drawn[0].rect, [416.0, 64.0, 32.0, 32.0]);
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
    fn camera_interpolates_diagonal_follow_in_source_axis_order() {
        let atlas = Atlas::load().unwrap();
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.player.sprite = "sStandLeft".into();
        game.player.facing = 1;
        game.player.set_image_speed(0.0);
        game.player.previous_pos = Vec2::new(80.0, 80.0);
        game.player.pos = game.player.previous_pos;
        game.previous_camera = Vec2::default();
        game.camera = Vec2::new(4.0, 4.0);

        let mut mirrored_uv = atlas.sprite("sStandLeft", 0.0).unwrap().uv;
        mirrored_uv[0] += mirrored_uv[2];
        mirrored_uv[2] = -mirrored_uv[2];
        let rendered_player = |alpha| {
            game_scene(&game, alpha, false, &atlas)
                .into_iter()
                .find(|quad| quad.uv == mirrored_uv)
                .expect("standing player sprite")
                .rect
        };
        let start = rendered_player(0.0);
        let halfway = rendered_player(0.5);

        assert_eq!(halfway[0], start[0] - 4.0);
        assert_eq!(halfway[1], start[1]);
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

    #[test]
    fn high_refresh_player_interpolation_follows_collision_safe_axis_order() {
        let atlas = Atlas::load().unwrap();
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.world.set_tile(7, 6, "oBlock");
        game.player.state = PlayerState::Falling;
        game.player.sprite = "sFallLeft".into();
        game.player.facing = 1;
        game.player.set_image_speed(0.0);
        game.player.previous_pos = Vec2::new(128.0, 86.0);
        game.player.pos = Vec2::new(134.0, 91.0);
        game.previous_camera = Vec2::default();
        game.camera = Vec2::default();

        let mut mirrored_uv = atlas.sprite("sFallLeft", 0.0).unwrap().uv;
        mirrored_uv[0] += mirrored_uv[2];
        mirrored_uv[2] = -mirrored_uv[2];
        for alpha in [0.0, 0.5, 1.0] {
            let quads = game_scene(&game, alpha, false, &atlas);
            let player = quads
                .iter()
                .find(|quad| quad.uv == mirrored_uv)
                .expect("falling player sprite");
            assert!(
                !opaque_sprite_quad_overlaps_rect(player, &atlas, [112.0, 96.0, 16.0, 16.0]),
                "player sprite clips the block at alpha {alpha}: {:?}",
                player.rect
            );
        }
    }

    #[test]
    fn high_refresh_item_interpolation_follows_move_to_around_block_corners() {
        let atlas = Atlas::load().unwrap();
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.world.set_tile(7, 6, "oBlock");
        let id = game.world.spawn("oArrow", Vec2::new(134.0, 114.0));
        let arrow = game.world.entity_mut(id).unwrap();
        arrow.previous_pos = Vec2::new(106.0, 90.0);
        arrow.sprite = "sArrowRight".into();
        arrow.data.insert("rotation".into(), 0.0);

        // Classic moveTo completes the X pass before beginning its Y pass.
        // At alpha 0.5 that puts the arrow at (132, 90), clear of the tile.
        // Straight interpolation would put its opaque pixels inside the block.
        assert_eq!(
            entity_position(&game, game.world.entity(id).unwrap(), 0.5),
            Vec2::new(132.0, 90.0)
        );
        let mut linear_quads = Vec::new();
        sprite(
            &mut linear_quads,
            &atlas,
            "sArrowRight",
            0.0,
            120.0,
            102.0,
            false,
            WHITE,
        );
        assert!(opaque_sprite_quad_overlaps_rect(
            &linear_quads[0],
            &atlas,
            [112.0, 96.0, 16.0, 16.0]
        ));

        let arrow_uv = atlas.sprite("sArrowRight", 0.0).unwrap().uv;
        let quads = game_scene(&game, 0.5, false, &atlas);
        let arrow = quads
            .iter()
            .find(|quad| quad.uv == arrow_uv)
            .expect("arrow sprite");
        assert!(
            !opaque_sprite_quad_overlaps_rect(arrow, &atlas, [112.0, 96.0, 16.0, 16.0]),
            "interpolated arrow clips the block at alpha 0.5: {:?}",
            arrow.rect
        );
    }

    #[test]
    fn move_to_entity_families_follow_source_axis_order_around_block_corners() {
        let atlas = Atlas::load().unwrap();
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.world.set_tile(7, 6, "oBlock");
        let previous = Vec2::new(106.0, 80.0);
        let current = Vec2::new(134.0, 104.0);

        // These source Steps all call moveTo, either directly or by inherited
        // family Step. The horizontal segment passes above the block, then the
        // vertical segment passes to its right. A straight midpoint clips it.
        for object in ["oCaveman", "oDiamond", "oBlood", "oArrow", "oBoulder"] {
            let id = game.world.spawn(object, current);
            let entity = game.world.entity_mut(id).unwrap();
            entity.previous_pos = previous;
            assert_eq!(
                entity_position(&game, game.world.entity(id).unwrap(), 0.5),
                Vec2::new(132.0, 80.0),
                "source moveTo interpolation for {object}"
            );
        }

        let id = game.world.spawn("oCaveman", current);
        let enemy = game.world.entity_mut(id).unwrap();
        enemy.previous_pos = previous;
        enemy.sprite = "sCavemanLeft".into();
        enemy.set_image_speed(0.0);
        enemy.facing = -1;

        let mut linear_quads = Vec::new();
        sprite(
            &mut linear_quads,
            &atlas,
            "sCavemanLeft",
            0.0,
            120.0,
            92.0,
            false,
            WHITE,
        );
        assert!(opaque_sprite_quad_overlaps_rect(
            &linear_quads[0],
            &atlas,
            [112.0, 96.0, 16.0, 16.0]
        ));

        let caveman_uv = atlas.sprite("sCavemanLeft", 0.0).unwrap().uv;
        let quads = game_scene(&game, 0.5, false, &atlas);
        let caveman = quads
            .iter()
            .find(|quad| quad.uv == caveman_uv)
            .expect("Caveman Draw");
        assert_eq!(caveman.rect, [132.0, 80.0, 16.0, 16.0]);
        assert!(!opaque_sprite_quad_overlaps_rect(
            caveman,
            &atlas,
            [112.0, 96.0, 16.0, 16.0]
        ));
    }

    #[test]
    fn damsel_exit_position_snap_is_not_render_interpolated() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        let id = game.world.spawn("oDamsel", Vec2::new(120.0, 104.0));
        let damsel = game.world.entity_mut(id).unwrap();
        damsel.previous_pos = Vec2::new(96.0, 80.0);
        damsel.sprite = "sDamselExit2".into();

        assert_eq!(
            entity_position(&game, game.world.entity(id).unwrap(), 0.5),
            Vec2::new(120.0, 104.0)
        );
    }

    #[test]
    fn held_item_and_player_light_follow_high_refresh_player_path() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.player.previous_pos = Vec2::new(40.0, 40.0);
        game.player.pos = Vec2::new(48.0, 48.0);
        let held = game.spawn("oRock", Vec2::new(52.0, 50.0));
        game.player.held = Some(held);
        let rock = game.world.entity_mut(held).unwrap();
        rock.held = true;
        rock.previous_pos = Vec2::new(44.0, 42.0);

        // At alpha 0.5, X-then-Y movement puts the player at (48,40).
        // The held item must retain its (4,2) owner offset, and the player
        // light must stay centered on that same presentation position.
        assert_eq!(
            entity_position(&game, game.world.entity(held).unwrap(), 0.5),
            Vec2::new(52.0, 42.0)
        );
        assert_eq!(scene_lights(&game, 0.5)[0].position, Vec2::new(48.0, 40.0));
    }

    #[test]
    fn grabbed_monkey_tracks_the_owners_collision_resolved_path() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.player.previous_pos = Vec2::new(106.0, 80.0);
        game.player.pos = Vec2::new(134.0, 104.0);
        let id = game.world.spawn("oMonkey", Vec2::new(140.0, 96.0));
        let monkey = game.world.entity_mut(id).unwrap();
        monkey.state = 7;
        monkey.previous_pos = Vec2::new(80.0, 120.0);

        // At alpha 0.5, the player completes the horizontal part of moveTo
        // first. The grabbed monkey remains six pixels right and eight above.
        assert_eq!(
            entity_position(&game, game.world.entity(id).unwrap(), 0.5),
            Vec2::new(138.0, 72.0)
        );
    }
}

#[cfg(test)]
#[path = "inherited_enemy_draw_tests.rs"]
mod inherited_enemy_draw_tests;

#[cfg(test)]
#[path = "ghost_player_visibility_draw_tests.rs"]
mod ghost_player_visibility_draw_tests;

#[cfg(test)]
#[path = "configured_whip_draw_tests.rs"]
mod configured_whip_draw_tests;

#[cfg(test)]
#[path = "hawkman_worship_draw_tests.rs"]
mod hawkman_worship_draw_tests;

#[cfg(test)]
#[path = "weapon_default_draw_tests.rs"]
mod weapon_default_draw_tests;

#[cfg(test)]
#[path = "source_red_lamp_begin_tests.rs"]
mod source_red_lamp_begin_tests;

#[cfg(test)]
#[path = "scene_generated_graphics_tests.rs"]
mod generated_graphics_tests;

#[cfg(test)]
#[path = "scene_draw_order_tests.rs"]
mod draw_order_tests;
