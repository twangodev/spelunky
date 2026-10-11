//! Source characterStepEvent295-298: point gate, then full-mask instance_place.
//! Controlled authored positions; only ordinary inputs after the first Step.
use spelunky_core::{Area, Game, InputFrame, PlayerState, RulesPreset, Vec2, World, characters};

fn scene(
    rules: RulesPreset,
    character: characters::Character,
    first: Vec2,
    reverse: bool,
    facing_right: bool,
) -> (Game, u64, u64) {
    let mut game = Game::shell(42, rules);
    game.world = World::new(24, 20, Area::Mines, 1);
    for x in 0..24 {
        game.world.set_tile(x, 10, "oBlock");
    }
    characters::apply(&mut game, character);
    game.player.pos = Vec2::new(95.0, 152.0);
    game.player.previous_pos = game.player.pos;
    let first = game.spawn("oWeb", first);
    let second = game.spawn("oWeb", Vec2::new(80.0, 144.0));
    if reverse {
        game.world.entities.reverse();
    }
    // Source grounded state and current standing sprite arise from a real
    // controller/Draw, rather than a test writing a pose at the contact tick.
    game.step(&InputFrame {
        left: !facing_right,
        ..Default::default()
    });
    assert_eq!(game.player.facing, if facing_right { 1 } else { -1 });
    assert_eq!(
        game.player.data.get("image_xscale"),
        Some(&if facing_right { -1.0 } else { 1.0 })
    );
    assert_eq!(game.player.pos, Vec2::new(95.0, 152.0));
    assert!(game.player.grounded);
    assert_eq!(game.player.state, PlayerState::Standing);
    assert_eq!(game.player.data.get("in_web"), Some(&1.0));
    assert_eq!(life(&game, first), 12.0);
    assert_eq!(life(&game, second), 12.0);
    (game, first, second)
}

fn life(game: &Game, id: u64) -> f64 {
    let web = game.world.entity(id).expect("source Web survived");
    assert!(web.alive && web.engine_active);
    assert_eq!(web.object, "oWeb");
    web.value("life")
}

fn matrix(mut run: impl FnMut(RulesPreset, characters::Character, bool, bool)) {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for character in [
            characters::Character::Explorer,
            characters::Character::Damsel,
            characters::Character::TunnelMan,
        ] {
            for reverse in [false, true] {
                for facing_right in [false, true] {
                    run(rules, character, reverse, facing_right);
                }
            }
        }
    }
}

fn escape(game: &mut Game) {
    game.step(&InputFrame {
        jump: true,
        jump_pressed: true,
        left_released: game.player.facing < 0,
        ..Default::default()
    });
    assert_eq!(game.player.state, PlayerState::Jumping);
    assert!(game.player.vel.y < 0.0);
    assert!(game.player.pos.y < 152.0);
    assert!(!game.player.is_stunned());
}

#[test]
fn natural_web_jump_skips_first_web_transparent_corner() {
    matrix(|rules, character, reverse, facing_right| {
        let (mut game, first, second) = scene(
            rules,
            character,
            Vec2::new(if facing_right { 101.0 } else { 100.0 }, 159.0),
            reverse,
            facing_right,
        );
        // Archived sWeb precise mask: local (0,0) is clear. It is the sole
        // intersection of the source standing mask with the first Web after
        // the real Draw establishes +1 Left or -1 Right image_xscale.
        let first_web = game.world.entity(first).unwrap();
        assert!(first_web.mask_contains(
            Vec2::new(if facing_right { 101.0 } else { 100.0 }, 159.0),
            spelunky_core::CollisionPrecision::BoundingBox
        ));
        assert!(!first_web.mask_contains(
            Vec2::new(if facing_right { 101.0 } else { 100.0 }, 159.0),
            spelunky_core::CollisionPrecision::Precise
        ));
        let mut control = game.clone();
        control.step(&InputFrame::default());
        assert_eq!(life(&control, first), 12.0);
        assert_eq!(life(&control, second), 12.0);
        escape(&mut game);
        assert_eq!(
            life(&game, first),
            12.0,
            "transparent corner consumed: {rules:?} {character:?} reverse={reverse}"
        );
        assert_eq!(
            life(&game, second),
            11.0,
            "source mask target not consumed: {rules:?} {character:?} reverse={reverse}"
        );
    });
}

#[test]
fn natural_web_jump_keeps_first_registered_opaque_corner_target() {
    matrix(|rules, character, reverse, facing_right| {
        let (mut game, first, second) = scene(
            rules,
            character,
            Vec2::new(if facing_right { 100.0 } else { 99.0 }, 159.0),
            reverse,
            facing_right,
        );
        // Moving the authored Web one pixel left exposes local (1,0), an
        // opaque pixel for either Draw mirror; the first Web is selected.
        assert!(game.world.entity(first).unwrap().mask_contains(
            Vec2::new(if facing_right { 101.0 } else { 100.0 }, 159.0),
            spelunky_core::CollisionPrecision::Precise
        ));
        escape(&mut game);
        assert_eq!(
            life(&game, first),
            11.0,
            "first opaque target lost: {rules:?} {character:?} reverse={reverse}"
        );
        assert_eq!(life(&game, second), 12.0);
    });
}

#[test]
fn natural_web_jump_uses_registration_order_when_both_masks_overlap() {
    matrix(|rules, character, reverse, facing_right| {
        let (mut game, first, second) = scene(
            rules,
            character,
            Vec2::new(88.0, 144.0),
            reverse,
            facing_right,
        );
        escape(&mut game);
        assert_eq!(
            life(&game, first),
            11.0,
            "source first target lost: {rules:?} {character:?} reverse={reverse}"
        );
        assert_eq!(life(&game, second), 12.0);
    });
}

#[test]
fn natural_web_overlap_without_jump_consumes_neither_lifetime() {
    matrix(|rules, character, reverse, facing_right| {
        let (mut game, first, second) = scene(
            rules,
            character,
            Vec2::new(if facing_right { 101.0 } else { 100.0 }, 159.0),
            reverse,
            facing_right,
        );
        let health = game.player.health;
        for _ in 0..3 {
            game.step(&InputFrame::default());
            assert_eq!(game.player.pos, Vec2::new(95.0, 152.0));
            // Source Left release adds a subpixel acceleration and may select Running.
            assert!(game.player.grounded && !game.player.is_stunned());
            assert_eq!(game.player.health, health);
            assert_eq!(life(&game, first), 12.0);
            assert_eq!(life(&game, second), 12.0);
        }
    });
}
