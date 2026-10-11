//! Source-derived Dice own-Step regressions.
//! Initial geometry, actors, facing, money and Craps style are authored fixture
//! context. After the first Step only ordinary InputFrame actions are supplied.
//! No actor velocity, HP, state, cooldown, private Dice flags or RNG is injected.
//! These are focused interactions, not complete generated dice-house routes.
use spelunky_core::{
    Area, CollisionPrecision, Game, GameEvent, InputFrame, PlayerState, RulesPreset, Vec2, World,
};

const PRESETS: [RulesPreset; 2] = [RulesPreset::Classic, RulesPreset::Assisted];

fn room(preset: RulesPreset, player: Vec2, floor: usize, facing: i8) -> Game {
    let mut game = Game::shell(23, preset);
    game.world = World::new(30, 20, Area::Mines, 1);
    game.world.name = "rLevel".into();
    game.player.pos = player;
    game.player.previous_pos = player;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.player.facing = facing;
    game.world.entrance = player;
    for x in 0..30 {
        game.world.set_tile(x, floor, "oBrick");
    }
    game
}

fn blood_count(game: &Game) -> usize {
    game.world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oBlood")
        .count()
}

fn dice_gate(game: &Game, die: u64, damsel: u64) -> bool {
    // Read the captured pre-Animation frame, particularly Exit2's native mask.
    let game = game.presentation();
    let die = game.world.entity(die).unwrap();
    let target = game.world.entity(damsel).unwrap();
    assert_eq!(die.pos.x.fract(), 0.0, "fixture uses integer pixel motion");
    assert_eq!(die.pos.y.fract(), 0.0, "fixture uses integer pixel motion");
    // Source prec0 uses an integer target bbox. For these integer endpoints,
    // inclusive rectangle overlap is equivalent to any of these 25 points.
    (die.vel.x.abs() > 3.0 || die.vel.y.abs() > 3.0)
        && (-2..=2).any(|dx| {
            (-2..=2).any(|dy| {
                target.mask_contains(
                    Vec2::new(die.pos.x + f64::from(dx), die.pos.y + f64::from(dy)),
                    CollisionPrecision::BoundingBox,
                )
            })
        })
}

#[test]
fn falling_die_creates_direct_blood_before_face_draw_without_damsel_cooldown() {
    for preset in PRESETS {
        for initial_bones in [0, 16] {
            let mut game = room(preset, Vec2::new(40.0, 152.0), 10, 1);
            let damsel = game.spawn("oDamsel", Vec2::new(152.0, 152.0));
            let bones: Vec<_> = (0..initial_bones)
                .map(|_| game.spawn("oBone", Vec2::new(240.0, 64.0)))
                .collect();
            let die = game.spawn("oDice", Vec2::new(152.0, 94.0));
            let mut hit = false;
            for _ in 0..32 {
                let mut expected = game.rng.clone();
                let expected_vx = expected.unit() * 4.0 - expected.unit() * 4.0;
                let expected_vy = -1.0 - expected.unit() * 2.0;
                let expected_gravity = f64::from(1 + expected.range(6)) * 0.1;
                let expected_face = f64::from(1 + expected.range(6));
                game.step(&InputFrame::default());
                let target = game.world.entity(damsel).unwrap();
                if target.health == 4 {
                    continue;
                }
                assert_eq!((target.health, target.state, target.timer), (3, 2, 120));
                assert_eq!(target.value("hit_cooldown"), 0.0);
                assert_eq!(target.value("assaulted"), 0.0);
                assert_eq!(target.vel.y, -6.0);
                assert!(dice_gate(&game, die, damsel));
                let die = game.world.entity(die).unwrap();
                assert_eq!(target.vel.x, die.vel.x * 0.3);
                let blood: Vec<_> = game
                    .world
                    .entities
                    .iter()
                    .filter(|e| e.alive && e.object == "oBlood")
                    .collect();
                assert_eq!(
                    blood.len(),
                    1,
                    "direct Create bypasses the script's 16 guard"
                );
                assert_eq!(blood[0].pos, die.pos, "Blood belongs to die context");
                assert_eq!(
                    blood[0].value("life"),
                    60.0,
                    "Blood370 has already stepped before Dice773"
                );
                assert_eq!(blood[0].vel, Vec2::new(expected_vx, expected_vy));
                assert_eq!(blood[0].value("gravity"), expected_gravity);
                assert_eq!(die.value("value"), expected_face);
                assert_eq!(
                    game.rng.clone().next_u64(),
                    expected.next_u64(),
                    "isolated port RNG order: Blood Create4, then final rolling face1"
                );
                assert!(bones.iter().all(|id| game.world.entity(*id).is_some()));
                assert!(game.events.iter().any(|e| matches!(e,
                    GameEvent::Sound { name, pos } if name == "sndHit" && *pos == die.pos
                )));
                hit = true;
                break;
            }
            assert!(
                hit,
                "authored falling die must produce the ordinary first hit"
            );
        }
    }
}

