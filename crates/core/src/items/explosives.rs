//! Bombs and ropes translated from Classic 1.1's oItem, oBomb, oRopeThrow,
//! oExplosion, and oPlayer1 events. All durations are simulation ticks.
use crate::model::*;

pub const BOMB_FUSE: u32 = 120;
pub const BOMB_FAST_FUSE: u32 = 40;
const EXPLOSION_TICKS: u32 = 13; // Ten sprite frames at image_speed 0.8.

pub fn init_entity(entity: &mut Entity) {
    if entity.value("explosives_initialized") != 0.0 {
        return;
    }
    match entity.object.as_str() {
        "oBomb" => {
            entity.sprite = "sBomb".into();
            entity.data.entry("armed".into()).or_insert(0.0);
        }
        "oFireFrogBomb" => {
            entity.data.insert("armed".into(), 1.0);
            entity.data.insert("heavy".into(), 1.0);
            entity.timer = BOMB_FUSE;
        }
        "oRopeThrow" => {
            entity.sprite = "sRopeEnd".into();
        }
        "oExplosion" => {
            entity.sprite = "sExplosion".into();
            entity.timer = EXPLOSION_TICKS;
            entity.animation = 0.0;
        }
        "oRope" | "oRopeTop" => {}
        "oRopeBurn" => {
            entity.sprite = "sRopeBurn".into();
            entity.vel.y = -1.0;
        }
        _ => return,
    }
    entity.data.insert("explosives_initialized".into(), 1.0);
}

fn can_use(game: &Game) -> bool {
    game.state == RunState::Playing
        && game.player.health > 0
        && game.player.stun == 0
        && game.player.attack_timer == 0
}

/// First use arms a carried bomb; subsequent use can throw it normally.
pub fn arm_bomb(bomb: &mut Entity) {
    init_entity(bomb);
    if bomb.value("armed") == 0.0 {
        bomb.data.insert("armed".into(), 1.0);
        bomb.sprite = "sBombArmed".into();
        bomb.timer = BOMB_FUSE;
        bomb.animation = 0.0;
    }
}

pub fn restore_stashed_item(game: &mut Game) {
    let id = game.player.data.remove("stash_id").unwrap_or(0.0) as u64;
    if let Some(entity) = game.world.entity_mut(id) {
        entity.held = true;
        entity.pos = game.player.pos;
        entity.previous_pos = entity.pos;
        entity.vel = Vec2::default();
        entity.data.remove("stashed");
        game.player.held = Some(id);
    }
}

fn equip_inventory_item(game: &mut Game, kind: &str) {
    let id = game.world.spawn(kind, game.player.pos);
    let paste = game.player.has("Paste");
    let entity = game.world.entity_mut(id).unwrap();
    init_entity(entity);
    entity.held = true;
    if kind == "oBomb" {
        entity.data.insert("sticky".into(), f64::from(paste));
        game.player.bombs -= 1;
    } else {
        game.player.ropes -= 1;
    }
    game.player.held = Some(id);
}

/// Cycle a free light item through reserved bomb, reserved rope, and the item.
pub fn cycle_inventory(game: &mut Game) {
    if !can_use(game) {
        return;
    }
    let held = game
        .player
        .held
        .and_then(|id| game.world.entity(id).cloned());
    if let Some(entity) = held {
        if entity.object == "oBomb" && entity.value("armed") != 0.0 {
            return;
        }
        if entity.object == "oBomb" || entity.object == "oRopeThrow" {
            game.world.entity_mut(entity.id).unwrap().alive = false;
            game.player.held = None;
            if entity.object == "oBomb" {
                game.player.bombs += 1;
                if game.player.ropes > 0 {
                    equip_inventory_item(game, "oRopeThrow");
                } else {
                    restore_stashed_item(game);
                }
            } else {
                game.player.ropes += 1;
                restore_stashed_item(game);
            }
            return;
        }
        if entity.value("heavy") != 0.0
            || entity.object == "oDamsel"
            || entity.value("cost") > 0.0
            || (game.player.bombs <= 0 && game.player.ropes <= 0)
        {
            return;
        }
        game.player.data.insert("stash_id".into(), entity.id as f64);
        let stashed = game.world.entity_mut(entity.id).unwrap();
        stashed.data.insert("stashed".into(), 1.0);
        stashed.pos = Vec2::new(-100_000.0, -100_000.0);
        stashed.previous_pos = stashed.pos;
        stashed.vel = Vec2::default();
        stashed.held = true;
        game.player.held = None;
    }
    if game.player.bombs > 0 {
        equip_inventory_item(game, "oBomb");
    } else if game.player.ropes > 0 {
        equip_inventory_item(game, "oRopeThrow");
    }
}

/// A carried rope has already been removed from the rope inventory.
pub fn use_held_rope(game: &mut Game, id: u64, input: &InputFrame) -> bool {
    if !game
        .world
        .entity(id)
        .is_some_and(|entity| entity.object == "oRopeThrow" && entity.held)
    {
        return false;
    }
    let before = game.player.ropes;
    game.player.ropes += 1;
    let mut rope_input = *input;
    rope_input.rope_pressed = true;
    use_rope(game, &rope_input);
    if game.player.ropes == before {
        game.world.entity_mut(id).unwrap().alive = false;
        game.player.held = None;
        restore_stashed_item(game);
    } else {
        game.player.ropes = before;
    }
    true
}

/// Dedicated bomb button. The normal inventory item starts unarmed instead.
pub fn use_bomb(game: &mut Game, input: &InputFrame) {
    if !input.bomb_pressed || game.player.bombs <= 0 || !can_use(game) {
        return;
    }
    let pos = game.player.pos;
    let mut velocity = Vec2::new(
        8.0 * f64::from(game.player.facing) + game.player.vel.x,
        -3.0,
    );
    if input.up {
        velocity.y = -9.0;
    }
    if input.down {
        velocity.y = 3.0;
        if game.player.grounded {
            velocity.x *= 0.1;
        }
    }
    let paste = game.player.has("Paste");
    let id = game.world.spawn("oBomb", pos);
    let bomb = game.world.entity_mut(id).unwrap();
    init_entity(bomb);
    bomb.vel = velocity;
    arm_bomb(bomb);
    bomb.data
        .insert("sticky".into(), if paste { 1.0 } else { 0.0 });
    bomb.data.insert("safe_ticks".into(), 10.0);
    game.player.bombs -= 1;
    game.sound("sndThrow", pos);
}

