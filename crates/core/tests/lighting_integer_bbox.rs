//! Source distance kernel reads inclusive cached aggregate integer bboxes.
//! Authored initial rooms and ordinary Game.step only; no runtime state writes.
//! Numeric oracles below do not call the production distance/mask helpers.
use spelunky_core::*;
const PRESETS: [RulesPreset; 2] = [RulesPreset::Classic, RulesPreset::Assisted];
fn scene(rules: RulesPreset, pos: Vec2) -> Game {
    let mut g = Game::shell(7, rules);
    g.world = World::new(20, 20, Area::Mines, 1);
    g.flags.insert("custom".into());
    g.world.dark = true;
    g.player.pos = pos;
    g.player.previous_pos = pos;
    g.player.state = PlayerState::Standing;
    g.player.grounded = true;
    g.world.entrance = pos;
    g
}
fn gap(a: [f64; 4], b: [f64; 4]) -> f64 {
    let dx = (b[0] - a[2]).max(a[0] - b[2]).max(0.);
    let dy = (b[1] - a[3]).max(a[1] - b[3]).max(0.);
    dx.hypot(dy)
}
#[test]
fn neutral_dark_room_lamp_uses_aggregate_bounds_at_integer_and_fractional_origins() {
    for rules in PRESETS {
        for (px, lx, expected) in [(40., 80., 36.), (40.5, 80.25, 35.), (40.25, 80.5, 37.)] {
            let mut g = scene(rules, Vec2::new(px, 88.));
            for x in 0..20 {
                g.world.set_tile(x, 6, "oHardBlock");
            }
            let lamp = g.spawn("oLamp", Vec2::new(lx, 80.));
            assert_eq!(g.player.sprite, "sStandLeft");
            assert_eq!(g.world.entity(lamp).unwrap().animation, 0.);
            // Aggregate sStandLeft [2,0,13,15], origin8,8; aggregate Lamp[1,0,14,15].
            let player = [
                (px - 6.).round_ties_even(),
                80.,
                (px + 5.).round_ties_even(),
                95.,
            ];
            let source = [
                (lx + 1.).round_ties_even(),
                80.,
                (lx + 14.).round_ties_even(),
                95.,
            ];
            assert_eq!(gap(player, source), expected);
            let mut rng = g.rng.clone();
            g.step(&InputFrame::default());
            assert_eq!(g.player.health, 4);
            assert_eq!(g.player.pos.x, px);
            assert_eq!(g.player.data["dist_to_nearest_light_source"], expected);
            assert_eq!(g.data["darkness"], expected / 160.);
            assert_eq!(g.rng.clone().next_u64(), rng.next_u64());
            assert_eq!(g.world.entity(lamp).unwrap().animation, 0.5);
        }
    }
}
#[test]
fn ordinary_contained_lava_surface_uses_aggregate_tile_bounds_before_player_motion() {
    for rules in PRESETS {
        let mut g = scene(rules, Vec2::new(88., 64.));
        g.player.grounded = false;
        g.player.state = PlayerState::Falling;
        // Initial contained pool geometry; Game's ordinary water pass selects sLavaTop.
        g.world.set_tile(4, 5, "oHardBlock");
        g.world.set_tile(6, 5, "oHardBlock");
        g.world.set_tile(5, 6, "oHardBlock");
        // Initial host terrain replacement requests the source water pass.
        // Filling empty storage alone does not mark fluid_dirty in this port.
        g.world.set_tile(5, 5, "oBlock");
        g.world.set_tile(5, 5, "oLava");
        assert!(g.world.fluid_dirty);
        assert_eq!(g.player.sprite, "sStandLeft");
        assert_eq!(gap([82., 56., 93., 71.], [80., 80., 95., 95.]), 9.);
        let mut rng = g.rng.clone();
        rng.range(201);
        g.step(&InputFrame::default());
        assert_eq!(g.world.tile(5, 5).unwrap().object, "oLava");
        let surface = g
            .world
            .entities
            .iter()
            .find(|e| e.alive && e.object == "oTerrainMetadata" && e.pos == Vec2::new(88., 88.))
            .expect("ordinary exposed-fluid pass produces surface metadata");
        assert_eq!(
            surface.text.get("terrain_sprite").map(String::as_str),
            Some("sLavaTop")
        );
        assert_eq!(g.player.health, 4);
        assert_eq!(g.player.data["dist_to_nearest_light_source"], 9.);
        assert_eq!(g.data["darkness"], 9. / 160.);
        assert_eq!(
            g.rng.clone().next_u64(),
            rng.next_u64(),
            "one source Lava spurtTime Create draw; distance query has no RNG"
        );
    }
}
#[test]
fn integer_light_cache_is_sampled_in_bright_rooms_without_overwriting_darkness() {
    for rules in PRESETS {
        let mut g = scene(rules, Vec2::new(40., 88.));
        g.world.dark = false;
        g.data.insert("darkness".into(), 0.7);
        for x in 0..20 {
            g.world.set_tile(x, 6, "oHardBlock");
        }
        g.spawn("oLamp", Vec2::new(80., 80.));
        let mut rng = g.rng.clone();
        g.step(&InputFrame::default());
        assert_eq!(g.player.data["dist_to_nearest_light_source"], 36.);
        assert_eq!(g.data["darkness"], 0.7);
        assert_eq!(g.rng.clone().next_u64(), rng.next_u64());
    }
}
