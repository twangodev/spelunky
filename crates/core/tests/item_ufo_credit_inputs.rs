//! Authored quiet room, source-default healthy Player/UFO and ordinary Rock
//! pickup/Up-throw. Credit policies are initial configured controls only.
use spelunky_core::{Area, Game, GameEvent, InputFrame, RulesPreset, RunState, Vec2, World};

#[derive(Clone, Copy, Debug)]
enum CreditPolicy {
    Default,
    CountsFalse,
    Environmental,
}

fn healthy(game: &Game) {
    assert_eq!(game.state, RunState::Playing);
    assert_eq!(game.player.health, 4.0);
    assert_eq!(game.player.invincible, 0);
    assert_eq!(game.player.stun, 0);
    assert!(!game.player.is_stunned());
    assert_eq!(game.player.pos, Vec2::new(136.0, 184.0));
    assert_eq!(game.player.vel, Vec2::default());
}

fn source_displacement(v: f64, time: u64) -> f64 {
    let fraction = v.abs().fract();
    let extra =
        fraction > 0.0 && time.is_multiple_of((1.0 / fraction).round_ties_even().max(1.0) as u64);
    f64::from(v.abs().floor() as i32 + i32::from(extra)) * v.signum()
}

fn ordinary_rock_route(rules: RulesPreset, policy: CreditPolicy) {
    let mut game = Game::shell(16, rules);
    game.world = World::new(30, 20, Area::Ice, 9);
    game.world.name = "rLevel".into();
    for x in 0..30 {
        game.world.set_tile(x, 12, "oHardBlock");
    }
    // All authored placements/policy writes precede the first input. Default
    // Falling/zero-velocity Player must establish support naturally.
    game.player.pos = Vec2::new(136.0, 184.0);
    game.player.previous_pos = game.player.pos;
    let ufo = game.spawn("oUFO", Vec2::new(184.0, 166.0));
    let target = game.world.entity_mut(ufo).unwrap();
    assert_eq!(target.health, 1);
    assert_eq!(target.state, 0);
    assert_eq!(target.vel, Vec2::default());
    // Literal first width-2 Create draw for fixed seed16 selects leftward
    // SEARCH. This is not a gameplay-outcome seed selector.
    assert_eq!(target.value("shift_toggle"), 1.0);
    assert_eq!(target.value("counts_as_kill"), 1.0);
    assert_eq!(target.value("environmental_death"), 0.0);
    match policy {
        CreditPolicy::Default => {}
        CreditPolicy::CountsFalse => {
            target.data.insert("counts_as_kill".into(), 0.0);
        }
        CreditPolicy::Environmental => {
            target.data.insert("environmental_death".into(), 1.0);
        }
    }
    let rock = game.spawn("oRock", Vec2::new(136.0, 188.0));
    assert_eq!(game.world.entity(rock).unwrap().value("heavy"), 0.0);
    for _ in 0..4 {
        game.step(&InputFrame::default());
        healthy(&game);
        assert_eq!(game.world.entity(ufo).unwrap().health, 1);
    }
    assert!(game.player.grounded);
    assert_eq!(game.player.facing, 1);
    game.step(&InputFrame {
        down: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    healthy(&game);
    assert_eq!(game.player.held, Some(rock), "ordinary Down+Attack pickup");
    game.step(&InputFrame {
        attack_released: true,
        ..Default::default()
    });
    healthy(&game);
    assert_eq!(game.player.held, Some(rock));
    game.step(&InputFrame {
        up: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    healthy(&game);
    assert_eq!(game.player.held, None, "ordinary Up+Attack throw");
    let thrown = game.world.entity(rock).unwrap();
    assert!(!thrown.held);
    assert_eq!(thrown.vel.x, 8.0);
    assert!(thrown.vel.y < -8.0);
    assert!(
        game.world.entity(ufo).is_some(),
        "must not bypass the approaching-hit phase"
    );
    assert_eq!(game.data.get("ufos").copied().unwrap_or(0.0), 0.0);

    let mut completed = false;
    for frame in 0..20 {
        let item = game
            .world
            .entity(rock)
            .expect("ordinary Rock must remain live");
        let target = game
            .world
            .entity(ufo)
            .expect("no unobserved preceding UFO death");
        assert_eq!(target.health, 1);
        assert_eq!(target.value("invincible"), 0.0);
        assert!(item.vel.x.abs() > 2.0 || item.vel.y.abs() > 2.0);
        let clock = game.engine_time().wrapping_add(1) % 100_000_001;
        let next = Vec2::new(
            item.pos.x + source_displacement(item.vel.x, clock),
            item.pos.y + source_displacement(item.vel.y, clock),
        );
        // Rock450 runs before UFO518. Its inherited raw bbox rectangle reads
        // this live pre-UFO-Step target. The authored tube contains no Solid.
        assert!(!game.world.solid_at(next.x, next.y));
        let expected_contact = game.world.rectangle_family_except(
            Vec2::new(next.x - 2.0, next.y - 2.0),
            Vec2::new(next.x + 2.0, next.y + 2.0),
            "oUFO",
            None,
            spelunky_core::CollisionPrecision::BoundingBox,
        );
        let birth_origin = Vec2::new(target.pos.x + 8.0, target.pos.y + 8.0);
        let first_birth = game.world.next_id;
        game.step(&InputFrame {
            attack_released: frame == 0,
            ..Default::default()
        });
        healthy(&game);
        assert_eq!(
            game.world.entity(rock).unwrap().pos,
            next,
            "literal unobstructed Item moveTo must precede its creature query"
        );
        if game.world.entity(ufo).is_none() {
            assert!(expected_contact, "REAL_ORDINARY_ITEM_UFO_CONTACT required");
            let births = game
                .world
                .entities
                .iter()
                .filter(|e| e.id >= first_birth)
                .collect::<Vec<_>>();
            let explosion = births
                .iter()
                .find(|e| e.object == "oExplosion")
                .expect("source item branch births Explosion before retirement");
            assert_eq!(explosion.pos, birth_origin);
            assert_eq!(explosion.value("announced"), 1.0);
            assert!(
                !births.iter().any(|e| e.object == "oUFOCrash"),
                "this must be the item branch, not the later HP-death producer"
            );
            if let Some(eject) = births.iter().find(|e| e.object == "oAlienEject") {
                assert!(
                    explosion.id < eject.id,
                    "synchronous Explosion then optional Eject"
                );
            }
            let sounds = game
                .events
                .iter()
                .filter(|event| {
                    matches!(event,
                GameEvent::Sound { name, .. } if name == "sndExplosion")
                })
                .count();
            assert_eq!(sounds, 2, "Explosion Create sound then caller sound");
            assert_eq!(
                game.data.get("ufos").copied().unwrap_or(0.0),
                1.0,
                "SOURCE_ITEM_UFO_UNCONDITIONAL_CREDIT {policy:?}"
            );
            assert_eq!(game.data.get("kills").copied().unwrap_or(0.0), 1.0);
            assert_eq!(game.lifetime_stats.enemy_kills[16], 1);
            assert_eq!(game.player.bombs, 4.0);
            assert_eq!(game.player.ropes, 4.0);
            println!(
                "REAL_ITEM_UFO policy={policy:?} rules={rules:?} tick={} origin={birth_origin:?}",
                game.tick
            );
            completed = true;
            break;
        }
        assert!(
            !expected_contact,
            "qualified source contact cannot leave UFO alive"
        );
        assert_eq!(game.data.get("ufos").copied().unwrap_or(0.0), 0.0);
    }
    assert!(
        completed,
        "fixed ordinary input route must reach its mandatory live UFO contact"
    );
}

#[test]
fn healthy_ordinary_rock_throw_credits_default_ufo() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        ordinary_rock_route(rules, CreditPolicy::Default);
    }
}

#[test]
fn healthy_ordinary_rock_throw_uses_literal_ufo_credit_despite_initial_credit_guards() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for policy in [CreditPolicy::CountsFalse, CreditPolicy::Environmental] {
            ordinary_rock_route(rules, policy);
        }
    }
}
