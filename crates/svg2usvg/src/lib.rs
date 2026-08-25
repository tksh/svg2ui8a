pub mod core;

use wasm_bindgen::prelude::*;

use crate::core::svg_to_usvg_bytes;

/// Parse an SVG and return the normalized usvg XML as UTF-8 bytes.
///
/// The return is `Uint8Array` (not a JS string) so every public payload in
/// `svg2ui8a` remains `Uint8Array`. Sync export; the TS wrapper exposes it as
/// `Promise<Uint8Array>` per constitution §4.4.
#[wasm_bindgen]
pub fn svg2usvg(svg: &str) -> Result<Box<[u8]>, JsValue> {
    svg_to_usvg_bytes(svg)
        .map(|bytes| bytes.into_boxed_slice())
        .map_err(|e| JsValue::from_str(&format!("svg2usvg failed: {}", e)))
}

pub use crate::core::svg_to_usvg_bytes as svg;
