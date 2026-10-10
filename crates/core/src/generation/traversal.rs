//! Geometry acceptance runs start at the actual generated entrance and use
//! only controller input. Enemy/hazard simulation is deliberately excluded;
//! these catch disconnected collision geometry, not complete-run balance.
use crate::{Game, InputFrame, Rng, RulesPreset, catalog, player};
use std::collections::BTreeSet;

fn traverse(seed: u64, level: u32, actions: &[u8]) {
    let mut game = Game::shell(seed, RulesPreset::Classic);
    game.world = super::generate(&mut Rng::new(seed), level, &BTreeSet::new());
    game.world.entities.retain(|entity| {
        entity.solid
            || catalog::is_a(&entity.object, "oLadder")
            || catalog::is_a(&entity.object, "oPlatform")
            || catalog::is_a(&entity.object, "oWater")
            || entity.object == "oExit"
    });
    game.player.pos = game.world.entrance;
    game.player.previous_pos = game.player.pos;
    let spawn = game.player.pos;
    for &action in actions {
        for phase in 0..6 {
            let jump = matches!(action, 3 | 4 | 5 | 10 | 11 | 12) && phase < 5;
            let input = InputFrame {
                left: matches!(action, 1 | 3 | 6 | 11),
                right: matches!(action, 2 | 4 | 7 | 12),
                up: matches!(action, 6 | 7 | 8 | 10 | 11 | 12),
                down: action == 9,
                jump,
                jump_pressed: jump && phase == 0,
                jump_released: !jump && phase == 5,
                run: true,
                ..InputFrame::default()
            };
            game.tick += 1;
            game.level_tick += 1;
            player::tick(&mut game, &input);
            assert!(
                !game.world.solid_at(game.player.pos.x, game.player.pos.y),
                "seed {seed}, level {level}, tick {} player {:?}",
                game.tick,
                game.player.pos
            );
        }
    }
    assert!(game.player.pos.distance(spawn) > 128.0);
    assert!(game.player.grounded, "must reach the actual supported exit");
    assert!(
        game.world
            .entities
            .iter()
            .any(|entity| entity.object == "oExit" && entity.bounds().contains(game.player.pos)),
        "seed {seed}, level {level}, pos {:?}, exit {:?}",
        game.player.pos,
        game.world.exit
    );
}

#[test]
fn generated_mines_are_traversed_from_the_entrance_with_ordinary_jumps() {
    traverse(
        0,
        1,
        &[
            2, 3, 1, 3, 0, 1, 2, 2, 1, 1, 1, 1, 2, 2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1,
        ],
    );
}

#[test]
fn generated_ice_islands_are_traversed_without_an_artificial_bottom_floor() {
    traverse(
        0,
        9,
        &[
            1, 3, 3, 9, 1, 2, 0, 0, 1, 0, 2, 1, 1, 3, 9, 3, 1, 1, 3, 1, 1, 9, 1, 5, 1, 3, 2,
        ],
    );
}
