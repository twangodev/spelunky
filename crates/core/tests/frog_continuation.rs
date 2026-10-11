//! Controlled initial receiver states exercise actual Game.step dispatch.
//! These are source-branch controls, not natural death-producer witnesses.
use spelunky_core::*;

fn room(
    rules: RulesPreset,
    state: i32,
    health: i32,
    floor: bool,
    bloodless: bool,
    detritus: usize,
) -> (Game, u64) {
    let mut game = Game::shell(42, rules);
    game.world = World::new(30, 20, Area::Jungle, 5);
    game.player.pos = Vec2::new(128.0, 120.0);
    game.player.previous_pos = game.player.pos;
    if floor {
        for x in 0..30 {
            game.world.set_tile(x, 8, "oBlock");
        }
    }
    let id = game.spawn("oFrog", Vec2::new(96.0, if floor { 112.0 } else { 80.0 }));
    let frog = game.world.entity_mut(id).unwrap();
    frog.health = health;
    frog.state = state;
    frog.vel = Vec2::default();
    frog.data.insert("bloodless".into(), f64::from(bloodless));
    for _ in 0..detritus {
        game.spawn("oBlood", Vec2::new(280.0, 80.0));
    }
    (game, id)
}

// Independent arithmetic over raw RNG values. Source oBlood/Create has three
// random() calls and one inclusive rand(1,6), before the Frog state-tail draw.
fn rand(rng: &mut Rng, width: u64) -> u32 {
    let algorithm = rng.algorithm();
    let raw = rng.next_u64();
    match algorithm {
        RngAlgorithm::Gm8Lcg32V1 => ((raw * width) >> 32) as u32,
        RngAlgorithm::LegacySplitMix64V1 => (raw % width) as u32,
    }
}

fn random(rng: &mut Rng) -> f64 {
    let algorithm = rng.algorithm();
    let raw = rng.next_u64();
    match algorithm {
        RngAlgorithm::Gm8Lcg32V1 => raw as f64 / 4_294_967_296.0,
        RngAlgorithm::LegacySplitMix64V1 => (raw >> 11) as f64 / 9_007_199_254_740_992.0,
    }
}

fn blood(rng: &mut Rng) -> (Vec2, f64) {
    let velocity = Vec2::new(
        random(rng) * 4.0 - random(rng) * 4.0,
        -1.0 - random(rng) * 2.0,
    );
    (velocity, f64::from(1 + rand(rng, 6)) * 0.1)
}

fn croaks(game: &Game) -> usize {
    game.events
        .iter()
        .filter(|event| matches!(event, GameEvent::Sound { name, .. } if name == "sndFrog"))
        .count()
}

#[test]
fn actual_step_normal_frog_own_death_continues_idle_croak_after_blood_with_cap_controls() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (bloodless, initial, births) in
            [(false, 0, 3), (false, 15, 1), (false, 16, 0), (true, 0, 0)]
        {
            let (mut game, id) = room(rules, 0, 0, true, bloodless, initial);
            let first_birth = game.world.next_id;
            let mut expected = game.rng.clone();
            let expected_blood: Vec<_> = (0..births).map(|_| blood(&mut expected)).collect();

            game.step(&InputFrame::default());

            assert!(game.world.entity(id).is_none());
            assert_eq!(
                croaks(&game),
                1,
                "retired IDLE receiver still reaches BOUNCE"
            );
            let actual: Vec<_> = game
                .world
                .entities
                .iter()
                .filter(|e| e.object == "oBlood" && e.id >= first_birth)
                .collect();
            assert_eq!(actual.len(), births);
            for (actual, (velocity, gravity)) in actual.iter().zip(expected_blood) {
                assert_eq!(actual.pos, Vec2::new(104.0, 120.0));
                assert_eq!(actual.vel, velocity);
                assert_eq!(actual.value("gravity"), gravity);
            }
            assert_eq!(game.rng.clone().next_u64(), expected.clone().next_u64());

            game.step(&InputFrame::default());
            assert_eq!(
                croaks(&game),
                0,
                "Destroy never schedules another Frog Step"
            );
            assert_eq!(
                game.world
                    .entities
                    .iter()
                    .filter(|e| e.object == "oBlood" && e.id >= first_birth)
                    .count(),
                births,
                "own death's three-attempt blood hook runs once"
            );
            assert_eq!(game.rng.clone().next_u64(), expected.clone().next_u64());
        }
    }
}

#[test]
fn actual_step_normal_frog_own_death_consumes_grounded_bounce_or_recover_after_blood() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (state, width) in [(1, 3), (2, 31)] {
            for (bloodless, initial, births) in [(false, 0, 3), (true, 0, 0), (false, 16, 0)] {
                let (mut game, id) = room(rules, state, 0, true, bloodless, initial);
                let first_birth = game.world.next_id;
                let mut expected = game.rng.clone();
                for _ in 0..births {
                    blood(&mut expected);
                }
                rand(&mut expected, width);

                game.step(&InputFrame::default());

                assert!(game.world.entity(id).is_none());
                assert_eq!(croaks(&game), 0);
                assert_eq!(
                    game.world
                        .entities
                        .iter()
                        .filter(|e| e.object == "oBlood" && e.id >= first_birth)
                        .count(),
                    births
                );
                assert_eq!(game.rng.clone().next_u64(), expected.clone().next_u64());
                game.step(&InputFrame::default());
                assert_eq!(game.rng.clone().next_u64(), expected.clone().next_u64());
            }
        }
    }
}

#[test]
fn actual_step_normal_frog_airborne_recover_death_control_has_no_tail_draw() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, id) = room(rules, 2, 0, false, false, 0);
        let mut expected = game.rng.clone();
        for _ in 0..3 {
            blood(&mut expected);
        }
        game.step(&InputFrame::default());
        assert!(game.world.entity(id).is_none());
        assert_eq!(croaks(&game), 0);
        assert_eq!(game.rng.clone().next_u64(), expected.clone().next_u64());
    }
}

#[test]
fn actual_step_normal_frog_live_recover_control_keeps_the_same_grounded_draw() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, id) = room(rules, 2, 1, true, false, 0);
        let mut expected = game.rng.clone();
        let counter = 10 + rand(&mut expected, 31);
        game.step(&InputFrame::default());
        let frog = game.world.entity(id).unwrap();
        assert_eq!(frog.health, 1);
        assert_eq!(frog.state, 0);
        assert_eq!(frog.timer, counter);
        assert!(!game.world.entities.iter().any(|e| e.object == "oBlood"));
        assert_eq!(croaks(&game), 0);
        assert_eq!(game.rng.clone().next_u64(), expected.clone().next_u64());
    }
}
