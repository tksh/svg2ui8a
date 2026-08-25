use cbor_core::{EncodeFormat, SequenceWriter, Value};
use intermediate::{
    DecodeError, Group, IntermediateV1, LineCap, LineJoin, Node, Paint, Shape, ShapeRendering,
    Stroke, MAX_GROUP_DEPTH,
};

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

/// Payload with the given root node and a 10×10 size.
fn payload_with_root(root: Value<'static>) -> Value<'static> {
    Value::map([
        (Value::from("root"), root),
        (
            Value::from("size"),
            Value::array([Value::from(10.0f64), Value::from(10.0f64)]),
        ),
        // geometricPrecision; rejection tests targeting this field override it.
        (Value::from("shape_rendering"), Value::from(0u64)),
    ])
}

/// A shape node map with the given fill / fill_opacity / stroke values.
fn shape_node(
    path_data: Value<'static>,
    fill: Value<'static>,
    fill_opacity: Value<'static>,
    stroke: Value<'static>,
) -> Value<'static> {
    Value::map([
        (Value::from("t"), Value::from(0u64)),
        (Value::from("path_data"), path_data),
        (Value::from("fill"), fill),
        (Value::from("fill_opacity"), fill_opacity),
        (Value::from("stroke"), stroke),
    ])
}

fn solid_paint(color: u64) -> Value<'static> {
    Value::array([Value::from(1u64), Value::from(color)])
}

fn default_stroke() -> Stroke {
    Stroke {
        paint: Paint::Color(0x888888),
        opacity: 1.0,
        width: 3.0,
        linecap: LineCap::Butt,
        linejoin: LineJoin::Miter,
        miterlimit: 4.0,
        dasharray: None,
        dashoffset: 0.0,
    }
}

#[test]
fn encode_decode_round_trip() {
    let intermediate = IntermediateV1 {
        root: Group {
            opacity: 1.0,
            children: vec![Node::Shape(Shape {
                path_data: vec![0x01, 0x02, 0x03],
                fill: Some(Paint::Color(0xff0000)),
                fill_opacity: 0.5,
                stroke: None,
            })],
        },
        size: (100, 100),
        shape_rendering: ShapeRendering::GeometricPrecision,
    };

    let bytes = intermediate.encode();
    let decoded = IntermediateV1::decode(&bytes).expect("decode should succeed");
    assert_eq!(decoded, intermediate);
}

#[test]
fn encode_decode_round_trip_both_shape_renderings() {
    for rendering in [
        ShapeRendering::GeometricPrecision,
        ShapeRendering::CrispEdges,
    ] {
        let intermediate = IntermediateV1 {
            root: Group {
                opacity: 1.0,
                children: vec![Node::Shape(Shape {
                    path_data: b"M 0 0 L 1 1 ".to_vec(),
                    fill: None,
                    fill_opacity: 1.0,
                    stroke: None,
                })],
            },
            size: (10, 10),
            shape_rendering: rendering,
        };
        let decoded =
            IntermediateV1::decode(&intermediate.encode()).expect("decode should succeed");
        assert_eq!(decoded, intermediate);
    }
}

