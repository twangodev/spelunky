use spelunky_core::{Game, InputFrame, RulesPreset, Vec2, level::Level};

fn source_sapphire_seed(symbol: char, chance: u32) -> u32 {
    (0..100_000)
        .find(|seed| {
            let mut rng = spelunky_core::Rng::new(*seed);
            if symbol == '1' {
                rng.range(10);
            }
            let roll = rng.range(100) + 1;
            roll >= 30 && rng.range(chance) == 0
        })
        .expect("source Create selector must find a Sapphire fixture")
}

#[test]
fn custom_destructible_terrain_creates_source_loot_before_destruction() {
    for (symbol, object, chance) in [
        ('1', "oBrick", 100),
        ('2', "oLush", 80),
        ('3', "oDark", 40),
        ('4', "oTemple", 60),
    ] {
        let mut level = Level::default();
        level.set_tile(6, 6, symbol).unwrap();
        let mut game = Game::shell(source_sapphire_seed(symbol, chance), RulesPreset::Classic);
        game.load_custom(&level, "buried-custom-treasure.lvl", "");

        let tile = game.world.tile(7, 7).unwrap();
        assert_eq!(tile.object, object);
        let parent = game
            .world
            .entities
            .iter()
            .find(|entity| {
                entity.alive
                    && entity.object == "oTerrainMetadata"
                    && entity.value("terrain_initialized") != 0.0
                    && entity.pos == Vec2::new(120.0, 120.0)
            })
            .expect("custom terrain Create must initialize its source tile");
        let child = game
            .world
            .entities
            .iter()
            .find(|entity| entity.alive && entity.object == "oSapphireBig")
            .expect("Classic creates the selected buried gem immediately");
        let child_id = child.id;

        assert_eq!(child.pos, Vec2::new(120.0, 120.0));
        assert_eq!(child.pos, parent.pos);
        assert_eq!(parent.value("embedded_treasure"), 0.0);
        assert!(!parent.text.contains_key("treasure"));

        // The parent Destroy event must not materialize the already-live
        // Create child a second time. Explosion Create runs synchronously;
        // its first Game::step dispatches the terrain collision.
        let gem_allocations = game
            .world
            .entities
            .iter()
            .filter(|entity| entity.object == "oSapphireBig")
            .count();
        let explosion = game.spawn("oExplosion", parent.pos);
        assert_eq!(game.world.entity(explosion).unwrap().image_speed(), 0.8);
        game.step(&InputFrame::default());
        assert!(game.world.tile(7, 7).unwrap().object.is_empty());
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|entity| entity.object == "oSapphireBig")
                .count(),
            gem_allocations
        );
        assert!(
            game.world
                .entities
                .iter()
                .any(|entity| entity.id == child_id)
        );
    }
}

#[test]
fn custom_lush_and_temple_create_their_source_treasure_immediately() {
    for (symbol, object) in [('2', "oLush"), ('4', "oTemple")] {
        let mut level = Level::default();
        level.set_tile(6, 6, symbol).unwrap();
        let mut game = Game::shell(53, RulesPreset::Classic);
        game.load_custom(&level, "buried-custom-treasure.lvl", "");

        let parent = game
            .world
            .entities
            .iter()
            .find(|entity| {
                entity.alive
                    && entity.object == "oTerrainMetadata"
                    && entity.value("terrain_initialized") != 0.0
                    && game
                        .world
                        .tile(7, 7)
                        .is_some_and(|tile| tile.object == object)
            })
            .expect("custom terrain Create must initialize its source tile");
        let child = game
            .world
            .entities
            .iter()
            .find(|entity| entity.alive && entity.object == "oSapphireBig")
            .expect("Classic creates the selected buried gem immediately");

        assert_eq!(child.pos, parent.pos);
        assert_eq!(parent.value("embedded_treasure"), 0.0);
        assert!(!parent.text.contains_key("treasure"));
    }
}

#[test]
fn custom_water_and_lava_receive_source_wall_setup_after_create() {
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
    assert_eq!(lava.value("lava_setup_initialized"), 1.0);
    assert!(matches!(lava.value("spurt"), 0.0 | 1.0));
    assert!((100.0..=300.0).contains(&lava.value("spurt_time")));
    assert_eq!(lava.value("spurt_counter"), lava.value("spurt_time"));
}

#[test]
fn custom_authored_items_inherit_depth_from_current_spectacles_state() {
    for (has_spectacles, expected_depth) in [(false, 101.0), (true, 51.0)] {
        let mut level = Level::default();
        level.set_tile(6, 6, '.').unwrap();
        let mut game = Game::shell(42, RulesPreset::Classic);
        if has_spectacles {
            game.player.equipment.insert("Spectacles".into());
        }
        game.load_custom(&level, "authored-free-item.lvl", "");

        let item = game
            .world
            .entities
            .iter()
            .find(|entity| entity.alive && entity.object == "oBombBag")
            .expect("custom item symbol must create its item");
        assert_eq!(item.value("depth"), expected_depth);
        assert_eq!(item.value("cost"), 0.0);
        assert_eq!(item.value("forSale"), 0.0);
    }
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
        assert_eq!(
            caveman.text.get("type").map(String::as_str),
            Some("Caveman")
        );
        assert_eq!(caveman.value("active"), 1.0);
        assert_eq!(caveman.value("blinkToggle"), 0.0);
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
        .filter(|entity| {
            matches!(entity.object.as_str(), "oDark" | "oBrick")
                && entity.pos.y == 80.0
                && (96.0..=112.0).contains(&entity.pos.x)
        })
        .collect();
    assert_eq!(terrain.len(), 2);
    for entity in terrain {
        assert_eq!(entity.value("terrain_initialized"), 1.0);
        assert_eq!(entity.value("terrain_setup_initialized"), 1.0);
        assert!(match entity.object.as_str() {
            "oDark" => entity.sprite.starts_with("sDark"),
            "oBrick" => entity.sprite.starts_with("sCave"),
            _ => false,
        });
    }
}
