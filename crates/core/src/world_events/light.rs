//! Source lighting queries share one ordered scan of the visible room.
use super::mask_distance;
use crate::{Game, Rect, Vec2, catalog};
use std::{
    collections::{BTreeMap, HashMap},
    sync::OnceLock,
};

const FAMILIES: [&str; 13] = [
    "oExplosion",
    "oLava",
    "oLamp",
    "oLampItem",
    "oFlareCrate",
    "oTikiTorch",
    "oArrowTrapLeftLit",
    "oArrowTrapRightLit",
    "oSpearTrapLit",
    "oSmashTrapLit",
    "oShotgunBlastLeft",
    "oShotgunBlastRight",
    "oFlare",
];
const BEFORE_PLAYER: u16 = (1 << 12) - 1;
const FLARE: u16 = 1 << 12;
const PENALTIES: [f64; 12] = [0., 0., 0., 0., 0., 48., 48., 48., 48., 48., 0., 0.];

#[derive(Clone, Copy, Debug, PartialEq)]
struct LightSource {
    mask: Rect,
    animation: f64,
    flare_distance: f64,
}

fn family_bits(object: &str) -> u16 {
    static MEMBERS: OnceLock<BTreeMap<&'static str, u16>> = OnceLock::new();
    MEMBERS
        .get_or_init(|| {
            catalog::object_names()
                .filter_map(|object| {
                    let bits = FAMILIES
                        .iter()
                        .enumerate()
                        .fold(0, |bits, (index, family)| {
                            bits | if catalog::is_a(object, family) {
                                1 << index
                            } else {
                                0
                            }
                        });
                    (bits != 0).then_some((object, bits))
                })
                .collect()
        })
        .get(object)
        .copied()
        .unwrap_or(0)
}

#[derive(Clone, Copy)]
enum SourceRef {
    Tile(usize),
    Entity(usize),
}

fn resolve(game: &Game, source: SourceRef) -> LightSource {
    match source {
        SourceRef::Tile(index) => {
            let x = index % game.world.width;
            let y = index / game.world.width;
            let animation = game
                .world
                .entities
                .iter()
                .find(|entity| {
                    entity.alive
                        && entity.object == "oTerrainMetadata"
                        && (entity.pos.x / 16.).floor() as usize == x
                        && (entity.pos.y / 16.).floor() as usize == y
                })
                .map_or(0., |entity| entity.animation);
            LightSource {
                mask: game
                    .world
                    .tile_collision_mask(x as i32, y as i32)
                    .point_bounds(),
                animation,
                flare_distance: 0.,
            }
        }
        SourceRef::Entity(index) => {
            let entity = &game.world.entities[index];
            LightSource {
                mask: entity.collision_mask().point_bounds(),
                animation: entity.animation,
                flare_distance: entity.data.get("dist_to_player").copied().unwrap_or(0.),
            }
        }
    }
}

