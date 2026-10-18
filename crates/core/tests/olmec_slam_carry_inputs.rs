//! Natural SLAM creates a new contact after a late ordinary jump.
//! Restored Slam floor destruction leaves this jump below the boss at entry.
//! Ordinary Right approaches, a neutral PREPARE pause, and a late held jump
//! follow the initial public level setup; no actor or Player state is injected.
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
fn right() -> InputFrame {
    InputFrame {
        right: true,
        ..Default::default()
    }
}
fn healthy(game: &Game, boss: u64) {
    let e = game.world.entity(boss).unwrap();
    assert_eq!(
        game.state,
        RunState::Playing,
        "boss={:?}/{:?}/{} carry={}",
        e.pos,
        e.vel,
        e.state,
        e.value("carryPlayer")
    );
    assert_eq!(
        game.player.health,
        4.0,
        "NATURAL_HEALTH_PREREQUISITE player={:?}/{:?} boss={:?}/{:?}/{} carry={}",
        game.player.pos,
        game.player.vel,
        e.pos,
        e.vel,
        e.state,
        e.value("carryPlayer")
    );
    assert!(
        !game.player.is_stunned(),
        "natural carry contact must keep player unstunned"
    );
}

// Literal isCollisionTop(1), with the current platform-controller bounds.
// This uses the stored source Draw transform, as the loaded object mask does.
fn head_probe(player: &Player) -> (Vec2, Vec2) {
    let bounds = player.bounds();
    let y = (bounds.y - 1.0).round_ties_even();
    (
        Vec2::new(bounds.x.round_ties_even(), y),
        Vec2::new((bounds.x + bounds.w - 1.0).round_ties_even(), y),
    )
}
fn boss_head_hit(boss: &Entity, player: &Player) -> bool {
    let (first, last) = head_probe(player);
    catalog::instance_mask(
        &boss.object,
        &boss.sprite,
        boss.pos,
        boss.animation,
        Vec2::new(
            boss.data.get("image_xscale").copied().unwrap_or(1.0),
            boss.data.get("image_yscale").copied().unwrap_or(1.0),
        ),
        -boss
            .data
            .get("image_angle")
            .copied()
            .unwrap_or(0.0)
            .to_radians(),
    )
    .intersects_axis_line(first, last, CollisionPrecision::Precise)
}

