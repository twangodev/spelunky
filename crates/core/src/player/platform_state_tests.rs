//! Configured lifecycle controls; ordinary reachability is tested separately.
use super::*;
use crate::{Area, InputFrame, RulesPreset, World};

fn room() -> Game {
    let mut game = Game::shell(42, RulesPreset::Classic);
    game.world = World::new(20, 15, Area::Mines, 1);
    game.world.name = "rLevel".into();
    game
}

#[test]
fn configured_inactive_timer_only_stun_retains_incoming_airborne_state() {
    let mut game = room();
    game.player.state = PlayerState::Jumping;
    game.player.stun = 3;
    game.player.vel.y = -3.0;
    game.player.set_sprite("sJumpLeft");
    game.player.data.insert("active".into(), 0.0);
    game.step(&InputFrame::default());
    assert_eq!(game.player.state, PlayerState::Stunned);
    assert_eq!(game.player.stun, 3);
    assert_eq!(game.player.data.get(KEY), Some(&15.0));
    assert!(source_airborne(&game));
    assert!(!source_grounded(&game));
}

#[test]
fn configured_stun_uses_literal_active_only_standing_assignment() {
    for active in [false, true] {
        let mut game = room();
        game.world.set_tile(2, 2, "oLadder");
        game.player.state = PlayerState::Climbing;
        game.player.stun = 3;
        game.player.set_sprite("sStunL");
        game.player.data.insert("active".into(), f64::from(active));
        enter_stunned(&mut game);
        game.step(&InputFrame::default());
        assert!(game.player.is_stunned());
        assert_eq!(game.player.stun, if active { 2 } else { 3 });
        assert_eq!(
            game.player.data.get(KEY),
            Some(&if active { 10.0 } else { 14.0 })
        );
        assert_eq!(source_climbing(&game), !active);
        assert_eq!(source_grounded(&game), active);
        assert!(!source_airborne(&game));
    }
}

#[test]
fn configured_duck_recovery_survives_clone_and_save() {
    let mut game = room();
    game.player.pos = crate::Vec2::new(104.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game.world.set_tile(6, 8, "oHardBlock");
    game.world.set_tile(6, 10, "oHardBlock");
    game.player.state = PlayerState::Ducking;
    game.player.stun = 1;
    game.player.set_sprite("sStunL");
    enter_stunned(&mut game);
    let restored: Game = serde_json::from_str(&serde_json::to_string(&game).unwrap()).unwrap();
    for mut branch in [game.clone(), restored] {
        assert!(source_ducking(&branch));
        assert!(branch.player.is_stunned());
        branch.step(&InputFrame {
            down: true,
            ..Default::default()
        });
        assert_eq!(branch.player.state, PlayerState::Ducking);
        assert_eq!(branch.player.stun, 0);
        assert!(!branch.player.is_stunned());
        assert!(source_ducking(&branch));
        assert_eq!(branch.player.vel, crate::Vec2::default());
    }
}

#[test]
fn configured_zero_timer_throw_then_dth_animation_keeps_independent_stun() {
    let mut game = room();
    game.player.state = PlayerState::Ducking;
    game.player.set_sprite("sDuckToHangL");
    game.player.data.insert("duck_to_hang".into(), 1.0);
    enter_stunned(&mut game);
    crate::player::animation_end(&mut game);
    assert_eq!(game.player.stun, 0);
    assert!(game.player.is_stunned());
    assert!(source_hanging(&game));
    assert!(!source_grounded(&game));
    assert_eq!(game.player.data.get(KEY), Some(&23.0));
    assert_eq!(game.player.data.get("duck_to_hang"), Some(&0.0));
    let restored: Game = serde_json::from_str(&serde_json::to_string(&game).unwrap()).unwrap();
    assert!(restored.player.is_stunned());
    assert!(source_hanging(&restored));
}

#[test]
fn configured_interrupted_dth_end_step_writes_standing_without_clearing_stun() {
    let mut game = room();
    game.player.state = PlayerState::Ducking;
    game.player.set_sprite("sDuckToHangL");
    game.player.data.insert("duck_to_hang".into(), 1.0);
    enter_stunned(&mut game);
    game.step(&InputFrame::default());
    assert!(game.player.is_stunned());
    assert_eq!(game.player.stun, 0);
    assert_eq!(game.player.data.get(KEY), Some(&10.0));
    assert_eq!(game.player.data.get("duck_to_hang"), Some(&0.0));
    assert!(source_grounded(&game));
    assert!(!source_hanging(&game));
}

#[test]
fn old_canonical_class_only_save_does_not_invent_subtypes() {
    let mut game = room();
    game.player.state = PlayerState::Stunned;
    game.player
        .data
        .insert("source_platform_grounded".into(), 1.0);
    game.player
        .data
        .insert("source_platform_climbing".into(), 0.0);
    let restored: Game = serde_json::from_str(&serde_json::to_string(&game).unwrap()).unwrap();
    assert!(source_grounded(&restored));
    assert!(!source_airborne(&restored));
    assert!(!source_ducking(&restored));
    assert!(!source_hanging(&restored));
    assert!(!restored.player.data.contains_key(KEY));
    assert_eq!(game.player.data, restored.player.data);
}

#[test]
fn historical_collapse_does_not_add_serialized_backing_or_stun_flag() {
    let mut game = Game::legacy_shell(42, RulesPreset::Classic);
    game.player.state = PlayerState::Jumping;
    let data = game.player.data.clone();
    enter_stunned(&mut game);
    assert_eq!(game.player.state, PlayerState::Stunned);
    assert!(!source_airborne(&game));
    assert_eq!(game.player.data, data);
    enter_dead(&mut game);
    assert_eq!(game.player.state, PlayerState::Dead);
    assert_eq!(game.player.data, data);
}
