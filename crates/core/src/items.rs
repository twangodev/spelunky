//! Classic item interactions. Reference: oItem, oPlayer1 Step and scrUseItem.
use crate::model::*;
#[path = "items/collectibles.rs"]
pub mod collectibles;
#[path = "items/explosives.rs"]
pub mod explosives;
#[path = "items/utility.rs"]
pub mod utility;
#[path = "items/weapons.rs"]
pub mod weapons;

pub fn is_item(name: &str) -> bool {
    collectibles::is_collectible(name)
        || matches!(
            name,
            "oRock"
                | "oSkull"
                | "oJar"
                | "oFishBone"
                | "oGoldIdol"
                | "oCrystalSkull"
                | "oLampItem"
                | "oLampRedItem"
                | "oBasketball"
                | "oDice"
                | "oBomb"
                | "oFireFrogBomb"
                | "oFireFrogArmed"
                | "oBall"
                | "oBall2"
                | "oRope"
                | "oRopeThrow"
                | "oArrow"
                | "oBow"
                | "oPistol"
                | "oShotgun"
                | "oSceptre"
                | "oWebCannon"
                | "oMachete"
                | "oMattock"
                | "oTeleporter"
                | "oFlare"
                | "oChest"
                | "oLockedChest"
                | "oKey"
                | "oCrate"
        )
}

pub fn init_entity(entity: &mut Entity) {
    if entity.value("items_initialized") != 0.0 {
        return;
    }
    entity.data.insert("items_initialized".into(), 1.0);
    if is_item(&entity.object) && !matches!(entity.object.as_str(), "oArrow" | "oFlare") {
        entity.data.entry("gravity".into()).or_insert(0.6);
        entity.data.entry("bounceFactor".into()).or_insert(0.5);
        entity.data.entry("frictionFactor".into()).or_insert(0.3);
        entity.data.entry("canPickUp".into()).or_insert(1.0);
        entity.data.entry("damage".into()).or_insert(1.0);
    }
    if matches!(
        entity.object.as_str(),
        "oGoldIdol" | "oCrystalSkull" | "oLampItem" | "oLampRedItem"
    ) {
        entity.data.insert("heavy".into(), 1.0);
    }
    if matches!(entity.object.as_str(), "oGoldIdol" | "oCrystalSkull") {
        entity.data.entry("trigger".into()).or_insert(1.0);
        let value = if entity.object == "oCrystalSkull" {
            15000.0
        } else {
            5000.0
        };
        entity.data.entry("value".into()).or_insert(value);
    }
    if entity.object == "oFireFrogArmed" {
        entity.data.insert("heavy".into(), 1.0);
        for (key, value) in [
            ("bboxLeft", -6.0),
            ("bboxTop", 0.0),
            ("bboxRight", 6.0),
            ("bboxBottom", 8.0),
        ] {
            entity.data.insert(key.into(), value);
        }
    }
    collectibles::init_entity(entity);
    explosives::init_entity(entity);
    weapons::init_entity(entity);
    utility::init_entity(entity);
}