fn nearest_sources(game: &Game, requested: u16) -> [Option<LightSource>; 13] {
    let mut sources = [None; 13];
    let mut distances = [f64::INFINITY; 13];
    let mut consider = |mut bits: u16, position: Vec2, source: SourceRef| {
        if bits == 0 {
            return;
        }
        let dx = position.x - game.player.pos.x;
        let dy = position.y - game.player.pos.y;
        let distance = dx * dx + dy * dy;
        while bits != 0 {
            let family = bits.trailing_zeros() as usize;
            bits &= bits - 1;
            // Source instance_nearest keeps the first origin on an exact tie.
            if distance < distances[family] {
                distances[family] = distance;
                sources[family] = Some(source);
            }
        }
    };
    // instance_nearest(object) walks that object's descendant array. Each
    // family preserves the same registration/rebuild order, including a
    // reactivated instance appended behind previously active instances.
    // Index only requested emitters so this remains one family scan rather
    // than repeating entity lookups for all thirteen source queries.
    let entities: HashMap<_, _> = game
        .world
        .entities
        .iter()
        .enumerate()
        .filter_map(|(index, entity)| {
            let bits = family_bits(&entity.object) & requested;
            (entity.alive && entity.engine_active && bits != 0)
                .then_some((entity.id, (bits, entity.pos, SourceRef::Entity(index))))
        })
        .collect();
    for instance in &game.dispatcher.registration_order {
        match *instance {
            crate::dispatcher::Instance::Terrain(index) => {
                let Some(tile) = game
                    .world
                    .tiles
                    .get(index)
                    .filter(|tile| tile.engine_active)
                else {
                    continue;
                };
                let bits = family_bits(&tile.object) & requested;
                if bits != 0 {
                    consider(
                        bits,
                        Vec2::new(
                            (index % game.world.width) as f64 * 16.,
                            (index / game.world.width) as f64 * 16.,
                        ),
                        SourceRef::Tile(index),
                    );
                }
            }
            crate::dispatcher::Instance::Entity(id) => {
                if let Some(&(bits, position, source)) = entities.get(&id) {
                    consider(bits, position, source);
                }
            }
            crate::dispatcher::Instance::Player => {}
        }
    }
    sources.map(|source| source.map(|source| resolve(game, source)))
}

/// oPlayer1 samples all twelve source families before character movement.
pub(crate) fn sample_light_distances(game: &mut Game) {
    let player = game.player.collision_mask().point_bounds();
    let sources = nearest_sources(game, BEFORE_PLAYER);
    let mut distance = 999.0_f64;
    if let Some(source) = sources[0] {
        distance = mask_distance(player, source.mask);
        if source.animation <= 3. {
            distance -= source.animation * 16.;
        } else {
            distance += (source.animation - 3.) * 16.;
        }
    }
    for (index, source) in sources[..12].iter().enumerate().skip(1) {
        if let Some(source) = source {
            distance = distance.min(mask_distance(player, source.mask) + PENALTIES[index]);
        }
    }
    game.player
        .data
        .insert("dist_to_nearest_light_source".into(), distance);
}

/// oLevel later selects the nearest Flare using the player's new position,
/// then consumes its existing cache and the earlier player-family cache.
pub(crate) fn lighting_tick(game: &mut Game) {
    if !game.world.dark || game.flags.contains("hub") {
        return;
    }
    let mut distance = if game.player.has("Crown") {
        0.
    } else {
        nearest_sources(game, FLARE)[12].map_or(160., |source| source.flare_distance)
    };
    let nearest = game
        .player
        .data
        .get("dist_to_nearest_light_source")
        .copied()
        .unwrap_or(999.);
    if nearest < 200. && nearest < distance {
        distance = nearest;
    }
    let mut darkness = if distance == 0. { 0. } else { distance / 160. };
    let lerp = game.data.get("darkness_lerp").copied().unwrap_or(0.);
    if lerp > 0. {
        darkness = lerp;
        game.data.insert("darkness_lerp".into(), lerp - 0.1);
    }
    game.data.insert("darkness".into(), darkness.min(0.9));
}

const SCREEN_RED_SAMPLE_TICK: &str = "source_screen_red_sample_tick";
const SCREEN_RED_DISTANCE_KEYS: [&str; 2] = [
    "source_screen_static_red_distance",
    "source_screen_carried_red_distance",
];

// distance_to_object compares all active family cached inclusive envelopes;
// it does not first choose an instance by origin distance. Querying is pure.
fn current_red_lamp_distances(game: &Game) -> [Option<f64>; 2] {
    if !game.player.engine_active {
        return [None, None];
    }
    let player = game.player.collision_mask().point_bounds();
    ["oLampRed", "oLampRedItem"].map(|family| {
        game.world
            .entities
            .iter()
            .filter(|e| e.alive && e.engine_active && catalog::is_a(&e.object, family))
            .map(|e| mask_distance(player, e.collision_mask().point_bounds()))
            .chain(
                game.world
                    .tiles
                    .iter()
                    .enumerate()
                    .filter(|(_, t)| t.engine_active && catalog::is_a(&t.object, family))
                    .map(|(index, _)| {
                        mask_distance(
                            player,
                            game.world
                                .tile_collision_mask(
                                    (index % game.world.width) as i32,
                                    (index / game.world.width) as i32,
                                )
                                .point_bounds(),
                        )
                    }),
            )
            .min_by(f64::total_cmp)
            .map(|distance| distance.min(1_000_000.0))
    })
}

