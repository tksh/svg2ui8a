# Plan: BBox Support for `@tksh/svg2ui8a` (revision 4)

**Status:** APPROVED for `0.4.0` — ready for implementation (`AGENTS.md` §2;
`docs/engineering-playbook.md` §4 step 3).

**Task source:** the human asked for a plan implementing the BBox capability
postponed in `docs/project-constitution.md` §3.2 (also listed as deferred in
§4.5), then asked for a **redesign**: stop being influenced by `resvg-js`
(`getBBox()` / `innerBBox()` / `cropByBBox()` are resvg-js inventions, not
upstream `linebender/resvg` API), and instead expose faithful measurements from
upstream
`usvg::Group::{abs_bounding_box, abs_stroke_bounding_box,
abs_layer_bounding_box}`,
reusing upstream names and avoiding new names where possible.

**Location note:** the task said `doc/plans/`; the repository uses
`docs/plans/`. The human confirmed saving as `docs/plans/bbox-support.md`.
Revision 4 fixes the version target to `0.4.0` per human approval and resolves
the last open question. It supersedes the mode/cropping design (revision 1) and
the opt-in design (revision 2). Per the human's decision, no bounding-box option
is added: all three result fields are always present with type `RectF | null`.

---

## 0. Summary

Expose the **upstream `usvg` bounding-box fields of the parsed document root**
as always-present, nullable metadata on the existing `svg2rgba` subpath. The
package computes nothing new and invents no box semantics: it returns, verbatim,
the three rects `usvg 0.47.0` precomputes on `tree.root()`:

| Field in result        | Upstream source (usvg 0.47.0, `usvg::Group`)      | Meaning per upstream docs                                                                                                                                       |
| ---------------------- | ------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `absBoundingBox`       | `Group::abs_bounding_box()` → `Rect`              | bounding box in canvas coordinates (`userSpaceOnUse` in SVG terms)                                                                                              |
| `absStrokeBoundingBox` | `Group::abs_stroke_bounding_box()` → `Rect`       | bounding box including stroke, in user coordinates                                                                                                              |
| `absLayerBoundingBox`  | `Group::abs_layer_bounding_box()` → `NonZeroRect` | "layer" box in canvas units: stroke box expanded/clipped by filter regions; what resvg uses to allocate layers. Cannot be zero-size; `0×0×1×1` for empty groups |

What is **dropped** relative to revision 1: the invented mode selector and the
crop behavior. Both were resvg-js-shaped ideas, not upstream ones. The
measurement is attached to the render call because that is the only entry point
the package has; no third subpath, no third Wasm artifact, no new function.

`svg2usvg` is untouched.

---

## 1. Constitution authorization

Quoted per playbook §4 step 2:

> ### 3.2 BBox — postponed
>
> The package does not expose bounding boxes. There is no `getBBox()`, no
> `innerBBox()`, no `cropByBBox()`.
>
> BBox support is **postponed**, not permanently forbidden. If needed, it will
> be added via `resvg`/`usvg` facilities under an approved plan.

And §4.5:

> - Fonts and BBox (see §3.1/§3.2).
>
> The agent must not implement any of these as part of an `svg2ui8a` task. If
> the human asks for them, the human will create a separate task and a separate
> plan.

This plan is that separate plan. Conditions satisfied:

- Added **via `usvg` facilities** and nothing else: the three rects are read
  straight off `usvg::Tree::root()` (a `Group`). No new math, no resvg-js
  behavior, no invented modes, no crop.
- Human approval before implementation.
- No proprietary format (§3.7): boxes travel as plain numbers on the existing
  result object, which §4.2 explicitly leaves to the implementer.
- §3.9: no new Wasm artifact; measurements accompany the existing `svg2rgba`
  result. Pixels and existing metadata remain unchanged; every result gains
  three keys, so object shape and serialized results are intentionally not
  identical. Determinism continues to hold.
- §3.2's named exclusions (`getBBox()` / `innerBBox()` / `cropByBBox()`) are not
  added: no new function of any kind.

---

## 2. API surface (reference shape)

Per constitution §4 (shape, not detail), the additions to
`jsr:@tksh/svg2ui8a/svg2rgba`:

