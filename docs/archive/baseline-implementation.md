# Baseline implementation plan for `@tksh/svg2ui8a`

## Goal

Create the initial package scaffolding and source for `@tksh/svg2ui8a` so that
it exposes two independent Wasm entry points:

- `jsr:@tksh/svg2ui8a/usvg` → `svg2usvg(svg: string): Promise<Uint8Array>`
- `jsr:@tksh/svg2ui8a/rgba` →
  `usvg2rgba(usvg: Uint8Array, options?): Promise<RgbaResult>`

This implementation will follow the governing rules in `AGENTS.md`,
`docs/project-constitution.md`, `docs/system-architecture.md`, and
`docs/engineering-playbook.md`.

## Authorization

This plan is authorized by:

- `docs/project-constitution.md` §2.1–§2.3 for the two-product shape and the
  canonical-CBOR envelope.
- `docs/project-constitution.md` §3 for the no-font, no-BBox, no-image,
  no-native, no-Node, and CBOR-only constraints.
- `docs/system-architecture.md` §1–§4 for the workspace layout, shared crate,
  and the two independent Wasm artifacts.
- `docs/engineering-playbook.md` §4 for drafting a plan under `docs/plans/`
  before implementation.

## Files to create or modify

### Repository scaffolding

- `Cargo.toml`
- `.gitignore`
- `jsr.json`
- `deno.json`
- `CHANGELOG.md`

### Shared crate

- `crates/intermediate/Cargo.toml`
- `crates/intermediate/src/lib.rs`

### Producer crate

- `crates/svg2usvg/Cargo.toml`
- `crates/svg2usvg/src/lib.rs`
- `crates/svg2usvg/src/core.rs`
- `crates/svg2usvg/tests/core.rs`

### Consumer crate

- `crates/usvg2rgba/Cargo.toml`
- `crates/usvg2rgba/src/lib.rs`
- `crates/usvg2rgba/src/core.rs`
- `crates/usvg2rgba/tests/core.rs`

### Wasm build and JS wrapper

- `scripts/build.ts`
- `scripts/vendor.ts`
- `scripts/check-cdn-free.ts`
- `scripts/test-wasm.ts`
- `src/usvg.ts`
- `src/rgba.ts`
- `src/mod.ts`

### Deno tests

- `tests/svg2usvg.test.ts`
- `tests/usvg2rgba.test.ts`
- `tests/end-to-end.test.ts`

## Implementation details

### The central reconstruction constraint

`usvg::Tree` (usvg 0.47) has **no public constructor**. All of its node structs
(`Group`, `Path`, `Fill`, `Stroke`) use `pub(crate)` fields and lack public
constructors. The only public way to obtain a `Tree` is `Tree::from_str` /
`from_data` / `from_xmltree`, i.e. by parsing an SVG/XML **string**. Because
`resvg::render` requires a `&usvg::Tree`, `IntermediateV1 → Tree` conversion
**must** serialize the DTO's drawing fields back to a minimal, canonical SVG
string and re-parse it. This is an implementer judgment recorded here per
`engineering-playbook.md` §0; it does not change the API surface (the package
never **outputs** an SVG string to callers) and it keeps `resvg`/`tiny-skia` as
the only rasterizer, per `playbook` §3.6.

### DTO schema (version 1)

The DTO mirrors the **normalized** tree that usvg produces, restricted to the
supported subset (solid-color fills/strokes, paths, group transforms and
opacity). It deliberately excludes text, raster images, gradients, patterns,
clip paths, masks, filters, BBoxes, animation state, and external resources.
`svg2usvg` rejects inputs whose normalized tree contains any excluded feature
rather than emitting an unrenderable payload (`constitution` §2.3).

Top-level map keys (fixed, `constitution` §2.3):

- `0`: text `"svg2ui8a/usvg"`
- `1`: unsigned `1`
- `2`: payload map

Payload map fields:

- `size`: `[width: f64, height: f64]` — `tree.size()`.
- `children`: array of node maps (order preserved; z-order).

Node is a CBOR map discriminated by a `t` (tag) field to keep the DTO
deterministic and canonical:

