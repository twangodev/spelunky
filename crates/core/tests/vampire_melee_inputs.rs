//! Source-contract proposals only: default Vampire attacks and ordinary melee.
//! Initial controlled Blood settings are explicitly separate from natural cases.
use spelunky_core::*;

fn modes() -> impl Iterator<Item = (RulesPreset, RngAlgorithm)> {
    [RulesPreset::Classic, RulesPreset::Assisted]
        .into_iter()
        .flat_map(|rules| {
            [RngAlgorithm::Gm8Lcg32V1, RngAlgorithm::LegacySplitMix64V1]
                .into_iter()
                .map(move |algorithm| (rules, algorithm))
        })
}
fn room(
    rules: RulesPreset,
    algorithm: RngAlgorithm,
    rear: bool,
    machete: bool,
    controlled_blood: bool,
    capped: bool,
) -> (Game, u64, Option<u64>) {
    let mut game = Game::shell_with_algorithm(42, rules, algorithm).unwrap();
    game.world = World::new(32, 20, Area::Jungle, 5);
    game.world.name = "rLevel".into();
    for x in 0..32 {
        game.world.set_tile(x, 10, "oBlock");
    }
    game.player.pos = Vec2::new(104.0, 152.0);
    game.player.previous_pos = game.player.pos;
    let weapon = machete.then(|| game.spawn("oMachete", game.player.pos));
    let id = game.spawn(
        "oVampire",
        Vec2::new(if rear { 24.0 } else { 168.0 }, 144.0),
    );
    let target = game.world.entity(id).unwrap();
    assert_eq!((target.health, target.state, target.timer), (6, 0, 0));
    assert_eq!(target.vel, Vec2::new(2.5, 0.0));
    assert_eq!(target.value("bloodless"), 1.0);
    assert_eq!(target.value("stun_time"), 60.0);
    if controlled_blood {
        // Deliberate initial callback-contract settings, before the first Step.
        // No natural source route to bloodless=false is claimed.
        let target = game.world.entity_mut(id).unwrap();
        target.data.insert("bloodless".into(), 0.0);
        target.data.insert("blood_left".into(), 0.0);
        target.health = 1;
    }
    if capped {
        // Controlled initial cap population, before any Step. Native Bone
        // constructors still run and consume their normal random draws. Static
        // initial fields keep these concrete particles alive for interception;
        // this does not claim an untouched native Bone launch trajectory.
        for _ in 0..16 {
            let bone = game.spawn("oBone", Vec2::new(280.0, 80.0));
            let bone = game.world.entity_mut(bone).unwrap();
            assert!(bone.alive && bone.engine_active);
            assert_eq!(bone.value("dying"), 0.0);
            bone.vel = Vec2::new(0.0, 0.0);
            bone.data.insert("gravity".into(), 0.0);
            bone.data.insert("bounce".into(), 0.0);
            bone.data.insert("life".into(), 200.0);
        }
        assert_cap_population(&game);
    }
    (game, id, weapon)
}
fn assert_cap_population(game: &Game) {
    let participants: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && e.engine_active && catalog::is_a(&e.object, "oDetritus"))
        .collect();
    assert_eq!(
        participants.len(),
        16,
        "CONTROLLED_ACTIVE_DETRITUS_CAP_PREREQUISITE"
    );
    assert!(
        participants.iter().all(|e| e.object == "oBone"
            && e.pos == Vec2::new(280.0, 80.0)
            && e.vel == Vec2::new(0.0, 0.0)
            && e.value("dying") == 0.0
            && e.value("life") > 0.0),
        "all sixteen declared initial Bones remain live at the cap check"
    );
}
fn step(game: &mut Game, input: InputFrame) {
    game.step(&input);
    assert_eq!(game.state, RunState::Playing);
    assert_eq!(
        game.player.health, 4.0,
        "NATURAL_VAMPIRE_MELEE_HEALTH_PREREQUISITE"
    );
}
fn first_hit(
    rules: RulesPreset,
    algorithm: RngAlgorithm,
    rear: bool,
    machete: bool,
    controlled_blood: bool,
    capped: bool,
) -> (Game, u64, Rng, i32) {
    let (mut game, id, weapon) = room(rules, algorithm, rear, machete, controlled_blood, capped);
    step(&mut game, InputFrame::default());
    if let Some(weapon) = weapon {
        step(
            &mut game,
            InputFrame {
                down: true,
                attack: true,
                attack_pressed: true,
                ..Default::default()
            },
        );
        assert_eq!(game.player.held, Some(weapon), "ORDINARY_MACHETE_PICKUP");
        step(&mut game, InputFrame::default());
    }
    let initial_health = game.world.entity(id).unwrap().health;
    let mut swung = false;
    let mut saw_projectile = false;
    for _ in 0..80 {
        let before = game.world.entity(id).unwrap().clone();
        assert_eq!(before.health, initial_health, "first real attack only");
        let gap = if rear {
            game.player.pos.x - before.pos.x
        } else {
            before.pos.x - game.player.pos.x
        };
        let ready = before.state == 2
            && before.image_speed() == 1.0
            && if machete {
                (28.0..=36.0).contains(&gap)
            } else if rear {
                (36.0..=44.0).contains(&gap)
            } else {
                (40.0..=48.0).contains(&gap)
            };
        let start = !swung && ready;
        if start {
            swung = true;
        }
        if capped {
            assert_cap_population(&game);
        }
        let rng_before = game.rng.clone();
        step(
            &mut game,
            InputFrame {
                attack: start,
                attack_pressed: start,
                ..Default::default()
            },
        );
        if capped {
            assert_cap_population(&game);
        }
        saw_projectile |= game.world.entities.iter().any(|e| {
            e.alive
                && e.object
                    == if machete {
                        if rear { "oMachetePre" } else { "oSlash" }
                    } else {
                        if rear { "oWhipPre" } else { "oWhip" }
                    }
        });
        let after = game.world.entity(id).unwrap();
        if after.health < initial_health {
            assert!(
                swung && saw_projectile,
                "REAL_FRONT_OR_REAR_PROJECTILE_PREREQUISITE"
            );
            assert_eq!(
                before.state, 2,
                "natural ATTACK enters selected melee frame"
            );
            assert_eq!(before.image_speed(), 1.0);
            assert_eq!(after.health, initial_health - if machete { 2 } else { 1 });
            assert_eq!((after.state, after.timer), (98, 60));
            assert_eq!(after.vel.y, -3.0);
            assert_eq!(after.vel.x.abs(), 2.0);
            assert_eq!(
                after.value("blood_left"),
                if controlled_blood { 0.0 } else { 4.0 },
                "Vampire own event never reads or spends bloodLeft"
            );
            assert!(
                game.events.iter().any(
                    |event| matches!(event, GameEvent::Sound { name, .. } if name == "sndHit")
                )
            );
            return (game, id, rng_before, initial_health);
        }
    }
    panic!("NATURAL_VAMPIRE_FRONT_REAR_INTERCEPTION_PREREQUISITE rear={rear} machete={machete}");
}
#[test]
fn default_attack_front_and_rear_plain_whip_reset_speed_without_puncture_gate() {
    for (rules, algorithm) in modes() {
        for rear in [false, true] {
            let (game, id, oracle, _) = first_hit(rules, algorithm, rear, false, false, false);
            let target = game.world.entity(id).unwrap();
            assert_eq!(
                target.image_speed(),
                0.5,
                "SOURCE_VAMPIRE_OWN_MELEE_RESETS_ATTACK_IMAGE_SPEED"
            );
            assert_eq!(target.value("bloodless"), 1.0);
            assert!(
                !game
                    .world
                    .entities
                    .iter()
                    .any(|e| e.alive && e.object == "oBlood")
            );
            assert_eq!(
                game.rng.state_bits(),
                oracle.state_bits(),
                "default bloodless script consumes no Blood RNG"
            );
        }
    }
}
#[test]
fn ordinary_machete_pickup_hits_default_attack_through_front_and_rear_events() {
    for (rules, algorithm) in modes() {
        for rear in [false, true] {
            let (game, id, _, _) = first_hit(rules, algorithm, rear, true, false, false);
            assert_eq!(
                game.world.entity(id).unwrap().image_speed(),
                0.5,
                "SOURCE_VAMPIRE_MACHETE_RESETS_ATTACK_IMAGE_SPEED"
            );
            assert!(
                !game
                    .world
                    .entities
                    .iter()
                    .any(|e| e.alive && e.object == "oBlood")
            );
        }
    }
}
#[test]
fn controlled_bloodless_false_and_zero_budget_still_call_guarded_blood_script() {
    for (rules, algorithm) in modes() {
        for capped in [false, true] {
            let (game, id, mut oracle, _) = first_hit(rules, algorithm, false, true, true, capped);
            let target = game.world.entity(id).unwrap();
            assert_eq!(target.health, -1);
            assert_eq!(target.value("blood_left"), 0.0);
            let blood: Vec<_> = game
                .world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == "oBlood")
                .collect();
            if capped {
                assert_eq!(blood.len(), 0, "source script cap still owns admission");
            } else {
                assert_eq!(
                    blood.len(),
                    1,
                    "SOURCE_VAMPIRE_SCRIPT_CALL_HAS_NO_BLOOD_BUDGET_GATE"
                );
                let velocity = Vec2::new(
                    oracle.unit() * 4.0 - oracle.unit() * 4.0,
                    -1.0 - oracle.unit() * 2.0,
                );
                let gravity = f64::from(1 + oracle.range(6)) * 0.1;
                assert_eq!(
                    blood[0].pos,
                    Vec2::new(target.pos.x + 8.0, target.pos.y + 8.0)
                );
                assert_eq!(
                    blood[0].age, 0,
                    "Blood pair resource370 has already passed before melee576+"
                );
                assert_eq!(blood[0].vel, velocity);
                assert_eq!(blood[0].value("gravity"), gravity);
            }
            assert_eq!(game.rng.state_bits(), oracle.state_bits());
        }
    }
}
#[test]
fn naturally_stunned_vampire_ignores_plain_whip_but_allows_machete_repeat() {
    for (rules, algorithm) in modes() {
        for machete in [false, true] {
            let (mut game, id, _, initial_health) =
                first_hit(rules, algorithm, false, machete, false, false);
            let first_health = initial_health - if machete { 2 } else { 1 };
            let mut witnessed = false;
            let mut second_swing = false;
            for _ in 0..80 {
                let before = game.world.entity(id).unwrap().clone();
                assert_eq!(before.state, 98, "ordinary first hit acquired source stun");
                assert!(before.timer > 0, "SOURCE_REPEAT_STUN_TIMER_PREREQUISITE");
                assert_eq!(
                    before.health, first_health,
                    "no other damage before the earliest selected source repeat contact"
                );
                let mut input = InputFrame::default();
                if !second_swing
                    && game.player.attack_timer == 0
                    && before.vel == Vec2::new(0.0, 0.0)
                    && before.pos.y == 144.0
                {
                    // Let recoil settle naturally, then walk/brake into front
                    // reach. On this floor PlayerY152 and VampireY144 place
                    // Whip/Slash against the ordinary stunned sprite mask.
                    let gap = before.pos.x - game.player.pos.x;
                    let vx = game.player.vel.x;
                    if vx.abs() >= 0.1 {
                        input.right = vx > 0.0 && gap > 24.0;
                        input.left = vx < 0.0 && gap < 12.0;
                    } else if gap > 20.0 {
                        input.right = true;
                    } else if gap < 14.0 {
                        input.left = true;
                    } else if game.player.facing != 1 {
                        input.right = true;
                    } else {
                        assert!(game.player.grounded);
                        assert_eq!(game.player.pos.y, 152.0);
                        assert_eq!(
                            game.player.attack_timer, 0,
                            "first swing ended before ordinary second input"
                        );
                        input.attack = true;
                        input.attack_pressed = true;
                        second_swing = true;
                    }
                }
                let player_before = game.player.pos;
                let attack_timer_before = game.player.attack_timer;
                let player_sprite_before = game.player.sprite.clone();
                let player_animation_before = game.player.animation;
                let rng_before = game.rng.state_bits();
                step(&mut game, input);
                let after = game.world.entity(id).unwrap();
                let contact_view = game.presentation();
                assert_eq!(contact_view.tick, game.tick);
                assert_eq!(
                    contact_view.dispatcher.phase,
                    spelunky_core::dispatcher::Phase::Draw
                );
                let contact_actor = contact_view.world.entity(id).unwrap();
                let expected_weapon = if machete { "oSlash" } else { "oWhip" };
                let incoming: Vec<_> = contact_view
                    .world
                    .entities
                    .iter()
                    .filter(|weapon| {
                        weapon.alive
                            && weapon.engine_active
                            && weapon.object == expected_weapon
                            && weapon.event_intersects(contact_actor)
                    })
                    .collect();
                // The source Machete gate permits a hit on every selected
                // stunned contact, including a Slash in the first attack cycle.
                // Plain Whip remains an explicit second ordinary swing control.
                if (machete || second_swing) && !incoming.is_empty() {
                    assert_eq!(
                        incoming.len(),
                        1,
                        "exact source repeat attack selected in the collision frame"
                    );
                    assert_eq!(
                        incoming[0].text.get("type").map(String::as_str),
                        Some(if machete { "Machete" } else { "Whip" })
                    );
                    assert_eq!(incoming[0].value("damage"), if machete { 2.0 } else { 1.0 });
                    assert_eq!(
                        game.player.pos, player_before,
                        "ordinary source repeat contact retains attachment origin"
                    );
                    if machete {
                        if !second_swing {
                            assert!(
                                !input.attack && !input.attack_pressed,
                                "FIRST_CYCLE_MACHETE_REPEAT_HAS_NO_NEW_PRESS"
                            );
                            assert!(attack_timer_before > 0);
                            assert!(matches!(
                                player_sprite_before.as_str(),
                                "sAttackLeft" | "sDamselAttackL" | "sTunnelAttackL"
                            ));
                            assert!(
                                player_animation_before > 4.0,
                                "source first-cycle Slash producer remains eligible by attack art"
                            );
                        }
                        assert_eq!(
                            after.health,
                            before.health - 2,
                            "SOURCE_MACHETE_BYPASSES_STUNNED_GATE"
                        );
                        assert_eq!((after.state, after.timer), (98, 60));
                        assert_eq!(after.image_speed(), 0.5);
                        assert_eq!(
                            after.vel,
                            Vec2::new(
                                if incoming[0].pos.x < after.pos.x + 8.0 {
                                    2.0
                                } else {
                                    -2.0
                                },
                                -3.0
                            )
                        );
                        assert!(game.events.iter().any(|event| matches!(event, GameEvent::Sound { name, .. } if name == "sndHit")));
                    } else {
                        assert!(second_swing, "SECOND_ORDINARY_WHIP_SWING_CONTACT_REQUIRED");
                        assert_eq!(
                            after.health, first_health,
                            "SOURCE_STUNNED_PLAIN_WHIP_IGNORED"
                        );
                        assert!(!game.events.iter().any(|event| matches!(event, GameEvent::Sound { name, .. } if name == "sndHit")));
                        assert_eq!(after.state, 98);
                        assert!(after.timer > 0);
                    }
                    assert_eq!(after.value("bloodless"), 1.0);
                    assert_eq!(after.value("blood_left"), 4.0);
                    assert!(
                        !game
                            .world
                            .entities
                            .iter()
                            .any(|e| e.alive && e.object == "oBlood")
                    );
                    assert_eq!(
                        game.rng.state_bits(),
                        rng_before,
                        "default bloodless source repeat callback consumes no Blood RNG"
                    );
                    witnessed = true;
                    break;
                }
                assert_eq!(
                    after.health, first_health,
                    "no damage without precise selected source repeat contact"
                );
            }
            assert!(witnessed, "NATURAL_REPEAT_PROJECTILE_CONTACT_PREREQUISITE");
        }
    }
}
