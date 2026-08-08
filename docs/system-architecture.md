# System Architecture: `@tksh/svg2ui8a`

This file is the **third** document an AI agent reads (after
`AGENTS.md` and `docs/project-constitution.md`). It describes the
package layout, the data flow, and the Wasm boundary.

The human owns this file. If the agent believes something here is
wrong, the agent must surface the issue, not rewrite the file.

---

## 1. Package layout

`@tksh/svg2ui8a` is a single JSR package. Internally it is a Cargo
workspace with two crates and a TypeScript wrapper layer:

```
svg2ui8a/
├── AGENTS.md
├── CHANGELOG.md
├── README.md
├── LICENSE
├── Cargo.toml                  # workspace root, pins shared deps
├── jsr.json
├── deno.json
├── .gitignore
├── src/
│   ├── mod.ts
│   ├── usvg.ts
│   └── rgba.ts
├── crates/
│   ├── svg2usvg/
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   └── core.rs        # non-bindgen core
│   │   └── tests/
│   └── usvg2rgba/
│       ├── Cargo.toml
│       ├── src/
│       │   ├── lib.rs
│       │   └── core.rs        # non-bindgen core
│       └── tests/
├── assets/
│   ├── svg2usvg_bg.wasm
│   └── usvg2rgba_bg.wasm
├── scripts/
│   ├── build.ts
│   ├── vendor.ts
│   └── check-cdn-free.ts
├── tests/                     # Deno integration tests
├── docs/
│   ├── project-constitution.md
│   ├── system-architecture.md
│   ├── engineering-playbook.md
│   ├── open-questions.md
│   └── open-questions-answers.md
├── docs/plans/                # working artifacts (agent-writable)
└── vendor/                    # gitignored except in release snapshots
```

The two Rust crates are **fully independent**. They share no
source files, no test code, and no build script. They share only
the workspace-pinned versions of `usvg` and `postcard`, sourced
from the root `Cargo.toml`'s `[workspace.dependencies]` table.
Drift between producer and consumer is structurally impossible.

The two TS wrappers are also fully independent. They each import
their own Wasm binary, manage their own `init()` flag, and export
their own function.

---

## 2. JSR subpath exports

The package exposes the following subpaths:

| Subpath                | What it loads                            | Wasm size |
|------------------------|------------------------------------------|-----------|
| `@tksh/svg2ui8a`       | Re-exports both `./usvg` and `./rgba`    | (sum)     |
| `@tksh/svg2ui8a/usvg`  | `svg2usvg` only                          | Small     |
| `@tksh/svg2ui8a/rgba`  | `usvg2rgba` only                         | Larger    |

**Subpath imports are the supported way to load a single Wasm
artifact.** The root import is a convenience for consumers who
want both; it does not enable any tree-shaking that the subpath
imports would not already enable.

`jsr.json` example:

```json
{
  "name": "@tksh/svg2ui8a",
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
    "test": "deno task test:rust && deno task test:wasm && deno test -A",
    "test:rust": "cd crates/svg2usvg && cargo test && cd ../usvg2rgba && cargo test",
    "test:wasm": "deno run -A scripts/test-wasm.ts",
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
import { svg2usvg } from "jsr:@tksh/svg2ui8a/usvg";

// Already have a cached usvg payload — load the rgba Wasm,
// do not load the SVG parser
import { usvg2rgba } from "jsr:@tksh/svg2ui8a/rgba";

// Both — load both Wasm artifacts
import { svg2usvg, usvg2rgba } from "jsr:@tksh/svg2ui8a";
```

---

## 3. The two Wasm builds

Both crates pin `usvg` and `postcard` via the workspace.

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
usvg = { workspace = true }
postcard = { workspace = true }
serde = { workspace = true }
wasm-bindgen = "0.2"

[dev-dependencies]
wasm-bindgen-test = "0.3"
```

Dependencies: `usvg`, `postcard`, `serde`, `wasm-bindgen`. That
is all. No `resvg`, no `tiny-skia`, no `png`, no `image`.

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
usvg = { workspace = true }
postcard = { workspace = true }
resvg = "0.34"
tiny-skia = "0.11"
serde = { workspace = true }
serde-wasm-bindgen = "0.6"
wasm-bindgen = "0.2"

[dev-dependencies]
wasm-bindgen-test = "0.3"
```

