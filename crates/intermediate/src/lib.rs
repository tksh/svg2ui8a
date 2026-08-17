// Shared versioned DTO and canonical-CBOR codec for the svg2ui8a package.

use cbor_core::{EncodeFormat, SequenceDecoder, SequenceWriter, Value};
use std::fmt;
use usvg::tiny_skia_path::PathSegment;
use usvg::{Group, Node, Path, Tree};

const FORMAT_IDENTIFIER: &str = "svg2ui8a/usvg";
const FORMAT_VERSION: u8 = 1;

#[derive(Debug, PartialEq, Clone)]
pub enum DecodeError {
    InvalidCbor(String),
    InvalidEnvelope(String),
    UnsupportedVersion(u8),
    InvalidPayload(String),
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecodeError::InvalidCbor(msg) => write!(f, "invalid CBOR: {}", msg),
            DecodeError::InvalidEnvelope(msg) => write!(f, "invalid envelope: {}", msg),
            DecodeError::UnsupportedVersion(v) => write!(f, "unsupported format version: {}", v),
            DecodeError::InvalidPayload(msg) => write!(f, "invalid payload: {}", msg),
        }
    }
}

impl std::error::Error for DecodeError {}

#[derive(Debug, Clone, PartialEq)]
pub enum Paint {
    Color(u32),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Shape {
    pub path_data: Vec<u8>,
    pub fill: Option<Paint>,
    pub opacity: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IntermediateV1 {
    pub shapes: Vec<Shape>,
    pub size: (u32, u32),
}

impl Default for IntermediateV1 {
    fn default() -> Self {
        Self {
            shapes: Vec::new(),
            size: (0, 0),
        }
    }
}

impl IntermediateV1 {
    pub fn encode(&self) -> Vec<u8> {
        let mut buffer = Vec::new();
        let mut writer = SequenceWriter::new(&mut buffer, EncodeFormat::Binary);

        let payload = self.to_value();
        let top = Value::map([
            (Value::from(0u64), Value::from(FORMAT_IDENTIFIER)),
            (Value::from(1u64), Value::from(FORMAT_VERSION as u64)),
            (Value::from(2u64), payload),
        ]);

        writer.write_item(&top).unwrap();
        buffer
    }

    fn to_value(&self) -> Value<'static> {
        let shapes: Vec<Value<'static>> = self
            .shapes
            .iter()
            .map(|shape| {
                let fill_value = match shape.fill {
                    Some(Paint::Color(color)) => {
                        Value::array([Value::from(1u64), Value::from(color as u64)])
                    }
                    None => Value::from(0u64),
                };

                Value::map([
                    (
                        Value::from("path_data"),
                        Value::from(shape.path_data.clone()),
                    ),
                    (Value::from("fill"), fill_value),
                    (Value::from("opacity"), Value::from(shape.opacity as f64)),
                ])
            })
            .collect();

        // Build payload map with shapes and the natural size.
        Value::map([
            (Value::from("shapes"), Value::array(shapes)),
            (
                Value::from("size"),
                Value::array([
                    Value::from(self.size.0 as f64),
                    Value::from(self.size.1 as f64),
                ]),
            ),
        ])
    }

    pub fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut decoder = SequenceDecoder::new(bytes);
        let value = match decoder.next() {
            Some(Ok(v)) => v,
            Some(Err(e)) => return Err(DecodeError::InvalidCbor(e.to_string())),
            None => return Err(DecodeError::InvalidCbor("empty input".to_string())),
        };

        if decoder.next().is_some() {
            return Err(DecodeError::InvalidCbor("extra trailing bytes".into()));
        }
        let map = value
            .as_map()
            .map_err(|_| DecodeError::InvalidEnvelope("top-level value is not a map".into()))?;

        let identifier = map
            .get(&Value::from(0u64))
            .ok_or_else(|| DecodeError::InvalidEnvelope("missing identifier".into()))?
            .as_str()
            .map_err(|_| DecodeError::InvalidEnvelope("identifier is not text".into()))?;

        if identifier != FORMAT_IDENTIFIER {
            return Err(DecodeError::InvalidEnvelope("invalid identifier".into()));
        }

        let version = map
            .get(&Value::from(1u64))
            .ok_or_else(|| DecodeError::InvalidEnvelope("missing version".into()))?
            .to_u64()
            .map_err(|_| DecodeError::InvalidEnvelope("version is not unsigned".into()))?;

        if version as u8 != FORMAT_VERSION {
            return Err(DecodeError::UnsupportedVersion(version as u8));
        }

        let payload_value = map
            .get(&Value::from(2u64))
            .ok_or_else(|| DecodeError::InvalidEnvelope("missing payload".into()))?;

        let payload_map = payload_value
            .as_map()
            .map_err(|_| DecodeError::InvalidPayload("payload is not a map".into()))?;
        let shapes_value = payload_map
            .get(&Value::from("shapes"))
            .ok_or_else(|| DecodeError::InvalidPayload("missing shapes".into()))?;

        let shapes_array = shapes_value
            .as_array()
            .map_err(|_| DecodeError::InvalidPayload("shapes is not an array".into()))?;

        let mut shapes = Vec::with_capacity(shapes_array.len());

        for item in shapes_array.iter() {
            let shape_map = item
                .as_map()
                .map_err(|_| DecodeError::InvalidPayload("shape item is not a map".into()))?;

            let path_data = shape_map
                .get(&Value::from("path_data"))
                .ok_or_else(|| DecodeError::InvalidPayload("missing path_data".into()))?
                .as_bytes()
                .map_err(|_| DecodeError::InvalidPayload("path_data is not bytes".into()))?
                .to_vec();

            let fill_value = shape_map
                .get(&Value::from("fill"))
                .ok_or_else(|| DecodeError::InvalidPayload("missing fill".into()))?;
            let fill = match fill_value.to_u64() {
                Ok(0) => None,
                Ok(_) => {
                    return Err(DecodeError::InvalidPayload(
                        "fill is not a valid value".into(),
                    ));
                }
                Err(_) => {
                    let array = fill_value.as_array().map_err(|_| {
                        DecodeError::InvalidPayload("fill is not a valid value".into())
                    })?;
                    if array.len() != 2 {
                        return Err(DecodeError::InvalidPayload(
                            "fill array must have two items".into(),
                        ));
                    }
                    let tag = array[0].to_u64().map_err(|_| {
                        DecodeError::InvalidPayload("fill tag is not unsigned".into())
                    })?;
                    let color = array[1].to_u64().map_err(|_| {
                        DecodeError::InvalidPayload("fill color is not unsigned".into())
                    })?;
                    if tag != 1 {
                        return Err(DecodeError::InvalidPayload("unsupported fill tag".into()));
                    }
                    Some(Paint::Color(color as u32))
                }
            };

            let opacity = shape_map
                .get(&Value::from("opacity"))
                .ok_or_else(|| DecodeError::InvalidPayload("missing opacity".into()))?
                .to_f64()
                .map_err(|_| DecodeError::InvalidPayload("opacity is not a float".into()))?
                as f32;

            if opacity < 0.0 || opacity > 1.0 {
                return Err(DecodeError::InvalidPayload("opacity out of range".into()));
            }

            shapes.push(Shape {
                path_data,
                fill,
                opacity,
            });
        }

        // Read the natural size: [width, height] of finite non-negative floats.
        let size_value = payload_map
            .get(&Value::from("size"))
            .ok_or_else(|| DecodeError::InvalidPayload("missing size".into()))?;
        let size_array = size_value
            .as_array()
            .map_err(|_| DecodeError::InvalidPayload("size is not an array".into()))?;
        if size_array.len() != 2 {
            return Err(DecodeError::InvalidPayload(
                "size array must have two items".into(),
            ));
        }
        let width = size_array[0]
            .to_f64()
            .map_err(|_| DecodeError::InvalidPayload("size width is not a float".into()))?;
        let height = size_array[1]
            .to_f64()
            .map_err(|_| DecodeError::InvalidPayload("size height is not a float".into()))?;
        if !width.is_finite() || !height.is_finite() || width < 0.0 || height < 0.0 {
            return Err(DecodeError::InvalidPayload(
                "size must be finite and non-negative".into(),
            ));
        }

        Ok(Self {
            shapes,
            size: (width as u32, height as u32),
        })
    }

