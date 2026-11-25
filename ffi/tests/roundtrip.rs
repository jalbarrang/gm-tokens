use gm_tokens_ffi::*;
use serde_json::Value;
use std::ffi::{c_char, CStr, CString};

fn owned(json: &str) -> *const c_char {
    CString::new(json).unwrap().into_raw() as *const c_char
}

fn back(ptr: *mut c_char) -> String {
    let text = unsafe { CStr::from_ptr(ptr) }.to_string_lossy().into_owned();
    gm_tokens_free(ptr);
    text
}

#[test]
fn abi_version_is_reported() {
    assert_eq!(gm_tokens_abi_version(), 1);
}

#[test]
fn events_between_round_trips() {
    let prev = owned("{}");
    let next = owned(
        r#"{"spirit_info_array":[{"spirit_num":0,"spirit_id":501,"effect_group_id":10}]}"#,
    );
    let masters = owned(r#"{"spirit_color":{"501":9040},"effect_attribute":{"10":"speed"}}"#);

    let diff = gm_tokens_events_between(prev, next, masters, 12, 0);
    assert!(!diff.is_null());
    let value: Value = serde_json::from_str(&back(diff)).unwrap();
    assert_eq!(value["events"][0]["color"], "red");
    assert_eq!(value["events"][0]["training"], "speed");
    assert_eq!(value["unresolved_slots"], 0);

    gm_tokens_free(prev as *mut c_char);
    gm_tokens_free(next as *mut c_char);
    gm_tokens_free(masters as *mut c_char);
}

#[test]
fn replay_returns_snapshot_and_advisories() {
    let events = owned(
        r#"[{"type":"phase_started","turn":1,"phase":"junior"},
            {"type":"fragment_gained","turn":1,"color":"red","training":"speed","source":"training"}]"#,
    );

    let result = gm_tokens_replay(events, 42);
    assert!(!result.is_null());
    let value: Value = serde_json::from_str(&back(result)).unwrap();
    assert_eq!(value["snapshot"]["pending_slots"].as_array().unwrap().len(), 1);
    assert!(!value["advisories"].as_array().unwrap().is_empty());

    gm_tokens_free(events as *mut c_char);
}

#[test]
fn a_bad_document_returns_null() {
    let bad = owned("not json");
    assert!(gm_tokens_replay(bad, 42).is_null());
    gm_tokens_free(bad as *mut c_char);
}
