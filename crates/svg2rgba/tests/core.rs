// Native tests for the svg2rgba one-shot rasterizer.
//
// General-purpose scope: whatever feature-disabled usvg/resvg support, minus
// `<text>` and `<image>`. Determinism is same-pixels only.

use svg2rgba::core::{rasterize_svg, RgbaOptions};

fn options(width: u32, height: u32, alpha_mode: &str) -> RgbaOptions {
    RgbaOptions {
        width,
        height,
        alpha_mode: alpha_mode.to_string(),
    }
}

/// A full-canvas red stroke (a 10-wide vertical stroke centered on x=5 covers
/// the whole 10×10 canvas) — used for uniform-color assertions.
const RED_CANVAS_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><path d="M 5 0 L 5 10" stroke="#ff0000" stroke-width="10" fill="none"/></svg>"##;

#[test]
fn simple_svg_renders_to_natural_size() {
    let result = rasterize_svg(RED_CANVAS_SVG, &options(0, 0, "straight")).expect("should succeed");
    assert_eq!(result.width, 10);
    assert_eq!(result.height, 10);
    assert_eq!(result.pixels.len(), 10 * 10 * 4);
}

#[test]
fn width_only_uses_natural_height() {
    let result =
        rasterize_svg(RED_CANVAS_SVG, &options(100, 0, "straight")).expect("should succeed");
    assert_eq!((result.width, result.height), (100, 10));
    assert_eq!(result.pixels.len(), 100 * 10 * 4);
}

#[test]
fn height_only_uses_natural_width() {
    let result =
        rasterize_svg(RED_CANVAS_SVG, &options(0, 30, "straight")).expect("should succeed");
    assert_eq!((result.width, result.height), (10, 30));
}

#[test]
fn both_set_non_uniform_scaling_is_exact() {
    let result =
        rasterize_svg(RED_CANVAS_SVG, &options(40, 30, "straight")).expect("should succeed");
    assert_eq!((result.width, result.height), (40, 30));
}

#[test]
fn full_coverage_renders_uniform_red() {
    let result = rasterize_svg(RED_CANVAS_SVG, &options(0, 0, "straight")).expect("should succeed");
    assert!(
        result.pixels.chunks(4).all(|px| px == [255, 0, 0, 255]),
        "every pixel should be opaque red"
    );
}

#[test]
fn default_alpha_straight_and_premultiplied_modes() {
    // A 50%-opaque red full-canvas stroke.
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><path d="M 5 0 L 5 10" stroke="#ff0000" stroke-width="10" fill="none" stroke-opacity="0.5"/></svg>"##;
    let straight = rasterize_svg(svg, &options(0, 0, "straight")).expect("should succeed");
    assert_eq!(&straight.pixels[0..4], &[255, 0, 0, 128]);
    let premultiplied =
        rasterize_svg(svg, &options(0, 0, "premultiplied")).expect("should succeed");
    assert_eq!(&premultiplied.pixels[0..4], &[128, 0, 0, 128]);
}

#[test]
fn malformed_svg_errors_not_panics() {
    assert!(rasterize_svg("<svg>unclosed", &options(0, 0, "straight")).is_err());
    assert!(rasterize_svg("", &options(0, 0, "straight")).is_err());
}

#[test]
fn text_content_errors() {
    let svg =
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><text>hi</text></svg>"##;
    assert!(
        rasterize_svg(svg, &options(0, 0, "straight")).is_err(),
        "<text> content must be rejected"
    );
}

#[test]
fn image_content_errors() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><image href="x.png"/></svg>"##;
    assert!(
        rasterize_svg(svg, &options(0, 0, "straight")).is_err(),
        "<image> content must be rejected"
    );
}

#[test]
fn general_purpose_scope_supports_rects_curves_gradients() {
    // Rects, curves, and gradients are all in scope for the one-shot path.
    let rect = r##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="10" height="10" fill="#00ff00"/></svg>"##;
    assert!(rasterize_svg(rect, &options(0, 0, "straight")).is_ok());

    let curve = r##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><path d="M 1 5 Q 5 0 9 5" stroke="#000" stroke-width="1" fill="none"/></svg>"##;
    assert!(rasterize_svg(curve, &options(0, 0, "straight")).is_ok());

    let gradient = r##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><defs><linearGradient id="g"><stop offset="0" stop-color="#fff"/><stop offset="1" stop-color="#000"/></linearGradient></defs><rect width="10" height="10" fill="url(#g)"/></svg>"##;
    assert!(rasterize_svg(gradient, &options(0, 0, "straight")).is_ok());
}

#[test]
fn renderer_is_deterministic_across_calls() {
    let a = rasterize_svg(RED_CANVAS_SVG, &options(0, 0, "straight")).expect("should succeed");
    let b = rasterize_svg(RED_CANVAS_SVG, &options(0, 0, "straight")).expect("should succeed");
    assert_eq!(a, b);
}
