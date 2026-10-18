use spelunky_core::{Entity, Game, GameEvent, InputFrame, Rng, RulesPreset, Vec2, level::Level};

fn room(rules: RulesPreset) -> Game {
    let mut level = Level::default();
    level.set_tile(5, 6, '@').unwrap();
    level.set_tile(34, 6, 'X').unwrap();
    for x in 0..spelunky_core::level::LEVEL_WIDTH {
        level.set_tile(x, 7, '1').unwrap();
    }
    let mut game = Game::shell(1, rules);
    game.load_custom(&level, "shop-melee-source", "");
    assert_eq!(game.player.pos, Vec2::new(104.0, 120.0));
    game
}

fn neutral(game: &mut Game, count: usize) {
    for _ in 0..count {
        game.step(&InputFrame::default());
        assert_eq!(
            game.player.health, 4.0,
            "ordinary route must avoid hostile body/projectile damage"
        );
    }
}

fn acquire(game: &mut Game, id: u64) {
    assert!(game.player.grounded);
    game.step(&InputFrame {
        down: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert_eq!(
        game.player.held,
        Some(id),
        "ordinary Down+Attack acquires the intended weapon"
    );
    neutral(game, 1);
}

fn blood_create(rng: &mut Rng) -> (Vec2, f64) {
    (
        Vec2::new(rng.unit() * 4.0 - rng.unit() * 4.0, -1.0 - rng.unit() * 2.0),
        f64::from(1 + rng.range(6)) * 0.1,
    )
}

fn first_hit(game: &mut Game, keeper: u64, expected_weapon: &str) -> (Entity, Entity, Rng) {
    assert_eq!(
        game.world.entity(keeper).unwrap().value("hit_cooldown"),
        0.0
    );
    assert!(!game.world.entities.iter().any(|e| e.object == "oBlood"));
    for frame in 0..18 {
        let before = game.rng.clone();
        game.step(&InputFrame {
            attack: frame == 0,
            attack_pressed: frame == 0,
            ..Default::default()
        });
        assert_eq!(
            game.player.health, 4.0,
            "hit fixture must retain ordinary player health"
        );
        let actor = game
            .world
            .entity(keeper)
            .expect("keeper survives its first melee collision frame");
        if actor.value("hit_cooldown") != 0.0 {
            assert_eq!(actor.value("hit_cooldown"), 10.0);
            let contact_view = game.presentation();
            assert_eq!(
                contact_view.tick, game.tick,
                "contact snapshot must belong to this actual Step"
            );
            assert_eq!(
                contact_view.dispatcher.phase,
                spelunky_core::dispatcher::Phase::Draw,
                "observe source collision masks before automatic animation advancement"
            );
            let contact_actor = contact_view
                .world
                .entity(keeper)
                .expect("contact receiver exists in pre-animation snapshot");
            let weapons: Vec<_> = contact_view
                .world
                .entities
                .iter()
                .filter(|weapon| {
                    weapon.alive
                        && weapon.object == expected_weapon
                        && weapon.event_intersects(contact_actor)
                })
                .collect();
            assert_eq!(
                weapons.len(),
                1,
                "ordinary input must create the identified actual contacting weapon"
            );
            assert_eq!(actor.alarms[0], 10);
            assert_eq!(actor.state, 2);
            assert_eq!(
                actor.vel,
                Vec2::new(
                    if weapons[0].pos.x < actor.pos.x {
                        1.0
                    } else {
                        -1.0
                    },
                    -2.0
                )
            );
            assert!(
                game.events.iter().any(
                    |event| matches!(event, GameEvent::Sound { name, .. } if name == "sndHit")
                )
            );
            return (actor.clone(), weapons[0].clone(), before);
        }
        assert!(
            !game.world.entities.iter().any(|e| e.object == "oBlood"),
            "no earlier Blood producer"
        );
    }
    panic!("ordinary inputs failed to contact keeper with {expected_weapon}");
}

fn bloods(game: &Game) -> Vec<&Entity> {
    game.world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oBlood")
        .collect()
}

fn assert_machete_blood(game: &Game, actor: &Entity, before: &mut Rng, emits: bool) {
    let blood = bloods(game);
    assert_eq!(
        blood.len(),
        usize::from(emits),
        "SOURCE_KEEPER_MACHETE_GUARDED_BLOOD_COUNT"
    );
    if emits {
        let (vel, grav) = blood_create(before);
        assert_eq!(
            blood[0].age, 0,
            "automatic Collision birth follows the already-passed Blood Step"
        );
        assert_eq!(
            spelunky_core::catalog::sprite_dimensions(&actor.sprite),
            Some((16, 16))
        );
        assert_eq!(
            blood[0].pos,
            Vec2::new(actor.pos.x + 8.0, actor.pos.y + 8.0)
        );
        assert_eq!(blood[0].vel, vel);
        assert_eq!(blood[0].value("gravity"), grav);
        assert_eq!(blood[0].alarms[0..3], [1, 1, 5]);
    }
    assert_eq!(
        game.rng.clone().next_u64(),
        before.next_u64(),
        "SOURCE_KEEPER_MELEE_BLOOD_CREATE_RNG_ORDER"
    );
}

#[test]
fn ordinary_machete_pickup_hits_keeper_front_and_rear_with_guarded_blood() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for rear in [false, true] {
            let mut game = room(rules);
            let machete = game.spawn("oMachete", game.player.pos);
            let keeper = game.spawn(
                "oShopkeeper",
                Vec2::new(if rear { 80.0 } else { 120.0 }, 112.0),
            );
            assert_eq!(game.world.entity(keeper).unwrap().health, 20);
            neutral(&mut game, 2);
            acquire(&mut game, machete);
            let (actor, _, mut before) = first_hit(
                &mut game,
                keeper,
                if rear { "oMachetePre" } else { "oSlash" },
            );
            assert_eq!(actor.health, 18);
            assert_eq!(actor.value("blood_left"), 4.0);
            assert_machete_blood(&game, &actor, &mut before, true);
            assert_eq!(actor.value("whipped"), 1.0);
            assert_eq!(actor.value("assaulted"), 0.0);
            assert_eq!(actor.value("killed_by_player"), 0.0);
            assert_eq!(
                actor.value("angered"),
                0.0,
                "own ATTACK Step has not run on collision frame"
            );
        }
    }
}