#[test]
fn naturally_thrown_status2_damsel_rejects_fast_die_without_blood() {
    for preset in PRESETS {
        for facing in [-1_i8, 1] {
            let mut game = room(preset, Vec2::new(152.0, 152.0), 10, facing);
            let damsel = game.spawn("oDamsel", game.player.pos);
            let die = game.spawn("oDice", Vec2::new(152.0 + f64::from(facing) * 24.0, 130.0));
            game.step(&InputFrame {
                down: true,
                attack_pressed: true,
                ..Default::default()
            });
            assert_eq!(game.player.held, Some(damsel));
            game.step(&InputFrame::default());
            game.step(&InputFrame {
                attack_pressed: true,
                ..Default::default()
            });
            assert!(game.player.held.is_none());
            assert_eq!(
                game.world.entity(damsel).unwrap().state,
                2,
                "ordinary throw establishes status2; no private-state setup"
            );
            let mut gate_observed = false;
            for _ in 0..12 {
                let target = game.world.entity(damsel).unwrap();
                assert_eq!(target.health, 4, "source rejects already thrown Damsel");
                assert_eq!(target.value("hit_cooldown"), 0.0);
                gate_observed |= dice_gate(&game, die, damsel);
                assert_eq!(blood_count(&game), 0);
                game.step(&InputFrame::default());
            }
            assert_eq!(game.world.entity(damsel).unwrap().health, 4);
            assert!(
                gate_observed,
                "the fast-die rectangle must actually contact the thrown Damsel"
            );
        }
    }
}

#[test]
fn naturally_rescued_invincible_damsel_rejects_fast_die() {
    for preset in PRESETS {
        let mut game = room(preset, Vec2::new(40.0, 152.0), 10, 1);
        game.spawn("oExit", Vec2::new(144.0, 144.0));
        let damsel = game.spawn("oDamsel", Vec2::new(152.0, 152.0));
        let die = game.spawn("oDice", Vec2::new(152.0, 94.0));
        game.step(&InputFrame::default());
        let target = game.world.entity(damsel).unwrap();
        assert_eq!(
            (target.health, target.state, target.value("invincible")),
            (4, 4, 1.0)
        );
        let mut gate_observed = false;
        for _ in 0..24 {
            gate_observed |= dice_gate(&game, die, damsel);
            assert_eq!(game.world.entity(damsel).unwrap().health, 4);
            assert_eq!(blood_count(&game), 0);
            game.step(&InputFrame::default());
        }
        assert!(
            gate_observed,
            "positive invincibility control requires real contact"
        );
        assert_eq!(game.world.entity(damsel).unwrap().health, 4);
    }
}

#[test]
fn natural_rock_hit_status2_remains_a_single_hit_when_die_arrives() {
    for preset in PRESETS {
        let mut game = room(preset, Vec2::new(40.0, 152.0), 10, 1);
        let damsel = game.spawn("oDamsel", Vec2::new(152.0, 152.0));
        game.spawn("oRock", Vec2::new(152.0, 100.0));
        let die = game.spawn("oDice", Vec2::new(152.0, 94.0));
        let mut contact = false;
        for _ in 0..24 {
            game.step(&InputFrame::default());
            let target = game.world.entity(damsel).unwrap();
            if target.health < 4 && dice_gate(&game, die, damsel) {
                assert_eq!((target.health, target.state), (3, 2));
                assert!(
                    target.value("hit_cooldown") > 0.0,
                    "ordinary Rock establishes cooldown, then Damsel Step precedes Dice"
                );
                assert_eq!(blood_count(&game), 1, "only ordinary Rock Blood");
                contact = true;
                break;
            }
        }
        assert!(
            contact,
            "real Rock hit and subsequent fast-die contact must occur"
        );
    }
}