Dependencies include `resvg` and `tiny-skia` because the work is
**Tree → RGBA pixels**. There is no PNG encoder. The output is
raw RGBA, period.

`serde-wasm-bindgen` is required to decode the JS-side
`Usvg2RgbaOptions` object into a Rust struct.

### 3.3 Why two builds

The `usvg` and `rgba` builds serve different use cases with
different cost profiles:

- A consumer who only needs cache keys (a CDN, a KV layer, a
  deduplication pipeline) should never pay for `resvg` and
  `tiny-skia` to be loaded into memory. The `usvg` Wasm is
  roughly 2.5–3x smaller than the `rgba` Wasm.
- A consumer who already has a cached `usvg` payload (e.g. on a
  hot path where 99% of requests hit the cache) should never
  pay for the SVG parser. The `rgba` Wasm is meaningfully
  smaller and starts faster than the `usvg` Wasm.

A single Wasm with both functions would force every consumer to
load both code paths. That violates
`docs/project-constitution.md` §3.9.

### 3.4 Workspace pin

```toml
# Cargo.toml (workspace root)
[workspace]
members = ["crates/svg2usvg", "crates/usvg2rgba"]
resolver = "2"

[workspace.dependencies]
usvg = "0.34"
postcard = "1.0"
serde = { version = "1.0", features = ["derive"] }
```

Both crates reference these via `{ workspace = true }`. There is
exactly one place to bump the `usvg` or `postcard` version.

---

## 4. The Rust APIs

### 4.1 `crates/svg2usvg/src/lib.rs` and `core.rs`

```rust
// crates/svg2usvg/src/core.rs
use usvg::Tree;

pub fn svg2usvg_core(svg: &str) -> Result<Vec<u8>, Svg2UsvgError> {
    let tree = Tree::from_str(svg, &usvg::Options::default())?;
    postcard::to_stdvec(&tree).map_err(Svg2UsvgError::from)
}

#[derive(Debug, thiserror::Error)]
pub enum Svg2UsvgError {
    #[error("usvg parse error: {0}")]
    Usvg(String),
    #[error("postcard encode error: {0}")]
    Postcard(String),
}
```

```rust
// crates/svg2usvg/src/lib.rs
use wasm_bindgen::prelude::*;
use crate::core::{svg2usvg_core, Svg2UsvgError};

#[wasm_bindgen]
pub fn svg2usvg(svg: &str) -> Result<Vec<u8>, JsError> {
    svg2usvg_core(svg).map_err(|e| JsError::new(&e.to_string()))
}
```

The `core` function is what `cargo test` exercises. The
`#[wasm_bindgen]` wrapper is what the Wasm tests exercise.

### 4.2 `crates/usvg2rgba/src/lib.rs` and `core.rs`

