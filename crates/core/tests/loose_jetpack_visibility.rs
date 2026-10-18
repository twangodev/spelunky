//! Authored initial actors; all subsequent state changes use ordinary input.
//! Loose Jetpack own Step follows inherited Item Step; Ghost hides Player in
//! the later automatic collision phase. Worn Jetpack is a separate global flag.
use spelunky_core::{Area, Game, GameEvent, InputFrame, PlayerState, RulesPreset, Vec2, World};

fn visible(game: &Game) -> bool {
    game.player.data.get("visible").copied().unwrap_or(1.0) != 0.0
}

fn room(rules: RulesPreset, owned: bool, with_ghost: bool) -> (Game, u64, Option<u64>) {
    let mut game = Game::legacy_shell(7, rules);
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
    let pickup = owned.then(|| game.spawn("oJetpack", game.player.pos));
    let loose = game.spawn("oJetpack", Vec2::new(160.0, 152.0));
    let ghost = with_ghost.then(|| game.spawn("oGhost", Vec2::new(240.0, 136.0)));
    for _ in 0..8 {
        game.step(&InputFrame::default());
        assert!(
            visible(&game),
            "fixture prerequisite: Ghost has not reached Player"
        );
        assert_eq!(game.player.health, 4.0);
        assert!(game.world.entity(loose).is_some());
    }
    if let Some(pickup) = pickup {
        assert_eq!(game.world.entity(pickup).unwrap().value("cost"), 0.0);
        game.step(&InputFrame {
            down: true,
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert!(
            game.player.has("Jetpack"),
            "fixture prerequisite: natural equipment pickup"
        );
        assert!(game.world.entity(pickup).is_none());
        assert!(game.player.held.is_none());
        assert!(visible(&game));
        assert_eq!(game.player.health, 4.0);
    }
    assert_eq!(game.player.has("Jetpack"), owned);
    (game, loose, ghost)
}

fn chase_until_invisible(game: &mut Game, loose: u64, ghost: u64) {
    for _ in 0..240 {
        assert!(visible(game));
        assert_eq!(game.player.health, 4.0);
        assert_eq!(game.player.invincible, 0);
        let pack = game
            .world
            .entity(loose)
            .expect("visible-player loose pickup survives");
        assert!(
            pack.engine_active,
            "fixture prerequisite: loose pickup is active"
        );
        assert!(!pack.held);
        assert_eq!(game.world.entity(ghost).unwrap().state, 1);
        game.step(&InputFrame::default());
        if !visible(game) {
            assert_eq!(game.player.health, -99.0, "real Ghost fatal contact");
            let ghost = game.world.entity(ghost).expect("fresh Ghost disappearance");
            assert_eq!(ghost.state, 0);
            assert_eq!(ghost.sprite, "sGhostDisappear");
            assert!(game.events.iter().any(|event| matches!(event,
                GameEvent::Sound { name, .. } if name == "sndGhost"
            )));
            assert!(game.events.iter().any(|event| matches!(event,
                GameEvent::Sound { name, .. } if name == "sndDie"
            )));
            let pack = game
                .world
                .entity(loose)
                .expect("Ghost hides Player after this tick's Jetpack Step");
            assert!(pack.engine_active);
            assert!(game.player.engine_active);
            assert!(pack.pos.x > game.camera.x - 16.0 && pack.pos.x < game.camera.x + 336.0);
            assert!(pack.pos.y > game.camera.y - 16.0 && pack.pos.y < game.camera.y + 256.0);
            return;
        }
    }
    panic!("fixture timing: natural Ghost chase did not reach Player within 240 ticks");
}

#[test]
fn natural_ghost_hiding_player_retires_loose_jetpack_on_next_item_step() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, loose, ghost) = room(rules, false, true);
        chase_until_invisible(&mut game, loose, ghost.unwrap());
        game.step(&InputFrame::default());
        assert!(!visible(&game));
        assert!(
            game.world.entity(loose).is_none(),
            "source Jetpack own Step retires the loose actor after Player becomes invisible"
        );
        assert!(!game.player.has("Jetpack"));
    }
}

#[test]
fn natural_ghost_cleanup_does_not_clear_naturally_owned_jetpack_equipment() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, loose, ghost) = room(rules, true, true);
        chase_until_invisible(&mut game, loose, ghost.unwrap());
        assert!(game.player.has("Jetpack"));
        game.step(&InputFrame::default());
        assert!(
            game.world.entity(loose).is_none(),
            "loose pickup is distinct from worn equipment"
        );
        assert!(
            game.player.has("Jetpack"),
            "source loose actor Destroy does not clear global Jetpack ownership"
        );
    }
}

#[test]
fn visible_player_preserves_loose_jetpack_with_and_without_owned_equipment() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for owned in [false, true] {
            let (mut game, loose, ghost) = room(rules, owned, false);
            assert!(ghost.is_none());
            for _ in 0..260 {
                game.step(&InputFrame::default());
                assert!(visible(&game));
                assert_eq!(game.player.health, 4.0);
                assert_eq!(game.player.has("Jetpack"), owned);
                let pack = game
                    .world
                    .entity(loose)
                    .expect("visible-player control retains loose Jetpack");
                assert!(pack.engine_active);
                assert_eq!(pack.object, "oJetpack");
                assert!(!pack.held);
                assert!(game.player.held.is_none());
            }
        }
    }
}

// Controlled initial visibility is deliberate: verifies the literal predicate
// independently of death, while the tests above prove natural Ghost reachability.
#[test]
fn initially_invisible_living_player_retires_only_loose_jetpack() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = Game::legacy_shell(7, rules);
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
        game.player.data.insert("visible".into(), 0.0);
        let pack = game.spawn("oJetpack", Vec2::new(160.0, 152.0));
        let cape = game.spawn("oCapePickup", Vec2::new(192.0, 152.0));
        assert!(game.player.engine_active);
        assert!(game.world.entity(pack).unwrap().engine_active);
        game.step(&InputFrame::default());
        assert_eq!(game.player.health, 4.0);
        assert!(!visible(&game));
        assert!(
            game.world.entity(pack).is_none(),
            "visibility rather than health is the source Jetpack condition"
        );
        assert!(
            game.world.entity(cape).is_some(),
            "do not generalize Jetpack own Step to other loose equipment"
        );
        assert!(!game.player.has("Jetpack"));
        assert!(!game.player.has("Cape"));
    }
}
