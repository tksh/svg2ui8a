// Native tests for the usvg2rgba core rasterizer (engineering-playbook §3.2).
//
// Fixtures are built with the package's own envelope helpers: construct an
// `IntermediateV1` and call `.encode()` (canonical CBOR). Malformed envelopes
// are assembled directly with `cbor_core`, the mandated codec.

use cbor_core::{EncodeFormat, SequenceWriter, Value};
use intermediate::{IntermediateV1, Paint, Shape};
use usvg2rgba::core::{rasterize, RgbaOptions};

fn options(width: u32, height: u32, alpha_mode: &str) -> RgbaOptions {
    RgbaOptions {
        width,
        height,
        alpha_mode: alpha_mode.to_string(),
    }
}

/// Valid envelope with no shapes and the given natural size.
fn natural_cbor(size: (u32, u32)) -> Vec<u8> {
    let dto = IntermediateV1 {
        shapes: vec![],
        size,
    };
    dto.encode()
}

/// Valid envelope with a full-canvas red rectangle at the given opacity.
fn red_rect_cbor(size: (u32, u32), opacity: f32) -> Vec<u8> {
    let dto = IntermediateV1 {
        shapes: vec![Shape {
            path_data: vec![],
            fill: Some(Paint::Color(0xff0000)),
            opacity,
        }],
        size,
    };
    dto.encode()
}

/// Hand-assembled envelope with a custom identifier / version / payload.
fn envelope(identifier: &str, version: u64, payload: Value<'static>) -> Vec<u8> {
    let top = Value::map([
        (Value::from(0u64), Value::from(identifier)),
        (Value::from(1u64), Value::from(version)),
        (Value::from(2u64), payload),
    ]);
    let mut buffer = Vec::new();
    let mut writer = SequenceWriter::new(&mut buffer, EncodeFormat::Binary);
    writer.write_item(&top).unwrap();
    buffer
}

/// Payload with one shape whose fill tag is unsupported.
fn unsupported_fill_variant_payload() -> Value<'static> {
    Value::map([
        (
            Value::from("shapes"),
            Value::array([Value::map([
                (Value::from("path_data"), Value::from(Vec::<u8>::new())),
                (
                    Value::from("fill"),
                    Value::array([Value::from(5u64), Value::from(0u64)]),
                ),
                (Value::from("opacity"), Value::from(1.0f64)),
            ])]),
        ),
        (
            Value::from("size"),
            Value::array([Value::from(10.0f64), Value::from(10.0f64)]),
        ),
    ])
}

/// Payload missing the `size` field (malformed DTO).
fn missing_size_payload() -> Value<'static> {
    Value::map([(Value::from("shapes"), Value::array([Value::from(0u64)]))])
}

#[test]
fn simple_svg_renders_to_natural_size() {
    let bytes = natural_cbor((20, 20));
    let result = rasterize(&bytes, &options(0, 0, "straight")).expect("rasterize should succeed");
    assert_eq!(result.width, 20);
    assert_eq!(result.height, 20);
    assert_eq!(result.pixels.len(), 20 * 20 * 4);
}

#[test]
fn width_only_uses_natural_height() {
    let bytes = natural_cbor((20, 10));
    let result = rasterize(&bytes, &options(100, 0, "straight")).expect("rasterize should succeed");
    assert_eq!(result.width, 100);
    assert_eq!(result.height, 10);
    assert_eq!(result.pixels.len(), 100 * 10 * 4);
}

#[test]
fn height_only_uses_natural_width() {
    let bytes = natural_cbor((20, 10));
    let result = rasterize(&bytes, &options(0, 100, "straight")).expect("rasterize should succeed");
    assert_eq!(result.width, 20);
    assert_eq!(result.height, 100);
    assert_eq!(result.pixels.len(), 20 * 100 * 4);
}

#[test]
fn both_set_non_uniform_scaling_is_exact() {
    let bytes = natural_cbor((20, 10));
    let result =
        rasterize(&bytes, &options(100, 30, "straight")).expect("rasterize should succeed");
    assert_eq!(result.width, 100);
    assert_eq!(result.height, 30);
    assert_eq!(result.pixels.len(), 100 * 30 * 4);
}

#[test]
fn non_canonical_and_non_cbor_payloads_error_not_panic() {
    let valid = natural_cbor((10, 10));
    let mut non_canonical = vec![0x11];
    non_canonical.extend_from_slice(&valid);
    assert!(rasterize(&non_canonical, &options(0, 0, "straight")).is_err());

    assert!(rasterize(b"not cbor at all", &options(0, 0, "straight")).is_err());

    // A well-formed CBOR value that is not a package envelope.
    let mut buffer = Vec::new();
    let mut writer = SequenceWriter::new(&mut buffer, EncodeFormat::Binary);
    writer.write_item(&Value::from(42u64)).unwrap();
    assert!(rasterize(&buffer, &options(0, 0, "straight")).is_err());
}

#[test]
fn unknown_identifier_unsupported_version_malformed_dto_and_variant_error() {
    let opts = options(0, 0, "straight");

    let empty_payload = Value::map(Vec::<(Value<'static>, Value<'static>)>::new());
    let unknown_id = envelope("other/format", 1, empty_payload.clone());
    assert!(rasterize(&unknown_id, &opts).is_err());

    let bad_version = envelope("svg2ui8a/usvg", 2, empty_payload);
    assert!(rasterize(&bad_version, &opts).is_err());

    let malformed = envelope("svg2ui8a/usvg", 1, missing_size_payload());
    assert!(rasterize(&malformed, &opts).is_err());

    let unsupported = envelope("svg2ui8a/usvg", 1, unsupported_fill_variant_payload());
    assert!(rasterize(&unsupported, &opts).is_err());
}

#[test]
fn zero_sized_svg_errors() {
    let bytes = natural_cbor((0, 0));
    assert!(rasterize(&bytes, &options(0, 0, "straight")).is_err());
}

#[test]
fn default_alpha_straight_red_is_unpremultiplied() {
    let bytes = red_rect_cbor((10, 10), 0.5);
    let result = rasterize(&bytes, &options(0, 0, "straight")).expect("rasterize should succeed");
    assert_eq!(result.width, 10);
    assert_eq!(result.height, 10);
    assert_eq!(&result.pixels[0..4], &[255, 0, 0, 128]);
}

#[test]
fn premultiplied_alpha_is_as_is() {
    let bytes = red_rect_cbor((10, 10), 0.5);
    let result =
        rasterize(&bytes, &options(0, 0, "premultiplied")).expect("rasterize should succeed");
    assert_eq!(result.width, 10);
    assert_eq!(result.height, 10);
    assert_eq!(&result.pixels[0..4], &[128, 0, 0, 128]);
}

#[test]
fn renderer_is_deterministic_across_calls() {
    let bytes = red_rect_cbor((10, 10), 0.5);
    let opts = options(0, 0, "straight");
    let result1 = rasterize(&bytes, &opts).expect("rasterize should succeed");
    let result2 = rasterize(&bytes, &opts).expect("rasterize should succeed");
    assert_eq!(result1.pixels, result2.pixels);
    assert_eq!(result1, result2);
}
