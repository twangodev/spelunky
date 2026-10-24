//! Native Ghost Create + actual Step/Collision dispatch. Player invincibility
//! is an explicit source-valid control so overlapping Ghost contact is harmless.
use super::*;

#[test]
fn ghost_actual_step_coincident_target_uses_zero_degrees_then_postmove_turns() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for coincident in [false, true] {
            let mut game = Game::shell(17, rules);
            game.world = World::new(24, 20, Area::Mines, 1);
            game.world.name = "rLevel".into();
            for x in 0..24 {
                game.world.set_tile(x, 6, "oHardBlock");
            }
            game.player.pos = Vec2::new(if coincident { 88.0 } else { 104.0 }, 88.0);
            game.player.previous_pos = game.player.pos;
            game.player.grounded = true;
            game.player.state = PlayerState::Standing;
            game.player.invincible = 2;
            game.world.entrance = game.player.pos;
            let target = game.player.pos;
            let id = game.spawn("oGhost", Vec2::new(80.0, 80.0));
            assert_eq!(game.world.entity(id).unwrap().state, 1);
            assert_eq!(game.world.entity(id).unwrap().sprite, "sGhostRight");
            let rng = game.rng.state_bits();
            game.step(&InputFrame::default());
            assert_eq!(game.player.pos, target);
            assert_eq!(game.player.invincible, 1);
            assert_eq!(game.player.health, 4.0);
            let ghost = game.world.entity(id).unwrap();
            assert_eq!(ghost.pos, Vec2::new(81.0, 80.0));
            assert_eq!(ghost.state, 1, "invincible contact does not dismiss Ghost");
            assert_eq!(
                ghost.sprite,
                if coincident {
                    "sGhostTurnLeft"
                } else {
                    "sGhostRight"
                }
            );
            assert_eq!(
                game.rng.state_bits(),
                rng,
                "chase and protected collision consume no draws"
            );
        }
    }
}

#[test]
fn lethal_attacking_ghost_finishes_its_source_step_tail_before_retirement() {
    let mut game = Game::shell(17, RulesPreset::Classic);
    game.player.pos = Vec2::new(64.0, 88.0);
    let mut ghost = Entity::new(7, "oGhost", Vec2::new(80.0, 80.0));
    init(&mut ghost);
    ghost.health = 0;
    ghost.state = 1;
    ghost.sprite = "sGhostRight".into();

    tick(&mut game, &mut ghost);

    assert!(!ghost.alive, "source instance_destroy retires the Ghost");
    assert_eq!(ghost.pos, Vec2::new(79.0, 80.0));
    assert_eq!(ghost.sprite, "sGhostTurnLeft");
    assert_eq!(game.data["kills"], 1.0);
}
