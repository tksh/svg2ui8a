// Shared versioned DTO and canonical-CBOR codec for the svg2ui8a package.

use cbor_core::{EncodeFormat, SequenceDecoder, SequenceWriter, Value};
use std::collections::BTreeMap;
use std::fmt;
use usvg::tiny_skia_path::PathSegment;
use usvg::{Group as UsvgGroup, Node as UsvgNode, Path as UsvgPath, Tree};

const FORMAT_IDENTIFIER: &str = "svg2ui8a/straightlines";
const FORMAT_VERSION: u8 = 1;

// Maximum supported group nesting depth. Decode rejects deeper payloads so
// recursive decoding and tree reconstruction cannot be driven to a stack
// overflow by adversarial input. Must stay comfortably below cbor_core's own
// RECURSION_LIMIT (200): each group consumes roughly two CBOR levels
// (group map -> children array), so ~99 groups would otherwise hit the codec's
// generic recursion error instead of this explicit, semantically meaningful
// one.
pub const MAX_GROUP_DEPTH: usize = 96;

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

/// `stroke-linecap`, mirroring `usvg::LineCap`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LineCap {
    Butt,
    Round,
    Square,
}

/// `stroke-linejoin`, mirroring `usvg::LineJoin`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LineJoin {
    Miter,
    MiterClip,
    Round,
    Bevel,
}

/// A resolved stroke style. All values are the effective per-path values
/// usvg computed (inheritance already applied upstream).
/// A stroke style.
#[derive(Debug, Clone, PartialEq)]
pub struct Stroke {
    pub paint: Paint,
    pub opacity: f32,
    pub width: f32,
    pub linecap: LineCap,
    pub linejoin: LineJoin,
    pub miterlimit: f32,
    pub dasharray: Option<Vec<f32>>,
    pub dashoffset: f32,
}

/// Root-level rendering hint (`shape-rendering` on the `<svg>` element).
///
/// The Straightlines spec requires every document to declare exactly one of
/// these values. usvg's other resolutions (`auto`, an omitted attribute, or
/// `optimizeSpeed`) never reach the DTO: `auto` is accepted as
/// `GeometricPrecision`, and everything else is a producer-side rejection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ShapeRendering {
    GeometricPrecision,
    CrispEdges,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Shape {
    pub path_data: Vec<u8>,
    pub fill: Option<Paint>,
    pub fill_opacity: f32,
    pub stroke: Option<Stroke>,
}

/// A layer: a composited group with its own opacity and ordered children.
#[derive(Debug, Clone, PartialEq)]
pub struct Group {
    pub opacity: f32,
    pub children: Vec<Node>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Shape(Shape),
    Group(Group),
}

#[derive(Debug, Clone, PartialEq)]
pub struct IntermediateV1 {
    pub root: Group,
    pub size: (u32, u32),
    pub shape_rendering: ShapeRendering,
}

impl Default for IntermediateV1 {
    fn default() -> Self {
        Self {
            root: Group {
                opacity: 1.0,
                children: Vec::new(),
            },
            size: (0, 0),
            shape_rendering: ShapeRendering::GeometricPrecision,
        }
    }
}

fn kv(key: &'static str, value: Value<'static>) -> (Value<'static>, Value<'static>) {
    (Value::from(key), value)
}

fn linecap_tag(cap: LineCap) -> u64 {
    match cap {
        LineCap::Butt => 0,
        LineCap::Round => 1,
        LineCap::Square => 2,
    }
}

fn linecap_from_tag(tag: u64) -> Option<LineCap> {
    match tag {
        0 => Some(LineCap::Butt),
        1 => Some(LineCap::Round),
        2 => Some(LineCap::Square),
        _ => None,
    }
}

fn linejoin_tag(join: LineJoin) -> u64 {
    match join {
        LineJoin::Miter => 0,
        LineJoin::MiterClip => 1,
        LineJoin::Round => 2,
        LineJoin::Bevel => 3,
    }
}

