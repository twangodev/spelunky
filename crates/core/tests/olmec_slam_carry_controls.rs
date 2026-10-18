//! Public inputs release Olmec's SLAM carry latch after leaving his head.
//! Only the boss's initial source phase is controlled; Player state, support,
//! PREPARE expiry, and jumping are established by unmodified Game::step calls.
use spelunky_core::*;

fn initial_phase(rules: RulesPreset, state: i32, timer: u32, supported: bool) -> (Game, u64) {
    let mut level = level::Level::default();
    level.set_tile(9, 6, '@').unwrap();
    let floor_row = if supported { 11 } else { 19 };
    for x in 0..40 {
        level.set_tile(x, floor_row, '1').unwrap();
    }
    let mut game = Game::shell(42, rules);
    game.load_custom(&level, "olmec-slam-head-departure", "");
    assert_eq!(game.player.pos, Vec2::new(168.0, 120.0));
    let id = game.spawn("oOlmec", Vec2::new(144.0, 128.0));
    // Source Step's PREPARE, IDLE, or second-contact SLAM phase, before the
    // first Step. No Player fields or post-start actor/terrain fields are set.
    let boss = game.world.entity_mut(id).unwrap();
    boss.set_sprite("sOlmec");
    boss.state = state;
    boss.timer = timer;
    if state == 5 {
        // Source RECOVER -> PREPARE sets myGrav = 0 alongside this phase.
        boss.data.insert("my_gravity".into(), 0.0);
    }
    if supported {
        assert_eq!(state, 6);
        boss.data.insert("slammed".into(), 1.0);
    }
    assert_eq!(boss.vel, Vec2::default());
    assert_eq!(boss.value("carryPlayer"), 0.0);
    (game, id)
}

fn diagnostic(game: &Game, id: u64) -> String {
    let boss = game.world.entity(id).unwrap();
    format!(
        "rules={:?} tick={} time={} player={:?}/{:?}/{:?} ground={} hp={} stun={} boss={:?}/{:?} state={} counter={} carry={}",
        game.rules,
        game.tick,
        game.engine_time(),
        game.player.pos,
        game.player.vel,
        game.player.state,
        game.player.grounded,
        game.player.health,
        game.player.stun,
        boss.pos,
        boss.vel,
        boss.state,
        boss.timer,
        boss.value("carryPlayer")
    )
}

fn near(actual: f64, expected: f64, label: &str, game: &Game, id: u64) {
    assert!(
        (actual - expected).abs() < 1e-9,
        "{label}: expected={expected} actual={actual}; {}",
        diagnostic(game, id)
    );
}

fn healthy(game: &Game, id: u64) {
    assert_eq!(game.state, RunState::Playing, "{}", diagnostic(game, id));
    assert_eq!(game.player.health, 4.0, "{}", diagnostic(game, id));
    assert!(!game.player.is_stunned(), "{}", diagnostic(game, id));
}

fn fresh_jump(game: &mut Game) {
    game.step(&InputFrame {
        jump: true,
        jump_pressed: true,
        ..Default::default()
    });
}

fn observed_jump(game: &Game, id: u64) {
    healthy(game, id);
    assert!(!game.player.grounded, "{}", diagnostic(game, id));
    near(game.player.vel.y, -4.0, "FRESH_JUMP_VELOCITY", game, id);
    assert!(
        game.events
            .iter()
            .any(|event| matches!(event, GameEvent::Sound { name, .. } if name == "sndJump"))
    );
}

// Literal numeric portion of Platform Engine/moveTo.gml: the time sample is
// independent of the production moving-solid and Olmec displacement helpers.
fn source_displacement(velocity: f64, time: u64) -> f64 {
    let fraction = velocity.abs().fract();
    let mut integer = 0.0;
    if fraction != 0.0 {
        let divisor = (1.0 / fraction).round_ties_even();
        if divisor != 0.0 && (time as f64) % divisor == 0.0 {
            integer = 1.0;
        }
    }
    integer += velocity.abs().floor();
    if velocity < 0.0 {
        integer *= -1.0;
    }
    integer.round_ties_even()
}

