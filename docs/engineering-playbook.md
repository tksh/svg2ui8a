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

| Command            | What it does                              |
|--------------------|-------------------------------------------|
| `deno task fmt`    | Format all `.ts` files in `src/` and `tests/`. |
| `deno task lint`   | Lint all `.ts` files.                     |
| `deno task check`  | Type-check all `.ts` files.               |
| `deno task test`   | Run the test suite (Deno + Rust).         |
| `deno task build`  | Rebuild the Wasm binary and the TS glue.  |

The agent must not invent commands. If a command above does not
work, stop and escalate per `./AGENTS.md` §10.

---

## 2. The build pipeline

The build pipeline has three steps:

1. **Compile the Wasm crate** with `wasm-pack`, target `deno`.
2. **Copy the Wasm artifact** to `assets/svg2usvg_bg.wasm`.
3. **Emit the TS glue** to `src/usvg.ts`.

All three are driven by `scripts/build.ts`, run via
`deno task build`. The agent must run the build script end to
end, not invoke the steps individually.

### 2.1 What the agent must never do

- Hand-edit `src/usvg.ts`. Regenerate it.
- Hand-edit anything under `assets/`. The Wasm is a build
  artifact.
- Pin `wasm-pack` to a version not recorded in
  `scripts/build.ts`. If a version bump is needed, edit the
  script.
- Commit a build artifact in a way that is not reproducible from
  source. The artifact must be buildable from a clean checkout.

### 2.2 Build output (what should appear after `deno task build`)

```
crates/svg2usvg/pkg/svg2usvg_bg.wasm       # the Wasm binary
crates/svg2usvg/pkg/svg2usbg.js            # the wasm-bindgen JS glue
crates/svg2usvg/pkg/svg2usbg.d.ts          # the wasm-bindgen type defs
src/usvg.ts                                # the re-export wrapper
assets/svg2usvg_bg.wasm                    # the vendored Wasm
```

If any of these are missing, the build is broken. Stop and
escalate.

---

## 3. Testing

There are three layers of tests, and they must all pass before
declaring a change done.

### 3.1 Rust unit tests (`cargo test`)

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
  determinism check from `docs/project-constitution.md` §5.1).

### 3.2 Rust Wasm tests (`wasm-pack test --headless --chrome`)

Run via `deno task test:wasm` (the agent should add this task if
it does not exist). These exercise the Wasm binary in a headless
browser, ensuring the JS boundary works as expected.

Required test cases:

- The TS entry point returns a `Uint8Array`.
- The `Uint8Array` is non-empty for a non-trivial SVG.
- The output is identical to the output of the Rust-native
  function for the same input.

### 3.3 Deno tests (`deno test`)

Located in `src/` and `tests/`. These exercise the TS wrapper.

Required test cases:

- `svg2usvg` returns a `Uint8Array`.
- The same SVG string always produces the same `Uint8Array`.
- A malformed SVG rejects the promise (does not throw, does not
  hang).
- `init` is idempotent: calling `svg2usvg` twice does not
  re-initialize.

### 3.4 Visual / pixel-equality tests

There are none for `svg2ui8a`. This package does not produce
pixels. Any pixel test would be testing a future `usvg2rgba`
package, not this one.

---

## 4. Adding a feature

The agent must follow this process for any non-trivial change:

1. **Read** `AGENTS.md`, `docs/project-constitution.md`,
   `docs/system-architecture.md`, this file.
2. **Draft a plan** in `docs/plans/<feature>.md`. The plan must:
   - Quote the section of the constitution that authorizes the
     change.
   - List the files to be touched.
   - List the tests to be added or updated.
   - State the expected size impact on the Wasm binary.
3. **Wait for human approval.** No implementation before approval.
4. **Implement.** Smallest possible diff.
5. **Test.** All three layers from §3 must pass.
6. **Update the build.** If any artifact changed, run
   `deno task build` and commit the regenerated files.
7. **Report.** Per `./AGENTS.md` §9.

### 4.1 Adding a new subpath export

Example: adding `./renderer` for a future `usvg2rgba` build.

1. Create `crates/usvg2rgba/`, mirroring the layout of
   `crates/svg2usvg/`.
2. Add a `build` step in `scripts/build.ts` that builds the
   second crate.
3. Add `./renderer` to `jsr.json#exports`.
4. Add `src/renderer.ts`, the re-export wrapper.
5. Add tests for the new subpath in all three layers.

**The agent must not bundle the new crate into the existing
`./usvg` build.** The two builds are independent Wasm artifacts
on purpose. Conflating them violates
`docs/project-constitution.md` §3.3.

### 4.2 Adding a new option to `svg2usvg`

Don't. The function takes one argument (an SVG string) and
returns one value (a `Uint8Array`). If the new option can be
expressed as a separate function (e.g.
`svg2usvgWithVersion(svg, version)`), do that instead. If it
cannot, escalate per `./AGENTS.md` §10.

---

## 5. Releasing

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

## 6. Common pitfalls

### 6.1 Forgetting to re-run the build

If the agent changes `crates/svg2usvg/src/lib.rs` and commits
without re-running `deno task build`, the published Wasm and
the committed TS glue will be out of sync. The CI must catch
this, but the agent should catch it first.

### 6.2 Pulling in `resvg` "for convenience"

`resvg` provides a `to_string()` method that produces an
SVG string from a `Tree`. It is tempting to use this as a
debugging aid (e.g. "let me see what `usvg` actually
produced"). Do not. The whole point of `svg2ui8a` is to
**not** produce an SVG string. Use `postcard` to inspect the
bytes, or write a Rust test that prints the tree structure.

### 6.3 Treating `usvg::Tree` as JSON-serializable

It is, via `serde_json`. But `svg2ui8a` is `postcard`, not
JSON. Adding `serde_json` as a dependency is a violation of
`docs/project-constitution.md` §3.7.

### 6.4 Adding a `createSvg2usvg` factory

There is no per-instance state worth amortizing in this
package. A factory function would add API surface for no
benefit. If a future package needs one, it can add it.

### 6.5 Optimizing the SVG parse step

`usvg` is upstream. The agent must not vendor a fork or apply
patches to it. If a specific input is too slow, escalate.

### 6.6 Optimizing postcard encode

Same. Upstream. Do not fork.

### 6.7 Adding caching inside the TS wrapper

The TS wrapper must not memoize, must not pre-warm, must not
maintain state beyond the single `initialized` boolean. The
Wasm binary is cheap to call; the consumer is in charge of
caching at the policy layer (KV, R2, etc.).

---

## 7. When in doubt

Stop and ask. The most common cases are:

- "Should this be a new subpath or a new function?"
  → Default: new function. Subpaths are for *different Wasm
  artifacts*, not for option permutations.
- "Should this go in `svg2ui8a` or a future `usvg2rgba`?"
  → Default: future `usvg2rgba`. `svg2ui8a` is intentionally
  minimal.
- "Should I add a dependency for X?"
  → Default: no. Look for a way to do X with the existing
  dependency list.
- "Should I optimize the Wasm size?"
  → Default: only if the change is trivial. If it requires
  restructuring, escalate.

If the answer is not in this playbook, it is in
`docs/project-constitution.md`. If it is not there either, it
is a human decision.