fn linejoin_from_tag(tag: u64) -> Option<LineJoin> {
    match tag {
        0 => Some(LineJoin::Miter),
        1 => Some(LineJoin::MiterClip),
        2 => Some(LineJoin::Round),
        3 => Some(LineJoin::Bevel),
        _ => None,
    }
}

fn shape_rendering_tag(rendering: ShapeRendering) -> u64 {
    match rendering {
        ShapeRendering::GeometricPrecision => 0,
        ShapeRendering::CrispEdges => 1,
    }
}

fn shape_rendering_from_tag(tag: u64) -> Option<ShapeRendering> {
    match tag {
        0 => Some(ShapeRendering::GeometricPrecision),
        1 => Some(ShapeRendering::CrispEdges),
        _ => None,
    }
}

fn paint_to_value(paint: &Paint) -> Value<'static> {
    match paint {
        Paint::Color(color) => Value::array([Value::from(1u64), Value::from(*color as u64)]),
    }
}

fn stroke_to_value(stroke: &Stroke) -> Value<'static> {
    let dasharray = match &stroke.dasharray {
        Some(dashes) => Value::array(
            dashes
                .iter()
                .map(|d| Value::from(*d as f64))
                .collect::<Vec<_>>(),
        ),
        None => Value::from(0u64),
    };
    Value::map([
        kv("paint", paint_to_value(&stroke.paint)),
        kv("opacity", Value::from(stroke.opacity as f64)),
        kv("width", Value::from(stroke.width as f64)),
        kv("linecap", Value::from(linecap_tag(stroke.linecap))),
        kv("linejoin", Value::from(linejoin_tag(stroke.linejoin))),
        kv("miterlimit", Value::from(stroke.miterlimit as f64)),
        kv("dasharray", dasharray),
        kv("dashoffset", Value::from(stroke.dashoffset as f64)),
    ])
}

fn shape_to_value(shape: &Shape) -> Value<'static> {
    let fill_value = match &shape.fill {
        Some(paint) => paint_to_value(paint),
        None => Value::from(0u64),
    };
    let stroke_value = match &shape.stroke {
        Some(stroke) => stroke_to_value(stroke),
        None => Value::from(0u64),
    };
    Value::map([
        kv("t", Value::from(0u64)),
        kv("path_data", Value::from(shape.path_data.clone())),
        kv("fill", fill_value),
        kv("fill_opacity", Value::from(shape.fill_opacity as f64)),
        kv("stroke", stroke_value),
    ])
}

fn group_to_value(group: &Group) -> Value<'static> {
    let children: Vec<Value<'static>> = group.children.iter().map(node_to_value).collect();
    Value::map([
        kv("t", Value::from(1u64)),
        kv("opacity", Value::from(group.opacity as f64)),
        kv("children", Value::array(children)),
    ])
}

fn node_to_value(node: &Node) -> Value<'static> {
    match node {
        Node::Group(group) => group_to_value(group),
        Node::Shape(shape) => shape_to_value(shape),
    }
}

