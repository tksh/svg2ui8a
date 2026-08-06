# Engineering Playbook: `svg2ui8a`

This file is the **fourth** document an AI agent reads (after
`AGENTS.md`, `docs/project-constitution.md`, and
`docs/system-architecture.md`). It describes the day-to-day
mechanics: how to build, how to test, how to add a feature without
breaking anything, and how to release.

The human owns this file. If the agent believes something here is
wrong, the agent must surface the issue, not rewrite the file.

---

## 1. Repository commands

All commands assume the working directory is the repository root.

| Command            | What it does                                    |
|--------------------|-------------------------------------------------|
| `deno task fmt`    | Format all `.ts` files in `src/` and `tests/`.  |
| `deno task lint`   | Lint all `.ts` files.                           |
| `deno task check`  | Type-check all `.ts` files.                     |
| `deno task test`   | Run the test suite (Deno + Rust).               |
| `deno task build`  | Rebuild the two Wasm binaries and the TS glue.  |

The agent must not invent commands. If a command above does not
work, stop and escalate per `./AGENTS.md` §10.

---

## 2. The build pipeline

The build pipeline has two parallel tracks (one per Wasm crate)
and a single driver script.

### 2.1 Per-crate track

For each of `crates/svg2usvg/` and `crates/usvg2rgba/`:

1. Compile the Rust crate to Wasm with `wasm-pack`,
   target `deno`.
2. Copy the Wasm artifact to `assets/<crate>_bg.wasm`.
3. Emit the TS glue (or a hand-written wrapper that uses the
   wasm-bindgen output) to `src/<subpath>.ts`.

### 2.2 The driver

`scripts/build.ts` runs both tracks and verifies the output:

```
crates/svg2usvg/pkg/svg2usvg_bg.wasm      → assets/svg2usvg_bg.wasm
crates/svg2usvg/pkg/svg2usvg.js           → (consumed by src/usvg.ts)
crates/usvg2rgba/pkg/usvg2rgba_bg.wasm   → assets/usvg2rgba_bg.wasm
crates/usvg2rgba/pkg/usvg2rgba.js        → (consumed by src/rgba.ts)
src/usvg.ts                               ← (committed, regenerated)
src/rgba.ts                               ← (committed, regenerated)
```

Run with `deno task build`. The agent must run the build script
end to end, not invoke the steps individually.

### 2.3 What the agent must never do

- Hand-edit `src/usvg.ts` or `src/rgba.ts`. Regenerate them.
- Hand-edit anything under `assets/`. The Wasm artifacts are
  build artifacts.
- Pin `wasm-pack` to a version not recorded in
  `scripts/build.ts`. If a version bump is needed, edit the
  script.
- Commit a build artifact in a way that is not reproducible from
  source. Both Wasm artifacts must be buildable from a clean
  checkout.
- Bundle the two crates into a single Wasm artifact. See
  `docs/project-constitution.md` §3.9.
- Add a runtime CDN import in the browser bundle. See
  `docs/project-constitution.md` §3.8 and `./AGENTS.md` §5.

### 2.4 Build output (what should appear after `deno task build`)

```
crates/svg2usvg/pkg/svg2usvg_bg.wasm
crates/usvg2rgba/pkg/usvg2rgba_bg.wasm
src/usvg.ts
src/rgba.ts
src/mod.ts                        # unchanged, hand-written
assets/svg2usvg_bg.wasm
assets/usvg2rgba_bg.wasm
```

If any of these are missing, the build is broken. Stop and
escalate.

---

## 3. Testing

There are four layers of tests, and they must all pass before
declaring a change done.

### 3.1 Rust native tests for `svg2usvg` (`cargo test`)

Located in `crates/svg2usvg/src/lib.rs` (or a sibling `tests/`
directory for integration tests). These run natively (not under
Wasm) and exercise the `svg2usvg` function directly.

Required test cases (at minimum):

- A simple SVG with a `<rect>` parses and produces a non-empty
  `Vec<u8>`.
- A simple SVG with a `<path>` produces stable bytes across two
  consecutive calls.
- A malformed SVG (e.g. unclosed tag) produces an error, not a
  panic.
- The same SVG string always produces the same bytes (the
  determinism check from `docs/project-constitution.md` §5.3).

### 3.2 Rust native tests for `usvg2rgba` (`cargo test`)

Same shape, but the input is a `Vec<u8>` (a known-good
postcard-encoded `usvg::Tree`), and the output is a
`tiny_skia::Pixmap` whose pixel buffer is checked.

