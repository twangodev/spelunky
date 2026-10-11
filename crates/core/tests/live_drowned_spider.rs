//! Source-produced drowning from an authored initial water/contact fixture.
//! All input after construction is neutral Game::step; this is not a dry-room
//! traversal or a replay from a generated entrance. No state4 is injected into
//! normal Spiders. Giant state4 controls below are explicitly synthetic.
use spelunky_core::*;

fn room(rules: RulesPreset) -> Game {
    let mut game = Game::legacy_shell(42, rules);
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

fn splash_count(game: &Game) -> usize {
    game.world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oSplash")
        .count()
}

fn wet_entry(rules: RulesPreset, ceiling: bool, side: i8) -> (Game, u64) {
    let mut game = room(rules);
    let x = match side {
        -1 => 79.0,
        2 => 63.0,
        _ => 65.0,
    };
    if ceiling {
        for col in 3..=6 {
            game.world.set_tile(col, 3, "oBlock");
        }
    }
    for col in 3..=6 {
        game.world.set_tile(col, 5, "oBlock");
    }
    if side == 1 {
        game.world.set_tile(5, 4, "oBlock");
    }
    if side == -1 {
        game.world.set_tile(4, 4, "oBlock");
    }
    if side == 2 {
        game.world.set_tile(3, 4, "oBlock");
        // Native non-grid source instance, supported by the floor. Together
        // with the left tile this leaves exactly the 14px movement footprint.
        game.spawn("oPushBlock", Vec2::new(78.0, 64.0));
    }
    // Ordinary warmup, rather than assigning the room clock. Entry then runs
    // at time3, first already-DROWNED Step at4, and comparison movement at5.
    game.step(&InputFrame::default());
    assert_eq!(game.engine_time(), 2);
    let water_x = if side == -1 { 80.0 } else { 64.0 };
    let water = game.spawn("oWaterSwim", Vec2::new(water_x, 64.0));
    let id = game.spawn("oSpider", Vec2::new(x, 59.0));
    let center = Vec2::new(x + 8.0, 67.0);
    let water = game.world.entity(water).unwrap();
    assert_eq!(water.object, "oWaterSwim");
    assert_eq!(water.sprite, "sWater");
    assert!(water.sprite_bounds().w > 0.0 && water.sprite_bounds().h > 0.0);
    assert!(
        water.sprite_bounds().contains(center),
        "actual masked water covers source center"
    );
    assert!(!game.world.solid_at(center.x, center.y));
    let spider = game.world.entity(id).unwrap();
    assert_eq!(spider.state, 0);
    assert_eq!(spider.health, 1);
    assert_eq!(spider.vel, Vec2::default());
    game.step(&InputFrame::default());
    assert_eq!(game.engine_time(), 3);
    let spider = game.world.entity(id).unwrap();
    assert_eq!(spider.state, 4, "real water action must establish DROWNED");
    assert_eq!(spider.sprite, "sSpiderDrowning");
    assert_eq!(spider.pos, Vec2::new(x, 59.0));
    assert_eq!(
        spider.vel,
        Vec2::new(0.0, 0.2),
        "first water entry overrides earlier probes"
    );
    assert_eq!(spider.alarms[1], 30);
    // Inherited Enemy and own Spider water entry each produce one Splash.
    assert_eq!(splash_count(&game), 2);
    assert_eq!(game.player.health, 4);
    (game, id)
}

#[test]
fn naturally_entered_drowned_spider_keeps_ceiling_probe_and_next_frame_motion() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, id) = wet_entry(rules, true, 0);
        let original = game.world.entity(id).unwrap().pos;
        let rng = game.rng.clone().next_u64();
        // Exact source line y+5-1==63 touches the ceiling; center remains clear.
        assert!(game.world.solid_at(original.x + 1.0, original.y + 4.0));
        assert!(!game.world.solid_at(original.x + 8.0, original.y + 8.0));
        assert!(!game.world.solid_at(original.x + 1.0, original.y + 16.0));
        game.step(&InputFrame::default());
        assert_eq!(game.engine_time(), 4);
        let spider = game.world.entity(id).unwrap();
        assert_eq!(spider.pos, original, ".2 has no displacement at time4");
        assert_eq!(spider.state, 4);
        assert_eq!(spider.vel, Vec2::new(0.0, 1.0), "DROWNED_TOP_PROBE");
        assert_eq!(spider.alarms[1], 29);
        assert_eq!(splash_count(&game), 2, "drowning does not reenter or rearm");
        assert_eq!(game.rng.clone().next_u64(), rng);
        game.step(&InputFrame::default());
        assert_eq!(game.engine_time(), 5);
        let spider = game.world.entity(id).unwrap();
        assert_eq!(spider.pos, Vec2::new(original.x, original.y + 1.0));
        assert_eq!(spider.state, 4);
        assert_eq!(spider.alarms[1], 28);
        assert_eq!(game.player.health, 4);
    }
}