```ts
export interface Svg2RgbaOptions {
  width?: number;
  height?: number;
  alphaMode?: "straight" | "premultiplied";
}

export interface RgbaResult {
  width: number;
  height: number;
  naturalWidth: number;
  naturalHeight: number;
  alphaMode: string;
  pixels: Uint8Array;

  absBoundingBox: RectF | null;
  absStrokeBoundingBox: RectF | null;
  absLayerBoundingBox: RectF | null;
}

/** Same four numbers as `usvg::Rect` / `usvg::NonZeroRect`. */
export interface RectF {
  x: number;
  y: number;
  width: number;
  height: number;
}
```

### 2.1 Naming rationale (upstream-first)

- Field names are the upstream method names, camelCased per the existing wrapper
  convention (`naturalWidth` ← `natural_width`): `absBoundingBox` ←
  `abs_bounding_box()`, etc. No new box vocabulary is introduced; the docs for
  each field can quote upstream verbatim.
- No new option or mode selector is introduced. The three stored boxes are read
  on every successful render. `Svg2RgbaOptions` stays unchanged; the negligible
  getter cost does not justify an opt-in API. Copying metadata across Wasm and
  allocating JS objects still has a small cost.
- Rect shape `{x, y, width, height}` mirrors `usvg::Rect`'s accessors (`x()`,
  `y()`, `width()`, `height()`).

### 2.2 Semantics

- Every successful call returns all three bounding-box keys as own properties,
  each containing a `RectF` or explicit `null`, never omitted or `undefined`.
  Pixels, sizing, alpha handling, and existing metadata remain unchanged.
- `null` denotes an unavailable upstream measurement, not an empty document or
  zero extent. In pinned usvg 0.47.0 these three getters return concrete rects,
  so successful calls populate all three. Preserve zero-area rects and the layer
  placeholder verbatim; do not introduce a rule turning them into `null`.
  Parse/render failures still reject the promise rather than returning null
  boxes.
- **Semantics are upstream's, whatever they are for a given document.** Notable
  upstream behaviors the implementation will surface as-is (verified against the
  pinned `usvg 0.47.0` in §6):
  - boxes are in canvas (absolute/user-space) coordinates and already include
    child transforms;
  - an empty document yields `absBoundingBox = {0,0,0,0}` and
    `absStrokeBoundingBox = {0,0,0,0}`;
  - `absLayerBoundingBox` is never zero-size — upstream returns `0×0×1×1` for
    empty groups — so consumers must treat it as "placeholder", not "empty".
- The reported values are **pre-render measurements of the input tree**. They
  are not affected by `width`/`height`/`alphaMode` options, and they say nothing
  about the output pixmap (that remains `width`/`height`/`naturalWidth`/
  `naturalHeight` exactly as today).

### 2.3 Explicit non-goals

- No crop, no viewBox rewriting, no padding/square options (resvg-js's
  `cropByBBox` ideas — out of scope by human decision).
