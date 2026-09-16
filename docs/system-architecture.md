# System Architecture: `@tksh/svg2ui8a`

This file is the **third** document an AI agent reads (after `AGENTS.md` and
`docs/project-constitution.md`). It describes the package layout, the data flow,
and the Wasm boundary.

The human owns this file. If the agent believes something here is wrong, the
agent must surface the issue, not rewrite the file.

---

## 1. Package layout

`@tksh/svg2ui8a` is a single JSR package. Internally it is a Cargo workspace
with two Wasm crates and a TypeScript wrapper layer:

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
│   └── svg2usvg.ts
├── crates/
│   ├── svg2usvg/
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   └── core.rs
│   │   └── examples/
│   └── svg2rgba/
│       ├── Cargo.toml
│       ├── src/
│       │   ├── lib.rs
│       │   └── core.rs
│       └── examples/
├── assets/
│   ├── svg2rgba_bg.wasm
│   └── svg2usvg_bg.wasm
├── scripts/
│   ├── build.ts
│   ├── vendor.ts
│   └── check-cdn-free.ts
├── tests/                      # Deno integration tests
├── docs/
│   ├── project-constitution.md
│   ├── system-architecture.md
│   ├── engineering-playbook.md
│   └── plans/                  # working artifacts (agent-writable)
│   └── archive/                # superseded Straightlines/CBOR plans
└── vendor/                     # gitignored except in release snapshots
```

The two Wasm crates are independent artifacts. They each import their own Wasm
binary, manage their own `init()` flag, and export their own function.

Historical note: `0.1.x`–`0.2.0` had a `crates/intermediate` shared crate and
`svg2stln`/`stln2rgba` Straightlines/CBOR pipeline (`cbor-core`). That pipeline
was removed in `0.3.0` (see `docs/archive/` and `CHANGELOG.md` `0.3.0`).

---

## 2. JSR subpath exports

The package exposes the following subpaths:

| Subpath                   | What it loads                   | Wasm size |
| ------------------------- | ------------------------------- | --------- |
| `@tksh/svg2ui8a`          | Re-exports both leaf subpaths   | (sum)     |
| `@tksh/svg2ui8a/svg2rgba` | One-shot general-purpose render | Medium    |
| `@tksh/svg2ui8a/svg2usvg` | SVG → normalized usvg XML bytes | Small     |

**Subpath imports are the supported way to load a single Wasm artifact.** The
root import is a convenience for consumers who want both; it does not enable any
tree-shaking that the subpath imports would not already enable.

`jsr.json` example (illustrative; the implementer fills the specifics):

```json
{
  "name": "@tksh/svg2ui8a",
  "exports": {
    ".": "./src/mod.ts",
    "./svg2rgba": "./src/svg2rgba.ts",
    "./svg2usvg": "./src/svg2usvg.ts"
  }
}
```

`deno.json` example (illustrative; the implementer fills the specifics):

```json
{
  "tasks": {
    "build": "deno run -A scripts/build.ts",
    "test": "deno task test:rust && deno task test:wasm && deno test -A",
    "test:rust": "cd crates/svg2usvg && cargo test && cd ../svg2rgba && cargo test",
    "test:wasm": "deno run -A scripts/test-wasm.ts",
    "fmt": "cargo fmt && deno fmt",
    "lint": "deno lint",
    "check": "deno check src/**/*.ts"
  }
}
```

### 2.1 Consumer import examples

```ts
// One-shot pixels — load only the rasterizer
import { svg2rgba } from "jsr:@tksh/svg2ui8a/svg2rgba";

// Normalized SVG bytes — load only the normalizer
import { svg2usvg } from "jsr:@tksh/svg2ui8a/svg2usvg";

