//! The oEnemy Step action runs before the child's Step, only for objects whose
//! event explicitly calls action_inherited. Bat/UFO/boss overrides are deliberate.
use super::*;

fn calls_parent(object: &str) -> bool {
    matches!(
        object,
        "oSnake"
            | "oSpider"
            | "oCaveman"
            | "oFrog"
            | "oFireFrog"
            | "oMonkey"
            | "oManTrap"
            | "oYeti"
            | "oYetiKing"
            | "oAlien"
            | "oHawkman"
            | "oTombLord"
            | "oSkeleton"
            | "oZombie"
            | "oVampire"
            | "oShopkeeper"
            | "oShopkeeper2"
    )
}

pub(super) fn point(world: &World, p: Vec2, family: &str) -> bool {
    world.tiles_point(p, family)
        || world.entities.iter().any(|other| {
            other.alive
                && crate::catalog::is_a(&other.object, family)
                && other.sprite_bounds().contains(p)
        })
}

pub(super) fn gravity(e: &Entity, fallback: f64) -> f64 {
    e.data.get("inherited_gravity").copied().unwrap_or(fallback)
}

pub(super) fn tick(game: &mut Game, e: &mut Entity) {
    if !calls_parent(&e.object)
        || e.pos.x <= game.camera.x - 20.0
        || e.pos.x >= game.camera.x + VIEW_WIDTH + 4.0
        || e.pos.y <= game.camera.y - 20.0
        || e.pos.y >= game.camera.y + VIEW_HEIGHT + 4.0
    {
        return;
    }
    e.data.remove("inherited_gravity");
    if e.held {
        e.vel = Vec2::default();
        e.pos = Vec2::new(
            game.player.pos.x + if game.player.facing < 0 { -12.0 } else { -4.0 },
            game.player.pos.y
                - if game.player.state == PlayerState::Ducking && game.player.vel.x.abs() < 2.0 {
                    10.0
                } else {
                    12.0
                },
        );
        e.facing = game.player.facing;
        if let Some(facing) = e.data.get_mut("source_facing") {
            *facing = f64::from(game.player.facing > 0);
        }
        if game.player.held.is_none() || e.state < 98 {
            e.held = false;
        }
        if e.held {
            let sprite = match (e.object.as_str(), e.health > 0) {
                ("oCaveman", true) => Some("sCavemanHeldL"),
                ("oCaveman", false) => Some("sCavemanDHeldL"),
                ("oYeti", true) => Some("sYetiHeldL"),
                ("oYeti", false) => Some("sYetiDHeldL"),
                ("oHawkman", true) => Some("sHawkHeldL"),
                ("oHawkman", false) => Some("sHawkDHeldL"),
                ("oShopkeeper" | "oShopkeeper2", true) => Some("sShopHeldL"),
                ("oShopkeeper" | "oShopkeeper2", false) => Some("sShopDHeldL"),
                _ => None,
            };
            if let Some(sprite) = sprite {
                e.set_sprite(sprite);
            }
        }
    }
    // These inherited families use 16px sprites, except the two 32px kings.
    let dimensions: (f64, f64) = if matches!(e.object.as_str(), "oYetiKing" | "oTombLord") {
        (32.0, 32.0)
    } else {
        (16.0, 16.0)
    };
    e.data
        .insert("depth".into(), if e.held { 1.0 } else { 60.0 });
    let center = Vec2::new(
        e.pos.x + (dimensions.0 / 2.0).floor(),
        e.pos.y + (dimensions.1 / 2.0).floor(),
    );
    let swimming = point(&game.world, center, "oWaterSwim");
    if swimming && e.value("swimming") == 0.0 {
        game.world.spawn("oSplash", Vec2::new(center.x, e.pos.y));
        game.sound("sndSplash", e.pos);
    }
    e.data.insert("swimming".into(), f64::from(swimming));
    if swimming && e.object == "oFireFrog" {
        let id = game.spawn("oFrog", e.pos);
        if let Some(frog) = game.world.entity_mut(id) {
            frog.state = e.state;
        }
        e.alive = false;
        // Source instance_destroy keeps executing this inherited event,
        // including its burning roll on the receiver being transformed.
    }
    let normal = if matches!(
        e.object.as_str(),
        "oFrog" | "oFireFrog" | "oSpider" | "oZombie" | "oVampire"
    ) {
        0.2
    } else {
        0.6
    };
    // Alien Step adds a literal 0.6, rather than the inherited myGrav.
    if e.object != "oAlien" {
        e.data.insert(
            "inherited_gravity".into(),
            if swimming { 0.2 } else { normal },
        );
    }
    if e.value("burning") > 0.0 {
        if game.rng.chance(1, 5) {
            // Source sprite_width/height are the current image canvas, not
            // collision bounds, and rand's upper endpoints are inclusive.
            let (width, height) = crate::catalog::sprite_dimensions(&e.sprite)
                .map_or(dimensions, |(w, h)| (f64::from(w), f64::from(h)));
            let pos = Vec2::new(
                e.pos.x + f64::from(game.rng.range(width as u32 + 1)),
                e.pos.y + f64::from(game.rng.range(height as u32 + 1)),
            );
            game.spawn("oBurn", pos);
        }
        e.data.insert("burning".into(), e.value("burning") - 1.0);
    }
    if point(&game.world, Vec2::new(center.x, e.pos.y - 1.0), "oLava") {
        e.alive = false;
        // Source instance_destroy does not return from the current event.
        // Later spear/spike statements still execute on this receiver.
    }
    if point(
        &game.world,
        Vec2::new(center.x, e.pos.y + dimensions.1 - 2.0),
        "oLava",
    ) {
        e.health = 0;
        set_counts_as_kill(e, false);
        e.data.insert("burning".into(), 1.0);
        e.data.insert("depth".into(), 999.0);
        e.data.insert("inherited_gravity".into(), 0.0);
        e.vel = Vec2::new(0.0, 0.1);
    }
    let spear_area = Rect::new(e.pos.x + 2.0, e.pos.y + 2.0, 13.0, 13.0);
    let spear = game
        .world
        .entities
        .iter()
        .filter(|other| {
            other.alive
                && crate::catalog::is_a(&other.object, "oSpearsLeft")
                && other.sprite_bounds().intersects(spear_area)
        })
        .min_by(|a, b| a.pos.distance(e.pos).total_cmp(&b.pos.distance(e.pos)))
        .map(|other| (other.pos, other.animation));
    if let Some((pos, _)) = spear.filter(|(_, frame)| *frame >= 20.0 && *frame < 24.0) {
        let stunned_body = matches!(
            e.object.as_str(),
            "oCaveman" | "oManTrap" | "oYeti" | "oHawkman" | "oShopkeeper" | "oShopkeeper2"
        );
        if !stunned_body || e.health > 0 {
            e.health -= 2;
            set_counts_as_kill(e, false);
            if stunned_body {
                e.state = 98;
                e.timer = e.value("stun_time") as u32;
                e.vel = Vec2::new(if pos.x < e.pos.x { 4.0 } else { -4.0 }, -6.0);
                e.data.insert("animation_speed".into(), 0.5);
            }
            game.sound("sndHit", e.pos);
            create_blood(
                game,
                e,
                blood_center(e, dimensions),
                if stunned_body { 2 } else { 1 },
            );
        }
    }
    if e.vel.y > 2.0
        && point(
            &game.world,
            Vec2::new(e.pos.x + 8.0, e.pos.y + 16.0),
            "oSpikes",
        )
    {
        if e.value("bloodless") == 0.0 {
            stain_spike(game, e);
        }
        if e.health > 0 {
            e.health = 0;
            set_counts_as_kill(e, false);
            create_blood(game, e, blood_center(e, dimensions), 3);
            if matches!(
                e.object.as_str(),
                "oCaveman" | "oManTrap" | "oYeti" | "oHawkman" | "oShopkeeper" | "oShopkeeper2"
            ) {
                e.state = 99;
            }
        }
        e.vel = Vec2::new(0.0, 0.2);
        e.data.insert("inherited_gravity".into(), 0.0);
    }
    crate::traps::altar::offering_entity(game, e);
    if e.state == 98 && (e.vel.x.abs() > 2.0 || e.vel.y.abs() > 2.0) {
        body_projectile(game, e);
    }
}

fn blood_center(e: &Entity, fallback: (f64, f64)) -> Vec2 {
    // sprite_width/height are the image canvas, not its collision rectangle.
    let (width, height) = crate::catalog::sprite_dimensions(&e.sprite)
        .map_or(fallback, |(width, height)| {
            (f64::from(width), f64::from(height))
        });
    Vec2::new(e.pos.x + width / 2.0, e.pos.y + height / 2.0)
}