#[test]
fn fresh_jump_after_natural_prepare_expiry_is_not_dragged_by_old_slam_contact() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, id) = initial_phase(rules, 5, 20, false);
        for remaining in (0..20).rev() {
            game.step(&InputFrame::default());
            let boss = game.world.entity(id).unwrap();
            assert_eq!(boss.state, 5, "{}", diagnostic(&game, id));
            assert_eq!(boss.timer, remaining);
            healthy(&game, id);
        }
        assert!(game.player.grounded, "{}", diagnostic(&game, id));
        near(
            game.player.pos.y + 8.0,
            128.0,
            "ACTUAL_HEAD_SUPPORT",
            &game,
            id,
        );
        game.step(&InputFrame::default());
        let boss = game.world.entity(id).unwrap();
        assert_eq!(boss.state, 6, "{}", diagnostic(&game, id));
        near(boss.vel.y, 5.0, "PREPARE_EXPIRES", &game, id);
        // The source else-if does not fall through from PREPARE to SLAM.
        assert_eq!(boss.value("carryPlayer"), 1.0);
        let head_y = boss.pos.y;
        let player_y = game.player.pos.y;
        game.step(&InputFrame::default());
        let boss = game.world.entity(id).unwrap();
        assert_eq!(boss.state, 6, "{}", diagnostic(&game, id));
        assert_eq!(boss.value("slammed"), 0.0);
        near(boss.pos.y, head_y + 10.0, "GLOBAL_AND_OWN_MOVE", &game, id);
        near(
            game.player.pos.y,
            player_y + 10.0,
            "CONTACT_FRAME_CARRIES",
            &game,
            id,
        );
        near(
            game.player.pos.y + 8.0,
            boss.pos.y,
            "STILL_ON_HEAD",
            &game,
            id,
        );
        assert!(game.player.grounded);
        healthy(&game, id);
        let contact_latch = boss.value("carryPlayer");
        let before_boss = boss.pos;
        let incoming_y_velocity = boss.vel.y;
        let before_player = game.player.pos;
        let time = game.engine_time() + 1;
        assert_eq!(time, 24);
        near(
            incoming_y_velocity,
            5.2,
            "RAW_FRACTIONAL_VELOCITY",
            &game,
            id,
        );
        let global_carry = source_displacement(incoming_y_velocity, time);
        assert_eq!(global_carry, 5.0);
        // After global carry and the real jump, even the lower Player sprite
        // edge is above the own-Step early rectangle. Only a stale latch can
        // now add raw boss velocity to the Player's origin.
        assert!(before_player.y + global_carry - 4.0 + 8.0 < before_boss.y + global_carry);
        fresh_jump(&mut game);
        observed_jump(&game, id);
        let boss = game.world.entity(id).unwrap();
        assert_eq!(boss.state, 6);
        near(
            boss.pos.y,
            before_boss.y + 2.0 * global_carry,
            "BOSS_STILL_FALLS",
            &game,
            id,
        );
        near(
            game.player.pos.x,
            before_player.x,
            "NO_HORIZONTAL_INPUT",
            &game,
            id,
        );
        near(
            game.player.pos.y,
            before_player.y + global_carry - 4.0,
            "DEPARTURE_MUST_NOT_ADD_PREVIOUS_SLAM_RAW_CARRY",
            &game,
            id,
        );
        assert_eq!(contact_latch, 0.0, "SLAM cleared the contact-frame latch");
        assert_eq!(boss.value("carryPlayer"), 0.0);
    }
}

#[test]
fn idle_departure_keeps_the_sources_previous_contact_raw_carry() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, id) = initial_phase(rules, 0, 60, false);
        game.step(&InputFrame::default());
        healthy(&game, id);
        assert!(game.player.grounded);
        let boss = game.world.entity(id).unwrap();
        assert_eq!(boss.state, 0);
        assert_eq!(boss.value("carryPlayer"), 1.0);
        near(boss.vel.y, 0.2, "IDLE_INCOMING_RAW_CARRY", &game, id);
        let before_player = game.player.pos;
        let before_boss = boss.pos;
        assert_eq!(source_displacement(0.2, game.engine_time() + 1), 0.0);
        fresh_jump(&mut game);
        observed_jump(&game, id);
        let boss = game.world.entity(id).unwrap();
        assert_eq!(boss.state, 0);
        assert_eq!(boss.pos, before_boss);
        assert_eq!(boss.value("carryPlayer"), 0.0);
        near(
            game.player.pos.y,
            before_player.y - 4.0 + 0.2,
            "IDLE_RAW_CARRY_PRESERVED",
            &game,
            id,
        );
    }
}

#[test]
fn supported_second_slam_contact_clears_latch_even_when_leaving_slam_state() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, id) = initial_phase(rules, 6, 0, true);
        game.step(&InputFrame::default());
        healthy(&game, id);
        assert!(game.player.grounded, "{}", diagnostic(&game, id));
        let boss = game.world.entity(id).unwrap();
        assert!(matches!(boss.state, 0 | 7), "{}", diagnostic(&game, id));
        assert_eq!(boss.timer, 60);
        assert_eq!(boss.vel, Vec2::default());
        assert_eq!(boss.pos, Vec2::new(144.0, 128.0));
        near(
            game.player.pos.y + 8.0,
            boss.pos.y,
            "SECOND_CONTACT_HEAD_SUPPORT",
            &game,
            id,
        );
        assert_eq!(
            boss.value("carryPlayer"),
            0.0,
            "SLAM clear precedes its grounded transition"
        );
    }
}
