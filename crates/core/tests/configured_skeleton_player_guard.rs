//! Controlled constructor/family-query cases; not ordinary reachability claims.
use spelunky_core::{
    Game, Rect, RulesPreset, Vec2,
    dispatcher::{self, Instance},
};

#[test]
fn skeleton_no_active_player_keeps_source_default_right() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut g = Game::shell(7, rules);
        g.player.pos = Vec2::new(40.0, 152.0);
        g.player.engine_active = false;
        let id = g.spawn("oSkeleton", Vec2::new(128.0, 144.0));
        let e = g.world.entity(id).unwrap();
        assert_eq!(e.facing, 1, "SOURCE_SKELETON_NO_ACTIVE_PLAYER");
        assert_eq!((e.health, e.state, e.timer, e.age), (1, 0, 20, 0));
    }
}

#[test]
fn skeleton_existing_player_is_not_gated_by_health_or_input_active() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for health in [0.0, 4.0] {
            for active in [0.0, 1.0] {
                let mut g = Game::shell(7, rules);
                g.player.pos = Vec2::new(40.0, 152.0);
                g.player.health = health;
                g.player.data.insert("active".into(), active);
                assert!(g.player.engine_active);
                let id = g.spawn("oSkeleton", Vec2::new(128.0, 144.0));
                assert_eq!(g.world.entity(id).unwrap().facing, -1);
                assert_eq!(g.player.health, health);
                assert_eq!(g.player.data["active"], active);
            }
        }
    }
}

#[test]
fn skeleton_reads_first_reactivated_player_family_member() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut g = Game::shell(7, rules);
        g.player.pos = Vec2::new(40.0, 152.0);
        g.player.engine_active = false;
        let other = g.spawn("oPlayer1", Vec2::new(160.0, 152.0));
        // A flag write alone does not commit native list removal. This initial
        // membership operation leaves primary Player outside its region and
        // commits its inactive ledger before the later activation operation.
        dispatcher::activate_region(&mut g, Rect::new(152.0, 144.0, 16.0, 16.0));
        assert_eq!(
            dispatcher::descendant_instances(&g, "oPlayer1"),
            vec![Instance::Entity(other)]
        );
        assert!(g.dispatcher.inactive_order.contains(&Instance::Player));
        dispatcher::activate_region(&mut g, Rect::new(32.0, 144.0, 16.0, 16.0));
        assert_eq!(
            dispatcher::descendant_instances(&g, "oPlayer1"),
            vec![Instance::Entity(other), Instance::Player]
        );
        let id = g.spawn("oSkeleton", Vec2::new(128.0, 144.0));
        assert_eq!(
            g.world.entity(id).unwrap().facing,
            1,
            "SOURCE_SKELETON_FIRST_ACTIVE_PLAYER"
        );
    }
}

#[test]
fn skeleton_pending_raw_player_birth_preserves_public_adapter_visibility() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut g = Game::shell(7, rules);
        g.player.pos = Vec2::new(200.0, 152.0);
        g.player.engine_active = false;
        let target = g.world.spawn("oPlayer1", Vec2::new(40.0, 152.0));
        assert!(g.world.entity(target).unwrap().engine_active);
        let id = g.spawn("oSkeleton", Vec2::new(128.0, 144.0));
        assert_eq!(
            g.world.entity(id).unwrap().facing,
            -1,
            "SOURCE_SKELETON_PENDING_PLAYER"
        );
    }
}

#[test]
fn skeleton_retired_or_inactive_player_targets_do_not_count() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for retired in [false, true] {
            let mut g = Game::shell(7, rules);
            g.player.pos = Vec2::new(200.0, 152.0);
            g.player.engine_active = false;
            let target = g.spawn("oPlayer1", Vec2::new(40.0, 152.0));
            let e = g.world.entity_mut(target).unwrap();
            if retired {
                e.alive = false;
            } else {
                e.engine_active = false;
            }
            let id = g.spawn("oSkeleton", Vec2::new(128.0, 144.0));
            assert_eq!(g.world.entity(id).unwrap().facing, 1);
        }
    }
}
