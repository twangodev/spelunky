use spelunky_core::{Game, InputFrame, RulesPreset, Vec2, level::Level};

#[test]
fn custom_terrain_retains_hidden_create_contents_and_releases_them_on_destruction() {
    let mut level = Level::default();
    for y in 4..20 {
        for x in 4..20 {
            level.set_tile(x, y, '3').unwrap();
        }
    }
    let mut game = Game::new(42, RulesPreset::Classic);
    game.load_custom(&level, "buried.lvl", "");
    let metadata = game
        .world
        .entities
        .iter()
        .find(|entity| {
            entity.object == "oTerrainMetadata" && entity.value("embedded_treasure") != 0.0
        })
        .expect("custom terrain Create events must retain buried contents")
        .clone();
    for entity in game.world.entities.iter().filter(|entity| {
        entity.object == "oTerrainMetadata" && entity.value("terrain_initialized") != 0.0
    }) {
        assert_eq!(entity.text["terrain_sprite"], "sDark");
        assert_eq!(entity.value("gold_chunks"), 0.0);
        assert_eq!(entity.value("frozen_caveman"), 0.0);
    }
    let treasure = metadata.text["treasure"].clone();
    let x = (metadata.pos.x / 16.0) as i32;
    let y = (metadata.pos.y / 16.0) as i32;
    assert_eq!(game.world.tile(x, y).unwrap().object, "oDark");
    let bomb = game.world.spawn("oBomb", metadata.pos);
    let bomb = game.world.entity_mut(bomb).unwrap();
    bomb.data.insert("armed".into(), 1.0);
    bomb.timer = 1;
    game.step(&InputFrame::default());
    assert!(game.world.tile(x, y).unwrap().object.is_empty());
    assert!(
        game.world
            .entities
            .iter()
            .any(|entity| entity.alive && entity.object == treasure)
    );
}

#[test]
fn custom_water_and_lava_keep_create_defaults_without_generated_setup() {
    let mut level = Level::default();
    level.set_tile(4, 4, 'w').unwrap();
    level.set_tile(8, 4, 'l').unwrap();
    let mut game = Game::new(42, RulesPreset::Classic);
    game.load_custom(&level, "fluids.lvl", "");
    assert_eq!(
        game.world.tile_bounds(5, 5),
        spelunky_core::catalog::bounds("oWaterSwim", Vec2::new(80.0, 80.0))
    );
    let lava = game
        .world
        .entities
        .iter()
        .find(|entity| entity.object == "oTerrainMetadata" && entity.pos == Vec2::new(152.0, 88.0))
        .unwrap();
    assert_eq!(lava.value("lava_initialized"), 1.0);
    assert_eq!(lava.value("spurt"), 0.0);
    assert_eq!(lava.value("lava_setup_initialized"), 0.0);
    assert!((100.0..=300.0).contains(&lava.value("spurt_time")));
    assert_eq!(lava.value("spurt_counter"), lava.value("spurt_time"));
}

#[test]
fn custom_ice_cleanup_releases_stunned_cavemen_without_frozen_overlays() {
    let mut level = Level::default();
    for y in 4..20 {
        for x in 4..20 {
            level.set_tile(x, y, 'i').unwrap();
        }
    }
    let mut game = Game::new(42, RulesPreset::Classic);
    game.load_custom(&level, "ice.lvl", "");
    let cavemen: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|entity| entity.object == "oCaveman")
        .collect();
    assert!(!cavemen.is_empty());
    for caveman in cavemen {
        assert_eq!(caveman.state, 98);
        assert_eq!(caveman.timer, 200);
        assert_eq!(caveman.value("invincible"), 20.0);
    }
    assert!(
        game.world.entities.iter().all(
            |entity| entity.object != "oFrozenCaveman" && entity.value("frozen_caveman") == 0.0
        )
    );
}

#[test]
fn overlapping_custom_terrain_also_receives_create_initialization() {
    let mut level = Level::default();
    level.set_tile(4, 4, '{').unwrap();
    level.set_tile(5, 4, '3').unwrap();
    level.set_tile(6, 4, '1').unwrap();
    let mut game = Game::new(42, RulesPreset::Classic);
    game.load_custom(&level, "overlap.lvl", "");
    let terrain: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|entity| matches!(entity.object.as_str(), "oDark" | "oBrick"))
        .collect();
    assert_eq!(terrain.len(), 2);
    for entity in terrain {
        assert_eq!(entity.value("terrain_initialized"), 1.0);
        assert_eq!(
            entity.sprite,
            if entity.object == "oDark" {
                "sDark"
            } else {
                "sBrick"
            }
        );
    }
}
