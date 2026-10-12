//! Configured source callback controls. These are not ordinary gameplay routes.
use super::*;
use crate::{PlayerState, RulesPreset, World};
fn scene(area: Area) -> (Game, u64, u64) {
    let mut g = Game::shell(7, RulesPreset::Classic);
    g.world = World::new(20, 20, area, 5);
    g.world.name = "rLevel".into();
    g.player.pos = Vec2::new(194., 152.);
    g.player.state = PlayerState::Ducking;
    g.player.set_sprite("sDuckLeft");
    g.player.data.insert("image_xscale".into(), -1.);
    let idol = g.spawn("oGoldIdol", Vec2::new(194., 156.));
    let trap = g.spawn("oTrapBlock", Vec2::new(80., 144.));
    (g, idol, trap)
}
#[test]
fn configured_active_character_family_minimum_skips_inactive_and_retired_targets() {
    for status in ["active", "inactive", "retired"] {
        let (mut g, idol, trap) = scene(Area::Jungle);
        let other = g.spawn("oCharacter", Vec2::new(120., 152.));
        {
            let c = g.world.entity_mut(other).unwrap();
            c.set_sprite("sStandLeft");
            c.engine_active = status != "inactive";
            c.alive = status != "retired";
        }
        idol_pickup(&mut g, idol);
        let trap = g.world.entity(trap).unwrap();
        assert_eq!(
            trap.data.get("dist").copied(),
            Some(if status == "active" { 19. } else { 90. })
        );
        assert_eq!(trap.value("dying"), f64::from(status == "active"));
    }
}
#[test]
fn configured_inactive_trap_receiver_has_no_temple_body_or_distance_assignment() {
    let (mut g, idol, trap) = scene(Area::Temple);
    g.world.entity_mut(trap).unwrap().engine_active = false;
    idol_pickup(&mut g, idol);
    let trap = g.world.entity(trap).unwrap();
    assert!(!trap.data.contains_key("dist"));
    assert_eq!(trap.value("dying"), 0.);
    assert!(
        !g.events
            .iter()
            .any(|e| matches!(e,GameEvent::Sound{name,..}if name=="sndThump"))
    );
}
