//! Configured own-Step branches; Vampire's separate point-distance path is untouched.
use super::*;
use crate::{Area, RulesPreset};
fn fixture(rules: RulesPreset, legacy: bool, gap: f64, timer: u32) -> (Game, Entity) {
    let mut g = if legacy {
        Game::legacy_shell(123, rules)
    } else {
        Game::shell(123, rules)
    };
    g.world = World::new(40, 20, Area::Mines, 1);
    for x in 0..40 {
        g.world.set_tile(x, 5, "oHardBlock");
    }
    // Zombie own moveTo refreshes x64->65 before dist. Aggregate x[0,15]
    // caches[65,80]; current frame3 ends at79. Mirrored Stand caches[P-7,P+6]
    // while the old smooth adapter begins atP-5, so its measured gap is gap+3.
    g.player.pos = Vec2::new(80.0 + gap + 7.0, 72.0);
    g.player.sprite = "sStandLeft".into();
    g.player.data.insert("image_xscale".into(), -1.0);
    let mut e = Entity::new(900_000, "oZombie", Vec2::new(64.0, 64.0));
    crate::enemies::init_entity(&mut e);
    e.animation = 3.0;
    e.vel.x = 1.0;
    e.timer = timer;
    assert!(bottom(&g.world, &e));
    assert!(!solid_at(&g.world, &e, e.pos.x + 8.0, e.pos.y + 8.0));
    (g, e)
}
#[test]
fn zombie_idle_near_and_expired_timer_gates_use_literal_cached_distances() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for legacy in [false, true] {
            for (gap, timer) in [
                (47.0, 5),
                (48.0, 5),
                (49.0, 5),
                (63.0, 0),
                (64.0, 0),
                (65.0, 0),
            ] {
                let (mut g, mut e) = fixture(rules, legacy, gap, timer);
                let literal_gap = (g.player.pos.x - 7.0 - 80.0).max(0.0);
                assert_eq!(literal_gap, gap);
                let bounce = literal_gap < 48.0 || (timer == 0 && literal_gap < 64.0);
                let rng = g.rng.state_bits();
                zombie(&mut g, &mut e);
                assert_eq!(e.pos, Vec2::new(65.0, 64.0));
                assert_eq!(e.state, i32::from(bounce));
                assert_eq!(e.timer, timer.saturating_sub(1));
                assert_eq!(e.vel, Vec2::new(0.0, 0.2));
                assert_eq!(e.sprite, "sZombieLeft");
                assert_eq!(e.animation, 3.0);
                assert_eq!(e.health, 1);
                assert_eq!(g.rng.state_bits(), rng);
                assert_eq!(
                    g.events
                        .iter()
                        .filter(
                            |ev| matches!(ev,crate::GameEvent::Sound{name,..} if name=="sndZombie")
                        )
                        .count(),
                    usize::from(bounce)
                );
            }
        }
    }
}
#[test]
fn zombie_distance_uses_active_player1_family_without_a_dead_receiver_gate() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for legacy in [false, true] {
            for (object, active, alive, bounce) in [
                ("oPlayer1", true, true, true),
                ("oPlayer1", false, true, false),
                ("oPlayer1", true, false, false),
                ("oCharacter", true, true, false),
            ] {
                let (mut g, mut e) = fixture(rules, legacy, 47.0, 5);
                g.player.engine_active = false;
                let id = g.world.spawn(object, g.player.pos);
                let t = g.world.entities.iter_mut().find(|t| t.id == id).unwrap();
                t.set_sprite("sStandLeft");
                t.data.insert("image_xscale".into(), -1.0);
                t.engine_active = active;
                t.alive = alive;
                let rng = g.rng.state_bits();
                zombie(&mut g, &mut e);
                assert_eq!(e.state, i32::from(bounce));
                assert_eq!(g.rng.state_bits(), rng);
            }
            // Negative HP on the active player is not an instance-destruction
            // predicate: original distance_to_object still sees the corpse.
            let (mut g, mut e) = fixture(rules, legacy, 47.0, 5);
            e.alive = false;
            g.player.health = -99;
            g.player.state = PlayerState::Dead;
            let rng = g.rng.state_bits();
            zombie(&mut g, &mut e);
            assert_eq!(e.state, 1);
            assert_eq!(g.rng.state_bits(), rng);
        }
    }
}
