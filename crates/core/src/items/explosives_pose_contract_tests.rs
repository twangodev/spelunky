//! Ordinary reservation/carry inputs plus explicitly configured source-state
//! controls. The blast-result controls do not claim an untouched blast route.
use super::*;

fn room(rules: RulesPreset) -> Game {
    let mut game = Game::shell(42, rules);
    game.world = World::new(30, 20, Area::Mines, 1);
    game.world.name = "rLevel".into();
    for x in 0..30 {
        game.world.set_tile(x, 7, "oHardBlock");
    }
    game.player.pos = Vec2::new(80.0, 104.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game
}

#[test]
fn ordinary_reserved_bomb_carry_uses_duck_and_standing_light_item_offsets() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for facing in [-1, 1] {
            let mut game = room(rules);
            game.player.facing = facing;
            game.step(&InputFrame {
                cycle_pressed: true,
                ..Default::default()
            });
            let id = game.player.held.expect("ordinary Cycle reserves Bomb");
            assert_eq!(game.world.entity(id).unwrap().object, "oBomb");
            assert_eq!(game.player.bombs, 3.0);
            assert_eq!(game.world.entity(id).unwrap().value("armed"), 0.0);
            game.step(&InputFrame {
                down: true,
                ..Default::default()
            });
            assert_eq!(game.player.state, PlayerState::Ducking);
            assert_eq!(game.player.vel.x, 0.0);
            let bomb = game.world.entity(id).unwrap();
            assert!(bomb.held);
            assert_eq!(game.player.held, Some(id));
            assert_eq!(
                bomb.pos,
                Vec2::new(
                    game.player.pos.x + 4.0 * f64::from(facing),
                    game.player.pos.y + 4.0,
                )
            );
            assert_eq!(bomb.vel, Vec2::default());
            game.step(&InputFrame::default());
            assert_eq!(game.player.state, PlayerState::Standing);
            assert_eq!(
                game.world.entity(id).unwrap().pos.y,
                game.player.pos.y + 2.0
            );
            assert_eq!(game.player.health, 4.0);
            assert_eq!(game.player.bombs, 3.0);
        }
    }
}

#[test]
fn configured_held_bomb_body_keeps_the_strict_duck_speed_boundary() {
    // Isolated parent-body controls: caller/controller state is initial data,
    // not produced by a gameplay route. Source predicate is strictly abs(vx)<2.
    for (state, vx, offset) in [
        (PlayerState::Ducking, -1.999, 4.0),
        (PlayerState::Ducking, 1.999, 4.0),
        (PlayerState::Ducking, -2.0, 2.0),
        (PlayerState::Ducking, 2.0, 2.0),
        (PlayerState::Standing, 0.0, 2.0),
    ] {
        let mut game = room(RulesPreset::Classic);
        game.player.state = state;
        game.player.vel.x = vx;
        let id = game.spawn("oBomb", game.player.pos);
        game.player.held = Some(id);
        let mut bomb = game.world.entity(id).unwrap().clone();
        bomb.held = true;
        step_bomb_body(&mut game, &mut bomb);
        assert_eq!(bomb.pos.y, game.player.pos.y + offset);
        assert_eq!(bomb.vel, Vec2::default());
        assert!(bomb.alive);
    }
}

#[test]
fn configured_blast_result_pasted_bomb_stops_by_sprite_independently_of_armed() {
    // Source-valid initial property combinations. sBombArmed/armed=false is
    // Explosion's unarmed Bomb result; sBomb/armed=true is a release result.
    // Neither producer is injected, and no state is edited after Game::step.
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (sprite, armed, sticks) in [
            ("sBombArmed", 0.0, true),
            ("sBombArmed", 1.0, true),
            ("sBomb", 0.0, false),
            ("sBomb", 1.0, false),
        ] {
            let mut game = room(rules);
            let id = game.spawn("oBomb", Vec2::new(96.0, 108.0));
            let bomb = game.world.entity_mut(id).unwrap();
            bomb.set_sprite(sprite);
            bomb.vel.x = 4.0;
            bomb.data.insert("sticky".into(), 1.0);
            bomb.data.insert("armed".into(), armed);
            game.step(&InputFrame::default());
            let bomb = game.world.entity(id).unwrap();
            assert_eq!(bomb.value("colBot"), 1.0);
            assert_eq!(bomb.pos.x, 100.0);
            assert_eq!(bomb.pos.y, if sticks { 109.0 } else { 108.0 });
            assert_eq!(bomb.vel.x, if sticks { 0.0 } else { 4.0 * 0.3 });
            assert_eq!(bomb.vel.y, 0.0);
            assert_eq!(bomb.value("armed"), armed);
            assert_eq!(bomb.value("attached_id"), 0.0);
            assert_eq!(game.player.health, 4.0);
        }
    }
}

#[test]
fn configured_down_bomb_action_reads_source_state_instead_of_geometric_support() {
    // Action-time controls isolate the ON_GROUND consumer after the controller.
    // A complete ordinary ledge/controller route is a separate obligation.
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (state, support, hanging, vx) in [
            (PlayerState::Standing, false, 0.0, 0.8),
            (PlayerState::Running, false, 0.0, 0.8),
            (PlayerState::Ducking, false, 0.0, 0.8),
            (PlayerState::LookingUp, false, 0.0, 0.8),
            (PlayerState::Falling, true, 0.0, 8.0),
            (PlayerState::Jumping, true, 0.0, 8.0),
            (PlayerState::Standing, true, 1.0, 8.0),
        ] {
            let mut game = room(rules);
            game.player.state = state;
            game.player.grounded = support;
            game.player.data.insert("duck_to_hang".into(), hanging);
            let id = game.world.next_id;
            use_bomb(
                &mut game,
                &InputFrame {
                    down: true,
                    bomb_pressed: true,
                    ..Default::default()
                },
            );
            let bomb = game.world.entity(id).unwrap();
            assert_eq!(bomb.object, "oBomb");
            assert_eq!(bomb.vel, Vec2::new(vx, 3.0));
            assert_eq!(bomb.sprite, "sBombArmed");
            assert_eq!(bomb.alarms[0], 80);
            assert_eq!(bomb.alarms[2], 10);
            assert_eq!(game.player.bombs, 3.0);
        }
    }
}
