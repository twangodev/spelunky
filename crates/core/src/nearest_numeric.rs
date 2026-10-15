//! Qualified finite-value proposal for original instance_nearest.
//! Separate explicit PC64/RN and PC53/RN entry admissions; no host floating arithmetic, Euclidean
//! substitute, generic GML comparisons, namespace defaults or global FPU writes.
use crate::finite_pc64 as f;
pub use crate::finite_pc64::ArithmeticError as NumericError;
use core::cmp::Ordering;

pub const PROFILE_ID: &str = "OriginalNearest_PC64_RN_FiniteQword_v1";
pub const PC53_PROFILE_ID: &str = "OriginalNearest_PC53_RN_FiniteQword_v1";
pub const NOONE_QWORD: u64 = 0xc010_0000_0000_0000;
pub const INITIAL_BEST_QWORD: u64 = 0x4202_a05f_2000_0000;

/// An explicit caller-admitted software profile word, not a hardware read or
/// proof of cold original-engine startup. Imported unknown numeric state must
/// stay unadmitted until its owning provider supplies this capability.
/// No Default: the host must supply its separately qualified entry capability.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Admission {
    profile_control_word: u16,
    precision: Precision,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Precision {
    Pc64,
    Pc53,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnsupportedControlWord(pub u16);
pub fn admit_control_word(profile_word: u16) -> Result<Admission, UnsupportedControlWord> {
    if profile_word & !0x0040 != 0x1332 {
        return Err(UnsupportedControlWord(profile_word));
    }
    Ok(Admission {
        profile_control_word: profile_word,
        precision: Precision::Pc64,
    })
}
/// Explicit PC53/RN counterpart. This is a caller-supplied reached-profile
/// capability, not evidence that an arbitrary original entry has this word.
/// Selectors use the same RC-only FISTP operation; nearest arithmetic dispatches
/// to direct PC53 operations with the extended exponent retained.
pub fn admit_pc53_control_word(profile_word: u16) -> Result<Admission, UnsupportedControlWord> {
    if profile_word & !0x0040 != 0x1232 {
        return Err(UnsupportedControlWord(profile_word));
    }
    Ok(Admission {
        profile_control_word: profile_word,
        precision: Precision::Pc53,
    })
}
impl Admission {
    pub fn profile_control_word(self) -> u16 {
        self.profile_control_word
    }
    fn sub(self, a: f::F128, b: f::F128) -> Result<f::F128, NumericError> {
        match self.precision {
            Precision::Pc64 => f::nearest_sub_pc64_rn(a, b),
            Precision::Pc53 => f::nearest_sub_pc53_rn(a, b),
        }
    }
    fn mul(self, a: f::F128, b: f::F128) -> Result<f::F128, NumericError> {
        match self.precision {
            Precision::Pc64 => f::nearest_mul_pc64_rn(a, b),
            Precision::Pc53 => f::nearest_mul_pc53_rn(a, b),
        }
    }
    fn add(self, a: f::F128, b: f::F128) -> Result<f::F128, NumericError> {
        match self.precision {
            Precision::Pc64 => f::nearest_add_pc64_rn(a, b),
            Precision::Pc53 => f::nearest_add_pc53_rn(a, b),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Selector {
    pub rounded_i64: i64,
    pub low_i32: i32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectorError {
    NonFinite,
    OutsideSuccessfulI64Conversion,
}
/// 403498: current-RC FISTP signed64 then low DWORD. This admitted profile fixes
/// RC=nearest-even. Masked-invalid recovery/other RCs/coercion remain unsupported.
pub fn selector_rte_i64_low_i32(_: &Admission, bits: u64) -> Result<Selector, SelectorError> {
    let field = ((bits >> 52) & 0x7ff) as i32;
    if field == 0x7ff {
        return Err(SelectorError::NonFinite);
    }
    let negative = bits >> 63 != 0;
    let sig = u128::from((bits & 0x000f_ffff_ffff_ffff) | if field != 0 { 1u64 << 52 } else { 0 });
    let exponent = field.max(1) - 1023 - 52;
    let magnitude = if sig == 0 {
        0
    } else if exponent >= 0 {
        if exponent > 63 {
            return Err(SelectorError::OutsideSuccessfulI64Conversion);
        }
        sig << exponent as u32
    } else {
        let shift = (-exponent) as u32;
        if shift > 64 {
            0
        } else {
            let integer = sig >> shift;
            let remainder = sig & ((1u128 << shift) - 1);
            let half = 1u128 << (shift - 1);
            integer + u128::from(remainder > half || remainder == half && integer & 1 != 0)
        }
    };
    let rounded = if negative {
        if magnitude > 1u128 << 63 {
            return Err(SelectorError::OutsideSuccessfulI64Conversion);
        }
        if magnitude == 1u128 << 63 {
            i64::MIN
        } else {
            -(magnitude as i64)
        }
    } else {
        if magnitude > i64::MAX as u128 {
            return Err(SelectorError::OutsideSuccessfulI64Conversion);
        }
        magnitude as i64
    };
    Ok(Selector {
        rounded_i64: rounded,
        low_i32: rounded as i32,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route {
    All,
    Family(i32),
}
/// Required raw backing. Count includes retained marked slots; family rows are
/// the actual current materialized descendant arrays, not birth/ID-sorted lists.
/// Negative/out-of-table family count returns zero by the sealed bounds helper.
/// No trait method may invent source callbacks or use a prefiltered snapshot.
pub trait RawCandidates {
    type Key: Copy + Eq;
    type Error;
    fn count(&mut self, route: Route) -> Result<i32, Self::Error>;
    fn current_slot(&mut self, route: Route, index: i32) -> Result<Option<Self::Key>, Self::Error>;
    fn tombstone108(&mut self, key: Self::Key) -> Result<bool, Self::Error>;
    fn x_qword(&mut self, key: Self::Key) -> Result<u64, Self::Error>;
    fn y_qword(&mut self, key: Self::Key) -> Result<u64, Self::Error>;
    fn visible_id_i32(&mut self, key: Self::Key) -> Result<i32, Self::Error>;
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error<E> {
    Selector(SelectorError),
    Arithmetic(NumericError),
    Provider(E),
    UnsupportedNegativeCount(i32),
    UnsupportedMissingSlot { route: Route, index: i32 },
}
/// Only source_result_qword is the exposed GML result. Keys/best are diagnostics:
/// later Source `with`/qualified access must resolve visible IDs again through
/// its actual alias/serial/namespace provider, never retain a hidden pointer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Outcome<K> {
    pub selector: Selector,
    pub selected_key: Option<K>,
    pub selected_native_id: Option<i32>,
    pub source_result_qword: u64,
    pub final_best_qword: u64,
}
fn number<E>(bits: u64) -> Result<f::F128, Error<E>> {
    f::from_f64(f64::from_bits(bits)).map_err(Error::Arithmetic)
}
fn slot<P: RawCandidates>(p: &mut P, route: Route, index: i32) -> Result<P::Key, Error<P::Error>> {
    p.current_slot(route, index)
        .map_err(Error::Provider)?
        .ok_or(Error::UnsupportedMissingSlot { route, index })
}
fn squared_distance<P: RawCandidates>(
    admission: &Admission,
    p: &mut P,
    route: Route,
    index: i32,
    query_x: u64,
    query_y: u64,
) -> Result<f::F128, Error<P::Error>> {
    // Native reloads the indexed slot separately for each raw field read.
    let x_key = slot(p, route, index)?;
    let x = number(p.x_qword(x_key).map_err(Error::Provider)?)?;
    let dx = admission
        .sub(x, number(query_x)?)
        .map_err(Error::Arithmetic)?;
    let xx = admission.mul(dx, dx).map_err(Error::Arithmetic)?;
    // xx has <=64 significant bits and normal Ext80 exponent (or zero), so the
    // original TBYTE spill/reload is exact and does not narrow to QWORD here.
    let y_key = slot(p, route, index)?;
    let y = number(p.y_qword(y_key).map_err(Error::Provider)?)?;
    let dy = admission
        .sub(y, number(query_y)?)
        .map_err(Error::Arithmetic)?;
    let yy = admission.mul(dy, dy).map_err(Error::Arithmetic)?;
    admission.add(yy, xx).map_err(Error::Arithmetic)
}

/// Finite values only when arithmetic is reached. Invalid/unavailable backing
/// fails typed; no fake empty family, noone or coordinates are substituted.
/// Native exact-ID branch and empty/all-marked arrays do not inspect X/Y.
pub fn evaluate<P: RawCandidates>(
    admission: &Admission,
    query_x: u64,
    query_y: u64,
    selector_bits: u64,
    p: &mut P,
) -> Result<Outcome<P::Key>, Error<P::Error>> {
    let selector = selector_rte_i64_low_i32(admission, selector_bits).map_err(Error::Selector)?;
    let mut result = Outcome {
        selector,
        selected_key: None,
        selected_native_id: None,
        source_result_qword: NOONE_QWORD,
        final_best_qword: INITIAL_BEST_QWORD,
    };
    if selector.low_i32 >= 100_000 {
        return Ok(result);
    }
    let route = if selector.low_i32 == -3 {
        Route::All
    } else {
        Route::Family(selector.low_i32)
    };
    let count = p.count(route).map_err(Error::Provider)?;
    if count < 0 {
        return Err(Error::UnsupportedNegativeCount(count));
    }
    for index in 0..count {
        let key = slot(p, route, index)?;
        if p.tombstone108(key).map_err(Error::Provider)? {
            continue;
        }
        let distance = squared_distance(admission, p, route, index, query_x, query_y)?;
        let best = number(result.final_best_qword)?;
        if f::nearest_compare_finite(distance, best).map_err(Error::Arithmetic)? == Ordering::Less {
            // Native recomputes, QWORD-stores, then reloads the selected pointer.
            let recomputed = squared_distance(admission, p, route, index, query_x, query_y)?;
            result.final_best_qword = f::to_f64_nearest(recomputed)
                .map_err(Error::Arithmetic)?
                .to_bits();
            result.selected_key = Some(slot(p, route, index)?);
        }
    }
    if let Some(key) = result.selected_key {
        let id = p.visible_id_i32(key).map_err(Error::Provider)?;
        let exact = f::from_scaled_u128(id < 0, u128::from(id.unsigned_abs()), 0)
            .map_err(Error::Arithmetic)?;
        result.source_result_qword = f::to_f64_nearest(exact)
            .map_err(Error::Arithmetic)?
            .to_bits();
        result.selected_native_id = Some(id);
    }
    Ok(result)
}

#[cfg(test)]
#[path = "nearest_numeric_pc53_tests.rs"]
mod pc53_tests;
