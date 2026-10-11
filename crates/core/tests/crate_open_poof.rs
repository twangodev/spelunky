//! Authored closed containers/default player, followed only by ordinary inputs.
//! Source Player Step654-710: loot, Pickup sound, stationary Poof, then Destroy.
use spelunky_core::{
    Area, Game, GameEvent, InputFrame, PlayerState, Rng, RulesPreset, Vec2, World,
};

fn ready(preset: RulesPreset, object: &str, x: f64) -> (Game, u64) {
    let mut game = Game::shell(7, preset);
    game.world = World::new(24, 20, Area::Mines, 1);
    game.world.name = "rLevel".into();
    for column in 0..24 {
        game.world.set_tile(column, 10, "oHardBlock");
    }
    game.player.pos = Vec2::new(80.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.world.entrance = game.player.pos;
    let id = game.spawn(object, Vec2::new(x, 152.0));
    // Complete room initialization and initial container alarms naturally.
    game.step(&InputFrame::default());
    assert!(game.world.entity(id).is_some());
    assert_eq!(game.player.health, 4);
    assert_eq!(game.player.attack_timer, 0);
    assert!(game.world.entities.iter().all(|e| e.object != "oPoof"));
    (game, id)
}

// Literal source loot tests, independent of the new Poof callback. The oracle
// tracks existing loot RNG only: Poof + DrawnSprite Create consume no draws.
fn source_crate_loot(rng: &mut Rng) -> &'static str {
    for (odds, object) in [
        (500, "oJetpack"),
        (200, "oCapePickup"),
        (100, "oShotgun"),
        (100, "oMattock"),
        (100, "oTeleporter"),
        (90, "oGloves"),
        (90, "oSpectacles"),
        (80, "oWebCannon"),
        (80, "oPistol"),
        (80, "oMitt"),
        (60, "oPaste"),
        (60, "oSpringShoes"),
        (60, "oSpikeShoes"),
        (60, "oMachete"),
        (40, "oBombBox"),
        (40, "oBow"),
        (20, "oCompass"),
        (10, "oParaPickup"),
        (2, "oRopePile"),
    ] {
        if rng.range(odds) == 0 {
            return object;
        }
    }
    "oBombBag"
}

#[test]
fn actual_up_attack_opens_crates_with_source_stationary_poof_after_loot() {
    for preset in [RulesPreset::Classic, RulesPreset::Assisted] {
        for object in ["oCrate", "oFlareCrate"] {
            let (mut game, id) = ready(preset, object, 84.0);
            let origin = game.world.entity(id).unwrap().pos;
            assert_ne!(
                origin.x, game.player.pos.x,
                "Poof must use container origin"
            );
            let existing_ids: Vec<_> = game.world.entities.iter().map(|e| e.id).collect();
            let mut closed = game.clone();
            closed.step(&InputFrame::default());
            let mut expected_rng = closed.rng.clone();
            let expected_loot = if object == "oCrate" {
                let loot = source_crate_loot(&mut expected_rng);
                // This quiet authored seed keeps loot observable, avoiding the
                // source late-Player automatic Bag/Box/RopePile collection.
                assert_eq!(loot, "oBow", "fixture prerequisite: observable loot");
                loot
            } else {
                for _ in 0..3 {
                    expected_rng.range(4);
                    expected_rng.range(4);
                    expected_rng.range(3);
                }
                "oFlare"
            };
            game.step(&InputFrame {
                up: true,
                attack: true,
                attack_pressed: true,
                ..Default::default()
            });
            assert!(
                game.world.entity(id).is_none(),
                "successful opening destroys container"
            );
            assert_eq!(game.player.attack_timer, 0, "opening consumes attack edge");
            assert!(game.player.held.is_none());
            assert_eq!(game.player.health, 4);
            let loot: Vec<_> = game
                .world
                .entities
                .iter()
                .filter(|e| !existing_ids.contains(&e.id) && e.object == expected_loot)
                .collect();
            assert_eq!(loot.len(), if object == "oCrate" { 1 } else { 3 });
            for e in &loot {
                assert_eq!(
                    e.previous_pos, origin,
                    "loot was born at source crate origin"
                );
                if object == "oFlareCrate" {
                    assert_eq!(
                        e.alarms[0], 1,
                        "new Flare is born after this tick's alarm phase"
                    );
                }
            }
            assert!(game.events.iter().any(|e| matches!(e,
                GameEvent::Sound { name, pos } if name == "sndPickup" && *pos == origin
            )));
            assert_eq!(
                game.rng.clone().next_u64(),
                expected_rng.next_u64(),
                "Poof birth must not consume RNG beyond existing source loot"
            );
            let poofs: Vec<_> = game
                .world
                .entities
                .iter()
                .filter(|e| e.object == "oPoof")
                .collect();
            assert_eq!(poofs.len(), 1, "one source Poof for one successful opening");
            let poof = poofs[0];
            assert_eq!(poof.pos, origin);
            assert_eq!(poof.previous_pos, origin);
            assert_eq!(poof.vel, Vec2::default());
            assert_eq!(poof.image_speed(), 0.4);
            assert_eq!(
                poof.animation, 0.4,
                "new effect advances once in this tick's final animation phase"
            );
            assert_eq!(
                poof.age, 0,
                "birth occurs after existing actors' tick-age snapshot"
            );
            assert!(
                loot.iter().all(|e| e.id < poof.id),
                "source creates loot before Poof"
            );
            assert_eq!(poof.id, loot.iter().map(|e| e.id).max().unwrap() + 1);
            assert_eq!(
                game.lifetime_stats.total_crates,
                if object == "oCrate" { 1 } else { 0 }
            );
        }
    }
}

#[test]
fn closed_crates_emit_no_opening_poof_without_up_attack_contact() {
    for preset in [RulesPreset::Classic, RulesPreset::Assisted] {
        for object in ["oCrate", "oFlareCrate"] {
            for (x, input) in [
                (
                    84.0,
                    InputFrame {
                        up: true,
                        ..Default::default()
                    },
                ),
                (
                    84.0,
                    InputFrame {
                        attack: true,
                        attack_pressed: true,
                        ..Default::default()
                    },
                ),
                (
                    128.0,
                    InputFrame {
                        up: true,
                        attack: true,
                        attack_pressed: true,
                        ..Default::default()
                    },
                ),
            ] {
                let (mut game, id) = ready(preset, object, x);
                game.step(&input);
                assert!(
                    game.world.entity(id).is_some(),
                    "closed control must retain its container"
                );
                assert!(game.world.entities.iter().all(|e| e.object != "oPoof"));
                assert!(!game.events.iter().any(|e| matches!(e,
                    GameEvent::Sound { name, .. } if name == "sndPickup"
                )));
                assert_eq!(game.lifetime_stats.total_crates, 0);
                assert_eq!(game.player.health, 4);
            }
        }
    }
}
