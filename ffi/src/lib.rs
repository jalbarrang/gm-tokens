use gm_tokens::advisories::{self, Advisory};
use gm_tokens::domain::FragmentSource;
use gm_tokens::events::RunEvent;
use gm_tokens::mapper::{events_between, MasterData, PacketDiff, VenusDataSet};
use gm_tokens::state::{replay_seeded, RunSnapshot};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::ffi::{c_char, CStr, CString};
use std::ptr;

/// The ABI version; bump on any signature change, hosts check it once at load.
pub const ABI_VERSION: u32 = 1;

#[derive(Serialize)]
struct TrackResult {
    snapshot: RunSnapshot,
    advisories: Vec<Advisory>,
}

/// `FragmentSource` ordinal for `gm_tokens_events_between`'s `source`
/// argument, in declaration order: Training 0, TargetRace 1, NormalRace 2,
/// Rest 3, Date 4, Ancestor 5, GoddessEvent 6.
fn source_from_ordinal(value: u32) -> Option<FragmentSource> {
    match value {
        0 => Some(FragmentSource::Training),
        1 => Some(FragmentSource::TargetRace),
        2 => Some(FragmentSource::NormalRace),
        3 => Some(FragmentSource::Rest),
        4 => Some(FragmentSource::Date),
        5 => Some(FragmentSource::Ancestor),
        6 => Some(FragmentSource::GoddessEvent),
        _ => None,
    }
}

/// Reads one UTF-8 JSON document, or None on a null pointer or parse failure.
fn read_json<T: DeserializeOwned>(ptr: *const c_char) -> Option<T> {
    if ptr.is_null() {
        return None;
    }
    let bytes = unsafe { CStr::from_ptr(ptr) }.to_bytes();
    serde_json::from_slice(bytes).ok()
}

/// Writes one UTF-8 JSON document; the caller returns it with `gm_tokens_free`.
fn write_json<T: Serialize>(value: &T) -> *mut c_char {
    serde_json::to_string(value)
        .ok()
        .and_then(|json| CString::new(json).ok())
        .map_or(ptr::null_mut(), |owned| owned.into_raw())
}

/// The ABI version as a plain integer, for a load-time compatibility check.
#[no_mangle]
pub extern "C" fn gm_tokens_abi_version() -> u32 {
    ABI_VERSION
}

/// Diffs two packet documents into a `PacketDiff` document; null on a bad
/// input. Field names are the Rust types' own serde names.
#[no_mangle]
pub extern "C" fn gm_tokens_events_between(
    prev: *const c_char,
    next: *const c_char,
    masters: *const c_char,
    turn: u32,
    source: u32,
) -> *mut c_char {
    let Some(prev) = read_json::<VenusDataSet>(prev) else {
        return ptr::null_mut();
    };
    let Some(next) = read_json::<VenusDataSet>(next) else {
        return ptr::null_mut();
    };
    let Some(masters) = read_json::<MasterData>(masters) else {
        return ptr::null_mut();
    };
    let Some(source) = source_from_ordinal(source) else {
        return ptr::null_mut();
    };
    let diff: PacketDiff = events_between(&prev, &next, &masters, turn, source);
    write_json(&diff)
}

/// Replays an event-list document into `{ snapshot, advisories }`; null on a
/// bad input. Same seed, same result.
#[no_mangle]
pub extern "C" fn gm_tokens_replay(events: *const c_char, seed: u64) -> *mut c_char {
    let Some(events) = read_json::<Vec<RunEvent>>(events) else {
        return ptr::null_mut();
    };
    let snapshot = replay_seeded(&events, seed);
    let result = TrackResult {
        advisories: advisories::evaluate(&snapshot),
        snapshot,
    };
    write_json(&result)
}

/// Frees a document this library returned. Never frees a null pointer.
#[no_mangle]
pub extern "C" fn gm_tokens_free(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe { drop(CString::from_raw(ptr)) };
    }
}
