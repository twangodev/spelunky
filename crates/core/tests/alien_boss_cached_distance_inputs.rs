//! Source-default actors, authored support, ordinary Left then neutral input.
//! No actor/counter/animation/velocity writes after initial construction.
use spelunky_core::*;

fn room(rules: RulesPreset, boss_x: f64) -> (Game, u64) {
    let mut g = Game::shell(42, rules);
    g.world = World::new(24, 18, Area::Ice, 9);
    g.player.pos = Vec2::new(112.0, 152.0);
    g.player.previous_pos = g.player.pos;
    g.world.entrance = g.player.pos;
    for x in 0..24 {
        g.world.set_tile(x, 10, "oBlock");
    }
    let id = g.spawn("oAlienBoss", Vec2::new(boss_x, 128.0));
    assert_eq!(g.player.state, PlayerState::Falling);
    assert_eq!(g.player.vel, Vec2::default());
    let boss = g.world.entity(id).unwrap();
    assert_eq!(boss.state, 0);
    assert_eq!(boss.health, 10);
    assert_eq!(boss.value("psychic_recover"), 100.0);
    assert_eq!(boss.animation, 0.0);
    (g, id)
}

fn attack_counts(g: &Game) -> (usize, usize, usize) {
    let creates = g
        .world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oPsychicCreate")
        .count();
    let waves = g
        .world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oPsychicWave")
        .count();
    let sounds = g
        .events
        .iter()
        .filter(|event| matches!(event, GameEvent::Sound { name, .. } if name == "sndPsychic"))
        .count();
    (creates, waves, sounds)
}

fn recover_naturally(g: &mut Game, id: u64, boss_x: f64) {
    for tick in 1..=100 {
        let input = if tick == 1 {
            InputFrame {
                left: true,
                ..Default::default()
            }
        } else {
            InputFrame::default()
        };
        g.step(&input);
        let boss = g.world.entity(id).unwrap();
        assert!(boss.alive && boss.engine_active);
        assert_eq!(boss.health, 10);
        assert_eq!(boss.state, 0);
        assert_eq!(boss.pos, Vec2::new(boss_x, 128.0));
        assert_eq!(boss.value("psychic_recover"), 100.0 - f64::from(tick));
        assert_eq!(
            attack_counts(g),
            (0, 0, 0),
            "source positive countdown prevents early cast"
        );
        assert_eq!(g.player.health, 4);
    }
    assert_eq!(g.player.pos, Vec2::new(111.0, 152.0));
    assert_eq!(g.player.state, PlayerState::Standing);
    assert_eq!(g.player.sprite, "sStandLeft");
    assert_eq!(g.player.vel, Vec2::default());
    assert_eq!(g.player.data["image_xscale"], 1.0);
    assert_eq!(g.player.stun, 0);
    assert_eq!(g.player.invincible, 0);
    let boss = g.world.entity(id).unwrap();
    assert_eq!(boss.sprite, "sAlienBoss");
    assert_eq!(boss.animation, 1.0);
}

#[test]
fn ordinary_left_then_wait_does_not_cast_at_native_inclusive_gap_96() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut g, id) = room(rules, 211.0);
        let mut expected = g.rng.clone();
        recover_naturally(&mut g, id, 211.0);
        assert_eq!(g.rng.clone().next_u64(), expected.clone().next_u64());
        g.step(&InputFrame::default());
        assert_eq!(g.player.pos, Vec2::new(111.0, 152.0));
        let boss = g.world.entity(id).unwrap();
        assert_eq!(boss.pos, Vec2::new(211.0, 128.0));
        // Original PNG: Stand[2,0,13,15], origin8; Boss[1,1,31,31], origin0.
        // Both are unit positive scale with overlapping Y envelopes.
        // Source right=111+13-8=116, left=211+1=212:96.
        // Old helper's half-open right=117 instead yields95.
        assert_eq!(boss.pos.x + 1.0 - (g.player.pos.x + 5.0), 96.0);
        assert_eq!(boss.pos.x + 1.0 - (g.player.pos.x + 6.0), 95.0);
        assert_eq!(
            boss.value("psychic_recover"),
            0.0,
            "SOURCE_INCLUSIVE_GAP96_MUST_NOT_START_PSYCHIC_CAST"
        );
        assert_eq!(attack_counts(&g), (0, 0, 0));
        assert_eq!(
            g.rng.clone().next_u64(),
            expected.next_u64(),
            "false distance branch consumes no attack RNG"
        );
        assert_eq!(g.player.health, 4);
    }
}

#[test]
fn ordinary_left_then_wait_nearer_boss_positive_control_preserves_six_creates_wave_sound_and_rng() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut g, id) = room(rules, 210.0);
        let mut expected = g.rng.clone();
        recover_naturally(&mut g, id, 210.0);
        for _ in 0..24 {
            expected.range(33);
        }
        g.step(&InputFrame::default());
        assert_eq!(g.player.pos, Vec2::new(111.0, 152.0));
        let boss = g.world.entity(id).unwrap();
        assert_eq!(boss.pos.x + 1.0 - (g.player.pos.x + 5.0), 95.0);
        assert_eq!(boss.value("psychic_recover"), 100.0);
        assert_eq!(attack_counts(&g), (6, 1, 1));
        assert_eq!(
            g.rng.clone().next_u64(),
            expected.next_u64(),
            "six source births each draw four rand(0,32); child Create consumes no RNG"
        );
        assert_eq!(g.player.health, 4);
    }
}

#[test]
fn ordinary_left_then_wait_farther_boss_negative_control_preserves_no_cast_and_rng() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut g, id) = room(rules, 212.0);
        let mut expected = g.rng.clone();
        recover_naturally(&mut g, id, 212.0);
        g.step(&InputFrame::default());
        let boss = g.world.entity(id).unwrap();
        assert_eq!(boss.pos.x + 1.0 - (g.player.pos.x + 5.0), 97.0);
        assert_eq!(boss.value("psychic_recover"), 0.0);
        assert_eq!(attack_counts(&g), (0, 0, 0));
        assert_eq!(g.rng.clone().next_u64(), expected.next_u64());
        assert_eq!(g.player.health, 4);
    }
}
