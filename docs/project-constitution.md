# Project Constitution: `@tksh/svg2ui8a`

This file is the **second** document an AI agent reads (after `AGENTS.md`). It
defines what the package _is_, what it _is not_, and the non-negotiable
decisions that govern its design.

The human owns this file. If the agent believes something here is wrong, the
agent must surface the issue, not rewrite the file.

---

## 1. Mission

`@tksh/svg2ui8a` is a JSR-published package that turns SVG strings into
**`Uint8Array`** representations — and, in the other direction, turns SVG
strings into **RGBA pixel buffers** that are also `Uint8Array`. **All payloads
in this package are `Uint8Array`.** Function return types may be `Uint8Array` or
small wrapper objects whose `Uint8Array` fields are the actual data; the package
never produces SVG strings, PNG bytes, JPEG bytes, or other file formats.

The package name reflects this:

- `svg2ui8a` = "SVG to `Uint8Array`" — both as a function (SVG → `Uint8Array`)
  and as a value (usvg bytes) and as a return (rgba pixels). The two functions
  in the package are the two transitions across that boundary.
- The `ui8a` part is a deliberate shorthand for `Uint8Array`. It is the lingua
  franca of the package.

The package is intended for use as a JSR library, importable selectively by
subpath. It is a faithful wrapper around `linebender/resvg` (`usvg` + `resvg`

- `tiny-skia`); it adds no package-unique rendering features and does not chase
  every upstream release when not needed.

---

## 2. The two products

The package exposes **two** functions, in two subpath imports of one package.
They are not in the same file. They are not in the same Wasm binary. They are
imported separately and used separately.

### 2.1 `svg2usvg` — SVG → normalized usvg XML bytes

```ts
// Subpath:  jsr:@tksh/svg2ui8a/svg2usvg
// Function: svg2usvg(svg: string): Promise<Uint8Array>
```

- Input: an SVG string.
- Output: a `Promise<Uint8Array>` of the normalized usvg XML, UTF-8 encoded. The
  bytes are the result of parsing with feature-disabled `usvg` and serializing
  with `usvg`'s default `XmlOptions` (minimal, no pretty-print, no extra
  header).
- Purpose: produce a deterministic, normalized SVG representation that can be
  hashed, cached, or fed to downstream tooling without re-parsing the original
  source.

The `Uint8Array` is the UTF-8 encoding of a standard SVG XML document, not a
proprietary binary format.

### 2.2 `svg2rgba` — SVG → RGBA pixels (general-purpose one-shot)

```ts
// Subpath:  jsr:@tksh/svg2ui8a/svg2rgba
// Function: svg2rgba(svg: string, options?: Svg2RgbaOptions): Promise<RgbaResult>
```

- Input: an SVG string plus optional sizing/alpha options.
- Output: a `Promise<RgbaResult>` of raw RGBA pixels.
- Scope: whatever feature-disabled `usvg`/`resvg` support, minus `<text>` and
  `<image>` content — rectangles, curves, gradients, groups, and clips that
  `resvg` renders.
- No intermediate payload and no cacheable-bytes contract beyond "same input →
  same pixels".

The two functions are intentionally **separate entry points**. A consumer who
only wants normalized SVG imports `./svg2usvg` and never loads the rasterizer. A
consumer who only wants pixels imports `./svg2rgba`. The two Wasm artifacts are
loaded independently.

---

## 3. Hard constraints

These are not preferences. They are not negotiable during implementation. Any
plan that violates them must be rejected.

### 3.1 Fonts (text rendering) — postponed

Fonts are **postponed**, not permanently forbidden. The current package does not
render text:

- No font loading, no font directory scanning, no `defaultFontFamily`.
- No system font enumeration.
- `svg2usvg` and `svg2rgba` reject input containing `<text>` elements.

If needed in the future, text support will be added as a faithful
`linebender/resvg` wrapper, not as a package-unique feature, and the package
will not chase every upstream release when not needed. Any such addition
requires an approved plan — do not add it outside a plan.

### 3.2 BBox — postponed

The package does not expose bounding boxes. There is no `getBBox()`, no
`innerBBox()`, no `cropByBBox()`. Functions such as `getBBox()` originate from
`resvg-js`, not from upstream `linebender/resvg`.

BBox support is **postponed**, not permanently forbidden. If needed, it will be
added via `resvg`/`usvg` facilities under an approved plan.

### 3.3 No PNG / WebP / JPEG / any image format

The package does not encode final image files. There is no PNG output, no WebP
output, no JPEG output. The output of `svg2rgba` is **raw RGBA pixels as a
`Uint8Array`**, and nothing else.

This implies:

- No `png` crate, no `image` crate, no `@jsquash/webp`, no `cwebp`-style tooling
  in the Wasm builds.
- No PNG decoder on the JS side, ever.
- No raster-image decoding or rendering in any Wasm artifact. `svg2usvg` and
  `svg2rgba` reject SVG `<image>` content rather than emitting an unrenderable
  payload.

If a consumer needs PNG output, they pipe the RGBA bytes through their own
encoder. That encoder is the consumer's concern, not ours.

