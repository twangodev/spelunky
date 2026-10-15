//! Natural ManTrap contact uses the inverse source visibility adapter.
//! Authored default actors; ordinary Right input then one neutral native Step.
use spelunky_core::{
    Area, Game, GameEvent, GenerationRevision, InputFrame, PlayerState, RulesPreset, RunState,
    Vec2, World,
};

fn loose_active(game: &Game, id: u64) {
    let pack = game.world.entity(id).expect("FIXTURE_LOOSE_JETPACK_EXISTS");
    assert_eq!(pack.object, "oJetpack");
    assert!(
        pack.alive && pack.engine_active,
        "FIXTURE_LOOSE_JETPACK_ACTIVE"
    );
    assert!(!pack.held, "FIXTURE_LOOSE_JETPACK_NOT_HELD");
    assert!(
        pack.pos.x > game.camera.x - 16.0 && pack.pos.x < game.camera.x + 336.0,
        "FIXTURE_LOOSE_JETPACK_IN_VIEW_X"
    );
    assert!(
        pack.pos.y > game.camera.y - 16.0 && pack.pos.y < game.camera.y + 256.0,
        "FIXTURE_LOOSE_JETPACK_IN_VIEW_Y"
    );
    assert!(
        game.world.solid_at(pack.pos.x, pack.pos.y + 9.0),
        "FIXTURE_SAFE_FLOOR_SUPPORT"
    );
    assert!(game.player.held.is_none());
    assert!(!game.player.has("Jetpack"));
}

