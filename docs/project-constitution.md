# Project Constitution: `svg2ui8a`

This file is the **second** document an AI agent reads (after
`AGENTS.md`). It defines what `svg2ui8a` *is*, what it *is not*, and
the non-negotiable decisions that govern its design.

The human owns this file. If the agent believes something here is
wrong, the agent must surface the issue, not rewrite the file.

---

## 1. Mission

`svg2ui8a` is a JSR-published package that turns SVG strings into
**`Uint8Array`** representations. The single product is a typed
binary handle to a normalized SVG; the rest of the rendering pipeline
(rasterization, PNG encoding) is a separate concern that may be layered
on top by the consumer.

The package is intended for use in:

- A Deno-first web application, both in the browser (bundled by
  `deno bundle`) and on Deno Deploy.
- Future consumers who want a portable, content-addressable form of
  an SVG they can hash, cache, or ship across a network.

---

## 2. The one product

`svg2ui8a` exposes exactly one operation:

> Given an SVG string, return a `Uint8Array` that is the
> **postcard-serialized** `usvg::Tree` produced by parsing that SVG.

The `Uint8Array` is a self-contained, content-addressable handle:

- The same SVG string always produces the same bytes (modulo
  `usvg`/`postcard` version, which must be tracked separately — see
  §6).
- The bytes can be hashed to produce a stable cache key.
- The bytes can be stored, transported, and later re-hydrated into a
  `usvg::Tree` by `usvg2rgba` (a future package, not this one).

That is the entire product surface.

---

## 3. Hard constraints

These are not preferences. They are not negotiable during
implementation. Any plan that violates them must be rejected.

### 3.1 No fonts

`svg2ui8a` is for the consumer's SVG pipeline that **does not render
text**. Fonts are out of scope:

- No font loading, no font directory scanning, no
  `defaultFontFamily`.
- No system font enumeration.
- If the input SVG contains `<text>` elements, they will be parsed
  by `usvg` but may not render meaningfully downstream. This is the
  caller's problem, not ours.

### 3.2 No BBox

`svg2ui8a` does not expose bounding boxes. There is no `getBBox()`,
no `innerBBox()`, no `cropByBBox()`. The output is the whole tree,
nothing more, nothing less.

### 3.3 No rasterization

`svg2ui8a` does not rasterize. It does not invoke `resvg` /
`tiny-skia`. It does not produce RGBA pixels. It does not produce
PNG bytes.

The Wasm binary built for this package must depend on `usvg` only.
It must not pull in `resvg`, `tiny-skia`, `png`, or any other
rasterization or encoding crate. (See
`./docs/system-architecture.md` §3 for why.)

### 3.4 No PNG decoder

Do not pull in `pngjs`, `sharp`, `@jsquash/png`, or any equivalent.
This package does not produce PNGs and does not read them. If a
consumer needs PNG output or dimensions, that is the consumer's
problem, addressed in a different package.

### 3.5 No native fallback

Do not introduce a native Rust binary, a Node.js addon, or any
non-Wasm build target. The package is Wasm-only.

### 3.6 No Node.js runtime

Do not target Node.js. The package is for Deno and the browser.
Imports must use Deno-style specifiers (`jsr:`, `npm:` via
Deno's resolver, or relative paths).

### 3.7 No custom serialization

The serialization format is `postcard`. It is not negotiable. Do
not introduce CBOR, MessagePack, bincode, JSON, or any other format.

`postcard` is the format because it is the smallest and fastest
option that requires zero new dependencies on the JS side (the JS
side never has to decode it; it only passes the bytes to a hashing
function or to a downstream consumer).

---

## 4. API surface

`svg2ui8a` exposes **one** function, and only one:

```ts
// svg2ui8a
export function svg2usvg(svg: string): Promise<Uint8Array>;
```

- Input: an SVG string. No options, no second argument.
- Output: a `Promise<Uint8Array>` of the postcard-encoded
  `usvg::Tree`.
- Errors: rejected with an error if the SVG cannot be parsed by
  `usvg`. Error type is whatever `wasm-bindgen` produces; consumers
  are expected to surface it.

That is all.

### 4.1 Things that are explicitly *not* in the API

- No `format` option (`'binary' | 'json' | ...`). The format is
  `postcard`. Always.
- No `width` / `height` / `scale` parameters. The output dimensions
  live in the tree.
- No `background` / `backgroundColor` parameter. There is no
  background in the output; that is a rendering concern.
- No `createSvg2usvg` factory. There is no per-instance state worth
  amortizing (no fonts, no options).
- No `dispose` method. The Wasm instance is stateless.
- No synchronous variant. The Wasm call is `async`. Always.
- No `toString()` / SVG-string output. The whole point is to *avoid*
  the SVG string.

### 4.2 Things that are explicitly *not yet* in scope

The following are intentionally deferred to a separate future
package, **not** bundled into this one:

- A `usvg2rgba` function that takes the `Uint8Array` and returns
  RGBA pixels. This would require a second, larger Wasm build.
- PNG encoding, IHDR-only size helpers, or any pixel-level
  post-processing.
- A `custom` variant that takes a structured JS object instead of
  an SVG string. (See `./AGENTS.md` §10: this is a design question
  the human has not yet answered.)

The agent must not implement any of these as part of an
`svg2ui8a` task. If the human asks for them, the human will create
a separate task and a separate plan.

---

## 5. What the output bytes mean

The `Uint8Array` returned by `svg2usvg` is:

- The Rust `usvg::Tree` (as produced by `usvg::Tree::from_str` with
  `usvg::Options::default()`), serialized via
  `postcard::to_stdvec`.
- A self-contained, opaque blob from the consumer's perspective.
  Consumers do not parse it.
- Identical byte-for-byte when produced from visually equivalent
  SVGs (i.e. SVGs that `usvg` normalizes to the same `Tree`).
  This is the property that makes it useful as a cache key input.

### 5.1 Determinism caveat

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
- Minors: additions to the API surface (none planned). Output bytes
  may change.
- Majors: changes to the output format itself, or removals from the
  API surface.

The output bytes are **not** part of the public API contract beyond
semver. The consumer is expected to use the version-prefixed cache
key from §6 to handle byte changes gracefully.

---

## 8. Anti-goals

The following are explicitly *not* goals of `svg2ui8a`, and the agent
must not be tempted to add them:

- Speed of *rasterization*. This package does not rasterize.
- General-purpose SVG manipulation. This is not an SVG editor.
- A unified "SVG toolkit". This is one function.
- Drop-in compatibility with `@resvg/resvg-js` or `svg2png-wasm`.
  Those libraries solve different problems; `svg2ui8a` solves a
  smaller one, deliberately.
- Supporting legacy bundlers. Deno and modern browser bundlers
  only.
