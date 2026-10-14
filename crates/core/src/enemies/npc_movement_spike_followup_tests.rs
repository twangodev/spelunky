//! Source-branch controls and ordinary neutral/pickup/jump routes.
use super::*;
fn room(rules: RulesPreset) -> Game {
    let mut g = Game::shell(71, rules);
    g.world = World::new(40, 20, Area::Mines, 1);
    g.world.name = "rLevel".into();
    for x in 0..40 {
        g.world.set_tile(x, 10, "oHardBlock");
    }
    g.player.pos = Vec2::new(80.0, 152.0);
    g.player.previous_pos = g.player.pos;
    g.world.entrance = g.player.pos;
    g
}
fn selected_keeper(g: &mut Game, keeper: &mut Entity) {
    shopkeeper(g, keeper);
}
#[test]
fn keeper_follow_turn_latch_above_stop_and_common_friction_follow_literal_branches() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut g = room(rules);
        let stock = g.spawn("oPistol", Vec2::new(80.0, 148.0));
        g.player.held = Some(stock);
        g.world.entity_mut(stock).unwrap().held = true;
        assert!(g.world.entity(stock).unwrap().value("cost") > 0.0);
        let id = g.spawn("oShopkeeper", Vec2::new(128.0, 144.0));
        let index = g.world.entities.iter().position(|e| e.id == id).unwrap();
        let mut keeper = g.world.entities.remove(index);
        keeper.state = 5;
        keeper.facing = 1;
        keeper.vel = Vec2::default();
        keeper.data.insert("turn_timer".into(), 0.0);
        g.player.pos = Vec2::new(80.0, 151.0);
        selected_keeper(&mut g, &mut keeper);
        assert_eq!(keeper.state, 5);
        assert_eq!(keeper.facing, -1);
        assert_eq!(keeper.value("turn_timer"), 10.0);
        assert!((keeper.vel.x + 2.9).abs() < 1e-12);
        g.player.pos = Vec2::new(240.0, 151.0);
        selected_keeper(&mut g, &mut keeper);
        assert_eq!(keeper.facing, -1);
        assert_eq!(keeper.value("turn_timer"), 9.0);
        g.player.pos.y = 100.0;
        selected_keeper(&mut g, &mut keeper);
        assert_eq!(
            keeper.vel.x, 0.0,
            "FOLLOW stops below an airborne character"
        );
        keeper.state = 1;
        keeper.facing = 1;
        keeper.vel = Vec2::default();
        g.player.pos = Vec2::new(300.0, 152.0);
        selected_keeper(&mut g, &mut keeper);
        // The complete WALK source now selects PATROL at this unsupported
        // edge, then overwrites xVel before the independent pause roll.
        assert_eq!(keeper.state, 4);
        assert!((20..=50).contains(&keeper.timer));
        assert_eq!(keeper.vel.x, 1.4);
        keeper.state = 0;
        keeper.vel = Vec2::new(1.0, 0.0);
        g.player.held = None;
        selected_keeper(&mut g, &mut keeper);
        assert_eq!(
            keeper.vel.x, 0.9,
            "IDLE retains source drift through common friction"
        );
    }
}
#[test]
fn ordinary_priced_pistol_pickup_then_jump_stops_following_keeper() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut g = room(rules);
        // Keep the ordinary jump inside the original shop room, whose
        // y144 boundary the previous floor/actor layout crossed.
        for x in 0..40 {
            g.world.set_tile(x, 10, "");
            g.world.set_tile(x, 12, "oHardBlock");
        }
        g.player.pos = Vec2::new(80.0, 184.0);
        g.player.previous_pos = g.player.pos;
        g.world.entrance = g.player.pos;
        let keeper = g.spawn("oShopkeeper", Vec2::new(128.0, 176.0));
        let pistol = g.spawn("oPistol", Vec2::new(80.0, 188.0));
        for _ in 0..4 {
            g.step(&InputFrame::default());
        }
        g.step(&InputFrame {
            down: true,
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert_eq!(g.player.held, Some(pistol), "ordinary grounded pickup");
        g.step(&InputFrame::default());
        assert_eq!(
            g.world.entity(keeper).unwrap().state,
            5,
            "positive-priced held stock engages FOLLOW"
        );
        g.step(&InputFrame {
            jump: true,
            jump_pressed: true,
            ..Default::default()
        });
        let mut stopped = false;
        for _ in 0..10 {
            g.step(&InputFrame {
                jump: true,
                ..Default::default()
            });
            let k = g.world.entity(keeper).unwrap();
            if g.player.pos.y < k.pos.y {
                assert_eq!(k.state, 5);
                assert_eq!(k.vel.x, 0.0);
                stopped = true;
                break;
            }
        }
        assert!(stopped, "ordinary jump must cross above the keeper origin");
        assert_eq!(g.player.health, 4);
        assert_eq!(g.player.held, Some(pistol));
        assert_eq!(g.world.entity(keeper).unwrap().health, 20);
        assert!(!g.flags.contains("wanted"));
    }
}
#[test]
fn ordinary_damsel_fall_stains_tile_spikes_without_run_kill_credit() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut g = room(rules);
        g.player.pos = Vec2::new(240.0, 128.0);
        g.player.previous_pos = g.player.pos;
        g.world.set_tile(6, 8, "oSpikes");
        let serial = g.world.tile(6, 8).unwrap().creation_serial;
        let damsel = g.spawn("oDamsel", Vec2::new(104.0, 80.0));
        let mut hit = false;
        for _ in 0..40 {
            g.step(&InputFrame::default());
            if g.world.entity(damsel).is_some_and(|e| e.health <= 0) {
                hit = true;
                break;
            }
        }
        assert!(hit, "ordinary gravity must reach authored terrain spikes");
        assert_eq!(g.world.entity(damsel).unwrap().health, 0);
        let metadata = g
            .world
            .entities
            .iter()
            .find(|e| {
                e.alive
                    && e.object == "oTerrainMetadata"
                    && e.text
                        .get("terrain_sprite")
                        .is_some_and(|s| s == "sSpikesBlood")
            })
            .expect("selected terrain spike must be stained");
        assert_eq!((metadata.pos.x / 16.0).floor(), 6.0);
        assert_eq!((metadata.pos.y / 16.0).floor(), 8.0);
        assert_eq!(g.world.tile(6, 8).unwrap().creation_serial, serial);
        assert_eq!(g.lifetime_stats.damsels_killed_total, 1);
        assert_eq!(g.data.get("kills").copied().unwrap_or(0.0), 0.0);
        assert_eq!(g.player.health, 4);
    }
}
