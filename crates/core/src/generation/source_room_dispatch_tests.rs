use crate::{Game, RulesPreset};
use std::cell::RefCell;

const MINES_JUNGLE_TEMPLE_ROOMS: &str =
    include_str!("../../../../references/classic-1.1/Rooms/rLevel.xml");
const JUNGLE_LAKE_ROOMS: &str =
    include_str!("../../../../references/classic-1.1/Rooms/rLevel3.xml");
const ICE_ROOMS: &str = include_str!("../../../../references/classic-1.1/Rooms/rLevel2.xml");
const BOSS_ROOMS: &str = include_str!("../../../../references/classic-1.1/Rooms/rOlmec.xml");

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

fn source_rand(state: &mut u32, low: u32, high: u32) -> u32 {
    *state = state.wrapping_mul(0x0808_8405).wrapping_add(1);
    low + ((u64::from(*state) * u64::from(high - low + 1)) >> 32) as u32
}

fn resource_room_order(resource: &str) -> Vec<(usize, usize)> {
    let mut room_waiting_for_position = false;
    let mut result = Vec::new();
    for line in resource.lines() {
        if line.contains("<object>oRoom</object>") {
            room_waiting_for_position = true;
            continue;
        }
        if !room_waiting_for_position {
            continue;
        }
        let Some(position) = line.trim().strip_prefix("<position ") else {
            continue;
        };
        let read_attribute = |name: &str| {
            let prefix = format!("{name}=\"");
            let start = position
                .find(&prefix)
                .expect("source room position attribute")
                + prefix.len();
            position[start..]
                .split('"')
                .next()
                .expect("source room coordinate")
                .parse::<usize>()
                .expect("integer room coordinate")
        };
        let x = read_attribute("x");
        let y = read_attribute("y");
        let col = (x - 16) / 160;
        let row = if y == 592 { 3 } else { (y - 16) / 128 };
        result.push((row, col));
        room_waiting_for_position = false;
    }
    result
}

#[test]
fn main_generator_room_calls_follow_each_authored_room_resource_order() {
    let cases = [
        (1, MINES_JUNGLE_TEMPLE_ROOMS),
        (5, MINES_JUNGLE_TEMPLE_ROOMS),
        (9, ICE_ROOMS),
        (13, MINES_JUNGLE_TEMPLE_ROOMS),
        (16, BOSS_ROOMS),
    ];

    for (level, default_resource) in cases {
        let mut game = Game::shell(0, RulesPreset::Classic);
        game.world.level = level - 1;
        begin();
        game.next_level();
        let actual = finish();
        // This fixture starts the transition stream at seed zero. At level 5,
        // oTransition's source Lake roll is the first draw, before scrInitLevel.
        let mut source_rng = 0;
        let source_lake = level == 5 && source_rand(&mut source_rng, 1, 10) == 1;
        let resource = if source_lake {
            JUNGLE_LAKE_ROOMS
        } else {
            default_resource
        };
        let expected = resource_room_order(resource);

        assert_eq!(game.world.level, level);
        assert_eq!(actual, expected, "resource room order for level {level}");
    }
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
