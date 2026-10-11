use spelunky_core::{Game, InputFrame, RulesPreset, Vec2, level::Level};

fn room(rules: RulesPreset) -> Game {
    let mut level = Level::default();
    level.set_tile(6, 7, '@').unwrap();
    level.set_tile(34, 7, 'X').unwrap();
    for x in 0..spelunky_core::level::LEVEL_WIDTH {
        level.set_tile(x, 8, '1').unwrap();
    }
    let mut game = Game::shell(42, rules);
    game.load_custom(&level, "particle-cap-membership", "");
    game
}

#[test]
fn initial_inactive_far_bones_do_not_cap_near_direct_blood_before_real_tick() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for inactive in [false, true] {
            let mut game = room(rules);
            let mut bones = Vec::new();
            for _ in 0..32 {
                bones.push(game.spawn("oBone", Vec2::new(560.0, 400.0)));
            }
            // Initial membership fixture only. Every Bone completed its real
            // inherited/own Create before this state; no later actor mutation.
            for &id in &bones {
                game.world.entity_mut(id).unwrap().engine_active = !inactive;
            }
            let mut oracle = game.rng.clone();
            let velocity = Vec2::new(
                oracle.unit() * 4.0 - oracle.unit() * 4.0,
                -1.0 - oracle.unit() * 2.0,
            );
            let gravity = f64::from(1 + oracle.range(6)) * 0.1;
            let blood = game.spawn("oBlood", game.player.pos);
            let raw = game.world.entities.iter().find(|e| e.id == blood).unwrap();
            assert_eq!(raw.vel, velocity);
            assert_eq!(raw.value("gravity"), gravity);
            assert_eq!(raw.value("life"), 60.0);
            assert_eq!(
                game.rng.clone().next_u64(),
                oracle.clone().next_u64(),
                "full4Create draws even when cap destroys newborn"
            );
            game.step(&InputFrame::default());
            assert_eq!(
                game.world.entity(blood).is_some(),
                inactive,
                "birth-time active membership governs cap, not far position alone"
            );
            assert_eq!(game.rng.next_u64(), oracle.next_u64());
            assert!(
                bones
                    .iter()
                    .all(|&id| game.world.entity(id).is_some_and(|e| !e.engine_active))
            );
        }
    }
}

#[test]
fn natural_offscreen_deactivation_then_whip_blood_remains_a_passing_control() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = Game::shell(42, rules);
        game.world = spelunky_core::World::new(80, 40, spelunky_core::Area::Mines, 1);
        game.world.name = "rLevel".into();
        for x in 0..80 {
            game.world.set_tile(x, 10, "oBlock");
        }
        game.player.pos = Vec2::new(80.0, 152.0);
        game.player.previous_pos = game.player.pos;
        let animal = game.spawn("oSnake", Vec2::new(104.0, 144.0));
        let bones: Vec<_> = (0..32)
            .map(|_| game.spawn("oBone", Vec2::new(1200.0, 400.0)))
            .collect();
        // Ordinary controller deactivation, not an injected midrun flag.
        game.step(&InputFrame::default());
        assert!(
            bones
                .iter()
                .all(|&id| game.world.entity(id).is_some_and(|e| !e.engine_active))
        );
        game.step(&InputFrame {
            right: true,
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        for _ in 0..30 {
            if game.world.entity(animal).is_none() {
                break;
            }
            game.step(&InputFrame {
                right: true,
                ..Default::default()
            });
        }
        assert!(
            game.world.entity(animal).is_none(),
            "ordinary whip must kill source snake"
        );
        assert!(
            game.world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == "oBlood")
                .count()
                >= 3
        );
        assert!(
            bones
                .iter()
                .all(|&id| game.world.entity(id).is_some_and(|e| !e.engine_active))
        );
        // This control is expected to pass before and after: normal dispatcher
        // phases already partition inactive storage away from callback queries.
    }
}
