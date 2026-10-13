//! Authored initial rooms and default Create actors; every advancing operation
//! is ordinary Game::step input. No post-start pose, frame, health or fuse writes.
use spelunky_core::{
    Area, CollisionPrecision, Entity, Game, GameEvent, InputFrame, PlayerState, Rect, RulesPreset,
    RunState, Vec2, World, catalog,
};

fn room(rules: RulesPreset, x: f64, wall: bool) -> Game {
    let mut game = Game::shell(7, rules);
    game.world = World::new(40, 30, Area::Mines, 1);
    game.world.name = "rLevel".into();
    for col in 0..40 {
        game.world.set_tile(col, 7, "oHardBlock");
    }
    if wall {
        for row in 0..7 {
            game.world.set_tile(10, row, "oHardBlock");
        }
    }
    game.player.pos = Vec2::new(x, 104.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.player.facing = 1;
    game.player.health = 30; // Initial survivor fixture exposes recoil/stun too.
    game
}
fn mask(e: &Entity) -> catalog::CollisionMask {
    assert_eq!(e.sprite, "sExplosion");
    assert_eq!(e.value("image_angle"), 0.0);
    assert_eq!(e.data.get("image_xscale").copied().unwrap_or(1.0), 1.0);
    assert_eq!(e.data.get("image_yscale").copied().unwrap_or(1.0), 1.0);
    catalog::instance_mask(
        &e.object,
        &e.sprite,
        e.pos,
        e.animation,
        Vec2::new(1.0, 1.0),
        0.0,
    )
}
// Independent literal aggregate [0,2,47,45], origin [24,24]. All fixtures
// use integer Explosion poses, avoiding any transform-rounding oracle reuse.
fn source_overlap(e: &Entity, p: Vec2) -> bool {
    assert_eq!(e.pos.x.fract(), 0.0);
    assert_eq!(e.pos.y.fract(), 0.0);
    let expected = e.pos.x - 24.0 <= p.x + 8.0
        && e.pos.x + 23.0 >= p.x - 8.0
        && e.pos.y - 22.0 <= p.y + 8.0
        && e.pos.y + 21.0 >= p.y - 8.0;
    // Public precision-zero point queries independently pin the aggregate
    // corners and immediate exterior; point_bounds is a crate-private API.
    for (x, y) in [
        (e.pos.x - 24.0, e.pos.y - 22.0),
        (e.pos.x + 23.0, e.pos.y + 21.0),
    ] {
        assert!(mask(e).contains(Vec2::new(x, y), CollisionPrecision::BoundingBox));
    }
    for (x, y) in [
        (e.pos.x - 25.0, e.pos.y),
        (e.pos.x + 24.0, e.pos.y),
        (e.pos.x, e.pos.y - 23.0),
        (e.pos.x, e.pos.y + 22.0),
    ] {
        assert!(!mask(e).contains(Vec2::new(x, y), CollisionPrecision::BoundingBox));
    }
    assert_eq!(
        mask(e).intersects_rectangle_bbox(
            Vec2::new(p.x - 8.0, p.y - 8.0),
            Vec2::new(p.x + 8.0, p.y + 8.0)
        ),
        expected
    );
    expected
}
fn assert_hurt(game: &Game, marker: &str) {
    assert_eq!(
        game.player.health, 20,
        "{marker}: pos={:?} vel={:?}",
        game.player.pos, game.player.vel
    );
    assert!(
        game.events
            .iter()
            .any(|e| matches!(e, GameEvent::Damage { amount: 10 })),
        "{marker}: damage"
    );
    assert!(
        (4.0..=6.0).contains(&game.player.vel.x.abs()),
        "{marker}: recoil"
    );
    assert_eq!(game.player.vel.y, -6.0);
    assert_eq!(game.player.stun, 100);
    assert_eq!(game.player.data.get("burning").copied(), Some(50.0));
    assert_eq!(game.state, RunState::Playing);
}
#[test]
fn default_explosion_first_frame_uses_raw_inclusive_margin() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for offset in [-31.0, -30.0, 31.0, 32.0] {
            let mut game = room(rules, 160.0, false);
            let id = game.spawn("oExplosion", Vec2::new(160.0 + offset, 104.0));
            let e = game.world.entity(id).unwrap();
            assert_eq!(e.animation, 0.0);
            assert_eq!(e.image_speed(), 0.8);
            assert!(e.alive && e.engine_active);
            assert!(source_overlap(e, game.player.pos));
            assert!(!e.mask_intersects(
                Rect::new(152.0, 96.0, 17.0, 17.0),
                CollisionPrecision::BoundingBox
            ));
            game.step(&InputFrame::default());
            assert_hurt(&game, "SOURCE_EXPLOSION_FIRST_FRAME_RAW_BBOX");
            assert_eq!(game.player.vel.x.is_sign_positive(), offset < 0.0);
        }
    }
}
#[test]
fn outside_raw_envelope_stays_safe_through_natural_last_frame() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for offset in [-32.0, 33.0] {
            let mut game = room(rules, 160.0, false);
            let id = game.spawn("oExplosion", Vec2::new(160.0 + offset, 104.0));
            let mut saw_last = false;
            for _ in 0..13 {
                if let Some(e) = game.world.entity(id).filter(|e| e.alive) {
                    assert!(!source_overlap(e, game.player.pos));
                    saw_last |= e.animation.floor() == 9.0;
                }
                game.step(&InputFrame::default());
                assert_eq!(game.player.health, 30);
                assert_eq!(game.player.pos, Vec2::new(160.0, 104.0));
                assert!(
                    !game
                        .events
                        .iter()
                        .any(|e| matches!(e, GameEvent::Damage { .. }))
                );
            }
            assert!(saw_last, "natural animation must reach frame nine");
        }
    }
}
#[test]
fn ordinary_walk_into_late_explosion_uses_same_aggregate_bbox() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = room(rules, 164.0, false);
        let id = game.spawn("oExplosion", Vec2::new(130.0, 104.0));
        for _ in 0..6 {
            game.step(&InputFrame::default());
            assert_eq!(game.player.health, 30);
        }
        let mut hit = false;
        for _ in 0..6 {
            let frame = game.world.entity(id).unwrap().animation;
            game.step(&InputFrame {
                left: true,
                ..Default::default()
            });
            if let Some(e) = game.world.entity(id).filter(|e| e.alive) {
                if source_overlap(e, game.player.pos) {
                    assert!(frame >= 4.0, "late-frame premise");
                    assert!(!e.mask_intersects(
                        Rect::new(game.player.pos.x - 8.0, game.player.pos.y - 8.0, 17.0, 17.0),
                        CollisionPrecision::BoundingBox
                    ));
                    assert_hurt(&game, "SOURCE_EXPLOSION_LATE_FRAME_RAW_BBOX");
                    hit = true;
                    break;
                }
            }
            assert_eq!(game.player.health, 30);
        }
        assert!(hit, "ordinary walking must reach the live aggregate margin");
    }
}
#[test]
fn inventory_bomb_natural_fuse_hits_player_at_first_blast_margin() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = room(rules, 124.0, true);
        game.step(&InputFrame {
            down: true,
            bomb_pressed: true,
            ..Default::default()
        });
        assert_eq!(game.player.bombs, 3);
        let id = game
            .world
            .entities
            .iter()
            .find(|e| e.alive && e.object == "oBomb")
            .unwrap()
            .id;
        assert_eq!(game.world.entity(id).unwrap().alarms[0], 80);
        let mut saw_second_alarm = false;
        let mut burst = false;
        for age in 0..145 {
            let bomb = game.world.entity(id).filter(|e| e.alive).cloned();
            if let Some(b) = &bomb {
                saw_second_alarm |= b.alarms[1] > 0;
            }
            game.step(&InputFrame {
                right: age < 35,
                ..Default::default()
            });
            if let Some(e) = game
                .world
                .entities
                .iter()
                .find(|e| e.alive && e.object == "oExplosion")
            {
                assert!(saw_second_alarm, "ordinary 80+40 fuse must run both alarms");
                assert!(
                    bomb.as_ref().is_some_and(|b| b.alarms[1] == 1),
                    "birth comes from final Alarm1"
                );
                assert_eq!(e.animation, 0.8, "first naturally born blast frame");
                assert!(
                    source_overlap(e, game.player.pos),
                    "bomb={:?} player={:?}",
                    e.pos,
                    game.player.pos
                );
                assert!(!e.mask_intersects(
                    Rect::new(game.player.pos.x - 8.0, game.player.pos.y - 8.0, 17.0, 17.0),
                    CollisionPrecision::BoundingBox
                ));
                assert_hurt(&game, "SOURCE_INVENTORY_BOMB_FIRST_BLAST_RAW_BBOX");
                assert!(game.player.vel.x > 0.0);
                assert!(!game.world.entities.iter().any(|e| e.id == id && e.alive));
                burst = true;
                break;
            }
            assert_eq!(game.player.health, 30, "pre-burst health age={age}");
            assert_eq!(game.state, RunState::Playing);
        }
        assert!(burst, "natural inventory bomb must reach Explosion Create");
    }
}
