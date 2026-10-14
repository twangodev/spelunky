//! Controlled source-branch cases cover UFO patrol boundaries and Alien side
//! contacts. The Yeti case uses a real default grab before a controlled incoming
//! projectile callback; it does not claim an ordinary UFO/Boss launch journey.
use super::*;
use crate::{Area, InputFrame, RngAlgorithm, RulesPreset, World};

fn variants(mut check: impl FnMut(Game)) {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for algorithm in [RngAlgorithm::Gm8Lcg32V1, RngAlgorithm::LegacySplitMix64V1] {
            let mut game = Game::shell_with_algorithm(7, rules, algorithm).unwrap();
            game.world = World::new(30, 20, Area::Ice, 9);
            game.world.name = "rLevel".into();
            game.player.pos = Vec2::new(40.0, 152.0);
            game.player.previous_pos = game.player.pos;
            check(game);
        }
    }
}

fn controlled_actor(object: &str, pos: Vec2) -> Entity {
    let mut actor = Entity::new(900, object, pos);
    crate::enemies::init_entity(&mut actor);
    actor.vel = Vec2::default();
    actor
}

#[test]
fn controlled_ufo_search_limit_resets_only_when_character_is_beyond_that_side() {
    variants(|game| {
        for (shift, toggle, player_x, expected_shift, expected_toggle) in [
            (31.0, 0.0, 120.0, 0.0, 1.0),
            (31.0, 0.0, 80.0, 32.0, 1.0),
            (-31.0, 1.0, 80.0, 0.0, 0.0),
            (-31.0, 1.0, 120.0, -32.0, 0.0),
        ] {
            let mut game = game.clone();
            game.player.pos.x = player_x;
            let mut actor = controlled_actor("oUFO", Vec2::new(100.0, 96.0));
            actor.data.insert("shift".into(), shift);
            actor.data.insert("shift_toggle".into(), toggle);
            let rng = game.rng.state_bits();
            ufo(&mut game, &mut actor);
            assert_eq!(actor.value("shift"), expected_shift);
            assert_eq!(actor.value("shift_toggle"), expected_toggle);
            assert_eq!(actor.state, 0);
            assert_eq!(actor.vel.y, 2.0);
            assert_eq!(game.rng.state_bits(), rng, "patrol consumes no RNG");
        }
    });
}

#[test]
fn controlled_ufo_destroy_limit_keeps_shift_even_with_character_beyond_side() {
    variants(|game| {
        for (shift, toggle, player_x, expected) in
            [(63.0, 0.0, 140.0, 64.0), (-63.0, 1.0, 60.0, -64.0)]
        {
            let mut game = game.clone();
            game.player.pos.x = player_x;
            let mut actor = controlled_actor("oUFO", Vec2::new(100.0, 96.0));
            actor.state = 1;
            actor.data.insert("shift".into(), shift);
            actor.data.insert("shift_toggle".into(), toggle);
            ufo(&mut game, &mut actor);
            assert_eq!(actor.value("shift"), expected);
            assert_eq!(actor.state, 1);
            assert_eq!(actor.vel.y, 0.0);
        }
    });
}

#[test]
fn controlled_ufo_search_crossover_finishes_search_without_same_step_charge() {
    variants(|mut game| {
        game.player.pos.x = 108.0;
        let mut actor = controlled_actor("oUFO", Vec2::new(100.0, 104.0));
        actor.vel.y = -2.0;
        actor.data.insert("shift".into(), 31.0);
        actor.data.insert("shift_toggle".into(), 0.0);
        ufo(&mut game, &mut actor);
        assert_eq!(actor.state, 1);
        assert_eq!(actor.vel.y, 2.0, "SEARCH crossover retains its yVel write");
        assert_eq!(actor.value("shift"), 0.0, "SEARCH still uses limit32/reset");
        assert!(!game.events.iter().any(|event| {
            matches!(event, GameEvent::Sound { name, .. } if name == "sndLaserCharge")
        }));
    });
}

#[test]
fn controlled_ufo_stationary_contact_nudges_and_sets_each_state_patrol_limit() {
    variants(|game| {
        for (state, limit) in [(0, 32.0), (1, 64.0)] {
            for (wall_x, origin_x, expected_x, toggle, shift) in
                [(5, 92.0, 93.0, 0.0, -limit), (7, 100.0, 99.0, 1.0, limit)]
            {
                let mut game = game.clone();
                game.world.set_tile(wall_x, 6, "oBlock");
                let mut actor = controlled_actor("oUFO", Vec2::new(origin_x, 96.0));
                actor.state = state;
                actor.data.insert("shift_toggle".into(), 0.0);
                actor.data.insert("shift".into(), 0.0);
                assert_eq!(actor.vel.x, 0.0, "no displacement wall flag can mask this");
                ufo(&mut game, &mut actor);
                assert_eq!(actor.pos.x, expected_x, "explicit postmove contact nudge");
                assert_eq!(actor.value("shift_toggle"), toggle);
                assert_eq!(actor.value("shift"), shift);
                assert_eq!(actor.state, state);
            }
        }
    });
}