#[test]
fn ordinary_machete_hits_moving_keeper2_through_inherited_own_handler() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = room(rules);
        let machete = game.spawn("oMachete", game.player.pos);
        let keeper = game.spawn("oShopkeeper2", Vec2::new(148.0, 112.0));
        assert_eq!(game.world.entity(keeper).unwrap().state, 2);
        assert_eq!(game.world.entity(keeper).unwrap().value("has_gun"), 0.0);
        neutral(&mut game, 2);
        acquire(&mut game, machete);
        let (actor, _, mut before) = first_hit(&mut game, keeper, "oSlash");
        assert!(
            actor.pos.x < 148.0,
            "Keeper2 moves naturally from source Create ATTACK state"
        );
        assert_eq!(actor.health, 18);
        assert_eq!(actor.value("blood_left"), 4.0);
        assert_machete_blood(&game, &actor, &mut before, true);
        assert_eq!(actor.value("whipped"), 1.0);
    }
}

#[test]
fn ordinary_plain_whip_front_and_rear_bump_keeper_without_damage_or_blood() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for rear in [false, true] {
            let mut game = room(rules);
            let keeper = game.spawn(
                "oShopkeeper",
                Vec2::new(if rear { 80.0 } else { 120.0 }, 112.0),
            );
            neutral(&mut game, 2);
            let (actor, _, mut before) =
                first_hit(&mut game, keeper, if rear { "oWhipPre" } else { "oWhip" });
            assert_eq!(actor.health, 20);
            assert_eq!(actor.value("blood_left"), 4.0);
            assert_machete_blood(&game, &actor, &mut before, false);
        }
    }
}

#[test]
fn ordinary_mattock_front_and_rear_bump_keeper_without_damage_or_blood() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for rear in [false, true] {
            let mut game = room(rules);
            let mattock = game.spawn("oMattock", game.player.pos);
            let keeper = game.spawn(
                "oShopkeeper",
                Vec2::new(if rear { 80.0 } else { 120.0 }, 112.0),
            );
            neutral(&mut game, 2);
            acquire(&mut game, mattock);
            let (actor, _, mut before) = first_hit(
                &mut game,
                keeper,
                if rear { "oMattockPre" } else { "oMattockHit" },
            );
            assert_eq!(
                actor.health, 20,
                "source own type guard ignores Mattock even when puncture is true"
            );
            assert_eq!(actor.value("blood_left"), 4.0);
            assert_machete_blood(&game, &actor, &mut before, false);
        }
    }
}

