//! Source world objects and visual effects that have no item/enemy/trap owner.
//! All animation rates below are frames per original 30 Hz simulation tick.
use crate::{Entity, Game, GameEvent, InputFrame, Rect, VIEW_HEIGHT, VIEW_WIDTH, Vec2, catalog};

pub fn tick(game: &mut Game, _input: &InputFrame) {
    if let Some(timer) = game.data.get_mut("message_timer") {
        *timer = (*timer - 1.0).max(0.0);
    }
    parachute(game);
    let count = game.world.entities.len();
    for index in 0..count {
        let mut e = game.world.entities[index].clone();
        if !e.alive {
            continue;
        }
        match e.object.as_str() {
            "oMsgSign" => sign(game, &mut e),
            "oLamp" | "oLampRed" | "oTikiTorch" => light(&mut e),
            "oIceBottom" => ice_bottom(game, &mut e),
            "oFrozenCaveman" => frozen_caveman(game, &mut e),
            "oBlood" | "oBone" => detritus(game, &mut e),
            "oPoof" | "oSmokePuff" | "oBloodSpark" | "oBloodTrail" | "oLavaDrip" | "oBubble" => {
                effect(game, &mut e)
            }
            "oBones" | "oFakeBones" | "oParaUsed" => fallen_decoration(game, &mut e),
            "oParachute" => {
                e.pos = Vec2::new(game.player.pos.x - 8.0, game.player.pos.y - 16.0);
                if game.world.entities.iter().any(|item| {
                    item.alive
                        && catalog::is_a(&item.object, "oItem")
                        && (item.vel.x > 0.0 || item.vel.y > 0.0)
                        && e.bounds().intersects(item.bounds())
                }) {
                    e.alive = false;
                    game.player.data.insert("parachute_open".into(), 0.0);
                    game.world.spawn("oParaUsed", e.pos);
                }
                if e.sprite == "sParaOpen" {
                    e.animation += 1.0;
                    if e.animation >= 5.0 {
                        e.sprite = "sParachute".into();
                        e.animation = 0.0;
                    }
                }
            }
            "oSmallCollect" | "oBigCollect" | "oHeart" | "oItemsGet" => collect_effect(&mut e),
            "oRubbleDark" => {
                e.pos.y += e.vel.y;
                e.vel.y += 0.6;
                if point_family(game, e.pos, "oBrick")
                    || point_family(game, e.pos, "oBlock")
                    || e.pos.x < 0.0
                    || e.pos.y < 0.0
                    || e.pos.x >= game.world.width as f64 * 16.0
                    || e.pos.y >= game.world.height as f64 * 16.0
                {
                    e.alive = false;
                }
            }
            _ if catalog::is_a(&e.object, "oRubblePiece") => rubble(game, &mut e),
            _ => {}
        }
        game.world.entities[index] = e;
    }
}

fn sign(game: &mut Game, e: &mut Entity) {
    let remaining = (e.value("message_cooldown") - 1.0).max(0.0);
    e.data.insert("message_cooldown".into(), remaining);
    if !e.bounds().intersects(game.player.bounds()) {
        return;
    }
    // The source collision refreshes the global display lifetime every tick.
    game.data.insert("message_timer".into(), 200.0);
    if remaining != 0.0 {
        return;
    }
    let first = e.text.get("message").map(String::as_str).unwrap_or("");
    let second = e.text.get("message2").map(String::as_str).unwrap_or("");
    let message = [first, second]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    if !message.is_empty() {
        game.events.push(GameEvent::Message(message));
        // Events notify the presentation once, rather than duplicating the same
        // notification on every overlapping tick. Display lifetime still refreshes.
        e.data.insert("message_cooldown".into(), 200.0);
    }
}

fn light(e: &mut Entity) {
    let torch = e.object == "oTikiTorch";
    e.animation = (e.animation + 0.5) % if torch { 5.0 } else { 3.0 };
    e.data.insert(
        "light_radius".into(),
        if e.object == "oLampRed" {
            0.0
        } else if torch {
            32.0
        } else {
            96.0
        },
    );
    e.data.insert("light_offset_x".into(), 8.0);
    e.data.insert("light_offset_y".into(), 8.0);
    if e.object == "oLampRed" {
        e.data.insert("red_light_range".into(), 96.0);
    }
}

fn point_family(game: &Game, point: Vec2, family: &str) -> bool {
    game.world
        .tile(
            (point.x / 16.0).floor() as i32,
            (point.y / 16.0).floor() as i32,
        )
        .is_some_and(|t| catalog::is_a(&t.object, family))
        || game
            .world
            .entities
            .iter()
            .any(|e| e.alive && catalog::is_a(&e.object, family) && e.bounds().contains(point))
}

