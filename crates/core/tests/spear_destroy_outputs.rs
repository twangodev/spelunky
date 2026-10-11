use spelunky_core::{Game, InputFrame, RulesPreset, Vec2, catalog, level::Level};

#[derive(Clone, Copy, Debug)]
enum Storage {
    Entity,
    Terrain,
}

fn fixture(
    rules: RulesPreset,
    object: &str,
    storage: Storage,
    x: usize,
    supported: bool,
) -> (Game, Option<u64>) {
    let mut level = Level::default();
    level.set_tile(3, 9, '@').unwrap();
    level.set_tile(34, 9, 'X').unwrap();
    for column in 0..spelunky_core::level::LEVEL_WIDTH {
        level.set_tile(column, 10, '1').unwrap();
    }
    if supported {
        // Authored cells map to world cells with a one-tile border.
        level.set_tile(x - 1, 5, '1').unwrap();
    }
    let mut game = Game::shell(47, rules);
    game.load_custom(&level, "spear-own-destroy", "");
    let position = Vec2::new(x as f64 * 16.0, 80.0);
    let id = match storage {
        Storage::Entity => Some(game.spawn(object, position)),
        Storage::Terrain => {
            game.world.set_tile(x, 5, object);
            None
        }
    };
    assert_eq!(
        game.world.solid_at(position.x, position.y + 16.0),
        supported,
        "fixture must establish the literal source support query"
    );
    (game, id)
}

fn fragments(game: &Game) -> Vec<&spelunky_core::Entity> {
    let mut result: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|entity| catalog::is_a(&entity.object, "oRubblePiece"))
        .collect();
    result.sort_by_key(|entity| entity.id);
    result
}

fn exists(game: &Game, id: Option<u64>, x: usize) -> bool {
    id.map_or_else(
        || !game.world.tiles[5 * game.world.width + x].object.is_empty(),
        |id| game.world.entity(id).is_some(),
    )
}

fn initial_field(game: &mut Game, id: Option<u64>, x: usize, key: &str, value: f64) {
    let receiver = if let Some(id) = id {
        game.world.entity_mut(id).unwrap()
    } else {
        let position = Vec2::new(x as f64 * 16.0 + 8.0, 88.0);
        let metadata = game
            .world
            .entities
            .iter()
            .find(|entity| {
                entity.alive && entity.object == "oTerrainMetadata" && entity.pos == position
            })
            .map(|entity| entity.id)
            .unwrap_or_else(|| game.spawn("oTerrainMetadata", position));
        game.world.entity_mut(metadata).unwrap()
    };
    receiver.data.insert(key.into(), value);
}

#[test]
fn natural_unsupported_spears_emit_three_ordered_tan_fragments_and_twelve_draws() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for object in ["oSpearTrapTop", "oSpearTrapBottom", "oSpearTrapLit"] {
            for storage in [Storage::Entity, Storage::Terrain] {
                for invincible in [false, true] {
                    for shop_wall in [false, true] {
                        let (mut game, id) = fixture(rules, object, storage, 10, false);
                        if invincible {
                            // Initial attack-immunity control: own lost-support
                            // instance_destroy is independent from attack guards.
                            initial_field(&mut game, id, 10, "invincible", 1.0);
                        }
                        if shop_wall {
                            initial_field(&mut game, id, 10, "shopWall", 1.0);
                        }
                        let mut oracle = game.rng.clone();
                        let expected: Vec<_> = (0..3)
                            .map(|_| {
                                Vec2::new(
                                    168.0 + f64::from(oracle.range(9)) - f64::from(oracle.range(9)),
                                    88.0 + f64::from(oracle.range(9)) - f64::from(oracle.range(9)),
                                )
                            })
                            .collect();
                        game.step(&InputFrame::default());
                        assert!(!exists(&game, id, 10), "{object} {storage:?}");
                        assert!(
                            !game.flags.contains("shopkeepers_angry"),
                            "own Destroy never inherits shop anger: {object} {storage:?} shopWall={shop_wall}"
                        );
                        assert!(!game.flags.contains("wanted"));
                        let actual = fragments(&game);
                        assert_eq!(
                            actual.len(),
                            3,
                            "{object} {storage:?} invincible={invincible}"
                        );
                        for ((fragment, position), (kind, sprite)) in
                            actual.iter().zip(expected).zip([
                                ("oRubble", "sRubbleTan"),
                                ("oRubbleSmall", "sRubbleTanSmall"),
                                ("oRubbleSmall", "sRubbleTanSmall"),
                            ])
                        {
                            assert!(fragment.alive);
                            assert_eq!(fragment.object, kind);
                            assert_eq!(fragment.sprite, sprite);
                            // Each zero-velocity source Create fixes the birth
                            // position; a same-frame Step cannot move it yet.
                            assert_eq!(fragment.pos, position);
                            assert_eq!(fragment.vel.x, 0.0);
                            assert_eq!(fragment.value("gravity"), 0.6);
                        }
                        assert_eq!(game.rng.clone().next_u64(), oracle.next_u64());
                        let fragment_ids: Vec<_> = actual.iter().map(|entity| entity.id).collect();
                        game.step(&InputFrame::default());
                        assert_eq!(
                            fragments(&game)
                                .iter()
                                .map(|entity| entity.id)
                                .collect::<Vec<_>>(),
                            fragment_ids,
                            "Destroy runs once"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn natural_clean_spear_destruction_emits_no_fragments_and_consumes_no_rng() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for object in ["oSpearTrapTop", "oSpearTrapBottom", "oSpearTrapLit"] {
            for storage in [Storage::Entity, Storage::Terrain] {
                for clean in ["cleanDeath", "flag", "data"] {
                    let (mut game, id) = fixture(rules, object, storage, 10, false);
                    // Real terrain metadata must not add parent shop anger to
                    // this own non-inheriting callback, even on clean death.
                    if matches!(storage, Storage::Terrain) {
                        initial_field(&mut game, id, 10, "shopWall", 1.0);
                    }
                    match clean {
                        "cleanDeath" => initial_field(&mut game, id, 10, "cleanDeath", 1.0),
                        "flag" => {
                            game.flags.insert("cleanSolids".into());
                        }
                        _ => {
                            game.data.insert("cleanSolids".into(), 1.0);
                        }
                    }
                    let mut oracle = game.rng.clone();
                    game.step(&InputFrame::default());
                    assert!(!exists(&game, id, 10), "{object} {storage:?} {clean}");
                    assert!(!game.flags.contains("shopkeepers_angry"));
                    assert!(!game.flags.contains("wanted"));
                    assert!(fragments(&game).is_empty());
                    assert_eq!(game.rng.next_u64(), oracle.next_u64());
                }
            }
        }
    }
}

#[test]
fn supported_and_offscreen_unsupported_spears_remain_without_destroy_outputs() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for object in ["oSpearTrapTop", "oSpearTrapBottom", "oSpearTrapLit"] {
            for storage in [Storage::Entity, Storage::Terrain] {
                for (x, supported) in [(10, true), (25, false)] {
                    let (mut game, id) = fixture(rules, object, storage, x, supported);
                    let mut oracle = game.rng.clone();
                    game.step(&InputFrame::default());
                    assert!(
                        exists(&game, id, x),
                        "{object} {storage:?} supported={supported}"
                    );
                    assert!(fragments(&game).is_empty());
                    assert_eq!(game.rng.next_u64(), oracle.next_u64());
                }
            }
        }
    }
}
