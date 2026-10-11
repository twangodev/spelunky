//! Source-derived preparation only: UNCOMPILED_UNEXECUTED.
//! Room, pickup, and inputs establish every Player/equipment/fall/stun state.
//! Three event-boundary tests explicitly add arriving controlled world hazards.
use spelunky_core::*;

fn count(game: &Game, object: &str) -> usize {
    game.world.entities.iter().filter(|entity| entity.alive && entity.engine_active && entity.object == object).count()
}
fn field(game: &Game, key: &str) -> f64 {
    game.player.data.get(key).copied().unwrap_or(0.0)
}
fn diagnostic(game: &Game) -> String {
    format!("tick={} pos={:?} vel={:?} hp={} state={:?} grounded={} stun={} fall={} Para={} Used={}", game.tick, game.player.pos, game.player.vel, game.player.health, game.player.state, game.player.grounded, game.player.stun, field(game,"fall_timer"), count(game,"oParachute"), count(game,"oParaUsed"))
}

fn acquired_before_natural_drop(rules: RulesPreset, exits: bool) -> Game {
    let mut level = level::Level::default();
    level.set_tile(8, 4, '@').unwrap();
    for x in 6..=10 { level.set_tile(x, 5, '1').unwrap(); }
    for x in 0..40 {
        level.set_tile(x, 16, '1').unwrap();
        if exits { level.set_tile(x, 15, 'X').unwrap(); }
    }
    let mut game = Game::shell(11, rules);
    game.load_custom(&level, "parachute-active-prelude", "");
    // Initial free pickup fixture, before the first simulated Step. Unlike an
    // airborne pickup, Down+Attack below runs with a genuinely grounded Player.
    let pickup = game.spawn("oParaPickup", Vec2::new(game.player.pos.x, game.player.pos.y + 4.0));
    let item = game.world.entity_mut(pickup).unwrap();
    item.data.insert("cost".into(), 0.0);
    item.data.insert("forSale".into(), 0.0);
    for _ in 0..8 { game.step(&InputFrame::default()); }
    assert!(game.player.grounded, "INITIAL_GROUND: {}", diagnostic(&game));
    assert!(!game.player.has("Parachute"));
    game.step(&InputFrame { down:true, attack:true, attack_pressed:true, ..Default::default() });
    assert!(game.player.has("Parachute"), "ACTUAL_PICKUP: {}", diagnostic(&game));
    assert!(game.world.entity(pickup).is_none());
    game.step(&InputFrame::default());
    for _ in 0..90 {
        game.step(&InputFrame { right:true, ..Default::default() });
        if !game.player.grounded && game.player.state == PlayerState::Falling { break }
    }
    assert!(!game.player.grounded, "ACTUAL_WALK_OFF: {}", diagnostic(&game));
    for _ in 0..40 {
        if field(&game,"fall_timer") == 14.0 { break }
        game.step(&InputFrame::default());
    }
    assert_eq!(field(&game,"fall_timer"), 14.0, "PRE_DEPLOY: {}", diagnostic(&game));
    assert!(game.player.vel.y > 0.0);
    assert!(game.player.has("Parachute"));
    assert!(!game.player.is_stunned());
    assert_eq!(game.player.health, 4);
    assert_eq!(count(&game,"oParachute"), 0);
    assert!(!game.world.solid_at(game.player.pos.x, game.player.pos.y + 32.0));
    game
}

fn deploy(game: &mut Game) {
    game.step(&InputFrame::default());
    assert!(!game.player.has("Parachute"));
    assert_eq!(count(game,"oParachute"),1,"DEPLOY: {}",diagnostic(game));
    assert_eq!(count(game,"oParaUsed"),0);
    assert_eq!(field(game,"fall_timer"),0.0);
    assert_eq!(game.player.health,4);
}

#[test]
fn natural_acquisition_fall_landing_and_fresh_jump_leave_used_canopy_at_pre_movement_origin() {
    for rules in [RulesPreset::Classic,RulesPreset::Assisted] {
        let mut game = acquired_before_natural_drop(rules,false);
        deploy(&mut game);
        for _ in 0..700 {
            if game.player.grounded { break }
            game.step(&InputFrame::default());
        }
        assert!(game.player.grounded,"NATURAL_LANDING: {}",diagnostic(&game));
        assert_eq!(game.player.health,4);
        assert_eq!(count(&game,"oParachute"),1);
        let origin = game.player.pos;
        game.step(&InputFrame { jump:true,jump_pressed:true,..Default::default() });
        assert!(game.player.pos.y < origin.y,"FRESH_JUMP: {}",diagnostic(&game));
        assert!(game.player.vel.y < 0.0);
        assert_eq!(count(&game,"oParachute"),0);
        assert_eq!(count(&game,"oParaUsed"),1);
        let used = game.world.entities.iter().find(|entity| entity.alive && entity.object=="oParaUsed").unwrap();
        assert_eq!(used.pos,Vec2::new(origin.x-8.0,origin.y-24.0));
        assert_eq!(game.player.health,4);
    }
}

