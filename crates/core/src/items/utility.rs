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
    let image_speed = match entity.object.as_str() {
        "oFlareCrate" => Some(0.2),
        "oFlareSpark" => Some(0.4),
        // None of these Create events overrides GameMaker's default speed.
        "oTeleporter" | "oLampItem" | "oLampRedItem" | "oBasketball" | "oDice" => Some(1.0),
        _ => None,
    };
    if let Some(speed) = image_speed {
        entity.set_image_speed(speed);
    }
    let movement_bounds = match entity.object.as_str() {
        "oDice" => Some((-6.0, 0.0, 6.0, 8.0)),
        "oBasketball" | "oLampItem" | "oLampRedItem" | "oTeleporter" => {
            Some((-4.0, -4.0, 4.0, 4.0))
        }
        _ => None,
    };
    if let Some((left, top, right, bottom)) = movement_bounds {
        for (key, value) in [
            ("bboxLeft", left),
            ("bboxTop", top),
            ("bboxRight", right),
            ("bboxBottom", bottom),
        ] {
            entity.data.entry(key.into()).or_insert(value);
        }
    }
    match entity.object.as_str() {
        "oTeleporter" => {
            entity.data.entry("cost".into()).or_insert(10000.0);
        }
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

/// Complete oDice Create while the caller still owns the creation RNG order.
/// In particular room generation must draw before stamping its next character,
/// even when this die will remain deactivated outside the initial view.
pub(crate) fn initialize_dice(rng: &mut Rng, entity: &mut Entity) {
    if entity.object != "oDice" || entity.value("dice_create_initialized") != 0.0 {
        return;
    }
    init_entity(entity);
    entity
        .data
        .insert("value".into(), f64::from(1 + rng.range(6)));
    entity.data.insert("dice_create_initialized".into(), 1.0);
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
            || !solid_rect(game, Rect::new(target.x - 4.0, target.y - 4.0, 9.0, 9.0))
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
        let id = game.spawn("oFlareSpark", pos);
        init_entity(game.world.entity_mut(id).unwrap());
    }
    game.player.pos = target;
    game.player.previous_pos = target;
    game.player.state = PlayerState::Falling;
    game.player.grounded = false;
    game.update_camera();
    game.previous_camera = game.camera;
    // The common held-item attachment and punishment-chain updates run later
    // in this tick. Present their final destination, including carry offsets,
    // rather than interpolating from the source room or the temporary anchor.
    game.flags.insert("render_discontinuity".into());
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
    // instance_place uses sprite masks, independent of setCollisionBounds used
    // by the platform engine. In particular the standing sprite is 12px wide
    // while the normal movement rectangle is 10px wide.
    let player_bounds = game.player.sprite_bounds();
    if let Some(enemy) = game.world.entities.iter_mut().find(|e| {
        e.alive
            && crate::catalog::is_a(&e.object, "oEnemy")
            && e.sprite_bounds().intersects(player_bounds)
    }) {
        // The original directly destroys the first intersecting enemy, bypassing
        // invulnerability, ordinary attack damage and death drops.
        enemy.health -= 99;
        enemy.alive = false;
    }
    game.sound("sndTeleport", target);
    true
}

fn solid_rect(game: &Game, rect: Rect) -> bool {
    // GameMaker collision queries do not create solid walls at room edges.
    // Tile storage must therefore ignore missing cells instead of using the
    // generic World::collides adapter's implicit outside-room collider.
    for y in (rect.y / TILE_SIZE).floor() as i32
        ..=((rect.y + rect.h - 0.00001) / TILE_SIZE).floor() as i32
    {
        for x in (rect.x / TILE_SIZE).floor() as i32
            ..=((rect.x + rect.w - 0.00001) / TILE_SIZE).floor() as i32
        {
            if game.world.tile(x, y).is_some_and(Tile::is_solid) {
                return true;
            }
        }
    }
    game.world.entities.iter().any(|e| {
        e.alive && crate::catalog::is_solid(&e.object) && rect.intersects(e.sprite_bounds())
    })
}