impl IntermediateV1 {
    pub fn encode(&self) -> Vec<u8> {
        let mut buffer = Vec::new();
        let mut writer = SequenceWriter::new(&mut buffer, EncodeFormat::Binary);

        let payload = Value::map([
            kv("root", group_to_value(&self.root)),
            kv(
                "size",
                Value::array([
                    Value::from(self.size.0 as f64),
                    Value::from(self.size.1 as f64),
                ]),
            ),
            kv(
                "shape_rendering",
                Value::from(shape_rendering_tag(self.shape_rendering)),
            ),
        ]);
        let top = Value::map([
            (Value::from(0u64), Value::from(FORMAT_IDENTIFIER)),
            (Value::from(1u64), Value::from(FORMAT_VERSION as u64)),
            (Value::from(2u64), payload),
        ]);

        writer.write_item(&top).unwrap();
        buffer
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

        let root_value = field(payload_map, "root")?;
        let root = match decode_node(root_value, 0)? {
            Node::Group(group) => group,
            Node::Shape(_) => {
                return Err(DecodeError::InvalidPayload("root must be a group".into()));
            }
        };

        let shape_rendering =
            shape_rendering_from_tag(field(payload_map, "shape_rendering")?.to_u64().map_err(
                |_| DecodeError::InvalidPayload("shape_rendering is not unsigned".into()),
            )?)
            .ok_or_else(|| DecodeError::InvalidPayload("unknown shape_rendering".into()))?;

        let size_value = field(payload_map, "size")?;
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
            root,
            size: (width as u32, height as u32),
            shape_rendering,
        })
    }

    pub fn from_tree(tree: &Tree) -> Result<Self, DecodeError> {
        // The producer parses with a sentinel `Options::shape_rendering`
        // (`OptimizeSpeed`, a value the Straightlines spec forbids), so any
        // path still carrying it means the attribute was absent (or set to
        // something unparseable). Documents must declare exactly one of the
        // two permitted values, consistently for every path.
        let mut seen: Option<usvg::ShapeRendering> = None;
        let root = collect_group(tree.root(), 0, &mut seen)?;
        let shape_rendering = match seen {
            Some(usvg::ShapeRendering::CrispEdges) => ShapeRendering::CrispEdges,
            Some(usvg::ShapeRendering::GeometricPrecision) => ShapeRendering::GeometricPrecision,
            Some(usvg::ShapeRendering::OptimizeSpeed) | None => {
                return Err(DecodeError::InvalidPayload(
                    "SVG must declare shape-rendering as geometricPrecision or crispEdges on the root element".into(),
                ));
            }
        };
        let size = (tree.size().width() as u32, tree.size().height() as u32);
        Ok(Self {
            root,
            size,
            shape_rendering,
        })
    }

    pub fn to_tree(&self) -> Result<Tree, DecodeError> {
        // Reconstruct a minimal usvg::Tree from IntermediateV1. Every value is
        // emitted explicitly (no reliance on attribute inheritance); groups
        // carry their own compositing opacity so resvg renders translucent
        // layers exactly like the original tree.
        let mut svg = String::from("<svg xmlns=\"http://www.w3.org/2000/svg\"");
        svg.push_str(&format!(" width=\"{}\"", self.size.0));
        svg.push_str(&format!(" height=\"{}\"", self.size.1));
        // GeometricPrecision is usvg's default and omitted, mirroring the
        // upstream SVG writer's convention.
        if self.shape_rendering == ShapeRendering::CrispEdges {
            svg.push_str(" shape-rendering=\"crispEdges\"");
        }
        svg.push_str(" role=\"presentation\">");
        write_group(&self.root, self.size, &mut svg);
        svg.push_str("</svg>");

        usvg::Tree::from_str(&svg, &usvg::Options::default())
            .map_err(|e| DecodeError::InvalidPayload(format!("failed to parse SVG: {}", e)))
    }
}

fn field<'a>(
    map: &'a BTreeMap<Value<'a>, Value<'a>>,
    key: &'a str,
) -> Result<&'a Value<'a>, DecodeError> {
    map.get(&Value::from(key))
        .ok_or_else(|| DecodeError::InvalidPayload(format!("missing {}", key)))
}

fn unit_interval(value: &Value, what: &str) -> Result<f32, DecodeError> {
    let v = value
        .to_f64()
        .map_err(|_| DecodeError::InvalidPayload(format!("{} is not a float", what)))?;
    if !v.is_finite() || !(0.0..=1.0).contains(&v) {
        return Err(DecodeError::InvalidPayload(format!(
            "{} out of range",
            what
        )));
    }
    Ok(v as f32)
}

