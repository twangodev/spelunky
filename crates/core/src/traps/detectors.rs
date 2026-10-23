//! Arrow detector collision events and spear object events from Classic 1.1.
//! The dispatcher owns alarm countdown, collision pairing, and image advancement.
use super::*;
use crate::dispatcher::Instance;

pub(super) fn handles(object: &str) -> bool {
    matches!(
        object,
        "oTemp"
            | "oArrowTrapLeft"
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
        "oTemp" => {
            // Inherited DrawnSprite/Solid Create precedes the own trap fields.
            for key in [
                "blinkToggle",
                "invincible",
                "shopWall",
                "cleanDeath",
                "fired",
            ] {
                entity.data.insert(key.into(), 0.0);
            }
            entity.text.insert("treasure".into(), String::new());
            entity.text.insert("type".into(), "Arrow Trap".into());
        }
        "oArrowTrapLeft" | "oArrowTrapRight" | "oArrowTrapLeftLit" | "oArrowTrapRightLit" => {
            entity.alarms[1] = 1;
        }
        "oSpearTrapTop" | "oSpearTrapBottom" | "oSpearTrapLit" => {
            for (field, value) in [
                ("fired", 0.0),
                ("firedMax", 50.0),
                ("prox", 4.0),
                ("invincible", 0.0),
            ] {
                entity.data.insert(field.into(), value);
            }
            if entity.object != "oSpearTrapBottom" {
                entity.data.insert("animation_speed".into(), 0.5);
            }
        }
        _ => {}
    }
}

