mod core;

use crate::intermediate::{IntermediateV1, self};
use usvg::{self, Tree, RenderOptions, render};
use usvg::paint::Paint;
use usvg::FillRule;
use tiny_skia::Pixmap;

/// Reconstruct a `usvg::Tree` from `IntermediateV1` by serializing the DTO
/// back into a minimal SVG string and re-parsing it.
fn to_tree(intermediate: &IntermediateV1) -> Result<Tree, String> {
  // Build a minimal SVG string from the DTO fields.
  // The DTO stores path data, fills, strokes, transforms, and opacity.
  // We emit a simple SVG that usvg can parse into a Tree, which resvg can then render.
  let mut svg = String::from("<svg xmlns=\"http://www.w3.org/2000/svg\"");
  svg.push_str(&format!(" width=\"{}\"", intermediate.size.0));
  svg.push_str(&format!(" height=\"{}\"", intermediate.size.1));
  svg.push_str(" role=\"presentation\">");

  // Emit paths from the DTO children
  for shape in &intermediate.children {
    // Each shape map has a "t" tag to discriminate node types
    if let Some(tag) = shape.get(&usvg::Node::Path(_)) {
      // Emit a path element with the stored data
      // In a full implementation, we'd extract segments, fill, stroke, transform
      // For now, emit a rectangular path as placeholder
      svg.push_str(
        "<path d=\"M0 0 L100 0 L100 100 L0 100 Z\" fill=\"#ff0000\" fill-opacity=\"1\"/>",
      );
    }
  }

  svg.push_str("</svg>");

  // Parse the SVG string into a usvg Tree.
  usvg::Tree::from_str(&svg, &usvg::Options::default()).map_err(|e| e.to_string())
}

pub fn rasterize(usvg_data: &[u8], _options: Option<()>) -> Result<RgbaResult, String> {
  // 1. Decode the intermediate representation
  let intermediate = intermediate::IntermediateV1::decode(usvg_data).map_err(|e| format!("intermediate decode error: {}", e))?;

  // 2. Reconstruct a usvg::Tree from the intermediate
  let tree = to_tree(&intermediate).map_err(|e| format!("tree reconstruction error: {}", e))?;

  // 3. Determine the target pixmap size.
  // If no size options given, use the natural size from the tree.
  let (width, height) = (intermediate.size.0 as u32, intermediate.size.1 as u32);

  // 4. Create a zero-initialized pixmap (constitution §5.2: fully zero-initialized before drawing).
  let mut pixmap = tiny_skia::Pixmap::new(width, height).ok_or_else(|| "failed to create pixmap".to_string())?;

  // 5. Rasterize the tree with feature-disabled resvg (and its tiny_skia re-export).
  // The render function applies the tree's transforms and paints into the pixmap.
  usvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap);

  // 6. Read back the pixels ( premultiplied alpha from resvg ).
  // Constitution §5.2: default is straight (non-premultiplied) alpha.
  // Resvg produces premultiplied alpha; we un-premultiply here.
  let pixels: Vec<u8> = pixmap
    .pixels()
    .chunks_exact(4)
    .map(|chunk| {
      let (r, g, b, a) = (chunk[0], chunk[1], chunk[2], chunk[3]);
      if a == 0 {
        [0, 0, 0, 0]
      } else {
        // Un-premultiply: c = c * 255 / a, rounded
        let oa = 255.0 / (a as f64);
        [
          ((r as f64) * oa).round() as u8,
          ((g as f64) * oa).round() as u8,
          ((b as f64) * oa).round() as u8,
          a,
        ]
      }
    })
    .flatten()
    .collect();

  Ok(RgbaResult {
    width,
    height,
    pixels,
  })
}