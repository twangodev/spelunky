//! Source-bound public API fixtures. Ordinary authored routes are separate from
//! configured callback controls. Prepared only: no runtime result is claimed.
use spelunky_core::hub::{self, HubKind};
use spelunky_core::{
    Entity, Game, GameEvent, InputFrame, PlayerState, RulesPreset, RunState, Vec2,
};

fn loaded(seed: u32, rules: RulesPreset) -> Game {
    let mut g = Game::shell(seed, rules);
    g.load_hub(HubKind::Moon);
    g
}
fn neutral(g: &mut Game, n: usize) {
    for _ in 0..n {
        g.step(&InputFrame::default());
    }
}
fn targets(g: &Game) -> Vec<Entity> {
    g.world
        .entities
        .iter()
        .filter(|e| e.alive && e.engine_active && e.object == "oBatTarget")
        .cloned()
        .collect()
}
fn data(g: &Game, key: &str) -> f64 {
    g.data.get(key).copied().unwrap_or(0.0)
}
fn own_step(mut e: Entity, timer: f64) -> Entity {
    // Literal complete oBatTarget Step. This oracle does not invoke port code.
    if e.value("dir") == 0.0 {
        e.pos.y -= e.value("difficulty");
        if e.pos.y <= 64.0 {
            e.data.insert("dir".into(), 1.0);
        }
    } else if e.value("dir") == 1.0 {
        e.pos.y += e.value("difficulty");
        if e.pos.y >= 160.0 {
            e.data.insert("dir".into(), 0.0);
        }
    }
    if timer <= 20.0 && e.pos.y > 64.0 {
        e.pos.x = e.value("xMid") - (e.value("xDiff").to_radians().sin() * 32.0).abs();
        e.data.insert("xDiff".into(), e.value("xDiff") + 0.01);
    }
    e
}
fn real_spike_death(seed: u32, rules: RulesPreset) -> Game {
    let mut g = loaded(seed, rules);
    neutral(&mut g, 100);
    assert_eq!(g.state, RunState::Playing);
    assert_eq!(g.player.health, 8.0);
    assert_eq!(targets(&g).len(), 3, "real Alarm3 target producer");
    // All actor state remains source-produced. Walk across the authored left
    // steps/SpringTrap, then release Right above the actual spike field.
    let mut reached = false;
    for _ in 0..80 {
        if g.player.pos.x >= 112.0 || g.state == RunState::Dead {
            reached = true;
            break;
        }
        g.step(&InputFrame {
            right: true,
            ..Default::default()
        });
    }
    assert!(
        reached,
        "ordinary Right must cross the source left platform"
    );
    let mut died = false;
    for _ in 0..240 {
        if g.state == RunState::Dead {
            died = true;
            break;
        }
        g.step(&InputFrame::default());
    }
    assert!(died, "ordinary authored spike fall must latch real death");
    assert!(
        g.events.contains(&GameEvent::Died),
        "the fatal frame is retained"
    );
    assert_eq!(g.player.health, 0.0);
    assert_eq!(g.player.state, PlayerState::Dead);
    assert_eq!(
        g.player.data.get("my_gravity"),
        Some(&0.0),
        "source spike fatal write"
    );
    assert!(
        targets(&g).len() >= 1,
        "a source-born active target survives death"
    );
    assert_eq!(data(&g, "timer"), -1.0);
    assert_eq!(data(&g, "moon_death_recorded"), 1.0);
    assert!(g.flags.contains("hub_challenge_finished"));
    g
}
#[test]
fn authored_moon_real_spike_death_keeps_whole_step_target_motion_running() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for seed in [7, 13] {
            let mut g = real_spike_death(seed, rules);
            for _ in 0..3 {
                let before = targets(&g);
                assert!(!before.is_empty());
                let timer = data(&g, "timer");
                let expected: Vec<_> = before.iter().cloned().map(|e| own_step(e, timer)).collect();
                let tick = g.tick;
                g.step(&InputFrame::default());
                assert_eq!(
                    g.tick,
                    tick + 1,
                    "whole dispatcher still advances while dead"
                );
                assert_eq!(g.state, RunState::Dead);
                for (old, want) in before.iter().zip(expected) {
                    let actual = g
                        .world
                        .entity(old.id)
                        .expect("surviving source target is not timeout-retired");
                    assert_eq!(
                        actual.pos.y, want.pos.y,
                        "fatal target vertical own-Step continues seed{seed}/{rules:?}/id{}",
                        old.id
                    );
                    assert!(
                        (actual.pos.x - want.pos.x).abs() < 1e-10,
                        "fatal target degree-based own-Step continues seed{seed}/{rules:?}/id{}: actual={}, expected={}",
                        old.id,
                        actual.pos.x,
                        want.pos.x,
                    );
                    assert_eq!(actual.value("dir"), want.value("dir"));
                    assert_eq!(actual.value("xDiff"), want.value("xDiff"));
                    assert_ne!(actual.pos.y, old.pos.y, "nonzero source-default difficulty");
                }
            }
            assert!(
                g.world
                    .entities
                    .iter()
                    .any(|e| e.alive && e.object == "oEntrance")
            );
            assert!(
                !g.world
                    .entities
                    .iter()
                    .any(|e| e.alive && e.object == "oXScores")
            );
        }
    }
}
#[test]
fn authored_moon_full_natural_timeout_destroys_targets_and_replaces_entrance() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for seed in [7, 13] {
            let mut g = loaded(seed, rules);
            let mut reached = false;
            for _ in 0..2200 {
                assert_eq!(
                    g.state,
                    RunState::Playing,
                    "neutral source left platform stays safe"
                );
                if data(&g, "timer") == 1.0 && data(&g, "hub_alarm11") == 1.0 {
                    reached = true;
                    break;
                }
                g.step(&InputFrame::default());
            }
            assert!(
                reached,
                "unmodified sixty-second timer must expire naturally"
            );
            let before = targets(&g);
            assert_eq!(before.len(), 3);
            let doors: Vec<_> = g
                .world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == "oEntrance")
                .cloned()
                .collect();
            assert!(!doors.is_empty());
            let first_birth = g.world.next_id;
            g.step(&InputFrame::default());
            assert_eq!(g.state, RunState::Playing);
            assert_eq!(data(&g, "timer"), -1.0);
            assert!(
                targets(&g).is_empty(),
                "timeout explicitly retires every target"
            );
            for e in &before {
                assert!(g.world.entity(e.id).is_none());
            }
            for e in &doors {
                assert!(g.world.entity(e.id).is_none());
            }
            assert_eq!(
                g.world.next_id,
                first_birth + 2 * before.len() as u64 + doors.len() as u64
            );
            let poofs: Vec<_> = g
                .world
                .entities
                .iter()
                .filter(|e| e.id >= first_birth && e.object == "oPoof")
                .collect();
            assert_eq!(poofs.len(), 6);
            for (target, pair) in before.iter().zip(poofs.chunks_exact(2)) {
                for (p, y) in pair.iter().zip([-1.0, 1.0]) {
                    // Alarm11 birth precedes Poof691 own Step this frame.
                    assert_eq!(p.previous_pos, Vec2::new(target.pos.x + 8.0, target.pos.y));
                    assert_eq!(p.pos, Vec2::new(target.pos.x + 8.0, target.pos.y + y));
                    assert_eq!(p.vel, Vec2::new(0.0, y));
                }
            }
            let scores: Vec<_> = g
                .world
                .entities
                .iter()
                .filter(|e| e.id >= first_birth && e.object == "oXScores")
                .collect();
            assert_eq!(scores.len(), doors.len());
            for (e, old) in scores.iter().zip(&doors) {
                assert_eq!(e.pos, old.pos);
            }
            assert_eq!(data(&g, "mini2"), 0.0);
            neutral(&mut g, 35);
            assert!(
                targets(&g).is_empty(),
                "restored callbacks do not revive timeout targets"
            );
        }
    }
}

