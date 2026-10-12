//! Source AlienBoss moveTo preserves velocity at an actual ceiling collision.
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
    // Initial authored roof only; no terrain or actor edits after gameplay starts.
    game.world.set_tile(7, 7, "oHardBlock");
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

// The selected-mask contact checks use only local copies; actual actors stay untouched.
fn no_existing_fly_contact_after_literal_motion(game: &Game, projected_boss: &Entity) {
    for fly in game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oFly")
    {
        let mut copy = fly.clone();
        copy.pos.x += copy.vel.x;
        copy.pos.y += copy.vel.y;
        copy.set_sprite("sFlyRight");
        assert!(
            !copy.event_intersects(projected_boss),
            "NO_COMPETING_EXISTING_FLY_AT_ROOF_CONTINUATION"
        );
    }
    // Tomb's new Fly starts at109 then advances only3..5 to112..114;
    // its bboxright <=117 cannot reach these Boss x125/130 masks.
}

fn top_line_at_x(actor: &Entity, x: f64) -> (Vec2, Vec2) {
    let mut copy = actor.clone();
    copy.pos.x = x;
    let b = copy.bounds();
    (
        Vec2::new(b.x.round_ties_even(), (b.y - 1.0).round_ties_even()),
        Vec2::new(
            (b.x + b.w - 1.0).round_ties_even(),
            (b.y - 1.0).round_ties_even(),
        ),
    )
}

#[test]
fn ordinary_tomb_fly_roof_collision_preserves_velocity_then_boss_rises_past_roof() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, boss) = ordinary_fly_hit(rules);
        let before = game.world.entity(boss).unwrap().clone();
        assert_eq!(before.pos, Vec2::new(120.0, 128.0));
        assert_eq!((before.health, before.state), (8, 98));
        assert_eq!(before.vel, Vec2::new(5.0, -4.0));
        assert!(
            !before.data.contains_key("inherited_gravity"),
            "BOSS_OWN_STEP_DOES_NOT_CALL_PARENT_GRAVITY"
        );
        assert!(game.world.tile(7, 7).unwrap().engine_active);
        assert_eq!(game.world.tile(7, 7).unwrap().object, "oHardBlock");
        assert!(
            !game.world.solid_point_except(Vec2::new(128.0, 136.0), boss),
            "ROOF_DID_NOT_CRUSH_ACTOR"
        );
        assert!(game.world.solid_line_except(
            Vec2::new(120.0, 160.0),
            Vec2::new(151.0, 160.0),
            boss
        ));
        assert!(before.alive && before.engine_active);
        assert!(before.pos.x > game.camera.x - 36.0 && before.pos.x < game.camera.x + 320.0);
        assert!(before.pos.y > game.camera.y - 36.0 && before.pos.y < game.camera.y + 240.0);
        let clock1 = game.engine_time() + 1;
        let vx1 = before.vel.x - 0.1;
        let vy1 = (before.vel.y + 0.6).min(10.0);
        assert!(source_displacement(vy1, clock1) <= -3.0);
        let x1 = before.pos.x + source_displacement(vx1, clock1);
        assert_eq!(x1, 125.0);
        let (first, last) = top_line_at_x(&before, x1);
        assert_eq!(
            (first, last),
            (Vec2::new(125.0, 127.0), Vec2::new(156.0, 127.0))
        );
        assert!(
            game.world.solid_line_except(first, last, boss),
            "FIRST_SOURCE_ROOF_COLLISION_AFTER_HORIZONTAL_MOTION"
        );
        assert!(!game.world.solid_line_except(
            Vec2::new(125.0, 128.0),
            Vec2::new(156.0, 128.0),
            boss
        ));
        let mut roof_pose = before.clone();
        roof_pose.pos = Vec2::new(x1, before.pos.y);
        no_existing_fly_contact_after_literal_motion(&game, &roof_pose);
        game.step(&InputFrame::default());
        assert_eq!(game.engine_time(), clock1);
        let at_roof = game.world.entity(boss).unwrap();
        assert_eq!((at_roof.health, at_roof.state), (8, 98));
        assert_eq!(
            at_roof.pos,
            Vec2::new(125.0, 128.0),
            "ACTUAL_UPWARD_MOVE_STOPPED_AT_INITIAL_ROOF"
        );
        assert_eq!(at_roof.vel.x, vx1);
        assert_eq!(
            at_roof.vel.y, vy1,
            "SOURCE_ALIEN_BOSS_MOVETO_CEILING_BREAK_PRESERVES_STORED_VELOCITY"
        );
        assert_eq!(
            at_roof.value("psychic_recover"),
            before.value("psychic_recover") - 1.0
        );
        assert_eq!(game.player.health, 4);
        let before_escape = at_roof.clone();
        assert!(before_escape.alive && before_escape.engine_active);
        assert!(
            before_escape.pos.x > game.camera.x - 36.0
                && before_escape.pos.x < game.camera.x + 320.0
        );
        assert!(
            before_escape.pos.y > game.camera.y - 36.0
                && before_escape.pos.y < game.camera.y + 240.0
        );
        let clock2 = game.engine_time() + 1;
        let vx2 = before_escape.vel.x - 0.1;
        let vy2 = (before_escape.vel.y + 0.6).min(10.0);
        let x2 = before_escape.pos.x + source_displacement(vx2, clock2);
        assert_eq!(x2, 130.0);
        let (first, last) = top_line_at_x(&before_escape, x2);
        assert_eq!(
            (first, last),
            (Vec2::new(130.0, 127.0), Vec2::new(161.0, 127.0))
        );
        assert!(
            !game.world.solid_line_except(first, last, boss),
            "SECOND_SOURCE_STEP_CLEARS_ROOF_AFTER_HORIZONTAL_MOTION"
        );
        let expected = Vec2::new(x2, before_escape.pos.y + source_displacement(vy2, clock2));
        assert_eq!(expected, Vec2::new(130.0, 125.0));
        let mut escaped_pose = before_escape.clone();
        escaped_pose.pos = expected;
        no_existing_fly_contact_after_literal_motion(&game, &escaped_pose);
        game.step(&InputFrame::default());
        assert_eq!(game.engine_time(), clock2);
        let escaped = game.world.entity(boss).unwrap();
        assert_eq!((escaped.health, escaped.state), (8, 98));
        assert_eq!(
            escaped.pos, expected,
            "PRESERVED_ROOF_VELOCITY_REACHES_NEXT_SOURCE_UPWARD_STEP"
        );
        assert_eq!(escaped.vel, Vec2::new(vx2, vy2));
        assert_eq!(
            escaped.value("psychic_recover"),
            before_escape.value("psychic_recover") - 1.0
        );
        assert_eq!(game.player.health, 4);
        assert!(game.world.tile(7, 7).unwrap().engine_active);
    }
}

#[test]
fn ordinary_healthy_alien_boss_still_settles_below_the_authored_roof_without_a_fly() {
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
