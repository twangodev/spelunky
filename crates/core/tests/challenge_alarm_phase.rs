use spelunky_core::{Game, InputFrame, PlayerState, RulesPreset, RunState, Vec2, hub::HubKind};

// These are source-authored return spawns, plus persistent unlock data. Up
// enters the real door and the real Player exit animation requests the room.
// Calling load_hub afterward is the existing host destination adapter boundary.
// No portal test moves the player, creates a fake door, or forces a score.
fn unlocked_portal(seed: u64, rules: RulesPreset, kind: HubKind) -> Game {
    let (start, unlock, door, destination, room) = match kind {
        HubKind::Sun => (1, "unlock_sun", "oXSun", "sun", "rSun"),
        HubKind::Stars => (3, "unlock_stars", "oXStars", "stars", "rStars"),
        _ => unreachable!(),
    };
    let mut game = Game::shell(seed, rules);
    game.flags
        .extend(spelunky_core::hub::record_unlocks(200_000, None, 120, 0));
    assert!(game.flags.contains(unlock));
    game.flags.insert(format!("scores_start:{start}"));
    game.load_hub(HubKind::Scores);
    assert!(
        game.world
            .entities
            .iter()
            .any(|entity| entity.object == door)
    );
    for _ in 0..5 {
        game.step(&InputFrame::default());
    }
    assert!(game.player.grounded);
    game.step(&InputFrame {
        up: true,
        ..Default::default()
    });
    assert!(game.player.is_exiting());
    for _ in 0..80 {
        if game.state == RunState::Transition {
            break;
        }
        game.step(&InputFrame::default());
    }
    assert_eq!(game.state, RunState::Transition);
    assert!(
        game.flags
            .contains(&format!("hub_destination:{destination}"))
    );
    game.load_hub(kind);
    assert_eq!(game.world.name, room);
    assert_eq!(game.level_tick, 0);
    assert_eq!(game.player.state, PlayerState::Falling);
    game
}

fn count(game: &Game, object: &str) -> usize {
    game.world
        .entities
        .iter()
        .filter(|entity| entity.alive && entity.object == object)
        .count()
}

#[test]
fn unlocked_stars_first_hazard_steps_on_its_natural_alarm_birth_frame() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = unlocked_portal(83, rules, HubKind::Stars);
        for _ in 0..99 {
            game.step(&InputFrame::default());
            assert_eq!(count(&game, "oShopkeeper2"), 0);
        }
        assert_eq!(game.data["hub_alarm0"], 1.0);
        game.step(&InputFrame::default());
        let keeper = game
            .world
            .entities
            .iter()
            .find(|entity| entity.object == "oShopkeeper2")
            .unwrap();
        assert_eq!(game.level_tick, 100);
        assert_eq!(keeper.age, 0);
        assert_eq!(keeper.pos.y, 0.0);
        // Initial yVel is zero: source moveTo need not move a pixel yet, but
        // the inherited Step must already apply its first gravity/AI writes.
        assert!((keeper.vel.y - 0.6).abs() < 1e-9);
        assert_eq!(keeper.value("simulation_tick"), game.engine_time() as f64);
        assert_eq!(game.data["hub_alarm0"], 200.0);
        assert_eq!(game.data["hub_alarm11"], 10.0);
        assert_eq!(game.data["stars_kills"], 0.0);
    }
}

#[test]
fn source_valid_sun_threshold_fixture_lets_natural_counters_fire_zero_before_eleven() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (score, expected, forbidden) in [
            (49.0, "oBomb", "oVampire"),
            (79.0, "oVampire", "oGhost"),
        ] {
            let mut game = unlocked_portal(83, rules, HubKind::Sun);
            // Bounded legal pre-callback fixture, not an earned survival score
            // or a claimed natural RNG/input trace to points49/79. From here
            // the ordinary Game.step scheduler alone decrements both counters.
            game.data.insert("points".into(), score);
            game.data.insert("hub_alarm0".into(), 2.0);
            game.data.insert("hub_alarm11".into(), 2.0);
            game.step(&InputFrame::default());
            assert_eq!(game.data["points"], score);
            assert_eq!(count(&game, expected), 0);
            assert_eq!(game.data["hub_alarm0"], 1.0);
            assert_eq!(game.data["hub_alarm11"], 1.0);
            game.step(&InputFrame::default());
            assert_eq!(game.data["points"], score + 1.0);
            assert_eq!(count(&game, expected), 1);
            assert_eq!(count(&game, forbidden), 0);
            assert_eq!(game.data["hub_alarm11"], 30.0);
            assert!((100.0..=200.0).contains(&game.data["hub_alarm0"]));
        }
    }
}

