//! Controlled initial imported metadata exercises the literal WHOA Bow branch.
//! This is not a natural acquisition claim: only the initial imported state is
//! authored; the consequence runs through one public Game.step with no input.
use spelunky_core::{Area, Game, GameEvent, InputFrame, PlayerState, RulesPreset, Vec2, World};
const ARMED: [f64; 7] = [-1.0, 0.0, 0.25, 0.499, 0.5, 0.75, 1.0];
fn initial(preset: RulesPreset, facing: i8, object: &str, armed: f64) -> (Game, u64) {
    let mut game = Game::shell(17, preset);
    game.world = World::new(42, 20, Area::Mines, 1);
    game.world.name = "rLevel".into();
    for x in 20..42 {
        game.world.set_tile(x, 10, "oHardBlock");
    }
    game.player.pos = Vec2::new(321.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.player.facing = facing;
    game.player.set_sprite("sWhoaLeft");
    game.player.data.insert("whoaTimer".into(), 0.0);
    game.player.data.insert("whoaTimerMax".into(), 30.0);
    game.player.data.insert("bow_armed".into(), armed);
    game.player.data.insert("bow_strength".into(), 3.0);
    game.player.bombs = 4.0;
    game.player.ropes = 4.0;
    game.data.insert("arrows".into(), 7.0);
    game.world.entrance = game.player.pos;
    let id = game.spawn(object, Vec2::new(321.0, 154.0));
    let item = game.world.entity_mut(id).unwrap();
    item.held = true;
    item.text.insert("type".into(), "Bow".into());
    item.data.insert("cost".into(), 0.0);
    game.player.held = Some(id);
    game.player.pickup_item_type = "Bow".into();
    (game, id)
}
fn sounds(game: &Game, name: &str) -> usize {
    game.events
        .iter()
        .filter(|event| matches!(event, GameEvent::Sound { name: actual, .. } if actual == name))
        .count()
}
#[test]
fn imported_bow_type_uses_literal_boolean_threshold_for_whoa_fire_and_release() {
    for preset in [RulesPreset::Classic, RulesPreset::Assisted] {
        for facing in [-1, 1] {
            for object in ["oBow", "oRock"] {
                for armed in ARMED {
                    let (mut game, held) = initial(preset, facing, object, armed);
                    let next = game.world.next_id;
                    game.step(&InputFrame::default());
                    let should_fire = armed >= 0.5;
                    // Primary source consequence precedes later metadata checks.
                    assert_eq!(
                        game.data["arrows"],
                        if should_fire { 6.0 } else { 7.0 },
                        "object={object}, armed={armed}, facing={facing}, preset={preset:?}"
                    );
                    let arrows: Vec<_> = game
                        .world
                        .entities
                        .iter()
                        .filter(|e| e.alive && e.object == "oArrow")
                        .collect();
                    assert_eq!(
                        arrows.len(),
                        usize::from(should_fire),
                        "WHOAsource condition uses type and >=0.5"
                    );
                    assert!(
                        game.player.held.is_none(),
                        "matching imported Bow type releases the existing actor"
                    );
                    assert!(game.player.pickup_item_type.is_empty());
                    assert!(!game.world.entity(held).unwrap().held);
                    assert_eq!(sounds(&game, "sndArrowTrap"), usize::from(should_fire));
                    assert_eq!(sounds(&game, "sndThrow"), 0);
                    assert_eq!((game.player.bombs, game.player.ropes), (4.0, 4.0));
                    if should_fire {
                        let arrow = arrows[0];
                        assert_eq!(
                            arrow.id, next,
                            "same-type release creates no replacement before Arrow Create"
                        );
                        assert_eq!(
                            arrow.previous_pos,
                            Vec2::new(321.0 + f64::from(facing) * 14.0, 152.0)
                        );
                        assert_eq!(arrow.vel.x, f64::from(facing) * 4.0);
                        assert_eq!(arrow.value("safe"), 1.0);
                        assert_eq!(arrow.alarms[2], 10);
                        assert_eq!(arrow.value("owner_player"), 1.0);
                        assert_eq!(arrow.value("thrown_by_player"), 1.0);
                        assert_eq!(arrow.sprite, "sArrowRight");
                        assert_eq!(
                            game.player.data["weapon_cooldown"], 9.0,
                            "WHOA writes10 then source earlyFIRING decrements once"
                        );
                        assert_eq!(game.player.data["bow_armed"], 0.0);
                        assert_eq!(game.player.data["bow_strength"], 0.0);
                    } else {
                        assert_eq!(
                            game.world.next_id, next,
                            "false WHOA boolean creates no Arrow or replacement"
                        );
                        assert_eq!(
                            game.player
                                .data
                                .get("weapon_cooldown")
                                .copied()
                                .unwrap_or(0.0),
                            0.0
                        );
                    }
                }
            }
        }
    }
}
