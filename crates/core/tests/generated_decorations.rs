use spelunky_core::{
    Game, InputFrame, Rng, RulesPreset, Vec2, generation, hub::HubKind, level::Level,
};
use std::collections::BTreeSet;

fn flags(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|name| (*name).into()).collect()
}

#[test]
fn generated_tiki_has_source_body_depth_and_both_random_arm_crops() {
    let world = generation::generate(&mut Rng::new(0), 1, &flags(&[]));
    let head = world
        .entities
        .iter()
        .find(|e| e.object == "oGiantTikiHead")
        .unwrap();
    let origin = Vec2::new(head.pos.x - 16.0, head.pos.y - 12.0);
    let body = world
        .entities
        .iter()
        .find(|e| e.sprite == "bgTiki")
        .unwrap();
    assert_eq!(body.pos, Vec2::new(origin.x, origin.y + 32.0));
    assert_eq!(body.value("depth"), 10001.0);
    let arms: Vec<_> = world
        .entities
        .iter()
        .filter(|e| e.sprite == "bgTikiArms")
        .collect();
    assert_eq!(arms.len(), 2);
    for (arm, dx, source_y) in [(arms[0], 32.0, 0.0), (arms[1], -16.0, 16.0)] {
        assert_eq!(arm.pos, Vec2::new(origin.x + dx, origin.y + 32.0));
        assert!([0.0, 16.0, 32.0].contains(&arm.value("source_x")));
        assert_eq!(arm.value("source_y"), source_y);
        assert_eq!(arm.value("source_w"), 16.0);
        assert_eq!(arm.value("source_h"), 16.0);
        assert_eq!(arm.value("depth"), 10001.0);
    }
}

#[test]
fn generated_city_of_gold_mural_retains_each_source_crop_and_block_origin() {
    let world = generation::generate(&mut Rng::new(0), 13, &flags(&["city_of_gold"]));
    let fragments: Vec<_> = world
        .entities
        .iter()
        .filter(|e| e.sprite == "bgLadyXoc")
        .collect();
    assert_eq!(fragments.len(), 30);
    let mut crops = BTreeSet::new();
    for fragment in fragments {
        assert_eq!(fragment.value("depth"), 99.0);
        assert_eq!(fragment.value("source_w"), 16.0);
        assert_eq!(fragment.value("source_h"), 16.0);
        crops.insert((
            fragment.value("source_x") as i32,
            fragment.value("source_y") as i32,
        ));
        assert_eq!(
            world
                .tile(
                    (fragment.pos.x / 16.0) as i32,
                    (fragment.pos.y / 16.0) as i32
                )
                .unwrap()
                .object,
            "oXocBlock"
        );
    }
    assert_eq!(
        crops,
        (0..5)
            .flat_map(|x| (0..6).map(move |y| (x * 16, y * 16)))
            .collect()
    );
}

#[test]
fn generated_shop_wanted_posters_use_source_gate_and_character_crops() {
    for (character, source_x) in [
        (None, 0.0),
        (Some("character_damsel"), 32.0),
        (Some("character_tunnelman"), 64.0),
    ] {
        for wanted in ["murderer", "wanted"] {
            let mut selected = flags(&["city_of_gold", wanted]);
            if let Some(character) = character {
                selected.insert(character.into());
            }
            let world = generation::generate(&mut Rng::new(1), 13, &selected);
            let posters: Vec<_> = world
                .entities
                .iter()
                .filter(|e| e.sprite == "bgWanted")
                .collect();
            assert!(!posters.is_empty(), "authored shop fixture must exercise W");
            for poster in posters {
                assert_eq!(poster.value("source_x"), source_x);
                assert_eq!(poster.value("source_y"), 0.0);
                assert_eq!(poster.value("source_w"), 32.0);
                assert_eq!(poster.value("source_h"), 32.0);
                assert_eq!(poster.value("depth"), 9004.0);
            }
        }
    }
    let clean = generation::generate(&mut Rng::new(1), 13, &flags(&["city_of_gold"]));
    assert!(!clean.entities.iter().any(|e| e.sprite == "bgWanted"));
}

