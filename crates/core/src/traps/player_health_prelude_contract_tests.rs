use crate::{Area, Game, GameEvent, InputFrame, Rng, RulesPreset, Vec2, World};

fn lava_room(rules: RulesPreset) -> Game {
    let mut game = Game::shell(7, rules);
    game.world = World::new(30, 20, Area::Mines, 1);
    game.world.name = "rLevel".into();
    for x in 0..30 { game.world.set_tile(x, 10, "oHardBlock"); }
    // The initial solid sides and floor retain the source fluid cell.
    game.world.set_tile(4, 9, "oHardBlock");
    game.world.set_tile(6, 9, "oHardBlock");
    game.world.set_tile(5, 9, "oLava");
    game.player.pos = Vec2::new(88.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.grounded = true;
    game
}

fn literal_blood(rng: &mut Rng) -> (Vec2, f64) {
    fn random(rng: &mut Rng, n: f64) -> f64 {
        ((rng.next_u64() >> 11) as f64 / (1_u64 << 53) as f64) * n
    }
    (Vec2::new(random(rng, 4.0) - random(rng, 4.0), -1.0 - random(rng, 2.0)),
        (1 + rng.next_u64() % 6) as f64 * 0.1)
}

#[test]
fn actual_lava_corpse_crosses_visibility_threshold_once_and_floors_late_health() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        let mut game = lava_room(rules);
        let mut deaths = 0;
        for step in 1..=105 {
            let feet = Vec2::new(game.player.pos.x, game.player.pos.y + 6.0);
            assert!((80.0..96.0).contains(&feet.x) && (144.0..160.0).contains(&feet.y), "natural corpse must stay in the authored Lava");
            let first = game.world.next_id;
            game.step(&InputFrame::default());
            deaths += game.events.iter().filter(|e| **e == GameEvent::Died).count();
            assert!(game.events.contains(&GameEvent::Damage { amount: 99 }));
            let expected = if step == 1 { 0 } else { (-99 * (step - 1)).max(-10000) };
            assert_eq!(game.player.health, expected, "source late health floor on step {step}");
            assert_eq!(game.player.data.get("hidden").copied().unwrap_or(0.0), if step < 3 { 0.0 } else { 1.0 });
            if step > 3 {
                assert!(!game.world.entities.iter().any(|e| e.id >= first && e.object == "oBlood"), "hidden corpse cannot request another gib burst");
            }
        }
        assert_eq!(deaths, 1);
        // Blood may retire in inherited Detritus Step inside Lava. Its creation
        // and synchronous RNG are tested before that later phase below.
    }
}

#[test]
fn natural_lava_threshold_callback_creates_capped_blood_synchronously_and_hides_once() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for (existing, bloodless) in [(0, 0.0), (15, 0.0), (16, 0.0), (0, 2.0), (0, -1.0)] {
            let mut game = lava_room(rules);
            game.step(&InputFrame::default());
            game.step(&InputFrame::default());
            assert_eq!(game.player.health, -99);
            // Supplemental callback-context witness from a naturally reached
            // corpse; stop before inherited particle Steps can destroy Blood.
            for _ in 0..existing { game.spawn("oBlood", Vec2::new(240.0, 80.0)); }
            game.player.data.insert("bloodless".into(), bloodless);
            super::player_lava(&mut game);
            assert_eq!(game.player.health, -198);
            let first = game.world.next_id;
            let admitted = if bloodless != 0.0 { 0 } else { (16_usize - existing).min(3) };
            let mut oracle = game.rng.clone();
            let expected: Vec<_> = (0..admitted).map(|_| literal_blood(&mut oracle)).collect();
            super::player_health_prelude(&mut game);
            let born: Vec<_> = game.world.entities.iter().filter(|e| e.alive && e.id >= first && e.object == "oBlood").collect();
            assert_eq!(born.len(), admitted);
            for (blood, (velocity, gravity)) in born.iter().zip(expected) {
                assert_eq!(blood.pos, game.player.pos);
                assert_eq!(blood.vel, velocity);
                assert_eq!(blood.value("gravity"), gravity);
                assert_eq!(blood.value("world_effect_initialized"), 1.0);
            }
            assert_eq!(game.rng.clone().next_u64(), oracle.next_u64());
            assert_eq!(game.player.data.get("hidden"), Some(&1.0));
            let mut unchanged_rng = game.rng.clone();
            let next_id = game.world.next_id;
            super::player_health_prelude(&mut game);
            assert_eq!(game.world.next_id, next_id);
            assert_eq!(game.rng.clone().next_u64(), unchanged_rng.next_u64());
        }
    }
}

#[test]
fn health_floor_is_independent_of_visibility_and_exiting_and_ankh_undoes_new_hide() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        // Explicit numeric/phase controls, not natural-trajectory claims.
        for hidden in [false, true] {
            let mut game = lava_room(rules);
            game.player.health = -10001;
            game.player.set_sprite("sPExit");
            if hidden { game.player.data.insert("hidden".into(), 1.0); }
            else { game.player.data.insert("visible".into(), 0.0); }
            let first = game.world.next_id;
            let mut unchanged_rng = game.rng.clone();
            super::player_health_prelude(&mut game);
            assert_eq!(game.player.health, -10000);
            assert_eq!(game.world.next_id, first);
            assert_eq!(game.rng.clone().next_u64(), unchanged_rng.next_u64());
        }
        let mut game = lava_room(rules);
        game.player.equipment.insert("Ankh".into());
        game.player.health = -198;
        super::player_health_prelude(&mut game);
        assert_eq!(game.player.data.get("hidden"), Some(&1.0));
        crate::progression::player_revival(&mut game);
        assert_eq!(game.player.health, 4);
        assert_eq!(game.player.data.get("hidden"), None);
        assert_ne!(game.player.data.get("visible"), Some(&0.0));
        assert_eq!(game.player.data.get("active"), Some(&1.0));
        assert!(!game.player.has("Ankh"));
    }
}
