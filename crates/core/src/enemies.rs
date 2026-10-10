//! Source-derived enemy state machines. Sprite collision remains an approximation:
//! actors use rectangular bounds rather than the GM sprite masks.
use crate::model::*;

mod caves;
mod ice;
mod jungle;
mod npcs;
mod temple;
mod undead;

pub fn is_enemy(object: &str) -> bool {
    caves::matches(object)
        || ice::matches(object)
        || jungle::matches(object)
        || temple::matches(object)
        || undead::matches(object)
        || matches!(
            object,
            "oGhost"
                | "oShopkeeper"
                | "oShopkeeper2"
                | "oDamsel"
                | "oScarab"
                | "oDeadFish"
                | "oBarrier"
                | "oBarrierEmitter"
        )
}

/// Initialize once, including enemies spawned during other systems' updates.
pub fn init_entity(e: &mut Entity) {
    if e.value("enemy_initialized") != 0.0 || !is_enemy(&e.object) {
        return;
    }
    e.data.insert("enemy_initialized".into(), 1.0);
    e.data.insert("enemy".into(), 1.0);
    e.data.insert("stun_time".into(), 200.0);
    if caves::matches(&e.object) {
        caves::init(e);
    } else if jungle::matches(&e.object) {
        jungle::init(e);
    } else if ice::matches(&e.object) {
        ice::init(e);
    } else if temple::matches(&e.object) {
        temple::init(e);
    } else if undead::matches(&e.object) {
        undead::init(e);
    } else {
        npcs::init(e);
    }
    let collision = match e.object.as_str() {
        "oSnake" | "oCaveman" | "oManTrap" | "oShopkeeper" | "oShopkeeper2" | "oYeti"
        | "oHawkman" | "oHawkmanWorship" | "oSkeleton" | "oVampire" | "oMagmaMan" => {
            Some([2.0, 0.0, 14.0, 16.0])
        }
        "oBat" => Some([2.0, 2.0, 14.0, 14.0]),
        "oSpider" => Some([1.0, 5.0, 15.0, 16.0]),
        "oGiantSpider" => Some([2.0, 16.0, 30.0, 32.0]),
        "oFrog" | "oFireFrog" | "oZombie" => Some([4.0, 4.0, 12.0, 16.0]),
        "oMonkey" => Some([4.0, 6.0, 12.0, 16.0]),
        "oPiranha" | "oDeadFish" => Some([0.0, 0.0, 8.0, 8.0]),
        "oJaws" => Some([0.0, 0.0, 48.0, 32.0]),
        "oDamsel" => Some([-4.0, -4.0, 4.0, 8.0]),
        "oGhost" => Some([4.0, 0.0, 12.0, 16.0]),
        "oScarab" => Some([4.0, 4.0, 12.0, 12.0]),
        "oYetiKing" | "oTombLord" => Some([6.0, 0.0, 26.0, 32.0]),
        "oAlien" => Some([2.0, 6.0, 14.0, 16.0]),
        "oUFO" => Some([4.0, 2.0, 12.0, 14.0]),
        "oAlienBoss" => Some([0.0, 0.0, 32.0, 32.0]),
        "oOlmec" => Some([2.0, 0.0, 62.0, 64.0]),
        "oMagma" => Some([-8.0, -8.0, 8.0, 8.0]),
        _ => None,
    };
    if let Some(values) = collision {
        for (key, value) in [
            "collision_left",
            "collision_top",
            "collision_right",
            "collision_bottom",
        ]
        .into_iter()
        .zip(values)
        {
            e.data.insert(key.into(), value);
        }
    }
}

