//! Source lighting queries share one ordered scan of the visible room.
use super::mask_distance;
use crate::{Game, Rect, Vec2, catalog};
use std::{collections::BTreeMap, sync::OnceLock};

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
                mask: game.world.tile_bounds(x as i32, y as i32),
                animation,
                flare_distance: 0.,
            }
        }
        SourceRef::Entity(index) => {
            let entity = &game.world.entities[index];
            LightSource {
                mask: entity.sprite_bounds(),
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
        let distance = position.distance(game.player.pos);
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
    // Keep each family's original tile-first, then entity creation order.
    for (index, tile) in game.world.tiles.iter().enumerate() {
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
    for (index, entity) in game.world.entities.iter().enumerate() {
        if entity.alive {
            consider(
                family_bits(&entity.object) & requested,
                entity.pos,
                SourceRef::Entity(index),
            );
        }
    }
    sources.map(|source| source.map(|source| resolve(game, source)))
}

/// oPlayer1 samples all twelve source families before character movement.
pub(crate) fn sample_light_distances(game: &mut Game) {
    let player = game.player.sprite_bounds();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Area, RulesPreset, World};

    // This is the original independent per-family selector, retained as the
    // equivalence oracle rather than sharing the indexed scan's machinery.
    fn original_nearest(game: &Game, family: &str) -> Option<LightSource> {
        let mut nearest = None;
        let mut distance = f64::INFINITY;
        for (index, tile) in game.world.tiles.iter().enumerate() {
            if !catalog::is_a(&tile.object, family) {
                continue;
            }
            let x = index % game.world.width;
            let y = index / game.world.width;
            let candidate_distance =
                Vec2::new(x as f64 * 16., y as f64 * 16.).distance(game.player.pos);
            if candidate_distance < distance {
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
                nearest = Some(LightSource {
                    mask: game.world.tile_bounds(x as i32, y as i32),
                    animation,
                    flare_distance: 0.,
                });
                distance = candidate_distance;
            }
        }
        for entity in game
            .world
            .entities
            .iter()
            .filter(|entity| entity.alive && catalog::is_a(&entity.object, family))
        {
            let candidate_distance = entity.pos.distance(game.player.pos);
            if candidate_distance < distance {
                nearest = Some(LightSource {
                    mask: entity.sprite_bounds(),
                    animation: entity.animation,
                    flare_distance: entity.data.get("dist_to_player").copied().unwrap_or(0.),
                });
                distance = candidate_distance;
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
    fn assert_sources(game: &Game) {
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
                assert_sources(&game);
            }
            count += 1;
        }
        assert!(count >= FAMILIES.len());
        assert_eq!(family_bits("not-a-bundled-object"), 0);
    }
    #[test]
    fn tile_and_entity_ties_keep_tile_order_and_first_metadata_animation() {
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
        assert_sources(&game);
        let source = nearest_sources(&game, BEFORE_PLAYER)[0].unwrap();
        assert_eq!(source.animation, 2.25);
        assert_eq!(source.mask, game.world.tile_bounds(5, 5));
        game.world.entity_mut(metadata).unwrap().alive = false;
        assert_sources(&game);
        assert_eq!(
            nearest_sources(&game, BEFORE_PLAYER)[0].unwrap().animation,
            4.5
        );
        game.world.set_tile(5, 5, "oLampRed");
        assert_sources(&game);
    }
    #[test]
    fn nearest_origin_precedes_mask_distance_and_membership_never_freezes_instances() {
        let mut game = room();
        let first = game.world.spawn("oLamp", Vec2::new(88., 80.));
        let second = game.world.spawn("oLampRed", Vec2::new(88., 80.));
        game.world.entity_mut(first).unwrap().sprite = "sExplosion".into();
        game.world.entity_mut(first).unwrap().animation = 3.;
        assert_sources(&game);
        assert_eq!(
            nearest_sources(&game, BEFORE_PLAYER)[2].unwrap().animation,
            3.
        );
        game.world.entity_mut(first).unwrap().alive = false;
        assert_sources(&game);
        game.world.entity_mut(second).unwrap().pos = Vec2::new(300., 300.);
        game.world.spawn("oLampRed", Vec2::new(82.5, 81.25));
        assert_sources(&game);
        game.world.entities.retain(|entity| entity.id != second);
        assert_sources(&game);
    }
    #[test]
    fn closer_origin_wins_even_when_farther_sprite_mask_is_closer() {
        let mut game = room();
        let near = game.world.spawn("oLamp", Vec2::new(100., 80.));
        let far = game.world.spawn("oLampRed", Vec2::new(112., 80.));
        game.world.entity_mut(far).unwrap().sprite = "sExplosion".into();
        let player = game.player.sprite_bounds();
        let near_mask = game.world.entity(near).unwrap().sprite_bounds();
        let far_mask = game.world.entity(far).unwrap().sprite_bounds();
        assert!(mask_distance(player, far_mask) < mask_distance(player, near_mask));
        assert_sources(&game);
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
                let hash = game.state_hash();
                assert_sources(&game);
                assert_eq!(
                    game.state_hash(),
                    hash,
                    "queries changed authoritative state"
                );
                let player = game.player.sprite_bounds();
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
        sample_light_distances(&mut game);
        assert_eq!(game.player.data["dist_to_nearest_light_source"], 999.);
        game.player.pos.x = 112.;
        assert_sources(&game);
        lighting_tick(&mut game);
        assert_eq!(game.data["darkness"], 64. / 160.);
        assert_eq!(game.player.data["dist_to_nearest_light_source"], 999.);
        game.world.dark = false;
        game.data.insert("darkness".into(), 0.7);
        lighting_tick(&mut game);
        assert_eq!(game.data["darkness"], 0.7);
    }
}
