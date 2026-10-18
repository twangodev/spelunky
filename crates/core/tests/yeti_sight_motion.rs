//! Ordinary default Yeti IDLE emits source builtin-moving sight.
//! All room/seed selection happens before the first Step; no midway writes.
use spelunky_core::*;

fn room(rules: RulesPreset, algorithm: RngAlgorithm, facing: i8, wall: bool) -> (Game, u64) {
    for seed in 0..128 {
        let mut game = Game::shell_with_algorithm(seed, rules, algorithm).unwrap();
        game.world = World::new(24, 20, Area::Ice, 9);
        for x in 0..game.world.width {
            game.world.set_tile(x, 10, "oBlock");
        }
        if wall {
            game.world
                .set_tile(if facing < 0 { 4 } else { 8 }, 9, "oBlock");
        }
        game.player.pos = Vec2::new(40.0, 152.0);
        game.player.previous_pos = game.player.pos;
        let id = game.spawn("oYeti", Vec2::new(96.0, 144.0));
        let e = game.world.entity(id).unwrap();
        assert_eq!(e.state, 0);
        assert_eq!(e.health, 5);
        assert_eq!(e.vel, Vec2::new(2.5, 0.0));
        assert_eq!(e.value("counter"), 0.0);
        assert_eq!(e.value("sight"), 0.0);
        assert!(!e.held);
        // Source's first ordinary IDLE-to-WALK transition draws rand(0,1).
        // Select only the initial seed; cloned prediction never writes RNG.
        let predicted = if game.rng.clone().range(2) == 0 {
            -1
        } else {
            1
        };
        if predicted == facing {
            return (game, id);
        }
    }
    panic!("SOURCE_INITIAL_SEED_FACING_UNAVAILABLE");
}
fn variants(mut check: impl FnMut(RulesPreset, RngAlgorithm, i8)) {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for algorithm in [RngAlgorithm::Gm8Lcg32V1, RngAlgorithm::LegacySplitMix64V1] {
            for facing in [-1, 1] {
                check(rules, algorithm, facing);
            }
        }
    }
}
fn first_ray(game: &Game, owner: u64) -> u64 {
    let rays: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oEnemySight" && e.value("owner") == owner as f64)
        .collect();
    assert_eq!(rays.len(), 1, "SOURCE_REAL_YETI_FIRST_SIGHT_BIRTH");
    rays[0].id
}
#[test]
fn natural_yeti_sight_moves_ten_pixels_at_birth_and_next_step_in_both_directions() {
    variants(|rules, algorithm, facing| {
        let (mut game, owner) = room(rules, algorithm, facing, false);
        game.step(&InputFrame::default());
        let actor = game.world.entity(owner).unwrap();
        assert_eq!(actor.facing, facing, "SOURCE_INITIAL_IDLE_FACING_DRAW");
        assert_eq!(actor.state, 1);
        assert_eq!(actor.health, 5);
        let origin = actor.pos;
        let id = first_ray(&game, owner);
        let ray = game.world.entity(id).unwrap();
        eprintln!(
            "SIGHT {rules:?}/{algorithm:?} facing={facing} actor={origin:?} ray={:?}",
            ray.pos
        );
        assert_eq!(
            ray.pos,
            Vec2::new(origin.x + f64::from(facing) * 10.0, origin.y),
            "SOURCE_YETI_SIGHT_BIRTH_BUILTIN_TEN_PIXELS"
        );
        assert_eq!(ray.value("hspeed"), f64::from(facing) * 10.0);
        assert_eq!(ray.value("vspeed"), 0.0);
        assert_eq!(
            ray.vel,
            Vec2::default(),
            "SOURCE_SIGHT_HAS_NO_CUSTOM_XVEL_STEP"
        );
        game.step(&InputFrame::default());
        let ray = game
            .world
            .entity(id)
            .expect("unobstructed source sight survives");
        assert_eq!(
            ray.pos,
            Vec2::new(origin.x + f64::from(facing) * 20.0, origin.y),
            "SOURCE_YETI_SIGHT_SECOND_BUILTIN_TEN_PIXELS"
        );
        assert_eq!(game.player.health, 4.0);
    });
}
#[test]
fn natural_yeti_sight_retires_at_authored_wall_on_second_step() {
    variants(|rules, algorithm, facing| {
        let (mut game, owner) = room(rules, algorithm, facing, true);
        game.step(&InputFrame::default());
        assert_eq!(game.world.entity(owner).unwrap().facing, facing);
        let id = first_ray(&game, owner);
        game.step(&InputFrame::default());
        assert!(
            game.world.entity(id).is_none(),
            "SOURCE_YETI_SIGHT_SECOND_TICK_WALL_RETIREMENT"
        );
        assert_eq!(game.world.entity(owner).unwrap().health, 5);
        assert_eq!(game.player.health, 4.0);
    });
}
#[test]
fn natural_yeti_sight_without_authored_wall_remains_alive() {
    variants(|rules, algorithm, facing| {
        let (mut game, owner) = room(rules, algorithm, facing, false);
        game.step(&InputFrame::default());
        assert_eq!(game.world.entity(owner).unwrap().facing, facing);
        let id = first_ray(&game, owner);
        for _ in 0..2 {
            game.step(&InputFrame::default());
        }
        assert!(
            game.world.entity(id).is_some(),
            "SOURCE_YETI_SIGHT_OPEN_PATH_SURVIVAL_CONTROL"
        );
        assert_eq!(game.world.entity(owner).unwrap().health, 5);
        assert_eq!(game.player.health, 4.0);
    });
}
