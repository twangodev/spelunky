//! Source-derived HP-state contract, UNRUN. No visible gameplay consequence
//! beyond public/serialized Actor.health retention is asserted.
//! Initial geometry/free prices/player pose only; no mid-route actor writes.
use spelunky_core::*;
fn scene(rules: RulesPreset, algorithm: RngAlgorithm) -> (Game, u64) {
    let mut g = Game::shell_with_algorithm(7, rules, algorithm).unwrap();
    g.world = World::new(30, 20, Area::Mines, 1);
    for col in 0..g.world.width {
        g.world.set_tile(col, 10, "oBlock");
    }
    g.world.set_tile(9, 9, "oLava");
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
fn walk_held_into_lava(g: &mut Game, id: u64, expected_hp: i32) {
    assert_eq!(g.player.held, Some(id));
    let mut reached = false;
    for _ in 0..12 {
        g.step(&InputFrame {
            left: true,
            ..Default::default()
        });
        assert_eq!(g.player.health, 4, "Hero remains unharmed");
        assert_eq!(g.player.held, Some(id));
        let e = g
            .world
            .entity(id)
            .expect("held Item must prevent retirement");
        if e.pos.x >= 144.0 && e.pos.x < 160.0 && e.pos.y + 6.0 >= 144.0 && e.pos.y + 6.0 < 160.0 {
            assert_eq!(e.value("dead"), 1.0);
            assert_eq!(e.state, 99);
            assert!(
                g.player.pos.x >= 160.0,
                "Hero foot remains outside Lava column"
            );
            assert!(
                e.pos.x >= 144.0 && e.pos.x < 160.0,
                "own Damsel probe reached narrow Lava"
            );
            assert_eq!(e.pos.y, 152.0);
            assert!(e.held);
            assert_eq!(
                e.health, expected_hp,
                "Lava assigns zero only to positive HP"
            );
            reached = true;
            break;
        }
    }
    assert!(reached, "ordinary Left input must reach held Lava branch");
}
#[test]
fn real_machete_negative_hp_then_held_lava_preserves_nonpositive_source_hp() {
    for (rules, algorithm) in modes() {
        let (mut g, damsel) = scene(rules, algorithm);
        let blade = g.spawn("oMachete", g.player.pos);
        g.world
            .entity_mut(blade)
            .unwrap()
            .data
            .insert("cost".into(), 0.0);
        g.step(&InputFrame {
            down: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert_eq!(g.player.held, Some(blade), "real Machete pickup");
        g.step(&InputFrame::default());
        // Fixed ordinary attacks, not a seed/outcome search. Each reached
        // Machete collision subtracts native damage2 with hit cooldown10.
        for _ in 0..3 {
            let hp = g.world.entity(damsel).unwrap().health;
            g.step(&InputFrame {
                attack_pressed: true,
                ..Default::default()
            });
            for _ in 0..20 {
                g.step(&InputFrame::default());
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
            g.step(&InputFrame {
                right: true,
                ..Default::default()
            });
        }
        g.step(&InputFrame {
            left: true,
            ..Default::default()
        });
        assert_eq!(g.player.facing, -1);
        assert_eq!(g.player.health, 4);
        g.step(&InputFrame {
            down: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert_eq!(g.player.held, None, "real Down weapon drop");
        for _ in 0..8 {
            g.step(&InputFrame::default());
            assert_eq!(g.player.health, 4);
        }
        let mut near = false;
        for _ in 0..12 {
            if g.player.pos.x >= 176.0 {
                near = true;
                break;
            }
            g.step(&InputFrame {
                right: true,
                ..Default::default()
            });
            assert_eq!(g.player.health, 4);
        }
        assert!(near, "real walking must reach a nearer Damsel pickup pose");
        assert!(g.player.pos.x <= 184.0);
        g.step(&InputFrame {
            down: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert_eq!(g.player.held, Some(damsel), "real negative Damsel pickup");
        g.step(&InputFrame::default());
        let hp = g.world.entity(damsel).unwrap().health;
        assert!(hp < 0);
        walk_held_into_lava(&mut g, damsel, hp);
    }
}
#[test]
fn real_positive_damsel_pickup_then_held_lava_still_zeroes_hp_control() {
    for (rules, algorithm) in modes() {
        let (mut g, damsel) = scene(rules, algorithm);
        g.step(&InputFrame {
            down: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert_eq!(g.player.held, Some(damsel));
        g.step(&InputFrame::default());
        assert_eq!(g.world.entity(damsel).unwrap().health, 4);
        walk_held_into_lava(&mut g, damsel, 0);
    }
}
