use spelunky_core::{Entity, Game, GameEvent, InputFrame, RulesPreset, level::Level};

fn room(rules: RulesPreset, keeper_left: Option<bool>) -> Game {
    let mut level = Level::default();
    level.set_tile(6, 7, '@').unwrap();
    level.set_tile(34, 7, 'X').unwrap();
    if let Some(left) = keeper_left {
        level.set_tile(if left { 4 } else { 8 }, 7, '!').unwrap();
    }
    for x in 0..spelunky_core::level::LEVEL_WIDTH {
        level.set_tile(x, 8, '1').unwrap();
    }
    let mut game = Game::shell(42, rules);
    game.load_custom(&level, "bullet-character", "");
    assert_eq!(game.player.health, 4.0);
    game
}

// Query-only proxy copies the real player mask; it is never spawned or inserted.
fn player_mask_proxy(game: &Game) -> Entity {
    let mut proxy = Entity::new(0, "oPlayer1", game.player.pos);
    proxy.sprite = game.player.sprite.clone();
    proxy.animation = game.player.animation;
    proxy.data = game.player.data.clone();
    proxy
        .data
        .entry("image_xscale".into())
        .or_insert(if game.player.facing > 0 { -1.0 } else { 1.0 });
    proxy
}

#[test]
fn authored_patrol_keeper_fires_and_first_actual_hit_creates_character_blood() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for left in [false, true] {
            let mut game = room(rules, Some(left));
            let mut saw_shot = false;
            let mut saw_projected_contact = false;
            let mut hit = false;
            for _ in 0..16 {
                let player_mask = player_mask_proxy(&game);
                let bullets: Vec<_> = game
                    .world
                    .entities
                    .iter()
                    .filter(|e| e.alive && e.object == "oBullet")
                    .cloned()
                    .collect();
                for bullet in &bullets {
                    let mut projected = bullet.clone();
                    projected.pos.x += bullet.vel.x;
                    projected.pos.y += bullet.vel.y;
                    saw_projected_contact |= projected.event_intersects(&player_mask);
                }
                game.step(&InputFrame::default());
                saw_shot |= game
                    .events
                    .iter()
                    .any(|e| matches!(e,GameEvent::Sound{name,..} if name=="sndShotgun"));
                if game.player.health < 4.0 {
                    assert_eq!(
                        game.player.health, 0.0,
                        "first ordinary Bullet hit deals source4damage"
                    );
                    let retired = bullets
                        .iter()
                        .filter(|b| game.world.entity(b.id).is_none())
                        .count();
                    assert!(
                        retired > 0,
                        "actual fired Bullet must retire on player contact"
                    );
                    let blood: Vec<_> = game
                        .world
                        .entities
                        .iter()
                        .filter(|e| e.object == "oBlood")
                        .collect();
                    assert_eq!(
                        blood.len(),
                        retired,
                        "direct source birth per actual Bullet contact, keeper_left={left}"
                    );
                    assert!(blood.iter().all(|e| e.alive && e.pos == game.player.pos));
                    assert!(saw_shot && saw_projected_contact);
                    assert_eq!(game.player.stun, 20);
                    hit = true;
                    break;
                }
            }
            assert!(
                hit,
                "unmodified authored patrol keeper must acquire, fire and hit"
            );
        }
    }
}

#[test]
fn initial_safe_owned_bullet_contact_still_hurts_and_creates_blood() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = room(rules, None);
        game.player.invincible = 100;
        let pos = game.player.pos;
        let bullet = game.spawn(
            "oBullet",
            spelunky_core::Vec2::new(pos.x - 12.0, pos.y - 2.0),
        );
        let projectile = game.world.entity_mut(bullet).unwrap();
        projectile.vel.x = 12.0;
        projectile.data.insert("safe".into(), 1.0);
        projectile.data.insert("owner_player".into(), 1.0);
        game.step(&InputFrame::default());
        assert_eq!(
            game.player.health, 0.0,
            "source Character callback ignores projectile safe/owner flags and contact invincibility"
        );
        assert!(game.world.entity(bullet).is_none());
        let blood: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|e| e.object == "oBlood")
            .collect();
        assert_eq!(blood.len(), 1);
        assert_eq!(blood[0].pos, game.player.pos);
    }
}

#[test]
fn ordinary_player_pistol_muzzle_shot_does_not_hit_its_shooter() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut level = Level::default();
        level.set_tile(6, 7, '@').unwrap();
        level.set_tile(7, 7, '-').unwrap();
        level.set_tile(34, 7, 'X').unwrap();
        for x in 0..spelunky_core::level::LEVEL_WIDTH {
            level.set_tile(x, 8, '1').unwrap();
        }
        let mut game = Game::shell(42, rules);
        game.load_custom(&level, "bullet-self-no-hit", "");
        for _ in 0..8 {
            game.step(&InputFrame::default());
        }
        for _ in 0..4 {
            game.step(&InputFrame {
                right: true,
                ..Default::default()
            });
        }
        game.step(&InputFrame {
            down: true,
            attack: true,
            attack_pressed: true,
            ..Default::default()
        });
        assert_eq!(
            game.world.entity(game.player.held.unwrap()).unwrap().object,
            "oPistol"
        );
        for _ in 0..2 {
            game.step(&InputFrame::default());
        }
        let mut fired = false;
        for tick in 0..12 {
            game.step(&InputFrame {
                attack: tick == 0,
                attack_pressed: tick == 0,
                ..Default::default()
            });
            fired |= game
                .world
                .entities
                .iter()
                .any(|e| e.alive && e.object == "oBullet");
            assert_eq!(game.player.health, 4.0);
            assert!(!game.world.entities.iter().any(|e| e.object == "oBlood"));
        }
        assert!(fired);
    }
}
