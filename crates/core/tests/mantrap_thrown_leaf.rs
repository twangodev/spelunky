//! Source-only authored-room proposal; no test in this packet has been run.
//! Actors use default Create. After setup only pickup, release and throw inputs
//! execute; no HP, state, velocity, timer, pose or RNG is injected mid-run.
use spelunky_core::*;

fn ordinary_throw(preset: RulesPreset, object: &str) -> (Game, u64, u64) {
    let mut game = Game::shell(0, preset);
    game.world = World::new(30, 20, Area::Jungle, 5);
    game.world.name = "rLevel".into();
    for x in 0..30 {
        game.world.set_tile(x, 10, "oHardBlock");
    }
    game.player.pos = Vec2::new(64.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.player.facing = 1;
    game.world.entrance = game.player.pos;
    let target = game.spawn(object, Vec2::new(112.0, 144.0));
    assert_eq!(game.world.entity(target).unwrap().health, 3);
    assert_eq!(game.world.entity(target).unwrap().state, 0);
    let rock = game.spawn("oRock", Vec2::new(64.0, 156.0));
    game.step(&InputFrame {
        down: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert_eq!(game.player.held, Some(rock), "ORDINARY_ROCK_PICKUP");
    assert_eq!(game.player.attack_timer, 0);
    game.step(&InputFrame {
        attack_released: true,
        ..Default::default()
    });
    assert_eq!(game.player.held, Some(rock));
    assert_eq!(game.player.state, PlayerState::Standing);
    assert_eq!(game.world.entity(target).unwrap().health, 3);
    (game, target, rock)
}

fn count(game: &Game, object: &str) -> usize {
    game.world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == object)
        .count()
}

fn first_hit(preset: RulesPreset, object: &str) {
    let (mut game, id, rock) = ordinary_throw(preset, object);
    for tick in 0..18 {
        let before = game.world.entity(id).expect("DEFAULT_TARGET_ALIVE").clone();
        assert_eq!(before.health, 3, "NO_EARLIER_HIT_OR_DEATH");
        assert_ne!(before.state, 98, "ELIGIBLE_NONSTUNNED_TARGET");
        assert_eq!(count(&game, "oLeaf"), 0);
        assert_eq!(count(&game, "oBlood"), 0);
        let mut oracle = game.rng.clone();
        // Native source oItem producer arguments precede ManTrap's own Step.
        let birth = Vec2::new(
            before.pos.x + f64::from(oracle.range(17)),
            before.pos.y - 8.0 + f64::from(oracle.range(17)),
        );
        oracle.range(8); // ManTrap's unconditional eager tail after the impact.
        game.step(&InputFrame {
            attack: tick == 0,
            attack_pressed: tick == 0,
            attack_released: tick == 1,
            ..Default::default()
        });
        assert_eq!(game.player.held, None, "ORDINARY_THROW_RELEASED_ROCK");
        assert_eq!(game.player.health, 4, "NO_UNRELATED_PLAYER_CONTACT");
        assert!(
            !game
                .world
                .entities
                .iter()
                .any(|e| e.alive && catalog::is_a(&e.object, "oWhip"))
        );
        let victim = game.world.entity(id).expect("ONE_HIT_SURVIVES");
        if victim.health != before.health {
            assert_eq!(victim.health, 2, "ACTUAL_SINGLE_MOVING_ITEM_HIT");
            assert_eq!(victim.state, 98);
            assert_eq!(victim.timer, 200, "SOURCE_STUN_COUNTER");
            assert!(
                game.events
                    .iter()
                    .any(|e| matches!(e, GameEvent::Sound{name,..} if name == "sndHit"))
            );
            let projectile = game.world.entity(rock).expect("ORDINARY_ROCK_SURVIVES");
            assert!(!projectile.held);
            assert!(
                projectile.vel.x.abs() > 2.0 || projectile.vel.y.abs() > 2.0,
                "SOURCE_MOVING_ITEM_GATE"
            );
            // Public source-bbox proxy for the explicit radius2 item query.
            // Rock450 precedes ManTrap532, so its target pose is the captured
            // pre-Step pose. Caveman325 already ran: use its post-Step pose.
            let presented = game.presentation();
            let queried = if object == "oManTrap" {
                &before
            } else {
                presented
                    .world
                    .entity(id)
                    .expect("PRE_ANIMATION_TARGET_PROXY")
            };
            let mask = catalog::instance_mask(
                &queried.object,
                &queried.sprite,
                queried.pos,
                queried.animation,
                Vec2::new(
                    queried.data.get("image_xscale").copied().unwrap_or(1.0),
                    queried.data.get("image_yscale").copied().unwrap_or(1.0),
                ),
                -queried.value("image_angle").to_radians(),
            );
            assert!(
                mask.intersects_rectangle_bbox(
                    Vec2::new(projectile.pos.x - 2.0, projectile.pos.y - 2.0),
                    Vec2::new(projectile.pos.x + 2.0, projectile.pos.y + 2.0),
                ),
                "PUBLIC_SOURCE_ITEM_RECTANGLE_PROXY"
            );
            if object == "oManTrap" {
                assert_eq!(count(&game, "oLeaf"), 1, "SOURCE_ORDINARY_MANTRAP_LEAF");
                assert_eq!(count(&game, "oBlood"), 0, "MANTRAP_LEAF_REPLACES_HIT_BLOOD");
                let leaf = game
                    .world
                    .entities
                    .iter()
                    .find(|e| e.alive && e.object == "oLeaf")
                    .unwrap();
                assert_eq!(
                    leaf.pos,
                    Vec2::new(birth.x, birth.y + 0.4),
                    "LEAF742_BIRTH_STEP_RAW_MOTION"
                );
                assert_eq!(leaf.vel, Vec2::new(0.0, 0.4 + 0.01));
                assert_eq!(leaf.image_speed(), 0.2);
                assert_eq!(leaf.animation, 0.2);
                assert_eq!(leaf.age, 0);
                assert_eq!(leaf.value("world_effect_initialized"), 1.0);
                assert_eq!(
                    game.rng.state_bits(),
                    oracle.state_bits(),
                    "TWO_CALLER_DRAWS_ZERO_CREATE_DRAWS_THEN_TAIL8"
                );
            } else {
                assert_eq!(count(&game, "oLeaf"), 0, "CAVEMAN_HAS_NO_LEAF_BRANCH");
                assert_eq!(
                    count(&game, "oBlood"),
                    1,
                    "MATCHED_CAVEMAN_EMITS_OWN_HIT_BLOOD"
                );
                // Caveman runs before Rock and has independent sight/AI RNG;
                // the ManTrap whole-frame draw oracle does not apply here.
            }
            return;
        }
    }
    panic!("BOUNDED_ORDINARY_ROCK_TARGET_CONTACT_NOT_REACHED: {object}");
}

#[test]
fn ordinary_pickup_throw_mantrap_hit_creates_one_source_leaf() {
    for preset in [RulesPreset::Classic, RulesPreset::Assisted] {
        first_hit(preset, "oManTrap");
    }
}

#[test]
fn matched_ordinary_caveman_hit_creates_blood_and_no_leaf() {
    for preset in [RulesPreset::Classic, RulesPreset::Assisted] {
        first_hit(preset, "oCaveman");
    }
}
