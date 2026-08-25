// Native core for svg2usvg.
//
// Parses an SVG with feature-disabled `usvg` and returns the UTF-8 bytes of
// the normalized usvg XML string (default XmlOptions, no pretty-print). The
// output is not a JS string — the Wasm boundary is `Uint8Array` so every
// public payload remains `Uint8Array` (constitution §1, plan 0.3.0 §2).

use usvg::{Indent, Tree, WriteOptions};

/// Convert an SVG string to the normalized usvg XML bytes.
///
/// Rejects `<text>` / `<image>` content (constitution §3.1/§3.3, postponed
/// fonts/BBox but still rejected for 0.3.0). Returns `Err` rather than
/// panicking on any malformed input.
pub fn svg_to_usvg_bytes(svg: &str) -> Result<Vec<u8>, String> {
    let lower = svg.to_lowercase();
    if lower.contains("<text") || lower.contains("<image") {
        return Err("SVG contains text or image content, which is not supported.".to_string());
    }

    let tree = Tree::from_str(svg, &usvg::Options::default()).map_err(|e| e.to_string())?;

    // Minimal payload: no pretty-print, no extra header — per 0.3.0 plan §11 Q5
    // (human-approved). Default WriteOptions pretty-prints with 4-space indent,
    // so override to `None` for minimal size / performance.
    let xml = tree.to_string(&WriteOptions {
        indent: Indent::None,
        ..WriteOptions::default()
    });
    Ok(xml.into_bytes())
}
