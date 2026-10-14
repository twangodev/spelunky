//! Source-derived Spike HP-state contract, UNRUN. No visible gameplay consequence
//! beyond public/serialized Actor.health retention is asserted.
//! Initial geometry/free prices/player pose only; no mid-route actor writes.
use spelunky_core::*;
fn scene(rules: RulesPreset, algorithm: RngAlgorithm) -> (Game, u64) {
    let mut g = Game::shell_with_algorithm(7, rules, algorithm).unwrap();
    g.world = World::new(30, 20, Area::Mines, 1);
    for col in 0..g.world.width {
        g.world.set_tile(col, 10, "oBlock");
    }
    for col in 6..=9 {
        g.world.set_tile(col, 9, "oSpikes");
    }
    g.player.pos = Vec2::new(168.0, 152.0);
    g.player.previous_pos = g.player.pos;
    g.player.state = PlayerState::Standing;
    g.player.grounded = true;
    g.player.facing = 1;
    g.set_simulation_view_extent(480.0, 320.0).unwrap();
    let damsel = g.spawn("oDamsel", Vec2::new(180.0, 152.0));
    g.world
        .entity_mut(damsel)
        .unwrap()
        .data
        .insert("cost".into(), 0.0);
    assert_eq!(g.world.entity(damsel).unwrap().health, 4);
    assert_eq!(g.world.entity(damsel).unwrap().state, 0);
    (g, damsel)
}
fn modes() -> impl Iterator<Item = (RulesPreset, RngAlgorithm)> {
    [RulesPreset::Classic, RulesPreset::Assisted]
        .into_iter()
        .flat_map(|r| {
            [RngAlgorithm::Gm8Lcg32V1, RngAlgorithm::LegacySplitMix64V1]
                .into_iter()
                .map(move |a| (r, a))
        })
}
fn unharmed(g: &Game) {
    assert_eq!(g.player.health, 4, "FIXTURE_NO_DAMAGE");
    assert_eq!(g.player.stun, 0, "FIXTURE_NO_STUN");
    assert!(!g.player.is_stunned(), "FIXTURE_NO_STUN_LATCH");
    assert!(
        !g.events
            .iter()
            .any(|event| matches!(event, GameEvent::Damage { .. })),
        "FIXTURE_NO_DAMAGE_EVENT"
    );
}
fn safe_step(g: &mut Game, input: &InputFrame) {
    g.step(input);
    unharmed(g);
}
fn spike_stains(g: &Game) -> usize {
    g.world
        .entities
        .iter()
        .filter(|e| {
            e.alive
                && e.object == "oTerrainMetadata"
                && e.text
                    .get("terrain_sprite")
                    .is_some_and(|s| s == "sSpikesBlood")
        })
        .count()
}
fn released_throw_into_spikes(g: &mut Game, id: u64, expected_hp: i32) {
    assert_eq!(
        g.player.held,
        Some(id),
        "FIXTURE_REAL_CORPSE_OR_HEALTHY_PICKUP"
    );
    let kills_before = g.lifetime_stats.damsels_killed_total;
    // Ordinary turn/approach only. Stop before the hero reaches the bed, then
    // let neutral inputs settle walking momentum before the heavy throw.
    let mut edge = false;
    for _ in 0..16 {
        safe_step(
            g,
            &InputFrame {
                left: true,
                ..Default::default()
            },
        );
        assert!(g.player.pos.x >= 160.0, "FIXTURE_HERO_OUTSIDE_SPIKES");
        assert_eq!(g.player.held, Some(id));
        assert_eq!(g.world.entity(id).unwrap().health, expected_hp);
        if g.player.pos.x <= 172.0 {
            edge = true;
            break;
        }
    }
    assert!(edge, "FIXTURE_ORDINARY_APPROACH_MUST_REACH_THROW_EDGE");
    for _ in 0..8 {
        safe_step(g, &InputFrame::default());
        assert!(g.player.pos.x >= 160.0, "FIXTURE_HERO_OUTSIDE_SPIKES");
        assert_eq!(g.player.held, Some(id));
        assert_eq!(g.world.entity(id).unwrap().health, expected_hp);
    }
    assert_eq!(g.player.facing, -1);
    assert!(g.player.grounded);
    assert_eq!(
        g.player.vel,
        Vec2::default(),
        "FIXTURE_STATIONARY_HEAVY_THROW"
    );
    assert!(!g.player.has("Mitt"));
    assert_eq!(spike_stains(g), 0);
    assert_eq!(g.lifetime_stats.damsels_killed_total, kills_before);
    safe_step(
        g,
        &InputFrame {
            attack_pressed: true,
            ..Default::default()
        },
    );
    assert_eq!(g.player.held, None, "FIXTURE_REAL_THROW_RELEASE");
    assert!(
        g.events
            .iter()
            .any(|event| matches!(event,GameEvent::Sound{name,..} if name=="sndThrow"))
    );
    assert!(
        g.events
            .iter()
            .any(|event| matches!(event,GameEvent::Sound{name,..} if name=="sndDamsel"))
    );
    let e = g.world.entity(id).unwrap();
    assert!(!e.held);
    assert_eq!(e.state, 2, "FIXTURE_REAL_DAMSEL_THROWN_STATE");
    assert_eq!(e.timer, 120);
    assert_eq!(e.alarms[2], 10);
    assert_eq!(e.health, expected_hp);
    assert!(
        (e.vel.x + 4.0).abs() < 1e-9,
        "FIXTURE_SOURCE_HEAVY_LEFT_VELOCITY"
    );
    assert!(
        (e.vel.y + 1.4).abs() < 1e-9,
        "FIXTURE_THROW_MINUS_TWO_THEN_PARENT_GRAVITY"
    );
    let mut reached = false;
    for _ in 0..20 {
        let before = g.world.entity(id).unwrap().clone();
        assert!(!before.held);
        assert_eq!(
            before.health, expected_hp,
            "FIXTURE_RAW_HP_BEFORE_SPIKE_TARGET"
        );
        let next_birth = g.world.next_id;
        safe_step(g, &InputFrame::default());
        assert!(g.player.pos.x >= 160.0);
        let e = g
            .world
            .entity(id)
            .expect("released Damsel remains for Spike callback");
        if spike_stains(g) == 0 {
            assert_eq!(e.health, expected_hp);
            continue;
        }
        // oItem moves first and adds .6 only without bottom contact. Its own
        // child Spike branch then overwrites vy with .2; the incoming parent
        // result must independently exceed2, not be fabricated after contact.
        assert_eq!(before.value("gravity"), 0.6);
        assert!(
            before.vel.y + before.value("gravity") > 2.0,
            "FIXTURE_PARENT_VELOCITY_EXCEEDS_TWO"
        );
        assert_eq!(e.value("colBot"), 0.0, "FIXTURE_NO_PARENT_FLOOR_BOUNCE");
        let probe = Vec2::new(e.pos.x, e.pos.y + 6.0);
        assert!(
            (6..=9).any(|col| g
                .world
                .tile_collision_mask(col, 9)
                .contains(probe, CollisionPrecision::BoundingBox)),
            "FIXTURE_SOURCE_SPIKE_POINT_BBOX"
        );
        assert!(!e.held);
        assert_eq!(e.state, 99);
        assert_eq!(e.value("dead"), 1.0);
        assert_eq!(e.value("gravity"), 0.0);
        assert_eq!(e.vel, Vec2::new(0.0, 0.2));
        assert_eq!(e.sprite, "sDamselDieL");
        let expected_after = if expected_hp > 0 { 0 } else { expected_hp };
        assert_eq!(
            e.health, expected_after,
            "DAMSEL_SPIKES_ZEROES_ONLY_POSITIVE_SOURCE_HP"
        );
        assert_eq!(
            g.lifetime_stats.damsels_killed_total,
            kills_before + u64::from(expected_hp > 0),
            "DAMSEL_SPIKES_POSITIVE_ONLY_DEATH_CREDIT"
        );
        let blood = g
            .world
            .entities
            .iter()
            .filter(|e| e.id >= next_birth && e.object == "oBlood")
            .count();
        assert_eq!(
            blood,
            if expected_hp > 0 { 3 } else { 0 },
            "DAMSEL_SPIKES_POSITIVE_ONLY_BLOOD"
        );
        reached = true;
        break;
    }
    assert!(reached, "FIXTURE_RELEASED_FALL_MUST_REACH_STAINED_SPIKES");
}
#[test]
fn real_machete_negative_hp_then_released_spikes_preserves_nonpositive_source_hp() {
    for (rules, algorithm) in modes() {
        let (mut g, damsel) = scene(rules, algorithm);
        let blade = g.spawn("oMachete", g.player.pos);
        g.world
            .entity_mut(blade)
            .unwrap()
            .data
            .insert("cost".into(), 0.0);
        safe_step(
            &mut g,
            &InputFrame {
                down: true,
                attack_pressed: true,
                ..Default::default()
            },
        );
        assert_eq!(g.player.held, Some(blade), "real Machete pickup");
        safe_step(&mut g, &InputFrame::default());
        // Fixed ordinary attacks, not a seed/outcome search. Each reached
        // Machete collision subtracts native damage2 with hit cooldown10.
        for _ in 0..3 {
            let hp = g.world.entity(damsel).unwrap().health;
            safe_step(
                &mut g,
                &InputFrame {
                    attack_pressed: true,
                    ..Default::default()
                },
            );
            for _ in 0..20 {
                safe_step(&mut g, &InputFrame::default());
            }
            let e = g
                .world
                .entity(damsel)
                .expect("Machete does not retire Damsel");
            assert_eq!(g.player.health, 4);
            assert!(e.health <= hp - 2, "each actual attack must hit");
            assert_eq!((hp - e.health) % 2, 0, "only native Machete damage2");
            assert!(e.pos.x >= 160.0, "dry initial target");
        }
        assert!(
            g.world.entity(damsel).unwrap().health < 0,
            "ordinary producer must reach negative HP"
        );
        assert_eq!(g.player.attack_timer, 0);
        // Source SlashRight frame2 reaches x183..186 at Damsel body height;
        // the target origin180 fits that reach. Turn left before dropping the
        // blade, then approach the target using input so it beats the blade
        // in the source nearest-item query. No hand/pose fields are rewritten.
        for _ in 0..2 {
            safe_step(
                &mut g,
                &InputFrame {
                    right: true,
                    ..Default::default()
                },
            );
        }
        safe_step(
            &mut g,
            &InputFrame {
                left: true,
                ..Default::default()
            },
        );
        assert_eq!(g.player.facing, -1);
        assert_eq!(g.player.health, 4);
        safe_step(
            &mut g,
            &InputFrame {
                down: true,
                attack_pressed: true,
                ..Default::default()
            },
        );
        assert_eq!(g.player.held, None, "real Down weapon drop");
        for _ in 0..8 {
            safe_step(&mut g, &InputFrame::default());
            assert_eq!(g.player.health, 4);
        }
        let mut near = false;
        for _ in 0..12 {
            if g.player.pos.x >= 176.0 {
                near = true;
                break;
            }
            safe_step(
                &mut g,
                &InputFrame {
                    right: true,
                    ..Default::default()
                },
            );
            assert_eq!(g.player.health, 4);
        }
        assert!(near, "real walking must reach a nearer Damsel pickup pose");
        assert!(g.player.pos.x <= 184.0);
        safe_step(
            &mut g,
            &InputFrame {
                down: true,
                attack_pressed: true,
                ..Default::default()
            },
        );
        assert_eq!(g.player.held, Some(damsel), "real negative Damsel pickup");
        safe_step(&mut g, &InputFrame::default());
        let hp = g.world.entity(damsel).unwrap().health;
        assert!(hp < 0);
        released_throw_into_spikes(&mut g, damsel, hp);
    }
}
#[test]
fn real_positive_damsel_pickup_then_released_spikes_still_zeroes_hp_control() {
    for (rules, algorithm) in modes() {
        let (mut g, damsel) = scene(rules, algorithm);
        safe_step(
            &mut g,
            &InputFrame {
                down: true,
                attack_pressed: true,
                ..Default::default()
            },
        );
        assert_eq!(g.player.held, Some(damsel));
        safe_step(&mut g, &InputFrame::default());
        assert_eq!(g.world.entity(damsel).unwrap().health, 4);
        released_throw_into_spikes(&mut g, damsel, 4);
    }
}
