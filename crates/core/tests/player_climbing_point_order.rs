//! UNRUN ordinary-input authored-overlap regression proposal, not a stock seed.
//! The only writes configure initial actors; later observations never mutate them.
use spelunky_core::dispatcher::{self, Instance};
use spelunky_core::{
    Area, CollisionPrecision, Game, GenerationRevision, InputFrame, PlayerState, RulesPreset, Vec2,
    World,
};

fn setup(rules: RulesPreset, entity_first: bool, historical: bool) -> (Game, u64, usize) {
    let mut game = if historical {
        Game::legacy_shell(42, rules)
    } else {
        Game::shell(42, rules)
    };
    assert_eq!(
        game.generation_revision,
        if historical {
            GenerationRevision::HistoricalV1
        } else {
            GenerationRevision::SourceClassicV2
        }
    );
    game.world = World::new(20, 15, Area::Mines, 1);
    game.world.name = "rLevel".into();
    for x in 0..20 {
        game.world.set_tile(x, 10, "oHardBlock");
    }
    let entity;
    if entity_first {
        entity = game.spawn("oLadderOrange", Vec2::new(82., 144.));
        game.world.set_tile(5, 9, "oLadderOrange");
    } else {
        game.world.set_tile(5, 9, "oLadderOrange");
        entity = game.spawn("oLadderOrange", Vec2::new(82., 144.));
    }
    let tile = 9 * game.world.width + 5;
    let entity_serial = game.world.entity(entity).unwrap().creation_serial;
    let tile_serial = game.world.tiles[tile].creation_serial;
    assert_eq!(entity_serial < tile_serial, entity_first);
    game.player.pos = Vec2::new(89., 152.);
    game.player.previous_pos = game.player.pos;
    (game, entity, tile)
}

fn point_receiver_guards(game: &Game, entity: u64, tile: usize, entity_first: bool) {
    let expected = if entity_first {
        vec![Instance::Entity(entity), Instance::Terrain(tile)]
    } else {
        vec![Instance::Terrain(tile), Instance::Entity(entity)]
    };
    assert_eq!(
        dispatcher::descendant_instances(game, "oLadder"),
        expected,
        "actual current source-family registration order"
    );
    for instance in expected {
        assert!(dispatcher::is_active(game, instance));
    }
    let ladder = game.world.entity(entity).expect("live original ladder");
    assert!(ladder.engine_active);
    assert!(game.world.tiles[tile].engine_active);
    // These are the existing native cached-bbox point predicates, not smooth
    // sprite rectangles. Both masks contain the actual queried origin, so the
    // first family member must determine the receiver's x+8.
    assert!(ladder.mask_contains(game.player.pos, CollisionPrecision::BoundingBox));
    assert!(game.world.tiles_point_precision(
        game.player.pos,
        "oLadder",
        CollisionPrecision::BoundingBox
    ));
}

fn healthy_step(game: &mut Game, input: InputFrame) {
    game.step(&input);
    assert_eq!(game.player.health, 4.0);
    assert!(!game.player.is_stunned());
}

fn ordinary_attachment_then_quiet_climb(rules: RulesPreset, entity_first: bool, historical: bool) {
    let (mut game, entity, tile) = setup(rules, entity_first, historical);
    healthy_step(&mut game, InputFrame::default());
    assert_eq!(game.player.state, PlayerState::Standing);
    assert!(game.player.grounded);
    assert_eq!(game.player.pos, Vec2::new(89., 152.));
    point_receiver_guards(&game, entity, tile, entity_first);

    healthy_step(
        &mut game,
        InputFrame {
            up: true,
            ..Default::default()
        },
    );
    let expected = Vec2::new(if entity_first { 90. } else { 88. }, 152.);
    assert_eq!(game.player.state, PlayerState::Climbing);
    assert_eq!(
        game.player.pos, expected,
        "real ordered instance_place attachment"
    );
    assert_eq!(game.player.vel, Vec2::default());
    point_receiver_guards(&game, entity, tile, entity_first);

    healthy_step(&mut game, InputFrame::default());
    let quiet_position = if historical {
        // Compatibility scope: historical climbing retains the tile-first
        // adapter even when its existing attachment chose an earlier entity.
        Vec2::new(88., 152.)
    } else {
        expected
    };
    point_receiver_guards(&game, entity, tile, entity_first);
    assert_eq!(game.player.state, PlayerState::Climbing);
    assert_eq!(game.player.vel, Vec2::default());
    assert_eq!(
        game.player.pos, quiet_position,
        "SOURCE_CLIMBING_POINT_RETAINS_FIRST_FAMILY_RECEIVER rules={rules:?} entity_first={entity_first} historical={historical}"
    );
    assert_eq!(
        game.player.previous_pos, expected,
        "previous_pos retains the actual quiet-frame start"
    );
}

#[test]
fn natural_entity_first_ladder_attachment_does_not_snap_to_later_terrain() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        ordinary_attachment_then_quiet_climb(rules, true, false);
    }
}

#[test]
fn natural_terrain_first_ladder_attachment_retains_the_original_tile_receiver() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        ordinary_attachment_then_quiet_climb(rules, false, false);
    }
}

#[test]
fn historical_mixed_ladder_birth_orders_retain_tile_first_climbing() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for entity_first in [true, false] {
            ordinary_attachment_then_quiet_climb(rules, entity_first, true);
        }
    }
}
