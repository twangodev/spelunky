//! Source instance_place uses the translated current sprite, not its origin.
//! All platforms/ladders are initial room geometry; after setup only real
//! Game.step inputs may alter the player or world.
use spelunky_core::*;

#[derive(Clone, Copy, Debug)]
enum Storage {
    AuthoredTile,
    SpawnedEntity,
}
#[derive(Clone, Copy, Debug)]
enum Placement {
    Aligned,
    Adjacent,
    Far,
}

fn mask(game: &Game, pos: Vec2) -> catalog::CollisionMask {
    let player = &game.player;
    catalog::instance_mask(
        "oPlayer1",
        &player.sprite,
        pos,
        player.animation,
        Vec2::new(
            player.data.get("image_xscale").copied().unwrap_or(1.0),
            player.data.get("image_yscale").copied().unwrap_or(1.0),
        ),
        -player
            .data
            .get("image_angle")
            .copied()
            .unwrap_or(0.0)
            .to_radians(),
    )
}

fn target_mask(game: &Game, instance: dispatcher::Instance) -> catalog::CollisionMask {
    match instance {
        dispatcher::Instance::Terrain(index) => game.world.tile_collision_mask(
            (index % game.world.width) as i32,
            (index / game.world.width) as i32,
        ),
        dispatcher::Instance::Entity(id) => {
            let entity = game.world.entity(id).unwrap();
            assert!(entity.alive && entity.engine_active);
            assert_eq!(entity.object, "oLadderOrange");
            catalog::instance_mask(
                &entity.object,
                &entity.sprite,
                entity.pos,
                entity.animation,
                Vec2::new(1.0, 1.0),
                0.0,
            )
        }
        dispatcher::Instance::Player => panic!("player is not a ladder"),
    }
}

fn room(rules: RulesPreset, storage: Storage, placement: Placement, side: i8) -> Game {
    let mut level = level::Level::default();
    level.set_tile(8, 4, '@').unwrap();
    // Level(x,y) maps to raw World(16+x*16,16+y*16).
    // This lower floor is a safety landing, 64px below the initial platform.
    for x in 0..40 {
        level.set_tile(x, 9, '1').unwrap();
    }
    let offset = match placement {
        Placement::Aligned => 0,
        Placement::Adjacent => 16,
        Placement::Far => 32,
    };
    let raw_x = 144 + i32::from(side) * offset;
    if matches!(storage, Storage::AuthoredTile) {
        level.set_tile((raw_x / 16 - 1) as usize, 5, 'L').unwrap();
    }
    let mut game = Game::shell(42, rules);
    game.load_custom(&level, "ladder-instance-place", "");
    game.spawn("oTreeBranchTile", Vec2::new(144.0, 96.0));
    if matches!(storage, Storage::SpawnedEntity) {
        game.spawn("oLadderOrange", Vec2::new(f64::from(raw_x), 96.0));
    }
    game
}

fn diagnostic(game: &Game) -> String {
    format!(
        "tick={} pos={:?} velocity={:?} state={:?} grounded={} HP={} scale={:?}",
        game.tick,
        game.player.pos,
        game.player.vel,
        game.player.state,
        game.player.grounded,
        game.player.health,
        game.player.data.get("image_xscale")
    )
}

fn approach(game: &mut Game, side: i8, placement: Placement) {
    for _ in 0..4 {
        game.step(&InputFrame::default());
    }
    assert_eq!(
        game.player.pos,
        Vec2::new(152.0, 88.0),
        "{}",
        diagnostic(game)
    );
    assert!(game.player.grounded);
    assert_eq!(game.player.health, 4.0);
    let toward = InputFrame {
        left: side < 0,
        right: side > 0,
        ..Default::default()
    };
    if matches!(placement, Placement::Aligned) {
        // A real directional edge persists source Draw mirroring before the
        // next Step, without injecting any image transform or facing.
        game.step(&toward);
        assert_eq!(game.player.pos, Vec2::new(152.0, 88.0));
    } else {
        let mut reached = false;
        for _ in 0..24 {
            game.step(&toward);
            assert!(game.player.grounded, "{}", diagnostic(game));
            assert_eq!(game.player.pos.y, 88.0, "{}", diagnostic(game));
            assert_eq!(game.player.health, 4.0);
            let delta = (game.player.pos.x - 152.0) * f64::from(side);
            if (5.0..=7.0).contains(&delta) {
                reached = true;
                break;
            }
        }
        assert!(
            reached,
            "ordinary walking must reach the platform edge: {}",
            diagnostic(game)
        );
    }
    assert_eq!(
        game.player.data.get("image_xscale"),
        Some(&-f64::from(side))
    );
    assert!(game.player.grounded);
    assert!(!game.player.is_stunned());
    assert_eq!(game.player.attack_timer, 0);
    assert!(
        !game
            .world
            .solid_at(game.player.pos.x, game.player.pos.y + 8.0)
    );
}

