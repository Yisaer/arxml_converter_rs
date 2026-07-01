//! Integration tests — end-to-end ARXML → decode scenarios.
//!
//! These tests exercise the full public API from file loading through
//! type resolution to binary decoding.

use arxml_converter_rs::{ArxmlCodec, Value};

/// Path to the minimal CP ARXML test fixture.
const MINIMAL_CP: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/test_data/minimal_cp.arxml");

// ---------------------------------------------------------------------------
// Happy-path
// ---------------------------------------------------------------------------

#[test]
fn load_and_resolve_speed_type() {
    let codec = ArxmlCodec::load(MINIMAL_CP).unwrap();
    let dt = codec.resolve_cp(10, 100).unwrap();
    assert_eq!(dt.short_name, "SpeedType");
    assert_eq!(dt.category, "VALUE");
}

#[test]
fn decode_u32_from_cp_arxml() {
    let codec = ArxmlCodec::load(MINIMAL_CP).unwrap();
    // SpeedType is a VALUE → uint32 (big-endian)
    let v = codec.decode_cp(10, 100, &[0x00, 0x00, 0x00, 0x2A]).unwrap();
    assert_eq!(v, Value::U32(42));
}

#[test]
fn decode_another_u32_value() {
    let codec = ArxmlCodec::load(MINIMAL_CP).unwrap();
    // Max u32
    let v = codec.decode_cp(10, 100, &[0xFF, 0xFF, 0xFF, 0xFF]).unwrap();
    assert_eq!(v, Value::U32(0xFFFF_FFFF));
}

#[test]
fn decode_zero_value() {
    let codec = ArxmlCodec::load(MINIMAL_CP).unwrap();
    let v = codec.decode_cp(10, 100, &[0, 0, 0, 0]).unwrap();
    assert_eq!(v, Value::U32(0));
}

// ---------------------------------------------------------------------------
// Error cases
// ---------------------------------------------------------------------------

#[test]
fn unknown_service_id() {
    let codec = ArxmlCodec::load(MINIMAL_CP).unwrap();
    let err = codec.resolve_cp(999, 100).unwrap_err();
    assert!(err.contains("no service found"));
}

#[test]
fn unknown_header_id() {
    let codec = ArxmlCodec::load(MINIMAL_CP).unwrap();
    let err = codec.resolve_cp(10, 999).unwrap_err();
    assert!(err.contains("no header ref"));
}

#[test]
fn decode_with_insufficient_bytes() {
    let codec = ArxmlCodec::load(MINIMAL_CP).unwrap();
    // SpeedType needs 4 bytes (u32), we only give 2
    let err = codec.decode_cp(10, 100, &[0x00, 0x01]).unwrap_err();
    assert!(err.contains("not enough bytes"));
}

#[test]
fn file_not_found() {
    let err = ArxmlCodec::load("/nonexistent/path.arxml").unwrap_err();
    assert!(err.contains("failed to read"));
}

#[test]
fn invalid_xml() {
    let dir = std::env::temp_dir();
    let path = dir.join("invalid.arxml");
    std::fs::write(&path, "<not><valid>").unwrap();
    let err = ArxmlCodec::load(&path).unwrap_err();
    assert!(err.contains("XML parse error") || err.contains("no <AUTOSAR>"));
    let _ = std::fs::remove_file(&path);
}
