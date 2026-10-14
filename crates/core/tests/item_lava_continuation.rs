//! Initially configured dormant Basketball shots below a default Lava cell.
//! Only Game::step follows setup; no receiver revival or mid-run state writes.
use spelunky_core::{
    Area, CollisionPrecision, Game, GameEvent, InputFrame, PlayerState, RulesPreset, Vec2, World,
};

fn room(
    rules: RulesPreset,
    lava: bool,
    speed: f64,
    invincible: bool,
) -> (Game, u64, u64, Option<u64>) {
    let mut game = Game::shell(42, rules);
    game.world = World::new(40, 24, Area::Mines, 1);
    game.world.name = "rMoon".into();
    game.flags.extend(["hub".into(), "hub_moon".into()]);
    for x in 0..40 {
        game.world.set_tile(x, 7, "oHardBlock");
        game.world.set_tile(x, 13, "oHardBlock");
    }
    game.player.pos = Vec2::new(24.0, 200.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.world.entrance = game.player.pos;
    game.data.insert("timer".into(), 1000.0);
    // Enemy inherited Step probes (x+8,y-1) BEFORE Snake's movement.
    // x151 places that probe159,95 outside Lava, while the later mask at
    // x154,96 still overlaps the ball's source rectangle at160,100.
    let snake = game.spawn("oSnake", Vec2::new(151.0, 96.0));
    // A source-reachable last IDLE counter suppresses unrelated facing RNG;
    // source Create HP1 and velocity2.5 remain unchanged.
    game.world.entity_mut(snake).unwrap().timer = 1;
    game.world
        .entity_mut(snake)
        .unwrap()
        .data
        .insert("invincible".into(), f64::from(invincible));
    let lava = lava.then(|| game.spawn("oLava", Vec2::new(160.0, 80.0)));
    let ball = game.spawn("oBasketball", Vec2::new(160.0 - speed, 100.0));
    game.world.entity_mut(ball).unwrap().vel.x = speed;
    assert_eq!(game.player.health, 4);
    assert_eq!(game.world.entity(snake).unwrap().health, 1);
    assert!(
        !game.world.fluid_dirty,
        "no destroyed solid requests drainage"
    );
    if let Some(lava) = lava {
        let mask = game.world.entity(lava).unwrap();
        assert_eq!(mask.sprite, "sLava");
        let target = game.world.entity(snake).unwrap();
        assert!(
            !mask.mask_contains(
                Vec2::new(target.pos.x + 8.0, target.pos.y - 1.0),
                CollisionPrecision::BoundingBox,
            ),
            "Enemy inherited Lava probe must leave the creature alive for the Item query"
        );
        assert!(mask.mask_contains(Vec2::new(160.0, 95.0), CollisionPrecision::BoundingBox));
        assert!(
            !mask.mask_intersects(
                spelunky_core::Rect::new(157.0, 97.0, 7.0, 7.0),
                CollisionPrecision::BoundingBox
            ),
            "retirement point hits, velocity damping rectangle misses"
        );
    }
    (game, ball, snake, lava)
}

fn hit_sounds(game: &Game) -> usize {
    game.events
        .iter()
        .filter(|event| {
            matches!(event,
        GameEvent::Sound { name, .. } if name == "sndHit")
        })
        .count()
}

#[test]
fn lava_destroyed_item_finishes_inherited_creature_impact_without_reviving() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let (mut game, ball, snake, lava) = room(rules, true, 3.0, false);
        let lava = lava.unwrap();
        game.step(&InputFrame::default());
        assert!(
            game.world.entity(ball).is_none(),
            "source Item Lava Destroy remains retired"
        );
        assert!(
            !game.world.entities.iter().any(|entity| entity.id == ball),
            "endframe removes the dead receiver"
        );
        let target = game
            .world
            .entity(snake)
            .expect("HP subtraction is after Snake's own Step");
        assert_eq!(
            target.state, 0,
            "incoming last IDLE counter reaches zero, not a new AI state"
        );
        assert_eq!(target.timer, 0);
        assert!((154.0..=155.0).contains(&target.pos.x));
        assert_eq!(target.pos.y, 96.0);
        assert!(
            target.mask_contains(Vec2::new(160.0, 100.0), CollisionPrecision::BoundingBox),
            "real post-movement enemy mask qualifies the inherited rectangle"
        );
        assert_eq!(game.world.entity(lava).unwrap().pos, Vec2::new(160.0, 80.0));
        assert_eq!(game.world.entity(lava).unwrap().value("spurt"), 0.0);
        assert_eq!(game.player.health, 4);
        assert_eq!(game.player.held, None);
        assert_eq!(target.health, 0, "POST_LAVA_DESTROY_ITEM_IMPACT {rules:?}");
        assert!(
            (target.vel.x - 0.9).abs() < 1e-12,
            "source post-with xVel=3*.3"
        );
        assert_eq!(hit_sounds(&game), 1);
        let blood: Vec<_> = game
            .world
            .entities
            .iter()
            .filter(|e| e.alive && e.object == "oBlood")
            .collect();
        assert_eq!(
            blood.len(),
            1,
            "one inherited impact producer, no death-body triple blood yet"
        );
        assert_eq!(
            blood[0].pos,
            Vec2::new(target.pos.x + 8.0, target.pos.y + 8.0)
        );
    }
}

#[test]
fn lava_continuation_preserves_velocity_threshold_invincibility_and_live_control() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (lava, speed, invincible, hit) in [
            (true, 2.0, false, false),
            (true, 3.0, true, false),
            (false, 3.0, false, true),
        ] {
            let (mut game, ball, snake, _) = room(rules, lava, speed, invincible);
            game.step(&InputFrame::default());
            assert_eq!(game.world.entity(ball).is_none(), lava);
            let target = game.world.entity(snake).unwrap();
            assert_eq!(target.pos.y, 96.0);
            assert!(target.mask_contains(Vec2::new(160.0, 100.0), CollisionPrecision::BoundingBox));
            assert_eq!(target.health, if hit { 0 } else { 1 });
            assert_eq!(hit_sounds(&game), usize::from(hit));
            assert_eq!(
                game.world
                    .entities
                    .iter()
                    .filter(|e| e.alive && e.object == "oBlood")
                    .count(),
                usize::from(hit)
            );
            assert_eq!(game.player.health, 4);
        }
    }
}
