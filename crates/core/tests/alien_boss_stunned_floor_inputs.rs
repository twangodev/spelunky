//! Source AlienBoss floor clearing excludes actual STUNNED knockback.
//! Authored default births; real TombLord -> Fly collision, then neutral inputs.
use spelunky_core::{
    Area, Entity, Game, GameEvent, InputFrame, RngAlgorithm, RulesPreset, Vec2, World,
};

fn room(rules: RulesPreset, tomb: bool) -> (Game, u64, Option<u64>) {
    let mut game = Game::shell_with_algorithm(0, rules, RngAlgorithm::Gm8Lcg32V1).unwrap();
    game.world = World::new(30, 20, Area::Ice, 9);
    game.world.name = "rLevel".into();
    for x in 0..30 {
        game.world.set_tile(x, 10, "oHardBlock");
    }
    game.player.pos = Vec2::new(192.0, 152.0);
    game.player.previous_pos = game.player.pos;
    let boss = game.spawn("oAlienBoss", Vec2::new(120.0, 128.0));
    let actor = game.world.entity(boss).unwrap();
    assert_eq!((actor.health, actor.state, actor.facing), (10, 0, -1));
    assert_eq!(
        actor.vel,
        Vec2::new(2.5, 0.0),
        "QUALIFIED_CONSTRUCTOR_DEPENDENCY"
    );
    assert_eq!(actor.value("psychic_recover"), 100.0);
    assert_eq!(actor.sprite, "sAlienBoss");
    let tomb = tomb.then(|| game.spawn("oTombLord", Vec2::new(80.0, 128.0)));
    if let Some(id) = tomb {
        let actor = game.world.entity(id).unwrap();
        assert_eq!((actor.health, actor.state, actor.facing), (20, 0, 1));
        assert_eq!(actor.vel, Vec2::new(2.5, 0.0));
    }
    (game, boss, tomb)
}

// Original moveTo.gml uses stored velocity's fractional reciprocal and oGame.time.
// Use the actual stored f64 result and clock; rational -3.4 does not justify
// assuming an even/odd period (its stored fractional part can be below .4).
fn source_displacement(velocity: f64, clock: u64) -> f64 {
    let fraction = velocity.abs().fract();
    let extra = if fraction == 0.0 {
        false
    } else {
        let period = (1.0 / fraction).round_ties_even() as u64;
        period != 0 && clock % period == 0
    };
    (velocity.abs().floor() + f64::from(extra)) * velocity.signum()
}

fn ordinary_fly_hit(rules: RulesPreset) -> (Game, u64) {
    let (mut game, boss, tomb) = room(rules, true);
    let tomb = tomb.unwrap();
    let mut first_fly = None;
    for _ in 0..28 {
        let before = game.world.entity(boss).unwrap().clone();
        assert_eq!((before.health, before.state), (10, 0));
        assert_eq!(before.pos, Vec2::new(120.0, 128.0));
        assert!(
            before.value("psychic_recover") > 0.0,
            "NO_COMPETING_PSYCHIC_CAST"
        );
        let predicted_first: Option<Entity> = first_fly.and_then(|id| {
            game.world.entity(id).map(|fly| {
                let mut copy = fly.clone();
                // Literal Fly own Step only. This local mask prediction never
                // steps a cloned Game or writes any actual world actor.
                copy.pos.x += copy.vel.x;
                copy.pos.y += copy.vel.y;
                copy.set_sprite("sFlyRight");
                copy
            })
        });
        game.step(&InputFrame::default());
        assert_eq!(game.player.health, 4, "NO_PLAYER_CONTACT_HIDING_ENEMY_HIT");
        assert_eq!(game.player.stun, 0);
        assert_eq!(game.player.invincible, 0);
        assert!(
            !game
                .world
                .entities
                .iter()
                .any(|e| matches!(e.object.as_str(), "oPsychicCreate" | "oPsychicWave"))
        );
        if first_fly.is_none() {
            if let Some(fly) = game
                .world
                .entities
                .iter()
                .find(|e| e.alive && e.object == "oFly")
            {
                assert_eq!(
                    game.world.entity(tomb).unwrap().state,
                    3,
                    "NATURAL_TOMB_ATTACK_PRODUCER"
                );
                assert_eq!(fly.vel.x, 5.0, "SOURCE_FIRST_FLY_CALLER_LAUNCH");
                assert_eq!(
                    fly.vel.y,
                    0.5 - 3.0 * (134775814.0 / 4294967296.0),
                    "SOURCE_FIRST_FLY_CREATE"
                );
                first_fly = Some(fly.id);
            }
        }
        let actor = game.world.entity(boss).unwrap();
        if actor.health < before.health {
            assert_eq!(
                (actor.health, actor.state),
                (8, 98),
                "ACTUAL_SINGLE_FLY_ENEMY_COLLISION"
            );
            assert_eq!(actor.pos, Vec2::new(120.0, 128.0));
            assert_eq!(
                actor.vel,
                Vec2::new(5.0, -4.0),
                "ACTUAL_FLY_KNOCKBACK_BEFORE_NEXT_BOSS_STEP"
            );
            let first = first_fly.expect("NATURAL_FIRST_FLY_BIRTH_PRECEDES_HIT");
            let predicted = predicted_first.expect("LIVE_FIRST_FLY_BEFORE_ACTUAL_HIT");
            assert!(
                predicted.event_intersects(&before),
                "SOURCE_SELECTED_FRAME_FIRST_FLY_CONTACT"
            );
            assert!(
                game.world.entity(first).is_none(),
                "SOURCE_FIRST_FLY_RETIRED"
            );
            assert!(
                game.events
                    .iter()
                    .any(|e| matches!(e, GameEvent::Sound {name,..} if name == "sndHit"))
            );
            return (game, boss);
        }
        if let Some(predicted) = predicted_first {
            assert!(
                !predicted.event_intersects(&before),
                "DO_NOT_SKIP_EARLIER_PRECISE_FLY_CONTACT"
            );
        }
    }
    panic!("SOURCE_DEFAULT_TOMB_FLY_NEVER_REACHED_BOSS_WITHIN_BOUND");
}

