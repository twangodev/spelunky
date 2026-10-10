//! Arrow detector collision events and spear object events from Classic 1.1.
//! The dispatcher owns alarm countdown, collision pairing, and image advancement.
use super::*;
use crate::dispatcher::Instance;

pub(super) fn handles(object: &str) -> bool {
    matches!(
        object,
        "oArrowTrapLeft"
            | "oArrowTrapRight"
            | "oArrowTrapLeftLit"
            | "oArrowTrapRightLit"
            | "oArrowRepeaterL"
            | "oArrowRepeaterR"
            | "oArrowTrapTest"
            | "oSpearTrapTop"
            | "oSpearTrapBottom"
            | "oSpearTrapLit"
            | "oSpearsLeft"
    )
}

fn hazard(game: &Game, instance: Instance) -> Option<Hazard> {
    if let Instance::Entity(id) = instance {
        game.world.entity(id)?;
    }
    let object = instance.object(game);
    if !handles(object) {
        return None;
    }
    let (id, data_key) = match instance {
        Instance::Entity(id) => (Some(id), format!("hazard_entity:{id}")),
        Instance::Terrain(index) => (None, format!("hazard:{index}")),
        Instance::Player => return None,
    };
    Some(Hazard {
        id,
        object: object.into(),
        pos: instance.position(game),
        data_key,
    })
}

fn right(object: &str) -> bool {
    matches!(
        object,
        "oArrowTrapRight" | "oArrowTrapRightLit" | "oArrowRepeaterR"
    )
}

/// Fixed Create defaults run in World::spawn before caller overrides.
pub(super) fn init_entity(entity: &mut Entity) {
    match entity.object.as_str() {
        "oArrowTrapLeft" | "oArrowTrapRight" | "oArrowTrapLeftLit" | "oArrowTrapRightLit" => {
            entity.alarms[1] = 1;
        }
        "oSpearTrapTop" | "oSpearTrapLit" => {
            entity.data.insert("animation_speed".into(), 0.5);
        }
        _ => {}
    }
}

/// Source Create initializes alarms even before an instance's first active Step.
pub(super) fn create(game: &mut Game, instance: Instance) {
    let Some(h) = hazard(game, instance) else {
        return;
    };
    let key = format!("{}:created", h.data_key);
    if game.data.contains_key(&key) {
        return;
    }
    game.data.insert(key, 1.0);
    match h.object.as_str() {
        "oArrowTrapLeft" | "oArrowTrapRight" | "oArrowTrapLeftLit" | "oArrowTrapRightLit" => {
            game.data.insert(format!("{}:fired", h.data_key), 0.0);
            if matches!(instance, Instance::Terrain(_)) {
                if let Some(alarms) = instance.alarms_mut(game) {
                    alarms[1] = 1;
                }
            }
        }
        "oArrowRepeaterL" | "oArrowRepeaterR" => {
            let delay = 100.0 + f64::from(game.rng.range(100));
            game.data.insert(format!("{}:fired", h.data_key), delay);
        }
        "oSpearTrapTop" | "oSpearTrapBottom" | "oSpearTrapLit" => {
            game.data.insert(format!("{}:cooldown", h.data_key), 0.0);
        }
        _ => {}
    }
}

/// Alarm 1 stretches a real sRed instance once; later wall changes do not resize it.
fn detector(game: &Game, h: &Hazard, faces_right: bool) -> Rect {
    let y = h.pos.y + 8.0;
    if faces_right {
        let mut x = h.pos.x + 16.0;
        let mut remaining = 100;
        while !game.world.solid_at(x, y) && remaining > 0 {
            if x - h.pos.x > 96.0 {
                break;
            }
            x += 1.0;
            remaining -= 1;
        }
        let span = (x - h.pos.x - 8.0).max(32.0);
        Rect::new(
            h.pos.x + 16.0,
            h.pos.y,
            ((span - 16.0) / 16.0).ceil() * 16.0,
            16.0,
        )
    } else {
        let mut x = h.pos.x - 1.0;
        while !game.world.solid_at(x, y) {
            if h.pos.x - x > 96.0 {
                break;
            }
            x -= 1.0;
        }
        x = x.min(h.pos.x - 16.0);
        Rect::new(x, h.pos.y, ((h.pos.x - 1.0 - x) / 16.0).ceil() * 16.0, 16.0)
    }
}

