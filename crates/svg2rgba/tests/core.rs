// Native tests for the svg2rgba one-shot rasterizer.
//
// General-purpose scope: whatever feature-disabled usvg/resvg support, minus
// `<text>` and `<image>`. Determinism is same-pixels only.

use svg2rgba::core::{rasterize_svg, RgbaOptions, UpstreamRect};

fn options(width: u32, height: u32, alpha_mode: &str) -> RgbaOptions {
    RgbaOptions {
        width,
        height,
        alpha_mode: alpha_mode.to_string(),
        region: None,
    }
}

fn region_options(width: u32, height: u32, region: (f32, f32, f32, f32)) -> RgbaOptions {
    let (x, y, w, h) = region;
    RgbaOptions {
        width,
        height,
        alpha_mode: "straight".to_string(),
        region: Some(UpstreamRect {
            x,
            y,
            width: w,
            height: h,
        }),
    }
}

/// A full-canvas red stroke (a 10-wide vertical stroke centered on x=5 covers
/// the whole 10×10 canvas) — used for uniform-color assertions.
const RED_CANVAS_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><path d="M 5 0 L 5 10" stroke="#ff0000" stroke-width="10" fill="none"/></svg>"##;
const FRACTIONAL_CANVAS_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="27.9" height="31"><rect width="27.9" height="31" fill="#ff0000"/></svg>"##;

#[test]
fn simple_svg_renders_to_natural_size() {
    let result = rasterize_svg(RED_CANVAS_SVG, &options(0, 0, "straight")).expect("should succeed");
    assert_eq!(result.width, 10);
    assert_eq!(result.height, 10);
    assert_eq!(result.natural_width, 10.0);
    assert_eq!(result.natural_height, 10.0);
    assert_eq!(result.pixels.len(), 10 * 10 * 4);
}

#[test]
fn width_only_uses_natural_height() {
    let result =
        rasterize_svg(RED_CANVAS_SVG, &options(100, 0, "straight")).expect("should succeed");
    assert_eq!((result.width, result.height), (100, 100));
    assert_eq!(result.pixels.len(), 100 * 100 * 4);
}

#[test]
fn height_only_uses_natural_width() {
    let result =
        rasterize_svg(RED_CANVAS_SVG, &options(0, 30, "straight")).expect("should succeed");
    assert_eq!((result.width, result.height), (30, 30));
}

#[test]
fn fractional_natural_size_is_used_for_height_only_aspect_ratio() {
    let result =
        rasterize_svg(FRACTIONAL_CANVAS_SVG, &options(0, 256, "straight")).expect("should succeed");
    assert_eq!((result.width, result.height), (230, 256));
    assert_eq!((result.natural_width, result.natural_height), (27.9, 31.0));
}

#[test]
fn fractional_natural_size_is_used_for_width_only_aspect_ratio() {
    let result =
        rasterize_svg(FRACTIONAL_CANVAS_SVG, &options(279, 0, "straight")).expect("should succeed");
    assert_eq!((result.width, result.height), (279, 310));
    assert_eq!((result.natural_width, result.natural_height), (27.9, 31.0));
}