fn stain_spike(game: &mut Game, e: &Entity) {
    // instance_place translates the receiver to the requested origin. The
    // preceding feet point only gates this separate first-instance query.
    let mut probe = e.clone();
    probe.pos = Vec2::new(e.pos.x + 8.0, e.pos.y + 14.0);
    crate::dispatcher::register_instances(game);
    let spike = crate::dispatcher::descendant_instances(game, "oSpikes")
        .into_iter()
        .find(|instance| match *instance {
            Instance::Entity(id) => game
                .world
                .entity(id)
                .is_some_and(|spike| spike.engine_active && probe.event_intersects(spike)),
            Instance::Terrain(index) => {
                let x = (index % game.world.width) as i32;
                let y = (index / game.world.width) as i32;
                game.world.tiles[index].engine_active
                    && probe
                        .collision_mask()
                        .intersects_mask(&game.world.tile_collision_mask(x, y))
            }
            Instance::Player => false,
        });
    match spike {
        Some(Instance::Entity(id)) => {
            if let Some(spike) = game.world.entity_mut(id) {
                spike.set_sprite("sSpikesBlood");
            }
        }
        Some(Instance::Terrain(index)) => {
            let origin = Instance::Terrain(index).position(game);
            let center = Vec2::new(origin.x + 8.0, origin.y + 8.0);
            let id = game
                .world
                .entities
                .iter()
                .find(|metadata| {
                    metadata.alive
                        && metadata.object == "oTerrainMetadata"
                        && (metadata.pos.x / 16.0).floor() == (origin.x / 16.0).floor()
                        && (metadata.pos.y / 16.0).floor() == (origin.y / 16.0).floor()
                })
                .map(|metadata| metadata.id)
                .unwrap_or_else(|| game.world.spawn("oTerrainMetadata", center));
            let metadata = game.world.entity_mut(id).unwrap();
            let mut sprite = metadata
                .text
                .get("terrain_sprite")
                .cloned()
                .unwrap_or_default();
            crate::animation::assign(&mut sprite, &mut metadata.animation, "sSpikesBlood");
            metadata.text.insert("terrain_sprite".into(), sprite);
        }
        _ => {}
    }
}

