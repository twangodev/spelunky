//! UNRUN source-only proposal. Authored integral initial geometry; default Idol
//! and TrapBlock Creates; actual neutral Right Draw then Down+Attack pickup.
//! No actor, field, animation, transform, alarm, or RNG writes after bootstrap.
use spelunky_core::*;

const PRESETS: [RulesPreset; 2] = [RulesPreset::Classic, RulesPreset::Assisted];
const AREAS: [Area; 2] = [Area::Jungle, Area::Temple];

struct Scene {
    game: Game,
    idol: u64,
    traps: Vec<u64>,
    player_x: f64,
    area: Area,
}

fn scene(preset: RulesPreset, area: Area, player_x: f64, trap_xs: &[f64]) -> Scene {
    let mut game = Game::shell(42, preset);
    game.world = World::new(20, 20, area, if area == Area::Jungle { 5 } else { 13 });
    game.world.name = "rLevel".into();
    for x in 0..game.world.width {
        game.world.set_tile(x, 10, "oHardBlock");
    }
    game.player.pos = Vec2::new(player_x, 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.player.facing = 1;
    game.world.entrance = game.player.pos;
    assert_eq!(game.player.sprite, "sStandLeft");
    assert!(game.player.equipment.is_empty());
    assert_eq!(game.player.held, None);
    assert_eq!(
        game.player.data.get("image_xscale").copied().unwrap_or(1.0),
        1.0
    );
    assert_eq!(
        game.player.data.get("image_angle").copied().unwrap_or(0.0),
        0.0
    );

    let idol = game.spawn("oGoldIdol", Vec2::new(player_x, 156.0));
    let idol_entity = game.world.entity(idol).unwrap();
    assert_eq!(
        idol_entity.value("cost"),
        0.0,
        "source inherited Item Create is free by default"
    );
    assert_eq!(idol_entity.value("forSale"), 0.0);
    assert_eq!(idol_entity.value("trigger"), 1.0);
    assert_eq!(idol_entity.value("canPickUp"), 1.0);
    assert!(!idol_entity.held);
    assert_eq!(idol_entity.vel, Vec2::default());

    let traps: Vec<_> = trap_xs
        .iter()
        .map(|x| {
            let id = game.spawn("oTrapBlock", Vec2::new(*x, 144.0));
            let trap = game.world.entity(id).unwrap();
            assert_eq!(trap.sprite, "sSkullBlock");
            assert_eq!(trap.state, 0);
            assert_eq!(trap.timer, 0);
            assert_eq!(trap.value("dying"), 0.0);
            assert_eq!(
                trap.value("deathTimer"),
                0.0,
                "own Create default; no shortened delay injection"
            );
            assert_eq!(trap.value("cleanDeath"), 0.0);
            id
        })
        .collect();
    assert!(
        !game
            .world
            .entities
            .iter()
            .any(|e| catalog::is_a(&e.object, "oCeilingTrap"))
    );
    assert!(!game.flags.contains("cleanSolids"));
    assert_eq!(game.data.get("cleanSolids").copied().unwrap_or(0.0), 0.0);
    assert!(!game.flags.contains("cemetery") && !game.flags.contains("cemetary"));

    // Independent source aggregate constants. No production mask/query helper.
    let duck = catalog::sprite_def("sDuckLeft").unwrap();
    assert_eq!((duck.origin, duck.bounds), ([8, 8], [1, 7, 15, 15]));
    let skull = catalog::sprite_def("sSkullBlock").unwrap();
    assert_eq!((skull.origin, skull.bounds), ([0, 0], [0, 0, 15, 15]));
    Scene {
        game,
        idol,
        traps,
        player_x,
        area,
    }
}

fn thumps(game: &Game) -> usize {
    game.events
        .iter()
        .filter(|event| matches!(event, GameEvent::Sound { name, .. } if name == "sndThump"))
        .count()
}

fn rubble(game: &Game) -> Vec<&Entity> {
    let mut pieces: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|entity| {
            entity.alive && matches!(entity.object.as_str(), "oRubble" | "oRubbleSmall")
        })
        .collect();
    pieces.sort_by_key(|entity| entity.creation_serial);
    pieces
}

