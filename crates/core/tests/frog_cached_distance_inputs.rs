//! Source-default actors in an authored supported room. Only ordinary Right
//! input and Game.step run after construction; no actor or animation writes.
use spelunky_core::*;

fn room(rules: RulesPreset, frog_x: f64) -> (Game, u64) {
    let mut g = Game::shell(42, rules);
    g.world = World::new(24, 18, Area::Jungle, 5);
    g.player.pos = Vec2::new(80.0, 152.0);
    g.player.previous_pos = g.player.pos;
    g.player.state = PlayerState::Standing;
    g.player.grounded = true;
    g.world.entrance = g.player.pos;
    for x in 0..24 {
        g.world.set_tile(x, 10, "oHardBlock");
    }
    let frog = g.spawn("oFrog", Vec2::new(frog_x, 144.0));
    assert_eq!(g.player.sprite, "sStandLeft");
    assert_eq!(g.player.animation, 0.0);
    assert_eq!(g.player.vel, Vec2::default());
    let e = g.world.entity(frog).unwrap();
    assert_eq!(e.state, 0);
    assert_eq!(e.timer, 0);
    assert_eq!(e.sprite, "sFrogLeft");
    assert_eq!(e.health, 1);
    (g, frog)
}

fn croaks(g: &Game) -> usize {
    g.events
        .iter()
        .filter(|event| {
            matches!(event,
                GameEvent::Sound { name, .. } if name == "sndFrog"
            )
        })
        .count()
}

fn first_eight(g: &mut Game, id: u64, frog_x: f64) {
    let right = InputFrame {
        right: true,
        ..Default::default()
    };
    for _ in 0..8 {
        g.step(&right);
        let frog = g.world.entity(id).unwrap();
        assert!(frog.alive && frog.engine_active);
        assert_eq!(frog.state, 0, "no earlier proximity activation");
        assert_eq!(frog.pos, Vec2::new(frog_x, 144.0));
        assert_eq!(croaks(g), 0);
        assert_eq!(g.player.health, 4);
    }
    assert_eq!(g.player.pos, Vec2::new(97.0, 152.0));
    assert_eq!(g.player.state, PlayerState::Running);
    assert_eq!(g.player.sprite, "sRunLeft");
    assert!((g.player.animation - 2.268).abs() < 1e-9);
    assert_eq!(g.player.data["image_xscale"], -1.0);
    assert_eq!(
        g.player.data.get("image_angle").copied().unwrap_or(0.0),
        0.0
    );
}

#[test]
fn ordinary_right_walk_wakes_default_frog_at_native_aggregate_gap_63() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut g, id) = room(rules, 168.0);
        let mut rng = g.rng.clone();
        first_eight(&mut g, id, 168.0);
        g.step(&InputFrame {
            right: true,
            ..Default::default()
        });
        assert_eq!(g.player.pos, Vec2::new(100.0, 152.0));
        assert_eq!(g.player.sprite, "sRunLeft");
        assert_eq!(g.player.data["image_xscale"], -1.0);
        let frog = g.world.entity(id).unwrap();
        assert!(frog.alive && frog.engine_active);
        assert_eq!(frog.pos, Vec2::new(168.0, 144.0));
        assert_eq!(frog.sprite, "sFrogLeft");
        // Independently decoded source masks: sRunLeft aggregate[1,0,14,15],
        // origin8,8; selected frame2[2,2,13,15]; Frog[2,4,14,15], origin0.
        // Prior Right Draw makes native Player right100+(8-1)=107;
        // smooth selected-frame reflection right100+(8-2)=106.
        assert_eq!(170.0_f64 - 107.0, 63.0);
        assert_eq!(170.0_f64 - 106.0, 64.0);
        assert_eq!(
            frog.state, 1,
            "SOURCE_GAP63_MUST_WAKE_FROG_BEFORE_OLD_GAP64"
        );
        assert_eq!(croaks(&g), 1);
        assert_eq!(g.player.health, 4);
        assert_eq!(
            g.rng.clone().next_u64(),
            rng.next_u64(),
            "distance/IDLE sound consumes no RNG"
        );
    }
}

#[test]
fn ordinary_right_walk_nearer_frog_positive_control_keeps_croak_and_rng() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut g, id) = room(rules, 166.0);
        let mut rng = g.rng.clone();
        first_eight(&mut g, id, 166.0);
        g.step(&InputFrame {
            right: true,
            ..Default::default()
        });
        assert_eq!(g.player.pos, Vec2::new(100.0, 152.0));
        assert_eq!(g.world.entity(id).unwrap().state, 1);
        assert_eq!(croaks(&g), 1);
        assert_eq!(g.rng.clone().next_u64(), rng.next_u64());
    }
}

#[test]
fn ordinary_right_walk_farther_frog_negative_control_keeps_idle_and_rng() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut g, id) = room(rules, 170.0);
        let mut rng = g.rng.clone();
        first_eight(&mut g, id, 170.0);
        g.step(&InputFrame {
            right: true,
            ..Default::default()
        });
        assert_eq!(g.player.pos, Vec2::new(100.0, 152.0));
        assert_eq!(g.world.entity(id).unwrap().state, 0);
        assert_eq!(croaks(&g), 0);
        assert_eq!(g.rng.clone().next_u64(), rng.next_u64());
    }
}