pub fn tick(game: &mut Game) {
    let ids: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && is_enemy(&e.object))
        .map(|e| e.id)
        .collect();
    for id in ids {
        let Some(index) = game
            .world
            .entities
            .iter()
            .position(|e| e.id == id && e.alive)
        else {
            continue;
        };
        // Removing the current actor makes independent spawning deterministic and avoids aliasing.
        let mut e = game.world.entities.remove(index);
        init_entity(&mut e);
        e.data.insert("simulation_tick".into(), game.tick as f64);
        let began_dead = e.health <= 0;
        if e.held && e.state == 98 && e.health > 0 {
            e.timer = e.timer.saturating_sub(1);
            if e.timer == 0 {
                e.held = false;
                game.player.held = game.player.held.filter(|held| *held != e.id);
                e.state = if matches!(e.object.as_str(), "oShopkeeper" | "oShopkeeper2") {
                    2
                } else {
                    0
                };
            }
        }
        let active = e.object == "oGhost"
            || (e.pos.x > game.camera.x - 36.0
                && e.pos.x < game.camera.x + VIEW_WIDTH + 20.0
                && e.pos.y > game.camera.y - 36.0
                && e.pos.y < game.camera.y + VIEW_HEIGHT + 20.0);
        if e.value("webbed") > 0.0 && !matches!(e.object.as_str(), "oSpider" | "oGiantSpider") {
            e.data.insert("webbed".into(), e.value("webbed") - 1.0);
            game.world
                .entities
                .insert(index.min(game.world.entities.len()), e);
            continue;
        }
        if active && !e.held && e.value("inactive") == 0.0 {
            if e.value("hit_cooldown") > 0.0 {
                e.data
                    .insert("hit_cooldown".into(), e.value("hit_cooldown") - 1.0);
            }
            if caves::matches(&e.object)
                || jungle::matches(&e.object)
                || matches!(
                    e.object.as_str(),
                    "oDamsel" | "oShopkeeper" | "oShopkeeper2" | "oGhost" | "oScarab"
                )
            {
                e.animation += e.value("animation_speed");
            }
            if game.world.solid_at(center(&e).x, center(&e).y)
                && !matches!(
                    e.object.as_str(),
                    "oGhost"
                        | "oOlmec"
                        | "oMagma"
                        | "oMagmaMan"
                        | "oMagmaTrail"
                        | "oYellowBall"
                        | "oPsychicCreate"
                        | "oLaser"
                        | "oPsychicWave"
                        | "oCavemanWorship"
                        | "oBarrier"
                        | "oBarrierEmitter"
                        | "oSpiderHang"
                        | "oGiantSpiderHang"
                )
            {
                e.health = 0;
            }
            if caves::matches(&e.object) {
                caves::tick(game, &mut e);
            } else if jungle::matches(&e.object) {
                jungle::tick(game, &mut e);
            } else if ice::matches(&e.object) {
                ice::tick(game, &mut e);
            } else if temple::matches(&e.object) {
                temple::tick(game, &mut e);
            } else if undead::matches(&e.object) {
                undead::tick(game, &mut e);
            } else {
                npcs::tick(game, &mut e);
            }
            if e.alive && e.health > 0 {
                combat(game, &mut e);
            }
        }
        if e.health <= 0 && (began_dead || !e.alive) {
            finish_death(game, &mut e);
        }
        game.world
            .entities
            .insert(index.min(game.world.entities.len()), e);
    }
}

pub(super) fn bounds(e: &Entity) -> Rect {
    e.bounds()
}

pub(super) fn center(e: &Entity) -> Vec2 {
    let b = bounds(e);
    Vec2::new(b.x + b.w / 2.0, b.y + b.h / 2.0)
}

/// Classic moveTo quantizes fractional speeds by global time, and resolves X first.
/// Movable-solid pushing and one-way platforms are not represented here yet.
pub(super) fn move_body(world: &World, e: &mut Entity, gravity: f64) -> (bool, bool) {
    fn collides(world: &World, e: &Entity, r: Rect) -> bool {
        world.collides(r)
            || world.entities.iter().any(|other| {
                other.id != e.id
                    && other.alive
                    && (other.solid || crate::catalog::is_solid(&other.object))
                    && other.bounds().intersects(r)
            })
    }
    fn displacement(v: f64, time: u64) -> i32 {
        let f = v.abs().fract();
        let extra = f > 0.0 && time.is_multiple_of((1.0 / f).round().max(1.0) as u64);
        ((v.abs().floor() as i32) + i32::from(extra)) * if v < 0.0 { -1 } else { 1 }
    }
    let time = e.value("simulation_tick") as u64;
    let dx = displacement(e.vel.x, time);
    let dy = displacement(e.vel.y, time);
    let mut wall = false;
    for _ in 0..dx.unsigned_abs() {
        let old = e.pos.x;
        e.pos.x += f64::from(dx.signum());
        if collides(world, e, bounds(e)) {
            e.pos.x = old;
            wall = true;
            break;
        }
    }
    for _ in 0..dy.unsigned_abs() {
        let old = e.pos.y;
        e.pos.y += f64::from(dy.signum());
        if collides(world, e, bounds(e)) {
            e.pos.y = old;
            e.vel.y = 0.0;
            break;
        }
    }
    let mut feet = bounds(e);
    feet.y += 1.0;
    let grounded = collides(world, e, feet);
    e.vel.y = (e.vel.y + gravity).min(10.0);
    if grounded && dy >= 0 {
        e.vel.y = 0.0;
    }
    (wall, grounded)
}

