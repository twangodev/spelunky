//! Ordinary pickup, ledge fall, and walking contact. Configured initial
//! bloodless/cap controls are labeled separately. No writes after bootstrap.
use spelunky_core::*;
fn modes() -> impl Iterator<Item = (RulesPreset, RngAlgorithm)> {
    [RulesPreset::Classic, RulesPreset::Assisted]
        .into_iter()
        .flat_map(|r| {
            [RngAlgorithm::Gm8Lcg32V1, RngAlgorithm::LegacySplitMix64V1]
                .into_iter()
                .map(move |a| (r, a))
        })
}
fn scene(
    r: RulesPreset,
    a: RngAlgorithm,
    fall: bool,
    shoes: bool,
    roof: bool,
) -> (Game, u64, Option<u64>) {
    scene_at(r, a, fall, shoes, roof, 80.0)
}
fn scene_at(
    r: RulesPreset,
    a: RngAlgorithm,
    fall: bool,
    shoes: bool,
    roof: bool,
    boss_x: f64,
) -> (Game, u64, Option<u64>) {
    let mut g = Game::shell_with_algorithm(42, r, a).unwrap();
    g.world = World::new(30, 20, Area::Ice, 9);
    for x in 0..=16 {
        g.world.set_tile(x, 10, "oBlock");
    }
    if fall {
        for x in 0..=4 {
            g.world.set_tile(x, 7, "oBlock");
        }
    }
    if roof {
        g.world.set_tile(4, 8, "oBlock"); // actual low ceiling over initial player
        g.world.set_tile(6, 7, "oBlock"); // ceiling immediately above AlienBoss's head
    }
    g.player.pos = Vec2::new(72.0, if fall { 104.0 } else { 152.0 });
    g.player.previous_pos = g.player.pos;
    if roof {
        assert!(g.world.solid_at(g.player.pos.x, g.player.pos.y - 9.0));
    }
    let k = g.spawn("oAlienBoss", Vec2::new(boss_x, 128.0));
    let item = shoes.then(|| {
        let id = g.spawn(
            "oSpikeShoes",
            Vec2::new(g.player.pos.x, g.player.pos.y + 4.0),
        );
        let e = g.world.entity_mut(id).unwrap();
        e.data.insert("cost".into(), 0.0);
        e.data.insert("forSale".into(), 0.0);
        id
    });
    assert_eq!(
        (
            g.world.entity(k).unwrap().state,
            g.world.entity(k).unwrap().health
        ),
        (0, 10)
    );
    assert_eq!(g.world.entity(k).unwrap().value("bloodless"), 0.0);
    assert!(g.player.equipment.is_empty());
    (g, k, item)
}
fn pickup(g: &mut Game, item: Option<u64>) {
    g.step(&InputFrame {
        down: true,
        attack: item.is_some(),
        attack_pressed: item.is_some(),
        ..Default::default()
    });
    if let Some(id) = item {
        assert!(g.world.entity(id).is_none());
        assert!(g.player.has("SpikeShoes"));
        assert!(
            !g.player.has("spike_shoes"),
            "ordinary pickup supplies canonical name only"
        );
    } else {
        assert!(g.player.equipment.is_empty());
    }
    assert_eq!(g.player.held, None);
}
fn rand(r: &mut Rng, width: u64) -> u32 {
    let a = r.algorithm();
    let raw = r.next_u64();
    match a {
        RngAlgorithm::Gm8Lcg32V1 => ((raw * width) >> 32) as u32,
        RngAlgorithm::LegacySplitMix64V1 => (raw % width) as u32,
    }
}
fn unit(r: &mut Rng) -> f64 {
    let a = r.algorithm();
    let raw = r.next_u64();
    match a {
        RngAlgorithm::Gm8Lcg32V1 => raw as f64 / 4_294_967_296.0,
        RngAlgorithm::LegacySplitMix64V1 => (raw >> 11) as f64 / 9_007_199_254_740_992.0,
    }
}
fn literal_blood(r: &mut Rng) -> (Vec2, f64) {
    let v = Vec2::new(unit(r) * 4.0 - unit(r) * 4.0, -1.0 - unit(r) * 2.0);
    (v, f64::from(1 + rand(r, 6)) * 0.1)
}
fn active_detritus(g: &Game) -> usize {
    g.world
        .entities
        .iter()
        .filter(|e| e.alive && e.engine_active && catalog::is_a(&e.object, "oDetritus"))
        .count()
}
fn stomp(mut g: Game, k: u64, item: Option<u64>) -> (Game, u64, Rng) {
    let cap = active_detritus(&g);
    pickup(&mut g, item);
    let mut falling = false;
    for _ in 0..40 {
        if cap > 0 {
            assert_eq!(
                active_detritus(&g),
                cap,
                "configured cap remains active/live before every actual step"
            );
        }
        let before = g.player.clone();
        let serial = g.world.creation_serial;
        let expected = g.rng.clone();
        falling |= before.vel.y > 0.0;
        g.step(&InputFrame {
            right: true,
            ..Default::default()
        });
        let boss = g.world.entity(k).unwrap();
        assert_eq!(g.player.health, 4);
        if boss.health != 10 {
            assert!(falling);
            assert_eq!(before.state, PlayerState::Falling);
            assert!(before.vel.y < 9.0);
            if cap > 0 {
                assert_eq!(
                    active_detritus(&g),
                    32,
                    "direct collision births reach but never exceed live cap32"
                );
            }
            assert!((1.0..16.0).contains(&before.data.get("fall_timer").copied().unwrap_or(0.0)));
            assert_eq!(g.player.vel.y, -6.0 - 0.2 * (before.vel.y + 1.0));
            assert_eq!(g.player.data.get("fall_timer").copied(), Some(0.0));
            assert_eq!(g.player.stun, 0);
            assert_ne!(g.player.state, PlayerState::Stunned);
            assert_eq!(boss.value("counts_as_kill"), 1.0);
            assert!(
                g.events
                    .iter()
                    .any(|e| matches!(e,GameEvent::Sound{name,..}if name=="sndHit"))
            );
            return (g, serial, expected);
        }
    }
    panic!("ordinary ledge fall must stomp source-default AlienBoss")
}
fn blood(g: &Game, k: u64, serial: u64, mut expected: Rng, shoes: bool, surviving: usize) {
    let mut drops: Vec<_> = g
        .world
        .entities
        .iter()
        .filter(|e| e.alive && e.creation_serial > serial && e.object == "oBlood")
        .collect();
    drops.sort_by_key(|e| e.creation_serial);
    assert_eq!(
        drops.len(),
        surviving,
        "source direct Blood uses cap32, independently of script cap16"
    );
    let boss = g.world.entity(k).unwrap();
    let mut origins = Vec::new();
    if shoes {
        origins.push(Vec2::new(g.player.pos.x, g.player.pos.y + 8.0));
    }
    origins.push(Vec2::new(boss.pos.x + 16.0, boss.pos.y + 8.0));
    for (i, origin) in origins.into_iter().enumerate() {
        let (v, gravity) = literal_blood(&mut expected);
        if let Some(drop) = drops.get(i) {
            assert_eq!(drop.previous_pos, origin);
            assert_eq!(drop.vel, v);
            assert_eq!(drop.value("gravity"), gravity);
            assert_eq!(&drop.alarms[..3], &[1, 1, 5]);
            assert_eq!(drop.value("bounce"), 0.0);
        }
    }
    assert_eq!(
        g.rng.clone().next_u64(),
        expected.next_u64(),
        "every direct Create finishes its tape, including capped births"
    );
}
#[test]
fn ordinary_spike_shoes_pickup_and_fall_triples_boss_damage_and_orders_both_blood_creates() {
    for (r, a) in modes() {
        let (g, k, item) = scene(r, a, true, true, false);
        let (g, s, rng) = stomp(g, k, item);
        assert_eq!(
            g.world.entity(k).unwrap().health,
            7,
            "canonical SpikeShoes triples (floor(fallTimer/16)+1)"
        );
        blood(&g, k, s, rng, true, 2);
    }
}
#[test]
fn ordinary_barefoot_fall_keeps_single_damage_but_still_creates_boss_origin_blood() {
    for (r, a) in modes() {
        let (g, k, item) = scene(r, a, true, false, false);
        let (g, s, rng) = stomp(g, k, item);
        assert_eq!(g.world.entity(k).unwrap().health, 9);
        blood(&g, k, s, rng, false, 1);
    }
}
#[test]
fn configured_initial_bloodless_guards_only_player_blood_on_ordinary_spiked_stomp() {
    for (r, a) in modes() {
        let (mut g, k, item) = scene(r, a, true, true, false);
        g.world
            .entity_mut(k)
            .unwrap()
            .data
            .insert("bloodless".into(), 1.0);
        let (g, s, rng) = stomp(g, k, item);
        assert_eq!(g.world.entity(k).unwrap().health, 7);
        blood(&g, k, s, rng, false, 1);
    }
}
#[test]
fn configured_initial_detritus_cap_preserves_direct_stomp_order_and_both_random_tapes() {
    for (r, a) in modes() {
        for cap in [31, 32] {
            let (mut g, k, item) = scene(r, a, true, true, false);
            for _ in 0..cap {
                // Source-default Bone fills the same cap without Blood terminal-speed retirement.
                g.spawn("oBone", Vec2::new(200.0, 64.0));
            }
            let (g, s, rng) = stomp(g, k, item);
            assert_eq!(g.world.entity(k).unwrap().health, 7);
            blood(&g, k, s, rng, true, 32 - cap);
        }
    }
}
#[test]
fn ordinary_ground_contact_hurts_once_with_or_without_low_roof_and_has_no_stomp_blood() {
    for (r, a) in modes() {
        for roof in [false, true] {
            let (mut g, k, item) = scene(r, a, false, true, roof);
            pickup(&mut g, item);
            assert_eq!(g.player.state, PlayerState::Ducking);
            let mut contacted = false;
            for _ in 0..40 {
                let state = g.player.state;
                let serial = g.world.creation_serial;
                let mut expected = g.rng.clone();
                g.step(&InputFrame {
                    right: true,
                    ..Default::default()
                });
                if g.player.health == 4 {
                    continue;
                }
                assert!(matches!(
                    state,
                    PlayerState::Standing | PlayerState::Running
                ));
                let actor = g.world.entity(k).unwrap();
                assert_eq!(
                    g.world.solid_at(actor.pos.x + 16.0, actor.pos.y - 4.0),
                    roof
                );
                assert_eq!(g.player.health, 3);
                assert_eq!(g.player.invincible, 30);
                assert_eq!(g.player.data.get("blink").copied(), Some(30.0));
                assert_eq!(g.player.stun, 0);
                assert_ne!(g.player.state, PlayerState::Stunned);
                assert_eq!(g.player.vel, Vec2::new(-6.0, 0.0));
                assert_eq!(g.world.entity(k).unwrap().health, 10);
                assert!(
                    !g.world
                        .entities
                        .iter()
                        .any(|e| e.creation_serial > serial && e.object == "oBlood")
                );
                assert_eq!(g.rng.clone().next_u64(), expected.next_u64());
                assert!(
                    g.events
                        .iter()
                        .any(|e| matches!(e, GameEvent::Damage { amount: 1 }))
                );
                assert!(
                    g.events
                        .iter()
                        .any(|e| matches!(e,GameEvent::Sound{name,..}if name=="sndHurt"))
                );
                for _ in 0..5 {
                    g.step(&InputFrame {
                        right: true,
                        ..Default::default()
                    });
                    assert_eq!(g.player.health, 3);
                }
                contacted = true;
                break;
            }
            assert!(
                contacted,
                "ordinary walking reaches AlienBoss's own Character callback"
            );
        }
    }
}
#[test]
fn authored_custom_room_contact_still_subtracts_one_life_and_keeps_hurt_effects() {
    for (r, a) in modes() {
        let (mut g, k, item) = scene(r, a, false, true, true);
        g.flags.insert("custom_level".into());
        assert!(!g.is_real_level());
        pickup(&mut g, item);
        let mut contacted = false;
        for _ in 0..40 {
            g.step(&InputFrame {
                right: true,
                ..Default::default()
            });
            if g.player.invincible == 0 {
                continue;
            }
            assert_eq!(g.player.invincible, 30);
            assert_eq!(g.player.health, 3);
            assert_eq!(g.world.entity(k).unwrap().health, 10);
            assert_eq!(g.player.vel, Vec2::new(-6.0, 0.0));
            assert!(
                g.events
                    .iter()
                    .any(|e| matches!(e,GameEvent::Sound{name,..}if name=="sndHurt"))
            );
            assert!(
                g.events
                    .iter()
                    .any(|e| matches!(e, GameEvent::Damage { amount: 1 }))
            );
            contacted = true;
            break;
        }
        assert!(contacted);
    }
}