#[test]
fn encode_decode_round_trip_nested_groups_and_strokes() {
    let intermediate = IntermediateV1 {
        root: Group {
            opacity: 0.8,
            children: vec![
                Node::Group(Group {
                    opacity: 0.5,
                    children: vec![
                        Node::Shape(Shape {
                            path_data: b"M 1 2 L 3 4 ".to_vec(),
                            fill: None,
                            fill_opacity: 1.0,
                            stroke: Some(Stroke {
                                paint: Paint::Color(0x123456),
                                opacity: 0.9,
                                width: 5.25,
                                linecap: LineCap::Round,
                                linejoin: LineJoin::Bevel,
                                miterlimit: 2.5,
                                dasharray: Some(vec![1.0, 2.5, 0.5]),
                                dashoffset: -7.75,
                            }),
                        }),
                        Node::Shape(Shape {
                            path_data: Vec::new(),
                            fill: Some(Paint::Color(0xabcdef)),
                            fill_opacity: 0.25,
                            stroke: Some(default_stroke()),
                        }),
                    ],
                }),
                Node::Group(Group {
                    opacity: 1.0,
                    children: vec![Node::Group(Group {
                        opacity: 0.0,
                        children: vec![Node::Group(Group {
                            opacity: 1.0,
                            children: Vec::new(),
                        })],
                    })],
                }),
            ],
        },
        size: (31, 31),
        shape_rendering: ShapeRendering::CrispEdges,
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
    let payload = payload_with_root(Value::array([shape_node(
        Value::from(Vec::<u8>::new()),
        solid_paint(0xff0000),
        Value::from(1.5f64), // Invalid: opacity > 1
        Value::from(0u64),
    )]));
    let bytes = envelope("svg2ui8a/usvg", 1, payload);
    assert!(IntermediateV1::decode(&bytes).is_err());
}

#[test]
fn nan_opacity_rejected() {
    let payload = payload_with_root(Value::array([shape_node(
        Value::from(Vec::<u8>::new()),
        solid_paint(0xff0000),
        Value::from(f64::NAN),
        Value::from(0u64),
    )]));
    let bytes = envelope("svg2ui8a/usvg", 1, payload);
    assert!(IntermediateV1::decode(&bytes).is_err());
}

#[test]
fn legacy_shapes_payload_rejected() {
    // The pre-extension v1 payload used a flat "shapes" array; it has no
    // "root" key and must be rejected by the current decoder.
    let payload = Value::map([
        (Value::from("shapes"), Value::array(Vec::<Value>::new())),
        (
            Value::from("size"),
            Value::array([Value::from(10.0f64), Value::from(10.0f64)]),
        ),
    ]);
    let bytes = envelope("svg2ui8a/usvg", 1, payload);
    assert!(IntermediateV1::decode(&bytes).is_err());
}

#[test]
fn missing_size_rejected() {
    let payload = Value::map([(Value::from("root"), group_node(Vec::<Value>::new(), 1.0f64))]);
    let bytes = envelope("svg2ui8a/usvg", 1, payload);
    assert!(IntermediateV1::decode(&bytes).is_err());
}

fn group_node(children: Vec<Value<'static>>, opacity: f64) -> Value<'static> {
    Value::map([
        (Value::from("t"), Value::from(1u64)),
        (Value::from("opacity"), Value::from(opacity)),
        (Value::from("children"), Value::array(children)),
    ])
}

fn stroked_shape(stroke: Value<'static>) -> Value<'static> {
    shape_node(
        Value::from(b"M 0 0 L 1 1 ".to_vec()),
        Value::from(0u64),
        Value::from(1.0f64),
        stroke,
    )
}

fn stroke_none() -> Value<'static> {
    Value::from(0u64)
}

/// Hand-assembled stroke map with individually controlled fields.
#[allow(clippy::too_many_arguments)]
fn stroke_map(
    width: Value<'static>,
    opacity: Value<'static>,
    miterlimit: Value<'static>,
    linecap: Value<'static>,
    linejoin: Value<'static>,
    dasharray: Value<'static>,
    dashoffset: Value<'static>,
) -> Value<'static> {
    Value::map([
        (Value::from("paint"), solid_paint(0x333333)),
        (Value::from("opacity"), opacity),
        (Value::from("width"), width),
        (Value::from("linecap"), linecap),
        (Value::from("linejoin"), linejoin),
        (Value::from("miterlimit"), miterlimit),
        (Value::from("dasharray"), dasharray),
        (Value::from("dashoffset"), dashoffset),
    ])
}

fn dasharray_some(values: &[f64]) -> Value<'static> {
    Value::array(values.iter().map(|v| Value::from(*v)).collect::<Vec<_>>())
}

#[test]
fn stroke_width_zero_and_negative_rejected() {
    for width in [0.0f64, -3.0] {
        let payload = payload_with_root(Value::array([stroked_shape(stroke_map(
            Value::from(width),
            Value::from(1.0f64),
            Value::from(4.0f64),
            Value::from(0u64),
            Value::from(0u64),
            Value::from(0u64),
            Value::from(0.0f64),
        ))]));
        let bytes = envelope("svg2ui8a/usvg", 1, payload);
        assert!(
            IntermediateV1::decode(&bytes).is_err(),
            "stroke width {} must be rejected",
            width
        );
    }
}

#[test]
fn stroke_miterlimit_below_one_rejected() {
    let payload = payload_with_root(Value::array([stroked_shape(stroke_map(
        Value::from(2.0f64),
        Value::from(1.0f64),
        Value::from(0.5f64),
        Value::from(0u64),
        Value::from(0u64),
        Value::from(0u64),
        Value::from(0.0f64),
    ))]));
    let bytes = envelope("svg2ui8a/usvg", 1, payload);
    assert!(IntermediateV1::decode(&bytes).is_err());
}