#[test]
fn fractional_natural_size_is_rounded_when_dimensions_are_omitted() {
    let result =
        rasterize_svg(FRACTIONAL_CANVAS_SVG, &options(0, 0, "straight")).expect("should succeed");
    assert_eq!((result.width, result.height), (28, 31));
    assert_eq!((result.natural_width, result.natural_height), (27.9, 31.0));
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

const TRANSFORMED_STROKED_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><g transform="translate(10 20) scale(2 3)"><rect x="1" y="2" width="4" height="5" stroke="#000000" stroke-width="2"/></g></svg>"##;

/// A 2×2 red square at (4, 4) on a transparent 10×10 canvas.
const OFFSET_SQUARE_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect x="4" y="4" width="2" height="2" fill="#ff0000"/></svg>"##;

/// A 20×20 red square placed entirely outside the 10×10 viewBox.
const OUTSIDE_VIEWBOX_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10" viewBox="0 0 10 10"><rect x="100" y="100" width="20" height="20" fill="#ff0000"/></svg>"##;

/// Straightlines-style stroke band: `stroke-width="31"` on lines at x=15/16
/// spans x=-0.5..31.5, i.e. half a unit beyond the 31×31 canvas on both sides.
const OVERFLOWING_STROKE_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="31" height="31" viewBox="0 0 31 31"><g stroke="#888888" stroke-width="31"><path d="M15,0 L15,31"/><path d="M16,0 L16,31"/></g></svg>"##;

#[test]
fn bounding_boxes_are_always_populated() {
    let result = rasterize_svg(RED_CANVAS_SVG, &options(0, 0, "straight")).expect("should succeed");
    assert!(
        result.abs_bounding_box.is_some(),
        "abs box must be populated"
    );
    assert!(
        result.abs_stroke_bounding_box.is_some(),
        "stroke box must be populated"
    );
    assert!(
        result.abs_layer_bounding_box.is_some(),
        "layer box must be populated"
    );
    // Existing metadata and full pixel buffers are unchanged by the addition.
    assert_eq!((result.width, result.height), (10, 10));
    assert_eq!((result.natural_width, result.natural_height), (10.0, 10.0));
    assert!(
        result.pixels.chunks(4).all(|px| px == [255, 0, 0, 255]),
        "pixels must match the pre-change baseline byte-for-byte"
    );
}

#[test]
fn abs_bounding_box_matches_usvg_root() {
    let result =
        rasterize_svg(TRANSFORMED_STROKED_SVG, &options(0, 0, "straight")).expect("should succeed");
    let fill = result.abs_bounding_box.expect("abs box must be populated");
    assert_eq!(
        (fill.x, fill.y, fill.width, fill.height),
        (12.0, 26.0, 8.0, 15.0)
    );
}

#[test]
fn abs_stroke_bounding_box_includes_stroke() {
    let result =
        rasterize_svg(TRANSFORMED_STROKED_SVG, &options(0, 0, "straight")).expect("should succeed");
    let fill = result.abs_bounding_box.expect("abs box must be populated");
    let stroke = result
        .abs_stroke_bounding_box
        .expect("stroke box must be populated");
    assert_eq!(
        (stroke.x, stroke.y, stroke.width, stroke.height),
        (10.0, 23.0, 12.0, 21.0)
    );
    assert!(
        stroke.width >= fill.width && stroke.height >= fill.height,
        "stroke box must cover at least the fill box"
    );
}

#[test]
fn abs_layer_bounding_box_equals_stroke_box_without_filters() {
    let result =
        rasterize_svg(TRANSFORMED_STROKED_SVG, &options(0, 0, "straight")).expect("should succeed");
    assert_eq!(
        result.abs_layer_bounding_box,
        result.abs_stroke_bounding_box
    );
}

#[test]
fn abs_layer_bounding_box_expands_with_filter_region() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><defs><filter id="f" x="-50%" y="-50%" width="200%" height="200%"><feGaussianBlur stdDeviation="5"/></filter></defs><rect x="10" y="10" width="20" height="20" filter="url(#f)"/></svg>"##;
    let result = rasterize_svg(svg, &options(0, 0, "straight")).expect("should succeed");
    let stroke = result
        .abs_stroke_bounding_box
        .expect("stroke box must be populated");
    let layer = result
        .abs_layer_bounding_box
        .expect("layer box must be populated");
    assert_eq!(
        (stroke.x, stroke.y, stroke.width, stroke.height),
        (10.0, 10.0, 20.0, 20.0)
    );
    assert_eq!(
        (layer.x, layer.y, layer.width, layer.height),
        (0.0, 0.0, 40.0, 40.0)
    );
}

#[test]
fn empty_document_reports_zero_boxes_and_layer_placeholder() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"></svg>"##;
    let result = rasterize_svg(svg, &options(0, 0, "straight")).expect("should succeed");
    let fill = result.abs_bounding_box.expect("abs box must be populated");
    let stroke = result
        .abs_stroke_bounding_box
        .expect("stroke box must be populated");
    let layer = result
        .abs_layer_bounding_box
        .expect("layer box must be populated");
    assert_eq!(
        (fill.x, fill.y, fill.width, fill.height),
        (0.0, 0.0, 0.0, 0.0)
    );
    assert_eq!(
        (stroke.x, stroke.y, stroke.width, stroke.height),
        (0.0, 0.0, 0.0, 0.0)
    );
    assert_eq!(
        (layer.x, layer.y, layer.width, layer.height),
        (0.0, 0.0, 1.0, 1.0)
    );
}

