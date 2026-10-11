//! Actual source Alarm callbacks that invalidate transient entity indices.
//! These preserve source resource order and callback boundaries through Game::step.
use spelunky_core::*;

fn room() -> Game {
    let mut game = Game::shell(123, RulesPreset::Classic);
    game.world = World::new(40, 30, Area::Mines, 1);
    for x in 0..game.world.width {
        game.world.set_tile(x, 10, "oBlock");
    }
    game.player.pos = Vec2::new(240.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.grounded = true;
    game.player.state = PlayerState::Standing;
    game
}

fn rng_checkpoint(game: &Game) -> u64 {
    game.rng.clone().next_u64()
}

fn step(game: &mut Game) {
    game.step(&InputFrame::default());
}

fn assert_room_create_music(game: &Game) {
    // oLevel Create calls startMusic when enabled: ordinary Mines selects
    // playMusic(musCave, true), then SS_SetSoundFreq(musCave, 44100).
    assert_eq!(
        game.events,
        vec![
            GameEvent::MusicPlay {
                name: "musCave".into(),
                looping: true,
                if_idle: true
            },
            GameEvent::MusicFrequency {
                name: "musCave".into(),
                hz: 44_100
            },
        ]
    );
}

#[test]
fn drowning_spider_cleanup_does_not_reuse_later_spiders_old_storage_index() {
    let mut game = room();
    let first = game.spawn("oSpider", Vec2::new(48.0, 80.0));
    let second = game.spawn("oSpider", Vec2::new(112.0, 80.0));
    for (id, countdown) in [(first, 1), (second, 3)] {
        let spider = game.world.entity_mut(id).unwrap();
        spider.state = 4;
        spider.alarms[1] = countdown;
    }
    let old_index = game
        .world
        .entities
        .iter()
        .position(|e| e.id == second)
        .unwrap();
    let rng_before = rng_checkpoint(&game);

    step(&mut game);
    assert!(game.world.entity(first).is_none());
    assert_eq!(game.world.entity(second).unwrap().alarms[1], 2);
    let new_index = game
        .world
        .entities
        .iter()
        .position(|e| e.id == second)
        .unwrap();
    assert!(
        new_index < old_index,
        "cleanup really moved the later actor"
    );
    assert_room_create_music(&game);

    step(&mut game);
    assert_eq!(game.world.entity(second).unwrap().alarms[1], 1);
    assert!(game.events.is_empty());
    step(&mut game);
    assert!(game.world.entity(second).is_none());
    assert!(game.events.is_empty());
    assert_eq!(rng_checkpoint(&game), rng_before);
}

#[test]
fn laser_alarm_callbacks_preserve_later_actors_after_removal_reinsertion_and_births() {
    let mut game = room();
    let first = game.spawn("oLaser", Vec2::new(64.0, 64.0));
    let second = game.spawn("oLaser", Vec2::new(128.0, 64.0));
    game.world.entity_mut(first).unwrap().alarms[1] = 7;
    game.world.entity_mut(second).unwrap().alarms[1] = 11;
    let rng_before = rng_checkpoint(&game);

    // The real enemy adapter temporarily removes each laser during Alarm0.
    // Each callback appends a higher-resource LaserTrail535 and rearms itself.
    for (tick, expected_later_slots) in [(1, [6, 10]), (2, [5, 9])] {
        step(&mut game);
        for (id, later_slot) in [
            (first, expected_later_slots[0]),
            (second, expected_later_slots[1]),
        ] {
            let laser = game.world.entity(id).unwrap();
            assert_eq!(laser.alarms[0], 1);
            assert_eq!(laser.alarms[1], later_slot);
        }
        let trails: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|e| e.alive && e.object == "oLaserTrail")
            .collect();
        assert_eq!(trails.len(), tick * 2);
        assert_eq!(
            trails.iter().map(|e| e.pos.x).collect::<Vec<_>>(),
            [64.0, 128.0].repeat(tick)
        );
        assert!(trails.windows(2).all(|pair| pair[0].id < pair[1].id));
        if tick == 1 {
            assert_room_create_music(&game);
        } else {
            assert!(game.events.is_empty());
        }
        assert_eq!(rng_checkpoint(&game), rng_before);
    }
}