fn avoid_alien_idle_roll(game: &mut Game) {
    // Controlled own-branch isolation, explicitly not an ordinary seed route.
    // Skip only initial RNG states whose next single WALK roll enters IDLE.
    while game.rng.clone().range(100) == 0 {
        game.rng.next_u64();
    }
}

#[test]
fn controlled_alien_unsupported_walk_selects_left_after_empty_side_checks() {
    variants(|game| {
        for facing in [-1, 1] {
            let mut game = game.clone();
            avoid_alien_idle_roll(&mut game);
            let mut actor = controlled_actor("oAlien", Vec2::new(96.0, 64.0));
            actor.state = 1;
            actor.facing = facing;
            alien(&mut game, &mut actor);
            assert_eq!(actor.state, 1);
            assert_eq!(
                actor.facing, -1,
                "both-invalid clause sets LEFT independently"
            );
            assert_eq!(actor.vel.x, 0.0);
            assert_eq!(actor.health, 1);
        }
    });
}

#[test]
fn controlled_alien_stationary_side_contact_assigns_direction_without_toggling() {
    variants(|game| {
        // Staggered authored opaque Blocks isolate each full-body side probe:
        // one wall contacts below the origin, opposite head blocks an empty
        // side reversal, and only the wall side has its foot point occupied.
        for (origin_x, upper_x, lower_x, expected) in [(98.0, 6, 7, -1), (94.0, 6, 5, 1)] {
            for facing in [-1, 1] {
                let mut game = game.clone();
                game.world.set_tile(upper_x, 3, "oBlock");
                game.world.set_tile(lower_x, 4, "oBlock");
                avoid_alien_idle_roll(&mut game);
                let mut actor = controlled_actor("oAlien", Vec2::new(origin_x, 60.0));
                actor.state = 1;
                actor.facing = facing;
                let contacts = crate::enemies::body_contacts(&game.world, &actor);
                assert_eq!(contacts.right, expected < 0);
                assert_eq!(contacts.left, expected > 0);
                assert!(!game.world.solid_at(actor.pos.x + 8.0, actor.pos.y + 8.0));
                alien(&mut game, &mut actor);
                assert_eq!(actor.state, 1);
                assert_eq!(actor.facing, expected);
                assert_eq!(actor.vel.x, f64::from(expected));
                assert_eq!(actor.health, 1);
                assert!(actor.alive);
            }
        }
    });
}

#[test]
fn default_yeti_grab_then_controlled_laser_or_wave_hit_retains_throw_friction() {
    variants(|game| {
        for object in ["oLaser", "oPsychicWave"] {
            let mut game = game.clone();
            for x in 0..game.world.width {
                game.world.set_tile(x, 10, "oBlock");
            }
            game.player.pos = Vec2::new(136.0, 152.0);
            game.player.previous_pos = game.player.pos;
            game.player.state = PlayerState::Standing;
            game.player.grounded = true;
            let id = game.spawn("oYeti", Vec2::new(128.0, 144.0));
            assert_eq!(game.world.entity(id).unwrap().vel.x, 2.5);
            game.step(&InputFrame::default());
            let actor = game.world.entity(id).unwrap();
            assert_eq!(actor.state, 3, "ordinary source-default grab enters THROW");
            assert_eq!(actor.vel.x, 0.0, "grab entry still stops Yeti");
            assert_eq!(actor.health, 5);
            let origin = actor.pos;
            // Controlled incoming source callback: no claim that these hazards
            // launched from an ordinary UFO/Boss in this focused regression.
            let mut projectile = controlled_actor(object, origin);
            assert!(collision(
                &mut game,
                &mut projectile,
                crate::dispatcher::Instance::Entity(id),
            ));
            let actor = game.world.entity(id).unwrap();
            assert_eq!(actor.state, 3, "incoming hazard preserves target status");
            assert_eq!(actor.health, 2);
            assert_eq!(actor.vel, Vec2::new(-1.0, -6.0));
            game.step(&InputFrame::default());
            let actor = game.world.entity(id).unwrap();
            assert_eq!(actor.state, 3);
            assert_eq!(actor.sprite, "sYetiThrowL");
            assert_eq!(actor.pos.x, origin.x - 1.0, "first knockback displacement");
            assert!(
                (actor.vel.x + 0.9).abs() < 1e-12,
                "THROW final friction, not zero"
            );
            assert_eq!(actor.health, 2);
        }
    });
}
