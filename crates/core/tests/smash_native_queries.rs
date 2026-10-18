//! Default integer Smash births reach half-integer motion through their own AI.
//! Safe authored component floor, then neutral inputs only; no mid-route writes.
//! This checks SourceClassic's unrotated stock masks, not native replay transport.
use spelunky_core::*;

fn scene(
    rules: RulesPreset,
    algorithm: RngAlgorithm,
    hero_x: f64,
    traps: &[f64],
) -> (Game, Vec<u64>) {
    let mut game = Game::shell_with_algorithm(42, rules, algorithm).unwrap();
    game.world = World::new(24, 20, Area::Mines, 1);
    for x in 0..game.world.width {
        game.world.set_tile(x, 10, "oHardBlock");
    }
    game.player.pos = Vec2::new(hero_x, 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.world.entrance = game.player.pos;
    let ids = traps
        .iter()
        .map(|&x| game.spawn("oSmashTrap", Vec2::new(x, 144.0)))
        .collect();
    (game, ids)
}
fn actor(game: &Game, id: u64) -> &Entity {
    let trap = game.world.entity(id).expect("default Smash remains alive");
    assert!(trap.engine_active);
    assert_eq!(trap.object, "oSmashTrap");
    assert_eq!(trap.sprite, "sSmashTrap");
    assert_eq!(
        trap.vel,
        Vec2::default(),
        "script xv is distinct from platform xVel"
    );
    assert!(trap.solid, "oGame platform pass restores solidity");
    assert_eq!(trap.data.get("image_angle"), None);
    assert_eq!(trap.data.get("image_xscale"), None);
    assert_eq!(trap.data.get("image_yscale"), None);
    trap
}
fn prefix(game: &Game, x: f64, clock: u64, rng_state: u64) {
    assert_eq!(game.level_tick, clock);
    assert_eq!(game.player.pos, Vec2::new(x, 152.0));
    assert_eq!(game.player.vel, Vec2::default());
    assert!(game.player.grounded);
    assert_eq!(game.player.health, 4.0);
    assert_eq!(game.state, RunState::Playing);
    assert_eq!(
        game.rng.state_bits(),
        rng_state,
        "quiet AI prefix draws nothing after Create"
    );
    assert!(
        game.events
            .iter()
            .all(|event| !matches!(event, GameEvent::Damage { .. }))
    );
    assert!(
        game.world
            .entities
            .iter()
            .all(|entity| entity.object != "oBlood")
    );
}
fn moving_left(trap: &Entity, x: f64, xv: f64) {
    assert_eq!(trap.pos, Vec2::new(x, 144.0));
    assert_eq!(trap.state, 1);
    assert_eq!(trap.value("direction"), 2.0);
    assert_eq!(trap.value("xa"), -0.5);
    assert_eq!(trap.value("xv"), xv);
    assert_eq!(trap.value("yv"), 0.0);
    assert_eq!(trap.value("hit"), 0.0);
}
fn damage(game: &Game, amount: i32) -> usize {
    game.events
        .iter()
        .filter(|event| matches!(event, GameEvent::Damage { amount: a } if *a == amount))
        .count()
}

#[test]
fn neutral_default_smash_half_motion_uses_raw_inclusive_hurt_bbox_then_real_contact() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, ids) = scene(rules, RngAlgorithm::Gm8Lcg32V1, 89.0, &[96.0]);
        assert_eq!(
            game.generation_revision,
            GenerationRevision::SourceClassicV2
        );
        let born_rng = u64::from(42_u32.wrapping_mul(134775813).wrapping_add(1));
        assert_eq!(
            game.rng.state_bits(),
            born_rng,
            "Smash Create consumes one dir draw"
        );
        game.step(&InputFrame::default());
        prefix(&game, 89.0, 1, born_rng);
        moving_left(actor(&game, ids[0]), 96.0, 0.0);
        game.step(&InputFrame::default());
        prefix(&game, 89.0, 2, born_rng);
        let trap = actor(&game, ids[0]);
        moving_left(trap, 95.5, -0.5);
        // Stock sprite has aggregatebbox0..15 and origin0,0. Source cache L=RTE95.5=96.
        let source_left = trap.pos.x.round_ties_even();
        let raw_query_max = game.player.pos.x + 6.0;
        assert_eq!(source_left, 96.0);
        assert_eq!(raw_query_max, 95.0);
        assert!(
            source_left > raw_query_max,
            "literal prec0 source query is clear"
        );
        assert!(
            trap.sprite_bounds()
                .intersects(Rect::new(83.0, 146.0, 13.0, 14.0)),
            "the old smooth Rect route incorrectly reports contact"
        );
        // Player319 reads the PRIOR Smash position before Smash's later own Step.
        game.step(&InputFrame::default());
        assert_eq!(game.level_tick, 3);
        moving_left(actor(&game, ids[0]), 94.5, -1.0);
        assert_eq!(game.player.pos, Vec2::new(89.0, 152.0));
        assert_eq!(
            game.player.health, 4.0,
            "half-integer false contact must not deal10 damage"
        );
        prefix(&game, 89.0, 3, born_rng);
        // Following frame: RTE94.5=94 <=95, so source true contact must still hurt.
        assert_eq!(actor(&game, ids[0]).pos.x.round_ties_even(), 94.0);
        game.step(&InputFrame::default());
        // Original Player Step1983 clamps the first fatal frame to0 without Ankh.
        assert_eq!(game.player.health, 0.0);
        assert_eq!(game.state, RunState::Dead);
        assert_eq!(
            game.events
                .iter()
                .filter(|e| matches!(e, GameEvent::Died))
                .count(),
            1
        );
        assert_eq!(damage(&game, 10), 1);
        assert_eq!(game.player.vel.y, -6.0);
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == "oBlood")
                .count(),
            1
        );
        let mut expected = born_rng as u32;
        for _ in 0..5 {
            expected = expected.wrapping_mul(134775813).wrapping_add(1);
        }
        assert_eq!(
            game.rng.state_bits(),
            u64::from(expected),
            "hurt speed draw then four synchronous Blood Create draws"
        );
    }
}

