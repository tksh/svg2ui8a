# Project Constitution: `@tksh/svg2ui8a`

This file is the **second** document an AI agent reads (after `AGENTS.md`). It
defines what the package _is_, what it _is not_, and the non-negotiable
decisions that govern its design.

The human owns this file. If the agent believes something here is wrong, the
agent must surface the issue, not rewrite the file.

---

## 1. Mission

`@tksh/svg2ui8a` is a JSR-published package that turns SVG strings into
**`Uint8Array`** representations — and, in the other direction, turns those
`Uint8Array` representations back into **RGBA pixel buffers** that are also
`Uint8Array`. **All payloads in this package are `Uint8Array`.** Function return
types may be `Uint8Array` or small wrapper objects whose `Uint8Array` fields are
the actual data; the package never produces SVG strings, PNG bytes, JPEG bytes,
or other file formats.

The package name reflects this:

- `svg2ui8a` = "SVG to `Uint8Array`" — both as a function (SVG → `Uint8Array`)
  and as a value (`usvg` payload) and as a return (`rgba` payload). The two
  functions in the package are the two transitions across that boundary.
- The `ui8a` part is a deliberate shorthand for `Uint8Array`. It is the lingua
  franca of the package.

The package is intended for use as a JSR library, importable selectively by
subpath.

---

## 2. The two products

The package exposes **three** functions, in three subpath imports of one
package. They are not in the same file. They are not in the same Wasm binary.
They are imported separately and used separately.

### 2.1 `svg2stln` — the Straightlines producer

```ts
// Subpath:  jsr:@tksh/svg2ui8a/svg2stln
// Function: svg2stln(svg: string): Promise<Uint8Array>
```

- Input: an SVG string.
- Output: a `Promise<Uint8Array>` of a versioned **CBOR-encoded file payload**
  representing the supported Straightlines subset (see §2.3 and the rename-plan
  scope freeze: two-point straight-line paths, flat layer groups, solid-color
  paints, required root-level `shape-rendering`).
- Purpose: produce a stable, content-addressable intermediate that can be
  hashed, cached, shipped, written as a `.cbor` file, and later passed unchanged
  to `stln2rgba`.

The `Uint8Array` is a self-contained file payload. Same supported SVG input →
same bytes (modulo the format version; see §6).

### 2.2 `stln2rgba` — the Straightlines consumer

```ts
// Subpath:  jsr:@tksh/svg2ui8a/rgba
// Function: stln2rgba(stln: Uint8Array, options?: Stln2RgbaOptions): Promise<RgbaResult>
```

- Input:
  - `stln`: a `Uint8Array` produced by `svg2stln`, including bytes read from its
    `.cbor` file. It must be a valid, supported version of the package's
    canonical-CBOR intermediate.
  - `options`: optional.
- Output: a `Promise<RgbaResult>`, where `RgbaResult` carries both the pixel
  data and the metadata a downstream consumer needs to know what to do with it.
- Purpose: turn the content-addressable handle into RGBA bytes that a canvas, an
  image pipeline, or a downstream encoder (PNG, WebP, etc.) can consume.

The two Straightlines functions are intentionally **separate entry points**. A
consumer who only wants to hash SVGs imports `./svg2stln` and never loads the
rasterizer. A consumer who already has a cached payload and wants pixels imports
`./stln2rgba` and never runs the SVG parser. A consumer who wants both imports
both, but the two Wasm artifacts are loaded independently.

`stln2rgba` validates its input by **shape**, not by **provenance**. It has no
way to know, and does not care, whether a given `Uint8Array` was produced by
`svg2stln`, read back from a `.cbor` file that `svg2stln` once wrote, or
assembled by an entirely different producer. Any canonical-CBOR byte string that
satisfies the envelope and DTO rules in §2.3 is a valid `stln2rgba` input.
`svg2stln` is the only producer this package ships, but it is not the only
producer the format allows. This is a deliberate design property, not an
accident: it is what lets a future, independently specified format (see
`notes/straightlines-vision.md`) emit envelope-conformant bytes directly and
call `stln2rgba`, skipping SVG string generation entirely, without requiring any
change to this package. Building such a producer is out of scope for this
package (§4.5) and is not this package's concern — only staying faithful to the
envelope contract is.