fn body_projectile(game: &mut Game, e: &Entity) {
    let area = Rect::new(e.pos.x, e.pos.y, 17.0, 17.0);
    let Some(index) = game.world.entities.iter().position(|other| {
        other.alive
            && other.id != e.id
            && is_enemy(&other.object)
            && area.intersects(other.sprite_bounds())
    }) else {
        return;
    };
    let mut hit = false;
    let mut explosion = None;
    let target = &mut game.world.entities[index];
    if target.value("invincible") > 0.0 || target.object == "oMagmaMan" {
        return;
    }
    if target.state < 98 {
        target.vel.x = e.vel.x;
    }
    match target.object.as_str() {
        "oCaveman" | "oManTrap" | "oVampire" | "oYeti" | "oHawkman" if target.state < 98 => {
            target.health -= 1;
            target.state = 98;
            target.timer = target.value("stun_time") as u32;
            target.vel.y = -6.0;
            hit = true;
        }
        "oShopkeeper" | "oShopkeeper2" if target.state < 98 => {
            target.health -= 1;
            target.state = 2;
            target.vel.y = -6.0;
            hit = true;
        }
        "oGiantSpider" | "oTombLord" if target.value("hit_cooldown") == 0.0 => {
            target.health -= 1;
            target.data.insert(
                "hit_cooldown".into(),
                if target.object == "oTombLord" {
                    20.0
                } else {
                    10.0
                },
            );
            hit = true;
        }
        "oAlienBoss" if target.state != 99 && target.sprite != "sAlienBossHurt" => {
            target.health -= 1;
            target.set_sprite("sAlienBossHurt");
            target.data.insert("animation_speed".into(), 0.8);
            hit = true;
        }
        "oUFO" => {
            explosion = Some(Vec2::new(target.pos.x + 8.0, target.pos.y + 8.0));
            target.alive = false;
            target.health = 0;
            // This source oEnemy Step branch updates run totals only;
            // unlike oItem's UFO branch it has no enemyKills[16] write.
            target.data.insert("death_recorded".into(), 1.);
            *game.data.entry("ufos".into()).or_default() += 1.;
            *game.data.entry("kills".into()).or_default() += 1.;
            hit = true;
        }
        "oCaveman" | "oManTrap" | "oVampire" | "oYeti" | "oHawkman" | "oShopkeeper"
        | "oShopkeeper2" | "oGiantSpider" | "oTombLord" | "oAlienBoss" => {}
        _ => {
            target.health -= 1;
            hit = true;
        }
    }
    if let Some(pos) = explosion {
        game.spawn("oExplosion", pos);
        if game.rng.range(3) == 0 {
            game.spawn("oAlienEject", pos);
        }
        game.sound("sndExplosion", pos);
    }
    if hit {
        game.sound("sndHit", e.pos);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn room() -> Game {
        let mut g = Game::shell(123, RulesPreset::Classic);
        g.world = World::new(40, 30, Area::Mines, 1);
        g.player.pos = Vec2::new(240.0, 160.0);
        g.player.previous_pos = g.player.pos;
        g
    }

    fn live_blood(game: &Game) -> Vec<&Entity> {
        game.world
            .entities
            .iter()
            .filter(|e| e.alive && e.object == "oBlood")
            .collect()
    }

    fn assert_blood_create(drop: &Entity) {
        assert!(drop.engine_active);
        assert_eq!(drop.sprite, "sBlood");
        assert_eq!(drop.value("world_effect_initialized"), 1.0);
        assert_eq!(drop.value("collectible"), 0.0);
        assert_eq!(drop.image_speed(), 0.3);
        assert_eq!(drop.alarms, [1, 1, 5, -1, -1, -1, -1, -1, -1, -1, -1, -1]);
    }

    fn naturally_extended_spear(rules: RulesPreset) -> Game {
        let mut game = Game::shell(71, rules);
        for x in 0..game.world.width {
            game.world.set_tile(x, 10, "oBlock");
        }
        game.player.pos = Vec2::new(79.0, 152.0);
        game.player.previous_pos = game.player.pos;
        game.spawn("oSpearTrapTop", Vec2::new(96.0, 144.0));
        for _ in 0..20 {
            game.step(&InputFrame::default());
            assert_eq!(game.player.health, 4);
        }
        let spear = game
            .world
            .entities
            .iter()
            .find(|e| e.alive && e.object == "oSpearsLeft")
            .expect("the real supported trap must extend its spear");
        assert_eq!(spear.pos, Vec2::new(80.0, 144.0));
        assert_eq!(spear.animation, 20.0);
        game
    }

    #[test]
    fn natural_spear_hit_emits_parent_blood_and_preserves_child_death_burst() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            for (object, expected_drops) in [("oCaveman", 2), ("oSnake", 4)] {
                let mut game = naturally_extended_spear(rules);
                // Public Create into a naturally extended hazard; no altered
                // actor HP/state/velocity or trap animation is required.
                let actor = game.spawn(object, Vec2::new(80.0, 144.0));
                game.step(&InputFrame::default());
                assert_eq!(live_blood(&game).len(), expected_drops, "{object}");
                assert_eq!(game.data.get("kills").copied().unwrap_or(0.0), 0.0);
                if object == "oCaveman" {
                    let actor = game.world.entity(actor).unwrap();
                    assert_eq!(actor.health, 1);
                    assert_eq!(actor.state, 98);
                } else {
                    assert!(game.world.entity(actor).is_none());
                }
                // Caveman runs before oBlood's resource; Snake after it.
                // previous_pos retains the newborn origin in either case.
                let drops = live_blood(&game);
                for drop in drops.iter().take(if object == "oCaveman" { 2 } else { 1 }) {
                    assert_eq!(drop.previous_pos, Vec2::new(88.0, 152.0));
                    assert_blood_create(drop);
                }
                if object == "oSnake" {
                    for drop in &drops[1..] {
                        // Source moveTo proposes three pixels, but the real
                        // oSolid spear trap at x96 stops the third: Snake's
                        // right edge x+14 reaches96 at origin82. Child blood
                        // therefore uses82+8, after the parent's center88.
                        assert_eq!(drop.pos, Vec2::new(90.0, 152.0));
                        assert_blood_create(drop);
                    }
                }
            }
        }
    }

    #[test]
    fn natural_spike_fall_emits_fresh_blood_and_stains_the_selected_spike() {
        for (object, expected_drops) in [("oCaveman", 3), ("oSnake", 6), ("oSkeleton", 0)] {
            let mut game = room();
            for x in 0..game.world.width {
                game.world.set_tile(x, 8, "oBlock");
            }
            game.player.pos = Vec2::new(240.0, 120.0);
            game.player.previous_pos = game.player.pos;
            for x in (64..=144).step_by(16) {
                game.spawn("oSpikes", Vec2::new(f64::from(x), 112.0));
            }
            let actor = game.spawn(object, Vec2::new(96.0, 64.0));
            let mut impact_origin = None;
            for _ in 0..60 {
                let before = game.world.entity(actor).unwrap().pos;
                game.step(&InputFrame::default());
                if game.world.entity(actor).is_none_or(|e| e.health <= 0) {
                    impact_origin = Some(before);
                    break;
                }
                assert!(
                    live_blood(&game).is_empty(),
                    "{object} bled before the spike hit"
                );
            }
            let before = impact_origin.expect("ordinary gravity must reach the spikes");
            let drops = live_blood(&game);
            assert_eq!(drops.len(), expected_drops, "{object}");
            for drop in drops.iter().take(3) {
                assert_eq!(drop.previous_pos, Vec2::new(before.x + 8.0, before.y + 8.0));
                assert_blood_create(drop);
            }
            let stained = game
                .world
                .entities
                .iter()
                .filter(|e| e.alive && e.object == "oSpikes" && e.sprite == "sSpikesBlood")
                .count();
            assert_eq!(stained, usize::from(object != "oSkeleton"), "{object}");
            assert_eq!(game.data.get("kills").copied().unwrap_or(0.0), 0.0);
        }
    }

    #[test]
    fn natural_tile_spike_fall_preserves_stain_mask_and_source_order_next_tick() {
        for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
            let mut game = Game::shell(123, rules);
            game.world = World::new(40, 30, Area::Mines, 1);
            for x in 0..game.world.width {
                game.world.set_tile(x, 8, "oBlock");
            }
            game.player.pos = Vec2::new(240.0, 120.0);
            game.player.previous_pos = game.player.pos;
            // Source Create launches a Caveman with xVel=2.5. A legal narrow
            // corridor constrains its ordinary drifting/turning AI before the
            // fall, without changing any actor state after Create. Custom
            // bounds [2,0,14,16] keep its origin in x94..98 and feet in tile6.
            for x in [5, 7] {
                for y in 4..=6 {
                    game.world.set_tile(x, y, "oBlock");
                }
            }
            // The translated receiver overlaps both neighboring spikes. Source
            // instance_place selects the first created, which is deliberately
            // different from the tile containing the feet point.
            for x in [7, 6, 4, 5, 8, 9] {
                game.world.set_tile(x, 7, "oSpikes");
            }
            let right = Instance::Terrain(7 * game.world.width + 7);
            let underfoot = Instance::Terrain(7 * game.world.width + 6);
            let source_order = vec![
                right,
                underfoot,
                Instance::Terrain(7 * game.world.width + 4),
                Instance::Terrain(7 * game.world.width + 5),
                Instance::Terrain(7 * game.world.width + 8),
                Instance::Terrain(7 * game.world.width + 9),
            ];
            let original_serial = game.world.tile(7, 7).unwrap().creation_serial;
            let actor = game.spawn("oCaveman", Vec2::new(96.0, 64.0));
            let mut impact = false;
            for _ in 0..60 {
                let before = game.world.entity(actor).unwrap().clone();
                let feet = Vec2::new(before.pos.x + 8.0, before.pos.y + 16.0);
                let entering_spikes = before.vel.y > 2.0 && game.world.tiles_point(feet, "oSpikes");
                if entering_spikes {
                    assert!(game.world.tile_bounds(6, 7).contains(feet));
                    assert!(!game.world.tile_bounds(7, 7).contains(feet));
                    let mut translated = before.clone();
                    translated.pos = Vec2::new(before.pos.x + 8.0, before.pos.y + 14.0);
                    assert!(
                        translated
                            .collision_mask()
                            .intersects_mask(&game.world.tile_collision_mask(7, 7))
                    );
                    assert_eq!(
                        crate::dispatcher::descendant_instances(&game, "oSpikes"),
                        source_order
                    );
                }
                game.step(&InputFrame::default());
                if game.world.entity(actor).is_some_and(|e| e.health <= 0) {
                    assert!(
                        entering_spikes,
                        "death must come from the ordinary falling spike gate"
                    );
                    impact = true;
                    break;
                }
                assert!(live_blood(&game).is_empty());
            }
            assert!(impact, "ordinary gravity must reach the tile-backed spikes");
            assert_eq!(live_blood(&game).len(), 3);
            assert_eq!(
                crate::dispatcher::descendant_instances(&game, "oSpikes"),
                source_order
            );
            assert_eq!(game.world.tile_bounds(7, 7).y, 116.0);
            assert_eq!(game.world.tile_bounds(6, 7).y, 115.0);
            let metadata_id = game
                .world
                .entities
                .iter()
                .find(|e| {
                    e.alive && e.object == "oTerrainMetadata" && e.pos == Vec2::new(120.0, 120.0)
                })
                .expect("running parent event must persist the selected terrain sprite")
                .id;
            assert_eq!(
                game.world
                    .entity(metadata_id)
                    .unwrap()
                    .text
                    .get("terrain_sprite")
                    .map(String::as_str),
                Some("sSpikesBlood")
            );
            game.step(&InputFrame::default());
            assert_eq!(
                game.world
                    .entity(metadata_id)
                    .unwrap()
                    .text
                    .get("terrain_sprite")
                    .map(String::as_str),
                Some("sSpikesBlood")
            );
            assert_eq!(game.world.tile_bounds(7, 7).y, 116.0);
            assert_eq!(game.world.tile_bounds(6, 7).y, 115.0);
            assert_eq!(game.world.tile(7, 7).unwrap().object, "oSpikes");
            assert_eq!(
                game.world.tile(7, 7).unwrap().creation_serial,
                original_serial
            );
            assert_eq!(
                crate::dispatcher::descendant_instances(&game, "oSpikes"),
                source_order
            );
            assert_eq!(
                live_blood(&game).len(),
                3,
                "a corpse must not repeat the fresh-death burst"
            );
        }
    }

    #[test]
    fn actual_step_upper_lava_destruction_continues_parent_spear_statements() {
        let mut game = naturally_extended_spear(RulesPreset::Classic);
        game.world.set_tile(5, 8, "oLava");
        let actor = game.spawn("oCaveman", Vec2::new(80.0, 144.0));
        game.step(&InputFrame::default());
        assert!(game.world.entity(actor).is_none());
        assert_eq!(live_blood(&game).len(), 2);
        assert!(
            game.events.iter().any(|event| {
                matches!(event, GameEvent::Sound { name, .. } if name == "sndHit")
            })
        );
    }

    #[test]
    fn parent_spear_script_obeys_per_drop_cap_bloodless_health_and_rng() {
        for object in [
            "oCaveman",
            "oManTrap",
            "oYeti",
            "oHawkman",
            "oShopkeeper",
            "oShopkeeper2",
            "oSnake",
        ] {
            for (detritus, bloodless, health) in [
                (0, false, 3),
                (15, false, 3),
                (16, false, 3),
                (0, true, 3),
                (0, false, 0),
            ] {
                let mut game = room();
                let spear = game.spawn("oSpearsLeft", Vec2::new(96.0, 96.0));
                game.world.entity_mut(spear).unwrap().animation = 20.0;
                let actor_id = game.spawn(object, Vec2::new(96.0, 96.0));
                let mut actor = game.world.entity(actor_id).unwrap().clone();
                actor.health = health;
                actor.data.insert("bloodless".into(), f64::from(bloodless));
                for _ in 0..detritus {
                    game.spawn("oBone", Vec2::new(280.0, 80.0));
                }
                let mut expected_rng = game.rng.clone();
                tick(&mut game, &mut actor);
                let body = object != "oSnake";
                let requested = if bloodless || (body && health == 0) {
                    0
                } else if body {
                    2
                } else {
                    1
                };
                let expected_count = requested.min(16 - detritus);
                let drops = live_blood(&game);
                assert_eq!(
                    drops.len(),
                    expected_count,
                    "{object}, cap={detritus}, bloodless={bloodless}, hp={health}"
                );
                for drop in drops {
                    assert_blood_create(drop);
                    assert_eq!(drop.pos, Vec2::new(104.0, 104.0));
                    assert_eq!(
                        drop.vel,
                        Vec2::new(
                            expected_rng.unit() * 4.0 - expected_rng.unit() * 4.0,
                            -1.0 - expected_rng.unit() * 2.0
                        )
                    );
                    assert_eq!(
                        drop.value("gravity"),
                        f64::from(1 + expected_rng.range(6)) * 0.1
                    );
                }
                assert_eq!(
                    game.rng.next_u64(),
                    expected_rng.next_u64(),
                    "{object}: suppressed attempts must not consume RNG"
                );
            }
        }
    }

    #[test]
    fn parent_spike_stain_uses_translated_receiver_first_instance_even_for_a_corpse() {
        for terrain in [false, true] {
            for (health, bloodless, expected_drops) in [(3, false, 3), (0, false, 0), (3, true, 0)]
            {
                let mut game = room();
                let right = if terrain {
                    game.world.set_tile(7, 7, "oSpikes");
                    game.world.set_tile(6, 7, "oSpikes");
                    None
                } else {
                    let right = game.spawn("oSpikes", Vec2::new(112.0, 112.0));
                    game.spawn("oSpikes", Vec2::new(96.0, 112.0));
                    Some(right)
                };
                let id = game.spawn("oCaveman", Vec2::new(96.0, 99.0));
                let mut actor = game.world.entity(id).unwrap().clone();
                actor.health = health;
                actor.vel.y = 3.0;
                actor.data.insert("bloodless".into(), f64::from(bloodless));
                tick(&mut game, &mut actor);
                assert_eq!(live_blood(&game).len(), expected_drops);
                if let Some(right) = right {
                    assert_eq!(
                        game.world.entity(right).unwrap().sprite,
                        if bloodless { "sSpikes" } else { "sSpikesBlood" }
                    );
                    assert!(game.world.entities.iter().any(|spike| {
                        spike.object == "oSpikes"
                            && spike.pos == Vec2::new(96.0, 112.0)
                            && spike.sprite == "sSpikes"
                    }));
                } else {
                    assert_eq!(
                        game.world.tile_bounds(7, 7).y,
                        if bloodless { 115.0 } else { 116.0 }
                    );
                    assert_eq!(game.world.tile_bounds(6, 7).y, 115.0);
                }
            }
        }
    }

    #[test]
    fn parent_environment_blood_center_uses_full_current_sprite_canvas() {
        for (object, sprite, center) in [
            ("oYeti", "sYetiStunL", Vec2::new(108.0, 104.0)),
            ("oHawkman", "sHawkStunL", Vec2::new(108.0, 104.0)),
            ("oManTrap", "sManTrapEatL", Vec2::new(112.0, 112.0)),
        ] {
            let mut game = room();
            let spear = game.spawn("oSpearsLeft", Vec2::new(96.0, 96.0));
            game.world.entity_mut(spear).unwrap().animation = 20.0;
            let id = game.spawn(object, Vec2::new(96.0, 96.0));
            let mut actor = game.world.entity(id).unwrap().clone();
            actor.set_sprite(sprite);
            tick(&mut game, &mut actor);
            assert_eq!(live_blood(&game).len(), 2, "{sprite}");
            assert!(
                live_blood(&game).iter().all(|drop| drop.pos == center),
                "{sprite}"
            );
        }
    }
    #[test]
    fn actual_step_fast_stunned_body_ufo_kill_has_run_credit_without_lifetime_row() {
        let mut game = room();
        let body = game.spawn("oCaveman", Vec2::new(80., 80.));
        let actor = game.world.entity_mut(body).unwrap();
        actor.state = 98;
        actor.timer = 20;
        actor.vel.x = 3.;
        let ufo = game.spawn("oUFO", Vec2::new(80., 80.));
        game.step(&InputFrame::default());
        assert!(game.world.entity(ufo).is_none());
        assert_eq!(game.data.get("ufos"), Some(&1.));
        assert_eq!(game.data.get("kills"), Some(&1.));
        assert_eq!(
            game.lifetime_stats.enemy_kills[crate::statistics::EnemyKill::Ufo as usize],
            0
        );
    }

    #[test]
    fn actual_step_inherited_lava_kills_without_player_kill_credit() {
        for object in [
            "oSnake",
            "oSpider",
            "oCaveman",
            "oFrog",
            "oMonkey",
            "oManTrap",
            "oYeti",
            "oYetiKing",
            "oAlien",
            "oHawkman",
            "oTombLord",
            "oSkeleton",
            "oZombie",
            "oVampire",
            "oShopkeeper",
        ] {
            let mut g = room();
            for y in 5..9 {
                for x in 5..9 {
                    g.world
                        .spawn("oLava", Vec2::new(f64::from(x * 16), f64::from(y * 16)));
                }
            }
            let id = g.world.spawn(object, Vec2::new(96.0, 96.0));
            g.step(&InputFrame::default());
            assert!(
                g.world.entity(id).is_none_or(|e| e.health <= 0),
                "{object} survived inherited lava"
            );
            assert_eq!(
                g.data.get("kills").copied().unwrap_or(0.0),
                0.0,
                "{object} credited lava kill"
            );
        }
    }
    #[test]
    fn actual_step_swimming_gravity_and_dry_monkey_match_parent_step() {
        for (object, wet, expected) in [
            ("oMonkey", false, 0.6),
            ("oMonkey", true, 0.2),
            ("oYeti", true, 0.2),
            ("oYetiKing", true, 0.2),
            ("oAlien", true, 0.6),
        ] {
            let mut g = room();
            if wet {
                for y in 5..9 {
                    for x in 5..9 {
                        g.world.spawn(
                            "oWaterSwim",
                            Vec2::new(f64::from(x * 16), f64::from(y * 16)),
                        );
                    }
                }
            }
            let id = g.world.spawn(object, Vec2::new(96.0, 96.0));
            let e = g.world.entity_mut(id).unwrap();
            e.vel = Vec2::default();
            e.state = 0;
            g.step(&InputFrame::default());
            let e = g.world.entity(id).unwrap();
            assert!(
                (e.vel.y - expected).abs() < 1e-9,
                "{object} wet={wet} vy={}",
                e.vel.y
            );
            assert_eq!(e.value("swimming"), f64::from(wet));
        }
    }
    #[test]
    fn actual_step_spikes_kill_descending_body_without_credit() {
        let mut g = room();
        g.world.spawn("oSpikes", Vec2::new(96.0, 112.0));
        let id = g.world.spawn("oCaveman", Vec2::new(96.0, 99.0));
        g.world.entity_mut(id).unwrap().vel = Vec2::new(0.0, 3.0);
        g.step(&InputFrame::default());
        assert!(g.world.entity(id).is_none_or(|e| e.health <= 0));
        assert_eq!(g.data.get("kills").copied().unwrap_or(0.0), 0.0);
    }
    #[test]
    fn actual_step_active_spears_damage_and_stun_enemy() {
        let mut g = room();
        let spear = g.world.spawn("oSpearsLeft", Vec2::new(96.0, 96.0));
        g.world.entity_mut(spear).unwrap().animation = 21.0;
        let id = g.world.spawn("oCaveman", Vec2::new(96.0, 96.0));
        g.step(&InputFrame::default());
        let enemy = g.world.entity(id).unwrap();
        assert_eq!(enemy.health, 1);
        assert_eq!(enemy.state, 98);
        assert!(enemy.vel.y < 0.0);
    }
    #[test]
    fn actual_step_fast_stunned_body_hits_another_enemy() {
        let mut g = room();
        let thrown = g.world.spawn("oCaveman", Vec2::new(80.0, 96.0));
        let e = g.world.entity_mut(thrown).unwrap();
        e.state = 98;
        e.timer = 200;
        e.vel = Vec2::new(4.0, 0.0);
        let target = g.world.spawn("oYeti", Vec2::new(94.0, 96.0));
        g.world.entity_mut(target).unwrap().vel = Vec2::default();
        g.step(&InputFrame::default());
        let target = g.world.entity(target).unwrap();
        assert_eq!(target.health, 4);
        assert_eq!(target.state, 98);
        assert!(target.vel.y < 0.0);
    }
    #[test]
    fn actual_step_stationary_owned_arrow_does_not_hit_enemy() {
        let mut g = room();
        let enemy = g.world.spawn("oCaveman", Vec2::new(96.0, 96.0));
        let arrow = g.world.spawn("oArrow", Vec2::new(104.0, 104.0));
        g.world
            .entity_mut(arrow)
            .unwrap()
            .data
            .insert("owner_player".into(), 1.0);
        g.step(&InputFrame::default());
        assert_eq!(g.world.entity(enemy).unwrap().health, 3);
    }
    #[test]
    fn actual_step_outside_room_enemy_destroys_without_credit() {
        let mut g = room();
        g.camera = Vec2::new(450.0, 0.0);
        g.player.pos = Vec2::new(600.0, 96.0);
        let id = g.world.spawn("oYeti", Vec2::new(650.0, 96.0));
        g.step(&InputFrame::default());
        assert!(g.world.entity(id).is_none());
        assert_eq!(g.data.get("kills"), None);
    }
    #[test]
    fn actual_step_yeti_king_turn_uses_feet_height_and_preserves_turn_counter() {
        let mut g = room();
        g.player.pos = Vec2::new(40.0, 100.0);
        let id = g.world.spawn("oYetiKing", Vec2::new(80.0, 64.0));
        let e = g.world.entity_mut(id).unwrap();
        e.state = 1;
        e.vel = Vec2::default();
        e.facing = 1;
        e.data.insert("counter".into(), 0.0);
        g.step(&InputFrame::default());
        assert_eq!(g.world.entity(id).unwrap().state, 2);
        assert_eq!(g.world.entity(id).unwrap().value("counter"), 30.0);
        g.step(&InputFrame::default());
        assert_eq!(g.world.entity(id).unwrap().value("counter"), 30.0);
    }
    #[test]
    fn actual_step_undead_animation_advances() {
        for (object, speed) in [("oSkeleton", 0.5), ("oZombie", 0.4), ("oVampire", 0.5)] {
            let mut g = room();
            let id = g.world.spawn(object, Vec2::new(96.0, 96.0));
            g.step(&InputFrame::default());
            assert_eq!(g.world.entity(id).unwrap().animation, speed, "{object}");
        }
    }
    #[test]
    fn actual_step_stunned_bodies_bounce_and_recover_on_support() {
        for object in ["oCaveman", "oHawkman", "oShopkeeper"] {
            let mut g = room();
            for x in 0..30 {
                g.world.set_tile(x, 8, "oBlock");
            }
            let id = g.world.spawn(object, Vec2::new(96.0, 112.0));
            let e = g.world.entity_mut(id).unwrap();
            e.state = 98;
            e.timer = 1;
            e.vel = Vec2::new(4.0, 6.0);
            e.data.insert("has_gun".into(), 0.0);
            g.step(&InputFrame::default());
            let e = g.world.entity(id).unwrap();
            assert_eq!(
                e.state, 98,
                "{object} recovered before counter was already zero"
            );
            assert!((e.vel.y + 3.3).abs() < 1e-9, "{object} vy={}", e.vel.y);
            assert!((e.vel.x - 1.1).abs() < 1e-9, "{object} vx={}", e.vel.x);
            assert_eq!(e.value("collision_top"), 6.0);
            for _ in 0..120 {
                g.step(&InputFrame::default());
                if g.world.entity(id).unwrap().state < 98 {
                    break;
                }
            }
            assert!(
                g.world.entity(id).unwrap().state < 98,
                "{object} never recovered"
            );
        }
    }
    #[test]
    fn actual_step_hawkman_ceiling_hit_damages_without_stun() {
        let mut g = room();
        g.player.pos = Vec2::new(104.0, 105.0);
        g.player.state = PlayerState::Falling;
        g.world.set_tile(6, 5, "oBlock");
        let id = g.world.spawn("oHawkman", Vec2::new(96.0, 96.0));
        g.world.entity_mut(id).unwrap().vel = Vec2::default();
        g.step(&InputFrame::default());
        assert_eq!(g.player.health, 3);
        assert_eq!(g.player.invincible, 30);
        assert_eq!(g.player.stun, 0);
        assert_ne!(g.player.state, PlayerState::Stunned);
        assert_eq!(g.player.vel.x, 6.0);
    }
    #[test]
    fn actual_step_magma_man_recognizes_dynamic_support_and_moves_current_walk_speed() {
        let mut g = room();
        g.world.set_tile(6, 9, "oBlock");
        g.world.set_tile(7, 9, "oBlock");
        g.world.spawn("oPushBlock", Vec2::new(96.0, 128.0));
        g.world.spawn("oPushBlock", Vec2::new(112.0, 128.0));
        let id = g.world.spawn("oMagmaMan", Vec2::new(96.0, 112.0));
        let e = g.world.entity_mut(id).unwrap();
        e.state = 1;
        e.facing = 1;
        e.vel = Vec2::default();
        g.step(&InputFrame::default());
        let e = g
            .world
            .entity(id)
            .expect("dynamic support must not turn MagmaMan into Magma");
        assert_eq!(
            e.pos.x, 98.0,
            "source room time2 emits 1.5 as2, selected before moveTo"
        );
    }
    #[test]
    fn source_move_to_boundary_lines_allow_escape_from_existing_corner_overlap() {
        let mut g = room();
        g.world.set_tile(6, 5, "oBlock");
        let mut e = Entity::new(99, "oCaveman", Vec2::new(83.0, 92.0));
        init_entity(&mut e);
        e.vel = Vec2::new(2.0, 0.0);
        move_body(&g.world, &mut e, 0.0);
        // Its top overlaps the block, but the source horizontal collision line
        // starts at tb+5 and therefore clears the corner at y97.
        assert_eq!(e.pos.x, 85.0);
    }
    #[test]
    fn actual_step_fast_items_use_inherited_impact_contract() {
        for (object, health) in [
            ("oRock", 2),
            ("oShotgun", 2),
            ("oKey", 2),
            ("oChest", 2),
            ("oJar", 3),
            ("oSkull", 3),
        ] {
            let mut g = room();
            let enemy = g.world.spawn("oCaveman", Vec2::new(96.0, 96.0));
            let item = g.world.spawn(object, Vec2::new(100.0, 104.0));
            g.world.entity_mut(item).unwrap().vel = Vec2::new(4.0, 0.0);
            g.step(&InputFrame::default());
            let e = g.world.entity(enemy).unwrap();
            assert_eq!(e.health, health, "{object}");
            assert_eq!(e.state, 98, "{object}");
            assert_eq!(e.vel.y, -6.0, "{object}");
        }
    }
    #[test]
    fn actual_step_arrow_stakes_vampire_and_generic_item_angers_keeper() {
        let mut g = room();
        let enemy = g.world.spawn("oVampire", Vec2::new(96.0, 96.0));
        g.world.entity_mut(enemy).unwrap().vel = Vec2::default();
        let item = g.world.spawn("oArrow", Vec2::new(100.0, 104.0));
        g.world.entity_mut(item).unwrap().vel = Vec2::new(4.0, 0.0);
        g.step(&InputFrame::default());
        assert_eq!(g.world.entity(enemy).unwrap().health, 2);
        let mut g = room();
        let enemy = g.world.spawn("oShopkeeper", Vec2::new(96.0, 96.0));
        g.world
            .entity_mut(enemy)
            .unwrap()
            .data
            .insert("has_gun".into(), 0.0);
        let item = g.world.spawn("oKey", Vec2::new(100.0, 104.0));
        g.world.entity_mut(item).unwrap().vel = Vec2::new(4.0, 0.0);
        g.step(&InputFrame::default());
        let e = g.world.entity(enemy).unwrap();
        assert_eq!(e.health, 19);
        assert_eq!(e.state, 2);
    }
    #[test]
    fn actual_step_held_enemy_offsets_follow_facing_and_ducking() {
        for (facing, duck) in [(-1, false), (1, false), (-1, true), (1, true)] {
            let mut g = room();
            for x in 0..30 {
                g.world.set_tile(x, 8, "oBlock");
            }
            g.player.pos = Vec2::new(120.0, 120.0);
            g.player.facing = facing;
            let id = g.world.spawn("oCaveman", g.player.pos);
            g.player.held = Some(id);
            let e = g.world.entity_mut(id).unwrap();
            e.held = true;
            e.state = 98;
            e.timer = 200;
            g.step(&InputFrame {
                down: duck,
                ..Default::default()
            });
            let e = g.world.entity(id).unwrap();
            assert_eq!(
                e.pos,
                Vec2::new(
                    g.player.pos.x + if facing < 0 { -12.0 } else { -4.0 },
                    g.player.pos.y - if duck { 10.0 } else { 12.0 }
                )
            );
            assert_eq!(e.sprite, "sCavemanHeldL");
        }
    }
    #[test]
    fn actual_step_safe_item_spares_carried_damsel_and_unsafe_item_releases_her() {
        for safe in [false, true] {
            let mut g = room();
            let id = g.world.spawn("oDamsel", g.player.pos);
            g.player.held = Some(id);
            g.world.entity_mut(id).unwrap().held = true;
            g.step(&InputFrame::default());
            let pos = g.world.entity(id).unwrap().pos;
            let rock = g.world.spawn("oRock", Vec2::new(pos.x - 4.0, pos.y));
            let e = g.world.entity_mut(rock).unwrap();
            e.vel = Vec2::new(4.0, 0.0);
            e.data.insert("safe".into(), f64::from(safe));
            g.step(&InputFrame::default());
            let e = g.world.entity(id).unwrap();
            assert_eq!(e.health, if safe { 4 } else { 3 }, "safe={safe}");
            assert_eq!(e.held, safe);
            assert_eq!(g.player.held.is_some(), safe);
            if !safe {
                let pos = e.pos;
                let rock = g.world.spawn("oRock", Vec2::new(pos.x - 4.0, pos.y));
                g.world.entity_mut(rock).unwrap().vel = Vec2::new(4.0, 0.0);
                g.step(&InputFrame::default());
                assert_eq!(
                    g.world.entity(id).unwrap().health,
                    3,
                    "hit cooldown must prevent a second immediate impact"
                );
            }
        }
    }
    #[test]
    fn actual_step_collision_events_run_outside_narrow_ai_gate() {
        let mut g = room();
        let enemy = g.world.spawn("oSnake", Vec2::new(370.0, 80.0));
        let bullet = g.world.spawn("oBullet", Vec2::new(375.0, 87.0));
        g.step(&InputFrame::default());
        assert!(g.world.entity(enemy).is_none_or(|e| e.health <= 0));
        assert!(g.world.entity(bullet).is_none());
    }
    #[test]
    fn actual_step_bullet_enemy_collision_uses_safe_flag_not_ownership() {
        for safe in [false, true] {
            let mut g = room();
            let enemy = g.world.spawn("oCaveman", Vec2::new(96.0, 96.0));
            let bullet = g.world.spawn("oBullet", Vec2::new(104.0, 104.0));
            g.world
                .entity_mut(bullet)
                .unwrap()
                .data
                .insert("safe".into(), f64::from(safe));
            g.step(&InputFrame::default());
            assert_eq!(
                g.world.entity(enemy).unwrap().health,
                if safe { 3 } else { -1 }
            );
            assert_eq!(g.world.entity(bullet).is_some(), safe);
        }
    }
    #[test]
    fn actual_step_dead_player_with_positive_health_is_not_a_living_psychic_target() {
        for dead in [false, true] {
            let mut g = room();
            g.player.pos = Vec2::new(112.0, 112.0);
            g.player.health = 1;
            g.player.state = if dead {
                PlayerState::Dead
            } else {
                PlayerState::Falling
            };
            g.state = if dead {
                RunState::Dead
            } else {
                RunState::Playing
            };
            let boss = g.world.spawn("oAlienBoss", Vec2::new(96.0, 96.0));
            let e = g.world.entity_mut(boss).unwrap();
            e.vel = Vec2::default();
            e.data.insert("psychic_recover".into(), 0.0);
            g.step(&InputFrame::default());
            let count = g
                .world
                .entities
                .iter()
                .filter(|e| e.object == "oPsychicCreate")
                .count();
            assert_eq!(count, if dead { 0 } else { 6 });
        }
        let mut g = room();
        g.player.health = 1;
        g.player.state = PlayerState::Dead;
        g.state = RunState::Dead;
        g.world.spawn("oPsychicWave", g.player.pos);
        g.step(&InputFrame::default());
        assert_eq!(g.player.health, 1);
        assert_eq!(g.player.stun, 0);
    }
    #[test]
    fn actual_step_airborne_magma_stuns_kings_but_magma_man_does_not() {
        for object in ["oMagma", "oMagmaMan"] {
            let mut g = room();
            for x in 0..30 {
                g.world.set_tile(x, 8, "oBlock");
            }
            let king = g.world.spawn("oYetiKing", Vec2::new(96.0, 96.0));
            g.world.entity_mut(king).unwrap().vel = Vec2::default();
            let magma = g.world.spawn(
                object,
                if object == "oMagma" {
                    Vec2::new(112.0, 112.0)
                } else {
                    Vec2::new(104.0, 112.0)
                },
            );
            let e = g.world.entity_mut(magma).unwrap();
            e.vel = Vec2::default();
            e.timer = 20;
            e.data.insert("started".into(), 1.0);
            e.data.insert("gravity".into(), 0.0);
            g.step(&InputFrame::default());
            let e = g.world.entity(king).unwrap();
            assert_eq!(e.health, 28);
            assert_eq!(e.state == 98, object == "oMagma");
        }
    }
    #[test]
    fn actual_step_yeti_and_hawk_throws_preserve_existing_stun_timer() {
        for object in ["oYeti", "oHawkman"] {
            let mut g = room();
            for x in 0..30 {
                g.world.set_tile(x, 9, "oBlock");
            }
            g.player.pos = Vec2::new(136.0, 136.0);
            g.player.state = PlayerState::Stunned;
            g.player.stun = 20;
            g.player.vel = Vec2::new(1.0, 0.0);
            g.player.sprite = "sDieLR".into();
            let id = g.world.spawn(object, Vec2::new(128.0, 128.0));
            let e = g.world.entity_mut(id).unwrap();
            e.vel = Vec2::default();
            e.timer = 20;
            g.step(&InputFrame::default());
            assert_eq!(
                g.world.entity(id).unwrap().state,
                3,
                "{object} must execute its actual throw"
            );
            assert_eq!(
                g.player.stun, 20,
                "{object} must preserve the preexisting timer"
            );
            assert_eq!(g.player.state, PlayerState::Stunned);
            assert_eq!(g.player.vel.y, -6.0);
        }
    }
    #[test]
    fn actual_step_keeper_does_not_throw_zero_timer_stunned_player() {
        let mut g = room();
        for x in 0..30 {
            g.world.set_tile(x, 9, "oBlock");
        }
        g.player.pos = Vec2::new(136.0, 136.0);
        g.player.state = PlayerState::Stunned;
        g.player.vel = Vec2::new(1.0, 0.0);
        g.player.sprite = "sDieLR".into();
        let id = g.world.spawn("oShopkeeper", Vec2::new(128.0, 128.0));
        let e = g.world.entity_mut(id).unwrap();
        e.vel = Vec2::default();
        e.state = 2;
        e.data.insert("has_gun".into(), 0.0);
        g.step(&InputFrame::default());
        assert_eq!(g.world.entity(id).unwrap().state, 2);
        assert_ne!(g.player.vel.y, -6.0);
    }
    #[test]
    fn actual_step_dark_fall_edge_uses_precise_solid_mask() {
        // Source getIdCollisionRight(1) uses precise=1. The DarkFall top
        // row begins with a transparent pixel, followed by opaque pixels.
        for dynamic in [false, true] {
            for opaque in [false, true] {
                let mut g = room();
                if dynamic {
                    g.world.spawn("oDarkFall", Vec2::new(80.0, 80.0));
                } else {
                    g.world.set_tile(5, 5, "oDarkFall");
                }
                let start_x = if opaque { 67.0 } else { 66.0 };
                let id = g.world.spawn("oSnake", Vec2::new(start_x, 65.0));
                let e = g.world.entity_mut(id).unwrap();
                e.vel = Vec2::new(1.0, 0.0);
                e.timer = 20;
                g.step(&InputFrame::default());
                assert_eq!(
                    g.world.entity(id).unwrap().pos.x,
                    start_x + if opaque { 0.0 } else { 1.0 },
                    "dynamic={dynamic}, opaque={opaque}"
                );
            }
        }
    }
    #[test]
    fn actual_step_leaves_underside_keeps_source_platform_family_filter() {
        // Leaves' last row has transparent pixels 0..4, then opaque pixel5.
        // A real Snake's upward edge reaches only that final terrain row.
        // Both controls remain open: Leaves derives oPlatform, not oSolid.
        for opaque in [false, true] {
            let mut g = room();
            g.world.set_tile(5, 5, "oLeaves");
            let id = g
                .world
                .spawn("oSnake", Vec2::new(if opaque { 72.0 } else { 67.0 }, 96.0));
            let e = g.world.entity_mut(id).unwrap();
            e.vel = Vec2::new(0.0, -1.0);
            e.timer = 20;
            g.step(&InputFrame::default());
            assert_eq!(g.world.entity(id).unwrap().pos.y, 95.0);
        }
    }
    #[test]
    fn actual_step_crush_point_keeps_source_nonprecise_flag() {
        let mut g = room();
        g.world.set_tile(5, 5, "oDarkFall");
        let id = g.world.spawn("oSnake", Vec2::new(72.0, 72.0));
        let e = g.world.entity_mut(id).unwrap();
        e.vel = Vec2::default();
        e.timer = 20;
        // Snake's explicit collision_point(x+8,y+8,oSolid,0,0) sees
        // the transparent top-left corner because its precision flag is 0.
        g.step(&InputFrame::default());
        assert!(g.world.entity(id).is_none_or(|e| e.health <= 0));
    }
    #[test]
    fn actual_step_enemy_item_impacts_keep_parent_viewport_and_fragile_overrides() {
        for object in ["oRock", "oShotgun", "oJar", "oSkull"] {
            let mut g = room();
            g.player.pos = Vec2::new(40.0, 160.0);
            let enemy = g.world.spawn("oCaveman", Vec2::new(370.0, 80.0));
            let item = g.world.spawn(object, Vec2::new(370.0, 87.0));
            g.world.entity_mut(item).unwrap().vel.x = 5.0;
            g.step(&InputFrame::default());
            let e = g.world.entity(enemy).unwrap();
            assert_eq!(e.health, 3, "{object}");
            assert_eq!(
                e.state == 98,
                matches!(object, "oJar" | "oSkull"),
                "{object}"
            );
            if matches!(object, "oJar" | "oSkull") {
                assert!(g.world.entity(item).is_none());
                assert!(g.world.entities.iter().any(|e| e.object == "oSmokePuff"));
            }
        }
    }
    #[test]
    fn actual_step_dice_uses_its_own_strict_three_speed_impact_threshold() {
        for speed in [3.0, 4.0] {
            let mut g = room();
            let enemy = g.world.spawn("oCaveman", Vec2::new(100.0, 80.0));
            g.world.entity_mut(enemy).unwrap().vel = Vec2::default();
            let die = g.world.spawn("oDice", Vec2::new(98.0, 88.0));
            g.world.entity_mut(die).unwrap().vel.x = speed;
            g.step(&InputFrame::default());
            assert_eq!(
                g.world.entity(enemy).unwrap().health,
                if speed == 3.0 { 3 } else { 2 }
            );
        }
    }
    #[test]
    fn actual_step_natural_mantrap_meal_finishes_after_player_falls_out_of_ai_view() {
        let mut g = room();
        for x in 8..40 {
            g.world.set_tile(x, 10, "oBlock");
        }
        for x in 0..8 {
            g.world.set_tile(x, 21, "oBlock");
        }
        g.player.pos = Vec2::new(40.0, 152.0);
        g.player.state = PlayerState::Falling;
        g.player.vel.y = 10.0;
        let trap = g.world.spawn("oManTrap", Vec2::new(144.0, 144.0));
        g.world.spawn("oCaveman", Vec2::new(144.0, 144.0));
        g.step(&InputFrame::default());
        assert_eq!(g.world.entity(trap).unwrap().state, 97);
        for _ in 0..70 {
            g.step(&InputFrame::default());
        }
        let e = g.world.entity(trap).unwrap();
        assert!(g.camera.y > e.pos.y + 20.0);
        assert_eq!(e.state, 98);
        assert_eq!(e.timer, 400); // independent AnimationEnd; AI counter remains frozen
        assert_eq!(e.sprite, "sManTrapStunL");
    }
    #[test]
    fn actual_step_natural_tomb_turn_finishes_after_player_runs_out_of_ai_view() {
        let mut g = room();
        for x in 0..40 {
            g.world.set_tile(x, 10, "oBlock");
        }
        g.player.pos = Vec2::new(300.0, 152.0);
        g.update_camera();
        let tomb = g.world.spawn("oTombLord", Vec2::new(398.0, 128.0));
        let input = InputFrame {
            left: true,
            run: true,
            ..InputFrame::default()
        };
        g.step(&input);
        g.step(&input);
        assert_eq!(g.world.entity(tomb).unwrap().state, 2);
        for _ in 0..40 {
            g.step(&input);
        }
        let e = g.world.entity(tomb).unwrap();
        assert!(e.pos.x >= g.camera.x + VIEW_WIDTH);
        assert_eq!(e.state, 1);
        assert_eq!(e.facing, -1);
    }
}