fn down(game: &mut Game, side: i8) {
    game.step(&InputFrame {
        down: true,
        down_pressed: true,
        left_released: side < 0,
        right_released: side > 0,
        ..Default::default()
    });
    assert_eq!(game.player.health, 4.0, "{}", diagnostic(game));
    assert!(!game.player.is_stunned());
    assert!(!game.world.solid_at(game.player.pos.x, game.player.pos.y));
}

#[test]
fn natural_adjacent_ladder_mask_blocks_platform_drop_without_snapping() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for storage in [Storage::AuthoredTile, Storage::SpawnedEntity] {
            for side in [-1, 1] {
                let mut game = room(rules, storage, Placement::Adjacent, side);
                approach(&mut game, side, Placement::Adjacent);
                let ladders = dispatcher::descendant_instances(&game, "oLadder");
                assert_eq!(ladders.len(), 1);
                let origin = ladders[0].position(&game);
                let query = Vec2::new(game.player.pos.x, game.player.pos.y + 16.0);
                let target = target_mask(&game, ladders[0]);
                assert!(!target.contains(query, CollisionPrecision::BoundingBox));
                assert!(
                    mask(&game, query).intersects_mask(&target),
                    "{}",
                    diagnostic(&game)
                );
                assert!((game.player.pos.x - (origin.x + 8.0)).abs() >= 4.0);
                let y = game.player.pos.y;
                down(&mut game, side);
                assert_eq!(
                    game.player.pos.y,
                    y,
                    "found-but-unaligned source ladder must block else-drop: {}",
                    diagnostic(&game)
                );
                assert!(game.player.grounded, "{}", diagnostic(&game));
                assert_ne!(game.player.state, PlayerState::Climbing);
                assert_ne!(game.player.state, PlayerState::Falling);
            }
        }
    }
}

#[test]
fn natural_aligned_ladder_attaches_but_nonoverlapping_ladder_allows_drop() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for storage in [Storage::AuthoredTile, Storage::SpawnedEntity] {
            for side in [-1, 1] {
                let mut aligned = room(rules, storage, Placement::Aligned, side);
                approach(&mut aligned, side, Placement::Aligned);
                let ladders = dispatcher::descendant_instances(&aligned, "oLadder");
                assert_eq!(ladders.len(), 1);
                let query = Vec2::new(aligned.player.pos.x, aligned.player.pos.y + 16.0);
                let target = target_mask(&aligned, ladders[0]);
                assert!(target.contains(query, CollisionPrecision::BoundingBox));
                assert!(mask(&aligned, query).intersects_mask(&target));
                assert!(
                    (aligned.player.pos.x - (ladders[0].position(&aligned).x + 8.0)).abs() < 4.0
                );
                down(&mut aligned, side);
                assert_eq!(
                    aligned.player.state,
                    PlayerState::Climbing,
                    "{}",
                    diagnostic(&aligned)
                );
                assert_eq!(aligned.player.pos, Vec2::new(152.0, 88.0));
                assert_eq!(aligned.player.vel, Vec2::default());

                let mut far = room(rules, storage, Placement::Far, side);
                approach(&mut far, side, Placement::Far);
                let ladders = dispatcher::descendant_instances(&far, "oLadder");
                assert_eq!(ladders.len(), 1);
                let query = Vec2::new(far.player.pos.x, far.player.pos.y + 16.0);
                let target = target_mask(&far, ladders[0]);
                assert!(!target.contains(query, CollisionPrecision::BoundingBox));
                assert!(!mask(&far, query).intersects_mask(&target));
                let y = far.player.pos.y;
                down(&mut far, side);
                assert_eq!(
                    far.player.state,
                    PlayerState::Falling,
                    "{}",
                    diagnostic(&far)
                );
                assert!(
                    far.player.pos.y > y && far.player.vel.y > 0.0,
                    "{}",
                    diagnostic(&far)
                );
            }
        }
    }
}