### 2.4 `svg2rgba` — the general-purpose one-shot

```ts
// Subpath:  jsr:@tksh/svg2ui8a/svg2rgba
// Function: svg2rgba(svg: string, options?: Svg2RgbaOptions): Promise<RgbaResult>
```

- Input: an SVG string plus optional sizing/alpha options.
- Output: a `Promise<RgbaResult>` of raw RGBA pixels.
- Scope: whatever feature-disabled `usvg`/`resvg` support, minus `<text>` and
  `<image>` content — deliberately broader than the Straightlines subset.
  Rectangles, curves, gradients, and clips render here.
- No CBOR envelope exists on this path. There is no intermediate payload, no
  cacheable-bytes contract, and no hashing guarantee; determinism is asserted
  only as "same input → same pixels". This path does not use the `intermediate`
  crate at all.

### 2.3 What "intermediate representation" means

The intermediate is a versioned package file format, not a serialized upstream
Rust type. Its canonical-CBOR top-level map has integer keys:

- `0`: the literal text identifier `"svg2ui8a/straightlines"`.
- `1`: the unsigned format version; the initial version is `1`.
- `2`: the version-specific package DTO payload.

The DTO's individual drawing fields are implementation-defined, within these
constraints:

- Be encoded as canonical **CBOR** (RFC 8949).
- Carry enough information for `stln2rgba` to render the same visual result that
  the original SVG would have produced.
- Be the **same format** on both ends — `svg2stln` writes it and `stln2rgba`
  reads it, including when the bytes came from a `.cbor` file.
- Be **deterministic** for a given supported SVG input, so that the bytes can be
  hashed to produce a stable cache key.
- Be a package-owned DTO mapped to and from the supported `usvg::Tree` subset;
  it must not directly serialize `usvg::Tree`.
- Exclude text, raster images, BBoxes, animation state, and external resources.

---

## 3. Hard constraints

These are not preferences. They are not negotiable during implementation. Any
plan that violates them must be rejected.

### 3.1 No fonts

The package is for SVG pipelines that **do not render text**. Fonts are out of
scope:

- No font loading, no font directory scanning, no `defaultFontFamily`.
- No system font enumeration.
- `svg2stln` rejects input containing `<text>` elements; it must not emit a
  payload that requires font support.

### 3.2 No BBox

The package does not expose bounding boxes. There is no `getBBox()`, no
`innerBBox()`, no `cropByBBox()`. The `usvg` payload is the whole tree, nothing
more, nothing less.

### 3.3 No PNG / WebP / JPEG / any image format

The package does not encode final image files. There is no PNG output, no WebP
output, no JPEG output. The outputs of `stln2rgba` and `svg2rgba` are **raw RGBA
pixels as a `Uint8Array`**, and nothing else.

This implies:

- No `png` crate, no `image` crate, no `@jsquash/webp`, no `cwebp`-style tooling
  in the Wasm builds.
- No PNG decoder on the JS side, ever.
- No raster-image decoding or rendering in any Wasm artifact. `svg2stln` and
  `svg2rgba` rejects SVG `<image>` content rather than emitting an unrenderable
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

### 3.7 CBOR is the serialization format

The serialization format is **CBOR (RFC 8949)**, encoded and decoded exclusively
with Cargo package `cbor-core` version `0.10.1` (`cbor_core` in Rust source).
This is not negotiable.

`cbor_core` emits canonical CBOR and accepts only canonical CBOR package
intermediates; non-canonical encodings are rejected. The decoder also validates
the format identifier, format version, required fields, field types, numeric
ranges, and supported DTO variants before it creates a render tree.

CBOR was chosen for the package because:

- It is an **IETF standard** with a stable, versioned spec.
- It is **CDDL-documentable** (RFC 8610), so the format can be formally defined
  alongside the package.
- It has a **`.cbor` extension** and a content-type, so the payload is
  recognizable as a first-class artifact, not a crate-internal blob.
- It is **independent of any one Rust crate**: a future migration to a different
  language or crate does not require a new encoding, provided it preserves the
  canonical-CBOR contract.

