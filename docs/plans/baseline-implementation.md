# Baseline implementation plan for `@tksh/svg2ui8a`

## Goal

Create the initial package scaffolding and source for `@tksh/svg2ui8a` so that
it exposes two independent Wasm entry points:

- `jsr:@tksh/svg2ui8a/usvg` → `svg2usvg(svg: string): Promise<Uint8Array>`
- `jsr:@tksh/svg2ui8a/rgba` → `usvg2rgba(usvg: Uint8Array, options?): Promise<RgbaResult>`

This implementation will follow the governing rules in `AGENTS.md`,
`docs/project-constitution.md`, `docs/system-architecture.md`, and
`docs/engineering-playbook.md`.

## Authorization

This plan is authorized by:

- `docs/project-constitution.md` §2.1–§2.3 for the two-product shape and the
  canonical-CBOR envelope.
- `docs/project-constitution.md` §3 for the no-font, no-BBox, no-image, no-native,
  no-Node, and CBOR-only constraints.
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
- `src/usvg.ts`
- `src/rgba.ts`
- `src/mod.ts`

### Deno tests

- `tests/svg2usvg.test.ts`
- `tests/usvg2rgba.test.ts`
- `tests/end-to-end.test.ts`

## Implementation details

### DTO and CBOR envelope

`crates/intermediate` will own the versioned intermediate representation.

- The top-level envelope is a canonical-CBOR map with integer keys:
  - `0`: the literal string `"svg2ui8a/usvg"`
  - `1`: unsigned format version `1`
  - `2`: the DTO payload
- The DTO will capture a deterministic subset of `usvg::Tree` sufficient for
  rendering straight-line SVGs without text, raster images, BBoxes, animation,
  or external resources.
- `encode(&IntermediateV1) -> Vec<u8>` will write only canonical CBOR.
- `decode(&[u8]) -> Result<IntermediateV1, DecodeError>` will reject:
  - non-canonical CBOR
  - unknown identifier
  - unsupported/unknown format versions
  - missing or mistyped fields
  - unsupported DTO variants
- The crate will implement conversions between `IntermediateV1`
  and `usvg::Tree`.

### `svg2usvg`

- `core(svg: &str) -> Result<Vec<u8>, Error>` will parse using feature-disabled
  `usvg`, reject SVGs containing `<text>` or `<image>`, convert the parsed tree
  into `IntermediateV1`, and encode the CBOR bytes.
- `lib.rs` will expose a `#[wasm_bindgen]` wrapper returning a
  `Promise<Uint8Array>`.

### `usvg2rgba`

- `rasterize(bytes: &[u8], options: Options) -> Result<RgbaResult, Error>` will
  decode the package payload, reconstruct a `usvg::Tree`, rasterize it with
  feature-disabled `resvg`, and return raw RGBA pixels plus metadata.
- It will support optional `width` and `height` with natural size fallback and
  independent scaling when both are set.
- It will default to straight alpha and support a premultiplied mode.
- `lib.rs` will expose a `#[wasm_bindgen]` wrapper returning a
  `Promise<RgbaResult>`.

### Build pipeline

- `scripts/build.ts` will compile both crates to Wasm, copy their binaries to
  `assets/`, and regenerate `src/usvg.ts`, `src/rgba.ts`, and `src/mod.ts`.
- `scripts/check-cdn-free.ts` will bundle the wrappers and fail if runtime CDN
  imports are present.
- `scripts/vendor.ts` will scaffold vendoring for new dependencies into
  `vendor/` and record pins in `VENDORED.md`.

## Tests

### Rust native

- `crates/svg2usvg` tests: parse valid SVG, determinism, malformed SVG error,
  unsupported `<text>` and `<image>` rejection, encode/decode round-trip,
  envelope structure.
- `crates/usvg2rgba` tests: natural-size rendering, width-only/height-only
  sizing, exact non-uniform scaling, invalid payload rejection, unsupported
  identifier/version/variant rejection, zero-sized SVG error, straight vs.
  premultiplied alpha, determinism.

### Wasm

- Verify each Wasm entry point returns the correct `Promise` type.
- Verify Wasm output matches the native `core` output.

### Deno

- `svg2usvg` returns `Uint8Array`.
- `usvg2rgba` returns `RgbaResult` with pixel buffer of length
  `width * height * 4`.
- `alphaMode` is exposed correctly.
- End-to-end flow produces correctly sized RGBA bytes.
- `svg2usvg` output can be written/read as `.cbor` and passed to `usvg2rgba`.
- `init` is idempotent and independent per entry point.
- Malformed SVG and malformed `usvg` payload reject promises.

## Dependency plan

- Use `cbor-core = "0.10.1"`, `usvg = { version = "0.47.0", default-features = false }`,
  and `resvg = { version = "0.47.0", default-features = false }` in the
  workspace root dependencies.
- No other runtime dependencies are required for the baseline.

## Notes and open questions

- The exact DTO field shape will be chosen during implementation, with a
  preference for minimal, deterministic geometry and paint fields.
- The first implementation will avoid any direct `tiny-skia` dependency in the
  producer crate, keeping it transitive through `resvg` only.

---

This plan is ready for human review. No source files will be added until it is
approved.
