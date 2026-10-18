//! Natural whip death preserves the rest of Snake Step after synchronous Destroy.
//! The literal RNG independently maps source draw order for both tagged streams.
//! Canonical arithmetic is EXE-derived; this is not original-game runtime parity.
use spelunky_core::*;

#[derive(Clone)]
struct LiteralRng(u64, bool);

impl LiteralRng {
    fn at(game: &Game) -> Self {
        let wire = serde_json::to_value(&game.rng).unwrap();
        let state = wire["state"].as_u64().unwrap();
        let canonical = match wire.get("algorithm").and_then(|value| value.as_str()) {
            None => false,
            Some("gm8_lcg32_v1") => true,
            other => panic!("unsupported literal RNG identity: {other:?}"),
        };
        assert!(!canonical || state <= u64::from(u32::MAX));
        Self(state, canonical)
    }

    fn next(&mut self) -> u64 {
        if self.1 {
            self.0 = u64::from((self.0 as u32).wrapping_mul(0x0808_8405).wrapping_add(1));
            return self.0;
        }
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut value = self.0;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
        value ^ (value >> 31)
    }

    fn range(&mut self, upper: u64) -> u64 {
        assert!((1..=u64::from(u32::MAX)).contains(&upper));
        let raw = self.next();
        if self.1 {
            (raw * upper) >> 32
        } else {
            raw % upper
        }
    }

    fn random(&mut self) -> f64 {
        let raw = self.next();
        if self.1 {
            raw as f64 / 4_294_967_296.0
        } else {
            (raw >> 11) as f64 / (1_u64 << 53) as f64
        }
    }

    fn blood_create(&mut self) -> (Vec2, f64) {
        // oBlood own Create, after fixed DrawnSprite/Detritus initialization.
        let velocity = Vec2::new(
            self.random() * 4.0 - self.random() * 4.0,
            -1.0 - self.random() * 2.0,
        );
        (velocity, (1 + self.range(6)) as f64 * 0.1)
    }

    fn walk_tail(&mut self) -> Option<u32> {
        // Snake Step: rand(1,100), then rand(20,50) ONLY on success.
        if self.range(100) == 0 {
            Some(20 + self.range(31) as u32)
        } else {
            None
        }
    }
}

fn room(rules: RulesPreset, snake_x: f64) -> (Game, u64) {
    let mut level = level::Level::default();
    level.set_tile(4, 8, '@').unwrap();
    for x in 0..40 {
        level.set_tile(x, 9, '1').unwrap();
    }
    let mut game = Game::shell(42, rules);
    game.load_custom(&level, "snake-own-destroy-tail", "");
    let snake = game.spawn("oSnake", Vec2::new(snake_x, 144.0));
    assert_eq!(game.player.pos, Vec2::new(88.0, 152.0));
    assert_eq!(game.world.entity(snake).unwrap().health, 1);
    (game, snake)
}

