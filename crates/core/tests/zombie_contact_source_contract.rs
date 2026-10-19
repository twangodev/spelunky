use spelunky_core::{Area, Game, InputFrame, PlayerState, RulesPreset, Vec2, World};

const SOURCE_CHARACTER_COLLISION: &str = include_str!(
    "../../../references/classic-1.1/Objects/Basis/oEnemy.events/Collision with oCharacter.xml"
);

#[test]
fn grounded_nonstomp_zombie_contact_matches_horizontal_only_source_contract() {
    let (_, nonstomp) = SOURCE_CHARACTER_COLLISION
        .split_once("else if (other.invincible == 0)")
        .expect("SOURCE_PARENT_NONSTOMP_CONTACT_BRANCH");
    let nonstomp = nonstomp
        .split_once("</argument>")
        .expect("SOURCE_PARENT_EVENT_BODY_END")
        .0;
    assert!(nonstomp.contains("other.invincible = 30;"));
    assert!(nonstomp.contains("other.xVel = -6"));
    assert!(nonstomp.contains("other.xVel = 6"));
    assert!(
        !nonstomp.contains("other.yVel"),
        "SOURCE_PARENT_NONSTOMP_HAS_NO_VERTICAL_KNOCKBACK"
    );

    let mut game = Game::shell(42, RulesPreset::Classic);
    game.world = World::new(20, 16, Area::Mines, 1);
    game.world.name = "rLevel".into();
    for x in 0..game.world.width {
        game.world.set_tile(x, 10, "oBlock");
    }
    // Keep the player grounded on a narrow ledge to the Zombie's right. The
    // inherited source callback does not classify Standing as a stomp.
    game.world.set_tile(7, 9, "oBlock");
    game.player.pos = Vec2::new(109.0, 136.0);
    game.player.previous_pos = game.player.pos;
    game.player.state = PlayerState::Standing;
    game.player.grounded = true;
    game.player.vel = Vec2::default();
    let zombie = game.spawn("oZombie", Vec2::new(96.0, 142.0));
    assert_eq!(game.world.entity(zombie).unwrap().health, 1);
    assert!(game.player.pos.y < game.world.entity(zombie).unwrap().pos.y);

    game.step(&InputFrame::default());

    assert_eq!(game.player.health, 3.0, "Zombie contact deals one damage");
    assert_eq!(
        game.world.entity(zombie).unwrap().health,
        1,
        "contact is not a stomp"
    );
    assert_eq!(game.player.invincible, 30);
    assert_eq!(game.player.vel.x, 6.0);
    assert_eq!(
        game.player.vel.y, 0.0,
        "SOURCE_NONSTOMP_NO_VERTICAL_KNOCKBACK"
    );
}
