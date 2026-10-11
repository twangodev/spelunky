//! Source oPlayer1 Step603–710 and993–1398; fixture-only initial rooms/inventory.
//! Every assertion follows ordinary Game::step, never an action helper.
use spelunky_core::{Area, Game, InputFrame, PlayerState, RulesPreset, Vec2, World};

fn room() -> Game {
    let mut g = Game::shell(19, RulesPreset::Classic);
    g.world = World::new(30, 20, Area::Mines, 1);
    g.world.name = "rLevel".into();
    for x in 0..30 {
        g.world.set_tile(x, 10, "oHardBlock");
    }
    g.player.pos = Vec2::new(88., 152.);
    g.player.previous_pos = g.player.pos;
    g.player.grounded = true;
    g.player.state = PlayerState::Standing;
    g.player.facing = 1;
    g.player.bombs = 4;
    g.player.ropes = 4;
    g.player.health = 20;
    g
}
fn live(g: &Game, object: &str) -> usize {
    g.world
        .entities
        .iter()
        .filter(|e| e.alive && e.object == object)
        .count()
}
fn input(item: bool, rope: bool, bomb: bool, attack: bool) -> InputFrame {
    InputFrame {
        cycle_pressed: item,
        rope_pressed: rope,
        bomb_pressed: bomb,
        attack_pressed: attack,
        attack,
        ..InputFrame::default()
    }
}
fn held(g: &mut Game, object: &str) -> u64 {
    let id = g.spawn(object, g.player.pos);
    g.world.entity_mut(id).unwrap().held = true;
    g.player.held = Some(id);
    id
}
#[test]
fn item_wins_simultaneous_bomb_and_rope() {
    for (rope, bomb) in [(false, true), (true, false), (true, true)] {
        let mut g = room();
        g.step(&input(true, rope, bomb, false));
        assert_eq!((g.player.bombs, g.player.ropes), (3, 4));
        assert_eq!((live(&g, "oBomb"), live(&g, "oRopeThrow")), (1, 0));
        let e = g
            .world
            .entity(g.player.held.expect("reserved Item bomb"))
            .unwrap();
        assert_eq!(e.object, "oBomb");
        assert_eq!(e.value("armed"), 0.);
    }
}
#[test]
fn eligible_rope_wins_bomb_and_attack() {
    let mut g = room();
    g.step(&input(false, true, true, true));
    assert_eq!((g.player.bombs, g.player.ropes), (4, 3));
    assert_eq!((live(&g, "oBomb"), live(&g, "oRopeThrow")), (0, 1));
    assert_eq!(g.player.attack_timer, 0);
    assert_eq!(live(&g, "oWhipPre"), 0);
}
#[test]
fn armed_inventory_item_noop_consumes_lower_branches() {
    let mut g = room();
    let id = held(&mut g, "oBomb");
    spelunky_core::items::explosives::arm_bomb(g.world.entity_mut(id).unwrap());
    g.step(&input(true, true, true, true));
    assert_eq!((g.player.bombs, g.player.ropes), (4, 4));
    assert_eq!(g.player.held, Some(id));
    assert_eq!((live(&g, "oBomb"), live(&g, "oRopeThrow")), (1, 0));
}
#[test]
fn unpaid_inventory_item_noop_still_consumes_attack() {
    let mut g = room();
    let keeper = g.spawn("oShopkeeper", Vec2::new(128., 144.));
    g.world
        .entity_mut(keeper)
        .unwrap()
        .text
        .insert("style".into(), "Craps".into());
    let id = held(&mut g, "oRock");
    g.world
        .entity_mut(id)
        .unwrap()
        .data
        .insert("forSale".into(), 1.);
    g.world
        .entity_mut(id)
        .unwrap()
        .data
        .insert("cost".into(), 100.);
    g.step(&input(true, false, false, true));
    assert_eq!(g.player.held, Some(id));
    assert!(g.world.entity(id).unwrap().held);
    assert_eq!((g.player.bombs, g.player.ropes), (4, 4));
}
#[test]
fn ceiling_blocked_rope_still_consumes_bomb_and_attack() {
    let mut g = room();
    g.world.set_tile(5, 8, "oHardBlock");
    g.step(&input(false, true, true, true));
    assert_eq!((g.player.bombs, g.player.ropes), (4, 4));
    assert_eq!((live(&g, "oBomb"), live(&g, "oRopeThrow")), (0, 0));
    assert_eq!(g.player.attack_timer, 0);
}
#[test]
fn ineligible_rope_falls_through_to_bomb_but_not_attack() {
    let mut g = room();
    g.player.ropes = 0;
    g.step(&input(false, true, true, true));
    assert_eq!(g.player.bombs, 3);
    assert_eq!(live(&g, "oBomb"), 1);
    assert!(g.player.held.is_none());
    assert_eq!(g.player.attack_timer, 0);
}
#[test]
fn no_inventory_resources_allow_attack_but_item_still_selects_noop() {
    for item in [false, true] {
        let mut g = room();
        g.player.bombs = 0;
        g.player.ropes = 0;
        g.step(&input(item, true, true, true));
        assert_eq!(g.player.attack_timer > 0, !item);
        // oPlayer1's pre-whip work precedes the new attack onset in this tick.
        assert_eq!(live(&g, "oWhipPre"), 0);
    }
}
#[test]
fn item_reservation_suppresses_attack_without_arming_reserved_bomb() {
    let mut g = room();
    g.step(&input(true, false, false, true));
    let e = g.world.entity(g.player.held.expect("held bomb")).unwrap();
    assert_eq!(e.value("armed"), 0.);
    assert_eq!(g.player.attack_timer, 0);
}
#[test]
fn chest_opens_before_item_selection_and_only_consumes_attack() {
    let mut g = room();
    let chest = g.spawn("oChest", g.player.pos);
    g.step(&InputFrame {
        up: true,
        ..input(true, true, true, true)
    });
    assert_eq!(g.world.entity(chest).unwrap().sprite, "sChestOpen");
    let reserved = g.player.held.expect("Item survives chest interaction");
    assert_eq!(g.world.entity(reserved).unwrap().object, "oBomb");
    // Literal source creates chest contents before reserving the Item bomb.
    assert!(
        g.world
            .entities
            .iter()
            .any(|e| e.id > chest && e.id < reserved)
    );
    assert_eq!(g.player.ropes, 4);
    assert_eq!(g.player.attack_timer, 0);
}
#[test]
fn pay_remains_independent_of_noop_inventory_selection() {
    let mut g = room();
    g.player.money = 100000;
    let keeper = g.spawn("oShopkeeper", Vec2::new(128., 144.));
    g.world
        .entity_mut(keeper)
        .unwrap()
        .text
        .insert("style".into(), "Craps".into());
    let id = held(&mut g, "oRock");
    let e = g.world.entity_mut(id).unwrap();
    e.data.insert("cost".into(), 15000.);
    e.data.insert("forSale".into(), 1.);
    g.step(&InputFrame {
        pay_pressed: true,
        ..input(true, false, false, false)
    });
    assert_eq!(g.player.money, 85000);
    assert_eq!(g.player.held, Some(id));
    assert_eq!(g.world.entity(id).unwrap().value("cost"), 0.);
}
#[test]
fn tunnelman_inventory_noop_suppresses_innate_attack() {
    let mut g = room();
    spelunky_core::characters::apply(&mut g, spelunky_core::characters::Character::TunnelMan);
    g.step(&input(true, false, false, true));
    assert_eq!(g.player.attack_timer, 0);
    assert!(!g.player.data.contains_key("character_attack_active"));
}
#[test]
fn hub_inventory_is_ineligible_but_attack_is_not() {
    let mut g = room();
    g.flags.insert("hub".into());
    g.step(&input(true, true, true, true));
    assert_eq!((g.player.bombs, g.player.ropes), (4, 4));
    assert_eq!((live(&g, "oBomb"), live(&g, "oRopeThrow")), (0, 0));
    assert!(g.player.attack_timer > 0);
}
