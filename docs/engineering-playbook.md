# Engineering Playbook: `@tksh/svg2ui8a`

This file is the **fourth** document an AI agent reads (after `AGENTS.md`,
`docs/project-constitution.md`, and `docs/system-architecture.md`). It describes
the day-to-day mechanics: how to build, how to test, how to add a feature
without breaking anything, and how to release.

The human owns this file. If the agent believes something here is wrong, the
agent must surface the issue, not rewrite the file.

---

## 0. How much freedom the agent has

This playbook, the constitution, and the architecture describe the **boundary**
of the package: what it does, what it does not, what the public API looks like,
how the Wasm artifacts are laid out, and what tests must pass.

They do **not** describe:

- The exact `usvg` / `resvg` call sequence for rasterization or serialization.
- The exact import path for the Wasm binary (raw `.wasm` vs. wasm-pack glue vs.
  bundled asset).
- The exact error type and message wording.
- The exact pixel-handling arithmetic (e.g. un-premultiply).

These are **implementer judgments**. The implementer records the choice and a
short rationale in the plan. If a choice turns out to be wrong, the next plan
corrects it; the constitution is amended only if the choice violated a hard
constraint.

The Rust dependency baseline is fixed: `usvg` `0.47.0` and `resvg` `0.47.0`
(`default-features = false`; text, system-font, memmap-font, and raster-image
features are prohibited). There is no `cbor-core` or other serialization crate;
the package has no proprietary serialization (constitution §3.7). An
implementation plan may explain how it uses these crates, but may not select
alternate versions, features, or codecs without an approved plan.

If the implementer is unsure between two choices, they **pick one and document
it**, then ask the human to confirm. They do not block on every small decision.

---

## 1. Repository commands

All commands assume the working directory is the repository root.

| Command               | What it does                                   |
| --------------------- | ---------------------------------------------- |
| `cargo fmt`           | Format changed `.rs` files.                    |
| `deno fmt`            | Format changed `.ts` and `.md` files.          |
| `deno task lint`      | Lint all `.ts` files.                          |
| `deno task check`     | Type-check all `.ts` files.                    |
| `deno task test`      | Run the full test suite (Rust + Wasm + Deno).  |
| `deno task test:rust` | Run `cargo test` in both crates.               |
| `deno task test:wasm` | Run the Wasm tests in a headless browser.      |
| `deno task build`     | Rebuild the two Wasm binaries and the TS glue. |

The "do not invent commands" rule from `AGENTS.md` applies to **ad-hoc** shell
commands. The `test:wasm` and `test:rust` tasks are explicitly called out by
this playbook and are not "invented".

The agent formats the files it has changed: use `cargo fmt` for `.rs` files and
`deno fmt` for `.ts` and `.md` files. If either formatter would change lines the
agent did not touch, those lines are left alone (revert any incidental changes
before committing).

---

## 2. The build pipeline

The build pipeline has two parallel tracks (one per Wasm crate) and a single
driver script.

### 2.1 Per-crate track

For each of `crates/svg2usvg/` and `crates/svg2rgba/`:

1. Compile the Rust crate to Wasm with `wasm-pack` (or the implementer's chosen
   equivalent).
2. The Wasm artifact and any generated glue end up in `crates/<crate>/pkg/` (or
   wherever `wasm-pack` writes).
3. The build script copies the Wasm to `assets/<crate>_bg.wasm`.
4. The build script regenerates `src/<subpath>.ts` from a template. The template
   may import the raw `.wasm` from `assets/`, or it may import the generated
   glue — the implementer chooses. The constraint is **§3 of the constitution**
   (no runtime CDN import) and the build verification in §5.2 of this playbook.

The `scripts/build.ts` is the **initial implementation scaffolding**, not
follow-up work. The first implementation task creates it as part of its
baseline.

### 2.2 The driver

`scripts/build.ts` runs both tracks and verifies the output. The exact output of
`deno task build` includes:

