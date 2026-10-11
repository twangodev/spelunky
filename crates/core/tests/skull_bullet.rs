use spelunky_core::{Game, InputFrame, RulesPreset, level::Level};

fn pistol_room(left: bool, behind: bool, rules: RulesPreset) -> (Game, u64) {
    let mut level = Level::default();
    level.set_tile(if left { 12 } else { 6 }, 7, '@').unwrap();
    level.set_tile(34, 7, 'X').unwrap();
    level.set_tile(if left { 11 } else { 7 }, 7, '-').unwrap();
    let skull_x = match (left, behind) {
        (false, false) => 11,
        (true, false) => 7,
        (false, true) => 2,
        (true, true) => 16,
    };
    level.set_tile(skull_x, 7, 'k').unwrap();
    for x in 0..spelunky_core::level::LEVEL_WIDTH {
        level.set_tile(x, 8, '1').unwrap();
    }
    let mut game = Game::shell(42, rules);
    game.load_custom(&level, "skull-bullet", "");
    let skull = game
        .world
        .entities
        .iter()
        .find(|e| e.object == "oSkull")
        .unwrap()
        .id;
    for _ in 0..8 {
        game.step(&InputFrame::default());
    }
    assert!(game.player.grounded);
    for _ in 0..4 {
        game.step(&InputFrame {
            left,
            right: !left,
            ..Default::default()
        });
    }
    game.step(&InputFrame {
        down: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    let held = game
        .player
        .held
        .expect("ordinary down+attack picks up authored pistol");
    assert_eq!(game.world.entity(held).unwrap().object, "oPistol");
    for _ in 0..2 {
        game.step(&InputFrame::default());
    }
    assert!(game.world.entity(skull).is_some());
    assert_eq!(game.player.facing, if left { -1 } else { 1 });
    (game, skull)
}

#[test]
fn natural_pistol_pickup_and_shot_breaks_skull_both_facings_and_presets() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for left in [false, true] {
            let (mut game, skull) = pistol_room(left, false, rules);
            let target = game.world.entity(skull).unwrap().clone();
            let mut observed_contact = false;
            let mut observed_bones = false;
            for tick in 0..18 {
                game.step(&InputFrame {
                    attack: tick == 0,
                    attack_pressed: tick == 0,
                    ..Default::default()
                });
                observed_contact |= game
                    .world
                    .entities
                    .iter()
                    .any(|e| e.alive && e.object == "oBullet" && e.event_intersects(&target));
                observed_bones |= game.world.entities.iter().any(|e| e.object == "oBone");
            }
            assert!(
                observed_contact,
                "actual live fired bullet must cross settled Skull precise mask, left={left}"
            );
            assert!(
                game.world.entity(skull).is_none(),
                "source Skull Bullet callback must destroy it after actual contact, left={left}"
            );
            assert!(observed_bones);
            assert_eq!(game.player.health, 4);
        }
    }
}

#[test]
fn natural_shot_away_from_skull_leaves_it_intact() {
    for left in [false, true] {
        let (mut game, skull) = pistol_room(left, true, RulesPreset::Classic);
        let mut observed_bullet = false;
        for tick in 0..18 {
            game.step(&InputFrame {
                attack: tick == 0,
                attack_pressed: tick == 0,
                ..Default::default()
            });
            let target = game
                .world
                .entity(skull)
                .expect("rear Skull is outside fired bullet path");
            observed_bullet |= game
                .world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oBullet");
            assert!(
                !game
                    .world
                    .entities
                    .iter()
                    .any(|e| e.alive && e.object == "oBullet" && e.event_intersects(target))
            );
        }
        assert!(observed_bullet);
        assert!(!game.world.entities.iter().any(|e| e.object == "oBone"));
        assert_eq!(game.player.health, 4);
    }
}
