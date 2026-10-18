//! Integrated regressions for `oPlayer1`'s held-arrow and Alarm 11 callbacks.
//!
//! Source anchors: `oPlayer1.events/Step.xml:43-55,1505-1520`,
//! `oPlayer1.events/Alarm 11.xml:16-20`, and `Scripts/Character/scrFireBow.gml:55-63`.
use crate::{Area, Game, InputFrame, PlayerState, RulesPreset, Vec2, World};

fn full_charge_bomb_arrow(fuse: f64) -> Game {
    let mut game = Game::shell(42, RulesPreset::Classic);
    game.world = World::new(20, 20, Area::Mines, 1);
    for x in 0..20 {
        game.world.set_tile(x, 10, "oBlock");
    }
    game.player.pos = Vec2::new(80.0, 152.0);
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    // The fuse expires at the player origin. Keep the authored bow draw alive
    // so the following Alarm 11 callback can test the independent holdArrow gate.
    game.player.cheat_invulnerable = true;
    let bow = game.world.spawn("oBow", game.player.pos);
    game.world.entity_mut(bow).unwrap().held = true;
    game.player.held = Some(bow);
    game.player.data.insert("bow_armed".into(), 1.0);
    game.player.data.insert("bow_strength".into(), 12.0);
    game.player.data.insert("bow_bomb_armed".into(), 1.0);
    game.player.data.insert("hold_arrow".into(), 2.0);
    game.player.data.insert("bow_bomb_fuse".into(), fuse);
    game.player.data.insert("bow_arrow_toggle".into(), 0.0);
    game.player.alarms[11] = 1;
    game
}

#[test]
fn game_step_stops_alarm_11_blink_after_full_charge_bomb_arrow_expires() {
    let mut game = full_charge_bomb_arrow(0.0);

    // Alarm 11 runs before Player Step, so it blinks once while the bomb arrow
    // still exists. Player Step then detonates and clears `holdArrow`; because
    // bowStrength is already >= 12, the later charge block does not restore it.
    game.step(&InputFrame::default());
    assert_eq!(game.player.data["bow_arrow_toggle"], 1.0);
    assert_eq!(game.player.data["bow_bomb_armed"], 0.0);
    assert_eq!(game.player.data["bow_strength"], 12.0);

    // Alarm 11 must now leave the arrow phase alone and not re-arm itself:
    // its source condition is holdArrow > 0, while bowArmed remains true.
    game.step(&InputFrame::default());
    assert_eq!(
        game.player.data["bow_arrow_toggle"], 1.0,
        "SOURCE_ALARM11_REQUIRES_HELD_ARROW_AFTER_FUSE_EXPIRY"
    );
    assert!(game.player.alarms[11] <= 0);
}

#[test]
fn game_step_fires_normal_arrow_after_full_charge_bomb_arrow_expires() {
    let mut game = full_charge_bomb_arrow(0.0);
    game.step(&InputFrame::default());
    // The detonation's normal oExplosion/oItem callback throws the held Bow
    // and clears Player.held (source Collision with oItem.xml:53-61). Restore
    // the retained-bow branch explicitly to exercise scrFireBow's projectile
    // selection after holdArrow has expired.
    let bow = game
        .world
        .entities
        .iter()
        .find(|entity| entity.alive && entity.object == "oBow")
        .unwrap()
        .id;
    game.world.entity_mut(bow).unwrap().held = true;
    game.player.held = Some(bow);
    game.player.pickup_item_type = "Bow".into();
    game.step(&InputFrame {
        attack_released: true,
        ..Default::default()
    });

    let arrows: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|entity| entity.alive && entity.object == "oArrow")
        .collect();
    assert_eq!(arrows.len(), 1, "one release creates one arrow");
    assert_eq!(arrows[0].sprite, "sArrowRight");
    assert_eq!(arrows[0].value("bomb_arrow"), 0.0);
}
