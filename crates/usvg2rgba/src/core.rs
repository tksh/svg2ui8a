use serde::Serialize;

#[derive(Serialize)]
pub struct RgbaResult {
  pub width: u32,
  pub height: u32,
  pub pixels: Vec<u8>,
}

pub fn rasterize(_usvg: &[u8], _options: Option<()>) -> Result<RgbaResult, String> {
  Ok(RgbaResult { width: 0, height: 0, pixels: Vec::new() })
}