#[cfg(test)]
mod burning_source_tests {
    use super::*;

    // Literal rand statements, independent of production chance/range helpers.
    fn literal_burn(rng: &mut Rng, denominator: u64, bounds: (u64, u64)) -> Option<Vec2> {
        if 1 + rng.next_u64() % denominator != 1 {
            return None;
        }
        Some(Vec2::new(
            (rng.next_u64() % (bounds.0 + 1)) as f64,
            (rng.next_u64() % (bounds.1 + 1)) as f64,
        ))
    }

    fn scene(seed: u64) -> Game {
        let mut game = Game::shell(seed, RulesPreset::Classic);
        game.world = World::new(40, 30, Area::Temple, 13);
        game.player.pos = Vec2::new(240.0, 152.0);
        game.player.previous_pos = game.player.pos;
        for x in 0..40 {
            game.world.set_tile(x, 10, "oTemple");
        }
        game
    }

    #[test]
    fn inherited_burning_literal_canvas_endpoints_and_next_rng() {
        let mut emitted = false;
        let mut skipped = false;
        for seed in 0..128 {
            for object in ["oCaveman", "oYetiKing", "oVampire"] {
                let mut game = scene(seed);
                let id = game.spawn(object, Vec2::new(80.0, 80.0));
                let mut actor = game.world.entity(id).unwrap().clone();
                // A source-local event oracle, not the natural ignition fixture.
                actor.data.insert("burning".into(), 1.0);
                let (w, h) = crate::catalog::sprite_dimensions(&actor.sprite).unwrap();
                let mut expected = game.rng.clone();
                let offset = literal_burn(&mut expected, 5, (w as u64, h as u64));
                tick(&mut game, &mut actor);
                let burns: Vec<_> = game
                    .world
                    .entities
                    .iter()
                    .filter(|e| e.object == "oBurn")
                    .collect();
                assert_eq!(
                    burns.len(),
                    usize::from(offset.is_some()),
                    "{object} seed {seed}"
                );
                if let Some(offset) = offset {
                    emitted = true;
                    assert_eq!(burns[0].pos, Vec2::new(80.0 + offset.x, 80.0 + offset.y));
                    assert_eq!(burns[0].vel.y, -0.1);
                    assert_eq!(burns[0].image_speed(), 0.4);
                } else {
                    skipped = true;
                }
                assert_eq!(actor.value("burning"), 0.0);
                assert_eq!(
                    game.rng.next_u64(),
                    expected.next_u64(),
                    "{object} seed {seed}"
                );
            }
        }
        assert!(emitted && skipped);
    }

