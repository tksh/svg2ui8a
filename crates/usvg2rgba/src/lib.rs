use wasm_bindgen::prelude::*;

#[derive(Debug, Clone, PartialEq)]
pub struct RgbaResult {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

pub fn rasterize(_usvg: &[u8], _options: Option<()>) -> Result<RgbaResult, String> {
    Ok(RgbaResult {
        width: 0,
        height: 0,
        pixels: Vec::new(),
    })
}

#[wasm_bindgen]
pub fn usvg2rgba(usvg: Box<[u8]>, options: JsValue) -> Result<String, JsValue> {
    let _ = options;
    // TODO: implement full rasterization
    Ok("width:0,height:0,pixels:[],alpha_mode:straight".to_string())
}
