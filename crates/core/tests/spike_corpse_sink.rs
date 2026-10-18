//! A natural fatal fall exposes the own-Step corpse spike prelude.
use spelunky_core::*;

fn controller_prefix(game: &Game) -> Game {
    let mut prefix = game.clone();
    // Dispatcher advances both clocks before Player; this static fixture has
    // no fluid/moving body or scheduled actor Alarm to alter controller motion.
    prefix.tick += 1;
    prefix.level_tick += 1;
    player::tick(&mut prefix, &InputFrame::default());
    prefix
}

#[test]
fn natural_spike_death_sinks_only_when_its_source_point_is_unsupported() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for floor in [false, true] {
            let mut game = Game::shell(7, rules);
            game.world = World::new(30, 20, Area::Mines, 1);
            game.world.name = "rLevel".into();
            if floor {
                for x in 0..30 {
                    game.world.set_tile(x, 7, "oHardBlock");
                }
            }
            game.player.pos = Vec2::new(88.0, 40.0);
            game.player.previous_pos = game.player.pos;
            let spikes = game.spawn("oSpikes", Vec2::new(80.0, 96.0));
            let mut lethal_gravity = None;
            for _ in 0..30 {
                let prefix = controller_prefix(&game);
                game.step(&InputFrame::default());
                if game.events.contains(&GameEvent::Damage { amount: 99 }) {
                    assert_eq!(
                        game.player.pos, prefix.player.pos,
                        "first fatal frame is not yet source dead"
                    );
                    assert_eq!(game.state, RunState::Dead);
                    assert_eq!(game.player.health, 0.0);
                    lethal_gravity = game.player.data.get("my_gravity").copied();
                    break;
                }
            }
            assert!(
                lethal_gravity.is_some() || game.state == RunState::Dead,
                "natural fatal contact required"
            );
            let mut sinking = 0;
            let mut supported = 0;
            for _ in 0..12 {
                let prefix = controller_prefix(&game);
                let pos = prefix.player.pos;
                let col_spikes = prefix
                    .world
                    .entity(spikes)
                    .unwrap()
                    .sprite_bounds()
                    .intersects(Rect::new(pos.x - 4.0, pos.y - 4.0, 9.0, 13.0));
                let support = prefix
                    .world
                    .tiles_point(Vec2::new(pos.x, pos.y + 9.0), "oSolid");
                let sink = col_spikes && !support;
                sinking += usize::from(sink);
                supported += usize::from(col_spikes && support);
                game.step(&InputFrame::default());
                assert_eq!(game.player.pos.x, pos.x);
                assert_eq!(
                    game.player.pos.y,
                    pos.y + if sink { 0.05 } else { 0.0 },
                    "{rules:?}, floor={floor}, source own-Step prelude"
                );
                assert_eq!(game.state, RunState::Dead);
            }
            assert!(
                sinking > 0,
                "authored corpse must reach unsupported spike overlap"
            );
            assert_eq!(
                supported > 0,
                floor,
                "floor scene also reaches the supported no-sink control"
            );
            assert_eq!(lethal_gravity, Some(0.0));
        }
    }
}