fn finite_f32(value: &Value, what: &str) -> Result<f32, DecodeError> {
    let v = value
        .to_f64()
        .map_err(|_| DecodeError::InvalidPayload(format!("{} is not a float", what)))?;
    if !v.is_finite() {
        return Err(DecodeError::InvalidPayload(format!(
            "{} must be finite",
            what
        )));
    }
    Ok(v as f32)
}

fn finite_positive(value: &Value, what: &str) -> Result<f32, DecodeError> {
    let v = finite_f32(value, what)?;
    if v <= 0.0 {
        return Err(DecodeError::InvalidPayload(format!(
            "{} must be positive",
            what
        )));
    }
    Ok(v)
}

fn finite_at_least(value: &Value, what: &str, min: f32) -> Result<f32, DecodeError> {
    let v = finite_f32(value, what)?;
    if v < min {
        return Err(DecodeError::InvalidPayload(format!(
            "{} must be at least {}",
            what, min
        )));
    }
    Ok(v)
}

fn decode_color(value: &Value, what: &str) -> Result<u32, DecodeError> {
    let array = value
        .as_array()
        .map_err(|_| DecodeError::InvalidPayload(format!("{} is not a valid value", what)))?;
    if array.len() != 2 {
        return Err(DecodeError::InvalidPayload(format!(
            "{} array must have two items",
            what
        )));
    }
    let tag = array[0]
        .to_u64()
        .map_err(|_| DecodeError::InvalidPayload(format!("{} tag is not unsigned", what)))?;
    if tag != 1 {
        return Err(DecodeError::InvalidPayload(format!(
            "unsupported {} tag",
            what
        )));
    }
    let color = array[1]
        .to_u64()
        .map_err(|_| DecodeError::InvalidPayload(format!("{} color is not unsigned", what)))?;
    if color > 0xff_ffff {
        return Err(DecodeError::InvalidPayload(format!(
            "{} color out of range",
            what
        )));
    }
    Ok(color as u32)
}

/// `0` means "none"; `[1, color]` means solid-color paint.
fn decode_optional_paint(value: &Value, what: &str) -> Result<Option<Paint>, DecodeError> {
    if value.to_u64() == Ok(0) {
        return Ok(None);
    }
    Ok(Some(Paint::Color(decode_color(value, what)?)))
}

fn decode_dasharray(value: &Value) -> Result<Option<Vec<f32>>, DecodeError> {
    if value.to_u64() == Ok(0) {
        return Ok(None);
    }
    let array = value
        .as_array()
        .map_err(|_| DecodeError::InvalidPayload("dasharray is not a valid value".into()))?;
    let mut dashes = Vec::with_capacity(array.len());
    for item in array.iter() {
        dashes.push(finite_at_least(item, "dasharray entry", 0.0)?);
    }
    Ok(Some(dashes))
}

fn decode_stroke(value: &Value) -> Result<Stroke, DecodeError> {
    let map = value
        .as_map()
        .map_err(|_| DecodeError::InvalidPayload("stroke is not a map".into()))?;

    let paint = match decode_optional_paint(field(map, "paint")?, "stroke paint")? {
        Some(paint) => paint,
        None => {
            return Err(DecodeError::InvalidPayload(
                "stroke paint must be present".into(),
            ));
        }
    };

    Ok(Stroke {
        paint,
        opacity: unit_interval(field(map, "opacity")?, "stroke opacity")?,
        width: finite_positive(field(map, "width")?, "stroke width")?,
        linecap: linecap_from_tag(
            field(map, "linecap")?
                .to_u64()
                .map_err(|_| DecodeError::InvalidPayload("linecap is not unsigned".into()))?,
        )
        .ok_or_else(|| DecodeError::InvalidPayload("unknown linecap tag".into()))?,
        linejoin: linejoin_from_tag(
            field(map, "linejoin")?
                .to_u64()
                .map_err(|_| DecodeError::InvalidPayload("linejoin is not unsigned".into()))?,
        )
        .ok_or_else(|| DecodeError::InvalidPayload("unknown linejoin tag".into()))?,
        miterlimit: finite_at_least(field(map, "miterlimit")?, "stroke miterlimit", 1.0)?,
        dasharray: decode_dasharray(field(map, "dasharray")?)?,
        dashoffset: finite_f32(field(map, "dashoffset")?, "stroke dashoffset")?,
    })
}