#[test]
fn ordinary_tomb_fly_knockback_leaves_the_floor_while_alien_boss_is_stunned() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, boss) = ordinary_fly_hit(rules);
        let before = game.world.entity(boss).unwrap().clone();
        assert!(before.pos.x > game.camera.x - 36.0 && before.pos.x < game.camera.x + 320.0);
        assert!(before.pos.y > game.camera.y - 36.0 && before.pos.y < game.camera.y + 240.0);
        let next_clock = game.engine_time() + 1;
        let source_vx = before.vel.x - 0.1;
        let source_vy = (before.vel.y + 0.6).min(10.0);
        let expected = Vec2::new(
            before.pos.x + source_displacement(source_vx, next_clock),
            before.pos.y + source_displacement(source_vy, next_clock),
        );
        assert_eq!(expected.x, 125.0);
        assert!(expected.y == 124.0 || expected.y == 125.0);
        game.step(&InputFrame::default());
        assert_eq!(game.engine_time(), next_clock);
        let actor = game.world.entity(boss).unwrap();
        assert_eq!((actor.health, actor.state), (8, 98));
        assert_eq!(actor.vel.x, source_vx);
        assert_eq!(
            actor.vel.y, source_vy,
            "SOURCE_ALIEN_BOSS_STUNNED_BOTTOM_DOES_NOT_ZERO_UPWARD_KNOCKBACK"
        );
        assert!(actor.vel.y < 0.0);
        assert!(actor.pos.y < before.pos.y);
        // Supplemental stored-f64/source-clock integer displacement oracle.
        assert_eq!(actor.pos, expected);
        assert_eq!(game.player.health, 4);
        assert_eq!(
            actor.value("psychic_recover"),
            before.value("psychic_recover") - 1.0
        );
    }
}

#[test]
fn ordinary_healthy_alien_boss_still_settles_on_the_floor_without_a_fly() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, boss, tomb) = room(rules, false);
        assert!(tomb.is_none());
        for tick in 1..=28 {
            game.step(&InputFrame::default());
            let actor = game.world.entity(boss).unwrap();
            assert_eq!((actor.health, actor.state), (10, 0));
            assert_eq!(actor.pos, Vec2::new(120.0, 128.0));
            assert_eq!(actor.vel, Vec2::default());
            assert_eq!(actor.sprite, "sAlienBoss");
            assert_eq!(actor.value("psychic_recover"), 100.0 - f64::from(tick));
            assert_eq!(game.player.health, 4);
            assert!(!game.world.entities.iter().any(|e| matches!(
                e.object.as_str(),
                "oFly" | "oPsychicCreate" | "oPsychicWave"
            )));
        }
    }
}
