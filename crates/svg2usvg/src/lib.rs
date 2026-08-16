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
    use intermediate::IntermediateV1;

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
    fn round_trip_through_intermediate() {
        let result = svg("<svg><rect width=\"100\" height=\"100\" fill=\"red\"/></svg>").unwrap();
        // Decode via intermediate
        let intermediate = IntermediateV1::decode(&result).expect("decode should succeed");
        // Re-encode
        let re_encoded = intermediate.encode();
        // Should produce valid CBOR
        assert!(!re_encoded.is_empty());
    }

    #[test]
    fn envelope_test_identifier_version() {
        let intermediate = IntermediateV1::default();
        let bytes = intermediate.encode();
        assert!(bytes.len() > 0);
    }
}
