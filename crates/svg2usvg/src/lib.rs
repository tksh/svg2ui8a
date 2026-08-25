mod svg_core;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn svg2usvg(svg: &str) -> Result<Box<[u8]>, JsValue> {
    svg_core::svg(svg)
        .map(|bytes| bytes.into_boxed_slice())
        .map_err(|e| JsValue::from_str(&format!("svg2usvg failed: {}", e)))
}

pub use svg_core::svg;

#[cfg(test)]
mod tests {
    use super::svg;
    use intermediate::{Group, IntermediateV1, Node, Paint, ShapeRendering};

    /// The Straightlines sample artwork (task.md §13).
    const FIXTURE_SVG: &str = include_str!("../../../tests/fixtures/straightlines-sample.svg");

    #[test]
    fn simple_svg_returns_non_empty() {
        let result = svg("<svg shape-rendering=\"geometricPrecision\"><rect width=\"100\" height=\"100\" fill=\"red\"/></svg>");
        assert!(result.is_ok());
        let bytes = result.unwrap();
        assert!(
            !bytes.is_empty(),
            "svg2usvg should return non-empty bytes for a simple SVG"
        );
    }

    #[test]
    fn same_svg_identical_bytes() {
        let svg_str = "<svg shape-rendering=\"geometricPrecision\"><rect width=\"100\" height=\"100\" fill=\"red\"/></svg>";
        let result1 = svg(&svg_str);
        let result2 = svg(&svg_str);
        assert!(result1.is_ok() && result2.is_ok());
        let bytes1 = result1.unwrap();
        let bytes2 = result2.unwrap();
        assert_eq!(
            bytes1, bytes2,
            "two calls with the same SVG should produce identical bytes"
        );
    }

    #[test]
    fn malformed_svg_returns_error() {
        let result = svg("<svg>unclosed");
        assert!(result.is_err(), "malformed SVG should return an error");
    }

    #[test]
    fn text_content_returns_error() {
        let result = svg("<svg><text>Hello</text></svg>");
        assert!(
            result.is_err(),
            "<text> content should return an error, not payload"
        );
    }

    #[test]
    fn image_content_returns_error() {
        let result = svg("<svg><image/></svg>");
        assert!(
            result.is_err(),
            "<image> content should return an error, not payload"
        );
    }