- `{"t": 0, "transform": [6 × f64], "opacity": f64, "children": [...]}` — a
  group (`transform` is the normalized relative transform stored on usvg groups;
  `opacity` is the group opacity).
- `{"t": 1, "segments": [...], "fill": ..., "stroke": ..., "fill_rule": 0|1,
  "stroke_first": bool, "rendering_mode": 0|1|2}`
  — a path.

Path fields:

- `segments`: sequential command array. Tags: `0` MoveTo, `1` LineTo, `2`
  QuadTo, `3` CubicTo, `4` Close; the command tag is followed by its coordinate
  floats (`f64`). Sources `tiny_skia_path::Path::segments()`.
- `fill`: `null` (no fill) or `[r, g, b, opacity_f64]`.
- `stroke`: `null` or
  `{"color": [r,g,b], "width": f64, "opacity": f64,
  "linecap": 0|1|2, "linejoin": 0|1|2|3, "dash": [...]|∅,
  "dashoffset": f64, "miterlimit": f64}`,
  mapping one-to-one to usvg `Stroke`.
- `fill_rule`: 0 (NonZero) | 1 (EvenOdd).
- `stroke_first`: paint order (default false = fill then stroke).
- `rendering_mode`: `ShapeRendering` (0 GeometricPrecision, 1 OptimizeSpeed, 2
  CrispEdges).

**Rationale per field** (1–3 sentences):

- `size` is required so `usvg2rgba` can size the pixmap for the natural-size
  case and derive the missing dimension when only one of `width`/`height` is
  given (`constitution` §4.3).
- `children` + group `transform`/`opacity` preserve usvg's normalized group
  structure. Group opacity must be kept at group granularity; folding it into
  per-path opacity would change the composite of overlapping siblings.
  Transforms are kept as usvg stores them (relative) so reconstruction re-parses
  to the same normalized tree.