```rust
// crates/usvg2rgba/src/core.rs
use serde::Deserialize;
use usvg::Tree;

#[derive(Debug, Default, Deserialize)]
pub struct RgbaOptions {
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub alpha_mode: Option<AlphaMode>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AlphaMode {
    Straight,
    Premultiplied,
}

impl Default for AlphaMode {
    fn default() -> Self { AlphaMode::Straight }
}

pub struct RgbaResultCore {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub alpha_mode: AlphaMode,
}

#[derive(Debug, thiserror::Error)]
pub enum Usvg2RgbaError {
    #[error("postcard decode error: {0}")]
    Postcard(String),
    #[error("pixmap allocation failed (width={0}, height={1})")]
    Allocation(u32, u32),
    #[error("invalid dimensions: width={0}, height={1}")]
    Dimensions(u32, u32),
    #[error("svg natural size is zero")]
    ZeroNaturalSize,
}

pub fn usvg2rgba_core(
    usvg_bytes: &[u8],
    opts: &RgbaOptions,
) -> Result<RgbaResultCore, Usvg2RgbaError> {
    let tree: Tree = postcard::from_bytes(usvg_bytes)
        .map_err(|e| Usvg2RgbaError::Postcard(e.to_string()))?;

    let natural = tree.size();
    if natural.width() <= 0.0 || natural.height() <= 0.0 {
        return Err(Usvg2RgbaError::ZeroNaturalSize);
    }

    let width = opts.width.unwrap_or(natural.width() as u32);
    let height = opts.height.unwrap_or(natural.height() as u32);
    if width == 0 || height == 0 {
        return Err(Usvg2RgbaError::Dimensions(width, height));
    }

    let mut pixmap = tiny_skia::Pixmap::new(width, height)
        .ok_or(Usvg2RgbaError::Allocation(width, height))?;

    // Render the tree into the pixmap.
    // Exact API: resvg::render(&tree, transform, &mut pixmap.as_mut())
    // (or a method on Tree, depending on resvg version).
    // The implementation task confirms the exact shape.
    // PLACEHOLDER — see engineering-playbook §3.2 for the
    // required test that exercises this path.
    let transform = tiny_skia::Transform::from_scale(
        width as f32 / natural.width() as f32,
        height as f32 / natural.height() as f32,
    );
    // TODO: actual resvg::render call (confirmed at impl time)
    let _ = (&tree, transform, &mut pixmap);

    let pixels = match opts.alpha_mode.unwrap_or_default() {
        AlphaMode::Straight => unpremultiply(pixmap.data()),
        AlphaMode::Premultiplied => pixmap.data().to_vec(),
    };

    Ok(RgbaResultCore {
        pixels,
        width,
        height,
        alpha_mode: opts.alpha_mode.unwrap_or_default(),
    })
}

fn unpremultiply(pixmap_data: &[u8]) -> Vec<u8> {
    let mut out = pixmap_data.to_vec();
    for chunk in out.chunks_exact_mut(4) {
        let a = chunk[3] as u32;
        if a == 0 {
            chunk[0] = 0; chunk[1] = 0; chunk[2] = 0;
        } else {
            chunk[0] = ((chunk[0] as u32 * 255) / a).min(255) as u8;
            chunk[1] = ((chunk[1] as u32 * 255) / a).min(255) as u8;
            chunk[2] = ((chunk[2] as u32 * 255) / a).min(255) as u8;
        }
    }
    out
}
```

```rust
// crates/usvg2rgba/src/lib.rs
use serde::Deserialize;
use wasm_bindgen::prelude::*;
use crate::core::{usvg2rgba_core, RgbaOptions, RgbaResultCore, Usvg2RgbaError};

#[wasm_bindgen]
pub fn usvg2rgba(usvg_bytes: &[u8], options: Option<JsValue>) 
    -> Result<JsValue, JsError> {
    let opts: RgbaOptions = match options {
        Some(v) => serde_wasm_bindgen::from_value(v)
            .map_err(|e| JsError::new(&e.to_string()))?,
        None => RgbaOptions::default(),
    };
    let core = usvg2rgba_core(usvg_bytes, &opts)
        .map_err(|e| JsError::new(&e.to_string()))?;
    Ok(/* serialize RgbaResultCore to JsValue */)
}
```

The `JsValue` returned by the `#[wasm_bindgen]` wrapper is an
**implementation detail**. The public TypeScript contract is
`RgbaResult` (see `docs/project-constitution.md` §4.2). The
wrapper may serialize via `serde_wasm_bindgen::to_value`, a
hand-built `js_sys::Object`, or any other internal mechanism;
that choice is not part of the API. The implementation task
selects the simplest correct approach.

---

## 5. The TypeScript surfaces

### 5.1 `src/usvg.ts` (template-shaped, committed)

