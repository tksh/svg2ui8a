// Native rasterization core for usvg2rgba.

use intermediate::IntermediateV1;
use resvg::tiny_skia::{Pixmap, Transform};

/// Options for rasterization. `width`/`height` of 0 mean "omitted".
#[derive(Debug, Clone, PartialEq)]
pub struct RgbaOptions {
    pub width: u32,
    pub height: u32,
    pub alpha_mode: String,
}

impl Default for RgbaOptions {
    fn default() -> Self {
        Self {
            width: 0,
            height: 0,
            alpha_mode: "straight".to_string(),
        }
    }
}

/// Result of a rasterization: flat RGBA pixels plus the metadata a consumer
/// needs (actual dimensions and the alpha mode of `pixels`).
#[derive(Debug, Clone, PartialEq)]
pub struct RgbaResult {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
    pub alpha_mode: String,
}

/// Decode a package CBOR envelope, reconstruct the tree, and rasterize it.
///
/// Errors (malformed payload, zero natural size) are returned as `Err`, never
/// panicked on.
pub fn rasterize(bytes: &[u8], options: &RgbaOptions) -> Result<RgbaResult, String> {
    let dto =
        IntermediateV1::decode(bytes).map_err(|e| format!("usvg2rgba decode error: {}", e))?;

    let natural_w = dto.size.0;
    let natural_h = dto.size.1;
    if natural_w == 0 || natural_h == 0 {
        return Err("usvg2rgba error: zero natural size".to_string());
    }

    let tree = IntermediateV1::to_tree(&dto)
        .map_err(|e| format!("usvg2rgba tree reconstruction error: {}", e))?;

    // Apply the sizing rule (constitution §4.3):
    // both omitted → natural; one set → the other is the natural value;
    // both set → exact width × height.
    let (render_w, render_h) = if options.width == 0 && options.height == 0 {
        (natural_w, natural_h)
    } else if options.width == 0 {
        (natural_w, options.height)
    } else if options.height == 0 {
        (options.width, natural_h)
    } else {
        (options.width, options.height)
    };

    if render_w == 0 || render_h == 0 {
        return Err("usvg2rgba error: zero requested size".to_string());
    }

    // Scale the drawing to fill the pixmap; identity when rendering at the
    // natural size.
    let scale_x = render_w as f32 / natural_w as f32;
    let scale_y = render_h as f32 / natural_h as f32;

    let mut pixmap = Pixmap::new(render_w, render_h)
        .ok_or_else(|| "usvg2rgba error: pixmap allocation failed".to_string())?;
    resvg::render(
        &tree,
        Transform::from_scale(scale_x, scale_y),
        &mut pixmap.as_mut(),
    );

    let pixels = extract_pixels(&pixmap, render_w, render_h, &options.alpha_mode);

    Ok(RgbaResult {
        width: render_w,
        height: render_h,
        pixels,
        alpha_mode: options.alpha_mode.clone(),
    })
}

/// Convert a pixmap (premultiplied RGBA) into the requested alpha mode.
///
/// Straight mode un-premultiplies each channel: `c * 255 / a`, rounded.
/// Premultiplied ("as-is") mode returns the bytes exactly as tiny_skia stored
/// them. Rows are top-to-bottom, one pixel per 4 bytes (R,G,B,A).
fn extract_pixels(pixmap: &Pixmap, width: u32, height: u32, alpha_mode: &str) -> Vec<u8> {
    let premultiplied = alpha_mode == "premultiplied";
    let data = pixmap.data();
    let mut pixels = Vec::with_capacity(width as usize * height as usize * 4);

    for y in 0..height as usize {
        let row_start = y * width as usize * 4;
        for x in 0..width as usize {
            let offset = row_start + x * 4;
            let r = data[offset];
            let g = data[offset + 1];
            let b = data[offset + 2];
            let a = data[offset + 3];
            if premultiplied {
                pixels.push(r);
                pixels.push(g);
                pixels.push(b);
                pixels.push(a);
            } else if a == 0 {
                pixels.push(0);
                pixels.push(0);
                pixels.push(0);
                pixels.push(0);
            } else {
                let a32 = a as u32;
                pixels.push(((r as u32 * 255 + a32 / 2) / a32) as u8);
                pixels.push(((g as u32 * 255 + a32 / 2) / a32) as u8);
                pixels.push(((b as u32 * 255 + a32 / 2) / a32) as u8);
                pixels.push(a);
            }
        }
    }

    pixels
}