// Configured source-callback controls: direct public hub adapter isolates effect
// sequencing/Create/RNG from its existing sweep and event-phase approximations.
// These fixtures deliberately assign collision geometry/flags; they are not
// natural input witnesses and never certify original native collision selection.
fn configured(dead: bool) -> (Game, u64) {
    let mut g = loaded(7, RulesPreset::Classic);
    neutral(&mut g, 100);
    let chosen = targets(&g)[0].id;
    for e in &mut g.world.entities {
        if e.object == "oBatTarget" && e.id != chosen {
            e.alive = false;
        }
    }
    let e = g.world.entity_mut(chosen).unwrap();
    e.pos = Vec2::new(160.0, 100.0);
    e.previous_pos = e.pos;
    e.data.insert("dir".into(), 1.0);
    e.data.insert("xDiff".into(), 0.0);
    if dead {
        g.player.health = 0.0;
        g.player.state = PlayerState::Dead;
        g.state = RunState::Dead;
        g.data.insert("timer".into(), -1.0);
        g.data.insert("moon_death_recorded".into(), 1.0);
        g.flags.insert("hub_challenge_finished".into());
    }
    g.events.clear();
    (g, chosen)
}
fn sound_names(g: &Game) -> Vec<&str> {
    g.events
        .iter()
        .filter_map(|e| {
            if let GameEvent::Sound { name, .. } = e {
                Some(name.as_str())
            } else {
                None
            }
        })
        .collect()
}
fn blood_oracle(g: &Game, count: usize) -> (spelunky_core::Rng, Vec<(Vec2, f64)>) {
    let mut rng = g.rng.clone();
    let mut births = Vec::new();
    for _ in 0..count {
        let vx = rng.unit() * 4.0 - rng.unit() * 4.0;
        let vy = -1.0 - rng.unit() * 2.0;
        let grav = f64::from(1 + rng.range(6)) * 0.1;
        births.push((Vec2::new(vx, vy), grav));
    }
    (rng, births)
}
#[test]
fn configured_finished_moon_arrow_restores_credit_births_and_immediate_blood_rng() {
    for (cap, bloodless) in [(0, false), (15, false), (16, false), (0, true)] {
        let (mut g, target) = configured(true);
        g.world
            .entity_mut(target)
            .unwrap()
            .data
            .insert("bloodless".into(), f64::from(bloodless));
        for _ in 0..cap {
            g.spawn("oBone", Vec2::new(40.0, 40.0));
        }
        let arrow = g.spawn("oArrow", Vec2::new(168.0, 106.0));
        g.world.entity_mut(arrow).unwrap().held = true;
        g.player.held = Some(arrow);
        let first = g.world.next_id;
        let count = if bloodless {
            0
        } else {
            3.min(16usize.saturating_sub(cap))
        };
        let (mut rng, births) = blood_oracle(&g, count);
        hub::tick(&mut g, &InputFrame::default());
        assert!(g.world.entity(target).is_none());
        assert!(g.world.entity(arrow).is_none());
        assert!(
            g.player.held.is_none(),
            "inherited held Arrow Destroy clears hand synchronously"
        );
        assert_eq!(data(&g, "baskets"), 1.0);
        assert_eq!(sound_names(&g), ["sndHit", "sndCoin"]);
        let replacement = g.world.entity(first).unwrap();
        assert_eq!(replacement.object, "oBatTarget");
        assert_eq!(replacement.pos, Vec2::new(160.0, -64.0));
        assert_eq!(replacement.value("difficulty"), 3.0);
        assert_eq!(replacement.value("bloodless"), 0.0);
        assert_eq!(replacement.value("dir"), 1.0);
        assert_eq!(replacement.value("xDiff"), 0.0);
        assert_eq!(replacement.sprite, "sUFO");
        for (index, (velocity, gravity)) in births.iter().enumerate() {
            let blood = g.world.entity(first + 1 + index as u64).unwrap();
            assert_eq!(blood.object, "oBlood");
            assert_eq!(blood.pos, Vec2::new(168.0, 109.0));
            assert_eq!(blood.vel, *velocity);
            assert_eq!(blood.value("gravity"), *gravity);
            assert_eq!(&blood.alarms[..3], &[1, 1, 5]);
        }
        assert_eq!(g.world.next_id, first + 1 + count as u64);
        assert_eq!(
            g.rng.clone().next_u64(),
            rng.next_u64(),
            "immediate source Create tape cap{cap}/bloodless{bloodless}"
        );
    }
}
#[test]
fn configured_finished_moon_ufo_arrow_creates_crash_after_replacement() {
    let (mut g, target) = configured(true);
    g.world.entity_mut(target).unwrap().set_sprite("sUFO");
    let arrow = g.spawn("oArrow", Vec2::new(168.0, 108.0));
    let first = g.world.next_id;
    let mut rng = g.rng.clone();
    hub::tick(&mut g, &InputFrame::default());
    assert!(g.world.entity(target).is_none());
    assert!(g.world.entity(arrow).is_none());
    assert_eq!(data(&g, "baskets"), 1.0);
    assert_eq!(sound_names(&g), ["sndHit", "sndCoin"]);
    assert_eq!(g.world.entity(first).unwrap().object, "oBatTarget");
    let crash = g.world.entity(first + 1).unwrap();
    assert_eq!(crash.object, "oUFOCrash");
    assert_eq!(crash.pos, Vec2::new(160.0, 101.0));
    assert_eq!(crash.vel, Vec2::default());
    assert_eq!(crash.alarms[0], 3);
    assert_eq!(g.world.next_id, first + 2);
    assert_eq!(g.rng.clone().next_u64(), rng.next_u64());
}
#[test]
fn configured_live_moon_stomp_keeps_distinct_callback_and_blood_create_tape() {
    let (mut g, target) = configured(false);
    g.player.pos = Vec2::new(168.0, 100.0);
    g.player.state = PlayerState::Jumping;
    g.player.vel.y = 2.0;
    g.player.set_sprite("sJumpLeft");
    let first = g.world.next_id;
    let (mut rng, births) = blood_oracle(&g, 3);
    hub::tick(&mut g, &InputFrame::default());
    assert!(g.world.entity(target).is_none());
    assert_eq!(g.player.vel.y, -6.4);
    assert_eq!(data(&g, "baskets"), 1.0);
    assert_eq!(sound_names(&g), ["sndHit", "sndCoin"]);
    assert_eq!(g.world.entity(first).unwrap().object, "oBatTarget");
    assert_eq!(g.world.entity(first).unwrap().pos, Vec2::new(160.0, -64.0));
    assert_eq!(g.world.entity(first).unwrap().value("difficulty"), 1.0);
    for (index, (velocity, gravity)) in births.iter().enumerate() {
        let blood = g.world.entity(first + 1 + index as u64).unwrap();
        assert_eq!(blood.pos, Vec2::new(168.0, 109.0));
        assert_eq!(blood.vel, *velocity);
        assert_eq!(blood.value("gravity"), *gravity);
    }
    assert_eq!(g.rng.clone().next_u64(), rng.next_u64());
}
#[test]
fn configured_finished_moon_corpse_contact_keeps_invulnerability_and_source_zero_life_guard() {
    for (life, invincible, bloodless) in [(0, 0, false), (1, 0, false), (0, 0, true), (0, 1, false)]
    {
        let (mut g, target) = configured(true);
        g.player.health = f64::from(life);
        g.player.invincible = invincible;
        g.player.pos = Vec2::new(168.0, 108.0);
        g.player.set_sprite("sDieL");
        g.world
            .entity_mut(target)
            .unwrap()
            .data
            .insert("bloodless".into(), f64::from(bloodless));
        let first = g.world.next_id;
        let count = usize::from(invincible == 0 && !bloodless);
        let (mut rng, births) = blood_oracle(&g, count);
        hub::tick(&mut g, &InputFrame::default());
        assert!(g.world.entity(target).is_some());
        assert_eq!(data(&g, "baskets"), 0.0);
        assert_eq!(
            g.player.health,
            if invincible == 0 {
                f64::from(life.max(1)) - 1.0
            } else {
                f64::from(life)
            }
        );
        assert_eq!(
            g.player.invincible,
            if invincible == 0 { 30 } else { invincible }
        );
        if invincible == 0 {
            assert_eq!(g.player.data["blink"], 30.0);
            assert_eq!(g.player.vel.x, 6.0);
            assert_eq!(sound_names(&g), ["sndHurt"]);
        } else {
            assert!(sound_names(&g).is_empty());
        }
        assert_eq!(
            g.events
                .iter()
                .filter(|e| matches!(e, GameEvent::Damage { .. }))
                .count(),
            usize::from(life > 0 && invincible == 0)
        );
        assert_eq!(g.world.next_id, first + count as u64);
        if let Some((velocity, gravity)) = births.first() {
            let blood = g.world.entity(first).unwrap();
            assert_eq!(blood.pos, Vec2::new(164.0, 105.0));
            assert_eq!(blood.vel, *velocity);
            assert_eq!(blood.value("gravity"), *gravity);
        }
        assert_eq!(g.rng.clone().next_u64(), rng.next_u64());
    }
}
