# Project Constitution: `svg2ui8a`

This file is the **second** document an AI agent reads (after
`AGENTS.md`). It defines what `svg2ui8a` *is*, what it *is not*, and
the non-negotiable decisions that govern its design.

The human owns this file. If the agent believes something here is
wrong, the agent must surface the issue, not rewrite the file.

---

## 1. Mission

`svg2ui8a` is a JSR-published package that turns SVG strings into
**`Uint8Array`** representations — and, in the other direction, turns
those `Uint8Array` representations back into **RGBA pixel buffers**
that are also `Uint8Array`. **Everything in this package is
`Uint8Array`. Nothing in this package is a final image file (PNG,
JPEG, WebP, etc.) and nothing is an SVG string.**

The package name reflects this:

- `svg2ui8a` = "SVG to `Uint8Array`" — both as a function (SVG →
  `Uint8Array`) and as a value (`usvg` payload) and as a return
  (`rgba` payload). The two functions in the package are the two
  transitions across that boundary.
- The `ui8a` part is a deliberate shorthand for `Uint8Array`. It is
  the lingua franca of the package.

The package is intended for use in:

- A Deno-first web application, both in the browser (bundled by
  `deno bundle`) and on Deno Deploy.
- Future consumers who want a portable, content-addressable form of
  an SVG they can hash, cache, or ship across a network — and who
  also want to rasterize that form to RGBA when they need pixels,
  without leaving the package.

---

## 2. The two products

`svg2ui8a` exposes **two** functions, in two subpath imports of one
package. They are not in the same file. They are not in the same
Wasm binary. They are imported separately and used separately.

### 2.1 `svg2usvg` — the producer

```ts
// Subpath:  jsr:@your-org/svg2ui8a/usvg
//           @your-org/svg2ui8a/usvg
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
// Subpath:  jsr:@your-org/svg2ui8a/rgba
//           @your-org/svg2ui8a/rgba
// Function: usvg2rgba(usvg: Uint8Array): Promise<Uint8Array>
```

- Input: a `Uint8Array` produced by `svg2usvg` (or by a compatible
  producer).
- Output: a `Promise<Uint8Array>` of **raw RGBA pixels**, packed
  as `R, G, B, A, R, G, B, A, ...`, one pixel per 4 bytes.
- Purpose: turn the content-addressable handle into something a
  canvas, an image pipeline, or a downstream encoder (PNG, WebP,
  etc.) can consume.
- The output `Uint8Array` is **width × height × 4** bytes long.
  Width and height are encoded in the input `usvg` payload; the
  caller may pass `width` and `height` as optional arguments to
  override the natural size of the SVG.

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

`svg2ui8a` is for SVG pipelines that **do not render text**. Fonts
are out of scope:

- No font loading, no font directory scanning, no
  `defaultFontFamily`.
- No system font enumeration.
- If the input SVG contains `<text>` elements, they will be parsed
  by `usvg` but may not render meaningfully downstream. This is the
  caller's problem, not ours.

### 3.2 No BBox

`svg2ui8a` does not expose bounding boxes. There is no `getBBox()`,
no `innerBBox()`, no `cropByBBox()`. The `usvg` payload is the
whole tree, nothing more, nothing less.

### 3.3 No PNG / WebP / JPEG / any image format

`svg2ui8a` does not encode final image files. There is no PNG
output, no WebP output, no JPEG output. The output of `usvg2rgba`
is **raw RGBA pixels as a `Uint8Array`**, and nothing else.

This implies:

- No `png` crate, no `image` crate, no `@jsquash/webp`, no
  `cwebp`-style tooling in the Wasm build.
- No PNG decoder on the JS side, ever (see §3.4).

If a consumer needs PNG output, they pipe the RGBA bytes through
their own encoder. That encoder is the consumer's concern, not
ours.

### 3.4 No PNG decoder on the JS side

Do not pull in `pngjs`, `sharp`, `@jsquash/png`, or any equivalent.
The only PNG-related code that may appear in this package is the
**IHDR-only size helper** (a 6-line function that reads the width
and height out of the PNG header) — and even that is a consumer
concern, not a package concern. The package itself contains no
PNG-decoding code.

### 3.5 No native fallback

Do not introduce a native Rust binary, a Node.js addon, or any
non-Wasm build target. The package is Wasm-only.

### 3.6 No Node.js runtime

Do not target Node.js. The package is for Deno and the browser.
Imports must use Deno-style specifiers (`jsr:`, `npm:` via
Deno's resolver, or relative paths).

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
`./AGENTS.md` §5.2 for the full rule. `svg2ui8a`'s browser bundle
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
// jsr:@your-org/svg2ui8a/usvg
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
// jsr:@your-org/svg2ui8a/rgba
export interface Usvg2RgbaOptions {
  width?: number;
  height?: number;
}

export interface RgbaResult {
  pixels: Uint8Array;   // width * height * 4 bytes, RGBA
  width: number;
  height: number;
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
    - `width`: render target width. If omitted, the natural width
      of the SVG is used.
    - `height`: render target height. If omitted, the natural
      height of the SVG is used.
- Output: a `Promise<RgbaResult>` where:
  - `pixels` is the raw RGBA buffer (`R, G, B, A, ...`).
  - `width` and `height` are the actual rendered dimensions, after
    any options are applied.
- Errors: rejected if the input `usvg` bytes are not a valid
  postcard-encoded `usvg::Tree`, or if the requested width/height
  are not positive integers.

### 4.3 Things that are explicitly *not* in the API

For both functions:

- No `format` option (`'binary' | 'json' | ...`). The format is
  `postcard`. Always.
- No `background` / `backgroundColor` parameter. There is no
  background in the output; the canvas is transparent.
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
- No `fitTo` / `zoom` semantics. `width` and `height` are
  pixel-exact output dimensions.

### 4.4 Things that are explicitly *not yet* in scope

The following are intentionally deferred:

- A "convenience" PNG-encoding helper that turns `RgbaResult.pixels`
  into a PNG file. If added, it would be a *third* subpath
  (`./png`) and a *third* Wasm build, with the PNG encoder as an
  opt-in dependency. It is not this package.
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

### 5.3 Determinism caveat

`postcard` does not, by spec, guarantee deterministic encoding.
`svg2ui8a` relies on `usvg::Tree`'s structure being
**sufficiently ordered** that `postcard` produces the same bytes
across runs. The agent should not "fix" this by re-ordering fields
or by switching formats; if a determinism bug is found, escalate
per `./AGENTS.md` §10.

The intended escape hatch for format changes is the version prefix
in the cache key (see §6), not a serialization rewrite.

---

## 6. Cache key guidance (for the consumer, not for this package)

`svg2ui8a` itself does not produce a cache key. The recommended
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

The following are explicitly *not* goals of `svg2ui8a`, and the
agent must not be tempted to add them:

- General-purpose SVG manipulation. This is not an SVG editor.
- A unified "SVG toolkit". This is two functions.
- Drop-in compatibility with `@resvg/resvg-js` or `svg2png-wasm`.
  Those libraries solve different problems; `svg2ui8a` solves a
  smaller one, deliberately.
- Speed of a single render at the cost of consumer-side complexity.
  `svg2ui8a` optimizes for the cache-hit case (no SVG parse, no
  format conversion), not for the cache-miss case.
- Supporting legacy bundlers. Deno and modern browser bundlers
  only.
- Producing anything other than `Uint8Array` for either function.
  If you find yourself wanting to return a `Buffer`, a `Blob`, a
  `string`, a `Stream`, a `Promise<Read>`, or anything else, you
  are out of scope.
