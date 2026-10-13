//! Configured source-envelope discriminator, with a real coordinate setter refresh.
//! Existing own-Step API compiles on both old and proposed production.
use super::*;
#[test]
fn follow_stop_uses_aggregate_integer_envelope_not_current_frame_bounds() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for legacy in [false, true] {
            let mut g = if legacy {
                Game::legacy_shell(1, rules)
            } else {
                Game::shell(1, rules)
            };
            g.world = World::new(40, 20, Area::Mines, 1);
            for x in 0..40 {
                g.world.set_tile(x, 10, "oHardBlock");
            }
            g.player.pos = Vec2::new(112.0, 152.0);
            g.player.previous_pos = g.player.pos;
            g.player.sprite = "sStandLeft".into();
            g.player.data.insert("image_xscale".into(), -1.0);
            let gun = g.spawn("oPistol", g.player.pos);
            g.player.held = Some(gun);
            g.world.entity_mut(gun).unwrap().held = true;
            let mut e = Entity::new(90000, "oShopkeeper", Vec2::new(127.0, 144.0));
            crate::enemies::init_entity(&mut e);
            e.state = 5;
            e.facing = -1;
            e.set_sprite("sShopRunLeft");
            e.animation = 3.0;
            e.vel.x = 1.0;
            // Source own moveTo writes x127→128 before dist. The native X
            // setter synchronously refreshes the aggregate bbox (4ABFB3→4AA958).
            // Post-move Run aggregate x[0,15]+128 =>[128,143]; mirrored Stand
            // x[2,13],origin8: round(112+(2-8)*-1)=118,
            // round(112+(13-8+1)*-1-1)=105. Y envelopes both[144,159].
            let source_post_move_distance = (128.0_f64 - 118.0).max(0.0).hypot(0.0);
            assert_eq!(source_post_move_distance, 10.0);
            assert_eq!(
                crate::world_events::mask_distance(e.sprite_bounds(), g.player.sprite_bounds()),
                13.0,
                "old selected frame3 starts at131 before the one-pixel move"
            );
            assert!(!g.world.solid_at(e.pos.x + 8.0, e.pos.y + 8.0));
            let rng = g.rng.state_bits();
            shopkeeper(&mut g, &mut e);
            assert_eq!(
                e.pos,
                Vec2::new(128.0, 144.0),
                "source coordinate write must occur before distance"
            );
            assert_eq!(e.state, 5);
            assert_eq!(
                e.vel.x, 0.0,
                "literal source post-move gap10 is below the strict12 stop gate"
            );
            assert_eq!(e.sprite, "sShopLeft");
            assert_eq!(e.health, 20);
            assert_eq!(g.player.held, Some(gun));
            assert_eq!(g.rng.state_bits(), rng);
        }
    }
}

#[test]
fn native_distance_absent_active_target_uses_million_cap_without_alive_receiver_guard() {
    let mut g = Game::shell(1, RulesPreset::Classic);
    g.player.pos = Vec2::new(112.0, 152.0);
    g.player.sprite = "sStandLeft".into();
    g.player.data.insert("image_xscale".into(), -1.0);
    let mut e = Entity::new(90000, "oShopkeeper", Vec2::new(128.0, 144.0));
    crate::enemies::init_entity(&mut e);
    e.set_sprite("sShopRunLeft");
    e.alive = false;
    assert_eq!(
        keeper_player_distance(&g, &e),
        10.0,
        "native receiver may continue querying after own Destroy"
    );
    g.player.engine_active = false;
    assert_eq!(keeper_player_distance(&g, &e), 1_000_000.0);
}
