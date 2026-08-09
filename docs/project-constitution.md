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
- Output: a `Promise<Uint8Array>` of a **CBOR-encoded payload**
  representing the parsed-and-normalized SVG.
- Purpose: produce a stable, content-addressable, opaque handle
  to the normalized SVG. Hash the bytes. Cache the bytes. Ship the
  bytes.

The `Uint8Array` is a self-contained, opaque blob. Same SVG input
→ same bytes (modulo a package-version prefix; see §6).

### 2.2 `usvg2rgba` — the consumer

```ts
// Subpath:  jsr:@tksh/svg2ui8a/rgba
// Function: usvg2rgba(usvg: Uint8Array, options?: Usvg2RgbaOptions): Promise<RgbaResult>
```

- Input:
  - `usvg`: a `Uint8Array` produced by `svg2usvg` (or by a
    compatible producer). Must be a valid CBOR-encoded payload of
    the package's intermediate representation.
  - `options`: optional.
- Output: a `Promise<RgbaResult>`, where `RgbaResult` carries both
  the pixel data and the metadata a downstream consumer needs to
  know what to do with it.
- Purpose: turn the content-addressable handle into RGBA bytes
  that a canvas, an image pipeline, or a downstream encoder
  (PNG, WebP, etc.) can consume.

The two functions are intentionally **separate entry points**. A
consumer who only wants to hash SVGs imports `./usvg` and never
loads the rasterizer. A consumer who already has a cached
`usvg` payload and wants pixels imports `./rgba` and never runs
the SVG parser. A consumer who wants both imports both, but the
two Wasm artifacts are loaded independently.

### 2.3 What "intermediate representation" means

The exact format of the bytes that flow between `svg2usvg` and
`usvg2rgba` is a **design decision left to the implementation**,
within the constraints of this constitution. The intermediate must:

- Be encoded as **CBOR** (RFC 8949).
- Carry enough information for `usvg2rgba` to render the same visual
  result that the original SVG would have produced.
- Be the **same format** on both ends — `svg2usvg` writes it,
  `usvg2rgba` reads it. There is no other producer or consumer in
  the package.
- Be **deterministic** for a given SVG input (modulo the
  package-version prefix in §6), so that the bytes can be hashed
  to produce a stable cache key.

The implementation is free to choose the inner structure (a
self-designed DTO, a serialization of `usvg::Tree` if the upstream
crate supports it, or any other representation) as long as the
constraints above hold. The implementation plan must document
which choice was made and why.

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
  but may not render meaningfully downstream. This is the
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

### 3.7 CBOR is the serialization format

The serialization format is **CBOR (RFC 8949)**. It is not
negotiable. Do not introduce `postcard`, MessagePack, bincode,
JSON, or any other format.

CBOR was chosen for the package because:

- It is an **IETF standard** with a stable, versioned spec.
- It is **CDDL-documentable** (RFC 8610), so the format can be
  formally defined alongside the package.
- It has a **`.cbor` extension** and a content-type, so the
  payload is recognizable as a first-class artifact, not a
  crate-internal blob.
- It is **independent of any one Rust crate**: a future migration
  to a different language or crate does not require a new
  encoding.

If a future task needs to ship a *different* format (e.g. for
performance), that task must explicitly amend this section. The
default is CBOR.

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

The API surface is **specified by shape, not by detail**. The
implementer chooses the inner types, the inner option names, the
inner error type. What is fixed is the *boundary*.

### 4.1 `svg2usvg`

```ts
// jsr:@tksh/svg2ui8a/usvg
export function svg2usvg(svg: string): Promise<Uint8Array>;
```

- Input: an SVG string. No options, no second argument.
- Output: a `Promise<Uint8Array>` of the CBOR-encoded intermediate.
- Errors: rejected with an error if the SVG cannot be parsed.
  Error type is whatever `wasm-bindgen` produces; consumers are
  expected to surface it.

### 4.2 `usvg2rgba`

