//! Default source repeater births and neutral-input first-volley timing.
//! The safe authored component map keeps the Hero below both firing lanes.
//! No generated campaign, complete Sun constructor chronology, or native replay claim.
use spelunky_core::{
    Area, Game, GameEvent, InputFrame, PlayerState, RngAlgorithm, RulesPreset, Vec2, World,
};

struct Tape {
    algorithm: RngAlgorithm,
    state: u64,
}
impl Tape {
    // Literal GM8 LCG and signed multiply/floor; no production RNG helper.
    fn canonical_draw(&mut self) -> u32 {
        let state = (self.state as u32).wrapping_mul(134775813).wrapping_add(1);
        self.state = u64::from(state);
        state
    }
    // Versioned historical SplitMix replay contract, independently spelled out.
    fn legacy_draw(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }
    fn first_delay(&mut self) -> i64 {
        match self.algorithm {
            RngAlgorithm::Gm8Lcg32V1 => {
                // rand(100): argument0=100, omitted argument1=0, randNum=-99.
                // fired=100+floor(random(-99))+100.
                200 + (-99_i64 * i64::from(self.canonical_draw())).div_euclid(1_i64 << 32)
            }
            RngAlgorithm::LegacySplitMix64V1 => 100 + (self.legacy_draw() % 100) as i64,
        }
    }
    fn later_delay(&mut self) -> i64 {
        match self.algorithm {
            RngAlgorithm::Gm8Lcg32V1 => {
                100 + ((u64::from(self.canonical_draw()) * 101) >> 32) as i64
            }
            RngAlgorithm::LegacySplitMix64V1 => 100 + (self.legacy_draw() % 101) as i64,
        }
    }
}

fn sounds(game: &Game) -> usize {
    game.events
        .iter()
        .filter(|event| {
            matches!(event,
                GameEvent::Sound { name, .. } if name == "sndArrowTrap"
            )
        })
        .count()
}

fn scene(
    rules: RulesPreset,
    algorithm: RngAlgorithm,
    seed: u32,
    object: &str,
) -> (Game, u64, Tape) {
    let mut game = Game::shell_with_algorithm(u64::from(seed), rules, algorithm).unwrap();
    game.world = World::new(24, 20, Area::Mines, 1);
    for x in 0..game.world.width {
        game.world.set_tile(x, 10, "oHardBlock");
    }
    game.player.pos = Vec2::new(64.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.world.entrance = game.player.pos;
    let tape = Tape {
        algorithm,
        state: game.rng.state_bits(),
    };
    assert_eq!(
        tape.state,
        u64::from(seed),
        "fresh constructor has not drawn"
    );
    let x = if object == "oArrowRepeaterL" {
        176.0
    } else {
        96.0
    };
    let trap = game.spawn(object, Vec2::new(x, 32.0));
    let actor = game
        .world
        .entity(trap)
        .expect("default repeater Create stays alive");
    assert_eq!(actor.object, object);
    assert!(actor.solid);
    assert_eq!(actor.vel, Vec2::default());
    assert_eq!(actor.value("invincible"), 0.0);
    assert!(game.world.entities.iter().all(|e| e.object != "oArrow"));
    (game, trap, tape)
}

fn check(rules: RulesPreset, algorithm: RngAlgorithm, seed: u32, object: &str) {
    let (mut game, trap, mut tape) = scene(rules, algorithm, seed, object);
    let delay = tape.first_delay();
    assert!((100..=200).contains(&delay));
    let key = format!("hazard_entity:{trap}:fired");
    let created_delay = game.data[&key];
    assert_eq!(
        game.rng.state_bits(),
        tape.state,
        "Create consumes precisely one draw"
    );
    for tick in 1..=delay {
        game.step(&InputFrame::default());
        assert_eq!(
            game.player.health, 4.0,
            "Hero remains safe below the arrow lane"
        );
        assert_eq!(game.player.pos, Vec2::new(64.0, 152.0));
        assert_eq!(game.player.vel, Vec2::default());
        assert!(game.player.grounded);
        assert!(
            game.world
                .entity(trap)
                .is_some_and(|e| e.alive && e.engine_active)
        );
        assert_eq!(
            sounds(&game),
            0,
            "no first volley before the literal source countdown expires"
        );
        assert!(game.world.entities.iter().all(|e| e.object != "oArrow"));
        assert_eq!(
            game.rng.state_bits(),
            tape.state,
            "quiet prefix adds no random draws"
        );
        // Source-sensitive assertions follow a real neutral input and all setup guards.
        assert_eq!(
            created_delay, delay as f64,
            "literal omitted-argument initial delay"
        );
        assert_eq!(game.data[&key], (delay - tick) as f64);
    }
    let next_delay = tape.later_delay();
    game.step(&InputFrame::default());
    assert_eq!(game.player.health, 4.0);
    assert_eq!(game.player.pos, Vec2::new(64.0, 152.0));
    assert_eq!(
        sounds(&game),
        1,
        "first volley fires one tick after fired reaches zero"
    );
    assert_eq!(
        game.data[&key], next_delay as f64,
        "later rand(100,200) remains unchanged"
    );
    assert_eq!(
        game.rng.state_bits(),
        tape.state,
        "Arrow Create has no RNG; reset consumes one draw"
    );
    let arrows: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oArrow")
        .collect();
    assert_eq!(arrows.len(), 1, "actual default arrow born by the repeater");
    assert_eq!(
        arrows[0].vel.x,
        if object == "oArrowRepeaterL" {
            -8.0
        } else {
            8.0
        }
    );
    assert_eq!(
        arrows[0].pos,
        Vec2::new(
            if object == "oArrowRepeaterL" {
                174.0
            } else {
                114.0
            },
            36.0
        ),
        "Arrow resource465 precedes repeaters723/724: newborn own Step waits until next frame"
    );
}

#[test]
fn canonical_default_repeaters_use_signed_initial_rand_and_source_first_volley() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for seed in [0, 42] {
            for object in ["oArrowRepeaterL", "oArrowRepeaterR"] {
                check(rules, RngAlgorithm::Gm8Lcg32V1, seed, object);
            }
        }
    }
}

#[test]
fn historical_default_repeater_replay_delay_and_later_reset_are_preserved() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for object in ["oArrowRepeaterL", "oArrowRepeaterR"] {
            check(rules, RngAlgorithm::LegacySplitMix64V1, 42, object);
        }
    }
}
