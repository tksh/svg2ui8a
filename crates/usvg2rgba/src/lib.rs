mod core;

use wasm_bindgen::prelude::*;

use intermediate::IntermediateV1;

use crate::core::render_tree_to_pixels;

// Simple RGBA result returned from usvg2rgba.
// Contains width, height, and pixel data length.
// The pixels field contains raw RGBA bytes (width * height * 4).
#[wasm_bindgen(getter_with_clone)]
pub struct RgbaResult {
    /// Width of the output pixmap in pixels.
    pub width: u32,
    /// Height of the output pixmap in pixels.
    pub height: u32,
    /// RGBA pixel data as a flat array (width * height * 4 bytes).
    pub pixels: Vec<u8>,
}

/// Options for usvg2rgba rasterization.
#[wasm_bindgen(getter_with_clone)]
pub struct Usvg2RgbaOptions {
    /// Width of the output pixmap. If None (0), the natural SVG width is used.
    pub width: u32,
    /// Height of the output pixmap. If None (0), the natural SVG height is used.
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
// Apply sizing rule, handle alpha mode, zero-initialize buffer.
// Return dimensions and alpha-mode metadata alongside `pixels`.
#[wasm_bindgen]
pub fn usvg2rgba(usvg: Box<[u8]>, options: Usvg2RgbaOptions) -> RgbaResult {
    // Decode + semantically validate via intermediate::decode
    let dto = IntermediateV1::decode(&usvg).unwrap_or_else(|e| {
        eprintln!("usvg2rgba decode error: {}", e);
        IntermediateV1::default()
    });

    // Reconstruct a supported usvg::Tree from IntermediateV1
    let tree_result = IntermediateV1::to_tree(&dto);
    let tree = match tree_result {
        Ok(t) => t,
        Err(e) => {
            eprintln!("usvg2rgba tree reconstruction error: {}", e);
            // Return default dimensions on error - pixels will be empty
            return RgbaResult {
                width: 0,
                height: 0,
                pixels: Vec::new(),
            };
        }
    };

    // Determine render dimensions using sizing rule
    let (render_width, render_height) = if options.width == 0 && options.height == 0 {
        // Both omitted → natural size
        let w = tree.size().width();
        let h = tree.size().height();
        (w as u32, h as u32)
    } else if options.width == 0 {
        // Only height set → derive width from natural aspect ratio
        let aspect = tree.size().height() as f64 / tree.size().width() as f64;
        let w = (options.height as f64 * aspect) as u32;
        (w, options.height)
    } else if options.height == 0 {
        // Only width set → derive height from natural aspect ratio
        let aspect = tree.size().height() as f64 / tree.size().width() as f64;
        let h = (options.width as f64 / aspect) as u32;
        (options.width, h)
    } else {
        // Both set → exact non-uniform scaling
        (options.width, options.height)
    };

    // Rasterize with feature-disabled resvg into RGBA pixel data
    let pixels = render_tree_to_pixels(&tree, render_width, render_height);

    RgbaResult {
        width: render_width,
        height: render_height,
        pixels,
    }
}