#[test]
fn ordinary_ascending_jump_with_zero_fall_timer_still_deals_floor_plus_one_damage() {
    for (rules, algorithm) in modes() {
        for shoes in [false, true] {
            let (mut game, boss, item) = scene_at(rules, algorithm, false, shoes, false, 88.0);
            pickup(&mut game, item);
            let mut hit = false;
            for tick in 0..20 {
                let before = game.player.clone();
                let serial = game.world.creation_serial;
                let expected = game.rng.clone();
                game.step(&InputFrame {
                    right: true,
                    jump: tick <= 5,
                    jump_pressed: tick == 1,
                    ..Default::default()
                });
                let actor = game.world.entity(boss).unwrap();
                assert_eq!(game.player.health, 4);
                if actor.health == 10 {
                    continue;
                }
                assert_eq!(tick, 5);
                assert_eq!(before.state, PlayerState::Jumping);
                assert_eq!(before.data.get("fall_timer").copied(), Some(0.0));
                assert!(before.vel.y < 0.0);
                assert_eq!(
                    actor.health,
                    if shoes { 7 } else { 9 },
                    "source floor(0/16)+1 remains one, unlike King's ceil"
                );
                assert_eq!(game.player.vel.y, -5.4);
                assert_eq!(game.player.data.get("fall_timer").copied(), Some(0.0));
                blood(
                    &game,
                    boss,
                    serial,
                    expected,
                    shoes,
                    if shoes { 2 } else { 1 },
                );
                hit = true;
                break;
            }
            assert!(
                hit,
                "ordinary jump reaches the source-owned ascending stomp"
            );
        }
    }
}
