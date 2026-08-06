# System Architecture: `svg2ui8a`

This file is the **third** document an AI agent reads (after
`AGENTS.md` and `docs/project-constitution.md`). It describes the
package layout, the data flow, and the Wasm boundary.

The human owns this file. If the agent believes something here is
wrong, the agent must surface the issue, not rewrite the file.

---

## 1. Package layout

`svg2ui8a` is a single JSR package. Internally it is a workspace
with two Rust crates and two TypeScript entry points:

```
svg2ui8a/
├── jsr.json
├── README.md
├── LICENSE
├── deno.json                    # Deno tasks, fmt config, lint config
├── src/
│   ├── mod.ts                   # Re-exports the two subpaths
│   ├── usvg.ts                  # TS wrapper for the usvg Wasm
│   └── rgba.ts                  # TS wrapper for the rgba Wasm
├── crates/
│   ├── svg2usvg/                # Wasm crate, usvg only
│   │   ├── Cargo.toml
│   │   ├── src/lib.rs
│   │   └── tests/
│   └── usvg2rgba/               # Wasm crate, usvg + resvg only
│       ├── Cargo.toml
│       ├── src/lib.rs
│       └── tests/
└── assets/
    ├── svg2usvg_bg.wasm         # Built artifact, vendored
    └── usvg2rgba_bg.wasm        # Built artifact, vendored
```

The two Rust crates are **fully independent**. They share no source
files, no test code, and no build script. The only thing they share
is the `usvg` upstream crate as a dependency, and that is a
zero-cost transitive link.

The two TS wrappers are also fully independent. They each import
their own Wasm binary, manage their own `init()` flag, and export
their own function.

---

## 2. JSR subpath exports

`svg2ui8a` exposes the following subpaths:

| Subpath         | What it loads                          | Wasm size |
|-----------------|----------------------------------------|-----------|
| `svg2ui8a`      | Re-exports both `./usvg` and `./rgba` | (sum)     |
| `svg2ui8a/usvg` | `svg2usvg` only                       | Small     |
| `svg2ui8a/rgba` | `usvg2rgba` only                      | Larger    |

Importing the root re-exports everything; importing a subpath
imports only that subpath's Wasm. The bundler is expected to
tree-shake whichever subpath is not used.

`jsr.json` example:

```json
{
  "name": "@your-org/svg2ui8a",
  "version": "0.1.0",
  "exports": {
    ".": "./src/mod.ts",
    "./usvg": "./src/usvg.ts",
    "./rgba": "./src/rgba.ts"
  }
}
```

`deno.json` example:

```json
{
  "tasks": {
    "build": "deno run -A scripts/build.ts",
    "test": "deno test -A",
    "fmt": "deno fmt",
    "lint": "deno lint",
    "check": "deno check src/**/*.ts"
  },
  "fmt": {
    "lineWidth": 100,
    "indentWidth": 2,
    "singleQuote": false,
    "semiColons": true
  },
  "lint": {
    "rules": {
      "tags": ["recommended"]
    }
  }
}
```

### 2.1 Consumer import examples

```ts
// Cache key only — load the usvg Wasm, do not load the rasterizer
import { svg2usvg } from "jsr:@your-org/svg2ui8a/usvg";

// Already have a cached usvg payload — load the rgba Wasm, do not
// load the SVG parser
import { usvg2rgba } from "jsr:@your-org/svg2ui8a/rgba";

// Both — load both Wasm artifacts
import { svg2usvg, usvg2rgba } from "jsr:@your-org/svg2ui8a";
```

---

## 3. The two Wasm builds

### 3.1 `svg2usvg` (the producer)

```toml
# crates/svg2usvg/Cargo.toml
[package]
name = "svg2usvg"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
usvg = "0.34"
postcard = "1.0"
wasm-bindgen = "0.2"
serde = { version = "1.0", features = ["derive"] }

[dev-dependencies]
wasm-bindgen-test = "0.3"
```

Dependencies: `usvg`, `postcard`, `wasm-bindgen`, `serde`. That is
all. No `resvg`, no `tiny-skia`, no `png`, no `image`.

