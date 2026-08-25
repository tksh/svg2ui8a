# svg2ui8a

SVG → `Uint8Array` and back: normalized SVG bytes and raw RGBA pixels from Wasm.
Two functions, two independently loadable Wasm modules.

[![JSR](https://jsr.io/badges/@tksh/svg2ui8a)](https://jsr.io/@tksh/svg2ui8a)

Published on JSR as [`@tksh/svg2ui8a`](https://jsr.io/@tksh/svg2ui8a). The
package follows semver; as a pre-1.0 package, `0.x` minor releases may contain
breaking changes (see `CHANGELOG.md`). See the version on the JSR badge.

## Overview

- **`svg2rgba(svg, options?)`** — one-shot SVG string in, raw RGBA pixels out.
  Whatever feature-disabled `usvg`/`resvg` support, minus `<text>` and
  `<image>`. No intermediate payload.
- **`svg2usvg(svg)`** — SVG string in, normalized usvg XML bytes (`Uint8Array`,
  UTF-8) out. Deterministic for a given input — hash it, cache it, or feed it to
  downstream tooling. Standard SVG XML, not a proprietary format.

Each function lives in its own subpath with its own Wasm artifact
(`assets/svg2rgba_bg.wasm`, `assets/svg2usvg_bg.wasm`). A consumer who only
needs pixels loads the rasterizer only; a consumer who only needs normalized SVG
loads the normalizer only.

## Installation / Import

Requires Deno (and a modern browser bundler for browser use). No `npm:`
specifier is used in the package itself.

```ts
// All functions (convenience re-export)
import { svg2rgba, svg2usvg } from "jsr:@tksh/svg2ui8a";

// Pixels only — loads only the rasterizer Wasm
import { svg2rgba } from "jsr:@tksh/svg2ui8a/svg2rgba";

// Normalized SVG only — loads only the normalizer Wasm
import { svg2usvg } from "jsr:@tksh/svg2ui8a/svg2usvg";
```

## Quick example

```ts
import { svg2rgba } from "jsr:@tksh/svg2ui8a/svg2rgba";
import { svg2usvg } from "jsr:@tksh/svg2ui8a/svg2usvg";

const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10">
  <rect width="10" height="10" fill="#ff0000"/>
</svg>`;

// One-shot: SVG string → RGBA pixels
const direct = await svg2rgba(svg);
console.log(direct.width, direct.height); // 10 10

// Normalize: SVG → usvg XML bytes → string
const usvgBytes = await svg2usvg(svg);
const usvgText = new TextDecoder().decode(usvgBytes);
console.log(usvgText.slice(0, 32)); // <svg ...

// Usvg bytes are valid SVG — they render identically
const viaUsvg = await svg2rgba(usvgText);
console.log(viaUsvg.pixels.length === direct.pixels.length); // true (for this input)

// Pixels with explicit size
const scaled = await svg2rgba(svg, { width: 20, height: 10 });
console.log(scaled.width, scaled.height); // 20 10
```

## API

```ts
// jsr:@tksh/svg2ui8a/svg2usvg
export function svg2usvg(svg: string): Promise<Uint8Array>;
// Rejects malformed SVG and <text>/<image> content. Returns UTF-8 bytes
// of the normalized usvg XML (default XmlOptions, no pretty-print).

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

export interface RgbaResult {
  width: number;
  height: number;
  alphaMode: string;
  pixels: Uint8Array; // length === width * height * 4, row-major RGBA
}
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
  dropped. Fonts are postponed (see `docs/project-constitution.md` §3.1), not
  permanently forbidden.
- Decode or render raster images — `<image>` is rejected.
- Encode PNG, WebP, JPEG, or any image file — output is raw `RgbaResult.pixels`
  only; consumers encode with their own library.
- Expose bounding boxes (postponed, see §3.2).

These exclusions keep the Wasm artifacts small and independently loadable.

## Format note

The `Uint8Array` produced by `svg2usvg` is the UTF-8 encoding of the normalized
SVG XML produced by `usvg` with default `XmlOptions` (minimal, deterministic).
It is standard SVG XML, not a proprietary binary format. Historical CBOR
intermediates (`0.1.x`–`0.2.0`, `cbor-core`) were removed in `0.3.0`; see
`CHANGELOG.md` `0.3.0` and `docs/archive/`.

## License

MIT — see [LICENSE](./LICENSE).