#[test]
fn generated_ice_underside_reaches_real_drip_alarm_behavior() {
    let world = generation::generate(&mut Rng::new(0), 9, &flags(&[]));
    let bottom = world
        .entities
        .iter()
        .find(|e| e.object == "oIceBottom")
        .unwrap()
        .clone();
    assert_eq!(
        world
            .tile(
                (bottom.pos.x / 16.0) as i32,
                (bottom.pos.y / 16.0) as i32 - 1
            )
            .unwrap()
            .object,
        "oIce"
    );
    assert!((20.0..=400.0).contains(&bottom.value("drip_alarm")));
    let mut game = Game::shell(0, RulesPreset::Classic);
    game.player.pos = Vec2::new(bottom.pos.x + 8.0, bottom.pos.y + 24.0);
    game.player.previous_pos = game.player.pos;
    game.camera = Vec2::new(
        (bottom.pos.x - 160.0).max(0.0),
        (bottom.pos.y - 100.0).max(0.0),
    );
    game.player.invincible = 100;
    game.world = world;
    game.world
        .entity_mut(bottom.id)
        .unwrap()
        .data
        .insert("drip_alarm".into(), 1.0);
    game.step(&InputFrame::default());
    assert!(game.world.entity(bottom.id).unwrap().alive);
    assert!(
        game.world
            .entities
            .iter()
            .any(|e| e.object == "oDrip"
                && e.pos == Vec2::new(bottom.pos.x + 8.0, bottom.pos.y + 4.0))
    );
}

#[test]
fn generated_fake_temple_doors_receive_setup_sprites_without_solidifying() {
    let world = generation::generate(&mut Rng::new(1), 13, &flags(&[]));
    let panels: Vec<_> = world
        .entities
        .iter()
        .filter(|e| e.object == "oTempleFake")
        .collect();
    assert_eq!(panels.len(), 4);
    assert!(
        panels
            .iter()
            .all(|e| e.value("fake_setup_initialized") == 1.0)
    );
    assert!(panels.iter().any(|e| e.sprite != "sTemple"));
    assert!(
        panels
            .iter()
            .all(|e| !spelunky_core::catalog::is_solid(&e.object))
    );
}

#[test]
fn generated_default_high_graphics_emits_source_fringes_and_twenty_extras() {
    for (level, background) in [
        (1, "bgCaveTop"),
        (5, "bgCaveTop2"),
        (9, "bgCaveTop3"),
        (13, "bgCaveTop4"),
    ] {
        let world = generation::generate(&mut Rng::new(0), level, &flags(&[]));
        let fringes: Vec<_> = world
            .entities
            .iter()
            .filter(|e| e.sprite == background)
            .collect();
        assert!(!fringes.is_empty(), "level {level}");
        for fringe in fringes {
            assert_eq!(fringe.value("depth"), 3.0);
            assert_eq!(fringe.value("source_w"), 16.0);
            assert_eq!(fringe.value("source_h"), 16.0);
            assert_eq!(fringe.value("source_y"), 0.0);
        }
        let extras: Vec<_> = world
            .entities
            .iter()
            .filter(|e| e.sprite.starts_with("bgExtras"))
            .collect();
        assert_eq!(extras.len(), 20, "level {level}");
        for extra in extras {
            assert_eq!(extra.value("depth"), 10002.0);
            assert_eq!(extra.value("source_w"), 32.0);
            assert_eq!(extra.value("source_h"), 32.0);
            assert!([0.0, 32.0].contains(&extra.value("source_x")));
            assert!((16.0..=672.0).contains(&extra.pos.x));
            assert!((16.0..=528.0).contains(&extra.pos.y));
        }
        let low = generation::generate(&mut Rng::new(0), level, &flags(&["low_graphics"]));
        assert!(
            !low.entities
                .iter()
                .any(|e| e.sprite.starts_with("bgCaveTop") || e.sprite.starts_with("bgExtras"))
        );
    }
}

#[test]
fn authored_hubs_emit_default_high_fringes_and_custom_rooms_skip_setup() {
    for kind in [
        HubKind::Title,
        HubKind::Scores,
        HubKind::Sun,
        HubKind::Moon,
        HubKind::Stars,
    ] {
        let mut game = Game::shell(0, RulesPreset::Classic);
        game.load_hub(kind);
        assert!(
            game.world.entities.iter().any(|e| e.sprite == "bgCaveTop"),
            "{kind:?}"
        );
        game.flags.insert("low_graphics".into());
        game.load_hub(kind);
        assert!(
            !game.world.entities.iter().any(|e| e.sprite == "bgCaveTop"),
            "{kind:?}"
        );
    }
    let mut game = Game::shell(0, RulesPreset::Classic);
    let mut level = Level::default();
    level.set_tile(4, 4, '1').unwrap();
    game.load_custom(&level, "no-setup.lvl", "");
    assert!(
        !game
            .world
            .entities
            .iter()
            .any(|e| e.sprite.starts_with("bgCaveTop") || e.sprite.starts_with("bgExtras"))
    );
}
