# Project Constitution: `@tksh/svg2ui8a`

This file is the **second** document an AI agent reads (after
`AGENTS.md`). It defines what the package *is*, what it *is not*, and
the non-negotiable decisions that govern its design.

The human owns this file. If the agent believes something here is
wrong, the agent must surface the issue, not rewrite the file.

---

## 1. Mission

`@tksh/svg2ui8a` is a JSR-published package that turns SVG strings into
**`Uint8Array`** representations — and, in the other direction, turns
those `Uint8Array` representations back into **RGBA pixel buffers**
that are also `Uint8Array`. **All payloads in this package are
`Uint8Array`.** Function return types may be `Uint8Array` or small
wrapper objects whose `Uint8Array` fields are the actual data; the
package never produces SVG strings, PNG bytes, JPEG bytes, or other
file formats.

The package name reflects this:

- `svg2ui8a` = "SVG to `Uint8Array`" — both as a function (SVG →
  `Uint8Array`) and as a value (`usvg` payload) and as a return
  (`rgba` payload). The two functions in the package are the two
  transitions across that boundary.
- The `ui8a` part is a deliberate shorthand for `Uint8Array`. It is
  the lingua franca of the package.

The package is intended for use as a JSR library, importable
selectively by subpath.

---

## 2. The two products

The package exposes **two** functions, in two subpath imports of one
package. They are not in the same file. They are not in the same
Wasm binary. They are imported separately and used separately.

### 2.1 `svg2usvg` — the producer

```ts
// Subpath:  jsr:@tksh/svg2ui8a/usvg
// Function: svg2usvg(svg: string): Promise<Uint8Array>
```

- Input: an SVG string.
- Output: a `Promise<Uint8Array>` of the **postcard-encoded
  `usvg::Tree`**.
- Purpose: produce a stable, content-addressable, opaque handle
  to the normalized SVG. Hash the bytes. Cache the bytes. Ship the
  bytes.

The `Uint8Array` is:

- The Rust `usvg::Tree` (as produced by `usvg::Tree::from_str` with
  `usvg::Options::default()`), serialized via
  `postcard::to_stdvec`.
- Self-contained and opaque from the consumer's perspective.
- Identical byte-for-byte when produced from visually equivalent
  SVGs (i.e. SVGs that `usvg` normalizes to the same `Tree`).

### 2.2 `usvg2rgba` — the consumer

```ts
// Subpath:  jsr:@tksh/svg2ui8a/rgba
// Function: usvg2rgba(usvg: Uint8Array, options?: Usvg2RgbaOptions): Promise<RgbaResult>
```

- Input:
  - `usvg`: a `Uint8Array` produced by `svg2usvg` (or by a
    compatible producer). Must be a valid postcard-encoded
    `usvg::Tree`.
  - `options`: optional.
- Output: a `Promise<RgbaResult>`, where `RgbaResult` carries both
  the pixel data and the metadata a downstream consumer needs to
  know what to do with it (width, height, alpha mode).
- Purpose: turn the content-addressable handle into RGBA bytes
  that a canvas, an image pipeline, or a downstream encoder
  (PNG, WebP, etc.) can consume.

The two functions are intentionally **separate entry points**. A
consumer who only wants to hash SVGs imports `./usvg` and never
loads the rasterizer. A consumer who already has a cached
`usvg` payload and wants pixels imports `./rgba` and never runs
the SVG parser. A consumer who wants both imports both, but the
two Wasm artifacts are loaded independently.

---

## 3. Hard constraints

These are not preferences. They are not negotiable during
implementation. Any plan that violates them must be rejected.

### 3.1 No fonts

The package is for SVG pipelines that **do not render text**. Fonts
are out of scope:

- No font loading, no font directory scanning, no
  `defaultFontFamily`.
- No system font enumeration.
- If the input SVG contains `<text>` elements, they will be parsed
  by `usvg` but may not render meaningfully downstream. This is the
  caller's problem, not ours.

### 3.2 No BBox

The package does not expose bounding boxes. There is no `getBBox()`,
no `innerBBox()`, no `cropByBBox()`. The `usvg` payload is the
whole tree, nothing more, nothing less.

### 3.3 No PNG / WebP / JPEG / any image format

The package does not encode final image files. There is no PNG
output, no WebP output, no JPEG output. The output of `usvg2rgba`
is **raw RGBA pixels as a `Uint8Array`**, and nothing else.

This implies:

- No `png` crate, no `image` crate, no `@jsquash/webp`, no
  `cwebp`-style tooling in the Wasm builds.
- No PNG decoder on the JS side, ever.

If a consumer needs PNG output, they pipe the RGBA bytes through
their own encoder. That encoder is the consumer's concern, not
ours.

### 3.4 No PNG decoder on the JS side

Do not pull in `pngjs`, `sharp`, `@jsquash/png`, or any equivalent.
The package produces no PNG and consumes no PNG. If a consumer
needs to read PNG dimensions, they use their own helper, in their
own code, in their own repo.

### 3.5 No native fallback