pub fn use_rope(game: &mut Game, input: &InputFrame) {
    if !input.rope_pressed || game.player.ropes <= 0 || !can_use(game) {
        return;
    }
    let pos = game.player.pos;
    if !input.down
        && game
            .world
            .collides_except(Rect::new(pos.x - 4.0, pos.y - 5.0, 8.0, 1.0), 0)
    {
        return;
    }
    let mut throw_pos = pos;
    if input.down {
        // Downward placement prefers the near side of the adjacent tile.
        let direction = f64::from(game.player.facing);
        if game.world.solid_at(pos.x + 8.0 * direction, pos.y) {
            return;
        }
        let snapped = ((pos.x + 16.0 * direction) / 16.0).round() * 16.0;
        let preferred = snapped - 8.0 * direction;
        let alternate = snapped + 8.0 * direction;
        let clear = |x| {
            !game
                .world
                .collides_except(Rect::new(x - 1.0, pos.y, 2.0, 16.0), 0)
        };
        throw_pos.x = if clear(preferred) {
            preferred
        } else if clear(alternate) {
            alternate
        } else {
            return;
        };
    }
    let id = game.world.spawn("oRopeThrow", throw_pos);
    let rope = game.world.entity_mut(id).unwrap();
    init_entity(rope);
    rope.data.insert("px".into(), pos.x);
    if input.down {
        rope.data.insert("falling".into(), 1.0);
        spawn_rope_part(game, "oRopeTop", throw_pos);
    } else {
        rope.data.insert("armed".into(), 1.0);
        rope.vel.y = -12.0;
    }
    game.player.ropes -= 1;
    game.sound("sndThrow", pos);
}

/// Uses stable IDs so entities spawned during a tick do not update early.
pub fn tick(game: &mut Game) {
    let ids: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|entity| {
            entity.alive
                && matches!(
                    entity.object.as_str(),
                    "oBomb"
                        | "oFireFrogBomb"
                        | "oRopeThrow"
                        | "oExplosion"
                        | "oRope"
                        | "oRopeTop"
                        | "oRopeBurn"
                )
        })
        .map(|entity| entity.id)
        .collect();
    for id in ids {
        let Some(mut entity) = game.world.entity(id).cloned() else {
            continue;
        };
        init_entity(&mut entity);
        match entity.object.as_str() {
            "oBomb" | "oFireFrogBomb" => tick_bomb(game, &mut entity),
            "oRopeThrow" => tick_rope(game, &mut entity),
            "oExplosion" => {
                announce_explosion(game, &mut entity);
                apply_explosion(game, &entity);
                entity.timer = entity.timer.saturating_sub(1);
                entity.animation += 0.8;
                if entity.timer == 0 {
                    entity.alive = false;
                }
            }
            "oRope" | "oRopeTop" => tick_rope_burn(game, &mut entity),
            "oRopeBurn" => tick_burning_flame(game, &mut entity),
            _ => unreachable!(),
        }
        if let Some(stored) = game
            .world
            .entities
            .iter_mut()
            .find(|stored| stored.id == id)
        {
            *stored = entity;
        }
    }
}

fn tick_bomb(game: &mut Game, bomb: &mut Entity) {
    if bomb.value("armed") != 0.0 {
        bomb.timer = bomb.timer.saturating_sub(1);
        bomb.animation += if bomb.timer <= BOMB_FAST_FUSE {
            1.0
        } else {
            0.2
        };
        if bomb.timer == 0 {
            bomb.alive = false;
            if let Some(stored) = game.world.entity_mut(bomb.id) {
                stored.alive = false;
            }
            if game.player.held == Some(bomb.id) {
                game.player.held = None;
            }
            explode(game, bomb.pos);
            return;
        }
    }
    let safe = bomb.value("safe_ticks");
    if safe > 0.0 {
        bomb.data.insert("safe_ticks".into(), safe - 1.0);
    }
    if bomb.held {
        bomb.pos = Vec2::new(
            game.player.pos.x + 4.0 * f64::from(game.player.facing),
            game.player.pos.y + 2.0,
        );
        bomb.vel = Vec2::default();
        return;
    }
    let attached = bomb.value("attached_id") as u64;
    if attached != 0 {
        if let Some(target) = game.world.entity(attached) {
            bomb.pos = Vec2::new(
                target.pos.x - bomb.value("sticky_x"),
                target.pos.y - bomb.value("sticky_y"),
            );
            bomb.vel = Vec2::default();
            return;
        }
        bomb.data.insert("attached_id".into(), 0.0);
    }
    if bomb.value("stuck") != 0.0
        && !game.world.collides_except(
            Rect::new(bomb.pos.x - 5.0, bomb.pos.y - 5.0, 10.0, 10.0),
            bomb.id,
        )
    {
        bomb.data.insert("stuck".into(), 0.0);
    }
    if bomb.value("stuck") == 0.0 {
        projectile_motion(
            &game.world,
            bomb,
            bomb.value("sticky") != 0.0 && bomb.value("armed") != 0.0,
        );
    }
    if game
        .world
        .tile(
            (bomb.pos.x / 16.0).floor() as i32,
            ((bomb.pos.y - 5.0) / 16.0).floor() as i32,
        )
        .is_some_and(|tile| tile.object == "oLava")
    {
        bomb.alive = false;
        if let Some(stored) = game.world.entity_mut(bomb.id) {
            stored.alive = false;
        }
        explode(game, bomb.pos);
        return;
    }
    if bomb.value("sticky") != 0.0 && (bomb.vel.x.abs() > 2.0 || bomb.vel.y.abs() > 2.0) {
        let contact = Rect::new(bomb.pos.x - 2.0, bomb.pos.y - 2.0, 4.0, 4.0);
        if let Some(target) = game.world.entities.iter().find(|target| {
            target.alive
                && (crate::enemies::is_enemy(&target.object) || target.object == "oDamsel")
                && contact.intersects(target.bounds())
        }) {
            bomb.data.insert("attached_id".into(), target.id as f64);
            bomb.data
                .insert("sticky_x".into(), target.pos.x - bomb.pos.x);
            bomb.data
                .insert("sticky_y".into(), target.pos.y - bomb.pos.y);
        }
    }
}

