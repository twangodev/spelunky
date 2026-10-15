//! Concrete raw ledger -> frozen numeric nearest. World owns live X/Y fields;
//! lookup is allocation-keyed and must not dispatch Source code or mutate ledger.
use crate::native_ledger::{Error as LedgerError, RoomLedger};
use crate::nearest_numeric::{self as numeric, Admission, RawCandidates, Route};
pub trait BodyCoordinates<K> {
    type Error;
    fn x_qword(&mut self, key: K) -> Result<u64, Self::Error>;
    fn y_qword(&mut self, key: K) -> Result<u64, Self::Error>;
}
#[derive(Debug, PartialEq, Eq)]
pub enum BackingError<E> {
    Ledger(LedgerError),
    Body(E),
    CountOverflow,
}
struct Candidates<'a, K: Copy + Ord, B> {
    room: &'a RoomLedger<K>,
    body: &'a mut B,
}
impl<K: Copy + Ord, B: BodyCoordinates<K>> RawCandidates for Candidates<'_, K, B> {
    type Key = K;
    type Error = BackingError<B::Error>;
    fn count(&mut self, r: Route) -> Result<i32, Self::Error> {
        let count = match r {
            Route::All => self.room.active_count(),
            Route::Family(id) => self.room.family_count(id),
        }
        .map_err(BackingError::Ledger)?;
        i32::try_from(count).map_err(|_| BackingError::CountOverflow)
    }
    fn current_slot(&mut self, r: Route, index: i32) -> Result<Option<K>, Self::Error> {
        let Ok(i) = usize::try_from(index) else {
            return Ok(None);
        };
        match r {
            Route::All => self.room.active_slot(i),
            Route::Family(id) => self.room.family_slot(id, i),
        }
        .map_err(BackingError::Ledger)
    }
    fn tombstone108(&mut self, key: K) -> Result<bool, Self::Error> {
        Ok(self
            .room
            .record(key)
            .map_err(BackingError::Ledger)?
            .marked108)
    }
    fn x_qword(&mut self, key: K) -> Result<u64, Self::Error> {
        self.room.record(key).map_err(BackingError::Ledger)?;
        self.body.x_qword(key).map_err(BackingError::Body)
    }
    fn y_qword(&mut self, key: K) -> Result<u64, Self::Error> {
        self.room.record(key).map_err(BackingError::Ledger)?;
        self.body.y_qword(key).map_err(BackingError::Body)
    }
    fn visible_id_i32(&mut self, key: K) -> Result<i32, Self::Error> {
        Ok(self
            .room
            .record(key)
            .map_err(BackingError::Ledger)?
            .native_id
            .0)
    }
}
/// Source value only. No winning allocation key can escape as a fake native ID.
/// Each qualified/with consumer resolves this numeric value at its actual use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeNumericValue {
    qword: u64,
}
impl NativeNumericValue {
    pub fn qword_bits(self) -> u64 {
        self.qword
    }
}
pub fn nearest_value<K: Copy + Ord, B: BodyCoordinates<K>>(
    admission: &Admission,
    room: &RoomLedger<K>,
    body: &mut B,
    x: u64,
    y: u64,
    selector: u64,
) -> Result<NativeNumericValue, numeric::Error<BackingError<B::Error>>> {
    let result = numeric::evaluate(admission, x, y, selector, &mut Candidates { room, body })?;
    Ok(NativeNumericValue {
        qword: result.source_result_qword,
    })
}