Do not introduce a native Rust binary, a Node.js addon, or any
non-Wasm build target. The package is Wasm-only.

### 3.6 No Node.js runtime

Do not target Node.js. The package is for Deno and the browser.
Imports use Deno-style specifiers (`jsr:`, `deno:`, or relative
paths). `npm:` is not allowed in package code (see
`./AGENTS.md` §5.1).

### 3.7 No custom serialization

The serialization format is `postcard`. It is not negotiable. Do
not introduce CBOR, MessagePack, bincode, JSON, or any other
format.

`postcard` is the format because it is the smallest and fastest
option that requires zero new dependencies on the JS side (the JS
side never has to decode it; it only passes the bytes to a hashing
function or to `usvg2rgba`).

### 3.8 No runtime CDN imports in the browser bundle

The browser bundle must contain every byte it needs to run. See
`./AGENTS.md` §5.2 for the full rule. The package's browser bundle
must not import from `https://deno.land/x/...`,
`https://esm.sh/...`, or any other runtime CDN.

### 3.9 Two Wasm artifacts, not one

The `usvg` and `rgba` builds are **two independent Wasm binaries**
served under two independent subpath imports. They are not
bundled. They are not feature-gated within a single Wasm artifact.
The agent must not propose a "single Wasm with both functions"
approach, even if it would be smaller in the abstract, because it
would force every consumer to load code they do not use.

---

## 4. API surface

### 4.1 `svg2usvg`

```ts
// jsr:@tksh/svg2ui8a/usvg
export function svg2usvg(svg: string): Promise<Uint8Array>;
```

- Input: an SVG string. No options, no second argument.
- Output: a `Promise<Uint8Array>` of the postcard-encoded
  `usvg::Tree`.
- Errors: rejected with an error if the SVG cannot be parsed by
  `usvg`. Error type is whatever `wasm-bindgen` produces; consumers
  are expected to surface it.

### 4.2 `usvg2rgba`

```ts
// jsr:@tksh/svg2ui8a/rgba

export type AlphaMode = "straight" | "premultiplied";

export interface Usvg2RgbaOptions {
  width?: number;
  height?: number;
  alphaMode?: AlphaMode;          // default: "straight"
}

export interface RgbaResult {
  pixels: Uint8Array;             // width * height * 4 bytes
  width: number;
  height: number;
  alphaMode: AlphaMode;           // matches the option that produced it
}

export function usvg2rgba(
  usvg: Uint8Array,
  options?: Usvg2RgbaOptions,
): Promise<RgbaResult>;
```

- Input:
  - `usvg`: a `Uint8Array` produced by `svg2usvg` (or by a
    compatible producer). Must be a valid postcard-encoded
    `usvg::Tree`.
  - `options`: optional.
    - `width`: render target width in pixels. If omitted, the
      natural width of the SVG is used.
    - `height`: render target height in pixels. If omitted, the
      natural height of the SVG is used.
    - `alphaMode`: `"straight"` (default) returns
      non-premultiplied RGBA, with the Wasm side doing the
      unmultiply. `"premultiplied"` returns the
      `tiny-skia` pixmap bytes as-is, with channels
      premultiplied by alpha.
- Output: a `Promise<RgbaResult>` where:
  - `pixels` is the raw RGBA buffer (`R, G, B, A, ...`).
  - `width` and `height` are the actual rendered dimensions,
    after any options are applied.
  - `alphaMode` is the alpha mode of the returned `pixels`
    (matches the option, or `"straight"` if the option was
    omitted).
- Errors: rejected if:
  - the input `usvg` bytes are not a valid postcard-encoded
    `usvg::Tree`,
  - the requested `width` or `height` is not a positive
    integer,
  - the SVG's natural size is zero in either dimension,
  - the Wasm linear memory is exhausted.

### 4.3 Independent scaling

If both `width` and `height` are omitted, the natural SVG size is
used. If only one is provided, the other is taken from the
natural SVG size — the output is **exactly** `width × height`,
with non-uniform scaling if the aspect ratio of
`(width, height)` does not match the natural aspect ratio. **This
is a deliberate design choice, not a bug.** It mirrors `resvg`'s
default behavior.

The natural size comes from `tree.size()` (user units). The cast
to `u32` is rounding toward zero (Rust `as u32` for the `f32` /
`f64` natural size). The natural size of a well-formed SVG is in
practice always a small integer; pathological floats are the
caller's problem.

### 4.4 Things that are explicitly *not* in the API

For both functions:

- No `format` option (`'binary' | 'json' | ...`). The format is
  `postcard`. Always.
- No `background` / `backgroundColor` parameter. There is no
  background in the output; the canvas is fully transparent
  outside the painted shapes.
- No `dispose` method on the module. The Wasm instances are
  stateless and managed by the runtime.
- No synchronous variant. The Wasm calls are `async`. Always.
- No `toString()` / SVG-string output anywhere. The whole point is
  to *avoid* the SVG string.

For `svg2usvg` only:

- No `width` / `height` / `scale` parameters. The output dimensions
  live in the tree.