#[test]
fn natural_whip_death_keeps_snake_walk_roll_after_blood_and_destroy() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, snake) = room(rules, 112.0);
        game.step(&InputFrame {
            right: true,
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        let mut saw_whip = false;
        let mut lethal = None;
        for _ in 0..30 {
            saw_whip |= game
                .world
                .entities
                .iter()
                .any(|entity| entity.alive && catalog::is_a(&entity.object, "oWhip"));
            let actor = game.world.entity(snake).expect("WAIT_FOR_REAL_WHIP_HIT");
            if actor.health <= 0 {
                lethal = Some(actor.clone());
                break;
            }
            game.step(&InputFrame {
                right: true,
                ..Default::default()
            });
        }
        let actor = lethal.expect("NATURAL_LETHAL_WHIP_PRECONDITION");
        assert!(saw_whip, "NATURAL_WHIP_INSTANCE_PRECONDITION");
        assert_eq!(actor.state, 1, "SOURCE_WALK_TAIL_PRECONDITION");
        assert_eq!(actor.timer, 0);
        assert_eq!(actor.pos.y, 144.0);
        assert_eq!(actor.vel.y, 0.0);
        assert_eq!(actor.vel.x.abs(), 1.0, "CLEAR_INTEGER_MOVE_PRECONDITION");
        assert_eq!(actor.value("burning"), 0.0);
        assert!(!actor.held);
        assert_eq!(actor.value("bloodless"), 0.0);
        assert_eq!(actor.value("counts_as_kill"), 1.0);
        assert!(game.player.health > 0.0 && !game.player.is_stunned());
        let prior_ids: std::collections::BTreeSet<_> =
            game.world.entities.iter().map(|entity| entity.id).collect();
        let detritus = game
            .world
            .entities
            .iter()
            .filter(|entity| {
                entity.alive && entity.engine_active && catalog::is_a(&entity.object, "oDetritus")
            })
            .count();
        assert!(detritus <= 13, "THREE_BLOOD_CAPACITY_PRECONDITION");
        let health = game.player.health;
        let origin = Vec2::new(actor.pos.x + actor.vel.x + 8.0, actor.pos.y + 8.0);
        let mut expected = LiteralRng::at(&game);
        let expected_blood = [
            expected.blood_create(),
            expected.blood_create(),
            expected.blood_create(),
        ];
        let _post_destroy_counter = expected.walk_tail();
        // No actor, health, timer, RNG, sprite, or position is changed after
        // initialization. This actual Step observes the lethal whip result.
        game.step(&InputFrame::default());
        assert!(game.world.entity(snake).is_none());
        assert_eq!(game.player.health, health);
        let mut drops: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|entity| {
                entity.alive && entity.object == "oBlood" && !prior_ids.contains(&entity.id)
            })
            .collect();
        drops.sort_by_key(|entity| entity.id);
        assert_eq!(drops.len(), 3, "EXACT_DEATH_BLOOD_BIRTHS");
        for (drop, (velocity, gravity)) in drops.into_iter().zip(expected_blood) {
            assert_eq!(drop.pos, origin, "MOVE_THEN_BLOOD_ORIGIN");
            assert_eq!(drop.vel, velocity, "BLOOD_CREATE_ORDER");
            assert_eq!(drop.value("gravity"), gravity);
            assert_eq!((drop.alarms[0], drop.alarms[1], drop.alarms[2]), (1, 1, 5));
        }
        assert_eq!(game.data.get("snakes"), Some(&1.0));
        assert_eq!(game.data.get("kills"), Some(&1.0));
        assert_eq!(
            LiteralRng::at(&game).0,
            expected.0,
            "POST_DESTROY_WALK_RNG: source continues after three Blood Creates"
        );
    }
}

#[test]
fn natural_healthy_snake_keeps_idle_and_walk_roll_boundaries() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, snake) = room(rules, 200.0);
        let mut expected = LiteralRng::at(&game);
        let facing = if expected.range(2) == 0 { -1 } else { 1 };
        game.step(&InputFrame::default());
        let actor = game.world.entity(snake).unwrap();
        assert_eq!((actor.health, actor.state, actor.timer), (1, 1, 0));
        assert_eq!(actor.facing, facing);
        assert_eq!(
            LiteralRng::at(&game).0,
            expected.0,
            "FIRST_IDLE_SINGLE_ROLL"
        );
        let counter = expected.walk_tail();
        game.step(&InputFrame::default());
        let actor = game.world.entity(snake).unwrap();
        assert_eq!(actor.health, 1);
        assert_eq!(
            (actor.state, actor.timer),
            counter.map_or((1, 0), |n| (0, n))
        );
        assert_eq!(LiteralRng::at(&game).0, expected.0, "LIVE_WALK_ORACLE");
        assert!(
            !game
                .world
                .entities
                .iter()
                .any(|entity| entity.object == "oBlood")
        );
        assert_eq!(game.data.get("snakes"), None);
        assert_eq!(game.player.health, 4.0);
    }
}

#[test]
fn literal_rng_keeps_both_independent_streams_and_mappings() {
    let mut canonical = LiteralRng(0, true);
    assert_eq!(canonical.next(), 1);
    assert_eq!(canonical.next(), 134_775_814);
    assert_eq!(canonical.next(), 3_698_175_007);
    let mut legacy = LiteralRng(0, false);
    assert_eq!(legacy.next(), 16_294_208_416_658_607_535);
    assert_eq!(legacy.next(), 7_960_286_522_194_355_700);
    let mut canonical = LiteralRng(42, true);
    assert_eq!(canonical.range(5), 1);
    assert_eq!(canonical.0, 1_365_616_851);
    let mut legacy = LiteralRng(42, false);
    assert_eq!(legacy.range(5), 3);
    assert_eq!(legacy.0, 11_400_714_819_323_198_527);
    assert_eq!(
        LiteralRng(0, true).random().to_bits(),
        0x3df0_0000_0000_0000
    );
    assert_eq!(
        LiteralRng(0, false).random().to_bits(),
        0x3fec_4415_072f_63b9
    );
}
