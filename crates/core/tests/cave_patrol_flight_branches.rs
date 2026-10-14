//! Literal own-Step branches from oSnake and oBat. Natural routes use fresh
//! actors and ordinary Game::step inputs after room bootstrap. The two initial
//! WALK/wet-ATTACK fixtures are explicitly configured source-branch controls.
//! These tests do not execute the original GameMaker runtime.
use spelunky_core::*;

fn modes() -> impl Iterator<Item = (RulesPreset, RngAlgorithm)> {
    [RulesPreset::Classic, RulesPreset::Assisted]
        .into_iter()
        .flat_map(|rules| {
            [RngAlgorithm::Gm8Lcg32V1, RngAlgorithm::LegacySplitMix64V1]
                .into_iter()
                .map(move |algorithm| (rules, algorithm))
        })
}

fn room(rules: RulesPreset, algorithm: RngAlgorithm, seed: u64) -> Game {
    let mut game = Game::shell_with_algorithm(seed, rules, algorithm).unwrap();
    game.world = World::new(30, 20, Area::Mines, 1);
    game.world.name = "rLevel".into();
    game.player.pos = Vec2::new(240.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game.world.set_tile(15, 10, "oBlock");
    game
}

#[test]
fn natural_snake_walk_on_isolated_support_fixes_left_facing_without_extra_rng() {
    for (rules, algorithm) in modes() {
        // Select the source IDLE draw before any Steps, never by the later
        // patrol result. Each actor retains all fresh Create fields.
        let mut selected = None;
        for seed in 0..16 {
            let game = room(rules, algorithm, seed);
            if game.rng.clone().range(2) == 0 {
                selected = Some(game);
                break;
            }
        }
        let mut game = selected.expect("SOURCE_IDLE_LEFT_SEED");
        game.world.set_tile(6, 10, "oBlock");
        let id = game.spawn("oSnake", Vec2::new(91.0, 144.0));
        let fresh = game.world.entity(id).unwrap();
        assert_eq!((fresh.state, fresh.facing), (0, 1));
        assert_eq!(fresh.vel, Vec2::new(2.5, 0.0));
        game.step(&InputFrame::default());
        let walk = game.world.entity(id).unwrap();
        assert_eq!(walk.pos, Vec2::new(94.0, 144.0));
        assert_eq!((walk.state, walk.facing), (1, -1));
        let mut expected = game.rng.clone();
        if expected.chance(1, 100) {
            expected.range(31);
        }
        game.step(&InputFrame::default());
        let snake = game.world.entity(id).unwrap();
        assert_eq!(snake.pos, Vec2::new(96.0, 144.0));
        assert!(!game.world.solid_at(95.0, 160.0));
        assert!(!game.world.solid_at(112.0, 160.0));
        assert!(!game.world.solid_at(95.0, 144.0));
        assert!(!game.world.solid_at(112.0, 144.0));
        assert_eq!(snake.health, 1);
        assert_eq!(snake.vel.x, 0.0);
        // The ledge reversal selects RIGHT; source Step82-87 then overrides
        // it to LEFT because this fixture's upper-left point is clear.
        assert_eq!(snake.facing, -1, "BOTH_BLOCKED_CLEAR_LEFT_POINT");
        assert_eq!(game.rng.clone().next_u64(), expected.next_u64());
        assert_eq!(game.player.health, 4);
    }
}

#[test]
fn configured_snake_walk_with_upper_left_wall_fixes_right_facing() {
    for (rules, algorithm) in modes() {
        let mut game = room(rules, algorithm, 42);
        game.world.set_tile(5, 9, "oBlock");
        game.world.set_tile(5, 10, "oBlock");
        game.world.set_tile(6, 10, "oBlock");
        let id = game.spawn("oSnake", Vec2::new(96.0, 144.0));
        // Initial WALK control, not a natural IDLE transition claim.
        let snake = game.world.entity_mut(id).unwrap();
        snake.state = 1;
        snake.vel = Vec2::default();
        assert_eq!(snake.facing, 1);
        assert!(game.world.solid_at(95.0, 144.0));
        assert!(!game.world.solid_at(112.0, 160.0));
        let mut expected = game.rng.clone();
        if expected.chance(1, 100) {
            expected.range(31);
        }
        game.step(&InputFrame::default());
        let snake = game.world.entity(id).unwrap();
        assert_eq!(snake.pos, Vec2::new(96.0, 144.0));
        assert_eq!(snake.health, 1);
        assert_eq!(snake.vel.x, 0.0);
        assert_eq!(snake.facing, 1, "BOTH_BLOCKED_OPAQUE_LEFT_POINT");
        assert_eq!(game.rng.clone().next_u64(), expected.next_u64());
    }
}

fn swimming_route(rules: RulesPreset, algorithm: RngAlgorithm, roof: bool) -> (Game, u64) {
    let mut game = room(rules, algorithm, 42);
    for x in 0..30 {
        game.world.set_tile(x, 10, "oBlock");
    }
    for x in 9..=19 {
        game.world.set_tile(x, 9, "oWaterSwim");
    }
    if roof {
        for x in 4..=15 {
            game.world.set_tile(x, 5, "oBlock");
        }
    }
    game.player.pos = Vec2::new(128.0, 152.0);
    game.player.previous_pos = game.player.pos;
    let bat = game.spawn("oBat", Vec2::new(176.0, 96.0));
    assert_eq!(game.world.entity(bat).unwrap().state, 0);
    assert_eq!(game.world.entity(bat).unwrap().vel, Vec2::default());
    assert_eq!(
        game.player.data.get("swimming").copied().unwrap_or(0.0),
        0.0
    );
    (game, bat)
}

#[test]
fn naturally_swimming_player_crosses_bat_without_changing_its_flight_sprite() {
    for (rules, algorithm) in modes() {
        let (mut game, bat) = swimming_route(rules, algorithm, false);
        let mut qualified = false;
        let mut saw_live_chase = false;
        for _ in 0..64 {
            let before = game.world.entity(bat).unwrap().clone();
            let was_swimming = game.player.data.get("swimming").copied().unwrap_or(0.0) != 0.0;
            saw_live_chase |= before.state == 1
                && before.sprite == "sBatLeft"
                && before.vel.x < 0.0
                && !was_swimming;
            game.step(&InputFrame {
                right: true,
                ..Default::default()
            });
            let after = game.world.entity(bat).unwrap();
            let swimming = game.player.data.get("swimming").copied().unwrap_or(0.0) != 0.0;
            if before.state == 1
                && after.state == 1
                && was_swimming
                && swimming
                && before.sprite == "sBatLeft"
                && game.player.pos.x >= after.pos.x + 8.0
            {
                assert!(saw_live_chase, "DEFAULT_BAT_MUST_FIRST_CHASE_DRY_PLAYER");
                assert!(!game.world.solid_at(after.pos.x + 8.0, after.pos.y - 1.0));
                assert_eq!(after.health, 1);
                assert_eq!(game.player.health, 4);
                assert_eq!(after.vel, Vec2::new(0.0, -1.0));
                assert_eq!(
                    after.sprite, before.sprite,
                    "UNAVAILABLE_PLAYER_RETAINS_FLIGHT_SPRITE"
                );
                qualified = true;
                break;
            }
        }
        assert!(
            qualified,
            "NATURAL_SWIMMING_HORIZONTAL_CROSSING {rules:?}/{algorithm:?}"
        );
    }
}

#[test]
fn naturally_swimming_bat_returns_under_ceiling_then_keeps_hang_displacement() {
    for (rules, algorithm) in modes() {
        let (mut game, bat) = swimming_route(rules, algorithm, true);
        let mut returned = None;
        let mut saw_live_chase = false;
        for _ in 0..64 {
            let before = game.world.entity(bat).unwrap().clone();
            let was_swimming = game.player.data.get("swimming").copied().unwrap_or(0.0) != 0.0;
            saw_live_chase |= before.state == 1 && before.vel.y > 0.0 && !was_swimming;
            game.step(&InputFrame {
                right: true,
                ..Default::default()
            });
            let after = game.world.entity(bat).unwrap();
            if before.state == 1 && after.state == 0 && was_swimming {
                assert!(
                    saw_live_chase,
                    "DEFAULT_BAT_MUST_FIRST_DESCEND_TOWARD_DRY_PLAYER"
                );
                assert_eq!(before.vel, Vec2::new(0.0, -1.0));
                assert!(game.world.solid_at(after.pos.x + 8.0, after.pos.y - 1.0));
                assert_eq!(after.health, 1);
                assert_eq!(after.vel, before.vel, "CEILING_RETURN_WRITES_STATUS_ONLY");
                returned = Some(after.pos);
                break;
            }
        }
        let pos = returned.expect("NATURAL_SWIMMING_CEILING_RETURN");
        game.step(&InputFrame::default());
        let after = game.world.entity(bat).unwrap();
        assert_eq!(after.state, 0);
        assert_eq!(after.sprite, "sBatHang");
        assert_eq!(
            after.pos,
            Vec2::new(pos.x, pos.y - 1.0),
            "NEXT_HANG_USES_RETAINED_VELOCITY"
        );
        assert_eq!(after.vel, Vec2::new(0.0, -1.0));
        assert_eq!(game.player.health, 4);
    }
}

#[test]
fn configured_wet_bat_retains_incoming_velocity_after_blocked_vertical_move() {
    for (rules, algorithm) in modes() {
        for roof in [false, true] {
            let mut game = room(rules, algorithm, 42);
            game.player.pos = Vec2::new(128.0, 152.0);
            game.player.previous_pos = game.player.pos;
            game.world.set_tile(8, 10, "oBlock");
            game.world.set_tile(6, 6, "oWaterSwim");
            if roof {
                game.world.set_tile(6, 5, "oBlock");
            }
            let bat = game.spawn("oBat", Vec2::new(96.0, 94.0));
            // Explicit initial ATTACK/velocity control isolates source Step's
            // skipped wet assignment. No natural water-entry route is claimed.
            let actor = game.world.entity_mut(bat).unwrap();
            actor.state = 1;
            actor.vel = Vec2::new(0.0, -1.0);
            assert!(!game.world.solid_at(104.0, 102.0));
            assert_eq!(game.world.solid_at(98.0, 95.0), roof);
            game.step(&InputFrame::default());
            let after = game.world.entity(bat).unwrap();
            assert_eq!(after.pos, Vec2::new(96.0, if roof { 94.0 } else { 93.0 }));
            assert_eq!(after.state, 1);
            assert_eq!(after.health, 1);
            assert_eq!(
                game.player.data.get("swimming").copied().unwrap_or(0.0),
                0.0
            );
            assert_eq!(
                after.vel,
                Vec2::new(0.0, -1.0),
                "WET_SKIP_RETAINS_PRE_MOVE_VELOCITY"
            );
            assert_eq!(
                after.sprite, "sBatRight",
                "ELIGIBLE_PLAYER_STILL_SELECTS_FLIGHT_SPRITE"
            );
            assert_eq!(game.player.health, 4);
        }
    }
}

#[test]
fn configured_far_eligible_bat_returns_under_ceiling_without_zeroing_velocity() {
    for (rules, algorithm) in modes() {
        let mut game = room(rules, algorithm, 42);
        game.player.pos = Vec2::new(280.0, 152.0);
        game.player.previous_pos = game.player.pos;
        game.world.set_tile(17, 10, "oBlock");
        game.world.set_tile(6, 5, "oBlock");
        let bat = game.spawn("oBat", Vec2::new(96.0, 96.0));
        // Explicit initial ATTACK control for the separate dist>=160 branch.
        let actor = game.world.entity_mut(bat).unwrap();
        actor.state = 1;
        actor.vel = Vec2::new(0.6, -0.8);
        game.step(&InputFrame::default());
        let after = game.world.entity(bat).unwrap();
        assert!(Vec2::new(after.pos.x + 8.0, after.pos.y + 8.0).distance(game.player.pos) >= 160.0);
        assert_eq!(
            game.player.data.get("swimming").copied().unwrap_or(0.0),
            0.0
        );
        assert!(game.world.solid_at(after.pos.x + 8.0, after.pos.y - 1.0));
        assert_eq!(after.pos, Vec2::new(97.0, 95.0));
        assert_eq!(after.state, 0);
        assert_eq!(after.health, 1);
        assert_eq!(
            after.vel,
            Vec2::new(0.6, -0.8),
            "FAR_ELIGIBLE_CEILING_RETURN_WRITES_STATUS_ONLY"
        );
        assert_eq!(
            after.sprite, "sBatRight",
            "FAR_ELIGIBLE_TARGET_STILL_SELECTS_SPRITE"
        );
        game.step(&InputFrame::default());
        let hang = game.world.entity(bat).unwrap();
        assert_eq!(hang.state, 0);
        assert_eq!(hang.pos, Vec2::new(97.0, 94.0));
        assert_eq!(hang.vel, Vec2::new(0.6, -0.8));
        assert_eq!(hang.sprite, "sBatHang");
        assert_eq!(game.player.health, 4);
    }
}