fn projectile_motion(world: &World, entity: &mut Entity, sticky: bool) {
    let steps = entity.vel.x.abs().max(entity.vel.y.abs()).ceil().max(1.0) as usize;
    let delta = Vec2::new(entity.vel.x / steps as f64, entity.vel.y / steps as f64);
    let mut side = false;
    let mut floor = false;
    let mut ceiling = false;
    for _ in 0..steps {
        let horizontal = Rect::new(entity.pos.x + delta.x - 4.0, entity.pos.y - 4.0, 8.0, 8.0);
        if delta.x != 0.0 && world.collides_except(horizontal, entity.id) {
            side = true;
        } else if !side {
            entity.pos.x += delta.x;
        }
        let vertical = Rect::new(entity.pos.x - 4.0, entity.pos.y + delta.y - 4.0, 8.0, 8.0);
        if delta.y != 0.0 && world.collides_except(vertical, entity.id) {
            if delta.y > 0.0 {
                floor = true;
            } else {
                ceiling = true;
            }
        } else if !floor && !ceiling {
            entity.pos.y += delta.y;
        }
    }
    if sticky && (side || floor || ceiling) {
        entity.vel = Vec2::default();
        entity.data.insert("stuck".into(), 1.0);
        return;
    }
    if side {
        entity.vel.x *= -0.5;
    }
    if ceiling && entity.vel.y < 0.0 {
        entity.vel.y *= -0.8;
    }
    let grounded = floor
        || world.collides_except(
            Rect::new(entity.pos.x - 4.0, entity.pos.y - 3.0, 8.0, 8.0),
            entity.id,
        );
    if grounded {
        entity.vel.y = if entity.vel.y > 1.0 {
            -entity.vel.y * 0.5
        } else {
            0.0
        };
        entity.vel.x *= 0.3;
        if entity.vel.x.abs() < 0.1 {
            entity.vel.x = 0.0;
        }
    } else {
        entity.vel.y = (entity.vel.y + 0.6).min(8.0);
    }
}

fn tick_rope(game: &mut Game, rope: &mut Entity) {
    if rope.value("falling") == 0.0 {
        projectile_motion(&game.world, rope, false);
        if rope.value("armed") != 0.0 && rope.vel.y >= 0.0 {
            let snapped = (rope.pos.x / 16.0).round() * 16.0;
            let near = if rope.value("px") < snapped {
                snapped - 8.0
            } else {
                snapped + 8.0
            };
            rope.pos.x = if game.world.solid_at(near, rope.pos.y) {
                snapped * 2.0 - near
            } else {
                near
            };
            rope.vel = Vec2::default();
            rope.data.insert("armed".into(), 0.0);
            rope.data.insert("falling".into(), 1.0);
            spawn_rope_part(game, "oRopeTop", rope.pos);
        }
    }
    if rope.value("falling") != 0.0 {
        rope.vel = Vec2::default();
        rope.pos.y += 8.0;
        let count = rope.value("fall_count") + 1.0;
        rope.data.insert("fall_count".into(), count);
        if count > 16.0
            || game.world.collides_except(
                Rect::new(rope.pos.x - 4.0, rope.pos.y - 4.0, 8.0, 9.0),
                rope.id,
            )
        {
            rope.alive = false;
        } else {
            spawn_rope_part(game, "oRope", Vec2::new(rope.pos.x - 8.0, rope.pos.y));
        }
    }
}

fn spawn_rope_part(game: &mut Game, kind: &str, pos: Vec2) {
    let id = game.world.spawn(kind, pos);
    init_entity(game.world.entity_mut(id).unwrap());
    // Climbing reads ladder tiles; rope entities remain for original rendering.
    let center_x = pos.x + if kind == "oRope" { 8.0 } else { 0.0 };
    let x = (center_x / 16.0).floor() as i32;
    let y = (pos.y / 16.0).floor() as i32;
    if game
        .world
        .tile(x, y)
        .is_some_and(|tile| tile.object.is_empty())
    {
        game.world.set_tile(x as usize, y as usize, "oRopeSegment");
    }
}

fn tick_rope_burn(game: &mut Game, rope: &mut Entity) {
    let burn = rope.value("burn_timer");
    let x = ((rope.pos.x + if rope.object == "oRope" { 8.0 } else { 0.0 }) / 16.0).floor() as i32;
    let y = (rope.pos.y / 16.0).floor() as i32;
    let in_lava = game
        .world
        .tile(x, y)
        .is_some_and(|tile| tile.object == "oLava");
    if burn == 0.0 && in_lava && rope.object == "oRope" {
        let id = game
            .world
            .spawn("oRopeBurn", Vec2::new(rope.pos.x + 8.0, rope.pos.y));
        init_entity(game.world.entity_mut(id).unwrap());
    } else if burn > 1.0 {
        rope.data.insert("burn_timer".into(), burn - 1.0);
    } else if burn == 1.0 {
        rope.alive = false;
        let another_segment = game.world.entities.iter().any(|other| {
            other.id != rope.id
                && other.alive
                && matches!(other.object.as_str(), "oRope" | "oRopeTop")
                && {
                    let center_x = other.pos.x + if other.object == "oRope" { 8.0 } else { 0.0 };
                    (center_x / 16.0).floor() as i32 == x
                        && (other.pos.y / 16.0).floor() as i32 == y
                }
        });
        if !another_segment
            && game
                .world
                .tile(x, y)
                .is_some_and(|tile| tile.object == "oRopeSegment")
        {
            game.world.set_tile(x as usize, y as usize, "");
        }
    }
}

fn tick_burning_flame(game: &mut Game, flame: &mut Entity) {
    flame.pos.y -= 1.0;
    flame.animation += 0.8;
    let near = Rect::new(flame.pos.x - 1.0, flame.pos.y - 8.0, 2.0, 16.0);
    if game.world.solid_at(flame.pos.x, flame.pos.y)
        || !game
            .world
            .entities
            .iter()
            .any(|rope| rope.alive && rope.object == "oRope" && near.intersects(rope.bounds()))
    {
        flame.alive = false;
        return;
    }
    if let Some(rope) = game
        .world
        .entities
        .iter_mut()
        .find(|rope| rope.alive && rope.object == "oRope" && rope.bounds().contains(flame.pos))
    {
        rope.data.insert("burn_timer".into(), 1.0);
    }
}