```ts
// jsr:@tksh/svg2ui8a/rgba
//
// The exact shape of `Usvg2RgbaOptions` and `RgbaResult` is an
// implementation choice. The shape below is a *reference shape*,
// not a contract. The implementation may add or rename fields,
// provided the boundary in §2.2 is preserved.

export interface Usvg2RgbaOptions {
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

export function usvg2rgba(
  usvg: Uint8Array,
  options?: Usvg2RgbaOptions,
): Promise<RgbaResult>;
```

- Input:
  - `usvg`: a `Uint8Array` produced by `svg2usvg` (or by a
    compatible producer). Must be a valid CBOR-encoded payload.
  - `options`: optional, see above.
- Output: a `Promise<RgbaResult>`, see above.
- Errors: rejected if the input is not a valid CBOR-encoded
  payload, if the requested dimensions are not positive integers,
  if the SVG's natural size is zero in either dimension, or if
  the Wasm linear memory is exhausted.

### 4.3 Sizing

If both `width` and `height` are omitted, the natural SVG size is
used. If only one is provided, the other is taken from the natural
SVG size — the output is exactly `width × height`, with
non-uniform scaling if the aspect ratio of `(width, height)` does
not match the natural aspect ratio. **This is a deliberate design
choice, not a bug.** It mirrors `resvg`'s default behavior.

The implementation documents the exact natural-size handling
(integer, non-zero, etc.) in its test suite.

### 4.4 Things that are explicitly *not* in the API

For both functions:

- No `format` option (`'binary' | 'cbor' | 'json' | ...`). The
  format is **CBOR**. Always.
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
  live in the intermediate.

For `usvg2rgba` only:

- No `shapeRendering` / `textRendering` / `imageRendering` knobs.
  Defaults from `resvg` are used.

### 4.5 Things that are explicitly *not yet* in scope

The following are intentionally deferred:

- A "convenience" PNG-encoding helper that turns
  `RgbaResult.pixels` into a PNG file. If added, it would be a
  *third* subpath (`./png`) and a *third* Wasm build.
- WebP encoding. Same story.
- A "convenience" wrapper that takes an SVG string and returns
  RGBA in one call (i.e. `svg2usvg` + `usvg2rgba` chained). If
  added, it would be a *fourth* subpath.
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

The `Uint8Array` returned by `svg2usvg` is a **CBOR-encoded
payload** of the package's intermediate representation. It is:

- Self-contained, opaque from the consumer's perspective.
- Identical byte-for-byte when produced from visually equivalent
  SVGs. This is the property that makes it useful as a cache key
  input.
- Not a `(intermediate, options)` pair; the natural dimensions
  are encoded inside the intermediate, and a consumer that wants
  a different size passes `width` / `height` to `usvg2rgba`, not
  to `svg2usvg`.

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
- The default alpha mode returns **straight (non-premultiplied)**
  channels (a 50%-opaque red pixel is `(255, 0, 0, 128)`, not
  `(128, 0, 0, 128)`). When the consumer requests the
  `tiny-skia`-as-is mode, channels are premultiplied by alpha.

### 5.3 Determinism caveat

CBOR with the standard encoder is not guaranteed to produce
byte-identical output for semantically-equal inputs. The
implementation must verify (and the test suite must assert) that
the package's CBOR encoding is deterministic for a given SVG
input. If the implementation cannot achieve this with the chosen
crate, the implementation must switch to a deterministic-encoding
profile (e.g. **dCBOR** — "deterministic CBOR") and document the
switch. Switching to dCBOR does not require a constitution
amendment, because dCBOR is a profile of CBOR; the package is
still "CBOR" by the rule in §3.7.

---

## 6. Cache key guidance (for the consumer, not for this package)

The package itself does not produce a cache key. The recommended
pattern, documented here so the agent does not invent something
different:

```
cacheKey = "<package-version>-<cbor-encoder-version>-" + hex(sha256(bytes))
```

The prefix is the consumer's responsibility. Including the
package version and the CBOR-encoder version ensures that any
change in the serialization pipeline (a crate upgrade, a
package-version bump, a change in the inner DTO) automatically
invalidates all existing cache entries without code-level
coordination.

---

## 7. Versioning

The package follows **strict semver**:

- Patches: Wasm runtime tweaks that do not change the output bytes
  for any input.
- Minors: additions to the API surface. Output bytes may change.
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
