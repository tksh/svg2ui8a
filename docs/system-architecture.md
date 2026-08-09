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
├── Cargo.toml                  # workspace root
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
│   │   │   └── core.rs
│   │   └── tests/
│   └── usvg2rgba/
│       ├── Cargo.toml
│       ├── src/
│       │   ├── lib.rs
│       │   └── core.rs
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
│   └── plans/                 # working artifacts (agent-writable)
└── vendor/                    # gitignored except in release snapshots
```

The two Rust crates are **fully independent**. They share no
source files, no test code, and no build script. They share only
the workspace-pinned versions of `usvg` and the CBOR codec, sourced
from the root `Cargo.toml`'s `[workspace.dependencies]` table.

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

`jsr.json` example (illustrative; the implementer fills the
specifics):

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

`deno.json` example (illustrative; the implementer fills the
specifics):

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

Both crates depend on `usvg` and on **a CBOR codec**. The
specific crate and the version of both are workspace-pinned
choices the implementation makes. The plan records the choice
and the reason.

### 3.1 `svg2usvg` (the producer)

The work is:

1. Parse the SVG into a normalized internal representation
   (using `usvg`, or a hand-rolled subset parser, or a
   combination — see §4).
2. Encode the internal representation as **CBOR**.
3. Return the CBOR bytes to the caller.

The work does **not** include:

- Rasterization (no `resvg`, no `tiny-skia`, no `png`).
- Geometry walk beyond what `usvg` itself does.

The implementation's chosen dependencies are recorded in the
plan. The plan must explain *why* each dependency is needed and
what role it plays in the boundary.

### 3.2 `usvg2rgba` (the consumer)

The work is:

1. Decode the input CBOR bytes into the internal representation.
2. Rasterize the representation (using `resvg` / `tiny-skia`).
3. Optionally un-premultiply alpha (default: yes).
4. Return the RGBA bytes (and dimensions / alpha mode) to the
   caller.

The work does **not** include:

- SVG parsing (no need — the input is the intermediate, not
  raw SVG).
- PNG / WebP / image-format encoding (the package's contract is
  RGBA; the consumer encodes).

### 3.3 Why two builds

The `usvg` and `rgba` builds serve different use cases with
different cost profiles:

- A consumer who only needs cache keys (a CDN, a KV layer, a
  deduplication pipeline) should never pay for the rasterizer
  to be loaded into memory.
- A consumer who already has a cached intermediate payload (e.g.
  on a hot path where 99% of requests hit the cache) should
  never pay for the SVG parser.

A single Wasm with both functions would force every consumer to
load both code paths. That violates
`docs/project-constitution.md` §3.9.

### 3.4 Workspace pin

The root `Cargo.toml` pins the shared dependencies once. Both
crates reference them via `{ workspace = true }`. There is
exactly one place to bump a shared version.

```toml
# Cargo.toml (workspace root)
[workspace]
members = ["crates/svg2usvg", "crates/usvg2rgba"]
resolver = "2"

