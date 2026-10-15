//! Host-context identity controls, independent of native ID values.
use crate::native_ledger::*;

fn populated_room() -> RoomLedger<u64> {
    let mut allocator = NativeAllocator::new(CounterInput::ExactReached {
        last_native_id: 600_000,
    });
    let mut room = RoomLedger::empty_destination(RoomStorageKey(4700)).unwrap();
    room.reserve_authored_all(
        &mut allocator,
        &[TemplateActor::unmutated_compiled(
            11,
            NativeId(101),
            class_by_name("oBrick").unwrap(),
        )],
    )
    .unwrap();
    room
}

#[test]
fn captured_context_rejects_another_actual_room_lifetime_with_identical_storage_keys() {
    let first = populated_room();
    let second = populated_room();
    assert_ne!(first.lifetime().unwrap(), second.lifetime().unwrap());
    let captured = first.capture_executing_self(11).unwrap();
    let other = second.capture_executing_self(11).unwrap();
    assert_ne!(captured, other);
    assert_eq!(
        second.captured_self_record(captured),
        Err(Error::WrongRoomContext)
    );
    assert_eq!(
        first.captured_self_record(other),
        Err(Error::WrongRoomContext)
    );
    assert_eq!(
        first.captured_self_record(captured).unwrap().native_id,
        NativeId(101)
    );
}

#[test]
fn captured_context_keeps_same_room_lifetime_across_mark_and_cached_reentry() {
    let mut room = populated_room();
    let lifetime = room.lifetime().unwrap();
    room.start_entry_callbacks().unwrap();
    let captured = room.capture_executing_self(11).unwrap();
    room.mark_after_destroy_callback(11).unwrap();
    room.select_cached_destination_entry().unwrap();
    assert!(room.start_entry_callbacks().unwrap().is_none());
    assert_eq!(room.lifetime().unwrap(), lifetime);
    assert_eq!(room.capture_executing_self(11).unwrap(), captured);
    let retained = room.captured_self_record(captured).unwrap();
    assert_eq!(retained.native_id, NativeId(101));
    assert!(retained.marked108);
}
