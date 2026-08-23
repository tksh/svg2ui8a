# svg2ui8a

SVG string → versioned CBOR (`Uint8Array`) → RGBA pixels (`Uint8Array`), via two
independent Wasm modules.

[![JSR](https://jsr.io/badges/@tksh/svg2ui8a)](https://jsr.io/@tksh/svg2ui8a)

Published on JSR as [`@tksh/svg2ui8a`](https://jsr.io/@tksh/svg2ui8a). The
package follows semver and is currently pre-1.0 — the API may still change
before 1.0. See the version shown on the JSR badge and in `jsr.json`.

## Overview

`svg2ui8a` provides two functions:

- `svg2usvg(svg: string): Promise<Uint8Array>` — parses a supported subset of
  SVG and encodes it as a versioned, canonical CBOR payload. The result is
  deterministic for a given input and can be hashed for caching or written to a
  `.cbor` file.
- `usvg2rgba(usvg: Uint8Array, options?): Promise<RgbaResult>` — decodes that
  payload and rasterizes it to raw RGBA bytes.

Each function lives in its own subpath and its own Wasm artifact
(`assets/svg2usvg_bg.wasm` and `assets/usvg2rgba_bg.wasm`). Importing
`jsr:@tksh/svg2ui8a/usvg` loads only the parser/encoder; importing
`jsr:@tksh/svg2ui8a/rgba` loads only the rasterizer. Consumers that only need
cache keys never pay for the rasterizer, and hot-path consumers that already
have a cached intermediate never pay for the parser.

The supported SVG subset is intentionally small — no text, no fonts, no raster
images, no filters or external resources — which keeps the CBOR format
deterministic, portable, and hashable. The exclusions are part of the design,
not oversights.

## Installation / Import

Requires Deno (and a modern browser bundler for browser use). No `npm:`
specifier is used in the package itself.

```ts
// Both functions (convenience re-export)
import { svg2usvg, usvg2rgba } from "jsr:@tksh/svg2ui8a";

// Cache-key only — loads only the usvg Wasm
import { svg2usvg } from "jsr:@tksh/svg2ui8a/usvg";

// Already have a cached payload — loads only the rgba Wasm
import { usvg2rgba } from "jsr:@tksh/svg2ui8a/rgba";
```

## Quick example

```ts
import { svg2usvg } from "jsr:@tksh/svg2ui8a/usvg";
import { usvg2rgba } from "jsr:@tksh/svg2ui8a/rgba";

const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10">
  <rect width="10" height="10" fill="#ff0000"/>
</svg>`;

// SVG → CBOR bytes
const usvgBytes = await svg2usvg(svg);

// Usable as a file — write and read back unchanged
await Deno.writeFile("example.cbor", usvgBytes);
const fileBytes = await Deno.readFile("example.cbor");

// CBOR → RGBA (natural size, then explicit size)
const natural = await usvg2rgba(fileBytes);
console.log(natural.width, natural.height, natural.alphaMode); // 10 10 "straight"
console.log(natural.pixels.length); // 10 * 10 * 4

const scaled = await usvg2rgba(fileBytes, { width: 20, height: 10 });
console.log(scaled.width, scaled.height); // 20 10
console.log(scaled.pixels.length); // 20 * 10 * 4
```

The intermediate is validated by shape, not provenance — any `Uint8Array` that
satisfies the canonical-CBOR envelope is accepted by `usvg2rgba`, whether it
came from `svg2usvg` or from a `.cbor` file.

## API

```ts
// jsr:@tksh/svg2ui8a/usvg
export function svg2usvg(svg: string): Promise<Uint8Array>;
// Rejects if parsing fails or the SVG uses unsupported content (<text>, <image>).

// jsr:@tksh/svg2ui8a/rgba
export interface Usvg2RgbaOptions {
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

export function usvg2rgba(
  usvg: Uint8Array,
  options?: Usvg2RgbaOptions,
): Promise<RgbaResult>;
// Rejects if the payload is non-canonical CBOR, has a wrong identifier/version,
// malformed DTO, or invalid dimensions.
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

These exclusions are hard constraints that keep the intermediate deterministic
and content-addressable, keep the two Wasm artifacts small and independently
loadable, and make caching correct without extra bookkeeping.

Other SVG features such as filters, masks, clip paths, patterns, and gradients
are not yet supported by the current version 1 DTO — they are skipped during
conversion rather than being a permanent design boundary, and could be added in
a future format version without violating the hard constraints.

## Format note

The `Uint8Array` produced by `svg2usvg` is a versioned, self-describing file
format: canonical CBOR (RFC 8949) with integer keys `0` (identifier
`"svg2ui8a/usvg"`), `1` (unsigned format version), and `2` (payload). Format
version `1` is the initial stable public contract — it is covered by semver
(incompatible schema changes require a new format version and a major release).

## License

MIT — see [LICENSE](./LICENSE).