    #[test]
    fn actual_explosion_ignites_enemy_and_damsel_then_emits_finite_burns() {
        for object in ["oCaveman", "oDamsel"] {
            let mut observed_ignition = false;
            let mut observed_burn = false;
            for seed in 0..16 {
                let mut game = scene(seed);
                let pos = if object == "oDamsel" {
                    Vec2::new(104.0, 120.0)
                } else {
                    Vec2::new(96.0, 112.0)
                };
                let id = game.spawn(object, pos);
                // Real source collision owns HP, launch, and burning=50.
                // Only initial authored scene geometry is supplied here.
                game.spawn("oExplosion", Vec2::new(104.0, 120.0));
                let mut ignited = false;
                let mut produced = false;
                for _ in 0..50 {
                    game.step(&InputFrame::default());
                    ignited |= game
                        .world
                        .entity(id)
                        .is_some_and(|e| e.value("burning") > 0.0);
                    produced |= game.world.entities.iter().any(|e| e.object == "oBurn");
                }
                assert!(
                    ignited,
                    "source Explosion collision must ignite {object}, seed {seed}"
                );
                observed_ignition |= ignited;
                observed_burn |= produced;
                for _ in 0..100 {
                    game.step(&InputFrame::default());
                }
                assert!(
                    !game.world.entities.iter().any(|e| e.object == "oBurn"),
                    "finite burn animation, {object} seed {seed}"
                );
            }
            assert!(
                observed_ignition && observed_burn,
                "{object} must produce actual burning feedback"
            );
        }
    }

