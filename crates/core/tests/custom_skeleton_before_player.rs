//! Stock custom K producer; ordinary neutral input after room loading.
//! The authored scene is controlled; no campaign frequency is claimed.
use spelunky_core::{Game, InputFrame, RulesPreset, RunState, Vec2, level::Level};

#[test]
fn custom_skeleton_create_before_later_player_keeps_right_on_first_step() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        // Source scrLoadLevel builds ALL tile objects before creating Player.
        // Mirror neither geometry nor behavior: two authored entrance positions
        // establish left-of-K and right-of-K controls with one default Skeleton.
        for (entrance_x, skeleton_x) in [(2, 7), (7, 2)] {
            let mut room = Level::default();
            for x in 0..40 {
                room.set_tile(x, 9, '1').unwrap();
            }
            room.set_tile(entrance_x, 8, '@').unwrap();
            room.set_tile(skeleton_x, 8, 'K').unwrap();
            let mut g = Game::shell(7, rules);
            g.load_custom(&room, "skeleton-create-before-player.lvl", "");
            let expected_pos = Vec2::new(16.0 + skeleton_x as f64 * 16.0, 144.0);
            let id = g
                .world
                .entities
                .iter()
                .find(|e| e.alive && e.object == "oSkeleton")
                .unwrap()
                .id;
            let initial = g.world.entity(id).unwrap();
            assert_eq!(initial.pos, expected_pos);
            assert_eq!(
                (
                    initial.facing,
                    initial.health,
                    initial.state,
                    initial.timer,
                    initial.age
                ),
                (1, 1, 0, 20, 0)
            );
            assert_eq!(
                g.player.pos,
                Vec2::new(24.0 + entrance_x as f64 * 16.0, 152.0)
            );
            assert!(g.player.engine_active);
            assert_eq!(g.player.health, 4.0);
            // No writes after setup; the first real Step decrements counter,
            // but source IDLE has no facing assignment. The constructor's
            // no-Player RIGHT result must survive the later Player birth.
            g.step(&InputFrame::default());
            let e = g.world.entity(id).unwrap();
            assert_eq!(e.facing, 1, "SOURCE_CUSTOM_K_BEFORE_PLAYER");
            assert_eq!(e.pos, expected_pos);
            assert_eq!((e.health, e.state, e.timer), (1, 0, 19));
            assert_eq!(g.state, RunState::Playing);
            assert_eq!(g.player.health, 4.0);
        }
    }
}