    fn shapes_of<'a>(group: &'a Group) -> Vec<&'a intermediate::Shape> {
        group
            .children
            .iter()
            .filter_map(|node| match node {
                Node::Shape(shape) => Some(shape),
                Node::Group(_) => None,
            })
            .collect()
    }

    fn groups_of(group: &Group) -> Vec<&Group> {
        group
            .children
            .iter()
            .filter_map(|node| match node {
                Node::Group(nested) => Some(nested),
                Node::Shape(_) => None,
            })
            .collect()
    }

    #[test]
    fn round_trip_through_intermediate_is_content_verifying() {
        let svg_str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10" viewBox="0 0 10 10" shape-rendering="geometricPrecision"><rect x="2" y="2" width="6" height="6" fill="#ff0000" opacity="0.5"/></svg>"##;
        let bytes = svg(svg_str).expect("svg should convert");
        let intermediate = IntermediateV1::decode(&bytes).expect("decode should succeed");

        assert_eq!(intermediate.size, (10, 10));
        assert_eq!(
            intermediate.shape_rendering,
            ShapeRendering::GeometricPrecision
        );
        // usvg models element-level `opacity` as a compositing wrapper group;
        // the DTO preserves that group instead of flattening the opacity.
        let groups = groups_of(&intermediate.root);
        assert_eq!(groups.len(), 1, "expected one wrapper group");
        assert!(
            (groups[0].opacity - 0.5).abs() < 1e-3,
            "group opacity should be 0.5, got {}",
            groups[0].opacity
        );

        let shapes = shapes_of(groups[0]);
        assert_eq!(shapes.len(), 1, "expected exactly one shape");

        let shape = shapes[0];
        assert!(
            !shape.path_data.is_empty(),
            "geometry must be present in path_data"
        );
        assert_eq!(shape.fill, Some(Paint::Color(0xff0000)));
        assert!((shape.fill_opacity - 1.0).abs() < 1e-6);

        // encode → decode → the same DTO (content-preserving round trip).
        let re_decoded =
            IntermediateV1::decode(&intermediate.encode()).expect("re-encode should decode");
        assert_eq!(re_decoded, intermediate);
    }

    /// The fixture is four `<g>` layers of stroke-only paths. Every layer,
    /// path, stroke color/width/opacity, and the Layer2 group opacity must
    /// survive into the DTO (previously all eleven paths were dropped).
    #[test]
    fn straightlines_fixture_structure_preserved() {
        let bytes = svg(FIXTURE_SVG).expect("fixture svg should convert");
        let dto = IntermediateV1::decode(&bytes).expect("fixture output should decode");

        assert_eq!(dto.size, (31, 31), "viewBox 0 0 31 31");
        assert_eq!(
            dto.shape_rendering,
            ShapeRendering::GeometricPrecision,
            "fixture declares geometricPrecision at the root"
        );
        let layers = groups_of(&dto.root);
        assert_eq!(layers.len(), 4, "expected four <g> layers");

        let expected = [
            // (stroke color, stroke width, stroke opacity, group opacity, path count)
            (0x888888u32, 31.0f32, 1.0f32, 1.0f32, 2usize),
            (0x000000, 1.0, 1.0, 1.0, 3),
            (0x333333, 3.0, 1.0, 0.8, 3),
            (0xffffff, 5.0, 0.9, 1.0, 3),
        ];
        for (layer, (color, width, stroke_opacity, group_opacity, count)) in
            layers.iter().zip(expected.iter())
        {
            assert!((layer.opacity - group_opacity).abs() < 1e-6);
            let shapes = shapes_of(layer);
            assert_eq!(shapes.len(), *count, "path count for one layer");
            for shape in &shapes {
                assert!(
                    !shape.path_data.is_empty(),
                    "geometry must be present in every fixture path"
                );
                assert_eq!(
                    shape.fill,
                    Some(Paint::Color(0x000000)),
                    "implicit default fill is black"
                );
                let stroke = shape.stroke.as_ref().expect("fixture paths are stroked");
                assert_eq!(stroke.paint, Paint::Color(*color));
                assert!((stroke.width - width).abs() < 1e-4);
                assert!((stroke.opacity - stroke_opacity).abs() < 1e-6);
                assert!((shape.fill_opacity - 1.0).abs() < 1e-6);
            }
        }
    }

    #[test]
    fn straightlines_fixture_is_deterministic() {
        let bytes1 = svg(FIXTURE_SVG).expect("fixture svg should convert");
        let bytes2 = svg(FIXTURE_SVG).expect("fixture svg should convert");
        assert_eq!(bytes1, bytes2, "fixture conversion must be deterministic");
    }

    // --- shape-rendering enforcement (intermediate-v1-shape-rendering.md) ---

    fn svg_with_rendering(value: &str) -> String {
        format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10" viewBox="0 0 10 10" shape-rendering="{value}"><path d="M 2 2 L 8 8" stroke="#000" stroke-width="1" fill="none"/></svg>"##
        )
    }

    #[test]
    fn missing_shape_rendering_rejected() {
        let result = svg(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="10" height="10" fill="red"/></svg>"#,
        );
        assert!(
            result.is_err(),
            "SVG without shape-rendering must be rejected"
        );
    }

    #[test]
    fn auto_is_accepted_as_geometric_precision() {
        let bytes = svg(&svg_with_rendering("auto")).expect("auto must be accepted");
        let dto = IntermediateV1::decode(&bytes).expect("decode should succeed");
        assert_eq!(dto.shape_rendering, ShapeRendering::GeometricPrecision);
    }

    #[test]
    fn optimize_speed_rejected() {
        assert!(svg(&svg_with_rendering("optimizeSpeed")).is_err());
    }

    #[test]
    fn invalid_shape_rendering_value_rejected() {
        assert!(svg(&svg_with_rendering("bogus")).is_err());
    }

    #[test]
    fn both_permitted_values_accepted_with_correct_field() {
        for (value, expected) in [
            ("geometricPrecision", ShapeRendering::GeometricPrecision),
            ("crispEdges", ShapeRendering::CrispEdges),
        ] {
            let bytes = svg(&svg_with_rendering(value)).expect("permitted value");
            let dto = IntermediateV1::decode(&bytes).expect("decode should succeed");
            assert_eq!(dto.shape_rendering, expected, "value: {}", value);
        }
    }

    #[test]
    fn inconsistent_per_element_override_rejected() {
        let svg_str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10" viewBox="0 0 10 10" shape-rendering="crispEdges"><path d="M 2 2 L 8 8" stroke="#000" stroke-width="1" fill="none"/><g shape-rendering="geometricPrecision"><path d="M 2 8 L 8 2" stroke="#000" stroke-width="1" fill="none"/></g></svg>"##;
        assert!(
            svg(svg_str).is_err(),
            "mixed per-element overrides must be rejected"
        );
    }

    #[test]
    fn document_without_paths_rejected() {
        let result = svg(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10" shape-rendering="geometricPrecision"/> "#,
        );
        assert!(
            result.is_err(),
            "a document with no paths has no declared rendering to verify"
        );
    }

    #[test]
    fn envelope_test_identifier_version() {
        let intermediate = IntermediateV1::default();
        let bytes = intermediate.encode();
        assert!(!bytes.is_empty());
    }
}