Required test cases (at minimum):

- A simple `<rect>` SVG renders to a pixmap of the expected
  size.
- `width` / `height` options are honored (the output pixmap is
  the requested size, not the natural size).
- A `vec<u8>` that is not a valid postcard payload produces an
  error, not a panic.

### 3.3 Rust Wasm tests (`wasm-pack test --headless --chrome`)

Run via `deno task test:wasm` (the agent should add this task if
it does not exist). These exercise the Wasm binaries in a
headless browser, ensuring the JS boundary works as expected.

Required test cases for each Wasm:

- The TS entry point returns a `Promise<Uint8Array>` /
  `Promise<RgbaResult>` as appropriate.
- The output is identical to the output of the Rust-native
  function for the same input.

### 3.4 Deno tests (`deno test`)

Located in `src/` and `tests/`. These exercise the TS wrappers
and the cross-Wasm flow.

Required test cases (at minimum):

- `svg2usvg` returns a `Uint8Array`.
- `usvg2rgba` returns an `RgbaResult` whose `pixels` is a
  `Uint8Array` of exactly `width * height * 4` bytes.
- The end-to-end flow
  `svg2usvg(svgString) → usvg2rgba(bytes, { width, height })`
  produces RGBA bytes of the correct size.
- `init` is idempotent: calling `svg2usvg` twice does not
  re-initialize; calling `usvg2rgba` twice does not
  re-initialize; calling `svg2usvg` and `usvg2rgba` does not
  cause cross-Wasm `init` interference.
- The same SVG string always produces the same `usvg` bytes
  (the determinism check, from the JS side).
- A malformed SVG rejects the `svg2usvg` promise (does not
  throw, does not hang).
- A malformed `usvg` payload rejects the `usvg2rgba` promise.

### 3.5 Visual / pixel-equality tests

There are none. This package produces RGBA pixels, not PNG
images. Any pixel test would be testing a *consumer's* encoder
on top of our output, which is out of scope.

If a future `png` subpath is added (out of scope for the
current task), visual tests for *that* subpath would be
appropriate.

---

## 4. Adding a feature

The agent must follow this process for any non-trivial change:

1. **Read** `AGENTS.md`, `docs/project-constitution.md`,
   `docs/system-architecture.md`, this file.
2. **Draft a plan** in `docs/plans/<feature>.md`. The plan must:
   - Quote the section of the constitution that authorizes the
     change.
   - List the files to be touched, including which of the two
     crates and which of the two Wasm artifacts are affected.
   - List the tests to be added or updated.
   - State the expected size impact on the Wasm binary.
   - Justify any new dependency under the source tiers in
     `./AGENTS.md` §5.
3. **Wait for human approval.** No implementation before approval.
4. **Implement.** Smallest possible diff.
5. **Test.** All four layers from §3 must pass.
6. **Update the build.** If any artifact changed, run
   `deno task build` and commit the regenerated files.
7. **Report.** Per `./AGENTS.md` §9.

### 4.1 Adding a new subpath export

Adding `./png` (a hypothetical RGBA-to-PNG helper) would be
an example of this.

1. Create `crates/rgba2png/`, mirroring the layout of the
   existing two crates.
2. Add a build step in `scripts/build.ts` that builds the third
   crate.
3. Add `./png` to `jsr.json#exports`.
4. Add `src/png.ts`, the re-export wrapper.
5. Add tests for the new subpath in all four layers.

**The agent must not bundle the new crate into either of the
existing Wasm artifacts.** Every subpath corresponds to a
distinct Wasm artifact, by the rule in
`docs/project-constitution.md` §3.9.

### 4.2 Adding a new option to an existing function

Don't. Each function takes a small, fixed set of arguments
(`svg2usvg`: one string; `usvg2rgba`: a `Uint8Array` and an
optional options object). If a new option is needed, escalate
per `./AGENTS.md` §10. The default position is that the option
is unnecessary.

If the option genuinely is necessary, the new option must be
**optional**, **additive** (existing callers must not change
behavior), and **documented in the constitution** as part of the
plan.

---

## 5. Vendoring and CDN-free builds

Per `./AGENTS.md` §5, the browser bundle must not import from a
runtime CDN. The build must produce a self-contained bundle.

### 5.1 Vendoring a new dependency

When a new dependency is added (with human approval), it must
be vendored into `./vendor/<name>/` and imported via a `deno.json`
import map entry. The vendored copy must be:

- A complete, working copy of the dependency, including all of
  its own dependencies.
- Pin-pointed to a specific version, recorded in a `VENDORED.md`
  or similar.