pub(super) fn steer_toward(e: &mut Entity, target: Vec2, speed: f64) {
    let origin = center(e);
    let distance = origin.distance(target);
    if distance > 0.0 {
        e.vel = Vec2::new(
            (target.x - origin.x) / distance * speed,
            (target.y - origin.y) / distance * speed,
        );
    } else {
        e.vel = Vec2::default();
    }
    e.facing = if target.x < origin.x { -1 } else { 1 };
}

fn combat(game: &mut Game, e: &mut Entity) {
    if matches!(
        e.object.as_str(),
        "oGhost"
            | "oScarab"
            | "oOlmec"
            | "oOlmecSlam"
            | "oMagmaTrail"
            | "oYellowBall"
            | "oFly"
            | "oLaser"
            | "oPsychicWave"
            | "oPsychicCreate"
            | "oAlienEject"
            | "oUFOCrash"
            | "oOlmecDebris"
            | "oCavemanWorship"
            | "oBarrier"
            | "oBarrierEmitter"
    ) {
        return;
    }
    let body = bounds(e);
    let mut hits = Vec::new();
    let mut impact_explosions = Vec::new();
    for projectile in &mut game.world.entities {
        if !projectile.alive
            || projectile.held
            || projectile.id == e.id
            || !body.intersects(projectile.bounds())
        {
            continue;
        }
        let melee = matches!(
            projectile.object.as_str(),
            "oWhip" | "oWhipPre" | "oSlash" | "oMachetePre" | "oMattockPre" | "oMattockHit"
        );
        let fired = matches!(
            projectile.object.as_str(),
            "oBullet" | "oPsychicWaveP" | "oArrow"
        ) && (projectile.value("owner_player") > 0.0
            || projectile.value("owner") > 0.0);
        let thrown = matches!(
            projectile.object.as_str(),
            "oArrow" | "oRock" | "oSkull" | "oJar" | "oFishBone"
        ) && (projectile.vel.x.abs() > 2.0 || projectile.vel.y.abs() > 2.0);
        if !(melee || fired || thrown)
            || e.value("invincible") > 0.0
            || (projectile.object == "oPsychicWaveP" && e.object == "oAlienBoss")
            || (melee && e.value("no_whip") > 0.0)
            || (!fired && e.value("hit_cooldown") > 0.0)
            || (melee
                && body_enemy(&e.object)
                && e.state >= 98
                && projectile.value("puncture") == 0.0)
        {
            continue;
        }
        let key = format!("hit_{}", e.id);
        if projectile.value(&key) > 0.0 {
            continue;
        }
        projectile.data.insert(key, 1.0);
        let damage = if projectile.value("damage") > 0.0 {
            projectile.value("damage") as i32
        } else {
            1
        };
        hits.push((
            damage,
            projectile.pos.x,
            melee,
            projectile.object == "oBullet",
            projectile.vel,
        ));
        if projectile.object == "oArrow" && projectile.value("bomb_arrow") > 0.0 {
            impact_explosions.push(projectile.pos);
        }
        if projectile.object == "oJar" {
            projectile.data.insert("broken".into(), 1.0);
        } else if matches!(
            projectile.object.as_str(),
            "oBullet" | "oArrow" | "oSkull" | "oFishBone"
        ) {
            projectile.alive = false;
        }
    }
    for pos in impact_explosions {
        game.world.spawn("oExplosion", pos);
        game.sound("sndExplosion", pos);
    }
    for (damage, x, melee, bullet, velocity) in hits {
        e.health -= damage;
        e.data.insert("assaulted".into(), 1.0);
        if e.health <= 0 {
            e.data.insert("killed_by_player".into(), 1.0);
        }
        game.sound("sndHit", e.pos);
        if body_enemy(&e.object) && (e.state < 98 || !melee) {
            e.state = 98;
            e.timer = e.value("stun_time") as u32;
            e.vel.y = -3.0;
            e.vel.x = if x < e.pos.x { 2.0 } else { -2.0 };
        }
        if bullet {
            e.vel = if matches!(e.object.as_str(), "oYetiKing" | "oTombLord") {
                Vec2::new(velocity.x * 0.5, -2.0)
            } else {
                Vec2::new(velocity.x, -4.0)
            };
            if matches!(
                e.object.as_str(),
                "oCaveman" | "oYeti" | "oHawkman" | "oShopkeeper" | "oShopkeeper2"
            ) {
                e.state = 98;
                e.timer = 20;
            }
        }
        if e.object == "oGiantSpider" {
            e.data.insert("hit_cooldown".into(), 10.0);
        }
        if e.object == "oTombLord" {
            e.data.insert("hit_cooldown".into(), 20.0);
        }
        if matches!(e.object.as_str(), "oShopkeeper" | "oShopkeeper2") && !in_stars(game) {
            game.flags.insert("shopkeepers_angry".into());
        }
        if e.object == "oDamsel" {
            e.state = 2;
            e.timer = 120;
            e.vel.y = -3.0;
        }
        if e.object == "oUFO" && melee {
            game.world
                .spawn("oExplosion", Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0));
            if game.rng.range(3) == 0 {
                game.world
                    .spawn("oAlienEject", Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0));
            }
            game.sound("sndExplosion", e.pos);
            e.health = 0;
            e.alive = false;
        }
    }
    if e.health <= 0 || e.state >= 98 || e.object == "oDamsel" {
        return;
    }
    if !body.intersects(game.player.bounds()) {
        return;
    }
    let c = center(e);
    let falling = game.player.vel.y > 0.0 && game.player.pos.y < c.y - 2.0;
    let spiked = game.player.has("SpikeShoes")
        || game.player.has("spike_shoes")
        || game.player.has("Spike Shoes")
        || game.player.has("oSpikeShoes");
    if falling
        && e.value("no_stomp") == 0.0
        && !(e.object == "oMonkey" && e.state == 7)
        && (e.object != "oManTrap" || spiked || e.state == 97)
    {
        let fall = game.player.data.get("fall_timer").copied().unwrap_or(0.0);
        e.health -= ((fall / 16.0).floor() as i32 + 1) * if spiked { 3 } else { 1 };
        game.player.vel.y = -6.0 - 0.2 * game.player.vel.y;
        game.player.data.insert("fall_timer".into(), 0.0);
        if body_enemy(&e.object) {
            e.state = 98;
            e.timer = e.value("stun_time") as u32;
            e.vel.y = -6.0;
        }
        game.sound("sndHit", e.pos);
    } else if !matches!(
        e.object.as_str(),
        "oYeti" | "oManTrap" | "oMonkey" | "oHawkman" | "oShopkeeper" | "oShopkeeper2"
    ) && game.player.invincible == 0
    {
        let damage = if e.value("contact_damage") > 0.0 {
            e.value("contact_damage") as i32
        } else {
            1
        };
        game.damage_player(damage);
        game.player.invincible = 30;
        game.player.vel.x = if game.player.pos.x < e.pos.x {
            -6.0
        } else {
            6.0
        };
        if game.player.pos.y < e.pos.y {
            game.player.vel.y = -6.0;
        }
    }
}