#[test]
fn sun_birth_uses_real_bomb_alarms_and_later_alarm_two_runs_in_the_same_frame() {
    let mut game = unlocked_portal(83, RulesPreset::Classic, HubKind::Sun);
    // Only the pending source hazard timer is shortened; score starts at zero.
    game.data.insert("hub_alarm0".into(), 1.0);
    game.step(&InputFrame::default());
    let bomb = game
        .world
        .entities
        .iter()
        .find(|entity| entity.object == "oBomb")
        .unwrap();
    assert_eq!(bomb.alarms[0], 80);
    assert_eq!(bomb.alarms[1], -1);
    // oBomb454 has passed in Alarm0 before Sun721 creates it. Alarm2's
    // subsequent resource pass must nevertheless see the new instance.
    assert_eq!(bomb.alarms[2], 9);
    assert_eq!(bomb.image_speed(), 0.2);
    assert_eq!(bomb.value("armed"), 1.0);
    assert_eq!(bomb.value("safe"), 0.0);
    assert_eq!(bomb.value("safe_ticks"), 0.0);
    assert!((bomb.vel.y - 0.6).abs() < 1e-9);
    assert_eq!(game.data["hub_alarm11"], 99.0);
}

#[test]
fn existing_bomb_resource_precedes_sun_and_new_bomb_waits_for_its_next_zero_pass() {
    let mut game = unlocked_portal(83, RulesPreset::Classic, HubKind::Sun);
    // A controlled existing source-valid actor isolates resource ordering.
    // This test does not claim that the actor was produced by prior inputs.
    let existing = game.spawn("oBomb", Vec2::new(200.0, 80.0));
    let bomb = game.world.entity_mut(existing).unwrap();
    bomb.data.insert("armed".into(), 1.0);
    bomb.alarms[0] = 1;
    bomb.set_image_speed(0.2);
    game.data.insert("hub_alarm0".into(), 1.0);
    game.step(&InputFrame::default());
    let old = game.world.entity(existing).unwrap();
    assert_eq!(old.alarms[0], 0);
    assert_eq!(old.alarms[1], 39);
    assert_eq!(old.image_speed(), 1.0);
    let new = game
        .world
        .entities
        .iter()
        .find(|entity| entity.object == "oBomb" && entity.id != existing)
        .unwrap();
    assert_eq!(new.alarms[0], 80);
    assert_eq!(new.alarms[1], -1);
    assert_eq!(new.alarms[2], 9);
}

#[test]
fn persisted_stars_counters_resume_before_the_first_natural_spawn() {
    let mut original = unlocked_portal(83, RulesPreset::Classic, HubKind::Stars);
    for _ in 0..43 {
        original.step(&InputFrame::default());
    }
    assert_eq!(original.data["hub_alarm0"], 57.0);
    let mut resumed: Game =
        serde_json::from_slice(&serde_json::to_vec(&original).unwrap()).unwrap();
    assert_eq!(original.state_hash(), resumed.state_hash());
    for _ in 0..57 {
        original.step(&InputFrame::default());
        resumed.step(&InputFrame::default());
        assert_eq!(original.state_hash(), resumed.state_hash());
    }
    assert_eq!(count(&resumed, "oShopkeeper2"), 1);
    let keeper = resumed
        .world
        .entities
        .iter()
        .find(|entity| entity.object == "oShopkeeper2")
        .unwrap();
    assert!((keeper.vel.y - 0.6).abs() < 1e-9);
}

#[test]
fn pending_sun_score_fires_before_player_step_observes_fatal_life() {
    let mut game = unlocked_portal(83, RulesPreset::Classic, HubKind::Sun);
    // Source-valid boundary after damage wrote global life, before Player's
    // Step has set dead. The alarm predicate is dead, not global life > 0.
    game.player.health = 0;
    game.data.insert("hub_alarm11".into(), 1.0);
    game.step(&InputFrame::default());
    assert_eq!(game.state, RunState::Dead);
    assert_eq!(game.data["points"], 1.0);
    assert_eq!(game.data["mini1"], 1.0);
    assert_eq!(game.data["hub_alarm11"], 30.0);
    game.step(&InputFrame::default());
    assert_eq!(game.data["hub_alarm11"], 29.0);
    assert_eq!(game.data["points"], 1.0);
}
