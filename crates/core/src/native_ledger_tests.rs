use crate::native_ledger::*;
use crate::nearest_ledger::*;
use crate::nearest_numeric::admit_control_word;
use std::collections::BTreeMap;
fn c(n: &str) -> ClassId {
    class_by_name(n).unwrap()
}
fn room(n: u64) -> RoomLedger<u64> {
    RoomLedger::empty_destination(RoomStorageKey(n)).unwrap()
}
fn a(key: u64, id: i32, name: &str) -> TemplateActor<u64> {
    TemplateActor::unmutated_compiled(key, NativeId(id), c(name))
}
#[test]
fn authored_reserve_all_counter_neutral_then_runtime_before_callback() {
    let mut alloc = NativeAllocator::new(CounterInput::CanonicalAssetLoaderCompleted);
    let mut r = room(1);
    r.reserve_authored_all(
        &mut alloc,
        &[
            a(701, 101, "oGame"),
            a(990, 103, "oPlayer1"),
            a(802, 102, "oLevel"),
        ],
    )
    .unwrap();
    assert_eq!(alloc.last_native_id(), NativeId(135506));
    assert_eq!(r.active_count().unwrap(), 3);
    // First actual callback (Game) can already query later Player/Level rows.
    assert_eq!(r.family_slot(c("oPlayer1").raw(), 0).unwrap(), Some(990));
    assert_eq!(r.record(701).unwrap().native_id, NativeId(101));
    let mut cursor = r.start_entry_callbacks().unwrap().unwrap();
    assert_eq!(cursor.bound(), 3);
    let id = r
        .reserve_runtime_before_create(&mut alloc, 17, c("oBrick"))
        .unwrap();
    assert_eq!(id, NativeId(135507));
    assert_eq!(r.family_slot(c("oSolid").raw(), 0).unwrap(), Some(17));
    let mut visited = vec![];
    while let Some(slot) = cursor.next(&r).unwrap() {
        visited.push(slot.key)
    }
    assert_eq!(visited, vec![701, 990, 802]);
}
#[test]
fn explicit_counter_wrap_and_storage_reuse_guard() {
    let mut alloc = NativeAllocator::new(CounterInput::ExactReached {
        last_native_id: i32::MAX,
    });
    let mut r = room(1);
    assert_eq!(
        r.reserve_runtime_before_create(&mut alloc, 7, c("oBrick"))
            .unwrap(),
        NativeId(i32::MIN)
    );
    assert_eq!(
        r.reserve_runtime_before_create(&mut alloc, 7, c("oBrick")),
        Err(Error::DuplicateStorageKey)
    );
    assert_eq!(alloc.last_native_id(), NativeId(i32::MIN));
    r.mark_without_callback(7).unwrap();
    r.cleanup_active_with(|_| Ok::<_, ()>(())).unwrap();
    assert_eq!(
        r.reserve_runtime_before_create(&mut alloc, 7, c("oBrick")),
        Err(Error::DuplicateStorageKey)
    );
}
#[test]
fn own_persistent_default_is_not_inherited_and_inactive_clone_preserves_bits() {
    let mut alloc = NativeAllocator::new(CounterInput::ExactReached {
        last_native_id: 200000,
    });
    let mut r = room(1);
    let mut x = a(1, 9, "oEnemy");
    x.membership = Membership::Inactive;
    x.pending_inactive109 = true;
    x.marked108 = true;
    r.reserve_authored_all(&mut alloc, &[x, a(2, 10, "oCaveman")])
        .unwrap();
    assert!(r.record(1).unwrap().persistent);
    assert!(!r.record(2).unwrap().persistent);
    assert_eq!(r.inactive_keys().unwrap(), &[1]);
    assert_eq!(r.family_count(c("oEnemy").raw()).unwrap(), 1);
    assert_eq!(r.family_slot(c("oEnemy").raw(), 0).unwrap(), Some(2));
    assert!(r.record(1).unwrap().marked108 && r.record(1).unwrap().pending_inactive109);
}
#[test]
fn draw_mutates_global_only_then_birth_old_family_then_conditional_cleanup_rebuild() {
    let mut alloc = NativeAllocator::new(CounterInput::ExactReached {
        last_native_id: 200000,
    });
    let mut r = room(1);
    r.reserve_authored_all(
        &mut alloc,
        &[a(1, 11, "oBrick"), a(2, 12, "oBrick"), a(3, 13, "oBrick")],
    )
    .unwrap();
    r.publish_draw_active_permutation(vec![3, 2, 1]).unwrap();
    assert_eq!(r.family_slot(c("oSolid").raw(), 0).unwrap(), Some(1));
    r.reserve_runtime_before_create(&mut alloc, 4, c("oBrick"))
        .unwrap();
    assert_eq!(
        (0..4)
            .map(|i| r.family_slot(c("oSolid").raw(), i).unwrap().unwrap())
            .collect::<Vec<_>>(),
        vec![1, 2, 3, 4]
    );
    assert_eq!(
        r.cleanup_active_with(|_| Ok::<_, ()>(())).unwrap(),
        Vec::<u64>::new()
    );
    assert_eq!(r.family_slot(c("oSolid").raw(), 0).unwrap(), Some(1));
    r.mark_after_destroy_callback(2).unwrap();
    let mut freed = vec![];
    r.cleanup_active_with(|key| {
        freed.push(key);
        Ok::<_, ()>(())
    })
    .unwrap();
    assert_eq!(freed, vec![2]);
    assert_eq!(
        (0..3)
            .map(|i| r.family_slot(c("oSolid").raw(), i).unwrap().unwrap())
            .collect::<Vec<_>>(),
        vec![3, 1, 4]
    );
}
#[test]
fn pending_partition_reactivations_append_first_marked_retained_and_empty_family_flushes() {
    let mut alloc = NativeAllocator::new(CounterInput::ExactReached {
        last_native_id: 300000,
    });
    let mut r = room(1);
    let mut i = a(3, 13, "oBrick");
    i.membership = Membership::Inactive;
    i.pending_inactive109 = true;
    i.marked108 = true;
    r.reserve_authored_all(&mut alloc, &[a(1, 11, "oBrick"), a(2, 12, "oBrick"), i])
        .unwrap();
    r.set_pending109(1, true).unwrap();
    r.set_pending109(3, false).unwrap();
    assert!(r.request_family_inactive(0, true).unwrap()); // hole matches0, still flush pending
    assert_eq!(
        (0..2)
            .map(|i| r.active_slot(i).unwrap().unwrap())
            .collect::<Vec<_>>(),
        vec![2, 3]
    );
    assert_eq!(r.inactive_keys().unwrap(), &[1]);
    assert!(r.record(3).unwrap().marked108);
    assert_eq!(r.family_count(c("oSolid").raw()).unwrap(), 2);
    r.publish_draw_active_permutation(vec![3, 2]).unwrap();
    assert!(!r.partition_pending_membership().unwrap());
    assert_eq!(r.family_slot(c("oSolid").raw(), 0).unwrap(), Some(2));
}
#[test]
fn carry_journal_after_callbacks_active_only_neutral_oldmark_preserves_pending() {
    let mut alloc = NativeAllocator::new(CounterInput::ExactReached {
        last_native_id: 400000,
    });
    let mut r = room(1);
    let mut inactive = a(2, 12, "oScreen");
    inactive.membership = Membership::Inactive;
    inactive.pending_inactive109 = true;
    r.reserve_authored_all(
        &mut alloc,
        &[a(1, 11, "oScreen"), inactive, a(3, 13, "oBrick")],
    )
    .unwrap();
    r.start_entry_callbacks().unwrap(); // RoomEnd later creates actualpersistent Music
    r.reserve_runtime_before_create(&mut alloc, 4, c("oMusic"))
        .unwrap();
    r.set_pending109(1, true).unwrap();
    let before = alloc.last_native_id();
    let journal = r.snapshot_active_carry_after_room_end().unwrap();
    assert_eq!(
        journal
            .iter()
            .map(CarryItem::source_key)
            .collect::<Vec<_>>(),
        vec![1, 4]
    );
    assert!(journal[0].copied().pending_inactive109);
    assert!(!journal[0].copied().marked108);
    assert_eq!(alloc.last_native_id(), before);
    assert!(r.record(1).unwrap().marked108);
    assert!(!r.record(2).unwrap().marked108);
    assert_eq!(r.inactive_keys().unwrap(), &[2]);
}
#[test]
fn carry_overlay_marks_all_same_id_burns_one_restores_id_and_excludes_create_suffix() {
    let mut alloc = NativeAllocator::new(CounterInput::ExactReached {
        last_native_id: 500000,
    });
    let mut old = room(1);
    old.reserve_authored_all(
        &mut alloc,
        &[a(1, 700001, "oScreen"), a(2, 700001, "oScreen")],
    )
    .unwrap();
    let journal = old.snapshot_active_carry_after_room_end().unwrap();
    let mut dest = room(2);
    let mut inactive = a(9, 700001, "oScreen");
    inactive.membership = Membership::Inactive;
    inactive.pending_inactive109 = true;
    dest.reserve_authored_all(
        &mut alloc,
        &[a(7, 700001, "oScreen"), a(8, 700001, "oScreen"), inactive],
    )
    .unwrap();
    let first = dest.attach_one_carry(&mut alloc, 10, &journal[0]).unwrap();
    assert_eq!(first.temporary_burned_id, NativeId(500001));
    assert_eq!(first.restored_id, NativeId(700001));
    let second = dest.attach_one_carry(&mut alloc, 11, &journal[1]).unwrap();
    assert_eq!(second.temporary_burned_id, NativeId(500002));
    assert!(
        dest.record(7).unwrap().marked108
            && dest.record(8).unwrap().marked108
            && dest.record(10).unwrap().marked108
    );
    assert!(!dest.record(11).unwrap().marked108 && !dest.record(9).unwrap().marked108);
    assert_eq!(dest.family_count(c("oScreen").raw()).unwrap(), 4);
    let mut cursor = dest.start_entry_callbacks().unwrap().unwrap();
    assert_eq!(cursor.bound(), 2);
    assert_eq!(
        cursor.next(&dest).unwrap(),
        Some(CurrentSlot {
            key: 7,
            marked108: true
        })
    );
    assert_eq!(
        cursor.next(&dest).unwrap(),
        Some(CurrentSlot {
            key: 8,
            marked108: true
        })
    );
    assert_eq!(cursor.next(&dest).unwrap(), None);
    assert_eq!(alloc.last_native_id(), NativeId(500002));
}
#[test]
fn cached_destination_does_not_fresh_create_and_context_mark_is_not_missing() {
    let mut alloc = NativeAllocator::new(CounterInput::ExactReached {
        last_native_id: 600000,
    });
    let mut r = room(1);
    r.reserve_authored_all(&mut alloc, &[a(1, 12, "oBrick")])
        .unwrap();
    r.start_entry_callbacks().unwrap();
    let token = r.capture_executing_self(1).unwrap();
    r.mark_after_destroy_callback(1).unwrap();
    assert_eq!(
        r.captured_self_record(token).unwrap().native_id,
        NativeId(12)
    );
    r.select_cached_destination_entry().unwrap();
    assert!(r.start_entry_callbacks().unwrap().is_none());
    r.cleanup_active_with(|_| Ok::<_, ()>(())).unwrap();
    assert_eq!(r.captured_self_record(token), Err(Error::MissingStorageKey));
    assert_eq!(
        room(2).captured_self_record(token),
        Err(Error::WrongRoomContext)
    );
}
#[test]
fn raw_fixed_count_reloads_current_slot_after_sort_and_ignores_append() {
    let mut alloc = NativeAllocator::new(CounterInput::ExactReached {
        last_native_id: 700000,
    });
    let mut r = room(1);
    r.reserve_authored_all(
        &mut alloc,
        &[a(1, 11, "oBrick"), a(2, 12, "oBrick"), a(3, 13, "oBrick")],
    )
    .unwrap();
    let mut cursor = r.fixed_current_active_cursor().unwrap();
    assert_eq!(cursor.next(&r).unwrap().unwrap().key, 1);
    r.publish_draw_active_permutation(vec![3, 2, 1]).unwrap();
    r.reserve_runtime_before_create(&mut alloc, 4, c("oBrick"))
        .unwrap();
    assert_eq!(cursor.next(&r).unwrap().unwrap().key, 2);
    assert_eq!(cursor.next(&r).unwrap().unwrap().key, 1);
    assert!(cursor.next(&r).unwrap().is_none());
}
#[test]
fn cleanup_failure_invalidates_no_fake_rollback_and_inactive_mark_is_not_freed() {
    let mut alloc = NativeAllocator::new(CounterInput::ExactReached {
        last_native_id: 800000,
    });
    let mut r = room(1);
    let mut i = a(3, 13, "oBrick");
    i.membership = Membership::Inactive;
    i.pending_inactive109 = true;
    i.marked108 = true;
    r.reserve_authored_all(&mut alloc, &[a(1, 11, "oBrick"), a(2, 12, "oBrick"), i])
        .unwrap();
    r.mark_without_callback(1).unwrap();
    let freed = r.cleanup_active_with(|_| Ok::<_, ()>(())).unwrap();
    assert_eq!(freed, vec![1]);
    assert!(r.record(3).unwrap().marked108);
    r.mark_without_callback(2).unwrap();
    assert_eq!(
        r.cleanup_active_with(|_| Err::<(), _>("free failed")),
        Err(CleanupError::Backing("free failed"))
    );
    assert_eq!(r.active_count(), Err(Error::UnavailableHistory));
    assert_eq!(alloc.last_native_id(), NativeId(800000));
}
struct Body(BTreeMap<u64, (u64, u64)>);
impl BodyCoordinates<u64> for Body {
    type Error = &'static str;
    fn x_qword(&mut self, k: u64) -> Result<u64, Self::Error> {
        self.0.get(&k).map(|p| p.0).ok_or("missingX")
    }
    fn y_qword(&mut self, k: u64) -> Result<u64, Self::Error> {
        self.0.get(&k).map(|p| p.1).ok_or("missingY")
    }
}
#[test]
fn actual_ledger_numeric_adapter_fractional_replacement_and_mark_pending_rules() {
    let mut alloc = NativeAllocator::new(CounterInput::ExactReached {
        last_native_id: 900000,
    });
    let mut r = room(1);
    let id1 = r
        .reserve_runtime_before_create(&mut alloc, 501, c("oBrick"))
        .unwrap();
    let id2 = r
        .reserve_runtime_before_create(&mut alloc, 19, c("oBrick"))
        .unwrap();
    let mut body = Body(BTreeMap::from([
        (501, (0x3fb999999999999a, 0)),
        (19, (0x3fb999999999999a, 0)),
    ]));
    let cap = admit_control_word(0x1372).unwrap();
    let selector = f64::from(c("oSolid").raw()).to_bits();
    let value = nearest_value(&cap, &r, &mut body, 0, 0, selector).unwrap();
    assert_eq!(value.qword_bits(), id2.qword_bits());
    r.set_pending109(19, true).unwrap();
    assert_eq!(
        nearest_value(&cap, &r, &mut body, 0, 0, selector)
            .unwrap()
            .qword_bits(),
        id2.qword_bits()
    );
    r.mark_without_callback(19).unwrap();
    assert_eq!(r.family_count(c("oSolid").raw()).unwrap(), 2);
    assert_eq!(
        nearest_value(&cap, &r, &mut body, 0, 0, selector)
            .unwrap()
            .qword_bits(),
        id1.qword_bits()
    );
}
#[test]
fn holes_and_history_sort_validation_are_typed() {
    assert_eq!(class_by_id(0), Err(Error::InvalidClass(0)));
    assert_eq!(class_by_id(-1), Err(Error::InvalidClass(-1)));
    let mut alloc = NativeAllocator::new(CounterInput::ExactReached {
        last_native_id: 100000,
    });
    let mut r = room(1);
    r.reserve_authored_all(&mut alloc, &[a(1, 11, "oBrick"), a(2, 11, "oBrick")])
        .unwrap();
    for id in [-3, -1, 0, 944] {
        assert_eq!(r.family_count(id).unwrap(), 0)
    }
    assert_eq!(
        r.publish_draw_active_permutation(vec![1, 1]),
        Err(Error::InvalidSortPermutation)
    );
    r.invalidate_unjoined_history();
    assert_eq!(r.family_count(126), Err(Error::UnavailableHistory));
}