#[test]
fn naturally_entered_drowned_spider_keeps_source_side_order() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (side, expected) in [(1, 1.0), (-1, -1.0), (2, -1.0)] {
            let (mut game, id) = wet_entry(rules, false, side);
            let p = game.world.entity(id).unwrap().pos;
            assert_eq!(game.world.solid_at(p.x + 15.0, p.y + 8.0), side != -1);
            assert_eq!(game.world.solid_at(p.x, p.y + 8.0), side != 1);
            assert!(!game.world.solid_at(p.x + 8.0, p.y + 8.0));
            let rng = game.rng.clone().next_u64();
            game.step(&InputFrame::default());
            let spider = game.world.entity(id).unwrap();
            assert_eq!(spider.state, 4);
            assert_eq!(spider.vel.x, expected, "DROWNED_SIDE_ORDER");
            assert!((spider.vel.y - 0.4).abs() < 1e-12);
            assert_eq!(spider.alarms[1], 29);
            assert_eq!(splash_count(&game), 2);
            assert_eq!(game.rng.clone().next_u64(), rng);
        }
    }
}

#[test]
fn drowned_noncontact_control_preserves_gravity_state_and_no_reentry() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, id) = wet_entry(rules, false, 0);
        let rng = game.rng.clone().next_u64();
        game.step(&InputFrame::default());
        let spider = game.world.entity(id).unwrap();
        assert_eq!(spider.state, 4);
        assert_eq!(spider.vel.x, 0.0);
        assert!((spider.vel.y - 0.4).abs() < 1e-12);
        assert_eq!(spider.alarms[1], 29);
        assert_eq!(splash_count(&game), 2);
        assert_eq!(game.rng.clone().next_u64(), rng);
    }
}

#[test]
fn synthetic_giant_drowned_conditional_keeps_top_timer_and_crawl_priority() {
    // Giant declares DROWNED but no natural status4 producer was established.
    // These initial field controls test the shared conditional body only.
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for bottom in [false, true] {
            let mut game = room(rules);
            for col in 4..=5 {
                game.world.set_tile(col, 3, "oBlock");
                if bottom {
                    game.world.set_tile(col, 5, "oBlock");
                }
            }
            let id = game.spawn("oGiantSpider", Vec2::new(64.0, 48.0));
            let spider = game.world.entity_mut(id).unwrap();
            spider.state = 4;
            spider.data.insert("squirt_timer".into(), 2.0);
            let rng = game.rng.clone().next_u64();
            game.step(&InputFrame::default());
            let spider = game.world.entity(id).unwrap();
            assert_eq!(spider.health, 1);
            assert_eq!(spider.state, if bottom { 3 } else { 4 });
            assert_eq!(spider.value("squirt_timer"), 1.0);
            assert_eq!(spider.vel.y, 1.0);
            assert_eq!(game.rng.clone().next_u64(), rng);
        }
    }
}
