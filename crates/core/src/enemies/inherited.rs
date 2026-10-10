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
                e.sprite = sprite.into();
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
        e.data.insert("burning".into(), e.value("burning") - 1.0);
    }
    if point(&game.world, Vec2::new(center.x, e.pos.y - 1.0), "oLava") {
        e.alive = false;
        return;
    }
    if point(
        &game.world,
        Vec2::new(center.x, e.pos.y + dimensions.1 - 2.0),
        "oLava",
    ) {
        e.health = 0;
        e.data.insert("environmental_death".into(), 1.0);
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
            e.data.insert("environmental_death".into(), 1.0);
            if stunned_body {
                e.state = 98;
                e.timer = e.value("stun_time") as u32;
                e.vel = Vec2::new(if pos.x < e.pos.x { 4.0 } else { -4.0 }, -6.0);
                e.data.insert("animation_speed".into(), 0.5);
            }
            game.sound("sndHit", e.pos);
        }
    }
    if e.vel.y > 2.0
        && point(
            &game.world,
            Vec2::new(e.pos.x + 8.0, e.pos.y + 16.0),
            "oSpikes",
        )
    {
        if e.health > 0 {
            e.health = 0;
            e.data.insert("environmental_death".into(), 1.0);
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
    if e.state == 98 && (e.vel.x.abs() > 2.0 || e.vel.y.abs() > 2.0) {
        body_projectile(game, e);
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
            target.sprite = "sAlienBossHurt".into();
            target.data.insert("animation_speed".into(), 0.8);
            hit = true;
        }
        "oUFO" => {
            explosion = Some(Vec2::new(target.pos.x + 8.0, target.pos.y + 8.0));
            target.alive = false;
            target.health = 0;
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
        game.world.spawn("oExplosion", pos);
        if game.rng.range(3) == 0 {
            game.world.spawn("oAlienEject", pos);
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
}
