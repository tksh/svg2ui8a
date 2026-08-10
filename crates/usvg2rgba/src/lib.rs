mod core;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub async fn usvg2rgba(usvg: Box<[u8]>, options: JsValue) -> Result<JsValue, JsValue> {
  let _ = options;
  core::rasterize(&usvg, None)
    .map(|result| JsValue::from_serde(&result).unwrap())
    .map_err(|e| JsValue::from_str(&format!("usvg2rgba failed: {}", e)))
}