#[cfg(test)]
pub(super) fn detector_occupied_bounds(area: Rect) -> Rect {
    let sprite = crate::catalog::sprite_def("sRed").unwrap();
    let [left, top, right, bottom] = sprite.bounds;
    let [ox, oy] = sprite.origin;
    let scale = area.w / 16.0;
    let x0 = (area.x + f64::from(left - ox) * scale).round_ties_even();
    let x1 = (area.x + f64::from(right - ox + 1) * scale - 1.0).round_ties_even();
    let y0 = (area.y + f64::from(top - oy)).round_ties_even();
    let y1 = (area.y + f64::from(bottom - oy)).round_ties_even();
    Rect::new(x0, y0, x1 - x0 + 1.0, y1 - y0 + 1.0)
}

fn emit(game: &mut Game, h: &Hazard, faces_right: bool, speed: f64, offset: f64) {
    let id = game.spawn("oArrow", Vec2::new(h.pos.x + offset, h.pos.y + 4.0));
    if let Some(arrow) = game.world.entity_mut(id) {
        arrow.vel.x = speed;
        arrow.facing = if faces_right { 1 } else { -1 };
        arrow.data.insert("armed".into(), 1.0);
        arrow.data.insert("owner".into(), -1.0);
    }
}

pub(super) fn alarm(game: &mut Game, instance: Instance, index: u8) {
    let Some(h) = hazard(game, instance) else {
        return;
    };
    match (h.object.as_str(), index) {
        ("oArrowTrapLeft" | "oArrowTrapRight" | "oArrowTrapLeftLit" | "oArrowTrapRightLit", 1) => {
            if game.flags.contains("level_editor") {
                return;
            }
            let area = detector(game, &h, right(&h.object));
            let id = game.spawn("oArrowTrapTest", Vec2::new(area.x, area.y));
            let detector = game.world.entity_mut(id).unwrap();
            detector.data.insert("image_xscale".into(), area.w / 16.0);
            match instance {
                Instance::Entity(parent) => {
                    detector.data.insert("trap_entity".into(), parent as f64);
                }
                Instance::Terrain(tile) => {
                    detector.data.insert("trap_tile".into(), tile as f64);
                }
                Instance::Player => unreachable!(),
            }
        }
        (
            "oArrowTrapLeft" | "oArrowTrapRight" | "oArrowTrapLeftLit" | "oArrowTrapRightLit"
            | "oArrowRepeaterL" | "oArrowRepeaterR",
            0,
        ) => emit(game, &h, true, 5.0, 16.0),
        ("oSpearTrapTop" | "oSpearTrapBottom" | "oSpearTrapLit", 0) => {
            emit(game, &h, false, -5.0, -16.0)
        }
        _ => {}
    }
}

pub(super) fn step_instance(game: &mut Game, instance: Instance) {
    let Some(h) = hazard(game, instance) else {
        return;
    };
    match h.object.as_str() {
        "oArrowRepeaterL" | "oArrowRepeaterR" => {
            let key = format!("{}:fired", h.data_key);
            let fired = game.data.get(&key).copied().unwrap_or(0.0);
            if fired > 0.0 {
                game.data.insert(key, fired - 1.0);
            } else if game.state != crate::RunState::Dead {
                let faces_right = right(&h.object);
                emit(
                    game,
                    &h,
                    faces_right,
                    if faces_right { 8.0 } else { -8.0 },
                    if faces_right { 18.0 } else { -2.0 },
                );
                let delay = 100.0 + f64::from(game.rng.range(101));
                game.data.insert(key, delay);
                game.sound("sndArrowTrap", h.pos);
            }
        }
        "oSpearTrapTop" | "oSpearTrapBottom" | "oSpearTrapLit" => {
            let key = format!("{}:cooldown", h.data_key);
            let cooldown = game.data.get(&key).copied().unwrap_or(0.0);
            let cooldown = (cooldown - 1.0).max(0.0);
            game.data.insert(key.clone(), cooldown);
            if cooldown == 0.0 {
                if let Some(faces_right) = spear_target(game, &h) {
                    let id = game.spawn(
                        "oSpearsLeft",
                        Vec2::new(h.pos.x + if faces_right { 16.0 } else { -16.0 }, h.pos.y),
                    );
                    let e = game.world.entity_mut(id).unwrap();
                    if faces_right {
                        e.sprite = "sSpearsRight".into();
                    }
                    e.facing = if faces_right { 1 } else { -1 };
                    game.data.insert(key, 50.0);
                }
            }
            let active = h.pos.x > game.camera.x - 8.0
                && h.pos.x < game.camera.x + 328.0
                && h.pos.y > game.camera.y - 8.0
                && h.pos.y < game.camera.y + 248.0;
            if active && !game.world.solid_at(h.pos.x, h.pos.y + 16.0) {
                remove_hazard(game, &h);
            }
            if h.object == "oSpearTrapBottom" {
                if let Some(id) = h.id {
                    if let Some(e) = game.world.entity_mut(id) {
                        e.pos.x = e.pos.x.ceil();
                        e.pos.y = e.pos.y.ceil();
                    }
                }
            }
        }
        "oSpearsLeft" => {
            let Some(id) = h.id else {
                return;
            };
            let Some(e) = game.world.entity(id) else {
                return;
            };
            let faces_right = e.sprite == "sSpearsRight";
            let anchor = Vec2::new(h.pos.x + if faces_right { -16.0 } else { 16.0 }, h.pos.y);
            let supported = game
                .world
                .tiles_collide(Rect::new(anchor.x, anchor.y, 1.0, 1.0), "oSpearTrapTop")
                || game
                    .world
                    .tiles_collide(Rect::new(anchor.x, anchor.y, 1.0, 1.0), "oSpearTrapBottom")
                || game.world.entities.iter().any(|trap| {
                    trap.alive
                        && (crate::catalog::is_a(&trap.object, "oSpearTrapTop")
                            || crate::catalog::is_a(&trap.object, "oSpearTrapBottom"))
                        && trap.sprite_bounds().contains(anchor)
                });
            if let Some(e) = game.world.entity_mut(id) {
                e.data.insert("depth".into(), 995.0);
                if !supported {
                    e.alive = false;
                }
            }
        }
        // Ordinary arrow traps have no active firing code in their Step.
        _ => {}
    }
}

