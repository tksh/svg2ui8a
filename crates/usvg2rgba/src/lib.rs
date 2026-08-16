mod core;

// Simple RGBA result returned from usvg2rgba.
// Contains width, height, and pixel data length.
pub struct RgbaResult {
    /// Width of the output pixmap in pixels.
    pub width: u32,
    /// Height of the output pixmap in pixels.
    pub height: u32,
    /// RGBA pixel data length (width * height * 4).
    pub pixels_len: usize,
}

// Options for usvg2rgba rasterization.
pub struct Usvg2RgbaOptions {
    /// Width of the output pixmap. If None, the natural SVG width is used.
    pub width: u32,
    /// Height of the output pixmap. If None, the natural SVG height is used.
    pub height: u32,
    /// Alpha mode: "straight" or "premultiplied".
    pub alpha_mode: String,
}

// Default options function - no wasm_bindgen attribute
pub fn usvg2rga_options_default() -> Usvg2RgbaOptions {
    Usvg2RgbaOptions {
        width: 100,
        height: 100,
        alpha_mode: "straight".to_string(),
    }
}

// usvg2rgba function - no wasm_bindgen attribute
pub fn usvg2rgba_func(svg_data: &[u8], options: Usvg2RgbaOptions) -> RgbaResult {
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