#[test]
fn earlier_alarm_index_rearms_pending_later_index_before_same_frame_decrement() {
    let mut game = room();
    let id = game.spawn("oBomb", Vec2::new(88.0, 152.0));
    let bomb = game.world.entity_mut(id).unwrap();
    items::explosives::arm_bomb(bomb);
    bomb.alarms[0] = 1;
    bomb.alarms[1] = 2;
    let rng_before = rng_checkpoint(&game);

    step(&mut game);
    let bomb = game.world.entity(id).unwrap();
    assert_eq!(bomb.alarms[0], 0);
    assert_eq!(
        bomb.alarms[1], 39,
        "Alarm0 writes40 before Alarm1 visits and decrements the new value"
    );
    assert_eq!(bomb.image_speed(), 1.0);
    assert!(!game.world.entities.iter().any(|e| e.object == "oExplosion"));
    assert_room_create_music(&game);
    assert_eq!(rng_checkpoint(&game), rng_before);

    step(&mut game);
    let bomb = game.world.entity(id).unwrap();
    assert_eq!(bomb.alarms[0], -1);
    assert_eq!(bomb.alarms[1], 38);
    assert!(game.events.is_empty());
    assert_eq!(rng_checkpoint(&game), rng_before);
}

#[test]
fn alarm_callback_newborn_enters_later_resource_before_normal_step() {
    let mut game = room();
    let trap = game.spawn("oArrowTrapRight", Vec2::new(64.0, 144.0));
    let trap_entity = game.world.entity_mut(trap).unwrap();
    trap_entity.alarms[0] = 1;
    trap_entity.alarms[1] = -1;
    let rng_before = rng_checkpoint(&game);

    // Trap463 Alarm0 emits Arrow465 at x+16. Its higher-resource Normal Step
    // adds xVel5 this frame. This proves registration/birth delivery, not a
    // newborn same-index Alarm callback: Arrow Create leaves Alarm0 unset.
    step(&mut game);
    let arrows: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oArrow")
        .collect();
    assert_eq!(arrows.len(), 1);
    let arrow = arrows[0];
    assert_eq!(arrow.pos.x, 85.0);
    assert_eq!(arrow.vel.x, 5.0);
    assert_eq!(arrow.value("armed"), 1.0);
    assert_eq!(game.world.entity(trap).unwrap().alarms[0], 0);
    assert!(
        game.dispatcher.resource_order[&catalog::object("oArrow").unwrap().id]
            .contains(&dispatcher::Instance::Entity(arrow.id))
    );
    assert_room_create_music(&game);
    assert_eq!(rng_checkpoint(&game), rng_before);
}

#[test]
fn duplicate_id_tombstone_retains_first_alarm_slot_and_separate_live_lookup() {
    // World::spawn guarantees unique IDs. This explicitly malformed public
    // state checks compatibility with the existing deserialized-state behavior.
    for first_alive in [false, true] {
        let mut game = room();
        let id = game.spawn("oBlood", Vec2::new(64.0, 80.0));
        game.world.entity_mut(id).unwrap().alarms = [-1; 12];
        step(&mut game); // Register the unique identity before adding a duplicate.

        let blood = game.world.entity_mut(id).unwrap();
        blood.alarms = [-1; 12];
        blood.alarms[2] = 9;
        blood.data.insert("collectible".into(), 0.0);
        let mut first = blood.clone();
        first.alive = first_alive;
        first.alarms[2] = 1;
        game.world.entities.insert(0, first);
        let rng_before = rng_checkpoint(&game);

        step(&mut game);
        let copies: Vec<_> = game.world.entities.iter().filter(|e| e.id == id).collect();
        assert_eq!(copies.len(), 2);
        assert!(copies.iter().all(|e| e.alive));
        assert_eq!(
            copies[0].value("collectible"),
            1.0,
            "first matching alarm slot fires while the callback looks up a live copy"
        );
        assert_eq!(copies[1].value("collectible"), 0.0);
        assert_eq!(copies[0].alarms[2], if first_alive { 0 } else { 9 });
        assert_eq!(copies[1].alarms[2], 9);
        assert!(game.events.is_empty());
        assert_eq!(rng_checkpoint(&game), rng_before);
    }
}

