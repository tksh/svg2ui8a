mod core;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn svg2usvg(svg: &str) -> Result<Box<[u8]>, JsValue> {
    core::svg(svg)
        .map(|bytes| bytes.into_boxed_slice())
        .map_err(|e| JsValue::from_str(&format!("svg2usvg failed: {}", e)))
}
