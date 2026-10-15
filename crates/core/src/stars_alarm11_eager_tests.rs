use super::*;

fn stars(seed: u32, rules: RulesPreset) -> Game {
    let mut game = Game::shell(seed, rules);
    assert_eq!(
        game.generation_revision,
        GenerationRevision::SourceClassicV2
    );
    game.load_hub(HubKind::Stars);
    assert_eq!(game.world.name, "rStars");
    assert_eq!(game.data["stars_kills"], 0.0);
    assert_eq!(game.data["hub_alarm11"], 110.0);
    game
}
fn count(game: &Game, object: &str) -> usize {
    game.world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == object)
        .count()
}
fn raw_lcg_step(state: u64) -> u64 {
    // GM8 random's independent state transition. Bounds do not change the burn.
    u64::from((state as u32).wrapping_mul(134_775_813).wrapping_add(1))
}
fn dead_alarm_fixture(game: &mut Game) {
    // Explicit initial callback boundary, not a claimed earned challenge death.
    game.player.health = 0;
    game.player.state = PlayerState::Dead;
    game.state = RunState::Dead;
    game.data.insert("stars_kills".into(), 99.0);
    game.data.insert("hub_alarm11".into(), 1.0);
}

#[test]
fn source_dead_stars_alarm11_burns_one_roll_without_birth_or_reschedule() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = stars(83, rules);
        dead_alarm_fixture(&mut game);
        let expected = raw_lcg_step(game.rng.state_bits());
        let entities = game.world.entities.len();
        let events = game.events.clone();
        challenge_alarm(&mut game, 11);
        assert_eq!(
            game.rng.state_bits(),
            expected,
            "Source eager AND must evaluate rand when dead"
        );
        assert_eq!(game.data["hub_alarm11"], 0.0);
        assert_eq!(game.data["stars_kills"], 99.0);
        assert_eq!(game.world.entities.len(), entities);
        assert_eq!(count(&game, "oSpider"), 0);
        assert_eq!(game.events, events);
        let after = game.rng.state_bits();
        challenge_alarm(&mut game, 11);
        assert_eq!(game.data["hub_alarm11"], -1.0);
        assert_eq!(
            game.rng.state_bits(),
            after,
            "expired alarm must not repeat"
        );
    }
}

#[test]
fn source_alive_stars_alarm11_keeps_its_roll_and_twenty_phase_schedule() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = stars(83, rules);
        game.data.insert("hub_alarm11".into(), 1.0);
        let expected = raw_lcg_step(game.rng.state_bits());
        challenge_alarm(&mut game, 11);
        assert_eq!(game.rng.state_bits(), expected);
        assert_eq!(game.data["hub_alarm11"], 20.0);
        assert_eq!(count(&game, "oSpider"), 0, "kills0 forbids the birth");
        for pending in (1..20).rev() {
            challenge_alarm(&mut game, 11);
            assert_eq!(game.data["hub_alarm11"], f64::from(pending));
            assert_eq!(game.rng.state_bits(), expected);
        }
        challenge_alarm(&mut game, 11);
        assert_eq!(game.rng.state_bits(), raw_lcg_step(expected));
        assert_eq!(game.data["hub_alarm11"], 20.0);
    }
}

#[test]
fn historical_dead_stars_and_dead_sun_preserve_their_no_roll_paths() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for algorithm in [RngAlgorithm::Gm8Lcg32V1, RngAlgorithm::LegacySplitMix64V1] {
            let mut game = Game::shell_with_algorithm(83, rules, algorithm).unwrap();
            game.generation_revision = GenerationRevision::HistoricalV1;
            game.load_hub(HubKind::Stars);
            dead_alarm_fixture(&mut game);
            let before = game.rng.state_bits();
            challenge_alarm(&mut game, 11);
            assert_eq!(game.rng.state_bits(), before);
            assert_eq!(game.data["hub_alarm11"], 0.0);
            assert_eq!(count(&game, "oSpider"), 0);
        }
        let mut game = Game::shell(83, rules);
        game.load_hub(HubKind::Sun);
        dead_alarm_fixture(&mut game);
        let before = game.rng.state_bits();
        let points = game.data["points"];
        challenge_alarm(&mut game, 11);
        assert_eq!(game.rng.state_bits(), before);
        assert_eq!(game.data["hub_alarm11"], 0.0);
        assert_eq!(game.data["points"], points);
    }
}

#[test]
fn natural_stars_hazard_death_leaves_a_pending_eager_alarm_callback() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = stars(83, rules);
        let mut saw_keeper = false;
        let mut saw_damage = false;
        // Ordinary authored Stars actors and inputs only. No forced actor,
        // timer, life, state, position, RNG, or inventory writes after loading.
        // This bounded producer premise remains UNRUN until Root validates it.
        for tick in 0..2400 {
            game.step(&InputFrame {
                left: tick >= 8,
                ..InputFrame::default()
            });
            saw_keeper |= count(&game, "oShopkeeper2") > 0;
            saw_damage |= game
                .events
                .iter()
                .any(|e| matches!(e, GameEvent::Damage { .. }));
            if crate::items::player_dead(&game) {
                break;
            }
        }
        assert!(
            saw_keeper && saw_damage,
            "ordinary Stars hazard must reach Player"
        );
        assert_eq!(game.state, RunState::Dead);
        assert_eq!(game.player.state, PlayerState::Dead);
        assert_eq!(game.player.health, 0);
        assert!((1.0..=20.0).contains(&game.data["hub_alarm11"]));
        while game.data["hub_alarm11"] > 1.0 {
            game.step(&InputFrame::default());
            assert_eq!(game.state, RunState::Dead);
        }
        assert_eq!(game.data["hub_alarm11"], 1.0);
        assert!(game.flags.contains("hub_stars"));
        // After the ordinary producer prefix, isolate this exact scheduled
        // callback from unrelated enemy Step RNG. This is not a whole-frame
        // one-draw oracle. The real scheduler's zero timer is checked separately.
        let mut callback = game.clone();
        let expected = raw_lcg_step(callback.rng.state_bits());
        let spiders = count(&callback, "oSpider");
        challenge_alarm(&mut callback, 11);
        assert_eq!(
            callback.rng.state_bits(),
            expected,
            "natural pending dead callback still burns rand100"
        );
        assert_eq!(callback.data["hub_alarm11"], 0.0);
        assert_eq!(count(&callback, "oSpider"), spiders);
        game.step(&InputFrame::default());
        assert_eq!(game.data["hub_alarm11"], 0.0);
        assert_eq!(game.state, RunState::Dead);
    }
}
