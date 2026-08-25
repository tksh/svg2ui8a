# System Architecture: `@tksh/svg2ui8a`

This file is the **third** document an AI agent reads (after `AGENTS.md` and
`docs/project-constitution.md`). It describes the package layout, the data flow,
and the Wasm boundary.

The human owns this file. If the agent believes something here is wrong, the
agent must surface the issue, not rewrite the file.

---

## 1. Package layout

`@tksh/svg2ui8a` is a single JSR package. Internally it is a Cargo workspace
with one shared library crate, three Wasm crates, and a TypeScript wrapper
layer:

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
│   ├── svg2rgba.ts
│   ├── stln.ts
│   └── stln-rgba.ts
├── crates/
│   ├── intermediate/
│   │   ├── Cargo.toml
│   │   └── src/lib.rs       # versioned DTO and CBOR codec
│   ├── svg2stln/
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   └── svg_core.rs
│   │   └── examples/
│   ├── stln2rgba/
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   └── core.rs
│   │   └── tests/
│   └── svg2rgba/
│       ├── Cargo.toml
│       ├── src/
│       │   ├── lib.rs
│       │   └── core.rs
│       └── examples/
│       ├── Cargo.toml
│       ├── src/
│       │   ├── lib.rs
│       │   └── core.rs
│       └── tests/
├── assets/
│   ├── svg2rgba_bg.wasm
│   ├── svg2stln_bg.wasm
│   └── stln2rgba_bg.wasm
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

The three Wasm crates are independent artifacts, but they share the non-Wasm
`intermediate` crate. It is the single source of truth for the versioned DTO,
canonical-CBOR envelope, validation, and conversion to and from the supported
`usvg::Tree` subset.

The two TS wrappers are also fully independent. They each import their own Wasm
binary, manage their own `init()` flag, and export their own function.

---

## 2. JSR subpath exports

The package exposes the following subpaths:

| Subpath                      | What it loads                                   | Wasm size |
| ---------------------------- | ----------------------------------------------- | --------- |
| Subpath                      | What it loads                                   | Wasm size |
| ---------------------------- | ----------------------------------------------- | --------- |
| `@tksh/svg2ui8a`             | Re-exports all three leaf subpaths              | (sum)     |
| `@tksh/svg2ui8a/svg2rgba`    | One-shot general-purpose render                 | Medium    |
| `@tksh/svg2ui8a/svg2stln`    | Straightlines producer only                     | Small     |
| `@tksh/svg2ui8a/stln2rgba`   | Straightlines rasterizer only                   | Larger    |

**Subpath imports are the supported way to load a single Wasm artifact.** The
root import is a convenience for consumers who want both; it does not enable any
tree-shaking that the subpath imports would not already enable.

`jsr.json` example (illustrative; the implementer fills the specifics):

```json
{
  "name": "@tksh/svg2ui8a",
  "version": "0.1.0",
  "exports": {
    ".": "./src/mod.ts",
    "./svg2rgba": "./src/svg2rgba.ts",
    "./svg2stln": "./src/stln.ts",
    "./stln2rgba": "./src/stln-rgba.ts"
  }
}
```

`deno.json` example (illustrative; the implementer fills the specifics):

```json
{
  "tasks": {
    "build": "deno run -A scripts/build.ts",
    "test": "deno task test:rust && deno task test:wasm && deno test -A",
    "test:rust": "cd crates/intermediate && cargo test && cd ../svg2stln && cargo test && cd ../stln2rgba && cargo test && cd ../svg2rgba && cargo test",
    "test:wasm": "deno run -A scripts/test-wasm.ts",
    "fmt": "cargo fmt && deno fmt",
    "lint": "deno lint",
    "check": "deno check src/**/*.ts"
  }
}
```

### 2.1 Consumer import examples

```ts
// Cache key only — load the usvg Wasm, do not load the rasterizer
import { svg2rgba } from "jsr:@tksh/svg2ui8a/svg2rgba";

// Already have a cached Straightlines payload — load the rasterizer,
// do not run the SVG parser or the general-purpose renderer
import { stln2rgba } from "jsr:@tksh/svg2ui8a/stln2rgba";

// Both — load both Wasm artifacts
import { svg2stln } from "jsr:@tksh/svg2ui8a/svg2stln";
import { stln2rgba, svg2rgba } from "jsr:@tksh/svg2ui8a";
```

---

## 3. The three Wasm builds

