//! Authored initial two-hazard room; ordinary-input recovery extension unrun.
//! not a stock seed: one ordinary jump crosses Magma then Tomb Lord callbacks.
use spelunky_core::*;

fn precise_overlap(player: &Player, entity: &Entity) -> bool {
    let a = player.sprite_bounds();
    let b = entity.sprite_bounds();
    let left = a.x.min(b.x).floor() as i32 - 2;
    let top = a.y.min(b.y).floor() as i32 - 2;
    let right = (a.x + a.w).max(b.x + b.w).ceil() as i32 + 2;
    let bottom = (a.y + a.h).max(b.y + b.h).ceil() as i32 + 2;
    (top..=bottom).any(|y| {
        (left..=right).any(|x| {
            let point = Vec2::new(f64::from(x), f64::from(y));
            player.mask_contains(point, CollisionPrecision::Precise)
                && entity.mask_contains(point, CollisionPrecision::Precise)
        })
    })
}

fn scenario(rules: RulesPreset, historical: bool, magma_present: bool) {
    let mut game = if historical {
        Game::legacy_shell(42, rules)
    } else {
        Game::shell(42, rules)
    };
    game.world = World::new(20, 15, Area::Mines, 1);
    game.world.name = "rLevel".into();
    for x in 0..20 {
        game.world.set_tile(x, 9, "oLadderTop");
        game.world.set_tile(x, 10, "oHardBlock");
    }
    game.player.pos = Vec2::new(104.0, 136.0);
    game.player.previous_pos = game.player.pos;
    let boss = game.spawn("oTombLord", Vec2::new(88.0, 128.0));
    let magma = magma_present.then(|| game.spawn("oMagma", Vec2::new(104.0, 123.0)));
    assert_eq!(game.player.health, 4);
    assert_eq!(game.player.stun, 0);
    assert_eq!(game.player.vel, Vec2::default());
    assert!(game.player.equipment.is_empty());
    assert_eq!(game.player.held, None);
    assert_eq!(game.world.entity(boss).unwrap().health, 20);
    // Only ordinary input follows the initial authored scene. Retain the
    // pre-Draw scale for a read-only reconstruction of first Collision masks.
    let collision_scale = game.player.data.get("image_xscale").copied().unwrap_or(1.0);
    game.step(&InputFrame {
        jump: true,
        jump_pressed: true,
        ..Default::default()
    });
    assert_eq!(game.state, RunState::Playing);
    assert_eq!(
        game.player.pos,
        Vec2::new(104.0, 132.0),
        "real supported jump"
    );
    let boss = game
        .world
        .entity(boss)
        .expect("actual default Tomb Lord survives");
    assert!(boss.engine_active && boss.alive);
    assert_eq!(boss.pos.y, 128.0);
    assert!((game.player.pos.x - (boss.pos.x + 16.0)).abs() <= 16.0);
    assert!(game.player.pos.y < boss.pos.y + 8.0);
    assert_eq!(
        game.player.data.get("swimming").copied().unwrap_or(0.0),
        0.0
    );
    let mut collision_player = game.player.clone();
    collision_player
        .data
        .insert("image_xscale".into(), collision_scale);
    assert!(collision_player.animation < 1.0 && boss.animation < 1.0);
    assert!(
        precise_overlap(&collision_player, boss),
        "actual same-frame Tomb Lord contact"
    );
    if let Some(id) = magma {
        let magma = game
            .world
            .entity(id)
            .expect("actual default Magma survives");
        assert!(magma.engine_active && magma.alive && magma.animation < 1.0);
        assert!(
            precise_overlap(&collision_player, magma),
            "actual first Magma contact"
        );
        // Original source aggregate boxes: Magma last Y is pos+5;
        // Tomb Lord Walk first Y is pos. Exclude its separate Magma-to-Enemy
        // burn callback, which could otherwise change the same HP oracle.
        assert!(matches!(
            boss.sprite.as_str(),
            "sTombLordWalkL" | "sTombLordWalkR"
        ));
        assert!(magma.pos.y + 5.0 < boss.pos.y);
        assert_eq!(boss.value("burning"), 0.0);
        assert_ne!(boss.state, 98);
        assert_eq!(game.player.health, 2, "actual Magma Damage2");
        assert_eq!(game.player.stun, 20);
        assert_eq!(game.player.invincible, 30);
        assert_eq!(game.player.data.get("burning"), Some(&100.0));
        assert!(
            game.events
                .iter()
                .any(|event| matches!(event, GameEvent::Damage { amount: 2 }))
        );
    } else {
        assert_eq!(game.player.health, 4);
        assert_eq!(game.player.stun, 0);
    }
    let expected = if historical && magma_present { 20 } else { 19 };
    assert_eq!(
        boss.health, expected,
        "SOURCE_MAGMA_STUN_RETAINS_AIRBORNE_FOR_LATER_TOMB_COLLISION rules={rules:?} historical={historical} magma={magma_present}"
    );
    assert_eq!(
        game.player.vel.y,
        if historical && magma_present {
            -4.0
        } else {
            -5.2
        }
    );
    if magma_present && !historical {
        assert_eq!(game.player.data.get("source_platform_state"), Some(&16.0));
        assert!(game.player.is_stunned());
        assert_eq!(game.player.data.get("bounced").copied().unwrap_or(0.0), 0.0);
        let incoming_velocity = game.player.vel;
        // Source Player Step338 assigns STANDING before its independent stun
        // motion. This is a logical state write, not a physical landing.
        game.step(&InputFrame::default());
        assert_eq!(game.player.health, 2);
        assert_eq!(game.state, RunState::Playing);
        assert_eq!(
            game.player.stun, 20,
            "incoming live-airborne art is not stationary stun art"
        );
        assert!(game.player.is_stunned());
        assert!(
            !game.player.grounded,
            "logical Standing is still physically airborne"
        );
        assert_eq!(game.player.vel.x, incoming_velocity.x);
        assert_eq!(game.player.vel.y, incoming_velocity.y + 0.6);
        assert_eq!(game.player.data.get("source_platform_state"), Some(&10.0));
        assert_eq!(game.player.data.get("source_platform_grounded"), Some(&1.0));
        assert_eq!(game.player.data.get("source_platform_climbing"), Some(&0.0));
    }
}

