//! Source-derived own Damsel Whip/WhipPre interactions in authored rooms.
//! Terrain and actor births are initial scene context, not generated routes.
//! After setup, only ordinary InputFrame actions change the game. The budget
//! and bloodless controls explicitly author those two initial fields.
use spelunky_core::{
    Area, Entity, Game, GameEvent, InputFrame, Rng, RngAlgorithm, RulesPreset, Vec2, World,
};

const PRESETS: [RulesPreset; 2] = [RulesPreset::Classic, RulesPreset::Assisted];
const ALGORITHMS: [RngAlgorithm; 2] = [RngAlgorithm::Gm8Lcg32V1, RngAlgorithm::LegacySplitMix64V1];

struct Scene {
    rear: bool,
    weapon: Option<&'static str>,
    stock: Option<bool>,
    exit: bool,
    detritus: usize,
    bloodless: bool,
    budget: f64,
}

impl Default for Scene {
    fn default() -> Self {
        Self {
            rear: false,
            weapon: Some("oMachete"),
            stock: None,
            exit: false,
            detritus: 0,
            bloodless: false,
            budget: 4.0,
        }
    }
}

fn arena(rules: RulesPreset, algorithm: RngAlgorithm, scene: &Scene) -> (Game, u64, Option<u64>) {
    let mut game = Game::shell_with_algorithm(23, rules, algorithm).unwrap();
    game.world = World::new(24, 20, Area::Mines, 1);
    game.world.name = "rLevel".into();
    // Native HardBlock prevents a Mattock hit from destroying the support or
    // creating unrelated debris/RNG. No floor is changed after simulation.
    for x in 0..24 {
        game.world.set_tile(x, 10, "oHardBlock");
    }
    game.player.pos = Vec2::new(88.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game.world.entrance = game.player.pos;
    let keeper = scene
        .stock
        .map(|_| game.spawn("oShopkeeper", Vec2::new(144.0, 144.0)));
    let target_pos = Vec2::new(if scene.rear { 72.0 } else { 104.0 }, 152.0);
    if scene.exit {
        game.spawn("oExit", Vec2::new(target_pos.x - 8.0, 144.0));
    }
    let target = game.spawn("oDamsel", target_pos);
    {
        let actor = game.world.entity_mut(target).unwrap();
        if let Some(sale) = scene.stock {
            // scrRoomGen's shop stock overrides, plus an otherwise identical
            // free-stock control. Neither its HP nor motion is authored.
            actor.state = 5;
            actor.data.insert("forSale".into(), f64::from(sale));
            actor
                .data
                .insert("cost".into(), if sale { 15000.0 } else { 0.0 });
        }
        if scene.bloodless {
            actor.data.insert("bloodless".into(), 1.0);
        }
        if scene.budget != 4.0 {
            actor.data.insert("blood_left".into(), scene.budget);
        }
    }
    let weapon = scene.weapon.map(|object| {
        let id = game.spawn(object, game.player.pos);
        // Free item producer context; paying for a weapon is outside this test.
        let item = game.world.entity_mut(id).unwrap();
        item.data.insert("cost".into(), 0.0);
        item.data.insert("forSale".into(), 0.0);
        id
    });
    for _ in 0..scene.detritus {
        game.spawn("oBone", Vec2::new(240.0, 64.0));
    }
    // Let the initial controller establish floor support through normal Step.
    for _ in 0..2 {
        game.step(&InputFrame::default());
    }
    assert!(game.player.grounded);
    assert_eq!(game.player.pos, Vec2::new(88.0, 152.0));
    if let Some(id) = weapon {
        game.step(&InputFrame {
            down: true,
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert_eq!(
            game.player.held,
            Some(id),
            "ordinary weapon pickup must succeed"
        );
        assert!(game.world.entity(id).unwrap().held);
        game.step(&InputFrame {
            attack_released: true,
            ..Default::default()
        });
        assert_eq!(game.player.held, Some(id));
    }
    let actor = game
        .world
        .entity(target)
        .expect("initial actor remains alive");
    assert_eq!(actor.health, 4);
    assert_eq!(actor.value("hit_cooldown"), 0.0);
    if scene.exit {
        assert_eq!(actor.state, 4, "the Damsel must rescue naturally");
        assert_eq!(actor.sprite, "sDamselExit2");
        assert_eq!(actor.value("invincible"), 1.0);
        assert_eq!(game.data.get("damsels").copied().unwrap_or(0.0), 1.0);
    } else if scene.stock.is_some() {
        assert_eq!(actor.state, 5);
        assert_eq!(
            actor.value("image_xscale"),
            if scene.rear { -1.0 } else { 1.0 },
            "source SLAVE facing and actual prior Draw must establish scale"
        );
    } else {
        assert_eq!(actor.state, 0);
        assert_eq!(actor.value("image_xscale"), 1.0);
    }
    if let Some(id) = keeper {
        assert_eq!(game.world.entity(id).unwrap().state, 0);
    }
    (game, target, keeper)
}

fn attack_object(weapon: Option<&str>, rear: bool) -> &'static str {
    match (weapon, rear) {
        (Some("oMachete"), true) => "oMachetePre",
        (Some("oMachete"), false) => "oSlash",
        (Some("oMattock"), true) => "oMattockPre",
        (Some("oMattock"), false) => "oMattockHit",
        (None, true) => "oWhipPre",
        (None, false) => "oWhip",
        _ => unreachable!(),
    }
}

fn blood_ids(game: &Game) -> Vec<u64> {
    game.world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oBlood")
        .map(|e| e.id)
        .collect()
}

fn first_hit(game: &mut Game, target: u64, expected_attack: &str) -> (Rng, Vec2, Vec<u64>) {
    let mut observed_overlap = false;
    for frame in 0..18 {
        let before = game.rng.clone();
        let actor = game.world.entity(target).unwrap();
        let center = Vec2::new(
            actor.pos.x + 8.0 * actor.value("image_xscale"),
            actor.pos.y + 8.0,
        );
        let old_blood = blood_ids(game);
        game.step(&InputFrame {
            attack: frame == 0,
            attack_pressed: frame == 0,
            ..Default::default()
        });
        // Draw precedes AnimationEnd: the Slash that contacted the Damsel
        // may already be retired in the authoritative post-Step world.
        let shown = game.presentation();
        let shown_actor = shown
            .world
            .entity(target)
            .expect("actor remains visible before AnimationEnd");
        let overlap = shown
            .world
            .entities
            .iter()
            .any(|e| e.alive && e.object == expected_attack && e.event_intersects(shown_actor));
        observed_overlap |= overlap;
        let actor = game
            .world
            .entity(target)
            .expect("actor survives the first hit frame");
        if actor.value("hit_cooldown") == 10.0 {
            assert!(
                overlap,
                "ordinary attack must present the selected contacting producer {expected_attack}"
            );
            assert_eq!(game.player.health, 4.0, "no unrelated player contact");
            return (before, center, old_blood);
        }
        assert_eq!(
            actor.value("hit_cooldown"),
            0.0,
            "no earlier collision may establish a cooldown"
        );
    }
    assert!(
        observed_overlap,
        "ordinary attack must genuinely overlap the Damsel before checking its ignored callback"
    );
    panic!("ordinary attack did not reach its own Damsel callback: {expected_attack}");
}

fn source_sounds(game: &Game) {
    let sounds: Vec<_> = game
        .events
        .iter()
        .filter_map(|event| match event {
            GameEvent::Sound { name, .. } => Some(name.as_str()),
            _ => None,
        })
        .collect();
    assert!(
        sounds.len() >= 2,
        "accepted callback must emit sndHit then sndDamsel"
    );
    assert_eq!(&sounds[sounds.len() - 2..], &["sndHit", "sndDamsel"]);
}

fn own_fields(actor: &Entity) {
    assert_eq!(
        actor.value("assaulted"),
        0.0,
        "own melee does not inherit Enemy assault flags"
    );
    assert_eq!(actor.value("killed_by_player"), 0.0);
    assert!(!actor.held);
}

fn source_blood(game: &Game, before: &Rng, center: Vec2, old_ids: &[u64], emitted: bool) {
    let drops: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oBlood" && !old_ids.contains(&e.id))
        .collect();
    assert_eq!(
        drops.len(),
        usize::from(emitted),
        "source guarded Blood birth count"
    );
    let mut expected = before.clone();
    if emitted {
        // Independent literal oBlood Create oracle: exactly four RNG draws.
        let velocity = Vec2::new(
            expected.unit() * 4.0 - expected.unit() * 4.0,
            -1.0 - expected.unit() * 2.0,
        );
        let gravity = f64::from(1 + expected.range(6)) * 0.1;
        assert_eq!(
            drops[0].pos, center,
            "signed current-sprite center at callback birth"
        );
        assert!((drops[0].vel.x - velocity.x).abs() < 1e-12);
        assert!((drops[0].vel.y - velocity.y).abs() < 1e-12);
        assert!((drops[0].value("gravity") - gravity).abs() < 1e-12);
    }
    assert_eq!(
        game.rng.clone().next_u64(),
        expected.next_u64(),
        "only permitted Blood Create consumes RNG"
    );
}

#[test]
fn ordinary_machete_pickup_front_and_rear_create_guarded_blood_and_two_sounds() {
    for rules in PRESETS {
        for algorithm in ALGORITHMS {
            for rear in [false, true] {
                let scene = Scene {
                    rear,
                    ..Default::default()
                };
                let (mut game, target, _) = arena(rules, algorithm, &scene);
                let (rng, center, old_ids) =
                    first_hit(&mut game, target, attack_object(scene.weapon, rear));
                let actor = game.world.entity(target).unwrap();
                assert_eq!((actor.health, actor.state), (2, 0));
                assert_eq!(actor.value("blood_left"), 4.0);
                source_blood(&game, &rng, center, &old_ids, true);
                source_sounds(&game);
                own_fields(actor);
                game.step(&InputFrame::default());
                let actor = game.world.entity(target).unwrap();
                assert_eq!(
                    actor.health, 2,
                    "retained attack cannot hit during source immunity"
                );
                assert_eq!(actor.value("hit_cooldown"), 9.0);
            }
        }
    }
}

#[test]
fn guarded_machete_blood_obeys_detritus_cap_and_controlled_initial_flags() {
    for rules in PRESETS {
        for algorithm in ALGORITHMS {
            for rear in [false, true] {
                for (detritus, bloodless, budget) in [
                    (15, false, 4.0),
                    (16, false, 4.0),
                    (0, true, 4.0),
                    (0, false, 0.0),
                ] {
                    let scene = Scene {
                        rear,
                        detritus,
                        bloodless,
                        budget,
                        ..Default::default()
                    };
                    let (mut game, target, _) = arena(rules, algorithm, &scene);
                    let (rng, center, old_ids) =
                        first_hit(&mut game, target, attack_object(scene.weapon, rear));
                    let actor = game.world.entity(target).unwrap();
                    assert_eq!(actor.health, 2);
                    assert_eq!(
                        actor.value("blood_left"),
                        budget,
                        "positive HP does not exhaust budget"
                    );
                    assert_eq!(
                        game.world
                            .entities
                            .iter()
                            .filter(|e| e.alive && e.engine_active && e.object == "oBone")
                            .count(),
                        detritus,
                        "authored active Bones must still establish the source cap prerequisite"
                    );
                    own_fields(actor);
                    source_sounds(&game);
                    source_blood(
                        &game,
                        &rng,
                        center,
                        &old_ids,
                        detritus < 16 && !bloodless && budget > 0.0,
                    );
                }
            }
        }
    }
}

#[test]
fn three_ordinary_machete_attacks_exhaust_budget_only_after_negative_hp() {
    for rules in PRESETS {
        for algorithm in ALGORITHMS {
            for rear in [false, true] {
                let scene = Scene {
                    rear,
                    ..Default::default()
                };
                let (mut game, target, _) = arena(rules, algorithm, &scene);
                for (hp, budget) in [(2, 4.0), (0, 4.0), (-2, 3.0)] {
                    let (rng, center, old_ids) =
                        first_hit(&mut game, target, attack_object(scene.weapon, rear));
                    let actor = game.world.entity(target).unwrap();
                    assert_eq!(actor.health, hp);
                    assert_eq!(actor.value("blood_left"), budget);
                    source_blood(&game, &rng, center, &old_ids, true);
                    source_sounds(&game);
                    own_fields(actor);
                    for _ in 0..24 {
                        if game.player.attack_timer == 0
                            && game.world.entity(target).unwrap().value("hit_cooldown") == 0.0
                        {
                            break;
                        }
                        game.step(&InputFrame::default());
                    }
                    assert_eq!(game.player.attack_timer, 0);
                    assert_eq!(
                        game.world.entity(target).unwrap().value("hit_cooldown"),
                        0.0
                    );
                }
            }
        }
    }
}

#[test]
fn ordinary_whip_and_mattock_front_and_rear_bump_without_damage_or_blood() {
    for rules in PRESETS {
        for algorithm in ALGORITHMS {
            for rear in [false, true] {
                for weapon in [None, Some("oMattock")] {
                    let scene = Scene {
                        rear,
                        weapon,
                        ..Default::default()
                    };
                    let (mut game, target, _) = arena(rules, algorithm, &scene);
                    first_hit(&mut game, target, attack_object(weapon, rear));
                    let actor = game.world.entity(target).unwrap();
                    assert_eq!((actor.health, actor.state), (4, 0));
                    assert_eq!(actor.vel, Vec2::new(0.0, -2.0));
                    assert_eq!(actor.value("blood_left"), 4.0);
                    assert!(blood_ids(&game).is_empty());
                    own_fields(actor);
                    source_sounds(&game);
                }
            }
        }
    }
}

#[test]
fn ordinary_stock_bump_angers_keeper_only_for_sale_stock() {
    for rules in PRESETS {
        for algorithm in ALGORITHMS {
            for rear in [false, true] {
                for weapon in [None, Some("oMattock")] {
                    for sale in [true, false] {
                        let scene = Scene {
                            rear,
                            weapon,
                            stock: Some(sale),
                            ..Default::default()
                        };
                        let (mut game, target, keeper) = arena(rules, algorithm, &scene);
                        first_hit(&mut game, target, attack_object(weapon, rear));
                        let actor = game.world.entity(target).unwrap();
                        assert_eq!((actor.health, actor.state), (4, 5));
                        assert_eq!(actor.vel.y, -2.0);
                        assert!(blood_ids(&game).is_empty());
                        assert_eq!(
                            game.world.entity(keeper.unwrap()).unwrap().state,
                            if sale { 2 } else { 0 }
                        );
                        assert_eq!(
                            game.data.get("thiefLevel").copied().unwrap_or(0.0),
                            if sale { 2.0 } else { 0.0 }
                        );
                        assert_eq!(game.flags.contains("wanted"), sale);
                        assert_eq!(game.events.iter().any(|event| matches!(event,
            GameEvent::Dialogue(message) if message.first == "HEY, ONLY I CAN DO THAT!" && message.second.is_empty() && message.ticks == 80)), sale);
                        source_sounds(&game);
                        own_fields(actor);
                    }
                }
            }
        }
    }
}

#[test]
fn ordinary_machete_stock_hit_uses_actual_prior_draw_scale_and_skips_bump_anger() {
    for rules in PRESETS {
        for algorithm in ALGORITHMS {
            for rear in [false, true] {
                let scene = Scene {
                    rear,
                    stock: Some(true),
                    ..Default::default()
                };
                let (mut game, target, keeper) = arena(rules, algorithm, &scene);
                let (rng, center, old_ids) =
                    first_hit(&mut game, target, attack_object(scene.weapon, rear));
                let actor = game.world.entity(target).unwrap();
                assert_eq!((actor.health, actor.state), (2, 5));
                assert_eq!(actor.value("image_xscale"), if rear { -1.0 } else { 1.0 });
                assert_eq!(center.x, actor.pos.x + if rear { -8.0 } else { 8.0 });
                source_blood(&game, &rng, center, &old_ids, true);
                source_sounds(&game);
                own_fields(actor);
                assert_eq!(game.world.entity(keeper.unwrap()).unwrap().state, 0);
                assert_eq!(game.data.get("thiefLevel").copied().unwrap_or(0.0), 0.0);
                assert!(!game.flags.contains("wanted"));
            }
        }
    }
}

#[test]
fn naturally_rescued_damsel_accepts_machete_during_invincible_exit_animation() {
    for rules in PRESETS {
        for algorithm in ALGORITHMS {
            for rear in [false, true] {
                let scene = Scene {
                    rear,
                    exit: true,
                    ..Default::default()
                };
                let (mut game, target, _) = arena(rules, algorithm, &scene);
                let (rng, center, old_ids) =
                    first_hit(&mut game, target, attack_object(scene.weapon, rear));
                let actor = game.world.entity(target).unwrap();
                assert_eq!((actor.health, actor.state), (2, 4));
                assert_eq!(actor.value("invincible"), 1.0);
                assert_eq!(actor.value("blood_left"), 4.0);
                assert_eq!(game.data.get("damsels").copied().unwrap_or(0.0), 1.0);
                source_blood(&game, &rng, center, &old_ids, true);
                source_sounds(&game);
                own_fields(actor);
            }
        }
    }
}

#[test]
fn naturally_rescued_damsel_ignores_overlapping_whip_and_mattock() {
    for rules in PRESETS {
        for algorithm in ALGORITHMS {
            for rear in [false, true] {
                for weapon in [None, Some("oMattock")] {
                    let scene = Scene {
                        rear,
                        weapon,
                        exit: true,
                        ..Default::default()
                    };
                    let (mut game, target, _) = arena(rules, algorithm, &scene);
                    let expected_attack = attack_object(weapon, rear);
                    let mut overlap = false;
                    for frame in 0..12 {
                        game.step(&InputFrame {
                            attack: frame == 0,
                            attack_pressed: frame == 0,
                            ..Default::default()
                        });
                        let shown = game.presentation();
                        let actor = shown
                            .world
                            .entity(target)
                            .expect("Exit remains alive before its animation ends");
                        overlap |= shown.world.entities.iter().any(|e| {
                            e.alive && e.object == expected_attack && e.event_intersects(actor)
                        });
                        let actor = game.world.entity(target).unwrap();
                        assert_eq!((actor.health, actor.state), (4, 4));
                        assert_eq!(actor.value("hit_cooldown"), 0.0);
                        assert_eq!(actor.vel, Vec2::default());
                        assert!(blood_ids(&game).is_empty());
                        assert!(!game.events.iter().any(|event| matches!(event,
                GameEvent::Sound { name, .. } if name == "sndHit" || name == "sndDamsel")));
                    }
                    assert!(
                        overlap,
                        "native attack must genuinely overlap the ignored Exit mask"
                    );
                }
            }
        }
    }
}
