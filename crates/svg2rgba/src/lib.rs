pub mod core;

use wasm_bindgen::prelude::*;

use crate::core::{rasterize_svg, RgbaOptions};

// Simple RGBA result returned from svg2rgba.
// Contains width, height, the alpha mode, and the pixel data.
// The pixels field contains raw RGBA bytes (width * height * 4).
#[wasm_bindgen(getter_with_clone)]
pub struct RgbaResult {
    /// Width of the output pixmap in pixels.
    pub width: u32,
    /// Height of the output pixmap in pixels.
    pub height: u32,
    /// Alpha mode of the pixels: "straight" or "premultiplied".
    pub alpha_mode: String,
    /// RGBA pixel data as a flat array (width * height * 4 bytes).
    pub pixels: Vec<u8>,
}

/// Options for svg2rgba rasterization.
#[wasm_bindgen(getter_with_clone)]
pub struct Svg2RgbaOptions {
    /// Width of the output pixmap. If None (0), the natural SVG width is used.
    pub width: u32,
    /// Height of the output pixmap. If None (0), the natural SVG height is used.
    pub height: u32,
    /// Alpha mode: "straight" or "premultiplied".
    pub alpha_mode: String,
}

#[wasm_bindgen]
impl Svg2RgbaOptions {
    /// Default options: straight alpha, natural size.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Svg2RgbaOptions {
        Svg2RgbaOptions {
            width: 0,
            height: 0,
            alpha_mode: "straight".to_string(),
        }
    }
}

/// Parse an SVG and render it to RGBA in one shot — no intermediate payload,
/// no cacheable-bytes contract (rename plan Phase 3).
#[wasm_bindgen]
pub fn svg2rgba(svg: &str, options: Svg2RgbaOptions) -> Result<RgbaResult, JsValue> {
    let opts = RgbaOptions {
        width: options.width,
        height: options.height,
        alpha_mode: options.alpha_mode,
    };
    match rasterize_svg(svg, &opts) {
        Ok(result) => Ok(RgbaResult {
            width: result.width,
            height: result.height,
            alpha_mode: result.alpha_mode,
            pixels: result.pixels,
        }),
        Err(e) => Err(JsValue::from_str(&e)),
    }
}
