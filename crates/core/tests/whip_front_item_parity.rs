//! Ordinary Player Step front-whip contacts for carried supplies and loose
//! treasure. Classic has a Whip collision event for ground Jars only.
use spelunky_core::{Game, GameEvent, InputFrame, PlayerState, RulesPreset, Vec2};

fn attack_room(object: &str, facing: i8, rules: RulesPreset) -> (Game, u64) {
    let mut game = Game::shell(7, rules);
    for x in 0..game.world.width {
        game.world.set_tile(x, 4, "oBlock");
    }
    game.player.pos = Vec2::new(32.0, 56.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.player.facing = facing;
    let origin = Vec2::new(32.0 + f64::from(facing) * 18.0, 56.0);
    let target = game.spawn(object, origin);
    (game, target)
}

fn create_front_whip(game: &mut Game) -> u64 {
    game.step(&InputFrame {
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    for _ in 0..4 {
        game.step(&InputFrame::default());
        assert!(
            !game
                .world
                .entities
                .iter()
                .any(|entity| entity.alive && entity.object == "oWhip"),
            "forward Whip must not exist before the source image_index > 4 gate"
        );
    }
    game.step(&InputFrame::default());
    game.world
        .entities
        .iter()
        .find(|entity| entity.alive && entity.object == "oWhip")
        .expect("Player Step creates the forward Whip on frame six")
        .id
}

#[test]
fn real_front_whip_only_destroys_ground_jars_across_facings_and_presets() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for facing in [-1, 1] {
            for object in [
                "oBombBag",
                "oBombBox",
                "oRopePile",
                "oGoldBars",
                "oEmeraldBig",
                "oSapphireBig",
                "oRubyBig",
                "oDiamond",
                "oSkull",
            ] {
                let (mut game, target) = attack_room(object, facing, rules);
                let supplies = (game.player.bombs, game.player.ropes);
                let whip = create_front_whip(&mut game);
                let target_entity = game.world.entity(target).expect("no Whip event");
                let whip_entity = game.world.entity(whip).unwrap();
                assert!(
                    whip_entity.event_intersects(target_entity),
                    "source hitbox overlap prerequisite: {object}, {rules:?}, facing={facing}"
                );
                assert!(
                    game.world.entity(target).is_some(),
                    "no source Collision with oWhip event: {object}, {rules:?}, facing={facing}"
                );
                assert_eq!((game.player.bombs, game.player.ropes), supplies);
                assert_eq!(game.player.health, 4.0);
                assert!(!game.events.iter().any(|event| matches!(
                    event,
                    GameEvent::Collected { object: collected, .. } if collected == object
                )));
                assert!(
                    !game
                        .world
                        .entities
                        .iter()
                        .any(|entity| matches!(entity.object.as_str(), "oRubbleSmall" | "oBone")),
                    "non-Jar item must not create break debris: {object}"
                );
            }

            let (mut game, jar) = attack_room("oJar", facing, rules);
            create_front_whip(&mut game);
            assert!(game.world.entity(jar).is_none());
            assert_eq!(
                game.world
                    .entities
                    .iter()
                    .filter(|entity| entity.object == "oRubbleSmall")
                    .count(),
                6,
                "Whip event then Jar Destroy each create three rubble pieces"
            );
            assert_eq!(
                game.events
                    .iter()
                    .filter(
                        |event| matches!(event, GameEvent::Sound { name, .. } if name == "sndBreak")
                    )
                    .count(),
                2,
                "Whip collision and inherited Destroy each play Break"
            );
        }
    }
}
