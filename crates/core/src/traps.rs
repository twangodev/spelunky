//! Classic hazards translated from object event code. Static hazard positions are
//! GameMaker instance origins (usually tile top-left), not actor centers.
use crate::model::{Entity, Game, GameEvent, PlayerState, Rect, Vec2};

#[path = "traps/altar.rs"]
mod altar;
#[path = "traps/curses.rs"]
mod curses;
#[path = "traps/moving.rs"]
mod moving;

/// Global platform movement runs before player input, as in oGame Step.
pub fn pre_tick(game: &mut Game) {
    moving::pre_tick(game);
}

pub fn tick(game: &mut Game) {
    environment(game);
    detectors(game);
    idol_traps(game);
    moving::tick(game);
    altar::tick(game);
    curses::tick(game);
}

#[derive(Clone)]
struct Hazard {
    id: Option<u64>,
    object: String,
    pos: Vec2,
    data_key: String,
}
fn hazards(game: &Game) -> Vec<Hazard> {
    let mut result = Vec::new();
    for (i, tile) in game.world.tiles.iter().enumerate() {
        if is_hazard(&tile.object) {
            result.push(Hazard {
                id: None,
                object: tile.object.clone(),
                pos: Vec2::new(
                    (i % game.world.width) as f64 * 16.0,
                    (i / game.world.width) as f64 * 16.0,
                ),
                data_key: format!("hazard:{i}"),
            });
        }
    }
    for e in game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && is_hazard(&e.object))
    {
        result.push(Hazard {
            id: Some(e.id),
            object: e.object.clone(),
            pos: e.pos,
            data_key: format!("hazard_entity:{}", e.id),
        });
    }
    result
}
fn is_hazard(object: &str) -> bool {
    matches!(
        object,
        "oSpikes"
            | "oLava"
            | "oWater"
            | "oWaterSwim"
            | "oWeb"
            | "oArrowTrapLeft"
            | "oArrowTrapRight"
            | "oArrowTrapLeftLit"
            | "oArrowTrapRightLit"
            | "oArrowRepeaterL"
            | "oArrowRepeaterR"
            | "oSpearTrapTop"
            | "oSpearTrapBottom"
            | "oSpearTrapLit"
            | "oSpearsLeft"
    )
}
fn rect(h: &Hazard) -> Rect {
    crate::catalog::bounds(&h.object, h.pos)
}
fn current_rect(game: &Game, h: &Hazard) -> Rect {
    h.id.and_then(|id| game.world.entity(id))
        .map(Entity::sprite_bounds)
        .unwrap_or_else(|| rect(h))
}
fn lethal(game: &mut Game) {
    // Source environmental kills bypass the ordinary damage invulnerability.
    if game.player.health > 0 {
        game.player.health -= 99;
        if game.player.health <= 0 {
            game.player.state = PlayerState::Dead;
        }
        game.events.push(GameEvent::Damage { amount: 99 });
    }
}
fn environment(game: &mut Game) {
    let all = hazards(game);
    let pos = game.player.pos;
    let mut lava_applied = false;
    let mut spikes_applied = false;
    for h in &all {
        if !lava_applied
            && h.object == "oLava"
            && current_rect(game, h).contains(Vec2::new(pos.x, pos.y + 6.0))
        {
            lava_applied = true;
            let living = game.player.health > 0;
            lethal(game);
            game.player.vel = Vec2::new(0.0, 0.1);
            game.player.data.insert("burning".into(), 100.0);
            if living {
                game.sound("sndFlame", pos);
            }
        }
        let fall = game.player.data.get("fall_ticks").copied().unwrap_or(0.0);
        if !spikes_applied
            && h.object == "oSpikes"
            && Rect::new(pos.x - 4.0, pos.y - 4.0, 9.0, 13.0).intersects(current_rect(game, h))
            && game.player.vel.y > 0.0
            && (fall > 4.0 || game.player.stun > 0)
        {
            spikes_applied = true;
            if game.player.health > 0 {
                lethal(game);
                game.player.vel = Vec2::default();
                game.sound("sndHurt", pos);
            }
            if let Some(id) = h.id {
                if let Some(e) = game.world.entity_mut(id) {
                    e.sprite = "sSpikesBlood".into();
                }
            }
        }
    }
    // Web collision handlers freeze movable objects. Spider-family immunity and
    // magma melting are explicit exceptions in the Classic event source.
    let webs: Vec<_> = all.iter().filter(|h| h.object == "oWeb").cloned().collect();
    for web in &webs {
        let r = current_rect(game, web);
        let wet = all.iter().any(|h| {
            matches!(h.object.as_str(), "oWater" | "oWaterSwim" | "oLava")
                && current_rect(game, h).intersects(r)
        });
        let magma = game
            .world
            .entities
            .iter()
            .any(|e| e.alive && e.object == "oMagmaMan" && e.sprite_bounds().intersects(r));
        if wet || magma {
            remove_hazard(game, web);
            continue;
        }
        for e in &mut game.world.entities {
            let enemy = crate::catalog::is_a(&e.object, "oEnemy");
            let item = crate::catalog::is_a(&e.object, "oItem");
            let rubble = crate::catalog::is_a(&e.object, "oRubblePiece");
            if e.alive
                && (!item || !e.held)
                && e.sprite_bounds().intersects(r)
                && (enemy || item || rubble || crate::catalog::is_a(&e.object, "oTreasure"))
                && !matches!(
                    e.object.as_str(),
                    "oSpider" | "oGiantSpider" | "oRope" | "oRopeTop" | "oRopeSegment"
                )
            {
                e.vel = Vec2::default();
                if enemy && crate::catalog::is_a(&e.object, "oShopkeeper") && e.health > 0 {
                    e.state = 2;
                }
                if rubble {
                    e.data.insert("animation_speed".into(), 0.0);
                }
            }
        }
        if let Some(entity) = web.id.and_then(|id| game.world.entity_mut(id)) {
            // Both jumping and natural decay consume the same source life.
            let life = entity.data.get("life").copied().unwrap_or(12.0);
            entity.data.insert("image_alpha".into(), life / 12.0);
            let life = life
                - if entity.value("dying") != 0.0 {
                    0.02
                } else {
                    0.0
                };
            entity.data.insert("life".into(), life);
            if life <= 1.0 {
                entity.alive = false;
            }
        }
    }
}
fn remove_hazard(game: &mut Game, h: &Hazard) {
    if let Some(id) = h.id {
        if let Some(e) = game.world.entity_mut(id) {
            e.alive = false;
        }
        crate::items::explosives::destroy_solid_instance(game, id);
    } else {
        game.world
            .set_tile((h.pos.x / 16.0) as usize, (h.pos.y / 16.0) as usize, "");
    }
}
fn movable(e: &Entity) -> bool {
    [
        "oCharacter",
        "oEnemy",
        "oItem",
        "oTreasure",
        "oMoveableSolid",
    ]
    .iter()
    .any(|parent| crate::catalog::is_a(&e.object, parent))
}

