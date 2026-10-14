//! Declare as a dispatcher child cfg(test) module; uses real existing
//! membership implementation, no production visibility/API change.
use super::*;
use crate::{RulesPreset, Vec2};

#[test]
fn skeleton_create_reads_first_player_after_actual_deactivation_and_reactivation() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut g = Game::shell(7, rules);
        g.player.pos = Vec2::new(40.0, 152.0);
        let other = g.spawn("oPlayer1", Vec2::new(160.0, 152.0));
        assert_eq!(
            descendant_instances(&g, "oPlayer1"),
            vec![Instance::Player, Instance::Entity(other)]
        );
        // Actual shared region-deactivation operation; no engine_active write.
        region_change(&mut g, Rect::new(32.0, 144.0, 16.0, 16.0), false, false);
        assert!(!g.player.engine_active);
        assert_eq!(
            descendant_instances(&g, "oPlayer1"),
            vec![Instance::Entity(other)]
        );
        assert!(g.dispatcher.inactive_order.contains(&Instance::Player));
        activate_region(&mut g, Rect::new(32.0, 144.0, 16.0, 16.0));
        assert!(g.player.engine_active);
        assert_eq!(
            descendant_instances(&g, "oPlayer1"),
            vec![Instance::Entity(other), Instance::Player]
        );
        let id = g.spawn("oSkeleton", Vec2::new(128.0, 144.0));
        assert_eq!(
            g.world.entity(id).unwrap().facing,
            1,
            "SOURCE_SKELETON_REAL_REACTIVATION_GETTER"
        );
    }
}
