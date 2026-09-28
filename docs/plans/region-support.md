# Plan: `region` Support for `@tksh/svg2ui8a/svg2rgba` (revision 2)

**Status:** APPROVED for `0.5.0` — ready for implementation (`AGENTS.md` §2;
`docs/engineering-playbook.md` §4 step 3). The option name is confirmed as
`region` (§2.3, §11.7).

**Task source:** the human asked to pause `svg2ui8a-demo` work and add
`region?: RectF` to `Svg2RgbaOptions` in this repository, targeting version
`0.5.0`.

**Revision 2 (2026-09-28):** records the human's answers to §11 (constitution
wording approved, `RectF` re-export approved, one-sided sizing from the region
aspect confirmed, invalid regions rejected, doc-comment cleanup approved, demo
work deferred) and the naming reconsideration requested by the human, with
`region` approved (§2.3, §11.7). `docs/project-constitution.md` §4.5 was updated
in this revision at the human's explicit request.

**Context:** the demo-side investigation is recorded in
`svg2ui8a-memo/2026-09-28/bbox-overflow-and-render-region.md`. It established,
against docs.rs 0.48.1 and the upstream source, that `resvg::render` takes a
root transform, clips only to the pixmap bounds, and never clips to the
document's viewBox. A `RectF` render region is therefore a thin mapping over an
existing `resvg` capability, not a new rendering behavior. The demo-side "Fit to
bounds" UI (paused) is a follow-up, not part of this plan.

---

## 0. Summary

Add one optional field to `svg2rgba`'s options:

```ts
region?: RectF; // canvas-space source window to rasterize
```

- `region` selects which part of the SVG's canvas is rendered into the output
  pixmap, via a translate+scale root transform. Default:
  `{x: 0, y: 0, width: naturalWidth, height: naturalHeight}`.
- The output size (`width`/`height`) is unchanged in meaning: it is the pixmap
  size. When `width`/`height` are omitted, the region's dimensions (rounded)
  replace the natural dimensions in the existing sizing rule.
- `naturalWidth`/`naturalHeight` and all three bounding boxes are unchanged:
  they remain measurements of the SVG's natural canvas. Nothing about the result
  object changes.
- Omitting `region` is byte-identical to `0.4.0` (regression guarantee).
- This is the library-side piece that lets a consumer (e.g. the demo) render the
  union of the bounding boxes, so strokes and content that overflow the
  document's viewBox become visible.

`svg2usvg` is untouched. No new crate, no new Wasm artifact, no new subpath.

---

## 1. Constitution authorization

Quoting `docs/project-constitution.md` §4.2 (playbook §4 step 2):

> The exact shape of `Svg2RgbaOptions` and `RgbaResult` is an implementation
> choice. The shape below is a _reference shape_, not a contract. The
> implementation may add or rename fields, provided the boundary in §2.2 is
> preserved.

An optional, additive field is within that latitude. The §2.2 boundary is
preserved: input is still an SVG string plus options; output is still raw RGBA
in a `Uint8Array` with metadata.

Related sections and how this plan respects them:

- §2.2 — no PNG/JPEG encoding, no intermediate payload; untouched.
- §3.7 — no proprietary format; `region` is four plain numbers.
- §3.9 — no new Wasm artifact; the option rides the existing `svg2rgba` build.
- §4.4 — no `format`/`background`/`dispose`/sync variant added.
- §6 — an additive, backward-compatible API addition is a **minor** release:
  `0.5.0`.
- §4.5 — the deferred list previously ended with "per-element lookup and
  cropping remain out of scope". The human approved the recommended wording
  change; §4.5 now records that a raw render-region override (the existing
  `resvg` render transform, exposed as `region`) is authorized for `0.5.0` under
  this plan, while `resvg-js`-style `getBBox()`/`cropByBBox()` remain out of
  scope. The agent edited that governing document only because the human
  explicitly requested it in revision 2.

---

## 2. API surface (reference shape)

`jsr:@tksh/svg2ui8a/svg2rgba`:

