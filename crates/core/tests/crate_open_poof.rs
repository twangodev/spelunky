//! Authored closed containers/default player, followed only by ordinary inputs.
//! Source Player Step654-710: loot, Pickup sound, stationary Poof, then Destroy.
use spelunky_core::{
    Area, Game, GameEvent, InputFrame, PlayerState, Rng, RulesPreset, Vec2, World,
};

fn ready(preset: RulesPreset, object: &str, x: f64, legacy: bool, seed: u32) -> (Game, u64) {
    let mut game = if legacy {
        Game::legacy_shell(u64::from(seed), preset)
    } else {
        Game::shell(seed, preset)
    };
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
    assert_eq!(game.player.health, 4.0);
    assert_eq!(game.player.attack_timer, 0);
    assert!(game.world.entities.iter().all(|e| e.object != "oPoof"));
    (game, id)
}

// Independent arithmetic oracle: copied state only, never production range/unit.
// Canonical recurrence and multiply-high are EXE-derived static math; original
// native generated-seed acceptance remains a separate unrun probe.
struct LiteralRng {
    state: u64,
    canonical: bool,
}
impl LiteralRng {
    fn from_snapshot(rng: &Rng) -> Self {
        let json = serde_json::to_value(rng).unwrap();
        let canonical = match json.get("algorithm").and_then(|x| x.as_str()) {
            Some("gm8_lcg32_v1") => true,
            None => false,
            value => panic!("unexpected RNG algorithm: {value:?}"),
        };
        Self {
            state: json["state"].as_u64().unwrap(),
            canonical,
        }
    }
    fn next_u64(&mut self) -> u64 {
        if self.canonical {
            self.state = u64::from(
                (self.state as u32)
                    .wrapping_mul(0x0808_8405)
                    .wrapping_add(1),
            );
            self.state
        } else {
            self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
            let mut z = self.state;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
            z ^ (z >> 31)
        }
    }
    fn range(&mut self, width: u64) -> u64 {
        assert!(
            width > 0 && width <= 500,
            "authored crate/flaring source domain"
        );
        let raw = self.next_u64();
        if self.canonical {
            (raw * width) >> 32
        } else {
            raw % width
        }
    }
}

// Literal source loot tests, independent of the new Poof callback. The oracle
// tracks existing loot RNG only: Poof + DrawnSprite Create consume no draws.
fn source_crate_loot(rng: &mut LiteralRng) -> &'static str {
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
    for (preset, legacy, seed) in [RulesPreset::Classic, RulesPreset::Assisted]
        .into_iter()
        .flat_map(|preset| [(preset, true, 7_u32), (preset, false, 22_u32)])
    {
        for object in ["oCrate", "oFlareCrate"] {
            let (mut game, id) = ready(preset, object, 84.0, legacy, seed);
            let origin = game.world.entity(id).unwrap().pos;
            assert_ne!(
                origin.x, game.player.pos.x,
                "Poof must use container origin"
            );
            let existing_ids: Vec<_> = game.world.entities.iter().map(|e| e.id).collect();
            let mut closed = game.clone();
            closed.step(&InputFrame::default());
            let mut expected_rng = LiteralRng::from_snapshot(&closed.rng);
            assert_eq!(expected_rng.canonical, !legacy);
            let expected_loot = if object == "oCrate" {
                // Neutral ordinary-Crate initialization/steps must not draw.
                assert_eq!(expected_rng.state, u64::from(seed));
                // Literal source selectors: historical seed7 and canonical
                // seed22 first succeed at Bow (selector16), preserving the
                // original observable-loot prerequisite and all its assertions.
                let odds = [
                    500, 200, 100, 100, 100, 90, 90, 80, 80, 80, 60, 60, 60, 60, 40, 40,
                ];
                let rolls = if legacy {
                    [487, 4, 46, 3, 74, 75, 88, 62, 65, 25, 43, 16, 30, 4, 30, 0]
                } else {
                    [345, 56, 37, 75, 4, 81, 78, 63, 43, 69, 43, 34, 1, 39, 7, 0]
                };
                let mut tape = LiteralRng {
                    state: u64::from(seed),
                    canonical: !legacy,
                };
                for (width, expected) in odds.into_iter().zip(rolls) {
                    assert_eq!(tape.range(width), expected);
                }
                assert_eq!(
                    tape.state,
                    if legacy {
                        16_390_740_445_785_211_223
                    } else {
                        90_347_142
                    }
                );
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
            assert_eq!(game.player.health, 4.0);
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
    for (preset, legacy, seed) in [RulesPreset::Classic, RulesPreset::Assisted]
        .into_iter()
        .flat_map(|preset| [(preset, true, 7_u32), (preset, false, 22_u32)])
    {
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
                let (mut game, id) = ready(preset, object, x, legacy, seed);
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
                assert_eq!(game.player.health, 4.0);
            }
        }
    }
}