pub(super) fn collision(game: &mut Game, instance: Instance, other: Instance) {
    let Instance::Entity(id) = instance else {
        return;
    };
    let Some(e) = game.world.entity(id) else {
        return;
    };
    if e.object != "oArrowTrapTest" {
        return;
    }
    let trap = if let Some(&id) = e.data.get("trap_entity") {
        Some(Instance::Entity(id as u64))
    } else {
        e.data
            .get("trap_tile")
            .map(|&index| Instance::Terrain(index as usize))
    };
    let object = other.object(game);
    let character = crate::catalog::is_a(object, "oCharacter");
    if !character
        && !["oEnemy", "oItem", "oMoveableSolid", "oTreasure", "oBoulder"]
            .iter()
            .any(|family| crate::catalog::is_a(object, family))
    {
        return;
    }
    let velocity = match other {
        Instance::Player => game.player.vel,
        Instance::Entity(id) => game.world.entity(id).map_or(Vec2::default(), |e| e.vel),
        Instance::Terrain(_) => Vec2::default(),
    };
    let duck_to_hang = character
        && matches!(
            game.player.sprite.as_str(),
            "sDuckToHangL" | "sDamselDtHL" | "sTunnelDtHL"
        )
        && game.player.animation > 6.0;
    if trap.is_some() && velocity == Vec2::default() && !duck_to_hang {
        return;
    }
    if let Some(h) = trap.and_then(|trap| hazard(game, trap)) {
        let key = format!("{}:fired", h.data_key);
        if game.data.get(&key).copied().unwrap_or(0.0) == 0.0 {
            let faces_right = right(&h.object);
            emit(
                game,
                &h,
                faces_right,
                if faces_right { 8.0 } else { -8.0 },
                if faces_right { 18.0 } else { -2.0 },
            );
            game.data.insert(key, 1.0);
            game.sound("sndArrowTrap", h.pos);
        }
    }
    if let Some(e) = game.world.entity_mut(id) {
        e.alive = false;
    }
}

pub(super) fn animation_end(game: &mut Game, instance: Instance) {
    if let Instance::Entity(id) = instance {
        if let Some(e) = game.world.entity_mut(id) {
            if e.object == "oSpearsLeft" {
                e.alive = false;
            }
        }
    }
}