fn ice_bottom(game: &mut Game, e: &mut Entity) {
    if !game.world.solid_at(e.pos.x + 8.0, e.pos.y - 1.0) {
        e.alive = false;
        return;
    }
    if !e.data.contains_key("drip_alarm") {
        e.data
            .insert("drip_alarm".into(), f64::from(20 + game.rng.range(381)));
    }
    let alarm = e.value("drip_alarm") - 1.0;
    if alarm <= 0.0 {
        game.world
            .spawn("oDrip", Vec2::new(e.pos.x + 8.0, e.pos.y + 4.0));
        e.data
            .insert("drip_alarm".into(), f64::from(20 + game.rng.range(381)));
    } else {
        e.data.insert("drip_alarm".into(), alarm);
    }
}

fn frozen_caveman(game: &mut Game, e: &mut Entity) {
    if point_family(game, e.pos, "oIce") {
        return;
    }
    e.alive = false;
    // Generated ice stores the frozen occupant on terrain metadata, whose
    // destruction adapter already owns its release. Explicit instances (e.g.
    // imported rooms) need the source Destroy event here instead.
    let adapted = game.world.entities.iter().any(|other| {
        other.value("frozen_caveman") != 0.0
            && (other.pos.x - e.pos.x).abs() < 16.0
            && (other.pos.y - e.pos.y).abs() < 16.0
    });
    if adapted
        || game.flags.contains("cleanSolids")
        || game.data.get("cleanSolids").copied().unwrap_or(0.0) != 0.0
    {
        return;
    }
    let id = game.world.spawn("oCaveman", e.pos);
    let caveman = game.world.entity_mut(id).unwrap();
    caveman.state = 98;
    caveman.timer = caveman.value("stun_time") as u32;
    caveman.data.insert("invincible".into(), 20.0);
}

fn effect(game: &mut Game, e: &mut Entity) {
    if e.value("world_effect_initialized") == 0.0 {
        match e.object.as_str() {
            "oSmokePuff" => e.vel.y = 0.1,
            "oBloodSpark" => e.vel.y = -f64::from(1 + game.rng.range(3)) * 0.2,
            "oBubble" => e.vel.y = -f64::from(1 + game.rng.range(3)) * 0.1,
            _ => {}
        }
        e.data.insert("world_effect_initialized".into(), 1.0);
    }
    let (speed, frames) = match e.object.as_str() {
        "oBloodSpark" => (0.5, 9.0),
        "oBloodTrail" => (0.8, 7.0),
        "oBubble" => (0.2, 9.0),
        "oPoof" => (0.4, 6.0),
        "oLavaDrip" => (0.4, 5.0),
        _ => (0.4, 8.0),
    };
    match e.object.as_str() {
        "oPoof" => {
            e.pos.x += e.vel.x;
            e.pos.y += e.vel.y;
        }
        "oSmokePuff" => e.pos.y -= e.vel.y,
        "oBloodSpark" | "oBubble" => e.pos.y += e.vel.y,
        _ => {}
    }
    e.animation += speed;
    if e.animation + 1e-9 >= frames
        || (e.object == "oBubble" && !point_family(game, e.pos, "oWater"))
    {
        e.alive = false;
    }
}

fn collect_effect(e: &mut Entity) {
    let items = e.object == "oItemsGet";
    if items {
        e.pos.y = (e.pos.y - 0.1).ceil();
        e.pos.x = e.pos.x.ceil();
        e.animation += 0.8;
    } else {
        e.pos.y -= 1.0;
        if e.object == "oSmallCollect" {
            e.pos.x = e.pos.x.ceil();
            e.pos.y = e.pos.y.ceil();
            e.animation += 0.4;
        }
    }
    if e.age >= if items { 40 } else { 30 } {
        e.alive = false;
    }
}

fn outside(game: &Game, p: Vec2, margin: f64) -> bool {
    p.x < game.camera.x - margin
        || p.x > game.camera.x + VIEW_WIDTH + margin
        || p.y < game.camera.y - margin
        || p.y > game.camera.y + VIEW_HEIGHT + margin
}

