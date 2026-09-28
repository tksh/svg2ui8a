// Native one-shot rasterization core for svg2rgba.
//
// General-purpose: parse an SVG with feature-disabled `usvg` and render it
// directly with `resvg`. No CBOR envelope, no `intermediate` crate, no
// cacheable-bytes contract — determinism is asserted only as "same input →
// same pixels".

use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg;

/// Options for rasterization. `width`/`height` of 0 mean "omitted";
/// `region` of `None` means "the full natural canvas".
#[derive(Debug, Clone, PartialEq)]
pub struct RgbaOptions {
    pub width: u32,
    pub height: u32,
    pub alpha_mode: String,
    pub region: Option<UpstreamRect>,
}

impl Default for RgbaOptions {
    fn default() -> Self {
        Self {
            width: 0,
            height: 0,
            alpha_mode: "straight".to_string(),
            region: None,
        }
    }
}

/// Result of a rasterization: flat RGBA pixels plus the metadata a consumer
/// needs (actual dimensions and the alpha mode of `pixels`).
///
/// The three bounding-box fields are verbatim upstream `usvg` measurements of
/// the parsed document root (`tree.root()`), read on every successful render.
/// They are pre-render measurements in canvas coordinates, unaffected by
/// sizing/alpha/region options. `None` is reserved for a genuinely unavailable
/// upstream measurement; with pinned `usvg 0.47.0` every successful call
/// populates all three (zero-area rects and the 1×1 layer placeholder pass
/// through verbatim).
#[derive(Debug, Clone, PartialEq)]
pub struct RgbaResult {
    pub width: u32,
    pub height: u32,
    pub natural_width: f32,
    pub natural_height: f32,
    pub pixels: Vec<u8>,
    pub alpha_mode: String,
    pub abs_bounding_box: Option<UpstreamRect>,
    pub abs_stroke_bounding_box: Option<UpstreamRect>,
    pub abs_layer_bounding_box: Option<UpstreamRect>,
}

/// Four canvas-space numbers shared by the upstream `usvg` rects and the
/// optional render region (both use the same `{x, y, width, height}` shape).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UpstreamRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Parse the SVG, apply the sizing rule, render with resvg, and return RGBA.
///
/// Text and image content are rejected (constitution §3.1/§3.3): neither is
/// compiled into the Wasm artifact, so a payload that silently omitted them
/// would be unrenderable. Errors are returned as `Err`, never panicked on.
pub fn rasterize_svg(svg: &str, options: &RgbaOptions) -> Result<RgbaResult, String> {
    let lower_svg = svg.to_lowercase();
    if lower_svg.contains("<text") || lower_svg.contains("<image") {
        return Err("SVG contains text or image content, which is not supported.".to_string());
    }

    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).map_err(|e| e.to_string())?;

    let natural_w = tree.size().width();
    let natural_h = tree.size().height();
    if natural_w <= 0.0 || natural_h <= 0.0 {
        return Err("svg2rgba error: zero natural size".to_string());
    }

    if let Some(region) = options.region {
        if !region.x.is_finite()
            || !region.y.is_finite()
            || !region.width.is_finite()
            || !region.height.is_finite()
            || region.width <= 0.0
            || region.height <= 0.0
        {
            return Err("svg2rgba error: invalid region".to_string());
        }
    }

    // The sizing rule measures the source window: the region when given, the
    // natural canvas otherwise. Natural dimensions stay metadata either way.
    let (source_w, source_h) = match options.region {
        Some(region) => (region.width, region.height),
        None => (natural_w, natural_h),
    };

    // Sizing rule (constitution §4.3): source dimensions are rounded only when
    // choosing output pixels; one set preserves the source aspect ratio.
    let (render_w, render_h) = if options.width == 0 && options.height == 0 {
        (source_w.round() as u32, source_h.round() as u32)
    } else if options.width == 0 {
        (
            (source_w * options.height as f32 / source_h).round() as u32,
            options.height,
        )
    } else if options.height == 0 {
        (
            options.width,
            (source_h * options.width as f32 / source_w).round() as u32,
        )
    } else {
        (options.width, options.height)
    };

    if render_w == 0 || render_h == 0 {
        return Err("svg2rgba error: zero requested size".to_string());
    }

    let scale_x = render_w as f32 / source_w;
    let scale_y = render_h as f32 / source_h;

    // A region shifts the source window's origin to (0, 0) in canvas space and
    // then scales it into the pixmap. Upstream `resvg::render_node` uses the
    // same shift-to-origin pattern via `Transform::pre_translate`.
    let transform = match options.region {
        Some(region) => Transform::from_scale(scale_x, scale_y).pre_translate(-region.x, -region.y),
        None => Transform::from_scale(scale_x, scale_y),
    };

    let mut pixmap = Pixmap::new(render_w, render_h)
        .ok_or_else(|| "svg2rgba error: pixmap allocation failed".to_string())?;
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    let pixels = extract_pixels(&pixmap, render_w, render_h, &options.alpha_mode);

    let root = tree.root();
    let fill = root.abs_bounding_box();
    let stroke = root.abs_stroke_bounding_box();
    let layer = root.abs_layer_bounding_box();

    Ok(RgbaResult {
        width: render_w,
        height: render_h,
        natural_width: natural_w,
        natural_height: natural_h,
        pixels,
        alpha_mode: options.alpha_mode.clone(),
        abs_bounding_box: Some(UpstreamRect {
            x: fill.x(),
            y: fill.y(),
            width: fill.width(),
            height: fill.height(),
        }),
        abs_stroke_bounding_box: Some(UpstreamRect {
            x: stroke.x(),
            y: stroke.y(),
            width: stroke.width(),
            height: stroke.height(),
        }),
        abs_layer_bounding_box: Some(UpstreamRect {
            x: layer.x(),
            y: layer.y(),
            width: layer.width(),
            height: layer.height(),
        }),
    })
}

/// Convert a pixmap (premultiplied RGBA) into the requested alpha mode.
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