#[test]
fn duplicate_id_inactive_tombstone_is_excluded_from_visible_alarm_lookup() {
    let mut game = room();
    let id = game.spawn("oBlood", Vec2::new(64.0, 80.0));
    game.world.entity_mut(id).unwrap().alarms = [-1; 12];
    step(&mut game);
    let blood = game.world.entity_mut(id).unwrap();
    blood.alarms[2] = 9;
    blood.data.insert("collectible".into(), 0.0);
    let mut hidden = blood.clone();
    hidden.alive = false;
    hidden.engine_active = false;
    hidden.alarms[2] = 1;
    game.world.entities.insert(0, hidden);
    let rng_before = rng_checkpoint(&game);

    step(&mut game);
    let live = game
        .world
        .entities
        .iter()
        .find(|e| e.id == id && e.alive)
        .unwrap();
    assert_eq!(live.alarms[2], 8);
    assert_eq!(
        live.value("collectible"),
        0.0,
        "the hidden tombstone's due alarm is outside the visible vector"
    );
    assert!(game.events.is_empty());
    assert_eq!(rng_checkpoint(&game), rng_before);
}

#[test]
fn multi_actor_resource_preserves_duplicate_tombstone_and_later_distinct_alarm() {
    // Keep the malformed duplicate compatibility check in a resource with two
    // distinct source identities, independently of the singleton lookup path.
    let mut game = room();
    let id = game.spawn("oBlood", Vec2::new(64.0, 80.0));
    game.world.entity_mut(id).unwrap().alarms = [-1; 12];
    step(&mut game);
    let later_id = game.spawn("oBlood", Vec2::new(128.0, 80.0));
    let later = game.world.entity_mut(later_id).unwrap();
    later.alarms = [-1; 12];
    later.alarms[2] = 4;

    let blood = game.world.entity_mut(id).unwrap();
    blood.alarms = [-1; 12];
    blood.alarms[2] = 9;
    blood.data.insert("collectible".into(), 0.0);
    let mut tombstone = blood.clone();
    tombstone.alive = false;
    tombstone.alarms[2] = 1;
    game.world.entities.insert(0, tombstone);
    let resource = catalog::object("oBlood").unwrap().id;
    assert_eq!(
        game.dispatcher.resource_order[&resource],
        [
            dispatcher::Instance::Entity(id),
            dispatcher::Instance::Entity(later_id)
        ]
    );
    let rng_before = rng_checkpoint(&game);

    step(&mut game);
    let copies: Vec<_> = game.world.entities.iter().filter(|e| e.id == id).collect();
    assert_eq!(copies.len(), 2);
    assert!(copies.iter().all(|e| e.alive));
    assert_eq!(copies[0].value("collectible"), 1.0);
    assert_eq!(copies[1].value("collectible"), 0.0);
    assert_eq!(copies[0].alarms[2], 9);
    assert_eq!(copies[1].alarms[2], 9);
    let later = game.world.entity(later_id).unwrap();
    assert_eq!(
        later.alarms[2], 3,
        "the callback invalidates cached indices before the later distinct actor"
    );
    assert_eq!(later.value("collectible"), 0.0);
    assert!(game.events.is_empty());
    assert_eq!(rng_checkpoint(&game), rng_before);
}