fn rubble(game: &Game, e: &mut Entity) {
    if e.value("world_effect_initialized") == 0.0 {
        if e.object == "oLeaf" {
            e.vel.y = 0.4;
        }
        e.data.insert("world_effect_initialized".into(), 1.0);
    }
    e.pos.x += e.vel.x;
    e.pos.y += e.vel.y;
    e.vel.y += if e.object == "oLeaf" { 0.01 } else { 0.6 };
    if e.object == "oLeaf" {
        e.animation = (e.animation + 0.2) % 10.0;
    }
    if point_family(game, e.pos, "oWaterSwim") {
        if e.object == "oDrip" {
            e.alive = false;
        } else if e.object == "oLeaf" {
            e.vel.y = 0.0;
            e.sprite = "sLeafStill".into();
        } else {
            e.vel.y = 0.2;
        }
    } else if point_family(game, e.pos, "oLava") {
        e.alive = false;
    }
    if game.world.solid_at(e.pos.x, e.pos.y) || outside(game, e.pos, 32.0) {
        e.alive = false;
    }
}

// moveTo uses reciprocal/modulo displacement, rather than accumulating fractions.
fn displacement(v: f64, tick: u64) -> i32 {
    let fraction = v.abs().fract();
    let extra = fraction != 0.0 && tick.is_multiple_of((1.0 / fraction).round().max(1.0) as u64);
    (v.abs().floor() as i32 + i32::from(extra)) * if v < 0.0 { -1 } else { 1 }
}

fn detritus(game: &mut Game, e: &mut Entity) {
    if e.value("world_effect_initialized") == 0.0 {
        e.vel = Vec2::new(
            game.rng.unit() * 4.0 - game.rng.unit() * 4.0,
            -1.0 - game.rng.unit() * 2.0,
        );
        e.data
            .entry("gravity".into())
            .or_insert_with(|| f64::from(1 + game.rng.range(6)) * 0.1);
        e.data.insert("life".into(), 60.0);
        e.data.insert("world_effect_initialized".into(), 1.0);
        for (key, value) in [
            ("collision_left", -4.0),
            ("collision_top", -4.0),
            ("collision_right", 4.0),
            ("collision_bottom", 4.0),
        ] {
            e.data.insert(key.into(), value);
        }
        if game
            .world
            .entities
            .iter()
            .filter(|other| other.alive && catalog::is_a(&other.object, "oDetritus"))
            .count()
            > 32
        {
            e.alive = false;
            return;
        }
    }
    let life = e.value("life");
    e.data.insert("life".into(), life - 1.0);
    if life <= 0.0 || outside(game, e.pos, 4.0) {
        e.alive = false;
        return;
    }
    let dx = displacement(e.vel.x, game.tick);
    let dy = displacement(e.vel.y, game.tick);
    for _ in 0..dx.unsigned_abs() {
        let next = Vec2::new(e.pos.x + f64::from(dx.signum()), e.pos.y);
        if game
            .world
            .collides_except(Rect::new(next.x - 4.0, next.y - 4.0, 8.0, 8.0), e.id)
        {
            break;
        }
        e.pos = next;
    }
    for _ in 0..dy.unsigned_abs() {
        let next = Vec2::new(e.pos.x, e.pos.y + f64::from(dy.signum()));
        if game
            .world
            .collides_except(Rect::new(next.x - 4.0, next.y - 4.0, 8.0, 8.0), e.id)
        {
            break;
        }
        e.pos = next;
    }
    if point_family(game, Vec2::new(e.pos.x, e.pos.y - 4.0), "oLava") {
        e.alive = false;
        return;
    }
    let bottom = game
        .world
        .collides_except(Rect::new(e.pos.x - 4.0, e.pos.y + 4.0, 8.0, 1.0), e.id);
    if e.vel.y < 6.0 {
        e.vel.y += e.value("gravity");
    }
    if e.vel.y < 0.0
        && game
            .world
            .collides_except(Rect::new(e.pos.x - 4.0, e.pos.y - 5.0, 8.0, 1.0), e.id)
    {
        e.vel.y *= -0.8;
    }
    if game
        .world
        .collides_except(Rect::new(e.pos.x - 5.0, e.pos.y - 4.0, 1.0, 8.0), e.id)
        || game
            .world
            .collides_except(Rect::new(e.pos.x + 4.0, e.pos.y - 4.0, 1.0, 8.0), e.id)
    {
        e.vel.x *= -0.5;
    }
    if bottom {
        e.vel.y = if e.vel.y > 1.0 { e.vel.y * -0.5 } else { 0.0 };
    }
    if e.object == "oBlood" {
        e.data.insert("collectible".into(), f64::from(e.age >= 5));
        if e.vel.y > 6.0 {
            e.alive = false;
        }
        if bottom && life > 20.0 {
            e.data.insert("life".into(), 20.0);
        }
        if e.age % 4 == 1
            && game
                .world
                .entities
                .iter()
                .filter(|other| other.alive && other.object == "oBloodTrail")
                .count()
                < 12
        {
            game.world.spawn("oBloodTrail", e.pos);
        }
        if e.age >= 5 && game.player.has("Kapala") && e.bounds().intersects(game.player.bounds()) {
            let blood = game.data.get("bloodLevel").copied().unwrap_or(0.0) + 1.0;
            game.data
                .insert("bloodLevel".into(), if blood > 8.0 { 0.0 } else { blood });
            game.world.spawn("oBloodSpark", e.pos);
            if blood > 8.0 {
                game.player.health += 1;
                game.world.spawn(
                    "oHeart",
                    Vec2::new(game.player.pos.x, game.player.pos.y - 8.0),
                );
                game.sound("sndKiss", game.player.pos);
            }
            e.alive = false;
        }
        e.animation = (e.animation + 0.3) % 3.0;
    } else {
        if bottom && e.value("dying") == 0.0 {
            e.sprite = "sSmokePuff".into();
            e.animation = 0.0;
            e.data.insert("dying".into(), 1.0);
        }
        e.animation += 0.3;
        if e.value("dying") != 0.0 && e.animation + 1e-9 >= 8.0 {
            e.alive = false;
        } else if e.value("dying") == 0.0 {
            e.animation %= 8.0;
        }
    }
}