    #[test]
    fn actual_authored_alien_eject_burns_only_during_ejection_with_literal_rng() {
        let mut emitted = false;
        for seed in 0..32 {
            let mut game = scene(seed);
            let id = game.spawn("oAlienEject", Vec2::new(96.0, 112.0));
            let mut expected = game.rng.clone();
            let mut y = 112.0;
            let mut vy = -6.0;
            let mut expected_births = Vec::new();
            for _ in 0..12 {
                y += vy;
                vy += 0.5;
                let emits = 1 + expected.next_u64() % 5 == 1;
                if emits {
                    let x =
                        96.0 + (expected.next_u64() % 4) as f64 - (expected.next_u64() % 4) as f64;
                    let py =
                        y + (expected.next_u64() % 4) as f64 - (expected.next_u64() % 4) as f64;
                    expected_births.push(Vec2::new(x, py));
                }
                let prior_id = game.world.entities.iter().map(|e| e.id).max().unwrap_or(0) + 1;
                game.step(&InputFrame::default());
                let newborns: Vec<_> = game
                    .world
                    .entities
                    .iter()
                    .filter(|e| e.object == "oBurn" && e.id >= prior_id)
                    .collect();
                assert_eq!(newborns.len(), usize::from(emits), "seed {seed}");
                // Birth coordinates precede the automatic clock. Burn's Step
                // resource precedes AlienEject, so it has not moved this frame.
                for burn in newborns {
                    assert_eq!(Some(&burn.pos), expected_births.last());
                    emitted = true;
                }
                assert_eq!(
                    game.rng.clone().next_u64(),
                    expected.clone().next_u64(),
                    "seed {seed}"
                );
            }
            assert_eq!(game.world.entity(id).unwrap().state, 2);
            let before = game.rng.clone().next_u64();
            game.step(&InputFrame::default());
            assert_eq!(
                game.rng.clone().next_u64(),
                before,
                "FLOAT makes no burn roll"
            );
        }
        assert!(emitted);
    }
    #[test]
    fn damsel_burning_literal_invincibility_gate_and_next_rng() {
        for invincible in [false, true] {
            for seed in 0..64 {
                let mut game = scene(seed);
                let id = game.spawn("oDamsel", Vec2::new(96.0, 112.0));
                let mut actor = game.world.entity(id).unwrap().clone();
                actor.data.insert("burning".into(), 1.0);
                actor
                    .data
                    .insert("invincible".into(), f64::from(invincible));
                let mut expected = game.rng.clone();
                let offset = if invincible {
                    None
                } else {
                    literal_burn(&mut expected, 5, (8, 8))
                };
                super::super::npcs::tick(&mut game, &mut actor);
                let burns: Vec<_> = game
                    .world
                    .entities
                    .iter()
                    .filter(|e| e.object == "oBurn")
                    .collect();
                assert_eq!(burns.len(), usize::from(offset.is_some()));
                if let Some(offset) = offset {
                    assert_eq!(
                        burns[0].pos,
                        Vec2::new(actor.pos.x + 4.0 + offset.x, actor.pos.y + 4.0 + offset.y)
                    );
                }
                assert_eq!(actor.value("burning"), f64::from(invincible));
                assert_eq!(game.rng.next_u64(), expected.next_u64());
            }
        }
    }

