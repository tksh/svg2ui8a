// Native tests for the stln2rgba core rasterizer (engineering-playbook §3.2).
//
// Fixtures are built with the package's own envelope helpers: construct an
// `IntermediateV1` and call `.encode()` (canonical CBOR). Malformed envelopes
// are assembled directly with `cbor_core`, the mandated codec.

use cbor_core::{EncodeFormat, SequenceWriter, Value};
use intermediate::{
    Group, IntermediateV1, LineCap, LineJoin, Node, Paint, Shape, ShapeRendering, Stroke,
};
use resvg::tiny_skia::{Pixmap, Transform};
use stln2rgba::core::{rasterize, RgbaOptions};

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
        root: Group {
            opacity: 1.0,
            children: vec![],
        },
        size,
        shape_rendering: ShapeRendering::GeometricPrecision,
    };
    dto.encode()
}

/// Valid envelope with a full-canvas red rectangle at the given fill opacity.
fn red_rect_cbor(size: (u32, u32), opacity: f32) -> Vec<u8> {
    let dto = IntermediateV1 {
        root: Group {
            opacity: 1.0,
            children: vec![Node::Shape(Shape {
                path_data: vec![],
                fill: Some(Paint::Color(0xff0000)),
                fill_opacity: opacity,
                stroke: None,
            })],
        },
        size,
        shape_rendering: ShapeRendering::GeometricPrecision,
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

/// A group node map with the given children and opacity.
fn group_node(children: Vec<Value<'static>>, opacity: f64) -> Value<'static> {
    Value::map([
        (Value::from("t"), Value::from(1u64)),
        (Value::from("opacity"), Value::from(opacity)),
        (Value::from("children"), Value::array(children)),
    ])
}

/// A shape node map with individually controlled fields.
#[allow(clippy::too_many_arguments)]
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

/// Payload whose single shape carries an unsupported fill tag.
fn unsupported_fill_variant_payload() -> Value<'static> {
    let shape = shape_node(
        Value::from(Vec::<u8>::new()),
        Value::array([Value::from(5u64), Value::from(0u64)]),
        Value::from(1.0f64),
        Value::from(0u64),
    );
    Value::map([
        (Value::from("root"), group_node(vec![shape], 1.0)),
        (
            Value::from("size"),
            Value::array([Value::from(10.0f64), Value::from(10.0f64)]),
        ),
    ])
}

/// Payload missing the `size` field (malformed DTO).
fn missing_size_payload() -> Value<'static> {
    Value::map([(Value::from("root"), group_node(vec![], 1.0))])
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

    let bad_version = envelope("svg2ui8a/straightlines", 2, empty_payload);
    assert!(rasterize(&bad_version, &opts).is_err());

    let malformed = envelope("svg2ui8a/straightlines", 1, missing_size_payload());
    assert!(rasterize(&malformed, &opts).is_err());

    let unsupported = envelope(
        "svg2ui8a/straightlines",
        1,
        unsupported_fill_variant_payload(),
    );
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

#[test]
fn end_to_end_svg_rasterizes_to_expected_color() {
    // A full-canvas rectangle is not in the Straightlines subset (a rect is a
    // closed five-segment path), so this end-to-end test builds the payload
    // from the DTO directly, using the documented empty-`path_data`
    // full-canvas shorthand.
    let dto = IntermediateV1 {
        root: Group {
            opacity: 1.0,
            children: vec![Node::Shape(Shape {
                path_data: vec![],
                fill: Some(Paint::Color(0xff0000)),
                fill_opacity: 1.0,
                stroke: None,
            })],
        },
        size: (10, 10),
        shape_rendering: ShapeRendering::GeometricPrecision,
    };
    let bytes = dto.encode();

    // The bytes must carry real geometry now: decode and confirm the shape is
    // a full-canvas red path, not a blank placeholder.
    let decoded = intermediate::IntermediateV1::decode(&bytes).expect("decode should succeed");
    assert_eq!(decoded.size, (10, 10));
    let shapes: Vec<&Shape> = decoded
        .root
        .children
        .iter()
        .filter_map(|node| match node {
            Node::Shape(shape) => Some(shape),
            Node::Group(_) => None,
        })
        .collect();
    assert_eq!(shapes.len(), 1);
    assert_eq!(shapes[0].fill, Some(Paint::Color(0xff0000)));

    let result = rasterize(&bytes, &options(0, 0, "straight")).expect("rasterize should succeed");
    assert_eq!((result.width, result.height), (10, 10));

    // Non-blank and the expected color: every pixel is opaque red.
    assert_eq!(
        &result.pixels[0..4],
        &[255, 0, 0, 255],
        "first pixel should be opaque red"
    );
    assert!(
        result.pixels.chunks(4).all(|px| px == &[255, 0, 0, 255]),
        "every pixel should be opaque red, got a blank or wrong-colored canvas"
    );
}

/// The Straightlines sample artwork (task.md §13).
const FIXTURE_SVG: &str = include_str!("../../../tests/fixtures/straightlines-sample.svg");

/// No-loss proof for the fixture: rendering the DTO pipeline output must be
/// byte-identical to rendering the parsed original SVG directly. Any dropped
/// layer, stroke property, or group-opacity difference shows up here.
///
/// Comparison happens in premultiplied space (tiny-skia's native output) so
/// the reference render needs no conversion.
#[test]
fn straightlines_fixture_renders_identical_through_dto_pipeline() {
    // Reference: parse the original SVG and render it directly.
    let reference_tree = resvg::usvg::Tree::from_str(FIXTURE_SVG, &resvg::usvg::Options::default())
        .expect("fixture svg should parse");
    let mut reference = Pixmap::new(31, 31).expect("reference pixmap");
    resvg::render(
        &reference_tree,
        Transform::identity(),
        &mut reference.as_mut(),
    );

    // Pipeline: SVG → DTO → canonical CBOR → decode → reconstructed tree.
    let bytes = svg2stln::svg(FIXTURE_SVG).expect("fixture svg should convert");
    let result = rasterize(&bytes, &options(0, 0, "premultiplied"))
        .expect("fixture payload should rasterize");
    assert_eq!((result.width, result.height), (31, 31));

    // Sanity: the canvas must not be blank (the old code lost everything).
    assert!(
        result.pixels.chunks(4).any(|px| px[3] != 0),
        "fixture render must contain visible pixels"
    );

    assert_eq!(
        result.pixels,
        reference.data().to_vec(),
        "DTO pipeline must be pixel-lossless against a direct render of the original SVG"
    );
}

/// The same fixture must also survive a straight-alpha round trip and scaled
/// rendering without structural loss (sizes are asserted; exact pixels are
/// covered by the premultiplied golden test above).
#[test]
fn straightlines_fixture_straight_alpha_and_scaling() {
    let bytes = svg2stln::svg(FIXTURE_SVG).expect("fixture svg should convert");

    let straight =
        rasterize(&bytes, &options(0, 0, "straight")).expect("fixture payload should rasterize");
    assert_eq!((straight.width, straight.height), (31, 31));
    assert_eq!(straight.pixels.len(), 31 * 31 * 4);

    // Center of the white horizontal bar (y=22 row): near-opaque white in
    // straight mode (stroke-opacity 0.9 → alpha ≈ 230).
    let idx = (22 * 31 + 15) * 4;
    let [r, g, b, a] = [
        straight.pixels[idx],
        straight.pixels[idx + 1],
        straight.pixels[idx + 2],
        straight.pixels[idx + 3],
    ];
    assert!(a > 200, "white bar should be ~90%% opaque, got alpha {}", a);
    assert!(
        (225..=255).contains(&r) && (225..=255).contains(&g) && (225..=255).contains(&b),
        "expected near-white pixel, got rgb({}, {}, {})",
        r,
        g,
        b
    );

    let scaled =
        rasterize(&bytes, &options(62, 62, "straight")).expect("scaled rasterize should work");
    assert_eq!((scaled.width, scaled.height), (62, 62));
    assert_eq!(scaled.pixels.len(), 62 * 62 * 4);
    assert!(
        scaled.pixels.chunks(4).any(|px| px[3] != 0),
        "scaled render must contain visible pixels"
    );
}

/// Stroke properties beyond color/width/opacity must survive reconstruction:
/// dashed strokes render differently from solid ones.
#[test]
fn dashed_stroke_differs_from_solid_stroke() {
    fn stroked_line(dasharray: Option<Vec<f32>>) -> Vec<u8> {
        let stroke = Stroke {
            paint: Paint::Color(0x000000),
            opacity: 1.0,
            width: 1.0,
            linecap: LineCap::Butt,
            linejoin: LineJoin::Miter,
            miterlimit: 4.0,
            dasharray,
            dashoffset: 0.0,
        };
        let dto = IntermediateV1 {
            root: Group {
                opacity: 1.0,
                children: vec![Node::Shape(Shape {
                    path_data: b"M 1 5 L 9 5 ".to_vec(),
                    fill: None,
                    fill_opacity: 1.0,
                    stroke: Some(stroke),
                })],
            },
            size: (10, 10),
            shape_rendering: ShapeRendering::GeometricPrecision,
        };
        dto.encode()
    }

    let solid = rasterize(&stroked_line(None), &options(0, 0, "straight"))
        .expect("solid stroke should rasterize");
    let dashed = rasterize(
        &stroked_line(Some(vec![1.0, 1.0])),
        &options(0, 0, "straight"),
    )
    .expect("dashed stroke should rasterize");

    assert_ne!(
        solid.pixels, dashed.pixels,
        "dash pattern must affect the rendered output"
    );
}

/// `crispEdges` must disable anti-aliasing end-to-end: the DTO pipeline output
/// for each permitted value matches a direct reference render of the same SVG,
/// and the two renderings are visibly different from each other.
#[test]
fn shape_rendering_values_render_like_their_references() {
    let mut pixels_by_value = Vec::new();
    for (value, expected) in [
        ("crispEdges", ShapeRendering::CrispEdges),
        ("geometricPrecision", ShapeRendering::GeometricPrecision),
    ] {
        let svg_src = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" viewBox="0 0 32 32" shape-rendering="{value}"><path d="M 2 3 L 29 22" stroke="#000000" stroke-width="1" fill="none"/></svg>"##
        );

        // Reference: parse the source directly and render it with resvg.
        let reference_tree =
            resvg::usvg::Tree::from_str(&svg_src, &resvg::usvg::Options::default())
                .expect("reference svg should parse");
        let mut reference = Pixmap::new(32, 32).expect("reference pixmap");
        resvg::render(
            &reference_tree,
            Transform::identity(),
            &mut reference.as_mut(),
        );

        // Pipeline: SVG → DTO → canonical CBOR → decode → reconstructed tree.
        let bytes = svg2stln::svg(&svg_src).expect("svg should convert");
        let dto = intermediate::IntermediateV1::decode(&bytes).expect("decode should succeed");
        assert_eq!(dto.shape_rendering, expected);

        let result =
            rasterize(&bytes, &options(0, 0, "premultiplied")).expect("payload should rasterize");
        assert_eq!((result.width, result.height), (32, 32));

        assert_eq!(
            result.pixels,
            reference.data().to_vec(),
            "{}: DTO pipeline must match a direct render",
            value
        );
        pixels_by_value.push((value, result.pixels));
    }

    // A 1px diagonal is AA-sensitive: crispEdges (no AA, hard pixels) and
    // geometricPrecision (anti-aliased) must not produce identical canvases.
    assert_ne!(
        pixels_by_value[0].1, pixels_by_value[1].1,
        "crispEdges and geometricPrecision must render differently"
    );
}