**Why so small**: the work is just SVG parse (`usvg`) and Tree
serialize (`postcard`). No geometry walk, no pixel allocation, no
encoding.

### 3.2 `usvg2rgba` (the consumer)

```toml
# crates/usvg2rgba/Cargo.toml
[package]
name = "usvg2rgba"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
usvg = "0.34"
postcard = "1.0"
resvg = "0.34"
tiny-skia = "0.11"
wasm-bindgen = "0.2"
serde = { version = "1.0", features = ["derive"] }

[dev-dependencies]
wasm-bindgen-test = "0.3"
```

Dependencies include `resvg` and `tiny-skia` because the work is
**Tree → RGBA pixels**. There is no PNG encoder. The output is
raw RGBA, period.

### 3.3 Why two builds

The `usvg` and `rgba` builds serve different use cases with
different cost profiles:

- A consumer who only needs cache keys (a CDN, a KV layer, a
  deduplication pipeline) should never pay for `resvg` and
  `tiny-skia` to be loaded into memory. The `usvg` Wasm is
  roughly 2.5–3x smaller than the `rgba` Wasm.
- A consumer who already has a cached `usvg` payload (e.g. on a
  hot path where 99% of requests hit the cache) should never
  pay for the SVG parser. The `rgba` Wasm is meaningfully smaller
  and starts faster than the `usvg` Wasm.

A single Wasm with both functions would force every consumer to
load both code paths. That violates
`docs/project-constitution.md` §3.9.

---

## 4. The Rust APIs

### 4.1 `crates/svg2usvg/src/lib.rs`

```rust
use usvg::Tree;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn svg2usvg(svg: &str) -> Result<Vec<u8>, JsError> {
    let tree = Tree::from_str(svg, &usvg::Options::default())
        .map_err(|e| JsError::new(&format!("usvg parse error: {e}")))?;

    postcard::to_stdvec(&tree)
        .map_err(|e| JsError::new(&format!("postcard encode error: {e}")))
}
```

That is the entire file.

### 4.2 `crates/usvg2rgba/src/lib.rs`

```rust
use serde::Deserialize;
use usvg::Tree;
use wasm_bindgen::prelude::*;

#[derive(Deserialize)]
pub struct RgbaOptions {
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[wasm_bindgen]
pub struct RgbaResult {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

#[wasm_bindgen]
impl RgbaResult {
    #[wasm_bindgen(getter)]
    pub fn pixels(&self) -> Vec<u8> {
        self.pixels.clone()
    }
    // ... getters for width and height ...
}

#[wasm_bindgen]
pub fn usvg2rgba(
    usvg_bytes: &[u8],
    options: Option<JsValue>,
) -> Result<JsValue, JsError> {
    let tree: Tree = postcard::from_bytes(usvg_bytes)
        .map_err(|e| JsError::new(&format!("postcard decode error: {e}")))?;

    let opts: RgbaOptions = match options {
        Some(v) => serde_wasm_bindgen::from_value(v)
            .map_err(|e| JsError::new(&format!("options decode error: {e}")))?,
        None => RgbaOptions { width: None, height: None },
    };

    let natural = tree.size();
    let width = opts.width.unwrap_or(natural.width() as u32);
    let height = opts.height.unwrap_or(natural.height() as u32);

    let mut pixmap = tiny_skia::Pixmap::new(width, height)
        .ok_or_else(|| JsError::new("pixmap allocation failed"))?;

    let transform = tiny_skia::Transform::from_scale(
        width as f32 / natural.width() as f32,
        height as f32 / natural.height() as f32,
    );

    tree.render(transform, &mut pixmap.as_mut());

    Ok(/* serialized RgbaResult */)
}
```

The exact structure of the return value is decided during
implementation; the only contract is what is in
`docs/project-constitution.md` §4.2.

---

## 5. The TypeScript surfaces

### 5.1 `src/usvg.ts` (auto-generated wrapper, then committed)

```ts
// Do not hand-edit. Re-run scripts/build.ts to regenerate.
import init, { svg2usvg as svg2usvgRaw } from "../assets/svg2usvg_bg.wasm";

let initialized = false;

async function ensureInit(): Promise<void> {
  if (initialized) return;
  await init();
  initialized = true;
}

export async function svg2usvg(svg: string): Promise<Uint8Array> {
  await ensureInit();
  return svg2usvgRaw(svg);
}
```