#[test]
fn boxes_are_deterministic_across_calls() {
    let a =
        rasterize_svg(TRANSFORMED_STROKED_SVG, &options(0, 0, "straight")).expect("should succeed");
    let b =
        rasterize_svg(TRANSFORMED_STROKED_SVG, &options(0, 0, "straight")).expect("should succeed");
    assert_eq!(a.abs_bounding_box, b.abs_bounding_box);
    assert_eq!(a.abs_stroke_bounding_box, b.abs_stroke_bounding_box);
    assert_eq!(a.abs_layer_bounding_box, b.abs_layer_bounding_box);
}

#[test]
fn region_omitted_and_full_canvas_region_match_baseline() {
    let baseline =
        rasterize_svg(RED_CANVAS_SVG, &options(0, 0, "straight")).expect("should succeed");
    let explicit = rasterize_svg(
        RED_CANVAS_SVG,
        &region_options(0, 0, (0.0, 0.0, 10.0, 10.0)),
    )
    .expect("should succeed");
    assert_eq!(
        (baseline.width, baseline.height),
        (explicit.width, explicit.height)
    );
    assert_eq!(baseline.pixels, explicit.pixels);
}

#[test]
fn region_selects_the_source_window() {
    // 1:1: the window contains only the red square.
    let square = rasterize_svg(
        OFFSET_SQUARE_SVG,
        &region_options(0, 0, (4.0, 4.0, 2.0, 2.0)),
    )
    .expect("should succeed");
    assert_eq!((square.width, square.height), (2, 2));
    assert!(square.pixels.chunks(4).all(|px| px == [255, 0, 0, 255]));

    // Scaled: the translate-then-scale order must keep the square centered.
    let scaled = rasterize_svg(
        OFFSET_SQUARE_SVG,
        &region_options(4, 0, (4.0, 4.0, 2.0, 2.0)),
    )
    .expect("should succeed");
    assert_eq!((scaled.width, scaled.height), (4, 4));
    assert!(scaled.pixels.chunks(4).all(|px| px == [255, 0, 0, 255]));

    // A window elsewhere on the canvas is transparent.
    let empty = rasterize_svg(
        OFFSET_SQUARE_SVG,
        &region_options(0, 0, (0.0, 0.0, 2.0, 2.0)),
    )
    .expect("should succeed");
    assert!(empty.pixels.chunks(4).all(|px| px == [0, 0, 0, 0]));
}

#[test]
fn region_reveals_content_outside_the_viewbox() {
    let clipped =
        rasterize_svg(OUTSIDE_VIEWBOX_SVG, &options(0, 0, "straight")).expect("should succeed");
    assert!(clipped.pixels.chunks(4).all(|px| px == [0, 0, 0, 0]));

    let revealed = rasterize_svg(
        OUTSIDE_VIEWBOX_SVG,
        &region_options(0, 0, (100.0, 100.0, 20.0, 20.0)),
    )
    .expect("should succeed");
    assert_eq!((revealed.width, revealed.height), (20, 20));
    assert!(revealed.pixels.chunks(4).all(|px| px == [255, 0, 0, 255]));
}

#[test]
fn region_reveals_negative_origin_content() {
    let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect x="-5" y="-5" width="5" height="5" fill="#ff0000"/></svg>"##;
    let clipped = rasterize_svg(svg, &options(0, 0, "straight")).expect("should succeed");
    assert!(clipped.pixels.chunks(4).all(|px| px == [0, 0, 0, 0]));

    let revealed =
        rasterize_svg(svg, &region_options(0, 0, (-5.0, -5.0, 5.0, 5.0))).expect("should succeed");
    assert_eq!((revealed.width, revealed.height), (5, 5));
    assert!(revealed.pixels.chunks(4).all(|px| px == [255, 0, 0, 255]));
}