#[test]
fn natural_deployment_with_controlled_arriving_arrow_creates_one_used_in_the_same_step() {
    for rules in [RulesPreset::Classic,RulesPreset::Assisted] {
        let mut game = acquired_before_natural_drop(rules,false);
        // Source-valid arriving Arrow fixture isolates own-Step hurt on the
        // natural deployment frame. This does not claim a trap firing route.
        let arrow = game.spawn("oArrow",Vec2::new(game.player.pos.x,game.player.pos.y+5.0));
        let projectile = game.world.entity_mut(arrow).unwrap();
        projectile.vel = Vec2::new(6.0,0.0);
        projectile.data.insert("safe".into(),0.0);
        game.step(&InputFrame::default());
        assert_eq!(game.player.health,2,"OWN_HURT: {}",diagnostic(&game));
        assert!(game.player.is_stunned());
        assert_eq!(game.player.stun,20);
        assert!(!game.player.has("Parachute"));
        assert!(game.world.entity(arrow).is_none());
        assert_eq!(count(&game,"oParachute"),0);
        assert_eq!(count(&game,"oParaUsed"),1);
        let used = game.world.entities.iter().find(|entity| entity.alive && entity.object=="oParaUsed").unwrap();
        assert_eq!(used.pos,Vec2::new(game.player.pos.x-8.0,game.player.pos.y-24.0));
        let blood:Vec<_> = game.world.entities.iter().filter(|entity| entity.object=="oBlood").collect();
        assert_eq!(blood.len(),3);
        assert!(blood.iter().all(|entity| entity.id < used.id));
    }
}

#[test]
fn natural_deployment_with_controlled_arriving_yeti_removes_before_throw_motion_and_recovers() {
    for rules in [RulesPreset::Classic,RulesPreset::Assisted] {
        let mut game = acquired_before_natural_drop(rules,false);
        deploy(&mut game);
        // This arriving live Yeti fixture matches its non-stomp throw overlap.
        // Its center is left of the Player, so the real throw travels right.
        // No Player/health/velocity/timer/equipment state is assigned here.
        let pos=game.player.pos;
        let yeti=game.spawn("oYeti",Vec2::new(pos.x-12.0,pos.y-14.0));
        game.step(&InputFrame::default());
        assert!(game.player.is_stunned(),"REAL_YETI_THROW: {}",diagnostic(&game));
        assert_eq!(game.player.stun,0);
        assert_eq!(game.player.vel.y,-6.0);
        assert_eq!(count(&game,"oParachute"),1);
        let origin=game.player.pos;
        game.step(&InputFrame::default());
        assert_eq!(count(&game,"oParachute"),0);
        assert_eq!(count(&game,"oParaUsed"),1);
        assert_ne!(game.player.pos,origin);
        let used=game.world.entities.iter().find(|entity| entity.alive && entity.object=="oParaUsed").unwrap();
        assert_eq!(used.pos,Vec2::new(origin.x-8.0,origin.y-24.0));
        for _ in 0..300 {
            if game.player.grounded && !game.player.is_stunned() { break }
            game.step(&InputFrame::default());
        }
        assert!(game.player.grounded && !game.player.is_stunned(),"RECOVERY: {}",diagnostic(&game));
        assert!(game.world.entity(yeti).is_some(),"thrower remains live");
        let before=game.player.pos;
        for _ in 0..8 { game.step(&InputFrame {right:true,..Default::default()}); }
        assert!(game.player.pos.x>before.x);
        let before=game.player.pos;
        game.step(&InputFrame {jump:true,jump_pressed:true,..Default::default()});
        assert!(game.player.pos.y<before.y && game.player.vel.y<0.0);
    }
}

#[test]
fn natural_deployment_with_controlled_arriving_lava_preserves_canopy_until_prior_dead_cleanup() {
    for rules in [RulesPreset::Classic,RulesPreset::Assisted] {
        let mut game=acquired_before_natural_drop(rules,false);
        // Arriving Lava at the source early feet-point, before active prelude.
        // Fatal life alone does not become the source dead flag until DIED.
        game.spawn("oLava",Vec2::new(game.player.pos.x-8.0,game.player.pos.y+5.0));
        game.step(&InputFrame::default());
        assert_eq!(game.player.health,0);
        assert_eq!(game.state,RunState::Dead);
        assert!(!game.player.has("Parachute"));
        assert_eq!(count(&game,"oParachute"),1,"FIRST_FATAL_STEP: {}",diagnostic(&game));
        assert_eq!(count(&game,"oParaUsed"),0);
        game.step(&InputFrame::default());
        assert_eq!(count(&game,"oParachute"),0);
        assert_eq!(count(&game,"oParaUsed"),1,"PRIOR_DEAD: {}",diagnostic(&game));
    }
}

#[test]
fn natural_deployment_landing_and_actual_exit_keep_canopy_while_controller_inactive() {
    for rules in [RulesPreset::Classic,RulesPreset::Assisted] {
        let mut game=acquired_before_natural_drop(rules,true);
        deploy(&mut game);
        for _ in 0..700 {
            game.step(&InputFrame {up:true,..Default::default()});
            if game.player.is_exiting() { break }
        }
        assert!(game.player.is_exiting(),"ACTUAL_EXIT: {}",diagnostic(&game));
        assert_eq!(game.player.data.get("active"),Some(&0.0));
        assert_eq!(count(&game,"oParachute"),1);
        assert_eq!(count(&game,"oParaUsed"),0);
        game.step(&InputFrame::default());
        assert!(game.player.is_exiting());
        assert_eq!(count(&game,"oParachute"),1);
        assert_eq!(count(&game,"oParaUsed"),0);
        assert_eq!(game.player.health,4);
    }
}
