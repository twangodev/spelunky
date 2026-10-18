//! Configured animated surface boundary followed by the actual early Player Step.
use spelunky_core::*;

#[test]
fn lava_top_point_uses_active_cached_aggregate_bounds() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (player_y, active, lethal) in [
            (160.0, true, true),
            (159.0, true, false),
            (160.0, false, false),
        ] {
            let mut game = Game::shell(42, rules);
            game.world = World::new(40, 24, Area::Mines, 1);
            // This configured room has no Level activation controller.
            game.world.name = "rIsolatedLavaPoint".into();
            game.player.pos = Vec2::new(160.0, player_y);
            game.player.previous_pos = game.player.pos;
            game.player.state = PlayerState::Standing;
            let lava = game.spawn("oLava", Vec2::new(160.0, 166.0));
            // Containment prevents oGame's real fluid pass from draining it.
            for pos in [
                Vec2::new(144.0, 166.0),
                Vec2::new(176.0, 166.0),
                Vec2::new(160.0, 182.0),
            ] {
                game.spawn("oHardBlock", pos);
            }
            let surface = game.world.entity_mut(lava).unwrap();
            surface.set_sprite("sLavaTop");
            surface.animation = 2.0;
            surface.engine_active = active;
            let foot = Vec2::new(160.0, player_y + 6.0);
            assert_eq!(
                surface.mask_contains(foot, CollisionPrecision::BoundingBox),
                player_y == 160.0
            );
            if player_y == 160.0 {
                assert!(
                    !surface.sprite_bounds().contains(foot),
                    "selected frame omits the aggregate top row"
                );
            }
            game.step(&InputFrame::default());
            let damage = game
                .events
                .iter()
                .filter(|event| matches!(event, GameEvent::Damage { amount: 99 }))
                .count();
            assert_eq!(damage, usize::from(lethal));
            assert_eq!(game.state == RunState::Dead, lethal);
            if !lethal {
                assert_eq!(game.player.health, 4.0);
            }
        }
    }
}
