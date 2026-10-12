//! SOURCE-ONLY packet: default King30HP/free acquisition fixtures are UNRUN.
//! King/Create/HP/state/RNG are never rewritten after the first actual Step.
use spelunky_core::*;
fn room(
    rear: bool,
    high: bool,
    weapon: Option<&str>,
    bloodless: bool,
    detritus: usize,
) -> (Game, u64, Option<u64>) {
    let mut game =
        Game::shell_with_algorithm(0, RulesPreset::Classic, RngAlgorithm::Gm8Lcg32V1).unwrap();
    game.world = World::new(30, 20, Area::Ice, 9);
    for x in 0..30 {
        game.world.set_tile(x, 10, "oHardBlock");
    }
    if high {
        for x in [3, 4] {
            game.world.set_tile(x, 9, "oHardBlock");
        }
    }
    game.player.pos = Vec2::new(
        if rear { 56.0 } else { 80.0 },
        if high { 136.0 } else { 152.0 },
    );
    game.player.previous_pos = game.player.pos;
    let item = weapon.map(|kind| {
        let id = game.spawn(kind, game.player.pos);
        let e = game.world.entity_mut(id).unwrap();
        e.data.insert("cost".into(), 0.0);
        e.data.insert("forSale".into(), 0.0);
        e.data.insert("for_sale".into(), 0.0);
        id
    });
    let id = game.spawn(
        "oYetiKing",
        Vec2::new(if rear { 16.0 } else { 80.0 }, 128.0),
    );
    assert_eq!(
        (
            game.world.entity(id).unwrap().health,
            game.world.entity(id).unwrap().state
        ),
        (30, 0)
    );
    // Explicit controlled INITIAL guard fields/cap population, not claims
    // of an ordinary producer for the bloodless flag. HP remains default30.
    if bloodless {
        game.world
            .entity_mut(id)
            .unwrap()
            .data
            .insert("bloodless".into(), 1.0);
    }
    for _ in 0..detritus {
        game.spawn("oBone", Vec2::new(240.0, 64.0));
    }
    (game, id, item)
}
fn blood(game: &Game) -> Vec<&Entity> {
    game.world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oBlood")
        .collect()
}
fn attack(
    rear: bool,
    weapon: Option<&str>,
    expected_attack: &str,
    bloodless: bool,
    detritus: usize,
) -> (Game, u64) {
    let (mut game, id, item) = room(rear, !rear, weapon, bloodless, detritus);
    let damaging = rear || weapon.is_some();
    let damage = if weapon.is_some() { 2 } else { 1 };
    let emits = damaging && !bloodless && detritus < 16;
    let mut hit = false;
    for tick in 0..18 {
        let before = game
            .world
            .entity(id)
            .expect("DEFAULT_KING_SURVIVES_FIRST_ATTACK")
            .clone();
        let mut oracle = game.rng.clone();
        let velocity = if emits {
            Some(Vec2::new(
                oracle.unit() * 4.0 - oracle.unit() * 4.0,
                -1.0 - oracle.unit() * 2.0,
            ))
        } else {
            None
        };
        let gravity = if emits {
            Some(f64::from(1 + oracle.range(6)) * 0.1)
        } else {
            None
        };
        let pickup = item.is_some() && tick == 0;
        let swing = if item.is_some() { tick == 2 } else { tick == 0 };
        game.step(&InputFrame {
            down: pickup,
            attack: pickup || swing,
            attack_pressed: pickup || swing,
            ..Default::default()
        });
        assert_eq!(
            game.player.health, 4,
            "NO_KING_BODY_CONTACT_HIDING_MELEE tick={tick}"
        );
        if pickup {
            assert_eq!(game.player.held, item, "REAL_FREE_WEAPON_PICKUP");
        }
        let actor = game.world.entity(id).unwrap();
        if actor.health != before.health
            || actor.value("hit_cooldown") > before.value("hit_cooldown")
        {
            // The Draw snapshot still contains the live final-frame attack;
            // source Animation End may retire Slash before step returns.
            let presented = game.presentation();
            let presented_target = presented.world.entity(id).expect("SOURCE_DRAW_TARGET");
            let actual = presented
                .world
                .entities
                .iter()
                .find(|e| e.alive && e.object == expected_attack)
                .expect("REAL_EXPECTED_ATTACK_FAMILY");
            assert!(
                actual.event_intersects(presented_target),
                "SOURCE_DRAW_PRE_ANIMATION_PRECISE_MELEE_PAIR"
            );
            assert_eq!(actor.health, 30 - if damaging { damage } else { 0 });
            assert_eq!(actor.value("hit_cooldown"), if rear { 0.0 } else { 10.0 });
            let particles = blood(&game);
            assert_eq!(
                particles.len(),
                usize::from(emits),
                "SOURCE_KING_PUNCTURE_SCRIPT_BLOOD"
            );
            if emits {
                let (w, h) = catalog::sprite_dimensions(&actor.sprite).unwrap();
                let sx = actor.data.get("image_xscale").copied().unwrap_or(1.0);
                let sy = actor.data.get("image_yscale").copied().unwrap_or(1.0);
                assert_eq!(
                    particles[0].pos,
                    Vec2::new(
                        actor.pos.x + f64::from(w) * sx / 2.0,
                        actor.pos.y + f64::from(h) * sy / 2.0
                    )
                );
                assert_eq!(particles[0].vel, velocity.unwrap());
                assert_eq!(particles[0].value("gravity"), gravity.unwrap());
                assert_eq!(&particles[0].alarms[..3], &[1, 1, 5]);
                assert_eq!(particles[0].age, 0);
            }
            assert_eq!(
                game.rng.state_bits(),
                oracle.state_bits(),
                "SOURCE_SCRIPT_ONLY_CREATE_DRAWS_OR_GUARD_ZERO"
            );
            assert!(
                game.events
                    .iter()
                    .any(|event| matches!(event,GameEvent::Sound{name,..}if name=="sndHit"))
            );
            hit = true;
            break;
        }
    }
    assert!(hit, "BOUNDED_DEFAULT_KING_MELEE_NOT_REACHED");
    (game, id)
}
#[test]
fn ordinary_front_puncture_slash_and_mattock_create_one_script_blood() {
    for (weapon, attack_name) in [("oMachete", "oSlash"), ("oMattock", "oMattockHit")] {
        attack(false, Some(weapon), attack_name, false, 0);
    }
}
#[test]
fn ordinary_front_plain_whip_bumps_without_damage_or_blood() {
    attack(false, None, "oWhip", false, 0);
}
#[test]
fn source_inherited_rear_whip_machete_and_mattock_keep_damage_and_blood() {
    for (weapon, attack_name) in [
        (None, "oWhipPre"),
        (Some("oMachete"), "oMachetePre"),
        (Some("oMattock"), "oMattockPre"),
    ] {
        attack(true, weapon, attack_name, false, 0);
    }
}
#[test]
fn initial_bloodless_and_sixteen_detritus_guard_script_without_suppressing_puncture() {
    for (bloodless, detritus) in [(true, 0), (false, 16)] {
        attack(false, Some("oMachete"), "oSlash", bloodless, detritus);
    }
}
#[test]
fn naturally_armed_front_cooldown_ignores_remaining_slash_frames() {
    let (mut game, id) = attack(false, Some("oMachete"), "oSlash", false, 0);
    let ids: Vec<_> = blood(&game).iter().map(|e| e.id).collect();
    game.step(&InputFrame::default());
    assert_eq!(game.world.entity(id).unwrap().health, 28);
    assert_eq!(game.world.entity(id).unwrap().value("hit_cooldown"), 9.0);
    assert_eq!(blood(&game).iter().map(|e| e.id).collect::<Vec<_>>(), ids);
    assert_eq!(game.player.health, 4);
}
