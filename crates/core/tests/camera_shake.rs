//! Source oLevel Step25-43 controls, including an ordinary inventory bomb.
use spelunky_core::{Area, Game, GameEvent, InputFrame, PlayerState, RulesPreset, Vec2, World};

fn room(rules: RulesPreset) -> Game {
    let mut game = Game::shell(7, rules);
    game.world = World::new(64, 32, Area::Mines, 1);
    game.flags.insert("custom".into());
    game.flags.insert("low_graphics".into());
    game.player.pos = Vec2::new(40., 120.);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    for x in 0..64 {
        game.world.set_tile(x, 8, "oHardBlock");
    }
    game
}

#[test]
fn ordinary_inventory_bomb_consumes_five_alternating_source_shake_ticks() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = room(rules);
        game.step(&InputFrame {
            down: true,
            bomb_pressed: true,
            ..Default::default()
        });
        assert_eq!(game.player.bombs, 3.0);
        assert!(
            game.world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oBomb")
        );
        let mut exploded = false;
        // Source Bomb Alarm0 waits80 ticks then arms Alarm1 for40.
        for _ in 0..160 {
            let right = game.player.pos.x < 180.;
            game.step(&InputFrame {
                right,
                ..Default::default()
            });
            if game
                .events
                .iter()
                .any(|e| matches!(e, GameEvent::Explosion { .. }))
            {
                exploded = true;
                break;
            }
        }
        assert!(exploded, "ordinary bomb must expire naturally");
        assert_eq!(game.player.health, 4.0, "walk clear of the real explosion");
        // Alarm creates Explosion before resource553; the first shake tick
        // occurs in the birth frame, with no RNG-bearing camera perturbation.
        for (index, (remaining, y)) in [(4., 3.), (3., 0.), (2., 3.), (1., 0.), (0., 3.), (0., 3.)]
            .into_iter()
            .enumerate()
        {
            if index != 0 {
                game.step(&InputFrame::default());
            }
            assert_eq!(game.data.get("shake").copied(), Some(remaining));
            assert_eq!(game.camera.y, y, "source shake frame {index}, {rules:?}");
            assert_eq!(game.presentation().camera.y, y);
            let digest = game.state_hash();
            for _ in 0..32 {
                let _ = game.presentation().state_hash();
            }
            assert_eq!(
                game.state_hash(),
                digest,
                "presentation reads cannot tick shake"
            );
        }
        assert_eq!(game.data.get("camera_vborder").copied(), Some(96.));
        assert_eq!(game.data.get("shake_toggle").copied(), Some(0.));
    }
}

#[test]
fn next_burst_retains_source_toggle_and_current_view_instead_of_recentering() {
    let mut game = room(RulesPreset::Classic);
    // A real Explosion Create is the source scrShake caller; this configured
    // producer control excludes player contact and all destructible terrain.
    for (burst, expected) in [[3., 0., 3., 0., 3.], [0., 3., 0., 3., 0.]]
        .into_iter()
        .enumerate()
    {
        game.spawn("oExplosion", Vec2::new(500., 64.));
        for (index, y) in expected.into_iter().enumerate() {
            game.step(&InputFrame::default());
            assert_eq!(game.camera.y, y, "burst{burst}/frame{index}");
            assert_eq!(game.data["shake"], 4. - index as f64);
        }
    }
}