The Straightlines pair (`svg2stln`, `stln2rgba`) both depend on `intermediate`,
which uses `usvg` 0.47.0 and `cbor-core` 0.10.1; `stln2rgba` additionally uses
`resvg` 0.47.0. The one-shot `svg2rgba` depends on `resvg` (and its `usvg`
re-export) only — no `intermediate`, no CBOR. All crates use default-features-
disabled upstreams, so text/system-font support and raster-image decoding are
compiled into none of them.

### 3.1 `svg2stln` (the Straightlines producer)

The work is:

1. Parse the SVG with feature-disabled `usvg` and reject text or image content.
   The document must also declare a supported root-level `shape-rendering`
   (`geometricPrecision` or `crispEdges`, consistently for every path; `auto` is
   accepted as `geometricPrecision`); anything else is rejected.
2. Convert the supported tree into `IntermediateV1` in `intermediate`.
3. Encode its canonical-CBOR envelope with `cbor_core`.
4. Return the CBOR bytes to the caller.

The work does **not** include:

- Rasterization (no `resvg`, no `tiny-skia`, no `png`).
- Geometry walk beyond what `usvg` itself does.

The implementation plan must explain why each required dependency is needed and
what role it plays in the boundary.

### 3.2 `stln2rgba` (the Straightlines consumer)

The work is:

1. Decode and semantically validate the canonical-CBOR envelope with
   `intermediate`; reject unknown identifiers, versions, and DTO variants.
2. Reconstruct a supported `usvg::Tree` from `IntermediateV1`.
3. Rasterize the tree with feature-disabled `resvg` and its `tiny-skia`
   re-export.
4. Optionally un-premultiply alpha (default: yes).
5. Return the RGBA bytes (and dimensions / alpha mode) to the caller.

The work does **not** include:

- SVG parsing (no need — the input is the intermediate, not raw SVG).
- PNG / WebP / image-format encoding (the package's contract is RGBA; the
  consumer encodes).

### 3.3 `svg2rgba` (the general-purpose one-shot)

The work is:

1. Reject `<text>` / `<image>` content (not compiled in — an omitted payload
   would be unrenderable).
2. Parse the SVG with feature-disabled `usvg`.
3. Rasterize directly with feature-disabled `resvg` into a `tiny_skia::Pixmap`.
4. Apply the same sizing rule and alpha-mode handling as `stln2rgba`.

No CBOR envelope is read or written, and the `intermediate` crate is not
involved. Determinism is asserted as "same input → same pixels" only.

### 3.4 Why three builds

The `usvg` and `rgba` builds serve different use cases with different cost
profiles:

- A consumer who only needs cache keys (a CDN, a KV layer, a deduplication
  pipeline) should never pay for the rasterizer to be loaded into memory.
- A consumer who already has a cached intermediate payload (e.g. on a hot path
  where 99% of requests hit the cache) should never pay for the SVG parser.

Merging any of them into a single Wasm would force every consumer to load code
they do not use. That violates `docs/project-constitution.md` §3.9 (as amended:
one independent artifact per capability).

### 3.4 Workspace pin

The root `Cargo.toml` pins the shared dependencies once. Both crates reference
them via `{ workspace = true }`. There is exactly one place to bump a shared
version.

```toml
# Cargo.toml (workspace root)
[workspace]
members = [
  "crates/intermediate", "crates/svg2rgba", "crates/svg2stln",
  "crates/stln2rgba",
]
resolver = "2"

[workspace.dependencies]
cbor-core = "0.10.1"
usvg = { version = "0.47.0", default-features = false }
resvg = { version = "0.47.0", default-features = false }
```

---

## 4. The internal representation

The representation is a package-owned DTO, not a serialization of `usvg::Tree`.
The exact drawing fields remain an implementation choice, but the envelope and
validation boundary are fixed.

The constraints on the choice are:

- The top-level map uses integer keys `0`, `1`, and `2` for the literal format
  identifier, unsigned format version, and DTO payload respectively.
- The representation must be encoded and decoded by `cbor_core` 0.10.1 as
  canonical CBOR, then semantically validated before tree reconstruction.
- The representation must be **deterministic for a given SVG input** (modulo the
  version prefix in `docs/project-constitution.md` §6), so the bytes can be
  hashed.
- The representation must be **readable by `stln2rgba`** through the shared
  `intermediate` crate; schema code is never duplicated between Wasm crates.
- The representation must **not include fonts, raster images, BBoxes, animation
  state, or external resources**.

The implementation's plan records:

