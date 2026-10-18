//! UFO own death creates Crash, credits, destroys, then continues its event.
//! Initial HP/state/frame cases are labelled controlled. The ordinary journey
//! uses a free authored SpringShoes pickup and default SEARCH UFO, then inputs.
use spelunky_core::*;
fn shell(legacy: bool, rules: RulesPreset) -> Game {
    if legacy {
        Game::legacy_shell(2, rules)
    } else {
        Game::shell(0, rules)
    }
}
fn room(legacy: bool, rules: RulesPreset) -> Game {
    let mut game = shell(legacy, rules);
    game.world = World::new(30, 20, Area::Ice, 9);
    game.world.name = "rLevel".into();
    for x in 0..30 {
        game.world.set_tile(x, 10, "oBlock");
    }
    game.player.pos = Vec2::new(104.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game
}
fn sounds(game: &Game, name: &str) -> usize {
    game.events
        .iter()
        .filter(|event| matches!(event, GameEvent::Sound { name: sound, .. } if sound == name))
        .count()
}
fn births<'a>(game: &'a Game, object: &str) -> Vec<&'a Entity> {
    game.world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == object)
        .collect()
}
fn controlled(
    legacy: bool,
    rules: RulesPreset,
    state: i32,
    animation: f64,
    dead: bool,
    alerted: bool,
) -> (Game, u64) {
    let mut game = room(legacy, rules);
    let id = game.spawn("oUFO", Vec2::new(96.0, 104.0));
    let ufo = game.world.entity_mut(id).unwrap();
    assert_eq!((ufo.health, ufo.state), (1, 0));
    // These source-reachable initial receiver fields deliberately isolate each
    // independent own-Step branch; they are not claimed natural producers.
    ufo.health = if dead { 0 } else { 1 };
    ufo.state = state;
    ufo.data.insert("alerted".into(), f64::from(alerted));
    if state == 2 {
        ufo.set_sprite("sUFOBlast");
    }
    ufo.animation = animation;
    (game, id)
}
fn credit(game: &Game) {
    assert_eq!(game.data.get("ufos"), Some(&1.0));
    assert_eq!(game.data.get("kills"), Some(&1.0));
    assert_eq!(game.lifetime_stats.enemy_kills[16], 1);
}
fn alert_case(legacy: bool) {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, id) = controlled(legacy, rules, 0, 0.0, true, false);
        let rng = game.rng.state_bits();
        game.step(&InputFrame::default());
        assert!(game.world.entity(id).is_none());
        credit(&game);
        assert_eq!(births(&game, "oUFOCrash").len(), 1);
        assert_eq!(sounds(&game, "sndAlien"), 1, "POST_DESTROY_ALERT_SOUND");
        assert_eq!(game.rng.state_bits(), rng, "NO_UFO_OWN_TAIL_RNG");
        assert_eq!(game.player.health, 4.0);
    }
}
fn charge_case(legacy: bool) {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, id) = controlled(legacy, rules, 1, 0.0, true, true);
        game.step(&InputFrame::default());
        assert!(game.world.entity(id).is_none());
        credit(&game);
        assert_eq!(
            sounds(&game, "sndLaserCharge"),
            1,
            "POST_DESTROY_ALIGNED_CHARGE"
        );
        assert!(
            births(&game, "oLaser").is_empty(),
            "DESTROY_TO_BLAST_IS_NOT_SAME_BRANCH"
        );
        assert_eq!(sounds(&game, "sndAlien"), 0);
        assert_eq!(game.player.health, 4.0);
    }
}
fn blast_case(legacy: bool, frame: f64, dead: bool) {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, id) = controlled(legacy, rules, 2, frame, dead, true);
        let expected_laser = usize::from(frame == 16.0);
        game.step(&InputFrame::default());
        assert_eq!(game.world.entity(id).is_none(), dead);
        if dead {
            credit(&game);
        } else {
            assert_eq!(game.data.get("kills"), None);
        }
        assert_eq!(births(&game, "oUFOCrash").len(), usize::from(dead));
        assert_eq!(
            births(&game, "oLaser").len(),
            expected_laser,
            "EXACT_FRAME16_LASER_BIRTH"
        );
        assert_eq!(
            sounds(&game, "sndLaser"),
            expected_laser,
            "EXACT_FRAME16_LASER_SOUND"
        );
        assert_eq!(sounds(&game, "sndLaserCharge"), 0);
        if expected_laser == 1 {
            let laser = births(&game, "oLaser")[0];
            assert_eq!(
                laser.pos,
                Vec2::new(104.0, 120.0),
                "LATER_LASER534_STEPS_AFTER_UFO518"
            );
            assert_eq!(laser.alarms[0], 1, "NEWBORN_MISSED_ALARM_PASS");
            assert_eq!(laser.vel, Vec2::new(0.0, 0.6));
        }
        assert_eq!(game.player.health, 4.0);
    }
}
fn ordinary_whip_case(legacy: bool) {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = room(legacy, rules);
        let shoes = game.spawn("oSpringShoes", game.player.pos);
        // An authored free item, matching the custom editor's free-item field.
        // Equipment is acquired by actual Down+Attack; no inventory injection.
        game.world
            .entity_mut(shoes)
            .unwrap()
            .data
            .insert("cost".into(), 0.0);
        let id = game.spawn("oUFO", Vec2::new(96.0, 80.0));
        assert_eq!(
            (
                game.world.entity(id).unwrap().health,
                game.world.entity(id).unwrap().state
            ),
            (1, 0)
        );
        let geometry: Vec<_> = game.world.tiles.iter().map(|t| t.object.clone()).collect();
        for _ in 0..8 {
            game.step(&InputFrame::default());
        }
        game.step(&InputFrame {
            down: true,
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert!(game.player.has("SpringShoes"), "ORDINARY_FREE_SHOES_PICKUP");
        assert_eq!(game.player.held, None);
        let mut blast = false;
        for _ in 0..180 {
            game.step(&InputFrame::default());
            if game.world.entity(id).is_some_and(|e| e.state == 2) {
                blast = true;
                break;
            }
        }
        assert!(blast, "DEFAULT_SEARCH_NATURALLY_ENTERS_BLAST");
        assert_eq!(game.world.entity(id).unwrap().animation, 5.0);
        for tick in 0..=21 {
            game.step(&InputFrame {
                left: tick < 4,
                right: tick == 4,
                jump: (7..27).contains(&tick),
                jump_pressed: tick == 7,
                attack: tick == 16,
                attack_pressed: tick == 16,
                ..Default::default()
            });
            let ufo = game.world.entity(id).expect("LIVE_UNTIL_OWN_NEXT_STEP");
            if tick < 21 {
                assert_eq!(ufo.health, 1);
            }
        }
        let ufo = game.world.entity(id).unwrap();
        assert_eq!(ufo.health, 0, "ORDINARY_WHIP_FATAL_HP");
        assert_eq!(ufo.state, 2);
        assert_eq!(ufo.animation, 16.0, "LITERAL_IMAGE_INDEX_ELIGIBILITY");
        assert_eq!(game.player.health, 4.0);
        assert!(births(&game, "oLaser").is_empty());
        assert!(births(&game, "oUFOCrash").is_empty());
        game.step(&InputFrame::default());
        assert!(game.world.entity(id).is_none());
        credit(&game);
        assert_eq!(births(&game, "oUFOCrash").len(), 1);
        assert_eq!(
            sounds(&game, "sndLaser"),
            1,
            "ORDINARY_FATAL_WHIP_STILL_FIRES_FRAME16"
        );
        assert_eq!(births(&game, "oLaser").len(), 1);
        assert_eq!(game.player.health, 4.0);
        assert_eq!(
            game.world
                .tiles
                .iter()
                .map(|t| t.object.clone())
                .collect::<Vec<_>>(),
            geometry
        );
    }
}
#[test]
fn canonical_controlled_death_alert() {
    alert_case(false);
}
#[test]
fn legacy_controlled_death_alert() {
    alert_case(true);
}
#[test]
fn canonical_controlled_death_charge() {
    charge_case(false);
}
#[test]
fn legacy_controlled_death_charge() {
    charge_case(true);
}
#[test]
fn canonical_controlled_death_frame16() {
    blast_case(false, 16.0, true);
}
#[test]
fn legacy_controlled_death_frame16() {
    blast_case(true, 16.0, true);
}
#[test]
fn canonical_frame15_half_negative_control() {
    blast_case(false, 15.5, true);
}
#[test]
fn legacy_frame15_half_negative_control() {
    blast_case(true, 15.5, true);
}
#[test]
fn canonical_live_frame16_positive_control() {
    blast_case(false, 16.0, false);
}
#[test]
fn legacy_live_frame16_positive_control() {
    blast_case(true, 16.0, false);
}
#[test]
fn canonical_ordinary_jump_whip_frame16_death() {
    ordinary_whip_case(false);
}
#[test]
fn legacy_ordinary_jump_whip_frame16_death() {
    ordinary_whip_case(true);
}
