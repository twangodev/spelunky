use spelunky_core::{Area, Game, InputFrame, PlayerState, RulesPreset, Vec2, World};

fn room(seed: u32) -> Game {
    let mut game = Game::shell(seed, RulesPreset::Classic);
    game.world = World::new(20, 16, Area::Temple, 13);
    game.flags.insert("custom".into());
    game.player.pos = Vec2::new(40.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    for x in 0..20 {
        game.world.set_tile(x, 10, "oBrick");
    }
    game.world.fluid_dirty = false;
    game
}

#[test]
fn actual_step_replaces_uncovered_fake_with_new_dynamic_temple() {
    let mut game = room(7);
    let origin = Vec2::new(80.0, 80.0);
    let fake = game.spawn("oTempleFake", origin);
    let fake_serial = game.world.entity(fake).unwrap().creation_serial;
    game.step(&InputFrame::default());
    assert!(game.world.entity(fake).is_none());
    let temple = game
        .world
        .entities
        .iter()
        .find(|e| e.alive && e.object == "oTemple" && e.pos == origin)
        .unwrap();
    assert_ne!(
        temple.id, fake,
        "source creates a new identity, not instance_change"
    );
    assert!(temple.creation_serial > fake_serial);
    assert_eq!(
        temple.value("runtime_temple_created"),
        1.0,
        "Create completed synchronously"
    );
    assert!(game.world.solid_at(88.0, 88.0));
    assert!(
        !game.world.fluid_dirty,
        "Fake has no parent/Destroy; Temple Create requests no water"
    );
    let id = temple.id;
    game.step(&InputFrame::default());
    assert!(game.world.entity(id).is_some());
    assert_eq!(
        game.world
            .entities
            .iter()
            .filter(|e| e.alive && e.object == "oTemple" && e.pos == origin)
            .count(),
        1
    );
}

#[test]
fn actual_falling_door_exposes_both_source_fake_cells() {
    let mut game = room(7);
    let door = game.spawn("oDoor", Vec2::new(80.0, 64.0));
    let top = game.spawn("oTempleFake", Vec2::new(80.0, 64.0));
    let bottom = game.spawn("oTempleFake", Vec2::new(80.0, 80.0));
    game.step(&InputFrame::default());
    assert!(
        game.world.entity(top).is_some() && game.world.entity(bottom).is_some(),
        "idle source door masks cover both fake centers"
    );
    game.world.entity_mut(door).unwrap().state = 1; // source DROP state
    for _ in 0..30 {
        game.step(&InputFrame::default());
    }
    assert!(game.world.entity(top).is_none() && game.world.entity(bottom).is_none());
    for y in [64.0, 80.0] {
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == "oTemple" && e.pos == Vec2::new(80.0, y))
                .count(),
            1
        );
    }
}