/// RNG-free source Spear inheritance and own Create. Historical generation
/// retains its older fixed-default adapter; the new revision shares this body
/// between generated births and runtime Create without resetting overrides.
pub(super) fn initialize_source_jungle_spear(entity: &mut Entity) {
    if !matches!(
        entity.object.as_str(),
        "oSpearTrapTop" | "oSpearTrapBottom" | "oSpearTrapLit"
    ) || entity.value("source_spear_create_initialized") != 0.0
    {
        return;
    }
    entity
        .data
        .insert("source_spear_create_initialized".into(), 1.0);
    for (key, value) in [
        ("blinkToggle", 0.0),
        ("invincible", 0.0),
        ("shopWall", 0.0),
        ("cleanDeath", 0.0),
        ("fired", 0.0),
        ("firedMax", 50.0),
        ("prox", 4.0),
    ] {
        entity.data.insert(key.into(), value);
    }
    entity.text.insert("type".into(), "None".into());
    entity.text.insert("treasure".into(), String::new());
    if entity.object != "oSpearTrapBottom" {
        entity.set_image_speed(0.5);
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
            // Source rand(100) supplies argument0=100 and omitted argument1=0.
            // Its signed width is -99; historical replay keeps its old mapping.
            let delay = match game.rng.algorithm() {
                crate::RngAlgorithm::Gm8Lcg32V1 => {
                    100.0
                        + game
                            .rng
                            .source_rand(100, 0)
                            .expect("fixed source rand width") as f64
                }
                crate::RngAlgorithm::LegacySplitMix64V1 => 100.0 + f64::from(game.rng.range(100)),
            };
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
        while !game
            .world
            .solid_point_active(Vec2::new(x, y), CollisionPrecision::BoundingBox, None)
            && remaining > 0
        {
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
        while !game
            .world
            .solid_point_active(Vec2::new(x, y), CollisionPrecision::BoundingBox, None)
        {
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
        ("oTemp", 0) => temp_emit(game, &h, -5.0),
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
        "oTemp" => temp_step(game, &h),
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
            spear_step(game, &h);
            let view = game.simulation_view_rect();
            let active = h.pos.x > view.x - 8.0
                && h.pos.x < view.x + (view.w + 8.0)
                && h.pos.y > view.y - 8.0
                && h.pos.y < view.y + (view.h + 8.0);
            if active && !spear_support(game, Vec2::new(h.pos.x, h.pos.y + 16.0), "oSolid") {
                remove_hazard(game, &h);
            }
            if h.object == "oSpearTrapBottom" {
                if let Some(id) = h.id {
                    if let Some(e) = crate::dispatcher::stored_entity_mut(game, id) {
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
            let supported = spear_support(game, anchor, "oSpearTrapTop")
                || spear_support(game, anchor, "oSpearTrapBottom");
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
    let Some((phase, faces_left)) = crate::dispatcher::descendant_instances(game, "oSpearsLeft")
        .iter()
        .filter(|instance| crate::dispatcher::is_active(game, **instance))
        .find_map(|instance| match instance {
            Instance::Entity(id) => game.world.entity(*id).and_then(|entity| {
                entity
                    .sprite_bounds()
                    .intersects(area)
                    .then_some((entity.animation, entity.sprite == "sSpearsLeft"))
            }),
            Instance::Terrain(_) | Instance::Player => None,
        })
    else {
        return;
    };
    if !(20.0..24.0).contains(&phase) {
        return;
    }
    if game.player.health > 0.0 && crate::statistics::is_real_level(game) {
        game.lifetime_stats
            .misc_death(crate::statistics::MiscDeath::SpearTrap);
    }
    game.player.lose_health(4);
    game.events.push(GameEvent::Damage { amount: 4 });
    let speed = 4.0 + f64::from(game.rng.range(3));
    game.player.vel = Vec2::new(if faces_left { -speed } else { speed }, -6.0);
    game.player.pos.y -= 1.0;
    // Source deliberately leaves the independent stun flag and movement state alone.
    if game.player.data.get("bloodless").copied().unwrap_or(0.0) == 0.0
        && crate::world_events::detritus_instance_count(game) < 16
    {
        game.spawn("oBlood", game.player.pos);
    }
}

fn spear_support(game: &Game, point: Vec2, family: &str) -> bool {
    crate::dispatcher::descendant_instances(game, family)
        .into_iter()
        .filter(|instance| crate::dispatcher::is_active(game, *instance))
        .any(|instance| match instance {
            Instance::Entity(id) => game.world.entity(id).is_some_and(|entity| {
                entity.mask_contains(point, crate::CollisionPrecision::BoundingBox)
            }),
            Instance::Terrain(index) => game
                .world
                .tile_collision_mask(
                    (index % game.world.width) as i32,
                    (index / game.world.width) as i32,
                )
                .contains(point, crate::CollisionPrecision::BoundingBox),
            Instance::Player => false,
        })
}

fn spear_value(game: &Game, h: &Hazard, field: &str, fallback: f64) -> f64 {
    h.id.and_then(|id| game.world.entity(id))
        .and_then(|entity| entity.data.get(field).copied())
        .unwrap_or(fallback)
}

fn spear_set_fired(game: &mut Game, h: &Hazard, fired: f64) {
    if let Some(id) = h.id {
        game.world
            .entity_mut(id)
            .unwrap()
            .data
            .insert("fired".into(), fired);
    }
    // Preserve the terrain adapter's state and existing inspection alias.
    game.data.insert(format!("{}:cooldown", h.data_key), fired);
}

fn spear_emit(game: &mut Game, h: &Hazard, right: bool, maximum: f64) {
    let id = game.spawn(
        "oSpearsLeft",
        Vec2::new(h.pos.x + if right { 16.0 } else { -16.0 }, h.pos.y),
    );
    if right {
        game.world.entity_mut(id).unwrap().sprite = "sSpearsRight".into();
        if let Some(receiver) = h.id.and_then(|id| game.world.entity_mut(id)) {
            receiver.data.insert("spears".into(), id as f64);
        }
    }
    spear_set_fired(game, h, maximum);
}

fn spear_step(game: &mut Game, h: &Hazard) {
    let key = format!("{}:cooldown", h.data_key);
    let mut fired = spear_value(
        game,
        h,
        "fired",
        game.data.get(&key).copied().unwrap_or(0.0),
    );
    if fired > 0.0 {
        fired -= 1.0;
    }
    spear_set_fired(game, h, fired);
    let prox = spear_value(game, h, "prox", 4.0);
    let maximum = spear_value(game, h, "firedMax", 50.0);
    if let Some(receiver) = h.id.and_then(|id| game.world.entity_mut(id)) {
        receiver.data.insert("range".into(), 64.0);
    }
    let center = Vec2::new(h.pos.x + 8.0, h.pos.y + 8.0);
    // Each source side performs Player, Enemy, MoveableSolid, Item in order.
    // Nearest scratch writes still occur after firing; a configured zero maximum
    // can legitimately admit subsequent branches in the same Step.
    for right in [false, true] {
        let side = |x: f64, boundary: f64| if right { x > boundary } else { x < boundary };
        if crate::dispatcher::is_active(game, Instance::Player)
            && fired == 0.0
            && (game.player.pos.y - center.y).abs() < prox
            && side(game.player.pos.x, if right { center.x } else { h.pos.x })
            && center.distance(game.player.pos) < 64.0
        {
            spear_emit(game, h, right, maximum);
            fired = maximum;
        }
        for family in ["oEnemy", "oMoveableSolid", "oItem"] {
            let target = temp_nearest(game, h.pos, family);
            if let Some(receiver) = h.id.and_then(|id| game.world.entity_mut(id)) {
                match target {
                    Some(Instance::Entity(id)) => {
                        receiver.data.insert("obj".into(), id as f64);
                    }
                    None => {
                        receiver.data.insert("obj".into(), -4.0);
                    }
                    _ => {} // The terrain adapter has no native numeric instance ID.
                }
            }
            if let Some(target) = target {
                let pos = target.position(game);
                let origin = if family == "oItem" { center } else { h.pos };
                let boundary = if family == "oItem" { center.x } else { h.pos.x };
                if fired == 0.0
                    && (pos.y - origin.y).abs() < prox
                    && side(pos.x, boundary)
                    && origin.distance(pos) < 64.0
                {
                    spear_emit(game, h, right, maximum);
                    fired = maximum;
                }
            }
        }
    }
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
                game.player.health, 4.0,
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
                    game.player.health, 4.0,
                    "exposed={exposed}, tick={tick}: damage must wait until the next Player Step sees frame20"
                );
            }
            game.step(&InputFrame::default());
            if exposed {
                assert_eq!(game.player.health, 0.0);
                assert_eq!(
                    game.player.stun, 0,
                    "spears do not set the source stun flag"
                );
                assert_eq!(game.state, RunState::Dead);
                assert_eq!(game.lifetime_stats.misc_deaths[7], 1);
            } else {
                assert_eq!(game.player.health, 4.0);
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
            assert_eq!(game.player.health, 20.0);
            // Only eligibility metadata changes: this is a controlled real-room
            // context comparison, not a claimed natural campaign route.
            if real_level {
                game.flags.remove("custom");
                game.flags.remove("custom_level");
            }
            assert_eq!(crate::statistics::is_real_level(&game), real_level);
            for _ in 8..20 {
                game.step(&InputFrame::default());
                assert_eq!(game.player.health, 20.0);
            }
            game.step(&InputFrame::default());
            assert_eq!(game.player.health, 16.0);
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

// oTemp is an older movement-triggered trap, not the stretched detector family.
fn temp_emit(game: &mut Game, h: &Hazard, speed: f64) {
    let arrow = game.spawn("oArrow", Vec2::new(h.pos.x - 16.0, h.pos.y + 4.0));
    game.world.entity_mut(arrow).unwrap().vel.x = speed;
    if let Some(id) = h.id {
        game.world
            .entity_mut(id)
            .unwrap()
            .data
            .insert("arrow".into(), arrow as f64);
    }
}

fn temp_nearest(game: &Game, origin: Vec2, family: &str) -> Option<Instance> {
    crate::dispatcher::descendant_instances(game, family)
        .into_iter()
        .filter(|instance| {
            crate::dispatcher::is_active(game, *instance)
                && match instance {
                    Instance::Entity(id) => game.world.entity(*id).is_some(),
                    _ => true,
                }
        })
        .min_by(|a, b| {
            a.position(game)
                .distance(origin)
                .total_cmp(&b.position(game).distance(origin))
        })
}

fn temp_moving(game: &Game, target: Instance) -> bool {
    match target {
        Instance::Player => game.player.vel.x.abs() > 0.0 || game.player.vel.y.abs() > 0.0,
        Instance::Entity(id) => game
            .world
            .entity(id)
            .is_some_and(|e| e.vel.x.abs() > 0.0 || e.vel.y.abs() > 0.0),
        Instance::Terrain(_) => false,
    }
}

fn temp_step(game: &mut Game, h: &Hazard) {
    let key = format!("{}:fired", h.data_key);
    let mut fired = h.id.and_then(|id| game.world.entity(id)).map_or_else(
        || game.data.get(&key).copied().unwrap_or(0.0),
        |e| e.value("fired"),
    );
    let center = Vec2::new(h.pos.x + 8.0, h.pos.y + 8.0);
    // Missing qualified Player reads are outside the infallible adapter's error
    // model; this branch is qualified only when the source Player is active.
    if crate::dispatcher::is_active(game, Instance::Player)
        && fired == 0.0
        && (game.player.pos.y - h.pos.y - 8.0).abs() < 8.0
        && game.player.pos.x < h.pos.x + 8.0
        && center.distance(game.player.pos) < 128.0
        && temp_moving(game, Instance::Player)
    {
        temp_emit(game, h, -8.0);
        fired += 1.0;
        temp_set_fired(game, h, fired);
        game.sound("sndArrowTrap", h.pos);
    }
    // Every nearest lookup still occurs after an earlier branch fired. Source
    // ordering lets the final Item lookup observe the newly created Arrow.
    for (family, origin, distance_origin, x_limit, requires_motion) in [
        ("oEnemy", h.pos, h.pos, h.pos.x, true),
        ("oMoveableSolid", h.pos, h.pos, h.pos.x, false),
        (
            "oItem",
            Vec2::new(h.pos.x - 16.0, h.pos.y),
            center,
            h.pos.x + 8.0,
            true,
        ),
    ] {
        let target = temp_nearest(game, origin, family);
        if let Some(id) = h.id {
            match target {
                Some(Instance::Entity(target_id)) => {
                    game.world
                        .entity_mut(id)
                        .unwrap()
                        .data
                        .insert("obj".into(), target_id as f64);
                }
                None => {
                    game.world
                        .entity_mut(id)
                        .unwrap()
                        .data
                        .insert("obj".into(), -4.0);
                }
                // Terrain has no numeric instance ID in the typed adapter;
                // selection/coordinates still use its actual family member.
                _ => {}
            }
        }
        if let Some(target) = target {
            let pos = target.position(game);
            if fired == 0.0
                && (pos.y - h.pos.y - 8.0).abs() < 8.0
                && pos.x < x_limit
                && distance_origin.distance(pos) < 128.0
                && (!requires_motion || temp_moving(game, target))
            {
                temp_emit(game, h, -8.0);
                fired += 1.0;
                temp_set_fired(game, h, fired);
                game.sound("sndArrowTrap", h.pos);
            }
        }
    }
    if let Some(id) = h.id {
        let receiver = game.world.entity_mut(id).unwrap();
        receiver.data.insert("fired".into(), fired);
        receiver.pos.x = receiver.pos.x.ceil();
        receiver.pos.y = receiver.pos.y.ceil();
    } else {
        game.data.insert(key, fired);
    }
}

fn temp_set_fired(game: &mut Game, h: &Hazard, fired: f64) {
    if let Some(id) = h.id {
        game.world
            .entity_mut(id)
            .unwrap()
            .data
            .insert("fired".into(), fired);
    } else {
        game.data.insert(format!("{}:fired", h.data_key), fired);
    }
}

#[cfg(test)]
#[path = "temp_contract_tests.rs"]
mod temp_contract_tests;

#[cfg(test)]
#[path = "spear_nearest_contract_tests.rs"]
mod spear_nearest_contract_tests;
