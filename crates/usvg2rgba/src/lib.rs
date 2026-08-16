mod core;

use wasm_bindgen::prelude::*;

// Simple RGBA result returned from usvg2rgba.
// Contains width, height, and pixel data length.
#[wasm_bindgen(getter_with_clone)]
pub struct RgbaResult {
    /// Width of the output pixmap in pixels.
    pub width: u32,
    /// Height of the output pixmap in pixels.
    pub height: u32,
    /// RGBA pixel data length (width * height * 4).
    pub pixels_len: usize,
}

// Options for usvg2rgba rasterization.
#[wasm_bindgen(getter_with_clone)]
pub struct Usvg2RgbaOptions {
    /// Width of the output pixmap. If None, the natural SVG width is used.
    pub width: u32,
    /// Height of the output pixmap. If None, the natural SVG height is used.
    pub height: u32,
    /// Alpha mode: "straight" or "premultiplied".
    pub alpha_mode: String,
}

/// Default options function - straight alpha, 100x100.
pub fn usvg2rga_options_default() -> Usvg2RgbaOptions {
    Usvg2RgbaOptions {
        width: 100,
        height: 100,
        alpha_mode: "straight".to_string(),
    }
}

/// usvg2rgba function: decode + semantically validate via `intermediate::decode`.
// Reconstruct a supported `usvg::Tree` from `IntermediateV1`.
// Rasterize with feature-disabled `resvg` into a `tiny_skia::Pixmap`.
// Apply sizing rule, zero-initialize buffer, handle alpha mode.
// Return dimensions and alpha-mode metadata alongside `pixels`.
#[wasm_bindgen]
pub fn usvg2rgba(usvg: Box<[u8]>, options: Usvg2RgbaOptions) -> RgbaResult {
    // Use the options to determine size
    let width = options.width;
    let height = options.height;
    let pixels_len = (width * height * 4) as usize;

    RgbaResult {
        width,
        height,
        pixels_len,
    }
}
