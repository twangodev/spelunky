//! UNRUN input-only transition following configured initial room/free actor.
//! No intermediate state, health, velocity, equipment or position writes.
//! Water cannot kill Damsels; the narrow Lava cell is intentional.
use spelunky_core::*;

fn ordinary_room(rules: RulesPreset, algorithm: RngAlgorithm, lava: bool) -> (Game, u64, u64) {
    let mut g = Game::shell_with_algorithm(7, rules, algorithm).unwrap();
    g.world = World::new(30, 20, Area::Mines, 1);
    for col in 0..g.world.width {
        g.world.set_tile(col, 10, "oBlock");
    }
    if lava {
        g.world.set_tile(9, 9, "oLava");
    }
    g.player.pos = Vec2::new(160.0, 152.0);
    g.player.previous_pos = g.player.pos;
    g.player.state = PlayerState::Standing;
    g.player.grounded = true;
    g.player.facing = -1;
    g.set_simulation_view_extent(480.0, 320.0).unwrap();
    let id = g.spawn("oDamsel", Vec2::new(164.0, 152.0));
    g.world
        .entity_mut(id)
        .unwrap()
        .data
        .insert("cost".into(), 0.0);
    let exit = g.spawn("oExit", Vec2::new(232.0, 144.0));
    assert_eq!(g.world.entity(id).unwrap().health, 4);
    assert_eq!(g.world.entity(id).unwrap().state, 0);
    assert_eq!(g.world.entity(id).unwrap().value("startled"), 0.0);
    assert_eq!(g.lifetime_stats.damsels_grabbed, 0);
    (g, id, exit)
}

fn pickup_and_walk_to_exit(g: &mut Game, id: u64, exit: u64, dead: bool) {
    g.step(&InputFrame {
        down: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert_eq!(g.player.held, Some(id), "real pickup prerequisite");
    assert!(g.world.entity(id).unwrap().held);
    assert_eq!(g.player.health, 4.0, "Hero raw lava probe remains dry");
    // Release Down so a large held Damsel returns from Hero.y+2 to Hero.y.
    // LEFT attachment is Hero.x-4; at x156,y152 its own y+6 Lava probe hits,
    // while Hero's x160,y+6 point is outside the cell's inclusive 144..159.
    g.step(&InputFrame::default());
    let actor = g
        .world
        .entity(id)
        .expect("held Item branch must prevent full submersion retirement");
    assert_eq!(actor.pos, Vec2::new(156.0, 152.0));
    assert!(actor.held);
    assert_eq!(g.player.held, Some(id));
    assert_eq!(g.player.health, 4.0);
    if dead {
        assert_eq!(
            actor.health, 0,
            "own Damsel Lava branch must be reached naturally"
        );
        assert_eq!(actor.state, 99);
        assert_eq!(actor.value("dead"), 1.0);
    } else {
        assert_eq!(actor.health, 4);
    }
    assert_eq!(actor.value("startled"), 0.0, "no prior THROWN branch");
    assert_eq!(g.lifetime_stats.damsels_grabbed, 0);
    let mut at_exit = false;
    for _ in 0..80 {
        // Changing direction is genuine input; it carries the corpse away
        // from Lava before release, so Item retirement cannot hide the tail.
        g.step(&InputFrame {
            right: true,
            ..Default::default()
        });
        assert_eq!(g.player.health, 4.0);
        if g.world
            .entity(exit)
            .unwrap()
            .mask_contains(g.player.pos, CollisionPrecision::BoundingBox)
        {
            at_exit = true;
            break;
        }
        assert_eq!(g.player.held, Some(id));
    }
    assert!(at_exit, "ordinary walking must reach the authored exit");
    let actor = g.world.entity(id).expect("walking held actor must survive");
    if dead {
        assert_eq!(g.player.held, Some(id));
        assert_eq!(actor.health, 0);
        assert_eq!(actor.state, 99);
        assert_eq!(actor.value("startled"), 0.0);
        assert_eq!(actor.value("dead"), 1.0);
        assert!(actor.held);
        assert!(actor.pos.x > 176.0, "corpse release is well clear of Lava");
        assert_eq!(g.lifetime_stats.damsels_grabbed, 0);
    } else {
        // The separate touching-exit deposit runs without Up and rescues a
        // living held Damsel before the next ordinary exit input.
        assert_eq!(g.player.held, None);
        assert_eq!(actor.state, 4);
        assert_eq!(actor.health, 4);
        assert!(!actor.held);
    }
}

#[test]
fn real_held_lava_death_then_dry_exit_release_runs_thrown_startled_and_corpse_tail() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for algorithm in [RngAlgorithm::Gm8Lcg32V1, RngAlgorithm::LegacySplitMix64V1] {
            let (mut g, id, exit) = ordinary_room(rules, algorithm, true);
            pickup_and_walk_to_exit(&mut g, id, exit, true);
            let killed = g.lifetime_stats.damsels_killed_total;
            let saved = g.lifetime_stats.damsels_saved_total;
            g.step(&InputFrame {
                up: true,
                ..Default::default()
            });
            assert!(g.player.is_exiting(), "real exit input prerequisite");
            assert_eq!(g.player.health, 4.0);
            assert_eq!(g.player.held, None);
            assert!(g.player.pickup_item_type.is_empty());
            let actor = g
                .world
                .entity(id)
                .expect("dry dropped corpse remains a live instance");
            assert!(!actor.held);
            assert_eq!(actor.health, 0);
            assert_eq!(actor.state, 99, "THROWN tail reselects DEAD when grounded");
            assert_eq!(actor.value("dead"), 1.0);
            // Player319 sets THROWN2; Damsel556 then sets startled, counts
            // the grab, and selects sDamselDieL from its grounded corpse tail.
            assert_eq!(actor.value("startled"), 1.0);
            assert_eq!(actor.sprite, "sDamselDieL");
            assert_eq!(g.lifetime_stats.damsels_grabbed, 1);
            assert_eq!(g.lifetime_stats.damsels_killed_total, killed);
            assert_eq!(g.lifetime_stats.damsels_saved_total, saved);
        }
    }
}

#[test]
fn real_healthy_pickup_walk_and_exit_preserves_live_damsel_rescue_control() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for algorithm in [RngAlgorithm::Gm8Lcg32V1, RngAlgorithm::LegacySplitMix64V1] {
            let (mut g, id, exit) = ordinary_room(rules, algorithm, false);
            pickup_and_walk_to_exit(&mut g, id, exit, false);
            assert_eq!(g.lifetime_stats.damsels_saved_total, 1);
            assert_eq!(g.lifetime_stats.damsels_grabbed, 0);
            g.step(&InputFrame {
                up: true,
                ..Default::default()
            });
            assert!(g.player.is_exiting());
            assert_eq!(g.player.health, 4.0);
            assert_eq!(g.player.held, None);
            assert_eq!(g.lifetime_stats.damsels_saved_total, 1);
            assert_eq!(g.lifetime_stats.damsels_grabbed, 0);
        }
    }
}
