//! UNRUN proposal: real Up+Attack opens a default FlareCrate in a dark room.
//! An independent inclusive aggregate bbox oracle checks each Flare's later
//! Step cache and Level's earlier consumption of the preceding cache.
//! No actor, cache, transform, alarm, or RNG writes after authored bootstrap.
use spelunky_core::*;

#[derive(Clone, Copy)]
struct Bbox {
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
}

// Source cached signed integer aggregate edges, not current-frame smooth
// bounds. This scene retains positive unit image scales and zero image angle.
fn aggregate(sprite: &str, pos: Vec2) -> Bbox {
    let def = catalog::sprite_def(sprite).expect("bundled source sprite");
    let [left, top, right, bottom] = def.bounds;
    Bbox {
        left: (pos.x + f64::from(left - def.origin[0])).round_ties_even(),
        top: (pos.y + f64::from(top - def.origin[1])).round_ties_even(),
        right: (pos.x + f64::from(right - def.origin[0])).round_ties_even(),
        bottom: (pos.y + f64::from(bottom - def.origin[1])).round_ties_even(),
    }
}

fn smooth_aggregate(sprite: &str, pos: Vec2) -> Bbox {
    let def = catalog::sprite_def(sprite).unwrap();
    let [left, top, right, bottom] = def.bounds;
    Bbox {
        left: pos.x + f64::from(left - def.origin[0]),
        top: pos.y + f64::from(top - def.origin[1]),
        right: pos.x + f64::from(right - def.origin[0]),
        bottom: pos.y + f64::from(bottom - def.origin[1]),
    }
}

fn inclusive_gap(a: Bbox, b: Bbox) -> f64 {
    let dx = (a.left - b.right).max(b.left - a.right).max(0.0);
    let dy = (a.top - b.bottom).max(b.top - a.bottom).max(0.0);
    (dx * dx + dy * dy).sqrt()
}

fn scalar(data: &std::collections::BTreeMap<String, f64>, key: &str, default: f64) -> f64 {
    data.get(key).copied().unwrap_or(default)
}

fn positive_unit(data: &std::collections::BTreeMap<String, f64>) {
    assert_eq!(scalar(data, "image_xscale", 1.0), 1.0);
    assert_eq!(scalar(data, "image_yscale", 1.0), 1.0);
    assert_eq!(scalar(data, "image_angle", 0.0), 0.0);
}

fn modes() -> impl Iterator<Item = (RulesPreset, RngAlgorithm)> {
    [RulesPreset::Classic, RulesPreset::Assisted]
        .into_iter()
        .flat_map(|rules| {
            [RngAlgorithm::Gm8Lcg32V1, RngAlgorithm::LegacySplitMix64V1]
                .into_iter()
                .map(move |algorithm| (rules, algorithm))
        })
}