fn explode(game: &mut Game, pos: Vec2) {
    let id = game.world.spawn("oExplosion", pos);
    init_entity(game.world.entity_mut(id).unwrap());
    // Newly spawned explosion begins its first collision frame immediately.
    let mut explosion = game.world.entity(id).unwrap().clone();
    announce_explosion(game, &mut explosion);
    *game.world.entity_mut(id).unwrap() = explosion.clone();
    apply_explosion(game, &explosion);
}

fn announce_explosion(game: &mut Game, explosion: &mut Entity) {
    if explosion.value("announced") != 0.0 {
        return;
    }
    explosion.data.insert("announced".into(), 1.0);
    game.events.push(GameEvent::Explosion {
        pos: explosion.pos,
        radius: 24.0,
    });
    game.sound("sndExplosion", explosion.pos);
    game.data.insert("shake".into(), 5.0);
}

fn apply_explosion(game: &mut Game, explosion: &Entity) {
    let frame = (explosion.animation.floor() as usize).min(9);
    let hits = |bounds| mask_intersects(explosion.pos, frame, bounds);
    for y in 0..game.world.height {
        for x in 0..game.world.width {
            let bounds = Rect::new(x as f64 * 16.0, y as f64 * 16.0, 16.0, 16.0);
            if game.world.tiles[y * game.world.width + x].is_destructible() && hits(bounds) {
                destroy_terrain(game, x, y);
            }
        }
    }
    if game.player.health > 0 && game.player.invincible == 0 && hits(game.player.bounds()) {
        game.damage_player(10);
        let speed = 4.0 + f64::from(game.rng.range(3));
        game.player.vel = Vec2::new(
            if game.player.pos.x > explosion.pos.x {
                speed
            } else {
                -speed
            },
            -6.0,
        );
        game.player.stun = 100;
        game.player.data.insert("burning".into(), 50.0);
    }
    let mut magma_spawns = Vec::new();
    for entity in &mut game.world.entities {
        if !entity.alive || entity.id == explosion.id || !hits(entity.bounds()) {
            continue;
        }
        if entity.object == "oBomb" {
            // A chain reaction has a short, randomized fuse, never instant recursion.
            if entity.value("chain_explosion_id") != explosion.id as f64 {
                entity
                    .data
                    .insert("chain_explosion_id".into(), explosion.id as f64);
                entity.sprite = "sBombArmed".into();
                entity.data.insert("armed".into(), 1.0);
                entity.data.insert("attached_id".into(), 0.0);
                entity.data.insert("stuck".into(), 0.0);
                entity.timer = 4 + game.rng.range(5);
                entity.vel = Vec2::new(
                    if entity.pos.x < explosion.pos.x {
                        -3.0
                    } else {
                        3.0
                    },
                    -3.0,
                );
            }
        } else if entity.object == "oMagmaMan" {
            entity.alive = false;
            for _ in 0..2 {
                magma_spawns.push((
                    Vec2::new(entity.pos.x + 8.0, entity.pos.y - 4.0),
                    -(1.0 + f64::from(game.rng.range(3))),
                ));
            }
        } else if (crate::enemies::is_enemy(&entity.object) || entity.object == "oDamsel")
            && entity.value("invincible") == 0.0
        {
            // Avoid applying damage twice for the same persistent explosion frame.
            if entity.value("damaged_explosion_id") != explosion.id as f64 {
                entity
                    .data
                    .insert("damaged_explosion_id".into(), explosion.id as f64);
                entity.health -= if entity.object == "oDamsel" { 100 } else { 30 };
                let speed = 4.0 + f64::from(game.rng.range(3));
                entity.vel = Vec2::new(
                    if entity.pos.x > explosion.pos.x {
                        speed
                    } else {
                        -speed
                    },
                    -6.0,
                );
                entity.data.insert("burning".into(), 50.0);
                if matches!(
                    entity.object.as_str(),
                    "oCaveman" | "oManTrap" | "oYeti" | "oHawkman" | "oShopkeeper" | "oVampire"
                ) {
                    entity.state = 98;
                    entity.timer = 100;
                }
            }
        } else if matches!(
            entity.object.as_str(),
            "oWeb" | "oBoulder" | "oBarrierEmitter" | "oArrow" | "oFishBone" | "oJar" | "oSkull"
        ) {
            entity.alive = false;
        } else if crate::catalog::is_a(&entity.object, "oItem")
            && entity.value("blast_impulse_id") != explosion.id as f64
        {
            entity
                .data
                .insert("blast_impulse_id".into(), explosion.id as f64);
            if entity.object != "oRopeThrow" || entity.value("falling") == 0.0 {
                entity.vel.y += if entity.pos.y < explosion.pos.y {
                    -6.0
                } else {
                    6.0
                };
                let speed = 4.0 + f64::from(game.rng.range(3));
                entity.vel.x += if entity.pos.x < explosion.pos.x {
                    -speed
                } else {
                    speed
                };
            }
        }
        if entity.held && entity.object != "oExplosion" {
            entity.held = false;
            if game.player.held == Some(entity.id) {
                game.player.held = None;
            }
        }
    }
    for (pos, velocity_y) in magma_spawns {
        let id = game.world.spawn("oMagma", pos);
        game.world.entity_mut(id).unwrap().vel.y = velocity_y;
    }
}

