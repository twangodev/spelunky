use spelunky_core::{Entity, Game, GameEvent, InputFrame, RulesPreset, Vec2, level::Level};

fn room(rules: RulesPreset, lower_right: bool) -> Game {
    let mut level = Level::default();
    level.set_tile(5, 6, '@').unwrap();
    level.set_tile(34, 6, 'X').unwrap();
    for x in 0..spelunky_core::level::LEVEL_WIDTH {
        level
            .set_tile(x, if lower_right && x >= 6 { 8 } else { 7 }, '1')
            .unwrap();
    }
    let mut game = Game::shell(1, rules);
    game.load_custom(&level, "bullet-blood-canvas", "");
    assert_eq!(game.player.pos, Vec2::new(104.0, 120.0));
    assert_eq!(game.player.health, 4.0);
    game
}

fn neutral(game: &mut Game, ticks: usize) {
    for _ in 0..ticks {
        game.step(&InputFrame::default());
        assert_eq!(game.player.health, 4.0);
    }
}

fn pickup(game: &mut Game, id: u64) {
    assert!(game.player.grounded);
    game.step(&InputFrame {
        down: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert_eq!(
        game.player.held,
        Some(id),
        "ordinary Down+Attack must acquire intended item"
    );
    neutral(game, 1);
}

fn first_pistol_blood(game: &mut Game, target_id: u64) -> (Entity, Entity, usize) {
    assert_eq!(
        game.world.entity(game.player.held.unwrap()).unwrap().object,
        "oPistol"
    );
    assert!(!game.world.entities.iter().any(|e| e.object == "oBlood"));
    for tick in 0..20 {
        game.step(&InputFrame {
            attack: tick == 0,
            attack_pressed: tick == 0,
            ..Default::default()
        });
        assert_eq!(game.player.health, 4.0);
        if tick == 0 {
            assert!(
                game.events
                    .iter()
                    .any(|e| matches!(e, GameEvent::Sound { name, .. } if name == "sndShotgun")),
                "ordinary held Pistol must fire"
            );
        }
        let blood: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|e| e.alive && e.object == "oBlood")
            .collect();
        if !blood.is_empty() {
            assert_eq!(
                blood.len(),
                1,
                "observe collision blood before later corpse/bounce blood"
            );
            assert_eq!(
                blood[0].age, 0,
                "Collision birth has not run a later Blood Step"
            );
            assert!(
                !game
                    .world
                    .entities
                    .iter()
                    .any(|e| e.alive && e.object == "oBullet")
            );
            return (
                game.world
                    .entity(target_id)
                    .expect("receiver survives its collision frame")
                    .clone(),
                blood[0].clone(),
                tick + 1,
            );
        }
    }
    panic!("ordinary pistol shot did not reach intended receiver in 20 ticks");
}

#[test]
fn pistol_giant_spider_blood_uses_32_pixel_canvas_on_collision_birth() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = room(rules, true);
        let pistol = game.spawn("oPistol", Vec2::new(104.0, 120.0));
        let giant = game.spawn("oGiantSpider", Vec2::new(116.0, 112.0));
        let initial = game.world.entity(giant).unwrap();
        assert_eq!(initial.health, 1);
        assert_eq!(initial.sprite, "sGiantSpiderFlip");
        assert_eq!(
            (initial.sprite_bounds().w, initial.sprite_bounds().h),
            (30.0, 15.0)
        );
        assert_eq!(
            spelunky_core::catalog::sprite_dimensions(&initial.sprite),
            Some((32, 32))
        );
        neutral(&mut game, 2);
        pickup(&mut game, pistol);
        assert_eq!(
            game.world.entity(giant).unwrap().pos,
            Vec2::new(116.0, 112.0)
        );
        let (target, blood, flight_ticks) = first_pistol_blood(&mut game, giant);
        assert_eq!(
            flight_ticks, 1,
            "Player319 births Bullet669 before Giant749 and earliest Alarm0 at tick6"
        );
        assert_eq!(target.sprite, "sGiantSpiderFlip");
        assert_eq!(target.health, -3);
        assert_eq!(target.value("blood_left"), 3.0);
        assert_eq!(target.sprite_bounds().w, 30.0);
        assert_eq!(
            blood.pos,
            Vec2::new(target.pos.x + 16.0, target.pos.y + 16.0),
            "source full canvas center, independent of changing Flip mask"
        );
    }
}

#[test]
fn pistol_yeti_16_pixel_full_mask_control_keeps_canvas_center() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = room(rules, false);
        let pistol = game.spawn("oPistol", Vec2::new(104.0, 120.0));
        let yeti = game.spawn("oYeti", Vec2::new(128.0, 112.0));
        assert_eq!(game.world.entity(yeti).unwrap().health, 5);
        neutral(&mut game, 2);
        pickup(&mut game, pistol);
        let (target, blood, flight_ticks) = first_pistol_blood(&mut game, yeti);
        assert!(
            flight_ticks <= 4,
            "close ordinary target is hit before non-full walking frame"
        );
        assert_eq!(target.health, 1);
        assert_eq!(target.state, 98);
        assert_eq!(target.value("blood_left"), 4.0);
        assert_eq!(
            spelunky_core::catalog::sprite_dimensions(&target.sprite),
            Some((16, 16))
        );
        assert_eq!(
            (target.sprite_bounds().w, target.sprite_bounds().h),
            (16.0, 16.0)
        );
        assert_eq!(blood.pos, Vec2::new(target.pos.x + 8.0, target.pos.y + 8.0));
    }
}

#[test]
fn pistol_damsel_blood_uses_negative_width_after_ordinary_carry_and_throw() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = room(rules, false);
        let damsel = game.spawn("oDamsel", Vec2::new(104.0, 120.0));
        let pistol = game.spawn("oPistol", Vec2::new(96.0, 120.0));
        neutral(&mut game, 2);
        pickup(&mut game, damsel);
        assert_eq!(game.player.facing, 1);
        assert_eq!(game.world.entity(damsel).unwrap().facing, 1);
        assert_eq!(
            game.world.entity(damsel).unwrap().value("image_xscale"),
            -1.0,
            "ordinary held-facing then Draw produces signed scale"
        );
        game.step(&InputFrame {
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert_eq!(game.player.held, None);
        assert_eq!(game.world.entity(damsel).unwrap().state, 2);
        neutral(&mut game, 8);
        pickup(&mut game, pistol);
        let receiver = game.world.entity(damsel).unwrap();
        assert_eq!(receiver.health, 4);
        assert_eq!(receiver.facing, 1);
        assert_eq!(receiver.value("image_xscale"), -1.0);
        let (target, blood, _) = first_pistol_blood(&mut game, damsel);
        assert_eq!(target.health, 0);
        assert_eq!(
            target.value("blood_left"),
            4.0,
            "Damsel blood check precedes four-HP subtraction"
        );
        assert_eq!(target.value("image_xscale"), -1.0);
        assert_eq!(
            spelunky_core::catalog::sprite_dimensions(&target.sprite),
            Some((16, 16))
        );
        assert_eq!(
            blood.pos,
            Vec2::new(target.pos.x - 8.0, target.pos.y + 8.0),
            "literal signed sprite_width/2, including negative width"
        );
    }
}
