use intermediate::IntermediateV1;

pub fn svg(svg: &str) -> Result<Vec<u8>, String> {
  let intermediate = IntermediateV1::default();
  Ok(intermediate.encode())
}
