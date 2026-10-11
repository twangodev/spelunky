use crate::{Area, Game, GameEvent, InputFrame, RulesPreset, Vec2, World};

#[test]
fn actual_fire_frog_bomb_alarm_creates_player_blood_after_velocity_rng() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut g = Game::shell(7, rules);
        g.world = World::new(30, 20, Area::Mines, 1);
        g.world.name = "rLevel".into();
        for x in 0..30 { g.world.set_tile(x, 7, "oHardBlock"); }
        g.player.pos = Vec2::new(80.0, 104.0);
        g.player.previous_pos = g.player.pos;
        g.player.grounded = true;
        let bomb = g.spawn("oFireFrogBomb", Vec2::new(88.0, 104.0));
        let mut reached = false;
        for _ in 0..130 {
            let first = g.world.next_id;
            let before = g.clone();
            g.step(&InputFrame::default());
            if !g.events.contains(&GameEvent::Damage { amount: 10 }) { continue; }
            reached = true;
            assert_eq!(before.world.entity(bomb).expect("hit must be on the natural Alarm frame").alarms[1], 1);
            assert!(!before.flags.contains("low_graphics"));
            assert_eq!(before.world.entities.iter().filter(|e| e.alive && e.engine_active && crate::catalog::is_a(&e.object, "oDetritus")).count(), 0);
            assert!(g.world.entity(bomb).is_none(), "hit follows natural Alarm1");
            assert_eq!(g.world.entities.iter().filter(|e| e.alive && e.id >= first && e.object == "oBlood").count(), 7, "Bomb Blood3, Explosion hurt Blood1, then grounded stun-bounce Blood3");
            assert_eq!(g.world.entities.iter().filter(|e| e.alive && e.id >= first && e.object == "oFlame").count(), 3);
            assert_eq!(g.player.health, 0);
            assert_eq!(g.player.data.get("bounced"), Some(&1.0));
            let mut oracle = before.rng.clone();
            // Literal own FireFrogBomb Alarm1: three Blood then three Flame;
            // each admitted synchronous Create consumes four ordered draws.
            for _ in 0..24 { oracle.next_u64(); }
            let _speed = 4 + oracle.next_u64() % 3;
            for _ in 0..4 { oracle.next_u64(); }
            // Source Step1895 then emits three more script Blood particles:
            // first-frame hurt is stunned, still grounded and not yet bounced.
            for _ in 0..12 { oracle.next_u64(); }
            assert_eq!(g.rng.clone().next_u64(), oracle.next_u64());
            break;
        }
        assert!(reached, "default armed Bomb must naturally explode on player");
    }
}

#[test]
fn naturally_explosion_killed_player_still_receives_source_explosion_callback() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut g = Game::shell(7, rules);
        g.world = World::new(30, 20, Area::Mines, 1);
        g.world.name = "rLevel".into();
        for x in 0..30 { g.world.set_tile(x, 7, "oHardBlock"); }
        g.player.pos = Vec2::new(80.0, 104.0);
        g.player.previous_pos = g.player.pos;
        g.player.grounded = true;
        g.spawn("oFireFrogBomb", Vec2::new(88.0, 104.0));
        let mut reached = false;
        for _ in 0..130 {
            g.step(&InputFrame::default());
            if !g.events.contains(&GameEvent::Damage { amount: 10 }) { continue; }
            reached = true;
            break;
        }
        assert!(reached);
        assert_eq!(g.state, crate::RunState::Dead);
        assert_eq!(g.player.health, 0);
        // Supplemental callback-context witness preserves the naturally born,
        // still-overlapping blast and stops before later corpse movement.
        let mut callback = g.clone();
        let first = callback.world.next_id;
        let mut oracle = callback.rng.clone();
        let speed = (4 + oracle.next_u64() % 3) as f64;
        for _ in 0..4 { oracle.next_u64(); }
        super::player_explosion_hurt(&mut callback);
        assert_eq!(callback.player.health, -10, "source Explosion has no dead guard");
        assert_eq!(callback.player.vel.x.abs(), speed);
        assert_eq!(callback.world.entities.iter().filter(|e| e.alive && e.id >= first && e.object == "oBlood").count(), 1);
        assert_eq!(callback.rng.clone().next_u64(), oracle.next_u64());
    }
}
