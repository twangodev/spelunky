//! Configured initial stun and authored room; real neutral Game::step inputs.
//! These are not recorded ordinary Boss-cast routes. No actor writes after start.
use spelunky_core::*;

const PRESETS: [RulesPreset; 2] = [RulesPreset::Classic, RulesPreset::Assisted];

fn scene(preset: RulesPreset, wave_x: f64) -> (Game, u64) {
    let mut game = Game::shell(42, preset);
    game.world = World::new(20, 16, Area::Ice, 9);
    game.world.name = "rLevel".into();
    for x in 0..game.world.width {
        game.world.set_tile(x, 7, "oHardBlock");
    }
    game.player.pos = Vec2::new(104.0, 104.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Stunned;
    game.player.stun = 40;
    game.player.sprite = "sStunL".into();
    game.player.grounded = true;
    assert_eq!(game.player.vel, Vec2::default());
    assert_eq!(game.player.health, 4.0);
    assert!(game.player.held.is_none());
    assert!(game.player.equipment.is_empty());
    let id = game.spawn("oPsychicWave", Vec2::new(wave_x, 104.0));
    let wave = game.world.entity(id).unwrap();
    assert_eq!(wave.sprite, "sPsychicWave");
    assert_eq!(wave.image_speed(), 0.25);
    assert_eq!(wave.animation, 0.0);
    assert_eq!(wave.vel, Vec2::default());
    (game, id)
}

fn assert_player_and_wave_prefix(game: &Game, id: u64, pos: Vec2) {
    assert_eq!(game.player.pos, Vec2::new(104.0, 104.0));
    assert_eq!(game.player.health, 4.0);
    assert!(game.player.is_stunned() && game.player.stun > 0);
    assert_eq!(game.player.sprite, "sStunL");
    assert_eq!(game.player.vel, Vec2::default());
    assert!(game.player.engine_active);
    let characters = dispatcher::descendant_instances(game, "oCharacter");
    assert_eq!(characters.first(), Some(&dispatcher::Instance::Player));
    let wave = game.world.entity(id).expect("source Wave remains live");
    assert!(wave.engine_active);
    assert_eq!(wave.pos, pos);
    assert_eq!(wave.image_speed(), 0.25);
    assert!(
        !game
            .events
            .iter()
            .any(|event| matches!(event, GameEvent::Damage { .. }))
    );
}

#[test]
fn configured_stunned_wave_reaches_character_then_moves_past_coincidence() {
    for preset in PRESETS {
        let (mut game, id) = scene(preset, 102.0);
        let rng = game.rng.clone();
        game.step(&InputFrame::default());
        // Positive producer prefix: the same real first Step reaches the target.
        assert_player_and_wave_prefix(&game, id, Vec2::new(104.0, 104.0));
        assert_eq!(game.rng.clone().next_u64(), rng.clone().next_u64());
        let before_animation = game.world.entity(id).unwrap().animation;
        assert_eq!(before_animation, 0.25);
        game.step(&InputFrame::default());
        assert_eq!(game.player.pos, Vec2::new(104.0, 104.0));
        assert_eq!(game.player.health, 4.0);
        assert!(game.player.is_stunned() && game.player.stun > 0);
        let wave = game
            .world
            .entity(id)
            .expect("stunned guard preserves the Wave");
        assert!(wave.engine_active);
        assert_eq!(wave.animation, 0.5);
        assert_eq!(
            wave.pos,
            Vec2::new(106.0, 104.0),
            "native point_direction at coincidence is zero; source still moves two pixels right"
        );
        assert_eq!(wave.data.get("dir").copied(), Some(0.0));
        assert_eq!(game.rng.clone().next_u64(), rng.clone().next_u64());
        assert_player_and_wave_prefix(&game, id, Vec2::new(106.0, 104.0));
    }
}

#[test]
fn configured_stunned_wave_at_coincidence_moves_on_its_first_step() {
    for preset in PRESETS {
        let (mut game, id) = scene(preset, 104.0);
        game.step(&InputFrame::default());
        assert_eq!(game.player.pos, Vec2::new(104.0, 104.0));
        assert_eq!(game.player.health, 4.0);
        assert!(game.player.is_stunned() && game.player.stun > 0);
        let wave = game.world.entity(id).unwrap();
        assert!(wave.engine_active);
        assert_eq!(wave.animation, 0.25);
        assert_eq!(
            wave.pos,
            Vec2::new(106.0, 104.0),
            "unconditional source coordinate statements also run at coincidence"
        );
        assert_eq!(wave.data.get("dir").copied(), Some(0.0));
    }
}

#[test]
fn configured_noncoincident_wave_preserves_axis_motion_and_stunned_guard() {
    for preset in PRESETS {
        let (mut game, id) = scene(preset, 100.0);
        let rng = game.rng.clone();
        game.step(&InputFrame::default());
        // No newly stored dir assertion: this compatibility control passes old code.
        assert_player_and_wave_prefix(&game, id, Vec2::new(102.0, 104.0));
        assert_eq!(game.world.entity(id).unwrap().animation, 0.25);
        assert_eq!(game.rng.clone().next_u64(), rng.clone().next_u64());
    }
}

#[test]
fn configured_level_reactivation_keeps_existing_character_before_player() {
    for preset in PRESETS {
        // Separate authored starting scene: no previous world, actors, or
        // dispatcher entries are replaced, and no source arrays are rewritten.
        let mut game = Game::shell(42, preset);
        game.world = World::new(20, 16, Area::Ice, 9);
        game.world.name = "rLevel".into();
        for x in 0..game.world.width {
            game.world.set_tile(x, 7, "oHardBlock");
        }
        game.player.pos = Vec2::new(200.0, 104.0);
        game.player.previous_pos = game.player.pos;
        game.player.engine_active = false;
        assert_eq!(game.player.vel, Vec2::default());
        assert_eq!(game.player.health, 4.0);
        let character = game.spawn("oCharacter", Vec2::new(104.0, 104.0));
        let wave = game.spawn("oPsychicWave", Vec2::new(104.0, 72.0));
        let rng = game.rng.clone();
        game.step(&InputFrame::default());
        let characters = dispatcher::descendant_instances(&game, "oCharacter");
        assert_eq!(
            characters,
            vec![
                dispatcher::Instance::Entity(character),
                dispatcher::Instance::Player
            ]
        );
        assert!(game.player.engine_active);
        assert_eq!(game.player.pos, Vec2::new(200.0, 104.0));
        assert_eq!(game.player.health, 4.0);
        let target = game.world.entity(character).unwrap();
        assert!(target.engine_active);
        assert_eq!(target.pos, Vec2::new(104.0, 104.0));
        let effect = game.world.entity(wave).unwrap();
        assert!(effect.engine_active);
        assert_eq!(effect.animation, 0.25);
        assert_eq!(
            effect.pos,
            Vec2::new(104.0, 74.0),
            "source Wave aims at the first current active Character, not always Player"
        );
        assert_eq!(effect.data.get("dir").copied(), Some(270.0));
        assert_eq!(game.rng.clone().next_u64(), rng.clone().next_u64());
    }
}
