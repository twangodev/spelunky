//! Original unlockable player characters and Tunnel Man's innate mattock.
//!
//! Source: scrClearGlobals, oPlayer1 Step/Animation End, characterStepEvent,
//! characterSprite, and oMattockHit events in the Classic 1.1 reference.
use crate::model::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Character {
    #[default]
    Explorer,
    Damsel,
    TunnelMan,
}

/// Select a character at run start or a title-room changing door.
/// Unlock authorization belongs to the host; this also supports editor tests.
pub fn apply(game: &mut Game, character: Character) {
    game.flags.remove("character_damsel");
    game.flags.remove("character_tunnelman");
    match character {
        Character::Explorer => {}
        Character::Damsel => {
            game.flags.insert("character_damsel".into());
        }
        Character::TunnelMan => {
            game.flags.insert("character_tunnelman".into());
        }
    }
    let tunnel = character == Character::TunnelMan;
    game.player.health = if tunnel { 2 } else { 4 };
    game.player.bombs = if tunnel { 0 } else { 4 };
    game.player.ropes = if tunnel { 0 } else { 4 };
    game.player.data.remove("character_attack_active");
    game.player.data.remove("character_attack_elapsed");
    game.player.attack_timer = 0;
    game.player.animation = 0.0;
    let sprite = match character {
        Character::Explorer => "sStandLeft",
        Character::Damsel => "sDamselLeft",
        Character::TunnelMan => "sTunnelLeft",
    };
    game.player.set_sprite(sprite);
}

pub fn selected(game: &Game) -> Character {
    if game.flags.contains("character_tunnelman") {
        Character::TunnelMan
    } else if game.flags.contains("character_damsel") {
        Character::Damsel
    } else {
        Character::Explorer
    }
}

fn swing_part(game: &mut Game, front: bool) {
    let facing = game.player.facing;
    let object = if front { "oMattockHit" } else { "oMattockPre" };
    let id = game.world.spawn(
        object,
        Vec2::new(
            game.player.pos.x + f64::from(facing) * if front { 16.0 } else { -16.0 },
            game.player.pos.y,
        ),
    );
    let part = game.world.entity_mut(id).unwrap();
    part.facing = facing;
    part.set_sprite(&format!(
        "sMattock{}{}",
        if front { "Hit" } else { "Pre" },
        if facing < 0 { "L" } else { "R" }
    ));
    part.timer = if front { 5 } else { 3 };
    part.data.insert("owner".into(), 1.0);
    part.data.insert("owner_player".into(), 1.0);
    part.data.insert("damage".into(), 2.0);
    part.data.insert("character_mattock".into(), 1.0);
    if front {
        part.data.insert("puncture".into(), 1.0);
    }
}

/// Called once from item handling after chest interactions and before projectile
/// updates. The item's caller suppresses attack_pressed for consumed interactions.
/// The first attack frame advances by 0.6; characterStepEvent sets whipping
/// animation speed to 1 on subsequent ticks. No inventory item is granted.
pub fn tick(game: &mut Game, input: &InputFrame) {
    let active = game
        .player
        .data
        .get("character_attack_active")
        .copied()
        .unwrap_or(0.0)
        != 0.0;
    if selected(game) != Character::TunnelMan
        || game.player.state == PlayerState::Dead
        || game.state == RunState::Dead
        // oPlayer1 Step's item/innate attack branch checks the independent
        // stunned flag, including enemy throws that assign no timer.
        || game.player.is_stunned()
    {
        if active {
            game.player.attack_timer = 0;
            game.player.data.remove("character_attack_active");
            game.player.data.remove("character_attack_elapsed");
            for entity in &mut game.world.entities {
                if entity.value("character_mattock") != 0.0 {
                    entity.alive = false;
                }
            }
        }
        return;
    }
    // Holding suppresses innate work, not an already active whipping flag.
    // Player Animation End and late hurt retain ownership of that clear.
    if game.player.held.is_some() {
        return;
    }
    if !active {
        if !input.attack_pressed
            || !crate::player::source_platform_grounded(game)
            || crate::player::source_ducking(game)
            || game.player.attack_timer > 0
            || game.player.data.get("duck_to_hang").copied().unwrap_or(0.0) > 0.0
        {
            return;
        }
        game.player
            .data
            .insert("character_attack_active".into(), 1.0);
        game.player
            .data
            .insert("character_attack_elapsed".into(), 0.0);
        game.player.set_sprite("sTunnelAttackL");
        game.player.animation = 0.0;
        game.player.set_image_speed(0.6);
    }
    let elapsed = game
        .player
        .data
        .entry("character_attack_elapsed".into())
        .or_default();
    *elapsed += 1.0;
    let age = *elapsed as u32;
    // The source tests the current image_index during Step. Endframe advances
    // it later; rebuilding a phase from elapsed ticks would animate twice.
    game.player.attack_timer = 12_u32.saturating_sub(age).max(1);
    if active
        && game.player.animation < 2.0
        && !game
            .world
            .entities
            .iter()
            .any(|e| e.alive && e.object == "oMattockPre")
    {
        swing_part(game, false);
    }
    if game.player.animation > 4.0
        && !game
            .world
            .entities
            .iter()
            .any(|e| e.alive && matches!(e.object.as_str(), "oWhip" | "oMattockHit" | "oSlash"))
    {
        swing_part(game, true);
        game.sound("sndWhip", game.player.pos);
    }
}