#[test]
fn naturally_spiked_status99_damsel_rejects_later_fast_die() {
    for preset in PRESETS {
        let mut game = room(preset, Vec2::new(40.0, 168.0), 11, 1);
        game.world.set_tile(9, 10, "oSpikes");
        let damsel = game.spawn("oDamsel", Vec2::new(152.0, 140.0));
        let die = game.spawn("oDice", Vec2::new(152.0, 50.0));
        let mut death_observed = false;
        let mut dead_gate_observed = false;
        for _ in 0..40 {
            game.step(&InputFrame::default());
            let target = game.world.entity(damsel).unwrap();
            if target.state == 99 {
                death_observed = true;
                assert_eq!(target.health, 0);
                if dice_gate(&game, die, damsel) {
                    dead_gate_observed = true;
                    assert!(!game.events.iter().any(|e| matches!(e,
                        GameEvent::Sound { name, .. } if name == "sndHit"
                    )));
                    break;
                }
            }
        }
        assert!(
            death_observed,
            "real falling Damsel must die on live Spikes first"
        );
        assert!(
            dead_gate_observed,
            "subsequent die must actually overlap corpse mask"
        );
    }
}

#[test]
fn ordinary_paid_wager_reroll_calls_anger_before_keeper_latches_next_step() {
    for preset in PRESETS {
        let mut game = room(preset, Vec2::new(208.0, 152.0), 10, 1);
        game.player.money = 5000;
        let keeper = game.spawn("oShopkeeper", Vec2::new(320.0, 144.0));
        game.world
            .entity_mut(keeper)
            .unwrap()
            .text
            .insert("style".into(), "Craps".into());
        // y144 and keeper y144 share source shop row1; y112 would trigger
        // the unrelated theft-before-movement branch before the wager.
        let die = game.spawn("oDice", Vec2::new(208.0, 144.0));
        let untouched = game.spawn("oDice", Vec2::new(300.0, 152.0));
        let source_row = |id| ((game.world.entity(id).unwrap().pos.y - 16.0) / 128.0).floor();
        assert_eq!(source_row(die), source_row(keeper));
        game.step(&InputFrame {
            pay_pressed: true,
            ..Default::default()
        });
        assert_eq!(game.player.data.get("bet").copied(), Some(1500.0));
        for _ in 0..128 {
            if game.world.entity(die).unwrap().value("rolled") != 0.0 {
                break;
            }
            game.step(&InputFrame::default());
        }
        assert_eq!(game.world.entity(die).unwrap().value("rolled"), 1.0);
        assert_eq!(game.world.entity(untouched).unwrap().value("rolled"), 0.0);
        assert_eq!(game.player.data.get("bet").copied(), Some(1500.0));
        assert_eq!(game.data.get("thiefLevel").copied().unwrap_or(0.0), 0.0);
        assert_eq!(game.world.entity(keeper).unwrap().value("angered"), 0.0);
        game.step(&InputFrame {
            down: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert_eq!(game.player.held, Some(die));
        game.step(&InputFrame::default());
        game.step(&InputFrame {
            attack_pressed: true,
            ..Default::default()
        });
        assert!(game.player.held.is_none());
        let mut was_rolling = game.world.entity(die).unwrap().value("rolling") != 0.0;
        let mut rerolled = false;
        for _ in 0..128 {
            let entity = game.world.entity(die).unwrap();
            was_rolling |= entity.value("rolling") != 0.0;
            if was_rolling && entity.value("rolling") == 0.0 {
                assert!(
                    entity.pos.x >= 176.0 && entity.pos.x < 336.0,
                    "settlement stays in keeper room, excluding theft-before-body path"
                );
                assert_eq!(game.data.get("thiefLevel").copied(), Some(2.0));
                assert_eq!(game.world.entity(keeper).unwrap().state, 2);
                assert_eq!(game.world.entity(keeper).unwrap().value("angered"), 0.0);
                assert!(game.events.iter().any(|e| matches!(e,
                    GameEvent::Dialogue(message) if message.first == "COME BACK HERE, THIEF!"
                )));
                game.step(&InputFrame::default());
                assert_eq!(game.world.entity(keeper).unwrap().value("angered"), 1.0);
                assert_eq!(game.data.get("thiefLevel").copied(), Some(2.0));
                rerolled = true;
                break;
            }
            game.step(&InputFrame::default());
        }
        assert!(
            was_rolling && rerolled,
            "ordinary pickup/throw must establish and settle reroll"
        );
    }
}
