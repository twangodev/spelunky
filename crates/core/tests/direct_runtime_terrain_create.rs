//! Controlled public Game::spawn births followed by ordinary Game::step.
//! Source revision is explicit; no generated-campaign or native seed claim.
use spelunky_core::*;

fn modes() -> impl Iterator<Item = (RulesPreset, RngAlgorithm)> {
    [RulesPreset::Classic, RulesPreset::Assisted]
        .into_iter()
        .flat_map(|preset| {
            [RngAlgorithm::Gm8Lcg32V1, RngAlgorithm::LegacySplitMix64V1]
                .into_iter()
                .map(move |algorithm| (preset, algorithm))
        })
}

fn room(
    seed: u64,
    preset: RulesPreset,
    algorithm: RngAlgorithm,
    revision: GenerationRevision,
) -> Game {
    let mut game = Game::shell_with_algorithm(seed, preset, algorithm).unwrap();
    game.generation_revision = revision;
    game.world = World::new(24, 16, Area::Mines, 1);
    game.world.name = "rLevel".into();
    game.player.pos = Vec2::new(32.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.world.entrance = game.player.pos;
    for x in 0..24 {
        game.world.set_tile(x, 10, "oHardBlock");
    }
    game
}

fn same_rng(actual: &Rng, expected: &Rng) {
    assert_eq!(actual.clone().next_u64(), expected.clone().next_u64());
}

#[test]
fn direct_moai_family_inherits_defaults_and_survives_real_explosion_collision() {
    for (preset, algorithm) in modes() {
        for object in ["oMoai", "oMoai2", "oMoai3", "oMoaiInside"] {
            let mut game = room(7, preset, algorithm, GenerationRevision::SourceClassicV2);
            let before = game.rng.clone();
            let pos = Vec2::new(160.0, 80.0);
            let id = game.spawn(object, pos);
            let moai = game.world.entity(id).unwrap();
            assert_eq!(moai.value("invincible"), 1.0);
            assert_eq!(moai.data.get("blinkToggle"), Some(&0.0));
            assert_eq!(moai.data.get("shopWall"), Some(&0.0));
            assert_eq!(moai.data.get("cleanDeath"), Some(&0.0));
            assert_eq!(moai.text.get("type").map(String::as_str), Some("None"));
            assert_eq!(moai.text.get("treasure").map(String::as_str), Some(""));
            same_rng(&game.rng, &before);
            let blast = game.spawn("oExplosion", Vec2::new(168.0, 88.0));
            assert!(
                game.world
                    .entity(id)
                    .unwrap()
                    .event_intersects(game.world.entity(blast).unwrap())
            );
            game.step(&InputFrame::default());
            assert_eq!(game.world.entity(id).unwrap().pos, pos);
            assert!(game.world.entity(blast).is_some());
            assert_eq!(game.player.health, 4);
        }
    }
}

#[test]
fn direct_ice_block_has_moveable_bounds_and_literal_city_sprite() {
    for (preset, algorithm) in modes() {
        for context in ["ordinary", "cityOfGold", "city_of_gold", "City of Gold"] {
            let mut game = room(7, preset, algorithm, GenerationRevision::SourceClassicV2);
            if context == "City of Gold" {
                game.world.name = context.into();
            } else if context != "ordinary" {
                game.flags.insert(context.into());
            }
            let before = game.rng.clone();
            let id = game.spawn("oIceBlock", Vec2::new(80.0, 64.0));
            let ice = game.world.entity(id).unwrap();
            assert_eq!(
                ice.sprite,
                if context == "ordinary" {
                    "sIceBlock"
                } else {
                    "sGoldBlock"
                }
            );
            assert_eq!(ice.vel, Vec2::default());
            assert_eq!(ice.data.get("push_gravity"), Some(&0.6));
            assert_eq!(ice.bounds(), Rect::new(80.0, 64.0, 16.0, 16.0));
            assert_eq!(ice.data.get("invincible"), Some(&0.0));
            assert_eq!(ice.data.get("cleanDeath"), Some(&0.0));
            assert_eq!(ice.text.get("type").map(String::as_str), Some("None"));
            same_rng(&game.rng, &before);
        }
    }
}

#[test]
fn direct_cemetery_branch_create_prevents_leaf_and_rng_on_unsupported_step() {
    for (preset, algorithm) in modes() {
        for flag in ["cemetary", "cemetery"] {
            let mut game = room(7, preset, algorithm, GenerationRevision::SourceClassicV2);
            game.flags.insert(flag.into());
            let before = game.rng.clone();
            let id = game.spawn("oTreeBranch", Vec2::new(160.0, 80.0));
            let branch = game.world.entity(id).unwrap();
            assert_eq!(branch.sprite, "sTreeBranchDeadR");
            assert_eq!(branch.data.get("blinkToggle"), Some(&0.0));
            assert_eq!(branch.text.get("type").map(String::as_str), Some(""));
            same_rng(&game.rng, &before);
            game.step(&InputFrame::default());
            assert!(
                game.world.entity(id).is_none(),
                "ordinary unsupported branch Step ran"
            );
            assert!(!game.world.entities.iter().any(|e| e.object == "oLeaf"));
            same_rng(&game.rng, &before);
        }
    }
}

#[test]
fn direct_ice_runs_one_source_draw_and_creates_real_frozen_child_at_parent_origin() {
    for (preset, algorithm) in modes() {
        let mut hits = 0;
        let mut misses = 0;
        for seed in 0..256 {
            let mut game = room(seed, preset, algorithm, GenerationRevision::SourceClassicV2);
            let mut expected = game.rng.clone();
            let frozen_expected = expected.range(80) == 0;
            let pos = Vec2::new(160.0, 80.0);
            let first_id = game.world.next_id;
            let id = game.spawn("oIce", pos);
            assert_eq!(id, first_id);
            let ice = game.world.entity(id).unwrap();
            assert_eq!(ice.data.get("invincible"), Some(&0.0));
            assert_eq!(ice.data.get("shopWall"), Some(&0.0));
            assert_eq!(ice.data.get("cleanDeath"), Some(&0.0));
            assert_eq!(ice.text.get("type").map(String::as_str), Some("None"));
            let children: Vec<_> = game.world.entities.iter().filter(|e| e.id > id).collect();
            assert_eq!(children.len(), usize::from(frozen_expected));
            if frozen_expected {
                hits += 1;
                let child = children[0];
                assert_eq!(child.object, "oFrozenCaveman");
                assert!(child.alive);
                assert_eq!(child.pos, pos);
                assert_eq!(child.creation_serial, ice.creation_serial + 1);
                assert_eq!(child.data.get("blinkToggle"), Some(&0.0));
                assert_eq!(child.text.get("type").map(String::as_str), Some(""));
                assert!(
                    !child.data.contains_key("invincible"),
                    "Frozen inherits DrawnSprite only"
                );
            } else {
                misses += 1;
            }
            same_rng(&game.rng, &expected);
        }
        assert!(
            hits > 0 && misses > 0,
            "fixed source-prefix seed range covers both branches"
        );
    }
}

#[test]
fn historical_direct_births_keep_existing_fields_sprites_children_and_rng() {
    for (preset, algorithm) in modes() {
        let mut game = room(7, preset, algorithm, GenerationRevision::HistoricalV1);
        game.flags.insert("cityOfGold".into());
        game.flags.insert("cemetary".into());
        let before = game.rng.clone();
        for object in [
            "oMoai",
            "oMoai2",
            "oMoai3",
            "oMoaiInside",
            "oIceBlock",
            "oIce",
            "oTreeBranch",
            "oFrozenCaveman",
        ] {
            let count = game.world.entities.len();
            let id = game.spawn(object, Vec2::new(160.0, 80.0));
            let entity = game.world.entity(id).unwrap();
            assert_eq!(entity.sprite, catalog::sprite(object).unwrap());
            assert!(
                !entity
                    .data
                    .contains_key("runtime_terrain_create_initialized")
            );
            assert!(!entity.data.contains_key("blinkToggle"));
            assert!(!entity.data.contains_key("invincible"));
            assert!(!entity.text.contains_key("type"));
            assert_eq!(game.world.entities.len(), count + 1);
            same_rng(&game.rng, &before);
        }
    }
}
