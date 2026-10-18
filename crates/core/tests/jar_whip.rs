use spelunky_core::{Game, InputFrame, RulesPreset, level::Level};
fn room(jar_x: usize, rules: RulesPreset) -> (Game, u64) {
    let mut level = Level::default();
    level.set_tile(6, 7, '@').unwrap();
    level.set_tile(34, 7, 'X').unwrap();
    level.set_tile(jar_x, 7, 'j').unwrap();
    for x in 0..spelunky_core::level::LEVEL_WIDTH {
        level.set_tile(x, 8, '1').unwrap();
    }
    let mut game = Game::shell(42, rules);
    game.load_custom(&level, "jar-whip", "");
    let jar = game
        .world
        .entities
        .iter()
        .find(|e| e.alive && e.object == "oJar")
        .unwrap()
        .id;
    for _ in 0..8 {
        game.step(&InputFrame::default());
    }
    assert!(game.player.grounded);
    assert!(game.world.entity(jar).is_some());
    (game, jar)
}
#[test]
fn normal_forward_whip_breaks_authored_jar_both_facings_and_presets() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for left in [false, true] {
            let (mut game, jar) = room(if left { 5 } else { 7 }, rules);
            game.step(&InputFrame {
                left,
                right: !left,
                ..Default::default()
            });
            assert_eq!(game.player.facing, if left { -1 } else { 1 });
            game.step(&InputFrame {
                attack: true,
                attack_pressed: true,
                ..Default::default()
            });
            let mut saw_contact = false;
            let mut saw_forward = false;
            let mut saw_six_fragments = false;
            for _ in 0..12 {
                game.step(&InputFrame::default());
                saw_forward |= game.world.entities.iter().any(|e| e.object == "oWhip");
                if let Some(target) = game.world.entities.iter().find(|e| e.id == jar) {
                    saw_contact |= game.world.entities.iter().any(|weapon| {
                        weapon.alive && weapon.object == "oWhip" && weapon.event_intersects(target)
                    });
                }
                if game.world.entity(jar).is_none() {
                    saw_six_fragments |= game
                        .world
                        .entities
                        .iter()
                        .filter(|e| e.object == "oRubbleSmall")
                        .count()
                        == 6;
                }
            }
            assert!(saw_forward);
            assert!(
                game.world.entity(jar).is_none(),
                "ordinary forward whip must break authored jar; facing={left}, observed_mask_contact={saw_contact}"
            );
            assert!(
                saw_six_fragments,
                "literal Whip collision then own Destroy each creates three fragments"
            );
            assert_eq!(game.player.health, 4.0);
        }
    }
}
#[test]
fn rear_whip_pre_reaches_jar_without_its_forward_whip_event() {
    let (mut game, jar) = room(5, RulesPreset::Classic);
    game.step(&InputFrame {
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    let mut saw_rear_contact = false;
    for _ in 0..12 {
        game.step(&InputFrame::default());
        let target = game
            .world
            .entity(jar)
            .expect("WhipPre has no Jar collision event");
        saw_rear_contact |= game.world.entities.iter().any(|weapon| {
            weapon.alive && weapon.object == "oWhipPre" && weapon.event_intersects(target)
        });
    }
    assert!(
        saw_rear_contact,
        "control must exercise actual rear mask contact"
    );
    assert_eq!(game.player.health, 4.0);
    assert!(
        !game
            .world
            .entities
            .iter()
            .any(|e| e.object == "oRubbleSmall")
    );
}