pub fn tick(game: &mut Game, input: &InputFrame) {
    for entity in &mut game.world.entities {
        init_entity(entity);
    }
    game.player
        .data
        .insert("attack_held".into(), f64::from(input.attack));
    if game
        .player
        .held
        .is_some_and(|id| game.world.entity(id).is_none())
    {
        game.player.held = None;
    }
    if game.player.health <= 0 || game.player.stun > 0 {
        crate::characters::tick(game, input);
        if let Some(id) = game.player.held.take() {
            if let Some(entity) = game.world.entity_mut(id) {
                entity.held = false;
                entity.vel = Vec2::new(f64::from(game.player.facing) * 2.0, -2.0);
            }
        }
    } else {
        if input.cycle_pressed && game.player.attack_timer == 0 {
            explosives::cycle_inventory(game);
        }
        let interacted = collectibles::interact(game, input);
        let character_input = InputFrame {
            attack_pressed: input.attack_pressed && !interacted,
            ..*input
        };
        crate::characters::tick(game, &character_input);
        if !interacted && game.player.attack_timer == 0 && input.attack_pressed {
            if let Some(id) = game.player.held {
                if game
                    .world
                    .entity(id)
                    .is_some_and(|e| e.object == "oRopeThrow")
                {
                    explosives::use_held_rope(game, id, input);
                } else if game
                    .world
                    .entity(id)
                    .is_some_and(|e| e.object == "oBomb" && e.value("armed") == 0.0)
                {
                    explosives::arm_bomb(game.world.entity_mut(id).unwrap());
                } else if input.down {
                    throw_item(game, id, input);
                } else if !weapons::use_weapon(game, id, input)
                    && !utility::use_utility(game, id, input)
                {
                    throw_item(game, id, input);
                }
            } else if input.down {
                pick_up(game);
            } else if !game.flags.contains("character_tunnelman") {
                start_whip(game);
            }
        }
        if game.player.attack_timer == 0 {
            if input.bomb_pressed && !weapons::arm_bomb_arrow(game) {
                explosives::use_bomb(game, input);
            }
            if input.rope_pressed {
                explosives::use_rope(game, input);
            }
        }
    }
    if game
        .player
        .data
        .get("character_attack_active")
        .copied()
        .unwrap_or(0.0)
        == 0.0
    {
        tick_whip(game);
    }
    attach_held(game);
    tick_bodies(game);
    explosives::tick(game);
    weapons::tick(game);
    collectibles::tick(game);
    utility::tick(game);
    // A projectile subsystem may have destroyed the carried entity this tick.
    if game
        .player
        .held
        .is_some_and(|id| game.world.entity(id).is_none())
    {
        game.player.held = None;
    }
}

fn pick_up(game: &mut Game) {
    let area = Rect::new(game.player.pos.x - 8.0, game.player.pos.y, 16.0, 8.0);
    let nearest = game
        .world
        .entities
        .iter()
        .filter(|entity| {
            entity.alive
                && !entity.held
                && entity.value("canPickUp") != 0.0
                && (is_item(&entity.object) || entity.state >= 98)
                && area.intersects(entity.bounds())
                && !game.world.solid_at(entity.pos.x, entity.pos.y)
        })
        .min_by(|a, b| {
            a.pos
                .distance(game.player.pos)
                .total_cmp(&b.pos.distance(game.player.pos))
        })
        .map(|e| e.id);
    if let Some(id) = nearest {
        let entity = game.world.entity_mut(id).unwrap();
        entity.held = true;
        if entity.object == "oBow" && entity.value("bow_picked_up") == 0.0 {
            entity.data.insert("bow_picked_up".into(), 1.0);
            *game.data.entry("arrows".into()).or_default() += 6.0;
        }
        game.player.held = Some(id);
        utility::on_pickup(game, id);
    }
}

fn throw_item(game: &mut Game, id: u64, input: &InputFrame) {
    let facing = f64::from(game.player.facing);
    let pos = game.player.pos;
    let velocity = game.player.vel;
    let grounded = game.player.grounded;
    let mitt = game.player.has("Mitt");
    let ducking = input.down && grounded;
    let throwing_bomb = game.world.entity(id).is_some_and(|e| e.object == "oBomb");
    let wall = game
        .world
        .collides_except(Rect::new(pos.x + facing * 8.0, pos.y, 0.001, 0.001), id);
    if let Some(entity) = game.world.entity_mut(id) {
        let heavy = entity.value("heavy") != 0.0 || entity.object == "oDamsel";
        entity.held = false;
        entity.vel.x = facing * if heavy { 4.0 } else { 8.0 } + velocity.x;
        entity.vel.y = if input.up {
            if heavy { -4.0 } else { -9.0 }
        } else if heavy {
            -2.0
        } else {
            -3.0
        };
        entity.data.insert("safe".into(), 10.0);
        entity.data.insert("armed".into(), 1.0);
        entity.data.insert("owner".into(), 1.0);
        if wall {
            entity.pos.x -= facing * 8.0;
        }
        if input.down {
            if grounded {
                entity.pos.y -= 2.0;
                entity.vel.x *= if matches!(
                    entity.object.as_str(),
                    "oMachete"
                        | "oMattock"
                        | "oShotgun"
                        | "oBow"
                        | "oPistol"
                        | "oSceptre"
                        | "oWebCannon"
                        | "oTeleporter"
                ) {
                    0.4
                } else {
                    0.6
                };
                entity.vel.y = 0.5;
            } else {
                entity.vel.y = 3.0;
            }
        }
        if mitt && !ducking {
            entity.vel.x += if entity.vel.x < 0.0 { -6.0 } else { 6.0 };
            if !input.up && !input.down {
                entity.vel.y = -0.4;
            } else if input.down {
                entity.vel.y = 6.0;
            }
            entity.data.insert("gravity".into(), 0.1);
        }
    }
    game.player.held = None;
    if throwing_bomb {
        explosives::restore_stashed_item(game);
    }
    game.sound("sndThrow", pos);
}

