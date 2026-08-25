# Versioned CBOR intermediate and Straightlines vision alignment

## Goal

Make `svg2ui8a`'s CBOR intermediate a versioned, self-describing package file
format that can be read from a `.cbor` file into `Uint8Array` and rendered by
the existing `usvg2rgba` API. Rewrite the private Straightlines vision note to
describe this relationship accurately and to incorporate `cbor_core`.

## Format and API decisions

- Keep the existing TypeScript API unchanged. Consumers read a `.cbor` file
  themselves and pass its bytes to `usvg2rgba`; the package does not add
  filesystem or path-based APIs.
- Define the intermediate as a canonical-CBOR top-level map with integer keys:
  `0` is the literal format identifier `"svg2ui8a/usvg"`, `1` is the unsigned
  format version `1`, and `2` is the version-1 DTO payload.
- Treat this envelope and its DTO schema as a public file-format contract:
  patches and minors preserve support for format version 1; an incompatible
  format requires a new format version and a major package release.
- Use `cbor-core` 0.10.1 (`cbor_core`) to encode and decode the envelope. Its
  canonical-CBOR validation is necessary but not sufficient: the decoder must
  validate identifier, version, required fields, field types, ranges, and the
  absence of unsupported DTO variants before constructing a `usvg::Tree`.
- The payload is a package-owned DTO, never a direct serialization of
  `usvg::Tree`. The DTO excludes text, raster images, BBoxes, animation, and
  external resources. `svg2usvg` rejects SVG containing unsupported text or
  image content instead of emitting a payload that cannot render faithfully.

## Architecture and dependency changes

- Add a non-Wasm `crates/intermediate/` workspace crate shared by the two Wasm
  crates. It owns `IntermediateV1`, the `cbor_core::Value` mapping, canonical
  encode/decode, schema validation, and conversion to/from the supported
  `usvg::Tree` subset. The workspace still ships exactly two Wasm artifacts.
- Pin `usvg` 0.47.0 and `resvg` 0.47.0 with `default-features = false`. This
  disables upstream text/system-font and raster-image features; retain
  `tiny-skia` only as resvg's transitive rasterizer dependency. Do not add image
  decoders, fonts, or direct `tiny-skia` APIs.
- Update the dependency graph and Cargo examples to show the shared intermediate
  crate, the feature-disabled dependencies, and the fact that `resvg` re-exports
  its matching `usvg` and `tiny-skia` versions.

## Documentation changes

- Replace `notes/straightlines-vision.md` in Japanese. Position Straightlines as
  a future, separately specified CBOR vector format; describe `svg2ui8a` as a
  practical, versioned usvg-derived CBOR pipeline rather than a postcard
  sibling. Remove postcard/ciborium/serde_cbor/dCBOR adoption claims, state the
  limits of usvg normalization, and record `cbor_core` canonical CBOR, the
  feature-disabled rendering stack, and the no-text/no-raster-image boundary.
- Update the constitution to define the file envelope, schema compatibility,
  canonical and semantic validation, unsupported-input rejection, and the
  unchanged `Uint8Array` APIs.
- Update the architecture to make `crates/intermediate` the single schema
  source, require DTO conversion rather than Tree serialization, and document
  the feature flags and data flow from file bytes through `usvg2rgba`.
- Update the engineering playbook with dependency-feature verification and test
  requirements for a `.cbor` file round trip, envelope/schema rejection,
  non-canonical CBOR rejection, and unsupported text/raster-image rejection.

## Verification

- Documentation review: no stale postcard, ciborium, serde_cbor, dCBOR, or
  direct-Tree-serialization claims; all dependency pins and feature flags agree.
- Rust tests: canonical encode/decode, semantic validation, format-version
  compatibility, DTO/Tree conversion, and deterministic RGBA output.
- Deno integration: write/read a `svg2usvg` result as a `.cbor` fixture and pass
  the read bytes to `usvg2rgba`, asserting RGBA dimensions and pixels.
- Build inspection: verify the resolved `usvg` and `resvg` feature sets exclude
  text, system-fonts, memmap-fonts, and raster-images; retain the existing
  CDN-free and two-Wasm checks.

## Assumptions

- The document-only repository will gain the described shared crate during the
  first implementation task; no source implementation is added by this plan.
- The format-v1 DTO's individual drawing fields remain an implementation task,
  but its envelope and validation boundary are fixed here.