fn dark_room(rules: RulesPreset, algorithm: RngAlgorithm) -> (Game, u64) {
    let mut game = Game::shell_with_algorithm(42, rules, algorithm).unwrap();
    game.world = World::new(80, 30, Area::Mines, 1);
    game.world.name = "rLevel".into();
    game.world.dark = true;
    for x in 0..game.world.width {
        game.world.set_tile(x, 10, "oHardBlock");
    }
    // Fractional authored origin exposes independently rounded .5 bbox edges.
    // Facing left naturally retains +1 image_xscale in subsequent Draw phases.
    game.player.pos = Vec2::new(400.5, 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.player.facing = -1;
    game.world.entrance = game.player.pos;
    assert!(game.player.equipment.is_empty());
    let crate_id = game.spawn("oFlareCrate", game.player.pos);
    let crate_entity = game.world.entity(crate_id).unwrap();
    assert_eq!(crate_entity.alarms[0], 1);
    assert_eq!(crate_entity.pos, game.player.pos);
    assert_eq!(crate_entity.vel.y, 0.0);
    assert_eq!(scalar(&game.data, "darkness_lerp", 0.0), 0.0);
    (game, crate_id)
}

fn flare_ids(game: &Game) -> Vec<u64> {
    dispatcher::descendant_instances(game, "oFlare")
        .into_iter()
        .filter_map(|instance| match instance {
            dispatcher::Instance::Entity(id) => game.world.entity(id),
            _ => None,
        })
        .filter(|entity| entity.alive && entity.engine_active)
        .map(|entity| entity.id)
        .collect()
}

fn open(game: &mut Game, crate_id: u64) -> Vec<u64> {
    let origin = game.player.pos;
    let mut expected_rng = game.rng.clone();
    // Existing crate alarm precedes Player's opening: four rand(0,3) draws.
    for _ in 0..4 {
        let _ = expected_rng.range(4);
    }
    // Three direct Flare Creates have no RNG; producer velocities draw nine.
    for _ in 0..3 {
        let _ = expected_rng.range(4);
        let _ = expected_rng.range(4);
        let _ = expected_rng.range(3);
    }
    game.step(&InputFrame {
        up: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert!(game.world.entity(crate_id).is_none());
    assert_eq!(game.player.held, None);
    assert_eq!(game.player.health, 4);
    assert_eq!(game.rng.clone().next_u64(), expected_rng.next_u64());
    let ids = flare_ids(game);
    assert_eq!(ids.len(), 3, "real Up+Attack births three source flares");
    for id in &ids {
        let flare = game.world.entity(*id).unwrap();
        assert!(!flare.held);
        assert_eq!(flare.sprite, "sFlare");
        assert_eq!(flare.previous_pos, origin);
        assert_eq!(flare.alarms[0], 1, "newborn missed earlier alarm pass");
        assert_eq!(flare.value("invincible"), 1.0);
        positive_unit(&flare.data);
    }
    assert_eq!(
        game.data["darkness"], 0.0,
        "Level553 sees new Flare cache0 before Flare629 refreshes it"
    );
    ids
}

fn check_after_flare_step(game: &Game, ids: &[u64]) -> bool {
    positive_unit(&game.player.data);
    let player = aggregate(&game.player.sprite, game.player.pos);
    let smooth_player = smooth_aggregate(&game.player.sprite, game.player.pos);
    let mut distinguishes_smooth = false;
    for id in ids {
        let flare = game
            .world
            .entity(*id)
            .expect("short dry ordinary scene keeps Flare alive");
        assert!(flare.engine_active);
        positive_unit(&flare.data);
        let expected = inclusive_gap(aggregate(&flare.sprite, flare.pos), player);
        let smooth = inclusive_gap(smooth_aggregate(&flare.sprite, flare.pos), smooth_player);
        assert!(
            (flare.value("dist_to_player") - expected).abs() < 1e-12,
            "Flare ownStep after inherited movement must cache integer aggregate inclusive bbox distance: pos={:?}, player={:?}, expected={expected}, smooth={smooth}",
            flare.pos,
            game.player.pos
        );
        distinguishes_smooth |= (expected - smooth).abs() > 1e-9;
    }
    distinguishes_smooth
}

#[test]
fn ordinary_flare_crate_open_and_left_motion_cache_integer_aggregate_bbox_distance() {
    for (rules, algorithm) in modes() {
        let (mut game, crate_id) = dark_room(rules, algorithm);
        let ids = open(&mut game, crate_id);
        let mut integer_witness = check_after_flare_step(&game, &ids);
        let mut separate_witness = false;
        for _ in 0..20 {
            game.step(&InputFrame {
                left: true,
                run: true,
                ..Default::default()
            });
            integer_witness |= check_after_flare_step(&game, &ids);
            separate_witness |= ids
                .iter()
                .any(|id| game.world.entity(*id).unwrap().value("dist_to_player") > 0.0);
        }
        assert!(
            integer_witness,
            "ordinary fractional-origin geometry must distinguish integer bbox from smooth distance"
        );
        assert!(
            separate_witness,
            "normal left motion must separate player and emitted flares"
        );
    }
}

#[test]
fn ordinary_flare_crate_darkness_consumes_prior_cache_selected_before_flare_motion() {
    for (rules, algorithm) in modes() {
        let (mut game, crate_id) = dark_room(rules, algorithm);
        let ids = open(&mut game, crate_id);
        let mut stale_cache_witness = false;
        for _ in 0..20 {
            let prior: Vec<_> = ids
                .iter()
                .map(|id| game.world.entity(*id).unwrap().clone())
                .collect();
            let mut expected_rng = game.rng.clone();
            for flare in &prior {
                if flare.alarms[0] == 1 {
                    for _ in 0..4 {
                        let _ = expected_rng.range(4);
                    }
                }
            }
            game.step(&InputFrame {
                left: true,
                run: true,
                ..Default::default()
            });
            // Player319 has moved when Level553 chooses nearest ORIGIN. Flare629
            // has not moved yet; its distance cache is still from the prior Step.
            let mut selected = None;
            let mut squared = f64::INFINITY;
            for flare in &prior {
                let dx = flare.pos.x - game.player.pos.x;
                let dy = flare.pos.y - game.player.pos.y;
                let candidate = dx * dx + dy * dy;
                if candidate < squared {
                    squared = candidate;
                    selected = Some(flare);
                }
            }
            let selected = selected.unwrap();
            let cached = selected.value("dist_to_player");
            let expected_darkness = (cached / 160.0).min(0.9);
            assert_eq!(
                game.player.data["dist_to_nearest_light_source"], 999.0,
                "crate retired; flares belong only to Level's later cache path"
            );
            assert_eq!(game.data["darkness"], expected_darkness);
            assert_eq!(
                game.rng.clone().next_u64(),
                expected_rng.next_u64(),
                "three due Flare alarms consume twelve draws; lighting queries consume none"
            );
            let fresh = inclusive_gap(
                aggregate("sFlare", game.world.entity(selected.id).unwrap().pos),
                aggregate(&game.player.sprite, game.player.pos),
            );
            stale_cache_witness |= (fresh / 160.0).min(0.9) != expected_darkness;
            assert_eq!(scalar(&game.data, "darkness_lerp", 0.0), 0.0);
            assert_eq!(game.player.health, 4);
        }
        assert!(
            stale_cache_witness,
            "ordinary motion must distinguish Level's prior cache from the following Flare refresh"
        );
    }
}
