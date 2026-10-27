//! Regression for the one-pixel spider contact from the natural Level 2 route.
//!
//! Source `oEnemy`'s `Collision with oCharacter` handler takes the non-stomp,
//! non-invincible branch at XML lines 29-72, applying one damage and `sndHurt`.
//! `sRunLeft.xml` and `sSpiderFlip.xml` have separate rectangular auto masks.
//! In `dispatcher.rs`, AutomaticCollision (line 2121) precedes Animation (line
//! 2173), so this pair uses the player's pre-advance frame and spider's
//! post-Step pose, before either frame advances.

use spelunky_core::{
    Area, CollisionPrecision, Game, GameEvent, InputFrame, PlayerState, RulesPreset, RunState,
    Vec2, World,
};

#[test]
fn spider_contact_includes_shared_mask_pixel_at_y_336() {
    let mut game = Game::shell(0, RulesPreset::Classic);
    game.world = World::new(40, 40, Area::Mines, 2);
    game.world.name = "rLevel".into();

    // Hold the recorded Player pose while keeping it as a live collision
    // instance; the grounded state prevents the inherited contact from taking
    // the stomp branch. Only the floor is solid, so this is not crush damage.
    for x in 29..=32 {
        game.world.set_tile(x, 22, "oHardBlock");
    }
    game.player.pos = Vec2::new(491.0, 344.0);
    game.player.previous_pos = game.player.pos;
    game.player.vel = Vec2::new(3.038, 0.0);
    game.player.state = PlayerState::Running;
    game.player.grounded = true;
    game.player.health = 1.0;
    game.player.invincible = 0;
    game.player.sprite = "sRunLeft".into();
    game.player.animation = 1.8114;
    game.player.data.insert("image_xscale".into(), -1.0);
    game.player.data.insert("fall_timer".into(), 0.0);
    game.player.data.insert("fall_ticks".into(), 0.0);
    game.player
        .data
        .insert("source_platform_grounded".into(), 1.0);
    game.player.data.insert("active".into(), 0.0);
    // Keep the actors inside the same active camera region as in the route.
    game.camera = Vec2::new(331.0, 224.0);
    game.previous_camera = game.camera;
    let player_collision_frame = game.player.clone();

    let spider_id = game.spawn("oSpider", Vec2::new(480.0, 325.0));
    let spider = game.world.entity_mut(spider_id).unwrap();
    spider.previous_pos = spider.pos;
    spider.vel = Vec2::new(0.0, 1.6);
    spider.state = 1;
    spider.sprite = "sSpiderFlip".into();
    spider.animation = 3.6;
    spider.data.insert("animation_speed".into(), 0.4);

    // On AutomaticCollision, Spider Step has moved it to y=327 while both
    // instances still use their pre-animation frames. Their sole shared row
    // is y=336; x=486..495 is occupied by both source masks.
    let mut spider_collision_frame = game.world.entity(spider_id).unwrap().clone();
    spider_collision_frame.pos = Vec2::new(480.0, 327.0);
    assert_eq!(
        player_collision_frame.sprite_bounds(),
        spelunky_core::Rect::new(486.0, 336.0, 13.0, 16.0)
    );
    spider_collision_frame.animation = 3.6;
    assert_eq!(
        spider_collision_frame.sprite_bounds(),
        spelunky_core::Rect::new(480.0, 328.0, 16.0, 9.0)
    );
    for x in 486..=495 {
        let shared_pixel = Vec2::new(f64::from(x), 336.0);
        assert!(player_collision_frame.mask_contains(shared_pixel, CollisionPrecision::Precise));
        assert!(spider_collision_frame.mask_contains(shared_pixel, CollisionPrecision::Precise));
    }

    game.step(&InputFrame::default());

    assert_eq!(game.state, RunState::Playing);
    assert_eq!(game.player.pos, Vec2::new(491.0, 344.0));
    assert_eq!(game.player.health, 0.0);
    assert_eq!(game.player.invincible, 30);
    assert_eq!(game.player.vel.x, 6.0);
    let damage_count = game
        .events
        .iter()
        .filter(|event| **event == GameEvent::Damage { amount: 1 })
        .count();
    let hurt_sound_count = game
        .events
        .iter()
        .filter(|event| matches!(event, GameEvent::Sound { name, .. } if name == "sndHurt"))
        .count();
    let hit_sound_count = game
        .events
        .iter()
        .filter(|event| matches!(event, GameEvent::Sound { name, .. } if name == "sndHit"))
        .count();
    assert_eq!(damage_count, 1, "one inherited enemy contact");
    assert_eq!(
        hurt_sound_count, 1,
        "the non-stomp source branch plays sndHurt"
    );
    assert_eq!(hit_sound_count, 0, "this is not a stomp");

    let spider = game.world.entity(spider_id).unwrap();
    assert_eq!(spider.pos, Vec2::new(480.0, 327.0));
    assert_eq!(spider.previous_pos, Vec2::new(480.0, 325.0));
}