[workspace.dependencies]
# The implementation fills in the specific versions after a
# survey of the current ecosystem. Suggested starting points:
#   usvg = "<latest stable at impl time>"
#   cbor-codec = "<crate chosen at impl time>"
# The pin is recorded in the implementation plan with reasons.
```

---

## 4. The internal representation

This section is **deliberately thin**. The exact fields, the
exact CBOR schema, the exact mapping from `usvg::Tree` (if
used) to the package's internal representation — all of this is
the **implementer's call**, not the constitution's.

The constraints on the choice are:

- The representation must be **CBOR-encodable** by a crate
  available in the Rust ecosystem at impl time.
- The representation must be **deterministic for a given SVG
  input** (modulo the version prefix in
  `docs/project-constitution.md` §6), so the bytes can be
  hashed.
- The representation must be **readable by `usvg2rgba`** —
  meaning that the producer and the consumer agree on the same
  schema. The schema lives in **one** place, not two (the
  `crates/svg2usvg` crate and the `crates/usvg2rgba` crate
  share the schema, even if they don't share the schema's
  source code).
- The representation may be:
  - a direct serialization of `usvg::Tree` (only if the
    chosen `usvg` version provides serde traits — verify
    during impl),
  - a **hand-designed DTO** that the package maps `usvg::Tree`
    into (recommended; avoids dependency on upstream serde
    support),
  - a hand-rolled SVG normalizer (most work, least dependency).
- The representation must **not include fonts, BBoxes, or
  animation state**.

The implementation's plan records:

- The chosen representation (DTO / Tree / custom).
- The chosen CBOR codec crate, and the version.
- The chosen strategy for determinism (CBOR encoder settings,
  dCBOR, etc.).
- A short rationale (1–3 sentences) for each.

If the implementer finds the constraints above cannot be met
with current ecosystem crates, they stop and escalate per
`./AGENTS.md` §10. The constitution §3.7 forbids switching
formats, but it does not forbid switching DTOs.

---

## 5. The TypeScript surfaces

### 5.1 `src/usvg.ts`

A thin Deno/Wasm wrapper that:

- Imports the Wasm binary (the exact import path is decided
  during impl; see `./docs/engineering-playbook.md` §2 for
  the build pipeline).
- Exposes a single async function `svg2usvg(svg: string):
  Promise<Uint8Array>`.
- Manages an `initialized` flag for `init()`.

The file is template-shaped by `scripts/build.ts`; the agent
regenerates it, never hand-edits it.

### 5.2 `src/rgba.ts`

A thin Deno/Wasm wrapper that:

- Imports the Wasm binary.
- Exposes a single async function `usvg2rgba(usvg: Uint8Array,
  options?: Usvg2RgbaOptions): Promise<RgbaResult>`. The exact
  shape of the options and the result is fixed by
  `docs/project-constitution.md` §4.2; the implementer chooses
  the inner type names and field names.
- Manages an `initialized` flag for `init()`.

The file is template-shaped by `scripts/build.ts`; the agent
regenerates it, never hand-edits it.

### 5.3 `src/mod.ts`

A hand-written re-export module that surfaces the two functions
(and the two types that `usvg2rgba` needs in its public surface)
from the root subpath. The contents of this file are:

```ts
export { svg2usvg } from "./usvg.ts";
export {
  usvg2rgba,
  // types are listed by name here; the implementer chooses
  // the names per constitution §4.2
} from "./rgba.ts";
```

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
│   1. Parse the SVG                                                │
│      (e.g. usvg::Tree::from_str)                                 │
│      (or hand-rolled; implementation choice)                      │
│                                                                  │
│   2. Project to the package's internal representation             │
│      (e.g. Tree → DTO conversion; or skip if direct)              │
│                                                                  │
│   3. Encode as CBOR                                               │
│      (e.g. ciborium, serde_cbor, or other)                        │
│      - returns: Vec<u8>                                          │
│                                                                  │
│   4. wasm-bindgen: Vec<u8> → Uint8Array (one memcpy)            │
└──────────────────────────────┬───────────────────────────────────┘
                               │ Uint8Array (CBOR bytes)
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
│     { /* width, height, alphaMode — impl-chosen shape */ },      │
│   );                                                             │
└──────────────────────────────┬───────────────────────────────────┘
                               │ Uint8Array (CBOR bytes)
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│ Wasm boundary (usvg2rgba_bg.wasm)                                │
│                                                                  │
│   1. wasm-bindgen: Uint8Array → &[u8] (no copy)                 │
│                                                                  │
│   2. Decode CBOR into the package's internal representation      │
│      - returns: DTO                                              │
│                                                                  │
│   3. Build a `usvg::Tree` (or equivalent) from the DTO           │
│                                                                  │
│   4. Rasterize into a `tiny_skia::Pixmap`                        │
│      - allocate RGBA buffer, zero-initialized                    │
│      - walk the tree                                             │
│      - draw each node                                            │
│      - write to pixmap.pixels()                                  │
│                                                                  │
│   5. (default) un-premultiply alpha                              │
│      (premultiplied option) skip                                 │
│                                                                  │
│   6. wasm-bindgen: pixmap.pixels() → Uint8Array (one memcpy)    │
│      plus the dimensions / alpha-mode fields for RgbaResult     │
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

The exact calls inside the Wasm (which `usvg` / `resvg`
function, which conversion shape) are the implementer's call.
The diagram above names candidate functions; the implementation
chooses which ones to use.

### 6.3 End-to-end (consumer-side)

```
SVG string
   │
   ▼  [svg2usvg Wasm]
