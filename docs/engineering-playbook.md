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

- The exact DTO shape (if any) inside the package's internal representation.
- The exact `usvg` / `resvg` call sequence for rasterization.
- The exact import path for the Wasm binary (raw `.wasm` vs. wasm-pack glue vs.
  bundled asset).
- The exact error type and message wording.
- The exact pixel-handling arithmetic (e.g. un-premultiply).

These are **implementer judgments**. The implementer records the choice and a
short rationale in the plan. If a choice turns out to be wrong, the next plan
corrects it; the constitution is amended only if the choice violated a hard
constraint.

The Rust dependency baseline is fixed: `cbor-core` 0.10.1, `usvg` 0.47.0, and
`resvg` 0.47.0. `usvg` and `resvg` use `default-features = false`; text,
system-font, memmap-font, and raster-image features are prohibited. An
implementation plan may explain how it uses these crates, but may not select
alternate versions, features, or CBOR codecs.

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

The shared `crates/intermediate/` crate is built into both tracks but produces
no Wasm artifact of its own. It owns the versioned DTO and its canonical-CBOR
encode, decode, and semantic validation.

### 2.1 Per-crate track

For each of `crates/svg2usvg/` and `crates/usvg2rgba/`:

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
- `assets/usvg2rgba_bg.wasm`
- `src/usvg.ts`
- `src/rgba.ts`
- `src/mod.ts`

The `crates/*/pkg/` intermediates may or may not be committed; the implementer
decides based on what the Wasm-import shape requires. Whatever the choice, the
`.gitignore` reflects it.

### 2.3 What the agent must never do

- Hand-edit `src/usvg.ts` or `src/rgba.ts`. Regenerate them.
- Hand-edit anything under `assets/`. The Wasm artifacts are build artifacts.
- Pin `wasm-pack` to a version not recorded in `scripts/build.ts`. If a version
  bump is needed, edit the script.
- Commit a build artifact in a way that is not reproducible from source. Both
  Wasm artifacts must be buildable from a clean checkout.
- Bundle the two crates into a single Wasm artifact. See
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

Located in `crates/svg2usvg/src/core.rs` and `crates/svg2usvg/tests/`. The
`core` function is tested natively; the `#[wasm_bindgen]` wrapper is tested at
the Wasm layer.

Required test cases (at minimum):

- A simple SVG parses and produces a non-empty `Vec<u8>`.
- A simple SVG produces stable bytes across two consecutive calls.
- A malformed SVG produces an error, not a panic.
- SVG containing `<text>` or `<image>` content produces an error, not a payload.
- The same SVG string always produces the same bytes (the determinism check from
  `docs/project-constitution.md` §5.3).
- **Round-trip test**: the chosen internal representation can be encoded and
  decoded back without loss. The exact form of this test depends on the chosen
  representation (DTO with serde, `cbor_core::Value` mapping, etc.) and is
  decided at impl time. **The test exists.** Its specific assertions are the
  implementer's call.
- **Envelope test**: the encoded map has identifier `svg2ui8a/usvg`, unsigned
  format version `1`, and a DTO payload at the required integer keys.

If the round-trip test fails for a reason the implementer cannot diagnose
quickly, the implementer stops and escalates per `./AGENTS.md` §10.

### 3.2 Rust native tests for `usvg2rgba` (`cargo test`)

Located in `crates/usvg2rgba/src/core.rs` and `crates/usvg2rgba/tests/`. The
`core` function is tested natively; the `#[wasm_bindgen]` wrapper is tested at
the Wasm layer.

Required test cases (at minimum):

- A simple SVG renders to a pixmap of the expected size.
- `width` / `height` options are honored (the output pixmap is the requested
  size, not the natural size).
- **Only `width` set**: the output is `width × natural_h`.
- **Only `height` set**: the output is `natural_w × height`.
- **Both set to a non-uniform aspect ratio**: the output is exactly
  `width × height` (independent scaling).
- A non-canonical, non-CBOR, or non-package payload produces an error, not a
  panic.
- An unknown format identifier, unsupported format version, malformed DTO, or
  unsupported DTO variant produces an error, not a panic.
- A zero-sized SVG produces an error.
- **Alpha mode: default (straight)** — a 50%-opaque red pixel renders to
  `(255, 0, 0, 128)`, not `(128, 0, 0, 128)`.
- **Alpha mode: as-is (premultiplied)** — the same input renders to
  `(128, 0, 0, 128)`.
- **Renderer determinism**: the same input produces the same bytes across two
  consecutive calls.

### 3.3 Rust Wasm tests (`wasm-pack test` or equivalent)

Run via `deno task test:wasm`. These exercise the Wasm binaries in a headless
browser, ensuring the JS boundary works as expected.

Required test cases for each Wasm:

- The TS entry point returns a `Promise<Uint8Array>` / `Promise<RgbaResult>` as
  appropriate.
- The output is identical to the output of the Rust-native `core` function for
  the same input.

### 3.4 Deno tests (`deno test`)

Located in `src/` and `tests/`. These exercise the TS wrappers and the
cross-Wasm flow.

Required test cases (at minimum):

- `svg2usvg` returns a `Uint8Array`.
- `usvg2rgba` returns an `RgbaResult` whose `pixels` is a `Uint8Array` of
  exactly `width * height * 4` bytes.
- `RgbaResult` carries the alpha mode that the option specified (or the default
  if no option was given).
- The end-to-end flow
  `svg2usvg(svgString) → usvg2rgba(bytes, { width, height })` produces RGBA
  bytes of the correct size.