    #[test]
    fn actual_magma_man_burn_roll_and_unsupported_destroy_continuation_rng() {
        let mut emitted = false;
        for unsupported in [false, true] {
            for seed in 0..64 {
                let mut game = scene(seed);
                let y = if unsupported { 80.0 } else { 144.0 };
                let id = game.spawn("oMagmaMan", Vec2::new(96.0, y));
                // Initial source-reachable IDLE counter distinguishes the
                // unsupported continuation from the grounded transition.
                game.world.entity_mut(id).unwrap().timer = 1;
                let mut expected = game.rng.clone();
                if unsupported {
                    // oMagma's own Create draws x twice, y, and acceleration
                    // after inherited Detritus defaults, before the burn roll.
                    for _ in 0..4 {
                        expected.next_u64();
                    }
                }
                let offset = literal_burn(&mut expected, 20, (8, 8));
                if !unsupported {
                    expected.next_u64(); // counter1 -> 0: facing rand(0,1)
                    let transform = 1 + expected.next_u64() % 6 == 1;
                    if transform {
                        for _ in 0..4 {
                            expected.next_u64();
                        }
                    }
                }
                game.step(&InputFrame::default());
                let burns: Vec<_> = game
                    .world
                    .entities
                    .iter()
                    .filter(|e| e.object == "oBurn")
                    .collect();
                assert_eq!(
                    burns.len(),
                    usize::from(offset.is_some()),
                    "unsupported={unsupported}, seed={seed}"
                );
                if let Some(offset) = offset {
                    emitted = true;
                    assert_eq!(
                        burns[0].pos,
                        Vec2::new(100.0 + offset.x, y + 4.0 + offset.y)
                    );
                }
                if unsupported {
                    assert!(game.world.entity(id).is_none());
                    assert_eq!(
                        game.world
                            .entities
                            .iter()
                            .filter(|e| e.object == "oMagma")
                            .count(),
                        1
                    );
                }
                assert_eq!(
                    game.rng.next_u64(),
                    expected.next_u64(),
                    "unsupported={unsupported}, seed={seed}"
                );
            }
        }
        assert!(emitted);
    }
    #[test]
    fn inherited_fire_frog_water_transform_continues_burning_after_destroy() {
        let mut emitted = false;
        let mut skipped = false;
        for burning in [0.0, 1.0] {
            for seed in 0..64 {
                let mut game = scene(seed);
                game.world.set_tile(6, 7, "oWaterSwim");
                let id = game.spawn("oFireFrog", Vec2::new(96.0, 112.0));
                let index = game.world.entities.iter().position(|e| e.id == id).unwrap();
                // Match the enemy event adapter's detached receiver. Initial
                // burning is a source-local control, not a natural ignition.
                let mut actor = game.world.entities.remove(index);
                actor.data.insert("burning".into(), burning);
                let (width, height) = crate::catalog::sprite_dimensions(&actor.sprite).unwrap();
                let mut expected = game.rng.clone();
                let expected_facing = if expected.next_u64() % 2 == 0 { -1 } else { 1 };
                let offset = if burning > 0.0 {
                    literal_burn(&mut expected, 5, (width as u64, height as u64))
                } else {
                    None
                };

                // The actual oEnemy parent event, which FireFrog Step inherits.
                tick(&mut game, &mut actor);

                assert!(!actor.alive, "water transforms the original FireFrog");
                assert_eq!(actor.value("burning"), 0.0);
                assert!(game.world.entity(id).is_none());
                let frogs: Vec<_> = game
                    .world
                    .entities
                    .iter()
                    .filter(|e| e.object == "oFrog")
                    .collect();
                assert_eq!(frogs.len(), 1);
                assert_eq!(frogs[0].pos, Vec2::new(96.0, 112.0));
                assert_eq!(frogs[0].state, actor.state);
                assert_eq!(frogs[0].facing, expected_facing);
                let burns: Vec<_> = game
                    .world
                    .entities
                    .iter()
                    .filter(|e| e.object == "oBurn")
                    .collect();
                assert_eq!(burns.len(), usize::from(offset.is_some()));
                if let Some(offset) = offset {
                    emitted = true;
                    assert!(
                        frogs[0].id < burns[0].id,
                        "Frog Create precedes burn emission"
                    );
                    assert_eq!(burns[0].pos, Vec2::new(96.0 + offset.x, 112.0 + offset.y));
                } else if burning > 0.0 {
                    skipped = true;
                }
                assert_eq!(
                    game.rng.next_u64(),
                    expected.next_u64(),
                    "burning={burning}, seed={seed}"
                );
            }
        }
        assert!(emitted && skipped);
    }