/// The player Animation End callback clears whipping; attack attachments retain
/// their own source lifetime instead of being removed early during Step.
pub fn animation_end(game: &mut Game) {
    game.player.data.remove("character_attack_active");
    game.player.data.remove("character_attack_elapsed");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_starting_stats_have_no_invented_equipment() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        for (character, health, bombs, ropes, sprite) in [
            (Character::TunnelMan, 2, 0, 0, "sTunnelLeft"),
            (Character::Damsel, 4, 4, 4, "sDamselLeft"),
            (Character::Explorer, 4, 4, 4, "sStandLeft"),
        ] {
            apply(&mut game, character);
            assert_eq!(selected(&game), character);
            assert_eq!(
                (game.player.health, game.player.bombs, game.player.ropes),
                (health, bombs, ropes)
            );
            assert_eq!(game.player.sprite, sprite);
            assert!(game.player.equipment.is_empty());
            assert!(game.player.held.is_none());
        }
    }

    #[test]
    fn innate_mattock_requires_ground_and_empty_hands() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        apply(&mut game, Character::TunnelMan);
        let attack = InputFrame {
            attack_pressed: true,
            ..Default::default()
        };
        tick(&mut game, &attack);
        assert!(game.world.entities.is_empty());
        game.player.grounded = true;
        game.player.held = Some(game.world.spawn("oRock", game.player.pos));
        tick(&mut game, &attack);
        assert_eq!(game.player.attack_timer, 0);
        game.player.held = None;
        // The direct helper receives the platform controller's duck pose;
        // Down alone is not Player Step1219's attack-onset guard.
        game.player.state = PlayerState::Ducking;
        game.player.set_sprite("sTunnelDuckL");
        tick(
            &mut game,
            &InputFrame {
                down: true,
                ..attack
            },
        );
        assert_eq!(game.player.attack_timer, 0);
        game.player.state = PlayerState::Standing;
        game.player.set_sprite("sTunnelLeft");
        tick(&mut game, &attack);
        assert_eq!(game.player.attack_timer, 11);
        assert_eq!(game.player.sprite, "sTunnelAttackL");
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|e| e.object == "oMattockPre")
                .count(),
            0
        );
        assert!(!game.player.data.contains_key("cantJump"));
    }

    #[test]
    fn innate_mattock_uses_fast_original_swing_and_two_damage() {
        let mut level = crate::level::Level::default();
        level.set_tile(4, 3, '@').unwrap();
        for x in 0..40 {
            level.set_tile(x, 4, '1').unwrap();
        }
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.load_custom(&level, "mattock-animation", "");
        apply(&mut game, Character::TunnelMan);
        for _ in 0..8 {
            game.step(&InputFrame::default());
        }
        game.step(&InputFrame {
            attack_pressed: true,
            ..Default::default()
        });
        assert!((game.player.animation - 0.6).abs() < 1e-9);
        for _ in 0..4 {
            game.step(&InputFrame::default());
        }
        assert!((game.player.animation - 4.6).abs() < 1e-9);
        assert!(
            !game
                .world
                .entities
                .iter()
                .any(|e| e.object == "oMattockHit")
        );
        game.step(&InputFrame::default());
        let hit = game
            .world
            .entities
            .iter()
            .find(|e| e.object == "oMattockHit")
            .unwrap();
        assert_eq!(hit.sprite, "sMattockHitR");
        assert_eq!(hit.pos.x, game.player.pos.x + 16.0);
        assert_eq!(hit.value("damage"), 2.0);
        assert_eq!(hit.value("puncture"), 1.0);
        for _ in 6..12 {
            game.step(&InputFrame::default());
        }
        assert_eq!(game.player.attack_timer, 0);
        assert!(!game.player.data.contains_key("character_attack_active"));
        assert!((game.player.animation - 0.6).abs() < 1e-9);
        assert_eq!(game.player.health, 2);
        assert!(game.player.equipment.is_empty());
    }

    #[test]
    fn stun_cancels_innate_swing_and_hitboxes() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        apply(&mut game, Character::TunnelMan);
        game.player.grounded = true;
        tick(
            &mut game,
            &InputFrame {
                attack_pressed: true,
                ..Default::default()
            },
        );
        game.player.stun = 2;
        tick(&mut game, &InputFrame::default());
        assert_eq!(game.player.attack_timer, 0);
        assert!(game.world.entities.iter().all(|e| !e.alive));
    }

    #[test]
    fn actual_yeti_zero_timer_throw_blocks_innate_attack_until_recovery() {
        let mut level = crate::level::Level::default();
        level.set_tile(6, 7, '@').unwrap();
        for x in 0..40 {
            level.set_tile(x, 8, '1').unwrap();
        }
        let mut game = Game::shell(11, RulesPreset::Classic);
        game.load_custom(&level, "tunnel-throw", "");
        apply(&mut game, Character::TunnelMan);
        for _ in 0..8 {
            game.step(&InputFrame::default());
        }
        assert!(game.player.grounded);
        assert_eq!(game.player.health, 2);
        let attack = InputFrame {
            attack: true,
            attack_pressed: true,
            ..Default::default()
        };
        let mut ordinary = game.clone();
        ordinary.step(&attack);
        assert_eq!(ordinary.player.attack_timer, 11);
        // Attachment checks precede the attack input in Player Step, so the
        // pre-swing instance is created on the following authoritative tick.
        ordinary.step(&InputFrame::default());
        assert!(ordinary.world.entities.iter().any(|e| {
            e.alive && e.object == "oMattockPre" && e.value("character_mattock") != 0.
        }));

        // The room entrance, character selection, and actual enemy collision
        // establish the complete state. No health, stun, or position is edited.
        let p = game.player.pos;
        game.world.spawn("oYeti", Vec2::new(p.x - 8., p.y - 8.));
        game.step(&InputFrame::default());
        assert!(game.player.is_stunned());
        assert_eq!(game.player.stun, 0);
        let mut recovered = false;
        let neutral = InputFrame::default();
        for tick in 0..120 {
            game.step(if tick % 2 == 0 { &attack } else { &neutral });
            if !game.player.is_stunned() {
                recovered = true;
                break;
            }
            assert_eq!(game.player.stun, 0);
            assert_eq!(game.player.attack_timer, 0);
            assert!(!game.player.data.contains_key("character_attack_active"));
            assert!(
                !game
                    .world
                    .entities
                    .iter()
                    .any(|e| { e.alive && e.value("character_mattock") != 0. })
            );
        }
        assert!(recovered, "the live Yeti throw must naturally recover");
        assert!(game.player.health > 0);
        for _ in 0..60 {
            if game.player.grounded
                && game.player.vel.y == 0.
                && !game.player.data.contains_key("character_attack_active")
            {
                break;
            }
            game.step(&neutral);
        }
        assert!(game.player.grounded);
        assert_eq!(game.player.vel.y, 0.);
        game.step(&attack);
        assert_eq!(game.player.attack_timer, 11);
        game.step(&neutral);
        assert!(game.world.entities.iter().any(|e| {
            e.alive && e.object == "oMattockPre" && e.value("character_mattock") != 0.
        }));
    }
}
