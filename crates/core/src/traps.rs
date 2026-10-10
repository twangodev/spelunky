//! Classic hazards translated from object event code. Static hazard positions are
//! GameMaker instance origins (usually tile top-left), not actor centers.
use crate::model::{Entity, Game, GameEvent, PlayerState, Rect, Vec2};

#[path = "traps/altar.rs"]
mod altar;
#[path = "traps/curses.rs"]
mod curses;
#[path = "traps/moving.rs"]
mod moving;

pub fn tick(game: &mut Game) {
    if game.player.health <= 0 {
        return;
    }
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
fn lethal(game: &mut Game) {
    // Source environmental kills bypass the ordinary damage invulnerability.
    if game.player.health > 0 {
        let amount = game.player.health.max(99);
        game.player.health -= amount;
        game.player.state = PlayerState::Dead;
        game.events.push(GameEvent::Damage { amount });
    }
}
fn environment(game: &mut Game) {
    let all = hazards(game);
    let pos = game.player.pos;
    for h in &all {
        if h.object == "oLava" && rect(h).contains(Vec2::new(pos.x, pos.y + 6.0)) {
            lethal(game);
            game.player.vel = Vec2::new(0.0, 0.1);
            game.player.data.insert("burning".into(), 100.0);
            game.sound("sndFlame", pos);
        }
        let fall = game.player.data.get("fall_ticks").copied().unwrap_or(0.0);
        if h.object == "oSpikes"
            && Rect::new(pos.x - 4.0, pos.y - 4.0, 8.0, 12.0).intersects(rect(h))
            && game.player.vel.y > 0.0
            && (fall > 4.0 || game.player.stun > 0)
        {
            lethal(game);
            game.player.vel = Vec2::default();
            game.sound("sndHurt", pos);
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
        let r = rect(web);
        let wet = all.iter().any(|h| {
            matches!(h.object.as_str(), "oWater" | "oWaterSwim" | "oLava") && rect(h).intersects(r)
        });
        let magma = game
            .world
            .entities
            .iter()
            .any(|e| e.alive && e.object == "oMagmaMan" && e.bounds().intersects(r));
        if wet || magma {
            remove_hazard(game, web);
            continue;
        }
        for e in &mut game.world.entities {
            if e.alive
                && !e.held
                && e.bounds().intersects(r)
                && movable(e)
                && !matches!(
                    e.object.as_str(),
                    "oSpider" | "oGiantSpider" | "oRope" | "oRopeTop" | "oRopeSegment"
                )
            {
                e.vel = Vec2::default();
                e.data.insert("webbed".into(), 2.0);
            }
        }
        let decay_key = format!("{}:life", web.data_key);
        let dying = web
            .id
            .and_then(|id| game.world.entity(id))
            .is_some_and(|e| e.value("dying") != 0.0);
        if dying {
            let life = game.data.get(&decay_key).copied().unwrap_or(12.0) - 0.02;
            game.data.insert(decay_key, life);
            if life <= 1.0 {
                remove_hazard(game, web);
            }
        }
    }
}
fn remove_hazard(game: &mut Game, h: &Hazard) {
    if let Some(id) = h.id {
        if let Some(e) = game.world.entity_mut(id) {
            e.alive = false;
        }
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
        "oRubblePiece",
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
                let activated = repeat
                    || (game.player.vel.x != 0.0 || game.player.vel.y != 0.0)
                        && area.intersects(game.player.bounds())
                    || game.world.entities.iter().any(|e| {
                        e.alive
                            && e.id != h.id.unwrap_or(0)
                            && (movable(e)
                                || matches!(e.object.as_str(), "oBoulder" | "oPushBlock"))
                            && (e.vel.x != 0.0 || e.vel.y != 0.0)
                            && area.intersects(e.bounds())
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
                    game.data.insert(key, cooldown - 1.0);
                    continue;
                }
                let center = Vec2::new(h.pos.x + 8.0, h.pos.y + 8.0);
                let mut target = None;
                if (game.player.pos.y - center.y).abs() < 4.0
                    && game.player.pos.distance(center) < 64.0
                {
                    target = Some(game.player.pos);
                }
                if target.is_none() {
                    target = game
                        .world
                        .entities
                        .iter()
                        .filter(|e| e.alive && movable(e))
                        .find(|e| (e.pos.y - center.y).abs() < 4.0 && e.pos.distance(center) < 64.0)
                        .map(|e| e.pos);
                }
                if let Some(target) = target {
                    let right = target.x > center.x;
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
                } else {
                    game.data.insert(key, 0.0);
                }
            }
            "oSpearsLeft" => {
                if let Some(id) = h.id {
                    let mut damage = None;
                    let overlaps = rect(&h).intersects(game.player.bounds());
                    if let Some(e) = game.world.entity_mut(id) {
                        e.animation += 1.0;
                        if e.animation >= 20.0 && e.animation < 24.0 && overlaps {
                            damage = Some(e.facing);
                        }
                        if e.animation >= 32.0 {
                            e.alive = false;
                        }
                    }
                    if let Some(facing) = damage {
                        game.damage_player(4);
                        game.player.vel = Vec2::new(
                            f64::from(facing) * (4.0 + f64::from(game.rng.range(3))),
                            -6.0,
                        );
                    }
                }
            }
            _ => {}
        }
    }
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
}