fn fallen_decoration(game: &mut Game, e: &mut Entity) {
    if e.object == "oFakeBones" && outside(game, e.pos, 16.0) {
        return;
    }
    if !game.world.solid_at(e.pos.x + 8.0, e.pos.y + 16.0) {
        e.pos.y += e.vel.y;
        e.vel.y += 0.2;
    }
    if game.world.solid_at(e.pos.x + 8.0, e.pos.y + 15.0) {
        e.pos.y -= 1.0;
    }
    if e.object == "oFakeBones" {
        if (game.player.pos.y - (e.pos.y + 8.0)).abs() < 8.0
            && (game.player.pos.x - (e.pos.x + 8.0)).abs() < 64.0
        {
            e.sprite = "sSkeletonCreateL".into();
        }
        if e.sprite == "sSkeletonCreateL" {
            e.animation += 1.0;
            if e.animation >= 6.0 {
                game.world.spawn("oSkeleton", e.pos);
                e.alive = false;
            }
        }
    }
}

fn parachute(game: &mut Game) {
    let open = game
        .player
        .data
        .get("parachute_open")
        .copied()
        .unwrap_or(0.0)
        != 0.0
        && game.player.health > 0
        && game.player.stun == 0;
    let existing: Vec<_> = game
        .world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oParachute")
        .map(|e| e.id)
        .collect();
    if open && existing.is_empty() {
        game.world.spawn(
            "oParachute",
            Vec2::new(game.player.pos.x - 8.0, game.player.pos.y - 16.0),
        );
    }
    if !open {
        for id in existing {
            if let Some(e) = game.world.entity_mut(id) {
                e.alive = false;
            }
            game.world.spawn(
                "oParaUsed",
                Vec2::new(game.player.pos.x - 8.0, game.player.pos.y - 24.0),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Area, RulesPreset, World};
    fn room() -> Game {
        let mut g = Game::shell(1, RulesPreset::Classic);
        g.world = World::new(20, 15, Area::Mines, 1);
        g.player.pos = Vec2::new(40.0, 40.0);
        g
    }
    fn advance(g: &mut Game) {
        g.tick += 1;
        for e in &mut g.world.entities {
            e.age += 1;
        }
        g.events.clear();
        tick(g, &InputFrame::default());
    }
    #[test]
    fn sign_preserves_both_metadata_lines_and_bounds_and_cooldown() {
        let mut g = room();
        let id = g.world.spawn("oMsgSign", Vec2::new(32.0, 32.0));
        let sign = g.world.entity_mut(id).unwrap();
        sign.text.insert("message".into(), "FIRST".into());
        sign.text.insert("message2".into(), "SECOND".into());
        advance(&mut g);
        assert_eq!(g.events, vec![GameEvent::Message("FIRST\nSECOND".into())]);
        for _ in 0..199 {
            advance(&mut g);
            assert!(g.events.is_empty());
        }
        assert_eq!(g.data["message_timer"], 200.0);
        advance(&mut g);
        assert_eq!(g.events.len(), 1);
        g.player.pos.x = 80.0;
        for _ in 0..200 {
            advance(&mut g);
        }
        assert!(g.events.is_empty());
        assert_eq!(g.data["message_timer"], 0.0);
    }
    #[test]
    fn source_lights_have_correct_ranges_origins_and_animation() {
        let mut g = room();
        for name in ["oLamp", "oLampRed", "oTikiTorch"] {
            g.world.spawn(name, Vec2::new(80.0, 80.0));
        }
        advance(&mut g);
        for e in &g.world.entities {
            assert_eq!(e.animation, 0.5);
            assert_eq!(e.value("light_offset_x"), 8.0);
            assert_eq!(e.value("light_offset_y"), 8.0);
            assert_eq!(
                e.value("light_radius"),
                match e.object.as_str() {
                    "oTikiTorch" => 32.0,
                    "oLampRed" => 0.0,
                    _ => 96.0,
                }
            );
        }
        assert_eq!(g.world.entities[1].value("red_light_range"), 96.0);
    }
    #[test]
    fn source_one_shot_animation_lifetimes() {
        for (object, lifetime) in [
            ("oPoof", 15),
            ("oSmokePuff", 20),
            ("oBloodTrail", 9),
            ("oBloodSpark", 18),
            ("oLavaDrip", 13),
        ] {
            let mut g = room();
            let id = g.world.spawn(object, Vec2::new(80.0, 80.0));
            for _ in 1..lifetime {
                advance(&mut g);
                assert!(g.world.entity(id).is_some(), "{object}");
            }
            advance(&mut g);
            assert!(g.world.entity(id).is_none(), "{object}");
        }
    }
    #[test]
    fn ice_trim_drips_and_requires_ceiling_support() {
        let mut g = room();
        g.world.set_tile(5, 3, "oIce");
        let id = g.world.spawn("oIceBottom", Vec2::new(80.0, 64.0));
        g.world
            .entity_mut(id)
            .unwrap()
            .data
            .insert("drip_alarm".into(), 1.0);
        advance(&mut g);
        assert!(g.world.entities.iter().any(|e| e.object == "oDrip"));
        g.world.set_tile(5, 3, "");
        advance(&mut g);
        assert!(g.world.entity(id).is_none());
    }
    #[test]
    fn explicit_frozen_caveman_releases_once_with_source_stun() {
        let mut g = room();
        g.world.set_tile(5, 4, "oIce");
        let frozen = g.world.spawn("oFrozenCaveman", Vec2::new(80.0, 64.0));
        advance(&mut g);
        assert!(g.world.entity(frozen).is_some());
        g.world.set_tile(5, 4, "");
        advance(&mut g);
        advance(&mut g);
        let released: Vec<_> = g
            .world
            .entities
            .iter()
            .filter(|e| e.alive && e.object == "oCaveman")
            .collect();
        assert_eq!(released.len(), 1);
        assert_eq!(released[0].state, 98);
        assert_eq!(released[0].timer, 200);
        assert_eq!(released[0].value("invincible"), 20.0);
    }
    #[test]
    fn fake_bones_rise_once_and_make_a_real_skeleton() {
        let mut g = room();
        g.player.pos = Vec2::new(40.0, 40.0);
        let id = g.world.spawn("oFakeBones", Vec2::new(32.0, 32.0));
        for _ in 0..6 {
            advance(&mut g);
        }
        assert!(g.world.entity(id).is_none());
        assert_eq!(
            g.world
                .entities
                .iter()
                .filter(|e| e.object == "oSkeleton")
                .count(),
            1
        );
    }
    #[test]
    fn parachute_visual_follows_opens_and_becomes_used() {
        let mut g = room();
        g.player.data.insert("parachute_open".into(), 1.0);
        for _ in 0..5 {
            advance(&mut g);
        }
        let e = g
            .world
            .entities
            .iter()
            .find(|e| e.alive && e.object == "oParachute")
            .unwrap();
        assert_eq!(e.sprite, "sParachute");
        assert_eq!(e.pos, Vec2::new(32.0, 24.0));
        g.player.data.insert("parachute_open".into(), 0.0);
        advance(&mut g);
        assert!(
            !g.world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oParachute")
        );
        assert!(
            g.world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oParaUsed")
        );
    }
    #[test]
    fn moving_item_cuts_parachute_with_source_direction_condition() {
        let mut g = room();
        g.player.data.insert("parachute_open".into(), 1.0);
        let rock = g.world.spawn("oRock", Vec2::new(40.0, 28.0));
        g.world.entity_mut(rock).unwrap().vel.x = -1.0;
        advance(&mut g);
        assert!(
            g.world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oParachute")
        );
        g.world.entity_mut(rock).unwrap().vel.x = 1.0;
        advance(&mut g);
        assert!(
            !g.world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oParachute")
        );
        assert_eq!(g.player.data["parachute_open"], 0.0);
    }
}
