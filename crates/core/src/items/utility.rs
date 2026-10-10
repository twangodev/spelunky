//! Utility items from Classic's scrUseItem and oBasketball/oDice/oLampItem events.
//!
//! Body movement, idol traps, sales and projectile combat belong to their owning
//! systems. Instance positions retain the original GameMaker origins.
use crate::model::*;

/// Defaults needed before the common item-body update.
pub fn init_entity(entity: &mut Entity) {
    if entity.value("utility_initialized") != 0.0 {
        return;
    }
    entity.data.insert("utility_initialized".into(), 1.0);
    match entity.object.as_str() {
        "oBasketball" => {
            entity.data.insert("bounceFactor".into(), 0.7);
            entity.data.insert("frictionFactor".into(), 0.6);
            entity.data.entry("bounces".into()).or_default();
            entity.data.entry("wallBounce".into()).or_default();
            entity.data.entry("goingIn".into()).or_default();
        }
        "oDice" => {
            entity.data.insert("heavy".into(), 1.0);
            entity.data.entry("rolled".into()).or_default();
            entity.data.entry("rolling".into()).or_default();
        }
        "oLampItem" | "oLampRedItem" => {
            entity.data.insert("heavy".into(), 1.0);
            entity.data.entry("value".into()).or_insert(1000.0);
            // oScreen Begin Step draws the white lamp circle at (x,y-4).
            // Red lamps tint nearby player light rather than emitting a circle.
            let radius = if entity.object == "oLampItem" {
                96.0
            } else {
                0.0
            };
            entity.data.insert("light_radius".into(), radius);
            entity.data.insert("light_offset_y".into(), -4.0);
            if entity.object == "oLampRedItem" {
                entity.data.insert("red_light_range".into(), 96.0);
            }
        }
        "oFlareSpark" => {
            entity.vel.y = -0.1;
        }
        _ => {}
    }
}

/// Returns whether the held object consumed this use. Down is handled by the
/// common drop/throw path; a teleporter is never thrown by a normal activation.
pub fn use_utility(game: &mut Game, held_id: u64, input: &InputFrame) -> bool {
    if !game
        .world
        .entity(held_id)
        .is_some_and(|e| e.object == "oTeleporter")
    {
        return false;
    }
    if input.down {
        return false;
    }
    if game.player.state == PlayerState::Ducking
        || game.player.attack_timer > 0
        || (!input.up
            && game
                .player
                .data
                .get("weapon_cooldown")
                .copied()
                .unwrap_or(0.0)
                > 0.0)
    {
        return true;
    }

    let origin = game.player.pos;
    let distance = 16.0 * f64::from(4 + game.rng.range(5));
    let mut target = origin;
    if input.up {
        target.y -= distance;
        while target.y < 16.0 {
            target.y += 16.0;
        }
    } else {
        target.x += f64::from(game.player.facing) * distance;
        target.x = target.x.clamp(8.0, game.world.width as f64 * 16.0 - 8.0);
    }
    // Classic searches only three tiles upward, even if the result stays solid.
    for _ in 0..3 {
        if target.y <= 16.0
            || !solid_rect(game, Rect::new(target.x - 4.0, target.y - 4.0, 8.0, 8.0))
        {
            break;
        }
        target.y -= 16.0;
    }
    for _ in 0..3 {
        let pos = Vec2::new(
            origin.x - 4.0 + f64::from(game.rng.range(9)),
            origin.y - 4.0 + f64::from(game.rng.range(9)),
        );
        let id = game.world.spawn("oFlareSpark", pos);
        init_entity(game.world.entity_mut(id).unwrap());
    }
    game.player.pos = target;
    game.player.previous_pos = target;
    game.player.state = PlayerState::Falling;
    game.player.grounded = false;
    game.update_camera();
    game.previous_camera = game.camera;
    // Teleporting is a discontinuity, including carried items and punishment
    // chains: interpolation must never sweep them through the intervening room.
    for entity in &mut game.world.entities {
        if entity.alive
            && (entity.id == held_id || matches!(entity.object.as_str(), "oBall" | "oChain"))
        {
            entity.pos = target;
            entity.previous_pos = target;
        }
    }
    let player_bounds = game.player.bounds();
    if let Some(enemy) = game.world.entities.iter_mut().find(|e| {
        e.alive && crate::catalog::is_a(&e.object, "oEnemy") && e.bounds().intersects(player_bounds)
    }) {
        // The original directly destroys the first intersecting enemy, bypassing
        // invulnerability, ordinary attack damage and death drops.
        enemy.health -= 99;
        enemy.alive = false;
    }
    game.sound("sndTeleport", target);
    // oPlayer1's crushed check follows item use in the same Step event. The
    // three retries can still leave the player embedded; invulnerability does
    // not protect against that failure, and no additional destination search is
    // permitted. The game controller owns final death/Ankh handling.
    if solid_rect(game, Rect::new(target.x, target.y, 0.001, 0.001)) {
        game.player.health -= 99;
        game.player.vel.y = -3.0;
        game.events.push(GameEvent::Damage { amount: 99 });
    }
    true
}

