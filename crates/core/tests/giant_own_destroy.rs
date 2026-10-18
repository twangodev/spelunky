//! Bounded authored-contact tests for Giant Spider's own Destroy continuation.
//! Initial SQUIRT branch state is synthetic where explicitly marked. These
//! fixtures do not establish full campaign or original-runtime parity.
use spelunky_core::*;

fn game(rules: RulesPreset) -> Game {
    let mut game = Game::legacy_shell(0, rules);
    game.player.pos = Vec2::new(216.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game.world.set_tile(13, 10, "oBlock");
    game
}
fn blood_draws(rng: &mut Rng, count: usize) {
    for _ in 0..count {
        let _x1 = rng.unit();
        let _x2 = rng.unit();
        let _y = rng.unit();
        let _gravity = rng.range(6);
    }
}

fn death_prefix(rng: &mut Rng) -> Vec<(&'static str, f64)> {
    let count = 1 + rng.range(3);
    let mut gems = Vec::new();
    for _ in 0..count {
        let object = ["oEmeraldBig", "oSapphireBig", "oRubyBig"][rng.range(3) as usize];
        let vx = f64::from(rng.range(4)) - f64::from(rng.range(4));
        gems.push((object, vx));
    }
    // Biggem/Paste inherited+own Create contains no random statements.
    blood_draws(rng, 4);
    gems
}
fn assert_prefix(game: &Game, giant: u64, gems: &[(&str, f64)]) {
    assert!(
        game.world.entity(giant).is_none(),
        "SOURCE_RECEIVER_RETIRED"
    );
    assert_eq!(game.data.get("giantspiders"), Some(&1.0));
    assert_eq!(game.data.get("kills"), Some(&1.0));
    let mut actual: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| {
            e.alive
                && matches!(
                    e.object.as_str(),
                    "oEmeraldBig" | "oSapphireBig" | "oRubyBig"
                )
        })
        .collect();
    actual.sort_by_key(|e| e.id);
    assert_eq!(actual.len(), gems.len());
    for (gem, (object, vx)) in actual.into_iter().zip(gems) {
        assert_eq!(gem.object, *object);
        assert_eq!(
            gem.pos,
            Vec2::new(80.0, 80.0),
            "EARLIER_RESOURCE_NO_BIRTH_STEP"
        );
        assert_eq!(gem.vel, Vec2::new(*vx, -2.0));
    }
    let paste: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oPaste")
        .collect();
    assert_eq!(paste.len(), 1);
    assert_eq!(paste[0].pos, Vec2::new(80.0, 80.0));
    assert_eq!(
        (paste[0].value("cost"), paste[0].value("forSale")),
        (0.0, 0.0)
    );
    let blood: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oBlood")
        .collect();
    assert_eq!(blood.len(), 4);
    for drop in blood {
        assert_eq!(drop.pos, Vec2::new(80.0, 80.0));
        assert_eq!(drop.value("life"), 60.0, "BLOOD370_PRECEDES_GIANT749");
    }
    assert_eq!(game.player.health, 4.0);
}
#[test]
fn fresh_giant_source_crush_continues_idle_alarm_after_death_prefix() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = game(rules);
        // Authored initial overlap, not a generated route: source center at
        // (80,80) is solid; top probe y71 is clear, preventing CRAWL override.
        game.world.set_tile(5, 5, "oBlock");
        let giant = game.spawn("oGiantSpider", Vec2::new(64.0, 56.0));
        let actor = game.world.entity(giant).unwrap();
        assert_eq!((actor.health, actor.state), (1, 0));
        assert!(game.world.solid_at(80.0, 80.0));
        assert!(!game.world.solid_at(80.0, 71.0));
        let mut expected = game.rng.clone();
        let gems = death_prefix(&mut expected);
        let _alarm = 5 + expected.range(16);
        game.step(&InputFrame::default());
        assert_prefix(&game, giant, &gems);
        assert_eq!(
            game.rng.clone().next_u64(),
            expected.next_u64(),
            "POST_DESTROY_IDLE_ALARM_DRAW"
        );
        assert!(
            !game
                .world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oWebBall")
        );
    }
}
#[test]
fn synthetic_squirt_crush_creates_web_then_cooldown_and_same_frame_web_step() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = game(rules);
        game.world.set_tile(5, 5, "oBlock");
        let giant = game.spawn("oGiantSpider", Vec2::new(64.0, 56.0));
        // Controlled incoming state, explicitly NOT a naturally entered SQUIRT
        // or a natural pistol-death witness. The lethal hit itself is source
        // collision_point, not injected HP; no writes after this setup.
        let actor = game.world.entity_mut(giant).unwrap();
        actor.state = 5;
        actor.sprite = "sGiantSpiderSquirt".into();
        actor.animation = 5.0;
        actor.data.insert("squirt_timer".into(), 0.0);
        let mut expected = game.rng.clone();
        let gems = death_prefix(&mut expected);
        let vy = -(expected.unit() * 3.0 + 1.0);
        // Mandatory PRE-Step source mask qualification, not post-outcome
        // selection: Web bottom RTE(72+vy+3) must be above Paste top80-6.
        // Seed0's pinned portable tape clears this; canonical migration must
        // requalify it rather than ignore any failed precondition.
        assert!(
            (75.0 + vy).round_ties_even() < 74.0,
            "QUALIFIED_NEWBORN_WEB_PASTE_MASK_SEPARATION"
        );
        let mut vx = f64::from(1 + expected.range(3));
        if expected.chance(1, 2) {
            vx *= -1.0;
        }
        let life = f64::from(20 + expected.range(81));
        let _cooldown = 100 + expected.range(901);
        game.step(&InputFrame::default());
        assert_prefix(&game, giant, &gems);
        let webs: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|e| e.alive && e.object == "oWebBall")
            .collect();
        assert_eq!(webs.len(), 1, "POST_DESTROY_SQUIRT_BIRTH");
        let web = webs[0];
        assert_eq!(web.value("web_create_initialized"), 1.0);
        assert_eq!(
            web.sprite, "sWebBall",
            "NO_SOLID_COLLISION_BEFORE_WEB_STEP_ASSERTS"
        );
        assert_eq!(
            web.pos,
            Vec2::new(80.0 + vx, 72.0 + vy),
            "LATER_RESOURCE750_BIRTH_STEP"
        );
        assert_eq!(web.vel, Vec2::new(vx, vy + 0.2));
        assert_eq!(web.value("life"), life - 1.0);
        assert_eq!(
            game.rng.clone().next_u64(),
            expected.next_u64(),
            "WEB_CREATE_FOUR_DRAWS_BEFORE_COOLDOWN"
        );
        game.step(&InputFrame::default());
        assert_eq!(
            game.world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == "oWebBall")
                .count(),
            1,
            "NO_SECOND_DEAD_RECEIVER_STEP"
        );
        assert_eq!(game.data.get("giantspiders"), Some(&1.0));
        assert_eq!(game.data.get("kills"), Some(&1.0));
    }
}
#[test]
fn fresh_live_giant_keeps_single_idle_draw_and_no_death_loot() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = game(rules);
        let giant = game.spawn("oGiantSpider", Vec2::new(64.0, 56.0));
        let mut expected = game.rng.clone();
        let alarm = (5 + expected.range(16)) as i32;
        game.step(&InputFrame::default());
        let actor = game.world.entity(giant).unwrap();
        assert_eq!((actor.health, actor.state, actor.alarms[0]), (1, 2, alarm));
        assert_eq!(game.rng.clone().next_u64(), expected.next_u64());
        assert!(
            !game
                .world
                .entities
                .iter()
                .any(|e| e.alive && matches!(e.object.as_str(), "oBlood" | "oPaste" | "oWebBall"))
        );
        assert_eq!(game.data.get("kills"), None);
    }
}
#[test]
fn hanging_source_crush_uses_else_if_and_does_not_create_replacement_spider() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (object, fresh) in [
            ("oSpiderHang", "oSpider"),
            ("oGiantSpiderHang", "oGiantSpider"),
        ] {
            let mut game = game(rules);
            game.world.set_tile(4, 4, "oBlock");
            game.world.set_tile(5, 4, "oBlock");
            let hanging = game.spawn(object, Vec2::new(64.0, 64.0));
            assert!(
                !game.world.solid_at(64.0, 48.0),
                "CONVERSION_LOST_CEILING_ALSO_TRUE"
            );
            assert!(
                game.world
                    .solid_at(if object == "oSpiderHang" { 72.0 } else { 80.0 }, 72.0)
            );
            game.step(&InputFrame::default());
            assert!(game.world.entity(hanging).is_none());
            assert!(
                !game
                    .world
                    .entities
                    .iter()
                    .any(|e| e.alive && e.object == fresh),
                "HANGING_ELSE_IF_EXCLUDES_POST_DEATH_CONVERSION"
            );
            assert_eq!(game.data.get("kills"), Some(&1.0));
        }
    }
}
