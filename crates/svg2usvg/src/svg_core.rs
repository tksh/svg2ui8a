use intermediate::IntermediateV1;

pub fn svg(svg: &str) -> Result<Vec<u8>, String> {
    // Reject SVG content containing <text> or <image> elements.
    let lower_svg = svg.to_lowercase();
    if lower_svg.contains("<text") || lower_svg.contains("<image") {
        return Err("SVG contains text or image content, which is not supported.".to_string());
    }

    // Parse the SVG with feature-disabled usvg. The `OptimizeSpeed` sentinel
    // marks "shape-rendering not declared": it is a value the Straightlines
    // spec forbids, so any path still carrying it after parsing makes
    // `IntermediateV1::from_tree` reject the document (reject-don't-guess,
    // like the <text>/<image> checks above).
    let options = usvg::Options {
        shape_rendering: usvg::ShapeRendering::OptimizeSpeed,
        ..Default::default()
    };
    let tree = usvg::Tree::from_str(svg, &options).map_err(|e| e.to_string())?;

    // Convert the parsed tree into our intermediate representation.
    let intermediate = IntermediateV1::from_tree(&tree).map_err(|e| e.to_string())?;

    // Encode the intermediate as canonical CBOR.
    Ok(intermediate.encode())
}