/// The trap subsystem owns the source idol pickup and Step producers.
pub fn on_pickup(_game: &mut Game, _id: u64) {}

/// Utility object event families handled by this module. Generic oItem body
/// movement is dispatched by the parent before these child Step handlers.
pub(crate) fn handles(object: &str) -> bool {
    matches!(
        object,
        "oFlareCrate"
            | "oDice"
            | "oBasketball"
            | "oFlareSpark"
            | "oTeleporter"
            | "oLampItem"
            | "oLampRedItem"
    )
}

pub(crate) fn alarm(game: &mut Game, id: u64, alarm: u8) {
    if alarm != 0 {
        return;
    }
    let Some(origin) = game
        .world
        .entity(id)
        .filter(|e| e.object == "oFlareCrate")
        .map(|e| e.pos)
    else {
        return;
    };
    let pos = Vec2::new(
        origin.x + f64::from(game.rng.range(4)) - f64::from(game.rng.range(4)),
        origin.y - 4.0 + f64::from(game.rng.range(4)) - f64::from(game.rng.range(4)),
    );
    let spark = game.spawn("oFlareSpark", pos);
    init_entity(game.world.entity_mut(spark).unwrap());
    if let Some(entity) = game.world.entity_mut(id) {
        entity.alarms[0] = 2;
    }
}

pub(crate) fn animation_end(game: &mut Game, id: u64) {
    if let Some(entity) = game.world.entity_mut(id)
        && entity.object == "oFlareSpark"
    {
        entity.alive = false;
    }
}

pub(crate) fn builtin_motion(_game: &mut Game, _id: u64) {}

pub(crate) fn collision(_game: &mut Game, _id: u64, _other: crate::dispatcher::Instance) {}

// Retained for callers migrating to the source dispatcher; alarm countdowns
// belong exclusively to its outer alarm-number pass.
pub fn pre_tick(_game: &mut Game) {}

/// oDice replaces oItem Step: its camera/shop checks precede held attachment
/// and movement, while the final rolling/face block runs outside this gate.
pub(crate) fn dice_begin_step(game: &mut Game, entity: &mut Entity) -> bool {
    if !super::item_body_active(game, entity) {
        return false;
    }
    if entity.value("cost") > 0.0
        && !game
            .world
            .entities
            .iter()
            .any(|keeper| keeper.alive && crate::catalog::is_a(&keeper.object, "oShopkeeper"))
    {
        entity.data.insert("cost".into(), 0.0);
    }
    if game.is_level() && !crate::economy::in_shop(game, entity.pos) {
        crate::economy::anger(game, entity.pos, "COME BACK HERE, THIEF!");
    }
    true
}