- The version-1 DTO drawing fields and its mapping to the supported tree subset.
  Besides per-shape fields, the version-1 payload carries one required
  root-level field, `shape_rendering` (`geometricPrecision` | `crispEdges`),
  mirroring the document-level `shape-rendering` declaration the producer
  requires on every input SVG.
- The mapping between the representation and `cbor_core::Value`.
- A short rationale (1–3 sentences) for each.

If the implementer finds the constraints above cannot be met with current
ecosystem crates, they stop and escalate per `./AGENTS.md` §10. The constitution
§3.7 forbids switching formats, but it does not forbid switching DTOs.

---

## 5. The TypeScript surfaces

### 5.1 `src/stln.ts`

A thin Deno/Wasm wrapper that:

- Imports the Wasm binary (the exact import path is decided during impl; see
  `./docs/engineering-playbook.md` §2 for the build pipeline).
- Exposes a single async function
  `svg2stln(svg: string):
  Promise<Uint8Array>`.
- Manages an `initialized` flag for `init()`.

The file is template-shaped by `scripts/build.ts`; the agent regenerates it,
never hand-edits it.

### 5.2 `src/stln-rgba.ts`

A thin Deno/Wasm wrapper that:

- Imports the Wasm binary.
- Exposes a single async function
  `stln2rgba(stln: Uint8Array,
  options?: Stln2RgbaOptions): Promise<RgbaResult>`.
  The exact shape of the options and the result is fixed by
  `docs/project-constitution.md` §4.2; the implementer chooses the inner type
  names and field names.
- Manages an `initialized` flag for `init()`.

The file is template-shaped by `scripts/build.ts`; the agent regenerates it,
never hand-edits it.

### 5.3 `src/mod.ts`

A hand-written re-export module that surfaces the two functions (and the two
types that `stln2rgba` needs in its public surface) from the root subpath. The
contents of this file are:

```ts
export { svg2rgba } from "./svg2rgba.ts";
export type { RgbaResult, Svg2RgbaOptions } from "./svg2rgba.ts";
export { svg2stln } from "./stln.ts";
export { stln2rgba } from "./stln-rgba.ts";
export type { RgbaResult, Stln2RgbaOptions } from "./stln-rgba.ts";
```

---

## 6. Data flow

### 6.1 Producer (`svg2stln`)

```
┌─────────────────────────────────────────────────────────────────┐
│ Consumer code (Deno / browser)                                   │
│                                                                  │
│   import { svg2rgba } from "jsr:@tksh/svg2ui8a/svg2rgba";           │
│                                                                  │
│   const bytes = await svg2stln(svgString);                       │
└──────────────────────────────┬───────────────────────────────────┘
                               │ "svg string" (UTF-8)
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│ Wasm boundary (svg2stln_bg.wasm)                                │
│                                                                  │
│   1. Parse the SVG with feature-disabled usvg                     │
│      - reject text and image content                              │
│                                                                  │
│   2. Project Tree → IntermediateV1                                │
│                                                                  │
│   3. Encode the versioned envelope as canonical CBOR              │
│      with cbor_core                                                │
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
│   - write/read a .cbor file, then pass the same bytes to         │
│     stln2rgba                                                     │
└─────────────────────────────────────────────────────────────────┘
```

The Wasm boundary is crossed **once**. No intermediate JS object, no JSON, no
SVG string, no pixel buffer.

### 6.2 Consumer (`stln2rgba`)

```
┌─────────────────────────────────────────────────────────────────┐
│ Consumer code                                                    │
│                                                                  │
│   import { stln2rgba } from "jsr:@tksh/svg2ui8a/stln2rgba";     │
│                                                                  │
│   const { pixels, width, height } = await stln2rgba(            │
│     usvgBytes,                                                   │
│     { /* width, height, alphaMode — impl-chosen shape */ },      │
│   );                                                             │
└──────────────────────────────┬───────────────────────────────────┘
                               │ Uint8Array (CBOR bytes)
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│ Wasm boundary (stln2rgba_bg.wasm)                                │
│                                                                  │
│   1. wasm-bindgen: Uint8Array → &[u8] (no copy)                 │
│                                                                  │
│   2. Decode and validate the versioned CBOR envelope              │
│      - returns: IntermediateV1                                    │
│                                                                  │
│   3. Build a supported `usvg::Tree` from IntermediateV1           │
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

The exact calls inside the Wasm (which `usvg` / `resvg` function, which
conversion shape) are the implementer's call. The diagram above names candidate
functions; the implementation chooses which ones to use.

### 6.3 End-to-end (consumer-side)

```
SVG string
   │
   ▼  [svg2stln Wasm]