- Path `segments` carry the geometry in the coordinate space usvg stores (local
  to the node's frame). Re-emitting them verbatim and re-parsing is
  byte-faithful (see reconstruction below).
- `fill`/`stroke` as solid RGBA colors capture usvg's resolved solid paints. The
  opacity is folded in separately (not multiplied) because usvg keeps
  `Paint::Color` + `Opacity` separate and groups apply opacity at the group
  layer.
- `fill_rule`, `stroke_first`, `rendering_mode`, and the stroke sub-attributes
  are copied from the corresponding usvg fields so reconstruction is faithful
  for the supported subset.

### Envelope and codec

- `encode(&IntermediateV1) -> Vec<u8>` writes canonical CBOR via
  `cbor_core::SequenceWriter` with `EncodeFormat::Binary` under `Value::map`.
  `cbor_core` sorts map keys canonically, guaranteeing deterministic bytes
  (`constitution` §2.3, §5.3).
- `decode(&[u8]) -> Result<IntermediateV1, DecodeError>` uses `SequenceDecoder`
  (default `Strictness::STRICT`, which rejects non-canonical input), then
  validates: identifier literal, version, payload is a map, `size` present with
  positive finite width/height, `children` present and an array, every node tag
  supported, numeric ranges valid (opacity in 0..=1, colors ≤ 255, no
  unsupported DTO variants). Any failure returns a typed `DecodeError`; the
  decoder never panics.
- `from_tree(&usvg::Tree) -> Result<IntermediateV1, DecodeError>` walks the
  normalized tree, copying only supported nodes and returning an error on any
  paint that is not `Paint::Color`, any group with clip/mask/filter/blend, any
  `Node::Image`/`Node::Text`, or any non-default paint order edge that cannot be
  reproduced.
- `to_tree(&self) -> Result<usvg::Tree, DecodeError>` serializes the DTO into a
  minimal canonical SVG string and calls `usvg::Tree::from_str`. Float values
  are emitted with Rust `{}` formatting, which yields the shortest decimal
  string that round-trips to the same `f32`; usvg parses back to identical
  `f32`s, so rendering is deterministic and correct.

### `svg2usvg`

- `core(svg: &str) -> Result<Vec<u8>, Error>`:
  1. Detect and reject `<text>` / `<image>` content **before** parsing, using a
     `roxmltree` scan for `text`, `tspan`, `textPath`, and `image` tags (usvg
     with the `text`/`raster-images` features disabled would silently drop them;
     the package must reject them, `constitution` §3.1, §3.3).
  2. Parse with feature-disabled `usvg::Tree::from_str`.
  3. Convert via `IntermediateV1::from_tree` (rejecting any excluded feature).
  4. Encode via `IntermediateV1::encode`.
- `lib.rs` exposes a **synchronous** `#[wasm_bindgen]`
  `pub fn svg2usvg(svg: &str) -> Result<Box<[u8]>, JsValue>`. The TS wrapper
  wraps the call in a `Promise` per `constitution` §4.4 ("always async at the JS
  surface"). A sync export avoids an extra `wasm-bindgen-futures` dependency.
- Dependency list: `intermediate`, `usvg` (workspace), `roxmltree` (pinned
  `"0.21"`, matching usvg's own transitive version — no new crate enters the
  tree), `wasm-bindgen`. **No** `resvg`, `tiny-skia`, `png`.
- `crates/svg2usvg/tests/core.rs`: simple SVG → non-empty bytes; determinism;
  malformed → error; `<text>` → error; `<image>` → error; `from_tree`/encode
  round-trip via `IntermediateV1::decode`; envelope key layout.

### `usvg2rgba`

- `core.rs`
  `rasterize(bytes: &[u8], options: RgbaOptions) -> Result<RgbaResult, Error>`:
  1. `IntermediateV1::decode` (semantic validation inside).
  2. `IntermediateV1::to_tree`.
  3. Determine target size: both omitted → natural; one set → the other scaled
     by the natural aspect ratio; both set → exact `width × height` (independent
     scaling, `constitution` §4.3). Zero natural size → error.
  4. `tiny_skia::Pixmap::new` + `pixmap.fill(TRANSPARENT)` (zero-init,
     `constitution` §5.2).
  5. `resvg::render(&tree, Transform::from_scale(fx, fy), &mut pixmap)`.
  6. If option requests `premultiplied` (as-is), keep `pixmap.pixels()`; else
     un-premultiply each pixel (`constitution` §4.2, §5.2): for alpha a>0,
     `c = c*255/a` rounded; a=0 → 0.
  7. Return `RgbaResult { pixels, width, height, alpha_mode }`.
- `lib.rs` exposes `#[wasm_bindgen]` struct `RgbaResult` (fields `width`,
  `height`, `alpha_mode`, `pixels`) with getters, and
  `pub fn usvg2rgba(bytes: &[u8], options: JsValue) -> Result<RgbaResult, JsValue>`.
  Options JS object `{ width?, height?, alphaMode? }` is read via
  `js_sys::Reflect` (values optional). Sync export; TS wrapper returns a
  `Promise`.
- Dependencies: `intermediate`, `resvg` (workspace), `wasm-bindgen`, `js_sys`.
  No PNG/WebP/JPEG encoder or decoder (`constitution` §3.3).
- `crates/usvg2rgba/tests/core.rs`: natural size; width-only; height-only; both
  non-uniform; non-canonical/non-CBOR/non-package payload → error; unknown
  identifier/version/malformed DTO/unsupported variant → error; zero-size →
  error; straight alpha `(255,0,0,128)`; premultiplied `(128,0,0,128)`; render
  determinism.

### Build pipeline

- `scripts/build.ts` (initial scaffolding per `playbook` §2.1):
  - `wasm-pack build crates/svg2usvg --target web` → copy `pkg/svg2usvg_bg.wasm`
    → `assets/svg2usvg_bg.wasm`.
  - `wasm-pack build crates/usvg2rgba --target web` → copy
    `pkg/usvg2rgba_bg.wasm` → `assets/usvg2rgba_bg.wasm`.
  - Regenerate `src/usvg.ts` / `src/rgba.ts` from templates that embed the Wasm
    bytes (base64) and call the wasm-bindgen `init`/`__wbg_init` with those
    bytes — **no runtime fetch**, so `constitution` §3.8 / `AGENTS.md` §5.2 are
    satisfied in Deno and browsers alike.
  - Generate `src/mod.ts` re-exporting `svg2usvg`, `usvg2rgba`, and the
    `RgbaResult`/`Usvg2RgbaOptions` types.
  - Run the CDN-free check; abort on failure.
- `scripts/vendor.ts`: reproducible vendoring scaffold for future dependencies;
  not exercised by the baseline dependency set.
- `scripts/check-cdn-free.ts`: bundle the three subpath entry points and grep
  for `https://`; hard fail on any match (`playbook` §5.2).
- `scripts/test-wasm.ts`: loads both Wasm binaries, verifies each export returns
  the expected shape and matches the Rust-native `core` output.

### TypeScript surface

- `src/usvg.ts` / `src/rgba.ts` (generated): single per-module `initialized`
  flag; `svg2usvg(svg: string): Promise<Uint8Array>` and
  `usvg2rgba(bytes: Uint8Array, options?: Usvg2RgbaOptions): Promise<RgbaResult>`.
  No caching, memoization, pre-warming, `dispose()`, or class API
  (`constitution` §4.4, `playbook` §7.4/§7.7).
- `src/mod.ts` (hand-written): re-exports both functions and the two public
  types (`RgbaResult`, `Usvg2RgbaOptions`).

### Tests (all four layers)

- Rust native: `crates/svg2usvg/tests/core.rs`, `crates/usvg2rgba/tests/core.rs`
  (see sections above) plus `crates/intermediate/tests/core.rs` covering
  encode/decode round-trip, envelope shape, and one error per invalid-input
  category.
- Wasm: `scripts/test-wasm.ts` (Promise shape + parity with native `core`).
- Deno: `tests/svg2usvg.test.ts`, `tests/usvg2rgba.test.ts`,
  `tests/end-to-end.test.ts` covering `Uint8Array` return,
  `pixels.length ===
  width*height*4`, `alphaMode` reflection, `.cbor` fixture
  round trip (`cbor-file-intermediate.md`), `init` idempotency, JS-side
  determinism, and rejection of malformed SVG / malformed payload.
- Layer order enforced by `deno task test` = `test:rust` → `test:wasm` →
  `deno test -A` (`playbook` §3).

### Dependency plan and feature verification

- Cargo: `cbor-core = "0.10.1"`,
  `usvg = { version = "0.47.0",
  default-features = false }`,
  `resvg = { version = "0.47.0",
  default-features = false }`
  (workspace-pinned). With `default-features =
  false`, usvg/resvg compile
  without `text`, `system-fonts`, `memmap-fonts`, or `raster-images`;
  `tiny-skia` is reached only through `resvg`. Verified at
  `cargo tree -p ... -e features`.
- `roxmltree = "0.21"` (no new crate: already in usvg's transitive graph).
- No `postcard`, MessagePack, bincode, JSON, PNG, WebP, font, or raster-image
  dependency in either artifact's tree.
- No `serde`/`serde_json`; `RgbaResult` uses wasm-bindgen getters instead of
  `JsValue::from_serde`.

### Wasm size expectation

- `svg2usvg_bg.wasm`: small (usvg parser only, ~600–900 KB before optimization;
  `opt-level="z"` + LTO in release for the wasm target).
- `usvg2rgba_bg.wasm`: larger (adds resvg + tiny-skia, ~1–2 MB before
  optimization). Both recorded after the first build as a follow-up note.

## Notes and open questions

- **Implementer judgment (recorded):** DTO mirrors the normalized tree instead
  of a fully flattened path list, and reconstruction re-parses a generated SVG
  string, because usvg 0.47 provides no public programmatic `Tree`/node
  constructors (verified against the 0.47.0 source). This is the only way to
  keep `resvg` as the sole rasterizer while satisfying the architecture's
  "reconstruct a `usvg::Tree`" step. No SVG string ever crosses the JS boundary.
- **Implementer judgment (recorded):** wasm exports are synchronous inside Wasm;
  the TS wrappers return `Promise` per the constitution. `RgbaResult` via
  wasm-bindgen getter struct, options via `js_sys::Reflect`.
- If a future task wants group opacity/transform folding, gradient, pattern,
  clip-path, mask, or filter support, that is a separate plan and a new DTO
  version.

---

This plan supersedes the earlier draft in this file. Source implementation
begins only after human approval.