fn natural_mantrap_hidden_cleanup(rules: RulesPreset, revision: GenerationRevision) {
    let mut game = Game::shell(7, rules);
    // Initial identity only, before scene/actor births: same canonical RNG and
    // public input route isolate the generation-revision compatibility guard.
    game.generation_revision = revision;
    game.world = World::new(30, 20, Area::Mines, 1);
    game.world.name = "rLevel".into();
    for column in 0..30 {
        game.world.set_tile(column, 10, "oHardBlock");
    }
    game.player.pos = Vec2::new(104.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.world.entrance = game.player.pos;
    // No actor cost, HP, status, velocity, invulnerability, held or visibility
    // override: Game.spawn runs the original default Create paths.
    let pack = game.spawn("oJetpack", Vec2::new(224.0, 152.0));
    let mantrap = game.spawn("oManTrap", Vec2::new(160.0, 144.0));
    assert_eq!(game.player.health, 4);
    assert!(!game.player.has("Ankh"), "FIXTURE_NO_ANKH_REVIVAL");
    assert!(
        !game.player.has("SpikeShoes"),
        "FIXTURE_DEFAULT_NO_STOMP_EQUIPMENT"
    );
    assert_eq!(game.player.invincible, 0);
    assert_eq!(game.player.data.get("hidden").copied().unwrap_or(0.0), 0.0);
    assert_eq!(game.player.data.get("visible").copied().unwrap_or(1.0), 1.0);
    assert_eq!(game.world.entity(pack).unwrap().value("cost"), 20000.0);
    assert!((game.world.entity(pack).unwrap().pos.x - game.player.pos.x).abs() > 32.0);
    assert_eq!(game.world.entity(mantrap).unwrap().health, 3);
    assert_eq!(game.world.entity(mantrap).unwrap().state, 0);
    assert_eq!(game.world.entity(mantrap).unwrap().vel.x, 2.5);
    loose_active(&game, pack);
    for _ in 0..96 {
        assert_eq!(game.player.health, 4, "FIXTURE_HEALTHY_UNTIL_REAL_EAT");
        assert_eq!(
            game.player.invincible, 0,
            "FIXTURE_NOT_PREINJURED_OR_INVINCIBLE"
        );
        assert!(game.player.engine_active, "FIXTURE_SOURCE_PLAYER_EXISTS");
        assert_eq!(game.player.data.get("hidden").copied().unwrap_or(0.0), 0.0);
        assert_ne!(game.player.data.get("visible").copied().unwrap_or(1.0), 0.0);
        loose_active(&game, pack);
        game.step(&InputFrame {
            right: true,
            ..Default::default()
        });
        if game.player.data.get("hidden") == Some(&1.0) {
            assert_eq!(
                game.player.health, -99,
                "FIXTURE_REAL_MANTRAP_FATAL_CONTACT"
            );
            assert_eq!(
                game.player.invincible, 9999,
                "FIXTURE_MANTRAP_INVINCIBILITY_WRITE"
            );
            let eater = game
                .world
                .entity(mantrap)
                .expect("FIXTURE_REAL_MANTRAP_ALIVE");
            assert_eq!(eater.state, 97, "FIXTURE_REAL_MANTRAP_EATING_STATUS");
            assert_eq!(
                eater.sprite, "sManTrapEatL",
                "FIXTURE_REAL_MANTRAP_EATING_ART"
            );
            assert!(
                game.events.iter().any(|event| matches!(event,
                GameEvent::Sound { name, .. } if name == "sndDie")),
                "FIXTURE_REAL_EAT_SOUND"
            );
            assert_ne!(
                game.player.data.get("visible").copied().unwrap_or(1.0),
                0.0,
                "FIXTURE_HIDDEN_ADAPTER_IS_DISTINCT_FROM_VISIBLE_ZERO"
            );
            assert_eq!(
                game.state,
                RunState::Playing,
                "FIXTURE_COLLISION_AFTER_PLAYER_NORMAL"
            );
            // Jetpack738 Normal Step preceded this automatic Collision.
            // It must still exist on the bite frame in BOTH old and new arms.
            loose_active(&game, pack);
            assert!(!game.player.has("Ankh"), "FIXTURE_NO_ANKH_BEFORE_CLEANUP");
            game.step(&InputFrame::default());
            assert!(
                game.player.engine_active,
                "FIXTURE_PLAYER_EXISTS_ON_CLEANUP_STEP"
            );
            assert_eq!(game.player.data.get("hidden"), Some(&1.0));
            assert_ne!(game.player.data.get("visible").copied().unwrap_or(1.0), 0.0);
            assert_eq!(
                game.state,
                RunState::Dead,
                "FIXTURE_NEXT_PLAYER_STEP_RECOGNIZED_DEATH"
            );
            assert_eq!(game.player.health, 0);
            assert!(!game.player.has("Jetpack"));
            assert!(game.player.held.is_none());
            if revision == GenerationRevision::SourceClassicV2 {
                assert!(
                    game.world.entity(pack).is_none(),
                    "JETPACK_HIDDEN_SOURCE_VISIBILITY_REQUIRES_RETIREMENT"
                );
            } else {
                loose_active(&game, pack);
                assert!(
                    game.world.entity(pack).is_some(),
                    "HISTORICAL_HIDDEN_ONLY_RETAINS_LOOSE_PACK"
                );
            }
            return;
        }
    }
    panic!("FIXTURE_ORDINARY_MANTRAP_WALK_DID_NOT_EAT_WITHIN_96_STEPS");
}

#[test]
fn classic_real_mantrap_hidden_player_retires_loose_jetpack_next_step() {
    natural_mantrap_hidden_cleanup(RulesPreset::Classic, GenerationRevision::SourceClassicV2);
}
#[test]
fn assisted_real_mantrap_hidden_player_retires_loose_jetpack_next_step() {
    natural_mantrap_hidden_cleanup(RulesPreset::Assisted, GenerationRevision::SourceClassicV2);
}

#[test]
fn classic_historical_real_mantrap_hidden_player_preserves_loose_jetpack() {
    natural_mantrap_hidden_cleanup(RulesPreset::Classic, GenerationRevision::HistoricalV1);
}
#[test]
fn assisted_historical_real_mantrap_hidden_player_preserves_loose_jetpack() {
    natural_mantrap_hidden_cleanup(RulesPreset::Assisted, GenerationRevision::HistoricalV1);
}