#[test]
fn stroke_opacity_out_of_range_rejected() {
    let payload = payload_with_root(Value::array([stroked_shape(stroke_map(
        Value::from(2.0f64),
        Value::from(1.5f64),
        Value::from(4.0f64),
        Value::from(0u64),
        Value::from(0u64),
        Value::from(0u64),
        Value::from(0.0f64),
    ))]));
    let bytes = envelope("svg2ui8a/usvg", 1, payload);
    assert!(IntermediateV1::decode(&bytes).is_err());
}

#[test]
fn group_opacity_out_of_range_rejected() {
    let payload = payload_with_root(Value::array([group_node(Vec::new(), 1.5)]));
    let bytes = envelope("svg2ui8a/usvg", 1, payload);
    assert!(IntermediateV1::decode(&bytes).is_err());
}

#[test]
fn unknown_linecap_tag_rejected() {
    let payload = payload_with_root(Value::array([stroked_shape(stroke_map(
        Value::from(2.0f64),
        Value::from(1.0f64),
        Value::from(4.0f64),
        Value::from(3u64),
        Value::from(0u64),
        Value::from(0u64),
        Value::from(0.0f64),
    ))]));
    let bytes = envelope("svg2ui8a/usvg", 1, payload);
    assert!(IntermediateV1::decode(&bytes).is_err());
}

#[test]
fn unknown_linejoin_tag_rejected() {
    let payload = payload_with_root(Value::array([stroked_shape(stroke_map(
        Value::from(2.0f64),
        Value::from(1.0f64),
        Value::from(4.0f64),
        Value::from(0u64),
        Value::from(4u64),
        Value::from(0u64),
        Value::from(0.0f64),
    ))]));
    let bytes = envelope("svg2ui8a/usvg", 1, payload);
    assert!(IntermediateV1::decode(&bytes).is_err());
}

#[test]
fn unknown_node_discriminator_rejected() {
    let node = Value::map([
        (Value::from("t"), Value::from(7u64)),
        (Value::from("opacity"), Value::from(1.0f64)),
        (Value::from("children"), Value::array(Vec::<Value>::new())),
    ]);
    let payload = payload_with_root(Value::array([node]));
    let bytes = envelope("svg2ui8a/usvg", 1, payload);
    assert!(IntermediateV1::decode(&bytes).is_err());
}

#[test]
fn root_shape_discriminator_rejected() {
    // The root node itself must be a group, not a shape.
    let payload = payload_with_root(shape_node(
        Value::from(Vec::<u8>::new()),
        Value::from(0u64),
        Value::from(1.0f64),
        Value::from(0u64),
    ));
    let bytes = envelope("svg2ui8a/usvg", 1, payload);
    assert!(IntermediateV1::decode(&bytes).is_err());
}

#[test]
fn dasharray_invalid_entries_rejected() {
    for bad in [
        dasharray_some(&[-1.0]),
        dasharray_some(&[f64::NAN]),
        dasharray_some(&[f64::INFINITY]),
    ] {
        let payload = payload_with_root(Value::array([stroked_shape(stroke_map(
            Value::from(2.0f64),
            Value::from(1.0f64),
            Value::from(4.0f64),
            Value::from(0u64),
            Value::from(0u64),
            bad,
            Value::from(0.0f64),
        ))]));
        let bytes = envelope("svg2ui8a/usvg", 1, payload);
        assert!(IntermediateV1::decode(&bytes).is_err());
    }
}

#[test]
fn negative_dashoffset_accepted_but_nan_rejected() {
    let ok = payload_with_root(group_node(
        vec![stroked_shape(stroke_map(
            Value::from(2.0f64),
            Value::from(1.0f64),
            Value::from(4.0f64),
            Value::from(0u64),
            Value::from(0u64),
            Value::from(0u64),
            Value::from(-5.5f64),
        ))],
        1.0,
    ));
    assert!(IntermediateV1::decode(&envelope("svg2ui8a/usvg", 1, ok)).is_ok());

    let bad = payload_with_root(Value::array([stroked_shape(stroke_map(
        Value::from(2.0f64),
        Value::from(1.0f64),
        Value::from(4.0f64),
        Value::from(0u64),
        Value::from(0u64),
        Value::from(0u64),
        Value::from(f64::NAN),
    ))]));
    assert!(IntermediateV1::decode(&envelope("svg2ui8a/usvg", 1, bad)).is_err());
}