### 3.4 No PNG decoder on the JS side

Do not pull in `pngjs`, `sharp`, `@jsquash/png`, or any equivalent. The package
produces no PNG and consumes no PNG. If a consumer needs to read PNG dimensions,
they use their own helper, in their own code, in their own repo.

### 3.5 No native fallback

Do not introduce a native Rust binary, a Node.js addon, or any non-Wasm build
target. The package is Wasm-only.

### 3.6 No Node.js runtime

Do not target Node.js. The package is for Deno and the browser. Imports use
Deno-style specifiers (`jsr:`, `deno:`, or relative paths). `npm:` is not
allowed in package code (see `./AGENTS.md` §5.1).

### 3.7 No proprietary binary serialization

The package has **no custom binary serialization format**. `svg2usvg` output is
standard SVG XML bytes (UTF-8) produced by `usvg` with default `XmlOptions`; the
package does not define a proprietary DTO. Earlier versions used a
canonical-CBOR intermediate (`cbor-core`, envelope with identifier
`"svg2ui8a/straightlines"`); that format was removed in `0.3.0` because the
custom representation failed to make cache-hit renders cheaper (see
`CHANGELOG.md` `0.3.0` and `docs/archive/`). Do not reintroduce a proprietary
format without an approved plan.

### 3.8 No runtime CDN imports in the browser bundle

The browser bundle must contain every byte it needs to run. See `./AGENTS.md`
§5.2 for the full rule. The package's browser bundle must not import from
`https://deno.land/x/...`, `https://esm.sh/...`, or any other runtime CDN.

### 3.9 Independent Wasm artifacts per capability, not one

Each independently-loadable capability is its own Wasm binary served under its
own subpath import: `svg2rgba` (`assets/svg2rgba_bg.wasm`) and `svg2usvg`
(`assets/svg2usvg_bg.wasm`). They are not bundled together. They are not
feature-gated within a single Wasm artifact. The agent must not propose a
"single merged Wasm" approach, even if it would be smaller in the abstract,
because it would force every consumer to load code they do not use.

---

## 4. API surface

The API surface is **specified by shape, not by detail**. The implementer
chooses the inner types, the inner option names, the inner error type. What is
fixed is the _boundary_.

### 4.1 `svg2usvg`

```ts
// jsr:@tksh/svg2ui8a/svg2usvg
export function svg2usvg(svg: string): Promise<Uint8Array>;
```

- Input: an SVG string. No options, no second argument.
- Output: a `Promise<Uint8Array>` of the normalized usvg XML bytes (UTF-8).
- Errors: rejected if SVG parsing fails or if the SVG uses unsupported text or
  image content. Error type is whatever `wasm-bindgen` produces; consumers are
  expected to surface it.

### 4.2 `svg2rgba`

```ts
// jsr:@tksh/svg2ui8a/svg2rgba
//
// The exact shape of `Svg2RgbaOptions` and `RgbaResult` is an
// implementation choice. The shape below is a *reference shape*,
// not a contract. The implementation may add or rename fields,
// provided the boundary in §2.2 is preserved.

export interface Svg2RgbaOptions {
  // Optional target dimensions. See §4.3.
  width?: number;
  height?: number;

  // Optional alpha handling. The implementation chooses the name
  // and the value set; the contract is that the default returns
  // straight (non-premultiplied) RGBA, and an option exists to
  // request the bytes as `tiny-skia` produced them (premultiplied).
  // A string literal enum ("straight" | "premultiplied") is
  // preferred over a boolean, so future modes can be added
  // without an API break.
  // alphaMode?: "straight" | "premultiplied";
}

export interface RgbaResult {
  // RGBA pixels. One pixel per 4 bytes. Total length is
  // width * height * 4. Row-major. See §5.2.
  pixels: Uint8Array;

  // Actual rendered dimensions after any options were applied.
  width: number;
  height: number;

  // Natural SVG dimensions before output sizing, in SVG user units. These may
  // be fractional.
  naturalWidth: number;
  naturalHeight: number;

  // The alpha mode of the returned pixels. Must reflect the option
  // that produced them (or the default).
  // alphaMode: "straight" | "premultiplied";
}

export function svg2rgba(
  svg: string,
  options?: Svg2RgbaOptions,
): Promise<RgbaResult>;

// jsr:@tksh/svg2ui8a/svg2usvg — see §2.1:
//
// export function svg2usvg(svg: string): Promise<Uint8Array>;
```

- Input: an SVG string plus optional sizing/alpha options.
- Output: a `Promise<RgbaResult>`.
- Errors: rejected if SVG parsing fails, if the SVG uses unsupported text or
  image content, if dimensions are invalid, if the natural size is zero, or if
  Wasm linear memory is exhausted.

### 4.3 Sizing

If both `width` and `height` are omitted, the natural SVG size is converted to
integer pixels by rounding each dimension to the nearest pixel. If only one is
provided, the other is calculated from the natural SVG aspect ratio and rounded
to the nearest pixel. If both are provided, the output is exactly
`width × height`, with non-uniform scaling if the requested aspect ratio does
not match the natural aspect ratio.