fn destroy_terrain(game: &mut Game, x: usize, y: usize) {
    let center = Vec2::new(x as f64 * 16.0 + 8.0, y as f64 * 16.0 + 8.0);
    let metadata = game
        .world
        .entities
        .iter()
        .find(|entity| entity.alive && entity.object == "oTerrainMetadata" && entity.pos == center)
        .cloned();
    if metadata
        .as_ref()
        .is_some_and(|metadata| metadata.value("invincible") != 0.0)
    {
        return;
    }
    game.world.set_tile(x, y, "");
    if let Some(metadata) = metadata {
        game.world.entity_mut(metadata.id).unwrap().alive = false;
        if metadata.value("shopWall") != 0.0 {
            game.flags.insert("shopkeepers_angry".into());
        }
        for (key, object) in [
            ("gold_chunks", "oGoldChunk"),
            ("gold_nuggets", "oGoldNugget"),
        ] {
            for _ in 0..metadata.value(key).max(0.0) as usize {
                let pos = Vec2::new(
                    center.x + f64::from(game.rng.range(5)) - f64::from(game.rng.range(5)),
                    center.y + f64::from(game.rng.range(5)) - f64::from(game.rng.range(5)),
                );
                let velocity = Vec2::new(
                    f64::from(game.rng.range(4)) - f64::from(game.rng.range(4)),
                    2.0 + f64::from(game.rng.range(3)),
                );
                let id = game.world.spawn(object, pos);
                game.world.entity_mut(id).unwrap().vel = velocity;
            }
        }
        if metadata.value("embedded_treasure") != 0.0 && metadata.value("frozen_caveman") == 0.0 {
            if let Some(treasure) = metadata.text.get("treasure") {
                let object = match treasure.as_str() {
                    "Big Ruby" => "oRubyBig",
                    "Big Sapphire" => "oSapphireBig",
                    "Big Emerald" => "oEmeraldBig",
                    "Ruby" => "oRuby",
                    "Sapphire" => "oSapphire",
                    "Emerald" => "oEmerald",
                    "Diamond" => "oDiamond",
                    name => name,
                };
                let dx = metadata.data.get("treasure_dx").copied().unwrap_or(0.0);
                let dy = metadata.data.get("treasure_dy").copied().unwrap_or(0.0);
                let id = game
                    .world
                    .spawn(object, Vec2::new(center.x + dx, center.y + dy));
                game.world.entity_mut(id).unwrap().state = 1;
            }
        }
        if metadata.value("frozen_caveman") != 0.0 {
            let id = game
                .world
                .spawn("oCaveman", Vec2::new(center.x - 8.0, center.y - 8.0));
            crate::enemies::init_entity(game.world.entity_mut(id).unwrap());
            let caveman = game.world.entity_mut(id).unwrap();
            caveman.state = 98;
            caveman.timer = 200;
            caveman.data.insert("invincible".into(), 20.0);
        }
    }
    // Source solids destroy fixtures resting on their upper face.
    let point = Vec2::new(center.x, center.y - 9.0);
    for entity in &mut game.world.entities {
        if entity.alive
            && matches!(entity.object.as_str(), "oSpikes" | "oTikiTorch" | "oGrave")
            && entity.bounds().contains(point)
        {
            entity.alive = false;
        }
    }
}