#[test]
fn keeper_melee_preserves_other_idle_keeper_and_runs_only_own_attack_consequence() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for machete in [false, true] {
            let mut game = room(rules);
            let weapon = machete.then(|| game.spawn("oMachete", game.player.pos));
            let keeper = game.spawn("oShopkeeper", Vec2::new(120.0, 112.0));
            let other = game.spawn("oShopkeeper", Vec2::new(240.0, 112.0));
            let stock = game.spawn("oJetpack", Vec2::new(288.0, 120.0));
            // Controlled initial shop stock, preceding every Step; actors otherwise retain source Creates.
            game.world
                .entity_mut(stock)
                .unwrap()
                .data
                .insert("cost".into(), 20000.0);
            game.world
                .entity_mut(stock)
                .unwrap()
                .data
                .insert("forSale".into(), 1.0);
            neutral(&mut game, 2);
            if let Some(weapon) = weapon {
                acquire(&mut game, weapon);
            }
            let (actor, _, _) =
                first_hit(&mut game, keeper, if machete { "oSlash" } else { "oWhip" });
            assert_eq!(actor.health, if machete { 18 } else { 20 });
            assert_eq!(actor.value("angered"), 0.0);
            assert_eq!(game.world.entity(other).unwrap().state, 0);
            assert_eq!(game.world.entity(stock).unwrap().value("cost"), 20000.0);
            game.step(&InputFrame::default());
            assert_eq!(
                game.world.entity(other).unwrap().state,
                0,
                "SOURCE_OWN_MELEE_MUST_NOT_GLOBALLY_ANGER_UNRELATED_IDLE_KEEPER"
            );
            assert_eq!(game.world.entity(other).unwrap().value("angered"), 0.0);
            assert_eq!(game.world.entity(keeper).unwrap().value("angered"), 1.0);
            assert_eq!(
                game.world.entity(stock).unwrap().value("cost"),
                0.0,
                "source first own ATTACK Step clears active Item prices"
            );
            assert_eq!(game.world.entity(stock).unwrap().value("forSale"), 0.0);
            assert!(!game.flags.contains("shopkeepers_angry"));
            assert_eq!(game.data.get("thiefLevel").copied().unwrap_or(0.0), 0.0);
        }
    }
}

#[test]
fn controlled_initial_keeper_machete_budget_exhaustion_respects_bloodless_and_cap() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for rear in [false, true] {
            for (hp, left, bloodless, cap) in [
                (1, 1.0, false, false),
                (2, 1.0, false, false),
                (1, 0.0, false, false),
                (1, 1.0, true, false),
                (1, 1.0, false, true),
            ] {
                let mut game = room(rules);
                let machete = game.spawn("oMachete", game.player.pos);
                let keeper = game.spawn(
                    "oShopkeeper",
                    Vec2::new(if rear { 80.0 } else { 120.0 }, 112.0),
                );
                // These are explicitly controlled INITIAL fields, not a natural HP/blood-budget regression.
                let actor = game.world.entity_mut(keeper).unwrap();
                actor.health = hp;
                actor.data.insert("blood_left".into(), left);
                actor.data.insert("bloodless".into(), f64::from(bloodless));
                actor.data.insert("counts_as_kill".into(), 0.0);
                actor.data.insert("environmental_death".into(), 1.0);
                if cap {
                    for x in 0..16 {
                        game.spawn("oDetritus", Vec2::new(176.0 + f64::from(x) * 2.0, 48.0));
                    }
                }
                neutral(&mut game, 2);
                acquire(&mut game, machete);
                let waiting = game.world.entity(keeper).unwrap();
                assert_eq!(
                    waiting.health, hp,
                    "controlled fixture must preserve intended pre-attack HP"
                );
                assert_eq!(
                    waiting.value("blood_left"),
                    left,
                    "controlled fixture must preserve intended pre-attack blood budget"
                );
                assert_eq!(waiting.value("bloodless"), f64::from(bloodless));
                let (actor, _, mut before) = first_hit(
                    &mut game,
                    keeper,
                    if rear { "oMachetePre" } else { "oSlash" },
                );
                assert_eq!(actor.health, hp - 2);
                assert_eq!(
                    actor.value("bloodless"),
                    f64::from(bloodless),
                    "controlled initial bloodless field remains source-authoritative"
                );
                if cap {
                    assert_eq!(
                        game.world
                            .entities
                            .iter()
                            .filter(|particle| particle.alive
                                && particle.engine_active
                                && spelunky_core::catalog::is_a(&particle.object, "oDetritus"))
                            .count(),
                        16,
                        "CONTROLLED_CAP_FIXTURE_MUST_RETAIN_16_ACTUAL_ACTIVE_DETRITUS_AT_CONTACT"
                    );
                }
                let emits = left > 0.0 && !bloodless && !cap;
                assert_machete_blood(&game, &actor, &mut before, emits);
                assert_eq!(
                    actor.value("blood_left"),
                    left - f64::from(left > 0.0 && hp - 2 < 0),
                    "SOURCE_KEEPER_POST_HP_BUDGET_EXHAUSTION_EVEN_WHEN_BLOOD_IS_SUPPRESSED"
                );
                assert_eq!(
                    actor.value("counts_as_kill"),
                    0.0,
                    "source own melee does not rewrite preexisting kill-credit disposition"
                );
                assert_eq!(actor.value("environmental_death"), 1.0);
                assert_eq!(actor.value("assaulted"), 0.0);
                assert_eq!(actor.value("killed_by_player"), 0.0);
                assert_eq!(actor.value("dead"), 0.0, "own death Step has not run yet");
                assert_eq!(actor.value("whipped"), 1.0);
            }
        }
    }
}