// Both — load both Wasm artifacts
import { svg2rgba, svg2usvg } from "jsr:@tksh/svg2ui8a";
```

---

## 3. The two Wasm builds

Both crates use default-features-disabled upstreams, so text/system-font support
and raster-image decoding are compiled into none of them. Current pins:
`usvg 0.47.0`, `resvg 0.47.0`.

### 3.1 `svg2usvg` (SVG → normalized usvg bytes)

The work is:

1. Reject `<text>` / `<image>` content (string scan, matching `svg2rgba`).
2. Parse the SVG with feature-disabled `usvg`.
3. Serialize the normalized tree with `usvg`'s default `XmlOptions` (minimal, no
   pretty-print) to UTF-8 bytes and return them.

The work does **not** include:

- Rasterization (no `resvg`, no `tiny-skia`, no `png`).
- Custom serialization (output is standard SVG XML, not CBOR).

### 3.2 `svg2rgba` (general-purpose one-shot)

The work is:

1. Reject `<text>` / `<image>` content.
2. Parse the SVG with feature-disabled `usvg`.
3. Rasterize directly with feature-disabled `resvg` into a `tiny_skia::Pixmap`.
4. Apply the sizing rule and alpha-mode handling.

No custom envelope is read or written. Determinism is asserted as "same input →
same pixels" only.

### 3.3 Why two builds

- A consumer who only needs normalized SVG bytes should never pay for the
  rasterizer.
- A consumer who only needs pixels should never pay for a separate intermediate.

Merging them into a single Wasm would force every consumer to load code they do
not use. That violates `docs/project-constitution.md` §3.9 (one independent
artifact per capability).

### 3.4 Workspace pin

The root `Cargo.toml` pins the shared dependencies once. Both crates reference
them via `{ workspace = true }`. There is exactly one place to bump a shared
version.

```toml
# Cargo.toml (workspace root)
[workspace]
members = ["crates/svg2usvg", "crates/svg2rgba"]
resolver = "2"

[workspace.dependencies]
usvg = { version = "0.47.0", default-features = false }
resvg = { version = "0.47.0", default-features = false }
```

---

## 4. Internal representation

There is **no proprietary internal representation**. The package does not define
a DTO or a versioned binary envelope.

- `svg2usvg` returns standard SVG XML bytes (UTF-8) produced by `usvg`; the
  bytes are valid SVG and re-parse with `usvg::Tree::from_str`.
- `svg2rgba` returns raw RGBA pixels; there is no intermediate.

Historical note: `0.1.x`–`0.2.0` used a canonical-CBOR envelope with integer
keys `0`/`1`/`2` and a package-owned DTO (`crates/intermediate`). That envelope
and DTO were removed in `0.3.0`; the archived plans in `docs/archive/` record
the design and its failure.

---

## 5. The TypeScript surfaces

### 5.1 `src/svg2usvg.ts`

A thin Deno/Wasm wrapper that:

- Imports the Wasm binary (the exact import path is decided during impl; see
  `./docs/engineering-playbook.md` §2 for the build pipeline).
- Exposes a single async function `svg2usvg(svg: string): Promise<Uint8Array>`.
- Manages an `initialized` flag for `init()`.

The file is template-shaped by `scripts/build.ts`; the agent regenerates it,
never hand-edits it.

### 5.2 `src/svg2rgba.ts`

A thin Deno/Wasm wrapper that:

- Imports the Wasm binary.
- Exposes a single async function
  `svg2rgba(svg: string, options?: Svg2RgbaOptions): Promise<RgbaResult>`. The
  exact shape of the options and the result is fixed by
  `docs/project-constitution.md` §4.2; the implementer chooses the inner type
  names and field names.
- Manages an `initialized` flag for `init()`.

The file is template-shaped by `scripts/build.ts`; the agent regenerates it,
never hand-edits it.

### 5.3 `src/mod.ts`

A hand-written re-export module that surfaces the two functions from the root
subpath. The contents of this file are:

```ts
export { svg2rgba } from "./svg2rgba.ts";
export type { RgbaResult, Svg2RgbaOptions } from "./svg2rgba.ts";
export { svg2usvg } from "./svg2usvg.ts";
```

---

## 6. Data flow

### 6.1 `svg2usvg`

```
┌─────────────────────────────────────────────────────────────────┐
│ Consumer code (Deno / browser)                                   │
│   import { svg2usvg } from "jsr:@tksh/svg2ui8a/svg2usvg";        │
│   const bytes = await svg2usvg(svgString);                       │
└──────────────────────────────┬───────────────────────────────────┘
                               │ "svg string" (UTF-8)
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│ Wasm boundary (svg2usvg_bg.wasm)                                 │
│   1. Reject text/image content                                   │
│   2. Parse with feature-disabled usvg                            │
│   3. Serialize with usvg default XmlOptions → Vec<u8>            │
│   4. wasm-bindgen: Vec<u8> → Uint8Array (one memcpy)            │
└──────────────────────────────┬───────────────────────────────────┘
                               │ Uint8Array (UTF-8 XML bytes)
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│ Consumer code                                                    │
│   - TextDecoder().decode(bytes) → normalized SVG string          │
│   - hash(bytes) → cache key                                      │
└─────────────────────────────────────────────────────────────────┘
```

### 6.2 `svg2rgba`

```
┌─────────────────────────────────────────────────────────────────┐
│ Consumer code                                                    │
│   import { svg2rgba } from "jsr:@tksh/svg2ui8a/svg2rgba";        │
│   const { pixels, width, height } = await svg2rgba(svgString);  │
└──────────────────────────────┬───────────────────────────────────┘
                               │ "svg string" (UTF-8)
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│ Wasm boundary (svg2rgba_bg.wasm)                                 │
│   1. Reject text/image content                                   │
│   2. Parse with usvg                                             │
│   3. Rasterize with resvg/tiny-skia into Pixmap                  │
│   4. Un-premultiply alpha (default) or keep premultiplied        │
│   5. wasm-bindgen: pixels → Uint8Array (one memcpy)             │
└──────────────────────────────┬───────────────────────────────────┘
                               │ Uint8Array (RGBA bytes, w*h*4)
                               ▼