/// The source uses per-frame precise alpha collision masks with origin (24,24).
/// Rows below are derived from the official sExplosion images, alpha > 0.
fn mask_intersects(pos: Vec2, frame: usize, rect: Rect) -> bool {
    let left = pos.x.round() - 24.0;
    let top = pos.y.round() - 24.0;
    let x0 = ((rect.x - left).floor() as i32).clamp(0, 48);
    let y0 = ((rect.y - top).floor() as i32).clamp(0, 48);
    let x1 = ((rect.x + rect.w - left).ceil() as i32).clamp(0, 48);
    let y1 = ((rect.y + rect.h - top).ceil() as i32).clamp(0, 48);
    if x1 <= x0 || y1 <= y0 {
        return false;
    }
    let columns = ((1u64 << x1) - 1) & !((1u64 << x0) - 1);
    EXPLOSION_MASKS[frame][y0 as usize..y1 as usize]
        .iter()
        .any(|row| row & columns != 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn game() -> Game {
        let mut g = Game::shell(7, RulesPreset::Classic);
        g.player.pos = Vec2::new(160.0, 160.0);
        g
    }

    #[test]
    fn bomb_throw_source_velocity_and_inventory() {
        let mut g = game();
        g.player.vel.x = 2.0;
        use_bomb(
            &mut g,
            &InputFrame {
                bomb_pressed: true,
                up: true,
                ..Default::default()
            },
        );
        assert_eq!(g.player.bombs, 3);
        assert_eq!(g.world.entities[0].vel, Vec2::new(10.0, -9.0));
        assert_eq!(g.world.entities[0].timer, 120);
        g.player.bombs = 0;
        use_bomb(
            &mut g,
            &InputFrame {
                bomb_pressed: true,
                ..Default::default()
            },
        );
        assert_eq!(g.world.entities.len(), 1);
    }

    #[test]
    fn fuse_takes_120_ticks_and_unarmed_bombs_do_not_explode() {
        let mut g = game();
        use_bomb(
            &mut g,
            &InputFrame {
                bomb_pressed: true,
                down: true,
                ..Default::default()
            },
        );
        for _ in 0..119 {
            tick(&mut g);
        }
        assert!(
            !g.events
                .iter()
                .any(|event| matches!(event, GameEvent::Explosion { .. }))
        );
        tick(&mut g);
        assert_eq!(
            g.events
                .iter()
                .filter(|event| matches!(event, GameEvent::Explosion { .. }))
                .count(),
            1
        );
        let mut g = game();
        g.world.spawn("oBomb", Vec2::new(80.0, 80.0));
        for _ in 0..150 {
            tick(&mut g);
        }
        assert!(g.world.entities[0].alive);
    }

    #[test]
    fn explosion_destroys_terrain_but_preserves_boundary() {
        let mut g = game();
        g.world.set_tile(5, 5, "oBlock");
        g.world.set_tile(6, 5, "oSolid");
        explode(&mut g, Vec2::new(88.0, 88.0));
        for _ in 0..13 {
            tick(&mut g);
        }
        assert_eq!(g.world.tile(5, 5).unwrap().object, "");
        assert_eq!(g.world.tile(6, 5).unwrap().object, "oSolid");
    }

    #[test]
    fn chain_reaction_arms_an_unarmed_bomb() {
        let mut g = game();
        let bomb = g.world.spawn("oBomb", Vec2::new(88.0, 88.0));
        init_entity(g.world.entity_mut(bomb).unwrap());
        explode(&mut g, Vec2::new(88.0, 88.0));
        let bomb = g.world.entity(bomb).unwrap();
        assert_eq!(bomb.value("armed"), 1.0);
        assert!((4..=8).contains(&bomb.timer));
    }

    #[test]
    fn downward_rope_extends_128_pixels_and_stops_at_terrain() {
        let mut g = game();
        use_rope(
            &mut g,
            &InputFrame {
                rope_pressed: true,
                down: true,
                ..Default::default()
            },
        );
        for _ in 0..17 {
            tick(&mut g);
        }
        assert_eq!(g.player.ropes, 3);
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|entity| entity.object == "oRope" && entity.alive)
                .count(),
            16
        );
        assert!(
            g.world
                .entities
                .iter()
                .any(|entity| entity.object == "oRopeTop")
        );
        assert!(g.world.tiles.iter().any(Tile::is_ladder));
    }

    #[test]
    fn blocked_rope_placement_does_not_spend_inventory() {
        let mut g = game();
        g.world.set_tile(10, 10, "oBlock");
        use_rope(
            &mut g,
            &InputFrame {
                rope_pressed: true,
                down: true,
                ..Default::default()
            },
        );
        assert_eq!(g.player.ropes, 4);
        assert!(g.world.entities.is_empty());
    }

    #[test]
    fn paste_sticks_to_terrain_and_releases_when_support_is_destroyed() {
        let mut g = game();
        g.player.pos = Vec2::new(88.0, 88.0);
        g.player.equipment.insert("Paste".into());
        g.world.set_tile(6, 5, "oBlock");
        use_bomb(
            &mut g,
            &InputFrame {
                bomb_pressed: true,
                ..Default::default()
            },
        );
        tick(&mut g);
        assert_eq!(g.world.entities[0].value("stuck"), 1.0);
        assert_eq!(g.world.entities[0].vel, Vec2::default());
        g.world.set_tile(6, 5, "");
        tick(&mut g);
        assert_eq!(g.world.entities[0].value("stuck"), 0.0);
        assert!(g.world.entities[0].vel.y > 0.0);
    }

    #[test]
    fn precise_explosion_mask_changes_with_animation_frame() {
        let pos = Vec2::new(24.0, 24.0);
        assert!(!mask_intersects(pos, 0, Rect::new(0.0, 0.0, 8.0, 48.0)));
        assert!(mask_intersects(pos, 3, Rect::new(0.0, 0.0, 8.0, 48.0)));
        assert!(!mask_intersects(pos, 3, Rect::new(48.0, 0.0, 8.0, 48.0)));
    }

    #[test]
    fn arming_carried_bomb_preserves_running_fuse() {
        let mut bomb = Entity::new(1, "oBomb", Vec2::default());
        arm_bomb(&mut bomb);
        assert_eq!(bomb.timer, BOMB_FUSE);
        bomb.timer = 27;
        arm_bomb(&mut bomb);
        assert_eq!(bomb.timer, 27);
    }

    #[test]
    fn inventory_cycle_refunds_reservations_and_restores_original_item() {
        let mut g = game();
        let original = g.world.spawn("oRock", g.player.pos);
        g.world.entity_mut(original).unwrap().held = true;
        g.player.held = Some(original);
        cycle_inventory(&mut g);
        assert_eq!(g.player.bombs, 3);
        assert_eq!(
            g.world.entity(g.player.held.unwrap()).unwrap().object,
            "oBomb"
        );
        assert_eq!(g.world.entity(original).unwrap().value("stashed"), 1.0);
        cycle_inventory(&mut g);
        assert_eq!((g.player.bombs, g.player.ropes), (4, 3));
        assert_eq!(
            g.world.entity(g.player.held.unwrap()).unwrap().object,
            "oRopeThrow"
        );
        cycle_inventory(&mut g);
        assert_eq!((g.player.bombs, g.player.ropes), (4, 4));
        assert_eq!(g.player.held, Some(original));
        assert_eq!(g.world.entity(original).unwrap().pos, g.player.pos);
    }

    #[test]
    fn using_reserved_rope_does_not_spend_a_second_rope() {
        let mut g = game();
        g.player.bombs = 0;
        cycle_inventory(&mut g);
        let held = g.player.held.unwrap();
        assert_eq!(g.player.ropes, 3);
        assert!(use_held_rope(&mut g, held, &InputFrame::default()));
        assert_eq!(g.player.ropes, 3);
        assert!(g.player.held.is_none());
        assert!(
            g.world
                .entities
                .iter()
                .any(|rope| rope.alive && rope.object == "oRopeThrow" && !rope.held)
        );
    }

    #[test]
    fn terrain_metadata_preserves_invincibility_and_releases_treasure_once() {
        let mut g = game();
        g.world.set_tile(5, 5, "oBlock");
        let id = g.world.spawn("oTerrainMetadata", Vec2::new(88.0, 88.0));
        let metadata = g.world.entity_mut(id).unwrap();
        metadata.data.insert("invincible".into(), 1.0);
        metadata.data.insert("gold_chunks".into(), 3.0);
        metadata.data.insert("embedded_treasure".into(), 1.0);
        metadata.data.insert("shopWall".into(), 1.0);
        metadata.text.insert("treasure".into(), "Big Ruby".into());
        destroy_terrain(&mut g, 5, 5);
        assert_eq!(g.world.tile(5, 5).unwrap().object, "oBlock");
        g.world
            .entity_mut(id)
            .unwrap()
            .data
            .insert("invincible".into(), 0.0);
        destroy_terrain(&mut g, 5, 5);
        destroy_terrain(&mut g, 5, 5);
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| e.object == "oGoldChunk")
                .count(),
            3
        );
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| e.object == "oRubyBig")
                .count(),
            1
        );
        assert!(g.flags.contains("shopkeepers_angry"));
    }

    #[test]
    fn bomb_bounces_off_dynamic_push_block_without_tunneling() {
        let mut g = game();
        let block = g.world.spawn("oPushBlock", Vec2::new(96.0, 80.0));
        g.world.entity_mut(block).unwrap().solid = true;
        let mut bomb = Entity::new(999, "oBomb", Vec2::new(88.0, 88.0));
        bomb.vel = Vec2::new(12.0, 0.0);
        projectile_motion(&g.world, &mut bomb, false);
        assert!(bomb.pos.x <= 92.0);
        assert!(bomb.vel.x < 0.0);
    }

    #[test]
    fn embedded_treasure_offsets_are_relative_to_tile_center() {
        let mut g = game();
        g.world.set_tile(5, 5, "oBlock");
        let id = g.world.spawn("oTerrainMetadata", Vec2::new(88.0, 88.0));
        let metadata = g.world.entity_mut(id).unwrap();
        metadata.data.insert("embedded_treasure".into(), 1.0);
        metadata.data.insert("treasure_dy".into(), -2.0);
        metadata.text.insert("treasure".into(), "oJetpack".into());
        destroy_terrain(&mut g, 5, 5);
        assert!(
            g.world
                .entities
                .iter()
                .any(|entity| entity.object == "oJetpack" && entity.pos == Vec2::new(88.0, 86.0))
        );
    }
}