/// One collision_rectangle query in oPlayer1's late Hurt action selects one spear.
pub(super) fn player_hurt(game: &mut Game) {
    let area = Rect::new(game.player.pos.x - 6.0, game.player.pos.y - 6.0, 13.0, 14.0);
    let Some((phase, faces_left)) = game
        .world
        .entities
        .iter()
        .find(|e| e.alive && e.object == "oSpearsLeft" && e.sprite_bounds().intersects(area))
        .map(|e| (e.animation, e.sprite == "sSpearsLeft"))
    else {
        return;
    };
    if !(20.0..24.0).contains(&phase) {
        return;
    }
    if game.player.health > 0 && crate::statistics::is_real_level(game) {
        game.lifetime_stats
            .misc_death(crate::statistics::MiscDeath::SpearTrap);
    }
    game.player.health -= 4;
    game.events.push(GameEvent::Damage { amount: 4 });
    let speed = 4.0 + f64::from(game.rng.range(3));
    game.player.vel = Vec2::new(if faces_left { -speed } else { speed }, -6.0);
    game.player.pos.y -= 1.0;
    // Source deliberately leaves the independent stun flag and movement state alone.
    if game.player.data.get("bloodless").copied().unwrap_or(0.0) == 0.0
        && game
            .world
            .entities
            .iter()
            .filter(|e| e.alive && crate::catalog::is_a(&e.object, "oDetritus"))
            .count()
            < 16
    {
        game.spawn("oBlood", game.player.pos);
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{InputFrame, RulesPreset, RunState};

    fn supported_room(player_x: f64) -> Game {
        let mut game = Game::shell(71, RulesPreset::Classic);
        for x in 0..game.world.width {
            game.world.set_tile(x, 10, "oBlock");
        }
        game.player.pos = Vec2::new(player_x, 152.0);
        game
    }

    #[test]
    fn game_step_arrow_detector_fires_after_motion_and_leaves_birth_arrow_unmoved() {
        for moving in [false, true] {
            let mut game = supported_room(104.0);
            game.spawn("oArrowTrapRight", Vec2::new(64.0, 144.0));
            game.step(&InputFrame {
                jump: moving,
                jump_pressed: moving,
                ..Default::default()
            });
            let arrows: Vec<_> = game
                .world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == "oArrow")
                .collect();
            assert_eq!(arrows.len(), usize::from(moving));
            assert_eq!(
                game.player.health, 4,
                "the detector emits after the player's Hurt action"
            );
            if moving {
                assert_eq!(
                    arrows[0].pos,
                    Vec2::new(82.0, 148.0),
                    "automatic collision birth cannot receive the earlier Arrow Step"
                );
            } else {
                assert!(
                    game.world
                        .entities
                        .iter()
                        .any(|e| e.alive && e.object == "oArrowTrapTest")
                );
            }
        }
    }

    #[test]
    fn game_step_naturally_extended_spear_reads_prior_frame_and_control_retracts_at_31() {
        for exposed in [false, true] {
            let mut game = supported_room(if exposed { 87.0 } else { 79.0 });
            game.spawn("oSpearTrapTop", Vec2::new(96.0, 144.0));
            for tick in 1..=20 {
                game.step(&InputFrame::default());
                assert_eq!(
                    game.player.health, 4,
                    "exposed={exposed}, tick={tick}: damage must wait until the next Player Step sees frame20"
                );
            }
            game.step(&InputFrame::default());
            if exposed {
                assert_eq!(game.player.health, 0);
                assert_eq!(
                    game.player.stun, 0,
                    "spears do not set the source stun flag"
                );
                assert_eq!(game.state, RunState::Dead);
                assert_eq!(game.lifetime_stats.misc_deaths[7], 1);
            } else {
                assert_eq!(game.player.health, 4);
                for _ in 21..31 {
                    game.step(&InputFrame::default());
                }
                assert!(
                    !game
                        .world
                        .entities
                        .iter()
                        .any(|e| e.alive && e.object == "oSpearsLeft"),
                    "31 source frames dispatch Animation End destruction"
                );
                assert_eq!(game.lifetime_stats.misc_deaths[7], 0);
            }
        }
    }

    #[test]
    fn authored_spear_survivor_counts_under_controlled_real_level_eligibility() {
        for real_level in [false, true] {
            let mut level = crate::level::Level::default();
            // Life is an authored setting. Entrance, trap, and support are all
            // imported through the public custom-level path.
            level.starting_life = 20;
            level.set_tile(6, 7, '@').unwrap();
            level.set_tile(7, 7, ']').unwrap();
            for x in 0..40 {
                level.set_tile(x, 8, '1').unwrap();
            }
            let mut game = Game::shell(71, RulesPreset::Classic);
            game.load_custom(&level, "authored-spear-counters", "");
            for _ in 0..8 {
                game.step(&InputFrame::default());
            }
            assert!(game.player.grounded);
            assert_eq!(game.player.health, 20);
            // Only eligibility metadata changes: this is a controlled real-room
            // context comparison, not a claimed natural campaign route.
            if real_level {
                game.flags.remove("custom");
                game.flags.remove("custom_level");
            }
            assert_eq!(crate::statistics::is_real_level(&game), real_level);
            for _ in 8..20 {
                game.step(&InputFrame::default());
                assert_eq!(game.player.health, 20);
            }
            game.step(&InputFrame::default());
            assert_eq!(game.player.health, 16);
            assert_eq!(game.state, RunState::Playing);
            assert_eq!(game.player.stun, 0);
            assert_eq!(game.lifetime_stats.misc_deaths[7], u64::from(real_level));
            assert_eq!(
                game.events
                    .iter()
                    .filter(|event| matches!(event, GameEvent::Damage { amount: 4 }))
                    .count(),
                1
            );
        }
    }
}