fn decode_shape(map: &BTreeMap<Value, Value>) -> Result<Shape, DecodeError> {
    let path_data = field(map, "path_data")?
        .as_bytes()
        .map_err(|_| DecodeError::InvalidPayload("path_data is not bytes".into()))?
        .to_vec();

    Ok(Shape {
        path_data,
        fill: decode_optional_paint(field(map, "fill")?, "fill")?,
        fill_opacity: unit_interval(field(map, "fill_opacity")?, "fill_opacity")?,
        stroke: match field(map, "stroke")? {
            value if value.to_u64() == Ok(0) => None,
            value => Some(decode_stroke(value)?),
        },
    })
}

fn decode_node(value: &Value, depth: usize) -> Result<Node, DecodeError> {
    let map = value
        .as_map()
        .map_err(|_| DecodeError::InvalidPayload("node is not a map".into()))?;

    let discriminant = field(map, "t")?
        .to_u64()
        .map_err(|_| DecodeError::InvalidPayload("node discriminator is not unsigned".into()))?;

    match discriminant {
        0 => Ok(Node::Shape(decode_shape(map)?)),
        1 => {
            if depth >= MAX_GROUP_DEPTH {
                return Err(DecodeError::InvalidPayload(
                    "group nesting exceeds maximum depth".into(),
                ));
            }
            let opacity = unit_interval(field(map, "opacity")?, "group opacity")?;
            let children_value = field(map, "children")?;
            let children_array = children_value
                .as_array()
                .map_err(|_| DecodeError::InvalidPayload("children is not an array".into()))?;
            let mut children = Vec::with_capacity(children_array.len());
            for child in children_array.iter() {
                children.push(decode_node(child, depth + 1)?);
            }
            Ok(Node::Group(Group { opacity, children }))
        }
        other => Err(DecodeError::InvalidPayload(format!(
            "unknown node discriminator: {}",
            other
        ))),
    }
}

fn linecap_svg(cap: LineCap) -> &'static str {
    match cap {
        LineCap::Butt => "butt",
        LineCap::Round => "round",
        LineCap::Square => "square",
    }
}

fn linejoin_svg(join: LineJoin) -> &'static str {
    match join {
        LineJoin::Miter => "miter",
        LineJoin::MiterClip => "miter-clip",
        LineJoin::Round => "round",
        LineJoin::Bevel => "bevel",
    }
}