CBOR bytes (Uint8Array)
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
independently loadable. A consumer that hits the cache 99% of
the time will only ever load the `usvg` Wasm.

---

## 7. What the agent should and should not optimize

### 7.1 Worth optimizing

- **Wasm binary size, per artifact.** Smaller is better. The
  plan records the chosen dependency set and a rough size
  estimate per artifact.
- **Wasm cold-start latency per artifact.** First-call
  `await init()` cost. The TS wrapper handles this with a
  single per-module `initialized` flag, which is sufficient.
- **Wasm linear-memory-to-JS-heap copies.** These are the two
  `memcpy`s in the data flow diagrams. There is nothing the
  agent can do about them short of changing the format
  (which is prohibited).

### 7.2 Not worth optimizing

- **SVG parse time inside `usvg`.** Upstream behavior. Do not
  patch `usvg`. If it is too slow for a specific input,
  surface the issue per `./AGENTS.md` §10.
- **`resvg` / `tiny-skia` render time.** Upstream behavior.
  Same rule.
- **CBOR encode / decode time.** Upstream behavior. Same
  rule.
- **The TS wrappers.** The current wrappers are correct. Do
  not turn them into classes, do not add caching, do not add
  options.

---

## 8. Failure modes

### 8.1 Malformed SVG (in `svg2usvg`)

The parser returns an error. `svg2usvg` rejects the returned
promise. The agent must not swallow the error.

### 8.2 Malformed CBOR bytes (in `usvg2rgba`)

The CBOR decoder returns an error if the input is not a valid
package intermediate. `usvg2rgba` rejects the returned promise.
The agent must not swallow the error.

### 8.3 Out-of-memory

If the SVG is pathologically large, or if `width` / `height`
are pathologically large, the Wasm linear memory may exhaust.
The runtime will surface this as a JS exception. The caller is
expected to handle it; the agent must not pre-emptively cap
input size inside the package.

### 8.4 Version skew

If the consumer's Wasm binary does not match the TS glue (e.g.
because one Wasm was rebuilt but the corresponding wrapper
was not regenerated), behavior is undefined. The build
script's job is to keep them in sync; the agent must not
bypass it.

### 8.5 Two-instance confusion

The two Wasm artifacts are **independent**. A payload
produced by an older `usvg` Wasm may not be decodable by a
newer `rgba` Wasm if the inner schema changed in between.
The cache key versioning in
`docs/project-constitution.md` §6 is the intended escape
hatch.

Because both crates use the workspace-pinned `usvg` and
CBOR-codec versions (§3.4), this kind of skew only happens
when the consumer deploys mismatched `usvg` and `rgba`
builds (a deploy-time concern), not when the package itself
is built.

---

## 9. Dependency graph (one-line summary)

```
@tksh/svg2ui8a (TS)
  ├── svg2usvg_bg.wasm (Wasm)
  │     ├── usvg            (workspace-pinned)
  │     ├── cbor-codec      (workspace-pinned, impl choice)
  │     ├── wasm-bindgen
  │     └── (impl-dependent: serde if codec requires it, etc.)
  │
  └── usvg2rgba_bg.wasm (Wasm)
        ├── usvg            (workspace-pinned)
        ├── cbor-codec      (workspace-pinned, same as above)
        ├── resvg
        ├── tiny-skia
        ├── wasm-bindgen
        └── (impl-dependent: serde, etc.)
```

The exact list of "impl-dependent" dependencies is decided
during implementation and recorded in the plan. The
constitution requires that the `usvg` and CBOR-codec versions
be **workspace-pinned** so that the two artifacts cannot drift.
