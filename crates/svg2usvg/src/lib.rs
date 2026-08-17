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
    use intermediate::{IntermediateV1, Paint};

    #[test]
    fn simple_svg_returns_non_empty() {
        let result = svg("<svg><rect width=\"100\" height=\"100\" fill=\"red\"/></svg>");
        assert!(result.is_ok());
        let bytes = result.unwrap();
        assert!(
            !bytes.is_empty(),
            "svg2usvg should return non-empty bytes for a simple SVG"
        );
    }

    #[test]
    fn same_svg_identical_bytes() {
        let svg_str = "<svg><rect width=\"100\" height=\"100\" fill=\"red\"/></svg>";
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

    #[test]
    fn round_trip_through_intermediate_is_content_verifying() {
        let svg_str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10" viewBox="0 0 10 10"><rect x="2" y="2" width="6" height="6" fill="#ff0000" opacity="0.5"/></svg>"##;
        let bytes = svg(svg_str).expect("svg should convert");
        let intermediate = IntermediateV1::decode(&bytes).expect("decode should succeed");

        assert_eq!(intermediate.size, (10, 10));
        assert_eq!(intermediate.shapes.len(), 1, "expected exactly one shape");

        let shape = &intermediate.shapes[0];
        assert!(
            !shape.path_data.is_empty(),
            "geometry must be present in path_data"
        );
        assert_eq!(shape.fill, Some(Paint::Color(0xff0000)));
        assert!(
            (shape.opacity - 0.5).abs() < 1e-3,
            "opacity should be 0.5, got {}",
            shape.opacity
        );

        // encode → decode → the same DTO (content-preserving round trip).
        let re_decoded =
            IntermediateV1::decode(&intermediate.encode()).expect("re-encode should decode");
        assert_eq!(re_decoded, intermediate);
    }

    #[test]
    fn envelope_test_identifier_version() {
        let intermediate = IntermediateV1::default();
        let bytes = intermediate.encode();
        assert!(bytes.len() > 0);
    }
}