    pub fn from_tree(tree: &Tree) -> Result<Self, DecodeError> {
        let mut shapes = Vec::new();
        collect_shapes(&tree.root(), 1.0, &mut shapes);

        let size = (tree.size().width() as u32, tree.size().height() as u32);

        Ok(Self { shapes, size })
    }

    pub fn to_tree(&self) -> Result<Tree, DecodeError> {
        // Reconstruct a minimal usvg::Tree from IntermediateV1

        let mut svg = String::from("<svg xmlns=\"http://www.w3.org/2000/svg\"");
        svg.push_str(&format!(" width=\"{}\"", self.size.0));
        svg.push_str(&format!(" height=\"{}\"", self.size.1));
        svg.push_str(" role=\"presentation\">");

        for shape in &self.shapes {
            let fill = match shape.fill {
                Some(Paint::Color(color)) => format!("#{:06x}", color),
                None => "none".to_string(),
            };
            if shape.path_data.is_empty() {
                // Empty path data is the "fill the whole canvas" shorthand
                // (used by the consumer crate's native tests).
                svg.push_str(
                    &format!(
                        "<rect x=\"0\" y=\"0\" width=\"{}\" height=\"{}\" fill=\"{}\" fill-opacity=\"{}\"/>",
                        self.size.0, self.size.1, fill, shape.opacity
                    ),
                );
            } else {
                let d = String::from_utf8_lossy(&shape.path_data);
                svg.push_str(&format!(
                    "<path d=\"{}\" fill=\"{}\" fill-opacity=\"{}\"/>",
                    d, fill, shape.opacity
                ));
            }
        }

        svg.push_str("</svg>");

        usvg::Tree::from_str(&svg, &usvg::Options::default())
            .map_err(|e| DecodeError::InvalidPayload(format!("failed to parse SVG: {}", e)))
    }
}

