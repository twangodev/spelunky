//! Outside Room is a distinct source event, with inclusive cached sprite bounds.
use spelunky_core::*;

fn room(seed: u32, preset: RulesPreset, right: bool) -> Game {
    let mut game = Game::shell(seed, preset);
    game.world = World::new(40, 15, Area::Ice, 9);
    for x in 0..40 {
        game.world.set_tile(x, 8, "oBlock");
    }
    game.player.pos = Vec2::new(if right { 560.0 } else { 40.0 }, 120.0);
    game.player.previous_pos = game.player.pos;
    game.player.grounded = true;
    game.player.state = PlayerState::Standing;
    game.update_camera();
    game
}

#[test]
fn dark_debris_keeps_exact_right_boundary_and_deletes_strictly_outside() {
    for preset in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (object, offset) in [("oRubbleDark", 3.0), ("oRubbleDarkSmall", 4.0)] {
            let mut game = room(17, preset, true);
            let touching = game.spawn(object, Vec2::new(640.0 + offset, 80.0));
            let beyond = game.spawn(object, Vec2::new(641.0 + offset, 80.0));
            assert_eq!(
                game.world.entity(touching).unwrap().sprite_bounds().x,
                640.0
            );
            assert_eq!(game.world.entity(beyond).unwrap().sprite_bounds().x, 641.0);
            game.step(&InputFrame::default());
            let kept = game
                .world
                .entity(touching)
                .expect("left == room_width is retained");
            assert_eq!(kept.pos, Vec2::new(640.0 + offset, 80.0));
            assert_eq!(kept.vel.y, 0.6);
            assert!(game.presentation().world.entity(touching).is_some());
            assert!(game.world.entity(beyond).is_none());
            assert!(game.presentation().world.entity(beyond).is_none());
            assert_eq!(game.player.health, 4);
        }
    }
}

#[test]
fn dark_debris_left_boundary_and_source_step_collisions_remain_literal() {
    for preset in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (object, touching_x) in [("oRubbleDark", -2.0), ("oRubbleDarkSmall", 1.0)] {
            let mut game = room(17, preset, false);
            let touching = game.spawn(object, Vec2::new(touching_x, 80.0));
            let beyond = game.spawn(object, Vec2::new(touching_x - 1.0, 80.0));
            for tile in ["oBrick", "oBlock"] {
                game.world
                    .set_tile(if tile == "oBrick" { 5 } else { 7 }, 5, tile);
            }
            let brick = game.spawn(object, Vec2::new(88.0, 88.0));
            let block = game.spawn(object, Vec2::new(120.0, 88.0));
            game.step(&InputFrame::default());
            assert!(
                game.world.entity(touching).is_some(),
                "inclusive right == 0"
            );
            for killed in [beyond, brick, block] {
                assert!(game.world.entity(killed).is_none(), "{object}/{killed}");
            }
            assert_eq!(game.player.health, 4);
        }
    }
}

// Source rand(a,b) uses one unsigned LCG sample. These authored widths are exact
// under floor(random(width)), independently of the production range helper.
fn literal_draw(state: &mut u32, width: u32) -> u32 {
    *state = state.wrapping_mul(0x0808_8405).wrapping_add(1);
    ((u64::from(*state) * u64::from(width)) >> 32) as u32
}

#[test]
fn alien_ship_destroy_born_debris_at_right_boundary_survives_its_first_step() {
    for preset in [RulesPreset::Classic, RulesPreset::Assisted] {
        // Seed17 projects draws4 and1 in width9, making first large debris x643.
        // The source's literal rand(0-8) is rand(-8,0), giving center+4-(-7).
        let mut expected_state = 17u32;
        let expected: Vec<_> = (0..3)
            .map(|_| {
                Vec2::new(
                    632.0 + f64::from(literal_draw(&mut expected_state, 9))
                        - (f64::from(literal_draw(&mut expected_state, 9)) - 8.0),
                    88.0 + f64::from(literal_draw(&mut expected_state, 9))
                        - (f64::from(literal_draw(&mut expected_state, 9)) - 8.0),
                )
            })
            .collect();
        assert_eq!(expected[0].x, 643.0);
        let mut game = room(17, preset, true);
        let ship = game.spawn("oAlienShip", Vec2::new(624.0, 80.0));
        game.spawn("oExplosion", Vec2::new(632.0, 88.0));
        game.step(&InputFrame::default());
        assert!(game.world.entity(ship).is_none());
        let debris: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|e| matches!(e.object.as_str(), "oRubbleDark" | "oRubbleDarkSmall"))
            .collect();
        assert_eq!(debris.len(), 3);
        for (actual, expected) in debris.iter().zip(expected) {
            assert_eq!(actual.pos, expected);
            assert_eq!(
                actual.vel.y, 0.0,
                "collision-born debris has no earlier Step"
            );
        }
        let large = debris[0].id;
        assert_eq!(debris[0].object, "oRubbleDark");
        assert_eq!(debris[0].sprite_bounds().x, 640.0);
        let actual_state = serde_json::to_value(&game.rng).unwrap()["state"]
            .as_u64()
            .unwrap();
        assert_eq!(actual_state, u64::from(expected_state));
        for velocity in [0.6, 1.2, 1.8] {
            game.step(&InputFrame::default());
            let kept = game.world.entity(large).expect("source edge survivor");
            assert!((kept.vel.y - velocity).abs() < 1e-12);
            assert_eq!(kept.pos.x, 643.0);
            assert!(game.presentation().world.entity(large).is_some());
        }
        assert_eq!(game.player.health, 4);
    }
}