If a future task needs to ship a _different_ format (e.g. for performance), that
task must explicitly amend this section. The default is CBOR.

### 3.8 No runtime CDN imports in the browser bundle

The browser bundle must contain every byte it needs to run. See `./AGENTS.md`
§5.2 for the full rule. The package's browser bundle must not import from
`https://deno.land/x/...`, `https://esm.sh/...`, or any other runtime CDN.

### 3.9 Independent Wasm artifacts per capability, not one

Each independently-loadable capability is its own Wasm binary served under its
own subpath import: `svg2rgba` (`assets/svg2rgba_bg.wasm`), `svg2stln`
(`assets/svg2stln_bg.wasm`), and `stln2rgba` (`assets/stln2rgba_bg.wasm`). They
are not bundled together. They are not feature-gated within a single Wasm
artifact. The agent must not propose a "single merged Wasm" approach, even if it
would be smaller in the abstract, because it would force every consumer to load
code they do not use.

---

## 4. API surface

The API surface is **specified by shape, not by detail**. The implementer
chooses the inner types, the inner option names, the inner error type. What is
fixed is the _boundary_.

### 4.1 `svg2stln`

```ts
// jsr:@tksh/svg2ui8a/svg2stln
export function svg2stln(svg: string): Promise<Uint8Array>;
```

- Input: an SVG string. No options, no second argument.
- Output: a `Promise<Uint8Array>` of the CBOR-encoded intermediate.
- Errors: rejected if SVG parsing fails; if the SVG uses unsupported text or
  image content; or if the document does not declare a supported root-level
  `shape-rendering` (`geometricPrecision` or `crispEdges` for every path; `auto`
  is accepted as `geometricPrecision`). Error type is whatever `wasm-bindgen`
  produces; consumers are expected to surface it.

### 4.2 `stln2rgba`

```ts
// jsr:@tksh/svg2ui8a/rgba
//
// The exact shape of `Stln2RgbaOptions` and `RgbaResult` is an
// implementation choice. The shape below is a *reference shape*,
// not a contract. The implementation may add or rename fields,
// provided the boundary in §2.2 is preserved.

export interface Stln2RgbaOptions {
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

  // The alpha mode of the returned pixels. Must reflect the option
  // that produced them (or the default).
  // alphaMode: "straight" | "premultiplied";
}

export function stln2rgba(
  stln: Uint8Array,
  options?: Stln2RgbaOptions,
): Promise<RgbaResult>;

// jsr:@tksh/svg2ui8a/svg2rgba — see §2.4; identical RgbaResult shape, but
// takes the SVG string directly:
//
// export function svg2rgba(
//   svg: string,
//   options?: Svg2RgbaOptions,
// ): Promise<RgbaResult>;
```

- Input:
  - `stln`: a `Uint8Array` produced by `svg2stln` or read from a compatible
    `.cbor` file. Must be a supported, canonical-CBOR package payload.
  - `options`: optional, see above.
- Output: a `Promise<RgbaResult>`, see above.
- Errors: rejected if the input is non-canonical CBOR, has an invalid identifier
  or DTO, uses an unsupported format version, requests invalid dimensions, has a
  zero natural size, or exhausts Wasm linear memory.

### 4.3 Sizing

If both `width` and `height` are omitted, the natural SVG size is used. If only
one is provided, the other is taken from the natural SVG size — the output is
exactly `width × height`, with non-uniform scaling if the aspect ratio of
`(width, height)` does not match the natural aspect ratio. **This is a
deliberate design choice, not a bug.** It mirrors `resvg`'s default behavior.

The implementation documents the exact natural-size handling (integer, non-zero,
etc.) in its test suite.

### 4.4 Things that are explicitly _not_ in the API

For both functions:

- No `format` option (`'binary' | 'cbor' | 'json' | ...`). The format is
  **CBOR**. Always.
- No `background` / `backgroundColor` parameter. There is no background in the
  output; the canvas is fully transparent outside the painted shapes.
- No `dispose` method on the module. The Wasm instances are stateless and
  managed by the runtime.