    #[test]
    fn damsel_crush_preserves_receiver_hp_and_continues_burning_after_destroy() {
        let mut emitted = false;
        let mut skipped = false;
        for burning in [0.0, 1.0] {
            for seed in 0..64 {
                let mut game = scene(seed);
                game.world.set_tile(6, 7, "oTemple");
                let id = game.spawn("oDamsel", Vec2::new(104.0, 120.0));
                let index = game.world.entities.iter().position(|e| e.id == id).unwrap();
                let mut actor = game.world.entities.remove(index);
                actor.data.insert("burning".into(), burning);
                let hp = actor.health;
                let mut expected = game.rng.clone();
                // scrCreateBlood's three fresh Blood instances each execute
                // their four own Create draws before instance_destroy resumes.
                for _ in 0..12 {
                    expected.next_u64();
                }
                let offset = if burning > 0.0 {
                    literal_burn(&mut expected, 5, (8, 8))
                } else {
                    None
                };

                // Actual Damsel Step includes its inherited oItem body first.
                super::super::npcs::tick(&mut game, &mut actor);

                assert!(!actor.alive, "the authored solid crush must destroy Damsel");
                assert!(game.world.solid_at(actor.pos.x, actor.pos.y));
                assert_eq!(
                    actor.health, hp,
                    "instance_destroy does not assign receiver hp=0"
                );
                assert_eq!(actor.value("burning"), 0.0);
                let blood: Vec<_> = game
                    .world
                    .entities
                    .iter()
                    .filter(|e| e.object == "oBlood")
                    .collect();
                assert_eq!(blood.len(), 3);
                assert!(blood.iter().all(|e| e.pos == actor.pos));
                let burns: Vec<_> = game
                    .world
                    .entities
                    .iter()
                    .filter(|e| e.object == "oBurn")
                    .collect();
                assert_eq!(burns.len(), usize::from(offset.is_some()));
                if let Some(offset) = offset {
                    emitted = true;
                    assert!(
                        blood.iter().all(|e| e.id < burns[0].id),
                        "all Blood Creates precede burn roll"
                    );
                    assert_eq!(
                        burns[0].pos,
                        Vec2::new(actor.pos.x + 4.0 + offset.x, actor.pos.y + 4.0 + offset.y)
                    );
                } else if burning > 0.0 {
                    skipped = true;
                }
                assert_eq!(
                    game.rng.next_u64(),
                    expected.next_u64(),
                    "burning={burning}, seed={seed}"
                );
            }
        }
        assert!(emitted && skipped);
    }
}
