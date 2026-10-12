//! SOURCE-ONLY packet: authored/default actor fixture feasibility is UNRUN.
//! After initial free item placement, only ordinary inputs advance the scene.
use spelunky_core::*;
fn room(rear: bool, weapon: Option<&str>) -> (Game, u64, Option<u64>) {
    let mut game =
        Game::shell_with_algorithm(0, RulesPreset::Classic, RngAlgorithm::Gm8Lcg32V1).unwrap();
    game.world = World::new(30, 20, Area::Jungle, 5);
    for x in 0..30 {
        game.world.set_tile(x, 10, "oHardBlock");
    }
    game.player.pos = Vec2::new(if rear { 56.0 } else { 64.0 }, 152.0);
    game.player.previous_pos = game.player.pos;
    let item = weapon.map(|kind| {
        let id = game.spawn(kind, game.player.pos);
        let e = game.world.entity_mut(id).unwrap();
        e.data.insert("cost".into(), 0.0);
        e.data.insert("forSale".into(), 0.0);
        e.data.insert("for_sale".into(), 0.0);
        id
    });
    let id = game.spawn("oManTrap", Vec2::new(if rear { 32.0 } else { 80.0 }, 144.0));
    assert_eq!(
        (
            game.world.entity(id).unwrap().health,
            game.world.entity(id).unwrap().state
        ),
        (3, 0)
    );
    (game, id, item)
}
fn leaves(game: &Game) -> Vec<&Entity> {
    game.world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == "oLeaf")
        .collect()
}
fn first_hit(rear: bool, weapon: Option<&str>, expected_attack: &str, damage: i32) -> (Game, u64) {
    let (mut game, id, item) = room(rear, weapon);
    let mut hit = false;
    for tick in 0..18 {
        let before = game
            .world
            .entity(id)
            .expect("DEFAULT_MANTRAP_SURVIVES_UNTIL_FIRST_MELEE")
            .clone();
        let mut oracle = game.rng.clone();
        // Literal default ManTrap own Step precedes automatic collision.
        if before.state == 0 && before.timer == 0 {
            oracle.range(2);
        } else if before.state == 1 && oracle.range(100) == 0 {
            oracle.range(31);
        }
        // Classic's eager and evaluates the tail rand(1,8) before collision.
        oracle.range(8);
        let offset = Vec2::new(
            f64::from(oracle.range(17)),
            -8.0 + f64::from(oracle.range(17)),
        );
        let pickup = item.is_some() && tick == 0;
        let attack = if item.is_some() { tick == 2 } else { tick == 0 };
        game.step(&InputFrame {
            down: pickup,
            attack: pickup || attack,
            attack_pressed: pickup || attack,
            ..Default::default()
        });
        assert_eq!(
            game.player.health, 4,
            "NO_BODY_CONTACT_HIDING_MELEE tick={tick}"
        );
        if pickup {
            assert_eq!(game.player.held, item, "ORDINARY_FREE_WEAPON_PICKUP");
        }
        let actor = game
            .world
            .entity(id)
            .expect("FIRST_HIT_RETIRES_ONLY_ON_LATER_OWN_STEP");
        if actor.health != before.health {
            assert_eq!(actor.health, 3 - damage, "SOURCE_LITERAL_ATTACK_DAMAGE");
            assert_eq!(actor.state, 98);
            assert_eq!(actor.timer, actor.value("stun_time") as u32);
            assert_eq!(actor.vel.y, -3.0);
            assert_eq!(actor.image_speed(), 0.5);
            assert_eq!(actor.value("counts_as_kill"), 1.0);
            // Draw precedes automatic animation: final-frame Slash is alive
            // here even when Animation End retires it before Game.step returns.
            let presented = game.presentation();
            let presented_target = presented.world.entity(id).expect("SOURCE_DRAW_TARGET");
            let actual = presented
                .world
                .entities
                .iter()
                .find(|e| e.alive && e.object == expected_attack)
                .expect("ORDINARY_EXPECTED_ATTACK_DESCENDANT");
            assert!(
                actual.event_intersects(presented_target),
                "SOURCE_DRAW_PRE_ANIMATION_PRECISE_MELEE_PAIR"
            );
            assert_eq!(
                actor.vel.x,
                if actual.pos.x < actor.pos.x + 8.0 {
                    2.0
                } else {
                    -2.0
                }
            );
            let particles = leaves(&game);
            assert_eq!(particles.len(), 1, "SOURCE_MANTRAP_OWN_MELEE_ONE_LEAF");
            assert_eq!(
                particles[0].pos,
                Vec2::new(actor.pos.x + offset.x, actor.pos.y + offset.y)
            );
            assert_eq!(particles[0].vel, Vec2::new(0.0, 0.4));
            assert_eq!(particles[0].value("gravity"), 0.01);
            assert_eq!(particles[0].image_speed(), 0.2);
            assert_eq!(particles[0].age, 0, "COLLISION_LEAF_BIRTH_NO_NORMAL_STEP");
            assert_eq!(
                game.rng.state_bits(),
                oracle.state_bits(),
                "SOURCE_TWO_COORDINATE_DRAWS_NO_LEAF_CREATE_RNG"
            );
            assert!(
                game.world
                    .entities
                    .iter()
                    .all(|e| !e.alive || e.object != "oBlood")
            );
            assert!(
                game.events
                    .iter()
                    .any(|e| matches!(e,GameEvent::Sound{name,..}if name=="sndHit"))
            );
            hit = true;
            break;
        }
    }
    assert!(hit, "BOUNDED_DEFAULT_MANTRAP_ATTACK_NOT_REACHED");
    (game, id)
}
#[test]
fn ordinary_front_and_rear_whip_machete_and_mattock_each_create_source_leaf() {
    for (rear, weapon, attack, damage) in [
        (false, None, "oWhip", 1),
        (true, None, "oWhipPre", 1),
        (false, Some("oMachete"), "oSlash", 4),
        (true, Some("oMachete"), "oMachetePre", 4),
        (false, Some("oMattock"), "oMattockHit", 2),
        (true, Some("oMattock"), "oMattockPre", 2),
    ] {
        first_hit(rear, weapon, attack, damage);
    }
}
#[test]
fn naturally_stunned_mantrap_ignores_remaining_plain_whip_collision_frames() {
    for rear in [false, true] {
        let (mut game, id) = first_hit(rear, None, if rear { "oWhipPre" } else { "oWhip" }, 1);
        let first_ids: Vec<_> = leaves(&game).iter().map(|e| e.id).collect();
        for _ in 0..2 {
            game.step(&InputFrame::default());
            let actor = game.world.entity(id).unwrap();
            assert_eq!(actor.health, 2);
            assert_eq!(actor.state, 98);
            assert_eq!(
                leaves(&game).iter().map(|e| e.id).collect::<Vec<_>>(),
                first_ids
            );
            assert_eq!(game.player.health, 4);
        }
    }
}