- The bytes returned by `svg2usvg` can be written to a `.cbor` fixture, read
  back as `Uint8Array`, and passed unchanged to `usvg2rgba` with the same RGBA
  result.
- `init` is idempotent: calling `svg2usvg` twice does not re-initialize; calling
  `usvg2rgba` twice does not re-initialize; calling `svg2usvg` and `usvg2rgba`
  does not cause cross-Wasm `init` interference.
- The same SVG string always produces the same `usvg` bytes (the determinism
  check, from the JS side).
- A malformed SVG rejects the `svg2usvg` promise.
- A malformed `usvg` payload rejects the `usvg2rgba` promise.

### 3.5 Visual / pixel-equality tests

There are no PNG visual tests. The package produces RGBA pixels, not PNG images.
Byte-level RGBA golden tests are **allowed** and **recommended** for the
`usvg2rgba` crate (see §3.2). They are unit tests, not visual tests, and do not
require human review before pass.

### 3.6 Dependency-feature verification

The implementation verifies the resolved Cargo features for `usvg` and `resvg`.
The feature graph must not include `text`, `system-fonts`, `memmap-fonts`, or
`raster-images`. `tiny-skia` is permitted only through `resvg`; no image decoder
or direct rasterizer dependency is added to the producer crate.

---

## 4. Adding a feature

The agent must follow this process for any non-trivial change:

1. **Read** `AGENTS.md`, `docs/project-constitution.md`,
   `docs/system-architecture.md`, this file.
2. **Draft a plan** in `docs/plans/<feature>.md`. The plan must:
   - Quote the section of the constitution that authorizes the change.
   - List the files to be touched, including which of the two crates and which
     of the two Wasm artifacts are affected.
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

The check bundles **each of the package's three subpath entry points**
(`@tksh/svg2ui8a`, `@tksh/svg2ui8a/usvg`, `@tksh/svg2ui8a/rgba`) and greps the
resulting bundle for `https://`. Any match is a hard fail.

This is about the **package's ship artifacts**, not the consumer's bundle. The
consumer is responsible for their own bundle hygiene.

---

## 6. Releasing

Releases are the human's responsibility. The agent's responsibilities around
releases are:

- Add an entry to `CHANGELOG.md` under the **"Unreleased"** section as part of
  every change. Version bumps and tags are **human-driven**; see `./AGENTS.md`
  §8 (no push / no merge without human approval).
- Run `deno task build` and commit the regenerated artifacts before the release.
- Tag the release commit with `v<version>` (human does this).

The agent must not push tags, publish to JSR, or merge release PRs. The human
does those.

---

## 7. Common pitfalls

### 7.1 Forgetting to re-run the build

If the agent changes a Rust source file and commits without re-running
`deno task build`, the published Wasm and the committed TS glue will be out of
sync. The CI must catch this, but the agent should catch it first.

### 7.2 Pulling in `resvg` "for convenience" in the `usvg` crate

`resvg` provides a `to_string()` method that produces an SVG string from a
`Tree`. It is tempting to use this as a debugging aid (e.g. "let me see what
`usvg` actually produced"). Do not. The whole point of the package is to _not_
produce an SVG string. Use the package's own debug output (e.g. print the DTO,
or hexdump the CBOR) instead.

Adding `resvg` to the `svg2usvg` crate would also violate
`docs/project-constitution.md` §3.9 by inflating the Wasm artifact that
consumers who only need cache keys are forced to load.

### 7.3 Treating the internal representation as JSON

The internal representation is CBOR, not JSON. Adding `serde_json` as a
dependency "for debugging" is a violation of `docs/project-constitution.md`
§3.7. Use `cbor_core` for any debug print.

### 7.4 Adding a `createSvg2usvg` factory

There is no per-instance state worth amortizing in either function. A factory
function would add API surface for no benefit.

### 7.5 Optimizing the SVG parse step

`usvg` is upstream. The agent must not vendor a fork or apply patches to it. If
a specific input is too slow, escalate.

### 7.6 Optimizing the rasterize step

`resvg` and `tiny-skia` are upstream. Same rule.

### 7.7 Adding caching inside the TS wrappers

The TS wrappers must not memoize, must not pre-warm, must not maintain state
beyond the single `initialized` boolean. The Wasm binaries are cheap to call;
the consumer is in charge of caching at the policy layer (KV, R2, etc.).

### 7.8 Adding a runtime CDN import

This is a hard rule. See `./AGENTS.md` §5.2. The build verification in §5.2 will
catch it.

### 7.9 Bundling the two crates

The `usvg` and `rgba` crates must stay as two independent Wasm artifacts.
Bundling them is a violation of `docs/project-constitution.md` §3.9. The build
pipeline in §2 is structured to make this hard to do by accident; the agent must
not work around it.

### 7.10 Adding PNG output "to make it more useful"

The package's job ends at RGBA. PNG encoding is a consumer concern. If a future
`./png` subpath is added, it is added as a _third_ crate and a _third_ Wasm
artifact, not as a dependency of `usvg2rgba`.

### 7.11 Skipping the round-trip test

`docs/engineering-playbook.md` §3.1 makes a round-trip test **required**, not
optional. If it fails, the implementer must not "work around" a failure by
switching formats (constitution §3.7 forbids that) or by editing the
constitution. They escalate.

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
- "Which serialization format does the package use?" → `cbor-core` / canonical
  CBOR, always. See `docs/project-constitution.md` §3.7.

If the answer is not in this playbook, it is in `docs/project-constitution.md`.
If it is not there either, it is a human decision.