- `assets/svg2usvg_bg.wasm`
- `assets/svg2rgba_bg.wasm`
- `src/svg2usvg.ts`
- `src/svg2rgba.ts`
- `src/mod.ts`

The `crates/*/pkg/` intermediates may or may not be committed; the implementer
decides based on what the Wasm-import shape requires. Whatever the choice, the
`.gitignore` reflects it.

### 2.3 What the agent must never do

- Hand-edit `src/svg2usvg.ts` or `src/svg2rgba.ts`. Regenerate them.
- Hand-edit anything under `assets/`. The Wasm artifacts are build artifacts.
- Pin `wasm-pack` to a version not recorded in `scripts/build.ts`. If a version
  bump is needed, edit the script.
- Commit a build artifact in a way that is not reproducible from source. Both
  Wasm artifacts must be buildable from a clean checkout.
- Bundle the crates into a single Wasm artifact. See
  `docs/project-constitution.md` §3.9.
- Add a runtime CDN import in the browser bundle. See
  `docs/project-constitution.md` §3.8 and `./AGENTS.md` §5.

---

## 3. Testing

There are four layers of tests, and they must all pass before declaring a change
done.

`deno task test` orchestrates all four: `test:rust` → `test:wasm` →
`deno test -A`.

The test layer that follows the **most domain-specific guarantee** is the
implementation's own; the test layer that follows the **constitution's API
contract** is the Deno layer.

### 3.1 Rust native tests for `svg2usvg` (`cargo test`)

Located in `crates/svg2usvg/src/core.rs`, `crates/svg2usvg/src/lib.rs`, and
`crates/svg2usvg/examples/dump_core.rs`. The `core` function is tested natively;
the `#[wasm_bindgen]` wrapper is tested at the Wasm layer.

Required test cases (at minimum):

- A simple SVG parses and produces non-empty `Vec<u8>` (UTF-8 XML bytes).
- A simple SVG produces stable bytes across two consecutive calls.
- A malformed SVG produces an error, not a panic.
- SVG containing `<text>` or `<image>` content produces an error, not a payload.
- Output bytes are valid UTF-8 and contain `<svg`.
- The bytes re-parse with `usvg::Tree::from_str`.

If a test fails for a reason the implementer cannot diagnose quickly, the
implementer stops and escalates per `./AGENTS.md` §10.

### 3.2 Rust native tests for `svg2rgba` (`cargo test`)

Located in `crates/svg2rgba/src/core.rs` and `crates/svg2rgba/tests/`. The
`core` function is tested natively; the `#[wasm_bindgen]` wrapper is tested at
the Wasm layer.

Required test cases (at minimum):

- A simple SVG renders to a pixmap of the expected size.
- `width` / `height` options are honored (the output pixmap is the requested
  size, not the natural size).
- **Only `width` set**: the output width is exact and the height is calculated
  from the fractional natural aspect ratio, then rounded to pixels.
- **Only `height` set**: the output height is exact and the width is calculated
  from the fractional natural aspect ratio, then rounded to pixels.
- **Fractional natural size**: natural dimensions remain fractional until the
  output pixel dimensions are selected.
- **Both set to a non-uniform aspect ratio**: the output is exactly
  `width × height` (independent scaling).
- A zero-sized SVG produces an error.
- **Alpha mode: default (straight)** — a 50%-opaque red pixel renders to
  `(255, 0, 0, 128)`, not `(128, 0, 0, 128)`.
- **Alpha mode: as-is (premultiplied)** — the same input renders to
  `(128, 0, 0, 128)`.
- **Renderer determinism**: the same input produces the same bytes across two
  consecutive calls.
- SVG containing `<text>` or `<image>` produces an error, not a payload.

### 3.3 Rust Wasm tests (`wasm-pack test` or equivalent)