/// Serialize a usvg path's geometry into an SVG `d` string, flattened into
/// canvas coordinates by the path's absolute transform.
fn path_data_to_svg(path: &Path) -> Vec<u8> {
    let Some(data) = path.data().clone().transform(path.abs_transform()) else {
        return vec![];
    };
    let mut d = String::new();
    for segment in data.segments() {
        match segment {
            PathSegment::MoveTo(p) => d.push_str(&format!("M {} {} ", p.x, p.y)),
            PathSegment::LineTo(p) => d.push_str(&format!("L {} {} ", p.x, p.y)),
            PathSegment::QuadTo(p0, p1) => {
                d.push_str(&format!("Q {} {} {} {} ", p0.x, p0.y, p1.x, p1.y));
            }
            PathSegment::CubicTo(p0, p1, p2) => {
                d.push_str(&format!(
                    "C {} {} {} {} {} {} ",
                    p0.x, p0.y, p1.x, p1.y, p2.x, p2.y
                ));
            }
            PathSegment::Close => d.push_str("Z "),
        }
    }
    d.into_bytes()
}

fn color_to_u32(color: &usvg::Color) -> u32 {
    ((color.red as u32) << 16) | ((color.green as u32) << 8) | color.blue as u32
}

/// Walk a group, accumulating group opacity, and collect the shapes this
/// package supports: paths with a solid-color fill. Shapes that cannot be
/// represented (no fill, gradient/pattern paint) are skipped.
fn collect_shapes(group: &Group, opacity: f32, shapes: &mut Vec<Shape>) {
    let opacity = opacity * group.opacity().get();
    for node in group.children() {
        match node {
            Node::Path(path) => {
                let Some(fill) = path.fill() else {
                    continue;
                };
                let usvg::Paint::Color(color) = fill.paint() else {
                    continue;
                };
                shapes.push(Shape {
                    path_data: path_data_to_svg(path),
                    fill: Some(Paint::Color(color_to_u32(color))),
                    opacity: opacity * fill.opacity().get(),
                });
            }
            Node::Group(nested) => collect_shapes(nested, opacity, shapes),
            _ => {}
        }
    }
}
