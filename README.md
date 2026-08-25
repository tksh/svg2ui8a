# svg2ui8a

SVG → `Uint8Array` and back: raw RGBA pixels from Wasm. Three functions, three
independently loadable Wasm modules.

[![JSR](https://jsr.io/badges/@tksh/svg2ui8a)](https://jsr.io/@tksh/svg2ui8a)

Published on JSR as [`@tksh/svg2ui8a`](https://jsr.io/@tksh/svg2ui8a). The
package follows semver; as a pre-1.0 package, `0.x` minor releases may contain
breaking changes (the `0.2.0` rename is one). See the version on the JSR badge.

## Overview

- **`svg2rgba(svg, options?)`** — general-purpose one-shot: an SVG string in,
  raw RGBA pixels out. Whatever feature-disabled `usvg`/`resvg` support, minus
  `<text>` and `<image>`. No intermediate payload, no caching contract.
- **`svg2stln(svg)`** — Straightlines-subset producer: parses the subset (see
  below) and encodes it as a versioned, canonical CBOR payload that is
  deterministic for a given input — hash it, cache it, write it to a `.cbor`
  file.
- **`stln2rgba(stln, options?)`** — Straightlines rasterizer: decodes that
  payload and renders it to raw RGBA bytes.

Each function lives in its own subpath with its own Wasm artifact
(`assets/svg2rgba_bg.wasm`, `assets/svg2stln_bg.wasm`,
`assets/stln2rgba_bg.wasm`). A consumer who only needs cache keys loads the
producer only; a hot-path consumer holding a cached payload loads the rasterizer
only; a consumer who just wants pixels now uses the one-shot path.

The Straightlines subset is intentionally small: two-point straight-line paths,
flat layer groups (`<g>`), solid-color fill and/or stroke with per-shape and
per-group opacity, and a required root-level `shape-rendering`
(`geometricPrecision` or `crispEdges`; `auto` is accepted as
`geometricPrecision`). Anything else — curves, arcs, nested groups, gradients,
patterns, clip-paths, masks, filters — is rejected by `svg2stln`, not silently
dropped.

## Installation / Import

Requires Deno (and a modern browser bundler for browser use). No `npm:`
specifier is used in the package itself.

```ts
// All functions (convenience re-export)
import { stln2rgba, svg2rgba, svg2stln } from "jsr:@tksh/svg2ui8a";

// One-shot only
import { svg2rgba } from "jsr:@tksh/svg2ui8a/svg2rgba";

// Cache-key only — loads only the producer Wasm
import { svg2stln } from "jsr:@tksh/svg2ui8a/svg2stln";

// Already have a cached payload — loads only the rasterizer Wasm
import { stln2rgba } from "jsr:@tksh/svg2ui8a/stln2rgba";
```

## Quick example

```ts
import { svg2rgba } from "jsr:@tksh/svg2ui8a/svg2rgba";
import { svg2stln } from "jsr:@tksh/svg2ui8a/svg2stln";
import { stln2rgba } from "jsr:@tksh/svg2ui8a/stln2rgba";

const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"
  shape-rendering="geometricPrecision">
  <path d="M 5 0 L 5 10" stroke="#ff0000" stroke-width="10" fill="none"/>
</svg>`;

// One-shot: SVG string → RGBA pixels
const direct = await svg2rgba(svg);
console.log(direct.width, direct.height); // 10 10

// Pipeline: SVG → CBOR bytes → usable as a file
const stlnBytes = await svg2stln(svg);
await Deno.writeFile("example.cbor", stlnBytes);
const fileBytes = await Deno.readFile("example.cbor");

// CBOR → RGBA (natural size, then explicit size)
const natural = await stln2rgba(fileBytes);
console.log(natural.alphaMode, natural.pixels.length); // "straight" 400

const scaled = await stln2rgba(fileBytes, { width: 20, height: 10 });
console.log(scaled.width, scaled.height); // 20 10
```

The Straightlines intermediate is validated by shape, not provenance — any
`Uint8Array` satisfying the canonical-CBOR envelope is accepted by `stln2rgba`,
whether it came from `svg2stln` or from a `.cbor` file.

## API

```ts
// jsr:@tksh/svg2ui8a/svg2rgba
export interface Svg2RgbaOptions {
  width?: number;
  height?: number;
  alphaMode?: "straight" | "premultiplied";
}
export function svg2rgba(
  svg: string,
  options?: Svg2RgbaOptions,
): Promise<RgbaResult>;
// Rejects malformed SVG and <text>/<image> content.

// jsr:@tksh/svg2ui8a/svg2stln
export function svg2stln(svg: string): Promise<Uint8Array>;
// Rejects parsing failures, <text>/<image>, anything outside the
// Straightlines subset, and missing/unsupported root shape-rendering.

// jsr:@tksh/svg2ui8a/stln2rgba
export interface Stln2RgbaOptions {
  width?: number;
  height?: number;
  alphaMode?: "straight" | "premultiplied";
}

export interface RgbaResult {
  width: number;
  height: number;
  alphaMode: string;
  pixels: Uint8Array; // length === width * height * 4, row-major RGBA
}

export function stln2rgba(
  stln: Uint8Array,
  options?: Stln2RgbaOptions,
): Promise<RgbaResult>;
// Rejects non-canonical CBOR, wrong identifier/version, malformed DTO,
// or invalid dimensions.
```

**Sizing:** both `width` and `height` omitted → natural SVG size; one set → the
other is taken from the natural size; both set → exact `width × height` with
independent scaling. Pixels are zero-initialized; the default `alphaMode` is
`"straight"` (non-premultiplied, e.g. 50%-opaque red is `255, 0, 0, 128`).
Passing `alphaMode: "premultiplied"` returns the `tiny-skia` as-is value
(`128, 0, 0, 128` for the same input).

## Design constraints

This package deliberately does not:

- Render text or handle fonts — SVG `<text>` is rejected rather than silently
  dropped.
- Decode or render raster images — `<image>` is rejected.
- Encode PNG, WebP, JPEG, or any image file — output is raw `RgbaResult.pixels`
  only; consumers encode with their own PNG/WebP library.
- Expose bounding boxes.

These exclusions are hard constraints across all three entry points. They keep
the Straightlines intermediate deterministic and content-addressable and keep
the Wasm artifacts small and independently loadable.

## Format note

The `Uint8Array` produced by `svg2stln` is a versioned, self-describing file
format: canonical CBOR (RFC 8949) with integer keys `0` (identifier
`"svg2ui8a/straightlines"`), `1` (unsigned format version), and `2` (payload).
Format version `1` under this identifier is the current contract. The earlier
identifier `"svg2ui8a/usvg"` (package `0.1.x`) was retired in `0.2.0` with no
compatibility path, per the pre-1.0 breaking-change convention recorded in the
CHANGELOG.

## License

MIT — see [LICENSE](./LICENSE).
