use intermediate::{IntermediateV1, Paint, Shape};

#[test]
fn encode_decode_round_trip() {
  let intermediate = IntermediateV1 {
    shapes: vec![Shape {
      path_data: vec![0x01, 0x02, 0x03],
      fill: Some(Paint::Color(0xff0000)),
      opacity: 0.5,
    }],
  };

  let bytes = intermediate.encode();
  let decoded = IntermediateV1::decode(&bytes).expect("decode should succeed");
  assert_eq!(decoded, intermediate);
}

#[test]
fn invalid_identifier_rejected() {
  let bytes = vec![0xa1, 0x00, 0x66, b'b', b'a', b'd'];
  assert!(IntermediateV1::decode(&bytes).is_err());
}