fn attach_held(game: &mut Game) {
    let Some(id) = game.player.held else {
        return;
    };
    let pos = game.player.pos;
    let facing = game.player.facing;
    let ducking = game.player.state == PlayerState::Ducking && game.player.vel.x.abs() < 2.0;
    if let Some(entity) = game.world.entity_mut(id) {
        entity.held = true;
        entity.vel = Vec2::default();
        let heavy = entity.value("heavy") != 0.0;
        let large = matches!(
            entity.object.as_str(),
            "oGoldIdol" | "oCrystalSkull" | "oLampItem" | "oLampRedItem" | "oDamsel"
        );
        let y_offset = if large {
            if ducking { 2.0 } else { 0.0 }
        } else if heavy {
            if ducking { -2.0 } else { -4.0 }
        } else if ducking {
            4.0
        } else {
            2.0
        };
        entity.pos = Vec2::new(pos.x + f64::from(facing) * 4.0, pos.y + y_offset);
        entity.facing = facing;
    }
}

fn start_whip(game: &mut Game) {
    game.player.attack_timer = 12;
    game.player.sprite = if game.flags.contains("character_damsel") {
        "sDamselAttackL"
    } else {
        "sAttackLeft"
    }
    .into();
    game.player.animation = 0.0;
    game.player.data.insert("whip_elapsed".into(), 0.0);
    let id = game.world.spawn(
        "oWhipPre",
        Vec2::new(
            game.player.pos.x - f64::from(game.player.facing) * 16.0,
            game.player.pos.y,
        ),
    );
    let entity = game.world.entity_mut(id).unwrap();
    entity.facing = game.player.facing;
    entity.timer = 4;
    entity.data.insert("damage".into(), 1.0);
    entity.data.insert("owner".into(), 1.0);
    entity.sprite = if entity.facing < 0 {
        "sWhipPreL"
    } else {
        "sWhipPreR"
    }
    .into();
}

fn tick_whip(game: &mut Game) {
    if game.player.attack_timer > 0 {
        let elapsed = game.player.data.entry("whip_elapsed".into()).or_default();
        *elapsed += 1.0;
        if game.player.held.is_none() {
            game.player.animation = 0.6 + (*elapsed - 1.0);
        }
        if *elapsed == 6.0 && game.player.held.is_none() {
            let id = game.world.spawn(
                "oWhip",
                Vec2::new(
                    game.player.pos.x + f64::from(game.player.facing) * 16.0,
                    game.player.pos.y,
                ),
            );
            let entity = game.world.entity_mut(id).unwrap();
            entity.facing = game.player.facing;
            entity.timer = 7;
            entity.data.insert("damage".into(), 1.0);
            entity.data.insert("owner".into(), 1.0);
            entity.sprite = if entity.facing < 0 {
                "sWhipLeft"
            } else {
                "sWhipRight"
            }
            .into();
            game.sound("sndWhip", game.player.pos);
        }
        game.player.attack_timer -= 1;
    }
    for entity in &mut game.world.entities {
        if !entity.alive || !matches!(entity.object.as_str(), "oWhip" | "oWhipPre") {
            continue;
        }
        let direction = if entity.object == "oWhipPre" {
            -1.0
        } else {
            1.0
        };
        entity.pos = Vec2::new(
            game.player.pos.x + direction * f64::from(entity.facing) * 16.0,
            game.player.pos.y,
        );
        entity.timer = entity.timer.saturating_sub(1);
        if entity.timer == 0 || game.player.health <= 0 || game.player.attack_timer == 0 {
            entity.alive = false;
        }
    }
}

