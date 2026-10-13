//! Source Boulder Collision with oSolid33–37 deletes one raw layer3 tile
//! before spikes and the target Destroy. These are configured initial rooms,
//! not a claim about an input-only generated Mines/Idol encounter. The Boulder
//! keeps its default Create: two neutral Steps produce its first landing roll
//! and the automatic Solid collision. No actor writes follow the first Step.
use spelunky_core::*;
use std::collections::BTreeSet;

fn modes() -> impl Iterator<Item = (RulesPreset, RngAlgorithm)> {
    [RulesPreset::Classic, RulesPreset::Assisted]
        .into_iter()
        .flat_map(|rules| {
            [RngAlgorithm::Gm8Lcg32V1, RngAlgorithm::LegacySplitMix64V1]
                .into_iter()
                .map(move |algorithm| (rules, algorithm))
        })
}

fn source_tile(game: &mut Game, point: Vec2, depth: f64) -> u64 {
    let id = game.spawn("oGenerationDecoration", point);
    let tile = game.world.entity_mut(id).unwrap();
    tile.set_sprite("bgCaveTop");
    for (key, value) in [
        ("decoration", 1.0),
        ("source_x", 0.0),
        ("source_y", 0.0),
        ("source_w", 16.0),
        ("source_h", 16.0),
        ("depth", depth),
    ] {
        tile.data.insert(key.into(), value);
    }
    id
}