```ts
export interface RectF {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface Svg2RgbaOptions {
  width?: number;
  height?: number;
  alphaMode?: "straight" | "premultiplied";

  /**
   * Source window to rasterize, in canvas coordinates (the space of
   * `naturalWidth`/`naturalHeight` and the `abs*` bounding boxes). Defaults to
   * the full natural canvas. May be negative and may exceed the canvas; the
   * window is not clipped to it.
   */
  region?: RectF;
}
```

`RgbaResult` is unchanged (no new fields).

### 2.1 Semantics

- **Coordinate space.** Canvas coordinates: the space `usvg` produces after
  baking the root viewBox into the tree, where 1 unit = 1 pixel at the natural
  size. The three bounding boxes are in this same space, so
  `region: absLayerBoundingBox` composes directly.
- **Transform.** With `scale_x = outputWidth / region.width` and
  `scale_y = outputHeight / region.height`, a canvas point `p` maps to
  `((p - region.xy) * scale)`. Non-uniform when both `width` and `height` are
  set, exactly like today's natural-size scaling.
- **Sizing rule with `region`** (otherwise identical to §4.3):
  - `width` and `height` omitted: output is
    `round(region.width) × round(region.height)`.
  - one set: the other is derived from the **region** aspect ratio and rounded
    (fractional region dimensions are kept until the final rounding, matching
    today's fractional-natural behavior).
  - both set: output is exactly `width × height` with independent scaling of the
    region.
  - A region whose rounded size is 0 still hits the existing
    `"svg2rgba error: zero requested size"` path.
- **Metadata.** `naturalWidth`, `naturalHeight`, `alphaMode`, `pixels`, and the
  three `abs*BoundingBox` fields keep their current meanings and values. In
  particular the boxes are **not** re-based to the region; a consumer mapping
  boxes to output pixels uses `scale = output / region` and offset `-region.xy`
  (documented in README).
- **Validation.** When `region` is present: `x`/`y` must be finite, and
  `width`/`height` finite and `> 0`; otherwise the promise rejects with an
  `"svg2rgba error: ..."` message, never a panic or a blank render. Values are
  `f32`, like all other geometry in the package.
- **Default.** `region` omitted (or the explicit full-canvas region
  `{0, 0, naturalWidth, naturalHeight}`) produces byte-identical pixels to
  `0.4.0`.
- **No interference with viewBox/preserveAspectRatio.** `usvg` has already
  applied them; `region` operates on the resulting canvas space.
- **Outside-viewBox content.** If the region covers content outside the
  document's viewport, that content is rendered: `resvg::render` does not clip
  to `tree.size()`, only the pixmap bounds clip (§6).

### 2.2 Explicit non-goals

- No `fitBounds`/auto-fit option. Choosing a box, padding it, and rounding it is
  consumer policy; the three box outputs + `region` make it a two-call operation
  for any consumer. A package-unique auto-fit would violate the spirit of
  "faithful `resvg` wrapper, no package-unique rendering features".
- No padding, clamping, or defaulting of `region` from the bounding boxes.
- No per-element lookup, no `cropByBBox`/`getBBox` shape.
- No `region` on `svg2usvg` (normalization is viewport-independent).
- No change to `RgbaResult`, `alphaMode`, or the existing sizing behavior when
  `region` is absent.

### 2.3 Naming rationale

Reconsidered in revision 2 after the human asked whether `transform` (the name
of `resvg::render`'s parameter) would be more faithful than `region`.

What upstream actually has:

- `resvg::render(tree: &usvg::Tree, transform: tiny_skia::Transform, pixmap:
  &mut tiny_skia::PixmapMut)`.
  The parameter is a `Transform`, i.e. a 6-number affine matrix (translate,
  scale, rotate, skew) — a _type_, not a noun for a rectangular window.
- There is no upstream API, type, or option named "region".
- The closest upstream user-facing vocabulary is the CLI's "export area"
  (`--export-area-drawing`, `--export-area-node`), which is CLI surface, not
  library API.

If the package adopted the word `transform` literally, the option would have to
be a matrix (or at least translate/scale numbers):

- it would be a much larger public surface (a new TypeScript transform type and
  its wasm-boundary encoding);
- it would enable rotation/skew, which this package has no use case for and
  would have to document and test;
- it would push the "which window contains the boxes" arithmetic back onto every
  consumer, defeating the motivation (boxes and region compose directly);
- "transform" already means the SVG `transform` attribute to every consumer, so
  a rect option named `transform` would be actively confusing.

The BBox precedent was "use upstream names where upstream names the data".
There, upstream has exact names for exactly the exposed data
(`abs_bounding_box()` etc.). Here, upstream has no name for "the part of the
canvas to rasterize"; it only has the matrix it accepts. The minimal, honest
expression of that concept is a rectangle, and `transform` is the one word that
should stay reserved: keeping it unused leaves room for a future faithful affine
option (e.g. `transform?: { a, b, c, d, e, f }` or translate/scale/rotate) built
on the very same `resvg` capability.

Alternatives considered:

- `viewBox` — collides with the document's own viewBox and suggests re-parsing
  or mutating the document, which this option explicitly does not do.
- `crop` — implies narrowing only, but this window can also expand.
- `zoom` + `translate` — two knobs and the CLI's private vocabulary; more
  surface for the same rect.
- `area` / `exportArea` — closest to the upstream CLI vocabulary, but "area" is
  ambiguous (area as a measure) and the CLI vocabulary is not library API.
- `transform` — rejected for the reasons above.
- `fitBounds` — policy, see §2.2.

**Decision: keep `region`** (human-approved, revision 2; §11.7). It is the
clearest noun for a rectangular source window, it is already the word used in
the demo-side memo, and it leaves `transform` available for a future affine
option. `area` (CLI-flavored) and a real `transform` matrix option remain the
candidates for a separate, larger plan if a strict upstream-first call is ever
preferred.

---

## 3. Implementation by layer

### 3.1 `crates/svg2rgba/src/core.rs` (Rust core)

- `RgbaOptions` gains `pub region: Option<UpstreamRect>`. `UpstreamRect` is
  reused as the shared "four f32 numbers" struct; its doc comment is updated to
  say it carries both upstream `usvg` rects and the render region. (Renaming it
  to `RectF` is possible but is a wider diff; not proposed.)
- `rasterize_svg`:
  1. Validate `region` when `Some` (§2.1).
  2. Keep `natural_w`/`natural_h` for metadata and the zero-natural-size check.
  3. Compute the sizing rule from `region` dimensions when present, from the
     natural dimensions otherwise.
  4. Build the root transform:
     ```rust
     let transform = match region {
         Some(r) => Transform::from_scale(scale_x, scale_y)
             .pre_translate(-r.x, -r.y),
         None => Transform::from_scale(scale_x, scale_y),
     };
     ```
     An explicit
     `Transform::from_row(scale_x, 0.0, 0.0, scale_y,
     -r.x * scale_x, -r.y * scale_y)`
     is equivalent and acceptable. The mapping is pinned by the native test in
     §4.1 rather than by reading the `pre_translate` docs: upstream
     `render_node` uses `transform.pre_translate(-bbox.x(), -bbox.y())` for
     exactly this shift-to-origin-then-apply pattern.
  5. `resvg::render(&tree, transform, &mut pixmap.as_mut())` as today.
- Pixels, alpha handling, and the box read-back are untouched.

### 3.2 `crates/svg2rgba/src/lib.rs` (wasm-bindgen boundary)

- `Svg2RgbaOptions` gains `region: bool` plus `region_x`, `region_y`,
  `region_width`, `region_height: f32` (0 defaults in `new()`). Inputs use the
  existing `0`-means-omitted style; `region: bool` distinguishes "region given"
  from "region absent" without `Option<f32>` sentinels.
- `svg2rgba()` maps them into `RgbaOptions.region` (`Some(UpstreamRect{..})`
  when the flag is set, `None` otherwise). The result mapping is unchanged.

### 3.3 `scripts/build.ts` (TS glue template; `src/*.ts` is regenerated)

- `Svg2RgbaOptions` TS interface: add `region?: RectF` with the JSDoc from §2.
- Wrapper body: set `opts.region = options?.region !== undefined;` and, when
  present, assign the four `f32` fields before `__wasm_svg2rgba(svg, opts)`.
- `generateModTs()`: add `RectF` to the re-exported types (approved, §11.2), so
  root-import consumers can type the option.
- One-line cleanup while the template is open (approved, §11.5): the doc comment
  above `RectF` currently reads "The result of a `svg2rgba` render."; it
  describes a rect, so it becomes something like "A rectangle in canvas
  coordinates."

### 3.4 README and CHANGELOG

- README: add `region` to the `Svg2RgbaOptions` listing, one example ("render a
  bounding box that lies outside the viewBox"), and the mapping note
  (`scale = output / region`, offset `-region.xy`) so box consumers get the
  arithmetic right.
- CHANGELOG: `0.5.0` entry (§10).
- The four governing documents are not edited.

---

## 4. Tests (all four layers, per playbook §3)

### 4.1 Rust native — `crates/svg2rgba/tests/core.rs`

- `region_omitted_and_full_canvas_region_match_baseline` — `None` vs explicit
  `{0, 0, naturalW, naturalH}` vs pre-change pixels: byte-identical.
- `region_selects_the_source_window` — a small colored square at a known canvas
  offset; a region around it produces the expected output size and the square's
  pixels; pixels outside the square are transparent.
- `region_reveals_content_outside_the_viewbox` — the strongest case: SVG with
  `viewBox="0 0 10 10"` and a `20×20` rect at `(100, 100)`. With no region the
  output is fully transparent; with `region = {100, 100, 20, 20}` every pixel is
  the rect's color.
- `region_covers_a_bounding_box_that_overflows` — a Straightlines-style fixture
  (31×31 viewBox, `stroke-width="31"` at x=15/16, stroke box
  `{-0.5, 0, 32, 31}`): `region = absStrokeBoundingBox` (and equivalently the
  literal rect) paints the previously clipped edge columns.
- `region_supports_negative_origins` — `region.x`/`y` < 0.
- `region_sizing_rule` — both omitted (region dims rounded), only `width`, only
  `height` (derived from the region's fractional aspect), both set (independent
  scaling).
- `region_is_deterministic_across_calls`.
- `invalid_region_errors` — `width <= 0`, `height <= 0`, NaN/∞ coordinates: all
  `Err`, no panic.
- `region_does_not_change_metadata_or_boxes` — `naturalWidth`/`naturalHeight`
  and all three boxes identical with and without a region (same canvas space).

### 4.2 Wasm layer — `scripts/test-wasm.ts`, `scripts/test-wasm-browser.ts`

- `crates/svg2rgba/examples/dump_core.rs`: keep the existing
  `<width> <height> <alpha_mode>` arguments and the raw-pixel stdout; accept an
  optional trailing `region` quartet (`<x> <y> <w> <h>`, all-or-none).
- Both harnesses gain region cases that pass the quartet and compare the Wasm
  output to the native core output byte-for-byte (pixels) and field-for-field
  (metadata). They also pin the boundary conversion of the TS option object to
  the wasm options struct (flag + four floats), including a region with negative
  origin.
- Existing no-region cases are unchanged, proving the boundary still converts
  the option surface exactly as before.

### 4.3 Deno tests — `tests/svg2rgba.test.ts`

- `region` is accepted as a bare object literal, alongside `width`/`height`/
  `alphaMode` (extend the existing plain-literal test).
- Omitted region, `{}`, and an explicit full-canvas region produce identical
  pixels.
- Region + sizing/alpha combinations produce the expected dimensions; the option
  object is not mutated.
- Invalid regions reject the promise (zero/negative/NaN dimensions).
- The result shape is unchanged: still a plain object, no new keys, survives
  `structuredClone` and JSON round-trips.
- Determinism with a region across consecutive calls.

### 4.4 Deno tests — `tests/conformance.test.ts`

- Render the normalized `svg2usvg` bytes with a region and compare with the
  original SVG rendered with the same region: identical pixels. This exercises
  the region across both Wasm modules and keeps the existing cross-Wasm
  guarantees.

---

## 5. Wasm size impact (playbook §4 step 2)

Baseline at HEAD `fc6be4b`: `assets/svg2rgba_bg.wasm` = 997,539 bytes,
`assets/svg2usvg_bg.wasm` = 657,772 bytes. `region` adds five option fields
(flag + four floats) and one transform composition in already-linked code;
expected delta is well under 1 KB, unmeasured until the build. Record
before/after sizes in the implementation report. `svg2usvg_bg.wasm` is
byte-identical.

---

## 6. Verification already performed for this plan (no repo code changed)

- The upstream contract was researched for the demo memo against docs.rs 0.48.1
  and the resvg source: `render` has no viewBox clip and takes a root transform;
  `render_node` shifts by `abs_layer_bounding_box` via `pre_translate`; the
  CLI's private `FitTo` / `--export-area-drawing` recipes are scale/translate +
  pixmap sizing. The API is stable since `FitTo` was removed in 0.33;
  `Cargo.toml` pins `resvg 0.47.0`.
- A Wasm probe with the published `0.4.0` measured the concrete case:
  Straightlines (`viewBox="0 0 31 31"`) reports
  `absStrokeBoundingBox = {-0.5, 0, 32, 31}` at any output size; the region
  `{-0.5, 0, 32, 31}` is the "show everything" window.
- `tiny_skia::Transform` is already imported and used by `core.rs`; no new
  dependency.
- Caveat recorded: the exact composition order of
  `from_scale(..).pre_translate(..)` is pinned by a native test (§4.1) instead
  of being trusted from memory.

---

## 7. Dependency changes (playbook §4 step 2 / `AGENTS.md` §5.3)

**None.** No Cargo or JSR dependency is added or version-bumped.

---

## 8. Files to touch

| File                                                   | Change                                                    |
| ------------------------------------------------------ | --------------------------------------------------------- |
| `crates/svg2rgba/src/core.rs`                          | `RgbaOptions.region`, validation, transform, sizing       |
| `crates/svg2rgba/src/lib.rs`                           | wasm options fields + mapping                             |
| `crates/svg2rgba/tests/core.rs`                        | native tests (§4.1)                                       |
| `crates/svg2rgba/examples/dump_core.rs`                | optional region quartet for the wasm harnesses            |
| `scripts/build.ts`                                     | options template, option mapping, `mod.ts` export         |
| `src/svg2rgba.ts`, `src/mod.ts`                        | regenerated by `deno task build` (never hand-edited)      |
| `tests/svg2rgba.test.ts`                               | Deno tests (§4.3)                                         |
| `tests/conformance.test.ts`                            | cross-Wasm region test (§4.4)                             |
| `scripts/test-wasm.ts`, `scripts/test-wasm-browser.ts` | wasm-layer region cases (§4.2)                            |
| `assets/svg2rgba_bg.wasm`                              | regenerated build artifact (committed per `AGENTS.md` §7) |
| `README.md`                                            | option docs, example, box-to-pixel mapping note           |
| `CHANGELOG.md`                                         | `0.5.0` entry                                             |

Untouched: `crates/svg2usvg/**`, `assets/svg2usvg_bg.wasm`, `src/svg2usvg.ts`,
and the four governing documents.

---

## 9. Implementation order & verification

1. `core.rs` + native tests → `deno task test:rust`.
2. `lib.rs` + `build.ts`, then `deno task build` (wasm-pack 0.15.0, cargo
   1.97.0; the build runs the CDN-free check, playbook §5.2).
3. Wasm harnesses + Deno/conformance tests → `deno task test`.
4. `deno task fmt`, `deno task lint`, `deno task check`.
5. Record Wasm sizes before/after; update README + CHANGELOG; report per
   `AGENTS.md` §9.

Commits (one logical change each, per `AGENTS.md` §7):

- `svg2ui8a: add region option to svg2rgba (constitution §4.2)`
- `svg2ui8a: regenerate wasm + glue for the region option`

---

## 10. Version `0.5.0` and release handling

`0.5.0` is the approved target version and matches constitution §6 (additive
minor). Per playbook §6 and `CHANGELOG.md` `0.3.3`, **the version is not stored
in `deno.json`**: the release tag is the single source of the published version.
Concretely:

1. Add a `# 0.5.0` section at the top of `CHANGELOG.md` describing the `region`
   option (same style as the `0.4.0` entry), referencing this plan.
2. The implementation commits do **not** bump any version field (there is none
   to bump). If the human expected a version field somewhere, that contradicts
   the playbook; flagged here instead of inventing one.
3. Tagging and publishing remain the human's step: annotated tag `v0.5.0` on the
   release commit, pushed with `git push --tags`; the publish workflow runs
   `deno publish --set-version 0.5.0` (playbook §6). The agent does not tag,
   publish, or push (§8 of `AGENTS.md`).

---

## 11. Human decisions (resolved in revision 2)

All six questions from revision 1 are resolved by the human (answers recorded
2026-09-28):

1. **Constitution §4.5 wording** — approved as recommended. `§4.5` was updated
   in this revision: a raw render-region override is authorized for `0.5.0`
   under this plan; `resvg-js`-style `getBBox()`/`cropByBBox()` remain out of
   scope.
2. **`RectF` re-export from the root subpath** — approved; the `mod.ts` template
   re-exports `RectF` alongside `RgbaResult` and `Svg2RgbaOptions`.
3. **Region aspect for one-sided sizing** — confirmed: derive the missing
   dimension from the region's aspect ratio.
4. **Invalid regions** — confirmed: reject (zero/negative/NaN/∞).
5. **`RectF` doc-comment cleanup in `scripts/build.ts`** — approved.
6. **Demo-side "Fit to bounds"** — deferred. A plan will be created inside
   `svg2ui8a-demo` once this change is implemented, committed, and published.

The human raised one follow-up in the same round:

7. **Naming: `region` vs `transform`** — **approved: keep `region`** (rationale
   in §2.3). `transform` is an upstream _type_ (affine matrix), not this option,
   and naming a rect `transform` would be a vocabulary mismatch. The decision
   implies no design change; implementation uses `region` as planned.

---

## Appendix A: semantics examples

| Options                                              | Output size                         | Rendered window                       |
| ---------------------------------------------------- | ----------------------------------- | ------------------------------------- |
| (no region, no width/height)                         | `round(naturalW) × round(naturalH)` | full natural canvas, `0.4.0` behavior |
| `region = {0,0,naturalW,naturalH}` (no width/height) | same as above                       | same, byte-identical                  |
| `region = {-0.5,0,32,31}` (no width/height)          | `32 × 31`                           | the overflowing stroke box            |
| `region = {100,100,20,20}` (no width/height)         | `20 × 20`                           | content entirely outside the viewBox  |
| `region = {0,0,10,10}, width = 100`                  | `100 × 100`                         | top-left 10×10 scaled up uniformly    |
| `region = {0,0,10,10}, width = 100, height = 50`     | `100 × 50`                          | non-uniform scaling of the region     |
| `region = {0,0,0,10}` or NaN coordinates             | rejects (`svg2rgba error: ...`)     | —                                     |

## Appendix B: what the demo memo asked for, and what this plan delivers

| Memo requirement (demo-side)                | Library-side answer here                                         |
| ------------------------------------------- | ---------------------------------------------------------------- |
| Render a region covering the bounding boxes | `region?: RectF` (§2)                                            |
| Keep it implementable from upstream `resvg` | one `Transform` passed to the existing `resvg::render` call (§3) |
| No package-unique rendering features        | no auto-fit, no padding, no box defaults (§2.2)                  |
| Bounding boxes stay usable for mapping      | boxes remain canvas-space; mapping documented in README (§2.1)   |
| Version `0.5.0`                             | §10, tag-driven release                                          |
