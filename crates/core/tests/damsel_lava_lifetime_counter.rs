//! Source-gated Damsel lava lifetime credit through public Game::step.
use spelunky_core::{Area, Game, InputFrame, RulesPreset, Vec2, World};

fn scene(hp: i32) -> (Game, u64) {
    let mut game = Game::shell(123, RulesPreset::Classic);
    game.world = World::new(30, 20, Area::Mines, 1);
    game.player.pos = Vec2::new(240.0, 128.0);
    game.player.previous_pos = game.player.pos;
    game.spawn("oLava", Vec2::new(96.0, 96.0));
    let damsel = game.spawn("oDamsel", Vec2::new(104.0, 90.0));
    game.world.entity_mut(damsel).unwrap().health = hp;
    (game, damsel)
}

#[test]
fn lava_credits_positive_hp_only_to_lifetime_counter_once() {
    for (initial_hp, expected_hp, expected_credits) in [(4, 0, 1), (-1, -1, 0)] {
        let (mut game, damsel) = scene(initial_hp);
        for _ in 0..6 {
            game.step(&InputFrame::default());
            if game
                .world
                .entity(damsel)
                .is_some_and(|entity| entity.value("dead") != 0.0)
            {
                break;
            }
        }

        let entity = game.world.entity(damsel).unwrap();
        assert_eq!(
            entity.health, expected_hp,
            "source HP gate for {initial_hp} HP"
        );
        assert_eq!(entity.value("dead"), 1.0);
        assert_eq!(game.lifetime_stats.damsels_killed_total, expected_credits);
        assert_eq!(game.data.get("kills").copied().unwrap_or(0.0), 0.0);
        assert_eq!(game.data.get("damselsKilled").copied().unwrap_or(0.0), 0.0);

        for _ in 0..4 {
            game.step(&InputFrame::default());
        }
        assert_eq!(game.lifetime_stats.damsels_killed_total, expected_credits);
    }
}