fn write_stroke_attributes(stroke: &Stroke, out: &mut String) {
    let Paint::Color(color) = stroke.paint;
    out.push_str(&format!(" stroke=\"#{:06x}\"", color));
    out.push_str(&format!(" stroke-opacity=\"{}\"", stroke.opacity));
    out.push_str(&format!(" stroke-width=\"{}\"", stroke.width));
    out.push_str(&format!(
        " stroke-linecap=\"{}\"",
        linecap_svg(stroke.linecap)
    ));
    out.push_str(&format!(
        " stroke-linejoin=\"{}\"",
        linejoin_svg(stroke.linejoin)
    ));
    out.push_str(&format!(" stroke-miterlimit=\"{}\"", stroke.miterlimit));
    if let Some(dashes) = &stroke.dasharray {
        let list = dashes
            .iter()
            .map(|d| d.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        out.push_str(&format!(" stroke-dasharray=\"{}\"", list));
        out.push_str(&format!(" stroke-dashoffset=\"{}\"", stroke.dashoffset));
    }
}

fn write_fill_attributes(fill: &Option<Paint>, fill_opacity: f32, out: &mut String) {
    let fill = match fill {
        Some(Paint::Color(color)) => format!("#{:06x}", color),
        None => "none".to_string(),
    };
    out.push_str(&format!(
        " fill=\"{}\" fill-opacity=\"{}\"",
        fill, fill_opacity
    ));
}

fn write_shape(shape: &Shape, size: (u32, u32), out: &mut String) {
    if shape.path_data.is_empty() {
        // Empty path data is the "fill the whole canvas" shorthand
        // (used by the consumer crate's native tests).
        out.push_str(&format!(
            "<rect x=\"0\" y=\"0\" width=\"{}\" height=\"{}\"",
            size.0, size.1
        ));
        write_fill_attributes(&shape.fill, shape.fill_opacity, out);
        if let Some(stroke) = &shape.stroke {
            write_stroke_attributes(stroke, out);
        }
        out.push_str("/>");
        return;
    }

    let d = String::from_utf8_lossy(&shape.path_data);
    out.push_str(&format!("<path d=\"{}\"", d));
    write_fill_attributes(&shape.fill, shape.fill_opacity, out);
    if let Some(stroke) = &shape.stroke {
        write_stroke_attributes(stroke, out);
    }
    out.push_str("/>");
}

fn write_group(group: &Group, size: (u32, u32), out: &mut String) {
    out.push_str(&format!("<g opacity=\"{}\">", group.opacity));
    for child in &group.children {
        match child {
            Node::Group(nested) => write_group(nested, size, out),
            Node::Shape(shape) => write_shape(shape, size, out),
        }
    }
    out.push_str("</g>");
}

/// Serialize a usvg path's geometry into an SVG `d` string, flattened into
/// canvas coordinates by the path's absolute transform.
fn path_data_to_svg(path: &UsvgPath) -> Vec<u8> {
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

/// Convert a resolved usvg stroke into the DTO form. Returns `None` when the
/// stroke paint is not a solid color (gradient/pattern): such a path is
/// dropped entirely rather than represented with a substituted paint.
fn stroke_from_usvg(stroke: &usvg::Stroke) -> Option<Stroke> {
    let usvg::Paint::Color(color) = stroke.paint() else {
        return None;
    };
    Some(Stroke {
        paint: Paint::Color(color_to_u32(color)),
        opacity: stroke.opacity().get(),
        width: stroke.width().get(),
        linecap: match stroke.linecap() {
            usvg::LineCap::Butt => LineCap::Butt,
            usvg::LineCap::Round => LineCap::Round,
            usvg::LineCap::Square => LineCap::Square,
        },
        linejoin: match stroke.linejoin() {
            usvg::LineJoin::Miter => LineJoin::Miter,
            usvg::LineJoin::MiterClip => LineJoin::MiterClip,
            usvg::LineJoin::Round => LineJoin::Round,
            usvg::LineJoin::Bevel => LineJoin::Bevel,
        },
        miterlimit: stroke.miterlimit().get(),
        dasharray: stroke.dasharray().map(<[f32]>::to_vec),
        dashoffset: stroke.dashoffset(),
    })
}

/// Convert a resolved usvg fill into the DTO form. Returns `None` when the
/// fill paint is not a solid color.
fn fill_from_usvg(fill: &usvg::Fill) -> Option<Paint> {
    match fill.paint() {
        usvg::Paint::Color(color) => Some(Paint::Color(color_to_u32(color))),
        _ => None,
    }
}

fn shape_from_path(path: &UsvgPath) -> Result<Option<Shape>, DecodeError> {
    validate_straight_line_geometry(path)?;
    let fill = path.fill();
    let stroke = path.stroke();

    let dto_fill = match fill {
        Some(fill) => match fill_from_usvg(fill) {
            Some(paint) => Some(paint),
            // Straightlines subset: gradient/pattern paints are rejected, not
            // silently skipped (reject-don't-guess).
            None => {
                return Err(DecodeError::InvalidPayload(
                    "Straightlines subset violation: gradient and pattern paints are not supported"
                        .into(),
                ));
            }
        },
        None => None,
    };
    let dto_stroke = match stroke {
        Some(stroke) => match stroke_from_usvg(stroke) {
            Some(stroke) => Some(stroke),
            None => {
                return Err(DecodeError::InvalidPayload(
                    "Straightlines subset violation: gradient and pattern paints are not supported"
                        .into(),
                ));
            }
        },
        None => None,
    };

    Ok(Some(Shape {
        path_data: path_data_to_svg(path),
        fill: dto_fill,
        fill_opacity: fill.map_or(1.0, |f| f.opacity().get()),
        stroke: dto_stroke,
    }))
}

/// Straightlines subset geometry: every path is exactly one two-point straight
/// segment (`M x0 y0 L x1 y1`). Curves, arcs, multi-segment polylines, and
/// closed shapes are rejected rather than silently mis-encoded.
fn validate_straight_line_geometry(path: &UsvgPath) -> Result<(), DecodeError> {
    let mut count = 0usize;
    let mut ok = true;
    for (index, segment) in path.data().segments().enumerate() {
        count += 1;
        let valid = matches!(
            (index, segment),
            (0, PathSegment::MoveTo(_)) | (1, PathSegment::LineTo(_))
        );
        if !valid {
            ok = false;
            break;
        }
    }
    if !ok || count != 2 {
        return Err(DecodeError::InvalidPayload(
            "Straightlines subset violation: paths must be a single two-point line segment".into(),
        ));
    }
    Ok(())
}

fn collect_group(
    group: &UsvgGroup,
    depth: usize,
    seen_rendering: &mut Option<usvg::ShapeRendering>,
) -> Result<Group, DecodeError> {
    if depth > MAX_GROUP_DEPTH {
        return Err(DecodeError::InvalidPayload(
            "group nesting exceeds maximum depth".into(),
        ));
    }

    // Straightlines subset: no clip-path, mask, or filter anywhere.
    if group.clip_path().is_some() || group.mask().is_some() || !group.filters().is_empty() {
        return Err(DecodeError::InvalidPayload(
            "Straightlines subset violation: clip-path, mask, and filter are not supported".into(),
        ));
    }
    // Straightlines subset: groups are flat — layers live directly under the
    // root and contain only shapes. usvg also models element-level `opacity`
    // on a shape as a wrapper group, which this rejects as nesting.
    if depth >= 2 {
        return Err(DecodeError::InvalidPayload(
            "Straightlines subset violation: nested groups are not supported".into(),
        ));
    }

    let mut children = Vec::new();
    for node in group.children() {
        match node {
            UsvgNode::Path(path) => {
                // Every path participates in the shape-rendering consistency
                // check, even one that is later dropped for an unsupported
                // paint: the document must declare exactly one value.
                let rendering = path.rendering_mode();
                if rendering == usvg::ShapeRendering::OptimizeSpeed {
                    return Err(DecodeError::InvalidPayload(
                        "SVG must declare shape-rendering as geometricPrecision or crispEdges on the root element".into(),
                    ));
                }
                match *seen_rendering {
                    None => *seen_rendering = Some(rendering),
                    Some(previous) if previous != rendering => {
                        return Err(DecodeError::InvalidPayload(
                            "inconsistent shape-rendering across elements is not supported".into(),
                        ));
                    }
                    Some(_) => {}
                }

                match shape_from_path(path)? {
                    Some(shape) => children.push(Node::Shape(shape)),
                    None => {}
                }
            }
            UsvgNode::Group(nested) => {
                children.push(Node::Group(collect_group(
                    nested,
                    depth + 1,
                    seen_rendering,
                )?));
            }
            _ => {}
        }
    }
    Ok(Group {
        opacity: group.opacity().get(),
        children,
    })
}