#[test]
fn region_covers_a_bounding_box_that_overflows() {
    let natural =
        rasterize_svg(OVERFLOWING_STROKE_SVG, &options(0, 0, "straight")).expect("should succeed");
    let stroke = natural
        .abs_stroke_bounding_box
        .expect("stroke box must be populated");
    assert_eq!(
        (stroke.x, stroke.y, stroke.width, stroke.height),
        (-0.5, 0.0, 32.0, 31.0)
    );

    let full = rasterize_svg(
        OVERFLOWING_STROKE_SVG,
        &region_options(0, 0, (stroke.x, stroke.y, stroke.width, stroke.height)),
    )
    .expect("should succeed");
    assert_eq!((full.width, full.height), (32, 31));
    // The extreme columns cover canvas x = -0.5 and x = 31.5: the parts of the
    // stroke band that the natural canvas clips. They must be opaque grey.
    let left = &full.pixels[..4];
    let last = ((full.height as usize - 1) * full.width as usize + full.width as usize - 1) * 4;
    let right = &full.pixels[last..last + 4];
    assert_eq!(left, [136, 136, 136, 255]);
    assert_eq!(right, [136, 136, 136, 255]);
}

#[test]
fn region_sizing_rule_uses_the_region_aspect() {
    // FRACTIONAL_CANVAS_SVG is 27.9×31 naturally.
    let region = (0.0, 0.0, 27.9, 31.0);
    let omitted = rasterize_svg(FRACTIONAL_CANVAS_SVG, &region_options(0, 0, region))
        .expect("should succeed");
    assert_eq!((omitted.width, omitted.height), (28, 31));

    let width_only = rasterize_svg(FRACTIONAL_CANVAS_SVG, &region_options(279, 0, region))
        .expect("should succeed");
    assert_eq!((width_only.width, width_only.height), (279, 310));

    let height_only = rasterize_svg(FRACTIONAL_CANVAS_SVG, &region_options(0, 256, region))
        .expect("should succeed");
    assert_eq!((height_only.width, height_only.height), (230, 256));

    let both = rasterize_svg(FRACTIONAL_CANVAS_SVG, &region_options(40, 30, region))
        .expect("should succeed");
    assert_eq!((both.width, both.height), (40, 30));

    // Region aspect (10×5), not natural aspect (10×10), drives the derived side.
    let aspect = rasterize_svg(
        RED_CANVAS_SVG,
        &region_options(100, 0, (0.0, 0.0, 10.0, 5.0)),
    )
    .expect("should succeed");
    assert_eq!((aspect.width, aspect.height), (100, 50));
}

#[test]
fn region_is_deterministic_across_calls() {
    let opts = region_options(0, 0, (4.0, 4.0, 2.0, 2.0));
    let a = rasterize_svg(OFFSET_SQUARE_SVG, &opts).expect("should succeed");
    let b = rasterize_svg(OFFSET_SQUARE_SVG, &opts).expect("should succeed");
    assert_eq!(a, b);
}

#[test]
fn invalid_region_errors() {
    let bad = [
        (0.0, 0.0, 0.0, 10.0),
        (0.0, 0.0, 10.0, -1.0),
        (f32::NAN, 0.0, 10.0, 10.0),
        (0.0, 0.0, f32::INFINITY, 10.0),
    ];
    for region in bad {
        assert!(
            rasterize_svg(RED_CANVAS_SVG, &region_options(0, 0, region)).is_err(),
            "{region:?} must be rejected"
        );
    }
}

#[test]
fn region_does_not_change_metadata_or_boxes() {
    let plain =
        rasterize_svg(TRANSFORMED_STROKED_SVG, &options(0, 0, "straight")).expect("should succeed");
    let windowed = rasterize_svg(
        TRANSFORMED_STROKED_SVG,
        &region_options(0, 0, (10.0, 23.0, 12.0, 21.0)),
    )
    .expect("should succeed");
    assert_eq!(
        (plain.natural_width, plain.natural_height),
        (windowed.natural_width, windowed.natural_height)
    );
    assert_eq!(plain.abs_bounding_box, windowed.abs_bounding_box);
    assert_eq!(
        plain.abs_stroke_bounding_box,
        windowed.abs_stroke_bounding_box
    );
    assert_eq!(
        plain.abs_layer_bounding_box,
        windowed.abs_layer_bounding_box
    );
    assert_eq!((plain.width, plain.height), (100, 100));
    assert_eq!((windowed.width, windowed.height), (12, 21));
}