Run via `deno task test:wasm`. These exercise the Wasm binaries in a headless
browser, ensuring the JS boundary works as expected. The "or equivalent" in this
section is fixed to **Astral-driven headless Chrome via CDP**
(`jsr:@astral/astral@0.5.6`, pinned Chrome `125.0.6400.0`); no chromedriver, no
WebDriver, no `npm:` package. The Wasm layer is verified in two complementary
harnesses: the Deno-side `deno task test:wasm` (`scripts/test-wasm.ts`) and the
browser-side `deno task test:wasm:browser` (`scripts/test-wasm-browser.ts`)
which loads the shipped `src/svg2usvg.ts` / `src/svg2rgba.ts` inside headless
Chrome via Astral and asserts the same two guarantees there. See
`docs/plans/wasm-browser-tests.md` §§2, 7, 12 for the pin, tradeoff, and CI
notes.

Required test cases for each Wasm:

- The TS entry point returns a `Promise<Uint8Array>` / `Promise<RgbaResult>` as
  appropriate.
- The output is identical to the output of the Rust-native `core` function for
  the same input.

### 3.4 Deno tests (`deno test`)

Located in `src/` and `tests/`. These exercise the TS wrappers and the
cross-Wasm flow.

Required test cases (at minimum):

- `svg2usvg` returns a `Uint8Array` containing valid SVG XML (non-empty,
  includes `<svg`, re-parseable).
- `svg2rgba` returns an `RgbaResult` whose `pixels` is a `Uint8Array` of exactly
  `width * height * 4` bytes.
- `RgbaResult` carries the alpha mode that the option specified (or the default
  if no option was given).
- The `svg2usvg` bytes `TextDecoder`-decoded re-render identically via
  `svg2rgba` (usvg XML is valid SVG input).
- `init` is idempotent: calling `svg2usvg` twice does not re-initialize; calling
  `svg2rgba` twice does not re-initialize; calling both does not cause
  cross-Wasm `init` interference.
- The same SVG string always produces the same `svg2usvg` bytes and the same
  `svg2rgba` pixels (determinism, from the JS side).
- A malformed SVG rejects the `svg2usvg` promise.
- `<text>` / `<image>` SVG rejects both promises.
- `svg2rgba`'s resolved result is a plain data object: its prototype is
  `Object.prototype` (not a wasm-bindgen class instance), it exposes no `free()`
  method and no internal wasm-bindgen pointer field, its `pixels` field is a
  genuine `Uint8Array` (not a wrapped subclass), and the result survives
  `structuredClone` and a `JSON.stringify` / `JSON.parse` round-trip without
  throwing or gaining/losing fields.
- A result returned from one `svg2rgba` call is unaffected by a subsequent
  `svg2rgba` call with different options / output (guards against `pixels` being
  a live view into reused or freed Wasm linear memory rather than an owned
  copy).
- `svg2rgba`'s `options` parameter accepts a bare object literal (not a
  wasm-bindgen class instance constructed via `new`), including a partial
  literal and an empty object `{}`, and `{}` behaves identically to omitting
  `options` entirely.
- Neither `src/svg2usvg.ts`, `src/svg2rgba.ts`, nor `src/mod.ts` exports any
  runtime symbol whose name starts with `__wasm_` (no internal wasm-bindgen glue
  is reachable from the public subpaths).

### 3.5 Visual / pixel-equality tests

There are no PNG visual tests. The package produces RGBA pixels, not PNG images.
Byte-level RGBA golden tests are **allowed** and **recommended** for the
`svg2rgba` crate (see §3.2). They are unit tests, not visual tests, and do not
require human review before pass.

### 3.6 Dependency-feature verification

The implementation verifies the resolved Cargo features for `usvg` and `resvg`.
The feature graph must not include `text`, `system-fonts`, `memmap-fonts`, or
`raster-images`. `tiny-skia` is permitted only through `resvg`; no image decoder
or direct rasterizer dependency is added to the `svg2usvg` crate.

---

## 4. Adding a feature

The agent must follow this process for any non-trivial change:

1. **Read** `AGENTS.md`, `docs/project-constitution.md`,
   `docs/system-architecture.md`, this file.