fn landing_scene(game: &mut Game, target: Vec2) -> u64 {
    // Source Create sets bounds(-14,-16,14,16), zero xVel/yVel,
    // bounced=false, invincible=true and viscidTop=1. Author only the room:
    // a floor, remote Player to the right, and a nonoverlapping birth origin.
    // Own Step59–79 supplies xVel=4.5 on first contact with that floor.
    let column = (target.x / 16.0) as usize;
    let row = (target.y / 16.0) as usize;
    for x in column - 3..=column + 6 {
        game.world.set_tile(x, row + 1, "oHardBlock");
    }
    game.player.pos = Vec2::new(target.x + 64.0, target.y + 8.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.world.entrance = game.player.pos;
    game.set_simulation_view_extent(
        game.world.width as f64 * 16.0,
        game.world.height as f64 * 16.0,
    )
    .unwrap();
    let boulder = game.spawn("oBoulder", Vec2::new(target.x - 16.0, target.y));
    let rock = game.world.entity(boulder).unwrap();
    assert_eq!(rock.vel, Vec2::default());
    assert_eq!(rock.value("bounced"), 0.0);
    assert_eq!(rock.value("invincible"), 1.0);
    assert_eq!(rock.value("viscidTop"), 1.0);
    assert_eq!(rock.sprite, "sBoulder");
    boulder
}

fn first_landing(game: &mut Game, boulder: u64) {
    game.step(&InputFrame::default());
    let rock = game.world.entity(boulder).unwrap();
    assert_eq!(rock.value("bounced"), 1.0);
    assert_eq!(rock.vel.x, 4.5, "source-default landing supplies the roll");
    assert_eq!(rock.vel.y, 0.0);
    assert_eq!(rock.sprite, "sBoulderRotateR");
    assert!(
        !game
            .events
            .iter()
            .any(|event| matches!(event, GameEvent::Sound { name, .. } if name == "sndCrunch"))
    );
}

fn generated_fixture(rules: RulesPreset, algorithm: RngAlgorithm) -> (Game, u64, usize, u64) {
    // Selection depends only on actual generated source-default records.
    // Keep the selected original Brick, metadata and fringe unchanged. Other
    // initial terrain/actors are cleared to isolate its first collision;
    // landing support/Player/Boulder are authored before the first Step.
    for seed in 0..8 {
        let mut game = Game::shell_with_algorithm(seed, rules, algorithm).unwrap();
        game.world = generation::generate(&mut game.rng, 1, &BTreeSet::new());
        let selected = game.world.entities.iter().find_map(|tile| {
            if tile.object != "oGenerationDecoration"
                || tile.sprite != "bgCaveTop"
                || tile.value("depth") != 3.0
                || tile.value("source_w") != 16.0
                || tile.value("source_h") != 16.0
                || !(80.0..480.0).contains(&tile.pos.x)
                || !(64.0..384.0).contains(&tile.pos.y)
            {
                return None;
            }
            let origin = Vec2::new(tile.pos.x, tile.pos.y + 16.0);
            let index = (origin.y / 16.0) as usize * game.world.width + (origin.x / 16.0) as usize;
            (game.world.tiles[index].object == "oBrick").then_some((tile.id, index, origin))
        });
        let Some((tile, index, origin)) = selected else {
            continue;
        };
        game.world.entities.retain(|entity| {
            entity.object == "oGenerationDecoration"
                || (entity.object == "oTerrainMetadata"
                    && entity.pos == Vec2::new(origin.x + 8.0, origin.y + 8.0))
        });
        for (i, terrain) in game.world.tiles.iter_mut().enumerate() {
            if i != index {
                *terrain = Tile::default();
            }
        }
        let boulder = landing_scene(&mut game, origin);
        return (game, tile, index, boulder);
    }
    panic!("no generated default Mines fringe in seeds0..8 for {rules:?}/{algorithm:?}");
}

#[test]
fn default_boulder_landing_roll_removes_actual_generated_fringe_and_its_brick() {
    for (rules, algorithm) in modes() {
        let (mut game, tile, index, boulder) = generated_fixture(rules, algorithm);
        let original = game.world.entity(tile).unwrap();
        assert_eq!(original.sprite, "bgCaveTop");
        assert_eq!(original.value("depth"), 3.0);
        assert_eq!(
            (original.value("source_w"), original.value("source_h")),
            (16.0, 16.0)
        );
        assert_eq!(
            original.sprite_bounds().w,
            0.0,
            "background is not a sprite mask"
        );
        first_landing(&mut game, boulder);
        assert_eq!(game.world.tiles[index].object, "oBrick");
        assert!(game.world.entity(tile).is_some());
        game.step(&InputFrame::default());
        assert!(
            game.world.tiles[index].object.is_empty(),
            "actual collision destroys Brick"
        );
        assert!(
            game.events
                .iter()
                .any(|event| matches!(event, GameEvent::Sound { name, .. } if name == "sndCrunch"))
        );
        assert!(
            game.world.entity(tile).is_none(),
            "BOULDER_MUST_DELETE_GENERATED_LAYER3_FRINGE"
        );
        assert!(
            game.world.entities.iter().any(|entity| {
                entity.alive
                    && entity.object == "oGenerationDecoration"
                    && entity.value("depth") == 3.0
            }),
            "one crushed Brick must not erase the layer"
        );
    }
}

#[test]
fn actual_boulder_collision_deletes_first_point_tile_and_preserves_overlap_depth_edges_and_actor() {
    for (rules, algorithm) in modes() {
        for terrain in [false, true] {
            let mut game = Game::shell_with_algorithm(42, rules, algorithm).unwrap();
            let origin = Vec2::new(192.0, 80.0);
            let target = if terrain {
                game.world.set_tile(12, 5, "oBrickSmooth");
                None
            } else {
                Some(game.spawn("oBrickSmooth", origin))
            };
            let point = Vec2::new(origin.x, origin.y - 16.0);
            let first = source_tile(&mut game, point, 3.0);
            let overlap = source_tile(&mut game, point, 3.0);
            let wrong_depth = source_tile(&mut game, point, 2.0);
            let outside_y = source_tile(&mut game, Vec2::new(point.x, point.y + 1.0), 3.0);
            let right_edge = source_tile(&mut game, Vec2::new(point.x - 16.0, point.y), 3.0);
            let remote = source_tile(&mut game, Vec2::new(point.x + 64.0, point.y), 3.0);
            let actor = game.spawn("oMsgSign", point);
            game.world
                .entity_mut(actor)
                .unwrap()
                .data
                .insert("depth".into(), 3.0);
            let boulder = landing_scene(&mut game, origin);
            first_landing(&mut game, boulder);
            assert!(
                game.world.entity(first).is_some(),
                "no premature tile deletion"
            );
            game.step(&InputFrame::default());
            if let Some(target) = target {
                assert!(game.world.entity(target).is_none());
            } else {
                assert!(game.world.tile(12, 5).unwrap().object.is_empty());
            }
            assert!(game.world.entity(boulder).unwrap().vel.x > 1.0);
            assert!(game.events.iter().any(
                |event| matches!(event, GameEvent::Sound { name, .. } if name == "sndCrunch")
            ));
            assert!(
                game.world.entity(first).is_none(),
                "BOULDER_MUST_DELETE_FIRST_POINT_LAYER3_TILE"
            );
            for id in [overlap, wrong_depth, outside_y, right_edge, remote, actor] {
                assert!(
                    game.world.entity(id).is_some(),
                    "must retain separate tile/actor control {id}"
                );
            }
        }
    }
}