- No per-element / per-`id` lookup, no DOM-`getBBox` analog.
- No new box math: if a consumer needs e.g. a fill-only box with custom
  stroke-opacity rules (resvg-js's `innerBBox`), they compute it from these
  numbers or from `svg2usvg` output themselves.
- No new subpath, no third crate, no third Wasm artifact.

---

## 3. Implementation by layer

### 3.1 `crates/svg2rgba/src/core.rs` (Rust core)

- `RgbaOptions` is unchanged.
- Native `RgbaResult` gains `abs_bounding_box: Option<UpstreamRect>`,
  `abs_stroke_bounding_box: Option<UpstreamRect>`,
  `abs_layer_bounding_box: Option<UpstreamRect>`, where `UpstreamRect` is a tiny
  `{x: f32, y: f32, width: f32, height: f32}` conversion struct (the only new
  type, mirroring `usvg::Rect`'s accessors 1:1).
- In `rasterize_svg`, unconditionally populate the three fields from
  `tree.root().abs_bounding_box()` / `.abs_stroke_bounding_box()` /
  `.abs_layer_bounding_box()` using `x()/y()/width()/height()` accessors. With
  usvg 0.47.0 each field is `Some`, including empty-document values. The render
  path itself is untouched.

### 3.2 `crates/svg2rgba/src/lib.rs` (wasm-bindgen boundary)

- `Svg2RgbaOptions` (wasm) is unchanged. The exported `RgbaResult` gains three
  nullable float-quads. wasm-bindgen maps `Option<f32>` fields to
  `number | undefined` getters; the TS wrapper assembles complete `RectF`
  objects or explicit `null`. It must preserve zero coordinates and extents,
  using presence checks rather than truthiness.

### 3.3 `scripts/build.ts` (TS glue — regenerated, never hand-edited)

- Extend only the `RgbaResult` template to §2's shape and add the `RectF`
  interface export. Leave the options template and option mapping unchanged.
- Always include all three keys in the returned plain object, using `RectF`
  values or explicit `null`. JSON serialization must preserve every key,
  including null-valued keys. Retain the existing Wasm result cleanup.

### 3.4 Docs/README

- `README.md` gains a short example. Governing documents are not edited.

---

## 4. Tests (all four layers, per playbook §3)

### 4.1 Rust native — `crates/svg2rgba/tests/core.rs`

- `bounding_boxes_are_always_populated` — default options produce all three
  upstream measurements; existing metadata and full pixel buffers match the
  pre-change baseline byte-for-byte (not just a checksum).
- `abs_bounding_box_matches_usvg_root` — for a transformed group containing a
  stroked rect, assert the exact upstream numbers (expected values taken from
  the probe in §6: `abs` boxes include child transforms).
- `abs_stroke_bounding_box_includes_stroke` — same doc, box is larger than the
  fill box by the stroke extent.
- `abs_layer_bounding_box_equals_stroke_box_without_filters` and expands when a
  filter region is present.
- `empty_document_reports_zero_boxes_and_layer_placeholder` — `{0,0,0,0}` for
  the two `Rect` boxes, `{0,0,1,1}` for the layer box (upstream's placeholder).
- `boxes_are_deterministic_across_calls`.
- Malformed input and `<text>`/`<image>` rejection are unchanged; errors do not
  resolve to a result with null boxes.

### 4.2 Wasm layer — `scripts/test-wasm.ts` / `scripts/test-wasm-browser.ts`

- Without any new option, the generated entry point returns a plain object with
  all three keys. Wasm boxes equal Rust-native core output for the same input.
  Extend `dump_core` metadata output and both harness parsers to carry the three
  measurements; keep its existing arguments and raw pixel output.
- Test the boundary conversion of absent internal measurements to explicit
  `null` with synthetic test inputs, without manufacturing nulls from valid
  upstream zero-area rects or adding a public test option.

### 4.3 Deno tests — `tests/svg2rgba.test.ts`

- Omitted options, `{}`, and existing sizing/alpha options all return own
  `absBoundingBox`, `absStrokeBoundingBox`, and `absLayerBoundingBox` keys.
  Assert each is a plain `RectF` object or explicit `null`, never `undefined`.
- All three measurements match Rust-native values and remain independent of
  output sizing and alpha mode. Repeated calls are deterministic.
- structuredClone and JSON round-trips preserve all three keys and values,
  including explicit nulls in synthetic boundary-conversion tests.
- Existing `{}`-behaves-like-omitted regression guard still passes; update
  exact-key expectations to include the three required result fields.

### 4.4 Deno tests — `tests/conformance.test.ts`

- Rendering normalized `svg2usvg` bytes still yields identical pixels for the
  existing conformance fixtures. Both render results contain all three box keys;
  module initialization remains idempotent.

---

## 5. Size impact on the Wasm binary (playbook §4 step 2)

- The boxes are **already computed and stored by usvg during parse** (fields on
  `Group`, not computed lazily), and the `svg2rgba` artifact already links
  everything needed. Expected delta on `assets/svg2rgba_bg.wasm`: **hundreds of
  bytes to a few KB**, unmeasured until implementation (result fields and
  getters). No option field or conditional request path is added. Reading the
  stored rects is cheap; metadata transfer and JS allocation occur on every
  call. No new crate or renderer code path is introduced.
- `assets/svg2usvg_bg.wasm`: byte-identical, untouched.

---

## 6. Verification already performed for this plan (no repo code changed)

A scratch probe outside the repo (`/tmp/opencode/bbox-repro`, deleted after use)
pinned `usvg = "=0.47.0", default-features = false` — the same pin as the root
`Cargo.toml` — and asserted against the workspace-pinned upstream source
(`~/.cargo/registry/src/.../usvg-0.47.0/src/tree/mod.rs:1146-1184`):

- `Tree::root()` is a `Group`; `abs_bounding_box()` /
  `abs_stroke_bounding_box()` return `Rect`, `abs_layer_bounding_box()` returns
  `NonZeroRect`, all with `x()/y()/width()/height()` accessors.
- For
  `<g transform="translate(10 20) scale(2 3)"><rect x="1" y="2"
  width="4" height="5" stroke-width="2"/></g>`:
  `abs_bounding_box` = `(12, 26, 8, 15)` (fill geometry, transformed),
  `abs_stroke_bounding_box` = `(10, 23, 12, 21)` (stroke expanded, transformed),
  `abs_layer_bounding_box` = same as stroke box (no filters).
- Empty document: `abs_*` boxes are `(0, 0, 0, 0)`; layer box is the upstream
  placeholder `(0, 0, 1, 1)`.
- Baseline `deno task test:rust` (14 tests) passed before any edits.

---

## 7. Dependency changes (playbook §4 step 2 / AGENTS.md §5.3)

**None.** `usvg 0.47.0` is already workspace-pinned with
`default-features = false`; the box fields ship in that build (confirmed in the
probe). No version change, no feature change, no vendoring, no new JSR import.

---

## 8. Files to touch

| File                                                   | Change                                                   |
| ------------------------------------------------------ | -------------------------------------------------------- |
| `crates/svg2rgba/src/core.rs`                          | three result measurements always read from `tree.root()` |
| `crates/svg2rgba/src/lib.rs`                           | wasm-bindgen field pass-through                          |
| `crates/svg2rgba/tests/core.rs`                        | native tests (§4.1)                                      |
| `crates/svg2rgba/examples/dump_core.rs`                | measurement output for wasm-layer parity (§4.2)          |
| `scripts/build.ts`                                     | `RgbaResult` template + `RectF` export (§3.3)            |
| `src/svg2rgba.ts`                                      | regenerated by `deno task build` (never hand-edited)     |
| `tests/svg2rgba.test.ts`                               | Deno tests (§4.3)                                        |
| `tests/conformance.test.ts`                            | cross-cutting tests (§4.4)                               |
| `scripts/test-wasm.ts`, `scripts/test-wasm-browser.ts` | wasm-layer cases (§4.2)                                  |
| `assets/svg2rgba_bg.wasm`                              | regenerated build artifact (committed per AGENTS §7)     |
| `README.md`                                            | short usage example                                      |
| `CHANGELOG.md`                                         | `0.4.0` entry under "Unreleased" (playbook §6)           |

Untouched: `crates/svg2usvg/**`, `assets/svg2usvg_bg.wasm`, `src/svg2usvg.ts`,
`src/mod.ts`, the four governing documents.

---

## 9. Implementation order & verification

1. Rust core + native tests → `deno task test:rust`.
2. wasm-bindgen + glue regeneration → `deno task build` (includes the CDN-free
   check, playbook §5.2).
3. All four test layers → `deno task test`.
4. `deno task fmt` / `deno task lint` / `deno task check`.
5. Record Wasm sizes before/after (`svg2rgba_bg.wasm` is 993,566 bytes today);
   update CHANGELOG (`0.4.0` entry); report per AGENTS §9.

Commits (one logical change each, per AGENTS §7):

- `svg2ui8a: expose usvg root bounding boxes via svg2rgba (constitution §3.2)`
- `svg2ui8a: regenerate wasm + glue for bounding-box metadata`

---

## 10. Open questions for the human

None. All questions from earlier revisions are resolved:

- Option name/shape: no option; the three fields are always present, nullable
  (human decision, revision 3).
- Field presence: always present as `RectF | null` (human decision, revision 3).
- Version: `0.4.0` additive minor per constitution §6 (human approval, revision
  4).

---

## Appendix A: differences from earlier revisions

| Revision 1 (withdrawn)                                                                | Revision 4 (this document)                                                                   |
| ------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| `bbox?: "content" \| "geometry" \| "canvas" \| false` mode selector (invented names)  | No option, no mode vocabulary                                                                |
| `bboxCrop` cropping with translate/scale render changes (resvg-js `cropByBBox` shape) | Dropped — no render-path changes; pixels always cover the full canvas as today               |
| Result field `bbox?: {…}` + `cropX`/`cropY`                                           | Three upstream-named fields: `absBoundingBox`, `absStrokeBoundingBox`, `absLayerBoundingBox` |
| Only two upstream methods used; `canvas` had no upstream analog                       | All three requested upstream methods exposed; nothing invented                               |
| Crop-specific sizing-rule interpretation (§4.3 applied to the box)                    | Unneeded — sizing rule untouched                                                             |
| (revision 2) `withBoundingBox?: boolean` opt-in                                       | Removed per human decision (revision 3); fields always present as `RectF \| null`            |
| (revision 3) version proposed as `0.4.0`, status DRAFT                                | Version fixed to `0.4.0` per human approval (revision 4); status APPROVED; §10 closed        |
