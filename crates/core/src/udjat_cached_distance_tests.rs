//! oGame's market branch executes within with oPlayer1, not from oGame's mask.
use super::*;
use crate::RulesPreset;
fn fixture(rules: RulesPreset, legacy: bool, player_x: f64, alarm: i32) -> (Game, u64) {
    let mut g = if legacy {
        Game::legacy_shell(123, rules)
    } else {
        Game::shell(123, rules)
    };
    g.world = crate::World::new(40, 20, Area::Mines, 1);
    g.world.name = "rLevel".into();
    g.player.pos = Vec2::new(player_x, 72.4);
    g.player.sprite = "sStandLeft".into();
    g.player.data.insert("image_xscale".into(), -1.0);
    g.alarms[2] = alarm;
    g.flags.insert("udjat_blink".into());
    let market = g.world.spawn("oXMarket", Vec2::new(100.4, 64.4));
    (g, market)
}
#[test]
fn udjat_market_query_uses_player_cached_envelope_and_only_shortens_the_alarm() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for legacy in [false, true] {
            for (px, old_alarm, expected) in
                [(128.4, -1, 6), (128.4, 7, 6), (128.4, 5, 5), (124.4, 7, 4)]
            {
                let (mut g, market) = fixture(rules, legacy, px, old_alarm);
                // Source Exit aggregate[0,0,15,15] at100.4,64.4 caches
                // [100,64,115,79]. Mirrored Stand aggregate[2,0,13,15],
                // origin8 independently rounds [P-7,P+6] and [64.4,79.4].
                let player_left = (px - 7.0).round_ties_even();
                let dm = (player_left - 115.0).max(0.0).max(4.0);
                let literal_alarm = if old_alarm < 1 || dm < f64::from(old_alarm) {
                    dm.round_ties_even() as i32
                } else {
                    old_alarm
                };
                assert_eq!(literal_alarm, expected);
                assert_eq!(
                    g.world
                        .entity(market)
                        .unwrap()
                        .collision_mask()
                        .point_bounds(),
                    Rect::new(100.0, 64.0, 16.0, 16.0)
                );
                assert_eq!(g.player.collision_mask().point_bounds().x, player_left);
                let rng = g.rng.state_bits();
                game_step(&mut g, &InputFrame::default());
                assert_eq!(g.alarms[2], literal_alarm);
                assert!(g.flags.contains("udjat_blink"));
                assert_eq!(g.rng.state_bits(), rng);
                assert_eq!(g.player.health, 4);
                assert!(g.events.is_empty());
            }
        }
    }
}
#[test]
fn udjat_existence_and_with_player_body_use_separate_active_membership() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for legacy in [false, true] {
            for (active, alive) in [(false, true), (true, false)] {
                let (mut g, id) = fixture(rules, legacy, 128.4, 100);
                let e = g.world.entities.iter_mut().find(|e| e.id == id).unwrap();
                e.engine_active = active;
                e.alive = alive;
                let rng = g.rng.state_bits();
                game_step(&mut g, &InputFrame::default());
                assert!(!g.flags.contains("udjat_blink"));
                assert_eq!(g.alarms[2], 100);
                assert_eq!(g.rng.state_bits(), rng);
            }
            let (mut g, id) = fixture(rules, legacy, 128.4, 100);
            g.world.entity_mut(id).unwrap().engine_active = false;
            let far = g.world.spawn("oXMarket", Vec2::new(200.4, 64.4));
            assert!(g.world.entity(far).unwrap().engine_active);
            let rng = g.rng.state_bits();
            game_step(&mut g, &InputFrame::default());
            // Player cached right134; active Exit cached left200 =>66.
            assert_eq!(g.alarms[2], 66);
            assert!(g.flags.contains("udjat_blink"));
            assert_eq!(g.rng.state_bits(), rng);
            let (mut g, _) = fixture(rules, legacy, 128.4, 100);
            g.player.engine_active = false;
            let rng = g.rng.state_bits();
            game_step(&mut g, &InputFrame::default());
            assert_eq!(
                g.alarms[2], 100,
                "inactive Player is absent from with oPlayer1"
            );
            assert!(
                g.flags.contains("udjat_blink"),
                "active marker still satisfies instance_exists"
            );
            assert_eq!(g.rng.state_bits(), rng);
            let (mut g, id) = fixture(rules, legacy, 128.4, 100);
            g.player.engine_active = false;
            g.world.entity_mut(id).unwrap().engine_active = false;
            game_step(&mut g, &InputFrame::default());
            assert_eq!(g.alarms[2], 100);
            assert!(!g.flags.contains("udjat_blink"));
        }
    }
}
