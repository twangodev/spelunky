use spelunky_core::{Game, InputFrame, PlayerState, Rng, RulesPreset, Vec2, generation};
use std::collections::BTreeSet;

#[test]
fn generated_jungle_pools_have_source_surfaces_and_allow_player_swimming() {
    let mut pools = 0;
    let mut surfaces = 0;
    for seed in 1..=16 {
        let world = generation::generate(&mut Rng::new(seed), 5, &BTreeSet::new());
        let water: Vec<_> = world
            .entities
            .iter()
            .filter(|e| e.alive && e.object == "oWaterSwim")
            .cloned()
            .collect();
        pools += water.len();
        surfaces += water.iter().filter(|e| e.sprite == "sWaterTop").count();
        if let Some(cell) = water
            .iter()
            .find(|e| e.sprite == "sWaterTop" && !world.solid_at(e.pos.x + 8.0, e.pos.y + 8.0))
        {
            let pos = Vec2::new(cell.pos.x + 8.0, cell.pos.y + 8.0);
            let mut game = Game::shell(seed, RulesPreset::Classic);
            game.world = world;
            game.player.pos = pos;
            game.player.previous_pos = pos;
            game.player.state = PlayerState::Falling;
            game.player.invincible = 100;
            game.camera = Vec2::new((pos.x - 160.0).max(0.0), (pos.y - 120.0).max(0.0));
            game.step(&InputFrame::default());
            assert_eq!(game.player.data.get("swimming"), Some(&1.0), "seed {seed}");
            assert!(
                game.world.entities.iter().any(|e| e.object == "oSplash"),
                "seed {seed}"
            );
        }
    }
    assert!(pools > 0, "fixture must exercise generated water");
    assert!(
        surfaces > 0,
        "generated pools never received source surface sprites"
    );
}

#[test]
fn custom_water_keeps_source_create_sprite_without_generated_setup() {
    let mut level = spelunky_core::level::Level::default();
    level.set_tile(3, 3, '@').unwrap();
    level.set_tile(5, 5, 'w').unwrap();
    let world = level.to_world();
    assert!(
        world.tiles.iter().any(|tile| tile.object == "oWaterSwim")
            || world.entities.iter().any(|e| e.object == "oWaterSwim")
    );
    assert!(
        !world
            .entities
            .iter()
            .any(|e| e.object == "oWaterSwim" && e.sprite == "sWaterTop")
    );
    assert!(!world.entities.iter().any(|e| {
        e.text
            .get("terrain_sprite")
            .is_some_and(|s| s == "sWaterTop")
    }));
}
