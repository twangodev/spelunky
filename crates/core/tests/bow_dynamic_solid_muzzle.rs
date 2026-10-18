//! `scrFireBow` checks collision_point(..., oSolid, 0, 0) at its muzzle.
//! This fixture uses an actual dynamic oSolid descendant whose GM `solid`
//! property is false, so the family collision query is exercised separately
//! from the native `solid` flag.
use spelunky_core::{Area, Game, InputFrame, PlayerState, RulesPreset, Vec2, World};

#[test]
fn bow_falls_back_to_player_origin_when_dynamic_solid_family_blocks_muzzle() {
    let mut game = Game::legacy_shell(42, RulesPreset::Classic);
    game.world = World::new(80, 40, Area::Mines, 1);
    game.world.name = "rLevel".into();
    for x in 0..game.world.width {
        game.world.set_tile(x, 10, "oHardBlock");
    }
    game.player.pos = Vec2::new(256.0, 152.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.player.facing = 1;
    game.world.entrance = game.player.pos;

    // oBlock is a direct oSolid child but its source GM `solid` property is
    // false. The point (player.x+14, player.y) is inside its sBlock mask.
    let blocker = game.spawn("oBlock", Vec2::new(264.0, 144.0));
    let blocker = game.world.entity(blocker).unwrap();
    assert!(
        !spelunky_core::catalog::object("oBlock").unwrap().solid,
        "FIXTURE: original GM `solid` object property is false"
    );
    assert!(
        blocker.solid,
        "runtime collision-family membership is distinct from the GM property"
    );
    assert!(spelunky_core::catalog::is_a(&blocker.object, "oSolid"));
    assert!(blocker.mask_contains(
        Vec2::new(270.0, 152.0),
        spelunky_core::CollisionPrecision::BoundingBox
    ));

    let bow = game.spawn("oBow", game.player.pos);
    game.step(&InputFrame {
        down: true,
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert_eq!(game.player.held, Some(bow), "FIXTURE: ordinary bow pickup");
    game.step(&InputFrame {
        attack_released: true,
        ..Default::default()
    });
    game.step(&InputFrame {
        attack: true,
        attack_pressed: true,
        ..Default::default()
    });
    assert_eq!(game.player.data.get("bow_armed"), Some(&1.0));
    game.step(&InputFrame {
        attack_released: true,
        ..Default::default()
    });

    let arrow = game
        .world
        .entities
        .iter()
        .find(|entity| entity.alive && entity.object == "oArrow")
        .expect("ordinary bow release creates an Arrow");
    assert_eq!(
        arrow.previous_pos, game.player.pos,
        "source scrFireBow falls back to (x,y) when its muzzle point hits any oSolid descendant"
    );
    assert_eq!(game.data["arrows"], 5.0);
}
