use crate::{Area, Game, GameEvent, InputFrame, PlayerState, RulesPreset, Vec2, World};

fn room(area: Area) -> Game {
    let mut game = Game::shell(37, RulesPreset::Classic);
    game.world.area = area;
    game.player.pos = Vec2::new(80.0, 80.0);
    game
}

fn idol_pickup(game: &mut Game) {
    let idol = game.spawn("oGoldIdol", game.player.pos);
    crate::traps::altar::idol_pickup(game, idol);
}

#[test]
fn mines_idol_alarm_chooses_nearest_active_tiki_head() {
    let mut game = room(Area::Mines);
    let inactive = game.spawn("oGiantTikiHead", Vec2::new(80.0, 16.0));
    let active = game.spawn("oGiantTikiHead", Vec2::new(144.0, 16.0));
    crate::dispatcher::register_instances(&mut game);
    game.world.entity_mut(inactive).unwrap().engine_active = false;

    idol_pickup(&mut game);

    assert_eq!(game.world.entity(active).unwrap().alarms[0], 100);
    assert_ne!(game.world.entity(inactive).unwrap().alarms[0], 100);
}

#[test]
fn game_step_mines_idol_pickup_runs_full_100_frame_tiki_alarm() {
    let mut game = Game::shell(37, RulesPreset::Classic);
    game.world = World::new(20, 20, Area::Mines, 1);
    game.world.name = "Mines".into();
    for x in 0..game.world.width {
        game.world.set_tile(x, 10, "oHardBlock");
    }
    game.player.pos = Vec2::new(80.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.player.facing = 1;
    game.world.entrance = game.player.pos;

    // The source Mines generator places both as ordinary authored instances.
    let head = game.spawn("oGiantTikiHead", Vec2::new(80.0, 88.0));
    let idol = game.spawn("oGoldIdol", Vec2::new(80.0, 156.0));

    game.step(&InputFrame::default());
    assert_eq!(game.player.state, PlayerState::Standing);
    assert_eq!(game.player.held, None);
    assert_eq!(game.world.entity(head).unwrap().alarms[0], -1);

    game.step(&InputFrame {
        down: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert_eq!(game.player.held, Some(idol));
    assert_eq!(game.data.get("idols_grabbed").copied(), Some(1.0));
    assert_eq!(game.world.entity(head).unwrap().alarms[0], 100);
    assert_eq!(game.world.entity(head).unwrap().sprite, "sGiantTikiHead");
    assert!(
        !game
            .world
            .entities
            .iter()
            .any(|e| e.alive && e.object == "oBoulder")
    );

    for _ in 0..99 {
        game.step(&InputFrame::default());
    }
    assert_eq!(game.world.entity(head).unwrap().alarms[0], 1);
    assert_eq!(game.world.entity(head).unwrap().sprite, "sGiantTikiHead");
    assert!(
        !game
            .world
            .entities
            .iter()
            .any(|e| e.alive && e.object == "oBoulder")
    );

    game.step(&InputFrame::default());

    assert_eq!(game.world.entity(head).unwrap().alarms[0], 0);
    assert_eq!(game.world.entity(head).unwrap().sprite, "sGTHHole");
    let boulders: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oBoulder")
        .collect();
    assert_eq!(boulders.len(), 1);
    assert!(boulders[0].value("invincible") != 0.0);
    assert_eq!(
        game.events
            .iter()
            .filter(|event| matches!(event, GameEvent::Sound { name, .. } if name == "sndThump"))
            .count(),
        1
    );
}

#[test]
fn temple_idol_falls_back_to_nearby_blocks_when_no_active_ceiling_exists() {
    let mut game = room(Area::Temple);
    let ceiling = game.spawn("oCeilingTrap", Vec2::new(78.0, 64.0));
    let block = game.spawn("oTrapBlock", game.player.pos);
    crate::dispatcher::register_instances(&mut game);
    game.world.entity_mut(ceiling).unwrap().engine_active = false;

    idol_pickup(&mut game);

    assert!(game.world.entity(ceiling).unwrap().state == 0);
    assert!(game.world.entity(block).is_none());
}

#[test]
fn temple_idol_door_nearest_queries_ignore_inactive_doors() {
    let mut game = room(Area::Temple);
    game.spawn("oCeilingTrap", Vec2::new(240.0, 64.0));
    let active_left = game.spawn("oDoor", Vec2::new(60.0, 16.0));
    let inactive_left = game.spawn("oDoor", Vec2::new(17.0, 16.0));
    let active_right = game.spawn("oDoor", Vec2::new(144.0, 16.0));
    crate::dispatcher::register_instances(&mut game);
    game.world.entity_mut(inactive_left).unwrap().engine_active = false;

    idol_pickup(&mut game);

    assert_eq!(game.world.entity(active_left).unwrap().state, 1);
    assert_eq!(game.world.entity(active_right).unwrap().state, 1);
    assert_eq!(game.world.entity(inactive_left).unwrap().state, 0);
}