#[test]
fn ordinary_jump_magma_stun_keeps_same_frame_tomb_stomp() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        scenario(rules, false, true);
    }
}

#[test]
fn ordinary_jump_tomb_alone_qualifies_the_source_stomp() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        scenario(rules, false, false);
    }
}

#[test]
fn historical_same_frame_magma_contact_keeps_earlier_tomb_behavior() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        scenario(rules, true, true);
    }
}

// Separate initial authored scene: a default Magma supplies the real stun,
// without a Tomb Lord whose source attack can create another damaging Fly.
// Centering gives either literal +/-6 knockback room to land on this floor.
fn magma_recovery(rules: RulesPreset) {
    let mut game = Game::shell(42, rules);
    game.world = World::new(20, 15, Area::Mines, 1);
    game.world.name = "rLevel".into();
    for x in 0..20 {
        game.world.set_tile(x, 9, "oLadderTop");
        game.world.set_tile(x, 10, "oHardBlock");
    }
    game.player.pos = Vec2::new(160.0, 136.0);
    game.player.previous_pos = game.player.pos;
    let magma = game.spawn("oMagma", Vec2::new(160.0, 123.0));
    assert_eq!(game.player.health, 4);
    assert_eq!(game.player.stun, 0);
    assert_eq!(game.player.vel, Vec2::default());
    assert!(game.player.equipment.is_empty());
    assert_eq!(game.player.held, None);
    let collision_scale = game.player.data.get("image_xscale").copied().unwrap_or(1.0);
    game.step(&InputFrame {
        jump: true,
        jump_pressed: true,
        ..Default::default()
    });
    assert_eq!(game.state, RunState::Playing);
    assert_eq!(game.player.pos, Vec2::new(160.0, 132.0));
    let magma = game
        .world
        .entity(magma)
        .expect("actual default Magma survives its first contact");
    assert!(magma.alive && magma.engine_active && magma.animation < 1.0);
    let mut collision_player = game.player.clone();
    collision_player
        .data
        .insert("image_xscale".into(), collision_scale);
    assert!(collision_player.animation < 1.0);
    assert!(
        precise_overlap(&collision_player, magma),
        "actual Magma collision mask admission"
    );
    assert_eq!(game.player.health, 2, "actual Magma Damage2");
    assert_eq!(game.player.stun, 20);
    assert_eq!(game.player.invincible, 30);
    assert_eq!(game.player.data.get("burning"), Some(&100.0));
    assert!(
        game.events
            .iter()
            .any(|event| matches!(event, GameEvent::Damage { amount: 2 }))
    );
    assert_eq!(game.player.data.get("source_platform_state"), Some(&16.0));
    assert!(game.player.is_stunned());
    assert_eq!(
        game.player.vel.y, -4.0,
        "Magma supplies its literal knockback"
    );
    assert_eq!(game.player.vel.x.abs(), 6.0);
    assert_eq!(game.player.data.get("bounced").copied().unwrap_or(0.0), 0.0);
    let incoming_velocity = game.player.vel;
    game.step(&InputFrame::default());
    assert_eq!(game.player.health, 2);
    assert_eq!(game.state, RunState::Playing);
    assert_eq!(game.player.stun, 20);
    assert!(game.player.is_stunned());
    assert!(!game.player.grounded);
    assert_eq!(game.player.vel.x, incoming_velocity.x);
    assert_eq!(game.player.vel.y, incoming_velocity.y + 0.6);
    assert_eq!(game.player.data.get("source_platform_state"), Some(&10.0));
    assert_eq!(game.player.data.get("source_platform_grounded"), Some(&1.0));
    assert_eq!(game.player.data.get("source_platform_climbing"), Some(&0.0));
    let mut recovered = false;
    for _ in 0..120 {
        let incoming = game.player.clone();
        let half = incoming
            .data
            .get("collision_half_width")
            .copied()
            .unwrap_or(5.0);
        let top = incoming.data.get("collision_top").copied().unwrap_or(-8.0);
        let first = Vec2::new(
            (incoming.pos.x - half).round_ties_even(),
            (incoming.pos.y + 8.0).round_ties_even(),
        );
        let last = Vec2::new((incoming.pos.x + half - 1.0).round_ties_even(), first.y);
        let solid_bottom =
            game.world
                .tiles_line_precision(first, last, "oSolid", CollisionPrecision::Precise);
        let platform_bottom =
            game.world
                .tiles_line_precision(first, last, "oPlatform", CollisionPrecision::Precise);
        let platform_body = game.world.rectangle_family_except(
            Vec2::new(incoming.pos.x - half, incoming.pos.y + top),
            Vec2::new(incoming.pos.x + half - 1.0, incoming.pos.y + 7.0),
            "oPlatform",
            None,
            CollisionPrecision::Precise,
        );
        let dynamic_target = game.world.entities.iter().any(|entity| {
            entity.alive
                && (spelunky_core::catalog::is_a(&entity.object, "oSolid")
                    || spelunky_core::catalog::is_a(&entity.object, "oPlatform"))
        });
        game.step(&InputFrame::default());
        assert_eq!(
            game.state,
            RunState::Playing,
            "ordinary recovery stays alive"
        );
        assert_eq!(game.player.health, 2, "no unrelated recovery injury");
        assert_eq!(game.player.data.get("source_platform_state"), Some(&10.0));
        if !game.player.is_stunned() {
            assert_eq!(incoming.stun, 1, "actual stationary countdown reaches zero");
            assert_eq!(incoming.sprite, "sStunL");
            assert_eq!(incoming.data.get("source_platform_state"), Some(&10.0));
            assert_eq!(incoming.data.get("source_stunned"), Some(&1.0));
            assert_eq!(incoming.vel.x, 0.0);
            assert_eq!(incoming.data.get("gravity"), Some(&1.0));
            assert_eq!(incoming.data.get("gravity_intensity"), Some(&0.1));
            // Original Step clears the independent flag, restores STANDING,
            // then the SAME characterStepEvent sees no incoming support.
            // Its walk-off branch writes FALLING and adds grav=1, rather than
            // adding the stale airborne gravityIntensity=0.1.
            assert!(
                !dynamic_target,
                "this authored room's supports are tiles only"
            );
            assert!(!solid_bottom && !platform_bottom && !platform_body);
            assert_eq!(game.player.stun, 0);
            assert!(!game.player.data.contains_key("source_stunned"));
            assert_eq!(game.player.state, PlayerState::Falling);
            assert_eq!(game.player.vel.y, incoming.vel.y + 1.0);
            assert_eq!(game.player.data.get("source_platform_grounded"), Some(&0.0));
            assert!(
                game.player.grounded,
                "movement reaches the real platform after the cached query"
            );
            assert_eq!(game.player.pos.y, 136.0);
            assert_eq!(
                game.player.vel.x, 0.0,
                "stationary stun art enabled actual recovery"
            );

            // The next normal controller sees that support, settles, and
            // accepts a fresh ordinary jump. No pose or counter is injected.
            game.step(&InputFrame::default());
            assert_eq!(game.state, RunState::Playing);
            assert_eq!(game.player.health, 2);
            assert!(!game.player.is_stunned());
            assert_eq!(game.player.stun, 0);
            assert_eq!(game.player.state, PlayerState::Standing);
            assert!(game.player.grounded);
            assert_eq!(game.player.pos.y, 136.0);
            assert_eq!(game.player.vel, Vec2::default());
            let supported = game.player.pos;
            game.step(&InputFrame {
                jump: true,
                jump_pressed: true,
                ..Default::default()
            });
            assert_eq!(game.state, RunState::Playing);
            assert_eq!(game.player.health, 2);
            assert!(!game.player.is_stunned());
            assert_eq!(game.player.stun, 0);
            assert_eq!(game.player.vel.x, 0.0);
            assert_eq!(game.player.vel.y, -4.0);
            assert_eq!(game.player.pos, Vec2::new(supported.x, supported.y - 4.0));
            assert!(!game.player.grounded);
            recovered = true;
            break;
        }
    }
    assert!(recovered, "finite neutral inputs reach real timed recovery");
}

#[test]
fn ordinary_magma_stun_resumes_the_source_fall_controller_then_can_jump_again() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        magma_recovery(rules);
    }
}