### 5.2 `src/rgba.ts` (auto-generated wrapper, then committed)

```ts
// Do not hand-edit. Re-run scripts/build.ts to regenerate.
import init, { usvg2rgba as usvg2rgbaRaw } from "../assets/usvg2rgba_bg.wasm";

let initialized = false;

async function ensureInit(): Promise<void> {
  if (initialized) return;
  await init();
  initialized = true;
}

export interface Usvg2RgbaOptions {
  width?: number;
  height?: number;
}

export interface RgbaResult {
  pixels: Uint8Array;
  width: number;
  height: number;
}

export async function usvg2rgba(
  usvg: Uint8Array,
  options?: Usvg2RgbaOptions,
): Promise<RgbaResult> {
  await ensureInit();
  return usvg2rgbaRaw(usvg, options);
}
```

### 5.3 `src/mod.ts`

```ts
export { svg2usvg } from "./usvg.ts";
export { usvg2rgba, type Usvg2RgbaOptions, type RgbaResult } from "./rgba.ts";
```

The agent must **not** hand-edit any file under `src/` that is
generated by `scripts/build.ts`. If the wrapper needs to change,
update the build script.

---

## 6. Data flow

### 6.1 Producer (`svg2usvg`)

```
┌─────────────────────────────────────────────────────────────────┐
│ Consumer code (Deno / browser)                                   │
│                                                                  │
│   import { svg2usvg } from "jsr:@your-org/svg2ui8a/usvg";       │
│                                                                  │
│   const bytes = await svg2usvg(svgString);                       │
└──────────────────────────────┬───────────────────────────────────┘
                               │ "svg string" (UTF-8)
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│ Wasm boundary (svg2usvg_bg.wasm)                                │
│                                                                  │
│   1. usvg::Tree::from_str(svg, &Options::default())             │
│      - XML parse                                                 │
│      - CSS / <style> resolve                                     │
│      - Attribute normalization                                   │
│      - <use> / <defs> reference resolution                       │
│      - returns: usvg::Tree                                       │
│                                                                  │
│   2. postcard::to_stdvec(&tree)                                  │
│      - returns: Vec<u8>                                          │
│                                                                  │
│   3. wasm-bindgen: Vec<u8> → Uint8Array (one memcpy)            │
└──────────────────────────────┬───────────────────────────────────┘
                               │ Uint8Array (postcard bytes)
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│ Consumer code                                                    │
│   - hash(bytes) → cache key                                      │
│   - cache.set(key, bytes)                                        │
│   - or pass bytes to usvg2rgba                                   │
└─────────────────────────────────────────────────────────────────┘
```

The Wasm boundary is crossed **once**. No intermediate JS object,
no JSON, no SVG string, no pixel buffer.

### 6.2 Consumer (`usvg2rgba`)

```
┌─────────────────────────────────────────────────────────────────┐
│ Consumer code                                                    │
│                                                                  │
│   import { usvg2rgba } from "jsr:@your-org/svg2ui8a/rgba";      │
│                                                                  │
│   const { pixels, width, height } = await usvg2rgba(            │
│     usvgBytes,                                                   │
│     { width: 1200, height: 630 },                                │
│   );                                                             │
└──────────────────────────────┬───────────────────────────────────┘
                               │ Uint8Array (postcard bytes)
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│ Wasm boundary (usvg2rgba_bg.wasm)                                │
│                                                                  │
│   1. wasm-bindgen: Uint8Array → &[u8] (no copy)                 │
│                                                                  │
│   2. postcard::from_bytes(&[u8])                                 │
│      - returns: usvg::Tree                                       │
│                                                                  │
│   3. tiny_skia::Pixmap::new(width, height)                      │
│      - allocate RGBA buffer                                      │
│                                                                  │
│   4. tree.render(transform, &mut pixmap)                         │
│      - walk the tree                                             │
│      - draw each node                                            │
│      - write to pixmap.pixels()                                  │
│                                                                  │
│   5. wasm-bindgen: pixmap.pixels() → Uint8Array (one memcpy)    │
└──────────────────────────────┬───────────────────────────────────┘
                               │ Uint8Array (RGBA bytes, w*h*4)
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│ Consumer code                                                    │
│   - encode to PNG (consumer's encoder)                           │
│   - upload to canvas                                             │
│   - run a pixel-level filter                                     │
│   - anything that wants raw RGBA                                 │
└─────────────────────────────────────────────────────────────────┘
```

