//! Inherited Create defaults and parentless Barrier state stay source-visible
//! through a real engine Step.
use spelunky_core::*;

const BARRIER_SOURCE: &str =
    include_str!("../../../references/classic-1.1/Objects/Enemies/oBarrier.xml");
const EMITTER_SOURCE: &str =
    include_str!("../../../references/classic-1.1/Objects/Enemies/oBarrierEmitter.xml");
const EMITTER_CREATE: &str = include_str!(
    "../../../references/classic-1.1/Objects/Enemies/oBarrierEmitter.events/Create.xml"
);
const MAGMA_SOURCE: &str =
    include_str!("../../../references/classic-1.1/Objects/Enemies/oMagma.xml");
const DETRITUS_SOURCE: &str =
    include_str!("../../../references/classic-1.1/Objects/Basis/oDetritus.xml");
const MAGMA_TRAIL_SOURCE: &str =
    include_str!("../../../references/classic-1.1/Objects/Enemies/oMagmaTrail.xml");
const DRAWN_SPRITE_CREATE: &str =
    include_str!("../../../references/classic-1.1/Objects/Basis/oDrawnSprite.events/Create.xml");

fn scene() -> Game {
    let mut game = Game::shell(42, RulesPreset::Classic);
    game.world = World::new(24, 20, Area::Temple, 1);
    game.world.name = "rLevel".into();
    game.player.pos = Vec2::new(24.0, 24.0);
    game.player.previous_pos = game.player.pos;
    game
}

fn assert_drawn_sprite_create(entity: &Entity) {
    assert_eq!(entity.text.get("type").map(String::as_str), Some(""));
    assert_eq!(entity.data.get("blinkToggle"), Some(&0.0));
}

#[test]
fn source_parents_and_inherited_defaults_match_runtime_state_after_step() {
    assert!(BARRIER_SOURCE.contains("<parent/>"));
    assert!(EMITTER_SOURCE.contains("<parent/>"));
    assert!(!EMITTER_CREATE.contains("invincible"));
    assert!(MAGMA_SOURCE.contains("<parent>oDetritus</parent>"));
    assert!(DETRITUS_SOURCE.contains("<parent>oDrawnSprite</parent>"));
    assert!(MAGMA_TRAIL_SOURCE.contains("<parent>oDrawnSprite</parent>"));
    assert!(DRAWN_SPRITE_CREATE.contains("type = \"\";\nblinkToggle = 0;"));

    let mut game = scene();
    // Keep the parentless emitter inside its source-supported Step branch.
    game.spawn("oHardBlock", Vec2::new(72.0, 56.0));
    let emitter = game.spawn("oBarrierEmitter", Vec2::new(80.0, 80.0));
    let barrier = game
        .world
        .entities
        .iter()
        .find(|entity| entity.alive && entity.object == "oBarrier")
        .expect("emitter Create synchronously makes its Barrier")
        .id;
    let magma = game.spawn("oMagma", Vec2::new(160.0, 80.0));
    let trail = game.spawn("oMagmaTrail", Vec2::new(200.0, 80.0));

    game.step(&InputFrame::default());

    for id in [barrier, emitter] {
        let entity = game
            .world
            .entity(id)
            .expect("parentless object survives Step");
        assert!(
            !entity.data.contains_key("invincible"),
            "{} has no source Create or parent assigning invincible",
            entity.object
        );
    }
    let magma = game.world.entity(magma).expect("Magma survives Step");
    assert_drawn_sprite_create(magma);
    assert_eq!(magma.data.get("invincible"), Some(&0.0));
    let trail = game.world.entity(trail).expect("MagmaTrail survives Step");
    assert_drawn_sprite_create(trail);
}