- Reproducible: there must be a script (e.g.
  `scripts/vendor.ts`) that re-creates the vendored copy from
  the source registry.

### 5.2 Verifying the bundle is CDN-free

After `deno task build` (or after the bundler runs), the agent
must run a check that the bundle contains no `import` or
`require` referencing a runtime URL. The exact check depends
on the bundler in use; the recommended approach is:

1. `deno bundle` the entry point to a single `.js` file.
2. Grep the resulting file for `https://`.
3. Any match is a hard fail.

The check is part of `deno task build` and a failed check
aborts the build.

---

## 6. Releasing

Releases are the human's responsibility. The agent's
responsibilities around releases are:

- Keep `CHANGELOG.md` up to date as part of every change.
- Bump the version in `jsr.json` as part of every change, per
  the semver rules in `docs/project-constitution.md` §7.
- Run `deno task build` and commit the regenerated artifacts
  before the release.
- Tag the release commit with `v<version>`.

The agent must not push tags, publish to JSR, or merge release
PRs. The human does those.

---

## 7. Common pitfalls

### 7.1 Forgetting to re-run the build

If the agent changes a Rust source file and commits without
re-running `deno task build`, the published Wasm and the
committed TS glue will be out of sync. The CI must catch this,
but the agent should catch it first.

### 7.2 Pulling in `resvg` "for convenience" in the `usvg` crate

`resvg` provides a `to_string()` method that produces an SVG
string from a `Tree`. It is tempting to use this as a debugging
aid (e.g. "let me see what `usvg` actually produced"). Do not.
The whole point of `svg2ui8a` is to *not* produce an SVG
string. Use `postcard` to inspect the bytes, or write a Rust
test that prints the tree structure.

Adding `resvg` to the `svg2usvg` crate would also violate
`docs/project-constitution.md` §3.9 by inflating the Wasm
artifact that consumers who only need cache keys are forced to
load.

### 7.3 Treating `usvg::Tree` as JSON-serializable

It is, via `serde_json`. But `svg2ui8a` is `postcard`, not
JSON. Adding `serde_json` as a dependency is a violation of
`docs/project-constitution.md` §3.7.

### 7.4 Adding a `createSvg2usvg` factory

There is no per-instance state worth amortizing in either
function. A factory function would add API surface for no
benefit.

### 7.5 Optimizing the SVG parse step

`usvg` is upstream. The agent must not vendor a fork or apply
patches to it. If a specific input is too slow, escalate.

### 7.6 Optimizing the rasterize step

`resvg` and `tiny-skia` are upstream. Same rule.

### 7.7 Adding caching inside the TS wrappers

The TS wrappers must not memoize, must not pre-warm, must not
maintain state beyond the single `initialized` boolean. The
Wasm binaries are cheap to call; the consumer is in charge of
caching at the policy layer (KV, R2, etc.).

### 7.8 Adding a runtime CDN import

This is a hard rule. See `./AGENTS.md` §5.2. The build
verification in §5.2 will catch it.

### 7.9 Bundling the two crates

The `usvg` and `rgba` crates must stay as two independent Wasm
artifacts. Bundling them is a violation of
`docs/project-constitution.md` §3.9. The build pipeline in §2
is structured to make this hard to do by accident; the agent
must not work around it.

### 7.10 Adding PNG output "to make it more useful"

The package's job ends at RGBA. PNG encoding is a consumer
concern. If a future `./png` subpath is added, it is added as
a *third* crate and a *third* Wasm artifact, not as a
dependency of `usvg2rgba`.

---

## 8. When in doubt

Stop and ask. The most common cases are:

- "Should this be a new subpath or a new function?"
  → Default: new subpath. Subpaths are for *different Wasm
  artifacts*, not for option permutations.
- "Should this go in `svg2ui8a` or a different package?"
  → Default: `svg2ui8a`, as long as it is a `Uint8Array` in
  and a `Uint8Array` out. Anything else is a different
  package.
- "Should I add a dependency for X?"
  → Default: no. Look for a way to do X with the existing
  dependency list. If X truly cannot be done without a new
  dependency, escalate with the source-tier justification
  (see `./AGENTS.md` §5).
- "Should I optimize the Wasm size?"
  → Default: only if the change is trivial. If it requires
  restructuring, escalate.
- "Can I use a CDN at runtime?"
  → No. See `./AGENTS.md` §5.2.

If the answer is not in this playbook, it is in
`docs/project-constitution.md`. If it is not there either, it
is a human decision.