fn body_enemy(object: &str) -> bool {
    matches!(
        object,
        "oCaveman"
            | "oManTrap"
            | "oVampire"
            | "oYeti"
            | "oHawkman"
            | "oShopkeeper"
            | "oShopkeeper2"
    )
}

fn finish_death(game: &mut Game, e: &mut Entity) {
    if e.object == "oVampire" && e.alive {
        return;
    }
    if matches!(
        e.object.as_str(),
        "oFly"
            | "oLaser"
            | "oPsychicWave"
            | "oPsychicCreate"
            | "oOlmecSlam"
            | "oOlmecDebris"
            | "oYellowBall"
            | "oMagmaTrail"
            | "oAlienEject"
            | "oUFOCrash"
    ) {
        e.alive = false;
        return;
    }
    if e.value("death_recorded") == 0.0 {
        e.data.insert("death_recorded".into(), 1.0);
        if matches!(e.object.as_str(), "oShopkeeper" | "oShopkeeper2") && in_stars(game) {
            let kills = game.data.entry("stars_kills".into()).or_default();
            *kills = (*kills + 1.0).min(99.0);
        } else {
            *game.data.entry("kills".into()).or_default() += 1.0;
        }
        if matches!(e.object.as_str(), "oGiantSpider" | "oGiantSpiderHang") {
            let drop = Vec2::new(e.pos.x + 16.0, e.pos.y + 24.0);
            game.world.spawn("oPaste", drop);
            let count = 1 + game.rng.range(3);
            for _ in 0..count {
                let gem = match game.rng.range(3) {
                    0 => "oEmeraldBig",
                    1 => "oSapphireBig",
                    _ => "oRubyBig",
                };
                game.world.spawn(gem, drop);
            }
        }
        if e.object == "oVampire" {
            game.world
                .spawn("oCapePickup", Vec2::new(e.pos.x + 8.0, e.pos.y + 8.0));
        }
        if matches!(e.object.as_str(), "oShopkeeper" | "oShopkeeper2") && !in_stars(game) {
            game.flags.insert("murderer".into());
            game.flags.insert("shopkeepers_angry".into());
        }
    }
    if e.object == "oAlienBoss" && e.alive {
        return;
    }
    if e.object == "oShopkeeper2" {
        return;
    }
    if body_enemy(&e.object) && e.object != "oVampire" || e.object == "oDamsel" {
        e.state = 99;
    } else {
        e.alive = false;
    }
}