The Wasm boundary is crossed **twice** (in, then out), with a
single memcpy each way. The pixmap is allocated and drawn entirely
inside Wasm.

### 6.3 End-to-end: SVG in, RGBA out

```
SVG string
   │
   ▼  [svg2usvg Wasm]
usvg bytes (Uint8Array)
   │
   ▼  [hash + cache lookup]
   │
   ├─ HIT  → return cached RGBA / PNG
   │
   └─ MISS
        │
        ▼  [usvg2rgba Wasm]
       RGBA bytes (Uint8Array)
        │
        ▼  [consumer's encoder]
       PNG (Uint8Array)
        │
        ▼  [cache.set]
        return
```

The two Wasm builds are independently loadable. A consumer that
hits the cache 99% of the time will only ever load the `usvg`
Wasm.

---

## 7. What the agent should and should not optimize

### 7.1 Worth optimizing

- **Wasm binary size, per artifact.** Smaller is better. The
  current dependency lists in §3 are the minimum; the agent
  should not add anything that inflates either one.
- **Wasm cold-start latency per artifact.** First-call
  `await init()` cost. The TS wrapper handles this with a single
  per-module `initialized` flag, which is sufficient.
- **Wasm linear-memory-to-JS-heap copies.** These are the two
  `memcpy`s in the data flow diagrams. There is nothing the agent
  can do about them short of changing the format (which is
  prohibited).

### 7.2 Not worth optimizing

- **SVG parse time inside `usvg`.** Upstream behavior. Do not
  patch `usvg`. If it is too slow for a specific input, surface
  the issue per `./AGENTS.md` §10.
- **`resvg` / `tiny-skia` render time.** Upstream behavior. Same
  rule.
- **postcard encode / decode time.** Upstream behavior. Same
  rule.
- **The TS wrappers.** The current wrappers are correct. Do not
  turn them into classes, do not add caching, do not add options.

---

## 8. Failure modes

### 8.1 Malformed SVG (in `svg2usvg`)

`usvg::Tree::from_str` returns an error. `svg2usvg` rejects the
returned promise. The agent must not swallow the error.

### 8.2 Malformed `usvg` bytes (in `usvg2rgba`)

`postcard::from_bytes` returns an error if the input is not a
valid postcard-encoded `usvg::Tree`. `usvg2rgba` rejects the
returned promise. The agent must not swallow the error.

### 8.3 Out-of-memory

If the SVG is pathologically large, or if `width` / `height` are
pathologically large, the Wasm linear memory may exhaust.
`wasm-bindgen` will surface this as a JS exception. The caller is
expected to handle it; the agent must not pre-emptively cap input
size inside `svg2ui8a`.

### 8.4 Version skew

If the consumer's Wasm binary does not match the TS glue (e.g.
because one Wasm was rebuilt but the corresponding wrapper was
not regenerated), behavior is undefined. The build script's job
is to keep them in sync; the agent must not bypass it.

### 8.5 Two-instance confusion

The two Wasm artifacts are **independent**. A `svg2usvg` payload
that was produced by an older `usvg` Wasm may not be decodable
by a newer `rgba` Wasm if the postcard layout changed in between.
The cache key versioning in
`docs/project-constitution.md` §6 is the intended escape hatch.

---

## 9. Dependency graph (one-line summary)

```
svg2ui8a (TS)
  ├── svg2usvg_bg.wasm (Wasm)
  │     ├── usvg
  │     └── postcard
  │
  └── usvg2rgba_bg.wasm (Wasm)
        ├── usvg
        ├── postcard
        ├── resvg
        └── tiny-skia
```

No other dependencies. If a future plan adds another dependency
at any level, that plan must justify it in the "Dependency
changes" section and the human must approve.