┌─────────────────────────────────────────────────────────────────┐
│ Consumer code                                                    │
│   - encode to PNG (consumer's encoder) / canvas draw             │
└─────────────────────────────────────────────────────────────────┘
```

---

## 7. What the agent should and should not optimize

### 7.1 Worth optimizing

- **Wasm binary size, per artifact.** Smaller is better. The plan records the
  chosen dependency set and a rough size estimate per artifact.
- **Wasm cold-start latency per artifact.** First-call `await init()` cost. The
  TS wrapper handles this with a single per-module `initialized` flag, which is
  sufficient.
- **Wasm linear-memory-to-JS-heap copies.** The `memcpy`s at the Wasm boundary.
  There is nothing the agent can do about them short of changing the format
  (which is prohibited from being proprietary).

### 7.2 Not worth optimizing

- **SVG parse time inside `usvg`.** Upstream behavior. Do not patch `usvg`. If
  it is too slow for a specific input, surface the issue per `./AGENTS.md` §10.
- **`resvg` / `tiny-skia` render time.** Upstream behavior. Same rule.
- **The TS wrappers.** The current wrappers are correct. Do not turn them into
  classes, do not add caching, do not add options.

---

## 8. Failure modes

### 8.1 Malformed SVG (in `svg2usvg` / `svg2rgba`)

The parser returns an error. The function rejects the returned promise. The
agent must not swallow the error.

### 8.2 Out-of-memory

If the SVG is pathologically large, or if `width` / `height` are pathologically
large, the Wasm linear memory may exhaust. The runtime will surface this as a JS
exception. The caller is expected to handle it; the agent must not pre-emptively
cap input size inside the package.

### 8.3 Version skew

If the consumer's Wasm binary does not match the TS glue (e.g. because one Wasm
was rebuilt but the corresponding wrapper was not regenerated), behavior is
undefined. The build script's job is to keep them in sync; the agent must not
bypass it.

### 8.4 Two-instance confusion

The two Wasm artifacts are **independent**. The build pipeline in
`docs/engineering-playbook.md` §2 is structured to make this hard to do by
accident; the agent must not work around it.

Because both crates use workspace-pinned `usvg 0.47.0` / `resvg 0.47.0`, skew
only happens when the consumer deploys mismatched builds (a deploy-time
concern), not when the package itself is built.

---

## 9. Dependency graph (one-line summary)

```
@tksh/svg2ui8a (TS)
  ├── svg2usvg_bg.wasm (Wasm, SVG → usvg bytes)
  │     ├── usvg 0.47.0   (default features disabled)
  │     └── wasm-bindgen
  │
  └── svg2rgba_bg.wasm (Wasm, one-shot)
        ├── resvg 0.47.0   (default features disabled)
        │     └── usvg 0.47.0 (re-export)
        ├── tiny-skia 0.12.0 (via resvg)
        └── wasm-bindgen
```

The `usvg`, `resvg` versions and the disabled upstream features are
workspace-pinned so no artifact can drift or acquire forbidden font/image
support.
