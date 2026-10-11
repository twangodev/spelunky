//! Ordinary free equipment pickup, ledge fall, and walking contact; initial room setup only.
use spelunky_core::*;

fn modes() -> impl Iterator<Item = (RulesPreset, RngAlgorithm)> {
    [RulesPreset::Classic, RulesPreset::Assisted]
        .into_iter()
        .flat_map(|rules| {
            [RngAlgorithm::Gm8Lcg32V1, RngAlgorithm::LegacySplitMix64V1]
                .into_iter()
                .map(move |algorithm| (rules, algorithm))
        })
}

fn scene(
    rules: RulesPreset,
    algorithm: RngAlgorithm,
    falling: bool,
    shoes: bool,
    roof: bool,
) -> (Game, u64, Option<u64>) {
    for seed in 0..128 {
        let mut game = Game::shell_with_algorithm(seed, rules, algorithm).unwrap();
        game.world = World::new(30, 20, Area::Ice, 9);
        for x in 0..=6 {
            game.world.set_tile(x, 10, "oBlock");
        }
        if falling {
            for x in 0..=4 {
                game.world.set_tile(x, 8, "oBlock");
            }
        }
        if roof {
            game.world.set_tile(6, 8, "oBlock");
        }
        game.player.pos = Vec2::new(72.0, if falling { 120.0 } else { 152.0 });
        game.player.previous_pos = game.player.pos;
        let yeti = game.spawn("oYeti", Vec2::new(96.0, 144.0));
        let item = if shoes {
            let id = game.spawn(
                "oSpikeShoes",
                Vec2::new(game.player.pos.x, game.player.pos.y + 4.0),
            );
            // Authored free item fields are the same as the editor's V tile.
            let item = game.world.entity_mut(id).unwrap();
            item.data.insert("cost".into(), 0.0);
            item.data.insert("forSale".into(), 0.0);
            Some(id)
        } else {
            None
        };
        // Initial seed condition alone selects the right-facing IDLE->WALK draw.
        if game.rng.clone().range(2) != 1 {
            continue;
        }
        let actor = game.world.entity(yeti).unwrap();
        assert_eq!((actor.state, actor.health), (0, 5));
        assert_eq!(actor.vel, Vec2::new(2.5, 0.0));
        assert!(game.player.equipment.is_empty());
        return (game, yeti, item);
    }
    panic!("initial-only right-facing source fixture unavailable");
}

fn random(rng: &mut Rng) -> f64 {
    let algorithm = rng.algorithm();
    let raw = rng.next_u64();
    match algorithm {
        RngAlgorithm::Gm8Lcg32V1 => raw as f64 / 4_294_967_296.0,
        RngAlgorithm::LegacySplitMix64V1 => (raw >> 11) as f64 / 9_007_199_254_740_992.0,
    }
}
fn rand(rng: &mut Rng, width: u64) -> u32 {
    let algorithm = rng.algorithm();
    let raw = rng.next_u64();
    match algorithm {
        RngAlgorithm::Gm8Lcg32V1 => ((raw * width) >> 32) as u32,
        RngAlgorithm::LegacySplitMix64V1 => (raw % width) as u32,
    }
}
fn literal_blood(rng: &mut Rng) -> (Vec2, f64) {
    let vx = random(rng) * 4.0 - random(rng) * 4.0;
    let vy = -1.0 - random(rng) * 2.0;
    (Vec2::new(vx, vy), f64::from(1 + rand(rng, 6)) * 0.1)
}