fn detector(game: &Game, h: &Hazard, right: bool) -> Rect {
    let y = h.pos.y + 8.0;
    if right {
        let mut x = h.pos.x + 16.0;
        while x - h.pos.x <= 96.0 && !game.world.solid_at(x, y) {
            x += 1.0;
        }
        let length = (x - (h.pos.x + 8.0)).max(32.0);
        Rect::new(
            h.pos.x + 16.0,
            h.pos.y,
            ((length - 16.0) / 16.0).ceil() * 16.0,
            16.0,
        )
    } else {
        let mut x = h.pos.x - 1.0;
        while h.pos.x - x <= 96.0 && !game.world.solid_at(x, y) {
            x -= 1.0;
        }
        x = x.min(h.pos.x - 16.0);
        Rect::new(x, h.pos.y, ((h.pos.x - 1.0 - x) / 16.0).ceil() * 16.0, 16.0)
    }
}
fn detectors(game: &mut Game) {
    for h in hazards(game) {
        match h.object.as_str() {
            "oArrowTrapLeft" | "oArrowTrapRight" | "oArrowTrapLeftLit" | "oArrowTrapRightLit"
            | "oArrowRepeaterL" | "oArrowRepeaterR" => {
                let right = matches!(
                    h.object.as_str(),
                    "oArrowTrapRight" | "oArrowTrapRightLit" | "oArrowRepeaterR"
                );
                let repeat = h.object.starts_with("oArrowRepeater");
                let key = format!("{}:fired", h.data_key);
                let fired = game.data.get(&key).copied().unwrap_or(0.0);
                if repeat && !game.data.contains_key(&key) {
                    let delay = 100 + game.rng.range(100);
                    game.data.insert(key, f64::from(delay));
                    continue;
                }
                if fired > 0.0 {
                    if repeat {
                        game.data.insert(key, fired - 1.0);
                    }
                    continue;
                }
                let range_key = format!("{}:range", h.data_key);
                let area = if let Some(&x) = game.data.get(&range_key) {
                    Rect::new(
                        x,
                        h.pos.y,
                        game.data
                            .get(&format!("{}:range_width", h.data_key))
                            .copied()
                            .unwrap_or(16.0),
                        16.0,
                    )
                } else {
                    let area = detector(game, &h, right);
                    game.data.insert(range_key, area.x);
                    game.data
                        .insert(format!("{}:range_width", h.data_key), area.w);
                    area
                };
                let duck_to_hang = matches!(
                    game.player.sprite.as_str(),
                    "sDuckToHangL" | "sDamselDtHL" | "sTunnelDtHL"
                ) && game.player.animation > 6.0;
                let activated = repeat
                    || (game.player.vel.x != 0.0 || game.player.vel.y != 0.0 || duck_to_hang)
                        && area.intersects(game.player.sprite_bounds())
                    || game.world.entities.iter().any(|e| {
                        e.alive
                            && e.id != h.id.unwrap_or(0)
                            && (movable(e)
                                || matches!(e.object.as_str(), "oBoulder" | "oPushBlock"))
                            && (e.vel.x != 0.0 || e.vel.y != 0.0)
                            && area.intersects(e.sprite_bounds())
                    });
                if activated {
                    let pos = Vec2::new(h.pos.x + if right { 18.0 } else { -2.0 }, h.pos.y + 4.0);
                    let id = game.world.spawn("oArrow", pos);
                    if let Some(e) = game.world.entity_mut(id) {
                        e.vel.x = if right { 8.0 } else { -8.0 };
                        e.facing = if right { 1 } else { -1 };
                        e.data.insert("armed".into(), 1.0);
                        e.data.insert("owner".into(), -1.0);
                    }
                    let delay = if repeat {
                        100.0 + f64::from(game.rng.range(101))
                    } else {
                        1.0
                    };
                    game.data.insert(key, delay);
                    game.sound("sndArrowTrap", h.pos);
                }
            }
            "oSpearTrapTop" | "oSpearTrapBottom" | "oSpearTrapLit" => {
                let key = format!("{}:cooldown", h.data_key);
                let cooldown = game.data.get(&key).copied().unwrap_or(0.0);
                if cooldown > 1.0 {
                    game.data.insert(key.clone(), cooldown - 1.0);
                }
                if cooldown <= 1.0
                    && let Some(right) = spear_target(game, &h)
                {
                    let id = game.world.spawn(
                        "oSpearsLeft",
                        Vec2::new(h.pos.x + if right { 16.0 } else { -16.0 }, h.pos.y),
                    );
                    if let Some(e) = game.world.entity_mut(id) {
                        e.sprite = if right { "sSpearsRight" } else { "sSpearsLeft" }.into();
                        e.facing = if right { 1 } else { -1 };
                        e.animation = 0.0;
                    }
                    game.data.insert(key, 50.0);
                } else if cooldown <= 1.0 {
                    game.data.insert(key, 0.0);
                }
                let active = h.pos.x > game.camera.x - 8.0
                    && h.pos.x < game.camera.x + 328.0
                    && h.pos.y > game.camera.y - 8.0
                    && h.pos.y < game.camera.y + 248.0;
                if active && !game.world.solid_at(h.pos.x, h.pos.y + 16.0) {
                    remove_hazard(game, &h);
                }
            }
            "oSpearsLeft" => {
                if let Some(id) = h.id {
                    let right = game
                        .world
                        .entity(id)
                        .is_some_and(|e| e.sprite == "sSpearsRight");
                    let anchor = Vec2::new(h.pos.x + if right { -16.0 } else { 16.0 }, h.pos.y);
                    let supported = hazards(game).iter().any(|trap| {
                        matches!(
                            trap.object.as_str(),
                            "oSpearTrapTop" | "oSpearTrapBottom" | "oSpearTrapLit"
                        ) && current_rect(game, trap).contains(anchor)
                    });
                    if !supported {
                        remove_hazard(game, &h);
                        continue;
                    }
                    let overlaps = current_rect(game, &h).intersects(Rect::new(
                        game.player.pos.x - 6.0,
                        game.player.pos.y - 6.0,
                        13.0,
                        14.0,
                    ));
                    let damaging = game
                        .world
                        .entity(id)
                        .is_some_and(|e| e.animation >= 20.0 && e.animation < 24.0 && overlaps);
                    if damaging && game.player.health > 0 {
                        // oPlayer1 Step intentionally bypasses both ordinary
                        // damage immunity and stun. Every active frame can hit.
                        game.player.health -= 4;
                        game.events.push(GameEvent::Damage { amount: 4 });
                        let speed = 4.0 + f64::from(game.rng.range(3));
                        game.player.vel = Vec2::new(if right { speed } else { -speed }, -6.0);
                        game.player.pos.y -= 1.0;
                    }
                    if let Some(e) = game.world.entity_mut(id) {
                        e.animation += 1.0;
                        if e.animation >= 32.0 {
                            e.alive = false;
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

/// The source tests the nearest member of each family, first on the left,
/// then on the right. Actor and item origins use different vertical probes.
fn spear_target(game: &Game, trap: &Hazard) -> Option<bool> {
    let center = Vec2::new(trap.pos.x + 8.0, trap.pos.y + 8.0);
    let nearest = |family: &str| {
        game.world
            .entities
            .iter()
            .filter(|e| e.alive && crate::catalog::is_a(&e.object, family))
            .min_by(|a, b| {
                a.pos
                    .distance(trap.pos)
                    .total_cmp(&b.pos.distance(trap.pos))
            })
    };
    for right in [false, true] {
        let on_side = |x: f64, boundary: f64| if right { x > boundary } else { x < boundary };
        if (game.player.pos.y - center.y).abs() < 4.0
            && game.player.pos.distance(center) < 64.0
            && on_side(game.player.pos.x, if right { center.x } else { trap.pos.x })
        {
            return Some(right);
        }
        for family in ["oEnemy", "oMoveableSolid", "oItem"] {
            if let Some(entity) = nearest(family) {
                let origin = if family == "oItem" { center } else { trap.pos };
                let boundary = if family == "oItem" {
                    center.x
                } else {
                    trap.pos.x
                };
                if (entity.pos.y - origin.y).abs() < 4.0
                    && entity.pos.distance(origin) < 64.0
                    && on_side(entity.pos.x, boundary)
                {
                    return Some(right);
                }
            }
        }
    }
    None
}
fn idol_traps(game: &mut Game) {
    let ids: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && matches!(e.object.as_str(), "oGiantTikiHead" | "oTrapBlock"))
        .map(|e| e.id)
        .collect();
    for id in ids {
        let mut spawn = None;
        let mut destroyed = None;
        if let Some(e) = game.world.entity_mut(id) {
            if e.object == "oGiantTikiHead" {
                let alarm = e.value("idol_alarm");
                if alarm > 0.0 {
                    e.data.insert("idol_alarm".into(), alarm - 1.0);
                    if alarm <= 1.0 {
                        e.sprite = "sGTHHole".into();
                        spawn = Some(e.pos);
                    }
                }
            } else if e.value("dying") != 0.0 {
                let delay = e.value("death_timer");
                if delay > 0.0 {
                    e.data.insert("death_timer".into(), delay - 1.0);
                } else {
                    e.alive = false;
                    destroyed = Some(e.pos);
                }
            }
        }
        if let Some(pos) = spawn {
            game.world.spawn("oBoulder", pos);
            game.sound("sndThump", pos);
        }
        if let Some(pos) = destroyed {
            game.sound("sndThump", pos);
            game.data.insert("screen_shake".into(), 10.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::RulesPreset;
    fn room() -> Game {
        let mut g = Game::shell(1, RulesPreset::Classic);
        g.player.pos = Vec2::new(40.0, 40.0);
        g
    }
    #[test]
    fn arrow_detector_requires_motion_and_fires_once() {
        let mut g = room();
        g.world.spawn("oArrowTrapLeft", Vec2::new(80.0, 32.0));
        tick(&mut g);
        assert!(!g.world.entities.iter().any(|e| e.object == "oArrow"));
        g.player.vel.x = 1.0;
        tick(&mut g);
        let arrows: Vec<_> = g
            .world
            .entities
            .iter()
            .filter(|e| e.object == "oArrow")
            .collect();
        assert_eq!(arrows.len(), 1);
        assert_eq!(arrows[0].vel.x, -8.0);
        tick(&mut g);
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| e.object == "oArrow")
                .count(),
            1
        );
    }
    #[test]
    fn wall_blocks_arrow_motion_detector() {
        let mut g = room();
        g.world.set_tile(3, 2, "oBlock");
        g.world.spawn("oArrowTrapLeft", Vec2::new(80.0, 32.0));
        g.player.vel.x = 1.0;
        tick(&mut g);
        assert!(!g.world.entities.iter().any(|e| e.object == "oArrow"));
    }
    #[test]
    fn arrow_detector_ignores_moving_debris_but_accepts_items_and_moveable_solids() {
        // oArrowTrapTest declares collisions with Character, Enemy, Item,
        // Treasure, MoveableSolid, and Boulder. RubblePiece inherits only
        // DrawnSprite: its visible motion must not activate those callbacks.
        for (object, should_fire) in [
            ("oRubble", false),
            ("oRubbleSmall", false),
            ("oRock", true),
            ("oPushBlock", true),
        ] {
            let mut game = room();
            game.world = crate::World::new(20, 15, crate::Area::Mines, 1);
            game.player.pos = Vec2::new(40.0, 184.0);
            game.player.previous_pos = game.player.pos;
            game.player.state = PlayerState::Standing;
            game.player.grounded = true;
            for x in 0..20 {
                game.world.set_tile(x, 12, "oBlock");
            }
            game.world.spawn("oArrowTrapLeft", Vec2::new(80.0, 80.0));
            let moving = game.world.spawn(object, Vec2::new(50.0, 82.0));
            game.world.entity_mut(moving).unwrap().vel.x = 1.0;

            game.step(&crate::InputFrame::default());

            let entity = game.world.entity(moving).unwrap();
            assert_ne!(
                entity.pos,
                Vec2::new(50.0, 82.0),
                "{object} must actually move"
            );
            assert_eq!(
                game.world.entities.iter().any(|e| e.object == "oArrow"),
                should_fire,
                "source collision family: {object}",
            );
        }
    }
    #[test]
    fn spikes_require_long_descent_or_stun() {
        let mut g = room();
        g.world.spawn("oSpikes", Vec2::new(32.0, 32.0));
        g.player.vel.y = 1.0;
        g.player.data.insert("fall_ticks".into(), 4.0);
        tick(&mut g);
        assert_eq!(g.player.health, 4);
        g.player.data.insert("fall_ticks".into(), 5.0);
        tick(&mut g);
        assert!(g.player.health <= 0);
    }
    #[test]
    fn lava_bypasses_damage_immunity() {
        let mut g = room();
        g.world.spawn("oLava", Vec2::new(32.0, 32.0));
        g.player.invincible = 60;
        tick(&mut g);
        assert!(g.player.health <= 0);
    }
    #[test]
    fn lava_applies_the_source_ninety_nine_damage_once_per_character_step() {
        let mut game = room();
        game.world.spawn("oLava", Vec2::new(32.0, 32.0));
        game.world.spawn("oLava", Vec2::new(32.0, 32.0));
        // Kali can heal without a source life cap. A collision_point query
        // subtracts 99 once even when authored lava instances overlap.
        game.player.health = 200;
        game.player.invincible = 60;
        game.step(&crate::InputFrame::default());
        assert_eq!(game.player.health, 101);
        assert_eq!(game.state, crate::RunState::Playing);
    }
    #[test]
    fn web_freezes_rock_but_spares_spiders() {
        let mut g = room();
        g.world.spawn("oWeb", Vec2::new(32.0, 32.0));
        let rock = g.world.spawn("oRock", Vec2::new(40.0, 40.0));
        let spider = g.world.spawn("oSpider", Vec2::new(40.0, 40.0));
        g.world.entity_mut(rock).unwrap().vel.x = 2.0;
        g.world.entity_mut(spider).unwrap().vel.x = 2.0;
        tick(&mut g);
        assert_eq!(g.world.entity(rock).unwrap().vel.x, 0.0);
        assert_eq!(g.world.entity(spider).unwrap().vel.x, 2.0);
    }

    #[test]
    fn dying_web_shares_escape_life_and_fades_before_destroying() {
        let mut game = room();
        game.player.pos = Vec2::new(120.0, 120.0);
        let web = game.world.spawn("oWeb", Vec2::new(32.0, 32.0));
        let entity = game.world.entity_mut(web).unwrap();
        entity.data.insert("life".into(), 1.03);
        entity.data.insert("dying".into(), 1.0);
        game.step(&crate::InputFrame::default());
        let entity = game.world.entity(web).unwrap();
        assert!((entity.value("life") - 1.01).abs() < 0.00001);
        assert!((entity.value("image_alpha") - 1.03 / 12.0).abs() < 0.00001);
        game.step(&crate::InputFrame::default());
        assert!(game.world.entity(web).is_none());
    }

    #[test]
    fn web_does_not_freeze_inherited_stun_timers_or_weapon_collision_events() {
        let mut game = room();
        game.player.pos = Vec2::new(120.0, 40.0);
        game.world.set_tile(5, 6, "oHardBlock");
        game.world.spawn("oWeb", Vec2::new(80.0, 80.0));
        let id = game.world.spawn("oCaveman", Vec2::new(80.0, 80.0));
        let caveman = game.world.entity_mut(id).unwrap();
        crate::enemies::init_entity(caveman);
        caveman.state = 98;
        caveman.timer = 5;
        caveman.vel = Vec2::default();
        game.step(&crate::InputFrame::default());
        game.step(&crate::InputFrame::default());
        assert_eq!(game.world.entity(id).unwrap().timer, 3);
        assert_eq!(game.world.entity(id).unwrap().value("webbed"), 0.0);
        let bullet = game.world.spawn("oBullet", Vec2::new(87.0, 87.0));
        let bullet = game.world.entity_mut(bullet).unwrap();
        bullet.data.insert("owner".into(), 1.0);
        bullet.data.insert("damage".into(), 4.0);
        game.step(&crate::InputFrame::default());
        assert!(game.world.entity(id).is_none_or(|e| e.health <= 0));
    }

    #[test]
    fn living_shopkeeper_in_web_becomes_angry_without_global_theft() {
        let mut game = room();
        game.player.pos = Vec2::new(120.0, 40.0);
        game.world.set_tile(5, 6, "oHardBlock");
        game.world.spawn("oWeb", Vec2::new(80.0, 80.0));
        let id = game.world.spawn("oShopkeeper", Vec2::new(80.0, 80.0));
        game.step(&crate::InputFrame::default());
        let keeper = game.world.entity(id).unwrap();
        assert_eq!(keeper.state, 2);
        assert_eq!(keeper.vel, Vec2::default());
    }

    fn spear_room(right: bool, player_x: f64) -> (Game, u64) {
        let mut game = room();
        for x in 0..game.world.width {
            game.world.set_tile(x, 10, "oHardBlock");
        }
        let trap_x = if right { 64.0 } else { 96.0 };
        let trap = game.world.spawn("oSpearTrapTop", Vec2::new(trap_x, 144.0));
        game.data
            .insert(format!("hazard_entity:{trap}:cooldown"), 50.0);
        let spear = game.world.spawn("oSpearsLeft", Vec2::new(80.0, 144.0));
        let entity = game.world.entity_mut(spear).unwrap();
        entity.sprite = if right { "sSpearsRight" } else { "sSpearsLeft" }.into();
        entity.animation = 20.0;
        game.player.pos = Vec2::new(player_x, 152.0);
        game.player.state = PlayerState::Standing;
        game.player.grounded = true;
        game.player.health = 16;
        (game, spear)
    }

    #[test]
    fn active_spear_damage_bypasses_immunity_without_creating_stun() {
        let (mut game, spear) = spear_room(false, 90.0);
        game.player.invincible = 60;
        game.step(&crate::InputFrame::default());
        assert_eq!(game.player.health, 12);
        assert_eq!(game.player.stun, 0);
        assert_eq!(game.player.invincible, 59);
        assert_eq!(game.player.pos.y, 151.0);
        assert_eq!(game.player.vel.y, -6.0);
        assert!((-6.0..=-4.0).contains(&game.player.vel.x));
        assert_eq!(game.world.entity(spear).unwrap().animation, 21.0);
    }

    #[test]
    fn spear_damage_uses_current_directional_mask_and_active_frame() {
        // Frame 20's right mask occupies x80..88; the left mask occupies
        // x87..95. A player probe x93..105 therefore hits only the left one.
        let (mut right, _) = spear_room(true, 99.0);
        right.step(&crate::InputFrame::default());
        assert_eq!(right.player.health, 16);
        let (mut left, spear) = spear_room(false, 90.0);
        left.world.entity_mut(spear).unwrap().animation = 19.0;
        left.step(&crate::InputFrame::default());
        assert_eq!(left.player.health, 16);
        left.step(&crate::InputFrame::default());
        assert_eq!(left.player.health, 12);
    }

    #[test]
    fn spear_animation_finishes_during_the_death_reveal() {
        let (mut game, spear) = spear_room(false, 40.0);
        game.player.health = 0;
        game.player.state = PlayerState::Dead;
        game.state = crate::RunState::Dead;
        for _ in 0..35 {
            game.step(&crate::InputFrame::default());
        }
        assert_eq!(game.state, crate::RunState::Dead);
        assert!(game.world.entity(spear).is_none());
        assert!(game.data.get("death_ticks").copied().unwrap_or(0.0) >= 35.0);
    }

    #[test]
    fn unsupported_spear_trap_and_detached_spears_are_removed_in_game_step() {
        let (mut game, spear) = spear_room(false, 90.0);
        game.world
            .entities
            .iter_mut()
            .find(|e| e.object == "oSpearTrapTop")
            .unwrap()
            .alive = false;
        game.step(&crate::InputFrame::default());
        assert!(game.world.entity(spear).is_none());
        assert_eq!(game.player.health, 16);
        let mut game = room();
        let trap = game.world.spawn("oSpearTrapTop", Vec2::new(80.0, 80.0));
        game.data
            .insert(format!("hazard_entity:{trap}:cooldown"), 25.0);
        game.step(&crate::InputFrame::default());
        assert!(game.world.entity(trap).is_none());
    }

    #[test]
    fn spear_detector_uses_enemy_origin_and_excludes_treasure() {
        let mut game = room();
        for x in 0..game.world.width {
            game.world.set_tile(x, 6, "oHardBlock");
        }
        game.world.spawn("oSpearTrapTop", Vec2::new(80.0, 80.0));
        game.world.spawn("oCaveman", Vec2::new(110.0, 80.0));
        game.step(&crate::InputFrame::default());
        assert!(
            game.world
                .entities
                .iter()
                .any(|e| e.object == "oSpearsLeft" && e.sprite == "sSpearsRight")
        );

        let mut game = room();
        for x in 0..game.world.width {
            game.world.set_tile(x, 6, "oHardBlock");
        }
        game.world.spawn("oSpearTrapTop", Vec2::new(80.0, 80.0));
        game.world.spawn("oGoldNugget", Vec2::new(110.0, 88.0));
        game.step(&crate::InputFrame::default());
        assert!(
            !game
                .world
                .entities
                .iter()
                .any(|e| e.object == "oSpearsLeft")
        );
    }
}
