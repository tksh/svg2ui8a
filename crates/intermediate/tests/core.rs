use intermediate::{IntermediateV1, Paint, Shape};

#[test]
fn encode_decode_round_trip() {
    let intermediate = IntermediateV1 {
        shapes: vec![Shape {
            path_data: vec![0x01, 0x02, 0x03],
            fill: Some(Paint::Color(0xff0000)),
            opacity: 0.5,
        }],
    };

    let bytes = intermediate.encode();
    let decoded = IntermediateV1::decode(&bytes).expect("decode should succeed");
    assert_eq!(decoded, intermediate);
}

#[test]
fn invalid_identifier_rejected() {
    let bytes = vec![0xa1, 0x00, 0x66, b'b', b'a', b'd'];
    assert!(IntermediateV1::decode(&bytes).is_err());
}

#[test]
fn invalid_version_rejected() {
    // Version 2 is not supported (only version 1 is)
    let bytes = vec![0xa1, 0x02, 0x66, b'b', b'a', b'd'];
    assert!(IntermediateV1::decode(&bytes).is_err());
}

#[test]
fn non_canonical_cbor_rejected() {
    // Use non-canonical CBOR encoding (extra leading byte on a simple value)
    // cbor_core STRICT mode rejects non-canonical encodings
    let bytes = vec![0x11, 0xa1, 0x00, 0x66, b'b', b'a', b'd'];
    assert!(IntermediateV1::decode(&bytes).is_err());
}

#[test]
fn missing_identifier_rejected() {
    // Map without key 0
    let bytes = vec![0xa1, 0x01, 0x66, b'b', b'a', b'd'];
    assert!(IntermediateV1::decode(&bytes).is_err());
}

#[test]
fn missing_version_rejected() {
    // Map without key 1
    let bytes = vec![0xa1, 0x00, 0x66, b'b', b'a', b'd'];
    assert!(IntermediateV1::decode(&bytes).is_err());
}

#[test]
fn fill_invalid_opacity_rejected() {
    let intermediate = IntermediateV1 {
        shapes: vec![Shape {
            path_data: vec![0x01, 0x02, 0x03],
            fill: Some(Paint::Color(0xff0000)),
            opacity: 1.5, // Invalid: opacity > 1
        }],
    };
    let bytes = intermediate.encode();
    assert!(IntermediateV1::decode(&bytes).is_err());
}

#[test]
fn path_data_not_bytes_rejected() {
    // path_data field is not bytes
    let bytes = vec![0xa1, 0x00, 0x01, 0xa2, 0xa3, 0x03];
    assert!(IntermediateV1::decode(&bytes).is_err());
}