fn fall_stomp(rules: RulesPreset, algorithm: RngAlgorithm, shoes: bool) -> (Game, u64, u64, Rng) {
    let (mut game, yeti, item) = scene(rules, algorithm, true, shoes, false);
    game.step(&InputFrame {
        down: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    if let Some(item) = item {
        assert!(
            game.world.entity(item).is_none(),
            "ordinary free SpikeShoes pickup consumes item"
        );
        assert!(game.player.has("SpikeShoes"));
        assert!(
            !game.player.has("spike_shoes"),
            "canonical pickup does not inject a lowercase alias"
        );
    } else {
        assert!(game.player.equipment.is_empty());
    }
    assert_eq!(game.player.held, None);
    let mut falling = false;
    for _frame in 0..40 {
        let actor = game.world.entity(yeti).unwrap().clone();
        let before = game.player.clone();
        let birth = game.world.next_id;
        let expected = game.rng.clone();
        falling |= before.vel.y > 0.0;
        game.step(&InputFrame {
            right: true,
            ..Default::default()
        });
        let after = game.world.entity(yeti).unwrap();
        assert_eq!(game.player.health, 4);
        if after.state == 98 {
            assert!(falling, "actual ledge fall precedes source stomp");
            let fall = before.data.get("fall_timer").copied().unwrap_or(0.0);
            assert!(
                (1.0..16.0).contains(&fall),
                "positive short fall selects ceil(fallTimer/16)==1"
            );
            assert_eq!(
                actor.state, 0,
                "naturally paused Yeti has no random WALK tape on stomp frame"
            );
            assert!(actor.value("counter") > 1.0);
            assert_eq!(after.timer, 200);
            assert!(game.player.vel.y < 0.0, "actual stomp bounces player");
            assert_eq!(
                game.player.data.get("fall_timer").copied().unwrap_or(0.0),
                0.0
            );
            return (game, yeti, birth, expected);
        }
        assert_eq!(after.health, 5);
    }
    panic!("ordinary ledge fall must stomp default Yeti");
}

fn assert_stomp_blood(game: &Game, birth: u64, mut expected: Rng, present: bool) {
    let drops: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.id >= birth && e.object == "oBlood")
        .collect();
    assert_eq!(
        drops.len(),
        usize::from(present),
        "SOURCE_YETI_SPIKED_STOMP_DIRECT_BLOOD"
    );
    if present {
        let (velocity, gravity) = literal_blood(&mut expected);
        let drop = drops[0];
        assert_eq!(
            drop.previous_pos,
            Vec2::new(game.player.pos.x, game.player.pos.y + 8.0)
        );
        assert_eq!(drop.vel, velocity);
        assert_eq!(drop.value("gravity"), gravity);
        assert_eq!(&drop.alarms[..3], &[1, 1, 5]);
        assert_eq!(drop.value("bounce"), 0.0);
    }
    assert_eq!(game.rng.clone().next_u64(), expected.next_u64());
}

#[test]
fn ordinary_spike_shoes_pickup_then_fall_stomp_uses_triple_damage() {
    for (rules, algorithm) in modes() {
        let (game, yeti, birth, expected) = fall_stomp(rules, algorithm, true);
        assert_eq!(
            game.world.entity(yeti).unwrap().health,
            2,
            "SOURCE_CANONICAL_SPIKE_SHOES_TRIPLE_DAMAGE"
        );
        assert_stomp_blood(&game, birth, expected, true);
    }
}

#[test]
fn ordinary_spike_shoes_pickup_then_fall_stomp_creates_direct_blood() {
    for (rules, algorithm) in modes() {
        let (game, yeti, birth, expected) = fall_stomp(rules, algorithm, true);
        assert_stomp_blood(&game, birth, expected, true);
        assert_eq!(game.world.entity(yeti).unwrap().health, 2);
    }
}

#[test]
fn ordinary_barefoot_fall_stomp_keeps_single_damage_without_spike_blood() {
    for (rules, algorithm) in modes() {
        let (game, yeti, birth, expected) = fall_stomp(rules, algorithm, false);
        assert_eq!(game.world.entity(yeti).unwrap().health, 4);
        assert_stomp_blood(&game, birth, expected, false);
    }
}

fn walk_contact(rules: RulesPreset, algorithm: RngAlgorithm, roof: bool) -> (Game, u64, u64, Rng) {
    let (mut game, yeti, _) = scene(rules, algorithm, false, false, roof);
    for _frame in 0..40 {
        let actor = game.world.entity(yeti).unwrap().clone();
        let birth = game.world.next_id;
        let expected = game.rng.clone();
        game.step(&InputFrame {
            right: true,
            ..Default::default()
        });
        let after = game.world.entity(yeti).unwrap();
        assert_eq!(after.health, 5);
        if game.player.health != 4 || after.state == 3 {
            assert_eq!(actor.state, 0);
            assert!(actor.value("counter") > 1.0);
            return (game, yeti, birth, expected);
        }
        assert_eq!(game.player.stun, 0);
    }
    panic!("ordinary walking must reach Yeti Character contact");
}

#[test]
fn ordinary_low_ceiling_yeti_contact_creates_direct_player_blood() {
    for (rules, algorithm) in modes() {
        let (game, yeti, birth, mut expected) = walk_contact(rules, algorithm, true);
        let drops: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|e| e.id >= birth && e.object == "oBlood")
            .collect();
        assert_eq!(drops.len(), 1, "SOURCE_YETI_LOW_CEILING_DIRECT_BLOOD");
        let (velocity, gravity) = literal_blood(&mut expected);
        assert_eq!(drops[0].previous_pos, game.player.pos);
        assert_eq!(drops[0].vel, velocity);
        assert_eq!(drops[0].value("gravity"), gravity);
        assert_eq!(&drops[0].alarms[..3], &[1, 1, 5]);
        assert_eq!(game.rng.clone().next_u64(), expected.next_u64());
        assert_eq!(game.player.health, 3);
        assert_eq!(game.player.invincible, 30);
        assert_eq!(game.player.stun, 0);
        assert_ne!(game.player.state, PlayerState::Stunned);
        assert_eq!(game.player.vel.x, 6.0);
        assert!(game.world.entity(yeti).unwrap().state < 98);
    }
}

#[test]
fn ordinary_clear_headroom_yeti_contact_throws_without_damage_or_blood() {
    for (rules, algorithm) in modes() {
        let (game, yeti, birth, mut expected) = walk_contact(rules, algorithm, false);
        assert_eq!(game.player.health, 4);
        assert_eq!(game.player.state, PlayerState::Stunned);
        assert_eq!(game.world.entity(yeti).unwrap().state, 3);
        assert!(
            !game
                .world
                .entities
                .iter()
                .any(|e| e.id >= birth && e.object == "oBlood")
        );
        assert_eq!(game.rng.clone().next_u64(), expected.next_u64());
    }
}
