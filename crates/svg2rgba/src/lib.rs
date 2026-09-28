pub mod core;

use wasm_bindgen::prelude::*;

use crate::core::{rasterize_svg, RgbaOptions, UpstreamRect};

// Simple RGBA result returned from svg2rgba.
// Contains width, height, the alpha mode, and the pixel data.
// The pixels field contains raw RGBA bytes (width * height * 4).
//
// Bounding boxes travel as flattened nullable float-quads: each upstream box
// contributes four `Option<f32>` fields (x, y, width, height). wasm-bindgen
// exposes each as a `number | undefined` getter; the TS wrapper assembles a
// complete `RectF` object when all four are present and explicit `null`
// otherwise, using presence checks (never truthiness) so zero coordinates
// and zero extents survive.
#[wasm_bindgen(getter_with_clone)]
pub struct RgbaResult {
    /// Width of the output pixmap in pixels.
    pub width: u32,
    /// Height of the output pixmap in pixels.
    pub height: u32,
    /// Natural SVG width before output sizing, in SVG user units.
    pub natural_width: f32,
    /// Natural SVG height before output sizing, in SVG user units.
    pub natural_height: f32,
    /// Alpha mode of the pixels: "straight" or "premultiplied".
    pub alpha_mode: String,
    /// RGBA pixel data as a flat array (width * height * 4 bytes).
    pub pixels: Vec<u8>,
    /// `abs_bounding_box` x (canvas coordinates), if measured.
    pub abs_bounding_box_x: Option<f32>,
    /// `abs_bounding_box` y (canvas coordinates), if measured.
    pub abs_bounding_box_y: Option<f32>,
    /// `abs_bounding_box` width, if measured.
    pub abs_bounding_box_width: Option<f32>,
    /// `abs_bounding_box` height, if measured.
    pub abs_bounding_box_height: Option<f32>,
    /// `abs_stroke_bounding_box` x, if measured.
    pub abs_stroke_bounding_box_x: Option<f32>,
    /// `abs_stroke_bounding_box` y, if measured.
    pub abs_stroke_bounding_box_y: Option<f32>,
    /// `abs_stroke_bounding_box` width, if measured.
    pub abs_stroke_bounding_box_width: Option<f32>,
    /// `abs_stroke_bounding_box` height, if measured.
    pub abs_stroke_bounding_box_height: Option<f32>,
    /// `abs_layer_bounding_box` x, if measured.
    pub abs_layer_bounding_box_x: Option<f32>,
    /// `abs_layer_bounding_box` y, if measured.
    pub abs_layer_bounding_box_y: Option<f32>,
    /// `abs_layer_bounding_box` width, if measured.
    pub abs_layer_bounding_box_width: Option<f32>,
    /// `abs_layer_bounding_box` height, if measured.
    pub abs_layer_bounding_box_height: Option<f32>,
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
    /// Whether the four `region_*` fields describe a render region.
    pub region: bool,
    /// Render region x, in canvas coordinates. Used when `region` is true.
    pub region_x: f32,
    /// Render region y, in canvas coordinates. Used when `region` is true.
    pub region_y: f32,
    /// Render region width, in canvas units. Used when `region` is true.
    pub region_width: f32,
    /// Render region height, in canvas units. Used when `region` is true.
    pub region_height: f32,
}

#[wasm_bindgen]
impl Svg2RgbaOptions {
    /// Default options: straight alpha, natural size, no region.
    #[wasm_bindgen(constructor)]
    pub fn new() -> Svg2RgbaOptions {
        Svg2RgbaOptions {
            width: 0,
            height: 0,
            alpha_mode: "straight".to_string(),
            region: false,
            region_x: 0.0,
            region_y: 0.0,
            region_width: 0.0,
            region_height: 0.0,
        }
    }
}

/// Parse an SVG and render it to RGBA in one shot — no intermediate payload,
/// no cacheable-bytes contract (rename plan Phase 3).
#[wasm_bindgen]
pub fn svg2rgba(svg: &str, options: Svg2RgbaOptions) -> Result<RgbaResult, JsValue> {
    let region = if options.region {
        Some(UpstreamRect {
            x: options.region_x,
            y: options.region_y,
            width: options.region_width,
            height: options.region_height,
        })
    } else {
        None
    };
    let opts = RgbaOptions {
        width: options.width,
        height: options.height,
        alpha_mode: options.alpha_mode,
        region,
    };
    match rasterize_svg(svg, &opts) {
        Ok(result) => Ok(RgbaResult {
            width: result.width,
            height: result.height,
            natural_width: result.natural_width,
            natural_height: result.natural_height,
            alpha_mode: result.alpha_mode,
            pixels: result.pixels,
            abs_bounding_box_x: result.abs_bounding_box.map(|r| r.x),
            abs_bounding_box_y: result.abs_bounding_box.map(|r| r.y),
            abs_bounding_box_width: result.abs_bounding_box.map(|r| r.width),
            abs_bounding_box_height: result.abs_bounding_box.map(|r| r.height),
            abs_stroke_bounding_box_x: result.abs_stroke_bounding_box.map(|r| r.x),
            abs_stroke_bounding_box_y: result.abs_stroke_bounding_box.map(|r| r.y),
            abs_stroke_bounding_box_width: result.abs_stroke_bounding_box.map(|r| r.width),
            abs_stroke_bounding_box_height: result.abs_stroke_bounding_box.map(|r| r.height),
            abs_layer_bounding_box_x: result.abs_layer_bounding_box.map(|r| r.x),
            abs_layer_bounding_box_y: result.abs_layer_bounding_box.map(|r| r.y),
            abs_layer_bounding_box_width: result.abs_layer_bounding_box.map(|r| r.width),
            abs_layer_bounding_box_height: result.abs_layer_bounding_box.map(|r| r.height),
        }),
        Err(e) => Err(JsValue::from_str(&e)),
    }
}