For `usvg2rgba` only:

- No `shapeRendering` / `textRendering` / `imageRendering` knobs.
  Defaults from `resvg` are used.

### 4.5 Things that are explicitly *not yet* in scope

The following are intentionally deferred:

- A "convenience" PNG-encoding helper that turns
  `RgbaResult.pixels` into a PNG file. If added, it would be a
  *third* subpath (`./png`) and a *third* Wasm build, with the
  PNG encoder as an opt-in dependency. It is not this package.
- WebP encoding. Same story.
- A "convenience" wrapper that takes an SVG string and returns
  RGBA in one call (i.e. `svg2usvg` + `usvg2rgba` chained). If
  added, it would be a *fourth* subpath. It is not this package.
- A structured-object input (e.g. a JS object representing the
  vector graphic, instead of an SVG string). This is a design
  question the human has not yet answered; see
  `./AGENTS.md` §10.

The agent must not implement any of these as part of an
`svg2ui8a` task. If the human asks for them, the human will create
a separate task and a separate plan.

---

## 5. What the output bytes mean

### 5.1 `svg2usvg` output

The `Uint8Array` returned by `svg2usvg` is the postcard-encoded
`usvg::Tree`. It is:

- Self-contained, opaque from the consumer's perspective.
- Identical byte-for-byte when produced from visually equivalent
  SVGs. This is the property that makes it useful as a cache key
  input.
- Not a `usvg`-Tree-bytes, then `usvg`-Tree-`options`-bytes; the
  natural dimensions are encoded inside the tree, and a consumer
  that wants a different size passes `width` / `height` to
  `usvg2rgba`, not to `svg2usvg`.

### 5.2 `usvg2rgba` output

The `Uint8Array` returned in `RgbaResult.pixels` is:

- Raw RGBA pixels, packed `R, G, B, A, R, G, B, A, ...`.
- One pixel per 4 bytes.
- Total length: `width * height * 4`.
- Row-major: the first `width * 4` bytes are the first row
  (top-to-bottom), the next are the second row, etc.
- **No** PNG header, **no** EXIF, **no** compression. Pure pixels.
- **Fully zero-initialized** before the SVG is drawn. Any pixel
  not painted by the SVG has zero alpha (and zero RGB).
- Alpha mode matches the `alphaMode` field of the
  `RgbaResult`. With the default `"straight"`, channels are
  **not premultiplied** (i.e. a 50%-opaque red pixel is
  `(255, 0, 0, 128)`, not `(128, 0, 0, 128)`). With
  `"premultiplied"`, channels **are premultiplied** by alpha
  (the unmodified `tiny-skia` pixmap bytes).

### 5.3 Determinism caveat

`postcard` does not, by spec, guarantee deterministic encoding.
The package relies on `usvg::Tree`'s structure being
**sufficiently ordered** that `postcard` produces the same bytes
across runs. The agent should not "fix" this by re-ordering
fields or by switching formats; if a determinism bug is found,
escalate per `./AGENTS.md` §10.

The intended escape hatch for format changes is the version
prefix in the cache key (see §6), not a serialization rewrite.

---

## 6. Cache key guidance (for the consumer, not for this package)

The package itself does not produce a cache key. The recommended
pattern, documented here so the agent does not invent something
different:

```
cacheKey = "<usvg-version>-<postcard-version>-" + hex(sha256(bytes))
```

The prefix is the consumer's responsibility. Including the
`usvg` and `postcard` versions ensures that any change in the
serialization pipeline (a `usvg` upgrade, a `postcard` upgrade, a
Rust compiler change that alters layout) automatically invalidates
all existing cache entries without code-level coordination.

---

## 7. Versioning

The package follows **strict semver**:

- Patches: Wasm runtime tweaks that do not change the output bytes
  for any input.
- Minors: additions to the API surface (none planned at present).
  Output bytes may change.
- Majors: changes to the output format itself, or removals from the
  API surface.

The output bytes of `svg2usvg` are **not** part of the public API
contract beyond semver. The consumer is expected to use the
version-prefixed cache key from §6 to handle byte changes
gracefully.

---

## 8. Anti-goals

The following are explicitly *not* goals of the package, and the
agent must not be tempted to add them:

- General-purpose SVG manipulation. This is not an SVG editor.
- A unified "SVG toolkit". This is two functions.
- Drop-in compatibility with `@resvg/resvg-js` or
  `svg2png-wasm`. Those libraries solve different problems; this
  package solves a smaller one, deliberately.
- Speed of a single render at the cost of consumer-side
  complexity. The package optimizes for the cache-hit case (no
  SVG parse, no format conversion), not for the cache-miss case.
- Supporting legacy bundlers. Deno and modern browser bundlers
  only.
- Producing anything other than `Uint8Array` payloads (SVG
  strings, PNG bytes, JPEG bytes, WebP bytes, etc.). Return-type
  wrappers (`RgbaResult`) are allowed because their `pixels`
  field is `Uint8Array`; producing an SVG string or a PNG byte
  array is not.