pub(crate) fn step_instance(game: &mut Game, id: u64) {
    let Some(index) = game
        .world
        .entities
        .iter()
        .position(|e| e.id == id && e.alive)
    else {
        return;
    };
    if !handles(&game.world.entities[index].object) {
        return;
    }
    init_entity(&mut game.world.entities[index]);
    let mut entity = game.world.entities[index].clone();
    match entity.object.as_str() {
        "oFlareCrate" => {
            let water = game
                .world
                .tile(
                    (entity.pos.x / TILE_SIZE).floor() as i32,
                    (entity.pos.y / TILE_SIZE).floor() as i32,
                )
                .is_some_and(|tile| crate::catalog::is_a(&tile.object, "oWater"))
                || game.world.entities.iter().any(|water| {
                    water.alive
                        && crate::catalog::is_a(&water.object, "oWater")
                        && water.sprite_bounds().contains(entity.pos)
                });
            if water {
                game.spawn("oSplash", entity.pos);
                game.spawn("oPoof", entity.pos);
                game.sound("sndSplash", entity.pos);
                entity.alive = false;
                entity.held = false;
                if game.player.held == Some(entity.id) {
                    game.player.held = None;
                }
            }
        }
        "oDice" => {
            if entity.value("value") == 0.0 {
                // Compatibility for directly authored/test instances;
                // generated dice have already run Create synchronously.
                initialize_dice(&mut game.rng, &mut entity);
            }
            if entity.vel.x.abs() > 2.0 || entity.vel.y.abs() > 2.0 {
                crate::animation::assign(&mut entity.sprite, &mut entity.animation, "sDiceRoll");
                entity
                    .data
                    .insert("value".into(), f64::from(1 + game.rng.range(6)));
                if game.player.data.get("bet").copied().unwrap_or(0.0) > 0.0 {
                    entity.data.insert("rolling".into(), 1.0);
                }
            } else {
                // isCollisionBottom(1) is a single line at bb, not a
                // translated body that can also hit an adjacent wall.
                let below = Rect::new(
                    (entity.pos.x + entity.value("bboxLeft")).round_ties_even(),
                    (entity.pos.y + entity.value("bboxBottom")).round_ties_even(),
                    entity.value("bboxRight") - entity.value("bboxLeft"),
                    1.0,
                );
                if super::solid_edge(&game.world, below, entity.id) {
                    let sprite = format!("sDice{}", entity.value("value") as u32);
                    crate::animation::assign(&mut entity.sprite, &mut entity.animation, &sprite);
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
            let sprite = if entity.held && game.world.solid_at(entity.pos.x, entity.pos.y + 6.0) {
                "sDribble"
            } else {
                "sBasketball"
            };
            crate::animation::assign(&mut entity.sprite, &mut entity.animation, sprite);
        }
        "oFlareSpark" => {
            entity.pos.y += entity.vel.y;
            // Animation End is dispatched by the shared endframe pass.
            if solid_rect(game, Rect::new(entity.pos.x, entity.pos.y, 0.001, 0.001)) {
                entity.alive = false;
            }
        }
        _ => {}
    }
    game.world.entities[index] = entity;
}

/// Compatibility batch wrapper; production dispatches one source instance.
pub fn tick(game: &mut Game) {
    let ids: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && handles(&e.object))
        .map(|e| e.id)
        .collect();
    for id in ids {
        step_instance(game, id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dice_shop() -> (Game, u64, u64) {
        let mut game = Game::shell(18, RulesPreset::Classic);
        for x in 0..game.world.width {
            game.world.set_tile(x, 10, "oBlock");
        }
        game.player.pos = Vec2::new(160.0, 152.0);
        game.player.previous_pos = game.player.pos;
        game.player.grounded = true;
        let keeper = game.spawn("oShopkeeper", Vec2::new(96.0, 144.0));
        game.world
            .entity_mut(keeper)
            .unwrap()
            .text
            .insert("style".into(), "Craps".into());
        let die = game.spawn("oDice", game.player.pos);
        game.step(&InputFrame {
            down: true,
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert_eq!(game.player.held, Some(die));
        (game, keeper, die)
    }

    #[test]
    fn actual_pickup_carry_and_throw_die_out_of_shop_reports_theft() {
        let (mut game, keeper, die) = dice_shop();
        for frame in 0..12 {
            game.step(&InputFrame {
                right: true,
                ..Default::default()
            });
            if frame == 6 {
                // The source checks the old position before this attachment
                // crosses the room boundary; anger begins on the next Step.
                assert_eq!(game.world.entity(die).unwrap().pos.x, 178.0);
                assert_eq!(game.data.get("thiefLevel").copied().unwrap_or(0.0), 0.0);
            }
            if frame == 7 {
                assert_eq!(game.data["thiefLevel"], 2.0);
                assert_eq!(game.world.entity(keeper).unwrap().state, 2);
                assert!(game.events.iter().any(|event| matches!(event,
                    GameEvent::Dialogue(message) if message.first == "COME BACK HERE, THIEF!"
                )));
            }
        }
        game.step(&InputFrame {
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert_eq!(game.player.held, None);
        assert!(!game.world.entity(die).unwrap().held);
        for _ in 0..9 {
            game.step(&InputFrame::default());
        }
        assert_eq!(game.data["thiefLevel"], 2.0);
        assert_eq!(game.world.entity(keeper).unwrap().value("angered"), 1.0);
    }

    #[test]
    fn actual_held_die_stays_in_shop_without_reporting_theft() {
        let (mut game, keeper, die) = dice_shop();
        for _ in 0..22 {
            game.step(&InputFrame::default());
        }
        assert_eq!(game.world.entity(die).unwrap().pos, Vec2::new(164.0, 150.0));
        assert_eq!(game.player.held, Some(die));
        assert_eq!(game.data.get("thiefLevel").copied().unwrap_or(0.0), 0.0);
        assert_eq!(game.world.entity(keeper).unwrap().state, 0);
        assert_eq!(game.world.entity(keeper).unwrap().value("angered"), 0.0);
    }

    #[test]
    fn actual_offcamera_die_keeps_position_but_draws_rolling_face() {
        let mut game = Game::shell(18, RulesPreset::Classic);
        game.player.pos = Vec2::new(160.0, 152.0);
        game.player.previous_pos = game.player.pos;
        let die = game.spawn("oDice", Vec2::new(336.0, 152.0));
        game.world.entity_mut(die).unwrap().vel.x = 4.0;
        let mut expected_rng = game.rng.clone();
        let face = 1 + expected_rng.range(6);
        game.step(&InputFrame::default());
        let entity = game.world.entity(die).unwrap();
        assert!(entity.engine_active);
        assert_eq!(entity.pos, Vec2::new(336.0, 152.0));
        assert_eq!(entity.vel, Vec2::new(4.0, 0.0));
        assert_eq!(entity.sprite, "sDiceRoll");
        assert_eq!(entity.value("value"), f64::from(face));
        assert_eq!(game.rng.next_u64(), expected_rng.next_u64());
    }

    fn held_teleporter(seed: u64) -> (Game, u64) {
        let mut game = Game::shell(seed, RulesPreset::Classic);
        game.player.pos = Vec2::new(160.0, 160.0);
        let id = game.spawn("oTeleporter", game.player.pos);
        game.world.entity_mut(id).unwrap().held = true;
        game.player.held = Some(id);
        (game, id)
    }

    #[test]
    fn dice_create_draws_once_and_first_step_retains_that_face() {
        let mut game = Game::shell(123, RulesPreset::Classic);
        let id = game.world.spawn("oDice", Vec2::new(96.0, 88.0));
        game.world.set_tile(6, 6, "oBlock");
        let mut expected_rng = game.rng.clone();
        let face = 1 + expected_rng.range(6);
        initialize_dice(&mut game.rng, game.world.entity_mut(id).unwrap());
        initialize_dice(&mut game.rng, game.world.entity_mut(id).unwrap());
        let entity = game.world.entity(id).unwrap();
        assert_eq!(entity.value("value"), f64::from(face));
        assert_eq!(entity.value("heavy"), 1.0);
        assert_eq!(entity.value("rolled"), 0.0);
        assert_eq!(entity.value("rolling"), 0.0);
        assert_eq!(entity.value("bboxLeft"), -6.0);
        assert_eq!(entity.value("bboxBottom"), 8.0);
        game.step(&InputFrame::default());
        assert_eq!(
            game.world.entity(id).unwrap().sprite,
            format!("sDice{face}")
        );
        assert_eq!(game.rng.next_u64(), expected_rng.next_u64());
    }

    #[test]
    fn utility_animation_advances_at_create_speed_once_per_game_step() {
        for (object, speed, frames) in [
            ("oFlareCrate", 0.2, 3.0),
            ("oTeleporter", 1.0, 16.0),
            ("oLampItem", 1.0, 3.0),
            ("oLampRedItem", 1.0, 3.0),
        ] {
            let mut game = Game::shell(7, RulesPreset::Classic);
            for x in 0..game.world.width {
                game.world.set_tile(x, 10, "oBlock");
            }
            // Beyond the parent's view+16 movement gate but still inside
            // oLevel's wider view+96 active-instance region.
            let id = game.spawn(object, Vec2::new(360.0, 144.0));
            for tick in 1..=30 {
                game.step(&InputFrame::default());
                let entity = game.world.entity(id).unwrap();
                let expected = (tick as f64 * speed).rem_euclid(frames);
                let delta = (entity.animation - expected).abs();
                assert!(
                    delta.min((delta - frames).abs()) < 1e-9,
                    "{object} tick {tick}: {} instead of {expected}",
                    entity.animation,
                );
            }
        }
    }

    #[test]
    fn rolling_dice_and_held_dribble_use_the_current_sprite_frame_count() {
        let mut game = Game::shell(7, RulesPreset::Classic);
        let die = game.spawn("oDice", Vec2::new(96.0, 64.0));
        game.world.entity_mut(die).unwrap().vel.x = 4.0;
        for tick in 1..=10 {
            game.step(&InputFrame::default());
            let die = game.world.entity(die).unwrap();
            assert_eq!(die.sprite, "sDiceRoll");
            assert_eq!(die.animation, (tick % 6) as f64);
        }

        let mut game = Game::shell(7, RulesPreset::Classic);
        for x in 0..game.world.width {
            game.world.set_tile(x, 10, "oBlock");
        }
        game.player.pos = Vec2::new(104.0, 152.0);
        game.player.state = PlayerState::Standing;
        game.player.grounded = true;
        let ball = game.spawn("oBasketball", game.player.pos);
        game.world.entity_mut(ball).unwrap().held = true;
        game.player.held = Some(ball);
        for tick in 1..=7 {
            game.step(&InputFrame::default());
            let ball = game.world.entity(ball).unwrap();
            assert_eq!(ball.sprite, "sDribble");
            assert_eq!(ball.animation, (tick % 3) as f64);
        }
    }

    #[test]
    fn flare_crate_emits_on_first_step_then_every_other_step() {
        let mut game = Game::shell(7, RulesPreset::Classic);
        let id = game.spawn("oFlareCrate", Vec2::new(96.0, 64.0));
        let mut counts = Vec::new();
        for _ in 0..6 {
            game.step(&InputFrame::default());
            counts.push(
                game.world
                    .entities
                    .iter()
                    .filter(|e| e.alive && e.object == "oFlareSpark")
                    .count(),
            );
        }
        assert!(game.world.entity(id).is_some());
        assert_eq!(counts, vec![1, 1, 2, 2, 3, 3]);
    }

    #[test]
    fn flare_crate_alarm_emits_before_the_crate_moves() {
        let mut game = Game::shell(7, RulesPreset::Classic);
        let id = game.spawn("oFlareCrate", Vec2::new(96.0, 64.0));
        game.world.entity_mut(id).unwrap().vel.x = 20.0;
        game.step(&InputFrame::default());
        assert_eq!(game.world.entity(id).unwrap().pos.x, 116.0);
        let spark = game
            .world
            .entities
            .iter()
            .find(|e| e.object == "oFlareSpark")
            .unwrap();
        assert!((93.0..=99.0).contains(&spark.pos.x));
    }

    #[test]
    fn held_flare_crate_is_destroyed_in_water_and_releases_the_hand() {
        for water in ["oWater", "oWaterSwim"] {
            let mut game = Game::shell(7, RulesPreset::Classic);
            game.world.set_tile(2, 2, water);
            let id = game.spawn("oFlareCrate", game.player.pos);
            game.world.entity_mut(id).unwrap().held = true;
            game.player.held = Some(id);
            game.step(&InputFrame::default());
            assert!(game.world.entity(id).is_none(), "{water}");
            assert_eq!(game.player.held, None, "{water}");
            assert!(
                game.events
                    .iter()
                    .any(|e| { matches!(e, GameEvent::Sound { name, .. } if name == "sndSplash") })
            );
        }
    }

    #[test]
    fn utility_items_settle_using_create_event_bounds() {
        for object in ["oTeleporter", "oLampItem", "oLampRedItem", "oBasketball"] {
            let mut game = Game::shell(7, RulesPreset::Classic);
            for x in 0..game.world.width {
                game.world.set_tile(x, 6, "oBlock");
            }
            let id = game.spawn(object, Vec2::new(104.0, 72.0));
            for _ in 0..100 {
                game.step(&InputFrame::default());
            }
            let entity = game.world.entity(id).unwrap();
            assert_eq!(entity.pos.y, 92.0, "{object}");
            assert_eq!(entity.vel.y, 0.0, "{object}");
        }
    }

    #[test]
    fn unsupported_die_cannot_finish_a_roll_against_a_side_wall() {
        let mut game = Game::shell(7, RulesPreset::Classic);
        game.world.set_tile(6, 5, "oBlock");
        let id = game.spawn("oDice", Vec2::new(91.0, 88.0));
        let die = game.world.entity_mut(id).unwrap();
        die.sprite = "sDiceRoll".into();
        die.data.insert("value".into(), 3.0);
        die.data.insert("rolling".into(), 1.0);
        tick(&mut game);
        let die = game.world.entity(id).unwrap();
        assert_eq!(die.sprite, "sDiceRoll");
        assert_eq!(die.value("rolled"), 0.0);
        assert_eq!(die.value("rolling"), 1.0);
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
        // Item use itself does not run the later Player Hurt block.
        assert_eq!(game.player.health, 4);
        assert!(!game.events.contains(&GameEvent::Damage { amount: 99 }));
    }

    #[test]
    fn teleport_destroys_only_first_overlapping_enemy_even_if_invincible() {
        let (mut game, id) = held_teleporter(11);
        let distance = 16.0 * f64::from(4 + game.rng.clone().range(5));
        let pos = Vec2::new(160.0 + distance, 160.0);
        let snake = game.spawn("oSnake", pos);
        game.world
            .entity_mut(snake)
            .unwrap()
            .data
            .insert("invincible".into(), 1.0);
        let second = game.spawn("oSnake", pos);
        use_utility(&mut game, id, &InputFrame::default());
        assert!(game.world.entity(snake).is_none());
        assert!(game.world.entity(second).is_some());
    }

    #[test]
    fn landing_probe_includes_the_source_right_endpoint() {
        let (mut game, id) = held_teleporter(11);
        game.player.pos.x = 156.0;
        let distance = 16.0 * f64::from(4 + game.rng.clone().range(5));
        let target_x = game.player.pos.x + distance;
        // The block begins exactly at tx+4, the last included source pixel.
        game.world
            .set_tile(((target_x + 4.0) / 16.0) as usize, 10, "oBlock");
        use_utility(&mut game, id, &InputFrame::default());
        assert_eq!(game.player.pos, Vec2::new(target_x, 144.0));
        assert_eq!(game.player.health, 4);
    }

    #[test]
    fn teleport_does_not_invent_outside_room_crushing() {
        let (mut game, id) = held_teleporter(11);
        game.player.pos.y = -8.0;
        use_utility(&mut game, id, &InputFrame::default());
        assert_eq!(game.player.pos.y, -8.0);
        assert_eq!(game.player.health, 4);
    }

    #[test]
    fn full_step_telefrag_uses_sprite_masks_and_resets_carried_presentation() {
        let (mut game, id) = held_teleporter(11);
        game.player.pos.y = 168.0;
        game.player.state = PlayerState::Standing;
        game.player.sprite = "sStandLeft".into();
        game.player.grounded = true;
        for x in 0..game.world.width {
            game.world.set_tile(x, 11, "oBlock");
        }
        let distance = 16.0 * f64::from(4 + game.rng.clone().range(5));
        let target = Vec2::new(160.0 + distance, 168.0);
        // The snake's source sprite starts at +1,+4. Its leftmost mask pixel
        // intersects the right-facing player's +6 sprite pixel, outside the
        // movement bounds (and outside the unmirrored sprite mask).
        let snake = game
            .world
            .spawn("oSnake", Vec2::new(target.x + 5.0, target.y - 4.0));
        game.world
            .entity_mut(snake)
            .unwrap()
            .data
            .insert("invincible".into(), 1.0);
        game.step(&InputFrame {
            attack_pressed: true,
            ..InputFrame::default()
        });
        assert_eq!(game.player.pos, target);
        assert!(game.world.entity(snake).is_none());
        assert_eq!(game.player.health, 4);
        assert_eq!(game.player.pos, game.player.previous_pos);
        assert_eq!(game.camera, game.previous_camera);
        assert!(game.flags.contains("render_discontinuity"));
        assert_eq!(
            game.world.entity(id).unwrap().pos,
            Vec2::new(target.x + 4.0, target.y + 2.0)
        );
        // The renderer bypasses interpolation for this tick, then Game::step
        // consumes the discontinuity before capturing the next snapshots.
        game.step(&InputFrame::default());
        assert!(!game.flags.contains("render_discontinuity"));
    }

    #[test]
    fn full_step_failed_teleport_crushes_through_invulnerability() {
        let (mut game, id) = held_teleporter(11);
        game.player.pos.y = 168.0;
        game.player.state = PlayerState::Standing;
        game.player.grounded = true;
        game.player.invincible = 100;
        for x in 0..game.world.width {
            game.world.set_tile(x, 11, "oBlock");
        }
        let distance = 16.0 * f64::from(4 + game.rng.clone().range(5));
        let tx = ((160.0 + distance) / 16.0) as usize;
        for y in 0..11 {
            game.world.set_tile(tx - 1, y, "oBlock");
            game.world.set_tile(tx, y, "oBlock");
        }
        game.step(&InputFrame {
            attack_pressed: true,
            ..InputFrame::default()
        });
        assert_eq!(game.state, RunState::Dead);
        assert_eq!(game.player.health, 0);
        assert!(game.events.contains(&GameEvent::Damage { amount: 99 }));
        assert!(game.events.contains(&GameEvent::Died));
        // The held teleporter remains an ordinary item until the next update;
        // this asserts the failed use didn't silently become a throw.
        assert_eq!(game.player.held, Some(id));
    }

    #[test]
    fn utility_defaults_follow_source_and_dice_faces_are_seeded() {
        let mut game = Game::shell(7, RulesPreset::Classic);
        let ball = game.spawn("oBasketball", Vec2::new(80.0, 80.0));
        let die = game.spawn("oDice", Vec2::new(80.0, 80.0));
        let lamp = game.spawn("oLampItem", Vec2::new(80.0, 80.0));
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
        game.step(&InputFrame::default());
        for (id, origin) in &sparks {
            assert!((game.world.entity(*id).unwrap().pos.y - (origin.y - 0.1)).abs() < 1e-8);
        }
        for _ in 1..12 {
            game.step(&InputFrame::default());
        }
        assert!(
            sparks
                .iter()
                .all(|(id, _)| game.world.entity(*id).is_some())
        );
        game.step(&InputFrame::default());
        assert!(
            sparks
                .iter()
                .all(|(id, _)| game.world.entity(*id).is_none())
        );
    }
}