#[cfg(test)]
mod frozen_caveman_regression {
    use super::*;

    #[test]
    fn frozen_metadata_releases_exactly_one_actor_with_200_tick_stun() {
        let mut game = Game::shell(7, RulesPreset::Classic);
        game.world.set_tile(5, 5, "oIce");
        let id = game.world.spawn("oTerrainMetadata", Vec2::new(88.0, 88.0));
        let metadata = game.world.entity_mut(id).unwrap();
        metadata.data.insert("embedded_treasure".into(), 1.0);
        metadata.data.insert("frozen_caveman".into(), 1.0);
        metadata.text.insert("treasure".into(), "oCaveman".into());
        destroy_terrain(&mut game, 5, 5);
        let actors: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|entity| entity.alive && entity.object == "oCaveman")
            .collect();
        assert_eq!(actors.len(), 1);
        assert_eq!(actors[0].timer, 200);
        assert_eq!(actors[0].state, 98);
    }
}

const EXPLOSION_MASKS: [[u64; 48]; 10] = [
    [
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000007f00000,
        0x00001ffc0000,
        0x00007fff0000,
        0x0000ffff8000,
        0x0001ffffc000,
        0x0003ffffe000,
        0x0007fffff000,
        0x0007fffff000,
        0x0007fffff000,
        0x000ffffff800,
        0x000ffffff800,
        0x000ffffff800,
        0x000ffffff800,
        0x000ffffff800,
        0x000ffffff800,
        0x0007fffff000,
        0x0007fffff000,
        0x0007fffff000,
        0x0003ffffe000,
        0x0001ffffc000,
        0x0000ffff8000,
        0x00007fff0000,
        0x00001ffc0000,
        0x000007f00000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
    ],
    [
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000007e00000,
        0x00007ffe0000,
        0x0001ffff8000,
        0x0003ffffc000,
        0x000ffffff000,
        0x001ffffff800,
        0x003ffffffc00,
        0x007ffffffe00,
        0x007ffffffe00,
        0x00ffffffff00,
        0x01ffffffff80,
        0x01ffffffff80,
        0x03ffffffffc0,
        0x03ffffffffc0,
        0x03ffffffffc0,
        0x03ffffffffc0,
        0x07ffffffffe0,
        0x07ffffffffe0,
        0x07ffffffffe0,
        0x07ffffffffe0,
        0x07ffffffffe0,
        0x07ffffffffe0,
        0x03ffffffffc0,
        0x03ffffffffc0,
        0x03ffffffffc0,
        0x03ffffffffc0,
        0x01ffffffff80,
        0x01ffffffff80,
        0x00ffffffff00,
        0x007ffffffe00,
        0x007ffffffe00,
        0x003ffffffc00,
        0x001ffffff800,
        0x000ffffff000,
        0x0003ffffc000,
        0x0001ffff8000,
        0x00007ffe0000,
        0x000007e00000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
    ],
    [
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000001000000,
        0x000001000000,
        0x000001100000,
        0x000001100000,
        0x000043100000,
        0x000043900000,
        0x000143b00000,
        0x000143b80000,
        0x001043f82400,
        0x001ce7f86400,
        0x000fe7f8f000,
        0x000fe7fdf040,
        0x010ffffff0c0,
        0x018ffffff180,
        0x01cffffff380,
        0x0cfffffff700,
        0x0fffffffff00,
        0x07ffffffff20,
        0x07ffffffff60,
        0x03ffffffff40,
        0x03fffffffe00,
        0x01fffffffe00,
        0x01fffffffe00,
        0x01fffffffe00,
        0x00fffffffe00,
        0x00fffffffe00,
        0x03ffffffff00,
        0x07ffffffff80,
        0x07ffffffff80,
        0x07ffffffff80,
        0x07ffffffff80,
        0x03ffffffff00,
        0x00fffffffc00,
        0x003ffffff000,
        0x003ffffff000,
        0x001ffffff000,
        0x001fffffe000,
        0x000fffffe000,
        0x0007ffffc000,
        0x0003ffff8000,
        0x0000fffe0000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
    ],
    [
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x00003fe07000,
        0x0000fff8f800,
        0x0063fffc1c00,
        0x0067ffffc400,
        0x000ffffff000,
        0x003ffffffc00,
        0x00fffffffe00,
        0x01ffffffff00,
        0x01ffffffff00,
        0x03ffffffff80,
        0x07ffffffff80,
        0x0fffffffffc0,
        0x0fffffffffe0,
        0x1ffffffffff8,
        0x3ffffffffffc,
        0x7ffffffffffe,
        0xffffffffffff,
        0xffffffffffff,
        0xffffffffffff,
        0xffffffffffff,
        0xffffffffffff,
        0xffffffffffff,
        0xffffffffffff,
        0xffffffffffff,
        0x7fffffffffff,
        0x7ffffffffffe,
        0x3ffffffffffc,
        0x1ffffffffff8,
        0x1ffffffffff8,
        0x0ffffffffff0,
        0x07fffffffff0,
        0x03ffffffffe0,
        0x03ffffffffe0,
        0x03ffffffffc0,
        0x03ffffffff80,
        0x01ffffffff00,
        0x00fffffffce0,
        0x007ffffff1e0,
        0x033fffffc1e0,
        0x039fffff80c0,
        0x010efffe0000,
        0x00003ffc0000,
        0x00000fe00000,
        0x000000000000,
        0x000000000000,
    ],
    [
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000033e00000,
        0x0000fff80000,
        0x0001fffe0000,
        0x0003ffff0000,
        0x0007ffff8000,
        0x000fffffc000,
        0x000fffffd800,
        0x003ffffffc00,
        0x003ffffffc00,
        0x007ffffffc00,
        0x007ffffffc00,
        0x00fffffffc00,
        0x00ffffffff00,
        0x00ffffffff80,
        0x007fffffff80,
        0x00ffffffff80,
        0x01ffffffff00,
        0x01fffffff800,
        0x01fffffffc00,
        0x01fffffffc00,
        0x007ffffff800,
        0x001ffffff000,
        0x001fffffc000,
        0x000fffffc000,
        0x0003ffff8000,
        0x0000fffe0000,
        0x00007ffc0000,
        0x00003ff80000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
    ],
    [
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x00003ff80000,
        0x00007ffe0000,
        0x0001ffff8000,
        0x0003ffffc000,
        0x0007ffffe000,
        0x000fffffe000,
        0x001ffffff800,
        0x003ffffffc00,
        0x00fffffffe00,
        0x01ffffffff00,
        0x03ffffffff80,
        0x03ffffffffc0,
        0x03ffffffffc0,
        0x07ffffffffc0,
        0x07ffffffffe0,
        0x07ffffffffe0,
        0x07ffffffffe0,
        0x07ffffffffe0,
        0x07ffffffffe0,
        0x03ffffffffe0,
        0x03ffffffffe0,
        0x01ffffffffc0,
        0x03ffffffffc0,
        0x03ffffffffc0,
        0x03ffffffffc0,
        0x01ffffffffc0,
        0x00ffffffff80,
        0x007ffffffc00,
        0x027ffffffe00,
        0x003ffffffe00,
        0x001ffffffe00,
        0x00cffffffe00,
        0x00c7fffffc00,
        0x0003ffff8000,
        0x0001ffff6000,
        0x0000fffef000,
        0x00006ff8f000,
        0x000007f06000,
        0x000003e00000,
        0x000001c00000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
    ],
    [
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x00003ff00000,
        0x0000fffe0000,
        0x0003ffff8000,
        0x0007ffffc000,
        0x000fffffe000,
        0x001fffffe000,
        0x001ffffff0c0,
        0x007ffffff0c0,
        0x01fffffff800,
        0x03fffffffe00,
        0x03ffffffff00,
        0x07ffffffff80,
        0x07ffffffff98,
        0x07fffffffffc,
        0x07fffffffffc,
        0x0ffffffffffc,
        0x0ffffffffffc,
        0x0ffffffffffc,
        0x0ffffffffffc,
        0x0ffffffffff8,
        0x0ffffffffff0,
        0x0ffffffffff0,
        0x0ffffffffff0,
        0x07ffffffffe0,
        0x07ffffffffe0,
        0x03ffffffffc0,
        0x03ffffffffc0,
        0x01ffffffff80,
        0x00ffffffff00,
        0x00fffffffc00,
        0x021ffffffe00,
        0x000ffffffe00,
        0x000ffffffe00,
        0x00c6fffffe00,
        0x00c67fff9c00,
        0x00031ffe0000,
        0x00010ff86000,
        0x00000fe0e000,
        0x000007c0c000,
        0x000002000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
    ],
    [
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x0183e7f80000,
        0x01cffffe0000,
        0x01dffffff0c0,
        0x019ffffff8c0,
        0x001ffffff800,
        0x03fffffffb00,
        0x07ffffffffc0,
        0x0fffffffffe0,
        0x1ffffffffff8,
        0x1ffffffffffc,
        0x1ffffffffffc,
        0x1ffffffffffc,
        0x1ffffffffffc,
        0x0ffffffffffc,
        0x1ffffffffff8,
        0x1ffffffffff0,
        0x1ffffffffff0,
        0x1ffffffffff0,
        0x0ffffffffff0,
        0x07ffffffffe0,
        0x07ffffffffe0,
        0x03ffffffffc0,
        0x03efffffffc0,
        0x01efffffff80,
        0x00e7ffffff00,
        0x00f7fffffc00,
        0x0003fffffe00,
        0x020dfffffe00,
        0x000c7ffffe00,
        0x00040ffffc00,
        0x000007ff8000,
        0x000003f84000,
        0x000000c00000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
    ],
    [
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000080000,
        0x0000441c0000,
        0x0000060e0000,
        0x0003e00e0000,
        0x000fcff40000,
        0x001ffff98000,
        0x001fffffc000,
        0x003fffffe400,
        0x003fffffe000,
        0x007ffffff000,
        0x007ffffff000,
        0x00fffffff800,
        0x00fffffffc00,
        0x00fffffffe00,
        0x00ffffffff00,
        0x00ffffffff00,
        0x00ffffffff00,
        0x00fffffffe00,
        0x007ffffffe00,
        0x007ffffffe00,
        0x007ffffffc00,
        0x007ffffff000,
        0x003ffffff000,
        0x001fffffe000,
        0x001fffffe000,
        0x0007ffffc000,
        0x0003ffff8000,
        0x0001ffff0000,
        0x00007ffe0000,
        0x00003ffc0000,
        0x00001ff80000,
        0x00004e000000,
        0x000000900000,
        0x000001800000,
        0x000000800000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
    ],
    [
        0x000000000000,
        0x000000000000,
        0x000000c00000,
        0x000001c00000,
        0x000021f00000,
        0x000031f80000,
        0x000033fc0000,
        0x00004bfd0000,
        0x00005ffe0000,
        0x00001ffe0000,
        0x0003fffe0000,
        0x000fdffc0000,
        0x001fffff8000,
        0x001fffff8000,
        0x003fffffc000,
        0x003fffffe000,
        0x003fffffc000,
        0x001fffffc000,
        0x001fffffc000,
        0x001fffffe000,
        0x001ffffff000,
        0x000ffffff000,
        0x000ffffff000,
        0x0007fffff000,
        0x0003fffff000,
        0x0001fffff000,
        0x0001ffffe000,
        0x0000ffff8000,
        0x0000fffc0000,
        0x00007ff80000,
        0x00000ff00000,
        0x00000fe00000,
        0x000007800000,
        0x000003000000,
        0x000003000000,
        0x000002400000,
        0x000008400000,
        0x000001400000,
        0x000001000000,
        0x000000000000,
        0x000002000000,
        0x000001000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
        0x000000000000,
    ],
];