2. **Draft a plan** in `docs/plans/<feature>.md`. The plan must:
   - Quote the section of the constitution that authorizes the change.
   - List the files to be touched, including which of the two crates and which
     Wasm artifacts are affected.
   - List the tests to be added or updated.
   - State the expected size impact on the Wasm binary.
   - Justify any new dependency under the source tiers in `./AGENTS.md` §5.
3. **Wait for human approval.** No implementation before approval.
4. **Implement.** Smallest possible diff.
5. **Test.** All four layers from §3 must pass.
6. **Update the build.** If any artifact changed, run `deno task build` and
   commit the regenerated files.
7. **Report.** Per `./AGENTS.md` §9.

### 4.1 Adding a new subpath export

Adding `./png` (a hypothetical RGBA-to-PNG helper) would be an example of this.

1. Create `crates/rgba2png/`, mirroring the layout of the existing two crates.
2. Add a build step in `scripts/build.ts` that builds the third crate.
3. Add `./png` to `jsr.json#exports`.
4. Add `src/png.ts`, the re-export wrapper.
5. Add tests for the new subpath in all four layers.

**The agent must not bundle the new crate into either of the existing Wasm
artifacts.** Every subpath corresponds to a distinct Wasm artifact, by the rule
in `docs/project-constitution.md` §3.9.

### 4.2 Adding a new option to an existing function

The constitution fixes the _shape_ of the option surface but not the _names_ of
the fields. The implementer may add a field that the constitution does not
contradict. If the new field is required for some new use case:

- It must be **optional** (existing callers must not change behavior).
- It must be **documented in the constitution** as part of the plan.
- It must be **justified** in the "Dependency changes" section (if the option
  requires a new dependency).

If the new option requires a new dependency that is not on the constitution's
allow-list, the plan escalates per `./AGENTS.md` §10.

---

## 5. Vendoring and CDN-free builds

Per `./AGENTS.md` §5, the browser bundle must not import from a runtime CDN. The
build must produce a self-contained bundle.

### 5.1 Vendoring a new dependency

When a new dependency is added (with human approval), it is **vendored into
`./vendor/<name>/`** and imported via a `deno.json` import map entry. The
vendored copy must be:

- A complete, working copy of the dependency, including all of its own
  dependencies.
- Pin-pointed to a specific version, recorded in a `VENDORED.md` (or similar)
  inside the vendored directory.
- Reproducible: there must be a script (`scripts/vendor.ts`) that re-creates the
  vendored copy from the source registry.

`vendor/` is **not committed** in normal development. It is gitignored.
Day-to-day work builds against the JSR dependency (per `./AGENTS.md` §5.1 tier
1). The vendored tree is **committed only as part of a release snapshot**, via
`deno task release` (which the human runs, not the agent).

### 5.2 Verifying the bundle is CDN-free

After `deno task build` (or after the bundler runs), the agent must run a check
that the bundle contains no `import` or `require` referencing a runtime URL. The
check is part of `deno task build` and a failed check aborts the build.

The check bundles **each of the package's three subpath entry points** (root +
two leaves) (`@tksh/svg2ui8a`, `@tksh/svg2ui8a/svg2rgba`,
`@tksh/svg2ui8a/svg2usvg`) and greps the resulting bundle for `https://`. Any
match is a hard fail.

This is about the **package's ship artifacts**, not the consumer's bundle. The
consumer is responsible for their own bundle hygiene.

---

## 6. Releasing

Releases are initiated by the human and published by GitHub Actions. The package
version is not stored in `deno.json`; the release tag is the single source of
the published version.

The release flow is:

1. Add an entry to `CHANGELOG.md` under the **"Unreleased"** section and update
   the release notes as appropriate.
2. Run `deno task build` and commit the regenerated artifacts before the
   release.
3. Create an annotated tag named `v<version>` on the release commit.
4. Push the tag with `git push --tags`.
5. The `.github/workflows/publish.yml` workflow runs for `v*.*.*` tags and
   invokes `deno publish --set-version <version>`, using the tag name without
   the leading `v`.