fn solid_rect(game: &Game, rect: Rect) -> bool {
    game.world.collides(rect)
        || game
            .world
            .entities
            .iter()
            .any(|e| e.alive && crate::catalog::is_solid(&e.object) && rect.intersects(e.bounds()))
}

/// The trap subsystem owns idol pickup triggers and the once-only idol count.
pub fn on_pickup(_game: &mut Game, _id: u64) {}

pub fn tick(game: &mut Game) {
    for index in 0..game.world.entities.len() {
        if !game.world.entities[index].alive {
            continue;
        }
        init_entity(&mut game.world.entities[index]);
        let mut entity = game.world.entities[index].clone();
        match entity.object.as_str() {
            "oDice" => {
                if entity.value("value") == 0.0 {
                    entity
                        .data
                        .insert("value".into(), f64::from(1 + game.rng.range(6)));
                }
                if entity.vel.x.abs() > 2.0 || entity.vel.y.abs() > 2.0 {
                    entity.sprite = "sDiceRoll".into();
                    entity
                        .data
                        .insert("value".into(), f64::from(1 + game.rng.range(6)));
                    if game.player.data.get("bet").copied().unwrap_or(0.0) > 0.0 {
                        entity.data.insert("rolling".into(), 1.0);
                    }
                } else {
                    let bounds = entity.bounds();
                    let below = Rect::new(bounds.x, bounds.y + 1.0, bounds.w, bounds.h);
                    if solid_rect(game, below) {
                        entity.sprite = format!("sDice{}", entity.value("value") as u32);
                        if entity.value("rolling") != 0.0 && entity.vel.y == 0.0 {
                            if entity.value("rolled") != 0.0 {
                                game.flags.insert("shopkeepers_angry".into());
                            }
                            entity.data.insert("rolled".into(), 1.0);
                            entity.data.insert("rolling".into(), 0.0);
                        }
                    }
                }
            }
            "oBasketball" => {
                entity.sprite =
                    if entity.held && game.world.solid_at(entity.pos.x, entity.pos.y + 6.0) {
                        "sDribble"
                    } else {
                        "sBasketball"
                    }
                    .into();
            }
            "oFlareSpark" => {
                entity.pos.y += entity.vel.y;
                entity.animation += 0.4;
                // Five source frames at image_speed .4, destroyed on animation
                // end or collision. No acceleration is applied by Step.
                if entity.animation >= 5.0
                    || solid_rect(game, Rect::new(entity.pos.x, entity.pos.y, 0.001, 0.001))
                {
                    entity.alive = false;
                }
            }
            _ => {}
        }
        game.world.entities[index] = entity;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn held_teleporter(seed: u64) -> (Game, u64) {
        let mut game = Game::shell(seed, RulesPreset::Classic);
        game.player.pos = Vec2::new(160.0, 160.0);
        let id = game.world.spawn("oTeleporter", game.player.pos);
        game.world.entity_mut(id).unwrap().held = true;
        game.player.held = Some(id);
        (game, id)
    }

    #[test]
    fn teleport_has_source_range_and_resets_interpolation_without_changing_velocity() {
        for seed in 0..20 {
            let (mut game, id) = held_teleporter(seed);
            game.player.vel = Vec2::new(2.0, -3.0);
            assert!(use_utility(&mut game, id, &InputFrame::default()));
            let distance = game.player.pos.x - 160.0;
            assert!((64.0..=128.0).contains(&distance));
            assert_eq!(distance % 16.0, 0.0);
            assert_eq!(game.player.pos, game.player.previous_pos);
            assert_eq!(game.camera, game.previous_camera);
            assert_eq!(game.player.vel, Vec2::new(2.0, -3.0));
            let held = game.world.entity(id).unwrap();
            assert_eq!(held.pos, held.previous_pos);
            assert_eq!(game.player.held, Some(id));
        }
    }

    #[test]
    fn upwards_teleport_stays_at_least_one_tile_below_room_top() {
        let (mut game, id) = held_teleporter(7);
        game.player.pos.y = 32.0;
        assert!(use_utility(
            &mut game,
            id,
            &InputFrame {
                up: true,
                ..InputFrame::default()
            }
        ));
        assert_eq!(game.player.pos, Vec2::new(160.0, 16.0));
    }

    #[test]
    fn down_delegates_to_throw_path_without_consuming_randomness() {
        let (mut game, id) = held_teleporter(7);
        let before = game.rng.clone().next_u64();
        assert!(!use_utility(
            &mut game,
            id,
            &InputFrame {
                down: true,
                ..InputFrame::default()
            }
        ));
        assert_eq!(game.rng.next_u64(), before);
        assert_eq!(game.player.pos, Vec2::new(160.0, 160.0));
    }

    #[test]
    fn occupied_destination_searches_no_more_than_three_tiles_up() {
        let (mut game, id) = held_teleporter(11);
        game.player.invincible = 100;
        let distance = 16.0 * f64::from(4 + game.rng.clone().range(5));
        let x = ((160.0 + distance) / 16.0) as usize;
        for y in 0..game.world.height {
            game.world.set_tile(x - 1, y, "oBlock");
            game.world.set_tile(x, y, "oBlock");
        }
        use_utility(&mut game, id, &InputFrame::default());
        assert_eq!(game.player.pos, Vec2::new(160.0 + distance, 112.0));
        assert_eq!(game.player.health, -95);
        assert!(game.events.contains(&GameEvent::Damage { amount: 99 }));
    }

    #[test]
    fn teleport_destroys_only_first_overlapping_enemy_even_if_invincible() {
        let (mut game, id) = held_teleporter(11);
        let distance = 16.0 * f64::from(4 + game.rng.clone().range(5));
        let pos = Vec2::new(160.0 + distance, 160.0);
        let snake = game.world.spawn("oSnake", pos);
        game.world
            .entity_mut(snake)
            .unwrap()
            .data
            .insert("invincible".into(), 1.0);
        let second = game.world.spawn("oSnake", pos);
        use_utility(&mut game, id, &InputFrame::default());
        assert!(game.world.entity(snake).is_none());
        assert!(game.world.entity(second).is_some());
    }

    #[test]
    fn utility_defaults_follow_source_and_dice_faces_are_seeded() {
        let mut game = Game::shell(7, RulesPreset::Classic);
        let ball = game.world.spawn("oBasketball", Vec2::new(80.0, 80.0));
        let die = game.world.spawn("oDice", Vec2::new(80.0, 80.0));
        let lamp = game.world.spawn("oLampItem", Vec2::new(80.0, 80.0));
        tick(&mut game);
        assert_eq!(game.world.entity(ball).unwrap().value("bounceFactor"), 0.7);
        assert_eq!(
            game.world.entity(ball).unwrap().value("frictionFactor"),
            0.6
        );
        assert_eq!(game.world.entity(die).unwrap().value("heavy"), 1.0);
        assert!((1.0..=6.0).contains(&game.world.entity(die).unwrap().value("value")));
        assert_eq!(game.world.entity(lamp).unwrap().value("light_radius"), 96.0);
        assert_eq!(
            game.world.entity(lamp).unwrap().value("light_offset_y"),
            -4.0
        );
        let face = game.world.entity(die).unwrap().value("value");
        tick(&mut game);
        assert_eq!(game.world.entity(die).unwrap().value("value"), face);
    }

    #[test]
    fn teleport_sparks_move_and_expire_on_original_animation_end() {
        let (mut game, id) = held_teleporter(7);
        use_utility(&mut game, id, &InputFrame::default());
        let sparks: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|e| e.object == "oFlareSpark")
            .map(|e| (e.id, e.pos))
            .collect();
        assert_eq!(sparks.len(), 3);
        tick(&mut game);
        for (id, origin) in &sparks {
            assert!((game.world.entity(*id).unwrap().pos.y - (origin.y - 0.1)).abs() < 1e-8);
        }
        for _ in 1..12 {
            tick(&mut game);
        }
        assert!(
            sparks
                .iter()
                .all(|(id, _)| game.world.entity(*id).is_some())
        );
        tick(&mut game);
        assert!(
            sparks
                .iter()
                .all(|(id, _)| game.world.entity(*id).is_none())
        );
    }
}