```ts
// Template-shaped by scripts/build.ts. The agent regenerates,
// never hand-edits.
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

The committed `src/usvg.ts` imports the **raw `.wasm` artifact**
that was copied to `assets/`. The `wasm-pack`-generated
`pkg/svg2usvg.js` glue is an intermediate that the build script
consumes; it is **not** committed and **not** imported directly
by the published wrapper.

### 5.2 `src/rgba.ts` (template-shaped, committed)

```ts
// Template-shaped by scripts/build.ts. The agent regenerates,
// never hand-edits.
import init, { usvg2rgba as usvg2rgbaRaw } from "../assets/usvg2rgba_bg.wasm";

let initialized = false;

async function ensureInit(): Promise<void> {
  if (initialized) return;
  await init();
  initialized = true;
}

export type AlphaMode = "straight" | "premultiplied";

export interface Usvg2RgbaOptions {
  width?: number;
  height?: number;
  alphaMode?: AlphaMode;
}

export interface RgbaResult {
  pixels: Uint8Array;
  width: number;
  height: number;
  alphaMode: AlphaMode;
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
export {
  usvg2rgba,
  type AlphaMode,
  type Usvg2RgbaOptions,
  type RgbaResult,
} from "./rgba.ts";
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
│   import { svg2usvg } from "jsr:@tksh/svg2ui8a/usvg";           │
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
│   import { usvg2rgba } from "jsr:@tksh/svg2ui8a/rgba";          │
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
│      - allocate RGBA buffer, zero-initialized                    │
│                                                                  │
│   4. tree.render(transform, &mut pixmap)                         │
│      - walk the tree                                             │
│      - draw each node                                            │
│      - write to pixmap.pixels()                                  │
│                                                                  │
│   5. (if alphaMode == "straight") unpremultiply                 │
│      (if alphaMode == "premultiplied") skip                      │
│                                                                  │
│   6. wasm-bindgen: pixmap.pixels() → Uint8Array (one memcpy)    │
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
single memcpy each way. The pixmap is allocated and drawn
entirely inside Wasm.

### 6.3 End-to-end (consumer-side)

```
SVG string
   │
   ▼  [svg2usvg Wasm]
usvg bytes (Uint8Array)
   │
   ▼  [hash + cache lookup]
   │
   ├─ HIT  → return the consumer's cached payload
   │
   └─ MISS
        │
        ▼  [usvg2rgba Wasm]
       RGBA bytes (Uint8Array)
        │
        ▼  [consumer's encoder]
       PNG (Uint8Array) or canvas draw
        │
        ▼  [cache.set]
        return
```

The package itself does not see PNG. The two Wasm builds are
independently loadable. A consumer that hits the cache 99% of the
time will only ever load the `usvg` Wasm.

---

## 7. What the agent should and should not optimize

### 7.1 Worth optimizing

- **Wasm binary size, per artifact.** Smaller is better. The
  current dependency lists in §3 are the minimum; the agent
  should not add anything that inflates either one.
- **Wasm cold-start latency per artifact.** First-call
  `await init()` cost. The TS wrapper handles this with a
  single per-module `initialized` flag, which is sufficient.
- **Wasm linear-memory-to-JS-heap copies.** These are the two
  `memcpy`s in the data flow diagrams. There is nothing the
  agent can do about them short of changing the format (which
  is prohibited).

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
size inside the package.

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

Because both crates use the workspace-pinned `usvg` and
`postcard` versions (§3.4), this kind of skew only happens when
the consumer deploys mismatched `usvg` and `rgba` builds (a
deploy-time concern), not when the package itself is built.

---

## 9. Dependency graph (one-line summary)

```
@tksh/svg2ui8a (TS)
  ├── svg2usvg_bg.wasm (Wasm)
  │     ├── usvg              (workspace-pinned)
  │     ├── postcard          (workspace-pinned)
  │     ├── serde
  │     └── wasm-bindgen
  │
  └── usvg2rgba_bg.wasm (Wasm)
        ├── usvg              (workspace-pinned)
        ├── postcard          (workspace-pinned)
        ├── serde
        ├── serde-wasm-bindgen
        ├── resvg
        ├── tiny-skia
        └── wasm-bindgen
```

No other dependencies. If a future plan adds another dependency
at any level, that plan must justify it in the "Dependency
changes" section and the human must approve.