The human is responsible for creating and pushing the tag. The GitHub Actions
workflow is responsible for publishing to JSR. The agent must not push tags,
publish to JSR, or merge release PRs.

---

## 7. Common pitfalls

### 7.1 Forgetting to re-run the build

If the agent changes a Rust source file and commits without re-running
`deno task build`, the published Wasm and the committed TS glue will be out of
sync. The CI must catch this, but the agent should catch it first.

### 7.2 Pulling in `resvg` "for convenience" in the `svg2usvg` crate

`resvg` provides rendering that `svg2usvg` does not need. Adding `resvg` to the
`svg2usvg` crate would inflate the Wasm artifact that consumers who only need
normalized SVG bytes are forced to load, violating
`docs/project-constitution.md` §3.9. (`svg2rgba` legitimately depends on `resvg`
— it is a separate artifact.)

### 7.3 Adding a `createSvg2usvg` factory

There is no per-instance state worth amortizing in either function. A factory
function would add API surface for no benefit.

### 7.4 Optimizing the SVG parse step

`usvg` is upstream. The agent must not vendor a fork or apply patches to it. If
a specific input is too slow, escalate.

### 7.5 Optimizing the rasterize step

`resvg` and `tiny-skia` are upstream. Same rule.

### 7.6 Adding caching inside the TS wrappers

The TS wrappers must not memoize, must not pre-warm, must not maintain state
beyond the single `initialized` boolean. The Wasm binaries are cheap to call;
the consumer is in charge of caching at the policy layer (KV, R2, etc.).

### 7.7 Adding a runtime CDN import

This is a hard rule. See `./AGENTS.md` §5.2. The build verification in §5.2 will
catch it.

### 7.8 Bundling the two crates

The `svg2usvg` and `svg2rgba` crates must stay as two independent Wasm
artifacts. Bundling them is a violation of `docs/project-constitution.md` §3.9
(one independent artifact per capability). The build pipeline in §2 is
structured to make this hard to do by accident; the agent must not work around
it.

### 7.9 Adding PNG output "to make it more useful"

The package's job ends at RGBA. PNG encoding is a consumer concern. If a future
`./png` subpath is added, it is added as a _third_ crate and a _third_ Wasm
artifact, not as a dependency of `svg2rgba`.

### 7.10 Returning a wasm-bindgen class instance instead of a plain object

`svg2rgba` must resolve to a plain data object, not a wasm-bindgen class
instance (`docs/project-constitution.md` §4.4). This includes values only
superficially disguised as plain data — e.g. removing a type export while the
runtime object is still a class instance carrying a `free()` method and an
internal wasm-bindgen pointer field. The §3.4 Deno tests catch this by asserting
the resolved prototype, the absence of `free()` and pointer fields, and
successful `structuredClone` / `JSON` round-trips.

---

## 8. When in doubt

Stop and ask. The most common cases are:

- "Should this be a new subpath or a new function?" → Default: new subpath.
  Subpaths are for _different Wasm artifacts_, not for option permutations.
- "Should this go in the package or a different package?" → Default: in this
  package, as long as the _payload_ is a `Uint8Array` in and a `Uint8Array` (or
  `RgbaResult` with a `Uint8Array` `pixels` field) out. Anything else is a
  different package.
- "Should I add a dependency for X?" → Default: no. Look for a way to do X with
  the existing dependency list. If X truly cannot be done without a new
  dependency, escalate with the source-tier justification (see `./AGENTS.md`
  §5).
- "Should I optimize the Wasm size?" → Default: only if the change is trivial.
  If it requires restructuring, escalate.
- "Can I use a CDN at runtime?" → No. See `./AGENTS.md` §5.2.
- "Which serialization does the package use?" → None proprietary; `svg2usvg`
  bytes are standard SVG XML. See `docs/project-constitution.md` §3.7.

If the answer is not in this playbook, it is in `docs/project-constitution.md`.
If it is not there either, it is a human decision.
