//! Internal weapons controls for the shared tile sprite metadata resolver.
//! Explicit raw metadata binding profile; no Source callbacks or Scene/native
//! lifecycle admission is supplied.
use super::*;

fn metadata(id: u64, animation: f64) -> Entity {
    let mut body = Entity::new(id, "oTerrainMetadata", Vec2::new(80., 80.));
    body.creation_serial = id;
    body.animation = animation;
    body.text.insert("terrain_sprite".into(), "sBlock".into());
    body
}
fn fixture(preferred: Option<u64>, metadata: Vec<Entity>) -> (Game, Entity) {
    let mut game = Game::shell(3, RulesPreset::Classic);
    game.generation_revision = crate::GenerationRevision::SourceClassicV2;
    game.world.set_tile(5, 5, "oBlock");
    let tile_index = 5 * game.world.width + 5;
    game.world.tiles[tile_index].source_metadata_id = preferred;
    // Deliberately supplied metadata allocations only, without spawn/Create.
    // Their host IDs and serials remain separate from the tile's stored origin.
    game.world.next_id = metadata.iter().map(|e| e.id).max().unwrap_or(0) + 1;
    game.world.creation_serial = game.world.creation_serial.max(
        metadata
            .iter()
            .map(|e| e.creation_serial)
            .max()
            .unwrap_or(0),
    );
    game.world.entities = metadata;
    let hit = Entity::new(9000, "oMattockHit", Vec2::new(88., 88.));
    assert!(hit.collision_mask().supports_source_query_pixels());
    (game, hit)
}

#[test]
fn preferred_valid_metadata_admits_precise_tile_target_despite_earlier_invalid_decoy() {
    let (mut game, mut hit) = fixture(Some(102), vec![metadata(101, f64::NAN), metadata(102, 0.)]);
    hit.pos.y = 72.; // First point misses; the +9 point and translated mask hit.
    assert_eq!(game.world.tile_sprite_metadata(5, 5).unwrap().id, 102);
    let target_mask = game.world.tile_collision_mask(5, 5);
    assert!(target_mask.supports_source_query_pixels());
    assert!(!target_mask.contains(hit.pos, CollisionPrecision::BoundingBox));
    assert!(!hit.collision_mask().intersects_mask(&target_mask));
    let mut translated = hit.clone();
    translated.pos.y += 9.;
    assert!(target_mask.contains(translated.pos, CollisionPrecision::BoundingBox));
    assert!(translated.collision_mask().intersects_mask(&target_mask));
    // Actual translated source target, rather than merely allowing the legacy
    // adapter to produce the same tile after an unsupported metadata frame.
    assert!(matches!(
        source_mattock_target(&mut game, &hit),
        Some(Some(DigTarget::Tile(5, 5)))
    ));
}

#[test]
fn preferred_invalid_metadata_retains_unsupported_domain_fallback_despite_valid_decoy() {
    for invalid in [f64::NAN, -1.] {
        let (mut game, hit) = fixture(Some(102), vec![metadata(101, 0.), metadata(102, invalid)]);
        assert_eq!(game.world.tile_sprite_metadata(5, 5).unwrap().id, 102);
        assert!(source_mattock_target(&mut game, &hit).is_none());
        assert!(matches!(
            legacy_mattock_target(&game, &hit),
            Some(DigTarget::Tile(5, 5))
        ));
        let selected = source_mattock_target(&mut game, &hit)
            .unwrap_or_else(|| legacy_mattock_target(&game, &hit));
        assert!(matches!(selected, Some(DigTarget::Tile(5, 5))));
    }
}

#[test]
fn unbound_metadata_uses_first_qualifying_coordinate_provider_and_keeps_its_invalid_frame() {
    let mut dead = metadata(101, f64::NAN);
    dead.alive = false;
    let mut wrong_object = metadata(102, f64::NAN);
    wrong_object.object = "oGoldChunk".into();
    let mut wrong_coordinate = metadata(103, f64::NAN);
    wrong_coordinate.pos.x = 96.;
    let mut missing_sprite = metadata(104, f64::NAN);
    missing_sprite.text.remove("terrain_sprite");
    let (mut game, hit) = fixture(
        None,
        vec![
            dead,
            wrong_object,
            wrong_coordinate,
            missing_sprite,
            metadata(105, 0.),
            metadata(106, f64::NAN),
        ],
    );
    assert_eq!(game.world.tile_sprite_metadata(5, 5).unwrap().id, 105);
    assert!(matches!(
        source_mattock_target(&mut game, &hit),
        Some(Some(DigTarget::Tile(5, 5)))
    ));
    // Metadata selection never searches ahead for a convenient valid frame.
    game.world.entities[4].animation = -1.;
    game.world.entities[5].animation = 0.;
    assert_eq!(game.world.tile_sprite_metadata(5, 5).unwrap().id, 105);
    assert!(source_mattock_target(&mut game, &hit).is_none());
    assert!(matches!(
        legacy_mattock_target(&game, &hit),
        Some(DigTarget::Tile(5, 5))
    ));
}

#[test]
fn missing_preferred_id_uses_resource_mask_without_selecting_nonmatching_coordinate_decoy() {
    let (mut game, hit) = fixture(Some(99999), vec![metadata(101, f64::NAN)]);
    assert!(game.world.tile_sprite_metadata(5, 5).is_none());
    assert!(game.world.tile_sprite_metadata(-1, 5).is_none());
    let resource = crate::catalog::instance_mask(
        "oBlock",
        "sBlock",
        Vec2::new(80., 80.),
        0.,
        Vec2::new(1., 1.),
        0.,
    );
    let actual = game.world.tile_collision_mask(5, 5);
    assert!(actual.supports_source_query_pixels());
    assert_eq!(actual.bounds(), resource.bounds());
    assert!(hit.collision_mask().intersects_mask(&actual));
    assert!(matches!(
        source_mattock_target(&mut game, &hit),
        Some(Some(DigTarget::Tile(5, 5)))
    ));
}
