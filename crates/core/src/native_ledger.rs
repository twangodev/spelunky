//! Native ID/membership producer, ordinary successful canonical engine flow.
//! Storage keys are supplied independently. No Default/nativeID-from-key cast,
//! no Source callbacks, no RNG and no position/custom-field authority here.
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct NativeId(pub i32);
impl NativeId {
    pub fn qword_bits(self) -> u64 {
        f64::from(self.0).to_bits()
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClassId(i32);
impl ClassId {
    pub fn raw(self) -> i32 {
        self.0
    }
}
#[derive(Clone, Copy, Debug)]
pub struct ClassDef {
    pub name: &'static str,
    pub parent: i32,
    pub persistent: bool,
}
include!("canonical_classes.rs");
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    UnavailableHistory,
    InvalidClass(i32),
    DuplicateStorageKey,
    MissingStorageKey,
    InvalidSortPermutation,
    InvalidFreshCarrySuffix,
    MissingCurrentSlot { index: usize },
    WrongRoomContext,
    InvalidCanonicalGraph,
    CountOverflow,
    WrongEntryPhase,
    UnsupportedMembershipSelector(i32),
    HostRoomLifetimeExhausted,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CounterInput {
    /// Exactly AFTER successful canonical asset loader stores through58F948 to
    ///6C739C. This does not mean arbitrary World::new or reached Game Create.
    CanonicalAssetLoaderCompleted,
    /// Required reached state after all prior source allocations/save restores.
    /// The caller owns provenance; there is deliberately no Unknown/Default.
    ExactReached { last_native_id: i32 },
}
#[derive(Debug)]
pub struct NativeAllocator<K: Copy + Ord> {
    last: i32,
    issued: BTreeSet<K>,
}
impl<K: Copy + Ord> NativeAllocator<K> {
    pub fn new(input: CounterInput) -> Self {
        let last = match input {
            CounterInput::CanonicalAssetLoaderCompleted => 135506,
            CounterInput::ExactReached { last_native_id } => last_native_id,
        };
        Self {
            last,
            issued: BTreeSet::new(),
        }
    }
    pub fn last_native_id(&self) -> NativeId {
        NativeId(self.last)
    }
    fn claim(&mut self, key: K) -> Result<(), Error> {
        if !self.issued.insert(key) {
            return Err(Error::DuplicateStorageKey);
        }
        Ok(())
    }
    fn next_claimed(&mut self) -> NativeId {
        self.last = self.last.wrapping_add(1);
        NativeId(self.last)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Membership {
    Active,
    Inactive,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Birth {
    AuthoredClone,
    Runtime,
    Carry,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    pub native_id: NativeId,
    pub class: ClassId,
    pub marked108: bool,
    pub pending_inactive109: bool,
    pub persistent: bool,
    pub birth: Birth,
}
/// Explicit copied template metadata. Body/property/creation-code copy is the
/// room owner; these fields must come from the same actual source template.
#[derive(Clone, Debug)]
pub struct TemplateActor<K> {
    pub key: K,
    pub native_id: NativeId,
    pub class: ClassId,
    pub membership: Membership,
    pub marked108: bool,
    pub pending_inactive109: bool,
    pub persistent: bool,
}
impl<K> TemplateActor<K> {
    /// Only an unmutated compiled authored-placement constructor profile, NOT an
    /// absent/imported/cached actor default. Native own persistent is not inherited.
    pub fn unmutated_compiled(key: K, native_id: NativeId, class: ClassId) -> Self {
        Self {
            key,
            native_id,
            class,
            membership: Membership::Active,
            marked108: false,
            pending_inactive109: false,
            persistent: CANONICAL_CLASSES[class.0 as usize].unwrap().persistent,
        }
    }
}
#[derive(Clone, Debug)]
pub struct CarryItem<K> {
    source_key: K,
    copied: Record,
}
impl<K: Copy> CarryItem<K> {
    pub fn source_key(&self) -> K {
        self.source_key
    }
    pub fn copied(&self) -> &Record {
        &self.copied
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CarryInstall<K> {
    pub old_key: K,
    pub new_key: K,
    pub temporary_burned_id: NativeId,
    pub restored_id: NativeId,
}
/// This is host storage/context identity, never a native ID/counter draw.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RoomStorageKey(pub u64);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CapturedSelf<K> {
    room: RoomStorageKey,
    lifetime: RoomLifetime,
    key: K,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CurrentSlot<K> {
    pub key: K,
    pub marked108: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct FixedCurrentCursor {
    next: usize,
    bound: usize,
}
impl FixedCurrentCursor {
    pub fn bound(&self) -> usize {
        self.bound
    }
    pub fn next<K: Copy + Ord>(
        &mut self,
        room: &RoomLedger<K>,
    ) -> Result<Option<CurrentSlot<K>>, Error> {
        room.available()?;
        if self.next >= self.bound {
            return Ok(None);
        }
        let index = self.next;
        self.next += 1;
        let key = *room
            .active
            .get(index)
            .ok_or(Error::MissingCurrentSlot { index })?;
        Ok(Some(CurrentSlot {
            key,
            marked108: room.record(key)?.marked108,
        }))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EntryPhase {
    PreparingFresh,
    PreparingCached,
    CallbacksStarted,
}
/// Full944-slot materialized rows (including holes). This stores raw keys,
/// including marked108 records until physical free. No query-time sorting/filter.
#[derive(Debug)]
pub struct RoomLedger<K: Copy + Ord> {
    room_key: RoomStorageKey,
    lifetime: RoomLifetime,
    records: BTreeMap<K, Record>,
    active: Vec<K>,
    inactive: Vec<K>,
    own: Vec<Vec<K>>,
    families: Vec<Vec<K>>,
    available: bool,
    phase: EntryPhase,
    entry_carry_count: usize,
}
/// Host-only nonduplicable room allocation continuity, not a resource/native ID.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RoomLifetime(u64);
static NEXT_ROOM_LIFETIME: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
fn issue_room_lifetime() -> Result<RoomLifetime, Error> {
    use std::sync::atomic::Ordering;
    NEXT_ROOM_LIFETIME
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |n| n.checked_add(1))
        .map(RoomLifetime)
        .map_err(|_| Error::HostRoomLifetimeExhausted)
}
/// Completed callback-free destruction of ONE actual nonpersistent room.
/// Minted only after every active then inactive physical-free hook succeeds.
/// This is metadata lifetime evidence, not a Source value or cached-room permit.
#[derive(Debug)]
pub struct DisposedRoom<K: Copy + Ord> {
    room: RoomStorageKey,
    lifetime: RoomLifetime,
    freed: Vec<K>,
}
impl<K: Copy + Ord> DisposedRoom<K> {
    pub fn room_key(&self) -> RoomStorageKey {
        self.room
    }
    pub fn freed_keys(&self) -> &[K] {
        &self.freed
    }
    pub fn lifetime(&self) -> RoomLifetime {
        self.lifetime
    }
}
impl<K: Copy + Ord> RoomLedger<K> {
    /// An actually empty native destination before its authored clone reserves.
    /// Caller must not use this to erase unknown/imported current state.
    pub fn empty_destination(room_key: RoomStorageKey) -> Result<Self, Error> {
        validate_canonical_graph()?;
        Ok(Self {
            room_key,
            lifetime: issue_room_lifetime()?,
            records: BTreeMap::new(),
            active: vec![],
            inactive: vec![],
            own: vec![vec![]; 944],
            families: vec![vec![]; 944],
            available: true,
            phase: EntryPhase::PreparingFresh,
            entry_carry_count: 0,
        })
    }
    fn available(&self) -> Result<(), Error> {
        if self.available {
            Ok(())
        } else {
            Err(Error::UnavailableHistory)
        }
    }
    pub fn invalidate_unjoined_history(&mut self) {
        self.available = false;
    }
    /// Owner has selected the actual retained cached room, without fresh cloning.
    pub fn select_cached_destination_entry(&mut self) -> Result<(), Error> {
        self.available()?;
        self.phase = EntryPhase::PreparingCached;
        self.entry_carry_count = 0;
        Ok(())
    }
    pub fn record(&self, key: K) -> Result<&Record, Error> {
        self.available()?;
        self.records.get(&key).ok_or(Error::MissingStorageKey)
    }
    pub fn active_count(&self) -> Result<usize, Error> {
        self.available()?;
        Ok(self.active.len())
    }
    pub fn active_slot(&self, index: usize) -> Result<Option<K>, Error> {
        self.available()?;
        Ok(self.active.get(index).copied())
    }
    pub fn inactive_keys(&self) -> Result<&[K], Error> {
        self.available()?;
        Ok(&self.inactive)
    }
    pub fn family_count(&self, id: i32) -> Result<usize, Error> {
        self.available()?;
        Ok(self.row(id, false).len())
    }
    pub fn family_slot(&self, id: i32, index: usize) -> Result<Option<K>, Error> {
        self.available()?;
        Ok(self.row(id, false).get(index).copied())
    }
    pub fn own_count(&self, id: i32) -> Result<usize, Error> {
        self.available()?;
        Ok(self.row(id, true).len())
    }
    pub fn own_slot(&self, id: i32, index: usize) -> Result<Option<K>, Error> {
        self.available()?;
        Ok(self.row(id, true).get(index).copied())
    }
    fn row(&self, id: i32, own: bool) -> &[K] {
        if !(0..944).contains(&id) {
            return &[];
        }
        if own {
            &self.own[id as usize]
        } else {
            &self.families[id as usize]
        }
    }
    fn append_rows(&mut self, key: K, class: ClassId) {
        self.own[class.0 as usize].push(key);
        for id in ancestors(class) {
            self.families[id as usize].push(key)
        }
    }
    fn rebuild(&mut self) {
        self.own.iter_mut().for_each(Vec::clear);
        self.families.iter_mut().for_each(Vec::clear);
        // Source rebuild order is CURRENT global active, not birth or visible-ID.
        for i in 0..self.active.len() {
            let key = self.active[i];
            let class = self.records[&key].class;
            self.append_rows(key, class)
        }
    }
    /// Clone ALL destination authored active+inactive storage before ANY reached
    /// placement code/Create. Direct ctor/copy consumes ZERO native counter draws.
    /// Two-pass shape validation avoids partial mutation for unsupported inputs;
    /// successful native heap allocation/copy is the admitted domain.
    pub fn reserve_authored_all(
        &mut self,
        alloc: &mut NativeAllocator<K>,
        actors: &[TemplateActor<K>],
    ) -> Result<(), Error> {
        self.available()?;
        if !self.records.is_empty() || self.phase != EntryPhase::PreparingFresh {
            return Err(Error::WrongEntryPhase);
        }
        if actors.len() > i32::MAX as usize {
            return Err(Error::CountOverflow);
        }
        let mut keys = BTreeSet::new();
        for a in actors {
            class_by_id(a.class.0)?;
            if !keys.insert(a.key) || alloc.issued.contains(&a.key) {
                return Err(Error::DuplicateStorageKey);
            }
        }
        // Caller order is exact template array order. Active/inactive each preserve it.
        for a in actors {
            alloc.claim(a.key)?;
            self.records.insert(
                a.key,
                Record {
                    native_id: a.native_id,
                    class: a.class,
                    marked108: a.marked108,
                    pending_inactive109: a.pending_inactive109,
                    persistent: a.persistent,
                    birth: Birth::AuthoredClone,
                },
            );
            match a.membership {
                Membership::Active => self.active.push(a.key),
                Membership::Inactive => self.inactive.push(a.key),
            }
        }
        // 4ADC48 final4ADF32/39 rebuild AFTER both array copy loops.
        self.rebuild();
        Ok(())
    }
    /// Commit constructor/global+own+ancestor registration BEFORE source Create.
    /// Parent must retain this birth even if subsequent Source Create returns error.
    pub fn reserve_runtime_before_create(
        &mut self,
        alloc: &mut NativeAllocator<K>,
        key: K,
        class: ClassId,
    ) -> Result<NativeId, Error> {
        self.available()?;
        class_by_id(class.0)?;
        if self.records.contains_key(&key) {
            return Err(Error::DuplicateStorageKey);
        }
        if self.active.len() >= i32::MAX as usize {
            return Err(Error::CountOverflow);
        }
        alloc.claim(key)?;
        let native_id = alloc.next_claimed();
        self.records.insert(
            key,
            Record {
                native_id,
                class,
                marked108: false,
                pending_inactive109: false,
                persistent: CANONICAL_CLASSES[class.0 as usize].unwrap().persistent,
                birth: Birth::Runtime,
            },
        );
        self.active.push(key);
        self.append_rows(key, class);
        Ok(native_id)
    }
    /// Callback-FREE actual runtime constructor boundary. Normal native4AED84
    /// burns the visible ID BEFORE 4AB45C, then installs global/own/family rows.
    /// The closure is infallible: it must be the admitted resource constructor,
    /// never Source code, callbacks, arbitrary services or a fallible fake ctor.
    /// Preflight errors change no native counter/rows. Allocation/RTL exceptions
    /// are outside this successful-constructor contract. Native's transient raw
    /// active-count increment before ctor has no Source observer in this domain.
    pub fn reserve_runtime_constructed<T>(
        &mut self,
        alloc: &mut NativeAllocator<K>,
        key: K,
        class: ClassId,
        constructor: impl FnOnce(NativeId, &NativeAllocator<K>) -> T,
    ) -> Result<(NativeId, T), Error> {
        self.available()?;
        class_by_id(class.0)?;
        if self.records.contains_key(&key) {
            return Err(Error::DuplicateStorageKey);
        }
        if self.active.len() >= i32::MAX as usize {
            return Err(Error::CountOverflow);
        }
        alloc.claim(key)?;
        let native_id = alloc.next_claimed();
        let body = constructor(native_id, &*alloc);
        self.records.insert(
            key,
            Record {
                native_id,
                class,
                marked108: false,
                pending_inactive109: false,
                persistent: CANONICAL_CLASSES[class.0 as usize].unwrap().persistent,
                birth: Birth::Runtime,
            },
        );
        self.active.push(key);
        self.append_rows(key, class);
        Ok((native_id, body))
    }
    /// Call after each actual Destroy callback returns. Already marked does NOT
    /// authorize skipping a repeated explicit self Destroy callback at the owner.
    pub fn mark_after_destroy_callback(&mut self, key: K) -> Result<(), Error> {
        self.mark_without_callback(key)
    }
    pub fn mark_without_callback(&mut self, key: K) -> Result<(), Error> {
        self.available()?;
        self.records
            .get_mut(&key)
            .ok_or(Error::MissingStorageKey)?
            .marked108 = true;
        Ok(())
    }
    pub fn set_persistent_backing(&mut self, key: K, value: bool) -> Result<(), Error> {
        self.available()?;
        self.records
            .get_mut(&key)
            .ok_or(Error::MissingStorageKey)?
            .persistent = value;
        Ok(())
    }
    pub fn set_pending109(&mut self, key: K, inactive: bool) -> Result<(), Error> {
        self.available()?;
        self.records
            .get_mut(&key)
            .ok_or(Error::MissingStorageKey)?
            .pending_inactive109 = inactive;
        Ok(())
    }
    /// 4AF00C transfers109 flags; it never physically frees108. A no-transfer call
    /// leaves materialized row order unchanged. Reactivations append FIRST.
    pub fn partition_pending_membership(&mut self) -> Result<bool, Error> {
        self.available()?;
        let moving = self
            .active
            .iter()
            .any(|k| self.records[k].pending_inactive109)
            || self
                .inactive
                .iter()
                .any(|k| !self.records[k].pending_inactive109);
        if !moving {
            return Ok(false);
        }
        let mut remaining = Vec::new();
        for key in std::mem::take(&mut self.inactive) {
            if !self.records[&key].pending_inactive109 {
                self.active.push(key)
            } else {
                remaining.push(key)
            }
        }
        self.inactive = remaining;
        let mut active = Vec::new();
        for key in std::mem::take(&mut self.active) {
            if self.records[&key].pending_inactive109 {
                self.inactive.push(key)
            } else {
                active.push(key)
            }
        }
        self.active = active;
        self.rebuild();
        Ok(true)
    }
    /// Selector dispatch/coercion stays separate. Known family flags ALL matching
    /// raw members (including marked), then partition even when family matched0.
    pub fn request_family_inactive(&mut self, id: i32, inactive: bool) -> Result<bool, Error> {
        self.available()?;
        // This is ONLY the nonnegative resource-family arm, not the named builtin.
        if !(0..100000).contains(&id) {
            return Err(Error::UnsupportedMembershipSelector(id));
        }
        let keys = if inactive {
            self.active.clone()
        } else {
            self.inactive.clone()
        };
        for key in keys {
            if ancestors(self.records[&key].class).contains(&id) {
                self.set_pending109(key, inactive)?
            }
        }
        self.partition_pending_membership()
    }
    /// Native Draw stable sort owner supplies its exact resulting permutation.
    /// This commits GLOBAL mutation only. Family rows deliberately remain old.
    pub fn publish_draw_active_permutation(&mut self, new_order: Vec<K>) -> Result<(), Error> {
        self.available()?;
        if new_order.len() != self.active.len()
            || new_order.iter().copied().collect::<BTreeSet<_>>()
                != self.active.iter().copied().collect()
        {
            return Err(Error::InvalidSortPermutation);
        }
        self.active = new_order;
        Ok(())
    }
    /// Concrete ordinary native backing-free order. Closure must NOT dispatch GML
    /// Destroy or arbitrary memory-manager callbacks. Failure invalidates backing;
    /// no fictitious rollback. Inactive marked records are untouched.
    pub fn cleanup_active_with<E>(
        &mut self,
        mut free: impl FnMut(K) -> Result<(), E>,
    ) -> Result<Vec<K>, CleanupError<E>> {
        self.available().map_err(CleanupError::Ledger)?;
        let mut write = 0;
        let mut removed = vec![];
        for index in 0..self.active.len() {
            let key = self.active[index];
            if self.records[&key].marked108 {
                if let Err(error) = free(key) {
                    self.available = false;
                    return Err(CleanupError::Backing(error));
                }
                self.records.remove(&key);
                removed.push(key);
            } else {
                self.active[write] = key;
                write += 1
            }
        }
        self.active.truncate(write);
        if !removed.is_empty() {
            self.rebuild()
        }
        Ok(removed)
    }
    /// Requires all authored reservation and entry-callback cursor admission.
    pub fn require_entry_callbacks_started(&self) -> Result<(), Error> {
        self.available()?;
        if self.phase != EntryPhase::CallbacksStarted {
            return Err(Error::WrongEntryPhase);
        }
        Ok(())
    }
    pub fn lifetime(&self) -> Result<RoomLifetime, Error> {
        self.available()?;
        Ok(self.lifetime)
    }
    /// Actual nonpersistent-room destruction AFTER RoomEnd and carry snapshot.
    /// The owner MUST prove the room resource persistent flag is false and that
    /// all executing/captured scope loans are idle. This does NOT select room exit
    /// or run Destroy/Create. The free callback performs actual body retirement,
    /// never Source callbacks. Canonical Room_Class destructor resets/free active
    /// slots ascending, then inactive slots ascending, irrespective108/109.
    /// Failed physical free invalidates this ledger, retains completed prefix,
    /// produces NO disposal proof and cannot be retried as successful destruction.
    pub fn dispose_nonpersistent_after_room_end_with<E>(
        &mut self,
        mut free: impl FnMut(K) -> Result<(), E>,
    ) -> Result<DisposedRoom<K>, CleanupError<E>> {
        self.available().map_err(CleanupError::Ledger)?;
        let mut freed = vec![];
        for index in 0..self.active.len() {
            let key = self.active[index];
            if let Err(error) = free(key) {
                self.available = false;
                return Err(CleanupError::Backing(error));
            }
            self.records.remove(&key);
            freed.push(key);
        }
        self.active.clear();
        for index in 0..self.inactive.len() {
            let key = self.inactive[index];
            if let Err(error) = free(key) {
                self.available = false;
                return Err(CleanupError::Backing(error));
            }
            self.records.remove(&key);
            freed.push(key);
        }
        self.inactive.clear();
        self.records.clear();
        for row in &mut self.own {
            row.clear()
        }
        for row in &mut self.families {
            row.clear()
        }
        self.available = false;
        Ok(DisposedRoom {
            room: self.room_key,
            lifetime: self.lifetime,
            freed,
        })
    }
    /// Read-only ordinary carry metadata capture. The Scene owner must copy the
    /// complete actual body and publish its journal entry BEFORE marking old108.
    /// Unlike the retained bulk compatibility method below, this does NOT mark.
    pub fn copy_active_carry_metadata(&self, key: K) -> Result<Option<CarryItem<K>>, Error> {
        self.available()?;
        let record = self.records.get(&key).ok_or(Error::MissingStorageKey)?;
        if !self.active.contains(&key) || record.marked108 || !record.persistent {
            return Ok(None);
        }
        Ok(Some(CarryItem {
            source_key: key,
            copied: record.clone(),
        }))
    }
    /// Host room provenance only; not a native room-resource or numeric actor ID.
    pub fn storage_room_key(&self) -> Result<RoomStorageKey, Error> {
        self.available()?;
        Ok(self.room_key)
    }
    /// Admit carry entry before touching any destination body or allocator. A
    /// selected cached room first calls select_cached_destination_entry().
    pub fn require_carry_entry(&self) -> Result<(), Error> {
        self.available()?;
        if self.phase == EntryPhase::CallbacksStarted {
            return Err(Error::WrongEntryPhase);
        }
        Ok(())
    }
    /// AFTER RoomEnd and its births/marks: journal only current active108==0,
    /// persistent. Value snapshot/old mark consumes no counter, no source callbacks.
    /// Inactive old-room records stay in this room; cache/fate is not invented.
    pub fn snapshot_active_carry_after_room_end(&mut self) -> Result<Vec<CarryItem<K>>, Error> {
        self.available()?;
        let mut journal = vec![];
        for key in self.active.clone() {
            let record = self.records[&key].clone();
            if !record.marked108 && record.persistent {
                journal.push(CarryItem {
                    source_key: key,
                    copied: record,
                });
                self.mark_without_callback(key)?;
            }
        }
        Ok(journal)
    }
    /// Native53DE13 marks EVERY matching current activeID, including a previously
    /// attached carry.4AED84 burns1 counter/append, then4AB670 restores copiedID.
    /// Owner must copy remaining actual body/property data BEFORE next carry entry;
    /// no Source Create, Destroy or callback is introduced by this operation.
    pub fn attach_one_carry(
        &mut self,
        alloc: &mut NativeAllocator<K>,
        new_key: K,
        item: &CarryItem<K>,
    ) -> Result<CarryInstall<K>, Error> {
        self.available()?;
        class_by_id(item.copied.class.0)?;
        if self.phase == EntryPhase::CallbacksStarted {
            return Err(Error::WrongEntryPhase);
        }
        if alloc.issued.contains(&new_key) || self.records.contains_key(&new_key) {
            return Err(Error::DuplicateStorageKey);
        }
        for key in self.active.clone() {
            if self.records[&key].native_id == item.copied.native_id {
                self.mark_without_callback(key)?
            }
        }
        let temporary = self.reserve_runtime_before_create(alloc, new_key, item.copied.class)?;
        let record = self.records.get_mut(&new_key).unwrap();
        *record = item.copied.clone();
        record.birth = Birth::Carry;
        self.entry_carry_count += 1;
        Ok(CarryInstall {
            old_key: item.source_key,
            new_key,
            temporary_burned_id: temporary,
            restored_id: record.native_id,
        })
    }
    /// Actual carry constructor boundary: mark matching visible-ID aliases,
    /// then native INC -> infallible raw constructor -> row append -> copied ID.
    /// Body/reference copy must finish before the owner advances the journal.
    /// The closure has the same callback-free successful-constructor domain as
    /// reserve_runtime_constructed; it receives no mutable ledger or services.
    /// Admission errors before constructor invocation retain no new raw body.
    pub fn attach_one_carry_constructed<T>(
        &mut self,
        alloc: &mut NativeAllocator<K>,
        new_key: K,
        item: &CarryItem<K>,
        constructor: impl FnOnce(NativeId, &NativeAllocator<K>) -> T,
    ) -> Result<(CarryInstall<K>, T), Error> {
        self.available()?;
        class_by_id(item.copied.class.0)?;
        if self.phase == EntryPhase::CallbacksStarted {
            return Err(Error::WrongEntryPhase);
        }
        if alloc.issued.contains(&new_key) || self.records.contains_key(&new_key) {
            return Err(Error::DuplicateStorageKey);
        }
        for key in self.active.clone() {
            if self.records[&key].native_id == item.copied.native_id {
                self.mark_without_callback(key)?
            }
        }
        let (temporary, body) =
            self.reserve_runtime_constructed(alloc, new_key, item.copied.class, constructor)?;
        // No callback can remove/change this freshly appended record. Replacing it
        // is infallible, so no post-constructor error can hide an attempted key.
        let mut copied = item.copied.clone();
        copied.birth = Birth::Carry;
        let restored = copied.native_id;
        self.records.insert(new_key, copied);
        self.entry_carry_count += 1;
        Ok((
            CarryInstall {
                old_key: item.source_key,
                new_key,
                temporary_burned_id: temporary,
                restored_id: restored,
            },
            body,
        ))
    }
    pub fn fixed_current_active_cursor(&self) -> Result<FixedCurrentCursor, Error> {
        self.available()?;
        Ok(FixedCurrentCursor {
            next: 0,
            bound: self.active.len(),
        })
    }
    /// Fresh entry bound activecount - journalcount. The appended carry suffix
    /// cannot receive Create again. Cached destination owner skips entire loop.
    pub fn start_entry_callbacks(&mut self) -> Result<Option<FixedCurrentCursor>, Error> {
        self.available()?;
        if self.phase == EntryPhase::CallbacksStarted {
            return Err(Error::WrongEntryPhase);
        }
        if self.phase == EntryPhase::PreparingCached {
            self.phase = EntryPhase::CallbacksStarted;
            return Ok(None);
        }
        let bound = self
            .active
            .len()
            .checked_sub(self.entry_carry_count)
            .ok_or(Error::InvalidFreshCarrySuffix)?;
        if self.active[bound..]
            .iter()
            .any(|key| self.records[key].birth != Birth::Carry)
        {
            return Err(Error::InvalidFreshCarrySuffix);
        }
        self.phase = EntryPhase::CallbacksStarted;
        Ok(Some(FixedCurrentCursor { next: 0, bound }))
    }
    /// Captured executing self has allocation identity, NOT deferred numeric lookup.
    /// No108/109 test here: operation-specific entry resolution is the owner.
    pub fn capture_executing_self(&self, key: K) -> Result<CapturedSelf<K>, Error> {
        self.record(key)?;
        Ok(CapturedSelf {
            room: self.room_key,
            lifetime: self.lifetime,
            key,
        })
    }
    pub fn captured_self_record(&self, token: CapturedSelf<K>) -> Result<&Record, Error> {
        if token.room != self.room_key || token.lifetime != self.lifetime {
            return Err(Error::WrongRoomContext);
        }
        self.record(token.key)
    }
}
#[derive(Debug, PartialEq, Eq)]
pub enum CleanupError<E> {
    Ledger(Error),
    Backing(E),
}
pub fn class_by_id(id: i32) -> Result<ClassId, Error> {
    if (0..944).contains(&id) && CANONICAL_CLASSES[id as usize].is_some() {
        Ok(ClassId(id))
    } else {
        Err(Error::InvalidClass(id))
    }
}
pub fn class_by_name(name: &str) -> Option<ClassId> {
    CANONICAL_CLASSES.iter().enumerate().find_map(|(i, c)| {
        c.as_ref()
            .filter(|c| c.name == name)
            .map(|_| ClassId(i as i32))
    })
}
fn ancestors(class: ClassId) -> Vec<i32> {
    let mut result = vec![];
    let mut id = class.0;
    while id >= 0 {
        let Some(c) = CANONICAL_CLASSES.get(id as usize).and_then(Option::as_ref) else {
            break;
        };
        result.push(id);
        id = c.parent;
    }
    result
}
pub fn validate_canonical_graph() -> Result<(), Error> {
    for (i, c) in CANONICAL_CLASSES.iter().enumerate() {
        if c.is_some() {
            let mut id = i as i32;
            let mut seen = BTreeSet::new();
            while id >= 0 {
                if !seen.insert(id) {
                    return Err(Error::InvalidCanonicalGraph);
                }
                let c = CANONICAL_CLASSES
                    .get(id as usize)
                    .and_then(Option::as_ref)
                    .ok_or(Error::InvalidCanonicalGraph)?;
                id = c.parent;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "native_ledger_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "native_ledger_capture_lifetime_tests.rs"]
mod capture_lifetime_tests;