fn neutral_then_pickup(s: &mut Scene) {
    s.game.step(&InputFrame::default());
    assert_eq!(s.game.player.pos, Vec2::new(s.player_x, 152.0));
    assert_eq!(s.game.player.state, PlayerState::Standing);
    assert_eq!(s.game.player.facing, 1);
    assert_eq!(
        s.game.player.data.get("image_xscale").copied(),
        Some(-1.0),
        "actual Right Draw persists the source mirror before pickup"
    );
    assert_eq!(s.game.player.held, None);
    let idol = s.game.world.entity(s.idol).unwrap();
    assert_eq!(idol.pos, Vec2::new(s.player_x, 156.0));
    assert_eq!(
        idol.value("colBot"),
        1.0,
        "ordinary inherited Item Step establishes support before trigger"
    );
    assert_eq!(idol.value("trigger"), 1.0);
    assert_eq!(idol.value("cost"), 0.0);
    assert_eq!(
        s.game.data.get("idols_grabbed").copied().unwrap_or(0.0),
        0.0
    );
    for id in &s.traps {
        let trap = s.game.world.entity(*id).unwrap();
        assert!(trap.engine_active);
        assert_eq!(trap.state, 0);
        assert_eq!(trap.timer, 0);
        assert_eq!(trap.value("dying"), 0.0);
        assert_eq!(trap.value("deathTimer"), 0.0);
    }
    assert_eq!(thumps(&s.game), 0);
    assert!(rubble(&s.game).is_empty());

    s.game.step(&InputFrame {
        down: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    // Establish actual producer prerequisites BEFORE interpreting any target effect.
    assert_eq!(s.game.player.pos, Vec2::new(s.player_x, 152.0));
    assert_eq!(s.game.player.state, PlayerState::Ducking);
    assert_eq!(s.game.player.sprite, "sDuckLeft");
    assert_eq!(s.game.player.data.get("image_xscale").copied(), Some(-1.0));
    assert_eq!(
        s.game.player.held,
        Some(s.idol),
        "actual Down+Attack selects the default GoldIdol"
    );
    assert_eq!(s.game.player.pickup_item_type, "Gold Idol");
    assert_eq!(s.game.player.health, 4.0);
    let idol = s.game.world.entity(s.idol).unwrap();
    assert!(idol.held);
    assert_eq!(idol.value("cost"), 0.0);
    assert_eq!(s.game.data.get("idols_grabbed").copied(), Some(1.0));
    assert_eq!(
        idol.value("colBot"),
        1.0,
        "held inherited Item Step preserves the prior supported contact before own Idol trigger checks"
    );
    assert_eq!(
        idol.value("trigger"),
        if s.area == Area::Jungle { 1.0 } else { 0.0 }
    );
}

fn assert_three_tan_pieces(s: &Scene) {
    let pieces = rubble(&s.game);
    assert_eq!(
        pieces.len(),
        3,
        "one default TrapBlock own Destroy creates three tan pieces"
    );
    assert_eq!(
        pieces.iter().map(|e| e.object.as_str()).collect::<Vec<_>>(),
        vec!["oRubble", "oRubbleSmall", "oRubbleSmall"]
    );
    assert_eq!(
        pieces.iter().map(|e| e.sprite.as_str()).collect::<Vec<_>>(),
        vec!["sRubbleTan", "sRubbleTanSmall", "sRubbleTanSmall"]
    );
    assert_eq!(
        thumps(&s.game),
        1,
        "Jungle dying Destroy or single Temple receiver body emits one Thump"
    );
}

#[test]
fn actual_idol_pickup_near_87_pixel_trap_is_a_positive_producer_control() {
    // P191: mirrored native Duck left182 minus Trap right95 =87 (<90).
    // Old smooth mirror left184 gives89 (<90), so both versions should collapse.
    for preset in PRESETS {
        for area in AREAS {
            let mut s = scene(preset, area, 191.0, &[80.0]);
            neutral_then_pickup(&mut s);
            assert!(s.game.world.entity(s.traps[0]).is_none());
            assert_three_tan_pieces(&s);
        }
    }
}

#[test]
fn actual_right_draw_then_idol_pickup_collapses_trap_at_native_89_not_smooth_91() {
    // P193: cached native Duck left184 minus Trap right95 =89 (<90).
    // Old smooth mirror left186 gives91, missing the source collapse.
    for preset in PRESETS {
        for area in AREAS {
            let mut s = scene(preset, area, 193.0, &[80.0]);
            neutral_then_pickup(&mut s);
            assert!(
                s.game.world.entity(s.traps[0]).is_none(),
                "native cached integer bbox89 is strictly inside Idol trap radius90"
            );
            assert_three_tan_pieces(&s);
        }
    }
}

#[test]
fn actual_idol_pickup_exact_native_90_pixel_trap_survives_and_records_distance() {
    // P194: native Duck left185 minus Trap right95 =90 (strictly excluded).
    // Old smooth left187 gives92; survival alone does not prove the geometry.
    for preset in PRESETS {
        for area in AREAS {
            let mut s = scene(preset, area, 194.0, &[80.0]);
            neutral_then_pickup(&mut s);
            let trap = s
                .game
                .world
                .entity(s.traps[0])
                .expect("source <90 excludes exact90");
            assert_eq!(trap.value("dying"), 0.0);
            assert_eq!(trap.value("deathTimer"), 0.0);
            assert_eq!(trap.state, 0);
            assert_eq!(trap.timer, 0);
            assert_eq!(
                trap.data.get("dist").copied(),
                Some(90.0),
                "source with body records dist even when its strict branch does not run"
            );
            assert!(rubble(&s.game).is_empty());
            assert_eq!(
                thumps(&s.game),
                usize::from(area == Area::Temple),
                "Temple sound lies outside the radius gate; Jungle dying sound does not"
            );
        }
    }
}

#[test]
fn actual_temple_idol_pickup_emits_one_thump_per_far_receiver_and_none_without_receivers() {
    for preset in PRESETS {
        for trap_xs in [&[80.0, 64.0][..], &[][..]] {
            let mut s = scene(preset, Area::Temple, 194.0, trap_xs);
            neutral_then_pickup(&mut s);
            for id in &s.traps {
                let trap = s
                    .game
                    .world
                    .entity(*id)
                    .expect("both90 and106 are outside strict radius");
                assert_eq!(trap.value("dying"), 0.0);
                assert_eq!(trap.value("deathTimer"), 0.0);
            }
            assert!(rubble(&s.game).is_empty());
            assert_eq!(
                thumps(&s.game),
                trap_xs.len(),
                "source Temple with body emits even for far receivers, and has no caller-global Thump without any receiver"
            );
            for (id, expected) in s.traps.iter().zip([90.0, 106.0]) {
                assert_eq!(
                    s.game.world.entity(*id).unwrap().data.get("dist").copied(),
                    Some(expected)
                );
            }
        }
    }
}
