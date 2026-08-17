/// Core rasterization helper functions.
use resvg::usvg::Tree;
use resvg::tiny_skia::{Pixmap, Transform};

/// Rasterize a usvg::Tree into RGBA pixel data.
/// The pixels are top-down, straight alpha, in a flat Vec<u8> of length width * height * 4.
pub fn render_tree_to_pixels(tree: &Tree, width: u32, height: u32) -> Vec<u8> {
    // Create pixmap and render
    let mut pixmap = Pixmap::new(width, height).unwrap();

    resvg::render(tree, Transform::default(), &mut pixmap.as_mut());

    // Extract pixels (top-down RGBA)
    // Pixmap stores pixels in BGR(A) order bottom-up;
    // we iterate top-to-bottom and output R,G,B,A (straight alpha)
    let row_stride = (width as usize) * 4;
    let pix_data = pixmap.data();
    let pix_data_len = pix_data.len();
    let mut pixels = Vec::with_capacity(width as usize * height as usize * 4);

    let y_max = height as usize;
    let x_max = width as usize;

    for y in 0..y_max {
        let start = y * row_stride;
        // Ensure we don't read beyond pixmap data
        let end = start.min(pix_data_len).min(start + row_stride);
        let row_data = &pix_data[start..end];
        for x in 0..x_max {
            let pixel_offset = x * 4;
            // Ensure we don't read beyond row data
            if pixel_offset + 3 >= row_data.len() {
                break;
            }
            // Pixmap pixels are BGR(A) order
            let b = row_data[pixel_offset];
            let g = row_data[pixel_offset + 1];
            let r = row_data[pixel_offset + 2];
            let a = row_data[pixel_offset + 3];
            // Straight (non-premultiplied) alpha: output as-is (R, G, B, A)
            let _base = y * row_stride + pixel_offset;
            pixels.push(r);
            pixels.push(g);
            pixels.push(b);
            pixels.push(a);
        }
    }

    pixels
}