pub(super) fn in_stars(game: &Game) -> bool {
    game.flags.contains("hub_stars") || game.world.name == "rStars"
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn classic_fraction_uses_global_tick() {
        let world = World::new(20, 20, Area::Mines, 1);
        let mut e = Entity::new(1, "oSnake", Vec2::new(50.0, 50.0));
        e.vel.x = 1.5;
        e.data.insert("simulation_tick".into(), 1.0);
        move_body(&world, &mut e, 0.0);
        assert_eq!(e.pos.x, 51.0);
        e.data.insert("simulation_tick".into(), 2.0);
        move_body(&world, &mut e, 0.0);
        assert_eq!(e.pos.x, 53.0);
    }
    #[test]
    fn held_enemy_does_not_move() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        let id = game.world.spawn("oSnake", Vec2::new(50.0, 50.0));
        game.world.entity_mut(id).unwrap().held = true;
        tick(&mut game);
        assert_eq!(game.world.entity(id).unwrap().pos, Vec2::new(50.0, 50.0));
    }
    #[test]
    fn whip_hits_each_actor_once() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.player.pos = Vec2::new(100.0, 100.0);
        let mut e = Entity::new(99, "oCaveman", Vec2::new(50.0, 50.0));
        init_entity(&mut e);
        game.world.spawn("oWhip", e.pos);
        combat(&mut game, &mut e);
        assert_eq!(e.health, 2);
        combat(&mut game, &mut e);
        assert_eq!(e.health, 2);
    }
    #[test]
    fn shotgun_pellets_damage_independently_in_one_tick() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.player.pos = Vec2::new(200.0, 200.0);
        let mut target = Entity::new(99, "oShopkeeper", Vec2::new(50.0, 50.0));
        init_entity(&mut target);
        for _ in 0..6 {
            let id = game.world.spawn("oBullet", Vec2::new(58.0, 58.0));
            let bullet = game.world.entity_mut(id).unwrap();
            bullet.data.insert("owner_player".into(), 1.0);
            bullet.data.insert("damage".into(), 4.0);
            bullet.vel.x = 8.0;
        }
        combat(&mut game, &mut target);
        assert_eq!(target.health, -4);
        assert_eq!(target.timer, 20);
        assert!(game.world.entities.iter().all(|e| !e.alive));
    }
    #[test]
    fn movement_collides_with_source_dynamic_solids() {
        let mut world = World::new(20, 20, Area::Mines, 1);
        world.spawn("oPushBlock", Vec2::new(64.0, 48.0));
        let mut e = Entity::new(99, "oSnake", Vec2::new(50.0, 48.0));
        init_entity(&mut e);
        e.vel.x = 2.0;
        assert!(move_body(&world, &mut e, 0.0).0);
        assert_eq!(e.pos.x, 50.0);
    }
    #[test]
    fn stars_shopkeeper_inherits_combat_without_campaign_statistics() {
        let mut game = Game::shell(1, RulesPreset::Classic);
        game.flags.insert("hub_stars".into());
        game.data.insert("stars_kills".into(), 98.0);
        game.player.pos = Vec2::new(200.0, 200.0);
        game.world.set_tile(4, 5, "oSolid");
        let id = game.world.spawn("oShopkeeper2", Vec2::new(64.0, 64.0));
        let keeper = game.world.entity_mut(id).unwrap();
        assert_eq!(keeper.object, "oShopkeeper2");
        assert_eq!(keeper.health, 20);
        assert_eq!(keeper.state, 2);
        assert_eq!(keeper.value("has_gun"), 0.0);
        assert_eq!(keeper.value("death_timer"), 200.0);
        keeper.health = 0;
        tick(&mut game);
        assert_eq!(game.data["stars_kills"], 99.0);
        assert_eq!(game.data.get("kills"), None);
        assert!(!game.flags.contains("murderer"));
        assert!(!game.flags.contains("shopkeepers_angry"));
        tick(&mut game);
        assert_eq!(game.data["stars_kills"], 99.0);
        assert!(
            !game
                .world
                .entities
                .iter()
                .any(|e| matches!(e.object.as_str(), "oShotgun" | "oGoldNugget"))
        );
    }
}