#[test]
fn fill_color_out_of_range_rejected() {
    let payload = payload_with_root(Value::array([shape_node(
        Value::from(Vec::<u8>::new()),
        Value::array([Value::from(1u64), Value::from(0x01000000u64)]),
        Value::from(1.0f64),
        Value::from(0u64),
    )]));
    let bytes = envelope("svg2ui8a/usvg", 1, payload);
    assert!(IntermediateV1::decode(&bytes).is_err());
}

#[test]
fn missing_required_keys_rejected() {
    // Missing "stroke" on a shape.
    let no_stroke = Value::map([
        (Value::from("t"), Value::from(0u64)),
        (Value::from("path_data"), Value::from(Vec::<u8>::new())),
        (Value::from("fill"), solid_paint(0xff0000)),
        (Value::from("fill_opacity"), Value::from(1.0f64)),
    ]);
    let bytes = envelope(
        "svg2ui8a/usvg",
        1,
        payload_with_root(Value::array([no_stroke])),
    );
    assert!(IntermediateV1::decode(&bytes).is_err());

    // Missing "fill_opacity" on a shape.
    let no_fill_opacity = Value::map([
        (Value::from("t"), Value::from(0u64)),
        (Value::from("path_data"), Value::from(Vec::<u8>::new())),
        (Value::from("fill"), solid_paint(0xff0000)),
        (Value::from("stroke"), Value::from(0u64)),
    ]);
    let bytes = envelope(
        "svg2ui8a/usvg",
        1,
        payload_with_root(Value::array([no_fill_opacity])),
    );
    assert!(IntermediateV1::decode(&bytes).is_err());

    // Missing "children" on a group.
    let no_children = Value::map([
        (Value::from("t"), Value::from(1u64)),
        (Value::from("opacity"), Value::from(1.0f64)),
    ]);
    let bytes = envelope(
        "svg2ui8a/usvg",
        1,
        payload_with_root(Value::array([no_children])),
    );
    assert!(IntermediateV1::decode(&bytes).is_err());
}

#[test]
fn deep_group_nesting_rejected_beyond_limit() {
    fn nested_groups(depth: usize) -> Value<'static> {
        if depth == 0 {
            return group_node(Vec::new(), 1.0);
        }
        group_node(vec![nested_groups(depth - 1)], 1.0)
    }

    // At the limit: accepted.
    let at_limit = payload_with_root(nested_groups(MAX_GROUP_DEPTH - 1));
    if let Err(e) = IntermediateV1::decode(&envelope("svg2ui8a/usvg", 1, at_limit)) {
        panic!("at_limit should decode, got: {}", e);
    }

    // One deeper: rejected with the DTO's own message, not a stack overflow
    // and not a codec-internal recursion error.
    let too_deep = payload_with_root(nested_groups(MAX_GROUP_DEPTH));
    match IntermediateV1::decode(&envelope("svg2ui8a/usvg", 1, too_deep)) {
        Err(DecodeError::InvalidPayload(msg)) => {
            assert!(msg.contains("maximum depth"), "unexpected error: {}", msg);
        }
        other => panic!("expected InvalidPayload, got {:?}", other.map(|_| ())),
    }
}

#[test]
fn shape_rendering_rejections() {
    let base = |extra: Vec<(Value<'static>, Value<'static>)>| {
        let mut fields = vec![
            (Value::from("root"), group_node(Vec::<Value>::new(), 1.0f64)),
            (
                Value::from("size"),
                Value::array([Value::from(10.0f64), Value::from(10.0f64)]),
            ),
        ];
        fields.extend(extra);
        Value::map(fields)
    };

    // Missing key entirely.
    let bytes = envelope("svg2ui8a/usvg", 1, base(vec![]));
    assert!(
        IntermediateV1::decode(&bytes).is_err(),
        "missing shape_rendering must be rejected"
    );

    // Unknown tag.
    for tag in [2u64, 7, u64::MAX] {
        let payload = base(vec![(Value::from("shape_rendering"), Value::from(tag))]);
        let bytes = envelope("svg2ui8a/usvg", 1, payload);
        assert!(
            IntermediateV1::decode(&bytes).is_err(),
            "shape_rendering tag {} must be rejected",
            tag
        );
    }

    // Non-unsigned value.
    let payload = base(vec![(
        Value::from("shape_rendering"),
        Value::from("geometricPrecision"),
    )]);
    let bytes = envelope("svg2ui8a/usvg", 1, payload);
    assert!(IntermediateV1::decode(&bytes).is_err());
}
