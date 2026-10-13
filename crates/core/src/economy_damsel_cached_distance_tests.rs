//! Initial configured purchase callbacks, not a naturally generated shop route.
use super::*;
use crate::{Area, RulesPreset, World};
fn fixture(rules: RulesPreset, legacy: bool, player_x: f64) -> (Game, u64) {
    let mut g = if legacy {
        Game::legacy_shell(123, rules)
    } else {
        Game::shell(123, rules)
    };
    g.world = World::new(40, 20, Area::Mines, 1);
    g.world.name = "rLevel".into();
    g.player.pos = Vec2::new(player_x, 152.0);
    g.player.previous_pos = g.player.pos;
    g.player.sprite = "sStandLeft".into();
    g.player.data.insert("image_xscale".into(), -1.0);
    g.player.money = 10000;
    let keeper = g.spawn("oShopkeeper", Vec2::new(160.0, 144.0));
    g.world
        .entity_mut(keeper)
        .unwrap()
        .text
        .insert("style".into(), "Kissing".into());
    let damsel = g.spawn("oDamsel", Vec2::new(100.0, 152.0));
    let d = g.world.entity_mut(damsel).unwrap();
    // An initial ongoing source kiss is eligible for a further paid kiss;
    // Player Step imposes no NPC state or current-animation restriction.
    d.state = 6;
    d.set_sprite("sDamselKissL");
    d.animation = 3.0;
    d.data.insert("forSale".into(), 1.0);
    assert_eq!(g.player.held, None);
    (g, damsel)
}
fn pay(g: &mut Game) {
    player_pay_step(
        g,
        &InputFrame {
            pay_pressed: true,
            ..Default::default()
        },
    );
}
#[test]
fn source_kiss_range_uses_literal_cached_envelopes_and_strict_sixteen_threshold() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for legacy in [false, true] {
            for (player_x, expected_gap) in [(128.0, 15.0), (129.0, 16.0), (130.0, 17.0)] {
                let (mut g, damsel) = fixture(rules, legacy, player_x);
                // Independent original cache arithmetic, not a call to the
                // production distance helper: Kiss aggregate x[0,14], origin8
                // at100 =>[92,106]. Mirrored Stand x[2,13],origin8 yields
                // min(round(P+(2-8)*-1),round(P+(13-8+1)*-1-1))=P-7.
                // Both cached y envelopes are[144,159].
                let literal_gap = ((player_x - 7.0) - 106.0).max(0.0).hypot(0.0);
                assert_eq!(literal_gap, expected_gap);
                let target = g.world.entity(damsel).unwrap();
                let cached = target.collision_mask().point_bounds();
                assert_eq!(cached, crate::Rect::new(92.0, 144.0, 15.0, 16.0));
                assert_eq!(
                    g.player.collision_mask().point_bounds(),
                    crate::Rect::new(player_x - 7.0, 144.0, 14.0, 16.0)
                );
                // Selected frame3 ends at103 and the smooth mirrored character
                // starts atP-5: source gap15 is incorrectly represented as20.
                let old_gap = ((player_x - 5.0) - 103.0).max(0.0);
                assert!(old_gap >= 20.0);
                let rng = g.rng.state_bits();
                pay(&mut g);
                let bought = literal_gap < 16.0;
                assert_eq!(g.player.health, if bought { 5 } else { 4 });
                assert_eq!(g.player.money, if bought { 5000 } else { 10000 });
                assert_eq!(
                    g.data.get("kissesBought").copied().unwrap_or(0.0),
                    f64::from(u8::from(bought))
                );
                assert_eq!(g.lifetime_stats.kisses_bought, u64::from(bought));
                assert_eq!(
                    g.world.entity(damsel).unwrap().animation,
                    if bought { 0.0 } else { 3.0 }
                );
                assert_eq!(g.rng.state_bits(), rng);
                assert_eq!(g.player.held, None);
            }
        }
    }
}
#[test]
fn inactive_nearest_damsel_does_not_gate_or_block_an_active_family_purchase() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for legacy in [false, true] {
            let (mut g, far) = fixture(rules, legacy, 128.0);
            g.world.entity_mut(far).unwrap().engine_active = false;
            pay(&mut g);
            assert_eq!((g.player.health, g.player.money), (4, 10000));
            assert_eq!(g.world.entity(far).unwrap().animation, 3.0);
            // A separate initial fixture controls nearest-family eligibility:
            // a closer inactive/not-forSale actor must not block the active one.
            let (mut g, active) = fixture(rules, legacy, 128.0);
            let near = g.spawn("oDamsel", Vec2::new(128.0, 152.0));
            {
                let n = g.world.entity_mut(near).unwrap();
                n.engine_active = false;
                n.data.insert("forSale".into(), 0.0);
            }
            let rng = g.rng.state_bits();
            pay(&mut g);
            assert_eq!((g.player.health, g.player.money), (5, 5000));
            assert_eq!(g.world.entity(active).unwrap().animation, 0.0);
            assert_eq!(g.world.entity(near).unwrap().state, 0);
            assert_eq!(g.rng.state_bits(), rng);
            // With that nearer actor active, source instance_nearest selects
            // it before eligibility, so its forSale=false blocks the purchase.
            let (mut g, active) = fixture(rules, legacy, 128.0);
            let near = g.spawn("oDamsel", Vec2::new(128.0, 152.0));
            g.world
                .entity_mut(near)
                .unwrap()
                .data
                .insert("forSale".into(), 0.0);
            let rng = g.rng.state_bits();
            pay(&mut g);
            assert_eq!((g.player.health, g.player.money), (4, 10000));
            assert_eq!(g.world.entity(active).unwrap().animation, 3.0);
            assert_eq!(g.rng.state_bits(), rng);
        }
    }
}
