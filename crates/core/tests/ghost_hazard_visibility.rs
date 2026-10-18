//! A natural Ghost death supplies the source visible=false own-hurt guard.
use spelunky_core::*;

#[test]
fn natural_ghost_hidden_corpse_ignores_later_blast_while_visible_player_is_hit() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for ghost in [true, false] {
            let mut game = Game::shell(7, rules);
            game.world = World::new(30, 20, Area::Mines, 1);
            game.world.name = "rLevel".into();
            for column in 0..30 {
                game.world.set_tile(column, 10, "oHardBlock");
            }
            game.player.pos = Vec2::new(104.0, 152.0);
            game.player.previous_pos = game.player.pos;
            let bomb = game.spawn("oFireFrogBomb", Vec2::new(120.0, 152.0));
            if ghost {
                game.spawn("oGhost", Vec2::new(128.0, 136.0));
            }
            let mut hidden = false;
            let mut first_death = false;
            let mut blast = false;
            let mut live_hit = false;
            for _ in 0..132 {
                game.step(&InputFrame::default());
                let visible = game.player.data.get("visible").copied().unwrap_or(1.0) != 0.0;
                if ghost && !hidden && !visible {
                    assert_eq!(game.player.health, -99.0, "natural Ghost own Collision");
                    assert_eq!(game.player.invincible, 9999);
                    assert_eq!(game.player.data.get("hidden").copied().unwrap_or(0.0), 0.0);
                    assert!(game.events.iter().any(|event| matches!(event,
                        GameEvent::Sound { name, .. } if name == "sndGhost"
                    )));
                    assert!(game.world.entity(bomb).is_some(), "Ghost precedes fuse");
                    hidden = true;
                }
                if ghost && hidden && game.state == RunState::Dead {
                    first_death = true;
                    assert!(!visible);
                    assert_eq!(
                        game.player.health, 0.0,
                        "invisible corpse skips source own-hurt block"
                    );
                    assert!(!game.events.contains(&GameEvent::Damage { amount: 10 }));
                }
                if let Some(explosion) = game
                    .world
                    .entities
                    .iter()
                    .find(|e| e.alive && e.object == "oExplosion")
                {
                    let pos = game.player.pos;
                    if explosion.mask_intersects(
                        Rect::new(pos.x - 8.0, pos.y - 8.0, 17.0, 17.0),
                        CollisionPrecision::BoundingBox,
                    ) {
                        blast = true;
                        if ghost {
                            assert!(
                                hidden && first_death,
                                "invisible death precedes actual blast overlap"
                            );
                        }
                    }
                }
                if !ghost && game.events.contains(&GameEvent::Damage { amount: 10 }) {
                    live_hit = true;
                    break;
                }
            }
            assert!(blast, "natural source Bomb Alarm must reach Player query");
            assert!(game.world.entity(bomb).is_none(), "natural fuse completed");
            if ghost {
                assert!(hidden && first_death);
                assert_eq!(game.player.health, 0.0);
                assert!(!live_hit);
            } else {
                assert!(live_hit, "visible control takes the same natural explosion");
                assert_eq!(game.state, RunState::Dead);
            }
        }
    }
}