/// Literal oScreen Begin Step samples red tint before alarms and movement.
/// The Screen receiver adapter is shared with that event's message callback.
pub(super) fn capture_screen_red_lamps(game: &mut Game, screen_present: bool) {
    game.data.remove(SCREEN_RED_SAMPLE_TICK);
    for key in SCREEN_RED_DISTANCE_KEYS {
        game.data.remove(key);
    }
    if !screen_present
        || !game.world.dark
        || game.player.state == crate::PlayerState::Dead
        || game.state == crate::RunState::Dead
    {
        return;
    }
    let distances = current_red_lamp_distances(game);
    game.data
        .insert(SCREEN_RED_SAMPLE_TICK.into(), game.level_tick as f64);
    for (key, distance) in SCREEN_RED_DISTANCE_KEYS.into_iter().zip(distances) {
        if let Some(distance) = distance {
            game.data.insert(key.into(), distance);
            // Source runs the static class first and carried class second.
            // Even an out-of-range second class writes this Player variable.
            game.player.data.insert("dist_to_lamp".into(), distance);
        }
    }
}

/// Presentation reads the admitted simulation sample without resampling it.
/// Only pre-first-step fixture/room Draw has a pure current-pose fallback.
/// A stepped runtime missing a matching sample cannot silently query again.
pub fn screen_red_lamp_distances(game: &Game) -> [Option<f64>; 2] {
    if game.data.get(SCREEN_RED_SAMPLE_TICK) == Some(&(game.level_tick as f64)) {
        SCREEN_RED_DISTANCE_KEYS.map(|key| game.data.get(key).copied())
    } else if game.level_tick == 0 {
        current_red_lamp_distances(game)
    } else {
        [None, None]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Area, RulesPreset, World};

    // Independent per-family source query: walk the actual descendant array
    // and compare squared origins, without the indexed selector's family table.
    fn original_nearest(game: &Game, family: &str) -> Option<LightSource> {
        let mut nearest = None;
        let mut distance = f64::INFINITY;
        for instance in crate::dispatcher::descendant_instances(game, family) {
            if !crate::dispatcher::is_active(game, instance) {
                continue;
            }
            let (position, source) = match instance {
                crate::dispatcher::Instance::Terrain(index) => {
                    if game.world.tiles[index].object.is_empty() {
                        continue;
                    }
                    let x = index % game.world.width;
                    let y = index / game.world.width;
                    let animation = game
                        .world
                        .entities
                        .iter()
                        .find(|entity| {
                            entity.alive
                                && entity.object == "oTerrainMetadata"
                                && (entity.pos.x / 16.).floor() as usize == x
                                && (entity.pos.y / 16.).floor() as usize == y
                        })
                        .map_or(0., |entity| entity.animation);
                    (
                        Vec2::new(x as f64 * 16., y as f64 * 16.),
                        LightSource {
                            mask: game
                                .world
                                .tile_collision_mask(x as i32, y as i32)
                                .point_bounds(),
                            animation,
                            flare_distance: 0.,
                        },
                    )
                }
                crate::dispatcher::Instance::Entity(id) => {
                    let Some(entity) = game.world.entity(id) else {
                        continue;
                    };
                    (
                        entity.pos,
                        LightSource {
                            mask: entity.collision_mask().point_bounds(),
                            animation: entity.animation,
                            flare_distance: entity
                                .data
                                .get("dist_to_player")
                                .copied()
                                .unwrap_or(0.),
                        },
                    )
                }
                crate::dispatcher::Instance::Player => continue,
            };
            let dx = position.x - game.player.pos.x;
            let dy = position.y - game.player.pos.y;
            let candidate = dx * dx + dy * dy;
            if candidate < distance {
                distance = candidate;
                nearest = Some(source);
            }
        }
        nearest
    }
    fn room() -> Game {
        let mut game = Game::shell(42, RulesPreset::Classic);
        game.world = World::new(16, 12, Area::Mines, 1);
        game.player.pos = Vec2::new(80., 80.);
        game
    }
    fn assert_sources(game: &mut Game) {
        crate::dispatcher::activate_object(game, "oDrawnSprite");
        for requested in [BEFORE_PLAYER, FLARE, BEFORE_PLAYER | FLARE] {
            let sources = nearest_sources(game, requested);
            for (index, family) in FAMILIES.iter().enumerate() {
                let expected = if requested & (1 << index) != 0 {
                    original_nearest(game, family)
                } else {
                    None
                };
                assert_eq!(sources[index], expected, "family {family}");
            }
        }
    }
    #[test]
    fn every_bundled_emitter_descendant_keeps_current_mask_and_cache() {
        let mut count = 0;
        for object in catalog::object_names()
            .filter(|name| FAMILIES.iter().any(|family| catalog::is_a(name, family)))
        {
            let mut game = room();
            let id = game.world.spawn(object, Vec2::new(89.25, 75.5));
            let source = game.world.entity_mut(id).unwrap();
            source.animation = 1.25;
            source.data.insert("dist_to_player".into(), 37.5);
            game.world.spawn(object, Vec2::new(160.5, 88.25));
            for animation in [0., 1.25, 3.5] {
                game.world.entity_mut(id).unwrap().animation = animation;
                assert_sources(&mut game);
            }
            count += 1;
        }
        assert!(count >= FAMILIES.len());
        assert_eq!(family_bits("not-a-bundled-object"), 0);
    }
    #[test]
    fn tile_and_entity_ties_keep_registration_order_and_first_metadata_animation() {
        let mut game = room();
        game.player.pos = Vec2::new(88., 80.);
        game.world.set_tile(5, 5, "oExplosion");
        game.world.set_tile(6, 5, "oExplosion");
        let metadata = game.world.spawn("oTerrainMetadata", Vec2::new(80., 80.));
        let first = game.world.entity_mut(metadata).unwrap();
        first.sprite = "sExplosion".into();
        first.animation = 2.25;
        let duplicate = game.world.spawn("oTerrainMetadata", Vec2::new(80., 80.));
        game.world.entity_mut(duplicate).unwrap().animation = 4.5;
        let id = game.world.spawn("oExplosion", Vec2::new(96., 80.));
        game.world.entity_mut(id).unwrap().animation = 4.;
        assert_sources(&mut game);
        let source = nearest_sources(&game, BEFORE_PLAYER)[0].unwrap();
        assert_eq!(source.animation, 2.25);
        // Source aggregate sExplosion [0,2,47,45], origin24 at(80,80):
        // inclusive bounds [56,58,103,101], independent of mask subimage.
        assert_eq!(source.mask, Rect::new(56., 58., 48., 44.));
        game.world.entity_mut(metadata).unwrap().alive = false;
        assert_sources(&mut game);
        assert_eq!(
            nearest_sources(&game, BEFORE_PLAYER)[0].unwrap().animation,
            4.5
        );
        game.world.set_tile(5, 5, "oLampRed");
        assert_sources(&mut game);
    }
    #[test]
    fn nearest_origin_precedes_mask_distance_and_membership_never_freezes_instances() {
        let mut game = room();
        let first = game.world.spawn("oLamp", Vec2::new(88., 80.));
        let second = game.world.spawn("oLampRed", Vec2::new(88., 80.));
        game.world.entity_mut(first).unwrap().sprite = "sExplosion".into();
        game.world.entity_mut(first).unwrap().animation = 3.;
        assert_sources(&mut game);
        assert_eq!(
            nearest_sources(&game, BEFORE_PLAYER)[2].unwrap().animation,
            3.
        );
        game.world.entity_mut(first).unwrap().alive = false;
        assert_sources(&mut game);
        game.world.entity_mut(second).unwrap().pos = Vec2::new(300., 300.);
        game.world.spawn("oLampRed", Vec2::new(82.5, 81.25));
        assert_sources(&mut game);
        game.world.entities.retain(|entity| entity.id != second);
        assert_sources(&mut game);
    }
    #[test]
    fn closer_origin_wins_even_when_farther_sprite_mask_is_closer() {
        let mut game = room();
        let near = game.world.spawn("oLamp", Vec2::new(100., 80.));
        let far = game.world.spawn("oLampRed", Vec2::new(112., 80.));
        game.world.entity_mut(far).unwrap().sprite = "sExplosion".into();
        let player = game.player.collision_mask().point_bounds();
        let near_mask = game
            .world
            .entity(near)
            .unwrap()
            .collision_mask()
            .point_bounds();
        let far_mask = game
            .world
            .entity(far)
            .unwrap()
            .collision_mask()
            .point_bounds();
        assert!(mask_distance(player, far_mask) < mask_distance(player, near_mask));
        assert_sources(&mut game);
        assert_eq!(
            nearest_sources(&game, BEFORE_PLAYER)[2].unwrap().mask,
            near_mask
        );
    }
    #[test]
    fn indexed_sampling_matches_original_scalars_in_bright_and_dark_rooms() {
        for dark in [false, true] {
            for animation in [0., 2.25, 4.5] {
                let mut game = room();
                game.world.dark = dark;
                for (index, family) in FAMILIES.iter().enumerate() {
                    let id = game
                        .world
                        .spawn(*family, Vec2::new(83.25 + index as f64 * 3.5, 78.5));
                    let entity = game.world.entity_mut(id).unwrap();
                    entity.animation = animation;
                    entity.data.insert("dist_to_player".into(), 18.5);
                }
                assert_sources(&mut game);
                let hash = game.state_hash();
                assert_sources(&mut game);
                assert_eq!(
                    game.state_hash(),
                    hash,
                    "queries changed authoritative state"
                );
                let player = game.player.collision_mask().point_bounds();
                let source = original_nearest(&game, "oExplosion").unwrap();
                let mut expected = mask_distance(player, source.mask);
                if source.animation <= 3. {
                    expected -= source.animation * 16.;
                } else {
                    expected += (source.animation - 3.) * 16.;
                }
                for (family, penalty) in [
                    ("oLava", 0.),
                    ("oLamp", 0.),
                    ("oLampItem", 0.),
                    ("oFlareCrate", 0.),
                    ("oTikiTorch", 48.),
                    ("oArrowTrapLeftLit", 48.),
                    ("oArrowTrapRightLit", 48.),
                    ("oSpearTrapLit", 48.),
                    ("oSmashTrapLit", 48.),
                    ("oShotgunBlastLeft", 0.),
                    ("oShotgunBlastRight", 0.),
                ] {
                    if let Some(source) = original_nearest(&game, family) {
                        expected = expected.min(mask_distance(player, source.mask) + penalty);
                    }
                }
                sample_light_distances(&mut game);
                assert_eq!(game.player.data["dist_to_nearest_light_source"], expected);
                game.data.insert("darkness_lerp".into(), 0.7);
                let mut original = game.clone();
                if dark {
                    let mut distance = original_nearest(&original, "oFlare")
                        .map_or(160., |source| source.flare_distance);
                    if expected < 200. && expected < distance {
                        distance = expected;
                    }
                    let mut darkness = if distance == 0. { 0. } else { distance / 160. };
                    if original.data["darkness_lerp"] > 0. {
                        darkness = original.data["darkness_lerp"];
                        original.data.insert("darkness_lerp".into(), darkness - 0.1);
                    }
                    original.data.insert("darkness".into(), darkness.min(0.9));
                }
                lighting_tick(&mut game);
                assert_eq!(game.state_hash(), original.state_hash());
            }
        }
    }
    #[test]
    fn later_flare_selection_uses_new_player_position_and_existing_flare_cache() {
        let mut game = room();
        game.world.dark = true;
        for (x, cached) in [(40., 18.), (120., 64.)] {
            let id = game.world.spawn("oFlare", Vec2::new(x, 80.));
            game.world
                .entity_mut(id)
                .unwrap()
                .data
                .insert("dist_to_player".into(), cached);
        }
        game.player.pos.x = 48.;
        crate::dispatcher::activate_object(&mut game, "oDrawnSprite");
        sample_light_distances(&mut game);
        assert_eq!(game.player.data["dist_to_nearest_light_source"], 999.);
        game.player.pos.x = 112.;
        assert_sources(&mut game);
        lighting_tick(&mut game);
        assert_eq!(game.data["darkness"], 64. / 160.);
        assert_eq!(game.player.data["dist_to_nearest_light_source"], 999.);
        game.world.dark = false;
        game.data.insert("darkness".into(), 0.7);
        lighting_tick(&mut game);
        assert_eq!(game.data["darkness"], 0.7);
    }
    #[test]
    fn actual_walk_reactivation_keeps_nearest_lamp_independent_of_storage_order() {
        fn walking_room() -> Game {
            let mut game = Game::shell(123, RulesPreset::Classic);
            game.world = World::new(80, 30, Area::Mines, 1);
            game.world.dark = true;
            for x in 0..80 {
                game.world.set_tile(x, 10, "oBlock");
            }
            game.player.pos = Vec2::new(700., 152.);
            game.player.previous_pos = game.player.pos;
            game.player.state = crate::PlayerState::Standing;
            game.player.grounded = true;
            game.camera = Vec2::new(508., 8.);
            game
        }
        let input = crate::InputFrame {
            left: true,
            run: true,
            ..Default::default()
        };
        let mut outcomes = Vec::new();
        for left_created_first in [true, false] {
            let mut game = walking_room();
            let left = Vec2::new(247., 152.);
            let right = Vec2::new(547., 152.);
            let (left_id, right_id) = if left_created_first {
                (
                    game.world.spawn("oLamp", left),
                    game.world.spawn("oLamp", right),
                )
            } else {
                let right_id = game.world.spawn("oLamp", right);
                (game.world.spawn("oLamp", left), right_id)
            };
            for _ in 0..53 {
                game.step(&input);
            }
            assert_eq!(game.player.pos, Vec2::new(397., 152.));
            assert_eq!(
                crate::dispatcher::descendant_instances(&game, "oLamp"),
                vec![
                    crate::dispatcher::Instance::Entity(right_id),
                    crate::dispatcher::Instance::Entity(left_id)
                ]
            );
            assert_eq!(
                game.world
                    .entities
                    .iter()
                    .find(|e| e.object == "oLamp")
                    .unwrap()
                    .id,
                if left_created_first {
                    left_id
                } else {
                    right_id
                }
            );
            game.step(&crate::InputFrame::default());
            // Source Player319 samples nearest origin before this movement;
            // Level553 consumes its cache later. The right Lamp wins the tie.
            assert_eq!(game.player.pos, Vec2::new(394., 152.));
            // Before this move, sRunLeft's aggregate right edge is
            // 397+14-8=403; right sLamp's aggregate left is547+1=548.
            assert_eq!(game.player.data["dist_to_nearest_light_source"], 145.);
            assert_eq!(game.data["darkness"], 0.9);
            outcomes.push((
                game.player.data["dist_to_nearest_light_source"],
                game.data["darkness"],
            ));
        }
        assert_eq!(outcomes[0], outcomes[1]);
    }
}
