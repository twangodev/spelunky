//! Default-Create authored narrow-corridor witnesses, plus one-sided controls.
//! Actor and PushBlock origins are authored initial coordinates. No post-Create
//! velocity/state writes; this is not a claim about generated room frequency.
use spelunky_core::*;
fn corridor(kind: &str, left: bool, right: bool, rules: RulesPreset) {
    let mut level = level::Level::default();
    level.set_tile(14, 8, '@').unwrap();
    level.set_tile(14, 9, '1').unwrap();
    let mut game = Game::shell(42, rules);
    game.load_custom(&level, "source-both-side-items", "");
    // Full floor supports the default movable walls, below all receiver bodies.
    for x in 4..=6 {
        game.world.set_tile(x, 5, "oBlock");
    }
    let half = if kind == "oDice" { 6.0 } else { 4.0 };
    let mut walls = Vec::new();
    if left {
        walls.push(game.spawn("oPushBlock", Vec2::new(88.0 - half - 16.0, 64.0)));
    }
    if right {
        walls.push(game.spawn("oPushBlock", Vec2::new(88.0 + half, 64.0)));
    }
    let item = game.spawn(
        kind,
        Vec2::new(88.0, if kind == "oDice" { 68.0 } else { 72.0 }),
    );
    assert_eq!(game.world.entity(item).unwrap().vel, Vec2::default());
    let wall_poses: Vec<_> = walls
        .iter()
        .map(|id| (*id, game.world.entity(*id).unwrap().pos))
        .collect();
    game.step(&InputFrame::default());
    let e = game.world.entity(item).unwrap();
    assert!(e.alive);
    assert_eq!(e.value("colLeft"), f64::from(left));
    assert_eq!(e.value("colRight"), f64::from(right));
    assert_eq!(e.value("colBot"), 0.0);
    assert_eq!(e.value("colTop"), 0.0);
    // The OUTER left branch wins. Its inner !right controls the nudge only;
    // both true keeps X while retaining the fragile receiver's yVel=0.
    let x = 88.0
        + if left && !right {
            1.0
        } else if !left && right {
            -1.0
        } else {
            0.0
        };
    assert_eq!(
        e.pos.x, x,
        "SOURCE_OUTER_LEFT_PRECEDENCE {kind}/{rules:?}/{left}/{right}"
    );
    assert_eq!(e.pos.y, if kind == "oDice" { 68.0 } else { 72.0 });
    let fragile = kind == "oJar" || kind == "oSkull";
    assert_eq!(e.vel.y, if fragile && (left || right) { 0.0 } else { 0.6 });
    for (id, pos) in wall_poses {
        assert_eq!(game.world.entity(id).unwrap().pos, pos);
    }
    assert_eq!(game.player.health, 4);
}
#[test]
fn authored_rock_keeps_center_between_two_solid_walls() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        corridor("oRock", true, true, rules);
    }
}
#[test]
fn authored_fragile_items_keep_center_between_two_solid_walls() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for kind in ["oJar", "oSkull"] {
            corridor(kind, true, true, rules);
        }
    }
}
#[test]
fn authored_dice_keeps_center_between_two_solid_walls() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        corridor("oDice", true, true, rules);
    }
}
#[test]
fn one_sided_and_clear_item_contacts_keep_source_nudges() {
    for rules in [RulesPreset::Classic, RulesPreset::Assisted] {
        for kind in ["oRock", "oJar", "oSkull", "oDice"] {
            for (left, right) in [(false, false), (true, false), (false, true)] {
                corridor(kind, left, right, rules);
            }
        }
    }
}