fn ordinary_body(name: &str) -> bool {
    is_item(name)
        && !matches!(
            name,
            "oBomb"
                | "oFireFrogBomb"
                | "oRope"
                | "oRopeThrow"
                | "oArrow"
                | "oFlare"
                | "oBall"
                | "oBall2"
        )
}

fn body_bounds(entity: &Entity, pos: Vec2) -> Rect {
    if entity.data.contains_key("bboxLeft") {
        return Rect::new(
            pos.x + entity.value("bboxLeft"),
            pos.y + entity.value("bboxTop"),
            entity.value("bboxRight") - entity.value("bboxLeft"),
            entity.value("bboxBottom") - entity.value("bboxTop"),
        );
    }
    if matches!(
        entity.object.as_str(),
        "oRock" | "oSkull" | "oFishBone" | "oGoldIdol" | "oCrystalSkull"
    ) {
        return Rect::new(pos.x - 4.0, pos.y - 4.0, 8.0, 8.0);
    }
    if entity.object == "oJar" {
        return Rect::new(pos.x - 4.0, pos.y - 6.0, 8.0, 12.0);
    }
    let bounds = entity.bounds();
    Rect::new(
        bounds.x + pos.x - entity.pos.x,
        bounds.y + pos.y - entity.pos.y,
        bounds.w,
        bounds.h,
    )
}

fn tick_bodies(game: &mut Game) {
    let mut broken = Vec::new();
    for index in 0..game.world.entities.len() {
        let mut entity = game.world.entities[index].clone();
        if !entity.alive || entity.held || !ordinary_body(&entity.object) {
            continue;
        }
        if entity.value("safe") > 0.0 {
            *entity.data.get_mut("safe").unwrap() -= 1.0;
        }
        let velocity = entity.vel;
        let fragile = matches!(entity.object.as_str(), "oJar" | "oSkull");
        let mut destroy = entity.value("broken") > 0.0;
        // Substeps preserve original integer-pixel wall contacts without tunneling.
        let steps = velocity.x.abs().max(velocity.y.abs()).ceil().max(1.0) as u32;
        let mut hit_x = false;
        let mut hit_y = false;
        for _ in 0..steps {
            let x = Vec2::new(entity.pos.x + velocity.x / f64::from(steps), entity.pos.y);
            if !game
                .world
                .collides_except(body_bounds(&entity, x), entity.id)
            {
                entity.pos.x = x.x;
            } else {
                hit_x = true;
            }
            let y = Vec2::new(entity.pos.x, entity.pos.y + velocity.y / f64::from(steps));
            if !game
                .world
                .collides_except(body_bounds(&entity, y), entity.id)
            {
                entity.pos.y = y.y;
            } else {
                hit_y = true;
            }
        }
        if hit_x {
            if fragile && velocity.x.abs() > if entity.object == "oSkull" { 2.0 } else { 3.0 } {
                destroy = true;
            }
            entity.vel.x = -velocity.x * 0.5;
            entity.data.insert("gravity".into(), 0.6);
        }
        if hit_y {
            if fragile && (velocity.y > 3.0 || velocity.y < -3.0) {
                destroy = true;
            }
            if velocity.y > 0.0 {
                entity.vel.y = if velocity.y > 1.0 {
                    -velocity.y * entity.value("bounceFactor")
                } else {
                    0.0
                };
                entity.vel.x = if entity.vel.x.abs() < 0.1 {
                    0.0
                } else {
                    entity.vel.x * entity.value("frictionFactor")
                };
            } else {
                entity.vel.y = -velocity.y * 0.8;
            }
            entity.data.insert("gravity".into(), 0.6);
        } else if !game.world.collides_except(
            body_bounds(&entity, Vec2::new(entity.pos.x, entity.pos.y + 1.0)),
            entity.id,
        ) {
            entity.vel.y =
                (entity.vel.y + entity.value("gravity")).min(if fragile { 6.0 } else { 8.0 });
        }
        let tile = game.world.tile(
            (entity.pos.x / 16.0).floor() as i32,
            ((entity.pos.y - 5.0) / 16.0).floor() as i32,
        );
        if tile.is_some_and(|t| t.object == "oLava") && entity.object != "oSceptre" {
            destroy = true;
        }
        if destroy {
            broken.push(entity.id);
        }
        game.world.entities[index] = entity;
    }
    for id in broken {
        break_item(game, id);
    }
}