- No synchronous variant. The Wasm calls are `async`. Always.
- No `toString()` / SVG-string output anywhere. The whole point is to _avoid_
  the SVG string.

For `svg2stln` only:

- No `width` / `height` / `scale` parameters. The output dimensions live in the
  intermediate.

For `stln2rgba` and `svg2rgba` only:

- No `shapeRendering` / `textRendering` / `imageRendering` knobs. Defaults from
  `resvg` are used.

### 4.5 Things that are explicitly _not yet_ in scope

The following are intentionally deferred:

- A "convenience" PNG-encoding helper that turns `RgbaResult.pixels` into a PNG
  file. If added, it would be a _third_ subpath (`./png`) and a _third_ Wasm
  build.
- WebP encoding. Same story.
- ~~A "convenience" wrapper that takes an SVG string and returns RGBA in one
  call.~~ Now shipped as `svg2rgba` (§2.4) under the rename plan; this list
  keeps the entry for historical context only.
- A structured-object input (e.g. a JS object representing the vector graphic,
  instead of an SVG string) accepted by a **new function this package would
  ship**. This is a design question the human has not yet answered; see
  `./AGENTS.md` §10. It is unrelated to §2.2's point that `stln2rgba` already
  accepts any envelope-conformant `Uint8Array` regardless of producer; that is
  an existing property of the shipped API, not a deferred one.

The agent must not implement any of these as part of an `svg2ui8a` task. If the
human asks for them, the human will create a separate task and a separate plan.

---

## 5. What the output bytes mean

### 5.1 `svg2stln` output

The `Uint8Array` returned by `svg2stln` is a **versioned, canonical-CBOR file
payload** of the package's intermediate representation. It is:

- Self-contained and usable directly after being written to and read from a
  `.cbor` file.
- Identical byte-for-byte when produced from the same supported SVG input. This
  is the property that makes it useful as a cache key input; visual equivalence
  alone is not a guarantee.
- Not a `(intermediate, options)` pair; the natural dimensions are encoded
  inside the intermediate, and a consumer that wants a different size passes
  `width` / `height` to `stln2rgba`, not to `svg2stln`.

### 5.2 `stln2rgba` output

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

`cbor_core` emits canonical CBOR, so the implementation must verify (and the
test suite must assert) that the versioned DTO maps a given supported SVG input
to byte-identical output across consecutive calls. Canonical CBOR does not make
an invalid DTO valid; semantic validation is required on decode.

---

## 6. Cache key guidance (for the consumer, not for this package)

The package itself does not produce a cache key. The recommended pattern,
documented here so the agent does not invent something different:

```
cacheKey = "<format-id>-<format-version>-" + hex(sha256(bytes))
```

The prefix is the consumer's responsibility. Including the format identifier and
format version prevents a future incompatible schema from sharing cache entries
with version 1.

---

## 7. Versioning

The package follows **strict semver**:

- Patches: Wasm runtime tweaks that preserve supported format versions and their
  output bytes for any input.
- Minors: additions to the API surface or support for a new, backward-compatible
  format version; version 1 remains decodable.
- Majors: incompatible changes to an existing format version or removals from
  the API surface.

The envelope and every supported format version are part of the public file
format contract. An incompatible schema gets a new format version and is only
introduced in a major release.

---

## 8. Anti-goals

The following are explicitly _not_ goals of the package, and the agent must not
be tempted to add them:

- General-purpose SVG manipulation. This is not an SVG editor.
- A unified "SVG toolkit". This is two functions.
- Drop-in compatibility with `@resvg/resvg-js` or `svg2png-wasm`. Those
  libraries solve different problems; this package solves a smaller one,
  deliberately.
- Speed of a single render at the cost of consumer-side complexity. The package
  optimizes for the cache-hit case (no SVG parse, no format conversion), not for
  the cache-miss case.
- Supporting legacy bundlers. Deno and modern browser bundlers only.
- Producing anything other than `Uint8Array` payloads (SVG strings, PNG bytes,
  JPEG bytes, WebP bytes, etc.). Return-type wrappers (`RgbaResult`) are allowed
  because their `pixels` field is `Uint8Array`; producing an SVG string or a PNG
  byte array is not.