#[test]
fn neutral_default_smash_peer_motion_uses_source_precise_edge_before_attack() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, ids) = scene(rules, RngAlgorithm::Gm8Lcg32V1, 72.0, &[80.0, 95.0]);
        let state1 = 42_u32.wrapping_mul(134775813).wrapping_add(1);
        let born_rng = u64::from(state1.wrapping_mul(134775813).wrapping_add(1));
        assert_eq!(
            game.rng.state_bits(),
            born_rng,
            "two default dir Creates in birth order"
        );
        for (clock, x, xv) in [(1, 80.0, 0.0), (2, 79.5, -0.5)] {
            game.step(&InputFrame::default());
            prefix(&game, 72.0, clock, born_rng);
            moving_left(actor(&game, ids[0]), x, xv);
            let waiting = actor(&game, ids[1]);
            assert_eq!(waiting.pos, Vec2::new(95.0, 144.0));
            assert_eq!(
                waiting.state, 0,
                "source left2 probe remains blocked through the first half motion"
            );
            assert_eq!(waiting.value("xv"), 0.0);
        }
        // In the third Step A moves first to78.5. RTE origin78 means filled precise pixels end93.
        // B's literal isCollisionLeft(2) line remains x=round((95+1)-2)=94,
        // y145..158. Cached right=RTE93.5=94 admits the broadphase, but local pixel16
        // is outside the stock filled mask0..15. Old smoothAmax94.5 still hits.
        game.step(&InputFrame::default());
        prefix(&game, 72.0, 3, born_rng);
        let a = actor(&game, ids[0]);
        moving_left(a, 78.5, -1.0);
        assert_eq!(a.pos.x.round_ties_even() + 15.0, 93.0);
        assert!(
            a.sprite_bounds()
                .intersects(Rect::new(94.0, 145.0, 1.0, 14.0))
        );
        moving_left(actor(&game, ids[1]), 95.0, 0.0);
    }
}

#[test]
fn historical_smash_hurt_and_peer_edge_routes_remain_unchanged() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut hurt, ids) = scene(rules, RngAlgorithm::LegacySplitMix64V1, 89.0, &[96.0]);
        assert_eq!(hurt.generation_revision, GenerationRevision::HistoricalV1);
        for _ in 0..3 {
            hurt.step(&InputFrame::default());
        }
        // Preserve the historical hurt plus the source first-fatal final clamp.
        assert_eq!(hurt.player.health, 0.0);
        assert_eq!(hurt.state, RunState::Dead);
        assert_eq!(
            hurt.events
                .iter()
                .filter(|e| matches!(e, GameEvent::Died))
                .count(),
            1
        );
        assert_eq!(damage(&hurt, 10), 1);
        assert_eq!(actor(&hurt, ids[0]).pos.x, 94.5);
        let (mut edge, ids) = scene(rules, RngAlgorithm::LegacySplitMix64V1, 72.0, &[80.0, 95.0]);
        let born_rng = edge.rng.state_bits();
        for _ in 0..3 {
            edge.step(&InputFrame::default());
        }
        prefix(&edge, 72.0, 3, born_rng);
        moving_left(actor(&edge, ids[0]), 78.5, -1.0);
        assert_eq!(actor(&edge, ids[1]).state, 0);
    }
}