/// Called by combat when a fragile thrown body breaks on a creature.
pub fn break_item(game: &mut Game, id: u64) {
    let Some(entity) = game.world.entities.iter_mut().find(|e| e.id == id) else {
        return;
    };
    if entity.value("break_done") != 0.0 {
        return;
    }
    entity.data.insert("break_done".into(), 1.0);
    entity.alive = false;
    let pos = entity.pos;
    let jar = entity.object == "oJar";
    if game.player.held == Some(id) {
        game.player.held = None;
    }
    game.sound("sndBreak", pos);
    if jar {
        let loot = if game.rng.chance(1, 3) {
            Some("oGoldChunk")
        } else if game.rng.chance(1, 6) {
            Some("oGoldNugget")
        } else if game.rng.chance(1, 12) {
            Some("oEmeraldBig")
        } else if game.rng.chance(1, 12) {
            Some("oSapphireBig")
        } else if game.rng.chance(1, 12) {
            Some("oRubyBig")
        } else if game.rng.chance(1, 6) {
            Some("oSpider")
        } else if game.rng.chance(1, 12) {
            Some("oSnake")
        } else {
            None
        };
        if let Some(name) = loot {
            game.world.spawn(name, pos);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn game() -> Game {
        Game::shell(7, RulesPreset::Classic)
    }
    #[test]
    fn pick_up_requires_down_and_attack_and_throw_uses_mitt() {
        let mut game = game();
        game.player.pos = Vec2::new(40.0, 40.0);
        let id = game.world.spawn("oRock", Vec2::new(40.0, 44.0));
        init_entity(game.world.entity_mut(id).unwrap());
        pick_up(&mut game);
        assert_eq!(game.player.held, Some(id));
        game.player.equipment.insert("Mitt".into());
        throw_item(&mut game, id, &InputFrame::default());
        assert_eq!(game.player.held, None);
        let rock = game.world.entity(id).unwrap();
        assert_eq!(rock.vel, Vec2::new(14.0, -0.4));
        assert_eq!(rock.value("gravity"), 0.1);
    }
    #[test]
    fn whip_has_rear_windup_and_delayed_forward_hit() {
        let mut game = game();
        start_whip(&mut game);
        assert_eq!(game.player.sprite, "sAttackLeft");
        assert_eq!(game.player.animation, 0.0);
        assert_eq!(game.world.entities[0].pos.x, 24.0);
        for _ in 0..5 {
            tick_whip(&mut game);
        }
        assert!(
            !game
                .world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oWhip")
        );
        tick_whip(&mut game);
        assert!(
            game.world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oWhip" && e.pos.x == 56.0)
        );
    }
    #[test]
    fn tunnel_man_innate_attack_digs_and_damages_through_game_step() {
        let mut game = game();
        crate::characters::apply(&mut game, crate::characters::Character::TunnelMan);
        for x in 0..game.world.width {
            game.world.set_tile(x, 4, "oBlock");
        }
        game.world.set_tile(3, 3, "oBlock");
        game.player.pos = Vec2::new(40.0, 56.0);
        game.player.previous_pos = game.player.pos;
        game.player.grounded = true;
        game.player.state = PlayerState::Standing;
        let mut damage_game = game.clone();
        damage_game.world.set_tile(3, 3, "");
        let enemy = damage_game.world.spawn("oCaveman", Vec2::new(50.0, 48.0));
        crate::enemies::init_entity(damage_game.world.entity_mut(enemy).unwrap());
        damage_game.world.entity_mut(enemy).unwrap().state = 98;
        damage_game.world.entity_mut(enemy).unwrap().timer = 200;
        damage_game.world.entity_mut(enemy).unwrap().vel = Vec2::default();
        damage_game.step(&InputFrame {
            attack_pressed: true,
            ..InputFrame::default()
        });
        for _ in 0..11 {
            damage_game.step(&InputFrame::default());
        }
        game.step(&InputFrame {
            attack_pressed: true,
            ..InputFrame::default()
        });
        assert_eq!(game.player.sprite, "sTunnelAttackL");
        assert!(
            !game
                .world
                .entities
                .iter()
                .any(|e| matches!(e.object.as_str(), "oWhip" | "oWhipPre"))
        );
        for _ in 0..11 {
            game.step(&InputFrame::default());
        }
        assert_eq!(game.world.tile(3, 3).unwrap().object, "");
        assert!(
            damage_game
                .world
                .entity(enemy)
                .is_none_or(|e| e.health <= 1),
            "enemy={:?}",
            damage_game.world.entity(enemy)
        );
        assert_eq!(game.player.attack_timer, 0);
        assert!(!game.player.data.contains_key("character_attack_active"));
        assert!(game.player.held.is_none());
        assert_eq!((game.player.bombs, game.player.ropes), (0, 0));
    }

    #[test]
    fn container_interaction_consumes_tunnel_attack_edge() {
        let mut game = game();
        crate::characters::apply(&mut game, crate::characters::Character::TunnelMan);
        game.player.grounded = true;
        let pos = game.player.pos;
        game.world.spawn("oCrate", pos);
        tick(
            &mut game,
            &InputFrame {
                up: true,
                attack_pressed: true,
                ..InputFrame::default()
            },
        );
        assert!(!game.player.data.contains_key("character_attack_active"));
        assert!(
            !game
                .world
                .entities
                .iter()
                .any(|e| matches!(e.object.as_str(), "oMattockPre" | "oWhipPre"))
        );
    }

    #[test]
    fn fire_frog_armed_is_heavy_body_and_ball_physics_is_delegated() {
        let mut game = game();
        let id = game.world.spawn("oFireFrogArmed", Vec2::new(40.0, 40.0));
        init_entity(game.world.entity_mut(id).unwrap());
        let frog = game.world.entity(id).unwrap();
        assert!(is_item(&frog.object));
        assert_eq!(frog.value("heavy"), 1.0);
        assert_eq!(
            body_bounds(frog, frog.pos),
            Rect::new(34.0, 40.0, 12.0, 8.0)
        );
        assert!(is_item("oBall") && is_item("oBall2"));
        assert!(!ordinary_body("oBall") && !ordinary_body("oBall2"));
    }

    #[test]
    fn throwing_reserved_bomb_restores_carried_item() {
        let mut game = game();
        let rock = game.world.spawn("oRock", game.player.pos);
        init_entity(game.world.entity_mut(rock).unwrap());
        game.player.held = Some(rock);
        game.world.entity_mut(rock).unwrap().held = true;
        explosives::cycle_inventory(&mut game);
        let bomb = game.player.held.unwrap();
        explosives::arm_bomb(game.world.entity_mut(bomb).unwrap());
        throw_item(&mut game, bomb, &InputFrame::default());
        assert_eq!(game.player.held, Some(rock));
        assert!(!game.world.entity(bomb).unwrap().held);
        assert_eq!(game.player.bombs, 3);
        assert_eq!(game.world.entity(rock).unwrap().value("stashed"), 0.0);
    }

    #[test]
    fn thrown_rock_bounces_from_dynamic_push_block() {
        let mut game = game();
        let block = game.world.spawn("oPushBlock", Vec2::new(64.0, 32.0));
        game.world.entity_mut(block).unwrap().solid = true;
        let id = game.world.spawn("oRock", Vec2::new(55.0, 40.0));
        let entity = game.world.entity_mut(id).unwrap();
        init_entity(entity);
        entity.vel.x = 10.0;
        tick_bodies(&mut game);
        let rock = game.world.entity(id).unwrap();
        assert!(rock.pos.x < 64.0);
        assert!(rock.vel.x < 0.0);
    }

    #[test]
    fn fast_jar_breaks_at_wall_without_tunneling() {
        let mut game = game();
        game.world.set_tile(4, 2, "oBlock");
        let id = game.world.spawn("oJar", Vec2::new(55.0, 40.0));
        let entity = game.world.entity_mut(id).unwrap();
        init_entity(entity);
        entity.vel.x = 10.0;
        tick_bodies(&mut game);
        assert!(game.world.entity(id).is_none());
        assert!(
            game.events
                .iter()
                .any(|e| matches!(e,GameEvent::Sound{name,..} if name=="sndBreak"))
        );
    }
}