fn natural_active_slam_entry_after_late_jump(
    rules: RulesPreset,
    algorithm: RngAlgorithm,
) -> (Game, u64) {
    let mut game = Game::new_with_algorithm(42, rules, algorithm).unwrap();
    while game.world.level < 16 {
        game.next_level();
    }
    assert_eq!(game.world.area, Area::Boss);
    assert_eq!(game.player.pos, Vec2::new(24.0, 552.0));
    assert_eq!(game.player.data.get("active"), Some(&0.0));
    let boss = game
        .world
        .entities
        .iter()
        .find(|e| e.object == "oOlmec")
        .unwrap()
        .id;
    let fresh = game.world.entity(boss).unwrap();
    assert_eq!((fresh.state, fresh.timer), (-1, 0));
    assert_eq!(fresh.vel, Vec2::default());
    assert_eq!(fresh.value("carryPlayer"), 0.0);
    let mut released = false;
    let mut natural_side_contact = false;
    let mut active_bounce = false;
    let mut active_prepare = false;
    let mut late_jump_observed = false;
    for tick in 1..=1200 {
        let before = game.world.entity(boss).unwrap().clone();
        let before_player = game.player.clone();
        if released && before.state == 0 && before.timer == 1 {
            assert_eq!(
                before.value("carryPlayer"),
                1.0,
                "natural Right approach creates side carry before the active bounce"
            );
            natural_side_contact = true;
        }
        let input = if released && before.state == 5 {
            if before.timer == 5 {
                assert!(
                    game.player.grounded,
                    "ordinary late PREPARE jump starts on the arena floor"
                );
                assert!(!late_jump_observed);
            }
            InputFrame {
                jump: before.timer <= 5,
                jump_pressed: before.timer == 5,
                ..Default::default()
            }
        } else {
            right()
        };
        game.step(&input);
        healthy(&game, boss);
        if released && before.state == 5 && before.timer == 5 {
            assert_eq!(
                game.player.vel.y, -4.0,
                "ordinary late jump executes source impulse"
            );
            assert!(
                game.events
                    .iter()
                    .any(|event| matches!(event,GameEvent::Sound{name,..} if name=="sndJump"))
            );
            late_jump_observed = true;
        }
        let after = game.world.entity(boss).unwrap();
        if released && before.state == 5 && input.jump {
            assert!(
                !boss_head_hit(&before, &before_player),
                "restored arena floor leaves the unchanged late jump below Olmec"
            );
            assert_eq!(game.player.data.get("colTop"), Some(&0.0));
        }
        if after.state != before.state
            || after.value("carryPlayer") != before.value("carryPlayer")
            || (released && before.state == 5 && before.timer <= 5)
        {
            eprintln!(
                "NATURAL {tick} {rules:?}/{algorithm:?} before={:?}/{:?}/{} timer{} carry{} after={:?}/{:?}/{} timer{} carry{} player={:?}/{:?} hp{} stun{}",
                before.pos,
                before.vel,
                before.state,
                before.timer,
                before.value("carryPlayer"),
                after.pos,
                after.vel,
                after.state,
                after.timer,
                after.value("carryPlayer"),
                game.player.pos,
                game.player.vel,
                game.player.health,
                game.player.stun
            );
            eprintln!(
                "GEOMETRY playerState={:?} playerBounds={:?} colTop={:?} bossBounds={:?} beforeFoot={} afterFoot={} jump={} pressed={}",
                game.player.state,
                game.player.sprite_bounds(),
                game.player.data.get("colTop"),
                after.sprite_bounds(),
                before.pos.y + 64.0,
                after.pos.y + 64.0,
                input.jump,
                input.jump_pressed
            );
        }
        if !released && game.player.data.get("active") == Some(&1.0) {
            assert_eq!((after.state, after.timer), (0, 99));
            released = true;
        }
        if released {
            active_bounce |= after.state == 1;
            active_prepare |= after.state == 5;
            if before.state == 5 && after.state == 6 {
                assert!(
                    natural_side_contact && active_bounce && active_prepare && late_jump_observed
                );
                assert_eq!(before.timer, 0);
                assert_eq!(after.vel.y, 5.0);
                assert_eq!(after.value("slammed"), 0.0);
                // PREPARE selects SLAM without running its body. The restored
                // floor leaves this late jump ascending below the boss; the
                // following own SLAM movement creates the new contact.
                assert_eq!(after.value("carryPlayer"), 0.0);
                assert_eq!(before_player.state, PlayerState::Jumping);
                assert!(before_player.vel.y < 0.0);
                assert_eq!(before_player.data.get("collision_top"), Some(&-8.0));
                assert!(!boss_head_hit(&before, &before_player));
                assert_eq!(game.player.data.get("colTop"), Some(&0.0));
                assert_eq!(game.player.state, PlayerState::Jumping);
                assert_eq!(game.player.vel.y, -2.5);
                assert_eq!(game.player.data.get("gravity_intensity"), Some(&0.6));
                assert!(
                    !Rect::new(after.pos.x, after.pos.y - 2.0, 65.0, 67.0)
                        .intersects(game.player.sprite_bounds()),
                    "entry Player is below the source carry rectangle"
                );
                return (game, boss);
            }
        }
    }
    panic!("natural active Olmec must reach a healthy contacted slam entry");
}
#[test]
fn natural_own_slam_clears_new_post_movement_contact_with_player_alive() {
    for (rules, algorithm) in modes() {
        let (mut game, boss) = natural_active_slam_entry_after_late_jump(rules, algorithm);
        let before = game.world.entity(boss).unwrap().clone();
        assert_eq!(
            before.state, 6,
            "source SLAM body executes only on own Step entering with state6"
        );
        assert_eq!(before.value("carryPlayer"), 0.0);
        game.step(&right());
        healthy(&game, boss);
        let after = game.world.entity(boss).unwrap();
        eprintln!(
            "OWN_SLAM {rules:?}/{algorithm:?} before={:?}/{:?}/{} carry{} after={:?}/{:?}/{} carry{} player={:?}/{:?} hp{}",
            before.pos,
            before.vel,
            before.state,
            before.value("carryPlayer"),
            after.pos,
            after.vel,
            after.state,
            after.value("carryPlayer"),
            game.player.pos,
            game.player.vel,
            game.player.health
        );
        assert_eq!(after.state, 6);
        assert_eq!(
            after.pos.y,
            before.pos.y + 10.0,
            "literal incoming5 global moving-solid plus own moveTo creates the new contact"
        );
        assert!(
            Rect::new(after.pos.x, after.pos.y - 2.0, 65.0, 67.0)
                .intersects(game.player.sprite_bounds()),
            "SOURCE_POST_MOVEMENT_CARRY_RECTANGLE_CONTACT_PREREQUISITE"
        );
        assert_eq!(
            after.value("carryPlayer"),
            0.0,
            "SOURCE_OLMEC_OWN_SLAM_UNCONDITIONAL_CARRY_CLEAR"
        );
    }
}
