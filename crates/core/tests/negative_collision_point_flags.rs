//! Natural patrol steps for GM8 `collision_point(..., -1, -1)` semantics.
use spelunky_core::*;

fn patrol_scene(object: &str) -> (Game, u64) {
    for seed in 1..256 {
        let mut game = Game::shell(seed, RulesPreset::Classic);
        game.world = World::new(20, 16, Area::Mines, 1);
        game.world.name = "rLevel".into();
        // The actor stands on two tiles. A dynamic DarkFall-sprite block is
        // placed immediately below its left corner; its top-right pixel is
        // transparent but remains inside its collision bbox.
        game.world.set_tile(5, 10, "oBlock");
        game.world.set_tile(6, 10, "oBlock");
        game.player.pos = Vec2::new(280.0, 40.0);
        game.player.previous_pos = game.player.pos;
        let actor = game.spawn(object, Vec2::new(80.0, 144.0));
        {
            let actor = game.world.entity_mut(actor).unwrap();
            actor.state = 1;
            actor.facing = -1;
            actor.vel = Vec2::default();
        }
        let corner = Vec2::new(79.0, 160.0);
        let ledge = game.spawn("oBlock", Vec2::new(64.0, 160.0));
        game.world
            .entity_mut(ledge)
            .unwrap()
            .set_sprite("sDarkFall");
        let ledge = game.world.entity(ledge).unwrap();
        assert!(ledge.mask_contains(corner, CollisionPrecision::BoundingBox));
        assert!(!ledge.mask_contains(corner, CollisionPrecision::Precise));

        // All four source WALK handlers have a one-in-one-hundred stop roll.
        // Select an ordinary seed where that independent branch does not
        // mask the corner-query result.
        if game.rng.clone().range(100) != 0 {
            return (game, actor);
        }
    }
    panic!("could not choose a seed without the independent patrol stop roll");
}

#[test]
fn snake_caveman_alien_and_vampire_patrols_use_bbox_corner_queries() {
    for (object, expected_speed) in [
        ("oSnake", -1.0),
        ("oCaveman", -1.4),
        ("oAlien", -1.0),
        ("oVampire", -1.4),
    ] {
        let source = match object {
            "oSnake" => include_str!(
                "../../../references/classic-1.1/Objects/Enemies/oSnake.events/Step.xml"
            ),
            "oCaveman" => include_str!(
                "../../../references/classic-1.1/Objects/Enemies/oCaveman.events/Step.xml"
            ),
            "oAlien" => include_str!(
                "../../../references/classic-1.1/Objects/Enemies/oAlien.events/Step.xml"
            ),
            "oVampire" => include_str!(
                "../../../references/classic-1.1/Objects/Enemies/oVampire.events/Step.xml"
            ),
            _ => unreachable!(),
        };
        assert!(
            source.contains("collision_point(x-1, y+16, oSolid, -1, -1)"),
            "{object}"
        );

        let (mut game, actor) = patrol_scene(object);
        game.step(&InputFrame::default());
        let actor = game.world.entity(actor).unwrap();
        assert_eq!(actor.state, 1, "{object} must keep its source WALK state");
        assert_eq!(
            actor.facing, -1,
            "{object} must treat the transparent bbox corner as solid"
        );
        assert_eq!(actor.vel.x, expected_speed, "{object} source patrol speed");
    }
}

#[test]
fn skeleton_walk_keeps_its_active_collision_line_logic() {
    let source =
        include_str!("../../../references/classic-1.1/Objects/Enemies/oSkeleton.events/Step.xml");
    let active_walk = source
        .split("else if (status == WALK)")
        .nth(1)
        .unwrap_or("")
        .split("/*")
        .next()
        .unwrap_or("");
    assert!(!active_walk.contains("collision_point"));
    assert!(active_walk.contains("isCollisionLeft(1)"));
    assert!(active_walk.contains("isCollisionRight(1)"));
    assert!(source.contains("if (facing == LEFT and not collision_point(x-1, y, oSolid, -1, -1)"));
    // The visually similar raw probes are commented out in Classic. Rust's
    // active WALK branch is covered by its natural dynamic-wall regression.
}