Natural SVG dimensions remain fractional during aspect-ratio calculations; they
are converted to integer pixels only when the output pixmap dimensions are
chosen. The implementation documents the exact non-zero and rounding behavior in
its test suite.

### 4.4 Things that are explicitly _not_ in the API

For both functions:

- No `format` option (`'binary' | 'cbor' | 'json' | ...`). There is no
  proprietary format. `svg2usvg` returns standard SVG XML bytes; `svg2rgba`
  returns RGBA.
- No `background` / `backgroundColor` parameter. There is no background in the
  output; the canvas is fully transparent outside the painted shapes.
- No `dispose` method on the module. The Wasm instances are stateless and
  managed by the runtime.
- No synchronous variant. The Wasm calls are `async`. Always.
- No PNG-encoding helper. The whole point is to avoid file-format encoding.

For `svg2usvg` only:

- No `width` / `height` / `scale` parameters.

For `svg2rgba` only:

- No `shapeRendering` / `textRendering` / `imageRendering` knobs. Defaults from
  `resvg` are used.

### 4.5 Things that are explicitly _not yet_ in scope

The following are intentionally deferred:

- A "convenience" PNG-encoding helper that turns `RgbaResult.pixels` into a PNG
  file. If added, it would be a _third_ subpath (`./png`) and a _third_ Wasm
  build.
- WebP encoding. Same story.
- A structured-object input accepted by a new function this package would ship.
- Fonts and BBox (see §3.1/§3.2).

The agent must not implement any of these as part of an `svg2ui8a` task. If the
human asks for them, the human will create a separate task and a separate plan.

---

## 5. What the output bytes mean

### 5.1 `svg2usvg` output

The `Uint8Array` returned by `svg2usvg` is the **UTF-8 encoding of the
normalized SVG XML** produced by `usvg` with default `XmlOptions` (minimal,
deterministic, no pretty-print). It is:

- Valid SVG XML that re-parses with `usvg::Tree::from_str`.
- Deterministic for a given supported SVG input (same input → same bytes).
- Standard SVG, not a proprietary format.

### 5.2 `svg2rgba` output

The `Uint8Array` returned in `RgbaResult.pixels` is:

- Raw RGBA pixels, packed `R, G, B, A, R, G, B, A, ...`.
- One pixel per 4 bytes.
- Total length: `width * height * 4`.
- Row-major: the first `width * 4` bytes are the first row (top-to-bottom), the
  next are the second row, etc.
- **No** PNG header, **no** EXIF, **no** compression. Pure pixels.
- **Fully zero-initialized** before the SVG is drawn. Any pixel not painted by
  the SVG has zero alpha (and zero RGB).
- The default alpha mode returns **straight (non-premultiplied)** channels (a
  50%-opaque red pixel is `(255, 0, 0, 128)`, not `(128, 0, 0, 128)`). When the
  consumer requests the `tiny-skia`-as-is mode, channels are premultiplied by
  alpha.

### 5.3 Determinism guarantee

Both functions are deterministic for supported inputs: the same SVG string
produces byte-identical `svg2usvg` output and byte-identical `svg2rgba` pixels
across consecutive calls. The test suite asserts this.

---

## 6. Versioning

The package follows **strict semver**:

- Patches: Wasm runtime tweaks that preserve output bytes for any input.
- Minors: additions to the API surface or support for new features that are
  backward-compatible.
- Majors: incompatible changes to an existing API or removals from the API
  surface.

As a pre-1.0 package, `0.x` minor releases may contain breaking changes (see
`CHANGELOG.md` `0.3.0` and `0.2.0`).

---

## 7. Anti-goals

The following are explicitly _not_ goals of the package, and the agent must not
be tempted to add them:

- General-purpose SVG manipulation. This is not an SVG editor.
- A unified "SVG toolkit". This is two functions.
- Drop-in compatibility with `@resvg/resvg-js` or `svg2png-wasm`. Those
  libraries solve different problems; this package solves a smaller one,
  deliberately.
- Supporting legacy bundlers. Deno and modern browser bundlers only.
- Producing anything other than `Uint8Array` payloads (SVG strings, PNG bytes,
  JPEG bytes, WebP bytes, etc.). Return-type wrappers (`RgbaResult`) are allowed
  because their `pixels` field is `Uint8Array`; producing an SVG string or a PNG
  byte array is not.

---

## 8. History

`0.1.x`–`0.2.0` used a proprietary Straightlines / CBOR intermediate
(`intermediate` crate, `svg2stln`/`stln2rgba`, `cbor-core`, envelope identifier
`"svg2ui8a/straightlines"`). That strategy failed to make cache-hit renders
cheaper — `usvg::Tree` can only be reconstituted by re-parsing SVG XML, so the
intermediate's re-serialization round-trip could not bypass the re-parse it was
designed to avoid. The format and its crates were removed in `0.3.0`; the
historical plan documents are archived in `docs/archive/`. The current design
(`svg2usvg` as normalized XML bytes + `svg2rgba` one-shot) replaces it as a
faithful `resvg` wrapper.
