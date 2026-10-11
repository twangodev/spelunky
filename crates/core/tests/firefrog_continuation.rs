//! Source-derived controlled initial states; uncompiled and unexecuted.
//! Every state write is before the first actual Game.step. No natural producer
//! or original-executable runtime capture is claimed by these fixtures.
use spelunky_core::*;

fn room(rules: RulesPreset, water: bool, floor: bool, health: i32, burning: f64) -> (Game, u64) {
    let mut game = Game::shell(42, rules);
    game.world = World::new(30, 20, Area::Jungle, 5);
    game.player.pos = Vec2::new(240.0, 120.0);
    game.player.previous_pos = game.player.pos;
    let pos = if floor {
        Vec2::new(96.0, 112.0)
    } else {
        Vec2::new(96.0, 80.0)
    };
    if floor {
        for x in 0..30 {
            game.world.set_tile(x, 8, "oBlock");
        }
    }
    if water {
        game.world
            .set_tile(6, if floor { 7 } else { 5 }, "oWaterSwim");
    }
    assert!(
        !game.world.fluid_dirty,
        "initial source water-check latch is false"
    );
    let id = game.spawn("oFireFrog", pos);
    let frog = game.world.entity_mut(id).unwrap();
    frog.health = health;
    frog.state = 2; // source RECOVER
    frog.vel = Vec2::default();
    frog.data.insert("burning".into(), burning);
    assert!(frog.engine_active && frog.alive);
    (game, id)
}

fn next_parent_burn(expected: &mut Rng, burning: f64) -> usize {
    if burning > 0.0 && expected.range(5) == 0 {
        // Both current FireFrog canvases are16x16; source inclusive endpoints.
        expected.range(17);
        expected.range(17);
        1
    } else {
        0
    }
}

#[test]
fn actual_step_water_recover_runs_retired_child_after_frog_create_and_parent_burn() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for burning in [0.0, 1.0] {
            let (mut game, old) = room(rules, true, true, 1, burning);
            let mut expected = game.rng.clone();
            let facing = if expected.range(2) == 0 { -1 } else { 1 }; // Frog Create
            let burns = next_parent_burn(&mut expected, burning);
            expected.range(31); // retired FireFrog RECOVER floor rand(10,40)

            game.step(&InputFrame::default());

            assert!(game.world.entity(old).is_none());
            let frogs: Vec<_> = game
                .world
                .entities
                .iter()
                .filter(|entity| entity.alive && entity.object == "oFrog")
                .collect();
            assert_eq!(frogs.len(), 1);
            assert_eq!(frogs[0].pos, Vec2::new(96.0, 112.0));
            assert_eq!(frogs[0].facing, facing);
            assert_eq!(
                frogs[0].state, 2,
                "parent copies old status before the retired child changes its own status"
            );
            assert_eq!(
                frogs[0].timer, 0,
                "newborn Frog554 cannot Step in the already-visited resource this frame"
            );
            assert_eq!(
                game.world
                    .entities
                    .iter()
                    .filter(|e| e.object == "oBurn")
                    .count(),
                burns
            );
            assert!(
                !game
                    .world
                    .entities
                    .iter()
                    .any(|e| e.object == "oFireFrogBomb")
            );
            assert_eq!(game.rng.clone().next_u64(), expected.next_u64());
        }
    }
}

#[test]
fn actual_step_water_hp_zero_births_frog_then_bomb_and_continues_recover_tail() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, old) = room(rules, true, true, 0, 0.0);
        let mut expected = game.rng.clone();
        expected.range(2); // Frog Create before old child death
        expected.range(31); // RECOVER tail after Bomb Create and Destroy

        game.step(&InputFrame::default());

        assert!(game.world.entity(old).is_none());
        let frog = game
            .world
            .entities
            .iter()
            .find(|e| e.object == "oFrog")
            .unwrap();
        let bombs: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|e| e.object == "oFireFrogBomb")
            .collect();
        assert_eq!(
            bombs.len(),
            1,
            "source old receiver still takes its hp<1 child branch"
        );
        assert!(frog.creation_serial < bombs[0].creation_serial);
        assert_eq!(frog.state, 2);
        assert_eq!(
            bombs[0].alarms[1], 120,
            "born after this frame's alarm pass"
        );
        assert_eq!(game.rng.clone().next_u64(), expected.next_u64());

        game.step(&InputFrame::default());
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|e| e.object == "oFireFrogBomb")
                .count(),
            1,
            "retired old receiver does not take the child again next frame"
        );
    }
}

#[test]
fn actual_step_dry_hp_zero_own_destroy_still_runs_recover_tail() {
    let (mut game, old) = room(RulesPreset::Classic, false, true, 0, 0.0);
    let mut expected = game.rng.clone();
    expected.range(31); // no Water Frog Create; own death continues RECOVER

    game.step(&InputFrame::default());

    assert!(game.world.entity(old).is_none());
    assert!(!game.world.entities.iter().any(|e| e.object == "oFrog"));
    assert_eq!(
        game.world
            .entities
            .iter()
            .filter(|e| e.object == "oFireFrogBomb")
            .count(),
        1
    );
    assert_eq!(game.rng.clone().next_u64(), expected.next_u64());
}

#[test]
fn actual_step_airborne_water_recover_control_has_no_floor_tail_draw_or_bomb() {
    let (mut game, old) = room(RulesPreset::Classic, true, false, 1, 0.0);
    let mut expected = game.rng.clone();
    expected.range(2); // only new Frog Create

    game.step(&InputFrame::default());

    assert!(game.world.entity(old).is_none());
    assert_eq!(
        game.world
            .entities
            .iter()
            .filter(|e| e.object == "oFrog")
            .count(),
        1
    );
    assert!(
        !game
            .world
            .entities
            .iter()
            .any(|e| e.object == "oFireFrogBomb")
    );
    assert_eq!(game.rng.clone().next_u64(), expected.next_u64());
}