CBOR bytes (Uint8Array)
   │
   ▼  [hash + cache lookup]
   │
   ├─ HIT  → return the consumer's cached payload
   │
   └─ MISS
        │
        ▼  [stln2rgba Wasm]
       RGBA bytes (Uint8Array)
        │
        ▼  [consumer's encoder]
       PNG (Uint8Array) or canvas draw
        │
        ▼  [cache.set]
        return
```

The package itself does not see PNG. The two Wasm builds are independently
loadable. A consumer that hits the cache 99% of the time will only ever load the
`usvg` Wasm.

---

## 7. What the agent should and should not optimize

### 7.1 Worth optimizing

- **Wasm binary size, per artifact.** Smaller is better. The plan records the
  chosen dependency set and a rough size estimate per artifact.
- **Wasm cold-start latency per artifact.** First-call `await init()` cost. The
  TS wrapper handles this with a single per-module `initialized` flag, which is
  sufficient.
- **Wasm linear-memory-to-JS-heap copies.** These are the two `memcpy`s in the
  data flow diagrams. There is nothing the agent can do about them short of
  changing the format (which is prohibited).

### 7.2 Not worth optimizing

- **SVG parse time inside `usvg`.** Upstream behavior. Do not patch `usvg`. If
  it is too slow for a specific input, surface the issue per `./AGENTS.md` §10.
- **`resvg` / `tiny-skia` render time.** Upstream behavior. Same rule.
- **CBOR encode / decode time.** Upstream behavior. Same rule.
- **The TS wrappers.** The current wrappers are correct. Do not turn them into
  classes, do not add caching, do not add options.

---

## 8. Failure modes

### 8.1 Malformed SVG (in `svg2stln`)

The parser returns an error. `svg2stln` rejects the returned promise. The agent
must not swallow the error.

### 8.2 Invalid CBOR file bytes (in `stln2rgba`)

The decoder returns an error if input is non-canonical CBOR, has a wrong format
identifier, unsupported format version, invalid DTO shape, or unsupported DTO
content. `stln2rgba` rejects the returned promise. The agent must not swallow
the error.

### 8.3 Out-of-memory

If the SVG is pathologically large, or if `width` / `height` are pathologically
large, the Wasm linear memory may exhaust. The runtime will surface this as a JS
exception. The caller is expected to handle it; the agent must not pre-emptively
cap input size inside the package.

### 8.4 Version skew

If the consumer's Wasm binary does not match the TS glue (e.g. because one Wasm
was rebuilt but the corresponding wrapper was not regenerated), behavior is
undefined. The build script's job is to keep them in sync; the agent must not
bypass it.

### 8.5 Two-instance confusion

The two Wasm artifacts are **independent**. A payload produced by an older
`usvg` Wasm may not be decodable by a newer `rgba` Wasm if the inner schema
changed in between. The cache key versioning in `docs/project-constitution.md`
§6 is the intended escape hatch.

Because both crates use workspace-pinned `usvg` 0.47.0 and `cbor-core` 0.10.1
versions (§3.4), this kind of skew only happens when the consumer deploys
mismatched `usvg` and `rgba` builds (a deploy-time concern), not when the
package itself is built.

---

## 9. Dependency graph (one-line summary)

```
@tksh/svg2ui8a (TS)
  ├── svg2rgba_bg.wasm (Wasm, one-shot)
  │     ├── resvg 0.47.0   (default features disabled)
  │     │     └── usvg 0.47.0 (re-export)
  │     ├── tiny-skia 0.12.0 (via resvg)
  │     └── wasm-bindgen
  │
  ├── svg2stln_bg.wasm (Wasm, Straightlines producer)
  │     ├── intermediate    (shared DTO and CBOR codec)
  │     ├── usvg 0.47.0     (default features disabled)
  │     ├── cbor-core 0.10.1 (workspace-pinned)
  │     └── wasm-bindgen
  │
  └── stln2rgba_bg.wasm (Wasm, Straightlines rasterizer)
        ├── intermediate    (shared DTO and CBOR codec)
        ├── usvg 0.47.0     (default features disabled)
        ├── cbor-core 0.10.1 (workspace-pinned, same as above)
        ├── resvg 0.47.0    (default features disabled)
        ├── tiny-skia 0.12.0 (via resvg)
        └── wasm-bindgen
```

The exact list of "impl-dependent" dependencies is decided during implementation
and recorded in the plan. The `usvg`, `resvg`, and `cbor-core` versions and the
disabled upstream features are workspace-pinned so no artifact can drift or
acquire forbidden font/image support.
