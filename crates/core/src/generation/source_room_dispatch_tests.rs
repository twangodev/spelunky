use crate::{Game, RulesPreset};
use std::cell::RefCell;

thread_local! {
    static ROOM_DISPATCH: RefCell<Option<Vec<(usize, usize)>>> = const { RefCell::new(None) };
}

pub(super) fn observe(row: usize, col: usize) {
    ROOM_DISPATCH.with(|trace| {
        if let Some(trace) = trace.borrow_mut().as_mut() {
            trace.push((row, col));
        }
    });
}

fn begin() {
    ROOM_DISPATCH.with(|trace| *trace.borrow_mut() = Some(Vec::new()));
}

fn finish() -> Vec<(usize, usize)> {
    ROOM_DISPATCH.with(|trace| trace.borrow_mut().take().unwrap())
}

#[test]
fn natural_boss_next_level_dispatches_authored_rooms_in_resource_order() {
    let mut game = Game::shell(0, RulesPreset::Classic);
    game.world.level = 15;
    game.flags.insert("no_dark_level".into());

    begin();
    game.next_level();
    let actual = finish();

    assert_eq!(game.world.level, 16);
    assert_eq!(game.world.area, crate::Area::Boss);
    // rOlmec.xml lists static oRoom instances at resource lines 2350-2454 in
    // this order. scrInitLevel's `with oRoom` calls scrRoomGen5 synchronously;
    // its selector and tile births advance RNG, so iteration by grid position
    // changes the concrete generated room stream.
    assert_eq!(
        actual,
        [
            (0, 0),
            (0, 3),
            (1, 0),
            (1, 1),
            (1, 2),
            (1, 3),
            (2, 0),
            (2, 1),
            (2, 2),
            (2, 3),
            (0, 1),
            (0, 2),
            (3, 0),
            (3, 1),
            (3, 2),
            (3, 3),
        ]
    );
}
